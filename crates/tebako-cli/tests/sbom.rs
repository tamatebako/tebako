//! `tebako sbom` integration tests (spec 15 §8): real zip-backend images
//! (manifest + gemspecs), the store resolve, and the determinism law —
//! temp dirs only, no network.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-sbom-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Tests that pin `SOURCE_DATE_EPOCH` mutate process-global state; the
/// determinism tests hold this lock across set → emit → remove so a
/// parallel test in the same binary never observes the window.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn manifest_yaml(name: &str, version: &str, kind: &str, requires: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: {kind}\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 2.8.85}}\n  created: \"2026-11-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \"~> 4.0.0\", implementation: mri, abi: \"arm64-darwin-24\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n{requires}",
        "0".repeat(64),
        "0".repeat(64)
    )
}

const REQUIRES_RUBY: &str =
    "requires:\n  - kind: language\n    engine: ruby\n    constraint: \"~> 4.0.0\"\n";

/// A zip image carrying the manifest and, per entry in `files`, extra
/// in-image files (gemspecs, staged layouts). Parent directories get
/// explicit entries — the tfs zip backend's C++ parity requires a real
/// `dir/` entry to read_dir a non-root directory, and the real image
/// writers emit them.
fn zip_image(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let mut dirs = std::collections::BTreeSet::new();
    for (path, _) in files {
        for (i, c) in path.char_indices() {
            if c == '/' {
                dirs.insert(path[..=i].to_string());
            }
        }
    }
    for dir in dirs {
        writer.add_directory(dir, options).unwrap();
    }
    for (path, content) in files {
        writer.start_file(*path, options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn closure_text() -> String {
    let mut rows = String::from("# the feedstock closure — <gem>-<version> <sha256>\n");
    for (gem, sha) in [
        ("activesupport-8.1.4.1", "11"),
        ("addressable-2.10.0", "22"),
        ("nokogiri-1.18.9-arm64-darwin", "33"),
    ] {
        rows.push_str(&format!("{gem} {}\n", &sha.repeat(33)[..64]));
    }
    rows
}

/// A gemspec in the installed-specification rendering: rubygems freezes
/// each scalar and each licenses ARRAY element individually (the real
/// in-image shape). `licenses` is the element list the caller spells,
/// frozen elements and all (`"\"MIT\".freeze, \"Apache-2.0\".freeze"`).
fn gemspec(name: &str, version: &str, licenses: &str) -> String {
    format!(
        "Gem::Specification.new do |s|\n  s.name = \"{name}\".freeze\n  s.version = \"{version}\".freeze\n  s.licenses = [{licenses}]\nend\n"
    )
}

fn opts(target: &Path, closure: Option<&Path>, out: &Path) -> tebako_cli::sbom::SbomOptions {
    tebako_cli::sbom::SbomOptions {
        target: target.to_string_lossy().into_owned(),
        closure: closure.map(|p| p.to_path_buf()),
        ecosystem: None,
        triplet: None,
        out: Some(out.to_path_buf()),
        random_serial: false,
        runtime: false,
        runtime_exe: false,
        crates: None,
        name: None,
        version: None,
        engine: None,
        engine_version: None,
    }
}

/// A kind:runtime manifest (the env image's shape — spec 03 §2.2): one
/// provided engine entry plus the build's source pin and patch set.
fn runtime_manifest_yaml() -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: \"0.0.7\"\n  producer: {{tool: tebako-runtime-ruby, tool_version: \"0.0.7\"}}\n  created: \"2026-11-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  provides:\n    engine: ruby\n    version: \"4.0.9\"\n    abi_line: \"4.0\"\n    platform: aarch64-macos\n  built_from:\n    src_sha256: \"{}\"\n    patch_set: test-chain\n  capabilities: {{exec: true, read: true, runtime: true}}\n",
        "0".repeat(64),
        "0".repeat(64),
        "c".repeat(64)
    )
}

/// The exe mode's SbomOptions: target = the exe path, the capture +
/// version required, the rest defaults.
fn exe_opts(target: &Path, crates: &Path, out: &Path) -> tebako_cli::sbom::SbomOptions {
    tebako_cli::sbom::SbomOptions {
        target: target.to_string_lossy().into_owned(),
        closure: None,
        ecosystem: None,
        triplet: Some("macos-arm64".to_string()),
        out: Some(out.to_path_buf()),
        random_serial: false,
        runtime: false,
        runtime_exe: true,
        crates: Some(crates.to_path_buf()),
        name: None,
        version: Some("0.17.15".to_string()),
        engine: Some("ruby".to_string()),
        engine_version: Some("4.0.9".to_string()),
    }
}

#[test]
fn closure_mode_emits_the_complete_hashed_inventory() {
    let dir = scratch("closure");
    let image = dir.join("probe-1.0.0.tfs");
    let manifest = manifest_yaml("probe", "1.0.0", "app", REQUIRES_RUBY);
    fs::write(
        &image,
        zip_image(&[
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
            ("bin/probe", b"#!/bin/sh\n"),
        ]),
    )
    .unwrap();
    let closure = dir.join("closure.txt");
    fs::write(&closure, closure_text()).unwrap();
    let out = dir.join("out.cdx.json");

    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let result = tebako_cli::sbom::sbom(&opts(&image, Some(&closure), &out), &home).unwrap();
    assert!(result.complete);
    assert_eq!(result.components, 3);
    let json = fs::read_to_string(&out).unwrap();
    // Identity from the manifest; the engine maps gem by default; the
    // gem platform rides as a property, never in the purl.
    assert!(json.contains("\"purl\": \"pkg:generic/probe@1.0.0\""));
    assert!(json.contains("pkg:gem/activesupport@8.1.4.1"));
    assert!(json.contains("pkg:gem/addressable@2.10.0"));
    assert!(json.contains("pkg:gem/nokogiri@1.18.9"));
    assert!(json.contains("\"tebako:gem:platform\""));
    assert!(json.contains("\"tebako:payload:engine\""));
    assert!(json.contains("\"tebako:payload:signing\""));
    assert!(json.contains("\"tebako:closure:complete\",\n        \"value\": \"true\""));
    assert!(json.contains("\"tebako:sbom:inventory-source\",\n        \"value\": \"closure\""));
    // Every closure row's hash lands as the component's SHA-256.
    assert!(json.contains(&"2".repeat(64)));
    // No annotation in complete mode.
    assert!(!json.contains("\"annotations\""));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn closure_mode_is_deterministic_byte_for_byte_under_source_date_epoch() {
    let dir = scratch("determinism");
    let image = dir.join("probe-1.0.0.tfs");
    let manifest = manifest_yaml("probe", "1.0.0", "app", REQUIRES_RUBY);
    fs::write(
        &image,
        zip_image(&[("__tpkg__/manifest.yaml", manifest.as_bytes())]),
    )
    .unwrap();
    let closure = dir.join("closure.txt");
    fs::write(&closure, closure_text()).unwrap();
    let out1 = dir.join("one.cdx.json");
    let out2 = dir.join("two.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    // The operator's reproducibility pin (the press's convention): with
    // it set, two runs on unchanged inputs are byte-identical.
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("SOURCE_DATE_EPOCH", "1700000000");
    tebako_cli::sbom::sbom(&opts(&image, Some(&closure), &out1), &home).unwrap();
    tebako_cli::sbom::sbom(&opts(&image, Some(&closure), &out2), &home).unwrap();
    std::env::remove_var("SOURCE_DATE_EPOCH");
    let first = fs::read(&out1).unwrap();
    let second = fs::read(&out2).unwrap();
    assert_eq!(first, second);
    let text = String::from_utf8(first).unwrap();
    assert!(text.contains("\"timestamp\": \"2023-11-14T22:13:20Z\""));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn degraded_mode_says_so_and_carries_gemspec_licenses() {
    let dir = scratch("degraded");
    let image = dir.join("probe-1.0.0.tfs");
    let manifest = manifest_yaml("probe", "1.0.0", "app", REQUIRES_RUBY);
    let rack = gemspec("rack", "3.1.0", "\"MIT\".freeze");
    let rake = gemspec("rake", "13.2.1", "\"MIT\".freeze, \"Apache-2.0\".freeze");
    let broken = "this is not a gemspec at all";
    fs::write(
        &image,
        zip_image(&[
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
            ("specifications/rack-3.1.0.gemspec", rack.as_bytes()),
            ("specifications/rake-13.2.1.gemspec", rake.as_bytes()),
            ("specifications/broken-9.gemspec", broken.as_bytes()),
        ]),
    )
    .unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let result = tebako_cli::sbom::sbom(&opts(&image, None, &out), &home).unwrap();
    assert!(!result.complete);
    assert_eq!(result.components, 2);
    let json = fs::read_to_string(&out).unwrap();
    // Completeness is NOT claimed; the annotation explains in words,
    // including the skipped spec's name.
    assert!(json.contains("\"tebako:closure:complete\",\n        \"value\": \"false\""));
    assert!(json.contains("\"annotations\""));
    assert!(json.contains("broken-9.gemspec"));
    // The gemspecs' licenses land as CycloneDX license expressions.
    assert!(json.contains("\"expression\": \"MIT\""));
    assert!(json.contains("\"expression\": \"MIT AND Apache-2.0\""));
    // No component hashes anywhere in degraded mode: the document's one
    // SHA-256 block is the SUBJECT's image hash (metadata.component) —
    // counting it beats string-section surgery, which metadata.tools'
    // own components array defeats.
    assert_eq!(json.matches("\"alg\": \"SHA-256\"").count(), 1);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn degraded_mode_finds_the_staged_runtime_layout() {
    let dir = scratch("staged-layout");
    let image = dir.join("probe-1.0.0.tfs");
    let manifest = manifest_yaml("probe", "1.0.0", "app", REQUIRES_RUBY);
    let rack = gemspec("rack", "3.1.0", "\"MIT\".freeze");
    fs::write(
        &image,
        zip_image(&[
            ("__tpkg__/manifest.yaml", manifest.as_bytes()),
            (
                "lib/ruby/gems/4.0.0/specifications/rack-3.1.0.gemspec",
                rack.as_bytes(),
            ),
        ]),
    )
    .unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let result = tebako_cli::sbom::sbom(&opts(&image, None, &out), &home).unwrap();
    assert_eq!(result.components, 1);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn degraded_mode_refuses_an_image_without_gemspecs_named() {
    let dir = scratch("no-gemspecs");
    let image = dir.join("probe-1.0.0.tfs");
    let manifest = manifest_yaml("probe", "1.0.0", "app", REQUIRES_RUBY);
    fs::write(
        &image,
        zip_image(&[("__tpkg__/manifest.yaml", manifest.as_bytes())]),
    )
    .unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let error = tebako_cli::sbom::sbom(&opts(&image, None, &out), &home).unwrap_err();
    assert_eq!(error.code, 65);
    assert!(error.message.contains("--closure"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filename_fallback_names_a_manifestless_image_loudly() {
    let dir = scratch("fallback");
    let image = dir.join("probe-1.2.3-macos-arm64.tfs");
    fs::write(&image, zip_image(&[("bin/probe", b"#!/bin/sh\n")])).unwrap();
    let closure = dir.join("closure.txt");
    fs::write(&closure, closure_text()).unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let result = tebako_cli::sbom::sbom(&opts(&image, Some(&closure), &out), &home).unwrap();
    assert_eq!(result.components, 3);
    let json = fs::read_to_string(&out).unwrap();
    // The identity comes from the filename stem; kind unknown.
    assert!(json.contains("\"purl\": \"pkg:generic/probe@1.2.3\""));
    assert!(json.contains("\"tebako:payload:kind\",\n          \"value\": \"unknown\""));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filename_fallback_refuses_a_versionless_stem_named() {
    let dir = scratch("fallback-refuse");
    let image = dir.join("probe.tfs");
    fs::write(&image, zip_image(&[("bin/probe", b"#!/bin/sh\n")])).unwrap();
    let closure = dir.join("closure.txt");
    fs::write(&closure, closure_text()).unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let error = tebako_cli::sbom::sbom(&opts(&image, Some(&closure), &out), &home).unwrap_err();
    assert_eq!(error.code, 65);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn store_resolve_carries_origin_and_newest_version() {
    let dir = scratch("store");
    let home = dir.join("home");
    let payload_dir = home.join("payloads/probe");
    fs::create_dir_all(&payload_dir).unwrap();
    let rack = gemspec("rack", "3.1.0", "\"MIT\".freeze");
    for version in ["1.0.0", "2.0.0"] {
        // The store entry's manifest carries ITS OWN version — the
        // in-image manifest is the doc's authoritative identity, so the
        // resolved newest version shows through it.
        let manifest = manifest_yaml("probe", version, "app", REQUIRES_RUBY);
        let image = payload_dir.join(format!("{version}.tfs"));
        fs::write(
            &image,
            zip_image(&[
                ("__tpkg__/manifest.yaml", manifest.as_bytes()),
                ("specifications/rack-3.1.0.gemspec", rack.as_bytes()),
            ]),
        )
        .unwrap();
        fs::write(
            payload_dir.join(format!("{version}.tfs.sha256")),
            format!(
                "{}  {version}.tfs\n",
                tebako_resolve::sha256_hex(&fs::read(&image).unwrap())
            ),
        )
        .unwrap();
        if version == "2.0.0" {
            fs::write(
                payload_dir.join(format!("{version}.tfs.origin")),
                "https://example.com/probe-2.0.0.tfs\n",
            )
            .unwrap();
            fs::write(
                payload_dir.join(format!("{version}.tfs.registry")),
                "tfs:github:example/probe\n",
            )
            .unwrap();
        }
    }
    // The bare name resolves the newest installed; the origin and
    // registry markers land as evidence.
    let out = dir.join("out.cdx.json");
    let target_opts = tebako_cli::sbom::SbomOptions {
        target: "probe".to_string(),
        closure: None,
        ecosystem: None,
        triplet: None,
        out: Some(out.clone()),
        random_serial: false,
        runtime: false,
        runtime_exe: false,
        crates: None,
        name: None,
        version: None,
        engine: None,
        engine_version: None,
    };
    let result = tebako_cli::sbom::sbom(&target_opts, &home).unwrap();
    assert_eq!(result.components, 1);
    let json = fs::read_to_string(&out).unwrap();
    assert!(json.contains("\"version\": \"2.0.0\""));
    assert!(json.contains("\"type\": \"distribution\""));
    assert!(json.contains("https://example.com/probe-2.0.0.tfs"));
    assert!(json.contains("tfs:github:example/probe"));
    // An explicit version resolves exactly; a missing name is the named
    // error, never a guess.
    let out2 = dir.join("out2.cdx.json");
    let exact = tebako_cli::sbom::SbomOptions {
        target: "probe@1.0.0".to_string(),
        closure: None,
        ecosystem: None,
        triplet: None,
        out: Some(out2),
        random_serial: false,
        runtime: false,
        runtime_exe: false,
        crates: None,
        name: None,
        version: None,
        engine: None,
        engine_version: None,
    };
    tebako_cli::sbom::sbom(&exact, &home).unwrap();
    let absent = tebako_cli::sbom::SbomOptions {
        target: "absent".to_string(),
        closure: None,
        ecosystem: None,
        triplet: None,
        out: Some(dir.join("out3.cdx.json")),
        random_serial: false,
        runtime: false,
        runtime_exe: false,
        crates: None,
        name: None,
        version: None,
        engine: None,
        engine_version: None,
    };
    let error = tebako_cli::sbom::sbom(&absent, &home).unwrap_err();
    assert_eq!(error.code, 65);
    fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------
// The runtime-artifact modes (spec 15 §8a, tebako#1121)
// ---------------------------------------------------------------------

#[test]
fn runtime_mode_emits_the_interpreter_build_and_both_gem_classes() {
    let dir = scratch("runtime");
    let image = dir.join("tebako-runtime-0.0.7-ruby-4.0.9-macos-arm64.tfs");
    let rake = gemspec("rake", "13.3.1", "\"MIT\".freeze");
    let digest = gemspec("digest", "3.2.1", "\"Ruby\".freeze");
    fs::write(
        &image,
        zip_image(&[
            ("__tpkg__/manifest.yaml", runtime_manifest_yaml().as_bytes()),
            (
                "lib/ruby/gems/4.0.0/specifications/rake-13.3.1.gemspec",
                rake.as_bytes(),
            ),
            (
                "lib/ruby/gems/4.0.0/specifications/default/digest-3.2.1.gemspec",
                digest.as_bytes(),
            ),
            ("lib/ruby/4.0.0/bundled_gems.rb", b"# stdlib\n"),
        ]),
    )
    .unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let mut runtime_opts = opts(&image, None, &out);
    runtime_opts.runtime = true;
    let result = tebako_cli::sbom::sbom(&runtime_opts, &home).unwrap();
    // Never complete in the closure sense; three components: the
    // interpreter build + one bundled gem + one default gem.
    assert!(!result.complete);
    assert_eq!(result.components, 3);
    let json = fs::read_to_string(&out).unwrap();
    // The subject is the image (kind runtime → platform), manifest-named.
    assert!(json.contains("\"purl\": \"pkg:generic/tebako-runtime-ruby@0.0.7\""));
    assert!(json.contains("\"tebako:payload:kind\",\n          \"value\": \"runtime\""));
    assert!(json.contains("\"tebako:triplet\",\n          \"value\": \"aarch64-macos\""));
    // The interpreter build component: the declared engine with its
    // build facts — the source pin and patch set are properties (never a
    // binary hash: the interpreter is statically linked into the exe).
    assert!(json.contains("\"purl\": \"pkg:generic/ruby@4.0.9\""));
    assert!(json.contains("\"tebako:runtime:abi_line\",\n          \"value\": \"4.0\""));
    assert!(json.contains("\"tebako:runtime:platform\",\n          \"value\": \"aarch64-macos\""));
    assert!(json.contains(&format!(
        "\"tebako:runtime:src_sha256\",\n          \"value\": \"{}\"",
        "c".repeat(64)
    )));
    assert!(json.contains("\"tebako:runtime:patch_set\",\n          \"value\": \"test-chain\""));
    // Both gem classes land, with the class evidence as a property —
    // never a purl facet.
    assert!(json.contains("\"purl\": \"pkg:gem/rake@13.3.1\""));
    assert!(json.contains("\"purl\": \"pkg:gem/digest@3.2.1\""));
    assert!(json.contains("\"tebako:gem:default\",\n          \"value\": \"false\""));
    assert!(json.contains("\"tebako:gem:default\",\n          \"value\": \"true\""));
    // The metadata frame: the runtime inventory source + the closure
    // claim honestly false, and the annotation names what the inventory
    // IS (counts, the sibling exe document).
    assert!(
        json.contains("\"tebako:sbom:inventory-source\",\n        \"value\": \"runtime-image\"")
    );
    assert!(json.contains("\"tebako:closure:complete\",\n        \"value\": \"false\""));
    assert!(json.contains("\"annotations\""));
    assert!(json.contains("1 bundled + 1 default"));
    assert!(json.contains("--runtime-exe"));
    // No component hashes: the subject's image hash is the only SHA-256.
    assert_eq!(json.matches("\"alg\": \"SHA-256\"").count(), 1);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn runtime_mode_refuses_non_runtime_images_named() {
    let dir = scratch("runtime-refuse");
    // An app manifest through the runtime gate is the named refusal.
    let image = dir.join("probe-1.0.0.tfs");
    fs::write(
        &image,
        zip_image(&[(
            "__tpkg__/manifest.yaml",
            manifest_yaml("probe", "1.0.0", "app", REQUIRES_RUBY).as_bytes(),
        )]),
    )
    .unwrap();
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let mut runtime_opts = opts(&image, None, &dir.join("out.cdx.json"));
    runtime_opts.runtime = true;
    let error = tebako_cli::sbom::sbom(&runtime_opts, &home).unwrap_err();
    assert_eq!(error.code, 65);
    assert!(error.message.contains("kind: app"));
    // A manifestless image likewise: no interpreter declaration, no
    // runtime document.
    let plain = dir.join("plain-1.0.0.tfs");
    fs::write(&plain, zip_image(&[("bin/x", b"x\n")])).unwrap();
    let mut plain_opts = opts(&plain, None, &dir.join("out2.cdx.json"));
    plain_opts.runtime = true;
    let error = tebako_cli::sbom::sbom(&plain_opts, &home).unwrap_err();
    assert_eq!(error.code, 65);
    assert!(error.message.contains("manifest"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn payload_lane_on_a_runtime_image_stays_on_its_own_inventory() {
    let dir = scratch("runtime-payload-lane");
    let image = dir.join("tebako-runtime-0.0.7-ruby-4.0.9-macos-arm64.tfs");
    let rake = gemspec("rake", "13.3.1", "\"MIT\".freeze");
    let digest = gemspec("digest", "3.2.1", "\"Ruby\".freeze");
    fs::write(
        &image,
        zip_image(&[
            ("__tpkg__/manifest.yaml", runtime_manifest_yaml().as_bytes()),
            (
                "lib/ruby/gems/4.0.0/specifications/rake-13.3.1.gemspec",
                rake.as_bytes(),
            ),
            (
                "lib/ruby/gems/4.0.0/specifications/default/digest-3.2.1.gemspec",
                digest.as_bytes(),
            ),
        ]),
    )
    .unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    // The payload lane (no --runtime) keeps its own degraded-inventory
    // discipline: the bundled specifications dir only, never the
    // default-gem class — the runtime mode is the lane that widens.
    let result = tebako_cli::sbom::sbom(&opts(&image, None, &out), &home).unwrap();
    assert!(!result.complete);
    assert_eq!(result.components, 1);
    let json = fs::read_to_string(&out).unwrap();
    assert!(json.contains("pkg:gem/rake@13.3.1"));
    assert!(!json.contains("pkg:gem/digest@3.2.1"));
    fs::remove_dir_all(&dir).unwrap();
}

/// The exe mode's capture fixture (schema 1, hand-spelled — the wire
/// grammar itself is under test, not the emitter).
fn capture_fixture() -> String {
    format!(
        "{{\n  \"capture_schema\": 1,\n  \"tool\": \"sbom-cargo\",\n  \"tool_version\": \"2.8.85\",\n  \"package\": \"tebako-driver\",\n  \"target\": \"aarch64-testos\",\n  \"root\": \"pkg:cargo/tebako-driver@2.8.85\",\n  \"components\": [\n    {{\"name\": \"tebako-driver\", \"version\": \"2.8.85\", \"purl\": \"pkg:cargo/tebako-driver@2.8.85\", \"sha256\": null, \"license\": \"BSD-3-Clause\", \"source\": \"workspace\"}},\n    {{\"name\": \"dep\", \"version\": \"1.0.0\", \"purl\": \"pkg:cargo/dep@1.0.0\", \"sha256\": \"{}\", \"license\": \"MIT\", \"source\": \"registry\"}}\n  ],\n  \"edges\": [\n    {{\"ref\": \"pkg:cargo/tebako-driver@2.8.85\", \"depends_on\": [\"pkg:cargo/dep@1.0.0\"]}},\n    {{\"ref\": \"pkg:cargo/dep@1.0.0\", \"depends_on\": []}}\n  ]\n}}\n",
        "a".repeat(64)
    )
}

#[test]
fn exe_mode_resubjects_the_capture_to_the_exes_bytes() {
    let dir = scratch("exe");
    let exe = dir.join("tebako-runtime-0.17.15-ruby-4.0.9-macos-arm64");
    fs::write(&exe, b"fake exe bytes\n").unwrap();
    let crates = dir.join("crate-inventory.json");
    fs::write(&crates, capture_fixture()).unwrap();
    let out = dir.join("out.cdx.json");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let result = tebako_cli::sbom::sbom(&exe_opts(&exe, &crates, &out), &home).unwrap();
    assert!(result.complete);
    assert_eq!(result.components, 2);
    let json = fs::read_to_string(&out).unwrap();
    // The subject: name defaults to the exe's stem, the version is the
    // explicit flag, and the digest is the exe's own bytes.
    assert!(json.contains(
        "\"purl\": \"pkg:generic/tebako-runtime-0.17.15-ruby-4.0.9-macos-arm64@0.17.15\""
    ));
    let exe_sha = tebako_resolve::sha256_hex(b"fake exe bytes\n");
    assert!(json.contains(&exe_sha));
    assert!(json.contains("\"tebako:payload:kind\",\n          \"value\": \"runtime\""));
    assert!(json.contains("\"tebako:platform\",\n          \"value\": \"macos-arm64\""));
    assert!(json.contains("\"tebako:target\",\n          \"value\": \"aarch64-testos\""));
    assert!(json.contains("\"tebako:runtime:engine\",\n          \"value\": \"ruby\""));
    assert!(json.contains("\"tebako:runtime:engine_version\",\n          \"value\": \"4.0.9\""));
    // The crates: the workspace root names its source, the registry dep
    // carries its lock checksum and license.
    assert!(json.contains("\"purl\": \"pkg:cargo/tebako-driver@2.8.85\""));
    assert!(json.contains("\"tebako:crate:source\",\n          \"value\": \"workspace\""));
    assert!(json.contains(&"a".repeat(64)));
    assert!(json.contains("\"expression\": \"MIT\""));
    // The capture's provenance and completeness frame.
    assert!(json.contains(
        "\"tebako:sbom:inventory-source\",\n        \"value\": \"cargo-resolution-capture\""
    ));
    assert!(json.contains("\"tebako:closure:complete\",\n        \"value\": \"true\""));
    assert!(json.contains("\"tebako:capture:tool\",\n        \"value\": \"sbom-cargo\""));
    // The graph: the subject roots at the capture's root crate, and the
    // capture's own edges survive.
    let deps = json.split("\"dependencies\"").nth(1).unwrap();
    let root = deps
        .split("\"ref\": \"pkg:generic/tebako-runtime-0.17.15-ruby-4.0.9-macos-arm64@0.17.15\"")
        .nth(1)
        .unwrap();
    assert!(root.contains("pkg:cargo/tebako-driver@2.8.85"));
    let driver = deps
        .split("\"ref\": \"pkg:cargo/tebako-driver@2.8.85\"")
        .nth(1)
        .unwrap();
    assert!(driver.contains("pkg:cargo/dep@1.0.0"));
    // Complete by construction — no annotation.
    assert!(!json.contains("\"annotations\""));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn exe_mode_refuses_a_wrong_schema_capture_named() {
    let dir = scratch("exe-schema");
    let exe = dir.join("tebako-runtime-0.17.15-ruby-4.0.9-macos-arm64");
    fs::write(&exe, b"fake exe bytes\n").unwrap();
    let crates = dir.join("crate-inventory.json");
    fs::write(
        &crates,
        capture_fixture().replace("\"capture_schema\": 1", "\"capture_schema\": 2"),
    )
    .unwrap();
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let error = tebako_cli::sbom::sbom(&exe_opts(&exe, &crates, &dir.join("out.cdx.json")), &home)
        .unwrap_err();
    assert_eq!(error.code, 65);
    assert!(error.message.contains("capture_schema"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn exe_and_runtime_modes_are_deterministic_byte_for_byte_under_source_date_epoch() {
    let dir = scratch("runtime-determinism");
    let image = dir.join("tebako-runtime-0.0.7-ruby-4.0.9-macos-arm64.tfs");
    let rake = gemspec("rake", "13.3.1", "\"MIT\".freeze");
    fs::write(
        &image,
        zip_image(&[
            ("__tpkg__/manifest.yaml", runtime_manifest_yaml().as_bytes()),
            (
                "lib/ruby/gems/4.0.0/specifications/rake-13.3.1.gemspec",
                rake.as_bytes(),
            ),
        ]),
    )
    .unwrap();
    let exe = dir.join("tebako-runtime-0.17.15-ruby-4.0.9-macos-arm64");
    fs::write(&exe, b"fake exe bytes\n").unwrap();
    let crates = dir.join("crate-inventory.json");
    fs::write(&crates, capture_fixture()).unwrap();
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("SOURCE_DATE_EPOCH", "1700000000");
    // The runtime env image document: two runs, one pinned epoch,
    // byte-identical.
    let r1 = dir.join("r1.cdx.json");
    let r2 = dir.join("r2.cdx.json");
    let mut runtime_opts1 = opts(&image, None, &r1);
    runtime_opts1.runtime = true;
    let mut runtime_opts2 = opts(&image, None, &r2);
    runtime_opts2.runtime = true;
    tebako_cli::sbom::sbom(&runtime_opts1, &home).unwrap();
    tebako_cli::sbom::sbom(&runtime_opts2, &home).unwrap();
    assert_eq!(fs::read(&r1).unwrap(), fs::read(&r2).unwrap());
    assert!(fs::read_to_string(&r1)
        .unwrap()
        .contains("\"timestamp\": \"2023-11-14T22:13:20Z\""));
    // The exe document: same law.
    let e1 = dir.join("e1.cdx.json");
    let e2 = dir.join("e2.cdx.json");
    tebako_cli::sbom::sbom(&exe_opts(&exe, &crates, &e1), &home).unwrap();
    tebako_cli::sbom::sbom(&exe_opts(&exe, &crates, &e2), &home).unwrap();
    assert_eq!(fs::read(&e1).unwrap(), fs::read(&e2).unwrap());
    assert!(fs::read_to_string(&e1)
        .unwrap()
        .contains("\"timestamp\": \"2023-11-14T22:13:20Z\""));
    std::env::remove_var("SOURCE_DATE_EPOCH");
    fs::remove_dir_all(&dir).unwrap();
}
