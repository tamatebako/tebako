//! `tebako trace explain` integration (spec 25 §5/§7, phase T4): the
//! fixtures under `tests/fixtures/explain/` (the README.md there is the
//! provenance contract — synthetic reproductions of the incident corpus
//! classes, authored against trace-event.yaml) replay through the real
//! binary, and the named red hop must match the incident's hand-derived
//! answer (§7's gate shape).
//!
//! stdout is the diagnosis report alone (the version banner rides
//! stderr — main.rs's machine_stdout rule); exit codes are the
//! trace-verbs convention: 0 clean / 1 red hop / 2 usage-or-IO.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/explain")
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

fn trace_explain(capture: &Path) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_tebako"))
        .arg("trace")
        .arg("explain")
        .arg(capture)
        .output()
        .unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// One fixture's expectation: the exit code and the exact RED/GREEN
/// verdict line (the §7 gate: the named hop matches the incident's
/// hand-derived answer).
struct Case {
    fixture: &'static str,
    exit: i32,
    verdict_line: &'static str,
    /// Extra stdout lines the report must carry (evidence pointers,
    /// bisect candidates).
    must_contain: &'static [&'static str],
}

#[test]
fn explain_replays_the_fixtures_to_the_named_hops() {
    let cases = [
        Case {
            fixture: "env-image-never-mounted",
            exit: 1,
            verdict_line:
                "RED hop: mount — env image never mounted (handoff env lost) [signature: env-image-never-mounted]",
            must_contain: &["no `mount/ok` verdict", "prelude-class stderr"],
        },
        Case {
            fixture: "os-bind-module-not-found",
            exit: 1,
            verdict_line:
                "RED hop: OS bind — the closure resolved and the loader still refused [signature: os-bind-module-not-found]",
            must_contain: &[
                "evidence: event #4",
                "dlopen /tfs/lib/libsass.so verdict=error:126 errno=126",
                "bisect candidates",
                "libsass-deps.dll → /tfs/lib/libsass-deps.dll (materialized)",
                "KERNEL32.dll",
            ],
        },
        Case {
            fixture: "policy-denial",
            exit: 1,
            verdict_line:
                "RED hop: policy — policy denial (the EACCES class) [signature: policy-denial]",
            must_contain: &[
                "evidence: event #3",
                "open /work/repo/private/token verdict=denied:user",
                "related:  event #2",
                "jail /work/repo/private verdict=deny:user",
            ],
        },
        Case {
            fixture: "materialize-error",
            exit: 1,
            verdict_line:
                "RED hop: materialize — exec-cache write failure [signature: materialize-error]",
            must_contain: &["evidence: event #2", "materialize /tfs/bin/tool verdict=error:28 errno=28"],
        },
        Case {
            fixture: "clean-run",
            exit: 0,
            verdict_line: "GREEN: no red hop — every hop's verdict is clean in 8 event(s)",
            must_contain: &[],
        },
    ];

    for case in cases {
        let capture = fixtures_dir().join(format!("{}.jsonl", case.fixture));
        let run = trace_explain(&capture);
        assert_eq!(
            run.rc, case.exit,
            "{}: exit code (stdout: {} stderr: {})",
            case.fixture, run.stdout, run.stderr
        );
        assert!(
            run.stdout.contains(case.verdict_line),
            "{}: the verdict line\nwant: {}\nstdout: {}",
            case.fixture,
            case.verdict_line,
            run.stdout
        );
        for line in case.must_contain {
            assert!(
                run.stdout.contains(line),
                "{}: missing `{line}`\nstdout: {}",
                case.fixture,
                run.stdout
            );
        }
        // The replay line names the hop chain (§5's axis).
        assert!(
            run.stdout
                .contains("hop chain: mount → manifest read → resolve → materialize → OS bind"),
            "{}: the hop chain line\nstdout: {}",
            case.fixture,
            run.stdout
        );
        // The banner never rides stdout (the machine-contract rule).
        assert!(
            !run.stdout.contains("Tebako executable packager version"),
            "{}: the banner leaked to stdout: {}",
            case.fixture,
            run.stdout
        );
    }

    // The clean fixture's crashed tail is dropped with a stderr note,
    // never a failure (spec 25 law 1's leniency on the read side).
    let run = trace_explain(&fixtures_dir().join("clean-run.jsonl"));
    assert!(
        run.stderr.contains("is not a complete event — skipped"),
        "stderr: {}",
        run.stderr
    );
}

#[test]
fn explain_usage_and_io_errors_exit_2() {
    // No capture.
    let out = Command::new(env!("CARGO_BIN_EXE_tebako"))
        .arg("trace")
        .arg("explain")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("usage: tebako trace explain"), "stderr: {stderr}");
    assert!(out.stdout.is_empty());

    // An unknown option.
    let out = Command::new(env!("CARGO_BIN_EXE_tebako"))
        .arg("trace")
        .arg("explain")
        .arg("--verbose")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown option '--verbose'"), "stderr: {stderr}");

    // An unreadable capture.
    let missing = std::env::temp_dir().join(format!("tebako-explain-nope-{}.jsonl", std::process::id()));
    let run = trace_explain(&missing);
    assert_eq!(run.rc, 2, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("cannot read"), "stderr: {}", run.stderr);
    assert!(run.stdout.is_empty());
}
