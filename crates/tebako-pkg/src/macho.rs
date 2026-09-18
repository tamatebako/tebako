//! Mach-O code-signature excision for the press (spec 31 §1.2).
//!
//! A press starts from a SIGNED bootstrap. Appending slots + the trailer
//! invalidates the embedded signature, and the stale superblob mid-file
//! leaves the package unfixable for codesign: strict validation fails on
//! re-sign and `codesign --remove-signature` errors out. The press
//! therefore emits a cleanly-UNSIGNED Mach-O, in the same shape
//! `codesign --remove-signature` itself produces:
//!
//! - the LC_CODE_SIGNATURE load command is REMOVED from the command
//!   table (ncmds/sizeofcmds decremented, the vacated bytes zeroed — a
//!   merely zeroed dataoff/datasize pair is rejected by codesign as
//!   "invalid or unsupported format for signature");
//! - when the superblob is the file tail (it always is in linker
//!   output), the byte stream is truncated at the superblob's dataoff;
//! - the __LINKEDIT segment's filesize/vmsize cover every remaining byte
//!   (codesign strict-validates that the file ends inside a segment);
//! - and because the press then appends slots + the trailer, a fixup
//!   record lets the assembler extend __LINKEDIT over the FINAL package
//!   length once that length is known.
//!
//! Universal binaries (FAT_MAGIC/FAT_MAGIC_64) are excised per slice —
//! slice offsets never move: a middle slice keeps its bytes (its
//! __LINKEDIT covers the dead superblob region); only the slice that
//! reaches the file tail can truncate, and its fat_arch size is extended
//! by the same fixup so the appended regions stay inside the slice.
//!
//! Non-Mach-O inputs and Mach-Os without LC_CODE_SIGNATURE pass through
//! byte-identical (the unsigned-bootstrap golden path is a no-op).

/// `LC_CODE_SIGNATURE` (a `linkedit_data_command`: cmd/cmdsize/dataoff/
/// datasize).
const LC_CODE_SIGNATURE: u32 = 0x1d;
/// `LC_SEGMENT_64` — the carrier of `__LINKEDIT`.
const LC_SEGMENT_64: u32 = 0x19;
/// 64-bit Mach-O header size (`mach_header_64`).
const HEADER_64_LEN: usize = 32;
/// Smallest legal load command (`cmd` + `cmdsize`).
const LOAD_COMMAND_MIN_LEN: u32 = 8;
/// `linkedit_data_command` size.
const LINKEDIT_DATA_LEN: u32 = 16;
/// `fat_header`: magic + nfat_arch.
const FAT_HEADER_LEN: usize = 8;
/// `fat_arch` (32-bit offsets) / `fat_arch_64` entry sizes.
const FAT_ARCH_LEN: usize = 20;
const FAT_ARCH_64_LEN: usize = 32;
/// Page granularity used when growing `__LINKEDIT`'s vmsize (16 KiB —
/// the arm64 page size; a multiple of the x86_64 one).
const PAGE: u64 = 0x4000;

/// The excision decision for one input buffer.
#[derive(Debug)]
pub enum Excision {
    /// Passthrough: not a Mach-O, or a Mach-O without a live signature.
    Unchanged,
    /// The excised byte stream plus the __LINKEDIT fixup for the
    /// assembler: once the slots + trailer are appended, `apply` extends
    /// the tail slice's __LINKEDIT (and a fat tail slice's declared
    /// size) over the final file so the package stays codesign-able.
    Excised(Excised),
}

/// The excised byte stream plus its post-assembly fixups.
#[derive(Debug)]
pub struct Excised {
    pub bytes: Vec<u8>,
    /// Fixups for slices that reached the input's end (at most one for a
    /// thin binary; the tail slice for a universal).
    pub fixups: Vec<LinkeditFixup>,
}

/// How to extend one slice's __LINKEDIT over the final package length.
#[derive(Debug)]
pub struct LinkeditFixup {
    /// Absolute offset of the __LINKEDIT `filesize` field in the output.
    pub filesize_field: u64,
    /// Absolute offset of the __LINKEDIT `vmsize` field in the output.
    pub vmsize_field: u64,
    /// The slice's __LINKEDIT `fileoff` (slice-relative per the format).
    pub fileoff: u64,
    /// The slice's absolute start in the output.
    pub slice_start: u64,
    /// Field byte order.
    pub big_endian: bool,
    /// Universal binaries: the absolute offset of the tail slice's
    /// declared-size field and its width (4 bytes for FAT_MAGIC, 8 for
    /// FAT_MAGIC_64, always big-endian).
    pub fat_size_field: Option<(u64, usize)>,
}

