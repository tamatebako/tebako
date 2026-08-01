//! Boot orchestration against the process-global TFS context (spec 17):
//! env-image mount, bare/package payload mounts, entry resolution,
//! jails, and full rollback on failure. All tests serialize on LOCK —
//! the context is process-global (the tests/contract pattern).

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tebako_driver::{boot, Env};
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
struct MapEnv(HashMap<String, String>);

impl MapEnv {
    fn new() -> MapEnv {
        MapEnv(HashMap::new())
    }

    fn set(&mut self, key: &str, value: impl Into<String>) {
        self.0.insert(key.to_string(), value.into());
    }
}

impl Env for MapEnv {
    fn var(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
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
