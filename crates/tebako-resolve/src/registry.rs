//! The registry (spec 04 §2): the developer-hosted `tpkg-registry.yaml`
//! model and its resolution. A registry is ANY git host repo carrying the
//! file; the git host's releases ARE the storage — zero central
//! infrastructure.
//!
//! The model MIRRORS only resolution-relevant fields (spec 03 §4 tier 3):
//! the dispatcher resolves — and selects the host-triplet artifact
//! DECLARATIVELY (`platforms[host].artifact` or `universal`) — without
//! downloading every payload. Reading is two-step, mirroring tpkg's
//! manifest discipline: [`Registry::from_yaml`] parses (serde) and
//! validates (semantics); unknown keys are tolerated for forward
//! compatibility.
//!
//! Registry resolution (locked — exactly one location per form, no
//! fallback chain; spec 37 §4 adds the explicit-host forms and the HTTPS
//! registry location, amending spec 04 §2's "no other locations" by that
//! one form):
//!
//! ```text
//! tfs:<svc>:owner/repo                          → /tpkg-registry.yaml on the
//!                                                 DEFAULT branch (contents API)
//! tfs+<svc>://host/owner/repo                   → the same at an explicit
//!                                                 host (GHE/self-hosted)
//! tfs:<svc>:owner/repo:version#tpkg-registry.yaml → release artifact
//! tfs+<svc>://host/owner/repo:version#tpkg-registry.yaml
//!                                               → release artifact, explicit host
//! tfs+git://host/owner/repo.git[@ref]#path      → git blob
//! tfs+https://host/path/tpkg-registry.yaml      → the registry file itself
//!                                                 over plain HTTPS (static
//!                                                 server, S3/CDN, generic repo)
//! file:///abs/path/tpkg-registry.yaml           → local mirror (tests,
//!                                                 air-gapped sites)
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tpkg::{PayloadKind, Platform};

use crate::error::{ReferenceError, RegistryError, ResolveError};
use crate::fetch::Fetcher;
use crate::reference::{check_component, Reference, Service};
use crate::transport::Transport;

/// The only `schema_version` this implementation reads and writes.
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------
// The model
// ---------------------------------------------------------------------

/// A `tpkg-registry.yaml` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    /// The schema MAJOR this document declares (spec 18 C8: missing =
    /// era 1, refused by name; newer = the upgrade refusal). Optional at
    /// the serde level so the reader can name the era-1 case — the
    /// validator refuses `None`; writers always set
    /// [`REGISTRY_SCHEMA_VERSION`].
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payloads: Vec<RegistryPayload>,
}

/// One listed payload: name, kind, the version entries, and the
/// registry-side default the dispatcher's version chain ends on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryPayload {
    pub name: String,
    pub kind: PayloadKind,
    /// The engine a `kind: runtime` entry serves (spec 30 §1's
    /// edge-discovery key, schema MINOR 1): edges resolve runtimes by
    /// (engine, implementation?, constraint). Absent on a runtime entry
    /// = pre-discovery legacy — resolvable by name, invisible to edges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// spec 28 §8's implementation axis (e.g. temurin): an edge naming
    /// an implementation matches only entries carrying the same value.
    /// Wins over the version-level compat spelling (MINOR 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    pub versions: Vec<RegistryVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// One version entry of a payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryVersion {
    pub version: String,
    /// The pre-MINOR-1 spelling of the payload-level `implementation`
    /// axis (MINOR 2's compat read) — never authored anew; the
    /// payload-level key wins when both are present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    pub platforms: RegistryPlatforms,
    /// The payload's release home — a spec 04 §1 reference (any class;
    /// per-triplet `platforms` require a service release, since artifact
    /// names only exist there).
    pub release: ReleaseRef,
    /// Opt-in OpenPGP signature of the artifact (spec 09): the signing
    /// key's PRIMARY keyid (16 lowercase hex — the identity, not the
    /// instrument; a signature issuing from a signing subkey resolves to
    /// its primary at verification) and the detached `.asc` — an asset
    /// name within the same release, or a full reference. One signature
    /// covers exactly one artifact, so per-triplet releases carry one asc
    /// per artifact by convention (`<artifact>.asc`) and the installer
    /// verifies the SELECTED artifact against its own asc; `asc` names
    /// the exact asset only for universal payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignaturePin>,
    /// The runtime the payload's entrypoints need (spec 03 §2.2);
    /// mirrored for dispatch-time runtime resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_requirement: Option<RegistryRuntimeRequirement>,
    /// The command names the payload PROVIDES (spec 03 §4 tier 3); each
    /// becomes a registered shim at install.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrypoints: Vec<String>,
    /// spec 04 §2's withdrawal axis (roadmap 85): `status: withdrawn`
    /// marks the row YANKED (release assets are immutable — withdrawal is
    /// the only remedy for a bad published artifact). An `Option<String>`,
    /// not an enum, for forward tolerance: unknown values parse and are
    /// inert; [`RegistryVersion::is_withdrawn`] names the one value with
    /// refusal semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// The platform axis (spec 04 §2): EITHER the bare string `universal`
/// (pure-language payloads — one artifact, selected by the single-`.tfs`
/// rule of spec 04 §1) OR a triplet → artifact map (native-extension
/// payloads — the declarative host selection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryPlatforms {
    Universal,
    PerTriplet(BTreeMap<Platform, PlatformArtifact>),
}

/// The per-triplet artifact entry: the asset name within the release and
/// its sha256 pin (the registry-supplied trust anchor, spec 05 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformArtifact {
    pub artifact: String,
    pub sha256: String,
}

/// `release: {ref: …}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseRef {
    pub r#ref: String,
}

/// `signature: {keyid: …, asc: …}` (opt-in).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePin {
    pub keyid: String,
    pub asc: String,
}

/// `runtime_requirement: {engine: …, constraint: …, implementation?,
/// abi?}` (optional mirror). `abi` is the runtime's platform string a
/// native-extension payload was built against (ruby:
/// `Gem::Platform.local.to_s`) — the resolution checks BOTH the version
/// line and the platform line (spec 05 §5); absent means pure-language
/// (the version line alone). `implementation` (spec 28 §8) mirrors the
/// L1 entry's implementation axis — REQUIRED in the mirror when `abi`
/// is present (an abi is per-implementation by construction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryRuntimeRequirement {
    pub engine: String,
    pub constraint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi: Option<String>,
}

