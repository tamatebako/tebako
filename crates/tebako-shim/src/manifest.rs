//! The installed payload's manifest mirror (spec 03 §4 tier 3 — the
//! dispatcher resolves without opening every image):
//! `~/.tebako/payloads/<name>/<version>.manifest.yaml`.
//!
//! The mirror IS the unified payload manifest — [`tpkg::PayloadManifest`]
//! (spec 03, `schema/tpkg-manifest-v1.schema.json`), never a parallel
//! model: the installer (`tebako install`) writes the image's embedded
//! manifest verbatim-tier here (or synthesizes one from the registry's
//! tier-3 fields when the image carries none), and the dispatcher reads
//! exactly the fields spec 07 dispatch needs — `identity.name` /
//! `identity.version`, the app PROVIDES `entrypoints[]` (each with its
//! own `runtime_requirement`; `None` = zero-runtime native entrypoint,
//! spec 03 §2.2), and the DEPENDS `requires[]` edges. Constraint parsing
//! is the manifest model's (tpkg's [`tpkg::Constraint`] validates at
//! parse); the dispatcher only EVALUATES constraints ([`crate::versions`]).

use tpkg::{Entrypoint, PayloadKind, PayloadManifest, Provides, Requirement};

use crate::{fail, ShimError, EX_TEBAKO_MANIFEST};

/// The dispatcher-visible half of an installed payload record: the parsed
/// manifest mirror. Wraps the unified model to keep the shim's named
/// errors (a missing/corrupt mirror points at `tebako-shim doctor`).
#[derive(Debug, Clone)]
pub struct Manifest {
    inner: PayloadManifest,
}

impl Manifest {
    pub fn parse(yaml: &str, source: &std::path::Path) -> Result<Manifest, ShimError> {
        let inner = PayloadManifest::from_yaml(yaml).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "corrupt payload manifest mirror {} ({e}) — the installed payload record is incomplete or damaged; run `tebako-shim doctor`",
                    source.display()
                ),
            )
        })?;
        Ok(Manifest { inner })
    }

    pub fn entrypoint(&self, name: &str) -> Option<&Entrypoint> {
        self.entrypoints().iter().find(|e| e.name == name)
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

    /// Wrap the unified manifest (the installer's half — embedded or
    /// synthesized).
    pub fn from_payload_manifest(inner: PayloadManifest) -> Manifest {
        Manifest { inner }
    }

    /// The unified manifest itself (the registry/publish surfaces need
    /// fields beyond the dispatch minimum).
    pub fn payload_manifest(&self) -> &PayloadManifest {
        &self.inner
    }

    pub fn name(&self) -> &str {
        &self.inner.identity.name
    }

    pub fn version(&self) -> &str {
        &self.inner.identity.version
    }

    pub fn kind(&self) -> PayloadKind {
        self.inner.identity.kind
    }

    /// The app PROVIDES entrypoints (empty for non-app kinds — a data
    /// payload provides no commands).
    pub fn entrypoints(&self) -> &[Entrypoint] {
        match &self.inner.provides {
            Provides::App(app) => &app.entrypoints,
            _ => &[],
        }
    }

    /// The DEPENDS edges (spec 03 §2.3).
    pub fn requires(&self) -> &[Requirement] {
        &self.inner.requires
    }

    /// The payload's declared host-access request (spec 08 §4 — the app
    /// PROVIDES `capabilities.host`; the dispatcher composes it with the
    /// user's tightening flags, spec 08 §2).
    pub fn host_jail(&self) -> Option<&tpkg::HostJail> {
        match &self.inner.provides {
            Provides::App(app) => app.capabilities.host.as_ref(),
            _ => None,
        }
    }

    /// Write the manifest mirror (the installer's half of the payload
    /// record): tmp + rename, like every cache-managed file. The manifest
    /// is re-validated on the way out (`to_yaml`).
    pub fn save(&self, path: &std::path::Path) -> Result<(), ShimError> {
        let text = self.inner.to_yaml().map_err(|e| {
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
