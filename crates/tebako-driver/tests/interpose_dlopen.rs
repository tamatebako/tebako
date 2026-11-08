//! tebako#989's class-L dlopen probe, driven end to end UNSIGNED: the
//! probe bin (src/bin/tebako-dlprobe.rs — the runtime exe's shape in
//! miniature: the driver's self-insertion boot + the sealed `tebako_fs_*`
//! export surface) boots against a fixture env image carrying a fixture
//! dylib, then dlopens the dylib's in-VFS path through the interposed
//! route and calls its one function.
//!
//! The honest assertion splits on the macOS-15 AMFI gate exactly the way
//! tests/interpose.rs's `insert_killing_host()` does: an adhoc-signed
//! probe on macOS >= 15 takes the driver's loud insertion SKIP (the
//! re-exec would die at dyld), so the verdict is the named uninserted
//! behavior (`DLPROBE-UNINSERTED`, exit 3); everywhere else (macOS < 15,
//! or a really-signed probe — the CI leg's shape) the insertion fires and
//! the route+materialize path answers (`DLPROBE-OK`, exit 0). The SIGNED
//! insertion proof is ci.yml's macos-15 leg's; this test is the local
//! truth and the disarmed gate's regression floor.
//!
//! Hermetic: the fixture dylib is compiled from the checked-in one-file
//! source (tests/fixtures/dlprobe_fixture.rs) with the rustc on PATH,
//! the env image is written in-process with limnifs-write (the driver
//! tests' fixture pattern), and the probe runs as a child process — no
//! network, no shared state.

#![cfg(target_os = "macos")]
#![allow(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The fixture contract (shared with the probe bin and the CI leg's
/// staging): the spec-18 era-2 layout declaration plus the fixture dylib.
const FIXTURE_VFS_PATH: &str = "/__tfs__/lib/dlprobe/libdlprobe_fixture.dylib";
/// The verdict's answer half — the literal dlprobe_fixture.rs returns.
const EXPECTED_VERDICT: &str = "DLPROBE-OK 0xd10ccafe0989";
/// A well-formed layout declaration for the runtime root (spec 18 C3 —
/// the boot.rs fixture's constant).
const GOOD_LAYOUT: &str = "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"3.4\"\n";

/// The probe binary this package builds (cargo hands the path to the
/// integration tests of the bin's own package).
const PROBE: &str = env!("CARGO_BIN_EXE_tebako-dlprobe");

/// The macOS-15 AMFI gate, mirrored for the probe: an adhoc-signed exe
/// (the linker's signature carries no CMS blob) re-exec'd with the
/// insert armed dies at dyld on macOS 15+ — so the driver skips the
/// insertion there, and this test asserts the skip. A signed probe (the
/// CI leg) or a pre-15 host inserts.
fn insert_killing_host() -> bool {
    macos_major().is_some_and(|v| v >= 15) && exe_is_adhoc(Path::new(PROBE))
}

/// The running OS's major version (`kern.osproductversion`).
fn macos_major() -> Option<u64> {
    let mut buf = [0u8; 32];
    let mut size = buf.len();
    // SAFETY: the named sysctl is a string; buf is sized for it, size is
    // in/out, and no new value is written (the null oldp form).
    let rc = unsafe {
        libc::sysctlbyname(
            c"kern.osproductversion".as_ptr(),
            buf.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(size);
    let s = std::str::from_utf8(&buf[..end]).ok()?;
    s.split('.').next()?.parse().ok()
}

/// `codesign -dv` on the probe: the linker's adhoc signature reads
/// `Signature=adhoc` and carries no CMS blob. A test-only shell-out —
/// never in shipped code.
fn exe_is_adhoc(exe: &Path) -> bool {
    Command::new("codesign")
        .arg("-dv")
        .arg(exe)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stderr).contains("Signature=adhoc"))
        .unwrap_or(false)
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tebako-driver-dlprobe-{tag}-{}-{uniq}",
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

/// The fixture dylib: the checked-in one-file source compiled in-leg
/// (never a workspace member — the fixture is this test's and the CI
/// leg's, not the workspace build's).
fn build_fixture_dylib(dir: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dlprobe_fixture.rs");
    assert!(
        source.is_file(),
        "the fixture source is checked in at {}",
        source.display()
    );
    let out = dir.join("libdlprobe_fixture.dylib");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let status = Command::new(rustc)
        .args(["--edition", "2021", "--crate-type", "cdylib"])
        .arg(&source)
        .arg("-o")
        .arg(&out)
        .status()
        .expect("spawn rustc for the fixture dylib");
    assert!(status.success(), "the fixture dylib failed to compile");
    out
}

/// The fixture env image: the era-2 layout declaration + the fixture
/// dylib, written in-process as a limnifs image (the driver tests'
/// pattern — never a limni binary).
fn write_env_image(dir: &Path, dylib: &Path) -> PathBuf {
    let root = dir.join("image-root");
    std::fs::create_dir_all(root.join("lib/tebako")).unwrap();
    std::fs::create_dir_all(root.join("lib/dlprobe")).unwrap();
    std::fs::write(root.join("lib/tebako/layout.yaml"), GOOD_LAYOUT).unwrap();
    std::fs::copy(dylib, root.join("lib/dlprobe/libdlprobe_fixture.dylib")).unwrap();
    let mut config = limnifs_write::WriteConfig::default_v0_1();
    config.dictionaries.enabled = false;
    let artifact = limnifs_write::write_directory_with_config(&root, &config).expect("write");
    assert!(
        artifact.metadata_sidecar.is_none(),
        "the fixture keeps its metadata inline"
    );
    let mut image = artifact.bytes;
    for slab in &artifact.slabs {
        image.extend_from_slice(&slab.bytes);
    }
    let path = dir.join("env.tfs");
    std::fs::write(&path, &image).unwrap();
    path
}

/// The newest macOS crash report's verdict lines (the metadata JSON is
/// the first line of an .ips; the report body — exception, termination,
/// the faulting thread's frames — is the second). Printed verbatim: the
/// report IS the diagnostic, parsing it into assertions would age worse.
/// (tests/interpose.rs's helper, mirrored for the probe's failure class:
/// an AMFI-killed re-exec leaves exactly this trace.)
fn dump_newest_crash_report() {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let dir = std::path::Path::new(&home).join("Library/Logs/DiagnosticReports");
    let Ok(read) = std::fs::read_dir(&dir) else {
        return;
    };
    let newest = read
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "ips"))
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some((m.modified().ok()?, e.path()))
        })
        .max_by_key(|(m, _)| *m);
    let Some((_, path)) = newest else { return };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    eprintln!("=== newest crash report: {} ===", path.display());
    if let Some(body) = content.lines().nth(1) {
        eprintln!("{}", &body[..body.len().min(4000)]);
    }
}

