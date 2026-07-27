//! Launcher-seam e2e for `tfs exec` (spec 07 §8 tier 1; spec 11 §6 #5).
//!
//! Builds tiny dynamic C tools + a data file + a plugin into a zip image,
//! then proves through the CLI (no extraction anywhere on the data path):
//!
//! - an IN-IMAGE entrypoint runs (materialized via dlmap2file) and reads
//!   memfs data,
//! - a HOST entrypoint sees the mounts,
//! - `--jail deny` makes a host read fail EPERM while the default keeps
//!   plain behavior,
//! - a grandchild (tool re-spawning itself) stays in the VFS,
//! - dlopen of a memfs library works via the dlmap2file host cache,
//! - usage/named errors.
//!
//! Skip policy (documented): tests SKIP when no `cc` is on PATH, the
//! shim cdylib is missing (e.g. under `cargo test -p tfs-cli` alone), or
//! the tfs binary is missing. A cc that EXISTS but fails to compile is a
//! hard failure, never a silent skip. The proof tools are shared with
//! libtfs-preload's e2e (single source in crates/libtfs-preload/tests).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const SECRET: &str = "VFS-SECRET-EXEC\n";
const MOUNT: &str = "/tfs";

// ---------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------

struct Fixtures {
    dir: PathBuf,
    zip: PathBuf,
    /// The built shim cdylib (passed to `tfs exec` via TEBAKO_TFS_PRELOAD).
    shim: PathBuf,
}

fn target_dir() -> PathBuf {
    if let Ok(t) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(t);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .canonicalize()
        .unwrap()
}

