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
//! unknown keys are tolerated for forward compatibility (only `jail` is
//! preserved verbatim — its shape is spec 08's, not this schema's), and
//! the versioned JSON Schema `schema/tpkg-package-manifest-v1.schema.json`
//! pins the structure.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

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
}

impl fmt::Display for PackageManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManifestError::Yaml(e) => write!(f, "package manifest yaml error: {e}"),
            PackageManifestError::Invalid(m) => write!(f, "invalid package manifest: {m}"),
        }
    }
}

impl std::error::Error for PackageManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PackageManifestError::Yaml(e) => Some(e),
            PackageManifestError::Invalid(_) => None,
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

/// The L2 package manifest (spec 03 §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub package: PackageIdentity,
    /// One entry per invocable command (N >= 1).
    pub entries: Vec<PackageEntry>,
    /// Package-level jail request (spec 08 owns the shape — preserved
    /// verbatim, not interpreted here).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jail: Option<BTreeMap<String, serde_yml::Value>>,
    /// Package-level env (composition rules: spec 07).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
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
    /// env keys. Unknown keys are tolerated (only `jail` is lossless).
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
    fn jail_is_preserved_verbatim() {
        let text = "schema_version: 1\n\
                    package: {name: x, version: 1.0.0, producer: {tool: t, tool_version: 1}, created: now}\n\
                    entries:\n  - {name: x, slot: 0, entrypoint: x, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n\
                    jail: {profile: strict, allow: [read, write]}\n";
        let m = PackageManifest::from_yaml(text).unwrap();
        let jail = m.jail.as_ref().unwrap();
        assert!(jail.contains_key("profile"));
        let rendered = m.to_yaml().unwrap();
        let back = PackageManifest::from_yaml(&rendered).unwrap();
        assert_eq!(back, m);
    }
}
