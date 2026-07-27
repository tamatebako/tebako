//! Extension-block integration tests (spec 02 §5b / spec 03 §6): `bundle`
//! embeds the L2 package manifest as the type-2 block, `info --full`
//! prints the package section, and the rewrite operations
//! (insert-image / remove-image / set-runtime) carry the block through the
//! atomic rewrite path — unsigned AND signed (the block sits inside the
//! canonical signed region). The env-touching signing tests serialize on
//! the same lock discipline as signed.rs.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use tebako_pkg::{
    bundle, info, info_full, insert_image, remove_image, set_runtime, PackageImage, PackageOptions,
    SignRequest,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-pkg-ext-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn parts(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let bootstrap = dir.join("bootstrap.bin");
    let app = dir.join("app.img");
    let extra = dir.join("extra.img");
    std::fs::write(&bootstrap, b"BOOTSTRAP-BYTES").unwrap();
    std::fs::write(&app, b"the app image payload").unwrap();
    std::fs::write(&extra, b"the extra image payload").unwrap();
    (bootstrap, app, extra)
}

fn image(path: &Path, mount: &str) -> PackageImage {
    PackageImage {
        path: path.to_path_buf(),
        mount_point: mount.to_string(),
        format_id: tpkg::TPKG_FORMAT_ZIP,
    }
}

fn package_manifest() -> tpkg::PackageManifest {
    tpkg::PackageManifest::from_yaml(
        "schema_version: 1\n\
         package: {name: metanorma, version: 1.2.3, producer: {tool: tebako-cli, tool_version: 0.16.0}, created: 2026-07-26T00:00:00Z}\n\
         entries:\n  - {name: metanorma, slot: 0, entrypoint: metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n",
    )
    .unwrap()
}

fn opts(sign: SignRequest) -> PackageOptions {
    PackageOptions {
        runtime_ref: "ruby@3.4.2;tebako=0.15.9".to_string(),
        package_flags: tpkg::TPKG_FLAG_LEAN,
        launcher_abi: 1,
        sign,
        package_manifest: Some(package_manifest()),
    }
}

fn read_manifest(pkg: &Path) -> tpkg::Manifest {
    let mut f = std::fs::File::open(pkg).unwrap();
    tpkg::read_from(&mut f).expect("parse trailer")
}

#[test]
fn bundle_embeds_the_package_manifest_block() {
    let dir = scratch("embed");
    let (bootstrap, app, _) = parts(&dir);
    let pkg = dir.join("out.pkg");
    bundle(
        &bootstrap,
        &[image(&app, "/app")],
        &pkg,
        &opts(SignRequest::None),
    )
    .unwrap();

    let m = read_manifest(&pkg);
    assert_eq!(m.ext_blocks.len(), 1);
    assert_eq!(
        m.ext_blocks[0].block_type,
        tpkg::TPKG_EXT_TYPE_PACKAGE_MANIFEST
    );
    assert_eq!(m.package_manifest().unwrap(), Some(package_manifest()));
    m.validate_strict().unwrap();
}

#[test]
fn bundle_without_manifest_keeps_the_exact_v1_shape() {
    let dir = scratch("plain");
    let (bootstrap, app, _) = parts(&dir);
    let pkg = dir.join("out.pkg");
    let mut o = opts(SignRequest::None);
    o.package_manifest = None;
    bundle(&bootstrap, &[image(&app, "/app")], &pkg, &o).unwrap();

    let m = read_manifest(&pkg);
    assert!(m.ext_blocks.is_empty());
    assert_eq!(m.package_manifest().unwrap(), None);
    // v1 trailer: slot table abuts the header — nothing between.
    let size = std::fs::metadata(&pkg).unwrap().len();
    let payload =
        std::fs::metadata(&bootstrap).unwrap().len() + std::fs::metadata(&app).unwrap().len();
    assert_eq!(
        size,
        payload + tpkg::TPKG_SLOT_SIZE as u64 + tpkg::TPKG_HEADER_SIZE as u64
    );
}

#[test]
fn bundle_rejects_entries_pointing_past_the_slots() {
    let dir = scratch("oob");
    let (bootstrap, app, _) = parts(&dir);
    let pm = tpkg::PackageManifest::from_yaml(
        "schema_version: 1\n\
         package: {name: x, version: 1, producer: {tool: t, tool_version: 1}, created: now}\n\
         entries:\n  - {name: x, slot: 1, entrypoint: x, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n",
    )
    .unwrap();
    let mut o = opts(SignRequest::None);
    o.package_manifest = Some(pm);
    let e = bundle(&bootstrap, &[image(&app, "/app")], &dir.join("x.pkg"), &o).unwrap_err();
    assert!(
        e.contains("references slot 1 but the package has 1 slot(s)"),
        "{e}"
    );
}

