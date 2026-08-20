//! tebako — the packager CLI: press + cache subcommands.

use std::path::PathBuf;
use std::process::ExitCode;

use tebako_cli::error::TebakoError;
use tebako_cli::options::{resolve_prefix, PressMode, PressOptions};
use tebako_cli::runner::verbose_mode;
use tebako_cli::{cache_list, cache_list_json, cache_prune, press, VERSION_BANNER};

const USAGE: &str = "Usage:
  tebako press -r <root> -e <entry> [-o <output>] [-p <prefix>] [-c <cwd>]
               [-R <ruby>] [-m lean|fat] [-l error|warn|debug|trace]
               [--image <path>:<mount>]... [--bootstrap <path>]
               [--tebako-version <v>] [--prefer-local] [--jail <spec>]
               [--format dwarfs|limnifs]
  tebako press --suite <suite.yaml> [-o <output>] [-p <prefix>] [-R <ruby>]
               one package, N commands (spec 03 §6: per-entry slots + type-2 manifest)
  tebako run <pkg> [--jail <spec>] [--mount <host:mount:ro|rw>]... [--no-host]
               [--] [<args>...]
  tebako trace run <pkg> [--capture <path>] [--out <path>] [--] [<args>...]
                                       run under TEBAKO_JAIL=record with the trace bus
                                       armed; synthesize a suggested manifest (spec 25 §4)
  tebako trace cover --inside <tfs.json> --outside <retrace.json> --prefix <path>
               [--pid N] [--window SECS] [--exclude-probes] [--json] [--layer libc|kernel]
                                       the escapes report (spec 25 §6, phase T3): outside
                                       touches under the prefix the inside stream never saw
                                       (exit 0 clean / 1 escapes / 2 error)
  tebako check <name | image.tfs | package | tebako.yaml>
               [--check <c>] [--list] [--record] [--keep-scratch]
               [--runtime <exe> --runtime-image <env.tfs>]
                                       the payload's in-image acceptance checks (spec 26 §2)
  tebako cache list [--json]
  tebako cache prune [--all] [--older-than Nd]
  tebako add-registry <ref>            register a tpkg-registry.yaml (spec 04 §2)
  tebako list-registries               list the registered registries
  tebako update-registries             refresh the dispatch-time registry cache
  tebako install <ref | name[@ver]>    install a payload + register its shims
  tebako uninstall <name>              remove a payload's shims and cache entry
  tebako info [topic] [--remote] [--json]
                                       the store/system surface (system|runtimes|payloads|shims|registries|store)
  tebako inspect <artifact> [flags]    payload/package introspection (spec 15);
                                       --contract prints the spec-18 contract card
                                       (era, contract versions, mount_root, abi, trust + verdict)
  tebako publish --name <app> [--version <v>] --release tfs:github:<owner>/<repo>[:<tag>]
               (--payload <path> | --payload <triplet>=<path>)...
               [--standalone <triplet>=<path>]... [--sign[=<keyid>]]
               [--upload-mirror <dir>] [--tap <org/homebrew-tap> [--tap-dir <dir>]]
               [--registry-out <path>] [--skip-verify]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliExit::Error(e)) => {
            println!("Tebako script failed: {} [{}]", e.message, e.code);
            ExitCode::from(e.code.clamp(1, 255) as u8)
        }
        Err(CliExit::Usage(msg)) => {
            eprintln!("{msg}");
            eprintln!("{USAGE}");
            ExitCode::from(1)
        }
    }
}

enum CliExit {
    Error(TebakoError),
    Usage(String),
}

impl From<TebakoError> for CliExit {
    fn from(e: TebakoError) -> Self {
        CliExit::Error(e)
    }
}

