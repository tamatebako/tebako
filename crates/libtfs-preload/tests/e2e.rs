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
//! - named errors + EX_CONFIG (78) on misformatted env,
//! - roadmap 39: the *at family (fstatat/statx — linux —, getdents64,
//!   the pre-glibc-2.33 `__xstat`; dirfd-relative resolution into the
//!   memfs; the AT_FDCWD regression pin), dir positioning
//!   (readdir_r/telldir/seekdir/rewinddir), execve/posix_spawn/
//!   posix_spawnp of an in-image helper via the dlmap2file host cache,
//!   and a rust-built dynamic tool proving the *at coverage end-to-end.
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
    let mut names = vec![
        "print-data",
        "list-dir",
        "dl-user",
        "spawn-self",
        "mk-dir",
        "at-probe",
        "dir-walk",
        "spawn-helper",
        "helper",
    ];
    if cfg!(target_os = "linux") {
        names.push("dents-probe");
    }
    for name in names {
        let src = src_dir.join(format!("{name}.c"));
        let out = dir.join("bin").join(name);
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let extra: &[&str] = if (name == "dl-user" || name == "dents-probe")
            && cfg!(target_os = "linux")
        {
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

    // The test image (zip backend): data, a directory, the plugin, the
    // in-image helper binary (exec/spawn proofs).
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
        zw.add_directory("bin/", rx).unwrap();
        zw.start_file("data/secret.txt", ro).unwrap();
        zw.write_all(SECRET.as_bytes()).unwrap();
        zw.start_file("dir/a.txt", ro).unwrap();
        zw.write_all(b"a\n").unwrap();
        zw.start_file("dir/b.txt", ro).unwrap();
        zw.write_all(b"b\n").unwrap();
        zw.start_file(format!("lib/{libname}"), rx).unwrap();
        zw.write_all(&std::fs::read(&plug_out).unwrap()).unwrap();
        zw.start_file("bin/helper", rx).unwrap();
        zw.write_all(&std::fs::read(dir.join("bin").join("helper")).unwrap())
            .unwrap();
        zw.finish().unwrap();
    }

    // The rust-built dynamic tool (roadmap 39): a workspace member, built
    // on demand so `cargo test -p libtfs-preload` alone has it too. cargo
    // is always on PATH under cargo test; a failed build is a hard
    // failure, never a silent skip.
    {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let o = Command::new("cargo")
            .args(["build", "-p", "tfs-preload-rust-at-tool"])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "building the rust-at-tool fixture failed: {}",
            String::from_utf8_lossy(&o.stderr)
        );
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
    run_full(f, tool, args, jail, &[], None)
}

