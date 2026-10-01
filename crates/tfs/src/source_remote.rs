//! The caching remote byte source (spec 39 §3/§4) — the
//! `backend-remote` feature's core. A [`ByteSource`] whose bytes live
//! behind a range-fetch closure, seeded into the runtime store entry's
//! block directory group by group (the locked 4 MiB fetch unit), every
//! fetched group sha256-verified against the publisher-anchored blksum
//! sidecar BEFORE it enters the cache and before any byte is served.
//!
//! The trust law (spec 39 §3, applied): a group present in the block
//! directory is verified by construction and serves WITHOUT
//! re-verification — the cache is trusted once written, exactly like
//! today's sealed entries ("verification at fetch/install, never per
//! run"). A group whose digest mismatches is dropped, refetched once,
//! and on a second mismatch is the named integrity failure — the mount
//! fails closed, never a silent serve.
//!
//! The crash law (spec 39 §4): fetch → verify → write `<NNNNNN>.blk.part`
//! → rename, under this source's install lock (the entry's flock is
//! the loader's, one layer up). A `.part` is never a seeded group; the
//! next open drops it (`tpkg::lazy::scan_blocks`).
//!
//! Transport-shaped, not URL-shaped (spec 39 §8): the source consumes
//! a [`RangeFetch`] closure — tebako-http's `get_range` today (the
//! caller adapts it, If-Range/ETag semantics included), an OCI blob
//! GET closure later, a test mock in the unit tiers. HTTPS-only and
//! the loopback/file carve-outs are the transport's enforcement, one
//! layer down — there is no insecure-source spelling here because
//! there is no URL here at all.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tpkg::lazy::{block_name, block_part_name, scan_blocks, sha256_hex, Blksum};

use crate::source::{ByteSource, SourceError};

/// One range fetch's answer, mirroring the transport's 206-vs-200
/// distinction (spec 39 §3's fallback signal): the requested window,
/// or the WHOLE representation (the server ignored the Range header,
/// or an `If-Range` validator mismatch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeFetchAnswer {
    /// The requested window (a 206; `etag` is the validator the
    /// response carried — the caller's later `If-Range` input).
    Partial {
        /// The window bytes (exactly the requested length).
        bytes: Vec<u8>,
        /// The response's ETag, when present.
        etag: Option<String>,
    },
    /// The whole representation (a 200) — the loud-eager-fallback
    /// signal. Never silently treated as the window.
    Full {
        /// The whole object's bytes.
        bytes: Vec<u8>,
        /// The response's ETag, when present.
        etag: Option<String>,
    },
}

/// The range-fetch closure the source consumes (spec 39 §8's closure
/// shape): `fetch(offset, len, if_range)` → the window or the whole
/// body. `if_range` carries the validator held from an earlier answer
/// (the transport answers a mismatch with `Full` — never stale window
/// bytes). The error is the transport's named cause, surfaced verbatim
/// in the source's log lines.
///
/// A blanket impl covers closures, so callers adapt any fetch surface
/// (`tebako_http::get_range` first) without a newtype.
pub trait RangeFetch: Send + Sync {
    /// Fetch `[offset, offset + len)` of the object.
    fn fetch(
        &self,
        offset: u64,
        len: usize,
        if_range: Option<&str>,
    ) -> Result<RangeFetchAnswer, String>;
}

impl<F> RangeFetch for F
where
    F: Fn(u64, usize, Option<&str>) -> Result<RangeFetchAnswer, String> + Send + Sync,
{
    fn fetch(
        &self,
        offset: u64,
        len: usize,
        if_range: Option<&str>,
    ) -> Result<RangeFetchAnswer, String> {
        self(offset, len, if_range)
    }
}

