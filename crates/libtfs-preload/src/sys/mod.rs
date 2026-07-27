//! The interposition layer — the ONLY module with `unsafe` in the crate.
//!
//! Delivery (spec 07 §8 tier 1):
//!
//! - **macOS** (`sys/macos.rs`): `__DATA,__interpose` tuples pointing at
//!   the shim functions below; injected with `DYLD_INSERT_LIBRARIES`. The
//!   original implementations are read back from the tuples (dyld
//!   interpose redirects even `dlsym(RTLD_NEXT)` results to the
//!   replacement — the tuple's replacee is the only preserved channel).
//! - **linux** (`sys/linux.rs`): the shim functions are exported under
//!   their libc names (`#[no_mangle]`); injected with `LD_PRELOAD`. The
//!   originals are resolved ONCE per symbol with `dlsym(RTLD_NEXT, …)` —
//!   the preloaded library precedes libc in the global scope, so the next
//!   match is always the original.
//!
//! Both platform modules expose the originals as `real_<name>() -> fn`;
//! the shim bodies below are platform-independent.
//!
//! Re-entrancy: the engine itself performs host IO (the mount family's
//! image read, `dlmap2file`'s host-cache extraction). Those calls re-enter
//! the interposed symbols (the shim's own references bind to the shim —
//! ELF interposition / dyld interpose are process-wide). `IN_ENGINE` marks
//! the thread as inside the engine; re-entrant calls pass straight through
//! to the real implementation without touching the engine (correct: engine
//! IO is host IO by definition, and the context lock would otherwise
//! deadlock).

use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_long, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::Mutex;

use tfs::context::TebakoCDirent;

use crate::route::{self, PathRoute};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
use linux as plat;
#[cfg(target_os = "macos")]
use macos as plat;

// ---------------------------------------------------------------------
// errno plumbing
// ---------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn set_errno(e: c_int) {
    // SAFETY: __error() returns the thread's errno cell.
    unsafe { *libc::__error() = e };
}

#[cfg(target_os = "linux")]
fn set_errno(e: c_int) {
    // SAFETY: __errno_location() returns the thread's errno cell.
    unsafe { *libc::__errno_location() = e };
}

// ---------------------------------------------------------------------
// Re-entrancy guard (see the module docs)
// ---------------------------------------------------------------------

thread_local! {
    static IN_ENGINE: Cell<bool> = const { Cell::new(false) };
}

/// Run `f` (a route-layer engine call) unless this thread is already
/// inside the engine: re-entrant calls (the engine's own host IO) get
/// `None` and the shim passes them straight to the real implementation.
fn engine_call<T>(f: impl FnOnce() -> T) -> Option<T> {
    IN_ENGINE.with(|c| {
        if c.get() {
            return None;
        }
        c.set(true);
        let out = f();
        c.set(false);
        Some(out)
    })
}

// ---------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------

/// Borrowed C path argument → &str. `None` (NULL or non-UTF-8) always
/// passes through to the real call verbatim: the kernel/libc answers for
/// NULL itself, and a non-UTF-8 path can never be memfs (the engine
/// rejects non-UTF-8 paths with EINVAL, so they are host paths by
/// construction — v1 limitation, documented in the crate docs).
///
/// # Safety
/// `ptr` must be NULL or point to a valid NUL-terminated string that
/// outlives the call (the intercepted-call contract).
unsafe fn c_path<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: per the caller contract above.
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str().ok()
}

/// The dirfd's own path for openat/faccessat routing (None when
/// unresolvable — the route layer then keeps the relative path, which a
/// deny policy fails closed on).
#[cfg(target_os = "linux")]
fn resolve_dirfd(dirfd: c_int) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/self/fd/{dirfd}")).ok()
}

/// macOS: fcntl(F_GETPATH). PATH_MAX is 1024 on macOS.
#[cfg(target_os = "macos")]
fn resolve_dirfd(dirfd: c_int) -> Option<PathBuf> {
    let mut buf = vec![0u8; 1025];
    // SAFETY: buf is writable for 1025 bytes; F_GETPATH writes a
    // NUL-terminated path of at most 1024 bytes into it.
    let rc = unsafe { libc::fcntl(dirfd, libc::F_GETPATH, buf.as_mut_ptr().cast::<c_char>()) };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0)?;
    Some(PathBuf::from(
        String::from_utf8_lossy(&buf[..end]).into_owned(),
    ))
}

