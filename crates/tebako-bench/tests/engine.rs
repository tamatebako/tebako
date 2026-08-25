//! Run-engine tests (spec 27 slices 5): the pure ordering/statistics/
//! expectation rules, and the matrix loop end-to-end against the
//! `tebako-bench-child` golden child — acquisition is injected as
//! prepared targets (Ready with the child bin as the program), so the
//! whole file is offline. The emitted results.json re-enters BOTH
//! validation gates (the engine's own emit path does this; the tests
//! assert it stays true).

use std::path::{Path, PathBuf};

use tebako_bench::engine::{self, Prepared, PreparedTarget};
use tebako_bench::platforms::{PackedMn, PlatformFile, Triplet};
use tebako_bench::result::{ResultFile, RunMode, RunRecord, RunStatus, RunnerMeta, Versions};
use tebako_bench::suite::{
    Expect, RunPolicy, Source, SourceKind, SuiteFile, Target, TargetKind, Workload,
};
use tebako_bench::{acquire::BenchLayout, validate, DocKind};

/// The test-helper binary's path (the same idiom as tests/sampler.rs:
/// CARGO_BIN_FILE_<name> at run time, the debug-profile path otherwise).
fn child_path() -> PathBuf {
    if let Some(p) = std::env::var_os("CARGO_BIN_FILE_tebako-bench-child") {
        return PathBuf::from(p);
    }
    let name = if cfg!(windows) {
        "tebako-bench-child.exe"
    } else {
        "tebako-bench-child"
    };
    let dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target")));
    dir.join("debug").join(name)
}

/// A one-workload suite whose vendored source lives in `repo_root` (the
/// caller writes `fixtures/doc.adoc` there).
fn test_suite(argv: &[&str], files: &[&str], policy: RunPolicy) -> SuiteFile {
    SuiteFile {
        schema_version: 1,
        name: "engine-test".to_string(),
        workloads: vec![Workload {
            id: "w-small".to_string(),
            opt_in: false,
            source: Source {
                kind: SourceKind::Vendored,
                path: "fixtures/doc.adoc".to_string(),
                url: None,
                git_ref: None,
            },
            argv: argv.iter().map(|s| s.to_string()).collect(),
            expect: Expect {
                exit: 0,
                files: files.iter().map(|s| s.to_string()).collect(),
            },
            timeout_s: 30,
        }],
        targets: Vec::new(),
        run_policy: policy,
    }
}

fn repo_root_with_doc(dir: &Path) -> PathBuf {
    let fixtures = dir.join("fixtures");
    std::fs::create_dir_all(&fixtures).unwrap();
    std::fs::write(fixtures.join("doc.adoc"), b"= Title\n\nbody\n").unwrap();
    dir.to_path_buf()
}

fn ready(id: &str, kind: TargetKind) -> PreparedTarget {
    PreparedTarget {
        target: Target {
            id: id.to_string(),
            kind,
            payload: None,
            registries: None,
            fat: None,
        },
        state: Prepared::Ready {
            program: child_path(),
        },
    }
}

fn unavailable(id: &str, kind: TargetKind, reason: &str) -> PreparedTarget {
    PreparedTarget {
        target: Target {
            id: id.to_string(),
            kind,
            payload: None,
            registries: None,
            fat: None,
        },
        state: Prepared::Unavailable {
            reason: reason.to_string(),
        },
    }
}

fn noop_reprime(_: &BenchLayout, _: &Target) -> Result<(), tebako_bench::BenchError> {
    Ok(())
}

fn test_platforms(triplet: &str) -> PlatformFile {
    PlatformFile {
        schema_version: 1,
        packed_mn: PackedMn {
            repo: "metanorma/packed-mn".to_string(),
            tag: "v1.14.4".to_string(),
        },
        triplets: [(
            triplet.to_string(),
            Triplet {
                runner: "test-runner".to_string(),
                container: None,
                v1_asset: None,
                v2_payload: false,
                v1_note: None,
            },
        )]
        .into_iter()
        .collect(),
    }
}

