//! Platform helpers: identity mapping, file ops, the install lock, and
//! the exec handoff.
//!
//! The public surface is behavior-identical on unix and Windows (the
//! launcher ABI and the named exit codes do not depend on the OS). All
//! Win32 FFI of the crate lives in this file — the only `unsafe` in
//! tebako-bootstrap — behind the same safe signatures the unix side
//! exposes.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{ERROR_IO_PENDING, ERROR_LOCK_VIOLATION, FALSE, TRUE};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    GetFileAttributesW, LockFileEx, MoveFileExW, SetFileAttributesW, UnlockFileEx,
    FILE_ATTRIBUTE_READONLY, INVALID_FILE_ATTRIBUTES, LOCKFILE_EXCLUSIVE_LOCK,
    LOCKFILE_FAIL_IMMEDIATELY, MOVEFILE_REPLACE_EXISTING,
};
#[cfg(windows)]
use windows_sys::Win32::System::Console::{SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_C_EVENT};
#[cfg(windows)]
use windows_sys::Win32::System::IO::OVERLAPPED;

/// Runtime-package platform string; must match tebako-runtime-ruby's asset
/// naming. glibc vs musl is a compile-time property (target_env).
pub fn platform_string() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x86_64";
    #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
    return "linux-gnu-x86_64";
    #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "aarch64"))]
    return "linux-gnu-arm64";
    #[cfg(all(target_os = "linux", target_env = "musl", target_arch = "x86_64"))]
    return "linux-musl-x86_64";
    #[cfg(all(target_os = "linux", target_env = "musl", target_arch = "aarch64"))]
    return "linux-musl-arm64";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x86_64";
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"),
        all(target_os = "linux", target_env = "gnu", target_arch = "aarch64"),
        all(target_os = "linux", target_env = "musl", target_arch = "x86_64"),
        all(target_os = "linux", target_env = "musl", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    compile_error!("unsupported platform");
}

pub fn exe_suffix() -> &'static str {
    #[cfg(windows)]
    return ".exe";
    #[cfg(not(windows))]
    return "";
}

pub fn file_exists(path: &Path) -> bool {
    path.exists()
}

pub fn mkdir_p(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(not(windows))]
pub fn os_rename(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::rename(src, dst)
}

/// Windows: MoveFileExW with MOVEFILE_REPLACE_EXISTING — rename(2)
/// semantics (atomic replace within a volume), matching what
/// std::fs::rename gives the unix side.
///
/// Antivirus scanners and the search indexer transiently hold handles on
/// files the bootstrap just wrote, which surfaces as
/// ERROR_SHARING_VIOLATION; that error — and only that error — is
/// retried on a fixed budget: up to 50 attempts spaced 100 ms (a 5 s
/// worst case) before the failure is returned. Every other error
/// (including ERROR_ACCESS_DENIED, which is not a transient-contention
/// signal here) fails immediately.
#[cfg(windows)]
pub fn os_rename(src: &Path, dst: &Path) -> io::Result<()> {
    let wsrc = wide(src);
    let wdst = wide(dst);
    let mut attempt = 0u32;
    loop {
        let ok = unsafe { MoveFileExW(wsrc.as_ptr(), wdst.as_ptr(), MOVEFILE_REPLACE_EXISTING) };
        if ok != FALSE {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        attempt += 1;
        if attempt >= RENAME_MAX_ATTEMPTS
            || !is_transient_rename_error(err.raw_os_error().unwrap_or(0))
        {
            return Err(err);
        }
        std::thread::sleep(std::time::Duration::from_millis(RENAME_RETRY_DELAY_MS));
    }
}

/// The rename retry policy: ERROR_SHARING_VIOLATION (32) is the one
/// transient-contention error MoveFileExW reports (see os_rename).
/// Pure decision logic, host-testable; the numbers are the Win32 codes.
#[cfg(any(windows, test))]
const ERROR_SHARING_VIOLATION_RAW: i32 = 32;
#[cfg(any(windows, test))]
const RENAME_MAX_ATTEMPTS: u32 = 50;
#[cfg(any(windows, test))]
const RENAME_RETRY_DELAY_MS: u64 = 100;

#[cfg(any(windows, test))]
fn is_transient_rename_error(raw_os_error: i32) -> bool {
    raw_os_error == ERROR_SHARING_VIOLATION_RAW
}

#[cfg(not(windows))]
pub fn remove_file(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path)
}

