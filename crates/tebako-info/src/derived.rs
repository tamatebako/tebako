//! Derived facts (spec 15 §2, labeled DERIVED in human mode): computed
//! from the manifest, never stored.
//!
//! - **shims**: the command names this payload would register (from
//!   entrypoints).
//! - **runtime compatibility**: for app payloads, each entrypoint's
//!   runtime constraint evaluated against `~/.tebako/runtimes` —
//!   `satisfied-by` / `requires-download` / `incompatible` (named, never
//!   silent). The cache is probed read-only; nothing is resolved or
//!   downloaded (info is local-only, spec 15 §7).
//! - **dependency names**: payload names reachable via `requires`
//!   (1 level — the full closure is the dispatcher's job).

use std::path::{Path, PathBuf};

use tpkg::{PayloadManifest, Provides, Requirement};

use crate::constraint;

/// One runtime-compatibility verdict for an (engine, constraint) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCompat {
    /// A cached runtime satisfies the constraint.
    SatisfiedBy {
        /// The cache entry name (e.g. `ruby-3.4.2-0.15.9-macos-arm64`).
        entry: String,
    },
    /// Nothing cached satisfies; a download would be needed. Info is
    /// local-only, so the REQUIREMENT is named, not a version: what the
    /// newest compatible would be cannot be known without the network.
    RequiresDownload {
        /// The unsatisfied requirement (`ruby >= 3.3, < 5.0`).
        requirement: String,
    },
    /// The constraint cannot be evaluated here (named reason).
    Incompatible {
        /// Why the requirement is incompatible.
        reason: String,
    },
}

/// The derived block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// Shim command names (entrypoint names, app payloads).
    pub shims: Vec<String>,
    /// One verdict per unique (engine, constraint) entrypoint requirement.
    pub runtime_compat: Vec<RuntimeCompat>,
    /// Dependency payload names, 1 level (deduplicated, order kept).
    pub dependency_names: Vec<String>,
}

/// A parsed runtime-cache entry name
/// (`ruby-<version>-<tebakoabi>-<platform>`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeEntry {
    name: String,
    engine: String,
    version: String,
}

/// Parse a cache entry directory name; `None` for foreign entries.
fn parse_entry_name(name: &str) -> Option<RuntimeEntry> {
    let (engine, rest) = name.split_once('-')?;
    let mut parts = rest.splitn(3, '-');
    let version = parts.next()?;
    let _tebako_abi = parts.next()?;
    let _platform = parts.next()?; // may itself contain '-'
    Some(RuntimeEntry {
        name: name.to_string(),
        engine: engine.to_string(),
        version: version.to_string(),
    })
}

/// The runtime cache root (`$TEBAKO_HOME/runtimes`, else
/// `~/.tebako/runtimes`) — the same root convention as the resolver.
pub fn runtime_cache_dir() -> PathBuf {
    if let Ok(home) = std::env::var("TEBAKO_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join("runtimes");
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".tebako").join("runtimes")
}

