//! The `tebako_fs_*` C ABI: extern "C" exports over the safe context.
//!
//! This is the ONLY module with `unsafe` in the crate, and the only one
//! that touches raw pointers. Every export: validates arguments (NULL →
//! EINVAL), runs the safe implementation, and reports through the
//! thread-local errno channel (0 on success).
//!
//! String arguments are converted losslessly only when they are valid
//! UTF-8; non-UTF-8 paths fail with EINVAL (v1 limitation, matching every
//! real fixture; the C++ side treats paths as byte strings).

use std::ffi::{c_char, c_void, CStr};

use crate::backend::EntryType;
use crate::context::{context, TebakoCDirent, TEBAKO_FD_FLAG};
use crate::errno::{get_errno, set_errno, strerror};
use crate::mount;
use crate::policy::{HostAccess, HostMountSpec, HostPolicy};

/// Convert a borrowed C string argument to &str (EINVAL on NULL/non-UTF-8).
///
/// # Safety
/// `ptr` must be NULL or point to a valid NUL-terminated string that
/// outlives the call.
unsafe fn path_arg<'a>(ptr: *const c_char) -> Result<&'a str, i32> {
    if ptr.is_null() {
        return Err(libc::EINVAL);
    }
    // SAFETY: per the caller contract above.
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str().map_err(|_| libc::EINVAL)
}

/// Convert an OPTIONAL borrowed C string argument to Option<&str>
/// (NULL → None; non-UTF-8 → EINVAL).
///
/// # Safety
/// `ptr` must be NULL or point to a valid NUL-terminated string that
/// outlives the call.
unsafe fn optional_path_arg<'a>(ptr: *const c_char) -> Result<Option<&'a str>, i32> {
    if ptr.is_null() {
        return Ok(None);
    }
    // SAFETY: per the caller contract above.
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str().map(Some).map_err(|_| libc::EINVAL)
}

/// Map the TEBAKO_MOUNT_* flag word to a MountMode (EINVAL otherwise;
/// negative values wrap to huge u32 and land on EINVAL too).
#[allow(clippy::cast_sign_loss)]
fn parse_mount_mode(mode: libc::c_int) -> Result<mount::MountMode, i32> {
    match mode as u32 {
        mount::TEBAKO_MOUNT_RO => Ok(mount::MountMode::ReadOnly),
        mount::TEBAKO_MOUNT_COW => Ok(mount::MountMode::Cow),
        mount::TEBAKO_MOUNT_RW => Ok(mount::MountMode::ReadWrite),
        _ => Err(libc::EINVAL),
    }
}

fn fail(err: i32) -> i32 {
    set_errno(err);
    -1
}

// ===================================================================
// Lifecycle Management
// ===================================================================

/// `tebako_fs_init_from_file`: mount an archive file (format auto-detected).
///
/// # Safety
/// C ABI entry point: pointer arguments must follow the C contract.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_init_from_file(
    archive_path: *const c_char,
    mount_point: *const c_char,
) -> libc::c_int {
    tebako_fs_init_from_file_at(archive_path, 0, 0, mount_point)
}

/// `tebako_fs_init_from_file_at`: mount a region of a file.
///
/// # Safety
/// C ABI entry point: pointer arguments must follow the C contract.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_init_from_file_at(
    archive_path: *const c_char,
    offset: u64,
    length: u64,
    mount_point: *const c_char,
) -> libc::c_int {
    let (archive_path, mount_point) = match (unsafe { path_arg(archive_path) }, unsafe {
        path_arg(mount_point)
    }) {
        (Ok(a), Ok(m)) => (a, m),
        _ => return fail(libc::EINVAL),
    };
    if mount_point.is_empty() {
        return fail(libc::EINVAL);
    }
    // Reading the image off the host is a host-passthrough decision:
    // the policy gates it (spec 08; a no-op under the default open policy).
    if let Err(e) = context()
        .read()
        .unwrap()
        .host_check(archive_path, HostAccess::Ro)
    {
        return fail(e);
    }
    let mount = match mount::build_from_file_at(archive_path, offset, length, mount_point) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    match context().write().unwrap().init_mount(mount) {
        Ok(()) => {
            set_errno(0);
            0
        }
        Err(e) => fail(e),
    }
}

