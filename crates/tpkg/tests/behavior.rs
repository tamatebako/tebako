//! Behavioral tests: absent-vs-corrupt discrimination, bounds, validation,
//! and the error-code surface — all mirroring the C implementation's
//! semantics (and its `test_tpkg` expectations).

use tpkg::*;

fn one_slot_manifest() -> Manifest {
    let mut m = Manifest::default();
    m.slots.push(Slot::new(0, 100, TPKG_FORMAT_ZIP, "/m"));
    m
}

fn image_with(m: &Manifest, payload: usize) -> Vec<u8> {
    let mut image = vec![0u8; payload];
    image.extend_from_slice(&encode_trailer(m, payload as u64).unwrap());
    image
}

#[test]
fn no_trailer_when_prefix_absent() {
    // A classic bundle: arbitrary bytes, no "TEBA" in the header window.
    let image = vec![0x5Au8; 4096];
    assert_eq!(parse_trailer(&image), Err(TpkgError::NoTrailer));

    // Too small to hold a header at all.
    let tiny = vec![0u8; TPKG_HEADER_SIZE - 1];
    assert_eq!(parse_trailer(&tiny), Err(TpkgError::NoTrailer));
}

#[test]
fn magic_error_when_prefix_present_but_magic_broken() {
    let mut image = image_with(&one_slot_manifest(), 128);
    // Corrupt one magic byte (keep the "TEBA" prefix intact).
    let hdr = image.len() - TPKG_HEADER_SIZE;
    image[hdr + 4] = b'X';
    assert_eq!(parse_trailer(&image), Err(TpkgError::Magic));
}

#[test]
fn crc_error_when_header_corrupted() {
    let mut image = image_with(&one_slot_manifest(), 128);
    // Flip a version byte: magic still OK, crc no longer matches.
    let hdr = image.len() - TPKG_HEADER_SIZE;
    image[hdr + 10] ^= 0xFF;
    assert_eq!(parse_trailer(&image), Err(TpkgError::Crc));
}

#[test]
fn version_error_for_unsupported_version() {
    let mut m = one_slot_manifest();
    m.version = 2;
    // Encode by hand (encode_trailer validates, so build the bytes manually:
    // patch a freshly encoded trailer's version field and fix the crc).
    let mut trailer = encode_trailer(&one_slot_manifest(), 0).unwrap();
    let hdr = trailer.len() - TPKG_HEADER_SIZE;
    trailer[hdr + 10..hdr + 14].copy_from_slice(&2u32.to_le_bytes());
    let crc = crc32(&trailer[hdr..hdr + 162]);
    trailer[hdr + 162..hdr + 166].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(parse_trailer(&trailer), Err(TpkgError::Version));
    m.version = 1; // keep clippy quiet about unused mutation intent
}

#[test]
fn slots_error_for_bad_slot_count() {
    let mut trailer = encode_trailer(&one_slot_manifest(), 0).unwrap();
    let hdr = trailer.len() - TPKG_HEADER_SIZE;
    trailer[hdr + 18..hdr + 22].copy_from_slice(&9u32.to_le_bytes()); // > MAX_SLOTS
    let crc = crc32(&trailer[hdr..hdr + 162]);
    trailer[hdr + 162..hdr + 166].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(parse_trailer(&trailer), Err(TpkgError::Slots));

    trailer[hdr + 18..hdr + 22].copy_from_slice(&0u32.to_le_bytes());
    let crc = crc32(&trailer[hdr..hdr + 162]);
    trailer[hdr + 162..hdr + 166].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(parse_trailer(&trailer), Err(TpkgError::Slots));
}

#[test]
fn bounds_error_when_table_outside_file() {
    let mut trailer = encode_trailer(&one_slot_manifest(), 0).unwrap();
    let hdr = trailer.len() - TPKG_HEADER_SIZE;
    trailer[hdr + 22..hdr + 30].copy_from_slice(&1_000_000u64.to_le_bytes());
    let crc = crc32(&trailer[hdr..hdr + 162]);
    trailer[hdr + 162..hdr + 166].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(parse_trailer(&trailer), Err(TpkgError::Bounds));
}

#[test]
fn validate_rejects_bad_manifests() {
    // slot_count == 0
    assert_eq!(Manifest::default().validate(), Err(TpkgError::Slots));

    // too many slots
    let mut m = Manifest::default();
    for _ in 0..=TPKG_MAX_SLOTS {
        m.slots.push(Slot::new(0, 1, TPKG_FORMAT_ZIP, "/m"));
    }
    assert_eq!(m.validate(), Err(TpkgError::Slots));

    // bad version
    let mut m = one_slot_manifest();
    m.version = 99;
    assert_eq!(m.validate(), Err(TpkgError::Version));

    // format_id out of range
    let mut m = one_slot_manifest();
    m.slots[0].format_id = TPKG_FORMAT_RUNTIME + 1;
    assert_eq!(m.validate(), Err(TpkgError::Invalid));

    // offset+size overflow
    let mut m = one_slot_manifest();
    m.slots[0].offset = u64::MAX - 10;
    m.slots[0].size = 20;
    assert_eq!(m.validate(), Err(TpkgError::Invalid));

    // mount_point without NUL termination
    let mut m = one_slot_manifest();
    m.slots[0].mount_point = [b'x'; TPKG_MOUNT_POINT_LEN];
    assert_eq!(m.validate(), Err(TpkgError::Invalid));

    // runtime_ref without NUL termination
    let mut m = one_slot_manifest();
    m.runtime_ref = [b'y'; TPKG_RUNTIME_REF_LEN];
    assert_eq!(m.validate(), Err(TpkgError::Invalid));
}

#[test]
fn error_codes_and_strings_match_c() {
    // TPKG_ERR_* numbering and tpkg_strerror() strings, 1:1 with the C header.
    assert_eq!(TpkgError::NoTrailer.code(), 1);
    assert_eq!(TpkgError::Magic.code(), 2);
    assert_eq!(TpkgError::Crc.code(), 3);
    assert_eq!(TpkgError::Io.code(), 4);
    assert_eq!(TpkgError::Bounds.code(), 5);
    assert_eq!(TpkgError::Slots.code(), 6);
    assert_eq!(TpkgError::Invalid.code(), 7);
    assert_eq!(TpkgError::Arg.code(), 8);
    assert_eq!(TpkgError::Version.code(), 9);

    assert_eq!(strerror(0), "success");
    assert_eq!(strerror(1), "no tpkg manifest trailer present");
    assert_eq!(strerror(2), "corrupt tpkg trailer magic");
    assert_eq!(strerror(3), "tpkg trailer header crc32 mismatch");
    assert_eq!(strerror(4), "tpkg i/o error");
    assert_eq!(strerror(5), "tpkg slot table out of file bounds");
    assert_eq!(
        strerror(6),
        "tpkg slot count out of range (1..TPKG_MAX_SLOTS)"
    );
    assert_eq!(strerror(7), "invalid tpkg manifest structure");
    assert_eq!(strerror(8), "invalid tpkg argument");
    assert_eq!(strerror(9), "unsupported tpkg manifest version");
    assert_eq!(strerror(42), "unknown tpkg error");

    assert_eq!(
        TpkgError::Crc.to_string(),
        "tpkg trailer header crc32 mismatch"
    );
}

#[test]
fn table_may_sit_at_odd_offset() {
    // The slot table (and payload) may sit at any — including odd — offset.
    let m = one_slot_manifest();
    let image = image_with(&m, 1023);
    assert_eq!(parse_trailer(&image).unwrap(), m);
}
