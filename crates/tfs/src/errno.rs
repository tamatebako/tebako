//! Thread-local errno channel for the C ABI.
//!
//! Mirrors the C++ implementation (`fs_context.cpp`): one thread-local cell
//! shared by the whole C API; every public `tebako_fs_*` function stores its
//! outcome here (0 on success, an errno value on failure).

use std::cell::Cell;

thread_local! {
    static ERRNO: Cell<i32> = const { Cell::new(0) };
}

/// Store an errno value and return it (for tail-position convenience).
/// Also writes the C `errno`: POSIX consumers of the C ABI (the ruby
/// io-routing patches, any `tebako_fs_*` caller) read the thread's
/// errno on failure, and an answer they cannot see is an answer that
/// never happened (a stale 0 surfaces as `Errno::NOERROR`).
///
/// Windows matters just as much: without the CRT-errno write, the msys
/// io shims read a STALE CRT errno from whatever host call last failed,
/// and ruby surfaces that instead of the real answer (bundler's
/// ProcessLock met a leftover EBADF where the held-tree gate had
/// answered EROFS — the 2026-08-01 boot-smoke class). The UCRT's
/// `_errno()` accessor lives in ucrtbase.dll, present on every supported
/// Windows.
#[cfg(windows)]
mod crt_errno {
    extern "C" {
        pub fn _errno() -> *mut libc::c_int;
    }
}

pub fn set_errno(err: i32) -> i32 {
    ERRNO.with(|c| c.set(err));
    // The FFI boundary: the one place touching the C errno cell.
    unsafe {
        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
        {
            *libc::__error() = err;
        }
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            *libc::__errno_location() = err;
        }
        #[cfg(windows)]
        {
            *crt_errno::_errno() = err;
        }
    }
    err
}

/// The error code of the last C API operation on this thread.
pub fn get_errno() -> i32 {
    ERRNO.with(|c| c.get())
}

/// Static, borrowed message for an errno value.
///
/// The C++ implementation defers to `std::strerror`; we use a static table
/// for the codes this library produces (stable storage, no locale/TLS
/// quirks) and fall back to "Unknown error".
pub fn strerror(err: i32) -> &'static [u8] {
    // All strings are NUL-terminated C strings.
    match err {
        0 => c"Success",
        libc::EPERM => c"Operation not permitted",
        libc::ENOENT => c"No such file or directory",
        libc::ESRCH => c"No such process",
        libc::EINTR => c"Interrupted system call",
        libc::EIO => c"Input/output error",
        libc::ENXIO => c"No such device or address",
        libc::EBADF => c"Bad file descriptor",
        libc::EACCES => c"Permission denied",
        libc::EFAULT => c"Bad address",
        libc::EBUSY => c"Device or resource busy",
        libc::EEXIST => c"File exists",
        libc::EXDEV => c"Cross-device link",
        libc::ENODEV => c"No such device",
        libc::ENOTDIR => c"Not a directory",
        libc::EISDIR => c"Is a directory",
        libc::EINVAL => c"Invalid argument",
        libc::ENFILE => c"Too many open files in system",
        libc::EMFILE => c"Too many open files",
        libc::EFBIG => c"File too large",
        libc::ENOSPC => c"No space left on device",
        libc::EROFS => c"Read-only file system",
        libc::ENAMETOOLONG => c"File name too long",
        libc::ENOTEMPTY => c"Directory not empty",
        libc::ELOOP => c"Too many levels of symbolic links",
        libc::ENOMEM => c"Cannot allocate memory",
        libc::EALREADY => c"Operation already in progress",
        libc::ENOTSUP => c"Operation not supported",
        crate::ENOKEY => c"Required key not available",
        _ => c"Unknown error",
    }
    .to_bytes_with_nul()
}

/// The Rust-text view of [`strerror`]: the same static message without
/// the C wire's NUL terminator. Text consumers (formatted diagnostics,
/// FFI error strings that ride `CString::new` — whose interior-NUL
/// rejection would otherwise silently drop the message) take this view;
/// the bytes view stays NUL-terminated for the C ABI (`tebako_strerror`).
pub fn strerror_text(err: i32) -> &'static str {
    let bytes = strerror(err);
    // The table entries are NUL-terminated ASCII: the terminator is
    // exactly the last byte, and the remainder is always valid UTF-8.
    std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap_or("Unknown error")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two views of one message: the C wire keeps its NUL
    /// terminator (`tebako_strerror` returns the pointer), the Rust
    /// text view drops it — a terminal NUL inside a formatted
    /// diagnostic trips `CString::new`'s interior-NUL rejection at the
    /// FFI boundary and the message vanishes (spec 00 §9's named-error
    /// guarantee).
    #[test]
    fn strerror_text_is_the_wire_string_minus_its_nul() {
        for code in [0, 1, 2, 13, 22, 65, 75, 76, -1, 9999] {
            let wire = strerror(code);
            assert_eq!(wire.last(), Some(&0), "the C view stays terminated");
            let text = strerror_text(code);
            assert!(!text.is_empty());
            assert!(!text.contains('\0'), "code {code}: NUL in '{text}'");
            assert_eq!(text.as_bytes(), &wire[..wire.len() - 1]);
        }
    }
}