/// `tebako_fs_init`: mount an archive from memory.
///
/// # Safety
/// `data` must point to `size` readable bytes (the image is copied).
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_init(
    data: *const c_void,
    size: usize,
    mount_point: *const c_char,
) -> libc::c_int {
    if data.is_null() || size == 0 {
        return fail(libc::EINVAL);
    }
    let mount_point = match unsafe { path_arg(mount_point) } {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    if mount_point.is_empty() {
        return fail(libc::EINVAL);
    }
    // SAFETY: caller guarantees data/size validity; we copy immediately.
    let data = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    let mount = match mount::build_from_memory(data, mount_point) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    match context().write().unwrap().init_mount(mount) {
        Ok(()) => {
            set_errno(0);
            0
        }
        Err(e) => fail(e),
    }
}

/// `tebako_fs_unmount`: unmount everything; safe to call repeatedly.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_unmount() {
    context().write().unwrap().unmount();
    set_errno(0);
}

/// `tebako_is_initialized`.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_is_initialized() -> libc::c_int {
    i32::from(context().read().unwrap().is_mounted())
}

// ===================================================================
// File Operations
// ===================================================================

/// `tebako_fs_open` (read-only; fds carry TEBAKO_FD_FLAG).
///
/// # Safety
/// C ABI entry point: `path` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_open(path: *const c_char, flags: libc::c_int) -> libc::c_int {
    let path = match unsafe { path_arg(path) } {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    match context().write().unwrap().open(path, flags) {
        Ok(fd) => {
            set_errno(0);
            fd
        }
        Err(e) => fail(e),
    }
}

/// `tebako_fs_read`.
///
/// # Safety
/// `buf` must be writable for `count` bytes.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_read(
    fd: libc::c_int,
    buf: *mut c_void,
    count: usize,
) -> libc::ssize_t {
    if buf.is_null() && count > 0 {
        return fail(libc::EINVAL) as libc::ssize_t;
    }
    // SAFETY: caller guarantees buf/count validity.
    let buf = unsafe { std::slice::from_raw_parts_mut(buf.cast::<u8>(), count) };
    match context().write().unwrap().read(fd, buf) {
        Ok(n) => {
            set_errno(0);
            n as libc::ssize_t
        }
        Err(e) => fail(e) as libc::ssize_t,
    }
}

/// `tebako_fs_pread` (fd position untouched).
///
/// # Safety
/// `buf` must be writable for `nbyte` bytes.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_pread(
    fd: libc::c_int,
    buf: *mut c_void,
    nbyte: usize,
    offset: libc::off_t,
) -> libc::ssize_t {
    if buf.is_null() && nbyte > 0 {
        return fail(libc::EINVAL) as libc::ssize_t;
    }
    let buf = unsafe { std::slice::from_raw_parts_mut(buf.cast::<u8>(), nbyte) };
    match context().write().unwrap().pread(fd, buf, offset) {
        Ok(n) => {
            set_errno(0);
            n as libc::ssize_t
        }
        Err(e) => fail(e) as libc::ssize_t,
    }
}

/// `tebako_fs_lseek`.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_lseek(
    fd: libc::c_int,
    offset: libc::off_t,
    whence: libc::c_int,
) -> libc::off_t {
    match context().write().unwrap().lseek(fd, offset, whence) {
        Ok(pos) => {
            set_errno(0);
            pos
        }
        Err(e) => fail(e) as libc::off_t,
    }
}

/// `tebako_fs_close`.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_close(fd: libc::c_int) -> libc::c_int {
    match context().write().unwrap().close(fd) {
        Ok(()) => {
            set_errno(0);
            0
        }
        Err(e) => fail(e),
    }
}

// ===================================================================
// Directory Operations
// ===================================================================

