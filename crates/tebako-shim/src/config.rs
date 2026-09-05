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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
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

/// Merge engine → runtime preferences into `~/.tebako/config.yaml`,
/// preserving every other key (the same structural-surgery discipline as
/// [`add_registry`]). A preference for an already-present engine is
/// REPLACED — the caller is the authority (publish's built-in verify
/// re-anchors the proof home to the publisher's picks, spec 16 §5).
/// This is the second authored-config write the toolchain performs;
/// the dispatcher itself still never writes this file.
pub fn set_runtime_prefs(
    home: &Path,
    prefs: &BTreeMap<String, RuntimePref>,
) -> Result<(), ShimError> {
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
    let key = serde_yaml::Value::String("runtimes".to_string());
    let entry = mapping
        .entry(key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let rt_map = entry.as_mapping_mut().ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("{}: `runtimes` must be a mapping", path.display()),
        )
    })?;
    for (engine, pref) in prefs {
        let value = serde_yaml::to_value(pref).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot serialize the runtime preference for {engine}: {e}"),
            )
        })?;
        rt_map.insert(serde_yaml::Value::String(engine.clone()), value);
    }
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
    Ok(())
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

/// Tool name → list of disabled selectors (strings on disk; parsed
/// through `tpkg::toolpin::DisableSelector`, the ONE grammar — spec 00
/// invariant 10): `all`, a bare version, `payload@all`, or
/// `payload@version` (spec 07 §0, the 2026-09-05 routing amendment).
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
    let disabled: Disabled = serde_yaml::from_str(&text).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot parse {} ({e})", path.display()),
        )
    })?;
    // Every selector validates at LOAD (invariant 9 — an unknown string
    // is a named error naming the file and entry, never silently
    // ignored).
    for (tool, selectors) in &disabled {
        for selector in selectors {
            tpkg::toolpin::DisableSelector::parse(selector).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_MANIFEST,
                    format!("{}: {tool}: {e}", path.display()),
                )
            })?;
        }
    }
    Ok(disabled)
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

/// Is the claim `payload` makes for `tool` at `version` gated?
/// Selectors parse through `tpkg::toolpin::DisableSelector` (validated
/// at load — a stored selector always parses).
pub fn is_disabled(disabled: &Disabled, tool: &str, payload: &str, version: &str) -> bool {
    disabled.get(tool).is_some_and(|selectors| {
        selectors.iter().any(|s| {
            tpkg::toolpin::DisableSelector::parse(s)
                .is_ok_and(|sel| sel.matches(payload, version))
        })
    })
}

/// The provider scan's skip test: is the payload's WHOLE claim for
/// `tool` gated? Only `all` and `<payload>@all` gate a whole claim —
/// a version selector leaves the claim routable at its other versions.
pub fn claim_disabled(disabled: &Disabled, tool: &str, payload: &str) -> bool {
    disabled.get(tool).is_some_and(|selectors| {
        selectors.iter().any(|s| {
            matches!(
                tpkg::toolpin::DisableSelector::parse(s),
                Ok(tpkg::toolpin::DisableSelector::All)
            ) || matches!(
                tpkg::toolpin::DisableSelector::parse(s),
                Ok(tpkg::toolpin::DisableSelector::PayloadAll(ref p)) if p == payload
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-shim-config-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn pref(version: &str, tebako: &str) -> RuntimePref {
        RuntimePref {
            version: version.to_string(),
            tebako: tebako.to_string(),
        }
    }

    #[test]
    fn set_runtime_prefs_writes_a_fresh_config() {
        let home = fresh_home("fresh");
        let mut prefs = BTreeMap::new();
        prefs.insert("java".to_string(), pref("21.0.12", "2.1.0"));
        set_runtime_prefs(&home, &prefs).unwrap();
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.runtimes.get("java"), Some(&pref("21.0.12", "2.1.0")));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn set_runtime_prefs_preserves_other_keys_and_replaces_the_engine() {
        let home = fresh_home("merge");
        add_registry(&home, "tfs:github:acme/app").unwrap();
        let mut first = BTreeMap::new();
        first.insert("java".to_string(), pref("21.0.12", "2.1.0"));
        set_runtime_prefs(&home, &first).unwrap();
        let mut second = BTreeMap::new();
        second.insert("java".to_string(), pref("21.0.13", "2.1.0"));
        second.insert("ruby".to_string(), pref("3.3.12", "0.16.18"));
        set_runtime_prefs(&home, &second).unwrap();
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.runtimes.get("java"), Some(&pref("21.0.13", "2.1.0")));
        assert_eq!(cfg.runtimes.get("ruby"), Some(&pref("3.3.12", "0.16.18")));
        assert_eq!(cfg.registries, vec!["tfs:github:acme/app".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn set_runtime_prefs_rejects_a_non_mapping_runtimes_key() {
        let home = fresh_home("badshape");
        std::fs::write(config_path(&home), "runtimes: [nope]\n").unwrap();
        let mut prefs = BTreeMap::new();
        prefs.insert("java".to_string(), pref("21.0.12", "2.1.0"));
        let err = set_runtime_prefs(&home, &prefs).unwrap_err();
        assert!(
            err.message.contains("`runtimes` must be a mapping"),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
