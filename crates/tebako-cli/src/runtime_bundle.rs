//! The release bundle's in-process unpack + verification (spec 36 §2/§4
//! — Rust `tar` + `flate2`, no shell-outs, invariant 1). The size-gated
//! tebako-bootstrap cannot link this crate and carries its own copy of
//! these semantics; the two are pinned by the same spec section:
//!
//! - the member grammar (§2): regular files only — no directories,
//!   symlinks, or device nodes — named by bare relative basenames (no
//!   absolute paths, no `..` segments);
//! - the member SET is exactly the shard's declared members plus the
//!   closing `SHA256SUMS` (the LAST member — anything after it violates
//!   the grammar); never a skipped member, never an extra one;
//! - every member's streamed sha256 against the shard's per-member pin,
//!   and the in-bundle `SHA256SUMS` agreeing with the member bytes;
//! - every disagreement is the named `InvalidBundle` (spec 36 §7 — exit
//!   70's integrity class; the gem-table row 121 is the CLI's mapping of
//!   that class), never a best-effort unpack, never a partial store
//!   write (the caller holds the entry lock and renames only verified
//!   bytes).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::Digest;

use crate::error::{packaging_error, TebakoError};

/// One member the shard declares the bundle carries: the served
/// basename plus the per-member pin (the shard's sha fields — spec 36
/// §3 — which pin the UNPACKED bytes).
#[derive(Debug, Clone)]
pub struct ExpectedMember {
    pub name: String,
    pub sha256: String,
}

/// A verified bundle member, staged in the store's tmp dir under a
/// fetch-unique spelling (the caller renames it into the entry — the
/// per-file install helpers take it from there).
#[derive(Debug)]
pub struct UnpackedMember {
    pub name: String,
    pub path: PathBuf,
    pub sha256: String,
}

/// The named spec 36 §7 error (exit 70's integrity class — row 121 in
/// the gem's PACKAGING_ERRORS table, the CLI's mapping of that class):
/// the message names the member and the shard it disagrees with.
pub fn invalid_bundle(bundle: &str, member: &str, detail: &str) -> TebakoError {
    packaging_error(
        121,
        Some(&format!(
            "invalid bundle {bundle} (member {member}): {detail} — the bundle disagrees with the shard that declared it; nothing was installed"
        )),
    )
}

/// The `SHA256SUMS` member is read into memory (it is the one member no
/// shard pin covers) — bound it: one coreutils line per member is
/// ~70 bytes; a megabyte is orders of magnitude past any real bundle.
const SUMS_MEMBER_CAP: u64 = 1 << 20;

/// Unpack + verify one fetched bundle (spec 36 §4). Verified members
/// land in `scratch` (the store's tmp dir — the same filesystem as the
/// entry the caller renames into) and come back in `members` order. On
/// ANY failure every staged byte is removed and nothing else was
/// touched.
pub fn unpack_bundle(
    bundle_path: &Path,
    bundle_name: &str,
    members: &[ExpectedMember],
    scratch: &Path,
) -> Result<Vec<UnpackedMember>, TebakoError> {
    let result = unpack_bundle_inner(bundle_path, bundle_name, members, scratch);
    if result.is_err() {
        for path in &staged_files(scratch, bundle_name) {
            let _ = fs::remove_file(path);
        }
    }
    result
}

