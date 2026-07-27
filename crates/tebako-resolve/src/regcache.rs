//! The dispatch-time registry cache (spec 04 §3 fetch discipline, spec 07
//! §2.1's registry-default chain link): every registry form of spec 04 §2
//! — default-branch contents API, pinned release artifact, git blob,
//! `file://` mirror — resolves through this cache at dispatch time:
//!
//! ```text
//! <root>/registries/<sha256-of-canonical-ref>.yaml   the registry file
//! <root>/registries/<sha256-of-canonical-ref>.meta   fetched-at + the ref
//! <root>/registries/<sha256-of-canonical-ref>.lock   per-entry flock
//! ```
//!
//! Fresh entries (age < [`REGISTRY_CACHE_TTL`], 24 h) are used without
//! touching the source. Stale entries are re-fetched — falling back to the
//! stale copy when the source is UNREACHABLE (the cache exists so dispatch
//! survives registry downtime), but never when the source serves an
//! unparseable registry (a broken publication is the named registry error,
//! not a silent fallback). [`RegistryCache::refresh`] (the
//! `tebako update-registries` command) forces the re-fetch regardless of
//! freshness. Offline (`TEBAKO_OFFLINE`, caller-supplied): a cache hit —
//! fresh or stale — or the named [`ResolveError::Offline`] (spec 04 §3:
//! cache-or-named-error).
//!
//! The cached yaml is written only AFTER the fetched bytes parse as a
//! valid [`Registry`] (and after any `?sha256=` pin verified inside the
//! fetch) — a cached entry is trusted, never re-validated per read (the
//! spec 05 §4 trust-anchor discipline).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{RegistryError, ResolveError};
use crate::fetch::Fetcher;
use crate::registry::{Registry, RegistryRef};
use crate::transport::Transport;

/// The registry cache TTL: 24 h (spec 07 §0's dispatch-time cache).
pub const REGISTRY_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Spec 05 §4: 120 s with stale-lock hint (the payload cache's number).
const LOCK_TIMEOUT: Duration = Duration::from_secs(120);

/// Where a resolved registry came from (reported by the shim's `which`
/// and by `update-registries`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrySource {
    /// Fetched from the source just now (and written into the cache).
    Fetched,
    /// Served from a fresh cache entry (age < TTL); the source was not
    /// touched.
    CacheFresh,
    /// Served from a stale cache entry: the source was unreachable (or
    /// offline) and the stale copy kept dispatch alive.
    CacheStale,
}

/// A resolved registry plus its provenance.
#[derive(Debug)]
pub struct CachedRegistry {
    pub registry: Registry,
    pub source: RegistrySource,
    /// When the entry was fetched from the source.
    pub fetched_at: SystemTime,
    /// The cache file the entry was read from / written to.
    pub cache_path: PathBuf,
}

/// The freshness report for one cached registry (`doctor`); reads the
/// cache only, never the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryCacheStatus {
    /// No cache entry.
    Missing,
    /// Cache entry younger than the TTL.
    Fresh { age: Duration },
    /// Cache entry older than the TTL (dispatch re-fetches on next use).
    Stale { age: Duration },
    /// A cache entry exists but does not read/parse — re-fetch repairs it.
    Corrupt { reason: String },
}

/// The `registries/` tree of the machine cache. `offline` is
/// caller-supplied (the shim resolves `TEBAKO_OFFLINE` from its injected
/// context; the CLI from the process env) — this module never reads the
/// process environment.
pub struct RegistryCache {
    root: PathBuf,
    ttl: Duration,
    offline: bool,
    lock_timeout: Duration,
}

impl RegistryCache {
    /// The cache under `root` (`~/.tebako`) with the 24 h TTL.
    pub fn new(root: impl Into<PathBuf>, offline: bool) -> Self {
        Self::with_ttl(root, REGISTRY_CACHE_TTL, offline)
    }

    /// The cache with an explicit TTL (tests).
    pub fn with_ttl(root: impl Into<PathBuf>, ttl: Duration, offline: bool) -> Self {
        RegistryCache {
            root: root.into(),
            ttl,
            offline,
            lock_timeout: LOCK_TIMEOUT,
        }
    }

