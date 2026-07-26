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
use std::ffi::{c_char, c_int, c_void, CStr};
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
    let base = if p.starts_with('/') || dirfd == libc::AT_FDCWD || route::is_memfs_fd(dirfd) {
        None
    } else {
        resolve_dirfd(dirfd)
    };
    let routed = match route::resolve_at(dirfd, p, base) {
        Ok(rp) => rp,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    match engine_call(|| route::vfs_open(&routed, flags)) {
        Some(PathRoute::Vfs(fd)) => fd,
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            -1
        }
        Some(PathRoute::Host) | None => unsafe { plat::real_openat()(dirfd, path, flags, mode) },
    }
}

/// Shared body of stat/lstat.
///
/// # Safety
/// `orig`/`st` follow the intercepted-call contract.
unsafe fn stat_via(
    p: Option<&str>,
    orig: *const c_char,
    st: *mut libc::stat,
    real: unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int,
) -> c_int {
    let Some(p) = p else {
        return unsafe { real(orig, st) };
    };
    if st.is_null() {
        return unsafe { real(orig, st) };
    }
    match engine_call(|| route::vfs_stat(p)) {
        // SAFETY: st is a valid struct stat pointer per the call contract.
        Some(PathRoute::Vfs(raw)) => match unsafe { fill_stat(&raw) } {
            Ok(s) => {
                unsafe { *st = s };
                0
            }
            Err(e) => {
                set_errno(e);
                -1
            }
        },
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            -1
        }
        Some(PathRoute::Host) | None => unsafe { real(orig, st) },
    }
}

/// Interposed `stat`.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn stat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), path, st, plat::real_stat()) }
}

/// Interposed `lstat` (memfs has no symlink duality: lstat == stat).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn lstat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), path, st, plat::real_lstat()) }
}

/// Interposed `fstat` (fd-flag dispatch, no path).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn fstat(fd: c_int, st: *mut libc::stat) -> c_int {
    if route::is_memfs_fd(fd) {
        if st.is_null() {
            set_errno(libc::EFAULT);
            return -1;
        }
        return match engine_call(|| route::vfs_fstat(fd)) {
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
        };
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
    let base = if p.starts_with('/') || dirfd == libc::AT_FDCWD || route::is_memfs_fd(dirfd) {
        None
    } else {
        resolve_dirfd(dirfd)
    };
    let routed = match route::resolve_at(dirfd, p, base) {
        Ok(rp) => rp,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    match engine_call(|| route::vfs_access(&routed, mode)) {
        Some(PathRoute::Vfs(())) => 0,
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            -1
        }
        Some(PathRoute::Host) | None => unsafe { plat::real_faccessat()(dirfd, path, mode, flags) },
    }
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