/// Windows: DeleteFile refuses FILE_ATTRIBUTE_READONLY files while
/// unlink(2) removes a 0444 file without blinking — the cache installs
/// read-only runtime images (item 30b), and tmp-cleanup must sweep them
/// exactly like the unix side does. On PermissionDenied, clear the
/// read-only attribute and retry once; the original error otherwise.
#[cfg(windows)]
pub fn remove_file(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            let w = wide(path);
            let cleared = unsafe {
                let attrs = GetFileAttributesW(w.as_ptr());
                attrs != INVALID_FILE_ATTRIBUTES
                    && attrs & FILE_ATTRIBUTE_READONLY != 0
                    && SetFileAttributesW(w.as_ptr(), attrs & !FILE_ATTRIBUTE_READONLY) != FALSE
            };
            if cleared {
                std::fs::remove_file(path)
            } else {
                Err(e)
            }
        }
        Err(e) => Err(e),
    }
}

#[allow(unused)]
pub fn remove_dir_all(path: &Path) -> io::Result<()> {
    std::fs::remove_dir_all(path)
}

/// Unix: rwxr-xr-x. Windows: executability is by extension (.exe) and
/// PATHEXT, not permission bits — no-op by design (ACLs untouched).
pub fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// The runtime image is an immutable cache artifact (item 30b).
/// Best-effort, like the unix chmod: callers proceed either way.
#[cfg(unix)]
pub fn make_readonly(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444));
}

/// Windows: FILE_ATTRIBUTE_READONLY via SetFileAttributesW, preserving
/// every other attribute bit.
#[cfg(windows)]
pub fn make_readonly(path: &Path) {
    let w = wide(path);
    unsafe {
        let attrs = GetFileAttributesW(w.as_ptr());
        if attrs != INVALID_FILE_ATTRIBUTES {
            SetFileAttributesW(w.as_ptr(), attrs | FILE_ATTRIBUTE_READONLY);
        }
    }
}

pub fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::copy(src, dst).map(|_| ())
}

pub fn write_small_file(path: &Path, content: &str) -> io::Result<()> {
    std::fs::write(path, content)
}

/// Wide (UTF-16, NUL-terminated) form of a path for the *W APIs. No
/// lossy conversion anywhere: OsStr on Windows already is UTF-16-shaped,
/// and every std::fs call above keeps paths as OsStr/OsString end to end.
#[cfg(windows)]
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt as _;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

// ---------------------------------------------------------------------
// per-entry install lock
// ---------------------------------------------------------------------

pub enum EntryLock {
    #[cfg(unix)]
    Fd(std::fs::File),
    #[cfg(windows)]
    File(std::fs::File),
}

const LOCK_POLL_MS: u64 = 200;

/// The lock-timeout error both platforms produce; lib.rs maps its kind
/// onto EX_TEBAKO_UNAVAILABLE with the stale-lock hint message.
fn lock_timeout() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "lock timeout")
}

#[cfg(unix)]
pub fn flock_acquire(path: &Path, timeout_ms: u64) -> io::Result<EntryLock> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&f);
    loop {
        let rc = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if rc == 0 {
            return Ok(EntryLock::Fd(f));
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EWOULDBLOCK) && err.kind() != io::ErrorKind::Interrupted
        {
            return Err(err);
        }
        if err.kind() != io::ErrorKind::Interrupted && std::time::Instant::now() >= deadline {
            return Err(lock_timeout());
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_POLL_MS));
    }
}

