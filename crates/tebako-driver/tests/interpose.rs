//! Functional proof of the macOS boot-head self-insertion (spec 22 §2
//! "Phase 1 delivery"): a process entering the driver's boot with no
//! sentinel re-execs itself EXACTLY ONCE and lands with the embedded
//! interpose dylib inserted — the dyld mechanism that interposes
//! `dlopen`/`dlerror` process-wide (the empirical verdict: tuples in an
//! INSERTED dylib apply process-wide; the nm gate proves the tuples are
//! in the embedded bytes). The re-exec'd child then SCRUBS the dylib's
//! entry and the sentinel back out of the env (tebako#448): the dylib
//! stays loaded here, but nothing this process spawns inherits an
//! insertion bound to this exe's exports.
//!
//! Hermetic: no network, no fixtures — the probe re-execs this very test
//! binary with a fresh environment (sentinel and DYLD_INSERT_LIBRARIES
//! stripped), and the proof rides the re-exec'd child's stdout markers.
//!
//! The probe deliberately never calls `dlopen`: this test binary does
//! not export the `tebako_fs_*` symbols (that is the runtime exe's
//! exports.txt contract), so an interposed `dlopen` here would abort on
//! the unbound dynamic_lookup symbol instead of routing. The dyld image
//! scan is the insertion proof; the full route proof is the factory
//! dogfood's (a real runtime exe with the exports).

#![cfg(target_os = "macos")]
#![allow(unsafe_code)]

use std::ffi::{CStr, CString};
use std::process::Command;

use libc::{c_char, c_int};

/// The macOS-15 AMFI gate, mirrored for the probe and the parent both
/// (the driver's self_insert skips the re-exec there): an adhoc-signed
/// exe (the linker's signature carries no CMS blob) re-exec'd with the
/// insert dies at dyld on macOS 15+ — the unified log's "no CMS blob …
/// Unrecoverable CT signature issue". This test asserts the SKIP there
/// and the insertion elsewhere.
fn insert_killing_host() -> bool {
    macos_major().is_some_and(|v| v >= 15) && exe_is_adhoc()
}

/// The running OS's major version (`kern.osproductversion`).
fn macos_major() -> Option<u64> {
    let mut buf = [0u8; 32];
    let mut size = buf.len();
    // SAFETY: the named sysctl is a string; buf is sized for it, size
    // is in/out, and no new value is written (the null oldp form).
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

/// `codesign -dv` on this binary: the linker's adhoc signature reads
/// `Signature=adhoc` and carries no CMS blob. A test-only shell-out —
/// never in shipped code.
fn exe_is_adhoc() -> bool {
    let exe = std::env::current_exe().expect("the test binary's own path");
    Command::new("codesign")
        .arg("-dv")
        .arg(&exe)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stderr).contains("Signature=adhoc"))
        .unwrap_or(false)
}

/// Marks the spawned probe run (the suite's own in-process run of the
/// probe test returns immediately without it).
const PROBE_FLAG: &str = "TEBAKO_INTERPOSE_PROBE";
/// The recursion depth guard: a working insertion re-execs EXACTLY once
/// (depth 1 inserts, depth 2 proceeds); anything deeper means the
/// sentinel did not stick.
const DEPTH_VAR: &str = "TEBAKO_INTERPOSE_DEPTH";

// The micro dylib links NO tebako library: its tebako_fs_* references
// bind the host exe's exports at run time via -undefined dynamic_lookup
// (loader_interpose.c's contract). This harness binary is not a runtime
// exe, so the exports are stubbed HERE — macOS 15's dyld resolves the
// dynamic_lookup eagerly enough that the symbols' absence crashes the
// re-exec'd child (SIGSEGV before main; macos-14's lazy resolution let
// the probe pass because it never dlopens). The stubs are inert and
// never called: the probe's proof is the dylib's presence in the dyld
// image list, not a route.
#[no_mangle]
extern "C" fn tebako_path_is_embedded(_path: *const c_char) -> c_int {
    0
}

#[no_mangle]
extern "C" fn tebako_fs_mount_of(_path: *const c_char) -> *mut c_char {
    std::ptr::null_mut()
}

