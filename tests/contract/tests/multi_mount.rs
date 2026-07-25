//! Multi-mount contract cases: ports of the C++ `CApiMultiMountTest`
//! suite (libtfs `tests/test_c_api.cpp`), exercising the same fixture
//! trees through the Rust `tebako_fs_mount_*` API.
//!
//! Fixture trees (identical to the C++ fixture):
//! - archive A: `content/alpha.txt` = "alpha-content",
//!   `nested/beta.txt` = "from-A"
//! - archive B: `beta.txt` = "beta-content"

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use tebako_contract_tests::{build_zip, TempDir};

static LOCK: Mutex<()> = Mutex::new(());

struct Mm {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    archive_a_path: PathBuf,
    archive_b_path: PathBuf,
    archive_a: Vec<u8>,
    archive_b: Vec<u8>,
}

fn setup() -> Mm {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { tfs::c_api::tebako_fs_unmount() };

    let tmp = TempDir::new("mm");
    let archive_a_path = tmp.0.join("a.zip");
    let archive_b_path = tmp.0.join("b.zip");
    build_zip(
        &archive_a_path,
        &["content/", "nested/"],
        &[
            ("content/alpha.txt", b"alpha-content".as_slice()),
            ("nested/beta.txt", b"from-A".as_slice()),
        ],
    );
    build_zip(
        &archive_b_path,
        &[],
        &[("beta.txt", b"beta-content".as_slice())],
    );

    Mm {
        _guard: guard,
        _tmp: tmp,
        archive_a: std::fs::read(&archive_a_path).unwrap(),
        archive_b: std::fs::read(&archive_b_path).unwrap(),
        archive_a_path,
        archive_b_path,
    }
}

impl Drop for Mm {
    fn drop(&mut self) {
        unsafe { tfs::c_api::tebako_fs_unmount() };
    }
}

impl Mm {
    fn mount_a_mem(&self, mp: &str) -> i32 {
        let mp = std::ffi::CString::new(mp).unwrap();
        let mut h: i32 = -1;
        let rc = unsafe {
            tfs::c_api::tebako_fs_mount_from_memory(
                self.archive_a.as_ptr().cast(),
                self.archive_a.len(),
                mp.as_ptr(),
                &mut h,
            )
        };
        assert_eq!(rc, 0, "mount A from memory");
        h
    }

    fn mount_b_mem(&self, mp: &str) -> i32 {
        let mp = std::ffi::CString::new(mp).unwrap();
        let mut h: i32 = -1;
        let rc = unsafe {
            tfs::c_api::tebako_fs_mount_from_memory(
                self.archive_b.as_ptr().cast(),
                self.archive_b.len(),
                mp.as_ptr(),
                &mut h,
            )
        };
        assert_eq!(rc, 0, "mount B from memory");
        h
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

// ===================================================================

#[test]
fn mount_two_from_memory_read_from_both() {
    let f = setup();
    let ha = f.mount_a_mem("/__mm_a__");
    let hb = f.mount_b_mem("/__mm_b__");
    assert!(ha >= 0 && hb >= 0);
    assert_ne!(ha, hb);
    assert_eq!(unsafe { is_initialized() }, 1);

    assert_eq!(
        unsafe { read_file_via_api("/__mm_a__/content/alpha.txt") },
        "alpha-content"
    );
    assert_eq!(
        unsafe { read_file_via_api("/__mm_b__/beta.txt") },
        "beta-content"
    );

    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(ha) }, 0);
    assert_eq!(unsafe { is_initialized() }, 1); // B still mounted
    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(hb) }, 0);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn mount_from_file_read_from_both() {
    let f = setup();
    let (mut ha, mut hb) = (-1, -1);
    let rc = unsafe {
        tfs::c_api::tebako_fs_mount_from_file(
            c(f.archive_a_path.to_str().unwrap()).as_ptr(),
            c("/__mm_fa__").as_ptr(),
            &mut ha,
        )
    };
    assert_eq!(rc, 0);
    let rc = unsafe {
        tfs::c_api::tebako_fs_mount_from_file(
            c(f.archive_b_path.to_str().unwrap()).as_ptr(),
            c("/__mm_fb__").as_ptr(),
            &mut hb,
        )
    };
    assert_eq!(rc, 0);
    assert!(ha >= 0 && hb >= 0);

    assert_eq!(
        unsafe { read_file_via_api("/__mm_fa__/content/alpha.txt") },
        "alpha-content"
    );
    assert_eq!(
        unsafe { read_file_via_api("/__mm_fb__/beta.txt") },
        "beta-content"
    );
}