/// Windows: LockFileEx on one byte at offset 0 of the lock file, the
/// same semantics as the unix flock — exclusive, non-blocking attempts
/// on a 200 ms poll until `timeout_ms` is exhausted (the caller maps the
/// timeout onto EX_TEBAKO_UNAVAILABLE with the stale-lock hint). The
/// handle is a plain synchronous one (hEvent NULL in the OVERLAPPED),
/// 1:1 with the C++ bootstrap's lock_acquire; a crashed process's lock
/// is released by the kernel when the handle dies, exactly like flock.
#[cfg(windows)]
pub fn flock_acquire(path: &Path, timeout_ms: u64) -> io::Result<EntryLock> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    let handle = std::os::windows::io::AsRawHandle::as_raw_handle(&f);
    loop {
        let mut ov = OVERLAPPED::default();
        let ok = unsafe {
            LockFileEx(
                handle,
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                1,
                0,
                &mut ov,
            )
        };
        if ok != FALSE {
            return Ok(EntryLock::File(f));
        }
        let err = io::Error::last_os_error();
        let raw = err.raw_os_error().unwrap_or(0);
        if raw != ERROR_LOCK_VIOLATION as i32 && raw != ERROR_IO_PENDING as i32 {
            return Err(err);
        }
        if std::time::Instant::now() >= deadline {
            return Err(lock_timeout());
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_POLL_MS));
    }
}

pub fn lock_release(lock: EntryLock) {
    match lock {
        #[cfg(unix)]
        EntryLock::Fd(f) => {
            let fd = std::os::unix::io::AsRawFd::as_raw_fd(&f);
            unsafe {
                libc::flock(fd, libc::LOCK_UN);
            }
        }
        #[cfg(windows)]
        EntryLock::File(f) => {
            let handle = std::os::windows::io::AsRawHandle::as_raw_handle(&f);
            let mut ov = OVERLAPPED::default();
            unsafe {
                UnlockFileEx(handle, 0, 1, 0, &mut ov);
            }
            // dropping f closes the handle, releasing the lock regardless
        }
    }
}

// ---------------------------------------------------------------------
// exec handoff
// ---------------------------------------------------------------------

/// Windows has no execve(2): the runtime is spawned as a child process
/// (std::process::Command → CreateProcessW, stdio inherited), the
/// bootstrap waits for it and exits with the child's exit code — the
/// user sees the runtime's code, while loader-side failures (65–74)
/// keep originating loader-side before this point. Never returns on
/// success; the io::Error return is the spawn/wait failure, mapped by
/// the caller onto EX_TEBAKO_IO with the same message body as the unix
/// exec failure ("cannot execute runtime …").
#[cfg(windows)]
pub fn spawn_handoff(
    runtime: &Path,
    args: &[String],
    image: Option<&Path>,
    jail: Option<&crate::JailEnv>,
) -> io::Error {
    install_ctrl_swallow();
    let mut cmd = std::process::Command::new(runtime);
    cmd.args(args);
    if let Some(image) = image {
        // item 30b: the runtime image rides the environment; image-era
        // drivers mount it instead of an embedded image, v1 drivers
        // ignore it. The handoff options themselves are unchanged.
        cmd.env("TEBAKO_RUNTIME_IMAGE", image);
    }
    if let Some(jail) = jail {
        // spec 08: the effective jail rides the environment (the unix
        // exec path exports the same triple).
        cmd.env("TEBAKO_JAIL", &jail.spec);
        cmd.env("TEBAKO_JAIL_SOURCE", jail.source);
        cmd.env("TEBAKO_JAIL_JOURNAL", &jail.journal);
    }
    let status = match cmd.spawn().and_then(|mut child| child.wait()) {
        Ok(status) => status,
        Err(e) => return e,
    };
    // ExitStatus::code() is always Some for a waited Windows child; the
    // fallback keeps the no-unwrap discipline if that ever changes.
    std::process::exit(status.code().unwrap_or(1));
}

