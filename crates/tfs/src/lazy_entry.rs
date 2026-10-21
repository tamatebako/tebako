//! The LAZY_SEEDING store entry's mount plumbing (spec 39 §4/§5/§6) —
//! the ONE owner of "read the on-disk state, open the byte source".
//!
//! Two consumers must rebuild the same lazy mount over the same store
//! entry, in different processes and without sharing an address space:
//!
//! - the driver, inside the runtime exe (`tebako-driver`), which mounts
//!   the env image for the boot and starts the background seal thread;
//! - the preload shim (`libtfs-preload`), which re-establishes the mount
//!   from `TEBAKO_TFS_MOUNTS` in the interpreter's own engine instance
//!   (and in every exec'd child) — the shipped defect tebako#848: the
//!   re-mount knew only whole files, so a lazy entry booted the
//!   interpreter with no runtime root at all.
//!
//! The transport stays the caller's (the byte layer is
//! transport-shaped, not URL-shaped — a [`LazyTransport`] is a range
//! fetch plus a small-document GET); everything above it — the state
//! machine, the cached-blksum arm, the offline arm, the source
//! construction — lives here so the two consumers cannot drift.

use std::path::PathBuf;
use std::sync::Arc;

use tpkg::lazy::{Blksum, LazyError, LazySeed};

use crate::source::SourceError;
use crate::source_remote::{RangeFetchAnswer, RemoteByteSource};

/// The handoff path's on-disk state (spec 39 §9: the path names the
/// entry; the consumer reads the state).
#[derive(Debug)]
pub enum EntryState {
    /// The sealed image file is present — the ordinary whole-file mount.
    Sealed,
    /// LAZY_SEEDING: the image is absent and the descriptor beside it
    /// validates.
    Seeding {
        /// The store entry (the descriptor's directory).
        entry_dir: PathBuf,
        /// The image's file name within the entry.
        image_base: String,
        /// The validated seed descriptor.
        seed: LazySeed,
    },
    /// Neither file nor descriptor — the caller's ordinary whole-file
    /// error path fires (the named unavailable, unchanged).
    Absent,
}

/// State-detect an image path. A PRESENT but torn descriptor is the
/// named 65 (never a guess, never a silent heal); store IO keeps its 74.
pub fn entry_state(image_path: &str) -> Result<EntryState, LazyError> {
    let path = std::path::Path::new(image_path);
    if path.is_file() {
        return Ok(EntryState::Sealed);
    }
    let (Some(entry_dir), Some(image_base)) =
        (path.parent(), path.file_name().and_then(|n| n.to_str()))
    else {
        return Ok(EntryState::Absent);
    };
    match tpkg::lazy::read_descriptor(entry_dir, image_base)? {
        Some(seed) => Ok(EntryState::Seeding {
            entry_dir: entry_dir.to_path_buf(),
            image_base: image_base.to_string(),
            seed,
        }),
        None => Ok(EntryState::Absent),
    }
}