    /// Test hook: shorten the flock wait.
    pub fn set_lock_timeout(&mut self, timeout: Duration) {
        self.lock_timeout = timeout;
    }

    fn paths(&self, r: &RegistryRef) -> (PathBuf, PathBuf, PathBuf) {
        let key = crate::fetch::sha256_hex(r.as_canonical_string().as_bytes());
        let dir = self.root.join("registries");
        (
            dir.join(format!("{key}.yaml")),
            dir.join(format!("{key}.meta")),
            dir.join(format!("{key}.lock")),
        )
    }

    /// Resolve `r`: fresh cache hit, else re-fetch under the per-entry
    /// lock with the stale/offline rules of the module docs.
    pub fn resolve<T: Transport>(
        &self,
        r: &RegistryRef,
        fetcher: &Fetcher<T>,
    ) -> Result<CachedRegistry, ResolveError> {
        let (yaml, meta, lock) = self.paths(r);
        if let Some(cached) = self.read_fresh(&yaml, &meta) {
            return Ok(cached);
        }
        self.with_lock(&lock, |cache| {
            // Re-check under the lock: another process may have refreshed.
            if let Some(cached) = cache.read_fresh(&yaml, &meta) {
                return Ok(cached);
            }
            let stale = cache.read_entry(&yaml, &meta).ok();
            if r.is_remote() && cache.offline {
                return match stale {
                    Some(mut c) => {
                        c.source = RegistrySource::CacheStale;
                        Ok(c)
                    }
                    None => Err(ResolveError::Offline {
                        what: format!("registry {r}"),
                    }),
                };
            }
            match fetcher.fetch_registry_gated(r, cache.offline) {
                Ok(bytes) => {
                    let registry = parse_registry(&bytes)?;
                    let fetched_at = SystemTime::now();
                    cache.write_entry(&yaml, &meta, r, &bytes, fetched_at)?;
                    Ok(CachedRegistry {
                        registry,
                        source: RegistrySource::Fetched,
                        fetched_at,
                        cache_path: yaml.clone(),
                    })
                }
                // Source unreachable: the stale copy keeps dispatch alive.
                // (An UNPARSEABLE source above is the named registry error
                // instead — never masked by the cache.)
                Err(e) => match stale {
                    Some(mut c) => {
                        c.source = RegistrySource::CacheStale;
                        Ok(c)
                    }
                    None => Err(e),
                },
            }
        })
    }

    /// Force the re-fetch regardless of freshness (the
    /// `tebako update-registries` refresh). Offline + remote is the named
    /// [`ResolveError::Offline`]; a stale cache never substitutes for an
    /// explicit refresh.
    pub fn refresh<T: Transport>(
        &self,
        r: &RegistryRef,
        fetcher: &Fetcher<T>,
    ) -> Result<CachedRegistry, ResolveError> {
        let (yaml, meta, lock) = self.paths(r);
        self.with_lock(&lock, |cache| {
            if r.is_remote() && cache.offline {
                return Err(ResolveError::Offline {
                    what: format!("registry {r}"),
                });
            }
            let bytes = fetcher.fetch_registry_gated(r, cache.offline)?;
            let registry = parse_registry(&bytes)?;
            let fetched_at = SystemTime::now();
            cache.write_entry(&yaml, &meta, r, &bytes, fetched_at)?;
            Ok(CachedRegistry {
                registry,
                source: RegistrySource::Fetched,
                fetched_at,
                cache_path: yaml.clone(),
            })
        })
    }

    /// The freshness of the cached copy of `r` (doctor; never touches the
    /// source).
    pub fn status(&self, r: &RegistryRef) -> RegistryCacheStatus {
        let (yaml, meta, _) = self.paths(r);
        if !yaml.is_file() {
            return RegistryCacheStatus::Missing;
        }
        match self.read_entry(&yaml, &meta) {
            Ok(c) => {
                let age = SystemTime::now()
                    .duration_since(c.fetched_at)
                    .unwrap_or(Duration::ZERO);
                if age < self.ttl {
                    RegistryCacheStatus::Fresh { age }
                } else {
                    RegistryCacheStatus::Stale { age }
                }
            }
            Err(e) => RegistryCacheStatus::Corrupt {
                reason: e.to_string(),
            },
        }
    }