/// The dirfd base for *at routing (openat/faccessat/fstatat/statx/
/// openat2): absolute paths, AT_FDCWD-relative paths, and memfs dirfds
/// need no host base (`resolve_at` answers for them itself). AT_FDCWD
/// (-100) carries the TEBAKO_FD_FLAG bit — the explicit AT_FDCWD arm
/// MUST precede any `is_memfs_fd` test, and the memfs-fd test is gated
/// on `dirfd >= 0` regardless (the 4.0 lesson; pinned in route.rs).
fn at_base(dirfd: c_int, p: &str) -> Option<PathBuf> {
    if p.starts_with('/') || dirfd == libc::AT_FDCWD || (dirfd >= 0 && route::is_memfs_fd(dirfd)) {
        None
    } else {
        resolve_dirfd(dirfd)
    }
}

/// Route a *at path through `resolve_at` and run `vfs` on the routed
/// path. `Ok(None)` means pass the original call through to the host.
fn route_at<T>(
    dirfd: c_int,
    p: &str,
    vfs: impl FnOnce(&str) -> PathRoute<T>,
) -> Result<Option<PathRoute<T>>, c_int> {
    let routed = match route::resolve_at(dirfd, p, at_base(dirfd, p)) {
        Ok(rp) => rp,
        Err(e) => return Err(e),
    };
    Ok(engine_call(|| vfs(&routed)))
}

// ---------------------------------------------------------------------
// ABI translations (stat + dirent)
// ---------------------------------------------------------------------

/// Fill a native `struct stat` from the engine's RawStat — byte-for-byte
/// the engine's C-ABI fill (zeroed first; type bits + perms, size, mtime,
/// nlink 1).
// The S_IF* constant widths differ per platform (u16 on macOS, u32 on
// Linux): the widening `as u32` is required on macOS and an identity cast
// on Linux — mirrored from tfs's c_api fill_stat.
#[allow(clippy::unnecessary_cast)]
unsafe fn fill_stat(raw: &tfs::backend::RawStat) -> Result<libc::stat, i32> {
    use tfs::backend::EntryType;
    // SAFETY: a zeroed struct stat is a valid stat (the engine's C ABI
    // does exactly this).
    let mut out: libc::stat = unsafe { std::mem::zeroed() };
    let type_bits: u32 = match raw.entry_type {
        EntryType::File => libc::S_IFREG as u32,
        EntryType::Directory => libc::S_IFDIR as u32,
        _ => return Err(libc::EINVAL),
    };
    out.st_mode = (type_bits | raw.perms) as libc::mode_t;
    out.st_size = raw.size as libc::off_t;
    out.st_mtime = raw.mtime as libc::time_t;
    out.st_nlink = 1 as _;
    Ok(out)
}

/// Per-memfs-handle storage for the dirent readdir returns (POSIX: valid
/// until the next readdir/closedir on the same stream).
static DIRENT_CACHE: Mutex<Option<HashMap<usize, Box<libc::dirent>>>> = Mutex::new(None);

/// Translate a TebakoCDirent into the native `struct dirent` (name
/// NUL-terminated, d_type, d_namlen/d_reclen per platform, nonzero d_ino —
/// some consumers skip zero-inode entries).
fn fill_dirent(slot: &mut libc::dirent, entry: &TebakoCDirent) {
    // SAFETY: a zeroed struct dirent is valid; fields are filled below.
    *slot = unsafe { std::mem::zeroed() };
    let name_end = entry
        .d_name
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(entry.d_name.len());
    let name = &entry.d_name[..name_end];
    let len = name.len();
    let reclen = (std::mem::offset_of!(libc::dirent, d_name) + len + 1 + 7) & !7;
    #[cfg(target_os = "macos")]
    {
        slot.d_ino = 1;
        slot.d_seekoff = 0;
        slot.d_reclen = reclen as u16;
        slot.d_namlen = len as u16;
    }
    #[cfg(target_os = "linux")]
    {
        slot.d_ino = 1;
        slot.d_off = 0;
        slot.d_reclen = reclen as u16;
    }
    slot.d_type = entry.d_type;
    slot.d_name[..len].copy_from_slice(name);
    slot.d_name[len] = 0;
}

