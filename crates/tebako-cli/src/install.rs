//! `tebako add-registry | list-registries | install | uninstall`
//! (spec 04 §2, spec 16 §3.3 — registry listing + native install,
//! roadmap 28 item 1).
//!
//! - **add-registry <ref>** — validates the reference, fetches the
//!   registry once (proving it parses), and appends it to
//!   `~/.tebako/config.yaml` `registries:` (the one authored-config write;
//!   other keys preserved). Shipped config has ZERO registries — explicit
//!   only.
//! - **install <ref | name[@ver]>** — a REFERENCE resolves directly
//!   (download → verify → cache → register shims); a NICKNAME searches
//!   the REGISTERED registries only (zero hits → a named error listing
//!   them plus the add-registry hint; several → AmbiguousRegistries).
//!   Version omitted → the registry default; `@ver` → exactly that. The
//!   host-triplet artifact is selected DECLARATIVELY from the registry
//!   entry (`universal` or `platforms[host]`) — never adapter-side
//!   guessing.
//! - **uninstall <name>** — removes the payload's shims and cache entry;
//!   the trust-anchor digests survive in the audit journal.
//!
//! Trust discipline: the reference's own `?sha256=` pin is verified at
//! the fetch boundary; the registry's per-triplet sha256 is the cache's
//! expected anchor; a registry-carried OpenPGP signature is verified
//! BEFORE anything enters the cache (strict — spec 09 §3), and an
//! unsigned entry is accepted with the v1-legacy stderr warning + audit
//! journal line (`TEBAKO_REQUIRE_SIGNED=1` hard-fails). Cache hits are
//! trusted per the `.sha256` anchor — never re-verified per run
//! (spec 05 §4). `TEBAKO_OFFLINE=1`: cache hit or the named hard error.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use tebako_resolve::registry::{
    PlatformSelection, RegistryPayload, RegistryRef, RegistryVersion, SignaturePin,
};
use tebako_resolve::{
    FetchedPayload, Fetcher, InstallStatus, PayloadCache, Reference, ResolveError, Transport,
};
use tebako_shim::config::{self, AddRegistryOutcome};
use tebako_shim::manifest::{self, Manifest, PayloadRecord};
use tebako_shim::{manage, versions, ShimError};
use tpkg::Platform;

use crate::error::TebakoError;
use crate::image_manifest;

// Exit codes: the spec 06 §4 named set (the tebako-cli TebakoError code
// is the process exit code).
const EX_USAGE: i32 = 64;
const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_UNAVAILABLE: i32 = 69;
const EX_TEBAKO_SHA: i32 = 70;
const EX_TEBAKO_SIGNATURE: i32 = 71;
const EX_TEBAKO_TRUST: i32 = 72;
const EX_TEBAKO_IO: i32 = 74;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, code)
}

pub(crate) fn map_resolve(e: ResolveError) -> TebakoError {
    let code = match &e {
        ResolveError::Reference(_) | ResolveError::GitPathRequired { .. } => EX_USAGE,
        ResolveError::Sha256Mismatch { .. } => EX_TEBAKO_SHA,
        ResolveError::NotFound { .. }
        | ResolveError::DownloadFailed { .. }
        | ResolveError::AssetNotFound { .. }
        | ResolveError::AmbiguousAssets { .. }
        | ResolveError::ServiceFailed { .. }
        | ResolveError::Git { .. }
        | ResolveError::GitAdapterDisabled { .. }
        | ResolveError::Offline { .. } => EX_TEBAKO_UNAVAILABLE,
        ResolveError::Registry(_) | ResolveError::InvalidCacheKey { .. } => EX_TEBAKO_MANIFEST,
        ResolveError::LockTimeout { .. } | ResolveError::CacheIo { .. } => EX_TEBAKO_IO,
    };
    TebakoError::new(e.to_string(), code)
}

pub(crate) fn map_shim(e: ShimError) -> TebakoError {
    TebakoError::new(e.message, i32::from(e.code))
}

// ---------------------------------------------------------------------
// add-registry / list-registries
// ---------------------------------------------------------------------

/// `tebako add-registry <ref>` (spec 04 §2): validate the reference,
/// fetch + parse the registry once, then register the canonical ref.
/// Returns the outcome and the fetched registry (for the summary line).
pub fn add_registry(
    home: &Path,
    registry_ref: &str,
) -> Result<(AddRegistryOutcome, tebako_resolve::Registry), TebakoError> {
    add_registry_with(home, registry_ref, &Fetcher::new())
}

/// The transport-injected half of [`add_registry`] (tests).
pub fn add_registry_with<T: Transport>(
    home: &Path,
    registry_ref: &str,
    fetcher: &Fetcher<T>,
) -> Result<(AddRegistryOutcome, tebako_resolve::Registry), TebakoError> {
    let r = RegistryRef::parse(registry_ref).map_err(|e| err(EX_USAGE, e.to_string()))?;
    let bytes = fetcher.fetch_registry(&r).map_err(map_resolve)?;
    let text = String::from_utf8(bytes.clone()).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("the registry file is not UTF-8: {e}"),
        )
    })?;
    let registry = tebako_resolve::Registry::from_yaml(&text).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("cannot parse the registry: {e}"),
        )
    })?;
    let outcome = config::add_registry(home, &r.as_canonical_string()).map_err(map_shim)?;
    // Prime the dispatch-time registry cache with the bytes just fetched
    // (roadmap 33): the shim's registry-default link then resolves this
    // remote registry without a second fetch. A prime failure never fails
    // the add — dispatch refreshes on demand; noted, not silent.
    if r.is_remote() {
        if let Err(e) = tebako_shim::regcache::prime(home, &r.as_canonical_string(), &bytes) {
            eprintln!(
                "tebako: note: could not prime the dispatch registry cache: {}",
                e.message
            );
        }
    }
    Ok((outcome, registry))
}

/// `tebako list-registries`: the registered refs, in config order.
pub fn list_registries(home: &Path) -> Result<Vec<String>, TebakoError> {
    Ok(config::load_config(home).map_err(map_shim)?.registries)
}

/// What `tebako update-registries` did (roadmap 33): per-registry outcome
/// of the force-renew of the dispatch-time cache.
#[derive(Debug, Default)]
pub struct UpdateRegistriesOutcome {
    /// Remote registries fetched and (re-)published into the cache.
    pub refreshed: Vec<String>,
    /// `file://` registries (read directly at dispatch — nothing to cache).
    pub local: Vec<String>,
    /// `(ref, error)` pairs that failed to refresh.
    pub failed: Vec<(String, String)>,
}

/// `tebako update-registries`: force-renew every registered registry's
/// dispatch cache. Every ref is attempted; failures are collected, not
/// fatal to the other refs (the CLI maps a non-empty `failed` to a
/// non-zero exit).
pub fn update_registries(home: &Path) -> Result<UpdateRegistriesOutcome, TebakoError> {
    update_registries_with(home, &Fetcher::new())
}

