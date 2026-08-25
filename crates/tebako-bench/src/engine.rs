//! The run engine (spec 27 §5–§7): the acquisition orchestration that
//! turns suite targets into prepared programs, the measured matrix loop
//! (warmup → warm, interleaved → cold-with-wipe), the expectation check,
//! and the statistics over the run records. Network access lives ONLY in
//! [`prepare_targets`] (it drives the acquisition slice); the matrix
//! itself — [`execute_matrix`] — is offline given prepared targets, which
//! is how the tests drive it with the `tebako-bench-child` golden child.
//!
//! Run-shape conventions (the engine owns these; the suite/fixtures were
//! authored against them):
//!
//! - Per-run scratch cell: `scratch/<workload>/<target>/<mode>-<iter>/`
//!   (warmups: `warmup-<k>`). The materialized source tree is copied in
//!   whole (relative includes must resolve); the run's working directory
//!   is the document's own directory inside the cell and `{doc}`
//!   substitutes the document's file NAME — so "scratch-relative"
//!   `expect.files` resolve against the directory the compile's outputs
//!   land in, for the vendored (flat) and git (deep tree) cases alike.
//! - Warmup runs are unmeasured priming. A warmup that misses its
//!   expectation is recorded `failed` with `iteration: 0` (named, never
//!   retried — spec 27 §2) and the cell is SKIPPED (no measured runs on
//!   an arm that cannot compile once); a warmup timeout is recorded
//!   `timeout` the same way. A warmup that cannot even START (spawn
//!   failure — e.g. the AMFI-killed v1 exe of §9 spike c) makes the arm
//!   a named gap: one `unavailable` row whose reason carries the spawn
//!   error, the "platform incapacity" clause of §6 applied at run time.
//! - Cold runs follow the §5 per-arm flow: wipe the arm's cache set,
//!   then (v2-managed only) an UNMEASURED re-`install` — the caller's
//!   `cold_reprime` callback — and the measured run (the runtime/env
//!   download lands inside the measured span).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::acquire::{self, BenchLayout, MaterializedSource, TebakoTools};
use crate::error::BenchError;
use crate::platforms::PlatformFile;
use crate::result::{ResultFile, RunMode, RunRecord, RunStatus, RunnerMeta, StatRecord, Versions};
use crate::sampler::{ChildSpec, Sample, Sampler};
use crate::suite::{Expect, SuiteFile, Target, TargetKind, Workload};
use crate::{exit, validate, DocKind};

/// The `run` subcommand's input: both documents parsed (main owns the
/// file I/O), the triplet this leg runs, and the run options.
pub struct RunRequest {
    pub suite: SuiteFile,
    pub platforms: PlatformFile,
    pub triplet: String,
    pub out: PathBuf,
    pub opt_in: Vec<String>,
    /// `--tebako-release <tag>`: the pinned tools release; None = latest.
    pub tebako_release: Option<String>,
    /// Vendored source paths resolve against this root (the repo root).
    pub repo_root: PathBuf,
}

/// One target after acquisition: either its measured program is staged
/// or the arm is a named gap (§6: explicit data, never a silent skip).
pub enum Prepared {
    Ready { program: PathBuf },
    Unavailable { reason: String },
}

pub struct PreparedTarget {
    pub target: Target,
    pub state: Prepared,
}

