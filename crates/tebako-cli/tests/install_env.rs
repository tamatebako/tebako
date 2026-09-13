//! The env-mutating legs of the install surface (TEBAKO_OFFLINE,
//! TEBAKO_REQUIRE_SIGNED) — a separate test binary because those
//! variables are process-global and would race the parallel suite in
//! install.rs. Within this file every test takes the one lock.

use std::fs;
use std::path::PathBuf;

use tebako_cli::install;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-cli-installenv-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Env {
    dir: PathBuf,
    home: PathBuf,
    shim_binary: PathBuf,
}

impl Env {
    fn new(tag: &str) -> Env {
        let dir = scratch(tag);
        let home = dir.join("home");
        let mirror = dir.join("mirror");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&mirror).unwrap();
        fs::write(mirror.join("app-1.0.tfs"), b"app-bytes").unwrap();
        let app_url = tebako_http::file_url(&mirror.join("app-1.0.tfs"));
        fs::write(
            mirror.join("tpkg-registry.yaml"),
            format!(
                "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {app_url}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [app]\n    default: 1.0\n",
            ),
        )
        .unwrap();
        let shim_binary = dir.join("tebako-shim");
        fs::write(&shim_binary, b"#!/bin/sh\n").unwrap();
        Env {
            dir,
            home,
            shim_binary,
        }
    }

    fn register(&self) {
        let reg_ref = tebako_http::file_url(&self.dir.join("mirror").join("tpkg-registry.yaml"));
        install::add_registry(&self.home, &reg_ref).unwrap();
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn offline_is_cache_hit_or_hard_error() {
    let _guard = ENV_LOCK.lock().unwrap();
    let env = Env::new("offline");
    env.register();

    std::env::set_var("TEBAKO_OFFLINE", "1");
    // a miss is the named hard error — no download attempted
    let err = install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap_err();
    assert_eq!(err.code, 69, "{err:?}");
    assert!(err.message.contains("TEBAKO_OFFLINE"), "{err:?}");

    // install online, then a reinstall offline is a clean cache hit
    std::env::remove_var("TEBAKO_OFFLINE");
    install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap();
    std::env::set_var("TEBAKO_OFFLINE", "yes");
    let out = install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap();
    assert_eq!(out.status, tebako_resolve::InstallStatus::Hit);
    std::env::remove_var("TEBAKO_OFFLINE");
}

#[test]
fn require_signed_hard_fails_unsigned_entries() {
    let _guard = ENV_LOCK.lock().unwrap();
    let env = Env::new("reqsigned");
    env.register();

    std::env::set_var("TEBAKO_REQUIRE_SIGNED", "1");
    let err = install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap_err();
    assert_eq!(err.code, 71, "{err:?}");
    assert!(err.message.contains("TEBAKO_REQUIRE_SIGNED"), "{err:?}");
    assert!(!env.home.join("payloads/app/1.0.tfs").exists());
    std::env::remove_var("TEBAKO_REQUIRE_SIGNED");

    // without it the same entry installs (legacy warn path)
    install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap();
    assert!(env.home.join("payloads/app/1.0.tfs").exists());
}

// ---------------------------------------------------------------------
// spec 09 §4's press-time point under TEBAKO_REQUIRE_SIGNED=1: the
// compose closure's and the spawn walk's payload fetches fail closed on
// an unsigned entry — before a byte enters the cache.
// ---------------------------------------------------------------------

/// A ZIP image carrying the given manifest at the well-known path (the
/// tfs ZIP backend's shape — the same helper the compose/spawn suites
/// carry).
fn zip_image_with_manifest(manifest_yaml: &str) -> Vec<u8> {
    use std::io::Write as _;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("__tpkg__/manifest.yaml", options)
        .unwrap();
    writer.write_all(manifest_yaml.as_bytes()).unwrap();
    writer.start_file("app/bin/app", options).unwrap();
    writer.write_all(b"#!/bin/sh\n").unwrap();
    writer.finish().unwrap().into_inner()
}

#[test]
fn require_signed_hard_fails_unsigned_compose_slices() {
    let _guard = ENV_LOCK.lock().unwrap();
    let env = Env::new("reqsigned-compose");
    env.register();

    std::env::set_var("TEBAKO_REQUIRE_SIGNED", "1");
    let (doc, warnings) = tpkg::parse_compose(
        "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nslices:\n  - {name: app, requirement: \"1.0\"}\n",
    )
    .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let err = tebako_cli::compose::resolve_closure(
        &env.home,
        &tebako_resolve::Fetcher::new(),
        &doc,
        tpkg::ComposePreset::SharedRuntime,
        tpkg::Platform::host(),
    )
    .unwrap_err();
    assert_eq!(err.code, 71, "{err:?}");
    assert!(err.message.contains("TEBAKO_REQUIRE_SIGNED"), "{err:?}");
    assert!(!env.home.join("payloads/app/1.0.tfs").exists());
    std::env::remove_var("TEBAKO_REQUIRE_SIGNED");

    // without it the same slice resolves (legacy warn + journal line)
    tebako_cli::compose::resolve_closure(
        &env.home,
        &tebako_resolve::Fetcher::new(),
        &doc,
        tpkg::ComposePreset::SharedRuntime,
        tpkg::Platform::host(),
    )
    .unwrap();
    assert!(env.home.join("payloads/app/1.0.tfs").exists());
    let journal = fs::read_to_string(env.home.join("journal.log")).unwrap();
    assert!(
        journal.contains("event=legacy-unsigned-accepted"),
        "{journal}"
    );
}

#[test]
fn require_signed_hard_fails_unsigned_spawn_providers() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = scratch("reqsigned-spawn");
    let home = dir.join("home");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&mirror).unwrap();

    // the unsigned provider payload + its registry
    let provider = zip_image_with_manifest(
        "identity:\n  schema_version: 1\n  kind: app\n  name: xml2rfc\n  version: \"3.2.1\"\n  producer: {tool: tebako, tool_version: 0.15.9}\n  created: \"2026-09-12T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n    blob_sha256: \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"\n  signing: {state: unsigned}\n  encryption: {state: none}\nprovides:\n  entrypoints:\n    - name: xml2rfc\n      path: /app/bin/xml2rfc\n      runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n  platforms: universal\n  capabilities: {exec: true, read: true}\nrequires:\n  - kind: language\n    engine: ruby\n    constraint: \"~> 3.4\"\n",
    );
    fs::write(mirror.join("xml2rfc-3.2.1.tfs"), &provider).unwrap();
    let provider_ref = tebako_http::file_url(&mirror.join("xml2rfc-3.2.1.tfs"));
    fs::write(
        mirror.join("registry.yaml"),
        format!(
            "schema_version: 1\npayloads:\n  - name: xml2rfc\n    kind: app\n    versions:\n      - version: 3.2.1\n        platforms: universal\n        release: {{ref: {provider_ref}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [xml2rfc]\n    default: 3.2.1\n"
        ),
    )
    .unwrap();
    let reg_ref = tebako_http::file_url(&mirror.join("registry.yaml"));
    install::add_registry(&home, &reg_ref).unwrap();

    // the app under press carries the expose-bearing executable edge
    let app = dir.join("app.tfs");
    fs::write(
        &app,
        zip_image_with_manifest(
            "identity:\n  schema_version: 1\n  kind: app\n  name: mn\n  version: \"1.0\"\n  producer: {tool: tebako, tool_version: 0.15.9}\n  created: \"2026-09-12T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n    blob_sha256: \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"\n  signing: {state: unsigned}\n  encryption: {state: none}\nprovides:\n  entrypoints:\n    - name: mn\n      path: /app/bin/mn\n      runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n  platforms: universal\n  capabilities: {exec: true, read: true}\nrequires:\n  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n",
        ),
    )
    .unwrap();

    std::env::set_var("TEBAKO_REQUIRE_SIGNED", "1");
    let err = tebako_cli::spawn::resolve_spawned_edges(
        || Ok(home.clone()),
        &tebako_resolve::Fetcher::new(),
        &app,
        "mn",
        tpkg::ComposePreset::SelfContained,
        tpkg::Platform::host(),
        1,
    )
    .unwrap_err();
    assert_eq!(err.code, 71, "{err:?}");
    assert!(err.message.contains("TEBAKO_REQUIRE_SIGNED"), "{err:?}");
    assert!(
        !home.join("payloads/xml2rfc/3.2.1.tfs").exists(),
        "nothing was cached"
    );
    std::env::remove_var("TEBAKO_REQUIRE_SIGNED");
    let _ = fs::remove_dir_all(&dir);
}
