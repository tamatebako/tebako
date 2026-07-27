//! The dispatch-time registry layer (roadmap 33, spec 07 §2.1's last
//! chain link + spec 04 §3): the registry-default link resolves every
//! registry form through tebako-resolve behind the
//! `~/.tebako/registries/<sha>.yaml` cache — TTL, offline
//! cache-or-named-error, `update-registries`, doctor freshness. Remote
//! resolution is exercised through `file://` mirrors and pre-seeded
//! caches — no live network in tests.

mod common;

use common::*;
use tebako_shim::resolve::{self, VersionSource};
use tebako_shim::Action;

fn seed_metanorma(home: &std::path::Path) {
    for v in ["1.0.0", "1.2.2", "1.2.3"] {
        write_payload(
            home,
            "metanorma",
            v,
            &app_manifest("metanorma", v, &entrypoint_yaml(RUBY_ENTRY, "metanorma")),
        );
    }
}

/// A spec 04 §2 registry pinning `default` for metanorma (the full
/// validated model — tebako-resolve parses it at the cache boundary).
fn registry_yaml(default: &str) -> String {
    format!(
        "schema_version: 1\npayloads:\n  - name: metanorma\n    kind: app\n    default: {default}\n    versions:\n      - version: {default}\n        platforms: universal\n        release: {{ref: file:///mirror/metanorma-{default}.tfs}}\n        entrypoints: [metanorma]\n"
    )
}

fn file_registry(root: &std::path::Path, default: &str) -> std::path::PathBuf {
    let reg = root.join("tpkg-registry.yaml");
    std::fs::write(&reg, registry_yaml(default)).expect("registry");
    reg
}

fn write_config_with_registry(home: &std::path::Path, reg: &std::path::Path) {
    write_config(
        home,
        &format!("registries:\n  - file://{}\n", reg.display()),
    );
}

/// The single `.meta` sidecar under `registries/` (one registered
/// registry per test).
fn only_meta(home: &std::path::Path) -> std::path::PathBuf {
    let dir = home.join("registries");
    let metas: Vec<_> = std::fs::read_dir(&dir)
        .expect("registries dir")
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".meta"))
        .collect();
    assert_eq!(metas.len(), 1, "expected exactly one cached registry");
    metas[0].path()
}

fn backdate_meta(home: &std::path::Path, age_secs: u64) {
    let meta = only_meta(home);
    let content = std::fs::read_to_string(&meta).unwrap();
    let ref_line = content.lines().find(|l| l.starts_with("ref: ")).unwrap();
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - age_secs;
    std::fs::write(meta, format!("fetched-at: {epoch}\n{ref_line}\n")).unwrap();
}

/// Seed the cache entry a REMOTE ref would have (the offline tests never
/// touch the network).
fn seed_remote_cache(home: &std::path::Path, canonical: &str, default: &str) {
    let key = tebako_resolve::sha256_hex(canonical.as_bytes());
    let dir = home.join("registries");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{key}.yaml")), registry_yaml(default)).unwrap();
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(
        dir.join(format!("{key}.meta")),
        format!("fetched-at: {epoch}\nref: {canonical}\n"),
    )
    .unwrap();
}

fn printed(action: Action) -> (String, u8) {
    match action {
        Action::Print { text, code } => (text, code),
        Action::Exec(_) => panic!("expected Print, got Exec"),
    }
}

#[test]
fn registry_default_resolves_through_the_cache_after_the_source_is_gone() {
    let tmp = TempDir::new("reg-cache-hit");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let reg = file_registry(tmp.path(), "1.0.0");
    write_config_with_registry(&home, &reg);
    let ctx = ctx(&home, tmp.path());

    let res = resolve::resolve("metanorma", &ctx).unwrap();
    assert_eq!(res.version, "1.0.0");
    assert!(matches!(res.source, VersionSource::RegistryDefault(_)));

    // the mirror file is gone — dispatch survives on the cached registry
    std::fs::remove_file(&reg).unwrap();
    let res = resolve::resolve("metanorma", &ctx).unwrap();
    assert_eq!(res.version, "1.0.0");
}

#[test]
fn fresh_cache_wins_until_the_ttl_expires() {
    let tmp = TempDir::new("reg-ttl");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let reg = file_registry(tmp.path(), "1.0.0");
    write_config_with_registry(&home, &reg);
    let ctx = ctx(&home, tmp.path());

    assert_eq!(
        resolve::resolve("metanorma", &ctx).unwrap().version,
        "1.0.0"
    );

    // the source moved on, the fresh cache has not
    std::fs::write(&reg, registry_yaml("1.2.2")).unwrap();
    assert_eq!(
        resolve::resolve("metanorma", &ctx).unwrap().version,
        "1.0.0"
    );

    // past the 24 h TTL the source is re-read
    backdate_meta(&home, 25 * 3600);
    assert_eq!(
        resolve::resolve("metanorma", &ctx).unwrap().version,
        "1.2.2"
    );
}

