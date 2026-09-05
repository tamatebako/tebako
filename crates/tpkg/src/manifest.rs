//! The payload manifest: the in-image self-description every payload
//! carries at [`PAYLOAD_MANIFEST_PATH`] (spec 03 — IDENTITY + PROVIDES +
//! DEPENDS on a common provenance/trust layer).
//!
//! Unlike the trailer (see the crate docs), the payload manifest is
//! **authored YAML**, never JSON (owner rule), versioned by
//! `identity.schema_version` ([`PAYLOAD_SCHEMA_VERSION`]) and pinned by the
//! JSON Schema in `schema/tpkg-manifest-v1.schema.json`. The three tiers do
//! not duplicate authority: this in-image manifest is the rich layer, the
//! trailer stays minimal and REFERENCES manifests by image digest, and the
//! registry mirrors only resolution-relevant fields.
//!
//! Reading is two-step, mirroring the trailer's parse-then-validate
//! discipline: [`PayloadManifest::from_yaml`] does both (serde structure,
//! then [`PayloadManifest::validate`] semantics). Unknown keys are
//! tolerated at every section level for forward compatibility; only
//! `identity.annotations` is guaranteed lossless (free-form k/v, preserved
//! on round-trip).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// Well-known in-image path of the payload manifest (spec 03 §1).
pub const PAYLOAD_MANIFEST_PATH: &str = "/__tpkg__/manifest.yaml";

/// The only `identity.schema_version` this implementation reads and writes.
pub const PAYLOAD_SCHEMA_VERSION: u32 = 1;

/// Error returned by payload-manifest operations.
///
/// Deliberately separate from [`crate::TpkgError`]: `TpkgError`'s codes are
/// 1:1 with the C implementation's `TPKG_ERR_*` values and the payload
/// manifest has no C counterpart.
#[derive(Debug)]
pub enum ManifestError {
    /// YAML parse/serialize failure (structural — the document does not
    /// match the serde model).
    Yaml(serde_yml::Error),
    /// Semantic validation failure (`validate()`).
    Invalid(&'static str),
    /// Semantic validation failure carrying a formatted reason (the
    /// constraint-grammar errors name the offending source string —
    /// versions.rs is the only producer).
    InvalidOwned(String),
    /// The `capabilities.host` jail block failed its own validation
    /// (spec 08 §4 — the reason travels with the jail error).
    Jail(crate::jail::JailError),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Yaml(e) => write!(f, "payload manifest yaml error: {e}"),
            ManifestError::Invalid(m) => write!(f, "invalid payload manifest: {m}"),
            ManifestError::InvalidOwned(m) => write!(f, "invalid payload manifest: {m}"),
            ManifestError::Jail(e) => write!(f, "invalid payload manifest capabilities.host: {e}"),
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ManifestError::Yaml(e) => Some(e),
            ManifestError::Invalid(_) | ManifestError::InvalidOwned(_) => None,
            ManifestError::Jail(e) => Some(e),
        }
    }
}

impl From<serde_yml::Error> for ManifestError {
    fn from(e: serde_yml::Error) -> ManifestError {
        ManifestError::Yaml(e)
    }
}

// ---------------------------------------------------------------------
// Platform axis (spec 03 §3)
// ---------------------------------------------------------------------

/// One platform of the locked vcpkg-triplet axis.
///
/// The SINGLE owner of the triplet ↔ release-asset-name mapping (spec 03
/// §3): dispatcher, release tooling and registry all consume this mapping.
/// `Aarch64WindowsUcrt` is part of the vocabulary but **reserved** — it
/// parses (so documents naming it are diagnosed, not mis-read) while
/// [`PayloadManifest::validate`] rejects its use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Platform {
    Aarch64Macos,
    X86_64Macos,
    X86_64LinuxGnu,
    Aarch64LinuxGnu,
    X86_64LinuxMusl,
    Aarch64LinuxMusl,
    X86_64WindowsUcrt,
    /// Reserved (spec 03 §3): not usable in v1 manifests.
    Aarch64WindowsUcrt,
}

impl Platform {
    /// Every variant, reserved included.
    pub const ALL: [Platform; 8] = [
        Platform::Aarch64Macos,
        Platform::X86_64Macos,
        Platform::X86_64LinuxGnu,
        Platform::Aarch64LinuxGnu,
        Platform::X86_64LinuxMusl,
        Platform::Aarch64LinuxMusl,
        Platform::X86_64WindowsUcrt,
        Platform::Aarch64WindowsUcrt,
    ];

    /// The vcpkg-form triplet (`aarch64-macos`, `x86_64-linux-gnu`, …).
    pub fn as_triplet(self) -> &'static str {
        match self {
            Platform::Aarch64Macos => "aarch64-macos",
            Platform::X86_64Macos => "x86_64-macos",
            Platform::X86_64LinuxGnu => "x86_64-linux-gnu",
            Platform::Aarch64LinuxGnu => "aarch64-linux-gnu",
            Platform::X86_64LinuxMusl => "x86_64-linux-musl",
            Platform::Aarch64LinuxMusl => "aarch64-linux-musl",
            Platform::X86_64WindowsUcrt => "x86_64-windows-ucrt",
            Platform::Aarch64WindowsUcrt => "aarch64-windows-ucrt",
        }
    }

    /// Parse a vcpkg-form triplet (`None` for strings outside the axis).
    pub fn from_triplet(triplet: &str) -> Option<Platform> {
        Platform::ALL
            .iter()
            .copied()
            .find(|p| p.as_triplet() == triplet)
    }

    /// The release-asset-name form (`macos-arm64`, `linux-gnu-x86_64`,
    /// `windows-ucrt64`, …) — the suffix used in published artifact names,
    /// matching `tebako-bootstrap`'s `platform_string()`.
    pub fn release_asset_name(self) -> &'static str {
        match self {
            Platform::Aarch64Macos => "macos-arm64",
            Platform::X86_64Macos => "macos-x86_64",
            Platform::X86_64LinuxGnu => "linux-gnu-x86_64",
            Platform::Aarch64LinuxGnu => "linux-gnu-arm64",
            Platform::X86_64LinuxMusl => "linux-musl-x86_64",
            Platform::Aarch64LinuxMusl => "linux-musl-arm64",
            Platform::X86_64WindowsUcrt => "windows-ucrt64",
            // Reserved; the name follows the same pattern so the mapping
            // stays total in both directions.
            Platform::Aarch64WindowsUcrt => "windows-ucrt-arm64",
        }
    }

    /// Parse a release-asset name (`None` for names outside the axis).
    pub fn from_release_asset_name(name: &str) -> Option<Platform> {
        Platform::ALL
            .iter()
            .copied()
            .find(|p| p.release_asset_name() == name)
    }

    /// True for the reserved triplet (`aarch64-windows-ucrt`, spec 03 §3).
    pub fn is_reserved(self) -> bool {
        self == Platform::Aarch64WindowsUcrt
    }

    /// The Platform of the compile target — the single owner of host
    /// detection. Every "what platform am I" question (bootstrap, shim,
    /// cli) delegates here; unsupported targets fail to compile.
    pub fn host() -> Platform {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return Platform::Aarch64Macos;
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return Platform::X86_64Macos;
        #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
        return Platform::X86_64LinuxGnu;
        #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "aarch64"))]
        return Platform::Aarch64LinuxGnu;
        #[cfg(all(target_os = "linux", target_env = "musl", target_arch = "x86_64"))]
        return Platform::X86_64LinuxMusl;
        #[cfg(all(target_os = "linux", target_env = "musl", target_arch = "aarch64"))]
        return Platform::Aarch64LinuxMusl;
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return Platform::X86_64WindowsUcrt;
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"),
            all(target_os = "linux", target_env = "gnu", target_arch = "aarch64"),
            all(target_os = "linux", target_env = "musl", target_arch = "x86_64"),
            all(target_os = "linux", target_env = "musl", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64")
        )))]
        compile_error!("unsupported platform (outside the spec 03 §3 axis)");
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_triplet())
    }
}

impl FromStr for Platform {
    type Err = ManifestError;
    fn from_str(s: &str) -> Result<Platform, ManifestError> {
        Platform::from_triplet(s).ok_or(ManifestError::Invalid("unknown platform triplet"))
    }
}

impl Serialize for Platform {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_triplet())
    }
}

impl<'de> Deserialize<'de> for Platform {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Platform, D::Error> {
        let s = String::deserialize(d)?;
        Platform::from_triplet(&s)
            .ok_or_else(|| serde::de::Error::custom(format_args!("unknown platform triplet {s:?}")))
    }
}

/// The `platforms` axis (spec 03 §3): EITHER `"universal"` OR an explicit
/// triplet list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platforms {
    /// Pure-ruby / data payloads: runs anywhere.
    Universal,
    /// Explicit vcpkg-form triplet list.
    Triplets(Vec<Platform>),
}

impl Platforms {
    fn validate(&self) -> Result<(), ManifestError> {
        if let Platforms::Triplets(ts) = self {
            if ts.is_empty() {
                return Err(ManifestError::Invalid(
                    "platforms list must not be empty (use \"universal\")",
                ));
            }
            check_platforms(ts)?;
        }
        Ok(())
    }

    /// True when this coverage declaration covers `platform` (universal
    /// covers everything). The compose document's `platforms:` coverage
    /// assertion (spec 23 §13.3) and the lock's per-host digest-pin lookup
    /// both phrase their checks in terms of this.
    pub fn covers(&self, platform: Platform) -> bool {
        match self {
            Platforms::Universal => true,
            Platforms::Triplets(ts) => ts.contains(&platform),
        }
    }
}

/// Shared triplet-list checks (app `platforms`, toolkit-dep `triplets`).
fn check_platforms(ts: &[Platform]) -> Result<(), ManifestError> {
    if ts.iter().any(|p| p.is_reserved()) {
        return Err(ManifestError::Invalid(
            "reserved platform triplet (aarch64-windows-ucrt) is not usable in v1",
        ));
    }
    let mut seen = ts.to_vec();
    seen.sort();
    if seen.windows(2).any(|w| w[0] == w[1]) {
        return Err(ManifestError::Invalid("duplicate platform triplet"));
    }
    Ok(())
}

impl Serialize for Platforms {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Platforms::Universal => s.serialize_str("universal"),
            Platforms::Triplets(ts) => ts.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for Platforms {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Platforms, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Str(String),
            List(Vec<Platform>),
        }
        match Repr::deserialize(d)? {
            Repr::Str(s) if s == "universal" => Ok(Platforms::Universal),
            Repr::Str(s) => Err(serde::de::Error::custom(format_args!(
                "platforms must be \"universal\" or a triplet list, got {s:?}"
            ))),
            Repr::List(ts) => Ok(Platforms::Triplets(ts)),
        }
    }
}

// ---------------------------------------------------------------------
// Version constraint grammar
// ---------------------------------------------------------------------

/// A version constraint (`constraint:`), validated at construction.
///
/// The grammar is deliberately NOT cargo-semver: spec 03 mixes cargo-style
/// ranges (`">= 3.3, < 5.0"`), ruby-style abi lines (`"~> 3.3.0"`) and
/// datever (`">= 2024.1"`). v1 is **parse-only** — the type guarantees the
/// grammar, evaluation against concrete versions lands with the resolver
/// (spec 05). Grammar:
///
/// ```text
/// constraint := clause ("," clause)*
/// clause     := op? version
/// op         := ">=" | "<=" | "~>" | ">" | "<" | "!=" | "="
/// version    := num ("." num){0,3}        # 1..=4 decimal components
/// ```
///
/// The original string is kept verbatim (lossless round-trip).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Constraint(String);

impl Constraint {
    /// Parse and validate a constraint string.
    pub fn new(s: &str) -> Result<Constraint, ManifestError> {
        check_constraint(s)?;
        Ok(Constraint(s.to_string()))
    }

