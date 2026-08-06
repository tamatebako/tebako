//! `tebako install <path>` — the local-package install (TODO.v2-1/12):
//! slices land from the package's own bytes with the trailer's digests
//! as anchors, idempotent skip, the drift rule, the NO_INSTALL refusal,
//! runtime-slot skipping, and explicit-only shims. No network, temp
//! TEBAKO_HOMEs throughout.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use tebako_cli::install;

/// The registered shim path for a command — windows names it
/// `<command>.exe` (production's own mapping, tebako-shim#manage).
fn shim_path(home: &Path, command: &str) -> PathBuf {
    home.join("shims")
        .join(tebako_shim::manage::shim_file_name(command))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-cli-install-local-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sha(byte: u8) -> String {
    use sha2::Digest as _;
    tpkg_hex(&sha2::Sha256::digest([byte; 32]))
}

fn tpkg_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_of(path: &Path) -> String {
    use sha2::Digest as _;
    tpkg_hex(&sha2::Sha256::digest(fs::read(path).unwrap()))
}

/// A ZIP image carrying an embedded manifest (mountable by the tfs zip
/// backend) — the same fixture shape the registry install tests use.
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

fn embedded_manifest_yaml(name: &str, version: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /app/bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    )
}

/// Stitch a tpkg package (fake base bytes + slots) with the given flags.
fn pressed_package(
    dir: &Path,
    name: &str,
    images: &[(PathBuf, String, u32)],
    package_flags: u32,
) -> PathBuf {
    let base = dir.join(format!("{name}.base"));
    fs::write(&base, b"FAKE-BASE-BYTES").unwrap();
    let out = dir.join(name);
    let pkg_images: Vec<tebako_pkg::PackageImage> = images
        .iter()
        .map(|(path, mount, format_id)| tebako_pkg::PackageImage {
            path: path.clone(),
            mount_point: mount.clone(),
            format_id: *format_id,
        })
        .collect();
    let options = tebako_pkg::PackageOptions {
        runtime_ref: "ruby@3.3.7;tebako=9.9.9".to_string(),
        package_flags,
        launcher_abi: 1,
        ..Default::default()
    };
    tebako_pkg::bundle_exact(&base, &pkg_images, &out, &options).unwrap();
    out
}

fn payload_image(dir: &Path, file: &str, name: &str, version: &str) -> PathBuf {
    let path = dir.join(file);
    fs::write(
        &path,
        zip_image_with_manifest(&embedded_manifest_yaml(name, version)),
    )
    .unwrap();
    path
}

fn payloads_dir(home: &Path) -> PathBuf {
    home.join("payloads")
}

#[test]
fn install_lands_the_slice_with_markers_and_mirror() {
    let dir = scratch("lands");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let img = payload_image(&dir, "app.tfs", "app", "1.0");
    let pkg = pressed_package(
        &dir,
        "myapp",
        &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
        0,
    );

    let outcome = install::install_local(&home, &pkg, false, None).unwrap();
    assert_eq!(outcome.installed.len(), 1);
    let slice = &outcome.installed[0];
    assert_eq!(
        (slice.name.as_str(), slice.version.as_str()),
        ("app", "1.0")
    );
    assert_eq!(slice.status, tebako_resolve::InstallStatus::Installed);
    assert_eq!(slice.commands, vec!["app".to_string()]);

    let cached = payloads_dir(&home).join("app").join("1.0.tfs");
    assert!(cached.is_file());
    // the trust anchor is the computed digest (unsigned package)
    assert_eq!(slice.sha256, sha256_of(&cached));
    assert!(cached.with_file_name("1.0.tfs.sha256").is_file());
    let origin = fs::read_to_string(cached.with_file_name("1.0.tfs.origin")).unwrap();
    assert!(origin.contains("payload="), "{origin}");
    assert!(origin.contains("slot=0"), "{origin}");
    // the manifest mirror mirrors the embedded manifest
    let mirror = tebako_shim::manifest::Manifest::load(
        &payloads_dir(&home).join("app").join("1.0.manifest.yaml"),
    )
    .unwrap();
    assert_eq!(mirror.entrypoint("app").unwrap().path, "/app/bin/app");
    // no shims without the explicit ask
    assert!(outcome.shims.is_empty());
    assert!(!shim_path(&home, "app").exists());
}

