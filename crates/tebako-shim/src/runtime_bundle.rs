//! The release bundle's in-process unpack + verification (spec 36 §2/§4
//! — Rust `tar` + `flate2`, no shell-outs, invariant 1). This is the
//! shim's copy of the grammar its siblings carry
//! (`crates/tebako-cli/src/runtime_bundle.rs`,
//! `crates/tebako-bootstrap/src/lib.rs`); the three are pinned by the
//! same spec section:
//!
//! - the member grammar (§2): regular files only — no directories,
//!   symlinks, or device nodes — named by bare relative basenames (no
//!   absolute paths, no `..` segments);
//! - the member SET is exactly the shard's declared members plus the
//!   closing `SHA256SUMS` (the LAST member — anything after it violates
//!   the grammar); never a skipped member, never an extra one;
//! - every member's streamed sha256 against the shard's per-member pin,
//!   and the in-bundle `SHA256SUMS` agreeing with the member bytes;
//! - every disagreement is the named InvalidBundle (spec 36 §7 — exit
//!   70's integrity class), never a best-effort unpack, never a partial
//!   store write (the caller stages into the entry's tmp dir and
//!   publishes by rename only after every member verified).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::Digest;

use crate::{ShimError, EX_TEBAKO_IO, EX_TEBAKO_SHA};

/// One member the shard declares the bundle carries: the served
/// basename plus the per-member pin (the shard's sha fields — spec 36
/// §3 — which pin the UNPACKED bytes).
#[derive(Debug, Clone)]
pub(crate) struct ExpectedMember {
    pub name: String,
    pub sha256: String,
}

/// A verified member staged under the scratch dir.
#[derive(Debug)]
pub(crate) struct UnpackedMember {
    pub name: String,
    pub path: PathBuf,
    pub sha256: String,
}

/// The closing `SHA256SUMS` member is text and bounded (spec 36 §2): it
/// is the only member read into memory, capped at 1 MiB.
const SUMS_MEMBER_CAP: u64 = 1 << 20;

/// The InvalidBundle class (spec 36 §7): exit 70's integrity row,
/// naming the bundle and the member it disagrees with.
fn invalid_bundle(bundle_name: &str, member: &str, detail: &str) -> ShimError {
    ShimError::new(
        EX_TEBAKO_SHA,
        format!(
            "invalid release bundle {bundle_name} (member {member}): {detail} — the release manifest and the bundle disagree; nothing was installed"
        ),
    )
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Unpack `bundle_path` (a fetched `<stem>.tar.gz`) under the §2
/// grammar, verifying each member against the shard-declared `members`
/// and the closing SHA256SUMS against the member bytes. Verified members
/// stage under `scratch` (prefixed by the bundle name + pid, so a sweep
/// never touches another fetch's files) and are returned in declaration
/// order.
pub(crate) fn unpack_bundle(
    bundle_path: &Path,
    bundle_name: &str,
    members: &[ExpectedMember],
    scratch: &Path,
) -> Result<Vec<UnpackedMember>, ShimError> {
    let result = unpack_bundle_inner(bundle_path, bundle_name, members, scratch);
    if result.is_err() {
        // A refused bundle stages nothing: sweep this unpack's prefix.
        for path in staged_files(scratch, bundle_name) {
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
) -> Result<Vec<UnpackedMember>, ShimError> {
    let file = fs::File::open(bundle_path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("{e} reading {}", bundle_path.display()),
        )
    })?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    let undecodable =
        |member: &str| invalid_bundle(bundle_name, member, "the tar/gzip stream is undecodable");
    let mut unpacked: Vec<UnpackedMember> = Vec::new();
    let mut sums_seen = false;
    let entries = archive.entries().map_err(|_| undecodable(bundle_name))?;
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
        let stage = scratch.join(format!("{bundle_name}.{}.{name}", std::process::id()));
        let mut out = fs::File::create(&stage).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("{e} staging {}", stage.display()),
            )
        })?;
        let mut hasher = sha2::Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = entry.read(&mut buf).map_err(|_| undecodable(&name))?;
            if n == 0 {
                break;
            }
            std::io::Write::write_all(&mut out, &buf[..n]).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_IO,
                    format!("{e} staging {}", stage.display()),
                )
            })?;
            hasher.update(&buf[..n]);
        }
        let sha256 = hex_lower(&hasher.finalize());
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
) -> Result<(), ShimError> {
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

/// Test fixtures shared with the fetch-commit tests in `runtime.rs`.
#[cfg(test)]
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(&sha2::Sha256::digest(bytes))
}

