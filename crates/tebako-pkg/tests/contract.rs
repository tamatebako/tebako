//! The spec-18 C6 contract gate at the tebako-pkg layer: `bundle` emits
//! the declaration (contract_era/pressed_by/reader_era ride the type-2
//! block), `check_contract` surfaces the typed refusal, the CLI verbs
//! `validate` / `info --verify` exit 77 on pre-era or era-mismatch
//! packages, and `info --full` (lib) prints the contract card.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tebako_pkg::{bundle, check_contract, info_full, PackageImage, PackageOptions, SignRequest};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-pkg-contract-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

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
        let cand = target.join(profile).join("tebako-pkg");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tebako-pkg binary not built (run `cargo build -p tebako-pkg`)")
}

fn run(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn parts(dir: &Path) -> (PathBuf, PathBuf) {
    let bootstrap = dir.join("bootstrap.bin");
    let app = dir.join("app.img");
    std::fs::write(&bootstrap, b"BOOTSTRAP-BYTES").unwrap();
    std::fs::write(&app, b"the app image payload").unwrap();
    (bootstrap, app)
}

fn image(path: &Path) -> PackageImage {
    PackageImage {
        path: path.to_path_buf(),
        mount_point: "/app".to_string(),
        format_id: tpkg::TPKG_FORMAT_ZIP,
    }
}

const PRE_ERA_YAML: &str = "schema_version: 1\n\
     package: {name: metanorma, version: 1.2.3, producer: {tool: tebako-cli, tool_version: 0.16.0}, created: 2026-07-26T00:00:00Z}\n\
     entries:\n  - {name: metanorma, slot: 0, entrypoint: metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n";

fn opts() -> PackageOptions {
    PackageOptions {
        package_manifest: Some(tpkg::PackageManifest::from_yaml(PRE_ERA_YAML).unwrap()),
        ..Default::default()
    }
}

/// Replace the type-2 block's payload with `yaml` verbatim (tpkg surgery
/// below the tebako-pkg layer: truncate the trailer, append the modified
/// one) — the way to forge a pre-era or future-era package in-test.
fn replace_block_payload(pkg: &Path, yaml: &str) {
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pkg)
        .unwrap();
    let mut m = tpkg::read_from(&mut f).unwrap();
    let old_tlen = tpkg::trailer_len(&m); // the trailer as it sits in the file
    m.ext_blocks
        .retain(|b| b.base_type() != tpkg::TPKG_EXT_TYPE_PACKAGE_MANIFEST);
    m.ext_blocks.push(
        tpkg::ExtBlock::new(
            tpkg::TPKG_EXT_TYPE_PACKAGE_MANIFEST,
            yaml.as_bytes().to_vec(),
        )
        .unwrap(),
    );
    let file_len = f.metadata().unwrap().len();
    f.set_len(file_len - old_tlen).unwrap();
    tpkg::write_to(&mut f, &m).unwrap();
}

#[test]
fn bundle_emits_the_contract_block() {
    let dir = scratch("emit");
    let (bootstrap, app) = parts(&dir);
    let pkg = dir.join("out.pkg");
    bundle(&bootstrap, &[image(&app)], &pkg, &opts()).unwrap();

    let mut f = std::fs::File::open(&pkg).unwrap();
    let m = tpkg::read_from(&mut f).unwrap();
    // the block carries the declaration…
    let c = m.package_contract().unwrap().unwrap();
    assert_eq!(c.contract_era, tpkg::TPKG_CONTRACT_ERA);
    assert!(!c.pressed_by.is_empty());
    assert_eq!(c.reader_era, tpkg::TPKG_CONTRACT_ERA);
    // …and passes its own gate
    m.verify_contract().unwrap();
    assert!(check_contract(&pkg).unwrap().is_none());

    // the card shows in the lib --full section
    let full = info_full(&pkg).unwrap();
    assert!(
        full.contains(&format!(
            "  contract: era {}, pressed_by ",
            tpkg::TPKG_CONTRACT_ERA
        )),
        "{full}"
    );
}

