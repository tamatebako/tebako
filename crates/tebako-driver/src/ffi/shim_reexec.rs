//! macOS in-process file-IO visibility (tebako#1125, spec 22 §3.6): the
//! post-boot shim re-exec.
//!
//! The gap. On Linux the runtime exe links the exe-shape archive and the
//! driver arms the file-IO family in-process (`crate::exe_shape`). On
//! macOS dyld honors `__interpose` tuples only from a dylib, so the exe
//! shape has no in-process spelling: the boot-head micro dylib carries
//! only the dlopen/dlerror tuples (class L), and the preload shim — the
//! full family — reached only SPAWNED CHILDREN (the injection export's
//! `DYLD_INSERT_LIBRARIES`). An unpatched native extension's raw
//! `open(2)`/`read(2)` of an in-image path therefore died ENOENT in the
//! interpreter process itself while a spawned child answered it —
//! tebako#1125's macOS half.
//!
//! The delivery. After the boot has mounted the namespace and the
//! injection export has materialized the shim (its host path arrives in
//! the `BootOutcome`), the boot's TAIL re-execs the interpreter once
//! with `DYLD_INSERT_LIBRARIES` naming the shim FIRST and the boot's own
//! micro dylib SECOND. dyld inserts both at the exec: the shim's
//! constructor rebuilds the namespace from `TEBAKO_TFS_MOUNTS` (the
//! shipped, child-proven injection path — no new shim code) and its
//! tuples cover the whole process, the exe's own C-library reads
//! included (dyld interposition is fixup-format-agnostic — the rebind
//! shape's chained-fixups blind spot does not exist here). The re-exec'd
//! boot is a fresh process — no double mount, no partial-mount window —
//! and it replays the caller's ORIGINAL argv, so the argv write-back
//! happens exactly once, in the final incarnation. The model is the R
//! runtime's proven two-incarnation boot (tebako-runtime-r's
//! `tebako_r_main.c`), generalized into the driver: every runtime whose
//! env image declares `preload_shim` gets the family in-process with no
//! factory change.
//!
//! The once-only. [`SENTINEL`] rides the re-exec; the next boot's HEAD
//! captures and CLEARS it (a verdict about this incarnation, never a
//! value for a child's env) and records whether the shim actually
//! LOADED — dyld drops an insertion silently on a hardened-runtime exe
//! without the entitlement pair, so presence is verified, never assumed.
//! A dropped insertion is a loud degraded line and the boot continues
//! (named, never a silent fallback); the macOS-15 AMFI gate that already
//! guards the head self-insertion guards this re-exec the same way.
//!
//! The env shape. The re-exec'd boot's head scrub (interpose.rs) strips
//! the micro entry, and the injection export then re-exports its own
//! value — the child env ends byte-identical with the pre-change shape.
//! Foreign (non-tebako) insert entries a user armed are captured at the
//! head and carried across the re-exec's re-compose: the export's
//! replacement covers CHILDREN, but without the capture the re-exec
//! would drop the user's dylib from THIS process as well. The exe-shape
//! arm consults [`shim_covers_in_process`] so the covered incarnation
//! names no missing-family gap.
//!
//! Compiles out on every non-macOS target.

#![allow(unsafe_code)]

use std::ffi::{CStr, CString, OsStr, OsString};
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::RwLock;

use libc::c_char;

use crate::ffi::interpose;

/// The once-per-process sentinel: armed by the tail's re-exec, captured
/// and cleared by the re-exec'd boot's head. Never inherited by a child
/// as a live value — the head clears it before any spawn, and a
/// respawned tebako exe re-decides from its own boot.
pub(crate) const SENTINEL: &str = "TEBAKO_DRIVER_SHIM_BOOTED";

/// The preload shim's artifact basename — the loaded-image scan's key.
/// The name is pinned across the ecosystem (the factory stages it, the
/// tfs CLI stages it, ci gates it); a second spelling would be a new
/// artifact, not a rename.
const SHIM_BASENAME: &str = "libtfs_preload.dylib";

/// The coverage verdicts a boot head can reach.
const UNKNOWN: u8 = 0; // no shim seen — every pre-injection boot
const PRESENT: u8 = 1; // the shim's image is loaded — the family is live
const DROPPED: u8 = 2; // the sentinel was armed but dyld did not load it

