//! SquashFS backend contract cases: ports of the C++ `SquashFSBackendTest`
//! / `SquashFSBackendMountedTest` cases (libtfs
//! `tests/test_squashfs_backend.cpp`) that map to the C ABI, against the
//! same fixture images (libtfs `tests/fixtures/squashfs/*.sqfs`, borrowed):
//!
//! - simple.sqfs — test.txt "Hello from SquashFS!\n" (21B),
//!   file2.txt "Second file\n" (12B)
//! - nested.sqfs — dir1/file1.txt "File 1\n", dir1/subdir/file2.txt
//!   "File 2\n", dir2/file3.txt "File 3\n"
//! - empty.sqfs — empty_file.txt (0B), empty_dir/
//! - permissions.sqfs — readonly.txt (444), script.sh (755),
//!   private.txt (600), restricted_dir/ (700)
//! - corrupted.sqfs — damaged image

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use tebako_contract_tests::TempDir;

static LOCK: Mutex<()> = Mutex::new(());

struct F {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    mount_point: String,
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}

fn setup() -> F {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { tfs::c_api::tebako_fs_unmount() };
    F {
        _guard: guard,
        _tmp: TempDir::new("sqfs"),
        mount_point: "/__sqfs_test__".to_string(),
    }
}

impl Drop for F {
    fn drop(&mut self) {
        unsafe { tfs::c_api::tebako_fs_unmount() };
    }
}

fn c(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).unwrap()
}

unsafe fn errno() -> i32 {
    unsafe { tfs::c_api::tebako_get_errno() }
}

fn p(f: &F, suffix: &str) -> std::ffi::CString {
    c(&format!("{}{suffix}", f.mount_point))
}

fn init_file(f: &F, image: &std::path::Path) {
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(image.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0, "mount {}", image.display());
}

fn read_file(f: &F, suffix: &str) -> String {
    unsafe {
        let fd = tfs::c_api::tebako_fs_open(p(f, suffix).as_ptr(), libc::O_RDONLY);
        assert!(fd > 0, "open {suffix}");
        let mut buf = vec![0u8; 8192];
        let n = tfs::c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast(), buf.len());
        assert!(n >= 0, "read {suffix}");
        tfs::c_api::tebako_fs_close(fd);
        String::from_utf8_lossy(&buf[..n as usize]).into_owned()
    }
}

fn stat(f: &F, suffix: &str) -> libc::stat {
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tfs::c_api::tebako_fs_stat(p(f, suffix).as_ptr(), &mut st) };
    assert_eq!(rc, 0, "stat {suffix}");
    st
}

fn readdir_names(f: &F, suffix: &str) -> Vec<(String, u8)> {
    unsafe {
        let dir = tfs::c_api::tebako_fs_opendir(p(f, suffix).as_ptr());
        assert!(!dir.is_null(), "opendir {suffix}");
        let mut out = Vec::new();
        loop {
            let entry = tfs::c_api::tebako_fs_readdir(dir);
            if entry.is_null() {
                break;
            }
            let name = std::ffi::CStr::from_ptr((*entry).d_name.as_ptr())
                .to_string_lossy()
                .into_owned();
            out.push((name, (*entry).d_type));
        }
        assert_eq!(tfs::c_api::tebako_fs_closedir(dir), 0);
        out
    }
}

// ===================================================================
// Mount / identity (ports MountValidArchiveSucceeds, BackendInfoCorrect,
// MountCorruptedArchiveFails, MountNonexistentArchiveFails)
// ===================================================================

#[test]
fn mount_simple_backend_name_and_getters() {
    let f = setup();
    let image = fixture("simple.sqfs");
    init_file(&f, &image);

    assert_eq!(unsafe { tfs::c_api::tebako_is_initialized() }, 1);
    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!bn.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"SquashFS"
    );

    let mp = unsafe { tfs::c_api::tebako_get_mount_point() };
    assert!(!mp.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(mp) }.to_bytes(),
        f.mount_point.as_bytes()
    );

    let ap = unsafe { tfs::c_api::tebako_get_archive_path() };
    assert!(!ap.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(ap) }.to_str().unwrap(),
        image.to_str().unwrap()
    );
}

#[test]
fn mount_nonexistent_fails() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c("/nonexistent/image.sqfs").as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_ne!(unsafe { errno() }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_is_initialized() }, 0);
}

#[test]
fn mount_corrupted_fails() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(fixture("corrupted.sqfs").to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_ne!(unsafe { errno() }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_is_initialized() }, 0);
}

// ===================================================================
// File reading (ports ReadFileContentsCorrect, ReadIncrementsPosition,
// ReadSetsEofFlag, SeekSet/Cur/End, SeekBeyondBoundsFails,
// CloseReleasesResource, OperationsAfterCloseFail)
// ===================================================================

