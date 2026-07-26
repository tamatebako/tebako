//! tebako-bootstrap chain-of-trust tests: sign → verify → tamper matrices
//! (slot sha256 mismatch, unknown signer, invalid signature), the v1
//! legacy acceptance + TEBAECO_REQUIRE_SIGNED hard fail, and the
//! trusted-cache second-run hit.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use tebako_bootstrap::{
    verify_chain_with_home, EX_TEBAKO_SHA, EX_TEBAKO_SIGNATURE, EX_TEBAKO_TRUST,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-boot-test-{}-{name}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(data);
    h.finalize().into()
}

/// Assemble a package file: `BOOTSTRAP` + slot image bytes + trailer
/// (v1 unsigned, or v2 signed with `home`'s press-local key).
/// Returns (package path, parsed manifest).
fn make_package(dir: &Path, home: &Path, image: &[u8], v1: bool) -> (PathBuf, tpkg::Manifest) {
    let bootstrap = b"BOOTSTRAP-BYTES";
    let mut bytes = bootstrap.to_vec();
    bytes.extend_from_slice(image);

    let mut m = tpkg::Manifest {
        version: if v1 {
            tpkg::TPKG_VERSION
        } else {
            tpkg::TPKG_VERSION_2
        },
        package_flags: 0,
        launcher_abi: 1,
        ..Default::default()
    };
    m.slots.push(tpkg::Slot::new(
        bootstrap.len() as u64,
        image.len() as u64,
        tpkg::TPKG_FORMAT_ZIP,
        "/app",
    ));

    if !v1 {
        let press = tebako_signer::press_local_key(home).expect("press key");
        let mut v2 = tpkg::V2Extension::default();
        v2.slot_digests[0] = sha256(image);
        v2.signer_keyid = press.keyid;
        v2.signature = vec![0u8];
        m.v2 = Some(v2);

        let trailer = tpkg::encode_trailer(&m, bytes.len() as u64).unwrap();
        let region = tpkg::v2_signed_region(&trailer).unwrap();
        let sig = tebako_signer::sign_detached(region, &press.secret_key, &press.fingerprint)
            .expect("sign");
        m.v2.as_mut().unwrap().signature = sig;
    }

    bytes.extend_from_slice(&tpkg::encode_trailer(&m, bytes.len() as u64).unwrap());
    let pkg = dir.join("package.bin");
    std::fs::write(&pkg, &bytes).unwrap();

    let mut f = std::fs::File::open(&pkg).unwrap();
    let parsed = tpkg::read_from(&mut f).expect("parse own package");
    (pkg, parsed)
}

fn journal(home: &Path) -> String {
    std::fs::read_to_string(home.join("journal.log")).unwrap_or_default()
}

#[test]
fn signed_and_registered_verifies_ok() {
    let dir = scratch("ok");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", false);
    let press = tebako_signer::press_local_key(&home).unwrap();
    tebako_signer::register_trusted(&home, &press.public_key).unwrap();

    verify_chain_with_home(&pkg, &m, &home).expect("chain verifies");
    assert!(journal(&home).contains("event=v2-trusted"));
}

#[test]
fn tampered_slot_is_sha_mismatch_named() {
    let dir = scratch("sha");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", false);
    let press = tebako_signer::press_local_key(&home).unwrap();
    tebako_signer::register_trusted(&home, &press.public_key).unwrap();

    // flip a byte inside the slot region (slot starts right after BOOTSTRAP-BYTES)
    let mut bytes = std::fs::read(&pkg).unwrap();
    bytes[16] ^= 0xFF;
    std::fs::write(&pkg, &bytes).unwrap();

    let err = verify_chain_with_home(&pkg, &m, &home).unwrap_err();
    assert_eq!(err.code, EX_TEBAKO_SHA);
    assert!(
        err.message.contains("SHA256 mismatch for slot 0"),
        "{}",
        err.message
    );
}

#[test]
fn unknown_signer_is_trust_error_named() {
    let dir = scratch("trust");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", false);
    // no registration: the signer's key is not in the trusted keyring

    let err = verify_chain_with_home(&pkg, &m, &home).unwrap_err();
    assert_eq!(err.code, EX_TEBAKO_TRUST);
    assert!(
        err.message.contains("not in the trusted keyring"),
        "{}",
        err.message
    );
}

#[test]
fn tampered_trailer_region_is_invalid_signature_named() {
    let dir = scratch("invalid");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", false);
    let press = tebako_signer::press_local_key(&home).unwrap();
    tebako_signer::register_trusted(&home, &press.public_key).unwrap();

    // flip a byte inside the trailer's canonical region (the digest array):
    // the signature no longer matches, even before slot hashing
    let mut bytes = std::fs::read(&pkg).unwrap();
    let digest_pos = bytes.len() - tpkg::trailer_len(&m) as usize + 280 + 166;
    bytes[digest_pos] ^= 0xFF;
    std::fs::write(&pkg, &bytes).unwrap();

    let err = verify_chain_with_home(&pkg, &m, &home).unwrap_err();
    assert_eq!(err.code, EX_TEBAKO_SIGNATURE);
    assert!(err.message.contains("INVALID"), "{}", err.message);
}

#[test]
fn v1_legacy_accepted_with_journal_record() {
    let dir = scratch("legacy");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", true);

    verify_chain_with_home(&pkg, &m, &home).expect("legacy accepted");
    assert!(journal(&home).contains("event=legacy-v1-accepted"));
}

#[test]
fn v1_require_signed_is_hard_fail() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch("require");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", true);

    std::env::set_var("TEBAKO_REQUIRE_SIGNED", "1");
    let err = verify_chain_with_home(&pkg, &m, &home).unwrap_err();
    std::env::remove_var("TEBAKO_REQUIRE_SIGNED");

    assert_eq!(err.code, EX_TEBAKO_SIGNATURE);
    assert!(
        err.message.contains("TEBAKO_REQUIRE_SIGNED=1"),
        "{}",
        err.message
    );
}

#[test]
fn trusted_cache_second_run_hits_marker() {
    let dir = scratch("cache");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", false);
    let press = tebako_signer::press_local_key(&home).unwrap();
    tebako_signer::register_trusted(&home, &press.public_key).unwrap();

    verify_chain_with_home(&pkg, &m, &home).expect("first run");
    verify_chain_with_home(&pkg, &m, &home).expect("second run");
    let j = journal(&home);
    assert!(j.contains("event=v2-slots-verified"), "{j}");
    assert!(j.contains("event=v2-slots-cache-hit"), "{j}");
}
