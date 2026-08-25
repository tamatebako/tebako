//! Acquisition tests (spec 27 slices 4): checksum parsing, the
//! single-member tgz rule, file://-backed verified downloads, the
//! SHA256SUMS version-learning rule, the cold-wipe set per arm, the
//! hermetic child env — and the fat package's WIRE round-trip through
//! tpkg's own reader (the L0 owner validates the bytes the bench wrote).
//! Everything here is offline; the networked flows are the CI legs'.

use std::path::Path;

use tebako_bench::acquire::{self, BenchLayout};
use tebako_bench::suite::{Target, TargetKind};

fn make_tgz(members: &[(&str, &[u8], bool)]) -> Vec<u8> {
    let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut builder = tar::Builder::new(enc);
    for (name, bytes, is_dir) in members {
        if *is_dir {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, name, std::io::empty())
                .unwrap();
        } else {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *bytes).unwrap();
        }
    }
    builder.into_inner().unwrap().finish().unwrap()
}

#[test]
fn sha256sums_parses_the_coreutils_format() {
    let sums = "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3  tebako-0.2.5-macos-arm64\n\
                b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c *tebako-shim-0.2.5-macos-arm64\n";
    assert_eq!(
        acquire::parse_sha256sums(sums, "tebako-0.2.5-macos-arm64").as_deref(),
        Some("a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3")
    );
    // the binary-marker form (`sha256sum -b`) is accepted
    assert_eq!(
        acquire::parse_sha256sums(sums, "tebako-shim-0.2.5-macos-arm64").as_deref(),
        Some("b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c")
    );
    assert!(acquire::parse_sha256sums(sums, "tfs-0.2.5-macos-arm64").is_none());
    // uppercase digests normalize to lowercase
    let upper = "A665A45920422F9D417E4867EFDC4FB8A04A1F3FFF1FA07E998E86F7F7A27AE3  x\n";
    assert_eq!(
        acquire::parse_sha256sums(upper, "x").as_deref(),
        Some("a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3")
    );
}

#[test]
fn bare_hash_sidecar_parses() {
    let h = "e82157502615d12ca61a9558c7e10c6f52215d9b5ee3695aaf4852c27408bbb4";
    assert_eq!(
        acquire::parse_bare_hash(&format!("{h}\n")).as_deref(),
        Some(h)
    );
    // a trailing filename (some sidecars carry one) is tolerated
    assert_eq!(
        acquire::parse_bare_hash(&format!("{h}  metanorma.tgz\n")).as_deref(),
        Some(h)
    );
    assert!(acquire::parse_bare_hash("not-a-hash\n").is_none());
    assert!(acquire::parse_bare_hash("").is_none());
}

#[test]
fn sha256_matches_the_known_vector() {
    // sha256("abc") — the canonical test vector.
    assert_eq!(
        acquire::sha256_bytes_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn single_member_tgz_extracts_the_one_executable() {
    let dir = tempfile::tempdir().unwrap();
    let tgz = make_tgz(&[("metanorma-darwin-arm64", b"\xcf\xfaMach-O fake", false)]);
    let exe = acquire::extract_single_member_tgz(&tgz, dir.path()).unwrap();
    assert_eq!(
        exe.file_name().unwrap().to_string_lossy(),
        "metanorma-darwin-arm64"
    );
    assert_eq!(std::fs::read(&exe).unwrap(), b"\xcf\xfaMach-O fake");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // the caller chmods; here the member must at least be a file
        assert!(exe.is_file());
        let _ = exe.metadata().unwrap().permissions().mode();
    }
}

#[test]
fn tgz_with_dirs_still_has_one_member() {
    let dir = tempfile::tempdir().unwrap();
    let tgz = make_tgz(&[
        ("./", b"", true),
        ("metanorma-linux-x86_64", b"ELF fake", false),
    ]);
    let exe = acquire::extract_single_member_tgz(&tgz, dir.path()).unwrap();
    assert_eq!(std::fs::read(&exe).unwrap(), b"ELF fake");
}

#[test]
fn multi_member_tgz_is_a_named_error() {
    let dir = tempfile::tempdir().unwrap();
    let tgz = make_tgz(&[("a", b"1", false), ("b", b"2", false)]);
    let err = acquire::extract_single_member_tgz(&tgz, dir.path()).unwrap_err();
    assert!(
        err.message.contains("single-member"),
        "named error expected: {}",
        err.message
    );
}

#[test]
fn empty_tgz_is_a_named_error() {
    let dir = tempfile::tempdir().unwrap();
    let tgz = make_tgz(&[]);
    let err = acquire::extract_single_member_tgz(&tgz, dir.path()).unwrap_err();
    assert!(
        err.message.contains("NO file member"),
        "named error expected: {}",
        err.message
    );
}

#[test]
fn download_verified_via_file_url() {
    let dir = tempfile::tempdir().unwrap();
    let asset = dir.path().join("asset.bin");
    std::fs::write(&asset, b"verified bytes").unwrap();
    let sha = acquire::sha256_file_hex(&asset).unwrap();
    let dest = dir.path().join("out").join("asset.bin");
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    let file_url = format!("file://{}", asset.display());
    acquire::download_verified(&file_url, &dest, &sha).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"verified bytes");
    // a mismatch writes NOTHING (the trust anchor is the checksum)
    let err =
        acquire::download_verified(&file_url, &dest.with_file_name("nope.bin"), &"0".repeat(64))
            .unwrap_err();
    assert!(err.message.contains("SHA256 mismatch"), "{}", err.message);
    assert!(!dest.with_file_name("nope.bin").exists());
}