impl Serialize for RegistryPlatforms {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            RegistryPlatforms::Universal => s.serialize_str("universal"),
            RegistryPlatforms::PerTriplet(m) => m.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for RegistryPlatforms {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<RegistryPlatforms, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Str(String),
            Map(BTreeMap<Platform, PlatformArtifact>),
        }
        match Repr::deserialize(d)? {
            Repr::Str(s) if s == "universal" => Ok(RegistryPlatforms::Universal),
            Repr::Str(s) => Err(serde::de::Error::custom(format_args!(
                "platforms must be \"universal\" or a triplet map, got {s:?}"
            ))),
            Repr::Map(m) => Ok(RegistryPlatforms::PerTriplet(m)),
        }
    }
}

// ---------------------------------------------------------------------
// Parse + validate (the tpkg manifest discipline)
// ---------------------------------------------------------------------

fn invalid_entry(reason: impl Into<String>) -> RegistryError {
    RegistryError::Invalid {
        reason: reason.into(),
    }
}

/// Names and versions become cache path components; entrypoints become
/// shim file names. The rule is the payload cache's key rule.
fn check_path_safe(what: &str, value: &str) -> Result<(), RegistryError> {
    let bad = value.is_empty()
        || value == "."
        || value == ".."
        || value
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control() || c.is_whitespace());
    if bad {
        return Err(invalid_entry(format!(
            "{what} '{value}' must be a single non-empty path component"
        )));
    }
    Ok(())
}

fn check_sha256(what: &str, value: &str) -> Result<(), RegistryError> {
    let ok = value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !ok {
        return Err(invalid_entry(format!(
            "{what} must be 64 lowercase hex characters, got '{value}'"
        )));
    }
    Ok(())
}

impl Registry {
    /// Parse and validate a `tpkg-registry.yaml` document.
    pub fn from_yaml(text: &str) -> Result<Registry, RegistryError> {
        let registry: Registry = serde_yml::from_str(text).map_err(|e| RegistryError::Yaml {
            reason: e.to_string(),
        })?;
        registry.validate()?;
        Ok(registry)
    }

    /// Serialize back to YAML (round-trip identity with [`Registry::from_yaml`]).
    pub fn to_yaml(&self) -> Result<String, RegistryError> {
        serde_yml::to_string(self).map_err(|e| RegistryError::Yaml {
            reason: e.to_string(),
        })
    }

    /// The payload named `name`, if the registry lists it.
    pub fn payload(&self, name: &str) -> Option<&RegistryPayload> {
        self.payloads.iter().find(|p| p.name == name)
    }

    /// The `kind: runtime` entries serving `engine` (spec 30 §1's edge
    /// discovery): an entry without the payload-level `engine` key is
    /// pre-discovery legacy — resolvable by name, invisible here.
    /// `Some(w)` narrows to the implementation axis (spec 28 §8 — an
    /// edge naming an implementation matches only entries carrying the
    /// same value, MINOR 2's compat read included); `None` lists every
    /// entry of the engine.
    pub fn runtime_entries(
        &self,
        engine: &str,
        implementation: Option<&str>,
    ) -> Vec<&RegistryPayload> {
        self.payloads
            .iter()
            .filter(|p| p.kind == PayloadKind::Runtime)
            .filter(|p| p.engine() == Some(engine))
            .filter(|p| match implementation {
                Some(w) => p.implementation() == Some(w),
                None => true,
            })
            .collect()
    }

    fn validate(&self) -> Result<(), RegistryError> {
        match self.schema_version {
            // spec 18 C8/S46: no `schema_version` is an era-1 document —
            // refused by name, never a silent default.
            None => return Err(RegistryError::PreEra),
            // S45: a newer MAJOR is the upgrade refusal.
            Some(v) if v > REGISTRY_SCHEMA_VERSION => {
                return Err(invalid_entry(format!(
                    "schema_version {v} is newer than this tebako speaks ({REGISTRY_SCHEMA_VERSION}) — upgrade tebako"
                )));
            }
            Some(v) if v < REGISTRY_SCHEMA_VERSION => {
                return Err(invalid_entry(format!(
                    "schema_version {v} is not a valid registry schema ({REGISTRY_SCHEMA_VERSION} expected)"
                )));
            }
            Some(_) => {}
        }
        for payload in &self.payloads {
            payload.validate()?;
        }
        let mut names: Vec<&str> = self.payloads.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(invalid_entry("duplicate payload name"));
        }
        Ok(())
    }
}

impl RegistryPayload {
    fn validate(&self) -> Result<(), RegistryError> {
        check_path_safe("payload name", &self.name)?;
        if let Some(engine) = &self.engine {
            if engine.is_empty() {
                return Err(invalid_entry(format!(
                    "payload '{}' engine must not be empty",
                    self.name
                )));
            }
        }
        if let Some(implementation) = &self.implementation {
            if implementation.is_empty() {
                return Err(invalid_entry(format!(
                    "payload '{}' implementation must not be empty",
                    self.name
                )));
            }
        }
        if self.versions.is_empty() {
            return Err(invalid_entry(format!(
                "payload '{}' lists no versions",
                self.name
            )));
        }
        for v in &self.versions {
            v.validate(self)?;
        }
        let mut versions: Vec<&str> = self.versions.iter().map(|v| v.version.as_str()).collect();
        versions.sort();
        if versions.windows(2).any(|w| w[0] == w[1]) {
            return Err(invalid_entry(format!(
                "payload '{}' lists a duplicate version",
                self.name
            )));
        }
        if let Some(default) = &self.default {
            if self.version(default).is_none() {
                return Err(invalid_entry(format!(
                    "payload '{}' default '{default}' names no listed version",
                    self.name
                )));
            }
        }
        Ok(())
    }

    /// The entry for version `version`.
    pub fn version(&self, version: &str) -> Option<&RegistryVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    /// The registry-side default entry (spec 07 §2.1, last chain link).
    pub fn default_version(&self) -> Option<&RegistryVersion> {
        self.default.as_deref().and_then(|d| self.version(d))
    }

    /// The engine axis (the payload-level key; spec 30 §1).
    pub fn engine(&self) -> Option<&str> {
        self.engine.as_deref()
    }

    /// The implementation axis (spec 28 §8): the payload-level key wins;
    /// absent there, MINOR 2's compat read — the version-level spelling
    /// when every version carrying one AGREES (a disagreement is no
    /// axis, never a guess).
    pub fn implementation(&self) -> Option<&str> {
        if let Some(implementation) = &self.implementation {
            return Some(implementation);
        }
        let mut carried = self
            .versions
            .iter()
            .filter_map(|v| v.implementation.as_deref());
        let first = carried.next()?;
        carried.all(|i| i == first).then_some(first)
    }
}

