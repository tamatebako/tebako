//! I/O readers/writers, mirroring the C `tpkg_read_fd()`/`tpkg_write_fd()`.

use std::io::{Read, Seek, SeekFrom, Write};

use crate::codec::{
    encode_trailer, finish, parse_ext_blocks, parse_header, parse_slot_record, parse_v2_extension,
};
use crate::error::TpkgError;
use crate::ext::ExtBlock;
use crate::model::{Manifest, Slot};
use crate::{
    TPKG_EXT_HEADER_SIZE, TPKG_FLAG_SIGNED_V2, TPKG_HEADER_SIZE, TPKG_SIG_MAX, TPKG_SLOT_SIZE,
    TPKG_V2_EXT_FIXED, TPKG_VERSION,
};

/// Read the manifest trailer from a seekable binary (e.g. `std::fs::File`).
///
/// Any i/o failure maps to [`TpkgError::Io`].
pub fn read_from<R: Read + Seek>(r: &mut R) -> Result<Manifest, TpkgError> {
    let size = r.seek(SeekFrom::End(0)).map_err(|_| TpkgError::Io)?;
    if size < TPKG_HEADER_SIZE as u64 {
        return Err(TpkgError::NoTrailer);
    }

    let mut hdr = [0u8; TPKG_HEADER_SIZE];
    r.seek(SeekFrom::Start(size - TPKG_HEADER_SIZE as u64))
        .and_then(|_| r.read_exact(&mut hdr))
        .map_err(|_| TpkgError::Io)?;
    let header = parse_header(&hdr, size)?;
    if header.version != TPKG_VERSION {
        return Err(TpkgError::Version);
    }

    let mut table = vec![0u8; header.slot_count as usize * TPKG_SLOT_SIZE];
    r.seek(SeekFrom::Start(header.slot_table_offset))
        .and_then(|_| r.read_exact(&mut table))
        .map_err(|_| TpkgError::Io)?;

    let slots: Vec<Slot> = (0..header.slot_count as usize)
        .map(|i| parse_slot_record(&table[i * TPKG_SLOT_SIZE..(i + 1) * TPKG_SLOT_SIZE]))
        .collect();

    // The gap between the slot table and the trailer header: extension
    // blocks, then (when SIGNED_V2) the v2 extension at the tail.
    let gap_start = header.slot_table_offset + table.len() as u64;
    let gap_len = size - TPKG_HEADER_SIZE as u64 - gap_start;
    let signed = header.package_flags & TPKG_FLAG_SIGNED_V2 != 0;

    let v2 = if signed {
        // sig_len is the u32be immediately before the trailer header.
        if gap_len < TPKG_V2_EXT_FIXED as u64 {
            return Err(TpkgError::Invalid);
        }
        let mut tail = [0u8; 4];
        r.seek(SeekFrom::Start(size - TPKG_HEADER_SIZE as u64 - 4))
            .and_then(|_| r.read_exact(&mut tail))
            .map_err(|_| TpkgError::Io)?;
        let sig_len = u32::from_be_bytes(tail) as usize;
        if sig_len == 0 || sig_len > TPKG_SIG_MAX as usize {
            return Err(TpkgError::Invalid);
        }
        let ext_len = TPKG_V2_EXT_FIXED + sig_len;
        if ext_len as u64 > gap_len {
            return Err(TpkgError::Invalid);
        }
        let mut x = vec![0u8; ext_len];
        r.seek(SeekFrom::Start(
            size - TPKG_HEADER_SIZE as u64 - ext_len as u64,
        ))
        .and_then(|_| r.read_exact(&mut x))
        .map_err(|_| TpkgError::Io)?;
        Some(parse_v2_extension(&x, header.slot_count)?)
    } else {
        None
    };

    let v2_ext_len = v2
        .as_ref()
        .map_or(0, |v2| (TPKG_V2_EXT_FIXED + v2.signature.len()) as u64);
    let block_len = gap_len - v2_ext_len;
    let ext_blocks: Vec<ExtBlock> = if block_len == 0 {
        Vec::new()
    } else if block_len < TPKG_EXT_HEADER_SIZE as u64 {
        // A truncated block header — garbage is never silently accepted.
        return Err(TpkgError::Invalid);
    } else {
        let mut region = vec![0u8; block_len as usize];
        r.seek(SeekFrom::Start(gap_start))
            .and_then(|_| r.read_exact(&mut region))
            .map_err(|_| TpkgError::Io)?;
        parse_ext_blocks(&region)?
    };

    finish(&header, slots, ext_blocks, v2)
}

/// Append the trailer (slot table, extension blocks, v2 extension when
/// present, trailer header) to a seekable binary at its current EOF. The
/// manifest is validated first; a rejected manifest appends nothing. Any
/// i/o failure maps to [`TpkgError::Io`].
pub fn write_to<W: Write + Seek>(w: &mut W, manifest: &Manifest) -> Result<(), TpkgError> {
    let end = w.seek(SeekFrom::End(0)).map_err(|_| TpkgError::Io)?;
    let trailer = encode_trailer(manifest, end)?;
    w.write_all(&trailer).map_err(|_| TpkgError::Io)
}
