//! Zip-backed contract cases: ports of the C++ `CApiTest` suite from
//! libtfs `tests/test_c_api.cpp`, exercising the SAME fixture tree through
//! the Rust `tebako_fs_*` C ABI with identical expectations.
//!
//! All tests serialize on LOCK (the C API state is process-global), like
//! the C++ suite's RESOURCE_LOCK.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use tebako_contract_tests::{build_fixture_zip, TempDir};

/// Ported expectations use the same constants as the C API header.
const TEBAKO_FD_FLAG: i32 = 0x4000_0000;

static LOCK: Mutex<()> = Mutex::new(());

struct Fixture {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    archive_path: PathBuf,
    mount_point: String,
}

fn setup() -> Fixture {
    // NB: ignore poisoning — a panicked sibling must not cascade into every
    // later test (the state is process-global; each setup resets it anyway).
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Fresh state even if a previous test panicked mid-mount.
    unsafe { tfs::c_api::tebako_fs_unmount() };

    let tmp = TempDir::new("zip-c-api");
    let archive_path = tmp.0.join("test.zip");
    build_fixture_zip(&archive_path);

    Fixture {
        _guard: guard,
        _tmp: tmp,
        archive_path,
        mount_point: "/__tebako_test__".to_string(),
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe { tfs::c_api::tebako_fs_unmount() };
    }
}

impl Fixture {
    fn archive_c(&self) -> std::ffi::CString {
        std::ffi::CString::new(self.archive_path.to_str().unwrap()).unwrap()
    }

    fn mp_c(&self) -> std::ffi::CString {
        std::ffi::CString::new(self.mount_point.clone()).unwrap()
    }

    fn path_c(&self, suffix: &str) -> std::ffi::CString {
        std::ffi::CString::new(format!("{}{suffix}", self.mount_point)).unwrap()
    }

    fn init(&self) {
        let rc = unsafe {
            tfs::c_api::tebako_fs_init_from_file(self.archive_c().as_ptr(), self.mp_c().as_ptr())
        };
        assert_eq!(rc, 0, "init must succeed");
    }
}

// --- tiny FFI helpers ---------------------------------------------------

unsafe fn errno() -> i32 {
    unsafe { tfs::c_api::tebako_get_errno() }
}

unsafe fn is_initialized() -> i32 {
    unsafe { tfs::c_api::tebako_is_initialized() }
}

unsafe fn read_file_via_api(path: &std::ffi::CString) -> String {
    unsafe {
        let fd = tfs::c_api::tebako_fs_open(path.as_ptr(), libc::O_RDONLY);
        if fd < 0 {
            return String::new();
        }
        let mut buffer = vec![0u8; 4096];
        let n = tfs::c_api::tebako_fs_read(fd, buffer.as_mut_ptr().cast(), buffer.len());
        tfs::c_api::tebako_fs_close(fd);
        if n <= 0 {
            return String::new();
        }
        String::from_utf8_lossy(&buffer[..n as usize]).into_owned()
    }
}

// ===================================================================
// Lifecycle Tests (mirrors CApiTest lifecycle block)
// ===================================================================

#[test]
fn init_from_file_success() {
    let f = setup();
    f.init();
    assert_eq!(unsafe { is_initialized() }, 1);
    assert_eq!(unsafe { errno() }, 0);
}

