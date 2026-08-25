//! Golden tests for the spec 27 §4 sampler: the measured child is the
//! crate's own `tebako-bench-child` helper (one identical child on every
//! triplet — no platform `time`/`sleep` flavor divergence). Assertions:
//! wall/cpu/rss nonzero, ordered (a busy child burns more CPU than a
//! sleeping one), unit-consistent (`peak_rss_bytes` is BYTES everywhere —
//! a KiB-confused Linux value fails the floors below by three orders of
//! magnitude), the timeout kill records 137, and the child's cwd/env/log
//! plumbing works (the run engine's expectation checks ride on it).

use std::path::PathBuf;
use std::time::Duration;

use tebako_bench::sampler::{ChildSpec, Sampler};

/// The test-helper binary's path. Cargo exports CARGO_BIN_FILE_<name> to
/// integration tests at RUN time (not compile time); the debug-profile
/// fallback keeps `cargo test` usable from any cargo version.
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

fn spec(dir: &std::path::Path, name: &str, args: &[&str], timeout_s: u64) -> ChildSpec {
    let mut argv = vec![child_path().to_string_lossy().into_owned()];
    argv.extend(args.iter().map(|s| s.to_string()));
    ChildSpec {
        argv,
        cwd: dir.to_path_buf(),
        env: Vec::new(),
        log_path: dir.join(format!("{name}.log")),
        timeout: Duration::from_secs(timeout_s),
    }
}

fn rss_sane(bytes: u64) -> bool {
    // Unit consistency: a byte-denominated RSS of any real process clears
    // 100 KiB; a KiB-confused value (the Linux/musl normalization bug the
    // spec calls out) reads ~1–4 THOUSAND here and fails the floor.
    (100_000..16 * 1024 * 1024 * 1024).contains(&bytes)
}

#[test]
fn sleep_child_measures_wall_cpu_rss() {
    let dir = tempfile::tempdir().unwrap();
    let mut sampler = Sampler::new();
    let sample = sampler
        .run(&spec(dir.path(), "sleep", &["--sleep-ms", "300"], 30))
        .unwrap();
    assert_eq!(sample.exit, 0);
    assert!(!sample.timed_out);
    assert!(
        sample.wall_s >= 0.25,
        "wall {}s must cover the 300ms sleep",
        sample.wall_s
    );
    let cpu = sample.cpu_user_s + sample.cpu_sys_s;
    assert!(cpu > 0.0, "cpu must be attributed, got 0");
    assert!(
        cpu < sample.wall_s,
        "a sleeping child cannot burn {cpu}s cpu in {}s wall",
        sample.wall_s
    );
    assert!(
        rss_sane(sample.peak_rss_bytes),
        "peak_rss_bytes {} fails the byte-unit sanity band",
        sample.peak_rss_bytes
    );
}

#[test]
fn busy_child_burns_more_cpu_than_a_sleeping_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut sampler = Sampler::new();
    let busy = sampler
        .run(&spec(dir.path(), "busy", &["--busy-ms", "1500"], 30))
        .unwrap();
    let sleepy = sampler
        .run(&spec(dir.path(), "sleepy", &["--sleep-ms", "400"], 30))
        .unwrap();
    let busy_cpu = busy.cpu_user_s + busy.cpu_sys_s;
    let sleepy_cpu = sleepy.cpu_user_s + sleepy.cpu_sys_s;
    // The spin is wall-anchored inside the child; on a loaded machine the
    // child is descheduled and the attributed CPU is its on-core share
    // only (observed: 0.13s of a 400ms spin under a full parallel cargo
    // build, 0.06s at load average 160 — the developer machine this suite
    // was written on). Absolute duty-cycle floors are therefore noise
    // traps; the golden claims are ATTRIBUTION (nonzero, and the busy
    // child out-burns the sleeping one) — sleepy cpu is the millisecond
    // startup cost, busy is the spin's on-core share, and the 3x margin
    // survives even a 6% duty cycle.
    assert!(
        busy_cpu > 0.005,
        "a 1500ms spin must attribute real cpu even starved, got {busy_cpu}s"
    );
    assert!(
        busy.wall_s >= 1.4,
        "wall {}s must cover the 1500ms spin",
        busy.wall_s
    );
    assert!(
        busy_cpu > sleepy_cpu * 3.0,
        "ordering: busy cpu {busy_cpu}s vs sleepy cpu {sleepy_cpu}s"
    );
    assert!(rss_sane(busy.peak_rss_bytes));
}