impl RegistryVersion {
    fn validate(&self, payload: &RegistryPayload) -> Result<(), RegistryError> {
        check_path_safe("version", &self.version)?;
        if let Some(implementation) = &self.implementation {
            if implementation.is_empty() {
                return Err(invalid_entry(format!(
                    "payload '{}' {} implementation must not be empty",
                    payload.name, self.version
                )));
            }
        }
        let release = Reference::parse(&self.release.r#ref).map_err(|e| {
            invalid_entry(format!(
                "payload '{}' {} release.ref does not parse: {e}",
                payload.name, self.version
            ))
        })?;
        if let Reference::Service {
            artifact: Some(_), ..
        } = release
        {
            return Err(invalid_entry(format!(
                "payload '{}' {} release.ref carries an #artifact — artifact selection belongs to the platforms map",
                payload.name, self.version
            )));
        }
        match (&self.platforms, &release) {
            (RegistryPlatforms::PerTriplet(map), Reference::Service { sha256, .. }) => {
                if sha256.is_some() {
                    return Err(invalid_entry(format!(
                        "payload '{}' {} release.ref carries a ?sha256= pin but platforms is per-triplet — the pins live in platforms[<triplet>].sha256",
                        payload.name, self.version
                    )));
                }
                if map.is_empty() {
                    return Err(invalid_entry(format!(
                        "payload '{}' {} platforms map is empty (use \"universal\")",
                        payload.name, self.version
                    )));
                }
                for (platform, entry) in map {
                    if entry.artifact.is_empty() {
                        return Err(invalid_entry(format!(
                            "payload '{}' {} platforms[{platform}].artifact must not be empty",
                            payload.name, self.version
                        )));
                    }
                    check_sha256(
                        &format!(
                            "payload '{}' {} platforms[{platform}].sha256",
                            payload.name, self.version
                        ),
                        &entry.sha256,
                    )?;
                }
            }
            (RegistryPlatforms::PerTriplet(_), _) => {
                return Err(invalid_entry(format!(
                    "payload '{}' {} has per-triplet platforms but release.ref is not a service release — artifact names only exist on tfs:<service>: releases",
                    payload.name, self.version
                )));
            }
            (RegistryPlatforms::Universal, _) => {}
        }
        if let Some(sig) = &self.signature {
            let keyid_ok = sig.keyid.len() == 16
                && sig
                    .keyid
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if !keyid_ok {
                return Err(invalid_entry(format!(
                    "payload '{}' {} signature.keyid must be 16 lowercase hex (the low 64 bits of the OpenPGP fingerprint)",
                    payload.name, self.version
                )));
            }
            if sig.asc.is_empty() {
                return Err(invalid_entry(format!(
                    "payload '{}' {} signature.asc must not be empty",
                    payload.name, self.version
                )));
            }
        }
        if let Some(req) = &self.runtime_requirement {
            if req.engine.is_empty() || req.constraint.is_empty() {
                return Err(invalid_entry(format!(
                    "payload '{}' {} runtime_requirement needs engine and constraint",
                    payload.name, self.version
                )));
            }
        }
        // Entrypoints are the dispatchable view's mirror (spec 03 §4 tier
        // 3, spec 07 §1: app entrypoints ∪ toolkit executables, one view):
        // an app declares at least one command; a toolkit declares its
        // executables (a pure-library toolkit none); every other kind
        // declares none — its consumption is mount-only.
        match payload.kind {
            PayloadKind::App if self.entrypoints.is_empty() => {
                return Err(invalid_entry(format!(
                    "payload '{}' {} is an app but declares no entrypoints",
                    payload.name, self.version
                )));
            }
            PayloadKind::App | PayloadKind::Toolkit => {}
            _ if !self.entrypoints.is_empty() => {
                return Err(invalid_entry(format!(
                    "payload '{}' {} is kind {:?} — only apps and toolkits declare entrypoints",
                    payload.name, self.version, payload.kind
                )));
            }
            _ => {}
        }
        for ep in &self.entrypoints {
            check_path_safe("entrypoint", ep)?;
        }
        Ok(())
    }

    /// The declarative host-triplet selection (spec 04 §2): `universal` →
    /// the release's single-`.tfs` rule (no artifact name; the release
    /// ref's own `?sha256=` pin is the digest channel); per-triplet →
    /// `platforms[host]`, `None` when the host triplet is not published.
    pub fn select(&self, host: Platform) -> Option<PlatformSelection<'_>> {
        match &self.platforms {
            RegistryPlatforms::Universal => Some(PlatformSelection::Universal),
            RegistryPlatforms::PerTriplet(map) => {
                map.get(&host).map(|e| PlatformSelection::Selected {
                    artifact: e.artifact.as_str(),
                    sha256: e.sha256.as_str(),
                })
            }
        }
    }

    /// The triplets this version is published for (for the named
    /// platform-missing error); empty for universal payloads.
    pub fn published_triplets(&self) -> Vec<Platform> {
        match &self.platforms {
            RegistryPlatforms::Universal => Vec::new(),
            RegistryPlatforms::PerTriplet(map) => map.keys().copied().collect(),
        }
    }

    /// spec 04 §2 (roadmap 85): the row is yanked (`status: withdrawn`) —
    /// resolvers refuse it by name ([`RegistryError::Withdrawn`]), never a
    /// silent skip, never a fallback to it.
    pub fn is_withdrawn(&self) -> bool {
        self.status.as_deref() == Some("withdrawn")
    }
}

/// The outcome of [`RegistryVersion::select`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformSelection<'a> {
    /// Pure-language payload: the release's single `.tfs` asset (spec 04
    /// §1 no-`#` rule), pinned only when the release ref carries
    /// `?sha256=`.
    Universal,
    /// The host triplet's declared artifact + sha256 pin.
    Selected { artifact: &'a str, sha256: &'a str },
}

// ---------------------------------------------------------------------
// The registry reference (spec 04 §2 — exactly one location per form)
// ---------------------------------------------------------------------

/// A parsed registry reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryRef {
    /// `tfs:<svc>:owner/repo[?sha256=<hex>]` — `/tpkg-registry.yaml` at
    /// the DEFAULT branch via the service contents API. The explicit-host
    /// form `tfs+<svc>://host/owner/repo` (spec 37 §4) carries the host;
    /// `None` is the service's canonical SaaS host. The pin (query form,
    /// any class — spec 04 §1) verifies the registry file itself.
    DefaultBranch {
        service: Service,
        /// The explicit host of the `tfs+<svc>://host/…` form (spec 37
        /// §4); `None` on the SaaS forms.
        host: Option<String>,
        owner: String,
        repo: String,
        sha256: Option<String>,
    },
    /// `tfs:<svc>:owner/repo:version#tpkg-registry.yaml` (or the
    /// explicit-host form, spec 37 §4) — the registry file as a release
    /// artifact (pinned-immutable, versioned with its payloads).
    ReleaseArtifact(Reference),
    /// `tfs+git://…#path` — a blob in any git ref/path.
    GitBlob(Reference),
    /// `tfs+https://host/path/tpkg-registry.yaml` — the registry file
    /// itself over plain HTTPS (spec 37 §4: the one new registry
    /// location; artifact refs inside it stay ordinary spec 04
    /// references). The File-like variant over [`Reference::Https`].
    Https(Reference),
    /// `file:///abs/path` — local mirror (tests, air-gapped sites).
    File(Reference),
}