/// The `run` surface: acquire, execute, emit, and return the exit code
/// (spec 27 §8 — 0, or 1 when every arm failed/unavailable; a red matrix
/// is a deliverable, the artifacts are written either way).
pub fn run(request: &RunRequest) -> Result<u8, BenchError> {
    let violations = request
        .suite
        .semantic_violations()
        .into_iter()
        .map(|v| format!("suite: {v}"))
        .chain(
            request
                .platforms
                .semantic_violations()
                .into_iter()
                .map(|v| format!("platforms: {v}")),
        )
        .collect::<Vec<_>>();
    if !violations.is_empty() {
        return Err(BenchError::operational(format!(
            "run: the input documents are invalid — validate them first:\n  {}",
            violations.join("\n  ")
        )));
    }

    let layout = BenchLayout::new(&request.out)?;
    let (prepared, versions, tools) = prepare_targets(
        &layout,
        &request.suite,
        &request.platforms,
        &request.triplet,
        request.tebako_release.as_deref(),
    )?;

    // The §5 cold flow's unmeasured half: the engine calls this only for
    // v2-managed targets — re-install the payload after the store wipe
    // (install does not fetch the runtime — that download lands in the
    // measured span).
    let mut cold_reprime = |layout: &BenchLayout, target: &Target| -> Result<(), BenchError> {
        let tools = tools.as_ref().ok_or_else(|| {
            BenchError::operational(format!(
                "engine: v2-managed target '{}' is ready but the tebako tools were never fetched (harness bug)",
                target.id
            ))
        })?;
        acquire::install_payload(layout, tools, target).map(|_| ())
    };
    let runs = execute_matrix(
        &layout,
        &request.suite,
        &request.opt_in,
        &prepared,
        &request.repo_root,
        &mut cold_reprime,
    )?;

    let result = assemble_result(
        &request.suite,
        &request.triplet,
        runner_meta(&request.platforms, &request.triplet)?,
        versions,
        runs,
    );
    emit_result(&request.out, &result)?;

    let any_ok = result.runs.iter().any(|r| r.status == RunStatus::Ok);
    if any_ok || result.runs.is_empty() {
        Ok(exit::OK)
    } else {
        Ok(exit::INVALID)
    }
}