/// This boot's verdicts — re-decided at EVERY boot head (test processes
/// boot repeatedly; ffi.rs's miniruby flag's discipline).
static BOOTED: AtomicBool = AtomicBool::new(false);
static COVERAGE: AtomicU8 = AtomicU8::new(UNKNOWN);
/// The foreign (non-tebako) insert entries the head captured, carried
/// across the tail's re-compose so a user's own insertion survives the
/// re-exec in THIS process (see the module doc).
static PRESERVED: RwLock<Vec<OsString>> = RwLock::new(Vec::new());

unsafe extern "C" {
    // The dyld image-list surface, declared directly: the libc crate
    // deprecated these re-exports in favor of mach2 — the same choice
    // rebind.rs and tests/interpose.rs make for the same stable calls.
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_name(image_index: u32) -> *const c_char;
}

/// Is `entry` a spelling of the preload shim? Basename match: the
/// materialized exec-cache copy keeps the in-image name.
fn is_shim_entry(entry: &OsStr) -> bool {
    Path::new(entry).file_name() == Some(OsStr::new(SHIM_BASENAME))
}

/// The insert list's foreign entries: every entry that is neither the
/// boot's micro dylib (interpose.rs's marker) nor the shim itself. Both
/// tebako entries are re-composed by their own owners; anything else
/// (a user's own instrumentation dylib) rides through verbatim.
fn foreign_entries(value: Option<OsString>) -> Vec<OsString> {
    let Some(v) = value else { return Vec::new() };
    v.as_bytes()
        .split(|b| *b == b':')
        .filter(|e| !e.is_empty())
        .map(OsStr::from_bytes)
        .filter(|e| !interpose::is_micro_dylib_entry(e))
        .filter(|e| !is_shim_entry(e))
        .map(OsString::from)
        .collect()
}

/// Is the shim's image loaded in THIS process? Presence in the dyld
/// image list IS the verdict: dyld lists an inserted library exactly
/// when it loaded it (tests/interpose.rs's proof shape).
fn shim_loaded() -> bool {
    let n = unsafe { _dyld_image_count() };
    for i in 0..n {
        let p = unsafe { _dyld_get_image_name(i) };
        if p.is_null() {
            continue;
        }
        let name = unsafe { CStr::from_ptr(p) }.to_string_lossy();
        if is_shim_entry(OsStr::new(name.as_ref())) {
            return true;
        }
    }
    false
}

/// The boot head's half: capture and CLEAR the sentinel, capture the
/// insert list's foreign entries, and record the coverage verdict the
/// arm and the tail consult. Runs BEFORE the head self-insertion (its
/// scrub would take the micro entries out from under the capture).
pub(crate) fn head() {
    let booted = std::env::var_os(SENTINEL).is_some_and(|v| !v.is_empty());
    std::env::remove_var(SENTINEL);
    BOOTED.store(booted, Ordering::SeqCst);
    *PRESERVED.write().unwrap() = foreign_entries(std::env::var_os(interpose::INSERT_VAR));
    let verdict = if shim_loaded() {
        // The family is already live: the re-exec'd incarnation, or a
        // respawned tebako child that INHERITED the insertion (its
        // parent's env named the shim and dyld inserted it at exec) —
        // armed either way, no tail work.
        PRESENT
    } else if booted {
        // We armed the insertion and dyld did not load it — the tail
        // names this loudly.
        DROPPED
    } else {
        UNKNOWN
    };
    COVERAGE.store(verdict, Ordering::SeqCst);
}

/// The exe-shape arm's consult (spec 22 §3.6): the covered incarnation
/// carries the shim's interposed family process-wide, so a missing
/// in-exe family is no gap to name.
pub(crate) fn shim_covers_in_process() -> bool {
    COVERAGE.load(Ordering::SeqCst) == PRESENT
}

/// The tail's decision, split from [`tail`] for tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    /// The family is already live in this process — nothing to do.
    Covered,
    /// The re-exec'd incarnation WITHOUT the family: dyld dropped the
    /// armed insertion — the loud degraded line, boot continues.
    Dropped,
    /// An insert re-exec of this exe would die at dyld (macOS 15+, no
    /// CMS blob) — the loud skip, boot continues.
    SkipAmfi,
    /// Re-exec with the shim inserted.
    Reexec,
}

fn decide(booted: bool, coverage: u8, amfi_kills: bool) -> Decision {
    if booted {
        return if coverage == PRESENT {
            Decision::Covered
        } else {
            Decision::Dropped
        };
    }
    if coverage == PRESENT {
        return Decision::Covered;
    }
    if amfi_kills {
        return Decision::SkipAmfi;
    }
    Decision::Reexec
}