fn run(args: &[String]) -> Result<(), CliExit> {
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("{VERSION_BANNER}");
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") || args.is_empty() {
        println!("{VERSION_BANNER}");
        println!("{USAGE}");
        return Ok(());
    }

    let subcommand = args[0].as_str();
    let rest = &args[1..];
    // `cache list --json` is a machine contract: the banner moves to
    // stderr so stdout is the document alone. Every other call keeps the
    // banner on stdout (unchanged). `trace cover` is a machine contract
    // too: its stdout byte-compares against the retrace golden fixtures
    // (spec 25 §6.3's parity clause).
    let machine_stdout = (subcommand == "cache"
        && rest.first().map(|a| a.as_str()) == Some("list")
        && rest.iter().any(|a| a == "--json"))
        || (subcommand == "trace" && rest.first().map(|a| a.as_str()) == Some("cover"));
    if machine_stdout {
        eprintln!("{VERSION_BANNER}");
    } else {
        println!("{VERSION_BANNER}");
    }
    // spec 18 C13: the store layout check runs once per process, ahead of
    // every store-touching verb (S41: a newer stamp is the upgrade
    // refusal; S42: a pre-versioning store is stamped and the named
    // migration announced on stderr — tebako-resolve::store owns both).
    if let Ok(home) = tebako_home() {
        match tebako_resolve::store::check_once(&home) {
            Ok(tebako_resolve::store::LayoutCheck::Migrated) => {
                eprintln!(
                    "tebako: note: {}",
                    tebako_resolve::store::migration_message(&home)
                );
            }
            Ok(_) => {}
            Err(e) => {
                return Err(CliExit::Error(tebako_cli::error::TebakoError::new(
                    e.to_string(),
                    74,
                )));
            }
        }
    }
    match subcommand {
        "press" => {
            let opts = parse_press(rest)?;
            press(&opts)?;
            Ok(())
        }
        "run" => run_package(rest),
        "trace" => run_trace(rest),
        "check" => run_check(rest),
        "cache" => run_cache(rest),
        "add-registry" => run_add_registry(rest),
        "list-registries" => run_list_registries(rest),
        "update-registries" => run_update_registries(rest),
        "install" => run_install(rest),
        "uninstall" => run_uninstall(rest),
        "info" => run_info(rest),
        "inspect" => run_inspect(rest),
        "publish" => run_publish(rest),
        "clean" | "setup" | "hash" => Err(CliExit::Usage(format!(
            "'tebako {subcommand}' is a later tebako-rs milestone"
        ))),
        other => Err(CliExit::Usage(format!("unknown command '{other}'"))),
    }
}

/// The tebako home the registry/install surface works against
/// ($TEBAKO_HOME > platform default — tebako-shim's rule).
fn tebako_home() -> Result<PathBuf, CliExit> {
    let env: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    tebako_shim::tebako_home(&env).map_err(|e| {
        CliExit::Error(tebako_cli::error::TebakoError::new(
            e.message,
            i32::from(e.code),
        ))
    })
}