fn bin() -> Option<PathBuf> {
    let target = target_dir();
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tfs");
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

fn shim_path() -> Option<PathBuf> {
    let name = if cfg!(target_os = "macos") {
        "libtfs_preload.dylib"
    } else {
        "libtfs_preload.so"
    };
    let target = target_dir();
    for profile in ["debug", "release"] {
        for cand in [
            target.join(profile).join(name),
            target.join(profile).join("deps").join(name),
        ] {
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn compile(src: &Path, out: &Path, extra: &[&str]) {
    let o = Command::new("cc")
        .arg("-O2")
        .arg("-o")
        .arg(out)
        .arg(src)
        .args(extra)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "cc failed for {}: {}",
        src.display(),
        String::from_utf8_lossy(&o.stderr)
    );
}

fn fixtures() -> Option<&'static Fixtures> {
    static FIX: OnceLock<Option<Fixtures>> = OnceLock::new();
    FIX.get_or_init(build_fixtures).as_ref()
}

fn build_fixtures() -> Option<Fixtures> {
    if bin().is_none() {
        eprintln!("skip: tfs binary not built");
        return None;
    }
    let Some(shim) = shim_path() else {
        eprintln!(
            "skip: libtfs_preload cdylib not found in the target dir \
             (build it with `cargo build -p libtfs-preload`)"
        );
        return None;
    };
    if !cc_available() {
        eprintln!("skip: no C compiler (`cc`) on PATH");
        return None;
    }

    // The proof tools are shared with libtfs-preload's e2e.
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../libtfs-preload/tests/fixtures")
        .canonicalize()
        .unwrap();
    let dir = std::env::temp_dir().join(format!("tfs-exec-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    let dir = std::fs::canonicalize(&dir).unwrap();

    let mut binaries: Vec<(String, PathBuf)> = Vec::new();
    for name in ["print-data", "spawn-self", "dl-user"] {
        let src = src_dir.join(format!("{name}.c"));
        let out = dir.join("bin").join(name);
        let extra: &[&str] = if name == "dl-user" && cfg!(target_os = "linux") {
            &["-ldl"]
        } else {
            &[]
        };
        compile(&src, &out, extra);
        binaries.push((name.to_string(), out));
    }
    let libname = if cfg!(target_os = "macos") {
        "libplug.dylib"
    } else {
        "libplug.so"
    };
    let plug_out = dir.join("bin").join(libname);
    let shared_flag = if cfg!(target_os = "macos") {
        "-dynamiclib"
    } else {
        "-shared"
    };
    compile(&src_dir.join("plug.c"), &plug_out, &["-fPIC", shared_flag]);

    // The image: the tools (in-image entrypoints), the data file, the
    // plugin.
    let zip_path = dir.join("img.zip");
    {
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        let ro = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);
        let rx = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);
        use std::io::Write as _;
        // Explicit directory entries (C++ zip semantics).
        zw.add_directory("bin/", rx).unwrap();
        zw.add_directory("data/", rx).unwrap();
        zw.add_directory("lib/", rx).unwrap();
        for (name, out) in &binaries {
            zw.start_file(format!("bin/{name}"), rx).unwrap();
            zw.write_all(&std::fs::read(out).unwrap()).unwrap();
        }
        zw.start_file("data/secret.txt", ro).unwrap();
        zw.write_all(SECRET.as_bytes()).unwrap();
        zw.start_file(format!("lib/{libname}"), rx).unwrap();
        zw.write_all(&std::fs::read(&plug_out).unwrap()).unwrap();
        zw.finish().unwrap();
    }

    Some(Fixtures {
        dir,
        zip: zip_path,
        shim,
    })
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

fn tfs_exec(f: &Fixtures, args: &[&str]) -> Run {
    let mut full: Vec<String> = vec!["exec".to_string(), format!("{}:{MOUNT}", f.zip.display())];
    full.extend(args.iter().map(|s| s.to_string()));
    let out = Command::new(bin().unwrap())
        .args(&full)
        // Determinism: no ambient preload env from the harness. The shim
        // is passed explicitly via the documented override (the sibling-
        // of-exe default only holds when libtfs-preload was built as a
        // -p target — under `cargo test -p tfs-cli` alone the cdylib
        // exists only in deps/).
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove("LD_PRELOAD")
        .env_remove("TEBAKO_TFS_MOUNTS")
        .env_remove("TEBAKO_JAIL")
        .env("TEBAKO_TFS_PRELOAD", &f.shim)
        .output()
        .unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn libname() -> &'static str {
    if cfg!(target_os = "macos") {
        "libplug.dylib"
    } else {
        "libplug.so"
    }
}

// ---------------------------------------------------------------------
// The proofs
// ---------------------------------------------------------------------

#[test]
fn exec_runs_in_image_tool_and_reads_vfs_data() {
    let Some(f) = fixtures() else { return };
    // The ENTRYPOINT lives inside the image (materialized via dlmap2file);
    // the data file is read through the VFS with no extraction.
    let r = tfs_exec(f, &["--", "/tfs/bin/print-data", "/tfs/data/secret.txt"]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, SECRET, "stderr: {}", r.stderr);
}

#[test]
fn exec_host_entrypoint_sees_mounts() {
    let Some(f) = fixtures() else { return };
    let tool = f.dir.join("bin").join("print-data");
    let r = tfs_exec(f, &["--", tool.to_str().unwrap(), "/tfs/data/secret.txt"]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, SECRET, "stderr: {}", r.stderr);
}

#[test]
fn exec_deny_jail_eperm_and_plain_ok() {
    let Some(f) = fixtures() else { return };
    // deny: the host read fails EPERM (the tool exits with the errno).
    let r = tfs_exec(
        f,
        &["--jail", "deny", "--", "/tfs/bin/print-data", "/etc/hosts"],
    );
    assert_eq!(r.rc, libc::EPERM, "stderr: {}", r.stderr);
    assert!(
        r.stderr.contains("Operation not permitted"),
        "stderr: {}",
        r.stderr
    );
    // …and under deny the same process still reads the memfs (spec 08 §3).
    let r = tfs_exec(
        f,
        &[
            "--jail",
            "deny",
            "--",
            "/tfs/bin/print-data",
            "/tfs/data/secret.txt",
        ],
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, SECRET);
    // No policy: plain behavior.
    let r = tfs_exec(f, &["--", "/tfs/bin/print-data", "/etc/hosts"]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(!r.stdout.is_empty());
}

#[test]
fn exec_grandchild_stays_in_vfs() {
    let Some(f) = fixtures() else { return };
    let r = tfs_exec(f, &["--", "/tfs/bin/spawn-self", "/tfs/data/secret.txt"]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.matches(SECRET).count(), 2, "stdout: {}", r.stdout);
    assert!(r.stdout.contains("CHILD-ENV:ok"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("SPAWN-RC:0"), "stdout: {}", r.stdout);
}

#[test]
fn exec_dlopen_memfs_library() {
    let Some(f) = fixtures() else { return };
    let r = tfs_exec(
        f,
        &["--", "/tfs/bin/dl-user", &format!("/tfs/lib/{}", libname())],
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), "42", "stdout: {}", r.stdout);
}

#[test]
fn exec_usage_and_named_errors() {
    let Some(f) = fixtures() else { return };
    // Missing `--`.
    let out = Command::new(bin().unwrap())
        .args(["exec", f.zip.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("requires `--`"), "stderr: {stderr}");

    // A missing image.
    let out = Command::new(bin().unwrap())
        .args(["exec", "/no/such/image.zip", "--", "/bin/true"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Image not found"), "stderr: {stderr}");

    // A mount at "/" is refused (it would bypass the jail).
    let out = Command::new(bin().unwrap())
        .args(["exec", &format!("{}:/", f.zip.display()), "--", "/bin/true"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("'/'"), "stderr: {stderr}");

    // A malformed --jail spec.
    let r = tfs_exec(f, &["--jail", "frob", "--", "/bin/true"]);
    assert_eq!(r.rc, 1);
    assert!(
        r.stderr.contains("invalid jail spec"),
        "stderr: {}",
        r.stderr
    );
}
