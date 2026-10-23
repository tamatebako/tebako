//! Symlink semantics through the C ABI (spec 11 §9) — the consumer end of
//! an image that contains links.
//!
//! The images here are real limnifs images written by the same writer the
//! press uses, because the defect this file pins was invisible to every
//! other test: the writer emitted correct `S_IFLNK` inodes and the C ABI
//! answered `EINVAL` for them, so `stat` said "missing", `ls` printed a
//! 0-byte regular file, and a runtime opening a link path found nothing.
//!
//! What is pinned:
//!
//! - **`lstat` reports the link** — `S_IFLNK`, `st_size` = target length,
//!   and `readlink` returns the stored target (relative stays relative).
//! - **`stat` follows** — the target's metadata answers, and `open`
//!   reads the target's bytes, so a runtime that names a link works.
//! - **absolute targets go through the router** — a link in one mount
//!   naming `/data/value.txt` lands in the mount that owns `/data`.
//! - **a cycle is `ELOOP`**, not a hang.
//! - **`opendir` follows a link to a directory.**
//!
//! All tests serialize on `LOCK`: the mount table is process-global.

// The fixtures are limnifs images, so the backend that reads them is the
// gate: a --no-default-features build has no limnifs backend and there is
// nothing to mount.
#![cfg(all(unix, feature = "backend-limnifs"))]
// The `st_mode`/`S_IF*` width story the crate's own files carry: the casts
// are required on macOS (u16) and identities on Linux, so clippy's
// unnecessary_cast is allowed here deliberately.
#![allow(clippy::unnecessary_cast)]