/// Write a `<stem>.tar.gz` fixture: the members (a `!`-prefixed name
/// writes the name field RAW — the tar crate refuses to *write* `..`
/// paths; mode 0o777000 marks the symlink grammar violation), then the
/// closing SHA256SUMS (or `sums_override`).
#[cfg(test)]
pub(crate) fn write_bundle(path: &Path, members: &[(&str, &[u8], u32)], sums_override: Option<&str>) {
    let file = fs::File::create(path).unwrap();
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::none());
    let mut builder = tar::Builder::new(gz);
    let mut sums = String::new();
    for (name, bytes, mode) in members {
        let mut h = tar::Header::new_gnu();
        if *mode == 0o777000 {
            h.set_entry_type(tar::EntryType::Symlink);
            h.set_size(0);
            h.set_mode(0o777);
            builder
                .append_data(&mut h, name, std::io::empty())
                .unwrap();
            continue;
        }
        h.set_entry_type(tar::EntryType::Regular);
        h.set_size(bytes.len() as u64);
        h.set_mode(*mode);
        if let Some(raw_name) = name.strip_prefix('!') {
            h.as_mut_bytes()[..raw_name.len()].copy_from_slice(raw_name.as_bytes());
            h.set_cksum();
            builder.append(&h, *bytes).unwrap();
            sums.push_str(&format!("{}  {}\n", sha256_hex(bytes), raw_name));
        } else {
            builder.append_data(&mut h, name, *bytes).unwrap();
            sums.push_str(&format!("{}  {}\n", sha256_hex(bytes), name));
        }
    }
    let sums = sums_override.map(str::to_string).unwrap_or(sums);
    let mut h = tar::Header::new_gnu();
    h.set_entry_type(tar::EntryType::Regular);
    h.set_size(sums.len() as u64);
    h.set_mode(0o444);
    builder
        .append_data(&mut h, "SHA256SUMS", sums.as_bytes())
        .unwrap();
    builder.finish().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-shim-bundle-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn expected(name: &str, bytes: &[u8]) -> ExpectedMember {
        ExpectedMember {
            name: name.to_string(),
            sha256: sha256_hex(bytes),
        }
    }

    #[test]
    fn a_wellformed_bundle_unpacks_verified_in_declaration_order() {
        let dir = scratch("ok");
        let bundle = dir.join("stem.tar.gz");
        write_bundle(
            &bundle,
            &[("exe", b"the interpreter", 0o755), ("img.tfs", b"the env image", 0o444)],
            None,
        );
        let members = [expected("exe", b"the interpreter"), expected("img.tfs", b"the env image")];
        let unpacked = unpack_bundle(&bundle, "stem.tar.gz", &members, &dir).unwrap();
        assert_eq!(unpacked.len(), 2);
        assert_eq!(unpacked[0].name, "exe");
        assert_eq!(fs::read(&unpacked[0].path).unwrap(), b"the interpreter");
        assert_eq!(unpacked[1].name, "img.tfs");
        assert_eq!(unpacked[1].sha256, sha256_hex(b"the env image"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn grammar_violations_are_the_named_invalid_bundle() {
        type Case = (&'static str, Vec<(&'static str, &'static [u8], u32)>, Option<String>, &'static str);
        let cases: Vec<Case> = vec![
            ("traverse", vec![("!../escape", b"x", 0o644)], None, "bare relative"),
            ("symlink", vec![("link", b"", 0o777000)], None, "regular files only"),
            (
                "undeclared",
                vec![("exe", b"real", 0o755), ("extra", b"x", 0o644)],
                None,
                "does not declare this member",
            ),
            (
                "pin-mismatch",
                vec![("exe", b"tampered", 0o755)],
                None,
                "member checksum mismatch",
            ),
            (
                "sums-missing-member",
                vec![("exe", b"real", 0o755)],
                Some(String::new()),
                "does not pin this member",
            ),
            (
                "sums-stranger",
                vec![("exe", b"real", 0o755)],
                Some(format!("{}  ghost\n", sha256_hex(b"real"))),
                "names a member the bundle does not carry",
            ),
        ];
        for (tag, members, sums, detail) in cases {
            let dir = scratch(tag);
            let bundle = dir.join("stem.tar.gz");
            write_bundle(&bundle, &members, sums.as_deref());
            let declared = [expected("exe", b"real")];
            let err = unpack_bundle(&bundle, "stem.tar.gz", &declared, &dir).unwrap_err();
            assert_eq!(err.code, EX_TEBAKO_SHA, "{tag}: {err:?}");
            assert!(err.message.contains(detail), "{tag}: {}", err.message);
            // A refused bundle stages nothing (the sweep).
            assert!(staged_files(&dir, "stem.tar.gz").is_empty(), "{tag}");
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn a_declared_but_absent_member_is_refused() {
        let dir = scratch("absent");
        let bundle = dir.join("stem.tar.gz");
        write_bundle(&bundle, &[("exe", b"real", 0o755)], None);
        let declared = [expected("exe", b"real"), expected("img.tfs", b"image")];
        let err = unpack_bundle(&bundle, "stem.tar.gz", &declared, &dir).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA);
        assert!(err.message.contains("does not carry it"), "{}", err.message);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_undecodable_stream_is_refused() {
        let dir = scratch("raw");
        let bundle = dir.join("stem.tar.gz");
        fs::write(&bundle, b"not a gzip stream at all").unwrap();
        let declared = [expected("exe", b"real")];
        let err = unpack_bundle(&bundle, "stem.tar.gz", &declared, &dir).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA);
        assert!(err.message.contains("undecodable"), "{}", err.message);
        let _ = fs::remove_dir_all(&dir);
    }
}