impl RegistryRef {
    /// Parse a registry reference. Anything outside the forms is a named
    /// error listing them — no search, no fallback chain (spec 04 §2;
    /// spec 37 §4's federation refusals ride along by name).
    pub fn parse(input: &str) -> Result<RegistryRef, RegistryError> {
        let bad = |reason: String| RegistryError::BadRef {
            input: input.to_string(),
            reason,
        };
        let input = input.trim();
        for (prefix, service) in [
            ("tfs:github:", Service::Github),
            ("tfs:gitlab:", Service::Gitlab),
            ("tfs:bb:", Service::Bitbucket),
        ] {
            if let Some(rest) = input.strip_prefix(prefix) {
                return parse_service_registry(input, rest, service, None);
            }
        }
        for (prefix, service) in [
            ("tfs+github://", Service::Github),
            ("tfs+gitlab://", Service::Gitlab),
        ] {
            if let Some(rest) = input.strip_prefix(prefix) {
                return parse_hosted_service_registry(input, rest, service);
            }
        }
        // The federation refusals (spec 37 §4) keep their names on the
        // registry path too — BadRef's reason carries the named message.
        for prefix in ["tfs+bb://", "tfs+bitbucket://"] {
            if input.starts_with(prefix) {
                return Err(bad(ReferenceError::UnsupportedService {
                    input: input.to_string(),
                }
                .to_string()));
            }
        }
        if crate::reference::is_ssh_form(input) {
            return Err(bad(ReferenceError::SshTransportUnsupported {
                input: input.to_string(),
            }
            .to_string()));
        }
        if input.starts_with("tfs+git://") {
            let reference = Reference::parse(input).map_err(|e| bad(format!("{e}")))?;
            return match &reference {
                Reference::Git { path: Some(_), .. } => Ok(RegistryRef::GitBlob(reference)),
                Reference::Git { url, .. } => Err(bad(format!(
                    "tfs+git://{url} names a repository, not the registry file — add #path"
                ))),
                _ => unreachable!("tfs+git:// parses as Reference::Git"),
            };
        }
        if input.starts_with("tfs+https://") {
            // The HTTPS registry location (spec 37 §4): the registry file
            // itself over plain HTTPS — the File-like variant.
            let reference = Reference::parse(input).map_err(|e| bad(format!("{e}")))?;
            return Ok(RegistryRef::Https(reference));
        }
        if input.starts_with("file://") {
            let reference = Reference::parse(input).map_err(|e| bad(format!("{e}")))?;
            return Ok(RegistryRef::File(reference));
        }
        Err(bad("no registry form matches".to_string()))
    }

    /// The canonical string form (what `add-registry` stores).
    pub fn as_canonical_string(&self) -> String {
        match self {
            RegistryRef::DefaultBranch {
                service,
                host,
                owner,
                repo,
                sha256,
            } => {
                let base = match host {
                    Some(h) => format!("tfs+{}://{h}/{owner}/{repo}", service.scheme()),
                    None => format!("tfs:{}:{owner}/{repo}", service.scheme()),
                };
                match sha256 {
                    Some(sha) => format!("{base}?sha256={sha}"),
                    None => base,
                }
            }
            RegistryRef::ReleaseArtifact(r)
            | RegistryRef::GitBlob(r)
            | RegistryRef::Https(r)
            | RegistryRef::File(r) => r.to_string(),
        }
    }

    /// True when resolution needs the network (TEBAKO_OFFLINE gate; only
    /// `file://` mirrors resolve offline).
    pub fn is_remote(&self) -> bool {
        !matches!(self, RegistryRef::File(_))
    }
}

impl std::fmt::Display for RegistryRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_canonical_string())
    }
}

/// The service forms: `owner/repo` (default branch) vs
/// `owner/repo:version#tpkg-registry.yaml` (release artifact). The split
/// mirrors `parse_service`'s grammar; a version WITHOUT the `#artifact`
/// is a payload reference, not a registry one — named error. `host` is
/// the explicit host of the `tfs+<svc>://host/…` form (spec 37 §4);
/// `None` is the SaaS form. ONE code path — the host is a parameter.
fn parse_service_registry(
    input: &str,
    rest: &str,
    service: Service,
    host: Option<&str>,
) -> Result<RegistryRef, RegistryError> {
    let bad = |reason: String| RegistryError::BadRef {
        input: input.to_string(),
        reason,
    };
    // Strip the fragment and query to see the grammar shape: a `:version`
    // suffix means the release-artifact form, its absence the
    // default-branch form.
    let before_frag = rest.split_once('#').map(|(b, _)| b).unwrap_or(rest);
    let body = before_frag
        .split_once('?')
        .map(|(b, _)| b)
        .unwrap_or(before_frag);
    if body.contains(':') {
        // Versioned form: must parse as a payload reference and carry the
        // #artifact (the registry file's name within the release).
        let reference = Reference::parse(input).map_err(|e| bad(format!("{e}")))?;
        return match &reference {
            Reference::Service {
                artifact: Some(_), ..
            } => Ok(RegistryRef::ReleaseArtifact(reference)),
            Reference::Service { .. } => Err(bad(
                "a versioned registry ref names the registry file as a release artifact: tfs:<service>:owner/repo:version#tpkg-registry.yaml"
                    .to_string(),
            )),
            _ => unreachable!("tfs:<svc>: parses as Reference::Service"),
        };
    }
    // Default-branch form: owner/repo, an optional ?sha256= pin, and no
    // fragment (the file's path is locked — never silently dropped).
    if rest.contains('#') {
        return Err(bad(
            "the default-branch form takes no #fragment — the file is /tpkg-registry.yaml"
                .to_string(),
        ));
    }
    let (body, query) = match rest.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (rest, None),
    };
    let sha256 = crate::reference::parse_exact_pin(input, query).map_err(|e: ReferenceError| {
        RegistryError::BadRef {
            input: input.to_string(),
            reason: e.to_string(),
        }
    })?;
    let Some((owner, repo)) = body.rsplit_once('/') else {
        return Err(bad("missing 'owner/repo' path".to_string()));
    };
    check_component(input, "owner", owner, &['?', '#', ':', '@'])
        .map_err(|e: ReferenceError| bad(e.to_string()))?;
    check_component(input, "repo", repo, &['?', '#', ':', '@'])
        .map_err(|e: ReferenceError| bad(e.to_string()))?;
    Ok(RegistryRef::DefaultBranch {
        service,
        host: host.map(str::to_string),
        owner: owner.to_string(),
        repo: repo.to_string(),
        sha256,
    })
}