/// A [`ByteSource`] over a range-fetch closure with the spec 39 §4
/// block cache: verified group payloads under `<image>.blocks/`,
/// seeded on demand.
pub struct RemoteByteSource {
    /// The range transport (a closure — tebako-http's `get_range`
    /// adapted by the caller). `None` on a cache-only source (the
    /// TEBAKO_OFFLINE=1 arm): fetches are FORBIDDEN without the digest
    /// table — fail-closed, offline at group granularity (spec 39 §6).
    fetch: Option<Arc<dyn RangeFetch>>,
    /// The verified blksum sidecar (the group digests + the
    /// whole-image sha256 the Full fallback verifies against). `None`
    /// on a cache-only source — cached groups serve unverified (the
    /// per-run law: the cache is trusted once written).
    blksum: Option<Blksum>,
    /// The whole image's byte length (the blksum's, or the
    /// descriptor's on a cache-only source).
    size_bytes: u64,
    /// The entry's `<image>.blocks` directory.
    blocks: PathBuf,
    /// The origin's display name for log lines (a URL, a digest —
    /// whatever the caller serves; the source itself never parses it).
    origin: String,
    /// The validator held from the last answer (the next fetch's
    /// `If-Range`).
    etag: Mutex<Option<String>>,
    /// Serializes group installs: demand reads re-check the cache
    /// under the lock (the store's re-check-under-lock discipline) so
    /// one group's fetch happens once no matter how many reads touch
    /// it. v1's scheduling law (on-demand preempts the seal) is the
    /// driver's, one layer up.
    install_lock: Mutex<()>,
}

impl RemoteByteSource {
    /// Open the source over `blocks` (created when missing): the
    /// directory scan drops crash debris and size-mismatched groups
    /// (`tpkg::lazy::scan_blocks` — the block map IS the directory).
    /// `blksum` arrives ALREADY verified against its pin (the loader's
    /// step, spec 39 §3 — a mismatch there is `Sha256Mismatch` before
    /// any source exists).
    pub fn new(
        fetch: Arc<dyn RangeFetch>,
        blksum: Blksum,
        blocks: &Path,
        origin: impl Into<String>,
    ) -> Result<RemoteByteSource, SourceError> {
        let size_bytes = blksum.size_bytes;
        let mut source = RemoteByteSource::new_cache_only(size_bytes, blocks, origin)?;
        source.fetch = Some(fetch);
        source.blksum = Some(blksum);
        Ok(source)
    }

    /// Open a CACHE-ONLY source over `blocks` (spec 39 §6's
    /// TEBAKO_OFFLINE=1 arm): cached groups serve (trusted once
    /// written, unverified per run); a miss is the named fetch failure
    /// on the touching read — never a fetch without the digest table,
    /// never a fabricated zero-fill.
    pub fn new_cache_only(
        size_bytes: u64,
        blocks: &Path,
        origin: impl Into<String>,
    ) -> Result<RemoteByteSource, SourceError> {
        let origin = origin.into();
        std::fs::create_dir_all(blocks)
            .map_err(|e| SourceError::io(format!("create {}: {e}", blocks.display())))?;
        // Drop the debris of a crashed previous run up front (the scan
        // re-validates on every open regardless — this keeps the
        // directory tight).
        let _ =
            scan_blocks(blocks, size_bytes).map_err(|e| SourceError::io(e.to_string()))?;
        Ok(RemoteByteSource {
            fetch: None,
            blksum: None,
            size_bytes,
            blocks: blocks.to_path_buf(),
            origin,
            etag: Mutex::new(None),
            install_lock: Mutex::new(()),
        })
    }

    /// The seeded set (sorted group indices) — the progress surface's
    /// data (`sealing <image> … groups 831/1340`, spec 39 §7).
    pub fn seeded_groups(&self) -> Result<Vec<u64>, SourceError> {
        scan_blocks(&self.blocks, self.size_bytes).map_err(|e| SourceError::io(e.to_string()))
    }

    /// The missing group indices, in index order (the seal thread's
    /// work list — it fills gaps around demand, spec 39 §5).
    pub fn missing_groups(&self) -> Result<Vec<u64>, SourceError> {
        let present = self.seeded_groups()?;
        let mut cursor = 0u64;
        let mut out = Vec::new();
        for &g in &present {
            while cursor < g {
                out.push(cursor);
                cursor += 1;
            }
            cursor = g + 1;
        }
        let count = self.group_count();
        while cursor < count {
            out.push(cursor);
            cursor += 1;
        }
        Ok(out)
    }

    /// Seed group `index` on demand — the seal thread's fetch unit
    /// (spec 39 §5). Identical to a touching read's seed: re-check
    /// under the install lock (an on-demand read preempts), fetch →
    /// verify → install. On a cache-only source it is the named
    /// failure — the seal thread is never spawned there.
    pub fn seed_group(&self, index: u64) -> Result<(), SourceError> {
        self.ensure_group(index)
    }

