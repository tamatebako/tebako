//! Management commands (spec 07 §3): `tebako-shim list | enable |
//! disable | which | doctor | install-shell | uninstall-shell`.
//! Hand-rolled argv — no clap.

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::config;
use crate::dispatch::{self, ExecPlan};
use crate::manifest;
use crate::regcache;
use crate::resolve::{self, Resolution};
use crate::runtime::RuntimeResolution;
use crate::shell;
#[cfg(not(windows))]
use crate::shell::Shell;
use crate::{fail, Action, Ctx, ShimError, EX_TEBAKO_IO, EX_USAGE};

#[cfg(not(windows))]
const USAGE: &str = "tebako-shim — the tebako dispatcher and version manager (spec 07)

invoked as ~/.tebako/shims/<tool> it dispatches; invoked as tebako-shim it manages:

  tebako-shim list [--json]            installed payloads, versions, defaults, shim links
  tebako-shim use <tool> <pin>         write the user default ([payload@]version) to config.yaml
  tebako-shim use --clear <tool>       remove the tool's user default
  tebako-shim use --runtime <engine>@<langver>[:<tebako>]
                                       write the engine's runtime preference
  tebako-shim enable <tool>[@<ver>] [--of <payload>]
                                       re-enable a disabled tool, version, or payload claim
  tebako-shim disable <tool>[@<ver>] [--of <payload>]
                                       refuse dispatch of a tool, version, or payload claim
  tebako-shim which <tool>             show the resolved provider, version, runtime, mounts, argv
  tebako-shim doctor                   diagnose the shim layer (missing/corrupt records, routing)
  tebako-shim install-shell [--shell bash|zsh|fish|csh]
                                       prepend ~/.tebako/shims to PATH in the shell startup file
  tebako-shim uninstall-shell [--shell bash|zsh|fish|csh]
                                       remove exactly the managed block";

#[cfg(windows)]
const USAGE: &str = "tebako-shim — the tebako dispatcher and version manager (spec 07)

invoked as <TEBAKO_HOME>\\shims\\<tool>.exe it dispatches; invoked as tebako-shim it manages:

  tebako-shim list [--json]            installed payloads, versions, defaults, shim links
  tebako-shim use <tool> <pin>         write the user default ([payload@]version) to config.yaml
  tebako-shim use --clear <tool>       remove the tool's user default
  tebako-shim use --runtime <engine>@<langver>[:<tebako>]
                                       write the engine's runtime preference
  tebako-shim enable <tool>[@<ver>] [--of <payload>]
                                       re-enable a disabled tool, version, or payload claim
  tebako-shim disable <tool>[@<ver>] [--of <payload>]
                                       refuse dispatch of a tool, version, or payload claim
  tebako-shim which <tool>             show the resolved provider, version, runtime, mounts, argv
  tebako-shim doctor                   diagnose the shim layer (missing/corrupt records, routing)
  tebako-shim install-shell            prepend the shim dir to the user PATH (registry)
  tebako-shim uninstall-shell          remove exactly our PATH entry";

pub fn run_command(args: &[String], ctx: &Ctx) -> Result<Action, ShimError> {
    let Some(cmd) = args.first() else {
        return Ok(Action::Print {
            text: USAGE.to_string(),
            code: 0,
        });
    };
    let rest = &args[1..];
    match cmd.as_str() {
        "list" => cmd_list(rest, ctx),
        "use" => cmd_use(rest, ctx),
        "enable" => cmd_enable(rest, ctx, true),
        "disable" => cmd_enable(rest, ctx, false),
        "which" => cmd_which(rest, ctx),
        "doctor" => cmd_doctor(ctx),
        "install-shell" => cmd_shell(rest, ctx, true),
        "uninstall-shell" => cmd_shell(rest, ctx, false),
        "help" | "--help" | "-h" => Ok(Action::Print {
            text: USAGE.to_string(),
            code: 0,
        }),
        other => fail(EX_USAGE, format!("unknown command \"{other}\"\n{USAGE}")),
    }
}

// ---------------------------------------------------------------------
// use (spec 07 §3 — the authored-config write verb, §0's write
// relaxation; tmp + rename in config.rs's edit_config)
// ---------------------------------------------------------------------

