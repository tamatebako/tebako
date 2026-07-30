//! Jail acceptance tests (spec 08 — host-access policy) through the
//! `tebako_fs_*` C ABI, proving the three spec profiles against a fixture
//! tree plus the enforcement details: EROFS on ro writes, argument files
//! under deny, symlink-escape failure, gated extraction/mounting, and
//! memfs paths unaffected.
//!
//! All tests serialize on LOCK (the context and the policy are
//! process-global), like the contract suite's RESOURCE_LOCK. Each setup
//! resets both the mount table and the policy, so a panicked sibling does
//! not cascade.

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use tfs::c_api;

static LOCK: Mutex<()> = Mutex::new(());

const MOUNT_POINT: &str = "/__jail_test__";

// --- tiny fixture helpers ------------------------------------------------

/// A temp directory that removes itself on drop (unique per instance).
/// Canonicalized: macOS temp dirs live behind /var -> /private/var, and
/// the policy compares canonical forms.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tfs-jails-test-{tag}-{}-{}",
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

/// Build the standard zip fixture (`content/hello.txt` = "Hello, World!").
fn build_fixture_zip(path: &Path) {
    use std::io::Write as _;
    let file = std::fs::File::create(path).unwrap();
    let mut w = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    w.add_directory("content/", opts).unwrap();
    w.start_file("content/hello.txt", opts).unwrap();
    w.write_all(b"Hello, World!").unwrap();
    w.finish().unwrap();
}

/// The shared host fixture tree:
///
/// ```text
/// <tmp>/work/hello.txt        (scoped rw-mount source)
/// <tmp>/sibling/secret.txt    (never granted)
/// <tmp>/rodir/ro.txt          (ro-mount source)
/// <tmp>/input.txt             (the argument file)
/// <tmp>/test.zip              (the memfs image)
/// ```
struct Fixture {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    work: PathBuf,
    sibling: PathBuf,
    rodir: PathBuf,
    input: PathBuf,
    archive: PathBuf,
}

fn setup(tag: &str) -> Fixture {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Fresh state even if a previous test panicked mid-mount; the policy
    // survives unmount by design, so reset it explicitly to open.
    unsafe {
        c_api::tebako_fs_unmount();
        policy_open();
    }

    let tmp = TempDir::new(tag);
    let work = tmp.0.join("work");
    let sibling = tmp.0.join("sibling");
    let rodir = tmp.0.join("rodir");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::create_dir_all(&rodir).unwrap();
    std::fs::write(work.join("hello.txt"), b"host hello").unwrap();
    std::fs::write(sibling.join("secret.txt"), b"secret").unwrap();
    std::fs::write(rodir.join("ro.txt"), b"ro").unwrap();
    let input = tmp.0.join("input.txt");
    std::fs::write(&input, b"input").unwrap();
    let archive = tmp.0.join("test.zip");
    build_fixture_zip(&archive);

    // The memfs mount: open/stat/opendir answer ENODEV with an empty mount
    // table, so the host-passthrough branch needs one active mount. It
    // doubles as the "memfs unaffected" witness.
    let rc = unsafe {
        c_api::tebako_fs_init_from_file(c(&archive).as_ptr(), c_str(MOUNT_POINT).as_ptr())
    };
    assert_eq!(rc, 0, "init must succeed");

    Fixture {
        _guard: guard,
        _tmp: tmp,
        work,
        sibling,
        rodir,
        input,
        archive,
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            c_api::tebako_fs_unmount();
            policy_open();
        }
    }
}

// --- tiny FFI helpers ----------------------------------------------------

fn c(path: &Path) -> CString {
    CString::new(path.to_str().unwrap()).unwrap()
}

