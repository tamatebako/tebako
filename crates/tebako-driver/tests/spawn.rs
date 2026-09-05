//! The spawned-runtime surface end to end (spec 30): the boot captures
//! the app payload's expose map, the FFI planner resolves a bare command
//! against the store's runtimes and composes the child boot (carried
//! mounts, env deletes, the jail union), and the PATH launcher tier
//! writes one script per expose. All tests serialize on LOCK — the tfs
//! context, the spawn state, and the process env are process-global
//! (the tests/boot.rs pattern).

use std::collections::HashMap;
use std::ffi::CString;
use std::io::Write as _;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tebako_driver::{boot, Env};
use tfs::context::context;

static LOCK: Mutex<()> = Mutex::new(());

struct Guard {
    _guard: MutexGuard<'static, ()>,
    tmp: TempDir,
    prior_home: Option<String>,
}

fn guard(tag: &str) -> Guard {
    let g = LOCK.lock().unwrap();
    reset();
    let tmp = TempDir::new(tag);
    let prior_home = std::env::var("TEBAKO_HOME").ok();
    std::env::set_var("TEBAKO_HOME", tmp.path().join("home"));
    Guard {
        _guard: g,
        tmp,
        prior_home,
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        match &self.prior_home {
            Some(v) => std::env::set_var("TEBAKO_HOME", v),
            None => std::env::remove_var("TEBAKO_HOME"),
        }
        reset();
    }
}