/// `tebako_fs_opendir`.
///
/// # Safety
/// C ABI entry point: `path` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_opendir(path: *const c_char) -> *mut c_void {
    let path = match unsafe { path_arg(path) } {
        Ok(p) => p,
        Err(e) => {
            fail(e);
            return std::ptr::null_mut();
        }
    };
    match context().write().unwrap().opendir(path) {
        Ok(id) => {
            set_errno(0);
            id as *mut c_void
        }
        Err(e) => {
            fail(e);
            std::ptr::null_mut()
        }
    }
}

/// `tebako_fs_readdir` (pointer valid until next readdir/closedir).
///
/// # Safety
/// `dir` must be a handle from tebako_fs_opendir.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_readdir(dir: *mut c_void) -> *mut TebakoCDirent {
    if dir.is_null() {
        fail(libc::EBADF);
        return std::ptr::null_mut();
    }
    let mut ctx = context().write().unwrap();
    match ctx.readdir_abi(dir as usize) {
        Ok(true) => {
            set_errno(0);
            ctx.dir_current_ptr(dir as usize) as *mut TebakoCDirent
        }
        Ok(false) => {
            set_errno(0);
            std::ptr::null_mut()
        }
        Err(e) => {
            fail(e);
            std::ptr::null_mut()
        }
    }
}

/// `tebako_fs_closedir`.
///
/// # Safety
/// `dir` must be a handle from tebako_fs_opendir.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_closedir(dir: *mut c_void) -> libc::c_int {
    if dir.is_null() {
        return fail(libc::EBADF);
    }
    match context().write().unwrap().closedir(dir as usize) {
        Ok(()) => {
            set_errno(0);
            0
        }
        Err(e) => fail(e),
    }
}

/// `tebako_fs_dir_is_embedded`: registry-membership test.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_dir_is_embedded(dir: *mut c_void) -> libc::c_int {
    if dir.is_null() {
        return 0;
    }
    i32::from(context().read().unwrap().dir_is_embedded(dir as usize))
}

// ===================================================================
// Metadata Operations
// ===================================================================

/// Fill a caller's `struct stat` from a RawStat (zeroed first, like C++).
// The S_IF* constant widths differ per platform (u16 on macOS, u32 on
// Linux): the widening `as u32` is required on macOS and an identity cast
// on Linux, so the platform-dependent unnecessary_cast lint is allowed
// here deliberately.
#[allow(clippy::unnecessary_cast)]
fn fill_stat(st: *mut libc::stat, raw: &crate::backend::RawStat) -> i32 {
    // SAFETY: caller guarantees `st` points to a valid struct stat.
    let out = unsafe { &mut *st };
    *out = unsafe { std::mem::zeroed() };
    let type_bits: u32 = match raw.entry_type {
        EntryType::File => libc::S_IFREG as u32,
        EntryType::Directory => libc::S_IFDIR as u32,
        // C++ returns EINVAL for anything that is neither file nor dir.
        _ => return libc::EINVAL,
    };
    out.st_mode = (type_bits | raw.perms) as libc::mode_t;
    out.st_size = raw.size as libc::off_t;
    out.st_mtime = raw.mtime as libc::time_t;
    out.st_nlink = 1 as _;
    0
}

/// `tebako_fs_stat`.
///
/// # Safety
/// `path` must be a valid C string; `st` must point to a valid struct stat.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_stat(path: *const c_char, st: *mut libc::stat) -> libc::c_int {
    if st.is_null() {
        return fail(libc::EINVAL);
    }
    let path = match unsafe { path_arg(path) } {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    match context().read().unwrap().stat(path) {
        Ok(raw) => {
            let rc = fill_stat(st, &raw);
            set_errno(rc);
            if rc == 0 {
                0
            } else {
                -1
            }
        }
        Err(e) => fail(e),
    }
}

/// `tebako_fs_fstat`.
///
/// # Safety
/// `st` must point to a valid struct stat.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_fstat(fd: libc::c_int, st: *mut libc::stat) -> libc::c_int {
    if st.is_null() {
        return fail(libc::EINVAL);
    }
    match context().read().unwrap().fstat(fd) {
        Ok(raw) => {
            let rc = fill_stat(st, &raw);
            set_errno(rc);
            if rc == 0 {
                0
            } else {
                -1
            }
        }
        Err(e) => fail(e),
    }
}