#[test]
fn mount_from_file_at_mixed_with_memory_mount() {
    let f = setup();
    // Combined file: junk prefix + archive A bytes.
    let junk_size: u64 = 1000;
    let combined_path = f._tmp.0.join("combined.bin");
    let mut combined = Vec::new();
    for i in 0..junk_size {
        combined.push(b'A' + ((i * 7) % 26) as u8);
    }
    combined.extend_from_slice(&f.archive_a);
    std::fs::write(&combined_path, &combined).unwrap();

    let mut ha: i32 = -1;
    let rc = unsafe {
        tfs::c_api::tebako_fs_mount_from_file_at(
            c(combined_path.to_str().unwrap()).as_ptr(),
            junk_size,
            f.archive_a.len() as u64,
            c("/__mm_at__").as_ptr(),
            &mut ha,
        )
    };
    assert_eq!(rc, 0);
    let hb = f.mount_b_mem("/__mm_mem__");
    assert!(hb >= 0);

    assert_eq!(
        unsafe { read_file_via_api("/__mm_at__/content/alpha.txt") },
        "alpha-content"
    );
    assert_eq!(
        unsafe { read_file_via_api("/__mm_mem__/beta.txt") },
        "beta-content"
    );
}

#[test]
fn longest_prefix_nested_mounts_dispatch() {
    let f = setup();
    let mp_outer = "/__mm__";
    let mp_nested = "/__mm__/nested";
    f.mount_a_mem(mp_outer);
    f.mount_b_mem(mp_nested);

    // Outer mount still serves its own subtree.
    assert_eq!(
        unsafe { read_file_via_api("/__mm__/content/alpha.txt") },
        "alpha-content"
    );

    // The nested mount owns the /__mm__/nested prefix: B's "beta.txt" must
    // win over A's shadowing "nested/beta.txt" ("from-A").
    assert_eq!(
        unsafe { read_file_via_api("/__mm__/nested/beta.txt") },
        "beta-content"
    );

    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tfs::c_api::tebako_fs_stat(c("/__mm__/nested/beta.txt").as_ptr(), &mut st) };
    assert_eq!(rc, 0);
    assert_eq!(st.st_size, "beta-content".len() as i64);
}

