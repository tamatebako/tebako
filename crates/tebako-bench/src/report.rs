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
//! - speedups are always "vs the v1 arm on the same triplet × workload ×
//!   mode". The report receives no suite file, so the v1 baseline is the
//!   cell whose target id starts with `v1` (the authored suite's v1-exe
//!   target is `v1-packed-mn`); a cell with no v1 arm renders "—", never
//!   an invented ratio;
//! - a merge whose every arm failed or was unavailable still writes both
//!   artifacts and exits 1 — a red matrix is a deliverable, not a crash
//!   (§8).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::engine::compute_stats;
use crate::error::BenchError;
use crate::exit;
use crate::result::{ResultFile, RunMode, RunRecord, RunStatus, StatRecord};

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
/// shape is pinned by tests/report.rs snapshots instead).
#[derive(Serialize)]
struct Dashboard {
    suite: String,
    generated_by: &'static str,
    triplets: Vec<DashboardTriplet>,
}

#[derive(Serialize)]
struct DashboardTriplet {
    triplet: String,
    runner: crate::result::RunnerMeta,
    versions: crate::result::Versions,
    cells: Vec<DashboardCell>,
    unavailable: Vec<DashboardRow>,
    failed: Vec<DashboardRow>,
}

#[derive(Serialize)]
struct DashboardCell {
    workload: String,
    target: String,
    mode: RunMode,
    n: u32,
    median_wall_s: f64,
    min_wall_s: f64,
    max_wall_s: f64,
    stdev_wall_s: Option<f64>,
    mean_wall_s: f64,
    median_cpu_s: f64,
    median_peak_rss_bytes: u64,
    /// v1 median / this median on the same triplet × workload × mode;
    /// null when the cell has no v1 arm (the named-gap shape).
    speedup_vs_v1: Option<f64>,
}

#[derive(Serialize)]
struct DashboardRow {
    workload: String,
    target: String,
    mode: Option<RunMode>,
    iteration: Option<u32>,
    status: RunStatus,
    error: Option<String>,
    reason: Option<String>,
}

pub fn report(req: &ReportRequest) -> Result<u8, BenchError> {
    let mut triplets: Vec<TripletReport> = Vec::new();
    let mut suite: Option<String> = None;
    for path in &req.results {
        let text = std::fs::read_to_string(path).map_err(|e| {
            BenchError::operational(format!("cannot read {}: {e}", path.display()))
        })?;
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
        if triplets.iter().any(|t: &TripletReport| t.file.triplet == file.triplet) {
            return Err(BenchError::operational(format!(
                "one result file per triplet (§7): '{}' given twice ({})",
                file.triplet,
                path.display()
            )));
        }
        let stats = compute_stats(&file.runs);
        triplets.push(TripletReport { file, stats });
    }
    triplets.sort_by(|a, b| a.file.triplet.cmp(&b.file.triplet));
    let suite = suite.unwrap_or_default();

    let md = render_markdown(&suite, &triplets);
    std::fs::write(&req.md, md).map_err(|e| {
        BenchError::operational(format!("cannot write {}: {e}", req.md.display()))
    })?;
    let dash = dashboard(&suite, &triplets);
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

/// The v1 baseline of a (workload × mode) cell: the row whose target id
/// starts with "v1" (see the module doc for the convention).
fn baseline<'a>(stats: &'a [StatRecord], s: &StatRecord) -> Option<&'a StatRecord> {
    stats
        .iter()
        .find(|b| b.workload == s.workload && b.mode == s.mode && b.target.starts_with("v1"))
}

fn speedup(stats: &[StatRecord], s: &StatRecord) -> Option<f64> {
    baseline(stats, s).map(|b| b.median_wall_s / s.median_wall_s)
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn render_markdown(suite: &str, triplets: &[TripletReport]) -> String {
    let mut out = format!("# tebako benchmark report — {suite}\n");
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
            out.push_str("| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |\n");
            out.push_str("|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
            for s in cells {
                let stdev = s
                    .stdev_wall_s
                    .map(|v| format!("{v:.3}"))
                    .unwrap_or_else(|| "—".to_string());
                let ratio = speedup(&t.stats, s)
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
         never mixed (spec 27 §1, §7).\n",
    );
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

fn dashboard(suite: &str, triplets: &[TripletReport]) -> Dashboard {
    Dashboard {
        suite: suite.to_string(),
        generated_by: "tebako-bench report (spec 27 §7)",
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
                        speedup_vs_v1: speedup(&t.stats, s),
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
