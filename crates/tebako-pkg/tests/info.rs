//! The spec-15 `tebako-pkg info` surface and `tebako-pkg validate`: the
//! full container report, slot payload views, the JSON contract
//! (`info_schema: 1`), depth/slot selection, per-slot format detection
//! and the strict verification exit codes (0/65/70/71/72).
//!
//! Fixtures: packages are built in-test with `tebako-pkg bundle`
//! (+ `--sign`; the crate's own machinery), images via the in-process
//! dwarfs-t writer carrying the crates/tpkg manifest fixtures, plus
//! zip/tar builders and the vendored SquashFS fixture. The default
//! (flag-less) info output is the C++ parity dump, covered by
//! golden.rs/cli.rs.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tebako_contract_tests::TempDir;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako-pkg"))
}

fn test_home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-pkg-info-home-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str], cwd: &Path, home: &Path) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("TEBAKO_HOME", home)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const APP_MANIFEST: &str = include_str!("../../tpkg/tests/fixtures/manifests/app-suite.yaml");
const DATA_MANIFEST: &str = include_str!("../../tpkg/tests/fixtures/manifests/data.yaml");

/// A dwarfs-t image built in-process (optionally carrying a manifest).
fn mk_image(w: &TempDir, name: &str, manifest: Option<&str>) -> PathBuf {
    mk_image_files(w, name, manifest, &[])
}

/// mk_image plus extra files (`(path, bytes)` pairs, parent dirs created).
fn mk_image_files(
    w: &TempDir,
    name: &str,
    manifest: Option<&str>,
    files: &[(&str, &[u8])],
) -> PathBuf {
    let src = w.0.join(format!("src-{name}"));
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("hello.txt"), b"hi").unwrap();
    for (rel, bytes) in files {
        let p = src.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    if let Some(text) = manifest {
        std::fs::create_dir_all(src.join("__tpkg__")).unwrap();
        std::fs::write(src.join("__tpkg__/manifest.yaml"), text).unwrap();
    }
    let img = w.0.join(name);
    let mut writer = dwarfs_t::Writer::new(dwarfs_t::WriterOptions::default()).unwrap();
    writer.add_tree(&src, "/").unwrap();
    writer.write(&img).unwrap();
    img
}

/// The app-suite image (APP_MANIFEST declares the `metanorma` and
/// `metanorma-nokogiri` entrypoints) WITH both entry files present — the
/// shape the entries cross-check (tebako#494) can fully pass on.
fn mk_suite_image(w: &TempDir, name: &str) -> PathBuf {
    let src = w.0.join(format!("src-{name}"));
    std::fs::create_dir_all(src.join("bin")).unwrap();
    std::fs::write(src.join("bin/metanorma"), b"#!/bin/sh\n").unwrap();
    std::fs::write(src.join("bin/metanorma-nokogiri"), b"#!/bin/sh\n").unwrap();
    std::fs::create_dir_all(src.join("__tpkg__")).unwrap();
    std::fs::write(src.join("__tpkg__/manifest.yaml"), APP_MANIFEST).unwrap();
    let img = w.0.join(name);
    let mut writer = dwarfs_t::Writer::new(dwarfs_t::WriterOptions::default()).unwrap();
    writer.add_tree(&src, "/").unwrap();
    writer.write(&img).unwrap();
    img
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}

fn patterned_bytes(n: usize, seed: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    let mut x = seed;
    for _ in 0..n {
        x = x.wrapping_mul(31).wrapping_add(17);
        v.push(x);
    }
    v
}

/// A minimal ustar tar with one file. Only the squashfs-era detection
/// test uses it (POSIX — TODO.v2-1/02).
#[cfg(not(windows))]
fn build_tar(path: &Path, name: &str, content: &[u8]) {
    let mut out = Vec::new();
    let mut hdr = [0u8; 512];
    hdr[0..name.len()].copy_from_slice(name.as_bytes());
    hdr[100..108].copy_from_slice(b"0000644\0");
    hdr[108..116].copy_from_slice(b"0000000\0");
    hdr[116..124].copy_from_slice(b"0000000\0");
    let size_field = format!("{:011o}\0", content.len());
    hdr[124..136].copy_from_slice(size_field.as_bytes());
    hdr[136..148].copy_from_slice(b"00000000000\0");
    hdr[148..156].copy_from_slice(b"        ");
    hdr[156] = b'0';
    hdr[257..263].copy_from_slice(b"ustar\0");
    hdr[263..265].copy_from_slice(b"00");
    let sum: u32 = hdr.iter().map(|&b| u32::from(b)).sum();
    let sum_field = format!("{sum:06o}\0 ");
    hdr[148..156].copy_from_slice(sum_field.as_bytes());
    out.extend_from_slice(&hdr);
    out.extend_from_slice(content);
    let pad = (512 - content.len() % 512) % 512;
    out.extend(std::iter::repeat(0).take(pad));
    out.extend(std::iter::repeat(0).take(1024));
    std::fs::write(path, out).unwrap();
}

fn bootstrap(w: &TempDir) -> PathBuf {
    let boot = w.0.join("boot.bin");
    std::fs::write(&boot, patterned_bytes(2048, 0x5A)).unwrap();
    boot
}

/// A valid L2 package manifest (spec 03 §6), pressed into the packages of
/// the verify-stage tests: the strict gate's contract stage (spec 18 C6,
/// exit 77) refuses pre-era packages, so the stages these tests exercise
/// (65/70/71/72/0) must press era-2 packages.
const PACKAGE_MANIFEST_YAML: &str = "schema_version: 1\n\
     package: {name: probe, version: 1.0.0, producer: {tool: tebako-pkg, tool_version: 0.1.0}, created: 2026-08-01T00:00:00Z}\n\
     entries:\n  - {name: probe, slot: 0, entrypoint: probe, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n";

fn package_manifest_file(w: &TempDir) -> PathBuf {
    let f = w.0.join("package-manifest.yaml");
    std::fs::write(&f, PACKAGE_MANIFEST_YAML).unwrap();
    f
}

/// bundle (via the binary) and assert success.
fn bundle(home: &Path, w: &TempDir, extra: &[&str], images: &[&Path], out: &Path) {
    let boot = bootstrap(w);
    let mut args: Vec<String> = vec![
        "bundle".into(),
        "--bootstrap".into(),
        boot.to_str().unwrap().into(),
    ];
    for img in images {
        args.push("--image".into());
        args.push(img.to_str().unwrap().into());
    }
    args.push("-o".into());
    args.push(out.to_str().unwrap().into());
    args.extend(extra.iter().map(|s| s.to_string()));
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (rc, _, err) = run(&arg_refs, &w.0, home);
    assert_eq!((rc, err.as_str()), (0, ""), "bundle must succeed: {args:?}");
}

