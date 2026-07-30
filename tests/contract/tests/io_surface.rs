//! IO-surface contract cases: directory positioning (telldir/seekdir/
//! rewinddir/dir_is_embedded), pread semantics, dlmap2file, extract_all,
//! and the ABI version export — ports of the corresponding C++ `CApiTest`
//! cases (libtfs `tests/test_c_api.cpp`) plus the ABI-version test.

use std::path::PathBuf;
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

    let tmp = TempDir::new("io");
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

impl F {
    fn init(&self) {
        let rc = unsafe {
            tfs::c_api::tebako_fs_init_from_file(
                std::ffi::CString::new(self.archive_path.to_str().unwrap())
                    .unwrap()
                    .as_ptr(),
                std::ffi::CString::new(self.mount_point.clone())
                    .unwrap()
                    .as_ptr(),
            )
        };
        assert_eq!(rc, 0);
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

fn open_hello(f: &F) -> i32 {
    unsafe { tfs::c_api::tebako_fs_open(p(f, "/content/hello.txt").as_ptr(), libc::O_RDONLY) }
}

fn readdir_names(dir: *mut std::ffi::c_void) -> Vec<String> {
    let mut out = Vec::new();
    loop {
        let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        out.push(
            unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
        );
    }
    out
}

// ===================================================================
// ABI Version
// ===================================================================

#[test]
fn abi_version_matches_header_constant() {
    let _f = setup();
    // TEBAKO_FS_ABI_VERSION == 1 in the current c_api.h; the loaded library
    // must report the same, and never less than 1.
    assert_eq!(unsafe { tfs::c_api::tebako_fs_abi_version() }, 1);
    assert!(unsafe { tfs::c_api::tebako_fs_abi_version() } >= 1);
}

// ===================================================================
// Directory handle introspection / positioning
// ===================================================================

#[test]
fn dir_is_embedded_handles() {
    let f = setup();
    f.init();

    let dir = unsafe { tfs::c_api::tebako_fs_opendir(p(&f, "/content").as_ptr()) };
    assert!(!dir.is_null());
    assert_eq!(unsafe { tfs::c_api::tebako_fs_dir_is_embedded(dir) }, 1);

    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);
    // Closed handle is no longer in the registry.
    assert_eq!(unsafe { tfs::c_api::tebako_fs_dir_is_embedded(dir) }, 0);
}

#[test]
fn dir_is_embedded_null_and_unknown() {
    let f = setup();
    f.init();

    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_dir_is_embedded(std::ptr::null_mut()) },
        0
    );
    // Wild pointer that was never registered: membership test must not crash.
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_dir_is_embedded(0xdeadbeefusize as *mut std::ffi::c_void) },
        0
    );
}

#[test]
fn telldir_tracks_read_position() {
    let f = setup();
    f.init();

    let dir = unsafe { tfs::c_api::tebako_fs_opendir(p(&f, "/content").as_ptr()) };
    assert!(!dir.is_null());

    // Fresh stream: next entry ordinal is 0.
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir) }, 0);

    let mut expected: i64 = 0;
    loop {
        let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        expected += 1;
        assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir) }, expected);
    }
    assert!(expected > 0);

    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);
}

#[test]
fn telldir_invalid_handle() {
    let f = setup();
    f.init();
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_telldir(std::ptr::null_mut()) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn rewinddir_resets_stream() {
    let f = setup();
    f.init();

    let dir = unsafe { tfs::c_api::tebako_fs_opendir(p(&f, "/content").as_ptr()) };
    assert!(!dir.is_null());

    let first_pass = readdir_names(dir);
    assert!(!first_pass.is_empty());

    unsafe { tfs::c_api::tebako_fs_rewinddir(dir) };
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir) }, 0);

    let second_pass = readdir_names(dir);
    assert_eq!(first_pass, second_pass);

    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);
}

