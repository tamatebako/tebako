//! I/O readers/writers, mirroring the C `tpkg_read_fd()`/`tpkg_write_fd()`.

use std::io::{Read, Seek, SeekFrom, Write};

use crate::codec::{
    encode_trailer, finish, parse_header, parse_slot_record, parse_v2_extension, trailing_sig_len,
    v2_header_offset,
};
use crate::error::TpkgError;
use crate::model::Manifest;
use crate::{
    TPKG_HEADER_SIZE, TPKG_MAGIC, TPKG_MAGIC_PREFIX_LEN, TPKG_SIGLEN_SIZE, TPKG_SLOT_SIZE,
    TPKG_V2_EXT_FIXED, TPKG_VERSION, TPKG_VERSION_2,
};

/// Read the manifest trailer from a seekable binary (e.g. `std::fs::File`).
///
/// Any i/o failure maps to [`TpkgError::Io`].
pub fn read_from<R: Read + Seek>(r: &mut R) -> Result<Manifest, TpkgError> {
    let size = r.seek(SeekFrom::End(0)).map_err(|_| TpkgError::Io)?;
    if size < TPKG_HEADER_SIZE as u64 {
        return Err(TpkgError::NoTrailer);
    }

    // Stage A: v2 probe (trailing big-endian sig_len)
    if size >= TPKG_SIGLEN_SIZE as u64 {
        let mut last4 = [0u8; TPKG_SIGLEN_SIZE];
        r.seek(SeekFrom::End(-(TPKG_SIGLEN_SIZE as i64)))
            .and_then(|_| r.read_exact(&mut last4))
            .map_err(|_| TpkgError::Io)?;
        if let Some(hoff) = v2_header_offset(size, trailing_sig_len(&last4)) {
            let mut prefix = [0u8; TPKG_MAGIC_PREFIX_LEN];
            r.seek(SeekFrom::Start(hoff))
                .and_then(|_| r.read_exact(&mut prefix))
                .map_err(|_| TpkgError::Io)?;
            if prefix[..] == TPKG_MAGIC[..TPKG_MAGIC_PREFIX_LEN] {
                let mut hdr = [0u8; TPKG_HEADER_SIZE];
                r.seek(SeekFrom::Start(hoff))
                    .and_then(|_| r.read_exact(&mut hdr))
                    .map_err(|_| TpkgError::Io)?;
                if let Ok(header) = parse_header(&hdr, hoff + TPKG_HEADER_SIZE as u64) {
                    if header.version == TPKG_VERSION_2 {
                        let mut x = vec![0u8; TPKG_V2_EXT_FIXED + trailing_sig_len(&last4) as usize];
                        r.seek(SeekFrom::Start(hoff + TPKG_HEADER_SIZE as u64))
                            .and_then(|_| r.read_exact(&mut x))
                            .map_err(|_| TpkgError::Io)?;
                        let v2 = parse_v2_extension(&x, header.slot_count)?;
                        let mut table = vec![0u8; header.slot_count as usize * TPKG_SLOT_SIZE];
                        r.seek(SeekFrom::Start(header.slot_table_offset))
                            .and_then(|_| r.read_exact(&mut table))
                            .map_err(|_| TpkgError::Io)?;
                        let slots = (0..header.slot_count as usize)
                            .map(|i| parse_slot_record(&table[i * TPKG_SLOT_SIZE..(i + 1) * TPKG_SLOT_SIZE]))
                            .collect();
                        return finish(&header, slots, Some(v2));
                    }
                }
            }
        }
    }

    // Stage B: v1 (header at EOF)
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

    let slots = (0..header.slot_count as usize)
        .map(|i| parse_slot_record(&table[i * TPKG_SLOT_SIZE..(i + 1) * TPKG_SLOT_SIZE]))
        .collect();
    finish(&header, slots, None)
}

/// Append the slot table + trailer header to a seekable binary (at its
/// current EOF). The manifest is validated first; a rejected manifest
/// appends nothing. Any i/o failure maps to [`TpkgError::Io`].
pub fn write_to<W: Write + Seek>(w: &mut W, manifest: &Manifest) -> Result<(), TpkgError> {
    let end = w.seek(SeekFrom::End(0)).map_err(|_| TpkgError::Io)?;
    let trailer = encode_trailer(manifest, end)?;
    w.write_all(&trailer).map_err(|_| TpkgError::Io)
}
