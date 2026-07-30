//! The C entry surface (spec 17 §1): the interpreter's `main` calls the
//! boot first and continues with the rewritten argv on success. This
//! module is the crate's only FFI boundary — `unsafe` lives here and
//! nowhere else (spec 14 §3).
//!
//! Two entries share one boot core:
//!
//! - [`tebako_driver_boot`] — the generic entry (any language): the
//!   caller names the runtime root.
//! - [`tebako_main`] — the ruby-runtime entry (the patched ruby's
//!   `main.c` hook): the root is the ruby memfs point, plus the
//!   build-time `miniruby` pass-through and the `TEBAKO_CONTRACT_VERSION`
//!   environment export (roadmap 45). The three compat getters
//!   ([`tebako_mount_point`], [`tebako_original_pwd`],
//!   [`tebako_is_running_miniruby`]) are exactly the surface the ruby
//!   io-routing patches and the toolchain stub reference.

#![allow(unsafe_code)]

use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

use libc::{c_char, c_int};

use crate::driver::ProcessEnv;
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

/// The ruby runtime root — the memfs mount point the patched ruby is
/// compiled against (a ruby-factory constant).
const RUBY_RUNTIME_ROOT: &str = "/__tebako_memfs__";

static DEFAULT_ROOT: &[u8] = b"/__tebako_memfs__\0";
static EMPTY: &[u8] = b"\0";

/// The mount point the boot established (read by the io-routing patches
/// via [`tebako_mount_point`]); set once, process-lifetime.
static MOUNT_POINT: OnceLock<CString> = OnceLock::new();
/// The process cwd captured at boot ([`tebako_original_pwd`]).
static ORIGINAL_PWD: OnceLock<CString> = OnceLock::new();
/// -1 when the boot passed through for `miniruby` (the build-time tool).
static RUNNING_MINIRUBY: AtomicI32 = AtomicI32::new(0);

/// The compiled-in contract version (spec 06 §6) — the runtime factory's
/// release manifest declares the same value, and its CI fails any
/// release where the two disagree.
///
/// # Safety
/// No pointers, no state — callable from any context.
#[no_mangle]
pub unsafe extern "C" fn tebako_driver_contract_version() -> u32 {
    crate::TEBAKO_CONTRACT_VERSION
}

/// The mount point the boot established (the runtime root). Before any
/// boot, the ruby default.
///
/// # Safety
/// Returns a process-lifetime C string; never free it.
#[no_mangle]
pub unsafe extern "C" fn tebako_mount_point() -> *const c_char {
    MOUNT_POINT
        .get()
        .map(|c| c.as_ptr())
        .unwrap_or(DEFAULT_ROOT.as_ptr() as *const c_char)
}

/// The process cwd captured at boot; empty before any boot.
///
/// # Safety
/// Returns a process-lifetime C string; never free it.
#[no_mangle]
pub unsafe extern "C" fn tebako_original_pwd() -> *const c_char {
    ORIGINAL_PWD
        .get()
        .map(|c| c.as_ptr())
        .unwrap_or(EMPTY.as_ptr() as *const c_char)
}

/// -1 when the boot passed through for `miniruby`, 0 otherwise.
///
/// # Safety
/// No pointers, plain load — callable from any context.
#[no_mangle]
pub unsafe extern "C" fn tebako_is_running_miniruby() -> c_int {
    RUNNING_MINIRUBY.load(Ordering::SeqCst)
}

/// The generic boot. `argc`/`argv` point at the process argv (argv[0]
/// included — the handoff grammar scans past it); on success they are
/// REPLACED with the driver-owned rewritten argv (leaked deliberately:
/// the boot runs once, pre-interpreter, and the memory lives exactly as
/// long as the process). `runtime_root` is the mount point the
/// interpreter was compiled against.
///
/// Returns 0 on success; a named loader exit code (65–74) on failure,
/// with the named message on stderr and nothing left mounted.
///
/// # Safety
/// C ABI entry point: `argc`, `argv`, `*argv`, and `runtime_root` must
/// be valid per the C contract (`argv[i]` NUL-terminated strings,
/// `runtime_root` NUL-terminated).
#[no_mangle]
pub unsafe extern "C" fn tebako_driver_boot(
    argc: *mut c_int,
    argv: *mut *mut *mut c_char,
    runtime_root: *const c_char,
) -> c_int {
    if runtime_root.is_null() {
        return EX_TEBAKO_MANIFEST;
    }
    boot_impl(argc, argv, runtime_root)
}

