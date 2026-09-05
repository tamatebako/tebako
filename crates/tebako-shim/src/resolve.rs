//! The dispatch-time resolution (spec 07 §2.0–§2.1):
//!
//! 0. argv0 is the selector: command name → the installed payload whose
//!    manifest provides an entrypoint of that name (multi-command suites:
//!    N shims → one payload, each entry resolved independently).
//!
//! 0.5. PROVIDER ROUTING (the 2026-09-05 amendment): the chain VALUE
//!    grammar is `[payload@]version` (`tpkg::toolpin` is the SSOT parser).
//!    A payload-qualified value makes the named payload THE provider; the
//!    provider scan skips claims disabled at `<payload>@all` /
//!    `<payload>@version`; a registry never routes commands.
//!
//! 1. Payload VERSION resolution, first match wins:
//!    `TEBAKO_<TOOL>_VERSION` env → nearest `.tebako-tools.yaml` walking
//!    up from cwd → user default (`~/.tebako/config.yaml` `defaults:`,
//!    written by `tebako-shim use`) → registry `default:` (spec 04 §2).

use std::path::{Path, PathBuf};

use tpkg::toolpin::ToolPin;

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

/// How the provider payload was chosen (spec 07 §2 step 0.5; `which` /
/// `list` render it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// The payload declares the command as its own entrypoint.
    Own,
    /// The payload EXPOSES the command through a spawn edge (spec 30/32).
    Exposed,
    /// A payload-qualified chain pin named this payload THE provider.
    Pinned,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderKind::Own => f.write_str("own"),
            ProviderKind::Exposed => f.write_str("exposed"),
            ProviderKind::Pinned => f.write_str("pinned"),
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
    /// How the provider was chosen (spec 07 §2 step 0.5).
    pub provider: ProviderKind,
    pub record: PayloadRecord,
    pub manifest: Manifest,
    /// spec 30 §3 + spec 32 §3: when the command resolved through the
    /// payload's `expose` list rather than its own entrypoints, the spawn
    /// edge it came through (a runtime edge or an executable edge).
    /// plan() composes the exposed dispatch for these — the consumer
    /// payload itself is never the child's program.
    pub exposed: Option<tpkg::Requirement>,
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
/// installed, whatever else the record holds). The path grammar is
/// `tpkg::payload_store`'s (the SSOT).
pub fn installed_versions(home: &Path, payload_name: &str) -> Result<Vec<String>, ShimError> {
    tpkg::payload_store::installed_versions(home, payload_name)
        .map_err(|e| ShimError::new(crate::EX_TEBAKO_IO, e))
}

/// Who provides `tool`: a payload declaring it as an entrypoint (the own
/// claim always wins), else — spec 30 §3 — a payload EXPOSING it through
/// a spawned-runtime edge's `expose` list.
enum Provider {
    Own(String),
    Exposed(String),
}

/// The corrected collision hint (spec 07 §7, 2026-09-05 amendment): the
/// routing verbs, not a dangling "remove one".
fn routing_hint(tool: &str) -> String {
    format!(
        "pin `{tool}: <payload>@<version>` in .tebako-tools.yaml, or disable one claim (`tebako-shim disable {tool} --of <payload>`)"
    )
}

