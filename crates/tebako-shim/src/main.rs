//! tebako-shim — the dispatcher (spec 07). Linked per command name under
//! ~/.tebako/shims/; also the management entry point.
//!
//! Exit codes (spec 06 §4 reused): 64 usage, 65 manifest/record, 69
//! runtime unresolvable, 70 sha256, 74 i/o. Dispatch execs the target
//! (no return on success).

use std::process::ExitCode;

use tebako_shim::{Action, Ctx};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    // spec 05 §3.1: a bundle-sourced home is exported up front so every
    // downstream tier (Ctx, the exec'd runtime, spawned payloads) agrees
    // on the store the run resolves from; an explicit TEBAKO_HOME is
    // never overridden.
    if std::env::var_os("TEBAKO_HOME").map_or(true, |v| v.is_empty()) {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(home) = tpkg::runtime_store::bundle_sibling_home(&exe) {
                std::env::set_var("TEBAKO_HOME", &home);
            }
        }
    }
    let ctx = match Ctx::from_env() {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("tebako-shim: {}", e.message);
            return ExitCode::from(e.code);
        }
    };
    // Enterprise networking (TODO.v2-1/33): config.yaml's network:
    // section under the env, installed before any fetch; a malformed
    // section is the named error here, at startup.
    if let Err(e) = tebako_shim::config::install_network_config(&ctx.home) {
        eprintln!("tebako-shim: {}", e.message);
        return ExitCode::from(e.code);
    }
    match tebako_shim::run(&argv, &ctx) {
        Ok(Action::Print { text, code }) => {
            print!("{text}");
            ExitCode::from(code)
        }
        Ok(Action::Exec(plan)) => {
            let err = tebako_shim::dispatch::exec(&plan);
            eprintln!("tebako-shim: {}", err.message);
            ExitCode::from(err.code)
        }
        Err(e) => {
            eprintln!("tebako-shim: {}", e.message);
            ExitCode::from(e.code)
        }
    }
}
