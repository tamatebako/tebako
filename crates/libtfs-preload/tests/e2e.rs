//! E2E proofs for the preload interposition shim (spec 07 §8 tier 1).
//!
//! Each test injects the built `libtfs_preload` cdylib into a tiny
//! dynamically-linked C tool via `DYLD_INSERT_LIBRARIES` (macOS) /
//! `LD_PRELOAD` (linux) and asserts observable behavior:
//!
//! - memfs read/stat/readdir with NO extraction,
//! - host passthrough (inert shim and open policy),
//! - the spec 08 jail (deny → EPERM; memfs unaffected; ro grant → EROFS
//!   on writes; rw grant passes),
//! - the grandchild staying in the VFS (env propagation),
//! - dlopen of a memfs library via the dlmap2file host cache,
//! - named errors + EX_CONFIG (78) on misformatted env.
//!
//! Skip policy (documented in the crate README/spec): tests SKIP (pass
//! trivially, with a note on stderr) when no C compiler (`cc`) is on PATH
//! or the cdylib was not built in the target dir (e.g. under
//! `cargo test -p tfs-cli` alone). A cc that EXISTS but fails to compile
//! is a hard failure, never a silent skip.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const SECRET: &str = "VFS-SECRET-E2E\n";
const MOUNT: &str = "/tfs";

// ---------------------------------------------------------------------
// Fixture: compiled C tools + a zip image + the shim path
// ---------------------------------------------------------------------

