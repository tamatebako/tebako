//! Port of the gem's BuildHelpers (lib/tebako/build_helpers.rb): the
//! environment scrub for runtime spawns and the capture-and-raise command
//! runner.

use std::path::Path;
use std::process::Command;

use crate::error::{plain_error, TebakoError};

/// Environment keys unset for every spawned runtime process: RUBYOPT /
/// RUBYLIB plus every BUNDLE_* / BUNDLER_* variable currently set (an
/// inherited bundler context boots the runtime's ruby into the press's own
/// bundle — gem commit 8adcc31).
const RUBY_ENV_SCRUB: [&str; 2] = ["RUBYOPT", "RUBYLIB"];
const RUBY_ENV_SCRUB_PREFIXES: [&str; 2] = ["BUNDLE_", "BUNDLER_"];

pub fn ruby_env_scrub() -> Vec<String> {
    let mut scrub: Vec<String> = RUBY_ENV_SCRUB.iter().map(|s| s.to_string()).collect();
    for (key, _) in std::env::vars_os() {
        if let Some(key) = key.to_str() {
            if RUBY_ENV_SCRUB_PREFIXES.iter().any(|p| key.starts_with(p))
                && !scrub.iter().any(|k| k == key)
            {
                scrub.push(key.to_string());
            }
        }
    }
    scrub
}

/// `VERBOSE=yes/true` switches mkdwarfs/extract/deploy to verbose.
pub fn verbose_mode() -> bool {
    matches!(std::env::var("VERBOSE").as_deref(), Ok("yes") | Ok("true"))
}

/// Build the scrubbed command: `env_sets` are exported (in order), the
/// scrub keys are removed, output is captured combined (stderr merged into
/// stdout like Open3.capture2e).
fn build_command(program: &Path, args: &[String], env_sets: &[(String, String)]) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(args);
    for key in ruby_env_scrub() {
        cmd.env_remove(key);
    }
    for (k, v) in env_sets {
        cmd.env(k, v);
    }
    cmd
}

fn status_string(status: std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit status {code}"),
        None => format!("signal {}", {
            #[cfg(unix)]
            {
                std::os::unix::process::ExitStatusExt::signal(&status).unwrap_or(-1)
            }
            #[cfg(not(unix))]
            {
                -1
            }
        }),
    }
}

/// Run `args`, capturing combined output; raise on failure with the gem's
/// message shape. The command line is always announced ("   ... @ ...").
pub fn run_with_capture(
    program: &Path,
    args: &[String],
    env_sets: &[(String, String)],
) -> Result<String, TebakoError> {
    let full: Vec<String> = std::iter::once(program.to_string_lossy().into_owned())
        .chain(args.iter().cloned())
        .collect();
    println!("   ... @ {}", full.join(" "));
    let output = match build_command(program, args, env_sets).output() {
        Ok(o) => o,
        Err(e) => {
            return Err(plain_error(format!(
                "Failed to run {} (spawn failed: {e}):\n ",
                full.join(" ")
            )))
        }
    };
    let mut out = String::from_utf8_lossy(&output.stdout).into_owned();
    out.push_str(&String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(plain_error(format!(
            "Failed to run {} ({}):\n {}",
            full.join(" "),
            status_string(output.status),
            out
        )));
    }
    Ok(out)
}

/// Verbose variant: appends `--verbose` and prints the captured output.
pub fn run_with_capture_v(
    program: &Path,
    args: &[String],
    env_sets: &[(String, String)],
    verbose: bool,
) -> Result<String, TebakoError> {
    if verbose {
        let mut args_v = args.to_vec();
        args_v.push("--verbose".to_string());
        let out = run_with_capture(program, &args_v, env_sets)?;
        print!("{out}");
        Ok(out)
    } else {
        run_with_capture(program, args, env_sets)
    }
}
