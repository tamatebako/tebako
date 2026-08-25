//! Boot orchestration against the process-global TFS context (spec 17):
//! env-image mount, bare/package payload mounts, entry resolution,
//! jails, and full rollback on failure. All tests serialize on LOCK —
//! the context is process-global (the tests/contract pattern).

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tebako_driver::{boot, Env, MountModes};
use tfs::context::context;

static LOCK: Mutex<()> = Mutex::new(());

struct Guard {
    _guard: MutexGuard<'static, ()>,
    tmp: TempDir,
}

impl Guard {
    fn path(&self) -> &Path {
        self.tmp.path()
    }
}

fn guard(tag: &str) -> Guard {
    let g = LOCK.lock().unwrap();
    let tmp = TempDir::new(tag);
    reset();
    Guard { _guard: g, tmp }
}

fn reset() {
    context().write().unwrap().unmount();
    context()
        .write()
        .unwrap()
        .set_host_policy(tfs::policy::HostPolicy::open(), None);
    tfs::trace::disarm();
}

// ---------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("tebako-driver-{tag}-{}-{uniq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A map-backed [`Env`] (no process-env mutation).
struct MapEnv(std::cell::RefCell<HashMap<String, String>>);

impl MapEnv {
    fn new() -> MapEnv {
        MapEnv(std::cell::RefCell::new(HashMap::new()))
    }

    fn set(&mut self, key: &str, value: impl Into<String>) {
        self.0.get_mut().insert(key.to_string(), value.into());
    }
}

impl Env for MapEnv {
    fn var(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }

    fn set_var(&self, key: &str, value: &str) {
        self.0
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
    }
}

/// Build a zip from an in-memory entry list (the tests/contract
/// fixture pattern).
fn build_zip(path: &Path, dirs: &[&str], files: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).expect("create zip");
    let mut w = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    for d in dirs {
        w.add_directory(*d, opts).unwrap();
    }
    for (name, content) in files {
        w.start_file(name, opts).unwrap();
        w.write_all(content).unwrap();
    }
    w.finish().unwrap();
}

/// A well-formed layout declaration for the tests' runtime root
/// (spec 18 C3 — `docs/spec/schemas/layout.yaml`).
const GOOD_LAYOUT: &str = "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"3.4\"\n";

/// The env-image fixture: the runtime's files + the spec-18 layout
/// declaration (an era-2 image — the layout check gates every boot).
fn write_env_image(dir: &Path) -> PathBuf {
    write_env_image_with_layout(dir, Some(GOOD_LAYOUT))
}

/// The env-image fixture with a custom (or absent) layout declaration —
/// the S17–S19 refusal fixtures.
fn write_env_image_with_layout(dir: &Path, layout: Option<&str>) -> PathBuf {
    let p = dir.join("runtime.tfs");
    let mut files: Vec<(&str, &[u8])> =
        vec![("lib/ruby/rubygems.rb", b"# rubygems core\n".as_slice())];
    if let Some(text) = layout {
        files.push(("lib/tebako/layout.yaml", text.as_bytes()));
    }
    build_zip(&p, &["lib/", "lib/ruby/", "lib/tebako/"], &files);
    p
}

/// The app-payload fixture: an entrypoint and a lib.
fn write_payload_image(dir: &Path) -> PathBuf {
    let p = dir.join("payload.tfs");
    build_zip(
        &p,
        &["bin/", "lib/"],
        &[
            ("bin/app", b"#!/usr/bin/env ruby\nputs 'hi'\n".as_slice()),
            ("lib/app.rb", b"# app\n".as_slice()),
        ],
    );
    p
}

/// Append a single-slot tpkg trailer to `path` (slot 0 = the whole
/// file), turning the bare image into a package.
fn package_in_place(path: &Path, format_id: u32) {
    let size = std::fs::metadata(path).unwrap().len();
    let mut m = tpkg::Manifest {
        package_flags: tpkg::TPKG_FLAG_LEAN,
        launcher_abi: 1,
        ..Default::default()
    };
    m.slots.push(tpkg::Slot::new(0, size, format_id, "/"));
    m.validate().unwrap();
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    tpkg::write_to(&mut f, &m).unwrap();
}

fn argv(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn read_file(path: &str) -> Vec<u8> {
    let mut ctx = context().write().unwrap();
    let fd = ctx.open(path, libc::O_RDONLY).expect("open in VFS");
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = ctx.read(fd, &mut buf).expect("read in VFS");
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
    }
    ctx.close(fd).expect("close in VFS");
    out
}

// ---------------------------------------------------------------------
// the tests
// ---------------------------------------------------------------------

#[test]
fn plain_boot_passes_argv_through_and_mounts_the_env_image() {
    let g = guard("plain");
    let env_image = write_env_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let out = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();
    assert_eq!(out.argv, argv(&["ruby", "--version"]));
    assert!(context().read().unwrap().is_mounted());
    let bytes = read_file("/__tfs__/lib/ruby/rubygems.rb");
    assert_eq!(bytes, b"# rubygems core\n");
}

#[test]
fn a_boot_without_the_env_image_is_legal_and_mounts_nothing() {
    let _g = guard("no-env-image");
    let env = MapEnv::new(); // no TEBAKO_RUNTIME_IMAGE — the warn's shape

    // The absence is named on stderr (an eprintln — process-global, not
    // captured here); what this pins is that the warn never became an
    // error: the bare boot stays a legal shape and nothing mounts.
    let out = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();
    assert_eq!(out.argv, argv(&["ruby", "--version"]));
    assert!(!context().read().unwrap().is_mounted());
}

// ---------------------------------------------------------------------
// the env-image slot form (spec 23 §13.1 — the two-slot carried pair)
// ---------------------------------------------------------------------

/// The carried pair's wire shape: dummy bootstrap bytes at slot 0, the
/// env image at slot 1 (both format AUTO — the orthogonality law), then
/// the trailer.
fn write_two_slot_package(dir: &Path) -> PathBuf {
    let env_image = write_env_image(dir);
    let env_bytes = std::fs::read(&env_image).unwrap();
    let exe = b"dummy bootstrap bytes (not an executable)";
    let p = dir.join("app.tpkg");
    let mut m = tpkg::Manifest {
        package_flags: 0,
        launcher_abi: 1,
        ..Default::default()
    };
    m.slots.push(tpkg::Slot::new(
        0,
        exe.len() as u64,
        tpkg::TPKG_FORMAT_AUTO,
        "",
    ));
    m.slots.push(tpkg::Slot::new(
        exe.len() as u64,
        env_bytes.len() as u64,
        tpkg::TPKG_FORMAT_AUTO,
        "",
    ));
    m.validate().unwrap();
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(exe).unwrap();
    f.write_all(&env_bytes).unwrap();
    tpkg::write_to(&mut f, &m).unwrap();
    p
}