    /// The constraint exactly as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn check_constraint(s: &str) -> Result<(), ManifestError> {
    const OPS: [&str; 7] = [">=", "<=", "~>", ">", "<", "!=", "="];
    if s.trim().is_empty() {
        return Err(ManifestError::Invalid("constraint must not be empty"));
    }
    for clause in s.split(',') {
        let clause = clause.trim();
        if clause.is_empty() {
            return Err(ManifestError::Invalid("constraint has an empty clause"));
        }
        // Longest-operator-first is implicit: every two-char op sorts
        // before its single-char prefix in OPS (">=" ahead of ">", …).
        let version = OPS
            .iter()
            .find_map(|op| clause.strip_prefix(op))
            .unwrap_or(clause)
            .trim();
        let parts: Vec<&str> = version.split('.').collect();
        if parts.is_empty()
            || parts.len() > 4
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(ManifestError::Invalid(
                "constraint clause version must be 1..=4 dot-separated decimals",
            ));
        }
    }
    Ok(())
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Constraint {
    type Err = ManifestError;
    fn from_str(s: &str) -> Result<Constraint, ManifestError> {
        Constraint::new(s)
    }
}

impl Serialize for Constraint {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Constraint {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Constraint, D::Error> {
        let s = String::deserialize(d)?;
        Constraint::new(&s).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------
// Small shared validators
// ---------------------------------------------------------------------

fn check_non_empty(s: &str, what: &'static str) -> Result<(), ManifestError> {
    if s.is_empty() {
        return Err(ManifestError::Invalid(what));
    }
    Ok(())
}

fn check_abs_path(s: &str, what: &'static str) -> Result<(), ManifestError> {
    if !s.starts_with('/') {
        return Err(ManifestError::Invalid(what));
    }
    Ok(())
}

fn check_sha256(s: &str, what: &'static str) -> Result<(), ManifestError> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ManifestError::Invalid(what));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// IDENTITY (spec 03 §2.1 — every payload)
// ---------------------------------------------------------------------

/// The payload kind (spec 03: `app | data | toolkit | runtime | language`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PayloadKind {
    App,
    Data,
    Toolkit,
    Runtime,
    Language,
}

/// Who produced the payload (`producer: {tool, tool_version}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Producer {
    pub tool: String,
    pub tool_version: String,
}

/// Provenance digests (`source:`) — whichever apply to the payload kind:
/// source tarball hash / app commit / builder id. At least one required
/// when the block is present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builder: Option<String>,
}

/// SBOM reference (`sbom: {ref: …}`, optional).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sbom {
    pub r#ref: String,
}

/// The two digests (spec 03 §2.1): `tree_hash` is the plaintext merkle
/// root — the payload's semantic identity (CAS) — rendered
/// `"<algorithm>:<64 lowercase hex>"` because the key does not name an
/// algorithm; `blob_sha256` is the transport identity (bare 64 lowercase
/// hex — the algorithm is in the key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digest {
    pub tree_hash: String,
    pub blob_sha256: String,
}

/// `signing.state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SigningState {
    Unsigned,
    Signed,
}

/// `signing.mechanism` — only OpenPGP in v1 (spec 09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SigningMechanism {
    Openpgp,
}

/// Signing state (spec 03 §2.1). `unsigned` carries no key material;
/// `signed` requires the signer `keyid` (16 lowercase hex — the low 64
/// bits of the OpenPGP fingerprint, as in the trailer's v2 extension) and
/// the `mechanism`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signing {
    pub state: SigningState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanism: Option<SigningMechanism>,
}

/// `encryption.state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncryptionState {
    None,
    Encrypted,
}

/// One encrypted part (spec 03 §2.1: `{paths, algorithm, envelope_refs}` —
/// **NEVER keys**; `envelope_refs` are references to the externally-held
/// grant envelopes of spec 10, never key material).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionPart {
    pub paths: Vec<String>,
    pub algorithm: String,
    pub envelope_refs: Vec<String>,
}

/// Encryption state: `state: none`, or `state: encrypted` with a non-empty
/// per-part list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Encryption {
    pub state: EncryptionState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<EncryptionPart>,
}

/// The IDENTITY block (spec 03 §2.1) — the common provenance/trust layer
/// every payload carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Identity {
    pub schema_version: u32,
    pub kind: PayloadKind,
    pub name: String,
    /// Free-form version string (semver AND datever payloads exist —
    /// e.g. `4.0.6`, `2024.11`); constraints interpret it, not the model.
    pub version: String,
    pub producer: Producer,
    /// Creation timestamp (RFC 3339 rendering; kept as a string — the
    /// model does not interpret time).
    pub created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sbom: Option<Sbom>,
    pub digest: Digest,
    pub signing: Signing,
    pub encryption: Encryption,
    /// Free-form k/v (spec 03: unknown keys preserved on round-trip).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, serde_yml::Value>,
}

impl Identity {
    fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != PAYLOAD_SCHEMA_VERSION {
            return Err(ManifestError::Invalid(
                "identity.schema_version is not supported",
            ));
        }
        check_non_empty(&self.name, "identity.name must not be empty")?;
        check_non_empty(&self.version, "identity.version must not be empty")?;
        check_non_empty(
            &self.producer.tool,
            "identity.producer.tool must not be empty",
        )?;
        check_non_empty(
            &self.producer.tool_version,
            "identity.producer.tool_version must not be empty",
        )?;
        check_non_empty(&self.created, "identity.created must not be empty")?;
        if let Some(source) = &self.source {
            if source.src_sha256.is_none() && source.commit.is_none() && source.builder.is_none() {
                return Err(ManifestError::Invalid(
                    "identity.source must carry at least one digest/id",
                ));
            }
            if let Some(src) = &source.src_sha256 {
                check_sha256(src, "identity.source.src_sha256 must be 64 lowercase hex")?;
            }
        }
        if let Some(sbom) = &self.sbom {
            check_non_empty(&sbom.r#ref, "identity.sbom.ref must not be empty")?;
        }
        check_tree_hash(&self.digest.tree_hash)?;
        check_sha256(
            &self.digest.blob_sha256,
            "identity.digest.blob_sha256 must be 64 lowercase hex",
        )?;
        match self.signing.state {
            SigningState::Unsigned => {
                if self.signing.keyid.is_some() || self.signing.mechanism.is_some() {
                    return Err(ManifestError::Invalid(
                        "identity.signing: unsigned payloads carry neither keyid nor mechanism",
                    ));
                }
            }
            SigningState::Signed => {
                let keyid = self.signing.keyid.as_ref().ok_or(ManifestError::Invalid(
                    "identity.signing: signed payloads require keyid",
                ))?;
                if !crate::envelope::is_valid_keyid(keyid) {
                    return Err(ManifestError::Invalid(
                        "identity.signing.keyid must be 16 lowercase hex (low 64 bits of the OpenPGP fingerprint)",
                    ));
                }
                if self.signing.mechanism.is_none() {
                    return Err(ManifestError::Invalid(
                        "identity.signing: signed payloads require mechanism",
                    ));
                }
            }
        }
        match self.encryption.state {
            EncryptionState::None => {
                if !self.encryption.parts.is_empty() {
                    return Err(ManifestError::Invalid(
                        "identity.encryption: state none carries no parts",
                    ));
                }
            }
            EncryptionState::Encrypted => {
                if self.encryption.parts.is_empty() {
                    return Err(ManifestError::Invalid(
                        "identity.encryption: state encrypted requires a non-empty parts list",
                    ));
                }
                for part in &self.encryption.parts {
                    if part.paths.is_empty() {
                        return Err(ManifestError::Invalid(
                            "identity.encryption parts require a non-empty paths list",
                        ));
                    }
                    for p in &part.paths {
                        check_abs_path(p, "identity.encryption part paths must be absolute")?;
                    }
                    check_non_empty(
                        &part.algorithm,
                        "identity.encryption part algorithm must not be empty",
                    )?;
                    if part.envelope_refs.is_empty()
                        || part.envelope_refs.iter().any(|r| r.is_empty())
                    {
                        return Err(ManifestError::Invalid(
                            "identity.encryption parts require non-empty envelope_refs (references, NEVER keys)",
                        ));
                    }
                }
            }
        }
        if self.annotations.keys().any(|k| k.is_empty()) {
            return Err(ManifestError::Invalid(
                "identity.annotations keys must not be empty",
            ));
        }
        Ok(())
    }
}

fn check_tree_hash(s: &str) -> Result<(), ManifestError> {
    // "<algorithm>:<64 lowercase hex>" — the algorithm is not fixed by the
    // key name (unlike blob_sha256), so the value self-describes.
    let (alg, hex) = s.split_once(':').ok_or(ManifestError::Invalid(
        "identity.digest.tree_hash must be \"<algorithm>:<64 lowercase hex>\"",
    ))?;
    if alg.is_empty()
        || !alg
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return Err(ManifestError::Invalid(
            "identity.digest.tree_hash algorithm must be lowercase alnum",
        ));
    }
    check_sha256(
        hex,
        "identity.digest.tree_hash must be \"<algorithm>:<64 lowercase hex>\"",
    )?;
    Ok(())
}

// ---------------------------------------------------------------------
// PROVIDES (spec 03 §2.2 — kind-specialized)
// ---------------------------------------------------------------------

/// The capability block. The per-kind truth table is locked (spec 03
/// §2.2) and enforced by [`PayloadManifest::validate`]:
/// app `{exec: true, read: true}`, runtime `{exec: true, read: true,
/// runtime: true}`, data `{exec: false, read: true}`.
///
/// `host` (spec 08 §4) is orthogonal to the truth table and may ride any
/// kind: the host access the payload was BUILT TO NEED (e.g. metanorma:
/// read the input file's directory, write the output directory). It is a
/// REQUEST, never a grant to itself — dispatch surfaces compose it with
/// the user's tightening (`manifest request ∩ user policy = effective
/// jail`, spec 08 §2). Absent = the open default (today's behavior).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub exec: bool,
    pub read: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<crate::jail::HostJail>,
}

impl Capabilities {
    /// The spec 08 §4 block validates itself (mount-point shape etc.).
    fn validate_host(&self) -> Result<(), ManifestError> {
        if let Some(host) = &self.host {
            host.validate().map_err(ManifestError::Jail)?;
        }
        Ok(())
    }
}

/// An entrypoint's runtime requirement (`{engine, constraint, abi?}`):
/// a range like `">= 3.3, < 5.0"` for pure payloads, an abi-line like
/// `"~> 3.3.0"` for native-extension ones (spec 03 §2.2). `abi` is the
/// runtime's own platform string the payload's native extensions were
/// built against (ruby: `Gem::Platform.local.to_s`, e.g.
/// `arm64-darwin-23`); present iff the payload carries native extensions
/// — the version line and the platform line are orthogonal constraints
/// and resolution checks both (spec 05 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRequirement {
    pub engine: String,
    pub constraint: Constraint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi: Option<String>,
}

/// One entrypoint of an app (spec 03 §2.2 — the ARRAY allows multi-entry
/// suites; N=1 for simple apps).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entrypoint {
    /// The command name (the shim registers under this).
    pub name: String,
    /// The entrypoint path inside the image.
    pub path: String,
    #[serde(default)]
    pub args_default: Vec<String>,
    /// `None` = a native / self-contained entrypoint: zero-runtime
    /// dispatch (spec 03 §2.2 locked — the dispatcher mounts zero runtime
    /// payloads). Omit the key entirely on the wire for those.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_requirement: Option<RuntimeRequirement>,
    /// The payload's default PATH exposure (spec 03 §2.2 completeness):
    /// `Some(false)` = declared and dispatchable (`tebako shim enable`
    /// links it on demand) but NOT registered at install. Absent/true =
    /// registered. Additive: pre-flag readers ignore the key and register
    /// every entrypoint (the pre-flag behavior).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

