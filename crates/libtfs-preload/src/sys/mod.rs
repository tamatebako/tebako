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
//!
//! Fork children: the engine's backends are not fork-safe (see the
//! `IN_FORK_CHILD` guard below) — a `pthread_atfork` child handler arms a
//! process-global flag, and every engine entry in a fork child gets `None`
//! (the same "pass through to the real libc" answer re-entrancy gets).

use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tfs::context::TebakoCDirent;

use crate::route::{self, PathRoute};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
pub(crate) mod statx_abi;

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

// ---------------------------------------------------------------------
// Fork-child guard (the 2026-08-22 preload fork/exec deadlock)
// ---------------------------------------------------------------------

/// Set in the child side of every `fork` (see `register_fork_guard`).
///
/// WHY: the engine's backends are not fork-safe. dwarfs-t's block cache
/// runs a worker pool whose threads die at `fork`; any backend-touching
/// route in the child — e.g. the execve materialization probe's
/// `__tpkg__/manifest.yaml` read behind a `/` mount — waits on a
/// promise/future that no dead thread will ever complete, wedging the
/// child permanently (proven against runtime 0.16.4: payload mounted at
/// `/`, payload spawns `git clone` → git's pre-exec helper child hangs in
/// `std::condition_variable::wait` inside the block-cache dispatch).
///
/// The guard makes every engine entry in a fork child answer `None`,
/// which every shim already maps to "pass through to the real libc /
/// fail safe". A fork child that goes on to `exec` therefore calls the
/// REAL execve with the original arguments, and the exec'd image
/// re-enters a fresh, healthy shim through the inherited preload env —
/// the spec 22 §3 child-namespace propagation never depended on engine
/// calls in the pre-exec window. A fork child that never execs sees the
/// host only; memfs fds it inherited answer EIO (they would be stale
/// copies of parent engine state even if served).
static IN_FORK_CHILD: AtomicBool = AtomicBool::new(false);

/// The atfork CHILD handler: arm the fork-child guard. Runs in the child,
/// on the forking thread, before the child's `fork` caller resumes.
///
/// Declared `unsafe` purely for signature compatibility: it coerces to
/// the (safe) `extern "C" fn()` slot some libc bindings declare.
unsafe extern "C" fn mark_fork_child() {
    // Relaxed is sufficient: the handler runs on the forking thread in
    // the child's address space, and fork copies ONLY that thread — the
    // store and every later load are same-thread program order.
    IN_FORK_CHILD.store(true, Ordering::Relaxed);
}

/// Register the atfork child handler. Called ONCE from [`init`] (the
/// library constructor's payload), unconditionally — cheap, and NOT
/// behind `route::initialize`'s OnceLock: the guard must exist in every
/// process that has the shim loaded, however it was configured.
///
/// A registration failure (ENOMEM) leaves the historical unguarded
/// behavior; following the trace-arm precedent that is a loud stderr
/// note, never an init error.
pub(crate) fn register_fork_guard() {
    // SAFETY: plain libc call; the handler is a valid extern "C" fn.
    let rc = unsafe { libc::pthread_atfork(None, None, Some(mark_fork_child)) };
    if rc != 0 {
        eprintln!(
            "libtfs-preload: pthread_atfork registration failed (rc={rc}); \
             fork children keep the engine (deadlock risk)"
        );
    }
}

/// Run `f` (a route-layer engine call) unless this thread is already
/// inside the engine: re-entrant calls (the engine's own host IO) get
/// `None` and the shim passes them straight to the real implementation.
///
/// Fork children (between `fork` and any `exec`) get `None` first, for
/// the fail-safe reason on `IN_FORK_CHILD` above: the backends' worker
/// threads do not survive `fork`, so an engine call there would wait on
/// dead threads.
///
/// pub(crate) for the TEST seam: a test that calls the route layer
/// directly must enter through this guard exactly like the shims do.
/// Unguarded direct entry leaves IN_ENGINE unset, so the engine's own
/// host IO (the extraction writes under the context lock) re-enters the
/// shims and deadlocks the context lock on glibc (the 2026-08-21
/// route_matrix ubuntu hang; macOS test binaries do not interpose
/// in-process, so only Linux CI saw it).
pub(crate) fn engine_call<T>(f: impl FnOnce() -> T) -> Option<T> {
    engine_call_inner(IN_FORK_CHILD.load(Ordering::Relaxed), f)
}

