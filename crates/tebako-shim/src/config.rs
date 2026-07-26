//! Authored and managed configuration surfaces (all YAML — spec 00
//! invariant 6, spec 07 §4):
//!
//! - `~/.tebako/config.yaml` — USER-authored: `defaults:` (per-tool
//!   version, written by `tebako use <tool>@<version>`), `registries:`
//!   (spec 04 refs), `runtimes:` (per-engine runtime preferences, written
//!   by `tebako use --runtime <engine>@<version>`). The shim only READS
//!   this file — user comments and formatting stay intact.
//! - `~/.tebako/shims/.disabled.yaml` — SHIM-managed state (enable /
//!   disable). Kept out of the authored config so `tebako-shim disable`
//!   never rewrites a hand-maintained file.
//! - `tpkg-registry.yaml` — the developer-hosted registry (spec 04 §2).
//!   v1 resolves `file://` refs and plain paths only; remote registry
//!   fetch lands with the registry resolver (roadmap 07).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{fail, ShimError, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

#[derive(Debug, Default, Deserialize)]
pub struct UserConfig {
    /// Tool/command name → version (`tebako use <tool>@<version>`).
    #[serde(default)]
    pub defaults: BTreeMap<String, String>,
    /// Spec 04 registry refs. v1: `file://` refs and plain local paths.
    #[serde(default)]
    pub registries: Vec<String>,
    /// Engine → runtime preference (the download fallback of spec 05 §5:
    /// "download the newest compatible" needs an exact ref; the
    /// preference names it until the runtime registry ships).
    #[serde(default)]
    pub runtimes: BTreeMap<String, RuntimePref>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimePref {
    /// Language version, e.g. `4.0.6`.
    pub version: String,
    /// The tebako (launcher) abi version the runtime was built with,
    /// e.g. `0.16.0` — the `<ver>` of the cache layout
    /// `runtimes/<lang>-<lv>-<ver>-<triplet>/`.
    pub tebako: String,
}

pub fn config_path(home: &Path) -> PathBuf {
    home.join("config.yaml")
}

pub fn load_config(home: &Path) -> Result<UserConfig, ShimError> {
    let path = config_path(home);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(UserConfig::default()),
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    serde_yaml::from_str(&text).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!(
                "cannot parse {} ({e}) — fix or remove it; run `tebako-shim doctor`",
                path.display()
            ),
        )
    })
}

// ---------------------------------------------------------------------
// registry mirror (spec 04 §2) — resolution-relevant fields only
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Registry {
    #[serde(default)]
    payloads: Vec<RegistryPayload>,
}

#[derive(Debug, Deserialize)]
struct RegistryPayload {
    name: String,
    #[serde(default)]
    default: Option<String>,
}

/// The registry default version for `payload_name`, scanning the user's
/// registered registries in order (first match wins).
pub fn registry_default(
    config: &UserConfig,
    payload_name: &str,
) -> Result<Option<(String, String)>, ShimError> {
    for reg_ref in &config.registries {
        let path = match local_registry_path(reg_ref) {
            Some(p) => p,
            None => {
                return fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "registry ref \"{reg_ref}\" is not a local file — remote registry resolution lands with roadmap 07; use a file:// ref for now"
                    ),
                )
            }
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return fail(
                    EX_TEBAKO_IO,
                    format!("cannot read registry {}: {e}", path.display()),
                )
            }
        };
        let registry: Registry = serde_yaml::from_str(&text).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!("cannot parse registry {} ({e})", path.display()),
            )
        })?;
        if let Some(p) = registry.payloads.iter().find(|p| p.name == payload_name) {
            if let Some(default) = &p.default {
                return Ok(Some((default.clone(), reg_ref.clone())));
            }
        }
    }
    Ok(None)
}

fn local_registry_path(reg_ref: &str) -> Option<PathBuf> {
    if let Some(rest) = reg_ref.strip_prefix("file://") {
        Some(PathBuf::from(rest))
    } else if reg_ref.starts_with('/') || reg_ref.starts_with("./") || reg_ref.starts_with("../") {
        Some(PathBuf::from(reg_ref))
    } else {
        None
    }
}

// ---------------------------------------------------------------------
// disabled state (shim-managed; never interleaved with authored config)
// ---------------------------------------------------------------------

/// Tool name → list of disabled selectors: exact versions, or `all`.
pub type Disabled = BTreeMap<String, Vec<String>>;

pub fn disabled_path(home: &Path) -> PathBuf {
    home.join("shims").join(".disabled.yaml")
}

pub fn load_disabled(home: &Path) -> Result<Disabled, ShimError> {
    let path = disabled_path(home);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Disabled::default()),
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    serde_yaml::from_str(&text).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot parse {} ({e})", path.display()),
        )
    })
}

pub fn save_disabled(home: &Path, disabled: &Disabled) -> Result<(), ShimError> {
    let dir = home.join("shims");
    std::fs::create_dir_all(&dir).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", dir.display()),
        )
    })?;
    let path = disabled_path(home);
    let tmp = dir.join(format!(".disabled.yaml.{}.tmp", std::process::id()));
    let text = serde_yaml::to_string(disabled).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot serialize disabled state: {e}"),
        )
    })?;
    std::fs::write(&tmp, text).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })
}

pub fn is_disabled(disabled: &Disabled, tool: &str, version: &str) -> bool {
    disabled
        .get(tool)
        .is_some_and(|selectors| selectors.iter().any(|s| s == "all" || s == version))
}