#[test]
fn the_env_image_slot_form_mounts_the_package_region() {
    let g = guard("env-slot");
    let pkg = write_two_slot_package(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", format!("{}:1", pkg.display()));

    let out = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();
    assert_eq!(out.argv, argv(&["ruby", "--version"]));
    let bytes = read_file("/__tfs__/lib/ruby/rubygems.rb");
    assert_eq!(bytes, b"# rubygems core\n");
    // The slot identity rides the mount: a spawned child's
    // TEBAKO_TFS_MOUNTS re-mounts this REGION, never the whole package
    // (spec 17 §2.1's emit rule).
    let mounts = context().read().unwrap().mounts_env().unwrap();
    let mounts = mounts.to_string_lossy().into_owned();
    assert!(
        mounts.contains(&format!("{}:1:/__tfs__", pkg.display())),
        "{mounts}"
    );
}

#[test]
fn the_slot_form_names_an_out_of_range_slot() {
    let g = guard("env-slot-range");
    let pkg = write_two_slot_package(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", format!("{}:5", pkg.display()));
    let err = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("out of range"), "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn the_slot_form_refuses_a_runtime_slot_by_name() {
    let g = guard("env-slot-runtime");
    let image = write_env_image(g.path());
    package_in_place(&image, tpkg::TPKG_FORMAT_RUNTIME);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", format!("{}:0", image.display()));
    let err = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("runtime payload slot"), "{}", err.message);
}

#[test]
fn a_bare_image_spelled_with_slot_zero_mounts_whole() {
    let g = guard("env-slot-zero");
    let image = write_env_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", format!("{}:0", image.display()));
    let out = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();
    assert_eq!(out.argv, argv(&["ruby", "--version"]));
    let bytes = read_file("/__tfs__/lib/ruby/rubygems.rb");
    assert_eq!(bytes, b"# rubygems core\n");
}

// ---------------------------------------------------------------------
// the trace channel (spec 25 §2, phase T1)
// ---------------------------------------------------------------------

#[test]
fn tebako_trace_env_arms_the_bus_before_any_mount() {
    let g = guard("trace-env");
    let env_image = write_env_image(g.path());
    let capture = g.path().join("trace.jsonl");
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("TEBAKO_TRACE", capture.display().to_string());

    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();

    // The channel opened at boot, BEFORE the env image mounted — so the
    // mount decision itself is in the capture.
    let text = std::fs::read_to_string(&capture).expect("the boot opened the channel");
    let mount = text
        .lines()
        .find(|l| l.contains("\"op\":\"mount\""))
        .unwrap_or_else(|| panic!("a mount event was traced: {text}"));
    assert!(mount.contains("\"verdict\":\"ok\""), "{mount}");
    assert!(mount.contains("/__tfs__"), "{mount}");
    assert!(tfs::trace::armed(), "the bus stays armed for the run");
}

#[test]
fn tebako_trace_argument_wins_over_the_env() {
    let g = guard("trace-arg");
    let env_capture = g.path().join("env.jsonl");
    let arg_capture = g.path().join("arg.jsonl");
    let mut env = MapEnv::new();
    env.set("TEBAKO_TRACE", env_capture.display().to_string());

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-trace",
            &arg_capture.display().to_string(),
            "--version",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    assert!(arg_capture.is_file(), "the argument's channel opened");
    assert!(
        !env_capture.exists(),
        "the env channel never opened — the argument wins"
    );
    // The consumed loader flag never leaks into the interpreter's argv.
    assert_eq!(out.argv, argv(&["ruby", "--version"]));
}

#[test]
fn a_broken_trace_channel_only_notes_and_the_boot_proceeds() {
    let g = guard("trace-broken");
    let blocker = g.path().join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let mut env = MapEnv::new();
    // The channel's parent is a regular FILE: arm() cannot open it —
    // one loud stderr note, the run proceeds (observability never gates).
    env.set(
        "TEBAKO_TRACE",
        blocker.join("t.jsonl").display().to_string(),
    );

    let out = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();
    assert_eq!(out.argv, argv(&["ruby", "--version"]));
    assert!(!tfs::trace::armed(), "the bus stayed disarmed");
}

// ---------------------------------------------------------------------
// the resolve channel (spec 25 §2, phase T2): one `resolve` event per
// --tebako-image triple, emitted BEFORE the mount it feeds. The env
// image has no slot decision — it emits no resolve event.
// ---------------------------------------------------------------------

/// The capture's `resolve` events, parsed (the bus render shape).
fn resolve_events(capture: &Path) -> Vec<tebako_json::Value> {
    let text = std::fs::read_to_string(capture).expect("read the capture");
    text.lines()
        .map(|l| tebako_json::parse(l).expect("each capture line parses"))
        .filter(|d| {
            d.find("op")
                .and_then(tebako_json::Value::as_string)
                .as_deref()
                == Some("resolve")
        })
        .collect()
}

fn field<'v>(doc: &'v tebako_json::Value, key: &str) -> &'v tebako_json::Value {
    doc.find(key)
        .unwrap_or_else(|| panic!("`{key}` present in {doc:?}"))
}

fn detail<'v>(doc: &'v tebako_json::Value, key: &str) -> &'v tebako_json::Value {
    field(doc, "detail")
        .find(key)
        .unwrap_or_else(|| panic!("detail `{key}` present in {doc:?}"))
}

#[test]
fn resolve_events_cover_the_whole_and_slot_decisions() {
    let g = guard("resolve-ok");
    let env_image = write_env_image(g.path());
    let bare = write_payload_image(g.path());
    let packaged = g.path().join("packaged.tfs");
    build_zip(&packaged, &["tools/"], &[("tools/x", b"x\n".as_slice())]);
    package_in_place(&packaged, tpkg::TPKG_FORMAT_ZIP);
    let capture = g.path().join("trace.jsonl");
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("TEBAKO_TRACE", capture.display().to_string());

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", bare.display()),
            "--tebako-image",
            &format!("{}:0:/opt/tool", packaged.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    let events = resolve_events(&capture);
    assert_eq!(
        events.len(),
        2,
        "one resolve event per triple — the env image has no slot decision: {events:?}"
    );

    // The bare image: the whole-file decision, slot as spelled.
    let whole = &events[0];
    assert_eq!(
        field(whole, "path").as_string().as_deref(),
        Some(bare.to_str().unwrap())
    );
    assert_eq!(
        field(whole, "verdict").as_string().as_deref(),
        Some("whole")
    );
    assert_eq!(detail(whole, "slot").as_string().as_deref(), Some("0"));
    assert_eq!(detail(whole, "mount").as_string().as_deref(), Some("/"));
    assert_eq!(
        field(whole, "pid").as_u64(),
        Some(u64::from(std::process::id())),
        "the resolve event rides the envelope's pid"
    );

    // The package: the slot region — the payload/slot identity the
    // correlator matches the outside capture's byte-range reads against.
    let slot = &events[1];
    assert_eq!(
        field(slot, "path").as_string().as_deref(),
        Some(packaged.to_str().unwrap())
    );
    assert_eq!(
        field(slot, "verdict").as_string().as_deref(),
        Some("slot:0")
    );
    assert_eq!(detail(slot, "offset").as_u64(), Some(0));
    assert!(
        detail(slot, "size").as_u64().unwrap_or(0) > 0,
        "the region's byte size: {slot:?}"
    );
    assert_eq!(detail(slot, "slots").as_u64(), Some(1));
    assert_eq!(
        detail(slot, "mount").as_string().as_deref(),
        Some("/opt/tool")
    );

    // Each triple's resolve decision precedes the mount it feeds (the
    // schema's ordering note): the resolve event naming the image lands
    // before the mount event whose image detail names it. (The env
    // image's mount comes first overall — it has no resolve event.)
    let text = std::fs::read_to_string(&capture).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let at = |op: &str, image: &str| {
        lines
            .iter()
            .position(|l| l.contains(&format!("\"op\":\"{op}\"")) && l.contains(image))
            .unwrap_or_else(|| panic!("a {op} event naming {image}: {text}"))
    };
    for image in [bare.display().to_string(), packaged.display().to_string()] {
        assert!(
            at("resolve", &image) < at("mount", &image),
            "resolve precedes the mount for {image}: {text}"
        );
    }
}

/// One failing boot with the bus armed; returns the resolve events.
fn resolve_failure_capture(
    g: &Guard,
    tag: &str,
    triple: &str,
) -> (tebako_driver::DriverError, Vec<tebako_json::Value>) {
    let capture = g.path().join(format!("{tag}.jsonl"));
    let mut env = MapEnv::new();
    env.set("TEBAKO_TRACE", capture.display().to_string());
    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            triple,
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    (err, resolve_events(&capture))
}

#[test]
fn resolve_errors_carry_the_errno_and_the_failure_class() {
    let g = guard("resolve-err");
    let bare = write_payload_image(g.path());
    let packaged = g.path().join("packaged.tfs");
    build_zip(&packaged, &["tools/"], &[("tools/x", b"x\n".as_slice())]);
    package_in_place(&packaged, tpkg::TPKG_FORMAT_ZIP);
    let runtime_slotted = g.path().join("runtime.tfs");
    build_zip(
        &runtime_slotted,
        &["tools/"],
        &[("tools/x", b"x\n".as_slice())],
    );
    package_in_place(&runtime_slotted, tpkg::TPKG_FORMAT_RUNTIME);

    // (triple, boot exit code, errno, reason): the named boot errors are
    // unchanged (65 manifest / 69 unavailable); the event carries the
    // errno-level fact and the class.
    let cases: Vec<(String, i32, i32, &str)> = vec![
        (
            format!("{}:3:/", bare.display()),
            65,
            libc::EINVAL,
            "no-trailer",
        ),
        (
            format!("{}:3:/", packaged.display()),
            65,
            libc::ERANGE,
            "slot-out-of-range",
        ),
        (
            format!("{}:-:/", packaged.display()),
            65,
            libc::EINVAL,
            "whole-on-package",
        ),
        (
            format!("{}:0:/", runtime_slotted.display()),
            65,
            libc::EINVAL,
            "runtime-slot",
        ),
        (
            format!("{}:0:/", g.path().join("nope.tfs").display()),
            69,
            libc::ENOENT,
            "open",
        ),
    ];
    for (i, (triple, code, errno, reason)) in cases.iter().enumerate() {
        let (err, events) = resolve_failure_capture(&g, &format!("case{i}"), triple);
        assert_eq!(err.code, *code, "{triple}: {}", err.message);
        assert_eq!(events.len(), 1, "{triple}: one resolve event: {events:?}");
        let event = &events[0];
        let verdict = format!("error:{errno}");
        assert_eq!(
            field(event, "verdict").as_string().as_deref(),
            Some(verdict.as_str()),
            "{triple}"
        );
        // The schema's typed-duplicate rule: the errno field IS the
        // verdict suffix.
        assert_eq!(
            field(event, "errno").as_u64(),
            Some(*errno as u64),
            "{triple}"
        );
        assert_eq!(
            detail(event, "reason").as_string().as_deref(),
            Some(*reason),
            "{triple}"
        );
        assert_eq!(detail(event, "mount").as_string().as_deref(), Some("/"));
        assert!(
            !context().read().unwrap().is_mounted(),
            "{triple}: a refused boot leaves nothing mounted"
        );
    }
}

#[test]
fn a_self_triple_without_a_trailer_traces_the_probe_and_the_refusal() {
    let g = guard("resolve-self");
    // The test executable carries no tpkg trailer: `self:0` probes clean
    // (a bare file mounts whole for a FILE triple) and the self-slot
    // rule then refuses — two resolve events, one decision each.
    let (err, events) = resolve_failure_capture(&g, "self", "self:0:/");
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("carries no tpkg trailer"),
        "{}",
        err.message
    );
    assert_eq!(events.len(), 2, "the probe + the refusal: {events:?}");
    assert_eq!(
        field(&events[0], "verdict").as_string().as_deref(),
        Some("whole")
    );
    assert_eq!(
        field(&events[1], "verdict").as_string().as_deref(),
        Some(format!("error:{}", libc::EINVAL).as_str())
    );
    assert_eq!(
        detail(&events[1], "reason").as_string().as_deref(),
        Some("self-not-packaged")
    );
    // The event's path is the file actually probed: the running exe.
    let exe = std::env::current_exe().unwrap();
    assert_eq!(
        field(&events[0], "path").as_string().as_deref(),
        Some(exe.to_str().unwrap())
    );
}

