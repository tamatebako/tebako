//! The 10 self-test scenarios (a direct port of the C++ `test/self-test.sh`)
//! against the Rust bootstrap.

mod harness;

use harness::{cpp_bootstrap, rust_bootstrap, Harness, RUBY_VER, TEBAKO_VER};

fn h() -> Harness {
    Harness::new(rust_bootstrap())
}

#[test]
fn s1_download_path() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home");
    let (rc, out, err) = h.run(&pkg, &home, &[], &["hello", "arg two"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
    assert!(out.contains("--tebako-image"), "{out}");
    assert!(out.contains(":0:/__tfs__"), "{out}");
    assert!(out.contains("--tebako-entry"), "{out}");
    assert!(out.contains("hello"), "{out}");
    assert!(out.contains("arg two"), "{out}");

    let cached = h.cache_exe(&home);
    assert!(
        cached.is_file(),
        "runtime not installed into cache: {}",
        cached.display()
    );
    let entry_dir = home.join("runtimes").join(&h.entry);
    assert!(
        entry_dir.join("sha256").is_file(),
        "sha256 metadata missing"
    );
    assert!(
        entry_dir.join("origin").is_file(),
        "origin metadata missing"
    );

    // origin carries provenance (runtime_ref / url / sha256).
    let origin = std::fs::read_to_string(entry_dir.join("origin")).unwrap();
    assert!(
        origin.contains(&format!("runtime_ref={}", h.runtime_ref)),
        "{origin}"
    );
    assert!(origin.contains("url="), "{origin}");
    let sha_meta = std::fs::read_to_string(entry_dir.join("sha256")).unwrap();
    assert_eq!(sha_meta, format!("{}  {}\n", h.sha, h.asset));
}

#[test]
fn s2_cache_hit_path_offline() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home");
    assert_eq!(h.run(&pkg, &home, &[], &["hello"]).0, 0);
    // Remove the mirror and force offline: the cache must serve the runtime.
    std::fs::rename(&h.mirror_root, h.tmp.0.join("mirror-gone")).unwrap();
    let (rc, out, _) = h.run(
        &pkg,
        &home,
        &[("TEBAKO_OFFLINE", "1")],
        &["hello", "arg two"],
    );
    assert_eq!(rc, 0);
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
}

#[test]
fn s3_offline_miss() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home2");
    let (rc, _, err) = h.run(&pkg, &home, &[("TEBAKO_OFFLINE", "1")], &[]);
    assert_eq!(rc, 69, "{err}");
    assert!(err.contains(&h.runtime_ref), "{err}");
    assert!(err.contains("TEBAKO_RUNTIME_MIRROR"), "{err}");
    assert!(err.contains("TEBAKO_OFFLINE"), "{err}");
}

#[test]
fn s4_sha256_mismatch_refused() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    // Poison the mirror's manifest.
    let badsha = "0".repeat(64);
    let manifest = h
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    let text = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace(&h.sha, &badsha);
    std::fs::write(&manifest, text).unwrap();

    let home = h.home("home3");
    let (rc, _, err) = h.run(&pkg, &home, &[], &[]);
    assert_eq!(rc, 70, "{err}");
    assert!(err.to_lowercase().contains("sha256"), "{err}");
    assert!(err.contains("deleted"), "{err}");
    assert!(
        !home.join("runtimes").join(&h.entry).exists(),
        "mismatched runtime entered the cache"
    );
}

