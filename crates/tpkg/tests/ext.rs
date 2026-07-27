//! Typed extension block tests (spec 02 §5b): byte layout against the
//! locked order [slot table][ext blocks…][v2 signing ext?][header],
//! round-trips through both readers, walk/insert/remove semantics, the
//! forward-compat rules (unknown types skipped by readers, rejected by
//! `validate_strict`), and fail-closed garbage handling.

use std::io::Cursor;

use tpkg::*;

const PAYLOAD: u64 = 4096; // slot table offset

fn one_slot_manifest() -> Manifest {
    let mut m = Manifest::default();
    m.slots.push(Slot::new(0, 100, TPKG_FORMAT_ZIP, "/m"));
    m
}

fn package_manifest_yaml() -> Vec<u8> {
    b"schema_version: 1\n\
      package: {name: metanorma, version: 1.2.3, producer: {tool: tebako-cli, tool_version: 0.16.0}, created: 2026-07-26T00:00:00Z}\n\
      entries:\n  - {name: metanorma, slot: 0, entrypoint: metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n"
        .to_vec()
}

fn manifest_with_blocks(blocks: Vec<ExtBlock>) -> Manifest {
    let mut m = one_slot_manifest();
    m.ext_blocks = blocks;
    m
}

fn image_with(m: &Manifest) -> Vec<u8> {
    let mut image = vec![0u8; PAYLOAD as usize];
    image.extend_from_slice(&encode_trailer(m, PAYLOAD).unwrap());
    image
}

#[test]
fn block_layout_byte_offsets() {
    let yaml = package_manifest_yaml();
    let m = manifest_with_blocks(vec![
        ExtBlock::new(TPKG_EXT_TYPE_PACKAGE_MANIFEST, yaml.clone()).unwrap(),
        ExtBlock::new(9, vec![0xAB; 3]).unwrap(), // unknown type, preserved
    ]);
    let trailer = encode_trailer(&m, PAYLOAD).unwrap();

    // [1 × 280 slot][u32be 2][u32be len][yaml][u32be 9][u32be 3][AB AB AB][header]
    let b0 = TPKG_SLOT_SIZE;
    assert_eq!(&trailer[b0..b0 + 4], &2u32.to_be_bytes());
    assert_eq!(&trailer[b0 + 4..b0 + 8], &(yaml.len() as u32).to_be_bytes());
    assert_eq!(
        &trailer[b0 + 8..b0 + 8 + yaml.len()],
        &yaml[..],
        "the YAML payload rides verbatim"
    );
    let b1 = b0 + 8 + yaml.len();
    assert_eq!(&trailer[b1..b1 + 4], &9u32.to_be_bytes());
    assert_eq!(&trailer[b1 + 4..b1 + 8], &3u32.to_be_bytes());
    assert_eq!(&trailer[b1 + 8..b1 + 11], &[0xAB; 3]);
    let hdr = b1 + 11;
    assert_eq!(&trailer[hdr..hdr + 10], TPKG_MAGIC);
    assert_eq!(trailer.len(), hdr + TPKG_HEADER_SIZE);
    assert_eq!(trailer_len(&m), trailer.len() as u64);
}

#[test]
fn round_trip_both_readers() {
    let m = manifest_with_blocks(vec![
        ExtBlock::new(TPKG_EXT_TYPE_PACKAGE_MANIFEST, package_manifest_yaml()).unwrap(),
        ExtBlock::new(7, vec![1, 2, 3, 4]).unwrap(),
    ]);
    let image = image_with(&m);

    let parsed = parse_trailer(&image).unwrap();
    assert_eq!(parsed, m);
    assert_eq!(parsed.ext_blocks.len(), 2);

    let mut cur = Cursor::new(&image);
    let parsed_io = read_from(&mut cur).unwrap();
    assert_eq!(parsed_io, m);

    // Re-encoding the parsed manifest reproduces the bytes (blocks included).
    let re = encode_trailer(&parsed, PAYLOAD).unwrap();
    assert_eq!(
        &image[PAYLOAD as usize..],
        &re[..],
        "parse -> encode is byte-identical"
    );
}

#[test]
fn empty_block_region_is_the_v1_layout() {
    // No blocks: parse reports none, and the trailer is the exact v1 shape
    // (this is what keeps the C golden vectors byte-identical).
    let m = one_slot_manifest();
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    assert!(parsed.ext_blocks.is_empty());
    assert_eq!(
        image.len() - PAYLOAD as usize,
        TPKG_SLOT_SIZE + TPKG_HEADER_SIZE
    );
}

