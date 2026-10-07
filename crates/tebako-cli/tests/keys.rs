//! `tebako keys list | remove` tests (spec 09 §9, tebako#617): the
//! trust inputs in one report (embedded root, keyring, pins) with the
//! registry bindings, and the fail-closed removal across both operator
//! stores. Temp TEBAKO_HOMEs, no network.

use std::fs;
use std::path::PathBuf;

fn home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-keys-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    home
}

/// press_local_key caches per home; distinct keys need distinct donor
/// homes (the donor's keys/ dir is the press side, never a trust input).
fn fresh_key(tag: &str) -> tebako_signer::PressKey {
    let donor = std::env::temp_dir().join(format!(
        "tebako-cli-keys-donor-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&donor);
    fs::create_dir_all(&donor).unwrap();
    tebako_signer::press_local_key(&donor).unwrap()
}

#[test]
fn keys_list_renders_every_trust_input_with_registry_bindings() {
    let home = home("list");

    // an untouched store lists exactly the embedded root
    let rows = tebako_cli::keys::list_keys(&home).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fingerprint, tebako_signer::ROOT_FINGERPRINT);
    assert_eq!(rows[0].sources, vec!["embedded root"]);
    assert!(rows[0].registries.is_empty());

    // one key in the keyring, one pinned, one in BOTH stores
    let keyring_key = fresh_key("list-a");
    let pinned_key = fresh_key("list-b");
    let both_key = fresh_key("list-c");
    tebako_signer::register_trusted(&home, &keyring_key.public_key).unwrap();
    tebako_signer::pin_trusted(&home, pinned_key.public_key.as_slice()).unwrap();
    tebako_signer::register_trusted(&home, &both_key.public_key).unwrap();
    tebako_signer::pin_trusted(&home, both_key.public_key.as_slice()).unwrap();

    // a registry binding recorded at pin time
    let pinned_fp = pinned_key.fingerprint.to_uppercase();
    fs::write(
        home.join("config.yaml"),
        format!(
            "registries:\n  - ref: tfs:github:acme/tools\n    signing_fingerprint: {pinned_fp}\n"
        ),
    )
    .unwrap();

    let rows = tebako_cli::keys::list_keys(&home).unwrap();
    assert_eq!(rows.len(), 4, "{rows:?}");
    let by_fp = |fp: &str| {
        rows.iter()
            .find(|r| r.fingerprint == fp.to_uppercase())
            .unwrap_or_else(|| panic!("no row for {fp}: {rows:?}"))
    };
    assert_eq!(by_fp(&keyring_key.fingerprint).sources, vec!["keyring"]);
    let pinned_row = by_fp(&pinned_key.fingerprint);
    assert_eq!(pinned_row.sources, vec!["pin"]);
    assert_eq!(pinned_row.registries, vec!["tfs:github:acme/tools"]);
    assert_eq!(by_fp(&both_key.fingerprint).sources, vec!["keyring", "pin"]);
}

#[test]
fn keys_remove_drops_both_stores_and_journals() {
    let home = home("remove");
    let key = fresh_key("remove-a");
    let survivor = fresh_key("remove-b");
    tebako_signer::register_trusted(&home, &key.public_key).unwrap();
    tebako_signer::pin_trusted(&home, key.public_key.as_slice()).unwrap();
    tebako_signer::register_trusted(&home, &survivor.public_key).unwrap();
    let fp = key.fingerprint.to_uppercase();
    assert!(tebako_signer::is_trusted(&home, &fp).unwrap());

    let removed = tebako_cli::keys::remove_key(&home, &fp.to_lowercase()).unwrap();
    assert_eq!(removed, fp);
    assert!(!tebako_signer::is_trusted(&home, &fp).unwrap());
    assert!(!tebako_signer::pin_path(&home, &fp).exists());
    // the keyring rebuild kept the other key
    assert!(tebako_signer::is_trusted(&home, &survivor.fingerprint).unwrap());

    let journal = fs::read_to_string(home.join("journal.log")).unwrap();
    assert!(
        journal.contains(&format!("event=trusted-key-removed fingerprint={fp}")),
        "{journal}"
    );
}

#[test]
fn keys_remove_refusals_are_named() {
    let home = home("remove-ref");

    // the embedded root is not removable
    let err = tebako_cli::keys::remove_key(&home, tebako_signer::ROOT_FINGERPRINT).unwrap_err();
    assert_eq!(err.code, 72, "{err:?}");
    assert!(err.message.contains("not removable"), "{err:?}");

    // an unknown fingerprint names the nothing-to-remove class
    let err = tebako_cli::keys::remove_key(&home, &"A".repeat(40)).unwrap_err();
    assert_eq!(err.code, 72, "{err:?}");
    assert!(err.message.contains("nothing to remove"), "{err:?}");

    // a malformed fingerprint is the usage class
    let err = tebako_cli::keys::remove_key(&home, "xyz").unwrap_err();
    assert_eq!(err.code, 64, "{err:?}");
}