    /// The whole image's byte length.
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// The group count.
    pub fn group_count(&self) -> u64 {
        match &self.blksum {
            Some(blksum) => blksum.group_count(),
            None => tpkg::lazy::group_count_for(self.size_bytes),
        }
    }

    /// One cached group file's path.
    fn block_path(&self, index: u64) -> PathBuf {
        self.blocks.join(block_name(index))
    }

    /// Is group `index` seeded? (Presence + exact size — the scan's
    /// validity law at one index, without the directory walk.)
    fn group_cached(&self, index: u64) -> bool {
        let Some((_, want_len)) = tpkg::lazy::group_span(self.size_bytes, index) else {
            return false;
        };
        std::fs::metadata(self.block_path(index))
            .map(|m| m.is_file() && m.len() == want_len)
            .unwrap_or(false)
    }

    /// Record an answer's validator for the next fetch's `If-Range`.
    fn learn_etag(&self, etag: Option<String>) {
        if etag.is_some() {
            *self.etag.lock().unwrap_or_else(|e| e.into_inner()) = etag;
        }
    }

    /// The held validator.
    fn held_etag(&self) -> Option<String> {
        self.etag.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Write one verified group: tmp (`<NNNNNN>.blk.part`) → rename,
    /// read-only (0444 — the store's read-only-artifacts rule).
    fn install_group(&self, index: u64, bytes: &[u8]) -> Result<(), SourceError> {
        let part = self.blocks.join(block_part_name(index));
        let final_path = self.block_path(index);
        std::fs::write(&part, bytes)
            .map_err(|e| SourceError::io(format!("write {}: {e}", part.display())))?;
        let mut perms = std::fs::metadata(&part)
            .map_err(|e| SourceError::io(format!("stat {}: {e}", part.display())))?
            .permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&part, perms)
            .map_err(|e| SourceError::io(format!("chmod {}: {e}", part.display())))?;
        std::fs::rename(&part, &final_path).map_err(|e| {
            SourceError::io(format!(
                "rename {} -> {}: {e}",
                part.display(),
                final_path.display()
            ))
        })?;
        Ok(())
    }

    /// One fetch of group `index`'s span (the transport's retry law
    /// rides inside the closure — a group is retried FROM ZERO, never
    /// resumed mid-body, spec 39 §6).
    fn fetch_group_once(
        &self,
        fetch: &dyn RangeFetch,
        index: u64,
    ) -> Result<RangeFetchAnswer, SourceError> {
        let Some((offset, len)) = tpkg::lazy::group_span(self.size_bytes, index) else {
            return Err(SourceError::fetch(format!(
                "group {index} is past the image's group count {}",
                self.group_count()
            )));
        };
        let len = usize::try_from(len)
            .map_err(|_| SourceError::fetch(format!("group {index} length exceeds usize")))?;
        let held = self.held_etag();
        fetch
            .fetch(offset, len, held.as_deref())
            .map_err(|why| SourceError::fetch(format!("group {index} of {}: {why}", self.origin)))
    }

    /// Verify fetched window bytes against the group's sidecar digest.
    fn group_verified(&self, blksum: &Blksum, index: u64, bytes: &[u8]) -> bool {
        sha256_hex(bytes) == blksum.groups[index as usize]
    }

    /// Ensure group `index` is seeded: serve from the cache when
    /// present (NO re-verification — the per-run law), else fetch →
    /// verify → install. A digest mismatch drops the bytes and
    /// refetches ONCE; a second mismatch is the named integrity
    /// failure (spec 39 §3 — fail closed, never a silent serve). A
    /// `Full` answer takes the loud eager fallback: the whole body
    /// verifies against the whole-image pin and seeds EVERY missing
    /// group (equal trust — every byte digest-verified; loud — warned
    /// on the log). On a cache-only source a miss is the named fetch
    /// failure (spec 39 §6's offline at group granularity) — named on
    /// the tebako-log with origin, group index, and cause either way.
    fn ensure_group(&self, index: u64) -> Result<(), SourceError> {
        if self.group_cached(index) {
            return Ok(());
        }
        let _guard = self.install_lock.lock().unwrap_or_else(|e| e.into_inner());
        // Re-check under the lock: a concurrent read may have seeded
        // this group while we waited.
        if self.group_cached(index) {
            return Ok(());
        }
        let (Some(fetch), Some(blksum)) = (&self.fetch, &self.blksum) else {
            let err = SourceError::fetch(format!(
                "group {index} of {} is not cached and this source is cache-only (offline at group granularity, spec 39 §6)",
                self.origin
            ));
            tebako_log::log!(
                tebako_log::Level::Warn,
                "tfs",
                "lazy: {err}"
            );
            return Err(err);
        };
        let answer = match self.fetch_group_once(fetch.as_ref(), index) {
            Ok(answer) => answer,
            Err(err) => {
                tebako_log::log!(
                    tebako_log::Level::Warn,
                    "tfs",
                    "lazy: fetch failed: {err}"
                );
                return Err(err);
            }
        };
        self.seed_from_answer(fetch.as_ref(), blksum, index, answer)
    }