#[test]
fn reinstall_is_a_skip_and_drift_never_overwrites() {
    let dir = scratch("drift");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let img = payload_image(&dir, "app.tfs", "app", "1.0");
    let pkg = pressed_package(
        &dir,
        "myapp",
        &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
        0,
    );
    install::install_local(&home, &pkg, false, None).unwrap();

    // second install of the same package: silent skip
    let again = install::install_local(&home, &pkg, false, None).unwrap();
    assert_eq!(
        again.installed[0].status,
        tebako_resolve::InstallStatus::Hit
    );
    assert!(
        again.notes.iter().any(|n| n.contains("already installed")),
        "{:?}",
        again.notes
    );

    // same identity, DIFFERENT bytes: loud warning, never overwrite
    let other_img = dir.join("app2.tfs");
    let mut bytes = zip_image_with_manifest(&embedded_manifest_yaml("app", "1.0"));
    bytes.extend_from_slice(b"DRIFT");
    fs::write(&other_img, bytes).unwrap();
    let pkg2 = pressed_package(
        &dir,
        "myapp2",
        &[(other_img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
        0,
    );
    let third = install::install_local(&home, &pkg2, false, None).unwrap();
    assert!(
        third.notes.iter().any(|n| n.contains("DIFFERENT content")),
        "{:?}",
        third.notes
    );
    let first_sha = install::install_local(&home, &pkg, false, None)
        .unwrap()
        .installed[0]
        .sha256
        .clone();
    assert_eq!(
        third.installed[0].sha256, first_sha,
        "the installed slice was replaced"
    );
}

#[test]
fn no_install_flag_refuses_with_zero_writes() {
    let dir = scratch("refused");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let img = payload_image(&dir, "app.tfs", "app", "1.0");
    let pkg = pressed_package(
        &dir,
        "frozen",
        &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
        tpkg::TPKG_FLAG_NO_INSTALL,
    );
    let err = install::install_local(&home, &pkg, false, None).unwrap_err();
    assert_eq!(err.code, 76, "{err}");
    assert!(err.to_string().contains("non-installable"), "{err}");
    assert!(
        !payloads_dir(&home).exists(),
        "a refused install wrote into the store"
    );
}

#[test]
fn lean_package_installs_and_the_runtime_slot_is_skipped_with_a_note() {
    let dir = scratch("lean");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let img = payload_image(&dir, "app.tfs", "app", "1.0");
    let runtime_blob = dir.join("runtime.bin");
    fs::write(&runtime_blob, b"FAKE RUNTIME").unwrap();
    let pkg = pressed_package(
        &dir,
        "leanpkg",
        &[
            (img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS),
            (runtime_blob, String::new(), tpkg::TPKG_FORMAT_RUNTIME),
        ],
        tpkg::TPKG_FLAG_LEAN,
    );
    let outcome = install::install_local(&home, &pkg, false, None).unwrap();
    assert_eq!(
        outcome.installed.len(),
        1,
        "only the payload slice installs"
    );
    assert!(
        outcome.notes.iter().any(|n| n.contains("runtime slot")),
        "{:?}",
        outcome.notes
    );
}

#[test]
fn a_slot_without_an_embedded_manifest_is_a_named_error() {
    let dir = scratch("nomanifest");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let bare = dir.join("bare.tfs");
    // a zip image WITHOUT /__tpkg__/manifest.yaml
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file("app/bin/app", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"#!/bin/sh\n").unwrap();
    fs::write(&bare, writer.finish().unwrap().into_inner()).unwrap();
    let pkg = pressed_package(
        &dir,
        "bare",
        &[(bare, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
        0,
    );
    let err = install::install_local(&home, &pkg, false, None).unwrap_err();
    assert_eq!(err.code, 76, "{err}");
    assert!(err.to_string().contains("no embedded manifest"), "{err}");
}

#[test]
fn shims_link_only_with_the_explicit_flag() {
    let dir = scratch("shims");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let shim_binary = dir.join("tebako-shim");
    fs::write(&shim_binary, b"#!/bin/sh\n").unwrap();
    let img = payload_image(&dir, "app.tfs", "app", "1.0");
    let pkg = pressed_package(
        &dir,
        "myapp",
        &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
        0,
    );

    let plain = install::install_local(&home, &pkg, false, None).unwrap();
    assert!(plain.shims.is_empty());

    let with = install::install_local(&home, &pkg, true, Some(&shim_binary)).unwrap();
    assert_eq!(with.shims.len(), 1);
    assert!(shim_path(&home, "app").exists());
}