#[test]
fn read_file_contents_correct() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    assert_eq!(read_file(&f, "/test.txt"), "Hello from SquashFS!\n");
    assert_eq!(read_file(&f, "/file2.txt"), "Second file\n");
}

#[test]
fn read_increments_position_and_eof() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    unsafe {
        let fd = tfs::c_api::tebako_fs_open(p(&f, "/test.txt").as_ptr(), libc::O_RDONLY);
        assert!(fd > 0);
        let mut buf = [0u8; 6];
        let n = tfs::c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast(), 6);
        assert_eq!(n, 6);
        assert_eq!(&buf, b"Hello ");
        let n = tfs::c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast(), 6);
        assert_eq!(n, 6);
        assert_eq!(&buf, b"from S");
        // drain to EOF, then 0
        let mut big = [0u8; 64];
        let n = tfs::c_api::tebako_fs_read(fd, big.as_mut_ptr().cast(), 64);
        assert!(n >= 0);
        let n2 = tfs::c_api::tebako_fs_read(fd, big.as_mut_ptr().cast(), 64);
        assert_eq!(n2, 0);
        assert_eq!(tfs::c_api::tebako_fs_close(fd), 0);
    }
}

#[test]
fn seek_positions_and_bounds() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    unsafe {
        let fd = tfs::c_api::tebako_fs_open(p(&f, "/test.txt").as_ptr(), libc::O_RDONLY);
        assert!(fd > 0);

        // SEEK_SET: "Hello from SquashFS!\n" offset 6 -> "from SquashFS!\n"
        assert_eq!(tfs::c_api::tebako_fs_lseek(fd, 6, libc::SEEK_SET), 6);
        let mut buf = [0u8; 4];
        assert_eq!(
            tfs::c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast(), 4),
            4
        );
        assert_eq!(&buf, b"from");

        // SEEK_END gives the size (21), SEEK_CUR advances.
        assert_eq!(tfs::c_api::tebako_fs_lseek(fd, 0, libc::SEEK_END), 21);
        assert_eq!(tfs::c_api::tebako_fs_lseek(fd, 0, libc::SEEK_SET), 0);
        assert_eq!(tfs::c_api::tebako_fs_lseek(fd, 3, libc::SEEK_CUR), 3);

        // Beyond bounds fails with EINVAL and leaves position untouched.
        assert_eq!(tfs::c_api::tebako_fs_lseek(fd, 22, libc::SEEK_SET), -1);
        assert_eq!(errno(), libc::EINVAL);
        assert_eq!(tfs::c_api::tebako_fs_lseek(fd, -30, libc::SEEK_CUR), -1);
        assert_eq!(errno(), libc::EINVAL);

        assert_eq!(tfs::c_api::tebako_fs_close(fd), 0);
    }
}

#[test]
fn pread_native_seeking() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    unsafe {
        let fd = tfs::c_api::tebako_fs_open(p(&f, "/test.txt").as_ptr(), libc::O_RDONLY);
        assert!(fd > 0);
        // SquashFS supports native seeking; pread at 6 reads "from ".
        let mut buf = [0u8; 5];
        assert_eq!(
            tfs::c_api::tebako_fs_pread(fd, buf.as_mut_ptr().cast(), 5, 6),
            5
        );
        assert_eq!(&buf, b"from ");
        // Position untouched.
        assert_eq!(
            tfs::c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast(), 5),
            5
        );
        assert_eq!(&buf, b"Hello");
        assert_eq!(tfs::c_api::tebako_fs_close(fd), 0);
    }
}

#[test]
fn operations_after_close_fail() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    unsafe {
        let fd = tfs::c_api::tebako_fs_open(p(&f, "/test.txt").as_ptr(), libc::O_RDONLY);
        assert!(fd > 0);
        assert_eq!(tfs::c_api::tebako_fs_close(fd), 0);
        let mut buf = [0u8; 8];
        assert_eq!(
            tfs::c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast(), 8),
            -1
        );
        assert_eq!(errno(), libc::EBADF);
        assert_eq!(tfs::c_api::tebako_fs_close(fd), -1);
        assert_eq!(errno(), libc::EBADF);
    }
}

#[test]
fn multiple_fds_read_independently() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    unsafe {
        let fd1 = tfs::c_api::tebako_fs_open(p(&f, "/test.txt").as_ptr(), libc::O_RDONLY);
        let fd2 = tfs::c_api::tebako_fs_open(p(&f, "/file2.txt").as_ptr(), libc::O_RDONLY);
        assert!(fd1 > 0 && fd2 > 0 && fd1 != fd2);
        let mut b1 = [0u8; 5];
        let mut b2 = [0u8; 6];
        assert_eq!(
            tfs::c_api::tebako_fs_read(fd1, b1.as_mut_ptr().cast(), 5),
            5
        );
        assert_eq!(
            tfs::c_api::tebako_fs_read(fd2, b2.as_mut_ptr().cast(), 6),
            6
        );
        assert_eq!(&b1, b"Hello");
        assert_eq!(&b2, b"Second");
        tfs::c_api::tebako_fs_close(fd1);
        tfs::c_api::tebako_fs_close(fd2);
    }
}

