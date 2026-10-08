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
    /// The registry publisher's signing key (spec 09 §9.1, schema MINOR
    /// 7): the add-registry TOFU channel — the reader displays the
    /// fingerprint for out-of-band confirmation and pins the key on
    /// consent. Additive: pre-MINOR-7 readers ignore the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing: Option<RegistrySigning>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payloads: Vec<RegistryPayload>,
}

/// The registry head's `signing:` block (spec 09 §9.1): the publisher's
/// armored public key, its primary fingerprint (the TOFU comparison
/// value), and an optional canonical URL where the same key is published
/// for cross-checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistrySigning {
    /// The armored public key (`-----BEGIN PGP PUBLIC KEY BLOCK-----`).
    pub key: String,
    /// The key's primary fingerprint (40 hex, either case — comparisons
    /// normalize).
    pub fingerprint: String,
    /// Where the same key is published for out-of-band confirmation
    /// (the publisher's well-known page).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
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
    /// spec 28 §3's no-selector pick (schema MINOR 9): the variant id
    /// (derived, never authored as a key — it appears here only as the
    /// SELECTION) resolution starts from. Validated against the default
    /// version's variants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_variant: Option<String>,
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
    /// The single-variant shorthand's platform axis (spec 04 §2): present
    /// exactly when `variants:` is absent (spec 28 §3's MECE law — both
    /// at once is a named validation error; neither is too).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<RegistryPlatforms>,
    /// The universal row's blksum sidecar pin (spec 39 §3, MINOR 4): a
    /// `platforms: universal` row names ONE artifact by the single-.tfs
    /// rule, so its sidecar pin lives at the version level — spelled on
    /// a per-triplet row it is a named validation error (the pins live
    /// in `platforms[<triplet>].blksum`; exactly one location per form).
    /// On a variants entry each arm carries its own pin (arm-level
    /// `blksum`, universal arms only) — the version-level spelling is a
    /// named error there too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blksum: Option<BlksumPin>,
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
    /// the exact asset only for universal payloads. On a `tfs+oci:`
    /// release ref the asc locator is DERIVED (spec 38 §3's
    /// `sha256-<signed blob hex>.asc` sibling tag) and `asc` must be
    /// absent — an authored one is a named validation error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignaturePin>,
    /// The runtime the payload's entrypoints need (spec 03 §2.2);
    /// mirrored for dispatch-time runtime resolution. Part of the
    /// single-variant shorthand — a variants entry carries the mirror per
    /// arm (the arm's requirement IS the variant's key, spec 28 §3).
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
    /// The variant arms (spec 28 §3, schema MINOR 9): one
    /// `{runtime_requirement, platforms}` row per build of this version.
    /// MECE with the shorthand — a version carries EITHER the top-level
    /// `platforms:` (+ `runtime_requirement:`) OR `variants:`, never
    /// both, never neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<RegistryVariant>>,
}

/// One variant arm of a version entry (spec 28 §3): the runtime
/// requirement that KEYS the variant (the variant id derives from it —
/// spec 28 §2 — and is never authored) plus the arm's own platforms map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryVariant {
    /// Absent keys the `universal` variant (a data slice, or a
    /// pure-language build — exactly one per version, the variant law).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_requirement: Option<RegistryRuntimeRequirement>,
    pub platforms: RegistryPlatforms,
    /// The arm's universal row sidecar pin (spec 39 §3's version-level
    /// rule applied per arm): present iff the arm's platforms are
    /// `universal`; on a per-triplet arm the pins live in the platforms
    /// map entries — the one-location-per-form rule, per arm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blksum: Option<BlksumPin>,
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
/// its sha256 pin (the registry-supplied trust anchor, spec 05 §4). The
/// additive `oci:` field (spec 38 §7) mirrors the artifact's OCI
/// locator — a MIRROR, never a second authority: the primary
/// `release.ref` resolves unless the book declares `channel: oci`;
/// pre-OCI readers ignore the field (spec 37 §2's forward-compat
/// leniency for unknown registry keys). The additive `blksum:` field
/// (spec 39 §3, MINOR 4) pins the artifact's lazy-mount digest sidecar.
/// The additive `release:` field (spec 04 §2, MINOR 6; tebako#711) names
/// the shard release carrying THIS row's bytes when a version line unions
/// rows from several per-platform shard tags (the version-level
/// `release.ref` cannot name the tag serving a given row); MINOR readers
/// prefer the row's ref, pre-MINOR readers ignore it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformArtifact {
    pub artifact: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oci: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blksum: Option<BlksumPin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<ReleaseRef>,
}

/// `blksum: {filename, sha256}` (spec 39 §3, additive — MINOR 4): the
/// pin anchoring the artifact's `<image>.blksum.json` lazy-mount digest
/// sidecar — the sidecar is fetched and verified against THIS sha256
/// BEFORE the first range read, exactly like `image.sha256` anchors the
/// image itself. Mirror-only (spec 03 §4's tier-3 rule): the sidecar
/// itself carries the per-group digests; a resolver never derives one.
/// Where the index is signed (spec 09 §5) the field is covered exactly
/// like `image.sha256`. Present but torn (a missing/empty half) is a
/// named validation error — never a silent downgrade to the eager path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlksumPin {
    /// The sidecar's asset name within the same release
    /// (`<artifact>.blksum.json`).
    pub filename: String,
    /// The sidecar bytes' sha256 (64 lowercase hex).
    pub sha256: String,
}

/// `release: {ref: …}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseRef {
    pub r#ref: String,
}

/// `signature: {keyid: …, asc: …}` (opt-in). `asc` names the detached
/// signature's locator — an asset name within the same release, or a
/// full reference; it is REQUIRED on every class except `tfs+oci:`
/// release refs, where the locator DERIVES from the signed blob's digest
/// (the `sha256-<hex>.asc` sibling tag, spec 38 §3) and an authored
/// `asc` is a named validation error (a locator that would be ignored
/// is an authoring bug).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePin {
    pub keyid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asc: Option<String>,
}

/// `runtime_requirement: {engine: …, constraint: …, implementation?,
/// abi?}` (optional mirror). `abi` is the runtime's platform string a
/// native-extension payload was built against (ruby:
/// `Gem::Platform.local.to_s`) — the resolution checks BOTH the version
/// line and the platform line (spec 05 §5); absent means pure-language
/// (the version line alone). `implementation` (spec 28 §8) mirrors the
/// L1 entry's implementation axis — REQUIRED in the mirror when `abi`
/// is present (an abi is per-implementation by construction). The abi is
/// per-triplet by construction, so the mirror carries it only when one
/// value holds for the whole entry (a `universal` or single-platform
/// row); a multi-platform per-triplet entry omits it — the authoritative
/// per-platform abi lives in each slice's embedded manifest (tebako#440).
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

