//! The press-time spawned-edge walk (spec 23 §13.6, spec 30 §2, spec 32
//! §6): `spawn::resolve_spawned_edges` composes the lock's `spawned[]`
//! rows from the app image's L1 `requires:` — runtime rows off the
//! machine store's cached runtimes, payload rows off `file://` fixture
//! registries. Everything runs against temp homes — no network, no env
//! mutation.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use tebako_cli::error::TebakoError;
use tebako_cli::spawn::{self, SpawnedPlan};
use tebako_resolve::{sha256_hex, Fetcher};
use tpkg::Platform;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-spawn-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A temp TEBAKO_HOME plus a mirror dir holding payload/registry files.
struct Fixture {
    dir: PathBuf,
    home: PathBuf,
    mirror: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let dir = scratch(tag);
        let home = dir.join("home");
        let mirror = dir.join("mirror");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&mirror).unwrap();
        Fixture { dir, home, mirror }
    }

    /// Write a mirror file and answer its file:// reference.
    fn mirror_file(&self, file: &str, bytes: &[u8]) -> String {
        fs::write(self.mirror.join(file), bytes).unwrap();
        tebako_http::file_url(&self.mirror.join(file))
    }

    /// The registered-registries config carrying exactly the given refs.
    fn register(&self, registries: &[String]) {
        let mut yaml = String::from("registries:\n");
        for r in registries {
            yaml.push_str(&format!("  - {r}\n"));
        }
        fs::write(self.home.join("config.yaml"), yaml).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn sha(byte: u8) -> String {
    String::from(char::from(byte)).repeat(64)
}

fn zip_image_with_manifest(manifest_yaml: &str) -> Vec<u8> {
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

/// The app image under press: a kind: app manifest whose own entrypoint
/// is `mn`, carrying the given `requires:` block verbatim.
fn app_image_with_requires(requires: &str) -> Vec<u8> {
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: mn\n  version: \"1.0\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-09-12T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: mn\n      path: /app/bin/mn\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\nrequires:\n{requires}",
        sha(b'a'),
        sha(b'b')
    );
    zip_image_with_manifest(&manifest)
}

/// A manifest-less image (the pre-era plain press shape).
fn manifestless_image() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("app/bin/app", options).unwrap();
    writer.write_all(b"#!/bin/sh\n").unwrap();
    writer.finish().unwrap().into_inner()
}

/// A cached java runtime store entry (spec 05 §3's grammar) whose env
/// image is a real zip image carrying a kind: runtime manifest with the
/// given spawn entrypoints; `index` is the cached release-index mirror
/// (manifest.json) when the entry carries one. Returns (exe, image).
fn cached_java_runtime(
    fx: &Fixture,
    entrypoints: &[&str],
    index: Option<String>,
) -> (PathBuf, PathBuf) {
    cached_runtime(fx, "java", "21.0.8", "0.3.0", entrypoints, index)
}

fn cached_ruby_runtime(fx: &Fixture) -> (PathBuf, PathBuf) {
    cached_runtime(fx, "ruby", "3.4.2", "0.3.0", &["ruby"], None)
}

fn cached_runtime(
    fx: &Fixture,
    engine: &str,
    lv: &str,
    ver: &str,
    entrypoints: &[&str],
    index: Option<String>,
) -> (PathBuf, PathBuf) {
    let platform = tebako_shim::runtime::platform_string();
    let dir = fx
        .home
        .join("runtimes")
        .join(format!("{engine}-{lv}-{ver}-{platform}"));
    fs::create_dir_all(&dir).unwrap();
    let exe_name = format!(
        "tebako-runtime-{ver}-{lv}-{platform}{}",
        tebako_shim::runtime::exe_suffix()
    );
    let exe = dir.join(&exe_name);
    fs::write(&exe, format!("fake {engine} runtime exe\n")).unwrap();
    let eps: String = entrypoints
        .iter()
        .map(|e| format!("    - {{name: {e}, path: /bin/{e}}}\n"))
        .collect();
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-{engine}\n  version: \"{lv}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-09-12T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  provides: {{engine: {engine}, version: \"{lv}\", abi_line: \"21\", platform: aarch64-macos}}\n  built_from: {{src_sha256: {}, patch_set: v1}}\n  entrypoints:\n{eps}  capabilities: {{exec: true, read: true, runtime: true}}\n",
        sha(b'a'),
        sha(b'b'),
        sha(b'c')
    );
    let image_bytes = zip_image_with_manifest(&manifest);
    let image_name = format!("tebako-runtime-{ver}-{lv}-{platform}.tfs");
    let image = dir.join(&image_name);
    fs::write(&image, &image_bytes).unwrap();
    fs::write(
        dir.join(format!("{image_name}.sha256")),
        format!("{}  {image_name}\n", sha256_hex(&image_bytes)),
    )
    .unwrap();
    if let Some(index) = index {
        fs::write(dir.join("manifest.json"), index).unwrap();
    }
    (exe, image)
}

