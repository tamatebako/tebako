//! The minimal payload-manifest surface the dispatcher resolves against.
//!
//! TODO(manifest-unify): the normative tpkg manifest model (spec 03,
//! `schema/tpkg-manifest-v1.schema.json`) is being built concurrently on
//! `feat/manifest-format`. DO NOT couple to that branch: this module
//! defines exactly the fields spec 07 dispatch needs —
//! `{name, version, entrypoints[], requires[]}` — and parses them from
//! the mirrored manifest copy in the installed payload record
//! (`~/.tebako/payloads/<name>/<version>.manifest.yaml`). When the shared
//! model lands, replace this module with it and delete this note.

use serde::{Deserialize, Serialize};

use crate::{fail, ShimError, EX_TEBAKO_MANIFEST};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrypoints: Vec<Entrypoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Require>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRequirement {
    pub engine: String,
    /// Range form (`>= 3.3, < 5.0`) for pure-language payloads; abi-line
    /// form (`~> 3.3.0`) for native-extension payloads (spec 05 §5).
    pub constraint: String,
}

/// A DEPENDS edge (spec 03 §2.3). Only the resolution fields the
/// dispatcher needs are modeled; `mount` is consumer-declared (locked
/// MOUNT RULE). `kind: language` edges are the runtime axis and are
/// resolved through the entrypoint's `runtime_requirement`, never mounted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Require {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount: Option<String>,
}

impl Manifest {
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