struct Fixtures {
    /// Temp root (canonicalized — jail prefix matching compares canonical
    /// forms, and macOS temp dirs live behind /var -> /private/var).
    dir: PathBuf,
    /// The test image (zip backend).
    zip: PathBuf,
    /// The built shim cdylib.
    shim: PathBuf,
    /// A host directory with one readable file (jail grant fixture).
    work: PathBuf,
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

fn compile(cc: &str, src: &Path, out: &Path, extra: &[&str]) {
    let o = Command::new(cc)
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

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = std::env::temp_dir().join(format!("libtfs-preload-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(&dir).unwrap();

    // Compile the tools (dynamic linking is the default; that is the whole
    // point — the shim interposes the libc they link against).
    let mut tools: Vec<(String, PathBuf)> = Vec::new();
    for name in ["print-data", "list-dir", "dl-user", "spawn-self", "mk-dir"] {
        let src = src_dir.join(format!("{name}.c"));
        let out = dir.join("bin").join(name);
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let extra: &[&str] = if name == "dl-user" && cfg!(target_os = "linux") {
            &["-ldl"]
        } else {
            &[]
        };
        compile("cc", &src, &out, extra);
        tools.push((name.to_string(), out));
    }
    // The plugin (shared library) for the dlopen proof.
    let libname = if cfg!(target_os = "macos") {
        "libplug.dylib"
    } else {
        "libplug.so"
    };
    let plug_src = src_dir.join("plug.c");
    let plug_out = dir.join("bin").join(libname);
    let shared_flag = if cfg!(target_os = "macos") {
        "-dynamiclib"
    } else {
        "-shared"
    };
    compile("cc", &plug_src, &plug_out, &["-fPIC", shared_flag]);

    // The jail-grant host fixture.
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("hostfile.txt"), b"HOST-FILE\n").unwrap();

    // The test image (zip backend): data, a directory, the plugin.
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
        // Explicit directory entries: the zip backend addresses only
        // explicit "path/" dirs (C++ semantics).
        zw.add_directory("data/", rx).unwrap();
        zw.add_directory("dir/", rx).unwrap();
        zw.add_directory("lib/", rx).unwrap();
        zw.start_file("data/secret.txt", ro).unwrap();
        zw.write_all(SECRET.as_bytes()).unwrap();
        zw.start_file("dir/a.txt", ro).unwrap();
        zw.write_all(b"a\n").unwrap();
        zw.start_file("dir/b.txt", ro).unwrap();
        zw.write_all(b"b\n").unwrap();
        zw.start_file(format!("lib/{libname}"), rx).unwrap();
        zw.write_all(&std::fs::read(&plug_out).unwrap()).unwrap();
        zw.finish().unwrap();
    }

    Some(Fixtures {
        dir,
        zip: zip_path,
        shim,
        work,
    })
}

// ---------------------------------------------------------------------
// Running tools under the shim
// ---------------------------------------------------------------------

fn preload_var() -> &'static str {
    if cfg!(target_os = "macos") {
        "DYLD_INSERT_LIBRARIES"
    } else {
        "LD_PRELOAD"
    }
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

/// Run a fixture tool under the shim. `jail` is the TEBAKO_JAIL value
/// (None = unset). The preload/mount env is set ONLY on the child.
fn run(f: &Fixtures, tool: &str, args: &[&str], jail: Option<&str>) -> Run {
    let mut cmd = Command::new(f.dir.join("bin").join(tool));
    cmd.args(args)
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", format!("{}:{MOUNT}", f.zip.display()))
        // Determinism: no inherited preload env from the test harness.
        .env_remove("DYLD_PRINT_LIBRARIES");
    match jail {
        Some(j) => {
            cmd.env("TEBAKO_JAIL", j);
        }
        None => {
            cmd.env_remove("TEBAKO_JAIL");
        }
    }
    let out = cmd.output().unwrap();
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
fn memfs_read_and_stat_no_extraction() {
    let Some(f) = fixtures() else { return };
    let r = run(
        f,
        "print-data",
        &[&format!("{MOUNT}/data/secret.txt")],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    // /tfs does not exist on the host: byte-correct output can only have
    // come through the VFS (stat+open+read+close, no extraction).
    assert_eq!(r.stdout, SECRET, "stderr: {}", r.stderr);
}

#[test]
fn memfs_readdir() {
    let Some(f) = fixtures() else { return };
    let r = run(f, "list-dir", &[&format!("{MOUNT}/dir")], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    let mut names: Vec<&str> = r
        .stdout
        .lines()
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["a.txt", "b.txt"], "stdout: {}", r.stdout);
}

#[test]
fn host_passthrough_open_policy() {
    let Some(f) = fixtures() else { return };
    // No jail: host paths behave exactly as without the shim.
    let r = run(f, "print-data", &["/etc/hosts"], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(!r.stdout.is_empty());
}

#[test]
fn deny_jail_eperm_and_memfs_unaffected() {
    let Some(f) = fixtures() else { return };
    // deny: a host read fails EPERM (exit code == errno).
    let r = run(f, "print-data", &["/etc/hosts"], Some("deny"));
    assert_eq!(r.rc, libc::EPERM, "stderr: {}", r.stderr);
    assert!(
        r.stderr.contains("Operation not permitted"),
        "stderr: {}",
        r.stderr
    );
    // …while the same process reads the memfs unimpeded (spec 08 §3:
    // the policy is about HOST paths; memfs mounts are unaffected).
    let r = run(
        f,
        "print-data",
        &[&format!("{MOUNT}/data/secret.txt")],
        Some("deny"),
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, SECRET);
}

#[test]
fn scoped_jail_grants_and_write_gating() {
    let Some(f) = fixtures() else { return };
    let ro_jail = format!("deny;{}:/work:ro", f.work.display());
    // ro grant: the file inside is readable…
    let r = run(
        f,
        "print-data",
        &[f.work.join("hostfile.txt").to_str().unwrap()],
        Some(&ro_jail),
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, "HOST-FILE\n");
    // …outside the grant is still EPERM…
    let r = run(f, "print-data", &["/etc/hosts"], Some(&ro_jail));
    assert_eq!(r.rc, libc::EPERM, "stderr: {}", r.stderr);
    // …and a WRITE against the ro grant is EROFS (docker-style).
    let r = run(
        f,
        "mk-dir",
        &[f.work.join("newdir").to_str().unwrap()],
        Some(&ro_jail),
    );
    assert_eq!(r.rc, libc::EROFS, "stderr: {}", r.stderr);
    assert!(!f.work.join("newdir").exists());
    // An rw grant passes the write through.
    let made = f.dir.join("made");
    std::fs::create_dir_all(&made).unwrap();
    let r = run(
        f,
        "mk-dir",
        &[made.join("sub").to_str().unwrap()],
        Some(&format!("deny;{}:/made:rw", made.display())),
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(made.join("sub").is_dir());
}

#[test]
fn memfs_writes_are_erofs() {
    let Some(f) = fixtures() else { return };
    // Write-class ops against a memfs path: payload images are always ro.
    let r = run(f, "mk-dir", &[&format!("{MOUNT}/data/newdir")], None);
    assert_eq!(r.rc, libc::EROFS, "stderr: {}", r.stderr);
}

#[test]
fn grandchild_stays_in_vfs() {
    let Some(f) = fixtures() else { return };
    let r = run(
        f,
        "spawn-self",
        &[&format!("{MOUNT}/data/secret.txt")],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    // Parent read + child read of the memfs file…
    assert_eq!(r.stdout.matches(SECRET).count(), 2, "stdout: {}", r.stdout);
    // …and the preload env propagated (the process tree stays in the VFS).
    assert!(r.stdout.contains("CHILD-ENV:ok"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("SPAWN-RC:0"), "stdout: {}", r.stdout);
}

#[test]
fn dlopen_memfs_library_via_dlmap2file() {
    let Some(f) = fixtures() else { return };
    let r = run(f, "dl-user", &[&format!("{MOUNT}/lib/{}", libname())], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), "42", "stdout: {}", r.stdout);
}

#[test]
fn misformatted_env_is_a_named_error() {
    let Some(f) = fixtures() else { return };
    // Relative image path in TEBAKO_TFS_MOUNTS.
    let mut cmd = Command::new(f.dir.join("bin").join("print-data"));
    cmd.arg("/etc/hosts")
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", "relative/img.zip:/tfs");
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(tfs_preload::spec::EX_CONFIG));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("TEBAKO_TFS_MOUNTS"), "stderr: {stderr}");
    assert!(stderr.contains("not absolute"), "stderr: {stderr}");

    // Garbage TEBAKO_JAIL.
    let mut cmd = Command::new(f.dir.join("bin").join("print-data"));
    cmd.arg("/etc/hosts")
        .env(preload_var(), &f.shim)
        .env("TEBAKO_JAIL", "frob");
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(tfs_preload::spec::EX_CONFIG));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("TEBAKO_JAIL"), "stderr: {stderr}");

    // An image that does not exist.
    let mut cmd = Command::new(f.dir.join("bin").join("print-data"));
    cmd.arg("/etc/hosts")
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", "/no/such/image.zip:/tfs");
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(tfs_preload::spec::EX_CONFIG));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot mount"), "stderr: {stderr}");
}