/// Command name → payload name. Fast path: a payload of the same name.
/// Suite path: scan every installed payload's manifest mirror for an
/// entrypoint of this name (spec 07 §2.0 multi-command suites). Only when
/// NO payload declares the entrypoint does the expose surface answer
/// (spec 30 §3). The scan skips claims disabled at `all` / `<payload>@all`
/// (spec 07 §2 step 0.5); the collision error fires among ENABLED claims
/// only — a sole-but-disabled claim falls through to resolve_named's
/// per-version refusal (the historical shape), several all-disabled
/// claims answer the no-provider error.
fn providing_payload(
    home: &Path,
    tool: &str,
    disabled: &config::Disabled,
) -> Result<Provider, ShimError> {
    manifest::check_path_component("command name", tool)?;
    if home.join("payloads").join(tool).is_dir() && !config::claim_disabled(disabled, tool, tool) {
        return Ok(Provider::Own(tool.to_string()));
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
    let mut exposers = Vec::new();
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
                if !exposers.contains(&name) && exposes(&m, tool) {
                    exposers.push(name.clone());
                    break;
                }
            }
        }
    }
    let enabled: Vec<&String> = providers
        .iter()
        .filter(|p| !config::claim_disabled(disabled, tool, p))
        .collect();
    match (enabled.len(), providers.len()) {
        (1, _) => return Ok(Provider::Own(enabled[0].clone())),
        (0, 1) => return Ok(Provider::Own(providers[0].clone())),
        (0, _) => {}
        _ => {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "command \"{tool}\" is provided by more than one installed payload ({}) — {}",
                    enabled
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    routing_hint(tool)
                ),
            )
        }
    }
    let enabled_ex: Vec<&String> = exposers
        .iter()
        .filter(|p| !config::claim_disabled(disabled, tool, p))
        .collect();
    match (enabled_ex.len(), exposers.len()) {
        (1, _) => Ok(Provider::Exposed(enabled_ex[0].clone())),
        (0, 1) => Ok(Provider::Exposed(exposers[0].clone())),
        (0, _) => Err(no_provider(home, tool)),
        _ => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "command \"{tool}\" is exposed by more than one installed payload ({}) — {}",
                enabled_ex
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                routing_hint(tool)
            ),
        ),
    }
}

/// spec 30 §3 + spec 32 §3: does the manifest's DEPENDS expose `tool`
/// through a spawn edge (a runtime edge's or an executable edge's
/// `expose` list)?
fn exposes(m: &Manifest, tool: &str) -> bool {
    m.requires().iter().any(|r| {
        let expose = match r {
            tpkg::Requirement::Runtime { expose, .. } => expose,
            tpkg::Requirement::Executable { expose, .. } => expose,
            _ => return false,
        };
        expose.iter().any(|e| e == tool)
    })
}