/// `tebako info [topic] [--remote] [--json]` — the store/system surface.
fn run_info(args: &[String]) -> Result<(), CliExit> {
    let mut topic: Option<&str> = None;
    let mut remote = false;
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--remote" => remote = true,
            "--json" => json = true,
            other if topic.is_none() && !other.starts_with('-') => topic = Some(other),
            other => {
                return Err(CliExit::Usage(format!(
                    "unknown info option '{other}' (usage: tebako info [topic] [--remote] [--json])"
                )))
            }
        }
    }
    let (out, code) = tebako_cli::info::run(&tebako_home()?, topic, remote, json)?;
    print!("{out}");
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// `tebako inspect <artifact> [flags]` — the spec-15 artifact surface.
fn run_inspect(args: &[String]) -> Result<(), CliExit> {
    let mut path: Option<String> = None;
    let mut opts = tebako_cli::inspect::InspectOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--manifest" => opts.manifest = true,
            "--provides" => opts.provides = true,
            "--requires" => opts.requires = true,
            "--platforms" => opts.platforms = true,
            "--json" => opts.json = true,
            "--verify" => opts.verify = true,
            "--require-signed" => {
                opts.verify = true;
                opts.require_signed = true;
            }
            "--backend-json" => opts.backend_json = true,
            "--contract" => opts.contract = true,
            "--slot" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| CliExit::Usage("--slot needs a number".to_string()))?;
                opts.slot = Some(
                    value
                        .parse()
                        .map_err(|_| CliExit::Usage(format!("--slot needs a number, got '{value}'")))?,
                );
            }
            other if path.is_none() && !other.starts_with('-') => path = Some(other.to_string()),
            other => {
                return Err(CliExit::Usage(format!(
                    "unknown inspect option '{other}' (usage: tebako inspect <artifact> [--contract|--manifest|--provides|--requires|--platforms|--json|--verify|--require-signed|--backend-json|--slot N])"
                )))
            }
        }
        i += 1;
    }
    let path =
        path.ok_or_else(|| CliExit::Usage("tebako inspect needs an artifact path".to_string()))?;
    let (out, code) = tebako_cli::inspect::inspect(std::path::Path::new(&path), &opts)?;
    print!("{out}");
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn run_add_registry(args: &[String]) -> Result<(), CliExit> {
    let [registry_ref] = args else {
        return Err(CliExit::Usage(
            "usage: tebako add-registry <ref>".to_string(),
        ));
    };
    let (outcome, registry) = tebako_cli::install::add_registry(&tebako_home()?, registry_ref)?;
    let names: Vec<&str> = registry.payloads.iter().map(|p| p.name.as_str()).collect();
    match outcome {
        tebako_shim::config::AddRegistryOutcome::Added => println!(
            "registered registry {registry_ref} ({} payload(s): {})",
            names.len(),
            names.join(", ")
        ),
        tebako_shim::config::AddRegistryOutcome::AlreadyPresent => {
            println!("registry {registry_ref} was already registered")
        }
    }
    Ok(())
}

fn run_list_registries(args: &[String]) -> Result<(), CliExit> {
    if !args.is_empty() {
        return Err(CliExit::Usage("usage: tebako list-registries".to_string()));
    }
    let registries = tebako_cli::install::list_registries(&tebako_home()?)?;
    if registries.is_empty() {
        println!("no registries registered — tebako add-registry <ref> registers one");
    } else {
        for r in &registries {
            println!("{r}");
        }
    }
    Ok(())
}

fn run_update_registries(args: &[String]) -> Result<(), CliExit> {
    if !args.is_empty() {
        return Err(CliExit::Usage(
            "usage: tebako update-registries".to_string(),
        ));
    }
    let outcome = tebako_cli::install::update_registries(&tebako_home()?)?;
    for r in &outcome.refreshed {
        println!("refreshed {r}");
    }
    for r in &outcome.local {
        println!("{r}: file:// registry — read directly at dispatch, nothing to cache");
    }
    for (r, e) in &outcome.failed {
        eprintln!("tebako: {r}: {e}");
    }
    if outcome.refreshed.is_empty() && outcome.local.is_empty() && outcome.failed.is_empty() {
        println!("no registries registered — tebako add-registry <ref> registers one");
    }
    if !outcome.failed.is_empty() {
        return Err(CliExit::Error(tebako_cli::error::TebakoError::new(
            format!(
                "{} registr{} failed to refresh",
                outcome.failed.len(),
                if outcome.failed.len() == 1 {
                    "y"
                } else {
                    "ies"
                }
            ),
            69,
        )));
    }
    Ok(())
}

