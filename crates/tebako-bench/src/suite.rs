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
    /// The arm the report's speedup column is computed against: an exact
    /// target id, or a PREFIX resolving to exactly one target per workload
    /// (the runtime suite's `on-system` pairs `on-system-<lang>` per
    /// workload). Absent = the v1-prefix law (spec 27 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
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
    /// The workload document. Absent for interpreter workloads (the runtime
    /// suite, spec 27 §10): the run's scratch cell is empty and `{doc}` is
    /// a validation error there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Substitutions: `{doc}` — the workload document's path in the run
    /// scratch (requires `source`); `{fixture}` — the ioread data file,
    /// resolved per target (in-image for runtime-exe, host path otherwise);
    /// `{classes}` — the in-leg compiled Java classes directory. Every
    /// other token is literal.
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
    /// on-system: the host toolchain binary, PATH-resolved (the workflow
    /// provisions it — spec 27 §10.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    /// on-system: the argv that prints the toolchain's version (the parity
    /// probe runs the tebako arm with the SAME argv).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_probe: Option<Vec<String>>,
    /// on-system: the version token BOTH arms' probe output must contain
    /// (the fair-comparison invariant; a mismatch is a named skip).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_expect: Option<String>,
    /// runtime-exe: the factory release the tebako runtime pair is fetched
    /// from, sha256-verified against the release's per-asset sidecars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeRef>,
    /// on-system java: the vendored .java directory compiled ONCE in-leg
    /// with the on-system javac; both JVMs run the same .class files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_classes: Option<String>,
}

/// The factory release coordinates of a tebako runtime pair (runtime-exe
/// targets): repo `owner/name`, the release `tag`, and the interpreter
/// version the pair carries (the asset names embed both versions).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRef {
    pub repo: String,
    pub tag: String,
    pub lang_version: String,
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
    /// The host toolchain binary (CI-provisioned, version-pinned to the
    /// tebako runtime under test — spec 27 §10.1).
    OnSystem,
    /// The tebako runtime pair (exe + env image) booted bare, fetched
    /// sha256-verified from its factory release (spec 27 §10.1).
    RuntimeExe,
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
            if w.argv.iter().any(|a| a == "{doc}") && w.source.is_none() {
                violations.push(format!(
                    "workloads/{}/argv: '{{doc}}' requires a source — interpreter workloads have no document",
                    w.id
                ));
            }
            if let Some(source) = &w.source {
                if source.kind == SourceKind::Git {
                    if source.url.is_none() {
                        violations
                            .push(format!("workloads/{}/source: a git source needs url", w.id));
                    }
                    match &source.git_ref {
                        Some(r)
                            if r.len() == 40
                                && r.bytes().all(|b| b.is_ascii_digit()
                                    || (b'a'..=b'f').contains(&b)) => {}
                        other => violations.push(format!(
                            "workloads/{}/source/ref: '{}' is not a pinned 40-hex commit — floating refs are a named error",
                            w.id,
                            other.as_deref().unwrap_or("<missing>")
                        )),
                    }
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for t in &self.targets {
            if !seen.insert(&t.id) {
                violations.push(format!("targets: duplicate id '{}'", t.id));
            }
            match t.kind {
                TargetKind::V2Managed | TargetKind::V2Press => {
                    if t.payload.is_none() {
                        violations.push(format!(
                            "targets/{}: a v2 target needs payload (name@version)",
                            t.id
                        ));
                    }
                    if t.registries.is_none() {
                        violations.push(format!(
                            "targets/{}: a v2 target needs registries (registry references)",
                            t.id
                        ));
                    }
                }
                TargetKind::OnSystem => {
                    for (field, present) in [
                        ("program", t.program.is_some()),
                        ("version_probe", t.version_probe.is_some()),
                        ("version_expect", t.version_expect.is_some()),
                    ] {
                        if !present {
                            violations.push(format!(
                                "targets/{}: an on-system target needs {field} (the parity law, spec 27 §10.1)",
                                t.id
                            ));
                        }
                    }
                }
                TargetKind::RuntimeExe => {
                    if t.runtime.is_none() {
                        violations.push(format!(
                            "targets/{}: a runtime-exe target needs runtime (repo, tag, lang_version)",
                            t.id
                        ));
                    }
                }
                TargetKind::V1Exe => {}
            }
        }
        if let Some(baseline) = &self.baseline {
            let exact = self.targets.iter().any(|t| &t.id == baseline);
            let prefix = self
                .targets
                .iter()
                .filter(|t| t.id.starts_with(baseline.as_str()))
                .count();
            if !exact && prefix == 0 {
                violations.push(format!(
                    "baseline: '{baseline}' names no target, neither exactly nor as a prefix"
                ));
            }
        }
        // The runtime suite's pairing law (spec 27 §10.1): every on-system
        // arm pairs with a tebako arm of the same language suffix.
        for t in &self.targets {
            let Some((prefix, lang)) = pair_suffix(&t.id) else {
                continue;
            };
            let counterpart = format!(
                "{}-{lang}",
                if prefix == "on-system" {
                    "tebako"
                } else {
                    "on-system"
                }
            );
            if !self.targets.iter().any(|o| o.id == counterpart) {
                violations.push(format!(
                    "targets/{}: the arm pairs with '{counterpart}', which the suite does not declare",
                    t.id
                ));
            }
        }
        violations
    }
}

/// The runtime-suite arm pairing (spec 27 §10.1): `on-system-<lang>` and
/// `tebako-<lang>` are one comparison pair. Returns (prefix, lang).
pub fn pair_suffix(target_id: &str) -> Option<(&'static str, &str)> {
    for prefix in ["on-system-", "tebako-"] {
        if let Some(lang) = target_id.strip_prefix(prefix) {
            return Some((
                if prefix == "on-system-" {
                    "on-system"
                } else {
                    "tebako"
                },
                lang,
            ));
        }
    }
    None
}
