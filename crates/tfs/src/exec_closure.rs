//! Executable dependency-closure parsing for `dlmap2file`.
//!
//! A materialized executable or shared library is loaded by the
//! platform loader (dyld, ld.so), whose path probes are RAW SYSCALLS —
//! the preload shim cannot interpose them (proven on macOS 15: dyld's
//! rpath probes never reach the interposed `open`). The only way to
//! satisfy them is to materialize the dependency closure EAGERLY into
//! the dlmap layout, which mirrors the memfs tree exactly, so the
//! loader's executable-relative candidates hit real host files.
//!
//! This module is the pure parser half: bytes → (dependency names,
//! rpaths). Resolution of those names against the mounts (rpath
//! expansion, `@executable_path` / `@loader_path` / `$ORIGIN`,
//! recursion with a visited set) lives in `context.rs`.
//!
//! Only the header region is parsed (the first [`HEADER_WINDOW`] bytes
//! of the extracted copy): Mach-O load commands and the ELF dynamic
//! string table both live in the first pages of any real image. A
//! truncated or unparseable header yields no dependencies — the loader
//! then answers for the host libraries exactly as before.

/// Header bytes examined for dependency metadata.
pub(crate) const HEADER_WINDOW: usize = 1 << 20;

/// A parsed image's dependency names and its own rpath/runpath list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImageDeps {
    /// LC_LOAD_DYLIB / DT_NEEDED names, verbatim.
    pub deps: Vec<String>,
    /// LC_RPATH / DT_RPATH / DT_RUNPATH entries, verbatim.
    pub rpaths: Vec<String>,
}

/// Parse a Mach-O (thin 32/64 or fat) or ELF (32/64, either endianness)
/// header. None when the bytes are neither format.
pub fn parse(bytes: &[u8]) -> Option<ImageDeps> {
    let window = &bytes[..bytes.len().min(HEADER_WINDOW)];
    parse_macho(window).or_else(|| parse_elf(window))
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
    let mut out = ImageDeps::default();
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
                u32e(bytes.get(base..base + 4)?)? as u32 as i32 as i64,
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

    let mut out = ImageDeps::default();
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
// byte helpers
// ---------------------------------------------------------------------

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
}