/// The transport-injected half of [`update_registries`] (tests).
pub fn update_registries_with<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
) -> Result<UpdateRegistriesOutcome, TebakoError> {
    let cfg = config::load_config(home).map_err(map_shim)?;
    let mut out = UpdateRegistriesOutcome::default();
    for reg_ref in &cfg.registries {
        match tebako_shim::regcache::refresh_with(home, reg_ref, fetcher) {
            Ok(tebako_shim::regcache::RefreshOutcome::Refreshed) => {
                out.refreshed.push(reg_ref.clone())
            }
            Ok(tebako_shim::regcache::RefreshOutcome::LocalSkipped) => {
                out.local.push(reg_ref.clone())
            }
            Err(e) => out.failed.push((reg_ref.clone(), e.message)),
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// install
// ---------------------------------------------------------------------

/// What an install produced.
#[derive(Debug)]
pub struct InstallOutcome {
    pub name: String,
    pub version: String,
    pub status: InstallStatus,
    /// The cached image (`payloads/<name>/<version>.tfs`).
    pub path: PathBuf,
    pub sha256: String,
    /// The registered commands (entrypoints).
    pub commands: Vec<String>,
    /// The shim links created.
    pub shims: Vec<PathBuf>,
    /// The verified signer keyid (16 hex) when the entry was signed.
    pub signer: Option<String>,
    /// Human notes (fallback path convention, manifest/ref mismatches).
    pub notes: Vec<String>,
}

/// Everything needed to place one payload — the two install forms
/// converge on this. `pub(crate)` for the compose press path (spec 23
/// §13): it resolves through the same registry selection, then stops
/// before the install verbs (mirror/shims/materialize are install's,
/// never press's).
pub(crate) struct InstallPlan {
    pub(crate) name: String,
    pub(crate) version: String,
    /// The payload kind (the registry declares it; the ref form assumes
    /// app — its mirror falls back to the entrypoint convention).
    pub(crate) kind: tpkg::PayloadKind,
    pub(crate) reference: Reference,
    /// The registry per-triplet sha256 anchor (the cache's expected
    /// digest; the reference's own pin is verified at the fetch boundary).
    pub(crate) expected_sha256: Option<String>,
    signature: Option<SignaturePin>,
    /// Registry-declared entrypoint names (the ref form has none and
    /// falls back to the payload name).
    pub(crate) entrypoints: Vec<String>,
    /// engine + constraint + the optional abi line (native-extension
    /// payloads, spec 05 §5)
    runtime_requirement: Option<(String, String, Option<String>)>,
    /// Name form: the embedded manifest MUST agree with the registry's
    /// name/version (the registry is the trust source).
    strict_identity: bool,
}

/// `tebako install <ref | name[@ver]>`. `host` overrides the detected
/// host platform (tests); `shim_binary` overrides the dispatcher binary
/// to link (tests; production: $TEBAKO_SHIM_BINARY or the sibling of the
/// tebako binary).
pub fn install(
    home: &Path,
    target: &str,
    host: Option<Platform>,
    shim_binary: Option<&Path>,
) -> Result<InstallOutcome, TebakoError> {
    install_with(home, target, host, shim_binary, &Fetcher::new())
}

/// The transport-injected half of [`install`] (tests).
pub fn install_with<T: Transport>(
    home: &Path,
    target: &str,
    host: Option<Platform>,
    shim_binary: Option<&Path>,
    fetcher: &Fetcher<T>,
) -> Result<InstallOutcome, TebakoError> {
    if looks_like_reference(target) {
        let plan = plan_from_reference(target)?;
        finish_install(home, fetcher, plan, shim_binary, &mut Vec::new())
    } else {
        let plan = plan_from_nickname(home, fetcher, target, host)?;
        finish_install(home, fetcher, plan, shim_binary, &mut Vec::new())
    }
}

/// A reference carries a scheme; a nickname never does.
fn looks_like_reference(target: &str) -> bool {
    target.contains("://") || target.starts_with("tfs:")
}

// ---- the local-package form (TODO.v2-1/12) ----------------------------

/// The exit code for a refused or impossible package install (spec 06 §4).
const EX_TEBAKO_INSTALL: i32 = 76;

/// True when `path` is a pressed tebako package (a readable tpkg
/// trailer) — the local-install discriminator (sniff the trailer, never
/// guess by extension).
pub fn is_tpkg_package(path: &Path) -> bool {
    std::fs::File::open(path)
        .map(|mut f| tpkg::read_from(&mut f).is_ok())
        .unwrap_or(false)
}

/// One installed slice of a local package.
#[derive(Debug)]
pub struct LocalInstalled {
    pub name: String,
    pub version: String,
    pub status: InstallStatus,
    pub path: PathBuf,
    pub sha256: String,
    pub commands: Vec<String>,
}

/// The local-package install report (a package may carry several payload
/// slices).
#[derive(Debug)]
pub struct LocalInstallOutcome {
    pub installed: Vec<LocalInstalled>,
    pub shims: Vec<PathBuf>,
    pub notes: Vec<String>,
}

/// `tebako install <path>` (TODO.v2-1/12): install every payload slice
/// of a local pressed package (fat or lean) into the store. Trust
/// anchors: the signed trailer's per-slot digest for v2 packages, the
/// computed digest for unsigned ones — exactly the run's own
/// enforcement strength, never upgraded (spec 06 §3's honesty).
/// Shims link only when explicitly asked (`--shims`) — a one-off
/// install never claims PATH names (the owner rule).
pub fn install_local(
    home: &Path,
    path: &Path,
    link_shims: bool,
    shim_binary: Option<&Path>,
) -> Result<LocalInstallOutcome, TebakoError> {
    let mut f = std::fs::File::open(path)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot open {}: {e}", path.display())))?;
    let m = tpkg::read_from(&mut f).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "corrupt tebako manifest trailer in {} ({})",
                path.display(),
                tpkg::strerror(e.code())
            ),
        )
    })?;
    if m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL != 0 {
        return Err(err(
            EX_TEBAKO_INSTALL,
            format!(
                "{} was built non-installable (TPKG_FLAG_NO_INSTALL — the publisher froze it); it runs standalone",
                path.display()
            ),
        ));
    }
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    // The two-slot era (spec 19 §6.1 / spec 23 §13): the lock's runtime
    // artifacts (exe / env image / windows dll) are not payload slices —
    // they seed the runtime cache on first run, never the payload store.
    let pm = m.package_manifest().map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "invalid package manifest (extension block type 2) in {}: {e}",
                path.display()
            ),
        )
    })?;
    let runtime_slots: BTreeSet<usize> = pm
        .and_then(|pm| pm.lock)
        .and_then(|lock| lock.runtime)
        .map(|runtime| {
            [&runtime.exe, &runtime.image, &runtime.dll]
                .into_iter()
                .flatten()
                .map(|a| a.slot as usize)
                .collect()
        })
        .unwrap_or_default();
    let cache = PayloadCache::with_root(home);
    let mut outcome = LocalInstallOutcome {
        installed: Vec::new(),
        shims: Vec::new(),
        notes: Vec::new(),
    };
    // The staging dir must be unique per CALL, not per clock tick: macOS's
    // SystemTime granularity is coarse enough that two parallel callers
    // land on the same nanosecond, and a shared slot-0.tfs let one
    // package's bytes masquerade as another's (the identity came from the
    // embedded manifest of whichever extract wrote last). tempfile gives
    // a guaranteed-unique dir and removes it on drop.
    let tmp_root = tempfile::tempdir()
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot create a staging dir: {e}")))?;
    let result = (|| {
        for (i, slot) in m.slots.iter().enumerate() {
            if slot.format_id == tpkg::TPKG_FORMAT_RUNTIME {
                outcome.notes.push(
                    "the runtime slot is not store-installed; it resolves into the runtime cache on first run (unchanged)"
                        .to_string(),
                );
                continue;
            }
            if runtime_slots.contains(&i) {
                outcome.notes.push(
                    "the carried runtime artifacts seed the runtime cache on first run (spec 05 §4) — not store-installed"
                        .to_string(),
                );
                continue;
            }
            let slice = install_local_slot(
                home,
                &cache,
                &abs,
                &m,
                i,
                slot,
                tmp_root.path(),
                &mut outcome.notes,
            )?;
            outcome.installed.push(slice);
        }
        Ok::<(), TebakoError>(())
    })();
    result?;

    // Shims: an explicit ask only.
    if link_shims {
        let commands: Vec<String> = outcome
            .installed
            .iter()
            .flat_map(|s| s.commands.clone())
            .collect();
        if !commands.is_empty() {
            let binary = resolve_shim_binary(shim_binary)?;
            let (shims, shim_notes) =
                manage::link_shims(home, &binary, &commands).map_err(map_shim)?;
            outcome.shims = shims;
            outcome.notes.extend(shim_notes);
        }
    }
    Ok(outcome)
}