#[test]
fn peak_rss_tracks_the_allocation_in_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let mut sampler = Sampler::new();
    let big = sampler
        .run(&spec(
            dir.path(),
            "alloc",
            &["--alloc-mb", "32", "--sleep-ms", "50"],
            30,
        ))
        .unwrap();
    // 32 MiB touched and held: the high-water mark must show it. A
    // KiB-confused record reads 32768 here — the floor is 500x that.
    assert!(
        big.peak_rss_bytes >= 16 * 1024 * 1024,
        "peak_rss_bytes {} must reflect the held 32 MiB",
        big.peak_rss_bytes
    );
}

#[test]
fn timeout_kills_the_child_and_records_137() {
    let dir = tempfile::tempdir().unwrap();
    let mut sampler = Sampler::new();
    let start = std::time::Instant::now();
    let sample = sampler
        .run(&spec(dir.path(), "stuck", &["--sleep-ms", "60000"], 1))
        .unwrap();
    let outer = start.elapsed();
    assert!(sample.timed_out, "the 60s child must hit the 1s timeout");
    assert_eq!(
        sample.exit, 137,
        "a timeout SIGKILL records 137 (the shell convention)"
    );
    assert!(
        sample.wall_s >= 1.0 && sample.wall_s < 15.0,
        "wall {}s should hug the 1s timeout",
        sample.wall_s
    );
    assert!(
        outer < Duration::from_secs(20),
        "the kill must actually land (outer elapsed {outer:?})"
    );
}

#[test]
fn exit_status_propagates_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    let mut sampler = Sampler::new();
    let sample = sampler
        .run(&spec(dir.path(), "exit3", &["--exit", "3"], 30))
        .unwrap();
    assert_eq!(sample.exit, 3);
    assert!(!sample.timed_out);
}

#[test]
fn cwd_env_and_log_plumbing() {
    let dir = tempfile::tempdir().unwrap();
    let scratch = dir.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let mut s = spec(
        &scratch,
        "plumbing",
        &[
            "--touch",
            "out.txt",
            "--print",
            "hello-bench",
            "--print-env",
            "TEBAKO_BENCH_PROBE",
        ],
        30,
    );
    s.env = vec![("TEBAKO_BENCH_PROBE".to_string(), "probe-value".to_string())];
    let mut sampler = Sampler::new();
    let sample = sampler.run(&s).unwrap();
    assert_eq!(sample.exit, 0);
    // cwd: the relative --touch landed inside the scratch dir.
    let touched = scratch.join("out.txt");
    assert!(touched.is_file(), "cwd-relative touch must land in scratch");
    assert!(
        std::fs::metadata(&touched).unwrap().len() > 0,
        "the expectation file must be non-empty (spec 27 §2)"
    );
    // log: stdout+stderr were appended to the log file.
    let log = std::fs::read_to_string(&s.log_path).unwrap();
    assert!(
        log.contains("hello-bench"),
        "log must capture stdout: {log}"
    );
    // env: the override flowed to the child.
    assert!(
        log.contains("TEBAKO_BENCH_PROBE=probe-value"),
        "env override must reach the child: {log}"
    );
}

#[test]
fn empty_argv_is_a_named_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut sampler = Sampler::new();
    let err = sampler
        .run(&ChildSpec {
            argv: Vec::new(),
            cwd: dir.path().to_path_buf(),
            env: Vec::new(),
            log_path: PathBuf::from(dir.path()).join("never.log"),
            timeout: Duration::from_secs(1),
        })
        .unwrap_err();
    assert!(
        err.message.contains("empty argv"),
        "named error expected, got: {}",
        err.message
    );
}

#[test]
fn spawn_failure_is_a_named_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(dir.path(), "nope", &[], 1);
    s.argv = vec!["tebako-bench-definitely-not-a-binary-9x7".to_string()];
    let mut sampler = Sampler::new();
    let err = sampler.run(&s).unwrap_err();
    assert!(
        err.message.contains("cannot spawn"),
        "named error expected, got: {}",
        err.message
    );
}

#[test]
fn ram_total_bytes_reports_a_sane_figure() {
    let ram = tebako_bench::ram_total_bytes().unwrap();
    assert!(
        ram >= 512 * 1024 * 1024,
        "runner.ram_bytes {ram} below any CI runner"
    );
}
