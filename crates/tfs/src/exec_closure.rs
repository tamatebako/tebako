//! Executable dependency-closure parsing for `dlmap2file`.
//!
//! A materialized executable or shared library is loaded by the
//! platform loader (dyld, ld.so, the windows loader), whose path probes
//! are RAW SYSCALLS —
//! the preload shim cannot interpose them (proven on macOS 15: dyld's
//! rpath probes never reach the interposed `open`). The only way to
//! satisfy them is to materialize the dependency closure EAGERLY into
//! the dlmap layout, which mirrors the memfs tree exactly, so the
//! loader's executable-relative candidates hit real host files.
//!
//! This module is the pure parser half: bytes → (dependency names,
//! rpaths). Resolution of those names against the mounts (rpath
//! expansion, `@executable_path` / `@loader_path` / `$ORIGIN`, PE's
//! importer-dir-only rule, recursion with a visited set) lives in
//! `context.rs`.
//!
//! Mach-O and ELF parse only the header region (the first
//! [`HEADER_WINDOW`] bytes of the extracted copy): their load commands
//! and dynamic tables ride the first pages of any real image. PE is the
//! exception (incident 13): the import directory is SECTION-resident
//! (.rdata), which sits past the window in a multi-MiB module (a
//! -static-libstdc++ build) — a windowed parse silently answers an
//! empty closure, so [`parse`] hands PE the whole buffer it was given
//! (the caller reads PE images in full). A truncated or unparseable
//! header yields no dependencies — the loader then answers for the host
//! libraries exactly as before.

/// Header bytes examined for dependency metadata.
pub(crate) const HEADER_WINDOW: usize = 1 << 20;

/// The executable container format a header parsed as. Resolution
/// semantics differ per format — PE has no rpath: its bare imports
/// resolve importer-dir-only (spec 22 §2.1) — so the parse result
/// carries it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImageFormat {
    /// Mach-O (thin 32/64 or a fat slice).
    #[default]
    MachO,
    /// ELF (32/64, either endianness).
    Elf,
    /// PE32/PE32+ (a windows DLL/EXE import directory).
    Pe,
}

/// A parsed image's dependency names and its own rpath/runpath list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImageDeps {
    /// The container format the bytes parsed as.
    pub format: ImageFormat,
    /// LC_LOAD_DYLIB / DT_NEEDED / import-descriptor names, verbatim.
    pub deps: Vec<String>,
    /// LC_RPATH / DT_RPATH / DT_RUNPATH entries, verbatim. Always empty
    /// for PE — no rpath exists on PE (spec 22 §2.1).
    pub rpaths: Vec<String>,
}

/// Parse a Mach-O (thin 32/64 or fat), ELF (32/64, either endianness),
/// or PE (PE32/PE32+) header. None when the bytes are none of the
/// formats.
pub fn parse(bytes: &[u8]) -> Option<ImageDeps> {
    let window = &bytes[..bytes.len().min(HEADER_WINDOW)];
    // PE parses the WHOLE buffer (the module doc — incident 13): its
    // import directory is section-resident, not header-resident.
    parse_macho(window)
        .or_else(|| parse_elf(window))
        .or_else(|| parse_pe(bytes))
}

/// Cheap magic-number answer: does this file CLAIM to be a load module?
/// Mirrors exactly the magics parse() recognizes (MZ, ELF, Mach-O fat
/// big-endian and thin little-endian). Used to split parse() failures
/// into "malformed load module" (its consumer is a platform loader that
/// raises its own error — the module rides alone, no sibling sweep) and
/// "genuine data file" (whose consumer can address sibling resources
/// relative to it, so the directory's files materialize with it).
pub fn sniffs_load_module(head: &[u8]) -> bool {
    let Some(magic) = head.get(..4) else {
        return false;
    };
    if magic.starts_with(b"MZ") || magic == b"\x7FELF" {
        return true;
    }
    if u32be(magic).is_some_and(|m| m == FAT_MAGIC || m == FAT_MAGIC_64) {
        return true;
    }
    u32le(magic).is_some_and(|m| m == MH_MAGIC || m == MH_MAGIC_64)
}

// ---------------------------------------------------------------------
// Mach-O
// ---------------------------------------------------------------------

const MH_MAGIC_64: u32 = 0xFEED_FACF;
const MH_MAGIC: u32 = 0xFEED_FACE;
const FAT_MAGIC: u32 = 0xCAFE_BABE;
const FAT_MAGIC_64: u32 = 0xCAFE_BABF;

const CPU_TYPE_ARM64: u32 = 0x0100_000C;
const CPU_TYPE_X86_64: u32 = 0x0100_0007;

const LC_LOAD_DYLIB: u32 = 0x0C;
const LC_ID_DYLIB: u32 = 0x0D;
const LC_LOAD_WEAK_DYLIB: u32 = 0x18 | 0x8000_0000; // LC_REQ_DYLD
const LC_REEXPORT_DYLIB: u32 = 0x1F;
const LC_RPATH: u32 = 0x1C | 0x8000_0000; // LC_REQ_DYLD