impl Entrypoint {
    /// Whether install registers this command by default (spec 03 §2.2).
    pub fn is_active(&self) -> bool {
        self.active != Some(false)
    }
}

/// PROVIDES of kind `app`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppProvides {
    pub entrypoints: Vec<Entrypoint>,
    pub platforms: Platforms,
    pub capabilities: Capabilities,
}

/// One engine a runtime provides (`{engine, version, abi_line,
/// platform, implementation?}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineProvides {
    pub engine: String,
    pub version: String,
    /// The abi line this build satisfies (e.g. `"4.0"`) — what
    /// native-extension payloads match their `"~> x.y.z"` against.
    pub abi_line: String,
    pub platform: Platform,
    /// The engine implementation this build is (spec 28 §8 — e.g. `mri`
    /// / `jruby` / `truffleruby` for engine `ruby`): what a spawned
    /// dependency edge's `implementation` filter matches. Additive —
    /// absent on single-implementation engines and on manifests that
    /// predate the field (the compat window: eligible, never a match
    /// failure of its own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
}

/// Runtime provenance (`built_from: {src_sha256, patch_set}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuiltFrom {
    pub src_sha256: String,
    pub patch_set: String,
}

/// PROVIDES of kind `runtime` (spec 03 §2.2).
///
/// `provides` is one-or-many on the wire: a single `{engine, …}` mapping
/// (the spec's N=1 form) or a list — one entry per platform the image
/// covers (e.g. a universal2 macOS runtime covering both macOS triplets
/// in one image).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeProvides {
    #[serde(with = "one_or_many")]
    pub provides: Vec<EngineProvides>,
    pub built_from: BuiltFrom,
    /// Environment defaults the dispatcher composes into the mount stack.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// The spawn surface (spec 30 §2): the commands this runtime boots as
    /// a separate process for a consumer payload's `runtime` edge
    /// `expose` list. The app-entrypoint grammar minus
    /// `runtime_requirement` (a runtime runs on itself). Additive —
    /// absent/empty = the runtime exposes no spawn commands and serves
    /// only as the primary co-mounted runtime.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrypoints: Vec<Entrypoint>,
    pub capabilities: Capabilities,
}

/// One executable a toolkit provides (`{name, path, version?}`) — a
/// native, zero-runtime program (spec 03 §2.2: toolkit executables
/// never carry a runtime_requirement; the dispatcher materializes and
/// execs them directly).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolkitExecutable {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// One library a toolkit provides (`{name, path}`) — linkable content
/// other payloads consume (informational in v1; the DEPENDS edges carry
/// the consumption).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolkitLibrary {
    pub name: String,
    pub path: String,
}

/// PROVIDES of kind `toolkit`: native executables (zero-runtime
/// dispatch) plus the libraries the toolkit carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolkitProvides {
    /// The shim-registered commands (N>=0 — a pure-library toolkit
    /// registers none).
    #[serde(default)]
    pub executables: Vec<ToolkitExecutable>,
    #[serde(default)]
    pub libraries: Vec<ToolkitLibrary>,
    pub platforms: Platforms,
    pub capabilities: Capabilities,
}

/// Suggested mount semantics of a data payload (`{suggested: …}`).
/// SUGGESTED only — the MOUNT RULE (spec 03 §2.3) gives the mount
/// decision to the consumer's manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountSemantics {
    pub suggested: String,
}

/// PROVIDES of kind `data`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataProvides {
    pub mount_semantics: MountSemantics,
    /// Known consumers (informational reverse edges, e.g. `[metanorma]`).
    #[serde(default)]
    pub consumers: Vec<String>,
    pub capabilities: Capabilities,
}

/// The kind-specialized PROVIDES block. The variant is selected by
/// `identity.kind` at parse time (see `PayloadManifest`'s `Deserialize`
/// impl); `language` has no locked v1 shape and round-trips verbatim as
/// [`Provides::Other`].
#[derive(Debug, Clone, PartialEq)]
pub enum Provides {
    App(AppProvides),
    Runtime(RuntimeProvides),
    Data(DataProvides),
    Toolkit(ToolkitProvides),
    /// `language`: no locked v1 shape — preserved losslessly.
    Other(BTreeMap<String, serde_yml::Value>),
}

impl Provides {
    fn validate(&self) -> Result<(), ManifestError> {
        match self {
            Provides::App(p) => p.validate(),
            Provides::Runtime(p) => p.validate(),
            Provides::Data(p) => p.validate(),
            Provides::Toolkit(p) => p.validate(),
            Provides::Other(_) => Ok(()),
        }
    }
}

impl Serialize for Provides {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Provides::App(p) => p.serialize(s),
            Provides::Runtime(p) => p.serialize(s),
            Provides::Data(p) => p.serialize(s),
            Provides::Toolkit(p) => p.serialize(s),
            Provides::Other(m) => m.serialize(s),
        }
    }
}

impl AppProvides {
    fn validate(&self) -> Result<(), ManifestError> {
        if self.entrypoints.is_empty() {
            return Err(ManifestError::Invalid(
                "provides.entrypoints must not be empty (N>=1)",
            ));
        }
        for ep in &self.entrypoints {
            check_non_empty(&ep.name, "provides.entrypoints[].name must not be empty")?;
            check_abs_path(
                &ep.path,
                "provides.entrypoints[].path must be absolute (inside the image)",
            )?;
            if let Some(req) = &ep.runtime_requirement {
                check_non_empty(
                    &req.engine,
                    "provides.entrypoints[].runtime_requirement.engine must not be empty",
                )?;
                if let Some(abi) = &req.abi {
                    check_non_empty(
                        abi,
                        "provides.entrypoints[].runtime_requirement.abi must not be empty when present",
                    )?;
                }
            }
        }
        self.platforms.validate()?;
        let caps = &self.capabilities;
        if !caps.exec || !caps.read || caps.runtime.is_some() {
            return Err(ManifestError::Invalid(
                "provides.capabilities for kind app is exactly {exec: true, read: true}",
            ));
        }
        caps.validate_host()?;
        Ok(())
    }
}

impl ToolkitProvides {
    fn validate(&self) -> Result<(), ManifestError> {
        for exe in &self.executables {
            check_non_empty(&exe.name, "provides.executables[].name must not be empty")?;
            check_abs_path(
                &exe.path,
                "provides.executables[].path must be absolute (inside the image)",
            )?;
        }
        for lib in &self.libraries {
            check_non_empty(&lib.name, "provides.libraries[].name must not be empty")?;
            check_abs_path(
                &lib.path,
                "provides.libraries[].path must be absolute (inside the image)",
            )?;
        }
        self.platforms.validate()?;
        let caps = &self.capabilities;
        if !caps.read {
            return Err(ManifestError::Invalid(
                "provides.capabilities for kind toolkit must carry read: true",
            ));
        }
        caps.validate_host()?;
        Ok(())
    }
}

impl RuntimeProvides {
    fn validate(&self) -> Result<(), ManifestError> {
        if self.provides.is_empty() {
            return Err(ManifestError::Invalid(
                "provides.provides must not be empty (N>=1)",
            ));
        }
        for ep in &self.provides {
            check_non_empty(&ep.engine, "provides.provides[].engine must not be empty")?;
            check_non_empty(&ep.version, "provides.provides[].version must not be empty")?;
            check_non_empty(
                &ep.abi_line,
                "provides.provides[].abi_line must not be empty",
            )?;
            if let Some(implementation) = &ep.implementation {
                check_non_empty(
                    implementation,
                    "provides.provides[].implementation must not be empty when present",
                )?;
            }
            if ep.platform.is_reserved() {
                return Err(ManifestError::Invalid(
                    "provides.provides[].platform must not be the reserved triplet",
                ));
            }
        }
        for ep in &self.entrypoints {
            check_non_empty(&ep.name, "provides.entrypoints[].name must not be empty")?;
            check_abs_path(
                &ep.path,
                "provides.entrypoints[].path must be absolute (inside the image)",
            )?;
            if ep.runtime_requirement.is_some() {
                return Err(ManifestError::Invalid(
                    "provides.entrypoints[].runtime_requirement is meaningless on a runtime (it runs on itself)",
                ));
            }
        }
        check_sha256(
            &self.built_from.src_sha256,
            "provides.built_from.src_sha256 must be 64 lowercase hex",
        )?;
        check_non_empty(
            &self.built_from.patch_set,
            "provides.built_from.patch_set must not be empty",
        )?;
        if self.env.keys().any(|k| k.is_empty()) {
            return Err(ManifestError::Invalid(
                "provides.env keys must not be empty",
            ));
        }
        let caps = &self.capabilities;
        if !caps.exec || !caps.read || caps.runtime != Some(true) {
            return Err(ManifestError::Invalid(
                "provides.capabilities for kind runtime is exactly {exec: true, read: true, runtime: true}",
            ));
        }
        caps.validate_host()?;
        Ok(())
    }
}

impl DataProvides {
    fn validate(&self) -> Result<(), ManifestError> {
        check_abs_path(
            &self.mount_semantics.suggested,
            "provides.mount_semantics.suggested must be absolute",
        )?;
        if self.consumers.iter().any(|c| c.is_empty()) {
            return Err(ManifestError::Invalid(
                "provides.consumers entries must not be empty",
            ));
        }
        let caps = &self.capabilities;
        if caps.exec || !caps.read || caps.runtime.is_some() {
            return Err(ManifestError::Invalid(
                "provides.capabilities for kind data is exactly {exec: false, read: true}",
            ));
        }
        caps.validate_host()?;
        Ok(())
    }
}

/// Serde helper: one-or-many — a single mapping N=1 (the spec's runtime
/// example) or a list. Serializes a single entry as a mapping, so the
/// N=1 wire shape round-trips.
mod one_or_many {
    use super::EngineProvides;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[EngineProvides], s: S) -> Result<S::Ok, S::Error> {
        if let [one] = v {
            one.serialize(s)
        } else {
            v.serialize(s)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<EngineProvides>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            One(EngineProvides),
            Many(Vec<EngineProvides>),
        }
        Ok(match Repr::deserialize(d)? {
            Repr::One(one) => vec![one],
            Repr::Many(many) => many,
        })
    }
}

// ---------------------------------------------------------------------
// DEPENDS (spec 03 §2.3 — `requires:`)
// ---------------------------------------------------------------------

/// One dependency edge, tagged by `kind`. The MOUNT RULE (locked): the
/// `mount` is declared HERE, in the CONSUMER's manifest — the provider
/// never dictates its mount location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Requirement {
    /// A language-runtime edge (`{kind: language, engine, constraint}`).
    Language {
        engine: String,
        constraint: Constraint,
    },
    /// A native toolkit layer (`{kind: toolkit, name, constraint,
    /// triplets?, mount?}`); `triplets` says where the dep ships.
    Toolkit {
        name: String,
        constraint: Constraint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        triplets: Option<Vec<Platform>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mount: Option<String>,
    },
    /// A data-payload edge (`{kind: data, name, constraint, mount?}`).
    Data {
        name: String,
        constraint: Constraint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mount: Option<String>,
    },
    /// A spawned-runtime edge (`{kind: runtime, engine,
    /// implementation?, constraint, expose?}`) — spec 30 §1
    /// (schema_minor 4): the depended runtime resolves through the
    /// RUNTIME index into the store's runtimes/ area and is NEVER
    /// co-mounted; its exposed entrypoints are spawned through the §2
    /// dispatch. `implementation` narrows the engine axis (spec 28 §8);
    /// `expose` names the depended entries the payload surfaces (the §3
    /// shim surface) — bare command names, like library_aliases names.
    Runtime {
        engine: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        implementation: Option<String>,
        constraint: Constraint,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        expose: Vec<String>,
    },
    /// An executable-capability edge (`{kind: executable, name,
    /// payload?, constraint, mount?, expose?, critical?}`) — spec 03
    /// §8 and spec 32 §1 (schema_minor 5): an executable another
    /// payload PROVIDES, exact-name matched against
    /// `provides.executables` ∪ `provides.entrypoints[].name`. `mount` and `expose` are the two
    /// ORTHOGONAL surfaces: `mount` co-mounts the provider image at the
    /// consumer-declared path (the VFS surface); `expose` opens the
    /// SPAWN surface — each exposed name dispatches the provider's own
    /// spec-17 dispatch as a child process, and an exposed name never
    /// takes the exec-tier path. `payload` is the by-name provider pin
    /// (the AmbiguousProvider escape hatch); `critical` is the schema
    /// evolution law's flag (a reader predating schema_minor 5 must
    /// refuse the edge, never skip it silently).
    Executable {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<String>,
        constraint: Constraint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mount: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        expose: Vec<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        critical: bool,
    },
}

