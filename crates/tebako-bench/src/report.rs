//! Slice 6 — the report renderer (spec 27 §7): merge N per-triplet result
//! files into one markdown report + one site-ingestible dashboard JSON.
//!
//! The merge laws (§7):
//!
//! - one result file per triplet; every file must name the SAME suite —
//!   violations are operational errors, never silent picks;
//! - statistics are RE-DERIVED from the merged run records via the
//!   engine's own `compute_stats`; a file's carried stats are ignored, so
//!   a hand-edited record cannot smuggle a stale stat past the report;
//! - speedups are always "vs the baseline arm on the same triplet ×
//!   workload × mode" (spec 27 §2/§7, amended 2026-09-17): the suite's
//!   declared baseline flows through the result documents
//!   (`baseline: {target, label}` — every merged file must agree, a
//!   disagreement is an operational error); with no declared baseline
//!   the law stays "the cell whose target id starts with `v1`". A cell
//!   with no baseline arm renders "—", never an invented ratio;
//! - a merge whose every arm failed or was unavailable still writes both
//!   artifacts and exits 1 — a red matrix is a deliverable, not a crash
//!   (§8).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::engine::compute_stats;
use crate::error::BenchError;
use crate::exit;
use crate::result::{Baseline, ResultFile, RunMode, RunRecord, RunStatus, StatRecord};

pub struct ReportRequest {
    pub results: Vec<PathBuf>,
    pub md: PathBuf,
    pub json: PathBuf,
}

/// One input file with its stats re-derived from its own run records.
struct TripletReport {
    file: ResultFile,
    stats: Vec<StatRecord>,
}

/// The dashboard document (§7: "site-ingestible"; no schema gate — the
/// shape is pinned by tests/report.rs snapshots instead). Deserializable:
/// `tebako-bench trend` reads the dashboards back in.
#[derive(Serialize, Deserialize)]
pub struct Dashboard {
    pub suite: String,
    pub generated_by: String,
    /// The declared baseline mirror (additive; absent when the suite
    /// declares none — the `speedup_vs_v1` cell field name never
    /// changes either way).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub baseline: Option<Baseline>,
    /// The method notes (§7, amended): the cold semantics of both arm
    /// kinds, the teardown construction, the on-system cold gap —
    /// present when the suite declares a non-v1 baseline.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub notes: Vec<String>,
    pub triplets: Vec<DashboardTriplet>,
}

#[derive(Serialize, Deserialize)]
pub struct DashboardTriplet {
    pub triplet: String,
    pub runner: crate::result::RunnerMeta,
    pub versions: crate::result::Versions,
    pub cells: Vec<DashboardCell>,
    pub unavailable: Vec<DashboardRow>,
    pub failed: Vec<DashboardRow>,
}

