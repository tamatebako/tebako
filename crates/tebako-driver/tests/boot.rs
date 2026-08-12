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