#[test]
fn bare_payload_mounts_whole_with_slot_0() {
    let g = guard("bare-0");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
            "--version",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    assert_eq!(out.argv, argv(&["ruby", "/bin/app", "--version"]));
    let bytes = read_file("/bin/app");
    assert_eq!(bytes, b"#!/usr/bin/env ruby\nputs 'hi'\n");
}

#[test]
fn bare_payload_mounts_whole_with_dash() {
    let g = guard("bare-dash");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/opt/app", payload.display()),
            "--tebako-entry",
            "bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    assert_eq!(out.argv, argv(&["ruby", "/opt/app/bin/app"]));
    assert_eq!(read_file("/opt/app/lib/app.rb"), b"# app\n");
}

#[test]
fn env_image_plus_payload_coexist_at_nested_points() {
    let g = guard("coexist");
    let env_image = write_env_image(g.path());
    let payload = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    assert_eq!(out.argv, argv(&["ruby", "/bin/app"]));
    // Both mounts live: the env image at its root, the payload at /.
    assert_eq!(
        read_file("/__tfs__/lib/ruby/rubygems.rb"),
        b"# rubygems core\n"
    );
    assert_eq!(read_file("/bin/app"), b"#!/usr/bin/env ruby\nputs 'hi'\n");
}

#[test]
fn slot_beyond_zero_on_a_bare_image_is_a_named_error_and_rolls_back() {
    let g = guard("bare-n");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:3:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("out of range"), "{}", err.message);
    assert!(
        !context().read().unwrap().is_mounted(),
        "a failed boot leaves nothing mounted"
    );
}

#[test]
fn packaged_file_mounts_the_slot_region() {
    let g = guard("pkg-slot");
    let payload = write_payload_image(g.path());
    package_in_place(&payload, tpkg::TPKG_FORMAT_ZIP);
    let env = MapEnv::new();

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    assert_eq!(out.argv, argv(&["ruby", "/bin/app"]));
    assert_eq!(read_file("/bin/app"), b"#!/usr/bin/env ruby\nputs 'hi'\n");
}

#[test]
fn a_slot_mounts_respawn_env_carries_the_slot_form() {
    // spec 17 §2.1's emit rule (tebako#455): a mount established from a
    // package slot serializes as image:slot:mount in TEBAKO_TFS_MOUNTS,
    // so a spawned child re-mounts the region — never the whole package
    // file (whose trailer the child's format sniff would hit).
    let g = guard("pkg-slot-env");
    let payload = write_payload_image(g.path());
    package_in_place(&payload, tpkg::TPKG_FORMAT_ZIP);
    let env = MapEnv::new();

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    let mounts = context().read().unwrap().mounts_env().unwrap();
    assert_eq!(
        mounts.to_string_lossy(),
        format!("{}:0:/", payload.display())
    );
}

#[test]
fn no_entry_starts_the_interpreter_with_its_own_args() {
    let g = guard("no-entry");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--version",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    // The bare `--tebako-image` invocation (the deploy-driver smoke):
    // the interpreter starts with its own args; the mount is live.
    assert_eq!(out.argv, argv(&["ruby", "--version"]));
    let bytes = read_file("/bin/app");
    assert_eq!(bytes, b"#!/usr/bin/env ruby\nputs 'hi'\n");
}

#[test]
fn the_interpreter_keyword_is_dropped() {
    let g = guard("keyword");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "ruby",
            "-e",
            "puts 1",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    // `--tebako-entry ruby` (the CLI's deploy shims): the keyword is
    // dropped; the interpreter starts with the user's args.
    assert_eq!(out.argv, argv(&["ruby", "-e", "puts 1"]));
    let bytes = read_file("/bin/app");
    assert_eq!(bytes, b"#!/usr/bin/env ruby\nputs 'hi'\n");
}

#[test]
fn dash_on_a_packaged_file_is_a_named_error() {
    let g = guard("pkg-dash");
    let payload = write_payload_image(g.path());
    package_in_place(&payload, tpkg::TPKG_FORMAT_ZIP);
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("is a package"), "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn runtime_role_slot_is_never_mounted() {
    let g = guard("pkg-runtime");
    let payload = write_payload_image(g.path());
    package_in_place(&payload, tpkg::TPKG_FORMAT_RUNTIME);
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("runtime payload slot"),
        "{}",
        err.message
    );
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn images_without_entry_starts_the_interpreter_bare() {
    let g = guard("no-entry-bare");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    // The smoke form: mounts established, the interpreter starts bare.
    assert_eq!(out.argv, argv(&["ruby"]));
    assert!(context().read().unwrap().is_mounted());
}

#[test]
fn missing_entry_is_named_65_and_rolls_back() {
    let g = guard("bad-entry");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/nope",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("not found"), "{}", err.message);
    assert!(
        !context().read().unwrap().is_mounted(),
        "the payload mount rolls back with the failure"
    );
}

#[test]
fn malformed_triple_is_named_65() {
    let g = guard("malformed");
    let env = MapEnv::new();
    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            "/x/y.tfs:/",
            "--tebako-entry",
            "/x",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    let _ = g;
}