/// The gate body with the fork-child flag as an explicit input, so the
/// unit pin can exercise both arms without mutating process-global state
/// (a global-flag test would race the sibling tests that enter through
/// `engine_call`).
fn engine_call_inner<T>(in_fork_child: bool, f: impl FnOnce() -> T) -> Option<T> {
    if in_fork_child {
        return None;
    }
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

/// The *at-family dirfd base (roadmap 39): `None` for absolute paths,
/// AT_FDCWD, memfs dirfds, and unknown negative dirfds — the strict
/// resolver handles each of those. CRITICAL: the fd branch gates on
/// `dirfd >= 0` — AT_FDCWD (-100) carries the TEBAKO_FD_FLAG bit, so a
/// bare bit test misroutes it into the memfs fd table (the bug class that
/// broke runtime builds; pinned in `route::tests::at_fdcwd_is_not_a_memfs_fd`).
fn at_base(dirfd: c_int, p: &str) -> Option<PathBuf> {
    if p.starts_with('/') || dirfd == libc::AT_FDCWD {
        return None;
    }
    if dirfd >= 0 && !route::is_memfs_fd(dirfd) {
        return resolve_dirfd(dirfd);
    }
    None
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

/// Interposed `open`. Declared with a fixed third `mode` parameter.
///
/// ABI note (proven against Temurin 21's NIO opens): on Darwin arm64
/// VARIADIC arguments are passed ON THE STACK (the first variadic slot
/// at [sp, 0]), not in the next register — a fixed-parameter shim reads
/// garbage for `mode` there (file creations landed mode 0000). The
/// trampoline below hoists the stack-passed mode into the register
/// before the Rust body runs. x86_64 Darwin and Linux pass it in the
/// register, so the fixed declaration is correct there.
#[cfg_attr(target_os = "linux", no_mangle)]
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    open_impl(path, flags, mode)
}

/// The Rust body of the open shim (entry point per ABI, see above).
#[no_mangle]
pub unsafe extern "C" fn open_impl(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
        eprintln!("[preload] open flags={flags:#o} mode={mode:#o}");
    }
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

/// Interposed `openat` (see `open` for the ABI note).
#[cfg_attr(target_os = "linux", no_mangle)]
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub unsafe extern "C" fn openat(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mode: c_int,
) -> c_int {
    openat_impl(dirfd, path, flags, mode)
}

/// The Rust body of the openat shim (entry point per ABI, see above).
#[no_mangle]
pub unsafe extern "C" fn openat_impl(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mode: c_int,
) -> c_int {
    if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
        eprintln!("[preload] openat flags={flags:#o} mode={mode:#o}");
    }
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_openat()(dirfd, path, flags, mode) };
    };
    let routed = match route::resolve_at_strict(dirfd, p, at_base(dirfd, p)) {
        Ok(route::AtRoute::Routed(rp)) => rp,
        Ok(route::AtRoute::Real) => {
            return unsafe { plat::real_openat()(dirfd, path, flags, mode) };
        }
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
/// `orig`/`st` follow the intercepted-call contract; `real` is the
/// platform's original implementation.
unsafe fn stat_via(
    p: Option<&str>,
    orig: *const c_char,
    st: *mut libc::stat,
    real: impl FnOnce(*const c_char, *mut libc::stat) -> c_int,
) -> c_int {
    let Some(p) = p else {
        return real(orig, st);
    };
    if st.is_null() {
        return real(orig, st);
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
        Some(PathRoute::Host) | None => real(orig, st),
    }
}

/// Interposed `stat`.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn stat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), path, st, |o, s| plat::real_stat()(o, s)) }
}