// ---------------------------------------------------------------------
// the pure rules
// ---------------------------------------------------------------------

#[test]
fn warm_schedule_rotates_targets_per_iteration() {
    assert_eq!(
        engine::warm_schedule(3, 2, true),
        vec![(0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        "interleaved: A/B/C per iteration so drift decorrelates"
    );
    assert_eq!(
        engine::warm_schedule(3, 2, false),
        vec![(0, 1), (0, 2), (1, 1), (1, 2), (2, 1), (2, 2)],
        "non-interleaved: each target to completion in turn (debugging)"
    );
    assert!(engine::warm_schedule(2, 0, true).is_empty());
}

fn ok_run(w: &str, t: &str, mode: RunMode, wall: f64, cpu: f64, rss: u64) -> RunRecord {
    RunRecord {
        workload: w.to_string(),
        target: t.to_string(),
        mode: Some(mode),
        iteration: Some(1),
        status: RunStatus::Ok,
        wall_s: Some(wall),
        cpu_user_s: Some(cpu / 2.0),
        cpu_sys_s: Some(cpu / 2.0),
        peak_rss_bytes: Some(rss),
        exit: Some(0),
        error: None,
        reason: None,
    }
}

#[test]
fn stats_math_matches_the_spec_rules() {
    let mut runs = vec![
        ok_run("w", "t", RunMode::Warm, 14.0, 12.0, 300),
        ok_run("w", "t", RunMode::Warm, 10.0, 8.0, 100),
        ok_run("w", "t", RunMode::Warm, 12.0, 10.0, 200),
        ok_run("w", "t", RunMode::Cold, 30.0, 20.0, 500),
        // failed + timeout runs never enter statistics
        RunRecord {
            status: RunStatus::Failed,
            wall_s: Some(999.0),
            error: Some("missed".to_string()),
            ..ok_run("w", "t", RunMode::Warm, 999.0, 999.0, 999)
        },
        RunRecord {
            status: RunStatus::Timeout,
            wall_s: Some(60.0),
            cpu_user_s: None,
            cpu_sys_s: None,
            peak_rss_bytes: None,
            exit: None,
            ..ok_run("w", "t", RunMode::Warm, 60.0, 0.0, 0)
        },
    ];
    let stats = engine::compute_stats(&runs);
    assert_eq!(stats.len(), 2, "one row per (workload × target × mode)");
    let warm = &stats[0];
    assert_eq!((warm.mode, warm.n), (RunMode::Warm, 3));
    assert_eq!(warm.median_wall_s, 12.0);
    assert_eq!(warm.min_wall_s, 10.0);
    assert_eq!(warm.max_wall_s, 14.0);
    assert_eq!(warm.mean_wall_s, 12.0);
    // sample stdev (n−1): deviations ±2, 0 → sqrt(8/2) = 2.0
    assert_eq!(warm.stdev_wall_s, Some(2.0));
    assert_eq!(warm.median_cpu_s, 10.0);
    assert_eq!(warm.median_peak_rss_bytes, 200);
    let cold = &stats[1];
    assert_eq!((cold.mode, cold.n), (RunMode::Cold, 1));
    assert_eq!(cold.stdev_wall_s, None, "stdev absent under n < 2");

    // Even counts average the two middles.
    runs.retain(|r| r.status == RunStatus::Ok && r.mode == Some(RunMode::Warm));
    runs.pop();
    let stats = engine::compute_stats(&runs);
    assert_eq!(stats[0].median_wall_s, 12.0, "(10+14)/2");
    assert_eq!(stats[0].median_peak_rss_bytes, 200, "(100+300)/2");

    // An arm whose runs all failed has NO stats row.
    let failed_only = vec![RunRecord {
        status: RunStatus::Failed,
        error: Some("missed".to_string()),
        ..ok_run("w", "t", RunMode::Warm, 1.0, 1.0, 1)
    }];
    assert!(engine::compute_stats(&failed_only).is_empty());
}

#[test]
fn check_expect_names_each_miss() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("out.xml"), b"<xml/>").unwrap();
    std::fs::write(dir.path().join("empty.xml"), b"").unwrap();
    let expect = Expect {
        exit: 0,
        files: vec!["out.xml".to_string()],
    };
    assert_eq!(engine::check_expect(dir.path(), &expect, 0), None);
    assert_eq!(
        engine::check_expect(dir.path(), &expect, 3),
        Some("exit 3 (expected 0)".to_string())
    );
    let missing = Expect {
        exit: 0,
        files: vec!["nope.xml".to_string()],
    };
    let miss = engine::check_expect(dir.path(), &missing, 0).unwrap();
    assert!(
        miss.contains("expect.files 'nope.xml' is missing"),
        "{miss}"
    );
    let empty = Expect {
        exit: 0,
        files: vec!["empty.xml".to_string()],
    };
    assert_eq!(
        engine::check_expect(dir.path(), &empty, 0),
        Some("expect.files 'empty.xml' is empty".to_string())
    );
}

