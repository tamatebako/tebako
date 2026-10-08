//! `tebako registry <verb>` tests (tebako#675 retire, tebako#680
//! validate): the library surface end to end against temp files — the
//! DEPENDS gate, the dangling-default guard, the last-row rule, the
//! read-back discipline, and the validate verdicts (human + --json).

use std::fs;
use std::path::PathBuf;

fn scratch(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tebako-cli-regverbs-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A two-row runtime registry with one depending app; `constraint`
/// steers the DEPENDS gate's strand/no-strand legs.
fn runtime_registry(constraint: &str) -> String {
    format!(
        "schema_version: 1\npayloads:\n  - name: ruby\n    kind: runtime\n    engine: ruby\n    versions:\n      - version: 3.3.12-0.16.29\n        platforms: universal\n        release: {{ref: tfs:github:o/ruby:3.3.12-0.16.29}}\n      - version: 4.0.7-0.17.1\n        platforms: universal\n        release: {{ref: tfs:github:o/ruby:4.0.7-0.17.1}}\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: tfs:github:o/app:1.0}}\n        runtime_requirement: {{engine: ruby, constraint: \"{constraint}\"}}\n        entrypoints: [app]\n    default: 1.0\n"
    )
}

// ---------------------------------------------------------------------
// retire
// ---------------------------------------------------------------------

#[test]
fn retire_removes_a_plain_row_and_journals() {
    let dir = scratch("plain");
    let home = dir.join("home");
    let registry = dir.join("tpkg-registry.yaml");
    fs::write(&registry, runtime_registry(">= 3.3")).unwrap();

    let report =
        tebako_cli::registry::retire(&home, &registry, "ruby", "3.3.12-0.16.29", false).unwrap();
    assert!(report.contains("retired ruby@3.3.12-0.16.29"), "{report}");

    let after = fs::read_to_string(&registry).unwrap();
    assert!(!after.contains("3.3.12-0.16.29"), "{after}");
    assert!(after.contains("4.0.7-0.17.1"), "{after}");
    // the written document reads back through the client parser
    tebako_resolve::Registry::from_yaml(&after).unwrap();

    let journal = fs::read_to_string(home.join("journal.log")).unwrap();
    assert!(
        journal.contains("event=registry-row-retired") && journal.contains("payload=ruby"),
        "{journal}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn retire_refuses_when_an_in_registry_edge_strands() {
    let dir = scratch("stranded");
    let home = dir.join("home");
    let registry = dir.join("tpkg-registry.yaml");
    let before = runtime_registry("~> 3.3.0");
    fs::write(&registry, &before).unwrap();

    // `~> 3.3.0` matches 3.3.12-0.16.29 but not the surviving
    // 4.0.7-0.17.1 — retiring the old line strands the app.
    let err = tebako_cli::registry::retire(&home, &registry, "ruby", "3.3.12-0.16.29", false)
        .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("RegistryRowStillRequired"), "{err:?}");
    assert!(err.message.contains("app 1.0"), "{err:?}");
    assert!(err.message.contains("--force"), "{err:?}");
    // the refusal never touched the file
    assert_eq!(fs::read_to_string(&registry).unwrap(), before);

    // --force retires loudly: the stranded edge is named in the report.
    let report =
        tebako_cli::registry::retire(&home, &registry, "ruby", "3.3.12-0.16.29", true).unwrap();
    assert!(report.contains("retired ruby@3.3.12-0.16.29"), "{report}");
    assert!(report.contains("forced past"), "{report}");
    assert!(report.contains("RegistryRowStillRequired"), "{report}");
    let after = fs::read_to_string(&registry).unwrap();
    assert!(!after.contains("3.3.12-0.16.29"), "{after}");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn retire_repoints_a_dangling_default_only_under_force() {
    let dir = scratch("default");
    let home = dir.join("home");
    let registry = dir.join("tpkg-registry.yaml");
    let before = runtime_registry(">= 3.3").replace(
        "  - name: ruby\n    kind: runtime\n    engine: ruby\n",
        "  - name: ruby\n    kind: runtime\n    engine: ruby\n    default: 3.3.12-0.16.29\n",
    );
    assert!(before.contains("default: 3.3.12-0.16.29"));
    fs::write(&registry, &before).unwrap();

    let err = tebako_cli::registry::retire(&home, &registry, "ruby", "3.3.12-0.16.29", false)
        .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message.contains("RegistryDefaultWouldDangle"),
        "{err:?}"
    );
    assert_eq!(fs::read_to_string(&registry).unwrap(), before);

    let report =
        tebako_cli::registry::retire(&home, &registry, "ruby", "3.3.12-0.16.29", true).unwrap();
    assert!(
        report.contains("default now points at 4.0.7-0.17.1"),
        "{report}"
    );
    let after = fs::read_to_string(&registry).unwrap();
    let reparsed = tebako_resolve::Registry::from_yaml(&after).unwrap();
    assert_eq!(
        reparsed.payload("ruby").unwrap().default.as_deref(),
        Some("4.0.7-0.17.1"),
        "{after}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn retire_removes_the_payload_entry_with_its_last_row_only_under_force() {
    let dir = scratch("lastrow");
    let home = dir.join("home");
    let registry = dir.join("tpkg-registry.yaml");
    fs::write(
        &registry,
        "schema_version: 1\npayloads:\n  - name: fonts\n    kind: data\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {ref: tfs:github:o/fonts:1.0}\n    default: 1.0\n",
    )
    .unwrap();

    let err = tebako_cli::registry::retire(&home, &registry, "fonts", "1.0", false).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("RegistryRowIsLast"), "{err:?}");

    let report = tebako_cli::registry::retire(&home, &registry, "fonts", "1.0", true).unwrap();
    assert!(
        report.contains("payload entry 'fonts' was removed"),
        "{report}"
    );
    let after =
        tebako_resolve::Registry::from_yaml(&fs::read_to_string(&registry).unwrap()).unwrap();
    assert!(after.payload("fonts").is_none());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn retire_names_the_unknown_payload_and_version() {
    let dir = scratch("unknown");
    let home = dir.join("home");
    let registry = dir.join("tpkg-registry.yaml");
    fs::write(&registry, runtime_registry(">= 3.3")).unwrap();

    let err = tebako_cli::registry::retire(&home, &registry, "bogus", "1.0", false).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("no payload named 'bogus'"), "{err:?}");
    assert!(err.message.contains("ruby"), "{err:?}");

    let err = tebako_cli::registry::retire(&home, &registry, "ruby", "9.9", false).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("has no version row '9.9'"), "{err:?}");
    assert!(err.message.contains("3.3.12-0.16.29"), "{err:?}");

    // a registry that does not parse surfaces the client error verbatim
    let broken = dir.join("broken.yaml");
    fs::write(&broken, "payloads: not-a-list\n").unwrap();
    let err = tebako_cli::registry::retire(&home, &broken, "ruby", "1.0", false).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message.contains("cannot parse the registry yaml"),
        "{err:?}"
    );

    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------