#[test]
fn duplicate_mount_point_fails_with_eexist() {
    let f = setup();
    f.mount_a_mem("/__mm_dup__");
    let mut h: i32 = -1;

    let rc = unsafe {
        tfs::c_api::tebako_fs_mount_from_memory(
            f.archive_b.as_ptr().cast(),
            f.archive_b.len(),
            c("/__mm_dup__").as_ptr(),
            &mut h,
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EEXIST);

    let rc = unsafe {
        tfs::c_api::tebako_fs_mount_from_file(
            c(f.archive_b_path.to_str().unwrap()).as_ptr(),
            c("/__mm_dup__").as_ptr(),
            &mut h,
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EEXIST);

    // Original mount is still fully usable.
    assert_eq!(
        unsafe { read_file_via_api("/__mm_dup__/content/alpha.txt") },
        "alpha-content"
    );
}

#[test]
fn mount_bad_arguments() {
    let f = setup();
    let mut h: i32 = -1;

    // NULL out_handle.
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_memory(
                f.archive_a.as_ptr().cast(),
                f.archive_a.len(),
                c("/__mm_x__").as_ptr(),
                std::ptr::null_mut(),
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_file(
                c(f.archive_a_path.to_str().unwrap()).as_ptr(),
                c("/__mm_x__").as_ptr(),
                std::ptr::null_mut(),
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_file_at(
                c(f.archive_a_path.to_str().unwrap()).as_ptr(),
                0,
                1,
                c("/__mm_x__").as_ptr(),
                std::ptr::null_mut(),
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);

    // NULL / empty mount point.
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_memory(
                f.archive_a.as_ptr().cast(),
                f.archive_a.len(),
                std::ptr::null(),
                &mut h,
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_memory(
                f.archive_a.as_ptr().cast(),
                f.archive_a.len(),
                c("").as_ptr(),
                &mut h,
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_file(
                c(f.archive_a_path.to_str().unwrap()).as_ptr(),
                c("").as_ptr(),
                &mut h,
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);

    // NULL data / zero size.
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_memory(
                std::ptr::null(),
                100,
                c("/__mm_x__").as_ptr(),
                &mut h,
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_memory(
                f.archive_a.as_ptr().cast(),
                0,
                c("/__mm_x__").as_ptr(),
                &mut h,
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);

    // NULL archive path.
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_file(std::ptr::null(), c("/__mm_x__").as_ptr(), &mut h)
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(
        unsafe {
            tfs::c_api::tebako_fs_mount_from_file_at(
                std::ptr::null(),
                0,
                0,
                c("/__mm_x__").as_ptr(),
                &mut h,
            )
        },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);

    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn unmount_handle_unknown_handle_enodev() {
    let f = setup();
    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(999) }, -1);
    assert_eq!(unsafe { errno() }, libc::ENODEV);

    f.mount_a_mem("/__mm_u__");
    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(999) }, -1);
    assert_eq!(unsafe { errno() }, libc::ENODEV);
}

#[test]
fn unmount_handle_force_closes_only_own_fds_and_dirs() {
    let f = setup();
    let ha = f.mount_a_mem("/__mm_iso_a__");
    let hb = f.mount_b_mem("/__mm_iso_b__");

    let fd_a = unsafe {
        tfs::c_api::tebako_fs_open(
            c("/__mm_iso_a__/content/alpha.txt").as_ptr(),
            libc::O_RDONLY,
        )
    };
    let fd_b =
        unsafe { tfs::c_api::tebako_fs_open(c("/__mm_iso_b__/beta.txt").as_ptr(), libc::O_RDONLY) };
    assert!(fd_a > 0 && fd_b > 0);

    let dir_a = unsafe { tfs::c_api::tebako_fs_opendir(c("/__mm_iso_a__/content").as_ptr()) };
    let dir_b = unsafe { tfs::c_api::tebako_fs_opendir(c("/__mm_iso_b__").as_ptr()) };
    assert!(!dir_a.is_null() && !dir_b.is_null());

    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(ha) }, 0);

    // A's handles are force-closed.
    let mut buf = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_read(fd_a, buf.as_mut_ptr().cast(), 16) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EBADF);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd_a) }, -1);
    assert_eq!(unsafe { errno() }, libc::EBADF);
    assert!(unsafe { tfs::c_api::tebako_fs_readdir(dir_a) }.is_null());
    assert_eq!(unsafe { errno() }, libc::EBADF);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir_a) }, -1);
    assert_eq!(unsafe { errno() }, libc::EBADF);

    // B is unaffected.
    let mut rbuf = [0u8; 64];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd_b, rbuf.as_mut_ptr().cast(), 64) };
    assert!(n > 0);
    assert_eq!(&rbuf[..n as usize], b"beta-content");
    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir_b) };
    assert!(!entry.is_null());
    let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
    assert_eq!(name.to_bytes(), b"beta.txt");

    // New operations on B still work.
    assert_eq!(
        unsafe { read_file_via_api("/__mm_iso_b__/beta.txt") },
        "beta-content"
    );
    assert_eq!(unsafe { is_initialized() }, 1);

    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd_b) }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir_b) }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(hb) }, 0);
    assert_eq!(unsafe { is_initialized() }, 0);
}

#[test]
fn unmount_handle_allows_remount_handles_not_reused() {
    let f = setup();
    let h1 = f.mount_a_mem("/__mm_re__");
    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(h1) }, 0);

    // The mount point is free again; the handle is not reused.
    let h2 = f.mount_b_mem("/__mm_re__");
    assert!(h2 > h1);
    assert_eq!(
        unsafe { read_file_via_api("/__mm_re__/beta.txt") },
        "beta-content"
    );
}

#[test]
fn compat_init_fails_when_mounts_exist() {
    let f = setup();
    f.mount_a_mem("/__mm_c__");

    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(f.archive_b_path.to_str().unwrap()).as_ptr(),
            c("/__mm_other__").as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EEXIST);

    let rc = unsafe {
        tfs::c_api::tebako_fs_init(
            f.archive_b.as_ptr().cast(),
            f.archive_b.len(),
            c("/__mm_other__").as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EEXIST);

    assert_eq!(
        unsafe { read_file_via_api("/__mm_c__/content/alpha.txt") },
        "alpha-content"
    );
}

