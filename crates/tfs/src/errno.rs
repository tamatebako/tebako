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
pub fn set_errno(err: i32) -> i32 {
    ERRNO.with(|c| c.set(err));
    #[cfg(unix)]
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
        crate::backends_enc::ENOKEY => c"Required key not available",
        _ => c"Unknown error",
    }
    .to_bytes_with_nul()
}
