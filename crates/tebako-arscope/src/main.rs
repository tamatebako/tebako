//! tebako-arscope — scope Rust staticlibs for single-link embedding.
//!
//! A Rust `staticlib` exports every symbol globally. That is correct for
//! distribution, but it makes the archive unlinkable next to a SECOND
//! Rust runtime in the same final link: ruby's YJIT (`libyjit.o`)
//! carries its own rustc std, and two rustc stds collide on
//! `rust_eh_personality`, the compiler-rt family, and every mangled std
//! name.
//!
//! The seal this tool applies — per archive member, in-process, with no
//! compiler driver, no linker, and no shell-outs: every DEFINED symbol
//! whose name does not start with the keep prefix (`tebako_`) is
//! RENAMED with the internal prefix (`__tebako_internal_`), definitions
//! and references alike (relocations are remapped to the renamed
//! symbols). Our own references keep resolving to our own copies; the
//! second Rust runtime keeps the original names; a collision becomes
//! impossible by construction.
//!
//! Rename, never hide: hiding per-object breaks intra-archive
//! references; renaming preserves them.
//!
//! ```text
//! tebako-arscope <in.a> <out.a> [--keep-prefix tebako_] [--prefix __tebako_internal_]
//! ```

use std::process::ExitCode;

mod macho;

use object::{
    File, Object as _, ObjectSection as _, ObjectSymbol as _, RelocationTarget, SymbolScope,
};

