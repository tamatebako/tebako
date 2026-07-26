//! tebako-resolve — L3 payload resolution (spec 04/05, roadmap 07).
//!
//! Three pieces, one crate:
//!
//! - [`Reference`] — the MECE reference syntax (spec 04 §1): explicit
//!   adapters, no default service, named errors listing the classes.
//! - [`Fetcher`] — reference → bytes over an injected [`Transport`]
//!   (tebako-http in production, mocks and `file://` mirrors in tests);
//!   `tfs+git:` via gitoxide, never the git CLI.
//! - [`PayloadCache`] — the shared `~/.tebako/payloads` store (spec 05
//!   §3–4): per-entry flock, tmp+rename atomic installs, `.sha256` trust
//!   anchor + `.origin` marker, `TEBAKO_OFFLINE` hard errors.
//!
//! Registry *listing* (`tpkg-registry.yaml`, spec 04 §2) is a later
//! roadmap item; this crate resolves and caches single payloads.

pub mod adapters;
pub mod cache;
pub mod error;
pub mod fetch;
pub mod git;
pub mod reference;
pub mod transport;

pub use cache::{default_cache_root, CacheEntry, InstallStatus, PayloadCache};
pub use error::{ReferenceError, ResolveError};
pub use fetch::{sha256_hex, FetchedPayload, Fetcher};
pub use reference::{Reference, Service};
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
