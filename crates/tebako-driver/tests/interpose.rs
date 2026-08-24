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

/// Marks the spawned probe run (the suite's own in-process run of the
/// probe test returns immediately without it).
const PROBE_FLAG: &str = "TEBAKO_INTERPOSE_PROBE";
/// The recursion depth guard: a working insertion re-execs EXACTLY once
/// (depth 1 inserts, depth 2 proceeds); anything deeper means the
/// sentinel did not stick.
const DEPTH_VAR: &str = "TEBAKO_INTERPOSE_DEPTH";

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
    // only the probe reruns.
    let args: Vec<String> = std::env::args().collect();
    let mut cargv = CArgv::new(&args);
    let mut argc: c_int = args.len() as c_int;
    let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();
    let root = CString::new("/__tfs__").unwrap();
    let rc =
        unsafe { tebako_driver::ffi::tebako_driver_boot(&mut argc, &mut argvp, root.as_ptr()) };
    assert_eq!(rc, 0, "the plain boot succeeds");
    // Only the re-exec'd child reaches here.
    let sentinel = std::env::var_os("TEBAKO_LOADER_INTERPOSED");
    let insert_var = std::env::var("DYLD_INSERT_LIBRARIES").unwrap_or_default();
    let loaded = interpose_dylib_loaded();
    println!("PROBE depth={depth}");
    println!("PROBE sentinel_present={}", sentinel.is_some());
    println!("PROBE insert_var={insert_var}");
    println!("PROBE dylib_loaded={loaded}");
    assert_eq!(depth, 2, "the re-exec must fire exactly once");
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
    assert!(
        loaded,
        "the interpose dylib is in the process's dyld image list"
    );
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
        .env_remove("TEBAKO_LOADER_INTERPOSED")
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove(DEPTH_VAR)
        .output()
        .expect("spawn the probe harness");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "the probe failed\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    let markers: Vec<&str> = stdout.lines().filter(|l| l.starts_with("PROBE ")).collect();
    assert_eq!(
        markers.len(),
        4,
        "the probe's markers appear exactly once — one completed boot:\n{stdout}"
    );
    assert!(stdout.contains("PROBE depth=2"), "{stdout}");
    assert!(stdout.contains("PROBE sentinel_present=false"), "{stdout}");
    assert!(stdout.contains("PROBE dylib_loaded=true"), "{stdout}");
}