/// The ruby-runtime entry (the patched ruby `main.c` hook). Identical to
/// [`tebako_driver_boot`] with the ruby memfs root, plus:
///
/// - the `miniruby` pass-through: the build-time tool links the same
///   patched `main.c` and must run as a plain interpreter (no mounts, no
///   exports — the v1 `tebako_main` behavior);
/// - the `TEBAKO_CONTRACT_VERSION` environment export (roadmap 45): the
///   runtime is authoritative for the contract it speaks, so an
///   inherited value is always overwritten;
/// - the mount-point and original-pwd getters, set for the io-routing
///   patches.
///
/// # Safety
/// Same contract as [`tebako_driver_boot`], minus `runtime_root`.
#[no_mangle]
pub unsafe extern "C" fn tebako_main(argc: *mut c_int, argv: *mut *mut *mut c_char) -> c_int {
    if argc.is_null() || argv.is_null() {
        return EX_TEBAKO_IO;
    }
    let argvp = unsafe { *argv };
    if argvp.is_null() {
        return EX_TEBAKO_IO;
    }
    let n = unsafe { *argc };
    if n < 1 {
        return EX_TEBAKO_IO;
    }
    let argv0 = unsafe { CStr::from_ptr(*argvp) }.to_string_lossy();
    if argv0.contains("miniruby") {
        RUNNING_MINIRUBY.store(-1, Ordering::SeqCst);
        return 0;
    }
    std::env::set_var(
        "TEBAKO_CONTRACT_VERSION",
        crate::TEBAKO_CONTRACT_VERSION.to_string(),
    );
    let root = CString::new(RUBY_RUNTIME_ROOT).expect("static root is NUL-free");
    boot_impl(argc, argv, root.as_ptr())
}

/// The shared boot core (both entries). `runtime_root` is a valid
/// NUL-terminated string (checked by the callers).
fn boot_impl(argc: *mut c_int, argv: *mut *mut *mut c_char, runtime_root: *const c_char) -> c_int {
    if argc.is_null() || argv.is_null() {
        return EX_TEBAKO_IO;
    }
    let argvp = unsafe { *argv };
    if argvp.is_null() {
        return EX_TEBAKO_IO;
    }
    let n = unsafe { *argc };
    if n < 0 {
        return EX_TEBAKO_IO;
    }
    let root = unsafe { CStr::from_ptr(runtime_root) }
        .to_string_lossy()
        .into_owned();
    if let Ok(c) = CString::new(root.as_str()) {
        let _ = MOUNT_POINT.set(c);
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(c) = CString::new(cwd.to_string_lossy().as_ref()) {
            let _ = ORIGINAL_PWD.set(c);
        }
    }
    let raw: &[*mut c_char] = unsafe { std::slice::from_raw_parts(argvp, n as usize) };
    let args: Vec<String> = raw
        .iter()
        .map(|&p| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
        .collect();

    match crate::driver::boot(&args, &root, &ProcessEnv) {
        Ok(outcome) => {
            let mut cstrings = Vec::with_capacity(outcome.argv.len());
            for s in &outcome.argv {
                let Ok(c) = CString::new(s.as_str()) else {
                    eprintln!("tebako-driver: argv value contains an interior NUL");
                    return EX_TEBAKO_MANIFEST;
                };
                cstrings.push(c);
            }
            let mut ptrs: Vec<*mut c_char> =
                cstrings.iter().map(|c| c.as_ptr() as *mut c_char).collect();
            ptrs.push(std::ptr::null_mut());
            let argv_out = Box::leak(ptrs.into_boxed_slice()).as_mut_ptr();
            // The strings the pointers reference live exactly as long:
            // process-lifetime, never freed.
            std::mem::forget(cstrings.into_boxed_slice());
            unsafe {
                *argv = argv_out;
                *argc = outcome.argv.len() as c_int;
            }
            0
        }
        Err(e) => {
            eprintln!("tebako-driver: {}", e.message);
            e.code
        }
    }
}