fn run_install(args: &[String]) -> Result<(), CliExit> {
    let mut target: Option<&String> = None;
    let mut link_shims = false;
    for arg in args {
        match arg.as_str() {
            "--shims" => link_shims = true,
            _ if target.is_none() => target = Some(arg),
            _ => {
                return Err(CliExit::Usage(
                    "usage: tebako install <ref | name[@version] | ./package> [--shims]"
                        .to_string(),
                ))
            }
        }
    }
    let Some(target) = target else {
        return Err(CliExit::Usage(
            "usage: tebako install <ref | name[@version] | ./package> [--shims]".to_string(),
        ));
    };
    // A local pressed package (fat or lean): slot-wise install from its
    // own bytes (TODO.v2-1/12) — never a registry flow. Shims link only
    // via the explicit --shims.
    let path = std::path::Path::new(target);
    if path.is_file() && tebako_cli::install::is_tpkg_package(path) {
        let outcome = tebako_cli::install::install_local(&tebako_home()?, path, link_shims, None)?;
        for note in &outcome.notes {
            eprintln!("tebako: note: {note}");
        }
        for slice in &outcome.installed {
            match slice.status {
                tebako_resolve::InstallStatus::Hit => {
                    println!(
                        "{} {} already present ({})",
                        slice.name,
                        slice.version,
                        slice.path.display()
                    )
                }
                tebako_resolve::InstallStatus::Installed => println!(
                    "installed {} {} -> {}",
                    slice.name,
                    slice.version,
                    slice.path.display()
                ),
            }
        }
        for shim in &outcome.shims {
            println!("  shim {}", shim.display());
        }
        return Ok(());
    }
    if link_shims {
        return Err(CliExit::Usage(
            "--shims only applies to a local package install (registry installs always register shims)"
                .to_string(),
        ));
    }
    let outcome = tebako_cli::install::install(&tebako_home()?, target, None, None)?;
    for note in &outcome.notes {
        eprintln!("tebako: note: {note}");
    }
    match outcome.status {
        tebako_resolve::InstallStatus::Hit => println!(
            "{} {} is already installed ({})",
            outcome.name,
            outcome.version,
            outcome.path.display()
        ),
        tebako_resolve::InstallStatus::Installed => println!(
            "installed {} {} -> {}",
            outcome.name,
            outcome.version,
            outcome.path.display()
        ),
    }
    if let Some(signer) = &outcome.signer {
        println!("  signature verified (signer {signer})");
    }
    for shim in &outcome.shims {
        println!("  shim {}", shim.display());
    }
    Ok(())
}

fn run_uninstall(args: &[String]) -> Result<(), CliExit> {
    let [name] = args else {
        return Err(CliExit::Usage("usage: tebako uninstall <name>".to_string()));
    };
    let outcome = tebako_cli::install::uninstall(&tebako_home()?, name)?;
    println!("removed {} ({})", outcome.name, outcome.versions.join(", "));
    for shim in &outcome.shims_removed {
        println!("  unlinked {}", shim.display());
    }
    Ok(())
}

/// `<triplet>=<path>` when the left side is a platform triplet, else the
/// universal `<path>` (a path carrying '=' keeps it — the left side must
/// be a triplet for the bound form).
fn parse_payload_arg(value: &str) -> Result<(Option<tpkg::Platform>, PathBuf), CliExit> {
    if let Some((lhs, rhs)) = value.split_once('=') {
        if let Some(platform) = tpkg::Platform::from_triplet(lhs) {
            if rhs.is_empty() {
                return Err(CliExit::Usage(format!(
                    "invalid payload argument '{value}' (<triplet>=<path>)"
                )));
            }
            return Ok((Some(platform), PathBuf::from(rhs)));
        }
    }
    Ok((None, PathBuf::from(value)))
}

