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
//! ## The early-boot rule (tebako#527)
//!
//! Interposed calls can arrive BEFORE this library's constructor runs:
//! an earlier-initialized dependency's constructor allocates (libstdc++'s
//! does), and with a `--export-dynamic` exe carrying a STATIC jemalloc
//! (the spec 29 link unit) that allocation enters jemalloc's lazy
//! `malloc_init_hard`, which holds the NON-recursive `init_lock` while
//! its own setup performs syscalls — the arena-base `pages_map` mmap,
//! and `pages_boot`'s THP probe `open`/`read` of
//! `/sys/kernel/mm/transparent_hugepage/*`. Those syscalls re-enter THIS
//! shim. Two links then each self-deadlock the single thread:
//!
//! 1. the engine path allocates (path normalization's Vec, the lazy
//!    mount pass) — re-entering jemalloc under `init_lock`;
//! 2. the host passthrough's lazy `dlsym(RTLD_NEXT, …)` ALLOCATES too
//!    (glibc's `_dl_find_object` growth / dlerror buffer) — same lock.
//!
//! So, on 64-bit linux: until the constructor's mount pass completes
//! (`BOOT_LIVE`, sys/mod.rs), the engine is barred (`engine_call`
//! answers None — every shim's "pass through" arm) and the thin-syscall
//! bodies answer from the RAW SYSCALL layer below — no engine, no
//! dlsym, no allocation. A pre-constructor call is definitionally
//! loader/allocator host IO: the VFS mounts exist only after the
//! constructor, so the raw host answer is the truthful one. The mm
//! family goes further and NEVER dlsyms (always raw): mmap is the
//! allocator's own primitive, called on every era's path including the
//! engine's own anonymous fills. glibc's `syscall(2)` wrapper sets errno
//! from the kernel's -errno return itself, and on 64-bit linux the
//! at-family syscalls carry byte offsets with the kernel's stat layout
//! equal to glibc's — the raw arms are byte-identical passthroughs
//! (aarch64 has no SYS_open/SYS_stat at all; the at-forms are what
//! glibc's wrappers call). The libc-composite surface (the DIR*/FILE*
//! families, realpath, dlopen, posix_spawn) keeps the lazy-dlsym
//! resolution: no allocator init builds a DIR*, and a library
//! constructor calling one BEFORE the preload's init entry is not
//! observed and is absurd by construction (documented residual risk).
//! 32-bit linux keeps the historical resolution everywhere (old_mmap's
//! arg-block ABI and mmap2's page-granular offset make the raw form
//! non-portable; no shipped runtime is 32-bit). musl and macOS never
//! wedged — musl's dlsym and macOS's dyld interpose never allocate — but
//! musl shares the raw arms (same law, same code); macOS takes the gate
//! only (its `real_*` come from interpose tuples, already dlsym-free).
//!
//! Coverage note: binaries built against glibc ≥ 2.33 call
//! `stat`/`lstat`/`fstat`/`fstatat` directly and are covered; binaries
//! built against older glibc use the versioned `__xstat`/`__lxstat`/
//! `__fxstat`/`__fxstatat` entry points, which are interposed as well
//! (roadmap 39). The LFS `open64`/`stat64`/`lstat64`/`fstat64`/`pread64`/
//! `lseek64` family is interposed too — Rust std and `_FILE_OFFSET_BITS=64`
//! builds call the 64 variants directly, and they are DISTINCT exported
//! symbols from the plain names (the JDK launcher maps `JLI_Lseek` to
//! `lseek64` on glibc — spec 22 class E). `mmap`/`mmap64` are interposed
//! for flagged memfs fds and served as private anonymous mappings
//! pre-filled from the VFS (the JDK's libzip mmaps a jar's central
//! directory at open — `USE_MMAP` is unconditional and `ZIP_Put_In_Cache`
//! passes `usemmap=TRUE`). The LFS `__xstat64`/`__lxstat64`/`__fxstat64`
//! versioned forms are interposed as delegations to
//! `__xstat`/`__lxstat`/`__fxstat` — on glibc the *64 versioned entries
//! are literally the same addresses (2.31 nm proof), the x86_64 layouts
//! are identical, and the plain `stat64`/`fstat64` dynamic symbols do
//! NOT exist before glibc 2.33, so the versioned entry is the only
//! resolvable host passthrough there (the JDK's libjava/libnio import
//! the *64 forms — spec 22 class E). The fortified `__read_chk` is
//! interposed too (the wrapper lives INSIDE libc and calls the syscall
//! stub directly, so an interposed `read` never sees a fortified caller
//! — the debian/temurin JDK's libjli imports exactly it for the jar
//! END-record read), and the fortified `__openat_2` (the three-arg
//! `openat`; its check is compile-time, so the runtime symbol is plain
//! openat — the 0.16.6 runtime's vendored C++ imports it, tebako#439).
//! `openat64` (Rust std's openat spelling under `_FILE_OFFSET_BITS=64`)
//! and `fopen64` (tebako#439: libcrypto's `crypto/o_fopen.c` defines
//! `_FILE_OFFSET_BITS=64` itself on linux, so `openssl_fopen` — the
//! `BIO_new_file` / `X509_LOOKUP_load_file` choke point — binds
//! `fopen64`, never `fopen`) are interposed as delegations to their
//! plain-name bodies, as is `__fxstatat64` (to `__fxstatat` — the
//! versioned stat family's last *64 form). The realpath family is
//! interposed too (`realpath`, `canonicalize_file_name`, and the fortified
//! `__realpath_chk` — spec 07 §8): glibc's realpath walks the path
//! with libc-INTERNAL stat/readlink aliases that no PLT interpose sees,
//! so a covered path answered by the host resolver leaked the
//! HOST-canonicalized spelling (usrmerge /lib → usr/lib) into the JDK's
//! URLClassPath — the 2026-09-03 dogfood linux-gnu jing ClassNotFound.
//! The shim answers covered paths with the mount spelling, lexically
//! normalized (the VFS is already canonical), and passes everything else
//! to the host. Remaining documented gaps:
//! the rest of the fortify open family (`__open_2`/`__open64_2`/
//! `__openat64_2` — no importer observed in the 0.16.6 linux-gnu
//! runtime), `readv`/`preadv`/`sendfile`/`copy_file_range`, the
//! write-side `pwrite64`/`ftruncate64`/`statvfs64` family on memfs fds,
//! and the internal-walk family `glob`/`nftw`/`ftw`/`wordexp` (the same
//! libc-internal-alias bypass class as realpath — UNAUDITED, no observed
//! importer; pinned debt). A
//! pre-existing landmine OUTSIDE the JDK path: the plain
//! `stat`/`lstat`/`fstat`/`stat64`/`lstat64`/`fstat64` host passthroughs
//! resolve dynamic symbols glibc only exports ≥ 2.33 — on an older
//! glibc a host call through them panics the resolver (nothing on the
//! JDK/ruby boot path takes them; both use the versioned entries).

