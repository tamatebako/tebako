//! The windows materialize tier (spec 17 §7), end to end against the
//! process-global TFS context: a runtime whose env image manifest
//! grants `windows_boot: materialize` boots from the extracted
//! exec-cache trees — `TEBAKO_MOUNT_ROOT` rewired to the extracted env
//! tree, the discovery vars pointing at host dirs, the entry rewritten
//! to its host twin — and every extracted file reads through plain host
//! IO. Windows-only by contract: the tier never engages on POSIX.
//!
//! All tests serialize on LOCK — the context is process-global (the
//! tests/boot.rs pattern).

#![cfg(windows)]

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

fn guard(tag: &str) -> Guard {
    let g = LOCK.lock().unwrap();
    let tmp = TempDir::new(tag);
    context().write().unwrap().unmount();
    context()
        .write()
        .unwrap()
        .set_host_policy(tfs::policy::HostPolicy::open(), None);
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
            "tebako-driver-tier-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
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
    fn set(&mut self, key: &str, value: impl Into<String>) {
        self.0.get_mut().insert(key.to_string(), value.into());
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

/// The spec-18 layout declaration for the fixture root `/__tfs__`.
const GOOD_LAYOUT: &str =
    "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"3.13\"\n";

/// The env image of a zero-patch runtime: the spec-18 layout, the
/// in-image manifest granting the tier (schema_minor 11), and a
/// stdlib-ish file the interpreter would read at boot.
fn write_env_image(dir: &Path, grant: bool) -> PathBuf {
    let key = if grant {
        "  windows_boot: materialize\n"
    } else {
        ""
    };
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: runtime\n  name: pyruntime\n  version: 3.13.1\n\
         \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: now\n\
         \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
         \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
         provides:\n  provides: {{engine: python, version: 3.13.1, abi_line: \"3.13\", platform: x86_64-windows-ucrt}}\n\
         \x20 built_from: {{src_sha256: {}, patch_set: v0.0.1}}\n{key}  capabilities: {{exec: true, read: true, runtime: true}}\n",
        "ab".repeat(32),
        "cd".repeat(32),
        "ef".repeat(32),
    );
    let p = dir.join("runtime.tfs");
    build_zip(
        &p,
        &["lib/", "lib/python/", "lib/tebako/", "__tpkg__/"],
        &[
            ("lib/python/site.py", b"# the stdlib\n".as_slice()),
            ("lib/tebako/layout.yaml", GOOD_LAYOUT.as_bytes()),
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ],
    );
    p
}

/// The app payload: an entrypoint the tier must hand over as a HOST
/// path.
fn write_payload_image(dir: &Path) -> PathBuf {
    let p = dir.join("payload.tfs");
    build_zip(
        &p,
        &["bin/"],
        &[("bin/app.py", b"print('hi')\n".as_slice())],
    );
    p
}

fn argv(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_granted_runtime_boots_from_the_extracted_trees() {
    let g = guard("boot");
    let env_image = write_env_image(g.tmp.0.as_path(), true);
    let payload = write_payload_image(g.tmp.0.as_path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let out = boot(
        &argv(&[
            "python",
            "--tebako-image",
            &format!("{}:-:/app", payload.display()),
            "--tebako-entry",
            "bin/app.py",
            "--verbose",
        ]),
        "/__tfs__",
        &env,
    )
    .expect("the tier boot succeeds");

    // The runtime root rewired to the extracted env tree, marker set.
    let root = env.get("TEBAKO_MOUNT_ROOT").expect("TEBAKO_MOUNT_ROOT");
    assert!(root.contains("tebako-exec-"), "{root}");
    assert!(root.contains("trees"), "{root}");
    assert_eq!(env.get("TEBAKO_MATERIALIZE_BOOT").as_deref(), Some("1"));
    assert_eq!(out.runtime_root, root);

    // The interpreter's stdlib reads as a plain host file — the tier's
    // whole point (no VFS on the consumption path).
    let site = PathBuf::from(root.replace('/', std::path::MAIN_SEPARATOR_STR))
        .join("lib")
        .join("python")
        .join("site.py");
    assert_eq!(std::fs::read(&site).unwrap(), b"# the stdlib\n");

    // The discovery surface names the payload's extracted HOST dir.
    let app_dir = env.get("TEBAKO_MOUNT_APP").expect("TEBAKO_MOUNT_APP");
    assert!(app_dir.contains("trees"), "{app_dir}");

    // The entry rewrote to its host twin and reads via plain host IO.
    let entry = out
        .argv
        .iter()
        .find(|a| a.ends_with("bin/app.py"))
        .expect("the entry");
    let entry_host = PathBuf::from(entry.replace('/', std::path::MAIN_SEPARATOR_STR));
    assert_eq!(std::fs::read(&entry_host).unwrap(), b"print('hi')\n");
    // The user's args trail verbatim.
    assert_eq!(out.argv.last().map(String::as_str), Some("--verbose"));

    // The second boot is the digest-pinned reuse: same roots, and the
    // cache tree is served (extraction would rewrite the read-only
    // files' times — the path equality + a fresh boot's success with
    // the same images is the contract-level assertion).
    let out2 = boot(
        &argv(&[
            "python",
            "--tebako-image",
            &format!("{}:-:/app", payload.display()),
            "--tebako-entry",
            "bin/app.py",
        ]),
        "/__tfs__",
        &env,
    )
    .expect("the second boot serves the cache");
    assert_eq!(out2.runtime_root, out.runtime_root);
}

#[test]
fn a_runtime_without_the_grant_boots_mounted_as_today() {
    let g = guard("nogrant");
    let env_image = write_env_image(g.tmp.0.as_path(), false);
    let payload = write_payload_image(g.tmp.0.as_path());
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());

    let out = boot(
        &argv(&[
            "python",
            "--tebako-image",
            &format!("{}:-:/app", payload.display()),
            "--tebako-entry",
            "bin/app.py",
        ]),
        "/__tfs__",
        &env,
    )
    .expect("the pre-tier boot succeeds");

    // Nothing rewired, nothing exported, nothing extracted.
    assert!(env.get("TEBAKO_MOUNT_ROOT").is_none());
    assert!(env.get("TEBAKO_MATERIALIZE_BOOT").is_none());
    assert_eq!(out.runtime_root, "/__tfs__");
    // The entry stays the in-VFS spelling (drive-qualified: /__tfs__
    // carries no drive, so the declared mounts stand unqualified).
    let entry = out
        .argv
        .iter()
        .find(|a| a.ends_with("bin/app.py"))
        .expect("the entry");
    assert_eq!(entry, "/app/bin/app.py");
}

#[test]
fn an_inherited_tier_state_is_scrubbed_before_the_override_read() {
    // spec 17 §7's respawn rule: a spawned child inheriting the tier's
    // own (marker + rewired root) must NOT read the root as a §1
    // override — the layout grants no mount_root_override, so an
    // unscrubbed read would be the named 78.
    let g = guard("respawn");
    let env_image = write_env_image(g.tmp.0.as_path(), true);
    let mut env = MapEnv::new();
    env.set("TEBAKO_RUNTIME_IMAGE", env_image.display().to_string());
    // The inherited tier state, as a spawned child would see it.
    env.set("TEBAKO_MATERIALIZE_BOOT", "1");
    env.set("TEBAKO_MOUNT_ROOT", "C:/stale/parent-tree");

    let out = boot(&argv(&["python", "-c", "pass"]), "/__tfs__", &env)
        .expect("the respawned boot re-derives the tier");
    // The stale inherited root was scrubbed and replaced by THIS boot's
    // own extracted tree (a cache hit against the first test's cache is
    // impossible here — a fresh TempDir keys nothing — but the tier
    // re-extracts and re-exports).
    let root = env.get("TEBAKO_MOUNT_ROOT").expect("TEBAKO_MOUNT_ROOT");
    assert_ne!(root, "C:/stale/parent-tree");
    assert!(root.contains("trees"), "{root}");
    assert!(out.runtime_root.contains("trees"), "{}", out.runtime_root);
}
