//! The release-over-release trend feed (spec 27 §11): one flat cell per
//! measured dashboard cell of a run, appended over the previous
//! `latest/trend.json`, deduplicated per full key with the new row
//! winning, capped at the last 20 releases.
//!
//! `ratio_vs_baseline` is THIS median ÷ the baseline median on the same
//! triplet × workload × mode — the OVERHEAD shape: 1.00 = parity, >1 =
//! tebako overhead, <1 = tebako faster. (The dashboard cell's
//! `speedup_vs_v1` is its reciprocal; the website derives from the
//! medians, so the two never disagree.) A cell with no baseline arm
//! carries `ratio_vs_baseline: null`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::BenchError;
use crate::exit;
use crate::report::Dashboard;
use crate::result::RunMode;

pub struct TrendRequest {
    /// The release name this run publishes under (`v2.8.8` or
    /// `run-<id>` for dispatches).
    pub release: String,
    /// The run date, YYYY-MM-DD (authored by the workflow).
    pub date: String,
    /// The run's dashboards (both suites).
    pub dashboards: Vec<PathBuf>,
    /// The previous latest/trend.json, when it exists.
    pub previous: Option<PathBuf>,
    /// The trend.json destination.
    pub out: PathBuf,
}

/// The trend document: flat cells, newest release last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendFile {
    pub generated_by: String,
    pub cells: Vec<TrendCell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendCell {
    pub release: String,
    pub date: String,
    pub suite: String,
    pub triplet: String,
    pub workload: String,
    pub target: String,
    pub mode: RunMode,
    pub median_wall_s: f64,
    pub ratio_vs_baseline: Option<f64>,
}

/// The dedupe key (spec 27 §11: release × suite × triplet × workload ×
/// target × mode — the new row wins).
type Key = (String, String, String, String, String, RunMode);

fn key(c: &TrendCell) -> Key {
    (
        c.release.clone(),
        c.suite.clone(),
        c.triplet.clone(),
        c.workload.clone(),
        c.target.clone(),
        c.mode,
    )
}

/// The baseline median of a (triplet × workload × mode) cell inside one
/// dashboard: the declared baseline target/prefix, else the v1 law.
fn baseline_median(dash: &Dashboard, triplet: &str, workload: &str, mode: RunMode) -> Option<f64> {
    let name = dash
        .baseline
        .as_ref()
        .map(|b| b.target.as_str())
        .unwrap_or("v1");
    let t = dash.triplets.iter().find(|t| t.triplet == triplet)?;
    t.cells
        .iter()
        .find(|c| c.workload == workload && c.mode == mode && c.target == name)
        .or_else(|| {
            t.cells
                .iter()
                .find(|c| c.workload == workload && c.mode == mode && c.target.starts_with(name))
        })
        .map(|c| c.median_wall_s)
}

/// Merge the previous trend with this run's cells: the new rows win per
/// key, and only the last `max_releases` distinct releases (in order of
/// first appearance) survive the cap.
pub fn merge(previous: Vec<TrendCell>, new: Vec<TrendCell>, max_releases: usize) -> Vec<TrendCell> {
    let mut order: Vec<Key> = Vec::new();
    let mut by_key: BTreeMap<Key, TrendCell> = BTreeMap::new();
    for c in previous.into_iter().chain(new) {
        let k = key(&c);
        if !by_key.contains_key(&k) {
            order.push(k.clone());
        }
        by_key.insert(k, c);
    }
    // Distinct releases in order of first appearance; keep the last N.
    let mut releases: Vec<String> = Vec::new();
    for k in &order {
        if !releases.contains(&k.0) {
            releases.push(k.0.clone());
        }
    }
    let skip = releases.len().saturating_sub(max_releases);
    let keep: Vec<String> = releases.into_iter().skip(skip).collect();
    order
        .into_iter()
        .filter(|k| keep.contains(&k.0))
        .filter_map(|k| by_key.get(&k).cloned())
        .collect()
}

pub fn trend(req: &TrendRequest) -> Result<u8, BenchError> {
    let mut new_cells: Vec<TrendCell> = Vec::new();
    for path in &req.dashboards {
        let text = std::fs::read_to_string(path).map_err(|e| {
            BenchError::operational(format!("trend: cannot read {}: {e}", path.display()))
        })?;
        let dash: Dashboard = serde_json::from_str(&text).map_err(|e| {
            BenchError::operational(format!("trend: cannot parse {}: {e}", path.display()))
        })?;
        for t in &dash.triplets {
            for c in &t.cells {
                let ratio = baseline_median(&dash, &t.triplet, &c.workload, c.mode)
                    .map(|b| c.median_wall_s / b);
                new_cells.push(TrendCell {
                    release: req.release.clone(),
                    date: req.date.clone(),
                    suite: dash.suite.clone(),
                    triplet: t.triplet.clone(),
                    workload: c.workload.clone(),
                    target: c.target.clone(),
                    mode: c.mode,
                    median_wall_s: c.median_wall_s,
                    ratio_vs_baseline: ratio,
                });
            }
        }
    }

    let previous = match &req.previous {
        None => Vec::new(),
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| {
                BenchError::operational(format!("trend: cannot read {}: {e}", path.display()))
            })?;
            let file: TrendFile = serde_json::from_str(&text).map_err(|e| {
                BenchError::operational(format!("trend: cannot parse {}: {e}", path.display()))
            })?;
            file.cells
        }
    };

    let file = TrendFile {
        generated_by: "tebako-bench trend".to_string(),
        cells: merge(previous, new_cells, 20),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| BenchError::operational(format!("trend: serialize: {e}")))?;
    std::fs::write(&req.out, format!("{json}\n")).map_err(|e| {
        BenchError::operational(format!("trend: cannot write {}: {e}", req.out.display()))
    })?;
    Ok(exit::OK)
}
