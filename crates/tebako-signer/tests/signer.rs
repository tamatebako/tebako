//! tebako-signer integration tests: press-local key generation/caching,
//! sign → verify → tamper matrices, unknown-signer trust error, and the
//! TOFU keyring registration flow.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use tebako_signer::{
    keyid_bytes_from_fingerprint, press_local_key, register_trusted, sign_detached,
    trusted_keyring_bytes, verify_detached, RegisterOutcome, VerifyOutcome,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-signer-test-{}-{name}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    std::fs::remove_dir_all(dir).ok();
}

const MSG: &[u8] = b"the canonical trailer bytes of a tebako package";

#[test]
fn press_key_generates_then_loads_same_key() {
    let home = scratch("press");
    let k1 = press_local_key(&home).expect("first use generates");
    assert!(home.join("keys/press-local.key").exists());
    assert!(home.join("keys/press-local.pub").exists());
    assert_eq!(k1.keyid_hex().len(), 16);

    let k2 = press_local_key(&home).expect("second use loads the cache");
    assert_eq!(k1.fingerprint, k2.fingerprint);
    assert_eq!(k1.keyid, k2.keyid);
    assert_eq!(
        k1.keyid,
        keyid_bytes_from_fingerprint(&k1.fingerprint).unwrap()
    );
    cleanup(&home);
}

#[test]
fn sign_verify_tamper_matrix() {
    let home = scratch("matrix");
    let press = press_local_key(&home).unwrap();
    let sig = sign_detached(MSG, &press.secret_key, &press.fingerprint).expect("sign");
    assert!(!sig.is_empty());

    // unknown signer (empty keyring) -> named Untrusted
    let outcome = verify_detached(&[], MSG, &sig, &press.keyid).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Untrusted(_)));

    // register (TOFU) -> verify Trusted
    let outcome = register_trusted(&home, &press.public_key).expect("register");
    let RegisterOutcome::Added(fp) = outcome else {
        panic!("expected Added, got {outcome:?}");
    };
    assert_eq!(fp, press.fingerprint);

    let keyring = trusted_keyring_bytes(&home).unwrap();
    let outcome = verify_detached(&keyring, MSG, &sig, &press.keyid).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Trusted(_)));

    // duplicate registration -> AlreadyTrusted
    let outcome = register_trusted(&home, &press.public_key).expect("re-register");
    assert_eq!(
        outcome,
        RegisterOutcome::AlreadyTrusted(press.fingerprint.clone())
    );

    // tampered data -> Invalid
    let mut bad = MSG.to_vec();
    bad[0] ^= 0xFF;
    let outcome = verify_detached(&keyring, &bad, &sig, &press.keyid).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Invalid(_)));

    // tampered signature -> Invalid
    let mut bad_sig = sig.clone();
    let n = bad_sig.len();
    bad_sig[n - 1] ^= 0xFF;
    let outcome = verify_detached(&keyring, MSG, &bad_sig, &press.keyid).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Invalid(_)));

    cleanup(&home);
}

#[test]
fn second_key_is_untrusted_until_registered() {
    let home_a = scratch("alice");
    let home_b = scratch("bob");
    let alice = press_local_key(&home_a).unwrap();
    let bob = press_local_key(&home_b).unwrap();
    assert_ne!(alice.fingerprint, bob.fingerprint);

    let sig = sign_detached(MSG, &bob.secret_key, &bob.fingerprint).unwrap();
    // bob's key is not in alice's keyring
    register_trusted(&home_a, &alice.public_key).unwrap();
    let keyring_a = trusted_keyring_bytes(&home_a).unwrap();
    let outcome = verify_detached(&keyring_a, MSG, &sig, &bob.keyid).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Untrusted(_)));

    // TOFU-register bob's key with alice -> Trusted
    register_trusted(&home_a, &bob.public_key).unwrap();
    let keyring_a = trusted_keyring_bytes(&home_a).unwrap();
    let outcome = verify_detached(&keyring_a, MSG, &sig, &bob.keyid).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Trusted(_)));

    cleanup(&home_a);
    cleanup(&home_b);
}

// ---------------------------------------------------------------------
// Subkeyed signers (the release-key shape: [C] primary + [S] subkey)
// ---------------------------------------------------------------------

