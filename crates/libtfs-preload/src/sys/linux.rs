//! Linux delivery: symbol interposition (activated by `LD_PRELOAD`) + the
//! library constructor.
//!
//! The shim functions in `sys/mod.rs` are exported under their libc names
//! (`#[no_mangle]` there does the work — this module resolves the
//! originals and registers the constructor). Because this library precedes
//! libc in the global link scope, every subsequently resolved reference
//! binds to the shim; the originals are recovered with
//! `dlsym(RTLD_NEXT, …)`, which searches the scope AFTER this object.
//! (macOS cannot use this mechanism — dyld interpose redirects dlsym
//! results too — so the real-function plumbing is per-platform.)
//!
//! Coverage note: binaries built against glibc ≥ 2.33 call
//! `stat`/`lstat`/`fstat`/`fstatat` directly and are covered; binaries
//! built against older glibc use the versioned `__xstat`/`__lxstat`/
//! `__fxstat`/`__fxstatat` entry points, which are interposed as well
//! (roadmap 39). The LFS `open64`/`stat64`/`lstat64`/`fstat64`/`pread64`
//! family is interposed too — Rust std and `_FILE_OFFSET_BITS=64` builds
//! call the 64 variants directly, and they are DISTINCT exported symbols
//! from the plain names; the LFS `__xstat64`/`__lxstat64`/`__fxstat64`/
//! `__fxstatat64` versioned forms remain a documented gap.

use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Resolve the original libc implementation of `$sym` (cached).
macro_rules! real_fn {
    ($name:ident, $sym:expr, $ty:ty) => {
        pub(super) fn $name() -> $ty {
            static SLOT: AtomicUsize = AtomicUsize::new(0);
            let mut p = SLOT.load(Ordering::Relaxed);
            if p == 0 {
                // SAFETY: dlsym with a valid NUL-terminated symbol name.
                p = unsafe { libc::dlsym(libc::RTLD_NEXT, $sym.as_ptr()) } as usize;
                if p == 0 {
                    // Do not re-run dlsym on every call after a failure.
                    p = usize::MAX;
                }
                SLOT.store(p, Ordering::Relaxed);
            }
            assert!(
                p != usize::MAX,
                "libtfs-preload: cannot resolve libc symbol {:?} via RTLD_NEXT",
                $sym
            );
            // SAFETY: the resolved address is the libc function of type $ty.
            unsafe { std::mem::transmute::<usize, $ty>(p) }
        }
    };
}

