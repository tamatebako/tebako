//! The bootstrap's embedded self-description (spec 18 §2.2, scenario
//! S38): a marked `artifact-info.yaml` block compiled into the binary,
//! so a reader (`tebako inspect`, a press) can learn the binary's
//! contract set — era, version, launcher ABI, spoken contract — WITHOUT
//! executing it, and refuse an era-1 bootstrap (a binary with no block
//! declares nothing; undeclared = pre-era).
//!
//! The block is assembled at compile time from the crate's own
//! constants ([`crate::SPOKEN_ERA`], [`crate::LAUNCHER_ABI`],
//! [`crate::SUPPORTED_CONTRACT`], `CARGO_PKG_VERSION`) — SSOT: the
//! values flow, nothing is hand-copied. `#[used]` keeps the block
//! through LTO and `strip`; readers scan the artifact's bytes for the
//! markers ([`extract`]).
//!
//! Note on the spec's "appended by the build": cargo has no post-link
//! hook, so the block rides the binary's read-only data instead of a
//! post-build append — same bytes discoverable by the same marker scan,
//! zero build plumbing. The release pipeline may additionally append it
//! (the markers make both placements readable); nothing downstream
//! re-authors it.

use crate::{LAUNCHER_ABI, SPOKEN_ERA, SUPPORTED_CONTRACT};

/// The begin marker (own line, LF-delimited on every platform).
pub const BLOCK_BEGIN: &str = "\n--- tebako-artifact-info-v1 ---\n";
/// The end marker.
pub const BLOCK_END: &str = "\n--- /tebako-artifact-info-v1 ---\n";

const MAX_BLOCK: usize = 512;

const fn push_bytes(
    mut out: [u8; MAX_BLOCK],
    mut pos: usize,
    bytes: &[u8],
) -> ([u8; MAX_BLOCK], usize) {
    let mut i = 0;
    while i < bytes.len() {
        out[pos] = bytes[i];
        pos += 1;
        i += 1;
    }
    (out, pos)
}

const fn push_u32(out: [u8; MAX_BLOCK], pos: usize, n: u32) -> ([u8; MAX_BLOCK], usize) {
    let mut digits = [0u8; 10];
    let mut n = n;
    let mut i = 10;
    loop {
        i -= 1;
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let mut out = out;
    let mut pos = pos;
    while i < 10 {
        out[pos] = digits[i];
        pos += 1;
        i += 1;
    }
    (out, pos)
}

/// The block bytes: markers + YAML, built from the crate constants.
const fn build_block() -> ([u8; MAX_BLOCK], usize) {
    let out = [0u8; MAX_BLOCK];
    let (out, pos) = push_bytes(out, 0, BLOCK_BEGIN.as_bytes());
    let (out, pos) = push_bytes(out, pos, b"schema: artifact-info\nschema_version: 1\n");
    let (out, pos) = push_bytes(out, pos, b"era: ");
    let (out, pos) = push_u32(out, pos, SPOKEN_ERA);
    let (out, pos) = push_bytes(out, pos, b"\nartifact: tebako-bootstrap\nversion: \"");
    let (out, pos) = push_bytes(out, pos, env!("CARGO_PKG_VERSION").as_bytes());
    let (out, pos) = push_bytes(out, pos, b"\"\nlauncher_abi: ");
    let (out, pos) = push_u32(out, pos, LAUNCHER_ABI);
    let (out, pos) = push_bytes(out, pos, b"\ncontract_version: ");
    let (out, pos) = push_u32(out, pos, SUPPORTED_CONTRACT);
    let (out, pos) = push_bytes(out, pos, b"\n");
    let (out, pos) = push_bytes(out, pos, BLOCK_END.as_bytes());
    (out, pos)
}

const BLOCK: ([u8; MAX_BLOCK], usize) = build_block();
const BLOCK_LEN: usize = BLOCK.1;

/// The marked block as embedded in the binary (`#[used]` — it survives
/// LTO and strip). Only the first `BLOCK_LEN` bytes carry content.
#[used]
pub static EMBEDDED: [u8; MAX_BLOCK] = BLOCK.0;

/// The artifact-info YAML body (the bytes between the markers), for
/// programmatic consumers — the binary itself and tests.
pub fn yaml() -> &'static str {
    let begin = BLOCK_BEGIN.len();
    let end = BLOCK_LEN - BLOCK_END.len();
    std::str::from_utf8(&EMBEDDED[begin..end]).unwrap_or("")
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Scan an artifact's bytes for the embedded block (the `tebako inspect`
/// / press read path): the YAML between the first begin/end marker pair.
/// `None` when no block is present — the era-1 signal (S38: a bootstrap
/// that declares nothing is refused, never assumed).
pub fn extract(bytes: &[u8]) -> Option<&str> {
    let begin = find_subslice(bytes, BLOCK_BEGIN.as_bytes())?;
    let rest = &bytes[begin + BLOCK_BEGIN.len()..];
    let end = find_subslice(rest, BLOCK_END.as_bytes())?;
    std::str::from_utf8(&rest[..end]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field<'a>(yaml: &'a str, key: &str) -> Option<&'a str> {
        let prefix = format!("{key}: ");
        yaml.lines()
            .find_map(|l| l.strip_prefix(&prefix))
            .map(|v| v.trim_matches('"'))
    }

    #[test]
    fn the_yaml_flows_from_the_crate_constants() {
        let yaml = yaml();
        assert_eq!(field(yaml, "schema"), Some("artifact-info"));
        assert_eq!(field(yaml, "schema_version"), Some("1"));
        assert_eq!(
            field(yaml, "era").and_then(|v| v.parse::<u32>().ok()),
            Some(SPOKEN_ERA)
        );
        assert_eq!(field(yaml, "artifact"), Some("tebako-bootstrap"));
        assert_eq!(field(yaml, "version"), Some(env!("CARGO_PKG_VERSION")));
        assert_eq!(
            field(yaml, "launcher_abi").and_then(|v| v.parse::<u32>().ok()),
            Some(LAUNCHER_ABI)
        );
        assert_eq!(
            field(yaml, "contract_version").and_then(|v| v.parse::<u32>().ok()),
            Some(SUPPORTED_CONTRACT)
        );
    }

    #[test]
    fn extract_round_trips_the_embedded_block() {
        assert_eq!(extract(&EMBEDDED), Some(yaml()));
        // markers survive arbitrary surrounding bytes (a stitched package
        // prefixes the bootstrap with nothing and appends slots + a
        // trailer; a release append would suffix)
        let mut blob = b"MZ fake exe bytes".to_vec();
        blob.extend_from_slice(&EMBEDDED[..BLOCK_LEN]);
        blob.extend_from_slice(b"trailing trailer bytes");
        assert_eq!(extract(&blob), Some(yaml()));
        // no block → the era-1 signal
        assert_eq!(extract(b"plain old era-1 binary"), None);
    }

    #[test]
    fn the_block_survives_a_real_link() {
        // The strongest available proof short of the release build: this
        // very test binary links the crate — the markers must be
        // discoverable in its bytes (#[used] holds under the default
        // profile; the release profile adds fat LTO and the CI size gate
        // re-proves it there).
        let exe = std::env::current_exe().unwrap();
        let bytes = std::fs::read(&exe).unwrap();
        let found = extract(&bytes).unwrap_or_else(|| {
            panic!("no artifact-info block in {}", exe.display());
        });
        assert_eq!(found, yaml());
    }
}
