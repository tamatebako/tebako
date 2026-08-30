//! Report-renderer tests (spec 27 slice 6): the merge rules (same-suite,
//! one file per triplet), the re-derived-never-carried statistics rule,
//! the v1-baseline speedup convention, named-gap rendering, and the
//! red-matrix exit-1 contract. All offline: result files are written to a
//! tempdir and the real report path renders them.

use std::path::Path;

use tebako_bench::report::{self, ReportRequest};
use tebako_bench::result::{
    ResultFile, RunMode, RunRecord, RunStatus, RunnerMeta, StatRecord, Versions,
};

fn ok_run(workload: &str, target: &str, mode: RunMode, iteration: u32, wall: f64) -> RunRecord {
    RunRecord {
        workload: workload.to_string(),
        target: target.to_string(),
        mode: Some(mode),
        iteration: Some(iteration),
        status: RunStatus::Ok,
        wall_s: Some(wall),
        cpu_user_s: Some(wall / 2.0),
        cpu_sys_s: Some(wall / 4.0),
        peak_rss_bytes: Some(512 * 1024 * 1024),
        exit: Some(0),
        error: None,
        reason: None,
    }
}

fn runner() -> RunnerMeta {
    RunnerMeta {
        runs_on: "ubuntu-24.04".to_string(),
        arch: "x86_64".to_string(),
        cpus: 4,
        ram_bytes: 16 * 1024 * 1024 * 1024,
    }
}

/// A result file whose CARRIED stats are garbage on purpose: the report
/// must re-derive from the run records, so an emitted 999.0 anywhere in
/// the markdown or dashboard is a test failure.
fn triplet_file(triplet: &str, runs: Vec<RunRecord>) -> ResultFile {
    ResultFile {
        schema_version: 1,
        suite: "metanorma-v1-vs-v2".to_string(),
        triplet: triplet.to_string(),
        runner: runner(),
        versions: Versions {
            tebako: Some("0.3.0".to_string()),
            runtime: Some("0.16.11-3.3.12".to_string()),
            payload: Some("1.16.9-3".to_string()),
            packed_mn: Some("v1.14.4 (metanorma-cli 1.14.4)".to_string()),
            image_format: None,
        },
        stats: vec![StatRecord {
            workload: "compile-small-iso".to_string(),
            target: "v1-packed-mn".to_string(),
            mode: RunMode::Warm,
            n: 99,
            median_wall_s: 999.0,
            min_wall_s: 999.0,
            max_wall_s: 999.0,
            stdev_wall_s: Some(999.0),
            mean_wall_s: 999.0,
            median_cpu_s: 999.0,
            median_peak_rss_bytes: 999,
        }],
        runs,
    }
}

fn write_result(dir: &Path, name: &str, file: &ResultFile) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, file.to_json().unwrap()).unwrap();
    p
}

fn run_report(dir: &Path, results: &[std::path::PathBuf]) -> (Result<u8, String>, String, String) {
    let md = dir.join("report.md");
    let json = dir.join("dashboard.json");
    let rc = report::report(&ReportRequest {
        results: results.to_vec(),
        md: md.clone(),
        json: json.clone(),
    })
    .map_err(|e| e.message);
    let md_text = std::fs::read_to_string(&md).unwrap_or_default();
    let json_text = std::fs::read_to_string(&json).unwrap_or_default();
    (rc, md_text, json_text)
}

/// The standard two-triplet fixture: v1 walls 9/10/11 (median 10),
/// v2-fat 4/5/6 (median 5 → 2.00×), v2-shim 2/2.5/3 (median 2.5 → 4.00×).
fn standard_runs() -> Vec<RunRecord> {
    let mut v = Vec::new();
    for (i, w) in [9.0, 10.0, 11.0].iter().enumerate() {
        v.push(ok_run("compile-small-iso", "v1-packed-mn", RunMode::Warm, i as u32 + 1, *w));
    }
    for (i, w) in [4.0, 5.0, 6.0].iter().enumerate() {
        v.push(ok_run("compile-small-iso", "v2-fat", RunMode::Warm, i as u32 + 1, *w));
    }
    for (i, w) in [2.0, 2.5, 3.0].iter().enumerate() {
        v.push(ok_run("compile-small-iso", "v2-shim", RunMode::Warm, i as u32 + 1, *w));
    }
    v.push(ok_run("compile-small-iso", "v1-packed-mn", RunMode::Cold, 1, 20.0));
    v.push(ok_run("compile-small-iso", "v2-shim", RunMode::Cold, 1, 8.0));
    v
}