/// Interposed `lstat` (memfs has no symlink duality: lstat == stat).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn lstat(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), path, st, |o, s| plat::real_lstat()(o, s)) }
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
    let routed = match route::resolve_at_strict(dirfd, p, at_base(dirfd, p)) {
        Ok(route::AtRoute::Routed(rp)) => rp,
        Ok(route::AtRoute::Real) => {
            return unsafe { plat::real_faccessat()(dirfd, path, mode, flags) };
        }
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

/// Interposed `dirfd` (roadmap 39): a memfs stream has NO host fd behind
/// it — the POSIX-honest answer is -1/ENOTSUP ("the implementation does
/// not support the association of a file descriptor with a directory").
/// Consumers that probe the fd degrade gracefully (Rust's read_dir calls
/// dirfd in its drop path before closedir); the stream itself keeps
/// working through the interposed readdir family. Host streams pass
/// through.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn dirfd(dirp: *mut libc::DIR) -> c_int {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        set_errno(libc::ENOTSUP);
        return -1;
    }
    unsafe { plat::real_dirfd()(dirp) }
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

/// Linux: `__read_chk` (the _FORTIFY_SOURCE=2 read wrapper). The check
/// wrapper lives inside libc and calls the syscall stub directly, so an
/// interposed `read` never sees a fortified caller — the debian/temurin
/// JDK's libjli imports this exact symbol for the jar END-record read
/// (spec 22 class E). The fortify contract: a request larger than the
/// compiler-known buffer aborts, otherwise it is a plain read.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __read_chk(
    fd: c_int,
    buf: *mut c_void,
    nbyte: usize,
    buflen: usize,
) -> libc::ssize_t {
    if !route::is_memfs_fd(fd) {
        return unsafe { plat::real___read_chk()(fd, buf, nbyte, buflen) };
    }
    if nbyte > buflen {
        // glibc's __read_chk calls __chk_fail here; it never returns.
        unsafe { plat::real___chk_fail()() }
    }
    unsafe { read(fd, buf, nbyte) }
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

// ---------------------------------------------------------------------
// The LFS *64 family (linux) — Rust std and _FILE_OFFSET_BITS=64 builds
// call the 64 variants directly (open64/stat64/fstat64/…), which are
// DISTINCT exported glibc symbols from the plain names. Same layouts,
// same routing — each delegates to its plain-name body.
// ---------------------------------------------------------------------

/// Linux: `open64` (the LFS alias of `open`).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn open64(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    unsafe { open(path, flags, mode) }
}

/// Linux: `stat64` (the LFS alias of `stat`).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn stat64(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), path, st, |o, s| plat::real_stat64()(o, s)) }
}

/// Linux: `lstat64` (the LFS alias of `lstat`).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn lstat64(path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { stat_via(c_path(path), path, st, |o, s| plat::real_lstat64()(o, s)) }
}

/// Linux: `fstat64` (the LFS alias of `fstat`).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn fstat64(fd: c_int, st: *mut libc::stat) -> c_int {
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
    unsafe { plat::real_fstat64()(fd, st) }
}

/// Linux: `pread64` (the LFS alias of `pread`).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn pread64(
    fd: c_int,
    buf: *mut c_void,
    nbyte: usize,
    offset: libc::off_t,
) -> libc::ssize_t {
    unsafe { pread(fd, buf, nbyte, offset) }
}

/// Linux: `lseek64` (the LFS alias of `lseek`). The JDK launcher maps
/// `JLI_Lseek` to `lseek64` on glibc builds (spec 22 class E) — an
/// un-interposed `lseek64` on a flagged memfs fd is an immediate EBADF,
/// which is exactly how the launcher's zip END-record probe failed before
/// this alias existed ("Invalid or corrupt jarfile").
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn lseek64(fd: c_int, offset: libc::off_t, whence: c_int) -> libc::off_t {
    unsafe { lseek(fd, offset, whence) }
}