// The real_* helpers mirror libc symbol names; the versioned ones carry
// double underscores (`__xstat` & co.), which are intentionally NOT
// snake_case. Allow that at the module level: per-item attributes on the
// macro invocations are flagged `unused_attributes` by rustc (attributes
// on macro calls do not propagate into the expansion).
#![allow(non_snake_case)]

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
// The mm family answers through the RAW SYSCALL, never dlsym — the
// module doc's tebako#527 paragraph has the full deadlock chain. glibc's
// syscall(2) wrapper sets errno from the kernel's -errno return itself,
// so these are byte-identical passthroughs on 64-bit linux (the shipped
// form; `mmap64` carries the same signature there). 32-bit linux keeps
// the dlsym resolution: old_mmap's arg-block ABI and mmap2's
// page-granular offset make the raw form non-portable, and no shipped
// runtime is 32-bit.
#[cfg(target_pointer_width = "64")]
pub(super) fn real_mmap(
) -> unsafe extern "C" fn(*mut c_void, usize, c_int, c_int, c_int, libc::off_t) -> *mut c_void {
    unsafe extern "C" fn via_syscall(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: libc::off_t,
    ) -> *mut c_void {
        unsafe { libc::syscall(libc::SYS_mmap, addr, len, prot, flags, fd, offset) as *mut c_void }
    }
    via_syscall
}
#[cfg(not(target_pointer_width = "64"))]
real_fn!(
    real_mmap,
    c"mmap",
    unsafe extern "C" fn(*mut c_void, usize, c_int, c_int, c_int, libc::off_t) -> *mut c_void
);
#[cfg(target_pointer_width = "64")]
pub(super) fn real_munmap() -> unsafe extern "C" fn(*mut c_void, usize) -> c_int {
    unsafe extern "C" fn via_syscall(addr: *mut c_void, len: usize) -> c_int {
        unsafe { libc::syscall(libc::SYS_munmap, addr, len) as c_int }
    }
    via_syscall
}
#[cfg(not(target_pointer_width = "64"))]
real_fn!(
    real_munmap,
    c"munmap",
    unsafe extern "C" fn(*mut c_void, usize) -> c_int
);
#[cfg(target_pointer_width = "64")]
pub(super) fn real_mprotect() -> unsafe extern "C" fn(*mut c_void, usize, c_int) -> c_int {
    unsafe extern "C" fn via_syscall(addr: *mut c_void, len: usize, prot: c_int) -> c_int {
        unsafe { libc::syscall(libc::SYS_mprotect, addr, len, prot) as c_int }
    }
    via_syscall
}
#[cfg(not(target_pointer_width = "64"))]
real_fn!(
    real_mprotect,
    c"mprotect",
    unsafe extern "C" fn(*mut c_void, usize, c_int) -> c_int
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
real_fn!(real_dup, c"dup", unsafe extern "C" fn(c_int) -> c_int);
real_fn!(
    real_dup2,
    c"dup2",
    unsafe extern "C" fn(c_int, c_int) -> c_int
);
real_fn!(
    real_fcntl,
    c"fcntl",
    unsafe extern "C" fn(c_int, c_int, ...) -> c_int
);
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
    real_fopen,
    c"fopen",
    unsafe extern "C" fn(*const c_char, *const c_char) -> *mut libc::FILE
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
#[cfg(target_pointer_width = "64")]
pub(super) fn real_statx() -> unsafe extern "C" fn(
    c_int,
    *const c_char,
    c_int,
    libc::c_uint,
    *mut super::statx_abi::statx,
) -> c_int {
    raw_statx
}

/// statx via SYS_statx (the resolver above's target, and the shim body's
/// early arm). The kernel uapi is one ABI on 64-bit linux and glibc's
/// wrapper IS the syscall; the kernel answers ENOSYS where it predates
/// statx(2) — the truthful passthrough, and inert where no caller can
/// name statx at all. (musl's wrapper arrived only in 1.2.4 — alpine >=
/// 3.19 — so RTLD_NEXT finds nothing on the 3.17 floor either way;
/// glibc's dlsym resolution ALLOCATES, the tebako#527 hazard this layer
/// removes.)
#[cfg(target_pointer_width = "64")]
pub(super) unsafe extern "C" fn raw_statx(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mask: libc::c_uint,
    stx: *mut super::statx_abi::statx,
) -> c_int {
    unsafe { libc::syscall(libc::SYS_statx, dirfd, path, flags, mask, stx) as c_int }
}
#[cfg(all(not(target_pointer_width = "64"), not(target_env = "musl")))]
real_fn!(
    real_statx,
    c"statx",
    unsafe extern "C" fn(
        c_int,
        *const c_char,
        c_int,
        libc::c_uint,
        *mut super::statx_abi::statx,
    ) -> c_int
);
#[cfg(all(not(target_pointer_width = "64"), target_env = "musl"))]
pub(super) fn real_statx() -> unsafe extern "C" fn(
    c_int,
    *const c_char,
    c_int,
    libc::c_uint,
    *mut super::statx_abi::statx,
) -> c_int {
    // 32-bit keeps the historical resolution; on musl the wrapper is
    // absent before 1.2.4, so the raw syscall answers there too.
    unsafe extern "C" fn via_syscall(
        dirfd: c_int,
        path: *const c_char,
        flags: c_int,
        mask: libc::c_uint,
        stx: *mut super::statx_abi::statx,
    ) -> c_int {
        unsafe { libc::syscall(libc::SYS_statx, dirfd, path, flags, mask, stx) as c_int }
    }
    via_syscall
}
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
// _FORTIFY_SOURCE=2 read: the check wrapper lives INSIDE libc and calls
// the syscall stub directly, so interposing `read` alone never catches
// it (the spec 22 class-E JDK's libjli imports `__read_chk`).
real_fn!(
    real___read_chk,
    c"__read_chk",
    unsafe extern "C" fn(c_int, *mut c_void, usize, usize) -> libc::ssize_t
);
real_fn!(real___chk_fail, c"__chk_fail", unsafe extern "C" fn() -> !);
// realpath(3) itself: the JDK's libjava canonicalization calls it through
// the PLT (the spec 07 §8 dogfood jing ClassNotFound) — the walk
// INSIDE glibc is unreachable, so the shim answers VFS paths itself and
// forwards host paths here.
real_fn!(
    real_realpath,
    c"realpath",
    unsafe extern "C" fn(*const c_char, *mut c_char) -> *mut c_char
);
// The always-allocating realpath spelling — the NULL-buffer pass-through
// arm's target on glibc. A plain `dlsym(RTLD_NEXT, "realpath")` is NOT safe
// for that arm: glibc carries a versioned compat twin `realpath@GLIBC_2.0`
// (`__old_realpath`, stdlib/canonicalize.c) that EINVALs a NULL buffer,
// and the default-version lookup lands on it at least on glibc 2.31
// (ubuntu-20.04 — the 0.16.19 factory re-cut's linux-gnu x86_64 boot
// smoke died there: Rust std's canonicalize always rides the NULL arm,
// so the jail bind's canonicalize of every grant path returned EINVAL,
// 2026-09-02). `canonicalize_file_name` entered glibc at 2.3 with no
// versioned history, so the resolution is unambiguous. glibc-ONLY:
// musl has no `canonicalize_file_name` at all (verified: zero symbols
// matching 'canonicalize' in musl 1.2.5's dynamic table, alpine
// 3.21.7 — the 0.16.19 musl boot smoke aborted on this very lookup,
// 2026-09-02) and needs no reroute anyway: with no symbol versioning
// there is no compat twin, so musl's plain realpath dlsym is already
// unambiguous.
#[cfg(target_env = "gnu")]
real_fn!(
    real_canonicalize_file_name,
    c"canonicalize_file_name",
    unsafe extern "C" fn(*const c_char) -> *mut c_char
);
real_fn!(
    real_rewinddir,
    c"rewinddir",
    unsafe extern "C" fn(*mut libc::DIR)
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

// ---------------------------------------------------------------------
// The raw syscall layer (the module doc's early-boot rule, tebako#527).
// Each fn is the byte-identical passthrough of glibc's thin wrapper on
// 64-bit linux: `syscall(2)` sets errno from the kernel's -errno return,
// the at-family forms are what glibc's wrappers call (asm-generic —
// aarch64 — has no SYS_open/SYS_stat/SYS_access/SYS_mkdir/SYS_unlink/
// SYS_rename at all), and the kernel's stat layout IS glibc's on every
// shipped 64-bit target. 64-bit only: old_mmap's arg-block ABI, mmap2's
// page-granular offset, and the 32-bit stat/fcntl64 splits make the raw
// form non-portable there — and no shipped runtime is 32-bit. The shim
// bodies call these from their `boot_arm!` early arm; they are never on
// the post-constructor path (the dlsym'd real_* answer there).
// ---------------------------------------------------------------------

/// `open` via SYS_openat(AT_FDCWD, …).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_open(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    unsafe { libc::syscall(libc::SYS_openat, libc::AT_FDCWD, path, flags, mode) as c_int }
}

/// `openat` via SYS_openat.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_openat(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mode: c_int,
) -> c_int {
    unsafe { libc::syscall(libc::SYS_openat, dirfd, path, flags, mode) as c_int }
}

