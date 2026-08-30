//! tebako-runtime-launcher — the spec-29 wrapper exe: the standalone
//! form of the tebako runtime driver for REPACKED runtimes (upstream
//! interpreter bytes tebako does not compile; the openjdk promotion is
//! the first instance — ecosystem TODO.java/04).
//!
//! The binary is deliberately thin: every decision (the spec-17 boot,
//! the interpreter declaration, the visibility mechanism, the argv
//! composition, `--tebako-extract`) lives in `tebako_driver::wrapper` —
//! the pattern's one home. What remains here is the process layer
//! (spec 29 §1/§4):
//!
//! - POSIX: `exec` — the interpreter REPLACES the wrapper process (no
//!   extra process; signals and the exit code behave naturally);
//! - windows: spawn the interpreter, wait, and propagate the child's
//!   exit code VERBATIM — console control events reach the child
//!   through the shared console (std's default); the wrapper never
//!   swallows, remaps, or invents exit codes;
//! - failures: the driver error's named code passes through (65–74,
//!   78); an exec/spawn rejection of the materialized interpreter is
//!   loader-side — exit 65 (spec 29 §4).

#![forbid(unsafe_code)]

use tebako_driver::wrapper::{run, BootAction, Launch, WRAPPER_RUNTIME_ROOT};
use tebako_driver::ProcessEnv;

/// An exec/spawn rejection of the materialized interpreter is
/// loader-side (spec 29 §4) — the loader's 65 class (spec 06 §4). The
/// driver crate keeps its code constants private; the code is the
/// contract.
const EX_SPAWN: i32 = 65;
/// The wait on a spawned child itself failed (windows path) — the
/// child's code is unknowable; named 74 (the loader's IO class), never
/// an invented child status.
#[cfg(windows)]
const EX_WAIT: i32 = 74;

fn main() {
    // argv may carry non-UTF-8 bytes (the C argv is bytes) — lossy
    // matches the driver's own C-entry conversion.
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    match run(&argv, WRAPPER_RUNTIME_ROOT, &ProcessEnv) {
        Err(e) => {
            eprintln!("tebako-runtime-launcher: {}", e.message);
            std::process::exit(e.code);
        }
        Ok(BootAction::Extracted {
            dest,
            skipped_symlinks,
        }) => {
            // Driver-side `--tebako-extract` (spec 29 §4): dump and exit
            // 0 — the interpreter never runs. The note is stderr:
            // stdout stays the payload's.
            eprintln!(
                "tebako-runtime-launcher: extracted the mounted images to '{dest}' (skipped {skipped_symlinks} symlinks)"
            );
        }
        Ok(BootAction::Launch(plan)) => launch(plan),
    }
}

/// POSIX: the interpreter REPLACES the wrapper process (spec 29 §1).
/// `CommandExt::exec` returns only on failure — the materialized exe
/// rejected by execve is loader-side, exit 65 (spec 29 §4).
#[cfg(unix)]
fn launch(launch: Launch) -> ! {
    use std::os::unix::process::CommandExt as _;
    let err = std::process::Command::new(&launch.program)
        .args(launch.argv.get(1..).unwrap_or(&[]))
        .exec();
    eprintln!(
        "tebako-runtime-launcher: cannot exec the materialized interpreter '{}': {err}",
        launch.program
    );
    std::process::exit(EX_SPAWN);
}

/// Windows (no exec): spawn the interpreter as a child, wait, and exit
/// with the child's code verbatim (spec 29 §1).
#[cfg(windows)]
fn launch(launch: Launch) -> ! {
    let mut child = match std::process::Command::new(&launch.program)
        .args(launch.argv.get(1..).unwrap_or(&[]))
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            // CreateProcess rejecting the materialized exe is
            // loader-side (spec 29 §4).
            eprintln!(
                "tebako-runtime-launcher: cannot spawn the materialized interpreter '{}': {e}",
                launch.program
            );
            std::process::exit(EX_SPAWN);
        }
    };
    match child.wait() {
        Ok(status) => match status.code() {
            Some(code) => std::process::exit(code),
            // std documents no signal case on windows — the status
            // always carries the child's code (a control-event death
            // included). If that contract is ever violated the run's
            // status is unknowable: name it, never invent a code.
            None => {
                eprintln!(
                    "tebako-runtime-launcher: the interpreter child ended without an exit code"
                );
                std::process::exit(EX_WAIT);
            }
        },
        Err(e) => {
            eprintln!(
                "tebako-runtime-launcher: lost the interpreter child '{}': {e}",
                launch.program
            );
            std::process::exit(EX_WAIT);
        }
    }
}