#[test]
fn compat_mount_after_init_getters_unaffected() {
    let f = setup();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(f.archive_a_path.to_str().unwrap()).as_ptr(),
            c("/__mm_compat__").as_ptr(),
        )
    };
    assert_eq!(rc, 0);

    let hb = f.mount_b_mem("/__mm_extra__");
    assert!(hb >= 0);
    assert_eq!(unsafe { is_initialized() }, 1);

    // Compat getters still report the init* mount.
    let mp = unsafe { tfs::c_api::tebako_get_mount_point() };
    assert!(!mp.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(mp) }.to_bytes(),
        b"/__mm_compat__"
    );
    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!bn.is_null());
    assert_eq!(unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(), b"ZIP");
    let ap = unsafe { tfs::c_api::tebako_get_archive_path() };
    assert!(!ap.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(ap) }.to_str().unwrap(),
        f.archive_a_path.to_str().unwrap()
    );

    // Both mounts readable.
    assert_eq!(
        unsafe { read_file_via_api("/__mm_compat__/content/alpha.txt") },
        "alpha-content"
    );
    assert_eq!(
        unsafe { read_file_via_api("/__mm_extra__/beta.txt") },
        "beta-content"
    );

    // tebako_fs_unmount() tears down everything.
    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(unsafe { is_initialized() }, 0);
    assert!(unsafe { tfs::c_api::tebako_get_mount_point() }.is_null());
}

#[test]
fn compat_unmount_all_force_closes_all_mounts() {
    let f = setup();
    f.mount_a_mem("/__mm_ua__");
    f.mount_b_mem("/__mm_ub__");

    let fd_a = unsafe {
        tfs::c_api::tebako_fs_open(c("/__mm_ua__/content/alpha.txt").as_ptr(), libc::O_RDONLY)
    };
    let fd_b =
        unsafe { tfs::c_api::tebako_fs_open(c("/__mm_ub__/beta.txt").as_ptr(), libc::O_RDONLY) };
    assert!(fd_a > 0 && fd_b > 0);

    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(unsafe { is_initialized() }, 0);

    let mut buf = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_read(fd_a, buf.as_mut_ptr().cast(), 16) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EBADF);
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_read(fd_b, buf.as_mut_ptr().cast(), 16) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn path_is_embedded_multi_mounts() {
    let f = setup();
    f.mount_a_mem("/__mm_pa__");
    f.mount_b_mem("/__mm_pb__");

    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(c("/__mm_pa__/content/alpha.txt").as_ptr()) },
        1
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(c("/__mm_pb__/beta.txt").as_ptr()) },
        1
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(c("/__mm_pb__").as_ptr()) },
        1
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(c("/__mm_other__/x").as_ptr()) },
        0
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_path_is_embedded(c("/tmp/file.txt").as_ptr()) },
        0
    );
}

#[test]
fn extract_all_multi_mount_subtrees() {
    let f = setup();
    f.mount_a_mem("/__mm_xa__");
    f.mount_b_mem("/__mm_xb__");

    let dest = f._tmp.0.join("extracted");
    std::fs::create_dir_all(&dest).unwrap();
    let rc = unsafe { tfs::c_api::tebako_fs_extract_all(c(dest.to_str().unwrap()).as_ptr()) };
    assert_eq!(rc, 0);

    // Each mount is extracted under its own mount-point-basename subtree.
    let a = std::fs::read(dest.join("__mm_xa__").join("content").join("alpha.txt")).unwrap();
    assert_eq!(a, b"alpha-content");
    let b = std::fs::read(dest.join("__mm_xb__").join("beta.txt")).unwrap();
    assert_eq!(b, b"beta-content");
}

// ===================================================================
// Multi-mount interplay: operations dispatch to the owning mount
// ===================================================================

