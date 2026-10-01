//! The lazy-mount models and the LAZY_SEEDING store record (spec 39
//! §3/§4): the per-block-group digest sidecar (`<image>.blksum.json`),
//! the seed descriptor (`<image>.lazy.json`), and the block-directory
//! grammar (`<image>.blocks/<NNNNNN>.blk`) inside a runtime store entry.
//!
//! Both documents are MACHINE JSON (the workspace's own `tebako-json`,
//! no serde — the runtime_store.rs convention); both are versioned by
//! `schema_version` and cross-checked against
//! `schema/tpkg-blksum-v1.schema.json` and
//! `schema/tpkg-lazy-seed-v1.schema.json` (tests/lazy.rs keeps the
//! models and the schemas MECE).
//!
//! The trust chain (spec 39 §3) in one paragraph: the publisher authors
//! the blksum sidecar in the same invocation that stages the image; the
//! release row pins the sidecar's sha256; the loader verifies the
//! sidecar against that pin BEFORE the first range read, writes the
//! seed descriptor (tmp+rename under the entry's flock — the flock is
//! the CALLER's, spec 05 §4) instead of the image download, and every
//! fetched group verifies against `groups[i]` before it enters the
//! block cache. The seal pass (spec 39 §5, the driver's) re-verifies
//! the whole-image sha256 over the assembled groups.
//!
//! This module is pure fs + tebako-json: no network, no unsafe. The
//! byte source that consumes these models lives in `tfs`
//! (`source_remote.rs`, the `backend-remote` feature).

use std::path::{Path, PathBuf};

/// The locked fetch unit: 4 MiB of IMAGE bytes (spec 39 §11.3). The
/// final group is short. Tunable only by a measured follow-up — never
/// by config.
pub const LAZY_GROUP_SIZE: u64 = 4 * 1024 * 1024;

/// `schema_version` of both v1 documents.
pub const LAZY_SCHEMA_VERSION: u64 = 1;

// ---------------------------------------------------------------------
// errors (spec 39 §6's exit classes — named, never silent)
// ---------------------------------------------------------------------

/// The lazy path's named errors. `exit_code()` is the spec 39 §6
/// class; `Display` leads with the error's NAME (spec 00 invariant 9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LazyError {
    /// The seed descriptor failed validation at open (spec 39 §4) —
    /// exit 65.
    LazyDescriptorInvalid(String),
    /// The blksum sidecar failed validation (spec 39 §6's
    /// "lazy config malformed" row) — exit 65.
    BlksumInvalid(String),
    /// A digest mismatch: the sidecar against its pin, a group against
    /// its sidecar digest (2nd fetch), the whole image at seal
    /// (spec 39 §3/§6) — exit 70.
    Sha256Mismatch(String),
    /// A fetch/mount-open failure or offline miss (spec 39 §6) —
    /// exit 69.
    LazyUnavailable(String),
    /// Store IO (lock, block write, assemble, rename) — exit 74.
    LazyStoreIo(String),
}

impl LazyError {
    /// The spec 39 §6 exit class (65 usage / 69 unavailable /
    /// 70 integrity / 74 IO — the 65–79 space stands, never extended).
    pub fn exit_code(&self) -> i32 {
        match self {
            LazyError::LazyDescriptorInvalid(_) | LazyError::BlksumInvalid(_) => 65,
            LazyError::LazyUnavailable(_) => 69,
            LazyError::Sha256Mismatch(_) => 70,
            LazyError::LazyStoreIo(_) => 74,
        }
    }
}

impl std::fmt::Display for LazyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LazyError::LazyDescriptorInvalid(why) => write!(f, "LazyDescriptorInvalid: {why}"),
            LazyError::BlksumInvalid(why) => write!(f, "BlksumInvalid: {why}"),
            LazyError::Sha256Mismatch(why) => write!(f, "Sha256Mismatch: {why}"),
            LazyError::LazyUnavailable(why) => write!(f, "LazyUnavailable: {why}"),
            LazyError::LazyStoreIo(why) => write!(f, "LazyStoreIo: {why}"),
        }
    }
}

impl std::error::Error for LazyError {}

// ---------------------------------------------------------------------
// shared validation helpers
// ---------------------------------------------------------------------

