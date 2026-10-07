//! `tebako trust add | list | remove` tests (tebako#541 — the operator
//! CA store): boundary validation, the read-only tmp+rename install, the
//! no-op/replace refusals, the journaled events, and the list report's
//! corruption marker. Temp TEBAKO_HOMEs, fabricated-but-valid PEM armor
//! (rustls-pemfile checks the armor, not the DER).

use std::fs;
use std::path::PathBuf;

fn home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-trust-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    home
}

/// A structurally valid one-block PEM whose base64 body is `body`
/// (4-char groups of the alphabet — armor-valid, DER garbage, which the
/// intake deliberately does not inspect).
fn pem(body: &str) -> String {
    format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n")
}

fn store_file(home: &std::path::Path, name: &str) -> PathBuf {
    tpkg::runtime_store::trust_ca_dir(home).join(format!("{name}.pem"))
}

#[test]
fn trust_add_validates_installs_read_only_and_journals() {
    let home = home("add");
    let src = home.join("corp.pem");
    let bytes = format!("{}{}", pem("AAEB"), pem("AgIE"));
    fs::write(&src, &bytes).unwrap();

    let outcome = tebako_cli::trust::add(&home, &src).unwrap();
    assert_eq!(
        outcome,
        tebako_cli::trust::TrustAddOutcome::Added("corp".to_string())
    );
    let stored = store_file(&home, "corp");
    assert_eq!(fs::read_to_string(&stored).unwrap(), bytes);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            stored.metadata().unwrap().permissions().mode() & 0o777,
            0o444
        );
    }
    let journal = fs::read_to_string(home.join("journal.log")).unwrap();
    assert!(
        journal.contains("event=trust-ca-added name=corp sha256="),
        "{journal}"
    );

    // identical bytes are a no-op, not an error
    let outcome = tebako_cli::trust::add(&home, &src).unwrap();
    assert_eq!(
        outcome,
        tebako_cli::trust::TrustAddOutcome::AlreadyPresent("corp".to_string())
    );

    let _ = fs::remove_dir_all(home.parent().unwrap());
}

#[test]
fn trust_add_refuses_malformed_pems_and_unusable_names() {
    let home = home("addref");

    let junk = home.join("junk.pem");
    fs::write(&junk, b"not a pem\n").unwrap();
    let err = tebako_cli::trust::add(&home, &junk).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("no certificate block"), "{err:?}");
    assert!(!store_file(&home, "junk").exists());

    let half = home.join("half.pem");
    // A broken BLOCK fails (garbage BETWEEN blocks is tolerated by the
    // PEM grammar's block scan — that is not what this asserts).
    fs::write(
        &half,
        format!(
            "{}-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n",
            pem("AAEB")
        ),
    )
    .unwrap();
    let err = tebako_cli::trust::add(&home, &half).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(!store_file(&home, "half").exists());

    let bad = home.join("bad name!.pem");
    fs::write(&bad, pem("AAEB")).unwrap();
    let err = tebako_cli::trust::add(&home, &bad).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("[A-Za-z0-9._-]+"), "{err:?}");

    let _ = fs::remove_dir_all(home.parent().unwrap());
}

#[test]
fn trust_add_refuses_a_name_held_by_different_bytes() {
    let home = home("addcoll");
    let src = home.join("corp.pem");
    fs::write(&src, pem("AAEB")).unwrap();
    tebako_cli::trust::add(&home, &src).unwrap();

    fs::write(&src, pem("AgIE")).unwrap();
    let err = tebako_cli::trust::add(&home, &src).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("different content"), "{err:?}");
    assert!(err.message.contains("tebako trust remove corp"), "{err:?}");
    // the original bytes survive the refused replace
    assert_eq!(
        fs::read_to_string(store_file(&home, "corp")).unwrap(),
        pem("AAEB")
    );

    let _ = fs::remove_dir_all(home.parent().unwrap());
}

#[test]
fn trust_remove_drops_and_journals_and_names_the_unknown() {
    let home = home("remove");
    let src = home.join("corp.pem");
    fs::write(&src, pem("AAEB")).unwrap();
    tebako_cli::trust::add(&home, &src).unwrap();

    let removed = tebako_cli::trust::remove(&home, "corp").unwrap();
    assert_eq!(removed, "corp");
    assert!(!store_file(&home, "corp").exists());
    let journal = fs::read_to_string(home.join("journal.log")).unwrap();
    assert!(
        journal.contains("event=trust-ca-removed name=corp"),
        "{journal}"
    );

    let err = tebako_cli::trust::remove(&home, "corp").unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("nothing to remove"), "{err:?}");

    let err = tebako_cli::trust::remove(&home, "..").unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");

    let _ = fs::remove_dir_all(home.parent().unwrap());
}

#[test]
fn trust_list_renders_rows_sorted_and_marks_corruption() {
    let home = home("list");
    assert!(tebako_cli::trust::list(&home).unwrap().is_empty());

    for (name, body) in [("b", "AgIE"), ("a", "AAEB")] {
        let src = home.join(format!("{name}.pem"));
        fs::write(&src, pem(body)).unwrap();
        tebako_cli::trust::add(&home, &src).unwrap();
    }
    // a hand-edited corrupt entry and a stray non-pem file
    let corrupt = store_file(&home, "z-corrupt");
    fs::write(
        &corrupt,
        b"-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n",
    )
    .unwrap();
    fs::write(
        tpkg::runtime_store::trust_ca_dir(&home).join("notes.txt"),
        b"hi\n",
    )
    .unwrap();

    let rows = tebako_cli::trust::list(&home).unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "z-corrupt"], "{rows:?}");
    assert_eq!(rows[0].certs, Some(1));
    assert_eq!(rows[0].sha256.len(), 64);
    assert_eq!(rows[2].certs, None, "the corrupt entry is marked");

    // no signing keys yet → the pointer stays hidden
    assert!(!tebako_cli::trust::signing_keys_present(&home));
    let donor = home.join("donor");
    fs::create_dir_all(&donor).unwrap();
    let key = tebako_signer::press_local_key(&donor).unwrap();
    tebako_signer::register_trusted(&home, &key.public_key).unwrap();
    assert!(tebako_cli::trust::signing_keys_present(&home));

    let _ = fs::remove_dir_all(home.parent().unwrap());
}
