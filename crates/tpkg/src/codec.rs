//! The wire codec: encode/parse the trailer, byte-exact with the C
//! implementation's `tpkg_write_fd()`/`tpkg_read_mem()`.

use crate::error::TpkgError;
use crate::model::{put_str, Manifest, Slot, V2Extension};
use crate::{crc32, off, rec};
use crate::{
    TPKG_DIGESTS_SIZE, TPKG_FLAG_SIGNED_V2, TPKG_HEADER_SIZE, TPKG_KEYID_LEN, TPKG_MAGIC,
    TPKG_MAGIC_LEN, TPKG_MAGIC_PREFIX_LEN, TPKG_MAX_SLOTS, TPKG_MOUNT_POINT_LEN,
    TPKG_RUNTIME_REF_LEN, TPKG_SHA256_LEN, TPKG_SIGLEN_SIZE, TPKG_SIG_MAX, TPKG_SLOT_SIZE,
    TPKG_V2_EXT_FIXED, TPKG_VERSION,
};

fn get32(p: &[u8]) -> u32 {
    u32::from(p[0]) | (u32::from(p[1]) << 8) | (u32::from(p[2]) << 16) | (u32::from(p[3]) << 24)
}

fn get64(p: &[u8]) -> u64 {
    let mut v = 0u64;
    for (i, &b) in p.iter().take(8).enumerate() {
        v |= u64::from(b) << (8 * i);
    }
    v
}

fn put32(p: &mut [u8], v: u32) {
    p[0] = (v & 0xFF) as u8;
    p[1] = ((v >> 8) & 0xFF) as u8;
    p[2] = ((v >> 16) & 0xFF) as u8;
    p[3] = ((v >> 24) & 0xFF) as u8;
}

fn put64(p: &mut [u8], v: u64) {
    for (i, b) in p.iter_mut().take(8).enumerate() {
        *b = ((v >> (8 * i)) & 0xFF) as u8;
    }
}

/// v2-extension numerics are big-endian (only the sig_len field today).
fn get32be(p: &[u8]) -> u32 {
    (u32::from(p[0]) << 24) | (u32::from(p[1]) << 16) | (u32::from(p[2]) << 8) | u32::from(p[3])
}

fn put32be(p: &mut [u8], v: u32) {
    p[0] = ((v >> 24) & 0xFF) as u8;
    p[1] = ((v >> 16) & 0xFF) as u8;
    p[2] = ((v >> 8) & 0xFF) as u8;
    p[3] = (v & 0xFF) as u8;
}

/// Lowercase hex rendering (signer keyid display).
pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