/// A sha256 hex spelling: exactly 64 lowercase hex characters.
fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The sha256 hex of `bytes` (the one anchor vocabulary — spec 39
/// §11.9: sha256 everywhere; BLAKE3 stays image-internal).
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// The group count of a `size_bytes` image: ceil(size / group size).
pub fn group_count_for(size_bytes: u64) -> u64 {
    size_bytes.div_ceil(LAZY_GROUP_SIZE)
}

/// The `[offset, len)` image span of group `index` in a `size_bytes`
/// image; `None` out of range. The final group is short; a
/// `size_bytes` that is an exact multiple still ends ON a full group.
pub fn group_span(size_bytes: u64, index: u64) -> Option<(u64, u64)> {
    if size_bytes == 0 || index >= group_count_for(size_bytes) {
        return None;
    }
    let offset = index.checked_mul(LAZY_GROUP_SIZE)?;
    let len = LAZY_GROUP_SIZE.min(size_bytes - offset);
    Some((offset, len))
}

fn required_u64(
    obj: &tebako_json::Value,
    key: &str,
    bad: &impl Fn(String) -> LazyError,
) -> Result<u64, LazyError> {
    obj.find(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| bad(format!("{key} is required (a non-negative integer)")))
}

fn required_sha256(
    obj: &tebako_json::Value,
    key: &str,
    bad: &impl Fn(String) -> LazyError,
) -> Result<String, LazyError> {
    let value = obj
        .find(key)
        .and_then(|v| v.as_string())
        .ok_or_else(|| bad(format!("{key} is required")))?;
    if !is_sha256_hex(&value) {
        return Err(bad(format!(
            "{key} must be 64 lowercase hex characters (a sha256)"
        )));
    }
    Ok(value)
}

/// The common head of both documents: schema_version == 1, the locked
/// 4 MiB group_size, size_bytes >= 1. Unknown keys are tolerated at
/// every level (the schemas' forward-compat rule — consumers ignore
/// keys they predate).
fn parse_common_head(
    text: &str,
    what: &str,
    bad: &impl Fn(String) -> LazyError,
) -> Result<(tebako_json::Value, u64), LazyError> {
    let parsed = tebako_json::parse(text).map_err(|e| bad(format!("{what} is not JSON: {e}")))?;
    if !matches!(parsed, tebako_json::Value::Object(_)) {
        return Err(bad(format!("{what} must be a JSON object")));
    }
    let version = required_u64(&parsed, "schema_version", bad)?;
    if version != LAZY_SCHEMA_VERSION {
        return Err(bad(format!(
            "schema_version {version} is not {LAZY_SCHEMA_VERSION} — a newer document is not a guess"
        )));
    }
    let group_size = required_u64(&parsed, "group_size", bad)?;
    if group_size != LAZY_GROUP_SIZE {
        return Err(bad(format!(
            "group_size {group_size} is not the locked {LAZY_GROUP_SIZE}"
        )));
    }
    let size_bytes = required_u64(&parsed, "size_bytes", bad)?;
    if size_bytes == 0 {
        return Err(bad("size_bytes must be at least 1".to_string()));
    }
    Ok((parsed, size_bytes))
}

// ---------------------------------------------------------------------
// the blksum sidecar (spec 39 §3) — one sha256 per 4 MiB group
// ---------------------------------------------------------------------

/// The parsed `<image>.blksum.json` sidecar: the whole-image sha256
/// (identical to the `.sha256` anchor's value — the seal pass's
/// mandatory second anchor) and one sha256 per 4 MiB group, in index
/// order. The fetch unit's trust at read: a range GET's bytes verify
/// against `groups[i]` BEFORE they enter the block cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blksum {
    /// The whole image's byte length (`group_count` derives —
    /// [`Blksum::group_count`] — and MUST equal `groups.len()`).
    pub size_bytes: u64,
    /// The whole-image sha256 (hex).
    pub sha256: String,
    /// One sha256 (hex) per group, in index order.
    pub groups: Vec<String>,
}

impl Blksum {
    /// The group count — always `groups.len()` on a validated sidecar.
    pub fn group_count(&self) -> u64 {
        self.groups.len() as u64
    }

    /// The `[offset, len)` image span of group `index`; `None` out of
    /// range (the final group is short).
    pub fn group_span(&self, index: u64) -> Option<(u64, u64)> {
        group_span(self.size_bytes, index)
    }

