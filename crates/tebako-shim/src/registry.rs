//! Dispatch-time registry access (spec 07 §2.1's last chain link + spec 04
//! §2/§3): the version-resolution chain's registry-default link resolves
//! EVERY configured registry through tebako-resolve's machinery — the
//! default-branch contents API, the pinned release artifact, the git blob,
//! or a `file://` mirror — behind the dispatch-time registry cache
//! (`~/.tebako/registries/<sha256-of-ref>.yaml`, 24 h TTL). The shim never
//! links tebako-cli; everything it needs is tebako-resolve's public
//! registry/cache surface.
//!
//! - `TEBAKO_OFFLINE`: a cache hit (fresh or stale) or the named
//!   unavailable error (spec 04 §3 cache-or-named-error).
//! - [`refresh`] is the `tebako-shim update-registries` body (the CLI's
//!   `tebako update-registries` drives the same tebako-resolve cache).
//! - [`statuses`] feeds `tebako-shim doctor`'s registry-freshness report.

use std::fmt::Write as _;

use tebako_resolve::regcache::RegistryCacheStatus;
use tebako_resolve::{Fetcher, RegistryCache, RegistryRef, ResolveError};

use crate::config::UserConfig;
use crate::{Ctx, ShimError, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST, EX_TEBAKO_UNAVAILABLE};

/// TEBAKO_OFFLINE truthiness from the injected context (the shim's
/// convention: set and not "0").
fn offline_mode(ctx: &Ctx) -> bool {
    ctx.env_get("TEBAKO_OFFLINE")
        .is_some_and(|v| !v.is_empty() && v != "0")
}

/// The dispatch-time registry cache for this context.
pub fn cache(ctx: &Ctx) -> RegistryCache {
    RegistryCache::new(ctx.home.clone(), offline_mode(ctx))
}

/// ResolveError → the shim's named-error exit codes (mirrors the CLI's
/// mapping in tebako-cli::install).
fn map_resolve(e: ResolveError) -> ShimError {
    let code = match &e {
        ResolveError::Reference(_) | ResolveError::GitPathRequired { .. } => crate::EX_USAGE,
        ResolveError::Sha256Mismatch { .. } => crate::EX_TEBAKO_SHA,
        ResolveError::Registry(_) | ResolveError::InvalidCacheKey { .. } => EX_TEBAKO_MANIFEST,
        ResolveError::LockTimeout { .. } | ResolveError::CacheIo { .. } => EX_TEBAKO_IO,
        ResolveError::NotFound { .. }
        | ResolveError::DownloadFailed { .. }
        | ResolveError::AssetNotFound { .. }
        | ResolveError::AmbiguousAssets { .. }
        | ResolveError::ServiceFailed { .. }
        | ResolveError::Git { .. }
        | ResolveError::Offline { .. } => EX_TEBAKO_UNAVAILABLE,
    };
    ShimError::new(code, e.to_string())
}

/// Parse a configured registry ref; a bad ref is a named error listing
/// the spec 04 §2 forms (never a fallback chain).
fn parse_ref(reg_ref: &str) -> Result<RegistryRef, ShimError> {
    RegistryRef::parse(reg_ref).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("registered registry \"{reg_ref}\" is invalid: {e}"),
        )
    })
}

/// The registry default version for `payload_name`, scanning the user's
/// registered registries in order (first match wins). Each registry
/// resolves through the dispatch-time cache — remote registries included.
pub fn registry_default(
    config: &UserConfig,
    payload_name: &str,
    ctx: &Ctx,
) -> Result<Option<(String, String)>, ShimError> {
    let cache = cache(ctx);
    let fetcher = Fetcher::new();
    for reg_ref in &config.registries {
        let r = parse_ref(reg_ref)?;
        let resolved = cache.resolve(&r, &fetcher).map_err(|e| {
            let mapped = map_resolve(e);
            ShimError::new(
                mapped.code,
                format!(
                    "registry \"{reg_ref}\" is unusable: {}\n  run `tebako-shim doctor` to diagnose the registry layer; `tebako update-registries` refreshes the cache",
                    mapped.message
                ),
            )
        })?;
        if let Some(p) = resolved.registry.payload(payload_name) {
            if let Some(default) = &p.default {
                return Ok(Some((default.clone(), reg_ref.clone())));
            }
        }
    }
    Ok(None)
}

