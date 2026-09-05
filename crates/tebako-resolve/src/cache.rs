//! The shared payload cache (spec 05 §3–4), mirroring tebako-cli's
//! runtime-cache semantics exactly:
//!
//! ```text
//! <root>/payloads/<name>/<version>.tfs          registry payloads (0444)
//! <root>/payloads/<name>/<version>.tfs.sha256   trust anchor ("<sha>  <file>\n")
//! <root>/payloads/<name>/<version>.tfs.origin   source reference/URL
//! <root>/payloads/<name>/.install-<version>.lock   per-entry flock
//! <root>/tmp/<name>-<version>.<pid>.part        in-flight downloads
//! ```
//!
//! Installs are serialized per entry with a flock'd lockfile (120 s
//! timeout, stale-lock hint); a fetch lands in `tmp/` first, is
//! sha256-verified, and moves into place with an atomic rename — a
//! partial install is invisible. The `.sha256` marker IS the trust
//! anchor: present ⇒ the artifact was verified at install; never
//! re-verified per run. `TEBAKO_OFFLINE=1`: cache hit or hard error.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::Digest as _;

use crate::error::ResolveError;
use crate::fetch::FetchedPayload;

const TMP_DIR: &str = "tmp";
/// Spec 05 §4: 120 s with stale-lock hint.
const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// `$TEBAKO_HOME` or `~/.tebako` (`%LOCALAPPDATA%\tebako` on Windows) —
/// identical to tebako-cli's `default_cache_root`.
pub fn default_cache_root() -> PathBuf {
    if let Ok(home) = std::env::var("TEBAKO_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    if cfg!(windows) {
        if let Ok(lad) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(lad).join("tebako");
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".tebako")
}

/// TEBAKO_OFFLINE truthiness, identical to tebako-cli (`1|true|yes`).
pub fn offline() -> bool {
    std::env::var("TEBAKO_OFFLINE")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

/// Whether the install produced the entry or it was already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStatus {
    Hit,
    Installed,
}

/// The outcome of [`PayloadCache::seed`] (the lazy-seed verb, spec 23
/// §13.4): never an overwrite, always loud on divergence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedOutcome {
    /// The bytes verified against the pin and were placed (markers too).
    Seeded,
    /// The entry already existed with the SAME trust anchor — nothing to
    /// do (idempotent re-seed).
    AlreadySame,
    /// The entry already exists with a DIFFERENT trust anchor — reported,
    /// never overwritten (the cache holds registry bytes; the existing
    /// anchor stays authoritative).
    Conflict { existing_sha256: String },
}

/// A cached payload entry (artifact + trust anchor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub name: String,
    pub version: String,
    /// `<root>/payloads/<name>/<version>.tfs`
    pub path: PathBuf,
    /// Digest from the trusted `.sha256` marker (never re-computed per run).
    pub sha256: String,
    /// Source URL/reference from the `.origin` marker, when present.
    pub origin: Option<String>,
}

/// The prune protected set: exact `(name, version)` pairs prune never
/// removes (`PayloadCache::prune`).
pub type ProtectedSet = std::collections::BTreeSet<(String, String)>;

/// The `payloads/` tree of the shared machine cache.
pub struct PayloadCache {
    root: PathBuf,
    lock_timeout: std::time::Duration,
}

impl PayloadCache {
    /// Cache at the default machine root.
    pub fn new() -> Self {
        Self::with_root(default_cache_root())
    }

