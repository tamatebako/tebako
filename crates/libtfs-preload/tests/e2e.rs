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
//! - the slot form of `TEBAKO_TFS_MOUNTS` (spec 17 §2.1): a packaged
//!   payload's slot mounts its region, the child hand-off preserves it,
//!   and resolution failures are named EX_CONFIG errors,
//! - dlopen of a memfs library via the dlmap2file host cache,
//! - the linux LFS64+fortify surface the JDK needs (lseek64 SEEK_END
//!   probe, __read_chk fortified read, __fxstat64, anonymous mmap with
//!   the fd -1 bit-test lie, mmap64 CEN window on a flagged memfs fd —
//!   spec 22 class E),
//! - the tebako#439 alias surface (fopen64/openat64/__openat_2/
//!   __fxstatat64 — the LFS64/fortify/versioned twins of already-covered
//!   names, incl. OpenSSL 3.6's `openssl_fopen` → `fopen64`): the jail
//!   gates each alias exactly like its plain-name twin,
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
    /// The stitched package: the zip image as slot 0 plus a tpkg trailer
    /// (spec 17 §2.1's slot-form proofs).
    pkg: PathBuf,
    /// The fork-exec test image (DWARFS backend — the fork/exec
    /// regression needs a backend with a worker pool; zip has none).
    dwarfs: PathBuf,
    /// The built shim cdylib.
    shim: PathBuf,
    /// A host directory with one readable file (jail grant fixture).
    work: PathBuf,
    /// The Rust dynamic tool (roadmap 39), when rustc is available.
    rust_tool: Option<PathBuf>,
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
    for name in [
        "print-data",
        "list-dir",
        "dl-user",
        "spawn-self",
        "mk-dir",
        "at-probe",
        "spawn-helper",
        "dir-stream",
        "mmap-probe",
        "close-probe",
        "fork-exec",
        "alias-probe",
    ] {
        let src = src_dir.join(format!("{name}.c"));
        let out = dir.join("bin").join(name);
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let extra: &[&str] = if name == "dl-user" && cfg!(target_os = "linux") {
            &["-ldl"]
        } else if name == "mmap-probe" && cfg!(target_os = "linux") {
            // fortify + LFS64: the probe's read/fstat compile to
            // __read_chk/__fxstat64 — the JDK's exact fortified entry
            // points (spec 22 class E).
            &["-D_FORTIFY_SOURCE=2", "-D_FILE_OFFSET_BITS=64", "-O1"]
        } else {
            &[]
        };
        compile("cc", &src, &out, extra);
    }
    // The Rust dynamic tool (roadmap 39's std::fs proof): skipped with a
    // note when no rustc is on PATH; a rustc that EXISTS but fails is a
    // hard failure (the documented skip policy).
    let rust_tool = dir.join("bin").join("rust-tool");
    let rust_built = Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let rust_tool = if rust_built {
        let o = Command::new("rustc")
            .arg("-O")
            .arg("-o")
            .arg(&rust_tool)
            .arg(src_dir.join("rust-tool.rs"))
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "rustc failed: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        Some(rust_tool)
    } else {
        eprintln!("skip: no rustc on PATH (the rust-tool proofs are skipped)");
        None
    };
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

    // The test image (zip backend): data, a directory, the plugin, and an
    // in-image helper binary (print-data — the spawn-helper proofs exec it
    // straight from the image, roadmap 39).
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
        let print_data = dir.join("bin").join("print-data");
        zw.start_file("bin/print-data", rx).unwrap();
        zw.write_all(&std::fs::read(&print_data).unwrap()).unwrap();
        zw.finish().unwrap();
    }

    // The stitched-package fixture (spec 17 §2.1): the zip image bytes as
    // slot 0 of a tpkg package. The slot form mounts the region — the
    // whole-file spelling would leave the trailer inside the sniffed
    // bytes (tebako#455).
    let pkg_path = dir.join("pkg.tebako");
    {
        let zip_len = std::fs::copy(&zip_path, &pkg_path).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&pkg_path)
            .unwrap();
        let manifest = tpkg::Manifest {
            slots: vec![tpkg::Slot::new(0, zip_len, tpkg::TPKG_FORMAT_ZIP, "/tfs")],
            ..Default::default()
        };
        tpkg::write_to(&mut file, &manifest).unwrap();
    }

    // The fork-exec image (DWARFS backend — its block-cache worker pool is
    // the fork hazard the guard exists for). Contents matter: an in-image
    // `__tpkg__/manifest.yaml` WITHOUT the java_home annotation (the exec
    // materialization probe reads exactly this file), plus one data file.
    let dwarfs_path = dir.join("img.dwarfs");
    {
        let src = dir.join("dwarfs-src");
        std::fs::create_dir_all(src.join("__tpkg__")).unwrap();
        std::fs::create_dir_all(src.join("data")).unwrap();
        // A minimal valid payload manifest WITHOUT the java_home
        // annotation (the exec materialization probe reads exactly this
        // file; the tolerant walk answers false → the closure walk).
        let manifest = [
            "schema_version: 1",
            "identity:",
            "  schema_version: 1",
            "  kind: app",
            "  name: fork-exec-e2e",
            "  version: 0.0.1",
            "  producer: {tool: libtfs-preload-e2e, tool_version: \"1\"}",
            "  created: \"2026-08-22T00:00:00Z\"",
            "  source:",
            "    commit: \"0000000000000000000000000000000000000000\"",
            "    builder: local",
            "  digest:",
            "    tree_hash: \"sha256:0000000000000000000000000000000000000000000000000000000000000000\"",
            "    blob_sha256: \"0000000000000000000000000000000000000000000000000000000000000000\"",
            "  signing: {state: unsigned}",
            "  encryption: {state: none}",
            "",
        ]
        .join("\n");
        std::fs::write(src.join("__tpkg__/manifest.yaml"), manifest).unwrap();
        std::fs::write(src.join("data/secret.txt"), SECRET.as_bytes()).unwrap();
        let mut writer =
            dwarfs_t::Writer::new(dwarfs_t::WriterOptions::default()).expect("dwarfs writer");
        writer.add_tree(&src, "/").expect("dwarfs writer: scan");
        writer.write(&dwarfs_path).expect("dwarfs writer: write");
    }

    Some(Fixtures {
        dir,
        zip: zip_path,
        pkg: pkg_path,
        dwarfs: dwarfs_path,
        shim,
        work,
        rust_tool,
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
    /// The killing signal (unix; None on a clean exit and elsewhere) —
    /// `code()` maps a signal death to None, which used to read as a
    /// bare "rc: -1" with no name.
    signal: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Run a fixture tool under the shim. `jail` is the TEBAKO_JAIL value
/// (None = unset). The preload/mount env is set ONLY on the child.
fn run(f: &Fixtures, tool: &str, args: &[&str], jail: Option<&str>) -> Run {
    let mounts = format!("{}:{MOUNT}", f.zip.display());
    run_with_mounts(f, tool, args, &mounts, jail)
}

/// [`run`] with an explicit `TEBAKO_TFS_MOUNTS` value (the slot-form
/// proofs).
fn run_with_mounts(
    f: &Fixtures,
    tool: &str,
    args: &[&str],
    mounts: &str,
    jail: Option<&str>,
) -> Run {
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
    let out = cmd.output().unwrap();
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&out.status);
    #[cfg(not(unix))]
    let signal = None;
    Run {
        rc: out.status.code().unwrap_or(-1),
        signal,
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

/// Spec 25 §2's arming contract holds for the preload delivery, not just
/// the driver: `TEBAKO_TRACE` arms the interception bus at the shim's
/// constructor, BEFORE any mount — the stream carries the mount decision,
/// the dispatched stat+open of the memfs file, and (a second process
/// appending to the same channel — §2's re-derivation clause) the host
/// passthrough's `host` verdict.
#[test]
fn trace_bus_arms_from_tebako_trace_env() {
    let Some(f) = fixtures() else { return };
    let capture = f.dir.join("trace-capture.jsonl");
    let _ = std::fs::remove_file(&capture);
    let traced = |arg: &str| {
        let mut cmd = Command::new(f.dir.join("bin").join("print-data"));
        cmd.arg(arg)
            .env(preload_var(), &f.shim)
            .env("TEBAKO_TFS_MOUNTS", format!("{}:{MOUNT}", f.zip.display()))
            .env(tfs::trace::TRACE_ENV, &capture)
            .env_remove("DYLD_PRINT_LIBRARIES");
        cmd.output().unwrap()
    };

    let out = traced(&format!("{MOUNT}/data/secret.txt"));
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, SECRET.as_bytes());
    // A second process re-derives the channel from the inherited env and
    // appends (§2's children clause; the channel is append-mode).
    let out = traced("/etc/hosts");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = std::fs::read_to_string(&capture).expect("the armed bus wrote the capture");
    let mount = text
        .lines()
        .find(|l| l.contains("\"op\":\"mount\""))
        .unwrap_or_else(|| panic!("a mount event was traced: {text}"));
    assert!(
        mount.contains(&format!("\"path\":\"{MOUNT}\"")) && mount.contains("\"verdict\":\"ok\""),
        "{mount}"
    );
    let open = text
        .lines()
        .find(|l| l.contains("\"op\":\"open\"") && l.contains("secret.txt"))
        .unwrap_or_else(|| panic!("an open event was traced: {text}"));
    assert!(open.contains("\"verdict\":\"image:/tfs\""), "{open}");
    let stat = text
        .lines()
        .find(|l| l.contains("\"op\":\"stat\"") && l.contains("secret.txt"))
        .unwrap_or_else(|| panic!("a stat event was traced: {text}"));
    assert!(stat.contains("\"verdict\":\"image:/tfs\""), "{stat}");
    let host = text
        .lines()
        .find(|l| l.contains("/etc/hosts"))
        .unwrap_or_else(|| panic!("the host passthrough was traced: {text}"));
    assert!(host.contains("\"verdict\":\"host\""), "{host}");
    // Every line carries the schema's envelope keys (v, pid, tid).
    for line in text.lines() {
        assert!(
            line.contains("\"v\":1") && line.contains("\"pid\":") && line.contains("\"tid\":"),
            "{line}"
        );
    }
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

// ---------------------------------------------------------------------
// spec 17 §2.1: the slot form of TEBAKO_TFS_MOUNTS (tebako#455)
// ---------------------------------------------------------------------

/// The consume side: a packaged payload's slot mounts its REGION. (The
/// whole-file spelling of a package leaves the appended trailer inside
/// the sniffed bytes — the packed-mn PDF leg's EINVAL/78.)
#[test]
fn slot_form_mounts_the_package_region() {
    let Some(f) = fixtures() else { return };
    let mounts = format!("{}:0:{MOUNT}", f.pkg.display());
    let r = run_with_mounts(
        f,
        "print-data",
        &[&format!("{MOUNT}/data/secret.txt")],
        &mounts,
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.matches(SECRET).count(), 1, "stdout: {}", r.stdout);
}

/// The issue's acceptance shape: a process holding a slot mount spawns a
/// child that reads a file through the mount — the slot form survives the
/// hand-off, so the child mounts the same region, never the whole
/// package file.
#[test]
fn slot_form_grandchild_reads_through_the_mount() {
    let Some(f) = fixtures() else { return };
    let mounts = format!("{}:0:{MOUNT}", f.pkg.display());
    let r = run_with_mounts(
        f,
        "spawn-self",
        &[&format!("{MOUNT}/data/secret.txt")],
        &mounts,
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.matches(SECRET).count(), 2, "stdout: {}", r.stdout);
    assert!(r.stdout.contains("CHILD-ENV:ok"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("SPAWN-RC:0"), "stdout: {}", r.stdout);
}

/// Slot-resolution failures are named errors (spec 17 §2.1), exit 78 —
/// never a silent whole-file fallback.
#[test]
fn slot_form_failures_are_named_errors() {
    let Some(f) = fixtures() else { return };
    // Out of range on a packaged file.
    let mounts = format!("{}:1:{MOUNT}", f.pkg.display());
    let r = run_with_mounts(f, "print-data", &["/etc/hosts"], &mounts, None);
    assert_eq!(r.rc, tfs_preload::spec::EX_CONFIG, "stderr: {}", r.stderr);
    assert!(r.stderr.contains("slot 1"), "stderr: {}", r.stderr);
    assert!(r.stderr.contains("out of range"), "stderr: {}", r.stderr);
    // A non-zero slot on a bare image (no slot table).
    let mounts = format!("{}:3:{MOUNT}", f.zip.display());
    let r = run_with_mounts(f, "print-data", &["/etc/hosts"], &mounts, None);
    assert_eq!(r.rc, tfs_preload::spec::EX_CONFIG, "stderr: {}", r.stderr);
    assert!(r.stderr.contains("no slot table"), "stderr: {}", r.stderr);
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

// ---------------------------------------------------------------------
// Roadmap 39: the *at family, execve/posix_spawn of memfs paths, dir
// streams, and the Rust dynamic tool
// ---------------------------------------------------------------------

/// Run a fixture tool with an explicit cwd (the AT_FDCWD proofs).
fn run_in_dir(f: &Fixtures, cwd: &Path, tool: &str, args: &[&str], jail: Option<&str>) -> Run {
    let mut cmd = Command::new(f.dir.join("bin").join(tool));
    cmd.args(args)
        .current_dir(cwd)
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", format!("{}:{MOUNT}", f.zip.display()))
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
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&out.status);
    #[cfg(not(unix))]
    let signal = None;
    Run {
        rc: out.status.code().unwrap_or(-1),
        signal,
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn at_family_serves_memfs_and_gates_at_fdcwd() {
    let Some(f) = fixtures() else { return };
    // fstatat on a memfs path: the engine answers (no extraction).
    let r = run(
        f,
        "at-probe",
        &["fstatat", &format!("{MOUNT}/data/secret.txt")],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), format!("SIZE:{}", SECRET.len()));

    // dirfd-relative through a HOST dirfd: passes through to the host.
    let r = run(
        f,
        "at-probe",
        &["fstatat-rel", "hostfile.txt", f.work.to_str().unwrap()],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), "SIZE:10");
    let r = run(
        f,
        "at-probe",
        &["openat", "hostfile.txt", f.work.to_str().unwrap()],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, "HOST-FILE\n");

    // The AT_FDCWD regression pin, e2e form: a cwd-relative fstatat must
    // resolve against the cwd — never ENOTDIR (the fd-branch bug class
    // that broke runtime builds).
    let r = run_in_dir(f, &f.work, "at-probe", &["fstatat", "hostfile.txt"], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), "SIZE:10");
    // …and the jail gates the same call (cwd outside the grant → EPERM).
    let r = run_in_dir(
        f,
        &f.work,
        "at-probe",
        &["fstatat", "hostfile.txt"],
        Some("deny"),
    );
    assert_eq!(r.rc, libc::EPERM, "stderr: {}", r.stderr);
}

#[cfg(target_os = "linux")]
#[test]
fn at_family_linux_extensions() {
    let Some(f) = fixtures() else { return };
    let secret = format!("{MOUNT}/data/secret.txt");

    // statx: the engine's answer in statx form, with the mask reported.
    let r = run(f, "at-probe", &["statx", &secret], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(
        r.stdout
            .starts_with(&format!("SIZE:{} MASK:", SECRET.len())),
        "stdout: {}",
        r.stdout
    );

    // fstatat64 (the LFS alias).
    let r = run(f, "at-probe", &["fstatat64", &secret], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), format!("SIZE:{}", SECRET.len()));

    // The versioned pre-glibc-2.33 entry points.
    let r = run(f, "at-probe", &["__xstat", &secret], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), format!("SIZE:{}", SECRET.len()));
    let r = run(f, "at-probe", &["__fxstatat", &secret], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), format!("SIZE:{}", SECRET.len()));

    // getdents64 on a host directory passes through…
    let r = run(
        f,
        "at-probe",
        &["getdents64", f.work.to_str().unwrap()],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.starts_with("BYTES:"), "stdout: {}", r.stdout);
    let bytes: i64 = r
        .stdout
        .trim_start_matches("BYTES:")
        .trim()
        .parse()
        .unwrap();
    assert!(bytes > 0, "stdout: {}", r.stdout);
    // …and on a memfs REGULAR file fd the honest answer is ENOTDIR (20).
    let r = run(f, "at-probe", &["getdents64-file", &secret], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), format!("ERRNO:{}", libc::ENOTDIR));
}

