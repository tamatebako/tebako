//! The package manifest (spec 03 §6): the L2 composition manifest, carried
//! as tpkg extension block type 2 (spec 02 §5b) — YAML, OUTSIDE every
//! payload image, readable without backend knowledge (the OCI model: the
//! manifest is a separate blob BESIDE the layers).
//!
//! It owns **composition** — package identity, the entrypoint/suite
//! entries, package-level jail + env, per-entry runtime refs — while the
//! payload manifests (spec 03 §1–5) stay inside the images and own
//! self-description. The package manifest never duplicates payload
//! manifests; it references slots. Per-entry `runtime_ref` strings kill
//! the trailer's 128-byte single-field limit (suites, multi-runtime
//! packages); the trailer's v1 field stays for v1-era loaders, and
//! packages without the block behave exactly as before (the block is
//! additive).
//!
//! Same authored-YAML discipline as the payload manifest
//! ([`crate::manifest`]): reading is two-step ([`PackageManifest::from_yaml`]
//! does serde structure, then [`PackageManifest::validate`] semantics),
//! unknown keys are tolerated for forward compatibility, and the versioned
//! JSON Schema `schema/tpkg-package-manifest-v1.schema.json` pins the
//! structure. The `jail` block is typed ([`crate::jail::HostJail`]) — spec
//! 08 §1 owns its shape, and the bootstrap composes it with the user's
//! tightening at handoff (spec 08 §2).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

use crate::jail::HostJail;
use crate::manifest::Producer;
use crate::TPKG_MAX_SLOTS;

/// The only `schema_version` this implementation reads and writes.
pub const PACKAGE_SCHEMA_VERSION: u32 = 1;

/// Error returned by package-manifest operations.
///
/// Deliberately separate from [`crate::TpkgError`] (1:1 with the C
/// `TPKG_ERR_*` codes; the package manifest has no C counterpart) and
/// from [`crate::ManifestError`] (the payload manifest's error — the two
/// YAML surfaces report under their own names).
#[derive(Debug)]
pub enum PackageManifestError {
    /// YAML parse/serialize failure (structural — the document does not
    /// match the serde model).
    Yaml(serde_yml::Error),
    /// Semantic validation failure (`validate()`).
    Invalid(&'static str),
    /// The `jail:` block failed the spec 08 validation (the reason travels
    /// with the jail error).
    Jail(crate::jail::JailError),
}

impl fmt::Display for PackageManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManifestError::Yaml(e) => write!(f, "package manifest yaml error: {e}"),
            PackageManifestError::Invalid(m) => write!(f, "invalid package manifest: {m}"),
            PackageManifestError::Jail(e) => write!(f, "invalid package manifest jail: {e}"),
        }
    }
}

impl std::error::Error for PackageManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PackageManifestError::Yaml(e) => Some(e),
            PackageManifestError::Invalid(_) => None,
            PackageManifestError::Jail(e) => Some(e),
        }
    }
}

impl From<serde_yml::Error> for PackageManifestError {
    fn from(e: serde_yml::Error) -> PackageManifestError {
        PackageManifestError::Yaml(e)
    }
}

fn check_non_empty(s: &str, what: &'static str) -> Result<(), PackageManifestError> {
    if s.is_empty() {
        return Err(PackageManifestError::Invalid(what));
    }
    Ok(())
}

/// The `package:` identity block (spec 03 §6) — name, version, producer,
/// created; the package-level provenance minimum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageIdentity {
    pub name: String,
    /// Free-form version string (semver AND datever packages exist — the
    /// model does not interpret versions).
    pub version: String,
    pub producer: Producer,
    /// Creation timestamp (RFC 3339 rendering; kept as a string — the
    /// model does not interpret time).
    pub created: String,
}

/// One invocable command of the package (spec 03 §6: one per entry; N=1
/// for simple apps, N entries for suites). `slot` names the payload image,
/// `entrypoint` the PROVIDES entrypoint inside it, and `runtime_ref` the
/// per-entry runtime reference (no 128-byte cap — suites/multi-runtime).
///
/// `slot` is `None` for a pointer-package entry (spec 23 §13): the entry's
/// slice is SHARED — resolved at run time from the machine cache by the
/// [`PackageLock`] under the entry's own name (entry name == lock slice
/// name). Every package pressed before the composition spectrum has
/// `Some(slot)`; the key stays absent in the YAML exactly when `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    /// The command name (the shim registers under this).
    pub name: String,
    /// Which payload slot carries the entrypoint's image; `None` = the
    /// entry's slice is shared (a lock slice named `name` must exist and
    /// be `carry: false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    /// Which PROVIDES entrypoint inside that image.
    pub entrypoint: String,
    /// Per-entry runtime reference (`type@version;tebako=<abi>[;params]`).
    pub runtime_ref: String,
}

/// The mount mode of one slot's image (spec 03 §6 / spec 17 §1). The
/// default is [`MountMode::Exclusive`]: a slot without a `mounts` row —
/// and every package without the block — behaves exactly as before (a
/// duplicate mount point is the driver's named EEXIST error).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountMode {
    /// The image claims the mount point alone; an occupied point is the
    /// driver's named EEXIST error (the historical behavior).
    #[default]
    Exclusive,
    /// The image merges over the images already mounted at the point:
    /// directories combine, file conflicts resolve by the declared
    /// precedence (the env image is always lowest), and every member
    /// stays read-only (spec 17 §1).
    Union,
    /// RESERVED spelling (spec 03 §6): the transforms law — COW overlays
    /// exist only in the Rust TFS, never as package mount semantics.
    /// [`PackageManifest::validate`] refuses it with a named error until
    /// its spec lands.
    Cow,
    /// RESERVED spelling (spec 03 §6): same axis as `cow`.
    Enc,
}

/// Where a union-mounted image sits in the stack at its point
/// (spec 03 §6): over the runtime's env image (the pressed-app form) or
/// over another payload slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precedence {
    /// Over the runtime's env image (`after-env` — the env image is
    /// always the lowest member of a union).
    AfterEnv,
    /// Over another payload slot (`after:<slot>`).
    AfterSlot(u32),
}

impl fmt::Display for Precedence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Precedence::AfterEnv => f.write_str("after-env"),
            Precedence::AfterSlot(n) => write!(f, "after:{n}"),
        }
    }
}

impl serde::Serialize for Precedence {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Precedence {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text == "after-env" {
            return Ok(Precedence::AfterEnv);
        }
        if let Some(n) = text.strip_prefix("after:") {
            if let Ok(n) = n.parse::<u32>() {
                return Ok(Precedence::AfterSlot(n));
            }
        }
        Err(serde::de::Error::custom(format!(
            "unknown precedence '{text}' — 'after-env' or 'after:<slot>'"
        )))
    }
}

/// One row of the `mounts:` block (spec 03 §6): the mount semantics of
/// one slot's image — point, mode, and (union only) precedence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageMount {
    /// Which payload slot the row governs.
    pub slot: u32,
    /// The mount point (identical to the slot's trailer mount point).
    pub point: String,
    /// exclusive (default) | union; `cow`/`enc` parse but are named
    /// errors at validation (reserved — the transforms law).
    #[serde(default)]
    pub mode: MountMode,
    /// Union-only: which member this image shadows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precedence: Option<Precedence>,
}

// ---------------------------------------------------------------------
// The press-time lock (spec 23 §4/§13 — the composition spectrum)
// ---------------------------------------------------------------------

