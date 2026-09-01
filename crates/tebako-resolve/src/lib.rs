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
//!   `tfs+git:` via gitoxide, never the git CLI (the default `git`
//!   feature — off in the size-capped bootstrap's link).
//! - [`PayloadCache`] — the shared `~/.tebako/payloads` store (spec 05
//!   §3–4): per-entry flock, tmp+rename atomic installs, `.sha256` trust
//!   anchor + `.origin` marker, `TEBAKO_OFFLINE` hard errors — plus the
//!   spec 23 §13.4 lazy-[`seed`](cache::PayloadCache::seed) verb and the
//!   lock-pinned [`resolve_locked_slice`].
//! - [`Registry`] / [`RegistryRef`] — the developer-hosted
//!   `tpkg-registry.yaml` model and its resolution (spec 04 §2): exactly
//!   one location per form, declarative host-triplet selection.
//! - [`contract`] — the runtime release card (spec 18 C2): the
//!   pre-download contract gate, era/contract_version/mount_root.
//! - [`store`] — the `~/.tebako` layout-version stamp (spec 18 C13):
//!   write-on-create, first-access check, named migration.

pub mod adapters;
pub mod cache;
pub mod contract;
pub mod error;
pub mod fetch;
#[cfg(feature = "git")]
pub mod git;
pub mod reference;
pub mod registry;
pub mod store;
pub mod transport;

pub use cache::{default_cache_root, CacheEntry, InstallStatus, PayloadCache, SeedOutcome};
pub use contract::{ContractError, ContractSet};
pub use error::{ReferenceError, RegistryError, ResolveError};
pub use fetch::{sha256_hex, FetchedPayload, Fetcher};
pub use reference::{Reference, Service};
pub use registry::{
    PlatformSelection, Registry, RegistryPayload, RegistryPlatforms, RegistryRef,
    RegistryRuntimeRequirement, RegistryVersion, ReleaseRef,
};
pub use transport::{HttpTransport, Transport};

/// The default tebako runtime release line (tamatebako/tebako-runtime-ruby)
/// the stack resolves against when nothing else pins one (spec 13 §2a's
/// default line). THE SINGLE OWNER: the CLI presses with it and the
/// shim's runtime download probes its release index — every consumer
/// flows it from here.
///
/// New-era only: from 0.16.1 the runtimes bake the renamed mount root
/// (`/__tfs__` on POSIX, `A:/t` on windows — per-platform baked defaults,
/// run-time overridable via `TEBAKO_MOUNT_ROOT` where the image grants
/// it; spec 17 §1); older releases carry the legacy `__tebako_memfs__`
/// layout and are NOT served (no old contract, no compat readers). From
/// 0.16.3 the runtimes carry the union-aware driver (the L2 `mounts:`
/// block the press writes, spec 03 §6) and the io.c zero-copy guard
/// (the linux deploy fix). From 0.16.7 the runtimes link the v0.2.2
/// driver unit (#433/#441/#443: the preload openssl BIO IO cover, the
/// gnu link-unit floor, the LFS64/fortify alias surface) on ruby source
/// v0.2.27, with the driver-owned SSL_CERT_FILE and the CRL_CHECK_ALL
/// removal (the tebako#437 TLS family) — verified end-to-end by the
/// factory's per-leg https_handshake boot smoke. From 0.16.9 the
/// runtimes link the v0.2.4 driver unit (#448/#449/#451: neither the
/// preload interpose dylib nor the driver's micro dylib reaches a
/// child process — dyld's arm64e insertion termination can no longer
/// kill runtime-spawned children; the 0.16.7/0.16.8 native-ext press
/// regression). From 0.16.18 the runtimes link the v2.1.0 driver unit
/// (the spec-30 spawn FFI): the driver's manifest grammar accepts the
/// `kind: runtime` DEPENDS edge — a pref-less resolution landing on an
/// older line fails closed at the payload's first dispatch with the
/// driver's named manifest error, so the default line must never trail
/// the newest published factory line.
pub const DEFAULT_TEBAKO_VERSION: &str = "0.16.18";

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