#[test]
fn free_walk_and_encode_round_trip() {
    let blocks = vec![
        ExtBlock::new(0, vec![]).unwrap(), // zero-length payload is legal
        ExtBlock::new(2, b"payload".to_vec()).unwrap(),
        ExtBlock::new(u32::MAX, vec![0xFF; 17]).unwrap(),
    ];
    let bytes = encode_ext_blocks(&blocks).unwrap();
    assert_eq!(parse_ext_blocks(&bytes).unwrap(), blocks);
    assert!(parse_ext_blocks(&[]).unwrap().is_empty());
}

#[test]
fn walker_rejects_garbage() {
    // truncated block header: 1..=7 dangling bytes
    for n in 1..TPKG_EXT_HEADER_SIZE {
        assert_eq!(
            parse_ext_blocks(&vec![0u8; n]),
            Err(TpkgError::Invalid),
            "{n} dangling bytes"
        );
    }
    // length overruns the region
    let mut bad = Vec::new();
    bad.extend_from_slice(&2u32.to_be_bytes());
    bad.extend_from_slice(&100u32.to_be_bytes());
    bad.extend_from_slice(&[0u8; 10]);
    assert_eq!(parse_ext_blocks(&bad), Err(TpkgError::Invalid));

    // …also inside a full package image.
    let m = manifest_with_blocks(vec![ExtBlock::new(2, vec![0u8; 8]).unwrap()]);
    let mut image = image_with(&m);
    let len_at = PAYLOAD as usize + TPKG_SLOT_SIZE + 4;
    image[len_at..len_at + 4].copy_from_slice(&10_000u32.to_be_bytes());
    assert_eq!(parse_trailer(&image), Err(TpkgError::Invalid));

    // dangling bytes after the last block
    let m = manifest_with_blocks(vec![ExtBlock::new(2, vec![0u8; 3]).unwrap()]);
    let mut image = image_with(&m);
    // shrink the declared payload so 2 bytes dangle before the header
    let len_at = PAYLOAD as usize + TPKG_SLOT_SIZE + 4;
    image[len_at..len_at + 4].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(parse_trailer(&image), Err(TpkgError::Invalid));
}

#[test]
fn reserved_type_1_fails_closed() {
    // Construction and insertion are refused with the named error…
    assert!(matches!(
        ExtBlock::new(TPKG_EXT_TYPE_V2_SIGNING, vec![]),
        Err(ExtError::ReservedType)
    ));
    let mut m = one_slot_manifest();
    assert!(matches!(
        m.insert_ext_block(ExtBlock {
            block_type: TPKG_EXT_TYPE_V2_SIGNING,
            payload: vec![]
        }),
        Err(ExtError::ReservedType)
    ));

    // …a hand-placed type-1 block fails encode/parse (validate gate)…
    let m = manifest_with_blocks(vec![ExtBlock {
        block_type: TPKG_EXT_TYPE_V2_SIGNING,
        payload: vec![0u8; 4],
    }]);
    assert_eq!(m.validate(), Err(TpkgError::Invalid));
    assert_eq!(encode_trailer(&m, PAYLOAD), Err(TpkgError::Invalid));
    assert!(
        matches!(m.validate_strict(), Err(ExtError::ReservedType)),
        "strict validation names the reserved type"
    );

    // …and encoding a type-1 block by hand then parsing fails closed too.
    let mut m2 = one_slot_manifest();
    m2.ext_blocks = vec![ExtBlock::new(2, vec![1, 2, 3, 4]).unwrap()];
    let mut image = image_with(&m2);
    let t_at = PAYLOAD as usize + TPKG_SLOT_SIZE;
    image[t_at..t_at + 4].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(parse_trailer(&image), Err(TpkgError::Invalid));
}

#[test]
fn signed_flag_with_no_gap_fails_closed_not_panics() {
    // Hand-craft: SIGNED_V2 set but the slot table abuts the header (no
    // room even for sig_len). Historically this underflowed a subtraction
    // in the v2 extension parser — it must be TpkgError::Invalid, never
    // a panic.
    let mut image = image_with(&one_slot_manifest());
    let hdr = image.len() - TPKG_HEADER_SIZE;
    image[hdr + 14..hdr + 18].copy_from_slice(&TPKG_FLAG_SIGNED_V2.to_le_bytes());
    let crc = crc32(&image[hdr..hdr + 162]);
    image[hdr + 162..hdr + 166].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(parse_trailer(&image), Err(TpkgError::Invalid));
    let mut cur = Cursor::new(&image);
    assert_eq!(read_from(&mut cur), Err(TpkgError::Invalid));
}

