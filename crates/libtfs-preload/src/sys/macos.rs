//! macOS delivery: dyld interpose tuples in `__DATA,__interpose`
//! (activated by `DYLD_INSERT_LIBRARIES`) + the library constructor.
//!
//! Each tuple is `(replacement, replacee)`; dyld rebinds EVERY reference
//! to the replacee process-wide to the replacement — including references
//! inside this library and, notably, the RESULT of `dlsym(RTLD_NEXT)`
//! lookups for the interposed symbol (verified on macOS 14: RTLD_NEXT
//! recurses into the replacement). The one channel dyld preserves is the
//! tuple's own replacee field, which keeps pointing at the original
//! implementation — so the originals are read back from the tuples
//! (volatile: dyld writes them at load time, outside Rust's view).

use std::ffi::{c_char, c_int, c_void};

/// The dyld interpose tuple layout.
#[repr(C)]
struct InterposeTuple {
    replacement: *const c_void,
    replacee: *const c_void,
}

// The tuples are written once by dyld at load time and only ever READ by
// dyld and the volatile readback below.
unsafe impl Sync for InterposeTuple {}

// The libc crate does not declare `faccessat`/`fstatat` for apple
// targets; the SDK has them since macOS 10.10.
extern "C" {
    fn faccessat(dirfd: c_int, path: *const c_char, mode: c_int, flags: c_int) -> c_int;
    fn fstatat(dirfd: c_int, path: *const c_char, st: *mut libc::stat, flags: c_int) -> c_int;
}

macro_rules! interpose {
    ($name:ident, $real:ident, $replacement:expr, $replacee:expr, $ty:ty) => {
        #[used]
        #[link_section = "__DATA,__interpose"]
        static $name: InterposeTuple = InterposeTuple {
            replacement: $replacement as *const c_void,
            replacee: $replacee as *const c_void,
        };
        /// The original implementation, read back from the tuple (see the
        /// module docs for why dlsym(RTLD_NEXT) cannot be used on macOS).
        /// dead_code: a delegating replacement (fopen$DARWIN_EXTSN → fopen)
        /// never calls its own original — it shares the target's body.
        #[allow(dead_code)]
        pub(super) fn $real() -> $ty {
            // SAFETY: the tuple is initialized statically and written by
            // dyld before any user code runs; volatile because dyld's
            // write is invisible to the compiler.
            let p = unsafe { std::ptr::read_volatile(std::ptr::addr_of!($name.replacee)) };
            assert!(
                !p.is_null(),
                concat!(
                    "libtfs-preload: interpose tuple ",
                    stringify!($name),
                    " has no original"
                )
            );
            // SAFETY: the tuple's replacee is the original implementation
            // of the interposed function, of type $ty.
            unsafe { std::mem::transmute::<*const c_void, $ty>(p) }
        }
    };
}

// Darwin arm64 passes VARIADIC arguments (open/openat's `mode`) ON THE
// STACK (first variadic slot at [sp, 0]) — a fixed-parameter Rust shim
// reads the wrong register and forwards garbage mode (Temurin's NIO
// file creations landed mode 0000). The trampolines hoist the
// stack-passed mode into the fixed-parameter register and tail-branch
// to the Rust body. x86_64 Darwin passes it in the register, so the
// tuples bind the Rust fn directly there.
#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".text",
    ".p2align 2",
    ".globl _tebako_tramp_open",
    "_tebako_tramp_open:",
    "    ldr w2, [sp]",
    "    b _open_impl",
    ".globl _tebako_tramp_openat",
    "_tebako_tramp_openat:",
    "    ldr w3, [sp]",
    "    b _openat_impl",
    ".globl _tebako_tramp_fcntl",
    "_tebako_tramp_fcntl:",
    "    ldr w2, [sp]",
    "    b _fcntl_impl",
);

