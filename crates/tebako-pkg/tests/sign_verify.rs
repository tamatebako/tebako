//! tebako-pkg sign/verify CLI tests (release tooling): detached .asc per
//! artifact + signed SHA256SUMS; verify against the trusted keyring with
//! named Trusted/Untrusted/Invalid outcomes.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tebako_contract_tests::TempDir;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn bin() -> PathBuf {
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .canonicalize()
        .unwrap();
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tebako-pkg");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tebako-pkg binary not built")
}

fn test_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-sign-test-home-{}-{name}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str], cwd: &Path, home: &Path) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("TEBAKO_HOME", home)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn write_artifacts(w: &TempDir) -> (PathBuf, PathBuf) {
    let a = w.0.join("a.bin");
    let b = w.0.join("b.bin");
    std::fs::write(&a, b"artifact A payload").unwrap();
    std::fs::write(&b, b"artifact B payload").unwrap();
    (a, b)
}

fn sha256(data: &[u8]) -> String {
    use sha2::Digest;
    let d = sha2::Sha256::digest(data);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn sign_then_verify_all_trusted() {
    let w = TempDir::new("sv1");
    let (a, b) = write_artifacts(&w);
    let home = test_home("sv1");

    let (rc, out, err) = run(
        &[
            "sign",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ],
        &w.0,
        &home,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "sign: {out}");
    for f in ["a.bin.asc", "b.bin.asc", "SHA256SUMS", "SHA256SUMS.asc"] {
        assert!(w.0.join(f).exists(), "missing {f}");
    }

    // the checksums file lists both artifacts with the right digests
    let sums = std::fs::read_to_string(w.0.join("SHA256SUMS")).unwrap();
    assert!(sums.contains(&format!("{}  a.bin", sha256(b"artifact A payload"))), "{sums}");
    assert!(sums.contains(&format!("{}  b.bin", sha256(b"artifact B payload"))), "{sums}");

    let (rc, out, err) = run(
        &["verify", a.to_str().unwrap(), b.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "verify: {out}");
    assert!(out.contains("a.bin: trusted"), "{out}");
    assert!(out.contains("b.bin: trusted"), "{out}");
}

#[test]
fn verify_reports_invalid_and_untrusted() {
    let w = TempDir::new("sv2");
    let (a, b) = write_artifacts(&w);
    let home = test_home("sv2");
    let (rc, _, _) = run(&["sign", a.to_str().unwrap(), b.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0);

    // tampered artifact -> INVALID SIGNATURE + exit 1
    std::fs::write(&a, b"artifact A tampered").unwrap();
    let (rc, out, _) = run(&["verify", a.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("INVALID SIGNATURE"), "{out}");

    // a keyring that lacks the signer -> UNTRUSTED + exit 1
    let stranger_home = test_home("stranger");
    let (rc, out, _) = run(&["verify", b.to_str().unwrap()], &w.0, &stranger_home);
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("UNTRUSTED"), "{out}");
}

#[test]
fn sign_with_key_file() {
    let w = TempDir::new("sv3");
    let (a, _) = write_artifacts(&w);
    let home = test_home("sv3");

    // produce a press key, then use its secret via --key-file
    let press = tebako_signer::press_local_key(&home).expect("press key");
    let key_file = w.0.join("root.key");
    std::fs::write(&key_file, &press.secret_key).unwrap();

    let (rc, out, err) = run(
        &[
            "sign",
            "--key-file",
            key_file.to_str().unwrap(),
            a.to_str().unwrap(),
        ],
        &w.0,
        &home,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "sign --key-file: {out}");

    let (rc, out, _) = run(&["verify", a.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("a.bin: trusted"), "{out}");
}

#[test]
fn verify_sums_file_as_artifact() {
    let w = TempDir::new("sv4");
    let (a, _) = write_artifacts(&w);
    let home = test_home("sv4");
    let (rc, _, _) = run(&["sign", a.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0);

    // SHA256SUMS itself is signed and verifies
    let (rc, out, _) = run(&["verify", "SHA256SUMS"], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("SHA256SUMS: trusted"), "{out}");

    // tampering the sums file invalidates it
    let mut sums = std::fs::read_to_string(w.0.join("SHA256SUMS")).unwrap();
    sums = sums.replacen(&sha256(b"artifact A payload")[..8], "00000000", 1);
    std::fs::write(w.0.join("SHA256SUMS"), &sums).unwrap();
    let (rc, out, _) = run(&["verify", "SHA256SUMS"], &w.0, &home);
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("INVALID SIGNATURE"), "{out}");
}
