//! The runtime-own command surface (spec 07 §10) — the CLI face.
//!
//! `tebako run <engine>[:<command>] [args…]` boots the engine's runtime
//! directly (the spec-17 wire: the wrapper exe + `TEBAKO_RUNTIME_IMAGE` +
//! `--tebako-entry <token>` + the user args verbatim — no payload
//! resolution, mount composition, spawn lock, or jail computation), and
//! `tebako install <engine>[:<command>]` pre-stages the runtime and
//! registers the runtime-own shim (tebako-shim::runtime_own owns the
//! claims file and the argv0 dispatch of the registered name).
//!
//! The two-class `run` target grammar (MECE, spec 07 §10): a token naming
//! an existing file is a package (the §2/pkg surface, unchanged);
//! otherwise the token parses as `<engine>[:<command>]`. The resolution,
//! the version chain, and the wire plan are tebako-shim's
//! (`runtime_own::engine_resolve` / `engine_plan`) — this module owns only
//! the target grammar and the CLI's verb plumbing.

use std::path::Path;

use crate::error::TebakoError;

/// The usage-error class (spec 07 §7 — the malformed-reference row).
const EX_USAGE: i32 = 64;

/// An engine reference: `engine` (the registry entry's `engine` key —
/// spec 04 §2 MINOR 1's bare lowercase name) plus the optional `:command`
/// (one of the resolved runtime's declared entrypoints).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineRef {
    pub engine: String,
    pub command: Option<String>,
}

/// Parse a target token as the engine grammar. `Ok(None)` = the token is
/// not engine-shaped (the caller's other target class — a payload
/// nickname, a file — decides). `Err` = the token aims at the engine
/// grammar and misses (a `:` form with an empty side, a malformed name) —
/// the usage error, naming the token.
pub fn parse_engine_ref(token: &str) -> Result<Option<EngineRef>, TebakoError> {
    let malformed = |why: &str| {
        TebakoError::new(
            format!("invalid engine reference \"{token}\" ({why}) — the grammar is <engine>[:<command>]"),
            EX_USAGE,
        )
    };
    match token.split(':').collect::<Vec<_>>().as_slice() {
        [engine] => {
            if tebako_shim::runtime_own::valid_engine_name(engine) {
                Ok(Some(EngineRef {
                    engine: (*engine).to_string(),
                    command: None,
                }))
            } else {
                Ok(None)
            }
        }
        [engine, command] => {
            if !tebako_shim::runtime_own::valid_engine_name(engine) {
                return Err(malformed("the engine names [a-z][a-z0-9-]*"));
            }
            tebako_shim::manifest::check_path_component("command name", command)
                .map_err(|e| malformed(&e.message))?;
            Ok(Some(EngineRef {
                engine: (*engine).to_string(),
                command: Some((*command).to_string()),
            }))
        }
        _ => Err(malformed("at most one ':'")),
    }
}

/// The `run` target classes (spec 07 §10's locked order): an existing
/// file is a package; otherwise the engine grammar.
pub enum RunTarget {
    /// The §2/pkg surface — the token named an existing file.
    Package,
    /// The engine surface.
    Engine(EngineRef),
}

/// Classify the first positional token of `tebako run`. A token that is
/// neither an existing file nor a well-formed engine reference is the §7
/// usage error, naming the token (the grammar is total — "not found" for
/// a path-shaped token lands here too, the message says both readings).
pub fn classify_run_target(token: &str) -> Result<RunTarget, TebakoError> {
    if Path::new(token).is_file() {
        return Ok(RunTarget::Package);
    }
    match parse_engine_ref(token)? {
        Some(erf) => Ok(RunTarget::Engine(erf)),
        None => Err(TebakoError::new(
            format!(
                "cannot run \"{token}\": not an existing file and not a well-formed <engine>[:<command>] reference"
            ),
            EX_USAGE,
        )),
    }
}

/// Is `engine` resolvable as a runtime on this machine — the install
/// verb's engine-form test (spec 07 §10): a cached runtime, a config
/// `runtimes:` preference, the product default (ruby), or a registered
/// registry's `kind: runtime` entry answers. Registry rows that cannot be
/// read are skipped — a probe never errors; the nickname path it gates
/// names its own failures.
pub fn engine_known(home: &Path, engine: &str) -> bool {
    if !tpkg::runtime_store::scan_cached(home, engine).is_empty() {
        return true;
    }
    let Ok(cfg) = tebako_shim::config::load_config(home) else {
        return engine == "ruby";
    };
    if cfg.runtimes.contains_key(engine) {
        return true;
    }
    let ctx = tebako_shim::Ctx {
        home: home.to_path_buf(),
        cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
        env: std::env::vars().collect(),
    };
    let Ok(book) = cfg.registry_book_scoped(None) else {
        return engine == "ruby";
    };
    for row in book {
        if let Ok(registry) = tebako_shim::regcache::registry_for(home, row.entry.reference(), &ctx)
        {
            if !registry.runtime_entries(engine, None).is_empty() {
                return true;
            }
        }
    }
    engine == "ruby"
}

/// The `tebako run <engine>[:<command>]` plan: the engine resolved
/// store-first through the shim's runtime-own machinery (the version
/// chain per engine included), the wire composed there; the CLI adapts
/// the dispatch plan to its own exec shape (argv[0] is the program).
pub fn run_engine_plan(erf: &EngineRef, args: &[String]) -> Result<crate::run::RunPlan, TebakoError> {
    let ctx = tebako_shim::Ctx::from_env().map_err(|e| TebakoError::new(e.message, i32::from(e.code)))?;
    let entry = erf.command.as_deref().unwrap_or("self");
    let plan = tebako_shim::runtime_own::engine_plan(&erf.engine, entry, args, &ctx, true)
        .map_err(|e| TebakoError::new(e.message, i32::from(e.code)))?;
    Ok(crate::run::RunPlan {
        program: plan.program,
        args: plan.argv[1..].to_vec(),
        env: plan.env,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_reference_grammar() {
        // bare engine
        let erf = parse_engine_ref("ruby").unwrap().unwrap();
        assert_eq!(erf.engine, "ruby");
        assert_eq!(erf.command, None);
        // engine:command
        let erf = parse_engine_ref("java:keytool").unwrap().unwrap();
        assert_eq!(erf.engine, "java");
        assert_eq!(erf.command.as_deref(), Some("keytool"));
        // not engine-shaped → None (the caller's other classes decide)
        for token in ["metanorma.xml", "Hello", "foo/bar", "./pkg"] {
            assert!(parse_engine_ref(token).unwrap().is_none(), "{token}");
        }
        // malformed engine references → the usage error, naming the token
        for token in ["node:", ":npm", "a:b:c", "Java:node"] {
            let err = parse_engine_ref(token).unwrap_err();
            assert_eq!(err.code, EX_USAGE, "{token}");
            assert!(err.message.contains(token), "{err}");
        }
    }
}