fn parse_macho(bytes: &[u8]) -> Option<ImageDeps> {
    let magic = u32be(bytes.get(..4)?)?;
    match magic {
        FAT_MAGIC | FAT_MAGIC_64 => parse_fat(bytes, magic),
        _ => {
            let magic = u32le(bytes.get(..4)?)?;
            parse_thin_macho(bytes, magic)
        }
    }
}

/// Fat binary: pick the slice for the host CPU (arm64 / x86_64), then
/// parse it as a thin image.
fn parse_fat(bytes: &[u8], magic: u32) -> Option<ImageDeps> {
    let fat64 = magic == FAT_MAGIC_64;
    let nfat = u32be(bytes.get(4..8)?)? as usize;
    let host_cpus = if cfg!(target_arch = "aarch64") {
        [CPU_TYPE_ARM64, CPU_TYPE_X86_64]
    } else {
        [CPU_TYPE_X86_64, CPU_TYPE_ARM64]
    };
    let mut first: Option<(u64, u64)> = None;
    for i in 0..nfat {
        // fat_arch: cputype, cpusubtype, offset, size, align (20 bytes);
        // fat_arch_64 adds a reserved word (32 bytes).
        let base = 8 + i * if fat64 { 32 } else { 20 };
        let cputype = u32be(bytes.get(base..base + 4)?)?;
        let (offset, size) = if fat64 {
            (
                u64be(bytes.get(base + 8..base + 16)?)?,
                u64be(bytes.get(base + 16..base + 24)?)?,
            )
        } else {
            (
                u32be(bytes.get(base + 8..base + 12)?)? as u64,
                u32be(bytes.get(base + 12..base + 16)?)? as u64,
            )
        };
        if first.is_none() {
            first = Some((offset, size));
        }
        if host_cpus.contains(&cputype) {
            return parse_macho(slice(bytes, offset, size)?);
        }
    }
    // No host slice: fall back to the first slice's metadata (its
    // dependency names are representative across slices in practice).
    let (offset, size) = first?;
    parse_macho(slice(bytes, offset, size)?)
}

fn parse_thin_macho(bytes: &[u8], magic: u32) -> Option<ImageDeps> {
    let is64 = match magic {
        MH_MAGIC_64 => true,
        MH_MAGIC => false,
        _ => return None,
    };
    // mach_header(_64): magic, cputype, cpusubtype, filetype, ncmds,
    // sizeofcmds, flags (, reserved).
    let ncmds = u32le(bytes.get(16..20)?)? as usize;
    let sizeofcmds = u32le(bytes.get(20..24)?)? as usize;
    let mut at: usize = if is64 { 32 } else { 28 };
    let end = at.checked_add(sizeofcmds)?.min(bytes.len());
    let mut out = ImageDeps {
        format: ImageFormat::MachO,
        ..ImageDeps::default()
    };
    for _ in 0..ncmds {
        let cmd = u32le(bytes.get(at..at + 4)?)?;
        let cmdsize = u32le(bytes.get(at + 4..at + 8)?)? as usize;
        if cmdsize < 8 || at + cmdsize > bytes.len() {
            break;
        }
        let body = &bytes[at..at + cmdsize];
        match cmd {
            LC_LOAD_DYLIB | LC_LOAD_WEAK_DYLIB | LC_REEXPORT_DYLIB => {
                // dylib_command: lc_str name at offset 8 (offset is
                // relative to the start of the load command).
                if let Some(name) = lc_str(body, 8) {
                    out.deps.push(name);
                }
            }
            LC_RPATH => {
                if let Some(path) = lc_str(body, 8) {
                    out.rpaths.push(path);
                }
            }
            _ => {}
        }
        at += cmdsize;
        if at >= end {
            break;
        }
    }
    let _ = LC_ID_DYLIB; // the image's own name — not a dependency
    Some(out)
}