/// `read` via SYS_read.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_read(fd: c_int, buf: *mut c_void, nbyte: usize) -> libc::ssize_t {
    unsafe { libc::syscall(libc::SYS_read, fd, buf, nbyte) as libc::ssize_t }
}

/// `__read_chk`: the fortify contract (a request larger than the
/// compiler-known buffer aborts — glibc's `__chk_fail` prints a note
/// first, cosmetic-only) over the raw read. abort(3) raises SIGABRT
/// without allocating, safe inside the allocator's own init.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw___read_chk(
    fd: c_int,
    buf: *mut c_void,
    nbyte: usize,
    buflen: usize,
) -> libc::ssize_t {
    if nbyte > buflen {
        // SAFETY: plain libc call; never returns.
        unsafe { libc::abort() }
    }
    unsafe { raw_read(fd, buf, nbyte) }
}

/// `pread` via SYS_pread64 (the only pread syscall on 64-bit).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_pread(
    fd: c_int,
    buf: *mut c_void,
    nbyte: usize,
    offset: libc::off_t,
) -> libc::ssize_t {
    unsafe { libc::syscall(libc::SYS_pread64, fd, buf, nbyte, offset) as libc::ssize_t }
}

/// `lseek` via SYS_lseek (64-bit off_t on both shipped arches).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_lseek(fd: c_int, offset: libc::off_t, whence: c_int) -> libc::off_t {
    unsafe { libc::syscall(libc::SYS_lseek, fd, offset, whence) as libc::off_t }
}