// ---------------------------------------------------------------------
// the matrix loop, driven by the golden child
// ---------------------------------------------------------------------

#[test]
fn engine_runs_the_matrix_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(&dir.path().join("out")).unwrap();
    let repo_root = repo_root_with_doc(dir.path());
    let policy = RunPolicy {
        warmup: 1,
        repetitions: 2,
        cold_repetitions: 1,
        interleave: true,
    };
    let suite = test_suite(
        &["--sleep-ms", "10", "--touch", "out.txt"],
        &["out.txt"],
        policy,
    );
    // v1-exe + v2-managed kinds exercise both cold-wipe paths.
    let prepared = vec![
        ready("t-one", TargetKind::V1Exe),
        ready("t-two", TargetKind::V2Managed),
    ];
    let mut reprimed: Vec<String> = Vec::new();
    let mut reprime = |_: &BenchLayout, t: &Target| {
        reprimed.push(t.id.clone());
        Ok(())
    };
    let runs =
        engine::execute_matrix(&layout, &suite, &[], &prepared, &repo_root, &mut reprime).unwrap();

    // 2 targets × (2 warm + 1 cold) ok runs; warmup rows are not recorded.
    assert_eq!(runs.len(), 6);
    assert!(runs.iter().all(|r| r.status == RunStatus::Ok));
    for r in &runs {
        assert!(r.wall_s.unwrap() > 0.0);
        assert!(r.cpu_user_s.is_some() && r.cpu_sys_s.is_some());
        assert!(r.peak_rss_bytes.unwrap() > 0);
        assert_eq!(r.exit, Some(0));
    }
    // The interleave: targets rotate per warm iteration (A/B, A/B).
    let warm: Vec<(&str, u32)> = runs
        .iter()
        .filter(|r| r.mode == Some(RunMode::Warm))
        .map(|r| (r.target.as_str(), r.iteration.unwrap()))
        .collect();
    assert_eq!(
        warm,
        vec![("t-one", 1), ("t-two", 1), ("t-one", 2), ("t-two", 2)]
    );
    // Cold: v2-managed was reprimed (the unmeasured re-install) exactly
    // once — before its single cold run; v1-exe never is.
    assert_eq!(reprimed, vec!["t-two".to_string()]);
    let cold: Vec<(&str, u32)> = runs
        .iter()
        .filter(|r| r.mode == Some(RunMode::Cold))
        .map(|r| (r.target.as_str(), r.iteration.unwrap()))
        .collect();
    assert_eq!(cold, vec![("t-one", 1), ("t-two", 1)]);

    // Scratch cells + per-run logs exist.
    assert!(layout
        .scratch
        .join("w-small/t-one/warm-1/out.txt")
        .is_file());
    assert!(layout.logs.join("w-small--t-one--warm-1.log").is_file());

    // The emitted results.json passes both validation gates.
    let triplet = "macos-arm64";
    let runner = engine::runner_meta(&test_platforms(triplet), triplet).unwrap();
    assert_eq!(runner.runs_on, "test-runner");
    assert!(runner.cpus >= 1 && runner.ram_bytes > 0);
    let result = engine::assemble_result(&suite, triplet, runner, Versions::default(), runs);
    assert_eq!(result.stats.len(), 4, "2 targets × 2 modes");
    let dest = engine::emit_result(&layout.root, &result).unwrap();
    let text = std::fs::read_to_string(&dest).unwrap();
    let violations = validate::validate_text(DocKind::Result, &text).unwrap();
    assert!(violations.is_empty(), "{violations:?}");
    let parsed = ResultFile::from_json(&text).unwrap();
    assert_eq!(parsed, result);
}