/// Read an lc_str field: u32 offset at `field`, NUL-terminated string
/// at that offset from the start of the load command.
fn lc_str(cmd: &[u8], field: usize) -> Option<String> {
    let offset = u32le(cmd.get(field..field + 4)?)? as usize;
    let start = offset.max(field + 4);
    let tail = cmd.get(start..)?;
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    let s = std::str::from_utf8(&tail[..end]).ok()?;
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

// ---------------------------------------------------------------------
// ELF
// ---------------------------------------------------------------------

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const DT_NEEDED: i64 = 1;
const DT_STRTAB: i64 = 5;
const DT_STRSZ: i64 = 10;
const DT_RPATH: i64 = 15;
const DT_RUNPATH: i64 = 29;
const DT_NULL: i64 = 0;

fn parse_elf(bytes: &[u8]) -> Option<ImageDeps> {
    if bytes.get(..4)? != b"\x7fELF" {
        return None;
    }
    let is64 = match bytes[4] {
        1 => false,
        2 => true,
        _ => return None,
    };
    let little = match bytes[5] {
        1 => true,
        2 => false,
        _ => return None,
    };
    let u16e = |r: &[u8]| -> Option<u16> {
        Some(if little {
            u16::from_le_bytes(r.get(..2)?.try_into().ok()?)
        } else {
            u16::from_be_bytes(r.get(..2)?.try_into().ok()?)
        })
    };
    let u32e = |r: &[u8]| -> Option<u32> {
        Some(if little {
            u32::from_le_bytes(r.get(..4)?.try_into().ok()?)
        } else {
            u32::from_be_bytes(r.get(..4)?.try_into().ok()?)
        })
    };
    let u64e = |r: &[u8]| -> Option<u64> {
        Some(if little {
            u64::from_le_bytes(r.get(..8)?.try_into().ok()?)
        } else {
            u64::from_be_bytes(r.get(..8)?.try_into().ok()?)
        })
    };

    let (phoff, phentsize, phnum) = if is64 {
        (
            u64e(bytes.get(32..40)?)? as usize,
            u16e(bytes.get(54..56)?)? as usize,
            u16e(bytes.get(56..58)?)? as usize,
        )
    } else {
        (
            u32e(bytes.get(28..32)?)? as usize,
            u16e(bytes.get(42..44)?)? as usize,
            u16e(bytes.get(44..46)?)? as usize,
        )
    };

    // Program headers: collect PT_LOAD (vaddr→offset) and PT_DYNAMIC.
    let mut loads: Vec<(u64, u64, u64)> = Vec::new(); // (vaddr, offset, filesz)
    let mut dynamic: Option<(usize, usize)> = None; // (offset, filesz)
    for i in 0..phnum {
        let base = phoff.checked_add(i.checked_mul(phentsize)?)?;
        let p_type = u32e(bytes.get(base..base + 4)?)?;
        let (p_offset, p_vaddr, p_filesz) = if is64 {
            (
                u64e(bytes.get(base + 8..base + 16)?)?,
                u64e(bytes.get(base + 16..base + 24)?)?,
                u64e(bytes.get(base + 32..base + 40)?)?,
            )
        } else {
            (
                u32e(bytes.get(base + 4..base + 8)?)? as u64,
                u32e(bytes.get(base + 8..base + 12)?)? as u64,
                u32e(bytes.get(base + 16..base + 20)?)? as u64,
            )
        };
        match p_type {
            PT_LOAD => loads.push((p_vaddr, p_offset, p_filesz)),
            PT_DYNAMIC => dynamic = Some((p_offset as usize, p_filesz as usize)),
            _ => {}
        }
    }
    let (dyn_off, dyn_filesz) = dynamic?;
    let vaddr_to_off = |vaddr: u64| -> Option<usize> {
        loads
            .iter()
            .find(|(v, _, filesz)| vaddr >= *v && vaddr < v + filesz)
            .map(|(v, off, _)| (vaddr - v + off) as usize)
    };

    // Dynamic entries: (i64 tag, u64 val) on 64-bit, (i32, u32) on 32.
    let entsz = if is64 { 16 } else { 8 };
    let mut strtab: Option<usize> = None;
    let mut strsz: Option<usize> = None;
    let mut needed: Vec<u64> = Vec::new();
    let mut rpath: Option<u64> = None;
    let mut runpath: Option<u64> = None;
    for i in 0..(dyn_filesz / entsz) {
        let base = dyn_off.checked_add(i.checked_mul(entsz)?)?;
        let (tag, val) = if is64 {
            (
                u64e(bytes.get(base..base + 8)?)? as i64,
                u64e(bytes.get(base + 8..base + 16)?)?,
            )
        } else {
            (
                u32e(bytes.get(base..base + 4)?)? as i32 as i64,
                u32e(bytes.get(base + 4..base + 8)?)? as u64,
            )
        };
        match tag {
            DT_NULL => break,
            DT_NEEDED => needed.push(val),
            DT_STRTAB => strtab = vaddr_to_off(val),
            DT_STRSZ => strsz = Some(val as usize),
            DT_RPATH => rpath = Some(val),
            DT_RUNPATH => runpath = Some(val),
            _ => {}
        }
    }
    let strtab = strtab?;
    let strsz = strsz
        .unwrap_or(usize::MAX)
        .min(bytes.len().saturating_sub(strtab));
    let strings = bytes.get(strtab..strtab.checked_add(strsz)?)?;
    let str_at = |offset: u64| -> Option<String> {
        let tail = strings.get(offset as usize..)?;
        let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        let s = std::str::from_utf8(&tail[..end]).ok()?;
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    };

    let mut out = ImageDeps {
        format: ImageFormat::Elf,
        ..ImageDeps::default()
    };
    for offset in needed {
        if let Some(name) = str_at(offset) {
            out.deps.push(name);
        }
    }
    // DT_RPATH is superseded by DT_RUNPATH when both are present.
    if let Some(val) = runpath.or(rpath) {
        // A (run)path value is a colon-separated list.
        if let Some(list) = str_at(val) {
            for entry in list.split(':') {
                if !entry.is_empty() {
                    out.rpaths.push(entry.to_string());
                }
            }
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------
// PE (PE32/PE32+) — spec 22 §2.1: the import directory joins the walk
// as the third parsed format, the PE analogue of DT_NEEDED/LC_LOAD_DYLIB.
// ---------------------------------------------------------------------

const PE32_MAGIC: u16 = 0x10B;
const PE32PLUS_MAGIC: u16 = 0x20B;
/// Data directory index of the import directory. The DELAY-load import
/// directory (index 13) is deliberately never read — delay-load is out
/// of phase W (spec 22 §2.1: no proven consumer).
const PE_DIRECTORY_IMPORT: usize = 1;
const PE_SECTION_HEADER_SIZE: usize = 40;
const PE_IMPORT_DESCRIPTOR_SIZE: usize = 20;

/// Parse a PE32/PE32+ header's import directory: one dependency name
/// per import descriptor, in descriptor order, verbatim. `rpaths` stays
/// empty — no rpath exists on PE. A PE with no import directory parses
/// as a dependency-free image; a structurally broken header (bad DOS/PE
/// signature, truncated COFF/optional header, an import-directory RVA
/// that maps nowhere) is None — not an image this walk reads.
fn parse_pe(bytes: &[u8]) -> Option<ImageDeps> {
    if bytes.get(..2)? != b"MZ" {
        return None;
    }
    // e_lfanew @ 0x3C → "PE\0\0" + the 20-byte COFF header.
    let pe_off = u32le(bytes.get(0x3C..0x40)?)? as usize;
    if bytes.get(pe_off..pe_off + 4)? != b"PE\0\0" {
        return None;
    }
    let coff = pe_off.checked_add(4)?;
    let nsections = u16le(bytes.get(coff + 2..coff + 4)?)? as usize;
    let opt_size = u16le(bytes.get(coff + 16..coff + 18)?)? as usize;
    let opt = coff + 20;
    let opt_bytes = bytes.get(opt..opt.checked_add(opt_size)?)?;
    let (ndirs_off, dirs_off): (usize, usize) = match u16le(opt_bytes.get(..2)?)? {
        PE32_MAGIC => (92, 96),
        PE32PLUS_MAGIC => (108, 112),
        _ => return None,
    };
    let ndirs = u32le(opt_bytes.get(ndirs_off..ndirs_off + 4)?)? as usize;
    let size_of_headers = u32le(opt_bytes.get(60..64)?)?;

    // Section table right after the optional header: RVA → file offset.
    // A section's raw bytes map [va, va + size_of_raw_data); RVAs below
    // SizeOfHeaders name the mapped header region 1:1 (u64 math — a
    // malformed va+size must never wrap).
    let sec_off = opt.checked_add(opt_size)?;
    let rva_to_off = |rva: u32| -> Option<usize> {
        let rva = rva as u64;
        for i in 0..nsections {
            let base = sec_off.checked_add(i.checked_mul(PE_SECTION_HEADER_SIZE)?)?;
            let sh = bytes.get(base..base + PE_SECTION_HEADER_SIZE)?;
            let va = u32le(sh.get(12..16)?)? as u64;
            let raw_size = u32le(sh.get(16..20)?)? as u64;
            let raw_off = u32le(sh.get(20..24)?)? as u64;
            if raw_size != 0 && (va..va + raw_size).contains(&rva) {
                return usize::try_from(raw_off + (rva - va)).ok();
            }
        }
        if rva < size_of_headers as u64 {
            usize::try_from(rva).ok()
        } else {
            None
        }
    };

    let mut out = ImageDeps {
        format: ImageFormat::Pe,
        ..ImageDeps::default()
    };
    if ndirs <= PE_DIRECTORY_IMPORT {
        return Some(out); // no import directory at all
    }
    let dir_at = dirs_off.checked_add(PE_DIRECTORY_IMPORT.checked_mul(8)?)?;
    let dir = opt_bytes.get(dir_at..dir_at + 8)?;
    let dir_rva = u32le(dir.get(..4)?)?;
    let dir_size = u32le(dir.get(4..8)?)? as usize;
    if dir_rva == 0 {
        return Some(out); // no imports
    }
    let imp_off = rva_to_off(dir_rva)?;
    // The descriptor array runs to its all-zero terminator, bounded by
    // the declared directory size when one is declared.
    let dir_end = if dir_size == 0 {
        usize::MAX
    } else {
        imp_off.checked_add(dir_size)?
    };
    let mut at = imp_off;
    while at.checked_add(PE_IMPORT_DESCRIPTOR_SIZE)? <= dir_end {
        let Some(desc) = bytes.get(at..at + PE_IMPORT_DESCRIPTOR_SIZE) else {
            break;
        };
        if desc.iter().all(|&b| b == 0) {
            break;
        }
        // IMAGE_IMPORT_DESCRIPTOR: Name (an RVA) @ +12.
        let name_rva = u32le(desc.get(12..16)?)?;
        if name_rva != 0 {
            if let Some(name) = rva_to_off(name_rva).and_then(|off| c_str_at(bytes, off)) {
                out.deps.push(name);
            }
        }
        at += PE_IMPORT_DESCRIPTOR_SIZE;
    }
    Some(out)
}

/// Read a NUL-terminated UTF-8 string at `off` (the import name).
fn c_str_at(bytes: &[u8], off: usize) -> Option<String> {
    let tail = bytes.get(off..)?;
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    let s = std::str::from_utf8(&tail[..end]).ok()?;
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

// ---------------------------------------------------------------------
// byte helpers
// ---------------------------------------------------------------------

fn u16le(r: &[u8]) -> Option<u16> {
    Some(u16::from_le_bytes(r.get(..2)?.try_into().ok()?))
}

fn u32le(r: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(r.get(..4)?.try_into().ok()?))
}

fn u32be(r: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(r.get(..4)?.try_into().ok()?))
}

fn u64be(r: &[u8]) -> Option<u64> {
    Some(u64::from_be_bytes(r.get(..8)?.try_into().ok()?))
}

fn slice(bytes: &[u8], offset: u64, size: u64) -> Option<&[u8]> {
    let start = usize::try_from(offset).ok()?;
    let end = start.checked_add(usize::try_from(size).ok()?)?;
    Some(&bytes[start..end.min(bytes.len())])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal 64-bit thin Mach-O with the given dylib
    /// references and rpaths.
    fn macho64(deps: &[&str], rpaths: &[&str]) -> Vec<u8> {
        let mut cmds = Vec::new();
        for name in deps {
            let name_bytes = name.as_bytes();
            let cmdsize = (24 + name_bytes.len() + 1 + 7) & !7;
            let mut cmd = vec![0u8; cmdsize];
            cmd[0..4].copy_from_slice(&LC_LOAD_DYLIB.to_le_bytes());
            cmd[4..8].copy_from_slice(&(cmdsize as u32).to_le_bytes());
            cmd[8..12].copy_from_slice(&24_u32.to_le_bytes()); // name offset
            cmd[24..24 + name_bytes.len()].copy_from_slice(name_bytes);
            cmds.extend_from_slice(&cmd);
        }
        for path in rpaths {
            let path_bytes = path.as_bytes();
            let cmdsize = (12 + path_bytes.len() + 1 + 7) & !7;
            let mut cmd = vec![0u8; cmdsize];
            cmd[0..4].copy_from_slice(&LC_RPATH.to_le_bytes());
            cmd[4..8].copy_from_slice(&(cmdsize as u32).to_le_bytes());
            cmd[8..12].copy_from_slice(&12_u32.to_le_bytes()); // path offset
            cmd[12..12 + path_bytes.len()].copy_from_slice(path_bytes);
            cmds.extend_from_slice(&cmd);
        }
        let ncmds = deps.len() + rpaths.len();
        let mut out = Vec::new();
        out.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
        out.extend_from_slice(&CPU_TYPE_ARM64.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes()); // cpusubtype
        out.extend_from_slice(&2_u32.to_le_bytes()); // MH_EXECUTE
        out.extend_from_slice(&(ncmds as u32).to_le_bytes());
        out.extend_from_slice(&(cmds.len() as u32).to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes()); // flags
        out.extend_from_slice(&0_u32.to_le_bytes()); // reserved
        out.extend_from_slice(&cmds);
        out
    }

    #[test]
    fn macho64_deps_and_rpaths() {
        let bytes = macho64(
            &[
                "@rpath/libinkscape_base.1.dylib",
                "/usr/lib/libSystem.B.dylib",
            ],
            &[
                "@executable_path/../lib",
                "@executable_path/../lib/inkscape",
            ],
        );
        let parsed = parse(&bytes).expect("a Mach-O parses");
        assert_eq!(
            parsed.deps,
            vec![
                "@rpath/libinkscape_base.1.dylib".to_string(),
                "/usr/lib/libSystem.B.dylib".to_string()
            ]
        );
        assert_eq!(
            parsed.rpaths,
            vec![
                "@executable_path/../lib".to_string(),
                "@executable_path/../lib/inkscape".to_string()
            ]
        );
    }

    #[test]
    fn fat_picks_host_slice() {
        let thin = macho64(&["@rpath/libfoo.dylib"], &["@executable_path/../lib"]);
        let mut fat = Vec::new();
        fat.extend_from_slice(&FAT_MAGIC.to_be_bytes());
        fat.extend_from_slice(&1_u32.to_be_bytes()); // nfat_arch
        fat.extend_from_slice(&CPU_TYPE_ARM64.to_be_bytes());
        fat.extend_from_slice(&0_u32.to_be_bytes()); // cpusubtype
        fat.extend_from_slice(&28_u32.to_be_bytes()); // offset
        fat.extend_from_slice(&(thin.len() as u32).to_be_bytes()); // size
        fat.extend_from_slice(&0_u32.to_be_bytes()); // align
        fat.extend_from_slice(&thin);
        let parsed = parse(&fat).expect("a fat Mach-O parses");
        assert_eq!(parsed.deps, vec!["@rpath/libfoo.dylib".to_string()]);
    }

    /// Build a minimal 64-bit LE ELF with one PT_LOAD covering the
    /// whole file and a PT_DYNAMIC with the given needed/runpath
    /// strings.
    fn elf64(needed: &[&str], runpath: Option<&str>) -> Vec<u8> {
        let phoff = 64usize;
        let phentsize = 56usize;
        let phnum = 2usize;
        let dyn_off = (phoff + phentsize * phnum + 7) & !7;
        // string table right after the dynamic section (3 entries + null)
        let dyn_entries = 3 + usize::from(runpath.is_some()) + 1;
        let strtab_off = dyn_off + dyn_entries * 16;
        let mut strings = vec![0u8];
        let mut offsets = Vec::new();
        for name in needed {
            offsets.push(strings.len() as u64);
            strings.extend_from_slice(name.as_bytes());
            strings.push(0);
        }
        let runpath_off = runpath.map(|rp| {
            let off = strings.len() as u64;
            strings.extend_from_slice(rp.as_bytes());
            strings.push(0);
            off
        });
        let total = strtab_off + strings.len();

        let mut out = vec![0u8; total];
        out[0..4].copy_from_slice(b"\x7fELF");
        out[4] = 2; // 64-bit
        out[5] = 1; // little-endian
        out[6] = 1; // version
        out[16..18].copy_from_slice(&3_u16.to_le_bytes()); // ET_DYN
        out[18..20].copy_from_slice(&62_u16.to_le_bytes()); // EM_X86_64
        out[32..40].copy_from_slice(&(phoff as u64).to_le_bytes());
        out[54..56].copy_from_slice(&(phentsize as u16).to_le_bytes());
        out[56..58].copy_from_slice(&(phnum as u16).to_le_bytes());
        // PT_LOAD covering the whole file at vaddr 0
        out[phoff..phoff + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
        out[phoff + 8..phoff + 16].copy_from_slice(&0_u64.to_le_bytes()); // offset
        out[phoff + 16..phoff + 24].copy_from_slice(&0_u64.to_le_bytes()); // vaddr
        out[phoff + 32..phoff + 40].copy_from_slice(&(total as u64).to_le_bytes()); // filesz
                                                                                    // PT_DYNAMIC
        let dph = phoff + phentsize;
        out[dph..dph + 4].copy_from_slice(&PT_DYNAMIC.to_le_bytes());
        out[dph + 8..dph + 16].copy_from_slice(&(dyn_off as u64).to_le_bytes());
        out[dph + 32..dph + 40].copy_from_slice(&((dyn_entries * 16) as u64).to_le_bytes());
        // dynamic entries
        let mut at = dyn_off;
        for off in &offsets {
            out[at..at + 8].copy_from_slice(&1_u64.to_le_bytes()); // DT_NEEDED
            out[at + 8..at + 16].copy_from_slice(&off.to_le_bytes());
            at += 16;
        }
        if let Some(off) = runpath_off {
            out[at..at + 8].copy_from_slice(&29_u64.to_le_bytes()); // DT_RUNPATH
            out[at + 8..at + 16].copy_from_slice(&off.to_le_bytes());
            at += 16;
        }
        out[at..at + 8].copy_from_slice(&5_u64.to_le_bytes()); // DT_STRTAB
        out[at + 8..at + 16].copy_from_slice(&(strtab_off as u64).to_le_bytes());
        at += 16;
        // DT_NULL
        let _ = at;
        out[strtab_off..strtab_off + strings.len()].copy_from_slice(&strings);
        out
    }

    #[test]
    fn elf64_needed_and_runpath() {
        let bytes = elf64(
            &["libfoo.so.1", "libc.so.6"],
            Some("$ORIGIN/../lib:/opt/payload/lib"),
        );
        let parsed = parse(&bytes).expect("an ELF parses");
        assert_eq!(
            parsed.deps,
            vec!["libfoo.so.1".to_string(), "libc.so.6".to_string()]
        );
        assert_eq!(
            parsed.rpaths,
            vec!["$ORIGIN/../lib".to_string(), "/opt/payload/lib".to_string()]
        );
    }

    #[test]
    fn non_image_bytes_parse_as_none() {
        assert_eq!(parse(b"#!/bin/sh\necho hi\n"), None);
        assert_eq!(parse(b""), None);
    }

    /// A minimal PE32+ image with an import directory naming `imports`
    /// (descriptor order) and a delay-load import directory naming
    /// `delay_imports` (which the parser must NEVER read — delay-load
    /// is out of phase W, spec 22 §2.1). One section (.rdata) maps RVA
    /// 0x1000 to the file offset right after the headers.
    fn pe64_fixture(imports: &[&str], delay_imports: &[&str]) -> Vec<u8> {
        pe64_fixture_deep(0, imports, delay_imports)
    }

    /// The same fixture with `pad` zero bytes between the headers and the
    /// section body: the import directory's FILE offset lands at
    /// 0x200 + pad — a multi-MiB module's .rdata behind a big .text
    /// (incident 13's libsass.so).
    fn pe64_fixture_deep(pad: usize, imports: &[&str], delay_imports: &[&str]) -> Vec<u8> {
        const HEADERS: usize = 0x200;
        const SECTION_RVA: u32 = 0x1000;
        let raw_off = HEADERS + pad;
        let import_dir_size = (imports.len() + 1) * PE_IMPORT_DESCRIPTOR_SIZE;
        // Section body: the import descriptors (+ all-zero terminator),
        // then the import name strings, then the delay-load descriptors
        // (+ terminator) and their name strings.
        let mut section = vec![0u8; import_dir_size];
        let mut import_name_rvas = Vec::new();
        for name in imports {
            import_name_rvas.push(SECTION_RVA + section.len() as u32);
            section.extend_from_slice(name.as_bytes());
            section.push(0);
        }
        let delay_base = section.len();
        let delay_dir_rva = SECTION_RVA + delay_base as u32;
        let mut delay_name_rvas = Vec::new();
        if !delay_imports.is_empty() {
            section.resize(
                section.len() + (delay_imports.len() + 1) * PE_IMPORT_DESCRIPTOR_SIZE,
                0,
            );
            for name in delay_imports {
                delay_name_rvas.push(SECTION_RVA + section.len() as u32);
                section.extend_from_slice(name.as_bytes());
                section.push(0);
            }
        }
        let mut out = vec![0u8; HEADERS];
        out[0..2].copy_from_slice(b"MZ");
        out[0x3C..0x40].copy_from_slice(&0x80_u32.to_le_bytes()); // e_lfanew
        out[0x80..0x84].copy_from_slice(b"PE\0\0");
        let coff = 0x84;
        out[coff..coff + 2].copy_from_slice(&0x8664_u16.to_le_bytes()); // AMD64
        out[coff + 2..coff + 4].copy_from_slice(&1_u16.to_le_bytes()); // sections
        out[coff + 16..coff + 18].copy_from_slice(&240_u16.to_le_bytes()); // opt hdr
        let opt = coff + 20;
        out[opt..opt + 2].copy_from_slice(&PE32PLUS_MAGIC.to_le_bytes());
        out[opt + 60..opt + 64].copy_from_slice(&(HEADERS as u32).to_le_bytes()); // SizeOfHeaders
        out[opt + 108..opt + 112].copy_from_slice(&16_u32.to_le_bytes()); // dirs count
        let dirs = opt + 112;
        // Import directory (index 1).
        out[dirs + 8..dirs + 12].copy_from_slice(&SECTION_RVA.to_le_bytes());
        out[dirs + 12..dirs + 16].copy_from_slice(&(import_dir_size as u32).to_le_bytes());
        // Delay-load import directory (index 13) — present, never read.
        if !delay_imports.is_empty() {
            let d = dirs + 13 * 8;
            out[d..d + 4].copy_from_slice(&delay_dir_rva.to_le_bytes());
            out[d + 4..d + 8].copy_from_slice(
                &(((delay_imports.len() + 1) * PE_IMPORT_DESCRIPTOR_SIZE) as u32).to_le_bytes(),
            );
        }
        // The one section header: .rdata, RVA 0x1000 → file raw_off.
        let sec = opt + 240;
        out[sec..sec + 6].copy_from_slice(b".rdata");
        out[sec + 8..sec + 12].copy_from_slice(&(section.len() as u32).to_le_bytes());
        out[sec + 12..sec + 16].copy_from_slice(&SECTION_RVA.to_le_bytes());
        out[sec + 16..sec + 20].copy_from_slice(&(section.len() as u32).to_le_bytes());
        out[sec + 20..sec + 24].copy_from_slice(&(raw_off as u32).to_le_bytes());
        out.resize(raw_off, 0);
        out.extend_from_slice(&section);
        // Fill the descriptors' name RVAs (thunks need not resolve —
        // the walk reads names only).
        for (i, rva) in import_name_rvas.iter().enumerate() {
            let at = raw_off + i * PE_IMPORT_DESCRIPTOR_SIZE;
            out[at..at + 4].copy_from_slice(&1_u32.to_le_bytes()); // OriginalFirstThunk
            out[at + 12..at + 16].copy_from_slice(&rva.to_le_bytes()); // Name
            out[at + 16..at + 20].copy_from_slice(&1_u32.to_le_bytes()); // FirstThunk
        }
        for (i, rva) in delay_name_rvas.iter().enumerate() {
            let at = raw_off + delay_base + i * PE_IMPORT_DESCRIPTOR_SIZE;
            out[at..at + 4].copy_from_slice(&1_u32.to_le_bytes());
            out[at + 12..at + 16].copy_from_slice(&rva.to_le_bytes());
            out[at + 16..at + 20].copy_from_slice(&1_u32.to_le_bytes());
        }
        out
    }

    #[test]
    fn pe64_import_directory_names_in_descriptor_order() {
        let bytes = pe64_fixture(&["sibling.dll", "vendor2.dll", "KERNEL32.dll"], &[]);
        let parsed = parse(&bytes).expect("a PE parses");
        assert_eq!(parsed.format, ImageFormat::Pe);
        assert_eq!(
            parsed.deps,
            vec![
                "sibling.dll".to_string(),
                "vendor2.dll".to_string(),
                "KERNEL32.dll".to_string()
            ]
        );
        assert!(parsed.rpaths.is_empty(), "no rpath exists on PE");
    }

    #[test]
    fn pe64_without_imports_parses_dependency_free() {
        // Import directory present but holding only the terminator.
        let bytes = pe64_fixture(&[], &[]);
        let parsed = parse(&bytes).expect("a PE parses");
        assert_eq!(parsed.format, ImageFormat::Pe);
        assert!(parsed.deps.is_empty());
        // A declared-but-null import directory (RVA 0).
        let mut nulled = bytes.clone();
        let dirs = 0x84 + 20 + 112;
        nulled[dirs + 8..dirs + 16].fill(0);
        let parsed = parse(&nulled).expect("a PE parses");
        assert_eq!(parsed.format, ImageFormat::Pe);
        assert!(parsed.deps.is_empty());
    }

    #[test]
    fn pe64_delay_load_imports_are_never_read() {
        let bytes = pe64_fixture(&["real.dll"], &["delayed.dll"]);
        let parsed = parse(&bytes).expect("a PE parses");
        assert_eq!(parsed.deps, vec!["real.dll".to_string()]);
    }

    #[test]
    fn pe_import_directory_beyond_the_header_window_needs_the_whole_image() {
        // Incident 13 (the msys libsass 126): a multi-MiB module's
        // import table sits in .rdata past HEADER_WINDOW, and a
        // windowed parse silently answers an empty dep set — the
        // closure walk then materializes the importer ALONE and the OS
        // load misses the vendored siblings. parse() hands PE the whole
        // buffer for exactly this reason; this pins both sides of the
        // hazard.
        let bytes = pe64_fixture_deep(HEADER_WINDOW + 0x400, &["deep.dll"], &[]);
        let parsed = parse(&bytes).expect("the whole image parses");
        assert_eq!(parsed.deps, vec!["deep.dll".to_string()]);
        let windowed = parse(&bytes[..HEADER_WINDOW]).expect("the window parses");
        assert!(
            windowed.deps.is_empty(),
            "the truncated window silently misses the deep import table"
        );
    }

    #[test]
    fn pe_malformed_headers_are_not_images_and_never_panic() {
        // The named answer for a malformed PE is None — no dependencies
        // parsed; the OS loader answers for the file exactly as before
        // (spec 22 §2.1's honest-failure rule). Never a panic.
        let good = pe64_fixture(&["sibling.dll"], &[]);
        // DOS stub only (e_lfanew unreadable).
        assert_eq!(parse(b"MZ"), None);
        // e_lfanew pointing past EOF.
        let mut bad = good.clone();
        bad[0x3C..0x40].copy_from_slice(&0xFFFF_FF00_u32.to_le_bytes());
        assert_eq!(parse(&bad), None);
        // A wrong PE signature.
        let mut bad = good.clone();
        bad[0x80..0x84].copy_from_slice(b"PX\0\0");
        assert_eq!(parse(&bad), None);
        // An unknown optional-header magic.
        let mut bad = good.clone();
        bad[0x98..0x9A].copy_from_slice(&0x9999_u16.to_le_bytes());
        assert_eq!(parse(&bad), None);
        // Optional header truncated away (declared size outruns the bytes).
        let mut bad = good.clone();
        bad.truncate(0x100);
        assert_eq!(parse(&bad), None);
        // Import-directory RVA mapping nowhere (past every section and
        // above SizeOfHeaders).
        let mut bad = good.clone();
        let dirs = 0x84 + 20 + 112;
        bad[dirs + 8..dirs + 12].copy_from_slice(&0x00F0_0000_u32.to_le_bytes());
        assert_eq!(parse(&bad), None);
        // Section table truncated mid-entry.
        let mut bad = good;
        bad.truncate(0x84 + 20 + 240 + 10);
        assert_eq!(parse(&bad), None);
        // A partial window (cut mid-descriptor) still parses, simply
        // dependency-free — the truncated-header rule, never a panic.
        let mut short = pe64_fixture(&["sibling.dll"], &[]);
        short.truncate(0x210);
        let parsed = parse(&short).expect("a partial window still parses");
        assert_eq!(parsed.format, ImageFormat::Pe);
        assert!(parsed.deps.is_empty());
    }

    #[test]
    fn pe_descriptor_with_an_unmappable_name_rva_is_skipped() {
        let mut bytes = pe64_fixture(&["sibling.dll", "vendor2.dll"], &[]);
        // Corrupt the FIRST descriptor's name RVA only; the second
        // descriptor is still read.
        bytes[0x200 + 12..0x200 + 16].copy_from_slice(&0x00F0_0000_u32.to_le_bytes());
        let parsed = parse(&bytes).expect("a PE parses");
        assert_eq!(parsed.deps, vec!["vendor2.dll".to_string()]);
    }
}
