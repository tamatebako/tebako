//! Binary-level smoke of the registry/install surface: main.rs argv
//! handling end to end (add-registry → list-registries → install →
//! uninstall) against a file:// mirror and a temp TEBAKO_HOME.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::Command;

/// The registered shim path for a command — windows names it
/// `<command>.exe` (production's own mapping, tebako-shim#manage).
fn shim_path(home: &std::path::Path, command: &str) -> PathBuf {
    home.join("shims")
        .join(tebako_shim::manage::shim_file_name(command))
}

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-reg-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(home: &PathBuf, shim: &PathBuf, args: &[&str]) -> (i32, String) {
    let out = Command::new(tebako_bin())
        .args(args)
        .env("TEBAKO_HOME", home)
        .env("TEBAKO_SHIM_BINARY", shim)
        .output()
        .unwrap_or_else(|e| panic!("spawn failed: {e}"));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn registry_install_uninstall_smoke() {
    let dir = scratch("smoke");
    let home = dir.join("home");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    let shim = dir.join("tebako-shim");
    fs::write(&shim, b"#!/bin/sh\n").unwrap();

    fs::write(mirror.join("app-1.0.tfs"), b"app-bytes").unwrap();
    let app_url = tebako_http::file_url(&mirror.join("app-1.0.tfs"));
    fs::write(
        mirror.join("tpkg-registry.yaml"),
        format!(
            "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {app_url}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [app]\n    default: 1.0\n",
        ),
    )
    .unwrap();
    let reg_ref = tebako_http::file_url(&mirror.join("tpkg-registry.yaml"));

    let (code, text) = run(&home, &shim, &["add-registry", &reg_ref]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("registered registry"), "{text}");
    assert!(text.contains("1 payload(s): app"), "{text}");

    let (code, text) = run(&home, &shim, &["list-registries"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains(&reg_ref), "{text}");

    // update-registries: a file:// registry is reported as local (nothing
    // to cache), exit 0
    let (code, text) = run(&home, &shim, &["update-registries"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("nothing to cache"), "{text}");

    let (code, text) = run(&home, &shim, &["install", "app"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("installed app 1.0"), "{text}");
    assert!(home.join("payloads/app/1.0.tfs").is_file(), "{text}");
    assert!(shim_path(&home, "app").exists(), "{text}");

    // the legacy unsigned warning fires on stderr but does not fail
    assert!(text.contains("WARNING"), "{text}");

    // nickname resolution error surfaces with the registry list + hint
    let (code, text) = run(&home, &shim, &["install", "bogus"]);
    assert_ne!(code, 0);
    assert!(
        text.contains("no registered registry carries a payload named 'bogus'"),
        "{text}"
    );
    assert!(text.contains("tebako add-registry <ref>"), "{text}");

    let (code, text) = run(&home, &shim, &["uninstall", "app"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("removed app (1.0)"), "{text}");
    assert!(!home.join("payloads/app").exists());
    assert!(!shim_path(&home, "app").exists());

    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// #459 — add-registry → install in one home, no refresh (the bench
// dogfood scenario): the registered set IS visible to install; a
// `requires:` edge whose carrier is not registered is the named 65,
// and registering the carrier lets the same home's retry proceed.
// ---------------------------------------------------------------------

/// A zip image with an embedded manifest (the tfs zip backend reads it)
/// — the same fixture shape the library-level install tests use.
fn zip_image(manifest_yaml: &str) -> Vec<u8> {
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

/// An app image whose embedded manifest carries one toolkit `requires:`
/// edge (the metanorma → inkscape shape that #459 dogfooded).
fn app_image_requiring(name: &str, version: &str, dep: &str, constraint: &str) -> Vec<u8> {
    zip_image(&format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /app/bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\nrequires:\n  - kind: toolkit\n    name: {dep}\n    constraint: \"{constraint}\"\n    mount: /opt/{dep}\n",
        "a".repeat(64),
        "b".repeat(64)
    ))
}

/// A toolkit image: an embedded kind:toolkit manifest with no
/// executables (a pure mountable layer — nothing to materialize).
fn toolkit_image(name: &str, version: &str) -> Vec<u8> {
    zip_image(&format!(
        "identity:\n  schema_version: 1\n  kind: toolkit\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  platforms: universal\n  capabilities: {{exec: false, read: true}}\n",
        "a".repeat(64),
        "b".repeat(64)
    ))
}

#[test]
fn add_registry_then_install_resolves_the_requires_closure_no_refresh() {
    let dir = scratch("closure");
    let home = dir.join("home");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    let shim = dir.join("tebako-shim");
    fs::write(&shim, b"#!/bin/sh\n").unwrap();

    fs::write(
        mirror.join("app-1.0.tfs"),
        app_image_requiring("app", "1.0", "demotool", ">= 1.3"),
    )
    .unwrap();
    let app_url = tebako_http::file_url(&mirror.join("app-1.0.tfs"));
    fs::write(
        mirror.join("app-registry.yaml"),
        format!(
            "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {app_url}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [app]\n    default: 1.0\n",
        ),
    )
    .unwrap();
    let app_reg = tebako_http::file_url(&mirror.join("app-registry.yaml"));

    fs::write(
        mirror.join("demotool-1.4.tfs"),
        toolkit_image("demotool", "1.4"),
    )
    .unwrap();
    let tool_url = tebako_http::file_url(&mirror.join("demotool-1.4.tfs"));
    fs::write(
        mirror.join("demotool-registry.yaml"),
        format!(
            "schema_version: 1\npayloads:\n  - name: demotool\n    kind: toolkit\n    versions:\n      - version: \"1.4\"\n        platforms: universal\n        release: {{ref: {tool_url}}}\n    default: \"1.4\"\n",
        ),
    )
    .unwrap();
    let tool_reg = tebako_http::file_url(&mirror.join("demotool-registry.yaml"));

    // add-registry → install in one home, no refresh: the freshly
    // registered registry IS seen (the #459 claim). The install fails on
    // the DEPENDENCY nobody carries — the error names it and lists the
    // registered registry, proving visibility.
    let (code, text) = run(&home, &shim, &["add-registry", &app_reg]);
    assert_eq!(code, 0, "{text}");
    let (code, text) = run(&home, &shim, &["install", "app@1.0"]);
    assert_eq!(code, 65, "{text}");
    assert!(
        text.contains("app 1.0 requires toolkit demotool (>= 1.3)"),
        "{text}"
    );
    assert!(text.contains("no registered registry carries it"), "{text}");
    assert!(
        text.contains(&app_reg),
        "the registered registry is listed — install saw it\n{text}"
    );

    // Register the carrier and retry in the SAME home, still no refresh:
    // the walk resolves the edge and the closure lands.
    let (code, text) = run(&home, &shim, &["add-registry", &tool_reg]);
    assert_eq!(code, 0, "{text}");
    let (code, text) = run(&home, &shim, &["install", "app@1.0"]);
    assert_eq!(code, 0, "{text}");
    // the retry is a cache hit for the app plus the closure walk — the
    // failed first attempt left the app's verified record (resume
    // semantics, library-pinned by install.rs's dep_walk_missing_dep
    // test: "the app stays installed")
    assert!(text.contains("app 1.0 is already installed"), "{text}");
    assert!(home.join("payloads/app/1.0.tfs").is_file(), "{text}");
    assert!(
        home.join("payloads/demotool/1.4.tfs").is_file(),
        "the dependency closure landed\n{text}"
    );
    assert!(home.join("payloads/demotool/1.4.manifest.yaml").is_file());
    assert!(
        !shim_path(&home, "demotool").exists(),
        "an executable-less toolkit gets no shim"
    );

    let _ = fs::remove_dir_all(&dir);
}
