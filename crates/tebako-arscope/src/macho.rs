//! Mach-O symbol scoping: raw LC_SYMTAB surgery.
//!
//! Renaming symbols never requires rebuilding an object: the nlist
//! symbol table and the string table are self-contained, and the string
//! table sits at EOF in every Mach-O object the Rust toolchain emits.
//! So the seal is a byte-level patch — read `LC_SYMTAB`, rename the
//! defined external non-`tebako_*` symbols, append the new names to the
//! string table, bump `strsize`. Sections, relocations, unwind info,
//! and every other byte stay exactly as rustc wrote them. This is why
//! the general object-rewrite path was wrong: it re-laid the sections
//! and broke ld64's fixed-size atomizers.

/// Mach-O constants (64-bit, little-endian — arm64/x86_64).
const MH_MAGIC_64: u32 = 0xfeedfacf;
const LC_SYMTAB: u32 = 0x2;
const N_EXT: u8 = 0x01;
const N_PEXT: u8 = 0x10;
const N_TYPE: u8 = 0x0e;
const N_UNDF: u8 = 0x00;

fn le_u32(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| format!("truncated Mach-O at {at:#x}"))
}

fn put_u32(bytes: &mut [u8], at: usize, v: u32) {
    bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// Locate LC_SYMTAB (exactly one in an MH_OBJECT).
fn symtab(bytes: &[u8]) -> Result<(usize, usize, usize, usize, usize), String> {
    if bytes.len() < 32 || le_u32(bytes, 0)? != MH_MAGIC_64 {
        return Err("not a 64-bit Mach-O object".to_string());
    }
    let ncmds = le_u32(bytes, 16)? as usize;
    let mut lc = None;
    let mut at = 32;
    for _ in 0..ncmds {
        let cmd = le_u32(bytes, at)?;
        let cmdsize = le_u32(bytes, at + 4)? as usize;
        if cmdsize < 8 {
            return Err(format!("malformed load command at {at:#x}"));
        }
        if cmd == LC_SYMTAB {
            lc = Some(at);
            break;
        }
        at += cmdsize;
    }
    let lc = lc.ok_or("no LC_SYMTAB in this Mach-O object")?;
    let symoff = le_u32(bytes, lc + 8)? as usize;
    let nsyms = le_u32(bytes, lc + 12)? as usize;
    let stroff = le_u32(bytes, lc + 16)? as usize;
    let strsize = le_u32(bytes, lc + 20)? as usize;
    if symoff + nsyms * 16 > bytes.len() || stroff + strsize > bytes.len() {
        return Err("LC_SYMTAB ranges out of bounds".to_string());
    }
    Ok((lc, symoff, nsyms, stroff, strsize))
}

/// Test-only view of the private symtab walker (the dual-spelling
/// regression test parses the scoped object back).
#[cfg(test)]
pub fn symtab_for_test(bytes: &[u8]) -> Result<(usize, usize, usize, usize, usize), String> {
    symtab(bytes)
}

/// One nlist entry, decoded.
struct Sym {
    at: usize,
    name_at: usize,
    name_len: usize,
    n_type: u8,
    common: bool,
}

impl Sym {
    /// A real definition: a section/absolute symbol, or a common
    /// (N_UNDF with a nonzero n_value — the tentative definition).
    fn is_def(&self) -> bool {
        (self.n_type & N_TYPE) != N_UNDF || self.common
    }
    /// Visible outside its own object: N_EXT globals AND N_PEXT private
    /// externs. Private externs are not exported from the final image,
    /// but two archives defining the same N_PEXT name still collide in
    /// one link, so they ride the rename exactly like globals.
    fn is_visible(&self) -> bool {
        (self.n_type & (N_EXT | N_PEXT)) != 0
    }
}

fn symbols(
    bytes: &[u8],
    symoff: usize,
    nsyms: usize,
    stroff: usize,
    strsize: usize,
) -> Result<Vec<Sym>, String> {
    let mut out = Vec::with_capacity(nsyms);
    for i in 0..nsyms {
        let at = symoff + i * 16;
        let strx = le_u32(bytes, at)? as usize;
        let n_type = bytes[at + 4];
        if strx >= strsize {
            return Err(format!("symbol {i}: strx out of bounds"));
        }
        let name_end = bytes[stroff + strx..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| format!("symbol {i}: unterminated name"))?;
        let common = (n_type & N_TYPE) == N_UNDF && bytes[at + 8..at + 16].iter().any(|&b| b != 0);
        out.push(Sym {
            at,
            name_at: stroff + strx,
            name_len: name_end,
            n_type,
            common,
        });
    }
    Ok(out)
}

/// The single rename rule both passes share: a definition is renamed
/// when it is visible outside its object and not under the keep
/// prefix. References are renamed exactly when their name is in the
/// set this rule collects across the whole archive.
fn renames_def(name: &[u8], keep: &str) -> bool {
    let logical = std::str::from_utf8(name).unwrap_or("");
    !logical.trim_start_matches('_').starts_with(keep)
}

/// The scoped spelling of a raw nlist name. The raw name rides the
/// prefix VERBATIM — the Mach-O leading underscore is NOT stripped.
/// blake3's x86-64 assembly legitimately defines every entry point
/// twice in one object (`_blake3_hash_many_sse2` AND
/// `blake3_hash_many_sse2`, the Mach-O and ELF spellings, same
/// address); stripping the underscore would collapse the pair into one
/// scoped name and ld64 errors "duplicate symbol" on the synthetic
/// collision (the 0.16.3-era macos-x86_64 miniruby link, 11 symbols).
/// Keeping the underscore keeps the spellings distinct:
/// `__tebako_internal__blake3_*` vs `__tebako_internal_blake3_*`.
fn scoped_name(name: &[u8], prefix: &str) -> String {
    format!("{prefix}{}", String::from_utf8_lossy(name))
}

/// Pass A: the raw strtab names of every definition this tool will
/// rename anywhere in the archive. References compare against these
/// raw names (Mach-O leading underscore included).
pub fn defined(bytes: &[u8], keep: &str) -> Result<std::collections::HashSet<String>, String> {
    let (_, symoff, nsyms, stroff, strsize) = symtab(bytes)?;
    let mut out = std::collections::HashSet::new();
    for sym in symbols(bytes, symoff, nsyms, stroff, strsize)? {
        let name = &bytes[sym.name_at..sym.name_at + sym.name_len];
        if sym.is_def() && sym.is_visible() && renames_def(name, keep) {
            out.insert(String::from_utf8_lossy(name).into_owned());
        }
    }
    Ok(out)
}

/// Scope one Mach-O object: rename defined visible symbols not under
/// `keep` — and every undefined reference whose target is renamed
/// elsewhere in the archive — appending the new names to the string
/// table. Relocations reference symbols by nlist index, so renaming
/// the nlist name remaps every reference site for free. Returns the
/// patched object and the archive-index names of every DEFINED VISIBLE
/// symbol (renamed or kept — the archive's consumers resolve through
/// these).
pub fn scope(
    bytes: &[u8],
    keep: &str,
    prefix: &str,
    defined: &std::collections::HashSet<String>,
) -> Result<(Vec<u8>, Vec<String>, usize, usize), String> {
    let (lc, symoff, nsyms, stroff, strsize) = symtab(bytes)?;

    let mut out = bytes.to_vec();
    // The new string table: the old one RELOCATED to EOF (it is not
    // always the last region — rnp's prebuilt objects carry linkedit
    // content after it), with the renamed names appended. Renamed
    // symbols point into it by their new strx.
    let mut new_strtab = bytes[stroff..stroff + strsize].to_vec();
    let mut exported: Vec<String> = Vec::new();
    let mut renamed = 0usize;
    let mut kept = 0usize;
    for sym in symbols(bytes, symoff, nsyms, stroff, strsize)? {
        let name = &bytes[sym.name_at..sym.name_at + sym.name_len];
        let rename = if sym.is_def() {
            if !sym.is_visible() {
                continue;
            }
            if renames_def(name, keep) {
                exported.push(scoped_name(name, prefix));
                true
            } else {
                exported.push(String::from_utf8_lossy(name).into_owned());
                kept += 1;
                false
            }
        } else {
            // A renamed definition is worthless to a reference that
            // kept the old name: refs to archive-renamed symbols ride
            // the prefix; refs to the outside world (libc, kept tebako_*
            // definitions) stay.
            defined.contains(&*String::from_utf8_lossy(name))
        };
        if !rename {
            continue;
        }
        renamed += 1;
        let new_name = scoped_name(name, prefix);
        let new_strx = new_strtab.len() as u32;
        new_strtab.extend_from_slice(new_name.as_bytes());
        new_strtab.push(0);
        put_u32(&mut out, sym.at, new_strx);
    }
    if renamed > 0 {
        let new_stroff = out.len() as u32;
        out.extend_from_slice(&new_strtab);
        put_u32(&mut out, lc + 16, new_stroff);
        put_u32(&mut out, lc + 20, new_strtab.len() as u32);
    }
    Ok((out, exported, renamed, kept))
}