    /// Cache at an explicit root (tests, `TEBAKO_HOME` overrides).
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        PayloadCache {
            root: root.into(),
            lock_timeout: LOCK_TIMEOUT,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Test hook: shorten the flock wait.
    pub fn set_lock_timeout(&mut self, timeout: std::time::Duration) {
        self.lock_timeout = timeout;
    }

    fn validate_key(component: &str) -> Result<(), ResolveError> {
        let bad = component.is_empty()
            || component == "."
            || component == ".."
            || component
                .chars()
                .any(|c| c == '/' || c == '\\' || c.is_control());
        if bad {
            return Err(ResolveError::InvalidCacheKey {
                key: component.to_string(),
                reason: "cache names/versions must be non-empty and contain no path separators"
                    .to_string(),
            });
        }
        Ok(())
    }

    fn entry_file(&self, name: &str, version: &str) -> Result<PathBuf, ResolveError> {
        Self::validate_key(name)?;
        Self::validate_key(version)?;
        Ok(self
            .root
            .join("payloads")
            .join(name)
            .join(format!("{version}.tfs")))
    }

    fn lock_file(&self, name: &str, version: &str) -> PathBuf {
        self.root
            .join("payloads")
            .join(name)
            .join(format!(".install-{version}.lock"))
    }

    /// Cache lookup: a hit requires the artifact AND its trust anchor
    /// (mirroring tebako-cli's image-marker rule; a malformed anchor is a
    /// miss and triggers a re-install under the lock).
    pub fn get(&self, name: &str, version: &str) -> Result<Option<CacheEntry>, ResolveError> {
        let file = self.entry_file(name, version)?;
        let marker = sha_marker(&file);
        if !(file.is_file() && marker.is_file()) {
            return Ok(None);
        }
        let Ok(text) = fs::read_to_string(&marker) else {
            return Ok(None);
        };
        let Some(sha256) = text.split_whitespace().next() else {
            return Ok(None);
        };
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(None);
        }
        let origin = fs::read_to_string(origin_marker(&file))
            .ok()
            .map(|s| s.trim().to_string());
        Ok(Some(CacheEntry {
            name: name.to_string(),
            version: version.to_string(),
            path: file,
            sha256: sha256.to_string(),
            origin,
        }))
    }

    /// List the cached payloads (artifact + trust anchor present), sorted
    /// by name then version like tebako-cli's runtime `entries` —
    /// read-only, for the spec-15 `cache list --json` surface.
    pub fn list(&self) -> Vec<CacheEntry> {
        let base = self.root.join("payloads");
        let mut names: Vec<String> = Vec::new();
        if let Ok(children) = fs::read_dir(&base) {
            for child in children.flatten() {
                if child.path().is_dir() {
                    names.push(child.file_name().to_string_lossy().into_owned());
                }
            }
        }
        names.sort();
        let mut out = Vec::new();
        for name in names {
            let mut versions: Vec<String> = Vec::new();
            if let Ok(files) = fs::read_dir(base.join(&name)) {
                for file in files.flatten() {
                    let file_name = file.file_name().to_string_lossy().into_owned();
                    if let Some(version) = file_name.strip_suffix(".tfs") {
                        versions.push(version.to_string());
                    }
                }
            }
            versions.sort();
            for version in versions {
                if let Ok(Some(entry)) = self.get(&name, &version) {
                    out.push(entry);
                }
            }
        }
        out
    }

    /// Prune cached payload versions (the payload arm of `cache prune`,
    /// spec 15 §4): the same cutoff math as tebako-cli's runtime
    /// `Resolver::prune` — `all` drops everything, `older_than_days`
    /// drops entries whose artifact mtime is older than now − N days, and
    /// one of the two is required ([`ResolveError::PruneNeedsSelector`]).
    /// `(name, version)` pairs in `protected` are NEVER removed, even
    /// under `all` (the locked rule: prune never strands a pin — the
    /// caller builds the set from config defaults, disabled selectors,
    /// and the per-name newest floor). Removal deletes the version's
    /// whole record — `<v>.tfs`, the `.tfs.sha256`/`.tfs.origin` markers,
    /// the `<v>.manifest.yaml` mirror, a materialized `<v>.tree/`, the
    /// install lock — and the `<name>/` dir when it goes empty. Returns
    /// the removed pairs in `list()` order (name, then version).
    pub fn prune(
        &self,
        all: bool,
        older_than_days: Option<u64>,
        protected: &ProtectedSet,
    ) -> Result<Vec<(String, String)>, ResolveError> {
        if !all && older_than_days.is_none() {
            return Err(ResolveError::PruneNeedsSelector);
        }
        let cutoff = older_than_days
            .map(|d| std::time::SystemTime::now() - std::time::Duration::from_secs(d * 86_400));
        let mut removed = Vec::new();
        for entry in self.list() {
            let key = (entry.name.clone(), entry.version.clone());
            if protected.contains(&key) {
                continue;
            }
            let installed_at = fs::metadata(&entry.path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if !(all || cutoff.is_some_and(|c| installed_at < c)) {
                continue;
            }
            let dir = self.root.join("payloads").join(&entry.name);
            // The artifact is 0444/readonly (item 30b) — clear the bit
            // first so removal works where readonly blocks unlink.
            make_writable(&entry.path).map_err(|e| cache_io("chmod", &entry.path, e))?;
            for file in [
                entry.path.clone(),
                sha_marker(&entry.path),
                origin_marker(&entry.path),
                dir.join(format!("{}.manifest.yaml", entry.version)),
                dir.join(format!(".install-{}.lock", entry.version)),
            ] {
                match fs::remove_file(&file) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(cache_io("pruning", &file, e)),
                }
            }
            let tree = dir.join(format!("{}.tree", entry.version));
            if tree.is_dir() {
                fs::remove_dir_all(&tree).map_err(|e| cache_io("pruning", &tree, e))?;
            }
            // Only succeeds when nothing else remains under the name.
            let _ = fs::remove_dir(&dir);
            removed.push(key);
        }
        Ok(removed)
    }