#[test]
fn missing_image_file_is_named_69() {
    let g = guard("missing-image");
    let env = MapEnv::new();
    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", g.path().join("nope.tfs").display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 69, "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn jail_deny_blocks_host_reads_but_not_the_payload() {
    let g = guard("jail");
    let payload = write_payload_image(g.path());
    let host_file = g.path().join("secret.txt");
    std::fs::write(&host_file, b"host secret\n").unwrap();
    let journal = g.path().join("journal.log");
    // The journal path is process-env (tfs-internal); point it at the
    // scratch dir for this test only.
    std::env::set_var("TEBAKO_JAIL_JOURNAL", &journal);
    let mut env = MapEnv::new();
    env.set("TEBAKO_JAIL", "deny");

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/opt/app", payload.display()),
            "--tebako-entry",
            "bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    assert_eq!(out.argv, argv(&["ruby", "/opt/app/bin/app"]));
    // The payload reads fine…
    assert_eq!(
        read_file("/opt/app/bin/app"),
        b"#!/usr/bin/env ruby\nputs 'hi'\n"
    );
    // …and a host path outside every mount is denied (EPERM), not served.
    let mut ctx = context().write().unwrap();
    let err = ctx
        .open(&host_file.display().to_string(), libc::O_RDONLY)
        .unwrap_err();
    assert_eq!(err, libc::EPERM);
    drop(ctx);
    std::env::remove_var("TEBAKO_JAIL_JOURNAL");
}

#[test]
fn malformed_jail_is_named_73_and_rolls_back() {
    let g = guard("jail-bad");
    let payload = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_JAIL", "not-a-policy");

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 73, "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
    let _ = g;
}

// ---------------------------------------------------------------------
// mount modes (spec 17 §1, locked 2026-08-04): an occupied point is
// governed by the L2 mounts block — exclusive is the named EEXIST it
// always was, union merges the trees. The shipped boot() reads the
// running package's own trailer (the test exe carries none → every
// spec answers "no row"); the union cases ride boot_with_mount_modes
// with a stub row source.
// ---------------------------------------------------------------------

/// A fixed L2-row answer for every triple (the union/exclusive cases).
struct StubModes(Option<tpkg::PackageMount>);

impl tebako_driver::MountModes for StubModes {
    fn row_for(
        &self,
        _spec: &tebako_driver::ImageSpec,
        _trailer: Option<&tpkg::Manifest>,
    ) -> Result<Option<tpkg::PackageMount>, tebako_driver::DriverError> {
        Ok(self.0.clone())
    }
}

fn union_row() -> tpkg::PackageMount {
    tpkg::PackageMount {
        slot: 0,
        point: "/__tfs__".to_string(),
        mode: tpkg::MountMode::Union,
        precedence: Some(tpkg::Precedence::AfterEnv),
    }
}

/// A trailer carrying the given L2 block as raw YAML (the production
/// row extraction's input; hand-authored so reserved spellings reach
/// the parser — `set_package_manifest` validates at write time).
fn trailer_with_l2(yaml: &str) -> tpkg::Manifest {
    let mut m = tpkg::Manifest {
        package_flags: tpkg::TPKG_FLAG_LEAN,
        launcher_abi: 1,
        ..Default::default()
    };
    m.slots.push(tpkg::Slot::new(
        0,
        100,
        tpkg::TPKG_FORMAT_DWARFS,
        "/__tfs__",
    ));
    m.insert_ext_block(
        tpkg::ExtBlock::new(
            tpkg::TPKG_EXT_TYPE_PACKAGE_MANIFEST,
            yaml.as_bytes().to_vec(),
        )
        .unwrap(),
    )
    .unwrap();
    m
}

const L2_HEADER: &str = "schema_version: 1\n\
                         package: {name: x, version: 1.0.0, producer: {tool: t, tool_version: 1}, created: now}\n\
                         entries:\n  - {name: x, slot: 0, entrypoint: /local/stub.rb, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n";

#[test]
fn own_trailer_source_reads_the_row_of_the_mounted_package() {
    // The production source: the L2 row comes from the trailer of the
    // very file the triple mounts (spec 17 §1 — `<self>` spelled as a
    // path is the same file).
    let trailer = trailer_with_l2(&format!(
        "{L2_HEADER}mounts:\n  - {{slot: 0, point: /__tfs__, mode: union, precedence: after-env}}\n"
    ));
    let file = tebako_driver::ImageSpec {
        source: tebako_driver::ImageSource::File(
            PathBuf::from("/x/pkg"),
            tebako_driver::SlotRef::Slot(0),
        ),
        mount: "/__tfs__".to_string(),
    };
    let row = tebako_driver::OwnTrailer
        .row_for(&file, Some(&trailer))
        .unwrap()
        .unwrap();
    assert_eq!(row.mode, tpkg::MountMode::Union);
    assert_eq!(row.precedence, Some(tpkg::Precedence::AfterEnv));
    // A slot without a row answers exclusive; a bare image (no
    // trailer) and a whole-file mount answer exclusive too.
    let other_slot = tebako_driver::ImageSpec {
        source: tebako_driver::ImageSource::File(
            PathBuf::from("/x/pkg"),
            tebako_driver::SlotRef::Slot(1),
        ),
        mount: "/__tfs__".to_string(),
    };
    assert_eq!(
        tebako_driver::OwnTrailer
            .row_for(&other_slot, Some(&trailer))
            .unwrap(),
        None
    );
    assert_eq!(
        tebako_driver::OwnTrailer.row_for(&file, None).unwrap(),
        None
    );
    let whole = tebako_driver::ImageSpec {
        source: tebako_driver::ImageSource::File(
            PathBuf::from("/x/bare.tfs"),
            tebako_driver::SlotRef::Whole,
        ),
        mount: "/__tfs__".to_string(),
    };
    assert_eq!(
        tebako_driver::OwnTrailer
            .row_for(&whole, Some(&trailer))
            .unwrap(),
        None
    );
}

#[test]
fn own_trailer_source_names_the_reserved_modes() {
    // `mode: cow` never parses into a valid L2 block — the driver's
    // answer is the named validation error, never a silent exclusive.
    let trailer = trailer_with_l2(&format!(
        "{L2_HEADER}mounts:\n  - {{slot: 0, point: /__tfs__, mode: cow}}\n"
    ));
    let file = tebako_driver::ImageSpec {
        source: tebako_driver::ImageSource::File(
            PathBuf::from("/x/pkg"),
            tebako_driver::SlotRef::Slot(0),
        ),
        mount: "/__tfs__".to_string(),
    };
    let err = tebako_driver::OwnTrailer
        .row_for(&file, Some(&trailer))
        .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("mode 'cow' is reserved"),
        "{}",
        err.message
    );
}

/// A payload zip that shares the env image's root AND one of its files.
fn write_shadowing_payload(dir: &Path) -> PathBuf {
    let p = dir.join("shadow.tfs");
    build_zip(
        &p,
        &["bin/", "lib/", "lib/ruby/", "local/"],
        &[
            ("bin/app", b"#!/usr/bin/env ruby\nputs 'hi'\n".as_slice()),
            ("local/stub.rb", b"load \"/__tfs__/bin/app\"\n".as_slice()),
            (
                "lib/ruby/rubygems.rb",
                b"# app-shadowed rubygems\n".as_slice(),
            ),
        ],
    );
    p
}

#[test]
fn occupied_point_without_a_row_is_the_named_eexist() {
    let g = guard("occupied-eexist");
    let env_image = write_env_image(g.path());
    let payload = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    // The env image owns /__tfs__; a payload triple onto the same point
    // with no L2 row is the historical named error (the shipped boot —
    // file triples are always exclusive).
    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/__tfs__", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("duplicate mount point"),
        "{}",
        err.message
    );
    assert!(
        !context().read().unwrap().is_mounted(),
        "a refused boot leaves nothing mounted"
    );
}

#[test]
fn an_exclusive_row_keeps_the_named_eexist() {
    let g = guard("exclusive-row");
    let env_image = write_env_image(g.path());
    let payload = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let modes = StubModes(Some(tpkg::PackageMount {
        slot: 0,
        point: "/__tfs__".to_string(),
        mode: tpkg::MountMode::Exclusive,
        precedence: None,
    }));

    let err = tebako_driver::boot_with_mount_modes(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/__tfs__", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
        &modes,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("duplicate mount point"),
        "{}",
        err.message
    );
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn union_row_merges_the_trees_at_the_runtime_root() {
    let g = guard("union-merge");
    let env_image = write_env_image(g.path());
    let payload = write_shadowing_payload(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let out = tebako_driver::boot_with_mount_modes(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/__tfs__", payload.display()),
            "--tebako-entry",
            "/local/stub.rb",
        ]),
        "/__tfs__",
        &env,
        &StubModes(Some(union_row())),
    )
    .unwrap();
    // The entry resolves against the first image's mount — through the
    // union (the app member holds local/stub.rb).
    assert_eq!(out.argv, argv(&["ruby", "/__tfs__/local/stub.rb"]));
    // Both members read through; the app member shadows the shared file.
    assert_eq!(
        read_file("/__tfs__/local/stub.rb"),
        b"load \"/__tfs__/bin/app\"\n"
    );
    assert_eq!(
        read_file("/__tfs__/lib/ruby/rubygems.rb"),
        b"# app-shadowed rubygems\n"
    );
    // Directories merge: the env member's lib/tebako rides alongside.
    let mut ctx = context().write().unwrap();
    let dir = ctx.opendir("/__tfs__/lib").unwrap();
    let mut seen = Vec::new();
    while ctx.readdir_abi(dir).unwrap() {
        let cur = ctx.dir_current(dir).unwrap();
        let len = cur
            .d_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(cur.d_name.len());
        seen.push(
            cur.d_name[..len]
                .iter()
                .map(|&c| c as u8 as char)
                .collect::<String>(),
        );
    }
    ctx.closedir(dir).unwrap();
    drop(ctx);
    seen.sort();
    assert_eq!(seen, vec!["ruby", "tebako"]);
}

// ---------------------------------------------------------------------
// The uniform VFS namespace on windows roots (spec 17 §1): declared
// mounts qualify onto the runtime root's drive before any mount/entry
// computation.
// ---------------------------------------------------------------------

/// The env fixture's layout declaration with the windows root spelling.
fn write_env_image_windows_root(dir: &Path) -> PathBuf {
    write_env_image_with_layout(dir, Some(&GOOD_LAYOUT.replace("/__tfs__", "A:/t")))
}

#[test]
fn windows_root_qualifies_declared_mounts_onto_the_vfs_drive() {
    let g = guard("win-qualify");
    let env_image = write_env_image_windows_root(g.path());
    let payload = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let out = boot(
        &argv(&[
            "ruby.exe",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
            "--version",
        ]),
        "A:/t",
        &env,
    )
    .unwrap();
    // The declared `/` mount qualified onto the VFS drive; the entry
    // resolves drive-qualified, so ruby's C-level expansion can never
    // re-root it onto the cwd drive.
    assert_eq!(out.argv, argv(&["ruby.exe", "A:/bin/app", "--version"]));
    // Both mounts live in the physical namespace: the env image at the
    // qualified root, the payload at the drive root.
    assert_eq!(read_file("A:/t/lib/ruby/rubygems.rb"), b"# rubygems core\n");
    assert_eq!(read_file("A:/bin/app"), b"#!/usr/bin/env ruby\nputs 'hi'\n");
}

