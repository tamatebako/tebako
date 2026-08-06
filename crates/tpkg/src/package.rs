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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    /// The command name (the shim registers under this).
    pub name: String,
    /// Which payload slot carries the entrypoint's image.
    pub slot: u32,
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
            if entry.slot >= TPKG_MAX_SLOTS {
                return Err(PackageManifestError::Invalid(
                    "entries[].slot is outside the container's slot capacity (0..TPKG_MAX_SLOTS-1)",
                ));
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
                slot: 0,
                entrypoint: "metanorma".to_string(),
                runtime_ref: "ruby@3.4.2;tebako=0.15.9".to_string(),
            }],
            jail: None,
            env: BTreeMap::new(),
            mounts: Vec::new(),
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
        m.entries[0].slot = TPKG_MAX_SLOTS;
        assert!(bad(&m));

        let mut m = minimal();
        m.package.name.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.package.created.clear();
        assert!(bad(&m));

        let mut m = minimal();
        m.entries.push(m.entries[0].clone()); // duplicate name
        m.entries[1].slot = 1;
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
}