    /// Seed group `index` from one fetch answer: a `Partial` window
    /// verifies against its sidecar digest (a mismatch refetches ONCE;
    /// a second is the named integrity failure — fail closed, never a
    /// silent serve); a `Full` answer takes the loud eager fallback.
    fn seed_from_answer(
        &self,
        fetch: &dyn RangeFetch,
        blksum: &Blksum,
        index: u64,
        answer: RangeFetchAnswer,
    ) -> Result<(), SourceError> {
        match answer {
            RangeFetchAnswer::Partial { bytes, etag } => {
                self.learn_etag(etag);
                let (_, want_len) =
                    tpkg::lazy::group_span(self.size_bytes, index)
                        .expect("an in-range group spans");
                if bytes.len() as u64 != want_len {
                    return Err(SourceError::fetch(format!(
                        "group {index} of {} arrived truncated ({} of {want_len} bytes)",
                        self.origin,
                        bytes.len()
                    )));
                }
                if self.group_verified(blksum, index, &bytes) {
                    return self.install_group(index, &bytes);
                }
                // First mismatch: drop, refetch once (spec 39 §3).
                tebako_log::log!(
                    tebako_log::Level::Warn,
                    "tfs",
                    "lazy: group {index} of {} failed its sidecar digest — refetching once",
                    self.origin
                );
                match self.fetch_group_once(fetch, index)? {
                    RangeFetchAnswer::Partial { bytes, etag } => {
                        self.learn_etag(etag);
                        if bytes.len() as u64 == want_len
                            && self.group_verified(blksum, index, &bytes)
                        {
                            return self.install_group(index, &bytes);
                        }
                        Err(SourceError::integrity(format!(
                            "group {index} of {} failed its sidecar digest on the refetch — the mount fails closed",
                            self.origin
                        )))
                    }
                    RangeFetchAnswer::Full { bytes, etag } => {
                        self.learn_etag(etag);
                        self.eager_seed(blksum, bytes, index)
                    }
                }
            }
            RangeFetchAnswer::Full { bytes, etag } => {
                self.learn_etag(etag);
                self.eager_seed(blksum, bytes, index)
            }
        }
    }

    /// The loud eager fallback (spec 39 §3's 200-answer law, one layer
    /// down): the whole body arrived — verify it against the
    /// whole-image pin (EQUAL trust strength, no downgrade), seed
    /// every missing group from it (each slice re-verified against its
    /// sidecar digest — a disagreement between a verified whole and
    /// the sidecar is evidence of a publisher-side fault and is
    /// reported, never healed), and warn LOUDLY.
    fn eager_seed(&self, blksum: &Blksum, bytes: Vec<u8>, touched: u64) -> Result<(), SourceError> {
        if sha256_hex(&bytes) != blksum.sha256 {
            return Err(SourceError::integrity(format!(
                "the whole body of {} does not match the whole-image sha256 pin — the fallback refuses it",
                self.origin
            )));
        }
        tebako_log::log!(
            tebako_log::Level::Warn,
            "tfs",
            "lazy: {} answered a Range GET with the whole body — seeding eagerly (the loud eager fallback, spec 39 §3)",
            self.origin
        );
        for index in 0..blksum.group_count() {
            if self.group_cached(index) {
                continue;
            }
            let (offset, len) = tpkg::lazy::group_span(self.size_bytes, index)
                .expect("an in-range group spans");
            let slice = &bytes[offset as usize..(offset + len) as usize];
            if !self.group_verified(blksum, index, slice) {
                return Err(SourceError::integrity(format!(
                    "group {index} of the verified whole body of {} disagrees with its sidecar digest — a publisher-side fault, reported never healed",
                    self.origin
                )));
            }
            self.install_group(index, slice)?;
        }
        debug_assert!(
            self.group_cached(touched),
            "the eager seed installed the touching group"
        );
        Ok(())
    }
}