// ---------------------------------------------------------------------
// mmap / mmap64 (linux) — the JDK's libzip mmaps a jar's central
// directory at open (`USE_MMAP` is unconditional, `ZIP_Put_In_Cache`
// passes `usemmap=TRUE`). A flagged memfs fd reaching the real mmap is
// an immediate EBADF → MAP_FAILED, and libzip treats that as a hard
// open failure. Serve a private anonymous mapping pre-filled from the
// VFS instead; the consumer (the CEN scan, python's mmap module, git's
// pack windows) only reads. `munmap` needs no interpose: the mapping is
// a real anonymous one.
// ---------------------------------------------------------------------

/// Shared body of the linux mmap/mmap64 shims.
#[cfg(target_os = "linux")]
unsafe fn mmap_memfs_or_host(
    addr: *mut c_void,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: libc::off_t,
) -> *mut c_void {
    // MAP_ANONYMOUS and negative fds are never memfs: fd -1 (the
    // conventional anonymous companion) has every bit set, TEBAKO_FD_FLAG
    // included, so the bare bit test lies exactly as it does for AT_FDCWD
    // (route::resolve_at_strict's discipline). The JVM's very first
    // PaX-check mmap is anonymous and died here.
    if fd < 0 || flags & libc::MAP_ANONYMOUS != 0 || !route::is_memfs_fd(fd) {
        return unsafe { plat::real_mmap()(addr, len, prot, flags, fd, offset) };
    }
    // A MAP_SHARED writable mapping would promise persistence the
    // read-only memfs cannot honor; mmap(2) answers EACCES for exactly
    // that against a read-only fd.
    if flags & libc::MAP_SHARED != 0 && prot & libc::PROT_WRITE != 0 {
        set_errno(libc::EACCES);
        return libc::MAP_FAILED;
    }
    // The anonymous sibling: same address request (MAP_FIXED forwarded),
    // private, fd -1. ALWAYS mapped writable regardless of the caller's
    // prot: the fill below stores the VFS bytes into it, and a backing
    // page created PROT_READ faults at the first fill store (ubuntu-24.04
    // mmap-probe: SEGV addr == the fresh page, rip in
    // __memmove_avx_unaligned_erms — tebako run 31721085665). The
    // requested protection is restored by the mprotect after the fill.
    let aflags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | (flags & libc::MAP_FIXED);
    let p = unsafe { plat::real_mmap()(addr, len, prot | libc::PROT_WRITE, aflags, -1, 0) };
    if p == libc::MAP_FAILED {
        return p;
    }
    // Fill [offset, offset+len) from the VFS. A region reaching past EOF
    // keeps the anonymous zero-fill where a real mapping would SIGBUS —
    // the documented deviation; every real consumer maps inside the file.
    let mut done = 0usize;
    while done < len {
        let slice = unsafe { std::slice::from_raw_parts_mut(p.cast::<u8>().add(done), len - done) };
        match engine_call(|| route::vfs_pread(fd, slice, offset + done as libc::off_t)) {
            Some(Ok(0)) => break, // EOF: leave the zero-fill
            Some(Ok(n)) => done += n,
            Some(Err(e)) => {
                unsafe { plat::real_munmap()(p, len) };
                set_errno(e);
                return libc::MAP_FAILED;
            }
            None => {
                unsafe { plat::real_munmap()(p, len) };
                set_errno(libc::EIO);
                return libc::MAP_FAILED;
            }
        }
    }
    // Drop the fill-time PROT_WRITE the caller did not ask for, so a
    // consumer write faults exactly as it would against a real mapping.
    if prot & libc::PROT_WRITE == 0 {
        // SAFETY: p/len name the live mapping; mprotect sets errno itself.
        if unsafe { plat::real_mprotect()(p, len, prot) } != 0 {
            unsafe { plat::real_munmap()(p, len) };
            return libc::MAP_FAILED;
        }
    }
    p
}

/// Linux: interposed `mmap` (fd-flag dispatch).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn mmap(
    addr: *mut c_void,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: libc::off_t,
) -> *mut c_void {
    unsafe { mmap_memfs_or_host(addr, len, prot, flags, fd, offset) }
}