/// readdir on a memfs handle (shared by the readdir/readdir64 exports).
fn readdir_memfs(id: usize) -> *mut libc::dirent {
    match engine_call(|| route::vfs_readdir(id)) {
        Some(Ok(Some(entry))) => {
            let mut cache = DIRENT_CACHE.lock().unwrap();
            let map = cache.get_or_insert_with(HashMap::new);
            let slot = map
                .entry(id)
                // SAFETY: a zeroed struct dirent is valid (filled below).
                .or_insert_with(|| Box::new(unsafe { std::mem::zeroed() }));
            fill_dirent(slot, &entry);
            slot.as_mut() as *mut libc::dirent
        }
        // End of directory: NULL with errno untouched (glibc semantics).
        Some(Ok(None)) => std::ptr::null_mut(),
        Some(Err(e)) => {
            set_errno(e);
            std::ptr::null_mut()
        }
        // Re-entrant call during engine IO cannot happen for readdir (the
        // engine never reads host directories under the context lock in
        // the shim's RO configuration); fail safe.
        None => {
            set_errno(libc::EIO);
            std::ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------
// The interposed functions
//
// Each one: parse the C arguments, ask the route layer, then either serve
// from the VFS, fail with the route's errno, or call the real libc
// implementation with the ORIGINAL arguments.
// ---------------------------------------------------------------------

/// Interposed `open`. Declared with a fixed third `mode` parameter (the
/// varargs promotion is identical on the supported ABIs; `mode` is
/// forwarded verbatim and read by the callee only under O_CREAT).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        // SAFETY: forwarding the original arguments.
        return unsafe { plat::real_open()(path, flags, mode) };
    };
    match engine_call(|| route::vfs_open(p, flags)) {
        Some(PathRoute::Vfs(fd)) => fd,
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            -1
        }
        Some(PathRoute::Host) | None => {
            // SAFETY: forwarding the original arguments to the real open.
            unsafe { plat::real_open()(path, flags, mode) }
        }
    }
}

/// Interposed `openat` (see `open` for the `mode` note).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn openat(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mode: c_int,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_openat()(dirfd, path, flags, mode) };
    };
    match route_at(dirfd, p, |rp| route::vfs_open(rp, flags)) {
        Ok(Some(PathRoute::Vfs(fd))) => fd,
        Ok(Some(PathRoute::Denied(e))) | Err(e) => {
            set_errno(e);
            -1
        }
        Ok(Some(PathRoute::Host)) | Ok(None) => unsafe {
            plat::real_openat()(dirfd, path, flags, mode)
        },
    }
}

/// Serve `p` from the VFS into `st`. Returns `Some(rc)` when the call
/// was answered (serve or fail), `None` to pass the original call
/// through to the real libc implementation.
///
/// # Safety
/// `st` must be a valid `struct stat` pointer per the intercepted-call
/// contract.
unsafe fn vfs_stat_into(p: &str, st: *mut libc::stat) -> Option<c_int> {
    match engine_call(|| route::vfs_stat(p)) {
        // SAFETY: st is a valid struct stat pointer per the call contract.
        Some(PathRoute::Vfs(raw)) => match unsafe { fill_stat(&raw) } {
            Ok(s) => {
                unsafe { *st = s };
                Some(0)
            }
            Err(e) => {
                set_errno(e);
                Some(-1)
            }
        },
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            Some(-1)
        }
        Some(PathRoute::Host) | None => None,
    }
}

/// Shared body of stat/lstat/__xstat/__lxstat.
///
/// # Safety
/// `st` follows the intercepted-call contract; `real` runs the original
/// libc call with the original arguments.
unsafe fn stat_via(p: Option<&str>, st: *mut libc::stat, real: impl FnOnce() -> c_int) -> c_int {
    let Some(p) = p else {
        return real();
    };
    if st.is_null() {
        return real();
    }
    match unsafe { vfs_stat_into(p, st) } {
        Some(rc) => rc,
        None => real(),
    }
}

/// Interposed `stat`.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn stat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), st, || plat::real_stat()(path, st)) }
}

/// Interposed `lstat` (memfs has no symlink duality: lstat == stat).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn lstat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), st, || plat::real_lstat()(path, st)) }
}

/// fstat on a memfs fd (shared by fstat/__fxstat and the AT_EMPTY_PATH
/// arms of fstatat/statx).
///
/// # Safety
/// `st` follows the intercepted-call contract.
unsafe fn fstat_memfs(fd: c_int, st: *mut libc::stat) -> c_int {
    if st.is_null() {
        set_errno(libc::EFAULT);
        return -1;
    }
    match engine_call(|| route::vfs_fstat(fd)) {
        // SAFETY: st is valid per the call contract.
        Some(Ok(raw)) => match unsafe { fill_stat(&raw) } {
            Ok(s) => {
                unsafe { *st = s };
                0
            }
            Err(e) => {
                set_errno(e);
                -1
            }
        },
        Some(Err(e)) => {
            set_errno(e);
            -1
        }
        None => {
            set_errno(libc::EIO);
            -1
        }
    }
}

/// Interposed `fstat` (fd-flag dispatch, no path).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn fstat(fd: c_int, st: *mut libc::stat) -> c_int {
    if route::is_memfs_fd(fd) {
        return unsafe { fstat_memfs(fd, st) };
    }
    unsafe { plat::real_fstat()(fd, st) }
}

