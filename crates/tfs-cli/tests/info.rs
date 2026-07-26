//! The spec-15 `tfs info` surface: every additive flag's output shape,
//! the JSON contract (`info_schema: 1`), `--verify` exit codes per failure
//! class, the cache-entry (directory) form and format detection per
//! backend. The default output is the legacy parity summary (covered by
//! golden.rs/cli.rs and asserted unchanged here).
//!
//! Fixtures: images are built in-test via `tfs mkimage` (dwarfs-t
//! FlatBuffers), the zip helper of tebako-contract-tests, a hand-rolled
//! ustar tar and the vendored SquashFS fixture; manifests come from
//! crates/tpkg/tests/fixtures/manifests (each kind).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tebako_contract_tests::TempDir;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tfs");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tfs binary not built")
}

/// An isolated TEBAKO_HOME per test (the derived runtime-compat fact and
/// the trusted keyring are read from it — never from the machine cache).
fn test_home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tfs-info-test-home-{tag}-{}-{}",
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
const RUNTIME_MANIFEST: &str = include_str!("../../tpkg/tests/fixtures/manifests/runtime.yaml");
const DATA_MANIFEST: &str = include_str!("../../tpkg/tests/fixtures/manifests/data.yaml");

/// A dwarfs-t image carrying `manifest` at /__tpkg__/manifest.yaml (or a
/// plain image when None) plus one payload file.
fn mk_image(w: &TempDir, name: &str, manifest: Option<&str>) -> PathBuf {
    let src = w.0.join(format!("src-{name}"));
    std::fs::create_dir_all(src.join("__tpkg__")).unwrap();
    std::fs::write(src.join("hello.txt"), b"hi").unwrap();
    if let Some(text) = manifest {
        std::fs::write(src.join("__tpkg__/manifest.yaml"), text).unwrap();
    }
    let img = w.0.join(name);
    let home = test_home("mk");
    let (rc, _, err) = run(
        &[
            "mkimage",
            "--format",
            "dwarfs",
            src.to_str().unwrap(),
            "-o",
            img.to_str().unwrap(),
        ],
        &w.0,
        &home,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed");
    img
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}

/// A minimal ustar tar with one file (the detection chain's weak
/// heuristic runs last — a real header exercises it).
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

// ---------------------------------------------------------------------
// Section flags (human output shapes)
// ---------------------------------------------------------------------

#[test]
fn manifest_view_full_shape() {
    let w = TempDir::new("iview");
    let home = test_home("iview");
    // A cached runtime the derived block can satisfy the range with.
    std::fs::create_dir_all(home.join("runtimes/ruby-3.4.2-0.15.9-macos-arm64")).unwrap();
    let img = mk_image(&w, "app.tfs", Some(APP_MANIFEST));

    let (rc, out, err) = run(
        &[
            "info",
            "--manifest",
            "--provides",
            "--requires",
            "--platforms",
            img.to_str().unwrap(),
        ],
        &w.0,
        &home,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "{out}");
    // Header.
    assert!(out.contains("image: "), "{out}");
    assert!(
        out.contains("  format: dwarfs-t (flatbuffers metadata)  ro  "),
        "{out}"
    );
    assert!(
        out.contains("  kind: app  name: metanorma  version: 1.2.3\n"),
        "{out}"
    );
    assert!(
        out.contains(
            "  platforms: aarch64-macos (macos-arm64), x86_64-linux-gnu (linux-gnu-x86_64)\n"
        ),
        "{out}"
    );
    assert!(
        out.contains("  digests: blob_sha256 7a5eb444…  tree_hash sha256:650f8ad9…\n"),
        "{out}"
    );
    assert!(out.contains("  signing: unsigned\n"), "{out}");
    assert!(out.contains("  encryption: none\n"), "{out}");
    // PROVIDES (kind-specialized).
    assert!(
        out.contains(
            "    entrypoint metanorma → /__app__/bin/metanorma  args: --format pretty  runtime: ruby >= 3.3, < 5.0\n"
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "    entrypoint metanorma-nokogiri → /__app__/bin/metanorma-nokogiri  runtime: ruby ~> 3.3.0\n"
        ),
        "{out}"
    );
    // REQUIRES edges as kind:name:constraint → mount.
    assert!(out.contains("    language:ruby:~> 3.3.0\n"), "{out}");
    assert!(
        out.contains(
            "    toolkit:gtk-layer:>= 3.24, < 3.25 → /__layers__/gtk (triplets: aarch64-macos, x86_64-linux-gnu)\n"
        ),
        "{out}"
    );
    // DERIVED.
    assert!(
        out.contains("    shims: metanorma, metanorma-nokogiri\n"),
        "{out}"
    );
    // ">= 3.3, < 5.0" is satisfied by the cached 3.4.2…
    assert!(
        out.contains("    runtime: satisfied-by ruby-3.4.2-0.15.9-macos-arm64 (cached)\n"),
        "{out}"
    );
    // …while "~> 3.3.0" is not (3.4.2 falls outside the abi line).
    assert!(
        out.contains(
            "    runtime: requires-download: ruby ~> 3.3.0 (no compatible runtime cached)\n"
        ),
        "{out}"
    );
    assert!(out.contains("    dependencies: ruby, gtk-layer\n"), "{out}");
    // --manifest: the parsed model re-serialized as YAML.
    assert!(out.contains("  manifest:\n"), "{out}");
    assert!(out.contains("      schema_version: 1\n"), "{out}");
    assert!(out.contains("        tool: tebako-cli\n"), "{out}");
    assert!(
        out.contains("        metanorma.org/flavor: full\n"),
        "{out}"
    );
}

#[test]
fn platforms_only_and_requires_only() {
    let w = TempDir::new("iflags");
    let home = test_home("iflags");
    let img = mk_image(&w, "app.tfs", Some(APP_MANIFEST));

    let (rc, out, _) = run(&["info", "--platforms", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("platforms: aarch64-macos (macos-arm64)"),
        "{out}"
    );
    assert!(!out.contains("entrypoint"), "{out}");

    let (rc, out, _) = run(&["info", "--requires", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("toolkit:gtk-layer"), "{out}");
    assert!(!out.contains("entrypoint"), "{out}");
    assert!(!out.contains("platforms:"), "{out}");
}

#[test]
fn runtime_and_data_kind_sections() {
    let w = TempDir::new("ikinds");
    let home = test_home("ikinds");
    let rt = mk_image(&w, "runtime.tfs", Some(RUNTIME_MANIFEST));
    let data = mk_image(&w, "data.tfs", Some(DATA_MANIFEST));

    let (rc, out, _) = run(&["info", "--provides", rt.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("  kind: runtime  name: tebako-runtime-ruby  version: 4.0.6\n"),
        "{out}"
    );
    assert!(
        out.contains("    provides ruby 4.0.6 (abi 4.0) aarch64-macos (macos-arm64)\n"),
        "{out}"
    );
    assert!(out.contains("    env GEM_HOME=/__tebako__/gems\n"), "{out}");
    assert!(
        out.contains("  signing: signed (keyid 0123456789abcdef)\n"),
        "{out}"
    );

    let (rc, out, _) = run(&["info", "--provides", data.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("  kind: data  name: liberation-fonts  version: 2024.11\n"),
        "{out}"
    );
    assert!(
        out.contains("    mount_semantics: suggested /usr/share/fonts\n"),
        "{out}"
    );
    assert!(out.contains("    consumers: metanorma\n"), "{out}");
    assert!(
        out.contains("  encryption: encrypted (1 part(s))\n"),
        "{out}"
    );
}

#[test]
fn plain_image_manifest_absent_is_a_named_note() {
    let w = TempDir::new("inote");
    let home = test_home("inote");
    let img = mk_image(&w, "plain.tfs", None);

    let (rc, out, _) = run(&["info", "--manifest", img.to_str().unwrap()], &w.0, &home);
    assert_eq!((rc, out.contains("Error")), (0, false), "{out}");
    assert!(
        out.contains("  manifest: none (no /__tpkg__/manifest.yaml — plain image)\n"),
        "{out}"
    );
}

#[test]
fn default_output_is_the_legacy_summary() {
    let w = TempDir::new("idef");
    let home = test_home("idef");
    let img = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let (rc, out, _) = run(&["info", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("Type: DwarFS"), "{out}");
    assert!(out.contains("Files: "), "{out}");
    assert!(!out.contains("kind: app"), "{out}");
}

// ---------------------------------------------------------------------
// Format detection per backend
// ---------------------------------------------------------------------

#[test]
fn format_detection_per_backend() {
    let w = TempDir::new("iformats");
    let home = test_home("iformats");

    // dwarfs-t (flatbuffers).
    let img = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let (rc, out, _) = run(&["info", "--provides", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(
        out.contains("format: dwarfs-t (flatbuffers metadata)"),
        "{out}"
    );

    // squashfs (vendored fixture; no manifest → named note).
    let (rc, out, _) = run(
        &[
            "info",
            "--manifest",
            fixture("simple.sqfs").to_str().unwrap(),
        ],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("  format: squashfs  ro  "), "{out}");
    assert!(out.contains("manifest: none"), "{out}");

    // zip with a manifest inside.
    let z = w.0.join("z.zip");
    tebako_contract_tests::build_zip(
        &z,
        &["__tpkg__/"],
        &[("__tpkg__/manifest.yaml", DATA_MANIFEST.as_bytes())],
    );
    let (rc, out, _) = run(&["info", "--provides", z.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("  format: zip  ro  "), "{out}");
    assert!(out.contains("kind: data  name: liberation-fonts"), "{out}");

    // tar (weak heuristic, last in the chain).
    let t = w.0.join("t.tar");
    build_tar(&t, "hello.txt", b"hi from tar");
    let (rc, out, _) = run(&["info", "--manifest", t.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("  format: tar  ro  "), "{out}");
    assert!(out.contains("manifest: none"), "{out}");

    // tar with a manifest.
    let t2 = w.0.join("t2.tar");
    build_tar(&t2, "__tpkg__/manifest.yaml", DATA_MANIFEST.as_bytes());
    let (rc, out, _) = run(&["info", "--provides", t2.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("  format: tar  ro  "), "{out}");
    assert!(out.contains("kind: data  name: liberation-fonts"), "{out}");
}

// ---------------------------------------------------------------------
// JSON contract
// ---------------------------------------------------------------------

#[test]
fn json_document_contract() {
    let w = TempDir::new("ijson");
    let home = test_home("ijson");
    std::fs::create_dir_all(home.join("runtimes/ruby-3.4.2-0.15.9-macos-arm64")).unwrap();
    let img = mk_image(&w, "app.tfs", Some(APP_MANIFEST));

    let (rc, out, _) = run(&["info", "--json", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    let doc = tebako_json::parse(&out).expect("output must parse as JSON");
    assert_eq!(
        doc.find("info_schema").and_then(|v| v.as_u64()),
        Some(1),
        "{out}"
    );
    let artifact = doc.find("artifact").expect("artifact");
    assert_eq!(
        artifact.find("kind").and_then(|v| v.as_string()).as_deref(),
        Some("image")
    );
    let format = doc.find("format").expect("format");
    assert_eq!(
        format.find("label").and_then(|v| v.as_string()).as_deref(),
        Some("dwarfs-t (flatbuffers metadata)")
    );
    let manifest = doc.find("manifest").expect("manifest");
    assert_eq!(
        manifest
            .find("identity")
            .and_then(|i| i.find("name"))
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("metanorma")
    );
    // spec 03 mapped 1:1 (YAML field names carried across).
    assert!(
        manifest
            .find("provides")
            .and_then(|p| p.find("entrypoints"))
            .is_some(),
        "{out}"
    );
    let derived = doc.find("derived").expect("derived");
    let tebako_json::Value::Array(shims) = derived.find("shims").unwrap() else {
        panic!("shims must be an array");
    };
    assert_eq!(shims.len(), 2);
    let compat = derived.find("runtime_compat").unwrap();
    assert!(format!("{compat:?}").contains("satisfied-by"), "{compat:?}");
    assert!(
        format!("{:?}", derived.find("dependency_names").unwrap()).contains("gtk-layer"),
        "{derived:?}"
    );

    // --json + --backend-json folds the backend metadata in.
    let (rc, out, _) = run(
        &["info", "--json", "--backend-json", img.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    let doc = tebako_json::parse(&out).unwrap();
    let backend = doc.find("backend").expect("backend member");
    assert!(backend.find("block_size").is_some(), "{out}");

    // A plain image's JSON carries the named note, not an error.
    let plain = mk_image(&w, "plain.tfs", None);
    let (rc, out, _) = run(&["info", "--json", plain.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    let doc = tebako_json::parse(&out).unwrap();
    assert!(doc.find("manifest").is_none(), "{out}");
    assert!(
        doc.find("manifest_note")
            .and_then(|v| v.as_string())
            .unwrap()
            .contains("plain image"),
        "{out}"
    );
}

// ---------------------------------------------------------------------
// --verify (spec 15 §5 exit codes)
// ---------------------------------------------------------------------

#[test]
fn verify_plain_image_passes_with_a_note() {
    let w = TempDir::new("iplain");
    let home = test_home("iplain");
    let img = mk_image(&w, "plain.tfs", None);
    let (rc, out, _) = run(&["info", "--verify", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("  image: ok"), "{out}");
    assert!(out.contains("  manifest: skip"), "{out}");
    assert!(out.contains("result: PASS\n"), "{out}");
}

#[test]
fn verify_malformed_manifest_is_65() {
    let w = TempDir::new("i65");
    let home = test_home("i65");
    let img = mk_image(&w, "bad.tfs", Some("identity: [not: valid: yaml"));
    let (rc, out, _) = run(&["info", "--verify", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 65, "{out}");
    assert!(out.contains("  manifest: FAILED"), "{out}");
    assert!(out.contains("result: FAILED (exit 65)\n"), "{out}");

    // Schema-invalid (parses, but violates the locked rules) is 65 too.
    let invalid = APP_MANIFEST.replace("name: metanorma\n", "name: ''\n");
    let img2 = mk_image(&w, "bad2.tfs", Some(&invalid));
    let (rc, out, _) = run(&["info", "--verify", img2.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 65, "{out}");
    assert!(out.contains("identity.name must not be empty"), "{out}");
}

#[test]
fn verify_digest_agreement_is_70() {
    let w = TempDir::new("i70");
    let home = test_home("i70");
    // The fixture manifest's blob_sha256 is a placeholder, so it can never
    // match the actual image bytes (a manifest embedded in the image it
    // describes cannot name that image's digest) — the check reports the
    // disagreement as exit 70.
    let img = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    let (rc, out, _) = run(&["info", "--verify", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 70, "{out}");
    assert!(out.contains("  manifest: ok"), "{out}");
    assert!(out.contains("  digest agreement: FAILED"), "{out}");
    assert!(out.contains("result: FAILED (exit 70)\n"), "{out}");
}

#[test]
fn verify_declared_signing_without_a_block_is_71() {
    let w = TempDir::new("i71");
    let home = test_home("i71");
    // runtime.yaml declares signing (keyid 0123456789abcdef) but no
    // <image>.asc sidecar exists.
    let img = mk_image(&w, "runtime.tfs", Some(RUNTIME_MANIFEST));
    let (rc, out, _) = run(&["info", "--verify", img.to_str().unwrap()], &w.0, &home);
    assert_eq!(rc, 71, "{out}");
    assert!(out.contains("  signature: FAILED"), "{out}");
    assert!(out.contains("manifest declares signing"), "{out}");
}

#[test]
fn verify_unsigned_under_require_signed_is_71() {
    let w = TempDir::new("i71b");
    let home = test_home("i71b");
    let img = mk_image(&w, "plain.tfs", None);
    let (rc, out, _) = run(
        &[
            "info",
            "--verify",
            "--require-signed",
            img.to_str().unwrap(),
        ],
        &w.0,
        &home,
    );
    assert_eq!(rc, 71, "{out}");
    assert!(out.contains("unsigned image (--require-signed)"), "{out}");
}

#[test]
fn verify_signature_outcomes_trusted_and_unknown_signer() {
    let w = TempDir::new("isig");
    let home = test_home("isig");
    // A manifest declaring signed with the PRESS key's keyid, so a trusted
    // signature passes the keyid cross-check.
    let press = tebako_signer::press_local_key(&home).expect("press key");
    tebako_signer::register_trusted(&home, &press.public_key).unwrap();
    let manifest = RUNTIME_MANIFEST.replace("0123456789abcdef", &press.keyid_hex());
    let img = mk_image(&w, "runtime.tfs", Some(&manifest));

    // Detached sidecar over the image bytes (the release .asc shape).
    let data = std::fs::read(&img).unwrap();
    let sig = tebako_signer::sign_detached(&data, &press.secret_key, &press.fingerprint).unwrap();
    let armored = rnp::armor_bytes(&sig, rnp::ops::ArmorType::Signature).unwrap();
    std::fs::write(w.0.join("runtime.tfs.asc"), armored).unwrap();

    let (rc, out, _) = run(&["info", "--verify", img.to_str().unwrap()], &w.0, &home);
    // The signature itself verifies (trusted); the declared blob_sha256
    // placeholder cannot match the image → 70 from digest agreement.
    assert_eq!(rc, 70, "{out}");
    assert!(out.contains("  signature: ok — trusted"), "{out}");

    // The same image against an empty keyring: unknown signer → 72.
    let stranger = test_home("stranger");
    let (rc, out, _) = run(
        &["info", "--verify", img.to_str().unwrap()],
        &w.0,
        &stranger,
    );
    assert_eq!(rc, 72, "{out}");
    assert!(out.contains("not in the trusted keyring"), "{out}");
    assert!(out.contains("result: FAILED (exit 72)\n"), "{out}");
}

#[test]
fn verify_json_carries_checks() {
    let w = TempDir::new("ijv");
    let home = test_home("ijv");
    let img = mk_image(&w, "plain.tfs", None);
    let (rc, out, _) = run(
        &["info", "--verify", "--json", img.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    let doc = tebako_json::parse(&out).unwrap();
    assert_eq!(doc.find("info_schema").and_then(|v| v.as_u64()), Some(1));
    let checks = doc.find("checks").expect("checks");
    assert!(format!("{checks:?}").contains("signature"), "{out}");
}

// ---------------------------------------------------------------------
// The cache-entry (directory) form (spec 15 §4)
// ---------------------------------------------------------------------

#[test]
fn info_on_a_cache_entry_directory() {
    let w = TempDir::new("icache");
    let home = test_home("icache");
    let img = mk_image(&w, "app.tfs", Some(APP_MANIFEST));
    // A cache entry IS artifacts + markers: one .tfs plus marker files.
    let entry = w.0.join("payloads/metanorma");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::copy(&img, entry.join("1.2.3.tfs")).unwrap();
    std::fs::write(entry.join("1.2.3.tfs.sha256"), "abc  1.2.3.tfs\n").unwrap();

    let (rc, out, _) = run(
        &["info", "--provides", entry.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("kind: app  name: metanorma"), "{out}");

    // No .tfs → named error.
    let empty = w.0.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let (rc, _, err) = run(
        &["info", "--provides", empty.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 1);
    assert!(err.contains("no .tfs payload in directory"), "{err}");

    // Several .tfs → named error.
    std::fs::copy(&img, entry.join("1.2.4.tfs")).unwrap();
    let (rc, _, err) = run(
        &["info", "--provides", entry.to_str().unwrap()],
        &w.0,
        &home,
    );
    assert_eq!(rc, 1);
    assert!(err.contains("several .tfs payloads"), "{err}");
}
