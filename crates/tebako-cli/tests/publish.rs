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

    // the registry records the entry (universal, signature pin, the
    // github release ref — mirror mode does not leak into the ref)
    let registry = registry_at(&fx);
    let app = registry.payload("app").unwrap();
    assert_eq!(app.default.as_deref(), Some("1.0"));
    let v = app.version("1.0").unwrap();
    assert!(matches!(
        v.platforms,
        tebako_resolve::RegistryPlatforms::Universal
    ));
    assert_eq!(v.release.r#ref, "tfs:github:acme/app:1.0");
    let sig = v.signature.clone().unwrap();
    assert_eq!(sig.keyid, keyid);
    assert_eq!(sig.asc, "app-1.0.tfs.asc");
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
    let tebako_resolve::RegistryPlatforms::PerTriplet(map) = &v.platforms else {
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

    // no embedded manifest
    let plain = fx.work.join("plain-1.0.tfs");
    fs::write(&plain, b"not an image").unwrap();
    let mut opts = base_opts(&fx, "app");
    opts.version = Some("1.0".to_string());
    opts.payloads.push(PayloadInput {
        triplet: None,
        path: plain,
    });
    let err = publish::publish_full(&opts, &fx.home, &fx.work, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("embedded manifest"), "{err:?}");

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