/// Serde helper: `critical: false` is the default and stays off the wire.
fn is_false(b: &bool) -> bool {
    !*b
}

/// The spawn-surface name grammar (spec 30 §1/§3, spec 32 §1 — one owner,
/// both spawn-carrying edge kinds ride it): exposed entries are bare
/// command names (no path separator, no drive qualifier — the
/// library_aliases grammar), never repeated.
fn check_expose_names(expose: &[String]) -> Result<(), ManifestError> {
    let mut seen = std::collections::HashSet::new();
    for e in expose {
        check_non_empty(e, "requires[].expose[] must not be empty")?;
        if e.bytes().any(|b| b == b'/' || b == b'\\' || b == b':') {
            return Err(ManifestError::Invalid(
                "requires[].expose[] must be a bare command name — no path separator, no drive qualifier (spec 30 §1)",
            ));
        }
        if !seen.insert(e) {
            return Err(ManifestError::Invalid(
                "requires[].expose[] must not repeat a command name",
            ));
        }
    }
    Ok(())
}

impl Requirement {
    fn validate(&self) -> Result<(), ManifestError> {
        match self {
            Requirement::Language { engine, .. } => {
                check_non_empty(engine, "requires[].engine must not be empty")?;
            }
            Requirement::Toolkit {
                name,
                triplets,
                mount,
                ..
            } => {
                check_non_empty(name, "requires[].name must not be empty")?;
                if let Some(ts) = triplets {
                    if ts.is_empty() {
                        return Err(ManifestError::Invalid(
                            "requires[].triplets must not be empty when present",
                        ));
                    }
                    check_platforms(ts)?;
                }
                if let Some(m) = mount {
                    check_abs_path(m, "requires[].mount must be absolute (consumer-declared)")?;
                }
            }
            Requirement::Data { name, mount, .. } => {
                check_non_empty(name, "requires[].name must not be empty")?;
                if let Some(m) = mount {
                    check_abs_path(m, "requires[].mount must be absolute (consumer-declared)")?;
                }
            }
            Requirement::Runtime {
                engine,
                implementation,
                expose,
                ..
            } => {
                check_non_empty(engine, "requires[].engine must not be empty")?;
                if let Some(imp) = implementation {
                    check_non_empty(
                        imp,
                        "requires[].implementation must not be empty when present",
                    )?;
                }
                // spec 30 §1/§3: exposed entries are bare command names
                // (the shared spawn-surface grammar).
                check_expose_names(expose)?;
            }
            Requirement::Executable {
                name,
                payload,
                mount,
                expose,
                ..
            } => {
                check_non_empty(name, "requires[].name must not be empty")?;
                if let Some(p) = payload {
                    check_non_empty(p, "requires[].payload must not be empty when present")?;
                }
                if let Some(m) = mount {
                    check_abs_path(m, "requires[].mount must be absolute (consumer-declared)")?;
                }
                // spec 32 §1: mount and expose are the orthogonal axes —
                // an edge declaring NEITHER opens no surface and is a
                // contentless dependency, a named error.
                if mount.is_none() && expose.is_empty() {
                    return Err(ManifestError::Invalid(
                        "requires[] executable edge declares neither mount nor expose — a contentless edge opens no surface (spec 32 §1)",
                    ));
                }
                // spec 32 §1: the spawn-surface grammar is spec 30 §1's,
                // and the depended capability must itself be surfaced.
                check_expose_names(expose)?;
                if !expose.is_empty() && !expose.iter().any(|e| e == name) {
                    return Err(ManifestError::Invalid(
                        "requires[] executable edge with expose requires name ∈ expose — the depended capability must be surfaced (spec 32 §1)",
                    ));
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------
// The manifest itself
// ---------------------------------------------------------------------

/// One `library_aliases:` entry (spec 03 §2.5, additive — schema_minor
/// 2): the exact bare name a loader call presents (`name` — no path
/// separator, no drive qualifier) resolving to the in-image absolute
/// file `path`. The declarative half of the windows Class-L bare-name
/// rule (spec 22 §2.1): a bare name matching no entry is a HOST
/// reference and passes through untouched; matching is verbatim and
/// case-insensitive (the windows loader's own comparison), never
/// extension-completed (`foo` does not match `foo.dll`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryAlias {
    /// The exact bare spelling a loader call presents.
    pub name: String,
    /// The in-image absolute file the name resolves to.
    pub path: String,
}

/// A check's `entry` (spec 26 §1): EITHER the reserved spelling `self` —
/// legal only when `identity.kind == Runtime` (§1.1: the runtime exe
/// itself, with the env image mounted) — OR the absolute in-image path
/// of the executable the check runs. A check never names a host system
/// executable (invariant 1). ABSENT `entry` is not this type's question:
/// it makes the check STRUCTURAL (see [`Check`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckEntry {
    /// `self` — the runtime exe (runtime slices only, spec 26 §1.1).
    SelfExe,
    /// An absolute in-image path.
    Path(String),
}

impl Serialize for CheckEntry {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            CheckEntry::SelfExe => s.serialize_str("self"),
            CheckEntry::Path(p) => s.serialize_str(p),
        }
    }
}

impl<'de> Deserialize<'de> for CheckEntry {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<CheckEntry, D::Error> {
        let s = String::deserialize(d)?;
        Ok(if s == "self" {
            CheckEntry::SelfExe
        } else {
            CheckEntry::Path(s)
        })
    }
}

/// A platform family of a check's `when:` filter (spec 26 §1): the OS
/// family axis (`windows | macos | linux`), NOT the spec 03 §3 triplet
/// axis — a check's filter is behavioral (the engine's per-platform
/// SKIP), never ABI-bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckPlatform {
    Windows,
    Macos,
    Linux,
}

/// One `needs:` entry of a check (spec 26 §1: additive host needs for
/// the check run ONLY — rare): a minimal mirror of the spec 23 §2 D1
/// entry grammar, `{path, access, when?, why?}`. The engine composes
/// them into the run's effective policy; the model only carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckNeed {
    /// Host path (absolute, or a spec 23 §2 symbolic atom).
    pub path: String,
    /// The grant bit — the same `ro | rw` axis as the jail model.
    pub access: crate::jail::JailAccess,
    /// Optional platform filter (the OS family axis).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<CheckPlatform>,
    /// Documentation for the reviewer (spec 23 §2's `why`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// A check's `requires:` block (spec 26 §1): the capabilities the
/// resolved composition must provide. An unmet prerequisite SKIPs the
/// check (loud, the missing capability named), never FAILs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRequires {
    pub provides: Vec<String>,
}

/// A check's `expect:` block (spec 26 §1) — the assertions. Byte-golden
/// assertions do not exist by construction: there is no such field
/// (output bytes churn with dependency versions; the Homebrew test's
/// `assert_path_exists` is the parity bar, invariant 8).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckExpect {
    /// Expected exit status (default 0).
    #[serde(default, skip_serializing_if = "u32_is_zero")]
    pub exit: u32,
    /// Scratch-relative paths asserted to exist and be non-empty after
    /// the run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// One regex the run's stdout must match. Carried UNCOMPILED — tpkg
    /// has no regex dependency by design; the check engine (spec 26 §2)
    /// compiles it at run time. Validated here only for non-emptiness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    /// In-image absolute paths asserted to exist and be non-empty — the
    /// STRUCTURAL check's assertion channel (spec 26 §1.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_files: Vec<String>,
}

impl CheckExpect {
    fn is_default(&self) -> bool {
        self == &CheckExpect::default()
    }
}

fn u32_is_zero(v: &u32) -> bool {
    *v == 0
}

/// One `checks:` entry (spec 26 §1, additive — schema_minor 3): the
/// payload's own acceptance contract, declared in-image — "given my
/// declared needs, I do my one real thing". Any kind may declare checks.
///
/// The shape is decided by ONE key, never a `kind:` flag (MECE, spec 26
/// §1.1): a check WITH `entry` is an EXEC check (the engine runs it
/// under the resolved composition); a check WITHOUT `entry` is a
/// STRUCTURAL check (the data-slice shape — the engine mounts the image
/// and asserts `expect.image_files`, no runtime, no composition).
///
/// The fixture families are MECE across the two `checks:` contexts
/// (spec 26 §2.1): a SLICE manifest speaks the in-image `fixtures`; a
/// COMPOSITION document (which has no image of its own) speaks
/// `fixtures_inline`/`fixtures_host`. Each family is a named validation
/// error in the other's context ([`Check::validate`] vs
/// [`Check::validate_composition`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    /// Exec checks: the in-image executable — or `self` on a runtime
    /// slice (spec 26 §1.1). Absent ⇒ STRUCTURAL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<CheckEntry>,
    /// Exec-check argv. `{scratch}` is the ONE substitution token (the
    /// per-run host scratch directory, spec 26 §2) — at most one
    /// occurrence per entry; every other spelling is literal
    /// passthrough. The engine substitutes; the model only carries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
    /// In-image fixtures directory whose CONTENTS land at the HOST
    /// scratch root (never VFS-spelled — the consumer may be the
    /// payload's own raw-surface component, spec 26 §1). Exec-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixtures: Option<String>,
    /// Composition-check fixture source (spec 26 §2.1): fixture name →
    /// content, written into the scratch root. Valid ONLY in a
    /// composition document's `checks:` block (a composition has no
    /// image of its own); a slice manifest declaring it is a named
    /// validation error — the fixture families are MECE.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fixtures_inline: BTreeMap<String, String>,
    /// Composition-check fixture source (spec 26 §2.1): a path relative
    /// to the composition FILE (the org repo's checked-in fixtures),
    /// copied to scratch. Valid ONLY in a composition document (the same
    /// MECE rule as [`Check::fixtures_inline`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixtures_host: Option<String>,
    /// The assertions (absent = the default `exit: 0`).
    #[serde(default, skip_serializing_if = "CheckExpect::is_default")]
    pub expect: CheckExpect,
    /// Additive host needs for the check run ONLY (spec 23 §2 grammar).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<CheckNeed>,
    /// Composition prerequisites; unmet ⇒ SKIP, never FAIL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<CheckRequires>,
    /// Platform filter (the OS family axis); absent = every platform.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<CheckPlatform>,
    /// Per-check timeout in seconds; expiry is a FAIL (spec 26 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// The check-name grammar (spec 26 §1): names appear in report lines
/// and scratch dir names — `[A-Za-z0-9][A-Za-z0-9._-]*`. Uniqueness is
/// enforced by the map's own deserializer (see `checks_map`). Public so
/// the composition-document model (spec 26 §2.1) validates its own
/// check blocks with the same rule.
pub fn check_check_name(name: &str) -> Result<(), ManifestError> {
    let ok = matches!(name.bytes().next(), Some(b) if b.is_ascii_alphanumeric())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !ok {
        return Err(ManifestError::Invalid(
            "checks[] names must match [A-Za-z0-9][A-Za-z0-9._-]* (they surface in report lines and scratch dir names)",
        ));
    }
    Ok(())
}