#[test]
fn pread_dispatches_to_owning_mount() {
    let f = setup();
    f.mount_a_mem("/__mm_pa__");
    f.mount_b_mem("/__mm_pb__");

    let fd_a = unsafe {
        tfs::c_api::tebako_fs_open(c("/__mm_pa__/content/alpha.txt").as_ptr(), libc::O_RDONLY)
    };
    let fd_b =
        unsafe { tfs::c_api::tebako_fs_open(c("/__mm_pb__/beta.txt").as_ptr(), libc::O_RDONLY) };
    assert!(fd_a > 0 && fd_b > 0);

    // "alpha-content": 7 bytes at offset 6 is "content".
    let mut buf_a = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd_a, buf_a.as_mut_ptr().cast(), 7, 6) },
        7
    );
    assert_eq!(&buf_a[..7], b"content");

    // "beta-content": 4 bytes at offset 0 is "beta".
    let mut buf_b = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd_b, buf_b.as_mut_ptr().cast(), 4, 0) },
        4
    );
    assert_eq!(&buf_b[..4], b"beta");

    // Both fd positions are untouched.
    let mut rest_a = [0u8; 32];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd_a, rest_a.as_mut_ptr().cast(), 31) };
    assert_eq!(n, 13);
    assert_eq!(&rest_a[..13], b"alpha-content");
    let mut rest_b = [0u8; 32];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd_b, rest_b.as_mut_ptr().cast(), 31) };
    assert_eq!(n, 12);
    assert_eq!(&rest_b[..12], b"beta-content");

    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd_a) }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd_b) }, 0);
}

#[test]
fn dir_positioning_independent_per_mount() {
    let f = setup();
    f.mount_a_mem("/__mm_dpa__");
    f.mount_b_mem("/__mm_dpb__");

    let dir_a = unsafe { tfs::c_api::tebako_fs_opendir(c("/__mm_dpa__/content").as_ptr()) };
    let dir_b = unsafe { tfs::c_api::tebako_fs_opendir(c("/__mm_dpb__").as_ptr()) };
    assert!(!dir_a.is_null() && !dir_b.is_null());

    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir_a) };
    assert!(!entry.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes(),
        b"alpha.txt"
    );
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir_a) }, 1);
    // Positioning on A does not disturb B's stream.
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir_b) }, 0);

    unsafe { tfs::c_api::tebako_fs_seekdir(dir_a, 0) };
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir_a) }, 0);
    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir_a) };
    assert!(!entry.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes(),
        b"alpha.txt"
    );

    // B still serves its own first entry.
    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir_b) };
    assert!(!entry.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes(),
        b"beta.txt"
    );

    unsafe { tfs::c_api::tebako_fs_rewinddir(dir_b) };
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir_b) }, 0);
    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir_b) };
    assert!(!entry.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes(),
        b"beta.txt"
    );

    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir_a) }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir_b) }, 0);
}

#[test]
fn dir_is_embedded_per_mount_registry() {
    let f = setup();
    let ha = f.mount_a_mem("/__mm_dia__");
    let hb = f.mount_b_mem("/__mm_dib__");

    let dir_a = unsafe { tfs::c_api::tebako_fs_opendir(c("/__mm_dia__/content").as_ptr()) };
    let dir_b = unsafe { tfs::c_api::tebako_fs_opendir(c("/__mm_dib__").as_ptr()) };
    assert!(!dir_a.is_null() && !dir_b.is_null());
    assert_eq!(unsafe { tfs::c_api::tebako_fs_dir_is_embedded(dir_a) }, 1);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_dir_is_embedded(dir_b) }, 1);

    // Unmounting A force-closes only A's dir handles.
    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(ha) }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_dir_is_embedded(dir_a) }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_dir_is_embedded(dir_b) }, 1);

    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir_b) }, 0);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_unmount_handle(hb) }, 0);
}

#[test]
fn dlmap2file_multi_mount_distinct_extractions() {
    let f = setup();
    f.mount_a_mem("/__mm_dla__");
    f.mount_b_mem("/__mm_dlb__");

    // Same basename "beta.txt" in both mounts, different content.
    let host_a =
        unsafe { tfs::c_api::tebako_fs_dlmap2file(c("/__mm_dla__/nested/beta.txt").as_ptr()) };
    let host_b = unsafe { tfs::c_api::tebako_fs_dlmap2file(c("/__mm_dlb__/beta.txt").as_ptr()) };
    assert!(!host_a.is_null() && !host_b.is_null());
    let path_a = unsafe { std::ffi::CStr::from_ptr(host_a) }
        .to_string_lossy()
        .into_owned();
    let path_b = unsafe { std::ffi::CStr::from_ptr(host_b) }
        .to_string_lossy()
        .into_owned();
    assert_ne!(path_a, path_b);

    let content_a = std::fs::read(&path_a).unwrap();
    assert_eq!(content_a, b"from-A");
    let content_b = std::fs::read(&path_b).unwrap();
    assert_eq!(content_b, b"beta-content");

    unsafe {
        libc::free(host_a.cast());
        libc::free(host_b.cast());
    }
}
