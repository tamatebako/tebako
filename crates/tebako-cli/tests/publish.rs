//! Publish tests (spec 16 §5, roadmap 41): accept per-triplet payloads →
//! optional sign → upload (file:// mirrors only — no network) → registry
//! upsert → tap render → the built-in clean-cache install proof.
//! Idempotent re-publish is part of the matrix.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use tebako_cli::publish::{self, PayloadInput, PublishOptions};
use tpkg::Platform;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-publish-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Fixture {
    dir: PathBuf,
    home: PathBuf,
    work: PathBuf,
    shim_binary: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let dir = scratch(tag);
        let home = dir.join("home");
        let work = dir.join("work");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&work).unwrap();
        let shim_binary = dir.join("tebako-shim");
        fs::write(&shim_binary, b"#!/bin/sh\n").unwrap();
        Fixture {
            dir,
            home,
            work,
            shim_binary,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn sha(c: u8) -> String {
    (0..64)
        .map(|i| b"0123456789abcdef"[((c + i as u8) % 16) as usize] as char)
        .collect()
}

/// The embedded app manifest (spec 03 §1) for the fixture images.
fn app_manifest_yaml(name: &str, version: &str, entrypoints: &[&str]) -> String {
    let entries: String = entrypoints
        .iter()
        .map(|e| {
            format!(
                "    - name: {e}\n      path: /app/bin/{e}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n"
            )
        })
        .collect();
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n{entries}  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    )
}

/// A ZIP image carrying the embedded manifest (the tfs ZIP backend reads
/// it — same fixture discipline as the install tests).
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

fn write_payload(fx: &Fixture, file: &str, manifest_yaml: &str) -> PathBuf {
    let path = fx.work.join(file);
    fs::write(&path, zip_image(manifest_yaml)).unwrap();
    path
}

fn base_opts(fx: &Fixture, name: &str) -> PublishOptions {
    PublishOptions {
        name: name.to_string(),
        version: None,
        release: "tfs:github:acme/app:1.0".to_string(),
        payloads: Vec::new(),
        standalones: Vec::new(),
        sign: None,
        upload_mirror: Some(fx.work.join("mirror")),
        tap: None,
        tap_dir: None,
        license: None,
        desc: None,
        homepage: None,
        registry_out: Some(fx.work.join("tpkg-registry.yaml").display().to_string()),
        skip_verify: false,
        oci: None,
    }
}

fn registry_at(fx: &Fixture) -> tebako_resolve::Registry {
    let text = fs::read_to_string(fx.work.join("tpkg-registry.yaml")).unwrap();
    tebako_resolve::Registry::from_yaml(&text).unwrap()
}