    /// A fresh cache entry, if one exists.
    fn read_fresh(&self, yaml: &Path, meta: &Path) -> Option<CachedRegistry> {
        let cached = self.read_entry(yaml, meta).ok()?;
        let age = SystemTime::now()
            .duration_since(cached.fetched_at)
            .unwrap_or(Duration::ZERO);
        if age < self.ttl {
            Some(CachedRegistry {
                source: RegistrySource::CacheFresh,
                ..cached
            })
        } else {
            None
        }
    }

    /// Read a cache entry regardless of freshness.
    fn read_entry(&self, yaml: &Path, meta: &Path) -> Result<CachedRegistry, ResolveError> {
        let text = fs::read_to_string(yaml).map_err(|e| cache_io("reading", yaml, e))?;
        let registry = parse_registry(text.as_bytes())?;
        let fetched_at = read_meta(meta)?;
        Ok(CachedRegistry {
            registry,
            source: RegistrySource::CacheStale, // the caller re-marks
            fetched_at,
            cache_path: yaml.to_path_buf(),
        })
    }

    /// tmp + rename for both files (a partial refresh is invisible).
    fn write_entry(
        &self,
        yaml: &Path,
        meta: &Path,
        r: &RegistryRef,
        bytes: &[u8],
        fetched_at: SystemTime,
    ) -> Result<(), ResolveError> {
        let dir = yaml.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(dir).map_err(|e| cache_io("creating", dir, e))?;
        let epoch = fetched_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let meta_text = format!("fetched-at: {epoch}\nref: {}\n", r.as_canonical_string());
        for (path, contents) in [(yaml, bytes), (meta, meta_text.as_bytes())] {
            let tmp = dir.join(format!(
                ".{}.{}.tmp",
                path.file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default(),
                std::process::id()
            ));
            fs::write(&tmp, contents).map_err(|e| cache_io("writing", &tmp, e))?;
            fs::rename(&tmp, path).map_err(|e| cache_io("installing", path, e))?;
        }
        Ok(())
    }