/// A slice's digest pin (spec 23 §13.3): the single digest — the carried
/// form (the bytes are fixed at stitch) or a `universal`-coverage slice —
/// OR the per-target-triplet digest map. The map keys are release-asset
/// platform names (`macos-arm64`, … — the registry row spelling, spec 04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestPin {
    /// One digest regardless of triplet.
    One(String),
    /// Per-triplet digests; the run-time lookup keys on the host triplet.
    PerTriplet(BTreeMap<String, String>),
}

impl Serialize for DigestPin {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            DigestPin::One(digest) => serializer.serialize_str(digest),
            DigestPin::PerTriplet(map) => map.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for DigestPin {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            One(String),
            Map(BTreeMap<String, String>),
        }
        match Repr::deserialize(deserializer)? {
            Repr::One(digest) => Ok(DigestPin::One(digest)),
            Repr::Map(map) => Ok(DigestPin::PerTriplet(map)),
        }
    }
}

fn check_sha256_hex(digest: &str, what: &'static str) -> Result<(), PackageManifestError> {
    if digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(PackageManifestError::Invalid(what))
    }
}

impl DigestPin {
    /// The digest locked for `host` (release-asset-name keyed map lookup);
    /// `None` when the map does not cover the host (a coverage gap the
    /// caller turns into the spec 23 §13.3 named error).
    pub fn for_host(&self, host: crate::Platform) -> Option<&str> {
        match self {
            DigestPin::One(digest) => Some(digest),
            DigestPin::PerTriplet(map) => map.get(host.release_asset_name()).map(String::as_str),
        }
    }

    fn validate(&self, what: &'static str) -> Result<(), PackageManifestError> {
        match self {
            DigestPin::One(digest) => check_sha256_hex(digest, what),
            DigestPin::PerTriplet(map) => {
                if map.is_empty() {
                    return Err(PackageManifestError::Invalid(what));
                }
                for (triplet, digest) in map {
                    if crate::Platform::from_release_asset_name(triplet).is_none() {
                        return Err(PackageManifestError::Invalid(
                            "lock digest map key is not a known platform release-asset name",
                        ));
                    }
                    check_sha256_hex(digest, what)?;
                }
                Ok(())
            }
        }
    }
}

/// One carried artifact of the runtime pair (spec 19 §6.1): the slot the
/// bytes ride in plus the press-time digest pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedArtifact {
    /// The trailer slot carrying the bytes.
    pub slot: u32,
    pub sha256: DigestPin,
    /// The PE install name (the windows dll-era facet only,
    /// tebako-runtime-ruby#40): the DLL is staged next to the exe under
    /// THIS name — the name the exe's imports reference — never the
    /// release-asset spelling. Flowed from the release manifest's
    /// `dll.install_as` at press; absent on every other artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_as: Option<String>,
}

/// The locked runtime (spec 23 §4): the concrete version, the carry
/// verdict, and — when carried (the self-contained preset, spec 19 §6.1) —
/// the exe / env-image / (windows) dll slots with their digest pins. A
/// shared runtime (`carry: false`) records only the version: run-time
/// resolution and verification then ride the ordinary spec 05 §5 chain
/// (runtime_ref → cache/index anchors), never the lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedRuntime {
    /// The concrete runtime version the press resolved.
    pub version: String,
    pub carry: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe: Option<LockedArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<LockedArtifact>,
    /// The windows dll-era facet (tebako-runtime-ruby#40): carried as a
    /// third slot when the resolved release declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dll: Option<LockedArtifact>,
}

/// One locked payload slice (spec 23 §4/§13): the concrete version, the
/// carry verdict, the digest pin, the declared mount when a consumer edge
/// declares one (spec 03 §2.3's mount rule), and — for a shared slice —
/// the fetch coordinates (a spec 04 reference) the press resolved from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedSlice {
    pub name: String,
    pub version: String,
    pub carry: bool,
    /// The trailer slot (carried slices only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    /// The declared mount point; absent = the slice is carried/shared as
    /// a cache prime with no mount (no consumer edge declared one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount: Option<String>,
    pub sha256: DigestPin,
    /// The fetch coordinates (shared slices only — required there; on a
    /// carried slice it is provenance, never a fallback fetch path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One artifact of a locked spawned-runtime pair (spec 23 §13.6): the
/// digest pin ALWAYS (spec 23 §13.4 records the digest carried or
/// shared), the trailer slot when carried, and — the windows dll-era
/// facet only — the PE install name (tebako-runtime-ruby#40, the same
/// rule as [`LockedArtifact`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedSpawnedArtifact {
    /// The trailer slot carrying the bytes (carried rows only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    pub sha256: DigestPin,
    /// The PE install name (the windows dll facet only): a bare file
    /// name, never the release-asset spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_as: Option<String>,
}

/// One locked spawned-runtime edge (spec 23 §13.6, spec 30 §2): the app
/// payload's L1 `requires[].kind: runtime` edge mirrored (engine /
/// implementation / constraint / expose), the press-time pick's version
/// pair (`version` = the runtime's language version, `tebako` = its own
/// tebako line), the carry verdict, and the pair's digest pins. The
/// loader (the bootstrap for a self-contained package) resolves each row
/// into the store's `runtimes/` area at dispatch and exports the pin via
/// `TEBAKO_SPAWN_LOCK`; the edge is NEVER co-mounted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedSpawnedRuntime {
    /// The L1 edge's engine (`java`, …).
    pub engine: String,
    /// The L1 edge's implementation axis (spec 28 §8), when named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    /// The L1 edge's constraint, mirrored verbatim (the validate
    /// cross-check asserts the mirror and that `version` satisfies it).
    pub constraint: crate::manifest::Constraint,
    /// The L1 edge's exposed command names (bare names), mirrored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<String>,
    /// The press-time pick: the runtime's language version.
    pub version: String,
    /// The press-time pick: the runtime's own tebako line.
    pub tebako: String,
    pub carry: bool,
    /// The wrapper exe — always present (the digest pin stands either
    /// way); `slot` exactly when carried.
    pub exe: LockedSpawnedArtifact,
    /// The env image — always present; `slot` exactly when carried.
    pub image: LockedSpawnedArtifact,
    /// The windows dll-era facet, when the resolved release declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dll: Option<LockedSpawnedArtifact>,
    /// The fetch coordinates (shared rows only — required there): the
    /// press-resolved release download base (the `{base}/v<tebako>/…`
    /// root the loader replays verbatim, spec 23 §13.6). On a carried
    /// row it is provenance, never a fallback fetch path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One locked spawned-PAYLOAD edge (spec 32 §6, spec 23 §13.6 amended):
