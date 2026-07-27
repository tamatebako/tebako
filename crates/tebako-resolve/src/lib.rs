//! tebako-resolve — L3 payload resolution (spec 04/05, roadmap 07/28).
//!
//! Four pieces, one crate:
//!
//! - [`Reference`] — the MECE reference syntax (spec 04 §1): explicit
//!   adapters, no default service, named errors listing the classes;
//!   `#artifact` selects one asset of a multi-artifact release (locked —
//!   never host-triplet guessing).
//! - [`Fetcher`] — reference → bytes over an injected [`Transport`]
//!   (tebako-http in production, mocks and `file://` mirrors in tests);
//!   `tfs+git:` via gitoxide, never the git CLI.
//! - [`PayloadCache`] — the shared `~/.tebako/payloads` store (spec 05
//!   §3–4): per-entry flock, tmp+rename atomic installs, `.sha256` trust
//!   anchor + `.origin` marker, `TEBAKO_OFFLINE` hard errors.
//! - [`Registry`] / [`RegistryRef`] — the developer-hosted
//!   `tpkg-registry.yaml` model and its resolution (spec 04 §2): exactly
//!   one location per form, declarative host-triplet selection.
//! - [`RegistryCache`] — the dispatch-time registry cache (spec 04 §3,
//!   spec 07 §2.1): `registries/<sha256-of-ref>.yaml`, 24 h TTL,
//!   `refresh` for `tebako update-registries`, offline cache-or-error.

pub mod adapters;
pub mod cache;
pub mod error;
pub mod fetch;
pub mod git;
pub mod reference;
pub mod regcache;
pub mod registry;
pub mod transport;

pub use cache::{default_cache_root, CacheEntry, InstallStatus, PayloadCache};
pub use error::{ReferenceError, RegistryError, ResolveError};
pub use fetch::{sha256_hex, FetchedPayload, Fetcher};
pub use reference::{Reference, Service};
pub use regcache::{
    CachedRegistry, RegistryCache, RegistryCacheStatus, RegistrySource, REGISTRY_CACHE_TTL,
};
pub use registry::{
    PlatformSelection, Registry, RegistryPayload, RegistryPlatforms, RegistryRef, RegistryVersion,
};
pub use transport::{HttpTransport, Transport};

/// Fetch `reference` (pin-verified at the fetch boundary) and install it
/// as `payloads/<name>/<version>.tfs` — or return the existing entry.
/// `expected_sha256` is the registry-supplied trust anchor; the
/// reference's own pin is verified inside [`Fetcher::fetch`].
pub fn fetch_and_cache(
    cache: &PayloadCache,
    reference: &Reference,
    name: &str,
    version: &str,
    expected_sha256: Option<&str>,
) -> Result<(CacheEntry, InstallStatus), ResolveError> {
    let fetcher = Fetcher::new();
    cache.install(name, version, expected_sha256, || fetcher.fetch(reference))
}
