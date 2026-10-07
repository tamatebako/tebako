//! `tebako trust add | list | remove` — the operator CA certificate
//! store (`$TEBAKO_HOME/trust/ca/<name>.pem`, tebako#541; spec 22 §4's
//! store arm). The driver's boot merges a non-empty store into the
//! runtime's TLS trust input; these verbs are the store's only write
//! path.
//!
//! Discipline (the store's own rules, mirroring the payload cache):
//! validated PEM at the boundary (every CERTIFICATE block parses, at
//! least one exists — a malformed file never enters the store), one
//! per-file flock for the mutation window (120 s, the store's lock
//! grammar), tmp + rename so a partial write is invisible, read-only
//! artifacts, journaled events. Identical bytes under the same name are
//! a no-op note; a name held by DIFFERENT bytes refuses (remove first —
//! never a silent replace). `list` is a report: it marks a corrupt
//! entry rather than failing (the boot path fails closed on it).

use std::path::Path;

use crate::error::TebakoError;
use crate::install::journal;

const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_IO: i32 = 74;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, code)
}

/// The io class (74) naming the path.
fn io_err(path: &Path, e: &std::io::Error) -> TebakoError {
    err(EX_TEBAKO_IO, format!("{}: {e}", path.display()))
}

/// The store file's name grammar: the input file's stem, ASCII
/// alphanumerics plus `. _ -` — the name doubles as the store file's
/// name and the `remove` argument.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The outcome of `trust add`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustAddOutcome {
    /// The CA entered the store under this name.
    Added(String),
    /// The name already holds exactly these bytes; nothing changed.
    AlreadyPresent(String),
}

/// One `trust list` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaRow {
    /// The store name (the file stem).
    pub name: String,
    /// Certificate blocks in the PEM; None when the file no longer
    /// parses (hand-edited — the boot path refuses it; the report marks
    /// it).
    pub certs: Option<usize>,
    /// The file's sha256 (the audit handle).
    pub sha256: String,
}

/// Validate a PEM file at the store boundary: every certificate block
/// parses and at least one exists (the driver's merge rule, mirrored at
/// intake). Answers the byte count's certificate count.
fn validate_pem(path: &Path, bytes: &[u8]) -> Result<usize, TebakoError> {
    let mut count = 0usize;
    let mut cursor = std::io::Cursor::new(bytes);
    for item in rustls_pemfile::certs(&mut cursor) {
        item.map_err(|e| {
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "'{}' does not parse: {e} — expected PEM (-----BEGIN CERTIFICATE-----); nothing was stored",
                    path.display()
                ),
            )
        })?;
        count += 1;
    }
    if count == 0 {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "no certificate block parses from '{}' — expected PEM (-----BEGIN CERTIFICATE-----); nothing was stored",
                path.display()
            ),
        ));
    }
    Ok(count)
}

/// `tebako trust add <file.pem>`: validate, then install read-only via
/// tmp + rename under the store lock, journaled.
pub fn add(home: &Path, file: &Path) -> Result<TrustAddOutcome, TebakoError> {
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    if !valid_name(&stem) {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the CA store name derives from the file's stem and must match [A-Za-z0-9._-]+ — '{}' is not a usable name; rename the file and retry",
                file.display()
            ),
        ));
    }
    let bytes = std::fs::read(file).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot read '{}': {e}", file.display()),
        )
    })?;
    validate_pem(file, &bytes)?;

    let dir = tpkg::runtime_store::trust_ca_dir(home);
    std::fs::create_dir_all(&dir).map_err(|e| io_err(&dir, &e))?;
    let target = dir.join(format!("{stem}.pem"));
    let _lock = store_lock(&dir)?;
    if target.exists() {
        let existing = std::fs::read(&target).map_err(|e| io_err(&target, &e))?;
        if existing == bytes {
            return Ok(TrustAddOutcome::AlreadyPresent(stem));
        }
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the CA store already holds '{stem}' with different content — refusing to replace it silently; \
                 remove it first with `tebako trust remove {stem}`"
            ),
        ));
    }
    let tmp = dir.join(format!(".{stem}.part-{}", std::process::id()));
    std::fs::write(&tmp, &bytes).map_err(|e| io_err(&tmp, &e))?;
    std::fs::rename(&tmp, &target).map_err(|e| io_err(&target, &e))?;
    // Store artifacts are read-only; after the rename so the staging
    // write never races the attribute.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444));
    }
    journal(
        home,
        &format!(
            "event=trust-ca-added name={stem} sha256={}",
            tebako_resolve::sha256_hex(&bytes)
        ),
    );
    Ok(TrustAddOutcome::Added(stem))
}

