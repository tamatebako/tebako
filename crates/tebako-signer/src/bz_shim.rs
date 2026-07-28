//! The bz_internal_error shim, in Rust.
//!
//! bzip2 1.0.8 DECLARES bz_internal_error (bzlib_private.h, called by the
//! AssertH macro on assertion failure) but NEVER DEFINES it — anywhere
//! (verified: official tarball, Debian pool, libarchive mirror, bzip2-sys
//! vendored copy). Dynamic libbz2 tolerates the dangling reference (shared
//! libraries allow undefined symbols; the assertion path never executes
//! with valid data), so every distro ships a libbz2 with this hole —
//! Debian bookworm's libbz2.so.1.0 and macOS libbz2.dylib both verified.
//! A STATIC link resolves everything and fails:
//!
//!   undefined reference to `bz_internal_error`
//!     >>> referenced by decompress.c:614 (bzip2-sys's libbz2.a)
//!
//! Provide the trivial ABI-compatible definition HERE in Rust: an
//! exported extern "C" fn links like any other symbol — no C toolchain,
//! no static archive, and crucially NO archive-order games (the C
//! static-archive version of this shim resolved on x86_64 but was
//! link-order-fragile on aarch64; the Rust form is order-independent).
//! Behavior matches upstream bzip2 1.0.6 (report and abort — reached
//! only on a failed internal assertion).

// The crate is #![forbid(unsafe_code)]; no_mangle is an unsafe attribute
// in edition 2024. This module is the single, documented exception.
#![allow(unsafe_code)]

use std::os::raw::c_int;

/// ABI-compatible `bz_internal_error` replacement (see module docs).
#[unsafe(no_mangle)]
pub extern "C" fn bz_internal_error(errcode: c_int) -> ! {
    eprintln!(
        "bzip2: internal assertion failed (error {errcode}) — the compressed stream is inconsistent with this library build"
    );
    std::process::exit(3);
}