/// The provider app image (spec 32): a kind: app manifest whose
/// entrypoint carries a runtime_requirement, plus — when given — the
/// `kind: language` edge the nested runtime row mirrors.
fn provider_image(name: &str, version: &str, entrypoint: &str, language: Option<&str>) -> Vec<u8> {
    let requires = match language {
        Some(constraint) => format!(
            "requires:\n  - kind: language\n    engine: ruby\n    constraint: \"{constraint}\"\n"
        ),
        None => String::new(),
    };
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-09-12T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {entrypoint}\n      path: /app/bin/{entrypoint}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n{requires}",
        sha(b'a'),
        sha(b'b')
    );
    zip_image_with_manifest(&manifest)
}

/// A provider whose entrypoint is runtime-less (the exec tier — no
/// spawn form, spec 32 §0/§1).
fn runtime_less_provider_image(name: &str, version: &str, entrypoint: &str) -> Vec<u8> {
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-09-12T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {entrypoint}\n      path: /app/bin/{entrypoint}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    );
    zip_image_with_manifest(&manifest)
}

/// The provider's registry document (entrypoints mirrored, universal).
fn provider_registry_yaml(
    name: &str,
    version: &str,
    payload_ref: &str,
    entrypoint: &str,
) -> String {
    format!(
        "schema_version: 1\npayloads:\n  - name: {name}\n    kind: app\n    versions:\n      - version: {version}\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [{entrypoint}]\n    default: {version}\n"
    )
}

/// Write the app image into the fixture and answer its path.
fn app_image(fx: &Fixture, bytes: &[u8]) -> PathBuf {
    let path = fx.dir.join("app.tfs");
    fs::write(&path, bytes).unwrap();
    path
}

/// The walk against the fixture home, self-contained, first slot 1.
fn walk(fx: &Fixture, app: &Path) -> Result<SpawnedPlan, TebakoError> {
    spawn::resolve_spawned_edges(
        || Ok(fx.home.clone()),
        &Fetcher::new(),
        app,
        "mn",
        tpkg::ComposePreset::SelfContained,
        Platform::host(),
        1,
    )
}

// ---------------------------------------------------------------------
// the runtime edge (spec 30 §2 / spec 23 §13.6)
// ---------------------------------------------------------------------

#[test]
fn runtime_edge_composes_the_carried_row_field_by_field() {
    let fx = Fixture::new("rt-row");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21, < 26\"\n    expose: [java]\n",
        ),
    );
    let (exe, image) = cached_java_runtime(&fx, &["java", "keytool"], None);

    let plan = walk(&fx, &app).unwrap();

    assert_eq!(plan.rows.len(), 1);
    let tpkg::LockedSpawned::Runtime(row) = &plan.rows[0] else {
        panic!("a runtime row: {:?}", plan.rows[0]);
    };
    // the L1 edge mirrored verbatim
    assert_eq!(row.engine, "java");
    assert_eq!(row.implementation, None);
    assert_eq!(row.constraint.as_str(), ">= 21, < 26");
    assert_eq!(row.expose, vec!["java".to_string()]);
    // the press-time pick
    assert_eq!(row.version, "21.0.8");
    assert_eq!(row.tebako, "0.3.0");
    // carried, with the store bytes' pins; source is the shared-row key
    assert!(row.carry);
    assert_eq!(row.source, None);
    assert_eq!(row.exe.slot, Some(1));
    assert_eq!(
        row.exe.sha256,
        tpkg::DigestPin::One(sha256_hex(&fs::read(&exe).unwrap()))
    );
    assert_eq!(row.exe.install_as, None);
    assert_eq!(row.image.slot, Some(2));
    assert_eq!(
        row.image.sha256,
        tpkg::DigestPin::One(sha256_hex(&fs::read(&image).unwrap()))
    );
    assert_eq!(row.dll, None);
    // the carried bytes join the press image list: exe AUTO, image
    // DWARFS, both with the empty mount (a carried spawned artifact is
    // never mounted)
    assert_eq!(
        plan.images,
        vec![
            (exe, String::new(), tpkg::TPKG_FORMAT_AUTO),
            (image, String::new(), tpkg::TPKG_FORMAT_DWARFS),
        ]
    );
}

