//! `tebako info` / `tebako inspect` surface tests (spec 15 §4): the
//! store/system views and the artifact umbrella verb — temp TEBAKO_HOMEs
//! and fake-byte payloads, no network.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-info-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Fixture {
    dir: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let dir = scratch(tag);
        let home = dir.join("home");
        fs::create_dir_all(&home).unwrap();
        Fixture { dir, home }
    }

    /// A cached payload with a mirror and origin marker.
    fn payload(&self, name: &str, version: &str, kind: &str) {
        let dir = self.home.join(format!("payloads/{name}"));
        fs::create_dir_all(&dir).unwrap();
        let image = dir.join(format!("{version}.tfs"));
        fs::write(&image, zip_image_with_manifest(name, version, kind)).unwrap();
        fs::write(
            dir.join(format!("{version}.tfs.sha256")),
            format!("{}  {version}.tfs\n", tebako_resolve::sha256_hex(&fs::read(&image).unwrap())),
        )
        .unwrap();
        fs::write(
            dir.join(format!("{version}.tfs.origin")),
            format!("url=file:///mirror/{name}-{version}.tfs\n"),
        )
        .unwrap();
        // the mirror (the dispatcher-visible record)
        let mirror = match kind {
            "app" => format!(
                "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \"~> 3.3.0\", abi: \"arm64-darwin-23\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
                "0".repeat(64),
                "0".repeat(64)
            ),
            _ => format!(
                "identity:\n  schema_version: 1\n  kind: toolkit\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  platforms: universal\n  capabilities: {{exec: false, read: true}}\n",
                "0".repeat(64),
                "0".repeat(64)
            ),
        };
        fs::write(dir.join(format!("{version}.manifest.yaml")), mirror).unwrap();
    }

    /// A cached runtime entry.
    fn runtime(&self, lv: &str, tebako: &str) {
        let platform = tebako_shim::runtime::platform_string();
        let dir = self.home.join(format!("runtimes/ruby-{lv}-{tebako}-{platform}"));
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join(format!("tebako-runtime-{tebako}-{lv}-{platform}"));
        fs::write(&exe, b"fake runtime exe\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(
            dir.join("manifest.json"),
            format!(
                "[{{\"filename\": \"tebako-runtime-{tebako}-{lv}-{platform}\", \"abi\": \"arm64-darwin-23\"}}]\n"
            ),
        )
        .unwrap();
    }

    fn shim(&self, name: &str) {
        let dir = self.home.join("shims");
        fs::create_dir_all(&dir).unwrap();
        let target = self.dir.join("tebako-shim");
        fs::write(&target, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, dir.join(name)).unwrap();
        #[cfg(windows)]
        fs::copy(&target, dir.join(name)).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A zip image with an embedded manifest (the tfs zip backend reads it).
fn zip_image_with_manifest(name: &str, version: &str, kind: &str) -> Vec<u8> {
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: {kind}\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \"~> 3.3.0\", abi: \"arm64-darwin-23\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        "0".repeat(64),
        "0".repeat(64)
    );
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("__tpkg__/manifest.yaml", options)
        .unwrap();
    writer.write_all(manifest.as_bytes()).unwrap();
    writer.start_file(format!("bin/{name}"), options).unwrap();
    writer.write_all(b"#!/bin/sh\n").unwrap();
    writer.finish().unwrap().into_inner()
}

// ---------------------------------------------------------------------
// tebako info
// ---------------------------------------------------------------------

#[test]
fn info_system_reports_the_machine_view() {
    let fx = Fixture::new("system");
    fx.runtime("3.3.7", "0.16.0");
    fx.payload("app", "1.0", "app");
    fx.payload("inkscape", "1.4.3", "toolkit");
    fx.shim("app");

    let (out, code) = tebako_cli::info::run(&fx.home, None, false, false).unwrap();
    assert_eq!(code, 0);
    assert!(out.contains("platform:"), "{out}");
    assert!(out.contains("runtimes: 1 cached"), "{out}");
    assert!(out.contains("payloads: 2 cached"), "{out}");
    assert!(out.contains("shims: 1 registered"), "{out}");

    let (json, _) = tebako_cli::info::run(&fx.home, Some("system"), false, true).unwrap();
    assert!(json.contains("\"info_schema\": 1"), "{json}");
    assert!(json.contains("\"runtimes\": 1"), "{json}");
}

#[test]
fn info_runtimes_lists_cached_with_abi() {
    let fx = Fixture::new("runtimes");
    fx.runtime("3.3.7", "0.16.0");
    fx.runtime("4.0.6", "0.16.0");

    let (out, _) = tebako_cli::info::run(&fx.home, Some("runtimes"), false, false).unwrap();
    assert!(out.contains("ruby 3.3.7 (tebako 0.16.0)"), "{out}");
    assert!(out.contains("ruby 4.0.6 (tebako 0.16.0)"), "{out}");
    assert!(out.contains("abi arm64-darwin-23"), "{out}");

    let (json, _) = tebako_cli::info::run(&fx.home, Some("runtimes"), false, true).unwrap();
    assert!(json.contains("\"abi\": \"arm64-darwin-23\""), "{json}");
}