// ===================================================================
// Path Detection
// ===================================================================

/// `tebako_path_is_embedded`.
///
/// # Safety
/// `path` must be NULL (returns 0) or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn tebako_path_is_embedded(path: *const c_char) -> libc::c_int {
    if path.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees a valid C string.
    let path = unsafe { CStr::from_ptr(path) };
    match path.to_str() {
        Ok(p) => i32::from(context().read().unwrap().path_is_embedded(p)),
        Err(_) => 0,
    }
}

/// `tebako_fd_is_embedded`: TEBAKO_FD_FLAG bit check.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_fd_is_embedded(fd: libc::c_int) -> libc::c_int {
    i32::from((fd & TEBAKO_FD_FLAG) != 0)
}

// ===================================================================
// Error Handling
// ===================================================================

/// `tebako_get_errno`: the thread-local error cell.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_get_errno() -> libc::c_int {
    get_errno()
}

/// `tebako_strerror`: static message for an errno value (do not free).
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_strerror(err: libc::c_int) -> *const c_char {
    strerror(err).as_ptr().cast()
}

// ===================================================================
// Utility Functions
// ===================================================================

/// `tebako_get_mount_point` (valid until unmount; NULL when not mounted).
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_get_mount_point() -> *const c_char {
    match context().read().unwrap().compat_mount_point() {
        Some(cs) => cs.as_ptr(),
        None => std::ptr::null(),
    }
}

/// `tebako_get_archive_path` (NULL for memory mounts / not mounted).
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_get_archive_path() -> *const c_char {
    match context().read().unwrap().compat_archive_path() {
        Some(cs) => cs.as_ptr(),
        None => std::ptr::null(),
    }
}

/// `tebako_get_backend_name` (NULL when not mounted).
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_get_backend_name() -> *const c_char {
    match context().read().unwrap().compat_backend_name() {
        Some(name) => name.as_ptr(),
        None => std::ptr::null(),
    }
}

// ===================================================================
// ABI Version
// ===================================================================

/// `tebako_fs_abi_version`: the C ABI version of this library
/// (== TEBAKO_FS_ABI_VERSION in the headers).
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_abi_version() -> libc::c_int {
    1
}

// ===================================================================
// Multi-Mount Management
// ===================================================================

/// Shared tail of the mount_* exports: insert the mount, report the handle.
fn finish_mount(
    result: Result<crate::context::Mount, i32>,
    out_handle: *mut libc::c_int,
) -> libc::c_int {
    let mount = match result {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    match context().write().unwrap().mount_checked(mount) {
        Ok(handle) => {
            // SAFETY: out_handle was NULL-checked by the caller.
            unsafe { *out_handle = handle };
            set_errno(0);
            0
        }
        Err(e) => fail(e),
    }
}

/// `tebako_fs_mount_from_file`.
///
/// # Safety
/// C ABI entry point: pointer arguments must follow the C contract.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_mount_from_file(
    archive_path: *const c_char,
    mount_point: *const c_char,
    out_handle: *mut libc::c_int,
) -> libc::c_int {
    // RO mount without an overlay (the historic behavior).
    tebako_fs_mount_from_file_with_mode(
        archive_path,
        mount_point,
        mount::TEBAKO_MOUNT_RO as libc::c_int,
        std::ptr::null(),
        out_handle,
    )
}