    /// Parse and validate a sidecar (schema/tpkg-blksum-v1.schema.json):
    /// every malformed class is the named [`LazyError::BlksumInvalid`],
    /// never a guess. Verification against the row's `image.blksum`
    /// sha256 pin is the CALLER's step, before the first range read
    /// (a mismatch is [`LazyError::Sha256Mismatch`]).
    pub fn parse(text: &str) -> Result<Blksum, LazyError> {
        let bad = |why: String| LazyError::BlksumInvalid(format!("the blksum sidecar: {why}"));
        let (parsed, size_bytes) = parse_common_head(text, "the blksum sidecar", &bad)?;
        let sha256 = required_sha256(&parsed, "sha256", &bad)?;
        let groups = match parsed.find("groups") {
            Some(tebako_json::Value::Array(items)) => {
                let mut groups = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    let digest = item
                        .as_string()
                        .ok_or_else(|| bad(format!("groups[{i}] must be a string")))?;
                    if !is_sha256_hex(&digest) {
                        return Err(bad(format!(
                            "groups[{i}] must be 64 lowercase hex characters (a sha256)"
                        )));
                    }
                    groups.push(digest);
                }
                groups
            }
            _ => return Err(bad("groups is required (an array of digests)".to_string())),
        };
        if groups.is_empty() {
            return Err(bad("groups must carry at least one digest".to_string()));
        }
        let want = group_count_for(size_bytes);
        if groups.len() as u64 != want {
            return Err(bad(format!(
                "size_bytes {size_bytes} implies {want} groups but {} digests are declared",
                groups.len()
            )));
        }
        Ok(Blksum {
            size_bytes,
            sha256,
            groups,
        })
    }

    /// The machine JSON form (keys in schema order).
    pub fn render(&self) -> String {
        let mut out = format!(
            "{{\"schema_version\":{LAZY_SCHEMA_VERSION},\"group_size\":{LAZY_GROUP_SIZE},\"size_bytes\":{},\"sha256\":\"{}\",\"groups\":[",
            self.size_bytes,
            tebako_json::escape(&self.sha256),
        );
        for (i, digest) in self.groups.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&tebako_json::escape(digest));
            out.push('"');
        }
        out.push_str("]}");
        out
    }

    /// Compute the sidecar of image bytes held in memory — the
    /// PUBLISHER side (spec 39 §3: `tebako publish` generates the
    /// sidecar in-process in the same invocation that stages the
    /// release bytes; a resolver NEVER derives one).
    pub fn from_image_bytes(bytes: &[u8]) -> Blksum {
        assert!(
            !bytes.is_empty(),
            "an empty image has no blksum (size_bytes >= 1)"
        );
        let size_bytes = bytes.len() as u64;
        let groups = (0..group_count_for(size_bytes))
            .map(|i| {
                let (offset, len) =
                    group_span(size_bytes, i).expect("an index below the group count spans");
                sha256_hex(&bytes[offset as usize..(offset + len) as usize])
            })
            .collect();
        Blksum {
            size_bytes,
            sha256: sha256_hex(bytes),
            groups,
        }
    }
}

// ---------------------------------------------------------------------
// the seed descriptor (spec 39 §4) — present ⇔ LAZY_SEEDING
// ---------------------------------------------------------------------

/// The parsed `<image>.lazy.json` seed descriptor: the concrete origin
/// URL, the whole-image sha256 pin, the blksum sidecar's sha256 (the
/// sidecar is verified against THIS before the first range read), and
/// the image geometry. Its rename is the lazy install's commit point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LazySeed {
    /// The concrete origin the groups fetch from (https, or file /
    /// loopback-http per spec 39 §6's carve-outs). Recorded verbatim.
    pub source: String,
    /// The whole-image sha256 pin — the seal pass re-verifies against
    /// exactly this value.
    pub sha256: String,
    /// The digest of the blksum sidecar this seed was opened against.
    pub blksum_sha256: String,
    /// The whole image's byte length.
    pub size_bytes: u64,
    /// `ceil(size_bytes / group_size)` — the block directory's
    /// completeness bound.
    pub group_count: u64,
}

impl LazySeed {
    /// The `[offset, len)` image span of group `index`; `None` out of
    /// range.
    pub fn group_span(&self, index: u64) -> Option<(u64, u64)> {
        group_span(self.size_bytes, index)
    }

