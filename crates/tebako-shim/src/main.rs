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
    let ctx = match Ctx::from_env() {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("tebako-shim: {}", e.message);
            return ExitCode::from(e.code);
        }
    };
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