/// Linux: `mmap64` (the LFS alias of `mmap`; glibc builds of the JDK's
/// libzip call it by this name).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn mmap64(
    addr: *mut c_void,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: libc::off_t,
) -> *mut c_void {
    unsafe { mmap_memfs_or_host(addr, len, prot, flags, fd, offset) }
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

/// Interposed `fopen` (read modes): libSystem's stdio opens files
/// through its own internal syscall path (never the interposed `open` —
/// the JVM's `jvm.cfg` read proved it), so stdio consumers need their
/// own hook. Like dlopen: the engine materializes the memfs original
/// (`dlmap2file`, dlmap-prefix redirect included) and the REAL fopen
/// opens that copy. Write/append/update modes pass through with the
/// ORIGINAL arguments (memfs content is read-only; the real fopen
/// answers).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn fopen(path: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    let (Some(p), Some(m)) = (unsafe { c_path(path) }, unsafe { c_path(mode) }) else {
        return unsafe { plat::real_fopen()(path, mode) };
    };
    if !m.starts_with('r') {
        return unsafe { plat::real_fopen()(path, mode) };
    }
    match engine_call(|| route::vfs_fopen(p)) {
        // SAFETY: `host` outlives the call.
        Some(PathRoute::Vfs(host)) => unsafe { plat::real_fopen()(host.as_ptr(), mode) },
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            std::ptr::null_mut()
        }
        Some(PathRoute::Host) | None => unsafe { plat::real_fopen()(path, mode) },
    }
}

/// macOS's `fopen$DARWIN_EXTSN` (the extended-signature export the 64-bit
/// SDK binds) — the same body as `fopen`.
#[cfg(target_os = "macos")]
pub unsafe extern "C" fn fopen_darwin_extsn(
    path: *const c_char,
    mode: *const c_char,
) -> *mut libc::FILE {
    unsafe { fopen(path, mode) }
}

/// Shared body of the *at stat family (fstatat / fstatat64 / __fxstatat):
/// resolve the (dirfd, path) pair through the one-gate strict resolver,
/// serve memfs from the engine, pass host through with the ORIGINAL
/// arguments (`flags` only refines the real call — the VFS answer is the
/// same; no symlink duality).
///
/// # Safety
/// All pointers follow the intercepted-call contract; `real` is the
/// pass-through invoking the platform's original with its full arguments.
unsafe fn fstatat_via(
    dirfd: c_int,
    path: *const c_char,
    st: *mut libc::stat,
    real: impl FnOnce() -> c_int,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return real();
    };
    if st.is_null() {
        return real();
    }
    let routed = match route::resolve_at_strict(dirfd, p, at_base(dirfd, p)) {
        Ok(route::AtRoute::Routed(rp)) => rp,
        Ok(route::AtRoute::Real) => return real(),
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    match engine_call(|| route::vfs_stat(&routed)) {
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
        Some(PathRoute::Host) | None => real(),
    }
}

/// Interposed `fstatat` (roadmap 39 — the *at family).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn fstatat(
    dirfd: c_int,
    path: *const c_char,
    st: *mut libc::stat,
    flags: c_int,
) -> c_int {
    unsafe {
        fstatat_via(dirfd, path, st, || {
            plat::real_fstatat()(dirfd, path, st, flags)
        })
    }
}

/// Linux: `fstatat64` (the LFS alias; same layout on 64-bit).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn fstatat64(
    dirfd: c_int,
    path: *const c_char,
    st: *mut libc::stat,
    flags: c_int,
) -> c_int {
    unsafe {
        fstatat_via(dirfd, path, st, || {
            plat::real_fstatat64()(dirfd, path, st, flags)
        })
    }
}