/// `tebako trust list`: the store's rows, sorted by name. An absent
/// store lists empty. A corrupt entry is MARKED, not fatal (the report
/// is the operator's inspection surface; boot fails closed on the same
/// file).
pub fn list(home: &Path) -> Result<Vec<CaRow>, TebakoError> {
    let dir = tpkg::runtime_store::trust_ca_dir(home);
    let read = match std::fs::read_dir(&dir) {
        Ok(read) => read,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err(&dir, &e)),
    };
    let mut rows = Vec::new();
    for entry in read {
        let entry = entry.map_err(|e| io_err(&dir, &e))?;
        let path = entry.path();
        if !(path.is_file() && path.extension().is_some_and(|ext| ext == "pem")) {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let bytes = std::fs::read(&path).map_err(|e| io_err(&path, &e))?;
        let certs = rustls_pemfile::certs(&mut std::io::Cursor::new(&bytes))
            .collect::<Result<Vec<_>, _>>()
            .ok()
            .map(|v| v.len());
        rows.push(CaRow {
            name,
            certs,
            sha256: tebako_resolve::sha256_hex(&bytes),
        });
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

/// `tebako trust remove <name>`: delete the store file under the store
/// lock, journaled. An unknown name is the named nothing-to-remove.
pub fn remove(home: &Path, name: &str) -> Result<String, TebakoError> {
    if !valid_name(name) {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "'{name}' is not a CA store name ([A-Za-z0-9._-]+) — `tebako trust list` prints the stored names"
            ),
        ));
    }
    let dir = tpkg::runtime_store::trust_ca_dir(home);
    let target = dir.join(format!("{name}.pem"));
    let _lock = store_lock(&dir)?;
    if !target.exists() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the CA store holds no '{name}' — nothing to remove; `tebako trust list` prints the stored names"
            ),
        ));
    }
    // A read-only artifact needs the write bit back before removal on
    // platforms whose unlink honors it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644));
    }
    std::fs::remove_file(&target).map_err(|e| io_err(&target, &e))?;
    journal(home, &format!("event=trust-ca-removed name={name}"));
    Ok(name.to_string())
}

/// The signing-key pointer for `trust list` (the two stores answer
/// different questions — spec 09's signing keys vs the TLS CAs here):
/// true when the operator trusts at least one signing key beyond the
/// embedded root (a keyring entry or an add-registry pin).
pub fn signing_keys_present(home: &Path) -> bool {
    !tebako_signer::list_trusted(home)
        .unwrap_or_default()
        .is_empty()
}

/// The mutation window's per-store flock (the shim's per-entry install
/// lock shape: exclusive, non-blocking attempts on a poll to the 120 s
/// store-lock timeout; the kernel releases a crashed holder).
fn store_lock(dir: &Path) -> Result<StoreLock, TebakoError> {
    std::fs::create_dir_all(dir).map_err(|e| io_err(dir, &e))?;
    let path = dir.join(".lock");
    flock_acquire(&path, 120_000).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut {
            err(
                EX_TEBAKO_IO,
                format!(
                    "timed out waiting for the CA store lock '{}' (120 s) — a stale lock clears by removing the file when no tebako process holds it",
                    path.display()
                ),
            )
        } else {
            io_err(&path, &e)
        }
    })
}

struct StoreLock(std::fs::File);

impl Drop for StoreLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let fd = std::os::unix::io::AsRawFd::as_raw_fd(&self.0);
            unsafe {
                libc::flock(fd, libc::LOCK_UN);
            }
        }
        // windows: closing the handle (the File's own drop) releases the
        // LockFileEx byte range.
    }
}

const LOCK_POLL_MS: u64 = 200;

#[cfg(unix)]
fn flock_acquire(path: &Path, timeout_ms: u64) -> std::io::Result<StoreLock> {
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
            return Ok(StoreLock(f));
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EWOULDBLOCK)
            && err.kind() != std::io::ErrorKind::Interrupted
        {
            return Err(err);
        }
        if err.kind() != std::io::ErrorKind::Interrupted && std::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "lock timeout",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_POLL_MS));
    }
}

/// Windows: LockFileEx on one byte at offset 0 of the lock file — the
/// tebako-shim runtime.rs shape, the fourth copy by convention (the
/// bootstrap's platform.rs and tebako-resolve's cache.rs carry the
/// same).
#[cfg(windows)]
fn flock_acquire(path: &Path, timeout_ms: u64) -> std::io::Result<StoreLock> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::{ERROR_IO_PENDING, ERROR_LOCK_VIOLATION};
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;

    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        let mut ov = OVERLAPPED::default();
        let ok = unsafe {
            LockFileEx(
                f.as_raw_handle(),
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                1,
                0,
                &mut ov,
            )
        };
        if ok != 0 {
            return Ok(StoreLock(f));
        }
        let err = std::io::Error::last_os_error();
        let raw = err.raw_os_error().unwrap_or(0);
        if raw != ERROR_LOCK_VIOLATION as i32 && raw != ERROR_IO_PENDING as i32 {
            return Err(err);
        }
        if std::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "lock timeout",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_POLL_MS));
    }
}