/// The explicit-host service forms (spec 37 §4):
/// `tfs+<svc>://host/owner/repo[?sha256=…]` (default branch at a GHE /
/// self-hosted GitLab host) and `…:version#tpkg-registry.yaml` (release
/// artifact). The host splits off at the FIRST '/' — a `host:port` never
/// collides with the grammar-shape detection on the body.
fn parse_hosted_service_registry(
    input: &str,
    rest: &str,
    service: Service,
) -> Result<RegistryRef, RegistryError> {
    let bad = |reason: String| RegistryError::BadRef {
        input: input.to_string(),
        reason,
    };
    let Some((host, body)) = rest.split_once('/') else {
        return Err(bad(format!(
            "missing 'owner/repo' path — the form is tfs+{}://host/owner/repo[:version#tpkg-registry.yaml]",
            service.scheme()
        )));
    };
    check_component(input, "host", host, &['?', '#', '@'])
        .map_err(|e: ReferenceError| bad(e.to_string()))?;
    parse_service_registry(input, body, service, Some(host))
}

// ---------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------

impl<T: Transport> Fetcher<T> {
    /// Fetch the registry file a [`RegistryRef`] names (spec 04 §2).
    /// `TEBAKO_OFFLINE=1`: only `file://` mirrors resolve — anything
    /// remote is the named hard error (spec 05 §4).
    pub fn fetch_registry(&self, r: &RegistryRef) -> Result<Vec<u8>, ResolveError> {
        if r.is_remote() && crate::cache::offline() {
            return Err(ResolveError::Offline {
                what: format!("registry {r}"),
            });
        }
        // The tier-1 key (spec 37 §5): the registry-file fetch matches
        // its OWN registry's alias — the book's ref reverse-lookup.
        let alias = self.effective_book().alias_of(&r.as_canonical_string());
        match r {
            RegistryRef::DefaultBranch {
                service,
                host,
                owner,
                repo,
                sha256,
            } => {
                let scoped = self.scoped_transport(alias.as_deref(), Some(*service), host.clone());
                let bytes = crate::adapters::adapter_for_host(*service, host.as_deref())?
                    .registry_file(&scoped, owner, repo)?;
                if let Some(expected) = sha256 {
                    let actual = crate::fetch::sha256_hex(&bytes);
                    if &actual != expected {
                        return Err(ResolveError::Sha256Mismatch {
                            origin: r.as_canonical_string(),
                            expected: expected.clone(),
                            actual,
                        });
                    }
                }
                Ok(bytes)
            }
            RegistryRef::ReleaseArtifact(reference)
            | RegistryRef::GitBlob(reference)
            | RegistryRef::Https(reference)
            | RegistryRef::File(reference) => {
                Ok(self.fetch_scoped(reference, alias.as_deref())?.bytes)
            }
        }
    }