fn parse_publish(args: &[String]) -> Result<tebako_cli::publish::PublishOptions, CliExit> {
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut release: Option<String> = None;
    let mut payloads = Vec::new();
    let mut standalones = Vec::new();
    let mut sign: Option<Option<String>> = None;
    let mut upload_mirror: Option<PathBuf> = None;
    let mut tap: Option<String> = None;
    let mut tap_dir: Option<PathBuf> = None;
    let mut license: Option<String> = None;
    let mut desc: Option<String> = None;
    let mut homepage: Option<String> = None;
    let mut registry_out: Option<String> = None;
    let mut skip_verify = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline_value) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let take_value = |i: &mut usize| -> Result<String, CliExit> {
            if let Some(v) = inline_value.clone() {
                return Ok(v);
            }
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| CliExit::Usage(format!("option '{flag}' requires a value")))
        };
        match flag {
            "--name" => name = Some(take_value(&mut i)?),
            "--version" => version = Some(take_value(&mut i)?),
            "--release" => release = Some(take_value(&mut i)?),
            "--payload" => {
                let (triplet, path) = parse_payload_arg(&take_value(&mut i)?)?;
                payloads.push(tebako_cli::publish::PayloadInput { triplet, path });
            }
            "--standalone" => {
                let value = take_value(&mut i)?;
                match parse_payload_arg(&value)? {
                    (Some(triplet), path) => standalones.push((triplet, path)),
                    (None, _) => {
                        return Err(CliExit::Usage(format!(
                            "--standalone expects <triplet>=<path>, got '{value}'"
                        )))
                    }
                }
            }
            "--sign" => sign = Some(inline_value.clone()),
            "--upload-mirror" => upload_mirror = Some(PathBuf::from(take_value(&mut i)?)),
            "--tap" => tap = Some(take_value(&mut i)?),
            "--tap-dir" => tap_dir = Some(PathBuf::from(take_value(&mut i)?)),
            "--license" => license = Some(take_value(&mut i)?),
            "--desc" => desc = Some(take_value(&mut i)?),
            "--homepage" => homepage = Some(take_value(&mut i)?),
            "--registry-out" => registry_out = Some(take_value(&mut i)?),
            "--skip-verify" => skip_verify = true,
            other => return Err(CliExit::Usage(format!("unknown publish option '{other}'"))),
        }
        i += 1;
    }

    let name = name.ok_or_else(|| CliExit::Usage("publish requires --name <app>".to_string()))?;
    let release =
        release.ok_or_else(|| CliExit::Usage("publish requires --release <ref>".to_string()))?;
    if payloads.is_empty() {
        return Err(CliExit::Usage(
            "publish requires at least one --payload".to_string(),
        ));
    }
    if tap_dir.is_some() && tap.is_none() {
        return Err(CliExit::Usage("--tap-dir needs --tap".to_string()));
    }

    Ok(tebako_cli::publish::PublishOptions {
        name,
        version,
        release,
        payloads,
        standalones,
        sign,
        upload_mirror,
        tap,
        tap_dir,
        license,
        desc,
        homepage,
        registry_out,
        skip_verify,
    })
}

fn run_publish(args: &[String]) -> Result<(), CliExit> {
    let opts = parse_publish(args)?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let outcome = tebako_cli::publish::publish(&opts, &tebako_home()?, &cwd)?;
    for note in &outcome.notes {
        eprintln!("tebako: note: {note}");
    }
    println!(
        "published {} {} to tfs:github release tag {}",
        outcome.name, outcome.version, outcome.tag
    );
    for (artifact, sha) in &outcome.artifacts {
        println!("  {artifact}  sha256:{sha}");
    }
    if let Some(signer) = &outcome.signer {
        println!("  signed (keyid {signer}): {} .asc", outcome.ascs.len());
    }
    if let Some(path) = &outcome.registry_path {
        println!("  registry {}", path.display());
    }
    if let Some(path) = &outcome.formula_path {
        println!("  tap formula {}", path.display());
    } else if let Some(formula) = &outcome.formula {
        println!("{formula}");
    }
    if let Some(verified) = &outcome.verified {
        println!("  {verified}");
    }
    Ok(())
}

/// `tebako run <pkg> [--jail <spec>] [--mount <host:mount:ro|rw>]...
/// [--no-host] [--] [<args>...]` — the dispatch surface for a pressed
/// package (spec 08 §2): the flags are the USER tightening; the package's
/// own `jail:` request is composed in (manifest request ∩ user policy)
/// and the effective jail rides TEBAKO_JAIL to the package's bootstrap.
/// Never returns on success on unix (the process is replaced).
fn run_package(args: &[String]) -> Result<(), CliExit> {
    let parsed = tebako_cli::run::parse_run_args(args).map_err(CliExit::Usage)?;
    let plan = tebako_cli::run::plan_run(&parsed).map_err(CliExit::Error)?;
    let err = tebako_cli::run::exec_plan(&plan);
    Err(CliExit::Error(err))
}

