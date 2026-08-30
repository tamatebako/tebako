//! The spec-29 wrapper pattern against the process-global TFS context:
//! the shared spec-17 boot + the wrapper boot-tail (interpreter
//! declaration, visibility decision, materialization, argv composition,
//! `--tebako-extract`) — and the golden parity of the linked vs wrapper
//! boots on the same inputs. All tests serialize on LOCK (the
//! tests/boot.rs pattern); fixtures are zips built in-code.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tebako_driver::wrapper::{run, BootAction, WRAPPER_RUNTIME_ROOT};
use tebako_driver::{boot, Env};
use tfs::context::context;

static LOCK: Mutex<()> = Mutex::new(());

/// The platform's injection variable (mirrors injection.rs's
/// crate-private `INJECT_VAR`; tests/boot.rs keeps its own copy too).
#[cfg(all(unix, target_os = "macos"))]
const INJECT_VAR: &str = "DYLD_INSERT_LIBRARIES";
#[cfg(all(unix, not(target_os = "macos")))]
const INJECT_VAR: &str = "LD_PRELOAD";

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
    context().write().unwrap().unmount();
    context()
        .write()
        .unwrap()
        .set_host_policy(tfs::policy::HostPolicy::open(), None);
    tfs::trace::disarm();
    Guard { _guard: g, tmp }
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tebako-wrapper-{tag}-{}-{uniq}",
            std::process::id()
        ));
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

    fn get(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }

    fn map(&self) -> HashMap<String, String> {
        self.0.borrow().clone()
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

/// The era-2 layout declaration for the tests' root (spec 18 C3).
const GOOD_LAYOUT: &str = "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"21\"\n";

/// The fake interpreter bytes the fixtures carry at `bin/stub`.
const STUB: &[u8] = b"#!/bin/sh\necho stub\n";

/// The wrapper-pattern env image: the layout declaration with the
/// caller's spec-29 keys appended, plus the stub interpreter. `shim`
/// adds a (fake) libtfs-preload and its `preload_shim` grant — the
/// preload mechanism's arming input (materialized, never loaded, in
/// these in-process tests).
fn write_env_image(dir: &Path, spec29_keys: &str, shim: bool) -> PathBuf {
    let layout = format!("{GOOD_LAYOUT}{spec29_keys}");
    let mut files: Vec<(&str, &[u8])> = vec![
        ("lib/tebako/layout.yaml", layout.as_bytes()),
        ("bin/stub", STUB),
    ];
    if shim {
        files.push((
            "lib/tebako/libtfs_preload.so",
            b"ELF pretend shim\n".as_slice(),
        ));
    }
    let p = dir.join("env.tfs");
    build_zip(&p, &["bin/", "lib/", "lib/tebako/"], &files);
    p
}

/// The spec-29 key sets used across the suite.
const EXEC_CACHE: &str = "interpreter: /bin/stub\nvisibility: exec-cache\n";
const PRELOAD: &str =
    "interpreter: /bin/stub\nvisibility: preload\npreload_shim: lib/tebako/libtfs_preload.so\n";
const DEFAULT_WITH_SHIM: &str =
    "interpreter: /bin/stub\npreload_shim: lib/tebako/libtfs_preload.so\n";
const NO_INTERPRETER: &str = "";

/// A payload-manifest fixture (spec 03; the tests/boot.rs shape).
fn payload_manifest(kind: &str, provides: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: {kind}\n  name: app\n  version: \"1\"\n  \
         producer: {{tool: t, tool_version: \"1\"}}\n  \
         created: \"2026-08-13T00:00:00Z\"\n  \
         digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
         signing: {{state: unsigned}}\n  encryption: {{state: none}}\n{provides}\n",
        z = "0".repeat(64)
    )
}