fn no_provider(home: &Path, tool: &str) -> ShimError {
    ShimError::new(
        EX_TEBAKO_MANIFEST,
        format!(
            "no installed payload provides or exposes the command \"{tool}\" (looked in {})\n  install the payload, or run `tebako-shim doctor` to diagnose the shim layer",
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

/// The chain's first three links (env → project pin → config default),
/// parsed as ONE `[payload@]version` value (the registry default is
/// payload-keyed and stays in [`resolve_named`]). An unparseable value is
/// a NAMED grammar error naming the link and value (spec 07 §0/§7 —
/// spec 23 §14's env-parse rule extended to every link), never a silent
/// skip.
pub fn chain_pick(
    tool: &str,
    ctx: &Ctx,
    cfg: &config::UserConfig,
) -> Result<Option<(ToolPin, VersionSource)>, ShimError> {
    let var = version_env_var(tool);
    let raw: Option<(String, VersionSource)> =
        if let Some(v) = ctx.env_get(&var).filter(|v| !v.is_empty()) {
            Some((v.to_string(), VersionSource::Env(var.clone())))
        } else if let Some((v, path)) = project_pin(&ctx.cwd, tool)? {
            Some((v, VersionSource::ProjectFile(path)))
        } else {
            cfg.defaults
                .get(tool)
                .filter(|v| !v.is_empty())
                .map(|v| (v.clone(), VersionSource::UserDefault))
        };
    match raw {
        Some((value, source)) => {
            let pin = ToolPin::parse(&value)
                .map_err(|e| ShimError::new(EX_TEBAKO_MANIFEST, format!("{source}: {e}")))?;
            Ok(Some((pin, source)))
        }
        None => Ok(None),
    }
}

/// The expose edge of `m` that names `tool` (spec 30 §3 + spec 32 §3).
fn expose_edge(m: &Manifest, tool: &str) -> Option<tpkg::Requirement> {
    m.requires()
        .iter()
        .find(|r| {
            let expose = match r {
                tpkg::Requirement::Runtime { expose, .. } => expose,
                tpkg::Requirement::Executable { expose, .. } => expose,
                _ => return false,
            };
            expose.iter().any(|e| e == tool)
        })
        .cloned()
}

/// Resolve the dispatch target for `tool` through the full chain.
pub fn resolve(tool: &str, ctx: &Ctx) -> Result<Resolution, ShimError> {
    let cfg = config::load_config(&ctx.home)?;
    // The routing amendment reads the chain FIRST (spec 07 §2 step 0.5):
    // a payload-qualified value won BOTH dimensions — the provider is the
    // named payload, the version is the pin's, and the remaining links
    // are not consulted for this dispatch.
    if let Some((pin, source)) = chain_pick(tool, ctx, &cfg)? {
        if let Some(payload_name) = pin.payload.clone() {
            return resolve_pinned(tool, &payload_name, &pin, source, ctx);
        }
        // Unqualified: the value is a bare version — pre-seed today's
        // flow (identical to the chain's first match, already parsed).
        return resolve_scanned(tool, ctx, Some((pin.version, source)));
    }
    resolve_scanned(tool, ctx, None)
}

/// The qualified-pin route: the named payload IS the provider — it must
/// be installed (resolve_named's existing named errors) and DECLARE or
/// EXPOSE the command, else NotAProvider.
fn resolve_pinned(
    tool: &str,
    payload_name: &str,
    pin: &ToolPin,
    source: VersionSource,
    ctx: &Ctx,
) -> Result<Resolution, ShimError> {
    manifest::check_path_component("payload name", payload_name)?;
    let mut res = resolve_named(
        tool,
        payload_name,
        ctx,
        Some((pin.version.clone(), source.clone())),
    )?;
    res.provider = ProviderKind::Pinned;
    if res.manifest.entrypoint(tool).is_some() {
        return Ok(res);
    }
    match expose_edge(&res.manifest, tool) {
        Some(edge) => {
            res.exposed = Some(edge);
            Ok(res)
        }
        None => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "pin \"{pin}\" (from {source}): payload \"{payload_name}\" {version} neither declares nor exposes the command \"{tool}\" (NotAProvider)\n  the qualified form pins a PROVIDER — name one of the payloads claiming \"{tool}\", or pin the bare version",
                version = res.version,
            ),
        ),
    }
}

/// Today's flow: the provider scan (fast path → suite scan → expose
/// scan), with the amendment's disabled-claim skips.
fn resolve_scanned(
    tool: &str,
    ctx: &Ctx,
    preseed: Option<(String, VersionSource)>,
) -> Result<Resolution, ShimError> {
    let disabled = config::load_disabled(&ctx.home)?;
    match providing_payload(&ctx.home, tool, &disabled)? {
        Provider::Own(payload_name) => {
            let mut res = resolve_named(tool, &payload_name, ctx, preseed)?;
            res.provider = ProviderKind::Own;
            if res.manifest.entrypoint(tool).is_none() {
                return fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "payload \"{payload_name}\" {version} declares no entrypoint \"{tool}\" in {mirror}\n  the shim link is stale; run `tebako-shim doctor`",
                        version = res.version,
                        mirror = res.record.manifest_mirror.display()
                    ),
                );
            }
            Ok(res)
        }
        Provider::Exposed(payload_name) => {
            let mut res = resolve_named(tool, &payload_name, ctx, preseed)?;
            res.provider = ProviderKind::Exposed;
            // The edge comes from the PICKED version's manifest — the
            // exposing scan established the payload, not the version.
            match expose_edge(&res.manifest, tool) {
                Some(edge) => {
                    res.exposed = Some(edge);
                    Ok(res)
                }
                None => fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "payload \"{payload_name}\" {version} does not expose \"{tool}\" (another installed version does)\n  pin the exposing version with .tebako-tools.yaml or {}",
                        version_env_var(tool),
                        payload_name = res.payload_name,
                        version = res.version,
                    ),
                ),
            }
        }
    }
}