#[test]
fn validate_accepts_a_sound_registry_and_counts_it() {
    let dir = scratch("vok");
    let registry = dir.join("tpkg-registry.yaml");
    fs::write(&registry, runtime_registry(">= 3.3")).unwrap();

    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("OK (2 payload(s), 3 version row(s))"),
        "{text}"
    );

    // the file:// spelling of the same document validates identically
    let url = tebako_http::file_url(&registry);
    let (text, code) = tebako_cli::registry::validate(&url, false).unwrap();
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("OK (2 payload(s), 3 version row(s))"),
        "{text}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_catches_the_dangling_default_class_verbatim() {
    // tebako#680's motivating break: a row pruned by hand leaving
    // `default:` dangling — the client parse rejects the whole registry,
    // and the gate reports THAT message.
    let dir = scratch("vdangle");
    let registry = dir.join("tpkg-registry.yaml");
    fs::write(
        &registry,
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {ref: tfs:github:o/app:1.0}\n        entrypoints: [app]\n    default: 9.9\n",
    )
    .unwrap();

    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 65, "{text}");
    assert!(
        text.contains("default '9.9' names no listed version"),
        "{text}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_collects_the_strict_extras_per_row() {
    let dir = scratch("vextras");
    let registry = dir.join("tpkg-registry.yaml");
    // Both extras ride rows the lenient reader accepts: a constraint the
    // version grammar rejects (a trailing comma's empty clause) and an
    // abi spelled with no implementation axis anywhere.
    fs::write(
        &registry,
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {ref: tfs:github:o/app:1.0}\n        runtime_requirement: {engine: ruby, constraint: \">= 3.3,\"}\n        entrypoints: [app]\n      - version: 2.0\n        platforms: universal\n        release: {ref: tfs:github:o/app:2.0}\n        runtime_requirement: {engine: ruby, constraint: \">= 3.3\", abi: aarch64-macos}\n        entrypoints: [app]\n    default: 2.0\n",
    )
    .unwrap();

    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 65, "{text}");
    assert!(
        text.contains("app 1.0: runtime_requirement.constraint does not parse"),
        "{text}"
    );
    assert!(
        text.contains("app 2.0: runtime_requirement.abi is spelled with no implementation axis"),
        "{text}"
    );

    // an implementation axis ANYWHERE (here: the payload-level key)
    // clears the abi row
    let with_impl = fs::read_to_string(&registry).unwrap().replace(
        "  - name: app\n    kind: app\n",
        "  - name: app\n    kind: app\n    implementation: mri\n",
    );
    fs::write(&registry, with_impl).unwrap();
    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 65, "{text}");
    assert!(!text.contains("abi"), "{text}");
    assert!(text.contains("constraint does not parse"), "{text}");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_names_an_abi_mirrored_on_a_multi_platform_row() {
    // tebako#440's mirror rule, producer-enforced: the abi is per-triplet
    // by construction, so the mirror carries it only when one value holds
    // for the WHOLE entry (a universal row, or a single-platform
    // per-triplet row). A multi-platform row's abi is the published-lie
    // class (one platform's value served to every triplet).
    let dir = scratch("vabi");
    let registry = dir.join("tpkg-registry.yaml");
    let sha = "a".repeat(64);
    let doc = |abi: &str| {
        format!(
            "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    implementation: mri\n    versions:\n      - version: 1.0\n        platforms:\n          aarch64-macos: {{artifact: app-1.0-macos-arm64.tfs, sha256: \"{sha}\"}}\n          x86_64-linux-gnu: {{artifact: app-1.0-linux-gnu-x86_64.tfs, sha256: \"{sha}\"}}\n        release: {{ref: tfs:github:o/app:1.0}}\n        runtime_requirement: {{engine: ruby, constraint: \"~> 3.3.0\", implementation: mri{abi}}}\n        entrypoints: [app]\n"
        )
    };

    // the multi-platform row carrying one platform's abi: the named lie
    fs::write(&registry, doc(", abi: arm64-darwin-23")).unwrap();
    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 65, "{text}");
    assert!(
        text.contains("app 1.0: runtime_requirement.abi on a 2-platform per-triplet row"),
        "{text}"
    );

    // the honest mirror of the same row: abi omitted
    fs::write(&registry, doc("")).unwrap();
    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 0, "{text}");

    // one value holding for the whole entry keeps the abi: the
    // single-platform per-triplet row …
    let single = format!(
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    implementation: mri\n    versions:\n      - version: 1.0\n        platforms:\n          aarch64-macos: {{artifact: app-1.0-macos-arm64.tfs, sha256: \"{sha}\"}}\n        release: {{ref: tfs:github:o/app:1.0}}\n        runtime_requirement: {{engine: ruby, constraint: \"~> 3.3.0\", implementation: mri, abi: arm64-darwin-23}}\n        entrypoints: [app]\n"
    );
    fs::write(&registry, &single).unwrap();
    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 0, "{text}");

    // … and the universal row
    let universal =
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    implementation: mri\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {ref: tfs:github:o/app:1.0}\n        runtime_requirement: {engine: ruby, constraint: \"~> 3.3.0\", implementation: mri, abi: arm64-darwin-23}\n        entrypoints: [app]\n";
    fs::write(&registry, universal).unwrap();
    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 0, "{text}");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_catches_a_signing_block_whose_key_is_not_the_declared_one() {
    let dir = scratch("vsigning");
    let registry = dir.join("tpkg-registry.yaml");
    let donor = dir.join("donor");
    fs::create_dir_all(&donor).unwrap();
    let key = tebako_signer::press_local_key(&donor).unwrap();
    let armored = String::from_utf8(key.public_key.clone()).unwrap();
    let indented = armored
        .lines()
        .map(|l| format!("    {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    let wrong = "0".repeat(40);
    let doc = |fp: &str| {
        format!(
            "schema_version: 1\nsigning:\n  key: |\n{indented}\n  fingerprint: {fp}\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: tfs:github:o/app:1.0}}\n        entrypoints: [app]\n    default: 1.0\n"
        )
    };
    fs::write(&registry, doc(&wrong)).unwrap();

    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 65, "{text}");
    assert!(text.contains("signing.fingerprint"), "{text}");
    // the report names the key's OWN fingerprint
    assert!(
        text.contains(&tebako_signer::public_key_fingerprint(&key.public_key).unwrap()),
        "{text}"
    );

    // declaring the key's own fingerprint clears the row
    let actual = tebako_signer::public_key_fingerprint(&key.public_key).unwrap();
    fs::write(&registry, doc(&actual)).unwrap();
    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), false).unwrap();
    assert_eq!(code, 0, "{text}");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_json_is_a_machine_document() {
    let dir = scratch("vjson");
    let registry = dir.join("tpkg-registry.yaml");
    fs::write(
        &registry,
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {ref: tfs:github:o/app:1.0}\n        entrypoints: [app]\n    default: 9.9\n",
    )
    .unwrap();

    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), true).unwrap();
    assert_eq!(code, 65, "{text}");
    let doc = tebako_pkg::json_parse(&text).unwrap();
    assert_eq!(
        doc.find("registry_validate_schema")
            .and_then(|v| v.as_u64()),
        Some(1)
    );
    assert!(matches!(
        doc.find("ok"),
        Some(tebako_pkg::JsonValue::Bool(false))
    ));
    let errors = match doc.find("errors") {
        Some(tebako_pkg::JsonValue::Array(items)) => items,
        other => panic!("errors is not an array: {other:?}"),
    };
    assert_eq!(errors.len(), 1, "{text}");
    assert!(
        errors[0]
            .find("message")
            .and_then(|v| v.as_string())
            .unwrap()
            .contains("default '9.9'"),
        "{text}"
    );

    // the sound document's json verdict
    fs::write(&registry, runtime_registry(">= 3.3")).unwrap();
    let (text, code) = tebako_cli::registry::validate(registry.to_str().unwrap(), true).unwrap();
    assert_eq!(code, 0, "{text}");
    let doc = tebako_pkg::json_parse(&text).unwrap();
    assert!(matches!(
        doc.find("ok"),
        Some(tebako_pkg::JsonValue::Bool(true))
    ));
    assert_eq!(doc.find("payloads").and_then(|v| v.as_u64()), Some(2));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_names_an_input_that_is_neither_file_nor_reference() {
    let dir = scratch("vnoinput");
    let missing = dir.join("nope.yaml");
    let err = tebako_cli::registry::validate(missing.to_str().unwrap(), false).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message
            .contains("is not an existing file and does not parse as a registry reference"),
        "{err:?}"
    );

    let _ = fs::remove_dir_all(&dir);
}
