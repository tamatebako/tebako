//! tebako#989's class-L dlopen probe (macOS): the executable half of the
//! AMFI gate's regression net.
//!
//! A runtime exe's shape is: the driver's boot self-inserts the embedded
//! interpose dylib (DYLD_INSERT_LIBRARIES + re-exec, spec 22 §2), the exe
//! exports the `tebako_fs_*` surface (build.rs seals this bin with the
//! tfs crate's exports.txt), and a later `dlopen` of a VFS-resident
//! library routes through the interposed dyld tuple — covered-path check
//! (`tebako_path_is_embedded`), materialization (`tebako_fs_dlmap2file`),
//! real `dlopen` of the exec-cache copy. This bin is that shape at
//! miniature scale: it boots the driver against a fixture env image
//! carrying a fixture dylib, then dlopens the dylib's in-VFS path,
//! resolves its one exported function, calls it, and prints the verdict.
//!
//! Verdict contract (stdout, exactly one line; the exit code matches):
//!
//! - `DLPROBE-OK 0x<answer>` (0) — the insertion fired, the route
//!   answered, and the fixture function returned the expected constant.
//!   The CI leg (macos-15, Developer ID signed) requires exactly this.
//! - `DLPROBE-UNINSERTED <vfs path>: <dlerror>` (3) — no interpose dylib
//!   in the dyld image list, and the raw `dlopen` of the in-VFS path
//!   failed on the host. The expected shape on macOS >= 15 with an
//!   adhoc-signed probe: the driver's AMFI gate SKIPS the insertion (the
//!   re-exec would die at dyld), loudly, and the boot proceeds
//!   uninserted.
//! - `DLPROBE-FAIL <stage>: <detail>` (1) — every other shape: a boot
//!   error, an inserted process whose dlopen/dlsym/call failed (the
//!   #988-class regression signal), or an uninserted process that
//!   somehow loaded the in-VFS path (a host path shadowing `/__tfs__` —
//!   never a quiet pass).
//!
//! The fixture image path rides argv[1] (the re-exec replays this bin's
//! full argv, so the argument survives the insertion) or an already-set
//! `TEBAKO_RUNTIME_IMAGE`. The in-image layout is the fixture contract
//! shared with tests/interpose_dlopen.rs and ci/macos-signed-dlopen.sh:
//!
//! ```text
//! lib/tebako/layout.yaml                  # the spec-18 era-2 declaration
//! lib/dlprobe/libdlprobe_fixture.dylib    # the fixture (dlprobe_answer)
//! ```
//!
//! Compiles on every platform (the workspace build builds all bins) and
//! runs only on macOS; elsewhere main refuses by name.

#![allow(unsafe_code)]

/// The in-VFS path of the fixture dylib (the fixture contract above).
#[cfg(target_os = "macos")]
const FIXTURE_VFS_PATH: &str = "/__tfs__/lib/dlprobe/libdlprobe_fixture.dylib";

