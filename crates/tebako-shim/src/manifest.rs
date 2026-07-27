//! The dispatcher-visible payload-manifest mirror (spec 07 §0, spec 03 §4
//! tier 3): the resolution fields — and ONLY those — of the unified tpkg
//! manifest model, mirrored next to the installed image so dispatch never
//! opens the payload.
//!
//! Roadmap 40 (manifest unification): the field TYPES come from `tpkg`
//! (the single source of truth for the manifest format, spec 03):
//!
//! - `requires` edges ARE [`tpkg::Requirement`] (the tagged
//!   language/toolkit/data union — consumer-declared `mount`, locked
//!   MOUNT RULE);
//! - an entrypoint's `runtime_requirement` IS
//!   [`tpkg::RuntimeRequirement`], its constraint a validated
//!   [`tpkg::Constraint`];
//! - [`Manifest::mirror_of`] maps a full [`tpkg::PayloadManifest`] (the
//!   in-image tier-1 layer) down to this tier-3 mirror.
//!
//! What stays shim-local: the mirror's FLAT wire shape (`{name, version,
//! entrypoints, requires}` — the payload record's `<version>.manifest.yaml`
//! is a mirror, not a full identity-carrying manifest) and the
//! `Option`-ness of `runtime_requirement` (`None` = native / self-contained
//! entrypoint, zero-runtime dispatch, spec 03 §2.2 — the unified model's
//! app entrypoint always requires one, the mirror must also carry
//! synthesized native entries).
//!
//! Constraint handling is split BY DESIGN: [`tpkg::Constraint`] owns the
//! grammar (parse/validate — cargo-style ranges, ruby abi lines, datever)
//! and is deliberately parse-only; EVALUATION against concrete versions
//! (the spec 05 §5 compatibility model, pessimistic `~>` semantics) lives
//! in [`crate::versions`] — tpkg implements no matcher, so the shim's
//! matcher stays. Its grammar is a superset of tpkg's (no 4-component
//! limit), so every constraint that survives manifest parse also parses
//! in the matcher.

use serde::{Deserialize, Serialize};

pub use tpkg::{Constraint, Requirement, RuntimeRequirement};

use crate::{fail, ShimError, EX_TEBAKO_MANIFEST};

/// The tier-3 mirror: exactly the fields spec 07 dispatch resolves
/// against, sourced from the unified model (see the module docs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrypoints: Vec<Entrypoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
}

/// One mirrored entrypoint. Identical in shape to [`tpkg::Entrypoint`]
/// except that `runtime_requirement` is optional (native entrypoints —
/// the mirror carries what the registry/installer synthesized, not only
/// what a full app manifest declares).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entrypoint {
    /// The command name; the shim registers and dispatches under it.
    pub name: String,
    /// The executable inside the payload image (the `--tebako-entry`).
    pub path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args_default: Vec<String>,
    /// `None` = native / self-contained entrypoint: zero-runtime dispatch
    /// (spec 03 §2.2 locked); the dispatcher mounts zero runtime payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_requirement: Option<RuntimeRequirement>,
}

impl Manifest {
    /// The tier-3 mirror of a full payload manifest (the unified model's
    /// PROVIDES/DEPENDS projected onto the dispatch surface). App
    /// entrypoints map one-to-one; other kinds provide no commands.
    /// `name`/`version` are the install's authoritative identity (the
    /// registry / reference), not re-read from the manifest.
    pub fn mirror_of(manifest: &tpkg::PayloadManifest, name: &str, version: &str) -> Manifest {
        let entrypoints = match &manifest.provides {
            tpkg::Provides::App(app) => app
                .entrypoints
                .iter()
                .map(|e| Entrypoint {
                    name: e.name.clone(),
                    path: e.path.clone(),
                    args_default: e.args_default.clone(),
                    runtime_requirement: Some(e.runtime_requirement.clone()),
                })
                .collect(),
            _ => Vec::new(),
        };
        Manifest {
            name: name.to_string(),
            version: version.to_string(),
            entrypoints,
            requires: manifest.requires.clone(),
        }
    }

    pub fn parse(yaml: &str, source: &std::path::Path) -> Result<Manifest, ShimError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "corrupt payload manifest mirror {} ({e}) — the installed payload record is incomplete or damaged; run `tebako-shim doctor`",
                    source.display()
                ),
            )
        })
    }

    pub fn entrypoint(&self, name: &str) -> Option<&Entrypoint> {
        self.entrypoints.iter().find(|e| e.name == name)
    }

    pub fn load(path: &std::path::Path) -> Result<Manifest, ShimError> {
        let text = std::fs::read_to_string(path).map_err(|_| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "installed payload record is missing its manifest mirror {} — the record is incomplete; run `tebako-shim doctor`",
                    path.display()
                ),
            )
        })?;
        Manifest::parse(&text, path)
    }

    /// Write the manifest mirror (the installer's half of the payload
    /// record): tmp + rename, like every cache-managed file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), ShimError> {
        let text = serde_yaml::to_string(self).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!("cannot serialize the manifest mirror: {e}"),
            )
        })?;
        let dir = path.parent().ok_or_else(|| {
            ShimError::new(
                crate::EX_TEBAKO_IO,
                format!("{} has no parent directory", path.display()),
            )
        })?;
        std::fs::create_dir_all(dir).map_err(|e| {
            ShimError::new(
                crate::EX_TEBAKO_IO,
                format!("cannot create {}: {e}", dir.display()),
            )
        })?;
        let tmp = dir.join(format!(
            ".{}.{}.tmp",
            path.file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default(),
            std::process::id()
        ));
        std::fs::write(&tmp, text).map_err(|e| {
            ShimError::new(
                crate::EX_TEBAKO_IO,
                format!("cannot write {}: {e}", tmp.display()),
            )
        })?;
        std::fs::rename(&tmp, path).map_err(|e| {
            ShimError::new(
                crate::EX_TEBAKO_IO,
                format!("cannot install {}: {e}", path.display()),
            )
        })
    }
}