#[allow(clippy::too_many_arguments)]
fn install_local_slot(
    home: &Path,
    cache: &PayloadCache,
    abs_pkg: &Path,
    m: &tpkg::Manifest,
    index: usize,
    slot: &tpkg::Slot,
    tmp_root: &Path,
    notes: &mut Vec<String>,
) -> Result<LocalInstalled, TebakoError> {
    // 1. the slot bytes, out of the package
    let tmp = tmp_root.join(format!("slot-{index}.tfs"));
    extract_file_region(abs_pkg, slot.offset, slot.size, &tmp)?;

    // 2. identity from the embedded manifest (authoritative, spec 03 §1)
    let text = image_manifest::read_embedded_manifest(&tmp)?.ok_or_else(|| {
        err(
            EX_TEBAKO_INSTALL,
            format!(
                "slot {index} of {} carries no embedded manifest (/__tpkg__/manifest.yaml) — cannot derive the payload identity; press the payload with a manifest",
                abs_pkg.display()
            ),
        )
    })?;
    let embedded = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the embedded manifest of slot {index} in {} does not parse: {e}",
                abs_pkg.display()
            ),
        )
    })?;
    let name = embedded.identity.name.clone();
    let version = embedded.identity.version.clone();
    manifest::check_path_component("payload name", &name).map_err(map_shim)?;
    manifest::check_path_component("payload version", &version).map_err(map_shim)?;

    // 3. the trust anchor
    let computed = sha256_file(&tmp)?;
    let (expected, signed_by) = match &m.v2 {
        Some(v2) => (
            v2.slot_digest(index).map(|d| hex_lower(d)),
            Some(v2.signer_keyid_hex()),
        ),
        None => (None, None),
    };

    // 4. the drift rule: same name+version → same content skips
    // silently; DIFFERENT content never overwrites (loud note, doctor
    // surfaces it). The Hit still reports the mirror's commands — an
    // explicit --shims after a plain install must link, not no-op.
    if let Some(entry) = cache.get(&name, &version).map_err(map_resolve)? {
        if entry.sha256.eq_ignore_ascii_case(&computed) {
            notes.push(format!(
                "{name} {version} is already installed ({}); skipping",
                entry.path.display()
            ));
        } else {
            notes.push(format!(
                "WARNING: {name} {version} is already installed with DIFFERENT content\n  installed: {}\n  this package: {}\n  keeping the installed one — inspect with `tebako doctor`",
                entry.sha256, computed
            ));
        }
        let record: PayloadRecord = manifest::payload_record(home, &name, &version);
        let commands = Manifest::load(&record.manifest_mirror)
            .map(|m| {
                m.entrypoints()
                    .iter()
                    .filter(|e| e.is_active())
                    .map(|e| e.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        return Ok(LocalInstalled {
            name,
            version,
            status: InstallStatus::Hit,
            path: entry.path,
            sha256: entry.sha256,
            commands,
        });
    }

    // 5. install into the store (provenance rides the origin marker)
    let origin = match &signed_by {
        Some(keyid) => format!(
            "payload={} slot={index} signed_by={keyid}",
            abs_pkg.display()
        ),
        None => format!("payload={} slot={index}", abs_pkg.display()),
    };
    let bytes = std::fs::read(&tmp)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot read {}: {e}", tmp.display())))?;
    let sha_for_closure = computed.clone();
    let (entry, status) = cache
        .install(&name, &version, expected.as_deref(), || {
            Ok(FetchedPayload {
                bytes,
                origin: origin.clone(),
                sha256: sha_for_closure,
            })
        })
        .map_err(map_resolve)?;

    // 6. the manifest mirror (the embedded manifest IS the authoritative
    // record — spec 03 §4 tier 1)
    let mirror = Manifest::from_payload_manifest(embedded);
    let record: PayloadRecord = manifest::payload_record(home, &name, &version);
    mirror.save(&record.manifest_mirror).map_err(map_shim)?;
    let commands: Vec<String> = mirror
        .entrypoints()
        .iter()
        .filter(|e| e.is_active())
        .map(|e| e.name.clone())
        .collect();

    journal(
        home,
        &format!(
            "event=payload-installed name={name} version={version} sha256={} origin={origin}",
            entry.sha256
        ),
    );
    Ok(LocalInstalled {
        name,
        version,
        status,
        path: entry.path,
        sha256: entry.sha256,
        commands,
    })
}

fn extract_file_region(src: &Path, offset: u64, size: u64, dst: &Path) -> Result<(), TebakoError> {
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut f = std::fs::File::open(src)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot open {}: {e}", src.display())))?;
    f.seek(SeekFrom::Start(offset))
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot seek {}: {e}", src.display())))?;
    let mut limited = f.take(size);
    let mut out = std::fs::File::create(dst).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", dst.display()),
        )
    })?;
    std::io::copy(&mut limited, &mut out).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot extract to {}: {e}", dst.display()),
        )
    })?;
    out.flush()
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot write {}: {e}", dst.display())))
}