#[derive(Serialize, Deserialize)]
pub struct DashboardCell {
    pub workload: String,
    pub target: String,
    pub mode: RunMode,
    pub n: u32,
    pub median_wall_s: f64,
    pub min_wall_s: f64,
    pub max_wall_s: f64,
    pub stdev_wall_s: Option<f64>,
    pub mean_wall_s: f64,
    pub median_cpu_s: f64,
    pub median_peak_rss_bytes: u64,
    /// baseline median / this median on the same triplet × workload ×
    /// mode; null when the cell has no baseline arm (the named-gap
    /// shape). The field NAME is frozen (additive-only law); the
    /// baseline it divides by comes from the dashboard's `baseline`.
    pub speedup_vs_v1: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub struct DashboardRow {
    pub workload: String,
    pub target: String,
    pub mode: Option<RunMode>,
    pub iteration: Option<u32>,
    pub status: RunStatus,
    pub error: Option<String>,
    pub reason: Option<String>,
}

/// The §10 method notes, stated on every artifact of a suite with a
/// declared non-v1 baseline (§7, amended 2026-09-17).
fn method_notes() -> Vec<String> {
    vec![
        "Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span."
            .to_string(),
        "Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal."
            .to_string(),
        "On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison."
            .to_string(),
    ]
}

pub fn report(req: &ReportRequest) -> Result<u8, BenchError> {
    let mut triplets: Vec<TripletReport> = Vec::new();
    let mut suite: Option<String> = None;
    let mut baseline: Option<Option<Baseline>> = None;
    for path in &req.results {
        let text = std::fs::read_to_string(path)
            .map_err(|e| BenchError::operational(format!("cannot read {}: {e}", path.display())))?;
        let file = ResultFile::from_json(&text).map_err(|e| {
            BenchError::operational(format!("cannot parse {}: {e}", path.display()))
        })?;
        let violations = file.semantic_violations();
        if !violations.is_empty() {
            return Err(BenchError::operational(format!(
                "{}: invalid result document: {}",
                path.display(),
                violations.join("; ")
            )));
        }
        match &suite {
            None => suite = Some(file.suite.clone()),
            Some(s) if *s == file.suite => {}
            Some(s) => {
                return Err(BenchError::operational(format!(
                    "report merges one suite only: {} is '{}', expected '{s}'",
                    path.display(),
                    file.suite
                )))
            }
        }
        // The declared baseline flows through every result file; the
        // merge refuses mixed baselines (never a silent pick).
        let file_baseline = file.baseline.clone();
        match &baseline {
            None => baseline = Some(file_baseline),
            Some(b) if *b == file_baseline => {}
            Some(_) => {
                return Err(BenchError::operational(format!(
                    "report merges one baseline only: {} disagrees with an earlier file",
                    path.display()
                )))
            }
        }
        if triplets
            .iter()
            .any(|t: &TripletReport| t.file.triplet == file.triplet)
        {
            return Err(BenchError::operational(format!(
                "one result file per triplet: '{}' given twice ({})",
                file.triplet,
                path.display()
            )));
        }
        let stats = compute_stats(&file.runs);
        triplets.push(TripletReport { file, stats });
    }
    triplets.sort_by(|a, b| a.file.triplet.cmp(&b.file.triplet));
    let suite = suite.unwrap_or_default();
    let baseline = baseline.unwrap_or_default();
    // The method notes ride the declared non-v1 baseline (§7, amended).
    let notes = match &baseline {
        Some(b) if !b.target.starts_with("v1") => method_notes(),
        _ => Vec::new(),
    };
    let label = baseline
        .as_ref()
        .map(|b| b.label.clone())
        .unwrap_or_else(|| "v1".to_string());

    let md = render_markdown(&suite, &triplets, &label, &notes);
    std::fs::write(&req.md, md)
        .map_err(|e| BenchError::operational(format!("cannot write {}: {e}", req.md.display())))?;
    let dash = dashboard(&suite, &triplets, baseline, notes);
    let json = serde_json::to_string_pretty(&dash)
        .map_err(|e| BenchError::operational(format!("dashboard serialize: {e}")))?;
    std::fs::write(&req.json, json).map_err(|e| {
        BenchError::operational(format!("cannot write {}: {e}", req.json.display()))
    })?;

    let any_ok = triplets
        .iter()
        .any(|t| t.file.runs.iter().any(|r| r.status == RunStatus::Ok));
    if any_ok {
        Ok(exit::OK)
    } else {
        eprintln!("tebako-bench: every arm failed or was unavailable — the red matrix is written [benchmark]");
        Ok(exit::INVALID)
    }
}

/// The baseline arm of a (workload × mode) cell: the declared baseline
/// names its target exactly or by prefix (the runtime suite's
/// `on-system` prefix-matches `on-system-ruby` within the ruby
/// workloads); with no declared baseline the law stays "starts with
/// `v1`" (spec 27 §2/§7).
fn baseline<'a>(
    stats: &'a [StatRecord],
    s: &StatRecord,
    declared: Option<&str>,
) -> Option<&'a StatRecord> {
    let name = declared.unwrap_or("v1");
    stats
        .iter()
        .find(|b| b.workload == s.workload && b.mode == s.mode && b.target == name)
        .or_else(|| {
            stats.iter().find(|b| {
                b.workload == s.workload && b.mode == s.mode && b.target.starts_with(name)
            })
        })
}