/// Serde helper: the checks map refuses a re-declared name — an
/// authoring ambiguity is a named structural error, never a silent
/// winner (the duplicate-alias discipline; serde_yml's plain map read
/// is last-wins, so the refusal lives here, not in `validate`).
/// Public for the other `checks:`-block surface — the spec 26 §2.1
/// composition document (tebako-cli's check engine) deserializes its
/// map through the same refusal.
pub mod checks_map {
    use super::Check;
    use serde::{de, Deserializer};
    use std::collections::BTreeMap;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<String, Check>, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = BTreeMap<String, Check>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a map of check name to check")
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = BTreeMap::new();
                while let Some((name, check)) = map.next_entry::<String, Check>()? {
                    if out.insert(name.clone(), check).is_some() {
                        return Err(de::Error::custom(format_args!(
                            "checks: duplicate check name {name:?} — an authoring ambiguity, never a silent winner"
                        )));
                    }
                }
                Ok(out)
            }
        }
        d.deserialize_map(Visitor)
    }
}

impl Check {
    /// The check context the grammar is validated under (spec 26 §2.1's
    /// MECE fixture rule): a SLICE check speaks the in-image `fixtures`;
    /// a COMPOSITION check speaks `fixtures_inline`/`fixtures_host` (the
    /// composition has no image of its own). Each family is a named
    /// error in the other's context.
    fn validate_in(&self, kind: PayloadKind, composition: bool) -> Result<(), ManifestError> {
        match &self.entry {
            Some(CheckEntry::SelfExe) => {
                // §1.1: the reserved spelling names the runtime exe — a
                // tebako artifact paired with the env image. On any
                // other kind it names nothing.
                if kind != PayloadKind::Runtime {
                    return Err(ManifestError::Invalid(
                        "checks[].entry: \"self\" is reserved for kind runtime (spec 26 §1.1)",
                    ));
                }
            }
            Some(CheckEntry::Path(p)) => {
                check_abs_path(p, "checks[].entry must be absolute (inside the image)")?;
                if p.split('/').any(|component| component == "..") {
                    return Err(ManifestError::Invalid(
                        "checks[].entry must not contain '..' components",
                    ));
                }
            }
            None => {
                // STRUCTURAL (spec 26 §1.1 — the data-slice shape): no
                // exec surface at all, so the exec-only keys are a named
                // error and the only assertion channel is image_files.
                if !self.argv.is_empty() {
                    return Err(ManifestError::Invalid(
                        "checks[].argv is exec-only — a structural check (no entry) declares none",
                    ));
                }
                if self.fixtures.is_some()
                    || !self.fixtures_inline.is_empty()
                    || self.fixtures_host.is_some()
                {
                    return Err(ManifestError::Invalid(
                        "checks[].fixtures/fixtures_inline/fixtures_host are exec-only — a structural check (no entry) declares none",
                    ));
                }
                if self.expect.image_files.is_empty() {
                    return Err(ManifestError::Invalid(
                        "a structural check (no entry) requires a non-empty expect.image_files — its only assertion channel (spec 26 §1.1)",
                    ));
                }
            }
        }
        for arg in &self.argv {
            if arg.matches("{scratch}").count() > 1 {
                return Err(ManifestError::Invalid(
                    "checks[].argv carries the {scratch} token at most once per entry",
                ));
            }
        }
        if let Some(fixtures) = &self.fixtures {
            check_abs_path(
                fixtures,
                "checks[].fixtures must be absolute (inside the image)",
            )?;
            if fixtures.split('/').any(|component| component == "..") {
                return Err(ManifestError::Invalid(
                    "checks[].fixtures must not contain '..' components",
                ));
            }
        }
        // The fixture-family rule (spec 26 §2.1, MECE): `fixtures` is the
        // slice family's in-image source; `fixtures_inline`/`fixtures_host`
        // are the composition family's. Each is a named error in the
        // other's context (never a silent ignore under the unknown-field
        // rule — these keys are KNOWN, and refused here).
        if composition {
            if self.fixtures.is_some() {
                return Err(ManifestError::Invalid(
                    "checks[].fixtures names an in-image dir a composition does not have — a composition check declares fixtures_inline or fixtures_host (spec 26 §2.1)",
                ));
            }
        } else if !self.fixtures_inline.is_empty() || self.fixtures_host.is_some() {
            return Err(ManifestError::Invalid(
                "checks[].fixtures_inline/fixtures_host belong to composition checks (spec 26 §2.1) — a slice check's fixtures are in-image (fixtures:)",
            ));
        }
        for name in self.fixtures_inline.keys() {
            if name.is_empty()
                || name.starts_with('/')
                || name.split('/').any(|c| c == "..")
                || name.split('/').any(|c| c.is_empty())
            {
                return Err(ManifestError::Invalid(
                    "checks[].fixtures_inline names must be non-empty scratch-relative file paths (never absolute, no '..' components)",
                ));
            }
        }
        if let Some(host) = &self.fixtures_host {
            check_non_empty(
                host,
                "checks[].fixtures_host must not be empty (a path relative to the composition file)",
            )?;
            // The absolute spellings of EITHER platform family (POSIX
            // `/…`, windows `X:…`/`\\…`) are refused everywhere — the
            // path is relative to the composition file by contract, and
            // a validator's answer never depends on the host OS.
            let drive_qualified = host.len() >= 2
                && host.as_bytes()[0].is_ascii_alphabetic()
                && host.as_bytes()[1] == b':';
            if host.starts_with('/')
                || host.starts_with('\\')
                || drive_qualified
                || host.split(['/', '\\']).any(|c| c == "..")
            {
                return Err(ManifestError::Invalid(
                    "checks[].fixtures_host must be relative to the composition file (never absolute, no '..' components)",
                ));
            }
        }
        for f in &self.expect.files {
            if f.is_empty() || f.starts_with('/') || f.split('/').any(|c| c == "..") {
                return Err(ManifestError::Invalid(
                    "checks[].expect.files entries must be non-empty scratch-relative paths (never absolute, no '..' components)",
                ));
            }
        }
        if let Some(stdout) = &self.expect.stdout {
            check_non_empty(
                stdout,
                "checks[].expect.stdout must not be empty (one regex the run's stdout must match)",
            )?;
        }
        for f in &self.expect.image_files {
            check_abs_path(
                f,
                "checks[].expect.image_files entries must be absolute (inside the image)",
            )?;
            if f.split('/').any(|component| component == "..") {
                return Err(ManifestError::Invalid(
                    "checks[].expect.image_files entries must not contain '..' components",
                ));
            }
        }
        for need in &self.needs {
            check_non_empty(&need.path, "checks[].needs[].path must not be empty")?;
            if let Some(why) = &need.why {
                check_non_empty(why, "checks[].needs[].why must not be empty when present")?;
            }
        }
        if let Some(requires) = &self.requires {
            if requires.provides.is_empty() || requires.provides.iter().any(|p| p.is_empty()) {
                return Err(ManifestError::Invalid(
                    "checks[].requires.provides must be a non-empty list of non-empty capabilities",
                ));
            }
        }
        if let Some(timeout) = self.timeout {
            if timeout == 0 {
                return Err(ManifestError::Invalid(
                    "checks[].timeout must be positive (seconds)",
                ));
            }
        }
        Ok(())
    }

    /// Slice-context validation (spec 26 §1 — the in-image manifest's
    /// `checks:` block): the grammar with the in-image fixture family.
    fn validate(&self, kind: PayloadKind) -> Result<(), ManifestError> {
        self.validate_in(kind, false)
    }

    /// Composition-context validation (spec 26 §2.1 — a composition
    /// document's `checks:` block): the slice grammar minus the in-image
    /// `fixtures` (the composition has no image), with `entry: self`
    /// still reserved for runtime slices (a composition check names a
    /// mounted slice's executable). Structural composition checks are
    /// legal: the engine asserts `expect.image_files` against the
    /// mounted slice set.
    pub fn validate_composition(&self) -> Result<(), ManifestError> {
        // Any non-runtime kind keeps `self` reserved — the composition
        // document is kind-less, so the runtime-only spelling never
        // binds here.
        self.validate_in(PayloadKind::App, true)
    }
}

/// The payload manifest (spec 03): IDENTITY + PROVIDES + DEPENDS on a
/// common provenance/trust layer, carried at [`PAYLOAD_MANIFEST_PATH`].
///
/// `provides` is kind-specialized — deserializing binds the
/// [`Provides`] variant to `identity.kind` (a mismatch is a structural
/// error, not a validation one); [`PayloadManifest::validate`] re-checks
/// the binding for programmatically built manifests.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PayloadManifest {
    pub identity: Identity,
    pub provides: Provides,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
    /// The spec-22 §4 class-R boot materialization list (additive —
    /// schema_minor 1; old readers ignore it, new readers enforce):
    /// absolute in-image paths of regular files the driver extracts to
    /// the exec cache after the mounts, before the interpreter handoff.
    /// Any kind may declare it (the runtime env image's own resource
    /// default is the canonical entry). Absent = nothing to materialize.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materialize: Vec<String>,
    /// The spec-22 §2.1 windows Class-L bare-name declarations
    /// (additive — schema_minor 2; old readers ignore it, new readers
    /// enforce): the ONLY bare library names resolving to the image's
    /// own files. Any kind may declare; no platform filter (native
    /// images are triplet-bound — an alias is platform surface by
    /// construction). Absent = no declared aliases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub library_aliases: Vec<LibraryAlias>,
    /// The spec-26 §1 payload checks (additive — schema_minor 3; old
    /// readers ignore the key, new readers enforce): the payload's own
    /// acceptance contracts, check-name → [`Check`]. Any kind may
    /// declare; a re-declared name is a named structural error (the
    /// duplicate-alias discipline, never a silent winner). Absent = no
    /// declared checks (an executable kind declaring none is a
    /// press-time lint WARNING — spec 26 §1 — never a manifest error).
    /// Note the map renders sorted on serialize; spec 26 §2's
    /// declaration-order run rule is the ENGINE's read of the in-image
    /// YAML mapping, not this model's storage order.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub checks: BTreeMap<String, Check>,
}

impl<'de> Deserialize<'de> for PayloadManifest {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<PayloadManifest, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            identity: Identity,
            provides: serde_yml::Value,
            #[serde(default)]
            requires: Vec<Requirement>,
            #[serde(default)]
            materialize: Vec<String>,
            #[serde(default)]
            library_aliases: Vec<LibraryAlias>,
            #[serde(default, deserialize_with = "checks_map::deserialize")]
            checks: BTreeMap<String, Check>,
        }
        let raw = Raw::deserialize(d)?;
        let provides = match raw.identity.kind {
            PayloadKind::App => Provides::App(
                AppProvides::deserialize(raw.provides).map_err(serde::de::Error::custom)?,
            ),
            PayloadKind::Runtime => Provides::Runtime(
                RuntimeProvides::deserialize(raw.provides).map_err(serde::de::Error::custom)?,
            ),
            PayloadKind::Data => Provides::Data(
                DataProvides::deserialize(raw.provides).map_err(serde::de::Error::custom)?,
            ),
            PayloadKind::Toolkit => Provides::Toolkit(
                ToolkitProvides::deserialize(raw.provides).map_err(serde::de::Error::custom)?,
            ),
            PayloadKind::Language => Provides::Other(
                BTreeMap::<String, serde_yml::Value>::deserialize(raw.provides)
                    .map_err(serde::de::Error::custom)?,
            ),
        };
        Ok(PayloadManifest {
            identity: raw.identity,
            provides,
            requires: raw.requires,
            materialize: raw.materialize,
            library_aliases: raw.library_aliases,
            checks: raw.checks,
        })
    }
}