    /// Fetch and parse the registry a [`RegistryRef`] names.
    pub fn resolve_registry(&self, r: &RegistryRef) -> Result<Registry, ResolveError> {
        let bytes = self.fetch_registry(r)?;
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::Yaml {
            reason: format!("{e} decoding the registry file"),
        })?;
        Ok(Registry::from_yaml(&text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
schema_version: 1
payloads:
  - name: metanorma
    kind: app
    versions:
      - version: 1.2.3
        platforms:
          x86_64-linux-gnu:
            artifact: metanorma-1.2.3-linux-gnu-x86_64.tfs
            sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
          aarch64-macos:
            artifact: metanorma-1.2.3-macos-arm64.tfs
            sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        release: {ref: tfs:github:metanorma/metanorma:1.2.3}
        signature: {keyid: "0123456789abcdef", asc: "metanorma-1.2.3-macos-arm64.tfs.asc"}
        runtime_requirement: {engine: ruby, constraint: "~> 3.3.0"}
        entrypoints: [metanorma]
    default: 1.2.3
  - name: pure-tool
    kind: app
    versions:
      - version: 2.0
        platforms: universal
        release: {ref: tfs:github:acme/pure-tool:2.0}
        entrypoints: [pure-tool]
"#;

    #[test]
    fn a_registry_without_schema_version_is_the_era_1_refusal() {
        // spec 18 C8/S46: missing schema_version is pre-era — "republish
        // the registry", never a silent default.
        let err = Registry::from_yaml("payloads: []\n").unwrap_err();
        assert!(matches!(err, RegistryError::PreEra));
        let msg = err.to_string();
        assert!(msg.contains("pre-era"), "{msg}");
        assert!(msg.contains("republish the registry"), "{msg}");
        // an explicit null is the same absence
        let err = Registry::from_yaml("schema_version: null\npayloads: []\n").unwrap_err();
        assert!(matches!(err, RegistryError::PreEra));
    }

    #[test]
    fn a_newer_schema_major_is_the_upgrade_refusal() {
        // spec 18 C8/S45.
        let err = Registry::from_yaml("schema_version: 99\npayloads: []\n").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("schema_version 99"), "{msg}");
        assert!(msg.contains("speaks (1)"), "{msg}");
        assert!(msg.contains("upgrade tebako"), "{msg}");
        // distinct from the era-1 message class
        assert!(!msg.contains("pre-era"), "{msg}");
    }

    #[test]
    fn model_round_trips_the_spec_example() {
        let registry = Registry::from_yaml(EXAMPLE).unwrap();
        assert_eq!(registry.schema_version, Some(1));
        assert_eq!(registry.payloads.len(), 2);

        let m = registry.payload("metanorma").unwrap();
        assert_eq!(m.kind, PayloadKind::App);
        assert_eq!(m.default.as_deref(), Some("1.2.3"));
        let v = m.default_version().unwrap();
        assert_eq!(
            v.select(Platform::Aarch64Macos),
            Some(PlatformSelection::Selected {
                artifact: "metanorma-1.2.3-macos-arm64.tfs",
                sha256: &"b".repeat(64),
            })
        );
        assert_eq!(
            v.select(Platform::X86_64LinuxGnu),
            Some(PlatformSelection::Selected {
                artifact: "metanorma-1.2.3-linux-gnu-x86_64.tfs",
                sha256: &"a".repeat(64),
            })
        );
        // a triplet the registry does not publish → None (the caller's
        // named error lists published_triplets)
        assert_eq!(v.select(Platform::X86_64WindowsUcrt), None);
        assert_eq!(
            v.published_triplets(),
            vec![Platform::Aarch64Macos, Platform::X86_64LinuxGnu]
        );
        assert_eq!(v.signature.as_ref().unwrap().keyid, "0123456789abcdef");

        let p = registry.payload("pure-tool").unwrap();
        let v = &p.versions[0];
        assert!(matches!(v.platforms, RegistryPlatforms::Universal));
        assert_eq!(
            v.select(Platform::X86_64WindowsUcrt),
            Some(PlatformSelection::Universal)
        );
        assert!(p.default_version().is_none());

        // round-trip identity
        let yaml = registry.to_yaml().unwrap();
        let again = Registry::from_yaml(&yaml).unwrap();
        assert_eq!(registry, again);
    }

    #[test]
    fn schema_errors_are_named() {
        for (yaml, needle) in [
            ("schema_version: 2\npayloads: []\n", "schema_version 2"),
            ("schema_version: 0\npayloads: []\n", "schema_version 0"),
            ("schema_version: one\n", "yaml"), // structural
            (
                "schema_version: 1\npayloads:\n  - {name: '', kind: app, versions: []}\n",
                "payload name",
            ),
            (
                "schema_version: 1\npayloads:\n  - {name: x, kind: app, versions: []}\n",
                "lists no versions",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: bogus}, entrypoints: [x]}\n",
                "release.ref does not parse",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - {version: 1.0, platforms: {aarch64-macos: {artifact: a.tfs, sha256: zz}}, release: {ref: tfs:github:o/x:1.0}, entrypoints: [x]}\n",
                "64 lowercase hex",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - {version: 1.0, platforms: {aarch64-macos: {artifact: a.tfs, sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}}, release: {ref: file:///m/a.tfs}, entrypoints: [x]}\n",
                "not a service release",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: tfs:github:o/x:1.0}}\n",
                "declares no entrypoints",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: data\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: file:///m/a.tfs}, entrypoints: [x]}\n",
                "only apps and toolkits declare entrypoints",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    default: 9.9\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: tfs:github:o/x:1.0}, entrypoints: [x]}\n",
                "default '9.9'",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: tfs:github:o/x:1.0#a.tfs}, entrypoints: [x]}\n",
                "carries an #artifact",
            ),
            (
                "schema_version: 1\npayloads:\n  - {name: x, kind: app, versions: [{version: 1.0, platforms: universal, release: {ref: tfs:github:o/x:1.0}, entrypoints: [x]}]}\n  - {name: x, kind: app, versions: [{version: 2.0, platforms: universal, release: {ref: tfs:github:o/x:2.0}, entrypoints: [x]}]}\n",
                "duplicate payload name",
            ),
            (
                "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: tfs:github:o/x:1.0}, entrypoints: [x], signature: {keyid: XYZ, asc: a.asc}}\n",
                "16 lowercase hex",
            ),
        ] {
            let err = Registry::from_yaml(yaml).unwrap_err();
            assert!(
                err.to_string().contains(needle),
                "expected '{needle}' in: {err}"
            );
        }
    }

    #[test]
    fn registry_ref_forms() {
        let r = RegistryRef::parse("tfs:github:metanorma/metanorma").unwrap();
        assert_eq!(
            r,
            RegistryRef::DefaultBranch {
                service: Service::Github,
                host: None,
                owner: "metanorma".into(),
                repo: "metanorma".into(),
                sha256: None,
            }
        );
        assert_eq!(r.as_canonical_string(), "tfs:github:metanorma/metanorma");
        assert!(r.is_remote());

        // the default-branch form takes the ?sha256= pin (any class) and
        // keeps it; a #fragment is a named error, never silently dropped
        let sha = "f".repeat(64);
        let r = RegistryRef::parse(&format!("tfs:bb:o/r?sha256={sha}")).unwrap();
        assert_eq!(
            r,
            RegistryRef::DefaultBranch {
                service: Service::Bitbucket,
                host: None,
                owner: "o".into(),
                repo: "r".into(),
                sha256: Some(sha.clone()),
            }
        );
        assert_eq!(r.as_canonical_string(), format!("tfs:bb:o/r?sha256={sha}"));

        let r = RegistryRef::parse("tfs:gitlab:group/sub/r:v1#tpkg-registry.yaml").unwrap();
        assert!(matches!(r, RegistryRef::ReleaseArtifact(_)));
        assert_eq!(
            r.as_canonical_string(),
            "tfs:gitlab:group/sub/r:v1#tpkg-registry.yaml"
        );

        let r = RegistryRef::parse("tfs+git://h/registry.git@main#tpkg-registry.yaml").unwrap();
        assert!(matches!(r, RegistryRef::GitBlob(_)));

        let r = RegistryRef::parse("file:///mirror/tpkg-registry.yaml").unwrap();
        assert!(matches!(r, RegistryRef::File(_)));
        assert!(!r.is_remote());

        for (bad, needle) in [
            // a versioned service ref without the #artifact is a payload ref
            ("tfs:github:o/r:v1", "release artifact"),
            ("tfs:github:o", "owner/repo"),
            ("tfs:github:o/r#x.yaml", "no #fragment"),
            ("tfs:github:o/r?x=1", "sha256"),
            ("tfs+git://h/registry.git", "add #path"),
            ("https://cdn/r.yaml", "no registry form matches"),
            ("metanorma", "no registry form matches"),
        ] {
            let err = RegistryRef::parse(bad).unwrap_err();
            assert!(
                matches!(err, RegistryError::BadRef { .. }),
                "{bad} must be BadRef, got {err:?}"
            );
            assert!(
                err.to_string().contains(needle),
                "{bad}: expected '{needle}' in: {err}"
            );
        }
    }

    // -----------------------------------------------------------------
    // The federation grammar (spec 37 §4): explicit hosts, the HTTPS
    // registry location, the two named refusals.
    // -----------------------------------------------------------------

    #[test]
    fn hosted_registry_refs_round_trip() {
        // default-branch registry at a GHE host
        let r = RegistryRef::parse("tfs+github://ghe.corp.internal/owner/repo").unwrap();
        assert_eq!(
            r,
            RegistryRef::DefaultBranch {
                service: Service::Github,
                host: Some("ghe.corp.internal".into()),
                owner: "owner".into(),
                repo: "repo".into(),
                sha256: None,
            }
        );
        assert_eq!(
            r.as_canonical_string(),
            "tfs+github://ghe.corp.internal/owner/repo"
        );
        assert!(r.is_remote());

        // the pin rides the hosted default-branch form
        let sha = "e".repeat(64);
        let r = RegistryRef::parse(&format!(
            "tfs+github://ghe.corp.internal:8443/owner/repo?sha256={sha}"
        ))
        .unwrap();
        assert_eq!(
            r,
            RegistryRef::DefaultBranch {
                service: Service::Github,
                host: Some("ghe.corp.internal:8443".into()),
                owner: "owner".into(),
                repo: "repo".into(),
                sha256: Some(sha.clone()),
            }
        );
        assert_eq!(
            r.as_canonical_string(),
            format!("tfs+github://ghe.corp.internal:8443/owner/repo?sha256={sha}")
        );

        // release-artifact form at a self-hosted gitlab, nested groups
        let r = RegistryRef::parse(
            "tfs+gitlab://gitlab.corp.internal/group/sub/r:v1#tpkg-registry.yaml",
        )
        .unwrap();
        assert!(matches!(r, RegistryRef::ReleaseArtifact(_)));
        assert_eq!(
            r.as_canonical_string(),
            "tfs+gitlab://gitlab.corp.internal/group/sub/r:v1#tpkg-registry.yaml"
        );

        for (bad, needle) in [
            (
                "tfs+github://ghe.corp.internal/owner/repo:v1",
                "release artifact",
            ),
            ("tfs+github://ghe.corp.internal", "owner/repo"),
            ("tfs+github://ghe.corp.internal/owner", "owner/repo"),
            ("tfs+github://ghe.corp.internal/o/r#x.yaml", "no #fragment"),
        ] {
            let err = RegistryRef::parse(bad).unwrap_err();
            assert!(
                matches!(err, RegistryError::BadRef { .. }),
                "{bad} must be BadRef, got {err:?}"
            );
            assert!(
                err.to_string().contains(needle),
                "{bad}: expected '{needle}' in: {err}"
            );
        }
    }

    #[test]
    fn the_https_registry_location_parses_and_round_trips() {
        // spec 37 §4: the registry file itself over plain HTTPS — the one
        // new registry location (static server, S3/CDN, generic repo).
        let r = RegistryRef::parse("tfs+https://artifacts.corp.internal/tebako/tpkg-registry.yaml")
            .unwrap();
        assert!(
            matches!(&r, RegistryRef::Https(Reference::Https { url, sha256: None }) if url == "https://artifacts.corp.internal/tebako/tpkg-registry.yaml")
        );
        assert_eq!(
            r.as_canonical_string(),
            "tfs+https://artifacts.corp.internal/tebako/tpkg-registry.yaml"
        );
        assert!(r.is_remote());

        // the digest pin (query form, any class) verifies the file itself
        let sha = "d".repeat(64);
        let r = RegistryRef::parse(&format!(
            "tfs+https://artifacts.corp.internal/tebako/tpkg-registry.yaml?sha256={sha}"
        ))
        .unwrap();
        assert!(
            matches!(&r, RegistryRef::Https(Reference::Https { sha256: Some(s), .. }) if s == &sha)
        );
        assert_eq!(
            r.as_canonical_string(),
            format!("tfs+https://artifacts.corp.internal/tebako/tpkg-registry.yaml?sha256={sha}")
        );

        // the bare https:// form stays outside the registry grammar
        let err =
            RegistryRef::parse("https://artifacts.corp.internal/tpkg-registry.yaml").unwrap_err();
        assert!(
            err.to_string().contains("no registry form matches"),
            "{err}"
        );
    }

    #[test]
    fn the_federation_refusals_keep_their_names_on_the_registry_path() {
        // Bitbucket Data Center is not a registry host variant of tfs:bb:
        let err = RegistryRef::parse("tfs+bb://bbdc.corp.internal/o/r").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("UnsupportedService"), "{msg}");
        assert!(
            msg.contains("bitbucket data center is not bitbucket cloud"),
            "{msg}"
        );

        // any ssh/git@ form fails closed by name
        for bad in [
            "git@ghe.corp.internal:o/r.git#tpkg-registry.yaml",
            "ssh://git@ghe.corp.internal/o/r.git",
        ] {
            let err = RegistryRef::parse(bad).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("SshTransportUnsupported"), "{bad}: {msg}");
            assert!(msg.contains("no-shell-outs law"), "{bad}: {msg}");
        }
    }

    // -----------------------------------------------------------------
    // The registry runtime axis (spec 30 §1's edge-discovery key, spec
    // 28 §8's implementation axis; schema MINOR 1/2).
    // -----------------------------------------------------------------

    const RUNTIME_AXIS: &str = r#"
