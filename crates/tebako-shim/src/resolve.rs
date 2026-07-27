//! The dispatch-time resolution (spec 07 §2.0–§2.1):
//!
//! 0. argv0 is the selector: command name → the installed payload whose
//!    manifest provides an entrypoint of that name (multi-command suites:
//!    N shims → one payload, each entry resolved independently).
//! 1. Payload VERSION resolution, first match wins:
//!    `TEBAKO_<TOOL>_VERSION` env → nearest `.tebako-tools.yaml` walking
//!    up from cwd → user default (`~/.tebako/config.yaml` `defaults:`,
//!    written by `tebako use`) → registry `default:` (spec 04 §2).

use std::path::{Path, PathBuf};

use crate::config;
use crate::manifest::{self, Manifest, PayloadRecord};
use crate::{fail, Ctx, ShimError, EX_TEBAKO_MANIFEST, EX_TEBAKO_UNAVAILABLE};

/// Which link of the chain produced the version (reported by `which`).
#[derive(Debug, Clone)]
pub enum VersionSource {
    /// `TEBAKO_<TOOL>_VERSION`.
    Env(String),
    /// The nearest `.tebako-tools.yaml` walking up from cwd.
    ProjectFile(PathBuf),
    /// `~/.tebako/config.yaml` `defaults:`.
    UserDefault,
    /// A registry's `default:` (the ref it came from).
    RegistryDefault(String),
}

impl std::fmt::Display for VersionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionSource::Env(var) => write!(f, "env {var}"),
            VersionSource::ProjectFile(path) => write!(f, "project {}", path.display()),
            VersionSource::UserDefault => write!(f, "user default (~/.tebako/config.yaml)"),
            VersionSource::RegistryDefault(reg) => write!(f, "registry default ({reg})"),
        }
    }
}

/// The fully resolved dispatch target.
#[derive(Debug)]
pub struct Resolution {
    /// The command (argv0 / shim) name.
    pub tool: String,
    /// The payload that provides the command (== tool for simple apps;
    /// differs for multi-command suites).
    pub payload_name: String,
    pub version: String,
    pub source: VersionSource,
    pub record: PayloadRecord,
    pub manifest: Manifest,
}

/// The env var that pins a tool's version: `TEBAKO_<TOOL>_VERSION`
/// (uppercased, non-alphanumerics to `_`).
pub fn version_env_var(tool: &str) -> String {
    let mut var = String::from("TEBAKO_");
    for c in tool.chars() {
        if c.is_ascii_alphanumeric() {
            var.push(c.to_ascii_uppercase());
        } else {
            var.push('_');
        }
    }
    var.push_str("_VERSION");
    var
}

/// Installed payload versions: the `<version>.tfs` files under
/// `~/.tebako/payloads/<name>/` (a version with no image is not
/// installed, whatever else the record holds).
pub fn installed_versions(home: &Path, payload_name: &str) -> Result<Vec<String>, ShimError> {
    let dir = home.join("payloads").join(payload_name);
    let rd = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return fail(
                crate::EX_TEBAKO_IO,
                format!("cannot read {}: {e}", dir.display()),
            )
        }
    };
    let mut versions = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(version) = name.strip_suffix(".tfs") {
            versions.push(version.to_string());
        }
    }
    versions.sort();
    Ok(versions)
}

/// Command name → payload name. Fast path: a payload of the same name.
/// Suite path: scan every installed payload's manifest mirror for an
/// entrypoint of this name (spec 07 §2.0 multi-command suites).
fn providing_payload(home: &Path, tool: &str) -> Result<String, ShimError> {
    manifest::check_path_component("command name", tool)?;
    if home.join("payloads").join(tool).is_dir() {
        return Ok(tool.to_string());
    }
    let payloads_dir = home.join("payloads");
    let rd = match std::fs::read_dir(&payloads_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(no_provider(home, tool)),
        Err(e) => {
            return fail(
                crate::EX_TEBAKO_IO,
                format!("cannot read {}: {e}", payloads_dir.display()),
            )
        }
    };
    let mut providers = Vec::new();
    for entry in rd.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        for version in installed_versions(home, &name)? {
            let record = manifest::payload_record(home, &name, &version);
            if let Ok(m) = Manifest::load(&record.manifest_mirror) {
                if m.entrypoint(tool).is_some() {
                    providers.push(name.clone());
                    break;
                }
            }
        }
    }
    match providers.len() {
        0 => Err(no_provider(home, tool)),
        1 => Ok(providers.pop().unwrap_or_default()),
        _ => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "command \"{tool}\" is provided by more than one installed payload ({}) — remove one, or pin the payload with .tebako-tools.yaml",
                providers.join(", ")
            ),
        ),
    }
}