fn sha256_file(path: &Path) -> Result<String, TebakoError> {
    use sha2::Digest as _;
    let mut f = std::fs::File::open(path)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot open {}: {e}", path.display())))?;
    let mut h = sha2::Sha256::new();
    std::io::copy(&mut f, &mut h)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot hash {}: {e}", path.display())))?;
    Ok(hex_lower(&h.finalize()))
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---- the ref form -----------------------------------------------------

fn plan_from_reference(target: &str) -> Result<InstallPlan, TebakoError> {
    let reference = Reference::parse(target).map_err(|e| err(EX_USAGE, e.to_string()))?;
    let (name, version) = reference_identity(&reference)?;
    manifest::check_path_component("payload name", &name).map_err(map_shim)?;
    manifest::check_path_component("version", &version).map_err(map_shim)?;
    Ok(InstallPlan {
        name,
        version,
        kind: tpkg::PayloadKind::App,
        reference,
        expected_sha256: None,
        signature: None,
        entrypoints: Vec::new(),
        runtime_requirement: None,
        strict_identity: false,
    })
}

/// Where a direct reference gets its `<name>/<version>` cache key from.
/// Service releases name both; git refs name the version via `@ref`;
/// verbatim https/file artifacts are content-addressed by their pin
/// (spec 16 §3.3's `?sha256=` form).
fn reference_identity(reference: &Reference) -> Result<(String, String), TebakoError> {
    match reference {
        Reference::Service { repo, version, .. } => Ok((repo.clone(), version.clone())),
        Reference::Git {
            url,
            git_ref: Some(r),
            ..
        } => Ok((stem_of(url), r.clone())),
        Reference::Git { url, .. } => Err(err(
            EX_USAGE,
            format!(
                "tfs+git://{url} names a repository, not a versioned payload — add @<ref> (and #path when the repo holds several images)"
            ),
        )),
        Reference::Https { url, sha256 } => Ok((
            stem_of(url),
            pinned_version(url, sha256.as_deref())?,
        )),
        Reference::File { path, sha256 } => Ok((
            stem_of(path),
            pinned_version(path, sha256.as_deref())?,
        )),
    }
}

/// `<stem>[.tfs]` of the last path segment (query already excluded for
/// https urls by the parser, stripped here for safety).
fn stem_of(path_or_url: &str) -> String {
    let base = path_or_url.rsplit('/').next().unwrap_or(path_or_url);
    let base = base.split('?').next().unwrap_or(base);
    let base = base.strip_suffix(".git").unwrap_or(base);
    base.strip_suffix(".tfs").unwrap_or(base).to_string()
}

/// The content address of a verbatim artifact: the `?sha256=` pin IS the
/// version (spec 16 §3.3's example form). Unpinned https/file references
/// carry no version at all — a named error, never a guess.
fn pinned_version(origin: &str, sha256: Option<&str>) -> Result<String, TebakoError> {
    sha256.map(str::to_string).ok_or_else(|| {
        err(
            EX_USAGE,
            format!(
                "{origin} carries no version — pin it (?sha256=<64 hex>, its content address), or install it through a registry (tebako add-registry <ref>)"
            ),
        )
    })
}

// ---- the nickname form --------------------------------------------------

fn parse_nickname(target: &str) -> Result<(String, Option<String>), TebakoError> {
    let (name, version) = match target.split_once('@') {
        Some((n, v)) => (n.to_string(), Some(v.to_string())),
        None => (target.to_string(), None),
    };
    manifest::check_path_component("payload name", &name).map_err(map_shim)?;
    if name.contains(':') {
        return Err(err(
            EX_USAGE,
            format!("'{target}' is neither a reference (tfs:…, tfs+…, https://…, file://…) nor a payload name"),
        ));
    }
    if let Some(v) = &version {
        manifest::check_path_component("version", v).map_err(map_shim)?;
        if v.contains(':') {
            return Err(err(
                EX_USAGE,
                format!("invalid version in '{target}' — the form is <name>@<version>"),
            ));
        }
    }
    Ok((name, version))
}

/// Search the REGISTERED registries for the payload, select version +
/// host entry, and build the plan (spec 04 §2, spec 16 §3.3 — nicknames
/// are EXPLICIT ONLY: zero defaults, zero global namespaces).
fn plan_from_nickname<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
    target: &str,
    host: Option<Platform>,
) -> Result<InstallPlan, TebakoError> {
    let (name, version_req) = parse_nickname(target)?;
    let mut found = find_in_registries(home, fetcher, &name)?;
    match found.len() {
        0 => {
            let registries = registered_registries_listing(home)?;
            Err(err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "no registered registry carries a payload named '{name}'\n  registered registries:\n{registries}\n  register one with: tebako add-registry <ref>"
                ),
            ))
        }
        1 => {
            let (reg_ref, payload) = found.pop().expect("len == 1 checked");
            plan_from_registry_entry(&reg_ref, &payload, version_req.as_deref(), host)
        }
        n => Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload '{name}' is listed by {n} registered registries (AmbiguousRegistries):\n{}\n  disambiguate with the full reference: tebako install tfs:<service>:owner/repo:version[#artifact]",
                found
                    .iter()
                    .map(|(r, _)| format!("    - {r}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )),
    }
}

/// Every registered registry carrying `name`, as (registry ref, payload)
/// pairs — the one search both the nickname form and dependency edges
/// run (spec 04 §2: the registered set is the whole namespace).
pub(crate) fn find_in_registries<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
    name: &str,
) -> Result<Vec<(String, RegistryPayload)>, TebakoError> {
    let cfg = config::load_config(home).map_err(map_shim)?;
    let mut found: Vec<(String, RegistryPayload)> = Vec::new();
    for reg_ref in &cfg.registries {
        let r = RegistryRef::parse(reg_ref).map_err(|e| {
            err(
                EX_TEBAKO_MANIFEST,
                format!("registered registry '{reg_ref}' is invalid: {e}"),
            )
        })?;
        let registry = fetcher.resolve_registry(&r).map_err(map_resolve)?;
        if let Some(payload) = registry.payload(name) {
            found.push((reg_ref.clone(), payload.clone()));
        }
    }
    Ok(found)
}

