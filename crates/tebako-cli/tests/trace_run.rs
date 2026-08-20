//! `tebako trace run` e2e (spec 25 §4 — the discovery front-end): the
//! package runs under TEBAKO_JAIL=record with TEBAKO_TRACE naming the
//! capture, the payload's exit code is the command's exit code, and the
//! capture synthesizes into the suggested-manifest draft. The fixture
//! "package" is a shebang script (the kernel execs it; it reports the
//! env and can hand-write a bus event into the capture). Unix only
//! (shebang exec), like tests/run.rs.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn workdir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tebako-trace-e2e-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The probe: report the trace/jail env, echo the payload args, exit 17.
const PROBE: &str = "#!/bin/sh\n\
echo \"TRACE=${TEBAKO_TRACE-UNSET}\"\n\
echo \"JAIL=${TEBAKO_JAIL-UNSET}\"\n\
echo \"ARGS=$*\"\n\
exit 17\n";

/// The emitter: one well-formed jail event into the capture (the shape
/// tfs::trace renders), then exit 0.
const EMITTER: &str = "#!/bin/sh\n\
printf '%s\\n' '{\"v\":1,\"ts\":\"2026-08-19T01:02:03.000000Z\",\"pid\":7,\"tid\":1,\"op\":\"jail\",\"path\":\"/etc/hostname\",\"verdict\":\"record\",\"detail\":{\"access\":\"read\"},\"dur_us\":5}' >> \"$TEBAKO_TRACE\"\n\
exit 0\n";

fn script_pkg(dir: &Path, name: &str, body: &str) -> PathBuf {
    let pkg = dir.join(name);
    std::fs::write(&pkg, body).unwrap();
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&pkg, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    pkg
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

fn tebako_trace(args: &[&str]) -> Run {
    let out = Command::new(tebako_bin())
        .arg("trace")
        .args(args)
        .output()
        .unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn line<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .unwrap_or_else(|| panic!("{key} missing from stdout:\n{stdout}"))
}

#[test]
fn trace_run_arms_the_bus_and_the_record_policy_and_propagates_the_exit_code() {
    let dir = workdir("env");
    let pkg = script_pkg(&dir, "pkg", PROBE);
    let capture = dir.join("capture.jsonl");

    let r = tebako_trace(&[
        "run",
        pkg.to_str().unwrap(),
        "--capture",
        capture.to_str().unwrap(),
        "--",
        "alpha",
        "--beta",
    ]);
    // The payload's exit code IS the command's (17 from the probe).
    assert_eq!(r.rc, 17, "stdout: {} stderr: {}", r.stdout, r.stderr);
    // The env reached the payload: the bus channel and the record jail.
    assert_eq!(line(&r.stdout, "TRACE="), capture.to_string_lossy());
    assert_eq!(line(&r.stdout, "JAIL="), "record");
    assert_eq!(line(&r.stdout, "ARGS="), "alpha --beta");
    // A shell script mounts nothing: no bus, no capture — the draft
    // degrades to the no-events note (observability never gates).
    assert!(
        r.stdout.contains("No interception events were captured"),
        "stdout: {}",
        r.stdout
    );
    assert!(r.stdout.contains("host: []"), "stdout: {}", r.stdout);
    assert!(r.stderr.contains("no capture at"), "stderr: {}", r.stderr);
}

#[test]
fn trace_run_synthesizes_the_capture_into_the_draft() {
    let dir = workdir("synthesize");
    let pkg = script_pkg(&dir, "pkg", EMITTER);
    let capture = dir.join("capture.jsonl");
    let out = dir.join("draft.yaml");

    let r = tebako_trace(&[
        "run",
        pkg.to_str().unwrap(),
        "--capture",
        capture.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(r.rc, 0, "stdout: {} stderr: {}", r.stdout, r.stderr);
    // --out: the draft is the file, stdout stays the payload's (empty).
    assert!(!r.stdout.contains("needs:"), "stdout: {}", r.stdout);
    let draft = std::fs::read_to_string(&out).unwrap();
    assert!(draft.contains("1 interception event(s)"), "{draft}");
    assert!(draft.contains("path: \"/etc/hostname\""), "{draft}");
    assert!(draft.contains("access: ro"), "{draft}");
    assert!(draft.contains("observed: 1 read, 0 write"), "{draft}");
    assert!(
        draft.contains("never edits a manifest by itself"),
        "{draft}"
    );
    // The capture was left in place and named on stderr.
    assert!(capture.is_file(), "the capture survives for replay");
    assert!(
        r.stderr.contains(&capture.display().to_string()),
        "stderr: {}",
        r.stderr
    );
}

#[test]
fn trace_run_defaults_the_capture_into_the_temp_dir() {
    let dir = workdir("default-capture");
    let pkg = script_pkg(&dir, "pkg", PROBE);

    let r = tebako_trace(&["run", pkg.to_str().unwrap()]);
    assert_eq!(r.rc, 17, "stderr: {}", r.stderr);
    let named = line(&r.stdout, "TRACE=").to_string();
    assert!(
        named.contains("tebako-trace-") && named.ends_with(".jsonl"),
        "the default capture name: {named}"
    );
}

#[test]
fn trace_run_rejects_a_missing_package_and_bad_options() {
    let dir = workdir("usage");
    let missing = dir.join("nope");

    let r = tebako_trace(&["run", missing.to_str().unwrap()]);
    assert_eq!(r.rc, 127, "stdout: {} stderr: {}", r.stdout, r.stderr);
    assert!(
        r.stdout.contains("package not found"),
        "stdout: {}",
        r.stdout
    );

    let pkg = script_pkg(&dir, "pkg", PROBE);
    let r = tebako_trace(&["run", pkg.to_str().unwrap(), "--frobnicate"]);
    assert_eq!(r.rc, 1);
    assert!(
        r.stderr.contains("unknown trace run option"),
        "stderr: {}",
        r.stderr
    );

    let r = tebako_trace(&["run", pkg.to_str().unwrap(), "--capture"]);
    assert_eq!(r.rc, 1);
    assert!(
        r.stderr.contains("requires a value"),
        "stderr: {}",
        r.stderr
    );

    // Bare `trace` and the later-milestone subcommands name themselves.
    let r = tebako_trace(&[]);
    assert_eq!(r.rc, 1);
    assert!(
        r.stderr.contains("trace subcommand expected"),
        "stderr: {}",
        r.stderr
    );
    for sub in ["explain", "cover"] {
        let r = tebako_trace(&[sub]);
        assert_eq!(r.rc, 1);
        assert!(
            r.stderr.contains(&format!(
                "'tebako trace {sub}' is a later tebako-rs milestone"
            )),
            "stderr: {}",
            r.stderr
        );
    }
}