fn cmd_use(args: &[String], ctx: &Ctx) -> Result<Action, ShimError> {
    match args.first().map(String::as_str) {
        Some("--clear") => {
            let Some(tool) = args.get(1) else {
                return fail(EX_USAGE, "usage: tebako-shim use --clear <tool>");
            };
            if args.len() > 2 {
                return fail(EX_USAGE, "usage: tebako-shim use --clear <tool>");
            }
            manifest::check_path_component("command name", tool)?;
            let changed = config::set_default(&ctx.home, tool, None)?;
            let text = if changed {
                format!("cleared the user default for {tool}\n")
            } else {
                format!("{tool} had no user default\n")
            };
            Ok(Action::Print { text, code: 0 })
        }
        Some("--runtime") => {
            let Some(spec) = args.get(1) else {
                return fail(
                    EX_USAGE,
                    "usage: tebako-shim use --runtime <engine>@<langver>[:<tebako>]",
                );
            };
            if args.len() > 2 {
                return fail(
                    EX_USAGE,
                    "usage: tebako-shim use --runtime <engine>@<langver>[:<tebako>]",
                );
            }
            let (engine, rest) = spec.split_once('@').ok_or_else(|| {
                ShimError::new(
                    EX_USAGE,
                    format!(
                        "invalid runtime preference \"{spec}\" — the grammar is <engine>@<langver>[:<tebako>]"
                    ),
                )
            })?;
            if engine.is_empty() || rest.is_empty() {
                return fail(
                    EX_USAGE,
                    format!(
                        "invalid runtime preference \"{spec}\" — the grammar is <engine>@<langver>[:<tebako>]"
                    ),
                );
            }
            manifest::check_path_component("engine", engine)?;
            let (version, tebako) = match rest.split_once(':') {
                Some((v, t)) if !v.is_empty() && !t.is_empty() => (v, Some(t)),
                Some(_) => {
                    return fail(
                        EX_USAGE,
                        format!(
                            "invalid runtime preference \"{spec}\" — the grammar is <engine>@<langver>[:<tebako>]"
                        ),
                    )
                }
                None => (rest, None),
            };
            config::set_runtime_pref(&ctx.home, engine, version, tebako)?;
            let text = match tebako {
                Some(t) => format!(
                    "runtime preference {engine}: version {version}, tebako {t} (~/.tebako/config.yaml)\n"
                ),
                None => format!(
                    "runtime preference {engine}: version {version} (~/.tebako/config.yaml; the tebako line follows the product default)\n"
                ),
            };
            Ok(Action::Print { text, code: 0 })
        }
        Some(tool) => {
            let Some(pin) = args.get(1) else {
                return fail(EX_USAGE, "usage: tebako-shim use <tool> <pin>");
            };
            if args.len() > 2 {
                return fail(EX_USAGE, "usage: tebako-shim use <tool> <pin>");
            }
            manifest::check_path_component("command name", tool)?;
            // Validate against the ONE grammar (tpkg::toolpin) BEFORE
            // writing — a bad pin is the named grammar error and the file
            // is untouched (spec 07 §0/§7).
            tpkg::toolpin::ToolPin::parse(pin).map_err(|e| {
                ShimError::new(
                    crate::EX_TEBAKO_MANIFEST,
                    format!("use {tool}: {e}"),
                )
            })?;
            config::set_default(&ctx.home, tool, Some(pin))?;
            Ok(Action::Print {
                text: format!("default {tool}: {pin} (~/.tebako/config.yaml)\n"),
                code: 0,
            })
        }
        None => fail(
            EX_USAGE,
            "usage: tebako-shim use <tool> <pin> | use --clear <tool> | use --runtime <engine>@<langver>[:<tebako>]",
        ),
    }
}

// ---------------------------------------------------------------------
// list
// ---------------------------------------------------------------------

/// One command row of `list`: the declaring payload, the shim link, the
/// effective resolution (provider payload + kind, version + source) or
/// the named failure, and the tool's disabled selectors.
struct CommandRow {
    tool: String,
    payload: String,
    shim: PathBuf,
    resolved: Option<(String, String, String, resolve::ProviderKind)>,
    error: Option<String>,
    selectors: Vec<String>,
}

/// One payload row of `list`: name, (version, disabled) marks, and the
/// newest version's commands.
struct PayloadRow {
    name: String,
    versions: Vec<(String, bool)>,
    tools: Vec<String>,
}

