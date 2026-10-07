//! `tebako registry retire` tests (tebako#675): the library surface end
//! to end against temp files — the DEPENDS gate, the dangling-default
//! guard, the last-row rule, and the read-back discipline.

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
