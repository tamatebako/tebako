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
//! fallback chain):
//!
//! ```text
//! tfs:<svc>:owner/repo                          → /tpkg-registry.yaml on the
//!                                                 DEFAULT branch (contents API)
//! tfs:<svc>:owner/repo:version#tpkg-registry.yaml → release artifact
//! tfs+git://host/owner/repo.git[@ref]#path      → git blob
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
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payloads: Vec<RegistryPayload>,
}

/// One listed payload: name, kind, the version entries, and the
/// registry-side default the dispatcher's version chain ends on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryPayload {
    pub name: String,
    pub kind: PayloadKind,
    pub versions: Vec<RegistryVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// One version entry of a payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryVersion {
    pub version: String,
    pub platforms: RegistryPlatforms,
    /// The payload's release home — a spec 04 §1 reference (any class;
    /// per-triplet `platforms` require a service release, since artifact
    /// names only exist there).
    pub release: ReleaseRef,
    /// Opt-in OpenPGP signature of the artifact (spec 09): the signer
    /// keyid (16 lowercase hex) and the detached `.asc` — an asset name
    /// within the same release, or a full reference. One signature covers
    /// exactly one artifact, so per-triplet releases carry one asc per
    /// artifact by convention (`<artifact>.asc`) and the installer
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

/// `runtime_requirement: {engine: …, constraint: …}` (optional mirror).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryRuntimeRequirement {
    pub engine: String,
    pub constraint: String,
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

    fn validate(&self) -> Result<(), RegistryError> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(invalid_entry(format!(
                "unsupported schema_version {} (this build reads {REGISTRY_SCHEMA_VERSION})",
                self.schema_version
            )));
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
}

impl RegistryVersion {
    fn validate(&self, payload: &RegistryPayload) -> Result<(), RegistryError> {
        check_path_safe("version", &self.version)?;
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
                    if platform.is_reserved() {
                        return Err(invalid_entry(format!(
                            "payload '{}' {} names the reserved triplet {platform}",
                            payload.name, self.version
                        )));
                    }
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
    /// the DEFAULT branch via the service contents API. The pin (query
    /// form, any class — spec 04 §1) verifies the registry file itself.
    DefaultBranch {
        service: Service,
        owner: String,
        repo: String,
        sha256: Option<String>,
    },
    /// `tfs:<svc>:owner/repo:version#tpkg-registry.yaml` — the registry
    /// file as a release artifact (pinned-immutable, versioned with its
    /// payloads).
    ReleaseArtifact(Reference),
    /// `tfs+git://…#path` — a blob in any git ref/path.
    GitBlob(Reference),
    /// `file:///abs/path` — local mirror (tests, air-gapped sites).
    File(Reference),
}

impl RegistryRef {
    /// Parse a registry reference. Anything outside the four forms is a
    /// named error listing them — no search, no fallback chain.
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
                return parse_service_registry(input, rest, service);
            }
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
                owner,
                repo,
                sha256,
            } => {
                let base = format!("tfs:{}:{owner}/{repo}", service.scheme());
                match sha256 {
                    Some(sha) => format!("{base}?sha256={sha}"),
                    None => base,
                }
            }
            RegistryRef::ReleaseArtifact(r) | RegistryRef::GitBlob(r) | RegistryRef::File(r) => {
                r.to_string()
            }
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
/// is a payload reference, not a registry one — named error.
fn parse_service_registry(
    input: &str,
    rest: &str,
    service: Service,
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
        owner: owner.to_string(),
        repo: repo.to_string(),
        sha256,
    })
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
        match r {
            RegistryRef::DefaultBranch {
                service,
                owner,
                repo,
                sha256,
            } => {
                let bytes = crate::adapters::adapter_for(*service).registry_file(
                    &self.transport,
                    owner,
                    repo,
                )?;
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
            | RegistryRef::File(reference) => Ok(self.fetch(reference)?.bytes),
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
    fn model_round_trips_the_spec_example() {
        let registry = Registry::from_yaml(EXAMPLE).unwrap();
        assert_eq!(registry.schema_version, 1);
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
}