fn cmd_list(args: &[String], ctx: &Ctx) -> Result<Action, ShimError> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            other => {
                return fail(
                    EX_USAGE,
                    format!("unexpected argument \"{other}\" — usage: tebako-shim list [--json]"),
                )
            }
        }
    }
    let payloads_dir = ctx.home.join("payloads");
    let mut names: Vec<String> = Vec::new();
    match std::fs::read_dir(&payloads_dir) {
        Ok(rd) => {
            for entry in rd.flatten() {
                if entry.path().is_dir() {
                    names.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return fail(
                EX_TEBAKO_IO,
                format!("cannot read {}: {e}", payloads_dir.display()),
            )
        }
    }
    names.sort();

    let disabled = config::load_disabled(&ctx.home)?;
    let mut payloads: Vec<PayloadRow> = Vec::new();
    let mut commands: Vec<CommandRow> = Vec::new();
    for name in &names {
        let versions = resolve::installed_versions(&ctx.home, name)?;
        // every entrypoint of the newest installed version is a command
        let newest = versions.last().cloned();
        let mut tools: Vec<String> = Vec::new();
        if let Some(v) = &newest {
            let record = manifest::payload_record(&ctx.home, name, v);
            if let Ok(m) = manifest::Manifest::load(&record.manifest_mirror) {
                tools = m.entrypoints().iter().map(|e| e.name.clone()).collect();
            }
        }
        let version_marks: Vec<(String, bool)> = versions
            .iter()
            .map(|v| {
                let is_dis = tools
                    .iter()
                    .any(|t| config::is_disabled(&disabled, t, name, v));
                (v.clone(), is_dis)
            })
            .collect();
        for tool in &tools {
            let shim = ctx.home.join("shims").join(tool);
            let row = match resolve::resolve(tool, ctx) {
                Ok(res) => CommandRow {
                    tool: tool.clone(),
                    payload: name.clone(),
                    shim,
                    resolved: Some((
                        res.version,
                        res.source.to_string(),
                        res.payload_name,
                        res.provider,
                    )),
                    error: None,
                    selectors: disabled.get(tool).cloned().unwrap_or_default(),
                },
                Err(e) => CommandRow {
                    tool: tool.clone(),
                    payload: name.clone(),
                    shim,
                    resolved: None,
                    error: Some(first_line(&e.message).to_string()),
                    selectors: disabled.get(tool).cloned().unwrap_or_default(),
                },
            };
            commands.push(row);
        }
        payloads.push(PayloadRow {
            name: name.clone(),
            versions: version_marks,
            tools,
        });
    }

    if json {
        return cmd_list_json(ctx, &payloads, &commands);
    }

    let mut out = String::new();
    if payloads.is_empty() {
        let _ = writeln!(
            out,
            "no installed payloads under {}",
            payloads_dir.display()
        );
    }
    for row in &payloads {
        let name = &row.name;
        let _ = writeln!(out, "{name}");
        let _ = writeln!(
            out,
            "  versions: {}",
            row.versions
                .iter()
                .map(|(v, is_dis)| {
                    if *is_dis {
                        format!("{v} [disabled]")
                    } else {
                        v.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        );
        for crow in commands.iter().filter(|r| r.payload == row.name) {
            let _ = writeln!(
                out,
                "  command {}: shim {}",
                crow.tool,
                if crow.shim.exists() {
                    crow.shim.display().to_string()
                } else {
                    "(not linked)".to_string()
                }
            );
            match &crow.resolved {
                Some((version, source, provider, kind)) => {
                    let _ = writeln!(
                        out,
                        "    resolved: {version} (from {source}) [provider {provider} ({kind})]"
                    );
                }
                None => {
                    let _ = writeln!(
                        out,
                        "    resolved: (unresolvable: {})",
                        crow.error.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }
    }
    Ok(Action::Print { text: out, code: 0 })
}

/// `tebako-shim list --json` (spec 07 §3): one `"info_schema": 1`
/// document, mirroring `tebako cache list --json`'s convention
/// (tebako-cli's cache_list_json).
fn cmd_list_json(
    ctx: &Ctx,
    payloads: &[PayloadRow],
    commands: &[CommandRow],
) -> Result<Action, ShimError> {
    use tebako_json::Value as J;

    let s = |v: &str| J::String(v.to_string());
    let payload_docs = payloads
        .iter()
        .map(|row| {
            J::Object(vec![
                ("name".to_string(), s(&row.name)),
                (
                    "versions".to_string(),
                    J::Array(
                        row.versions
                            .iter()
                            .map(|(v, is_dis)| {
                                J::Object(vec![
                                    ("version".to_string(), s(v)),
                                    ("disabled".to_string(), J::Bool(*is_dis)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                (
                    "commands".to_string(),
                    J::Array(row.tools.iter().map(|t| s(t)).collect()),
                ),
            ])
        })
        .collect();
    let command_docs = commands
        .iter()
        .map(|row| {
            let mut obj = vec![
                ("name".to_string(), s(&row.tool)),
                ("payload".to_string(), s(&row.payload)),
                (
                    "shim".to_string(),
                    if row.shim.exists() {
                        s(&row.shim.display().to_string())
                    } else {
                        J::Null
                    },
                ),
            ];
            match &row.resolved {
                Some((version, source, provider, kind)) => {
                    obj.push(("provider".to_string(), s(provider)));
                    obj.push(("provider_kind".to_string(), s(&kind.to_string())));
                    obj.push(("version".to_string(), s(version)));
                    obj.push(("source".to_string(), s(source)));
                }
                None => {
                    obj.push((
                        "error".to_string(),
                        s(row.error.as_deref().unwrap_or("unknown")),
                    ));
                }
            }
            obj.push((
                "disabled".to_string(),
                J::Array(row.selectors.iter().map(|sel| s(sel)).collect()),
            ));
            J::Object(obj)
        })
        .collect();
    let doc = J::Object(vec![
        ("info_schema".to_string(), J::Number("1".to_string())),
        (
            "store".to_string(),
            J::String(ctx.home.display().to_string()),
        ),
        ("payloads".to_string(), J::Array(payload_docs)),
        ("commands".to_string(), J::Array(command_docs)),
    ]);
    Ok(Action::Print {
        text: format!("{}\n", tebako_json::to_string(&doc)),
        code: 0,
    })
}

fn first_line(message: &str) -> &str {
    message.lines().next().unwrap_or(message)
}

/// "<N>m/h/d old" for doctor's registry freshness lines.
fn human_age(secs: u64) -> String {
    if secs < 3600 {
        format!("{}m old", secs / 60)
    } else if secs < 86_400 {
        format!("{}h old", secs / 3600)
    } else {
        format!("{}d old", secs / 86_400)
    }
}

// ---------------------------------------------------------------------
// enable / disable
// ---------------------------------------------------------------------

/// `<tool>[@<version>]`; without a version the whole tool is gated.
fn parse_target(arg: &str) -> Result<(String, Option<String>), ShimError> {
    let (tool, version) = match arg.split_once('@') {
        Some((t, v)) => (t.to_string(), Some(v.to_string())),
        None => (arg.to_string(), None),
    };
    manifest::check_path_component("command name", &tool)?;
    if let Some(v) = &version {
        manifest::check_path_component("version", v)?;
    }
    if tool.is_empty() {
        return fail(
            EX_USAGE,
            "usage: tebako-shim enable|disable <tool>[@<version>]",
        );
    }
    Ok((tool, version))
}

fn cmd_enable(args: &[String], ctx: &Ctx, enable: bool) -> Result<Action, ShimError> {
    let verb = if enable { "enable" } else { "disable" };
    let mut target: Option<&str> = None;
    let mut of: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--of" {
            i += 1;
            match args.get(i) {
                Some(p) => of = Some(p.clone()),
                None => {
                    return fail(
                        EX_USAGE,
                        format!("--of expects a payload name — usage: tebako-shim {verb} <tool>[@<version>] [--of <payload>]"),
                    )
                }
            }
        } else if let Some(p) = arg.strip_prefix("--of=") {
            of = Some(p.to_string());
        } else if target.is_none() && !arg.starts_with('-') {
            target = Some(arg);
        } else {
            return fail(
                EX_USAGE,
                format!(
                    "unexpected argument \"{arg}\" — usage: tebako-shim {verb} <tool>[@<version>] [--of <payload>]"
                ),
            );
        }
        i += 1;
    }
    let Some(target) = target else {
        return fail(
            EX_USAGE,
            format!("usage: tebako-shim {verb} <tool>[@<version>] [--of <payload>]"),
        );
    };
    let (tool, version) = parse_target(target)?;
    if let Some(p) = &of {
        if p.is_empty() {
            return fail(
                EX_USAGE,
                format!("--of expects a payload name — usage: tebako-shim {verb} <tool>[@<version>] [--of <payload>]"),
            );
        }
        manifest::check_path_component("payload name", p)?;
    }
    // The selector grammar (tpkg::toolpin::DisableSelector owns it —
    // these writes produce exactly its four spellings, spec 07 §0).
    let selector = match (&version, &of) {
        (None, None) => "all".to_string(),
        (Some(v), None) => v.clone(),
        (None, Some(p)) => format!("{p}@all"),
        (Some(v), Some(p)) => format!("{p}@{v}"),
    };
    let target_desc = match (&version, &of) {
        (None, None) => tool.clone(),
        (Some(v), None) => format!("{tool}@{v}"),
        (None, Some(p)) => format!("{tool} --of {p}"),
        (Some(v), Some(p)) => format!("{tool}@{v} --of {p}"),
    };
    let mut disabled = config::load_disabled(&ctx.home)?;
    let selectors = disabled.entry(tool.clone()).or_default();
    let changed = if enable {
        // `enable <tool>` clears every selector; `enable <tool>@v
        // [--of p]` drops exactly the computed selector.
        let before = selectors.len();
        if version.is_none() && of.is_none() {
            selectors.clear();
        } else {
            selectors.retain(|s| s != &selector);
        }
        selectors.len() != before
    } else if selectors.iter().any(|s| s == &selector) {
        false
    } else {
        selectors.push(selector.clone());
        true
    };
    if selectors.is_empty() {
        disabled.remove(&tool);
    }
    if changed {
        config::save_disabled(&ctx.home, &disabled)?;
    }
    let suffix = if of.is_some() {
        format!(" ({selector})")
    } else {
        String::new()
    };
    let text = match (enable, changed) {
        (true, true) => format!("enabled {target_desc}{suffix}\n"),
        (true, false) => format!("{target_desc} was not disabled\n"),
        (false, true) => format!("disabled {target_desc}{suffix}\n"),
        (false, false) => format!("{target_desc} was already disabled\n"),
    };
    // Enabling a DECLARED-but-unlinked command (spec 03 §2.2's
    // `active: false` — install registered nothing for it) materializes
    // the link now: the dispatcher links ITSELF (the argv0 model admits
    // real links/copies only, spec 07 §1). The declaration check is the
    // suite scan — an undeclared command earns resolve's named error,
    // never a dangling link.
    let text = if enable {
        let link = shims_dir(&ctx.home).join(shim_file_name(&tool));
        if link.exists() {
            text
        } else {
            resolve::resolve(&tool, ctx)?;
            let binary = std::env::current_exe().map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_IO,
                    format!("cannot locate the dispatcher binary: {e}"),
                )
            })?;
            let (_shims, _notes) = link_shims(&ctx.home, &binary, std::slice::from_ref(&tool))?;
            format!("{text}linked {}", link.display())
        }
    } else {
        text
    };
    Ok(Action::Print { text, code: 0 })
}

// ---------------------------------------------------------------------
// which
// ---------------------------------------------------------------------

fn cmd_which(args: &[String], ctx: &Ctx) -> Result<Action, ShimError> {
    let Some(tool) = args.first() else {
        return fail(EX_USAGE, "usage: tebako-shim which <tool>");
    };
    let res: Resolution = resolve::resolve(tool, ctx)?;
    let plan: ExecPlan = dispatch::plan(&res, &[], ctx, false, Vec::new())?;
    let entry = res
        .manifest
        .entrypoint(tool)
        .expect("resolve checked the entrypoint");
    let mut out = String::new();
    let _ = writeln!(out, "tool: {tool}");
    let _ = writeln!(out, "payload: {} {}", res.payload_name, res.version);
    let _ = writeln!(out, "  provider: {} ({})", res.payload_name, res.provider);
    match res.provider {
        resolve::ProviderKind::Pinned => {
            // The pin won both dimensions — show the qualified value.
            let _ = writeln!(
                out,
                "  version source: {} (pin {}@{})",
                res.source, res.payload_name, res.version
            );
        }
        _ => {
            let _ = writeln!(out, "  version source: {}", res.source);
        }
    }
    let _ = writeln!(out, "  image: {}", res.record.image.display());
    let _ = writeln!(out, "entrypoint: {}", entry.path);
    match &plan.runtime {
        RuntimeResolution::Zero => {
            let _ = writeln!(
                out,
                "runtime: none (native entrypoint — zero-runtime dispatch)"
            );
        }
        RuntimeResolution::Ready(rt) => {
            let req = entry
                .runtime_requirement
                .as_ref()
                .expect("Ready implies a requirement");
            let _ = writeln!(
                out,
                "runtime: {} \"{}\" → {} {} (cached)",
                req.engine, req.constraint, rt.engine, rt.lang_version
            );
            let _ = writeln!(out, "  exe: {}", rt.exe.display());
            if let Some(image) = &rt.image {
                let _ = writeln!(out, "  image: {}", image.display());
            }
        }
    }
    let _ = writeln!(out, "mounts:");
    for m in &plan.mounts {
        let _ = writeln!(out, "  {}", m.triple());
    }
    let _ = writeln!(out, "exec argv:");
    for a in &plan.argv {
        let _ = writeln!(out, "  {a}");
    }
    Ok(Action::Print { text: out, code: 0 })
}

// ---------------------------------------------------------------------
// doctor
// ---------------------------------------------------------------------

fn cmd_doctor(ctx: &Ctx) -> Result<Action, ShimError> {
    let mut problems: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    // PATH carries the shim dir? (std::env::split_paths — the separator
    // is ':' on unix and ';' on Windows; a hand-rolled ':' split would
    // never match on Windows.)
    let shims_dir = ctx.home.join("shims");
    let on_path = ctx
        .env_get("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir == shims_dir))
        .unwrap_or(false);
    if !on_path {
        problems.push(format!(
            "{} is not on PATH — run `tebako-shim install-shell`",
            shims_dir.display()
        ));
    }

    // shim links → payload records (spec 07 §7: missing/corrupt target).
    if let Ok(rd) = std::fs::read_dir(&shims_dir) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue; // shim-managed state files
            }
            let name = crate::command_from_shim_name(&name).to_string();
            match resolve::resolve(&name, ctx) {
                Ok(_) => notes.push(format!("shim {name}: ok")),
                Err(e) => problems.push(format!("shim {name}: {}", first_line(&e.message))),
            }
        }
    } else {
        problems.push(format!(
            "{} does not exist — no shims are linked",
            shims_dir.display()
        ));
    }

    // payload records: image + trust marker + manifest mirror; verify the
    // marker against the image (install-time trust anchor, spec 05 §4).
    let payloads_dir = ctx.home.join("payloads");
    if let Ok(rd) = std::fs::read_dir(&payloads_dir) {
        for entry in rd.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            match resolve::installed_versions(&ctx.home, &name) {
                Ok(versions) if versions.is_empty() => {
                    problems.push(format!("payload {name}: no installed versions"));
                }
                Err(e) => problems.push(format!("payload {name}: {}", first_line(&e.message))),
                Ok(versions) => {
                    for v in versions {
                        check_payload_record(ctx, &name, &v, &mut problems);
                    }
                }
            }
        }
    }

    // authored config + registries: parse, local existence, and the
    // dispatch-cache freshness of remote refs (spec 04 §2 + roadmap 33).
    match config::load_config(&ctx.home) {
        Ok(cfg) => {
            for reg in &cfg.registries {
                match regcache::freshness(&ctx.home, reg) {
                    regcache::RegistryFreshness::Local => {
                        let path = reg.strip_prefix("file://").unwrap_or(reg);
                        if !std::path::Path::new(path).is_file() {
                            problems.push(format!("registry {reg}: file not found"));
                        }
                    }
                    regcache::RegistryFreshness::Fresh(age) => notes.push(format!(
                        "registry {reg}: dispatch cache fresh ({})",
                        human_age(age)
                    )),
                    regcache::RegistryFreshness::Stale(age) => notes.push(format!(
                        "registry {reg}: dispatch cache is stale ({}) — the next dispatch refreshes it, or run `tebako update-registries`",
                        human_age(age)
                    )),
                    regcache::RegistryFreshness::Missing => problems.push(format!(
                        "registry {reg}: not in the dispatch cache — run `tebako update-registries` (online dispatch fetches on demand; TEBAKO_OFFLINE dispatch would fail)"
                    )),
                    regcache::RegistryFreshness::BadRef(_) => problems.push(format!(
                        "registry {reg}: does not parse as a spec 04 §2 registry reference"
                    )),
                }
            }
        }
        Err(e) => problems.push(format!("config.yaml: {}", first_line(&e.message))),
    }

    // cached runtimes: every entry dir holds its executable + markers.
    let runtimes_dir = ctx.home.join("runtimes");
    if let Ok(rd) = std::fs::read_dir(&runtimes_dir) {
        for entry in rd.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let entry_name = entry.file_name().to_string_lossy().into_owned();
            let has_exe = std::fs::read_dir(entry.path())
                .map(|files| {
                    files.flatten().any(|f| {
                        f.file_name()
                            .to_string_lossy()
                            .starts_with("tebako-runtime-")
                            && !f.file_name().to_string_lossy().ends_with(".tfs")
                    })
                })
                .unwrap_or(false);
            if !has_exe {
                problems.push(format!(
                    "runtime entry {entry_name}: no tebako-runtime executable — remove {}",
                    entry.path().display()
                ));
            } else if !entry.path().join("sha256").is_file() {
                problems.push(format!(
                    "runtime entry {entry_name}: missing sha256 trust marker"
                ));
            }
        }
    }

    // Routing health (spec 07 §3, the 2026-09-05 amendment): collisions,
    // dangling pins, disabled-but-pinned conflicts.
    doctor_routing(ctx, &mut problems);

    let mut out = String::new();
    for note in &notes {
        let _ = writeln!(out, "ok: {note}");
    }
    if problems.is_empty() {
        let _ = writeln!(out, "tebako-shim doctor: no problems found");
        Ok(Action::Print { text: out, code: 0 })
    } else {
        for p in &problems {
            let _ = writeln!(out, "problem: {p}");
        }
        let _ = writeln!(out, "tebako-shim doctor: {} problem(s)", problems.len());
        Ok(Action::Print { text: out, code: 1 })
    }
}

/// The routing health reports (spec 07 §3, the 2026-09-05 amendment):
/// collisions (a command with more than one ENABLED claim), dangling pins
/// (project-file/config pins naming an uninstalled payload or version —
/// the env link is per-invocation and not inspectable), and
/// disabled-but-pinned conflicts. Diagnose-only, like every doctor
/// finding.
fn doctor_routing(ctx: &Ctx, problems: &mut Vec<String>) {
    use std::collections::BTreeMap;

    // command → claiming payloads (declared entrypoints + expose edges)
    let mut claims: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let payloads_dir = ctx.home.join("payloads");
    if let Ok(rd) = std::fs::read_dir(&payloads_dir) {
        for entry in rd.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let versions = resolve::installed_versions(&ctx.home, &name).unwrap_or_default();
            for v in versions {
                let record = manifest::payload_record(&ctx.home, &name, &v);
                if let Ok(m) = manifest::Manifest::load(&record.manifest_mirror) {
                    let mut claim = |cmd: &str| {
                        let entry = claims.entry(cmd.to_string()).or_default();
                        if !entry.contains(&name) {
                            entry.push(name.clone());
                        }
                    };
                    for e in m.entrypoints() {
                        claim(&e.name);
                    }
                    for r in m.requires() {
                        let expose = match r {
                            tpkg::Requirement::Runtime { expose, .. } => expose,
                            tpkg::Requirement::Executable { expose, .. } => expose,
                            _ => continue,
                        };
                        for e in expose {
                            claim(e);
                        }
                    }
                }
            }
        }
    }

    let disabled = match config::load_disabled(&ctx.home) {
        Ok(d) => d,
        Err(e) => {
            problems.push(format!(".disabled.yaml: {}", first_line(&e.message)));
            return;
        }
    };

    // collisions: more than one ENABLED claim (disabled claims route out)
    for (cmd, ps) in &claims {
        let enabled: Vec<&String> = ps
            .iter()
            .filter(|p| !config::claim_disabled(&disabled, cmd, p))
            .collect();
        if enabled.len() > 1 {
            problems.push(format!(
                "command {cmd}: claimed by more than one enabled payload ({}) — pin `{cmd}: <payload>@<version>` in .tebako-tools.yaml, or disable one claim (`tebako-shim disable {cmd} --of <payload>`)",
                enabled
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // pins: config defaults + project files (nearest-wins per tool)
    let mut pins: Vec<(String, String, String)> = Vec::new(); // (tool, value, origin)
    if let Ok(cfg) = config::load_config(&ctx.home) {
        for (tool, value) in &cfg.defaults {
            pins.push((
                tool.clone(),
                value.clone(),
                "~/.tebako/config.yaml defaults".to_string(),
            ));
        }
    }
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut dir: Option<&std::path::Path> = Some(&ctx.cwd);
    while let Some(d) = dir {
        let candidate = d.join(".tebako-tools.yaml");
        if candidate.is_file() {
            match std::fs::read_to_string(&candidate)
                .map_err(|e| e.to_string())
                .and_then(|text| {
                    serde_yaml::from_str::<serde_yaml::Value>(&text).map_err(|e| e.to_string())
                }) {
                Ok(value) => {
                    if let Some(m) = value.as_mapping() {
                        for (k, v) in m {
                            if let (Some(tool), Some(pin)) = (k.as_str(), v.as_str()) {
                                if seen.insert(tool.to_string()) {
                                    pins.push((
                                        tool.to_string(),
                                        pin.to_string(),
                                        format!("project {}", candidate.display()),
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(e) => problems.push(format!("{}: {e}", candidate.display())),
            }
        }
        dir = d.parent();
    }

    for (tool, value, origin) in &pins {
        let pin = match tpkg::toolpin::ToolPin::parse(value) {
            Ok(pin) => pin,
            Err(e) => {
                problems.push(format!("pin \"{value}\" for {tool} ({origin}): {e}"));
                continue;
            }
        };
        match &pin.payload {
            Some(payload) => {
                if !ctx.home.join("payloads").join(payload).is_dir() {
                    problems.push(format!(
                        "dangling pin: \"{pin}\" for {tool} ({origin}) names payload \"{payload}\", which is not installed"
                    ));
                    continue;
                }
                let installed = resolve::installed_versions(&ctx.home, payload).unwrap_or_default();
                if !installed.iter().any(|v| v == &pin.version) {
                    problems.push(format!(
                        "dangling pin: \"{pin}\" for {tool} ({origin}) names version {} of \"{payload}\" — installed: {}",
                        pin.version,
                        installed.join(", ")
                    ));
                    continue;
                }
                if config::is_disabled(&disabled, tool, payload, &pin.version) {
                    problems.push(format!(
                        "disabled but pinned: \"{pin}\" for {tool} ({origin}) — dispatch refuses it until `tebako-shim enable {tool}@{version} --of {payload}`",
                        version = pin.version
                    ));
                }
            }
            None => {
                let Some(ps) = claims.get(tool) else {
                    problems.push(format!(
                        "dangling pin: no installed payload provides or exposes \"{tool}\" (pinned \"{pin}\" at {origin})"
                    ));
                    continue;
                };
                let with_version: Vec<&String> = ps
                    .iter()
                    .filter(|p| {
                        resolve::installed_versions(&ctx.home, p)
                            .map(|vs| vs.iter().any(|v| v == &pin.version))
                            .unwrap_or(false)
                    })
                    .collect();
                if with_version.is_empty() {
                    problems.push(format!(
                        "dangling pin: version {} of \"{tool}\" (pinned at {origin}) is not installed in any claiming payload ({})",
                        pin.version,
                        ps.join(", ")
                    ));
                    continue;
                }
                if with_version
                    .iter()
                    .all(|p| config::is_disabled(&disabled, tool, p, &pin.version))
                {
                    problems.push(format!(
                        "disabled but pinned: version {} of \"{tool}\" ({origin}) is disabled for every claiming payload ({})",
                        pin.version,
                        with_version
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }
    }
}

fn check_payload_record(ctx: &Ctx, name: &str, version: &str, problems: &mut Vec<String>) {
    let record = manifest::payload_record(&ctx.home, name, version);
    let tag = format!("payload {name} {version}");
    if !record.sha_marker.is_file() {
        problems.push(format!(
            "{tag}: missing trust anchor {} — the image was never verified at install",
            record.sha_marker.display()
        ));
    } else {
        // "<sha>  <file>\n" — re-verify (doctor is the diagnostic path).
        let marker = std::fs::read_to_string(&record.sha_marker).unwrap_or_default();
        let expected = marker.split_whitespace().next().unwrap_or("");
        match sha256_file_hex(&record.image) {
            Ok(actual) if expected.len() == 64 && actual == expected => {}
            Ok(actual) => problems.push(format!(
                "{tag}: sha256 mismatch — expected {expected}, actual {actual}; the image is corrupt, reinstall it"
            )),
            Err(e) => problems.push(format!("{tag}: cannot hash image: {e}")),
        }
    }
    match manifest::Manifest::load(&record.manifest_mirror) {
        Ok(m) => {
            if m.name() != name || m.version() != version {
                problems.push(format!(
                    "{tag}: manifest mirror declares {} {} — the record is inconsistent",
                    m.name(),
                    m.version()
                ));
            }
        }
        Err(e) => problems.push(format!("{tag}: {}", first_line(&e.message))),
    }
}

fn sha256_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    use sha2::Digest as _;
    use std::io::Read as _;
    let mut f = std::fs::File::open(path)?;
    let mut h = sha2::Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let digest = h.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

// ---------------------------------------------------------------------
// shim links (the installer's registration half — library API for the
// `tebako` CLI; spec 07 §1: ONE tebako-shim binary, linked per command)
// ---------------------------------------------------------------------

/// The directory every shim lives in (the ONE PATH entry, spec 07 §3).
pub fn shims_dir(home: &std::path::Path) -> PathBuf {
    home.join("shims")
}

/// Link `shim_binary` as `~/.tebako/shims/<command>` for every command:
/// symlink on unix; on Windows an NTFS hardlink FIRST (needs no admin
/// and no Developer Mode — symlink creation needs
/// SeCreateSymbolicLinkPrivilege), falling back to a byte copy when the
/// hardlink fails (hardlinks are same-volume only, and some filesystems
/// decline them). Both Windows shapes place the dispatcher's own bytes
/// at `<command>.exe`, so argv0 dispatch is byte-identical either way —
/// a `.cmd` wrapper was rejected: argv would re-parse through cmd.exe
/// quoting, and CreateProcess callers could not spawn it in place.
/// Existing links are replaced (install/reinstall is idempotent).
/// Returns the linked paths in command order plus the notes naming each
/// copy fallback (spec 07: a fallback is documented and named, never
/// silent).
pub fn link_shims(
    home: &std::path::Path,
    shim_binary: &std::path::Path,
    commands: &[String],
) -> Result<(Vec<PathBuf>, Vec<String>), ShimError> {
    if !shim_binary.is_file() {
        return fail(
            EX_TEBAKO_IO,
            format!(
                "the dispatcher binary {} does not exist — shims would point at nothing",
                shim_binary.display()
            ),
        );
    }
    let dir = shims_dir(home);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", dir.display()),
        )
    })?;
    let mut linked = Vec::new();
    let mut notes = Vec::new();
    for command in commands {
        manifest::check_path_component("command name", command)?;
        let link = dir.join(shim_file_name(command));
        if link.symlink_metadata().is_ok() {
            std::fs::remove_file(&link).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_IO,
                    format!("cannot replace {}: {e}", link.display()),
                )
            })?;
        }
        if let Some(note) = link_one(shim_binary, &link)? {
            notes.push(note);
        }
        linked.push(link);
    }
    Ok((linked, notes))
}

/// The shim's on-disk filename for a command: `<command>` on unix
/// (executability is permission bits), `<command>.exe` on Windows —
/// CreateProcess/PATH resolution goes through PATHEXT, so the copied
/// dispatcher needs the suffix. The dispatcher strips it from argv[0]
/// (lib.rs `run`), so registration and lookup stay suffix-free.
pub fn shim_file_name(command: &str) -> String {
    #[cfg(windows)]
    return format!("{command}.exe");
    #[cfg(not(windows))]
    return command.to_string();
}

#[cfg(unix)]
fn link_one(
    shim_binary: &std::path::Path,
    link: &std::path::Path,
) -> Result<Option<String>, ShimError> {
    std::os::unix::fs::symlink(shim_binary, link).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!(
                "cannot link {} -> {}: {e}",
                link.display(),
                shim_binary.display()
            ),
        )
    })?;
    Ok(None)
}

/// Windows: NTFS hardlink first — no admin, no Developer Mode (symlink
/// creation needs SeCreateSymbolicLinkPrivilege) and the shims share the
/// dispatcher's bytes. A hardlink is same-volume only, so a store on a
/// different volume than the binary (or a filesystem without hardlink
/// support) falls back to a byte copy, NAMED in the returned note (the
/// installer prints it). Copy, not a `.cmd` wrapper: the copied file IS
/// the dispatcher (argv0 dispatch byte-identical), while a wrapper
/// would re-parse argv through cmd.exe quoting and could not be spawned
/// in place by CreateProcess callers.
#[cfg(windows)]
fn link_one(
    shim_binary: &std::path::Path,
    link: &std::path::Path,
) -> Result<Option<String>, ShimError> {
    match std::fs::hard_link(shim_binary, link) {
        Ok(()) => Ok(None),
        Err(hard_err) => {
            std::fs::copy(shim_binary, link).map(|_| ()).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_IO,
                    format!(
                        "cannot link {} -> {} (hardlink: {hard_err}; copy: {e})",
                        link.display(),
                        shim_binary.display()
                    ),
                )
            })?;
            Ok(Some(format!(
                "{}: NTFS hardlink unavailable ({hard_err}) — copied the dispatcher instead (byte-identical bytes; keep the tebako home and the binary on one volume to share them)",
                link.display()
            )))
        }
    }
}

/// Remove `~/.tebako/shims/<command>` for every command (idempotent —
/// missing links are skipped). Returns the paths actually removed.
pub fn unlink_shims(
    home: &std::path::Path,
    commands: &[String],
) -> Result<Vec<PathBuf>, ShimError> {
    let dir = shims_dir(home);
    let mut removed = Vec::new();
    for command in commands {
        manifest::check_path_component("command name", command)?;
        let link = dir.join(shim_file_name(command));
        match std::fs::remove_file(&link) {
            Ok(()) => removed.push(link),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return fail(
                    EX_TEBAKO_IO,
                    format!("cannot remove {}: {e}", link.display()),
                )
            }
        }
    }
    Ok(removed)
}

// ---------------------------------------------------------------------
// install-shell / uninstall-shell
// ---------------------------------------------------------------------

#[cfg(not(windows))]
fn parse_shell_flag(args: &[String], ctx: &Ctx) -> Result<Shell, ShimError> {
    let mut i = 0;
    let mut shell: Option<String> = None;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--shell" {
            i += 1;
            shell = args.get(i).cloned();
            if shell.is_none() {
                return fail(EX_USAGE, "--shell expects bash|zsh|fish|csh");
            }
        } else if let Some(v) = arg.strip_prefix("--shell=") {
            shell = Some(v.to_string());
        } else {
            return fail(
                EX_USAGE,
                format!("unexpected argument \"{arg}\" — usage: tebako-shim install-shell [--shell bash|zsh|fish|csh]"),
            );
        }
        i += 1;
    }
    match shell {
        Some(name) => Shell::parse(&name),
        None => Shell::detect(ctx.env_get("SHELL")),
    }
}

#[cfg(not(windows))]
fn user_home(ctx: &Ctx) -> Result<PathBuf, ShimError> {
    ctx.env_get("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            ShimError::new(
                EX_TEBAKO_IO,
                "cannot determine the user home (HOME is unset)".to_string(),
            )
        })
}

#[cfg(not(windows))]
fn cmd_shell(args: &[String], ctx: &Ctx, install: bool) -> Result<Action, ShimError> {
    let sh = parse_shell_flag(args, ctx)?;
    let file = shell::startup_file(sh, &user_home(ctx)?);
    let text = if install {
        match shell::install(sh, &file)? {
            shell::Change::Installed => format!(
                "installed the tebako shim block in {} ({})\nrestart the shell, or re-source the file",
                file.display(),
                sh.name()
            ),
            shell::Change::AlreadyPresent => format!(
                "the tebako shim block is already present in {} — nothing to do",
                file.display()
            ),
            _ => unreachable!("install only yields Installed/AlreadyPresent"),
        }
    } else {
        match shell::uninstall(&file)? {
            shell::Change::Removed => {
                format!("removed the tebako shim block from {}", file.display())
            }
            shell::Change::NotPresent => {
                format!("no tebako shim block in {} — nothing to do", file.display())
            }
            _ => unreachable!("uninstall only yields Removed/NotPresent"),
        }
    };
    Ok(Action::Print {
        text: format!("{text}\n"),
        code: 0,
    })
}

/// Windows: no rc files — the shim dir goes onto the user PATH in the
/// registry (shell_windows.rs). `--shell` is a unix-only option and says
/// so, never a silent ignore.
#[cfg(windows)]
fn cmd_shell(args: &[String], ctx: &Ctx, install: bool) -> Result<Action, ShimError> {
    if !args.is_empty() {
        return fail(
            EX_USAGE,
            format!(
                "unexpected argument \"{}\" — on Windows install-shell edits the user PATH in the registry (HKCU\\Environment) and takes no --shell",
                args.join(" ")
            ),
        );
    }
    let dir = shims_dir(&ctx.home);
    let text = if install {
        match crate::shell_windows::install(&dir)? {
            shell::Change::Installed => format!(
                "added {} to the user PATH (HKCU\\Environment)\nopen a NEW terminal — running consoles keep the old PATH",
                dir.display()
            ),
            shell::Change::AlreadyPresent => format!(
                "{} is already on the user PATH — nothing to do",
                dir.display()
            ),
            _ => unreachable!("install only yields Installed/AlreadyPresent"),
        }
    } else {
        match crate::shell_windows::uninstall(&dir)? {
            shell::Change::Removed => format!("removed {} from the user PATH", dir.display()),
            shell::Change::NotPresent => {
                format!("{} was not on the user PATH — nothing to do", dir.display())
            }
            _ => unreachable!("uninstall only yields Removed/NotPresent"),
        }
    };
    Ok(Action::Print {
        text: format!("{text}\n"),
        code: 0,
    })
}
