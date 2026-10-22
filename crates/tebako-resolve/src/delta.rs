//! Block-diff transfer for image updates (the distribution fabric's delta
//! fetch): when the store already holds a previous version of the same
//! artifact family, the new image is assembled from the OLD file plus the
//! block groups the target's `<image>.blksum.json` says changed, instead of
//! downloading the whole image again.
//!
//! **The trust rule.** Block digests only decide what to FETCH; the assembled
//! file is verified against the caller's whole-image pin before it is returned,
//! and a mismatch deletes the output and names both digests. A lying sidecar
//! can therefore waste bandwidth, never install bytes.
//!
//! The planner is pure over the donor file and the sidecar; the bytes arrive
//! through a caller-supplied range fetcher, so the same code serves the shim
//! (a transport's ranged GET), the press resolver, and the tests (a counting
//! stub).

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tpkg::lazy::{group_count_for, Blksum};

/// What the assembly moved: the bytes copied locally, the bytes fetched from
/// the host, and how many groups each path covered. The journal carries these
/// per install, so a delta's effect is auditable rather than inferred.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeltaStats {
    pub bytes_copied: u64,
    pub bytes_fetched: u64,
    pub groups_copied: u64,
    pub groups_fetched: u64,
}

impl DeltaStats {
    /// Total bytes written (the assembled image's size).
    pub fn bytes_written(&self) -> u64 {
        self.bytes_copied + self.bytes_fetched
    }

    /// The share of the image that travelled on the wire (0 when the donor
    /// was identical).
    pub fn fetched_fraction(&self) -> f64 {
        let total = self.bytes_written();
        if total == 0 {
            0.0
        } else {
            self.bytes_fetched as f64 / total as f64
        }
    }

    /// The journal line's body (`event=delta-fetch …`).
    pub fn journal_fields(&self) -> String {
        format!(
            "bytes_transferred={} bytes_total={} groups_fetched={} groups_copied={}",
            self.bytes_fetched,
            self.bytes_written(),
            self.groups_fetched,
            self.groups_copied
        )
    }
}

/// Every way an assembly can refuse. All are named: a delta that cannot be
/// completed is the caller's fallback signal, never a silent partial file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaError {
    /// The sidecar contradicts itself (group count off the size) — a
    /// corrupt release, refused before a byte moves.
    Sidecar(String),
    /// The donor file could not be read (a permission or a vanished file);
    /// the caller may retry from a full fetch.
    Donor(String),
    /// A range the host was asked for did not arrive intact.
    Fetch(String),
    /// The assembled bytes do not verify the whole-image pin. The output is
    /// deleted before this is returned.
    ShaMismatch { expected: String, actual: String },
    /// The output could not be written.
    Io(String),
}