#[test]
fn windows_root_union_merges_at_the_qualified_root() {
    let g = guard("win-union");
    let env_image = write_env_image_windows_root(g.path());
    let payload = write_shadowing_payload(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let out = tebako_driver::boot_with_mount_modes(
        &argv(&[
            "ruby.exe",
            "--tebako-image",
            &format!("{}:0:/t", payload.display()),
            "--tebako-entry",
            "/local/stub.rb",
        ]),
        "A:/t",
        &env,
        &StubModes(Some(union_row())),
    )
    .unwrap();
    // The declared union target is the root's declared form, so the
    // qualified points coincide and the union row governs.
    assert_eq!(out.argv, argv(&["ruby.exe", "A:/t/local/stub.rb"]));
    assert_eq!(
        read_file("A:/t/local/stub.rb"),
        b"load \"/__tfs__/bin/app\"\n"
    );
    assert_eq!(
        read_file("A:/t/lib/ruby/rubygems.rb"),
        b"# app-shadowed rubygems\n"
    );
}

// ---------------------------------------------------------------------
// TEBAKO_MOUNT_ROOT (spec 17 §1): the run-time root override — validated
// at boot (exit 65), gated on the env image's mount_root_override grant
// post-mount (exit 78).
// ---------------------------------------------------------------------

#[test]
fn mount_root_override_redirects_the_env_mount() {
    let g = guard("root-override");
    let env_image = write_env_image_with_layout(
        g.path(),
        Some(&format!("{GOOD_LAYOUT}mount_root_override: true\n")),
    );
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("TEBAKO_MOUNT_ROOT", "/rt");

    let out = boot(&argv(&["ruby"]), "/__tfs__", &env).unwrap();
    assert_eq!(out.argv, argv(&["ruby"]));
    // The env image mounted at the override, never at the baked root.
    assert_eq!(read_file("/rt/lib/ruby/rubygems.rb"), b"# rubygems core\n");
    let mut ctx = context().write().unwrap();
    assert!(ctx
        .open("/__tfs__/lib/ruby/rubygems.rb", libc::O_RDONLY)
        .is_err());
}

#[test]
fn mount_root_override_requires_the_images_grant() {
    let g = guard("root-override-refused");
    let env_image = write_env_image(g.path()); // GOOD_LAYOUT: no grant key
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("TEBAKO_MOUNT_ROOT", "/rt");

    let err = boot(&argv(&["ruby"]), "/__tfs__", &env).unwrap_err();
    assert_eq!(err.code, 78, "{err}");
    assert!(err.message.contains("TEBAKO_MOUNT_ROOT"), "{err}");
    // The refusal rolled the mount back (never a partial mount).
    let mut ctx = context().write().unwrap();
    assert!(ctx
        .open("/rt/lib/tebako/layout.yaml", libc::O_RDONLY)
        .is_err());
}

#[test]
fn a_malformed_mount_root_override_is_a_named_error() {
    let _g = guard("root-override-malformed");
    let mut env = MapEnv::new();
    env.set("TEBAKO_MOUNT_ROOT", "relative/x");
    let err = boot(&argv(&["ruby"]), "/__tfs__", &env).unwrap_err();
    assert_eq!(err.code, 65, "{err}");
    assert!(err.message.contains("TEBAKO_MOUNT_ROOT"), "{err}");
}

#[test]
fn a_named_mode_source_error_surfaces_and_rolls_back() {
    let g = guard("modes-err");
    let env_image = write_env_image(g.path());
    let payload = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    // The reserved-mode refusal (cow/enc never parse into a valid L2
    // block — tpkg's validation names them) reaches the boot as the
    // mode source's named error.
    struct ReservedModes;
    impl tebako_driver::MountModes for ReservedModes {
        fn row_for(
            &self,
            _spec: &tebako_driver::ImageSpec,
            _trailer: Option<&tpkg::Manifest>,
        ) -> Result<Option<tpkg::PackageMount>, tebako_driver::DriverError> {
            Err(tebako_driver::DriverError::new(
                65,
                "invalid L2 package manifest in the mounted package: mounts[].mode 'cow' is reserved"
                    .to_string(),
            ))
        }
    }
    let err = tebako_driver::boot_with_mount_modes(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/__tfs__", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
        &ReservedModes,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("reserved"), "{}", err.message);
    assert!(
        !context().read().unwrap().is_mounted(),
        "a refused boot leaves nothing mounted"
    );
}

// ---------------------------------------------------------------------
// the env-image layout check (spec 18 C3 / S17–S19, exit 78)
// ---------------------------------------------------------------------

/// Boot the plain path against an env image carrying `layout`
/// (`Some(yaml)` / `None` for the absent case).
fn boot_with_layout(
    g: &Guard,
    layout: Option<&str>,
) -> Result<tebako_driver::BootOutcome, tebako_driver::DriverError> {
    let env_image = write_env_image_with_layout(g.path(), layout);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env)
}

#[test]
fn layout_absent_is_an_era1_refusal_naming_the_image() {
    // S17: no /lib/tebako/layout.yaml — era 1, never assumed.
    let g = guard("layout-absent");
    let err = boot_with_layout(&g, None).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(
        err.message.contains("pre-era image (era 1)"),
        "{}",
        err.message
    );
    assert!(
        err.message.contains("runtime.tfs"),
        "the image is named: {}",
        err.message
    );
    assert!(
        !context().read().unwrap().is_mounted(),
        "a refused boot leaves nothing mounted"
    );
}

#[test]
fn layout_newer_major_is_an_upgrade_refusal() {
    // S18: a newer schema MAJOR — upgrade, both versions named.
    let g = guard("layout-major");
    let yaml = GOOD_LAYOUT.replace("schema_version: 1", "schema_version: 2");
    let err = boot_with_layout(&g, Some(&yaml)).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("schema 2"), "{}", err.message);
    assert!(
        err.message.contains("upgrade your tebako"),
        "{}",
        err.message
    );
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn layout_newer_era_and_generation_are_upgrade_refusals() {
    let g = guard("layout-newer");
    // era newer than the driver speaks
    let yaml = GOOD_LAYOUT.replace("era: 2", "era: 3");
    let err = boot_with_layout(&g, Some(&yaml)).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("era 3"), "{}", err.message);
    assert!(err.message.contains("speaks era 2"), "{}", err.message);

    // image_layout generation newer than the driver speaks (same guard —
    // one LOCK acquisition per test)
    let yaml = GOOD_LAYOUT.replace("image_layout: 1", "image_layout: 2");
    let err = boot_with_layout(&g, Some(&yaml)).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("generation 2"), "{}", err.message);
    assert!(
        err.message.contains("upgrade your tebako"),
        "{}",
        err.message
    );
}

#[test]
fn layout_declared_era1_is_refused_like_the_undeclared() {
    let g = guard("layout-era1");
    let yaml = GOOD_LAYOUT.replace("era: 2", "era: 1");
    let err = boot_with_layout(&g, Some(&yaml)).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("pre-era"), "{}", err.message);
}

#[test]
fn layout_mount_root_mismatch_is_78_with_both_values() {
    // S19: the image was built for another root — a mismatched pair,
    // never a ruby LoadError.
    let g = guard("layout-root");
    let yaml = GOOD_LAYOUT.replace("/__tfs__", "/__other__");
    let err = boot_with_layout(&g, Some(&yaml)).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("'/__other__'"), "{}", err.message);
    assert!(err.message.contains("'/__tfs__'"), "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn layout_malformed_is_a_named_78() {
    let g = guard("layout-malformed");
    let err = boot_with_layout(&g, Some("schema_version: [1\n")).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("malformed"), "{}", err.message);
}

#[test]
fn layout_check_gates_the_handoff_path_too() {
    // The handoff boot (payload + entry) runs the same check before any
    // payload mount or interpreter touch.
    let g = guard("layout-handoff");
    let env_image = write_env_image_with_layout(g.path(), None);
    let payload = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("pre-era image"), "{}", err.message);
    assert!(
        !context().read().unwrap().is_mounted(),
        "the env image rolls back with the refusal"
    );
}

// ---------------------------------------------------------------------
// spec 22 §6: the exec-cache export
// ---------------------------------------------------------------------