/// The registered-registries listing for the not-found errors.
fn registered_registries_listing(home: &Path) -> Result<String, TebakoError> {
    let cfg = config::load_config(home).map_err(map_shim)?;
    Ok(if cfg.registries.is_empty() {
        "    (none)".to_string()
    } else {
        cfg.registries
            .iter()
            .map(|r| format!("    - {r}"))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// The plan for one `requires:` edge (spec 03 §2.3): the edge's
/// constraint — not the registry default — selects the version, newest
/// satisfying. The consumer's identity heads the errors so the user
/// knows which payload declared the edge.
fn plan_from_dependency_edge<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
    consumer: &Manifest,
    kind: &str,
    name: &str,
    constraint: &tpkg::Constraint,
) -> Result<InstallPlan, TebakoError> {
    let mut found = find_in_registries(home, fetcher, name)?;
    let declares = || {
        format!(
            "{} {} requires {kind} {name} ({constraint})",
            consumer.name(),
            consumer.version()
        )
    };
    match found.len() {
        0 => {
            let registries = registered_registries_listing(home)?;
            Err(err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "{} but no registered registry carries it\n  registered registries:\n{registries}\n  register one with: tebako add-registry <ref>",
                    declares()
                ),
            ))
        }
        1 => {
            let (reg_ref, payload) = found.pop().expect("len == 1 checked");
            let eval = versions::from_validated(constraint);
            let version = payload
                .versions
                .iter()
                .map(|v| v.version.as_str())
                .filter(|v| eval.matches(v))
                .max_by(|a, b| versions::compare(a, b));
            match version {
                Some(v) => plan_from_registry_entry(&reg_ref, &payload, Some(v), None),
                None => Err(err(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "{} but registry {reg_ref} offers no satisfying version (available: {})",
                        declares(),
                        payload
                            .versions
                            .iter()
                            .map(|v| v.version.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )),
            }
        }
        n => Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "{} and it is listed by {n} registered registries (AmbiguousRegistries):\n{}\n  install the dependency explicitly first: tebako install {name}@<version>",
                declares(),
                found
                    .iter()
                    .map(|(r, _)| format!("    - {r}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )),
    }
}

pub(crate) fn plan_from_registry_entry(
    reg_ref: &str,
    payload: &RegistryPayload,
    version_req: Option<&str>,
    host: Option<Platform>,
) -> Result<InstallPlan, TebakoError> {
    let name = &payload.name;
    let available = || {
        payload
            .versions
            .iter()
            .map(|v| v.version.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let entry: &RegistryVersion = match version_req {
        Some(v) => payload.version(v).ok_or_else(|| {
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "registry {reg_ref} has no version '{v}' of '{name}' — available: {}",
                    available()
                ),
            )
        })?,
        None => payload.default_version().ok_or_else(|| {
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "registry {reg_ref} pins no default for '{name}' — pick a version: tebako install {name}@<version> (available: {})",
                    available()
                ),
            )
        })?,
    };

    let reference = Reference::parse(&entry.release.r#ref)
        .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;

    // The declarative host-triplet selection (spec 04 §2).
    let host = match host {
        Some(h) => h,
        None => host_platform()?,
    };
    let (reference, expected_sha256) = match entry.select(host) {
        Some(PlatformSelection::Universal) => (reference, None),
        Some(PlatformSelection::Selected { artifact, sha256 }) => {
            let mut reference = reference;
            if let Reference::Service { artifact: slot, .. } = &mut reference {
                *slot = Some(artifact.to_string());
            }
            (reference, Some(sha256.to_string()))
        }
        None => {
            let published = entry
                .published_triplets()
                .iter()
                .map(|p| p.as_triplet())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(err(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "{name} {} is not published for the host triplet {host} — published: {published}",
                    entry.version
                ),
            ));
        }
    };

    Ok(InstallPlan {
        name: name.clone(),
        version: entry.version.clone(),
        kind: payload.kind,
        reference,
        expected_sha256,
        signature: entry.signature.clone(),
        entrypoints: entry.entrypoints.clone(),
        runtime_requirement: entry
            .runtime_requirement
            .as_ref()
            .map(|r| (r.engine.clone(), r.constraint.clone(), r.abi.clone())),
        strict_identity: true,
    })
}

/// The host platform as a tpkg [`Platform`] (spec 03 §3: ONE type owns
/// the mapping — the CLI's host form is the release-asset name).
pub(crate) fn host_platform() -> Result<Platform, TebakoError> {
    let host = crate::options::host_platform()?;
    Platform::from_release_asset_name(&host).ok_or_else(|| {
        err(
            EX_TEBAKO_UNAVAILABLE,
            format!("the host platform '{host}' is not on the spec 03 §3 triplet axis"),
        )
    })
}

// ---- the shared tail: fetch → verify → cache → mirror → shims → deps --------

/// `chain` is the dependency path from the top-level install down to
/// this plan (spec 18 §5.6 S32): the closure walk pushes each payload's
/// name before walking its edges and pops after — a `requires:` edge
/// naming a payload already on the path is a dependency CYCLE, a named
/// error instead of the cache check's silent short-circuit.
fn finish_install<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
    plan: InstallPlan,
    shim_binary: Option<&Path>,
    chain: &mut Vec<String>,
) -> Result<InstallOutcome, TebakoError> {
    let cache = PayloadCache::with_root(home);
    let mut notes = Vec::new();

    // Cache hit ⇒ the trust anchor stands (spec 05 §4 — verified at
    // install, never re-verified per run); a miss fetches, verifies the
    // signature BEFORE anything enters the cache, then installs.
    let (entry, status, signer) = match cache.get(&plan.name, &plan.version).map_err(map_resolve)? {
        Some(entry) => (entry, InstallStatus::Hit, None),
        None => {
            if tebako_resolve::cache::offline() {
                return Err(map_resolve(ResolveError::Offline {
                    what: format!("payload {}@{}", plan.name, plan.version),
                }));
            }
            let fetched = fetcher.fetch(&plan.reference).map_err(map_resolve)?;
            let signer = verify_signature(home, fetcher, &fetched, &plan)?;
            let (entry, status) = cache
                .install(
                    &plan.name,
                    &plan.version,
                    plan.expected_sha256.as_deref(),
                    || Ok(fetched),
                )
                .map_err(map_resolve)?;
            (entry, status, signer)
        }
    };

    // The manifest mirror (the dispatcher-visible record, spec 07 §0):
    // the embedded manifest when the image carries one (tier 1,
    // authoritative), else the registry's tier-3 mirror fields.
    let mirror = build_mirror(&entry, &plan, &mut notes)?;
    let record: PayloadRecord = manifest::payload_record(home, &plan.name, &plan.version);
    mirror.save(&record.manifest_mirror).map_err(map_shim)?;

    // Zero-runtime entrypoints: materialize the native program + its
    // exec closure into the store tree NOW (a run never materializes —
    // install is the explicit verb, spec 07 §2).
    materialize_zero_runtime(home, &entry, &mirror)?;

    // Register the shims the payload PROVIDES declares ACTIVE (spec 07 §1
    // + spec 03 §2.2 `active`): app entrypoints ∪ toolkit executables, one
    // dispatchable view. The FULL declared set rides the manifest mirror;
    // an inactive-by-default command links on demand (`tebako shim
    // enable <name>`).
    let commands: Vec<String> = mirror
        .dispatchables()
        .iter()
        .filter(|e| e.active)
        .map(|e| e.name.clone())
        .collect();
    let shims = if commands.is_empty() {
        Vec::new()
    } else {
        let binary = resolve_shim_binary(shim_binary)?;
        let (shims, shim_notes) = manage::link_shims(home, &binary, &commands).map_err(map_shim)?;
        notes.extend(shim_notes);
        shims
    };

    journal(
        home,
        &format!(
            "event=payload-installed name={} version={} sha256={} origin={}",
            plan.name,
            plan.version,
            entry.sha256,
            entry.origin.as_deref().unwrap_or("")
        ),
    );

    // The dependency closure (spec 03 §2.3): the mirror's toolkit/data
    // edges resolve to cached-or-registry installs, recursively. Every
    // payload is cached BEFORE its own edges walk, so a re-encountered
    // name short-circuits on the cache check — the chain guard (S32)
    // turns a true cycle into the named error first.
    chain.push(plan.name.clone());
    let closure = install_dependency_closure(home, fetcher, &mirror, shim_binary, chain);
    chain.pop();
    closure?;

    Ok(InstallOutcome {
        name: plan.name,
        version: plan.version,
        status,
        path: entry.path,
        sha256: entry.sha256,
        commands,
        shims,
        signer,
        notes,
    })
}

