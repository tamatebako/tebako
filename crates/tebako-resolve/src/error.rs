//! Named errors for the L3 resolution layer (spec 00 invariant 9: named
//! errors, never silent fallbacks; spec 04 §1: an unparseable reference is
//! a named error listing the classes, never a guess).

use std::fmt;
use std::path::PathBuf;

use crate::reference::Service;

/// The reference classes, spelled out in every [`ReferenceError::UnknownScheme`]
/// message (spec 04 §1 dispatch table, last row).
pub const REFERENCE_CLASSES: &str = "tfs:github:owner/repo:version, \
     tfs:gitlab:owner/repo:version, tfs:bb:owner/repo:version, \
     tfs+git://host/owner/repo.git[@ref][#path], tfs+https://url, https://url, \
     file:///path (any of them with an optional ?sha256=<64 hex> pin)";

/// Reference syntax errors (spec 04 §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceError {
    /// The string matches no reference class — rejected, never guessed.
    UnknownScheme { input: String },
    /// A recognized class whose pieces are malformed (empty owner, bad
    /// sha256 pin, non-absolute file path, …).
    Invalid { input: String, reason: String },
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceError::UnknownScheme { input } => write!(
                f,
                "unknown reference '{input}': no default service exists; expected one of {REFERENCE_CLASSES}"
            ),
            ReferenceError::Invalid { input, reason } => {
                write!(f, "invalid reference '{input}': {reason}")
            }
        }
    }
}

impl std::error::Error for ReferenceError {}

/// Registry errors (spec 04 §2): a bad registry *reference*, an
/// unparseable `tpkg-registry.yaml`, or a semantically invalid one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// The registry reference matches no registry form — rejected with the
    /// forms listed, never a fallback chain (spec 04 §2: exactly one
    /// location per form).
    BadRef { input: String, reason: String },
    /// The document carries no `schema_version` (spec 18 C8/S46): an
    /// era-1 document — refused by name, never a silent default.
    PreEra,
    /// YAML structural failure (the document does not match the model).
    Yaml { reason: String },
    /// Semantic validation failure (bad schema_version, dangling default,
    /// malformed sha256, …).
    Invalid { reason: String },
}

