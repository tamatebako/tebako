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