/// The boot tail's half: re-exec once with the shim inserted, or name
/// why not and return. `shim_host` is the injection export's
/// materialized host path; `c_argv` is the process's ORIGINAL C argv
/// (NULL-terminated — the boot entries' FFI contract), replayed
/// verbatim. Called only when the env image declared a shim; every
/// failure is a loud continue, never a hard boot error (the pre-change
/// behavior IS the fallback).
pub(crate) fn tail(shim_host: &str, c_argv: *mut *mut c_char) {
    let amfi_kills = std::env::current_exe()
        .map(|exe| interpose::amfi_kills_this_exe(&exe))
        .unwrap_or(false);
    match decide(
        BOOTED.load(Ordering::SeqCst),
        COVERAGE.load(Ordering::SeqCst),
        amfi_kills,
    ) {
        Decision::Covered => {}
        Decision::Dropped => eprintln!(
            "tebako: preload-shim insertion dropped: dyld did not load '{shim_host}' from the \
             armed DYLD_INSERT_LIBRARIES (a hardened-runtime exe without the entitlement pair \
             strips it) — the interpreter process runs WITHOUT the in-process file-IO family: \
             an unpatched native extension's raw file reads meet the host filesystem only"
        ),
        Decision::SkipAmfi => eprintln!(
            "tebako: preload-shim insertion skipped: this exe's adhoc signature carries no CMS \
             blob, and macOS 15+'s AMFI kills an insert re-exec of it — the interpreter process \
             runs without the in-process file-IO family (a signed runtime is unaffected)"
        ),
        Decision::Reexec => {
            if let Err(reason) = try_reexec(shim_host, c_argv) {
                eprintln!(
                    "tebako: preload-shim insertion unavailable: {reason} — the interpreter \
                     process runs without the in-process file-IO family"
                );
            }
        }
    }
}