    /// Parse and validate a descriptor
    /// (schema/tpkg-lazy-seed-v1.schema.json): every malformed class is
    /// the named [`LazyError::LazyDescriptorInvalid`] — never a guess
    /// (spec 39 §4).
    pub fn parse(text: &str) -> Result<LazySeed, LazyError> {
        let bad =
            |why: String| LazyError::LazyDescriptorInvalid(format!("the seed descriptor: {why}"));
        let (parsed, size_bytes) = parse_common_head(text, "the seed descriptor", &bad)?;
        let source = parsed
            .find("source")
            .and_then(|v| v.as_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| bad("source is required (the concrete origin URL)".to_string()))?;
        let sha256 = required_sha256(&parsed, "sha256", &bad)?;
        let blksum_sha256 = required_sha256(&parsed, "blksum_sha256", &bad)?;
        let group_count = required_u64(&parsed, "group_count", &bad)?;
        let want = group_count_for(size_bytes);
        if group_count != want {
            return Err(bad(format!(
                "size_bytes {size_bytes} implies {want} groups but group_count is {group_count}"
            )));
        }
        Ok(LazySeed {
            source,
            sha256,
            blksum_sha256,
            size_bytes,
            group_count,
        })
    }

    /// The machine JSON form (keys in schema order).
    pub fn render(&self) -> String {
        format!(
            "{{\"schema_version\":{LAZY_SCHEMA_VERSION},\"source\":\"{}\",\"sha256\":\"{}\",\"blksum_sha256\":\"{}\",\"group_size\":{LAZY_GROUP_SIZE},\"size_bytes\":{},\"group_count\":{}}}",
            tebako_json::escape(&self.source),
            tebako_json::escape(&self.sha256),
            tebako_json::escape(&self.blksum_sha256),
            self.size_bytes,
            self.group_count,
        )
    }
}

// ---------------------------------------------------------------------
// the store record (spec 39 §4) — descriptor + block directory grammar
// ---------------------------------------------------------------------

/// The descriptor's file name inside a runtime store entry:
/// `<image>.lazy.json` — present if and only if the entry is
/// LAZY_SEEDING.
pub fn descriptor_name(image_base: &str) -> String {
    format!("{image_base}.lazy.json")
}

/// The block directory's name inside the entry: `<image>.blocks/`.
pub fn blocks_dir_name(image_base: &str) -> String {
    format!("{image_base}.blocks")
}

/// One group file's name: the zero-padded six-digit index
/// (`<NNNNNN>.blk`). Six digits cover a 4 TiB image at the locked
/// group size; wider indices still parse (the grammar is "digits +
/// `.blk`").
pub fn block_name(index: u64) -> String {
    format!("{index:06}.blk")
}

/// The in-progress spelling of a group file (`<NNNNNN>.blk.part`):
/// fetch → verify → write tmp → rename is the per-group crash law, so
/// a `.part` is never a seeded group — the next open drops it.
pub fn block_part_name(index: u64) -> String {
    format!("{}.part", block_name(index))
}

/// Parse a block file name back to its group index; `None` for
/// anything off the grammar (`*.part`, non-digits, the descriptor,
/// …).
pub fn parse_block_name(name: &str) -> Option<u64> {
    let digits = name.strip_suffix(".blk")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Write the seed descriptor — tmp+rename (`<name>.part` → `<name>`),
/// the store's crash law. The entry's flock is the CALLER's (spec 05
/// §4): this is the commit point of the lazy install, and it must land
/// under the same lock the exe install ran under.
pub fn write_descriptor(
    entry_dir: &Path,
    image_base: &str,
    seed: &LazySeed,
) -> Result<(), LazyError> {
    let final_path = entry_dir.join(descriptor_name(image_base));
    let part_path = entry_dir.join(format!("{}.part", descriptor_name(image_base)));
    let io = |what: &str, e: std::io::Error| {
        LazyError::LazyStoreIo(format!("{what} {}: {e}", final_path.display()))
    };
    std::fs::write(&part_path, seed.render()).map_err(|e| io("write", e))?;
    std::fs::rename(&part_path, &final_path).map_err(|e| io("rename", e))?;
    Ok(())
}

/// Read and validate the entry's seed descriptor: `Ok(None)` when
/// absent (the entry is not LAZY_SEEDING), the named
/// [`LazyError::LazyDescriptorInvalid`] when present but invalid
/// (spec 39 §4 — never a guess).
pub fn read_descriptor(entry_dir: &Path, image_base: &str) -> Result<Option<LazySeed>, LazyError> {
    let path = entry_dir.join(descriptor_name(image_base));
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(LazyError::LazyStoreIo(format!(
                "read {}: {e}",
                path.display()
            )))
        }
    };
    LazySeed::parse(&text).map(Some)
}