#[test]
fn platform_conditioned_edges_skip_or_compose_per_the_target_host() {
    // spec 03 §2.3 (schema_minor 9): a spawn edge whose triplets: list
    // does not cover the press's target host contributes NO spawned row
    // and NO carried bytes — the java runtime is deliberately NOT in the
    // store: any resolution attempt on the skipped edge would fail by
    // name. The covering list composes exactly as an unconditioned edge.
    let fx = Fixture::new("edge-skip");
    let skipped = Platform::ALL
        .iter()
        .find(|p| **p != Platform::host() && !p.is_reserved())
        .unwrap()
        .as_triplet();
    let app = app_image(
        &fx,
        &app_image_with_requires(&format!(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n    expose: [java]\n    triplets: [{skipped}]\n"
        )),
    );
    let plan = walk(&fx, &app).unwrap();
    assert!(plan.rows.is_empty(), "the skipped edge composes no row");
    assert!(plan.images.is_empty(), "the skipped edge carries no bytes");

    let host = Platform::host().as_triplet();
    let app = app_image(
        &fx,
        &app_image_with_requires(&format!(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n    expose: [java]\n    triplets: [{host}]\n"
        )),
    );
    let _ = cached_java_runtime(&fx, &["java", "keytool"], None);
    let plan = walk(&fx, &app).unwrap();
    assert_eq!(plan.rows.len(), 1, "the covering edge composes its row");
}

#[test]
fn runtime_edge_without_expose_still_composes_a_row() {
    // The row mirrors the edge whether or not it exposes names (the
    // validate reverse check requires a row for EVERY runtime edge).
    let fx = Fixture::new("rt-noexpose");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n",
        ),
    );
    cached_java_runtime(&fx, &["java"], None);

    let plan = walk(&fx, &app).unwrap();

    assert_eq!(plan.rows.len(), 1);
    let tpkg::LockedSpawned::Runtime(row) = &plan.rows[0] else {
        panic!("a runtime row: {:?}", plan.rows[0]);
    };
    assert_eq!(row.expose, Vec::<String>::new());
    assert!(row.carry);
    assert_eq!(row.exe.slot, Some(1));
    assert_eq!(row.image.slot, Some(2));
    assert_eq!(plan.images.len(), 2);
}

#[test]
fn runtime_edge_expose_outside_the_spawn_surface_is_a_named_error() {
    let fx = Fixture::new("rt-badexpose");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n    expose: [javac]\n",
        ),
    );
    cached_java_runtime(&fx, &["java"], None);

    let err = walk(&fx, &app).unwrap_err();
    assert_eq!(err.code, 65);
    assert!(
        err.message.contains("declares no entrypoint \"javac\""),
        "{err:?}"
    );
}

#[test]
fn expose_colliding_with_the_apps_own_entrypoint_is_a_named_error() {
    // spec 30 §3 / spec 32 §1: an exposed name never collides with the
    // app payload's own entries — the MANIFEST VALIDATOR owns the refusal
    // (the app's own entrypoint here is "mn"); the walk surfaces it as
    // the named manifest error at parse.
    let fx = Fixture::new("collision");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n    expose: [mn]\n",
        ),
    );
    cached_java_runtime(&fx, &["mn"], None);

    let err = walk(&fx, &app).unwrap_err();
    assert_eq!(err.code, 65);
    assert!(
        err.message
            .contains("collides with the payload's own entrypoint name"),
        "{err:?}"
    );
}