/// Serialize the slot table + v2 extension (when present) + trailer
/// header for `manifest`, placing the slot table at absolute file offset
/// `slot_table_offset` (i.e. the current EOF when appending). Returns
/// `slot_count * TPKG_SLOT_SIZE + TPKG_HEADER_SIZE` bytes for an unsigned
/// manifest, plus `TPKG_V2_EXT_FIXED + signature.len()` more for a signed
/// (TPKG_FLAG_SIGNED_V2) one.
///
/// The manifest is validated first; nothing is produced for a rejected
/// manifest (mirrors the C writer appending nothing).
pub fn encode_trailer(manifest: &Manifest, slot_table_offset: u64) -> Result<Vec<u8>, TpkgError> {
    manifest.validate()?;

    let ext_len = manifest
        .v2
        .as_ref()
        .map_or(0, |v2| TPKG_V2_EXT_FIXED + v2.signature.len());
    let mut out = vec![0u8; manifest.slots.len() * TPKG_SLOT_SIZE + TPKG_HEADER_SIZE + ext_len];

    // slot table
    for (i, slot) in manifest.slots.iter().enumerate() {
        let base = i * TPKG_SLOT_SIZE;
        let r = &mut out[base..base + TPKG_SLOT_SIZE];
        put64(&mut r[rec::OFFSET..], slot.offset);
        put64(&mut r[rec::SIZE..], slot.size);
        put32(&mut r[rec::FORMAT..], slot.format_id);
        put32(&mut r[rec::FLAGS..], slot.flags);
        put_str(
            &mut r[rec::MOUNT..rec::MOUNT + TPKG_MOUNT_POINT_LEN],
            &slot.mount_point,
        );
    }

    // v2 chain-of-trust extension (between the slot table and the header;
    // sig_len big-endian, immediately before the header)
    let hbase = manifest.slots.len() * TPKG_SLOT_SIZE + ext_len;
    if let Some(v2) = &manifest.v2 {
        let xbase = manifest.slots.len() * TPKG_SLOT_SIZE;
        let sig_len = v2.signature.len();
        let x = &mut out[xbase..];
        for (i, d) in v2.slot_digests.iter().enumerate() {
            x[i * TPKG_SHA256_LEN..(i + 1) * TPKG_SHA256_LEN].copy_from_slice(d);
        }
        x[TPKG_DIGESTS_SIZE..TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN].copy_from_slice(&v2.signer_keyid);
        let sig_base = TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN;
        x[sig_base..sig_base + sig_len].copy_from_slice(&v2.signature);
        put32be(&mut x[sig_base + sig_len..], sig_len as u32);
    }

    // trailer header (at EOF, exactly as v1)
    let hdr = &mut out[hbase..hbase + TPKG_HEADER_SIZE];
    hdr[off::MAGIC..off::MAGIC + TPKG_MAGIC_LEN].copy_from_slice(TPKG_MAGIC);
    put32(&mut hdr[off::VERSION..], manifest.version);
    put32(&mut hdr[off::PACKAGE_FLAGS..], manifest.package_flags);
    put32(&mut hdr[off::SLOT_COUNT..], manifest.slots.len() as u32);
    put64(&mut hdr[off::TABLE..], slot_table_offset);
    put_str(
        &mut hdr[off::RUNTIME_REF..off::RUNTIME_REF + TPKG_RUNTIME_REF_LEN],
        &manifest.runtime_ref,
    );
    put32(&mut hdr[off::LAUNCHER_ABI..], manifest.launcher_abi);
    let crc = crc32(&hdr[..off::CRC32]);
    put32(&mut hdr[off::CRC32..], crc);

    Ok(out)
}

/// Parse state shared by the in-memory and i/o readers: checks the header
/// window and returns header fields of interest.
pub(crate) struct HeaderInfo {
    pub version: u32,
    pub package_flags: u32,
    pub slot_count: u32,
    pub launcher_abi: u32,
    pub slot_table_offset: u64,
    pub runtime_ref: [u8; TPKG_RUNTIME_REF_LEN],
}

