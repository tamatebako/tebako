//! Mach-O awareness for the trailer locator (spec 02 §1, spec 31 §1.2).
//!
//! A package that was codesigned post-press carries the Mach-O
//! code-signature superblob AFTER the trailer: codesign appends at the
//! physical EOF (16-byte aligned, zero-padded) and points the tail
//! slice's LC_CODE_SIGNATURE at it. The trailer therefore ends where the
//! superblob's alignment padding begins. `trailer_end` recognizes
//! exactly that shape — a tail-reaching superblob — and locates the
//! trailer by its self-validating header (magic + crc + bounds) in the
//! window ending at the superblob start or at most 15 zero pad bytes
//! earlier. Every other shape (non-Mach-O, unsigned, superblob not at
//! the tail, unparseable header, no valid trailer under the superblob)
//! yields the physical EOF unchanged. It never guesses and never fails
//! on content: only i/o errors propagate.

use std::io::{Read, Seek, SeekFrom};

use crate::codec::parse_header;
use crate::error::TpkgError;
use crate::TPKG_HEADER_SIZE;

const LC_CODE_SIGNATURE: u32 = 0x1d;
const HEADER_64_LEN: usize = 32;
/// Upper bound for the header + command-table read (linker output is a
/// few KiB; anything larger is not a shape the locator recognizes).
const MAX_COMMAND_AREA: u64 = 1 << 20;
/// codesign aligns the superblob to 16 bytes; the zero pad between the
/// trailer and the superblob is therefore shorter than this.
const SUPERBLOB_ALIGN: u64 = 16;

/// The effective trailer end for a package of `eof` bytes.
pub fn trailer_end<R: Read + Seek>(r: &mut R, eof: u64) -> Result<u64, TpkgError> {
    if eof < 4 {
        return Ok(eof);
    }
    let mut magic = [0u8; 4];
    r.seek(SeekFrom::Start(0))
        .and_then(|_| r.read_exact(&mut magic))
        .map_err(|_| TpkgError::Io)?;
    let superblob = match magic {
        [0xcf, 0xfa, 0xed, 0xfe] => slice_signature_end(r, 0, eof, false),
        [0xfe, 0xed, 0xfa, 0xcf] => slice_signature_end(r, 0, eof, true),
        [0xca, 0xfe, 0xba, 0xbe] | [0xca, 0xfe, 0xba, 0xbf] => {
            fat_signature_end(r, eof, magic[3] == 0xbf)
        }
        _ => None,
    };
    match superblob {
        Some(start) => Ok(validated_end_before(r, start).unwrap_or(eof)),
        None => Ok(eof),
    }
}

/// The trailer end immediately before a superblob at `start`: `start`
/// itself, or up to `SUPERBLOB_ALIGN - 1` zero pad bytes earlier. Only a
/// candidate whose `TPKG_HEADER_SIZE` window validates as a trailer
/// header (magic + crc + slot-table bounds) is accepted, so a superblob
/// that does not bury a trailer yields `None` rather than a guess.
fn validated_end_before<R: Read + Seek>(r: &mut R, start: u64) -> Option<u64> {
    let mut end = start;
    loop {
        if end >= TPKG_HEADER_SIZE as u64 {
            let mut hdr = [0u8; TPKG_HEADER_SIZE];
            if r.seek(SeekFrom::Start(end - TPKG_HEADER_SIZE as u64))
                .and_then(|_| r.read_exact(&mut hdr))
                .is_ok()
                && parse_header(&hdr, end).is_ok()
            {
                return Some(end);
            }
        }
        if start - end >= SUPERBLOB_ALIGN - 1 || end == 0 {
            return None;
        }
        let mut pad = [0u8; 1];
        if r.seek(SeekFrom::Start(end - 1))
            .and_then(|_| r.read_exact(&mut pad))
            .is_err()
            || pad[0] != 0
        {
            return None;
        }
        end -= 1;
    }
}