fn speedup(stats: &[StatRecord], s: &StatRecord, declared: Option<&str>) -> Option<f64> {
    baseline(stats, s, declared).map(|b| b.median_wall_s / s.median_wall_s)
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn render_markdown(
    suite: &str,
    triplets: &[TripletReport],
    label: &str,
    notes: &[String],
) -> String {
    let mut out = format!("# tebako benchmark report — {suite}\n");
    let baseline_name = triplets
        .first()
        .and_then(|t| t.file.baseline.as_ref())
        .map(|b| b.target.clone());
    for t in triplets {
        let f = &t.file;
        out.push_str(&format!("\n## {}\n", f.triplet));
        out.push_str(&format!(
            "\nRunner: {} · {} · {} cpus · {:.1} GiB\n",
            f.runner.runs_on,
            f.runner.arch,
            f.runner.cpus,
            mib(f.runner.ram_bytes) * 1024.0 / 1024.0
        ));
        let mut versions = Vec::new();
        if let Some(v) = &f.versions.tebako {
            versions.push(format!("tebako {v}"));
        }
        if let Some(v) = &f.versions.runtime {
            versions.push(format!("runtime {v}"));
        }
        if let Some(v) = &f.versions.payload {
            versions.push(format!("payload {v}"));
        }
        if let Some(v) = &f.versions.packed_mn {
            versions.push(format!("packed-mn {v}"));
        }
        if let Some(v) = &f.versions.image_format {
            versions.push(format!("image {}", format!("{v:?}").to_lowercase()));
        }
        if !versions.is_empty() {
            out.push_str(&format!("Versions: {}\n", versions.join(" · ")));
        }

        // (workload, mode) sections in deterministic order, warm before cold.
        let mut sections: BTreeMap<(String, RunMode), Vec<&StatRecord>> = BTreeMap::new();
        for s in &t.stats {
            sections
                .entry((s.workload.clone(), s.mode))
                .or_default()
                .push(s);
        }
        for ((workload, mode), mut cells) in sections {
            cells.sort_by(|a, b| a.target.cmp(&b.target));
            let mode_label = match mode {
                RunMode::Warm => "warm".to_string(),
                RunMode::Cold => "cold (install/first-boot)".to_string(),
            };
            out.push_str(&format!("\n### {workload} — {mode_label}\n\n"));
            out.push_str(&format!("| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs {label} |\n"));
            out.push_str("|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
            for s in cells {
                let stdev = s
                    .stdev_wall_s
                    .map(|v| format!("{v:.3}"))
                    .unwrap_or_else(|| "—".to_string());
                let ratio = speedup(&t.stats, s, baseline_name.as_deref())
                    .map(|v| format!("{v:.2}×"))
                    .unwrap_or_else(|| "—".to_string());
                out.push_str(&format!(
                    "| {} | {} | {:.3} | {:.3} | {:.3} | {} | {:.3} | {:.3} | {:.1} | {} |\n",
                    s.target,
                    s.n,
                    s.median_wall_s,
                    s.min_wall_s,
                    s.max_wall_s,
                    stdev,
                    s.mean_wall_s,
                    s.median_cpu_s,
                    mib(s.median_peak_rss_bytes),
                    ratio
                ));
            }
        }

        let gaps: Vec<&RunRecord> = f
            .runs
            .iter()
            .filter(|r| r.status == RunStatus::Unavailable)
            .collect();
        if !gaps.is_empty() {
            out.push_str("\nUnavailable arms:\n\n");
            for r in gaps {
                out.push_str(&format!(
                    "- {} / {}{}: unavailable — {}\n",
                    r.workload,
                    r.target,
                    mode_suffix(r),
                    r.reason.as_deref().unwrap_or("(no reason given)")
                ));
            }
        }
        let failed: Vec<&RunRecord> = f
            .runs
            .iter()
            .filter(|r| matches!(r.status, RunStatus::Failed | RunStatus::Timeout))
            .collect();
        if !failed.is_empty() {
            out.push_str("\nFailed runs:\n\n");
            for r in failed {
                let status = match r.status {
                    RunStatus::Failed => "failed",
                    RunStatus::Timeout => "timeout",
                    _ => unreachable!(),
                };
                let detail = r
                    .error
                    .as_deref()
                    .map(|e| format!(" — {e}"))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "- {} / {}{} #{}{}: {status}{}\n",
                    r.workload,
                    r.target,
                    mode_suffix(r),
                    r.iteration.unwrap_or(0),
                    r.exit.map(|c| format!(" (exit {c})")).unwrap_or_default(),
                    detail
                ));
            }
        }
    }

    out.push_str("\n---\n\nRunner metadata:");
    for t in triplets {
        let f = &t.file;
        out.push_str(&format!(
            " {} = {} / {} / {} cpus / {:.1} GiB;",
            f.triplet,
            f.runner.runs_on,
            f.runner.arch,
            f.runner.cpus,
            mib(f.runner.ram_bytes) * 1024.0 / 1024.0
        ));
    }
    out.push_str(
        "\n\nGitHub-hosted runners are shared, multi-tenant machines; treat differences under \
         ~10% as noise and read min alongside median — min is the cross-noise-comparable figure \
         (noise inflates, never deflates).\n\n\
         Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 \
         payload is current — compare ratios, not absolutes. Numbers across image formats are \
         never mixed.\n\n\
         Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory \
         pressure, so a tebako arm's RSS delta overstates its real memory cost.\n",
    );
    if !notes.is_empty() {
        out.push_str("\nMethod notes:\n\n");
        for note in notes {
            out.push_str(&format!("- {note}\n"));
        }
    }
    out
}

