//! tebako-bootstrap — the launcher (part A of the three-part package).
//!
//! Exit codes: 65 manifest, 66 ABI, 67 runtime_ref, 69 unavailable, 70
//! SHA256, 74 i/o. Success execs the runtime (no return).

use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    match tebako_bootstrap::run(&argv) {
        Ok(_) => ExitCode::SUCCESS, // unreachable on unix (exec replaces us)
        Err(e) => {
            eprintln!("tebako-bootstrap: {}", e.message);
            ExitCode::from(e.code)
        }
    }
}
