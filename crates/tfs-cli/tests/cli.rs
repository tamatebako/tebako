//! CLI-level tests: flag forms, help, unknown commands, and the
//! beyond-C++ --json info flag.

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::TempDir;

fn bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tfs");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tfs binary not built")
}

fn run(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn fixture_zip(w: &TempDir) -> PathBuf {
    let z = w.0.join("t.zip");
    tebako_contract_tests::build_fixture_zip(&z);
    z
}

#[test]
fn help_and_unknown() {
    let w = TempDir::new("cli");
    let (rc, out, _) = run(&["help"], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("tfs - generic VFS image tool (tebako)"));
    assert!(out.contains("mkimage"));
    assert!(out.contains("tebako-pkg"));

    let (rc, _, err) = run(&["frob"], &w.0);
    assert_eq!(rc, 1);
    assert_eq!(
        err,
        "Error: Unknown command: frob\nUse 'tfs help' for usage information\n"
    );
}

#[test]
fn flag_forms() {
    let w = TempDir::new("cli2");
    let z = fixture_zip(&w);
    let zs = z.to_str().unwrap();

    // Combined short flags.
    let (rc, out, _) = run(&["ls", "-rl", zs], &w.0);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("-rw-r--r--"), "{out}");
    assert!(out.contains("hello.txt"), "{out}");

    // --flag=value and separate forms.
    let dest = w.0.join("out");
    let (rc, _, _) = run(
        &["extract", &format!("--dest={}", dest.display()), zs],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert!(dest.join("content/hello.txt").is_file());

    // -d with separate value.
    let dest2 = w.0.join("out2");
    let (rc, _, _) = run(&["extract", "-d", dest2.to_str().unwrap(), zs], &w.0);
    assert_eq!(rc, 0);
    assert!(dest2.join("content/hello.txt").is_file());

    // Unknown option.
    let (rc, _, err) = run(&["ls", "--frobnicate", zs], &w.0);
    assert_eq!(rc, 1);
    assert!(err.contains("unknown option"), "{err}");
}

#[test]
fn quiet_extract_prints_nothing() {
    let w = TempDir::new("cli3");
    let z = fixture_zip(&w);
    let dest = w.0.join("out");
    let (rc, out, _) = run(
        &[
            "extract",
            "-q",
            "-d",
            dest.to_str().unwrap(),
            z.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(out, "");
    assert!(dest.join("content/hello.txt").is_file());
}

#[test]
fn info_json_dwarfs() {
    let w = TempDir::new("cli4");
    let img = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures/simple.dwarfs");

    // --backend-json on a dwarfs image: backend metadata JSON (item 24;
    // this was `--json` before spec 15 made `--json` the info document).
    let (rc, out, _) = run(&["info", "--backend-json", img.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("\"version\""), "{out}");
    assert!(out.contains("\"block_size\""), "{out}");

    // --json is now the spec-15 info document (info_schema 1).
    let (rc, out, _) = run(&["info", "--json", img.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("\"info_schema\": 1"), "{out}");
    assert!(out.contains("\"kind\": \"image\""), "{out}");

    // --backend-json on a zip: ENOTSUP path.
    let z = fixture_zip(&w);
    let (rc, _, err) = run(&["info", "--backend-json", z.to_str().unwrap()], &w.0);
    assert_eq!(rc, 1);
    assert!(err.contains("not available"), "{err}");
}
