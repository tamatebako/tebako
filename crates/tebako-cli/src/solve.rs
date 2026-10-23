//! `tebako solve` — tree solving and lockfiles (spec 42).
//!
//! Resolution today answers one edge at a time. A composition is a
//! constraint problem: the maximal assignment that satisfies every edge
//! at once, or a named conflict that says which requirement, which
//! candidates, and why.
//!
//! This module owns three things and nothing else:
//!
//! - the requirement set (edges, the closure map, the host triplet);
//! - the search (deterministic, one version per name, no silent
//!   downgrade);
//! - the lockfile (content anchors and snapshot roots, never URLs;
//!   byte-identical round-trip; `--locked` refuses drift).
//!
//! Runtime edges (`language`, `runtime`) are NOT solved here: their pick
//! is spec 05's resolver, and a second matcher in this file would drift
//! from the dispatcher's. They are refused by name until that seam is
//! wired (spec 42 §2).

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tebako_resolve::registry::{PlatformSelection, Registry};
use tpkg::Platform;

use crate::error::TebakoError;

/// The solver's exit class (spec 42 §5): a named conflict, lockfile
/// drift, or a malformed requirement set.
pub const EX_TEBAKO_SOLVE: i32 = 81;

/// The lockfile's own schema version (spec 42 §6).
pub const LOCK_VERSION: u32 = 1;

/// The canonical file name (spec 42 §6).
pub const LOCK_FILE: &str = "tebako.lock";

/// One requirement edge (spec 03 §2.3's grammar, minus the fields that
/// are mount-side only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub kind: EdgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    pub constraint: String,
    /// The triplets the edge resolves on; absent = every triplet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triplets: Option<Vec<String>>,
}

impl Edge {
    /// How the edge is spelled in a conflict or a lockfile note.
    fn declared(&self) -> String {
        let target = match (&self.name, &self.engine) {
            (Some(name), _) => name.clone(),
            (None, Some(engine)) => match &self.implementation {
                Some(implementation) => format!("{engine}({implementation})"),
                None => engine.clone(),
            },
            (None, None) => "<no target>".to_string(),
        };
        format!("{} {target} {}", self.kind, self.constraint)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    App,
    Data,
    Language,
    Runtime,
    Toolkit,
}

impl EdgeKind {
    fn rank(self) -> u8 {
        match self {
            EdgeKind::App => 0,
            EdgeKind::Data => 1,
            EdgeKind::Language => 2,
            EdgeKind::Runtime => 3,
            EdgeKind::Toolkit => 4,
        }
    }

    fn is_payload_edge(self) -> bool {
        matches!(self, EdgeKind::App | EdgeKind::Data | EdgeKind::Toolkit)
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeKind::App => "app",
            EdgeKind::Data => "data",
            EdgeKind::Language => "language",
            EdgeKind::Runtime => "runtime",
            EdgeKind::Toolkit => "toolkit",
        };
        f.write_str(s)
    }
}

/// The solved `(kind, name) -> version` row (spec 42 §4/§6).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Assignment {
    pub kind: EdgeKind,
    pub name: String,
    pub version: String,
    pub triplet: String,
    /// The artifact's own anchor, or `-` for a universal row whose pin
    /// lives in the release ref (spec 41 §2's rendering convention).
    pub sha256: String,
}

/// An edge that does not apply on this host (spec 42 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skipped {
    pub edge: String,
    pub triplet: String,
    pub reason: String,
}

/// `tebako.lock` (spec 42 §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    pub lock_version: u32,
    pub assignments: Vec<Assignment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<Skipped>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<String>,
}

/// The requirement set the solver consumes (spec 42 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveRequest {
    pub triplet: String,
    #[serde(default)]
    pub requires: Vec<Edge>,
    /// `<kind> <name>@<version>` -> that payload's own `requires:`
    /// (from L1 — the registry does not carry DEPENDS by the layer
    /// model). A missing key for a chosen candidate is a named
    /// `solve-closure-missing`.
    #[serde(default)]
    pub closure: BTreeMap<String, Vec<Edge>>,
}