/// `close` via SYS_close.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_close(fd: c_int) -> c_int {
    unsafe { libc::syscall(libc::SYS_close, fd) as c_int }
}

/// `dup` via SYS_dup.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_dup(fd: c_int) -> c_int {
    unsafe { libc::syscall(libc::SYS_dup, fd) as c_int }
}

/// `dup2` via SYS_dup2.
#[cfg(all(target_pointer_width = "64", not(target_arch = "aarch64")))]
pub(super) unsafe fn raw_dup2(old: c_int, new: c_int) -> c_int {
    unsafe { libc::syscall(libc::SYS_dup2, old, new) as c_int }
}

/// `dup2` via SYS_dup3 on aarch64-linux: the arm64 kernel never wired the
/// dup2 syscall — libc spells `dup2` as `dup3(old, new, 0)`. `old == new`
/// keeps the dup2 no-op semantics (a validity probe); dup3 answers EINVAL.
#[cfg(all(target_pointer_width = "64", target_arch = "aarch64"))]
pub(super) unsafe fn raw_dup2(old: c_int, new: c_int) -> c_int {
    if old == new {
        return unsafe {
            if libc::syscall(libc::SYS_fcntl, old, libc::F_GETFD) >= 0 {
                new
            } else {
                -1
            }
        };
    }
    unsafe { libc::syscall(libc::SYS_dup3, old, new, 0) as c_int }
}