/// `tebako_fs_mount_from_file_with_mode` (spec 11 §3, additive): mount
/// with an explicit mode — TEBAKO_MOUNT_RO (0, the default),
/// TEBAKO_MOUNT_COW (1; `overlay_dir` required, created when missing),
/// TEBAKO_MOUNT_RW (2; ENOTSUP — no in-tree backend writes in place).
///
/// # Safety
/// C ABI entry point: pointer arguments must follow the C contract.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_mount_from_file_with_mode(
    archive_path: *const c_char,
    mount_point: *const c_char,
    mode: libc::c_int,
    overlay_dir: *const c_char,
    out_handle: *mut libc::c_int,
) -> libc::c_int {
    if out_handle.is_null() {
        return fail(libc::EINVAL);
    }
    let (archive_path, mount_point) = match (unsafe { path_arg(archive_path) }, unsafe {
        path_arg(mount_point)
    }) {
        (Ok(a), Ok(m)) => (a, m),
        _ => return fail(libc::EINVAL),
    };
    if mount_point.is_empty() {
        return fail(libc::EINVAL);
    }
    // Reading the image off the host is a host-passthrough decision (spec 08).
    if let Err(e) = context()
        .read()
        .unwrap()
        .host_check(archive_path, HostAccess::Ro)
    {
        return fail(e);
    }
    let mode = match parse_mount_mode(mode) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    let overlay_dir = match unsafe { optional_path_arg(overlay_dir) } {
        Ok(o) => o,
        Err(e) => return fail(e),
    };
    finish_mount(
        mount::build_from_file_with_mode(archive_path, mount_point, mode, overlay_dir),
        out_handle,
    )
}

/// `tebako_fs_mount_from_file_at`.
///
/// # Safety
/// C ABI entry point: pointer arguments must follow the C contract.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_mount_from_file_at(
    archive_path: *const c_char,
    offset: u64,
    length: u64,
    mount_point: *const c_char,
    out_handle: *mut libc::c_int,
) -> libc::c_int {
    // RO mount without an overlay (the historic behavior).
    tebako_fs_mount_from_file_at_with_mode(
        archive_path,
        offset,
        length,
        mount_point,
        mount::TEBAKO_MOUNT_RO as libc::c_int,
        std::ptr::null(),
        out_handle,
    )
}

/// `tebako_fs_mount_from_file_at_with_mode` (spec 11 §3, additive).
///
/// # Safety
/// C ABI entry point: pointer arguments must follow the C contract.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_mount_from_file_at_with_mode(
    archive_path: *const c_char,
    offset: u64,
    length: u64,
    mount_point: *const c_char,
    mode: libc::c_int,
    overlay_dir: *const c_char,
    out_handle: *mut libc::c_int,
) -> libc::c_int {
    if out_handle.is_null() {
        return fail(libc::EINVAL);
    }
    let (archive_path, mount_point) = match (unsafe { path_arg(archive_path) }, unsafe {
        path_arg(mount_point)
    }) {
        (Ok(a), Ok(m)) => (a, m),
        _ => return fail(libc::EINVAL),
    };
    if mount_point.is_empty() {
        return fail(libc::EINVAL);
    }
    // Reading the image off the host is a host-passthrough decision (spec 08).
    if let Err(e) = context()
        .read()
        .unwrap()
        .host_check(archive_path, HostAccess::Ro)
    {
        return fail(e);
    }
    let mode = match parse_mount_mode(mode) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    let overlay_dir = match unsafe { optional_path_arg(overlay_dir) } {
        Ok(o) => o,
        Err(e) => return fail(e),
    };
    finish_mount(
        mount::build_from_file_at_with_mode(
            archive_path,
            offset,
            length,
            mount_point,
            mode,
            overlay_dir,
        ),
        out_handle,
    )
}

/// `tebako_fs_mount_from_memory`.
///
/// # Safety
/// `data` must point to `size` readable bytes (the image is copied).
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_mount_from_memory(
    data: *const c_void,
    size: usize,
    mount_point: *const c_char,
    out_handle: *mut libc::c_int,
) -> libc::c_int {
    // RO mount without an overlay (the historic behavior).
    tebako_fs_mount_from_memory_with_mode(
        data,
        size,
        mount_point,
        mount::TEBAKO_MOUNT_RO as libc::c_int,
        std::ptr::null(),
        out_handle,
    )
}