/// The store trust-anchor sidecar next to an image (the store layout:
/// `<image>.sha256`, sha256sum format).
fn write_sidecar(image: &Path, hex64: &str) {
    std::fs::write(
        format!("{}.sha256", image.display()),
        format!(
            "{hex64}  {}\n",
            image.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();
}

#[test]
fn boot_exports_the_exec_cache_root_keyed_by_the_env_image_sidecar() {
    let g = guard("exec-cache");
    let env_image = write_env_image(g.path());
    write_sidecar(&env_image, &"ab".repeat(32));
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();

    let cache = env
        .var("TEBAKO_EXEC_CACHE")
        .expect("the handoff env names the exec cache (spec 22 §6)");
    let want = std::env::temp_dir().join("tebako-exec-abababababababab");
    assert_eq!(Path::new(&cache), want.as_path());
}

#[test]
fn a_second_runtime_image_sha_never_reads_the_firsts_exec_cache() {
    // Rule L3 segregation: two boots whose env images carry different
    // shas name different cache roots — a rebuilt runtime's process
    // never reads the previous runtime's extraction namespace.
    let g = guard("exec-cache-l3");
    let image_a = write_env_image(g.path());
    write_sidecar(&image_a, &"aa".repeat(32));
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", image_a.display().to_string());
    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();
    let root_a = env
        .var("TEBAKO_EXEC_CACHE")
        .expect("the first boot exports the cache root");

    reset();
    let image_b = g.path().join("runtime-b.tfs");
    std::fs::copy(&image_a, &image_b).unwrap();
    write_sidecar(&image_b, &"bb".repeat(32));
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", image_b.display().to_string());
    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();
    let root_b = env
        .var("TEBAKO_EXEC_CACHE")
        .expect("the second boot exports the cache root");

    assert_ne!(root_a, root_b);
    assert!(root_a.contains(&"a".repeat(16)), "{root_a}");
    assert!(root_b.contains(&"b".repeat(16)), "{root_b}");
}

#[test]
fn a_boot_without_a_runtime_image_exports_the_host_keyed_cache() {
    let g = guard("exec-cache-host");
    let payload = write_payload_image(g.path());
    let env = MapEnv::new();

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    let cache = env
        .var("TEBAKO_EXEC_CACHE")
        .expect("a payload-only boot still names the exec cache");
    let want = std::env::temp_dir().join("tebako-exec-host");
    assert_eq!(Path::new(&cache), want.as_path());
}

// ---------------------------------------------------------------------
// spec 22 §6 + v2-1/20: the mount-discovery env
// ---------------------------------------------------------------------

#[test]
fn co_mounted_payloads_export_their_mount_vars() {
    let g = guard("mount-vars");
    let env_image = write_env_image(g.path());
    let app = write_payload_image(g.path());
    let jdk = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/", app.display()),
            "--tebako-image",
            &format!("{}:-:/tools/jdk", jdk.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    // The app at / exports nothing: TEBAKO_MOUNT_ROOT stays the spec-17
    // mount-root override, never a discovery var (the ffi suite's process
    // env is the regression net for the clobber).
    assert!(env.var("TEBAKO_MOUNT_ROOT").is_none());
    assert_eq!(
        env.var("TEBAKO_MOUNT_TOOLS_JDK").as_deref(),
        Some("/tools/jdk")
    );
}

#[test]
fn the_mount_var_values_are_windows_safe() {
    // The uniform namespace (spec 17 §1): declared mounts qualify onto
    // the runtime root's drive; the exported value is the physical
    // point, the slug stays the declared mechanical form.
    let g = guard("mount-vars-win");
    let env_image =
        write_env_image_with_layout(g.path(), Some(&GOOD_LAYOUT.replace("/__tfs__", "A:/t")));
    let jdk = write_payload_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/tools/jdk", jdk.display()),
        ]),
        "A:/t",
        &env,
    )
    .unwrap();

    assert_eq!(
        env.var("TEBAKO_MOUNT_TOOLS_JDK").as_deref(),
        Some("A:/tools/jdk")
    );
}

#[test]
fn a_slug_collision_is_a_named_boot_error() {
    let g = guard("mount-vars-collision");
    let a = write_payload_image(g.path());
    let b = write_payload_image(g.path());
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/a-b", a.display()),
            "--tebako-image",
            &format!("{}:-:/a/b", b.display()),
            "--tebako-entry",
            "/a-b/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("TEBAKO_MOUNT_A_B"), "{}", err.message);
    assert!(
        !context().read().unwrap().is_mounted(),
        "the refused composition unmounts everything"
    );
}

// ---------------------------------------------------------------------
// spec 22 §3 (Rules E2/E3): the child-injection env
// ---------------------------------------------------------------------

/// The env-image fixture carrying the preload shim (schema_minor 2): the
/// staged file plus its layout declaration.
fn write_env_image_with_shim(dir: &Path) -> PathBuf {
    let layout = format!("{GOOD_LAYOUT}preload_shim: lib/tebako/libtfs_preload.so\n");
    let p = dir.join("runtime.tfs");
    build_zip(
        &p,
        &["lib/", "lib/ruby/", "lib/tebako/"],
        &[
            ("lib/ruby/rubygems.rb", b"# rubygems core\n".as_slice()),
            ("lib/tebako/layout.yaml", layout.as_bytes()),
            (
                "lib/tebako/libtfs_preload.so",
                b"ELF pretend shim\n".as_slice(),
            ),
        ],
    );
    p
}

/// The platform's injection variable (the driver's INJECT_VAR).
#[cfg(target_os = "macos")]
const INJECT_VAR: &str = "DYLD_INSERT_LIBRARIES";
#[cfg(all(unix, not(target_os = "macos")))]
const INJECT_VAR: &str = "LD_PRELOAD";

#[cfg(unix)]
#[test]
fn a_declared_shim_is_materialized_and_armed_in_the_handoff_env() {
    let g = guard("inject");
    let env_image = write_env_image_with_shim(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();

    // The spawn hook's source: the VFS spelling, flowed from the image's
    // own declaration (SSOT — no hand-written copy anywhere).
    assert_eq!(
        env.var("TEBAKO_PRELOAD_SHIM").as_deref(),
        Some("/__tfs__/lib/tebako/libtfs_preload.so")
    );
    // The injection var names the MATERIALIZED copy (a real host file).
    let host = env
        .var(INJECT_VAR)
        .expect("the preload var rides the handoff env");
    let bytes = std::fs::read(&host).expect("the materialized shim exists");
    assert_eq!(bytes, b"ELF pretend shim\n");
    // The mounts list lets an injected child rebuild the namespace.
    let mounts = env.var("TEBAKO_TFS_MOUNTS").expect("the mounts list");
    assert!(
        mounts.contains(&format!("{}:/__tfs__", env_image.display())),
        "{mounts}"
    );
}

#[test]
fn a_declared_but_absent_shim_is_a_named_boot_error() {
    let g = guard("inject-lie");
    let layout = format!("{GOOD_LAYOUT}preload_shim: lib/tebako/libtfs_preload.so\n");
    // The declaration WITHOUT the file — the image lies.
    let env_image = {
        let p = g.path().join("runtime.tfs");
        build_zip(
            &p,
            &["lib/", "lib/ruby/", "lib/tebako/"],
            &[
                ("lib/ruby/rubygems.rb", b"# rubygems core\n".as_slice()),
                ("lib/tebako/layout.yaml", layout.as_bytes()),
            ],
        );
        p
    };
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let err = boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap_err();
    assert_eq!(err.code, 78, "{}", err.message);
    assert!(err.message.contains("declaration lies"), "{}", err.message);
    assert!(
        !context().read().unwrap().is_mounted(),
        "the refusal unmounts everything"
    );
}

#[test]
fn an_undeclared_image_arms_only_the_mounts_list() {
    let g = guard("inject-old");
    let env_image = write_env_image(g.path()); // no preload_shim key
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();

    assert!(env.var("TEBAKO_PRELOAD_SHIM").is_none());
    #[cfg(unix)]
    assert!(env.var(INJECT_VAR).is_none());
    let mounts = env.var("TEBAKO_TFS_MOUNTS").expect("the mounts list");
    assert!(mounts.contains(":/__tfs__"), "{mounts}");
}

// ---------------------------------------------------------------------
// spec 22 §3.2: the dependency mounts' declared bin dirs ride PATH
// ---------------------------------------------------------------------

/// A payload-manifest fixture (spec 03): kind-specific PROVIDES body.
fn payload_manifest(kind: &str, provides: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: {kind}\n  name: x\n  version: \"1\"\n  \
         producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-13T00:00:00Z\"\n  \
         digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
         signing: {{state: unsigned}}\n  encryption: {{state: none}}\n{provides}\n",
        z = "0".repeat(64)
    )
}

/// A toolkit-payload fixture: `bin/java` plus the in-image manifest
/// declaring it (spec 03 §2.2 — the openjdk shape).
fn write_toolkit_image(dir: &Path) -> PathBuf {
    write_toolkit_image_with_java(dir, b"#!/bin/sh\n")
}

/// The toolkit fixture with a caller-chosen `bin/java` body (the
/// launcher-tier run test makes it print a marker).
fn write_toolkit_image_with_java(dir: &Path, java: &[u8]) -> PathBuf {
    let manifest = payload_manifest(
        "toolkit",
        "provides:\n  executables:\n    - {name: java, path: /bin/java}\n  \
         platforms: [aarch64-macos]\n  capabilities: {exec: true, read: true}",
    );
    let p = dir.join("toolkit.tfs");
    build_zip(
        &p,
        &["bin/", "__tpkg__/"],
        &[
            ("bin/java", java),
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ],
    );
    p
}