#[test]
fn merges_triplets_re_deriving_stats_and_speedups() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_result(dir.path(), "a.json", &triplet_file("linux-gnu-x86_64", standard_runs()));
    let b = write_result(dir.path(), "b.json", &triplet_file("macos-arm64", standard_runs()));
    let (rc, md, json) = run_report(dir.path(), &[a, b]);
    assert_eq!(rc, Ok(0));
    // re-derived, never carried: the 999.0 garbage stats must not appear
    assert!(!md.contains("999"), "carried stats leaked into the report");
    assert!(md.contains("# tebako benchmark report — metanorma-v1-vs-v2"));
    assert!(md.contains("## linux-gnu-x86_64"));
    assert!(md.contains("## macos-arm64"));
    // warm medians: v1 = 10.000, v2-fat = 5.000 at 2.00×, v2-shim 2.500 at 4.00×
    assert!(md.contains("| v1-packed-mn | 3 | 10.000 |"), "{md}");
    assert!(md.contains("2.00×"), "{md}");
    assert!(md.contains("4.00×"), "{md}");
    // cold reported separately
    assert!(md.contains("cold (install/first-boot)"), "{md}");
    // the dashboard parses and carries the re-derived median + speedup
    let dash: serde_json::Value = serde_json::from_str(&json).unwrap();
    let cell = dash["triplets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["target"] == "v2-fat" && c["mode"] == "warm")
        .unwrap()
        .clone();
    assert_eq!(cell["median_wall_s"], 5.0);
    assert_eq!(cell["speedup_vs_v1"], 2.0);
}

#[test]
fn suite_mismatch_is_operational() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_result(dir.path(), "a.json", &triplet_file("linux-gnu-x86_64", standard_runs()));
    let mut other = triplet_file("macos-arm64", standard_runs());
    other.suite = "another-suite".to_string();
    let b = write_result(dir.path(), "b.json", &other);
    let (rc, _, _) = run_report(dir.path(), &[a, b]);
    let msg = rc.expect_err("mismatched suites must refuse to merge");
    assert!(msg.contains("suite"), "{msg}");
}

#[test]
fn duplicate_triplet_is_operational() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_result(dir.path(), "a.json", &triplet_file("linux-gnu-x86_64", standard_runs()));
    let b = write_result(dir.path(), "b.json", &triplet_file("linux-gnu-x86_64", standard_runs()));
    let (rc, _, _) = run_report(dir.path(), &[a, b]);
    let msg = rc.expect_err("one result file per triplet");
    assert!(msg.contains("linux-gnu-x86_64"), "{msg}");
}

#[test]
fn every_arm_unavailable_writes_artifacts_and_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let gap = RunRecord {
        workload: "compile-small-iso".to_string(),
        target: "v1-packed-mn".to_string(),
        mode: None,
        iteration: None,
        status: RunStatus::Unavailable,
        wall_s: None,
        cpu_user_s: None,
        cpu_sys_s: None,
        peak_rss_bytes: None,
        exit: None,
        error: None,
        reason: Some("no packed-mn asset for this triplet".to_string()),
    };
    let a = write_result(dir.path(), "a.json", &triplet_file("linux-gnu-x86_64", vec![gap]));
    let (rc, md, json) = run_report(dir.path(), &[a]);
    assert_eq!(rc, Ok(1), "a red matrix is a deliverable, exit 1");
    assert!(md.contains("unavailable — no packed-mn asset for this triplet"), "{md}");
    assert!(json.contains("no packed-mn asset"), "{json}");
}

#[test]
fn missing_v1_arm_renders_dash_never_an_invented_ratio() {
    let dir = tempfile::tempdir().unwrap();
    let runs = vec![ok_run("compile-small-iso", "v2-shim", RunMode::Warm, 1, 2.5)];
    let a = write_result(dir.path(), "a.json", &triplet_file("linux-gnu-x86_64", runs));
    let (rc, md, _) = run_report(dir.path(), &[a]);
    assert_eq!(rc, Ok(0));
    assert!(md.contains("| v2-shim | 1 | 2.500 |"), "{md}");
    let row = md.lines().find(|l| l.contains("v2-shim")).unwrap();
    assert!(row.ends_with("| — |"), "{row}");
}

#[test]
fn semantically_invalid_input_is_operational() {
    let dir = tempfile::tempdir().unwrap();
    let mut bad = ok_run("compile-small-iso", "v2-shim", RunMode::Warm, 1, 2.5);
    bad.wall_s = None; // an ok run without its wall violates the §6 rules
    let a = write_result(dir.path(), "a.json", &triplet_file("linux-gnu-x86_64", vec![bad]));
    let (rc, _, _) = run_report(dir.path(), &[a]);
    let msg = rc.expect_err("invalid input must be an operational error");
    assert!(msg.contains("wall_s"), "{msg}");
}
