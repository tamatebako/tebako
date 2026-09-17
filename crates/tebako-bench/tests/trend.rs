//! Trend-feed tests (spec 27 §11): the dedupe-new-wins rule, the 20-release
//! cap, the OVERHEAD ratio convention (this median ÷ the baseline median;
//! 1.00 = parity; the dashboard's speedup_vs_v1 is its reciprocal), and the
//! previous-file append path. All offline: dashboards are handcrafted JSON.

use tebako_bench::result::RunMode;
use tebako_bench::trend::{self, TrendCell, TrendFile, TrendRequest};

fn cell(release: &str, suite: &str, target: &str, median: f64, ratio: Option<f64>) -> TrendCell {
    TrendCell {
        release: release.to_string(),
        date: "2026-09-17".to_string(),
        suite: suite.to_string(),
        triplet: "macos-arm64".to_string(),
        workload: "w".to_string(),
        target: target.to_string(),
        mode: RunMode::Warm,
        median_wall_s: median,
        ratio_vs_baseline: ratio,
    }
}

#[test]
fn merge_dedupes_new_wins_and_preserves_order() {
    let previous = vec![
        cell("v1", "s", "a", 1.0, Some(1.0)),
        cell("v1", "s", "b", 2.0, Some(2.0)),
    ];
    let new = vec![cell("v1", "s", "b", 9.0, Some(9.0))];
    let merged = trend::merge(previous, new, 20);
    assert_eq!(merged.len(), 2, "the new row replaces, never duplicates");
    assert_eq!(merged[0].target, "a", "order of first appearance survives");
    assert_eq!(merged[1].median_wall_s, 9.0, "the new row wins");
}

#[test]
fn merge_caps_at_the_last_20_releases() {
    let mut previous = Vec::new();
    for i in 0..25 {
        previous.push(cell(&format!("v{i}"), "s", "a", 1.0, Some(1.0)));
    }
    let merged = trend::merge(previous, vec![cell("v25", "s", "a", 1.0, None)], 20);
    let releases: Vec<&str> = merged.iter().map(|c| c.release.as_str()).collect();
    assert_eq!(releases.len(), 20);
    assert_eq!(releases.first().copied(), Some("v6"), "the oldest fall off");
    assert_eq!(releases.last().copied(), Some("v25"), "the newest land");
}

/// A two-cell dashboard: on-system-ruby median 2.0 (the declared
/// baseline's prefix arm) and tebako-ruby median 5.0 → ratio 2.5.
fn dashboard_json(with_baseline: bool) -> String {
    let baseline = if with_baseline {
        r#""baseline": {"target": "on-system", "label": "on-system"},"#
    } else {
        ""
    };
    format!(
        r#"{{"suite": "runtime-on-system-vs-tebako", "generated_by": "tebako-bench report",
{baseline}"triplets": [{{"triplet": "macos-arm64",
  "runner": {{"runs_on": "macos-14", "arch": "aarch64", "cpus": 4, "ram_bytes": 1}},
  "versions": {{}},
  "cells": [
    {{"workload": "ruby-boot", "target": "on-system-ruby", "mode": "warm", "n": 5,
      "median_wall_s": 2.0, "min_wall_s": 2.0, "max_wall_s": 2.0, "stdev_wall_s": null,
      "mean_wall_s": 2.0, "median_cpu_s": 2.0, "median_peak_rss_bytes": 1,
      "speedup_vs_v1": 1.0}},
    {{"workload": "ruby-boot", "target": "tebako-ruby", "mode": "warm", "n": 5,
      "median_wall_s": 5.0, "min_wall_s": 5.0, "max_wall_s": 5.0, "stdev_wall_s": null,
      "mean_wall_s": 5.0, "median_cpu_s": 5.0, "median_peak_rss_bytes": 1,
      "speedup_vs_v1": 0.4}}
  ], "unavailable": [], "failed": []}}]}}"#
    )
}

fn run_trend(
    dir: &std::path::Path,
    dashboards: &[std::path::PathBuf],
    previous: Option<std::path::PathBuf>,
) -> TrendFile {
    let out = dir.join("trend.json");
    let rc = trend::trend(&TrendRequest {
        release: "v9.9.9".to_string(),
        date: "2026-09-17".to_string(),
        dashboards: dashboards.to_vec(),
        previous,
        out: out.clone(),
    })
    .unwrap();
    assert_eq!(rc, 0);
    let text = std::fs::read_to_string(&out).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn trend_emits_the_overhead_ratio_from_the_declared_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let dash = dir.path().join("dashboard.json");
    std::fs::write(&dash, dashboard_json(true)).unwrap();
    let file = run_trend(dir.path(), &[dash], None);
    assert_eq!(file.generated_by, "tebako-bench trend");
    assert_eq!(file.cells.len(), 2);
    let base = file
        .cells
        .iter()
        .find(|c| c.target == "on-system-ruby")
        .unwrap();
    assert_eq!(base.ratio_vs_baseline, Some(1.0), "the baseline cell is parity");
    let tb = file.cells.iter().find(|c| c.target == "tebako-ruby").unwrap();
    assert_eq!(
        tb.ratio_vs_baseline,
        Some(2.5),
        "this median ÷ the baseline median — the OVERHEAD shape"
    );
    assert_eq!(tb.release, "v9.9.9");
    assert_eq!(tb.suite, "runtime-on-system-vs-tebako");
}

#[test]
fn a_cell_with_no_baseline_arm_carries_null() {
    let dir = tempfile::tempdir().unwrap();
    let dash = dir.path().join("dashboard.json");
    // No declared baseline → the v1 law; no v1 arm in this dashboard.
    std::fs::write(&dash, dashboard_json(false)).unwrap();
    let file = run_trend(dir.path(), &[dash], None);
    assert!(
        file.cells
            .iter()
            .all(|c| c.ratio_vs_baseline.is_none()),
        "no baseline arm → null, never an invented ratio"
    );
}

#[test]
fn trend_appends_over_the_previous_feed() {
    let dir = tempfile::tempdir().unwrap();
    let dash = dir.path().join("dashboard.json");
    std::fs::write(&dash, dashboard_json(true)).unwrap();
    let previous = dir.path().join("previous.json");
    let prev = TrendFile {
        generated_by: "tebako-bench trend".to_string(),
        cells: vec![cell("v9.9.8", "runtime-on-system-vs-tebako", "tebako-ruby", 6.0, Some(3.0))],
    };
    std::fs::write(&previous, serde_json::to_string(&prev).unwrap()).unwrap();
    let file = run_trend(dir.path(), &[dash], Some(previous));
    assert_eq!(file.cells.len(), 3, "the previous cells survive the append");
    assert_eq!(file.cells[0].release, "v9.9.8", "oldest first");
}
