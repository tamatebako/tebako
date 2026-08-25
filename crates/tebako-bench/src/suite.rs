//! The suite document model (spec 27 §2) — authored YAML, the WHAT-runs
//! SSOT. Structurally gated by `schema/tebako-bench-suite-v1.schema.json`;
//! this serde model is the same shape (the MECE cross-check lives in
//! tests/validate.rs). Unknown keys are tolerated (forward compatibility);
//! `schema_version` pins the version.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuiteFile {
    pub schema_version: u32,
    pub name: String,
    pub workloads: Vec<Workload>,
    pub targets: Vec<Target>,
    pub run_policy: RunPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workload {
    pub id: String,
    /// Opt-in workloads (private-fonts/credentialed environments) run only
    /// on explicit request; skipped ones emit NO result rows (spec 27 §2).
    #[serde(default)]
    pub opt_in: bool,
    pub source: Source,
    /// `{doc}` is the one substitution — the workload document's path in the
    /// run scratch; every other token is literal.
    pub argv: Vec<String>,
    pub expect: Expect,
    pub timeout_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub kind: SourceKind,
    /// vendored: repo-relative file path. git: the document's path inside
    /// the pinned tree.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The pinned 40-hex commit (the YAML key is `ref`; floating refs are
    /// invalid). Fetched as the host's in-process HTTPS archive — never a
    /// git shell-out.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Vendored,
    Git,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expect {
    /// Expected process exit status (default 0).
    #[serde(default)]
    pub exit: i32,
    /// Scratch-relative outputs that must exist and be non-empty.
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub id: String,
    pub kind: TargetKind,
    /// `name@version` — the registry payload reference (v2 kinds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// spec 04 registry references the v2 arms resolve through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registries: Option<Vec<String>>,
    /// v2-press explicitness flag: the package carries the runtime as a
    /// slot (the one-file contract).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fat: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    /// The packed-mn release executable (asset named by platforms.yaml).
    V1Exe,
    /// Registry install + shim dispatch, warm store — v2's primary form.
    V2Managed,
    /// A fat tpkg assembled in-leg from verified published artifacts.
    V2Press,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunPolicy {
    /// Unmeasured priming runs per (workload × target).
    pub warmup: u32,
    /// Measured warm runs per (workload × target).
    pub repetitions: u32,
    /// Measured cold runs, each preceded by the spec 27 §5 cache wipe;
    /// reported separately, never mixed into warm statistics.
    pub cold_repetitions: u32,
    /// true: rotate targets per iteration (drift decorrelation). false:
    /// each target to completion in turn (debugging only).
    pub interleave: bool,
}

impl SuiteFile {
    pub fn from_yaml(text: &str) -> Result<Self, serde_yml::Error> {
        serde_yml::from_str(text)
    }

    /// Semantic checks (spec 27 §8's second half): the cross-field rules the
    /// schema cannot express (id uniqueness), plus the model-side restatement
    /// of the kind-conditional requirements so the two gates agree (MECE —
    /// tests/validate.rs asserts exactly that).
    pub fn semantic_violations(&self) -> Vec<String> {
        let mut violations = Vec::new();
        if self.schema_version != 1 {
            violations.push(format!(
                "schema_version: expected 1, got {}",
                self.schema_version
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for w in &self.workloads {
            if !seen.insert(&w.id) {
                violations.push(format!("workloads: duplicate id '{}'", w.id));
            }
            if w.source.kind == SourceKind::Git {
                if w.source.url.is_none() {
                    violations.push(format!("workloads/{}/source: a git source needs url", w.id));
                }
                match &w.source.git_ref {
                    Some(r)
                        if r.len() == 40
                            && r.bytes().all(|b| b.is_ascii_digit()
                                || (b'a'..=b'f').contains(&b)) => {}
                    other => violations.push(format!(
                        "workloads/{}/source/ref: '{}' is not a pinned 40-hex commit — floating refs are a named error (spec 27 §2)",
                        w.id,
                        other.as_deref().unwrap_or("<missing>")
                    )),
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for t in &self.targets {
            if !seen.insert(&t.id) {
                violations.push(format!("targets: duplicate id '{}'", t.id));
            }
            if matches!(t.kind, TargetKind::V2Managed | TargetKind::V2Press) {
                if t.payload.is_none() {
                    violations.push(format!(
                        "targets/{}: a v2 target needs payload (name@version)",
                        t.id
                    ));
                }
                if t.registries.is_none() {
                    violations.push(format!(
                        "targets/{}: a v2 target needs registries (spec 04 references)",
                        t.id
                    ));
                }
            }
        }
        violations
    }
}