/// The app payload: `bin/app` plus the manifest declaring the
/// entrypoint's `args_default: ["-jar"]` (spec 03 §2.2 — the jar-entry
/// shape of spec 29 §1's example).
fn write_app_image(dir: &Path) -> PathBuf {
    let manifest = payload_manifest(
        "app",
        "provides:\n  \
         entrypoints: [{name: app, path: /bin/app, args_default: [\"-jar\"]}]\n  \
         platforms: universal\n  capabilities: {exec: true, read: true}",
    );
    let p = dir.join("app.tfs");
    build_zip(
        &p,
        &["bin/", "__tpkg__/"],
        &[
            ("bin/app", b"#!/usr/bin/env java\necho app\n".as_slice()),
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ],
    );
    p
}

fn argv(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// The launch half of a run(), unwrapped.
fn launch_of(action: BootAction) -> tebako_driver::Launch {
    match action {
        BootAction::Launch(l) => l,
        other => panic!("expected a Launch, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// the launch shapes
// ---------------------------------------------------------------------

#[test]
fn exec_cache_composes_argv_and_bridges_the_entry() {
    let g = guard("ec-launch");
    let env_image = write_env_image(g.path(), EXEC_CACHE, false);
    let app = write_app_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let launch = launch_of(
        run(
            &argv(&[
                "tebako-runtime-launcher",
                "--tebako-image",
                &format!("{}:-:/", app.display()),
                "--tebako-entry",
                "/bin/app",
                "user1",
                "user 2",
            ]),
            WRAPPER_RUNTIME_ROOT,
            &env,
        )
        .unwrap(),
    );

    // [interpreter, args_default…, entry, user args…] (spec 29 §1).
    assert_eq!(launch.argv.len(), 5, "{:?}", launch.argv);
    assert_eq!(launch.argv[0], launch.program);
    assert_eq!(launch.argv[1], "-jar");
    assert_eq!(launch.argv[4], "user 2");
    assert_eq!(launch.argv[3], "user1");
    // The interpreter is the MATERIALIZED host copy of the in-image stub.
    assert_ne!(launch.program, "/__tfs__/bin/stub");
    assert_eq!(std::fs::read(&launch.program).unwrap(), STUB);
    // exec-cache's child is host-plain: the entry token is bridged to
    // its host twin (never the VFS spelling it cannot read).
    assert_ne!(launch.argv[2], "/bin/app");
    assert_eq!(
        std::fs::read(&launch.argv[2]).unwrap(),
        b"#!/usr/bin/env java\necho app\n"
    );
    // The boot armed the handoff env (the exec-cache root export).
    assert!(env.get("TEBAKO_EXEC_CACHE").is_some());
    let mounts = env.get("TEBAKO_TFS_MOUNTS").expect("the mounts list");
    assert!(mounts.contains(&format!("{}:/", app.display())), "{mounts}");
}

#[cfg(unix)]
#[test]
fn preload_keeps_the_vfs_entry_and_arms_the_injection_env() {
    let g = guard("pl-launch");
    let env_image = write_env_image(g.path(), PRELOAD, true);
    let app = write_app_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let launch = launch_of(
        run(
            &argv(&[
                "tebako-runtime-launcher",
                "--tebako-image",
                &format!("{}:-:/", app.display()),
                "--tebako-entry",
                "/bin/app",
                "-version",
            ]),
            WRAPPER_RUNTIME_ROOT,
            &env,
        )
        .unwrap(),
    );

    assert_eq!(launch.argv[0], launch.program);
    assert_eq!(launch.argv[1], "-jar");
    // Under preload the armed shim serves the VFS spelling — the entry
    // is NOT bridged (spec 29 §3's interposition tier).
    assert_eq!(launch.argv[2], "/bin/app");
    assert_eq!(launch.argv[3], "-version");
    assert_eq!(std::fs::read(&launch.program).unwrap(), STUB);
    // The spec-22 §3 arming already happened in the shared boot (reuse):
    // the shim's VFS spelling for the spawn hook, the host copy on the
    // platform's injection var, the mounts list for the re-entry.
    assert_eq!(
        env.get("TEBAKO_PRELOAD_SHIM").as_deref(),
        Some("/__tfs__/lib/tebako/libtfs_preload.so")
    );
    let inject = env.get(INJECT_VAR).expect("the injection var is armed");
    assert_eq!(std::fs::read(&inject).unwrap(), b"ELF pretend shim\n");
}

#[cfg(unix)]
#[test]
fn the_default_order_picks_preload_when_the_image_delivers_the_shim() {
    let g = guard("pl-default");
    let env_image = write_env_image(g.path(), DEFAULT_WITH_SHIM, true);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let launch = launch_of(
        run(
            &argv(&["tebako-runtime-launcher", "--version"]),
            WRAPPER_RUNTIME_ROOT,
            &env,
        )
        .unwrap(),
    );
    // The bare smoke form: [interpreter, interpreter args…].
    assert_eq!(
        launch.argv,
        vec![launch.program.clone(), "--version".to_string()]
    );
    // Tier 1 by default (spec 29 §3): the closure answer (dlmap2file)
    // — the env image stays mounted, the shim serves its reads.
    assert_eq!(std::fs::read(&launch.program).unwrap(), STUB);
    assert!(env.get("TEBAKO_PRELOAD_SHIM").is_some());
}

#[test]
fn the_interpreter_keyword_composes_without_an_entry_or_defaults() {
    let g = guard("keyword");
    let env_image = write_env_image(g.path(), EXEC_CACHE, false);
    let app = write_app_image(g.path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let launch = launch_of(
        run(
            &argv(&[
                "tebako-runtime-launcher",
                "--tebako-image",
                &format!("{}:-:/", app.display()),
                "--tebako-entry",
                "java",
                "-version",
            ]),
            WRAPPER_RUNTIME_ROOT,
            &env,
        )
        .unwrap(),
    );
    // The keyword is the interpreter itself (the deploy shims' re-entry
    // form, spec 17 §1): dropped, no entry, no args_default.
    assert_eq!(
        launch.argv,
        vec![launch.program.clone(), "-version".to_string()]
    );
}

#[test]
fn an_entry_naming_no_declared_entrypoint_composes_positionally() {
    let g = guard("no-defaults");
    let env_image = write_env_image(g.path(), EXEC_CACHE, false);
    // One payload carrying BOTH the declared entrypoint (bin/app, with
    // args_default) and an undeclared executable (bin/raw): the entry
    // resolves against the first image's mount, so both spellings must
    // live in the same image for the miss-on-defaults case to boot.
    let manifest = payload_manifest(
        "app",
        "provides:\n  \
         entrypoints: [{name: app, path: /bin/app, args_default: [\"-jar\"]}]\n  \
         platforms: universal\n  capabilities: {exec: true, read: true}",
    );
    let app = g.path().join("app.tfs");
    build_zip(
        &app,
        &["bin/", "__tpkg__/"],
        &[
            ("bin/app", b"#!/usr/bin/env java\necho app\n".as_slice()),
            ("bin/raw", b"#!/bin/sh\necho raw\n".as_slice()),
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ],
    );
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let launch = launch_of(
        run(
            &argv(&[
                "tebako-runtime-launcher",
                "--tebako-image",
                &format!("{}:-:/", app.display()),
                "--tebako-entry",
                "/bin/raw",
            ]),
            WRAPPER_RUNTIME_ROOT,
            &env,
        )
        .unwrap(),
    );
    // /bin/raw is in no manifest entrypoint — args_default is empty, the
    // interpreter takes the entry positionally (spec 29 §1): exactly
    // [program, entry], with "-jar" nowhere. Under exec-cache the entry
    // token is still bridged to its host twin.
    assert_eq!(launch.argv.len(), 2, "{:?}", launch.argv);
    assert!(
        !launch.argv.iter().any(|a| a == "-jar"),
        "{:?}",
        launch.argv
    );
    assert_ne!(launch.argv[1], "/bin/raw");
    assert_eq!(
        std::fs::read(&launch.argv[1]).unwrap(),
        b"#!/bin/sh\necho raw\n"
    );
}

#[test]
fn extract_dumps_the_mounts_and_never_launches() {
    let g = guard("extract");
    let env_image = write_env_image(g.path(), EXEC_CACHE, false);
    let app = write_app_image(g.path());
    let dest = g.path().join("dump");
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let action = run(
        &argv(&[
            "tebako-runtime-launcher",
            "--tebako-extract",
            &dest.display().to_string(),
            "--tebako-image",
            &format!("{}:-:/", app.display()),
        ]),
        WRAPPER_RUNTIME_ROOT,
        &env,
    )
    .unwrap();
    let BootAction::Extracted {
        dest: got,
        skipped_symlinks,
    } = action
    else {
        panic!("expected Extracted, got {action:?}");
    };
    assert_eq!(got, dest.display().to_string());
    assert_eq!(skipped_symlinks, 0);
    // Two mounts: each extracts into <dest>/<mount-point-basename> (the
    // context's extract_all contract) — the env image's tree and the
    // app payload's tree, both whole.
    assert_eq!(
        std::fs::read(dest.join("__tfs__/bin/stub")).unwrap(),
        STUB,
        "the env image's interpreter landed"
    );
    assert!(
        dest.join("__tfs__/lib/tebako/layout.yaml").is_file(),
        "the env image's layout declaration landed"
    );
    assert!(dest.join("root/bin/app").is_file(), "the payload landed");
}

#[test]
fn extract_with_nothing_mounted_is_a_named_65() {
    let g = guard("extract-empty");
    let env = MapEnv::new();
    let err = run(
        &argv(&[
            "tebako-runtime-launcher",
            "--tebako-extract",
            &g.path().join("dump").display().to_string(),
        ]),
        WRAPPER_RUNTIME_ROOT,
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("--tebako-extract"), "{}", err.message);
}

// ---------------------------------------------------------------------
// the named boot errors (spec 29 §2/§3/§4 — exit 65, nothing mounted)
// ---------------------------------------------------------------------

fn run_err(v: &[&str], env: &MapEnv) -> tebako_driver::DriverError {
    run(&argv(v), WRAPPER_RUNTIME_ROOT, env).unwrap_err()
}

#[test]
fn no_env_image_names_the_interpreter_key() {
    let g = guard("no-env");
    let app = write_app_image(g.path());
    let env = MapEnv::new();
    let err = run_err(
        &[
            "tebako-runtime-launcher",
            "--tebako-image",
            &format!("{}:-:/", app.display()),
            "--tebako-entry",
            "/bin/app",
        ],
        &env,
    );
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("layout.interpreter"),
        "{}",
        err.message
    );
    assert!(
        err.message.contains("TEBAKO_RUNTIME_IMAGE"),
        "{}",
        err.message
    );
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn the_absent_interpreter_key_is_a_named_65() {
    let g = guard("no-interp");
    let env_image = write_env_image(g.path(), NO_INTERPRETER, false);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = run_err(&["tebako-runtime-launcher", "--version"], &env);
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("layout.interpreter"),
        "{}",
        err.message
    );
    assert!(
        err.message.contains("LINKED"),
        "the refusal names the linked pattern: {}",
        err.message
    );
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn an_unresolvable_interpreter_names_the_path_and_the_mount() {
    let g = guard("interp-miss");
    let env_image = write_env_image(
        g.path(),
        "interpreter: /bin/java\nvisibility: exec-cache\n",
        false,
    );
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = run_err(&["tebako-runtime-launcher", "--version"], &env);
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("/bin/java"), "{}", err.message);
    assert!(err.message.contains("/__tfs__"), "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn a_malformed_interpreter_is_a_named_65() {
    let g = guard("interp-bad");
    let env_image = write_env_image(
        g.path(),
        "interpreter: bin/java\nvisibility: exec-cache\n",
        false,
    );
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = run_err(&["tebako-runtime-launcher", "--version"], &env);
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(
        err.message.contains("layout.interpreter"),
        "{}",
        err.message
    );
    assert!(err.message.contains("bin/java"), "{}", err.message);
}

#[test]
fn an_unknown_visibility_is_a_named_65() {
    let g = guard("vis-bad");
    let env_image = write_env_image(
        g.path(),
        "interpreter: /bin/stub\nvisibility: fuse\n",
        false,
    );
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = run_err(&["tebako-runtime-launcher", "--version"], &env);
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("'fuse'"), "{}", err.message);
    assert!(err.message.contains("layout.visibility"), "{}", err.message);
    // FUSE on the exec path stays refused by construction (spec 07 §8's
    // locked law) — the mechanism set simply does not contain it.
}

#[test]
fn seccomp_notify_is_named_65_until_the_tier_lands() {
    let g = guard("vis-seccomp");
    let env_image = write_env_image(
        g.path(),
        "interpreter: /bin/stub\nvisibility: seccomp-notify\n",
        false,
    );
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = run_err(&["tebako-runtime-launcher", "--version"], &env);
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("seccomp-notify"), "{}", err.message);
}

#[cfg(unix)]
#[test]
fn declared_preload_without_the_shim_grant_is_a_named_65() {
    let g = guard("pl-no-grant");
    let env_image = write_env_image(
        g.path(),
        "interpreter: /bin/stub\nvisibility: preload\n",
        false,
    );
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = run_err(&["tebako-runtime-launcher", "--version"], &env);
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("preload"), "{}", err.message);
    assert!(err.message.contains("preload_shim"), "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
}

#[cfg(unix)]
#[test]
fn the_default_order_without_a_shim_grant_fails_closed() {
    let g = guard("default-no-grant");
    // No visibility key, no shim: the POSIX default (tier 1) cannot arm
    // — a named 65, never a silent slide to exec-cache (spec 29 §3).
    let env_image = write_env_image(g.path(), "interpreter: /bin/stub\n", false);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = run_err(&["tebako-runtime-launcher", "--version"], &env);
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(err.message.contains("preload_shim"), "{}", err.message);
}

#[test]
fn a_malformed_triple_is_a_named_65_and_mounts_nothing() {
    let g = guard("malformed");
    let env = MapEnv::new();
    let err = run_err(
        &[
            "tebako-runtime-launcher",
            "--tebako-image",
            "/x/y.tfs:/",
            "--tebako-entry",
            "/x",
        ],
        &env,
    );
    assert_eq!(err.code, 65, "{}", err.message);
    assert!(!context().read().unwrap().is_mounted());
    let _ = g;
}

// ---------------------------------------------------------------------
// golden parity: the wrapper's boot IS the linked boot
// ---------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn linked_and_wrapper_boots_compose_identically() {
    let g = guard("parity");
    let env_image = write_env_image(g.path(), PRELOAD, true);
    let app = write_app_image(g.path());
    let wire = argv(&[
        "tebako-runtime-launcher",
        "--tebako-image",
        &format!("{}:-:/", app.display()),
        "--tebako-entry",
        "/bin/app",
        "user1",
    ]);

    // The linked boot on the same inputs.
    let mut linked_env = MapEnv::new();
    linked_env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let linked = boot(&wire, "/__tfs__", &linked_env).unwrap();

    // Reset the process-global context; the wrapper boot reruns the same
    // boot internally.
    context().write().unwrap().unmount();
    context()
        .write()
        .unwrap()
        .set_host_policy(tfs::policy::HostPolicy::open(), None);

    let mut wrapper_env = MapEnv::new();
    wrapper_env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let launch = launch_of(run(&wire, WRAPPER_RUNTIME_ROOT, &wrapper_env).unwrap());

    // Parity: the handoff env the two boots produce is byte-identical,
    // and the wrapper's argv tail past the interpreter + args_default is
    // the linked argv minus its program name (the interpreter stands at
    // index 0 in the wrapper's composition, spec 29 §1).
    assert_eq!(linked_env.map(), wrapper_env.map(), "the handoff envs");
    let linked_tail: Vec<String> = linked.argv.iter().skip(1).cloned().collect();
    let wrapper_tail: Vec<String> = launch.argv.iter().skip(2).cloned().collect();
    assert_eq!(wrapper_tail, linked_tail, "the rewritten argv tails");
    assert_eq!(launch.argv[1], "-jar", "the entrypoint's args_default");
}