/// The name/form of a payload's versioned files inside
/// `~/.tebako/payloads/<name>/`.
#[derive(Debug)]
pub struct PayloadRecord {
    pub image: std::path::PathBuf,
    pub sha_marker: std::path::PathBuf,
    pub manifest_mirror: std::path::PathBuf,
}

pub fn payload_record(home: &std::path::Path, name: &str, version: &str) -> PayloadRecord {
    let dir = home.join("payloads").join(name);
    PayloadRecord {
        image: dir.join(format!("{version}.tfs")),
        sha_marker: dir.join(format!("{version}.tfs.sha256")),
        manifest_mirror: dir.join(format!("{version}.manifest.yaml")),
    }
}

impl PayloadRecord {
    pub fn installed(&self) -> bool {
        self.image.is_file()
    }
}

/// Reject names/versions that would escape the cache layout (they become
/// path components).
pub fn check_path_component(what: &str, value: &str) -> Result<(), ShimError> {
    if value.is_empty()
        || value
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ' ' | '\t' | '\r' | '\n'))
        || value == "."
        || value == ".."
    {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!("invalid {what} \"{value}\" — it must be a single path component"),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The unified-model round trip: a full in-image manifest (tier 1)
    /// mirrors down to the dispatch surface (tier 3) and the mirror
    /// round-trips losslessly.
    #[test]
    fn mirror_of_a_full_payload_manifest_round_trips() {
        let full = r#"
identity:
  schema_version: 1
  kind: app
  name: metanorma
  version: 1.2.3
  producer: {tool: tebako, tool_version: 0.16.0}
  created: "2026-07-26T00:00:00Z"
  digest:
    tree_hash: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    blob_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  signing: {state: unsigned}
  encryption: {state: none}
provides:
  entrypoints:
    - name: metanorma
      path: /app/bin/metanorma
      args_default: ["--safe"]
      runtime_requirement: {engine: ruby, constraint: ">= 3.3, < 5.0"}
  platforms: universal
  capabilities: {exec: true, read: true}
requires:
  - kind: data
    name: iso-codes
    constraint: ">= 2024.1"
    mount: /__app__/share/iso-codes
"#;
        let payload = tpkg::PayloadManifest::from_yaml(full).unwrap();
        let mirror = Manifest::mirror_of(&payload, "metanorma", "1.2.3");

        let ep = mirror.entrypoint("metanorma").unwrap();
        assert_eq!(ep.path, "/app/bin/metanorma");
        assert_eq!(ep.args_default, vec!["--safe".to_string()]);
        let req = ep.runtime_requirement.as_ref().unwrap();
        assert_eq!(req.engine, "ruby");
        assert_eq!(req.constraint.as_str(), ">= 3.3, < 5.0");
        assert_eq!(mirror.requires.len(), 1);
        match &mirror.requires[0] {
            Requirement::Data {
                name,
                constraint,
                mount,
            } => {
                assert_eq!(name, "iso-codes");
                assert_eq!(constraint.as_str(), ">= 2024.1");
                assert_eq!(mount.as_deref(), Some("/__app__/share/iso-codes"));
            }
            other => panic!("expected a data edge, got {other:?}"),
        }

        // the mirror itself round-trips (save/load is the payload record)
        let dir = std::env::temp_dir().join(format!("tebako-shim-mirror-{}", std::process::id()));
        let path = dir.join("1.2.3.manifest.yaml");
        mirror.save(&path).unwrap();
        let back = Manifest::load(&path).unwrap();
        assert_eq!(back.name, "metanorma");
        assert_eq!(back.entrypoints.len(), 1);
        assert_eq!(back.requires.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A constraint outside the unified grammar is rejected at MIRROR
    /// parse time (tpkg::Constraint validates on deserialize) — not later
    /// at dispatch.
    #[test]
    fn a_bad_constraint_fails_the_mirror_parse() {
        let yaml = "name: x\nversion: 1.0\nentrypoints:\n  - name: x\n    path: /x\n    runtime_requirement: {engine: ruby, constraint: \"~> 3.x\"}\n";
        let err = Manifest::parse(yaml, std::path::Path::new("test.yaml")).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST);
        assert!(
            err.message.contains("corrupt payload manifest mirror"),
            "{}",
            err.message
        );
    }

    /// A `requires` edge of an unknown kind is a structural error (the
    /// unified model locks language/toolkit/data), never a silent skip.
    #[test]
    fn an_unknown_requirement_kind_fails_the_mirror_parse() {
        let yaml = "name: x\nversion: 1.0\nrequires:\n  - kind: widget\n    name: y\n    constraint: \">= 1\"\n";
        assert!(Manifest::parse(yaml, std::path::Path::new("test.yaml")).is_err());
    }
}