#[test]
fn unavailable_arms_emit_named_gap_rows() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(&dir.path().join("out")).unwrap();
    let repo_root = repo_root_with_doc(dir.path());
    let suite = test_suite(
        &["--touch", "out.txt"],
        &["out.txt"],
        RunPolicy {
            warmup: 1,
            repetitions: 1,
            cold_repetitions: 1,
            interleave: true,
        },
    );
    let prepared = vec![
        ready("t-one", TargetKind::V1Exe),
        unavailable(
            "t-two",
            TargetKind::V2Managed,
            "no published payload for this triplet",
        ),
    ];
    let runs = engine::execute_matrix(
        &layout,
        &suite,
        &[],
        &prepared,
        &repo_root,
        &mut noop_reprime,
    )
    .unwrap();
    let gaps: Vec<&RunRecord> = runs
        .iter()
        .filter(|r| r.status == RunStatus::Unavailable)
        .collect();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].target, "t-two");
    assert_eq!(
        gaps[0].reason.as_deref(),
        Some("no published payload for this triplet")
    );
    assert!(gaps[0].mode.is_none() && gaps[0].iteration.is_none());
    // The ready arm ran; the gap enters no statistics.
    assert_eq!(runs.iter().filter(|r| r.status == RunStatus::Ok).count(), 2);
    let result = engine::assemble_result(
        &suite,
        "macos-arm64",
        RunnerMeta {
            runs_on: "test".to_string(),
            arch: "test".to_string(),
            cpus: 1,
            ram_bytes: 1,
        },
        Versions::default(),
        runs,
    );
    assert!(result.stats.iter().all(|s| s.target == "t-one"));
    let violations = result.semantic_violations();
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn failed_warmup_is_recorded_once_and_the_cell_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(&dir.path().join("out")).unwrap();
    let repo_root = repo_root_with_doc(dir.path());
    let policy = RunPolicy {
        warmup: 1,
        repetitions: 2,
        cold_repetitions: 1,
        interleave: true,
    };
    let suite = test_suite(&["--exit", "3"], &[], policy);
    // Two broken arms: each must get exactly ONE failed warmup row
    // (iteration 0) and no measured runs — a broken compile is named,
    // never retried (spec 27 §2), and the skip is cell-scoped.
    let prepared = vec![
        ready("t-bad", TargetKind::V1Exe),
        ready("t-also-bad", TargetKind::V2Managed),
    ];
    let runs = engine::execute_matrix(
        &layout,
        &suite,
        &[],
        &prepared,
        &repo_root,
        &mut noop_reprime,
    )
    .unwrap();
    // Each arm: exactly ONE failed warmup row (iteration 0), no measured
    // runs — a broken compile is named, never retried (spec 27 §2).
    assert_eq!(runs.len(), 2);
    for r in &runs {
        assert_eq!(r.status, RunStatus::Failed);
        assert_eq!(r.mode, Some(RunMode::Warm));
        assert_eq!(r.iteration, Some(0));
        assert_eq!(r.exit, Some(3));
        assert!(
            r.error.as_deref().unwrap().contains("exit 3 (expected 0)"),
            "{:?}",
            r.error
        );
    }
    assert!(engine::compute_stats(&runs).is_empty());
    // The failed rows are schema-shaped (mode/iteration/exit/error).
    let result = engine::assemble_result(
        &suite,
        "macos-arm64",
        RunnerMeta {
            runs_on: "test".to_string(),
            arch: "test".to_string(),
            cpus: 1,
            ram_bytes: 1,
        },
        Versions::default(),
        runs,
    );
    let violations = result.semantic_violations();
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn warmup_timeout_is_recorded_and_the_cell_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(&dir.path().join("out")).unwrap();
    let repo_root = repo_root_with_doc(dir.path());
    let mut suite = test_suite(
        &["--sleep-ms", "3000"],
        &[],
        RunPolicy {
            warmup: 1,
            repetitions: 1,
            cold_repetitions: 0,
            interleave: true,
        },
    );
    suite.workloads[0].timeout_s = 1;
    let prepared = vec![ready("t-slow", TargetKind::V1Exe)];
    let runs = engine::execute_matrix(
        &layout,
        &suite,
        &[],
        &prepared,
        &repo_root,
        &mut noop_reprime,
    )
    .unwrap();
    assert_eq!(runs.len(), 1);
    let r = &runs[0];
    assert_eq!(r.status, RunStatus::Timeout);
    assert_eq!(r.mode, Some(RunMode::Warm));
    assert_eq!(r.iteration, Some(0));
    let wall = r.wall_s.unwrap();
    assert!(
        wall > 0.5 && wall < 2.9,
        "killed near the 1 s timeout: {wall}"
    );
    assert!(r.cpu_user_s.is_none() && r.peak_rss_bytes.is_none() && r.exit.is_none());
}