/// The default keep prefix: the whole public surface (exports.txt is
/// the CI-gated form of the same rule).
const KEEP_PREFIX: &str = "tebako_";
/// The default internal prefix for scoped symbols.
const SCOPE_PREFIX: &str = "__tebako_internal_";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut keep = KEEP_PREFIX.to_string();
    let mut prefix = SCOPE_PREFIX.to_string();
    let mut paths = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--keep-prefix" => match args.next() {
                Some(v) => keep = v,
                None => return usage("--keep-prefix needs a value"),
            },
            "--prefix" => match args.next() {
                Some(v) => prefix = v,
                None => return usage("--prefix needs a value"),
            },
            _ => paths.push(arg),
        }
    }
    if paths.len() != 2 {
        return usage("expected: tebako-arscope <in.a> <out.a> [--keep-prefix P] [--prefix P]");
    }
    match run(&paths[0], &paths[1], &keep, &prefix) {
        Ok(report) => {
            println!(
                "arscope: {} -> {} ({} member(s), {} symbol(s) scoped, {} kept public)",
                paths[0], paths[1], report.members, report.scoped, report.kept
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("tebako-arscope: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("tebako-arscope: {msg}");
    ExitCode::from(64)
}

#[derive(Default)]
struct Report {
    members: usize,
    scoped: usize,
    kept: usize,
}

fn run(input: &str, output: &str, keep: &str, prefix: &str) -> Result<Report, String> {
    let bytes = std::fs::read(input).map_err(|e| format!("cannot read {input}: {e}"))?;
    let archive = object::read::archive::ArchiveFile::parse(&bytes[..])
        .map_err(|e| format!("cannot parse {input} as an archive: {e}"))?;

    // Pass A: collect every symbol this tool will RENAME anywhere in
    // the archive (the rename set for references — a renamed definition
    // must be matched by its references or the link breaks). Mach-O
    // members use the raw nlist scan so this pass and the rewrite share
    // one rule; other formats use the object crate.
    let mut defined: std::collections::HashSet<String> = std::collections::HashSet::new();
    for member in archive.members() {
        let member = member.map_err(|e| format!("cannot read a member of {input}: {e}"))?;
        let data = member
            .data(&bytes[..])
            .map_err(|e| format!("cannot read a member of {input}: {e}"))?;
        match macho::defined(data, keep) {
            Ok(names) => defined.extend(names),
            Err(_) => {
                if let Ok(obj) = File::parse(data) {
                    for symbol in obj.symbols() {
                        let external = symbol.scope() != SymbolScope::Unknown || symbol.is_weak();
                        if !symbol.is_undefined() && external {
                            if let Ok(name) = symbol.name() {
                                defined.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Pass B: rewrite every member, collecting its exported names
    // (post-rename) for the archive symbol index.
    let mut report = Report::default();
    let mut members: Vec<(String, Vec<u8>, Vec<String>)> = Vec::new();
    for member in archive.members() {
        let member = member.map_err(|e| format!("cannot read a member of {input}: {e}"))?;
        let name = String::from_utf8_lossy(member.name()).into_owned();
        let data = member
            .data(&bytes[..])
            .map_err(|e| format!("cannot read member {name} of {input}: {e}"))?;
        let (mut rewritten, exported) = scope_object(data, keep, prefix, &defined, &mut report)
            .map_err(|reason| format!("member {name}: {reason}"))?;
        let _ = &exported;
        // ld64 does not follow the 2-byte ar member alignment: its
        // archive walk advances to the next member at
        // align4(header + size). A member whose size is not a multiple
        // of 4 desyncs the walk ("archive member invalid control bits")
        // or pushes the computed extent past EOF ("malformed archive,
        // member exceeds file size"). Pad every member's content to a
        // multiple of 4 — trailing zeros are harmless slack to a
        // Mach-O object (all load-command offsets are unaffected).
        let pad4 = (4 - rewritten.len() % 4) % 4;
        rewritten.extend_from_slice(&[0; 4][..pad4]);
        report.members += 1;
        members.push((name, rewritten, exported));
    }

    // The archive symbol index: ld consumes archives THROUGH it (a
    // missing index is "no table of contents"). The index FORM follows
    // the OBJECT format inside: Mach-O archives get the BSD __.SYMDEF
    // (the cargo/llvm-ranlib form — see the note above), everything
    // else the GNU "/" index. (The input's own index member, when it
    // has one, is the same discriminator in practice; a hand-made
    // Mach-O fixture without one still gets the BSD form.)
    let bsd = {
        let mut is_macho = false;
        for member in archive.members() {
            let member = member.map_err(|e| format!("cannot read a member of {input}: {e}"))?;
            let data = member
                .data(&bytes[..])
                .map_err(|e| format!("cannot read a member of {input}: {e}"))?;
            if macho::defined(data, keep).is_ok() {
                is_macho = true;
                break;
            }
        }
        is_macho
    };
    let index = build_index(&members, bsd);
    let index_name = if bsd { "__.SYMDEF" } else { "/" };

    let mut out =
        std::fs::File::create(output).map_err(|e| format!("cannot create {output}: {e}"))?;
    use std::io::Write as _;
    out.write_all(b"!<arch>\n")
        .map_err(|e| format!("cannot write the archive magic: {e}"))?;
    write_member(&mut out, index_name, &index, bsd)?;
    for (name, data, _) in &members {
        write_member(&mut out, name, data, bsd)?;
    }
    Ok(report)
}

/// The BSD "#1/N" inline-name length for a member name on Darwin.
/// Every member uses the "#1/N" form there — the plain 16-byte name
/// field has no terminator, so a name of exactly 16 bytes lets the
/// date field bleed into the name (ld64 then reads e.g.
/// "stream-ctx.cpp.o0" and a garbage member size). N is the smallest
/// value strictly greater than the name length with N ≡ 4 (mod 8),
/// floored at 12 — the observed modern ar/llvm-ar convention:
/// a.o→#1/12, __.SYMDEF→#1/12, __.SYMDEF SORTED→#1/20, 16→20,
/// 36→44, 74→76. The floor matters: ld64 rejects an archive whose
/// member uses "#1/4" with "archive member invalid control bits".
fn bsd_name_pad(name: &str) -> usize {
    let mut n = (name.len() + 1).max(12);
    while n % 8 != 4 {
        n += 1;
    }
    n
}

/// Write one archive member: the 60-byte header (BSD "#1/<padded-name>"
/// inline form — on Darwin for EVERY name, elsewhere only for long
/// names — space-padded plain name otherwise), the name bytes, the
/// content, and the even-alignment pad. The index member rides the
/// same writer (its name is always "__.SYMDEF", padded to 20 — the
/// ranlib form ld expects).
fn write_member(
    out: &mut impl std::io::Write,
    name: &str,
    data: &[u8],
    bsd: bool,
) -> Result<(), String> {
    let long_form = if bsd {
        true
    } else {
        name.len() > 16 || name.contains(' ')
    };
    let padded_len = if long_form {
        if bsd {
            bsd_name_pad(name)
        } else {
            name.len() + ((4 - name.len() % 4) % 4)
        }
    } else {
        0
    };
    let size = data.len() + padded_len;
    let mut header = Vec::with_capacity(60);
    if long_form {
        header.extend_from_slice(format!("#1/{:<13}", padded_len).as_bytes());
    } else {
        header.extend_from_slice(format!("{:<16}", name).as_bytes());
    }
    header.extend_from_slice(
        format!("{:<12}{:<6}{:<6}{:<8o}{:<10}`\n", 0, 0, 0, 0o644, size).as_bytes(),
    );
    debug_assert_eq!(header.len(), 60);
    out.write_all(&header)
        .map_err(|e| format!("cannot write a member header for {name}: {e}"))?;
    if long_form {
        out.write_all(name.as_bytes())
            .map_err(|e| format!("cannot write the name of {name}: {e}"))?;
        out.write_all(&vec![0; padded_len - name.len()])
            .map_err(|e| format!("cannot pad the name of {name}: {e}"))?;
    }
    out.write_all(data)
        .map_err(|e| format!("cannot write member {name}: {e}"))?;
    if size % 2 != 0 {
        out.write_all(b"\n")
            .map_err(|e| format!("cannot pad member {name}: {e}"))?;
    }
    Ok(())
}

/// The member size of the header + name (BSD "#1/<padded>" inline
/// form — on Darwin for every name, elsewhere only for long names).
/// The on-disk size of one member (header + inline name bytes). MUST
/// stay in lockstep with write_member: the index offsets are computed
/// from these sizes. The BSD form pads names per bsd_name_pad; the GNU
/// long form pads the 60-byte header's inline name to a 4-byte
/// multiple, short names ride the 60-byte header alone.
fn member_header_size(name: &str, bsd: bool) -> usize {
    if bsd {
        return 60 + bsd_name_pad(name);
    }
    let len = name.len();
    if len > 16 || name.contains(' ') {
        60 + len + ((4 - len % 4) % 4)
    } else {
        60
    }
}

/// Build the archive symbol index over the exported names. BSD
/// __.SYMDEF on Darwin (u32 ranlib-bytes, n × {strx, off}, u32
/// strsize, strings); the GNU "/" form elsewhere (u32be count, n ×
/// u32be off, nul-terminated names).
fn build_index(members: &[(String, Vec<u8>, Vec<String>)], bsd: bool) -> Vec<u8> {
    let index_name = if bsd { "__.SYMDEF" } else { "/" };

    // Index fields are fixed-width — the size is known directly.
    let index_len = index_size(members, index_name, 0, bsd);

    let index_member_size = member_header_size(index_name, bsd) + index_len;
    let mut member_offsets = Vec::with_capacity(members.len());
    let mut offset = 8 + index_member_size + (index_member_size % 2);
    for (name, data, _) in members {
        member_offsets.push(offset);
        let member_size = member_header_size(name, bsd) + data.len();
        offset += member_size + (member_size % 2);
    }

    let mut out = Vec::with_capacity(index_len);
    if bsd {
        // __.SYMDEF in member order (nlist order per member): ld64
        // walks the TOC validating that member offsets are
        // non-decreasing — a name-sorted TOC fails its parse with
        // "invalid control bits" (the ranlib order is the proof).
        let count: usize = members.iter().map(|(_, _, e)| e.len()).sum();
        out.extend_from_slice(&((count * 8) as u32).to_le_bytes());
        let mut strx = 0u32;
        for ((_, _, exported), header_offset) in members.iter().zip(&member_offsets) {
            for name in exported {
                out.extend_from_slice(&strx.to_le_bytes());
                out.extend_from_slice(&(*header_offset as u32).to_le_bytes());
                strx += name.len() as u32 + 1;
            }
        }
        // The string table is padded with NULs to an 8-byte multiple
        // and strsize counts the padding (the ranlib convention;
        // cargo's archives carry the same alignment as member slack).
        // An unpadded table leaves the SYMDEF member size odd, and
        // ld64 rejects the whole archive with "archive member invalid
        // control bits".
        let str_pad = ((8 - strx % 8) % 8) as usize;
        out.extend_from_slice(&(strx + str_pad as u32).to_le_bytes());
        for (_, _, exported) in members {
            for name in exported {
                out.extend_from_slice(name.as_bytes());
                out.push(0);
            }
        }
        out.extend_from_slice(&vec![0; str_pad]);
    } else {
        let count: usize = members.iter().map(|(_, _, e)| e.len()).sum();
        out.extend_from_slice(&(count as u32).to_be_bytes());
        for ((_, _, exported), header_offset) in members.iter().zip(&member_offsets) {
            for _ in exported {
                out.extend_from_slice(&(*header_offset as u32).to_be_bytes());
            }
        }
        for (_, _, exported) in members {
            for name in exported {
                out.extend_from_slice(name.as_bytes());
                out.push(0);
            }
        }
    }
    out
}

/// The index size for a provisional value of itself.
fn index_size(
    members: &[(String, Vec<u8>, Vec<String>)],
    index_name: &str,
    index_len: usize,
    bsd: bool,
) -> usize {
    let _ = (index_name, index_len);
    let count: usize = members.iter().map(|(_, _, e)| e.len()).sum();
    let strings: usize = members
        .iter()
        .flat_map(|(_, _, e)| e.iter())
        .map(|n| n.len() + 1)
        .sum();
    if bsd {
        // String table padded to an 8-byte multiple (see build_index).
        let strings = strings + (8 - strings % 8) % 8;
        4 + count * 8 + 4 + strings
    } else {
        4 + count * 4 + strings
    }
}

/// True when the symbol's logical name is under the keep prefix and
/// therefore stays public (used by the generic non-Mach-O path; the
/// Mach-O path shares the rule through macho::renames_def).
fn keeps_name(name: &str, keep: &str) -> bool {
    name.trim_start_matches('_').starts_with(keep)
}

/// The symbol's name without the format's mangling prefix (Mach-O
/// prepends '_' to every external name; ELF names are verbatim).
fn logical_name(name: &str, format: object::BinaryFormat) -> &str {
    if format == object::BinaryFormat::MachO {
        name.strip_prefix('_').unwrap_or(name)
    } else {
        name
    }
}

/// Rewrite one object member: sections and symbols copied, defined
/// non-keep symbols renamed, relocations remapped to the renamed ids.
fn scope_object(
    data: &[u8],
    keep: &str,
    prefix: &str,
    defined: &std::collections::HashSet<String>,
    report: &mut Report,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let obj = File::parse(data).map_err(|e| format!("not an object file: {e}"))?;
    // Mach-O: raw LC_SYMTAB surgery (sections and relocations stay
    // byte-identical — the general rewrite breaks ld64's atomizers).
    if obj.format() == object::BinaryFormat::MachO {
        let (out, exported, renamed, kept) = macho::scope(data, keep, prefix, defined)?;
        report.scoped += renamed;
        report.kept += kept;
        return Ok((out, exported));
    }
    let mut out = object::write::Object::new(obj.format(), obj.architecture(), obj.endianness());
    // The archive-index names: every defined, externally visible symbol.
    let mut exported: Vec<String> = Vec::new();

    let mut section_ids = std::collections::HashMap::new();
    for section in obj.sections() {
        let segment = section
            .segment_name()
            .map_err(|e| format!("section segment name: {e}"))?
            .unwrap_or("")
            .as_bytes()
            .to_vec();
        let name = section
            .name_bytes()
            .map_err(|e| format!("section name: {e}"))?
            .to_vec();
        let id = out.add_section(segment, name, section.kind());
        let new_section = out.section_mut(id);
        // COMDAT membership does not survive the rewrite (the object
        // crate emits no SHT_GROUP section): a member section keeping
        // SHF_GROUP without its group is an inconsistent ELF, and GNU
        // ld rejects the whole archive ("no group info for section
        // '.data.DW.ref.rust_eh_personality'", then 'file format not
        // recognized' — binutils 2.34, the ubuntu:20.04 floor; ld64 and
        // the mingw link tolerate it, which is why only the ELF path
        // broke). Clear the membership flag — the weak symbols inside
        // still merge by name at link time.
        new_section.flags = match section.flags() {
            object::SectionFlags::Elf { sh_flags } => object::SectionFlags::Elf {
                sh_flags: sh_flags & !0x200,
            },
            other => other,
        };
        if section.kind().is_bss() {
            // BSS carries no bytes but keeps its size and alignment.
            new_section.append_bss(section.size(), section.align());
        } else {
            new_section.set_data(
                section.data().map_err(|e| format!("section data: {e}"))?,
                section.align(),
            );
        }
        section_ids.insert(section.index(), id);
    }

    let mut symbol_ids = std::collections::HashMap::new();
    for symbol in obj.symbols() {
        // COFF section symbols are per-section bookkeeping the writer
        // regenerates (the object crate's COFF emit rejects re-adding
        // them). Relocations reference them by index — remap those to
        // the writer's own section symbol for the same section.
        if symbol.kind() == object::SymbolKind::Section {
            if let object::SymbolSection::Section(index) = symbol.section() {
                let id = out.section_symbol(section_ids[&index]);
                symbol_ids.insert(symbol.index(), id);
            }
            continue;
        }
        let name = symbol.name().map_err(|e| format!("symbol name: {e}"))?;
        let logical = logical_name(name, obj.format());
        // Rename everything EXTERNALLY VISIBLE (weak or Linkage/
        // Dynamic): true locals (Unknown/Compilation scope) are
        // invisible already; undefined references are not definitions.
        // The write side re-adds the format's mangling prefix, so names
        // are decided UNPREFIXED here.
        let visible = symbol.is_weak()
            || matches!(symbol.scope(), SymbolScope::Linkage | SymbolScope::Dynamic);
        let rename = if symbol.is_undefined() {
            // A renamed definition is worthless to a reference that
            // kept the old name: refs to archive-defined symbols ride
            // the prefix; refs to the outside world stay.
            defined.contains(name)
        } else {
            visible && !keeps_name(logical, keep)
        };
        let renamed = if !rename {
            if !symbol.is_undefined() && visible && keeps_name(logical, keep) {
                report.kept += 1;
                exported.push(name.to_string());
            }
            logical.to_string()
        } else {
            report.scoped += 1;
            let n = format!("{prefix}{logical}");
            if !symbol.is_undefined() {
                exported.push(n.clone());
            }
            n
        };
        let section = match symbol.section() {
            object::SymbolSection::Section(index) => {
                object::write::SymbolSection::Section(section_ids[&index])
            }
            object::SymbolSection::Undefined => object::write::SymbolSection::Undefined,
            object::SymbolSection::Absolute => object::write::SymbolSection::Absolute,
            object::SymbolSection::Common => object::write::SymbolSection::Common,
            _ => object::write::SymbolSection::Undefined,
        };
        let flags = map_symbol_flags(symbol.flags(), &section_ids, &symbol_ids)?;
        let id = out.add_symbol(object::write::Symbol {
            name: renamed.into_bytes(),
            value: symbol.address(),
            size: symbol.size(),
            kind: symbol.kind(),
            scope: symbol.scope(),
            weak: symbol.is_weak(),
            section,
            flags,
        });
        symbol_ids.insert(symbol.index(), id);
    }

    for section in obj.sections() {
        for (offset, relocation) in section.relocations() {
            let symbol = match relocation.target() {
                RelocationTarget::Symbol(index) => symbol_ids[&index],
                RelocationTarget::Section(index) => out.section_symbol(section_ids[&index]),
                other => {
                    return Err(format!(
                        "an unsupported relocation target ({other:?}, offset {offset:#x}, kind {:?}) — implement it when the ELF leg needs it",
                        relocation.kind()
                    ));
                }
            };
            out.add_relocation(
                section_ids[&section.index()],
                object::write::Relocation {
                    offset,
                    symbol,
                    addend: relocation.addend(),
                    flags: relocation.flags(),
                },
            )
            .map_err(|e| format!("relocation: {e}"))?;
        }
    }

    let bytes = out
        .write()
        .map_err(|e| format!("cannot emit the rewritten object: {e}"))?;
    Ok((bytes, exported))
}

/// Symbol flags carry ids in the COFF/XCOFF group forms; remap the
/// section id where one rides along.
fn map_symbol_flags(
    flags: object::SymbolFlags<object::SectionIndex, object::SymbolIndex>,
    section_ids: &std::collections::HashMap<object::SectionIndex, object::write::SectionId>,
    symbol_ids: &std::collections::HashMap<object::SymbolIndex, object::write::SymbolId>,
) -> Result<object::SymbolFlags<object::write::SectionId, object::write::SymbolId>, String> {
    Ok(match flags {
        object::SymbolFlags::None => object::SymbolFlags::None,
        object::SymbolFlags::Elf { st_info, st_other } => {
            object::SymbolFlags::Elf { st_info, st_other }
        }
        object::SymbolFlags::MachO { n_desc } => object::SymbolFlags::MachO { n_desc },
        object::SymbolFlags::CoffSection {
            selection,
            associative_section,
        } => object::SymbolFlags::CoffSection {
            selection,
            associative_section: associative_section.map(|index| section_ids[&index]),
        },
        object::SymbolFlags::Xcoff {
            n_sclass,
            x_smtyp,
            x_smclas,
            containing_csect,
        } => object::SymbolFlags::Xcoff {
            n_sclass,
            x_smtyp,
            x_smclas,
            containing_csect: containing_csect.map(|index| symbol_ids[&index]),
        },
        // Unknown future flag forms lose their flags rather than their
        // symbols — the rewrite is about names, not flag trivia.
        _ => object::SymbolFlags::None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tiny object with one public tebako_* symbol, one internal
    /// global, one local, and one undefined reference.
    fn fixture_object() -> Vec<u8> {
        let format = if cfg!(target_os = "macos") {
            object::BinaryFormat::MachO
        } else {
            object::BinaryFormat::Elf
        };
        let arch = if cfg!(target_arch = "aarch64") {
            object::Architecture::Aarch64
        } else {
            object::Architecture::X86_64
        };
        let endian = if cfg!(target_endian = "little") {
            object::Endianness::Little
        } else {
            object::Endianness::Big
        };
        let mut out = object::write::Object::new(format, arch, endian);
        let text = out.add_section(
            b"__TEXT".to_vec(),
            b"__text".to_vec(),
            object::SectionKind::Text,
        );
        out.section_mut(text).set_data(b"\x90\x90\xc3", 1);
        out.add_symbol(object::write::Symbol {
            name: b"tebako_probe".to_vec(),
            value: 0,
            size: 1,
            kind: object::SymbolKind::Text,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(text),
            flags: object::SymbolFlags::None,
        });
        out.add_symbol(object::write::Symbol {
            name: b"rust_eh_personality".to_vec(),
            value: 1,
            size: 1,
            kind: object::SymbolKind::Text,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(text),
            flags: object::SymbolFlags::None,
        });
        out.add_symbol(object::write::Symbol {
            name: b"Lhelper".to_vec(),
            value: 2,
            size: 1,
            kind: object::SymbolKind::Text,
            scope: object::SymbolScope::Compilation,
            weak: false,
            section: object::write::SymbolSection::Section(text),
            flags: object::SymbolFlags::None,
        });
        out.add_symbol(object::write::Symbol {
            name: b"malloc".to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::Unknown,
            scope: object::SymbolScope::Unknown,
            weak: false,
            section: object::write::SymbolSection::Undefined,
            flags: object::SymbolFlags::None,
        });
        out.write().expect("fixture object")
    }

    #[test]
    fn defined_non_keep_symbols_are_renamed_and_undefined_refs_are_not() {
        let mut report = Report::default();
        let (bytes, _exported) = scope_object(
            &fixture_object(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            &std::collections::HashSet::new(),
            &mut report,
        )
        .expect("scope the fixture");
        let obj = File::parse(&bytes[..]).expect("parse the rewritten object");
        let names: Vec<String> = obj
            .symbols()
            .filter_map(|s: object::Symbol<'_, '_>| s.name().ok().map(str::to_string))
            .collect();
        let has = |want: &str| {
            names
                .iter()
                .any(|n: &String| n.trim_start_matches('_') == want.trim_start_matches('_'))
        };
        assert!(
            has("tebako_probe"),
            "the public surface survives: {names:?}"
        );
        assert!(
            has("__tebako_internal_rust_eh_personality"),
            "the internal symbol is renamed: {names:?}"
        );
        assert!(
            has("malloc"),
            "an undefined reference is never renamed: {names:?}"
        );
        assert!(
            has("Lhelper"),
            "a local symbol keeps its name (already invisible): {names:?}"
        );
        assert_eq!(report.kept, 1);
        assert_eq!(report.scoped, 1);
    }

    /// A fixture object with one defined global and two undefined
    /// references: one to the sibling member's definition, one to libc.
    #[cfg(target_os = "macos")]
    fn fixture_object_with_ref() -> Vec<u8> {
        let mut out = object::write::Object::new(
            object::BinaryFormat::MachO,
            object::Architecture::Aarch64,
            object::Endianness::Little,
        );
        let text = out.add_section(
            b"__TEXT".to_vec(),
            b"__text".to_vec(),
            object::SectionKind::Text,
        );
        out.section_mut(text).set_data(b"\x90\x90\xc3", 1);
        out.add_symbol(object::write::Symbol {
            name: b"consumer_fn".to_vec(),
            value: 0,
            size: 3,
            kind: object::SymbolKind::Text,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(text),
            flags: object::SymbolFlags::None,
        });
        out.add_symbol(object::write::Symbol {
            // Real Mach-O externals carry the leading underscore; the
            // object-crate writer adds it for definitions but writes
            // undefined names verbatim, so the ref spells it out.
            name: b"_rust_eh_personality".to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::Unknown,
            scope: object::SymbolScope::Unknown,
            weak: false,
            section: object::write::SymbolSection::Undefined,
            flags: object::SymbolFlags::None,
        });
        out.add_symbol(object::write::Symbol {
            name: b"malloc".to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::Unknown,
            scope: object::SymbolScope::Unknown,
            weak: false,
            section: object::write::SymbolSection::Undefined,
            flags: object::SymbolFlags::None,
        });
        out.write().expect("fixture object")
    }

    /// A renamed definition must be matched by its references: an
    /// undefined ref whose name is in the pass-A rename set rides the
    /// prefix; refs to the outside world (libc) never do.
    #[test]
    #[cfg(target_os = "macos")]
    fn refs_to_renamed_definitions_ride_the_prefix() {
        let def_member = fixture_object();
        let ref_member = fixture_object_with_ref();
        let mut defined = macho::defined(&def_member, KEEP_PREFIX).expect("pass A over the def");
        defined.extend(macho::defined(&ref_member, KEEP_PREFIX).expect("pass A over the ref"));
        let (out, _exported, renamed, _kept) =
            macho::scope(&ref_member, KEEP_PREFIX, SCOPE_PREFIX, &defined).expect("scope the ref");
        let obj = File::parse(&out[..]).expect("parse the rewritten ref member");
        let names: Vec<String> = obj
            .symbols()
            .filter_map(|s: object::Symbol<'_, '_>| s.name().ok().map(str::to_string))
            .collect();
        let has = |want: &str| {
            names
                .iter()
                .any(|n: &String| n.trim_start_matches('_') == want.trim_start_matches('_'))
        };
        assert!(
            has("__tebako_internal_rust_eh_personality"),
            "the ref to the renamed def rides the prefix: {names:?}"
        );
        assert!(has("malloc"), "the libc ref stays: {names:?}");
        assert!(
            has("__tebako_internal_consumer_fn"),
            "the member's own def is renamed: {names:?}"
        );
        assert_eq!(renamed, 2, "one def + one ref renamed");
    }

    /// The archive layout ld64 demands, pinned byte-for-byte: BSD "#1/N"
    /// member names with N ≡ 4 (mod 8) floored at 12, every member size
    /// a multiple of 4 (ld64 walks members at 4-byte-aligned extents),
    /// and a __.SYMDEF whose string table is 8-padded with offsets that
    /// land on the real member headers and chain exactly to EOF.
    #[test]
    #[cfg(target_os = "macos")]
    fn archive_layout_survives_ld64_walk() {
        let tmp = std::env::temp_dir().join(format!("arscope-layout-{}.a", std::process::id()));
        let tmp_out =
            std::env::temp_dir().join(format!("arscope-layout-out-{}.a", std::process::id()));
        // Two members with awkward name/content sizes: a 3-char name
        // (the "#1/4" trap) and a 16-char name (the unterminated plain
        // field trap), one with an odd-sized content.
        let mut input = b"!<arch>\n".to_vec();
        let mut bytes = fixture_object();
        bytes.push(0); // odd content size
        write_member(&mut input, "a.o", &bytes, true).expect("member a.o");
        let bytes2 = fixture_object_with_ref();
        write_member(&mut input, "sixteen_chars_.o", &bytes2, true).expect("member sixteen");
        std::fs::write(&tmp, &input).expect("write the input archive");

        run(
            tmp.to_str().unwrap(),
            tmp_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
        )
        .expect("scope the archive");
        let out = std::fs::read(&tmp_out).expect("read the scoped archive");
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp_out);

        assert_eq!(&out[..8], b"!<arch>\n");
        let mut at = 8usize;
        let mut member_starts = Vec::new();
        while at < out.len() {
            let header = &out[at..at + 60];
            assert_eq!(&header[58..60], b"`\n", "member at {at}: bad header magic");
            let size: usize = std::str::from_utf8(&header[48..58])
                .unwrap()
                .trim()
                .parse()
                .expect("member size is decimal");
            assert_eq!(
                size % 4,
                0,
                "member at {at}: size {size} not a multiple of 4"
            );
            let name_field = std::str::from_utf8(&header[..16]).unwrap();
            assert!(
                name_field.starts_with("#1/"),
                "member at {at}: BSD inline-name form expected, got {name_field:?}"
            );
            let nlen: usize = name_field[3..].trim().parse().expect("#1/N parses");
            assert!(
                nlen >= 12,
                "member at {at}: inline name padded below the floor"
            );
            assert_eq!(
                nlen % 8,
                4,
                "member at {at}: inline name pad {nlen} not ≡ 4 (mod 8)"
            );
            member_starts.push(at);
            at += 60 + size;
        }
        assert_eq!(at, out.len(), "the member walk lands exactly on EOF");
        assert_eq!(member_starts.len(), 3, "SYMDEF + two objects");

        // The SYMDEF: entries point at the real member headers, the
        // string table is 8-padded.
        let symdef = &out[8..];
        let nlen: usize = std::str::from_utf8(&symdef[..3 + 13]).unwrap()[3..]
            .trim()
            .parse()
            .unwrap();
        let content_at = 8 + 60 + nlen;
        let ransize =
            u32::from_le_bytes(out[content_at..content_at + 4].try_into().unwrap()) as usize;
        let count = ransize / 8;
        // member 1 exports tebako_probe (kept) + the renamed def;
        // member 2 exports its renamed def. Renamed refs do not land
        // in the TOC.
        assert_eq!(count, 3, "renamed defs land in the TOC");
        for i in 0..count {
            let e = content_at + 4 + i * 8;
            let off = u32::from_le_bytes(out[e + 4..e + 8].try_into().unwrap()) as usize;
            assert!(
                member_starts.contains(&off),
                "TOC entry {i} offset {off} is not a member start {member_starts:?}"
            );
        }
        let strsize_at = content_at + 4 + ransize;
        let strsize =
            u32::from_le_bytes(out[strsize_at..strsize_at + 4].try_into().unwrap()) as usize;
        assert_eq!(strsize % 8, 0, "the SYMDEF string table is 8-padded");
    }
}
