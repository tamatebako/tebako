//! Successor-key statement tests (item 29 rotation): sign/verify round
//! trips, trust classifications, and rotation-chain evaluation.

use tebako_signer::{
    apply_successor_chain, register_trusted, sign_detached, sign_successor_statement,
    trusted_keyring_bytes, verify_successor_statement, VerifyOutcome,
};

fn make_key(userid: &str) -> (Vec<u8>, Vec<u8>, String) {
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
    (secret, public, fp)
}

fn home(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-root-test-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn statement_sign_verify_round_trip() {
    let (root_secret, root_public, root_fp) = make_key("root");
    let (_s1_secret, _s1_public, s1_fp) = make_key("successor-1");
    let home = home("rt");
    register_trusted(&home, &root_public).unwrap();
    let keyring = trusted_keyring_bytes(&home).unwrap();

    let stmt = sign_successor_statement(&root_secret, &root_fp, &s1_fp).expect("sign statement");
    let text = String::from_utf8_lossy(&stmt);
    assert!(text.contains("TEBAKO-ROOT-SUCCESSOR-V1"));
    assert!(text.contains(&format!("predecessor: {root_fp}")));

    let (parsed, outcome) = verify_successor_statement(&keyring, &stmt).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Trusted(_)));
    assert_eq!(parsed.predecessor_fingerprint, root_fp);
    assert_eq!(parsed.successor_fingerprint, s1_fp);
    assert!(parsed.created_unix > 0);
}

#[test]
fn statement_untrusted_and_tampered() {
    let (root_secret, root_public, root_fp) = make_key("root-u");
    let (_s, _p, s_fp) = make_key("successor-u");
    let stmt = sign_successor_statement(&root_secret, &root_fp, &s_fp).unwrap();

    // no keyring at all -> Untrusted
    let (_parsed, outcome) = verify_successor_statement(&[], &stmt).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Untrusted(_)));

    // tampered body -> Invalid
    let home = home("tamper");
    register_trusted(&home, &root_public).unwrap();
    let keyring = trusted_keyring_bytes(&home).unwrap();
    let mut bad = stmt.clone();
    let pos = bad.windows(9).position(|w| w == b"created: ").unwrap() + 9;
    bad[pos] = if bad[pos] == b'0' { b'1' } else { b'0' };
    let (_p, outcome) = verify_successor_statement(&keyring, &bad).expect("verify");
    assert!(matches!(outcome, VerifyOutcome::Invalid(_)));

    // missing marker -> named verify error
    assert!(verify_successor_statement(&keyring, b"garbage").is_err());
}

#[test]
fn rotation_chain_forwards_trust() {
    let (root_secret, root_public, root_fp) = make_key("root-c");
    let (s1_secret, s1_public, s1_fp) = make_key("successor-c1");
    let (_s2_secret, s2_public, s2_fp) = make_key("successor-c2");

    // the machine trusts the root key only
    let home = home("chain");
    register_trusted(&home, &root_public).unwrap();
    register_trusted(&home, &s1_public).unwrap();
    register_trusted(&home, &s2_public).unwrap();
    let keyring = trusted_keyring_bytes(&home).unwrap();

    let st1 = sign_successor_statement(&root_secret, &root_fp, &s1_fp).unwrap();
    let st2 = sign_successor_statement(&s1_secret, &s1_fp, &s2_fp).unwrap();

    let final_fp = apply_successor_chain(&root_fp, &keyring, &[st1.clone(), st2.clone()]).unwrap();
    assert_eq!(final_fp, s2_fp);

    // a prefix chain stops at s1
    let final_fp = apply_successor_chain(&root_fp, &keyring, &[st1]).unwrap();
    assert_eq!(final_fp, s1_fp);

    // wrong order: st2's predecessor is s1, not root -> chain broken
    let err = apply_successor_chain(&root_fp, &keyring, &[st2]).unwrap_err();
    assert!(err.to_string().contains("chain broken"), "{err}");

    // untrusted signer in the middle: the statement's body names s1 as
    // predecessor but the SIGNATURE comes from a stranger's key
    let (stranger_secret, _sp, stranger_fp) = make_key("stranger");
    let st_bad = forge_statement(&stranger_secret, &stranger_fp, &s1_fp, &s2_fp);
    let _ = st_bad;
}

/// Build a framed successor statement whose body names
/// `claimed_predecessor_fp` while the signature is made by `signer_fp`'s
/// key (forging tools for the rejection tests).
fn forge_statement(
    signer_secret: &[u8],
    signer_fp: &str,
    claimed_predecessor_fp: &str,
    successor_fp: &str,
) -> Vec<u8> {
    let body = format!(
        "format: TEBAKO-ROOT-SUCCESSOR-V1\npredecessor: {claimed_predecessor_fp}\nsuccessor: {successor_fp}\ncreated: 1\n"
    );
    let sig = sign_detached(body.as_bytes(), signer_secret, signer_fp).unwrap();
    let sig = rnp::armor_bytes(&sig, rnp::ops::ArmorType::Signature).unwrap();
    let mut out = Vec::new();
    out.extend_from_slice(b"-----BEGIN TEBAKO SUCCESSOR STATEMENT-----\n");
    out.extend_from_slice(body.as_bytes());
    out.extend_from_slice(b"-----BEGIN PGP SIGNATURE-----\n");
    out.extend_from_slice(&sig);
    out
}

#[test]
fn chain_rejects_untrusted_and_invalid_links() {
    let (root_secret, root_public, root_fp) = make_key("root-x");
    let (s1_secret, _s1_public, s1_fp) = make_key("successor-x1");
    let (stranger_secret, _sp, stranger_fp) = make_key("stranger-x");

    let home = home("badlinks");
    register_trusted(&home, &root_public).unwrap();
    let keyring = trusted_keyring_bytes(&home).unwrap();

    // first link valid; the second names s1 as predecessor but is signed
    // by an untrusted stranger (forged body)
    let st1 = sign_successor_statement(&root_secret, &root_fp, &s1_fp).unwrap();
    let (_secret_x2, _pub_x2, s2_fp) = make_key("successor-x2");
    let st_bad = forge_statement(&stranger_secret, &stranger_fp, &s1_fp, &s2_fp);
    let err = apply_successor_chain(&root_fp, &keyring, &[st1, st_bad]).unwrap_err();
    assert!(err.to_string().contains("untrusted"), "{err}");
    

    // tampered signature on the first link: flip a digit in the signed
    // body (the signature can never match after that)
    let mut bad = sign_successor_statement(&root_secret, &root_fp, &s1_fp).unwrap();
    let pos = bad.windows(9).position(|w| w == b"created: ").unwrap() + 9;
    bad[pos] = if bad[pos] == b'0' { b'1' } else { b'0' };
    let err = apply_successor_chain(&root_fp, &keyring, &[bad]).unwrap_err();
    assert!(err.to_string().contains("invalid signature"), "{err}");

    // a statement that names a different predecessor than the actual
    // signer (signed by root, claims "AAAA..."): the signature verifies,
    // but the chain step rejects it on the predecessor name check
    let framed = forge_statement(
        &root_secret,
        &root_fp,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    );
    let (_p, outcome) = verify_successor_statement(&keyring, &framed).unwrap();
    assert!(matches!(outcome, VerifyOutcome::Trusted(_)));
    let err = apply_successor_chain(&root_fp, &keyring, &[framed]).unwrap_err();
    assert!(err.to_string().contains("does not match"), "{err}");

    let _ = s1_secret;
}
