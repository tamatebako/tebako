//! `tebako trace import procmon` integration (spec 25 §6.2, the rest of
//! phase T3). The parity contract, end to end: every golden correlate
//! case that ships an `outside.csv` (a byte-verbatim procmon export —
//! the upstream CTest regenerates `outside.json` from it with
//! procmon2retrace and diffs) must
//!
//!   1. convert to that case's `outside.json`, byte for byte —
//!      `tebako trace import procmon outside.csv` IS upstream's
//!      conversion (the document-level parity pin), and
//!   2. drive `tebako trace cover` to the case's golden verdict when
//!      the conversion is the `--outside` stream: `expected.txt` on
//!      stdout, byte for byte, and the `exit.txt` code.
//!
//! stdout is the machine contract for both verbs (the version banner
//! rides stderr — main.rs's machine_stdout rule); stderr stays outside
//! the contract by design.

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

fn tebako(args: &[String]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_tebako"))
        .args(args)
        .output()
        .unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: out.stdout,
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn trace_import(csv: &Path) -> Run {
    tebako(&[
        "trace".to_string(),
        "import".to_string(),
        "procmon".to_string(),
        csv.to_string_lossy().into_owned(),
    ])
}

/// The cases shipping a procmon CSV (today exactly 06-libsass-importer;
/// the loop picks up any future one).
fn csv_cases() -> Vec<PathBuf> {
    let mut cases: Vec<PathBuf> = std::fs::read_dir(cases_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("outside.csv").is_file())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "the golden tree ships an outside.csv case");
    cases
}

#[test]
fn golden_csv_converts_byte_for_byte_and_drives_cover() {
    for case in csv_cases() {
        let name = case.file_name().unwrap().to_string_lossy().into_owned();

        // 1. The document-level parity pin: the conversion IS
        //    outside.json, byte for byte (upstream's own conversion).
        let csv = case.join("outside.csv");
        let run = trace_import(&csv);
        let want_json = std::fs::read(case.join("outside.json")).unwrap();
        assert_eq!(run.rc, 0, "{name}: import exit code (stderr: {})", run.stderr);
        assert_eq!(
            run.stdout,
            want_json,
            "{name}: the conversion drifted from outside.json\n--- expected ---\n{}\n--- actual ---\n{}",
            String::from_utf8_lossy(&want_json),
            String::from_utf8_lossy(&run.stdout)
        );
        assert!(
            run.stderr.contains("entries="),
            "{name}: the stderr summary names the counts: {}",
            run.stderr
        );

        // 2. The converted stream is cover's --outside: the golden
        //    verdict reproduces (the process-substitution flow,
        //    file-mediated here).
        let dir = std::env::temp_dir().join(format!(
            "tebako-import-e2e-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let converted = dir.join("outside.json");
        std::fs::write(&converted, &run.stdout).unwrap();

        let prefix = read_line_file(&case.join("prefix.txt")).unwrap();
        let want_stdout = std::fs::read(case.join("expected.txt")).unwrap();
        let want_exit: i32 = read_line_file(&case.join("exit.txt"))
            .and_then(|s| s.parse().ok())
            .unwrap();
        let options: Vec<String> = read_line_file(&case.join("options.txt"))
            .map(|s| s.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();

        let mut args = vec![
            "trace".to_string(),
            "cover".to_string(),
            "--inside".to_string(),
            case.join("inside.json").to_string_lossy().into_owned(),
            "--outside".to_string(),
            converted.to_string_lossy().into_owned(),
            "--prefix".to_string(),
            prefix,
        ];
        args.extend(options);
        let run = tebako(&args);
        assert_eq!(
            run.stdout,
            want_stdout,
            "{name}: cover over the converted stream — stdout mismatch\n--- expected ---\n{}\n--- actual ---\n{}\nstderr: {}",
            String::from_utf8_lossy(&want_stdout),
            String::from_utf8_lossy(&run.stdout),
            run.stderr
        );
        assert_eq!(
            run.rc, want_exit,
            "{name}: cover over the converted stream — exit code mismatch (stderr: {})",
            run.stderr
        );
    }
}

#[test]
fn import_usage_and_io_errors_exit_2() {
    // No format token / no csv.
    let run = tebako(&["trace".to_string(), "import".to_string()]);
    assert_eq!(run.rc, 2, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("usage: tebako trace import procmon"));

    // An unknown format names itself.
    let run = tebako(&[
        "trace".to_string(),
        "import".to_string(),
        "strace".to_string(),
        "x.log".to_string(),
    ]);
    assert_eq!(run.rc, 2, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("unknown import format 'strace'"));

    // An unreadable csv.
    let missing = std::env::temp_dir().join(format!("tebako-import-nope-{}.csv", std::process::id()));
    let run = trace_import(&missing);
    assert_eq!(run.rc, 2, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("cannot read"), "stderr: {}", run.stderr);

    // stdout stays clean on every error path (the document is the only
    // stdout writer).
    let run = tebako(&["trace".to_string(), "import".to_string()]);
    assert!(run.stdout.is_empty());
}

#[test]
fn import_zero_entries_exits_1_with_the_empty_document() {
    // A header-only capture converts to the empty array document and
    // exits 1 (upstream's `(entries > 0) ? 0 : 1`).
    let dir = std::env::temp_dir().join(format!("tebako-import-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("header-only.csv");
    std::fs::write(
        &csv,
        "\"Time of Day\",\"Process Name\",\"PID\",\"Operation\",\"Path\",\"Result\",\"Detail\"\r\n",
    )
    .unwrap();
    let run = trace_import(&csv);
    assert_eq!(run.rc, 1, "stderr: {}", run.stderr);
    assert_eq!(run.stdout, b"[\n]\n");
    assert!(run.stderr.contains("entries=0"), "stderr: {}", run.stderr);
}
