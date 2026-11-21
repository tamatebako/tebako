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
    /// What one run of this workload does (spec 27 §2, amended 2026-11-21):
    /// `exec` (the default) runs the target's measured program with the
    /// workload argv; `press` runs the v2-tools arm's staged `tebako` CLI
    /// with the workload argv (the author loop's re-press — `{package}`
    /// substitutes the run cell's package output path).
    #[serde(default)]
    pub kind: WorkloadKind,
    /// The explicit arm allowlist (spec 27 §10.2, amended 2026-11-21):
    /// when present it REPLACES the id-prefix routing law — the workload
    /// runs on exactly these target ids (each must exist — a semantic
    /// error otherwise). Absent, the prefix law routes as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
    /// The workload document. Absent for interpreter workloads (the runtime
    /// suite, spec 27 §10): the run's scratch cell is empty and `{doc}` is
    /// a validation error there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Substitutions: `{doc}` — the workload document's path in the run
    /// scratch (requires `source`); `{fixture}` — the ioread data file,
    /// resolved per target (in-image for runtime-exe, host path otherwise);
    /// `{classes}` — the in-leg compiled Java classes directory;
    /// `{package}` — the press-kind run cell's package output path
    /// (2026-11-21). May be EMPTY on an exec workload (the target's
    /// program runs bare — the pressed package's first run); a press
    /// workload's argv is never empty (the press args are the workload).
    /// Every other token is literal.
    pub argv: Vec<String>,
    pub expect: Expect,
    pub timeout_s: u64,
}

/// What one run of a workload measures (spec 27 §2, amended 2026-11-21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkloadKind {
    /// Run the target's measured program with the workload argv.
    #[default]
    Exec,
    /// Run the v2-tools arm's staged `tebako` CLI with the workload argv
    /// (a `tebako press` of the workload's source project; `{package}`
    /// substitutes the run cell's output path). Press workloads route to
    /// v2-tools targets only.
    Press,
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
/// `implementation` is spec 28 §8's flavor axis: factories whose engine
/// has flavors (openjdk ships temurin + graalvm) spell the asset stem
/// with the implementation, not the repo-derived engine —
/// `tebako-runtime-2.8.50-temurin-21.0.12-<triplet>`. Absent, the engine
/// segment derives from the repo basename as before.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRef {
    pub repo: String,
    pub tag: String,
    pub lang_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
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
    /// The same pair staged as a spec 39 LAZY_SEEDING entry (spec 27
    /// §10.5): the exe present, the env image absent, the seed
    /// descriptor's source naming the harness's in-process loopback
    /// Range fixture. The driver state-detects the seeding entry — no
    /// opt-in env rides the measured run.
    RuntimeExeLazy,
    /// Whole-image extraction, timed like any other target (spec 27
    /// §10.6, added 2026-11-21): the arm's measured program is the
    /// release's `tfs` tool and every measured run is
    /// `tfs extract -q -d <run cell> <the runtime ref's env image>`,
    /// whatever the workload's argv says (the argv is ignored BY KIND
    /// CONTRACT — the extraction is the workload). Carries `runtime`
    /// exactly like runtime-exe; the image is shared with the paired
    /// runtime-exe arm's acquisition, never double-downloaded.
    TfsExtract,
    /// The author loop's tooling arm (spec 27 §11.1, added 2026-11-21):
    /// the release's `tebako` CLI stages a priming press of the
    /// target's one press workload at acquisition (the runtime download
    /// lands there, outside every measured span), and the pressed
    /// package is the arm's measured program for its exec workloads.
    /// Carries `registries` — the press resolves the default runtime
    /// pair through them (tebako#895's chain). No cold story: a cold
    /// press would re-resolve the runtime (a download) inside the
    /// measured span and the matrix is offline — the cold cell is a
    /// named gap.
    V2Tools,
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
                TargetKind::RuntimeExe | TargetKind::RuntimeExeLazy | TargetKind::TfsExtract => {
                    if t.runtime.is_none() {
                        violations.push(format!(
                            "targets/{}: a {} target needs runtime (repo, tag, lang_version)",
                            t.id,
                            match t.kind {
                                TargetKind::RuntimeExe => "runtime-exe",
                                TargetKind::RuntimeExeLazy => "runtime-exe-lazy",
                                _ => "tfs-extract",
                            }
                        ));
                    }
                }
                TargetKind::V2Tools => {
                    if t.registries.is_none() {
                        violations.push(format!(
                            "targets/{}: a v2-tools target needs registries (the press resolves the default runtime pair through them)",
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
        // The workload-level rules that reach across both lists
        // (2026-11-21): the explicit arm allowlist and the press kind.
        for w in &self.workloads {
            if let Some(allow) = &w.targets {
                if allow.is_empty() {
                    violations.push(format!(
                        "workloads/{}/targets: an empty allowlist routes nowhere — a workload that runs on no arm produces nothing",
                        w.id
                    ));
                }
                for id in allow {
                    if !self.targets.iter().any(|t| &t.id == id) {
                        violations.push(format!(
                            "workloads/{}/targets: '{id}' names no declared target",
                            w.id
                        ));
                    }
                }
            }
            if w.kind == WorkloadKind::Press {
                if w.source.is_none() {
                    violations.push(format!(
                        "workloads/{}: a press workload needs a source (the project it re-presses)",
                        w.id
                    ));
                }
                if w.targets.is_none() {
                    violations.push(format!(
                        "workloads/{}/targets: a press workload needs an explicit allowlist naming its v2-tools arm",
                        w.id
                    ));
                }
                if w.argv.is_empty() {
                    violations.push(format!(
                        "workloads/{}: a press workload's argv carries the press arguments — empty presses nothing",
                        w.id
                    ));
                }
                for id in w.targets.iter().flatten() {
                    match self.targets.iter().find(|t| &t.id == id) {
                        Some(t) if t.kind == TargetKind::V2Tools => {}
                        Some(t) => violations.push(format!(
                            "workloads/{}/targets: a press workload runs on v2-tools arms only — '{id}' is {}",
                            w.id,
                            match t.kind {
                                TargetKind::V1Exe => "v1-exe",
                                TargetKind::V2Managed => "v2-managed",
                                TargetKind::V2Press => "v2-press",
                                TargetKind::OnSystem => "on-system",
                                TargetKind::RuntimeExe => "runtime-exe",
                                TargetKind::RuntimeExeLazy => "runtime-exe-lazy",
                                TargetKind::TfsExtract => "tfs-extract",
                                TargetKind::V2Tools => unreachable!(),
                            }
                        )),
                        None => {} // the unknown id is named above
                    }
                }
            }
        }
        // A v2-tools arm presses exactly one project at acquisition (its
        // package serves the arm's exec workloads) — two press workloads
        // routing to the same arm is an ambiguity, named.
        for t in &self.targets {
            if t.kind != TargetKind::V2Tools {
                continue;
            }
            let presses = self
                .workloads
                .iter()
                .filter(|w| {
                    w.kind == WorkloadKind::Press
                        && w.targets.as_ref().map(|a| a.contains(&t.id)).unwrap_or(false)
                })
                .count();
            if presses != 1 {
                violations.push(format!(
                    "targets/{}: a v2-tools arm needs exactly one press workload routing to it, found {presses}",
                    t.id
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

/// The workload→arm routing key (spec 27 §10.2): the id's prefix before
/// the first '-' names the language pair the workload measures.
pub fn workload_lang(workload_id: &str) -> &str {
    match workload_id.split_once('-') {
        Some((prefix, _)) => prefix,
        None => workload_id,
    }
}