impl PayloadManifest {
    /// Parse and validate a payload manifest from YAML text.
    pub fn from_yaml(text: &str) -> Result<PayloadManifest, ManifestError> {
        let manifest: PayloadManifest = serde_yml::from_str(text)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate and serialize to YAML text.
    pub fn to_yaml(&self) -> Result<String, ManifestError> {
        self.validate()?;
        Ok(serde_yml::to_string(self)?)
    }

    /// Semantic checks beyond the serde structure: schema version,
    /// kind ↔ provides binding, the locked capability truth tables,
    /// digest/keyid shapes, signing/encryption state consistency, the
    /// reserved-triplet rule, the absolute-path rules, the
    /// materialize grammar (spec 22 §4), the library_aliases
    /// grammar (spec 03 §2.5 / spec 22 §2.1), and the checks grammar
    /// (spec 26 §1: name grammar, exec/structural split, `entry: self`
    /// kind binding, path rules, platform filter). Unknown keys are
    /// tolerated everywhere (only `annotations` is lossless by contract).
    pub fn validate(&self) -> Result<(), ManifestError> {
        self.identity.validate()?;
        let bound = matches!(
            (self.identity.kind, &self.provides),
            (PayloadKind::App, Provides::App(_))
                | (PayloadKind::Runtime, Provides::Runtime(_))
                | (PayloadKind::Data, Provides::Data(_))
                | (PayloadKind::Toolkit, Provides::Toolkit(_))
                | (PayloadKind::Language, Provides::Other(_))
        );
        if !bound {
            return Err(ManifestError::Invalid(
                "provides does not match identity.kind",
            ));
        }
        self.provides.validate()?;
        for req in &self.requires {
            req.validate()?;
        }
        // spec 30 §3, extended one class by spec 32 §1: an exposed
        // depended-entry name never collides with the payload's OWN
        // entrypoints — a named error at press, whether the exposing edge
        // is a runtime edge or an executable edge (this cross-field rule
        // is the model's; the JSON Schema cannot express the set
        // intersection).
        if let Provides::App(app) = &self.provides {
            for req in &self.requires {
                let expose = match req {
                    Requirement::Runtime { expose, .. } => expose,
                    Requirement::Executable { expose, .. } => expose,
                    _ => continue,
                };
                for e in expose {
                    if app.entrypoints.iter().any(|ep| &ep.name == e) {
                        return Err(ManifestError::Invalid(
                            "requires[].expose[] collides with the payload's own entrypoint name (spec 30 §3)",
                        ));
                    }
                }
            }
        }
        for p in &self.materialize {
            check_abs_path(p, "materialize[] must be absolute (inside the image)")?;
            if p.split('/').any(|component| component == "..") {
                return Err(ManifestError::Invalid(
                    "materialize[] must not contain '..' components (the extraction target derives from the entry)",
                ));
            }
        }
        for (i, alias) in self.library_aliases.iter().enumerate() {
            check_non_empty(&alias.name, "library_aliases[].name must be non-empty")?;
            // The bare-name grammar (spec 03 §2.5): no path separator,
            // no drive qualifier. Rejecting ':' covers the drive form
            // ('C:foo.dll') and every other qualified spelling.
            if alias
                .name
                .bytes()
                .any(|b| b == b'/' || b == b'\\' || b == b':')
            {
                return Err(ManifestError::Invalid(
                    "library_aliases[].name must be a bare name — no path separator, no drive qualifier",
                ));
            }
            check_abs_path(
                &alias.path,
                "library_aliases[].path must be absolute (inside the image)",
            )?;
            if alias.path.split('/').any(|component| component == "..") {
                return Err(ManifestError::Invalid(
                    "library_aliases[].path must not contain '..' components (the materialization target derives from the entry)",
                ));
            }
            // A duplicate name within one image is an authoring
            // ambiguity — a named manifest error, never a silent winner
            // (the comparison is the match rule: verbatim,
            // case-insensitive).
            if self.library_aliases[..i]
                .iter()
                .any(|prior| prior.name.eq_ignore_ascii_case(&alias.name))
            {
                return Err(ManifestError::Invalid(
                    "library_aliases[] declares a duplicate name (case-insensitive) — an authoring ambiguity, never a silent winner",
                ));
            }
        }
        for (name, check) in &self.checks {
            check_check_name(name)?;
            check.validate(self.identity.kind)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(c: u8) -> String {
        // deterministic 64-hex placeholder
        (0..64)
            .map(|i| b"0123456789abcdef"[((c + i as u8) % 16) as usize] as char)
            .collect()
    }

    #[test]
    fn platform_triplet_roundtrip() {
        for p in Platform::ALL {
            assert_eq!(Platform::from_triplet(p.as_triplet()), Some(p));
        }
        assert_eq!(Platform::from_triplet("x86_64-unknown-linux-gnu"), None);
        assert_eq!(Platform::from_triplet("universal"), None);
        assert_eq!(Platform::from_triplet(""), None);
    }

    #[test]
    fn entrypoint_active_flag_defaults_true_and_roundtrips() {
        // spec 03 §2.2: absent/true = registered at install; explicit
        // false = declared-but-inactive. The key is additive on the wire
        // (a pre-flag reader ignores it — serde's unknown-field default).
        let text = format!(
            "identity:\n  schema_version: 1\n  kind: app\n  name: x\n  version: 1.0.0\n\
             \x20 producer: {{tool: t, tool_version: 1}}\n  created: now\n\
             \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
             \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  entrypoints:\n    - name: metanorma\n      path: /bin/metanorma\n\
             \x20   - name: fontist\n      path: /bin/fontist\n      active: false\n\
             \x20 platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
            sha(1),
            sha(2),
        );
        let m = PayloadManifest::from_yaml(&text).unwrap();
        let Provides::App(app) = &m.provides else {
            panic!("expected app provides");
        };
        assert!(app.entrypoints[0].is_active());
        assert!(!app.entrypoints[1].is_active());
        assert_eq!(app.entrypoints[1].active, Some(false));

        // the wire form omits the key unless explicitly false
        let out = serde_yml::to_string(app).unwrap();
        assert!(!out.contains("active: true"), "{out}");
        assert!(out.contains("active: false"), "{out}");
    }

    #[test]
    fn platform_release_asset_name_roundtrip() {
        for p in Platform::ALL {
            assert_eq!(
                Platform::from_release_asset_name(p.release_asset_name()),
                Some(p)
            );
        }
        // the locked examples (spec 03 §3)
        assert_eq!(
            Platform::from_release_asset_name("macos-arm64"),
            Some(Platform::Aarch64Macos)
        );
        assert_eq!(
            Platform::from_release_asset_name("linux-gnu-x86_64"),
            Some(Platform::X86_64LinuxGnu)
        );
        assert_eq!(
            Platform::from_release_asset_name("windows-ucrt64"),
            Some(Platform::X86_64WindowsUcrt)
        );
        assert_eq!(Platform::from_release_asset_name("windows-x86_64"), None);
        assert_eq!(Platform::from_release_asset_name("linux-gnu-aarch64"), None);
    }

    #[test]
    fn platform_reserved() {
        assert!(Platform::Aarch64WindowsUcrt.is_reserved());
        assert!(!Platform::X86_64WindowsUcrt.is_reserved());
    }

    #[test]
    fn platform_host_is_on_the_axis() {
        let h = Platform::host();
        assert!(Platform::ALL.contains(&h));
        assert!(!h.is_reserved());
    }

    #[test]
    fn platform_serde_is_triplet_string() {
        let y = serde_yml::to_string(&Platform::Aarch64Macos).unwrap();
        assert_eq!(y.trim(), "aarch64-macos");
        let p: Platform = serde_yml::from_str("x86_64-linux-musl").unwrap();
        assert_eq!(p, Platform::X86_64LinuxMusl);
        assert!(serde_yml::from_str::<Platform>("riscv64-linux-gnu").is_err());
    }

    #[test]
    fn platforms_axis_serde() {
        let u: Platforms = serde_yml::from_str("universal").unwrap();
        assert_eq!(u, Platforms::Universal);
        assert_eq!(serde_yml::to_string(&u).unwrap().trim(), "universal");
        let l: Platforms = serde_yml::from_str("[aarch64-macos, x86_64-linux-gnu]").unwrap();
        assert_eq!(
            l,
            Platforms::Triplets(vec![Platform::Aarch64Macos, Platform::X86_64LinuxGnu])
        );
        assert!(serde_yml::from_str::<Platforms>("everywhere").is_err());
    }

    #[test]
    fn constraint_accepts_spec_examples() {
        for ok in [
            ">= 3.3, < 5.0",
            "~> 3.3.0",
            ">= 3.24, < 3.25",
            ">= 2024.1",
            "4.0.6",
            "= 1.2.3.4",
            "!= 3.0.0",
            ">2,<3",
        ] {
            Constraint::new(ok).unwrap_or_else(|e| panic!("{ok:?} should parse: {e}"));
        }
    }

    #[test]
    fn constraint_rejects_garbage() {
        for bad in [
            "",
            "   ",
            ">= , < 5.0",
            ">= 3.3,",
            ", >= 3.3",
            "=> 3.3",
            "3..3",
            "3.3.3.3.3",
            "~>",
            ">= abc",
            "3.3-rc1",
        ] {
            assert!(Constraint::new(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn constraint_serde_is_verbatim_string() {
        let c = Constraint::new(">= 3.3, < 5.0").unwrap();
        let y = serde_yml::to_string(&c).unwrap();
        let back: Constraint = serde_yml::from_str(&y).unwrap();
        assert_eq!(back.as_str(), ">= 3.3, < 5.0");
        let c2: Constraint = serde_yml::from_str("\"~> 3.3.0\"").unwrap();
        assert_eq!(c2.as_str(), "~> 3.3.0");
        assert!(serde_yml::from_str::<Constraint>("\"3..3\"").is_err());
    }

    fn minimal_identity(kind: PayloadKind) -> Identity {
        Identity {
            schema_version: PAYLOAD_SCHEMA_VERSION,
            kind,
            name: "x".to_string(),
            version: "1.0.0".to_string(),
            producer: Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.16.0".to_string(),
            },
            created: "2026-07-26T00:00:00Z".to_string(),
            source: None,
            sbom: None,
            digest: Digest {
                tree_hash: format!("sha256:{}", sha(3)),
                blob_sha256: sha(7),
            },
            signing: Signing {
                state: SigningState::Unsigned,
                keyid: None,
                mechanism: None,
            },
            encryption: Encryption {
                state: EncryptionState::None,
                parts: Vec::new(),
            },
            annotations: BTreeMap::new(),
        }
    }

    #[test]
    fn runtime_provides_accepts_one_or_many() {
        let one: RuntimeProvides = serde_yml::from_str(
            "provides: {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: x86_64-linux-gnu}\n\
             built_from: {src_sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", patch_set: \"v0.2.8\"}\n\
             capabilities: {exec: true, read: true, runtime: true}\n",
        )
        .unwrap();
        assert_eq!(one.provides.len(), 1);
        // N=1 serializes back as a mapping (the spec's wire shape)
        let y = serde_yml::to_string(&one).unwrap();
        assert!(y.contains("provides:\n  engine: ruby"), "{y}");
        let two: RuntimeProvides = serde_yml::from_str(
            "provides:\n\
             \x20 - {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: aarch64-macos}\n\
             \x20 - {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: x86_64-macos}\n\
             built_from: {src_sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", patch_set: \"v0.2.8\"}\n\
             capabilities: {exec: true, read: true, runtime: true}\n",
        )
        .unwrap();
        assert_eq!(two.provides.len(), 2);
    }

    #[test]
    fn unquoted_timestamp_and_dates_stay_strings() {
        // YAML 1.2 core schema: timestamps are NOT resolved, so the spec's
        // unquoted `created: 2026-07-26T00:00:00Z` parses as a string.
        let v: serde_yml::Value = serde_yml::from_str("2026-07-26T00:00:00Z").unwrap();
        assert!(matches!(v, serde_yml::Value::String(_)), "{v:?}");
        // …but a two-component datever like 2024.11 IS a float — authors
        // must quote it (identity.version is a string).
        let v: serde_yml::Value = serde_yml::from_str("2024.11").unwrap();
        assert!(matches!(v, serde_yml::Value::Number(_)), "{v:?}");
    }

    #[test]
    fn kind_binds_provides_variant_at_parse() {
        let mk = |provides: &str| {
            format!(
                "identity:\n  schema_version: 1\n  kind: data\n  name: x\n  version: 1.0.0\n\
                 \x20 producer: {{tool: t, tool_version: 1}}\n  created: now\n\
                 \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
                 \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
                 provides:\n{provides}",
                sha(1),
                sha(2),
            )
        };
        // kind data with a data-shaped provides parses…
        let ok = mk("  mount_semantics: {suggested: /usr/share/fonts}\n  capabilities: {exec: false, read: true}\n");
        let m: PayloadManifest = serde_yml::from_str(&ok).unwrap();
        assert!(matches!(m.provides, Provides::Data(_)));
        // …kind data with an app-shaped provides is a structural error.
        let bad = mk(
            "  entrypoints: []\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        assert!(serde_yml::from_str::<PayloadManifest>(&bad).is_err());
    }

    #[test]
    fn validate_rejects_kind_provides_mismatch_when_built_programmatically() {
        let m = PayloadManifest {
            identity: minimal_identity(PayloadKind::App),
            provides: Provides::Other(BTreeMap::new()),
            requires: Vec::new(),
            materialize: Vec::new(),
            library_aliases: Vec::new(),
            checks: BTreeMap::new(),
        };
        assert!(matches!(m.validate(), Err(ManifestError::Invalid(_))));
    }

    #[test]
    fn validate_capability_truth_tables() {
        let caps = |exec, read, runtime| Capabilities {
            exec,
            read,
            runtime,
            host: None,
        };
        let base = |c: Capabilities| AppProvides {
            entrypoints: vec![Entrypoint {
                name: "x".into(),
                path: "/x".into(),
                args_default: vec![],
                runtime_requirement: Some(RuntimeRequirement {
                    engine: "ruby".into(),
                    constraint: Constraint::new(">= 3.3, < 5.0").unwrap(),
                    abi: None,
                }),
                active: None,
            }],
            platforms: Platforms::Universal,
            capabilities: c,
        };
        assert!(base(caps(true, true, None)).validate().is_ok());
        assert!(base(caps(false, true, None)).validate().is_err());
        assert!(base(caps(true, true, Some(true))).validate().is_err());
    }

    #[test]
    fn capabilities_host_block_round_trips_and_validates() {
        // spec 08 §4: capabilities.host rides the truth-table kinds as the
        // payload's declared host-access REQUEST.
        let mut app = AppProvides {
            entrypoints: vec![Entrypoint {
                name: "x".into(),
                path: "/x".into(),
                args_default: vec![],
                runtime_requirement: Some(RuntimeRequirement {
                    engine: "ruby".into(),
                    constraint: Constraint::new(">= 3.3, < 5.0").unwrap(),
                    abi: None,
                }),
                active: None,
            }],
            platforms: Platforms::Universal,
            capabilities: Capabilities {
                exec: true,
                read: true,
                runtime: None,
                host: Some(crate::jail::HostJail::from_yaml(
                    "default: deny\nmounts: [{host: /work, mount: /work, access: rw}]\nargument_files: auto-allowed\n",
                ).unwrap()),
            },
        };
        app.validate().unwrap();
        let y = serde_yml::to_string(&app).unwrap();
        assert!(y.contains("host:"), "{y}");
        assert!(y.contains("default: deny"), "{y}");
        let back: AppProvides = serde_yml::from_str(&y).unwrap();
        assert_eq!(back, app);
        // A bad jail block surfaces as the Jail error kind.
        app.capabilities.host = Some(crate::jail::HostJail {
            mounts: vec![crate::jail::JailMount {
                host: "/h".into(),
                mount: "relative".into(),
                access: crate::jail::JailAccess::Ro,
            }],
            ..crate::jail::HostJail::deny()
        });
        assert!(matches!(app.validate(), Err(ManifestError::Jail(_))));
    }

    #[test]
    fn validate_signing_and_encryption_states() {
        let mut id = minimal_identity(PayloadKind::Data);
        id.signing.keyid = Some("0123456789abcdef".into());
        assert!(id.validate().is_err()); // unsigned must not carry keyid
        id.signing.state = SigningState::Signed;
        assert!(id.validate().is_err()); // signed requires mechanism
        id.signing.mechanism = Some(SigningMechanism::Openpgp);
        assert!(id.validate().is_ok());
        id.signing.keyid = Some("0123456789abcdeg".into());
        assert!(id.validate().is_err()); // non-hex keyid
        id.signing.keyid = Some("0123456789abcdef".into());

        id.encryption.parts = vec![EncryptionPart {
            paths: vec!["/secret".into()],
            algorithm: "age-x25519".into(),
            envelope_refs: vec!["vault:x#1".into()],
        }];
        assert!(id.validate().is_err()); // state none carries no parts
        id.encryption.state = EncryptionState::Encrypted;
        assert!(id.validate().is_ok());
        id.encryption.parts[0].paths = vec!["relative".into()];
        assert!(id.validate().is_err());
    }

    /// A minimal kind-data document with `extra` appended verbatim (the
    /// materialize tests' vehicle).
    fn minimal_data_yaml(extra: &str) -> String {
        format!(
            "identity:\n  schema_version: 1\n  kind: data\n  name: x\n  version: 1.0.0\n\
             \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: now\n\
             \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
             \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  mount_semantics: {{suggested: /usr/share/x}}\n  capabilities: {{exec: false, read: true}}\n{extra}",
            sha(1),
            sha(2),
        )
    }

    #[test]
    fn materialize_defaults_empty_and_round_trips() {
        // spec 22 §4 class R (schema_minor 1): the additive boot-
        // materialization key — absolute in-image paths the driver
        // extracts to the exec cache before the interpreter handoff.
        let bare = PayloadManifest::from_yaml(&minimal_data_yaml("")).unwrap();
        assert!(bare.materialize.is_empty());
        // An absent key never serializes (additive on the wire: old
        // readers see the document they always saw).
        assert!(!bare.to_yaml().unwrap().contains("materialize"));

        let with = PayloadManifest::from_yaml(&minimal_data_yaml(
            "materialize: [/lib/tebako/cacert.pem, /share/icu/icudt.dat]\n",
        ))
        .unwrap();
        assert_eq!(
            with.materialize,
            vec![
                "/lib/tebako/cacert.pem".to_string(),
                "/share/icu/icudt.dat".to_string()
            ]
        );
        let rendered = with.to_yaml().unwrap();
        assert!(rendered.contains("materialize:"), "{rendered}");
        let back = PayloadManifest::from_yaml(&rendered).unwrap();
        assert_eq!(back, with);
    }

    #[test]
    fn materialize_entries_must_be_absolute_and_escape_free() {
        // A relative entry is a named validation error.
        let err = PayloadManifest::from_yaml(&minimal_data_yaml("materialize: [lib/x.pem]\n"))
            .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("materialize")),
            "{err}"
        );
        // A '..' component would escape the exec-cache resources
        // namespace on the host — rejected at validation, never at write.
        let err = PayloadManifest::from_yaml(&minimal_data_yaml("materialize: [/../../host/x]\n"))
            .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("materialize")),
            "{err}"
        );
        // A scalar is a structural error, never a one-item list.
        let err = PayloadManifest::from_yaml(&minimal_data_yaml("materialize: /x\n")).unwrap_err();
        assert!(matches!(err, ManifestError::Yaml(_)), "{err}");
    }

    #[test]
    fn library_aliases_default_empty_and_round_trip() {
        // spec 03 §2.5 (schema_minor 2): the additive windows Class-L
        // bare-name declarations — name → in-image file.
        let bare = PayloadManifest::from_yaml(&minimal_data_yaml("")).unwrap();
        assert!(bare.library_aliases.is_empty());
        // An absent key never serializes (additive on the wire: old
        // readers see the document they always saw).
        assert!(!bare.to_yaml().unwrap().contains("library_aliases"));

        let with = PayloadManifest::from_yaml(&minimal_data_yaml(
            "library_aliases:\n  - {name: libfoo-3.dll, path: /lib/libfoo-3.dll}\n  - {name: bar.dll, path: /vendor/bar.dll}\n",
        ))
        .unwrap();
        assert_eq!(
            with.library_aliases,
            vec![
                LibraryAlias {
                    name: "libfoo-3.dll".to_string(),
                    path: "/lib/libfoo-3.dll".to_string(),
                },
                LibraryAlias {
                    name: "bar.dll".to_string(),
                    path: "/vendor/bar.dll".to_string(),
                },
            ]
        );
        let rendered = with.to_yaml().unwrap();
        assert!(rendered.contains("library_aliases:"), "{rendered}");
        let back = PayloadManifest::from_yaml(&rendered).unwrap();
        assert_eq!(back, with);
    }

    #[test]
    fn library_alias_names_must_be_bare() {
        // The bare-name grammar (spec 03 §2.5): no path separator, no
        // drive qualifier — a named validation error, never a guess.
        for bad in [
            "lib/foo.dll",
            "lib\\foo.dll",
            "C:foo.dll",
            "C:\\lib\\foo.dll",
        ] {
            // Single-quoted YAML scalars: a backslash stays literal
            // (double quotes would eat it as an escape).
            let err = PayloadManifest::from_yaml(&minimal_data_yaml(&format!(
                "library_aliases:\n  - {{name: '{bad}', path: /lib/foo.dll}}\n"
            )))
            .unwrap_err();
            assert!(
                matches!(err, ManifestError::Invalid(m) if m.contains("library_aliases")),
                "{bad}: {err}"
            );
        }
        // An empty name is a structural nothing.
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "library_aliases:\n  - {name: \"\", path: /lib/foo.dll}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("library_aliases")),
            "{err}"
        );
        // A missing name or path key is a structural error.
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "library_aliases:\n  - {path: /lib/foo.dll}\n",
        ))
        .unwrap_err();
        assert!(matches!(err, ManifestError::Yaml(_)), "{err}");
    }