#[test]
fn unknown_types_pass_readers_but_not_strict_validation() {
    let m = manifest_with_blocks(vec![
        ExtBlock::new(TPKG_EXT_TYPE_PACKAGE_MANIFEST, package_manifest_yaml()).unwrap(),
        ExtBlock::new(7, vec![0u8; 2]).unwrap(),
        ExtBlock::new(0, vec![]).unwrap(),
    ]);
    // readers: fine, preserved
    assert_eq!(m.validate(), Ok(()));
    let image = image_with(&m);
    assert_eq!(parse_trailer(&image).unwrap(), m);
    // strict validation: the FIRST unknown type is the named error
    match m.validate_strict() {
        Err(ExtError::UnknownType(7)) => {}
        other => panic!("expected UnknownType(7), got {other:?}"),
    }
    // with only the known type 2 present, strict validation passes
    let known_only = manifest_with_blocks(vec![ExtBlock::new(
        TPKG_EXT_TYPE_PACKAGE_MANIFEST,
        package_manifest_yaml(),
    )
    .unwrap()]);
    known_only.validate_strict().unwrap();
    // strict validation still runs the structural gate
    let mut broken = known_only.clone();
    broken.slots[0].format_id = 99;
    assert!(matches!(
        broken.validate_strict(),
        Err(ExtError::Trailer(TpkgError::Invalid))
    ));
}

#[test]
fn insert_replace_remove_semantics() {
    let mut m = one_slot_manifest();
    assert!(m.ext_block(2).is_none());

    m.insert_ext_block(ExtBlock::new(2, b"v1".to_vec()).unwrap())
        .unwrap();
    m.insert_ext_block(ExtBlock::new(3, b"other".to_vec()).unwrap())
        .unwrap();
    assert_eq!(m.ext_blocks.len(), 2);

    // same type replaces in place (block types are singletons in v1)
    m.insert_ext_block(ExtBlock::new(2, b"v2".to_vec()).unwrap())
        .unwrap();
    assert_eq!(m.ext_blocks.len(), 2);
    assert_eq!(m.ext_block(2).unwrap().payload, b"v2");
    assert_eq!(m.ext_block(3).unwrap().payload, b"other");

    assert!(!m.remove_ext_block(9));
    assert!(m.remove_ext_block(2));
    assert!(m.ext_block(2).is_none());
    assert!(m.remove_ext_block(3));
    assert!(m.ext_blocks.is_empty());
}

#[test]
fn package_manifest_accessors() {
    let text = String::from_utf8(package_manifest_yaml()).unwrap();
    let pm = PackageManifest::from_yaml(&text).unwrap();

    let mut m = one_slot_manifest();
    assert_eq!(m.package_manifest().unwrap(), None);

    m.set_package_manifest(&pm).unwrap();
    assert_eq!(m.package_manifest().unwrap(), Some(pm.clone()));
    // replacement, not duplication
    m.set_package_manifest(&pm).unwrap();
    assert_eq!(m.ext_blocks.len(), 1);
    // survives the wire
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    assert_eq!(parsed.package_manifest().unwrap(), Some(pm));
    // removal
    assert!(parsed
        .clone()
        .remove_ext_block(TPKG_EXT_TYPE_PACKAGE_MANIFEST));
}

#[test]
fn corrupt_package_manifest_block_is_a_named_error_not_a_parse_failure() {
    // The trailer itself is structurally fine; only the YAML payload is
    // broken — readers keep working, the interpreted accessor reports.
    let m = manifest_with_blocks(vec![ExtBlock::new(
        TPKG_EXT_TYPE_PACKAGE_MANIFEST,
        b"schema_version: 99\n".to_vec(),
    )
    .unwrap()]);
    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    assert!(parsed.package_manifest().is_err());

    let non_utf8 = manifest_with_blocks(vec![ExtBlock::new(
        TPKG_EXT_TYPE_PACKAGE_MANIFEST,
        vec![0xFF, 0xFE],
    )
    .unwrap()]);
    assert!(non_utf8.package_manifest().is_err());
}

// ---------------------------------------------------------------------
// with the v2 signing extension: blocks precede it, it keeps the tail
// ---------------------------------------------------------------------

