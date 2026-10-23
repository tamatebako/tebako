//! `tfs blksum` — the lazy-mount digest sidecar (spec 39 §3), produced by
//! the shared emitter every other caller uses.
//!
//! The property the runtime factories depend on (tebako#868): the sidecar
//! describes the SERVED image, so a sidecar derived from the published
//! bytes must parse back into the same geometry and digest the image
//! really has.

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::TempDir;

fn rust_tfs() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tfs"))
}

fn run(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let mut cmd = Command::new(rust_tfs());
    cmd.args(args).current_dir(cwd);
    let out = cmd.output().expect("spawn tfs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn image(w: &TempDir) -> PathBuf {
    let src = w.0.join("app");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("one.txt"), "one").unwrap();
    std::fs::write(src.join("two.txt"), "two").unwrap();
    let img = w.0.join("app.tfs");
    let (code, _, err) = run(
        &[
            "mkimage",
            "--format",
            "limnifs",
            src.to_str().unwrap(),
            "--output",
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(code, 0, "mkimage failed: {err}");
    img
}

#[test]
fn the_sidecar_lands_beside_the_image_and_parses_back() {
    let w = TempDir::new("blksum-beside");
    let img = image(&w);
    let (code, out, err) = run(&["blksum", img.to_str().unwrap()], &w.0);
    assert_eq!(code, 0, "blksum failed: {err}");

    let sidecar = img.with_extension("tfs.blksum.json");
    assert!(sidecar.exists(), "no sidecar at {}", sidecar.display());

    let text = std::fs::read_to_string(&sidecar).unwrap();
    let parsed = tpkg::lazy::Blksum::parse(&text).expect("the sidecar is a spec 39 §3 document");
    let bytes = std::fs::read(&img).unwrap();
    assert_eq!(parsed.size_bytes, bytes.len() as u64);
    assert_eq!(
        parsed.group_count(),
        tpkg::lazy::group_count_for(bytes.len() as u64)
    );
    assert_eq!(parsed.sha256, tpkg::lazy::sha256_hex(&bytes));

    // The progress line is the `.sha256` convention a factory appends to
    // its own metadata: `<sha256>  <filename>`.
    let line = out.trim();
    let (sha, name) = line.split_once("  ").expect("`<sha>  <name>`");
    assert_eq!(name, "app.tfs.blksum.json");
    assert_eq!(sha, tpkg::lazy::sha256_hex(text.as_bytes()));
}

#[test]
fn out_places_the_sidecar_elsewhere() {
    let w = TempDir::new("blksum-out");
    let img = image(&w);
    let target = w.0.join("published.tfs.blksum.json");
    let (code, out, err) = run(
        &[
            "blksum",
            img.to_str().unwrap(),
            "--out",
            target.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(code, 0, "blksum --out failed: {err}");
    assert!(target.exists());
    assert!(out.contains("published.tfs.blksum.json"));
}

#[test]
fn the_derivation_is_deterministic() {
    let w = TempDir::new("blksum-deterministic");
    let img = image(&w);
    let first = w.0.join("first.json");
    let second = w.0.join("second.json");
    assert_eq!(
        run(
            &[
                "blksum",
                img.to_str().unwrap(),
                "--out",
                first.to_str().unwrap()
            ],
            &w.0
        )
        .0,
        0
    );
    assert_eq!(
        run(
            &[
                "blksum",
                img.to_str().unwrap(),
                "--out",
                second.to_str().unwrap()
            ],
            &w.0
        )
        .0,
        0
    );
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap(),
        "the same image must derive byte-identical sidecars"
    );
}

#[test]
fn a_zero_byte_image_is_refused_by_name() {
    let w = TempDir::new("blksum-empty");
    let img = w.0.join("empty.tfs");
    std::fs::write(&img, b"").unwrap();
    let (code, _, err) = run(&["blksum", img.to_str().unwrap()], &w.0);
    assert_eq!(code, 65, "a 0-byte image is a manifest-class refusal");
    assert!(err.contains("0-byte image has no blksum"), "{err}");
}

#[test]
fn a_missing_image_is_an_io_class_error() {
    let w = TempDir::new("blksum-missing");
    let (code, _, err) = run(&["blksum", w.0.join("nope.tfs").to_str().unwrap()], &w.0);
    assert_eq!(code, 74, "a missing file is the IO class");
    assert!(err.contains("cannot read"), "{err}");
}