#[test]
fn info_payloads_lists_cached_with_origin() {
    let fx = Fixture::new("payloads");
    fx.payload("metanorma", "1.16.9", "app");
    fx.payload("inkscape", "1.4.3", "toolkit");

    let (out, _) = tebako_cli::info::run(&fx.home, Some("payloads"), false, false).unwrap();
    assert!(out.contains("metanorma 1.16.9 (App)"), "{out}");
    assert!(out.contains("inkscape 1.4.3 (Toolkit)"), "{out}");
    assert!(out.contains("file:///mirror/metanorma-1.16.9.tfs"), "{out}");
}

#[test]
fn info_shims_previews_the_dispatch() {
    let fx = Fixture::new("shims");
    fx.payload("app", "1.0", "app");
    fx.shim("app");
    fs::write(fx.home.join("config.yaml"), "defaults:\n  app: 1.0\n").unwrap();

    let (out, _) = tebako_cli::info::run(&fx.home, Some("shims"), false, false).unwrap();
    assert!(out.contains("app → app 1.0"), "{out}");
    assert!(out.contains("runtime: ruby ~> 3.3.0"), "{out}");
}

#[test]
fn info_registries_reports_freshness() {
    let fx = Fixture::new("registries");
    fs::write(
        fx.home.join("config.yaml"),
        "registries:\n  - file:///tmp/none/tpkg-registry.yaml\n",
    )
    .unwrap();

    let (out, _) = tebako_cli::info::run(&fx.home, Some("registries"), false, false).unwrap();
    assert!(out.contains("file:///tmp/none/tpkg-registry.yaml"), "{out}");
    assert!(out.contains("local"), "{out}");
}

#[test]
fn info_store_breaks_down_disk_usage() {
    let fx = Fixture::new("store");
    fx.runtime("3.3.7", "0.16.0");
    fx.payload("app", "1.0", "app");

    let (out, _) = tebako_cli::info::run(&fx.home, Some("store"), false, false).unwrap();
    assert!(out.contains("runtimes"), "{out}");
    assert!(out.contains("payloads"), "{out}");

    let (json, _) = tebako_cli::info::run(&fx.home, Some("store"), false, true).unwrap();
    assert!(json.contains("\"total_bytes\":"), "{json}");
}

#[test]
fn info_unknown_topic_is_a_named_error() {
    let fx = Fixture::new("badtopic");
    let err = tebako_cli::info::run(&fx.home, Some("bogus"), false, false).unwrap_err();
    assert_eq!(err.code, 64);
    assert!(err.message.contains("bogus"), "{err:?}");
}

// ---------------------------------------------------------------------
// tebako inspect
// ---------------------------------------------------------------------

#[test]
fn inspect_payload_summary_and_sections() {
    let fx = Fixture::new("inspect");
    let image = fx.dir.join("app-1.0.tfs");
    fs::write(&image, zip_image_with_manifest("app", "1.0", "app")).unwrap();

    let opts = tebako_cli::inspect::InspectOptions::default();
    let (out, code) = tebako_cli::inspect::inspect(&image, &opts).unwrap();
    assert_eq!(code, 0);
    assert!(out.contains("kind: app"), "{out}");
    assert!(out.contains("name: app"), "{out}");
    assert!(out.contains("version: 1.0"), "{out}");

    let opts = tebako_cli::inspect::InspectOptions {
        provides: true,
        ..Default::default()
    };
    let (out, _) = tebako_cli::inspect::inspect(&image, &opts).unwrap();
    assert!(out.contains("entrypoint app → /bin/app"), "{out}");
    assert!(out.contains("runtime: ruby ~> 3.3.0"), "{out}");
    assert!(out.contains("abi arm64-darwin-23"), "{out}");
}

#[test]
fn inspect_json_carries_the_manifest() {
    let fx = Fixture::new("inspectjson");
    let image = fx.dir.join("app-1.0.tfs");
    fs::write(&image, zip_image_with_manifest("app", "1.0", "app")).unwrap();

    let opts = tebako_cli::inspect::InspectOptions {
        json: true,
        ..Default::default()
    };
    let (out, code) = tebako_cli::inspect::inspect(&image, &opts).unwrap();
    assert_eq!(code, 0);
    let doc = tebako_json::parse(&out).unwrap();
    let manifest = doc.find("manifest").expect("manifest key");
    let identity = manifest.find("identity").expect("identity key");
    assert_eq!(
        identity.find("name").and_then(|n| n.as_string()).as_deref(),
        Some("app")
    );
}