/// tebako#439 — the LFS64/fortify/versioned ALIAS surface is the same
/// jail. OpenSSL 3.6's BIO file path binds `fopen64` (crypto/o_fopen.c
/// defines `_FILE_OFFSET_BITS=64` itself on linux, so `openssl_fopen` —
/// the `BIO_new_file`/`X509_LOOKUP_load_file` choke point behind ruby's
/// `X509::Store#add_file` — tail-calls `fopen64@plt`, never `fopen`);
/// Rust std binds `openat64`, vendored fortify C++ binds `__openat_2`,
/// and `__fxstatat64` completes the versioned stat family (all four
/// imported by the 0.16.6 linux-gnu runtime exe). Each alias must serve
/// the memfs, fail EPERM under a deny jail, and pass an ro grant —
/// exactly like its plain-name twin.
#[cfg(target_os = "linux")]
#[test]
fn linux_alias_surface_jail_parity() {
    let Some(f) = fixtures() else { return };
    let secret = format!("{MOUNT}/data/secret.txt");

    // The alias serves the memfs (fopen64 delegates to the fopen body:
    // the Vfs answer materializes through dlmap2file like any stdio
    // read) — and keeps working under a deny jail (spec 08 §3).
    for jail in [None, Some("deny")] {
        let r = run(f, "alias-probe", &["fopen64", &secret], jail);
        assert_eq!(
            r.rc, 0,
            "fopen64 memfs, jail {jail:?}, stderr: {}",
            r.stderr
        );
        assert_eq!(r.stdout, SECRET, "fopen64 memfs stdout");
    }

    // The hole's shape: a denied HOST path through each alias → EPERM.
    for leg in ["fopen64", "openat64", "__openat_2", "__fxstatat64"] {
        let r = run(f, "alias-probe", &[leg, "/etc/hosts"], Some("deny"));
        assert_eq!(
            r.rc,
            libc::EPERM,
            "{leg} of a denied host path must fail EPERM, stderr: {}",
            r.stderr
        );
    }

    // The allowed-path control: an ro grant passes every read alias.
    let ro_jail = format!("deny;{}:/work:ro", f.work.display());
    let host = f.work.join("hostfile.txt");
    let host = host.to_str().unwrap();
    for leg in ["fopen64", "openat64", "__openat_2"] {
        let r = run(f, "alias-probe", &[leg, host], Some(&ro_jail));
        assert_eq!(r.rc, 0, "{leg} under the ro grant, stderr: {}", r.stderr);
        assert_eq!(r.stdout, "HOST-FILE\n", "{leg} stdout");
    }
    let r = run(f, "alias-probe", &["__fxstatat64", host], Some(&ro_jail));
    assert_eq!(
        r.rc, 0,
        "__fxstatat64 under the ro grant, stderr: {}",
        r.stderr
    );
    assert_eq!(r.stdout.trim(), "SIZE:10");
}

