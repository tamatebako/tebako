//! The dispatch-time registry cache (spec 04 §2 + spec 05 §4 discipline,
//! roadmap 33): the shim's registry-default chain link resolves EVERY
//! registry form through tebako-resolve — service contents API, pinned
//! release artifact, git blob, `file://` — and remote registries ride a
//! per-ref cache so dispatch never blocks on the network twice in a row:
//!
//! ```text
//! ~/.tebako/registries/<sha256-of-canonical-ref>.yaml        # the registry bytes
//! ~/.tebako/registries/<sha256-of-canonical-ref>.fetched-at  # unix seconds
//! ```
//!
//! Semantics (locked with the item):
//!
//! - fresh cache (age < [`REGISTRY_TTL`], 24 h) → read the cache, no fetch;
//! - stale/missing cache, online → fetch through tebako-resolve, publish
//!   tmp + rename, then read;
//! - `TEBAKO_OFFLINE` → the cache (ANY age) or a named error — never a
//!   fetch attempt;
//! - `file://` refs (and hand-authored plain paths) read directly, no
//!   cache at all;
//! - `tebako update-registries` ([`refresh`]) force-renews the cache;
//!   `tebako add-registry` primes it with the bytes it already fetched.

use std::path::{Path, PathBuf};

use tebako_resolve::registry::RegistryRef;
use tebako_resolve::{Fetcher, Registry, ResolveError, Transport};

use crate::{fail, Ctx, ShimError, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST, EX_TEBAKO_UNAVAILABLE};

/// Dispatch-cache freshness window: 24 hours.
pub const REGISTRY_TTL_SECS: u64 = 24 * 3600;

/// The directory the per-ref cache lives in (`~/.tebako/registries`).
pub fn registries_dir(home: &Path) -> PathBuf {
    home.join("registries")
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn offline(ctx: &Ctx) -> bool {
    ctx.env_get("TEBAKO_OFFLINE")
        .is_some_and(|v| !v.is_empty() && v != "0")
}

/// `file://` refs and hand-authored plain paths read directly (the
/// lenient read side — `tebako add-registry` only ever WRITES canonical
/// refs, but a hand-maintained config.yaml may carry a path).
fn local_registry_path(reg_ref: &str) -> Option<PathBuf> {
    if let Some(rest) = reg_ref.strip_prefix("file://") {
        // RFC 8089 drive-path recovery (file:///C:/x → C:/x on Windows),
        // the same rule tebako-http's reader applies.
        return Some(PathBuf::from(tebako_http::file_path_from_url(rest)));
    }
    // A plain absolute path (`/x`, `C:\x`) or an explicit relative one —
    // never a bare name (that is a registry ref's grammar, not a path).
    if Path::new(reg_ref).is_absolute()
        || reg_ref.starts_with("./")
        || reg_ref.starts_with("../")
        || reg_ref.starts_with(".\\")
        || reg_ref.starts_with("..\\")
    {
        Some(PathBuf::from(reg_ref))
    } else {
        None
    }
}

fn cache_key(canonical_ref: &str) -> String {
    tebako_resolve::sha256_hex(canonical_ref.as_bytes())
}

fn cache_file(home: &Path, canonical_ref: &str) -> PathBuf {
    registries_dir(home).join(format!("{}.yaml", cache_key(canonical_ref)))
}

fn fetched_at_file(home: &Path, canonical_ref: &str) -> PathBuf {
    registries_dir(home).join(format!("{}.fetched-at", cache_key(canonical_ref)))
}

fn read_fetched_at(home: &Path, canonical_ref: &str) -> Option<u64> {
    std::fs::read_to_string(fetched_at_file(home, canonical_ref))
        .ok()
        .and_then(|t| t.trim().parse().ok())
}

/// Write the cache (tmp + rename, like every cache-managed file).
fn prime_unchecked(home: &Path, canonical_ref: &str, bytes: &[u8]) -> Result<(), ShimError> {
    let dir = registries_dir(home);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", dir.display()),
        )
    })?;
    let file = cache_file(home, canonical_ref);
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        file.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
        std::process::id()
    ));
    std::fs::write(&tmp, bytes).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &file).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", file.display()),
        )
    })?;
    std::fs::write(
        fetched_at_file(home, canonical_ref),
        format!("{}\n", now_unix()),
    )
    .map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot write the fetched-at marker: {e}"),
        )
    })
}