impl ByteSource for RemoteByteSource {
    /// Serve `[offset, offset + len)` (short only at the image's end):
    /// the covering groups seed on demand, then the window assembles
    /// from the cached group files. A group that cannot be seeded is
    /// the named failure on THIS read — the mount and every cached
    /// group remain valid; the next read resumes (spec 39 §6).
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, SourceError> {
        let size = self.size_bytes;
        if len == 0 || offset >= size {
            return Ok(Vec::new());
        }
        let end = (offset + len as u64).min(size);
        let first = offset / tpkg::lazy::LAZY_GROUP_SIZE;
        let last = (end - 1) / tpkg::lazy::LAZY_GROUP_SIZE;
        for index in first..=last {
            self.ensure_group(index)?;
        }
        let mut out = Vec::with_capacity((end - offset) as usize);
        for index in first..=last {
            let group_offset = index * tpkg::lazy::LAZY_GROUP_SIZE;
            let file = std::fs::read(self.block_path(index)).map_err(|e| {
                SourceError::io(format!(
                    "read the seeded {}: {e}",
                    self.block_path(index).display()
                ))
            })?;
            let lo = (offset.max(group_offset) - group_offset) as usize;
            let hi = (end.min(group_offset + tpkg::lazy::LAZY_GROUP_SIZE) - group_offset) as usize;
            if hi > file.len() {
                // The seeded file shrank under us (an operator's hand
                // in the store): refuse, never zero-fill.
                return Err(SourceError::io(format!(
                    "the seeded {} is shorter than its group span",
                    self.block_path(index).display()
                )));
            }
            out.extend_from_slice(&file[lo..hi]);
        }
        Ok(out)
    }

    fn len(&self) -> Option<u64> {
        Some(self.size_bytes)
    }
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic bytes: 2.5 groups.
    const SIZE: u64 = 2 * tpkg::lazy::LAZY_GROUP_SIZE + tpkg::lazy::LAZY_GROUP_SIZE / 2;

    fn image_bytes() -> Vec<u8> {
        (0..SIZE as usize)
            .map(|i| ((i * 2654435761) >> 13 & 0xFF) as u8)
            .collect()
    }

    /// A mock range transport over owned bytes: pread semantics, an
    /// ETag, a request log, and an optional corruption plan (group
    /// index → how many fetches to corrupt before serving clean).
    struct Mock {
        bytes: Vec<u8>,
        requests: Mutex<Vec<(u64, usize, Option<String>)>>,
        corrupt: Mutex<std::collections::HashMap<u64, usize>>,
        whole_only: bool,
    }