#[test]
fn dir_stream_rewind_tell_seek_and_readdir_r() {
    let Some(f) = fixtures() else { return };
    let r = run(f, "dir-stream", &[&format!("{MOUNT}/dir")], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    let expected = "r1:a.txt\ntell:1\nr2:b.txt\nr3:<end>\nafter-rewind:a.txt\nreaddir_r:0:b.txt\nreaddir_r:0:<end>\n";
    assert_eq!(r.stdout, expected, "stdout:\n{}", r.stdout);
}

#[test]
fn execve_of_memfs_helper_runs_without_extraction() {
    let Some(f) = fixtures() else { return };
    let helper = format!("{MOUNT}/bin/print-data");
    let data = format!("{MOUNT}/data/secret.txt");
    // execve replaces the process: the helper (materialized through the
    // dlmap2file host cache) prints the memfs data — proof the copy ran
    // AND the preload env reached it (it re-mounts the same image).
    let r = run(f, "spawn-helper", &["execve", &helper, &data], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, SECRET, "stderr: {}", r.stderr);
}

#[test]
fn posix_spawn_of_memfs_helper_and_deny_jail_child_io() {
    let Some(f) = fixtures() else { return };
    let helper = format!("{MOUNT}/bin/print-data");
    let data = format!("{MOUNT}/data/secret.txt");
    let r = run(f, "spawn-helper", &["posix_spawn", &helper, &data], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, SECRET, "stderr: {}", r.stderr);

    // Under a deny jail the memfs helper still spawns (memfs is
    // unaffected); the child's HOST read is what fails EPERM.
    let r = run(
        f,
        "spawn-helper",
        &["posix_spawn", &helper, "/etc/hosts"],
        Some("deny"),
    );
    assert_eq!(r.rc, libc::EPERM, "stderr: {}", r.stderr);
}

#[test]
fn rust_tool_reads_in_image_data() {
    let Some(f) = fixtures() else { return };
    if f.rust_tool.is_none() {
        return; // documented skip: no rustc on PATH
    }
    // std::fs::read_to_string (open/read/close through the shim).
    let r = run(
        f,
        "rust-tool",
        &["read", &format!("{MOUNT}/data/secret.txt")],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, SECRET);
    // std::fs::metadata (the glibc stat family internally).
    let r = run(
        f,
        "rust-tool",
        &["metadata", &format!("{MOUNT}/data/secret.txt")],
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), format!("SIZE:{}", SECRET.len()));
    // std::fs::read_dir (opendir/readdir through the shim).
    let r = run(f, "rust-tool", &["read_dir", &format!("{MOUNT}/dir")], None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout.trim(), "a.txt b.txt");
    // …and the deny jail gates the Rust binary identically.
    let r = run(f, "rust-tool", &["read", "/etc/hosts"], Some("deny"));
    assert_eq!(r.rc, libc::EPERM, "stderr: {}", r.stderr);
}