#[test]
fn doc_substitution_and_the_scratch_layout() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(&dir.path().join("out")).unwrap();
    let repo_root = repo_root_with_doc(dir.path());
    // {doc} substitutes the document's file NAME (the exact token;
    // anything else is literal — spec 27 §2); the run's cwd is the
    // document's directory (the vendored flat case: the cell root).
    let suite = test_suite(
        &["--touch", "{doc}"],
        &["doc.adoc"],
        RunPolicy {
            warmup: 0,
            repetitions: 1,
            cold_repetitions: 0,
            interleave: true,
        },
    );
    let prepared = vec![ready("t-one", TargetKind::V1Exe)];
    let runs = engine::execute_matrix(
        &layout,
        &suite,
        &[],
        &prepared,
        &repo_root,
        &mut noop_reprime,
    )
    .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, RunStatus::Ok, "{:?}", runs[0].error);
    let cell = layout.scratch.join("w-small/t-one/warm-1");
    // The child overwrote the COPIED document via the substituted path —
    // the repo-root original is untouched (runs never mutate sources).
    assert_eq!(
        std::fs::read(cell.join("doc.adoc")).unwrap(),
        b"tebako-bench-child\n"
    );
    assert_eq!(
        std::fs::read(repo_root.join("fixtures/doc.adoc")).unwrap(),
        b"= Title\n\nbody\n"
    );
}

#[test]
fn opt_in_gating_is_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(&dir.path().join("out")).unwrap();
    let repo_root = repo_root_with_doc(dir.path());
    let mut suite = test_suite(
        &["--touch", "out.txt"],
        &["out.txt"],
        RunPolicy {
            warmup: 0,
            repetitions: 1,
            cold_repetitions: 0,
            interleave: true,
        },
    );
    suite.workloads[0].opt_in = true;
    let prepared = vec![ready("t-one", TargetKind::V1Exe)];

    // Skipped silently-by-design: nothing was asked, so NO rows at all.
    let runs = engine::execute_matrix(
        &layout,
        &suite,
        &[],
        &prepared,
        &repo_root,
        &mut noop_reprime,
    )
    .unwrap();
    assert!(runs.is_empty());

    // An unknown --opt-in id is a named operational error (a typo is
    // never silent); so is a non-opt-in id.
    let err = engine::execute_matrix(
        &layout,
        &suite,
        &["nope".to_string()],
        &prepared,
        &repo_root,
        &mut noop_reprime,
    )
    .unwrap_err();
    assert!(
        err.message.contains("--opt-in 'nope' names no workload"),
        "{}",
        err.message
    );

    // Opted in: the workload runs.
    let runs = engine::execute_matrix(
        &layout,
        &suite,
        &["w-small".to_string()],
        &prepared,
        &repo_root,
        &mut noop_reprime,
    )
    .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, RunStatus::Ok);
}