// ===================================================================
// stat / metadata (ports FileSizeCorrect, ModificationTimeNonZero,
// PermissionsPreservedCorrectly, FileSizeInvalidFileReturnsError)
// ===================================================================

#[test]
fn stat_regular_file_and_dir() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    let st = stat(&f, "/test.txt");
    assert_eq!(st.st_mode & libc::S_IFMT, libc::S_IFREG as _);
    assert_eq!(st.st_size, 21);
    let st_dir = stat(&f, "/");
    assert_eq!(st_dir.st_mode & libc::S_IFMT, libc::S_IFDIR as _);
}

#[test]
fn permissions_preserved() {
    let f = setup();
    init_file(&f, &fixture("permissions.sqfs"));
    assert_eq!(stat(&f, "/readonly.txt").st_mode & 0o777, 0o444);
    assert_eq!(stat(&f, "/script.sh").st_mode & 0o777, 0o755);
    assert_eq!(stat(&f, "/private.txt").st_mode & 0o777, 0o600);
    let st = stat(&f, "/restricted_dir");
    assert_eq!(st.st_mode & libc::S_IFMT, libc::S_IFDIR as _);
    assert_eq!(st.st_mode & 0o777, 0o700);
}

#[test]
fn modification_time_nonzero_and_error_paths() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    assert!(stat(&f, "/test.txt").st_mtime > 0);

    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tfs::c_api::tebako_fs_stat(p(&f, "/nonexistent").as_ptr(), &mut st) };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

#[test]
fn pread_on_dir_eisdir_and_opendir_on_file_enotdir() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    unsafe {
        let mut buf = [0u8; 4];
        let fd = tfs::c_api::tebako_fs_open(p(&f, "/").as_ptr(), libc::O_RDONLY);
        assert_eq!(fd, -1);
        assert_eq!(errno(), libc::EISDIR);

        let dir = tfs::c_api::tebako_fs_opendir(p(&f, "/test.txt").as_ptr());
        assert!(dir.is_null());
        assert_eq!(errno(), libc::ENOTDIR);

        let _ = &mut buf;
    }
}

// ===================================================================
// Directory listing (ports ListDirectoryReturnsAllEntries,
// DirectoryEntryHasCorrectMetadata, IteratorResetWorks,
// ListNestedDirectoryWorks, ListEmptyDirectoryReturnsNoEntries)
// ===================================================================

#[test]
fn list_root_and_types() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    let entries = readdir_names(&f, "/");
    assert!(
        entries
            .iter()
            .any(|(n, t)| n == "test.txt" && *t == tfs::DT_REG),
        "{entries:?}"
    );
    assert!(
        entries
            .iter()
            .any(|(n, t)| n == "file2.txt" && *t == tfs::DT_REG),
        "{entries:?}"
    );
    assert!(!entries.iter().any(|(n, _)| n == "." || n == ".."));
}

#[test]
fn list_nested_directory() {
    let f = setup();
    init_file(&f, &fixture("nested.sqfs"));
    assert_eq!(read_file(&f, "/dir1/file1.txt"), "File 1\n");
    assert_eq!(read_file(&f, "/dir1/subdir/file2.txt"), "File 2\n");
    assert_eq!(read_file(&f, "/dir2/file3.txt"), "File 3\n");

    let dir1 = readdir_names(&f, "/dir1");
    assert!(
        dir1.iter()
            .any(|(n, t)| n == "file1.txt" && *t == tfs::DT_REG),
        "{dir1:?}"
    );
    assert!(
        dir1.iter().any(|(n, t)| n == "subdir" && *t == tfs::DT_DIR),
        "{dir1:?}"
    );

    let subdir = readdir_names(&f, "/dir1/subdir");
    assert_eq!(subdir.len(), 1);
    assert_eq!(subdir[0].0, "file2.txt");

    let root = readdir_names(&f, "/");
    assert!(
        root.iter().any(|(n, t)| n == "dir1" && *t == tfs::DT_DIR),
        "{root:?}"
    );
    assert!(
        root.iter().any(|(n, t)| n == "dir2" && *t == tfs::DT_DIR),
        "{root:?}"
    );
}