impl LinkeditFixup {
    /// Patch the fields at their recorded offsets in `out` so the slice
    /// covers a file of `total_len` bytes. Pure byte surgery — the caller
    /// positions nothing.
    pub fn apply<W: std::io::Write + std::io::Seek>(
        &self,
        out: &mut W,
        total_len: u64,
    ) -> Result<(), String> {
        let filesize = total_len - self.slice_start - self.fileoff;
        let vmsize = filesize.div_ceil(PAGE) * PAGE;
        let mut write = |at: u64, width: usize, value: u64, be: bool| -> Result<(), String> {
            out.seek(std::io::SeekFrom::Start(at))
                .map_err(|_| "mach-o excise: fixup seek failed".to_string())?;
            let bytes = if be {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            };
            out.write_all(&bytes[8 - width..])
                .map_err(|_| "mach-o excise: fixup write failed".to_string())
        };
        write(self.filesize_field, 8, filesize, self.big_endian)?;
        write(self.vmsize_field, 8, vmsize, self.big_endian)?;
        if let Some((field, width)) = self.fat_size_field {
            write(field, width, total_len - self.slice_start, true)?;
        }
        Ok(())
    }
}

/// True when `magic` opens a 64-bit Mach-O (either endianness) or a
/// FAT_MAGIC/FAT_MAGIC_64 universal.
pub fn is_mach_o(magic: &[u8]) -> bool {
    matches!(
        magic,
        [0xcf, 0xfa, 0xed, 0xfe] // MH_MAGIC_64, little-endian
            | [0xfe, 0xed, 0xfa, 0xcf] // MH_CIGAM_64, big-endian
            | [0xca, 0xfe, 0xba, 0xbe] // FAT_MAGIC
            | [0xca, 0xfe, 0xba, 0xbf] // FAT_MAGIC_64
    )
}

/// A live LC_CODE_SIGNATURE found in a slice.
struct SignatureCommand {
    /// Slice-relative offset of the load command.
    command: usize,
    cmdsize: u32,
    dataoff: u64,
    datasize: u64,
}

/// A slice's parsed shape: its signature commands (if any), the
/// __LINKEDIT segment's location, and whether the command-table walk
/// finished cleanly. Walk errors are tolerated for slices WITHOUT a live
/// signature (an unsigned Mach-O passes through byte-identical no matter
/// how odd its tail is); they are fatal once a signature is being
/// excised.
struct SliceScan {
    signatures: Vec<SignatureCommand>,
    /// Slice-relative offset of the __LINKEDIT LC_SEGMENT_64 command.
    linkedit: Option<usize>,
    big_endian: bool,
    malformed: Option<String>,
}

fn u32_at(bytes: &[u8], at: usize, big_endian: bool) -> Result<u32, String> {
    let b = bytes
        .get(at..at + 4)
        .ok_or_else(|| format!("mach-o excise: truncated field at {at:#x}"))?;
    Ok(if big_endian {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    } else {
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    })
}

fn u64_at(bytes: &[u8], at: usize, big_endian: bool) -> Result<u64, String> {
    let b = bytes
        .get(at..at + 8)
        .ok_or_else(|| format!("mach-o excise: truncated field at {at:#x}"))?;
    Ok(if big_endian {
        u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    } else {
        u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    })
}

fn put_u32(bytes: &mut [u8], at: usize, v: u32, big_endian: bool) {
    let b = if big_endian {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    };
    bytes[at..at + 4].copy_from_slice(&b);
}

fn put_u64(bytes: &mut [u8], at: usize, v: u64, big_endian: bool) {
    let b = if big_endian {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    };
    bytes[at..at + 8].copy_from_slice(&b);
}