#[test]
fn check_contract_distinguishes_the_refusal_classes() {
    let dir = scratch("classes");
    let (bootstrap, app) = parts(&dir);

    // no block at all: era 1
    let pkg = dir.join("plain.pkg");
    bundle(&bootstrap, &[image(&app)], &pkg, &PackageOptions::default()).unwrap();
    assert_eq!(
        check_contract(&pkg).unwrap(),
        Some(tpkg::ContractError::PreEra)
    );

    // a block pressed by tebako < 0.16.1 (no contract keys): era 1
    let pkg = dir.join("pre-era.pkg");
    bundle(&bootstrap, &[image(&app)], &pkg, &opts()).unwrap();
    replace_block_payload(&pkg, PRE_ERA_YAML);
    assert_eq!(
        check_contract(&pkg).unwrap(),
        Some(tpkg::ContractError::PreEra)
    );
    let full = info_full(&pkg).unwrap();
    assert!(
        full.contains("  contract: none (pre-era package — spec 18 C6)\n"),
        "{full}"
    );

    // era mismatch: the distinct typed path (never the pre-era one)
    replace_block_payload(
        &pkg,
        "schema_version: 1\ncontract_era: 3\npressed_by: 0.20.0\nreader_era: 3\n",
    );
    let err = check_contract(&pkg).unwrap().unwrap();
    assert_eq!(
        err,
        tpkg::ContractError::EraTooNew {
            package_era: 3,
            reader_era: tpkg::TPKG_CONTRACT_ERA
        }
    );
    assert_eq!(err.exit_code(), 77);

    // an unknown CRITICAL block: refused, never skipped (S10)
    let pkg = dir.join("critical.pkg");
    bundle(&bootstrap, &[image(&app)], &pkg, &opts()).unwrap();
    {
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pkg)
            .unwrap();
        let mut m = tpkg::read_from(&mut f).unwrap();
        let old_tlen = tpkg::trailer_len(&m); // the trailer as it sits in the file
        m.ext_blocks
            .push(tpkg::ExtBlock::new_critical(9, b"future".to_vec()).unwrap());
        let file_len = f.metadata().unwrap().len();
        f.set_len(file_len - old_tlen).unwrap();
        tpkg::write_to(&mut f, &m).unwrap();
    }
    assert_eq!(
        check_contract(&pkg).unwrap(),
        Some(tpkg::ContractError::CriticalBlock(9))
    );

    // not a package at all: not a contract question here
    let junk = dir.join("junk.bin");
    std::fs::write(&junk, b"no trailer here").unwrap();
    assert!(check_contract(&junk).unwrap().is_none());
}

#[test]
fn validate_cli_exits_77_for_pre_era_and_mismatch() {
    let dir = scratch("cli77");
    let (bootstrap, app) = parts(&dir);
    let pkg = dir.join("pre-era.pkg");
    bundle(&bootstrap, &[image(&app)], &pkg, &opts()).unwrap();
    replace_block_payload(&pkg, PRE_ERA_YAML);

    let (rc, _, err) = run(&["validate", pkg.to_str().unwrap()], &dir);
    assert_eq!(rc, 77, "{err}");
    assert!(
        err.contains("pre-era package — re-press with tebako ≥ 0.16.1 or run it with tebako v1"),
        "{err}"
    );

    // info --verify shares the gate
    let (rc, _, err) = run(&["info", "--verify", pkg.to_str().unwrap()], &dir);
    assert_eq!(rc, 77, "{err}");
    assert!(err.contains("pre-era package"), "{err}");

    // era mismatch: both eras printed
    replace_block_payload(
        &pkg,
        "schema_version: 1\ncontract_era: 3\npressed_by: 0.20.0\nreader_era: 2\n",
    );
    let (rc, _, err) = run(&["validate", pkg.to_str().unwrap()], &dir);
    assert_eq!(rc, 77, "{err}");
    assert!(
        err.contains("package from a newer tebako (era 3) — upgrade your tebako (speaks era 2)"),
        "{err}"
    );
}

#[test]
fn a_package_pressed_now_passes_the_cli_gate() {
    let dir = scratch("clipass");
    let (bootstrap, app) = parts(&dir);
    let pkg = dir.join("current.pkg");
    bundle(&bootstrap, &[image(&app)], &pkg, &opts()).unwrap();
    // The gate passes; the spec-15 verify path then owns the verdict
    // (the garbage slot bytes fail its payload checks, never the contract
    // stage) — anything but 77 proves the stage ordering.
    let (rc, _, _) = run(&["validate", pkg.to_str().unwrap()], &dir);
    assert_ne!(rc, 77, "a declared package must pass the contract stage");
}

#[test]
fn signed_packages_carry_the_contract_inside_the_signed_region() {
    let home = scratch("signed-home");
    std::env::set_var("TEBAKO_HOME", &home);
    let dir = home.join("work");
    std::fs::create_dir_all(&dir).unwrap();
    let (bootstrap, app) = parts(&dir);
    let pkg = dir.join("signed.pkg");
    let mut o = opts();
    o.sign = SignRequest::PressLocal;
    bundle(&bootstrap, &[image(&app)], &pkg, &o).unwrap();

    let mut f = std::fs::File::open(&pkg).unwrap();
    let m = tpkg::read_from(&mut f).unwrap();
    assert!(m.v2.is_some(), "bundled signed");
    m.verify_contract().unwrap();
    // the contract keys sit inside the canonical signed region
    let tlen = tpkg::trailer_len(&m) as usize;
    let bytes = std::fs::read(&pkg).unwrap();
    let trailer = &bytes[bytes.len() - tlen..];
    let region = tpkg::v2_signed_region(trailer).unwrap();
    let needle = format!("contract_era: {}", tpkg::TPKG_CONTRACT_ERA);
    assert!(
        region.windows(needle.len()).any(|w| w == needle.as_bytes()),
        "the contract declaration is covered by the signature"
    );
    std::env::remove_var("TEBAKO_HOME");
}