#[test]
fn info_full_prints_the_package_section() {
    let dir = scratch("info");
    let (bootstrap, app, _) = parts(&dir);
    let pkg = dir.join("out.pkg");
    bundle(
        &bootstrap,
        &[image(&app, "/app")],
        &pkg,
        &opts(SignRequest::None),
    )
    .unwrap();

    let full = info_full(&pkg).unwrap();
    assert!(
        full.contains("Package manifest: schema v1 (ext block type 2,"),
        "{full}"
    );
    assert!(full.contains("package: metanorma 1.2.3"), "{full}");
    assert!(
        full.contains(
            "[0] metanorma -> slot 0, entrypoint metanorma, runtime ruby@3.4.2;tebako=0.15.9"
        ),
        "{full}"
    );

    // the default dump is untouched (parity) — no package section
    let plain = info(&pkg).unwrap();
    assert!(!plain.contains("Package manifest:"), "{plain}");

    // --full on a block-less package says so explicitly
    let plain_pkg = dir.join("plain.pkg");
    let mut o = opts(SignRequest::None);
    o.package_manifest = None;
    bundle(&bootstrap, &[image(&app, "/app")], &plain_pkg, &o).unwrap();
    let full = info_full(&plain_pkg).unwrap();
    assert!(
        full.contains("Package manifest: none (v1 package)"),
        "{full}"
    );
}

#[test]
fn rewrites_preserve_the_block_unsigned() {
    let dir = scratch("rewrite");
    let (bootstrap, app, extra) = parts(&dir);
    let pkg = dir.join("out.pkg");
    bundle(
        &bootstrap,
        &[image(&app, "/app"), image(&extra, "/extra")],
        &pkg,
        &opts(SignRequest::None),
    )
    .unwrap();

    insert_image(&pkg, &extra, "/third").unwrap();
    let m = read_manifest(&pkg);
    assert_eq!(m.slots.len(), 3);
    assert_eq!(m.package_manifest().unwrap(), Some(package_manifest()));

    remove_image(&pkg, 2).unwrap();
    let m = read_manifest(&pkg);
    assert_eq!(m.slots.len(), 2);
    assert_eq!(m.package_manifest().unwrap(), Some(package_manifest()));

    let new_boot = dir.join("new-boot.bin");
    std::fs::write(&new_boot, b"NEW-BOOTSTRAP").unwrap();
    set_runtime(&pkg, &new_boot).unwrap();
    let m = read_manifest(&pkg);
    assert_eq!(m.package_manifest().unwrap(), Some(package_manifest()));
    m.validate_strict().unwrap();
}

#[test]
fn unknown_block_types_survive_rewrites() {
    // Forward-compat end to end: a block this build does not know rides
    // through insert-image verbatim.
    let dir = scratch("unknown");
    let (bootstrap, app, extra) = parts(&dir);
    let pkg = dir.join("out.pkg");
    bundle(
        &bootstrap,
        &[image(&app, "/app")],
        &pkg,
        &opts(SignRequest::None),
    )
    .unwrap();

    // hand-place an unknown block type via the tpkg API (surgery below
    // the tebako-pkg layer: truncate the trailer, append the extended one)
    let orig = read_manifest(&pkg);
    let old_tlen = tpkg::trailer_len(&orig);
    let mut m0 = orig.clone();
    m0.insert_ext_block(tpkg::ExtBlock::new(7, b"future".to_vec()).unwrap())
        .unwrap();
    {
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pkg)
            .unwrap();
        let file_len = f.metadata().unwrap().len();
        f.set_len(file_len - old_tlen).unwrap();
        tpkg::write_to(&mut f, &m0).unwrap();
    }
    let m0 = read_manifest(&pkg);
    assert_eq!(m0.ext_blocks.len(), 2);

    insert_image(&pkg, &extra, "/second").unwrap();
    let m = read_manifest(&pkg);
    assert_eq!(m.ext_blocks.len(), 2);
    assert_eq!(m.ext_block(7).unwrap().payload, b"future");
    assert_eq!(m.ext_blocks[0], m0.ext_blocks[0]);
    // strict validation names the unknown type
    assert!(matches!(
        m.validate_strict(),
        Err(tpkg::ExtError::UnknownType(7))
    ));
}

#[test]
fn signed_rewrites_preserve_the_block_and_the_signing_state() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let home = std::env::temp_dir().join(format!("tebako-pkg-ext-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("TEBAKO_HOME", &home);

    let dir = home.join("work");
    std::fs::create_dir_all(&dir).unwrap();
    let (bootstrap, app, extra) = parts(&dir);
    let pkg = dir.join("signed.pkg");
    bundle(
        &bootstrap,
        &[image(&app, "/app")],
        &pkg,
        &opts(SignRequest::PressLocal),
    )
    .unwrap();

    let m = read_manifest(&pkg);
    assert!(m.v2.is_some(), "bundled signed");
    assert_eq!(m.package_manifest().unwrap(), Some(package_manifest()));

    // insert rewrites (and re-signs) the package: the block survives and
    // the package is still v2-signed over the block's bytes.
    insert_image(&pkg, &extra, "/second").unwrap();
    let m = read_manifest(&pkg);
    assert_eq!(m.slots.len(), 2);
    assert!(m.v2.is_some(), "still signed after the rewrite");
    assert_eq!(m.package_manifest().unwrap(), Some(package_manifest()));

    // the signature covers the block: the canonical region contains it
    let tlen = tpkg::trailer_len(&m) as usize;
    let bytes = std::fs::read(&pkg).unwrap();
    let trailer = &bytes[bytes.len() - tlen..];
    let region = tpkg::v2_signed_region(trailer).unwrap();
    let yaml = m.ext_blocks[0].payload.clone();
    assert!(
        region.windows(yaml.len()).any(|w| w == &yaml[..]),
        "the type-2 block sits inside the canonical signed region"
    );

    std::env::remove_var("TEBAKO_HOME");
    std::fs::remove_dir_all(&home).ok();
}