#[test]
fn universal_signed_publish_end_to_end() {
    let fx = Fixture::new("universal");
    let payload = write_payload(
        &fx,
        "app-1.0.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    opts.sign = Some(None); // the press-local key

    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    assert_eq!(outcome.version, "1.0");
    assert_eq!(outcome.tag, "1.0");
    assert_eq!(outcome.artifacts.len(), 1);
    assert_eq!(outcome.artifacts[0].0, "app-1.0.tfs");
    assert_eq!(outcome.ascs, vec!["app-1.0.tfs.asc".to_string()]);
    let keyid = outcome.signer.clone().unwrap();
    assert_eq!(keyid.len(), 16);

    // the mirror holds the release layout, idempotent file content
    let mirror = fx.work.join("mirror/1.0");
    assert_eq!(
        fs::read(mirror.join("app-1.0.tfs")).unwrap(),
        fs::read(fx.work.join("app-1.0.tfs")).unwrap()
    );
    assert!(mirror.join("app-1.0.tfs.asc").is_file());

    // the per-artifact .sha256 sidecar (the pin verification pass's
    // read-back source): one "<sha256>  <name>" line naming the artifact
    let image_sha = tebako_resolve::sha256_hex(&fs::read(fx.work.join("app-1.0.tfs")).unwrap());
    assert_eq!(
        fs::read_to_string(mirror.join("app-1.0.tfs.sha256")).unwrap(),
        format!("{image_sha}  app-1.0.tfs\n")
    );
    assert!(
        outcome
            .notes
            .iter()
            .any(|n| n.contains("pin verification: 1 row(s)")),
        "the pin pass reports its rows: {:?}",
        outcome.notes
    );

    // the blksum sidecar (spec 39 §3): staged beside the image in the
    // same invocation, unsigned (its anchor is the registry row's pin —
    // the ascs above carry the image's signature alone), and its
    // digests verify against the image bytes
    let sidecar = fs::read(mirror.join("app-1.0.tfs.blksum.json")).unwrap();
    let sum = tpkg::lazy::Blksum::parse(std::str::from_utf8(&sidecar).unwrap()).unwrap();
    let image_bytes = fs::read(fx.work.join("app-1.0.tfs")).unwrap();
    assert_eq!(sum.size_bytes, image_bytes.len() as u64);
    assert_eq!(sum.sha256, tebako_resolve::sha256_hex(&image_bytes));
    assert_eq!(sum.group_count(), 1, "the fixture image is one group");
    assert_eq!(sum.groups[0], sum.sha256);
    let sidecar_sha = tebako_resolve::sha256_hex(&sidecar);
    assert_eq!(
        outcome.blksums,
        vec![("app-1.0.tfs.blksum.json".to_string(), sidecar_sha.clone())]
    );

    // the registry records the entry (universal, signature pin, the
    // github release ref — mirror mode does not leak into the ref)
    let registry = registry_at(&fx);
    let app = registry.payload("app").unwrap();
    assert_eq!(app.default.as_deref(), Some("1.0"));
    let v = app.version("1.0").unwrap();
    assert!(matches!(
        v.platforms,
        Some(tebako_resolve::RegistryPlatforms::Universal)
    ));
    // the universal row's version-level pin anchors the sidecar's bytes
    let pin = v.blksum.clone().unwrap();
    assert_eq!(pin.filename, "app-1.0.tfs.blksum.json");
    assert_eq!(pin.sha256, sidecar_sha);
    assert_eq!(v.release.r#ref, "tfs:github:acme/app:1.0");
    let sig = v.signature.clone().unwrap();
    assert_eq!(sig.keyid, keyid);
    assert_eq!(sig.asc.as_deref(), Some("app-1.0.tfs.asc"));
    assert_eq!(v.entrypoints, vec!["app"]);
    assert!(v.runtime_requirement.is_some());

    // the built-in verify ran (clean-cache install proof, signed leg)
    let verified = outcome.verified.unwrap();
    assert!(
        verified.contains("verified: clean-cache install of app 1.0"),
        "{verified}"
    );
    assert!(
        verified.contains(&format!("signed by {keyid}")),
        "{verified}"
    );
}

#[test]
fn per_triplet_publish_and_idempotent_republish() {
    let fx = Fixture::new("triplet");
    let mac = write_payload(
        &fx,
        "app-1.0-macos-arm64.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let linux = write_payload(
        &fx,
        "app-1.0-linux-gnu-x86_64.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads = vec![
        PayloadInput {
            triplet: Some(Platform::Aarch64Macos),
            path: mac,
        },
        PayloadInput {
            triplet: Some(Platform::X86_64LinuxGnu),
            path: linux,
        },
    ];

    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    assert!(outcome.signer.is_none());
    let registry = registry_at(&fx);
    let app = registry.payload("app").unwrap();
    let v = app.version("1.0").unwrap();
    let Some(tebako_resolve::RegistryPlatforms::PerTriplet(map)) = &v.platforms else {
        panic!("per-triplet platforms");
    };
    assert_eq!(map.len(), 2);
    assert_eq!(
        map[&Platform::Aarch64Macos].artifact,
        "app-1.0-macos-arm64.tfs"
    );
    assert_eq!(
        map[&Platform::X86_64LinuxGnu].sha256.len(),
        64,
        "sha pinned per triplet"
    );

    // every per-triplet row pins its image's blksum sidecar in the
    // platforms map entry (spec 39 §3); the version-level spelling
    // stays empty (one location per form), and each sidecar's bytes
    // land in the mirror under the pinned digest
    assert!(v.blksum.is_none());
    let mirror = fx.work.join("mirror/1.0");
    assert_eq!(outcome.blksums.len(), 2);
    for platform in [Platform::Aarch64Macos, Platform::X86_64LinuxGnu] {
        let entry = &map[&platform];
        let pin = entry.blksum.as_ref().unwrap();
        assert_eq!(pin.filename, format!("{}.blksum.json", entry.artifact));
        let sidecar = fs::read(mirror.join(&pin.filename)).unwrap();
        assert_eq!(pin.sha256, tebako_resolve::sha256_hex(&sidecar));
        let sum = tpkg::lazy::Blksum::parse(std::str::from_utf8(&sidecar).unwrap()).unwrap();
        assert_eq!(sum.sha256, entry.sha256);
        assert!(outcome
            .blksums
            .contains(&(pin.filename.clone(), pin.sha256.clone())));
        // the per-artifact .sha256 sidecar names the row's pin
        assert_eq!(
            fs::read_to_string(mirror.join(format!("{}.sha256", entry.artifact))).unwrap(),
            format!("{}  {}\n", entry.sha256, entry.artifact)
        );
    }
    assert!(
        outcome
            .notes
            .iter()
            .any(|n| n.contains("pin verification: 2 row(s)")),
        "the pin pass reports its rows: {:?}",
        outcome.notes
    );

    // re-publish: idempotent — one version entry, a "replaced" note
    let outcome2 = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    assert!(outcome2.notes.iter().any(|n| n.contains("replaced")));
    let registry = registry_at(&fx);
    assert_eq!(registry.payload("app").unwrap().versions.len(), 1);

    // a second version appends; the default stays put
    let payload11 = write_payload(
        &fx,
        "app-1.1-macos-arm64.tfs",
        &app_manifest_yaml("app", "1.1", &["app"]),
    );
    let mut opts11 = base_opts(&fx, "app");
    opts11.release = "tfs:github:acme/app:1.1".to_string();
    opts11.payloads = vec![PayloadInput {
        triplet: Some(Platform::Aarch64Macos),
        path: payload11,
    }];
    publish::publish_full(&opts11, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let registry = registry_at(&fx);
    let app = registry.payload("app").unwrap();
    assert_eq!(app.versions.len(), 2);
    assert_eq!(app.default.as_deref(), Some("1.0"));
}

#[test]
fn pin_verification_runs_even_under_skip_verify() {
    // --skip-verify skips only the clean-cache install proof; the pin
    // verification pass (cheap sidecar reads) runs on every publish.
    let fx = Fixture::new("skippin");
    let payload = write_payload(
        &fx,
        "app-2.0.tfs",
        &app_manifest_yaml("app", "2.0", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.release = "tfs:github:acme/app:2.0".to_string();
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    opts.skip_verify = true;

    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    assert!(
        outcome.verified.is_none(),
        "the install proof stayed skipped"
    );
    assert!(
        outcome
            .notes
            .iter()
            .any(|n| n.contains("pin verification: 1 row(s)")),
        "the pin pass ran anyway: {:?}",
        outcome.notes
    );
    let mirror = fx.work.join("mirror/2.0");
    let sha = tebako_resolve::sha256_hex(&fs::read(fx.work.join("app-2.0.tfs")).unwrap());
    assert_eq!(
        fs::read_to_string(mirror.join("app-2.0.tfs.sha256")).unwrap(),
        format!("{sha}  app-2.0.tfs\n")
    );
}

/// An app manifest whose entrypoint carries a native-extension runtime
/// requirement (engine + constraint + implementation + abi — an abi in
/// force requires the implementation axis).
fn app_manifest_native(name: &str, version: &str, constraint: &str, abi: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /app/bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \"{constraint}\", implementation: mri, abi: \"{abi}\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    )
}

#[test]
fn multi_platform_entry_omits_abi_from_the_registry_mirror() {
    // tebako#440: the abi is per-triplet by construction — the mirror of
    // a multi-platform per-triplet entry keeps engine/constraint/
    // implementation (identical on every leg) and omits abi (each
    // slice's embedded manifest owns the authoritative value).
    let fx = Fixture::new("abimulti");
    let mac = write_payload(
        &fx,
        "app-1.0-macos-arm64.tfs",
        &app_manifest_native("app", "1.0", "~> 3.3.0", "arm64-darwin-23"),
    );
    let linux = write_payload(
        &fx,
        "app-1.0-linux-gnu-x86_64.tfs",
        &app_manifest_native("app", "1.0", "~> 3.3.0", "x86_64-linux-gnu"),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads = vec![
        PayloadInput {
            triplet: Some(Platform::Aarch64Macos),
            path: mac,
        },
        PayloadInput {
            triplet: Some(Platform::X86_64LinuxGnu),
            path: linux,
        },
    ];

    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let registry = registry_at(&fx);
    let req = registry
        .payload("app")
        .unwrap()
        .version("1.0")
        .unwrap()
        .runtime_requirement
        .clone()
        .unwrap();
    assert_eq!(req.engine, "ruby");
    assert_eq!(req.constraint, "~> 3.3.0");
    assert_eq!(req.implementation.as_deref(), Some("mri"));
    assert_eq!(req.abi, None, "the multi-platform mirror omits abi");
    assert!(
        outcome.notes.iter().any(|n| n.contains("omits abi")),
        "{:?}",
        outcome.notes
    );
}

#[test]
fn single_platform_and_universal_entries_keep_abi_in_the_mirror() {
    // one value actually holds on a single-platform per-triplet row
    let fx = Fixture::new("abisingle");
    let mac = write_payload(
        &fx,
        "app-1.0-macos-arm64.tfs",
        &app_manifest_native("app", "1.0", "~> 3.3.0", "arm64-darwin-23"),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads = vec![PayloadInput {
        triplet: Some(Platform::Aarch64Macos),
        path: mac,
    }];
    publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let req = registry_at(&fx)
        .payload("app")
        .unwrap()
        .version("1.0")
        .unwrap()
        .runtime_requirement
        .clone()
        .unwrap();
    assert_eq!(req.abi.as_deref(), Some("arm64-darwin-23"));

    // …and on a universal row
    let fx = Fixture::new("abiuniversal");
    let payload = write_payload(
        &fx,
        "app-2.0.tfs",
        &app_manifest_native("app", "2.0", "~> 3.3.0", "universal-darwin"),
    );
    let mut opts = base_opts(&fx, "app");
    opts.release = "tfs:github:acme/app:2.0".to_string();
    opts.payloads = vec![PayloadInput {
        triplet: None,
        path: payload,
    }];
    publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let req = registry_at(&fx)
        .payload("app")
        .unwrap()
        .version("2.0")
        .unwrap()
        .runtime_requirement
        .clone()
        .unwrap();
    assert_eq!(req.abi.as_deref(), Some("universal-darwin"));
}

#[test]
fn two_abi_lines_publish_the_variant_arms_round_trip() {
    // tebako#557 (spec 28 §3): per-triplet builds whose requirements
    // disagree on the ABI line group into variant arms — one
    // `{runtime_requirement, platforms}` row per line. The shorthand
    // fields stay empty (MECE), the artifacts keep their input basenames
    // (the publisher authors the variant infix), and the no-selector
    // pick is the newest ABI line.
    let fx = Fixture::new("variants");
    let mut opts = base_opts(&fx, "app");
    // the variant infix keeps the file-name version derivation from
    // agreeing — the version is explicit
    opts.version = Some("1.0".to_string());
    for (file, triplet, constraint, abi) in [
        (
            "app-1.0-ruby3.3-macos-arm64.tfs",
            Platform::Aarch64Macos,
            "~> 3.3.0",
            "arm64-darwin-23",
        ),
        (
            "app-1.0-ruby3.3-linux-gnu-x86_64.tfs",
            Platform::X86_64LinuxGnu,
            "~> 3.3.0",
            "x86_64-linux-gnu",
        ),
        (
            "app-1.0-ruby4.0-macos-arm64.tfs",
            Platform::Aarch64Macos,
            "~> 4.0.0",
            "arm64-darwin-23",
        ),
        (
            "app-1.0-ruby4.0-linux-gnu-x86_64.tfs",
            Platform::X86_64LinuxGnu,
            "~> 4.0.0",
            "x86_64-linux-gnu",
        ),
    ] {
        opts.payloads.push(PayloadInput {
            triplet: Some(triplet),
            path: write_payload(
                &fx,
                file,
                &app_manifest_native("app", "1.0", constraint, abi),
            ),
        });
    }

    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();

    // the emitted document round-trips through the reader's own parser
    let text = fs::read_to_string(fx.work.join("tpkg-registry.yaml")).unwrap();
    assert!(text.contains("variants:"), "{text}");
    let registry = tebako_resolve::Registry::from_yaml(&text).unwrap();
    let v = registry.payload("app").unwrap().version("1.0").unwrap();
    // MECE with the shorthand: neither top-level platforms: nor
    // runtime_requirement: on a variant entry
    assert!(v.platforms.is_none(), "{:?}", v.platforms);
    assert!(v.runtime_requirement.is_none());
    assert!(v.blksum.is_none());
    let variants = v.variants.as_ref().expect("the variant arms");
    assert_eq!(variants.len(), 2);
    let ids: Vec<String> = variants
        .iter()
        .map(|arm| tebako_resolve::registry::variant_id(arm.runtime_requirement.as_ref()).unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["ruby-mri-3.3".to_string(), "ruby-mri-4.0".to_string()]
    );
    // every arm: its own per-triplet map of the input basenames, the
    // abi-free mirror (each slice's manifest owns the per-platform value)
    for (arm, prefix) in variants.iter().zip(["ruby3.3", "ruby4.0"]) {
        let req = arm.runtime_requirement.as_ref().unwrap();
        assert_eq!(req.abi, None, "{prefix} arm omits abi");
        assert!(arm.blksum.is_none(), "the arm's pins live per row");
        let tebako_resolve::RegistryPlatforms::PerTriplet(map) = &arm.platforms else {
            panic!("per-triplet variant arm");
        };
        assert_eq!(map.len(), 2);
        assert_eq!(
            map[&Platform::Aarch64Macos].artifact,
            format!("app-1.0-{prefix}-macos-arm64.tfs")
        );
        assert_eq!(
            map[&Platform::X86_64LinuxGnu].artifact,
            format!("app-1.0-{prefix}-linux-gnu-x86_64.tfs")
        );
    }

    // the no-selector pick is the newest ABI line; the explicit selector
    // names its arm
    let view = v.resolve_variant(None).unwrap();
    assert_eq!(
        view.picked_by,
        tebako_resolve::registry::VariantPick::NewestLine
    );
    assert_eq!(view.id.as_deref(), Some("ruby-mri-4.0"));
    let Some(tebako_resolve::registry::PlatformSelection::Selected { artifact, .. }) =
        view.select(Platform::Aarch64Macos)
    else {
        panic!("the macos row of the newest line");
    };
    assert_eq!(artifact, "app-1.0-ruby4.0-macos-arm64.tfs");
    let pinned = v.resolve_variant(Some("ruby-mri-3.3")).unwrap();
    assert_eq!(
        pinned.picked_by,
        tebako_resolve::registry::VariantPick::Default
    );
    assert_eq!(pinned.id.as_deref(), Some("ruby-mri-3.3"));

    // the built-in verify installed from the mirror — the newest line's
    // host artifact through the emitted variant entry
    let verified = outcome.verified.unwrap();
    assert!(
        verified.contains("verified: clean-cache install of app 1.0"),
        "{verified}"
    );
}

#[test]
fn two_arms_canonizing_to_one_variant_id_is_a_named_error() {
    // spec 28 §1: builds of one version that canonize to one ABI line
    // are ONE variant — `~> 3.3.0` and `~> 3.3` both derive ruby-mri-3.3.
    let fx = Fixture::new("vardup");
    let mut opts = base_opts(&fx, "app");
    opts.version = Some("1.0".to_string());
    for (file, triplet, constraint, abi) in [
        (
            "app-1.0-r330-macos-arm64.tfs",
            Platform::Aarch64Macos,
            "~> 3.3.0",
            "arm64-darwin-23",
        ),
        (
            "app-1.0-r330-linux-gnu-x86_64.tfs",
            Platform::X86_64LinuxGnu,
            "~> 3.3.0",
            "x86_64-linux-gnu",
        ),
        (
            "app-1.0-r33-macos-arm64.tfs",
            Platform::Aarch64Macos,
            "~> 3.3",
            "arm64-darwin-23",
        ),
        (
            "app-1.0-r33-linux-gnu-x86_64.tfs",
            Platform::X86_64LinuxGnu,
            "~> 3.3",
            "x86_64-linux-gnu",
        ),
    ] {
        opts.payloads.push(PayloadInput {
            triplet: Some(triplet),
            path: write_payload(
                &fx,
                file,
                &app_manifest_native("app", "1.0", constraint, abi),
            ),
        });
    }
    let e = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert_eq!(e.code, 65, "{e:?}");
    assert!(
        e.message.contains("derive the same variant 'ruby-mri-3.3'"),
        "{}",
        e.message
    );
}

#[test]
fn multi_variant_artifact_names_are_the_input_basenames_checked() {
    // a payload file that is not a .tfs
    let fx = Fixture::new("varnotfs");
    let mut opts = base_opts(&fx, "app");
    opts.version = Some("1.0".to_string());
    opts.payloads.push(PayloadInput {
        triplet: Some(Platform::Aarch64Macos),
        path: write_payload(
            &fx,
            "app-1.0-ruby3.3-macos-arm64.tfs",
            &app_manifest_native("app", "1.0", "~> 3.3.0", "arm64-darwin-23"),
        ),
    });
    let bad = fx.work.join("app-1.0-ruby4.0-macos-arm64.img");
    fs::write(
        &bad,
        zip_image(&app_manifest_native(
            "app",
            "1.0",
            "~> 4.0.0",
            "arm64-darwin-23",
        )),
    )
    .unwrap();
    opts.payloads.push(PayloadInput {
        triplet: Some(Platform::Aarch64Macos),
        path: bad,
    });
    let e = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert_eq!(e.code, 64, "{e:?}");
    assert!(e.message.contains("is not a .tfs"), "{}", e.message);

    // a per-triplet name missing the platform suffix
    let fx = Fixture::new("varnosuffix");
    let mut opts = base_opts(&fx, "app");
    opts.version = Some("1.0".to_string());
    opts.payloads.push(PayloadInput {
        triplet: Some(Platform::Aarch64Macos),
        path: write_payload(
            &fx,
            "app-1.0-ruby3.3-macos-arm64.tfs",
            &app_manifest_native("app", "1.0", "~> 3.3.0", "arm64-darwin-23"),
        ),
    });
    opts.payloads.push(PayloadInput {
        triplet: Some(Platform::Aarch64Macos),
        path: write_payload(
            &fx,
            "app-1.0-ruby4.0.tfs",
            &app_manifest_native("app", "1.0", "~> 4.0.0", "arm64-darwin-23"),
        ),
    });
    let e = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert_eq!(e.code, 64, "{e:?}");
    assert!(e.message.contains("does not end"), "{}", e.message);

    // two arms, one basename
    let fx = Fixture::new("varsamename");
    fs::create_dir_all(fx.work.join("a")).unwrap();
    fs::create_dir_all(fx.work.join("b")).unwrap();
    let mut opts = base_opts(&fx, "app");
    opts.version = Some("1.0".to_string());
    for (dir, constraint) in [("a", "~> 3.3.0"), ("b", "~> 4.0.0")] {
        opts.payloads.push(PayloadInput {
            triplet: Some(Platform::Aarch64Macos),
            path: write_payload(
                &fx,
                &format!("{dir}/x-macos-arm64.tfs"),
                &app_manifest_native("app", "1.0", constraint, "arm64-darwin-23"),
            ),
        });
    }
    let e = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert_eq!(e.code, 64, "{e:?}");
    assert!(e.message.contains("share a basename"), "{}", e.message);
}

#[test]
fn tap_formula_renders_from_the_standalones() {
    let fx = Fixture::new("tap");
    let payload = write_payload(
        &fx,
        "my-app-2.0.tfs",
        &app_manifest_yaml("my-app", "2.0", &["my-app"]),
    );
    let mut opts = base_opts(&fx, "my-app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    for (p, tag) in [
        (Platform::Aarch64Macos, b"mac-arm".as_slice()),
        (Platform::X86_64Macos, b"mac-intel".as_slice()),
        (Platform::Aarch64LinuxGnu, b"linux-arm".as_slice()),
        (Platform::X86_64LinuxGnu, b"linux-intel".as_slice()),
    ] {
        let path = fx
            .work
            .join(format!("my-app-2.0-{}", p.release_asset_name()));
        fs::write(&path, tag).unwrap();
        opts.standalones.push((p, path));
    }
    opts.tap = Some("acme/homebrew-tap".to_string());
    opts.tap_dir = Some(fx.work.join("tap"));

    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let formula_path = fx.work.join("tap/Formula/my-app.rb");
    let formula = fs::read_to_string(&formula_path).unwrap();
    assert_eq!(outcome.formula_path.as_ref(), Some(&formula_path));
    assert!(formula.contains("class MyApp < Formula"), "{formula}");
    assert!(formula.contains("version \"2.0\""), "{formula}");
    assert!(
        formula.contains("https://github.com/acme/app/releases/download"),
        "{formula}"
    );
    for (p, tag) in [
        (Platform::Aarch64Macos, b"mac-arm".as_slice()),
        (Platform::X86_64Macos, b"mac-intel".as_slice()),
        (Platform::Aarch64LinuxGnu, b"linux-arm".as_slice()),
        (Platform::X86_64LinuxGnu, b"linux-intel".as_slice()),
    ] {
        let sha = tebako_resolve::sha256_hex(tag);
        assert!(formula.contains(&sha), "{p} sha in formula");
    }
    assert!(
        !formula
            .lines()
            .any(|l| !l.trim_start().starts_with('#') && l.contains("@@")),
        "no placeholder survives outside template comments: {formula}"
    );
    // standalones uploaded alongside the payloads
    assert!(fx.work.join("mirror/1.0/my-app-2.0-macos-arm64").is_file());
}

#[test]
fn tap_requires_all_template_standalones() {
    let fx = Fixture::new("tapmissing");
    let payload = write_payload(
        &fx,
        "app-1.0.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    opts.tap = Some("acme/homebrew-tap".to_string());
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("standalone"), "{err:?}");
    assert!(err.message.contains("macos-arm64"), "{err:?}");
}

#[test]
fn publish_errors_are_named() {
    let fx = Fixture::new("puberr");
    let payload = write_payload(
        &fx,
        "app-1.0.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );

    // gitlab write leg: not this milestone
    let mut opts = base_opts(&fx, "app");
    opts.release = "tfs:gitlab:acme/app:1.0".to_string();
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload.clone(),
    });
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("GitHub releases"), "{err:?}");

    // mixing universal + per-triplet
    let mut opts = base_opts(&fx, "app");
    opts.payloads = vec![
        PayloadInput {
            triplet: None,
            path: payload.clone(),
        },
        PayloadInput {
            triplet: Some(Platform::Aarch64Macos),
            path: payload.clone(),
        },
    ];
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("mix"), "{err:?}");

    // duplicate triplet
    let mut opts = base_opts(&fx, "app");
    opts.payloads = vec![
        PayloadInput {
            triplet: Some(Platform::Aarch64Macos),
            path: payload.clone(),
        },
        PayloadInput {
            triplet: Some(Platform::Aarch64Macos),
            path: payload.clone(),
        },
    ];
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("duplicate"), "{err:?}");

    // not a readable image at all — the stricter, earlier named error
    // (tebako#744): the reader refuses the bytes before the manifest
    // question exists
    let plain = fx.work.join("plain-1.0.tfs");
    fs::write(&plain, b"not an image").unwrap();
    let mut opts = base_opts(&fx, "app");
    opts.version = Some("1.0".to_string());
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: plain,
    });
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("cannot read"), "{err:?}");
    assert!(err.message.contains("not in a format"), "{err:?}");

    // name mismatch against the embedded manifest
    let mut opts = base_opts(&fx, "other-app");
    opts.version = Some("1.0".to_string());
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload.clone(),
    });
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("declares app 1.0"), "{err:?}");

    // version not derivable from odd file names and no --version
    let odd = fx.work.join("mystery.bin");
    fs::write(&odd, zip_image(&app_manifest_yaml("app", "1.0", &["app"]))).unwrap();
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: odd,
    });
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("--version"), "{err:?}");

    // an empty payload is not a publishable .tfs (a 0-byte image has no
    // blksum — the sidecar grammar's size_bytes >= 1, spec 39 §3)
    let empty = fx.work.join("empty-1.0.tfs");
    fs::write(&empty, b"").unwrap();
    let mut opts = base_opts(&fx, "app");
    opts.version = Some("1.0".to_string());
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: empty,
    });
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("is empty"), "{err:?}");

    // an unknown --sign=<keyid> is a named error
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    opts.sign = Some(Some("0123456789abcdef".to_string()));
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("no secret key"), "{err:?}");
}