impl std::fmt::Display for DeltaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeltaError::Sidecar(why) => write!(f, "the blksum sidecar is inconsistent: {why}"),
            DeltaError::Donor(why) => write!(f, "the donor image is unusable: {why}"),
            DeltaError::Fetch(why) => write!(f, "a range fetch failed: {why}"),
            DeltaError::ShaMismatch { expected, actual } => write!(
                f,
                "the assembled image hashes {actual} but the pin is {expected}; nothing was kept"
            ),
            DeltaError::Io(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for DeltaError {}

/// Assemble `sidecar`'s image into `out` from `donor` plus ranges fetched
/// through `fetch`, verifying the whole-image `pin` (lowercase hex) before
/// returning. `fetch(offset, len)` is asked for the spans whose donor bytes do
/// not hash to the target group digest.
///
/// The donor may be shorter than the target (a smaller previous version) or
/// absent entirely: each group then falls to the fetch path.
pub fn assemble(
    donor: &Path,
    sidecar: &Blksum,
    pin: &str,
    out: &Path,
    fetch: &mut dyn FnMut(u64, u64) -> Result<Vec<u8>, DeltaError>,
) -> Result<DeltaStats, DeltaError> {
    let expected_groups = group_count_for(sidecar.size_bytes);
    if sidecar.groups.len() as u64 != expected_groups {
        return Err(DeltaError::Sidecar(format!(
            "{} groups for {} bytes (expected {expected_groups})",
            sidecar.groups.len(),
            sidecar.size_bytes
        )));
    }
    // Assembling over the donor would destroy the very bytes the copy arm
    // reads; the caller's tmp naming never does this, and a direct call
    // that tries is refused by name rather than corrupting a store entry.
    if out.exists() {
        let same = fs::canonicalize(out)
            .ok()
            .zip(fs::canonicalize(donor).ok())
            .map(|(a, b)| a == b)
            .unwrap_or(false);
        if same {
            return Err(DeltaError::Donor(
                "the output path is the donor image itself — refusing to assemble over the source"
                    .to_string(),
            ));
        }
    }
    let mut donor_file = File::open(donor).ok();
    let donor_len = donor_file
        .as_ref()
        .and_then(|f| f.metadata().ok())
        .map(|m| m.len())
        .unwrap_or(0);
    let mut out_file = File::create(out)
        .map_err(|e| DeltaError::Io(format!("cannot create {}: {e}", out.display())))?;
    let mut stats = DeltaStats::default();

    for (index, digest) in sidecar.groups.iter().enumerate() {
        let (offset, len) = tpkg::lazy::group_span(sidecar.size_bytes, index as u64)
            .ok_or_else(|| DeltaError::Sidecar(format!("group {index} has no span")))?;
        let mut carried = false;
        if offset + len <= donor_len {
            let mut buf = vec![0u8; len as usize];
            if let Some(file) = donor_file.as_mut() {
                let read = file
                    .seek(SeekFrom::Start(offset))
                    .and_then(|_| file.read_exact(&mut buf));
                if let Err(e) = read {
                    return Err(DeltaError::Donor(format!(
                        "cannot read {} at {offset}+{len}: {e}",
                        donor.display()
                    )));
                }
            }
            if &tpkg::lazy::sha256_hex(&buf) == digest {
                out_file
                    .write_all(&buf)
                    .map_err(|e| DeltaError::Io(format!("cannot write {}: {e}", out.display())))?;
                stats.bytes_copied += len;
                stats.groups_copied += 1;
                carried = true;
            }
        }
        if carried {
            continue;
        }
        let bytes = fetch(offset, len)?;
        if bytes.len() as u64 != len {
            return Err(DeltaError::Fetch(format!(
                "group {index} at {offset}+{len} came back as {} bytes",
                bytes.len()
            )));
        }
        if &tpkg::lazy::sha256_hex(&bytes) != digest {
            return Err(DeltaError::Fetch(format!(
                "group {index} at {offset}+{len} does not hash to the sidecar's digest"
            )));
        }
        out_file
            .write_all(&bytes)
            .map_err(|e| DeltaError::Io(format!("cannot write {}: {e}", out.display())))?;
        stats.bytes_fetched += len;
        stats.groups_fetched += 1;
    }

    out_file
        .flush()
        .map_err(|e| DeltaError::Io(format!("cannot flush {}: {e}", out.display())))?;
    drop(out_file);

    let actual = sha256_file(out)?;
    if !actual.eq_ignore_ascii_case(pin) {
        let _ = fs::remove_file(out);
        return Err(DeltaError::ShaMismatch {
            expected: pin.to_string(),
            actual,
        });
    }
    Ok(stats)
}

/// The streaming sha256 of a file (the whole-image verification's own read).
fn sha256_file(path: &Path) -> Result<String, DeltaError> {
    let mut file = File::open(path)
        .map_err(|e| DeltaError::Io(format!("cannot open {}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| DeltaError::Io(format!("cannot read {}: {e}", path.display())))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// The store entries that can donate blocks to a target of this family:
/// every `runtimes/<engine>-<ver>-<tebako>-<platform>/` directory holding at
/// least one `.tfs`, most recently modified first, never the target's own
/// entry. The shim calls this with its own `runtimes` root; the press
/// resolver calls it with the store it resolves against.
///
/// The scan is deliberately dumb — a directory listing, no manifest reads —
/// because a donor is a hint, not a trust decision: the assembler verifies
/// every carried group against the target's sidecar, so a wrong donor costs
/// fetches, never correctness.
pub fn donor_images(root: &Path, engine: &str, platform: &str, exclude_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let prefix = format!("{engine}-");
    let suffix = format!("-{platform}");
    let mut donors: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(&prefix) || !name.ends_with(&suffix) {
            continue;
        }
        let dir = entry.path();
        if dir == exclude_dir || !dir.is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(&dir) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) != Some("tfs") {
                continue;
            }
            let modified = file
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            donors.push((modified, path));
        }
    }
    donors.sort_by(|a, b| b.0.cmp(&a.0));
    donors.into_iter().map(|(_, path)| path).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tebako-delta-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A deterministic image body: the byte value is a function of the
    /// offset, so changing one group leaves the others' digests intact.
    fn body(size: u64, tint: u8) -> Vec<u8> {
        // One 4 KiB chunk per group-index keeps the generator O(chunks)
        // rather than O(bytes): the 48 MiB fixture is test DATA, not a
        // workload.
        let mut out = Vec::with_capacity(size as usize);
        let mut index: u64 = 0;
        while out.len() < size as usize {
            let fill = ((index % 251) as u8) ^ tint;
            let take = (4096u64).min(size - out.len() as u64) as usize;
            out.extend(std::iter::repeat(fill).take(take));
            index += 1;
        }
        out
    }

    /// A 64-hex placeholder computed from a seed — a pin that is WRONG for
    /// any fixture, without a digest-shaped literal in the source.
    fn fake_digest(seed: u8) -> String {
        let mut hasher = Sha256::new();
        hasher.update([seed]);
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn sidecar_for(bytes: &[u8]) -> Blksum {
        let groups: Vec<String> = (0..group_count_for(bytes.len() as u64))
            .map(|i| {
                let (offset, len) = tpkg::lazy::group_span(bytes.len() as u64, i).unwrap();
                tpkg::lazy::sha256_hex(&bytes[offset as usize..(offset + len) as usize])
            })
            .collect();
        Blksum {
            size_bytes: bytes.len() as u64,
            sha256: tpkg::lazy::sha256_hex(bytes),
            groups,
        }
    }

    #[test]
    fn an_identical_donor_costs_no_bytes() {
        let dir = scratch("identical");
        let image = body(5 * 1024 * 1024, 0);
        let donor = dir.join("donor.tfs");
        let out = dir.join("out.tfs");
        fs::write(&donor, &image).unwrap();
        let sidecar = sidecar_for(&image);
        let mut fetched = 0u64;
        let stats = assemble(&donor, &sidecar, &sidecar.sha256, &out, &mut |_, len| {
            fetched += len;
            Err(DeltaError::Fetch("nothing should be fetched".into()))
        })
        .unwrap();
        assert_eq!(fetched, 0);
        assert_eq!(stats.bytes_fetched, 0);
        assert_eq!(stats.groups_fetched, 0);
        assert_eq!(stats.groups_copied, 2);
        assert_eq!(fs::read(&out).unwrap(), image);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_the_changed_group_is_fetched() {
        let dir = scratch("changed");
        let donor_bytes = body(5 * 1024 * 1024, 0);
        let mut target = donor_bytes.clone();
        // Flip one byte inside the SECOND group (offset >= 4 MiB).
        target[4 * 1024 * 1024 + 17] ^= 0xff;
        let donor = dir.join("donor.tfs");
        let out = dir.join("out.tfs");
        fs::write(&donor, &donor_bytes).unwrap();
        let sidecar = sidecar_for(&target);
        let mut spans: Vec<(u64, u64)> = Vec::new();
        let stats = assemble(
            &donor,
            &sidecar,
            &sidecar.sha256,
            &out,
            &mut |offset, len| {
                spans.push((offset, len));
                Ok(target[offset as usize..(offset + len) as usize].to_vec())
            },
        )
        .unwrap();
        assert_eq!(spans, vec![(4 * 1024 * 1024, 1024 * 1024)]);
        assert_eq!(stats.groups_fetched, 1);
        assert_eq!(stats.groups_copied, 1);
        assert_eq!(stats.bytes_fetched, 1024 * 1024);
        assert_eq!(fs::read(&out).unwrap(), target);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_donor_fetches_everything() {
        let dir = scratch("nod onor");
        let target = body(5 * 1024 * 1024, 3);
        let sidecar = sidecar_for(&target);
        let out = dir.join("out.tfs");
        let stats = assemble(
            &dir.join("absent.tfs"),
            &sidecar,
            &sidecar.sha256,
            &out,
            &mut |offset, len| Ok(target[offset as usize..(offset + len) as usize].to_vec()),
        )
        .unwrap();
        assert_eq!(stats.groups_fetched, 2);
        assert_eq!(stats.bytes_fetched, target.len() as u64);
        assert_eq!(fs::read(&out).unwrap(), target);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pin_mismatch_names_both_digests_and_keeps_nothing() {
        let dir = scratch("pin");
        let image = body(5 * 1024 * 1024, 0);
        let donor = dir.join("donor.tfs");
        let out = dir.join("out.tfs");
        fs::write(&donor, &image).unwrap();
        let sidecar = sidecar_for(&image);
        let wrong = fake_digest(0x5a);
        let err = assemble(&donor, &sidecar, &wrong, &out, &mut |_, _| {
            Err(DeltaError::Fetch("no fetch expected".into()))
        })
        .unwrap_err();
        match err {
            DeltaError::ShaMismatch { expected, actual } => {
                assert_eq!(expected, wrong);
                assert_eq!(actual, sidecar.sha256);
            }
            other => panic!("expected ShaMismatch, got {other:?}"),
        }
        assert!(!out.exists(), "the unverified output must not survive");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_range_is_refused_before_it_reaches_the_output() {
        let dir = scratch("badrange");
        let target = body(5 * 1024 * 1024, 1);
        let sidecar = sidecar_for(&target);
        let out = dir.join("out.tfs");
        let err = assemble(
            &dir.join("absent.tfs"),
            &sidecar,
            &sidecar.sha256,
            &out,
            &mut |_, _| Ok(vec![0u8; 10]),
        )
        .unwrap_err();
        assert!(matches!(err, DeltaError::Fetch(_)), "{err:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_donor_scan_matches_the_family_and_never_the_target() {
        let dir = scratch("donors");
        let root = dir.join("runtimes");
        let mk = |name: &str, image: Option<&str>| {
            let entry = root.join(name);
            fs::create_dir_all(&entry).unwrap();
            if let Some(image) = image {
                fs::write(entry.join(image), b"bytes").unwrap();
            }
        };
        mk("node-24.9.0-2.8.10-macos-arm64", Some("a.tfs"));
        mk("node-24.10.0-2.8.12-macos-arm64", Some("b.tfs"));
        mk("node-24.11.0-2.8.13-macos-arm64", None); // no image: not a donor
        mk("ruby-4.0.7-0.16.32-macos-arm64", Some("c.tfs")); // other engine
        mk("node-24.12.0-2.8.14-linux-gnu-x86_64", Some("d.tfs")); // other platform
        let target = root.join("node-24.13.0-2.8.56-macos-arm64");
        fs::create_dir_all(&target).unwrap();

        let donors = donor_images(&root, "node", "macos-arm64", &target);
        let names: Vec<String> = donors
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.contains(&"a.tfs".to_string()), "{names:?}");
        assert!(names.contains(&"b.tfs".to_string()), "{names:?}");

        // The target's own entry is never a donor, even when it holds an
        // image.
        fs::write(target.join("self.tfs"), b"bytes").unwrap();
        let donors = donor_images(&root, "node", "macos-arm64", &target);
        assert!(!donors.iter().any(|p| p.starts_with(&target)), "{donors:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn assembling_over_the_donor_is_refused() {
        let dir = scratch("over");
        let image = body(5 * 1024 * 1024, 0);
        let donor = dir.join("donor.tfs");
        fs::write(&donor, &image).unwrap();
        let sidecar = sidecar_for(&image);
        let err = assemble(&donor, &sidecar, &sidecar.sha256, &donor, &mut |_, _| {
            Err(DeltaError::Fetch("no fetch expected".into()))
        })
        .unwrap_err();
        assert!(matches!(err, DeltaError::Donor(_)), "{err:?}");
        assert_eq!(
            fs::read(&donor).unwrap(),
            image,
            "the donor must be untouched"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    // Heavy on purpose: the group size is the format's fixed 4 MiB, so a
    // <10% claim needs at least eleven groups. The wall time is debug-build
    // sha256 (~240 MB hashed across the fixture and the assembly), not I/O.
    fn a_twelve_group_image_moves_under_ten_percent() {
        let dir = scratch("ten");
        let donor_bytes = body(12 * 4 * 1024 * 1024, 0);
        let mut target = donor_bytes.clone();
        target[7 * 4 * 1024 * 1024 + 5] ^= 0x01;
        let donor = dir.join("donor.tfs");
        let out = dir.join("out.tfs");
        fs::write(&donor, &donor_bytes).unwrap();
        let sidecar = sidecar_for(&target);
        let stats = assemble(
            &donor,
            &sidecar,
            &sidecar.sha256,
            &out,
            &mut |offset, len| Ok(target[offset as usize..(offset + len) as usize].to_vec()),
        )
        .unwrap();
        assert!(
            stats.bytes_fetched * 10 < stats.bytes_written(),
            "{stats:?}"
        );
        assert_eq!(stats.groups_fetched, 1);
        assert_eq!(fs::read(&out).unwrap(), target);
        assert!(stats.journal_fields().contains("bytes_transferred="));
        let _ = fs::remove_dir_all(&dir);
    }
}
