//! Golden comparisons against the C++ tebakofs oracle for the overlapping
//! commands (ls/tree/cat/stat/extract/find/info), including the zip
//! explicit-entry edge semantics and error paths.
//!
//! The oracle is located via TEBAKOFS_CPP, the libtfs build tree, or PATH;
//! tests skip without it. The libtfs v0.13.0 RELEASE binary cannot mount
//! SquashFS (a capability gap of that build): sqfs mount comparisons
//! probe-skip with a note.

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::TempDir;

fn cpp_tebakofs() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TEBAKOFS_CPP") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let well_known = PathBuf::from("/Users/mulgogi/src/tamatebako/libtfs-pkgwt/build/tebakofs");
    if well_known.is_file() {
        return Some(well_known);
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let cand = PathBuf::from(dir).join("tebakofs");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn rust_tfs() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tfs"))
}

fn run(tool: &Path, args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn tool");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Fixture zips: one WITH explicit dir entries (C++ fixture style), one
/// WITHOUT (python zipfile style — exercises the explicit-entry-only
/// semantics).
fn fixtures(w: &TempDir) -> (PathBuf, PathBuf) {
    let with_dirs = w.0.join("with-dirs.zip");
    tebako_contract_tests::build_zip(
        &with_dirs,
        &["content/", "content/sub/"],
        &[
            ("content/hello.txt", b"Hello, World!".as_slice()),
            ("content/nested.txt", b"Nested file content".as_slice()),
            ("content/sub/deep.txt", b"deep".as_slice()),
        ],
    );
    let no_dirs = w.0.join("no-dirs.zip");
    tebako_contract_tests::build_zip(
        &no_dirs,
        &[],
        &[("content/hello.txt", b"Hello, World!".as_slice())],
    );
    (with_dirs, no_dirs)
}

fn compare(cpp: &Path, rs: &Path, cwd: &Path, args: &[&str]) {
    let a = run(cpp, args, cwd);
    let b = run(rs, args, cwd);
    assert_eq!(a, b, "tfs {:?} must match tebakofs", args);
}

#[test]
fn golden_ls_tree_cat_stat_find() {
    let Some(cpp) = cpp_tebakofs() else {
        eprintln!("skipping golden test: no C++ tebakofs oracle");
        return;
    };
    let rs = rust_tfs();
    let w = TempDir::new("tfs-golden");
    let (with_dirs, no_dirs) = fixtures(&w);
    let z = with_dirs.to_str().unwrap();

    for args in [
        vec!["ls", z].as_slice(),
        &["ls", z, "/content"][..],
        &["ls", "-r", z][..],
        &["ls", "-rl", z][..],
        &["ls", "-l", z, "/content"][..],
        &["ls", z, "/nonexistent"][..],
        &["tree", z][..],
        &["tree", z, "/content"][..],
        &["tree", z, "/nonexistent"][..],
        &["stat", z, "content/hello.txt"][..],
        &["stat", z, "content"][..],
        &["stat", z, "/content/sub"][..],
        &["stat", z, "nonexistent"][..],
        &["cat", z, "content/nested.txt"][..],
        &["cat", z, "content"][..],
        &["cat", z, "nonexistent"][..],
        &["find", z, "*.txt"][..],
        &["find", z, "deep*"][..],
        &["find", z, "nomatch*"][..],
        &["info", z][..],
    ] {
        compare(&cpp, &rs, &w.0, args);
    }

    // The no-explicit-entries zip: C++ lists nothing at root and ENOENTs
    // implicit paths — exact parity on the edge semantics.
    let n = no_dirs.to_str().unwrap();
    for args in [
        &["ls", n][..],
        &["ls", n, "/content"][..],
        &["stat", n, "content"][..],
        &["tree", n][..],
    ] {
        compare(&cpp, &rs, &w.0, args);
    }
}

#[test]
fn golden_extract() {
    let Some(cpp) = cpp_tebakofs() else {
        eprintln!("skipping golden test: no C++ tebakofs oracle");
        return;
    };
    let rs = rust_tfs();
    let w = TempDir::new("tfs-golden-x");
    let (with_dirs, _) = fixtures(&w);
    let z = with_dirs.to_str().unwrap();

    // Whole-archive extraction: stdout identical, trees identical.
    let out_cpp = w.0.join("out-cpp");
    let out_rs = w.0.join("out-rs");
    std::fs::create_dir_all(&out_cpp).unwrap();
    std::fs::create_dir_all(&out_rs).unwrap();
    let a = run(&cpp, &["extract", "-d", out_cpp.to_str().unwrap(), z], &w.0);
    let b = run(&rs, &["extract", "-d", out_rs.to_str().unwrap(), z], &w.0);
    assert_eq!(a.0, b.0, "extract rc");
    assert_eq!(a.2, b.2, "extract stderr");
    // stdout carries the destination path (differs by dir name); normalize.
    assert_eq!(
        a.1.replace(out_cpp.to_str().unwrap(), "<DEST>"),
        b.1.replace(out_rs.to_str().unwrap(), "<DEST>")
    );
    for rel in [
        "content/hello.txt",
        "content/nested.txt",
        "content/sub/deep.txt",
    ] {
        assert_eq!(
            std::fs::read(out_cpp.join(rel)).unwrap(),
            std::fs::read(out_rs.join(rel)).unwrap(),
            "{rel} content"
        );
    }

    // Selected paths (file + dir) and the missing-file warning path.
    for args in [
        &["extract", z, "content/hello.txt", "content"][..],
        &["extract", "-q", z, "nonexistent.txt"][..],
    ] {
        compare(&cpp, &rs, &w.0, args);
    }
}

#[test]
fn golden_info_formats_and_errors() {
    let Some(cpp) = cpp_tebakofs() else {
        eprintln!("skipping golden test: no C++ tebakofs oracle");
        return;
    };
    let rs = rust_tfs();
    let w = TempDir::new("tfs-golden-i");
    let (with_dirs, _) = fixtures(&w);
    let dwarfs = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures/simple.dwarfs");

    for args in [
        &["info", with_dirs.to_str().unwrap()][..],
        &["info", dwarfs.to_str().unwrap()][..],
        &["info", "/nonexistent.zip"][..],
    ] {
        compare(&cpp, &rs, &w.0, args);
    }

    // The release oracle cannot mount SquashFS: probe and skip.
    let sqfs = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures/simple.sqfs");
    let (rc, _, err) = run(&cpp, &["info", sqfs.to_str().unwrap()], &w.0);
    if rc != 0 && err.contains("Unsupported format") {
        eprintln!("note: oracle cannot mount sqfs (release build gap), skipping");
    } else {
        compare(&cpp, &rs, &w.0, &["info", sqfs.to_str().unwrap()]);
    }

    // Unknown magic.
    let junk = w.0.join("junk.bin");
    std::fs::write(&junk, b"\xff\x00\x01garbage".repeat(16)).unwrap();
    compare(&cpp, &rs, &w.0, &["info", junk.to_str().unwrap()]);
}