#[no_mangle]
extern "C" fn tebako_fs_dlmap2file(_path: *const c_char) -> *mut c_char {
    std::ptr::null_mut()
}

struct CArgv {
    _strings: Vec<CString>,
    ptrs: Vec<*mut c_char>,
}

impl CArgv {
    fn new(args: &[String]) -> CArgv {
        let strings: Vec<CString> = args
            .iter()
            .map(|s| CString::new(s.as_str()).unwrap())
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

// The dyld image-list surface (declared directly: the libc crate
// deprecated these re-exports in favor of mach2, which this test does
// not need as a dependency for two stable libdyld calls).
extern "C" {
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_name(image_index: u32) -> *const c_char;
}

/// The interpose dylib is present in this process's dyld image list
/// (dyld only loads an inserted library whose path it was handed at
/// launch — presence IS the insertion).
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

/// The probe: enters the driver's boot the way the runtime exe's main
/// does. The FIRST entry (no sentinel) never returns from here — the
/// boot execv's this binary with the identical argv. The re-exec'd child
/// (sentinel seen, then scrubbed with the micro's insert entry)
/// completes the boot and reports.
#[test]
fn self_insert_child_probe() {
    if std::env::var_os(PROBE_FLAG).is_none() {
        return;
    }
    let depth = std::env::var(DEPTH_VAR)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0)
        + 1;
    std::env::set_var(DEPTH_VAR, depth.to_string());
    if depth > 2 {
        eprintln!("PROBE the re-exec looped (depth {depth}) — the sentinel did not stick");
        std::process::exit(3);
    }
    // The boot entry with the process's own argv: the re-exec replays
    // this exact harness invocation (the libtest filter included), so
    // only the probe reruns. The phase markers ride stderr (uncaptured
    // under --nocapture): a silent death's phase is the last one seen.
    let args: Vec<String> = std::env::args().collect();
    let mut cargv = CArgv::new(&args);
    let mut argc: c_int = args.len() as c_int;
    let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();
    let root = CString::new("/__tfs__").unwrap();
    eprintln!("PROBE phase=pre-boot depth={depth}");
    let rc =
        unsafe { tebako_driver::ffi::tebako_driver_boot(&mut argc, &mut argvp, root.as_ptr()) };
    eprintln!("PROBE phase=post-boot rc={rc}");
    assert_eq!(rc, 0, "the plain boot succeeds");
    // Only the re-exec'd child reaches here — on an insert-killing host
    // (macOS 15+ over an adhoc exe) the driver's self_insert SKIPS the
    // re-exec instead, so the first and only entry is this one.
    let sentinel = std::env::var_os("TEBAKO_LOADER_INTERPOSED");
    let insert_var = std::env::var("DYLD_INSERT_LIBRARIES").unwrap_or_default();
    let loaded = interpose_dylib_loaded();
    println!("PROBE depth={depth}");
    println!("PROBE sentinel_present={}", sentinel.is_some());
    println!("PROBE insert_var={insert_var}");
    println!("PROBE dylib_loaded={loaded}");
    let killing = insert_killing_host();
    let want_depth = if killing { 1 } else { 2 };
    assert_eq!(
        depth, want_depth,
        "the re-exec fires exactly once — or never, on the AMFI-killing host"
    );
    // tebako#448: the micro dylib is bound to THIS exe's exports — the
    // re-exec'd child scrubs its entry (and the sentinel) back out of
    // the env before the boot proceeds, so nothing it spawns inherits an
    // insertion dyld may TERMINATE on (an arm64e target over an
    // arm64-only entry).
    assert!(
        sentinel.is_none(),
        "the re-exec'd child scrubs the sentinel before the boot proceeds"
    );
    assert!(
        !insert_var.contains("tebako-interpose/"),
        "the micro dylib's entry is scrubbed out of the child-facing env: {insert_var}"
    );
    if killing {
        assert!(
            !loaded,
            "the AMFI-killing host boots uninserted (the driver skips the re-exec)"
        );
    } else {
        assert!(
            loaded,
            "the interpose dylib is in the process's dyld image list"
        );
    }
    // The scrub IS the child-inheritance contract: a spawned grandchild
    // (a fat /bin/sh — exactly the tebako#448 dying shape on a
    // terminating dyld) launches clean under the scrubbed env.
    let out = Command::new("/bin/sh")
        .arg("-c")
        .arg("echo grandchild-ok")
        .output()
        .expect("spawn the grandchild probe");
    assert!(
        out.status.success(),
        "the grandchild survives the inherited env: {out:?}"
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("grandchild-ok"));
}

/// The newest macOS crash report's verdict lines (the metadata JSON is
/// the first line of an .ips; the report body — exception, termination,
/// the faulting thread's frames — is the second). Printed verbatim: the
/// report IS the diagnostic, parsing it into assertions would age worse.
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
    // Print the body's head raw (the second line of an .ips) — the
    // exception and termination blocks sit at its front.
    if let Some(body) = content.lines().nth(1) {
        eprintln!("{}", &body[..body.len().min(4000)]);
    }
}