/// The linux LFS64+fortify surface (spec 22 class E): the JDK launcher
/// maps `JLI_Lseek` to `lseek64`, reads the END record through the
/// fortified `__read_chk`, stats via `__fxstat64`, and libzip mmaps the
/// jar central directory — all must be served by the shim on a flagged
/// memfs fd, never hit the kernel with the virtual fd (EBADF → "Invalid
/// or corrupt jarfile"). The anonymous-mmap probe pins the fd -1
/// bit-test lie (the JVM's PaX check): an ANONYMOUS request must pass
/// through to the host.
#[test]
fn linux_lseek64_and_mmap64_on_a_memfs_fd() {
    if !cfg!(target_os = "linux") {
        eprintln!("skip: lseek64/mmap64 are glibc entry points (linux only)");
        return;
    }
    let Some(f) = fixtures() else { return };
    let path = format!("{MOUNT}/data/secret.txt");
    let r = run(f, "mmap-probe", &[path.as_str()], None);
    assert_eq!(
        r.rc, 0,
        "mmap-probe failed (signal: {:?}), stderr: {} stdout: {}",
        r.signal, r.stderr, r.stdout
    );
    assert!(r.stdout.contains("anon-mmap:ok"), "stdout: {}", r.stdout);
    assert!(
        r.stdout.contains("lseek64-tail:E2E\n"),
        "stdout: {}",
        r.stdout
    );
    assert!(
        r.stdout.contains("mmap64-head:VFS-SECRET-E2E"),
        "stdout: {}",
        r.stdout
    );
}

