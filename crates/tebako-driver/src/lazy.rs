//! The LAZY_SEEDING env-image mount (spec 39 §4/§5/§6): the driver
//! state-detects the handoff path's on-disk state — the sealed image
//! file present is today's whole-file mount; the image ABSENT with a
//! valid `<image>.lazy.json` descriptor beside it is the lazy mount:
//! a caching remote byte source over the descriptor's origin, mounted
//! through `tfs::mount::build_from_source`, with the background seal
//! thread filling the remaining groups and committing the sealed
//! entry. A pre-lazy driver fails CLOSED on the same entry (the image
//! is absent; the ordinary named unavailable error fires — spec 39
//! §9's locked interaction).
//!
//! The failure classes (spec 39 §6's table, the driver's own codes):
//! 65 — descriptor/blksum malformed, a malformed `TEBAKO_LAZY_SEAL`;
//! 69 — the mount-open sidecar fetch, a cache-only (offline) open, a
//! compiled-out `backend-remote`; 70 — the sidecar's pin or the
//! blksum/image-sha cross-check; 74 — block-cache IO. Mid-mount group
//! failures ride the errno channel (EIO on the touching read) — the
//! byte source names them on the tebako-log; this module never sees
//! them.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tfs::source::{SourceError, SourceErrorKind};
use tpkg::lazy::{Blksum, LazySeed, SealOutcome};

use crate::driver::DriverError;
use crate::{EX_TEBAKO_MANIFEST, EX_TEBAKO_UNAVAILABLE};

/// The background seal's operator switch (spec 39 §5):
/// `TEBAKO_LAZY_SEAL=0` disables the seal thread (seeding stays
/// on-demand only); a malformed value is the named 65. Not in tpkg's
/// SETTINGS table — a run-time operator knob, not a press setting (the
/// RUNTIME_LAZY precedent).
const LAZY_SEAL: tpkg::settings::Setting = tpkg::settings::Setting {
    config: None,
    env: Some("TEBAKO_LAZY_SEAL"),
    cli: None,
    doc: "disable the lazy env image's background seal thread (spec 39)",
};

/// The handoff path's on-disk state (spec 39 §9: the path names the
/// entry; the driver reads the state).
pub(crate) enum EnvImageState {
    /// The sealed image file is present — today's whole-file mount.
    Sealed,
    /// LAZY_SEEDING: the image is absent and the descriptor beside it
    /// validates.
    Seeding {
        /// The runtime store entry (the descriptor's directory).
        entry_dir: PathBuf,
        /// The env image's file name within the entry.
        image_base: String,
        /// The validated seed descriptor.
        seed: LazySeed,
    },
    /// Neither file nor descriptor — the caller's ordinary whole-file
    /// error path fires (the named unavailable, unchanged).
    Absent,
}

/// State-detect the env-image handoff path. A PRESENT but torn
/// descriptor is the named 65 (never a guess, never a silent heal —
/// the store scan's rule at mount time); store IO keeps its 74.
pub(crate) fn env_image_state(image_path: &str) -> Result<EnvImageState, DriverError> {
    let path = Path::new(image_path);
    if path.is_file() {
        return Ok(EnvImageState::Sealed);
    }
    let (Some(entry_dir), Some(image_base)) = (path.parent(), path.file_name().and_then(|n| n.to_str()))
    else {
        return Ok(EnvImageState::Absent);
    };
    match tpkg::lazy::read_descriptor(entry_dir, image_base) {
        Ok(Some(seed)) => Ok(EnvImageState::Seeding {
            entry_dir: entry_dir.to_path_buf(),
            image_base: image_base.to_string(),
            seed,
        }),
        Ok(None) => Ok(EnvImageState::Absent),
        Err(e) => Err(DriverError::new(
            e.exit_code(),
            format!("the lazy seed descriptor for '{image_path}': {e}"),
        )),
    }
}

