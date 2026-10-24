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
    /// The kernel's per-process path answer (`libproc.h`): the channel
    /// that stays honest when the interpose readback does not (tebako#920).
    fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
}

/// The tuple alone, for a symbol whose original is resolved by hand. The
/// generated readback is right for every replacement that delegates
/// straight through; `getcwd` needs more, because dyld has been observed
/// handing ITS tuple the replacement rather than the original (tebako#920).
macro_rules! interpose_tuple {
    ($name:ident, $replacement:expr, $replacee:expr) => {
        #[used]
        #[link_section = "__DATA,__interpose"]
        static $name: InterposeTuple = InterposeTuple {
            replacement: $replacement as *const c_void,
            replacee: $replacee as *const c_void,
        };
    };
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
use {
    tebako_tramp_fcntl as shim_fcntl, tebako_tramp_open as shim_open,
    tebako_tramp_openat as shim_openat,
};

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
// creat is NOT variadic (fixed path+mode) — no arm64 trampoline (the
// dup/dup2 note applies verbatim).
interpose!(
    INTERPOSE_CREAT,
    real_creat,
    super::creat,
    libc::creat,
    unsafe extern "C" fn(*const c_char, c_int) -> c_int
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
    INTERPOSE_READLINK,
    real_readlink,
    super::readlink,
    libc::readlink,
    unsafe extern "C" fn(*const c_char, *mut c_char, libc::size_t) -> libc::ssize_t
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
    INTERPOSE_CHDIR,
    real_chdir,
    super::chdir,
    libc::chdir,
    unsafe extern "C" fn(*const c_char) -> c_int
);
interpose!(
    INTERPOSE_FCHDIR,
    real_fchdir,
    super::fchdir,
    libc::fchdir,
    unsafe extern "C" fn(c_int) -> c_int
);
interpose_tuple!(INTERPOSE_GETCWD, super::getcwd, libc::getcwd);

/// `getcwd`'s original, resolved through the tuple and checked before it
/// is believed.
///
/// dyld has been observed handing this one tuple the REPLACEMENT (a
/// crash report with 506 frames of the same `libtfs_preload:getcwd`
/// image offset, reproduced from the library's own constructor before
/// any mount or policy existed — tebako#920). An address that is null or
/// is our own shim is not the original, and the kernel spelling is then
/// the only honest channel: `__getcwd` cannot re-enter the replacement.
pub(super) fn real_getcwd() -> unsafe extern "C" fn(*mut c_char, libc::size_t) -> *mut c_char {
    // SAFETY: the tuple is initialized statically and written by dyld
    // before any user code runs; volatile because dyld's write is
    // invisible to the compiler.
    let p = unsafe { std::ptr::read_volatile(std::ptr::addr_of!(INTERPOSE_GETCWD.replacee)) };
    if p.is_null() || std::ptr::eq(p, super::getcwd as *const c_void) {
        return kernel_getcwd;
    }
    // SAFETY: the tuple's replacee is the original getcwd, of this type.
    unsafe { std::mem::transmute::<*const c_void, _>(p) }
}

/// `proc_pidinfo`'s flavor for the process's vnode paths (`sys/proc_info.h`).
const PROC_PIDVNODEPATHINFO: c_int = 9;
/// `sizeof(struct proc_vnodepathinfo)` and the offset of
/// `pvi_cdir.vip_path` inside it, both read from the SDK header and
/// measured on macOS arm64 with `sizeof`/`offsetof` in C. A different
/// kernel layout is caught at runtime by the answered-size check below
/// rather than trusted.
const VNODEPATHINFO_SIZE: usize = 2352;
const VIP_PATH_OFFSET: usize = 152;
/// `MAXPATHLEN` on darwin: the `vip_path` field's capacity.
const VIP_PATH_CAPACITY: usize = 1024;

/// The kernel spelling: no tuple, no replacement, no recursion.
///
/// `proc_pidinfo(PROC_PIDVNODEPATHINFO)` asks the kernel for the process's
/// working directory directly, so it cannot re-enter the shim. The answer
/// is copied with `getcwd`'s own contract — `ERANGE` when the buffer is
/// too small, `ENOENT` when the directory has no path any more — never a
/// truncated path.
unsafe extern "C" fn kernel_getcwd(buf: *mut c_char, size: libc::size_t) -> *mut c_char {
    let mut info = [0u8; VNODEPATHINFO_SIZE];
    let asked = VNODEPATHINFO_SIZE;
    // SAFETY: the buffer is VNODEPATHINFO_SIZE bytes and the flavor fills
    // exactly that struct on success (the answered size proves it).
    let answered = unsafe {
        proc_pidinfo(
            libc::getpid(),
            PROC_PIDVNODEPATHINFO,
            0,
            info.as_mut_ptr().cast(),
            asked as c_int,
        )
    };
    if answered as usize != asked {
        super::set_errno(libc::ENOENT);
        return std::ptr::null_mut();
    }
    let field = &info[VIP_PATH_OFFSET..VIP_PATH_OFFSET + VIP_PATH_CAPACITY];
    let len = field.iter().position(|&b| b == 0).unwrap_or(VIP_PATH_CAPACITY);
    if len == 0 {
        super::set_errno(libc::ENOENT);
        return std::ptr::null_mut();
    }
    if size < len + 1 {
        super::set_errno(libc::ERANGE);
        return std::ptr::null_mut();
    }
    // SAFETY: the caller guarantees `buf` holds `size` bytes, and the
    // length check above proved len + 1 of them fit.
    unsafe {
        std::ptr::copy_nonoverlapping(field.as_ptr().cast::<c_char>(), buf, len);
        *buf.add(len) = 0;
    }
    buf
}
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
// dup/dup2 are NOT variadic — no arm64 trampoline needed (unlike
// open/openat/fcntl): the tuples bind the Rust bodies directly on both
// Darwin arches.
interpose!(
    INTERPOSE_DUP,
    real_dup,
    super::dup,
    libc::dup,
    unsafe extern "C" fn(c_int) -> c_int
);
interpose!(
    INTERPOSE_DUP2,
    real_dup2,
    super::dup2,
    libc::dup2,
    unsafe extern "C" fn(c_int, c_int) -> c_int
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
// The SAME split exists in the other direction on arm64: there
// `libc::close` is plain `close`, and the NOCANCEL spelling is a separate
// import that libc never names — so node's libuv (`uv__close_nocancel`,
// used by every internal file operation) reached the KERNEL with a
// flagged fd and got EBADF, which node turns into
// `Assertion failed: (0) == (uv_fs_close(...))` and an abort. The
// observable was a `readFileSync` of an in-image file dying at close.
// Declare the NOCANCEL spelling here so both arches route both spellings
// to the shim: on x86_64 the main tuple already covers NOCANCEL and the
// plain twin above covers C binaries; on arm64 the main tuple covers
// plain and this twin covers NOCANCEL.
#[cfg(target_arch = "aarch64")]
unsafe extern "C" {
    #[link_name = "close$NOCANCEL"]
    fn close_nocancel(fd: c_int) -> c_int;
}
#[cfg(target_arch = "aarch64")]
interpose!(
    INTERPOSE_CLOSE_NOCANCEL,
    real_close_nocancel,
    super::close,
    close_nocancel,
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