    /// The per-entry flock (spec 05 §4 discipline): LOCK_EX|LOCK_NB
    /// retried for `lock_timeout`, then the named stale-lock-hint error.
    fn with_lock<T>(
        &self,
        lock_path: &Path,
        f: impl FnOnce(&RegistryCache) -> Result<T, ResolveError>,
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
            if crate::cache::flock(&lock, libc::LOCK_EX | libc::LOCK_NB) {
                break;
            }
            if std::time::Instant::now() >= deadline {
                return Err(ResolveError::LockTimeout {
                    lockfile: lock_path.to_path_buf(),
                    waited_secs: self.lock_timeout.as_secs(),
                });
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let result = f(self);
        crate::cache::flock(&lock, libc::LOCK_UN);
        result
    }
}

/// Parse registry bytes (UTF-8 + the model's two-step parse/validate) —
/// the same named errors as `Fetcher::resolve_registry`.
fn parse_registry(bytes: &[u8]) -> Result<Registry, ResolveError> {
    let text = String::from_utf8(bytes.to_vec()).map_err(|e| RegistryError::Yaml {
        reason: format!("{e} decoding the registry file"),
    })?;
    Ok(Registry::from_yaml(&text)?)
}

/// The `.meta` sidecar: `fetched-at: <epoch secs>` + `ref: <canonical>`.
fn read_meta(meta: &Path) -> Result<SystemTime, ResolveError> {
    let text = fs::read_to_string(meta).map_err(|e| cache_io("reading", meta, e))?;
    let epoch = text
        .lines()
        .find_map(|l| l.strip_prefix("fetched-at: "))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .ok_or_else(|| ResolveError::CacheIo {
            op: "parsing",
            path: meta.to_path_buf(),
            reason: "no fetched-at line".to_string(),
        })?;
    Ok(UNIX_EPOCH + Duration::from_secs(epoch))
}

fn cache_io(op: &'static str, path: &Path, e: std::io::Error) -> ResolveError {
    ResolveError::CacheIo {
        op,
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tebako_http::FetchError;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The fetch.rs mock, duplicated: one canned answer per URL.
    pub struct MockTransport {
        answers: std::collections::HashMap<String, Vec<u8>>,
    }
    impl MockTransport {
        fn empty() -> Self {
            MockTransport {
                answers: std::collections::HashMap::new(),
            }
        }
        fn answer(url: &str, body: &str) -> Self {
            let mut answers = std::collections::HashMap::new();
            answers.insert(url.to_string(), body.as_bytes().to_vec());
            MockTransport { answers }
        }
    }
    impl Transport for MockTransport {
        fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            self.answers
                .get(url)
                .cloned()
                .ok_or_else(|| FetchError::IndexUnavailable(url.to_string()))
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-resolve-regcache-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const REGISTRY_YAML_V1: &str = r#"
schema_version: 1
payloads:
  - name: tool
    kind: app
    versions:
      - version: 1.0
        platforms: universal
        release: {ref: file:///m/tool-1.0.tfs}
        entrypoints: [tool]
    default: 1.0
"#;

    const REGISTRY_YAML_V2: &str = r#"
schema_version: 1
payloads:
  - name: tool
    kind: app
    versions:
      - version: 2.0
        platforms: universal
        release: {ref: file:///m/tool-2.0.tfs}
        entrypoints: [tool]
    default: 2.0
"#;

    fn file_ref(root: &Path) -> (RegistryRef, PathBuf) {
        let file = root.join("tpkg-registry.yaml");
        fs::write(&file, REGISTRY_YAML_V1).unwrap();
        let r = RegistryRef::parse(&format!("file://{}", file.display())).unwrap();
        (r, file)
    }

    fn default_of(c: &CachedRegistry) -> String {
        c.registry
            .payload("tool")
            .and_then(|p| p.default.clone())
            .unwrap()
    }

    #[test]
    fn file_mirror_resolves_then_serves_fresh_from_the_cache() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch("file");
        let (r, file) = file_ref(&root);
        let fetcher = Fetcher::new();
        let cache = RegistryCache::new(&root, false);

        let first = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(first.source, RegistrySource::Fetched);
        assert_eq!(default_of(&first), "1.0");

        // the source is gone: the fresh cache still resolves
        fs::remove_file(&file).unwrap();
        let second = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(second.source, RegistrySource::CacheFresh);
        assert_eq!(default_of(&second), "1.0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_entries_refetch_and_unreachable_sources_fall_back_to_stale() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch("ttl");
        let (r, file) = file_ref(&root);
        let fetcher = Fetcher::new();
        // TTL 0: every entry is immediately stale.
        let cache = RegistryCache::with_ttl(&root, Duration::ZERO, false);

        let first = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(first.source, RegistrySource::Fetched);

        // the source changed: stale → re-fetch picks it up
        fs::write(&file, REGISTRY_YAML_V2).unwrap();
        let second = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(second.source, RegistrySource::Fetched);
        assert_eq!(default_of(&second), "2.0");

        // the source is unreachable: the stale copy keeps dispatch alive
        fs::remove_file(&file).unwrap();
        let third = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(third.source, RegistrySource::CacheStale);
        assert_eq!(default_of(&third), "2.0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unparseable_source_is_the_named_error_never_a_stale_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch("broken");
        let (r, file) = file_ref(&root);
        let fetcher = Fetcher::new();
        let cache = RegistryCache::with_ttl(&root, Duration::ZERO, false);
        cache.resolve(&r, &fetcher).unwrap();

        fs::write(&file, "schema_version: 2\npayloads: []\n").unwrap();
        let err = cache.resolve(&r, &fetcher).unwrap_err();
        assert!(matches!(err, ResolveError::Registry(_)), "{err:?}");
        assert!(err.to_string().contains("schema_version 2"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_ignores_freshness() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch("refresh");
        let (r, file) = file_ref(&root);
        let fetcher = Fetcher::new();
        let cache = RegistryCache::new(&root, false);

        cache.resolve(&r, &fetcher).unwrap();
        // fresh cache, source changed: plain resolve would NOT see it…
        fs::write(&file, REGISTRY_YAML_V2).unwrap();
        let cached = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(default_of(&cached), "1.0");
        // …refresh does (the update-registries command)
        let refreshed = cache.refresh(&r, &fetcher).unwrap();
        assert_eq!(default_of(&refreshed), "2.0");
        let _ = fs::remove_dir_all(&root);
    }

    /// A REMOTE registry resolved without the network: the default-branch
    /// form behind a mock transport (gitlab's single-GET contents API).
    fn remote_ref() -> (RegistryRef, Fetcher<MockTransport>) {
        let url =
            "https://gitlab.com/api/v4/projects/o%2Fr/repository/files/tpkg-registry.yaml/raw";
        let fetcher = Fetcher::with_transport(MockTransport::answer(url, REGISTRY_YAML_V1));
        let r = RegistryRef::parse("tfs:gitlab:o/r").unwrap();
        assert!(r.is_remote());
        (r, fetcher)
    }

    #[test]
    fn remote_registry_fetches_through_the_cache() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch("remote");
        let (r, fetcher) = remote_ref();
        let cache = RegistryCache::new(&root, false);
        let got = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(got.source, RegistrySource::Fetched);
        assert_eq!(default_of(&got), "1.0");

        // second resolve is a cache hit — the mock answers nothing more
        let fetcher = Fetcher::with_transport(MockTransport::empty());
        let again = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(again.source, RegistrySource::CacheFresh);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn offline_is_cache_hit_or_the_named_error() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch("offline");
        let (r, _) = remote_ref();
        let cache = RegistryCache::new(&root, true);
        let fetcher = Fetcher::with_transport(MockTransport::empty());

        // no cache → the named offline error
        let err = cache.resolve(&r, &fetcher).unwrap_err();
        assert!(matches!(err, ResolveError::Offline { .. }), "{err:?}");
        assert!(err.to_string().contains("registry tfs:gitlab:o/r"), "{err}");

        // refresh is equally refused (never a stale substitute)
        let err = cache.refresh(&r, &fetcher).unwrap_err();
        assert!(matches!(err, ResolveError::Offline { .. }));

        // a populated cache resolves offline (stale included)
        let online = RegistryCache::new(&root, false);
        let (r2, fetcher2) = remote_ref();
        online.resolve(&r2, &fetcher2).unwrap();
        let hit = cache.resolve(&r, &fetcher).unwrap();
        assert_eq!(hit.source, RegistrySource::CacheFresh);
        let zero_ttl = RegistryCache::with_ttl(&root, Duration::ZERO, true);
        let stale = zero_ttl.resolve(&r, &fetcher).unwrap();
        assert_eq!(stale.source, RegistrySource::CacheStale);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn status_reports_fresh_stale_missing_corrupt() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = scratch("status");
        let (r, _file) = file_ref(&root);
        let fetcher = Fetcher::new();
        let cache = RegistryCache::new(&root, false);

        assert_eq!(cache.status(&r), RegistryCacheStatus::Missing);
        cache.resolve(&r, &fetcher).unwrap();
        match cache.status(&r) {
            RegistryCacheStatus::Fresh { age } => assert!(age < Duration::from_secs(60)),
            other => panic!("expected Fresh, got {other:?}"),
        }

        // backdate the meta beyond the TTL → Stale
        let (yaml, meta, _) = cache.paths(&r);
        let old = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 25 * 3600;
        fs::write(&meta, format!("fetched-at: {old}\nref: {r}\n")).unwrap();
        assert!(matches!(
            cache.status(&r),
            RegistryCacheStatus::Stale { .. }
        ));

        fs::write(&yaml, "not a registry\n").unwrap();
        assert!(matches!(
            cache.status(&r),
            RegistryCacheStatus::Corrupt { .. }
        ));
        let _ = fs::remove_dir_all(&root);
    }
}
