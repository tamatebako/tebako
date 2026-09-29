//! PE/Authenticode awareness for the trailer locator (spec 02 §1, spec
//! 34 §1.2/§6 — the PE twin of the Mach-O arm in `macho.rs`).
//!
//! Authenticode signing appends the WIN_CERTIFICATE table at the physical
//! EOF of a PE file and points the optional header's security data
//! directory (`IMAGE_DIRECTORY_ENTRY_SECURITY`, index 4) at it — a file
//! offset, not an RVA, per the PE specification. On a package signed
//! post-press the trailer therefore ends where the certificate table (or
//! its 8-byte alignment pad) begins. `certificate_table_start` recognizes
//! exactly that shape — a certificate table reaching the physical EOF
//! exactly — and returns its start; every other shape (non-PE, unsigned,
//! table not at the tail, unparseable headers) yields `None`. It never
//! guesses and never fails on content: like the Mach-O arm, only the
//! self-validating trailer header under the returned offset (checked by
//! the caller) distinguishes a buried trailer from coincidence.

use std::io::{Read, Seek, SeekFrom};

/// DOS header: offset of `e_lfanew`, the PE header pointer.
const E_LFANEW: u64 = 0x3c;
const PE_SIG: [u8; 4] = *b"PE\0\0";
const COFF_HEADER_LEN: u64 = 20;
const PE32_MAGIC: u16 = 0x10b;
const PE32_PLUS_MAGIC: u16 = 0x20b;
/// `IMAGE_DIRECTORY_ENTRY_SECURITY` in the data directory table.
const DIR_INDEX_SECURITY: u64 = 4;
const DIR_ENTRY_LEN: u64 = 8;