// The checked-in fixture — see tests/fixtures/README.md for why this is
// not an in-process mint (librnp's keygen leaves an EdDSA primary
// signing-capable regardless of usage flags; gpg's honors them).
const SUBKEYED_SECRET: &str = include_str!("fixtures/subkeyed-release-shape.key.asc");
const SUBKEYED_PUBLIC: &str = include_str!("fixtures/subkeyed-release-shape.pub.asc");
const SUBKEYED_PRIMARY_FP: &str = "58CF65380FB5C5A8FA7239DD50849A8E5658E47A";
const SUBKEYED_PRIMARY_KEYID: &str = "50849a8e5658e47a";
const SUBKEYED_SUBKEY_FP: &str = "2B09411497507C279F85F41E2D6DF607DFC5AAD5";
const SUBKEYED_SUBKEY_KEYID: &str = "2d6df607dfc5aad5";

#[test]
fn subkeyed_signer_issues_from_the_subkey_and_resolves_to_the_primary() {
    // Signing selects the signing subkey (a certify-only primary never
    // signs): the signature's issuer is NOT the primary.
    let sig = sign_detached(MSG, SUBKEYED_SECRET.as_bytes(), SUBKEYED_PRIMARY_FP).expect("sign");
    let issuer_fp = tebako_signer::signature_issuer_fingerprint(&sig).unwrap();
    assert_eq!(issuer_fp, SUBKEYED_SUBKEY_FP, "the issuer is the subkey");
    let primary_keyid = tebako_signer::press_key_from_secret_bytes(SUBKEYED_SECRET.as_bytes())
        .unwrap()
        .keyid;

    // The signature verifies Trusted against the registered public key…
    let home = scratch("subkeyed");
    register_trusted(&home, SUBKEYED_PUBLIC.as_bytes()).unwrap();
    let keyring = trusted_keyring_bytes(&home).unwrap();
    let outcome = verify_detached(&keyring, MSG, &sig, &primary_keyid).expect("verify");
    match outcome {
        VerifyOutcome::Trusted(signer) => assert_eq!(signer, SUBKEYED_SUBKEY_KEYID),
        other => panic!("expected Trusted, got {other:?}"),
    }

    // …and the issuer resolves back to the PRIMARY keyid — the identity a
    // registry signature pin names (spec 09 §9).
    let resolved = tebako_signer::primary_keyid_of(&keyring, SUBKEYED_SUBKEY_KEYID).unwrap();
    assert_eq!(resolved.as_deref(), Some(SUBKEYED_PRIMARY_KEYID));
    // A primary resolves to itself; an unknown keyid resolves to None.
    let self_resolved = tebako_signer::primary_keyid_of(&keyring, SUBKEYED_PRIMARY_KEYID).unwrap();
    assert_eq!(self_resolved.as_deref(), Some(SUBKEYED_PRIMARY_KEYID));
    assert_eq!(
        tebako_signer::primary_keyid_of(&keyring, "0000000000000000").unwrap(),
        None
    );

    cleanup(&home);
}

#[test]
fn verification_keyring_folds_the_embedded_root_and_the_override() {
    let home = scratch("vkeyring");
    // Fresh home: the trusted keyring is empty, yet the verification
    // keyring already trusts the embedded tamatebako root.
    let ring = tebako_signer::verification_keyring(&home).unwrap();
    let ctx = rnp::Context::new().unwrap();
    ctx.load_keys(rnp::KeyringFormat::Gpg, &ring, rnp::LoadSaveFlags::PUBLIC)
        .unwrap();
    let fps: Vec<String> = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .unwrap()
        .collect();
    assert!(
        fps.iter()
            .any(|fp| fp.eq_ignore_ascii_case(tebako_signer::ROOT_FINGERPRINT)),
        "the embedded root must verify without any registration: {fps:?}"
    );

    // The TEBAKO_TRUSTED_ROOT dev override (a path to an armored public
    // key) folds its key in too — the grammar the bootstrap documents.
    let key_path = home.join("override.pub");
    std::fs::write(&key_path, SUBKEYED_PUBLIC).unwrap();
    let folded =
        tebako_signer::trusted_root_override_key(Some(key_path.to_string_lossy().into_owned()));
    assert!(folded.is_some());
    // A bare fingerprint names a keyring-held key — no file, no bytes.
    assert!(
        tebako_signer::trusted_root_override_key(Some(SUBKEYED_PRIMARY_FP.to_string())).is_none()
    );
    assert!(tebako_signer::trusted_root_override_key(None).is_none());
    assert!(tebako_signer::trusted_root_override_key(Some(String::new())).is_none());

    cleanup(&home);
}