#[test]
fn init_from_file_null_path() {
    let f = setup();
    let rc = unsafe { tfs::c_api::tebako_fs_init_from_file(std::ptr::null(), f.mp_c().as_ptr()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn init_from_file_null_mount_point() {
    let f = setup();
    let rc =
        unsafe { tfs::c_api::tebako_fs_init_from_file(f.archive_c().as_ptr(), std::ptr::null()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn init_from_file_nonexistent_file() {
    let f = setup();
    let bad = std::ffi::CString::new("/nonexistent/file.zip").unwrap();
    let rc = unsafe { tfs::c_api::tebako_fs_init_from_file(bad.as_ptr(), f.mp_c().as_ptr()) };
    assert_eq!(rc, -1);
    assert_ne!(unsafe { errno() }, 0);
}

#[test]
fn init_twice_fails_with_eexist() {
    let f = setup();
    f.init();
    let rc =
        unsafe { tfs::c_api::tebako_fs_init_from_file(f.archive_c().as_ptr(), f.mp_c().as_ptr()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EEXIST);
}

#[test]
fn unmount_cleans_up() {
    let f = setup();
    f.init();
    assert_eq!(unsafe { is_initialized() }, 1);
    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn unmount_idempotent() {
    let _f = setup();
    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(unsafe { is_initialized() }, 0);
    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn operations_fail_after_unmount() {
    let f = setup();
    f.init();
    unsafe { tfs::c_api::tebako_fs_unmount() };
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert_eq!(fd, -1);
    assert_eq!(unsafe { errno() }, libc::ENODEV);
}

// ===================================================================
// Memory Mounting Tests
// ===================================================================

#[test]
fn init_from_memory_success() {
    let f = setup();
    let data = std::fs::read(&f.archive_path).unwrap();
    let rc =
        unsafe { tfs::c_api::tebako_fs_init(data.as_ptr().cast(), data.len(), f.mp_c().as_ptr()) };
    assert_eq!(rc, 0);
    assert_eq!(unsafe { is_initialized() }, 1);

    let mp = unsafe { tfs::c_api::tebako_get_mount_point() };
    assert!(!mp.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(mp) }.to_str().unwrap(),
        f.mount_point
    );

    // Archive path is NULL (or empty) for memory mounts.
    let ap = unsafe { tfs::c_api::tebako_get_archive_path() };
    assert!(
        ap.is_null()
            || unsafe { std::ffi::CStr::from_ptr(ap) }
                .to_bytes()
                .is_empty()
    );

    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!bn.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_str().unwrap(),
        "ZIP"
    );
}

#[test]
fn init_from_memory_read_file() {
    let f = setup();
    let data = std::fs::read(&f.archive_path).unwrap();
    let rc =
        unsafe { tfs::c_api::tebako_fs_init(data.as_ptr().cast(), data.len(), f.mp_c().as_ptr()) };
    assert_eq!(rc, 0);
    let content = unsafe { read_file_via_api(&f.path_c("/content/hello.txt")) };
    assert_eq!(content, "Hello, World!");
}

#[test]
fn init_from_memory_invalid_data() {
    let f = setup();
    let bad = [0xFFu8; 6];
    let rc =
        unsafe { tfs::c_api::tebako_fs_init(bad.as_ptr().cast(), bad.len(), f.mp_c().as_ptr()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn init_from_memory_null_data() {
    let f = setup();
    let rc = unsafe { tfs::c_api::tebako_fs_init(std::ptr::null(), 100, f.mp_c().as_ptr()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn init_from_memory_zero_size() {
    let f = setup();
    let data = [0x50u8, 0x4B, 0x03, 0x04];
    let rc = unsafe { tfs::c_api::tebako_fs_init(data.as_ptr().cast(), 0, f.mp_c().as_ptr()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn init_from_memory_null_mount_point() {
    let _f = setup();
    let data = [0x50u8, 0x4B, 0x03, 0x04];
    let rc =
        unsafe { tfs::c_api::tebako_fs_init(data.as_ptr().cast(), data.len(), std::ptr::null()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

// ===================================================================
// File Operations Tests
// ===================================================================

#[test]
fn open_valid_file() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fd_is_embedded(fd) }, 1);
    assert_eq!(unsafe { errno() }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
}

#[test]
fn open_nonexistent_file() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(
            f.path_c("/content/nonexistent.txt").as_ptr(),
            libc::O_RDONLY,
        )
    };
    assert_eq!(fd, -1);
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

#[test]
fn open_null_path() {
    let f = setup();
    f.init();
    let fd = unsafe { tfs::c_api::tebako_fs_open(std::ptr::null(), libc::O_RDONLY) };
    assert_eq!(fd, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn open_write_mode_fails() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_WRONLY)
    };
    assert_eq!(fd, -1);
    assert_eq!(unsafe { errno() }, libc::EROFS);
}

#[test]
fn read_success() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    let mut buffer = [0u8; 100];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
    assert!(n > 0);
    assert_eq!(&buffer[..n as usize], b"Hello, World!");
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn read_empty_file() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/empty.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    let mut buffer = [0u8; 10];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
    assert_eq!(n, 0); // EOF immediately
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn read_invalid_fd() {
    let _f = setup();
    let mut buffer = [0u8; 10];
    let n = unsafe { tfs::c_api::tebako_fs_read(999, buffer.as_mut_ptr().cast(), buffer.len()) };
    assert_eq!(n, -1);
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn read_null_buffer() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, std::ptr::null_mut(), 10) };
    assert_eq!(n, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn lseek_seek_set() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    let pos = unsafe { tfs::c_api::tebako_fs_lseek(fd, 7, libc::SEEK_SET) };
    assert_eq!(pos, 7);
    let mut buffer = [0u8; 10];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, buffer.as_mut_ptr().cast(), 5) };
    assert_eq!(n, 5);
    assert_eq!(&buffer[..5], b"World");
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn lseek_seek_cur() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    let mut buffer = [0u8; 5];
    unsafe { tfs::c_api::tebako_fs_read(fd, buffer.as_mut_ptr().cast(), 5) };
    let pos = unsafe { tfs::c_api::tebako_fs_lseek(fd, 2, libc::SEEK_CUR) };
    assert_eq!(pos, 7);
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn lseek_seek_end() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    let size = unsafe { tfs::c_api::tebako_fs_lseek(fd, 0, libc::SEEK_END) };
    assert_eq!(size, 13); // strlen("Hello, World!")
    let mut buffer = [0u8; 10];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
    assert_eq!(n, 0);
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn lseek_invalid_fd() {
    let _f = setup();
    let pos = unsafe { tfs::c_api::tebako_fs_lseek(999, 0, libc::SEEK_SET) };
    assert_eq!(pos, -1);
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn close_success() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
    let mut buffer = [0u8; 10];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
    assert_eq!(n, -1);
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn close_invalid_fd() {
    let _f = setup();
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(999) }, -1);
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn multiple_fds_independent() {
    let f = setup();
    f.init();
    let fd1 = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    let fd2 = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/data.bin").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd1 > 0 && fd2 > 0);
    assert_ne!(fd1, fd2);
    let mut buf1 = [0u8; 10];
    let mut buf2 = [0u8; 10];
    assert!(unsafe { tfs::c_api::tebako_fs_read(fd1, buf1.as_mut_ptr().cast(), 10) } > 0);
    assert!(unsafe { tfs::c_api::tebako_fs_read(fd2, buf2.as_mut_ptr().cast(), 10) } > 0);
    unsafe {
        tfs::c_api::tebako_fs_close(fd1);
        tfs::c_api::tebako_fs_close(fd2);
    }
}

// ===================================================================
// Directory Operations Tests
// ===================================================================

#[test]
fn opendir_success() {
    let f = setup();
    f.init();
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content").as_ptr()) };
    assert!(!dir.is_null());
    assert_eq!(unsafe { errno() }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);
}

#[test]
fn opendir_nonexistent() {
    let f = setup();
    f.init();
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/nonexistent").as_ptr()) };
    assert!(dir.is_null());
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

#[test]
fn opendir_null_path() {
    let f = setup();
    f.init();
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(std::ptr::null()) };
    assert!(dir.is_null());
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn readdir_list_files() {
    let f = setup();
    f.init();
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content").as_ptr()) };
    assert!(!dir.is_null());

    let mut entries = Vec::new();
    loop {
        let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        entries.push(name);
    }

    assert!(!entries.is_empty());
    let has = |n: &str| entries.iter().any(|e| e == n);
    assert!(
        has("hello.txt") || has("data.bin") || has("subdir"),
        "expected test files not found: {entries:?}"
    );
    assert!(has("hello.txt") && has("data.bin") && has("subdir") && has("empty.txt"));
    unsafe { tfs::c_api::tebako_fs_closedir(dir) };
}

#[test]
fn readdir_check_types() {
    let f = setup();
    f.init();
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content").as_ptr()) };
    assert!(!dir.is_null());

    let mut found_file = false;
    loop {
        let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        if unsafe { (*entry).d_type } == tfs::DT_REG {
            found_file = true;
        }
        if unsafe { (*entry).d_type } == tfs::DT_DIR {
            // subdir must be reported as a directory
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            if name.to_bytes() == b"subdir" {
                // good
            }
        }
    }
    assert!(found_file, "should find at least one regular file");
    unsafe { tfs::c_api::tebako_fs_closedir(dir) };
}

#[test]
fn readdir_empty_at_end() {
    let f = setup();
    f.init();
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content").as_ptr()) };
    assert!(!dir.is_null());
    while !unsafe { tfs::c_api::tebako_fs_readdir(dir) }.is_null() {}
    assert!(unsafe { tfs::c_api::tebako_fs_readdir(dir) }.is_null());
    unsafe { tfs::c_api::tebako_fs_closedir(dir) };
}

#[test]
fn readdir_invalid_handle() {
    let _f = setup();
    let entry = unsafe { tfs::c_api::tebako_fs_readdir(std::ptr::null_mut()) };
    assert!(entry.is_null());
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn closedir_success() {
    let f = setup();
    f.init();
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content").as_ptr()) };
    assert!(!dir.is_null());
    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);
    assert_eq!(unsafe { errno() }, 0);
}

#[test]
fn closedir_invalid_handle() {
    let _f = setup();
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_closedir(std::ptr::null_mut()) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn multiple_dirs_independent() {
    let f = setup();
    f.init();
    let dir1 = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content").as_ptr()) };
    let dir2 = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content/subdir").as_ptr()) };
    assert!(!dir1.is_null() && !dir2.is_null());
    assert_ne!(dir1, dir2);
    assert!(!unsafe { tfs::c_api::tebako_fs_readdir(dir1) }.is_null());
    unsafe {
        tfs::c_api::tebako_fs_closedir(dir1);
        tfs::c_api::tebako_fs_closedir(dir2);
    }
}

// ===================================================================
// Metadata Operations Tests
// ===================================================================

#[test]
fn stat_regular_file() {
    let f = setup();
    f.init();
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc =
        unsafe { tfs::c_api::tebako_fs_stat(f.path_c("/content/hello.txt").as_ptr(), &mut st) };
    assert_eq!(rc, 0);
    assert_ne!(st.st_mode & libc::S_IFMT, libc::S_IFDIR);
    assert_eq!(st.st_mode & libc::S_IFMT, libc::S_IFREG);
    assert_eq!(st.st_size, 13); // strlen("Hello, World!")
}

#[test]
fn stat_directory() {
    let f = setup();
    f.init();
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tfs::c_api::tebako_fs_stat(f.path_c("/content/subdir").as_ptr(), &mut st) };
    assert_eq!(rc, 0);
    assert_eq!(st.st_mode & libc::S_IFMT, libc::S_IFDIR);
}

#[test]
fn stat_nonexistent() {
    let f = setup();
    f.init();
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tfs::c_api::tebako_fs_stat(f.path_c("/nonexistent").as_ptr(), &mut st) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

#[test]
fn stat_null_arguments() {
    let f = setup();
    f.init();
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tfs::c_api::tebako_fs_stat(std::ptr::null(), &mut st) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    let rc = unsafe {
        tfs::c_api::tebako_fs_stat(
            f.path_c("/content/hello.txt").as_ptr(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn fstat_success() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { tfs::c_api::tebako_fs_fstat(fd, &mut st) }, 0);
    assert_eq!(st.st_mode & libc::S_IFMT, libc::S_IFREG);
    assert_eq!(st.st_size, 13);
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn fstat_invalid_fd() {
    let _f = setup();
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { tfs::c_api::tebako_fs_fstat(999, &mut st) }, -1);
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn fstat_null_stat() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_fstat(fd, std::ptr::null_mut()) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

// ===================================================================
// Path Detection Tests
// ===================================================================

#[test]
fn path_is_embedded_valid_paths() {
    let f = setup();
    f.init();
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(f.path_c("/content/hello.txt").as_ptr()) },
        1
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(f.path_c("/any/path").as_ptr()) },
        1
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(f.mp_c().as_ptr()) },
        1
    );
}

#[test]
fn path_is_embedded_external_paths() {
    let f = setup();
    f.init();
    let p1 = std::ffi::CString::new("/tmp/file.txt").unwrap();
    let p2 = std::ffi::CString::new("/usr/bin/ls").unwrap();
    let p3 = std::ffi::CString::new("relative/path.txt").unwrap();
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(p1.as_ptr()) },
        0
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(p2.as_ptr()) },
        0
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(p3.as_ptr()) },
        0
    );
}

#[test]
fn path_is_embedded_null_path() {
    let _f = setup();
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(std::ptr::null()) },
        0
    );
}

#[test]
fn path_is_embedded_not_initialized() {
    let f = setup(); // NB: no init()
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(f.path_c("/file.txt").as_ptr()) },
        0
    );
}

#[test]
fn mount_of_covered_paths() {
    let f = setup();
    f.init();
    let p = unsafe { tfs::c_api::tebako_fs_mount_of(f.path_c("/content/hello.txt").as_ptr()) };
    assert!(!p.is_null());
    assert_eq!(unsafe { errno() }, 0);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy(),
        f.mount_point
    );
    unsafe { libc::free(p.cast()) };

    // The mount root itself is covered.
    let p = unsafe { tfs::c_api::tebako_fs_mount_of(f.mp_c().as_ptr()) };
    assert!(!p.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy(),
        f.mount_point
    );
    unsafe { libc::free(p.cast()) };
}

#[test]
fn mount_of_uncovered_path() {
    let f = setup();
    f.init();
    let p1 = std::ffi::CString::new("/tmp/file.txt").unwrap();
    let p = unsafe { tfs::c_api::tebako_fs_mount_of(p1.as_ptr()) };
    assert!(p.is_null());
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

#[test]
fn mount_of_null_path() {
    let _f = setup();
    let p = unsafe { tfs::c_api::tebako_fs_mount_of(std::ptr::null()) };
    assert!(p.is_null());
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn fd_is_embedded_valid_fd() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fd_is_embedded(fd) }, 1);
    assert_eq!(
        unsafe { tfs::c_api::tebako_fd_is_embedded(libc::STDOUT_FILENO) },
        0
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_fd_is_embedded(libc::STDIN_FILENO) },
        0
    );
    unsafe { tfs::c_api::tebako_fs_close(fd) };
}

#[test]
fn fd_is_embedded_flag_check() {
    let _f = setup();
    let fake_fd = 123 | TEBAKO_FD_FLAG;
    assert_eq!(unsafe { tfs::c_api::tebako_fd_is_embedded(fake_fd) }, 1);
    assert_eq!(unsafe { tfs::c_api::tebako_fd_is_embedded(123) }, 0);
}

// ===================================================================
// Error Handling Tests
// ===================================================================

#[test]
fn get_errno_thread_local() {
    let f = setup();
    unsafe { tfs::c_api::tebako_fs_open(std::ptr::null(), libc::O_RDONLY) };
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    f.init();
    unsafe { tfs::c_api::tebako_fs_open(f.path_c("/nonexistent").as_ptr(), libc::O_RDONLY) };
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

#[test]
fn strerror_valid_codes() {
    let msg = unsafe { tfs::c_api::tebako_strerror(libc::ENOENT) };
    assert!(!msg.is_null());
    assert!(!unsafe { std::ffi::CStr::from_ptr(msg) }
        .to_bytes()
        .is_empty());
    let msg = unsafe { tfs::c_api::tebako_strerror(libc::EINVAL) };
    assert!(!msg.is_null());
}

#[test]
fn strerror_do_not_free() {
    let msg1 = unsafe { tfs::c_api::tebako_strerror(libc::ENOENT) };
    let msg2 = unsafe { tfs::c_api::tebako_strerror(libc::ENOENT) };
    assert_eq!(msg1, msg2); // same static storage
}

// ===================================================================
// Utility Functions Tests
// ===================================================================

#[test]
fn get_mount_point_success() {
    let f = setup();
    f.init();
    let mp = unsafe { tfs::c_api::tebako_get_mount_point() };
    assert!(!mp.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(mp) }.to_str().unwrap(),
        f.mount_point
    );
}

#[test]
fn get_mount_point_not_mounted() {
    let _f = setup(); // no init
    assert!(unsafe { tfs::c_api::tebako_get_mount_point() }.is_null());
}

#[test]
fn get_backend_name_success() {
    let f = setup();
    f.init();
    let name = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!name.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(name) }.to_str().unwrap(),
        "ZIP"
    );
}

// ===================================================================
// Integration Tests
// ===================================================================

#[test]
fn integration_read_nested_file() {
    let f = setup();
    f.init();
    let content = unsafe { read_file_via_api(&f.path_c("/content/subdir/nested.txt")) };
    assert_eq!(content, "Nested file content");
}

#[test]
fn integration_full_workflow() {
    let f = setup();
    f.init();
    assert_eq!(unsafe { is_initialized() }, 1);
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(f.path_c("/content").as_ptr()) },
        1
    );

    let dir = unsafe { tfs::c_api::tebako_fs_opendir(f.path_c("/content").as_ptr()) };
    assert!(!dir.is_null());

    let mut file_count = 0;
    loop {
        let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        file_count += 1;
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let full = std::ffi::CString::new(format!("{}/content/{name}", f.mount_point)).unwrap();
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { tfs::c_api::tebako_fs_stat(full.as_ptr(), &mut st) },
            0,
            "stat {name}"
        );
        if unsafe { (*entry).d_type } == tfs::DT_REG {
            let fd = unsafe { tfs::c_api::tebako_fs_open(full.as_ptr(), libc::O_RDONLY) };
            assert!(fd > 0, "open {name}");
            unsafe { tfs::c_api::tebako_fs_close(fd) };
        }
    }
    assert!(file_count > 0);
    unsafe { tfs::c_api::tebako_fs_closedir(dir) };

    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(unsafe { is_initialized() }, 0);
}

