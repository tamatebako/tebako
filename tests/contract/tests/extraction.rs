//! Extraction contract cases: ports of the C++ `ExtractionTest` cases
//! (libtfs `tests/test_extraction.cpp`) that map to the C ABI
//! (`tebako_fs_extract_all`), against the same fixture tree plus the
//! squashfs fixtures for the metadata-preservation cases.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tebako_contract_tests::{build_fixture_zip, TempDir};

static LOCK: Mutex<()> = Mutex::new(());

struct F {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    archive_path: PathBuf,
    mount_point: String,
}

fn setup() -> F {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { tfs::c_api::tebako_fs_unmount() };

    let tmp = TempDir::new("extract");
    let archive_path = tmp.0.join("test.zip");
    build_fixture_zip(&archive_path);
    F {
        _guard: guard,
        _tmp: tmp,
        archive_path,
        mount_point: "/__tebako_test__".to_string(),
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

fn init_zip(f: &F) {
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(f.archive_path.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
}

fn extract(dest: &Path) -> i32 {
    unsafe { tfs::c_api::tebako_fs_extract_all(c(dest.to_str().unwrap()).as_ptr()) }
}

// ===================================================================

#[test]
fn extract_all_success_and_content() {
    let f = setup();
    init_zip(&f);
    let dest = f._tmp.0.join("out");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(extract(&dest), 0);
    assert_eq!(unsafe { errno() }, 0);

    // ExtractedFiles_ContentCorrect / BinaryDataPreserved / FileSizesCorrect
    let hello = std::fs::read(dest.join("content/hello.txt")).unwrap();
    assert_eq!(hello, b"Hello, World!");
    let data = std::fs::read(dest.join("content/data.bin")).unwrap();
    assert_eq!(data, vec![b'X'; 1024]);
    let nested = std::fs::read(dest.join("content/subdir/nested.txt")).unwrap();
    assert_eq!(nested, b"Nested file content");
}

#[test]
fn extract_all_null_destination() {
    let f = setup();
    init_zip(&f);
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_extract_all(std::ptr::null()) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn extract_all_creates_nonexistent_directory() {
    let f = setup();
    init_zip(&f);
    // C++ creates the destination if missing (mirrors
    // ExtractAll_CreatesNonexistentDirectory).
    let dest = f._tmp.0.join("deep/nonexistent/out");
    assert!(!dest.exists());
    assert_eq!(extract(&dest), 0);
    assert!(dest.join("content/hello.txt").exists());
}

#[test]
fn extracted_empty_file_and_dirs_structure() {
    let f = setup();
    init_zip(&f);
    let dest = f._tmp.0.join("out2");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(extract(&dest), 0);

    // ExtractedFiles_EmptyFileCorrect
    let empty = dest.join("content/empty.txt");
    assert!(empty.exists());
    assert_eq!(std::fs::metadata(&empty).unwrap().len(), 0);

    // ExtractedDirs_StructurePreserved / NestedPathsCorrect
    assert!(dest.join("content/subdir").is_dir());
    assert!(dest.join("content/subdir/nested.txt").is_file());
}

#[test]
fn extract_twice_overwrites() {
    let f = setup();
    init_zip(&f);
    let dest = f._tmp.0.join("out3");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(extract(&dest), 0);
    // Second extraction into the same dir succeeds and content stays right.
    assert_eq!(extract(&dest), 0);
    let hello = std::fs::read(dest.join("content/hello.txt")).unwrap();
    assert_eq!(hello, b"Hello, World!");
}

#[test]
fn memory_mounted_archive_extracts() {
    let f = setup();
    let data = std::fs::read(&f.archive_path).unwrap();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(data.as_ptr().cast(), data.len(), c(&f.mount_point).as_ptr())
    };
    assert_eq!(rc, 0);
    let dest = f._tmp.0.join("out_mem");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(extract(&dest), 0);
    assert_eq!(
        std::fs::read(dest.join("content/hello.txt")).unwrap(),
        b"Hello, World!"
    );
}

#[test]
fn sqfs_extraction_preserves_permissions_and_empty_dir() {
    let f = setup();
    let image = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/permissions.sqfs")
        .canonicalize()
        .unwrap();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(image.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);

    let dest = f._tmp.0.join("out_sqfs");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(extract(&dest), 0);

    // SquashFS preserves POSIX permissions through extraction
    // (Metadata_PermissionsPreserved).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(dest.join("readonly.txt"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o444);
        let mode = std::fs::metadata(dest.join("script.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        let mode = std::fs::metadata(dest.join("private.txt"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    // ExtractedDirs_EmptyDirectoryCreated (via empty.sqfs).
    let image = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/empty.sqfs")
        .canonicalize()
        .unwrap();
    unsafe { tfs::c_api::tebako_fs_unmount() };
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(image.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    let dest2 = f._tmp.0.join("out_sqfs_empty");
    std::fs::create_dir_all(&dest2).unwrap();
    assert_eq!(extract(&dest2), 0);
    assert!(dest2.join("empty_dir").is_dir());
    assert!(dest2.join("empty_file.txt").is_file());
}

#[test]
fn dwarfs_extraction_multiple_backends_supported() {
    let f = setup();
    let image = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/simple.dwarfs")
        .canonicalize()
        .unwrap();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(image.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    let dest = f._tmp.0.join("out_dwarfs");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(extract(&dest), 0);

    // The fixture tree is extracted with content intact.
    let hello = dest.join("hello.txt");
    assert!(hello.is_file());
    assert!(std::fs::metadata(&hello).unwrap().len() > 0);
    assert!(dest.join("subdir/nested.txt").is_file());
}

#[test]
fn extracted_files_paths_with_spaces() {
    let f = setup();
    // A zip with a spaced filename (mirrors EdgeCase_PathWithSpaces).
    let archive = f._tmp.0.join("spaced.zip");
    tebako_contract_tests::build_zip(
        &archive,
        &[],
        &[("dir with space/file name.txt", b"spaced".as_slice())],
    );
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(archive.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    let dest = f._tmp.0.join("out_spaced");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(extract(&dest), 0);
    assert_eq!(
        std::fs::read(dest.join("dir with space/file name.txt")).unwrap(),
        b"spaced"
    );
}