/// The staged spellings this unpack may have written (member staging is
/// prefixed by the bundle name + pid, so a sweep never touches another
/// fetch's files).
fn staged_files(scratch: &Path, bundle_name: &str) -> Vec<PathBuf> {
    let prefix = format!("{bundle_name}.{}.", std::process::id());
    let Ok(children) = fs::read_dir(scratch) else {
        return Vec::new();
    };
    children
        .filter_map(|c| c.ok())
        .map(|c| c.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect()
}

fn unpack_bundle_inner(
    bundle_path: &Path,
    bundle_name: &str,
    members: &[ExpectedMember],
    scratch: &Path,
) -> Result<Vec<UnpackedMember>, TebakoError> {
    let file = fs::File::open(bundle_path).map_err(|e| {
        crate::error::plain_error(format!("{e} reading {}", bundle_path.display()))
    })?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    let undecodable = |member: &str| {
        invalid_bundle(bundle_name, member, "the tar/gzip stream is undecodable")
    };
    let mut unpacked: Vec<UnpackedMember> = Vec::new();
    let mut sums_seen = false;
    let entries = archive
        .entries()
        .map_err(|_| undecodable(bundle_name))?;
    for entry in entries {
        let mut entry = entry.map_err(|_| undecodable(bundle_name))?;
        let path = entry.path().map_err(|_| undecodable(bundle_name))?.into_owned();
        // The §2 grammar: a bare relative basename — nothing else.
        let mut comps = path.components();
        let name = match (comps.next(), comps.next()) {
            (Some(std::path::Component::Normal(n)), None) => n.to_string_lossy().into_owned(),
            _ => {
                return Err(invalid_bundle(
                    bundle_name,
                    &path.to_string_lossy(),
                    "member names must be bare relative file names (no directories, no absolute paths, no `..` segments)",
                ));
            }
        };
        if sums_seen {
            return Err(invalid_bundle(
                bundle_name,
                &name,
                "SHA256SUMS is the closing member — no member may follow it",
            ));
        }
        if entry.header().entry_type() != tar::EntryType::Regular {
            return Err(invalid_bundle(
                bundle_name,
                &name,
                "bundle members are regular files only (no directories, symlinks, or device nodes)",
            ));
        }
        if name == "SHA256SUMS" {
            let mut text = String::new();
            entry
                .take(SUMS_MEMBER_CAP + 1)
                .read_to_string(&mut text)
                .map_err(|_| undecodable("SHA256SUMS"))?;
            if text.len() as u64 > SUMS_MEMBER_CAP {
                return Err(invalid_bundle(
                    bundle_name,
                    "SHA256SUMS",
                    "the checksum member exceeds its size bound",
                ));
            }
            sums_seen = true;
            check_sums(bundle_name, &text, &unpacked)?;
            continue;
        }
        let Some(expected) = members.iter().find(|m| m.name == name) else {
            return Err(invalid_bundle(
                bundle_name,
                &name,
                "the shard does not declare this member",
            ));
        };
        let stage = scratch.join(format!(
            "{bundle_name}.{}.{name}",
            std::process::id()
        ));
        let mut out = fs::File::create(&stage).map_err(|e| {
            crate::error::plain_error(format!("{e} staging {}", stage.display()))
        })?;
        let mut hasher = sha2::Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = entry.read(&mut buf).map_err(|_| undecodable(&name))?;
            if n == 0 {
                break;
            }
            std::io::Write::write_all(&mut out, &buf[..n]).map_err(|e| {
                crate::error::plain_error(format!("{e} staging {}", stage.display()))
            })?;
            hasher.update(&buf[..n]);
        }
        let sha256 = crate::resolve::hex_lower(&hasher.finalize());
        if sha256 != expected.sha256 {
            return Err(invalid_bundle(
                bundle_name,
                &name,
                &format!(
                    "member checksum mismatch — the shard pins {}, the bundle carries {sha256}",
                    expected.sha256
                ),
            ));
        }
        unpacked.push(UnpackedMember {
            name,
            path: stage,
            sha256,
        });
    }
    if !sums_seen {
        return Err(invalid_bundle(
            bundle_name,
            "SHA256SUMS",
            "the closing SHA256SUMS member is absent",
        ));
    }
    // The exact member SET: every shard-declared member must have
    // appeared (extras were refused inline above).
    for expected in members {
        if !unpacked.iter().any(|m| m.name == expected.name) {
            return Err(invalid_bundle(
                bundle_name,
                &expected.name,
                "the shard declares this member but the bundle does not carry it",
            ));
        }
    }
    Ok(members
        .iter()
        .filter_map(|m| {
            unpacked
                .iter()
                .find(|u| u.name == m.name)
                .map(|u| UnpackedMember {
                    name: u.name.clone(),
                    path: u.path.clone(),
                    sha256: u.sha256.clone(),
                })
        })
        .collect())
}

/// The in-bundle `SHA256SUMS` (coreutils `<sha>  <file>` lines, a `*`
/// prefix tolerated) must cover exactly the already-verified members
/// and agree with their bytes (spec 36 §4's cross-check).
fn check_sums(
    bundle_name: &str,
    text: &str,
    unpacked: &[UnpackedMember],
) -> Result<(), TebakoError> {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches(['\r', ' ', '\t']);
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let (Some(sha), Some(file)) = (parts.next(), parts.next()) else {
            return Err(invalid_bundle(
                bundle_name,
                "SHA256SUMS",
                "a SHA256SUMS line is not \"<sha256>  <file>\"",
            ));
        };
        let file = file.trim().trim_start_matches('*');
        if !(sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit())) {
            return Err(invalid_bundle(
                bundle_name,
                "SHA256SUMS",
                "a SHA256SUMS line carries no 64-hex digest",
            ));
        }
        seen.push((sha, file));
    }
    for (sha, file) in &seen {
        match unpacked.iter().find(|m| m.name == *file) {
            Some(m) if m.sha256.eq_ignore_ascii_case(sha) => {}
            Some(m) => {
                return Err(invalid_bundle(
                    bundle_name,
                    &m.name,
                    "SHA256SUMS disagrees with the member bytes",
                ));
            }
            None => {
                return Err(invalid_bundle(
                    bundle_name,
                    file,
                    "SHA256SUMS names a member the bundle does not carry",
                ));
            }
        }
    }
    for m in unpacked {
        if !seen.iter().any(|(_, f)| *f == m.name) {
            return Err(invalid_bundle(
                bundle_name,
                &m.name,
                "SHA256SUMS does not pin this member",
            ));
        }
    }
    Ok(())
}