fn joined_path(dirs: &[&str]) -> String {
    std::env::join_paths(dirs.iter().map(std::path::PathBuf::from))
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[test]
fn the_dependency_bin_dirs_prepend_path_in_triple_order() {
    let g = guard("path-env");
    let env_image = write_env_image(g.path());
    let payload = write_payload_image(g.path()); // no manifest: nothing declared
    let toolkit = write_toolkit_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("PATH", "/usr/bin:/bin");

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/app", payload.display()),
            "--tebako-image",
            &format!("{}:-:/opt/openjdk", toolkit.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    assert_eq!(
        env.var("PATH").as_deref(),
        Some(joined_path(&["/opt/openjdk/bin", "/usr/bin", "/bin"]).as_str())
    );
}

#[test]
fn the_app_payloads_own_bins_are_never_prepended() {
    let g = guard("path-env-first");
    let env_image = write_env_image(g.path());
    let app = write_toolkit_image(g.path()); // declares /bin, mounted FIRST
    let dep = write_toolkit_image(g.path());
    // A distinct bin for the dep so the assertions cannot conflate the two.
    let dep = {
        let _ = dep;
        let manifest = payload_manifest(
            "toolkit",
            "provides:\n  executables:\n    - {name: x, path: /sbin/x}\n  \
             platforms: [aarch64-macos]\n  capabilities: {exec: true, read: true}",
        );
        let p = g.path().join("dep.tfs");
        build_zip(
            &p,
            &["sbin/", "__tpkg__/"],
            &[
                ("sbin/x", b"#!/bin/sh\n".as_slice()),
                ("__tpkg__/manifest.yaml", manifest.as_bytes()),
            ],
        );
        p
    };
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("PATH", "/usr/bin");

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/app", app.display()),
            "--tebako-image",
            &format!("{}:-:/opt/dep", dep.display()),
            "--tebako-entry",
            "/bin/java",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    // Only the dependency contributes: the app's own /bin stays off PATH.
    assert_eq!(
        env.var("PATH").as_deref(),
        Some(joined_path(&["/opt/dep/sbin", "/usr/bin"]).as_str())
    );
}

#[test]
fn an_image_without_a_manifest_declares_no_bins() {
    let g = guard("path-env-plain");
    let env_image = write_env_image(g.path());
    let payload = write_payload_image(g.path());
    let plain = write_payload_image(g.path()); // the dependency: no manifest at all
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("PATH", "/usr/bin");

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/app", payload.display()),
            "--tebako-image",
            &format!("{}:-:/opt/plain", plain.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    assert_eq!(env.var("PATH").as_deref(), Some("/usr/bin"));
}

#[test]
fn a_corrupt_dependency_manifest_is_a_named_65() {
    let g = guard("path-env-corrupt");
    let env_image = write_env_image(g.path());
    let payload = write_payload_image(g.path());
    let corrupt = {
        let p = g.path().join("corrupt.tfs");
        build_zip(
            &p,
            &["__tpkg__/"],
            &[(
                "__tpkg__/manifest.yaml",
                b"identity: [not a mapping\n".as_slice(),
            )],
        );
        p
    };
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/app", payload.display()),
            "--tebako-image",
            &format!("{}:-:/opt/corrupt", corrupt.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("self-description lies"),
        "{}",
        err.message
    );
    assert!(
        !context().read().unwrap().is_mounted(),
        "the refusal unmounts everything"
    );
}

// ---------------------------------------------------------------------
// spec 22 §3.2: the host-launcher tier — self-injecting PATH wrappers
// ---------------------------------------------------------------------

/// The boot shape shared by the launcher tests: the shim env image, a
/// plain app payload, and one toolkit dependency mounted at `point`.
#[cfg(unix)]
fn boot_with_toolkit(g: &Guard, point: &str, java: &[u8]) -> MapEnv {
    let env_image = write_env_image_with_shim(g.path());
    let payload = write_payload_image(g.path());
    let toolkit = write_toolkit_image_with_java(g.path(), java);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("PATH", "/usr/bin:/bin");
    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:-:/app", payload.display()),
            "--tebako-image",
            &format!("{}:-:{point}", toolkit.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    env
}

#[cfg(unix)]
#[test]
fn dependency_executables_materialize_as_self_injecting_launchers() {
    use std::os::unix::fs::PermissionsExt as _;
    let g = guard("path-env-wrap");
    let env = boot_with_toolkit(&g, "/opt/openjdk", b"#!/bin/sh\n");

    // PATH leads with the launcher dir, then the VFS bin dir, then the
    // inherited value.
    let path = env.var("PATH").unwrap();
    let dirs: Vec<String> = std::env::split_paths(std::ffi::OsStr::new(&path))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(dirs.len(), 4, "{path}");
    let wrap_dir = PathBuf::from(&dirs[0]);
    assert_eq!(wrap_dir.file_name().unwrap().to_string_lossy(), "wrap-bin");
    assert!(
        wrap_dir
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("tebako-dl-"),
        "the launchers live under the cleaned dl root: {}",
        wrap_dir.display()
    );
    assert_eq!(dirs[1], "/opt/openjdk/bin");

    // The wrapper re-arms the injection var explicitly and execs the
    // materialized binary (the dl layout mirrors the full VFS path).
    let shim_host = env.var(INJECT_VAR).unwrap();
    let target = wrap_dir.parent().unwrap().join("opt/openjdk/bin/java");
    let wrap = wrap_dir.join("java");
    assert_eq!(
        std::fs::read_to_string(&wrap).unwrap(),
        format!(
            "#!/bin/sh\n{v}='{shim_host}'\nexport {v}\nexec '{}' \"$@\"\n",
            target.display(),
            v = INJECT_VAR
        )
    );
    assert_eq!(
        std::fs::metadata(&wrap).unwrap().permissions().mode() & 0o777,
        0o755
    );
    // The zip reports 0644 — the tier forces the exec bit on the copy.
    assert_ne!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o111,
        0
    );
}

/// The launcher's own proof on ELF: nothing is inherited (`env_clear`)
/// — the wrapper alone arms the injection (ld.so then complains about
/// the fixture shim, proving the var reached it) and execs the target.
/// On macOS the same text is the SIP answer (dyld aborts on a bogus
/// insert, so the run leg lives on linux; the macOS proof is the
/// dogfood).
#[cfg(target_os = "linux")]
#[test]
fn a_launcher_re_arms_the_injection_and_execs_the_target() {
    let g = guard("path-env-wrap-run");
    // A distinct mount point: the dl cache keys by memfs path, and the
    // content test's plain java must not shadow this one's marker.
    let env = boot_with_toolkit(&g, "/opt/jdkrun", b"#!/bin/sh\necho TEBAKO-WRAP-OK\n");
    let path = env.var("PATH").unwrap();
    let wrap_dir = std::env::split_paths(std::ffi::OsStr::new(&path))
        .next()
        .unwrap();

    let out = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("java")
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", wrap_dir.display()))
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "TEBAKO-WRAP-OK",
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("LD_PRELOAD"),
        "the wrapper armed the injection var: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ---------------------------------------------------------------------
// spec 22 §4 class R: declarative boot materialization
// ---------------------------------------------------------------------

/// A runtime-kind in-image manifest declaring `materialize` (the env
/// image's own resource declaration — the cert case).
fn runtime_manifest(materialize: &[&str]) -> String {
    let list: String = materialize.iter().map(|p| format!("  - {p}\n")).collect();
    format!(
        "identity:\n  schema_version: 1\n  kind: runtime\n  name: rt\n  version: \"1\"\n  \
         producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-14T00:00:00Z\"\n  \
         digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
         signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
         provides:\n  provides: {{engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: aarch64-macos}}\n  \
         built_from: {{src_sha256: {z}, patch_set: v0}}\n  capabilities: {{exec: true, read: true, runtime: true}}\n\
         materialize:\n{list}",
        z = "0".repeat(64)
    )
}

/// An app-kind in-image manifest declaring `materialize` (a payload's
/// own resource declaration).
fn app_manifest(materialize: &[&str]) -> String {
    let list: String = materialize.iter().map(|p| format!("  - {p}\n")).collect();
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: app\n  version: \"1\"\n  \
         producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-14T00:00:00Z\"\n  \
         digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
         signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
         provides:\n  entrypoints: [{{name: app, path: /bin/app}}]\n  \
         platforms: universal\n  capabilities: {{exec: true, read: true}}\n\
         materialize:\n{list}",
        z = "0".repeat(64)
    )
}

/// The env-image fixture carrying a manifest that declares the cert
/// resource (spec 22 §4: the image-OWNED default — R2's cert case).
fn write_env_image_with_resource(dir: &Path, cert: &[u8]) -> PathBuf {
    let p = dir.join("runtime.tfs");
    build_zip(
        &p,
        &["lib/", "lib/ruby/", "lib/tebako/", "__tpkg__/"],
        &[
            ("lib/ruby/rubygems.rb", b"# rubygems core\n".as_slice()),
            ("lib/tebako/layout.yaml", GOOD_LAYOUT.as_bytes()),
            ("lib/tebako/cacert.pem", cert),
            (
                "__tpkg__/manifest.yaml",
                runtime_manifest(&["/lib/tebako/cacert.pem"]).as_bytes(),
            ),
        ],
    );
    p
}

/// A payload fixture carrying a manifest that declares one resource.
fn write_payload_image_with_resource(dir: &Path, resource: &[u8]) -> PathBuf {
    let p = dir.join("payload.tfs");
    build_zip(
        &p,
        &["bin/", "lib/", "__tpkg__/"],
        &[
            ("bin/app", b"#!/usr/bin/env ruby\nputs 'hi'\n".as_slice()),
            ("lib/app.pem", resource),
            (
                "__tpkg__/manifest.yaml",
                app_manifest(&["/lib/app.pem"]).as_bytes(),
            ),
        ],
    );
    p
}

/// The resources namespace one boot of `image` extracts into
/// (`<TEBAKO_EXEC_CACHE>/resources/<image-key>`, spec 22 §6) — computed
/// pre-boot so the test can reset the write-once cache between runs.
fn resources_dir(image: &Path, env_image: Option<&Path>) -> PathBuf {
    let root_key = env_image
        .map(tebako_driver::exec_cache::image_key)
        .unwrap_or_else(|| "host".to_string());
    tebako_driver::exec_cache::root_for(&std::env::temp_dir(), &root_key)
        .join("resources")
        .join(tebako_driver::exec_cache::image_key(image))
}

#[test]
fn boot_materializes_a_declared_env_image_resource() {
    let g = guard("mat-env");
    let cert = b"CERT-A\n";
    let env_image = write_env_image_with_resource(g.path(), cert);
    write_sidecar(&env_image, &"ca".repeat(32));
    let resources = resources_dir(&env_image, Some(&env_image));
    let _ = std::fs::remove_dir_all(&resources);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    boot(&argv(&["ruby", "--version"]), "/__tfs__", &env).unwrap();

    // The declared path lands at <exec-cache>/resources/<image-key>/<P>,
    // read-only (Rule R3), content-exact.
    let extracted = resources.join("lib/tebako/cacert.pem");
    assert_eq!(std::fs::read(&extracted).unwrap(), cert);
    assert!(
        std::fs::metadata(&extracted)
            .unwrap()
            .permissions()
            .readonly(),
        "the materialized resource is read-only"
    );
    // The recorded digest pins the extraction to the bytes the image
    // served (the tfs-merkle-1 file value).
    let recorded = std::fs::read_to_string(resources.join("lib/tebako/cacert.pem.tfs-digest"))
        .expect("the digest record rides alongside");
    let mut h = tpkg::merkle::FileHasher::new();
    h.update(cert);
    assert_eq!(recorded.trim(), tpkg::merkle::render_tree_hash(&h.finish()));
    // The mount stays live — materialization is additive, never a move.
    assert_eq!(read_file("/__tfs__/lib/tebako/cacert.pem"), cert);
    let _ = std::fs::remove_dir_all(&resources);
}

#[test]
fn boot_materializes_a_declared_payload_resource() {
    let g = guard("mat-payload");
    let payload = write_payload_image_with_resource(g.path(), b"PAYLOAD-PEM\n");
    let resources = resources_dir(&payload, None);
    let _ = std::fs::remove_dir_all(&resources);
    let env = MapEnv::new();

    let out = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    assert_eq!(out.argv, argv(&["ruby", "/bin/app"]));
    let extracted = resources.join("lib/app.pem");
    assert_eq!(std::fs::read(&extracted).unwrap(), b"PAYLOAD-PEM\n");
    let _ = std::fs::remove_dir_all(&resources);
}

#[test]
fn a_listed_but_absent_path_is_a_named_boot_error() {
    let g = guard("mat-absent");
    // The manifest declares a path the image does not carry — the
    // manifest lied (Rule R3), never a skipped entry.
    let payload = {
        let p = g.path().join("payload.tfs");
        build_zip(
            &p,
            &["bin/", "__tpkg__/"],
            &[
                ("bin/app", b"#!/usr/bin/env ruby\n".as_slice()),
                (
                    "__tpkg__/manifest.yaml",
                    app_manifest(&["/lib/missing.pem"]).as_bytes(),
                ),
            ],
        );
        p
    };
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("/lib/missing.pem"), "{}", err.message);
    assert!(
        err.message.contains("self-description lies"),
        "{}",
        err.message
    );
    assert!(
        !context().read().unwrap().is_mounted(),
        "the refusal unmounts everything"
    );
}

#[test]
fn a_listed_directory_is_a_named_boot_error() {
    let g = guard("mat-dir");
    // Rule R3 is whole-FILE: a listed directory is out of grammar.
    let payload = {
        let p = g.path().join("payload.tfs");
        build_zip(
            &p,
            &["bin/", "lib/", "__tpkg__/"],
            &[
                ("bin/app", b"#!/usr/bin/env ruby\n".as_slice()),
                ("lib/app.rb", b"# app\n".as_slice()),
                ("__tpkg__/manifest.yaml", app_manifest(&["/lib"]).as_bytes()),
            ],
        );
        p
    };
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("not a regular file"),
        "{}",
        err.message
    );
}

#[test]
fn a_materialize_escape_path_is_a_named_error() {
    let g = guard("mat-escape");
    // '..' would escape the resources namespace on the host — the
    // manifest fails validation, so the image's self-description is
    // corrupt by definition (the driver's named 65).
    let payload = {
        let p = g.path().join("payload.tfs");
        build_zip(
            &p,
            &["bin/", "__tpkg__/"],
            &[
                ("bin/app", b"#!/usr/bin/env ruby\n".as_slice()),
                (
                    "__tpkg__/manifest.yaml",
                    app_manifest(&["/../../host/x"]).as_bytes(),
                ),
            ],
        );
        p
    };
    let env = MapEnv::new();

    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("self-description lies"),
        "{}",
        err.message
    );
}

