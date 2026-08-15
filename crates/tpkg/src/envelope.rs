//! The envelope manifest (`/__tpkg__/envelopes.yaml`, spec 10 §2): the
//! in-image record of DEK grant envelopes — who may open which subtree.
//!
//! Per spec 03 §2.1 the payload manifest's `encryption.parts` carry only
//! `{paths, algorithm, envelope_refs}` — NEVER keys. The wrapped DEKs
//! themselves live HERE, one YAML document beside the payload manifest,
//! inside the image (sealed with it) but outside the tree hash (the
//! spec-03 §7 exclusion covers all of `/__tpkg__/`). A wrapped DEK plus
//! the manifest digest IS a capability: possessing it grants exactly
//! that subtree, nothing else.
//!
//! # Wire shape (v1)
//!
//! ```yaml
//! schema_version: 1
//! suite: SUITE-1          # versioned cipher-suite registry (spec 10 §5)
//! grants:
//!   - id: root            # grant id; manifest envelope_refs point here
//!     path: /             # subtree root this grant opens ("/" = whole image)
//!     recipients: ["0123456789abcdef"]   # informational keyids (16 hex)
//!     envelope: |         # ONE ASCII-armored OpenPGP message: the DEK
//!       -----BEGIN PGP MESSAGE-----      # wrapped as PKESK packets to the
//!       ...                                # recipients (tebako-signer)
//! ```
//!
//! **Algorithm agility from day one** (spec 10 §5): `suite` is a
//! registry id, never implied. Unknown ids fail with a named error,
//! never a guess — v1 implements SUITE-1 only (X25519 + Ed25519 +
//! AES-256-GCM + SHA-256); SUITE-2/3 (hybrid PQC) are registry growth,
//! not a format change.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::manifest::ManifestError;

/// Well-known in-image path of the envelope manifest.
pub const ENVELOPES_PATH: &str = "/__tpkg__/envelopes.yaml";

/// The only `schema_version` this implementation reads and writes.
pub const ENVELOPES_SCHEMA_VERSION: u32 = 1;

/// The versioned cipher-suite registry (spec 10 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    /// X25519 + Ed25519 + AES-256-GCM + SHA-256 (classical).
    Suite1,
}

impl Suite {
    /// The registry id (`SUITE-1`).
    pub fn as_id(self) -> &'static str {
        match self {
            Suite::Suite1 => "SUITE-1",
        }
    }

    /// Parse a registry id. Unknown ids are a named error, never a
    /// guess (algorithm agility, spec 10 §5).
    pub fn from_id(id: &str) -> Result<Suite, ManifestError> {
        match id {
            "SUITE-1" => Ok(Suite::Suite1),
            other => Err(ManifestError::Invalid(match other {
                // Known-but-unimplemented registry members name themselves.
                "SUITE-2" | "SUITE-3" => "cipher suite is registered but not implemented in v1 (PQC suites land with the hermetic rnp swap)",
                _ => "unknown cipher suite id (the registry is versioned; never guess)",
            })),
        }
    }
}

impl fmt::Display for Suite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_id())
    }
}

impl Serialize for Suite {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_id())
    }
}

impl<'de> Deserialize<'de> for Suite {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Suite, D::Error> {
        let s = String::deserialize(d)?;
        Suite::from_id(&s).map_err(serde::de::Error::custom)
    }
}

/// One wrapped-DEK grant: the recipients may open exactly the subtree
/// rooted at `path` (the whole image for `/`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// Grant id (the payload manifest's `envelope_refs` point here).
    pub id: String,
    /// Subtree root (absolute, `/` = the whole image).
    pub path: String,
    /// Informational recipient keyids (16 lowercase hex each) — the
    /// envelope's PKESK slots are the authority; this is the
    /// human/audit view.
    pub recipients: Vec<String>,
    /// The wrapped DEK: ONE ASCII-armored OpenPGP message (PKESK per
    /// recipient), produced by tebako-signer's `wrap_dek`.
    pub envelope: String,
}

/// The envelope manifest: the suite plus every grant of the image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeManifest {
    pub schema_version: u32,
    pub suite: Suite,
    pub grants: Vec<Grant>,
}

