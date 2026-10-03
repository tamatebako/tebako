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
use crate::{EX_TEBAKO_MANIFEST, EX_TEBAKO_SHA, EX_TEBAKO_UNAVAILABLE};

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
#[derive(Debug)]
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
    let (Some(entry_dir), Some(image_base)) =
        (path.parent(), path.file_name().and_then(|n| n.to_str()))
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
        return std::fs::read(path)
            .map_err(|e| DriverError::new(EX_TEBAKO_UNAVAILABLE, format!("{url}: {e}")));
    }
    let response = tebako_http::get_raw(url, Some("application/json"), None)
        .map_err(|e| DriverError::new(EX_TEBAKO_UNAVAILABLE, format!("{url}: {e}")))?;
    if response.status != 200 {
        return Err(DriverError::new(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "{url}: status {} — the blksum sidecar must be a 200",
                response.status
            ),
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

// ---------------------------------------------------------------------
// the spec 39 §10 PR-4 e2e (the loader-side wire, file:// source)
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixture image: a small tree plus ~5 MiB of incompressible
    /// bytes (two 4 MiB groups) written as a limnifs image on disk,
    /// with its blksum sidecar beside it (the tfs lazy_remote fixture's
    /// shape, file://-served).
    fn fixture_image() -> (tempfile::TempDir, PathBuf, Blksum) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("tree");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("hello.txt"), b"hello, lazy driver\n").unwrap();
        let mut big = vec![0u8; 5 * 1024 * 1024];
        let mut state = 0x9E3779B97F4A7C15u64;
        for chunk in big.chunks_mut(8) {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let bytes = state.wrapping_mul(0x2545F4914F6CDD1D).to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        std::fs::write(root.join("big.bin"), &big).unwrap();
        let mut config = limnifs_write::WriteConfig::default_v0_1();
        config.dictionaries.enabled = false;
        let artifact =
            limnifs_write::write_directory_with_config(&root, &config).expect("write succeeds");
        assert!(
            artifact.metadata_sidecar.is_none(),
            "the fixture tree must keep its metadata inline"
        );
        let mut image = artifact.bytes;
        for slab in &artifact.slabs {
            image.extend_from_slice(&slab.bytes);
        }
        let image_path = tmp.path().join("image.tfs");
        std::fs::write(&image_path, &image).unwrap();
        let blksum = Blksum::from_image_bytes(&image);
        std::fs::write(tmp.path().join("image.tfs.blksum.json"), blksum.render()).unwrap();
        (tmp, image_path, blksum)
    }

    /// The LAZY_SEEDING store entry around the fixture: the descriptor
    /// present (file:// source), the image ABSENT.
    fn seed_entry(image_path: &Path, blksum: &Blksum) -> (tempfile::TempDir, String, LazySeed) {
        let entry = tempfile::tempdir().expect("entry");
        let seed = LazySeed {
            source: format!("file://{}", image_path.display()),
            sha256: blksum.sha256.clone(),
            blksum_sha256: tpkg::lazy::sha256_hex(blksum.render().as_bytes()),
            size_bytes: blksum.size_bytes,
            group_count: blksum.group_count(),
        };
        tpkg::lazy::write_descriptor(entry.path(), "image.tfs", &seed).unwrap();
        (entry, "image.tfs".to_string(), seed)
    }

    fn light_seed() -> LazySeed {
        LazySeed {
            source: "file:///x.tfs".to_string(),
            sha256: "a".repeat(64),
            blksum_sha256: "b".repeat(64),
            size_bytes: tpkg::lazy::LAZY_GROUP_SIZE,
            group_count: 1,
        }
    }

    #[test]
    fn env_image_state_classifies_the_three_states_and_the_torn() {
        let tmp = tempfile::tempdir().unwrap();
        let image = tmp.path().join("img.tfs");
        let image = image.to_str().unwrap();
        // Absent: nothing on disk — the caller's ordinary error path.
        assert!(matches!(
            env_image_state(image).unwrap(),
            EnvImageState::Absent
        ));
        // Seeding: a valid descriptor, the image absent.
        let seed = light_seed();
        tpkg::lazy::write_descriptor(tmp.path(), "img.tfs", &seed).unwrap();
        match env_image_state(image).unwrap() {
            EnvImageState::Seeding { seed: got, .. } => assert_eq!(got, seed),
            _ => panic!("expected Seeding"),
        }
        // Sealed: the image present wins outright.
        std::fs::write(image, b"x").unwrap();
        assert!(matches!(
            env_image_state(image).unwrap(),
            EnvImageState::Sealed
        ));
        std::fs::remove_file(image).unwrap();
        // Torn: a garbage descriptor is the named 65, never a guess.
        std::fs::write(tmp.path().join("img.tfs.lazy.json"), b"not json").unwrap();
        let err = env_image_state(image).unwrap_err();
        assert_eq!(err.code, 65, "{err:?}");
        assert!(err.message.contains("img.tfs"), "{err:?}");
    }

    #[test]
    fn the_settings_arms_parse_and_name_the_malformed() {
        assert!(!offline(None));
        assert!(offline(Some("YES".to_string())));
        assert!(!offline(Some("0".to_string())));
        assert!(seal_enabled(None).unwrap());
        assert!(!seal_enabled(Some("0".to_string())).unwrap());
        let err = seal_enabled(Some("maybe".to_string())).unwrap_err();
        assert_eq!(err.code, 65, "{err:?}");
        assert!(err.message.contains("TEBAKO_LAZY_SEAL"), "{err:?}");
    }

    #[test]
    fn the_lazy_wire_mounts_seeds_and_seals_the_entry() {
        let (_t, image_path, blksum) = fixture_image();
        let (entry, image_base, seed) = seed_entry(&image_path, &blksum);
        let source = open_lazy_source(entry.path(), &image_base, &seed, false).expect("opens");
        let byte_source: Arc<dyn tfs::source::ByteSource> = source.clone();
        let mount_point = format!("/lazy-seal-{}", std::process::id());
        let mount = tfs::mount::build_from_source(byte_source, &mount_point).expect("mounts");
        // A touching read answers the golden bytes through the wire.
        let mut buf = [0u8; 32];
        let n = mount.backend.pread("hello.txt", &mut buf, 0).expect("read");
        assert_eq!(&buf[..n], b"hello, lazy driver\n");
        // The seal thread fills the remaining groups and commits; the
        // entry flips SEALED (image + anchor, descriptor + blocks gone).
        spawn_seal_thread(&source, entry.path().to_path_buf(), image_base.clone());
        let sealed_image = entry.path().join(&image_base);
        let sidecar = entry.path().join(format!("{image_base}.sha256"));
        // Wait for the TERMINAL state, not the first write: the seal thread
        // commits image → anchor → cleanup in order, and slow filesystems
        // (windows-gnu under AV scan) expose the gap (#664).
        for _ in 0..200 {
            if sealed_image.is_file()
                && sidecar.is_file()
                && !tpkg::lazy::descriptor_path(entry.path(), &image_base).exists()
                && !tpkg::lazy::blocks_dir(entry.path(), &image_base).exists()
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(sealed_image.is_file(), "the seal thread installs the image");
        assert!(sidecar.is_file(), "the seal thread installs the anchor");
        assert!(!tpkg::lazy::descriptor_path(entry.path(), &image_base).exists());
        assert!(!tpkg::lazy::blocks_dir(entry.path(), &image_base).exists());
        // The sealed bytes are byte-identical with the origin's.
        assert_eq!(
            tpkg::lazy::sha256_hex(&std::fs::read(&sealed_image).unwrap()),
            seed.sha256
        );
        // The entry now state-detects SEALED — the lazy machinery is
        // out of the loop entirely.
        assert!(matches!(
            env_image_state(sealed_image.to_str().unwrap()).unwrap(),
            EnvImageState::Sealed
        ));
        drop(mount);
    }

    #[test]
    fn the_offline_cache_only_open_serves_seeded_groups_and_names_the_miss() {
        let (_t, image_path, blksum) = fixture_image();
        let (entry, image_base, seed) = seed_entry(&image_path, &blksum);
        // Seed group 0 ONLINE (the mount-open prefix), then go offline.
        let online = open_lazy_source(entry.path(), &image_base, &seed, false).unwrap();
        online.seed_group(0).expect("group 0 seeds");
        drop(online);
        let source = open_lazy_source(entry.path(), &image_base, &seed, true)
            .expect("the cache-only open needs no network");
        let byte_source: Arc<dyn tfs::source::ByteSource> = source.clone();
        let mount_point = format!("/lazy-offline-{}", std::process::id());
        let mount = tfs::mount::build_from_source(byte_source, &mount_point)
            .expect("the offline mount opens from the cached prefix");
        let mut buf = [0u8; 32];
        let n = mount
            .backend
            .pread("hello.txt", &mut buf, 0)
            .expect("cached read");
        assert_eq!(&buf[..n], b"hello, lazy driver\n");
        // An UNseeded group's touching read is EIO — never a fabricated
        // zero-fill, never a silent short read (spec 39 §6).
        let mut miss = vec![0u8; 8192];
        let err = mount
            .backend
            .pread("big.bin", &mut miss, 4 * 1024 * 1024 + 10)
            .expect_err("a miss offline is EIO");
        assert_eq!(err, libc::EIO);
        drop(mount);
    }
}