/// `tebako trace <run|cover> ...` — the spec 25 front-ends. `run` (§4,
/// phase T1, discovery) runs the package under `TEBAKO_JAIL=record` with
/// the interception bus armed, then synthesizes the capture into a
/// suggested-manifest draft; it never returns on success (the process
/// exits with the payload's exit code). `cover` (§6, phase T3) is the
/// escapes correlator; it never returns either (the exit code is the
/// coverage verdict: 0 clean / 1 escapes / 2 usage-or-IO, the
/// retrace-correlate parity codes). The spawn/resolve emission is T2
/// (bus-side, landed); `import` (the procmon converter, the rest of T3)
/// and `explain` (§5, T4) are later milestones.
fn run_trace(args: &[String]) -> Result<(), CliExit> {
    let Some(action) = args.first() else {
        return Err(CliExit::Usage(
            "trace subcommand expected: run | cover (explain | import are later milestones)"
                .to_string(),
        ));
    };
    match action.as_str() {
        "run" => {
            let parsed = tebako_cli::trace::parse_trace_run_args(&args[1..]).map_err(CliExit::Usage)?;
            tebako_cli::trace::trace_run(&parsed)?;
            Ok(())
        }
        "cover" => tebako_cli::trace::cover::trace_cover(&args[1..]),
        other @ ("explain" | "import") => Err(CliExit::Usage(format!(
            "'tebako trace {other}' is a later tebako-rs milestone (spec 25: import is the rest of T3, explain is T4)"
        ))),
        other => Err(CliExit::Usage(format!("unknown trace subcommand '{other}'"))),
    }
}

/// `tebako check <target> [flags]` — the spec 26 §2 check engine. The
/// verdict lines print as checks run; the engine's return is the process
/// exit code (0, or EX_TEBAKO_CHECK when any check FAILs).
fn run_check(args: &[String]) -> Result<(), CliExit> {
    let parsed = tebako_cli::check::parse_check_args(args).map_err(CliExit::Usage)?;
    let code = tebako_cli::check::run(&parsed)?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn run_cache(args: &[String]) -> Result<(), CliExit> {
    let Some(action) = args.first() else {
        return Err(CliExit::Usage(
            "cache subcommand expected: list | prune".to_string(),
        ));
    };
    match action.as_str() {
        "list" => {
            let mut json = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--json" => json = true,
                    other => return Err(CliExit::Usage(format!("unknown cache option '{other}'"))),
                }
            }
            if json {
                cache_list_json();
            } else {
                cache_list();
            }
            Ok(())
        }
        "prune" => {
            let mut all = false;
            let mut older_than: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--all" => all = true,
                    "--older-than" => {
                        i += 1;
                        older_than = args.get(i).cloned();
                    }
                    s if s.starts_with("--older-than=") => {
                        older_than = Some(s["--older-than=".len()..].to_string());
                    }
                    other => return Err(CliExit::Usage(format!("unknown cache option '{other}'"))),
                }
                i += 1;
            }
            cache_prune(all, older_than.as_deref())?;
            Ok(())
        }
        other => Err(CliExit::Usage(format!(
            "unknown cache subcommand '{other}'"
        ))),
    }
}