/// Full form: `extra_mounts` append `image:mount` pairs to
/// TEBAKO_TFS_MOUNTS; `cwd` sets the child's working directory (the
/// AT_FDCWD-relative proofs need a known cwd).
fn run_full(
    f: &Fixtures,
    tool: &str,
    args: &[&str],
    jail: Option<&str>,
    extra_mounts: &[String],
    cwd: Option<&Path>,
) -> Run {
    let mut mounts = format!("{}:{MOUNT}", f.zip.display());
    for m in extra_mounts {
        mounts.push(',');
        mounts.push_str(m);
    }
    let mut cmd = Command::new(f.dir.join("bin").join(tool));
    cmd.args(args)
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", mounts)
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
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    let out = cmd.output().unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// The built rust-at-tool fixture binary (built in build_fixtures).
fn rust_at_tool() -> PathBuf {
    let target = target_dir();
    for profile in ["debug", "release"] {
        let cand = target
            .join(profile)
            .join(if cfg!(windows) {
                "rust-at-tool.exe"
            } else {
                "rust-at-tool"
            });
        if cand.is_file() {
            return cand;
        }
    }
    panic!("rust-at-tool fixture binary not found under {}", target.display())
}

/// Run the rust-at-tool fixture under the shim (it lives outside the
/// fixture bin dir, so it gets its own runner).
fn run_rust(f: &Fixtures, args: &[&str], cwd: Option<&Path>) -> Run {
    let mut cmd = Command::new(rust_at_tool());
    cmd.args(args)
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", format!("{}:{MOUNT}", f.zip.display()))
        .env_remove("TEBAKO_JAIL")
        .env_remove("DYLD_PRINT_LIBRARIES");
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
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

// ---------------------------------------------------------------------
// roadmap 39: the *at family, exec/spawn of memfs paths, dir positioning
// ---------------------------------------------------------------------

#[test]
fn fstatat_and_statx_memfs() {
    let Some(f) = fixtures() else { return };
    let r = run(f, "at-probe", &[&format!("{MOUNT}/data/secret.txt")], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    // /tfs does not exist on the host: the *at answers came from the VFS.
    assert!(r.stdout.contains("FSTATAT:15"), "stdout: {}", r.stdout);
    if cfg!(target_os = "linux") {
        assert!(r.stdout.contains("STATX:15"), "stdout: {}", r.stdout);
    }
}

/// THE AT_FDCWD regression pin (the 4.0 lesson): TEBAKO_FD_FLAG is a bit
/// check and AT_FDCWD (-100) has that bit set. An ungated is_memfs_fd
/// branch in a *at shim answers ENOTDIR for a cwd-relative HOST path;
/// the gated shim passes it through.
#[test]
fn at_fdcwd_relative_host_passthrough_regression() {
    let Some(f) = fixtures() else { return };
    let r = run_full(f, "at-probe", &["--rel", "hostfile.txt"], None, &[], Some(&f.work));
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), "REL:10", "stdout: {}", r.stdout);
}

#[test]
fn at_dirfd_relative_into_memfs() {
    let Some(f) = fixtures() else { return };
    // A second mount INSIDE a host directory, so a dirfd-relative path
    // resolves into the memfs through the dirfd's own host path.
    let mnt = f.work.join("mnt");
    let extra = format!("{}:{}", f.zip.display(), mnt.display());
    let r = run_full(
        f,
        "at-probe",
        &["--dirfd", f.work.to_str().unwrap(), "mnt/data/secret.txt"],
        None,
        &[extra],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("FSTATAT:15"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains(SECRET.trim_end()), "stdout: {}", r.stdout);
}

#[cfg(target_os = "linux")]
#[test]
fn getdents64_and_xstat() {
    let Some(f) = fixtures() else { return };
    let r = run(
        f,
        "dents-probe",
        &[&format!("{MOUNT}/data/secret.txt"), f.work.to_str().unwrap()],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(
        r.stdout.contains("DENTS-MEMFS:ENOTDIR"),
        "stdout: {}",
        r.stdout
    );
    assert!(r.stdout.contains("DENTS-HOST:ok"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("XSTAT:15"), "stdout: {}", r.stdout);
}

#[test]
fn dir_positioning_and_readdir_r() {
    let Some(f) = fixtures() else { return };
    // memfs: index-based cookies, fully deterministic.
    let r = run(f, "dir-walk", &[&format!("{MOUNT}/dir")], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(
        r.stdout, "R1:a.txt\nTELL:1\nR2:b.txt\nBACK:b.txt\nREW:a.txt\nEND:eod\n",
        "stdout: {}",
        r.stdout
    );
    // host passthrough: cookie VALUES are the kernel's, but the seek/
    // rewind behavior is deterministic (dot entries are skipped).
    let r = run(f, "dir-walk", &[f.work.to_str().unwrap()], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    let lines: Vec<&str> = r.stdout.lines().collect();
    assert_eq!(lines.first(), Some(&"R1:hostfile.txt"), "stdout: {}", r.stdout);
    assert!(lines.get(1).is_some_and(|l| l.starts_with("TELL:")));
    assert_eq!(lines.get(2), Some(&"R2:eod"), "stdout: {}", r.stdout);
    assert_eq!(lines.get(3), Some(&"BACK:eod"), "stdout: {}", r.stdout);
    assert_eq!(lines.get(4), Some(&"REW:hostfile.txt"), "stdout: {}", r.stdout);
    assert_eq!(lines.get(5), Some(&"END:eod"), "stdout: {}", r.stdout);
}

#[test]
fn posix_spawn_of_memfs_helper() {
    let Some(f) = fixtures() else { return };
    let args = [
        "--spawn",
        &format!("{MOUNT}/bin/helper"),
        &format!("{MOUNT}/data/secret.txt"),
    ];
    let r = run(f, "spawn-helper", &args, None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("HELPER:ok"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains(SECRET.trim_end()), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("SPAWN-RC:0"), "stdout: {}", r.stdout);
    // …and under a deny jail the memfs helper still spawns (spec 08 §3:
    // the policy is about HOST paths; memfs mounts are unaffected).
    let r = run(f, "spawn-helper", &args, Some("deny"));
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("SPAWN-RC:0"), "stdout: {}", r.stdout);
}

#[test]
fn posix_spawnp_of_memfs_helper() {
    let Some(f) = fixtures() else { return };
    let r = run(
        f,
        "spawn-helper",
        &[
            "--spawnp",
            &format!("{MOUNT}/bin/helper"),
            &format!("{MOUNT}/data/secret.txt"),
        ],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("SPAWN-RC:0"), "stdout: {}", r.stdout);
}

#[test]
fn execve_of_memfs_helper() {
    let Some(f) = fixtures() else { return };
    let r = run(
        f,
        "spawn-helper",
        &[
            "--execve",
            &format!("{MOUNT}/bin/helper"),
            &format!("{MOUNT}/data/secret.txt"),
        ],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("HELPER:ok"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains(SECRET.trim_end()), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("SPAWN-RC:0"), "stdout: {}", r.stdout);
}

#[test]
fn spawn_of_host_path_denied() {
    let Some(f) = fixtures() else { return };
    // Jails extend to exec: posix_spawn of a host binary under a deny
    // jail fails with the error NUMBER as posix_spawn's return value.
    let r = run(
        f,
        "spawn-helper",
        &["--spawn", "/bin/echo", "never"],
        Some("deny"),
    );
    assert_eq!(r.rc, libc::EPERM, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("SPAWN-ERR:1"), "stdout: {}", r.stdout);
}

/// The rust-built dynamic tool (roadmap 39): std::fs plus direct
/// fstatat/statx against in-image data — the *at coverage end-to-end.
#[test]
fn rust_tool_at_family_end_to_end() {
    let Some(f) = fixtures() else { return };
    let r = run_rust(
        f,
        &[&format!("{MOUNT}/data/secret.txt"), "hostfile.txt"],
        Some(&f.work),
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains(SECRET.trim_end()), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("META:15"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("FSTATAT:15"), "stdout: {}", r.stdout);
    if cfg!(target_os = "linux") {
        assert!(r.stdout.contains("STATX:15"), "stdout: {}", r.stdout);
    }
    assert!(r.stdout.contains("REL:10"), "stdout: {}", r.stdout);
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