/// The block map IS the directory (spec 39 §4): scan `blocks_dir`,
/// returning the sorted indices of the seeded set. A group file
/// present = verified, by construction — anything off that law is
/// dropped so the next fetch reinstalls it: `*.part` (a crashed
/// write), unparseable names, size-mismatched payloads, indices at or
/// past the group count. A missing directory is the empty set.
pub fn scan_blocks(blocks_dir: &Path, size_bytes: u64) -> Result<Vec<u64>, LazyError> {
    let io = |what: &str, e: std::io::Error| {
        LazyError::LazyStoreIo(format!("{what} {}: {e}", blocks_dir.display()))
    };
    let rd = match std::fs::read_dir(blocks_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io("scan", e)),
    };
    let mut present = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let Some(index) = parse_block_name(&name) else {
            // `.part` and every off-grammar name: dropped (crash
            // debris, never a seeded group).
            let _ = std::fs::remove_file(&path);
            continue;
        };
        let valid = match group_span(size_bytes, index) {
            Some((_, want_len)) => entry
                .metadata()
                .map(|m| m.is_file() && m.len() == want_len)
                .unwrap_or(false),
            // An index at or past the group count is not a group of
            // THIS image.
            None => false,
        };
        if valid {
            present.push(index);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
    present.sort_unstable();
    Ok(present)
}

/// The entry's block directory path (`<entry>/<image>.blocks`).
pub fn blocks_dir(entry_dir: &Path, image_base: &str) -> PathBuf {
    entry_dir.join(blocks_dir_name(image_base))
}

