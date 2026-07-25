//! The wire codec: encode/parse the trailer, byte-exact with the C
//! implementation's `tpkg_write_fd()`/`tpkg_read_mem()`.

use crate::error::TpkgError;
use crate::model::{put_str, Manifest, Slot};
use crate::{crc32, off, rec};
use crate::{
    TPKG_HEADER_SIZE, TPKG_MAGIC, TPKG_MAGIC_LEN, TPKG_MAGIC_PREFIX_LEN, TPKG_MAX_SLOTS,
    TPKG_MOUNT_POINT_LEN, TPKG_RUNTIME_REF_LEN, TPKG_SLOT_SIZE, TPKG_VERSION,
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

/// Serialize the slot table + trailer header for `manifest`, placing the
/// slot table at absolute file offset `slot_table_offset` (i.e. the current
/// EOF when appending). Returns `slot_count * TPKG_SLOT_SIZE +
/// TPKG_HEADER_SIZE` bytes.
///
/// The manifest is validated first; nothing is produced for a rejected
/// manifest (mirrors the C writer appending nothing).
pub fn encode_trailer(manifest: &Manifest, slot_table_offset: u64) -> Result<Vec<u8>, TpkgError> {
    manifest.validate()?;

    let mut out = vec![0u8; manifest.slots.len() * TPKG_SLOT_SIZE + TPKG_HEADER_SIZE];

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

    // trailer header
    let hbase = manifest.slots.len() * TPKG_SLOT_SIZE;
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
    pub package_flags: u32,
    pub slot_count: u32,
    pub launcher_abi: u32,
    pub slot_table_offset: u64,
    pub runtime_ref: [u8; TPKG_RUNTIME_REF_LEN],
}

/// Check the last-`TPKG_HEADER_SIZE` window of an image of `size` bytes and
/// decode its header fields. `hdr` must be exactly the window bytes.
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
    if version != TPKG_VERSION {
        return Err(TpkgError::Version);
    }
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

/// Finish parsing after the slot records have been read: assemble and
/// validate the manifest (a valid crc does not imply well-formed fields
/// when the trailer was not written by us).
pub(crate) fn finish(header: &HeaderInfo, slots: Vec<Slot>) -> Result<Manifest, TpkgError> {
    let manifest = Manifest {
        version: TPKG_VERSION,
        package_flags: header.package_flags,
        launcher_abi: header.launcher_abi,
        runtime_ref: header.runtime_ref,
        slots,
    };
    manifest.validate()?;
    Ok(manifest)
}

/// Read the manifest trailer from an in-memory image of the binary
/// (mirrors the C `tpkg_read_mem()`).
pub fn parse_trailer(data: &[u8]) -> Result<Manifest, TpkgError> {
    let size = data.len() as u64;
    if size < TPKG_HEADER_SIZE as u64 {
        return Err(TpkgError::NoTrailer);
    }
    let hdr = &data[data.len() - TPKG_HEADER_SIZE..];
    let header = parse_header(hdr, size)?;

    let table_start = header.slot_table_offset as usize;
    let table_len = header.slot_count as usize * TPKG_SLOT_SIZE;
    let table = &data[table_start..table_start + table_len];

    let slots = (0..header.slot_count as usize)
        .map(|i| parse_slot_record(&table[i * TPKG_SLOT_SIZE..(i + 1) * TPKG_SLOT_SIZE]))
        .collect();
    finish(&header, slots)
}