/// The answer `dlprobe_answer` must return — the mirror of the literal in
/// tests/fixtures/dlprobe_fixture.rs (the two move together or not at
/// all; the CI leg greps the full verdict line, so a drift is red).
#[cfg(target_os = "macos")]
const EXPECTED_ANSWER: u64 = 0xD10C_CAFE_0989;

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt as _;

    use libc::{c_char, c_int};

    use super::{EXPECTED_ANSWER, FIXTURE_VFS_PATH};

    // The dyld image-list surface (declared directly: the libc crate
    // deprecated these re-exports in favor of mach2 — the same choice
    // tests/interpose.rs makes for the same two stable libdyld calls).
    extern "C" {
        fn _dyld_image_count() -> u32;
        fn _dyld_get_image_name(image_index: u32) -> *const c_char;
    }

    /// The process's original argv as a C argv for the boot entry — the
    /// FULL argv, argv[0] and the fixture argument included: the boot's
    /// self-insertion execv's this bin with exactly this argv, so the
    /// re-exec'd child re-reads the image path from it.
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

    /// The insertion verdict: the interpose dylib's content-keyed path in
    /// this process's dyld image list (dyld only loads an inserted
    /// library whose path it was handed at launch — presence IS the
    /// insertion; tests/interpose.rs's proof shape).
    fn interpose_dylib_loaded() -> bool {
        let n = unsafe { _dyld_image_count() };
        for i in 0..n {
            let name = unsafe { _dyld_get_image_name(i) };
            if name.is_null() {
                continue;
            }
            let s = unsafe { CStr::from_ptr(name) }.to_string_lossy();
            if s.contains("tebako-interpose/") && s.ends_with(".dylib") {
                return true;
            }
        }
        false
    }

    /// The current dlerror text (never a null deref: dlerror may answer
    /// NULL when nothing failed).
    fn dlerror_text() -> String {
        let p = unsafe { libc::dlerror() };
        if p.is_null() {
            "(no dlerror text)".to_string()
        } else {
            unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    }

    pub fn run() -> ! {
        // The fixture image: argv[1] wins; else the caller armed
        // TEBAKO_RUNTIME_IMAGE directly. The driver reads only the env
        // var, so the argv spelling is exported here (idempotently — the
        // re-exec'd child redoes exactly this).
        if let Some(path) = std::env::args_os().nth(1) {
            std::env::set_var("TEBAKO_RUNTIME_IMAGE", path);
        } else if std::env::var_os("TEBAKO_RUNTIME_IMAGE").is_none() {
            eprintln!("usage: tebako-dlprobe <fixture env image>   (or set TEBAKO_RUNTIME_IMAGE)");
            std::process::exit(64);
        }

        // The boot head: self-insertion (re-exec once, unless the AMFI
        // gate skips it) → env-image mount → layout check → rewritten
        // argv. The first entry never returns here on an inserting host;
        // the re-exec'd child (or the skipped host's only entry) lands
        // with rc 0 on success.
        let mut cargv = CArgv::process();
        let mut argc: c_int = (cargv.ptrs.len() - 1) as c_int;
        let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();
        let root = CString::new("/__tfs__").unwrap();
        let rc =
            unsafe { tebako_driver::ffi::tebako_driver_boot(&mut argc, &mut argvp, root.as_ptr()) };
        if rc != 0 {
            // The boot already named the failure on stderr.
            println!("DLPROBE-FAIL boot: the driver answered rc={rc}");
            std::process::exit(1);
        }

        let inserted = interpose_dylib_loaded();
        eprintln!(
            "tebako-dlprobe: boot ok, interpose dylib {}",
            if inserted { "inserted" } else { "not inserted" }
        );

        // The class-L route, attempted in both shapes: inserted, the
        // interposed dlopen materializes and loads; uninserted, the raw
        // dlopen meets the bare in-VFS path on the host and fails.
        let path = CString::new(FIXTURE_VFS_PATH).unwrap();
        let handle = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW) };
        if handle.is_null() {
            let err = dlerror_text();
            if inserted {
                println!("DLPROBE-FAIL dlopen: inserted but the route failed: {err}");
                std::process::exit(1);
            }
            println!("DLPROBE-UNINSERTED {FIXTURE_VFS_PATH}: {err}");
            std::process::exit(3);
        }
        if !inserted {
            println!(
                "DLPROBE-FAIL dlopen: uninserted yet '{FIXTURE_VFS_PATH}' loaded — a host path shadows the VFS root; the probe's premise is gone"
            );
            std::process::exit(1);
        }

        let name = CString::new("dlprobe_answer").unwrap();
        let sym = unsafe { libc::dlsym(handle, name.as_ptr()) };
        if sym.is_null() {
            println!("DLPROBE-FAIL dlsym: {}", dlerror_text());
            std::process::exit(1);
        }
        // SAFETY: the fixture's contract names dlprobe_answer an
        // `extern "C" fn() -> u64`; dlsym's void* is that function's
        // address (the dlsym idiom).
        let answer: unsafe extern "C" fn() -> u64 = unsafe { std::mem::transmute(sym) };
        let got = unsafe { answer() };
        if got != EXPECTED_ANSWER {
            println!(
                "DLPROBE-FAIL answer: dlprobe_answer() returned 0x{got:x}, want 0x{EXPECTED_ANSWER:x}"
            );
            std::process::exit(1);
        }
        println!("DLPROBE-OK 0x{got:x}");
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
        "tebako-dlprobe is the macOS class-L probe (tebako#989) — nothing to run on this platform"
    );
    std::process::exit(2);
}