    impl Mock {
        fn new(bytes: Vec<u8>) -> Mock {
            Mock {
                bytes,
                requests: Mutex::new(Vec::new()),
                corrupt: Mutex::new(std::collections::HashMap::new()),
                whole_only: false,
            }
        }

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        fn fetch_impl(
            &self,
            offset: u64,
            len: usize,
            if_range: Option<&str>,
        ) -> Result<RangeFetchAnswer, String> {
            self.requests
                .lock()
                .unwrap()
                .push((offset, len, if_range.map(str::to_string)));
            if self.whole_only {
                return Ok(RangeFetchAnswer::Full {
                    bytes: self.bytes.clone(),
                    etag: Some("\"v1\"".to_string()),
                });
            }
            let end = (offset + len as u64).min(self.bytes.len() as u64) as usize;
            let mut window = self.bytes[offset as usize..end].to_vec();
            let group = offset / tpkg::lazy::LAZY_GROUP_SIZE;
            let mut corrupt = self.corrupt.lock().unwrap();
            if let Some(remaining) = corrupt.get_mut(&group) {
                if *remaining > 0 {
                    *remaining -= 1;
                    let at = window.len() / 2;
                    window[at] ^= 0xFF;
                }
            }
            Ok(RangeFetchAnswer::Partial {
                bytes: window,
                etag: Some("\"v1\"".to_string()),
            })
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tfs-lazy-remote-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn source(mock: Arc<Mock>, blocks: &Path) -> RemoteByteSource {
        let sum = Blksum::from_image_bytes(&mock.bytes);
        let fetch = Arc::new(move |o: u64, l: usize, v: Option<&str>| mock.fetch_impl(o, l, v));
        RemoteByteSource::new(fetch, sum, blocks, "mock://image.tfs").unwrap()
    }

    #[test]
    fn windows_spanning_groups_answer_the_original_bytes() {
        let dir = scratch("windows");
        let mock = Arc::new(Mock::new(image_bytes()));
        let src = source(Arc::clone(&mock), &dir);
        // A window inside one group.
        let got = src.read_at(100, 4096).unwrap();
        assert_eq!(got, &mock.bytes[100..4196]);
        // A window spanning all three groups (incl. the short tail).
        let start = tpkg::lazy::LAZY_GROUP_SIZE - 10;
        let got = src.read_at(start, (SIZE - start) as usize).unwrap();
        assert_eq!(got, &mock.bytes[start as usize..]);
        // EOF clamp + past-end empty.
        assert_eq!(src.read_at(SIZE, 10).unwrap(), Vec::<u8>::new());
        assert_eq!(
            src.read_at(SIZE - 5, 10).unwrap(),
            &mock.bytes[(SIZE - 5) as usize..]
        );
        // All three groups seeded with exactly three fetches.
        assert_eq!(mock.request_count(), 3);
        assert_eq!(src.seeded_groups().unwrap(), vec![0, 1, 2]);
        // A re-read is served from the cache — no new fetch (the
        // per-run law: the cache is trusted once written).
        let got = src.read_at(100, 4096).unwrap();
        assert_eq!(got, &mock.bytes[100..4196]);
        assert_eq!(mock.request_count(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_group_digest_mismatch_refetches_once_then_serves() {
        let dir = scratch("refetch");
        let mock = Arc::new(Mock::new(image_bytes()));
        mock.corrupt.lock().unwrap().insert(1, 1); // corrupt group 1 once
        let src = source(Arc::clone(&mock), &dir);
        let got = src.read_at(tpkg::lazy::LAZY_GROUP_SIZE, 128).unwrap();
        assert_eq!(
            got,
            &mock.bytes[tpkg::lazy::LAZY_GROUP_SIZE as usize..][..128]
        );
        // Two fetches of the same span (the refetch-once law).
        let requests = mock.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].0, requests[1].0);
        drop(requests);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_digest_mismatch_is_the_named_integrity_failure() {
        let dir = scratch("integrity");
        let mock = Arc::new(Mock::new(image_bytes()));
        mock.corrupt.lock().unwrap().insert(0, 2); // corrupt group 0 twice
        let src = source(Arc::clone(&mock), &dir);
        let err = src.read_at(0, 128).unwrap_err();
        assert_eq!(err.kind, crate::source::SourceErrorKind::Integrity);
        assert!(err.detail.contains("fails closed"), "{err}");
        // Nothing seeded: the group file never landed.
        assert_eq!(src.seeded_groups().unwrap(), Vec::<u64>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_if_range_validator_rides_the_next_fetch() {
        let dir = scratch("if-range");
        let mock = Arc::new(Mock::new(image_bytes()));
        let src = source(Arc::clone(&mock), &dir);
        let _ = src.read_at(0, 8).unwrap();
        let _ = src.read_at(tpkg::lazy::LAZY_GROUP_SIZE, 8).unwrap();
        let requests = mock.requests.lock().unwrap();
        assert_eq!(requests[0].2, None);
        assert_eq!(requests[1].2.as_deref(), Some("\"v1\""));
        drop(requests);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_whole_body_answer_seeds_eagerly_and_warns() {
        let dir = scratch("eager");
        let mut mock = Mock::new(image_bytes());
        mock.whole_only = true;
        let mock = Arc::new(mock);
        let src = source(Arc::clone(&mock), &dir);
        // One read: the whole body arrived once, every group seeded.
        let got = src.read_at(SIZE - 16, 16).unwrap();
        assert_eq!(got, &mock.bytes[(SIZE - 16) as usize..]);
        assert_eq!(mock.request_count(), 1);
        assert_eq!(src.seeded_groups().unwrap(), vec![0, 1, 2]);
        // Later reads are cache serves (no new fetches).
        let got = src.read_at(0, 4096).unwrap();
        assert_eq!(got, &mock.bytes[..4096]);
        assert_eq!(mock.request_count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_eager_fallback_refuses_a_body_off_the_pin() {
        let dir = scratch("eager-bad");
        let mut evil = image_bytes();
        evil[123] ^= 0xFF;
        let mut mock = Mock::new(evil);
        mock.whole_only = true;
        let mock = Arc::new(mock);
        // The blksum is computed over the CLEAN bytes (the pin the
        // loader anchored); the mock serves evil ones.
        let sum = Blksum::from_image_bytes(&image_bytes());
        let m = Arc::clone(&mock);
        let fetch = Arc::new(move |o: u64, l: usize, v: Option<&str>| m.fetch_impl(o, l, v));
        let src = RemoteByteSource::new(fetch, sum, &dir, "mock://evil.tfs").unwrap();
        let err = src.read_at(0, 8).unwrap_err();
        assert_eq!(err.kind, crate::source::SourceErrorKind::Integrity);
        assert!(err.detail.contains("whole-image sha256"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cache_only_source_serves_seeded_groups_and_names_the_miss() {
        let dir = scratch("cache-only");
        let bytes = image_bytes();
        let blocks = dir.join("blocks");
        // Seed group 1 through a fetching source, then open cache-only
        // (the TEBAKO_OFFLINE=1 arm).
        let mock = Arc::new(Mock::new(bytes.clone()));
        let src = source(Arc::clone(&mock), &blocks);
        let _ = src.read_at(tpkg::lazy::LAZY_GROUP_SIZE, 128).unwrap();
        drop(src);
        let requests_before = mock.request_count();
        let offline = RemoteByteSource::new_cache_only(SIZE, &blocks, "mock://image.tfs").unwrap();
        assert_eq!(offline.size_bytes(), SIZE);
        assert_eq!(offline.group_count(), 3);
        // The seeded group serves without a fetch and without
        // re-verification (the per-run law: trusted once written).
        let got = offline.read_at(tpkg::lazy::LAZY_GROUP_SIZE, 128).unwrap();
        assert_eq!(
            got,
            &bytes[tpkg::lazy::LAZY_GROUP_SIZE as usize..][..128]
        );
        assert_eq!(mock.request_count(), requests_before);
        // A miss is the named fetch failure — never a fetch without
        // the digest table, never a zero-fill.
        let err = offline.read_at(0, 8).unwrap_err();
        assert_eq!(err.kind, crate::source::SourceErrorKind::Fetch);
        assert!(err.detail.contains("cache-only"), "{err}");
        // seed_group on a cache-only source names the same refusal
        // (the seal thread is never spawned there).
        let err = offline.seed_group(2).unwrap_err();
        assert_eq!(err.kind, crate::source::SourceErrorKind::Fetch);
        assert_eq!(mock.request_count(), requests_before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_groups_is_the_seeded_set_complement() {
        let dir = scratch("missing");
        let mock = Arc::new(Mock::new(image_bytes()));
        let src = source(Arc::clone(&mock), &dir);
        assert_eq!(src.missing_groups().unwrap(), vec![0, 1, 2]);
        // Group 1 seeds via a touching read…
        let _ = src.read_at(tpkg::lazy::LAZY_GROUP_SIZE, 8).unwrap();
        assert_eq!(src.missing_groups().unwrap(), vec![0, 2]);
        // …group 2 via the short tail…
        let _ = src.read_at(SIZE - 8, 8).unwrap();
        assert_eq!(src.missing_groups().unwrap(), vec![0]);
        // …and group 0 via the seal thread's unit.
        src.seed_group(0).unwrap();
        assert_eq!(src.missing_groups().unwrap(), Vec::<u64>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn proptest_arbitrary_windows_match_the_original_bytes() {
        use proptest::prelude::*;
        proptest!(|(start in 0..SIZE, len in 1usize..65536)| {
            let dir = scratch("prop");
            let mock = Arc::new(Mock::new(image_bytes()));
            let src = source(mock, &dir);
            let got = src.read_at(start, len).unwrap();
            let end = (start + len as u64).min(SIZE) as usize;
            let bytes = image_bytes();
            assert_eq!(got, &bytes[start as usize..end]);
            let _ = std::fs::remove_dir_all(&dir);
        });
    }
}