/// Hand-rolled press option parsing (Thor's surface: long and short
/// spellings, '--opt=value' and '--opt value').
fn parse_press(args: &[String]) -> Result<PressOptions, CliExit> {
    let mut root: Option<String> = None;
    let mut entrance: Option<String> = None;
    let mut output: Option<String> = None;
    let mut prefix: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut ruby: Option<String> = None;
    let mut mode = PressMode::Lean;
    let mut log_level = "error".to_string();
    let mut image_specs: Vec<String> = Vec::new();
    let mut bootstrap: Option<PathBuf> = None;
    let mut tebako_version = tebako_cli::DEFAULT_TEBAKO_VERSION.to_string();
    let mut prefer_local = false;
    let mut devmode = false;
    let mut suite: Option<PathBuf> = None;
    let mut jail: Option<String> = None;
    let mut no_install = false;
    let mut format = tebako_cli::options::PressImageFormat::Dwarfs;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline_value) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let take_value = |i: &mut usize| -> Result<String, CliExit> {
            if let Some(v) = inline_value.clone() {
                return Ok(v);
            }
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| CliExit::Usage(format!("option '{flag}' requires a value")))
        };
        match flag {
            "-r" | "--root" => root = Some(take_value(&mut i)?),
            "-e" | "--entry-point" | "--entry" => entrance = Some(take_value(&mut i)?),
            "-o" | "--output" => output = Some(take_value(&mut i)?),
            "-p" | "--prefix" => prefix = Some(take_value(&mut i)?),
            "-c" | "--cwd" => cwd = Some(take_value(&mut i)?),
            "-R" | "--Ruby" => ruby = Some(take_value(&mut i)?),
            "-m" | "--mode" => {
                let v = take_value(&mut i)?;
                mode = PressMode::parse(&v).map_err(CliExit::Usage)?;
            }
            "-l" | "--log-level" => log_level = take_value(&mut i)?,
            "--image" => image_specs.push(take_value(&mut i)?),
            "--bootstrap" => bootstrap = Some(PathBuf::from(take_value(&mut i)?)),
            "--tebako-version" => tebako_version = take_value(&mut i)?,
            "--prefer-local" => prefer_local = true,
            "--suite" => suite = Some(PathBuf::from(take_value(&mut i)?)),
            "--jail" => jail = Some(take_value(&mut i)?),
            "--no-install" => no_install = true,
            "--format" => {
                let v = take_value(&mut i)?;
                format =
                    tebako_cli::options::PressImageFormat::parse(&v).map_err(CliExit::Usage)?;
            }
            "-D" | "--devmode" => devmode = true,
            "-t" | "--tebafile" => {
                let _ = take_value(&mut i)?;
                return Err(CliExit::Usage(
                    ".tebako.yml is not supported by the tebako-rs CLI (pass the options directly)"
                        .to_string(),
                ));
            }
            other => return Err(CliExit::Usage(format!("unknown option '{other}'"))),
        }
        i += 1;
    }

    // Thor's required-options check, message shape included. A suite
    // carries its own roots/entries, so -r/-e are neither required nor
    // accepted with --suite.
    if suite.is_some() {
        if root.is_some() || entrance.is_some() {
            return Err(CliExit::Usage(
                "--root/--entry-point come from the suite file with --suite (do not pass them)"
                    .to_string(),
            ));
        }
    } else {
        let mut missing = String::new();
        if root.is_none() {
            missing += " '--root'";
        }
        if entrance.is_none() {
            if !missing.is_empty() {
                missing += ", ";
            }
            missing += " '--entry-point'";
        }
        if !missing.is_empty() {
            return Err(CliExit::Usage(format!(
                "No value provided for required options {missing}"
            )));
        }
    }

    let fs_current = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .to_string_lossy()
        .replace('\\', "/");

    Ok(PressOptions {
        root_arg: root.unwrap_or_default().replace('\\', "/"),
        entrance: entrance.unwrap_or_default().replace('\\', "/"),
        output,
        prefix: resolve_prefix(prefix.as_deref()),
        cwd: cwd.map(|c| c.replace('\\', "/")),
        ruby_requested: ruby,
        mode,
        log_level,
        image_specs,
        bootstrap,
        tebako_version,
        prefer_local,
        verbose: verbose_mode(),
        devmode,
        fs_current,
        suite,
        jail,
        no_install,
        format,
    })
}