/// `TEBAKO_OFFLINE` truthiness — the store convention's spelling
/// (`1|true|yes`, case-insensitive; tebako-resolve's `cache::offline`
/// owns the convention).
pub fn offline(value: Option<&str>) -> bool {
    value
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

/// The transport a lazy mount needs from its consumer: the group range
/// fetch (the caching source's unit) and one small document GET (the
/// blksum sidecar). Both return the transport's named cause on failure;
/// the source and the sidecar arm own the verification.
pub trait LazyTransport: Send + Sync {
    /// Fetch `[offset, offset + len)` of the image.
    fn range(
        &self,
        offset: u64,
        len: usize,
        if_range: Option<&str>,
    ) -> Result<RangeFetchAnswer, String>;

    /// Fetch one small document (the `<source>.blksum.json` sidecar).
    fn document(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// Open the lazy mount's byte source for a validated seed. Offline: the
/// CACHE-ONLY arm — no sidecar fetch, cached groups serve, a miss is EIO
/// on the touching read (spec 39 §6's offline at group granularity).
/// Online: the blksum anchors first (the cached-blksum arm is consulted
/// BEFORE the wire — spec 39 §3), then the source serves groups on
/// demand through `transport`.
pub fn open_source(
    entry_dir: &std::path::Path,
    image_base: &str,
    seed: &LazySeed,
    offline: bool,
    transport: Arc<dyn LazyTransport>,
) -> Result<Arc<RemoteByteSource>, SourceError> {
    let blocks = tpkg::lazy::blocks_dir(entry_dir, image_base);
    if offline {
        return RemoteByteSource::new_cache_only(seed.size_bytes, &blocks, &seed.source)
            .map(Arc::new);
    }
    let blksum = fetch_blksum(entry_dir, image_base, seed, transport.as_ref())?;
    let fetch = {
        let transport = Arc::clone(&transport);
        move |offset: u64, len: usize, if_range: Option<&str>| {
            transport.range(offset, len, if_range)
        }
    };
    RemoteByteSource::new(Arc::new(fetch), blksum, &blocks, &seed.source).map(Arc::new)
}

/// Fetch and anchor the blksum sidecar (spec 39 §3): the cached copy is
/// consulted FIRST — a verified hit makes the mount-open zero-network. A
/// miss fetches through `transport` and writes through (best-effort: the
/// verified blksum is already in hand, so a cache write failure never
/// fails the mount); a present-but-off-pin cache is the named error,
/// never a silent drop-and-refetch.
fn fetch_blksum(
    entry_dir: &std::path::Path,
    image_base: &str,
    seed: &LazySeed,
    transport: &dyn LazyTransport,
) -> Result<Blksum, SourceError> {
    if let Some(sum) = tpkg::lazy::read_blksum_cache(entry_dir, image_base, seed)
        .map_err(|e| lazy_error(e, None))?
    {
        return Ok(sum);
    }
    let url = format!("{}.blksum.json", seed.source);
    let body = transport
        .document(&url)
        .map_err(|e| SourceError::fetch(format!("{url}: {e}")))?;
    let sum = tpkg::lazy::verify_blksum(seed, &body).map_err(|e| lazy_error(e, Some(&url)))?;
    let _ = tpkg::lazy::write_blksum_cache(entry_dir, image_base, &body);
    Ok(sum)
}

/// A `tpkg::lazy` failure's byte-layer class (spec 39 §6's table),
/// optionally naming the document the failure came from.
fn lazy_error(e: LazyError, at: Option<&str>) -> SourceError {
    let mut detail = e.to_string();
    if let Some(at) = at {
        detail = format!("{at}: {detail}");
    }
    match e {
        LazyError::LazyDescriptorInvalid(_) | LazyError::BlksumInvalid(_) => {
            SourceError::invalid(detail)
        }
        LazyError::Sha256Mismatch(_) => SourceError::integrity(detail),
        LazyError::LazyUnavailable(_) => SourceError::fetch(detail),
        LazyError::LazyStoreIo(_) => SourceError::io(detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_states_and_the_torn_descriptor() {
        let tmp = tempfile::tempdir().unwrap();
        let image = tmp.path().join("img.tfs");
        let image = image.to_str().unwrap();
        assert!(matches!(entry_state(image).unwrap(), EntryState::Absent));
        let seed = LazySeed {
            source: "file:///x.tfs".to_string(),
            sha256: "a".repeat(64),
            blksum_sha256: "b".repeat(64),
            size_bytes: tpkg::lazy::LAZY_GROUP_SIZE,
            group_count: 1,
        };
        tpkg::lazy::write_descriptor(tmp.path(), "img.tfs", &seed).unwrap();
        match entry_state(image).unwrap() {
            EntryState::Seeding { seed: got, .. } => assert_eq!(got, seed),
            other => panic!("expected Seeding, got {other:?}"),
        }
        std::fs::write(image, b"x").unwrap();
        assert!(matches!(entry_state(image).unwrap(), EntryState::Sealed));
        std::fs::remove_file(image).unwrap();
        std::fs::write(tmp.path().join("img.tfs.lazy.json"), b"not json").unwrap();
        let err = entry_state(image).unwrap_err();
        assert_eq!(err.exit_code(), 65, "{err}");
    }

    #[test]
    fn the_offline_spelling_is_the_store_convention() {
        assert!(!offline(None));
        assert!(offline(Some("YES")));
        assert!(offline(Some("1")));
        assert!(!offline(Some("0")));
        assert!(!offline(Some("maybe")));
    }
}