/// The acquisition half: stage every target's measured program (or name
/// its gap) and collect the resolved versions (§6: what actually ran,
/// never what was requested). Acquisition FAILURES degrade the arm to a
/// named gap whose reason carries the error — one broken download never
/// costs the other arms their numbers.
fn prepare_targets(
    layout: &BenchLayout,
    suite: &SuiteFile,
    platforms: &PlatformFile,
    triplet: &str,
    tebako_release: Option<&str>,
) -> Result<(Vec<PreparedTarget>, Versions, Option<TebakoTools>), BenchError> {
    let entry = platforms.triplets.get(triplet).ok_or_else(|| {
        BenchError::operational(format!(
            "engine: platforms.yaml has no triplet '{triplet}' (known: {})",
            platforms
                .triplets
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    let mut tools: Option<TebakoTools> = None;
    let mut versions = Versions::default();
    let mut prepared = Vec::new();
    for target in &suite.targets {
        let state = match target.kind {
            TargetKind::V1Exe => match &entry.v1_asset {
                None => Prepared::Unavailable {
                    reason: format!(
                        "platforms.yaml: v1_asset is null for {triplet} — packed-mn shipped no asset for this triplet{}",
                        entry
                            .v1_note
                            .as_ref()
                            .map(|n| format!(" ({n})"))
                            .unwrap_or_default()
                    ),
                },
                Some(_) => match acquire::acquire_v1_exe(layout, platforms, triplet) {
                    Ok(exe) => {
                        let tag = &platforms.packed_mn.tag;
                        versions.packed_mn =
                            Some(format!("{tag} (metanorma-cli {})", tag.trim_start_matches('v')));
                        Prepared::Ready { program: exe }
                    }
                    Err(e) => Prepared::Unavailable {
                        reason: format!("v1 acquisition failed: {e}"),
                    },
                },
            },
            TargetKind::V2Managed | TargetKind::V2Press => {
                if !entry.v2_payload {
                    Prepared::Unavailable {
                        reason: format!(
                            "platforms.yaml: v2_payload is false for {triplet} — no published payload"
                        ),
                    }
                } else {
                    match prepare_v2(layout, triplet, tebako_release, &mut tools, target) {
                        Ok(staged) => {
                            versions.tebako = Some(staged.tools_version);
                            versions.runtime = Some(format!(
                                "{}-{}",
                                staged.runtime_tebako_version, staged.runtime_lang_version
                            ));
                            versions.payload = Some(staged.payload_release_tag);
                            versions.image_format = Some(staged.image_format);
                            Prepared::Ready {
                                program: staged.program,
                            }
                        }
                        Err(e) => Prepared::Unavailable {
                            reason: format!("v2 acquisition failed: {e}"),
                        },
                    }
                }
            }
        };
        prepared.push(PreparedTarget {
            target: target.clone(),
            state,
        });
    }
    Ok((prepared, versions, tools))
}

/// What a staged v2 arm reports upward (the measured program plus the
/// resolved-version records the result document carries).
struct StagedV2 {
    program: PathBuf,
    tools_version: String,
    runtime_tebako_version: String,
    runtime_lang_version: String,
    payload_release_tag: String,
    image_format: crate::result::ImageFormat,
}

/// Stage one v2 arm through the dogfood path (spec 27 §5): the
/// downloaded CLI installs the payload, a shim dispatch primes the
/// runtime, and the arm's program is the shim (managed) or the
/// in-process-assembled fat package (press).
fn prepare_v2(
    layout: &BenchLayout,
    triplet: &str,
    tebako_release: Option<&str>,
    tools: &mut Option<TebakoTools>,
    target: &Target,
) -> Result<StagedV2, BenchError> {
    if tools.is_none() {
        *tools = Some(acquire::fetch_tebako_tools(
            layout,
            tebako_release,
            triplet,
        )?);
    }
    let tools_ref = tools.as_ref().ok_or_else(|| {
        BenchError::operational(
            "engine: the tools slot is empty right after the fetch (harness bug)",
        )
    })?;
    let payload = acquire::install_payload(layout, tools_ref, target)?;
    let runtime = acquire::prime_runtime(layout, triplet, &payload)?;
    let program = match target.kind {
        TargetKind::V2Managed => acquire::shim_path(layout, triplet, &payload)?,
        TargetKind::V2Press => {
            acquire::assemble_fat_package(layout, tools_ref, &payload, &runtime, target)?
        }
        TargetKind::V1Exe => {
            return Err(BenchError::operational(format!(
                "engine: prepare_v2 called for the v1 target '{}' (harness bug)",
                target.id
            )))
        }
    };
    Ok(StagedV2 {
        program,
        tools_version: tools_ref.version.clone(),
        runtime_tebako_version: runtime.tebako_version,
        runtime_lang_version: runtime.lang_version,
        payload_release_tag: payload.release_tag,
        image_format: payload.image_format,
    })
}

/// The measured matrix (spec 27 §5): per selected workload, the named
/// gaps first (explicit rows), warmups per ready target, the warm
/// repetitions (targets interleaved per iteration when the policy says
/// so — drift decorrelation), then the cold repetitions (wipe → the
/// v2-managed-only unmeasured reprime → measured run). Returns every run
/// record in execution order.
pub fn execute_matrix(
    layout: &BenchLayout,
    suite: &SuiteFile,
    opt_in: &[String],
    prepared: &[PreparedTarget],
    repo_root: &Path,
    cold_reprime: &mut dyn FnMut(&BenchLayout, &Target) -> Result<(), BenchError>,
) -> Result<Vec<RunRecord>, BenchError> {
    // An --opt-in id naming nothing (or a non-opt-in workload) is a
    // typo; named, never silent (invariant 9).
    for id in opt_in {
        match suite.workloads.iter().find(|w| &w.id == id) {
            Some(w) if w.opt_in => {}
            Some(_) => {
                return Err(BenchError::operational(format!(
                    "engine: --opt-in '{id}' names a workload that is not opt_in"
                )))
            }
            None => {
                return Err(BenchError::operational(format!(
                    "engine: --opt-in '{id}' names no workload in suite '{}'",
                    suite.name
                )))
            }
        }
    }
    let workloads: Vec<&Workload> = suite
        .workloads
        .iter()
        .filter(|w| !w.opt_in || opt_in.contains(&w.id))
        .collect();

    let mut sampler = Sampler::new();
    let mut runs = Vec::new();
    for w in workloads {
        let source = acquire::materialize_source(w, layout, repo_root)?;

        // Named gaps are explicit rows, one per workload (§6).
        for pt in prepared {
            if let Prepared::Unavailable { reason } = &pt.state {
                runs.push(RunRecord {
                    workload: w.id.clone(),
                    target: pt.target.id.clone(),
                    mode: None,
                    iteration: None,
                    status: RunStatus::Unavailable,
                    wall_s: None,
                    cpu_user_s: None,
                    cpu_sys_s: None,
                    peak_rss_bytes: None,
                    exit: None,
                    error: None,
                    reason: Some(reason.clone()),
                });
            }
        }

        // Warmups: unmeasured priming. A missed expectation or a timeout
        // kills the cell (the failed/timeout row at iteration 0 tells the
        // story); a spawn failure makes the arm a named gap.
        let mut dead: BTreeSet<usize> = BTreeSet::new();
        for (t_idx, pt) in prepared.iter().enumerate() {
            let Prepared::Ready { program } = &pt.state else {
                continue;
            };
            for k in 1..=suite.run_policy.warmup {
                match run_once(
                    &mut sampler,
                    layout,
                    &source,
                    w,
                    &pt.target,
                    program,
                    RunMode::Warm,
                    k,
                    true,
                ) {
                    Ok((sample, cwd)) => {
                        if let Some(record) = warmup_outcome(w, &pt.target, &sample, &cwd) {
                            runs.push(record);
                            dead.insert(t_idx);
                            break;
                        }
                    }
                    Err(e) => {
                        runs.push(RunRecord {
                            workload: w.id.clone(),
                            target: pt.target.id.clone(),
                            mode: None,
                            iteration: None,
                            status: RunStatus::Unavailable,
                            wall_s: None,
                            cpu_user_s: None,
                            cpu_sys_s: None,
                            peak_rss_bytes: None,
                            exit: None,
                            error: None,
                            reason: Some(format!(
                                "the staged program could not start on this triplet: {e}"
                            )),
                        });
                        dead.insert(t_idx);
                        break;
                    }
                }
            }
        }

        // The warm repetitions.
        for (t_idx, iteration) in warm_schedule(
            prepared.len(),
            suite.run_policy.repetitions,
            suite.run_policy.interleave,
        ) {
            let pt = &prepared[t_idx];
            let Prepared::Ready { program } = &pt.state else {
                continue;
            };
            if dead.contains(&t_idx) {
                continue;
            }
            let (sample, cwd) = run_once(
                &mut sampler,
                layout,
                &source,
                w,
                &pt.target,
                program,
                RunMode::Warm,
                iteration,
                false,
            )?;
            runs.push(measured_record(
                w,
                &pt.target,
                RunMode::Warm,
                iteration,
                &sample,
                &cwd,
            ));
        }

        // The cold repetitions: wipe → unmeasured reprime → measured run.
        for (t_idx, pt) in prepared.iter().enumerate() {
            let Prepared::Ready { program } = &pt.state else {
                continue;
            };
            if dead.contains(&t_idx) {
                continue;
            }
            for iteration in 1..=suite.run_policy.cold_repetitions {
                layout.wipe_cold_caches(&pt.target.id, pt.target.kind)?;
                // The §5 cold flow's unmeasured re-install is v2-managed's
                // alone (v1 re-extracts in-span; the fat package needs no
                // store) — the engine owns WHEN, the callback owns HOW.
                if pt.target.kind == TargetKind::V2Managed {
                    cold_reprime(layout, &pt.target)?;
                }
                let (sample, cwd) = run_once(
                    &mut sampler,
                    layout,
                    &source,
                    w,
                    &pt.target,
                    program,
                    RunMode::Cold,
                    iteration,
                    false,
                )?;
                runs.push(measured_record(
                    w,
                    &pt.target,
                    RunMode::Cold,
                    iteration,
                    &sample,
                    &cwd,
                ));
            }
        }
    }
    Ok(runs)
}

/// One child execution: fresh scratch cell (the source tree copied in),
/// cwd = the document's directory, `{doc}` = the document's file name,
/// the hermetic bench-home env, output to `logs/`. Warmup cells are
/// named `warmup-<k>` so priming outputs never pollute a measured cell.
#[allow(clippy::too_many_arguments)]
fn run_once(
    sampler: &mut Sampler,
    layout: &BenchLayout,
    source: &MaterializedSource,
    workload: &Workload,
    target: &Target,
    program: &Path,
    mode: RunMode,
    iteration: u32,
    warmup: bool,
) -> Result<(Sample, PathBuf), BenchError> {
    let cell_name = if warmup {
        format!("warmup-{iteration}")
    } else {
        format!("{}-{iteration}", mode_str(mode))
    };
    let cell = layout
        .scratch
        .join(&workload.id)
        .join(&target.id)
        .join(&cell_name);
    if cell.exists() {
        std::fs::remove_dir_all(&cell).map_err(|e| {
            BenchError::operational(format!("engine: cannot clear {}: {e}", cell.display()))
        })?;
    }
    copy_tree(&source.root, &cell)?;
    // The hermetic per-target TMPDIR must EXIST before the child boots
    // (the v1 bootstrap's temp_directory_path aborts on a missing dir —
    // the cold wipe recreates it; the first run must too).
    let target_tmp = layout.tmp.join(&target.id);
    std::fs::create_dir_all(&target_tmp).map_err(|e| {
        BenchError::operational(format!(
            "engine: cannot create {}: {e}",
            target_tmp.display()
        ))
    })?;
    let cwd = cell
        .join(source.doc_rel.parent().unwrap_or_else(|| Path::new("")))
        .canonicalize()
        .map_err(|e| {
            BenchError::operational(format!(
                "engine: the document directory for workload '{}' is missing under {}: {e}",
                workload.id,
                cell.display()
            ))
        })?;
    let doc_name = source
        .doc_rel
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| {
            BenchError::operational(format!(
                "engine: workload '{}' document path has no file name",
                workload.id
            ))
        })?;
    let mut argv = vec![program.to_string_lossy().into_owned()];
    argv.extend(workload.argv.iter().map(|a| {
        if a == "{doc}" {
            doc_name.clone()
        } else {
            a.clone()
        }
    }));
    let spec = ChildSpec {
        argv,
        cwd: cwd.clone(),
        env: layout.child_env(&target.id),
        log_path: layout
            .logs
            .join(format!("{}--{}--{}.log", workload.id, target.id, cell_name)),
        timeout: Duration::from_secs(workload.timeout_s),
    };
    let sample = sampler.run(&spec)?;
    Ok((sample, cwd))
}

/// The warmup verdict: None on success, else the one row that closes the
/// cell (failed = expectation missed; timeout = killed at timeout_s).
fn warmup_outcome(
    workload: &Workload,
    target: &Target,
    sample: &Sample,
    cwd: &Path,
) -> Option<RunRecord> {
    if sample.timed_out {
        return Some(timeout_record(workload, target, RunMode::Warm, 0, sample));
    }
    check_expect(cwd, &workload.expect, sample.exit).map(|miss| RunRecord {
        workload: workload.id.clone(),
        target: target.id.clone(),
        mode: Some(RunMode::Warm),
        iteration: Some(0),
        status: RunStatus::Failed,
        wall_s: Some(sample.wall_s),
        cpu_user_s: Some(sample.cpu_user_s),
        cpu_sys_s: Some(sample.cpu_sys_s),
        peak_rss_bytes: Some(sample.peak_rss_bytes),
        exit: Some(sample.exit),
        error: Some(format!("warmup: {miss}")),
        reason: None,
    })
}

/// The measured-run record (§6): timeout carries wall only; failed
/// carries the exit, whatever metrics exist, and the named miss; ok
/// carries everything.
fn measured_record(
    workload: &Workload,
    target: &Target,
    mode: RunMode,
    iteration: u32,
    sample: &Sample,
    cwd: &Path,
) -> RunRecord {
    if sample.timed_out {
        return timeout_record(workload, target, mode, iteration, sample);
    }
    let base = RunRecord {
        workload: workload.id.clone(),
        target: target.id.clone(),
        mode: Some(mode),
        iteration: Some(iteration),
        status: RunStatus::Ok,
        wall_s: Some(sample.wall_s),
        cpu_user_s: Some(sample.cpu_user_s),
        cpu_sys_s: Some(sample.cpu_sys_s),
        peak_rss_bytes: Some(sample.peak_rss_bytes),
        exit: Some(sample.exit),
        error: None,
        reason: None,
    };
    match check_expect(cwd, &workload.expect, sample.exit) {
        None => base,
        Some(miss) => RunRecord {
            status: RunStatus::Failed,
            error: Some(miss),
            ..base
        },
    }
}

fn timeout_record(
    workload: &Workload,
    target: &Target,
    mode: RunMode,
    iteration: u32,
    sample: &Sample,
) -> RunRecord {
    RunRecord {
        workload: workload.id.clone(),
        target: target.id.clone(),
        mode: Some(mode),
        iteration: Some(iteration),
        status: RunStatus::Timeout,
        wall_s: Some(sample.wall_s),
        cpu_user_s: None,
        cpu_sys_s: None,
        peak_rss_bytes: None,
        exit: None,
        error: None,
        reason: None,
    }
}

/// The expectation check (§2): exit status, then every `expect.files`
/// entry existing and non-empty. The first miss is returned, named.
pub fn check_expect(cwd: &Path, expect: &Expect, exit_code: i32) -> Option<String> {
    if exit_code != expect.exit {
        return Some(format!("exit {exit_code} (expected {})", expect.exit));
    }
    for f in &expect.files {
        let p = cwd.join(f);
        match std::fs::metadata(&p) {
            Ok(m) if m.len() > 0 => {}
            Ok(_) => return Some(format!("expect.files '{f}' is empty")),
            Err(e) => return Some(format!("expect.files '{f}' is missing ({e})")),
        }
    }
    None
}

/// The warm-run order (§2's run_policy): interleaved — rotate targets
/// per iteration so runner drift decorrelates across arms; otherwise
/// each target to completion in turn (debugging only). Pure: the
/// ordering is pinned by unit tests.
pub fn warm_schedule(n_targets: usize, repetitions: u32, interleave: bool) -> Vec<(usize, u32)> {
    if interleave {
        (1..=repetitions)
            .flat_map(|i| (0..n_targets).map(move |t| (t, i)))
            .collect()
    } else {
        (0..n_targets)
            .flat_map(|t| (1..=repetitions).map(move |i| (t, i)))
            .collect()
    }
}

/// The statistics (§6/§7): one StatRecord per (workload × target ×
/// mode) over status-ok runs only, n ≥ 1. An arm whose runs all failed
/// has no row — the run records tell the story.
pub fn compute_stats(runs: &[RunRecord]) -> Vec<StatRecord> {
    let mut cells: BTreeMap<(&str, &str, RunMode), Vec<&RunRecord>> = BTreeMap::new();
    for r in runs.iter().filter(|r| r.status == RunStatus::Ok) {
        if let Some(mode) = r.mode {
            cells
                .entry((&r.workload, &r.target, mode))
                .or_default()
                .push(r);
        }
    }
    cells
        .into_iter()
        .map(|((workload, target, mode), cell)| {
            let mut walls: Vec<f64> = cell.iter().filter_map(|r| r.wall_s).collect();
            walls.sort_by(|a, b| a.total_cmp(b));
            let mut cpus: Vec<f64> = cell
                .iter()
                .filter_map(|r| Some(r.cpu_user_s? + r.cpu_sys_s?))
                .collect();
            cpus.sort_by(|a, b| a.total_cmp(b));
            let mut rss: Vec<u64> = cell.iter().filter_map(|r| r.peak_rss_bytes).collect();
            rss.sort_unstable();
            let n = cell.len() as u32;
            let mean = walls.iter().sum::<f64>() / walls.len() as f64;
            StatRecord {
                workload: workload.to_string(),
                target: target.to_string(),
                mode,
                n,
                median_wall_s: median_f64(&walls),
                min_wall_s: walls.first().copied().unwrap_or(0.0),
                max_wall_s: walls.last().copied().unwrap_or(0.0),
                stdev_wall_s: stdev_sample(&walls, mean),
                mean_wall_s: mean,
                median_cpu_s: median_f64(&cpus),
                median_peak_rss_bytes: median_u64(&rss),
            }
        })
        .collect()
}

/// Median of a sorted f64 slice (even counts average the two middles).
fn median_f64(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// Median of a sorted u64 slice, integer-rounded.
fn median_u64(sorted: &[u64]) -> u64 {
    let n = sorted.len();
    if n == 0 {
        return 0;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2
    }
}

/// Sample standard deviation (n−1); absent under n < 2 (§6).
fn stdev_sample(sorted: &[f64], mean: f64) -> Option<f64> {
    if sorted.len() < 2 {
        return None;
    }
    let var =
        sorted.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (sorted.len() - 1) as f64;
    Some(var.sqrt())
}

/// The §6 runner metadata: the runner label flows from platforms.yaml,
/// the rest is read off the machine (numbers without their environment
/// are not numbers).
pub fn runner_meta(platforms: &PlatformFile, triplet: &str) -> Result<RunnerMeta, BenchError> {
    let entry = platforms.triplets.get(triplet).ok_or_else(|| {
        BenchError::operational(format!("engine: platforms.yaml has no triplet '{triplet}'"))
    })?;
    Ok(RunnerMeta {
        runs_on: entry.runner.clone(),
        arch: std::env::consts::ARCH.to_string(),
        cpus: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1),
        ram_bytes: crate::ram_total_bytes()?,
    })
}

/// Assemble the result document (statistics derived from the run
/// records, never authored).
pub fn assemble_result(
    suite: &SuiteFile,
    triplet: &str,
    runner: RunnerMeta,
    versions: Versions,
    runs: Vec<RunRecord>,
) -> ResultFile {
    let stats = compute_stats(&runs);
    ResultFile {
        schema_version: 1,
        suite: suite.name.clone(),
        triplet: triplet.to_string(),
        runner,
        versions,
        runs,
        stats,
    }
}

/// Write `<out>/results.json`, self-checked: the emitted text re-enters
/// BOTH validation gates (the versioned schema + the serde model +
/// semantics). A violation here is a harness bug, surfaced operationally
/// instead of shipping a bad artifact.
pub fn emit_result(out: &Path, result: &ResultFile) -> Result<PathBuf, BenchError> {
    let text = result.to_json().map_err(|e| {
        BenchError::operational(format!(
            "engine: the result document does not serialize: {e}"
        ))
    })?;
    let violations = validate::validate_text(DocKind::Result, &text)?;
    if !violations.is_empty() {
        return Err(BenchError::operational(format!(
            "engine: the emitted results.json fails validation (harness bug):\n  {}",
            violations.join("\n  ")
        )));
    }
    let dest = out.join("results.json");
    std::fs::write(&dest, format!("{text}\n")).map_err(|e| {
        BenchError::operational(format!("engine: cannot write {}: {e}", dest.display()))
    })?;
    Ok(dest)
}

fn mode_str(mode: RunMode) -> &'static str {
    match mode {
        RunMode::Warm => "warm",
        RunMode::Cold => "cold",
    }
}

/// Recursive copy of the materialized source tree into a run cell
/// (files + dirs; symlinks are not expected in suite sources — a
/// non-file/non-dir entry is copied as its file bytes or named).
fn copy_tree(src: &Path, dst: &Path) -> Result<(), BenchError> {
    std::fs::create_dir_all(dst).map_err(|e| {
        BenchError::operational(format!("engine: cannot create {}: {e}", dst.display()))
    })?;
    let entries = std::fs::read_dir(src).map_err(|e| {
        BenchError::operational(format!("engine: cannot list {}: {e}", src.display()))
    })?;
    for entry in entries.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type().map_err(|e| {
            BenchError::operational(format!("engine: cannot stat {}: {e}", from.display()))
        })?;
        if ft.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| {
                BenchError::operational(format!(
                    "engine: cannot copy {} into the run scratch: {e}",
                    from.display()
                ))
            })?;
        }
    }
    Ok(())
}