/// The unified log's own record of the probe's death (the runners write
/// no .ips for it): the kernel's kill line and dyld's entries for the
/// re-exec'd image, read back through `log show`. Printed verbatim.
fn dump_unified_log() {
    let out = Command::new("log")
        .args([
            "show",
            "--last",
            "3m",
            "--predicate",
            "process == \"tebako-dlprobe\" OR eventMessage CONTAINS[c] \"tebako-dlprobe\"",
            "--style",
            "compact",
        ])
        .output();
    match out {
        Ok(o) => {
            eprintln!(
                "=== unified log (the last 3 min for tebako-dlprobe) ===\n{}",
                String::from_utf8_lossy(&o.stdout)
            );
        }
        Err(e) => eprintln!("unified log unread: {e}"),
    }
}

/// The build.rs seal's proof: the probe exe exports the `tebako_fs_*`
/// surface the interpose dylib binds (the route's three symbols named —
/// the whole list is crates/tfs/exports.txt). Without the seal the
/// inserted dylib's dynamic_lookup references find nothing and the
/// signed leg's premise is void.
#[test]
fn the_probe_exports_the_tfs_route_surface() {
    let out = Command::new("nm")
        .arg("-gU")
        .arg(PROBE)
        .output()
        .expect("nm the probe binary");
    assert!(
        out.status.success(),
        "nm failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let symbols = String::from_utf8_lossy(&out.stdout);
    for want in [
        "_tebako_path_is_embedded",
        "_tebako_fs_mount_of",
        "_tebako_fs_dlmap2file",
    ] {
        assert!(
            symbols.contains(&format!(" T {want}\n")),
            "the probe exports {want}:\n{symbols}"
        );
    }
}

/// The end-to-end verdict: build the fixture image, run the probe
/// against it, assert the gate-shaped verdict. On the AMFI-killing shape
/// (macOS >= 15, adhoc probe) the driver's skip is the truth; elsewhere
/// the route answers.
#[test]
fn the_unsigned_probe_meets_the_amfi_gate() {
    let tmp = TempDir::new("e2e");
    let dylib = build_fixture_dylib(&tmp.0);
    let image = write_env_image(&tmp.0, &dylib);
    let out = Command::new(PROBE)
        .arg(&image)
        // dyld's own error channel: on a red run its rejection words
        // distinguish "dyld refused the insert" from "the kernel killed
        // the exec'd image" (the macos-15 silent death class).
        .env("DYLD_PRINT_ERRORS", "1")
        .env_remove("TEBAKO_LOADER_INTERPOSED")
        .env_remove("DYLD_INSERT_LIBRARIES")
        .output()
        .expect("spawn the probe");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let killing = insert_killing_host();
    let want = if killing {
        Some(3) // DLPROBE-UNINSERTED — the driver's loud skip
    } else {
        Some(0) // DLPROBE-OK — the route answered
    };
    if out.status.code() != want {
        dump_newest_crash_report();
        dump_unified_log();
    }
    let verdicts: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("DLPROBE-"))
        .collect();
    assert_eq!(
        verdicts.len(),
        1,
        "exactly one verdict line:\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    if killing {
        assert_eq!(out.status.code(), Some(3), "{stdout}\n{stderr}");
        assert!(
            verdicts[0].starts_with("DLPROBE-UNINSERTED "),
            "the skipped insertion's named verdict: {stdout}"
        );
        assert!(
            verdicts[0].contains(FIXTURE_VFS_PATH),
            "the verdict names the in-VFS path that failed: {stdout}"
        );
        assert!(
            stderr.contains("loader interposition skipped"),
            "the AMFI-killing host says the skip loudly:\n{stderr}"
        );
    } else {
        assert_eq!(out.status.code(), Some(0), "{stdout}\n{stderr}");
        assert_eq!(
            verdicts[0], EXPECTED_VERDICT,
            "the inserted route answers the fixture's constant"
        );
    }
}
