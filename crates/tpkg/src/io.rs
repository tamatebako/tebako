//! I/O readers/writers, mirroring the C `tpkg_read_fd()`/`tpkg_write_fd()`.

use std::io::{Read, Seek, SeekFrom, Write};

use crate::codec::{encode_trailer, finish, parse_header, parse_slot_record};
use crate::error::TpkgError;
use crate::model::Manifest;
use crate::{TPKG_HEADER_SIZE, TPKG_SLOT_SIZE};

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

    let mut table = vec![0u8; header.slot_count as usize * TPKG_SLOT_SIZE];
    r.seek(SeekFrom::Start(header.slot_table_offset))
        .and_then(|_| r.read_exact(&mut table))
        .map_err(|_| TpkgError::Io)?;

    let slots = (0..header.slot_count as usize)
        .map(|i| parse_slot_record(&table[i * TPKG_SLOT_SIZE..(i + 1) * TPKG_SLOT_SIZE]))
        .collect();
    finish(&header, slots)
}

/// Append the slot table + trailer header to a seekable binary (at its
/// current EOF). The manifest is validated first; a rejected manifest
/// appends nothing. Any i/o failure maps to [`TpkgError::Io`].
pub fn write_to<W: Write + Seek>(w: &mut W, manifest: &Manifest) -> Result<(), TpkgError> {
    let end = w.seek(SeekFrom::End(0)).map_err(|_| TpkgError::Io)?;
    let trailer = encode_trailer(manifest, end)?;
    w.write_all(&trailer).map_err(|_| TpkgError::Io)
}