/// The registry reference forms, spelled out in every
/// [`RegistryError::BadRef`] message.
pub const REGISTRY_REF_FORMS: &str =
    "tfs:<service>:owner/repo (default-branch tpkg-registry.yaml), \
     tfs:<service>:owner/repo:version#tpkg-registry.yaml (release artifact), \
     tfs+git://host/owner/repo.git[@ref]#path, file:///path/tpkg-registry.yaml";

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::BadRef { input, reason } => write!(
                f,
                "invalid registry reference '{input}': {reason}; expected one of {REGISTRY_REF_FORMS}"
            ),
            RegistryError::PreEra => write!(
                f,
                "the registry carries no schema_version — a pre-era (era 1) document; republish the registry with a current tebako (spec 18 C8)"
            ),
            RegistryError::Yaml { reason } => {
                write!(f, "cannot parse the registry yaml: {reason}")
            }
            RegistryError::Invalid { reason } => write!(f, "invalid registry: {reason}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Fetch / cache errors (spec 04 §3, spec 05 §4).
#[derive(Debug)]
pub enum ResolveError {
    /// The reference string did not parse.
    Reference(ReferenceError),
    /// The remote object does not exist (HTTP 404 / ENOENT on file://).
    NotFound { origin: String },
    /// A download failed at the transport layer (after retries).
    DownloadFailed { origin: String, reason: String },
    /// The downloaded bytes do not match the pinned/expected digest; the
    /// download was deleted and nothing entered the cache (spec 04 §3).
    Sha256Mismatch {
        origin: String,
        expected: String,
        actual: String,
    },
    /// A service release carries no `.tfs` asset (no `#artifact` given), or
    /// no asset with the requested `#artifact` name (spec 04 §1).
    AssetNotFound {
        service: Service,
        owner: String,
        repo: String,
        version: String,
        /// The `#artifact` selector when one was given.
        artifact: Option<String>,
    },
    /// A service release carries several candidate `.tfs` assets and there
    /// is no rule to pick one (never a guess — the registry's per-platform
    /// entries disambiguate).
    AmbiguousAssets {
        service: Service,
        owner: String,
        repo: String,
        version: String,
        assets: Vec<String>,
    },
    /// The service API answered with something unusable.
    ServiceFailed { service: Service, reason: String },
    /// The git adapter failed (clone, ref resolution, blob lookup).
    Git { url: String, reason: String },
    /// A `tfs+git:` reference without `#path` names a registry repo, not a
    /// payload file (spec 04 §1) — fetching payload bytes needs the path.
    GitPathRequired { url: String },
    /// A `tfs+git:` reference reached a build without the git adapter
    /// (feature `git` off — the size-capped tebako-bootstrap, spec 04 §3):
    /// a named refusal, never a silent skip.
    GitAdapterDisabled { url: String },
    /// TEBAKO_OFFLINE is set and the entry is not cached (spec 05 §4:
    /// cache hit or hard error).
    Offline { what: String },
    /// The per-entry flock was not acquired within the timeout (spec 05 §4:
    /// 120 s with stale-lock hint).
    LockTimeout { lockfile: PathBuf, waited_secs: u64 },
    /// A cache key (name or version) is not path-safe.
    InvalidCacheKey { key: String, reason: String },
    /// An I/O failure inside the cache, with operation and path context.
    CacheIo {
        op: &'static str,
        path: PathBuf,
        reason: String,
    },
    /// The registry reference/model layer (spec 04 §2).
    Registry(RegistryError),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::Reference(e) => write!(f, "{e}"),
            ResolveError::NotFound { origin } => write!(f, "not found: {origin}"),
            ResolveError::DownloadFailed { origin, reason } => {
                write!(f, "download failed for {origin}: {reason}")
            }
            ResolveError::Sha256Mismatch {
                origin,
                expected,
                actual,
            } => write!(
                f,
                "sha256 mismatch for {origin}: expected {expected}, got {actual}; download deleted, nothing cached"
            ),
            ResolveError::AssetNotFound {
                service,
                owner,
                repo,
                version,
                artifact,
            } => match artifact {
                Some(a) => write!(
                    f,
                    "{} release {owner}/{repo}:{version} carries no asset named '{a}'",
                    service.name()
                ),
                None => write!(
                    f,
                    "{} release {owner}/{repo}:{version} carries no .tfs asset",
                    service.name()
                ),
            },
            ResolveError::AmbiguousAssets {
                service,
                owner,
                repo,
                version,
                assets,
            } => write!(
                f,
                "{} release {owner}/{repo}:{version} carries several .tfs assets ({}); re-run with #name to select one, or resolve a per-platform entry from the registry",
                service.name(),
                assets.join(", ")
            ),
            ResolveError::ServiceFailed { service, reason } => {
                write!(f, "{} API error: {reason}", service.name())
            }
            ResolveError::Git { url, reason } => write!(f, "git fetch of {url} failed: {reason}"),
            ResolveError::GitPathRequired { url } => write!(
                f,
                "tfs+git://{url} names a repository, not a payload: add #path to select the image file"
            ),
            ResolveError::GitAdapterDisabled { url } => write!(
                f,
                "tfs+git://{url} needs the git adapter, which is not compiled into this build: \
                 fetch it in managed mode (tebako install / the shim) or mirror the payload to tfs+https"
            ),
            ResolveError::Offline { what } => write!(
                f,
                "{what} is not cached and downloads are disabled (TEBAKO_OFFLINE is set)"
            ),
            ResolveError::LockTimeout {
                lockfile,
                waited_secs,
            } => write!(
                f,
                "another process is installing this payload (no lock after {waited_secs}s; lockfile: {} — remove it if the holder crashed)",
                lockfile.display()
            ),
            ResolveError::InvalidCacheKey { key, reason } => {
                write!(f, "invalid cache key '{key}': {reason}")
            }
            ResolveError::CacheIo { op, path, reason } => {
                write!(f, "{reason} ({op} {})", path.display())
            }
            ResolveError::Registry(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ResolveError {}

impl From<ReferenceError> for ResolveError {
    fn from(e: ReferenceError) -> Self {
        ResolveError::Reference(e)
    }
}

impl From<RegistryError> for ResolveError {
    fn from(e: RegistryError) -> Self {
        ResolveError::Registry(e)
    }
}
