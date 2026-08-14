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
    /// The `capabilities.host` jail block failed its own validation
    /// (spec 08 §4 — the reason travels with the jail error).
    Jail(crate::jail::JailError),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Yaml(e) => write!(f, "payload manifest yaml error: {e}"),
            ManifestError::Invalid(m) => write!(f, "invalid payload manifest: {m}"),
            ManifestError::Jail(e) => write!(f, "invalid payload manifest capabilities.host: {e}"),
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ManifestError::Yaml(e) => Some(e),
            ManifestError::Invalid(_) => None,
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
                if keyid.len() != 16
                    || !keyid
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
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
}

/// PROVIDES of kind `app`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppProvides {
    pub entrypoints: Vec<Entrypoint>,
    pub platforms: Platforms,
    pub capabilities: Capabilities,
}

/// One engine a runtime provides (`{engine, version, abi_line, platform}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineProvides {
    pub engine: String,
    pub version: String,
    /// The abi line this build satisfies (e.g. `"4.0"`) — what
    /// native-extension payloads match their `"~> x.y.z"` against.
    pub abi_line: String,
    pub platform: Platform,
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
            if ep.platform.is_reserved() {
                return Err(ManifestError::Invalid(
                    "provides.provides[].platform must not be the reserved triplet",
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
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------
// The manifest itself
// ---------------------------------------------------------------------

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
    /// reserved-triplet rule, the absolute-path rules, and the
    /// materialize grammar (spec 22 §4). Unknown keys are
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
        for p in &self.materialize {
            check_abs_path(p, "materialize[] must be absolute (inside the image)")?;
            if p.split('/').any(|component| component == "..") {
                return Err(ManifestError::Invalid(
                    "materialize[] must not contain '..' components (the extraction target derives from the entry)",
                ));
            }
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
}