/// The darwin plain-`close` surface (spec 22 class E): the JVM's
/// `FileDescriptor.close0` imports PLAIN `close`, while the libc crate
/// maps `libc::close` to `close$NOCANCEL` on x86_64 darwin — so the
/// shim's close tuple used to cover only the NOCANCEL spelling and the
/// JVM's close of a flagged memfs fd fell through to the kernel (EBADF
/// → LauncherHelper jar.error1 — the macos-15-intel leg). The probe
/// CHECKS close's return on the flagged fd; on x86_64 it red-flags any
/// regression of the plain-close tuple (arm64: `libc::close` IS plain
/// close, one tuple covers both spellings).
#[test]
fn macos_plain_close_on_a_memfs_fd() {
    if !cfg!(target_os = "macos") {
        eprintln!("skip: the $NOCANCEL variant family is mach-o (macos only)");
        return;
    }
    let Some(f) = fixtures() else { return };
    let path = format!("{MOUNT}/data/secret.txt");
    let r = run(f, "close-probe", &[path.as_str()], None);
    assert_eq!(r.rc, 0, "close-probe failed, stderr: {}", r.stderr);
    assert!(r.stdout.contains("close-probe:ok"), "stdout: {}", r.stdout);
}

/// The 2026-08-22 fork/exec deadlock regression pin (runtime 0.16.4: a
/// payload mounted at `/` spawning `git clone` wedged git's pre-exec
/// helper child). The fixture forks; the CHILD execve's a HOST binary
/// whose path is covered by the root mount — the exec materialization
/// probe reads the in-image manifest through dwarfs-t's block cache,
/// whose worker pool did not survive the fork, and waits on a future no
/// dead thread completes. The fork-child guard passes every engine entry
/// in a fork child through to the real libc, so the exec completes.
///
/// Needs the DWARFS image: the zip backend has no worker pool, so a zip
/// mount cannot distinguish the guard from the bug. The fixture is its
/// own watchdog — a wedged child is SIGKILLed and reported as rc 124, so
/// a regression FAILS here instead of hanging the suite.
///
/// The exec'd grandchild (print-data, a non-SIP host binary) re-enters a
/// fresh, healthy shim through the inherited preload env and reads a host
/// file covered by the root mount — proving the spec 22 §3
/// child-namespace propagation survives the guard.
#[test]
fn fork_child_exec_under_root_mount_completes() {
    let Some(f) = fixtures() else { return };
    let mut cmd = Command::new(f.dir.join("bin").join("fork-exec"));
    cmd.arg(f.dir.join("bin").join("print-data"))
        .arg(f.work.join("hostfile.txt"))
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", format!("{}:/", f.dwarfs.display()))
        .env_remove("DYLD_PRINT_LIBRARIES")
        .env_remove("TEBAKO_JAIL");
    let out = cmd.output().unwrap();
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&out.status);
    #[cfg(not(unix))]
    let signal = None;
    assert_eq!(
        out.status.code(),
        Some(0),
        "fork-exec failed (signal: {signal:?}, rc 124 = wedged child — the \
         fork/exec deadlock regression), stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        out.stdout, b"HOST-FILE\n",
        "the grandchild's host read under the root mount"
    );
}