use std::ffi::{c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use tfs::c_api;

static LOCK: Mutex<()> = Mutex::new(());

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tfs-symlink-test-{tag}-{}-{}",
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

fn c_str(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Build a limnifs image from `entries`: each is `(path, content)` for a
/// regular file, or `(path, '@' + target)` for a symlink.
fn build_image(path: &Path, entries: &[(&str, &str)]) {
    let tree = TempDir::new("tree");
    for (rel, value) in entries {
        let full = tree.0.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        match value.strip_prefix('@') {
            Some(target) => std::os::unix::fs::symlink(target, &full).unwrap(),
            None => std::fs::write(&full, value).unwrap(),
        }
    }
    let artifact = limnifs_write::write_directory_with_config(
        &tree.0,
        &limnifs_write::WriteConfig::default_v0_1(),
    )
    .expect("write image");
    let mut bytes = artifact.bytes;
    for slab in &artifact.slabs {
        bytes.extend_from_slice(&slab.bytes);
    }
    std::fs::write(path, bytes).unwrap();
}

struct Fixture {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
}

impl Fixture {
    /// Mount one image at `/app` (and optionally a second at `/data`).
    fn mount(tag: &str, entries: &[(&str, &str)]) -> Fixture {
        Fixture::mount_pair(tag, entries, None)
    }

    fn mount_pair(
        tag: &str,
        entries: &[(&str, &str)],
        second: Option<(&str, &[(&str, &str)])>,
    ) -> Fixture {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { c_api::tebako_fs_unmount() };
        let tmp = TempDir::new(tag);
        let app = tmp.0.join("app.tfs");
        build_image(&app, entries);
        let rc = unsafe {
            c_api::tebako_fs_init_from_file(c_str_path(&app).as_ptr(), c_str("/app").as_ptr())
        };
        assert_eq!(rc, 0, "the app image must mount");
        if let Some((point, other)) = second {
            let data = tmp.0.join("data.tfs");
            build_image(&data, other);
            let mut handle: c_int = -1;
            let rc = unsafe {
                c_api::tebako_fs_mount_from_file(
                    c_str_path(&data).as_ptr(),
                    c_str(point).as_ptr(),
                    &mut handle,
                )
            };
            assert_eq!(rc, 0, "the second image must mount at {point}");
        }
        Fixture {
            _guard: guard,
            _tmp: tmp,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe { c_api::tebako_fs_unmount() };
    }
}

fn c_str_path(path: &Path) -> CString {
    CString::new(path.to_str().unwrap()).unwrap()
}

fn lstat(path: &str) -> Result<libc::stat, c_int> {
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { c_api::tebako_fs_lstat(c_str(path).as_ptr(), &mut st) };
    if rc == 0 {
        Ok(st)
    } else {
        Err(unsafe { c_api::tebako_get_errno() })
    }
}

fn stat(path: &str) -> Result<libc::stat, c_int> {
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { c_api::tebako_fs_stat(c_str(path).as_ptr(), &mut st) };
    if rc == 0 {
        Ok(st)
    } else {
        Err(unsafe { c_api::tebako_get_errno() })
    }
}

fn readlink(path: &str) -> Result<String, c_int> {
    let mut buf = vec![0u8; 256];
    let n = unsafe {
        c_api::tebako_fs_readlink(c_str(path).as_ptr(), buf.as_mut_ptr().cast(), buf.len())
    };
    if n < 0 {
        return Err(unsafe { c_api::tebako_get_errno() });
    }
    buf.truncate(n as usize);
    Ok(String::from_utf8(buf).unwrap())
}

fn read_file(path: &str) -> Result<String, c_int> {
    let fd = unsafe { c_api::tebako_fs_open(c_str(path).as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return Err(unsafe { c_api::tebako_get_errno() });
    }
    let mut buf = vec![0u8; 256];
    let n = unsafe { c_api::tebako_fs_read(fd, buf.as_mut_ptr().cast::<c_void>(), buf.len()) };
    unsafe { c_api::tebako_fs_close(fd) };
    if n < 0 {
        return Err(unsafe { c_api::tebako_get_errno() });
    }
    buf.truncate(n as usize);
    Ok(String::from_utf8(buf).unwrap())
}

fn mode_type(st: &libc::stat) -> u32 {
    st.st_mode as u32 & libc::S_IFMT as u32
}

#[test]
fn lstat_reports_the_link_and_stat_follows_it() {
    let _f = Fixture::mount(
        "follow",
        &[
            ("lib/tool/run.sh", "tool-bytes"),
            ("bin/tool", "@../lib/tool/run.sh"),
        ],
    );

    // lstat: the entry itself.
    let link = lstat("/app/bin/tool").expect("lstat must see the link");
    assert_eq!(mode_type(&link), libc::S_IFLNK as u32, "S_IFLNK expected");
    assert_eq!(
        link.st_size as usize,
        "../lib/tool/run.sh".len(),
        "a link's st_size is its target's length"
    );

    // readlink: the stored target, verbatim (relative stays relative).
    assert_eq!(
        readlink("/app/bin/tool").expect("readlink"),
        "../lib/tool/run.sh"
    );

    // stat: the target answers.
    let followed = stat("/app/bin/tool").expect("stat must follow the link");
    assert_eq!(mode_type(&followed), libc::S_IFREG as u32);
    assert_eq!(followed.st_size, "tool-bytes".len() as i64);

    // open: the target's bytes.
    assert_eq!(
        read_file("/app/bin/tool").expect("open through a link"),
        "tool-bytes"
    );

    // readlink of a regular file is EINVAL, as POSIX says.
    assert_eq!(readlink("/app/lib/tool/run.sh").unwrap_err(), libc::EINVAL);
}

#[test]
fn an_absolute_target_resolves_through_the_mount_table() {
    let _f = Fixture::mount_pair(
        "router",
        &[("bin/tool", "@/data/value.txt")],
        Some(("/data", &[("value.txt", "from-the-data-mount")])),
    );

    // The link lives in /app and names a path only /data holds: the
    // router must land there (spec 11 §9's cross-mount case).
    let followed = stat("/app/bin/tool").expect("the absolute target must route");
    assert_eq!(mode_type(&followed), libc::S_IFREG as u32);
    assert_eq!(
        read_file("/app/bin/tool").expect("open through the cross-mount link"),
        "from-the-data-mount"
    );
}

#[test]
fn a_cycle_is_eloop_not_a_hang() {
    let _f = Fixture::mount("cycle", &[("a", "@b"), ("b", "@a")]);

    assert_eq!(stat("/app/a").unwrap_err(), libc::ELOOP);
    assert_eq!(read_file("/app/a").unwrap_err(), libc::ELOOP);
    // lstat and readlink still answer: they never follow.
    assert_eq!(mode_type(&lstat("/app/a").unwrap()), libc::S_IFLNK as u32);
    assert_eq!(readlink("/app/a").unwrap(), "b");
}

#[test]
fn a_link_to_a_directory_opens_through_opendir() {
    let _f = Fixture::mount(
        "dir",
        &[("share/real/one.txt", "one"), ("share/alias", "@real")],
    );

    let dir = unsafe { c_api::tebako_fs_opendir(c_str("/app/share/alias").as_ptr()) };
    assert!(
        !dir.is_null(),
        "opendir must follow a link to a directory (errno {})",
        unsafe { c_api::tebako_get_errno() }
    );
    let mut names = Vec::new();
    loop {
        let entry = unsafe { c_api::tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        let cur = unsafe { &*entry };
        let end = cur
            .d_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(cur.d_name.len());
        names.push(
            cur.d_name[..end]
                .iter()
                .map(|&c| c as c_int as u8 as char)
                .collect::<String>(),
        );
    }
    unsafe { c_api::tebako_fs_closedir(dir) };
    assert_eq!(names, vec!["one.txt".to_string()]);
}

#[test]
fn a_dangling_link_is_found_by_lstat_and_missing_by_stat() {
    let _f = Fixture::mount("dangling", &[("gone", "@nowhere/target.txt")]);

    assert_eq!(
        mode_type(&lstat("/app/gone").unwrap()),
        libc::S_IFLNK as u32
    );
    assert_eq!(readlink("/app/gone").unwrap(), "nowhere/target.txt");
    assert_eq!(stat("/app/gone").unwrap_err(), libc::ENOENT);
    assert_eq!(read_file("/app/gone").unwrap_err(), libc::ENOENT);
}

#[test]
fn an_undersized_readlink_buffer_is_erange_not_truncation() {
    let _f = Fixture::mount("erange", &[("long", "@aaaaaaaaaaaaaaaaaaaaaaaaaaaa")]);

    let mut buf = [0u8; 4];
    let n = unsafe {
        c_api::tebako_fs_readlink(
            c_str("/app/long").as_ptr(),
            buf.as_mut_ptr().cast(),
            buf.len(),
        )
    };
    assert_eq!(n, -1, "a short buffer must fail, never silently truncate");
    assert_eq!(unsafe { c_api::tebako_get_errno() }, libc::ERANGE);
}

#[test]
fn open_with_o_nofollow_refuses_the_link() {
    let _f = Fixture::mount(
        "nofollow",
        &[("real.txt", "payload"), ("link.txt", "@real.txt")],
    );

    // POSIX: the final component is a link and the caller said not to
    // follow it, so the answer is ELOOP — not an fd over a link entry.
    let fd = unsafe {
        c_api::tebako_fs_open(
            c_str("/app/link.txt").as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW,
        )
    };
    assert_eq!(fd, -1, "O_NOFOLLOW on a link must not hand out an fd");
    assert_eq!(unsafe { c_api::tebako_get_errno() }, libc::ELOOP);

    // A regular file is unaffected by the flag.
    let fd = unsafe {
        c_api::tebako_fs_open(
            c_str("/app/real.txt").as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW,
        )
    };
    assert!(fd >= 0, "O_NOFOLLOW on a regular file must open it");
    unsafe { c_api::tebako_fs_close(fd) };
}
