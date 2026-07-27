//! Authored and managed configuration surfaces (all YAML — spec 00
//! invariant 6, spec 07 §4):
//!
//! - `~/.tebako/config.yaml` — USER-authored: `defaults:` (per-tool
//!   version, written by `tebako use <tool>@<version>`), `registries:`
//!   (spec 04 refs), `runtimes:` (per-engine runtime preferences, written
//!   by `tebako use --runtime <engine>@<version>`). The dispatcher only
//!   READS this file; the one write path is `tebako add-registry`
//!   ([`add_registry`]) — a structural edit that preserves keys, not
//!   comments.
//! - `~/.tebako/shims/.disabled.yaml` — SHIM-managed state (enable /
//!   disable). Kept out of the authored config so `tebako-shim disable`
//!   never rewrites a hand-maintained file.
//! - `tpkg-registry.yaml` — the developer-hosted registry (spec 04 §2).
//!   The registry-default chain link resolves every registry form through
//!   tebako-resolve behind the dispatch-time cache ([`crate::regcache`]:
//!   24 h TTL, `tebako update-registries`, `TEBAKO_OFFLINE` = cache-or-
//!   named-error); the registry model is tebako-resolve's (one model,
//!   parse + validate).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{fail, Ctx, ShimError, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

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
// registry registration (the `tebako add-registry` write side)
// ---------------------------------------------------------------------

/// The outcome of [`add_registry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddRegistryOutcome {
    Added,
    AlreadyPresent,
}

/// Append `reg_ref` to `registries:` in `~/.tebako/config.yaml`,
/// preserving every other key. This is the ONE authored-config write the
/// toolchain performs (spec 04 §2: `tebako add-registry <ref>` registers
/// one; the dispatcher itself still never writes this file). The edit is
/// structural (serde_yaml Value surgery), so user comments/formatting are
/// not preserved — keys and values are. The write is tmp + rename, the
/// same discipline as the disabled-state file.
pub fn add_registry(home: &Path, reg_ref: &str) -> Result<AddRegistryOutcome, ShimError> {
    let path = config_path(home);
    let mut root: serde_yaml::Value = match std::fs::read_to_string(&path) {
        Ok(t) => serde_yaml::from_str(&t).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "cannot parse {} ({e}) — fix or remove it; run `tebako-shim doctor`",
                    path.display()
                ),
            )
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        }
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    let mapping = root.as_mapping_mut().ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("{} must be a YAML mapping", path.display()),
        )
    })?;
    let key = serde_yaml::Value::String("registries".to_string());
    let entry = mapping
        .entry(key)
        .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
    let seq = entry.as_sequence_mut().ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("{}: `registries` must be a list", path.display()),
        )
    })?;
    if seq.iter().any(|v| v.as_str() == Some(reg_ref)) {
        return Ok(AddRegistryOutcome::AlreadyPresent);
    }
    seq.push(serde_yaml::Value::String(reg_ref.to_string()));
    let text = serde_yaml::to_string(&root).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot serialize {}: {e}", path.display()),
        )
    })?;
    std::fs::create_dir_all(home).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", home.display()),
        )
    })?;
    let tmp = home.join(format!("config.yaml.{}.tmp", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })?;
    Ok(AddRegistryOutcome::Added)
}

// ---------------------------------------------------------------------
// the registry-default chain link (spec 07 §2.1, last resort)
// ---------------------------------------------------------------------

/// The registry default version for `payload_name`, scanning the user's
/// registered registries in order (first match wins). Every registry form
/// of spec 04 §2 resolves through the dispatch-time cache
/// ([`crate::regcache`]); the registry model is tebako-resolve's.
pub fn registry_default(
    home: &Path,
    config: &UserConfig,
    payload_name: &str,
    ctx: &Ctx,
) -> Result<Option<(String, String)>, ShimError> {
    for reg_ref in &config.registries {
        let registry = crate::regcache::registry_for(home, reg_ref, ctx)?;
        if let Some(p) = registry.payload(payload_name) {
            if let Some(default) = &p.default {
                return Ok(Some((default.clone(), reg_ref.clone())));
            }
        }
    }
    Ok(None)
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