#[test]
fn s5_launcher_abi_mismatch() {
    let h = h();
    let img = h.fake_image();
    let pkg = h.tmp.0.join("abi99");
    h.stitch(
        &h.bootstrap,
        &[(img, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__")],
        &h.runtime_ref,
        99,
        &pkg,
    );
    let (rc, _, err) = h.run(&pkg, &h.home("h5"), &[], &[]);
    assert_eq!(rc, 66, "{err}");
    assert!(err.contains("99"), "{err}");
    assert!(err.contains("ABI"), "{err}");
}

#[test]
fn s6_no_trailer() {
    let h = h();
    let (rc, _, err) = h.run(&h.bootstrap, &h.home("h6"), &[], &[]);
    assert_eq!(rc, 65, "{err}");
    assert!(err.to_lowercase().contains("manifest"), "{err}");
    assert!(err.contains("no tebako manifest trailer"), "{err}");
}

#[test]
fn s7_corrupt_trailer() {
    let h = h();
    let pkg = h.lean_pkg("corrupt");
    // Flip one byte inside the trailer header's crc field (last 4 bytes).
    use std::io::{Read, Seek, SeekFrom, Write as _};
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&pkg)
        .unwrap();
    let len = f.metadata().unwrap().len();
    f.seek(SeekFrom::Start(len - 1)).unwrap();
    let mut b = [0u8; 1];
    f.read_exact(&mut b).unwrap();
    f.seek(SeekFrom::Start(len - 1)).unwrap();
    f.write_all(&[b[0] ^ 0xFF]).unwrap();
    drop(f);

    let (rc, _, err) = h.run(&pkg, &h.home("h7"), &[], &[]);
    assert_eq!(rc, 65, "{err}");
    assert!(err.contains("corrupt"), "{err}");
}

#[test]
fn s8_fat_package_installs_payload_offline() {
    let h = h();
    let fat = h.fat_pkg("fatapp", &h.fake_runtime);
    let home = h.home("home-fat");
    let (rc, out, err) = h.run(
        &fat,
        &home,
        &[("TEBAKO_OFFLINE", "1")],
        &["hello", "arg two"],
    );
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
    // The payload slot must NOT leak into --tebako-image argv.
    assert_eq!(out.matches("--tebako-image").count(), 1, "{out}");
    assert!(out.contains(":0:/__tfs__"), "{out}");
    assert!(out.contains("hello"), "{out}");

    let cached = h.cache_exe(&home);
    assert!(cached.is_file(), "payload not installed into cache");
    let entry_dir = home.join("runtimes").join(&h.entry);
    let sha_meta = std::fs::read_to_string(entry_dir.join("sha256")).unwrap();
    assert_eq!(sha_meta, format!("{}  {}\n", h.sha, h.asset));
    let origin = std::fs::read_to_string(entry_dir.join("origin")).unwrap();
    assert!(origin.contains("payload="), "{origin}");
}

#[test]
fn s9_fat_tampered_payload_refused() {
    let h = h();
    let tampered = h.tmp.0.join("tampered-runtime");
    std::fs::copy(&h.fake_runtime, &tampered).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&tampered)
        .unwrap();
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&tampered)
        .unwrap()
        .write_all(b"X")
        .unwrap();
    let bad = h.fat_pkg_mismatched("badfat", &tampered);
    let home = h.home("home-bad");
    let (rc, _, err) = h.run(&bad, &home, &[("TEBAKO_OFFLINE", "1")], &[]);
    assert_eq!(rc, 70, "{err}");
    assert!(err.to_lowercase().contains("payload"), "{err}");
    assert!(
        !home.join("runtimes").join(&h.entry).exists(),
        "mismatched payload entered the cache"
    );
}

#[test]
fn s10_populated_cache_wins_over_payload() {
    let h = h();
    let fat = h.fat_pkg("fatapp", &h.fake_runtime);
    let home = h.home("home-fat");
    assert_eq!(
        h.run(&fat, &home, &[("TEBAKO_OFFLINE", "1")], &["hello"]).0,
        0
    );

    let tampered = h.tmp.0.join("tampered-runtime");
    std::fs::copy(&h.fake_runtime, &tampered).unwrap();
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&tampered)
        .unwrap()
        .write_all(b"X")
        .unwrap();
    let bad = h.fat_pkg_mismatched("badfat", &tampered);
    // Same cache: the populated entry is used without re-verifying the payload.
    let (rc, out, _) = h.run(&bad, &home, &[("TEBAKO_OFFLINE", "1")], &["hello"]);
    assert_eq!(rc, 0);
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
}

// ---------------------------------------------------------------------
// Unit surfaces: runtime_ref / checksum extraction / naming
// ---------------------------------------------------------------------