schema_version: 1
payloads:
  - name: tebako-runtime-ruby
    kind: runtime
    engine: ruby
    versions:
      - {version: '3.4.2-0.4.0', platforms: universal, release: {ref: tfs:github:tamatebako/tebako-runtime-ruby:3.4.2-0.4.0}}
  - name: tebako-runtime-java-temurin
    kind: runtime
    engine: java
    implementation: temurin
    versions:
      - {version: '21.0.12-0.4.0', platforms: universal, release: {ref: tfs:github:tamatebako/tebako-runtime-java:21.0.12-0.4.0}}
  - name: tebako-runtime-java-zulu
    kind: runtime
    engine: java
    implementation: zulu
    versions:
      - {version: '21.0.11-0.4.0', platforms: universal, release: {ref: tfs:github:tamatebako/tebako-runtime-java:21.0.11-0.4.0}}
  - name: legacy-runtime
    kind: runtime
    versions:
      - {version: '1.0', platforms: universal, release: {ref: tfs:github:acme/legacy-runtime:1.0}}
  - name: version-keyed
    kind: runtime
    engine: java
    versions:
      - {version: '21.0.1', implementation: temurin, platforms: universal, release: {ref: tfs:github:acme/version-keyed:21.0.1}}
      - {version: '21.0.2', implementation: temurin, platforms: universal, release: {ref: tfs:github:acme/version-keyed:21.0.2}}
  - name: disagreeing
    kind: runtime
    engine: java
    versions:
      - {version: '21.0.1', implementation: temurin, platforms: universal, release: {ref: tfs:github:acme/disagreeing:21.0.1}}
      - {version: '21.0.2', implementation: zulu, platforms: universal, release: {ref: tfs:github:acme/disagreeing:21.0.2}}
  - name: both-spellings
    kind: runtime
    engine: java
    implementation: zulu
    versions:
      - {version: '21.0.1', implementation: temurin, platforms: universal, release: {ref: tfs:github:acme/both-spellings:21.0.1}}