/// the app payload's expose-carrying L1 `requires[].kind: executable`
/// edge mirrored (payload / constraint / expose), the press-time pick's
/// `version`, the carry verdict, the provider image's digest pin, and —
/// nested — the provider's OWN `kind: language` edge resolved as a §13.6
/// runtime row (its `constraint` mirrored from the PROVIDER's manifest;
/// the press validate cross-checks both levels). The loader resolves the
/// provider image into `payloads/` and the nested pair into `runtimes/`
/// at dispatch; a carried spawned payload is never mounted BY THE PARENT
/// (its slots ride the lock's claimed-slot set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedSpawnedPayload {
    /// The resolved provider payload's name (the L1 edge's `payload` pin
    /// when it names one, else the capability's provider).
    pub payload: String,
    /// The consumer's L1 edge constraint, mirrored verbatim.
    pub constraint: crate::manifest::Constraint,
    /// The consumer's L1 edge exposed command names (bare names),
    /// mirrored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<String>,
    /// The press-time pick: the provider payload's version.
    pub version: String,
    pub carry: bool,
    /// The provider image — the digest pin always; `slot` exactly when
    /// carried.
    pub image: LockedSpawnedArtifact,
    /// The provider's own language edge, resolved — the §13.6 runtime
    /// row, nested (its `expose` stays empty: the spawn surface rides
    /// THIS row's `expose`).
    pub runtime: LockedSpawnedRuntime,
    /// The fetch coordinates (shared rows only — required there): the
    /// press-resolved download base for the payload image (a spec 04
    /// reference, replayed verbatim); the nested runtime pair's base
    /// rides `runtime.source` per §13.6's shared-row rule. On a carried
    /// row it is provenance, never a fallback fetch path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One row of the lock's `spawned[]` list (spec 23 §13.6, spec 32 §6):
/// either a spawned-runtime edge (spec 30) or a spawned-payload edge
/// (spec 32). The two shapes are MECE on the wire — a runtime row keys
/// on `engine:`, a payload row keys on `payload:` (the untagged form;
/// both row shapes validate fail-closed in [`PackageLock::validate`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LockedSpawned {
    /// A spawned-runtime edge (spec 30, spec 23 §13.6).
    Runtime(LockedSpawnedRuntime),
    /// A spawned-payload edge (spec 32 §6).
    Payload(LockedSpawnedPayload),
}

impl LockedSpawned {
    /// Every artifact the row claims pins for (the slot set the bootstrap
    /// must never hand to the driver as payload mounts): a runtime row's
    /// exe + image (+ dll); a payload row's image plus the nested runtime
    /// row's set (spec 32 §6).
    pub fn artifacts(&self) -> Vec<&LockedSpawnedArtifact> {
        fn runtime_artifacts(row: &LockedSpawnedRuntime) -> Vec<&LockedSpawnedArtifact> {
            let mut out = vec![&row.exe, &row.image];
            if let Some(dll) = &row.dll {
                out.push(dll);
            }
            out
        }
        match self {
            LockedSpawned::Runtime(row) => runtime_artifacts(row),
            LockedSpawned::Payload(row) => {
                let mut out = vec![&row.image];
                out.extend(runtime_artifacts(&row.runtime));
                out
            }
        }
    }
}

/// The `lock:` block of the L2 package manifest (spec 23 §4/§13): what
/// press resolved is what runs — the full composition closure locked per
/// slice. Run-time resolution follows the lock by locked digest, never
/// fresh semver (fail-closed on mismatch — spec 18 S63).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageLock {
    /// The locked runtime pair; absent on packages pressed before the
    /// composition spectrum (and on suites, whose per-entry refs ride
    /// `entries[].runtime_ref` unchanged).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<LockedRuntime>,
    /// Every resolved payload slice, in mount order: the app payload
    /// first, then its dependency closure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slices: Vec<LockedSlice>,
    /// Every spawned-runtime edge (spec 30) of the app payload, in
    /// manifest order, press-locked (spec 23 §13.6): the L1 edge mirror
    /// (engine/implementation/constraint/expose), the pick's version
    /// pair, the carry verdict, and the pair's digest pins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spawned: Vec<LockedSpawned>,
}

impl PackageLock {
    /// The slice named `name`, when the lock carries one.
    pub fn slice(&self, name: &str) -> Option<&LockedSlice> {
        self.slices.iter().find(|s| s.name == name)
    }

    /// The trailer slots the lock claims (the carried runtime pair plus
    /// every carried slice plus every carried spawned pair) — the set the
    /// bootstrap must never hand to the driver as payload mounts.
    pub fn claimed_slots(&self) -> Vec<u32> {
        let mut out = Vec::new();
        if let Some(runtime) = &self.runtime {
            for artifact in [&runtime.exe, &runtime.image, &runtime.dll]
                .into_iter()
                .flatten()
            {
                out.push(artifact.slot);
            }
        }
        for slice in &self.slices {
            if let Some(slot) = slice.slot {
                out.push(slot);
            }
        }
        for spawned in &self.spawned {
            for artifact in spawned.artifacts() {
                if let Some(slot) = artifact.slot {
                    out.push(slot);
                }
            }
        }
        out
    }

    /// The shared slices (carry: false) in declaration order.
    pub fn shared_slices(&self) -> impl Iterator<Item = &LockedSlice> {
        self.slices.iter().filter(|s| !s.carry)
    }