/// Check a `TPKG_HEADER_SIZE` window as a trailer header (magic + crc) and
/// decode its header fields. `hdr` must be exactly the window bytes;
/// `size` is the total image size (used for the bounds check). The version
/// field is returned, not validated here (the caller decides which
/// versions it accepts).
pub(crate) fn parse_header(hdr: &[u8], size: u64) -> Result<HeaderInfo, TpkgError> {
    debug_assert_eq!(hdr.len(), TPKG_HEADER_SIZE);
    if size < TPKG_HEADER_SIZE as u64 {
        return Err(TpkgError::NoTrailer);
    }

    // absent vs corrupt: no "TEBA" prefix -> classic bundle, no trailer
    if hdr[off::MAGIC..off::MAGIC + TPKG_MAGIC_PREFIX_LEN] != TPKG_MAGIC[..TPKG_MAGIC_PREFIX_LEN] {
        return Err(TpkgError::NoTrailer);
    }
    if hdr[off::MAGIC..off::MAGIC + TPKG_MAGIC_LEN] != TPKG_MAGIC[..] {
        return Err(TpkgError::Magic);
    }
    if crc32(&hdr[..off::CRC32]) != get32(&hdr[off::CRC32..]) {
        return Err(TpkgError::Crc);
    }

    let version = get32(&hdr[off::VERSION..]);
    let slot_count = get32(&hdr[off::SLOT_COUNT..]);
    if slot_count == 0 || slot_count > TPKG_MAX_SLOTS {
        return Err(TpkgError::Slots);
    }

    let slot_table_offset = get64(&hdr[off::TABLE..]);
    let avail = size - TPKG_HEADER_SIZE as u64; // bytes preceding the header
                                                // overflow-free: the table must fit entirely before the header
    if slot_table_offset > avail
        || u64::from(slot_count) > (avail - slot_table_offset) / TPKG_SLOT_SIZE as u64
    {
        return Err(TpkgError::Bounds);
    }

    let mut runtime_ref = [0u8; TPKG_RUNTIME_REF_LEN];
    runtime_ref.copy_from_slice(&hdr[off::RUNTIME_REF..off::RUNTIME_REF + TPKG_RUNTIME_REF_LEN]);

    Ok(HeaderInfo {
        version,
        package_flags: get32(&hdr[off::PACKAGE_FLAGS..]),
        slot_count,
        launcher_abi: get32(&hdr[off::LAUNCHER_ABI..]),
        slot_table_offset,
        runtime_ref,
    })
}

/// Decode one slot record (280 bytes) from the table.
pub(crate) fn parse_slot_record(rec_bytes: &[u8]) -> Slot {
    debug_assert_eq!(rec_bytes.len(), TPKG_SLOT_SIZE);
    let mut mount_point = [0u8; TPKG_MOUNT_POINT_LEN];
    mount_point.copy_from_slice(&rec_bytes[rec::MOUNT..rec::MOUNT + TPKG_MOUNT_POINT_LEN]);
    Slot {
        offset: get64(&rec_bytes[rec::OFFSET..]),
        size: get64(&rec_bytes[rec::SIZE..]),
        format_id: get32(&rec_bytes[rec::FORMAT..]),
        flags: get32(&rec_bytes[rec::FLAGS..]),
        mount_point,
    }
}

/// Decode the v2 extension that immediately follows the trailer header
/// (`TPKG_V2_EXT_FIXED + sig_len` bytes): digests, keyid, signature, and
/// the trailing big-endian sig_len field.
pub(crate) fn parse_v2_extension(x: &[u8], slot_count: u32) -> Result<V2Extension, TpkgError> {
    debug_assert!(x.len() >= TPKG_V2_EXT_FIXED);
    let sig_len = get32be(&x[x.len() - TPKG_SIGLEN_SIZE..]) as usize;
    if sig_len == 0 || sig_len > TPKG_SIG_MAX as usize {
        return Err(TpkgError::Invalid);
    }
    if x.len() != TPKG_V2_EXT_FIXED + sig_len {
        return Err(TpkgError::Invalid);
    }

    let mut slot_digests = [[0u8; TPKG_SHA256_LEN]; TPKG_MAX_SLOTS as usize];
    for (i, d) in slot_digests.iter_mut().enumerate() {
        d.copy_from_slice(&x[i * TPKG_SHA256_LEN..(i + 1) * TPKG_SHA256_LEN]);
    }
    if slot_digests[slot_count as usize..]
        .iter()
        .any(|d| *d != [0; TPKG_SHA256_LEN])
    {
        return Err(TpkgError::Invalid);
    }

    let mut signer_keyid = [0u8; TPKG_KEYID_LEN];
    signer_keyid.copy_from_slice(&x[TPKG_DIGESTS_SIZE..TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN]);

    Ok(V2Extension {
        slot_digests,
        signer_keyid,
        signature: x
            [TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN..TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN + sig_len]
            .to_vec(),
    })
}