    /// `expected_sha256` when given (registry-supplied trust anchor), and
    /// place atomically under the per-entry flock. A digest mismatch
    /// deletes the download and caches nothing (spec 04 §3).
    pub fn install<F>(
        &self,
        name: &str,
        version: &str,
        expected_sha256: Option<&str>,
        fetch: F,
    ) -> Result<(CacheEntry, InstallStatus), ResolveError>
    where
        F: FnOnce() -> Result<FetchedPayload, ResolveError>,
    {
        let file = self.entry_file(name, version)?;
        if let Some(entry) = self.get(name, version)? {
            return Ok((entry, InstallStatus::Hit));
        }
        // spec 18 C13 (defense in depth — the loader binaries check at
        // their entry points): a store stamped by a newer tebako refuses
        // every install; a pre-versioning store is stamped once.
        crate::store::check_once(&self.root).map_err(|e| ResolveError::CacheIo {
            op: "checking the store layout of",
            path: self.root.clone(),
            reason: e.to_string(),
        })?;
        let lock_path = self.lock_file(name, version);
        self.with_entry_lock(&lock_path, || {
            if let Some(entry) = self.get(name, version)? {
                return Ok((entry, InstallStatus::Hit));
            }
            if offline() {
                return Err(ResolveError::Offline {
                    what: format!("payload {name}@{version}"),
                });
            }
            let fetched = fetch()?;
            if let Some(expected) = expected_sha256 {
                let expected = expected.to_ascii_lowercase();
                if fetched.sha256 != expected {
                    return Err(ResolveError::Sha256Mismatch {
                        origin: fetched.origin,
                        expected,
                        actual: fetched.sha256,
                    });
                }
            }
            self.place(&file, &fetched)?;
            Ok((
                CacheEntry {
                    name: name.to_string(),
                    version: version.to_string(),
                    path: file,
                    sha256: fetched.sha256,
                    origin: Some(fetched.origin),
                },
                InstallStatus::Installed,
            ))
        })
    }