/// `TEBAKO_OFFLINE` truthiness — the store convention's spelling
/// (`1|true|yes`, case-insensitive; tebako-resolve's `cache::offline`
/// owns the convention).
pub(crate) fn offline(value: Option<String>) -> bool {
    value
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

/// The background-seal resolution: default ON, `TEBAKO_LAZY_SEAL`
/// overrides per key, a malformed value is the named 65 (fail-closed,
/// never a silent clamp).
pub(crate) fn seal_enabled(value: Option<String>) -> Result<bool, DriverError> {
    tpkg::settings::resolve_bool(&LAZY_SEAL, None, value, Some(true))
        .map_err(|e| DriverError::new(EX_TEBAKO_MANIFEST, e.to_string()))
}

/// A byte-source failure's driver class (spec 39 §6's table).
fn source_error(e: SourceError) -> DriverError {
    let code = match e.kind {
        SourceErrorKind::Fetch | SourceErrorKind::Unsupported => EX_TEBAKO_UNAVAILABLE,
        SourceErrorKind::Integrity => EX_TEBAKO_SHA,
        SourceErrorKind::Io => crate::EX_TEBAKO_IO,
    };
    DriverError::new(code, format!("the lazy env image: {e}"))
}

/// One small document GET for the mount-open path: `file://` reads
/// from disk (the test/airgap spelling), everything else rides
/// tebako-http's one agent (https + the loopback carve-out — the
/// transport's enforcement, one layer down). A non-200 is the named
/// 69, never a guess.
fn get_document(url: &str) -> Result<Vec<u8>, DriverError> {
    if let Some(path) = url.strip_prefix("file://") {
        return std::fs::read(path).map_err(|e| {
            DriverError::new(EX_TEBAKO_UNAVAILABLE, format!("{url}: {e}"))
        });
    }
    let response = tebako_http::get_raw(url, Some("application/json"), None)
        .map_err(|e| DriverError::new(EX_TEBAKO_UNAVAILABLE, format!("{url}: {e}")))?;
    if response.status != 200 {
        return Err(DriverError::new(
            EX_TEBAKO_UNAVAILABLE,
            format!("{url}: status {} — the blksum sidecar must be a 200", response.status),
        ));
    }
    Ok(response.body)
}

/// Fetch and anchor the blksum sidecar at mount-open (spec 39 §3):
/// `<source>.blksum.json`, verified against the descriptor's pins
/// BEFORE any range read — the sidecar pin (70), the strict parse (65),
/// the whole-image cross-check (70) are tpkg::lazy::verify_blksum's
/// single ownership; this is the driver's transport call into it.
fn fetch_blksum(seed: &LazySeed) -> Result<Blksum, DriverError> {
    let url = format!("{}.blksum.json", seed.source);
    let body = get_document(&url)?;
    tpkg::lazy::verify_blksum(seed, &body)
        .map_err(|e| DriverError::new(e.exit_code(), format!("{url}: {e}")))
}

/// Open the lazy mount's byte source. Offline (TEBAKO_OFFLINE=1): the
/// CACHE-ONLY arm — no sidecar fetch, cached groups serve, a miss is
/// EIO on the touching read (spec 39 §6's offline at group
/// granularity). Online: the sidecar anchors first, then the source
/// wraps tebako-http's `get_range` (the spec 39 §8 closure shape —
/// the transport's retry law rides inside).
pub(crate) fn open_lazy_source(
    entry_dir: &Path,
    image_base: &str,
    seed: &LazySeed,
    offline: bool,
) -> Result<Arc<tfs::source_remote::RemoteByteSource>, DriverError> {
    let blocks = tpkg::lazy::blocks_dir(entry_dir, image_base);
    if offline {
        return tfs::source_remote::RemoteByteSource::new_cache_only(
            seed.size_bytes,
            &blocks,
            &seed.source,
        )
        .map(Arc::new)
        .map_err(source_error);
    }
    let blksum = fetch_blksum(seed)?;
    let url = seed.source.clone();
    let fetch = move |offset: u64, len: usize, if_range: Option<&str>| {
        let range = tebako_http::ByteRange {
            offset,
            len: len as u64,
        };
        match tebako_http::get_range(&url, range, if_range, None) {
            Ok(tebako_http::RangeAnswer::Partial(body)) => {
                Ok(tfs::source_remote::RangeFetchAnswer::Partial {
                    bytes: body.bytes,
                    etag: body.etag,
                })
            }
            Ok(tebako_http::RangeAnswer::Full(body)) => {
                Ok(tfs::source_remote::RangeFetchAnswer::Full {
                    bytes: body.bytes,
                    etag: body.etag,
                })
            }
            Err(e) => Err(e.to_string()),
        }
    };
    tfs::source_remote::RemoteByteSource::new(Arc::new(fetch), blksum, &blocks, &seed.source)
        .map(Arc::new)
        .map_err(source_error)
}

/// The background seal thread (spec 39 §5): fill the remaining groups
/// around demand (on-demand reads preempt through the source's
/// install-lock re-check), then the seal commit flips the entry to
/// SEALED. Best-effort BY LAW: a failure is a tebako-log line, never
/// a failed run — the next open resumes from the block map. The
/// thread detaches; the interpreter boot never waits on it.
pub(crate) fn spawn_seal_thread(
    source: &Arc<tfs::source_remote::RemoteByteSource>,
    entry_dir: PathBuf,
    image_base: String,
) {
    let source = Arc::clone(source);
    std::thread::spawn(move || {
        loop {
            let missing = match source.missing_groups() {
                Ok(missing) => missing,
                Err(e) => {
                    tebako_log::log!(
                        tebako_log::Level::Warn,
                        "driver",
                        "lazy: the background seal of {image_base} cannot read the block map (the run is unaffected; the next open resumes): {e}"
                    );
                    return;
                }
            };
            if missing.is_empty() {
                break;
            }
            for index in missing {
                if let Err(e) = source.seed_group(index) {
                    tebako_log::log!(
                        tebako_log::Level::Warn,
                        "driver",
                        "lazy: the background seal of {image_base} paused on group {index} (the run is unaffected; the next open resumes): {e}"
                    );
                    return;
                }
            }
        }
        match tpkg::lazy::seal_entry(&entry_dir, &image_base) {
            Ok(SealOutcome::Sealed { bytes, groups }) => {
                tebako_log::log!(
                    tebako_log::Level::Debug,
                    "driver",
                    "lazy: sealed {image_base} ({bytes} bytes, {groups} groups) — the entry is SEALED"
                );
            }
            Ok(SealOutcome::AlreadySealed) => {
                tebako_log::log!(
                    tebako_log::Level::Debug,
                    "driver",
                    "lazy: {image_base} was already sealed (a concurrent sealer committed first)"
                );
            }
            Err(e) => {
                tebako_log::log!(
                    tebako_log::Level::Warn,
                    "driver",
                    "lazy: the seal commit of {image_base} failed (the run is unaffected; the next open resumes): {e}"
                );
            }
        }
    });
}
