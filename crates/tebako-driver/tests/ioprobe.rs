//! tebako#1125's in-process file-IO probe, driven end to end UNSIGNED:
//! the probe bin (src/bin/tebako-ioprobe.rs — the runtime exe's shape in
//! miniature: the driver's boot + the tail's shim re-exec + the sealed
//! `tebako_fs_*` export surface) boots against a fixture env image that
//! declares `preload_shim` and carries the REAL preload shim cdylib,
//! then performs a raw `open(2)`/`read(2)` of an in-VFS marker — an
//! unpatched native extension's exact call shape.
//!
//! The honest assertion splits on the macOS-15 AMFI gate exactly the way
//! tests/interpose_dlopen.rs does: an adhoc-signed probe on macOS >= 15
//! takes the tail's loud SKIP (an insert re-exec would die at dyld), so
//! the verdict is the named uninserted behavior (`IOPROBE-UNINSERTED`,
//! exit 3); everywhere else (macOS < 15, or a really-signed probe — the
//! CI leg's shape) the re-exec fires, the shim's interpose covers the
//! process, and the raw open answers the marker (`IOPROBE-OK`, exit 0).
//! The SIGNED insertion proof is ci.yml's macos-15 leg's; this test is
//! the local truth and the disarmed gate's regression floor.
//!
//! Hermetic: the env image is written in-process with limnifs-write (the
//! driver tests' fixture pattern), the shim is the workspace's own
//! `cargo build -p libtfs-preload` product (the ci.yml `test` job builds
//! it before the workspace sweep), and the probe runs as a child
//! process — no network, no shared state.

#![cfg(target_os = "macos")]
#![allow(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The fixture contract (shared with the probe bin and the CI leg's
/// staging): the era-2 layout declaration WITH the preload_shim key,
/// the real shim, and the marker file.
const MARKER_VFS_PATH: &str = "/__tfs__/lib/shimprobe/marker.txt";
/// The marker's exact bytes — the mirror of the probe bin's constant.
const MARKER: &[u8] = b"ioprobe-marker-0x1125";
/// The verdict's OK half.
const EXPECTED_VERDICT: &str = "IOPROBE-OK ioprobe-marker-0x1125";
/// The era-2 layout declaration (spec 18 C3) with the schema_minor-2
/// `preload_shim` key — the key's presence is the whole trigger.
const LAYOUT: &str = "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"3.4\"\npreload_shim: lib/tebako/libtfs_preload.dylib\n";

/// The probe binary this package builds (cargo hands the path to the
/// integration tests of the bin's own package).
const PROBE: &str = env!("CARGO_BIN_EXE_tebako-ioprobe");

/// The preload shim cdylib the fixture image carries: the
/// `TEBAKO_SHIM_PROBE_DYLIB` override first (a rehearsal points it at a
/// staged artifact), then the workspace build's product (the
/// libtfs-preload e2e's discovery pattern). Absent ⇒ the test skips
/// loudly — never a quiet pass on a missing premise.
fn shim_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("TEBAKO_SHIM_PROBE_DYLIB") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        for cand in [
            target.join(profile).join("libtfs_preload.dylib"),
            target
                .join(profile)
                .join("deps")
                .join("libtfs_preload.dylib"),
        ] {
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

/// The macOS-15 AMFI gate, mirrored for the probe: an adhoc-signed exe
/// (the linker's signature carries no CMS blob) re-exec'd with the
/// insert armed dies at dyld on macOS 15+ — so the driver skips the
/// re-exec there, and this test asserts the skip. A signed probe (the
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
            "tebako-driver-ioprobe-{tag}-{}-{uniq}",
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

/// The fixture env image: the era-2 layout declaration with the
/// preload_shim key, the REAL shim cdylib, and the marker file — written
/// in-process as a limnifs image (the driver tests' pattern — never a
/// limni binary).
fn write_env_image(dir: &Path, shim: &Path) -> PathBuf {
    let root = dir.join("image-root");
    std::fs::create_dir_all(root.join("lib/tebako")).unwrap();
    std::fs::create_dir_all(root.join("lib/shimprobe")).unwrap();
    std::fs::write(root.join("lib/tebako/layout.yaml"), LAYOUT).unwrap();
    std::fs::copy(shim, root.join("lib/tebako/libtfs_preload.dylib")).unwrap();
    std::fs::write(root.join("lib/shimprobe/marker.txt"), MARKER).unwrap();
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

/// The build.rs seal's proof: the probe exe exports the `tebako_fs_*`
/// surface the micro interpose dylib binds (the route's three symbols
/// named — the whole list is crates/tfs/exports.txt). Without the seal
/// the inserted dylib's dynamic_lookup references find nothing and the
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

/// The end-to-end verdict: build the shim-carrying fixture image, run
/// the probe against it, assert the gate-shaped verdict. On the
/// AMFI-killing shape (macOS >= 15, adhoc probe) the tail's skip is the
/// truth; elsewhere the re-exec arms the family and the raw open
/// answers — tebako#1125's macOS half, proven in-process.
#[test]
fn the_unsigned_probe_meets_the_shim_reexec() {
    let Some(shim) = shim_path() else {
        eprintln!(
            "skip: libtfs_preload.dylib not found in the target dir \
             (build it with `cargo build -p libtfs-preload`)"
        );
        return;
    };
    let tmp = TempDir::new("e2e");
    let image = write_env_image(&tmp.0, &shim);
    let out = Command::new(PROBE)
        .arg(&image)
        // dyld's own error channel: on a red run its rejection words
        // distinguish "dyld refused the insert" from "the kernel killed
        // the exec'd image" (the macos-15 silent death class).
        .env("DYLD_PRINT_ERRORS", "1")
        .env_remove("TEBAKO_LOADER_INTERPOSED")
        .env_remove("TEBAKO_DRIVER_SHIM_BOOTED")
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove("TEBAKO_TFS_MOUNTS")
        .env_remove("TEBAKO_PRELOAD_SHIM")
        .output()
        .expect("spawn the probe");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let killing = insert_killing_host();
    let verdicts: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("IOPROBE-"))
        .collect();
    assert_eq!(
        verdicts.len(),
        1,
        "exactly one verdict line:\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    if killing {
        assert_eq!(out.status.code(), Some(3), "{stdout}\n{stderr}");
        assert!(
            verdicts[0].starts_with("IOPROBE-UNINSERTED "),
            "the skipped re-exec's named verdict: {stdout}"
        );
        assert!(
            verdicts[0].contains(MARKER_VFS_PATH),
            "the verdict names the in-VFS path that failed: {stdout}"
        );
        assert!(
            stderr.contains("preload-shim insertion skipped"),
            "the AMFI-killing host says the tail's skip loudly:\n{stderr}"
        );
    } else {
        assert_eq!(out.status.code(), Some(0), "{stdout}\n{stderr}");
        assert_eq!(
            verdicts[0], EXPECTED_VERDICT,
            "the re-exec'd process answers the marker through the interposed family"
        );
        assert!(
            stderr.contains("preload shim inserted, micro dylib inserted"),
            "the final incarnation carries BOTH dylibs (the shim's family + the \
             class-L channel — ruby#388's boot-head check's shape):\n{stderr}"
        );
    }
}