real_fn!(
    real_open,
    c"open",
    unsafe extern "C" fn(*const c_char, c_int, ...) -> c_int
);
real_fn!(
    real_openat,
    c"openat",
    unsafe extern "C" fn(c_int, *const c_char, c_int, ...) -> c_int
);
real_fn!(
    real_stat,
    c"stat",
    unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int
);
real_fn!(
    real_lstat,
    c"lstat",
    unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int
);
real_fn!(
    real_fstat,
    c"fstat",
    unsafe extern "C" fn(c_int, *mut libc::stat) -> c_int
);
real_fn!(
    real_access,
    c"access",
    unsafe extern "C" fn(*const c_char, c_int) -> c_int
);
real_fn!(
    real_faccessat,
    c"faccessat",
    unsafe extern "C" fn(c_int, *const c_char, c_int, c_int) -> c_int
);
real_fn!(
    real_opendir,
    c"opendir",
    unsafe extern "C" fn(*const c_char) -> *mut libc::DIR
);
real_fn!(
    real_readdir,
    c"readdir",
    unsafe extern "C" fn(*mut libc::DIR) -> *mut libc::dirent
);
real_fn!(
    real_readdir64,
    c"readdir64",
    unsafe extern "C" fn(*mut libc::DIR) -> *mut libc::dirent
);
real_fn!(
    real_closedir,
    c"closedir",
    unsafe extern "C" fn(*mut libc::DIR) -> c_int
);
real_fn!(
    real_pread,
    c"pread",
    unsafe extern "C" fn(c_int, *mut c_void, usize, libc::off_t) -> libc::ssize_t
);
real_fn!(
    real_read,
    c"read",
    unsafe extern "C" fn(c_int, *mut c_void, usize) -> libc::ssize_t
);
real_fn!(
    real_lseek,
    c"lseek",
    unsafe extern "C" fn(c_int, libc::off_t, c_int) -> libc::off_t
);
real_fn!(real_close, c"close", unsafe extern "C" fn(c_int) -> c_int);
real_fn!(
    real_mkdir,
    c"mkdir",
    unsafe extern "C" fn(*const c_char, libc::mode_t) -> c_int
);
real_fn!(
    real_unlink,
    c"unlink",
    unsafe extern "C" fn(*const c_char) -> c_int
);
real_fn!(
    real_rename,
    c"rename",
    unsafe extern "C" fn(*const c_char, *const c_char) -> c_int
);
real_fn!(
    real_dlopen,
    c"dlopen",
    unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void
);
real_fn!(
    real_fstatat,
    c"fstatat",
    unsafe extern "C" fn(c_int, *const c_char, *mut libc::stat, c_int) -> c_int
);
real_fn!(
    real_fstatat64,
    c"fstatat64",
    unsafe extern "C" fn(c_int, *const c_char, *mut libc::stat, c_int) -> c_int
);
real_fn!(
    real_statx,
    c"statx",
    unsafe extern "C" fn(c_int, *const c_char, c_int, libc::c_uint, *mut libc::statx) -> c_int
);
real_fn!(
    real_getdents64,
    c"getdents64",
    unsafe extern "C" fn(c_int, *mut c_void, usize) -> libc::ssize_t
);
// the libc symbol is `__xstat` (versioned pre-glibc-2.33 entry)
real_fn!(
    real___xstat,
    c"__xstat",
    unsafe extern "C" fn(c_int, *const c_char, *mut libc::stat) -> c_int
);
real_fn!(
    real___lxstat,
    c"__lxstat",
    unsafe extern "C" fn(c_int, *const c_char, *mut libc::stat) -> c_int
);
real_fn!(
    real___fxstat,
    c"__fxstat",
    unsafe extern "C" fn(c_int, c_int, *mut libc::stat) -> c_int
);
real_fn!(
    real___fxstatat,
    c"__fxstatat",
    unsafe extern "C" fn(c_int, c_int, *const c_char, *mut libc::stat, c_int) -> c_int
);
real_fn!(
    real_rewinddir,
    c"rewinddir",
    unsafe extern "C" fn(*mut libc::DIR)
);
real_fn!(
    real_open64,
    c"open64",
    unsafe extern "C" fn(*const c_char, c_int, ...) -> c_int
);
real_fn!(
    real_stat64,
    c"stat64",
    unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int
);
real_fn!(
    real_lstat64,
    c"lstat64",
    unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int
);
real_fn!(
    real_fstat64,
    c"fstat64",
    unsafe extern "C" fn(c_int, *mut libc::stat) -> c_int
);
real_fn!(
    real_pread64,
    c"pread64",
    unsafe extern "C" fn(c_int, *mut c_void, usize, libc::off_t) -> libc::ssize_t
);
real_fn!(
    real_dirfd,
    c"dirfd",
    unsafe extern "C" fn(*mut libc::DIR) -> c_int
);
real_fn!(
    real_telldir,
    c"telldir",
    unsafe extern "C" fn(*mut libc::DIR) -> libc::c_long
);
real_fn!(
    real_seekdir,
    c"seekdir",
    unsafe extern "C" fn(*mut libc::DIR, libc::c_long)
);
real_fn!(
    real_readdir_r,
    c"readdir_r",
    unsafe extern "C" fn(*mut libc::DIR, *mut libc::dirent, *mut *mut libc::dirent) -> c_int
);
real_fn!(
    real_execve,
    c"execve",
    unsafe extern "C" fn(*const c_char, *const *mut c_char, *const *mut c_char) -> c_int
);
real_fn!(
    real_posix_spawn,
    c"posix_spawn",
    unsafe extern "C" fn(
        *mut libc::pid_t,
        *const c_char,
        *const libc::posix_spawn_file_actions_t,
        *const libc::posix_spawnattr_t,
        *const *mut c_char,
        *const *mut c_char,
    ) -> c_int
);
real_fn!(
    real_posix_spawnp,
    c"posix_spawnp",
    unsafe extern "C" fn(
        *mut libc::pid_t,
        *const c_char,
        *const libc::posix_spawn_file_actions_t,
        *const libc::posix_spawnattr_t,
        *const *mut c_char,
        *const *mut c_char,
    ) -> c_int
);

/// Library constructor: establish the namespace before the program's main.
#[used]
#[link_section = ".init_array"]
static INIT: extern "C" fn() = {
    extern "C" fn init() {
        super::init();
    }
    init
};