/// Linux: `statx` (roadmap 39). The engine's RawStat fills the statx
/// answer; stx_mask reports exactly the fields written. The ABI comes
/// from statx_abi (libc on glibc, the uapi mirror on musl — libc
/// 0.2.189 has no musl statx, and musl itself lacks the wrapper before
/// 1.2.4).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn statx(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mask: libc::c_uint,
    stx: *mut statx_abi::statx,
) -> c_int {
    use tfs::backend::EntryType;

    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_statx()(dirfd, path, flags, mask, stx) };
    };
    if stx.is_null() {
        return unsafe { plat::real_statx()(dirfd, path, flags, mask, stx) };
    }
    let routed = match route::resolve_at_strict(dirfd, p, at_base(dirfd, p)) {
        Ok(route::AtRoute::Routed(rp)) => rp,
        Ok(route::AtRoute::Real) => {
            return unsafe { plat::real_statx()(dirfd, path, flags, mask, stx) };
        }
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    match engine_call(|| route::vfs_stat(&routed)) {
        Some(PathRoute::Vfs(raw)) => {
            #[allow(clippy::unnecessary_cast)] // identity on linux, required on macOS
            let type_bits: u32 = match raw.entry_type {
                EntryType::File => libc::S_IFREG as u32,
                EntryType::Directory => libc::S_IFDIR as u32,
                _ => {
                    set_errno(libc::EINVAL);
                    return -1;
                }
            };
            // SAFETY: a zeroed struct statx is valid; stx is valid per the
            // call contract. Field-by-field assignment: statx_timestamp's
            // padding is private (libc-version dependent).
            let mut out: statx_abi::statx = unsafe { std::mem::zeroed() };
            out.stx_mode = (type_bits | raw.perms) as u16;
            out.stx_size = raw.size as u64;
            out.stx_nlink = 1;
            out.stx_mtime.tv_sec = raw.mtime;
            out.stx_mtime.tv_nsec = 0;
            out.stx_mask = statx_abi::STATX_TYPE
                | statx_abi::STATX_MODE
                | statx_abi::STATX_NLINK
                | statx_abi::STATX_SIZE
                | statx_abi::STATX_MTIME;
            unsafe { *stx = out };
            0
        }
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            -1
        }
        Some(PathRoute::Host) | None => unsafe {
            plat::real_statx()(dirfd, path, flags, mask, stx)
        },
    }
}

// Linux: `openat2`? — NO: glibc exposes no `openat2` wrapper or symbol
// (only `SYS_openat2`), so there is nothing to interpose on linux-gnu; a
// raw `syscall(2)` caller bypasses userland interposition by
// construction (musl's wrapper is a later consideration).

/// Linux: `getdents64` (roadmap 39). VFS directory enumeration rides
/// opendir/readdir (a memfs directory can never be fd-opened — open
/// answers EISDIR), so a memfs fd here is a regular FILE and the honest
/// answer is ENOTDIR; host fds pass through.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn getdents64(fd: c_int, dirp: *mut c_void, count: usize) -> libc::ssize_t {
    if route::is_memfs_fd(fd) {
        set_errno(libc::ENOTDIR);
        return -1;
    }
    unsafe { plat::real_getdents64()(fd, dirp, count) }
}

/// Linux: `__xstat` (the pre-glibc-2.33 versioned stat entry — binaries
/// built against older glibc; roadmap 39).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __xstat(ver: c_int, path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe {
        stat_via(c_path(path), path, st, |o, s| {
            plat::real___xstat()(ver, o, s)
        })
    }
}

/// Linux: `__lxstat` (versioned lstat; memfs has no symlink duality).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __lxstat(ver: c_int, path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe {
        stat_via(c_path(path), path, st, |o, s| {
            plat::real___lxstat()(ver, o, s)
        })
    }
}

/// Linux: `__fxstat` (versioned fstat — fd-flag dispatch, no path).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __fxstat(ver: c_int, fd: c_int, st: *mut libc::stat) -> c_int {
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
    unsafe { plat::real___fxstat()(ver, fd, st) }
}

/// Linux: `__fxstatat` (versioned fstatat).
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __fxstatat(
    ver: c_int,
    dirfd: c_int,
    path: *const c_char,
    st: *mut libc::stat,
    flags: c_int,
) -> c_int {
    unsafe {
        fstatat_via(dirfd, path, st, || {
            plat::real___fxstatat()(ver, dirfd, path, st, flags)
        })
    }
}