/// The unified log's own record of the child's death (the runners write
/// no .ips for it): the kernel's kill line and dyld's entries for the
/// re-exec'd image, read back through `log show`. Printed verbatim.
fn dump_unified_log() {
    let exe = std::env::current_exe().expect("the test binary's own path");
    let stem = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "interpose".to_string());
    let out = std::process::Command::new("log")
        .args([
            "show",
            "--last",
            "3m",
            "--predicate",
            &format!("process == \"{stem}\" OR eventMessage CONTAINS[c] \"{stem}\""),
            "--style",
            "compact",
        ])
        .output();
    match out {
        Ok(o) => {
            eprintln!(
                "=== unified log (the last 3 min for {stem}) ===\n{}",
                String::from_utf8_lossy(&o.stdout)
            );
        }
        Err(e) => eprintln!("unified log unread: {e}"),
    }
}

/// The driver: spawns this test binary as a fresh process (sentinel and
/// DYLD_INSERT_LIBRARIES stripped — a clean launch), filtered to the
/// probe. The probe's first entry re-execs; the grandchild's markers are
/// the verdict.
#[test]
fn self_insert_reexecs_once_and_inserts_the_dylib() {
    if std::env::var_os(PROBE_FLAG).is_some() {
        return; // the spawned harness runs only the probe (the filter)
    }
    let exe = std::env::current_exe().expect("the test binary's own path");
    let out = Command::new(exe)
        .arg("self_insert_child_probe")
        .arg("--exact")
        .arg("--nocapture")
        .env(PROBE_FLAG, "1")
        // dyld's own error channel: silent on a clean run, and on a red
        // run its rejection words are the difference between "dyld
        // refused the insert" and "the kernel killed the exec'd image"
        // (the macos-15 arm64 leg's silent death class).
        .env("DYLD_PRINT_ERRORS", "1")
        .env_remove("TEBAKO_LOADER_INTERPOSED")
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove(DEPTH_VAR)
        .output()
        .expect("spawn the probe harness");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let status = out.status;
    // A silent death is a dyld rejection (it speaks on
    // DYLD_PRINT_ERRORS) or a hard crash (it does not): the crash report
    // in ~/Library/Logs/DiagnosticReports carries the exception and the
    // faulting frame — and where no .ips is written (the CI runners
    // prove it), the unified log does (the kernel's kill line and dyld's
    // own entries). Both are read on the failure path.
    if !status.success() {
        dump_newest_crash_report();
        dump_unified_log();
    }
    assert!(
        out.status.success(),
        "the probe failed with {status:?}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    let markers: Vec<&str> = stdout.lines().filter(|l| l.starts_with("PROBE ")).collect();
    assert_eq!(
        markers.len(),
        4,
        "the probe's markers appear exactly once — one completed boot:\n{stdout}"
    );
    let killing = insert_killing_host();
    let (want_depth, want_loaded) = if killing { (1, false) } else { (2, true) };
    assert!(
        stdout.contains(&format!("PROBE depth={want_depth}")),
        "{stdout}"
    );
    assert!(stdout.contains("PROBE sentinel_present=false"), "{stdout}");
    assert!(
        stdout.contains(&format!("PROBE dylib_loaded={want_loaded}")),
        "{stdout}"
    );
    if killing {
        assert!(
            stderr.contains("loader interposition skipped"),
            "the AMFI-killing host says the skip loudly:\n{stderr}"
        );
    }
}