fn no_provider(home: &Path, tool: &str) -> ShimError {
    ShimError::new(
        EX_TEBAKO_MANIFEST,
        format!(
            "no installed payload provides the command \"{tool}\" (looked in {})\n  install the payload, or run `tebako-shim doctor` to diagnose the shim layer",
            home.join("payloads").display()
        ),
    )
}

/// Nearest `.tebako-tools.yaml` walking up from `start`; the file is a
/// flat YAML mapping of command name → version.
fn project_pin(start: &Path, tool: &str) -> Result<Option<(String, PathBuf)>, ShimError> {
    let mut dir: Option<&Path> = Some(start);
    while let Some(d) = dir {
        let candidate = d.join(".tebako-tools.yaml");
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate).map_err(|e| {
                ShimError::new(
                    crate::EX_TEBAKO_IO,
                    format!("cannot read {}: {e}", candidate.display()),
                )
            })?;
            let value: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_MANIFEST,
                    format!("cannot parse {} ({e})", candidate.display()),
                )
            })?;
            if let Some(version) = value
                .as_mapping()
                .and_then(|m| m.get(serde_yaml::Value::String(tool.to_string())))
                .and_then(|v| v.as_str())
            {
                return Ok(Some((version.to_string(), candidate)));
            }
            // A nearer file that does not pin this tool does NOT shadow a
            // farther one that does — keep walking.
        }
        dir = d.parent();
    }
    Ok(None)
}

/// Resolve the dispatch target for `tool` through the full chain.
pub fn resolve(tool: &str, ctx: &Ctx) -> Result<Resolution, ShimError> {
    let payload_name = providing_payload(&ctx.home, tool)?;
    manifest::check_path_component("payload name", &payload_name)?;
    let installed = installed_versions(&ctx.home, &payload_name)?;
    if installed.is_empty() {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload \"{payload_name}\" (providing \"{tool}\") has no installed versions under {}\n  run `tebako-shim doctor` to diagnose",
                ctx.home.join("payloads").join(&payload_name).display()
            ),
        );
    }

    let cfg = config::load_config(&ctx.home)?;

    // The chain, first match wins (spec 07 §2.1).
    let mut picked: Option<(String, VersionSource)> = None;

    // 1. TEBAKO_<TOOL>_VERSION
    let var = version_env_var(tool);
    if let Some(version) = ctx.env_get(&var).filter(|v| !v.is_empty()) {
        picked = Some((version.to_string(), VersionSource::Env(var.clone())));
    }
    // 2. nearest .tebako-tools.yaml walking up from cwd
    if picked.is_none() {
        if let Some((version, path)) = project_pin(&ctx.cwd, tool)? {
            picked = Some((version, VersionSource::ProjectFile(path)));
        }
    }
    // 3. user default (tebako use <tool>@<version>)
    if picked.is_none() {
        if let Some(version) = cfg.defaults.get(tool).filter(|v| !v.is_empty()) {
            picked = Some((version.clone(), VersionSource::UserDefault));
        }
    }
    // 4. registry default
    if picked.is_none() {
        if let Some((version, reg)) = config::registry_default(&ctx.home, &cfg, &payload_name, ctx)?
        {
            picked = Some((version, VersionSource::RegistryDefault(reg)));
        }
    }

    let (version, source) = picked.ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "no version resolved for \"{tool}\" — the chain found nothing:\n  TEBAKO_{} env: unset\n  .tebako-tools.yaml from {}: none pins it\n  ~/.tebako/config.yaml defaults: none\n  registry default: none\n  pin a version (`tebako use {tool}@<version>`) or install one; installed: {}",
                var.trim_start_matches("TEBAKO_"),
                ctx.cwd.display(),
                installed.join(", ")
            ),
        )
    })?;

    manifest::check_path_component("version", &version)?;

    if !installed.iter().any(|v| v == &version) {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "version {version} of \"{tool}\" (from {source}) is not installed — installed: {}\n  install it, or re-pin; run `tebako-shim doctor` to diagnose",
                installed.join(", ")
            ),
        );
    }

    let disabled = config::load_disabled(&ctx.home)?;
    if config::is_disabled(&disabled, tool, &version) {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "version {version} of \"{tool}\" is disabled — re-enable it with `tebako-shim enable {tool}@{version}`"
            ),
        );
    }

    let record = manifest::payload_record(&ctx.home, &payload_name, &version);
    if !record.installed() {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "the payload image for \"{tool}\" {version} is missing ({})\n  the installed record is incomplete; run `tebako-shim doctor`",
                record.image.display()
            ),
        );
    }
    let manifest = Manifest::load(&record.manifest_mirror)?;
    if manifest.entrypoint(tool).is_none() {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload \"{payload_name}\" {version} declares no entrypoint \"{tool}\" in {}\n  the shim link is stale; run `tebako-shim doctor`",
                record.manifest_mirror.display()
            ),
        );
    }

    Ok(Resolution {
        tool: tool.to_string(),
        payload_name,
        version,
        source,
        record,
        manifest,
    })
}