/// `tebako_fs_mount_from_memory_with_mode` (spec 11 §3, additive).
///
/// # Safety
/// `data` must point to `size` readable bytes (the image is copied).
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_mount_from_memory_with_mode(
    data: *const c_void,
    size: usize,
    mount_point: *const c_char,
    mode: libc::c_int,
    overlay_dir: *const c_char,
    out_handle: *mut libc::c_int,
) -> libc::c_int {
    if out_handle.is_null() {
        return fail(libc::EINVAL);
    }
    if data.is_null() || size == 0 {
        return fail(libc::EINVAL);
    }
    let mount_point = match unsafe { path_arg(mount_point) } {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    if mount_point.is_empty() {
        return fail(libc::EINVAL);
    }
    let mode = match parse_mount_mode(mode) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    let overlay_dir = match unsafe { optional_path_arg(overlay_dir) } {
        Ok(o) => o,
        Err(e) => return fail(e),
    };
    let data = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    finish_mount(
        mount::build_from_memory_with_mode(data, mount_point, mode, overlay_dir),
        out_handle,
    )
}

/// `tebako_fs_unmount_handle`.
///
/// # Safety
/// C ABI entry point.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_unmount_handle(handle: libc::c_int) -> libc::c_int {
    match context().write().unwrap().unmount_handle(handle) {
        Ok(()) => {
            set_errno(0);
            0
        }
        Err(e) => fail(e),
    }
}

// ===================================================================
// Directory positioning
// ===================================================================

/// `tebako_fs_rewinddir`.
///
/// # Safety
/// `dir` must be a handle from tebako_fs_opendir.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_rewinddir(dir: *mut c_void) {
    if dir.is_null() {
        fail(libc::EBADF);
        return;
    }
    match context().write().unwrap().rewinddir(dir as usize) {
        Ok(()) => {
            set_errno(0);
        }
        Err(e) => {
            fail(e);
        }
    }
}

/// `tebako_fs_telldir`.
///
/// # Safety
/// `dir` must be a handle from tebako_fs_opendir.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_telldir(dir: *mut c_void) -> libc::c_long {
    if dir.is_null() {
        return fail(libc::EBADF) as libc::c_long;
    }
    match context().read().unwrap().telldir(dir as usize) {
        Ok(pos) => {
            set_errno(0);
            pos as libc::c_long
        }
        Err(e) => fail(e) as libc::c_long,
    }
}

/// `tebako_fs_seekdir` (index-based cookies).
///
/// # Safety
/// `dir` must be a handle from tebako_fs_opendir.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_seekdir(dir: *mut c_void, pos: libc::c_long) {
    if dir.is_null() {
        fail(libc::EBADF);
        return;
    }
    match context().write().unwrap().seekdir(dir as usize, pos) {
        Ok(()) => {
            set_errno(0);
        }
        Err(e) => {
            fail(e);
        }
    }
}

// ===================================================================
// Extraction
// ===================================================================

/// `tebako_fs_extract_all`.
///
/// # Safety
/// `dest_path` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_extract_all(dest_path: *const c_char) -> libc::c_int {
    let dest = match unsafe { path_arg(dest_path) } {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    match context()
        .write()
        .unwrap()
        .extract_all(std::path::Path::new(dest))
    {
        Ok(()) => {
            set_errno(0);
            0
        }
        Err(e) => fail(e),
    }
}

// ===================================================================
// Dynamic Loading Support
// ===================================================================

/// `tebako_fs_dlmap2file`: the returned string is heap-allocated with
/// libc `malloc` — the C contract says the caller releases it with `free()`.
///
/// # Safety
/// `path` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_dlmap2file(path: *const c_char) -> *mut c_char {
    let path = match unsafe { path_arg(path) } {
        Ok(p) => p,
        Err(e) => {
            fail(e);
            return std::ptr::null_mut();
        }
    };
    let host = match context().write().unwrap().dlmap2file(path) {
        Ok(h) => h,
        Err(e) => {
            fail(e);
            return std::ptr::null_mut();
        }
    };
    // Allocate with libc malloc so the C caller can free() the string.
    let bytes = host.as_bytes_with_nul();
    // SAFETY: malloc'd buffer of bytes.len(); copy then hand over ownership.
    let out = unsafe { libc::malloc(bytes.len()).cast::<c_char>() };
    if out.is_null() {
        fail(libc::ENOMEM);
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr().cast(), out, bytes.len()) };
    set_errno(0);
    out
}

// ===================================================================
// Host-Access Policy (Jails, spec 08)
// ===================================================================