/// Finish parsing after the slot records have been read: assemble and
/// validate the manifest (a valid crc does not imply well-formed fields
/// when the trailer was not written by us).
pub(crate) fn finish(
    header: &HeaderInfo,
    slots: Vec<Slot>,
    v2: Option<V2Extension>,
) -> Result<Manifest, TpkgError> {
    let manifest = Manifest {
        version: header.version,
        package_flags: header.package_flags,
        launcher_abi: header.launcher_abi,
        runtime_ref: header.runtime_ref,
        slots,
        v2,
    };
    manifest.validate()?;
    Ok(manifest)
}

/// The canonical (signed) region of a v2 trailer: slot table || digest
/// array || keyid || trailer header — the two contiguous spans
/// concatenated (everything except the signature and its length field).
/// `trailer` must be the exact trailer bytes (slot table + extension +
/// header).
pub fn v2_signed_region(trailer: &[u8]) -> Result<Vec<u8>, TpkgError> {
    if trailer.len() < TPKG_V2_EXT_FIXED + TPKG_HEADER_SIZE {
        return Err(TpkgError::Invalid);
    }
    let sig_len = get32be(
        &trailer
            [trailer.len() - TPKG_HEADER_SIZE - TPKG_SIGLEN_SIZE..trailer.len() - TPKG_HEADER_SIZE],
    ) as usize;
    if sig_len == 0 || sig_len > TPKG_SIG_MAX as usize {
        return Err(TpkgError::Invalid);
    }
    let ext_len = TPKG_V2_EXT_FIXED + sig_len;
    if trailer.len() < ext_len + TPKG_HEADER_SIZE {
        return Err(TpkgError::Invalid);
    }
    let table_len = trailer.len() - TPKG_HEADER_SIZE - ext_len;
    let mut region =
        Vec::with_capacity(table_len + TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN + TPKG_HEADER_SIZE);
    // span 1: slot table + digests + keyid
    region.extend_from_slice(&trailer[..table_len + TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN]);
    // span 2: the trailer header
    region.extend_from_slice(&trailer[table_len + ext_len..]);
    Ok(region)
}

/// Total on-disk trailer length for a parsed manifest (slot table +
/// [v2 extension +] header).
pub fn trailer_len(m: &Manifest) -> u64 {
    let base = m.slots.len() as u64 * TPKG_SLOT_SIZE as u64 + TPKG_HEADER_SIZE as u64;
    base + m
        .v2
        .as_ref()
        .map_or(0, |v2| (TPKG_V2_EXT_FIXED + v2.signature.len()) as u64)
}

/// Read the manifest trailer from an in-memory image of the binary
/// (mirrors the C `tpkg_read_mem()`, extended with the v2 flag check).
pub fn parse_trailer(data: &[u8]) -> Result<Manifest, TpkgError> {
    let size = data.len() as u64;
    if size < TPKG_HEADER_SIZE as u64 {
        return Err(TpkgError::NoTrailer);
    }

    let hdr = &data[data.len() - TPKG_HEADER_SIZE..];
    let header = parse_header(hdr, size)?;
    if header.version != TPKG_VERSION {
        return Err(TpkgError::Version);
    }

    let table_start = header.slot_table_offset as usize;
    let table_len = header.slot_count as usize * TPKG_SLOT_SIZE;
    let table = &data[table_start..table_start + table_len];

    let slots: Vec<Slot> = (0..header.slot_count as usize)
        .map(|i| parse_slot_record(&table[i * TPKG_SLOT_SIZE..(i + 1) * TPKG_SLOT_SIZE]))
        .collect();

    let v2 = if header.package_flags & TPKG_FLAG_SIGNED_V2 != 0 {
        let ext_start = table_start + table_len;
        let ext_end = data.len() - TPKG_HEADER_SIZE;
        Some(parse_v2_extension(
            &data[ext_start..ext_end],
            header.slot_count,
        )?)
    } else {
        None
    };

    finish(&header, slots, v2)
}