fn c_str(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn errno() -> i32 {
    unsafe { c_api::tebako_get_errno() }
}

/// The thread's C errno (POSIX parity — the ruby io-routing patches
/// read this, not `tebako_get_errno`).
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
fn c_errno() -> i32 {
    unsafe { *libc::__error() }
}

/// The thread's C errno (see above).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn c_errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

/// Reset to the open policy (today's behavior).
unsafe fn policy_open() {
    unsafe {
        c_api::tebako_fs_host_policy(1, std::ptr::null(), 0, std::ptr::null(), 0);
    }
}

/// Install a policy from path lists (access: 0 = ro, 1 = rw).
fn install_policy(default_open: i32, mounts: &[(&Path, &str, i32)], arg_files: &[&Path]) -> i32 {
    let host_strings: Vec<CString> = mounts.iter().map(|(h, _, _)| c(h)).collect();
    let mount_strings: Vec<CString> = mounts.iter().map(|(_, m, _)| c_str(m)).collect();
    let c_mounts: Vec<c_api::TebakoHostMount> = mounts
        .iter()
        .enumerate()
        .map(|(i, (_, _, a))| c_api::TebakoHostMount {
            host: host_strings[i].as_ptr(),
            mount: mount_strings[i].as_ptr(),
            access: *a,
        })
        .collect();
    let file_strings: Vec<CString> = arg_files.iter().map(|f| c(f)).collect();
    let file_ptrs: Vec<*const libc::c_char> = file_strings.iter().map(|f| f.as_ptr()).collect();
    unsafe {
        c_api::tebako_fs_host_policy(
            default_open,
            c_mounts.as_ptr(),
            c_mounts.len(),
            file_ptrs.as_ptr(),
            file_ptrs.len(),
        )
    }
}

fn open(path: &Path, flags: i32) -> i32 {
    unsafe { c_api::tebako_fs_open(c(path).as_ptr(), flags) }
}

fn stat(path: &Path) -> i32 {
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    unsafe { c_api::tebako_fs_stat(c(path).as_ptr(), &mut st) }
}

fn opendir(path: &Path) -> *mut std::ffi::c_void {
    unsafe { c_api::tebako_fs_opendir(c(path).as_ptr()) }
}

fn opendir_str(path: &str) -> *mut std::ffi::c_void {
    unsafe { c_api::tebako_fs_opendir(c_str(path).as_ptr()) }
}

fn memfs_open_hello() -> i32 {
    unsafe {
        c_api::tebako_fs_open(
            c_str(&format!("{MOUNT_POINT}/content/hello.txt")).as_ptr(),
            libc::O_RDONLY,
        )
    }
}

/// Ruby passes literal `lib/../x.yaml` paths through: the mounts must
/// answer the lexical normalization exactly like the host does.
#[test]
fn dot_dot_paths_normalize_lexically() {
    let f = setup("dotdot");
    let _ = &f;
    for path in [
        format!("{MOUNT_POINT}/content/sub/../hello.txt"),
        format!("{MOUNT_POINT}/content/./hello.txt"),
        format!("{MOUNT_POINT}/content/deep/sub/../../hello.txt"),
    ] {
        let fd = unsafe { c_api::tebako_fs_open(c_str(&path).as_ptr(), libc::O_RDONLY) };
        assert!(fd >= 0, "open of {path} must serve the normalized entry");
        assert_eq!(unsafe { c_api::tebako_fs_close(fd) }, 0);
    }
}

// --- the profiles ---------------------------------------------------------

#[test]
fn default_state_and_open_profile_pass_through_unrestricted() {
    let f = setup("open");
    let host_file = f.sibling.join("secret.txt");

    // No tebako_fs_host_policy call at all: the initial policy is open —
    // every host path answers ENOENT ("not ours, pass through"), never
    // EPERM, exactly like before the jail existed.
    assert_eq!(open(&host_file, libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(open(&host_file, libc::O_WRONLY | libc::O_CREAT), -1);
    assert_eq!(errno(), libc::ENOENT);
    assert!(opendir_str("/").is_null());
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(stat(&host_file), -1);
    assert_eq!(errno(), libc::ENOENT);

    // Profile 1 made explicit: identical answers.
    assert_eq!(install_policy(1, &[], &[]), 0);
    assert_eq!(open(&host_file, libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
    assert!(opendir_str("/").is_null());
    assert_eq!(errno(), libc::ENOENT);
}

#[test]
fn deny_all_cannot_enumerate_or_read_but_memfs_is_unaffected() {
    let f = setup("deny");
    assert_eq!(install_policy(0, &[], &[]), 0);

    // Profile 3: cannot even enumerate the root.
    assert!(opendir_str("/").is_null());
    assert_eq!(errno(), libc::EPERM);
    assert!(opendir(&f.sibling).is_null());
    assert_eq!(errno(), libc::EPERM);
    // Reads and stats of host files: EPERM, not ENOENT.
    assert_eq!(open(&f.sibling.join("secret.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
    assert_eq!(stat(&f.sibling.join("secret.txt")), -1);
    assert_eq!(errno(), libc::EPERM);
    // Writes too.
    assert_eq!(
        open(&f.work.join("hello.txt"), libc::O_WRONLY | libc::O_CREAT),
        -1
    );
    assert_eq!(errno(), libc::EPERM);

    // The policy is about HOST paths: memfs is untouched.
    let fd = memfs_open_hello();
    assert!(fd >= 0, "memfs open must still work");
    assert_eq!(unsafe { c_api::tebako_fs_close(fd) }, 0);
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        c_api::tebako_fs_stat(
            c_str(&format!("{MOUNT_POINT}/content/hello.txt")).as_ptr(),
            &mut st,
        )
    };
    assert_eq!(rc, 0, "memfs stat must still work");
    let dir = opendir_str(&format!("{MOUNT_POINT}/content"));
    assert!(!dir.is_null(), "memfs opendir must still work");
    assert_eq!(unsafe { c_api::tebako_fs_closedir(dir) }, 0);
}

/// The v2 app-payload shape (the image mounted at "/"): every host path
/// is COVERED by the mount, but the image holds none of them. A covered
/// path the image does not hold takes the host-passthrough decision
/// (policy-gated), never the mounted-content answers — this is what
/// keeps the host filesystem reachable under a "/" mount.
#[test]
fn covered_but_not_held_paths_fall_through_to_the_host_decision() {
    let f = setup("fallthrough");
    unsafe {
        c_api::tebako_fs_unmount();
    }
    let rc =
        unsafe { c_api::tebako_fs_init_from_file(c(&f.archive).as_ptr(), c_str("/").as_ptr()) };
    assert_eq!(rc, 0, "the image mounts at /");

    // Covered + held: served from the image; a write open is EROFS.
    let fd = unsafe { c_api::tebako_fs_open(c_str("/content/hello.txt").as_ptr(), libc::O_RDONLY) };
    assert!(fd >= 0, "held content serves from the mount");
    assert_eq!(unsafe { c_api::tebako_fs_close(fd) }, 0);
    assert_eq!(open(Path::new("/content/hello.txt"), libc::O_WRONLY), -1);
    assert_eq!(errno(), libc::EROFS, "held content is read-only");

    // Covered + absent: the host decision — ENOENT under the open policy
    // ("not ours, pass through"), for reads AND writes.
    let host_file = f.sibling.join("secret.txt");
    assert_eq!(open(&host_file, libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
    // POSIX parity: the C ABI also writes the thread's C errno — the
    // ruby io-routing patches read it (a stale 0 is Errno::NOERROR).
    assert_eq!(c_errno(), libc::ENOENT);
    assert_eq!(stat(&host_file), -1);
    assert_eq!(errno(), libc::ENOENT);
    assert!(opendir(&f.sibling).is_null());
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(c_errno(), libc::ENOENT);
    // dlmap2file: the extension-loading path takes the same decision —
    // the consumer falls back to the host dlopen.
    let mapped = unsafe { c_api::tebako_fs_dlmap2file(c(&host_file).as_ptr()) };
    assert!(mapped.is_null());
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(open(&host_file, libc::O_WRONLY | libc::O_CREAT), -1);
    assert_eq!(
        errno(),
        libc::ENOENT,
        "absent content passes writes through"
    );

    // The jail still engages on the fall-through: deny answers EPERM…
    assert_eq!(install_policy(0, &[], &[]), 0);
    assert_eq!(open(&host_file, libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
    assert_eq!(stat(&host_file), -1);
    assert_eq!(errno(), libc::EPERM);
    assert!(opendir(&f.sibling).is_null());
    assert_eq!(errno(), libc::EPERM);
    // …while held content stays unaffected.
    let fd = unsafe { c_api::tebako_fs_open(c_str("/content/hello.txt").as_ptr(), libc::O_RDONLY) };
    assert!(fd >= 0, "held content is unaffected by the jail");
    assert_eq!(unsafe { c_api::tebako_fs_close(fd) }, 0);
}

#[test]
fn directory_scoped_mount_rw_works_and_sibling_denies() {
    let f = setup("scoped");
    // Profile 2: the working directory mapped to /work, rw; nothing else.
    assert_eq!(install_policy(0, &[(&f.work, "/work", 1)], &[]), 0);

    // Reads and writes inside the grant: ENOENT = allowed pass-through.
    assert_eq!(open(&f.work.join("hello.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(
        open(&f.work.join("new.txt"), libc::O_WRONLY | libc::O_CREAT),
        -1
    );
    assert_eq!(errno(), libc::ENOENT);
    assert!(opendir(&f.work).is_null());
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(stat(&f.work.join("hello.txt")), -1);
    assert_eq!(errno(), libc::ENOENT);

    // The sibling is outside the grant.
    assert_eq!(open(&f.sibling.join("secret.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
    assert!(opendir(&f.sibling).is_null());
    assert_eq!(errno(), libc::EPERM);
    assert_eq!(stat(&f.sibling.join("secret.txt")), -1);
    assert_eq!(errno(), libc::EPERM);
}

#[test]
fn ro_mount_refuses_writes_with_erofs() {
    let f = setup("ro");
    assert_eq!(install_policy(0, &[(&f.rodir, "/ro", 0)], &[]), 0);

    // Reads: allowed.
    assert_eq!(open(&f.rodir.join("ro.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
    // Writes — existing file and write-create alike: EROFS.
    assert_eq!(open(&f.rodir.join("ro.txt"), libc::O_WRONLY), -1);
    assert_eq!(errno(), libc::EROFS);
    assert_eq!(
        open(&f.rodir.join("new.txt"), libc::O_WRONLY | libc::O_CREAT),
        -1
    );
    assert_eq!(errno(), libc::EROFS);
    // Outside the grant: EPERM.
    assert_eq!(open(&f.sibling.join("secret.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
}

#[test]
fn tight_jail_allows_only_the_argument_file() {
    let f = setup("tight");
    // Profile 3: deny + argument files only.
    assert_eq!(install_policy(0, &[], &[&f.input]), 0);

    assert_eq!(open(&f.input, libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
    // Read-only grant: writing the input file is not part of the deal.
    assert_eq!(open(&f.input, libc::O_WRONLY), -1);
    assert_eq!(errno(), libc::EPERM);
    // Nothing else exists as far as the payload is concerned.
    assert_eq!(open(&f.sibling.join("secret.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
    assert!(opendir_str("/").is_null());
    assert_eq!(errno(), libc::EPERM);
}

#[cfg(unix)]
#[test]
fn symlink_escape_attempt_fails() {
    let f = setup("escape");
    assert_eq!(install_policy(0, &[(&f.work, "/work", 1)], &[]), 0);

    // A symlink inside the granted tree pointing at the sibling.
    std::os::unix::fs::symlink(&f.sibling, f.work.join("evil")).unwrap();
    assert_eq!(
        open(&f.work.join("evil").join("secret.txt"), libc::O_RDONLY),
        -1
    );
    assert_eq!(errno(), libc::EPERM);
    // A file-level symlink escape too.
    std::os::unix::fs::symlink(f.sibling.join("secret.txt"), f.work.join("link.txt")).unwrap();
    assert_eq!(open(&f.work.join("link.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
    // The honest path inside the grant is unaffected.
    assert_eq!(open(&f.work.join("hello.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
}

#[cfg(unix)]
#[test]
fn symlink_swap_after_bind_fails_on_revalidation() {
    let f = setup("swap");
    let data = f.work.join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("f.txt"), b"f").unwrap();
    assert_eq!(install_policy(0, &[(&f.work, "/work", 1)], &[]), 0);
    assert_eq!(open(&data.join("f.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);

    // Swap the real dir for a symlink after the policy was installed: the
    // per-open realpath re-validation must catch it.
    std::fs::remove_dir_all(&data).unwrap();
    std::os::unix::fs::symlink(&f.sibling, &data).unwrap();
    assert_eq!(open(&data.join("secret.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
}

#[test]
fn extract_all_is_a_gated_host_write() {
    let f = setup("extract");
    // Deny-all: extraction to the host is refused.
    assert_eq!(install_policy(0, &[], &[]), 0);
    let dest = f.work.join("out");
    let rc = unsafe { c_api::tebako_fs_extract_all(c(&dest).as_ptr()) };
    assert_eq!(rc, -1);
    assert_eq!(errno(), libc::EPERM);
    assert!(
        !dest.exists(),
        "a denied extraction must not write anything"
    );

    // Scoped rw on /work: the same extraction runs and lands on the host.
    assert_eq!(install_policy(0, &[(&f.work, "/work", 1)], &[]), 0);
    let rc = unsafe { c_api::tebako_fs_extract_all(c(&dest).as_ptr()) };
    assert_eq!(rc, 0);
    assert_eq!(
        std::fs::read(dest.join("content/hello.txt")).unwrap(),
        b"Hello, World!"
    );
}

#[test]
fn mount_family_reads_the_image_through_the_policy() {
    let f = setup("mountgate");
    assert_eq!(install_policy(0, &[], &[]), 0);

    // Mounting another image means reading a host file: denied.
    let mut handle: libc::c_int = -1;
    let rc = unsafe {
        c_api::tebako_fs_mount_from_file(
            c(&f.archive).as_ptr(),
            c_str("/__jail_extra__").as_ptr(),
            &mut handle,
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(errno(), libc::EPERM);

    // Grant the image's directory (ro is enough to mount): allowed.
    let parent = f.archive.parent().unwrap().to_path_buf();
    assert_eq!(install_policy(0, &[(&parent, "/img", 0)], &[]), 0);
    let rc = unsafe {
        c_api::tebako_fs_mount_from_file(
            c(&f.archive).as_ptr(),
            c_str("/__jail_extra__").as_ptr(),
            &mut handle,
        )
    };
    assert_eq!(rc, 0);
    assert!(handle >= 0);
    assert_eq!(unsafe { c_api::tebako_fs_unmount_handle(handle) }, 0);
}

#[test]
fn policy_install_validates_its_arguments() {
    let f = setup("validation");
    // Start from a known deny policy; failed installs must not clobber it.
    assert_eq!(install_policy(0, &[], &[]), 0);

    // Unknown access value.
    assert_eq!(install_policy(0, &[(&f.work, "/work", 2)], &[]), -1);
    assert_eq!(errno(), libc::EINVAL);
    // Relative virtual mount point.
    assert_eq!(install_policy(0, &[(&f.work, "work", 1)], &[]), -1);
    assert_eq!(errno(), libc::EINVAL);
    // Mount source does not exist.
    assert_eq!(
        install_policy(0, &[(&f.work.join("no-such-dir"), "/work", 1)], &[]),
        -1
    );
    assert_eq!(errno(), libc::ENOENT);
    // Argument file does not exist.
    assert_eq!(install_policy(0, &[], &[&f.work.join("no-such-file")]), -1);
    assert_eq!(errno(), libc::ENOENT);
    // NULL pointer with a nonzero count.
    let rc = unsafe { c_api::tebako_fs_host_policy(0, std::ptr::null(), 1, std::ptr::null(), 0) };
    assert_eq!(rc, -1);
    assert_eq!(errno(), libc::EINVAL);
    let rc = unsafe { c_api::tebako_fs_host_policy(0, std::ptr::null(), 0, std::ptr::null(), 1) };
    assert_eq!(rc, -1);
    assert_eq!(errno(), libc::EINVAL);
    // NULL entry inside the mounts array.
    let work_mp = c_str("/work");
    let bad = c_api::TebakoHostMount {
        host: std::ptr::null(),
        mount: work_mp.as_ptr(),
        access: 1,
    };
    let rc = unsafe { c_api::tebako_fs_host_policy(0, &bad, 1, std::ptr::null(), 0) };
    assert_eq!(rc, -1);
    assert_eq!(errno(), libc::EINVAL);

    // All those failures left the deny policy in place…
    assert_eq!(open(&f.sibling.join("secret.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);

    // …and a valid install reports success with a clean errno.
    assert_eq!(install_policy(1, &[], &[]), 0);
    assert_eq!(errno(), 0);
}

#[test]
fn policy_survives_unmount_fail_closed() {
    let f = setup("survive");
    assert_eq!(install_policy(0, &[], &[]), 0);

    // Tearing the namespace down must not open the jail.
    unsafe { c_api::tebako_fs_unmount() };
    let rc = unsafe {
        c_api::tebako_fs_init_from_file(c(&f.archive).as_ptr(), c_str(MOUNT_POINT).as_ptr())
    };
    assert_eq!(rc, -1, "even the image read is gated once a policy is set");
    assert_eq!(errno(), libc::EPERM);

    // With the image dir granted (ro), remounting works, writes into the
    // ro grant are refused, and the world outside the grant stays denied.
    let parent = f.archive.parent().unwrap().to_path_buf();
    assert_eq!(install_policy(0, &[(&parent, "/img", 0)], &[]), 0);
    let rc = unsafe {
        c_api::tebako_fs_init_from_file(c(&f.archive).as_ptr(), c_str(MOUNT_POINT).as_ptr())
    };
    assert_eq!(rc, 0);
    assert_eq!(open(&f.archive, libc::O_WRONLY), -1);
    assert_eq!(errno(), libc::EROFS);
    assert!(opendir_str("/").is_null());
    assert_eq!(errno(), libc::EPERM);
}

// --- the audit journal (spec 08 §2) -------------------------------------

/// Read the journal file's lines, asserting the shared shape: `<unix
/// seconds> event=jail-deny path=<p> op=<read|write> source=<s>`.
fn journal_lines(log: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(log).unwrap();
    text.lines().map(|l| l.to_string()).collect()
}

fn assert_line_shape(line: &str, path: &Path, op: &str, source: &str) {
    let (ts, rest) = line.split_once(' ').unwrap();
    assert!(
        !ts.is_empty() && ts.bytes().all(|b| b.is_ascii_digit()),
        "line must start with the unix seconds: {line}"
    );
    assert_eq!(
        rest,
        format!(
            "event=jail-deny path={} op={} source={}",
            path.display(),
            op,
            source
        )
    );
}

#[test]
fn violations_are_journaled_with_path_op_and_source() {
    let f = setup("journal");
    let log = f.input.parent().unwrap().join("audit").join("journal.log");
    std::env::set_var("TEBAKO_JAIL_JOURNAL", &log);
    std::env::set_var("TEBAKO_JAIL_SOURCE", "manifest+user");
    let cleanup = || {
        std::env::remove_var("TEBAKO_JAIL_JOURNAL");
        std::env::remove_var("TEBAKO_JAIL_SOURCE");
    };

    // deny: a read, a stat and a write are three denials, three lines.
    assert_eq!(install_policy(0, &[], &[]), 0);
    assert_eq!(open(&f.sibling.join("secret.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);
    assert_eq!(stat(&f.sibling.join("secret.txt")), -1);
    assert_eq!(errno(), libc::EPERM);
    assert_eq!(
        open(&f.work.join("new.txt"), libc::O_WRONLY | libc::O_CREAT),
        -1
    );
    assert_eq!(errno(), libc::EPERM);

    let lines = journal_lines(&log);
    assert_eq!(lines.len(), 3, "journal: {lines:?}");
    assert_line_shape(
        &lines[0],
        &f.sibling.join("secret.txt"),
        "read",
        "manifest+user",
    );
    assert_line_shape(
        &lines[1],
        &f.sibling.join("secret.txt"),
        "read",
        "manifest+user",
    );
    assert_line_shape(&lines[2], &f.work.join("new.txt"), "write", "manifest+user");

    // An ro-write refusal (EROFS) is a violation too — journaled with the
    // same shape; allowed passes never journal.
    assert_eq!(install_policy(1, &[(&f.rodir, "/ro", 0)], &[]), 0);
    assert_eq!(open(&f.rodir.join("ro.txt"), libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT, "allowed read keeps the pass-through");
    assert_eq!(
        open(&f.rodir.join("new.txt"), libc::O_WRONLY | libc::O_CREAT),
        -1
    );
    assert_eq!(errno(), libc::EROFS);

    let lines = journal_lines(&log);
    assert_eq!(lines.len(), 4, "journal: {lines:?}");
    assert_line_shape(
        &lines[3],
        &f.rodir.join("new.txt"),
        "write",
        "manifest+user",
    );

    cleanup();
}

#[test]
fn no_policy_no_journal_and_the_journal_never_fails_the_op() {
    let f = setup("journal-quiet");
    let log = f.input.parent().unwrap().join("audit2").join("journal.log");
    std::env::set_var("TEBAKO_JAIL_JOURNAL", &log);

    // The open policy never denies — and never journals.
    let host_file = f.sibling.join("secret.txt");
    assert_eq!(open(&host_file, libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::ENOENT);
    assert!(!log.exists(), "no policy, no journal file");

    // An UNWRITABLE journal target (a directory) does not change the
    // denial's answer: EPERM stands, best-effort journaling is swallowed.
    std::env::set_var("TEBAKO_JAIL_JOURNAL", f.rodir.as_path());
    std::env::set_var("TEBAKO_JAIL_SOURCE", "TEBAKO_JAIL");
    assert_eq!(install_policy(0, &[], &[]), 0);
    assert_eq!(open(&host_file, libc::O_RDONLY), -1);
    assert_eq!(errno(), libc::EPERM);

    std::env::remove_var("TEBAKO_JAIL_JOURNAL");
    std::env::remove_var("TEBAKO_JAIL_SOURCE");
}