#[test]
fn version_from_sums_learns_the_latest_release_version() {
    let sums = "\
d34db33fd34db33fd34db33fd34db33fd34db33fd34db33fd34db33fd34db33f  tebako-0.2.5-macos-arm64\n\
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  tebako-shim-0.2.5-macos-arm64\n\
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  tebako-bootstrap-0.2.5-macos-arm64\n\
cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc  tebako-pkg-0.2.5-macos-arm64\n\
dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd  tfs-0.2.5-macos-arm64\n\
eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee  tebako-0.2.5-linux-gnu-x86_64\n";
    // the version is the middle of the tebako-<ver>-<triplet> line ONLY —
    // shim/bootstrap/pkg/tfs lines must not leak into it.
    let learned = acquire::testonly_version_from_sums(sums, "macos-arm64");
    assert_eq!(learned.as_deref(), Some("0.2.5"));
    let learned = acquire::testonly_version_from_sums(sums, "linux-gnu-x86_64");
    assert_eq!(learned.as_deref(), Some("0.2.5"));
    assert!(acquire::testonly_version_from_sums(sums, "windows-ucrt64").is_none());
}

// ---------------------------------------------------------------------
// the fat package wire round-trip (tpkg reads what the bench wrote)
// ---------------------------------------------------------------------

fn fake_payload_home(dir: &Path) -> acquire::PayloadHome {
    let image = dir.join("metanorma-1.16.9-macos-arm64.tfs");
    std::fs::write(&image, b"DWARFS-fake-payload-bytes").unwrap();
    let mirror_yaml = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1.16.9\"\n  producer: {tool: test, tool_version: \"1\"}\n  created: \"2026-08-25T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n    blob_sha256: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n  signing: {state: unsigned}\n  encryption: {state: none}\nprovides:\n  entrypoints:\n    - name: metanorma\n      path: /__tebako__/bin/metanorma\n      runtime_requirement: {engine: ruby, constraint: \"~> 3.3.0\", abi: arm64-darwin-23}\n  platforms: universal\n  capabilities: {exec: true, read: true}\n";
    acquire::PayloadHome {
        name: "metanorma".to_string(),
        version: "1.16.9".to_string(),
        release_tag: "1.16.9-3".to_string(),
        image,
        mirror: tpkg::PayloadManifest::from_yaml(mirror_yaml).unwrap(),
        image_format: tebako_bench::result::ImageFormat::Dwarfs,
    }
}

