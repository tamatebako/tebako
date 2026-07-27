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
               [--tebako-version <v>]
  tebako cache list [--json]
  tebako cache prune [--all] [--older-than Nd]";

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
    // banner on stdout (unchanged).
    let json_cache_list = subcommand == "cache"
        && rest.first().map(|a| a.as_str()) == Some("list")
        && rest.iter().any(|a| a == "--json");
    if json_cache_list {
        eprintln!("{VERSION_BANNER}");
    } else {
        println!("{VERSION_BANNER}");
    }
    match subcommand {
        "press" => {
            let opts = parse_press(rest)?;
            press(&opts)?;
            Ok(())
        }
        "cache" => run_cache(rest),
        "clean" | "setup" | "hash" => Err(CliExit::Usage(format!(
            "'tebako {subcommand}' is a later tebako-rs milestone"
        ))),
        other => Err(CliExit::Usage(format!("unknown command '{other}'"))),
    }
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
    let mut devmode = false;

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

    // Thor's required-options check, message shape included.
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

    let fs_current = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .to_string_lossy()
        .replace('\\', "/");

    Ok(PressOptions {
        root_arg: root.unwrap().replace('\\', "/"),
        entrance: entrance.unwrap().replace('\\', "/"),
        output,
        prefix: resolve_prefix(prefix.as_deref()),
        cwd: cwd.map(|c| c.replace('\\', "/")),
        ruby_requested: ruby,
        mode,
        log_level,
        image_specs,
        bootstrap,
        tebako_version,
        verbose: verbose_mode(),
        devmode,
        fs_current,
    })
}