#[test]
fn runtime_ref_parsing() {
    use tebako_bootstrap::parse_runtime_ref;
    let rr = parse_runtime_ref("ruby@3.3.7;tebako=1").unwrap();
    assert_eq!(rr.r#type, "ruby");
    assert_eq!(rr.version, "3.3.7");
    assert_eq!(rr.abi, "1");

    // Trailing parameters tolerated.
    let rr = parse_runtime_ref("ruby@3.3.7;tebako=1;sha256=abc").unwrap();
    assert_eq!(rr.abi, "1");

    for bad in [
        "ruby",
        "@3.3.7;tebako=1",
        "ruby@;tebako=1",
        "ruby@3.3.7",
        "ruby@3.3.7;tebako=",
        "ru by@3.3.7;tebako=1",
        "ruby@3.3.7;tebako=/1",
    ] {
        assert!(parse_runtime_ref(bad).is_err(), "{bad} must fail");
        assert_eq!(parse_runtime_ref(bad).unwrap_err().code, 67);
    }
}

#[test]
fn runtime_ref_sha256_extraction() {
    use tebako_bootstrap::runtime_ref_sha256;
    let sha = "a".repeat(64);
    assert_eq!(
        runtime_ref_sha256(&format!("ruby@3.3.7;tebako=1;sha256={sha}")).unwrap(),
        sha
    );
    assert!(runtime_ref_sha256("ruby@3.3.7;tebako=1").is_err());
    assert!(runtime_ref_sha256(&format!("ruby@3.3.7;tebako=1;sha256={}", "A".repeat(64))).is_err()); // uppercase refused
    assert!(runtime_ref_sha256(&format!("ruby@3.3.7;tebako=1;sha256={sha}x")).is_err());
    // wrong length
}

#[test]
fn release_checksum_extraction() {
    use tebako_bootstrap::{sha_from_manifest_json, sha_from_sums};
    let sha = "c".repeat(64);
    let manifest = format!(
        "[{{\"filename\": \"other\", \"sha256\": \"{}\"}}, {{\"filename\": \"asset-a\", \"sha256\": \"{sha}\"}}]",
        "d".repeat(64)
    );
    assert_eq!(sha_from_manifest_json(&manifest, "asset-a").unwrap(), sha);
    assert!(sha_from_manifest_json(&manifest, "missing").is_err());

    let sums = format!("{}  other\n{sha}  asset-a\n", "d".repeat(64));
    assert_eq!(sha_from_sums(&sums, "asset-a").unwrap(), sha);
    let sums_star = format!("{sha} *asset-a\n");
    assert_eq!(sha_from_sums(&sums_star, "asset-a").unwrap(), sha);
    assert!(sha_from_sums(&sums, "missing").is_err());
}

#[test]
fn platform_and_asset_naming() {
    // The launcher's compile-time mapping (cross-checked against the C++
    // self-test's independent derivation).
    let plat = tebako_bootstrap::platform::platform_string();
    let exe = tebako_bootstrap::platform::exe_suffix();
    let asset = format!("tebako-runtime-{TEBAKO_VER}-{RUBY_VER}-{plat}{exe}");
    assert!(asset.starts_with("tebako-runtime-9.9.9-3.3.7-"), "{asset}");
    let entry = format!("ruby-{RUBY_VER}-{TEBAKO_VER}-{plat}");
    assert!(entry.starts_with("ruby-3.3.7-9.9.9-"), "{entry}");
}

// ---------------------------------------------------------------------
// Cross-oracle parity: the SAME fixtures through the C++ bootstrap
// ---------------------------------------------------------------------

#[test]
fn parity_with_cpp_bootstrap() {
    let Some(cpp) = cpp_bootstrap() else {
        eprintln!("skipping C++ parity: no tebako-bootstrap oracle (set TEBAKO_CPP_BOOTSTRAP)");
        return;
    };

    // Build the harness around the ORACLE binary: the packages stitch the
    // C++ bootstrap as their base; then run BOTH bases' packages through
    // the same mirror and compare key outcomes.
    for (base, tag) in [(rust_bootstrap(), "rs"), (cpp.clone(), "cpp")] {
        let h = Harness::new(base);
        let pkg = h.lean_pkg(&format!("pkg-{tag}"));
        let home = h.home(&format!("home-{tag}"));
        let (rc, out, err) = h.run(&pkg, &home, &[], &["hello", "arg two"]);
        assert_eq!((rc, err.as_str()), (0, ""), "{tag}: {err}");
        assert!(out.contains("FAKE-RUNTIME"), "{tag}: {out}");
        assert!(out.contains("--tebako-image"), "{tag}: {out}");
        assert!(out.contains(":0:/__tfs__"), "{tag}: {out}");
        assert!(out.contains("--tebako-entry"), "{tag}: {out}");
        assert!(out.contains("hello"), "{tag}: {out}");
        assert!(out.contains("arg two"), "{tag}: {out}");
        assert!(h.cache_exe(&home).is_file(), "{tag}: not cached");

        // error paths: offline miss and abi mismatch, same codes + key text.
        let (rc, _, err) = h.run(
            &pkg,
            &h.home(&format!("home2-{tag}")),
            &[("TEBAKO_OFFLINE", "1")],
            &[],
        );
        assert_eq!(rc, 69, "{tag}: {err}");
        assert!(err.contains(&h.runtime_ref), "{tag}: {err}");
        assert!(err.contains("TEBAKO_RUNTIME_MIRROR"), "{tag}: {err}");

        let img = h.fake_image();
        let abi_pkg = h.tmp.0.join(format!("abi99-{tag}"));
        h.stitch(
            &h.bootstrap,
            &[(img, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__")],
            &h.runtime_ref,
            99,
            &abi_pkg,
        );
        let (rc, _, err) = h.run(&abi_pkg, &h.home(&format!("h5-{tag}")), &[], &[]);
        assert_eq!(rc, 66, "{tag}: {err}");
        assert!(err.contains("99"), "{tag}: {err}");

        // fat package: payload install + argv shape.
        let fat = h.fat_pkg(&format!("fat-{tag}"), &h.fake_runtime);
        let home_fat = h.home(&format!("home-fat-{tag}"));
        let (rc, out, err) = h.run(&fat, &home_fat, &[("TEBAKO_OFFLINE", "1")], &["hello"]);
        assert_eq!((rc, err.as_str()), (0, ""), "{tag}: {err}");
        assert_eq!(out.matches("--tebako-image").count(), 1, "{tag}: {out}");
    }
}

// ---------------------------------------------------------------------
// item 30b: the `;image` flag (runtime-as-image resolution)
// ---------------------------------------------------------------------

#[test]
fn s11_image_era_lean_resolves_image_and_hands_env() {
    let h1 = h();
    let pkg = h1.lean_pkg_image("imgapp");
    let home = h1.home("home");
    let (rc, out, err) = h1.run(&pkg, &home, &[], &["hello"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");

    // The image landed in the cache as an immutable artifact + markers.
    let image = h1.cache_image(&home);
    assert!(image.is_file(), "image not installed: {}", image.display());
    let marker = home
        .join("runtimes")
        .join(&h1.entry)
        .join(format!("{}.sha256", h1.image_asset));
    assert!(marker.is_file(), "trusted marker missing");
    let marker_text = std::fs::read_to_string(&marker).unwrap();
    assert_eq!(
        marker_text,
        format!("{}  {}\n", h1.image_sha, h1.image_asset)
    );
    let origin = home
        .join("runtimes")
        .join(&h1.entry)
        .join(format!("{}.origin", h1.image_asset));
    assert!(origin.is_file(), "image origin missing");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&image).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o444, "image must be read-only: {mode:o}");
    }
    // No extracted layout tree anywhere in the cache.
    assert!(
        !home
            .join("runtimes")
            .join(&h1.entry)
            .join("layout")
            .exists(),
        "the cache must not hold an extracted layout tree"
    );

    // The driver gets the image path in the environment; the handoff
    // options themselves are the v1 shape.
    assert!(
        out.contains(&format!("TEBAKO_RUNTIME_IMAGE={}", image.display())),
        "{out}"
    );
    assert!(out.contains("--tebako-image"), "{out}");
    assert!(out.contains(":0:/__tfs__"), "{out}");

    // A v1 (no-`;image`) package against the SAME image-carrying mirror
    // resolves the executable only — byte-identical v1 behavior.
    let h2 = h();
    let pkg_v1 = h2.lean_pkg("plainapp");
    let home_v1 = h2.home("home-v1");
    let (rc, out, err) = h2.run(&pkg_v1, &home_v1, &[], &["hello"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("TEBAKO_RUNTIME_IMAGE=\n"), "{out}");
    assert!(
        !h2.cache_image(&home_v1).exists(),
        "v1 must not fetch the image"
    );
}

#[test]
fn s12_image_cache_hit_offline() {
    let h = h();
    let pkg = h.lean_pkg_image("imgapp");
    let home = h.home("home");
    assert_eq!(h.run(&pkg, &home, &[], &[]).0, 0);
    // Remove the mirror and force offline: the cache must serve both.
    std::fs::rename(&h.mirror_root, h.tmp.0.join("mirror-gone")).unwrap();
    let (rc, out, _) = h.run(&pkg, &home, &[("TEBAKO_OFFLINE", "1")], &["hello"]);
    assert_eq!(rc, 0);
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
    assert!(out.contains("TEBAKO_RUNTIME_IMAGE="), "{out}");
}

#[test]
fn s13_image_offline_miss_and_sha_mismatch() {
    // executable cached but image missing + offline: the image offline
    // error names the image (69).
    let h1 = h();
    let home = h1.home("home2");
    let v1 = h1.lean_pkg("warmapp");
    assert_eq!(h1.run(&v1, &home, &[], &[]).0, 0, "warm-up install");
    let pkg = h1.lean_pkg_image("imgapp");
    let (rc, _, err) = h1.run(&pkg, &home, &[("TEBAKO_OFFLINE", "1")], &[]);
    assert_eq!(rc, 69, "{err}");
    assert!(err.contains("runtime image"), "{err}");
    assert!(err.contains("TEBAKO_OFFLINE"), "{err}");

    // tampered mirror image: 70, download deleted, cache untouched.
    let h2 = h();
    let tampered = h2
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join(&h2.image_asset);
    std::fs::write(&tampered, b"TAMPERED IMAGE BYTES").unwrap();
    let pkg = h2.lean_pkg_image("imgapp2");
    let home = h2.home("home3");
    let (rc, _, err) = h2.run(&pkg, &home, &[], &[]);
    assert_eq!(rc, 70, "{err}");
    assert!(
        err.contains("SHA256 mismatch for downloaded runtime image"),
        "{err}"
    );
    assert!(
        !h2.cache_image(&home).exists(),
        "tampered image must not install"
    );
    assert!(
        !home
            .join("runtimes")
            .join(&h2.entry)
            .join(format!("{}.sha256", h2.image_asset))
            .exists(),
        "no marker without a verified image"
    );
}

#[test]
fn s14_image_sums_fallback_and_fat_payload() {
    // manifest.json WITHOUT the image key (but WITH the contract set —
    // the pre-download gate runs first): the SHA256SUMS line supplies
    // the expected checksum.
    let h1 = h();
    let manifest = h1
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    std::fs::write(
        &manifest,
        format!(
            "[\n  {{\n    \"tebako_version\": \"{TEBAKO_VER}\",\n    \"contract_era\": 2,\n    \"contract_version\": 2,\n    \"mount_root\": \"/__tfs__\",\n    \"ruby_version\": \"{RUBY_VER}\",\n    \"platform\": \"{}\",\n    \"filename\": \"{}\",\n    \"sha256\": \"{}\",\n    \"size_bytes\": 12345\n  }}\n]\n",
            harness::platform(),
            h1.asset,
            h1.sha
        ),
    )
    .unwrap();
    let pkg = h1.lean_pkg_image("imgapp");
    let home = h1.home("home");
    let (rc, _, err) = h1.run(&pkg, &home, &[], &[]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(h1.cache_image(&home).is_file());

    // fat package with the `;image` flag: the payload installs the
    // executable, the image resolves from the mirror (offline-fat +
    // online-mirror combination).
    let h2 = h();
    let fat = {
        let out = h2.tmp.0.join("fatimg");
        let img = h2.fake_image();
        let ref_full = format!(
            "{};image;sha256={}",
            h2.runtime_ref,
            harness::sha256_of(&h2.fake_runtime)
        );
        h2.stitch(
            &h2.bootstrap,
            &[
                (img, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__"),
                (h2.fake_runtime.clone(), tpkg::TPKG_FORMAT_RUNTIME, ""),
            ],
            &ref_full,
            0,
            &out,
        );
        out
    };
    let home2 = h2.home("home-fat");
    let (rc, out, err) = h2.run(&fat, &home2, &[], &["hello"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
    assert!(
        h2.cache_image(&home2).is_file(),
        "image must resolve for ;image fat"
    );
}

#[test]
fn runtime_ref_wants_image_parsing() {
    use tebako_bootstrap::runtime_ref_wants_image as wants;
    assert!(wants("ruby@3.3.7;tebako=0.15.9;image"));
    assert!(wants("ruby@3.3.7;tebako=0.15.9;image;sha256=604e87a1b1d74a6868b35ecdbb11c4e3db01b23286cea9f078636fdf246172b8"));
    assert!(!wants("ruby@3.3.7;tebako=0.15.9"));
    assert!(!wants("ruby@3.3.7;tebako=0.15.9;sha256=604e87a1b1d74a6868b35ecdbb11c4e3db01b23286cea9f078636fdf246172b8"));
    assert!(!wants("ruby@3.3.7;tebako=0.15.9;imagefoo"));
}

#[test]
fn manifest_image_sha_parsing() {
    let text = r#"[
  {
    "tebako_version": "0.15.9",
    "ruby_version": "3.3.7",
    "platform": "macos-arm64",
    "filename": "tebako-runtime-0.15.9-3.3.7-macos-arm64",
    "sha256": "604e87a1b1d74a6868b35ecdbb11c4e3db01b23286cea9f078636fdf246172b8",
    "size_bytes": 24191976,
    "image": {"filename": "tebako-runtime-0.15.9-3.3.7-macos-arm64.tfs", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "size_bytes": 20410208}
  }
]"#;
    // the executable's checksum is unaffected by the image key
    assert_eq!(
        tebako_bootstrap::sha_from_manifest_json(text, "tebako-runtime-0.15.9-3.3.7-macos-arm64"),
        Ok("604e87a1b1d74a6868b35ecdbb11c4e3db01b23286cea9f078636fdf246172b8".to_string())
    );
    assert_eq!(
        tebako_bootstrap::sha_from_manifest_image(
            text,
            "tebako-runtime-0.15.9-3.3.7-macos-arm64.tfs"
        ),
        Ok("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
    );
    assert!(tebako_bootstrap::sha_from_manifest_image(text, "nope.tfs").is_err());
}

// ---------------------------------------------------------------------
// roadmap 45: bootstrap↔runtime contract negotiation
// ---------------------------------------------------------------------

#[test]
fn contract_version_parsing() {
    use tebako_bootstrap::contract_from_manifest_json as contract;
    // Realistic post-45 entry: contract_version precedes filename; the
    // additive image object follows size_bytes.
    let text = r#"[
  {
    "tebako_version": "0.15.9",
    "contract_version": 1,
    "ruby_version": "3.3.7",
    "platform": "macos-arm64",
    "filename": "tebako-runtime-0.15.9-3.3.7-macos-arm64",
    "sha256": "604e87a1b1d74a6868b35ecdbb11c4e3db01b23286cea9f078636fdf246172b8",
    "size_bytes": 24191976,
    "image": {"filename": "tebako-runtime-0.15.9-3.3.7-macos-arm64.tfs", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "size_bytes": 20410208}
  }
]"#;
    let exe = "tebako-runtime-0.15.9-3.3.7-macos-arm64";
    assert_eq!(contract(text, exe), Some(1));

    // A declared 2 is reported (the gate, not the parser, refuses it).
    let text2 = text.replace("\"contract_version\": 1", "\"contract_version\": 2");
    assert_eq!(contract(&text2, exe), Some(2));

    // Absent field = None (the gate reads that as the pre-era signal).
    let legacy = text.replace("    \"contract_version\": 1,\n", "");
    assert!(!legacy.contains("contract_version"));
    assert_eq!(contract(&legacy, exe), None);

    // A non-integer value parses as absent.
    let bad = text.replace("\"contract_version\": 1", "\"contract_version\": \"1\"");
    assert_eq!(contract(&bad, exe), None);

    // Unknown asset.
    assert_eq!(contract(text, "nope"), None);
}

#[test]
fn s15_contract_negotiation_accept_then_refuse() {
    // The era-2 factory shape (the harness default: contract_era 2,
    // contract_version 2, mount_root declared): accepted.
    let h1 = h();
    let pkg = h1.lean_pkg("myapp");
    let home = h1.home("home-c1");
    let (rc, out, err) = h1.run(&pkg, &home, &[], &["hello"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");

    // Declared contract 2 (a future generation): fail CLOSED, exit 75,
    // nothing enters the cache — and the refusal fires BEFORE the
    // download (no download progress in the raw transcript).
    let h2 = h();
    let manifest2 = h2
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    let declared2 = std::fs::read_to_string(&manifest2)
        .unwrap()
        .replace("\"contract_version\": 2", "\"contract_version\": 3");
    std::fs::write(&manifest2, declared2).unwrap();
    let pkg2 = h2.lean_pkg("myapp");
    let home2 = h2.home("home-c2");
    let (rc, _, err) = h2.run_raw(&pkg2, &home2, &[], &[]);
    assert_eq!(rc, 75, "{err}");
    assert!(err.contains("contract_version 3"), "{err}");
    assert!(err.contains("speaks contract 2"), "{err}");
    assert!(
        !err.contains("downloading "),
        "the contract refusal must precede any download: {err}"
    );
    assert!(
        !home2.join("runtimes").join(&h2.entry).exists(),
        "a refused-contract runtime entered the cache"
    );
}

#[test]
fn s11_pre_era_manifest_is_refused_before_download() {
    // spec 18 S11: a release manifest that declares no contract set
    // (the pre-18 factory shape) is refused by name — never adapted.
    let h1 = h();
    let manifest = h1
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    let pre_era = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("    \"contract_era\": 2,\n", "")
        .replace("    \"contract_version\": 2,\n", "")
        .replace("    \"mount_root\": \"/__tfs__\",\n", "");
    assert!(!pre_era.contains("contract_"));
    std::fs::write(&manifest, pre_era).unwrap();
    let pkg = h1.lean_pkg("myapp");
    let home = h1.home("home-pre-era");
    let (rc, _, err) = h1.run_raw(&pkg, &home, &[], &[]);
    assert_eq!(rc, 75, "{err}");
    assert!(err.contains("pre-era"), "{err}");
    assert!(
        err.contains("contract_era, contract_version, mount_root"),
        "{err}"
    );
    assert!(
        !err.contains("downloading "),
        "the pre-era refusal must precede any download: {err}"
    );
    assert!(
        !home.join("runtimes").join(&h1.entry).exists(),
        "a pre-era runtime entered the cache"
    );

    // A newer declared era is the upgrade refusal (both numbers named).
    let h3 = h();
    let manifest3 = h3
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    let era3 = std::fs::read_to_string(&manifest3)
        .unwrap()
        .replace("\"contract_era\": 2", "\"contract_era\": 3");
    std::fs::write(&manifest3, era3).unwrap();
    let pkg3 = h3.lean_pkg("myapp");
    let home3 = h3.home("home-era3");
    let (rc, _, err) = h3.run(&pkg3, &home3, &[], &[]);
    assert_eq!(rc, 75, "{err}");
    assert!(err.contains("era 3"), "{err}");
    assert!(err.contains("speaks era 2"), "{err}");
    assert!(!err.contains("pre-era"), "{err}");
}

#[test]
fn s16_image_contract_mismatch_refused() {
    // The image path applies the same gate (the package entry's
    // contract_version governs its additive image too). Warm the
    // executable cache first so the image path is the one negotiating.
    let h = h();
    let home = h.home("home-ci");
    let warm = h.lean_pkg("warmapp");
    assert_eq!(h.run(&warm, &home, &[], &[]).0, 0, "warm-up install");

    let manifest = h
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    let declared2 = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("\"contract_version\": 2", "\"contract_version\": 3");
    std::fs::write(&manifest, declared2).unwrap();

    let pkg = h.lean_pkg_image("imgapp");
    let (rc, _, err) = h.run_raw(&pkg, &home, &[], &[]);
    assert_eq!(rc, 75, "{err}");
    assert!(err.contains("contract_version 3"), "{err}");
    assert!(
        !err.contains("downloading "),
        "the image-path refusal must precede the image download: {err}"
    );
    assert!(
        !h.cache_image(&home).exists(),
        "a refused-contract image entered the cache"
    );
}

// ---------------------------------------------------------------------
// TODO.v2-1/12: the explicit install verb + run-never-seeds
// ---------------------------------------------------------------------

#[test]
fn s17_install_verb_guides_and_the_flag_refuses() {
    // A normal package: the verb guides to the CLI (exit 76, actionable).
    let h1 = h();
    let pkg = h1.lean_pkg("myapp");
    let (rc, _, err) = h1.run(&pkg, &h1.home("h-inst"), &[], &["--tebako-install"]);
    assert_eq!(rc, 76, "{err}");
    assert!(err.contains("tebako install"), "{err}");
    assert!(
        !h1.home("h-inst").join("payloads").exists(),
        "the verb wrote into the payloads store"
    );

    // A frozen package (TPKG_FLAG_NO_INSTALL): the named refusal.
    let h2 = h();
    let img = h2.fake_image();
    let frozen = h2.tmp.0.join("frozen");
    h2.stitch_flags(
        &h2.bootstrap,
        &[(img, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__")],
        &h2.runtime_ref,
        0,
        tpkg::TPKG_FLAG_NO_INSTALL,
        &frozen,
    );
    let (rc, _, err) = h2.run(&frozen, &h2.home("h-frozen"), &[], &["--tebako-install"]);
    assert_eq!(rc, 76, "{err}");
    assert!(err.contains("non-installable"), "{err}");
}

#[test]
fn s18_a_run_never_seeds_the_store() {
    // TODO.v2-1/12's core law: a run is a run — the runtime cache fills
    // (intrinsic to executing) but NO payload slice touches the store.
    let h = h();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home-run");
    let (rc, out, err) = h.run(&pkg, &home, &[], &["hello"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
    assert!(
        h.cache_exe(&home).is_file(),
        "the runtime cache is intrinsic"
    );
    assert!(
        !home.join("payloads").exists(),
        "a plain run seeded the payloads store"
    );
}

// ---------------------------------------------------------------------
// tebako#456: the release index's `filename` is the ONLY authoritative
// asset spelling (spec 05 §2 SSOT) — the factory publishes windows exe
// assets SUFFIX-LESS; the bootstrap matches the identity triple and
// flows the declared names into the URL, the cache layout and the gate.
// ---------------------------------------------------------------------

#[test]
fn s19_index_filename_is_the_authoritative_spelling() {
    let plat = harness::platform();
    // Two spellings: the real windows factory shape (suffix-less — equal
    // to the synthesized name on posix) and one that diverges from the
    // synthesized spelling on EVERY host.
    for exe_name in [
        format!("tebako-runtime-{TEBAKO_VER}-{RUBY_VER}-{plat}"),
        format!("tebako-runtime-{TEBAKO_VER}-{RUBY_VER}-{plat}-renamed"),
    ] {
        let h = Harness::new_with_exe_asset(rust_bootstrap(), &exe_name);
        let pkg = h.lean_pkg_image("imgapp");
        let home = h.home("home");
        let (rc, out, err) = h.run(&pkg, &home, &[], &["hello"]);
        assert_eq!((rc, err.as_str()), (0, ""), "{exe_name}: {err}");
        assert!(out.contains("FAKE-RUNTIME"), "{exe_name}: {out}");

        // The cache holds the entry under the INDEX spelling, verbatim.
        let entry_dir = home.join("runtimes").join(&h.entry);
        let cached = entry_dir.join(&exe_name);
        assert!(
            cached.is_file(),
            "{exe_name}: runtime not cached under the index spelling"
        );
        let sha_meta = std::fs::read_to_string(entry_dir.join("sha256")).unwrap();
        assert_eq!(sha_meta, format!("{}  {exe_name}\n", h.sha), "{exe_name}");
        assert!(
            h.cache_image(&home).is_file(),
            "{exe_name}: image not cached under the flowed spelling"
        );

        // The cached index flows on the hit path too: mirror gone +
        // offline, the cache still serves (the exe found by identity).
        std::fs::rename(&h.mirror_root, h.tmp.0.join("mirror-gone")).unwrap();
        let (rc, out, err) = h.run(&pkg, &home, &[("TEBAKO_OFFLINE", "1")], &["hello"]);
        assert_eq!(rc, 0, "{exe_name}: {err}");
        assert!(out.contains("FAKE-RUNTIME"), "{exe_name}: {out}");
    }
}

#[test]
fn s20_missing_entry_names_the_identity() {
    // No manifest entry matches the identity triple: the pre-era refusal
    // names the triple alongside the spelling tried (exit 75, before any
    // download — spec 05 §2's honest fallback error).
    let h = h();
    let manifest = h
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    let renamed = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace(
            &format!("\"ruby_version\": \"{RUBY_VER}\""),
            "\"ruby_version\": \"0.0.0\"",
        )
        .replace(
            &format!("\"filename\": \"{}\"", h.asset),
            "\"filename\": \"tebako-runtime-9.9.9-0.0.0-nowhere\"",
        );
    std::fs::write(&manifest, renamed).unwrap();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home-no-entry");
    let (rc, _, err) = h.run_raw(&pkg, &home, &[], &[]);
    assert_eq!(rc, 75, "{err}");
    assert!(err.contains("no entry"), "{err}");
    assert!(err.contains(&format!("ruby_version={RUBY_VER}")), "{err}");
    assert!(
        err.contains(&format!("platform={}", harness::platform())),
        "{err}"
    );
    assert!(
        !err.contains("downloading "),
        "the refusal must precede any download: {err}"
    );
    assert!(
        !home.join("runtimes").join(&h.entry).exists(),
        "a refused runtime entered the cache"
    );
}
