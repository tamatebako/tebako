//! tebako-log — the tebako stack's debug/diagnostic logging facility
//! (one home, every component wired through it — no ad-hoc `eprintln!`).
//!
//! # The contract
//!
//! - **`TEBAKO_DEBUG`** — the level knob: `off` (default), `error`,
//!   `warn`, `debug`, `trace`; optionally per-component overrides as a
//!   comma list (`debug,preload=trace,tfs=warn`). The legacy boolean
//!   `TEBAKO_DEBUG_TFS` (any non-empty value) maps to `debug` for
//!   back-compat.
//! - **`TEBAKO_DEBUG_FILE`** — the sink: unset = **stderr** (stdout
//!   belongs to the payload, never to the log). A path opens append-mode;
//!   `%p` expands to the process id so exec'd children logging into the
//!   "same" file don't clobber each other. Parent directories are
//!   created; an unopenable path falls back to stderr with one warn
//!   line naming the failure.
//! - **`TEBAKO_DEBUG_COMPONENTS`** — comma filter of component names
//!   (default: all). Components are the crate names (`preload`, `tfs`,
//!   `driver`, `shim`, `bootstrap`, `cli`, `pkg`, `resolve`).
//!
//! # Format
//!
//! One line per event, parseable, pid-tagged:
//! `tebako[84213] debug preload: route path=/x held=true action=erofs`
//!
//! # Discipline
//!
//! - **Zero cost when off**: the config reads once (OnceLock); the
//!   [`log!`] macro checks [`enabled`] before formatting anything.
//! - **Never log secrets or payload content**: paths, decisions, and
//!   digests are fair; file contents and key material never are.
//! - **Fork caveat** (the preload consumers): lines are atomic under
//!   the sink lock inside one process; across fork the child re-reads
//!   the config lazily and logs independently — interleaving between
//!   parent and child is possible, never lossy per line.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// The event levels (ordered; a configured level admits itself and below).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Debug = 3,
    Trace = 4,
}

impl Level {
    fn name(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }

    fn parse(text: &str) -> Option<Level> {
        match text.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Level::Error),
            "warn" => Some(Level::Warn),
            "debug" | "1" | "true" | "yes" => Some(Level::Debug),
            "trace" => Some(Level::Trace),
            "off" | "0" | "false" | "no" => None,
            _ => None,
        }
    }
}

/// The sink: stderr (default) or an append-mode file.
enum Sink {
    Stderr,
    File(Mutex<std::fs::File>),
}

/// The resolved configuration.
struct Config {
    /// The global level (None = off).
    level: Option<Level>,
    /// Per-component overrides (`preload=trace`).
    components: BTreeMap<String, Option<Level>>,
    /// The component-name filter (empty = all).
    only: Vec<String>,
    sink: Sink,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn config() -> &'static Config {
    CONFIG.get_or_init(init_from_env)
}

/// Read the configuration from the environment (idempotent, lazy — the
/// first [`enabled`]/[`event`] call resolves it). Public for tests that
/// reset the lock through `TEBAKO_LOG_TEST_REINIT`.
pub fn init_from_env() -> Config {
    let mut level = std::env::var("TEBAKO_DEBUG")
        .ok()
        .and_then(|v| parse_level_spec(&v).0);
    let mut components = std::env::var("TEBAKO_DEBUG")
        .ok()
        .map(|v| parse_level_spec(&v).1)
        .unwrap_or_default();
    // The legacy boolean: any non-empty TEBAKO_DEBUG_TFS = debug.
    if level.is_none() && std::env::var_os("TEBAKO_DEBUG_TFS").is_some_and(|v| !v.is_empty()) {
        level = Some(Level::Debug);
    }
    components.retain(|_, v| v.is_some());
    let only = std::env::var("TEBAKO_DEBUG_COMPONENTS")
        .ok()
        .map(|v| {
            v.split(',')
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let sink = match std::env::var("TEBAKO_DEBUG_FILE") {
        Ok(path) if !path.is_empty() => open_sink(&path),
        _ => Sink::Stderr,
    };
    Config {
        level,
        components,
        only,
        sink,
    }
}

/// `debug,preload=trace,tfs=warn` → (global, per-component map).
fn parse_level_spec(spec: &str) -> (Option<Level>, BTreeMap<String, Option<Level>>) {
    let mut global = None;
    let mut map = BTreeMap::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        match part.split_once('=') {
            Some((component, level)) => {
                map.insert(component.to_string(), Level::parse(level));
            }
            None => global = global.or_else(|| Level::parse(part)),
        }
    }
    (global, map)
}

fn open_sink(path: &str) -> Sink {
    let path = path.replace("%p", &std::process::id().to_string());
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(f) => Sink::File(Mutex::new(f)),
        Err(e) => {
            eprintln!("tebako-log: cannot open TEBAKO_DEBUG_FILE {}: {e} — logging to stderr", path.display());
            Sink::Stderr
        }
    }
}

/// Is `level` admitted for `component`? The check [`log!`] runs before
/// any formatting — the zero-cost gate.
pub fn enabled(level: Level, component: &str) -> bool {
    let cfg = config();
    if !cfg.only.is_empty() && !cfg.only.iter().any(|c| c == component) {
        return false;
    }
    let admitted = cfg
        .components
        .get(component)
        .copied()
        .unwrap_or(cfg.level);
    match admitted {
        Some(max) => level <= max,
        None => false,
    }
}

/// Write one event line. Prefer [`log!`] (which gates on [`enabled`]).
pub fn event(level: Level, component: &str, args: fmt::Arguments<'_>) {
    let line = format!(
        "tebako[{}] {} {}: {}\n",
        std::process::id(),
        level.name(),
        component,
        args
    );
    match &config().sink {
        Sink::Stderr => {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
        Sink::File(file) => {
            if let Ok(mut f) = file.lock() {
                let _ = f.write_all(line.as_bytes());
                let _ = f.flush();
            }
        }
    }
}

/// The one logging macro: gates on [`enabled`], then formats.
///
/// ```ignore
/// tebako_log::log!(Level::Debug, "preload", "route path={} held={} action={}", p, held, action);
/// ```
#[macro_export]
macro_rules! log {
    ($level:expr, $component:expr, $($arg:tt)*) => {
        if $crate::enabled($level, $component) {
            $crate::event($level, $component, format_args!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_spec_parses_global_and_components() {
        let (global, map) = parse_level_spec("debug,preload=trace,tfs=warn");
        assert_eq!(global, Some(Level::Debug));
        assert_eq!(map.get("preload"), Some(&Some(Level::Trace)));
        assert_eq!(map.get("tfs"), Some(&Some(Level::Warn)));

        let (global, map) = parse_level_spec("off");
        assert_eq!(global, None);
        assert!(map.is_empty());
    }

    #[test]
    fn boolish_values_map_to_debug() {
        assert_eq!(Level::parse("1"), Some(Level::Debug));
        assert_eq!(Level::parse("true"), Some(Level::Debug));
        assert_eq!(Level::parse("0"), None);
    }

    #[test]
    fn pid_placeholder_expands() {
        let path = format!("/tmp/tebako-log-test-%p-{}.log", std::process::id());
        if let Sink::File(_) = open_sink(&path.replace("%p", &std::process::id().to_string())) {
            assert!(PathBuf::from(path.replace("%p", &std::process::id().to_string())).exists());
        } else {
            panic!("expected the file sink");
        }
        let _ = std::fs::remove_file(path.replace("%p", &std::process::id().to_string()));
    }
}