/// Prime the cache with already-fetched bytes (the `tebako add-registry`
/// flow fetched the registry once to validate it — those bytes ARE the
/// first cache entry). No-op for `file://` refs (they read directly).
pub fn prime(home: &Path, canonical_ref: &str, bytes: &[u8]) -> Result<(), ShimError> {
    if local_registry_path(canonical_ref).is_some() {
        return Ok(());
    }
    let parsed = RegistryRef::parse(canonical_ref).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot prime the registry cache for '{canonical_ref}': {e}"),
        )
    })?;
    if !parsed.is_remote() {
        return Ok(());
    }
    prime_unchecked(home, &parsed.as_canonical_string(), bytes)
}

fn parse_registry(bytes: &[u8], origin: &str) -> Result<Registry, ShimError> {
    let text = String::from_utf8(bytes.to_vec()).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("the registry from {origin} is not UTF-8: {e}"),
        )
    })?;
    Registry::from_yaml(&text).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot parse the registry from {origin}: {e}"),
        )
    })
}

fn map_resolve(home: &Path, reg_ref: &str, e: ResolveError) -> ShimError {
    let code = match &e {
        ResolveError::Sha256Mismatch { .. } => crate::EX_TEBAKO_SHA,
        ResolveError::LockTimeout { .. } | ResolveError::CacheIo { .. } => EX_TEBAKO_IO,
        ResolveError::Registry(_) | ResolveError::Reference(_) => EX_TEBAKO_MANIFEST,
        _ => EX_TEBAKO_UNAVAILABLE,
    };
    ShimError::new(
        code,
        format!(
            "cannot resolve registry \"{reg_ref}\" at dispatch time: {e}\n  retry, or run `tebako update-registries`; the dispatch cache lives in {}",
            registries_dir(home).display()
        ),
    )
}

/// Resolve the registry `reg_ref` names for the dispatch-time chain:
/// `file://`/plain paths directly, remote refs cache-first (fresh cache,
/// else fetch + prime; `TEBAKO_OFFLINE` = cache-or-named-error).
pub fn registry_for(home: &Path, reg_ref: &str, ctx: &Ctx) -> Result<Registry, ShimError> {
    registry_for_with(home, reg_ref, &Fetcher::new(), offline(ctx), now_unix())
}

/// The transport/clock-injected half of [`registry_for`] (tests).
pub fn registry_for_with<T: Transport>(
    home: &Path,
    reg_ref: &str,
    fetcher: &Fetcher<T>,
    offline: bool,
    now: u64,
) -> Result<Registry, ShimError> {
    // Local forms read directly, no cache (and no fetch).
    if let Some(path) = local_registry_path(reg_ref) {
        let bytes = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ShimError::new(
                    EX_TEBAKO_MANIFEST,
                    format!("registry file {} does not exist", path.display()),
                )
            } else {
                ShimError::new(
                    EX_TEBAKO_IO,
                    format!("cannot read registry {}: {e}", path.display()),
                )
            }
        })?;
        return parse_registry(bytes.as_bytes(), &path.display().to_string());
    }
    let parsed = RegistryRef::parse(reg_ref).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("registry ref \"{reg_ref}\" is invalid: {e}"),
        )
    })?;
    let canonical = parsed.as_canonical_string();
    let cache = cache_file(home, &canonical);
    let age = read_fetched_at(home, &canonical).map(|t| now.saturating_sub(t));

    if offline {
        if cache.is_file() {
            let bytes = std::fs::read(&cache).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_IO,
                    format!("cannot read the cached registry {}: {e}", cache.display()),
                )
            })?;
            return parse_registry(&bytes, &format!("the dispatch cache {}", cache.display()));
        }
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "registry \"{canonical}\" is not in the dispatch cache ({}) and TEBAKO_OFFLINE is set\n  run `tebako update-registries` while online",
                registries_dir(home).display()
            ),
        );
    }

    if age.is_some_and(|a| a < REGISTRY_TTL_SECS) && cache.is_file() {
        let bytes = std::fs::read(&cache).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot read the cached registry {}: {e}", cache.display()),
            )
        })?;
        return parse_registry(&bytes, &format!("the dispatch cache {}", cache.display()));
    }

    // Stale or missing: fetch, publish, read.
    let bytes = fetcher
        .fetch_registry(&parsed)
        .map_err(|e| map_resolve(home, &canonical, e))?;
    prime_unchecked(home, &canonical, &bytes)?;
    parse_registry(&bytes, &canonical)
}