fn signed_manifest(blocks: Vec<ExtBlock>) -> Manifest {
    let mut m = manifest_with_blocks(blocks);
    m.package_flags = TPKG_FLAG_SIGNED_V2;
    let mut v2 = V2Extension::default();
    v2.slot_digests[0] = [0x42; TPKG_SHA256_LEN];
    v2.signer_keyid = [1, 2, 3, 4, 5, 6, 7, 8];
    v2.signature = vec![0x5A; 130];
    m.v2 = Some(v2);
    m
}

#[test]
fn blocks_precede_the_signing_extension_which_keeps_the_tail() {
    let yaml = package_manifest_yaml();
    let m = signed_manifest(vec![ExtBlock::new(
        TPKG_EXT_TYPE_PACKAGE_MANIFEST,
        yaml.clone(),
    )
    .unwrap()]);
    let trailer = encode_trailer(&m, PAYLOAD).unwrap();

    // [slot][block][256 digests][8 keyid][130 sig][4 siglen][166 header]
    let b0 = TPKG_SLOT_SIZE;
    assert_eq!(&trailer[b0..b0 + 4], &2u32.to_be_bytes());
    let x0 = b0 + TPKG_EXT_HEADER_SIZE + yaml.len();
    assert!(trailer[x0..x0 + 32].iter().all(|&b| b == 0x42));
    assert_eq!(&trailer[x0 + 256..x0 + 264], &[1, 2, 3, 4, 5, 6, 7, 8]);
    let siglen_at = trailer.len() - TPKG_HEADER_SIZE - 4;
    assert_eq!(&trailer[siglen_at..siglen_at + 4], &130u32.to_be_bytes());

    let image = image_with(&m);
    let parsed = parse_trailer(&image).unwrap();
    assert_eq!(parsed, m);
    let mut cur = Cursor::new(&image);
    assert_eq!(read_from(&mut cur).unwrap(), m);
    assert_eq!(trailer_len(&m), trailer.len() as u64);
}

#[test]
fn signed_region_covers_the_blocks() {
    let yaml = package_manifest_yaml();
    let m = signed_manifest(vec![ExtBlock::new(
        TPKG_EXT_TYPE_PACKAGE_MANIFEST,
        yaml.clone(),
    )
    .unwrap()]);
    let trailer = encode_trailer(&m, PAYLOAD).unwrap();
    let region = v2_signed_region(&trailer).unwrap();
    // canonical = slot table || blocks || digests || keyid || header
    assert_eq!(
        region.len(),
        TPKG_SLOT_SIZE + TPKG_EXT_HEADER_SIZE + yaml.len() + 256 + 8 + TPKG_HEADER_SIZE
    );
    // the YAML payload sits inside the signed region, right after the table
    assert_eq!(
        &region[TPKG_SLOT_SIZE + TPKG_EXT_HEADER_SIZE
            ..TPKG_SLOT_SIZE + TPKG_EXT_HEADER_SIZE + yaml.len()],
        &yaml[..]
    );
    // …and the header closes the region (signature + sig_len excluded by
    // the length math above)
    assert_eq!(
        &region[region.len() - TPKG_HEADER_SIZE..region.len() - TPKG_HEADER_SIZE + 10],
        TPKG_MAGIC
    );
}

#[test]
fn tampered_block_breaks_the_walk_not_the_parser_state() {
    let m = signed_manifest(vec![ExtBlock::new(2, vec![0u8; 8]).unwrap()]);
    let mut image = image_with(&m);
    // inflate the block length so it swallows the v2 extension's sig_len
    // slot — the extension no longer fits its tail position: fail closed
    let len_at = PAYLOAD as usize + TPKG_SLOT_SIZE + 4;
    image[len_at..len_at + 4].copy_from_slice(&999u32.to_be_bytes());
    assert_eq!(parse_trailer(&image), Err(TpkgError::Invalid));

    let mut cur = Cursor::new(&image);
    assert_eq!(read_from(&mut cur), Err(TpkgError::Invalid));
}

#[test]
fn signed_without_blocks_is_byte_identical_to_pre_blocks_format() {
    // The v2 golden discipline: a signed package with no blocks encodes
    // exactly as before (blocks region empty, extension fills the gap).
    let m = signed_manifest(vec![]);
    let trailer = encode_trailer(&m, PAYLOAD).unwrap();
    assert_eq!(
        trailer.len(),
        TPKG_SLOT_SIZE + TPKG_V2_EXT_FIXED + 130 + TPKG_HEADER_SIZE
    );
    let image = image_with(&m);
    assert_eq!(parse_trailer(&image).unwrap(), m);
}