/// Install the dependency closure a mirror declares (spec 03 §2.3):
/// every toolkit/data `requires:` edge resolves to the newest CACHED
/// satisfying version (a no-op — the pin the user installed stands) or,
/// failing that, the newest registry version satisfying the edge's
/// constraint, installed through the same fetch → verify → cache → walk
/// tail. `kind: language` edges are the runtime axis — the dispatcher
/// resolves them at run time, never here. An edge naming a payload
/// already on `chain` is a requires CYCLE (spec 18 §5.6 S32) — a named
/// error, never the cache check's silent short-circuit.
fn install_dependency_closure<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
    mirror: &Manifest,
    shim_binary: Option<&Path>,
    chain: &mut Vec<String>,
) -> Result<(), TebakoError> {
    for req in mirror.requires() {
        // spec 30 §1/§3: a spawned-runtime edge installs the RUNTIME into
        // the store (never a payload in the closure walk), verifies the
        // expose list against the runtime's own spawn surface, and
        // registers one shim per exposed name.
        if let tpkg::Requirement::Runtime {
            engine,
            implementation,
            constraint,
            expose,
        } = req
        {
            install_runtime_edge(
                home,
                engine,
                implementation.as_deref(),
                constraint,
                expose,
                shim_binary,
            )?;
            continue;
        }
        let (kind, name, constraint) = match req {
            tpkg::Requirement::Language { .. } => continue,
            tpkg::Requirement::Runtime { .. } => {
                unreachable!("runtime edges install above the tuple match")
            }
            tpkg::Requirement::Toolkit {
                name, constraint, ..
            } => ("toolkit", name, constraint),
            tpkg::Requirement::Data {
                name, constraint, ..
            } => ("data", name, constraint),
        };
        if chain.iter().any(|n| n == name) {
            let start = chain
                .iter()
                .position(|n| n == name)
                .expect("membership checked");
            let mut cycle: Vec<String> = chain[start..].to_vec();
            cycle.push(name.clone());
            return Err(err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "dependency cycle: {} (spec 18 §5.6 S32) — break the cycle in the payloads' requires: declarations",
                    cycle.join(" → ")
                ),
            ));
        }
        let eval = versions::from_validated(constraint);
        let installed = tebako_shim::resolve::installed_versions(home, name).map_err(map_shim)?;
        if installed.iter().any(|v| eval.matches(v)) {
            continue;
        }
        let plan = plan_from_dependency_edge(home, fetcher, mirror, kind, name, constraint)?;
        finish_install(home, fetcher, plan, shim_binary, chain)?;
    }
    Ok(())
}

/// Install one spawned-runtime edge (spec 30 §1/§3): resolve + download
/// the RUNTIME into the store's runtimes/ area (never a payload in the
/// closure walk), then — when the edge exposes names — verify the list
/// against the runtime's own spawn surface (the embedded manifest's
/// entrypoints; the install-time cross-check class of spec 26 §5, since
/// the shim layer deliberately cannot read images) and register one shim
/// per exposed name (spec 07's argv0 model; the runtime entrypoint's
/// `active` flag applies).
fn install_runtime_edge(
    home: &Path,
    engine: &str,
    implementation: Option<&str>,
    constraint: &tpkg::Constraint,
    expose: &[String],
    shim_binary: Option<&Path>,
) -> Result<(), TebakoError> {
    let ctx = tebako_shim::Ctx {
        home: home.to_path_buf(),
        cwd: std::env::current_dir().map_err(|e| err(EX_TEBAKO_IO, e.to_string()))?,
        env: std::env::vars().collect(),
    };
    // Pre-staging the runtime IS install's job (the dispatch would
    // download it otherwise); expose only drives the cross-check + shims.
    let rt =
        tebako_shim::runtime::resolve_runtime_edge(engine, implementation, constraint, true, &ctx)
            .map_err(map_shim)?;
    journal(
        home,
        &format!(
            "event=runtime-edge-resolved engine={engine} lang_version={} tebako_version={}",
            rt.lang_version, rt.tebako_version
        ),
    );
    if expose.is_empty() {
        return Ok(());
    }
    let image = rt
        .image
        .as_ref()
        .expect("edge resolution post-asserts the env image");
    let text = image_manifest::read_embedded_manifest(image)?.ok_or_else(|| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the {engine} runtime image {} carries no embedded manifest — the expose list cannot be verified",
                image.display()
            ),
        )
    })?;
    let manifest = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the embedded manifest of {} does not parse: {e}",
                image.display()
            ),
        )
    })?;
    let tpkg::Provides::Runtime(rt_provides) = &manifest.provides else {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "{} is not a runtime payload — a spawn edge can only expose a runtime's entrypoints",
                image.display()
            ),
        ));
    };
    let mut names = Vec::new();
    for name in expose {
        let Some(ep) = rt_provides.entrypoints.iter().find(|e| &e.name == name) else {
            return Err(err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "expose: the {engine} runtime {} declares no entrypoint \"{name}\" (declared: {}) — fix the payload's expose list or the runtime's spawn surface",
                    rt.lang_version,
                    rt_provides
                        .entrypoints
                        .iter()
                        .map(|e| e.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        };
        if ep.is_active() {
            names.push(name.clone());
        }
    }
    if names.is_empty() {
        return Ok(());
    }
    let binary = resolve_shim_binary(shim_binary)?;
    let (_links, shim_notes) = manage::link_shims(home, &binary, &names).map_err(map_shim)?;
    for note in shim_notes {
        journal(
            home,
            &format!("event=spawn-shim-note engine={engine} note={note}"),
        );
    }
    Ok(())
}

/// Materialize every zero-runtime entrypoint (no `runtime_requirement`)
/// into the payload's store tree (`<version>.tree/<in-image path>`):
/// the native program plus its Mach-O/ELF dependency closure, so the
/// dispatcher execs it without linking tfs (spec 07 §2 — install is
/// the materialization verb; a run never extracts).
///
/// A payload declaring a home layout (`annotations.java_home` — the
/// root IS a runtime home: a JRE's bin/java probes lib/jvm.cfg relative
/// to its own real path) materializes WHOLE: the closure walk only
/// ever sees linked binaries, never the home's data files (the openjdk
/// jvm.cfg miss, dogfood-found 2026-08-10).
fn materialize_zero_runtime(
    home: &Path,
    entry: &tebako_resolve::CacheEntry,
    mirror: &Manifest,
) -> Result<(), TebakoError> {
    let zero: Vec<String> = mirror
        .dispatchables()
        .iter()
        .filter(|e| e.runtime_requirement.is_none())
        .map(|e| e.path.clone())
        .collect();
    if zero.is_empty() {
        return Ok(());
    }
    let whole_tree = mirror.home_layout().is_some();
    let record: PayloadRecord = manifest::payload_record(home, mirror.name(), mirror.version());
    image_manifest::with_image_mounted(&entry.path, || {
        if whole_tree {
            return tfs::context::context()
                .write()
                .unwrap()
                .extract_all(&record.tree)
                .map(|_| ())
                .map_err(|e| {
                    err(
                        EX_TEBAKO_UNAVAILABLE,
                        format!(
                            "cannot materialize the home-layout payload {}: {}",
                            entry.path.display(),
                            String::from_utf8_lossy(tfs::errno::strerror(e)).into_owned()
                        ),
                    )
                });
        }
        for path in &zero {
            tfs::context::context()
                .write()
                .unwrap()
                .extract_exec_closure(path, &record.tree)
                .map_err(|e| {
                    err(
                        EX_TEBAKO_UNAVAILABLE,
                        format!(
                            "cannot materialize the zero-runtime entrypoint {path} of {}: {}",
                            entry.path.display(),
                            String::from_utf8_lossy(tfs::errno::strerror(e)).into_owned()
                        ),
                    )
                })?;
        }
        Ok(())
    })
}

/// The manifest mirror for the payload record (spec 03 §4 tier 3 — the
/// unified [`tpkg::PayloadManifest`], item 40). The embedded manifest is
/// authoritative and mirrored as parsed; the name form cross-checks it
/// against the registry (the registry is the trust source — a mismatch
/// means it lied). Without an embedded manifest the mirror is synthesized
/// from the registry's tier-3 fields (ref form: the payload name) with
/// the `/<command>` entry-path convention — LOUD, never silent.
fn build_mirror(
    entry: &tebako_resolve::CacheEntry,
    plan: &InstallPlan,
    notes: &mut Vec<String>,
) -> Result<Manifest, TebakoError> {
    if let Some(text) = image_manifest::read_embedded_manifest(&entry.path)? {
        let embedded = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "the embedded manifest of {} does not parse: {e}",
                    entry.path.display()
                ),
            )
        })?;
        if embedded.identity.name != plan.name || embedded.identity.version != plan.version {
            let msg = format!(
                "the embedded manifest declares {} {} but the install named {} {}",
                embedded.identity.name, embedded.identity.version, plan.name, plan.version
            );
            if plan.strict_identity {
                return Err(err(
                    EX_TEBAKO_MANIFEST,
                    format!("{msg} — the registry entry is inconsistent with the payload it names"),
                ));
            }
            notes.push(msg);
        }
        return Ok(Manifest::from_payload_manifest(embedded));
    }
    notes.push(
        "the image carries no embedded manifest (/__tpkg__/manifest.yaml); \
         the mirror is synthesized with the /<command> entry-path convention"
            .to_string(),
    );
    Ok(Manifest::from_payload_manifest(synthesize_manifest(
        entry, plan,
    )?))
}