/// The payload-addressed resolution (spec 26 §2's check engine): the name
/// IS the payload (never the suite scan — a check target names its slice
/// directly), the version chain / record / mirror load are dispatch's
/// own, and there is NO entrypoint gate (a data slice declares none —
/// structural checks resolve exactly this far).
pub fn resolve_payload(payload_name: &str, ctx: &Ctx) -> Result<Resolution, ShimError> {
    manifest::check_path_component("payload name", payload_name)?;
    if !ctx.home.join("payloads").join(payload_name).is_dir() {
        return Err(no_provider(&ctx.home, payload_name));
    }
    let mut res = resolve_named(payload_name, payload_name, ctx, None)?;
    res.provider = ProviderKind::Own;
    Ok(res)
}

/// The shared resolution tail: version chain → disabled gate → record →
/// mirror load. `tool` keys the version chain (env var, project pin,
/// defaults); `payload_name` keys the store record. `preseed` carries an
/// already-resolved (version, source) — the routing amendment's pin won
/// the chain, so no link (registry default included) is consulted again.
fn resolve_named(
    tool: &str,
    payload_name: &str,
    ctx: &Ctx,
    preseed: Option<(String, VersionSource)>,
) -> Result<Resolution, ShimError> {
    manifest::check_path_component("payload name", payload_name)?;
    let installed = installed_versions(&ctx.home, payload_name)?;
    if installed.is_empty() {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload \"{payload_name}\" (providing \"{tool}\") has no installed versions under {}\n  run `tebako-shim doctor` to diagnose",
                ctx.home.join("payloads").join(payload_name).display()
            ),
        );
    }

    let cfg = config::load_config(&ctx.home)?;

    // The chain, first match wins (spec 07 §2.1). A preseed (the
    // amendment's pin) won it already; otherwise the four links run.
    let mut picked: Option<(String, VersionSource)> = preseed;

    let var = version_env_var(tool);
    if picked.is_none() {
        // 1. TEBAKO_<TOOL>_VERSION
        if let Some(version) = ctx.env_get(&var).filter(|v| !v.is_empty()) {
            picked = Some((version.to_string(), VersionSource::Env(var.clone())));
        }
        // 2. nearest .tebako-tools.yaml walking up from cwd
        if picked.is_none() {
            if let Some((version, path)) = project_pin(&ctx.cwd, tool)? {
                picked = Some((version, VersionSource::ProjectFile(path)));
            }
        }
        // 3. user default (tebako-shim use <tool> <pin>)
        if picked.is_none() {
            if let Some(version) = cfg.defaults.get(tool).filter(|v| !v.is_empty()) {
                picked = Some((version.clone(), VersionSource::UserDefault));
            }
        }
        // 4. registry default
        if picked.is_none() {
            if let Some((version, reg)) =
                config::registry_default(&ctx.home, &cfg, payload_name, ctx)?
            {
                picked = Some((version, VersionSource::RegistryDefault(reg)));
            }
        }
    }

    let (version, source) = picked.ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "no version resolved for \"{tool}\" — the chain found nothing:\n  TEBAKO_{} env: unset\n  .tebako-tools.yaml from {}: none pins it\n  ~/.tebako/config.yaml defaults: none\n  registry default: none\n  pin a version (`tebako-shim use {tool} <version>`) or install one; installed: {}",
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
    if config::is_disabled(&disabled, tool, payload_name, &version) {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "version {version} of \"{tool}\" is disabled — re-enable it with `tebako-shim enable {tool}@{version}`"
            ),
        );
    }

    let record = manifest::payload_record(&ctx.home, payload_name, &version);
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

    Ok(Resolution {
        tool: tool.to_string(),
        payload_name: payload_name.to_string(),
        version,
        source,
        provider: ProviderKind::Own, // the caller stamps the real kind
        record,
        manifest,
        exposed: None,
    })
}