    /// The lazy-seed verb (spec 23 §13.4): place payload bytes the caller
    /// already holds (a carried slice read out of the running package)
    /// into the cache — sha256-verified against `expected_sha256` BEFORE
    /// anything lands, tmp+rename under the per-entry lock. NO offline
    /// gate: seeding moves local bytes, never the network. Never an
    /// overwrite: an existing entry with the same anchor is
    /// [`SeedOutcome::AlreadySame`] (idempotent), a different anchor is
    /// [`SeedOutcome::Conflict`] — the cached bytes win and the caller
    /// journals the divergence.
    pub fn seed(
        &self,
        name: &str,
        version: &str,
        expected_sha256: &str,
        origin: &str,
        mut reader: impl std::io::Read,
    ) -> Result<SeedOutcome, ResolveError> {
        let file = self.entry_file(name, version)?;
        let expected = expected_sha256.to_ascii_lowercase();
        if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ResolveError::InvalidCacheKey {
                key: format!("{name}@{version}"),
                reason: "the seed pin must be 64 hex".to_string(),
            });
        }
        if let Some(entry) = self.get(name, version)? {
            return Ok(if entry.sha256 == expected {
                SeedOutcome::AlreadySame
            } else {
                SeedOutcome::Conflict {
                    existing_sha256: entry.sha256,
                }
            });
        }
        // The same defense-in-depth store gate as install (spec 18 C13).
        crate::store::check_once(&self.root).map_err(|e| ResolveError::CacheIo {
            op: "checking the store layout of",
            path: self.root.clone(),
            reason: e.to_string(),
        })?;
        let lock_path = self.lock_file(name, version);
        let origin = origin.to_string();
        self.with_entry_lock(&lock_path, || {
            if let Some(entry) = self.get(name, version)? {
                return Ok(if entry.sha256 == expected {
                    SeedOutcome::AlreadySame
                } else {
                    SeedOutcome::Conflict {
                        existing_sha256: entry.sha256,
                    }
                });
            }
            let (tmp, file_name) = self.tmp_path(&file)?;
            let result = (|| {
                let mut hasher = sha2::Sha256::new();
                {
                    let mut out =
                        fs::File::create(&tmp).map_err(|e| cache_io("writing", &tmp, e))?;
                    let mut buf = [0u8; 65536];
                    loop {
                        let n = std::io::Read::read(&mut reader, &mut buf)
                            .map_err(|e| cache_io("streaming the seed bytes into", &tmp, e))?;
                        if n == 0 {
                            break;
                        }
                        sha2::Digest::update(&mut hasher, &buf[..n]);
                        std::io::Write::write_all(&mut out, &buf[..n])
                            .map_err(|e| cache_io("writing", &tmp, e))?;
                    }
                }
                let actual = crate::fetch::hex_digest(&sha2::Digest::finalize(hasher));
                if actual != expected {
                    return Err(ResolveError::Sha256Mismatch {
                        origin: origin.clone(),
                        expected,
                        actual,
                    });
                }
                self.finish_place(&tmp, &file, &file_name, &expected, &origin)
            })();
            if result.is_err() {
                let _ = fs::remove_file(&tmp);
            }
            result.map(|_| SeedOutcome::Seeded)
        })
    }

    /// tmp path + the entry's file name (the place/seed prelude).
    fn tmp_path(&self, file: &Path) -> Result<(PathBuf, String), ResolveError> {
        let tmp_dir = self.root.join(TMP_DIR);
        fs::create_dir_all(&tmp_dir).map_err(|e| cache_io("creating", &tmp_dir, e))?;
        let file_name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "payload".to_string());
        Ok((
            tmp_dir.join(format!("{file_name}.{}.part", std::process::id())),
            file_name,
        ))
    }

    /// tmp + rename (a partial install is invisible), then the markers —
    /// the same order as tebako-cli's `place`.
    fn place(&self, file: &Path, fetched: &FetchedPayload) -> Result<(), ResolveError> {
        let (tmp, file_name) = self.tmp_path(file)?;
        let result = (|| {
            fs::write(&tmp, &fetched.bytes).map_err(|e| cache_io("writing", &tmp, e))?;
            self.finish_place(&tmp, file, &file_name, &fetched.sha256, &fetched.origin)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    /// Readonly → rename → markers: the shared tail of place/seed.
    fn finish_place(
        &self,
        tmp: &Path,
        file: &Path,
        file_name: &str,
        sha256: &str,
        origin: &str,
    ) -> Result<(), ResolveError> {
        make_readonly(tmp).map_err(|e| cache_io("chmod", tmp, e))?;
        fs::rename(tmp, file).map_err(|e| cache_io("installing", file, e))?;
        fs::write(sha_marker(file), format!("{sha256}  {file_name}\n"))
            .map_err(|e| cache_io("marking", &sha_marker(file), e))?;
        fs::write(origin_marker(file), format!("{origin}\n"))
            .map_err(|e| cache_io("marking", &origin_marker(file), e))?;
        Ok(())
    }

    /// The per-entry flock (spec 05 §4): LOCK_EX|LOCK_NB retried for
    /// `lock_timeout`, then the named stale-lock-hint error.
    fn with_entry_lock<T>(
        &self,
        lock_path: &Path,
        f: impl FnOnce() -> Result<T, ResolveError>,
    ) -> Result<T, ResolveError> {
        let dir = lock_path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(dir).map_err(|e| cache_io("creating", dir, e))?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| cache_io("opening", lock_path, e))?;
        let deadline = std::time::Instant::now() + self.lock_timeout;
        loop {
            if flock_exclusive_nb(&lock) {
                break;
            }
            if std::time::Instant::now() >= deadline {
                return Err(ResolveError::LockTimeout {
                    lockfile: lock_path.to_path_buf(),
                    waited_secs: self.lock_timeout.as_secs(),
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let result = f();
        flock_unlock(&lock);
        result
    }
}

impl Default for PayloadCache {
    fn default() -> Self {
        Self::new()
    }
}

fn sha_marker(file: &Path) -> PathBuf {
    file.with_file_name(format!(
        "{}.sha256",
        file.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ))
}

fn origin_marker(file: &Path) -> PathBuf {
    file.with_file_name(format!(
        "{}.origin",
        file.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ))
}

fn cache_io(op: &'static str, path: &Path, e: std::io::Error) -> ResolveError {
    ResolveError::CacheIo {
        op,
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

/// The cached image is an immutable artifact (item 30b): 0444 on unix;
/// on Windows the FILE_ATTRIBUTE_READONLY bit with every other attribute
/// preserved (tebako-bootstrap's platform.rs shape). Best-effort errors
/// surface through the caller, never silently.
#[cfg(unix)]
fn make_readonly(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o444);
    std::fs::set_permissions(path, perms)
}

#[cfg(windows)]
fn make_readonly(path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_READONLY, INVALID_FILE_ATTRIBUTES,
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let attrs = GetFileAttributesW(wide.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(std::io::Error::last_os_error());
        }
        if SetFileAttributesW(wide.as_ptr(), attrs | FILE_ATTRIBUTE_READONLY) == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// The inverse of [`make_readonly`], for prune: 0644 on unix; on Windows
/// the readonly attribute cleared (DeleteFile refuses a readonly file).
#[cfg(unix)]
fn make_writable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(path, perms)
}

#[cfg(windows)]
fn make_writable(path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_READONLY, INVALID_FILE_ATTRIBUTES,
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let attrs = GetFileAttributesW(wide.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(std::io::Error::last_os_error());
        }
        if SetFileAttributesW(wide.as_ptr(), attrs & !FILE_ATTRIBUTE_READONLY) == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// The per-entry lock pair, one shape on each platform (no op-value
/// passing — named operations only):
///
/// Unix: flock(2) LOCK_EX|LOCK_NB / LOCK_UN on a live fd (the same libc
/// call tebako-cli's resolver uses). Windows: LockFileEx on one byte at
/// offset 0 — exclusive, non-blocking, released by the kernel when the
/// handle dies, exactly like flock (the shape tebako-bootstrap's
/// platform.rs uses).
#[cfg(unix)]
fn flock_exclusive_nb(file: &fs::File) -> bool {
    use std::os::unix::io::AsRawFd;
    // SAFETY: flock(2) on a live fd; identical to tebako-cli/src/resolve.rs.
    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 }
}

#[cfg(unix)]
fn flock_unlock(file: &fs::File) -> bool {
    use std::os::unix::io::AsRawFd;
    // SAFETY: flock(2) on a live fd.
    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) == 0 }
}

#[cfg(windows)]
fn flock_exclusive_nb(file: &fs::File) -> bool {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;
    let mut ov = OVERLAPPED::default();
    unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut ov,
        ) != 0
    }
}

#[cfg(windows)]
fn flock_unlock(file: &fs::File) -> bool {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::UnlockFileEx;
    use windows_sys::Win32::System::IO::OVERLAPPED;
    let mut ov = OVERLAPPED::default();
    unsafe { UnlockFileEx(file.as_raw_handle(), 0, 1, 0, &mut ov) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test in this module touches the machine-cache env knobs
    /// (TEBAKO_OFFLINE is read on each install), so they serialize on the
    /// crate-wide test-env mutex — the default parallel test runner would
    /// otherwise race them.
    use crate::TEST_ENV_LOCK as ENV_LOCK;

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tebako-resolve-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn fetched(bytes: &[u8]) -> FetchedPayload {
        FetchedPayload {
            bytes: bytes.to_vec(),
            origin: "https://cdn.example.com/tool.tfs".to_string(),
            sha256: crate::fetch::sha256_hex(bytes),
        }
    }

    /// Backdate a file's mtime (prune-age fixtures). The same libc/Win32
    /// pair this module's flock uses — no new dependency.
    #[cfg(unix)]
    fn set_age_days(path: &Path, days: u64) {
        use std::os::unix::ffi::OsStrExt as _;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let at = libc::timespec {
            tv_sec: now - (days * 86_400) as i64,
            tv_nsec: 0,
        };
        let times = [at, at];
        let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: a valid NUL-terminated path and a two-element times array.
        let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), times.as_ptr(), 0) };
        assert_eq!(rc, 0, "utimensat failed for {}", path.display());
    }

    /// Backdate a file's mtime (prune-age fixtures) — the Win32 arm.
    #[cfg(windows)]
    fn set_age_days(path: &Path, days: u64) {
        use std::os::windows::ffi::OsStrExt as _;
        use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, SetFileTime, FILE_ATTRIBUTE_NORMAL, FILE_WRITE_ATTRIBUTES, OPEN_EXISTING,
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let then = now - days * 86_400 + 11_644_473_600;
        let ticks = (then as u128 * 10_000_000) as u64;
        let ft = FILETIME {
            dwLowDateTime: ticks as u32,
            dwHighDateTime: (ticks >> 32) as u32,
        };
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: a valid wide path; the handle is checked and closed.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_WRITE_ATTRIBUTES,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(handle, INVALID_HANDLE_VALUE);
        // SAFETY: a live handle and a valid FILETIME.
        let rc = unsafe { SetFileTime(handle, std::ptr::null(), std::ptr::null(), &ft) };
        // SAFETY: a live handle.
        unsafe { CloseHandle(handle) };
        assert_ne!(rc, 0, "SetFileTime failed for {}", path.display());
    }

    fn install_aged(
        cache: &PayloadCache,
        root: &Path,
        name: &str,
        version: &str,
        bytes: &[u8],
        days: u64,
    ) {
        cache
            .install(name, version, None, || Ok(fetched(bytes)))
            .unwrap();
        set_age_days(
            &root
                .join("payloads")
                .join(name)
                .join(format!("{version}.tfs")),
            days,
        );
    }

    #[test]
    fn install_then_hit_with_markers() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        let (entry, status) = cache
            .install("tool", "1.0", None, || Ok(fetched(b"payload")))
            .unwrap();
        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(entry.path, root.join("payloads/tool/1.0.tfs"));
        assert_eq!(entry.sha256, crate::fetch::sha256_hex(b"payload"));

        // artifact is 0444, markers mirror the runtime-cache formats
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&entry.path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o444);
        }
        let sha = fs::read_to_string(root.join("payloads/tool/1.0.tfs.sha256")).unwrap();
        assert_eq!(sha, format!("{}  1.0.tfs\n", entry.sha256));
        let origin = fs::read_to_string(root.join("payloads/tool/1.0.tfs.origin")).unwrap();
        assert_eq!(origin, "https://cdn.example.com/tool.tfs\n");

        // second install: hit, fetch closure never runs
        let (entry2, status2) = cache
            .install("tool", "1.0", None, || panic!("must not fetch on a hit"))
            .unwrap();
        assert_eq!(status2, InstallStatus::Hit);
        assert_eq!(entry2.sha256, entry.sha256);
        assert_eq!(
            entry2.origin.as_deref(),
            Some("https://cdn.example.com/tool.tfs")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn digest_mismatch_caches_nothing() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        let err = cache
            .install("tool", "1.0", Some(&"f".repeat(64)), || {
                Ok(fetched(b"payload"))
            })
            .unwrap_err();
        assert!(matches!(err, ResolveError::Sha256Mismatch { .. }));
        assert!(!root.join("payloads/tool/1.0.tfs").exists());
        assert!(!root.join("payloads/tool/1.0.tfs.sha256").exists());
        // tmp is cleaned up too
        assert_eq!(
            fs::read_dir(root.join("tmp"))
                .map(|d| d.count())
                .unwrap_or(0),
            0
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn offline_is_cache_hit_or_hard_error() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        std::env::set_var("TEBAKO_OFFLINE", "1");
        let err = cache
            .install("tool", "1.0", None, || Ok(fetched(b"payload")))
            .unwrap_err();
        assert!(matches!(err, ResolveError::Offline { .. }));
        assert!(err.to_string().contains("tool@1.0"));

        cache
            .install("tool", "2.0", None, || Ok(fetched(b"payload")))
            .unwrap_err();
        // install with offline unset, then re-hit with offline set
        std::env::remove_var("TEBAKO_OFFLINE");
        cache
            .install("tool", "2.0", None, || Ok(fetched(b"payload")))
            .unwrap();
        std::env::set_var("TEBAKO_OFFLINE", "yes");
        let (_, status) = cache
            .install("tool", "2.0", None, || panic!("offline hit must not fetch"))
            .unwrap();
        assert_eq!(status, InstallStatus::Hit);
        std::env::remove_var("TEBAKO_OFFLINE");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cache_keys_are_path_safe() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        for (name, version) in [("", "1.0"), ("..", "1.0"), ("a/b", "1.0"), ("tool", "1/0")] {
            assert!(matches!(
                cache.install(name, version, None, || Ok(fetched(b"x"))),
                Err(ResolveError::InvalidCacheKey { .. })
            ));
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn list_returns_entries_with_trust_anchors() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        cache
            .install("tool", "1.0", None, || Ok(fetched(b"one")))
            .unwrap();
        cache
            .install("tool", "2.0", None, || Ok(fetched(b"two")))
            .unwrap();
        cache
            .install("other", "0.1", None, || Ok(fetched(b"three")))
            .unwrap();
        // An anchor-less artifact is not a cache entry (never listed).
        fs::create_dir_all(root.join("payloads/stray")).unwrap();
        fs::write(root.join("payloads/stray/9.9.tfs"), b"stray").unwrap();

        let entries = cache.list();
        let keys: Vec<(&str, &str)> = entries
            .iter()
            .map(|e| (e.name.as_str(), e.version.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![("other", "0.1"), ("tool", "1.0"), ("tool", "2.0")]
        );
        assert_eq!(entries[1].sha256, crate::fetch::sha256_hex(b"one"));
        assert_eq!(
            entries[1].origin.as_deref(),
            Some("https://cdn.example.com/tool.tfs")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lock_timeout_is_a_named_error_with_stale_hint() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let mut cache = PayloadCache::with_root(&root);
        cache.set_lock_timeout(std::time::Duration::from_millis(300));
        // Hold the entry lock from this process on a second fd.
        let lock_path = root.join("payloads/tool/.install-1.0.lock");
        fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .unwrap();
        assert!(flock_exclusive_nb(&held));
        let err = cache
            .install("tool", "1.0", None, || Ok(fetched(b"x")))
            .unwrap_err();
        flock_unlock(&held);
        assert!(matches!(err, ResolveError::LockTimeout { .. }));
        let msg = err.to_string();
        assert!(msg.contains("lockfile") && msg.contains("remove it if the holder crashed"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn seed_places_verified_bytes_and_reseeds_idempotently() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        let pin = crate::fetch::sha256_hex(b"carried-slice");
        let origin = "seeded-from:/tmp/pkg.tpkg#slot2";
        let outcome = cache
            .seed("tool", "1.0", &pin, origin, b"carried-slice".as_slice())
            .unwrap();
        assert_eq!(outcome, SeedOutcome::Seeded);
        // the seeded entry is a full cache citizen: 0444 + both markers
        let entry = cache.get("tool", "1.0").unwrap().unwrap();
        assert_eq!(entry.sha256, pin);
        assert_eq!(entry.origin.as_deref(), Some(origin));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&entry.path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o444);
        }
        // re-seed with the same pin: idempotent
        let outcome = cache
            .seed("tool", "1.0", &pin, origin, b"carried-slice".as_slice())
            .unwrap();
        assert_eq!(outcome, SeedOutcome::AlreadySame);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn seed_conflict_never_overwrites() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        cache
            .install("tool", "1.0", None, || Ok(fetched(b"registry-bytes")))
            .unwrap();
        let other = crate::fetch::sha256_hex(b"carried-slice");
        let outcome = cache
            .seed("tool", "1.0", &other, "seeded", b"carried-slice".as_slice())
            .unwrap();
        let SeedOutcome::Conflict { existing_sha256 } = outcome else {
            panic!("expected Conflict, got {outcome:?}")
        };
        assert_eq!(existing_sha256, crate::fetch::sha256_hex(b"registry-bytes"));
        // the cached bytes are the registry's, untouched
        assert_eq!(
            fs::read(root.join("payloads/tool/1.0.tfs")).unwrap(),
            b"registry-bytes"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn seed_verifies_before_place_and_caches_nothing_on_mismatch() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        let err = cache
            .seed(
                "tool",
                "1.0",
                &"f".repeat(64),
                "seeded",
                b"carried-slice".as_slice(),
            )
            .unwrap_err();
        assert!(matches!(err, ResolveError::Sha256Mismatch { .. }));
        assert!(!root.join("payloads/tool/1.0.tfs").exists());
        assert!(!root.join("payloads/tool/1.0.tfs.sha256").exists());
        // tmp is cleaned up too
        assert_eq!(
            fs::read_dir(root.join("tmp"))
                .map(|d| d.count())
                .unwrap_or(0),
            0
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn seed_has_no_offline_gate() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        std::env::set_var("TEBAKO_OFFLINE", "1");
        let pin = crate::fetch::sha256_hex(b"carried-slice");
        let outcome = cache
            .seed("tool", "1.0", &pin, "seeded", b"carried-slice".as_slice())
            .unwrap();
        std::env::remove_var("TEBAKO_OFFLINE");
        assert_eq!(outcome, SeedOutcome::Seeded);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prune_requires_a_selector() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        let protected = std::collections::BTreeSet::new();
        let err = cache.prune(false, None, &protected).unwrap_err();
        assert!(matches!(err, ResolveError::PruneNeedsSelector));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prune_older_than_removes_only_old_unprotected_versions() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        install_aged(&cache, &root, "tool", "1.0", b"one", 40);
        install_aged(&cache, &root, "tool", "2.0", b"two", 40);
        install_aged(&cache, &root, "tool", "3.0", b"three", 1);
        install_aged(&cache, &root, "other", "0.1", b"other", 40);
        let protected: std::collections::BTreeSet<(String, String)> =
            [("tool".to_string(), "2.0".to_string())]
                .into_iter()
                .collect();
        let removed = cache.prune(false, Some(30), &protected).unwrap();
        assert_eq!(
            removed,
            vec![
                ("other".to_string(), "0.1".to_string()),
                ("tool".to_string(), "1.0".to_string())
            ]
        );
        // the protected and the fresh versions keep their full records
        for v in ["2.0", "3.0"] {
            assert!(root.join(format!("payloads/tool/{v}.tfs")).is_file());
            assert!(root.join(format!("payloads/tool/{v}.tfs.sha256")).is_file());
        }
        assert!(!root.join("payloads/tool/1.0.tfs").exists());
        assert!(!root.join("payloads/tool/1.0.tfs.sha256").exists());
        assert!(!root.join("payloads/tool/1.0.tfs.origin").exists());
        // a fully pruned name loses its directory
        assert!(!root.join("payloads/other").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prune_all_respects_protected_and_removes_the_whole_record() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        install_aged(&cache, &root, "tool", "1.0", b"one", 0);
        install_aged(&cache, &root, "tool", "2.0", b"two", 0);
        // the record is more than the artifact: manifest mirror,
        // materialized zero-runtime tree, install lock
        fs::create_dir_all(root.join("payloads/tool/1.0.tree/local")).unwrap();
        fs::write(root.join("payloads/tool/1.0.tree/local/x.rb"), b"x").unwrap();
        fs::write(root.join("payloads/tool/1.0.manifest.yaml"), b"manifest").unwrap();
        fs::write(root.join("payloads/tool/2.0.manifest.yaml"), b"manifest").unwrap();
        let protected: std::collections::BTreeSet<(String, String)> =
            [("tool".to_string(), "2.0".to_string())]
                .into_iter()
                .collect();
        let removed = cache.prune(true, None, &protected).unwrap();
        assert_eq!(removed, vec![("tool".to_string(), "1.0".to_string())]);
        assert!(!root.join("payloads/tool/1.0.tfs").exists());
        assert!(!root.join("payloads/tool/1.0.tree").exists());
        assert!(!root.join("payloads/tool/1.0.manifest.yaml").exists());
        assert!(!root.join("payloads/tool/.install-1.0.lock").exists());
        assert!(root.join("payloads/tool/2.0.tfs").is_file());
        assert!(root.join("payloads/tool/2.0.manifest.yaml").is_file());
        let _ = fs::remove_dir_all(&root);
    }
}