#[test]
fn rewinddir_invalid_handle() {
    let f = setup();
    f.init();
    unsafe { tfs::c_api::tebako_fs_rewinddir(std::ptr::null_mut()) }; // no crash
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn seekdir_round_trips_cookie() {
    let f = setup();
    f.init();

    let dir = unsafe { tfs::c_api::tebako_fs_opendir(p(&f, "/content").as_ptr()) };
    assert!(!dir.is_null());

    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
    assert!(!entry.is_null());
    let name0 = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
        .to_string_lossy()
        .into_owned();

    let cookie = unsafe { tfs::c_api::tebako_fs_telldir(dir) }; // ordinal of the next entry
    assert_eq!(cookie, 1);

    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
    assert!(!entry.is_null());
    let name1 = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
        .to_string_lossy()
        .into_owned();

    // Seek back to the saved cookie: the same entry must come again.
    unsafe { tfs::c_api::tebako_fs_seekdir(dir, cookie) };
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir) }, cookie);
    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
    assert!(!entry.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_string_lossy(),
        name1
    );

    // seekdir(dir, 0) is a rewind.
    unsafe { tfs::c_api::tebako_fs_seekdir(dir, 0) };
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir) }, 0);
    let entry = unsafe { tfs::c_api::tebako_fs_readdir(dir) };
    assert!(!entry.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_string_lossy(),
        name0
    );

    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);
}

#[test]
fn seekdir_invalid_handle_and_negative_pos() {
    let f = setup();
    f.init();

    unsafe { tfs::c_api::tebako_fs_seekdir(std::ptr::null_mut(), 0) }; // no crash
    assert_eq!(unsafe { errno() }, libc::EBADF);

    let dir = unsafe { tfs::c_api::tebako_fs_opendir(p(&f, "/content").as_ptr()) };
    assert!(!dir.is_null());

    unsafe { tfs::c_api::tebako_fs_seekdir(dir, -1) };
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    // Failed seek leaves the stream where it was.
    assert_eq!(unsafe { tfs::c_api::tebako_fs_telldir(dir) }, 0);

    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(dir) }, 0);
}

// ===================================================================
// tebako_fs_dlmap2file
// ===================================================================

#[test]
fn dlmap2file_extracts_readable_host_file() {
    let f = setup();
    f.init();

    let host_path =
        unsafe { tfs::c_api::tebako_fs_dlmap2file(p(&f, "/content/hello.txt").as_ptr()) };
    assert!(!host_path.is_null());
    assert_eq!(unsafe { errno() }, 0);

    // The mapped file lives on the host and carries the memfs content.
    let path = unsafe { std::ffi::CStr::from_ptr(host_path) }
        .to_string_lossy()
        .into_owned();
    let content = std::fs::read(&path).unwrap();
    assert_eq!(content, b"Hello, World!");

    unsafe { libc::free(host_path.cast()) };
}

#[test]
fn dlmap2file_caches_extraction() {
    let f = setup();
    f.init();

    let first = unsafe { tfs::c_api::tebako_fs_dlmap2file(p(&f, "/content/data.bin").as_ptr()) };
    assert!(!first.is_null());
    let second = unsafe { tfs::c_api::tebako_fs_dlmap2file(p(&f, "/content/data.bin").as_ptr()) };
    assert!(!second.is_null());

    // Same memfs path maps to the same host file (separate string copies).
    let p1 = unsafe { std::ffi::CStr::from_ptr(first) }.to_string_lossy();
    let p2 = unsafe { std::ffi::CStr::from_ptr(second) }.to_string_lossy();
    assert_eq!(p1, p2);
    assert_ne!(first, second); // distinct allocations
    assert_eq!(std::fs::metadata(&*p1).unwrap().len(), 1024);

    unsafe {
        libc::free(first.cast());
        libc::free(second.cast());
    }
}

