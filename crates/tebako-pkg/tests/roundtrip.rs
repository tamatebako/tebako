//! Round-trip properties (oracle-free): bundle/unbundle/reassemble must
//! preserve the payload bytes exactly; insert/remove/set-runtime must
//! compose.

use std::path::{Path, PathBuf};

use tebako_contract_tests::TempDir;
use tebako_pkg::{
    bundle, bundle_exact, default_mount, insert_image, parse_image_spec, reassemble, remove_image,
    set_runtime, sniff_format, unbundle, PackageImage, PackageOptions,
};

fn patterned_bytes(n: usize, seed: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    let mut x = seed;
    for _ in 0..n {
        x = x.wrapping_mul(31).wrapping_add(17);
        v.push(x);
    }
    v
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}

fn opts() -> PackageOptions {
    PackageOptions {
        runtime_ref: "rt-1.0".into(),
        package_flags: tpkg::TPKG_FLAG_LEAN,
        launcher_abi: 2,
    }
}

struct Tree {
    boot: PathBuf,
    a: PathBuf,
    b: PathBuf,
}

fn tree(w: &TempDir) -> Tree {
    let boot = w.0.join("boot.bin");
    std::fs::write(&boot, patterned_bytes(4096, 0x01)).unwrap();
    let a = w.0.join("a.dwarfs");
    std::fs::copy(fixture("simple.dwarfs"), &a).unwrap();
    let b = w.0.join("b.sqfs");
    std::fs::copy(fixture("simple.sqfs"), &b).unwrap();
    Tree { boot, a, b }
}

fn bundle_tree(t: &Tree, out: &Path) {
    let images = vec![
        parse_image_spec(t.a.to_str().unwrap()),
        parse_image_spec(&format!("{}:/data", t.b.display())),
    ];
    bundle(&t.boot, &images, out, &opts()).expect("bundle");
}

#[test]
fn bundle_unbundle_reassemble_preserves_payload_bytes() {
    let w = TempDir::new("rt");
    let t = tree(&w);
    let pkg = w.0.join("pkg");
    bundle_tree(&t, &pkg);

    let parts = w.0.join("parts");
    unbundle(&pkg, &parts).expect("unbundle");

    // The bootstrap part equals the bootstrap input; each image part equals
    // its input image (the property: no payload mutation anywhere).
    assert_eq!(
        std::fs::read(parts.join("bootstrap.bin")).unwrap(),
        std::fs::read(&t.boot).unwrap()
    );
    assert_eq!(
        std::fs::read(parts.join("image-0.bin")).unwrap(),
        std::fs::read(&t.a).unwrap()
    );
    assert_eq!(
        std::fs::read(parts.join("image-1.bin")).unwrap(),
        std::fs::read(&t.b).unwrap()
    );

    // reassemble reproduces the original package byte for byte.
    let re = w.0.join("re");
    reassemble(&parts, &re).expect("reassemble");
    assert_eq!(std::fs::read(&pkg).unwrap(), std::fs::read(&re).unwrap());
}

#[test]
fn insert_then_remove_restores_the_package() {
    let w = TempDir::new("rt2");
    let t = tree(&w);
    let pkg = w.0.join("pkg");
    bundle_tree(&t, &pkg);
    let original = std::fs::read(&pkg).unwrap();

    // Insert a third image with the default mount point.
    let extra = w.0.join("extra.sqfs");
    std::fs::copy(fixture("nested.sqfs"), &extra).unwrap();
    insert_image(&pkg, &extra, "").expect("insert-image");

    // The new slot must be slot 2 with the default mount and sniffed format.
    let mut f = std::fs::File::open(&pkg).unwrap();
    let m = tpkg::read_from(&mut f).expect("manifest after insert");
    assert_eq!(m.slots.len(), 3);
    assert_eq!(m.slots[2].format_id, tpkg::TPKG_FORMAT_SQUASHFS);
    assert_eq!(m.slots[2].mount_point_str(), Some("/__tebako_memfs_2__"));

    // Removing it restores the original bytes exactly.
    remove_image(&pkg, 2).expect("remove-image");
    assert_eq!(std::fs::read(&pkg).unwrap(), original);
}

#[test]
fn set_runtime_changes_only_the_bootstrap_region() {
    let w = TempDir::new("rt3");
    let t = tree(&w);
    let pkg = w.0.join("pkg");
    bundle_tree(&t, &pkg);

    let boot2 = w.0.join("boot2.bin");
    std::fs::write(&boot2, patterned_bytes(2048, 0x42)).unwrap();
    set_runtime(&pkg, &boot2).expect("set-runtime");

    // Slots (offset shifted by the new bootstrap size) keep their bytes and
    // trailer fields (flags, launcher_abi, runtime_ref, mount points).
    let mut f = std::fs::File::open(&pkg).unwrap();
    let m = tpkg::read_from(&mut f).expect("manifest after set-runtime");
    assert_eq!(m.slots.len(), 2);
    assert_eq!(m.slots[0].offset, 2048);
    assert_eq!(
        m.slots[0].size as usize,
        std::fs::metadata(&t.a).unwrap().len() as usize
    );
    assert_eq!(m.slots[1].mount_point_str(), Some("/data"));
    assert_eq!(m.package_flags, tpkg::TPKG_FLAG_LEAN);
    assert_eq!(m.launcher_abi, 2);
    assert_eq!(m.runtime_ref_str(), Some("rt-1.0"));

    // The image bytes survive at their new offsets.
    let all = std::fs::read(&pkg).unwrap();
    let a = std::fs::read(&t.a).unwrap();
    assert_eq!(&all[2048..2048 + a.len()], &a[..]);
}