#[test]
fn version_is_derived_from_the_artifact_names() {
    let fx = Fixture::new("derive");
    let payload = write_payload(
        &fx,
        "app-2.3.4.tfs",
        &app_manifest_yaml("app", "2.3.4", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.release = "tfs:github:acme/app".to_string(); // tag defaults to the version
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    assert_eq!(outcome.version, "2.3.4");
    assert_eq!(outcome.tag, "2.3.4");
}

// ---------------------------------------------------------------------
// the dependency-closure proof (spec 03 §2.3 — publish verifies the
// whole graph the way a user's install resolves it)
// ---------------------------------------------------------------------

/// An app manifest with `requires:` edges (embedded in the published image).
fn app_manifest_with_requires(name: &str, version: &str, requires: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /app/bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\nrequires:\n{requires}",
        sha(b'a'),
        sha(b'b')
    )
}

/// A toolkit image + its one-version registry in the publisher's home.
fn register_dep(fx: &Fixture, name: &str, version: &str) -> String {
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: toolkit\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  platforms: universal\n  capabilities: {{exec: false, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    );
    let dep_path = fx.work.join(format!("{name}-{version}.tfs"));
    fs::write(&dep_path, zip_image(&manifest)).unwrap();
    let dep_url = tebako_http::file_url(&dep_path);
    let registry_path = fx.work.join(format!("{name}-registry.yaml"));
    fs::write(
        &registry_path,
        format!(
            "schema_version: 1\npayloads:\n  - name: {name}\n    kind: toolkit\n    versions:\n      - version: {version}\n        platforms: universal\n        release: {{ref: {dep_url}}}\n    default: {version}\n",
        ),
    )
    .unwrap();
    let registry_ref = tebako_http::file_url(&registry_path);
    tebako_cli::install::add_registry(&fx.home, &registry_ref).unwrap();
    registry_ref
}

#[test]
fn publish_verify_proves_the_dependency_closure_with_the_publishers_registries() {
    let fx = Fixture::new("pubdeps");
    register_dep(&fx, "inkscape", "1.4.3");

    let payload = write_payload(
        &fx,
        "app-1.0.tfs",
        &app_manifest_with_requires(
            "app",
            "1.0",
            "  - kind: toolkit\n    name: inkscape\n    constraint: \">= 1.3\"\n    mount: /opt/inkscape\n",
        ),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let verified = outcome.verified.unwrap();
    assert!(
        verified.contains("verified: clean-cache install of app 1.0"),
        "{verified}"
    );
    assert!(
        verified.contains("1 publisher registry(ies) inherited"),
        "{verified}"
    );
}

#[test]
fn publish_verify_fails_closed_when_a_dep_is_unresolvable() {
    let fx = Fixture::new("pubnodeps");
    let payload = write_payload(
        &fx,
        "app-1.0.tfs",
        &app_manifest_with_requires(
            "app",
            "1.0",
            "  - kind: toolkit\n    name: inkscape\n    constraint: \">= 1.3\"\n    mount: /opt/inkscape\n",
        ),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload,
    });
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("inkscape"), "{err:?}");
    assert!(err.message.contains("add-registry"), "{err:?}");
}

// ---------------------------------------------------------------------
// every payload kind publishes (apps, toolkits, data — spec 03 §2)
// ---------------------------------------------------------------------

/// A toolkit image with two executables (zero-runtime dispatch).
fn toolkit_image_with_executables(name: &str, version: &str, executables: &[&str]) -> Vec<u8> {
    let execs: String = executables
        .iter()
        .map(|e| format!("    - {{name: \"{e}\", path: \"/bin/{e}\", version: \"{version}\"}}\n"))
        .collect();
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: toolkit\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  executables:\n{execs}  libraries: []\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    );
    // the executables must exist in the image — the verify install
    // materializes every zero-runtime entrypoint into the store tree
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("__tpkg__/manifest.yaml", options)
        .unwrap();
    writer.write_all(manifest.as_bytes()).unwrap();
    for e in executables {
        writer.start_file(format!("bin/{e}"), options).unwrap();
        writer.write_all(b"#!/bin/sh\n").unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A data image (mount semantics only).
fn data_image(name: &str, version: &str) -> Vec<u8> {
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: data\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  mount_semantics: {{suggested: \"/\"}}\n  capabilities: {{exec: false, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    );
    zip_image(&manifest)
}

#[test]
fn publish_ships_toolkit_payloads_with_their_executables_as_entrypoints() {
    let fx = Fixture::new("pubtoolkit");
    let path = fx.work.join("openjdk-21.0.12.tfs");
    fs::write(
        &path,
        toolkit_image_with_executables("openjdk", "21.0.12", &["java", "keytool"]),
    )
    .unwrap();
    let mut opts = base_opts(&fx, "openjdk");
    opts.release = "tfs:github:acme/openjdk:21.0.12".to_string();
    opts.payloads.push(PayloadInput {
        triplet: None,
        path,
    });
    let outcome = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let registry = fs::read_to_string(fx.work.join("tpkg-registry.yaml")).unwrap();
    assert!(registry.contains("kind: toolkit"), "{registry}");
    assert!(
        registry.contains("entrypoints:\n    - java\n    - keytool"),
        "{registry}"
    );
    assert!(!registry.contains("runtime_requirement"), "{registry}");
    // the verify proof registered the executables' shims
    let verified = outcome.verified.unwrap();
    assert!(verified.contains("java, keytool"), "{verified}");
}

#[test]
fn publish_ships_data_payloads_with_no_entrypoints() {
    let fx = Fixture::new("pubdata");
    let path = fx.work.join("fonts-2.1.tfs");
    fs::write(&path, data_image("fonts", "2.1")).unwrap();
    let mut opts = base_opts(&fx, "fonts");
    opts.release = "tfs:github:acme/fonts:2.1".to_string();
    opts.payloads.push(PayloadInput {
        triplet: None,
        path,
    });
    publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap();
    let registry = fs::read_to_string(fx.work.join("tpkg-registry.yaml")).unwrap();
    assert!(registry.contains("kind: data"), "{registry}");
    assert!(
        !registry.contains("entrypoints"),
        "a data payload declares no entrypoints: {registry}"
    );
}

// ---------- the OCI dual-publish leg (spec 38 §7/§11) ----------

/// A scripted OCI sink: records every request, answers a digest-pinned
/// origin (the shape production reports); `skip_tags` answers the
/// idempotent re-publish skip for the named tags.
struct ScriptedSink {
    requests: std::sync::Mutex<Vec<publish::OciPushRequest>>,
    skip_tags: Vec<String>,
}

impl ScriptedSink {
    fn new() -> ScriptedSink {
        ScriptedSink {
            requests: std::sync::Mutex::new(Vec::new()),
            skip_tags: Vec::new(),
        }
    }

    fn call(
        &self,
        req: &publish::OciPushRequest,
    ) -> Result<publish::OciPushResult, tebako_cli::error::TebakoError> {
        let skipped = self.skip_tags.contains(&req.tag);
        let origin = format!(
            "tfs+oci://{}/{}@sha256:{}",
            req.host,
            req.repo,
            "d".repeat(64)
        );
        self.requests.lock().unwrap().push(req.clone());
        Ok(publish::OciPushResult { origin, skipped })
    }

    fn taken(&self) -> Vec<publish::OciPushRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[test]
fn per_triplet_publish_with_oci_pushes_and_mirrors_the_rows() {
    let fx = Fixture::new("ocitriplet");
    let mac = write_payload(
        &fx,
        "app-1.0-macos-arm64.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let linux = write_payload(
        &fx,
        "app-1.0-linux-gnu-x86_64.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads = vec![
        PayloadInput {
            triplet: Some(Platform::Aarch64Macos),
            path: mac.clone(),
        },
        PayloadInput {
            triplet: Some(Platform::X86_64LinuxGnu),
            path: linux.clone(),
        },
    ];
    opts.oci = Some("tfs+oci://oci.example/acme/app".to_string());

    let sink = ScriptedSink::new();
    let outcome = publish::publish_full_with_oci_sink(
        &opts,
        &fx.home,
        &fx.work,
        Some(&fx.shim_binary),
        &|req| sink.call(req),
    )
    .unwrap();

    let reqs = sink.taken();
    assert_eq!(
        reqs.len(),
        4,
        "one §3 artifact + one blksum sibling per payload: {reqs:?}"
    );
    assert_eq!(reqs[0].host, "oci.example");
    assert_eq!(reqs[0].repo, "acme/app");
    assert_eq!(reqs[0].tag, "1.0-aarch64-macos");
    assert_eq!(reqs[1].tag, "1.0-x86_64-linux-gnu");
    assert!(matches!(
        reqs[0].class,
        tebako_resolve::ArtifactClass::Payload
    ));
    // the §3 annotation map mirrors the L3 subset
    let ann = &reqs[0].annotations;
    assert_eq!(ann.title.as_deref(), Some("app-1.0-macos-arm64.tfs"));
    assert_eq!(ann.name.as_deref(), Some("app"));
    assert_eq!(ann.version.as_deref(), Some("1.0"));
    assert_eq!(ann.kind.as_deref(), Some("app"));
    assert_eq!(ann.triplet.as_deref(), Some("aarch64-macos"));
    assert_eq!(ann.entrypoints.as_deref(), Some("app"));
    let rr = ann.runtime_requirement.as_deref().unwrap();
    assert!(rr.contains("\"engine\":\"ruby\""), "{rr}");
    assert!(rr.contains("\"constraint\":\""), "{rr}");
    // the SAME staged bytes the release leg uploaded
    assert_eq!(reqs[0].bytes, fs::read(&mac).unwrap());
    assert_eq!(reqs[1].bytes, fs::read(&linux).unwrap());

    // the blksum siblings (spec 39 §8): one `sha256-<hex>.blksum.json`
    // tag per image, keyed by the image's digest, carrying the staged
    // sidecar bytes the registry row pins
    let mac_sha = tebako_resolve::sha256_hex(&fs::read(&mac).unwrap());
    for (req, image_sha, sidecar_name) in [
        (&reqs[2], mac_sha, "app-1.0-macos-arm64.tfs.blksum.json"),
        (
            &reqs[3],
            tebako_resolve::sha256_hex(&fs::read(&linux).unwrap()),
            "app-1.0-linux-gnu-x86_64.tfs.blksum.json",
        ),
    ] {
        assert!(matches!(req.class, tebako_resolve::ArtifactClass::Blksum));
        assert_eq!(req.tag, tebako_resolve::blksum_tag(&image_sha));
        assert_eq!(req.annotations.title.as_deref(), Some(sidecar_name));
        assert_eq!(
            req.annotations.blksum_subject.as_deref(),
            Some(format!("sha256:{image_sha}").as_str())
        );
        assert_eq!(
            req.bytes,
            fs::read(fx.work.join(format!("mirror/1.0/{sidecar_name}"))).unwrap()
        );
    }

    // the registry rows mirror the per-triplet oci: locator
    let registry = registry_at(&fx);
    let v = registry.payload("app").unwrap().version("1.0").unwrap();
    let Some(tebako_resolve::RegistryPlatforms::PerTriplet(map)) = &v.platforms else {
        panic!("per-triplet platforms");
    };
    assert_eq!(
        map[&Platform::Aarch64Macos].oci.as_deref(),
        Some("tfs+oci://oci.example/acme/app:1.0-aarch64-macos")
    );
    assert_eq!(
        map[&Platform::X86_64LinuxGnu].oci.as_deref(),
        Some("tfs+oci://oci.example/acme/app:1.0-x86_64-linux-gnu")
    );
    // …and the release.ref stays the primary (dual publish, §11)
    assert_eq!(v.release.r#ref, "tfs:github:acme/app:1.0");

    // the outcome records the digest-pinned origins
    assert_eq!(outcome.oci_refs.len(), 4);
    assert!(outcome.oci_refs[0].starts_with("tfs+oci://oci.example/acme/app@sha256:"));
}

#[test]
fn signed_publish_with_oci_pushes_the_signature_siblings() {
    let fx = Fixture::new("ocisigned");
    let payload = write_payload(
        &fx,
        "app-1.0.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: payload.clone(),
    });
    opts.sign = Some(None); // the press-local key
    opts.oci = Some("tfs+oci://oci.example/acme/app".to_string());

    let sink = ScriptedSink::new();
    let outcome = publish::publish_full_with_oci_sink(
        &opts,
        &fx.home,
        &fx.work,
        Some(&fx.shim_binary),
        &|req| sink.call(req),
    )
    .unwrap();

    let sha = tebako_resolve::sha256_hex(&fs::read(&payload).unwrap());
    let reqs = sink.taken();
    assert_eq!(
        reqs.len(),
        3,
        "the payload + its signature sibling + its blksum sibling"
    );
    // the universal payload rides the <version> tag
    assert_eq!(reqs[0].tag, "1.0");
    assert_eq!(reqs[0].annotations.triplet.as_deref(), Some("universal"));
    // the sibling tag keys on the SIGNED BLOB's digest (spec 38 §3)
    let sig = &reqs[1];
    assert!(matches!(
        sig.class,
        tebako_resolve::ArtifactClass::Signature
    ));
    assert_eq!(sig.tag, tebako_resolve::signature_tag(&sha));
    assert_eq!(sig.annotations.title.as_deref(), Some("app-1.0.tfs.asc"));
    assert_eq!(
        sig.annotations.signature_keyid.as_deref(),
        Some(outcome.signer.as_deref().unwrap())
    );
    assert_eq!(
        sig.annotations.signature_subject.as_deref(),
        Some(format!("sha256:{sha}").as_str())
    );
    assert_eq!(
        sig.bytes,
        fs::read(fx.work.join("mirror/1.0/app-1.0.tfs.asc")).unwrap()
    );

    // the blksum sibling rides signed publishes too — it is integrity
    // data about the image, not a signature (spec 39 §8)
    let blk = &reqs[2];
    assert!(matches!(blk.class, tebako_resolve::ArtifactClass::Blksum));
    assert_eq!(blk.tag, tebako_resolve::blksum_tag(&sha));
    assert_eq!(
        blk.annotations.title.as_deref(),
        Some("app-1.0.tfs.blksum.json")
    );
    assert_eq!(
        blk.annotations.blksum_subject.as_deref(),
        Some(format!("sha256:{sha}").as_str())
    );
    assert_eq!(
        blk.bytes,
        fs::read(fx.work.join("mirror/1.0/app-1.0.tfs.blksum.json")).unwrap()
    );

    // spec 38 §7's row mirror is per-triplet only: the universal row
    // keeps the primary ref alone, and the publish says so
    let registry = registry_at(&fx);
    let v = registry.payload("app").unwrap().version("1.0").unwrap();
    assert!(matches!(
        v.platforms,
        Some(tebako_resolve::RegistryPlatforms::Universal)
    ));
    assert!(
        outcome
            .notes
            .iter()
            .any(|n| n.contains("row mirror is per-triplet only")),
        "{:?}",
        outcome.notes
    );
}

#[test]
fn oci_republish_skips_and_sink_errors_propagate() {
    let fx = Fixture::new("ociskip");
    let payload = write_payload(
        &fx,
        "app-1.0-macos-arm64.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let mut opts = base_opts(&fx, "app");
    opts.payloads.push(PayloadInput {
        triplet: Some(Platform::Aarch64Macos),
        path: payload,
    });
    opts.oci = Some("tfs+oci://oci.example/acme/app".to_string());

    // the tag already names these bytes: the idempotent skip is a note,
    // never an error
    let mut sink = ScriptedSink::new();
    sink.skip_tags = vec!["1.0-aarch64-macos".to_string()];
    let outcome = publish::publish_full_with_oci_sink(
        &opts,
        &fx.home,
        &fx.work,
        Some(&fx.shim_binary),
        &|req| sink.call(req),
    )
    .unwrap();
    assert!(
        outcome
            .notes
            .iter()
            .any(|n| n.contains("idempotent re-publish skip")),
        "{:?}",
        outcome.notes
    );

    // a sink failure (production: OciTagConflict's 69) propagates
    // verbatim — publish never retries, never downgrades
    let err = publish::publish_full_with_oci_sink(
        &opts,
        &fx.home,
        &fx.work,
        Some(&fx.shim_binary),
        &|_req| {
            Err(tebako_cli::error::TebakoError::new(
                "OciTagConflict: tag moved".to_string(),
                69,
            ))
        },
    )
    .unwrap_err();
    assert_eq!(err.code, 69);
    assert!(err.message.contains("OciTagConflict"), "{err:?}");
}

#[test]
fn the_oci_option_names_the_bare_repo_only() {
    let fx = Fixture::new("ociusage");
    let payload = write_payload(
        &fx,
        "app-1.0.tfs",
        &app_manifest_yaml("app", "1.0", &["app"]),
    );
    let hex = "a".repeat(64);
    for bad in [
        "tfs+oci://oci.example/acme/app:1.0".to_string(),
        format!("tfs+oci://oci.example/acme/app@sha256:{hex}"),
        format!("tfs+oci://oci.example/acme/app?sha256={hex}"),
        "tfs:github:acme/app:1.0".to_string(),
    ] {
        let mut opts = base_opts(&fx, "app");
        opts.payloads = vec![PayloadInput {
            triplet: None,
            path: payload.clone(),
        }];
        opts.oci = Some(bad.clone());
        let sink = ScriptedSink::new();
        let err = publish::publish_full_with_oci_sink(
            &opts,
            &fx.home,
            &fx.work,
            Some(&fx.shim_binary),
            &|req| sink.call(req),
        )
        .unwrap_err();
        assert_eq!(err.code, 64, "{bad}: {err:?}");
        assert!(err.message.contains("bare tfs+oci://"), "{bad}: {err:?}");
        assert!(sink.taken().is_empty(), "{bad}: nothing pushed");
    }
}