    #[test]
    fn library_alias_paths_must_be_absolute_and_escape_free() {
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "library_aliases:\n  - {name: foo.dll, path: lib/foo.dll}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("library_aliases")),
            "{err}"
        );
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "library_aliases:\n  - {name: foo.dll, path: /../../host/foo.dll}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("library_aliases")),
            "{err}"
        );
    }

    #[test]
    fn library_alias_duplicate_names_are_a_named_error() {
        // A duplicate within one image — on the match rule's own
        // comparison (verbatim, case-insensitive) — is an authoring
        // ambiguity, never a silent winner (spec 03 §2.5).
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "library_aliases:\n  - {name: Foo.dll, path: /lib/a.dll}\n  - {name: foo.DLL, path: /lib/b.dll}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("duplicate")),
            "{err}"
        );
    }

    /// A minimal kind-app document with `extra` appended verbatim (the
    /// checks tests' vehicle).
    fn minimal_app_yaml(extra: &str) -> String {
        format!(
            "identity:\n  schema_version: 1\n  kind: app\n  name: x\n  version: 1.0.0\n\
             \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: now\n\
             \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
             \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  entrypoints: [{{name: x, path: /x}}]\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n{extra}",
            sha(1),
            sha(2),
        )
    }

    /// A minimal kind-runtime document with `extra` appended verbatim.
    fn minimal_runtime_yaml(extra: &str) -> String {
        format!(
            "identity:\n  schema_version: 1\n  kind: runtime\n  name: x\n  version: 4.0.6\n\
             \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: now\n\
             \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
             \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  provides: {{engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: aarch64-macos}}\n\
             \x20 built_from: {{src_sha256: {}, patch_set: v0.2.8}}\n  capabilities: {{exec: true, read: true, runtime: true}}\n{extra}",
            sha(1),
            sha(2),
            sha(3),
        )
    }

    #[test]
    fn checks_default_empty_and_round_trip() {
        // spec 26 §1 (schema_minor 3): the additive `checks:` key —
        // the payload's own acceptance contracts, declared in-image.
        let bare = PayloadManifest::from_yaml(&minimal_data_yaml("")).unwrap();
        assert!(bare.checks.is_empty());
        // An absent key never serializes (additive on the wire: old
        // readers see the document they always saw).
        assert!(!bare.to_yaml().unwrap().contains("checks"));

        // A full EXEC check (the spec 26 §4 metanorma shape) round-trips.
        let with = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  html-xml:\n    entry: /bin/metanorma\n\
             \x20   argv: [\"--type\", \"iso\", \"{scratch}/test-iso.adoc\", \"--agree-to-terms\"]\n\
             \x20   fixtures: /__tpkg__/check/html-xml\n\
             \x20   expect: {exit: 0, files: [test-iso.xml, test-iso.html], stdout: '\"ok\":1'}\n\
             \x20   needs: [{path: /opt/vendor-tool, access: ro, when: [macos], why: \"probes its install root\"}]\n\
             \x20   requires: {provides: [jvm]}\n\
             \x20   when: [windows, macos, linux]\n\
             \x20   timeout: 180\n",
        ))
        .unwrap();
        let check = &with.checks["html-xml"];
        assert_eq!(
            check.entry,
            Some(CheckEntry::Path("/bin/metanorma".to_string()))
        );
        assert_eq!(
            check.argv,
            vec![
                "--type",
                "iso",
                "{scratch}/test-iso.adoc",
                "--agree-to-terms"
            ]
        );
        assert_eq!(check.fixtures.as_deref(), Some("/__tpkg__/check/html-xml"));
        assert_eq!(check.expect.exit, 0);
        assert_eq!(check.expect.files, vec!["test-iso.xml", "test-iso.html"]);
        assert_eq!(check.expect.stdout.as_deref(), Some("\"ok\":1"));
        assert_eq!(
            check.needs,
            vec![CheckNeed {
                path: "/opt/vendor-tool".to_string(),
                access: crate::jail::JailAccess::Ro,
                when: vec![CheckPlatform::Macos],
                why: Some("probes its install root".to_string()),
            }]
        );
        assert_eq!(
            check.requires,
            Some(CheckRequires {
                provides: vec!["jvm".to_string()]
            })
        );
        assert_eq!(
            check.when,
            vec![
                CheckPlatform::Windows,
                CheckPlatform::Macos,
                CheckPlatform::Linux
            ]
        );
        assert_eq!(check.timeout, Some(180));
        let back = PayloadManifest::from_yaml(&with.to_yaml().unwrap()).unwrap();
        assert_eq!(back, with);

        // A STRUCTURAL check (no entry — the data-slice shape, spec 26
        // §1.1) round-trips; the all-default expect stays absent on the
        // wire.
        let structural = PayloadManifest::from_yaml(&minimal_data_yaml(
            "checks:\n  layout:\n    expect:\n      image_files: [/templates/org/cover.adoc, /templates/org/header.html]\n",
        ))
        .unwrap();
        let check = &structural.checks["layout"];
        assert_eq!(check.entry, None);
        assert_eq!(
            check.expect.image_files,
            vec!["/templates/org/cover.adoc", "/templates/org/header.html"]
        );
        let back = PayloadManifest::from_yaml(&structural.to_yaml().unwrap()).unwrap();
        assert_eq!(back, structural);
        // An exec check asserting only the default exit carries no
        // expect block on the wire at all.
        let smoke = PayloadManifest::from_yaml(&minimal_runtime_yaml(
            "checks:\n  boot:\n    entry: self\n    argv: [\"-e\", \"puts 1\"]\n",
        ))
        .unwrap();
        assert!(!smoke.to_yaml().unwrap().contains("expect"));
    }

    #[test]
    fn checks_entry_self_is_runtime_only() {
        // spec 26 §1.1: `self` names the runtime exe — legal on kind
        // runtime only (the ruby runtime's boot-and-stdlib shape).
        let ok = PayloadManifest::from_yaml(&minimal_runtime_yaml(
            "checks:\n  boot-and-stdlib:\n    entry: self\n\
             \x20   argv: [\"-e\", 'require \"json\"; puts JSON.generate({ok: 1})']\n\
             \x20   expect: {exit: 0, stdout: '\"ok\":1'}\n    timeout: 60\n",
        ))
        .unwrap();
        assert_eq!(
            ok.checks["boot-and-stdlib"].entry,
            Some(CheckEntry::SelfExe)
        );
        // On any other kind the reserved spelling names nothing — a
        // named validation error.
        let err = PayloadManifest::from_yaml(&minimal_app_yaml("checks:\n  c:\n    entry: self\n"))
            .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("self")),
            "{err}"
        );
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "checks:\n  c:\n    entry: self\n    expect: {image_files: [/x]}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("self")),
            "{err}"
        );
    }

    #[test]
    fn checks_structural_rejects_exec_only_keys() {
        // spec 26 §1.1 (MECE — one key decides the shape): a check with
        // no entry is structural BY GRAMMAR; argv/fixtures are exec-only.
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "checks:\n  layout:\n    argv: [\"-e\", \"puts 1\"]\n    expect: {image_files: [/x]}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("argv")),
            "{err}"
        );
        let err = PayloadManifest::from_yaml(&minimal_data_yaml(
            "checks:\n  layout:\n    fixtures: /x\n    expect: {image_files: [/x]}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("fixtures")),
            "{err}"
        );
        // A structural check with no image_files asserts nothing at all.
        let err =
            PayloadManifest::from_yaml(&minimal_data_yaml("checks:\n  layout: {}\n")).unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("image_files")),
            "{err}"
        );
    }

    #[test]
    fn checks_path_rules() {
        // expect.files are scratch-RELATIVE: never absolute, no '..'.
        for bad in ["/abs/out.xml", "../escape.xml", "a/../../escape.xml"] {
            let err = PayloadManifest::from_yaml(&minimal_app_yaml(&format!(
                "checks:\n  c:\n    entry: /bin/x\n    expect: {{files: ['{bad}']}}\n"
            )))
            .unwrap_err();
            assert!(
                matches!(err, ManifestError::Invalid(m) if m.contains("files")),
                "{bad}: {err}"
            );
        }
        // expect.image_files and fixtures are in-image ABSOLUTE, no '..'.
        for (key, bad) in [
            ("image_files", "rel/x.adoc"),
            ("image_files", "/a/../b.adoc"),
        ] {
            let err = PayloadManifest::from_yaml(&minimal_data_yaml(&format!(
                "checks:\n  layout:\n    expect: {{{key}: ['{bad}']}}\n"
            )))
            .unwrap_err();
            assert!(
                matches!(err, ManifestError::Invalid(m) if m.contains("image_files")),
                "{bad}: {err}"
            );
        }
        for bad in ["rel/fixtures", "/a/../fixtures"] {
            let err = PayloadManifest::from_yaml(&minimal_app_yaml(&format!(
                "checks:\n  c:\n    entry: /bin/x\n    fixtures: '{bad}'\n"
            )))
            .unwrap_err();
            assert!(
                matches!(err, ManifestError::Invalid(m) if m.contains("fixtures")),
                "{bad}: {err}"
            );
        }
        // …and entry itself is an in-image absolute path.
        for bad in ["bin/x", "/a/../x"] {
            let err = PayloadManifest::from_yaml(&minimal_app_yaml(&format!(
                "checks:\n  c:\n    entry: '{bad}'\n"
            )))
            .unwrap_err();
            assert!(
                matches!(err, ManifestError::Invalid(m) if m.contains("entry")),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn checks_when_values_come_from_the_platform_family_set() {
        // A value outside windows/macos/linux is a named structural
        // error (serde names the variant set).
        let err = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n    when: [solaris]\n",
        ))
        .unwrap_err();
        assert!(matches!(err, ManifestError::Yaml(_)), "{err}");
        // The full set round-trips in declaration order.
        let ok = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n    when: [linux, windows]\n",
        ))
        .unwrap();
        assert_eq!(
            ok.checks["c"].when,
            vec![CheckPlatform::Linux, CheckPlatform::Windows]
        );
    }

    #[test]
    fn checks_name_grammar() {
        // Names surface in report lines and scratch dir names:
        // [A-Za-z0-9][A-Za-z0-9._-]*.
        for good in ["html-xml", "boot.v2_ok", "9lives"] {
            PayloadManifest::from_yaml(&minimal_app_yaml(&format!(
                "checks:\n  {good}:\n    entry: /bin/x\n"
            )))
            .unwrap_or_else(|e| panic!("{good:?} should parse: {e}"));
        }
        for bad in ["-lead", ".lead", "has space", "has/slash"] {
            let err = PayloadManifest::from_yaml(&minimal_app_yaml(&format!(
                "checks:\n  '{bad}':\n    entry: /bin/x\n"
            )))
            .unwrap_err();
            assert!(
                matches!(err, ManifestError::Invalid(m) if m.contains("checks[] names")),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn checks_timeout_stdout_requires_rules() {
        // timeout is positive seconds.
        let err = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n    timeout: 0\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("timeout")),
            "{err}"
        );
        // stdout is one non-empty regex (carried uncompiled — the engine
        // compiles it; tpkg has no regex dependency by design).
        let err = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n    expect: {stdout: \"\"}\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("stdout")),
            "{err}"
        );
        // requires.provides is a non-empty list of non-empty names.
        for bad in ["{provides: []}", "{provides: [\"\"]}"] {
            let err = PayloadManifest::from_yaml(&minimal_app_yaml(&format!(
                "checks:\n  c:\n    entry: /bin/x\n    requires: {bad}\n"
            )))
            .unwrap_err();
            assert!(
                matches!(err, ManifestError::Invalid(m) if m.contains("provides")),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn checks_scratch_token_at_most_once_per_arg() {
        // `{scratch}` is the ONE argv substitution (spec 26 §1): at most
        // one occurrence per entry — one per arg across several args is
        // fine; twice in one arg is a named error.
        let ok = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n    argv: [\"{scratch}/a.adoc\", \"--out\", \"{scratch}/b\"]\n",
        ))
        .unwrap();
        assert_eq!(ok.checks["c"].argv.len(), 3);
        let err = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n    argv: [\"{scratch}/a:{scratch}/b\"]\n",
        ))
        .unwrap_err();
        assert!(
            matches!(err, ManifestError::Invalid(m) if m.contains("{scratch}")),
            "{err}"
        );
    }

    #[test]
    fn checks_tolerate_unknown_keys() {
        // The crate's forward-compat discipline (spec 18 §3's
        // unknown-field rule) applies INSIDE the block too: a key the
        // model does not own changes nothing it does. NOTE: spec 26 §1's
        // "unknown keys a named error" mis-describes the spec-03
        // discipline — toleration is what makes the additive MINOR
        // mechanism work (a schema_minor 4 field must not break a
        // schema_minor 3 reader).
        let m = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n    future_key: {x: 1}\n",
        ))
        .expect("unknown key inside a check tolerated");
        assert!(m.checks.contains_key("c"));
    }

    #[test]
    fn checks_duplicate_names_are_a_named_error() {
        // A re-declared check name is an authoring ambiguity — the map's
        // deserializer refuses it (never a silent winner; serde_yml's
        // plain map read would be last-wins, cf. the duplicate-alias rule).
        let err = PayloadManifest::from_yaml(&minimal_app_yaml(
            "checks:\n  c:\n    entry: /bin/x\n  c:\n    entry: /bin/y\n",
        ))
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            matches!(err, ManifestError::Yaml(_)) && msg.contains("duplicate check name"),
            "{msg}"
        );
    }
}
