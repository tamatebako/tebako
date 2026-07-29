//! tebako-bootstrap chain-of-trust tests: sign → verify → tamper matrices
//! (slot sha256 mismatch, unknown signer, invalid signature), the v1
//! legacy acceptance + TEBAECO_REQUIRE_SIGNED hard fail, and the
//! trusted-cache second-run hit.
//!
//! POSIX-only (TODO.v2-1/08): the tests mint keys through rnp, whose
//! 0.1.7 vendored prebuilt has no Windows target — the dev-deps are
//! per-target gated in Cargo.toml, and this whole file compiles out on
//! Windows until the gate lifts.
#![cfg(not(windows))]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "openpgp-verify")]
use tebako_bootstrap::EX_TEBAKO_TRUST;
use tebako_bootstrap::{verify_chain_with_home, EX_TEBAKO_SHA, EX_TEBAKO_SIGNATURE};

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
    let (pkg, m) = make_package_with(dir, image, v1, home, |home| {
        let press = tebako_signer::press_local_key(home).expect("press key");
        (
            press.secret_key.clone(),
            press.fingerprint.clone(),
            press.keyid,
        )
    });
    (pkg, m)
}

/// Assemble a package; for the non-v1 case the signer is resolved up
/// front via `key` (returns secret key bytes, fingerprint, keyid), the
/// keyid is placed into the trailer, and the canonical region is signed
/// with it.
fn make_package_with(
    dir: &Path,
    image: &[u8],
    v1: bool,
    home: &Path,
    key: impl FnOnce(&Path) -> (Vec<u8>, String, [u8; 8]),
) -> (PathBuf, tpkg::Manifest) {
    let bootstrap = b"BOOTSTRAP-BYTES";
    let mut bytes = bootstrap.to_vec();
    bytes.extend_from_slice(image);

    let mut m = tpkg::Manifest {
        version: tpkg::TPKG_VERSION,
        package_flags: if v1 { 0 } else { tpkg::TPKG_FLAG_SIGNED_V2 },
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
        let (secret, fingerprint, keyid) = key(home);
        let mut v2 = tpkg::V2Extension::default();
        v2.slot_digests[0] = sha256(image);
        v2.signer_keyid = keyid; // keyid lands in the canonical region
        v2.signature = vec![0u8];
        m.v2 = Some(v2);

        let trailer = tpkg::encode_trailer(&m, bytes.len() as u64).unwrap();
        let region = tpkg::v2_signed_region(&trailer).unwrap();
        let sig = tebako_signer::sign_detached(&region, &secret, &fingerprint).expect("sign");
        m.v2.as_mut().unwrap().signature = sig;
    }

    bytes.extend_from_slice(&tpkg::encode_trailer(&m, bytes.len() as u64).unwrap());
    let pkg = dir.join("package.bin");
    std::fs::write(&pkg, &bytes).unwrap();

    let mut f = std::fs::File::open(&pkg).unwrap();
    let parsed = tpkg::read_from(&mut f).expect("parse own package");
    (pkg, parsed)
}

/// Generate an OpenPGP signing key directly (for the root/rotation tests).
#[cfg(feature = "openpgp-verify")]
fn make_key(userid: &str) -> (Vec<u8>, Vec<u8>, String, [u8; 8]) {
    let ctx = rnp::Context::new().unwrap();
    let key = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
        .hash(rnp::Hash::Sha256)
        .userid(userid)
        .add_usage(rnp::KeyUsage::Sign)
        .build(&ctx)
        .unwrap();
    let secret = key
        .export(rnp::ExportFlags::ARMORED | rnp::ExportFlags::SECRET | rnp::ExportFlags::SUBKEYS)
        .unwrap();
    let public = key
        .export(rnp::ExportFlags::ARMORED | rnp::ExportFlags::PUBLIC | rnp::ExportFlags::SUBKEYS)
        .unwrap();
    let fp = key.fingerprint().unwrap();
    let keyid = tebako_signer::keyid_bytes_from_fingerprint(&fp).unwrap();
    (secret, public, fp, keyid)
}

fn journal(home: &Path) -> String {
    std::fs::read_to_string(home.join("journal.log")).unwrap_or_default()
}

#[cfg(feature = "openpgp-verify")]
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
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
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

#[cfg(feature = "openpgp-verify")]
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

#[cfg(feature = "openpgp-verify")]
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
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
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
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
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

// ---------------------------------------------------------------------
// embedded-root path + rotation forwarding (item 29 phase 2)
// ---------------------------------------------------------------------

#[cfg(feature = "openpgp-verify")]
#[test]
fn trusted_root_env_verifies_root_signed_package() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch("rootenv");
    let home = dir.join("home");
    let (root_secret, root_public, root_fp, root_keyid) = make_key("root-env");
    // the override carries the root's public key as a FILE (the keyring
    // itself does NOT have it — the root branch must supply the crypto)
    let root_pub_path = dir.join("root.pub");
    std::fs::write(&root_pub_path, &root_public).unwrap();

    let (pkg, m) = make_package_with(&dir, b"the app image payload", false, &home, |_home| {
        (root_secret.clone(), root_fp.clone(), root_keyid)
    });

    // without the override the signer is untrusted (empty keyring)
    std::env::remove_var("TEBAKO_TRUSTED_ROOT");
    let err = verify_chain_with_home(&pkg, &m, &home).unwrap_err();
    assert_eq!(err.code, EX_TEBAKO_TRUST);

    // with TEBAKO_TRUSTED_ROOT=<path to the root's public key> the root's
    // own package verifies
    std::env::set_var("TEBAKO_TRUSTED_ROOT", &root_pub_path);
    verify_chain_with_home(&pkg, &m, &home).expect("root verifies via override");
    std::env::remove_var("TEBAKO_TRUSTED_ROOT");
    assert!(journal(&home).contains("event=v2-trusted-root"));
}