/// Resolve a lock-pinned shared slice (spec 23 §4/§13 — the composition
/// spectrum): a cache hit requires the trust anchor to EQUAL the lock's
/// pin — a different anchor is the spec 18 S63 digest mismatch,
/// fail-closed, never a silent re-resolve by semver. A miss fetches the
/// lock's `source` reference with the pin as the trust anchor
/// (`TEBAKO_OFFLINE=1` → the named offline error; a pin mismatch at the
/// fetch boundary deletes the download and caches nothing).
pub fn resolve_locked_slice(
    cache: &PayloadCache,
    name: &str,
    version: &str,
    pin: &str,
    source: &Reference,
) -> Result<CacheEntry, ResolveError> {
    let pin = pin.to_ascii_lowercase();
    let check = |entry: CacheEntry| -> Result<CacheEntry, ResolveError> {
        if entry.sha256 == pin {
            Ok(entry)
        } else {
            Err(ResolveError::Sha256Mismatch {
                origin: entry
                    .origin
                    .clone()
                    .unwrap_or_else(|| format!("cached payload {}@{}", entry.name, entry.version)),
                expected: pin.clone(),
                actual: entry.sha256.clone(),
            })
        }
    };
    if let Some(entry) = cache.get(name, version)? {
        return check(entry);
    }
    // The entry may land between the miss check and the install lock (a
    // concurrent process) — install returns it as a hit UNVERIFIED
    // against the pin, so the anchor check applies to its outcome too.
    let (entry, _) = fetch_and_cache(cache, source, name, version, Some(&pin))?;
    check(entry)
}

/// Every env-mutating test in this crate (TEBAKO_OFFLINE & friends)
/// serializes on this one mutex — the test modules share one process.
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> std::path::PathBuf {
        // Unique per test, not just per process: the tests run threaded,
        // and a shared pid-only dir means one test's remove_dir_all wipes
        // a sibling's mirror mid-fetch (the ubuntu CI NotFound flake).
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tebako-resolve-locked-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn file_reference(path: &std::path::Path) -> Reference {
        Reference::File {
            path: path.to_string_lossy().into_owned(),
            sha256: None,
        }
    }

    #[test]
    fn locked_slice_hit_requires_the_anchor_to_equal_the_pin() {
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        let pin = sha256_hex(b"payload");
        cache
            .install("tool", "1.0", None, || {
                Ok(FetchedPayload {
                    bytes: b"payload".to_vec(),
                    origin: "https://cdn.example.com/tool.tfs".to_string(),
                    sha256: pin.clone(),
                })
            })
            .unwrap();
        // The source does not exist — a fetch would fail; a hit never fetches.
        let missing = file_reference(&root.join("definitely-not-there.tfs"));
        let entry = resolve_locked_slice(&cache, "tool", "1.0", &pin, &missing).unwrap();
        assert_eq!(entry.sha256, pin);

        let err =
            resolve_locked_slice(&cache, "tool", "1.0", &"f".repeat(64), &missing).unwrap_err();
        let ResolveError::Sha256Mismatch {
            expected, actual, ..
        } = &err
        else {
            panic!("expected Sha256Mismatch, got {err:?}")
        };
        assert_eq!(expected, &"f".repeat(64));
        assert_eq!(actual, &pin);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_slice_miss_fetches_with_the_pin_as_anchor() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("TEBAKO_OFFLINE");
        let root = scratch();
        let mirror = root.join("mirror");
        std::fs::create_dir_all(&mirror).unwrap();
        let file = mirror.join("tool-1.0.tfs");
        std::fs::write(&file, b"payload").unwrap();
        let cache = PayloadCache::with_root(&root);
        let pin = sha256_hex(b"payload");
        let entry =
            resolve_locked_slice(&cache, "tool", "1.0", &pin, &file_reference(&file)).unwrap();
        assert_eq!(entry.sha256, pin);
        assert_eq!(std::fs::read(&entry.path).unwrap(), b"payload");
        // The second resolution is a cache hit — the mirror can go away.
        std::fs::remove_file(&file).unwrap();
        let entry =
            resolve_locked_slice(&cache, "tool", "1.0", &pin, &file_reference(&file)).unwrap();
        assert_eq!(entry.sha256, pin);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_slice_offline_miss_is_the_named_offline_error() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let root = scratch();
        let cache = PayloadCache::with_root(&root);
        std::env::set_var("TEBAKO_OFFLINE", "1");
        let err = resolve_locked_slice(
            &cache,
            "tool",
            "1.0",
            &"a".repeat(64),
            &file_reference(&root.join("x.tfs")),
        )
        .unwrap_err();
        std::env::remove_var("TEBAKO_OFFLINE");
        assert!(matches!(err, ResolveError::Offline { .. }));
        assert!(err.to_string().contains("tool@1.0"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_slice_pin_mismatch_at_the_boundary_caches_nothing() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("TEBAKO_OFFLINE");
        let root = scratch();
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("tool-1.0.tfs");
        std::fs::write(&file, b"payload").unwrap();
        let cache = PayloadCache::with_root(&root);
        let err = resolve_locked_slice(
            &cache,
            "tool",
            "1.0",
            &"f".repeat(64),
            &file_reference(&file),
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::Sha256Mismatch { .. }));
        assert!(!root.join("payloads/tool/1.0.tfs").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
