//! DwarFS backend contract cases: ports of the C++ `CApiOffsetTest`
//! (libtfs `tests/test_c_api.cpp`) against the same fixture image
//! (`tests/fixtures/simple.dwarfs`, borrowed from libtfs's test fixtures:
//! hello.txt + test.txt + subdir/nested.txt), plus basic mount/stat/pread/
//! readdir coverage and the "DwarFS" backend-name check.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tebako_contract_tests::TempDir;

static LOCK: Mutex<()> = Mutex::new(());

const JUNK_SIZE: u64 = 1000;

struct F {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    plain_image_path: PathBuf,
    combined_path: PathBuf,
    image: Vec<u8>,
    mount_point: String,
}

fn fixture_image() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/simple.dwarfs")
        .canonicalize()
        .expect("simple.dwarfs fixture must exist")
}

fn setup() -> F {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { tfs::c_api::tebako_fs_unmount() };

    let tmp = TempDir::new("dwarfs");
    let plain_image_path = fixture_image();
    let image = std::fs::read(&plain_image_path).expect("read fixture");

    // junk prefix (deliberately not a valid archive magic) + image
    let combined_path = tmp.0.join("combined.bin");
    let mut combined = Vec::new();
    for i in 0..JUNK_SIZE {
        combined.push(b'A' + ((i * 7) % 26) as u8);
    }
    combined.extend_from_slice(&image);
    std::fs::write(&combined_path, &combined).unwrap();

    F {
        _guard: guard,
        _tmp: tmp,
        plain_image_path,
        combined_path,
        image,
        mount_point: "/__tebako_offset_test__".to_string(),
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

unsafe fn is_initialized() -> i32 {
    unsafe { tfs::c_api::tebako_is_initialized() }
}

unsafe fn read_file_via_api(path: &str) -> String {
    unsafe {
        let p = c(path);
        let fd = tfs::c_api::tebako_fs_open(p.as_ptr(), libc::O_RDONLY);
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

fn mp(f: &F) -> std::ffi::CString {
    c(&f.mount_point)
}

// ===================================================================

#[test]
fn plain_mount_backend_name_and_reads() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(f.plain_image_path.to_str().unwrap()).as_ptr(),
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(unsafe { is_initialized() }, 1);

    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!bn.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"DwarFS"
    );

    let ap = unsafe { tfs::c_api::tebako_get_archive_path() };
    assert!(!ap.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(ap) }.to_str().unwrap(),
        f.plain_image_path.to_str().unwrap()
    );

    assert!(!unsafe { read_file_via_api(&format!("{}/hello.txt", f.mount_point)) }.is_empty());
    assert!(!unsafe { read_file_via_api(&format!("{}/test.txt", f.mount_point)) }.is_empty());
    assert!(
        !unsafe { read_file_via_api(&format!("{}/subdir/nested.txt", f.mount_point)) }.is_empty()
    );
}

#[test]
fn offset_mount_explicit_length_reads_match_plain_image() {
    let f = setup();
    // Reference content: mount the plain fixture image.
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(f.plain_image_path.to_str().unwrap()).as_ptr(),
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    let expected_hello = unsafe { read_file_via_api(&format!("{}/hello.txt", f.mount_point)) };
    let expected_test = unsafe { read_file_via_api(&format!("{}/test.txt", f.mount_point)) };
    assert!(
        !expected_hello.is_empty(),
        "fixture hello.txt unexpectedly empty"
    );
    assert!(
        !expected_test.is_empty(),
        "fixture test.txt unexpectedly empty"
    );
    let mut expected_st: libc::stat = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_stat(
                c(&format!("{}/hello.txt", f.mount_point)).as_ptr(),
                &mut expected_st,
            )
        },
        0
    );
    unsafe { tfs::c_api::tebako_fs_unmount() };

    // Mount the same image embedded at offset JUNK_SIZE with explicit length.
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.combined_path.to_str().unwrap()).as_ptr(),
            JUNK_SIZE,
            f.image.len() as u64,
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(unsafe { is_initialized() }, 1);

    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!bn.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"DwarFS"
    );

    // Reads from the offset mount must match the plain mount.
    assert_eq!(expected_hello, unsafe {
        read_file_via_api(&format!("{}/hello.txt", f.mount_point))
    });
    assert_eq!(expected_test, unsafe {
        read_file_via_api(&format!("{}/test.txt", f.mount_point))
    });

    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_stat(c(&format!("{}/hello.txt", f.mount_point)).as_ptr(), &mut st)
        },
        0
    );
    assert_eq!(expected_st.st_size, st.st_size);
}