/// One 64-bit slice: the absolute superblob start when its
/// LC_CODE_SIGNATURE reaches `eof` exactly. `None` on any other shape.
fn slice_signature_end<R: Read + Seek>(
    r: &mut R,
    start: u64,
    eof: u64,
    big_endian: bool,
) -> Option<u64> {
    if eof < start + HEADER_64_LEN as u64 {
        return None;
    }
    let mut hdr = [0u8; HEADER_64_LEN];
    r.seek(SeekFrom::Start(start))
        .and_then(|_| r.read_exact(&mut hdr))
        .ok()?;
    let u32s = |b: &[u8], at: usize| {
        let b = b.get(at..at + 4)?;
        Some(if big_endian {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    let ncmds = u32s(&hdr, 16)? as u64;
    let sizeofcmds = u32s(&hdr, 20)? as u64;
    if sizeofcmds > MAX_COMMAND_AREA {
        return None;
    }
    let mut cmds = vec![0u8; sizeofcmds as usize];
    r.seek(SeekFrom::Start(start + HEADER_64_LEN as u64))
        .and_then(|_| r.read_exact(&mut cmds))
        .ok()?;
    let mut at = 0usize;
    for _ in 0..ncmds {
        let cmd = u32s(&cmds, at)?;
        let cmdsize = u32s(&cmds, at + 4)? as usize;
        if cmdsize < 8 || at + cmdsize > cmds.len() {
            return None;
        }
        if cmd == LC_CODE_SIGNATURE && cmdsize >= 16 {
            let dataoff = u32s(&cmds, at + 8)? as u64;
            let datasize = u32s(&cmds, at + 12)? as u64;
            if dataoff != 0 && start + dataoff + datasize == eof {
                return Some(start + dataoff);
            }
        }
        at += cmdsize;
    }
    None
}

/// A universal binary: each slice's superblob is slice-relative; the
/// trailer ends where the EOF-reaching slice's superblob begins.
fn fat_signature_end<R: Read + Seek>(r: &mut R, eof: u64, is_64: bool) -> Option<u64> {
    let entry_len: u64 = if is_64 { 32 } else { 20 };
    let mut hdr = [0u8; 8];
    r.seek(SeekFrom::Start(0))
        .and_then(|_| r.read_exact(&mut hdr))
        .ok()?;
    let nfat = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as u64;
    if nfat == 0 || nfat > 64 {
        return None;
    }
    let mut table = vec![0u8; (nfat * entry_len) as usize];
    r.seek(SeekFrom::Start(8))
        .and_then(|_| r.read_exact(&mut table))
        .ok()?;
    for i in 0..nfat as usize {
        let e = &table[i * entry_len as usize..(i + 1) * entry_len as usize];
        let (offset, size) = if is_64 {
            (
                u64::from_be_bytes(e[8..16].try_into().ok()?),
                u64::from_be_bytes(e[16..24].try_into().ok()?),
            )
        } else {
            (
                u32::from_be_bytes(e[8..12].try_into().ok()?) as u64,
                u32::from_be_bytes(e[12..16].try_into().ok()?) as u64,
            )
        };
        // Only the slice reaching the file tail can hold the trailer's
        // superblob.
        if offset.checked_add(size)? > eof {
            continue;
        }
        let magic_at = offset;
        let mut magic = [0u8; 4];
        if r.seek(SeekFrom::Start(magic_at))
            .and_then(|_| r.read_exact(&mut magic))
            .is_err()
        {
            continue;
        }
        let be = match magic {
            [0xcf, 0xfa, 0xed, 0xfe] => false,
            [0xfe, 0xed, 0xfa, 0xcf] => true,
            _ => continue,
        };
        if let Some(end) = slice_signature_end(r, offset, eof, be) {
            return Some(end);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Manifest, Slot};
    use crate::TPKG_FORMAT_ZIP;
    use std::io::Cursor;

    /// A thin LE 64-bit Mach-O header carrying one LC_CODE_SIGNATURE at
    /// (dataoff, datasize); nothing follows the command table yet.
    fn signed_macho(dataoff: u32, datasize: u32) -> Vec<u8> {
        let le = |v: u32| v.to_le_bytes();
        let mut v = Vec::new();
        v.extend_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        v.extend_from_slice(&le(0x0100_000c));
        v.extend_from_slice(&le(0));
        v.extend_from_slice(&le(2));
        v.extend_from_slice(&le(1)); // ncmds
        v.extend_from_slice(&le(16)); // sizeofcmds
        v.extend_from_slice(&le(0));
        v.extend_from_slice(&le(0));
        v.extend_from_slice(&le(LC_CODE_SIGNATURE));
        v.extend_from_slice(&le(16));
        v.extend_from_slice(&le(dataoff));
        v.extend_from_slice(&le(datasize));
        v
    }

    /// A real encoded trailer (slot table + header), valid magic+crc.
    fn trailer() -> Vec<u8> {
        let mut m = Manifest::default();
        m.slots.push(Slot::new(0, 100, TPKG_FORMAT_ZIP, "/m"));
        crate::codec::encode_trailer(&m, 0).unwrap()
    }

    /// The codesigned-package shape: [mach-o][trailer][zero pad][superblob].
    /// `dataoff` is the (16-aligned) superblob start; the trailer ends at
    /// `dataoff - pad`.
    fn signed_package(dataoff: usize, datasize: usize, pad: usize) -> Vec<u8> {
        let trailer = trailer();
        assert!(dataoff >= signed_macho(0, 0).len() + trailer.len() + pad);
        let mut v = signed_macho(dataoff as u32, datasize as u32);
        v.resize(dataoff - pad - trailer.len(), 0);
        v.extend_from_slice(&trailer);
        v.resize(dataoff, 0);
        let n = v.len();
        v.resize(n + datasize, 0xbb);
        v
    }

    fn end_of(bytes: &[u8]) -> u64 {
        trailer_end(&mut Cursor::new(bytes), bytes.len() as u64).unwrap()
    }

    #[test]
    fn superblob_at_eof_reveals_the_trailer_end() {
        // [mach-o][trailer][superblob] — no alignment pad.
        let v = signed_package(0x400, 0x80, 0);
        assert_eq!(end_of(&v), 0x400);
    }

    #[test]
    fn alignment_pad_before_the_superblob_is_skipped() {
        // codesign pads the superblob to 16 bytes with zeros; the trailer
        // ends at the unaligned pre-sign EOF.
        let trailer_end = 0x400usize - 9; // pad of 9 up to the aligned 0x400
        let v = signed_package(0x400, 0x80, 9);
        assert_eq!(end_of(&v), trailer_end as u64);
    }

    #[test]
    fn unsigned_and_non_macho_keep_the_physical_eof() {
        let mut unsigned = signed_macho(0, 0);
        unsigned.extend_from_slice(&trailer());
        assert_eq!(end_of(&unsigned), unsigned.len() as u64);
        let pe = b"MZ\x90\x00 payload".as_slice();
        assert_eq!(end_of(pe), pe.len() as u64);
        let short = b"\xcf\xfa".as_slice();
        assert_eq!(end_of(short), short.len() as u64);
    }

    #[test]
    fn superblob_without_a_valid_trailer_keeps_the_physical_eof() {
        // A superblob at EOF but only zeros beneath it: not a package.
        let mut v = signed_macho(0x400, 0x80);
        v.resize(0x400, 0);
        let n = v.len();
        v.resize(n + 0x80, 0xbb);
        assert_eq!(end_of(&v), v.len() as u64);
        // A corrupt trailer (bad crc) under the superblob is not located.
        let mut v = signed_package(0x400, 0x80, 0);
        v[0x400 - TPKG_HEADER_SIZE + 20] ^= 0xff;
        assert_eq!(end_of(&v), v.len() as u64);
        // A nonzero byte between the trailer and the superblob is not the
        // codesign pad: the scan stops, the EOF stands.
        let mut v = signed_package(0x400, 0x80, 9);
        v[0x400 - 5] = 0x77;
        assert_eq!(end_of(&v), v.len() as u64);
    }

    #[test]
    fn superblob_not_at_eof_keeps_the_physical_eof() {
        // bytes after the superblob: not the codesigned-package shape.
        let mut v = signed_package(0x400, 0x80, 0);
        v.extend_from_slice(b"more");
        assert_eq!(end_of(&v), v.len() as u64);
    }

    #[test]
    fn fat_tail_slice_reveals_the_trailer_end() {
        // Two slices; the tail slice's superblob reaches EOF, its trailer
        // ends 7 pad bytes earlier (slice-relative offsets).
        let pad = 7usize;
        let s1_dataoff = 0x400usize;
        let trailer = trailer();
        let mut s0 = signed_macho(0, 0);
        s0.resize(0x180, 0);
        let mut s1 = signed_macho(s1_dataoff as u32, 0x60);
        s1.resize(s1_dataoff - pad - trailer.len(), 0);
        s1.extend_from_slice(&trailer);
        s1.resize(s1_dataoff, 0);
        let n = s1.len();
        s1.resize(n + 0x60, 0xbb);

        let off0 = 0x1000usize;
        let off1 = 0x2000usize;
        let mut v = Vec::new();
        v.extend_from_slice(&0xcafebabeu32.to_be_bytes());
        v.extend_from_slice(&2u32.to_be_bytes());
        for (off, s) in [(off0, &s0), (off1, &s1)] {
            v.extend_from_slice(&0x0100_000cu32.to_be_bytes());
            v.extend_from_slice(&0u32.to_be_bytes());
            v.extend_from_slice(&(off as u32).to_be_bytes());
            v.extend_from_slice(&(s.len() as u32).to_be_bytes());
            v.extend_from_slice(&12u32.to_be_bytes());
        }
        v.resize(off0, 0);
        v.extend_from_slice(&s0);
        v.resize(off1, 0);
        v.extend_from_slice(&s1);
        assert_eq!(end_of(&v), (off1 + s1_dataoff - pad) as u64);
    }
}