// ---------------------------------------------------------------------
// tebako#448: the macOS insertion guard (arm64e / Rosetta exec targets)
// ---------------------------------------------------------------------

/// Compile the insert-probe for one `-arch`. None — with a stderr note,
/// the suite's documented skip idiom — when the toolchain cannot build
/// that arch at all (the absent-prerequisite arm; the standard-fixture
/// "a cc that fails is a hard failure" rule does not cover optional arch
/// slices).
#[cfg(target_os = "macos")]
fn compile_arch_probe(f: &Fixtures, arch: &str) -> Option<PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/insert-probe.c");
    let out = f.dir.join("bin").join(format!("insert-probe-{arch}"));
    let o = Command::new("cc")
        .arg("-O2")
        .arg("-arch")
        .arg(arch)
        .arg("-o")
        .arg(&out)
        .arg(&src)
        .output()
        .unwrap();
    if !o.status.success() {
        eprintln!(
            "skip: cc -arch {arch} unsupported: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        return None;
    }
    Some(out)
}

/// Preflight: does the probe RUN here unarmed (no insertion, no mounts)?
/// Distinguishes a usable arch leg from a host that cannot run the arch
/// at all — Rosetta absent, or a kernel refusing third-party arm64e
/// (macOS 14.1: "not running binary built against preview arm64e ABI").
#[cfg(target_os = "macos")]
fn arch_probe_runs(probe: &Path) -> bool {
    Command::new(probe)
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove("TEBAKO_TFS_MOUNTS")
        .output()
        .map(|o| o.status.code() == Some(42))
        .unwrap_or(false)
}