#[test]
fn empty_image_cases() {
    let f = setup();
    init_file(&f, &fixture("empty.sqfs"));

    // Empty file: stat size 0, read EOF immediately.
    let st = stat(&f, "/empty_file.txt");
    assert_eq!(st.st_size, 0);
    unsafe {
        let fd = tfs::c_api::tebako_fs_open(p(&f, "/empty_file.txt").as_ptr(), libc::O_RDONLY);
        assert!(fd > 0);
        let mut buf = [0u8; 8];
        assert_eq!(
            tfs::c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast(), 8),
            0
        );
        tfs::c_api::tebako_fs_close(fd);
    }

    // Empty directory lists no entries.
    let entries = readdir_names(&f, "/empty_dir");
    assert!(entries.is_empty(), "{entries:?}");

    // rewinddir on an empty dir keeps telldir at 0.
    unsafe {
        let dir = tfs::c_api::tebako_fs_opendir(p(&f, "/empty_dir").as_ptr());
        assert!(!dir.is_null());
        assert_eq!(tfs::c_api::tebako_fs_telldir(dir), 0);
        tfs::c_api::tebako_fs_rewinddir(dir);
        assert_eq!(tfs::c_api::tebako_fs_telldir(dir), 0);
        assert!(tfs::c_api::tebako_fs_readdir(dir).is_null());
        assert_eq!(tfs::c_api::tebako_fs_closedir(dir), 0);
    }
}

#[test]
fn rewinddir_resets_listing() {
    let f = setup();
    init_file(&f, &fixture("simple.sqfs"));
    unsafe {
        let dir = tfs::c_api::tebako_fs_opendir(p(&f, "/").as_ptr());
        assert!(!dir.is_null());
        let first = tfs::c_api::tebako_fs_readdir(dir);
        assert!(!first.is_null());
        let name0 = std::ffi::CStr::from_ptr((*first).d_name.as_ptr())
            .to_string_lossy()
            .into_owned();
        tfs::c_api::tebako_fs_rewinddir(dir);
        assert_eq!(tfs::c_api::tebako_fs_telldir(dir), 0);
        let again = tfs::c_api::tebako_fs_readdir(dir);
        assert!(!again.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr((*again).d_name.as_ptr()).to_string_lossy(),
            name0
        );
        assert_eq!(tfs::c_api::tebako_fs_closedir(dir), 0);
    }
}

// ===================================================================
// Memory + region mounts (ports MountFromMemory equivalents)
// ===================================================================

#[test]
fn memory_mount_reads() {
    let f = setup();
    let data = std::fs::read(fixture("simple.sqfs")).unwrap();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(data.as_ptr().cast(), data.len(), c(&f.mount_point).as_ptr())
    };
    assert_eq!(rc, 0);
    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"SquashFS"
    );
    assert!(unsafe { tfs::c_api::tebako_get_archive_path() }.is_null());
    assert_eq!(read_file(&f, "/test.txt"), "Hello from SquashFS!\n");
}

#[test]
fn region_mount_embedded() {
    let f = setup();
    let image = std::fs::read(fixture("simple.sqfs")).unwrap();
    let junk: u64 = 1000;
    let mut combined = vec![0xABu8; junk as usize];
    combined.extend_from_slice(&image);
    let combined_path = f._tmp.0.join("combined.bin");
    std::fs::write(&combined_path, &combined).unwrap();

    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(combined_path.to_str().unwrap()).as_ptr(),
            junk,
            image.len() as u64,
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"SquashFS"
    );
    assert_eq!(read_file(&f, "/test.txt"), "Hello from SquashFS!\n");
}

// ===================================================================
// Format detection (the BackendFactory intent, through the C ABI)
// ===================================================================

#[test]
fn format_detection_dispatches_all_backends() {
    let f = setup();

    // zip -> ZIP
    let zip_path = f._tmp.0.join("a.zip");
    tebako_contract_tests::build_fixture_zip(&zip_path);
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(zip_path.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(tfs::c_api::tebako_get_backend_name()) }.to_bytes(),
        b"ZIP"
    );
    unsafe { tfs::c_api::tebako_fs_unmount() };

    // dwarfs -> DwarFS
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(fixture("simple.dwarfs").to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(tfs::c_api::tebako_get_backend_name()) }.to_bytes(),
        b"DwarFS"
    );
    unsafe { tfs::c_api::tebako_fs_unmount() };

    // sqfs -> SquashFS
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(fixture("simple.sqfs").to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(tfs::c_api::tebako_get_backend_name()) }.to_bytes(),
        b"SquashFS"
    );
    unsafe { tfs::c_api::tebako_fs_unmount() };

    // unknown -> EINVAL
    let junk = [0xFFu8; 32];
    let junk_path = f._tmp.0.join("junk.bin");
    std::fs::write(&junk_path, junk).unwrap();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(junk_path.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}