/// `fcntl` via SYS_fcntl (64-bit fcntl == fcntl64). Forwards the shim's
/// fixed third argument exactly as the dlsym'd passthrough does.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int {
    unsafe { libc::syscall(libc::SYS_fcntl, fd, cmd, arg) as c_int }
}

/// `mkdir` via SYS_mkdirat(AT_FDCWD, …).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_mkdir(path: *const c_char, mode: libc::mode_t) -> c_int {
    unsafe { libc::syscall(libc::SYS_mkdirat, libc::AT_FDCWD, path, mode) as c_int }
}

/// `unlink` via SYS_unlinkat(AT_FDCWD, …, 0) (no flags — a directory
/// unlink (AT_REMOVEDIR) is rmdir(2), a different name).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_unlink(path: *const c_char) -> c_int {
    unsafe { libc::syscall(libc::SYS_unlinkat, libc::AT_FDCWD, path, 0) as c_int }
}

/// `rename` via SYS_renameat(AT_FDCWD, …, AT_FDCWD, …).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_rename(old: *const c_char, new: *const c_char) -> c_int {
    unsafe { libc::syscall(libc::SYS_renameat, libc::AT_FDCWD, old, libc::AT_FDCWD, new) as c_int }
}

/// `stat` via SYS_newfstatat(AT_FDCWD, …, 0) — follows symlinks, as
/// stat(2) does.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_stat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { libc::syscall(libc::SYS_newfstatat, libc::AT_FDCWD, path, st, 0) as c_int }
}