#[test]
fn dlmap2file_not_embedded() {
    let f = setup();
    f.init();
    let p = c("/tmp/not-embedded.txt");
    assert!(unsafe { tfs::c_api::tebako_fs_dlmap2file(p.as_ptr()) }.is_null());
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

#[test]
fn dlmap2file_null_path() {
    let f = setup();
    f.init();
    assert!(unsafe { tfs::c_api::tebako_fs_dlmap2file(std::ptr::null()) }.is_null());
}

#[test]
fn dlmap2file_missing_in_mount() {
    let f = setup();
    f.init();
    assert!(unsafe {
        tfs::c_api::tebako_fs_dlmap2file(p(&f, "/content/nonexistent.txt").as_ptr())
    }
    .is_null());
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

// ===================================================================
// dlmap-prefix redirect (the exec/dyld-closure path)
// ===================================================================

/// Materialize `/content/hello.txt` and return the per-process dlmap
/// root (the path up to and excluding the full-memfs-path tail).
fn dlmap_root(f: &F) -> String {
    let host = unsafe { tfs::c_api::tebako_fs_dlmap2file(p(f, "/content/hello.txt").as_ptr()) };
    assert!(!host.is_null());
    let path = unsafe { std::ffi::CStr::from_ptr(host) }
        .to_string_lossy()
        .into_owned();
    unsafe { libc::free(host.cast()) };
    path.trim_end_matches("/__tebako_test__/content/hello.txt")
        .to_string()
}

#[test]
fn dlmap2file_layout_preserves_full_memfs_path() {
    let f = setup();
    f.init();
    let host = unsafe { tfs::c_api::tebako_fs_dlmap2file(p(&f, "/content/hello.txt").as_ptr()) };
    assert!(!host.is_null());
    let path = unsafe { std::ffi::CStr::from_ptr(host) }
        .to_string_lossy()
        .into_owned();
    unsafe { libc::free(host.cast()) };
    assert!(path.contains("/tebako-dl-"), "dlmap marker dir: {path}");
    assert!(
        path.ends_with("/__tebako_test__/content/hello.txt"),
        "the full memfs path is the extraction tail: {path}"
    );
}

#[test]
fn open_dlmap_prefix_redirect_materializes_real_fd() {
    let f = setup();
    f.init();
    let root = dlmap_root(&f);
    // Open a NOT-yet-materialized file through its dlmap spelling: the
    // redirect materializes the memfs original and answers with a real
    // host fd (no TEBAKO_FD_FLAG) — mmap-capable, as dyld needs.
    let spelling = c(&format!("{root}/__tebako_test__/content/data.bin"));
    let fd = unsafe { tfs::c_api::tebako_fs_open(spelling.as_ptr(), libc::O_RDONLY) };
    assert!(fd >= 0, "redirect open errno: {}", unsafe { errno() });
    assert_eq!(
        unsafe { tfs::c_api::tebako_fd_is_embedded(fd) },
        0,
        "a real host fd, not a token"
    );
    let mut buf = [0u8; 1024];
    let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
    assert_eq!(n, 1024);
    assert_eq!(unsafe { libc::close(fd) }, 0);
    // …and the spelling is now a real host file as well.
    assert_eq!(
        std::fs::metadata(spelling.to_str().unwrap()).unwrap().len(),
        1024
    );
}

#[test]
fn stat_dlmap_prefix_redirect_answers_memfs_metadata() {
    let f = setup();
    f.init();
    let root = dlmap_root(&f);
    let spelling = c(&format!("{root}/__tebako_test__/content/data.bin"));
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tfs::c_api::tebako_fs_stat(spelling.as_ptr(), &mut st) };
    assert_eq!(rc, 0, "redirect stat errno: {}", unsafe { errno() });
    assert_eq!(st.st_size, 1024);
}

#[test]
fn open_dlmap_prefix_tail_not_held_passes_through() {
    let f = setup();
    f.init();
    let root = dlmap_root(&f);
    // A tail the image does not hold: the redirect declines and the
    // literal path takes the host answer — ENOENT.
    let spelling = c(&format!("{root}/__tebako_test__/content/nonexistent.txt"));
    let fd = unsafe { tfs::c_api::tebako_fs_open(spelling.as_ptr(), libc::O_RDONLY) };
    assert_eq!(fd, -1);
    assert_eq!(unsafe { errno() }, libc::ENOENT);
}

// ===================================================================
// dlmap2file dependency closure (exec/dlopen of in-image binaries)
// ===================================================================

/// A minimal 64-bit thin Mach-O fixture with the given LC_LOAD_DYLIB
/// references and LC_RPATH entries.
fn macho64_fixture(deps: &[&str], rpaths: &[&str]) -> Vec<u8> {
    const LC_LOAD_DYLIB: u32 = 0x0C;
    const LC_RPATH: u32 = 0x1C | 0x8000_0000;
    let mut cmds = Vec::new();
    for name in deps {
        let name_bytes = name.as_bytes();
        let cmdsize = (24 + name_bytes.len() + 1 + 7) & !7;
        let mut cmd = vec![0u8; cmdsize];
        cmd[0..4].copy_from_slice(&LC_LOAD_DYLIB.to_le_bytes());
        cmd[4..8].copy_from_slice(&(cmdsize as u32).to_le_bytes());
        cmd[8..12].copy_from_slice(&24_u32.to_le_bytes());
        cmd[24..24 + name_bytes.len()].copy_from_slice(name_bytes);
        cmds.extend_from_slice(&cmd);
    }
    for path in rpaths {
        let path_bytes = path.as_bytes();
        let cmdsize = (12 + path_bytes.len() + 1 + 7) & !7;
        let mut cmd = vec![0u8; cmdsize];
        cmd[0..4].copy_from_slice(&LC_RPATH.to_le_bytes());
        cmd[4..8].copy_from_slice(&(cmdsize as u32).to_le_bytes());
        cmd[8..12].copy_from_slice(&12_u32.to_le_bytes());
        cmd[12..12 + path_bytes.len()].copy_from_slice(path_bytes);
        cmds.extend_from_slice(&cmd);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&0xFEED_FACF_u32.to_le_bytes()); // MH_MAGIC_64
    out.extend_from_slice(&0x0100_000C_u32.to_le_bytes()); // CPU_TYPE_ARM64
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&2_u32.to_le_bytes()); // MH_EXECUTE
    out.extend_from_slice(&((deps.len() + rpaths.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(cmds.len() as u32).to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&cmds);
    out
}

#[test]
fn dlmap2file_materializes_dependency_closure() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { tfs::c_api::tebako_fs_unmount() };
    let tmp = TempDir::new("io-closure");
    let archive = tmp.0.join("closure.zip");
    let prog = macho64_fixture(&["@rpath/libfoo.dylib"], &["@executable_path/../lib"]);
    let libfoo = macho64_fixture(&[], &[]);
    tebako_contract_tests::build_zip(
        &archive,
        &["bin/", "lib/"],
        &[("bin/prog", &prog), ("lib/libfoo.dylib", &libfoo)],
    );
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(archive.to_str().unwrap()).as_ptr(),
            c("/__tebako_closure__").as_ptr(),
        )
    };
    assert_eq!(rc, 0);

    let host = unsafe {
        tfs::c_api::tebako_fs_dlmap2file(c("/__tebako_closure__/bin/prog").as_ptr())
    };
    assert!(!host.is_null(), "dlmap errno: {}", unsafe { errno() });
    let exe = unsafe { std::ffi::CStr::from_ptr(host) }
        .to_string_lossy()
        .into_owned();
    unsafe { libc::free(host.cast()) };

    // The closure: the rpath-resolved dylib is materialized at the very
    // spot the loader's @executable_path/../lib probe will stat.
    let root = exe.trim_end_matches("/__tebako_closure__/bin/prog");
    let dep = format!("{root}/__tebako_closure__/lib/libfoo.dylib");
    assert_eq!(
        std::fs::read(&dep).expect("the dependency closure is materialized"),
        libfoo
    );
    unsafe { tfs::c_api::tebako_fs_unmount() };
}

