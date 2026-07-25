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
    assert!(out.contains(":0:/__tebako_memfs__"), "{out}");
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
        &[(img, tpkg::TPKG_FORMAT_DWARFS, "/__tebako_memfs__")],
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
    assert!(out.contains(":0:/__tebako_memfs__"), "{out}");
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
        assert!(out.contains(":0:/__tebako_memfs__"), "{tag}: {out}");
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
            &[(img, tpkg::TPKG_FORMAT_DWARFS, "/__tebako_memfs__")],
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