/// What [`refresh`] did with one registry ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// `file://`/plain path: reads directly, nothing to cache.
    LocalSkipped,
    /// Fetched and (re-)published into the dispatch cache.
    Refreshed,
}

/// Force-renew one registry's dispatch cache (`tebako update-registries`).
pub fn refresh(home: &Path, reg_ref: &str) -> Result<RefreshOutcome, ShimError> {
    refresh_with(home, reg_ref, &Fetcher::new())
}

/// The transport-injected half of [`refresh`] (tests).
pub fn refresh_with<T: Transport>(
    home: &Path,
    reg_ref: &str,
    fetcher: &Fetcher<T>,
) -> Result<RefreshOutcome, ShimError> {
    if local_registry_path(reg_ref).is_some() {
        return Ok(RefreshOutcome::LocalSkipped);
    }
    let parsed = RegistryRef::parse(reg_ref).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("registry ref \"{reg_ref}\" is invalid: {e}"),
        )
    })?;
    let canonical = parsed.as_canonical_string();
    let bytes = fetcher
        .fetch_registry(&parsed)
        .map_err(|e| map_resolve(home, &canonical, e))?;
    // refresh proves the registry still parses before caching it
    parse_registry(&bytes, &canonical)?;
    prime_unchecked(home, &canonical, &bytes)?;
    Ok(RefreshOutcome::Refreshed)
}

/// The dispatch-cache state of one registered registry (doctor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryFreshness {
    /// `file://`/plain path: no cache involved (the file's existence is
    /// the doctor's separate check).
    Local,
    /// Cached, age below the TTL (seconds).
    Fresh(u64),
    /// Cached but older than the TTL (seconds) — the next dispatch
    /// refreshes it; offline dispatch still resolves from it.
    Stale(u64),
    /// Remote and not cached: online dispatch fetches on demand, offline
    /// dispatch fails — `tebako update-registries` fixes it ahead of time.
    Missing,
    /// The configured ref does not parse.
    BadRef(String),
}

/// The freshness of one registered registry's dispatch cache.
pub fn freshness(home: &Path, reg_ref: &str) -> RegistryFreshness {
    freshness_at(home, reg_ref, now_unix())
}

/// The clock-injected half of [`freshness`] (tests).
pub fn freshness_at(home: &Path, reg_ref: &str, now: u64) -> RegistryFreshness {
    if local_registry_path(reg_ref).is_some() {
        return RegistryFreshness::Local;
    }
    let Ok(parsed) = RegistryRef::parse(reg_ref) else {
        return RegistryFreshness::BadRef(reg_ref.to_string());
    };
    let canonical = parsed.as_canonical_string();
    if !cache_file(home, &canonical).is_file() {
        return RegistryFreshness::Missing;
    }
    match read_fetched_at(home, &canonical) {
        Some(t) => {
            let age = now.saturating_sub(t);
            if age < REGISTRY_TTL_SECS {
                RegistryFreshness::Fresh(age)
            } else {
                RegistryFreshness::Stale(age)
            }
        }
        None => RegistryFreshness::Missing,
    }
}
