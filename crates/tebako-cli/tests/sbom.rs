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
    };
    tebako_cli::sbom::sbom(&exact, &home).unwrap();
    let absent = tebako_cli::sbom::SbomOptions {
        target: "absent".to_string(),
        closure: None,
        ecosystem: None,
        triplet: None,
        out: Some(dir.join("out3.cdx.json")),
        random_serial: false,
    };
    let error = tebako_cli::sbom::sbom(&absent, &home).unwrap_err();
    assert_eq!(error.code, 65);
    fs::remove_dir_all(&dir).unwrap();
}
