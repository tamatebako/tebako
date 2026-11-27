//! The witnessed transparency log's verification transcript (spec 41,
//! tebako#1103), pinned as golden fixtures: `tebako registry log verify`
//! end to end against checked-in checkpoints whose two signatures were
//! minted by two keys held in two separate custodies — the head key's
//! and the witness key's (docs/registry-witness-custody.md).
//!
//! One directory per case under `tests/fixtures/registry-log/`:
//!
//! - `registry.yaml` — the document the checkpoint seals;
//! - `registry.log`  — the checkpoint(s): the witnessed head, the
//!   single-signature head, or the head whose witness seal was forged
//!   after witnessing;
//! - `expected.txt`  — the exact stdout transcript;
//! - `exit.txt`      — the expected exit code.
//!
//! The byte pin is the transcript; the per-case marker assertions below
//! are the SEMANTICS the bytes must keep meaning: a checkpoint carrying
//! two valid signatures verifies (`witnessed`), a single-signature
//! checkpoint is named as such (never silently downgraded), and a forged
//! second signature fails with the named finding in the signature class.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/registry-log")
}

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-cli-registry-log-transcript-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The verifier's home: both public halves pinned (the witness's unknown-
/// key case is covered by the unit suite), no secrets. The pins ride the
/// same store the binary's verification keyring reads — spec 09 §9.1's
/// layout, mirrored for the witness exactly like the custody doc says.
fn verifier_home(dir: &Path) -> PathBuf {
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    // Pre-stamp the store layout (spec 18 C13's `layout-version` file):
    // the binary's first-run migration note is legitimate output, but it
    // is not the transcript under test.
    fs::write(home.join("layout-version"), "1\n").unwrap();
    for name in ["primary.pub.asc", "witness.pub.asc"] {
        let pubkey = fs::read(fixtures().join("keys").join(name))
            .unwrap_or_else(|e| panic!("the fixture key {name} must exist: {e}"));
        tebako_signer::register_trusted(&home, &pubkey)
            .unwrap_or_else(|e| panic!("cannot pin {name}: {e}"));
    }
    home
}

/// One case, run the way a user runs it: the real binary, a clean
/// TEBAKO_HOME, the checkpoint and registry by their relative names so
/// the transcript is byte-stable across machines. The CLI's version
/// banner rides stdout ahead of the transcript — it is the preamble,
/// not the transcript, and the golden contract does not pin it (a
/// version bump must never break this fixture).
fn run_case(case_dir: &Path) -> (i32, String) {
    let work = scratch(case_dir.file_name().unwrap().to_str().unwrap());
    for file in ["registry.yaml", "registry.log"] {
        fs::copy(case_dir.join(file), work.join(file))
            .unwrap_or_else(|e| panic!("cannot stage {file}: {e}"));
    }
    let home = verifier_home(&work);
    let out = Command::new(tebako_bin())
        .args([
            "registry",
            "log",
            "verify",
            "registry.log",
            "--registry",
            "registry.yaml",
        ])
        .env("TEBAKO_HOME", &home)
        .current_dir(&work)
        .output()
        .unwrap_or_else(|e| panic!("spawn failed: {e}"));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.is_empty(),
        "the transcript belongs to stdout: {stderr}"
    );
    let text = text
        .strip_prefix("Tebako executable packager version ")
        .and_then(|rest| rest.split_once('\n').map(|(_, t)| t.to_string()))
        .unwrap_or(text);
    let code = out.status.code().unwrap_or(-1);
    let expected = fs::read_to_string(case_dir.join("expected.txt")).unwrap();
    assert_eq!(
        text, expected,
        "{}: the verification transcript drifted\n--- expected ---\n{expected}\n--- actual ---\n{text}",
        case_dir.display()
    );
    let expected_code: i32 = fs::read_to_string(case_dir.join("exit.txt"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        code,
        expected_code,
        "{}: exit code drifted",
        case_dir.display()
    );
    let _ = fs::remove_dir_all(&work);
    (code, text)
}

#[test]
fn a_checkpoint_carrying_two_valid_signatures_verifies() {
    let (code, text) = run_case(&fixtures().join("witnessed"));
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("signature valid, witness witnessed"),
        "{text}"
    );
    assert!(text.contains("log verify: sound"), "{text}");
}

#[test]
fn a_single_signature_checkpoint_is_named_as_such() {
    let (code, text) = run_case(&fixtures().join("single-signature"));
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("witness single-signature"), "{text}");
    assert!(text.contains("SINGLE-SIGNATURE checkpoint"), "{text}");
    // Named, never silently downgraded — and never a failure by itself:
    // the honest state of a pre-witness log (spec 41 §5).
    assert!(text.contains("log verify: sound"), "{text}");
}

#[test]
fn a_forged_second_signature_fails_with_the_named_finding() {
    let (code, text) = run_case(&fixtures().join("forged-witness"));
    assert_eq!(code, 71, "{text}");
    assert!(
        text.contains("COUNTERSIGNATURE DOES NOT VALIDATE"),
        "{text}"
    );
    // The primary re-signed over the edit, so it still validates: the
    // witness seal is exactly what names the fork — never degraded to
    // `single-signature`.
    assert!(text.contains("signature valid, witness invalid"), "{text}");
    assert!(!text.contains("log verify: sound"), "{text}");
}