// ---------------------------------------------------------------------
// --full (the container report)
// ---------------------------------------------------------------------

#[test]
fn full_report_unsigned_classic() {
    let w = TempDir::new("pfull1");
    let home = test_home("pfull1");
    let app = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let pkg = w.0.join("metanorma");
    bundle(&home, &w, &[], &[&app], &pkg);

    let (rc, out, err) = run(&["info", "--full", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!((rc, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("package: metanorma (tpkg v1, launcher_abi 0)\n"),
        "{out}"
    );
    assert!(out.contains("  size: "), "{out}");
    assert!(
        out.contains(" B  trailer: 446 B (166 header + 1 slot × 280)\n"),
        "{out}"
    );
    assert!(
        out.contains("  bootstrap: 2,048 B (portion before slot 0)\n"),
        "{out}"
    );
    assert!(
        out.contains("  runtime_ref: (none — classic bundle)\n"),
        "{out}"
    );
    assert!(
        out.contains("  trust: unsigned (v1 legacy trailer) — unverified\n"),
        "{out}"
    );
    assert!(out.contains("    [0] "), "{out}");
    assert!(
        out.contains(" B @ 2,048  format: dwarfs  mount: /__tfs__\n"),
        "{out}"
    );
    assert!(
        out.contains("     kind: app  metanorma 1.2.3  (2 entrypoints, runtime ruby >= 3.3, < 5.0; ruby ~> 3.3.0)\n"),
        "{out}"
    );
    // No lean marker, no lean trailing line, no backend lines at depth 1.
    assert!(!out.contains("(tpkg v1, lean"), "{out}");
    assert!(!out.contains("lean: none"), "{out}");
    assert!(!out.contains("backend:"), "{out}");
}

// squashfs slots mount on POSIX only — the windows tfs ships no squashfs
// backend, so the probe renders the named mount-failure state there
// (TODO.v2-1/02).
#[cfg(not(windows))]
#[test]
fn full_report_signed_lean_and_depths() {
    let w = TempDir::new("pfull2");
    let home = test_home("pfull2");
    let app = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let sqfs = fixture("simple.sqfs");
    let pkg = w.0.join("pkg");
    bundle(
        &home,
        &w,
        &[
            "--lean",
            "--runtime-ref",
            "ruby@3.4.2;tebako=0.15.9",
            "--launcher-abi",
            "1",
            "--sign",
        ],
        &[&app, &sqfs],
        &pkg,
    );

    let (rc, out, err) = run(&["info", "--full", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!((rc, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("package: pkg (tpkg v1, lean, launcher_abi 1)\n"),
        "{out}"
    );
    assert!(out.contains("166 header + 2 slots × 280 + "), "{out}");
    assert!(out.contains(" v2 extension)\n"), "{out}");
    assert!(
        out.contains("  runtime_ref: ruby@3.4.2;tebako=0.15.9 (resolution hint; lean)\n"),
        "{out}"
    );
    assert!(out.contains("  trust: v2-signed, signer "), "{out}");
    assert!(out.contains(" — unverified\n"), "{out}");
    assert!(out.contains("format: dwarfs  mount: /__tfs__\n"), "{out}");
    assert!(
        out.contains("format: squashfs  mount: /__tfs_1__\n"),
        "{out}"
    );
    assert!(
        out.contains("     (no payload manifest — plain image)\n"),
        "{out}"
    );
    assert!(
        out.contains("    [2] — runtime payload slots are never mounted; lean: none\n"),
        "{out}"
    );

    // Depth 0: trailer only — format hints, no payload reads.
    let (rc, out, _) = run(
        &["info", "--full", "--depth", "0", pkg.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("format: dwarfs"), "{out}");
    assert!(!out.contains("kind: app"), "{out}");
    assert!(!out.contains("plain image"), "{out}");

    // Depth 2: backend metadata per slot.
    let (rc, out, _) = run(
        &["info", "--full", "--depth", "2", pkg.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("     backend: {"), "{out}");
    assert!(out.contains("block_size"), "{out}");
    assert!(
        out.contains("     backend: (no metadata surface)\n"),
        "{out}"
    );

    // --depth 9 is a named error.
    let (rc, _, err) = run(
        &["info", "--full", "--depth", "9", pkg.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 1);
    assert!(err.contains("invalid --depth value"), "{err}");
}

#[test]
fn full_report_runtime_legacy_role_slot() {
    let w = TempDir::new("pfat");
    // Slot 0 carries the `probe` file the package manifest's entry names
    // (the entries cross-check stats it — tebako#494).
    let app = mk_image_files(&w, "app.tfs", None, &[("probe", b"#!/bin/sh\n")]);
    let rt = mk_image(&w, "runtime.tfs", None);
    let boot = bootstrap(&w);
    let pkg = w.0.join("fat");
    // A fat package's runtime payload slot carries format_id 4 (the v1
    // legacy role wart) — built via the lib's bundle_exact (the binary's
    // sniff never assigns 4). The package manifest makes it era-2 (the
    // strict gate's contract stage below).
    tebako_pkg::bundle_exact(
        &boot,
        &[
            tebako_pkg::PackageImage {
                path: app,
                mount_point: "/__tfs__".into(),
                format_id: tpkg::TPKG_FORMAT_DWARFS,
            },
            tebako_pkg::PackageImage {
                path: rt,
                mount_point: String::new(),
                format_id: tpkg::TPKG_FORMAT_RUNTIME,
            },
        ],
        &pkg,
        &tebako_pkg::PackageOptions {
            package_manifest: Some(
                tpkg::PackageManifest::from_yaml(PACKAGE_MANIFEST_YAML).unwrap(),
            ),
            ..Default::default()
        },
    )
    .unwrap();

    let home = test_home("pfat");
    let (rc, out, _) = run(&["info", "--full", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("    [1] ")
            && out.contains("format: runtime (legacy role)  mount: (never mounted)\n"),
        "{out}"
    );
    // No lean trailing line for a fat package, and the runtime slot is
    // never mounted (its table row is the last line — no payload read).
    assert!(!out.contains("lean: none"), "{out}");
    assert!(
        out.trim_end()
            .ends_with("format: runtime (legacy role)  mount: (never mounted)"),
        "{out}"
    );

    // --slot on the runtime slot is a named note, not an image read.
    let (rc, _, err) = run(&["info", "--slot", "1", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 1);
    assert!(err.contains("never mounted"), "{err}");

    // validate skips the runtime slot's manifest checks (a launcher, not
    // an image payload) instead of failing them.
    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("  slot[1] manifest: skip — runtime (legacy role) — never mounted\n"),
        "{out}"
    );
    assert!(out.contains("result: PASS\n"), "{out}");
}

// ---------------------------------------------------------------------
// --slot
// ---------------------------------------------------------------------

#[test]
fn slot_view_and_errors() {
    let w = TempDir::new("pslot");
    let home = test_home("pslot");
    let app = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let sqfs = fixture("simple.sqfs");
    let pkg = w.0.join("pkg");
    bundle(&home, &w, &["--sign"], &[&app, &sqfs], &pkg);

    let (rc, out, err) = run(&["info", "--slot", "0", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!((rc, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("package slot [0] of "), "{out}");
    assert!(
        out.contains("  format: dwarfs-t (flatbuffers metadata)  ro  "),
        "{out}"
    );
    assert!(
        out.contains("  kind: app  name: metanorma  version: 1.2.3\n"),
        "{out}"
    );
    assert!(
        out.contains("    entrypoint metanorma → /__app__/bin/metanorma"),
        "{out}"
    );
    assert!(out.contains("  manifest:\n"), "{out}");
    assert!(out.contains("      schema_version: 1\n"), "{out}");

    // A plain-image slot reports the named note. POSIX only — the
    // windows tfs ships no squashfs backend; the sqfs slot reports the
    // named mount failure there (TODO.v2-1/02).
    #[cfg(not(windows))]
    {
        let (rc, out, _) = run(&["info", "--slot", "1", pkg.to_str().unwrap()], &w.0, &home);
        assert_eq!(rc, 0, "{out}");
        assert!(out.contains("  format: squashfs  ro  "), "{out}");
        assert!(
            out.contains("  manifest: none (no /__tpkg__/manifest.yaml — plain image)\n"),
            "{out}"
        );
    }

    // Out of range is a named error.
    let (rc, _, err) = run(&["info", "--slot", "9", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 1);
    assert!(err.contains("slot index 9 out of range"), "{err}");

    let (rc, _, err) = run(&["info", "--slot", "x", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 1);
    assert!(err.contains("invalid --slot value"), "{err}");
}

// ---------------------------------------------------------------------
// --json
// ---------------------------------------------------------------------

#[test]
fn json_document_contract() {
    let w = TempDir::new("pjson");
    let home = test_home("pjson");
    let app = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let sqfs = fixture("simple.sqfs");
    let pkg = w.0.join("pkg");
    bundle(&home, &w, &["--lean", "--sign"], &[&app, &sqfs], &pkg);

    let (rc, out, _) = run(&["info", "--json", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    let doc = tebako_pkg::json_parse(&out).expect("output must parse as JSON");
    assert_eq!(
        doc.find("info_schema").and_then(|v| v.as_u64()),
        Some(1),
        "{out}"
    );
    let artifact = doc.find("artifact").unwrap();
    assert_eq!(
        artifact.find("kind").and_then(|v| v.as_string()).as_deref(),
        Some("package")
    );
    let package = doc.find("package").unwrap();
    assert_eq!(
        format!("{:?}", package.find("flags").unwrap()),
        r#"Array([String("lean"), String("signed-v2")])"#
    );
    assert_eq!(
        package.find("launcher_abi").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert!(package.find("bootstrap_bytes").and_then(|v| v.as_u64()) == Some(2048));
    let trust = doc.find("trust").unwrap();
    assert_eq!(
        trust.find("state").and_then(|v| v.as_string()).as_deref(),
        Some("v2-signed")
    );
    assert_eq!(
        trust.find("outcome").and_then(|v| v.as_string()).as_deref(),
        Some("unverified")
    );
    assert!(trust.find("keyid").is_some(), "{out}");
    let tebako_pkg::JsonValue::Array(slots) = doc.find("slots").unwrap() else {
        panic!("slots must be an array");
    };
    assert_eq!(slots.len(), 2, "{out}");
    assert_eq!(
        slots[0]
            .find("format")
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("dwarfs")
    );
    assert_eq!(
        slots[0]
            .find("detected_format")
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("dwarfs-t (flatbuffers metadata)")
    );
    assert!(slots[0].find("manifest").is_some(), "{out}");
    assert_eq!(
        slots[1]
            .find("format")
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("squashfs")
    );
    // squashfs mounts on POSIX only — windows carries the named
    // mount_error instead of the plain-image note (TODO.v2-1/02).
    #[cfg(not(windows))]
    assert!(
        slots[1]
            .find("manifest_note")
            .and_then(|v| v.as_string())
            .unwrap()
            .contains("plain image"),
        "{out}"
    );

    // --json on a plain archive is a named error (no tpkg container).
    let (rc, _, err) = run(&["info", "--json", sqfs.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 1);
    assert!(err.contains("no tpkg trailer present"), "{err}");
}

// ---------------------------------------------------------------------
// validate / --verify (spec 15 §5 exit codes)
// ---------------------------------------------------------------------

// squashfs slots mount on POSIX only (TODO.v2-1/02) — on windows the
// slot's manifest check fails the mount (named ENOTSUP), never skips.
#[cfg(not(windows))]
#[test]
fn validate_signed_plain_slots_passes() {
    let w = TempDir::new("pok");
    let home = test_home("pok");
    let z = w.0.join("z.zip");
    tebako_contract_tests::build_zip(
        &z,
        &["content/"],
        &[("content/a.txt", b"a"), ("probe", b"#!/bin/sh\n")],
    );
    let sqfs = fixture("simple.sqfs");
    let pm = package_manifest_file(&w);
    let pkg = w.0.join("pkg");
    bundle(
        &home,
        &w,
        &["--sign", "--package-manifest", pm.to_str().unwrap()],
        &[&z, &sqfs],
        &pkg,
    );

    let (rc, out, err) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!((rc, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("  trailer: ok — structural validation (spec 02 §6)\n"),
        "{out}"
    );
    assert!(
        out.contains("  slot[0] sha256: ok — digest matches\n"),
        "{out}"
    );
    assert!(
        out.contains("  slot[1] sha256: ok — digest matches\n"),
        "{out}"
    );
    assert!(out.contains("  signature: ok — trusted, signer "), "{out}");
    assert!(out.contains("  slot[0] manifest: skip"), "{out}");
    assert!(out.contains("  slot[1] manifest: skip"), "{out}");
    // The entries cross-check (tebako#494): `probe` stats in slot 0's zip;
    // the name facet is unchecked (the plain image carries no L1 manifest).
    assert!(
        out.contains(
            "  entry[probe]: ok — path exists in slot 0; name unchecked (no usable L1 manifest)\n"
        ),
        "{out}"
    );
    assert!(out.contains("result: PASS\n"), "{out}");

    // info --verify is the same machinery with the same code.
    let (rc, out, _) = run(&["info", "--verify", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("result: PASS\n"), "{out}");

    // --verify --json carries checks.
    let (rc, out, _) = run(
        &["info", "--verify", "--json", pkg.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    let doc = tebako_pkg::json_parse(&out).unwrap();
    assert_eq!(doc.find("info_schema").and_then(|v| v.as_u64()), Some(1));
    let checks = doc.find("checks").unwrap();
    assert!(format!("{checks:?}").contains("slot[0] sha256"), "{out}");
    assert_eq!(
        doc.find("trust")
            .and_then(|t| t.find("outcome"))
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("trusted"),
        "{out}"
    );
}

#[test]
fn validate_tampered_slot_is_70() {
    let w = TempDir::new("p70");
    let home = test_home("p70");
    let z = w.0.join("z.zip");
    tebako_contract_tests::build_zip(
        &z,
        &["content/"],
        &[("content/a.txt", b"a"), ("probe", b"#!/bin/sh\n")],
    );
    let pm = package_manifest_file(&w);
    let pkg = w.0.join("pkg");
    bundle(
        &home,
        &w,
        &["--sign", "--package-manifest", pm.to_str().unwrap()],
        &[&z],
        &pkg,
    );

    // Flip a byte inside slot 0's image bytes (the trailer is untouched:
    // the signature still verifies; the digest does not).
    let mut bytes = std::fs::read(&pkg).unwrap();
    let slot0 = 2048usize;
    bytes[slot0 + 10] ^= 0xFF;
    std::fs::write(&pkg, &bytes).unwrap();

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}");
    assert!(
        out.contains("  slot[0] sha256: FAILED — digest mismatch"),
        "{out}"
    );
    assert!(out.contains("  signature: ok — trusted"), "{out}");
    assert!(out.contains("result: FAILED (exit 70)\n"), "{out}");
}

#[test]
fn validate_unsigned_require_signed_is_71() {
    let w = TempDir::new("p71");
    let home = test_home("p71");
    let z = w.0.join("z.zip");
    tebako_contract_tests::build_zip(
        &z,
        &["content/"],
        &[("content/a.txt", b"a"), ("probe", b"#!/bin/sh\n")],
    );
    let pm = package_manifest_file(&w);
    let pkg = w.0.join("pkg");
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&z],
        &pkg,
    );

    let (rc, out, _) = run(
        &["validate", "--require-signed", pkg.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 71, "{out}");
    assert!(
        out.contains("  signature: FAILED — unsigned package (--require-signed)\n"),
        "{out}"
    );
    assert!(out.contains("result: FAILED (exit 71)\n"), "{out}");

    // Without --require-signed the same package passes (unsigned is a
    // state, not a failure).
    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("  signature: skip — unsigned (v1 legacy trailer)\n"),
        "{out}"
    );
}

#[test]
fn validate_unknown_signer_is_72() {
    let w = TempDir::new("p72");
    let home_a = test_home("p72a");
    let z = w.0.join("z.zip");
    tebako_contract_tests::build_zip(
        &z,
        &["content/"],
        &[("content/a.txt", b"a"), ("probe", b"#!/bin/sh\n")],
    );
    let pm = package_manifest_file(&w);
    let pkg = w.0.join("pkg");
    bundle(
        &home_a,
        &w,
        &["--sign", "--package-manifest", pm.to_str().unwrap()],
        &[&z],
        &pkg,
    );

    // A keyring that never saw the signer.
    let home_b = test_home("p72b");
    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home_b);
    assert_eq!(rc, 72, "{out}");
    assert!(out.contains("not in the trusted keyring"), "{out}");
    assert!(out.contains("result: FAILED (exit 72)\n"), "{out}");
}

#[test]
fn validate_malformed_is_65() {
    let w = TempDir::new("p65");
    let home = test_home("p65");

    // Junk: no trailer at all.
    let junk = w.0.join("junk.bin");
    std::fs::write(&junk, patterned_bytes(512, 0x99)).unwrap();
    let (rc, out, _) = run(&["validate", junk.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 65, "{out}");
    assert!(out.contains("  trailer: FAILED"), "{out}");
    assert!(out.contains("result: FAILED (exit 65)\n"), "{out}");

    // Corrupt trailer (magic ok, crc bad).
    let z = w.0.join("z.zip");
    tebako_contract_tests::build_zip(
        &z,
        &["content/"],
        &[("content/a.txt", b"a"), ("probe", b"#!/bin/sh\n")],
    );
    let pkg = w.0.join("pkg");
    bundle(&home, &w, &[], &[&z], &pkg);
    let mut bytes = std::fs::read(&pkg).unwrap();
    let len = bytes.len();
    bytes[len - 100] ^= 0xFF; // inside the trailer header, before the crc
    std::fs::write(&pkg, &bytes).unwrap();
    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 65, "{out}");

    // Schema-invalid slot manifest is 65 (unsigned: digests/signature
    // skip, the manifest check decides). The era-2 block lets it pass the
    // contract stage to reach that check.
    let bad = mk_image(&w, "bad.tfs", Some("identity: [not: valid: yaml"));
    let pm = package_manifest_file(&w);
    let pkg2 = w.0.join("pkg2");
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&bad],
        &pkg2,
    );
    let (rc, out, _) = run(&["validate", pkg2.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 65, "{out}");
    assert!(out.contains("  slot[0] manifest: FAILED"), "{out}");
}

#[test]
fn validate_digest_agreement_is_70() {
    let w = TempDir::new("p70b");
    let home = test_home("p70b");
    // The fixture manifest's blob_sha256 is a placeholder: a manifest
    // embedded in the image it describes cannot name that image's digest,
    // so the agreement check reports the disagreement as 70.
    let app = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let pm = package_manifest_file(&w);
    let pkg = w.0.join("pkg");
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&app],
        &pkg,
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}");
    assert!(
        out.contains("  slot[0] manifest: ok — schema valid\n"),
        "{out}"
    );
    assert!(out.contains("  slot[0] digest agreement: FAILED"), "{out}");
    assert!(out.contains("result: FAILED (exit 70)\n"), "{out}");
}

// ---------------------------------------------------------------------
// validate: the L2 entries[] ↔ L1 entrypoints cross-check (tebako#494)
// ---------------------------------------------------------------------

/// Write `text` as the package-manifest file of a test.
fn pm_file(w: &TempDir, text: &str) -> PathBuf {
    let f = w.0.join(format!(
        "pm-{}.yaml",
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&f, text).unwrap();
    f
}

/// A valid L2 manifest whose two entries mirror the app-suite L1 (names
/// declared, paths present in the suite image).
const SUITE_PACKAGE_MANIFEST_YAML: &str = "schema_version: 1\n\
     package: {name: mn, version: 1.0.0, producer: {tool: tebako-pkg, tool_version: 0.1.0}, created: 2026-08-01T00:00:00Z}\n\
     entries:\n  - {name: metanorma, slot: 0, entrypoint: bin/metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n  - {name: metanorma-nokogiri, slot: 0, entrypoint: bin/metanorma-nokogiri, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n";

// The app-suite manifest's blob_sha256 can never name the image that
// embeds it (the check-5 note in verify.rs), so the suite packages below
// exit 70 on digest agreement — the entry checks are asserted by their
// report lines.

#[test]
fn validate_entries_crosscheck_name_and_path() {
    let w = TempDir::new("pxc");
    let home = test_home("pxc");
    let app = mk_suite_image(&w, "suite.tfs");
    let pm = pm_file(&w, SUITE_PACKAGE_MANIFEST_YAML);
    let pkg = w.0.join("mn");
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&app],
        &pkg,
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement (check 5), see above
    assert!(
        out.contains("  entry[metanorma]: ok — path exists in slot 0; name declared\n"),
        "{out}"
    );
    assert!(
        out.contains("  entry[metanorma-nokogiri]: ok — path exists in slot 0; name declared\n"),
        "{out}"
    );
}

#[test]
fn validate_entries_dangling_path_is_65() {
    let w = TempDir::new("pxd");
    let home = test_home("pxd");
    // No L1 manifest: the name facet skips; the path facet still decides.
    // PACKAGE_MANIFEST_YAML's entry `probe` has no `probe` file here.
    let app = mk_image(&w, "plain.tfs", None);
    let pm = package_manifest_file(&w);
    let pkg = w.0.join("pkg");
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&app],
        &pkg,
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 65, "{out}");
    assert!(
        out.contains(
            "  entry[probe]: FAILED — entrypoint path 'probe' does not exist in slot 0's image\n"
        ),
        "{out}"
    );
    assert!(out.contains("result: FAILED (exit 65)\n"), "{out}");
}

#[test]
fn validate_entries_dangling_name_fails() {
    let w = TempDir::new("pxn");
    let home = test_home("pxn");
    let app = mk_suite_image(&w, "suite.tfs");
    // `fontist`'s path exists (bin/metanorma is in the image) but no
    // declared entrypoint of the suite payload carries the name.
    let pm = pm_file(
        &w,
        "schema_version: 1\n\
         package: {name: mn, version: 1.0.0, producer: {tool: tebako-pkg, tool_version: 0.1.0}, created: 2026-08-01T00:00:00Z}\n\
         entries:\n  - {name: fontist, slot: 0, entrypoint: bin/metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n",
    );
    let pkg = w.0.join("mn");
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&app],
        &pkg,
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement precedes the entry check
    assert!(
        out.contains(
            "  entry[fontist]: FAILED — 'fontist' is not a declared entrypoint of slot 0's payload (declared: metanorma, metanorma-nokogiri)\n"
        ),
        "{out}"
    );
}

#[test]
fn press_refuses_an_entry_slot_beyond_the_container() {
    let w = TempDir::new("pxs");
    let home = test_home("pxs");
    let app = mk_suite_image(&w, "suite.tfs");
    // The PRESS refuses an entry slot beyond the container's slots with a
    // named error; the verify-side range check (verify.rs entry_checks) is
    // the backstop for hand-built containers that bypass the press.
    let pm = pm_file(
        &w,
        "schema_version: 1\n\
         package: {name: mn, version: 1.0.0, producer: {tool: tebako-pkg, tool_version: 0.1.0}, created: 2026-08-01T00:00:00Z}\n\
         entries:\n  - {name: ghost, slot: 3, entrypoint: bin/metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n",
    );
    let boot = bootstrap(&w);
    let pkg = w.0.join("mn");
    let (rc, _, err) = run(
        &[
            "bundle",
            "--bootstrap",
            boot.to_str().unwrap(),
            "--image",
            app.to_str().unwrap(),
            "-o",
            pkg.to_str().unwrap(),
            "--package-manifest",
            pm.to_str().unwrap(),
        ],
        &w.0,
        &home,
    );
    assert_eq!(rc, 1, "{err}");
    assert!(
        err.contains(
            "package manifest entry 0 (ghost) references slot 3 but the package has 1 slot(s)"
        ),
        "{err}"
    );
}

#[test]
fn validate_entries_shared_slice_skips() {
    let w = TempDir::new("pxl");
    let home = test_home("pxl");
    let app = mk_image(&w, "plain.tfs", None);
    // A slot-less entry (spec 23 §13 — the pointer-package form): the
    // slice is shared, so there is nothing local to cross-check.
    let pm = pm_file(
        &w,
        "schema_version: 1\n\
         package: {name: ptr, version: 1.0.0, producer: {tool: tebako-pkg, tool_version: 0.1.0}, created: 2026-08-01T00:00:00Z}\n\
         entries:\n  - {name: probe, entrypoint: bin/probe, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n\
         lock:\n  slices:\n    - {name: probe, version: 1.0.0, carry: false, sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef, source: \"tfs:github:acme/probe@1.0.0\"}\n",
    );
    let pkg = w.0.join("ptr");
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&app],
        &pkg,
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains(
            "  entry[probe]: skip — shared slice — resolved and checked at run time (spec 23 §13)\n"
        ),
        "{out}"
    );
    assert!(out.contains("result: PASS\n"), "{out}");
}

// ---------------------------------------------------------------------
// Format detection per slot (never trusting format_id alone)
// ---------------------------------------------------------------------

// squashfs detection mounts the slot — POSIX only (TODO.v2-1/02); the
// windows row renders the named "squashfs (undetected)" state instead.
#[cfg(not(windows))]
#[test]
fn format_detection_per_slot() {
    let w = TempDir::new("pfmt");
    let home = test_home("pfmt");
    let dwarfs = mk_image(&w, "app.tfs", Some(DATA_MANIFEST));
    let sqfs = fixture("simple.sqfs");
    let z = w.0.join("z.zip");
    tebako_contract_tests::build_zip(
        &z,
        &["content/"],
        &[("content/a.txt", b"a"), ("probe", b"#!/bin/sh\n")],
    );
    let t = w.0.join("t.tar");
    build_tar(&t, "hello.txt", b"hi from tar");
    let pkg = w.0.join("pkg");
    // The tar slot's format_id sniffs as `auto` — detection must answer.
    bundle(&home, &w, &[], &[&dwarfs, &sqfs, &z, &t], &pkg);

    let (rc, out, _) = run(&["info", "--full", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("    [0] ") && out.contains("format: dwarfs  "),
        "{out}"
    );
    assert!(
        out.contains("    [1] ") && out.contains("format: squashfs  "),
        "{out}"
    );
    assert!(
        out.contains("    [2] ") && out.contains("format: zip  "),
        "{out}"
    );
    assert!(
        out.contains("    [3] ") && out.contains("format: tar  "),
        "{out}"
    );
    assert!(
        out.contains(
            "     kind: data  liberation-fonts 2024.11  (suggested mount /usr/share/fonts)\n"
        ),
        "{out}"
    );

    // Depth 0 falls back to the format_id hints (tar → auto).
    let (rc, out, _) = run(
        &["info", "--full", "--depth", "0", pkg.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("    [3] ") && out.contains("format: auto  "),
        "{out}"
    );
}

// ---------------------------------------------------------------------
// LimniFS sniff (spec 20 §6): the bundle verb's magic sniff must stamp
// the limnifs hint — the format_name catch-all rendered such slots as
// "auto" before §6, and an auto hint on an LMFS image misinforms every
// hint-only reader (depth-0 info, legacy dump). Mount detection stays
// authoritative; the hint is a mirror.
// ---------------------------------------------------------------------

#[test]
fn limnifs_magic_sniffs_the_limnifs_hint() {
    let w = TempDir::new("plimsniff");
    let home = test_home("plimsniff");
    // A bare LMFS-magic file suffices: bundle is a container op (no
    // mount), so the sniff alone decides the slot's format hint.
    let img = w.0.join("payload.lim");
    let mut bytes = b"LMFS".to_vec();
    bytes.resize(512, 0);
    std::fs::write(&img, bytes).unwrap();
    let pkg = w.0.join("pkg");
    bundle(&home, &w, &[], &[&img], &pkg);

    let mut f = std::fs::File::open(&pkg).unwrap();
    let trailer = tpkg::read_from(&mut f).unwrap();
    assert_eq!(trailer.slots[0].format_id, tpkg::TPKG_FORMAT_LIMNIFS);

    // The legacy dump names the hint (was "auto" via the catch-all).
    let (rc, out, _) = run(&["info", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("format=limnifs"), "{out}");
}

// ---------------------------------------------------------------------
// The default (flag-less) output is untouched
// ---------------------------------------------------------------------

#[test]
fn default_output_is_the_legacy_trailer_dump() {
    let w = TempDir::new("pdef");
    let home = test_home("pdef");
    let app = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let pkg = w.0.join("pkg");
    bundle(&home, &w, &[], &[&app], &pkg);

    let (rc, out, _) = run(&["info", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("Format: tebako three-part package (tpkg v1)"),
        "{out}"
    );
    assert!(out.contains("Trailer: valid (magic and crc32 ok)"), "{out}");
    assert!(!out.contains("kind: app"), "{out}");
}

// ---------------------------------------------------------------------
// validate: the L2 lock.spawned[] ↔ L1 requires[].kind: runtime
// cross-check (spec 30 §2, spec 23 §13.6)
// ---------------------------------------------------------------------

/// The L1 shape shared by the spawned cross-check tests: the java edge
/// plus the `probe` entrypoint the L2 entries mirror. Its digest block
/// can never name the image that embeds it (the check-5 note in
/// verify.rs), so these packages exit 70 on digest agreement — the
/// spawned checks are asserted by their report lines (the suite tests'
/// rule).
const SPAWNED_APP_MANIFEST: &str = "identity:\n\
     \x20 schema_version: 1\n\
     \x20 kind: app\n\
     \x20 name: mn\n\
     \x20 version: 1.0.0\n\
     \x20 producer: {tool: tebako-pkg, tool_version: 0.1.0}\n\
     \x20 created: \"2026-08-01T00:00:00Z\"\n\
     \x20 source:\n\
     \x20   commit: 4f3c2b1a9d8e7f605a4b3c2d1e0f9a8b7c6d5e4f\n\
     \x20   builder: gha:run:20260721-1042\n\
     \x20 sbom: {ref: sbom/mn-1.0.0.spdx.json}\n\
     \x20 digest:\n\
     \x20   tree_hash: sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\n\
     \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
     \x20 signing: {state: unsigned}\n\
     \x20 encryption: {state: none}\n\
     provides:\n\
     \x20 entrypoints:\n\
     \x20   - name: probe\n\
     \x20     path: /probe\n\
     \x20     runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n\
     \x20 platforms: [aarch64-macos, x86_64-linux-gnu]\n\
     \x20 capabilities: {exec: true, read: true}\n\
     requires:\n\
     \x20 - kind: runtime\n\
     \x20   engine: java\n\
     \x20   constraint: \">= 21, < 26\"\n\
     \x20   expose: [java]\n";

/// The L2 shape shared by the spawned cross-check tests: the entries
/// row plus whatever lock the test names.
fn spawned_pm(lock_yaml: &str) -> String {
    format!(
        "schema_version: 1\n\
         package: {{name: mn, version: 1.0.0, producer: {{tool: tebako-pkg, tool_version: 0.1.0}}, created: 2026-08-01T00:00:00Z}}\n\
         entries:\n  - {{name: probe, slot: 0, entrypoint: probe, runtime_ref: ruby@3.4.2;tebako=0.15.9}}\n\
         {lock_yaml}"
    )
}

/// The lock block mirroring the L1 java edge (a shared row: no slots,
/// the press-resolved coordinates + pins).
const SPAWNED_LOCK_YAML: &str = "lock:\n\
     \x20 spawned:\n\
     \x20   - engine: java\n\
     \x20     constraint: \">= 21, < 26\"\n\
     \x20     expose: [java]\n\
     \x20     version: \"21.0.12\"\n\
     \x20     tebako: \"2.1.5\"\n\
     \x20     carry: false\n\
     \x20     exe: {sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n\
     \x20     image: {sha256: \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}\n\
     \x20     source: \"https://github.com/tebako-packages/openjdk/releases/download\"\n";

fn bundle_spawned_app(w: &TempDir, home: &Path, pm_yaml: &str) -> PathBuf {
    let app = mk_image_files(
        w,
        "spawned.tfs",
        Some(SPAWNED_APP_MANIFEST),
        &[("probe", b"#!/bin/sh\n")],
    );
    let pm = pm_file(w, pm_yaml);
    let pkg =
        w.0.join(format!("pkg-{}", COUNTER.fetch_add(1, Ordering::Relaxed)));
    bundle(
        home,
        w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&app],
        &pkg,
    );
    pkg
}

#[test]
fn validate_spawned_mirror_passes_when_the_lock_mirrors_the_edge() {
    let w = TempDir::new("psp-ok");
    let home = test_home("psp-ok");
    let pkg = bundle_spawned_app(&w, &home, &spawned_pm(SPAWNED_LOCK_YAML));

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement (check 5), see above
    assert!(
        out.contains(
            "  spawned[java]: ok — mirrors the L1 edge; the locked version 21.0.12 satisfies \">= 21, < 26\"\n"
        ),
        "{out}"
    );
}

#[test]
fn validate_spawned_unmirrored_edge_is_65() {
    let w = TempDir::new("psp-miss");
    let home = test_home("psp-miss");
    // The lock without the spawned row: the L1 java edge would never
    // resolve on the standalone path — named, fail-closed.
    let pkg = bundle_spawned_app(&w, &home, &spawned_pm(""));

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement masks the 65, see above
    assert!(
        out.contains(
            "  spawned[java]: FAILED — the app payload's L1 manifest declares this spawned-runtime edge but the lock carries no row"
        ),
        "{out}"
    );
}

#[test]
fn validate_spawned_mirror_mismatches_are_65() {
    let w = TempDir::new("psp-bad");
    let home = test_home("psp-bad");
    // The row's locked version does not satisfy the mirrored constraint.
    let pkg = bundle_spawned_app(
        &w,
        &home,
        &spawned_pm(&SPAWNED_LOCK_YAML.replace("version: \"21.0.12\"", "version: \"20.0.2\"")),
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement masks the 65, see above
    assert!(
        out.contains(
            "  spawned[java]: FAILED — the locked version 20.0.2 does not satisfy the mirrored constraint \">= 21, < 26\""
        ),
        "{out}"
    );
}

/// The CARRIED spawned-runtime layout (packed-mn's 4-slot shape, spec 30
/// §1/§2): slot 1 is the RAW wrapper exe — never an image. validate must
/// skip its manifest check (the packed-mn#254 gate failure: slot[1]
/// manifest FAILED with "cannot mount the image (errno 22)").
#[test]
fn validate_spawned_carried_raw_exe_slot_skips_the_manifest_check() {
    use sha2::{Digest, Sha256};
    let sha256_hex = |p: &Path| format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()));

    let w = TempDir::new("psp-raw");
    let home = test_home("psp-raw");
    let app = mk_image_files(
        &w,
        "spawned.tfs",
        Some(SPAWNED_APP_MANIFEST),
        &[("probe", b"#!/bin/sh\n")],
    );
    let exe = w.0.join("java-exe");
    std::fs::write(&exe, b"\x7fELF raw wrapper bytes - never an image").unwrap();
    let env = mk_image(&w, "java-env.tfs", None);

    let pm_yaml = spawned_pm(&format!(
        "lock:\n  spawned:\n   - engine: java\n     constraint: \">= 21, < 26\"\n     expose: [java]\n     version: \"21.0.12\"\n     tebako: \"2.1.6\"\n     carry: true\n     exe: {{slot: 1, sha256: \"{}\"}}\n     image: {{slot: 2, sha256: \"{}\"}}\n",
        sha256_hex(&exe),
        sha256_hex(&env)
    ));
    let pm = pm_file(&w, &pm_yaml);
    let pkg =
        w.0.join(format!("pkg-{}", COUNTER.fetch_add(1, Ordering::Relaxed)));
    bundle(
        &home,
        &w,
        &["--package-manifest", pm.to_str().unwrap(), "--exact-mounts"],
        &[&app, &exe, &env],
        &pkg,
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // slot 0 digest agreement, see above
    assert!(
        out.contains(
            "  slot[1] manifest: skip — spawned runtime artifact (raw bytes — never mounted; the trailer digest and the lock pin carry its identity)\n"
        ),
        "{out}"
    );
    assert!(!out.contains("slot[1] manifest: FAILED"), "{out}");
    // The lock row still mirrors the L1 edge — the cross-check stands.
    assert!(
        out.contains(
            "  spawned[java]: ok — mirrors the L1 edge; the locked version 21.0.12 satisfies \">= 21, < 26\"\n"
        ),
        "{out}"
    );
}

// ---------------------------------------------------------------------
// validate: the L2 lock.spawned[] payload rows ↔ L1
// requires[].kind: executable cross-check (spec 32 §6, spec 23 §13.6
// amended)
// ---------------------------------------------------------------------

/// The spec-32 L1 shape: the `probe` entrypoint plus the expose-carrying
/// xml2rfc executable edge (SPAWNED_APP_MANIFEST with the edge swapped).
/// The same digest-agreement caveat applies — the spawned checks are
/// asserted by their report lines.
fn spawned32_app_manifest() -> String {
    let swapped = SPAWNED_APP_MANIFEST.replace(
        "  - kind: runtime\n    engine: java\n    constraint: \">= 21, < 26\"\n    expose: [java]\n",
        "  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n",
    );
    assert!(
        swapped.contains("kind: executable"),
        "the edge swap applied (replace is silent on a no-match)"
    );
    swapped
}

/// The lock block mirroring the L1 xml2rfc edge (a SHARED payload row:
/// no slots, the press-resolved coordinates + pins; the nested runtime
/// row is the provider's own language edge, resolved — spec 32 §6).
const SPAWNED32_LOCK_YAML: &str = "lock:\n\
     \x20 spawned:\n\
     \x20   - payload: xml2rfc\n\
     \x20     constraint: \">= 3.0\"\n\
     \x20     expose: [xml2rfc]\n\
     \x20     version: \"3.2.1\"\n\
     \x20     carry: false\n\
     \x20     image: {sha256: \"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\"}\n\
     \x20     runtime:\n\
     \x20       engine: ruby\n\
     \x20       constraint: \">= 3.3, < 5.0\"\n\
     \x20       version: \"3.4.2\"\n\
     \x20       tebako: \"2.1.6\"\n\
     \x20       carry: false\n\
     \x20       exe: {sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n\
     \x20       image: {sha256: \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}\n\
     \x20       source: \"https://github.com/tamatebako/tebako-runtime-ruby/releases/download\"\n\
     \x20     source: \"tfs:github:tebako-packages/xml2rfc\"\n";

fn bundle_spawned32_app(w: &TempDir, home: &Path, pm_yaml: &str) -> PathBuf {
    let manifest = spawned32_app_manifest();
    let app = mk_image_files(
        w,
        "spawned32.tfs",
        Some(&manifest),
        &[("probe", b"#!/bin/sh\n")],
    );
    let pm = pm_file(w, pm_yaml);
    let pkg =
        w.0.join(format!("pkg-{}", COUNTER.fetch_add(1, Ordering::Relaxed)));
    bundle(
        home,
        w,
        &["--package-manifest", pm.to_str().unwrap()],
        &[&app],
        &pkg,
    );
    pkg
}

#[test]
fn validate_spawned32_mirror_passes_when_the_lock_mirrors_the_edge() {
    let w = TempDir::new("p32-ok");
    let home = test_home("p32-ok");
    let pkg = bundle_spawned32_app(&w, &home, &spawned_pm(SPAWNED32_LOCK_YAML));

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement (check 5), see above
    assert!(
        out.contains(
            "  spawned[xml2rfc]: ok — mirrors the L1 executable edge; the locked provider version 3.2.1 satisfies \">= 3.0\"\n"
        ),
        "{out}"
    );
    // the shared row's provider image is not inspectable at verify — the
    // nested runtime mirror is skip-loud, never silent
    assert!(
        out.contains(
            "  spawned[xml2rfc].runtime: skip — the provider image is not inspectable here — the nested runtime mirror is unchecked (the locked version 3.4.2 satisfies the nested constraint \">= 3.3, < 5.0\")\n"
        ),
        "{out}"
    );
}

#[test]
fn validate_spawned32_unmirrored_edge_is_65() {
    let w = TempDir::new("p32-miss");
    let home = test_home("p32-miss");
    // The lock without the payload row: the L1 xml2rfc edge would never
    // resolve on the standalone path — named, fail-closed.
    let pkg = bundle_spawned32_app(&w, &home, &spawned_pm(""));

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement masks the 65, see above
    assert!(
        out.contains(
            "  spawned[xml2rfc]: FAILED — the app payload's L1 manifest declares this expose-carrying `kind: executable` edge but the lock carries no payload row"
        ),
        "{out}"
    );
}

#[test]
fn validate_spawned32_mirror_mismatches_are_65() {
    let w = TempDir::new("p32-bad");
    let home = test_home("p32-bad");
    // The row's locked provider version does not satisfy the mirrored
    // constraint.
    let pkg = bundle_spawned32_app(
        &w,
        &home,
        &spawned_pm(&SPAWNED32_LOCK_YAML.replace("version: \"3.2.1\"", "version: \"2.9.0\"")),
    );

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // digest agreement masks the 65, see above
    assert!(
        out.contains(
            "  spawned[xml2rfc]: FAILED — the locked provider version 2.9.0 does not satisfy the mirrored constraint \">= 3.0\""
        ),
        "{out}"
    );
}

/// The CARRIED spawned-payload layout (spec 32 §6): slot 1 is the
/// provider image (a REAL image — the normal slot-manifest check
/// applies), slots 2/3 the nested runtime pair (the exe RAW — never an
/// image). The nested runtime row cross-checks against the provider
/// image's own L1 language edge.
fn bundle_spawned32_carried(w: &TempDir, home: &Path, nested_constraint: &str) -> PathBuf {
    use sha2::{Digest, Sha256};
    let sha256_hex = |p: &Path| format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()));

    let manifest = spawned32_app_manifest();
    let app = mk_image_files(
        w,
        "spawned32.tfs",
        Some(&manifest),
        &[("probe", b"#!/bin/sh\n")],
    );
    let provider_manifest = format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: xml2rfc\n  version: 3.2.1\n  producer: {{tool: tebako-pkg, tool_version: 0.1.0}}\n  created: \"2026-08-01T00:00:00Z\"\n  digest:\n    tree_hash: sha256:{}\n    blob_sha256: {}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: xml2rfc\n      path: /app/bin/xml2rfc\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: [aarch64-macos, x86_64-linux-gnu]\n  capabilities: {{exec: true, read: true}}\nrequires:\n  - kind: language\n    engine: ruby\n    constraint: \">= 3.3, < 5.0\"\n",
        "d".repeat(64),
        "e".repeat(64)
    );
    let provider = mk_image_files(
        w,
        "xml2rfc.tfs",
        Some(&provider_manifest),
        &[("app/bin/xml2rfc", b"#!/bin/sh\n")],
    );
    let exe = w.0.join("ruby-exe");
    std::fs::write(&exe, b"\x7fELF raw wrapper bytes - never an image").unwrap();
    let env = mk_image(w, "ruby-env.tfs", None);

    let pm_yaml = spawned_pm(&format!(
        "lock:\n  spawned:\n   - payload: xml2rfc\n     constraint: \">= 3.0\"\n     expose: [xml2rfc]\n     version: \"3.2.1\"\n     carry: true\n     image: {{slot: 1, sha256: \"{}\"}}\n     runtime:\n       engine: ruby\n       constraint: \"{nested_constraint}\"\n       version: \"3.4.2\"\n       tebako: \"2.1.6\"\n       carry: true\n       exe: {{slot: 2, sha256: \"{}\"}}\n       image: {{slot: 3, sha256: \"{}\"}}\n",
        sha256_hex(&provider),
        sha256_hex(&exe),
        sha256_hex(&env)
    ));
    let pm = pm_file(w, &pm_yaml);
    let pkg =
        w.0.join(format!("pkg-{}", COUNTER.fetch_add(1, Ordering::Relaxed)));
    bundle(
        home,
        w,
        &["--package-manifest", pm.to_str().unwrap(), "--exact-mounts"],
        &[&app, &provider, &exe, &env],
        &pkg,
    );
    pkg
}

#[test]
fn validate_spawned32_carried_provider_image_cross_checks_the_nested_runtime() {
    let w = TempDir::new("p32-car");
    let home = test_home("p32-car");
    let pkg = bundle_spawned32_carried(&w, &home, ">= 3.3, < 5.0");

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // slot digest agreement, see above
    assert!(
        out.contains(
            "  spawned[xml2rfc]: ok — mirrors the L1 executable edge; the locked provider version 3.2.1 satisfies \">= 3.0\"\n"
        ),
        "{out}"
    );
    // the nested row mirrors the PROVIDER's L1 language edge (the
    // two-level cross-check, spec 32 §6)
    assert!(
        out.contains(
            "  spawned[xml2rfc].runtime: ok — mirrors the provider's L1 language edge; the locked runtime 3.4.2 (2.1.6) satisfies \">= 3.3, < 5.0\"\n"
        ),
        "{out}"
    );
    // the nested runtime's RAW exe slot skips the manifest check
    assert!(
        out.contains(
            "  slot[2] manifest: skip — spawned runtime artifact (raw bytes — never mounted; the trailer digest and the lock pin carry its identity)\n"
        ),
        "{out}"
    );
}

#[test]
fn validate_spawned32_nested_constraint_mirror_mismatch_is_65() {
    let w = TempDir::new("p32-nest");
    let home = test_home("p32-nest");
    // The nested runtime constraint mirrors nothing the provider
    // declares (3.4.2 satisfies it — the MIRROR is the failure).
    let pkg = bundle_spawned32_carried(&w, &home, ">= 3.2");

    let (rc, out, _) = run(&["validate", pkg.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}"); // slot digest agreement, see above
    assert!(
        out.contains(
            "  spawned[xml2rfc].runtime: FAILED — the nested runtime constraint mirror differs — the provider's L1 declares \">= 3.3, < 5.0\", the lock carries \">= 3.2\""
        ),
        "{out}"
    );
}