impl SolveRequest {
    pub fn from_yaml(text: &str) -> Result<SolveRequest, TebakoError> {
        serde_yml::from_str(text).map_err(|e| {
            TebakoError::new(
                format!("the requirement set is not parseable: {e}"),
                EX_TEBAKO_SOLVE,
            )
        })
    }
}

/// A candidate the search may pick.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    kind: EdgeKind,
    name: String,
    version: String,
    triplet: String,
    sha256: String,
    /// The closure key: `<kind> <name>@<version>`.
    key: String,
}

impl Candidate {
    fn assignment(&self) -> Assignment {
        Assignment {
            kind: self.kind,
            name: self.name.clone(),
            version: self.version.clone(),
            triplet: self.triplet.clone(),
            sha256: self.sha256.clone(),
        }
    }
}

/// Why a row was not a candidate (spec 42 §5's `excluded` column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exclusion {
    Constraint,
    Platform,
    Withdrawn,
}

impl Exclusion {
    fn as_str(self) -> &'static str {
        match self {
            Exclusion::Constraint => "constraint",
            Exclusion::Platform => "platform",
            Exclusion::Withdrawn => "withdrawn",
        }
    }
}

/// One failed search step, rendered by [`solve_conflict`].
#[derive(Debug, Clone)]
struct Conflict {
    edge: Edge,
    triplet: String,
    depth: usize,
    rows: Vec<(String, Option<Exclusion>)>,
    /// The reason the search could not proceed where the edge's target
    /// was already assigned (spec 42 §5's `pinned`).
    pinned: bool,
}