// ===================================================================
// Region mount + format-stub tests (Rust-milestone additions)
// ===================================================================

#[test]
fn init_from_file_at_whole_file_and_embedded() {
    let f = setup();
    let zip = std::fs::read(&f.archive_path).unwrap();

    // Whole file as a region (offset=0,length=0 fast path AND explicit length).
    let len = zip.len() as u64;
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(f.archive_c().as_ptr(), 0, len, f.mp_c().as_ptr())
    };
    assert_eq!(rc, 0);
    let content = unsafe { read_file_via_api(&f.path_c("/content/hello.txt")) };
    assert_eq!(content, "Hello, World!");
    unsafe { tfs::c_api::tebako_fs_unmount() };

    // The same zip embedded behind a junk prefix.
    let mut embedded = vec![0xABu8; 4096];
    embedded.extend_from_slice(&zip);
    let embedded_path = f._tmp.0.join("embedded.bin");
    std::fs::write(&embedded_path, &embedded).unwrap();
    let embedded_c = std::ffi::CString::new(embedded_path.to_str().unwrap()).unwrap();

    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(embedded_c.as_ptr(), 4096, len, f.mp_c().as_ptr())
    };
    assert_eq!(rc, 0, "mount embedded region");
    let content = unsafe { read_file_via_api(&f.path_c("/content/subdir/nested.txt")) };
    assert_eq!(content, "Nested file content");
}