/// `lstat` via SYS_newfstatat(…, AT_SYMLINK_NOFOLLOW).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_lstat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe {
        libc::syscall(
            libc::SYS_newfstatat,
            libc::AT_FDCWD,
            path,
            st,
            libc::AT_SYMLINK_NOFOLLOW,
        ) as c_int
    }
}

/// `fstat` via SYS_fstat.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_fstat(fd: c_int, st: *mut libc::stat) -> c_int {
    unsafe { libc::syscall(libc::SYS_fstat, fd, st) as c_int }
}

/// `fstatat` via SYS_newfstatat. The versioned `__xstat`/`__lxstat`/
/// `__fxstat(at)`(+64) entries share these arms — the ver argument is a
/// no-op where the kernel struct IS the glibc struct (the *64 delegation
/// rationale above), and the LFS `stat64`/`fstat64`/`fstatat64` spellings
/// are layout-identical on 64-bit.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_fstatat(
    dirfd: c_int,
    path: *const c_char,
    st: *mut libc::stat,
    flags: c_int,
) -> c_int {
    unsafe { libc::syscall(libc::SYS_newfstatat, dirfd, path, st, flags) as c_int }
}

/// `access` via SYS_faccessat(AT_FDCWD, …) — the 3-arg (flags-less)
/// form, whose real-id semantics are access(2)'s exactly (glibc's access
/// wrapper makes the same call).
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_access(path: *const c_char, mode: c_int) -> c_int {
    unsafe { libc::syscall(libc::SYS_faccessat, libc::AT_FDCWD, path, mode) as c_int }
}

/// `faccessat` with flags == 0 via SYS_faccessat. Callers carrying flags
/// (AT_EACCESS/AT_SYMLINK_NOFOLLOW) fall through to the dlsym'd wrapper —
/// the pre-5.8-kernel emulation glibc performs for them is not ours to
/// re-create, and no early-boot caller passes flags.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_faccessat(dirfd: c_int, path: *const c_char, mode: c_int) -> c_int {
    unsafe { libc::syscall(libc::SYS_faccessat, dirfd, path, mode) as c_int }
}

/// `getdents64` via SYS_getdents64.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_getdents64(fd: c_int, dirp: *mut c_void, count: usize) -> libc::ssize_t {
    unsafe { libc::syscall(libc::SYS_getdents64, fd, dirp, count) as libc::ssize_t }
}

/// `execve` via SYS_execve.
#[cfg(target_pointer_width = "64")]
pub(super) unsafe fn raw_execve(
    path: *const c_char,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    unsafe { libc::syscall(libc::SYS_execve, path, argv, envp) as c_int }
}

/// Library constructor: establish the namespace before the program's main.
#[used]
#[link_section = ".init_array"]
static INIT: extern "C" fn() = {
    extern "C" fn init() {
        super::init();
    }
    init
};
