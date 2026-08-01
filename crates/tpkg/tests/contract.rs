//! Spec-18 C6 wire tests: the contract declaration rides the type-2 block
//! as plain YAML keys (additive — pre-contract readers tolerate them as
//! unknown keys), the critical flag is the high bit of the block type
//! word (wire-compatible), and the reader-side gate (`verify_contract`,
//! exit 77) distinguishes "no block" (era 1) from "block present, era
//! mismatch".

use std::io::Cursor;

use tpkg::*;

const PAYLOAD: u64 = 4096; // slot table offset

fn one_slot_manifest() -> Manifest {
    let mut m = Manifest::default();
    m.slots.push(Slot::new(0, 100, TPKG_FORMAT_ZIP, "/m"));
    m
}

fn image_with(m: &Manifest) -> Vec<u8> {
    let mut image = vec![0u8; PAYLOAD as usize];
    image.extend_from_slice(&encode_trailer(m, PAYLOAD).unwrap());
    image
}

fn pre_era_package_manifest() -> PackageManifest {
    PackageManifest::from_yaml(
        "schema_version: 1\n\
         package: {name: metanorma, version: 1.2.3, producer: {tool: tebako-cli, tool_version: 0.16.0}, created: 2026-07-26T00:00:00Z}\n\
         entries:\n  - {name: metanorma, slot: 0, entrypoint: metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n",
    )
    .unwrap()
}

#[test]
fn a_pressed_block_carries_the_contract_and_still_validates_strict() {
    let mut m = one_slot_manifest();
    m.set_package_manifest(&pre_era_package_manifest()).unwrap();

    // The block's type word stays the plain base type 2 — the contract
    // rides as YAML keys, never as a new wire type, so pre-contract
    // readers tolerate it (unknown keys, era-1 compat).
    assert_eq!(m.ext_blocks.len(), 1);
    assert_eq!(m.ext_blocks[0].block_type, TPKG_EXT_TYPE_PACKAGE_MANIFEST);
    let text = String::from_utf8(m.ext_blocks[0].payload.clone()).unwrap();
    assert!(text.contains("contract_era: 2"), "{text}");
    assert!(text.contains("pressed_by: "), "{text}");
    assert!(text.contains("reader_era: 2"), "{text}");

    // Round-trip through the wire: parse + both gates.
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    parsed.validate_strict().unwrap();
    parsed.verify_contract().unwrap();
    let mut cur = Cursor::new(&image);
    read_from(&mut cur).unwrap().verify_contract().unwrap();
}

#[test]
fn pre_era_packages_refuse_as_era_1_not_as_mismatch() {
    // No block at all: era 1.
    let m = one_slot_manifest();
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    assert_eq!(parsed.verify_contract(), Err(ContractError::PreEra));

    // A block pressed by tebako < 0.16.1 (no contract keys): era 1 — the
    // typed error is PreEra ("no block"), never EraTooNew ("mismatch").
    let mut m = one_slot_manifest();
    let yaml = pre_era_package_manifest().to_yaml().unwrap(); // raw, no contract merge
    m.ext_blocks
        .push(ExtBlock::new(TPKG_EXT_TYPE_PACKAGE_MANIFEST, yaml.into_bytes()).unwrap());
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    let err = parsed.verify_contract().unwrap_err();
    assert_eq!(err, ContractError::PreEra);
    assert_eq!(err.exit_code(), EX_TEBAKO_CONTRACT_ERA);
    assert_eq!(err.exit_code(), 77);
    assert!(err.to_string().contains("pre-era package"), "{err}");
}

#[test]
fn era_mismatch_is_a_distinct_typed_path_from_pre_era() {
    let mut m = one_slot_manifest();
    m.ext_blocks
        .push(
            ExtBlock::new(
                TPKG_EXT_TYPE_PACKAGE_MANIFEST,
                b"schema_version: 1\ncontract_era: 3\npressed_by: 0.20.0\nreader_era: 3\npackage: {name: x, version: 1, producer: {tool: t, tool_version: 1}, created: now}\nentries:\n  - {name: x, slot: 0, entrypoint: x, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n".to_vec(),
            )
            .unwrap(),
        );
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    let err = parsed.verify_contract().unwrap_err();
    assert_eq!(
        err,
        ContractError::EraTooNew {
            package_era: 3,
            reader_era: TPKG_CONTRACT_ERA
        }
    );
    assert_ne!(
        err,
        ContractError::PreEra,
        "mismatch is never the pre-era path"
    );
    assert_eq!(err.exit_code(), 77);
}

#[test]
fn the_critical_flag_rides_the_high_bit_of_the_type_word() {
    let crit = ExtBlock::new_critical(9, vec![0xAB; 3]).unwrap();
    assert!(crit.is_critical());
    assert_eq!(crit.base_type(), 9);
    assert_eq!(crit.block_type, 9 | TPKG_EXT_FLAG_CRITICAL);

    let m = {
        let mut m = one_slot_manifest();
        m.ext_blocks.push(crit.clone());
        m
    };
    let trailer = encode_trailer(&m, PAYLOAD).unwrap();
    // [1 × 280 slot][u32be 9|CRIT][u32be 3][AB AB AB][header]
    let b0 = TPKG_SLOT_SIZE;
    assert_eq!(
        &trailer[b0..b0 + 4],
        &(9u32 | TPKG_EXT_FLAG_CRITICAL).to_be_bytes()
    );
    // …and parses back with the flag intact (readers carry it verbatim).
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    assert_eq!(parsed.ext_blocks, vec![crit]);
    // strict validation names the critical unknown separately…
    assert!(matches!(
        parsed.validate_strict(),
        Err(ExtError::CriticalType(9))
    ));
    // …and the open gate refuses it by name (S10).
    assert_eq!(
        parsed.verify_contract(),
        Err(ContractError::CriticalBlock(9))
    );
}

#[test]
fn non_critical_unknown_blocks_still_skip_everywhere() {
    let mut m = one_slot_manifest();
    m.set_package_manifest(&pre_era_package_manifest()).unwrap();
    m.ext_blocks.push(ExtBlock::new(9, vec![0xAB; 3]).unwrap());
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    // readers skip (invariant 7), the open gate accepts, strict names it
    parsed.verify_contract().unwrap();
    assert!(matches!(
        parsed.validate_strict(),
        Err(ExtError::UnknownType(9))
    ));
    // re-encode preserves the block byte-identically
    let re = encode_trailer(&parsed, PAYLOAD).unwrap();
    assert_eq!(&image[PAYLOAD as usize..], &re[..]);
}

#[test]
fn reserved_type_1_stays_reserved_with_the_critical_flag() {
    assert!(matches!(
        ExtBlock::new_critical(TPKG_EXT_TYPE_V2_SIGNING, vec![]),
        Err(ExtError::ReservedType)
    ));
    // a hand-placed critical type-1 block fails the structural gate
    let mut m = one_slot_manifest();
    m.ext_blocks.push(ExtBlock {
        block_type: TPKG_EXT_TYPE_V2_SIGNING | TPKG_EXT_FLAG_CRITICAL,
        payload: vec![0u8; 4],
    });
    assert_eq!(m.validate(), Err(TpkgError::Invalid));
}