/// The entry's descriptor path (`<entry>/<image>.lazy.json`).
pub fn descriptor_path(entry_dir: &Path, image_base: &str) -> PathBuf {
    entry_dir.join(descriptor_name(image_base))
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const HEX2: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tpkg-lazy-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // -------------------------------------------------------------
    // group math
    // -------------------------------------------------------------

    #[test]
    fn group_math_covers_the_edges() {
        // One full group exactly.
        assert_eq!(group_count_for(LAZY_GROUP_SIZE), 1);
        assert_eq!(group_span(LAZY_GROUP_SIZE, 0), Some((0, LAZY_GROUP_SIZE)));
        assert_eq!(group_span(LAZY_GROUP_SIZE, 1), None);
        // The short final group.
        let size = 2 * LAZY_GROUP_SIZE + 7;
        assert_eq!(group_count_for(size), 3);
        assert_eq!(group_span(size, 2), Some((2 * LAZY_GROUP_SIZE, 7)));
        // A single short group (an image smaller than one group).
        assert_eq!(group_count_for(100), 1);
        assert_eq!(group_span(100, 0), Some((0, 100)));
        // An exact multiple ends ON a full group — no phantom tail.
        assert_eq!(group_count_for(2 * LAZY_GROUP_SIZE), 2);
        assert_eq!(group_span(2 * LAZY_GROUP_SIZE, 2), None);
        // The empty image has no groups (and no valid blksum).
        assert_eq!(group_count_for(0), 0);
        assert_eq!(group_span(0, 0), None);
    }

    // -------------------------------------------------------------
    // the blksum model
    // -------------------------------------------------------------

    fn blksum_doc(size: u64, groups: &[&str]) -> String {
        format!(
            "{{\"schema_version\":1,\"group_size\":4194304,\"size_bytes\":{size},\"sha256\":\"{HEX}\",\"groups\":[{}]}}",
            groups
                .iter()
                .map(|g| format!("\"{g}\""))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    #[test]
    fn blksum_round_trips() {
        let sum = Blksum {
            size_bytes: LAZY_GROUP_SIZE + 1,
            sha256: HEX.to_string(),
            groups: vec![HEX.to_string(), HEX2.to_string()],
        };
        let parsed = Blksum::parse(&sum.render()).unwrap();
        assert_eq!(parsed, sum);
        assert_eq!(parsed.group_count(), 2);
        assert_eq!(parsed.group_span(1), Some((LAZY_GROUP_SIZE, 1)));
    }

    #[test]
    fn blksum_computes_over_image_bytes() {
        // 2.5 groups of deterministic bytes: per-group digests land on
        // the group spans, the whole digest on the whole.
        let bytes: Vec<u8> = (0..(2 * LAZY_GROUP_SIZE + LAZY_GROUP_SIZE / 2) as usize)
            .map(|i| (i % 251) as u8)
            .collect();
        let sum = Blksum::from_image_bytes(&bytes);
        assert_eq!(sum.size_bytes, bytes.len() as u64);
        assert_eq!(sum.group_count(), 3);
        assert_eq!(sum.sha256, sha256_hex(&bytes));
        assert_eq!(
            sum.groups[2],
            sha256_hex(&bytes[(2 * LAZY_GROUP_SIZE) as usize..])
        );
        // Round-trips through the wire form.
        assert_eq!(Blksum::parse(&sum.render()).unwrap(), sum);
    }

    #[test]
    fn blksum_rejects_every_malformed_class_by_name() {
        let good = blksum_doc(LAZY_GROUP_SIZE + 1, &[HEX, HEX2]);
        assert!(Blksum::parse(&good).is_ok());
        let cases = [
            "not json",
            "[1,2]",
            &blksum_doc(0, &[HEX]), // size < 1
            &good.replace("\"schema_version\":1", "\"schema_version\":2"),
            &good.replace("\"group_size\":4194304", "\"group_size\":1048576"),
            &good.replace(HEX2, "\"unterminated"),
            &good.replace(HEX2, "zz23"),              // groups[1] not hex
            &blksum_doc(LAZY_GROUP_SIZE + 1, &[HEX]), // count short
            &blksum_doc(LAZY_GROUP_SIZE + 1, &[HEX, HEX2, HEX]), // count long
            &good.replace("\"groups\":", "\"groupz\":"), // missing groups
            &good.replace("\"size_bytes\":", "\"size_bytez\":"), // missing size
        ];
        for case in cases {
            let err = Blksum::parse(case).unwrap_err();
            assert!(
                matches!(err, LazyError::BlksumInvalid(_)),
                "{case:?} -> {err:?}"
            );
            assert_eq!(err.exit_code(), 65);
            assert!(err.to_string().starts_with("BlksumInvalid: "), "{err}");
        }
        // Unknown keys ride forward (the schemas' forward-compat rule).
        let forward = good.replace("\"groups\":", "\"future_key\":{\"x\":1},\"groups\":");
        assert!(Blksum::parse(&forward).is_ok());
    }

    // -------------------------------------------------------------
    // the seed descriptor model
    // -------------------------------------------------------------

    fn seed() -> LazySeed {
        LazySeed {
            source: "https://example.test/img.tfs".to_string(),
            sha256: HEX.to_string(),
            blksum_sha256: HEX2.to_string(),
            size_bytes: LAZY_GROUP_SIZE + 1,
            group_count: 2,
        }
    }

    #[test]
    fn seed_round_trips() {
        let seed = seed();
        assert_eq!(LazySeed::parse(&seed.render()).unwrap(), seed);
        assert_eq!(seed.group_span(1), Some((LAZY_GROUP_SIZE, 1)));
        assert_eq!(seed.group_span(2), None);
    }

    #[test]
    fn seed_rejects_every_malformed_class_by_name() {
        let good = seed().render();
        let cases = [
            "not json".to_string(),
            good.replace("\"schema_version\":1", "\"schema_version\":9"),
            good.replace("\"group_size\":4194304", "\"group_size\":42"),
            good.replace(
                &format!("\"source\":\"{}\"", seed().source),
                "\"source\":\"\"",
            ),
            good.replace(HEX2, "xyz"),
            good.replace("\"group_count\":2", "\"group_count\":3"),
            good.replace("\"blksum_sha256\":", "\"blksum\":"),
        ];
        for case in cases {
            let err = LazySeed::parse(&case).unwrap_err();
            assert!(
                matches!(err, LazyError::LazyDescriptorInvalid(_)),
                "{case} -> {err:?}"
            );
            assert_eq!(err.exit_code(), 65);
            assert!(
                err.to_string().starts_with("LazyDescriptorInvalid: "),
                "{err}"
            );
        }
        // Unknown keys ride forward.
        let forward = good.replace("\"source\":", "\"tomorrow\":true,\"source\":");
        assert!(LazySeed::parse(&forward).is_ok());
    }

    // -------------------------------------------------------------
    // the store record
    // -------------------------------------------------------------

    #[test]
    fn block_name_grammar_round_trips() {
        assert_eq!(block_name(0), "000000.blk");
        assert_eq!(block_name(831), "000831.blk");
        assert_eq!(parse_block_name("000831.blk"), Some(831));
        assert_eq!(parse_block_name("000831.blk.part"), None);
        assert_eq!(parse_block_name("x.blk"), None);
        assert_eq!(parse_block_name(".blk"), None);
        assert_eq!(parse_block_name("img.tfs.lazy.json"), None);
        // Wider-than-six indices still parse (the grammar is digits).
        assert_eq!(parse_block_name("1000000.blk"), Some(1_000_000));
    }

    #[test]
    fn descriptor_write_read_and_absent() {
        let dir = scratch("descriptor");
        let image = "tebako-runtime-0.16.6-4.0.6-aarch64-macos.tfs";
        // Absent: Ok(None) — the entry is not LAZY_SEEDING.
        assert_eq!(read_descriptor(&dir, image).unwrap(), None);
        let seed = seed();
        write_descriptor(&dir, image, &seed).unwrap();
        // The commit landed; no tmp debris.
        assert!(descriptor_path(&dir, image).is_file());
        assert!(!dir
            .join(format!("{}.part", descriptor_name(image)))
            .exists());
        assert_eq!(read_descriptor(&dir, image).unwrap(), Some(seed));
        // Present but invalid: the named error, never a guess.
        std::fs::write(descriptor_path(&dir, image), "{{").unwrap();
        let err = read_descriptor(&dir, image).unwrap_err();
        assert!(
            matches!(err, LazyError::LazyDescriptorInvalid(_)),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_blocks_is_the_block_map() {
        let dir = scratch("blocks");
        let blocks = dir.join("img.tfs.blocks");
        std::fs::create_dir_all(&blocks).unwrap();
        // A 2-group-plus-tail image: groups 0 and 2 seeded.
        let size = 2 * LAZY_GROUP_SIZE + 7;
        std::fs::write(
            blocks.join(block_name(0)),
            vec![0u8; LAZY_GROUP_SIZE as usize],
        )
        .unwrap();
        std::fs::write(blocks.join(block_name(2)), vec![0u8; 7]).unwrap();
        // Debris: a crashed write, a size-mismatched group, an
        // off-grammar name, an index past the count.
        std::fs::write(blocks.join(block_part_name(1)), b"half").unwrap();
        std::fs::write(blocks.join(block_name(1)), b"short").unwrap();
        std::fs::write(blocks.join("notes.txt"), b"x").unwrap();
        std::fs::write(blocks.join(block_name(3)), vec![0u8; 7]).unwrap();

        let present = scan_blocks(&blocks, size).unwrap();
        assert_eq!(present, vec![0, 2]);
        // The debris is gone (dropped so the next fetch reinstalls).
        assert!(!blocks.join(block_part_name(1)).exists());
        assert!(!blocks.join(block_name(1)).exists());
        assert!(!blocks.join("notes.txt").exists());
        assert!(!blocks.join(block_name(3)).exists());
        // The seeded groups survive.
        assert!(blocks.join(block_name(0)).is_file());
        assert!(blocks.join(block_name(2)).is_file());
        // A missing directory is the empty set (a fresh LAZY_SEEDING).
        assert_eq!(
            scan_blocks(&dir.join("nope"), size).unwrap(),
            Vec::<u64>::new()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -------------------------------------------------------------
    // property: garbage never panics, never parses silently
    // -------------------------------------------------------------

    #[test]
    fn proptest_garbage_never_panics() {
        use proptest::prelude::*;
        proptest!(|(text in ".*")| {
            // Either a clean named error or — vanishingly — a valid
            // document; never a panic, never a half-parse.
            let _ = Blksum::parse(&text);
            let _ = LazySeed::parse(&text);
            let _ = parse_block_name(&text);
        });
    }
}
