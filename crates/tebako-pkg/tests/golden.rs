//! Golden comparisons against the C++ tebakofs binary (the parity oracle).
//!
//! The oracle is located via (in order): the TEBAKOFS_CPP env var, the
//! well-known libtfs build tree path, or `tebakofs` on PATH. Tests skip
//! with a message when no oracle is available — the local run proves
//! byte-parity; CI runs the oracle-free suites (roundtrip/cli).

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::TempDir;

fn cpp_tebakofs() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TEBAKOFS_CPP") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let well_known = PathBuf::from("/Users/mulgogi/src/tamatebako/libtfs-pkgwt/build/tebakofs");
    if well_known.is_file() {
        return Some(well_known);
    }
    // PATH lookup.
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let cand = PathBuf::from(dir).join("tebakofs");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn rust_tebako_pkg() -> PathBuf {
    // The integration test runs next to the workspace target dir.
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tebako-pkg");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tebako-pkg binary not built (run `cargo build -p tebako-pkg`)")
}

fn fixtures(w: &TempDir) -> (PathBuf, PathBuf, PathBuf) {
    let boot = w.0.join("boot.bin");
    std::fs::write(&boot, patterned_bytes(8192, 0x5A)).unwrap();
    let a = w.0.join("a.dwarfs");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/contract/tests/fixtures/simple.dwarfs"),
        &a,
    )
    .unwrap();
    let b = w.0.join("b.sqfs");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/contract/tests/fixtures/simple.sqfs"),
        &b,
    )
    .unwrap();
    (boot, a, b)
}

/// Deterministic pseudo-random bytes (both tools must see identical input).
fn patterned_bytes(n: usize, seed: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    let mut x = seed;
    for _ in 0..n {
        x = x.wrapping_mul(31).wrapping_add(17);
        v.push(x);
    }
    v
}