/// The synthesized mirror (no embedded manifest): a minimal valid
/// [`tpkg::PayloadManifest`] from the plan's tier-3 fields. `created` is
/// the install time (the model never interprets it); `tree_hash` is a
/// placeholder — the fixed-point rule (spec 03 §7) keeps real digests one
/// tier out, and `blob_sha256` carries the payload's verified digest.
fn synthesize_manifest(
    entry: &tebako_resolve::CacheEntry,
    plan: &InstallPlan,
) -> Result<tpkg::PayloadManifest, TebakoError> {
    let requirement = match &plan.runtime_requirement {
        Some((engine, constraint, abi)) => Some(tpkg::RuntimeRequirement {
            engine: engine.clone(),
            constraint: tpkg::Constraint::new(constraint).map_err(|e| {
                err(
                    EX_TEBAKO_MANIFEST,
                    format!("the registry's runtime_requirement constraint is invalid: {e}"),
                )
            })?,
            abi: abi.clone(),
        }),
        None => None,
    };
    let provides = match plan.kind {
        tpkg::PayloadKind::App => {
            let names = if plan.entrypoints.is_empty() {
                vec![plan.name.clone()]
            } else {
                plan.entrypoints.clone()
            };
            tpkg::Provides::App(tpkg::AppProvides {
                entrypoints: names
                    .into_iter()
                    .map(|n| tpkg::Entrypoint {
                        path: format!("/{n}"),
                        name: n,
                        args_default: Vec::new(),
                        runtime_requirement: requirement.clone(),
                        active: None,
                    })
                    .collect(),
                platforms: tpkg::Platforms::Universal,
                capabilities: tpkg::Capabilities {
                    exec: true,
                    read: true,
                    runtime: None,
                    host: None,
                },
            })
        }
        // Non-app kinds declare no entrypoints (never a shim); the
        // consumer declares the mount (MOUNT RULE), the suggested "/" is
        // the bare-image convention (spec 07 §0).
        tpkg::PayloadKind::Data => tpkg::Provides::Data(tpkg::DataProvides {
            mount_semantics: tpkg::MountSemantics {
                suggested: "/".to_string(),
            },
            consumers: Vec::new(),
            capabilities: tpkg::Capabilities {
                exec: false,
                read: true,
                runtime: None,
                host: None,
            },
        }),
        other => {
            return Err(err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "cannot synthesize a manifest mirror for a {other:?} payload without an embedded manifest — press the payload with an embedded /__tpkg__/manifest.yaml"
                ),
            ))
        }
    };
    Ok(tpkg::PayloadManifest {
        identity: tpkg::Identity {
            schema_version: tpkg::PAYLOAD_SCHEMA_VERSION,
            kind: plan.kind,
            name: plan.name.clone(),
            version: plan.version.clone(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            created: rfc3339_utc(now_unix()),
            source: None,
            sbom: None,
            digest: tpkg::Digest {
                tree_hash: format!("sha256:{}", "0".repeat(64)),
                blob_sha256: entry.sha256.clone(),
            },
            signing: tpkg::Signing {
                state: tpkg::SigningState::Unsigned,
                keyid: None,
                mechanism: None,
            },
            encryption: tpkg::Encryption {
                state: tpkg::EncryptionState::None,
                parts: Vec::new(),
            },
            annotations: Default::default(),
        },
        provides,
        requires: Vec::new(),
        // A synthesized mirror declares nothing to materialize — only an
        // embedded manifest can (the store layout: the embedded manifest
        // wins; the mirror is synthesized LOUDLY).
        materialize: Vec::new(),
        // …and no library aliases (spec 03 §2.5 — the same mirror rule:
        // declarations live in the embedded manifest only).
        library_aliases: Vec::new(),
        // …and no checks (spec 26 §1 — the same mirror rule).
        checks: Default::default(),
    })
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Unix seconds → "YYYY-MM-DDTHH:MM:SSZ" (RFC 3339, UTC). The manifest's
/// `created` is a string the model never interprets; no time crate rides
/// along for one rendering.
pub(crate) fn rfc3339_utc(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, min, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil-from-days (Howard Hinnant's algorithm, days-from-civil inverse).
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// The dispatcher binary the shims link to: the explicit override >
/// $TEBAKO_SHIM_BINARY > the `tebako-shim` sibling of the tebako binary
/// (the same sibling convention as tebako-bootstrap).
fn resolve_shim_binary(explicit: Option<&Path>) -> Result<PathBuf, TebakoError> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    if let Ok(env) = std::env::var("TEBAKO_SHIM_BINARY") {
        if !env.is_empty() {
            return Ok(PathBuf::from(env));
        }
    }
    let exe = std::env::current_exe().map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot determine the tebako binary: {e}"),
        )
    })?;
    let name = if cfg!(windows) {
        "tebako-shim.exe"
    } else {
        "tebako-shim"
    };
    Ok(exe
        .parent()
        .map(|d| d.join(name))
        .unwrap_or_else(|| PathBuf::from(name)))
}