#[test]
fn duplicate_runtime_edges_are_a_named_error() {
    let fx = Fixture::new("rt-dup");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n",
        ),
    );
    cached_java_runtime(&fx, &["java"], None);

    let err = walk(&fx, &app).unwrap_err();
    assert_eq!(err.code, 65);
    assert!(
        err.message
            .contains("one lock.spawned[] row per engine+implementation"),
        "{err:?}"
    );
}

#[test]
fn shared_runtime_preset_refuses_the_spawned_runtime_row() {
    // spec 23 §13.6: press resolves spawned runtimes through the machine
    // store and records no replayable `source:` — the shared row is the
    // named error advising --mode=self-contained.
    let fx = Fixture::new("rt-shared");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n",
        ),
    );
    cached_java_runtime(&fx, &["java"], None);

    let err = spawn::resolve_spawned_edges(
        || Ok(fx.home.clone()),
        &Fetcher::new(),
        &app,
        "mn",
        tpkg::ComposePreset::SharedRuntime,
        Platform::host(),
        1,
    )
    .unwrap_err();
    assert_eq!(err.code, 65);
    assert!(err.message.contains("--mode=self-contained"), "{err:?}");
}

#[test]
fn the_dll_facet_rides_the_carried_row() {
    // tebako-runtime-ruby#40: the cached release index declares the dll
    // facet; the store staged it under install_as; the row pins it as the
    // pair's third slot.
    let fx = Fixture::new("rt-dll");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n",
        ),
    );
    let platform = tebako_shim::runtime::platform_string();
    let exe_name = format!(
        "tebako-runtime-0.3.0-21.0.8-{platform}{}",
        tebako_shim::runtime::exe_suffix()
    );
    let dll_bytes = b"fake jvm dll\n";
    let index = format!(
        "[{{\"filename\": \"{exe_name}\", \"dll\": {{\"filename\": \"jvm-assets.dll\", \"install_as\": \"jvm.dll\", \"sha256\": \"{}\"}}}}]",
        sha256_hex(dll_bytes)
    );
    let (exe, image) = cached_java_runtime(&fx, &["java"], Some(index));
    let dll_path = exe.parent().unwrap().join("jvm.dll");
    fs::write(&dll_path, dll_bytes).unwrap();

    let plan = walk(&fx, &app).unwrap();

    let tpkg::LockedSpawned::Runtime(row) = &plan.rows[0] else {
        panic!("a runtime row: {:?}", plan.rows[0]);
    };
    let dll = row.dll.as_ref().expect("the dll facet rides the row");
    assert_eq!(dll.slot, Some(3));
    assert_eq!(dll.install_as, Some("jvm.dll".to_string()));
    assert_eq!(dll.sha256, tpkg::DigestPin::One(sha256_hex(dll_bytes)));
    assert_eq!(
        plan.images,
        vec![
            (exe, String::new(), tpkg::TPKG_FORMAT_AUTO),
            (image, String::new(), tpkg::TPKG_FORMAT_DWARFS),
            (dll_path, String::new(), tpkg::TPKG_FORMAT_AUTO),
        ]
    );
}

// ---------------------------------------------------------------------
// the payload edge (spec 32 §6 / spec 23 §13.6)
// ---------------------------------------------------------------------

/// The pinned payload-edge fixture: the provider registered at 3.2.1,
/// the ruby runtime cached for the nested row. Returns the provider
/// image bytes (the pin's expectation).
fn payload_fixture(fx: &Fixture) -> Vec<u8> {
    let provider_bytes = provider_image("xml2rfc", "3.2.1", "xml2rfc", Some("~> 3.4"));
    let provider_ref = fx.mirror_file("xml2rfc-3.2.1.tfs", &provider_bytes);
    let registry_ref = fx.mirror_file(
        "xml2rfc-registry.yaml",
        provider_registry_yaml("xml2rfc", "3.2.1", &provider_ref, "xml2rfc").as_bytes(),
    );
    fx.register(&[registry_ref]);
    cached_ruby_runtime(fx);
    provider_bytes
}