#[test]
fn fat_package_round_trips_through_tpkg() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(dir.path()).unwrap();

    // The three verified inputs: a bootstrap, the payload image (fake
    // home), the runtime exe + its trust-marker digest.
    let bootstrap = dir.path().join("tebako-bootstrap");
    std::fs::write(&bootstrap, b"BOOTSTRAP-BYTES").unwrap();
    let runtime_exe = dir.path().join("tebako-runtime-0.16.9-3.3.7-macos-arm64");
    std::fs::write(&runtime_exe, b"RUNTIME-EXE-BYTES").unwrap();
    let runtime_sha = acquire::sha256_file_hex(&runtime_exe).unwrap();
    let tools = acquire::TebakoTools {
        cli: bootstrap.clone(),
        shim: bootstrap.clone(),
        bootstrap: bootstrap.clone(),
        version: "0.2.5".to_string(),
    };
    let payload = fake_payload_home(dir.path());
    let runtime = acquire::RuntimeEntry {
        engine: "ruby".to_string(),
        lang_version: "3.3.7".to_string(),
        tebako_version: "0.16.9".to_string(),
        exe: runtime_exe,
        exe_sha256: runtime_sha.clone(),
    };
    let target = Target {
        id: "v2-fat".to_string(),
        kind: TargetKind::V2Press,
        payload: Some("metanorma@1.16.9".to_string()),
        registries: Some(vec!["tfs:github:tebako-packages/metanorma".to_string()]),
        fat: Some(true),
    };

    let package = acquire::assemble_fat_package(&layout, &tools, &payload, &runtime, &target)
        .unwrap_or_else(|e| panic!("assemble: {e}"));

    // The wire check rides the L0 owner's reader.
    let mut f = std::fs::File::open(&package).unwrap();
    let m = tpkg::read_from(&mut f).unwrap();
    assert_eq!(m.slots.len(), 2, "payload slot + runtime slot");
    // slot 0: the payload image at "/", bytes intact after the bootstrap.
    assert_eq!(m.slots[0].mount_point_str(), Some("/"));
    assert_eq!(
        m.slots[0].offset as usize,
        b"BOOTSTRAP-BYTES".len(),
        "the payload follows the bootstrap bytes"
    );
    assert_eq!(m.slots[0].size as usize, b"DWARFS-fake-payload-bytes".len());
    // slot 1: the runtime exe, role format, never mounted.
    assert_eq!(m.slots[1].format_id, tpkg::TPKG_FORMAT_RUNTIME);
    assert_eq!(m.slots[1].mount_point_str(), Some(""));
    // the trailer's runtime_ref pins the verified exe digest (the fat
    // package's runtime trust anchor, spec 27 §4's acquisition rule).
    let rr = m.runtime_ref_str().unwrap();
    assert_eq!(
        rr,
        format!("ruby@3.3.7;tebako=0.16.9;image;sha256={runtime_sha}")
    );
    // the type-2 package manifest rode along and names the entrypoint
    // (the payload mirror's authoritative path) + the "/" union mount.
    let pm = m.package_manifest().unwrap().expect("type-2 block present");
    assert_eq!(pm.entries.len(), 1);
    assert_eq!(pm.entries[0].name, "metanorma");
    assert_eq!(pm.entries[0].entrypoint, "/__tebako__/bin/metanorma");
    assert_eq!(pm.entries[0].runtime_ref, rr);
    assert_eq!(pm.mounts.len(), 1);
    assert_eq!(pm.mounts[0].point, "/");
    assert_eq!(pm.mounts[0].mode, tpkg::MountMode::Union);
    // the assembled bytes carry the parts verbatim (the exe slot bytes
    // land where the trailer says they do).
    let bytes = std::fs::read(&package).unwrap();
    let slot1 = &bytes[m.slots[1].offset as usize..(m.slots[1].offset + m.slots[1].size) as usize];
    assert_eq!(slot1, b"RUNTIME-EXE-BYTES");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            package.metadata().unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
}

// ---------------------------------------------------------------------
// the cold-wipe set + the hermetic env (spec 27 §5)
// ---------------------------------------------------------------------

#[test]
fn cold_wipe_matches_the_arm() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(dir.path()).unwrap();
    let mark = |p: &Path| {
        std::fs::create_dir_all(p).unwrap();
        std::fs::write(p.join("marker"), b"x").unwrap();
    };
    for d in [
        layout.home.join(".metanorma"),
        layout.home.join(".relaton"),
        layout.store(),
        layout.store().join("runtimes"),
        layout.tmp.join("t1"),
    ] {
        mark(&d);
    }
    // v1-exe: the payload caches + the per-target TMPDIR go; the store stays.
    layout.wipe_cold_caches("t1", TargetKind::V1Exe).unwrap();
    assert!(!layout.home.join(".metanorma/marker").exists());
    assert!(!layout.home.join(".relaton/marker").exists());
    assert!(!layout.tmp.join("t1/marker").exists());
    assert!(layout.tmp.join("t1").is_dir(), "the TMPDIR is recreated");
    assert!(
        layout.store().join("marker").exists(),
        "v1 never wipes the store"
    );

    // v2-press: runtimes/ goes (the env image re-downloads in-span); the
    // store's payload side survives (the fat package never uses it).
    layout.wipe_cold_caches("t1", TargetKind::V2Press).unwrap();
    assert!(!layout.store().join("runtimes/marker").exists());
    assert!(layout.store().join("marker").exists());

    // v2-managed: the whole store goes (payload re-install is unmeasured,
    // the runtime download lands in the measured span).
    layout
        .wipe_cold_caches("t1", TargetKind::V2Managed)
        .unwrap();
    assert!(!layout.store().join("marker").exists());
}

#[test]
fn child_env_is_the_hermetic_bench_home() {
    let dir = tempfile::tempdir().unwrap();
    let layout = BenchLayout::new(dir.path()).unwrap();
    let env: std::collections::BTreeMap<String, String> =
        layout.child_env("v2-shim").into_iter().collect();
    assert_eq!(
        env.get("HOME").map(String::as_str),
        Some(layout.home.to_string_lossy().as_ref())
    );
    assert_eq!(
        env.get("TEBAKO_HOME").map(String::as_str),
        Some(layout.store().to_string_lossy().as_ref())
    );
    assert_eq!(
        env.get("TMPDIR").map(String::as_str),
        Some(layout.tmp.join("v2-shim").to_string_lossy().as_ref())
    );
    #[cfg(windows)]
    {
        assert!(env.contains_key("USERPROFILE"));
        assert!(env.contains_key("TEMP"));
        assert!(env.contains_key("TMP"));
    }
}