// ===================================================================
// pread semantics (ports of the C++ pread block)
// ===================================================================

#[test]
fn pread_reads_at_offset_leaves_position_intact() {
    let f = setup();
    f.init();
    let fd = open_hello(&f);
    assert!(fd > 0);

    // "Hello, World!" — 5 bytes at offset 7 is "World".
    let mut buffer = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd, buffer.as_mut_ptr().cast(), 5, 7) },
        5
    );
    assert_eq!(&buffer[..5], b"World");

    // The fd position must be untouched: a plain read starts at offset 0.
    let mut full = [0u8; 32];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, full.as_mut_ptr().cast(), 31) };
    assert_eq!(n, 13);
    assert_eq!(&full[..13], b"Hello, World!");

    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
}

#[test]
fn pread_after_seek_does_not_move_position() {
    let f = setup();
    f.init();
    let fd = open_hello(&f);
    assert!(fd > 0);

    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_lseek(fd, 3, libc::SEEK_SET) },
        3
    );

    let mut buffer = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd, buffer.as_mut_ptr().cast(), 5, 7) },
        5
    );
    assert_eq!(&buffer[..5], b"World");

    // Plain read must resume from the seek position (3), not from 7+5.
    let mut rest = [0u8; 32];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, rest.as_mut_ptr().cast(), 31) };
    assert_eq!(n, 10);
    assert_eq!(&rest[..10], b"lo, World!");

    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
}