fn mode_suffix(r: &RunRecord) -> String {
    r.mode
        .map(|m| match m {
            RunMode::Warm => " [warm]",
            RunMode::Cold => " [cold]",
        })
        .unwrap_or_default()
        .to_string()
}

fn dashboard(
    suite: &str,
    triplets: &[TripletReport],
    baseline: Option<Baseline>,
    notes: Vec<String>,
) -> Dashboard {
    let baseline_name = baseline.as_ref().map(|b| b.target.clone());
    Dashboard {
        suite: suite.to_string(),
        generated_by: "tebako-bench report".to_string(),
        baseline,
        notes,
        triplets: triplets
            .iter()
            .map(|t| {
                let cells = t
                    .stats
                    .iter()
                    .map(|s| DashboardCell {
                        workload: s.workload.clone(),
                        target: s.target.clone(),
                        mode: s.mode,
                        n: s.n,
                        median_wall_s: s.median_wall_s,
                        min_wall_s: s.min_wall_s,
                        max_wall_s: s.max_wall_s,
                        stdev_wall_s: s.stdev_wall_s,
                        mean_wall_s: s.mean_wall_s,
                        median_cpu_s: s.median_cpu_s,
                        median_peak_rss_bytes: s.median_peak_rss_bytes,
                        speedup_vs_v1: speedup(&t.stats, s, baseline_name.as_deref()),
                    })
                    .collect();
                let row = |r: &RunRecord| DashboardRow {
                    workload: r.workload.clone(),
                    target: r.target.clone(),
                    mode: r.mode,
                    iteration: r.iteration,
                    status: r.status,
                    error: r.error.clone(),
                    reason: r.reason.clone(),
                };
                DashboardTriplet {
                    triplet: t.file.triplet.clone(),
                    runner: t.file.runner.clone(),
                    versions: t.file.versions.clone(),
                    cells,
                    unavailable: t
                        .file
                        .runs
                        .iter()
                        .filter(|r| r.status == RunStatus::Unavailable)
                        .map(row)
                        .collect(),
                    failed: t
                        .file
                        .runs
                        .iter()
                        .filter(|r| matches!(r.status, RunStatus::Failed | RunStatus::Timeout))
                        .map(row)
                        .collect(),
                }
            })
            .collect(),
    }
}