impl Conflict {
    fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "solve-conflict: {}\n  needs {} on {}\n",
            self.edge.declared(),
            self.edge.kind,
            self.triplet
        ));
        let live: Vec<&str> = self
            .rows
            .iter()
            .filter(|(_, excluded)| excluded.is_none())
            .map(|(version, _)| version.as_str())
            .collect();
        out.push_str(&format!(
            "  candidates: {}\n",
            if live.is_empty() {
                "-".to_string()
            } else {
                live.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ));
        let excluded: Vec<String> = self
            .rows
            .iter()
            .filter_map(|(version, excluded)| {
                excluded.map(|e| format!("{version} — {}", e.as_str()))
            })
            .collect();
        if !excluded.is_empty() {
            out.push_str(&format!("  excluded: {}\n", excluded.join("; ")));
        }
        if self.pinned {
            out.push_str(
                "  pinned: another edge of this set requires a different version of the same name\n",
            );
        }
        out.push_str(&format!("  fix: {}\n", self.fix()));
        out
    }

    /// The smallest change that would satisfy the edge, when one is
    /// derivable from the rows in hand. Never a guess about versions the
    /// registries do not carry.
    fn fix(&self) -> String {
        if self.pinned {
            return "choose one version of the name, or split the composition".to_string();
        }
        let live: Vec<&String> = self
            .rows
            .iter()
            .filter(|(_, excluded)| excluded.is_none())
            .map(|(version, _)| version)
            .collect();
        if live.is_empty() {
            return "no row in the configured registries satisfies this edge — configure a registry that carries it, or relax the constraint"
                .to_string();
        }
        format!(
            "the live candidates ({}) exist; the edge was rejected for the reason above",
            live.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// The solve failed: always [`EX_TEBAKO_SOLVE`], always named.
fn solve_error(message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, EX_TEBAKO_SOLVE)
}

/// Solve a requirement set (spec 42 §4).
///
/// `registries` are the documents the press is configured with; their
/// order never affects the answer.
pub fn solve(registries: &[&Registry], request: &SolveRequest) -> Result<Lock, TebakoError> {
    let host = Platform::from_str(&request.triplet).map_err(|e| {
        solve_error(format!(
            "triplet {:?} is not a tebako platform: {e}",
            request.triplet
        ))
    })?;

    // Constraints are parsed once, up front: a malformed constraint is a
    // requirement-set error, not a solve failure.
    for edge in request
        .requires
        .iter()
        .chain(request.closure.values().flatten())
    {
        if !edge.kind.is_payload_edge() {
            return Err(solve_error(format!(
                "solve-runtime-edge: {} — a runtime edge's pick is spec 05's resolver; \
                 tree solving covers payload edges (app, data, toolkit) until that seam is wired",
                edge.declared()
            )));
        }
        tpkg::versions::parse_constraint(&edge.constraint)
            .map_err(|e| solve_error(format!("{}: {e}", edge.declared())))?;
    }

    let mut skipped: Vec<Skipped> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    for edge in &request.requires {
        if applies_on(edge, &host) {
            edges.push(edge.clone());
        } else {
            skipped.push(Skipped {
                edge: edge.declared(),
                triplet: host.to_string(),
                reason: "triplet-not-listed".to_string(),
            });
        }
    }

    let mut assigned: BTreeMap<String, Candidate> = BTreeMap::new();
    let mut deepest: Option<Conflict> = None;
    let ok = search(
        registries,
        request,
        &host,
        &mut assigned,
        edges,
        &mut deepest,
        0,
    )?;
    if !ok {
        let conflict = deepest.expect("a failed search records its deepest conflict");
        return Err(solve_error(conflict.render()));
    }

    let mut assignments: Vec<Assignment> = assigned.values().map(Candidate::assignment).collect();
    assignments.sort();
    skipped.sort_by(|a, b| {
        (a.edge.clone(), a.triplet.clone()).cmp(&(b.edge.clone(), b.triplet.clone()))
    });
    Ok(Lock {
        lock_version: LOCK_VERSION,
        assignments,
        skipped,
        registries: Vec::new(),
    })
}

/// Does the edge apply on this host? An edge with no `triplets:` applies
/// everywhere; otherwise the host must be listed (spec 03 §2.3).
fn applies_on(edge: &Edge, host: &Platform) -> bool {
    match &edge.triplets {
        None => true,
        Some(list) => list.iter().any(|t| t == &host.to_string()),
    }
}

/// The depth-first search (spec 42 §4): assign in canonical order,
/// highest version first, backtracking when a deeper edge cannot be
/// satisfied. The first complete assignment is therefore the maximal
/// one under the spec's component order.
#[allow(clippy::too_many_arguments)]
fn search(
    registries: &[&Registry],
    request: &SolveRequest,
    host: &Platform,
    assigned: &mut BTreeMap<String, Candidate>,
    mut pending: Vec<Edge>,
    deepest: &mut Option<Conflict>,
    depth: usize,
) -> Result<bool, TebakoError> {
    pending.sort_by_key(|a| (a.kind.rank(), target_key(a)));

    // Edges whose name is already assigned are decided: check, never
    // assign a second version of the same name (spec 42 §4's one-version
    // law).
    loop {
        let mut progressed = false;
        let mut index = 0;
        while index < pending.len() {
            let edge = pending[index].clone();
            if let Some(name) = &edge.name {
                if let Some(existing) = assigned.get(name) {
                    let constraint = tpkg::versions::parse_constraint(&edge.constraint)
                        .map_err(|e| solve_error(format!("{}: {e}", edge.declared())))?;
                    if !constraint.matches(&existing.version) {
                        remember(
                            deepest,
                            Conflict {
                                edge,
                                triplet: host.to_string(),
                                depth,
                                rows: vec![(existing.version.clone(), None)],
                                pinned: true,
                            },
                        );
                        return Ok(false);
                    }
                    pending.remove(index);
                    progressed = true;
                    continue;
                }
            }
            index += 1;
        }
        if !progressed {
            break;
        }
    }

    let Some(edge) = pending.first().cloned() else {
        return Ok(true);
    };

    let (live, rows) = candidates(registries, &edge, host);
    if live.is_empty() {
        remember(
            deepest,
            Conflict {
                edge: edge.clone(),
                triplet: host.to_string(),
                depth,
                rows,
                pinned: false,
            },
        );
        return Ok(false);
    }

    for candidate in live {
        let key = candidate.key.clone();
        assigned.insert(candidate.name.clone(), candidate.clone());
        let mut expanded: Vec<Edge> = pending[1..].to_vec();
        match request.closure.get(&key) {
            Some(children) => {
                for child in children {
                    if applies_on(child, host) {
                        expanded.push(child.clone());
                    }
                }
            }
            None => {
                assigned.remove(&candidate.name);
                return Err(solve_error(format!(
                    "solve-closure-missing: {key} has no entry in the closure map — the press must supply the payload's own requires: (L1), an implicit \"no dependencies\" is never assumed"
                )));
            }
        }
        if search(
            registries,
            request,
            host,
            assigned,
            expanded,
            deepest,
            depth + 1,
        )? {
            return Ok(true);
        }
        assigned.remove(&candidate.name);
    }
    remember(
        deepest,
        Conflict {
            edge,
            triplet: host.to_string(),
            depth,
            rows: Vec::new(),
            pinned: false,
        },
    );
    Ok(false)
}

fn remember(deepest: &mut Option<Conflict>, conflict: Conflict) {
    let replace = match deepest {
        None => true,
        Some(current) => conflict.depth >= current.depth,
    };
    if replace {
        *deepest = Some(conflict);
    }
}

fn target_key(edge: &Edge) -> String {
    match (&edge.name, &edge.engine) {
        (Some(name), _) => name.clone(),
        (None, Some(engine)) => match &edge.implementation {
            Some(implementation) => format!("{engine}({implementation})"),
            None => engine.clone(),
        },
        (None, None) => String::new(),
    }
}

/// Every row the edge could consider, plus why each rejected row was
/// rejected (spec 42 §5's `excluded` column).
fn candidates(
    registries: &[&Registry],
    edge: &Edge,
    host: &Platform,
) -> (Vec<Candidate>, Vec<(String, Option<Exclusion>)>) {
    let constraint = tpkg::versions::parse_constraint(&edge.constraint)
        .expect("constraints are validated before the search");
    let want = edge.kind;
    let Some(name) = edge.name.as_deref() else {
        return (Vec::new(), Vec::new());
    };
    let mut live: Vec<Candidate> = Vec::new();
    let mut rows: Vec<(String, Option<Exclusion>)> = Vec::new();

    for registry in registries {
        let Some(payload) = registry.payload(name) else {
            continue;
        };
        if kind_of(payload.kind) != want {
            continue;
        }
        for version in &payload.versions {
            let Some(view) = version
                .resolve_variant(payload.default_variant.as_deref())
                .ok()
            else {
                rows.push((version.version.clone(), Some(Exclusion::Platform)));
                continue;
            };
            let exclusion = if version.is_withdrawn() {
                Some(Exclusion::Withdrawn)
            } else if !constraint.matches(&version.version) {
                Some(Exclusion::Constraint)
            } else if view.select(*host).is_none() {
                Some(Exclusion::Platform)
            } else {
                None
            };
            if exclusion.is_some() {
                rows.push((version.version.clone(), exclusion));
                continue;
            }
            let (triplet, sha256) = match view.select(*host) {
                Some(PlatformSelection::Selected { sha256, .. }) => {
                    (host.to_string(), sha256.to_string())
                }
                // A universal row's anchor lives in the release ref
                // (spec 04 §1); `-` is spec 41 §2's rendering for "not
                // carried here".
                Some(PlatformSelection::Universal) => ("universal".to_string(), "-".to_string()),
                None => unreachable!("exclusion covers the None arm"),
            };
            rows.push((version.version.clone(), None));
            live.push(Candidate {
                kind: want,
                name: payload.name.clone(),
                version: version.version.clone(),
                triplet,
                sha256,
                key: format!("{want} {}@{}", payload.name, version.version),
            });
        }
    }
    live.sort_by(|a, b| {
        tpkg::versions::compare(&b.version, &a.version)
            .then(a.name.cmp(&b.name))
            .then(a.sha256.cmp(&b.sha256))
    });
    live.dedup_by(|a, b| a.name == b.name && a.version == b.version && a.triplet == b.triplet);
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows.dedup_by(|a, b| a.0 == b.0);
    (live, rows)
}

fn kind_of(kind: tpkg::PayloadKind) -> EdgeKind {
    match kind {
        tpkg::PayloadKind::App => EdgeKind::App,
        tpkg::PayloadKind::Data => EdgeKind::Data,
        tpkg::PayloadKind::Toolkit => EdgeKind::Toolkit,
        tpkg::PayloadKind::Runtime => EdgeKind::Runtime,
        tpkg::PayloadKind::Language => EdgeKind::Language,
    }
}

/// Render a lockfile (spec 42 §6): a function of the assignment alone.
pub fn render_lock(lock: &Lock) -> Result<String, TebakoError> {
    serde_yml::to_string(lock)
        .map_err(|e| solve_error(format!("the lockfile could not be rendered: {e}")))
}

/// Parse a lockfile (spec 42 §6).
pub fn parse_lock(text: &str) -> Result<Lock, TebakoError> {
    let lock: Lock = serde_yml::from_str(text)
        .map_err(|e| solve_error(format!("the lockfile is not parseable: {e}")))?;
    if lock.lock_version != LOCK_VERSION {
        return Err(solve_error(format!(
            "lockfile lock_version {} is not {LOCK_VERSION} — re-solve without --locked and commit the new lock",
            lock.lock_version
        )));
    }
    Ok(lock)
}

/// `--locked`: refuse to change the assignment (spec 42 §6).
pub fn check_locked(existing: &Lock, solved: &Lock) -> Result<(), TebakoError> {
    if existing.assignments == solved.assignments {
        return Ok(());
    }
    let mut lines = String::new();
    for a in &existing.assignments {
        lines.push_str(&format!(
            "  locked: {} {} {} {}\n",
            a.kind, a.name, a.version, a.triplet
        ));
    }
    for a in &solved.assignments {
        lines.push_str(&format!(
            "  solve:  {} {} {} {}\n",
            a.kind, a.name, a.version, a.triplet
        ));
    }
    Err(solve_error(format!(
        "lock-drift: the solve disagrees with {LOCK_FILE} — run tebako solve without --locked and commit the new lock\n{lines}"
    )))
}

/// `tebako solve <request.yaml> --registry <path|ref>... [--lock <path>]
/// [--locked] [--write-lock <path>] [--json]` (spec 42 §6).
pub fn run(args: Vec<String>) -> Result<(), TebakoError> {
    let mut request_path: Option<String> = None;
    let mut registry_inputs: Vec<String> = Vec::new();
    let mut lock_path: Option<String> = None;
    let mut write_lock: Option<String> = None;
    let mut locked = false;
    let mut json = false;

    let mut rest = args.into_iter().peekable();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--registry" | "--lock" | "--write-lock" => {
                let Some(value) = rest.next() else {
                    return Err(solve_error(format!("{arg} needs a value")));
                };
                match arg.as_str() {
                    "--registry" => registry_inputs.push(value),
                    "--lock" => lock_path = Some(value),
                    _ => write_lock = Some(value),
                }
            }
            "--locked" => locked = true,
            "--json" => json = true,
            "--help" | "-h" => {
                return Err(solve_error(
                    "usage: tebako solve <request.yaml> --registry <path|ref> [--registry …] \
                     [--lock <path>] [--locked] [--write-lock <path>] [--json]",
                ));
            }
            other if other.starts_with("--") => {
                return Err(solve_error(format!(
                    "unknown option '{other}' (usage: tebako solve <request.yaml> --registry <path|ref> …)"
                )));
            }
            other => {
                if request_path.is_some() {
                    return Err(solve_error(
                        "tebako solve takes exactly one requirement set",
                    ));
                }
                request_path = Some(other.to_string());
            }
        }
    }

    let Some(request_path) = request_path else {
        return Err(solve_error(
            "usage: tebako solve <request.yaml> --registry <path|ref> [--registry …]",
        ));
    };
    if registry_inputs.is_empty() {
        return Err(solve_error(
            "solve needs at least one --registry (the candidate space is the published documents, never a network probe)",
        ));
    }

    let text = std::fs::read_to_string(&request_path).map_err(|e| {
        solve_error(format!(
            "the requirement set {request_path} is not readable: {e}"
        ))
    })?;
    let request = SolveRequest::from_yaml(&text)?;

    let mut registries: Vec<Registry> = Vec::new();
    for input in &registry_inputs {
        registries.push(crate::registry::load(input)?);
    }
    let refs: Vec<&Registry> = registries.iter().collect();

    let mut lock = solve(&refs, &request)?;
    let mut roots: Vec<String> = Vec::new();
    for registry in &registries {
        let lines = crate::registry_log::canonical_lines(registry)?;
        roots.push(crate::registry_log::hex(&crate::registry_log::tree_root(
            &lines,
        )));
    }
    roots.sort();
    roots.dedup();
    lock.registries = roots;

    if locked {
        let path = lock_path.clone().unwrap_or_else(|| LOCK_FILE.to_string());
        let existing = std::fs::read_to_string(&path).map_err(|e| {
            solve_error(format!(
                "--locked needs the lockfile {path}, which is not readable: {e}"
            ))
        })?;
        let existing = parse_lock(&existing)?;
        check_locked(&existing, &lock)?;
    }

    let rendered = render_lock(&lock)?;
    if let Some(path) = &write_lock {
        std::fs::write(path, &rendered)
            .map_err(|e| solve_error(format!("the lockfile {path} could not be written: {e}")))?;
    }

    if json {
        print!("{rendered}");
        return Ok(());
    }
    for assignment in &lock.assignments {
        println!(
            "{} {} {} {} {}",
            assignment.kind,
            assignment.name,
            assignment.version,
            assignment.triplet,
            assignment.sha256
        );
    }
    for skipped in &lock.skipped {
        println!(
            "skipped {} on {} ({})",
            skipped.edge, skipped.triplet, skipped.reason
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry(yaml: &str) -> Registry {
        Registry::from_yaml(yaml).expect("the fixture registry parses")
    }

    fn fixture() -> Registry {
        registry(
            r#"
schema_version: 1
payloads:
- name: iso-codes
  kind: data
  versions:
  - version: '2024.1'
    platforms:
      aarch64-macos:
        artifact: iso-codes-2024.1-macos-arm64.tfs
        sha256: '1111111111111111111111111111111111111111111111111111111111111111'
      x86_64-linux-gnu:
        artifact: iso-codes-2024.1-linux-gnu-x86_64.tfs
        sha256: '1111111111111111111111111111111111111111111111111111111111111111'
    release:
      ref: tfs:github:tebako-packages/fixture:2024.1
  - version: '2024.2'
    platforms:
      aarch64-macos:
        artifact: iso-codes-2024.2.tfs
        sha256: '2222222222222222222222222222222222222222222222222222222222222222'
      x86_64-linux-gnu:
        artifact: iso-codes-2024.2-linux.tfs
        sha256: '3333333333333333333333333333333333333333333333333333333333333333'
    release:
      ref: tfs:github:tebako-packages/fixture:2024.2
- name: gtk-layer
  kind: toolkit
  versions:
  - version: '3.24.1'
    platforms:
      aarch64-macos:
        artifact: gtk-3.24.1.tfs
        sha256: '4444444444444444444444444444444444444444444444444444444444444444'
    release:
      ref: tfs:github:tebako-packages/fixture:3.24.1
  - version: '3.25.0'
    platforms:
      aarch64-macos:
        artifact: gtk-3.25.0.tfs
        sha256: '5555555555555555555555555555555555555555555555555555555555555555'
    release:
      ref: tfs:github:tebako-packages/fixture:3.25.0
"#,
        )
    }

    fn universal_fixture() -> Registry {
        registry(
            r#"
schema_version: 1
payloads:
- name: iso-codes
  kind: data
  versions:
  - version: '2024.1'
    platforms: universal
    release:
      ref: tfs:github:tebako-packages/fixture:2024.1?sha256=1111111111111111111111111111111111111111111111111111111111111111
"#,
        )
    }

    fn request(requires: Vec<Edge>, closure: &[(&str, Vec<Edge>)]) -> SolveRequest {
        SolveRequest {
            triplet: "aarch64-macos".to_string(),
            requires,
            closure: closure
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        }
    }

    fn data_edge(name: &str, constraint: &str) -> Edge {
        edge(EdgeKind::Data, name, constraint)
    }

    fn toolkit_edge(name: &str, constraint: &str) -> Edge {
        edge(EdgeKind::Toolkit, name, constraint)
    }

    fn edge(kind: EdgeKind, name: &str, constraint: &str) -> Edge {
        Edge {
            kind,
            name: Some(name.to_string()),
            engine: None,
            implementation: None,
            constraint: constraint.to_string(),
            triplets: None,
        }
    }

    #[test]
    fn the_maximal_assignment_is_chosen() {
        let reg = fixture();
        let regs = [&reg];
        let solved = solve(
            &regs,
            &request(
                vec![data_edge("iso-codes", ">= 2024.1")],
                &[("data iso-codes@2024.2", Vec::new())],
            ),
        )
        .expect("the set solves");
        assert_eq!(solved.assignments.len(), 1);
        assert_eq!(solved.assignments[0].version, "2024.2");
        assert_eq!(solved.assignments[0].triplet, "aarch64-macos");
    }

    #[test]
    fn a_constraint_conflict_names_the_candidates_and_reasons() {
        let reg = fixture();
        let regs = [&reg];
        let err = solve(
            &regs,
            &request(vec![data_edge("iso-codes", ">= 2099.1")], &[]),
        )
        .expect_err("the set cannot solve");
        assert_eq!(err.code, EX_TEBAKO_SOLVE);
        assert!(err.message.contains("solve-conflict"), "{}", err.message);
        assert!(
            err.message.contains("excluded: 2024.1 — constraint"),
            "{}",
            err.message
        );
        assert!(
            err.message.contains("2024.2 — constraint"),
            "{}",
            err.message
        );
    }

    #[test]
    fn a_platform_gap_is_named_as_such() {
        let reg = fixture();
        let regs = [&reg];
        let mut req = request(vec![data_edge("iso-codes", "= 2024.2")], &[]);
        req.triplet = "x86_64-linux-musl".to_string();
        let err = solve(&regs, &req).expect_err("no musl row");
        assert!(err.message.contains("platform"), "{}", err.message);
    }

    #[test]
    fn one_version_per_name_is_a_pinned_conflict() {
        let reg = fixture();
        let regs = [&reg];
        let err = solve(
            &regs,
            &request(
                vec![
                    data_edge("iso-codes", ">= 2024.2"),
                    data_edge("iso-codes", "= 2024.1"),
                ],
                &[
                    ("data iso-codes@2024.1", Vec::new()),
                    ("data iso-codes@2024.2", Vec::new()),
                ],
            ),
        )
        .expect_err("two versions of one name");
        assert!(err.message.contains("pinned"), "{}", err.message);
    }

    #[test]
    fn a_skipped_edge_is_recorded_not_silent() {
        let reg = fixture();
        let regs = [&reg];
        let mut edge = data_edge("iso-codes", ">= 2024.1");
        edge.triplets = Some(vec!["x86_64-linux-gnu".to_string()]);
        let solved = solve(&regs, &request(vec![edge], &[])).expect("vacuous solve");
        assert!(solved.assignments.is_empty());
        assert_eq!(solved.skipped.len(), 1);
        assert_eq!(solved.skipped[0].reason, "triplet-not-listed");
    }

    #[test]
    fn the_closure_expands_and_a_missing_entry_is_named() {
        let reg = fixture();
        let regs = [&reg];
        let gtk = toolkit_edge("gtk-layer", ">= 3.24");
        let solved = solve(
            &regs,
            &request(
                vec![data_edge("iso-codes", ">= 2024.1")],
                &[
                    ("data iso-codes@2024.2", vec![gtk]),
                    ("toolkit gtk-layer@3.25.0", Vec::new()),
                ],
            ),
        )
        .expect("the closure solves");
        assert_eq!(solved.assignments.len(), 2);
        assert!(solved.assignments.iter().any(|a| a.name == "gtk-layer"));

        let missing = solve(
            &regs,
            &request(vec![data_edge("iso-codes", ">= 2024.1")], &[]),
        )
        .expect_err("no closure entry");
        assert!(
            missing.message.contains("solve-closure-missing"),
            "{}",
            missing.message
        );
    }

    #[test]
    fn withdrawn_rows_are_not_candidates() {
        let reg = registry(
            r#"
schema_version: 1
payloads:
- name: iso-codes
  kind: data
  versions:
  - version: '2024.2'
    status: withdrawn
    platforms:
      aarch64-macos:
        artifact: iso-codes-2024.2.tfs
        sha256: '2222222222222222222222222222222222222222222222222222222222222222'
    release:
      ref: tfs:github:tebako-packages/fixture:2024.2
"#,
        );
        let regs = [&reg];
        let err = solve(
            &regs,
            &request(vec![data_edge("iso-codes", ">= 2024.1")], &[]),
        )
        .expect_err("the only row is withdrawn");
        assert!(err.message.contains("withdrawn"), "{}", err.message);
    }

    #[test]
    fn the_lock_round_trips_byte_identically() {
        let reg = fixture();
        let regs = [&reg];
        let req = request(
            vec![data_edge("iso-codes", ">= 2024.1")],
            &[("data iso-codes@2024.2", Vec::new())],
        );
        let first = solve(&regs, &req).expect("solves");
        let rendered = render_lock(&first).expect("renders");
        let parsed = parse_lock(&rendered).expect("parses");
        assert_eq!(first, parsed);
        assert_eq!(render_lock(&parsed).expect("renders"), rendered);

        let again = solve(&regs, &req).expect("solves again");
        assert_eq!(render_lock(&again).expect("renders"), rendered);
    }

    #[test]
    fn locked_refuses_drift() {
        let reg = fixture();
        let regs = [&reg];
        let solved = solve(
            &regs,
            &request(
                vec![data_edge("iso-codes", ">= 2024.1")],
                &[("data iso-codes@2024.2", Vec::new())],
            ),
        )
        .expect("solves");
        let mut stale = solved.clone();
        stale.assignments[0].version = "2024.1".to_string();
        let err = check_locked(&stale, &solved).expect_err("drift");
        assert!(err.message.contains("lock-drift"), "{}", err.message);
        check_locked(&solved, &solved).expect("no drift");
    }

    #[test]
    fn a_universal_row_locks_with_its_own_triplet_and_dash_anchor() {
        let reg = universal_fixture();
        let regs = [&reg];
        let solved = solve(
            &regs,
            &request(
                vec![data_edge("iso-codes", ">= 2024.0")],
                &[("data iso-codes@2024.1", Vec::new())],
            ),
        )
        .expect("solves");
        assert_eq!(solved.assignments[0].triplet, "universal");
        assert_eq!(solved.assignments[0].sha256, "-");
    }

    #[test]
    fn registry_order_does_not_change_the_lock() {
        let one = fixture();
        let two = fixture();
        let a = [&one, &two];
        let b = [&two, &one];
        let req = request(
            vec![data_edge("iso-codes", ">= 2024.1")],
            &[("data iso-codes@2024.2", Vec::new())],
        );
        assert_eq!(
            render_lock(&solve(&a, &req).expect("solves")).expect("renders"),
            render_lock(&solve(&b, &req).expect("solves")).expect("renders")
        );
    }

    #[test]
    fn a_runtime_edge_is_refused_by_name() {
        let reg = fixture();
        let regs = [&reg];
        let edge = Edge {
            kind: EdgeKind::Language,
            name: None,
            engine: Some("ruby".to_string()),
            implementation: None,
            constraint: ">= 3.3, < 5.0".to_string(),
            triplets: None,
        };
        let err = solve(&regs, &request(vec![edge], &[])).expect_err("runtime edge");
        assert!(
            err.message.contains("solve-runtime-edge"),
            "{}",
            err.message
        );
    }

    #[test]
    fn a_toolkit_edge_resolves_without_a_closure_entry() {
        // A leaf candidate still needs its closure key: the solver never
        // assumes "no dependencies" (spec 42 §2).
        let reg = fixture();
        let regs = [&reg];
        let err = solve(
            &regs,
            &request(vec![toolkit_edge("gtk-layer", ">= 3.24")], &[]),
        )
        .expect_err("closure missing");
        assert!(
            err.message.contains("solve-closure-missing"),
            "{}",
            err.message
        );
        let solved = solve(
            &regs,
            &request(
                vec![toolkit_edge("gtk-layer", ">= 3.24")],
                &[("toolkit gtk-layer@3.25.0", Vec::new())],
            ),
        )
        .expect("solves with the leaf entry");
        assert_eq!(solved.assignments[0].version, "3.25.0");
    }
}
