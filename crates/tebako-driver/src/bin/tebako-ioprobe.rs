//! tebako#1125's in-process file-IO probe (macOS): the executable half of
//! the shim re-exec's regression net.
//!
//! A runtime exe's shape is: the driver's boot mounts the env image, the
//! injection export materializes the declared preload shim, and the
//! boot's TAIL re-execs the interpreter once with the shim inserted
//! (spec 22 §3.6) so the file-IO family's interpose tuples cover the
//! process. This bin is that shape at miniature scale: it boots the
//! driver against a fixture env image that declares
//! `preload_shim: lib/tebako/libtfs_preload.dylib` and carries the REAL
//! shim cdylib, then performs a RAW `open(2)`/`read(2)` of an in-VFS
//! marker file — exactly an unpatched native extension's call shape —
//! and prints the verdict.
//!
//! Verdict contract (stdout, exactly one line; the exit code matches):
//!
//! - `IOPROBE-OK <marker>` (0) — the shim is live in this process: the
//!   raw open routed through the interposed family and answered the
//!   marker bytes, and the driver's own engine serves the same marker
//!   (the tebako#964 depth seam keeps the engine's host IO clean under
//!   the interposition). The signed CI leg requires exactly this.
//! - `IOPROBE-UNINSERTED <vfs path>` (3) — no shim in the dyld image
//!   list, and the raw open of the in-VFS path died ENOENT on the host.
//!   The expected shape on macOS >= 15 with an adhoc-signed probe: the
//!   tail's AMFI gate SKIPS the re-exec, loudly, and the boot proceeds
//!   without the family.
//! - `IOPROBE-FAIL <stage>: <detail>` (1) — every other shape: a boot
//!   error, an inserted process whose raw open/read failed or answered
//!   wrong bytes (the #1125 regression signal), or an uninserted
//!   process that somehow opened the in-VFS path (a host path shadowing
//!   `/__tfs__` — never a quiet pass).
//!
//! The fixture image path rides argv[1] (the re-execs replay this bin's
//! full argv, so the argument survives) or an already-set
//! `TEBAKO_RUNTIME_IMAGE`. The in-image layout is the fixture contract
//! shared with tests/ioprobe.rs and ci/macos-signed-dlopen.sh:
//!
//! ```text
//! lib/tebako/layout.yaml               # era-2 declaration + preload_shim key
//! lib/tebako/libtfs_preload.dylib      # the real preload shim cdylib
//! lib/shimprobe/marker.txt             # the marker bytes
//! ```
//!
//! Compiles on every platform (the workspace build builds all bins) and
//! runs only on macOS; elsewhere main refuses by name.

#![allow(unsafe_code)]

/// The in-VFS path of the marker file (the fixture contract above).
#[cfg(target_os = "macos")]
const MARKER_VFS_PATH: &str = "/__tfs__/lib/shimprobe/marker.txt";

