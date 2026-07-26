//! v2 chain-of-trust trailer tests: golden byte-offset layout, round-trips
//! through both readers (in-memory + seekable), the canonical signed
//! region, and the structural rejections. v2 is new — no C vectors exist
//! yet; the layout asserted here is the one documented in the crate docs.

use std::io::Cursor;

use tpkg::*;

const PAYLOAD: u64 = 8192; // slot table offset in the golden build

fn digest(byte: u8) -> [u8; TPKG_SHA256_LEN] {
    [byte; TPKG_SHA256_LEN]
}

fn golden_v2_manifest() -> Manifest {
    let mut m = Manifest {
        version: TPKG_VERSION_2,
        package_flags: TPKG_FLAG_LEAN,
        launcher_abi: 3,
        ..Default::default()
    };
    m.set_runtime_ref(b"tebako-runtime-0.15.9-linux-arm64");
    m.slots
        .push(Slot::new(0, 4778, TPKG_FORMAT_DWARFS, "/__tebako__"));
    m.slots
        .push(Slot::new(4778, 1_048_576, TPKG_FORMAT_ZIP, "/data"));

    let mut v2 = V2Extension::default();
    v2.slot_digests[0] = digest(0x11);
    v2.slot_digests[1] = digest(0x22);
    v2.signer_keyid = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xF0, 0x0D];
    v2.signature = vec![0x5A; 130]; // stand-in OpenPGP packet bytes
    m.v2 = Some(v2);
    m
}

#[test]
fn v2_golden_layout_offsets() {
    let trailer = encode_trailer(&golden_v2_manifest(), PAYLOAD).unwrap();

    // 2*280 slot records + 166 header + 256 digests + 8 keyid + 4 siglen + 130 sig
    assert_eq!(trailer.len(), 2 * 280 + 166 + 256 + 8 + 4 + 130);

    let hbase = 2 * 280;
    // header magic + version (v1 codec: little-endian)
    assert_eq!(&trailer[hbase..hbase + 10], TPKG_MAGIC);
    assert_eq!(&trailer[hbase + 10..hbase + 14], &2u32.to_le_bytes());
    // slot table offset (LE, as v1)
    assert_eq!(&trailer[hbase + 22..hbase + 30], &PAYLOAD.to_le_bytes());

    // digests immediately after the header
    let xbase = hbase + 166;
    assert!(trailer[xbase..xbase + 32].iter().all(|&b| b == 0x11));
    assert!(trailer[xbase + 32..xbase + 64].iter().all(|&b| b == 0x22));
    // unused digest entries zeroed
    assert!(trailer[xbase + 64..xbase + 256].iter().all(|&b| b == 0));

    // signer keyid
    let kbase = xbase + 256;
    assert_eq!(
        &trailer[kbase..kbase + 8],
        &[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xF0, 0x0D]
    );

    // siglen BIG-ENDIAN at the very end, after the signature bytes
    let sbase = kbase + 8;
    assert!(trailer[sbase..sbase + 130].iter().all(|&b| b == 0x5A));
    let lbase = sbase + 130;
    assert_eq!(&trailer[lbase..lbase + 4], &130u32.to_be_bytes());
}

#[test]
fn v2_round_trip_both_readers() {
    let m = golden_v2_manifest();
    let mut image = vec![0u8; PAYLOAD as usize];
    image.extend_from_slice(&encode_trailer(&m, PAYLOAD).unwrap());

    let parsed = parse_trailer(&image).unwrap();
    assert_eq!(parsed, m);

    let mut cur = Cursor::new(&image);
    let parsed_io = read_from(&mut cur).unwrap();
    assert_eq!(parsed_io, m);

    let v2 = parsed.v2.as_ref().unwrap();
    assert_eq!(v2.slot_digest(0), Some(&digest(0x11)));
    assert_eq!(v2.slot_digest(1), Some(&digest(0x22)));
    assert_eq!(v2.slot_digest(2), Some(&digest(0x00)));
    assert_eq!(v2.signer_keyid_hex(), "deadbeefcafef00d");
    assert_eq!(v2.signature, vec![0x5A; 130]);
}

#[test]
fn v2_signed_region_is_the_contiguous_prefix() {
    let m = golden_v2_manifest();
    let trailer = encode_trailer(&m, PAYLOAD).unwrap();
    let region = v2_signed_region(&trailer).unwrap();
    // canonical = slot table + header + digests + keyid (no siglen, no sig)
    assert_eq!(region.len(), 2 * 280 + 166 + 256 + 8);
    assert_eq!(&region[..], &trailer[..region.len()]);
}

#[test]
fn v2_trailer_len_matches_encoding() {
    let m = golden_v2_manifest();
    let trailer = encode_trailer(&m, PAYLOAD).unwrap();
    assert_eq!(trailer_len(&m), trailer.len() as u64);
}

#[test]
fn v2_rejects_nonzero_digest_tail() {
    let mut m = golden_v2_manifest();
    m.v2.as_mut().unwrap().slot_digests[7] = digest(0xFF);
    assert_eq!(encode_trailer(&m, PAYLOAD), Err(TpkgError::Invalid));
}

#[test]
fn v2_rejects_empty_signature() {
    let mut m = golden_v2_manifest();
    m.v2.as_mut().unwrap().signature.clear();
    assert_eq!(encode_trailer(&m, PAYLOAD), Err(TpkgError::Invalid));
}

#[test]
fn v2_rejects_zero_keyid() {
    let mut m = golden_v2_manifest();
    m.v2.as_mut().unwrap().signer_keyid = [0; TPKG_KEYID_LEN];
    assert_eq!(encode_trailer(&m, PAYLOAD), Err(TpkgError::Invalid));
}

#[test]
fn v2_extension_without_version_2_is_rejected() {
    let mut m = golden_v2_manifest();
    m.version = TPKG_VERSION; // v1 header carrying an extension
    assert_eq!(encode_trailer(&m, PAYLOAD), Err(TpkgError::Invalid));
}

#[test]
fn unsupported_version_3_is_rejected() {
    let mut m = golden_v2_manifest();
    m.version = 3;
    assert_eq!(encode_trailer(&m, PAYLOAD), Err(TpkgError::Version));
}

#[test]
fn truncated_signature_fails_closed() {
    let m = golden_v2_manifest();
    let mut image = vec![0u8; PAYLOAD as usize];
    image.extend_from_slice(&encode_trailer(&m, PAYLOAD).unwrap());
    // drop 10 bytes from the middle of the signature: the trailing sig_len
    // probe then lands off the header and the file no longer reads as a
    // package at all — refused (fail closed), never silently accepted
    let sig_start = PAYLOAD as usize + 2 * 280 + 166 + 264 + 20;
    image.drain(sig_start..sig_start + 10);
    assert_eq!(parse_trailer(&image), Err(TpkgError::NoTrailer));
}

#[test]
fn tampered_digest_tail_is_rejected() {
    let m = golden_v2_manifest();
    let mut image = vec![0u8; PAYLOAD as usize];
    image.extend_from_slice(&encode_trailer(&m, PAYLOAD).unwrap());
    // flip a byte in an unused digest entry
    let hbase = PAYLOAD as usize + 2 * 280 + 166;
    image[hbase + 7 * 32] ^= 0xFF;
    assert_eq!(parse_trailer(&image), Err(TpkgError::Invalid));
}