/// List the runtime cache (entry names, sorted) — read-only.
fn cached_runtimes() -> Vec<RuntimeEntry> {
    let mut out: Vec<RuntimeEntry> = Vec::new();
    if let Ok(children) = std::fs::read_dir(runtime_cache_dir()) {
        for child in children.flatten() {
            if !child.path().is_dir() {
                continue;
            }
            let name = child.file_name().to_string_lossy().into_owned();
            if let Some(entry) = parse_entry_name(&name) {
                out.push(entry);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Component-wise version ordering (`3.4.2` > `3.10.0` is false; numeric).
fn version_key(v: &str) -> Vec<u64> {
    v.split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect::<Vec<u64>>()
}

/// The newest cached runtime of `engine` satisfying `constraint_text`.
fn newest_satisfying(
    entries: &[RuntimeEntry],
    engine: &str,
    constraint: &tpkg::Constraint,
) -> Result<Option<String>, String> {
    let mut best: Option<(&RuntimeEntry, Vec<u64>)> = None;
    for entry in entries.iter().filter(|e| e.engine == engine) {
        if !constraint::satisfies(constraint, &entry.version)? {
            continue;
        }
        let key = version_key(&entry.version);
        let better = match &best {
            None => true,
            Some((_, best_key)) => key > *best_key,
        };
        if better {
            best = Some((entry, key));
        }
    }
    Ok(best.map(|(entry, _)| entry.name.clone()))
}

/// Compute the derived block of a valid manifest.
pub fn derive(m: &PayloadManifest) -> Derived {
    let mut shims = Vec::new();
    let mut requirements: Vec<(String, tpkg::Constraint)> = Vec::new();
    if let Provides::App(app) = &m.provides {
        for ep in &app.entrypoints {
            shims.push(ep.name.clone());
            let pair = (
                ep.runtime_requirement.engine.clone(),
                ep.runtime_requirement.constraint.clone(),
            );
            if !requirements.contains(&pair) {
                requirements.push(pair);
            }
        }
    }

    let entries = cached_runtimes();
    let runtime_compat = requirements
        .iter()
        .map(|(engine, c)| {
            if !entries.iter().any(|e| e.engine == *engine) {
                // No cached runtime of this engine at all: the requirement
                // is well-formed but unsatisfiable from the local cache.
                return RuntimeCompat::RequiresDownload {
                    requirement: format!("{engine} {c}"),
                };
            }
            match newest_satisfying(&entries, engine, c) {
                Ok(Some(entry)) => RuntimeCompat::SatisfiedBy { entry },
                Ok(None) => RuntimeCompat::RequiresDownload {
                    requirement: format!("{engine} {c}"),
                },
                Err(reason) => RuntimeCompat::Incompatible { reason },
            }
        })
        .collect();

    let mut dependency_names: Vec<String> = Vec::new();
    for req in &m.requires {
        let name = match req {
            Requirement::Language { engine, .. } => engine,
            Requirement::Toolkit { name, .. } => name,
            Requirement::Data { name, .. } => name,
        };
        if !dependency_names.contains(name) {
            dependency_names.push(name.clone());
        }
    }

    Derived {
        shims,
        runtime_compat,
        dependency_names,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_runtime_entry_names() {
        let e = parse_entry_name("ruby-3.4.2-0.15.9-macos-arm64").unwrap();
        assert_eq!(e.engine, "ruby");
        assert_eq!(e.version, "3.4.2");
        assert!(parse_entry_name("ruby-3.4.2-0.15.9").is_none());
        assert!(parse_entry_name("stray").is_none());
    }

    #[test]
    fn newest_satisfying_picks_the_max_version() {
        let entries = vec![
            parse_entry_name("ruby-3.3.7-0.15.9-macos-arm64").unwrap(),
            parse_entry_name("ruby-3.4.2-0.15.9-macos-arm64").unwrap(),
            parse_entry_name("ruby-5.0.0-0.15.9-macos-arm64").unwrap(),
        ];
        let c = tpkg::Constraint::new(">= 3.3, < 5.0").unwrap();
        assert_eq!(
            newest_satisfying(&entries, "ruby", &c).unwrap(),
            Some("ruby-3.4.2-0.15.9-macos-arm64".to_string())
        );
        let none = tpkg::Constraint::new(">= 9.9").unwrap();
        assert_eq!(newest_satisfying(&entries, "ruby", &none).unwrap(), None);
        let abi = tpkg::Constraint::new("~> 3.3.0").unwrap();
        assert_eq!(
            newest_satisfying(&entries, "ruby", &abi).unwrap(),
            Some("ruby-3.3.7-0.15.9-macos-arm64".to_string())
        );
    }

    #[test]
    fn derive_collects_shims_and_dependency_names() {
        let m = PayloadManifest::from_yaml(include_str!(
            "../../tpkg/tests/fixtures/manifests/app-suite.yaml"
        ))
        .unwrap();
        let d = derive(&m);
        assert_eq!(d.shims, vec!["metanorma", "metanorma-nokogiri"]);
        assert_eq!(d.dependency_names, vec!["ruby", "gtk-layer"]);
        // Two distinct constraints → two verdicts (cache-independent).
        assert_eq!(d.runtime_compat.len(), 2);
    }
}
