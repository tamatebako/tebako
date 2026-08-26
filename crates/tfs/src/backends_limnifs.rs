//! LimniFS backend (spec 20, image format 5) over `limnifs-core` — pure
//! safe Rust (no FFI, so the `unsafe`-at-FFI-boundary rule never
//! triggers), read-only forever (the transforms law: COW/ENC stack
//! above it unchanged).
//!
//! ## The self-contained image layout
//!
//! A stock LimniFS artifact is a manifest plus `file:`-located sidecar
//! slabs. A tebako payload is ONE file, so the tebako writer path
//! (`tfs mkimage --format limnifs`, `tebako press --format limnifs`)
//! emits the writer's manifest bytes verbatim followed by every slab in
//! slab-ordinal order:
//!
//! ```text
//! [manifest header][feature flags][metadata_reference][slab_index]
//! [history][slab 0 (LIM1…)][slab 1 (LIM1…)]…
//! ```
//!
//! Mount-open therefore parses, from the image byte slice (spec 20 §4):
//! `ManifestCursor` → `parse_manifest_header` → `parse_metadata_reference`
//! → `parse_metadata_blob` (inline metadata is REQUIRED — a `file:`
//! metadata sidecar would be a second artifact) → `parse_slab_index` →
//! `parse_history` → `parse_slab` per appended slab (index only — no
//! upfront decompression; drops materialize per read window).
//!
//! `LIM1` is a SECTION magic inside the image, never an offset-0 image
//! magic — detection keys on `LMFS` only (spec 20 §3).
//!
//! ## Error mapping (spec 20 §4, errno-valued, named, never silent)
//!
//! `TooShort`/`BadMagic`/`Corrupt` → `EINVAL` at mount-open (not a
//! limnifs image / broken structural invariant / metadata checksum);
//! `UnsupportedFeature` → `ENOTSUP` naming the feature (on the
//! `TEBAKO_DEBUG` log — the errno channel carries the code); while
//! serving, `Corrupt` → `EIO`, `UnsupportedFeature` → `ENOTSUP`, path
//! misses → `ENOENT`.

use std::collections::HashMap;

use limnifs_core::{
    parse_feature_flags_section, parse_history, parse_manifest_header, parse_metadata_blob,
    parse_metadata_reference, parse_slab, parse_slab_header, parse_slab_index, ContentHandle,
    CoreError, DropRecord, Inode, ManifestCursor, ManifestHeader, MetadataBlob, SLAB_HEADER_LEN,
};

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat};

/// The slab section magic (`LIM1`) — a section marker inside the
/// image, checked at the slab-region boundary (spec 20 §3).
const SLAB_MAGIC: &[u8; 4] = b"LIM1";

/// Nanoseconds per second, for the `mtime_ns` → `RawStat` seconds
/// truncation (spec 20 §8).
const NANOS_PER_SEC: u64 = 1_000_000_000;