/// The absolute start of the WIN_CERTIFICATE table when it reaches `eof`
/// exactly; `None` on any other shape. The reader is positioned nowhere
/// in particular on return.
pub(crate) fn certificate_table_start<R: Read + Seek>(r: &mut R, eof: u64) -> Option<u64> {
    // DOS header → e_lfanew (u32le file offset of the PE signature).
    if eof < E_LFANEW + 4 {
        return None;
    }
    let mut b4 = [0u8; 4];
    r.seek(SeekFrom::Start(E_LFANEW))
        .and_then(|_| r.read_exact(&mut b4))
        .ok()?;
    let pe_off = u32::from_le_bytes(b4) as u64;
    // PE signature + COFF header must fit.
    if pe_off.checked_add(4 + COFF_HEADER_LEN)? > eof {
        return None;
    }
    let mut sig = [0u8; 4];
    r.seek(SeekFrom::Start(pe_off))
        .and_then(|_| r.read_exact(&mut sig))
        .ok()?;
    if sig != PE_SIG {
        return None;
    }
    let mut coff = [0u8; COFF_HEADER_LEN as usize];
    r.seek(SeekFrom::Start(pe_off + 4))
        .and_then(|_| r.read_exact(&mut coff))
        .ok()?;
    let opt_size = u16::from_le_bytes([coff[16], coff[17]]) as u64;
    let opt_off = pe_off + 4 + COFF_HEADER_LEN;
    if opt_off.checked_add(opt_size)? > eof {
        return None;
    }
    // Optional header magic selects the data directory layout.
    let mut b2 = [0u8; 2];
    r.seek(SeekFrom::Start(opt_off))
        .and_then(|_| r.read_exact(&mut b2))
        .ok()?;
    let (num_rva_at, dir_base) = match u16::from_le_bytes(b2) {
        PE32_MAGIC => (92u64, 96u64),
        PE32_PLUS_MAGIC => (108u64, 112u64),
        _ => return None,
    };
    let security_at = dir_base + DIR_INDEX_SECURITY * DIR_ENTRY_LEN;
    // The entry (and therefore the NumberOfRvaAndSizes read before it)
    // must lie inside the declared optional header.
    if opt_size < security_at + DIR_ENTRY_LEN {
        return None;
    }
    // A directory index past NumberOfRvaAndSizes reads as zero = unsigned.
    r.seek(SeekFrom::Start(opt_off + num_rva_at))
        .and_then(|_| r.read_exact(&mut b4))
        .ok()?;
    if u64::from(u32::from_le_bytes(b4)) <= DIR_INDEX_SECURITY {
        return None;
    }
    let mut dir = [0u8; DIR_ENTRY_LEN as usize];
    r.seek(SeekFrom::Start(opt_off + security_at))
        .and_then(|_| r.read_exact(&mut dir))
        .ok()?;
    let cert_off = u32::from_le_bytes(dir[0..4].try_into().ok()?) as u64;
    let cert_size = u32::from_le_bytes(dir[4..8].try_into().ok()?) as u64;
    // Absent or zero-length entry = unsigned.
    if cert_off == 0 || cert_size == 0 {
        return None;
    }
    // Only a table reaching the physical EOF exactly can bury a trailer.
    if cert_off.checked_add(cert_size)? == eof {
        Some(cert_off)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macho::trailer_end;
    use crate::model::{Manifest, Slot};
    use crate::TPKG_FORMAT_ZIP;
    use std::io::Cursor;

    const E_LFANEW_VALUE: usize = 0x80;
    const OPT_SIZE: usize = 0xf0; // PE32+ optional header, 16 directories

    /// A thin PE32+ carrying a security directory entry at
    /// (`cert_off`, `cert_size`); nothing after the headers yet.
    fn signed_pe(cert_off: u32, cert_size: u32) -> Vec<u8> {
        let le32 = |v: u32| v.to_le_bytes();
        let le16 = |v: u16| v.to_le_bytes();
        let mut v = Vec::new();
        v.extend_from_slice(b"MZ");
        v.resize(E_LFANEW as usize, 0);
        v.extend_from_slice(&le32(E_LFANEW_VALUE as u32));
        v.resize(E_LFANEW_VALUE, 0);
        v.extend_from_slice(b"PE\0\0");
        v.extend_from_slice(&le16(0x8664)); // machine: AMD64
        v.extend_from_slice(&le16(1)); // sections
        v.extend_from_slice(&[0; 12]);
        v.extend_from_slice(&le16(OPT_SIZE as u16));
        v.extend_from_slice(&le16(0x22)); // characteristics
        let opt = v.len();
        v.extend_from_slice(&le16(PE32_PLUS_MAGIC));
        v.resize(opt + 108, 0);
        v.extend_from_slice(&le32(16)); // NumberOfRvaAndSizes
        v.resize(opt + 112 + 4 * 8, 0); // directories 0..=3
        v.extend_from_slice(&le32(cert_off));
        v.extend_from_slice(&le32(cert_size));
        v.resize(opt + OPT_SIZE, 0);
        v
    }

    /// A real encoded trailer (slot table + header), valid magic+crc, the
    /// slot table offset pointing at its own position inside the fixture.
    fn trailer(at: u64) -> Vec<u8> {
        let mut m = Manifest::default();
        m.slots.push(Slot::new(0, 100, TPKG_FORMAT_ZIP, "/m"));
        crate::codec::encode_trailer(&m, at).unwrap()
    }

    /// The signed-post-press shape: [pe][trailer][zero pad][certificate].
    /// `cert_off` is the (8-aligned) certificate table start; the trailer
    /// ends at `cert_off - pad`.
    fn signed_package(cert_off: usize, cert_size: usize, pad: usize) -> Vec<u8> {
        let trailer_at = cert_off - pad - trailer(0).len();
        let mut v = signed_pe(cert_off as u32, cert_size as u32);
        assert!(trailer_at >= v.len());
        v.resize(trailer_at, 0);
        v.extend_from_slice(&trailer(trailer_at as u64));
        v.resize(cert_off, 0);
        let n = v.len();
        v.resize(n + cert_size, 0xbb);
        v
    }

    fn end_of(bytes: &[u8]) -> u64 {
        trailer_end(&mut Cursor::new(bytes), bytes.len() as u64).unwrap()
    }

    #[test]
    fn certificate_table_at_eof_reveals_the_trailer_end() {
        // [pe][trailer][certificate] — no alignment pad.
        let v = signed_package(0x400, 0x200, 0);
        assert_eq!(end_of(&v), 0x400);
        // … and the full reader resolves the buried manifest.
        let m = crate::io::read_from(&mut Cursor::new(&v)).unwrap();
        assert_eq!(m.slots.len(), 1);
        assert_eq!(m.slots[0].mount_point_str(), Some("/m"));
    }

    #[test]
    fn alignment_pad_before_the_certificate_table_is_skipped() {
        // WIN_CERTIFICATE entries are 8-byte aligned; the zero pad between
        // the trailer and the table is therefore shorter than 8.
        let trailer_end = 0x400usize - 5; // pad of 5 up to the aligned 0x400
        let v = signed_package(0x400, 0x200, 5);
        assert_eq!(end_of(&v), trailer_end as u64);
        crate::io::read_from(&mut Cursor::new(&v)).unwrap();
    }

    #[test]
    fn unsigned_pe_keeps_the_physical_eof() {
        // Absent security directory (zeroed entry).
        let mut v = signed_pe(0, 0);
        let at = v.len() as u64;
        v.extend_from_slice(&trailer(at));
        assert_eq!(end_of(&v), v.len() as u64);
        crate::io::read_from(&mut Cursor::new(&v)).unwrap();
        // A directory index past NumberOfRvaAndSizes reads as zero too.
        let mut v = signed_pe(0x400, 0x200);
        let num_rva = E_LFANEW_VALUE + 24 + 108;
        v[num_rva] = 4; // only directories 0..=3 exist
        let at = v.len() as u64;
        v.extend_from_slice(&trailer(at));
        assert_eq!(end_of(&v), v.len() as u64);
        crate::io::read_from(&mut Cursor::new(&v)).unwrap();
    }

    #[test]
    fn certificate_table_not_at_eof_keeps_the_physical_eof() {
        // Bytes after the certificate table: not the signed-package shape
        // (a mid-file stale table from a signed input bootstrap is dead
        // bytes, not a trailer burier).
        let mut v = signed_package(0x400, 0x200, 0);
        v.extend_from_slice(b"more");
        assert_eq!(end_of(&v), v.len() as u64);
    }

    #[test]
    fn certificate_table_without_a_valid_trailer_keeps_the_physical_eof() {
        // A certificate table at EOF but only zeros beneath it: not a
        // package.
        let mut v = signed_pe(0x400, 0x200);
        v.resize(0x400, 0);
        let n = v.len();
        v.resize(n + 0x200, 0xbb);
        assert_eq!(end_of(&v), v.len() as u64);
        // A corrupt trailer (bad crc) under the table is not located.
        let mut v = signed_package(0x400, 0x200, 0);
        v[0x400 - crate::TPKG_HEADER_SIZE + 20] ^= 0xff;
        assert_eq!(end_of(&v), v.len() as u64);
        // A nonzero byte between the trailer and the table is not the
        // alignment pad: the scan stops, the EOF stands.
        let mut v = signed_package(0x400, 0x200, 5);
        v[0x400 - 3] = 0x77;
        assert_eq!(end_of(&v), v.len() as u64);
    }

    #[test]
    fn malformed_pe_headers_keep_the_physical_eof_without_panicking() {
        // Every case: an unsigned PE with a trailer appended — a malformed
        // header field must leave the physical EOF (and the trailer)
        // untouched, never panic.
        let keeps_eof = |mut v: Vec<u8>| {
            let at = v.len() as u64;
            v.extend_from_slice(&trailer(at));
            assert_eq!(end_of(&v), v.len() as u64);
            crate::io::read_from(&mut Cursor::new(&v)).unwrap();
        };
        // e_lfanew past the physical EOF.
        let mut v = signed_pe(0, 0);
        v[E_LFANEW as usize] = 0xf0;
        v[E_LFANEW as usize + 1] = 0xff;
        keeps_eof(v);
        // Bad PE signature at e_lfanew.
        let mut v = signed_pe(0, 0);
        v[E_LFANEW_VALUE] = b'X';
        keeps_eof(v);
        // Unknown optional-header magic (ROM image, etc.).
        let mut v = signed_pe(0x400, 0x200);
        v[E_LFANEW_VALUE + 24] = 0x07; // 0x107 = ROM
        v[E_LFANEW_VALUE + 24 + 1] = 0x01;
        keeps_eof(v);
        // Optional header too short to hold the security directory.
        let mut v = signed_pe(0x400, 0x200);
        let sz = E_LFANEW_VALUE + 4 + 16;
        v[sz] = 0x70; // SizeOfOptionalHeader = 0x70 < 112 + 5*8
        v[sz + 1] = 0x00;
        keeps_eof(v);
        // Certificate range running past the physical EOF.
        keeps_eof(signed_pe(0x400, 0xffff_ffff));
        // A truncated DOS header (no room for e_lfanew).
        let v = b"MZ\x01".as_slice();
        assert_eq!(end_of(v), v.len() as u64);
    }
}