    fn validate(&self) -> Result<(), PackageManifestError> {
        if let Some(runtime) = &self.runtime {
            check_non_empty(&runtime.version, "lock.runtime.version must not be empty")?;
            match runtime.carry {
                true => {
                    if runtime.exe.is_none() || runtime.image.is_none() {
                        return Err(PackageManifestError::Invalid(
                            "lock.runtime with carry: true requires the exe and image slots (the two-slot carried pair, spec 19 §6.1)",
                        ));
                    }
                }
                false => {
                    if runtime.exe.is_some() || runtime.image.is_some() || runtime.dll.is_some() {
                        return Err(PackageManifestError::Invalid(
                            "lock.runtime with carry: false declares no slots — a shared runtime resolves through the ordinary spec 05 §5 chain",
                        ));
                    }
                }
            }
            for artifact in [&runtime.exe, &runtime.image, &runtime.dll]
                .into_iter()
                .flatten()
            {
                if artifact.slot >= TPKG_MAX_SLOTS {
                    return Err(PackageManifestError::Invalid(
                        "lock.runtime slot is outside the container's slot capacity (0..TPKG_MAX_SLOTS-1)",
                    ));
                }
                artifact
                    .sha256
                    .validate("lock.runtime sha256 pins must be 64 lowercase hex")?;
                if let Some(install_as) = &artifact.install_as {
                    if install_as.is_empty()
                        || install_as.contains('/')
                        || install_as.contains('\\')
                    {
                        return Err(PackageManifestError::Invalid(
                            "lock.runtime install_as must be a bare file name (the PE import name)",
                        ));
                    }
                }
            }
        }
        let mut names: Vec<&str> = Vec::new();
        for slice in &self.slices {
            check_non_empty(&slice.name, "lock.slices[].name must not be empty")?;
            check_non_empty(&slice.version, "lock.slices[].version must not be empty")?;
            names.push(slice.name.as_str());
            slice
                .sha256
                .validate("lock.slices[].sha256 pins must be 64 lowercase hex")?;
            match (slice.carry, slice.slot) {
                (true, Some(slot)) => {
                    if slot >= TPKG_MAX_SLOTS {
                        return Err(PackageManifestError::Invalid(
                            "lock.slices[].slot is outside the container's slot capacity (0..TPKG_MAX_SLOTS-1)",
                        ));
                    }
                }
                (true, None) => {
                    return Err(PackageManifestError::Invalid(
                        "lock.slices[] with carry: true requires its trailer slot",
                    ));
                }
                (false, Some(_)) => {
                    return Err(PackageManifestError::Invalid(
                        "lock.slices[] with carry: false declares no slot — a shared slice rides the machine cache",
                    ));
                }
                (false, None) => {
                    if slice.source.as_deref().map_or(true, |s| s.is_empty()) {
                        return Err(PackageManifestError::Invalid(
                            "lock.slices[] with carry: false requires its fetch coordinates (source:)",
                        ));
                    }
                }
            }
            if let Some(mount) = &slice.mount {
                if mount.is_empty() || !mount.starts_with('/') {
                    return Err(PackageManifestError::Invalid(
                        "lock.slices[].mount must be a declared (POSIX-absolute) mount point",
                    ));
                }
            }
        }
        names.sort_unstable();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(PackageManifestError::Invalid(
                "duplicate lock.slices[].name (one lock row per slice)",
            ));
        }
        let mut runtime_edges: Vec<(&str, Option<&str>)> = Vec::new();
        let mut payload_edges: Vec<&str> = Vec::new();
        for spawned in &self.spawned {
            match spawned {
                LockedSpawned::Runtime(row) => {
                    validate_spawned_runtime_row(row)?;
                    let edge = (row.engine.as_str(), row.implementation.as_deref());
                    if runtime_edges.contains(&edge) {
                        return Err(PackageManifestError::Invalid(
                            "duplicate lock.spawned[] edge (one lock row per engine+implementation)",
                        ));
                    }
                    runtime_edges.push(edge);
                }
                LockedSpawned::Payload(row) => {
                    // spec 32 §6: the payload row mirrors the consumer's
                    // expose-carrying `kind: executable` edge; the nested
                    // runtime row is the provider's own language edge,
                    // resolved (the §13.6 row rules ride).
                    check_non_empty(&row.payload, "lock.spawned[].payload must not be empty")?;
                    check_non_empty(&row.version, "lock.spawned[].version must not be empty")?;
                    check_spawned_expose(&row.expose)?;
                    row.image
                        .sha256
                        .validate("lock.spawned[] sha256 pins must be 64 lowercase hex")?;
                    match (row.carry, row.image.slot) {
                        (true, Some(slot)) => {
                            if slot >= TPKG_MAX_SLOTS {
                                return Err(PackageManifestError::Invalid(
                                    "lock.spawned[] slot is outside the container's slot capacity (0..TPKG_MAX_SLOTS-1)",
                                ));
                            }
                        }
                        (true, None) => {
                            return Err(PackageManifestError::Invalid(
                                "lock.spawned[] payload row with carry: true requires the image slot (the carried provider image)",
                            ));
                        }
                        (false, Some(_)) => {
                            return Err(PackageManifestError::Invalid(
                                "lock.spawned[] payload row with carry: false declares no slot — a shared spawned payload rides the machine cache",
                            ));
                        }
                        (false, None) => {}
                    }
                    if !row.carry && row.source.as_deref().map_or(true, |s| s.is_empty()) {
                        return Err(PackageManifestError::Invalid(
                            "lock.spawned[] payload row with carry: false requires its fetch coordinates (source:)",
                        ));
                    }
                    if !row.runtime.expose.is_empty() {
                        return Err(PackageManifestError::Invalid(
                            "lock.spawned[] payload row's nested runtime row declares no expose — the spawn surface rides the payload row's own expose list (spec 32 §6)",
                        ));
                    }
                    validate_spawned_runtime_row(&row.runtime)?;
                    if payload_edges.contains(&row.payload.as_str()) {
                        return Err(PackageManifestError::Invalid(
                            "duplicate lock.spawned[] payload edge (one lock row per provider payload)",
                        ));
                    }
                    payload_edges.push(row.payload.as_str());
                }
            }
        }
        let mut slots = self.claimed_slots();
        slots.sort_unstable();
        if slots.windows(2).any(|w| w[0] == w[1]) {
            return Err(PackageManifestError::Invalid(
                "two locked artifacts claim the same trailer slot",
            ));
        }
        Ok(())
    }
}

/// The spawned-row expose grammar (spec 23 §13.6, spec 32 §6): bare
/// command names — no path separator, no drive qualifier.
fn check_spawned_expose(expose: &[String]) -> Result<(), PackageManifestError> {
    for name in expose {
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains(':') {
            return Err(PackageManifestError::Invalid(
                "lock.spawned[].expose names must be bare command names",
            ));
        }
    }
    Ok(())
}

/// One spawned-runtime row's validation (spec 23 §13.6) — shared by the
/// top-level `spawned[]` rows (spec 30) and the nested runtime row of a
/// spawned-payload row (spec 32 §6, where it spells the provider's own
/// resolved language edge).
fn validate_spawned_runtime_row(
    spawned: &LockedSpawnedRuntime,
) -> Result<(), PackageManifestError> {
    check_non_empty(&spawned.engine, "lock.spawned[].engine must not be empty")?;
    check_non_empty(&spawned.version, "lock.spawned[].version must not be empty")?;
    check_non_empty(&spawned.tebako, "lock.spawned[].tebako must not be empty")?;
    if let Some(implementation) = &spawned.implementation {
        check_non_empty(
            implementation,
            "lock.spawned[].implementation, when present, must not be empty",
        )?;
    }
    check_spawned_expose(&spawned.expose)?;
    let mut artifacts = vec![(&spawned.exe, "exe"), (&spawned.image, "image")];
    if let Some(dll) = &spawned.dll {
        artifacts.push((dll, "dll"));
    }
    for (artifact, which) in artifacts {
        artifact
            .sha256
            .validate("lock.spawned[] sha256 pins must be 64 lowercase hex")?;
        match (spawned.carry, artifact.slot) {
            (true, Some(slot)) => {
                if slot >= TPKG_MAX_SLOTS {
                    return Err(PackageManifestError::Invalid(
                        "lock.spawned[] slot is outside the container's slot capacity (0..TPKG_MAX_SLOTS-1)",
                    ));
                }
            }
            (true, None) => {
                return Err(PackageManifestError::Invalid(
                    match which {
                        "dll" => {
                            "lock.spawned[].dll with carry: true requires its trailer slot"
                        }
                        _ => {
                            "lock.spawned[] with carry: true requires the exe and image slots (the carried pair)"
                        }
                    },
                ));
            }
            (false, Some(_)) => {
                return Err(PackageManifestError::Invalid(
                    "lock.spawned[] with carry: false declares no slots — a shared spawned runtime rides the machine cache",
                ));
            }
            (false, None) => {}
        }
        if let Some(install_as) = &artifact.install_as {
            if install_as.is_empty() || install_as.contains('/') || install_as.contains('\\') {
                return Err(PackageManifestError::Invalid(
                    "lock.spawned[] install_as must be a bare file name (the PE import name)",
                ));
            }
        }
    }
    if !spawned.carry && spawned.source.as_deref().map_or(true, |s| s.is_empty()) {
        return Err(PackageManifestError::Invalid(
            "lock.spawned[] with carry: false requires its fetch coordinates (source:)",
        ));
    }
    Ok(())
}

/// The L2 package manifest (spec 03 §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub package: PackageIdentity,
    /// One entry per invocable command (N >= 1).
    pub entries: Vec<PackageEntry>,
    /// Package-level jail request (spec 08 §1 owns the shape): the access
    /// the package was pressed with (`tebako press --jail`). The bootstrap
    /// composes it with the user's tightening at handoff — manifest
    /// request ∩ user policy = effective jail (spec 08 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jail: Option<HostJail>,
    /// Package-level env (composition rules: spec 07).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Per-slot mount semantics (spec 03 §6, locked 2026-08-04): the
    /// driver reads the modes from the running package's OWN trailer
    /// (spec 17 §1). A slot without a row mounts **exclusive** — the
    /// historical behavior; an absent block (the v1 shape) is every slot
    /// exclusive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<PackageMount>,
    /// The press-time composition lock (spec 23 §4/§13). Absent on
    /// pre-spectrum packages — they resolve exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock: Option<PackageLock>,
}