"#;

    fn entry_names(entries: Vec<&RegistryPayload>) -> Vec<&str> {
        entries.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn the_payload_level_engine_and_implementation_keys_parse_and_round_trip() {
        let registry = Registry::from_yaml(RUNTIME_AXIS).unwrap();
        let temurin = registry.payload("tebako-runtime-java-temurin").unwrap();
        assert_eq!(temurin.engine(), Some("java"));
        assert_eq!(temurin.implementation(), Some("temurin"));
        let ruby = registry.payload("tebako-runtime-ruby").unwrap();
        assert_eq!(ruby.engine(), Some("ruby"));
        assert_eq!(ruby.implementation(), None);
        // the keys survive the write/read round-trip
        let again = Registry::from_yaml(&registry.to_yaml().unwrap()).unwrap();
        assert_eq!(registry, again);
    }

    #[test]
    fn the_version_level_implementation_is_the_minor_2_compat_read() {
        let registry = Registry::from_yaml(RUNTIME_AXIS).unwrap();
        // every version carrying the key AGREES → the payload reads it
        assert_eq!(
            registry.payload("version-keyed").unwrap().implementation(),
            Some("temurin")
        );
        // a disagreement across versions is no axis, never a guess
        assert_eq!(
            registry.payload("disagreeing").unwrap().implementation(),
            None
        );
        // the payload-level key wins over the version-level spelling
        assert_eq!(
            registry.payload("both-spellings").unwrap().implementation(),
            Some("zulu")
        );
    }

    #[test]
    fn an_engine_less_runtime_resolves_by_name_but_stays_invisible_to_edges() {
        let registry = Registry::from_yaml(RUNTIME_AXIS).unwrap();
        // pre-discovery legacy: payload() still resolves it…
        let legacy = registry.payload("legacy-runtime").unwrap();
        assert_eq!(legacy.kind, PayloadKind::Runtime);
        assert_eq!(legacy.engine(), None);
        // …and no edge listing ever contains it
        for engine in ["ruby", "java"] {
            for implementation in [None, Some("temurin")] {
                assert!(
                    !entry_names(registry.runtime_entries(engine, implementation))
                        .contains(&"legacy-runtime"),
                    "legacy-runtime leaked into runtime_entries({engine}, {implementation:?})"
                );
            }
        }
    }

    #[test]
    fn runtime_entries_filters_the_engine_and_implementation_axes() {
        let registry = Registry::from_yaml(RUNTIME_AXIS).unwrap();
        // the engine axis alone lists every edge-visible entry serving it
        assert_eq!(
            entry_names(registry.runtime_entries("java", None)),
            vec![
                "tebako-runtime-java-temurin",
                "tebako-runtime-java-zulu",
                "version-keyed",
                "disagreeing",
                "both-spellings",
            ]
        );
        assert_eq!(
            entry_names(registry.runtime_entries("ruby", None)),
            vec!["tebako-runtime-ruby"]
        );
        // an edge naming an implementation matches only entries carrying
        // the same value — payload-level and (agreeing) compat-read alike;
        // a disagreement (None) never matches a named implementation
        assert_eq!(
            entry_names(registry.runtime_entries("java", Some("temurin"))),
            vec!["tebako-runtime-java-temurin", "version-keyed"]
        );
        assert_eq!(
            entry_names(registry.runtime_entries("java", Some("zulu"))),
            vec!["tebako-runtime-java-zulu", "both-spellings"]
        );
        // an unknown engine is no answer, never a guess
        assert!(registry.runtime_entries("python", None).is_empty());
    }

    #[test]
    fn the_runtime_axis_keys_are_non_empty_when_present() {
        for (yaml, needle) in [
            (
                "schema_version: 1\npayloads:\n  - {name: x, kind: runtime, engine: '', versions: [{version: '1.0', platforms: universal, release: {ref: file:///m/a.tfs}}]}\n",
                "engine must not be empty",
            ),
            (
                "schema_version: 1\npayloads:\n  - {name: x, kind: runtime, implementation: '', versions: [{version: '1.0', platforms: universal, release: {ref: file:///m/a.tfs}}]}\n",
                "implementation must not be empty",
            ),
            (
                "schema_version: 1\npayloads:\n  - {name: x, kind: runtime, versions: [{version: '1.0', implementation: '', platforms: universal, release: {ref: file:///m/a.tfs}}]}\n",
                "implementation must not be empty",
            ),
        ] {
            let err = Registry::from_yaml(yaml).unwrap_err();
            assert!(
                err.to_string().contains(needle),
                "expected '{needle}' in: {err}"
            );
        }
    }

    // -----------------------------------------------------------------
    // The withdrawal axis (spec 04 §2, roadmap 85)
    // -----------------------------------------------------------------

    #[test]
    fn a_withdrawn_row_parses_flags_and_round_trips() {
        let yaml = r#"
schema_version: 1
payloads:
  - name: metanorma
    kind: app
    versions:
      - {version: 1.2.3, status: withdrawn, platforms: universal, release: {ref: tfs:github:o/m:1.2.3}, entrypoints: [metanorma]}
      - {version: 1.2.4, platforms: universal, release: {ref: tfs:github:o/m:1.2.4}, entrypoints: [metanorma]}
    default: 1.2.4
"#;
        let registry = Registry::from_yaml(yaml).unwrap();
        let m = registry.payload("metanorma").unwrap();
        assert!(m.version("1.2.3").unwrap().is_withdrawn());
        assert!(!m.version("1.2.4").unwrap().is_withdrawn());
        // the key survives the write/read round-trip
        let again = Registry::from_yaml(&registry.to_yaml().unwrap()).unwrap();
        assert_eq!(registry, again);
    }

    #[test]
    fn an_unknown_status_is_forward_tolerant_and_inert() {
        // Option<String>, not an enum: a status this build predates parses
        // and carries no refusal semantics.
        let yaml = r#"
schema_version: 1
payloads:
  - name: tool
    kind: app
    versions:
      - {version: 2.0, status: deprecated, platforms: universal, release: {ref: tfs:github:o/tool:2.0}, entrypoints: [tool]}
"#;
        let registry = Registry::from_yaml(yaml).unwrap();
        let v = registry.payload("tool").unwrap().version("2.0").unwrap();
        assert_eq!(v.status.as_deref(), Some("deprecated"));
        assert!(!v.is_withdrawn());
    }

    #[test]
    fn the_withdrawn_error_names_the_payload_and_version() {
        let err = RegistryError::Withdrawn {
            payload: "metanorma".to_string(),
            version: "1.2.3".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("WithdrawnPayload"), "{msg}");
        assert!(msg.contains("metanorma"), "{msg}");
        assert!(msg.contains("1.2.3"), "{msg}");
        assert!(msg.contains("status: withdrawn"), "{msg}");
        // the L3 wrapper carries it through verbatim
        let wrapped = ResolveError::from(err);
        assert!(wrapped.to_string().contains("WithdrawnPayload"));
    }
}