#[test]
fn pread_offset_beyond_eof_returns_zero() {
    let f = setup();
    f.init();
    let fd = open_hello(&f);
    assert!(fd > 0);

    let mut buffer = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd, buffer.as_mut_ptr().cast(), 16, 13) },
        0
    );
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd, buffer.as_mut_ptr().cast(), 16, 4096) },
        0
    );

    // Position still intact.
    let mut full = [0u8; 32];
    let n = unsafe { tfs::c_api::tebako_fs_read(fd, full.as_mut_ptr().cast(), 31) };
    assert_eq!(n, 13);
    assert_eq!(&full[..13], b"Hello, World!");

    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
}

#[test]
fn pread_invalid_fd() {
    let f = setup();
    f.init();
    let mut buffer = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(999, buffer.as_mut_ptr().cast(), 16, 0) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EBADF);

    // Host fds are not libtfs fds even with a valid buffer.
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(0, buffer.as_mut_ptr().cast(), 16, 0) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EBADF);
}

#[test]
fn pread_null_buffer() {
    let f = setup();
    f.init();
    let fd = open_hello(&f);
    assert!(fd > 0);
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd, std::ptr::null_mut(), 16, 0) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
}

#[test]
fn pread_negative_offset() {
    let f = setup();
    f.init();
    let fd = open_hello(&f);
    assert!(fd > 0);
    let mut buffer = [0u8; 16];
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_pread(fd, buffer.as_mut_ptr().cast(), 16, -1) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
}

// ===================================================================
// extract_all (single mount: tree directly into dest)
// ===================================================================

#[test]
fn extract_all_single_mount_into_dest_root() {
    let f = setup();
    f.init();

    let dest = f._tmp.0.join("extracted");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_extract_all(c(dest.to_str().unwrap()).as_ptr()) },
        0
    );

    let hello = std::fs::read(dest.join("content").join("hello.txt")).unwrap();
    assert_eq!(hello, b"Hello, World!");
    let nested = std::fs::read(dest.join("content").join("subdir").join("nested.txt")).unwrap();
    assert_eq!(nested, b"Nested file content");
    let data = std::fs::read(dest.join("content").join("data.bin")).unwrap();
    assert_eq!(data, vec![b'X'; 1024]);
}

#[test]
fn extract_all_not_mounted() {
    let f = setup(); // no init
    let dest = f._tmp.0.join("extracted2");
    assert_eq!(
        unsafe { tfs::c_api::tebako_fs_extract_all(c(dest.to_str().unwrap()).as_ptr()) },
        -1
    );
    assert_eq!(unsafe { errno() }, libc::ENODEV);
}
