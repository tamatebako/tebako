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
    let src = w.0.join(format!("src-{name}"));
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("hello.txt"), b"hi").unwrap();
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

// squashfs slots mount on POSIX only — the windows tfs is dwarfs-only,
// where the probe renders the named mount-failure state (TODO.v2-1/02).
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
    let app = mk_image(&w, "app.tfs", None);
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
    // windows tfs is dwarfs-only, the sqfs slot reports the named mount
    // failure there (TODO.v2-1/02).
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
    tebako_contract_tests::build_zip(&z, &["content/"], &[("content/a.txt", b"a")]);
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
    tebako_contract_tests::build_zip(&z, &["content/"], &[("content/a.txt", b"a")]);
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
    tebako_contract_tests::build_zip(&z, &["content/"], &[("content/a.txt", b"a")]);
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
    tebako_contract_tests::build_zip(&z, &["content/"], &[("content/a.txt", b"a")]);
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
    tebako_contract_tests::build_zip(&z, &["content/"], &[("content/a.txt", b"a")]);
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
    tebako_contract_tests::build_zip(&z, &["content/"], &[("content/a.txt", b"a")]);
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
