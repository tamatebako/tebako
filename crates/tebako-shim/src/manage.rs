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

  tebako-shim list                     installed payloads, versions, defaults, shim links
  tebako-shim enable <tool>[@<ver>]    re-enable a disabled tool or version
  tebako-shim disable <tool>[@<ver>]   refuse dispatch of a tool or version
  tebako-shim which <tool>             show the resolved version, runtime, mounts, argv
  tebako-shim doctor                   diagnose the shim layer (missing/corrupt records)
  tebako-shim install-shell [--shell bash|zsh|fish|csh]
                                       prepend ~/.tebako/shims to PATH in the shell startup file
  tebako-shim uninstall-shell [--shell bash|zsh|fish|csh]
                                       remove exactly the managed block";

#[cfg(windows)]
const USAGE: &str = "tebako-shim — the tebako dispatcher and version manager (spec 07)

invoked as <TEBAKO_HOME>\\shims\\<tool>.exe it dispatches; invoked as tebako-shim it manages:

  tebako-shim list                     installed payloads, versions, defaults, shim links
  tebako-shim enable <tool>[@<ver>]    re-enable a disabled tool or version
  tebako-shim disable <tool>[@<ver>]   refuse dispatch of a tool or version
  tebako-shim which <tool>             show the resolved version, runtime, mounts, argv
  tebako-shim doctor                   diagnose the shim layer (missing/corrupt records)
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
        "list" => cmd_list(ctx),
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
// list
// ---------------------------------------------------------------------

fn cmd_list(ctx: &Ctx) -> Result<Action, ShimError> {
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
    let mut out = String::new();
    if names.is_empty() {
        let _ = writeln!(
            out,
            "no installed payloads under {}",
            payloads_dir.display()
        );
    }
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
        let _ = writeln!(out, "{name}");
        let _ = writeln!(
            out,
            "  versions: {}",
            versions
                .iter()
                .map(|v| {
                    let is_dis = tools.iter().any(|t| config::is_disabled(&disabled, t, v));
                    if is_dis {
                        format!("{v} [disabled]")
                    } else {
                        v.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        );
        for tool in &tools {
            let shim = ctx.home.join("shims").join(tool);
            let _ = writeln!(
                out,
                "  command {tool}: shim {}",
                if shim.exists() {
                    shim.display().to_string()
                } else {
                    "(not linked)".to_string()
                }
            );
            match resolve::resolve(tool, ctx) {
                Ok(res) => {
                    let _ = writeln!(out, "    resolved: {} (from {})", res.version, res.source);
                }
                Err(e) => {
                    let _ = writeln!(
                        out,
                        "    resolved: (unresolvable: {})",
                        first_line(&e.message)
                    );
                }
            }
        }
    }
    Ok(Action::Print { text: out, code: 0 })
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
    let Some(target) = args.first() else {
        return fail(
            EX_USAGE,
            format!("usage: tebako-shim {verb} <tool>[@<version>]"),
        );
    };
    let (tool, version) = parse_target(target)?;
    let selector = version.clone().unwrap_or_else(|| "all".to_string());
    let mut disabled = config::load_disabled(&ctx.home)?;
    let selectors = disabled.entry(tool.clone()).or_default();
    let changed = if enable {
        // `enable <tool>` clears every selector; `enable <tool>@v` drops v.
        let before = selectors.len();
        if version.is_none() {
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
    let target_desc = match &version {
        Some(v) => format!("{tool}@{v}"),
        None => tool.clone(),
    };
    let text = match (enable, changed) {
        (true, true) => format!("enabled {target_desc}\n"),
        (true, false) => format!("{target_desc} was not disabled\n"),
        (false, true) => format!("disabled {target_desc}\n"),
        (false, false) => format!("{target_desc} was already disabled\n"),
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
    let _ = writeln!(out, "  version source: {}", res.source);
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
fn shim_file_name(command: &str) -> String {
    #[cfg(windows)]
    return format!("{command}.exe");
    #[cfg(not(windows))]
    return command.to_string();
}

#[cfg(unix)]
fn link_one(shim_binary: &std::path::Path, link: &std::path::Path) -> Result<Option<String>, ShimError> {
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
fn link_one(shim_binary: &std::path::Path, link: &std::path::Path) -> Result<Option<String>, ShimError> {
    match std::fs::hard_link(shim_binary, link) {
        Ok(()) => Ok(None),
        Err(hard_err) => {
            std::fs::copy(shim_binary, link)
                .map(|_| ())
                .map_err(|e| {
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