/// Interposed `access`.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn access(path: *const c_char, mode: c_int) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_access()(path, mode) };
    };
    match engine_call(|| route::vfs_access(p, mode)) {
        Some(PathRoute::Vfs(())) => 0,
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            -1
        }
        Some(PathRoute::Host) | None => unsafe { plat::real_access()(path, mode) },
    }
}

/// Interposed `faccessat`. The `flags` bits (AT_SYMLINK_NOFOLLOW,
/// AT_EACCESS) only refine the real call — the VFS answer is the same
/// (no symlink duality; effective == real ids in the delivery model).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn faccessat(
    dirfd: c_int,
    path: *const c_char,
    mode: c_int,
    flags: c_int,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_faccessat()(dirfd, path, mode, flags) };
    };
    match route_at(dirfd, p, |rp| route::vfs_access(rp, mode)) {
        Ok(Some(PathRoute::Vfs(()))) => 0,
        Ok(Some(PathRoute::Denied(e))) | Err(e) => {
            set_errno(e);
            -1
        }
        Ok(Some(PathRoute::Host)) | Ok(None) => unsafe {
            plat::real_faccessat()(dirfd, path, mode, flags)
        },
    }
}

/// Interposed `fstatat` (both platforms — the macOS *at stat call).
/// The `flags` bits (AT_SYMLINK_NOFOLLOW) only refine the real call —
/// the VFS answer is the same (no symlink duality). Linux' AT_EMPTY_PATH
/// with an empty path stats the dirfd itself (a memfs dirfd is fstat'd);
/// an empty path WITHOUT AT_EMPTY_PATH passes through (the kernel's
/// ENOENT is its own answer).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn fstatat(
    dirfd: c_int,
    path: *const c_char,
    st: *mut libc::stat,
    flags: c_int,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_fstatat()(dirfd, path, st, flags) };
    };
    if st.is_null() {
        return unsafe { plat::real_fstatat()(dirfd, path, st, flags) };
    }
    #[cfg(target_os = "linux")]
    if p.is_empty() {
        if (flags & libc::AT_EMPTY_PATH) != 0 && dirfd >= 0 && route::is_memfs_fd(dirfd) {
            return unsafe { fstat_memfs(dirfd, st) };
        }
        return unsafe { plat::real_fstatat()(dirfd, path, st, flags) };
    }
    #[cfg(target_os = "macos")]
    if p.is_empty() {
        return unsafe { plat::real_fstatat()(dirfd, path, st, flags) };
    }
    match route_at(dirfd, p, route::vfs_stat) {
        Ok(Some(PathRoute::Vfs(raw))) => match unsafe { fill_stat(&raw) } {
            // SAFETY: st is valid per the call contract.
            Ok(s) => {
                unsafe { *st = s };
                0
            }
            Err(e) => {
                set_errno(e);
                -1
            }
        },
        Ok(Some(PathRoute::Denied(e))) | Err(e) => {
            set_errno(e);
            -1
        }
        Ok(Some(PathRoute::Host)) | Ok(None) => unsafe {
            plat::real_fstatat()(dirfd, path, st, flags)
        },
    }
}

/// Fill a native `struct statx` from the engine's RawStat (the BASIC_STATS
/// fields the engine can honestly answer: type+perms, size, mtime, nlink).
#[cfg(target_os = "linux")]
unsafe fn fill_statx(raw: &tfs::backend::RawStat) -> Result<libc::statx, i32> {
    use tfs::backend::EntryType;
    // SAFETY: a zeroed struct statx is valid (as in fill_stat).
    let mut out: libc::statx = unsafe { std::mem::zeroed() };
    let type_bits: u16 = match raw.entry_type {
        EntryType::File => libc::S_IFREG as u16,
        EntryType::Directory => libc::S_IFDIR as u16,
        _ => return Err(libc::EINVAL),
    };
    out.stx_mask = libc::STATX_BASIC_STATS;
    out.stx_blksize = 4096;
    out.stx_nlink = 1;
    out.stx_mode = type_bits | raw.perms as u16;
    out.stx_size = raw.size as u64;
    out.stx_mtime.tv_sec = raw.mtime;
    Ok(out)
}

/// statx on a memfs fd (the AT_EMPTY_PATH arm of `statx`).
///
/// # Safety
/// `buf` follows the intercepted-call contract.
#[cfg(target_os = "linux")]
unsafe fn statx_memfs(fd: c_int, buf: *mut libc::statx) -> c_int {
    if buf.is_null() {
        set_errno(libc::EFAULT);
        return -1;
    }
    match engine_call(|| route::vfs_fstat(fd)) {
        // SAFETY: buf is valid per the call contract.
        Some(Ok(raw)) => match unsafe { fill_statx(&raw) } {
            Ok(s) => {
                unsafe { *buf = s };
                0
            }
            Err(e) => {
                set_errno(e);
                -1
            }
        },
        Some(Err(e)) => {
            set_errno(e);
            -1
        }
        None => {
            set_errno(libc::EIO);
            -1
        }
    }
}