#[cfg(feature = "openpgp-verify")]
#[test]
fn successor_chain_forwards_trust() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch("forward");
    let home = dir.join("home");
    let (r0_secret, r0_public, r0_fp, _r0_keyid) = make_key("root-r0");
    let (r1_secret, r1_public, r1_fp, r1_keyid) = make_key("root-r1");
    let (r2_secret, r2_public, r2_fp, r2_keyid) = make_key("root-r2");

    // the chain verifies from r0's registered public key
    tebako_signer::register_trusted(&home, &r0_public).unwrap();

    // successors dir: R0->R1 statement + R1->R2 statement + both public keys
    let succ = home.join("keyring/successors");
    std::fs::create_dir_all(&succ).unwrap();
    let st01 = tebako_signer::sign_successor_statement(&r0_secret, &r0_fp, &r1_fp).unwrap();
    eprintln!("DEBUG r0_fp={r0_fp} r1_fp={r1_fp}");
    eprintln!("DEBUG st01:\n{}", String::from_utf8_lossy(&st01));
    let st12 = tebako_signer::sign_successor_statement(&r1_secret, &r1_fp, &r2_fp).unwrap();
    std::fs::write(succ.join("01.asc"), &st01).unwrap();
    std::fs::write(succ.join("02.asc"), &st12).unwrap();
    std::fs::write(succ.join(format!("{r1_fp}.pub")), &r1_public).unwrap();
    std::fs::write(succ.join(format!("{r2_fp}.pub")), &r2_public).unwrap();

    std::env::set_var("TEBAKO_TRUSTED_ROOT", &r0_fp);

    // R1-signed package: trust forwards through one rotation
    let (pkg1, m1) = make_package_with(&dir, b"the app image payload", false, &home, |_home| {
        (r1_secret.clone(), r1_fp.clone(), r1_keyid)
    });
    verify_chain_with_home(&pkg1, &m1, &home).expect("forward to r1");
    assert!(journal(&home).contains("event=v2-trusted-forwarded"));

    // R2-signed package: trust forwards through two rotations
    let (pkg2, m2) = make_package_with(&dir, b"the app image payload", false, &home, |_home| {
        (r2_secret.clone(), r2_fp.clone(), r2_keyid)
    });
    verify_chain_with_home(&pkg2, &m2, &home).expect("forward to r2");

    std::env::remove_var("TEBAKO_TRUSTED_ROOT");
}

#[cfg(feature = "openpgp-verify")]
#[test]
fn broken_successor_chain_keeps_trust_error() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch("brokenfwd");
    let home = dir.join("home");
    let (_r0_secret, _r0_public, r0_fp, _r0_keyid) = make_key("root-b0");
    let (r1_secret, _r1_public, r1_fp, r1_keyid) = make_key("root-b1");
    let (stranger_secret, _sp, _sfp, _sk) = make_key("stranger-b");

    // the statement claims R0->R1 but is signed by a stranger
    let st = tebako_signer::sign_detached(
        format!("format: TEBAKO-ROOT-SUCCESSOR-V1\npredecessor: {r0_fp}\nsuccessor: {r1_fp}\ncreated: 1\n").as_bytes(),
        &stranger_secret,
        &_sfp,
    )
    .unwrap();
    let st = rnp::armor_bytes(&st, rnp::ops::ArmorType::Signature).unwrap();
    let succ = home.join("keyring/successors");
    std::fs::create_dir_all(&succ).unwrap();
    std::fs::write(succ.join("01.asc"), {
        let mut v = b"-----BEGIN TEBAKO SUCCESSOR STATEMENT-----\n".to_vec();
        v.extend_from_slice(
            format!("format: TEBAKO-ROOT-SUCCESSOR-V1\npredecessor: {r0_fp}\nsuccessor: {r1_fp}\ncreated: 1\n").as_bytes(),
        );
        v.extend_from_slice(b"-----BEGIN PGP SIGNATURE-----\n");
        v.extend_from_slice(&st);
        v
    })
    .unwrap();
    std::fs::write(succ.join(format!("{r1_fp}.pub")), &_r1_public).unwrap();

    let (pkg, m) = make_package_with(&dir, b"the app image payload", false, &home, |_home| {
        (r1_secret.clone(), r1_fp.clone(), r1_keyid)
    });

    std::env::set_var("TEBAKO_TRUSTED_ROOT", &r0_fp);
    let err = verify_chain_with_home(&pkg, &m, &home).unwrap_err();
    std::env::remove_var("TEBAKO_TRUSTED_ROOT");
    assert_eq!(err.code, EX_TEBAKO_TRUST);
}

#[cfg(not(feature = "openpgp-verify"))]
#[test]
fn signed_package_accepted_unverified_with_warning() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch("unverified");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", false);

    verify_chain_with_home(&pkg, &m, &home).expect("unverified accepted");
    assert!(journal(&home).contains("event=v2-unverified-accepted"));
}

#[cfg(not(feature = "openpgp-verify"))]
#[test]
fn signed_require_signed_fails_closed_when_verification_disabled() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch("require-unverified");
    let home = dir.join("home");
    let (pkg, m) = make_package(&dir, &home, b"the app image payload", false);

    std::env::set_var("TEBAKO_REQUIRE_SIGNED", "1");
    let err = verify_chain_with_home(&pkg, &m, &home).unwrap_err();
    std::env::remove_var("TEBAKO_REQUIRE_SIGNED");

    assert_eq!(err.code, EX_TEBAKO_SIGNATURE);
    assert!(
        err.message.contains("WITHOUT OpenPGP verification"),
        "{}",
        err.message
    );
}