#[test]
fn payload_edge_composes_the_carried_row_field_by_field() {
    let fx = Fixture::new("payload-row");
    let provider_bytes = payload_fixture(&fx);
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n",
        ),
    );

    let plan = walk(&fx, &app).unwrap();

    assert_eq!(plan.rows.len(), 1);
    let tpkg::LockedSpawned::Payload(row) = &plan.rows[0] else {
        panic!("a payload row: {:?}", plan.rows[0]);
    };
    // the L1 edge mirrored verbatim (the resolved provider name)
    assert_eq!(row.payload, "xml2rfc");
    assert_eq!(row.constraint.as_str(), ">= 3.0");
    assert_eq!(row.expose, vec!["xml2rfc".to_string()]);
    // the press-time pick; carried with the pin; no source on a carried row
    assert_eq!(row.version, "3.2.1");
    assert!(row.carry);
    assert_eq!(row.source, None);
    assert_eq!(row.image.slot, Some(1));
    assert_eq!(
        row.image.sha256,
        tpkg::DigestPin::One(sha256_hex(&provider_bytes))
    );
    // the nested runtime row: the provider's OWN language edge mirrored
    // verbatim (NOT the entrypoint's runtime_requirement), expose empty
    let rt = &row.runtime;
    assert_eq!(rt.engine, "ruby");
    assert_eq!(rt.implementation, None);
    assert_eq!(rt.constraint.as_str(), "~> 3.4");
    assert_eq!(rt.expose, Vec::<String>::new());
    assert_eq!(rt.version, "3.4.2");
    assert_eq!(rt.tebako, "0.3.0");
    assert!(rt.carry);
    assert_eq!(rt.source, None);
    assert_eq!(rt.exe.slot, Some(2));
    assert_eq!(rt.image.slot, Some(3));
    assert_eq!(rt.dll, None);
    // slot order: the provider image, then the nested exe, then the
    // nested image — all empty-mounted
    let paths: Vec<&Path> = plan.images.iter().map(|(p, _, _)| p.as_path()).collect();
    assert!(
        paths[0].ends_with("payloads/xml2rfc/3.2.1.tfs"),
        "{paths:?}"
    );
    assert!(
        paths[1]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("tebako-runtime-0.3.0-3.4.2-"),
        "{paths:?}"
    );
    assert!(
        paths[2].extension().is_some_and(|e| e == "tfs"),
        "{paths:?}"
    );
    assert_eq!(plan.images[0].2, tpkg::TPKG_FORMAT_AUTO);
    assert_eq!(plan.images[1].2, tpkg::TPKG_FORMAT_AUTO);
    assert_eq!(plan.images[2].2, tpkg::TPKG_FORMAT_DWARFS);
    assert!(plan.images.iter().all(|(_, mount, _)| mount.is_empty()));
}

#[test]
fn payload_edge_capability_scan_resolves_the_unpinned_provider() {
    // spec 32 §1 / spec 03 §8 at press: no `payload:` pin — the
    // registered registries' entrypoint scan answers the provider.
    let fx = Fixture::new("payload-scan");
    let provider_bytes = provider_image("xml2rfc-pkg", "3.2.1", "xml2rfc", Some("~> 3.4"));
    let provider_ref = fx.mirror_file("xml2rfc-pkg-3.2.1.tfs", &provider_bytes);
    let registry_ref = fx.mirror_file(
        "xml2rfc-registry.yaml",
        provider_registry_yaml("xml2rfc-pkg", "3.2.1", &provider_ref, "xml2rfc").as_bytes(),
    );
    fx.register(&[registry_ref]);
    cached_ruby_runtime(&fx);
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: executable\n    name: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n",
        ),
    );

    let plan = walk(&fx, &app).unwrap();

    assert_eq!(plan.rows.len(), 1);
    let tpkg::LockedSpawned::Payload(row) = &plan.rows[0] else {
        panic!("a payload row: {:?}", plan.rows[0]);
    };
    assert_eq!(row.payload, "xml2rfc-pkg");
    assert_eq!(row.version, "3.2.1");
    assert_eq!(
        row.image.sha256,
        tpkg::DigestPin::One(sha256_hex(&provider_bytes))
    );
}