#[test]
fn manifest_json_carries_crc32_of_each_part() {
    let w = TempDir::new("rt4");
    let t = tree(&w);
    let pkg = w.0.join("pkg");
    bundle_tree(&t, &pkg);
    let parts = w.0.join("parts");
    unbundle(&pkg, &parts).expect("unbundle");

    let manifest = std::fs::read_to_string(parts.join("manifest.json")).unwrap();
    let boot_crc = tpkg::crc32(&std::fs::read(&t.boot).unwrap());
    let a_crc = tpkg::crc32(&std::fs::read(&t.a).unwrap());
    let b_crc = tpkg::crc32(&std::fs::read(&t.b).unwrap());
    assert!(
        manifest.contains(&format!("\"crc32\": {boot_crc}")),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!("\"crc32\": {a_crc}")),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!("\"crc32\": {b_crc}")),
        "{manifest}"
    );
}

// ---------------------------------------------------------------------
// Small unit surfaces
// ---------------------------------------------------------------------

#[test]
fn image_spec_parsing_and_defaults() {
    let img = parse_image_spec("a.dwarfs:/mnt");
    assert_eq!(img.path, Path::new("a.dwarfs"));
    assert_eq!(img.mount_point, "/mnt");

    let img = parse_image_spec("a.dwarfs");
    assert_eq!(img.path, Path::new("a.dwarfs"));
    assert_eq!(img.mount_point, "");

    // The C++ rule is purely syntactic: a colon followed by '/' splits,
    // even in drive-letter-looking specs ("C:/x" -> path "C", mount "/x").
    let img = parse_image_spec("C:/images/a.dwarfs");
    assert_eq!(img.path, Path::new("C"));
    assert_eq!(img.mount_point, "/images/a.dwarfs");

    assert_eq!(default_mount(0), "/__tebako_memfs__");
    assert_eq!(default_mount(3), "/__tebako_memfs_3__");

    assert_eq!(
        sniff_format(&fixture("simple.dwarfs")),
        tpkg::TPKG_FORMAT_DWARFS
    );
    assert_eq!(
        sniff_format(&fixture("simple.sqfs")),
        tpkg::TPKG_FORMAT_SQUASHFS
    );
}

#[test]
fn validation_errors_match_cpp() {
    let w = TempDir::new("rt5");
    let t = tree(&w);

    // No images.
    let e = bundle(&t.boot, &[], &w.0.join("x"), &opts()).unwrap_err();
    assert_eq!(e, "image count out of range (1..8)");

    // Missing bootstrap.
    let images = vec![PackageImage {
        path: t.a.clone(),
        mount_point: String::new(),
        format_id: 0,
    }];
    let e = bundle(&w.0.join("nope"), &images, &w.0.join("x"), &opts()).unwrap_err();
    assert!(e.starts_with("bootstrap file not found: "), "{e}");

    // runtime_ref too long.
    let bad = PackageOptions {
        runtime_ref: "x".repeat(128),
        ..opts()
    };
    let e = bundle(&t.boot, &images, &w.0.join("x"), &bad).unwrap_err();
    assert_eq!(e, "runtime_ref is too long (max 127 characters)");

    // Output clobbers an input.
    let e = bundle(&t.boot, &images, &t.a, &opts()).unwrap_err();
    assert!(
        e.starts_with("output path must differ from the image file: "),
        "{e}"
    );

    // Unbundle a non-package.
    let junk = w.0.join("junk.bin");
    std::fs::write(&junk, patterned_bytes(256, 0x33)).unwrap();
    let e = unbundle(&junk, &w.0.join("o")).unwrap_err();
    assert_eq!(
        e,
        format!(
            "{}: no tpkg manifest trailer present (not a three-part package)",
            junk.display()
        )
    );

    // Remove-image out of range / last slot.
    let pkg = w.0.join("pkg");
    bundle_tree(&t, &pkg);
    let e = remove_image(&pkg, 7).unwrap_err();
    assert_eq!(
        e,
        format!(
            "{}: slot index 7 out of range (package has 2 slot(s))",
            pkg.display()
        )
    );
}

#[test]
fn bundle_exact_keeps_mount_points_as_given() {
    let w = TempDir::new("rt6");
    let t = tree(&w);
    let out = w.0.join("exact.pkg");

    // bundle: an empty mount point defaults per slot index (C++ contract).
    let images = vec![
        parse_image_spec(t.a.to_str().unwrap()),
        parse_image_spec(t.b.to_str().unwrap()),
    ];
    bundle(&t.boot, &images, &out, &opts()).expect("bundle");
    let mut f = std::fs::File::open(&out).unwrap();
    let m = tpkg::read_from(&mut f).unwrap();
    assert_eq!(m.slots[1].mount_point_str().unwrap(), default_mount(1));

    // bundle_exact: the empty mount point stays empty (the Ruby Stitcher's
    // semantics for a fat package's FORMAT_RUNTIME payload slot).
    let images = vec![
        parse_image_spec(&format!("{}:/app", t.a.display())),
        PackageImage {
            path: t.b.clone(),
            mount_point: String::new(),
            format_id: tpkg::TPKG_FORMAT_RUNTIME,
        },
    ];
    bundle_exact(&t.boot, &images, &out, &opts()).expect("bundle_exact");
    let mut f = std::fs::File::open(&out).unwrap();
    let m = tpkg::read_from(&mut f).unwrap();
    assert_eq!(m.slots[0].mount_point_str().unwrap(), "/app");
    assert_eq!(m.slots[1].mount_point_str().unwrap(), "");
    assert_eq!(m.slots[1].format_id, tpkg::TPKG_FORMAT_RUNTIME);
}