// ---------------------------------------------------------------------
// signature verification (spec 09 — strict when the entry is signed,
// v1-legacy warn when it is not)
// ---------------------------------------------------------------------

/// Verify the fetched payload against the registry entry's signature.
/// Returns the verified signer keyid. `None` signature → the spec 09 §3
/// v1-legacy rule (loud warn + audit line; `TEBAKO_REQUIRE_SIGNED=1`
/// hard-fails).
fn verify_signature<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
    fetched: &FetchedPayload,
    plan: &InstallPlan,
) -> Result<Option<String>, TebakoError> {
    let Some(sig) = &plan.signature else {
        if require_signed() {
            return Err(err(
                EX_TEBAKO_SIGNATURE,
                format!(
                    "{} is unsigned and TEBAKO_REQUIRE_SIGNED=1 is set — refusing to install",
                    fetched.origin
                ),
            ));
        }
        eprintln!(
            "tebako: WARNING: {} is unsigned\n  — accepted for compatibility (the registry entry carries no signature); ask the publisher to sign the release",
            fetched.origin
        );
        journal(
            home,
            &format!("event=legacy-unsigned-accepted origin={}", fetched.origin),
        );
        return Ok(None);
    };

    let asc_ref = signature_reference(sig, &plan.reference)?;
    let asc = fetcher.fetch(&asc_ref).map_err(map_resolve)?;
    let keyring =
        tebako_signer::trusted_keyring_bytes(home).map_err(|e| err(EX_TEBAKO_IO, e.to_string()))?;
    let outcome = tebako_signer::verify_detached_full(&keyring, &fetched.bytes, &asc.bytes)
        .map_err(|e| err(EX_TEBAKO_SIGNATURE, e.to_string()))?;
    match outcome {
        tebako_signer::VerifyOutcome::Trusted(keyid) => {
            if !keyid.eq_ignore_ascii_case(&sig.keyid) {
                return Err(err(
                    EX_TEBAKO_SIGNATURE,
                    format!(
                        "{} is signed by {keyid} but the registry pins {} — refusing to install; nothing was cached",
                        fetched.origin, sig.keyid
                    ),
                ));
            }
            let keyid = keyid.to_ascii_lowercase();
            journal(
                home,
                &format!(
                    "event=payload-signature-trusted origin={} signer={keyid}",
                    fetched.origin
                ),
            );
            Ok(Some(keyid))
        }
        tebako_signer::VerifyOutcome::Untrusted(keyid) => Err(err(
            EX_TEBAKO_TRUST,
            format!(
                "{} is signed by {keyid}, which is not in the trusted keyring — register the publisher's key (~/.tebako/keyring/trusted.pgp), then retry; nothing was cached",
                fetched.origin
            ),
        )),
        tebako_signer::VerifyOutcome::Invalid(keyid) => Err(err(
            EX_TEBAKO_SIGNATURE,
            format!(
                "signature verification failed for {} (signer {}) — the payload or its signature is corrupt; nothing was cached",
                fetched.origin,
                keyid.unwrap_or_else(|| "unknown".to_string())
            ),
        )),
    }
}

/// The `.asc` of a signature pin: a full reference, or an asset name
/// within the same release (the common case — the `.asc` rides the
/// release next to the artifact it signs).
///
/// One detached signature covers exactly ONE artifact's bytes, so a
/// release carrying N artifacts carries N ascs by convention:
/// `<artifact>.asc`. When the install already selected an artifact
/// (per-triplet registry entries, explicit `#artifact` forms), the asc
/// follows the SELECTED artifact — `sig.asc` is read as the exact name
/// only when no selection happened (universal payloads), which is also
/// the shape the registry's spec example documents.
fn signature_reference(sig: &SignaturePin, release: &Reference) -> Result<Reference, TebakoError> {
    if looks_like_reference(&sig.asc) {
        return Reference::parse(&sig.asc).map_err(|e| {
            err(
                EX_TEBAKO_MANIFEST,
                format!("signature.asc does not parse: {e}"),
            )
        });
    }
    let mut reference = release.clone();
    match &mut reference {
        Reference::Service {
            artifact, sha256, ..
        } => {
            // The payload's own pin must not leak onto the signature asset.
            *sha256 = None;
            *artifact = Some(match artifact {
                Some(selected) => format!("{selected}.asc"),
                None => sig.asc.clone(),
            });
            Ok(reference)
        }
        _ => Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "signature.asc '{}' is an asset name but the release is not a service release — name a full reference instead",
                sig.asc
            ),
        )),
    }
}

/// `TEBAKO_REQUIRE_SIGNED` truthiness (mirrors the bootstrap's rule:
/// set and not "0").
fn require_signed() -> bool {
    std::env::var("TEBAKO_REQUIRE_SIGNED").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Append one line to the audit journal (<home>/journal.log), mirroring
/// the bootstrap's convention. Best-effort: journaling never fails the
/// install.
fn journal(home: &Path, line: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(home);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("journal.log"))
    {
        let _ = writeln!(f, "{now} {line}");
    }
}

// ---------------------------------------------------------------------
// uninstall
// ---------------------------------------------------------------------

/// What an uninstall removed.
#[derive(Debug)]
pub struct UninstallOutcome {
    pub name: String,
    pub versions: Vec<String>,
    pub shims_removed: Vec<PathBuf>,
}

/// `tebako uninstall <name>`: remove the payload's shims and its cache
/// entry (`payloads/<name>/`). The trust-anchor digests are journaled
/// BEFORE removal — the audit line is where the markers survive.
pub fn uninstall(home: &Path, name: &str) -> Result<UninstallOutcome, TebakoError> {
    manifest::check_path_component("payload name", name).map_err(map_shim)?;
    let versions = tebako_shim::resolve::installed_versions(home, name).map_err(map_shim)?;
    if versions.is_empty() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload '{name}' is not installed ({} has no versions)",
                home.join("payloads").join(name).display()
            ),
        ));
    }

    // Every command the payload provides, across all installed versions;
    // the payload name itself covers the simple-app convention.
    let mut commands = BTreeSet::from([name.to_string()]);
    let mut anchors: Vec<(String, String)> = Vec::new();
    for v in &versions {
        let record = manifest::payload_record(home, name, v);
        if let Ok(m) = Manifest::load(&record.manifest_mirror) {
            for e in m.dispatchables() {
                commands.insert(e.name.clone());
            }
        }
        if let Ok(text) = std::fs::read_to_string(&record.sha_marker) {
            if let Some(sha) = text.split_whitespace().next() {
                anchors.push((v.clone(), sha.to_string()));
            }
        }
    }

    let commands: Vec<String> = commands.into_iter().collect();
    let shims_removed = manage::unlink_shims(home, &commands).map_err(map_shim)?;

    let dir = home.join("payloads").join(name);
    std::fs::remove_dir_all(&dir).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot remove {}: {e}", dir.display()),
        )
    })?;

    for (v, sha) in &anchors {
        journal(
            home,
            &format!("event=payload-uninstalled name={name} version={v} sha256={sha}"),
        );
    }
    Ok(UninstallOutcome {
        name: name.to_string(),
        versions,
        shims_removed,
    })
}