/// The `update-registries` body: force-refresh every registered registry
/// regardless of freshness. Per-registry lines; exit code 1 when any
/// refresh failed (the failures are named, never silent).
pub fn refresh(ctx: &Ctx) -> Result<(String, u8), ShimError> {
    let cfg = crate::config::load_config(&ctx.home)?;
    let cache = cache(ctx);
    let fetcher = Fetcher::new();
    let mut out = String::new();
    let mut failures = 0u32;
    if cfg.registries.is_empty() {
        let _ = writeln!(
            out,
            "no registries registered — `tebako add-registry <ref>` registers one"
        );
    }
    for reg_ref in &cfg.registries {
        let line = match parse_ref(reg_ref) {
            Ok(r) => match cache.refresh(&r, &fetcher) {
                Ok(cached) => format!(
                    "refreshed {reg_ref} ({} payload(s), cached at {})",
                    cached.registry.payloads.len(),
                    cached.cache_path.display()
                ),
                Err(e) => {
                    failures += 1;
                    format!("failed {reg_ref}: {}", first_line(&map_resolve(e).message))
                }
            },
            Err(e) => {
                failures += 1;
                format!("failed {reg_ref}: {}", first_line(&e.message))
            }
        };
        let _ = writeln!(out, "{line}");
    }
    Ok((out, u8::from(failures > 0)))
}

/// The doctor report: one freshness line per registered registry (cache
/// reads only — doctor never touches the network).
pub fn statuses(ctx: &Ctx) -> Result<(Vec<String>, Vec<String>), ShimError> {
    let cfg = crate::config::load_config(&ctx.home)?;
    let cache = cache(ctx);
    let mut notes = Vec::new();
    let mut problems = Vec::new();
    for reg_ref in &cfg.registries {
        let r = match parse_ref(reg_ref) {
            Ok(r) => r,
            Err(e) => {
                problems.push(format!("registry {reg_ref}: {}", first_line(&e.message)));
                continue;
            }
        };
        match cache.status(&r) {
            RegistryCacheStatus::Missing => {
                if r.is_remote() {
                    problems.push(format!(
                        "registry {reg_ref}: not in the dispatch-time cache — run `tebako update-registries`"
                    ));
                } else {
                    // a file:// mirror with no cache entry yet: check the
                    // file itself (first dispatch populates the cache)
                    let path = reg_ref.strip_prefix("file://").unwrap_or(reg_ref);
                    if std::path::Path::new(path).is_file() {
                        notes.push(format!("registry {reg_ref}: local mirror (not yet cached)"));
                    } else {
                        problems.push(format!("registry {reg_ref}: file not found"));
                    }
                }
            }
            RegistryCacheStatus::Fresh { age } => notes.push(format!(
                "registry {reg_ref}: cached, fresh (age {})",
                format_age(age)
            )),
            RegistryCacheStatus::Stale { age } => problems.push(format!(
                "registry {reg_ref}: cache is stale (age {}, TTL 24h) — run `tebako update-registries`",
                format_age(age)
            )),
            RegistryCacheStatus::Corrupt { reason } => problems.push(format!(
                "registry {reg_ref}: cache entry is corrupt ({}) — run `tebako update-registries` to repair it",
                first_line(&reason)
            )),
        }
    }
    Ok((notes, problems))
}

fn first_line(message: &str) -> &str {
    message.lines().next().unwrap_or(message)
}

fn format_age(age: std::time::Duration) -> String {
    let secs = age.as_secs();
    if secs < 3600 {
        format!("{} min", secs.div_ceil(60))
    } else if secs < 48 * 3600 {
        format!("{} h", secs / 3600)
    } else {
        format!("{} d", secs / (24 * 3600))
    }
}
