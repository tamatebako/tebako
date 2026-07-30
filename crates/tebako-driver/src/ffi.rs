//! The C entry surface (spec 17 §1): the interpreter's `main` calls
//! [`tebako_driver_boot`] first and continues with the rewritten argv on
//! success. This module is the crate's only FFI boundary — `unsafe`
//! lives here and nowhere else (spec 14 §3).

#![allow(unsafe_code)]

use std::ffi::{CStr, CString};

use libc::{c_char, c_int};

use crate::driver::ProcessEnv;
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

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

/// The boot. `argc`/`argv` point at the process argv (argv[0] included —
/// the handoff grammar scans past it); on success they are REPLACED with
/// the driver-owned rewritten argv (leaked deliberately: the boot runs
/// once, pre-interpreter, and the memory lives exactly as long as the
/// process). `runtime_root` is the mount point the interpreter was
/// compiled against (ruby: `/__tebako_memfs__`).
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
    if argc.is_null() || argv.is_null() {
        return EX_TEBAKO_IO;
    }
    let argvp = unsafe { *argv };
    if argvp.is_null() {
        return EX_TEBAKO_IO;
    }
    if runtime_root.is_null() {
        return EX_TEBAKO_MANIFEST;
    }
    let n = unsafe { *argc };
    if n < 0 {
        return EX_TEBAKO_IO;
    }
    let root = unsafe { CStr::from_ptr(runtime_root) }
        .to_string_lossy()
        .into_owned();
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