/// The OpenPGP keyid form (SSOT — spec 03 §2.1's keyid, the trailer's v2
/// extension, envelope recipients, and spec 24's `pgp:<keyid>` reference
/// all speak it): exactly 16 lowercase hex chars, the low 64 bits of the
/// OpenPGP fingerprint.
pub fn is_valid_keyid(keyid: &str) -> bool {
    keyid.len() == 16
        && keyid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn check_keyid(keyid: &str) -> Result<(), ManifestError> {
    if !is_valid_keyid(keyid) {
        return Err(ManifestError::Invalid(
            "envelope manifest recipient keyids must be 16 lowercase hex",
        ));
    }
    Ok(())
}

/// Normalize an absolute subtree path: exactly one leading `/`, no
/// trailing `/` (except the root itself), no empty components.
fn normalize_grant_path(path: &str) -> Result<String, ManifestError> {
    if !path.starts_with('/') {
        return Err(ManifestError::Invalid(
            "envelope manifest grant paths must be absolute",
        ));
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return Ok("/".to_string());
    }
    // `trimmed` starts with exactly one `/` here; components are what
    // follows, and none may be empty (`//`, `/a//b`).
    if trimmed[1..].split('/').any(|c| c.is_empty()) {
        return Err(ManifestError::Invalid(
            "envelope manifest grant paths must not contain empty components",
        ));
    }
    Ok(trimmed.to_string())
}

impl EnvelopeManifest {
    /// Parse and validate from YAML text.
    pub fn from_yaml(text: &str) -> Result<EnvelopeManifest, ManifestError> {
        let mut manifest: EnvelopeManifest = serde_yml::from_str(text)?;
        manifest.validate()?;
        // Paths are normalized on read so comparisons are exact.
        for grant in &mut manifest.grants {
            grant.path = normalize_grant_path(&grant.path)?;
        }
        Ok(manifest)
    }

    /// Validate and serialize to YAML text.
    pub fn to_yaml(&self) -> Result<String, ManifestError> {
        self.validate()?;
        Ok(serde_yml::to_string(self)?)
    }

    /// Semantic checks: schema version, unique ids, unique normalized
    /// absolute paths, ≥ 1 recipient per grant, keyid shapes, an armored
    /// OpenPGP envelope. Unknown suite ids already failed at parse time
    /// (the serde `Suite` binding — agility, not guessing).
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != ENVELOPES_SCHEMA_VERSION {
            return Err(ManifestError::Invalid(
                "envelope manifest schema_version is not supported",
            ));
        }
        if self.grants.is_empty() {
            return Err(ManifestError::Invalid(
                "envelope manifest must carry at least one grant",
            ));
        }
        for grant in &self.grants {
            if grant.id.is_empty() {
                return Err(ManifestError::Invalid(
                    "envelope grant ids must not be empty",
                ));
            }
            if self.grants.iter().filter(|g| g.id == grant.id).count() > 1 {
                return Err(ManifestError::Invalid("duplicate envelope grant id"));
            }
            let normalized = normalize_grant_path(&grant.path)?;
            if self
                .grants
                .iter()
                .filter(|g| normalize_grant_path(&g.path).ok().as_deref() == Some(&*normalized))
                .count()
                > 1
            {
                return Err(ManifestError::Invalid("duplicate envelope grant path"));
            }
            if grant.recipients.is_empty() {
                return Err(ManifestError::Invalid(
                    "envelope grants require at least one recipient",
                ));
            }
            for keyid in &grant.recipients {
                check_keyid(keyid)?;
            }
            if !grant.envelope.starts_with("-----BEGIN PGP MESSAGE-----") {
                return Err(ManifestError::Invalid(
                    "envelope grant envelopes must be ASCII-armored OpenPGP messages",
                ));
            }
        }
        Ok(())
    }

    /// The grant with this id (manifest `envelope_refs` resolution).
    pub fn grant_by_id(&self, id: &str) -> Option<&Grant> {
        self.grants.iter().find(|g| g.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARMORED: &str =
        "-----BEGIN PGP MESSAGE-----\n\nwcBMAyQJ1ER4NVrzAQgAAQEA\n-----END PGP MESSAGE-----\n";

    fn doc() -> String {
        format!("schema_version: 1\nsuite: SUITE-1\ngrants:\n  - id: root\n    path: /\n    recipients: [0123456789abcdef]\n    envelope: |\n      {}", ARMORED.replace('\n', "\n      ").trim_end())
    }

    #[test]
    fn roundtrip_and_lookup() {
        let m = EnvelopeManifest::from_yaml(&doc()).unwrap();
        assert_eq!(m.schema_version, 1);
        assert_eq!(m.suite, Suite::Suite1);
        assert_eq!(m.grants.len(), 1);
        assert!(m.grant_by_id("root").is_some());
        assert!(m.grant_by_id("nope").is_none());
        let text = m.to_yaml().unwrap();
        let m2 = EnvelopeManifest::from_yaml(&text).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn unknown_suite_is_a_named_error_never_a_guess() {
        assert!(matches!(
            Suite::from_id("SUITE-2"),
            Err(ManifestError::Invalid(m)) if m.contains("registered but not implemented")
        ));
        assert!(matches!(
            Suite::from_id("SUITE-9"),
            Err(ManifestError::Invalid(m)) if m.contains("unknown cipher suite id")
        ));
        let bad = doc().replace("SUITE-1", "rot13");
        assert!(EnvelopeManifest::from_yaml(&bad).is_err());
    }

    #[test]
    fn paths_normalize_and_duplicates_fail() {
        let m = EnvelopeManifest::from_yaml(&doc().replace("path: /", "path: /a/b/")).unwrap();
        assert_eq!(m.grants[0].path, "/a/b");
        let dup = format!("{}\n  - id: other\n    path: /a/b\n    recipients: [0123456789abcdef]\n    envelope: |\n      {}", doc().replace("path: /", "path: /a/b"), ARMORED.replace('\n', "\n      ").trim_end());
        assert!(matches!(
            EnvelopeManifest::from_yaml(&dup),
            Err(ManifestError::Invalid(m)) if m.contains("duplicate envelope grant path")
        ));
        let relative = doc().replace("path: /", "path: a/b");
        assert!(EnvelopeManifest::from_yaml(&relative).is_err());
    }

    #[test]
    fn recipient_and_envelope_shapes_are_checked() {
        let bad_keyid = doc().replace("0123456789abcdef", "XYZ");
        assert!(EnvelopeManifest::from_yaml(&bad_keyid).is_err());
        let no_recipients = doc().replace("[0123456789abcdef]", "[]");
        assert!(EnvelopeManifest::from_yaml(&no_recipients).is_err());
        let not_armored = doc().replace("-----BEGIN PGP MESSAGE-----", "not pgp");
        assert!(EnvelopeManifest::from_yaml(&not_armored).is_err());
        let no_grants = "schema_version: 1\nsuite: SUITE-1\ngrants: []\n";
        assert!(EnvelopeManifest::from_yaml(no_grants).is_err());
        let bad_version = doc().replace("schema_version: 1", "schema_version: 2");
        assert!(EnvelopeManifest::from_yaml(&bad_version).is_err());
    }
}
