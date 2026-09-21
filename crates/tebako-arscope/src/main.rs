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
//! COFF import-library members (the dlltool long form: `.idata$N`
//! sections) are link-editor directives, not code, and GNU ld's PE
//! scripts collect each DLL's chunks via SORT_BY_NAME on the
//! `archive(member)` string — a DLL's lookup run terminates correctly
//! only when its members sort h < sNNNNN < t. rustc's staticlib bundler
//! disambiguates duplicate member names with a numeric "NNNNN_" prefix,
//! which then sorts by bundle position instead: the null terminator (t)
//! lands ahead of the entries (s), and the descriptor's lookup run
//! swallows the NEXT DLL's functions (the v2.8.4 windows runtime defect:
//! miniruby imported WaitOnAddress from ncrypt.dll, the loader answered
//! STATUS_ENTRYPOINT_NOT_FOUND 0xC0000139, the shell reported exit 127).
//! The rewrite therefore restores the canonical member name (strips the
//! numeric prefix) and drops byte-identical duplicates; a same-name
//! member with different bytes is a second import set for the DLL and
//! is kept (ld pulls members by symbol through the index, never by
//! name). Symbol scoping itself applies to import members exactly as to
//! code members — the prefix is consistent within the archive and the
//! factory link proven against it (prefixed symbols + canonical names
//! produce a clean import table in every member/pull order). rustc's
//! gnullvm raw-dylib sets add two shapes of their own: the descriptor
//! references the thunk chunks through UNDEFINED section symbols
//! (carried — see coff_mark_undefined_section_symbols), and one
//! short-form IMPORT_OBJECT_HEADER member per function rides along
//! byte-identical with its in-band names indexed.
//!
//! Two ELF-only repairs ride the same pass (tebako#413): defined
//! STB_GNU_UNIQUE symbols demote to STB_WEAK — the rewrite drops the
//! SHT_GROUP the binding folds through, and binutils < 2.35 reads a
//! group-less UNIQUE as a strong duplicate — and FILE bookkeeping
//! symbols stay in the local symtab region the writer's layout expects.
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
/// ELF st_info bindings (the object crate has no named constants for
/// them): glibc-target g++'s vague-linkage binding and its demotion
/// target (see the STB_GNU_UNIQUE neutralization in scope_object).
const STB_GNU_UNIQUE: u8 = 10;
const STB_WEAK: u8 = 2;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut keep = KEEP_PREFIX.to_string();
    let mut prefix = SCOPE_PREFIX.to_string();
    let mut dedupe_against = None;
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
            "--dedupe-against" => match args.next() {
                Some(v) => dedupe_against = Some(v),
                None => return usage("--dedupe-against needs a value"),
            },
            _ => paths.push(arg),
        }
    }
    if paths.len() != 2 {
        return usage("expected: tebako-arscope <in.a> <out.a> [--keep-prefix P] [--prefix P] [--dedupe-against base.a]");
    }
    match run(
        &paths[0],
        &paths[1],
        &keep,
        &prefix,
        dedupe_against.as_deref(),
    ) {
        Ok(report) => {
            println!(
                "arscope: {} -> {} ({} member(s), {} symbol(s) scoped, {} kept public, {} import member(s), {} cross-deduped)",
                paths[0], paths[1], report.members, report.scoped, report.kept, report.imports, report.deduped
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

/// Sections the object-crate writer regenerates from the symbol and
/// relocation re-adds (.symtab/.strtab/.shstrtab, the .rel./.rela.
/// relocation set) plus the COMDAT bookkeeping it cannot represent
/// (.group). The .rel. form carries the trailing dot: a bare ".rel"
/// prefix would also match .data.rel.ro*, a real data section every
/// rustc object with relocated statics carries.
fn skipped_bookkeeping(name: &[u8]) -> bool {
    matches!(
        name,
        b".symtab" | b".strtab" | b".shstrtab" | b".group" | b".rel"
    ) || name.starts_with(b".rela")
        || name.starts_with(b".rel.")
}

#[derive(Default, Debug)]
struct Report {
    members: usize,
    scoped: usize,
    kept: usize,
    imports: usize,
    deduped: usize,
}

/// The (name, content) index of an already-scoped base archive, for
/// `--dedupe-against`. cargo's staticlib bundling packs the base crate's
/// whole native closure into every dependent archive (the driver's graph
/// includes tfs, so libtebako_driver.a ships libtfs.a's objects byte for
/// byte); linked together that is a duplicate definition of every shared
/// member — GNU ld shrugs first-wins, but Apple ld asserts (ld_prime) or
/// errors "duplicate symbol" (ld_classic, the 0.16.3-era macos x86_64
/// legs). Members match on (final name, final bytes) — post-scope and
/// post-canonicalization, so raw byte-identical members still meet (the
/// rewrite is deterministic), and a name-shared member whose content
/// differs is KEPT (the keep-both import sets below: two windows-targets
/// lines, one canonical name, different thunk sets — dropping by name
/// alone would silently lose one).
fn base_member_keys(path: &str) -> Result<std::collections::HashSet<(String, Vec<u8>)>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let archive = object::read::archive::ArchiveFile::parse(&bytes[..])
        .map_err(|e| format!("cannot parse {path} as an archive: {e}"))?;
    let mut keys = std::collections::HashSet::new();
    for member in archive.members() {
        let member = member.map_err(|e| format!("cannot read a member of {path}: {e}"))?;
        let name = String::from_utf8_lossy(member.name()).into_owned();
        if name == "/" || name == "//" || name.starts_with("__.SYMDEF") {
            continue;
        }
        let data = member
            .data(&bytes[..])
            .map_err(|e| format!("cannot read member {name} of {path}: {e}"))?;
        keys.insert((name, data.to_vec()));
    }
    Ok(keys)
}

fn run(
    input: &str,
    output: &str,
    keep: &str,
    prefix: &str,
    dedupe_against: Option<&str>,
) -> Result<Report, String> {
    let bytes = std::fs::read(input).map_err(|e| format!("cannot read {input}: {e}"))?;
    let archive = object::read::archive::ArchiveFile::parse(&bytes[..])
        .map_err(|e| format!("cannot parse {input} as an archive: {e}"))?;

    // Pass A: collect every symbol this tool will RENAME anywhere in
    // the archive (the rename set for references — a renamed definition
    // must be matched by its references or the link breaks). Mach-O
    // members use the raw nlist scan so this pass and the rewrite share
    // one rule; other formats use the object crate.
    let mut defined: std::collections::HashSet<String> = std::collections::HashSet::new();
    // The object format decides the archive's index form (BSD __.SYMDEF
    // for Mach-O, GNU "/" elsewhere) and whether members get ld64's
    // 4-byte content pad (Mach-O only — see Pass B). Detected here: one
    // Mach-O member makes the archive Mach-O.
    let mut is_macho = false;
    for member in archive.members() {
        let member = member.map_err(|e| format!("cannot read a member of {input}: {e}"))?;
        let data = member
            .data(&bytes[..])
            .map_err(|e| format!("cannot read a member of {input}: {e}"))?;
        match macho::defined(data, keep) {
            Ok(names) => {
                is_macho = true;
                defined.extend(names);
            }
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

    // The index FORM follows the object format inside: Mach-O archives
    // get the BSD __.SYMDEF (the cargo/llvm-ranlib form), everything
    // else the GNU "/" index. (The input's own index member, when it has
    // one, is the same discriminator in practice; a hand-made Mach-O
    // fixture without one still gets the BSD form.)
    let bsd = is_macho;

    // Pass B: rewrite every member, collecting its exported names
    // (post-rename) for the archive symbol index.
    let mut report = Report::default();
    let mut members: Vec<(String, Vec<u8>, Vec<String>)> = Vec::new();
    // Canonical-name registry for import members: name -> index of the
    // first surviving member in `members`. Byte-identical duplicates
    // (rustc's bundler ships the same generated import lib through
    // several crate paths) collapse. A same-name member with DIFFERENT
    // bytes is a second import set for the same DLL — the dependency
    // graph legitimately carries two windows-targets lines whose
    // per-DLL thunk sets differ (the v2.8.5 windows link-unit:
    // bcryptprimitives.dllt.o twice) — and archives have no
    // unique-member-name rule: ld pulls members by SYMBOL through the
    // index, never by name, and its PE script's
    // SORT_BY_NAME(archive(member)) still groups both sets adjacently
    // under the one canonical name. Keep both; a genuine same-symbol
    // redefinition surfaces as ld's own duplicate-definition error,
    // never a silent pick. (The dedup compares against the first
    // survivor only: a third member byte-identical to a kept later set
    // is kept too — redundant, never wrong.)
    let mut import_names: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let base_keys = match dedupe_against {
        Some(base) => Some(base_member_keys(base)?),
        None => None,
    };
    for member in archive.members() {
        let member = member.map_err(|e| format!("cannot read a member of {input}: {e}"))?;
        let name = String::from_utf8_lossy(member.name()).into_owned();
        let data = member
            .data(&bytes[..])
            .map_err(|e| format!("cannot read member {name} of {input}: {e}"))?;
        let canonical = canonical_import_member_name(&name, data);
        let (mut rewritten, exported) = scope_object(data, keep, prefix, &defined, &mut report)
            .map_err(|reason| format!("member {name}: {reason}"))?;
        let _ = &exported;
        // ld64 does not follow the 2-byte ar member alignment: its
        // archive walk advances to the next member at
        // align4(header + size). A member whose size is not a multiple
        // of 4 desyncs the walk ("archive member invalid control bits")
        // or pushes the computed extent past EOF ("malformed archive,
        // member exceeds file size"). Pad every MACH-O member's content
        // to a multiple of 4 — trailing zeros are harmless slack to a
        // Mach-O object (all load-command offsets are unaffected).
        //
        // The pad is Mach-O-only. A COFF short-form import member's
        // size is exact — 20 header bytes + SizeOfData — and lld's
        // ImportFile parse rejects a longer member outright ("broken
        // import library"). v2.8.14/v2.8.15 shipped the pad on COFF:
        // 1162 of the aarch64 libtebako_driver.a's 1536 short-form
        // members each carried 1–3 slack bytes, killing every
        // windows-arm64 exe link (python factory run 35560837935,
        // ruby factory run 35560840341). COFF objects tolerate the
        // zeros, but there is no COFF consumer that needs them.
        if bsd {
            let pad4 = (4 - rewritten.len() % 4) % 4;
            rewritten.extend_from_slice(&[0; 4][..pad4]);
        }
        let (name, is_import) = match canonical {
            Some(canon) => {
                report.imports += 1;
                (canon, true)
            }
            None => (name, false),
        };
        // The cross-archive dedupe (stage 2b of the old staging tool, moved
        // here): a member the base archive already carries, byte for byte
        // under the same final name, is dropped — re-archiving through
        // tempfile basenames (the Ruby deduper's `%05d_` prefix) is what
        // re-broke the import members' h < sNNNNN < t sort AFTER arscope
        // had restored it (the v2.8.6 windows miniruby: ncrypt.dll's
        // lookup run swallowed api-ms-win-core-synch's, 0xC0000139). It
        // runs before the import registry below so a dropped member never
        // claims a survivors' slot index.
        if let Some(keys) = &base_keys {
            if keys.contains(&(name.clone(), rewritten.clone())) {
                report.deduped += 1;
                continue;
            }
        }
        if is_import {
            match import_names.get(&name) {
                Some(&prev) if members[prev].1 == rewritten => continue,
                Some(_) => {}
                None => {
                    import_names.insert(name.clone(), members.len());
                }
            }
        }
        report.members += 1;
        members.push((name, rewritten, exported));
    }

    // The archive symbol index: ld consumes archives THROUGH it (a
    // missing index is "no table of contents"); its form was decided by
    // the object format in pass A.
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

/// The sort-canonical name for a dlltool-style long-form import member
/// (a COFF object carrying `.idata$N` sections), or None for any other
/// member. GNU ld's PE scripts collect each DLL's `.idata$4`/`.idata$5`
/// chunks via SORT_BY_NAME on the `archive(member)` string, and a DLL's
/// import lookup run terminates at its null chunk only when the DLL's
/// members sort h < sNNNNN < t. rustc's staticlib bundler disambiguates
/// duplicate member names with a numeric "NNNNN_" prefix, which then
/// sorts by bundle position instead of by the h/s/t suffix — the null
/// terminator (t) lands ahead of the entries (s) and the descriptor
/// swallows the next DLL's functions (the v2.8.4 windows runtime defect:
/// miniruby imported WaitOnAddress from ncrypt.dll, the loader answered
/// STATUS_ENTRYPOINT_NOT_FOUND 0xC0000139, the shell reported exit 127).
/// Stripping exactly one leading numeric prefix restores the canonical
/// order; members already canonical pass through unchanged. Short-format
/// import objects carry their DLL name in-band and never reach here (the
/// object-crate parse fails on them — scope_object passes them through
/// byte-identical with their in-band names indexed).
fn canonical_import_member_name(name: &str, data: &[u8]) -> Option<String> {
    let obj = File::parse(data).ok()?;
    if obj.format() != object::BinaryFormat::Coff {
        return None;
    }
    let is_import = obj.sections().any(|s| {
        s.name_bytes()
            .map(|n| n.starts_with(b".idata$"))
            .unwrap_or(false)
    });
    if !is_import {
        return None;
    }
    let stripped = match name.find('_') {
        Some(at) if !name[..at].is_empty() && name[..at].bytes().all(|b| b.is_ascii_digit()) => {
            &name[at + 1..]
        }
        _ => name,
    };
    Some(stripped.to_string())
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
    let obj = match File::parse(data) {
        Ok(obj) => obj,
        Err(e) => {
            // rustc's gnullvm raw-dylib import sets carry one SHORT-FORM
            // import object per imported function next to the long-form
            // descriptor/thunk members (the 53-byte ProcessPrng member of
            // run 35512778802's libtfs.a). The form is self-contained —
            // no sections, no relocations — and its symbols name a SYSTEM
            // DLL's exports, never archive-internal definitions (pass A
            // skips unparseable members, so references to them stay
            // unprefixed to match). It passes through byte-identical; the
            // archive index still needs the names it defines, or ld can
            // never reach the member.
            if let Some(exports) = short_form_import_exports(data) {
                verify_coff_short_import(data)?;
                report.imports += 1;
                return Ok((data.to_vec(), exports));
            }
            return Err(format!("not an object file: {e}"));
        }
    };
    // Mach-O: raw LC_SYMTAB surgery (sections and relocations stay
    // byte-identical — the general rewrite breaks ld64's atomizers).
    if obj.format() == object::BinaryFormat::MachO {
        // A member with no LC_SYMTAB carries no symbols at all (an
        // empty TU or an LTO bitcode blob): nothing to rename, nothing
        // exported — inert to scoping by construction, so it passes
        // through byte-identical (macos-x86_64: filters_comp_filter.o).
        // The literal is macho.rs's symtab() error for exactly this
        // case — one crate, one spelling.
        if let Err(e) = macho::defined(data, keep) {
            if e == "no LC_SYMTAB in this Mach-O object" {
                return Ok((data.to_vec(), Vec::new()));
            }
            return Err(format!("Mach-O member: {e}"));
        }
        let (out, exported, renamed, kept) = macho::scope(data, keep, prefix, defined)?;
        report.scoped += renamed;
        report.kept += kept;
        return Ok((out, exported));
    }
    let mut out = object::write::Object::new(obj.format(), obj.architecture(), obj.endianness());
    // The archive-index names: every defined, externally visible symbol.
    let mut exported: Vec<String> = Vec::new();

    let mut section_ids = std::collections::HashMap::new();
    let mut skipped: std::collections::HashSet<object::SectionIndex> =
        std::collections::HashSet::new();
    for section in obj.sections() {
        let name = section
            .name_bytes()
            .map_err(|e| format!("section name: {e}"))?
            .to_vec();
        // Sections the writer regenerates from the symbol/relocation
        // re-adds below (.symtab/.strtab/.shstrtab, the .rel./.rela.
        // relocation set) and the COMDAT bookkeeping it cannot
        // represent (.group): copying them as DATA produced stray
        // mis-typed sections (a PROGBITS .strtab, PROGBITS relocation
        // dumps) that GNU ld then resolves against — links succeeded,
        // binaries died at startup (the gnu leg's miniruby; ld64 and
        // the mingw link tolerate the strays, which is why only the
        // ELF path broke). The .rel. form carries the trailing dot:
        // a bare ".rel" prefix would also eat .data.rel.ro*, a real
        // data section every rustc object with relocated statics has
        // (its symbols then missed their section mapping).
        if skipped_bookkeeping(&name) {
            skipped.insert(section.index());
            continue;
        }
        let segment = section
            .segment_name()
            .map_err(|e| format!("section segment name: {e}"))?
            .unwrap_or("")
            .as_bytes()
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
    // Undefined section symbols carried through the writer as plain
    // undefined data symbols, re-marked as IMAGE_SYM_CLASS_SECTION after
    // the write (see coff_mark_undefined_section_symbols).
    let mut carried_section_symbols: Vec<&str> = Vec::new();
    // COFF COMDAT groups captured from the section symbols' aux records
    // (head section → selection byte; associative member → head section)
    // — re-registered through add_comdat after the symbol pass.
    let mut comdat_heads: Vec<(object::SectionIndex, u8)> = Vec::new();
    let mut comdat_members: Vec<(object::SectionIndex, object::SectionIndex)> = Vec::new();
    for symbol in obj.symbols() {
        // COFF section symbols are per-section bookkeeping the writer
        // regenerates (its add_symbol routes every Section-kind symbol at
        // its own per-section record). Relocations reference them by
        // index — remap those to the writer's own section symbol for the
        // same section.
        if symbol.kind() == object::SymbolKind::Section {
            if let object::SymbolSection::Section(index) = symbol.section() {
                // A skipped bookkeeping section (.rela & co) can carry a
                // section symbol; it has no counterpart in the output
                // and nothing references it there.
                if let Some(out_index) = section_ids.get(&index) {
                    let id = out.section_symbol(*out_index);
                    symbol_ids.insert(symbol.index(), id);
                    // The regenerated section symbol's aux record starts
                    // life with Selection 0 — while the section header
                    // keeps IMAGE_SCN_LNK_COMDAT from the passed-through
                    // characteristics. ld.bfd never reads the byte (the
                    // ucrt64 x64 legs shipped like this unnoticed); lld
                    // rejects selection 0 outright ("unknown comdat type
                    // 0", python factory run 35532416136 — 3338 COMDAT
                    // sections in the shipped v2.8.13 aarch64 link unit).
                    // Capture the group here; add_comdat re-emits it.
                    if let object::SymbolFlags::CoffSection {
                        selection,
                        associative_section,
                    } = symbol.flags()
                    {
                        match selection {
                            0 => {}
                            object::pe::IMAGE_COMDAT_SELECT_ASSOCIATIVE => {
                                let Some(head) = associative_section else {
                                    return Err(format!(
                                        "associative COMDAT section '{}' names no head section — refusing to guess",
                                        symbol.name().unwrap_or("<unnamed>")
                                    ));
                                };
                                comdat_members.push((index, head));
                            }
                            object::pe::IMAGE_COMDAT_SELECT_NODUPLICATES
                                ..=object::pe::IMAGE_COMDAT_SELECT_EXACT_MATCH
                            | object::pe::IMAGE_COMDAT_SELECT_LARGEST
                            | object::pe::IMAGE_COMDAT_SELECT_NEWEST => {
                                comdat_heads.push((index, selection));
                            }
                            other => {
                                return Err(format!(
                                    "section '{}' carries an unknown COMDAT selection ({other}) — extend the mapping instead of dropping it",
                                    symbol.name().unwrap_or("<unnamed>")
                                ));
                            }
                        }
                    }
                }
            } else if matches!(symbol.section(), object::SymbolSection::Undefined)
                && obj.format() == object::BinaryFormat::Coff
            {
                // gnullvm raw-dylib descriptor members reference the
                // thunk member's .idata$4/.idata$5 chunks through
                // UNDEFINED section symbols (IMAGE_SYM_CLASS_SECTION with
                // section number 0 — a cross-member section reference;
                // run 35512778802's bcryptprimitives.dll descriptor; the
                // reader reports it as Section-kind + SymbolSection::
                // Undefined, and NOT is_undefined — that predicate is
                // EXTERNAL-class-only). The writer's public API cannot
                // emit that shape (its Section-kind path needs an output
                // section), so the symbol rides through as an undefined
                // data symbol and is re-marked after the write.
                // Compilation scope, never renamed — the thunk member's
                // chunk keeps its name.
                let name = symbol.name().map_err(|e| format!("symbol name: {e}"))?;
                let id = out.add_symbol(object::write::Symbol {
                    name: name.as_bytes().to_vec(),
                    value: 0,
                    size: 0,
                    kind: object::SymbolKind::Data,
                    scope: SymbolScope::Compilation,
                    weak: false,
                    section: object::write::SymbolSection::Undefined,
                    flags: object::SymbolFlags::None,
                });
                symbol_ids.insert(symbol.index(), id);
                carried_section_symbols.push(name);
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
                if skipped.contains(&index) {
                    // A symbol defined in bookkeeping data (a group
                    // signature symbol and its kin) is referenced only
                    // from the skipped section itself and drops with
                    // it — but an exported definition there means the
                    // skip list ate something real: name it, loudly.
                    let visible = symbol.is_weak()
                        || matches!(symbol.scope(), SymbolScope::Linkage | SymbolScope::Dynamic);
                    if visible && !symbol.is_undefined() {
                        return Err(format!(
                            "symbol '{}' is defined in a skipped bookkeeping section — refusing to drop an exported definition",
                            symbol.name().unwrap_or("<unnamed>")
                        ));
                    }
                    continue;
                }
                object::write::SymbolSection::Section(section_ids[&index])
            }
            object::SymbolSection::Undefined => object::write::SymbolSection::Undefined,
            object::SymbolSection::Absolute => object::write::SymbolSection::Absolute,
            object::SymbolSection::Common => object::write::SymbolSection::Common,
            object::SymbolSection::None if symbol.kind() == object::SymbolKind::File => {
                // COFF .file bookkeeping rides IMAGE_SYM_DEBUG (-2),
                // which the reader reports as SymbolSection::None and
                // the writer maps straight back to -2 (the enum's only
                // "no section" slot — its doc names file symbols as the
                // case). Letting it fall into the catch-all turns -2
                // into 0 and ld.lld refuses the link (".file should not
                // refer to special section 0", python factory run
                // 35543897983 — 401 such symbols across the shipped
                // v2.8.14 aarch64 link unit; ld.bfd never reads it).
                object::write::SymbolSection::None
            }
            _ => object::write::SymbolSection::Undefined,
        };
        let flags = map_symbol_flags(symbol.flags(), &section_ids, &symbol_ids)?;
        // object 0.37's ELF reader maps STB_GNU_UNIQUE (glibc-target
        // g++'s vague-linkage binding — template/inline statics;
        // libstdc++.a alone carries 154) to SymbolScope::Unknown, and its
        // writer asserts a defined symbol is scoped (write/mod.rs:434).
        // Re-scope to Linkage for the write. Only the gnu legs carry such
        // members (musl and Mach-O have no GNU_UNIQUE), which is why the
        // floor leg's scoper panic (run 30988106906) was the first time
        // a gnu release build reached this code.
        //
        // The same reader maps a FILE symbol's SHN_UNDEF to
        // SymbolScope::Unknown too (the SHN_UNDEF short-circuit runs
        // before the binding match — read/elf/symbol.rs:452). Left
        // unscoped, the writer files it in the NON-LOCAL symtab region
        // while the passthrough st_info keeps STB_LOCAL: an inconsistent
        // symtab ("local symbol at index N (>= sh_info of N)") that
        // binutils 2.34 rejects outright (tebako#413's floor). Re-scope
        // FILE bookkeeping to Compilation — the writer's local region,
        // where its binding belongs (rustc emits one FILE symbol per TU).
        let scope = match symbol.scope() {
            SymbolScope::Unknown if symbol.kind() == object::SymbolKind::File => {
                SymbolScope::Compilation
            }
            SymbolScope::Unknown if !symbol.is_undefined() => SymbolScope::Linkage,
            scope => scope,
        };
        // STB_GNU_UNIQUE neutralization (tebako#413): the binding is a
        // DYNAMIC-linking construct (an ld.so process-singleton across
        // DSOs) and the scoped link unit is a STATIC archive set with no
        // DSO boundary for it to govern. Worse, the rewrite cannot keep
        // the SHT_GROUP COMDAT section the binding folds through (the
        // object crate emits no .group — see the SHF_GROUP clear above),
        // leaving a group-less GNU_UNIQUE that binutils ld 2.34 (the
        // factory's ubuntu:20.04 floor) treats as a STRONG definition:
        // the second archive member defining the same inline variable
        // (libstdc++'s __to_chars_10_impl::__digits, std::ranges::
        // __cust::*, nlohmann::json statics) is a "multiple definition"
        // error (tebako-runtime-ruby#107; v0.1.9's libtfs.a carried
        // 1356). Demote DEFINED GNU_UNIQUE symbols to WEAK: name-based
        // coalescing at static link time is exactly the vague-linkage
        // semantics minus the DSO property — the same object g++
        // -fno-gnu-unique would have emitted. Undefined GNU_UNIQUE
        // references stay strong: a reference must resolve, never
        // silently zero.
        let mut weak = symbol.is_weak();
        let flags = match flags {
            object::SymbolFlags::Elf { st_info, st_other }
                if !symbol.is_undefined() && st_info >> 4 == STB_GNU_UNIQUE =>
            {
                weak = true;
                object::SymbolFlags::Elf {
                    st_info: (STB_WEAK << 4) | (st_info & 0x0f),
                    st_other,
                }
            }
            other => other,
        };
        let id = out.add_symbol(object::write::Symbol {
            name: renamed.into_bytes(),
            value: symbol.address(),
            size: symbol.size(),
            kind: symbol.kind(),
            scope,
            weak,
            section,
            flags,
        });
        symbol_ids.insert(symbol.index(), id);
    }

    // Re-register the captured COFF COMDAT groups: the writer derives the
    // section-symbol aux record (selection byte, associative number) and
    // the IMAGE_SCN_LNK_COMDAT characteristic from these. The capture
    // above already rejected every selection outside 1..=7, so the
    // fall-through arm is Newest by construction.
    for (head_index, selection) in &comdat_heads {
        let kind = match *selection {
            object::pe::IMAGE_COMDAT_SELECT_NODUPLICATES => object::ComdatKind::NoDuplicates,
            object::pe::IMAGE_COMDAT_SELECT_ANY => object::ComdatKind::Any,
            object::pe::IMAGE_COMDAT_SELECT_SAME_SIZE => object::ComdatKind::SameSize,
            object::pe::IMAGE_COMDAT_SELECT_EXACT_MATCH => object::ComdatKind::ExactMatch,
            object::pe::IMAGE_COMDAT_SELECT_LARGEST => object::ComdatKind::Largest,
            _ => object::ComdatKind::Newest,
        };
        let head_id = section_ids[head_index];
        let mut sections = vec![head_id];
        for (member, head) in &comdat_members {
            if head == head_index {
                // Members are captured only when their section mapped, so
                // the output id exists; section_symbol() memoizes — this
                // call only guarantees the writer's "missing symbol for
                // COMDAT section" invariant.
                let member_id = section_ids[member];
                out.section_symbol(member_id);
                sections.push(member_id);
            }
        }
        let symbol = out.section_symbol(head_id);
        out.add_comdat(object::write::Comdat {
            kind,
            sections,
            symbol,
        });
    }

    for section in obj.sections() {
        // Skipped bookkeeping sections carry no relocations that matter
        // (their own data is never emitted); a real section always has
        // its output id here.
        let Some(out_section) = section_ids.get(&section.index()) else {
            continue;
        };
        for (offset, relocation) in section.relocations() {
            let symbol = match relocation.target() {
                RelocationTarget::Symbol(index) => *symbol_ids.get(&index).ok_or_else(|| {
                    format!(
                        "relocation at {offset:#x} references a dropped bookkeeping symbol — unhandled case"
                    )
                })?,
                RelocationTarget::Section(index) => {
                    let Some(out_index) = section_ids.get(&index) else {
                        return Err(format!(
                            "relocation targets a skipped bookkeeping section (offset {offset:#x}) — not seen for compiler-emitted objects"
                        ));
                    };
                    out.section_symbol(*out_index)
                }
                other => {
                    return Err(format!(
                        "an unsupported relocation target ({other:?}, offset {offset:#x}, kind {:?}) — implement it when the ELF leg needs it",
                        relocation.kind()
                    ));
                }
            };
            out.add_relocation(
                *out_section,
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

    let mut bytes = out
        .write()
        .map_err(|e| format!("cannot emit the rewritten object: {e}"))?;
    if obj.format() == object::BinaryFormat::Coff {
        verify_coff_comdat(&bytes)?;
        verify_coff_file_records(&bytes)?;
    }
    if !carried_section_symbols.is_empty() {
        coff_mark_undefined_section_symbols(&mut bytes, &carried_section_symbols)?;
    }
    Ok((bytes, exported))
}

/// The rewrite's load-bearing COFF invariant, asserted on the emitted
/// bytes: every COMDAT-flagged section must have a section symbol whose
/// aux record carries a non-zero Selection. v2.8.13 shipped aarch64 link
/// units with 3338 COMDAT sections whose regenerated section symbols all
/// read Selection 0 — ld.bfd ignored it, ld.lld refuses the link
/// ("unknown comdat type 0", python factory run 35532416136). The
/// regression must never ship silently again.
fn verify_coff_comdat(bytes: &[u8]) -> Result<(), String> {
    let obj = File::parse(bytes)
        .map_err(|e| format!("internal: cannot re-parse the rewritten COFF object: {e}"))?;
    let mut comdat_sections = std::collections::HashSet::new();
    for section in obj.sections() {
        if let object::SectionFlags::Coff { characteristics } = section.flags() {
            if characteristics & object::pe::IMAGE_SCN_LNK_COMDAT != 0 {
                comdat_sections.insert(section.index());
            }
        }
    }
    if comdat_sections.is_empty() {
        return Ok(());
    }
    let mut validated = std::collections::HashSet::new();
    for symbol in obj.symbols() {
        if symbol.kind() != object::SymbolKind::Section {
            continue;
        }
        let object::SymbolSection::Section(index) = symbol.section() else {
            continue;
        };
        if !comdat_sections.contains(&index) {
            continue;
        }
        let valid = matches!(
            symbol.flags(),
            object::SymbolFlags::CoffSection { selection, .. } if selection != 0
        );
        if !valid {
            return Err(format!(
                "internal: the rewritten COFF object lost the COMDAT selection of section '{}' — refusing to emit an ld.bfd-only object",
                symbol.name().unwrap_or("<unnamed>")
            ));
        }
        validated.insert(index);
    }
    if let Some(missing) = comdat_sections.difference(&validated).next() {
        let name = obj
            .sections()
            .find(|s| s.index() == *missing)
            .and_then(|s| s.name().ok().map(str::to_owned))
            .unwrap_or_else(|| format!("#{missing:?}"));
        return Err(format!(
            "internal: COMDAT-flagged section '{name}' has no section symbol in the rewritten COFF object"
        ));
    }
    Ok(())
}

/// The rewrite's second load-bearing COFF invariant, asserted on the
/// emitted bytes: every .file symbol must sit at IMAGE_SYM_DEBUG (-2),
/// which the reader reports as SymbolSection::None. v2.8.14 shipped
/// aarch64 link units with 401 .file symbols rewritten to section 0 —
/// ld.bfd ignored it, ld.lld refuses the link (".file should not refer
/// to special section 0", python factory run 35543897983).
fn verify_coff_file_records(bytes: &[u8]) -> Result<(), String> {
    let obj = File::parse(bytes)
        .map_err(|e| format!("internal: cannot re-parse the rewritten COFF object: {e}"))?;
    for symbol in obj.symbols() {
        if symbol.kind() != object::SymbolKind::File {
            continue;
        }
        if !matches!(symbol.section(), object::SymbolSection::None) {
            return Err(format!(
                "internal: the rewritten COFF object carries .file symbol '{}' at a section other than IMAGE_SYM_DEBUG — refusing to emit an ld.bfd-only object",
                symbol.name().unwrap_or("<unnamed>")
            ));
        }
    }
    Ok(())
}

/// The rewrite's third load-bearing COFF invariant, asserted on the
/// member bytes: a short-form import member's size is EXACTLY the
/// 20-byte header plus its SizeOfData string tail. lld's ImportFile
/// parse rejects any other length ("broken import library"; the check
/// fires before a name is even read). v2.8.14/v2.8.15 shipped 1162
/// short-form members in the aarch64 libtebako_driver.a carrying the
/// writer's 4-byte alignment slack (CloseHandle 48 bytes against 45
/// exact) — ld.bfd ignored it, ld.lld refused every windows-arm64 exe
/// link (python factory run 35560837935, ruby factory run 35560840341).
/// Called on the passthrough path, so a pre-padded input is refused
/// just as a writer-side regression would be.
fn verify_coff_short_import(data: &[u8]) -> Result<(), String> {
    let size_of_data = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
    if data.len() != 20 + size_of_data {
        return Err(format!(
            "internal: short-form import member is {} bytes, not the exact 20 + SizeOfData ({}) — refusing to emit an ld.bfd-only member",
            data.len(),
            20 + size_of_data
        ));
    }
    Ok(())
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

/// The defined names of a short-form import object (the PE/COFF
/// IMPORT_OBJECT_HEADER form — sig1 0, sig2 0xFFFF, then a 20-byte
/// header and two NUL-terminated strings: the symbol name and the DLL
/// name), or None when the bytes are not the form. rustc's gnullvm
/// raw-dylib bundler emits one per imported function. A code import
/// defines both the thunk and the IAT entry; a data/const import is
/// reached through the IAT entry alone. (The `__imp_` spelling is the
/// x64/arm64 decoration — no 32-bit platform ships.)
fn short_form_import_exports(data: &[u8]) -> Option<Vec<String>> {
    if data.len() < 21 || data[0..4] != [0, 0, 0xff, 0xff] {
        return None;
    }
    let import_type = u16::from_le_bytes([data[18], data[19]]) & 3; // 0 code, 1 data, 2 const
    let tail = &data[20..];
    let end = tail.iter().position(|&b| b == 0)?;
    let name = std::str::from_utf8(&tail[..end]).ok()?;
    if name.is_empty() {
        return None;
    }
    let iat = format!("__imp_{name}");
    Some(match import_type {
        0 => vec![name.to_string(), iat],
        _ => vec![iat],
    })
}

/// Mark the named UNDEFINED symbols of a freshly written COFF object as
/// section symbols (storage class IMAGE_SYM_CLASS_SECTION, section
/// number 0 — the dlltool long-form descriptor's cross-member references
/// to the thunk member's `.idata$4`/`.idata$5` chunks, the gnullvm
/// raw-dylib shape of run 35512778802's libtfs.a). The object crate's
/// writer cannot express the shape publicly: add_symbol routes EVERY
/// Section-kind symbol at section_symbol(), which unwraps an output
/// section id (write/mod.rs:438) — and an undefined section symbol has
/// none by construction. The symbols therefore ride through the writer
/// as plain undefined data symbols (class EXTERNAL, section number 0)
/// and the storage-class byte is set here, on OUR OWN just-written
/// deterministic layout. Every named record must be found and flipped
/// exactly once, or the rewrite fails loudly.
fn coff_mark_undefined_section_symbols(bytes: &mut [u8], names: &[&str]) -> Result<(), String> {
    if bytes.len() < 20 {
        return Err("internal: a written COFF object shorter than its header".to_string());
    }
    let symptr = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let nsyms = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let strtab = symptr + nsyms * 18;
    if symptr == 0 || strtab + 4 > bytes.len() {
        return Err("internal: the written COFF object has no symbol table".to_string());
    }
    fn record_name(bytes: &[u8], strtab: usize, rec: &[u8]) -> Option<String> {
        if rec[0..4] == [0, 0, 0, 0] {
            let off = u32::from_le_bytes(rec[4..8].try_into().unwrap()) as usize;
            let at = strtab.checked_add(off)?;
            let end = bytes[at..].iter().position(|&b| b == 0)?;
            Some(String::from_utf8_lossy(&bytes[at..at + end]).into_owned())
        } else {
            let end = rec[0..8].iter().position(|&b| b == 0).unwrap_or(8);
            Some(String::from_utf8_lossy(&rec[0..end]).into_owned())
        }
    }
    let mut flipped: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut i = 0usize;
    while i < nsyms {
        let at = symptr + i * 18;
        if at + 18 > bytes.len() {
            return Err("internal: the written COFF symbol table overruns the file".to_string());
        }
        let class = bytes[at + 16];
        let secnum = i16::from_le_bytes([bytes[at + 12], bytes[at + 13]]);
        let aux = bytes[at + 17] as usize;
        if class == 2 && secnum == 0 && aux == 0 {
            if let Some(name) = record_name(bytes, strtab, &bytes[at..at + 18]) {
                if names.iter().any(|n| *n == name) {
                    bytes[at + 16] = 104; // IMAGE_SYM_CLASS_SECTION
                    *flipped.entry(name).or_insert(0) += 1;
                }
            }
        }
        i += 1 + aux;
    }
    for name in names {
        match flipped.get(*name) {
            Some(1) => {}
            other => {
                return Err(format!(
                    "internal: carried section symbol {name} flipped {other:?} times, expected exactly 1"
                ))
            }
        }
    }
    Ok(())
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
        // macho.rs scopes the raw nlist name VERBATIM (leading
        // underscore kept — blake3's dual `_sym`/`sym` spellings must
        // not collapse), so the Mach-O expectation carries the doubled
        // underscore; the ELF path has no mangling prefix to keep.
        let internal = if cfg!(target_os = "macos") {
            "__tebako_internal__rust_eh_personality"
        } else {
            "__tebako_internal_rust_eh_personality"
        };
        assert!(
            has("tebako_probe"),
            "the public surface survives: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == internal),
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

    /// A defined STB_GNU_UNIQUE symbol (glibc-target g++'s vague-linkage
    /// binding — every C++ object with template/inline statics carries
    /// them on the gnu legs) reads back as SymbolScope::Unknown; the
    /// rewrite must re-scope it for the writer instead of panicking on
    /// its defined-symbol assert (the floor leg's scoper panic, release
    /// run 30988106906) — and, since tebako#413, demote the binding to
    /// STB_WEAK (the rewrite drops the SHT_GROUP the binding folds
    /// through, and binutils 2.34 rejects group-less duplicates). The
    /// second fixture symbol is an UNDEFINED GNU_UNIQUE reference: it
    /// must stay strong (a reference must resolve, never silently zero).
    /// ELF-only: Mach-O has no such binding.
    fn fixture_object_gnu_unique() -> Vec<u8> {
        let arch = if cfg!(target_arch = "aarch64") {
            object::Architecture::Aarch64
        } else {
            object::Architecture::X86_64
        };
        let mut out =
            object::write::Object::new(object::BinaryFormat::Elf, arch, object::Endianness::Little);
        let text = out.add_section(
            b".text".to_vec(),
            b".text".to_vec(),
            object::SectionKind::Text,
        );
        out.section_mut(text).set_data(b"\x90\x90\xc3", 1);
        out.add_symbol(object::write::Symbol {
            name: b"_ZN1HIiE1vE".to_vec(),
            value: 0,
            size: 4,
            kind: object::SymbolKind::Data,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(text),
            // st_info = STB_GNU_UNIQUE (10) << 4 | STT_OBJECT (1)
            flags: object::SymbolFlags::Elf {
                st_info: (10 << 4) | 1,
                st_other: 0,
            },
        });
        out.add_symbol(object::write::Symbol {
            name: b"_ZN1WIiE1uE".to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::Unknown,
            scope: object::SymbolScope::Unknown,
            weak: false,
            section: object::write::SymbolSection::Undefined,
            // st_info = STB_GNU_UNIQUE (10) << 4 | STT_NOTYPE (0)
            flags: object::SymbolFlags::Elf {
                st_info: 10 << 4,
                st_other: 0,
            },
        });
        // The TU bookkeeping symbol every rustc/g++ object carries:
        // STB_LOCAL | STT_FILE at SHN_UNDEF — object 0.37's reader maps
        // the SHN_UNDEF to SymbolScope::Unknown, and without the re-scope
        // in scope_object the writer would file it in the non-local
        // symtab region (a symtab binutils 2.34 rejects as inconsistent).
        out.add_symbol(object::write::Symbol {
            name: b"fixture.c".to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::File,
            scope: object::SymbolScope::Compilation,
            weak: false,
            section: object::write::SymbolSection::Undefined,
            // st_info = STB_LOCAL (0) << 4 | STT_FILE (4)
            flags: object::SymbolFlags::Elf {
                st_info: 4,
                st_other: 0,
            },
        });
        out.write().expect("fixture object")
    }

    #[test]
    fn gnu_unique_definitions_demote_to_weak_references_stay_strong() {
        let fixture = fixture_object_gnu_unique();
        let obj = File::parse(&fixture[..]).expect("parse the fixture");
        let sym = obj
            .symbols()
            .find(|s| s.name().ok() == Some("_ZN1HIiE1vE"))
            .expect("the fixture carries the unique symbol");
        assert_eq!(
            sym.scope(),
            SymbolScope::Unknown,
            "the fixture must read back the way glibc-target objects do"
        );

        let mut report = Report::default();
        let (bytes, _exported) = scope_object(
            &fixture,
            KEEP_PREFIX,
            SCOPE_PREFIX,
            &std::collections::HashSet::new(),
            &mut report,
        )
        .expect("scope the fixture");
        let obj = File::parse(&bytes[..]).expect("parse the rewritten object");

        // The definition demotes to STB_WEAK, type (STT_OBJECT) intact.
        let sym = obj
            .symbols()
            .find(|s| s.name().ok() == Some("_ZN1HIiE1vE"))
            .expect("the unique symbol survives the rewrite");
        assert!(!sym.is_undefined());
        assert!(
            sym.is_weak(),
            "a defined GNU_UNIQUE demotes to weak (tebako#413)"
        );
        match sym.flags() {
            object::SymbolFlags::Elf { st_info, .. } => {
                assert_eq!(st_info >> 4, STB_WEAK, "the emitted binding is WEAK");
                assert_eq!(st_info & 0x0f, 1, "the symbol type is preserved");
            }
            other => panic!("expected ELF symbol flags, got {other:?}"),
        }

        // The undefined GNU_UNIQUE reference keeps its strong binding:
        // demoting a reference to weak would let a missing definition
        // resolve to zero instead of failing the link.
        let sym = obj
            .symbols()
            .find(|s| s.name().ok() == Some("_ZN1WIiE1uE"))
            .expect("the undefined reference survives the rewrite");
        assert!(sym.is_undefined());
        match sym.flags() {
            object::SymbolFlags::Elf { st_info, .. } => {
                assert_eq!(
                    st_info >> 4,
                    STB_GNU_UNIQUE,
                    "an undefined reference stays GNU_UNIQUE (strong)"
                );
            }
            other => panic!("expected ELF symbol flags, got {other:?}"),
        }

        // The FILE bookkeeping symbol must stay in the local symtab
        // region: binutils 2.34 rejects a symtab whose local symbols
        // are not all ahead of sh_info ("local symbol at index N (>=
        // sh_info of N)"). Assert the ordering invariant directly: no
        // STB_LOCAL symbol may follow the first non-local one.
        let mut seen_nonlocal = false;
        for s in obj.symbols() {
            let local = matches!(
                s.flags(),
                object::SymbolFlags::Elf { st_info, .. } if st_info >> 4 == 0
            );
            if local {
                assert!(
                    !seen_nonlocal,
                    "local symbol '{}' follows a non-local one — the rewritten symtab is inconsistent (binutils 2.34 rejects it)",
                    s.name().unwrap_or("<unnamed>")
                );
            } else {
                seen_nonlocal = true;
            }
        }
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
            names
                .iter()
                .any(|n| n == "__tebako_internal__rust_eh_personality"),
            "the ref to the renamed def rides the prefix (raw name verbatim): {names:?}"
        );
        assert!(has("malloc"), "the libc ref stays: {names:?}");
        assert!(
            names.iter().any(|n| n == "__tebako_internal__consumer_fn"),
            "the member's own def is renamed (raw name verbatim): {names:?}"
        );
        assert_eq!(renamed, 2, "one def + one ref renamed");
    }

    /// The 0.16.3-era macos-x86_64 regression: blake3's x86-64 assembly
    /// defines every entry point TWICE in one object — `_name` (the
    /// Mach-O spelling) and `name` (the ELF spelling), same address.
    /// Scoping must keep the pair distinct; stripping the leading
    /// underscore before prefixing collapses them into one scoped name
    /// and ld64 errors "duplicate symbol" (11 blake3 symbols on the
    /// miniruby link). The fixture is a raw LC_SYMTAB blob, so the test
    /// runs on every host.
    #[test]
    fn macho_dual_spelling_definitions_stay_distinct() {
        // Minimal MH_OBJECT: header + one LC_SYMTAB + symtab + strtab.
        // macho.rs reads nothing else.
        let strtab: &[u8] = b"\0_dual\0dual\0_tebako_api\0";
        let strx = |name: &str| {
            strtab
                .windows(name.len())
                .position(|w| w == name.as_bytes())
                .unwrap() as u32
        };
        let nlist = |strx: u32, n_type: u8, value: u64| {
            let mut e = Vec::with_capacity(16);
            e.extend_from_slice(&strx.to_le_bytes());
            e.push(n_type);
            e.push(1); // n_sect
            e.extend_from_slice(&[0, 0]); // n_desc
            e.extend_from_slice(&value.to_le_bytes());
            e
        };
        const N_SECT_EXT: u8 = 0x0e | 0x01; // N_SECT | N_EXT — a global definition
        const N_UNDF_EXT: u8 = 0x01; // N_UNDF | N_EXT — an undefined reference
        let mut syms = Vec::new();
        syms.extend(nlist(strx("_dual"), N_SECT_EXT, 0x100)); // Mach-O spelling
        syms.extend(nlist(strx("dual"), N_SECT_EXT, 0x100)); // ELF spelling, same address
        syms.extend(nlist(strx("_tebako_api"), N_SECT_EXT, 0x200)); // kept public
        syms.extend(nlist(strx("_dual"), N_UNDF_EXT, 0)); // a reference to the def

        let symoff = 32 + 24;
        let stroff = symoff + syms.len();
        let mut obj = Vec::new();
        obj.extend_from_slice(&0xfeedfacfu32.to_le_bytes()); // MH_MAGIC_64
        obj.extend_from_slice(&[0; 12]); // cputype/cpusubtype/filetype
        obj.extend_from_slice(&1u32.to_le_bytes()); // ncmds
        obj.extend_from_slice(&24u32.to_le_bytes()); // sizeofcmds
        obj.extend_from_slice(&[0; 4]); // flags
        obj.extend_from_slice(&[0; 4]); // reserved
        obj.extend_from_slice(&2u32.to_le_bytes()); // LC_SYMTAB
        obj.extend_from_slice(&24u32.to_le_bytes()); // cmdsize
        obj.extend_from_slice(&(symoff as u32).to_le_bytes());
        obj.extend_from_slice(&4u32.to_le_bytes()); // nsyms
        obj.extend_from_slice(&(stroff as u32).to_le_bytes());
        obj.extend_from_slice(&(strtab.len() as u32).to_le_bytes());
        obj.extend_from_slice(&syms);
        obj.extend_from_slice(strtab);

        let defined = macho::defined(&obj, KEEP_PREFIX).expect("pass A");
        let (out, exported, renamed, kept) =
            macho::scope(&obj, KEEP_PREFIX, SCOPE_PREFIX, &defined).expect("scope the fixture");
        assert_eq!(renamed, 3, "both spellings + the reference ride the prefix");
        assert_eq!(kept, 1, "the tebako_* definition stays public");

        let (_, symoff, nsyms, stroff, _strsize) =
            macho::symtab_for_test(&out).expect("parse the scoped symtab");
        let names: Vec<String> = (0..nsyms)
            .map(|i| {
                let at = symoff + i * 16;
                let strx = u32::from_le_bytes(out[at..at + 4].try_into().unwrap()) as usize;
                let end = out[stroff + strx..].iter().position(|&b| b == 0).unwrap();
                String::from_utf8_lossy(&out[stroff + strx..stroff + strx + end]).into_owned()
            })
            .collect();
        assert!(
            names.iter().any(|n| n == "__tebako_internal__dual"),
            "the Mach-O spelling scopes verbatim: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "__tebako_internal_dual"),
            "the ELF spelling scopes verbatim — no collapse: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "_tebako_api"),
            "the public surface keeps its name: {names:?}"
        );
        assert_eq!(names.len(), 4, "no symbol is dropped or added: {names:?}");
        assert!(
            exported.iter().any(|n| n == "__tebako_internal__dual")
                && exported.iter().any(|n| n == "__tebako_internal_dual"),
            "both spellings land in the archive index: {exported:?}"
        );
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
            None,
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

    /// A minimal dlltool-style long-form import member: a COFF object
    /// whose import identity is its `.idata$N` section set.
    fn coff_import_fixture(sections: &[&str], symbol: &str) -> Vec<u8> {
        let mut out = object::write::Object::new(
            object::BinaryFormat::Coff,
            object::Architecture::X86_64,
            object::Endianness::Little,
        );
        let mut first = None;
        for name in sections {
            let id = out.add_section(
                Vec::new(),
                name.as_bytes().to_vec(),
                object::SectionKind::Data,
            );
            out.section_mut(id).set_data(vec![0; 8], 8);
            first = first.or(Some(id));
        }
        out.add_symbol(object::write::Symbol {
            name: symbol.as_bytes().to_vec(),
            value: 0,
            size: 8,
            kind: object::SymbolKind::Data,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(first.unwrap()),
            flags: object::SymbolFlags::None,
        });
        out.write().expect("import fixture object")
    }

    /// A plain COFF code member (no .idata sections) — must keep its
    /// name and keep riding the symbol scoping unchanged.
    fn coff_code_fixture() -> Vec<u8> {
        let mut out = object::write::Object::new(
            object::BinaryFormat::Coff,
            object::Architecture::X86_64,
            object::Endianness::Little,
        );
        let text = out.add_section(Vec::new(), b".text".to_vec(), object::SectionKind::Text);
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
        out.write().expect("code fixture object")
    }

    /// A COMDAT COFF member in the aarch64 rustc shape: a head section
    /// (.text$mn, Selection ANY) with an associative unwind section
    /// (.xdata$mn) and one exported function symbol in the head.
    fn coff_comdat_fixture() -> Vec<u8> {
        let mut out = object::write::Object::new(
            object::BinaryFormat::Coff,
            object::Architecture::Aarch64,
            object::Endianness::Little,
        );
        let text = out.add_section(Vec::new(), b".text$mn".to_vec(), object::SectionKind::Text);
        out.section_mut(text).set_data(b"\xc0\x03\x5f\xd6", 4);
        let xdata = out.add_section(
            Vec::new(),
            b".xdata$mn".to_vec(),
            object::SectionKind::ReadOnlyData,
        );
        out.section_mut(xdata).set_data(&[0u8; 8], 4);
        let head = out.section_symbol(text);
        out.section_symbol(xdata);
        out.add_symbol(object::write::Symbol {
            name: b"fold".to_vec(),
            value: 0,
            size: 4,
            kind: object::SymbolKind::Text,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(text),
            flags: object::SymbolFlags::None,
        });
        out.add_comdat(object::write::Comdat {
            kind: object::ComdatKind::Any,
            sections: vec![text, xdata],
            symbol: head,
        });
        out.write().expect("comdat fixture object")
    }

    /// The section-symbol aux record must keep its Selection byte across
    /// the rewrite: the writer regenerates section symbols with Selection
    /// 0 while the section header keeps IMAGE_SCN_LNK_COMDAT — ld.bfd
    /// ignores the byte, ld.lld refuses the link ("unknown comdat type
    /// 0", python factory run 35532416136; 3338 COMDAT sections in the
    /// shipped v2.8.13 aarch64 link unit).
    #[test]
    fn coff_comdat_selection_survives_the_rewrite() {
        let bytes = coff_comdat_fixture();
        verify_coff_comdat(&bytes).expect("the fixture is a valid COMDAT object");
        let mut report = Report::default();
        let (scoped, _exported) = scope_object(
            &bytes,
            KEEP_PREFIX,
            SCOPE_PREFIX,
            &std::collections::HashSet::new(),
            &mut report,
        )
        .expect("scope the comdat fixture");
        verify_coff_comdat(&scoped).expect("the rewrite keeps the COMDAT selections");
        let obj = File::parse(&scoped[..]).expect("parse the rewritten object");
        let head_selection = obj
            .symbols()
            .find(|s| {
                s.kind() == object::SymbolKind::Section
                    && s.name().map(|n| n == ".text$mn").unwrap_or(false)
            })
            .and_then(|s| match s.flags() {
                object::SymbolFlags::CoffSection { selection, .. } => Some(selection),
                _ => None,
            })
            .expect("the head section symbol carries an aux record");
        assert_eq!(head_selection, object::pe::IMAGE_COMDAT_SELECT_ANY);
        let (member_selection, member_head) = obj
            .symbols()
            .find(|s| {
                s.kind() == object::SymbolKind::Section
                    && s.name().map(|n| n == ".xdata$mn").unwrap_or(false)
            })
            .and_then(|s| match s.flags() {
                object::SymbolFlags::CoffSection {
                    selection,
                    associative_section,
                } => Some((selection, associative_section)),
                _ => None,
            })
            .expect("the member section symbol carries an aux record");
        assert_eq!(
            member_selection,
            object::pe::IMAGE_COMDAT_SELECT_ASSOCIATIVE
        );
        let head_index = obj
            .section_by_name_bytes(b".text$mn")
            .expect("the head section")
            .index();
        assert_eq!(member_head, Some(head_index));
        let names: Vec<String> = obj
            .symbols()
            .filter_map(|s: object::Symbol<'_, '_>| s.name().ok().map(str::to_string))
            .collect();
        assert!(
            names.iter().any(|n| n == "__tebako_internal_fold"),
            "the function symbol is renamed: {names:?}"
        );
    }

    fn coff_file_fixture(section: object::write::SymbolSection) -> Vec<u8> {
        let mut out = object::write::Object::new(
            object::BinaryFormat::Coff,
            object::Architecture::Aarch64,
            object::Endianness::Little,
        );
        let text = out.add_section(Vec::new(), b".text".to_vec(), object::SectionKind::Text);
        out.section_mut(text).set_data(b"\xc0\x03\x5f\xd6", 4);
        out.add_symbol(object::write::Symbol {
            name: b".file".to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::File,
            scope: object::SymbolScope::Compilation,
            weak: false,
            section,
            flags: object::SymbolFlags::None,
        });
        out.add_symbol(object::write::Symbol {
            name: b"fold".to_vec(),
            value: 0,
            size: 4,
            kind: object::SymbolKind::Text,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(text),
            flags: object::SymbolFlags::None,
        });
        out.write().expect("file fixture object")
    }

    /// rustc's COFF objects carry one .file symbol per TU at
    /// IMAGE_SYM_DEBUG (-2). The rewrite must keep it there: mapping it
    /// to section 0 makes ld.lld refuse the link (".file should not
    /// refer to special section 0", python factory run 35543897983 —
    /// 401 such symbols across the shipped v2.8.14 aarch64 link unit).
    #[test]
    fn coff_file_symbol_keeps_the_debug_section() {
        let bytes = coff_file_fixture(object::write::SymbolSection::None);
        verify_coff_file_records(&bytes).expect("the fixture is a valid .file object");
        let mut report = Report::default();
        let (scoped, _exported) = scope_object(
            &bytes,
            KEEP_PREFIX,
            SCOPE_PREFIX,
            &std::collections::HashSet::new(),
            &mut report,
        )
        .expect("scope the file fixture");
        verify_coff_file_records(&scoped).expect("the rewrite keeps the .file section");
        let obj = File::parse(&scoped[..]).expect("parse the rewritten object");
        let file_section = obj
            .symbols()
            .find(|s| s.kind() == object::SymbolKind::File)
            .map(|s| s.section())
            .expect("the .file symbol survives the rewrite");
        assert!(
            matches!(file_section, object::SymbolSection::None),
            "the .file symbol stays at IMAGE_SYM_DEBUG, got {file_section:?}"
        );
        let corrupt = coff_file_fixture(object::write::SymbolSection::Undefined);
        assert!(
            verify_coff_file_records(&corrupt).is_err(),
            "the gate rejects a .file symbol rewritten to section 0"
        );
    }

    fn archive_member_names(bytes: &[u8]) -> Vec<String> {
        let archive = object::read::archive::ArchiveFile::parse(bytes).expect("parse output");
        archive
            .members()
            .map(|m| String::from_utf8_lossy(m.expect("member").name()).into_owned())
            .filter(|n| n != "/" && n != "//")
            .collect()
    }

    /// rustc's staticlib bundler disambiguates duplicate import member
    /// names with a numeric "NNNNN_" prefix, and GNU ld's PE scripts sort
    /// .idata chunks by the member NAME — the prefix sorts the null
    /// terminator ahead of the entries and the DLL descriptor swallows
    /// the next DLL's functions (the v2.8.4 miniruby 0xC0000139 defect).
    /// The rewrite must restore the canonical `<dll>.dll[h|sNNNNN|t].o`
    /// names, collapse byte-identical duplicates, and leave the symbol
    /// scoping itself untouched.
    #[test]
    fn coff_import_members_get_sort_canonical_names() {
        let h = coff_import_fixture(&[".idata$2"], "_head_ncrypt");
        let s = coff_import_fixture(&[".idata$4", ".idata$5"], "__imp_NCryptFreeObject");
        let t = coff_import_fixture(&[".idata$4", ".idata$5", ".idata$7"], "ncrypt_thunks");
        let code = coff_code_fixture();

        let mut input = b"!<arch>\n".to_vec();
        write_member(&mut input, "00007_ncrypt.dllh.o", &h, false).expect("h member");
        write_member(&mut input, "00008_ncrypt.dlls00000.o", &s, false).expect("s member");
        // rustc bundles the same generated import lib through several
        // crate paths: after the prefix strip the duplicate collides by
        // name and must collapse (byte-identical).
        write_member(&mut input, "00009_ncrypt.dlls00000.o", &s, false).expect("dup s member");
        write_member(&mut input, "00010_ncrypt.dllt.o", &t, false).expect("t member");
        write_member(&mut input, "code.o", &code, false).expect("code member");

        let tmp = std::env::temp_dir().join(format!("arscope-import-{}.a", std::process::id()));
        let tmp_out =
            std::env::temp_dir().join(format!("arscope-import-out-{}.a", std::process::id()));
        std::fs::write(&tmp, &input).expect("write the input archive");
        let report = run(
            tmp.to_str().unwrap(),
            tmp_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            None,
        )
        .expect("scope the archive");
        let out = std::fs::read(&tmp_out).expect("read the scoped archive");
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp_out);

        assert_eq!(
            archive_member_names(&out),
            vec![
                "ncrypt.dllh.o",
                "ncrypt.dlls00000.o",
                "ncrypt.dllt.o",
                "code.o"
            ],
            "numeric prefixes stripped, the byte-identical dup collapsed, order preserved"
        );
        assert_eq!(report.imports, 4, "four import members seen (dup included)");

        // Symbol scoping applies to import members exactly as to code:
        // the defined import thunk rides the prefix (the prefixed
        // symbols + canonical names shape is link-proven clean).
        let archive = object::read::archive::ArchiveFile::parse(&out[..]).expect("reparse");
        let s_member = archive
            .members()
            .find_map(|m| {
                let m = m.expect("member");
                (String::from_utf8_lossy(m.name()) == "ncrypt.dlls00000.o")
                    .then(|| m.data(&out[..]).expect("member data"))
            })
            .expect("the s member");
        let obj = File::parse(s_member).expect("parse the s member");
        let names: Vec<String> = obj
            .symbols()
            .filter_map(|s| s.name().ok().map(str::to_string))
            .collect();
        assert!(
            names
                .iter()
                .any(|n| n == "__tebako_internal___imp_NCryptFreeObject"),
            "the import thunk definition is scoped like any other: {names:?}"
        );
    }

    /// Two import members that canonicalize to the same name but differ
    /// in content are two import sets for the SAME DLL (the dependency
    /// graph legitimately carries two windows-targets lines — the v2.8.5
    /// windows link-unit's bcryptprimitives.dllt.o twice). ld pulls
    /// members by symbol, never by name, and SORT_BY_NAME still groups
    /// both sets adjacently: both survive under the one canonical name;
    /// only byte-identical duplicates collapse.
    #[test]
    fn coff_import_member_same_name_different_sets_are_both_kept() {
        let s1 = coff_import_fixture(&[".idata$4", ".idata$5"], "__imp_NCryptFreeObject");
        // Same canonical name, different content (a disjoint thunk set).
        let s2 = coff_import_fixture(&[".idata$4", ".idata$5"], "__imp_NCryptOpenKey");

        let mut input = b"!<arch>\n".to_vec();
        write_member(&mut input, "00001_ncrypt.dlls00000.o", &s1, false).expect("s1");
        write_member(&mut input, "00002_ncrypt.dlls00000.o", &s2, false).expect("s2");
        write_member(&mut input, "00003_ncrypt.dlls00000.o", &s1, false).expect("dup of s1");

        let tmp = std::env::temp_dir().join(format!("arscope-dupsets-{}.a", std::process::id()));
        let tmp_out =
            std::env::temp_dir().join(format!("arscope-dupsets-out-{}.a", std::process::id()));
        std::fs::write(&tmp, &input).expect("write the input archive");
        let report = run(
            tmp.to_str().unwrap(),
            tmp_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            None,
        )
        .expect("two import sets for one DLL are kept, never an error");
        let out = std::fs::read(&tmp_out).expect("read the scoped archive");
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp_out);

        assert_eq!(
            archive_member_names(&out),
            vec!["ncrypt.dlls00000.o", "ncrypt.dlls00000.o"],
            "both sets survive under the canonical name; the byte-identical dup collapsed"
        );
        assert_eq!(
            report.imports, 3,
            "three import members seen (dup included)"
        );

        let archive = object::read::archive::ArchiveFile::parse(&out[..]).expect("reparse");
        let mut symbols: Vec<String> = archive
            .members()
            .filter_map(|m| {
                let m = m.expect("member");
                (String::from_utf8_lossy(m.name()) == "ncrypt.dlls00000.o")
                    .then(|| m.data(&out[..]).expect("member data"))
            })
            .flat_map(|data| {
                File::parse(data)
                    .expect("parse a surviving member")
                    .symbols()
                    .filter_map(|s| s.name().ok().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .collect();
        symbols.sort();
        assert_eq!(
            symbols,
            vec![
                "__tebako_internal___imp_NCryptFreeObject",
                "__tebako_internal___imp_NCryptOpenKey"
            ],
            "each surviving member carries its own scoped thunk symbol: {symbols:?}"
        );
    }

    /// `--dedupe-against` drops from the derived archive every member the
    /// (already scoped) base carries under the same final name with the
    /// same final bytes — cargo's staticlib bundling packs the base's
    /// closure into the derived archive and the pair would otherwise
    /// duplicate-define every shared member at link (Apple ld errors,
    /// GNU ld shrugs first-wins). Matching is (name, content), never name
    /// alone: a same-name import member with different bytes is a second
    /// import set and survives. The base is scoped FIRST (the production
    /// order) so raw byte-identical members meet post-rewrite.
    #[test]
    fn cross_dedupe_drops_base_carried_members_by_name_and_content() {
        let code = coff_code_fixture();
        let s1 = coff_import_fixture(&[".idata$4", ".idata$5"], "__imp_NCryptFreeObject");
        let s2 = coff_import_fixture(&[".idata$4", ".idata$5"], "__imp_NCryptOpenKey");

        // The base archive, scoped exactly as the staging tool does first.
        let mut base_raw = b"!<arch>\n".to_vec();
        write_member(&mut base_raw, "code.o", &code, false).expect("base code");
        write_member(&mut base_raw, "00001_ncrypt.dlls00000.o", &s1, false).expect("base s1");
        let pid = std::process::id();
        let base_in = std::env::temp_dir().join(format!("arscope-xded-base-{pid}.a"));
        let base_out = std::env::temp_dir().join(format!("arscope-xded-base-scoped-{pid}.a"));
        std::fs::write(&base_in, &base_raw).expect("write the raw base");
        run(
            base_in.to_str().unwrap(),
            base_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            None,
        )
        .expect("scope the base");

        // The derived archive: the base's code + import members byte for
        // byte (cargo's bundling), a same-canonical-name import member with
        // DIFFERENT bytes (a second set — must survive), and its own code.
        let mut derived_raw = b"!<arch>\n".to_vec();
        write_member(&mut derived_raw, "code.o", &code, false).expect("derived code");
        write_member(&mut derived_raw, "00007_ncrypt.dlls00000.o", &s1, false).expect("derived s1");
        write_member(&mut derived_raw, "00008_ncrypt.dlls00000.o", &s2, false).expect("derived s2");
        write_member(&mut derived_raw, "unique.o", &code, false).expect("derived unique");
        let der_in = std::env::temp_dir().join(format!("arscope-xded-der-{pid}.a"));
        let der_out = std::env::temp_dir().join(format!("arscope-xded-der-scoped-{pid}.a"));
        std::fs::write(&der_in, &derived_raw).expect("write the raw derived");
        let report = run(
            der_in.to_str().unwrap(),
            der_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            Some(base_out.to_str().unwrap()),
        )
        .expect("scope + cross-dedupe the derived archive");
        let out = std::fs::read(&der_out).expect("read the result");
        for f in [&base_in, &base_out, &der_in, &der_out] {
            let _ = std::fs::remove_file(f);
        }

        assert_eq!(
            report.deduped, 2,
            "code.o and the byte-identical s1 dropped"
        );
        assert_eq!(
            archive_member_names(&out),
            vec!["ncrypt.dlls00000.o", "unique.o"],
            "the different-bytes import set survives under the shared canonical name"
        );

        // The surviving import member is s2's (scoped), not s1's.
        let archive = object::read::archive::ArchiveFile::parse(&out[..]).expect("reparse");
        let member = archive
            .members()
            .find_map(|m| {
                let m = m.expect("member");
                (String::from_utf8_lossy(m.name()) == "ncrypt.dlls00000.o")
                    .then(|| m.data(&out[..]).expect("member data"))
            })
            .expect("the surviving import member");
        let names: Vec<String> = File::parse(member)
            .expect("parse the survivor")
            .symbols()
            .filter_map(|s| s.name().ok().map(str::to_string))
            .collect();
        assert!(
            names
                .iter()
                .any(|n| n == "__tebako_internal___imp_NCryptOpenKey"),
            "the second set's thunk survives scoped: {names:?}"
        );
    }

    /// The gnullvm raw-dylib descriptor shape (the libtfs.a
    /// bcryptprimitives.dll member of run 35512778802): `.idata$2`'s
    /// OriginalFirstThunk/FirstThunk fields relocate against UNDEFINED
    /// section symbols naming the thunk member's `.idata$4`/`.idata$5`
    /// chunks (IMAGE_SYM_CLASS_SECTION with section number 0 — a
    /// cross-member section reference). The rewrite must carry them;
    /// dropping them stranded the relocations ("references a dropped
    /// bookkeeping symbol").
    #[test]
    fn coff_import_descriptor_undefined_section_symbols_roundtrip() {
        let mut out = object::write::Object::new(
            object::BinaryFormat::Coff,
            object::Architecture::X86_64,
            object::Endianness::Little,
        );
        let descriptor =
            out.add_section(Vec::new(), b".idata$2".to_vec(), object::SectionKind::Data);
        out.section_mut(descriptor).set_data(vec![0; 20], 4);
        let dll_name_section =
            out.add_section(Vec::new(), b".idata$6".to_vec(), object::SectionKind::Data);
        out.section_mut(dll_name_section)
            .set_data(b"bcryptprimitives.dll\0".to_vec(), 1);
        out.add_symbol(object::write::Symbol {
            name: b"__IMPORT_DESCRIPTOR_bcryptprimitives".to_vec(),
            value: 0,
            size: 20,
            kind: object::SymbolKind::Data,
            scope: object::SymbolScope::Linkage,
            weak: false,
            section: object::write::SymbolSection::Section(descriptor),
            flags: object::SymbolFlags::None,
        });
        // The undefined section symbols: the writer's public API cannot
        // emit class SECTION without a section id, so the fixture builds
        // them as undefined data symbols and marks them — exactly the
        // bytes rustc's gnullvm bundler emits (class 104, section 0).
        let undefined_section_symbol = |name: &[u8]| object::write::Symbol {
            name: name.to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::Data,
            scope: object::SymbolScope::Compilation,
            weak: false,
            section: object::write::SymbolSection::Undefined,
            flags: object::SymbolFlags::None,
        };
        let idata4 = out.add_symbol(undefined_section_symbol(b".idata$4"));
        let idata5 = out.add_symbol(undefined_section_symbol(b".idata$5"));
        let dll_name = out.add_symbol(object::write::Symbol {
            name: b".idata$6".to_vec(),
            value: 0,
            size: 0,
            kind: object::SymbolKind::Data,
            scope: object::SymbolScope::Compilation,
            weak: false,
            section: object::write::SymbolSection::Section(dll_name_section),
            flags: object::SymbolFlags::None,
        });
        // IMAGE_REL_AMD64_ADDR32NB: the descriptor's three pointer fields.
        for (offset, symbol) in [(0u64, idata4), (0xc, dll_name), (0x10, idata5)] {
            out.add_relocation(
                descriptor,
                object::write::Relocation {
                    offset,
                    symbol,
                    addend: 0,
                    flags: object::RelocationFlags::Coff { typ: 3 },
                },
            )
            .expect("descriptor relocation");
        }
        let mut bytes = out.write().expect("descriptor fixture object");
        coff_mark_undefined_section_symbols(&mut bytes, &[".idata$4", ".idata$5"])
            .expect("the fixture carries the rustc/gnullvm descriptor shape");

        let mut report = Report::default();
        let defined = std::collections::HashSet::new();
        let (rewritten, _exported) =
            scope_object(&bytes, KEEP_PREFIX, SCOPE_PREFIX, &defined, &mut report)
                .expect("a descriptor member's undefined section symbols survive the rewrite");

        let obj = File::parse(&rewritten[..]).expect("parse the rewritten descriptor");
        let section = obj
            .sections()
            .find(|s| s.name_bytes().map(|n| n == b".idata$2").unwrap_or(false))
            .expect(".idata$2 survives");
        let targets: Vec<(u64, String)> = section
            .relocations()
            .map(|(offset, r)| match r.target() {
                RelocationTarget::Symbol(i) => {
                    let sym = obj.symbol_by_index(i).expect("reloc target symbol");
                    (offset, sym.name().expect("reloc target name").to_string())
                }
                other => panic!("expected a symbol reloc target, got {other:?}"),
            })
            .collect();
        assert_eq!(
            targets,
            vec![
                (0, ".idata$4".to_string()),
                (0xc, ".idata$6".to_string()),
                (0x10, ".idata$5".to_string()),
            ],
            "every descriptor relocation still points at its named chunk"
        );
        // The carried symbols keep the input's shape: undefined section
        // symbols, which the reader reports as Section-kind +
        // SymbolSection::Undefined (its is_undefined predicate is
        // EXTERNAL-class-only and deliberately stays false here).
        for wanted in [".idata$4", ".idata$5"] {
            let sym = obj
                .symbols()
                .find(|s| s.name() == Ok(wanted))
                .unwrap_or_else(|| panic!("{wanted} carried"));
            assert_eq!(sym.kind(), object::SymbolKind::Section, "{wanted} kind");
            assert!(
                matches!(sym.section(), object::SymbolSection::Undefined),
                "{wanted} stays a cross-member (undefined) section reference"
            );
        }
    }

    /// A short-form import object (the PE/COFF IMPORT_OBJECT_HEADER form
    /// — sig1 0, sig2 0xFFFF, the names in-band) is NOT a COFF object:
    /// rustc's gnullvm raw-dylib bundler ships one per imported function
    /// next to the long-form descriptor/thunk members (the 53-byte
    /// ProcessPrng member of run 35512778802's libtfs.a). It has no
    /// sections or relocations and its symbols name a system DLL's
    /// exports, so the rewrite passes it through byte-identical — but
    /// the archive index must still carry the names it defines, or ld
    /// can never reach the member.
    #[test]
    fn short_form_import_member_passes_through_byte_identical() {
        let mut short = Vec::new();
        short.extend_from_slice(&[0x00, 0x00, 0xff, 0xff]); // sig1, sig2
        short.extend_from_slice(&0u16.to_le_bytes()); // version
        short.extend_from_slice(&0x8664u16.to_le_bytes()); // machine AMD64
        short.extend_from_slice(&0u32.to_le_bytes()); // timestamp
        short.extend_from_slice(&33u32.to_le_bytes()); // string tail size
        short.extend_from_slice(&0u16.to_le_bytes()); // ordinal/hint
        short.extend_from_slice(&4u16.to_le_bytes()); // import type CODE, name type NAME
        short.extend_from_slice(b"ProcessPrng\0");
        short.extend_from_slice(b"bcryptprimitives.dll\0");
        assert_eq!(short.len(), 53);

        let mut input = b"!<arch>\n".to_vec();
        write_member(&mut input, "bcryptprimitives.dll", &short, false).expect("short member");
        let pid = std::process::id();
        let tmp = std::env::temp_dir().join(format!("arscope-shortform-{pid}.a"));
        let tmp_out = std::env::temp_dir().join(format!("arscope-shortform-out-{pid}.a"));
        std::fs::write(&tmp, &input).expect("write the input archive");
        let report = run(
            tmp.to_str().unwrap(),
            tmp_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            None,
        )
        .expect("a short-form import member passes through");
        let out = std::fs::read(&tmp_out).expect("read the scoped archive");
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp_out);

        assert_eq!(archive_member_names(&out), vec!["bcryptprimitives.dll"]);
        assert_eq!(report.imports, 1, "counted as an import member");

        // The member bytes are verbatim AND exact-length: lld's
        // ImportFile parse rejects a member longer than
        // 20 + SizeOfData ("broken import library"), so no alignment
        // slack may ride along (the ld64 4-byte pad is Mach-O-only).
        let archive = object::read::archive::ArchiveFile::parse(&out[..]).expect("reparse");
        let data = archive
            .members()
            .find_map(|m| {
                let m = m.expect("member");
                (String::from_utf8_lossy(m.name()) == "bcryptprimitives.dll")
                    .then(|| m.data(&out[..]).expect("member data"))
            })
            .expect("the short-form member");
        assert_eq!(data, &short[..], "verbatim bytes, exact length");

        // Its defined names ride the archive index — ld reaches the
        // member by symbol, never by name. (The reader consumes the "/"
        // member as the index, so members() never yields it; the symbol
        // iterator reads the parsed index directly.)
        let indexed: Vec<Vec<u8>> = archive
            .symbols()
            .expect("read the archive index")
            .expect("the archive has an index")
            .map(|s| s.expect("index entry").name().to_vec())
            .collect();
        assert!(
            indexed.iter().any(|n| n == b"ProcessPrng"),
            "the thunk name is indexed: {indexed:?}"
        );
        assert!(
            indexed.iter().any(|n| n == b"__imp_ProcessPrng"),
            "the IAT entry name is indexed: {indexed:?}"
        );
    }

    /// The v2.8.14/v2.8.15 regression shape: a short-form import member
    /// carrying alignment slack is refused by name — lld's ImportFile
    /// parse is exact-size ("broken import library", python factory run
    /// 35560837935, ruby factory run 35560840341).
    #[test]
    fn short_form_import_member_with_alignment_slack_is_refused() {
        let mut short = Vec::new();
        short.extend_from_slice(&[0x00, 0x00, 0xff, 0xff]); // sig1, sig2
        short.extend_from_slice(&0u16.to_le_bytes()); // version
        short.extend_from_slice(&0x8664u16.to_le_bytes()); // machine AMD64
        short.extend_from_slice(&0u32.to_le_bytes()); // timestamp
        short.extend_from_slice(&33u32.to_le_bytes()); // string tail size
        short.extend_from_slice(&0u16.to_le_bytes()); // ordinal/hint
        short.extend_from_slice(&4u16.to_le_bytes()); // import type CODE, name type NAME
        short.extend_from_slice(b"ProcessPrng\0");
        short.extend_from_slice(b"bcryptprimitives.dll\0");
        assert_eq!(short.len(), 53);
        short.extend_from_slice(&[0, 0, 0]); // the v2.8.14 alignment slack

        let mut input = b"!<arch>\n".to_vec();
        write_member(&mut input, "bcryptprimitives.dll", &short, false).expect("short member");
        let pid = std::process::id();
        let tmp = std::env::temp_dir().join(format!("arscope-shortslack-{pid}.a"));
        let tmp_out = std::env::temp_dir().join(format!("arscope-shortslack-out-{pid}.a"));
        std::fs::write(&tmp, &input).expect("write the input archive");
        let err = run(
            tmp.to_str().unwrap(),
            tmp_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            None,
        )
        .expect_err("a padded short-form import member is refused");
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp_out);
        assert!(
            err.contains("20 + SizeOfData"),
            "the exact-size gate names itself: {err}"
        );
    }

    /// ld64's 4-byte member alignment pad is MACH-O's, and it survives
    /// the gate: the same pad that must never touch a COFF short-form
    /// import member still applies to a Mach-O archive, or ld64's
    /// archive walk desyncs ("archive member invalid control bits").
    #[test]
    fn macho_members_keep_the_ld64_alignment_pad() {
        // Minimal MH_OBJECT: header + one LC_SYMTAB + symtab + strtab,
        // with string lengths that push the scoped output off a 4-byte
        // boundary so the pad is load-bearing.
        let strtab: &[u8] = b"\0_tebako_api\0_tebako_k\0";
        let strx = |name: &str| {
            strtab
                .windows(name.len())
                .position(|w| w == name.as_bytes())
                .unwrap() as u32
        };
        let nlist = |strx: u32, n_type: u8, value: u64| {
            let mut e = Vec::with_capacity(16);
            e.extend_from_slice(&strx.to_le_bytes());
            e.push(n_type);
            e.push(1); // n_sect
            e.extend_from_slice(&[0, 0]); // n_desc
            e.extend_from_slice(&value.to_le_bytes());
            e
        };
        const N_SECT_EXT: u8 = 0x0e | 0x01; // N_SECT | N_EXT — a global definition
        let mut syms = Vec::new();
        syms.extend(nlist(strx("_tebako_api"), N_SECT_EXT, 0x100));
        syms.extend(nlist(strx("_tebako_k"), N_SECT_EXT, 0x200));

        let symoff = 32 + 24;
        let stroff = symoff + syms.len();
        let mut obj = Vec::new();
        obj.extend_from_slice(&0xfeedfacfu32.to_le_bytes()); // MH_MAGIC_64
        obj.extend_from_slice(&[0; 12]); // cputype/cpusubtype/filetype
        obj.extend_from_slice(&1u32.to_le_bytes()); // ncmds
        obj.extend_from_slice(&24u32.to_le_bytes()); // sizeofcmds
        obj.extend_from_slice(&[0; 4]); // flags
        obj.extend_from_slice(&[0; 4]); // reserved
        obj.extend_from_slice(&2u32.to_le_bytes()); // LC_SYMTAB
        obj.extend_from_slice(&24u32.to_le_bytes()); // cmdsize
        obj.extend_from_slice(&(symoff as u32).to_le_bytes());
        obj.extend_from_slice(&2u32.to_le_bytes()); // nsyms
        obj.extend_from_slice(&(stroff as u32).to_le_bytes());
        obj.extend_from_slice(&(strtab.len() as u32).to_le_bytes());
        obj.extend_from_slice(&syms);
        obj.extend_from_slice(strtab);

        let defined = macho::defined(&obj, KEEP_PREFIX).expect("pass A");
        let (scoped, _, _, _) =
            macho::scope(&obj, KEEP_PREFIX, SCOPE_PREFIX, &defined).expect("scope the fixture");
        let padded_len = scoped.len().div_ceil(4) * 4;
        assert!(
            padded_len > scoped.len(),
            "the fixture must exercise the pad (scoped {} bytes) — tune the fixture",
            scoped.len()
        );

        let mut input = b"!<arch>\n".to_vec();
        write_member(&mut input, "macho.o", &obj, true).expect("macho member");
        let pid = std::process::id();
        let tmp = std::env::temp_dir().join(format!("arscope-machopad-{pid}.a"));
        let tmp_out = std::env::temp_dir().join(format!("arscope-machopad-out-{pid}.a"));
        std::fs::write(&tmp, &input).expect("write the input archive");
        run(
            tmp.to_str().unwrap(),
            tmp_out.to_str().unwrap(),
            KEEP_PREFIX,
            SCOPE_PREFIX,
            None,
        )
        .expect("scope the Mach-O archive");
        let out = std::fs::read(&tmp_out).expect("read the scoped archive");
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp_out);

        let archive = object::read::archive::ArchiveFile::parse(&out[..]).expect("reparse");
        let data = archive
            .members()
            .find_map(|m| {
                let m = m.expect("member");
                (String::from_utf8_lossy(m.name()) == "macho.o")
                    .then(|| m.data(&out[..]).expect("member data"))
            })
            .expect("the Mach-O member");
        assert_eq!(data.len(), padded_len, "the ld64 pad rides the member");
        assert_eq!(&data[..scoped.len()], &scoped[..], "scoped bytes verbatim");
        assert!(data[scoped.len()..].iter().all(|&b| b == 0), "zero slack");
    }
}