/// Interposed `statx` (linux; glibc ≥ 2.28 exports the wrapper). The
/// `flags`/`mask` bits only refine the real call — the VFS always
/// answers BASIC_STATS.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn statx(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mask: libc::c_uint,
    buf: *mut libc::statx,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_statx()(dirfd, path, flags, mask, buf) };
    };
    if buf.is_null() {
        return unsafe { plat::real_statx()(dirfd, path, flags, mask, buf) };
    }
    if p.is_empty() {
        if (flags & libc::AT_EMPTY_PATH) != 0 && dirfd >= 0 && route::is_memfs_fd(dirfd) {
            return unsafe { statx_memfs(dirfd, buf) };
        }
        return unsafe { plat::real_statx()(dirfd, path, flags, mask, buf) };
    }
    match route_at(dirfd, p, route::vfs_stat) {
        Ok(Some(PathRoute::Vfs(raw))) => match unsafe { fill_statx(&raw) } {
            // SAFETY: buf is valid per the call contract.
            Ok(s) => {
                unsafe { *buf = s };
                0
            }
            Err(e) => {
                set_errno(e);
                -1
            }
        },
        Ok(Some(PathRoute::Denied(e))) | Err(e) => {
            set_errno(e);
            -1
        }
        Ok(Some(PathRoute::Host)) | Ok(None) => unsafe {
            plat::real_statx()(dirfd, path, flags, mask, buf)
        },
    }
}

/// Interposed `openat2` (linux). glibc exports no openat2 wrapper, so
/// this binds only for `dlsym("openat2")` consumers — callers of the raw
/// syscall bypass ALL interposition by construction (documented). For a
/// memfs path the struct is validated (size covers open_how; resolve ==
/// 0 — RESOLVE_* semantics are kernel path-walk rules the VFS does not
/// reimplement) and the open routes exactly like openat.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn openat2(
    dirfd: c_int,
    path: *const c_char,
    how: *mut libc::open_how,
    size: usize,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_openat2()(dirfd, path, how, size) };
    };
    if how.is_null() {
        set_errno(libc::EFAULT);
        return -1;
    }
    // SAFETY: how is valid per the call contract; the size check below
    // mirrors the kernel's ABI validation before any field is read.
    let how_ref = unsafe { &*how };
    if size < std::mem::size_of::<libc::open_how>() {
        set_errno(libc::EINVAL);
        return -1;
    }
    match route_at(dirfd, p, |rp| route::vfs_open(rp, how_ref.flags as i32)) {
        Ok(Some(PathRoute::Vfs(fd))) => {
            if how_ref.resolve != 0 {
                // RESOLVE_* on a memfs path is unsupported (memfs has no
                // symlinks/magic links and no beneath-root re-walk).
                let _ = engine_call(|| route::vfs_close(fd));
                set_errno(libc::EINVAL);
                -1
            } else {
                fd
            }
        }
        Ok(Some(PathRoute::Denied(e))) | Err(e) => {
            set_errno(e);
            -1
        }
        Ok(Some(PathRoute::Host)) | Ok(None) => unsafe {
            plat::real_openat2()(dirfd, path, how, size)
        },
    }
}

/// Interposed `getdents64` (linux; the glibc ≥ 2.30 wrapper — readdir's
/// raw substrate for DIRECT callers). A memfs fd is always a regular
/// file (a memfs directory never opens — EISDIR at open time; listings
/// are served through DIR streams), so the honest answer is ENOTDIR.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn getdents64(fd: c_int, dirp: *mut c_void, count: usize) -> libc::ssize_t {
    if fd >= 0 && route::is_memfs_fd(fd) {
        set_errno(libc::ENOTDIR);
        return -1;
    }
    unsafe { plat::real_getdents64()(fd, dirp, count) }
}

/// Interposed `__xstat` (linux; the pre-glibc-2.33 versioned entry point
/// that binaries built against older glibc call for stat). `ver` is the
/// glibc stat-version — x86-64 knows only _STAT_VER == 1, the modern
/// `struct stat` layout the shim fills — so it is accepted and ignored
/// (other architectures never reach this shim: the interposed surface is
/// 64-bit linux-gnu).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __xstat(ver: c_int, path: *const c_char, st: *mut libc::stat) -> c_int {
    let _ = ver;
    unsafe { stat_via(c_path(path), st, || plat::real_xstat()(ver, path, st)) }
}