/// A mounted LimniFS image.
#[derive(Debug)]
pub struct LimnifsBackend {
    /// The whole image (manifest + appended slabs), owned.
    image: Vec<u8>,
    /// Parsed manifest header (versions for the info surface).
    header: ManifestHeader,
    /// The parsed metadata blob (inodes + directory nodes).
    blob: MetadataBlob,
    /// Path → inode number (keys relative, no leading slash; `""` is
    /// the root), built once at open (O(N)) for O(1) lookups.
    paths: HashMap<String, u64>,
    /// The root directory's inode number.
    root: u64,
    /// One entry per appended slab: the absolute offset of its solid
    /// window inside `image`.
    slab_windows: Vec<usize>,
    /// Drop id → (slab ordinal, drop record), built once at open.
    drops: HashMap<[u8; 32], (usize, DropRecord)>,
    /// Section spans for `image_info_json` (`tfs info --backend-json`).
    sections: Vec<(&'static str, usize)>,
    /// Per-slab drop counts (the info surface).
    slab_drop_counts: Vec<usize>,
    /// The last served drop's plaintext, keyed by drop id (tebako#464).
    /// Sequential chunked readers (the dlmap extraction walks a file in
    /// 8 KiB windows) hit the same drop for hundreds of consecutive
    /// preads; without the memo each call re-decompresses the WHOLE drop
    /// — a 19.5 MiB shim read that way cost ~48 GiB of lz4 work (~19 s,
    /// the reported ~1 MiB/s). Bounded at one drop: random access never
    /// holds more than the current drop's plaintext.
    last_drop: std::sync::Mutex<Option<([u8; 32], Vec<u8>)>>,
    /// Test-only decompress counter — the memo's proof: one decompress
    /// per distinct drop, not per pread window.
    #[cfg(test)]
    decompress_calls: std::sync::Mutex<usize>,
}

/// Mount-open mapping (spec 20 §4): `TooShort`/`BadMagic`/`Corrupt` →
/// `EINVAL`, `UnsupportedFeature` → `ENOTSUP`. The named reason rides
/// the debug log — the errno channel carries the code.
fn open_error(e: CoreError) -> i32 {
    let errno = match &e {
        CoreError::UnsupportedFeature { .. } => libc::ENOTSUP,
        CoreError::TooShort { .. } | CoreError::BadMagic { .. } | CoreError::Corrupt { .. } => {
            libc::EINVAL
        }
    };
    tebako_log::log!(
        tebako_log::Level::Trace,
        "tfs",
        "limnifs mount-open failed: {e} (errno {errno})"
    );
    errno
}

/// A feature the adapter does not implement is refused by NAME on the
/// debug log and by `ENOTSUP` on the errno channel (spec 20 §5's
/// compiled-out/unsupported rule: never a silent re-route).
fn unsupported(what: String) -> i32 {
    tebako_log::log!(
        tebako_log::Level::Warn,
        "tfs",
        "limnifs: unsupported feature: {what} (ENOTSUP)"
    );
    libc::ENOTSUP
}

/// Serving-side mapping (spec 20 §4): `Corrupt` → `EIO`,
/// `UnsupportedFeature` → `ENOTSUP`.
fn serve_error(e: CoreError) -> i32 {
    match &e {
        CoreError::UnsupportedFeature { .. } => libc::ENOTSUP,
        CoreError::TooShort { .. } | CoreError::BadMagic { .. } | CoreError::Corrupt { .. } => {
            libc::EIO
        }
    }
}

/// Normalize an in-image path: no leading or trailing `/`, `""` for root.
fn normalize(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

impl LimnifsBackend {
    /// Open a self-contained limnifs image (module doc layout) held in
    /// memory. Every mount source (whole file, file region, memory) is
    /// read into one owned byte slice and funnels here — spec 11 §5's
    /// four mount-source kinds all serve one `&[u8]` core.
    pub fn from_image(data: Vec<u8>) -> Result<LimnifsBackend, i32> {
        let mut cursor = ManifestCursor::new(&data);

        let header = parse_manifest_header(&mut cursor).map_err(open_error)?;
        let header_end = cursor.position();

        let flags = parse_feature_flags_section(&mut cursor).map_err(open_error)?;
        for entry in &flags.entries {
            if entry.required {
                return Err(unsupported(format!(
                    "required feature flag 0x{:04X}",
                    entry.flag_id
                )));
            }
            // Optional flags are silently ignored (limnifs spec §18's
            // unknown-flag policy).
        }
        let flags_end = cursor.position();

        let meta_ref = parse_metadata_reference(&mut cursor).map_err(open_error)?;
        let meta_ref_end = cursor.position();
        let Some(blob_bytes) = meta_ref.inline_metadata.as_deref() else {
            return Err(unsupported(
                "external metadata locator (a self-contained tebako image inlines the metadata blob)"
                    .to_string(),
            ));
        };
        // The reference's hash commits to the uncompressed blob: a
        // checksum failure is Corrupt at mount-open → EINVAL (spec 20 §4).
        if limnifs_core::hash_section(blob_bytes) != meta_ref.metadata_hash {
            return Err(open_error(CoreError::Corrupt {
                reason: "metadata blob does not match the metadata_reference hash".to_string(),
            }));
        }
        let blob = parse_metadata_blob(&mut ManifestCursor::new(blob_bytes)).map_err(open_error)?;
        let root = blob.root_inode_number().ok_or_else(|| {
            open_error(CoreError::Corrupt {
                reason: "no unique root directory inode".to_string(),
            })
        })?;

        let mut paths: HashMap<String, u64> =
            HashMap::with_capacity(blob.inodes.len().saturating_add(1));
        paths.insert(String::new(), root);
        for (path, number) in blob.build_path_index() {
            // The limnifs index is absolute (`/a/b`); the Backend trait
            // speaks relative paths (`a/b`, `""` for root).
            paths.insert(normalize(&path).to_string(), number);
        }

        let slab_index = parse_slab_index(&mut cursor).map_err(open_error)?;
        let slab_index_end = cursor.position();

        // The writer always emits a history section after the slab
        // index; parsing it lands the cursor exactly on the slab region.
        let _history = parse_history(&mut cursor).map_err(open_error)?;
        let history_end = cursor.position();

        let sections: Vec<(&'static str, usize)> = vec![
            ("MANIFEST_HEADER", header_end),
            ("FEATURE_FLAGS", flags_end - header_end),
            ("METADATA_REFERENCE", meta_ref_end - flags_end),
            ("SLAB_INDEX", slab_index_end - meta_ref_end),
            ("HISTORY", history_end - slab_index_end),
        ];

        let mut slab_windows: Vec<usize> = Vec::with_capacity(slab_index.len());
        let mut slab_drop_counts: Vec<usize> = Vec::with_capacity(slab_index.len());
        let mut drops: HashMap<[u8; 32], (usize, DropRecord)> = HashMap::new();
        let mut pos = history_end;
        if slab_index.is_empty() {
            if pos != data.len() {
                return Err(open_error(CoreError::Corrupt {
                    reason: format!(
                        "{} trailing bytes after the manifest of a slab-less image",
                        data.len() - pos
                    ),
                }));
            }
        } else {
            for entry in &slab_index.entries {
                if data.len() < pos.saturating_add(SLAB_HEADER_LEN)
                    || &data[pos..pos.saturating_add(4)] != SLAB_MAGIC
                {
                    // Slab bytes are missing (external `file:` locators —
                    // a second artifact) or a trailing manifest section
                    // this adapter does not know (profile descriptor,
                    // dictionary, …) sits before the slab region.
                    return Err(unsupported(format!(
                        "slab ordinal {} is not appended to the image (external locator or unknown trailing section)",
                        entry.slab_id.ordinal
                    )));
                }
                let mut slab_cursor = ManifestCursor::at_start(&data, pos).map_err(open_error)?;
                let slab_header = parse_slab_header(&mut slab_cursor).map_err(open_error)?;
                if slab_header.is_sealed() {
                    return Err(unsupported(
                        "AEAD-sealed slab (spec 20 §7: tebako-side encryption stays the spec-10 transform)"
                            .to_string(),
                    ));
                }
                if slab_header.slab_id != entry.slab_id {
                    return Err(open_error(CoreError::Corrupt {
                        reason: format!(
                            "appended slab id (ordinal {}) does not match the slab_index entry (ordinal {})",
                            slab_header.slab_id.ordinal, entry.slab_id.ordinal
                        ),
                    }));
                }
                let total = usize::try_from(slab_header.total_length).map_err(|_| {
                    open_error(CoreError::Corrupt {
                        reason: "slab total_length exceeds usize".to_string(),
                    })
                })?;
                let Some(end) = pos.checked_add(total) else {
                    return Err(libc::EINVAL);
                };
                if end > data.len() {
                    return Err(open_error(CoreError::TooShort {
                        have: data.len() - pos,
                        need: total,
                    }));
                }
                let view = parse_slab(&data[pos..end]).map_err(open_error)?;
                slab_drop_counts.push(view.drop_records().len());
                for record in view.drop_records() {
                    drops.insert(*record.drop_id.as_bytes(), (slab_windows.len(), *record));
                }
                slab_windows.push(pos + view.solid_window_offset());
                pos = end;
            }
            if pos != data.len() {
                return Err(open_error(CoreError::Corrupt {
                    reason: format!(
                        "{} trailing bytes after the last appended slab",
                        data.len() - pos
                    ),
                }));
            }
        }

        Ok(LimnifsBackend {
            image: data,
            header,
            blob,
            paths,
            root,
            slab_windows,
            drops,
            sections,
            slab_drop_counts,
            last_drop: std::sync::Mutex::new(None),
            #[cfg(test)]
            decompress_calls: std::sync::Mutex::new(0),
        })
    }

    /// The inode for a Backend-convention path, `ENOENT` when missing.
    fn inode_for(&self, path: &str) -> Result<&Inode, i32> {
        let number = if path.is_empty() {
            self.root
        } else {
            *self.paths.get(path).ok_or(libc::ENOENT)?
        };
        self.blob.inode_by_number(number).ok_or_else(|| {
            tebako_log::log!(
                tebako_log::Level::Trace,
                "tfs",
                "limnifs: dangling inode number {number} (EIO)"
            );
            libc::EIO
        })
    }

    /// The materialized plaintext of one drop (on-demand, per-class
    /// decompression — spec 20 §4; no slab access for inline drops ever
    /// reaches here).
    fn drop_plaintext(&self, drop_id: &[u8; 32]) -> Result<Vec<u8>, i32> {
        let Some((slab, record)) = self.drops.get(drop_id) else {
            // A slice references a drop no slab carries: a broken
            // cross-reference, i.e. corruption while serving.
            return Err(libc::EIO);
        };
        if record.representation.aead != 0x00 {
            return Err(unsupported(format!(
                "drop AEAD 0x{:02X} (only plaintext drops are mounted)",
                record.representation.aead
            )));
        }
        if record.solid_window_index != 0 {
            return Err(unsupported(format!(
                "solid_window_index {} (single-window slabs only)",
                record.solid_window_index
            )));
        }
        if record.dict_id != limnifs_core::drop_record::NO_DICT {
            return Err(unsupported(
                "dictionary-compressed drop (the tebako writer path emits no dictionaries)"
                    .to_string(),
            ));
        }
        let start = self.slab_windows[*slab]
            .saturating_add(usize::try_from(record.offset_in_window).map_err(|_| libc::EIO)?);
        let end =
            start.saturating_add(usize::try_from(record.len_in_window).map_err(|_| libc::EIO)?);
        if end > self.image.len() {
            return Err(libc::EIO);
        }
        limnifs_core::codec::decompress(
            record.representation.codec,
            &self.image[start..end],
            record.plaintext_len,
        )
        .map_err(serve_error)
    }

    /// The `[start, end)` window of one drop's plaintext, memoized
    /// against the last drop served (the field note carries the
    /// tebako#464 math). The lock is held across the decompress so
    /// concurrent readers never duplicate the work; the copy out is the
    /// window only.
    fn drop_window(&self, drop_id: &[u8; 32], start: usize, end: usize) -> Result<Vec<u8>, i32> {
        let mut memo = self.last_drop.lock().map_err(|_| libc::EIO)?;
        let hit = matches!(memo.as_ref(), Some((id, _)) if id == drop_id);
        if !hit {
            let plain = self.drop_plaintext(drop_id)?;
            #[cfg(test)]
            {
                *self.decompress_calls.lock().unwrap() += 1;
            }
            *memo = Some((*drop_id, plain));
        }
        let Some((_, plain)) = memo.as_ref() else {
            return Err(libc::EIO); // unreachable: the miss arm just filled it
        };
        if end > plain.len() {
            return Err(libc::EIO);
        }
        Ok(plain[start..end].to_vec())
    }

    /// The byte length a stat reports for a regular file's content
    /// handle: inline length for `InlineData` (`SharedInline` resolves
    /// to `InlineData` at parse), summed `SliceRef` spans for
    /// `SliceMap` (spec 20 §4).
    fn content_size(inode: &Inode) -> i64 {
        match &inode.content_handle {
            ContentHandle::InlineData(data) => data.len() as i64,
            ContentHandle::SliceMap(slices) => slices
                .iter()
                .map(|s| (s.file_byte_end - s.file_byte_start) as i64)
                .sum(),
            ContentHandle::Symlink(target) => target.len() as i64,
            ContentHandle::SharedInline(_) => 0, // unreachable post-parse (EIO on read)
            ContentHandle::Directory(_) | ContentHandle::Device(_) | ContentHandle::Pipe(_) => 0,
        }
    }
}

impl Backend for LimnifsBackend {
    fn name(&self) -> &'static std::ffi::CStr {
        c"LimniFS"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let inode = self.inode_for(normalize(path))?;
        let entry_type = match inode.file_type() {
            limnifs_core::S_IFREG => EntryType::File,
            limnifs_core::S_IFDIR => EntryType::Directory,
            limnifs_core::S_IFLNK => EntryType::Symlink,
            _ => EntryType::Other,
        };
        Ok(RawStat {
            entry_type,
            perms: inode.mode & 0o7777,
            size: Self::content_size(inode),
            mtime: (inode.mtime_ns / NANOS_PER_SEC) as i64,
        })
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let inode = self.inode_for(normalize(path))?;
        match &inode.content_handle {
            // Inline drops: served straight from the metadata blob — no
            // slab access, no decompression (spec 20 §4).
            ContentHandle::InlineData(data) => {
                let size = data.len() as u64;
                if offset >= size || buf.is_empty() {
                    return Ok(0);
                }
                let want = std::cmp::min(buf.len() as u64, size - offset) as usize;
                buf[..want].copy_from_slice(&data[offset as usize..offset as usize + want]);
                Ok(want)
            }
            // Slab drops: only the drops intersecting the requested
            // window are materialized (spec 20 §4). Callers clamp to
            // EOF; short reads allowed.
            ContentHandle::SliceMap(slices) => {
                let size: u64 = slices
                    .iter()
                    .map(|s| s.file_byte_end - s.file_byte_start)
                    .sum();
                if offset >= size || buf.is_empty() {
                    return Ok(0);
                }
                let win_start = offset;
                let win_end = offset + std::cmp::min(buf.len() as u64, size - offset);
                let mut pos = win_start; // expected contiguous coverage start
                let mut done = 0usize;
                for slice in slices {
                    if slice.file_byte_end <= win_start {
                        continue;
                    }
                    if slice.file_byte_start >= win_end {
                        break;
                    }
                    let span = slice.file_byte_end - slice.file_byte_start;
                    if u64::from(slice.drop_byte_len) != span {
                        return Err(libc::EIO); // a partial-drop mapping we cannot serve
                    }
                    if slice.file_byte_start > pos {
                        return Err(libc::EIO); // a hole in the file's coverage
                    }
                    let lo = std::cmp::max(pos, slice.file_byte_start);
                    let hi = std::cmp::min(win_end, slice.file_byte_end);
                    if lo >= hi {
                        continue;
                    }
                    let ds =
                        (u64::from(slice.drop_byte_start) + (lo - slice.file_byte_start)) as usize;
                    let de =
                        (u64::from(slice.drop_byte_start) + (hi - slice.file_byte_start)) as usize;
                    let window = self.drop_window(slice.drop_id.as_bytes(), ds, de)?;
                    let at = (lo - win_start) as usize;
                    buf[at..at + (hi - lo) as usize].copy_from_slice(&window);
                    done += (hi - lo) as usize;
                    pos = hi;
                }
                Ok(done)
            }
            ContentHandle::SharedInline(_) => Err(libc::EIO), // unresolved shared table
            ContentHandle::Directory(_) => Err(libc::EISDIR),
            ContentHandle::Symlink(_) | ContentHandle::Device(_) | ContentHandle::Pipe(_) => {
                Err(libc::EINVAL)
            }
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        let inode = self.inode_for(normalize(path))?;
        let ContentHandle::Directory(hash) = &inode.content_handle else {
            return Err(libc::ENOTDIR);
        };
        let node = self.blob.dir_node_by_hash(hash).ok_or(libc::EIO)?;
        Ok(node
            .entries
            .iter()
            .map(|e| RawDirEntry {
                name: e.name.clone(),
                is_dir: e.entry_type == limnifs_core::directory_node::entry_type::DIRECTORY,
            })
            .collect())
    }

    fn read_link(&self, path: &str) -> Result<String, i32> {
        let inode = self.inode_for(normalize(path))?;
        match &inode.content_handle {
            ContentHandle::Symlink(target) => Ok(target.clone()),
            _ => Err(libc::EINVAL),
        }
    }

    fn image_info_json(&self) -> Option<String> {
        let mut json = format!(
            "{{\"format\":\"limnifs\",\"versions\":{{\"drop_store\":{},\"metadata\":{},\"manifest\":{}}},\"inode_count\":{},\"directory_count\":{},\"slab_count\":{},\"drop_count\":{},\"image_bytes\":{},\"sections\":[",
            self.header.drop_store_version,
            self.header.metadata_version,
            self.header.manifest_version,
            self.blob.inodes.len(),
            self.blob.dir_nodes.len(),
            self.slab_windows.len(),
            self.drops.len(),
            self.image.len(),
        );
        let mut first = true;
        for (name, size) in &self.sections {
            if !first {
                json.push(',');
            }
            first = false;
            json.push_str(&format!("{{\"type\":\"{name}\",\"size\":{size}}}"));
        }
        for (ordinal, drops) in self.slab_drop_counts.iter().enumerate() {
            json.push_str(&format!(
                ",{{\"type\":\"SLAB\",\"ordinal\":{ordinal},\"drops\":{drops}}}"
            ));
        }
        json.push_str("]}");
        Some(json)
    }
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

/// Golden fixtures are built IN-PROCESS with limnifs-write (never a
/// `limni` binary — invariant 1) in a tempdir, then mounted through the
/// backend and the mount constructors. The dwarfs backend is the parity
/// oracle (spec 20 §8): same tree in → same logical VFS answers out.
#[cfg(all(test, feature = "backend-limnifs"))]
mod tests {
    use super::*;
    use crate::backend::detect_format;
    use crate::backend::ImageFormat;

    /// Build the tebako single-file layout (module doc): the writer's
    /// manifest bytes verbatim + every slab appended in ordinal order.
    /// Dictionaries are disabled so the manifest section order is the
    /// fixed writer sequence the mount-open walk relies on.
    fn build_image(dir: &std::path::Path) -> Vec<u8> {
        let mut config = limnifs_write::WriteConfig::default_v0_1();
        config.dictionaries.enabled = false;
        let artifact =
            limnifs_write::write_directory_with_config(dir, &config).expect("write succeeds");
        assert!(
            artifact.metadata_sidecar.is_none(),
            "the fixture tree must keep its metadata inline"
        );
        let mut image = artifact.bytes;
        for slab in &artifact.slabs {
            image.extend_from_slice(&slab.bytes);
        }
        image
    }

    /// Deterministic 200_000-byte payload (multi-drop slab coverage).
    fn big_payload() -> Vec<u8> {
        (0..200_000u32)
            .map(|i| ((i.wrapping_mul(2654435761) >> 13) & 0xFF) as u8)
            .collect()
    }

    /// The fixture tree: an inline file, a nested inline file, and a
    /// slab-backed file (> 4096-byte inline threshold).
    fn fixture_tree() -> (tempfile::TempDir, Vec<u8>) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("hello.txt"), b"hello, limnifs\n").unwrap();
        std::fs::write(root.join("sub").join("nested.txt"), b"nested content here").unwrap();
        std::fs::write(root.join("big.bin"), big_payload()).unwrap();
        let image = build_image(root);
        (tmp, image)
    }

    // -----------------------------------------------------------------
    // Committed slab-v1 golden fixture — the transition proof for the
    // limnifs format evolution (slab v2 lands as "the v1" without a
    // format-version bump; spec 20 §3). The committed bytes were written
    // by the pinned limnifs-write with `WriteConfig::default_v0_1()`;
    // every future reader MUST mount them and answer identically.
    // Regenerate (only while `default_v0_1()` still emits the slab-v1
    // layout) with:
    //   cargo test -p tfs write_slab_v1_golden_fixture -- --ignored
    // and commit the result.
    // -----------------------------------------------------------------

    /// Fixed mtime for the golden tree (2026-01-01T00:00:00Z) so the
    /// committed image bytes are deterministic.
    const GOLDEN_MTIME_SECS: u64 = 1_767_225_600;

    fn golden_mtime() -> std::time::SystemTime {
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(GOLDEN_MTIME_SECS)
    }

    const GOLDEN_FIXTURE_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/limnifs-slab-v1.lmfs"
    );

