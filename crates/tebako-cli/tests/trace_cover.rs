//! `tebako trace cover` golden parity (spec 25 §6.3, phase T3): the
//! fixtures under `tests/fixtures/correlate/` are a BYTE-VERBATIM copy of
//! retrace's shared golden tree (tools/correlate/golden/, SSOT:
//! riboseinc/retrace — the README.md in the fixture root is the parity
//! contract). Every case must produce `expected.txt` on stdout and the
//! `exit.txt` code when run as:
//!
//!     tebako trace cover --inside <case>/inside.json \
//!                        --outside <case>/outside.json \
//!                        --prefix <contents of prefix.txt> [options.txt...]
//!
//! The comparison is on raw stdout bytes (the version banner rides stderr
//! for this subcommand — main.rs's machine-contract rule). stderr is
//! outside the contract by design. Re-sync the fixtures with upstream by
//! re-copying the tree; a diff anywhere is a parity regression.

use std::path::{Path, PathBuf};
use std::process::Command;

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/correlate")
}

/// The golden runner's chomp: strip trailing LF/CR from a line file.
fn read_line_file(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.trim_end_matches(['\n', '\r']).to_string())
}

struct Run {
    rc: i32,
    stdout: Vec<u8>,
    stderr: String,
}

fn trace_cover(args: &[String]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_tebako"))
        .arg("trace")
        .arg("cover")
        .args(args)
        .output()
        .unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: out.stdout,
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn golden_cases_match_byte_for_byte() {
    let dir = cases_dir();
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(cases.len() >= 10, "the golden tree ships 10 cases");

    for case in cases {
        let name = case.file_name().unwrap().to_string_lossy().into_owned();
        let prefix = read_line_file(&case.join("prefix.txt"))
            .unwrap_or_else(|| panic!("{name}: prefix.txt missing"));
        let want_stdout = std::fs::read(case.join("expected.txt")).unwrap();
        let want_exit: i32 = read_line_file(&case.join("exit.txt"))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("{name}: exit.txt missing or not a number"));
        // options.txt is optional: verbatim extra flags (--pid N,
        // --window SECS, --exclude-probes), word-split like the shell
        // splice in retrace's golden_runner.
        let options: Vec<String> = read_line_file(&case.join("options.txt"))
            .map(|s| s.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();

        let mut args = vec![
            "--inside".to_string(),
            case.join("inside.json").to_string_lossy().into_owned(),
            "--outside".to_string(),
            case.join("outside.json").to_string_lossy().into_owned(),
            "--prefix".to_string(),
            prefix,
        ];
        args.extend(options);

        let run = trace_cover(&args);
        assert_eq!(
            run.stdout,
            want_stdout,
            "{name}: stdout mismatch\n--- expected ---\n{}\n--- actual ---\n{}\nstderr: {}",
            String::from_utf8_lossy(&want_stdout),
            String::from_utf8_lossy(&run.stdout),
            run.stderr
        );
        assert_eq!(
            run.rc, want_exit,
            "{name}: exit code mismatch (stderr: {})",
            run.stderr
        );
        // The spec 25 §6.3 coverage block names the producing layer and
        // the per-surface percentages on stderr (outside the contract).
        assert!(
            run.stderr.contains("outside capture layer:"),
            "{name}: the stderr coverage block names the producing layer: {}",
            run.stderr
        );
    }
}

#[test]
fn cover_usage_and_io_errors_exit_2() {
    // Missing the required trio.
    let run = trace_cover(&[]);
    assert_eq!(run.rc, 2, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("usage: tebako trace cover"));

    // A prefix that is not a path.
    let run = trace_cover(&[
        "--inside".to_string(),
        "x".to_string(),
        "--outside".to_string(),
        "y".to_string(),
        "--prefix".to_string(),
        "notapath".to_string(),
    ]);
    assert_eq!(run.rc, 2, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("--prefix is not a path"));

    // An unreadable inside capture.
    let dir = std::env::temp_dir().join(format!("tebako-cover-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let missing = dir.join("nope.json");
    let run = trace_cover(&[
        "--inside".to_string(),
        missing.to_string_lossy().into_owned(),
        "--outside".to_string(),
        missing.to_string_lossy().into_owned(),
        "--prefix".to_string(),
        "/mnt/tfs".to_string(),
    ]);
    assert_eq!(run.rc, 2, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("cannot read"), "stderr: {}", run.stderr);

    // stdout stays clean on every error path (the report is the only
    // stdout writer).
    let run = trace_cover(&[]);
    assert!(run.stdout.is_empty());
}

/// A tebako-bus JSONL inside stream (trace-event.yaml grammar) drives
/// the same correlation: the pid/tid fields carry numbers, the string
/// `ts` simply never feeds --window (documented in cover.rs).
#[test]
fn cover_consumes_a_bus_jsonl_inside_stream() {
    let dir = std::env::temp_dir().join(format!("tebako-cover-jsonl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let inside = dir.join("inside.jsonl");
    let outside = dir.join("outside.json");
    std::fs::write(
        &inside,
        concat!(
            "{\"v\":1,\"ts\":\"2026-08-20T01:00:00.000000Z\",\"pid\":42,\"tid\":1,\"op\":\"open\",\"path\":\"/mnt/tfs/served.dat\",\"verdict\":\"image:/tfs\",\"detail\":{},\"dur_us\":3}\n",
            "{\"v\":1,\"ts\":\"2026-08-20T01:00:01.000000Z\",\"pid\":42,\"tid\":1,\"op\":\"stat\",\"path\":\"/mnt/tfs/also-served\",\"verdict\":\"image:/tfs\",\"detail\":{},\"dur_us\":2}\n",
        ),
    )
    .unwrap();
    std::fs::write(
        &outside,
        concat!(
            "[\n",
            "{ \"time\": 101, \"pid\": 42, \"tid\": 2, \"message\": { \"func\": \"open\", \"params\": { \"path\": \"/mnt/tfs/served.dat\" } } },\n",
            "{ \"time\": 102, \"pid\": 42, \"tid\": 3, \"message\": { \"func\": \"open\", \"params\": { \"path\": \"/mnt/tfs/escape.dat\" } } }\n",
            "]\n",
        ),
    )
    .unwrap();
    let run = trace_cover(&[
        "--inside".to_string(),
        inside.to_string_lossy().into_owned(),
        "--outside".to_string(),
        outside.to_string_lossy().into_owned(),
        "--prefix".to_string(),
        "/mnt/tfs".to_string(),
    ]);
    assert_eq!(run.rc, 1, "stderr: {}", run.stderr);
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        "escape /mnt/tfs/escape.dat func=open tid=3 pid=42 class=read\n"
    );
}