#[test]
fn offline_remote_registry_is_cache_or_named_error() {
    let tmp = TempDir::new("reg-offline");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    write_config(&home, "registries:\n  - tfs:github:o/r\n");
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert("TEBAKO_OFFLINE".into(), "1".into());

    // no cache → the named error (never a network attempt)
    let err = resolve::resolve("metanorma", &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("TEBAKO_OFFLINE"), "{}", err.message);
    assert!(err.message.contains("tfs:github:o/r"), "{}", err.message);

    // a seeded cache resolves offline
    seed_remote_cache(&home, "tfs:github:o/r", "1.2.3");
    let res = resolve::resolve("metanorma", &ctx).unwrap();
    assert_eq!(res.version, "1.2.3");
    match res.source {
        VersionSource::RegistryDefault(reg) => assert_eq!(reg, "tfs:github:o/r"),
        other => panic!("expected RegistryDefault, got {other:?}"),
    }
}

#[test]
fn unparseable_and_unreadable_registries_are_named_errors() {
    let tmp = TempDir::new("reg-broken");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let reg = tmp.path().join("tpkg-registry.yaml");
    std::fs::write(&reg, "schema_version: 2\npayloads: []\n").unwrap();
    write_config_with_registry(&home, &reg);
    let err = resolve::resolve("metanorma", &ctx(&home, tmp.path())).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("schema_version"), "{}", err.message);

    // a ref outside the spec 04 §2 forms is a named error listing them
    write_config(&home, "registries:\n  - https://cdn.example.com/r.yaml\n");
    let err = resolve::resolve("metanorma", &ctx(&home, tmp.path())).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(
        err.message.contains("invalid registry reference"),
        "{}",
        err.message
    );
}

#[test]
fn update_registries_refreshes_and_reports_failures() {
    let tmp = TempDir::new("reg-update");
    let home = tmp.path().join("home");
    let reg = file_registry(tmp.path(), "1.0.0");
    write_config_with_registry(&home, &reg);
    let ctx = ctx(&home, tmp.path());

    let argv: Vec<String> = vec!["tebako-shim".into(), "update-registries".into()];
    let (text, code) = printed(tebako_shim::run(&argv, &ctx).unwrap());
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("refreshed file://"), "{text}");
    assert!(only_meta(&home).is_file());

    // a failing registry is named and flips the exit code
    write_config(
        &home,
        &format!(
            "registries:\n  - file://{}\n  - file://{}\n",
            reg.display(),
            tmp.path().join("missing.yaml").display()
        ),
    );
    let (text, code) = printed(tebako_shim::run(&argv, &ctx).unwrap());
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("refreshed file://"), "{text}");
    assert!(text.contains("failed file://"), "{text}");
}

#[test]
fn doctor_reports_registry_freshness() {
    let tmp = TempDir::new("reg-doctor");
    let home = tmp.path().join("home");
    let reg = file_registry(tmp.path(), "1.0.0");
    write_config_with_registry(&home, &reg);
    let ctx = ctx(&home, tmp.path());
    let argv: Vec<String> = vec!["tebako-shim".into(), "doctor".into()];

    // nothing cached yet: the file mirror is a note, not a problem
    let (text, _) = printed(tebako_shim::run(&argv, &ctx).unwrap());
    assert!(text.contains("local mirror (not yet cached)"), "{text}");

    // after a refresh the mirror reports fresh
    tebako_shim::run(&["tebako-shim".into(), "update-registries".into()], &ctx).unwrap();
    let (text, _) = printed(tebako_shim::run(&argv, &ctx).unwrap());
    assert!(text.contains("cached, fresh"), "{text}");

    // past the TTL the freshness report turns into a problem
    backdate_meta(&home, 26 * 3600);
    let (text, code) = printed(tebako_shim::run(&argv, &ctx).unwrap());
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("cache is stale"), "{text}");
    assert!(text.contains("update-registries"), "{text}");

    // a remote ref with no cache entry is a problem pointing at the
    // refresh command (doctor never touches the network)
    write_config(&home, "registries:\n  - tfs:github:o/r\n");
    let (text, code) = printed(tebako_shim::run(&argv, &ctx).unwrap());
    assert_eq!(code, 1, "{text}");
    assert!(
        text.contains("registry tfs:github:o/r: not in the dispatch-time cache"),
        "{text}"
    );
}