/// Parse one 64-bit slice's command table. Returns `Ok(None)` when the
/// slice is not a 64-bit Mach-O (left untouched).
fn scan_slice(input: &[u8], start: usize, end: usize) -> Result<Option<SliceScan>, String> {
    let big_endian = match input.get(start..start + 4) {
        Some([0xcf, 0xfa, 0xed, 0xfe]) => false,
        Some([0xfe, 0xed, 0xfa, 0xcf]) => true,
        _ => return Ok(None),
    };
    if end - start < HEADER_64_LEN {
        return Ok(Some(SliceScan {
            signatures: Vec::new(),
            linkedit: None,
            big_endian,
            malformed: Some("mach-o excise: truncated mach_header_64".to_string()),
        }));
    }
    let slice = &input[start..end];
    let slice_len = (end - start) as u64;
    let ncmds = match u32_at(slice, 16, big_endian) {
        Ok(n) => n as usize,
        Err(e) => {
            return Ok(Some(SliceScan {
                signatures: Vec::new(),
                linkedit: None,
                big_endian,
                malformed: Some(e),
            }))
        }
    };
    let mut scan = SliceScan {
        signatures: Vec::new(),
        linkedit: None,
        big_endian,
        malformed: None,
    };
    let mut at = HEADER_64_LEN;
    for _ in 0..ncmds {
        let (cmd, cmdsize) = match u32_at(slice, at, big_endian)
            .and_then(|c| u32_at(slice, at + 4, big_endian).map(|s| (c, s)))
        {
            Ok(v) => v,
            Err(e) => {
                scan.malformed = Some(e);
                break;
            }
        };
        if cmdsize < LOAD_COMMAND_MIN_LEN {
            scan.malformed = Some(format!(
                "mach-o excise: malformed load command at {at:#x} (cmdsize {cmdsize})"
            ));
            break;
        }
        let Some(next) = (at + cmdsize as usize <= slice.len()).then_some(at + cmdsize as usize)
        else {
            scan.malformed =
                Some("mach-o excise: load commands extend beyond the end of the slice".to_string());
            break;
        };
        match cmd {
            LC_CODE_SIGNATURE => {
                if cmdsize < LINKEDIT_DATA_LEN {
                    return Err(format!(
                        "mach-o excise: malformed LC_CODE_SIGNATURE at {at:#x} (cmdsize {cmdsize})"
                    ));
                }
                let dataoff = u32_at(slice, at + 8, big_endian)? as u64;
                let datasize = u32_at(slice, at + 12, big_endian)? as u64;
                // dataoff=0/datasize=0 is the no-signature marker already.
                if dataoff != 0 || datasize != 0 {
                    if dataoff > slice_len || datasize > slice_len - dataoff {
                        return Err(
                            "mach-o excise: LC_CODE_SIGNATURE superblob beyond the end of the file"
                                .to_string(),
                        );
                    }
                    scan.signatures.push(SignatureCommand {
                        command: at,
                        cmdsize,
                        dataoff,
                        datasize,
                    });
                }
            }
            LC_SEGMENT_64 => {
                if cmdsize < 72 {
                    scan.malformed = Some(format!(
                        "mach-o excise: malformed LC_SEGMENT_64 at {at:#x} (cmdsize {cmdsize})"
                    ));
                    break;
                }
                if slice.get(at + 8..at + 24) == Some(b"__LINKEDIT\0\0\0\0\0\0".as_slice()) {
                    match u64_at(slice, at + 40, big_endian)
                        .and_then(|f| u64_at(slice, at + 48, big_endian).map(|s| (f, s)))
                    {
                        Ok((fileoff, filesize))
                            if fileoff <= slice_len && filesize <= slice_len - fileoff =>
                        {
                            scan.linkedit = Some(at);
                        }
                        // An out-of-bounds __LINKEDIT is tolerated here;
                        // it becomes fatal only if a signature is excised.
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        at = next;
    }
    Ok(Some(scan))
}

/// Excise a stale code signature from a Mach-O buffer. Returns
/// `Excision::Unchanged` for non-Mach-O inputs and for Mach-Os that carry
/// no live LC_CODE_SIGNATURE (the unsigned-bootstrap golden path stays a
/// byte-identical no-op).
pub fn excise_code_signature(input: &[u8]) -> Result<Excision, String> {
    /// A slice window inside the input; `fat_size_field` is the absolute
    /// offset and width of the slice's declared-size field in the
    /// fat_arch table.
    #[derive(Clone, Copy)]
    struct Window {
        start: usize,
        end: usize,
        fat_size_field: Option<(u64, usize)>,
    }

    let Some(magic) = input.get(..4) else {
        return Ok(Excision::Unchanged);
    };
    let windows: Vec<Window> = match magic {
        [0xcf, 0xfa, 0xed, 0xfe] | [0xfe, 0xed, 0xfa, 0xcf] => vec![Window {
            start: 0,
            end: input.len(),
            fat_size_field: None,
        }],
        [0xca, 0xfe, 0xba, 0xbe] | [0xca, 0xfe, 0xba, 0xbf] => {
            let is_64 = magic[3] == 0xbf;
            let entry_len = if is_64 { FAT_ARCH_64_LEN } else { FAT_ARCH_LEN };
            if input.len() < FAT_HEADER_LEN {
                return Err("mach-o excise: truncated fat_header".to_string());
            }
            let nfat = u32_at(input, 4, true)? as usize;
            let table_end = FAT_HEADER_LEN
                .checked_add(
                    nfat.checked_mul(entry_len)
                        .ok_or("mach-o excise: fat header overflow")?,
                )
                .ok_or("mach-o excise: fat header overflow")?;
            if table_end > input.len() {
                return Err("mach-o excise: truncated fat_arch table".to_string());
            }
            let mut windows = Vec::with_capacity(nfat);
            for i in 0..nfat {
                let e = FAT_HEADER_LEN + i * entry_len;
                let (offset, size, size_field) = if is_64 {
                    (
                        u64_at(input, e + 8, true)?,
                        u64_at(input, e + 16, true)?,
                        (e + 16, 8usize),
                    )
                } else {
                    (
                        u32_at(input, e + 8, true)? as u64,
                        u32_at(input, e + 12, true)? as u64,
                        (e + 12, 4usize),
                    )
                };
                let offset = usize::try_from(offset)
                    .map_err(|_| "mach-o excise: fat slice offset overflow".to_string())?;
                let size = usize::try_from(size)
                    .map_err(|_| "mach-o excise: fat slice size overflow".to_string())?;
                let end = offset
                    .checked_add(size)
                    .filter(|n| *n <= input.len())
                    .ok_or_else(|| {
                        "mach-o excise: fat slice beyond the end of the file".to_string()
                    })?;
                windows.push(Window {
                    start: offset,
                    end,
                    fat_size_field: Some((size_field.0 as u64, size_field.1)),
                });
            }
            windows
        }
        _ => return Ok(Excision::Unchanged),
    };

    // Read-only pass: any live signature anywhere? A slice whose walk
    // broke down is fatal only when that slice carries a live signature
    // (an unsigned Mach-O passes through byte-identical no matter how
    // odd its tail is).
    let mut scans: Vec<(Window, SliceScan)> = Vec::new();
    for w in &windows {
        if let Some(scan) = scan_slice(input, w.start, w.end)? {
            if scan.signatures.is_empty() {
                continue;
            }
            if let Some(e) = &scan.malformed {
                return Err(e.clone());
            }
            scans.push((*w, scan));
        }
    }
    if scans.is_empty() {
        return Ok(Excision::Unchanged);
    }

    let mut out = input.to_vec();
    let mut fixups = Vec::new();
    let mut truncate_at: Option<usize> = None;

    for (w, scan) in &scans {
        let slice_len = (w.end - w.start) as u64;
        // Truncate only when this slice reaches the file tail and the
        // superblob is its last region.
        let sig = scan
            .signatures
            .iter()
            .max_by_key(|s| s.dataoff + s.datasize)
            .expect("non-empty");
        let tail_slice = w.end == input.len();
        let superblob_is_tail = sig.dataoff + sig.datasize == slice_len;
        let do_truncate = tail_slice && superblob_is_tail;
        let covered_len = if do_truncate { sig.dataoff } else { slice_len };

        // Remove the LC_CODE_SIGNATURE commands from the table (the
        // codesign --remove-signature shape): last-to-first so earlier
        // positions stay valid under the shift.
        let mut sigs: Vec<&SignatureCommand> = scan.signatures.iter().collect();
        sigs.sort_by_key(|s| s.command);
        let be = scan.big_endian;
        let mut sizeofcmds = u32_at(&out, w.start + 16 + 4, be)?;
        let mut ncmds = u32_at(&out, w.start + 16, be)?;
        for s in sigs.iter().rev() {
            let cmd_at = w.start + s.command;
            let area_end = w.start + HEADER_64_LEN + sizeofcmds as usize;
            let after = cmd_at + s.cmdsize as usize;
            out.copy_within(after..area_end, cmd_at);
            out[area_end - s.cmdsize as usize..area_end].fill(0);
            sizeofcmds -= s.cmdsize;
            ncmds -= 1;
        }
        put_u32(&mut out, w.start + 16, ncmds, be);
        put_u32(&mut out, w.start + 16 + 4, sizeofcmds, be);

        // The command-table shift may have moved __LINKEDIT; rescan.
        let lc = scan_slice(&out, w.start, w.end)?
            .and_then(|s| s.linkedit)
            .ok_or_else(|| {
                "mach-o excise: signed Mach-O without a __LINKEDIT segment".to_string()
            })?;
        let seg_at = w.start + lc;
        let fileoff = u64_at(&out, seg_at + 40, be)?;
        if fileoff > covered_len {
            return Err(
                "mach-o excise: __LINKEDIT fileoff beyond the truncation point".to_string(),
            );
        }
        let filesize = covered_len - fileoff;
        let vmsize = filesize.div_ceil(PAGE) * PAGE;
        put_u64(&mut out, seg_at + 48, filesize, be);
        put_u64(&mut out, seg_at + 32, vmsize, be);

        if tail_slice {
            fixups.push(LinkeditFixup {
                filesize_field: (seg_at + 48) as u64,
                vmsize_field: (seg_at + 32) as u64,
                fileoff,
                slice_start: w.start as u64,
                big_endian: be,
                fat_size_field: w.fat_size_field,
            });
            if do_truncate {
                truncate_at = Some(w.start + sig.dataoff as usize);
            }
        }
    }
    if let Some(at) = truncate_at {
        out.truncate(at);
    }
    Ok(Excision::Excised(Excised { bytes: out, fixups }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le32(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }

    fn be32(v: u32) -> [u8; 4] {
        v.to_be_bytes()
    }

    fn le64(v: u64) -> [u8; 8] {
        v.to_le_bytes()
    }

    const SEG_SIZE: u32 = 72;
    const CS_SIZE: u32 = 16;

    /// A 64-bit Mach-O: header + LC_SEGMENT_64("__LINKEDIT") +
    /// LC_CODE_SIGNATURE, filler up to `dataoff`, the superblob, then
    /// `tail` extra bytes. __LINKEDIT: fileoff=0x100, covers to the
    /// superblob end.
    fn thin(big_endian: bool, dataoff: u32, datasize: u32, tail: usize) -> Vec<u8> {
        let u32s = |v: u32| -> [u8; 4] {
            if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let u64s = |v: u64| -> [u8; 8] {
            if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let magic: [u8; 4] = if big_endian {
            [0xfe, 0xed, 0xfa, 0xcf]
        } else {
            [0xcf, 0xfa, 0xed, 0xfe]
        };
        let mut v = Vec::new();
        v.extend_from_slice(&magic);
        v.extend_from_slice(&u32s(0x0100_000c)); // CPU_TYPE_ARM64
        v.extend_from_slice(&u32s(0));
        v.extend_from_slice(&u32s(2)); // MH_EXECUTE
        v.extend_from_slice(&u32s(2)); // ncmds
        v.extend_from_slice(&u32s(SEG_SIZE + CS_SIZE)); // sizeofcmds
        v.extend_from_slice(&u32s(0)); // flags
        v.extend_from_slice(&u32s(0)); // reserved
        v.extend_from_slice(&u32s(LC_SEGMENT_64));
        v.extend_from_slice(&u32s(SEG_SIZE));
        v.extend_from_slice(b"__LINKEDIT\0\0\0\0\0\0");
        let fileoff = 0x100u64;
        let filesize = dataoff as u64 + datasize as u64 - fileoff;
        v.extend_from_slice(&u64s(0x1_0000_0000)); // vmaddr
        v.extend_from_slice(&u64s(filesize.div_ceil(PAGE) * PAGE)); // vmsize
        v.extend_from_slice(&u64s(fileoff));
        v.extend_from_slice(&u64s(filesize));
        v.extend_from_slice(&vec![0u8; (SEG_SIZE - 56) as usize]); // prot/nsects/flags
        v.extend_from_slice(&u32s(LC_CODE_SIGNATURE));
        v.extend_from_slice(&u32s(CS_SIZE));
        v.extend_from_slice(&u32s(dataoff));
        v.extend_from_slice(&u32s(datasize));
        assert!(dataoff as usize >= v.len());
        v.resize(dataoff as usize, 0xaa);
        v.extend_from_slice(&vec![0xbb; datasize as usize]);
        v.extend_from_slice(&vec![0xcc; tail]);
        v
    }

    /// The excised LC_CODE_SIGNATURE slot position in `thin`'s layout.
    const THIN_LC_AT: usize = HEADER_64_LEN + SEG_SIZE as usize;

    fn fat(is_64: bool, tail_slice0: usize) -> (Vec<u8>, usize, usize) {
        let s0 = thin(false, 0x400, 0x80, tail_slice0);
        let s1 = thin(false, 0x500, 0x40, 0);
        let entry_len = if is_64 { FAT_ARCH_64_LEN } else { FAT_ARCH_LEN };
        let table_end = FAT_HEADER_LEN + 2 * entry_len;
        let off0 = 0x1000usize;
        let off1 = 0x2000usize;
        let mut v = Vec::new();
        v.extend_from_slice(&be32(if is_64 { 0xcafe_babf } else { 0xcafe_babe }));
        v.extend_from_slice(&be32(2));
        for (i, s) in [&s0, &s1].iter().enumerate() {
            let off = if i == 0 { off0 } else { off1 };
            let len = s.len();
            v.extend_from_slice(&be32(0x0100_000c));
            v.extend_from_slice(&be32(0));
            if is_64 {
                v.extend_from_slice(&(off as u64).to_be_bytes());
                v.extend_from_slice(&(len as u64).to_be_bytes());
                v.extend_from_slice(&be32(12)); // align
                v.extend_from_slice(&be32(0)); // reserved
            } else {
                v.extend_from_slice(&be32(off as u32));
                v.extend_from_slice(&be32(len as u32));
                v.extend_from_slice(&be32(12));
            }
        }
        assert_eq!(v.len(), table_end);
        v.resize(off0, 0x00);
        v.extend_from_slice(&s0);
        v.resize(off1, 0x00);
        v.extend_from_slice(&s1);
        (v, off0, off1)
    }

    fn read_u64(out: &[u8], at: usize) -> u64 {
        u64::from_le_bytes(out[at..at + 8].try_into().unwrap())
    }

    #[test]
    fn thin_le_superblob_at_tail_truncates() {
        let input = thin(false, 0x400, 0x80, 0);
        let Excision::Excised(e) = excise_code_signature(&input).unwrap() else {
            panic!("expected excision");
        };
        let out = &e.bytes;
        assert_eq!(out.len(), 0x400);
        // The command is removed from the table: ncmds 2→1,
        // sizeofcmds 88→72, the vacated 16 bytes zeroed.
        assert_eq!(le32(1).as_slice(), &out[16..20]);
        assert_eq!(le32(SEG_SIZE).as_slice(), &out[20..24]);
        assert_eq!(
            &out[THIN_LC_AT..THIN_LC_AT + CS_SIZE as usize],
            &vec![0u8; CS_SIZE as usize][..]
        );
        // __LINKEDIT shrunk to the truncation point (fileoff=0x100).
        assert_eq!(read_u64(out, HEADER_64_LEN + 40), 0x100);
        assert_eq!(read_u64(out, HEADER_64_LEN + 48), 0x400 - 0x100);
        assert_eq!(read_u64(out, HEADER_64_LEN + 32), PAGE);
        // Everything outside the header counts, the __LINKEDIT
        // vmsize/filesize fields, and the vacated command is intact.
        assert_eq!(&out[..16], &input[..16]);
        assert_eq!(&out[24..64], &input[24..64]);
        assert_eq!(&out[72..80], &input[72..80]); // fileoff untouched
        assert_eq!(&out[88..THIN_LC_AT], &input[88..THIN_LC_AT]);
        // One fixup, for the tail slice.
        assert_eq!(e.fixups.len(), 1);
        assert_eq!(e.fixups[0].filesize_field, (HEADER_64_LEN + 48) as u64);
        assert!(e.fixups[0].fat_size_field.is_none());
    }

    #[test]
    fn fixup_extends_linkedit_over_the_appended_regions() {
        let input = thin(false, 0x400, 0x80, 0);
        let Excision::Excised(e) = excise_code_signature(&input).unwrap() else {
            panic!("expected excision");
        };
        // Simulate the assembled package: excised bytes + slots + trailer.
        let mut pkg = e.bytes.clone();
        pkg.extend_from_slice(&vec![0x77; 0x180]); // slots + trailer stand-in
        let total = pkg.len() as u64;
        let mut cursor = std::io::Cursor::new(&mut pkg);
        e.fixups[0].apply(&mut cursor, total).unwrap();
        let pkg = cursor.into_inner();
        assert_eq!(read_u64(pkg, HEADER_64_LEN + 48), total - 0x100);
        assert_eq!(
            read_u64(pkg, HEADER_64_LEN + 32),
            (total - 0x100).div_ceil(PAGE) * PAGE
        );
    }

    #[test]
    fn thin_le_superblob_not_tail_keeps_bytes() {
        let input = thin(false, 0x400, 0x80, 64);
        let Excision::Excised(e) = excise_code_signature(&input).unwrap() else {
            panic!("expected excision");
        };
        let out = &e.bytes;
        assert_eq!(out.len(), input.len());
        // LC removed; the superblob and the trailing bytes survive, now
        // covered by __LINKEDIT (filesize reaches the file end).
        assert_eq!(le32(1).as_slice(), &out[16..20]);
        assert_eq!(&out[0x400..0x480], &vec![0xbb; 0x80][..]);
        assert_eq!(&out[0x480..], &vec![0xcc; 64][..]);
        assert_eq!(
            read_u64(out, HEADER_64_LEN + 48),
            input.len() as u64 - 0x100
        );
        assert_eq!(e.fixups.len(), 1);
    }

    #[test]
    fn thin_be_excises() {
        let input = thin(true, 0x400, 0x80, 0);
        let Excision::Excised(e) = excise_code_signature(&input).unwrap() else {
            panic!("expected excision");
        };
        let out = &e.bytes;
        assert_eq!(out.len(), 0x400);
        assert_eq!(be32(1).as_slice(), &out[16..20]);
        assert_eq!(
            be32((0x400 - 0x100) as u32).as_slice(),
            &out[HEADER_64_LEN + 52..HEADER_64_LEN + 56]
        );
        assert!(e.fixups[0].big_endian);
    }

    #[test]
    fn mid_table_signature_command_is_removed_by_shifting() {
        // LC_CODE_SIGNATURE followed by another command (LC_SEGMENT_64
        // "__LINKEDIT" then LC_CODE_SIGNATURE then a second segment): the
        // removal shifts the trailing command down into place.
        let mut v = Vec::new();
        v.extend_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        v.extend_from_slice(&le32(0x0100_000c));
        v.extend_from_slice(&le32(0));
        v.extend_from_slice(&le32(2));
        v.extend_from_slice(&le32(3)); // ncmds
        v.extend_from_slice(&le32(SEG_SIZE + CS_SIZE + 8)); // sizeofcmds
        v.extend_from_slice(&le32(0));
        v.extend_from_slice(&le32(0));
        v.extend_from_slice(&le32(LC_SEGMENT_64));
        v.extend_from_slice(&le32(SEG_SIZE));
        v.extend_from_slice(b"__LINKEDIT\0\0\0\0\0\0");
        let filesize = 0x400u64 + 0x80 - 0x100;
        v.extend_from_slice(&le64(0x1_0000_0000));
        v.extend_from_slice(&le64(filesize.div_ceil(PAGE) * PAGE));
        v.extend_from_slice(&le64(0x100));
        v.extend_from_slice(&le64(filesize));
        v.extend_from_slice(&vec![0u8; (SEG_SIZE - 56) as usize]);
        v.extend_from_slice(&le32(LC_CODE_SIGNATURE));
        v.extend_from_slice(&le32(CS_SIZE));
        v.extend_from_slice(&le32(0x400));
        v.extend_from_slice(&le32(0x80));
        v.extend_from_slice(&le32(0x2c)); // LC_UUID, a trailing command
        v.extend_from_slice(&le32(8));
        v.resize(0x400, 0xaa);
        v.extend_from_slice(&[0xbb; 0x80]);

        let Excision::Excised(e) = excise_code_signature(&v).unwrap() else {
            panic!("expected excision");
        };
        let out = &e.bytes;
        assert_eq!(out.len(), 0x400);
        assert_eq!(le32(2).as_slice(), &out[16..20]);
        // The trailing LC_UUID shifted into the vacated slot.
        let seg_end = HEADER_64_LEN + SEG_SIZE as usize;
        assert_eq!(le32(0x2c).as_slice(), &out[seg_end..seg_end + 4]);
        assert_eq!(le32(8).as_slice(), &out[seg_end + 4..seg_end + 8]);
    }

    #[test]
    fn fat_two_slices_excises_per_slice_without_moving_boundaries() {
        for is_64 in [false, true] {
            let (input, off0, off1) = fat(is_64, 0x80);
            let Excision::Excised(e) = excise_code_signature(&input).unwrap() else {
                panic!("expected excision (is_64={is_64})");
            };
            let out = &e.bytes;
            // Slice 1's superblob is the file tail: truncated there.
            assert_eq!(out.len(), off1 + 0x500);
            // Both slices: LC removed (ncmds 2→1).
            assert_eq!(le32(1).as_slice(), &out[off0 + 16..off0 + 20]);
            assert_eq!(le32(1).as_slice(), &out[off1 + 16..off1 + 20]);
            // Slice 0 keeps its bytes; its __LINKEDIT covers to its end
            // (the dead superblob included).
            assert_eq!(&out[off0 + 0x400..off0 + 0x480], &vec![0xbb; 0x80][..]);
            assert_eq!(
                read_u64(out, off0 + HEADER_64_LEN + 48),
                (0x480 + 0x80) - 0x100
            );
            // Slice 1's __LINKEDIT is shrunk to the truncation point.
            assert_eq!(read_u64(out, off1 + HEADER_64_LEN + 48), 0x500 - 0x100);
            // Slice offsets never moved; the fat arch table's offset
            // fields are untouched.
            let entry_len = if is_64 { FAT_ARCH_64_LEN } else { FAT_ARCH_LEN };
            assert_eq!(
                &out[..FAT_HEADER_LEN + 2 * entry_len],
                &input[..FAT_HEADER_LEN + 2 * entry_len]
            );
            // The tail slice's fixup also names the fat_arch size field.
            assert_eq!(e.fixups.len(), 1);
            assert_eq!(
                e.fixups[0].fat_size_field,
                Some((
                    (FAT_HEADER_LEN + entry_len + if is_64 { 16 } else { 12 }) as u64,
                    if is_64 { 8 } else { 4 }
                ))
            );
        }
    }

    #[test]
    fn fat_fixup_extends_the_tail_slice_size() {
        let (input, _, _) = fat(false, 0x80);
        let Excision::Excised(e) = excise_code_signature(&input).unwrap() else {
            panic!("expected excision");
        };
        let mut pkg = e.bytes.clone();
        pkg.extend_from_slice(&vec![0x77; 0x180]);
        let total = pkg.len() as u64;
        let mut cursor = std::io::Cursor::new(&mut pkg);
        e.fixups[0].apply(&mut cursor, total).unwrap();
        let pkg = cursor.into_inner();
        // The tail slice's declared size (BE u32) now covers the whole file.
        let (size_field, width) = e.fixups[0].fat_size_field.unwrap();
        assert_eq!(width, 4);
        let size_field = size_field as usize;
        assert_eq!(
            u32::from_be_bytes(pkg[size_field..size_field + 4].try_into().unwrap()),
            (total - 0x2000) as u32
        );
    }

    #[test]
    fn no_code_signature_passes_through() {
        // A Mach-O whose only command is LC_SEGMENT_64 (ncmds=1).
        let mut v = Vec::new();
        v.extend_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        v.extend_from_slice(&le32(0x0100_000c));
        v.extend_from_slice(&le32(0));
        v.extend_from_slice(&le32(2));
        v.extend_from_slice(&le32(1));
        v.extend_from_slice(&le32(SEG_SIZE));
        v.extend_from_slice(&le32(0));
        v.extend_from_slice(&le32(0));
        v.extend_from_slice(&le32(LC_SEGMENT_64));
        v.extend_from_slice(&le32(SEG_SIZE));
        v.extend_from_slice(&vec![0x55; (SEG_SIZE - 8) as usize]);
        v.extend_from_slice(&[0xdd; 100]);
        assert!(matches!(
            excise_code_signature(&v).unwrap(),
            Excision::Unchanged
        ));
        // An already-neutralized LC (dataoff=0/datasize=0) is also a no-op.
        let mut input = thin(false, 0x400, 0x80, 0);
        input[THIN_LC_AT + 8..THIN_LC_AT + 16].fill(0);
        assert!(matches!(
            excise_code_signature(&input).unwrap(),
            Excision::Unchanged
        ));
    }

    #[test]
    fn non_macho_passes_through() {
        let pe = b"MZ\x90\x00 this is a PE file".as_slice();
        assert!(matches!(
            excise_code_signature(pe).unwrap(),
            Excision::Unchanged
        ));
        let elf = b"\x7fELF\x02\x01\x01\x00".as_slice();
        assert!(matches!(
            excise_code_signature(elf).unwrap(),
            Excision::Unchanged
        ));
        let short = b"\xcf\xfa".as_slice();
        assert!(matches!(
            excise_code_signature(short).unwrap(),
            Excision::Unchanged
        ));
    }

    #[test]
    fn malformed_inputs_error() {
        // The superblob beyond the end of the file.
        let mut v = thin(false, 0x400, 0x80, 0);
        let n = v.len();
        v[THIN_LC_AT + 8..THIN_LC_AT + 12].copy_from_slice(&le32((n + 64) as u32));
        assert!(excise_code_signature(&v).is_err());
        // Fat slice beyond the end of the file.
        let (mut v, _, _) = fat(false, 0x80);
        v.truncate(0x1800);
        assert!(excise_code_signature(&v).is_err());
        // A signed Mach-O with no __LINKEDIT segment.
        let mut v = thin(false, 0x400, 0x80, 0);
        v[HEADER_64_LEN + 8..HEADER_64_LEN + 16].copy_from_slice(b"__TEXT\0\0");
        assert!(excise_code_signature(&v).is_err());
    }
}