/// Interposed `__lxstat` (pre-glibc-2.33 lstat; no symlink duality).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __lxstat(ver: c_int, path: *const c_char, st: *mut libc::stat) -> c_int {
    let _ = ver;
    unsafe { stat_via(c_path(path), st, || plat::real_lxstat()(ver, path, st)) }
}

/// Interposed `__fxstat` (pre-glibc-2.33 fstat; fd-flag dispatch).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __fxstat(ver: c_int, fd: c_int, st: *mut libc::stat) -> c_int {
    let _ = ver;
    if route::is_memfs_fd(fd) {
        return unsafe { fstat_memfs(fd, st) };
    }
    unsafe { plat::real_fxstat()(ver, fd, st) }
}

/// Interposed `opendir`. Memfs handles are the engine's small-integer ids
/// cast to `DIR *` (host `DIR *` values are heap pointers — never small
/// integers, so the registry-membership test cannot confuse them).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn opendir(path: *const c_char) -> *mut libc::DIR {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_opendir()(path) };
    };
    match engine_call(|| route::vfs_opendir(p)) {
        Some(PathRoute::Vfs(id)) => id as *mut libc::DIR,
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            std::ptr::null_mut()
        }
        Some(PathRoute::Host) | None => unsafe { plat::real_opendir()(path) },
    }
}

/// Interposed `readdir`.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn readdir(dirp: *mut libc::DIR) -> *mut libc::dirent {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        return readdir_memfs(dirp as usize);
    }
    unsafe { plat::real_readdir()(dirp) }
}

/// Linux: `readdir64` (glibc's LFS alias; same layout on 64-bit).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn readdir64(dirp: *mut libc::DIR) -> *mut libc::dirent {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        return readdir_memfs(dirp as usize);
    }
    unsafe { plat::real_readdir64()(dirp) }
}

/// Interposed `closedir`.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn closedir(dirp: *mut libc::DIR) -> c_int {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        if let Some(m) = DIRENT_CACHE.lock().unwrap().as_mut() {
            m.remove(&(dirp as usize));
        }
        return match engine_call(|| route::vfs_closedir(dirp as usize)) {
            Some(Ok(())) => 0,
            Some(Err(e)) => {
                set_errno(e);
                -1
            }
            None => {
                set_errno(libc::EIO);
                -1
            }
        };
    }
    unsafe { plat::real_closedir()(dirp) }
}

/// Interposed `readdir_r` (the deprecated reentrant readdir; still
/// exported by glibc and libSystem): fills the CALLER's dirent and
/// points `*result` at it (NULL at end of directory). Returns 0 on
/// success, the errno value directly on error.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn readdir_r(
    dirp: *mut libc::DIR,
    entry: *mut libc::dirent,
    result: *mut *mut libc::dirent,
) -> c_int {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        if entry.is_null() || result.is_null() {
            return libc::EINVAL;
        }
        return match engine_call(|| route::vfs_readdir(dirp as usize)) {
            Some(Ok(Some(ent))) => {
                // SAFETY: entry/result are valid per the call contract.
                unsafe {
                    fill_dirent(&mut *entry, &ent);
                    *result = entry;
                }
                0
            }
            // SAFETY: result is valid per the call contract.
            Some(Ok(None)) => {
                unsafe { *result = std::ptr::null_mut() };
                0
            }
            Some(Err(e)) => e,
            None => libc::EIO,
        };
    }
    unsafe { plat::real_readdir_r()(dirp, entry, result) }
}

/// Interposed `telldir` (index-based cookies, exactly the engine's).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn telldir(dirp: *mut libc::DIR) -> c_long {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        return match engine_call(|| route::vfs_telldir(dirp as usize)) {
            Some(Ok(pos)) => pos as c_long,
            Some(Err(e)) => {
                set_errno(e);
                -1
            }
            None => {
                set_errno(libc::EIO);
                -1
            }
        };
    }
    unsafe { plat::real_telldir()(dirp) }
}

/// Interposed `seekdir` (void return — an error survives only in errno,
/// which is what glibc's seekdir does too).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn seekdir(dirp: *mut libc::DIR, pos: c_long) {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        if let Some(Err(e)) = engine_call(|| route::vfs_seekdir(dirp as usize, pos as i64)) {
            set_errno(e);
        }
        return;
    }
    unsafe { plat::real_seekdir()(dirp, pos) }
}

/// Interposed `rewinddir` (void return, like seekdir).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn rewinddir(dirp: *mut libc::DIR) {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        if let Some(Err(e)) = engine_call(|| route::vfs_rewinddir(dirp as usize)) {
            set_errno(e);
        }
        return;
    }
    unsafe { plat::real_rewinddir()(dirp) }
}