/// Launch the inserted spawn-helper so its INTERPOSED exec/spawn surface
/// execs the probe with the inherited env — the guard's decision point.
/// `path_override` arms the spawnp bare-name leg's PATH search; `debug`
/// sets TEBAKO_DEBUG_TFS so the guard's own strip note lands on stderr.
#[cfg(target_os = "macos")]
fn run_guarded(f: &Fixtures, mode: &str, target: &str, path: Option<&Path>, debug: bool) -> Run {
    let mut cmd = Command::new(f.dir.join("bin").join("spawn-helper"));
    cmd.arg(mode)
        .arg(target)
        .arg("x")
        .env(preload_var(), &f.shim)
        .env("TEBAKO_TFS_MOUNTS", "")
        .env_remove("TEBAKO_JAIL")
        .env_remove("DYLD_PRINT_LIBRARIES");
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    if debug {
        cmd.env("TEBAKO_DEBUG_TFS", "1");
    }
    let out = cmd.output().unwrap();
    let signal = std::os::unix::process::ExitStatusExt::signal(&out.status);
    Run {
        rc: out.status.code().unwrap_or(-1),
        signal,
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// tebako#448 — the macOS insertion guard. A virtualized parent (the
/// shim loaded via DYLD_INSERT_LIBRARIES) execs a child whose Mach-O the
/// arm64-only interpose dylib cannot load: before the fix the forwarded
/// insertion made dyld TERMINATE the child ("mach-o file, but is an
/// incompatible architecture (have 'arm64', need 'arm64e')" — the 0.16.7
/// native_ext leg on macos-14). The interposed execve/posix_spawn/
/// posix_spawnp must strip DYLD_INSERT_LIBRARIES for exactly those
/// targets and keep it otherwise.
///
/// Three legs: an arm64 CONTROL (insertion kept — the child reports the
/// var and exits 42), an x86_64-only probe under Rosetta (no arm64 slice
/// → stripped; runs all three exec surfaces incl. spawnp's bare-name PATH
/// resolution), and an arm64e probe — the issue's exact shape — which
/// skips loudly on hosts whose kernel refuses third-party arm64e
/// binaries (macOS 14.1's "preview arm64e ABI" refusal; the macos-14 CI
/// runners run them, and that is where the bug bit).
#[cfg(target_os = "macos")]
#[test]
fn exec_strips_insertion_when_the_target_cannot_load_the_dylib() {
    let Some(f) = fixtures() else { return };

    // The arm64 control: the guard must NOT strip what dyld can load —
    // the inserted child reports the inherited var and exits 42. (cc must
    // build the host's own arch — a hard failure otherwise, per the
    // suite's skip policy.)
    let arm64 = compile_arch_probe(f, "arm64").expect("the host arch compiles");
    let r = run_guarded(f, "execve", arm64.to_str().unwrap(), None, false);
    assert_eq!(
        r.rc, 42,
        "arm64 control (signal: {:?}), stderr: {}",
        r.signal, r.stderr
    );
    assert_eq!(r.stdout, "INSERT:set\n", "the control keeps insertion");

    // The x86_64-only leg: no arm64 slice → the arm64 dylib cannot load
    // (dyld's own words on this host, pre-fix: "incompatible architecture
    // (have 'arm64', need 'x86_64')" + SIGABRT). Stripped, the child runs.
    if let Some(x64) = compile_arch_probe(f, "x86_64") {
        if arch_probe_runs(&x64) {
            for mode in ["execve", "posix_spawn"] {
                let r = run_guarded(f, mode, x64.to_str().unwrap(), None, true);
                assert_eq!(
                    r.rc, 42,
                    "x86_64 {mode} (signal: {:?} — pre-fix dyld aborts here), stderr: {}",
                    r.signal, r.stderr
                );
                assert_eq!(r.stdout, "INSERT:unset\n", "x86_64 {mode} kept the var");
                // The guard's own note on the crate's debug channel —
                // proof the strip fired, not that dyld relented.
                assert!(
                    r.stderr.contains("[preload] strip DYLD_INSERT_LIBRARIES"),
                    "x86_64 {mode}: the guard note, stderr: {}",
                    r.stderr
                );
            }
            // spawnp of a BARE name: the guard resolves it through the
            // caller's PATH, then strips by the resolved file's slices.
            let r = run_guarded(
                f,
                "posix_spawnp",
                "insert-probe-x86_64",
                Some(&f.dir.join("bin")),
                false,
            );
            assert_eq!(
                r.rc, 42,
                "x86_64 posix_spawnp (signal: {:?}), stderr: {}",
                r.signal, r.stderr
            );
            assert_eq!(r.stdout, "INSERT:unset\n", "spawnp bare-name strip");
        } else {
            eprintln!("skip: the x86_64 probe does not run here (Rosetta absent)");
        }
    }

    // The arm64e leg — the issue's exact dyld kill.
    if let Some(arm64e) = compile_arch_probe(f, "arm64e") {
        if arch_probe_runs(&arm64e) {
            for mode in ["execve", "posix_spawn"] {
                let r = run_guarded(f, mode, arm64e.to_str().unwrap(), None, false);
                assert_eq!(
                    r.rc, 42,
                    "arm64e {mode} (signal: {:?} — the tebako#448 kill), stderr: {}",
                    r.signal, r.stderr
                );
                assert_eq!(r.stdout, "INSERT:unset\n", "arm64e {mode} kept the var");
            }
        } else {
            eprintln!(
                "skip: this host's kernel refuses third-party arm64e binaries \
                 (preview ABI) — the arm64e leg proves out on macos-14 CI"
            );
        }
    }
}