#[test]
fn payload_edge_exposing_a_runtime_less_entrypoint_is_a_named_error() {
    // spec 32 §0/§1: the exec tier has no spawn form.
    let fx = Fixture::new("payload-runtmeless");
    let provider_bytes = runtime_less_provider_image("xml2rfc", "3.2.1", "xml2rfc");
    let provider_ref = fx.mirror_file("xml2rfc-3.2.1.tfs", &provider_bytes);
    let registry_ref = fx.mirror_file(
        "xml2rfc-registry.yaml",
        provider_registry_yaml("xml2rfc", "3.2.1", &provider_ref, "xml2rfc").as_bytes(),
    );
    fx.register(&[registry_ref]);
    cached_ruby_runtime(&fx);
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n",
        ),
    );

    let err = walk(&fx, &app).unwrap_err();
    assert_eq!(err.code, 65);
    assert!(err.message.contains("no spawn form"), "{err:?}");
}

#[test]
fn payload_edge_provider_without_a_language_edge_is_a_named_error() {
    // spec 32 §6: the nested row's constraint mirrors the provider's OWN
    // kind: language edge — absent, the row has no source.
    let fx = Fixture::new("payload-nolang");
    let provider_bytes = provider_image("xml2rfc", "3.2.1", "xml2rfc", None);
    let provider_ref = fx.mirror_file("xml2rfc-3.2.1.tfs", &provider_bytes);
    let registry_ref = fx.mirror_file(
        "xml2rfc-registry.yaml",
        provider_registry_yaml("xml2rfc", "3.2.1", &provider_ref, "xml2rfc").as_bytes(),
    );
    fx.register(&[registry_ref]);
    cached_ruby_runtime(&fx);
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n",
        ),
    );

    let err = walk(&fx, &app).unwrap_err();
    assert_eq!(err.code, 65);
    assert!(err.message.contains("no kind: language edge"), "{err:?}");
}

#[test]
fn two_edges_resolving_to_one_provider_are_a_named_error() {
    let fx = Fixture::new("payload-dup");
    let _ = payload_fixture(&fx);
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n  - kind: executable\n    name: rfc2html\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [rfc2html]\n",
        ),
    );

    let err = walk(&fx, &app).unwrap_err();
    assert_eq!(err.code, 65);
    assert!(
        err.message
            .contains("one lock.spawned[] row per provider payload"),
        "{err:?}"
    );
}

// ---------------------------------------------------------------------
// the negative space: no spawn edges, no work
// ---------------------------------------------------------------------

#[test]
fn app_image_without_a_manifest_composes_no_rows() {
    let fx = Fixture::new("no-manifest");
    let app = app_image(&fx, &manifestless_image());

    // The home closure must never fire for a spawn-less press.
    let plan = spawn::resolve_spawned_edges(
        || -> Result<PathBuf, TebakoError> { panic!("home must stay unresolved") },
        &Fetcher::new(),
        &app,
        "mn",
        tpkg::ComposePreset::SelfContained,
        Platform::host(),
        1,
    )
    .unwrap();
    assert!(plan.rows.is_empty());
    assert!(plan.images.is_empty());
}

#[test]
fn toolkit_only_requires_compose_no_rows() {
    let fx = Fixture::new("toolkit-only");
    let app = app_image(
        &fx,
        &app_image_with_requires(
            "  - kind: toolkit\n    name: openjdk\n    constraint: \">= 21\"\n    mount: /opt/openjdk\n",
        ),
    );

    let plan = spawn::resolve_spawned_edges(
        || -> Result<PathBuf, TebakoError> { panic!("home must stay unresolved") },
        &Fetcher::new(),
        &app,
        "mn",
        tpkg::ComposePreset::SelfContained,
        Platform::host(),
        1,
    )
    .unwrap();
    assert!(plan.rows.is_empty());
    assert!(plan.images.is_empty());
}