    /// Regenerate `tests/fixtures/limnifs-slab-v1.lmfs` with the pinned
    /// writer. Ignored by default — run explicitly (see above) and
    /// commit the result.
    #[test]
    #[ignore = "fixture generator — run explicitly"]
    fn write_slab_v1_golden_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("hello.txt"), b"hello, limnifs\n").unwrap();
        std::fs::write(root.join("sub").join("nested.txt"), b"nested content here").unwrap();
        std::fs::write(root.join("big.bin"), big_payload()).unwrap();
        // Pin every mtime (the writer records the host file's real
        // mtime) so the image bytes are deterministic.
        for path in ["sub", "hello.txt", "sub/nested.txt", "big.bin"] {
            std::fs::File::open(root.join(path))
                .unwrap()
                .set_modified(golden_mtime())
                .unwrap();
        }
        let image = build_image(root);
        std::fs::write(GOLDEN_FIXTURE_PATH, &image).unwrap();
        eprintln!("wrote {} bytes to {GOLDEN_FIXTURE_PATH}", image.len());
    }

    #[test]
    fn slab_v1_golden_fixture_mounts_and_answers() {
        let image: &[u8] = include_bytes!("../tests/fixtures/limnifs-slab-v1.lmfs");
        assert_eq!(detect_format(image), ImageFormat::Limnifs);
        let backend = LimnifsBackend::from_image(image.to_vec()).expect("v1 golden mounts");
        assert_eq!(backend.name().to_str().unwrap(), "LimniFS");

        let mut root = backend.read_dir("").expect("root lists");
        root.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<(&str, bool)> = root.iter().map(|e| (e.name.as_str(), e.is_dir)).collect();
        assert_eq!(
            names,
            vec![("big.bin", false), ("hello.txt", false), ("sub", true)]
        );

        let hello = backend.stat("hello.txt").expect("file stats");
        assert_eq!(hello.entry_type, EntryType::File);
        assert_eq!(hello.perms, 0o644);
        assert_eq!(hello.size, 15);
        assert_eq!(hello.mtime, GOLDEN_MTIME_SECS as i64);
        let mut buf = [0u8; 15];
        assert_eq!(backend.pread("hello.txt", &mut buf, 0).unwrap(), 15);
        assert_eq!(&buf, b"hello, limnifs\n");

        let mut buf = [0u8; 19];
        assert_eq!(backend.pread("sub/nested.txt", &mut buf, 0).unwrap(), 19);
        assert_eq!(&buf, b"nested content here");

        let big = backend.stat("big.bin").expect("big stats");
        assert_eq!(big.size, 200_000);
        assert_eq!(big.mtime, GOLDEN_MTIME_SECS as i64);
        let want = big_payload();
        let mut got = vec![0u8; want.len()];
        let mut off = 0u64;
        while (off as usize) < want.len() {
            let n = backend
                .pread("big.bin", &mut got[off as usize..], off)
                .expect("read");
            assert!(n > 0);
            off += n as u64;
        }
        assert_eq!(got, want);
    }

    #[test]
    fn lmfs_magic_detects_limnifs() {
        let mut magic = [0u8; 512];
        magic[..4].copy_from_slice(b"LMFS");
        assert_eq!(detect_format(&magic), ImageFormat::Limnifs);
        // The other strong magics keep their answers (probe order
        // unchanged above limnifs).
        magic[..4].copy_from_slice(b"PK\x03\x04");
        assert_eq!(detect_format(&magic), ImageFormat::Zip);
        magic[..6].copy_from_slice(b"DWARFS");
        assert_eq!(detect_format(&magic), ImageFormat::Dwarfs);
        magic[..4].copy_from_slice(b"hsqs");
        assert_eq!(detect_format(&magic), ImageFormat::Squashfs);
        magic[..4].copy_from_slice(b"LMFS");
        // A slot STAMPED limnifs whose bytes are not limnifs mounts by
        // what the magic says — here the gzip tar envelope.
        magic[..4].copy_from_slice(&[0x1f, 0x8b, 8, 0]);
        assert_eq!(detect_format(&magic), ImageFormat::TarGz);
        magic[..4].copy_from_slice(b"\x28\xb5\x2f\xfd");
        assert_eq!(detect_format(&magic), ImageFormat::TarZst);
    }

    #[test]
    fn lim1_slab_magic_is_not_an_image_magic() {
        // `LIM1` is a section magic inside the image (spec 20 §3) —
        // detection must NOT claim it.
        let mut magic = [0u8; 512];
        magic[..4].copy_from_slice(b"LIM1");
        assert_eq!(detect_format(&magic), ImageFormat::Unknown);
    }

    #[test]
    fn stat_reports_types_perms_sizes_and_truncated_mtime() {
        let (tmp, image) = fixture_tree();
        let backend = LimnifsBackend::from_image(image).expect("mounts");
        assert_eq!(backend.name().to_str().unwrap(), "LimniFS");

        let root = backend.stat("").expect("root stats");
        assert_eq!(root.entry_type, EntryType::Directory);
        assert_eq!(root.perms, 0o755);

        let hello = backend.stat("hello.txt").expect("file stats");
        assert_eq!(hello.entry_type, EntryType::File);
        assert_eq!(hello.perms, 0o644);
        assert_eq!(hello.size, 15);
        // mtime truncation: limnifs mtime_ns → RawStat seconds (the
        // writer takes the host file's real mtime).
        let host_secs = std::fs::metadata(tmp.path().join("hello.txt"))
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(hello.mtime, host_secs as i64);

        let big = backend.stat("big.bin").expect("big stats");
        assert_eq!(big.entry_type, EntryType::File);
        assert_eq!(big.size, 200_000);

        let sub = backend.stat("sub").expect("dir stats");
        assert_eq!(sub.entry_type, EntryType::Directory);

        assert_eq!(backend.stat("nope").unwrap_err(), libc::ENOENT);
        assert_eq!(backend.stat("sub/nope").unwrap_err(), libc::ENOENT);
    }

    #[test]
    fn has_entry_or_children_is_the_stat_answer() {
        let (_tmp, image) = fixture_tree();
        let backend = LimnifsBackend::from_image(image).expect("mounts");
        assert!(backend.has_entry_or_children(""));
        assert!(backend.has_entry_or_children("sub"));
        assert!(backend.has_entry_or_children("hello.txt"));
        assert!(!backend.has_entry_or_children("nope"));
        assert!(!backend.has_entry_or_children("sub/nope"));
    }

    #[test]
    fn inline_only_tree_has_no_slab_at_all() {
        // A tree of small files produces NO slab: every read is served
        // straight from the metadata blob (spec 20 §4's zero-copy path).
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), b"alpha").unwrap();
        std::fs::write(tmp.path().join("b.txt"), b"beta").unwrap();
        let backend = LimnifsBackend::from_image(build_image(tmp.path())).expect("mounts");
        assert!(backend.drops.is_empty());
        assert!(backend.slab_windows.is_empty());
        let mut buf = [0u8; 8];
        assert_eq!(backend.pread("a.txt", &mut buf, 0).unwrap(), 5);
        assert_eq!(&buf[..5], b"alpha");
        assert_eq!(backend.pread("b.txt", &mut buf, 1).unwrap(), 3);
        assert_eq!(&buf[..3], b"eta");
    }

    #[test]
    fn pread_inline_drop_never_touches_the_slab() {
        let (_tmp, image) = fixture_tree();
        let backend = LimnifsBackend::from_image(image).expect("mounts");
        // Full read.
        let mut buf = [0u8; 64];
        let n = backend.pread("hello.txt", &mut buf, 0).unwrap();
        assert_eq!(&buf[..n], b"hello, limnifs\n");
        // Windowed read.
        let n = backend.pread("hello.txt", &mut buf, 7).unwrap();
        assert_eq!(&buf[..n], b"limnifs\n");
        // EOF clamping: offset at/past end reads 0.
        assert_eq!(backend.pread("hello.txt", &mut buf, 15).unwrap(), 0);
        assert_eq!(backend.pread("hello.txt", &mut buf, 4096).unwrap(), 0);
        // Short read at the tail.
        let n = backend.pread("hello.txt", &mut buf, 13).unwrap();
        assert_eq!(&buf[..n], b"s\n");
        // Errors: missing file, directory, empty buffer.
        assert_eq!(
            backend.pread("nope", &mut buf, 0).unwrap_err(),
            libc::ENOENT
        );
        assert_eq!(backend.pread("sub", &mut buf, 0).unwrap_err(), libc::EISDIR);
        assert_eq!(backend.pread("hello.txt", &mut [], 0).unwrap(), 0);
    }

    #[test]
    fn pread_slab_drop_materializes_per_window() {
        let (_tmp, image) = fixture_tree();
        let backend = LimnifsBackend::from_image(image).expect("mounts");
        assert!(!backend.drops.is_empty(), "big.bin must be slab-backed");
        let want = big_payload();

        // Whole-file read in one buffer.
        let mut buf = vec![0u8; want.len()];
        let n = backend.pread("big.bin", &mut buf, 0).unwrap();
        assert_eq!(n, want.len());
        assert_eq!(buf, want);

        // A middle window crossing chunk (drop) boundaries.
        let mut win = [0u8; 30_000];
        let n = backend.pread("big.bin", &mut win, 77_777).unwrap();
        assert_eq!(n, 30_000);
        assert_eq!(&win[..], &want[77_777..107_777]);

        // A tiny window inside one drop.
        let mut small = [0u8; 17];
        let n = backend.pread("big.bin", &mut small, 3).unwrap();
        assert_eq!(&small[..n], &want[3..20]);

        // Tail short read + EOF clamp.
        let mut tail = [0u8; 128];
        let n = backend.pread("big.bin", &mut tail, 199_950).unwrap();
        assert_eq!(n, 50);
        assert_eq!(&tail[..n], &want[199_950..]);
        assert_eq!(backend.pread("big.bin", &mut tail, 200_000).unwrap(), 0);
    }

    #[test]
    fn sequential_chunked_reads_decompress_each_drop_once() {
        // tebako#464: the dlmap extraction reads a file in 8 KiB windows.
        // The last-drop memo must collapse the per-window decompressions
        // to one per DISTINCT drop — before it, a 19.5 MiB shim read that
        // way ran ~48 GiB of lz4 work (~19 s, the reported ~1 MiB/s).
        let (_tmp, image) = fixture_tree();
        let backend = LimnifsBackend::from_image(image).expect("mounts");
        let want = big_payload();

        let mut acc = Vec::with_capacity(want.len());
        let mut buf = [0u8; 8192];
        let mut offset = 0u64;
        while (offset as usize) < want.len() {
            let n = backend.pread("big.bin", &mut buf, offset).expect("read");
            assert!(n > 0, "forward progress at {offset}");
            acc.extend_from_slice(&buf[..n]);
            offset += n as u64;
        }
        assert_eq!(acc, want, "the memo serves the same bytes");

        let inode = backend.inode_for("big.bin").expect("statted above");
        let ContentHandle::SliceMap(slices) = &inode.content_handle else {
            panic!("big.bin must be slab-backed");
        };
        let distinct = slices
            .iter()
            .map(|s| *s.drop_id.as_bytes())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let calls = *backend.decompress_calls.lock().unwrap();
        assert_eq!(
            calls, distinct,
            "one decompress per distinct drop ({distinct}), not per 8 KiB window (25)"
        );
        // A re-read never re-decompresses per window: a fully warm memo
        // (a single-drop file) hits outright; a multi-drop file re-misses
        // at most once per drop (the memo carries only the last).
        let mut offset = 0u64;
        while (offset as usize) < want.len() {
            let n = backend.pread("big.bin", &mut buf, offset).expect("re-read");
            assert!(n > 0, "forward progress at {offset}");
            offset += n as u64;
        }
        let calls = *backend.decompress_calls.lock().unwrap();
        assert!(
            calls <= 2 * distinct,
            "two sweeps cost at most two decompressions per drop ({calls} > 2×{distinct})"
        );
    }

    #[test]
    fn read_dir_lists_direct_children_only() {
        let (_tmp, image) = fixture_tree();
        let backend = LimnifsBackend::from_image(image).expect("mounts");

        let mut root = backend.read_dir("").expect("root lists");
        root.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<(&str, bool)> = root.iter().map(|e| (e.name.as_str(), e.is_dir)).collect();
        assert_eq!(
            names,
            vec![("big.bin", false), ("hello.txt", false), ("sub", true)]
        );
        // No `.`/`..`, and the nested file is not a direct child.
        assert!(!root.iter().any(|e| e.name == "." || e.name == ".."));
        assert!(!root.iter().any(|e| e.name == "nested.txt"));

        let sub = backend.read_dir("sub").expect("sub lists");
        assert_eq!(sub.len(), 1);
        assert_eq!(sub[0].name, "nested.txt");
        assert!(!sub[0].is_dir);

        assert_eq!(backend.read_dir("hello.txt").unwrap_err(), libc::ENOTDIR);
        assert_eq!(backend.read_dir("nope").unwrap_err(), libc::ENOENT);
    }

    #[test]
    fn nested_paths_resolve_through_the_tree() {
        let (_tmp, image) = fixture_tree();
        let backend = LimnifsBackend::from_image(image).expect("mounts");
        let st = backend.stat("sub/nested.txt").expect("nested stats");
        assert_eq!(st.entry_type, EntryType::File);
        assert_eq!(st.size, 19);
        let mut buf = [0u8; 32];
        let n = backend.pread("sub/nested.txt", &mut buf, 0).unwrap();
        assert_eq!(&buf[..n], b"nested content here");
        // Leading-slash tolerance (the mount layer strips, the backend
        // normalizes regardless).
        let n = backend.pread("/sub/nested.txt", &mut buf, 7).unwrap();
        assert_eq!(&buf[..n], b"content here");
    }

    #[test]
    fn symlink_inodes_read_link() {
        let image = symlink_image();
        let backend = LimnifsBackend::from_image(image).expect("mounts");

        let st = backend.stat("link").expect("symlink stats");
        assert_eq!(st.entry_type, EntryType::Symlink);
        assert_eq!(backend.read_link("link").unwrap(), "a.txt");
        assert_eq!(backend.read_link("a.txt").unwrap_err(), libc::EINVAL);
        assert_eq!(backend.read_link("nope").unwrap_err(), libc::ENOENT);
        let mut buf = [0u8; 8];
        assert_eq!(
            backend.pread("link", &mut buf, 0).unwrap_err(),
            libc::EINVAL
        );
    }

    #[test]
    fn corrupt_metadata_checksum_is_einvl_at_open() {
        let (_tmp, mut image) = fixture_tree();
        // Flip the inline metadata blob's last wire byte (a hash
        // mismatch when the blob is stored, a decode failure when it
        // is compressed — Corrupt at mount-open either way → EINVAL).
        let at = metadata_blob_wire_end(&image) - 1;
        image[at] ^= 0xFF;
        assert_eq!(LimnifsBackend::from_image(image).unwrap_err(), libc::EINVAL);
    }

    #[test]
    fn truncated_image_is_einvl_at_open() {
        let (_tmp, image) = fixture_tree();
        let short = image[..image.len() / 2].to_vec();
        assert_eq!(LimnifsBackend::from_image(short).unwrap_err(), libc::EINVAL);
        let empty: Vec<u8> = Vec::new();
        assert_eq!(LimnifsBackend::from_image(empty).unwrap_err(), libc::EINVAL);
    }

    #[test]
    fn trailing_garbage_is_einvl_at_open() {
        let (_tmp, mut image) = fixture_tree();
        image.extend_from_slice(b"garbage");
        assert_eq!(LimnifsBackend::from_image(image).unwrap_err(), libc::EINVAL);
    }

    #[test]
    fn required_feature_flag_is_enotsup_at_open() {
        let (_tmp, mut image) = fixture_tree();
        // The feature-flags section follows the 16-byte header:
        // version(1) + count(4). Declare one required flag.
        image[16] = 1; // section version
        image[17..21].copy_from_slice(&1u32.to_le_bytes());
        let flag = [0x20, 0x00, 0x01]; // id 0x0020, required
        image.splice(21..21, flag);
        assert_eq!(
            LimnifsBackend::from_image(image).unwrap_err(),
            libc::ENOTSUP
        );
    }

    #[test]
    fn external_metadata_locator_is_enotsup_at_open() {
        // A stock limnifs manifest whose metadata is a `file:` sidecar
        // cannot be a tebako payload (one file, byte-identical) — the
        // mount refuses by name, never a silent re-route.
        let image = external_metadata_image();
        assert_eq!(
            LimnifsBackend::from_image(image).unwrap_err(),
            libc::ENOTSUP
        );
    }

    #[test]
    fn image_info_json_reports_sections_and_counts() {
        let (_tmp, image) = fixture_tree();
        let image_len = image.len();
        let backend = LimnifsBackend::from_image(image).expect("mounts");
        let json = backend.image_info_json().expect("limnifs has a surface");
        assert!(json.contains("\"format\":\"limnifs\""), "{json}");
        assert!(json.contains("\"inode_count\":5"), "{json}");
        assert!(json.contains("\"slab_count\":1"), "{json}");
        assert!(
            json.contains(&format!("\"image_bytes\":{image_len}")),
            "{json}"
        );
        assert!(json.contains("METADATA_REFERENCE"), "{json}");
        assert!(json.contains("\"type\":\"SLAB\""), "{json}");
    }

    #[test]
    fn mounts_through_the_three_mount_constructors() {
        let (_tmp, image) = fixture_tree();

        // Memory.
        let mount = crate::mount::build_from_memory(&image, "/mnt").expect("memory mount");
        let st = mount.backend.stat("hello.txt").expect("stats");
        assert_eq!(st.size, 15);

        // Whole file.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fs.tfs");
        std::fs::write(&path, &image).unwrap();
        let mount =
            crate::mount::build_from_file(&path.to_string_lossy(), "/mnt").expect("file mount");
        assert_eq!(mount.backend.name().to_str().unwrap(), "LimniFS");

        // File region (the image embedded after a 4 KiB prefix — the
        // tpkg slot shape).
        let mut packaged = vec![0x5Au8; 4096];
        packaged.extend_from_slice(&image);
        let pkg_path = tmp.path().join("pkg.bin");
        std::fs::write(&pkg_path, &packaged).unwrap();
        let mount = crate::mount::build_from_file_at(
            &pkg_path.to_string_lossy(),
            4096,
            image.len() as u64,
            "/mnt",
        )
        .expect("region mount");
        let mut buf = [0u8; 16];
        let n = mount.backend.pread("hello.txt", &mut buf, 0).unwrap();
        assert_eq!(&buf[..n], b"hello, limnifs\n");
    }

    // ---------------------------------------------------------------
    // fixture encoders (mirroring limnifs-core's own test encoders)
    // ---------------------------------------------------------------

    /// The inline metadata blob's wire end (one past its last byte),
    /// walked with the real parsers. The blob is the reference section's
    /// trailing field, so this is inside it for any non-empty blob.
    fn metadata_blob_wire_end(image: &[u8]) -> usize {
        let mut cursor = ManifestCursor::new(image);
        let _ = parse_manifest_header(&mut cursor).unwrap();
        let _ = parse_feature_flags_section(&mut cursor).unwrap();
        let reference = parse_metadata_reference(&mut cursor).unwrap();
        assert!(reference.is_inlined());
        cursor.position()
    }

    fn encode_inode(number: u64, mode: u32, mtime_ns: u64, flags: u8, body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&number.to_le_bytes());
        bytes.extend_from_slice(&mode.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes()); // uid
        bytes.extend_from_slice(&0u32.to_le_bytes()); // gid
        bytes.extend_from_slice(&mtime_ns.to_le_bytes());
        bytes.extend_from_slice(&mtime_ns.to_le_bytes()); // ctime
        bytes.extend_from_slice(&1u32.to_le_bytes()); // nlink
        bytes.push(flags);
        bytes.extend_from_slice(body);
        bytes
    }

    fn encode_dir_node(entries: &[(&str, u64, u8)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(1u8); // DIRECTORY_NODE_VERSION
        bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (name, number, ty) in entries {
            bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(&number.to_le_bytes());
            bytes.push(*ty);
        }
        bytes
    }

    /// Assemble a minimal manifest (header + empty flags + inline
    /// metadata reference v1 + empty slab index + the mandatory single
    /// build history entry) around a hand-built metadata blob.
    fn assemble_manifest(blob: &[u8]) -> Vec<u8> {
        let mut image = Vec::new();
        image.extend_from_slice(&ManifestHeader::current().to_bytes());
        image.push(1u8); // feature flags section version
        image.extend_from_slice(&0u32.to_le_bytes());
        image.push(1u8); // metadata_reference section version 1
        image.extend_from_slice(&limnifs_core::hash_section(blob));
        image.extend_from_slice(&0u32.to_le_bytes()); // no locators
        image.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        image.extend_from_slice(blob);
        image.push(1u8); // slab_index section version
        image.extend_from_slice(&0u32.to_le_bytes());
        append_build_history(&mut image);
        image
    }

    /// The writer's mandatory history section: version 1, one build
    /// entry (op 0x01, zero epoch, no inputs, no params) — an empty
    /// history is Corrupt ("every image has a build entry").
    fn append_build_history(image: &mut Vec<u8>) {
        image.push(1u8); // history section version
        image.extend_from_slice(&1u32.to_le_bytes());
        image.push(0x01); // build op
        image.extend_from_slice(&0u64.to_le_bytes());
        image.extend_from_slice(&0u32.to_le_bytes());
        image.extend_from_slice(&0u32.to_le_bytes());
    }

    /// An image with one inline file and one symlink at the root —
    /// hand-encoded because limnifs-write v0.2 does not walk symlinks.
    fn symlink_image() -> Vec<u8> {
        const S_IFREG: u32 = 0o100_000;
        const S_IFDIR: u32 = 0o040_000;
        const S_IFLNK: u32 = 0o120_000;
        const INLINE: u8 = 0x04; // INODE_FLAG_INLINE_DATA

        let file_body: Vec<u8> = {
            let mut b = (5u32).to_le_bytes().to_vec();
            b.extend_from_slice(b"data!");
            b
        };
        let link_body: Vec<u8> = {
            let mut b = (5u32).to_le_bytes().to_vec();
            b.extend_from_slice(b"a.txt");
            b
        };
        let node = encode_dir_node(&[("a.txt", 2, 0x01), ("link", 3, 0x03)]);
        let node_hash = limnifs_core::metadata::dir_node_hash(&[
            limnifs_core::DirEntry {
                name: "a.txt".to_string(),
                inode_number: 2,
                entry_type: 0x01,
            },
            limnifs_core::DirEntry {
                name: "link".to_string(),
                inode_number: 3,
                entry_type: 0x03,
            },
        ]);

        let mut blob = Vec::new();
        blob.extend_from_slice(&3u32.to_le_bytes());
        blob.extend_from_slice(&encode_inode(1, S_IFDIR | 0o755, 0, 0, &node_hash));
        blob.extend_from_slice(&encode_inode(2, S_IFREG | 0o644, 0, INLINE, &file_body));
        blob.extend_from_slice(&encode_inode(3, S_IFLNK | 0o777, 0, 0, &link_body));
        blob.extend_from_slice(&1u32.to_le_bytes());
        blob.extend_from_slice(&node);
        assemble_manifest(&blob)
    }

    /// A manifest whose metadata lives behind a `file:` locator.
    fn external_metadata_image() -> Vec<u8> {
        let blob: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0]; // empty blob (unused)
        let mut image = Vec::new();
        image.extend_from_slice(&ManifestHeader::current().to_bytes());
        image.push(1u8);
        image.extend_from_slice(&0u32.to_le_bytes());
        image.push(1u8); // metadata_reference v1
        image.extend_from_slice(&limnifs_core::hash_section(&blob));
        image.extend_from_slice(&1u32.to_le_bytes()); // one locator
        let uri = b"file:metadata.bin";
        image.extend_from_slice(&(uri.len() as u32).to_le_bytes());
        image.extend_from_slice(uri);
        image.extend_from_slice(&0u32.to_le_bytes()); // no inline data
        image.push(1u8); // slab_index
        image.extend_from_slice(&0u32.to_le_bytes());
        append_build_history(&mut image);
        image
    }
}