fn reset() {
    context().write().unwrap().unmount();
    context()
        .write()
        .unwrap()
        .set_host_policy(tfs::policy::HostPolicy::open(), None);
    tfs::trace::disarm();
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tebako-spawn-it-{tag}-{}-{uniq}",
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

struct MapEnv(std::cell::RefCell<HashMap<String, String>>);

impl MapEnv {
    fn new() -> MapEnv {
        MapEnv(std::cell::RefCell::new(HashMap::new()))
    }

    fn set(&self, key: &str, value: String) {
        self.0.borrow_mut().insert(key.to_string(), value);
    }

    fn get(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
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

fn argv(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
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

const GOOD_LAYOUT: &str = "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"3.4\"\n";

/// A manifest fixture. `provides` is the pre-indented YAML tail.
fn payload_manifest(kind: &str, name: &str, provides: &str) -> String {
    let z = "0".repeat(64);
    format!(
        "identity:\n  schema_version: 1\n  kind: {kind}\n  name: {name}\n  version: \"1\"\n  \
         producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-13T00:00:00Z\"\n  \
         digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
         signing: {{state: unsigned}}\n  encryption: {{state: none}}\n{provides}\n"
    )
}

/// The runtime payload's manifest (the env image's self-description):
/// the spawn surface `entrypoints` plus the kind-exact capabilities.
fn runtime_manifest(entrypoints: &str) -> String {
    let z = "0".repeat(64);
    payload_manifest(
        "runtime",
        "openjdk",
        &format!(
            "provides:\n  \
             provides: [{{engine: java, version: \"21.0.12\", abi_line: \"21\", platform: aarch64-macos}}]\n  \
             built_from: {{src_sha256: {z}, patch_set: base}}\n  \
             entrypoints: {entrypoints}\n  \
             capabilities: {{exec: true, read: true, runtime: true}}"
        ),
    )
}

/// The env image: the era-2 layout plus a runtime manifest whose spawn
/// surface declares `entrypoints_yaml` (e.g. `[{name: tool, path:
/// /bin/tool, args_default: ["--x"]}]`).
fn write_env_image(dir: &Path, entrypoints_yaml: &str) -> PathBuf {
    let p = dir.join("runtime.tfs");
    build_zip(
        &p,
        &["lib/", "lib/tebako/", "bin/", "__tpkg__/"],
        &[
            ("lib/tebako/layout.yaml", GOOD_LAYOUT.as_bytes()),
            ("bin/tool", b"#!/bin/sh\nexit 0\n".as_slice()),
            (
                "__tpkg__/manifest.yaml",
                runtime_manifest(entrypoints_yaml).as_bytes(),
            ),
        ],
    );
    p
}

/// The app payload carrying a spawned-runtime edge (spec 30 §1):
/// `/bin/app`, a manifest with `requires: [{kind: runtime, engine:
/// java, constraint, expose}]`. `expose_yaml` is e.g. `[java]`.
fn write_app_image(dir: &Path, expose_yaml: &str) -> PathBuf {
    let manifest = payload_manifest(
        "app",
        "metanorma",
        &format!(
            "provides:\n  \
             entrypoints: [{{name: app, path: /bin/app}}]\n  \
             platforms: universal\n  capabilities: {{exec: true, read: true}}\n\
             requires:\n  \
             - {{kind: runtime, engine: java, constraint: \">= 21\", expose: {expose_yaml}}}"
        ),
    );
    let p = dir.join("app.tfs");
    build_zip(
        &p,
        &["bin/", "__tpkg__/"],
        &[
            ("bin/app", b"#!/usr/bin/env ruby\nputs 'hi'\n".as_slice()),
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ],
    );
    p
}

/// A store entry for the depended runtime: exe + REAL zip image (the
/// spawn surface read mounts it) + trust sidecar.
fn store_entry(home: &Path, lv: &str, ver: &str) {
    let platform = tpkg::runtime_store::platform_string();
    let dir = home
        .join("runtimes")
        .join(format!("java-{lv}-{ver}-{platform}"));
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(not(windows))]
    let exe = format!("tebako-runtime-{ver}-{lv}-{platform}");
    #[cfg(windows)]
    let exe = format!("tebako-runtime-{ver}-{lv}-{platform}.exe");
    std::fs::write(dir.join(exe), b"exe").unwrap();
    let image = format!("tebako-runtime-{ver}-{lv}-{platform}.tfs");
    build_zip(
        &dir.join(&image),
        &["__tpkg__/"],
        &[(
            "__tpkg__/manifest.yaml",
            runtime_manifest("[{name: java, path: /bin/java}]").as_bytes(),
        )],
    );
    std::fs::write(dir.join(format!("{image}.sha256")), b"x").unwrap();
}

/// The boot with the env image + the app payload mounted at `/`.
fn boot_app(g: &Guard, entrypoints_yaml: &str, expose_yaml: &str) -> (MapEnv, PathBuf) {
    let env_image = write_env_image(g.tmp.path(), entrypoints_yaml);
    let app = write_app_image(g.tmp.path(), expose_yaml);
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    boot(
        &argv(&[
            "app",
            "--tebako-image",
            &format!("{}:-:/", app.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    (env, app)
}

/// Call the FFI planner; on a plan, (argv, env ops) unpacked.
#[allow(clippy::type_complexity)]
fn ffi_plan(command: &str, args: &[&str]) -> (i32, Option<(Vec<String>, Vec<String>)>) {
    let cmd = CString::new(command).unwrap();
    let mut packed: Vec<u8> = Vec::new();
    for a in args {
        packed.extend_from_slice(a.as_bytes());
        packed.push(0);
    }
    let mut out_exe: *mut c_char = std::ptr::null_mut();
    let mut out_argv: *mut c_char = std::ptr::null_mut();
    let mut out_argv_len: usize = 0;
    let mut out_env: *mut c_char = std::ptr::null_mut();
    let mut out_env_len: usize = 0;
    let mut out_error: *mut c_char = std::ptr::null_mut();
    let rc = unsafe {
        tebako_driver::ffi::tebako_spawn_runtime_plan(
            cmd.as_ptr(),
            packed.as_ptr() as *const c_char,
            packed.len(),
            &mut out_exe,
            &mut out_argv,
            &mut out_argv_len,
            &mut out_env,
            &mut out_env_len,
            &mut out_error,
        )
    };
    let unpack = |p: *mut c_char, len: usize| -> Vec<String> {
        let raw: &[u8] = unsafe { std::slice::from_raw_parts(p as *const u8, len) };
        let v: Vec<String> = raw
            .split(|b| *b == 0)
            .filter(|e| !e.is_empty())
            .map(|e| String::from_utf8_lossy(e).into_owned())
            .collect();
        v
    };
    let result = match rc {
        1 => {
            let argv = unpack(out_argv, out_argv_len);
            let env = unpack(out_env, out_env_len);
            unsafe {
                libc::free(out_exe as *mut libc::c_void);
                libc::free(out_argv as *mut libc::c_void);
                libc::free(out_env as *mut libc::c_void);
            }
            (rc, Some((argv, env)))
        }
        _ => {
            if !out_error.is_null() {
                let msg = unsafe { std::ffi::CStr::from_ptr(out_error) }
                    .to_string_lossy()
                    .into_owned();
                eprintln!("ffi plan error for '{command}': {msg}");
                unsafe { libc::free(out_error as *mut libc::c_void) };
            }
            (rc, None)
        }
    };
    result
}

#[test]
fn the_boot_captures_and_the_ffi_plans_with_carried_mounts() {
    let g = guard("ffi");
    store_entry(&g.tmp.path().join("home"), "21.0.12", "0.3.0");
    let (_env, app) = boot_app(&g, "[{name: java, path: /bin/java}]", "[java]");

    // An unexposed name passes through untouched.
    let (rc, plan) = ffi_plan("javac", &[]);
    assert_eq!(rc, 0);
    assert!(plan.is_none());

    // The exposed name plans: argv carries the payload's mount (the
    // `/bin/app` argument touches it) and the entry grammar.
    let (rc, plan) = ffi_plan("java", &["/bin/app", "-version"]);
    assert_eq!(rc, 1);
    let (argv, env) = plan.expect("planned");
    assert!(argv[0].contains("tebako-runtime-0.3.0-21.0.12"), "{argv:?}");
    assert!(argv.contains(&"--tebako-entry".to_string()));
    let entry_pos = argv.iter().position(|a| a == "--tebako-entry").unwrap();
    assert_eq!(argv[entry_pos + 1], "java");
    let entry_args = &argv[entry_pos + 2..];
    #[cfg(not(windows))]
    {
        // POSIX: the argument's mount rides as a triple; the argument
        // passes verbatim.
        let triple = format!("{}:-:/", app.display());
        assert!(argv.contains(&triple), "{argv:?}");
        assert_eq!(entry_args, ["/bin/app".to_string(), "-version".to_string()]);
        // The runtime root's own decl is never carried (EEXIST).
        assert!(!argv.iter().any(|a| a.ends_with(":/__tfs__")), "{argv:?}");
    }
    #[cfg(windows)]
    {
        // Windows: nothing carries; the embedded argument materialized
        // to its host twin.
        assert!(!entry_args.contains(&"/bin/app".to_string()), "{argv:?}");
        assert!(entry_args.last().is_some_and(|a| a == "-version"));
    }
    // The env ops delete the parent's wiring and set the child's image.
    assert!(env.iter().any(|e| e.starts_with("TEBAKO_RUNTIME_IMAGE=")));
    for key in [
        "TEBAKO_JAIL",
        "TEBAKO_TFS_MOUNTS",
        "TEBAKO_PRELOAD_SHIM",
        "TEBAKO_MOUNT_ROOT",
        "TEBAKO_SPAWN_LOCK",
    ] {
        assert!(env.contains(&key.to_string()), "{key} deleted: {env:?}");
    }
}

#[test]
fn a_duplicate_expose_name_is_a_named_65() {
    let g = guard("dup");
    let env_image = write_env_image(g.tmp.path(), "[]");
    // Two edges both exposing `java`.
    let manifest = payload_manifest(
        "app",
        "metanorma",
        "provides:\n  \
         entrypoints: [{name: app, path: /bin/app}]\n  \
         platforms: universal\n  capabilities: {exec: true, read: true}\n\
         requires:\n  \
         - {kind: runtime, engine: java, constraint: \">= 21\", expose: [java]}\n  \
         - {kind: runtime, engine: java, constraint: \">= 21\", expose: [java]}",
    );
    let app = g.tmp.path().join("app.tfs");
    build_zip(
        &app,
        &["bin/", "__tpkg__/"],
        &[
            ("bin/app", b"x".as_slice()),
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ],
    );
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = boot(
        &argv(&[
            "app",
            "--tebako-image",
            &format!("{}:-:/", app.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err}");
    assert!(err.message.contains("ambiguous"), "{err}");
}

#[test]
fn a_torn_spawn_lock_is_a_named_65() {
    let g = guard("torn");
    let env_image = write_env_image(g.tmp.path(), "[]");
    let app = write_app_image(g.tmp.path(), "[java]");
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    env.set("TEBAKO_SPAWN_LOCK", "java=21".to_string());
    let err = boot(
        &argv(&[
            "app",
            "--tebako-image",
            &format!("{}:-:/", app.display()),
            "--tebako-entry",
            "/bin/app",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err}");
    assert!(err.message.contains("TEBAKO_SPAWN_LOCK"), "{err}");
}

#[test]
fn the_bare_name_arm_resolves_the_env_declaration() {
    let g = guard("bare");
    let env_image = write_env_image(
        g.tmp.path(),
        "[{name: tool, path: /bin/tool, args_default: [\"--x\"]}]",
    );
    let app = write_app_image(g.tmp.path(), "[]");
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let out = boot(
        &argv(&[
            "app",
            "--tebako-image",
            &format!("{}:-:/", app.display()),
            "--tebako-entry",
            "tool",
            "a",
            "b",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    // The declared entrypoint composes: argv0, the declaration's
    // args_default, the resolved path, the user args (spec 30 §2).
    assert_eq!(
        out.argv,
        argv(&["app", "--x", "/__tfs__/bin/tool", "a", "b"])
    );
}

#[test]
fn the_bare_name_arm_refuses_an_undeclared_name() {
    let g = guard("bare-65");
    let env_image = write_env_image(g.tmp.path(), "[{name: java, path: /bin/java}]");
    let app = write_app_image(g.tmp.path(), "[]");
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = boot(
        &argv(&[
            "app",
            "--tebako-image",
            &format!("{}:-:/", app.display()),
            "--tebako-entry",
            "jing",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err}");
    assert!(err.message.contains("'jing'"), "{err}");
}

/// The PATH launcher tier (spec 30 §3, unix): one script per expose in
/// the wrap-bin dir that leads PATH.
#[cfg(not(windows))]
#[test]
fn the_path_launcher_is_written_per_expose() {
    let g = guard("launcher");
    store_entry(&g.tmp.path().join("home"), "21.0.12", "0.3.0");
    let (env, _app) = boot_app(&g, "[{name: java, path: /bin/java}]", "[java]");
    let path = env.get("PATH").expect("PATH composed");
    let wrap_dir = path.split(':').next().unwrap();
    assert!(wrap_dir.ends_with("wrap-bin"), "{path}");
    let script = std::fs::read_to_string(Path::new(wrap_dir).join("java")).unwrap();
    assert!(script.starts_with("#!/bin/sh\n"), "{script}");
    assert!(script.contains("unset "), "{script}");
    assert!(script.contains("TEBAKO_RUNTIME_IMAGE="), "{script}");
    assert!(script.contains("--tebako-entry' 'java"), "{script}");
    assert!(script.contains("exec '"), "{script}");
}

/// The fail-closed launcher (spec 30 §3): the expose exists but no
/// cached runtime resolves — the script answers with the named error,
/// never a fall-through.
#[cfg(not(windows))]
#[test]
fn the_path_launcher_fails_closed_when_unresolved() {
    let g = guard("launcher-69");
    let (env, _app) = boot_app(&g, "[{name: java, path: /bin/java}]", "[java]");
    let path = env.get("PATH").expect("PATH composed");
    let wrap_dir = path.split(':').next().unwrap();
    let script = std::fs::read_to_string(Path::new(wrap_dir).join("java")).unwrap();
    assert!(script.contains("exit 69"), "{script}");
    assert!(script.contains("never downloads"), "{script}");
}

// ---------------------------------------------------------------------
// the spawned PAYLOAD child's bare-name arm (spec 32 §2): the FIRST
// triple's own App manifest owns the bare name; the env image's
// runtimeProvides is the fallback surface.
// ---------------------------------------------------------------------

/// The provider payload image: an app manifest declaring the `xml2rfc`
/// entrypoint (path + args_default), no spawn edges.
fn write_provider_image(dir: &Path) -> PathBuf {
    let manifest = payload_manifest(
        "app",
        "xml2rfc",
        "provides:\n  \
         entrypoints: [{name: xml2rfc, path: /bin/xml2rfc, args_default: [\"--x\"]}]\n  \
         platforms: universal\n  capabilities: {exec: true, read: true}",
    );
    let p = dir.join("xml2rfc.tfs");
    build_zip(
        &p,
        &["bin/", "__tpkg__/"],
        &[
            ("bin/xml2rfc", b"#!/usr/bin/env python\n".as_slice()),
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ],
    );
    p
}

#[test]
fn the_bare_name_arm_prefers_the_first_payloads_declaration() {
    let g = guard("bare-payload");
    let env_image = write_env_image(g.tmp.path(), "[]");
    let provider = write_provider_image(g.tmp.path());
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let out = boot(
        &argv(&[
            "python",
            "--tebako-image",
            &format!("{}:-:/", provider.display()),
            "--tebako-entry",
            "xml2rfc",
            "doc.xml",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    // The PROVIDER's declaration composes — argv0, the declaration's
    // args_default, the resolved path, the user args (spec 32 §2); the
    // env image's empty surface never enters.
    assert_eq!(
        out.argv,
        argv(&["python", "--x", "/bin/xml2rfc", "doc.xml"])
    );
}

#[test]
fn the_bare_name_arm_falls_back_to_the_env_declaration() {
    let g = guard("bare-fallback");
    let env_image = write_env_image(
        g.tmp.path(),
        "[{name: pytool, path: /bin/tool, args_default: [\"--y\"]}]",
    );
    let provider = write_provider_image(g.tmp.path());
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    // The provider declares `xml2rfc` only; `pytool` is the env image's
    // own runtime entrypoint (spec 30 §2's surface, untouched).
    let out = boot(
        &argv(&[
            "python",
            "--tebako-image",
            &format!("{}:-:/", provider.display()),
            "--tebako-entry",
            "pytool",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap();
    assert_eq!(out.argv, argv(&["python", "--y", "/__tfs__/bin/tool"]));
}

#[test]
fn the_bare_name_arm_refuses_a_name_neither_surface_declares() {
    let g = guard("bare-neither");
    let env_image = write_env_image(g.tmp.path(), "[]");
    let provider = write_provider_image(g.tmp.path());
    let env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    let err = boot(
        &argv(&[
            "python",
            "--tebako-image",
            &format!("{}:-:/", provider.display()),
            "--tebako-entry",
            "ghost",
        ]),
        "/__tfs__",
        &env,
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err}");
    assert!(err.message.contains("'ghost'"), "{err}");
    assert!(
        !context().read().unwrap().is_mounted(),
        "a failed boot leaves nothing mounted"
    );
}