#[cfg(target_arch = "aarch64")]
unsafe extern "C" {
    fn tebako_tramp_open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn tebako_tramp_openat(dirfd: c_int, path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn tebako_tramp_fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
}

#[cfg(not(target_arch = "aarch64"))]
use {super::fcntl as shim_fcntl, super::open as shim_open, super::openat as shim_openat};
#[cfg(target_arch = "aarch64")]
use {tebako_tramp_fcntl as shim_fcntl, tebako_tramp_open as shim_open, tebako_tramp_openat as shim_openat};

interpose!(
    INTERPOSE_OPEN,
    real_open,
    shim_open,
    libc::open,
    unsafe extern "C" fn(*const c_char, c_int, ...) -> c_int
);
interpose!(
    INTERPOSE_OPENAT,
    real_openat,
    shim_openat,
    libc::openat,
    unsafe extern "C" fn(c_int, *const c_char, c_int, ...) -> c_int
);
interpose!(
    INTERPOSE_STAT,
    real_stat,
    super::stat,
    libc::stat,
    unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int
);
interpose!(
    INTERPOSE_LSTAT,
    real_lstat,
    super::lstat,
    libc::lstat,
    unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int
);
interpose!(
    INTERPOSE_FSTAT,
    real_fstat,
    super::fstat,
    libc::fstat,
    unsafe extern "C" fn(c_int, *mut libc::stat) -> c_int
);
interpose!(
    INTERPOSE_ACCESS,
    real_access,
    super::access,
    libc::access,
    unsafe extern "C" fn(*const c_char, c_int) -> c_int
);
interpose!(
    INTERPOSE_FACCESSAT,
    real_faccessat,
    super::faccessat,
    faccessat,
    unsafe extern "C" fn(c_int, *const c_char, c_int, c_int) -> c_int
);
interpose!(
    INTERPOSE_OPENDIR,
    real_opendir,
    super::opendir,
    libc::opendir,
    unsafe extern "C" fn(*const c_char) -> *mut libc::DIR
);
interpose!(
    INTERPOSE_READDIR,
    real_readdir,
    super::readdir,
    libc::readdir,
    unsafe extern "C" fn(*mut libc::DIR) -> *mut libc::dirent
);
interpose!(
    INTERPOSE_CLOSEDIR,
    real_closedir,
    super::closedir,
    libc::closedir,
    unsafe extern "C" fn(*mut libc::DIR) -> c_int
);
interpose!(
    INTERPOSE_PREAD,
    real_pread,
    super::pread,
    libc::pread,
    unsafe extern "C" fn(c_int, *mut c_void, usize, libc::off_t) -> libc::ssize_t
);
interpose!(
    INTERPOSE_READ,
    real_read,
    super::read,
    libc::read,
    unsafe extern "C" fn(c_int, *mut c_void, usize) -> libc::ssize_t
);
interpose!(
    INTERPOSE_LSEEK,
    real_lseek,
    super::lseek,
    libc::lseek,
    unsafe extern "C" fn(c_int, libc::off_t, c_int) -> libc::off_t
);
interpose!(
    INTERPOSE_CLOSE,
    real_close,
    super::close,
    libc::close,
    unsafe extern "C" fn(c_int) -> c_int
);
// `close` vs `close$NOCANCEL` (x86_64): the libc crate maps `libc::close`
// to `close$NOCANCEL` on x86_64 darwin (libc's unix/mod.rs `link_name`),
// so the tuple above covers ONLY the NOCANCEL spelling there — while C
// binaries (the JVM's libjava/libjli/libzip among them) import PLAIN
// `close`, whose flagged-fd calls then fell through to the kernel
// (EBADF from FileDescriptor.close0 → LauncherHelper jar.error1 — the
// class-E macos-15-intel leg). Declare the plain spelling directly (the
// fopen$DARWIN_EXTSN pattern) so both route to the shim. arm64 needs no
// second tuple: there `libc::close` IS plain close.
#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    #[link_name = "close"]
    fn close_plain(fd: c_int) -> c_int;
}
#[cfg(target_arch = "x86_64")]
interpose!(
    INTERPOSE_CLOSE_PLAIN,
    real_close_plain,
    super::close,
    close_plain,
    unsafe extern "C" fn(c_int) -> c_int
);
// fcntl is variadic (a cmd-dependent third argument) — the open/openat
// trampoline note applies verbatim on arm64. No $NOCANCEL twin exists.
interpose!(
    INTERPOSE_FCNTL,
    real_fcntl,
    shim_fcntl,
    libc::fcntl,
    unsafe extern "C" fn(c_int, c_int, ...) -> c_int
);
interpose!(
    INTERPOSE_MKDIR,
    real_mkdir,
    super::mkdir,
    libc::mkdir,
    unsafe extern "C" fn(*const c_char, libc::mode_t) -> c_int
);
interpose!(
    INTERPOSE_UNLINK,
    real_unlink,
    super::unlink,
    libc::unlink,
    unsafe extern "C" fn(*const c_char) -> c_int
);
interpose!(
    INTERPOSE_RENAME,
    real_rename,
    super::rename,
    libc::rename,
    unsafe extern "C" fn(*const c_char, *const c_char) -> c_int
);
interpose!(
    INTERPOSE_DLOPEN,
    real_dlopen,
    super::dlopen,
    libc::dlopen,
    unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void
);
// `fopen$DARWIN_EXTSN`: the 64-bit SDK's stdio export — no libc crate
// binding, declared here so its address (the REAL function) can seed the
// tuple the way libc::* addresses do (the JVM imports it directly).
unsafe extern "C" {
    #[link_name = "fopen$DARWIN_EXTSN"]
    fn fopen_extsn(path: *const c_char, mode: *const c_char) -> *mut libc::FILE;
}
interpose!(
    INTERPOSE_FOPEN,
    real_fopen,
    super::fopen,
    libc::fopen,
    unsafe extern "C" fn(*const c_char, *const c_char) -> *mut libc::FILE
);
interpose!(
    INTERPOSE_FOPEN_EXTSN,
    real_fopen_extsn,
    super::fopen_darwin_extsn,
    fopen_extsn,
    unsafe extern "C" fn(*const c_char, *const c_char) -> *mut libc::FILE
);
interpose!(
    INTERPOSE_FSTATAT,
    real_fstatat,
    super::fstatat,
    fstatat,
    unsafe extern "C" fn(c_int, *const c_char, *mut libc::stat, c_int) -> c_int
);
interpose!(
    INTERPOSE_REWINDDIR,
    real_rewinddir,
    super::rewinddir,
    libc::rewinddir,
    unsafe extern "C" fn(*mut libc::DIR)
);
interpose!(
    INTERPOSE_DIRFD,
    real_dirfd,
    super::dirfd,
    libc::dirfd,
    unsafe extern "C" fn(*mut libc::DIR) -> c_int
);
interpose!(
    INTERPOSE_TELLDIR,
    real_telldir,
    super::telldir,
    libc::telldir,
    unsafe extern "C" fn(*mut libc::DIR) -> libc::c_long
);
interpose!(
    INTERPOSE_SEEKDIR,
    real_seekdir,
    super::seekdir,
    libc::seekdir,
    unsafe extern "C" fn(*mut libc::DIR, libc::c_long)
);
interpose!(
    INTERPOSE_READDIR_R,
    real_readdir_r,
    super::readdir_r,
    libc::readdir_r,
    unsafe extern "C" fn(*mut libc::DIR, *mut libc::dirent, *mut *mut libc::dirent) -> c_int
);
interpose!(
    INTERPOSE_EXECVE,
    real_execve,
    super::execve,
    libc::execve,
    unsafe extern "C" fn(*const c_char, *const *mut c_char, *const *mut c_char) -> c_int
);
interpose!(
    INTERPOSE_POSIX_SPAWN,
    real_posix_spawn,
    super::posix_spawn,
    libc::posix_spawn,
    unsafe extern "C" fn(
        *mut libc::pid_t,
        *const c_char,
        *const libc::posix_spawn_file_actions_t,
        *const libc::posix_spawnattr_t,
        *const *mut c_char,
        *const *mut c_char,
    ) -> c_int
);
interpose!(
    INTERPOSE_POSIX_SPAWNP,
    real_posix_spawnp,
    super::posix_spawnp,
    libc::posix_spawnp,
    unsafe extern "C" fn(
        *mut libc::pid_t,
        *const c_char,
        *const libc::posix_spawn_file_actions_t,
        *const libc::posix_spawnattr_t,
        *const *mut c_char,
        *const *mut c_char,
    ) -> c_int
);
// realpath + realpath$DARWIN_EXTSN (spec 07 §8): xnu's realpath(3)
// walks the path with internal syscalls the interpose tuples never see —
// the same host-leak class as glibc (a payload path under a host symlink
// prefix, `/tmp`/`/etc`/`/var` on macOS, canonicalizes to the host
// spelling). The JDK's `File.getCanonicalFile` calls it through the PLT,
// which the tuple DOES redirect; the VFS answer lives in the shim body.
interpose!(
    INTERPOSE_REALPATH,
    real_realpath,
    super::realpath,
    libc::realpath,
    unsafe extern "C" fn(*const c_char, *mut c_char) -> *mut c_char
);
// `realpath$DARWIN_EXTSN`: the SDK's stdlib.h maps `realpath` here under
// _DARWIN_C_FULL_SOURCE (the default) — no libc crate binding, declared
// so its address can seed the tuple (the fopen$DARWIN_EXTSN pattern).
unsafe extern "C" {
    #[link_name = "realpath$DARWIN_EXTSN"]
    fn realpath_extsn(path: *const c_char, resolved: *mut c_char) -> *mut c_char;
}
interpose!(
    INTERPOSE_REALPATH_EXTSN,
    real_realpath_extsn,
    super::realpath_darwin_extsn,
    realpath_extsn,
    unsafe extern "C" fn(*const c_char, *mut c_char) -> *mut c_char
);

/// Library constructor: establish the namespace before the program's main.
#[used]
#[link_section = "__DATA,__mod_init_func"]
static INIT: extern "C" fn() = {
    extern "C" fn init() {
        super::init();
    }
    init
};