/// Interposed `read` (fd-flag dispatch).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn read(fd: c_int, buf: *mut c_void, nbyte: usize) -> libc::ssize_t {
    if route::is_memfs_fd(fd) {
        if buf.is_null() && nbyte > 0 {
            set_errno(libc::EFAULT);
            return -1;
        }
        // SAFETY: caller guarantees buf/nbyte per the read contract;
        // empty when buf is NULL.
        let slice = unsafe {
            if buf.is_null() {
                &mut []
            } else {
                std::slice::from_raw_parts_mut(buf.cast::<u8>(), nbyte)
            }
        };
        return match engine_call(|| route::vfs_read(fd, slice)) {
            Some(Ok(n)) => n as libc::ssize_t,
            Some(Err(e)) => {
                set_errno(e);
                -1
            }
            None => {
                set_errno(libc::EIO);
                -1
            }
        };
    }
    unsafe { plat::real_read()(fd, buf, nbyte) }
}

/// Interposed `pread` (fd position untouched).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn pread(
    fd: c_int,
    buf: *mut c_void,
    nbyte: usize,
    offset: libc::off_t,
) -> libc::ssize_t {
    if route::is_memfs_fd(fd) {
        if buf.is_null() && nbyte > 0 {
            set_errno(libc::EFAULT);
            return -1;
        }
        // SAFETY: as in `read`.
        let slice = unsafe {
            if buf.is_null() {
                &mut []
            } else {
                std::slice::from_raw_parts_mut(buf.cast::<u8>(), nbyte)
            }
        };
        return match engine_call(|| route::vfs_pread(fd, slice, offset)) {
            Some(Ok(n)) => n as libc::ssize_t,
            Some(Err(e)) => {
                set_errno(e);
                -1
            }
            None => {
                set_errno(libc::EIO);
                -1
            }
        };
    }
    unsafe { plat::real_pread()(fd, buf, nbyte, offset) }
}

/// Interposed `lseek` (additive to the spec 07 §8 surface list: stdio
/// fseek on a memfs fd must stay on the VFS).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn lseek(fd: c_int, offset: libc::off_t, whence: c_int) -> libc::off_t {
    if route::is_memfs_fd(fd) {
        return match engine_call(|| route::vfs_lseek(fd, offset, whence)) {
            Some(Ok(pos)) => pos,
            Some(Err(e)) => {
                set_errno(e);
                -1
            }
            None => {
                set_errno(libc::EIO);
                -1
            }
        };
    }
    unsafe { plat::real_lseek()(fd, offset, whence) }
}

/// Interposed `close` (fd-flag dispatch; a memfs fd never reaches the
/// real close).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn close(fd: c_int) -> c_int {
    if route::is_memfs_fd(fd) {
        return match engine_call(|| route::vfs_close(fd)) {
            Some(Ok(())) => 0,
            Some(Err(e)) => {
                set_errno(e);
                -1
            }
            None => {
                set_errno(libc::EIO);
                -1
            }
        };
    }
    unsafe { plat::real_close()(fd) }
}

/// Interposed `mkdir` (write-class: memfs → EROFS, host → policy-gated).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn mkdir(path: *const c_char, mode: libc::mode_t) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_mkdir()(path, mode) };
    };
    match engine_call(|| route::vfs_write_path(p)) {
        Some(Ok(())) | None => unsafe { plat::real_mkdir()(path, mode) },
        Some(Err(e)) => {
            set_errno(e);
            -1
        }
    }
}

/// Interposed `unlink` (write-class).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn unlink(path: *const c_char) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_unlink()(path) };
    };
    match engine_call(|| route::vfs_write_path(p)) {
        Some(Ok(())) | None => unsafe { plat::real_unlink()(path) },
        Some(Err(e)) => {
            set_errno(e);
            -1
        }
    }
}

/// Interposed `rename` (write-class; both paths gated).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int {
    let (Some(o), Some(n)) = (unsafe { c_path(old) }, unsafe { c_path(new) }) else {
        return unsafe { plat::real_rename()(old, new) };
    };
    match engine_call(|| route::vfs_rename(o, n)) {
        Some(Ok(())) | None => unsafe { plat::real_rename()(old, new) },
        Some(Err(e)) => {
            set_errno(e);
            -1
        }
    }
}

