//! The recursive mount algebra (spec 11 §2's table, spec 23's composition
//! proven at DEPTH): a five-level nesting mounts once, in order, and
//! dispatches by longest prefix exactly as the flat case does.
//!
//! The properties this file pins, because each of them is something a
//! nested mount table can silently get wrong:
//!
//! - **longest prefix wins, not first match.** Every level carries a file
//!   of the SAME name with distinct content, so a table that resolved by
//!   mount ORDER instead of prefix length would answer with the wrong
//!   level's bytes.
//! - **a mount point is namespace, not filesystem.** Level 3 mounts at a
//!   path that only exists inside level 2's image; nothing has to be
//!   created for it to serve.
//! - **duplicate mount points are EEXIST**, as in the flat case.
//! - **unmounting is per-handle, never a cascade.** Dropping a middle
//!   mount leaves its descendants serving their own prefixes.
//!
//! All tests serialize on LOCK: the mount table is process-global.

use std::ffi::{c_char, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use tfs::c_api;

static LOCK: Mutex<()> = Mutex::new(());

const LEVELS: [&str; 5] = ["/a", "/a/b", "/a/b/c", "/a/b/c/d", "/a/b/c/d/e"];

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tfs-nested-test-{tag}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(std::fs::canonicalize(&dir).unwrap())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn c(path: &Path) -> CString {
    CString::new(path.to_str().unwrap()).unwrap()
}

fn c_str(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// A one-file image: `same.txt` (the dispatch witness) and `level.txt`
/// (the level's own name, so a mixed-up mount is obvious).
fn build_level_zip(path: &Path, level: usize) {
    use std::io::Write as _;
    let file = std::fs::File::create(path).unwrap();
    let mut w = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    w.add_directory("content/", opts).unwrap();
    w.start_file("content/same.txt", opts).unwrap();
    w.write_all(format!("level-{level}").as_bytes()).unwrap();
    w.start_file("content/level.txt", opts).unwrap();
    w.write_all(format!("{level}").as_bytes()).unwrap();
    w.finish().unwrap();
}

struct Fixture {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    images: Vec<PathBuf>,
    handles: Vec<i32>,
}

impl Fixture {
    /// Mount all five levels, in order, and return the fixture.
    fn mount_five(tag: &str) -> Fixture {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { c_api::tebako_fs_unmount() };

        let tmp = TempDir::new(tag);
        let mut images = Vec::new();
        for level in 1..=LEVELS.len() {
            let path = tmp.0.join(format!("level{level}.zip"));
            build_level_zip(&path, level);
            images.push(path);
        }

        // Level 1 initializes the engine (the flat case: one mount).
        let rc = unsafe {
            c_api::tebako_fs_init_from_file(c(&images[0]).as_ptr(), c_str(LEVELS[0]).as_ptr())
        };
        assert_eq!(rc, 0, "level 1 must initialize the table");

        let mut handles = Vec::new();
        for (idx, image) in images.iter().enumerate().skip(1) {
            let mut handle: i32 = -1;
            let rc = unsafe {
                c_api::tebako_fs_mount_from_file(
                    c(image).as_ptr(),
                    c_str(LEVELS[idx]).as_ptr(),
                    &mut handle,
                )
            };
            assert_eq!(rc, 0, "level {} must mount at {}", idx + 1, LEVELS[idx]);
            handles.push(handle);
        }

        Fixture {
            _guard: guard,
            _tmp: tmp,
            images,
            handles,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe { c_api::tebako_fs_unmount() };
    }
}

fn read_file(path: &str) -> Option<String> {
    let fd = unsafe { c_api::tebako_fs_open(c_str(path).as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return None;
    }
    let mut buf = vec![0u8; 256];
    let n = unsafe { c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast::<c_void>(), buf.len()) };
    unsafe { c_api::tebako_fs_close(fd) };
    if n < 0 {
        return None;
    }
    buf.truncate(n as usize);
    Some(String::from_utf8(buf).unwrap())
}

#[test]
fn five_nested_mounts_dispatch_by_longest_prefix() {
    let _f = Fixture::mount_five("depth");

    // Every level's own file answers at its own prefix.
    for (idx, mount) in LEVELS.iter().enumerate() {
        let level = idx + 1;
        assert_eq!(
            read_file(&format!("{mount}/content/level.txt")).as_deref(),
            Some(level.to_string().as_str()),
            "level {level}'s own file must answer at {mount}"
        );
    }

    // The SAME name under every prefix resolves to that prefix's mount,
    // never to the first mount that happens to carry the name.
    for (idx, mount) in LEVELS.iter().enumerate() {
        let level = idx + 1;
        assert_eq!(
            read_file(&format!("{mount}/content/same.txt")).as_deref(),
            Some(format!("level-{level}").as_str()),
            "{mount}/content/same.txt must serve level {level}, not a shallower mount"
        );
    }
}

#[test]
fn a_mount_point_is_namespace_not_a_path_in_the_parent_image() {
    let f = Fixture::mount_five("ns");

    // Level 2's image holds only content/; level 3 is mounted at
    // /a/b/c, a path that exists in NO image. It still serves.
    assert!(read_file("/a/b/c/content/level.txt").is_some());
    // And the deepest path works without anything having created it.
    assert_eq!(
        read_file("/a/b/c/d/e/content/level.txt").as_deref(),
        Some("5")
    );
    // The parent's own content is still reachable beside the child mount.
    assert_eq!(read_file("/a/b/content/level.txt").as_deref(), Some("2"));
    // A path that no mount covers is simply absent.
    assert!(read_file("/a/b/nowhere.txt").is_none());
    drop(f);
}

#[test]
fn a_duplicate_mount_point_is_eexist() {
    let f = Fixture::mount_five("dup");
    let mut handle: i32 = -1;
    // A REAL image at an already-taken point: the refusal must be about
    // the mount point, not about the archive being unreadable.
    let rc = unsafe {
        c_api::tebako_fs_mount_from_file(
            c(&f.images[1]).as_ptr(),
            c_str(LEVELS[1]).as_ptr(),
            &mut handle,
        )
    };
    assert_ne!(rc, 0, "a duplicate mount point must be refused");
    // The refusal is EEXIST by name, not ENOENT or a panic.
    let errno = std::io::Error::last_os_error().raw_os_error();
    assert_eq!(errno, Some(libc::EEXIST), "duplicate mount must be EEXIST");
}

#[test]
fn unmounting_a_middle_level_does_not_cascade() {
    let f = Fixture::mount_five("middle");
    // handles[1] is level 3 (/a/b/c).
    let rc = unsafe { c_api::tebako_fs_unmount_handle(f.handles[1]) };
    assert_eq!(rc, 0, "level 3 must unmount cleanly");

    // Level 3's own prefix is gone…
    assert!(read_file("/a/b/c/content/level.txt").is_none());
    // …while its parent and its DESCENDANTS keep serving.
    assert_eq!(read_file("/a/b/content/level.txt").as_deref(), Some("2"));
    assert_eq!(
        read_file("/a/b/c/d/content/level.txt").as_deref(),
        Some("4")
    );
    assert_eq!(
        read_file("/a/b/c/d/e/content/same.txt").as_deref(),
        Some("level-5")
    );
}

#[test]
fn unmounting_the_deepest_mount_falls_through_to_the_parents_own_tree() {
    let f = Fixture::mount_five("deepest");
    // handles[3] is level 5 (/a/b/c/d/e).
    assert_eq!(unsafe { c_api::tebako_fs_unmount_handle(f.handles[3]) }, 0);
    // "Falling through" is a RELATIVE resolution against the longest
    // remaining prefix (/a/b/c/d), not a re-mount of the child: level 4's
    // image holds `content/`, never `e/content/`, so the path is honestly
    // absent. A table that answered here would be serving bytes from a
    // directory the parent image does not have.
    assert!(
        read_file("/a/b/c/d/e/content/same.txt").is_none(),
        "the parent image does not hold e/, so the path must be absent"
    );
    // The parent's OWN content still answers under the same prefix.
    assert_eq!(
        read_file("/a/b/c/d/content/same.txt").as_deref(),
        Some("level-4")
    );
    // Re-mounting level 5 by name works (the point is free again).
    let mut handle: i32 = -1;
    let rc = unsafe {
        c_api::tebako_fs_mount_from_file(
            c(&f.images[4]).as_ptr(),
            c_str(LEVELS[4]).as_ptr(),
            &mut handle,
        )
    };
    assert_eq!(rc, 0, "a freed mount point must be mountable again");
    assert_eq!(
        read_file("/a/b/c/d/e/content/same.txt").as_deref(),
        Some("level-5")
    );
}

// Keep the c_char import used even where a test only passes its pointer.
#[allow(dead_code)]
fn _ptr(p: *const c_char) -> *const c_char {
    p
}