/// Linux: `__xstat64` (the LFS64 versioned stat entry — the JDK's
/// libjava/libnio import it on glibc < 2.33 fortify builds, spec 22
/// class E). Delegates to `__xstat`: on glibc the *64 versioned entries
/// are literally the same addresses as the plain versioned ones (2.31
/// nm proof), the x86_64 layouts are identical, and the plain
/// `stat64`/`fstat64` dynamic symbols do NOT exist before glibc 2.33 —
/// the versioned entry is the only resolvable host passthrough there.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __xstat64(ver: c_int, path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { __xstat(ver, path, st) }
}

/// Linux: `__lxstat64` (versioned LFS64 lstat; memfs has no symlink
/// duality). Same delegation rationale as `__xstat64`.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __lxstat64(ver: c_int, path: *const c_char, st: *mut libc::stat) -> c_int {
    unsafe { __lxstat(ver, path, st) }
}

/// Linux: `__fxstat64` (versioned LFS64 fstat — fd-flag dispatch). Same
/// delegation rationale as `__xstat64`.
#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn __fxstat64(ver: c_int, fd: c_int, st: *mut libc::stat) -> c_int {
    unsafe { __fxstat(ver, fd, st) }
}

/// Interposed `rewinddir` (roadmap 39): reset a memfs stream to its first
/// entry. Void — an unknown-handle error is surfaced via errno only.
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

/// Interposed `telldir` (roadmap 39): the stream's position cookie (the
/// engine's readdir ordinal).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn telldir(dirp: *mut libc::DIR) -> libc::c_long {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        return match engine_call(|| route::vfs_telldir(dirp as usize)) {
            // c_long == i64 on every platform the shim targets.
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
    unsafe { plat::real_telldir()(dirp) }
}

/// Interposed `seekdir` (roadmap 39): set the stream's position to a
/// telldir cookie (clamped to end-of-directory).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn seekdir(dirp: *mut libc::DIR, pos: libc::c_long) {
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        // c_long == i64 on every platform the shim targets.
        if let Some(Err(e)) = engine_call(|| route::vfs_seekdir(dirp as usize, pos)) {
            set_errno(e);
        }
        return;
    }
    unsafe { plat::real_seekdir()(dirp, pos) }
}

/// Interposed `readdir_r` (roadmap 39; the deprecated reentrant form —
/// the caller's `entry` is filled directly, no per-handle cache needed).
/// Returns 0 on success, the errno on error (its own contract, never -1).
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn readdir_r(
    dirp: *mut libc::DIR,
    entry: *mut libc::dirent,
    result: *mut *mut libc::dirent,
) -> c_int {
    if entry.is_null() || result.is_null() {
        return libc::EINVAL;
    }
    // SAFETY: result is valid per the call contract (null-checked above).
    unsafe { *result = std::ptr::null_mut() };
    if !dirp.is_null() && engine_call(|| route::dir_is_embedded(dirp as usize)) == Some(true) {
        return match engine_call(|| route::vfs_readdir(dirp as usize)) {
            // SAFETY: entry is valid per the call contract.
            Some(Ok(Some(e))) => {
                unsafe { fill_dirent(&mut *entry, &e) };
                unsafe { *result = entry };
                0
            }
            Some(Ok(None)) => 0,
            Some(Err(e)) => e,
            None => libc::EIO,
        };
    }
    unsafe { plat::real_readdir_r()(dirp, entry, result) }
}

/// Interposed `execve` (roadmap 39): a MEMFS path is materialized through
/// the engine's `dlmap2file` host cache and the REAL execve loads that
/// copy (execve needs a host path); a host path execs the original
/// ungated — exec of a host binary is not an IO route in the policy's op
/// classes, and the child's own IO stays jailed via env propagation.
#[cfg_attr(target_os = "linux", no_mangle)]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    let Some(p) = (unsafe { c_path(path) }) else {
        return unsafe { plat::real_execve()(path, argv, envp) };
    };
    match engine_call(|| route::vfs_materialize_exec(p)) {
        // SAFETY: `host` outlives the call; argv/envp forward verbatim.
        Some(PathRoute::Vfs(host)) => unsafe { plat::real_execve()(host.as_ptr(), argv, envp) },
        Some(PathRoute::Denied(e)) => {
            set_errno(e);
            -1
        }
        Some(PathRoute::Host) | None => unsafe { plat::real_execve()(path, argv, envp) },
    }
}