#[test]
fn init_from_file_at_bad_region() {
    let f = setup();
    let len = std::fs::metadata(&f.archive_path).unwrap().len();
    // Past EOF.
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            f.archive_c().as_ptr(),
            len + 100,
            0,
            f.mp_c().as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    // Exceeds file size.
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(f.archive_c().as_ptr(), 10, len, f.mp_c().as_ptr())
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn dwarfs_and_unknown_formats_fail_cleanly() {
    let f = setup();
    // Squashfs-magic garbage (backend is REAL since milestone 3): the mount
    // is attempted and fails in the superblock read -> EIO, not a crash.
    let mut fake_squashfs = b"hsqs".to_vec();
    fake_squashfs.extend_from_slice(&[0u8; 64]);
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(
            fake_squashfs.as_ptr().cast(),
            fake_squashfs.len(),
            f.mp_c().as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EIO);
    assert_eq!(unsafe { is_initialized() }, 0);

    // Dwarfs-magic garbage (backend is REAL since milestone 2): the mount
    // is attempted and fails in the image parser -> EIO, not a crash.
    let mut fake_dwarfs = b"DWARFS".to_vec();
    fake_dwarfs.extend_from_slice(&[0u8; 64]);
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(
            fake_dwarfs.as_ptr().cast(),
            fake_dwarfs.len(),
            f.mp_c().as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EIO);
    assert_eq!(unsafe { is_initialized() }, 0);

    // Truly unknown magic -> EINVAL.
    let junk = [0xFFu8; 64];
    let rc =
        unsafe { tfs::c_api::tebako_fs_init(junk.as_ptr().cast(), junk.len(), f.mp_c().as_ptr()) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn pread_does_not_move_position() {
    let f = setup();
    f.init();
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(f.path_c("/content/hello.txt").as_ptr(), libc::O_RDONLY)
    };
    assert!(fd > 0);

    // pread at offset 7, then a sequential read must still start at 0.
    let mut buf = [0u8; 5];
    let n = unsafe { tfs::c_api::tebako_fs_pread(fd, buf.as_mut_ptr().cast(), 5, 7) };
    assert_eq!(n, 5);
    assert_eq!(&buf, b"World");

    let mut buf2 = [0u8; 5];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, buf2.as_mut_ptr().cast(), 5) };
    assert_eq!(n, 5);
    assert_eq!(&buf2, b"Hello");

    // pread at EOF -> 0; negative offset -> EINVAL.
    let n = unsafe { tfs::c_api::tebako_fs_pread(fd, buf2.as_mut_ptr().cast(), 1, 13) };
    assert_eq!(n, 0);
    let n = unsafe { tfs::c_api::tebako_fs_pread(fd, buf2.as_mut_ptr().cast(), 1, -1) };
    assert_eq!(n, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);

    unsafe { tfs::c_api::tebako_fs_close(fd) };
}