/// Interposed `dlopen`: a memfs library is materialized through the
/// engine's `dlmap2file` host cache and the REAL dlopen loads that copy;
/// a host library passes through (policy-gated like any read). Failures
/// return NULL; dlerror() text is dyld/ld.so-internal and cannot be set
/// portably (errno carries the cause; documented).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void {
    if path.is_null() {
        return unsafe { plat::real_dlopen()(path, mode) };
    }
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_dlopen()(path, mode) };
    };
    match engine_call(|| route::vfs_dlmap(p)) {
        // SAFETY: `host` outlives the call.
        Some(PathRoute::Vfs(host)) => unsafe { plat::real_dlopen()(host.as_ptr(), mode) },
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            std::ptr::null_mut()
        }
        Some(PathRoute::Host) | None => unsafe { plat::real_dlopen()(path, mode) },
    }
}

/// Shared exec/spawn routing (spec 07 §8, roadmap 39): a memfs target is
/// materialized through the `dlmap2file` host cache — the same mechanism
/// as the `tfs exec` ENTRYPOINT path — and the real exec/spawn loads that
/// copy; the caller's argv/envp pass through verbatim, so the preload
/// env propagates and the child stays in the VFS. `Some(Err(e))` fails
/// the call with `e`; `None` passes the ORIGINAL path through.
fn exec_materialized(p: &str) -> Option<Result<CString, i32>> {
    match engine_call(|| route::vfs_exec_materialize(p)) {
        Some(PathRoute::Vfs(host)) => Some(Ok(host)),
        Some(PathRoute::Denied(e)) => Some(Err(e)),
        Some(PathRoute::Host) | None => None,
    }
}

/// Interposed `execve`. The execl/execv/execvp family are libc wrappers
/// whose inner exec call binds inside libc (not interposable) — DIRECT
/// execve callers (shells, supervisors, most tools) are covered, which
/// also makes `system()` of an in-image helper work (the shell's own
/// execve is interposed).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_execve()(path, argv, envp) };
    };
    match exec_materialized(p) {
        // SAFETY: `host` outlives the call (execve only returns on error).
        Some(Ok(host)) => unsafe { plat::real_execve()(host.as_ptr(), argv, envp) },
        Some(Err(e)) => {
            set_errno(e);
            -1
        }
        None => unsafe { plat::real_execve()(path, argv, envp) },
    }
}

/// Interposed `posix_spawn`. NOTE the return convention: 0 on success,
/// the error NUMBER directly on failure (never -1/errno).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn posix_spawn(
    pid: *mut libc::pid_t,
    path: *const c_char,
    file_actions: *const libc::posix_spawn_file_actions_t,
    attrp: *const libc::posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_posix_spawn()(pid, path, file_actions, attrp, argv, envp) };
    };
    match exec_materialized(p) {
        Some(Ok(host)) => unsafe {
            plat::real_posix_spawn()(pid, host.as_ptr(), file_actions, attrp, argv, envp)
        },
        Some(Err(e)) => e,
        None => unsafe { plat::real_posix_spawn()(pid, path, file_actions, attrp, argv, envp) },
    }
}

/// Interposed `posix_spawnp`: a file containing '/' routes exactly like
/// posix_spawn. A bare name passes through — the PATH search happens
/// inside libc/against the host PATH, so an in-image PATH entry cannot
/// be found by it (documented limit; use a full path).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn posix_spawnp(
    pid: *mut libc::pid_t,
    file: *const c_char,
    file_actions: *const libc::posix_spawn_file_actions_t,
    attrp: *const libc::posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    let Some(p) = (unsafe { c_path(file) }) else {
        return unsafe { plat::real_posix_spawnp()(pid, file, file_actions, attrp, argv, envp) };
    };
    if !p.contains('/') {
        return unsafe { plat::real_posix_spawnp()(pid, file, file_actions, attrp, argv, envp) };
    }
    match exec_materialized(p) {
        Some(Ok(host)) => unsafe {
            plat::real_posix_spawnp()(pid, host.as_ptr(), file_actions, attrp, argv, envp)
        },
        Some(Err(e)) => e,
        None => unsafe { plat::real_posix_spawnp()(pid, file, file_actions, attrp, argv, envp) },
    }
}

// ---------------------------------------------------------------------
// Initialization (the library constructor's payload)
// ---------------------------------------------------------------------

/// The constructor payload: establish the namespace from the environment.
/// Misformatted `TEBAKO_TFS_MOUNTS` / `TEBAKO_JAIL`, or an image that will
/// not mount, is a named configuration error: a clear stderr message
/// naming the variable and the offending token, then EX_CONFIG (78).
pub fn init() {
    if let Err(msg) = route::initialize() {
        eprintln!("libtfs-preload: {msg}");
        // SAFETY: plain libc call.
        unsafe { libc::exit(crate::spec::EX_CONFIG) };
    }
}