/// The re-exec proper. Every step before the execv returns the failure
/// as the reason; the execv itself returns only on failure, which
/// becomes the reason (interpose.rs's try_insert discipline).
fn try_reexec(shim_host: &str, c_argv: *mut *mut c_char) -> Result<(), String> {
    if c_argv.is_null() {
        return Err("the process argv is unavailable".to_string());
    }
    let shim = Path::new(shim_host);
    if !shim.is_file() {
        return Err(format!(
            "the materialized shim '{}' is not on disk",
            shim.display()
        ));
    }
    let micro = interpose::ensure_micro_dylib(&std::env::temp_dir())?;
    // The insert list: the shim FIRST, the boot's micro dylib SECOND,
    // the head's captured foreign entries after both. The re-exec'd
    // boot's head scrub strips the micro, the injection export
    // re-exports its own value, and the interpose sentinel (below)
    // keeps the head self-insertion from firing again — the child env
    // ends byte-identical with the pre-change shape.
    let mut insert = interpose::compose_insert_libraries(shim, Some(micro.into_os_string()));
    for entry in PRESERVED.read().unwrap().iter() {
        insert.push(":");
        insert.push(entry);
    }
    std::env::set_var(interpose::INSERT_VAR, insert);
    std::env::set_var(interpose::SENTINEL, "1");
    std::env::set_var(SENTINEL, "1");
    let exe = std::env::current_exe()
        .map_err(|e| format!("cannot determine own executable path: {e}"))?;
    let exe_c = CString::new(exe.as_os_str().as_bytes()).map_err(|_| {
        format!(
            "own executable path '{}' contains a NUL byte",
            exe.display()
        )
    })?;
    // SAFETY: execv replaces the process on success — it only ever
    // returns the failure. exe_c and c_argv are valid NUL-terminated C
    // strings (the FFI contract); environ rides along with the three
    // variables above as the only delta.
    unsafe { libc::execv(exe_c.as_ptr(), c_argv as *const *const c_char) };
    Err(format!(
        "execv of '{}' failed: {}",
        exe.display(),
        std::io::Error::last_os_error()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shim_entry_match_is_basename_exact() {
        assert!(is_shim_entry(OsStr::new(
            "/var/folders/x/T/tebako-exec-a/tebako-dlcache/k/__tfs__/lib/tebako/libtfs_preload.dylib"
        )));
        assert!(is_shim_entry(OsStr::new("libtfs_preload.dylib")));
        assert!(!is_shim_entry(OsStr::new(
            "/opt/tebako/libtfs_preload.dylib.bak"
        )));
        assert!(!is_shim_entry(OsStr::new("/opt/libtfs_preload.so")));
        assert!(!is_shim_entry(OsStr::new("")));
    }

    #[test]
    fn the_foreign_capture_drops_only_tebakos_own_entries() {
        let shim = "/exec/tebako-dlcache/k/__tfs__/lib/tebako/libtfs_preload.dylib";
        let micro = "/var/folders/x/T/tebako-interpose/ab12cd.dylib";
        // No var at all → nothing to carry.
        assert_eq!(foreign_entries(None), Vec::<OsString>::new());
        // The tail-composed list (shim + micro + a user's dylib) → only
        // the user's entry is foreign.
        assert_eq!(
            foreign_entries(Some(OsString::from(format!(
                "{shim}:{micro}:/a/user.dylib"
            )))),
            vec![OsString::from("/a/user.dylib")]
        );
        // The head self-insertion's list (micro first, the user's own
        // value preserved) → the micro goes, the rest rides through in
        // order.
        assert_eq!(
            foreign_entries(Some(OsString::from(format!(
                "{micro}:/a/user.dylib:/b/other.dylib"
            )))),
            vec![
                OsString::from("/a/user.dylib"),
                OsString::from("/b/other.dylib")
            ]
        );
        // An inherited shim-only list (the respawned-child shape) →
        // nothing foreign.
        assert_eq!(
            foreign_entries(Some(OsString::from(shim))),
            Vec::<OsString>::new()
        );
        // Empty segments carry nothing.
        assert_eq!(
            foreign_entries(Some(OsString::from(format!("{micro}::/a/user.dylib")))),
            vec![OsString::from("/a/user.dylib")]
        );
    }

    #[test]
    fn the_decision_table_is_mece() {
        // The re-exec'd incarnation: the family landed → done; it did
        // not → the degraded line, never a second re-exec (the once-only
        // is the sentinel's whole point).
        assert_eq!(decide(true, PRESENT, false), Decision::Covered);
        assert_eq!(decide(true, PRESENT, true), Decision::Covered);
        assert_eq!(decide(true, DROPPED, false), Decision::Dropped);
        // booted with no verdict is the same loud shape (unreachable —
        // the head pins DROPPED — but never a loop).
        assert_eq!(decide(true, UNKNOWN, false), Decision::Dropped);
        // Not the re-exec'd incarnation, but the family is live anyway
        // (an inherited insertion): done — no re-exec, no warning.
        assert_eq!(decide(false, PRESENT, false), Decision::Covered);
        assert_eq!(decide(false, PRESENT, true), Decision::Covered);
        // The first boot, no family: the AMFI gate wins over the re-exec.
        assert_eq!(decide(false, UNKNOWN, true), Decision::SkipAmfi);
        assert_eq!(decide(false, UNKNOWN, false), Decision::Reexec);
        assert_eq!(decide(false, DROPPED, false), Decision::Reexec);
    }

    #[test]
    fn the_loaded_scan_sees_the_test_processes_own_images() {
        // The scan itself, proven against an image that MUST be listed:
        // the running test binary (image 0 in every dyld list). The shim
        // is never loaded here — the negative half is the shipped shape.
        let exe = std::env::current_exe().unwrap();
        let mut saw_self = false;
        let n = unsafe { _dyld_image_count() };
        assert!(n > 0);
        for i in 0..n {
            let p = unsafe { _dyld_get_image_name(i) };
            if p.is_null() {
                continue;
            }
            let name = unsafe { CStr::from_ptr(p) }.to_string_lossy();
            if name == exe.to_string_lossy() {
                saw_self = true;
            }
        }
        assert!(saw_self, "the dyld image list names this test binary");
        assert!(!shim_loaded(), "no preload shim rides a cargo test process");
    }

    #[test]
    fn the_head_clears_the_sentinel_and_records_the_verdict() {
        // env-mutating: the crate's one test lock serializes it.
        let _g = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var(SENTINEL, "1");
        head();
        assert!(
            std::env::var_os(SENTINEL).is_none(),
            "the head clears the sentinel — a child never inherits it"
        );
        assert!(BOOTED.load(Ordering::SeqCst));
        assert_eq!(
            COVERAGE.load(Ordering::SeqCst),
            DROPPED,
            "armed-but-unloaded is the dropped verdict in a shimless test process"
        );
        head();
        assert!(
            !BOOTED.load(Ordering::SeqCst),
            "a second boot re-decides from its own env"
        );
        assert_eq!(COVERAGE.load(Ordering::SeqCst), UNKNOWN);
    }
}