/// The additive blksum sidecar pin (spec 39 §3): present but torn (an
/// empty filename or a malformed digest) is a named error — the lazy
/// reader's loud "blksum-missing" eager fallback applies to an ABSENT
/// pin, never to a broken one.
fn check_blksum_pin(what: &str, pin: &BlksumPin) -> Result<(), RegistryError> {
    if pin.filename.is_empty() {
        return Err(invalid_entry(format!("{what}.filename must not be empty")));
    }
    check_sha256(&format!("{what}.sha256"), &pin.sha256)
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
        if let Some(signing) = &self.signing {
            signing.validate()?;
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

impl RegistrySigning {
    /// The shape checks a pure-YAML reader can run (spec 09 §9.1). The
    /// cryptographic half — the declared fingerprint must BE the key's
    /// primary fingerprint — needs the OpenPGP stack and runs at the
    /// add-registry / registry-validate layer (tebako-cli), keeping this
    /// crate rnp-free for the size-gated bootstrap.
    fn validate(&self) -> Result<(), RegistryError> {
        let fp = &self.fingerprint;
        if fp.len() != 40 || !fp.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid_entry(format!(
                "signing.fingerprint '{fp}' is not a 40-hex fingerprint"
            )));
        }
        if !self.key.contains("-----BEGIN PGP PUBLIC KEY BLOCK-----")
            || !self.key.contains("-----END PGP PUBLIC KEY BLOCK-----")
        {
            return Err(invalid_entry(
                "signing.key is not an armored PGP public key block",
            ));
        }
        if let Some(url) = &self.url {
            if !url.starts_with("https://") {
                return Err(invalid_entry(format!(
                    "signing.url '{url}' is not an https URL — the cross-check page is a publication, fetched out of band"
                )));
            }
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
        // spec 28 §3/§10: `default_variant` is the no-selector pick among
        // the DEFAULT version's variants — it must exist (a default
        // version carrying variants), and it must name one of them.
        if let Some(dv) = &self.default_variant {
            let Some(default) = &self.default else {
                return Err(invalid_entry(format!(
                    "payload '{}' default_variant '{dv}' is declared but the payload pins no default version — the no-selector pick starts from the default version's variants",
                    self.name
                )));
            };
            let entry = self.version(default).expect("the default validated above");
            let Some(arms) = &entry.variants else {
                return Err(invalid_entry(format!(
                    "payload '{}' default_variant '{dv}' is declared but the default version {default} carries no variants:",
                    self.name
                )));
            };
            let mut ids = Vec::with_capacity(arms.len());
            for arm in arms {
                let (id, _) = variant_key(arm.runtime_requirement.as_ref()).map_err(|reason| {
                    invalid_entry(format!(
                        "payload '{}' {default} variant: {reason}",
                        self.name
                    ))
                })?;
                ids.push(id);
            }
            if !ids.iter().any(|id| id == dv) {
                return Err(invalid_entry(format!(
                    "payload '{}' default_variant '{dv}' names no variant of the default version {default} (variants: {})",
                    self.name,
                    ids.join(", ")
                )));
            }
        }
        // spec 28 §4 rule 1's authoring gate: variants spanning
        // IMPLEMENTATIONS need the declared default — the newest-line
        // rule is defined within one implementation, never across them.
        for v in &self.versions {
            let Some(arms) = &v.variants else { continue };
            let mut impls: Vec<&str> = arms
                .iter()
                .filter_map(|a| {
                    a.runtime_requirement
                        .as_ref()
                        .and_then(|r| r.implementation.as_deref())
                })
                .collect();
            impls.sort_unstable();
            impls.dedup();
            if impls.len() > 1 && self.default_variant.is_none() {
                return Err(invalid_entry(format!(
                    "payload '{}' {} variants span implementations ({}) but the payload declares no default_variant — the no-selector pick never guesses across implementations",
                    self.name,
                    v.version,
                    impls.join(", ")
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
        // spec 28 §3's MECE law: a version entry carries EITHER the
        // shorthand (top-level `platforms:` + `runtime_requirement:` —
        // exactly the single-variant form) OR `variants:`, never both,
        // never neither.
        match (&self.platforms, &self.variants) {
            (Some(_), Some(_)) => {
                return Err(invalid_entry(format!(
                    "payload '{}' {} carries both the shorthand (platforms:) and variants: — one form per version entry",
                    payload.name, self.version
                )));
            }
            (None, None) => {
                return Err(invalid_entry(format!(
                    "payload '{}' {} carries neither platforms: nor variants: — a version entry names its artifacts in exactly one of the two forms",
                    payload.name, self.version
                )));
            }
            (Some(platforms), None) => {
                validate_platforms_map(
                    &payload.name,
                    &self.version,
                    "platforms",
                    platforms,
                    &release,
                )?;
            }
            (None, Some(variants)) => {
                if self.runtime_requirement.is_some() {
                    return Err(invalid_entry(format!(
                        "payload '{}' {} runtime_requirement at the version level is the shorthand's mirror — a variants entry mirrors the requirement per arm (it is each variant's key)",
                        payload.name, self.version
                    )));
                }
                if variants.is_empty() {
                    return Err(invalid_entry(format!(
                        "payload '{}' {} variants: lists no arms",
                        payload.name, self.version
                    )));
                }
                let mut ids: Vec<String> = Vec::with_capacity(variants.len());
                for (i, variant) in variants.iter().enumerate() {
                    if let Some(req) = &variant.runtime_requirement {
                        if req.engine.is_empty() || req.constraint.is_empty() {
                            return Err(invalid_entry(format!(
                                "payload '{}' {} variants[{i}] runtime_requirement needs engine and constraint",
                                payload.name, self.version
                            )));
                        }
                    }
                    let (id, _) =
                        variant_key(variant.runtime_requirement.as_ref()).map_err(|reason| {
                            invalid_entry(format!(
                                "payload '{}' {} variants[{i}]: {reason}",
                                payload.name, self.version
                            ))
                        })?;
                    // The variant law (spec 28 §1): two arms of one
                    // version declaring the same requirement are the same
                    // variant — a duplicate key is a named authoring
                    // error.
                    if ids.contains(&id) {
                        return Err(invalid_entry(format!(
                            "payload '{}' {} lists the variant '{id}' twice — two builds of one version that declare the same requirement are the same variant",
                            payload.name, self.version
                        )));
                    }
                    validate_platforms_map(
                        &payload.name,
                        &self.version,
                        &format!("variants[{id}].platforms"),
                        &variant.platforms,
                        &release,
                    )?;
                    // spec 39 §3's one-location rule, per arm: the
                    // arm-level pin exists iff the arm names ONE
                    // artifact (its platforms are universal).
                    if let Some(blksum) = &variant.blksum {
                        if !matches!(variant.platforms, RegistryPlatforms::Universal) {
                            return Err(invalid_entry(format!(
                                "payload '{}' {} variants[{id}].blksum names the arm's single artifact — a per-triplet arm pins the sidecar in its platforms map entries",
                                payload.name, self.version
                            )));
                        }
                        check_blksum_pin(
                            &format!(
                                "payload '{}' {} variants[{id}].blksum",
                                payload.name, self.version
                            ),
                            blksum,
                        )?;
                    }
                    ids.push(id);
                }
            }
        }
        // The universal shorthand row's sidecar pin (spec 39 §3): a
        // `platforms: universal` row names ONE artifact by the
        // single-.tfs rule, so the pin lives at the version level;
        // spelled on a per-triplet row it duplicates the platforms map's
        // job — a named error. On a variants entry each arm carries its
        // own pin (variants[].blksum, universal arms only).
        if let Some(blksum) = &self.blksum {
            if self.variants.is_some() {
                return Err(invalid_entry(format!(
                    "payload '{}' {} blksum at the version level has no meaning on a variants entry — each arm carries its own (variants[].blksum on a universal arm; the platforms map entries otherwise)",
                    payload.name, self.version
                )));
            }
            if !matches!(self.platforms, Some(RegistryPlatforms::Universal)) {
                return Err(invalid_entry(format!(
                    "payload '{}' {} blksum at the version level names the universal payload's single artifact — per-triplet pins live in platforms[<triplet>].blksum",
                    payload.name, self.version
                )));
            }
            check_blksum_pin(
                &format!("payload '{}' {} blksum", payload.name, self.version),
                blksum,
            )?;
        }
        // The additive per-row OCI mirror / blksum pin / shard release
        // ref checks run inside validate_platforms_map (per shorthand map
        // and per variant arm's map).
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
            // spec 38 §3: on a tfs+oci: release ref the asc locator
            // DERIVES from the signed blob's digest (the sibling
            // digest-tag) — an authored one is a locator that would be
            // ignored, i.e. an authoring bug, named here.
            let is_oci = matches!(release, Reference::Oci { .. });
            match (&sig.asc, is_oci) {
                (Some(_), true) => {
                    return Err(invalid_entry(format!(
                        "payload '{}' {} signature.asc is authored on a tfs+oci: release ref — the asc locator derives from the signed blob's digest (the sha256-<hex>.asc sibling tag); drop the asc key",
                        payload.name, self.version
                    )));
                }
                (Some(asc), false) if asc.is_empty() => {
                    return Err(invalid_entry(format!(
                        "payload '{}' {} signature.asc must not be empty",
                        payload.name, self.version
                    )));
                }
                (None, false) => {
                    return Err(invalid_entry(format!(
                        "payload '{}' {} signature.asc is required (only a tfs+oci: release ref derives it)",
                        payload.name, self.version
                    )));
                }
                _ => {}
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

    /// The no-selector variant resolution (spec 28 §4 rule 1): the
    /// shorthand is the single-variant case; on a variants entry the
    /// payload's declared `default_variant` wins when it names a variant
    /// of THIS version (a mismatch is a named error — never a silent
    /// fallthrough), a one-arm entry resolves to its arm, and otherwise
    /// the variant whose requirement's line is NEWEST wins (a `universal`
    /// variant orders below every line-carrying one).
    pub fn resolve_variant(
        &self,
        default_variant: Option<&str>,
    ) -> Result<VariantView<'_>, RegistryError> {
        let Some(variants) = &self.variants else {
            let platforms = self.platforms.as_ref().ok_or_else(|| {
                invalid_entry(format!(
                    "version {} carries neither platforms: nor variants:",
                    self.version
                ))
            })?;
            return Ok(VariantView {
                id: None,
                picked_by: VariantPick::Shorthand,
                runtime_requirement: self.runtime_requirement.as_ref(),
                platforms,
                blksum: self.blksum.as_ref(),
            });
        };
        let mut arms: Vec<(String, Option<String>, &RegistryVariant)> = Vec::new();
        for variant in variants {
            let (id, line) =
                variant_key(variant.runtime_requirement.as_ref()).map_err(|reason| {
                    invalid_entry(format!("version {} variant: {reason}", self.version))
                })?;
            arms.push((id, line, variant));
        }
        let (picked, picked_by) = if let Some(dv) = default_variant {
            (
                arms.iter().find(|(id, _, _)| id == dv).ok_or_else(|| {
                    invalid_entry(format!(
                        "default_variant '{dv}' names no variant of version {} (variants: {})",
                        self.version,
                        arms.iter()
                            .map(|(id, _, _)| id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?,
                VariantPick::Default,
            )
        } else if arms.len() == 1 {
            (&arms[0], VariantPick::Only)
        } else {
            (
                arms.iter()
                    .max_by(|a, b| match (&a.1, &b.1) {
                        (Some(x), Some(y)) => tpkg::versions::compare(x, y),
                        (Some(_), None) => std::cmp::Ordering::Greater,
                        (None, Some(_)) => std::cmp::Ordering::Less,
                        (None, None) => std::cmp::Ordering::Equal,
                    })
                    .expect("variants non-empty post-validation"),
                VariantPick::NewestLine,
            )
        };
        Ok(VariantView {
            id: Some(picked.0.clone()),
            picked_by,
            runtime_requirement: picked.2.runtime_requirement.as_ref(),
            platforms: &picked.2.platforms,
            blksum: picked.2.blksum.as_ref(),
        })
    }

    /// spec 04 §2 (roadmap 85): the row is yanked (`status: withdrawn`) —
    /// resolvers refuse it by name ([`RegistryError::Withdrawn`]), never a
    /// silent skip, never a fallback to it.
    pub fn is_withdrawn(&self) -> bool {
        self.status.as_deref() == Some("withdrawn")
    }

    /// Every runtime-requirement mirror this version carries: the
    /// shorthand's plus each variant arm's (spec 28 §3). Consumers that
    /// scan requirements (the authoring gate, the retire gate) walk this
    /// — never the fields directly.
    pub fn requirement_mirrors(&self) -> impl Iterator<Item = &RegistryRuntimeRequirement> {
        self.runtime_requirement.iter().chain(
            self.variants
                .iter()
                .flatten()
                .filter_map(|a| a.runtime_requirement.as_ref()),
        )
    }

    /// Every (requirement mirror, its row's per-triplet platform count)
    /// pair — the authoring gate's per-triplet abi rule reads the count
    /// of the ROW the mirror sits on (the shorthand map, or the arm's
    /// map; universal rows count 0).
    pub fn requirement_rows(&self) -> Vec<(&RegistryRuntimeRequirement, usize)> {
        fn rows(p: &RegistryPlatforms) -> usize {
            match p {
                RegistryPlatforms::PerTriplet(m) => m.len(),
                RegistryPlatforms::Universal => 0,
            }
        }
        let mut out = Vec::new();
        if let (Some(req), Some(platforms)) = (&self.runtime_requirement, self.platforms.as_ref()) {
            out.push((req, rows(platforms)));
        }
        for arm in self.variants.iter().flatten() {
            if let Some(req) = &arm.runtime_requirement {
                out.push((req, rows(&arm.platforms)));
            }
        }
        out
    }
}

/// The resolved per-variant view of a version entry (spec 28 §3): the
/// shorthand is the one-variant case. The declarative host selection
/// lives here — a consumer never reads the version's `platforms`
/// directly.
#[derive(Debug, Clone)]
pub struct VariantView<'a> {
    /// The derived variant id (spec 28 §2); `None` for the shorthand.
    pub id: Option<String>,
    /// How the no-selector pick chose this arm (§4 rule 1's journal
    /// fields).
    pub picked_by: VariantPick,
    /// The picked arm's runtime requirement mirror (the shorthand's
    /// version-level mirror for the single-variant form).
    pub runtime_requirement: Option<&'a RegistryRuntimeRequirement>,
    pub platforms: &'a RegistryPlatforms,
    /// The universal row's sidecar pin (spec 39 §3): the version-level
    /// pin for the shorthand, the arm-level pin for a universal variant.
    pub blksum: Option<&'a BlksumPin>,
}

/// How [`RegistryVersion::resolve_variant`] picked the arm (spec 28 §4
/// rule 1's `source=` journal field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariantPick {
    /// The shorthand — one variant by construction.
    Shorthand,
    /// The payload's declared `default_variant`.
    Default,
    /// §4 rule 1's fallback: the newest requirement line.
    NewestLine,
    /// A one-arm `variants:` list.
    Only,
}

impl VariantView<'_> {
    /// The declarative host-triplet selection (spec 04 §2): `universal` →
    /// the release's single-`.tfs` rule (no artifact name; the release
    /// ref's own `?sha256=` pin is the digest channel); per-triplet →
    /// `platforms[host]`, `None` when the host triplet is not published.
    pub fn select(&self, host: Platform) -> Option<PlatformSelection<'_>> {
        match self.platforms {
            RegistryPlatforms::Universal => Some(PlatformSelection::Universal),
            RegistryPlatforms::PerTriplet(map) => {
                map.get(&host).map(|e| PlatformSelection::Selected {
                    artifact: e.artifact.as_str(),
                    sha256: e.sha256.as_str(),
                    oci: e.oci.as_deref(),
                    release: e.release.as_ref().map(|r| r.r#ref.as_str()),
                })
            }
        }
    }

    /// The triplets this variant is published for (for the named
    /// platform-missing error); empty for universal rows.
    pub fn published_triplets(&self) -> Vec<Platform> {
        match self.platforms {
            RegistryPlatforms::Universal => Vec::new(),
            RegistryPlatforms::PerTriplet(map) => map.keys().copied().collect(),
        }
    }
}

/// spec 28 §2's derived variant id: `(id, line)` where `line` is the
/// ABI line the id carries (`None` for the `universal` variant — it
/// orders below every line-carrying variant in the §4 rule-1 pick).
///
/// - No requirement → `universal`.
/// - `abi:` with no `implementation` → a named error (spec 28 §10's
///   registry-load gate: an abi is per-implementation by construction).
/// - No `abi:` and no `implementation` → `universal` (a pure-language
///   build serves every implementation its requirement admits — the
///   axis does not fork a pure variant).
/// - `implementation` present (with or without the mirror's abi key —
///   the abi is per-triplet by construction, so a multi-platform arm's
///   mirror omits it): the constraint must be the single ABI-line clause
///   `~> X.Y[.0]` → `<engine>-<implementation>-<X.Y>`; anything else
///   cannot canonize → a named error. (A range constraint with an
///   implementation axis but no abi is a pure-language requirement —
///   `universal`.)
pub fn variant_id(req: Option<&RegistryRuntimeRequirement>) -> Result<String, String> {
    variant_key(req).map(|(id, _)| id)
}

fn variant_key(
    req: Option<&RegistryRuntimeRequirement>,
) -> Result<(String, Option<String>), String> {
    let Some(req) = req else {
        return Ok(("universal".to_string(), None));
    };
    match (&req.implementation, &req.abi) {
        (None, Some(_)) => Err(
            "runtime_requirement carries abi with no implementation — an abi is per-implementation by construction"
                .to_string(),
        ),
        (None, None) => Ok(("universal".to_string(), None)),
        (Some(implementation), abi) => {
            match pessimistic_line(&req.constraint) {
                Some(line) => Ok((
                    format!("{}-{}-{}", req.engine, implementation, line),
                    Some(line),
                )),
                // The ABI-line clause is the native lock's shape. With an
                // abi in force anything else is uncanonizable (spec 28 §2's
                // "anything else" rule); without one the row is a
                // pure-language requirement — the universal variant.
                None if abi.is_some() => Err(format!(
                    "runtime_requirement constraint {:?} cannot canonize to a variant id — the variant grammar keys ABI-line builds (\"~> X.Y[.0]\")",
                    req.constraint
                )),
                None => Ok(("universal".to_string(), None)),
            }
        }
    }
}

/// `~> X.Y` / `~> X.Y.0` → `Some("X.Y")` (the ABI line); anything else
/// → `None`.
fn pessimistic_line(constraint: &str) -> Option<String> {
    let rest = constraint.trim().strip_prefix("~>")?.trim();
    let mut parts = rest.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;
    let patch = parts.next();
    if parts.next().is_some() {
        return None;
    }
    let numeric = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !numeric(major) || !numeric(minor) {
        return None;
    }
    match patch {
        None => Some(format!("{major}.{minor}")),
        Some("0") => Some(format!("{major}.{minor}")),
        _ => None,
    }
}

/// The platforms-map checks, shared by the shorthand and every variant
/// arm: the release-ref class rules (spec 04 §2, spec 38 §2/§3) and the
/// per-row additive mirrors (spec 38 §7's `oci:`, spec 39 §3's `blksum:`,
/// MINOR 6's shard `release:`). `loc` is the map's key path in error
/// messages (`platforms` for the shorthand, `variants[<id>].platforms`
/// per arm).
fn validate_platforms_map(
    payload_name: &str,
    version: &str,
    loc: &str,
    platforms: &RegistryPlatforms,
    release: &Reference,
) -> Result<(), RegistryError> {
    match (platforms, release) {
        (RegistryPlatforms::PerTriplet(map), Reference::Service { sha256, .. }) => {
            if sha256.is_some() {
                return Err(invalid_entry(format!(
                    "payload '{payload_name}' {version} release.ref carries a ?sha256= pin but platforms is per-triplet — the pins live in platforms[<triplet>].sha256"
                )));
            }
            if map.is_empty() {
                return Err(invalid_entry(format!(
                    "payload '{payload_name}' {version} platforms map is empty (use \"universal\")"
                )));
            }
            for (platform, entry) in map {
                if entry.artifact.is_empty() {
                    return Err(invalid_entry(format!(
                        "payload '{payload_name}' {version} {loc}[{platform}].artifact must not be empty"
                    )));
                }
                check_sha256(
                    &format!("payload '{payload_name}' {version} {loc}[{platform}].sha256"),
                    &entry.sha256,
                )?;
            }
        }
        // spec 38 §2/§3: a per-triplet payload published to OCI. The
        // platform rows keep their declarative pins (the sha256 binds
        // the channels — spec 38 §11); the per-triplet TAG derives
        // (<version>-<triplet>, the registry's declarative selection
        // unchanged), so the ref itself must not carry a selector.
        (RegistryPlatforms::PerTriplet(map), Reference::Oci { tag, digest, .. }) => {
            if tag.is_some() || digest.is_some() {
                return Err(invalid_entry(format!(
                    "payload '{payload_name}' {version} release.ref is tfs+oci: with per-triplet platforms but carries a :tag/@digest — the per-triplet tag derives as <version>-<triplet>; name the bare tfs+oci://<host>/<repo>"
                )));
            }
            if map.is_empty() {
                return Err(invalid_entry(format!(
                    "payload '{payload_name}' {version} platforms map is empty (use \"universal\")"
                )));
            }
            for (platform, entry) in map {
                if entry.artifact.is_empty() {
                    return Err(invalid_entry(format!(
                        "payload '{payload_name}' {version} {loc}[{platform}].artifact must not be empty"
                    )));
                }
                check_sha256(
                    &format!("payload '{payload_name}' {version} {loc}[{platform}].sha256"),
                    &entry.sha256,
                )?;
            }
        }
        (RegistryPlatforms::PerTriplet(_), _) => {
            return Err(invalid_entry(format!(
                "payload '{payload_name}' {version} has per-triplet platforms but release.ref is not a service release — artifact names only exist on tfs:<service>: releases"
            )));
        }
        (RegistryPlatforms::Universal, _) => {}
    }
    // The additive per-row OCI mirror (spec 38 §7): when spelled it
    // parses as a `tfs+oci:` reference (the row's artifact at its OCI
    // locator — a mirror of resolution fields, never a second
    // authority).
    if let RegistryPlatforms::PerTriplet(map) = platforms {
        for (platform, entry) in map {
            if let Some(oci) = &entry.oci {
                match Reference::parse(oci) {
                    Ok(Reference::Oci { .. }) => {}
                    Ok(_) => {
                        return Err(invalid_entry(format!(
                            "payload '{payload_name}' {version} {loc}[{platform}].oci is not a tfs+oci: reference"
                        )))
                    }
                    Err(e) => {
                        return Err(invalid_entry(format!(
                            "payload '{payload_name}' {version} {loc}[{platform}].oci does not parse: {e}"
                        )))
                    }
                }
            }
            if let Some(blksum) = &entry.blksum {
                check_blksum_pin(
                    &format!("payload '{payload_name}' {version} {loc}[{platform}].blksum"),
                    blksum,
                )?;
            }
            // The additive per-row shard release ref (spec 04 §2,
            // MINOR 6; tebako#711): the same grammar discipline as the
            // version-level ref under per-triplet platforms — no
            // `#artifact` (artifact selection belongs to THIS map
            // entry), no `?sha256=` pin (the row's `sha256` key is the
            // digest channel), and a class whose releases carry named
            // artifacts (a shard tag names a service release; OCI rows
            // keep the derived <version>-<triplet> tag on the
            // version-level ref).
            if let Some(row_release) = &entry.release {
                match Reference::parse(&row_release.r#ref) {
                    Ok(Reference::Service {
                        artifact: None,
                        sha256: None,
                        ..
                    }) => {}
                    Ok(Reference::Service {
                        artifact: Some(_), ..
                    }) => {
                        return Err(invalid_entry(format!(
                            "payload '{payload_name}' {version} {loc}[{platform}].release.ref carries an #artifact — artifact selection belongs to the platforms map entry"
                        )));
                    }
                    Ok(Reference::Service {
                        sha256: Some(_), ..
                    }) => {
                        return Err(invalid_entry(format!(
                            "payload '{payload_name}' {version} {loc}[{platform}].release.ref carries a ?sha256= pin — the row's sha256 key is the digest channel"
                        )));
                    }
                    Ok(_) => {
                        return Err(invalid_entry(format!(
                            "payload '{payload_name}' {version} {loc}[{platform}].release.ref is not a service release — artifact names only exist on tfs:<service>: releases"
                        )));
                    }
                    Err(e) => {
                        return Err(invalid_entry(format!(
                            "payload '{payload_name}' {version} {loc}[{platform}].release.ref does not parse: {e}"
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

/// The outcome of [`VariantView::select`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformSelection<'a> {
    /// Pure-language payload: the release's single `.tfs` asset (spec 04
    /// §1 no-`#` rule), pinned only when the release ref carries
    /// `?sha256=`.
    Universal,
    /// The host triplet's declared artifact + sha256 pin. `oci` mirrors
    /// the artifact's `tfs+oci:` locator (spec 38 §7) — read only when
    /// the book declares `channel: oci`; the sha256 pin binds both
    /// channels. `release` is the row's additive shard release ref
    /// (MINOR 6; tebako#711): when spelled it names the shard tag
    /// carrying THIS row's bytes, winning over the version-level
    /// `release.ref`.
    Selected {
        artifact: &'a str,
        sha256: &'a str,
        oci: Option<&'a str>,
        release: Option<&'a str>,
    },
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
    /// `tfs+oci://host/repo[:tag|@sha256:digest]` — the registry index
    /// as a spec 38 §3 registry-class artifact (the third location
    /// form). A tag pull rides the dispatch-time cache's 24 h TTL; a
    /// digest pull is the pinned-immutable form (spec 38 §4).
    Oci(Reference),
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
        if input.starts_with("tfs+oci://") {
            // The OCI registry location (spec 38 §4): the index as a
            // registry-class artifact, tag- or digest-pulled.
            let reference = Reference::parse(input).map_err(|e| bad(format!("{e}")))?;
            return Ok(RegistryRef::Oci(reference));
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
            | RegistryRef::Oci(r)
            | RegistryRef::File(r) => r.to_string(),
        }
    }

    /// True when resolution needs the network (TEBAKO_OFFLINE gate; only
    /// `file://` mirrors resolve offline).
    pub fn is_remote(&self) -> bool {
        !matches!(self, RegistryRef::File(_))
    }

    /// The pinned-immutable registry forms (spec 38 §4): a digest-pulled
    /// OCI registry is cached forever, never re-fetched — the digest
    /// names the bytes, the TTL is inapplicable.
    pub fn is_digest_pinned(&self) -> bool {
        matches!(
            self,
            RegistryRef::Oci(Reference::Oci {
                digest: Some(_),
                ..
            })
        )
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
            RegistryRef::Oci(reference) => {
                #[cfg(feature = "oci")]
                {
                    let (bytes, _origin) = crate::oci::fetch_registry_file(
                        &self.transport,
                        reference,
                        alias.as_deref(),
                    )?;
                    Ok(bytes)
                }
                #[cfg(not(feature = "oci"))]
                {
                    let _ = reference;
                    Err(ResolveError::OciAdapterDisabled {
                        reference: r.as_canonical_string(),
                    })
                }
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

    /// [`fetch_registry`] plus, for an OCI registry ref, the manifest
    /// digest the bytes were pulled under (spec 38 §4's dispatch-cache
    /// sidecar — the digest flows out of the pull's own digest-pinned
    /// origin, never a second manifest read; `None` for non-OCI refs).
    pub fn fetch_registry_with_oci_digest(
        &self,
        r: &RegistryRef,
    ) -> Result<(Vec<u8>, Option<String>), ResolveError> {
        let RegistryRef::Oci(reference) = r else {
            return Ok((self.fetch_registry(r)?, None));
        };
        #[cfg(feature = "oci")]
        {
            if crate::cache::offline() {
                return Err(ResolveError::Offline {
                    what: format!("registry {r}"),
                });
            }
            let alias = self.effective_book().alias_of(&r.as_canonical_string());
            let (bytes, origin) =
                crate::oci::fetch_registry_file(&self.transport, reference, alias.as_deref())?;
            let digest = match Reference::parse(&origin) {
                Ok(Reference::Oci { digest, .. }) => digest,
                _ => None,
            };
            Ok((bytes, digest))
        }
        #[cfg(not(feature = "oci"))]
        {
            let _ = reference;
            Err(ResolveError::OciAdapterDisabled {
                reference: r.as_canonical_string(),
            })
        }
    }

    /// spec 38 §4's re-serve check: the manifest digest an OCI registry
    /// ref resolves to right now (None for non-OCI refs) — the dispatch
    /// cache compares it against the digest recorded at fetch time and
    /// re-serves the cached bytes when they agree, saving the blob pull.
    pub fn oci_manifest_digest(&self, r: &RegistryRef) -> Result<Option<String>, ResolveError> {
        #[cfg(feature = "oci")]
        {
            let RegistryRef::Oci(reference) = r else {
                return Ok(None);
            };
            let alias = self.effective_book().alias_of(&r.as_canonical_string());
            crate::oci::resolve_manifest_digest(&self.transport, reference, alias.as_deref())
                .map(Some)
        }
        #[cfg(not(feature = "oci"))]
        {
            let _ = r;
            Ok(None)
        }
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
        let view = v.resolve_variant(None).unwrap();
        assert_eq!(view.id, None);
        assert_eq!(view.picked_by, VariantPick::Shorthand);
        assert_eq!(
            view.select(Platform::Aarch64Macos),
            Some(PlatformSelection::Selected {
                artifact: "metanorma-1.2.3-macos-arm64.tfs",
                sha256: &"b".repeat(64),
                oci: None,
                release: None,
            })
        );
        assert_eq!(
            view.select(Platform::X86_64LinuxGnu),
            Some(PlatformSelection::Selected {
                artifact: "metanorma-1.2.3-linux-gnu-x86_64.tfs",
                sha256: &"a".repeat(64),
                oci: None,
                release: None,
            })
        );
        // a triplet the registry does not publish → None (the caller's
        // named error lists published_triplets)
        assert_eq!(view.select(Platform::X86_64WindowsUcrt), None);
        assert_eq!(
            view.published_triplets(),
            vec![Platform::Aarch64Macos, Platform::X86_64LinuxGnu]
        );
        assert_eq!(v.signature.as_ref().unwrap().keyid, "0123456789abcdef");

        let p = registry.payload("pure-tool").unwrap();
        let v = &p.versions[0];
        assert!(matches!(v.platforms, Some(RegistryPlatforms::Universal)));
        let view = v.resolve_variant(None).unwrap();
        assert_eq!(
            view.select(Platform::X86_64WindowsUcrt),
            Some(PlatformSelection::Universal)
        );
        assert!(p.default_version().is_none());

        // round-trip identity
        let yaml = registry.to_yaml().unwrap();
        let again = Registry::from_yaml(&yaml).unwrap();
        assert_eq!(registry, again);
    }

    #[test]
    fn the_blksum_pin_round_trips_and_torn_pins_are_named() {
        // spec 39 §3's additive row field: the pin anchoring an
        // artifact's `<image>.blksum.json` sidecar — per-triplet rows
        // carry it in the platforms map entry, a universal row at the
        // version level (exactly one location per form).
        let whole = "a".repeat(64);
        let pin_sha = "c".repeat(64);
        let per_triplet = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        platforms:\n          aarch64-macos:\n            artifact: a.tfs\n            sha256: \"{whole}\"\n            blksum: {{filename: a.tfs.blksum.json, sha256: \"{pin_sha}\"}}\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n"
        );
        let registry = Registry::from_yaml(&per_triplet).unwrap();
        let v = registry.payload("x").unwrap().version("1.0").unwrap();
        assert!(v.blksum.is_none());
        let Some(RegistryPlatforms::PerTriplet(map)) = &v.platforms else {
            panic!("per-triplet platforms");
        };
        let pin = map[&Platform::Aarch64Macos].blksum.as_ref().unwrap();
        assert_eq!(pin.filename, "a.tfs.blksum.json");
        assert_eq!(pin.sha256, pin_sha);
        // round-trip identity with the pin present
        let again = Registry::from_yaml(&registry.to_yaml().unwrap()).unwrap();
        assert_eq!(registry, again);

        let universal = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        blksum: {{filename: x-1.0.tfs.blksum.json, sha256: \"{pin_sha}\"}}\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n"
        );
        let registry = Registry::from_yaml(&universal).unwrap();
        let v = registry.payload("x").unwrap().version("1.0").unwrap();
        let pin = v.blksum.as_ref().unwrap();
        assert_eq!(pin.filename, "x-1.0.tfs.blksum.json");
        let again = Registry::from_yaml(&registry.to_yaml().unwrap()).unwrap();
        assert_eq!(registry, again);

        // Torn pins are named errors — the loud "blksum-missing" eager
        // fallback (spec 39 §3) applies to an ABSENT pin, never a
        // broken one.
        let bad_sha = per_triplet.replace(&pin_sha, "zz23");
        let err = Registry::from_yaml(&bad_sha).unwrap_err();
        assert!(
            err.to_string()
                .contains("platforms[aarch64-macos].blksum.sha256 must be 64 lowercase hex"),
            "{err}"
        );
        let empty_name = per_triplet.replace("filename: a.tfs.blksum.json", "filename: \"\"");
        let err = Registry::from_yaml(&empty_name).unwrap_err();
        assert!(
            err.to_string()
                .contains("blksum.filename must not be empty"),
            "{err}"
        );
        // A pin missing a required half inside the platforms map fails
        // at the untagged platforms parse — the same named YAML refusal
        // a missing `artifact`/`sha256` key produces there.
        let missing_half = per_triplet.replace(&format!(", sha256: \"{pin_sha}\"",), "");
        let err = Registry::from_yaml(&missing_half).unwrap_err();
        assert!(matches!(err, RegistryError::Yaml { .. }), "{err}");
        let torn_universal = universal.replace(
            &format!("filename: x-1.0.tfs.blksum.json, sha256: \"{pin_sha}\""),
            "filename: x-1.0.tfs.blksum.json",
        );
        let err = Registry::from_yaml(&torn_universal).unwrap_err();
        assert!(err.to_string().contains("sha256"), "{err}");
        // The version-level spelling on a per-triplet row duplicates the
        // platforms map's job — refused by name.
        let misplaced = per_triplet.replace(
            "        release:",
            &format!("        blksum: {{filename: a.tfs.blksum.json, sha256: \"{pin_sha}\"}}\n        release:"),
        );
        let err = Registry::from_yaml(&misplaced).unwrap_err();
        assert!(
            err.to_string()
                .contains("per-triplet pins live in platforms[<triplet>].blksum"),
            "{err}"
        );
    }

    #[test]
    fn the_per_row_shard_release_ref_round_trips_and_torn_refs_are_named() {
        // spec 04 §2 (MINOR 6; tebako#711): a version line unioning rows
        // from several per-platform shard tags carries the serving tag on
        // the ROW — the version-level ref cannot name the tag a given
        // row's bytes come from.
        let whole = "a".repeat(64);
        let yaml = format!(
            "schema_version: 1\npayloads:\n  - name: ruby\n    kind: runtime\n    engine: ruby\n    versions:\n      - version: 4.0.7-0.17.1\n        platforms:\n          aarch64-macos:\n            artifact: tebako-runtime-0.17.1-ruby-4.0-aarch64-macos.tfs\n            sha256: \"{whole}\"\n            release: {{ref: tfs:github:o/ruby:v0.17.1-ruby4.0-macos}}\n          x86_64-linux-gnu:\n            artifact: tebako-runtime-0.17.1-ruby-4.0-x86_64-linux-gnu.tfs\n            sha256: \"{whole}\"\n        release: {{ref: tfs:github:o/ruby:v0.17.1}}\n"
        );
        let registry = Registry::from_yaml(&yaml).unwrap();
        let v = registry
            .payload("ruby")
            .unwrap()
            .version("4.0.7-0.17.1")
            .unwrap();
        let view = v.resolve_variant(None).unwrap();
        // the shard row flows its own ref …
        assert_eq!(
            view.select(Platform::Aarch64Macos),
            Some(PlatformSelection::Selected {
                artifact: "tebako-runtime-0.17.1-ruby-4.0-aarch64-macos.tfs",
                sha256: &whole,
                oci: None,
                release: Some("tfs:github:o/ruby:v0.17.1-ruby4.0-macos"),
            })
        );
        // … and the unsharded row flows None (the version-level ref
        // serves it).
        assert_eq!(
            view.select(Platform::X86_64LinuxGnu),
            Some(PlatformSelection::Selected {
                artifact: "tebako-runtime-0.17.1-ruby-4.0-x86_64-linux-gnu.tfs",
                sha256: &whole,
                oci: None,
                release: None,
            })
        );
        // round-trip identity with the field present
        let again = Registry::from_yaml(&registry.to_yaml().unwrap()).unwrap();
        assert_eq!(registry, again);

        // Torn spellings are named errors, never silently ignored.
        for (mutated, needle) in [
            (
                yaml.replace("tfs:github:o/ruby:v0.17.1-ruby4.0-macos", "bogus"),
                "platforms[aarch64-macos].release.ref does not parse",
            ),
            (
                yaml.replace(
                    "tfs:github:o/ruby:v0.17.1-ruby4.0-macos",
                    "tfs:github:o/ruby:v0.17.1-ruby4.0-macos#a.tfs",
                ),
                "carries an #artifact",
            ),
            (
                yaml.replace(
                    "tfs:github:o/ruby:v0.17.1-ruby4.0-macos",
                    &format!("tfs:github:o/ruby:v0.17.1-ruby4.0-macos?sha256={whole}"),
                ),
                "carries a ?sha256= pin",
            ),
            (
                yaml.replace("tfs:github:o/ruby:v0.17.1-ruby4.0-macos", "file:///m/a.tfs"),
                "not a service release",
            ),
        ] {
            let err = Registry::from_yaml(&mutated).unwrap_err();
            assert!(
                err.to_string().contains(needle),
                "expected '{needle}' in: {err}"
            );
        }
    }

    #[test]
    fn the_head_signing_block_round_trips_and_torn_blocks_are_named() {
        // spec 09 §9.1 (MINOR 7; tebako#617): the registry head MAY carry
        // the publisher's signing key for the add-registry TOFU pin.
        let yaml = "schema_version: 1\nsigning:\n  key: |\n    -----BEGIN PGP PUBLIC KEY BLOCK-----\n    mDMEAAAA\n    -----END PGP PUBLIC KEY BLOCK-----\n  fingerprint: 9E210CA8E9FDE9E6587740B2EFC3C250F7862A48\n  url: https://example.com/.well-known/tebako-key.asc\npayloads: []\n";
        let registry = Registry::from_yaml(yaml).unwrap();
        let signing = registry.signing.as_ref().unwrap();
        assert_eq!(
            signing.fingerprint,
            "9E210CA8E9FDE9E6587740B2EFC3C250F7862A48"
        );
        assert!(signing.key.contains("BEGIN PGP PUBLIC KEY BLOCK"));
        assert_eq!(
            signing.url.as_deref(),
            Some("https://example.com/.well-known/tebako-key.asc")
        );
        // round-trip identity with the block present
        let again = Registry::from_yaml(&registry.to_yaml().unwrap()).unwrap();
        assert_eq!(registry, again);
        // a block-less document parses to None (the pre-MINOR-7 shape)
        let plain = Registry::from_yaml("schema_version: 1\npayloads: []\n").unwrap();
        assert_eq!(plain.signing, None);
        // Torn blocks are named errors, never silently ignored.
        for (mutated, needle) in [
            (
                yaml.replace(
                    "9E210CA8E9FDE9E6587740B2EFC3C250F7862A48",
                    "efc3c250f7862a48",
                ),
                "signing.fingerprint 'efc3c250f7862a48' is not a 40-hex fingerprint",
            ),
            (
                yaml.replace("    -----BEGIN PGP PUBLIC KEY BLOCK-----\n", ""),
                "signing.key is not an armored PGP public key block",
            ),
            (
                yaml.replace(
                    "https://example.com/.well-known/tebako-key.asc",
                    "http://example.com/key.asc",
                ),
                "signing.url 'http://example.com/key.asc' is not an https URL",
            ),
        ] {
            let err = Registry::from_yaml(&mutated).unwrap_err();
            assert!(
                err.to_string().contains(needle),
                "expected '{needle}' in: {err}"
            );
        }
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

    #[test]
    fn a_version_level_engine_key_is_parse_dropped_never_an_axis() {
        // tebako#549's silent-drop class: the model owns the engine axis
        // at the payload level — a version-level spelling parses away
        // under reader leniency (never a coercion), leaving the entry
        // edge-invisible. The producer gate (tebako registry validate)
        // names the misplaced key; the reader must NOT invent one.
        let yaml = "schema_version: 1\npayloads:\n  - name: x\n    kind: runtime\n    versions:\n      - {version: '1.0', engine: java, platforms: universal, release: {ref: file:///m/a.tfs}}\n";
        let registry = Registry::from_yaml(yaml).unwrap();
        let p = registry.payload("x").unwrap();
        assert_eq!(p.engine(), None);
        assert!(registry.runtime_entries("java", None).is_empty());
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

    // -----------------------------------------------------------------
    // The variant dimension (spec 28 §2/§3/§4 rule 1; schema MINOR 9)
    // -----------------------------------------------------------------

    /// spec 28 §3's two-arm example: the 3.3 line's arm is the
    /// multi-platform mirror (the abi omitted per tebako#440), the 4.0
    /// line's arm a single-platform row (its abi kept — one value holds).
    fn variants_fixture(default_variant: Option<&str>) -> String {
        let sha_a = "a".repeat(64);
        let sha_b = "b".repeat(64);
        let dv = default_variant
            .map(|d| format!("    default_variant: {d}\n"))
            .unwrap_or_default();
        format!(
            "schema_version: 1\npayloads:\n  - name: metanorma\n    kind: app\n    default: 1.16.9\n{dv}    versions:\n      - version: 1.16.9\n        release: {{ref: tfs:github:tebako-packages/metanorma:1.16.9-4}}\n        entrypoints: [metanorma]\n        variants:\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: metanorma-1.16.9-ruby3.3-macos-arm64.tfs, sha256: \"{sha_a}\"}}\n              x86_64-linux-gnu: {{artifact: metanorma-1.16.9-ruby3.3-linux-gnu-x86_64.tfs, sha256: \"{sha_a}\"}}\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 4.0.0\", abi: arm64-darwin-24}}\n            platforms:\n              aarch64-macos: {{artifact: metanorma-1.16.9-ruby4.0-macos-arm64.tfs, sha256: \"{sha_b}\"}}\n"
        )
    }

    #[test]
    fn variants_parse_resolve_and_round_trip() {
        let registry = Registry::from_yaml(&variants_fixture(None)).unwrap();
        let m = registry.payload("metanorma").unwrap();
        let v = m.version("1.16.9").unwrap();
        assert!(
            v.platforms.is_none(),
            "a variants entry carries no shorthand"
        );
        assert_eq!(v.variants.as_ref().unwrap().len(), 2);

        // §4 rule 1's newest-line fallback (no declared default).
        let view = v.resolve_variant(None).unwrap();
        assert_eq!(view.id.as_deref(), Some("ruby-mri-4.0"));
        assert_eq!(view.picked_by, VariantPick::NewestLine);
        assert_eq!(view.runtime_requirement.unwrap().constraint, "~> 4.0.0");
        assert_eq!(
            view.select(Platform::Aarch64Macos),
            Some(PlatformSelection::Selected {
                artifact: "metanorma-1.16.9-ruby4.0-macos-arm64.tfs",
                sha256: &"b".repeat(64),
                oci: None,
                release: None,
            })
        );
        // …scoped to the variant: the 4.0 arm is not published for linux.
        assert_eq!(view.select(Platform::X86_64LinuxGnu), None);
        assert_eq!(view.published_triplets(), vec![Platform::Aarch64Macos]);

        // The declared default wins over the newest-line rule.
        let registry = Registry::from_yaml(&variants_fixture(Some("ruby-mri-3.3"))).unwrap();
        let m = registry.payload("metanorma").unwrap();
        let v = m.version("1.16.9").unwrap();
        let view = v.resolve_variant(m.default_variant.as_deref()).unwrap();
        assert_eq!(view.id.as_deref(), Some("ruby-mri-3.3"));
        assert_eq!(view.picked_by, VariantPick::Default);
        assert_eq!(
            view.published_triplets(),
            vec![Platform::Aarch64Macos, Platform::X86_64LinuxGnu]
        );
        // round-trip identity, variants form included
        let again = Registry::from_yaml(&registry.to_yaml().unwrap()).unwrap();
        assert_eq!(registry, again);
    }

    #[test]
    fn the_variants_shorthand_mece_law_is_named() {
        let sha = "a".repeat(64);
        let row = format!("aarch64-macos: {{artifact: a.tfs, sha256: \"{sha}\"}}");
        // both forms at once
        let both = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        platforms:\n          {row}\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n        variants:\n          - platforms:\n              {row}\n"
        );
        let err = Registry::from_yaml(&both).unwrap_err();
        assert!(err.to_string().contains("both the shorthand"), "{err}");
        // neither form
        let neither = "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        release: {ref: tfs:github:o/x:1.0}\n        entrypoints: [x]\n";
        let err = Registry::from_yaml(neither).unwrap_err();
        assert!(
            err.to_string().contains("neither platforms: nor variants:"),
            "{err}"
        );
        // the shorthand's requirement mirror on a variants entry
        let req_too = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.3\"}}\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n        variants:\n          - platforms:\n              {row}\n"
        );
        let err = Registry::from_yaml(&req_too).unwrap_err();
        assert!(err.to_string().contains("per arm"), "{err}");
        // an empty arms list
        let empty = "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        release: {ref: tfs:github:o/x:1.0}\n        entrypoints: [x]\n        variants: []\n";
        let err = Registry::from_yaml(empty).unwrap_err();
        assert!(err.to_string().contains("lists no arms"), "{err}");
    }

    #[test]
    fn a_duplicate_variant_id_is_a_named_error() {
        // §1's variant law: two arms declaring the same requirement are
        // the same variant — a duplicate key is a named authoring error.
        let sha = "a".repeat(64);
        let yaml = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n        variants:\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: a-3.3.tfs, sha256: \"{sha}\"}}\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            platforms:\n              x86_64-linux-gnu: {{artifact: b-3.3.tfs, sha256: \"{sha}\"}}\n"
        );
        let err = Registry::from_yaml(&yaml).unwrap_err();
        assert!(
            err.to_string()
                .contains("lists the variant 'ruby-mri-3.3' twice"),
            "{err}"
        );
    }

    #[test]
    fn variant_id_derivation_rules() {
        let req = |implementation: Option<&str>, constraint: &str, abi: Option<&str>| {
            RegistryRuntimeRequirement {
                engine: "ruby".to_string(),
                constraint: constraint.to_string(),
                implementation: implementation.map(str::to_string),
                abi: abi.map(str::to_string),
            }
        };
        // no requirement → universal (a data slice)
        assert_eq!(variant_id(None).unwrap(), "universal");
        // a pure-language constraint → universal (the implementation axis
        // does not fork a pure variant)
        assert_eq!(
            variant_id(Some(&req(None, ">= 3.3, < 5.0", None))).unwrap(),
            "universal"
        );
        // … even when the pure requirement names an implementation
        assert_eq!(
            variant_id(Some(&req(Some("jruby"), ">= 9.4, < 10.0", None))).unwrap(),
            "universal"
        );
        // abi with no implementation → §10's registry-load gate
        let err = variant_id(Some(&req(None, "~> 3.3.0", Some("arm64-darwin-23")))).unwrap_err();
        assert!(err.contains("abi with no implementation"), "{err}");
        // the ABI-line clause canonizes — with or without the mirror's
        // abi key (a multi-platform arm omits it, tebako#440)
        assert_eq!(
            variant_id(Some(&req(Some("mri"), "~> 3.3.0", Some("arm64-darwin-23")))).unwrap(),
            "ruby-mri-3.3"
        );
        assert_eq!(
            variant_id(Some(&req(Some("mri"), "~> 4.0", None))).unwrap(),
            "ruby-mri-4.0"
        );
        assert_eq!(
            variant_id(Some(&req(
                Some("jruby"),
                "~> 9.4.0",
                Some("universal-java")
            )))
            .unwrap(),
            "ruby-jruby-9.4"
        );
        // a non-ABI-line constraint with an abi in force cannot canonize
        let err = variant_id(Some(&req(
            Some("mri"),
            ">= 3.3, < 5.0",
            Some("x86_64-linux"),
        )))
        .unwrap_err();
        assert!(err.contains("cannot canonize"), "{err}");
        let err =
            variant_id(Some(&req(Some("mri"), "~> 3.3.5", Some("x86_64-linux")))).unwrap_err();
        assert!(err.contains("cannot canonize"), "{err}");
    }

    #[test]
    fn default_variant_validation_is_named() {
        // naming no variant of the default version
        let err = Registry::from_yaml(&variants_fixture(Some("ruby-mri-9.9"))).unwrap_err();
        assert!(
            err.to_string()
                .contains("default_variant 'ruby-mri-9.9' names no variant of the default version 1.16.9 (variants: ruby-mri-3.3, ruby-mri-4.0)"),
            "{err}"
        );
        // declared without a payload default version
        let no_default =
            variants_fixture(Some("ruby-mri-3.3")).replace("    default: 1.16.9\n", "");
        let err = Registry::from_yaml(&no_default).unwrap_err();
        assert!(err.to_string().contains("pins no default version"), "{err}");
        // the default version carries no variants
        let shorthand = variants_fixture(Some("ruby-mri-3.3")).replace(
            "        variants:\n          - runtime_requirement: {engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}\n            platforms:\n              aarch64-macos: {artifact: metanorma-1.16.9-ruby3.3-macos-arm64.tfs, sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n              x86_64-linux-gnu: {artifact: metanorma-1.16.9-ruby3.3-linux-gnu-x86_64.tfs, sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n          - runtime_requirement: {engine: ruby, implementation: mri, constraint: \"~> 4.0.0\", abi: arm64-darwin-24}\n            platforms:\n              aarch64-macos: {artifact: metanorma-1.16.9-ruby4.0-macos-arm64.tfs, sha256: \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}\n",
            "        platforms: universal\n        runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n",
        );
        let err = Registry::from_yaml(&shorthand).unwrap_err();
        assert!(err.to_string().contains("carries no variants:"), "{err}");
        // and at resolution: a default naming no variant of THIS
        // (non-default) version is a named error, never a fallthrough
        let sha = "a".repeat(64);
        let two_versions = format!(
            "schema_version: 1\npayloads:\n  - name: metanorma\n    kind: app\n    default: 1.16.9\n    default_variant: ruby-mri-3.3\n    versions:\n      - version: 1.16.9\n        release: {{ref: tfs:github:o/m:1.16.9}}\n        entrypoints: [metanorma]\n        variants:\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: m-33.tfs, sha256: \"{sha}\"}}\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 4.0.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: m-40.tfs, sha256: \"{sha}\"}}\n      - version: 1.17.0\n        release: {{ref: tfs:github:o/m:1.17.0}}\n        entrypoints: [metanorma]\n        variants:\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 4.1.0\"}}\n            platforms: universal\n"
        );
        let registry = Registry::from_yaml(&two_versions).unwrap();
        let v = registry
            .payload("metanorma")
            .unwrap()
            .version("1.17.0")
            .unwrap();
        let err = v.resolve_variant(Some("ruby-mri-3.3")).unwrap_err();
        assert!(
            err.to_string()
                .contains("default_variant 'ruby-mri-3.3' names no variant of version 1.17.0"),
            "{err}"
        );
    }

    #[test]
    fn implementation_spanning_variants_need_the_declared_default() {
        // §4 rule 1's authoring gate: the newest-line rule never picks
        // across implementations.
        let sha = "a".repeat(64);
        let yaml = |dv: &str| {
            format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    default: 1.0\n{dv}    versions:\n      - version: 1.0\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n        variants:\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: x-mri.tfs, sha256: \"{sha}\"}}\n          - runtime_requirement: {{engine: ruby, implementation: jruby, constraint: \"~> 9.4.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: x-jruby.tfs, sha256: \"{sha}\"}}\n"
        )
        };
        let err = Registry::from_yaml(&yaml("")).unwrap_err();
        assert!(
            err.to_string()
                .contains("variants span implementations (jruby, mri)"),
            "{err}"
        );
        let registry = Registry::from_yaml(&yaml("    default_variant: ruby-jruby-9.4\n")).unwrap();
        let m = registry.payload("x").unwrap();
        let view = m
            .version("1.0")
            .unwrap()
            .resolve_variant(m.default_variant.as_deref())
            .unwrap();
        assert_eq!(view.id.as_deref(), Some("ruby-jruby-9.4"));
        assert_eq!(view.picked_by, VariantPick::Default);
    }

    #[test]
    fn the_newest_line_pick_orders_universal_below() {
        // A universal arm (a pure-language build) beside a line-carrying
        // arm: the line wins the no-selector pick.
        let sha = "a".repeat(64);
        let yaml = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    default: 1.0\n    versions:\n      - version: 1.0\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n        variants:\n          - platforms:\n              aarch64-macos: {{artifact: x-pure.tfs, sha256: \"{sha}\"}}\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: x-33.tfs, sha256: \"{sha}\"}}\n"
        );
        let registry = Registry::from_yaml(&yaml).unwrap();
        let v = registry.payload("x").unwrap().version("1.0").unwrap();
        let view = v.resolve_variant(None).unwrap();
        assert_eq!(view.id.as_deref(), Some("ruby-mri-3.3"));
        assert_eq!(view.picked_by, VariantPick::NewestLine);
        // a one-arm variants: list resolves to its arm
        let one = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n        variants:\n          - platforms:\n              aarch64-macos: {{artifact: x.tfs, sha256: \"{sha}\"}}\n"
        );
        let registry = Registry::from_yaml(&one).unwrap();
        let v = registry.payload("x").unwrap().version("1.0").unwrap();
        let view = v.resolve_variant(None).unwrap();
        assert_eq!(view.id.as_deref(), Some("universal"));
        assert_eq!(view.picked_by, VariantPick::Only);
    }

    #[test]
    fn variant_arm_blksum_discipline() {
        // spec 39 §3 per arm: the arm-level pin exists iff the arm is
        // universal; on a per-triplet arm the platforms map carries it.
        let sha = "a".repeat(64);
        let pin_sha = "c".repeat(64);
        let ok = format!(
            "schema_version: 1\npayloads:\n  - name: x\n    kind: app\n    versions:\n      - version: 1.0\n        release: {{ref: tfs:github:o/x:1.0}}\n        entrypoints: [x]\n        variants:\n          - platforms: universal\n            blksum: {{filename: x-1.0.tfs.blksum.json, sha256: \"{pin_sha}\"}}\n          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            platforms:\n              aarch64-macos: {{artifact: x-33.tfs, sha256: \"{sha}\"}}\n"
        );
        let registry = Registry::from_yaml(&ok).unwrap();
        let v = registry.payload("x").unwrap().version("1.0").unwrap();
        let arms = v.variants.as_ref().unwrap();
        assert_eq!(
            arms[0].blksum.as_ref().unwrap().filename,
            "x-1.0.tfs.blksum.json"
        );
        let view = v.resolve_variant(None).unwrap();
        assert_eq!(view.id.as_deref(), Some("ruby-mri-3.3"));
        assert!(view.blksum.is_none());

        // a per-triplet arm carrying the arm-level pin → named
        let bad = ok.replace(
            "          - runtime_requirement: {engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}\n            platforms:\n              aarch64-macos: {artifact: x-33.tfs, sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n",
            &format!(
                "          - runtime_requirement: {{engine: ruby, implementation: mri, constraint: \"~> 3.3.0\"}}\n            blksum: {{filename: x-33.tfs.blksum.json, sha256: \"{pin_sha}\"}}\n            platforms:\n              aarch64-macos: {{artifact: x-33.tfs, sha256: \"{sha}\"}}\n"
            ),
        );
        let err = Registry::from_yaml(&bad).unwrap_err();
        assert!(
            err.to_string()
                .contains("variants[ruby-mri-3.3].blksum names the arm's single artifact"),
            "{err}"
        );

        // the version-level pin on a variants entry → named
        let bad = ok.replace(
            "        variants:\n",
            &format!(
                "        blksum: {{filename: x-1.0.tfs.blksum.json, sha256: \"{pin_sha}\"}}\n        variants:\n"
            ),
        );
        let err = Registry::from_yaml(&bad).unwrap_err();
        assert!(
            err.to_string()
                .contains("blksum at the version level has no meaning on a variants entry"),
            "{err}"
        );
    }
}