/// `tebako_host_mount_t` from spec 08 §3: one host-mount grant
/// (docker `-v host:mount:access` semantics).
#[repr(C)]
pub struct TebakoHostMount {
    /// Host directory; realpath-canonicalized at bind time.
    pub host: *const c_char,
    /// Virtual mount point the host dir is exposed at; must be absolute.
    pub mount: *const c_char,
    /// Access grant: 0 = read-only, 1 = read-write.
    pub access: libc::c_int,
}

/// `tebako_fs_host_policy`: install the host-access policy (spec 08 §3),
/// replacing any previous one.
///
/// `default_open != 0` keeps today's pass-through as the namespace default;
/// `default_open == 0` denies every host path not covered by a mount grant
/// or an argument file. Mount sources and argument files are
/// realpath-canonicalized NOW (a missing one fails the call with its
/// errno); enforcement re-canonicalizes on each IO route, so symlinks
/// swapped in after this call resolve to their target and escapes fail.
///
/// Enforcement: the host-passthrough path of every IO route — open, stat,
/// opendir, dlmap2file, extract_all's destination, and the mount family's
/// image read — answers EPERM (denied) / EROFS (write against an ro grant)
/// instead of the ENOENT that tells the consumer to fall through to the
/// host fs. memfs mounts are unaffected. The policy is process state:
/// `tebako_fs_unmount` does NOT reset it (fail-closed). Install it after
/// the payload mounts are established — the mount family itself is
/// policy-gated once a policy is active.
///
/// # Safety
/// C ABI entry point: `mounts` must point to `n_mounts` valid entries and
/// `arg_files` to `n_arg_files` C-string pointers; either may be NULL when
/// its count is 0.
#[no_mangle]
pub unsafe extern "C" fn tebako_fs_host_policy(
    default_open: libc::c_int,
    mounts: *const TebakoHostMount,
    n_mounts: usize,
    arg_files: *const *const c_char,
    n_arg_files: usize,
) -> libc::c_int {
    if n_mounts > 0 && mounts.is_null() {
        return fail(libc::EINVAL);
    }
    if n_arg_files > 0 && arg_files.is_null() {
        return fail(libc::EINVAL);
    }
    // SAFETY: counts NULL-checked above; entries valid per the C contract.
    let mounts: &[TebakoHostMount] = if n_mounts == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(mounts, n_mounts) }
    };
    let mut specs = Vec::with_capacity(n_mounts);
    for m in mounts {
        let (host, mount) = match (unsafe { path_arg(m.host) }, unsafe { path_arg(m.mount) }) {
            (Ok(h), Ok(v)) => (h, v),
            _ => return fail(libc::EINVAL),
        };
        let access = match m.access {
            0 => HostAccess::Ro,
            1 => HostAccess::Rw,
            _ => return fail(libc::EINVAL),
        };
        specs.push(HostMountSpec {
            host: std::path::PathBuf::from(host),
            mount: mount.to_string(),
            access,
        });
    }
    // SAFETY: same contract as mounts.
    let arg_files: &[*const c_char] = if n_arg_files == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(arg_files, n_arg_files) }
    };
    let mut files = Vec::with_capacity(n_arg_files);
    for &f in arg_files {
        match unsafe { path_arg(f) } {
            Ok(f) => files.push(std::path::PathBuf::from(f)),
            Err(e) => return fail(e),
        }
    }
    let policy = match HostPolicy::bind(default_open != 0, specs, files) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    // The audit-journal source label: the installer names itself via
    // TEBAKO_JAIL_SOURCE (the bootstrap/dispatch surfaces label the
    // composition `manifest` / `user` / `manifest+user`); a bare C-ABI
    // install is attributed to the entry point itself. The journal file is
    // resolved and opened HERE, before the context guard — a denial under
    // the lock is then a bare write(2) (see `crate::journal`; a policy
    // that can never deny gets no file).
    let source = std::env::var("TEBAKO_JAIL_SOURCE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "tebako_fs_host_policy".to_string());
    let journal = if policy.never_denies() {
        None
    } else {
        crate::journal::open_journal()
    };
    context()
        .write()
        .unwrap()
        .set_host_policy(policy.with_source(source), journal);
    set_errno(0);
    0
}