#[test]
fn offset_mount_length_zero_means_to_end_of_file() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.combined_path.to_str().unwrap()).as_ptr(),
            JUNK_SIZE,
            0,
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(unsafe { is_initialized() }, 1);
    assert!(!unsafe { read_file_via_api(&format!("{}/hello.txt", f.mount_point)) }.is_empty());
}

#[test]
fn offset_mount_zero_offset_explicit_length() {
    let f = setup();
    // Region path with offset == 0 but explicit length (no trailing data).
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.plain_image_path.to_str().unwrap()).as_ptr(),
            0,
            f.image.len() as u64,
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(unsafe { is_initialized() }, 1);
    assert!(!unsafe { read_file_via_api(&format!("{}/hello.txt", f.mount_point)) }.is_empty());
}

#[test]
fn offset_past_end_fails() {
    let f = setup();
    let file_size = JUNK_SIZE + f.image.len() as u64;
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.combined_path.to_str().unwrap()).as_ptr(),
            file_size + 1,
            0,
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn offset_at_end_empty_region_fails() {
    let f = setup();
    let file_size = JUNK_SIZE + f.image.len() as u64;
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.combined_path.to_str().unwrap()).as_ptr(),
            file_size,
            0,
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn length_overflow_fails() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.combined_path.to_str().unwrap()).as_ptr(),
            JUNK_SIZE,
            f.image.len() as u64 + 1,
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn init_from_file_at_null_path_fails() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(std::ptr::null(), JUNK_SIZE, 0, mp(&f).as_ptr())
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn init_from_file_at_null_mount_point_fails() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.combined_path.to_str().unwrap()).as_ptr(),
            JUNK_SIZE,
            0,
            std::ptr::null(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
}

#[test]
fn init_from_file_at_nonexistent_file_fails() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c("/nonexistent/combined.bin").as_ptr(),
            JUNK_SIZE,
            0,
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_ne!(unsafe { errno() }, 0);
}

#[test]
fn memory_mount_reads() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(f.image.as_ptr().cast(), f.image.len(), mp(&f).as_ptr())
    };
    assert_eq!(rc, 0);
    assert_eq!(unsafe { is_initialized() }, 1);

    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!bn.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"DwarFS"
    );

    // Memory mounts report no archive path.
    assert!(unsafe { tfs::c_api::tebako_get_archive_path() }.is_null());

    assert!(!unsafe { read_file_via_api(&format!("{}/hello.txt", f.mount_point)) }.is_empty());
}

#[test]
fn dwarfs_stat_pread_readdir() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(f.plain_image_path.to_str().unwrap()).as_ptr(),
            mp(&f).as_ptr(),
        )
    };
    assert_eq!(rc, 0);

    // stat: regular file with sane mode/size; directory on root.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_stat(c(&format!("{}/hello.txt", f.mount_point)).as_ptr(), &mut st)
        },
        0
    );
    assert_eq!(st.st_mode & libc::S_IFMT, libc::S_IFREG as _);
    assert!(st.st_size > 0);
    let mut st_dir: libc::stat = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_stat(
                c(&format!("{}/subdir", f.mount_point)).as_ptr(),
                &mut st_dir,
            )
        },
        0
    );
    assert_eq!(st_dir.st_mode & libc::S_IFMT, libc::S_IFDIR as _);

    // pread matches a sequential read of the same file.
    let fd = unsafe {
        tfs::c_api::tebako_fs_open(
            c(&format!("{}/hello.txt", f.mount_point)).as_ptr(),
            libc::O_RDONLY,
        )
    };
    assert!(fd > 0);
    let mut whole = vec![0u8; st.st_size as usize];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, whole.as_mut_ptr().cast(), whole.len()) };
    assert_eq!(n as i64, st.st_size);
    let mut tail = [0u8; 4];
    let n = unsafe { tfs::c_api::tebako_fs_pread(fd, tail.as_mut_ptr().cast(), 4, st.st_size - 4) };
    assert_eq!(n, 4);
    assert_eq!(&tail, &whole[whole.len() - 4..]);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);

    // readdir on the root lists the known entries with types.
    let dir = unsafe { tfs::c_api::tebako_fs_opendir(c(&f.mount_point).as_ptr()) };
    assert!(!dir.is_null());
    let mut names = Vec::new();
    loop {
        let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let ty = unsafe { (*entry).d_type };
        names.push((name, ty));
    }
    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);

    let has = |n: &str, t: u8| names.iter().any(|(name, ty)| name == n && *ty == t);
    assert!(has("hello.txt", tfs::DT_REG), "{names:?}");
    assert!(has("test.txt", tfs::DT_REG), "{names:?}");
    assert!(has("subdir", tfs::DT_DIR), "{names:?}");
}