/// Console Ctrl handling for the spawn handoff. The child shares our
/// console process group, so CTRL_C/CTRL_BREAK events are delivered to
/// it directly by the console (the runtime sees its normal SIGINT); the
/// bootstrap must outlive the child to propagate its exit code, so its
/// own copy of those events is swallowed. Forwarding with
/// GenerateConsoleCtrlEvent to a CREATE_NEW_PROCESS_GROUP child was
/// considered and rejected: CTRL_C_EVENT cannot be generated for a
/// process group (MSDN), and detaching the child from console Ctrl
/// events would change the runtime's signal behavior.
#[cfg(windows)]
unsafe extern "system" fn ctrl_swallow(ctrl_type: u32) -> windows_sys::core::BOOL {
    match ctrl_type {
        CTRL_C_EVENT | CTRL_BREAK_EVENT => TRUE,
        // CLOSE/LOGOFF/SHUTDOWN keep the default processing (terminate).
        _ => FALSE,
    }
}

#[cfg(windows)]
fn install_ctrl_swallow() {
    // Best-effort: without the handler a console Ctrl event kills the
    // bootstrap before the child is reaped, but the handoff itself still
    // works — never fail an exec over it.
    unsafe {
        SetConsoleCtrlHandler(Some(ctrl_swallow), TRUE);
    }
}

#[allow(unused)]
fn _keep_imports(_: PathBuf, _: &mut dyn Read, _: &mut dyn Write) {}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let uniq = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "tebako-boot-plat-{tag}-{}-{uniq}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn exe_suffix_matches_the_platform() {
        #[cfg(windows)]
        assert_eq!(exe_suffix(), ".exe");
        #[cfg(not(windows))]
        assert_eq!(exe_suffix(), "");
    }

    #[test]
    fn platform_string_is_a_known_asset_triple() {
        let known = [
            "macos-arm64",
            "macos-x86_64",
            "linux-gnu-x86_64",
            "linux-gnu-arm64",
            "linux-musl-x86_64",
            "linux-musl-arm64",
            "windows-x86_64",
        ];
        assert!(known.contains(&platform_string()));
    }

    #[test]
    fn rename_retry_policy_is_the_documented_budget() {
        // documented contract: ~50 tries × 100 ms (a 5 s worst case),
        // ERROR_SHARING_VIOLATION (32) only
        assert_eq!(RENAME_MAX_ATTEMPTS, 50);
        assert_eq!(RENAME_RETRY_DELAY_MS, 100);
        assert!(u64::from(RENAME_MAX_ATTEMPTS) * RENAME_RETRY_DELAY_MS <= 5_000);
        assert!(is_transient_rename_error(32));
        for code in [0, 2, 3, 5, 33, 80, 183, -1] {
            assert!(
                !is_transient_rename_error(code),
                "error {code} must not be retried"
            );
        }
    }

    #[test]
    fn lock_timeout_error_shape_is_stable() {
        let e = lock_timeout();
        assert_eq!(e.kind(), io::ErrorKind::TimedOut);
        assert_eq!(e.to_string(), "lock timeout");
    }

    #[test]
    fn os_rename_replaces_an_existing_destination() {
        let t = TempDir::new("rename");
        let src = t.0.join("a");
        let dst = t.0.join("b");
        std::fs::write(&src, b"new").unwrap();
        std::fs::write(&dst, b"old").unwrap();
        os_rename(&src, &dst).unwrap();
        assert!(!file_exists(&src));
        assert_eq!(std::fs::read(&dst).unwrap(), b"new");
    }

    #[test]
    fn remove_file_sweeps_readonly_files_like_unix_unlink() {
        let t = TempDir::new("rm-ro");
        let f = t.0.join("image.tfs");
        std::fs::write(&f, b"img").unwrap();
        make_readonly(&f);
        remove_file(&f).unwrap();
        assert!(!file_exists(&f));
    }

    #[test]
    fn entry_lock_excludes_a_second_holder_until_released() {
        let t = TempDir::new("lock");
        let lock_path = t.0.join("entry.lock");
        let lock = flock_acquire(&lock_path, 1_000).expect("first acquire");
        match flock_acquire(&lock_path, 300) {
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::TimedOut, "{e}"),
            Ok(_) => panic!("second acquire must time out while the first lock is held"),
        }
        lock_release(lock);
        flock_acquire(&lock_path, 1_000).expect("acquire after release");
    }
}
