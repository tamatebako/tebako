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

    /// Cache hit or install: fetch (via `fetch`), verify against
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

    /// tmp + rename (a partial install is invisible), then the markers —
    /// the same order as tebako-cli's `place`.
    fn place(&self, file: &Path, fetched: &FetchedPayload) -> Result<(), ResolveError> {
        let tmp_dir = self.root.join(TMP_DIR);
        fs::create_dir_all(&tmp_dir).map_err(|e| cache_io("creating", &tmp_dir, e))?;
        let file_name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "payload".to_string());
        let tmp = tmp_dir.join(format!("{file_name}.{}.part", std::process::id()));
        let result = (|| {
            fs::write(&tmp, &fetched.bytes).map_err(|e| cache_io("writing", &tmp, e))?;
            let mut perms = fs::metadata(&tmp)
                .map_err(|e| cache_io("stat", &tmp, e))?
                .permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                perms.set_mode(0o444);
            }
            fs::set_permissions(&tmp, perms).map_err(|e| cache_io("chmod", &tmp, e))?;
            fs::rename(&tmp, file).map_err(|e| cache_io("installing", file, e))?;
            fs::write(
                sha_marker(file),
                format!("{}  {file_name}\n", fetched.sha256),
            )
            .map_err(|e| cache_io("marking", &sha_marker(file), e))?;
            fs::write(origin_marker(file), format!("{}\n", fetched.origin))
                .map_err(|e| cache_io("marking", &origin_marker(file), e))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
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
            if flock(&lock, libc::LOCK_EX | libc::LOCK_NB) {
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
        flock(&lock, libc::LOCK_UN);
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

/// flock(2) wrapper; returns true when the operation succeeded (the same
/// libc call tebako-cli's resolver uses — the only FFI in the crate).
fn flock(file: &fs::File, op: i32) -> bool {
    use std::os::unix::io::AsRawFd;
    // SAFETY: flock(2) on a live fd; identical to tebako-cli/src/resolve.rs.
    unsafe { libc::flock(file.as_raw_fd(), op) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test in this module touches the machine-cache env knobs
    /// (TEBAKO_OFFLINE is read on each install), so they serialize on one
    /// mutex — the default parallel test runner would otherwise race them.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        assert!(flock(&held, libc::LOCK_EX | libc::LOCK_NB));
        let err = cache
            .install("tool", "1.0", None, || Ok(fetched(b"x")))
            .unwrap_err();
        flock(&held, libc::LOCK_UN);
        assert!(matches!(err, ResolveError::LockTimeout { .. }));
        let msg = err.to_string();
        assert!(msg.contains("lockfile") && msg.contains("remove it if the holder crashed"));
        let _ = fs::remove_dir_all(&root);
    }
}
