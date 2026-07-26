//! Platform helpers: identity mapping, file ops, the install lock, and the
//! curl-CLI download (platform-native TLS at zero binary cost).

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

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

pub fn os_rename(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::rename(src, dst)
}

pub fn remove_file(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path)
}

#[allow(unused)]
pub fn remove_dir_all(path: &Path) -> io::Result<()> {
    std::fs::remove_dir_all(path)
}

pub fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = path;
}

pub fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::copy(src, dst).map(|_| ())
}

pub fn write_small_file(path: &Path, content: &str) -> io::Result<()> {
    std::fs::write(path, content)
}

/// curl CLI (present on modern macOS/Linux/Windows 10+): platform-native
/// TLS without a line of TLS code in the binary.
#[allow(clippy::result_unit_err)] // C-style -1 error by design
pub fn spawn_curl(url: &str, out: &Path) -> Result<(), ()> {
    let status = std::process::Command::new("curl")
        .args(["-fSLsS", "--retry", "3", "--connect-timeout", "30", "-o"])
        .arg(out)
        .arg(url)
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => Err(()),
    }
}

// ---------------------------------------------------------------------
// per-entry install lock
// ---------------------------------------------------------------------

pub enum EntryLock {
    #[cfg(unix)]
    Fd(std::fs::File),
    #[cfg(windows)]
    #[allow(dead_code)]
    Unsupported,
}

const LOCK_POLL_MS: u64 = 200;

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
            return Err(io::Error::new(io::ErrorKind::TimedOut, "lock timeout"));
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_POLL_MS));
    }
}

#[cfg(windows)]
pub fn flock_acquire(_path: &Path, _timeout_ms: u64) -> io::Result<EntryLock> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "install lock is not implemented on this platform in v1",
    ))
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
        EntryLock::Unsupported => {}
    }
}

#[allow(unused)]
fn _keep_imports(_: PathBuf, _: &mut dyn Read, _: &mut dyn Write) {}
