//! The result document model (spec 27 §6) — one machine-written JSON file
//! per triplet per run, merged by `report`. Structurally gated by
//! `schema/tebako-bench-result-v1.schema.json`; this serde model is the
//! same shape (MECE cross-check in tests/validate.rs).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultFile {
    pub schema_version: u32,
    /// The suite's `name` — report merges only same-suite results.
    pub suite: String,
    /// The release workflow's triplet spelling.
    pub triplet: String,
    /// Mandatory environment metadata — numbers without their environment
    /// are not numbers.
    pub runner: RunnerMeta,
    /// What actually ran (resolved versions, never requested ones). Fields
    /// may be absent when no arm of that world ran on the triplet.
    pub versions: Versions,
    /// One record per attempted run AND one per named gap.
    pub runs: Vec<RunRecord>,
    /// Per (workload × target × mode) statistics over `Ok` runs only.
    pub stats: Vec<StatRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerMeta {
    /// The CI runner label (e.g. ubuntu-24.04, macos-14, windows-latest).
    pub runs_on: String,
    pub arch: String,
    pub cpus: u32,
    pub ram_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Versions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tebako: Option<String>,
    /// The RESOLVED runtime (e.g. 0.16.9-3.3.12), never the requested line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    /// The resolved payload release (e.g. 1.16.9-3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// e.g. "v1.14.4 (metanorma-cli 1.14.4)" — the version-skew record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packed_mn: Option<String>,
    /// The v2 arms' image backend; numbers across formats never mix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_format: Option<ImageFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Dwarfs,
    Limnifs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub workload: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<RunMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteration: Option<u32>,
    pub status: RunStatus,
    /// Instant-elapsed seconds around spawn→wait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_s: Option<f64>,
    /// POSIX getrusage(RUSAGE_CHILDREN) delta / Windows GetProcessTimes
    /// (spec 27 §4 — one measured child at a time, the delta is exact).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_user_s: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_sys_s: Option<f64>,
    /// Bytes, ALWAYS — ru_maxrss is KiB on Linux/musl and bytes on macOS;
    /// the sampler normalizes at record time. Windows: PeakWorkingSetSize.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak_rss_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<i32>,
    /// failed runs: names the missed expectation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// unavailable rows: the mandatory human-readable gap reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunMode {
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    /// Expectation met; all metric fields present.
    Ok,
    /// Ran, expectation missed — never enters statistics.
    Failed,
    /// Killed at the workload's timeout_s.
    Timeout,
    /// The named-gap record: never attempted on this triplet; `reason`
    /// mandatory. Gaps are explicit data (invariant 9).
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatRecord {
    pub workload: String,
    pub target: String,
    pub mode: RunMode,
    /// Count of status-Ok runs in this cell.
    pub n: u32,
    /// The headline figure.
    pub median_wall_s: f64,
    /// The cross-noise-comparable figure (noise inflates, never deflates).
    pub min_wall_s: f64,
    pub max_wall_s: f64,
    /// Sample standard deviation (n−1); absent when n < 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdev_wall_s: Option<f64>,
    pub mean_wall_s: f64,
    /// Median of (cpu_user_s + cpu_sys_s).
    pub median_cpu_s: f64,
    pub median_peak_rss_bytes: u64,
}

impl ResultFile {
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Semantic checks beyond the schema's structural gate: the per-status
    /// field requirements restated model-side (the run engine produces these
    /// records; the check protects hand-written or merged result files), the
    /// stats-cell uniqueness the schema cannot express.
    pub fn semantic_violations(&self) -> Vec<String> {
        let mut violations = Vec::new();
        if self.schema_version != 1 {
            violations.push(format!(
                "schema_version: expected 1, got {}",
                self.schema_version
            ));
        }
        for (i, r) in self.runs.iter().enumerate() {
            let at = format!("runs/{i}");
            match r.status {
                RunStatus::Ok => {
                    for (name, present) in [
                        ("mode", r.mode.is_some()),
                        ("iteration", r.iteration.is_some()),
                        ("wall_s", r.wall_s.is_some()),
                        ("cpu_user_s", r.cpu_user_s.is_some()),
                        ("cpu_sys_s", r.cpu_sys_s.is_some()),
                        ("peak_rss_bytes", r.peak_rss_bytes.is_some()),
                        ("exit", r.exit.is_some()),
                    ] {
                        if !present {
                            violations.push(format!("{at}: an ok run carries {name} (spec 27 §6)"));
                        }
                    }
                }
                RunStatus::Failed => {
                    for (name, present) in [
                        ("mode", r.mode.is_some()),
                        ("iteration", r.iteration.is_some()),
                        ("exit", r.exit.is_some()),
                        ("error", r.error.is_some()),
                    ] {
                        if !present {
                            violations.push(format!("{at}: a failed run carries {name}"));
                        }
                    }
                }
                RunStatus::Timeout => {
                    for (name, present) in [
                        ("mode", r.mode.is_some()),
                        ("iteration", r.iteration.is_some()),
                        ("wall_s", r.wall_s.is_some()),
                    ] {
                        if !present {
                            violations.push(format!("{at}: a timeout run carries {name}"));
                        }
                    }
                }
                RunStatus::Unavailable => {
                    if r.reason.is_none() {
                        violations.push(format!(
                            "{at}: an unavailable row carries reason — the named gap is never silent (invariant 9)"
                        ));
                    }
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for s in &self.stats {
            if !seen.insert((&s.workload, &s.target, s.mode)) {
                violations.push(format!(
                    "stats: duplicate cell ({}, {}, {:?}) — one row per workload × target × mode",
                    s.workload, s.target, s.mode
                ));
            }
        }
        violations
    }
}