/// Interposed `posix_spawn` (roadmap 39): the execve routing with the
/// spawn contract — the return IS the errno (never -1). The trace op is
/// `spawn` (spec 25 §2: a child is created — the stream's process-tree
/// signal), never `exec`.
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
    match engine_call(|| route::vfs_materialize_spawn(p)) {
        Some(PathRoute::Vfs(host)) => unsafe {
            plat::real_posix_spawn()(pid, host.as_ptr(), file_actions, attrp, argv, envp)
        },
        Some(PathRoute::Denied(e)) => e,
        Some(PathRoute::Host) | None => unsafe {
            plat::real_posix_spawn()(pid, path, file_actions, attrp, argv, envp)
        },
    }
}

/// Interposed `posix_spawnp` (roadmap 39): an explicit path routes like
/// posix_spawn (the `spawn` trace op, spec 25 §2); a bare name is a host
/// PATH search (memfs dirs are not in the host PATH — pass through,
/// stated honestly in the crate docs; no VFS decision exists, so no
/// trace event).
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
    match engine_call(|| route::vfs_materialize_spawn(p)) {
        Some(PathRoute::Vfs(host)) => unsafe {
            plat::real_posix_spawnp()(pid, host.as_ptr(), file_actions, attrp, argv, envp)
        },
        Some(PathRoute::Denied(e)) => e,
        Some(PathRoute::Host) | None => unsafe {
            plat::real_posix_spawnp()(pid, file, file_actions, attrp, argv, envp)
        },
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
    // Arm the fork-child guard FIRST, before any mount can happen: a
    // backend worker pool comes into existence at mount time, and the
    // guard must already be registered before any later fork.
    register_fork_guard();
    if let Err(msg) = route::initialize() {
        eprintln!("libtfs-preload: {msg}");
        // SAFETY: plain libc call.
        unsafe { libc::exit(crate::spec::EX_CONFIG) };
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The guard gate: a fork child never enters the engine (`None` is
    /// every shim's "pass through to the real libc / fail safe" answer);
    /// a normal thread enters, and re-entrancy still answers `None`. Both
    /// arms are exercised through `engine_call_inner`'s explicit flag
    /// input so this test never mutates process-global state (a global
    /// flag set here would race the sibling route tests that enter
    /// through `engine_call`).
    #[test]
    fn fork_child_gate_short_circuits_engine_call() {
        assert_eq!(engine_call_inner(true, || 42), None);
        assert_eq!(engine_call_inner(false, || 42), Some(42));
    }

    /// The atfork wiring, for real: register, `fork`, and have the CHILD
    /// answer whether the handler armed the flag. The child's post-fork
    /// work is one atomic load and `_exit` (async-signal-safe in
    /// practice); the parent asserts the exit code. The flag is set
    /// copy-on-write in the child's address space only — the parent's
    /// flag stays clear, so no sibling test is affected. Repeated
    /// registration across test runs is harmless (the handlers all set
    /// the same flag, and nothing else in this process forks).
    #[test]
    fn atfork_child_handler_arms_the_flag() {
        register_fork_guard();
        // SAFETY: plain fork; the child's post-fork work is an atomic
        // load + _exit, both safe after fork in a threaded process.
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed: {}", std::io::Error::last_os_error());
        if pid == 0 {
            let rc = if IN_FORK_CHILD.load(Ordering::Relaxed) {
                0
            } else {
                42
            };
            // SAFETY: plain libc call; never returns.
            unsafe { libc::_exit(rc) };
        }
        let mut status = 0;
        // SAFETY: pid is our child; status is a writable out-parameter.
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
        assert!(libc::WIFEXITED(status), "child did not exit normally");
        assert_eq!(
            libc::WEXITSTATUS(status),
            0,
            "fork child saw the guard flag clear"
        );
    }
}