fn run(tool: &Path, args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn tool");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Semantic package equality in the signing era (item 29): the oracle
/// produces v1-unsigned trailers while the Rust tool signs every package
/// with the press-local key, so byte-identity is impossible by design
/// (signatures are per-key and per-time). What MUST hold:
/// - the payload region (bootstrap + images) is byte-identical;
/// - the v1-visible manifest fields match: version, slots, runtime_ref,
///   launcher_abi, package_flags modulo TPKG_FLAG_SIGNED_V2;
/// - the Rust package carries a non-empty v2 signature.
fn assert_packages_semantically_equal(cpp: &Path, rs: &Path) {
    let cpp_bytes = std::fs::read(cpp).unwrap();
    let rs_bytes = std::fs::read(rs).unwrap();
    let mc = tpkg::read_from(&mut std::fs::File::open(cpp).unwrap()).unwrap();
    let mr = tpkg::read_from(&mut std::fs::File::open(rs).unwrap()).unwrap();

    let payload_end =
        |m: &tpkg::Manifest| m.slots.iter().map(|s| s.offset + s.size).max().unwrap_or(0) as usize;
    assert_eq!(
        &cpp_bytes[..payload_end(&mc)],
        &rs_bytes[..payload_end(&mr)],
        "payload region must be byte-identical"
    );

    assert_eq!(mc.version, mr.version, "version");
    assert_eq!(mc.launcher_abi, mr.launcher_abi, "launcher_abi");
    assert_eq!(mc.runtime_ref, mr.runtime_ref, "runtime_ref");
    assert_eq!(
        mc.package_flags & !tpkg::TPKG_FLAG_SIGNED_V2,
        mr.package_flags & !tpkg::TPKG_FLAG_SIGNED_V2,
        "package_flags (modulo SIGNED_V2)"
    );
    assert_eq!(mc.slots, mr.slots, "slots");

    let v2 = mr.v2.as_ref().expect("rust package must be signed (v2)");
    assert!(!v2.signature.is_empty());
}

/// The rust `info` adds a Signature line that the oracle does not have.
fn strip_signature_line(out: &str) -> String {
    out.lines()
        .filter(|l| !l.starts_with("Signature:"))
        .map(|l| format!("{l}\n"))
        .collect()
}

/// unbundle's manifest.json carries the trailer's package_flags (the
/// SIGNED_V2 bit) and header_crc32 (recomputed over the signed header) —
/// both necessarily differ from the unsigned oracle. Normalize them out
/// for the comparison.
fn normalize_manifest_json(text: &str) -> String {
    let mut out = String::new();
    for l in text.lines() {
        let t = l.trim_start();
        if t.starts_with("\"package_flags\":") {
            out.push_str("  \"package_flags\": 0,\n");
        } else if t.starts_with("\"header_crc32\":") {
            out.push_str("  \"header_crc32\": 0,\n");
        } else {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

#[test]
fn golden_bundle_info_unbundle_reassemble() {
    let Some(cpp) = cpp_tebakofs() else {
        eprintln!("skipping golden test: no C++ tebakofs oracle (set TEBAKOFS_CPP)");
        return;
    };
    let rs = rust_tebako_pkg();
    let w = TempDir::new("golden");
    let (boot, a, b) = fixtures(&w);

    // bundle → byte-identical packages.
    let pkg_cpp = w.0.join("pkg-cpp");
    let pkg_rs = w.0.join("pkg-rs");
    let (rc, _, err) = run(
        &cpp,
        &[
            "bundle",
            "--bootstrap",
            boot.to_str().unwrap(),
            "--image",
            a.to_str().unwrap(),
            "--image",
            &format!("{}:/data", b.display()),
            "--runtime-ref",
            "tebako-runtime-0.15.9",
            "--lean",
            "--launcher-abi",
            "3",
            "-o",
            pkg_cpp.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "C++ bundle");
    let (rc, _, err) = run(
        &rs,
        &[
            "bundle",
            "--bootstrap",
            boot.to_str().unwrap(),
            "--image",
            a.to_str().unwrap(),
            "--image",
            &format!("{}:/data", b.display()),
            "--runtime-ref",
            "tebako-runtime-0.15.9",
            "--lean",
            "--launcher-abi",
            "3",
            "-o",
            pkg_rs.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "rust bundle");
    // Byte-identity is impossible by design (the Rust tool signs every
    // package; the oracle does not) — assert the semantic contract.
    assert_packages_semantically_equal(&pkg_cpp, &pkg_rs);

    // info on the SAME package file → identical output modulo the rust-only
    // Signature line (here: unsigned v1 oracle package, reported as legacy).
    let (_, cpp_out, _) = run(&cpp, &["info", "pkg-cpp"], &w.0);
    let (_, rs_out, _) = run(&rs, &["info", "pkg-cpp"], &w.0);
    assert_eq!(
        cpp_out,
        strip_signature_line(&rs_out),
        "info output must match"
    );

    // unbundle → identical manifest.json + byte-identical parts.
    let parts_cpp = w.0.join("parts-cpp");
    let parts_rs = w.0.join("parts-rs");
    assert_eq!(
        run(
            &cpp,
            &["unbundle", "pkg-cpp", "-o", parts_cpp.to_str().unwrap()],
            &w.0
        )
        .0,
        0
    );
    assert_eq!(
        run(
            &rs,
            &["unbundle", "pkg-rs", "-o", parts_rs.to_str().unwrap()],
            &w.0
        )
        .0,
        0
    );
    assert_eq!(
        normalize_manifest_json(&std::fs::read_to_string(parts_cpp.join("manifest.json")).unwrap()),
        normalize_manifest_json(&std::fs::read_to_string(parts_rs.join("manifest.json")).unwrap()),
        "manifest.json must match (modulo signing fields)"
    );
    for name in ["bootstrap.bin", "image-0.bin", "image-1.bin"] {
        assert_eq!(
            std::fs::read(parts_cpp.join(name)).unwrap(),
            std::fs::read(parts_rs.join(name)).unwrap(),
            "{name} must be byte-identical"
        );
    }

    // reassemble → byte-identical to the original package.
    let re_cpp = w.0.join("re-cpp");
    let re_rs = w.0.join("re-rs");
    assert_eq!(
        run(
            &cpp,
            &[
                "reassemble",
                parts_cpp.to_str().unwrap(),
                "-o",
                re_cpp.to_str().unwrap()
            ],
            &w.0
        )
        .0,
        0
    );
    assert_eq!(
        run(
            &rs,
            &[
                "reassemble",
                parts_rs.to_str().unwrap(),
                "-o",
                re_rs.to_str().unwrap()
            ],
            &w.0
        )
        .0,
        0
    );
    // reassemble: re-signs with the local press key (the signature cannot
    // be byte-preserved), so compare semantically — and the reassembled
    // package must match the original oracle bundle on the v1 surface.
    assert_packages_semantically_equal(&re_cpp, &re_rs);
    let mc = tpkg::read_from(&mut std::fs::File::open(&pkg_cpp).unwrap()).unwrap();
    let mr = tpkg::read_from(&mut std::fs::File::open(&re_rs).unwrap()).unwrap();
    assert_eq!(mc.slots, mr.slots, "reassemble must preserve slots");
    assert_eq!(
        mc.runtime_ref, mr.runtime_ref,
        "reassemble must preserve runtime_ref"
    );
}

#[test]
fn golden_insert_remove_set_runtime() {
    let Some(cpp) = cpp_tebakofs() else {
        eprintln!("skipping golden test: no C++ tebakofs oracle (set TEBAKOFS_CPP)");
        return;
    };
    let rs = rust_tebako_pkg();
    let w = TempDir::new("golden2");
    let (boot, a, b) = fixtures(&w);

    for (tool, tag) in [(&cpp, "cpp"), (&rs, "rs")] {
        assert_eq!(
            run(
                tool,
                &[
                    "bundle",
                    "--bootstrap",
                    boot.to_str().unwrap(),
                    "--image",
                    a.to_str().unwrap(),
                    "-o",
                    &format!("pkg-{tag}"),
                ],
                &w.0,
            )
            .0,
            0
        );
    }
    let pkg_cpp = w.0.join("pkg-cpp");
    let pkg_rs = w.0.join("pkg-rs");
    assert_packages_semantically_equal(&pkg_cpp, &pkg_rs);

    // insert-image with an explicit mount point.
    assert_eq!(
        run(
            &cpp,
            &[
                "insert-image",
                "pkg-cpp",
                &format!("{}:/extra", b.display())
            ],
            &w.0
        )
        .0,
        0
    );
    assert_eq!(
        run(
            &rs,
            &["insert-image", "pkg-rs", &format!("{}:/extra", b.display())],
            &w.0
        )
        .0,
        0
    );
    assert_packages_semantically_equal(&pkg_cpp, &pkg_rs);

    // remove the inserted slot.
    assert_eq!(run(&cpp, &["remove-image", "pkg-cpp", "1"], &w.0).0, 0);
    assert_eq!(run(&rs, &["remove-image", "pkg-rs", "1"], &w.0).0, 0);
    assert_packages_semantically_equal(&pkg_cpp, &pkg_rs);

    // set-runtime swaps only the bootstrap region.
    let boot2 = w.0.join("boot2.bin");
    std::fs::write(&boot2, patterned_bytes(4096, 0x11)).unwrap();
    assert_eq!(
        run(
            &cpp,
            &["set-runtime", "pkg-cpp", boot2.to_str().unwrap()],
            &w.0
        )
        .0,
        0
    );
    assert_eq!(
        run(
            &rs,
            &["set-runtime", "pkg-rs", boot2.to_str().unwrap()],
            &w.0
        )
        .0,
        0
    );
    assert_packages_semantically_equal(&pkg_cpp, &pkg_rs);
}

#[test]
fn golden_info_plain_archives_and_errors() {
    let Some(cpp) = cpp_tebakofs() else {
        eprintln!("skipping golden test: no C++ tebakofs oracle (set TEBAKOFS_CPP)");
        return;
    };
    let rs = rust_tebako_pkg();
    let w = TempDir::new("golden3");
    let (_, a, b) = fixtures(&w);

    for file in [&a, &b] {
        // The libtfs v0.13.0 RELEASE tebakofs binary cannot mount SquashFS
        // images (a capability gap of that build — the source build of the
        // same libtfs mounts them fine). Compare only cases the oracle can
        // actually produce; note and skip the rest.
        let (rc1, out1, err1) = run(&cpp, &["info", file.to_str().unwrap()], &w.0);
        if rc1 != 0 && err1.contains("Unsupported format") {
            eprintln!(
                "note: oracle cannot mount {} (release build gap), skipping comparison",
                file.display()
            );
            continue;
        }
        let (rc2, out2, err2) = run(&rs, &["info", file.to_str().unwrap()], &w.0);
        assert_eq!(
            (rc1, out1, err1),
            (rc2, out2, err2),
            "plain info on {}",
            file.display()
        );
    }

    // Error paths: missing file and unknown magic.
    let (rc1, out1, err1) = run(&cpp, &["info", "/nonexistent.zip"], &w.0);
    let (rc2, out2, err2) = run(&rs, &["info", "/nonexistent.zip"], &w.0);
    assert_eq!((rc1, out1, err1), (rc2, out2, err2), "info on missing file");

    let junk = w.0.join("junk.bin");
    std::fs::write(&junk, patterned_bytes(256, 0x7F)).unwrap();
    let (rc1, out1, err1) = run(&cpp, &["info", junk.to_str().unwrap()], &w.0);
    let (rc2, out2, err2) = run(&rs, &["info", junk.to_str().unwrap()], &w.0);
    assert_eq!(
        (rc1, out1, err1),
        (rc2, out2, err2),
        "info on unknown magic"
    );
}