/// The marker's exact bytes — the mirror of the literal in
/// tests/ioprobe.rs (the two move together or not at all; the CI leg
/// greps the full verdict line, so a drift is red).
#[cfg(target_os = "macos")]
const EXPECTED_MARKER: &[u8] = b"ioprobe-marker-0x1125";

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt as _;

    use libc::{c_char, c_int, c_void};

    use super::{EXPECTED_MARKER, MARKER_VFS_PATH};

    unsafe extern "C" {
        // The dyld image-list surface (declared directly: the libc crate
        // deprecated these re-exports — the dlprobe bin's own choice for
        // the same two stable libdyld calls).
        fn _dyld_image_count() -> u32;
        fn _dyld_get_image_name(image_index: u32) -> *const c_char;
        // The driver engine's C ABI (the class-L surface): linked from
        // the tfs crate through the driver, sealed dynamic by build.rs.
        fn tebako_fs_open(path: *const c_char, flags: c_int) -> c_int;
        fn tebako_fs_read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
        fn tebako_fs_close(fd: c_int) -> c_int;
    }

    /// The process's original argv as a C argv for the boot entry — the
    /// FULL argv, argv[0] and the fixture argument included: BOTH
    /// re-execs (the head self-insertion and the tail's shim re-exec)
    /// replay this bin with exactly this argv, so the final incarnation
    /// re-reads the image path from it.
    struct CArgv {
        _strings: Vec<CString>,
        ptrs: Vec<*mut c_char>,
    }

    impl CArgv {
        fn process() -> CArgv {
            let strings: Vec<CString> = std::env::args_os()
                .map(|a| {
                    CString::new(a.as_os_str().as_bytes())
                        .expect("the process argv is NUL-free by construction")
                })
                .collect();
            let mut ptrs: Vec<*mut c_char> =
                strings.iter().map(|c| c.as_ptr() as *mut c_char).collect();
            ptrs.push(std::ptr::null_mut());
            CArgv {
                _strings: strings,
                ptrs,
            }
        }
    }

    /// The image-list verdicts: the preload shim (the file-IO family)
    /// and the boot's micro interpose dylib (the class-L channel —
    /// ruby#388's boot-head insertion check keeps expecting it in the
    /// final process).
    fn loaded_images() -> (bool, bool) {
        let (mut shim, mut micro) = (false, false);
        let n = unsafe { _dyld_image_count() };
        for i in 0..n {
            let name = unsafe { _dyld_get_image_name(i) };
            if name.is_null() {
                continue;
            }
            let s = unsafe { CStr::from_ptr(name) }.to_string_lossy();
            if s.ends_with("/libtfs_preload.dylib") {
                shim = true;
            }
            if s.contains("tebako-interpose/") && s.ends_with(".dylib") {
                micro = true;
            }
        }
        (shim, micro)
    }

    /// Read `path` to end through a raw POSIX fd loop — the unpatched
    /// extension's exact call shape. `None` on any syscall failure.
    fn raw_read(path: &CString) -> Option<Vec<u8>> {
        let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY) };
        if fd < 0 {
            return None;
        }
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n < 0 {
                unsafe { libc::close(fd) };
                return None;
            }
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n as usize]);
        }
        unsafe { libc::close(fd) };
        Some(data)
    }

    /// The same read through the driver engine's own C ABI (class L):
    /// proves the interposed process keeps the engine's route clean
    /// (the tebako#964 depth seam — the shim passes the engine's host
    /// IO through, never recurses into it).
    fn engine_read(path: &CString) -> Option<Vec<u8>> {
        let fd = unsafe { tebako_fs_open(path.as_ptr(), libc::O_RDONLY) };
        if fd < 0 {
            return None;
        }
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { tebako_fs_read(fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n < 0 {
                unsafe { tebako_fs_close(fd) };
                return None;
            }
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n as usize]);
        }
        unsafe { tebako_fs_close(fd) };
        Some(data)
    }

    pub fn run() -> ! {
        // The fixture image: argv[1] wins; else the caller armed
        // TEBAKO_RUNTIME_IMAGE directly. The driver reads only the env
        // var, so the argv spelling is exported here (idempotently —
        // every re-exec'd incarnation redoes exactly this).
        if let Some(path) = std::env::args_os().nth(1) {
            std::env::set_var("TEBAKO_RUNTIME_IMAGE", path);
        } else if std::env::var_os("TEBAKO_RUNTIME_IMAGE").is_none() {
            eprintln!("usage: tebako-ioprobe <fixture env image>   (or set TEBAKO_RUNTIME_IMAGE)");
            std::process::exit(64);
        }

        // The boot: the head self-insertion (re-exec one), the mounts,
        // the injection export, and the tail's shim re-exec (re-exec
        // two). An inserting host never returns from the earlier
        // incarnations; the final one lands here with rc 0 on success.
        let mut cargv = CArgv::process();
        let mut argc: c_int = (cargv.ptrs.len() - 1) as c_int;
        let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();
        let root = CString::new("/__tfs__").unwrap();
        let rc =
            unsafe { tebako_driver::ffi::tebako_driver_boot(&mut argc, &mut argvp, root.as_ptr()) };
        if rc != 0 {
            // The boot already named the failure on stderr.
            println!("IOPROBE-FAIL boot: the driver answered rc={rc}");
            std::process::exit(1);
        }

        let (shim, micro) = loaded_images();
        eprintln!(
            "tebako-ioprobe: boot ok, preload shim {}, micro dylib {}",
            if shim { "inserted" } else { "not inserted" },
            if micro { "inserted" } else { "not inserted" },
        );

        // The unpatched-extension call shape, attempted in both shapes:
        // inserted, the shim's interposed open routes the in-VFS path;
        // uninserted, the raw open meets the bare spelling on the host
        // and dies ENOENT.
        let path = CString::new(MARKER_VFS_PATH).unwrap();
        let Some(raw) = raw_read(&path) else {
            let err = std::io::Error::last_os_error();
            if shim {
                println!("IOPROBE-FAIL open: the shim is inserted yet the raw open failed: {err}");
                std::process::exit(1);
            }
            println!("IOPROBE-UNINSERTED {MARKER_VFS_PATH}");
            std::process::exit(3);
        };
        if !shim {
            println!(
                "IOPROBE-FAIL open: uninserted yet '{MARKER_VFS_PATH}' opened — a host path shadows the VFS root; the probe's premise is gone"
            );
            std::process::exit(1);
        }
        if raw != EXPECTED_MARKER {
            println!(
                "IOPROBE-FAIL read: the raw read answered {} bytes, want the {} marker bytes",
                raw.len(),
                EXPECTED_MARKER.len()
            );
            std::process::exit(1);
        }
        match engine_read(&path) {
            Some(engine) if engine == EXPECTED_MARKER => {}
            Some(engine) => {
                println!(
                    "IOPROBE-FAIL engine: the driver's own route answered {} bytes, want {} — \
                     the interposed process broke the engine's read",
                    engine.len(),
                    EXPECTED_MARKER.len()
                );
                std::process::exit(1);
            }
            None => {
                println!(
                    "IOPROBE-FAIL engine: the driver's own route cannot serve the marker under \
                     the interposition"
                );
                std::process::exit(1);
            }
        }
        if !micro {
            // Named, never silent: the shim-first insert list must keep
            // the micro beside it (the class-L channel's shape).
            println!(
                "IOPROBE-FAIL micro: the shim is inserted but the boot's micro interpose dylib \
                 is not — the tail's insert list dropped it"
            );
            std::process::exit(1);
        }
        println!("IOPROBE-OK {}", String::from_utf8_lossy(EXPECTED_MARKER));
        std::process::exit(0);
    }
}

#[cfg(target_os = "macos")]
fn main() {
    imp::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "tebako-ioprobe is the macOS in-process file-IO probe (tebako#1125) — nothing to run on this platform"
    );
    std::process::exit(2);
}