#[test]
fn a_second_boot_serves_the_verified_write_once_cache() {
    let g = guard("mat-writeonce");
    let payload = write_payload_image_with_resource(g.path(), b"PEM-ONE\n");
    write_sidecar(&payload, &"ef".repeat(32));
    let resources = resources_dir(&payload, None);
    let _ = std::fs::remove_dir_all(&resources);
    let env = MapEnv::new();

    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    let extracted = resources.join("lib/app.pem");
    assert_eq!(std::fs::read(&extracted).unwrap(), b"PEM-ONE\n");

    // Rebuild the image at the same path with DIFFERENT content under
    // the same sidecar key: the write-once namespace serves the verified
    // first extraction (the store's content key is the production
    // segregation; a fixed path key is the documented dev-boot caveat —
    // exec_cache.rs).
    write_payload_image_with_resource(g.path(), b"PEM-TWO\n");
    reset();
    let env = MapEnv::new();
    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(&extracted).unwrap(),
        b"PEM-ONE\n",
        "write-once: the verified cache is served, never re-extracted under one key"
    );
    let _ = std::fs::remove_dir_all(&resources);
}

#[test]
fn a_tampered_extracted_resource_fails_verification() {
    let g = guard("mat-tamper");
    let payload = write_payload_image_with_resource(g.path(), b"PEM-ONE\n");
    write_sidecar(&payload, &"ad".repeat(32));
    let resources = resources_dir(&payload, None);
    let _ = std::fs::remove_dir_all(&resources);
    let env = MapEnv::new();
    boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();

    // Tamper with the cached copy: the next boot's verification fails
    // closed (Rule R3) — never a silently served corruption.
    let extracted = resources.join("lib/app.pem");
    // Make the read-only-extracted copy writable: an explicit mode on
    // unix, the readonly attribute on windows.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&extracted, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    #[cfg(windows)]
    {
        let mut perms = std::fs::metadata(&extracted).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        std::fs::set_permissions(&extracted, perms).unwrap();
    }
    std::fs::write(&extracted, b"PEM-FORGED\n").unwrap();

    reset();
    let env = MapEnv::new();
    let err = boot(
        &argv(&[
            "ruby",
            "--tebako-image",
            &format!("{}:0:/", payload.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 70, "{}", err.message);
    assert!(err.message.contains("/lib/app.pem"), "{}", err.message);
    assert!(err.message.contains("verification"), "{}", err.message);
    assert!(
        !context().read().unwrap().is_mounted(),
        "the refusal unmounts everything"
    );
    let _ = std::fs::remove_dir_all(&resources);
}
