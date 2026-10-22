//! Resume state for interrupted pulls.
//!
//! A pull that dies mid-transfer keeps its bytes today nowhere: the staged
//! `tmp/` file is discarded, and the next attempt restarts from zero. This
//! module owns the state that makes the remainder fetchable — where the
//! part lives, how it is named, how its prefix re-enters the digest, and
//! what range the next attempt asks for.
//!
//! The rules the state obeys:
//!
//! - **Only transports that can answer a range resume.** `https` and
//!   `oci` can; `file://` and the GitHub contents API are whole-object by
//!   nature, so a pull from them never leaves resumable state.
//! - **Only a pull with a digest anchor resumes.** Without a pin there is
//!   nothing to verify the join against, and a spliced file from two
//!   responses is worse than a restart.
//! - **The part is named for the artifact, not the process** — a stable
//!   name across runs is the whole point (the ordinary tmp name carries
//!   the pid and a plan sequence, which is exactly what resume must not).
//! - **A resumed pull re-verifies the WHOLE object at the end.** The
//!   prefix is re-hashed before the stream continues and the final digest
//!   is checked against the pin exactly as a fresh fetch is; a corrupt
//!   part fails named and is discarded, never healed.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// `TEBAKO_RESUME=1` opts in; anything else (including unset) keeps the
/// current restart-from-zero behavior.
pub fn enabled() -> bool {
    std::env::var("TEBAKO_RESUME").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Whether a reference of this kind can carry resumable state. Kept next
/// to [`enabled`] so the one policy question has one home.
pub fn transport_resumes(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("tfs+oci:")
}

/// The stable part path for one artifact: `<tmp_dir>/<display>.<pin8>.resume`.
/// The pin (the row's digest) is part of the NAME, so a part can never be
/// spliced onto a different artifact's bytes — a changed pin simply names
/// a different file, and the old part is collected by the tmp sweep.
pub fn part_path(tmp_dir: &Path, display: &str, pin: &str) -> PathBuf {
    let safe: String = display
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let pin8: String = pin.chars().take(8).collect();
    tmp_dir.join(format!("{safe}.{pin8}.resume"))
}

/// Re-enter the part's prefix into the digest: returns the hasher already
/// fed with the existing bytes and their count. Reading the prefix is the
/// price of never trusting a spliced file — it is O(part), not O(total),
/// and the alternative (persisting a mid-stream hasher state) would make
/// an interrupted program's crash points part of the wire format.
pub fn seed(part: &Path) -> std::io::Result<(Sha256, u64)> {
    use std::io::Read as _;
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(part)?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    Ok((hasher, total))
}

/// Discard a part — the corrupt-join and abandoned-artifact path.
pub fn discard(part: &Path) {
    let _ = std::fs::remove_file(part);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-resolve-resume-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_part_is_named_for_the_artifact_and_its_pin_not_the_process() {
        let dir = scratch("name");
        let pin = "a".repeat(64);
        let path = part_path(&dir, "payload app@1.0", &pin);
        assert_eq!(path, dir.join("payload app@1.0.aaaaaaaa.resume"));
        // A different pin is a different file: a part can never be
        // spliced onto another artifact's bytes.
        let other = part_path(&dir, "payload app@1.0", &"b".repeat(64));
        assert_ne!(path, other);
        // Path separators in the display never escape the tmp dir.
        let nested = part_path(&dir, "a/b\\c", &pin);
        assert_eq!(nested.parent(), Some(dir.as_path()));
    }

    #[test]
    fn seeding_re_enters_the_prefix_into_the_digest() {
        let dir = scratch("seed");
        let part = dir.join("x.resume");
        let prefix = b"0123456789";
        std::fs::write(&part, prefix).unwrap();

        let (mut hasher, have) = seed(&part).unwrap();
        assert_eq!(have, prefix.len() as u64);
        hasher.update(b"abcdef");
        let joined = crate::fetch::hex_digest(&hasher.finalize());

        // The join hashes exactly the concatenation — one digest, no
        // second implementation of the stream.
        let mut whole = Sha256::new();
        whole.update(prefix);
        whole.update(b"abcdef");
        assert_eq!(joined, crate::fetch::hex_digest(&whole.finalize()));
    }

    #[test]
    fn an_absent_part_is_an_io_error_and_discard_is_idempotent() {
        let dir = scratch("absent");
        assert!(seed(&dir.join("missing.resume")).is_err());
        discard(&dir.join("missing.resume")); // never panics
    }

    #[test]
    fn only_range_capable_transports_resume() {
        assert!(transport_resumes("https://example/a.tfs"));
        assert!(transport_resumes("tfs+oci://ghcr.io/o/r:1.0"));
        assert!(!transport_resumes("file:///tmp/a.tfs"));
        assert!(!transport_resumes("tfs:github:o/r:1.0"));
    }
}