impl PackageManifest {
    /// Parse and validate a package manifest from YAML text.
    pub fn from_yaml(text: &str) -> Result<PackageManifest, PackageManifestError> {
        let manifest: PackageManifest = serde_yml::from_str(text)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate and serialize to YAML text.
    pub fn to_yaml(&self) -> Result<String, PackageManifestError> {
        self.validate()?;
        Ok(serde_yml::to_string(self)?)
    }

    /// Semantic checks beyond the serde structure: schema version,
    /// non-empty identity/entry fields, N >= 1 entries, slot indexes
    /// inside the container's slot capacity, unique entry names, non-empty
    /// env keys, and the spec 08 jail block's own validation. Unknown keys
    /// are tolerated at every level.
    pub fn validate(&self) -> Result<(), PackageManifestError> {
        if self.schema_version != PACKAGE_SCHEMA_VERSION {
            return Err(PackageManifestError::Invalid(
                "schema_version is not supported",
            ));
        }
        check_non_empty(&self.package.name, "package.name must not be empty")?;
        check_non_empty(&self.package.version, "package.version must not be empty")?;
        check_non_empty(
            &self.package.producer.tool,
            "package.producer.tool must not be empty",
        )?;
        check_non_empty(
            &self.package.producer.tool_version,
            "package.producer.tool_version must not be empty",
        )?;
        check_non_empty(&self.package.created, "package.created must not be empty")?;
        if self.entries.is_empty() {
            return Err(PackageManifestError::Invalid(
                "entries must not be empty (N>=1)",
            ));
        }
        for entry in &self.entries {
            check_non_empty(&entry.name, "entries[].name must not be empty")?;
            check_non_empty(&entry.entrypoint, "entries[].entrypoint must not be empty")?;
            check_non_empty(
                &entry.runtime_ref,
                "entries[].runtime_ref must not be empty",
            )?;
            match entry.slot {
                Some(slot) if slot >= TPKG_MAX_SLOTS => {
                    return Err(PackageManifestError::Invalid(
                        "entries[].slot is outside the container's slot capacity (0..TPKG_MAX_SLOTS-1)",
                    ));
                }
                // A slot-less entry is the pointer-package form (spec 23
                // §13): the lock must carry the entry's slice as shared,
                // under the entry's own name.
                None => {
                    let backed = self
                        .lock
                        .as_ref()
                        .and_then(|lock| lock.slice(&entry.name))
                        .is_some_and(|slice| !slice.carry);
                    if !backed {
                        return Err(PackageManifestError::Invalid(
                            "entries[].slot is absent (a shared entry slice) but no lock slice with the entry's name and carry: false backs it",
                        ));
                    }
                }
                _ => {}
            }
        }
        let mut names: Vec<&str> = self.entries.iter().map(|e| e.name.as_str()).collect();
        names.sort_unstable();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(PackageManifestError::Invalid(
                "duplicate entries[].name (one entry per invocable command)",
            ));
        }
        if self.env.keys().any(|k| k.is_empty()) {
            return Err(PackageManifestError::Invalid("env keys must not be empty"));
        }
        for mount in &self.mounts {
            if mount.slot >= TPKG_MAX_SLOTS {
                return Err(PackageManifestError::Invalid(
                    "mounts[].slot is outside the container's slot capacity (0..TPKG_MAX_SLOTS-1)",
                ));
            }
            check_non_empty(&mount.point, "mounts[].point must not be empty")?;
            match mount.mode {
                MountMode::Exclusive | MountMode::Union => {}
                MountMode::Cow => {
                    return Err(PackageManifestError::Invalid(
                        "mounts[].mode 'cow' is reserved — COW overlays exist only in the Rust TFS (the transforms law) and are not package mount semantics until their spec lands",
                    ));
                }
                MountMode::Enc => {
                    return Err(PackageManifestError::Invalid(
                        "mounts[].mode 'enc' is reserved — ENC overlays exist only in the Rust TFS (the transforms law) and are not package mount semantics until their spec lands",
                    ));
                }
            }
            match (mount.mode, mount.precedence) {
                (MountMode::Union, None) => {
                    return Err(PackageManifestError::Invalid(
                        "mounts[].precedence is required for mode 'union' (after-env | after:<slot>)",
                    ));
                }
                (MountMode::Exclusive, Some(_)) => {
                    return Err(PackageManifestError::Invalid(
                        "mounts[].precedence is union-only — an exclusive row declares no shadowing",
                    ));
                }
                _ => {}
            }
            if let Some(Precedence::AfterSlot(n)) = mount.precedence {
                if n >= TPKG_MAX_SLOTS {
                    return Err(PackageManifestError::Invalid(
                        "mounts[].precedence after:<slot> is outside the container's slot capacity",
                    ));
                }
                if n == mount.slot {
                    return Err(PackageManifestError::Invalid(
                        "mounts[].precedence after:<slot> must not name the row's own slot",
                    ));
                }
            }
        }
        {
            let mut slots: Vec<u32> = self.mounts.iter().map(|m| m.slot).collect();
            slots.sort_unstable();
            if slots.windows(2).any(|w| w[0] == w[1]) {
                return Err(PackageManifestError::Invalid(
                    "duplicate mounts[].slot (one mount-semantics row per slot)",
                ));
            }
        }
        if let Some(lock) = &self.lock {
            lock.validate()?;
            // Entry ↔ slice consistency: an entry whose name a lock slice
            // carries must agree on the physical placement — carried slice
            // slot == entry slot, shared slice ⇔ slot-less entry.
            for entry in &self.entries {
                let Some(slice) = lock.slice(&entry.name) else {
                    continue;
                };
                match (slice.carry, slice.slot, entry.slot) {
                    (true, Some(slice_slot), Some(entry_slot)) if slice_slot == entry_slot => {}
                    (false, None, None) => {}
                    _ => {
                        return Err(PackageManifestError::Invalid(
                            "entries[] and the lock disagree on the slice's placement (carry verdict / slot)",
                        ));
                    }
                }
            }
        }
        if let Some(jail) = &self.jail {
            jail.validate().map_err(PackageManifestError::Jail)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> PackageManifest {
        PackageManifest {
            schema_version: PACKAGE_SCHEMA_VERSION,
            package: PackageIdentity {
                name: "metanorma".to_string(),
                version: "1.2.3".to_string(),
                producer: Producer {
                    tool: "tebako-cli".to_string(),
                    tool_version: "0.16.0".to_string(),
                },
                created: "2026-07-26T00:00:00Z".to_string(),
            },
            entries: vec![PackageEntry {
                name: "metanorma".to_string(),
                slot: Some(0),
                entrypoint: "metanorma".to_string(),
                runtime_ref: "ruby@3.4.2;tebako=0.15.9".to_string(),
            }],
            jail: None,
            env: BTreeMap::new(),
            mounts: Vec::new(),
            lock: None,
        }
    }

    #[test]
    fn validate_accepts_minimal() {
        minimal().validate().unwrap();
    }

    #[test]
    fn validate_rejections() {
        let bad = |m: &PackageManifest| m.validate().is_err();

        let mut m = minimal();
        m.schema_version = 2;
        assert!(bad(&m));

        let mut m = minimal();
        m.entries.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.entries[0].name.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.entries[0].entrypoint.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.entries[0].runtime_ref.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.entries[0].slot = Some(TPKG_MAX_SLOTS);
        assert!(bad(&m));

        let mut m = minimal();
        m.entries[0].slot = None; // a shared entry slice with no lock backing
        assert!(bad(&m));

        let mut m = minimal();
        m.package.name.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.package.created.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.entries.push(m.entries[0].clone()); // duplicate name
        m.entries[1].slot = Some(1);
        assert!(bad(&m));

        let mut m = minimal();
        m.env.insert(String::new(), "x".to_string());
        assert!(bad(&m));
    }

    #[test]
    fn yaml_round_trip() {
        let m = minimal();
        let text = m.to_yaml().unwrap();
        let back = PackageManifest::from_yaml(&text).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn unknown_keys_are_tolerated() {
        let text = "schema_version: 1\n\
                    package: {name: x, version: 1.0.0, producer: {tool: t, tool_version: 1}, created: now, future: yes}\n\
                    entries:\n  - {name: x, slot: 0, entrypoint: x, runtime_ref: ruby@3.4.2;tebako=0.15.9, note: later}\n\
                    future_section: {anything: goes}\n";
        let m = PackageManifest::from_yaml(text).unwrap();
        assert_eq!(m.package.name, "x");
        assert_eq!(m.entries.len(), 1);
    }

    #[test]
    fn jail_block_is_typed_and_round_trips() {
        let text = "schema_version: 1\n\
                    package: {name: x, version: 1.0.0, producer: {tool: t, tool_version: 1}, created: now}\n\
                    entries:\n  - {name: x, slot: 0, entrypoint: x, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n\
                    jail:\n\
                    \x20 default: deny\n\
                    \x20 mounts:\n\
                    \x20   - {host: /home/u/src, mount: /work, access: rw}\n\
                    \x20 argument_files: auto-allowed\n";
        let m = PackageManifest::from_yaml(text).unwrap();
        let jail = m.jail.as_ref().unwrap();
        assert!(!jail.default_open);
        assert_eq!(jail.mounts.len(), 1);
        assert_eq!(jail.mounts[0].access, crate::jail::JailAccess::Rw);
        assert!(jail.argument_files.auto);
        let rendered = m.to_yaml().unwrap();
        let back = PackageManifest::from_yaml(&rendered).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn jail_block_validates() {
        let mut m = minimal();
        m.jail = Some(HostJail {
            mounts: vec![crate::jail::JailMount {
                host: "/h".to_string(),
                mount: "relative".to_string(),
                access: crate::jail::JailAccess::Ro,
            }],
            ..HostJail::deny()
        });
        assert!(matches!(m.validate(), Err(PackageManifestError::Jail(_))));
        // Unknown keys inside the block are tolerated (forward-compat);
        // the block's declared shape still parses.
        let text = "schema_version: 1\n\
                    package: {name: x, version: 1.0.0, producer: {tool: t, tool_version: 1}, created: now}\n\
                    entries:\n  - {name: x, slot: 0, entrypoint: x, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n\
                    jail: {default: deny, future: yes}\n";
        let m = PackageManifest::from_yaml(text).unwrap();
        assert!(!m.jail.as_ref().unwrap().default_open);
    }

    // ---------------------------------------------------------------
    // The mounts block (spec 03 §6, locked 2026-08-04)
    // ---------------------------------------------------------------

    const HEADER: &str = "schema_version: 1\n\
                          package: {name: x, version: 1.0.0, producer: {tool: t, tool_version: 1}, created: now}\n\
                          entries:\n  - {name: x, slot: 0, entrypoint: x, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n";

    #[test]
    fn mounts_absent_block_is_the_v1_shape() {
        // No mounts: key → empty block, and the empty block never
        // serializes (v1-era packages keep their exact shape).
        let m = PackageManifest::from_yaml(HEADER).unwrap();
        assert_eq!(m.mounts, Vec::new());
        assert!(!m.to_yaml().unwrap().contains("mounts"));
    }

    #[test]
    fn mounts_exclusive_is_the_default_mode() {
        let text = format!("{HEADER}mounts:\n  - {{slot: 0, point: /data}}\n");
        let m = PackageManifest::from_yaml(&text).unwrap();
        assert_eq!(m.mounts.len(), 1);
        assert_eq!(m.mounts[0].mode, MountMode::Exclusive);
        assert_eq!(m.mounts[0].precedence, None);
        let back = PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn mounts_union_after_env_round_trips() {
        let text = format!(
            "{HEADER}mounts:\n  - {{slot: 0, point: /__tfs__, mode: union, precedence: after-env}}\n"
        );
        let m = PackageManifest::from_yaml(&text).unwrap();
        assert_eq!(m.mounts[0].mode, MountMode::Union);
        assert_eq!(m.mounts[0].precedence, Some(Precedence::AfterEnv));
        let back = PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn mounts_union_after_slot_round_trips() {
        let text = format!(
            "{HEADER}mounts:\n  - {{slot: 2, point: /opt/x, mode: union, precedence: 'after:1'}}\n"
        );
        let m = PackageManifest::from_yaml(&text).unwrap();
        assert_eq!(m.mounts[0].mode, MountMode::Union);
        assert_eq!(m.mounts[0].precedence, Some(Precedence::AfterSlot(1)));
        let back = PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn mounts_cow_and_enc_are_named_reserved_mode_errors() {
        for spelling in ["cow", "enc"] {
            let text = format!("{HEADER}mounts:\n  - {{slot: 0, point: /x, mode: {spelling}}}\n");
            let err = PackageManifest::from_yaml(&text).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains(&format!("mode '{spelling}' is reserved")),
                "{spelling}: {msg}"
            );
        }
        // An unknown spelling is a structural error, never a silent skip.
        let text = format!("{HEADER}mounts:\n  - {{slot: 0, point: /x, mode: bogus}}\n");
        assert!(PackageManifest::from_yaml(&text).is_err());
    }

    #[test]
    fn mounts_semantic_rejections() {
        let bad =
            |rows: &str| PackageManifest::from_yaml(&format!("{HEADER}mounts:\n{rows}")).is_err();
        // union without precedence
        assert!(bad("  - {slot: 0, point: /x, mode: union}\n"));
        // precedence on an exclusive row
        assert!(bad("  - {slot: 0, point: /x, precedence: after-env}\n"));
        // empty point
        assert!(bad("  - {slot: 0, point: ''}\n"));
        // slot out of capacity
        assert!(bad("  - {slot: 8, point: /x}\n"));
        // precedence naming the row's own slot
        assert!(bad(
            "  - {slot: 1, point: /x, mode: union, precedence: 'after:1'}\n"
        ));
        // precedence slot out of capacity
        assert!(bad(
            "  - {slot: 1, point: /x, mode: union, precedence: 'after:8'}\n"
        ));
        // duplicate slot rows
        assert!(bad("  - {slot: 0, point: /x}\n  - {slot: 0, point: /y}\n"));
        // a malformed precedence spelling
        assert!(bad(
            "  - {slot: 0, point: /x, mode: union, precedence: first}\n"
        ));
    }

    // ---------------------------------------------------------------
    // The press-time lock (spec 23 §4/§13)
    // ---------------------------------------------------------------

    fn sha(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn carried_slice() -> LockedSlice {
        LockedSlice {
            name: "metanorma".to_string(),
            version: "2.1.4".to_string(),
            carry: true,
            slot: Some(0),
            mount: Some("/__tfs__".to_string()),
            sha256: DigestPin::One(sha('a')),
            source: Some(
                "tfs:github:tebako-packages/metanorma-feedstock:2.1.4#metanorma-2.1.4.tfs"
                    .to_string(),
            ),
        }
    }

    fn shared_slice() -> LockedSlice {
        LockedSlice {
            name: "openjdk".to_string(),
            version: "21.0.5".to_string(),
            carry: false,
            slot: None,
            mount: Some("/opt/openjdk".to_string()),
            sha256: DigestPin::PerTriplet(BTreeMap::from([(
                "macos-arm64".to_string(),
                sha('b'),
            )])),
            source: Some(
                "tfs:github:tebako-packages/openjdk-feedstock:21.0.5#openjdk-21.0.5-macos-arm64.tfs"
                    .to_string(),
            ),
        }
    }

    fn carried_lock() -> PackageLock {
        PackageLock {
            runtime: Some(LockedRuntime {
                version: "3.3.7".to_string(),
                carry: true,
                exe: Some(LockedArtifact {
                    slot: 1,
                    sha256: DigestPin::PerTriplet(BTreeMap::from([(
                        "macos-arm64".to_string(),
                        sha('c'),
                    )])),
                    install_as: None,
                }),
                image: Some(LockedArtifact {
                    slot: 2,
                    sha256: DigestPin::PerTriplet(BTreeMap::from([(
                        "macos-arm64".to_string(),
                        sha('d'),
                    )])),
                    install_as: None,
                }),
                dll: None,
            }),
            slices: vec![carried_slice(), shared_slice()],
            spawned: vec![],
        }
    }

    #[test]
    fn lock_round_trips_and_stays_absent_when_none() {
        // absent lock: the v1 shape, byte-stable
        let m = minimal();
        let text = m.to_yaml().unwrap();
        assert!(!text.contains("lock"), "{text}");

        let mut m = minimal();
        m.lock = Some(carried_lock());
        let text = m.to_yaml().unwrap();
        let back = PackageManifest::from_yaml(&text).unwrap();
        assert_eq!(back, m);
    }

    /// A carried spawned-runtime row (spec 23 §13.6): the java pair.
    fn spawned_java() -> LockedSpawnedRuntime {
        LockedSpawnedRuntime {
            engine: "java".to_string(),
            implementation: None,
            constraint: crate::manifest::Constraint::new(">= 21, < 26").unwrap(),
            expose: vec!["java".to_string()],
            version: "21.0.12".to_string(),
            tebako: "2.1.5".to_string(),
            carry: true,
            exe: LockedSpawnedArtifact {
                slot: Some(3),
                sha256: DigestPin::PerTriplet(BTreeMap::from([(
                    "macos-arm64".to_string(),
                    sha('e'),
                )])),
                install_as: None,
            },
            image: LockedSpawnedArtifact {
                slot: Some(4),
                sha256: DigestPin::PerTriplet(BTreeMap::from([(
                    "macos-arm64".to_string(),
                    sha('f'),
                )])),
                install_as: None,
            },
            dll: None,
            source: Some(
                "tfs:github:tebako-packages/openjdk:2.1.5#tebako-runtime-2.1.5-21.0.12-macos-arm64.tfs"
                    .to_string(),
            ),
        }
    }

    /// A carried spawned-PAYLOAD row (spec 32 §6): the xml2rfc provider
    /// image with its own resolved python pair nested.
    fn spawned_xml2rfc() -> LockedSpawnedPayload {
        let mut runtime = spawned_java();
        runtime.engine = "python".to_string();
        runtime.expose = Vec::new();
        runtime.constraint = crate::manifest::Constraint::new(">= 3.10").unwrap();
        runtime.version = "3.13.15".to_string();
        runtime.tebako = "2.1.10".to_string();
        runtime.exe.slot = Some(5);
        runtime.image.slot = Some(6);
        runtime.source = Some(
            "tfs:github:tebako-packages/python:2.1.10#tebako-runtime-2.1.10-3.13.15-macos-arm64.tfs"
                .to_string(),
        );
        LockedSpawnedPayload {
            payload: "xml2rfc".to_string(),
            constraint: crate::manifest::Constraint::new(">= 3.34").unwrap(),
            expose: vec!["xml2rfc".to_string()],
            version: "3.34.0".to_string(),
            carry: true,
            image: LockedSpawnedArtifact {
                slot: Some(7),
                sha256: DigestPin::PerTriplet(BTreeMap::from([(
                    "macos-arm64".to_string(),
                    sha('a'),
                )])),
                install_as: None,
            },
            runtime,
            source: Some(
                "tfs:github:tebako-packages/xml2rfc:3.34.0#xml2rfc-3.34.0.tfs".to_string(),
            ),
        }
    }

    #[test]
    fn spawned_rows_round_trip_and_claim_their_slots() {
        let mut lock = carried_lock();
        lock.spawned.push(LockedSpawned::Runtime(spawned_java()));
        let mut m = minimal();
        m.lock = Some(lock);
        let text = m.to_yaml().unwrap();
        let back = PackageManifest::from_yaml(&text).unwrap();
        assert_eq!(back, m);

        // the carried pair's slots join the claimed set (never mounted)
        let claimed = m.lock.as_ref().unwrap().claimed_slots();
        assert!(claimed.contains(&3) && claimed.contains(&4), "{claimed:?}");
    }

    #[test]
    fn spawned_payload_rows_round_trip_and_claim_their_slots() {
        // spec 32 §6: the payload row (provider image + the nested
        // runtime pair) serializes beside the runtime rows in the SAME
        // spawned[] list and claims its three slots.
        let mut lock = carried_lock();
        lock.spawned.push(LockedSpawned::Runtime(spawned_java()));
        lock.spawned.push(LockedSpawned::Payload(spawned_xml2rfc()));
        let mut m = minimal();
        m.lock = Some(lock);
        let text = m.to_yaml().unwrap();
        assert!(text.contains("payload: xml2rfc"), "{text}");
        let back = PackageManifest::from_yaml(&text).unwrap();
        assert_eq!(back, m);
        let Some(LockedSpawned::Payload(row)) = back.lock.as_ref().unwrap().spawned.get(1) else {
            panic!("payload row, got {:?}", back.lock.as_ref().unwrap().spawned);
        };
        assert_eq!(row.payload, "xml2rfc");
        assert_eq!(row.version, "3.34.0");
        assert_eq!(row.runtime.engine, "python");
        assert_eq!(row.runtime.version, "3.13.15");
        assert_eq!(row.runtime.tebako, "2.1.10");

        let claimed = back.lock.as_ref().unwrap().claimed_slots();
        assert!(
            claimed.contains(&5) && claimed.contains(&6) && claimed.contains(&7),
            "{claimed:?}"
        );
    }

    #[test]
    fn spawned_payload_row_validation_is_fail_closed() {
        let bad = |lock: PackageLock| {
            let mut m = minimal();
            m.lock = Some(lock);
            m.validate().is_err()
        };

        // carried payload row without its image slot
        let mut lock = carried_lock();
        let mut row = spawned_xml2rfc();
        row.image.slot = None;
        lock.spawned.push(LockedSpawned::Payload(row));
        assert!(bad(lock));

        // shared payload row without fetch coordinates
        let mut lock = carried_lock();
        let mut row = spawned_xml2rfc();
        row.carry = false;
        row.image.slot = None;
        row.source = None;
        lock.spawned.push(LockedSpawned::Payload(row));
        assert!(bad(lock));

        // shared payload row declaring a slot
        let mut lock = carried_lock();
        let mut row = spawned_xml2rfc();
        row.carry = false;
        row.source =
            Some("tfs:github:tebako-packages/xml2rfc:3.34.0#xml2rfc-3.34.0.tfs".to_string());
        lock.spawned.push(LockedSpawned::Payload(row));
        assert!(bad(lock));

        // a nested runtime row carrying its own expose list
        let mut lock = carried_lock();
        let mut row = spawned_xml2rfc();
        row.runtime.expose = vec!["python".to_string()];
        lock.spawned.push(LockedSpawned::Payload(row));
        assert!(bad(lock));

        // a nested runtime row breaking the pair rules
        let mut lock = carried_lock();
        let mut row = spawned_xml2rfc();
        row.runtime.image.slot = None;
        lock.spawned.push(LockedSpawned::Payload(row));
        assert!(bad(lock));

        // duplicate provider payload edge
        let mut lock = carried_lock();
        lock.spawned.push(LockedSpawned::Payload(spawned_xml2rfc()));
        lock.spawned.push(LockedSpawned::Payload(spawned_xml2rfc()));
        assert!(bad(lock));

        // a non-bare expose name
        let mut lock = carried_lock();
        let mut row = spawned_xml2rfc();
        row.expose = vec!["/usr/bin/xml2rfc".to_string()];
        lock.spawned.push(LockedSpawned::Payload(row));
        assert!(bad(lock));

        // a payload-row slot colliding with the nested pair's slot
        let mut lock = carried_lock();
        let mut row = spawned_xml2rfc();
        row.image.slot = Some(5);
        lock.spawned.push(LockedSpawned::Payload(row));
        assert!(bad(lock));
    }

    #[test]
    fn spawned_row_validation_is_fail_closed() {
        let bad = |lock: PackageLock| {
            let mut m = minimal();
            m.lock = Some(lock);
            m.validate().is_err()
        };

        // carried row without the pair slots
        let mut lock = carried_lock();
        let mut row = spawned_java();
        row.image.slot = None;
        lock.spawned.push(LockedSpawned::Runtime(row));
        assert!(bad(lock));

        // shared row declaring a slot
        let mut lock = carried_lock();
        let mut row = spawned_java();
        row.carry = false;
        row.exe.slot = None;
        row.image.slot = Some(5);
        row.dll = None;
        lock.spawned.push(LockedSpawned::Runtime(row));
        assert!(bad(lock));

        // shared row without fetch coordinates
        let mut lock = carried_lock();
        let mut row = spawned_java();
        row.carry = false;
        row.exe.slot = None;
        row.image.slot = None;
        row.source = None;
        lock.spawned.push(LockedSpawned::Runtime(row));
        assert!(bad(lock));

        // duplicate engine edge
        let mut lock = carried_lock();
        lock.spawned.push(LockedSpawned::Runtime(spawned_java()));
        lock.spawned.push(LockedSpawned::Runtime(spawned_java()));
        assert!(bad(lock));

        // a non-bare expose name
        let mut lock = carried_lock();
        let mut row = spawned_java();
        row.expose = vec!["/usr/bin/java".to_string()];
        lock.spawned.push(LockedSpawned::Runtime(row));
        assert!(bad(lock));

        // a carried pair slot colliding with a carried slice slot
        let mut lock = carried_lock();
        let mut row = spawned_java();
        row.exe.slot = Some(0);
        lock.spawned.push(LockedSpawned::Runtime(row));
        assert!(bad(lock));

        // an empty engine
        let mut lock = carried_lock();
        let mut row = spawned_java();
        row.engine = String::new();
        lock.spawned.push(LockedSpawned::Runtime(row));
        assert!(bad(lock));
    }

    #[test]
    fn lock_validation_is_fail_closed() {
        let bad = |lock: PackageLock| {
            let mut m = minimal();
            m.lock = Some(lock);
            m.validate().is_err()
        };

        // carried runtime without the pair slots
        let mut lock = carried_lock();
        lock.runtime.as_mut().unwrap().image = None;
        assert!(bad(lock));

        // shared runtime carrying slots
        let mut lock = carried_lock();
        let rt = lock.runtime.as_mut().unwrap();
        rt.carry = false;
        assert!(bad(lock));

        // carried slice without a slot
        let mut lock = carried_lock();
        lock.slices[0].slot = None;
        assert!(bad(lock));

        // shared slice with a slot
        let mut lock = carried_lock();
        lock.slices[1].slot = Some(3);
        assert!(bad(lock));

        // shared slice without fetch coordinates
        let mut lock = carried_lock();
        lock.slices[1].source = None;
        assert!(bad(lock));

        // duplicate slice names
        let mut lock = carried_lock();
        lock.slices.push(lock.slices[1].clone());
        assert!(bad(lock));

        // two artifacts claiming one slot (the app slice and the exe)
        let mut lock = carried_lock();
        lock.runtime.as_mut().unwrap().exe.as_mut().unwrap().slot = 0;
        assert!(bad(lock));

        // slot outside the container's capacity
        let mut lock = carried_lock();
        lock.runtime.as_mut().unwrap().image.as_mut().unwrap().slot = TPKG_MAX_SLOTS;
        assert!(bad(lock));

        // a malformed digest
        let mut lock = carried_lock();
        lock.slices[0].sha256 = DigestPin::One("not-hex".to_string());
        assert!(bad(lock));

        // an unknown triplet key in the digest map
        let mut lock = carried_lock();
        lock.slices[1].sha256 =
            DigestPin::PerTriplet(BTreeMap::from([("plan9-mips".to_string(), sha('b'))]));
        assert!(bad(lock));

        // a relative mount point
        let mut lock = carried_lock();
        lock.slices[0].mount = Some("relative".to_string());
        assert!(bad(lock));
    }

    #[test]
    fn lock_entry_slice_agreement_is_checked() {
        // entry slot and the carried slice's slot must agree
        let mut m = minimal();
        m.lock = Some(carried_lock());
        m.entries[0].slot = Some(3);
        assert!(m.validate().is_err());

        // a slot-less entry backed by a shared lock slice of the same name
        let mut m = minimal();
        m.entries[0].name = "openjdk".to_string();
        m.entries[0].slot = None;
        m.lock = Some(PackageLock {
            runtime: None,
            slices: vec![shared_slice()],
            spawned: vec![],
        });
        m.validate().unwrap();

        // ... but a CARRIED slice of the entry's name refuses the slot-less entry
        let mut m = minimal();
        m.entries[0].slot = None;
        m.lock = Some(PackageLock {
            runtime: None,
            slices: vec![carried_slice()],
            spawned: vec![],
        });
        assert!(m.validate().is_err());
    }

    #[test]
    fn digest_pin_host_lookup() {
        let one = DigestPin::One(sha('a'));
        assert_eq!(
            one.for_host(crate::Platform::Aarch64Macos),
            Some(sha('a').as_str())
        );
        let map = DigestPin::PerTriplet(BTreeMap::from([("macos-arm64".to_string(), sha('b'))]));
        assert_eq!(
            map.for_host(crate::Platform::Aarch64Macos),
            Some(sha('b').as_str())
        );
        assert_eq!(map.for_host(crate::Platform::X86_64LinuxGnu), None);
    }
}
