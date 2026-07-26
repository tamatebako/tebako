//! TAR backend (read-only): plain `tar`, `tar.gz` and `tar.zst` images via
//! a mount-time offset index built in ONE streaming pass (spec 11 §3).
//!
//! There is no C++ tar oracle; semantics are defined here:
//! - directories are SYNTHESIZED from entry paths (a tar of `a/b/c` has
//!   `a/` and `a/b/` even without explicit entries; explicit entries later
//!   replace implicit ones — tar's append semantics: last entry wins)
//! - names are normalized component-wise (`./x` → `x`, `a//b` → `a/b`,
//!   absolute paths rooted); entries with `..` components or non-UTF-8
//!   names are not exposed (the C ABI path space cannot address them)
//! - hard links resolve to their target's data at lookup time; a link with
//!   no indexed target stats as `Other` (EINVAL through the C ABI, like
//!   every non-file/dir in `fill_stat`)
//! - GNU sparse files stat normally but refuse pread with ENOTSUP (their
//!   raw bytes are the sparse map, not the apparent content)
//! - permissions come from the header (0 when absent → 0644/0755 defaults);
//!   mtime from the header
//!
//! ## Memory profile (the 1 GiB / < 64 MiB RSS budget)
//!
//! - **Index**: O(entry count), not archive size — one `TarEntry`
//!   (~100–150 B) plus two `BTreeMap` node sets per path. A 1 GiB archive
//!   of 256 × 4 MiB files costs ~50 KiB; ~100k entries stay ≈ 15–25 MiB.
//!   The mount pass itself streams (64 KiB buffers); with a seekable source
//!   entry data is skipped with `seek`, not read.
//! - **Plain tar**: zero extra state; reads are positioned reads
//!   (`pread`-style) on the file — RSS does not grow with archive size.
//! - **tar.gz**: deflate has no random access, so the mount pass snapshots
//!   the (pure-Rust, cloneable) miniz_oxide `InflateState` every
//!   [`GZ_CHECKPOINT_SPACING`] uncompressed bytes — the zran pattern.
//!   Checkpoints cost ~34 KiB each ≈ 0.2 % of the uncompressed size
//!   (1 GiB → 64 checkpoints ≈ 2.2 MiB, capped at [`GZ_MAX_CHECKPOINTS`]).
//!   A cold read resumes at the nearest checkpoint and decodes at most one
//!   spacing; a shared cursor keeps sequential access (extraction) O(n).
//!   gzip headers are parsed by hand (single-member; trailing members of a
//!   multi-member stream are ignored); the gzip CRC32 is not verified —
//!   integrity is anchored in the tar header checksums during the scan.
//! - **tar.zst**: ruzstd exposes no decoder-state snapshots, so random
//!   access is a shared forward-only cursor: reads at or ahead of the
//!   cursor decode forward (O(n) total for sequential patterns), a cold
//!   BACKWARD seek re-decodes from the stream start. Memory is bounded by
//!   the frame's declared window (typically ≤ 8 MiB) plus 64 KiB I/O
//!   buffers. Plain single-frame `.tar.zst` is fundamentally sequential —
//!   prefer plain tar or tar.gz for random-access workloads.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom, Take};
use std::sync::{Arc, Mutex};

use miniz_oxide::inflate::stream::{inflate, InflateState};
use miniz_oxide::{DataFormat, MZFlush, MZStatus};
use ruzstd::StreamingDecoder;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat};

/// Default I/O chunk for decompression pumps and buffered readers.
const IO_CHUNK: usize = 64 * 1024;
/// Uncompressed bytes between two gzip checkpoints (see module docs).
const GZ_CHECKPOINT_SPACING: u64 = 16 * 1024 * 1024;
/// Hard cap on the gzip checkpoint count (degenerate images stay mountable;
/// reads just fall back to longer resumes).
const GZ_MAX_CHECKPOINTS: usize = 4096;
/// Cap for NUL-terminated gzip header fields (FNAME/FCOMMENT).
const GZ_HEADER_FIELD_MAX: u64 = 1024 * 1024;

/// Compression envelope around the tar stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TarCompression {
    /// Plain tar.
    None,
    /// gzip-wrapped.
    Gzip,
    /// zstd-frame-wrapped.
    Zstd,
}

// ===================================================================
// Source: random-access bytes (whole file / file region / memory)
// ===================================================================

/// Owned, `Send + Sync` positioned reader over a memory image.
struct SliceReader {
    data: Arc<Vec<u8>>,
    pos: usize,
}

impl Read for SliceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let avail = &self.data[self.pos..];
        let n = avail.len().min(buf.len());
        buf[..n].copy_from_slice(&avail[..n]);
        self.pos += n;
        Ok(n)
    }
}

/// A file (or region of it) as a positioned-read source.
struct FileSource {
    file: File,
    /// Absolute offset of byte 0 of the source (region mounts).
    base: u64,
    /// Source length in bytes.
    len: u64,
    /// Serializes seek+read on platforms without positioned reads.
    #[cfg_attr(unix, allow(dead_code))]
    seek_lock: Mutex<()>,
}

impl FileSource {
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), i32> {
        let end = offset.checked_add(buf.len() as u64).ok_or(libc::EINVAL)?;
        if end > self.len {
            return Err(libc::EIO);
        }
        let mut abs = self.base.checked_add(offset).ok_or(libc::EINVAL)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileExt as _;
            let mut buf = buf;
            while !buf.is_empty() {
                let n = self.file.read_at(buf, abs).map_err(|_| libc::EIO)?;
                if n == 0 {
                    return Err(libc::EIO); // short read before the region end
                }
                abs += n as u64;
                buf = &mut buf[n..];
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _guard = self.seek_lock.lock().map_err(|_| libc::EIO)?;
            (&self.file)
                .seek(SeekFrom::Start(abs))
                .map_err(|_| libc::EIO)?;
            (&self.file).read_exact(buf).map_err(|_| libc::EIO)
        }
    }
}

/// The compressed/plain byte source behind a [`TarStream`].
enum Source {
    File(FileSource),
    Memory(Arc<Vec<u8>>),
}

impl Source {
    fn len(&self) -> u64 {
        match self {
            Source::File(f) => f.len,
            Source::Memory(d) => d.len() as u64,
        }
    }

    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), i32> {
        match self {
            Source::File(f) => f.read_exact_at(offset, buf),
            Source::Memory(d) => {
                let end = offset.checked_add(buf.len() as u64).ok_or(libc::EINVAL)?;
                if end > d.len() as u64 {
                    return Err(libc::EIO);
                }
                let start = offset as usize;
                buf.copy_from_slice(&d[start..start + buf.len()]);
                Ok(())
            }
        }
    }

    /// A fresh buffered stream positioned at `offset`, bounded to the
    /// source end (regions cannot read past their declared length).
    fn reader_at(&self, offset: u64) -> Result<BufReader<Box<dyn Read + Send>>, i32> {
        if offset > self.len() {
            return Err(libc::EINVAL);
        }
        match self {
            Source::Memory(d) => Ok(BufReader::with_capacity(
                IO_CHUNK,
                Box::new(SliceReader {
                    data: Arc::clone(d),
                    pos: offset as usize,
                }) as Box<dyn Read + Send>,
            )),
            Source::File(f) => {
                let mut clone = f.file.try_clone().map_err(|_| libc::EIO)?;
                let abs = f.base.checked_add(offset).ok_or(libc::EINVAL)?;
                clone.seek(SeekFrom::Start(abs)).map_err(|_| libc::EIO)?;
                let bounded: Take<File> = clone.take(f.len - offset);
                Ok(BufReader::with_capacity(IO_CHUNK, Box::new(bounded)))
            }
        }
    }
}

// ===================================================================
// gzip: header parse, checkpointed cursor, positioned reads
// ===================================================================

/// A resumable point in the deflate stream: all input consumed, all output
/// produced (captured at input-buffer boundaries only, so `c_offset` is
/// exact and `state` is self-consistent).
struct GzCheckpoint {
    /// Uncompressed offset.
    u_offset: u64,
    /// Absolute source offset of the next compressed byte.
    c_offset: u64,
    /// Inflater state at (c_offset, u_offset).
    state: Box<InflateState>,
}

/// The live decompression cursor (one per reader; shared across preads
/// under a mutex so sequential scans never re-decode).
struct GzCursor {
    state: Box<InflateState>,
    reader: BufReader<Box<dyn Read + Send>>,
    in_buf: Vec<u8>,
    in_pos: usize,
    in_len: usize,
    /// Compressed source exhausted without a StreamEnd (truncated image).
    src_eof: bool,
    /// Deflate StreamEnd reached.
    stream_eof: bool,
    /// Uncompressed bytes produced so far.
    u_pos: u64,
}

impl GzCursor {
    fn new(
        source: &Source,
        c_offset: u64,
        state: Box<InflateState>,
        u_pos: u64,
        io_chunk: usize,
    ) -> Result<GzCursor, i32> {
        Ok(GzCursor {
            state,
            reader: source.reader_at(c_offset)?,
            in_buf: vec![0u8; io_chunk.max(512)],
            in_pos: 0,
            in_len: 0,
            src_eof: false,
            stream_eof: false,
            u_pos,
        })
    }
}

/// Checkpoint capture during the mount scan.
struct GzCapture {
    /// Source offset where the deflate stream starts (after the gzip header).
    c0: u64,
    /// Compressed bytes fed to inflate so far (exact at capture points).
    c_fed: u64,
    spacing: u64,
    last_cp_u: u64,
    cps: Vec<GzCheckpoint>,
}

impl GzCapture {
    fn maybe_capture(&mut self, cur: &GzCursor) {
        if cur.u_pos - self.last_cp_u >= self.spacing && self.cps.len() < GZ_MAX_CHECKPOINTS {
            self.cps.push(GzCheckpoint {
                u_offset: cur.u_pos,
                c_offset: self.c0 + self.c_fed,
                state: cur.state.clone(),
            });
            self.last_cp_u = cur.u_pos;
        }
    }
}

/// Decompress into `out`, returning the number of bytes written
/// (< `out.len()` only at stream end). Errors: EIO on corrupt/truncated
/// input (the mount scan maps any failure to EINVAL — not a valid image).
fn gz_pump(
    cur: &mut GzCursor,
    mut out: &mut [u8],
    mut cap: Option<&mut GzCapture>,
) -> Result<usize, i32> {
    let mut written = 0usize;
    while !out.is_empty() && !cur.stream_eof {
        if cur.in_pos == cur.in_len {
            if cur.src_eof {
                return Err(libc::EIO); // truncated: source ended mid-stream
            }
            // Consistent boundary: every fed byte was consumed by inflate.
            if let Some(c) = cap.as_mut() {
                c.maybe_capture(cur);
            }
            cur.in_len = cur.reader.read(&mut cur.in_buf).map_err(|_| libc::EIO)?;
            cur.in_pos = 0;
            if cur.in_len == 0 {
                cur.src_eof = true;
                continue;
            }
        }
        let res = inflate(
            &mut cur.state,
            &cur.in_buf[cur.in_pos..cur.in_len],
            out,
            MZFlush::None,
        );
        cur.in_pos += res.bytes_consumed;
        if let Some(c) = cap.as_mut() {
            c.c_fed += res.bytes_consumed as u64;
        }
        let status = res.status.map_err(|_| libc::EIO)?;
        cur.u_pos += res.bytes_written as u64;
        written += res.bytes_written;
        out = &mut out[res.bytes_written..];
        if status == MZStatus::StreamEnd {
            cur.stream_eof = true;
        } else if res.bytes_consumed == 0 && res.bytes_written == 0 && cur.in_pos != cur.in_len {
            return Err(libc::EIO); // stalled with input left: corrupt stream
        }
    }
    Ok(written)
}

/// The mount scan: tar parsing drives decompression; checkpoints accrete.
struct GzScan {
    cur: GzCursor,
    cap: GzCapture,
}

impl Read for GzScan {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        gz_pump(&mut self.cur, out, Some(&mut self.cap)).map_err(io::Error::from_raw_os_error)
    }
}

/// Parse the gzip member header; returns the deflate stream's source
/// offset. EINVAL when the magic/method is not gzip-deflate.
fn parse_gzip_header(source: &Source) -> Result<u64, i32> {
    let mut hdr = [0u8; 10];
    source
        .read_exact_at(0, &mut hdr)
        .map_err(|_| libc::EINVAL)?;
    if hdr[0] != 0x1f || hdr[1] != 0x8b || hdr[2] != 8 {
        return Err(libc::EINVAL);
    }
    let flags = hdr[3];
    let mut pos = 10u64;
    if flags & 0x04 != 0 {
        // FEXTRA: 2-byte little-endian length + payload
        let mut l = [0u8; 2];
        source
            .read_exact_at(pos, &mut l)
            .map_err(|_| libc::EINVAL)?;
        pos += 2 + u64::from(u16::from_le_bytes(l));
    }
    for flag in [0x08u8, 0x10u8] {
        // FNAME, FCOMMENT: NUL-terminated strings
        if flags & flag != 0 {
            pos = scan_nul(source, pos)?;
        }
    }
    if flags & 0x02 != 0 {
        pos += 2; // FHCRC
    }
    Ok(pos)
}

/// Advance past a NUL-terminated field starting at `pos` (bounds-checked).
fn scan_nul(source: &Source, mut pos: u64) -> Result<u64, i32> {
    let mut buf = [0u8; 256];
    let mut scanned = 0u64;
    loop {
        if scanned >= GZ_HEADER_FIELD_MAX {
            return Err(libc::EINVAL);
        }
        let want = (source.len() - pos).min(buf.len() as u64) as usize;
        if want == 0 {
            return Err(libc::EINVAL);
        }
        source
            .read_exact_at(pos, &mut buf[..want])
            .map_err(|_| libc::EINVAL)?;
        if let Some(i) = buf[..want].iter().position(|&b| b == 0) {
            return Ok(pos + i as u64 + 1);
        }
        pos += want as u64;
        scanned += want as u64;
    }
}

/// The gzip random-access state behind a mounted `tar.gz`.
struct GzRandom {
    source: Source,
    checkpoints: Vec<GzCheckpoint>,
    cursor: Mutex<GzCursor>,
    io_chunk: usize,
}

/// Read `buf` fully at uncompressed offset `target`.
fn gz_read_at(gz: &GzRandom, target: u64, buf: &mut [u8]) -> Result<(), i32> {
    let mut cur = gz.cursor.lock().map_err(|_| libc::EIO)?;
    // Nearest checkpoint at or before the target (checkpoints[0] is the
    // stream start, so the index always exists).
    let idx = gz.checkpoints.partition_point(|cp| cp.u_offset <= target) - 1;
    let cp = &gz.checkpoints[idx];
    let keep_cursor =
        !cur.stream_eof && cur.u_pos <= target && (target - cur.u_pos) <= (target - cp.u_offset);
    if !keep_cursor {
        let fresh = GzCursor::new(
            &gz.source,
            cp.c_offset,
            cp.state.clone(),
            cp.u_offset,
            gz.io_chunk,
        )?;
        *cur = fresh;
    }
    let mut scratch = [0u8; IO_CHUNK];
    let mut remain = target - cur.u_pos;
    while remain > 0 {
        let n = gz_pump(
            &mut cur,
            &mut scratch[..(remain as usize).min(IO_CHUNK)],
            None,
        )?;
        if n == 0 {
            return Err(libc::EIO);
        }
        remain -= n as u64;
    }
    let mut got = 0usize;
    while got < buf.len() {
        let n = gz_pump(&mut cur, &mut buf[got..], None)?;
        if n == 0 {
            return Err(libc::EIO);
        }
        got += n;
    }
    Ok(())
}

// ===================================================================
// zstd: shared forward cursor (see module docs for the cost model)
// ===================================================================

type ZstDecoder = StreamingDecoder<BufReader<Box<dyn Read + Send>>, ruzstd::FrameDecoder>;

struct ZstCursor {
    dec: ZstDecoder,
    /// Uncompressed bytes produced so far.
    u_pos: u64,
}

fn zst_cursor(source: &Source) -> Result<ZstCursor, i32> {
    let reader = source.reader_at(0)?;
    let dec = StreamingDecoder::new(reader).map_err(|_| libc::EINVAL)?;
    Ok(ZstCursor { dec, u_pos: 0 })
}

struct ZstRandom {
    source: Source,
    cursor: Mutex<ZstCursor>,
}

/// Read `buf` fully at uncompressed offset `target` (backward seeks
/// re-decode from the stream start — the plain-`.zst` cost model).
fn zst_read_at(zr: &ZstRandom, target: u64, buf: &mut [u8]) -> Result<(), i32> {
    let mut cur = zr.cursor.lock().map_err(|_| libc::EIO)?;
    if target < cur.u_pos {
        *cur = zst_cursor(&zr.source).map_err(|_| libc::EIO)?;
    }
    let mut scratch = [0u8; IO_CHUNK];
    let mut remain = target - cur.u_pos;
    while remain > 0 {
        let n = cur
            .dec
            .read(&mut scratch[..(remain as usize).min(IO_CHUNK)])
            .map_err(|_| libc::EIO)?;
        if n == 0 {
            return Err(libc::EIO);
        }
        cur.u_pos += n as u64;
        remain -= n as u64;
    }
    let mut got = 0usize;
    while got < buf.len() {
        let n = cur.dec.read(&mut buf[got..]).map_err(|_| libc::EIO)?;
        if n == 0 {
            return Err(libc::EIO);
        }
        cur.u_pos += n as u64;
        got += n;
    }
    Ok(())
}

/// Counts bytes for the mount scan (cursor bookkeeping after parsing).
struct CountingRead<'a, R: Read> {
    inner: &'a mut R,
    n: u64,
}

impl<R: Read> Read for CountingRead<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.n += n as u64;
        Ok(n)
    }
}

// ===================================================================
// The offset index
// ===================================================================

#[derive(Debug, PartialEq)]
enum TarEntryKind {
    /// `readable` is false for GNU sparse entries (raw bytes ≠ content).
    File {
        data_offset: u64,
        size: u64,
        readable: bool,
    },
    Directory,
    Symlink,
    HardLink {
        target: String,
    },
    Other,
}

#[derive(Debug)]
struct TarEntry {
    /// Normalized full path (no leading/trailing `/`).
    name: String,
    kind: TarEntryKind,
    perms: u32,
    mtime: i64,
}

#[derive(Default)]
struct IndexBuild {
    entries: Vec<TarEntry>,
    by_path: BTreeMap<String, usize>,
    /// Directory path ("" = root) → indices of its direct children.
    children: BTreeMap<String, Vec<usize>>,
}

impl IndexBuild {
    /// Insert one entry, synthesizing implicit ancestor directories and
    /// applying tar's last-entry-wins rule for duplicates.
    fn insert(&mut self, entry: TarEntry) {
        let comps: Vec<&str> = entry.name.split('/').collect();
        let mut prefix = String::new();
        for comp in &comps[..comps.len() - 1] {
            let parent = prefix.clone();
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(comp);
            if !self.by_path.contains_key(&prefix) {
                let idx = self.entries.len();
                self.entries.push(TarEntry {
                    name: prefix.clone(),
                    kind: TarEntryKind::Directory,
                    perms: 0o755,
                    mtime: 0,
                });
                self.by_path.insert(prefix.clone(), idx);
                self.children.entry(parent).or_default().push(idx);
            }
        }
        let parent = match entry.name.rfind('/') {
            Some(i) => entry.name[..i].to_string(),
            None => String::new(),
        };
        if let Some(&old) = self.by_path.get(&entry.name) {
            // Later entry wins (tar append semantics); the children slot is
            // already registered for this index.
            self.entries[old] = entry;
        } else {
            let idx = self.entries.len();
            self.by_path.insert(entry.name.clone(), idx);
            self.entries.push(entry);
            self.children.entry(parent).or_default().push(idx);
        }
    }
}

/// Normalize an in-image path component-wise; `None` for paths that must
/// not be exposed (non-UTF-8, `..` components, empty/root pseudo-names).
fn normalize_name(bytes: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(bytes).ok()?;
    let mut out = String::with_capacity(s.len());
    for comp in s.split('/') {
        match comp {
            "" | "." => {}
            ".." => return None,
            c => {
                if !out.is_empty() {
                    out.push('/');
                }
                out.push_str(c);
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Build the offset index from a tar entry stream.
fn index_entries<'a, R, I>(entries: I) -> Result<IndexBuild, i32>
where
    R: Read + 'a,
    I: Iterator<Item = io::Result<tar::Entry<'a, R>>>,
{
    let mut build = IndexBuild::default();
    for entry in entries {
        let entry = entry.map_err(|_| libc::EINVAL)?;
        let header = entry.header();
        let Some(name) = normalize_name(&entry.path_bytes()) else {
            continue;
        };
        let mode = header.mode().unwrap_or(0) & 0o7777;
        let mtime = header.mtime().unwrap_or(0).min(i64::MAX as u64) as i64;
        let kind = match header.entry_type() {
            tar::EntryType::Regular | tar::EntryType::Continuous => TarEntryKind::File {
                data_offset: entry.raw_file_position(),
                size: header.size().unwrap_or(0),
                readable: true,
            },
            tar::EntryType::GNUSparse => TarEntryKind::File {
                data_offset: entry.raw_file_position(),
                size: header.size().unwrap_or(0),
                readable: false,
            },
            tar::EntryType::Directory => TarEntryKind::Directory,
            tar::EntryType::Symlink => TarEntryKind::Symlink,
            tar::EntryType::Link => match header.link_name_bytes().and_then(|t| normalize_name(&t))
            {
                Some(target) => TarEntryKind::HardLink { target },
                None => TarEntryKind::Other,
            },
            _ => TarEntryKind::Other,
        };
        let perms = match (mode, &kind) {
            (0, TarEntryKind::Directory) => 0o755,
            (0, TarEntryKind::Symlink) => 0o777,
            (0, _) => 0o644,
            (m, _) => m,
        };
        build.insert(TarEntry {
            name,
            kind,
            perms,
            mtime,
        });
    }
    Ok(build)
}

/// Streaming index pass (compressed sources and bounded regions).
fn index_from_reader<R: Read>(reader: R) -> Result<IndexBuild, i32> {
    let mut archive = tar::Archive::new(reader);
    let entries = archive.entries().map_err(|_| libc::EINVAL)?;
    index_entries(entries)
}

/// Index pass with O(1) data skips (seekable plain-tar sources).
fn index_from_seekable<R: Read + Seek>(reader: R) -> Result<IndexBuild, i32> {
    let mut archive = tar::Archive::new(reader);
    let entries = archive.entries_with_seek().map_err(|_| libc::EINVAL)?;
    index_entries(entries)
}

/// Index a plain-tar source: seek-skips for whole files / memory images,
/// bounded streaming for regions.
fn index_plain(source: &Source) -> Result<IndexBuild, i32> {
    let build = match source {
        Source::Memory(data) => index_from_seekable(Cursor::new(data.as_slice())),
        Source::File(f)
            if f.base == 0 && f.len == f.file.metadata().map_err(|_| libc::EIO)?.len() =>
        {
            // The shared handle may sit anywhere (a caller's sniff pass
            // consumed the first block): index from the region base, not
            // from the inherited position.
            let mut clone = f.file.try_clone().map_err(|_| libc::EIO)?;
            clone.seek(SeekFrom::Start(0)).map_err(|_| libc::EIO)?;
            index_from_seekable(clone)
        }
        _ => index_from_reader(source.reader_at(0)?),
    }?;
    // Seek-skips can jump past EOF silently: a truncated image would
    // otherwise mount with its tail entry present but unreadable.
    for e in &build.entries {
        if let TarEntryKind::File {
            data_offset, size, ..
        } = &e.kind
        {
            match data_offset.checked_add(*size) {
                Some(end) if end <= source.len() => {}
                _ => return Err(libc::EINVAL),
            }
        }
    }
    Ok(build)
}

// ===================================================================
// The backend
// ===================================================================

/// The byte stream behind the index.
enum TarStream {
    Plain(Source),
    Gz(Box<GzRandom>),
    Zst(Box<ZstRandom>),
}

/// Mounted tar/tar.gz/tar.zst archive (read-only).
pub struct TarBackend {
    stream: TarStream,
    entries: Vec<TarEntry>,
    by_path: BTreeMap<String, usize>,
    children: BTreeMap<String, Vec<usize>>,
    format: &'static CStr,
    compression: &'static str,
    uncompressed_size: u64,
}

impl TarBackend {
    /// Mount a whole file.
    pub fn from_file(file: File, compression: TarCompression) -> Result<TarBackend, i32> {
        let len = file.metadata().map_err(|_| libc::EIO)?.len();
        Self::from_source(
            Source::File(FileSource {
                file,
                base: 0,
                len,
                seek_lock: Mutex::new(()),
            }),
            compression,
        )
    }

    /// Mount `length` bytes starting at `offset` of a file (region mount).
    pub fn from_file_at(
        file: File,
        offset: u64,
        length: u64,
        compression: TarCompression,
    ) -> Result<TarBackend, i32> {
        Self::from_source(
            Source::File(FileSource {
                file,
                base: offset,
                len: length,
                seek_lock: Mutex::new(()),
            }),
            compression,
        )
    }

    /// Mount an in-memory image (owned).
    pub fn from_memory(data: Vec<u8>, compression: TarCompression) -> Result<TarBackend, i32> {
        Self::from_source(Source::Memory(Arc::new(data)), compression)
    }

    fn from_source(source: Source, compression: TarCompression) -> Result<TarBackend, i32> {
        Self::from_source_cfg(source, compression, GZ_CHECKPOINT_SPACING, IO_CHUNK)
    }

    /// Constructor with tunable gzip checkpoint parameters (tests shrink
    /// them to exercise the resume machinery on small inputs).
    fn from_source_cfg(
        source: Source,
        compression: TarCompression,
        gz_spacing: u64,
        gz_io_chunk: usize,
    ) -> Result<TarBackend, i32> {
        let (stream, build, format, comp_name, uncompressed_size) = match compression {
            TarCompression::None => {
                let uncompressed_size = source.len();
                let build = index_plain(&source)?;
                (
                    TarStream::Plain(source),
                    build,
                    c"TAR",
                    "none",
                    uncompressed_size,
                )
            }
            TarCompression::Gzip => {
                let c0 = parse_gzip_header(&source)?;
                let state0 = Box::new(InflateState::new(DataFormat::Raw));
                let cur = GzCursor::new(&source, c0, state0.clone(), 0, gz_io_chunk)?;
                let cap = GzCapture {
                    c0,
                    c_fed: 0,
                    spacing: gz_spacing,
                    last_cp_u: 0,
                    cps: vec![GzCheckpoint {
                        u_offset: 0,
                        c_offset: c0,
                        state: state0,
                    }],
                };
                let mut scan = GzScan { cur, cap };
                let build = index_from_reader(&mut scan)?;
                let GzScan { cur, cap } = scan;
                let uncompressed_size = cur.u_pos;
                (
                    TarStream::Gz(Box::new(GzRandom {
                        source,
                        checkpoints: cap.cps,
                        cursor: Mutex::new(cur),
                        io_chunk: gz_io_chunk,
                    })),
                    build,
                    c"TAR.GZ",
                    "gzip",
                    uncompressed_size,
                )
            }
            TarCompression::Zstd => {
                let mut cur = zst_cursor(&source)?;
                let build = {
                    let mut cr = CountingRead {
                        inner: &mut cur.dec,
                        n: 0,
                    };
                    let build = index_from_reader(&mut cr)?;
                    cur.u_pos = cr.n;
                    build
                };
                let uncompressed_size = cur.u_pos;
                (
                    TarStream::Zst(Box::new(ZstRandom {
                        source,
                        cursor: Mutex::new(cur),
                    })),
                    build,
                    c"TAR.ZST",
                    "zstd",
                    uncompressed_size,
                )
            }
        };
        Ok(TarBackend {
            stream,
            entries: build.entries,
            by_path: build.by_path,
            children: build.children,
            format,
            compression: comp_name,
            uncompressed_size,
        })
    }

    /// Resolve a path to (size, data_offset, readable) for regular files,
    /// following hard links one hop (links to links are not chained by tar).
    fn file_entry(&self, path: &str) -> Option<(u64, u64, bool)> {
        let entry = self.entries.get(*self.by_path.get(path)?)?;
        match &entry.kind {
            TarEntryKind::File {
                data_offset,
                size,
                readable,
            } => Some((*size, *data_offset, *readable)),
            TarEntryKind::HardLink { target } => {
                let entry = self.entries.get(*self.by_path.get(target)?)?;
                match &entry.kind {
                    TarEntryKind::File {
                        data_offset,
                        size,
                        readable,
                    } => Some((*size, *data_offset, *readable)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// Normalize an in-image path: no leading or trailing `/`, `""` for root.
fn normalize(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

impl Backend for TarBackend {
    fn name(&self) -> &'static CStr {
        self.format
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let path = normalize(path);
        if path.is_empty() {
            return Ok(RawStat {
                entry_type: EntryType::Directory,
                perms: 0o755,
                size: 0,
                mtime: 0,
            });
        }
        let entry = self
            .entries
            .get(*self.by_path.get(path).ok_or(libc::ENOENT)?)
            .ok_or(libc::ENOENT)?;
        let (entry_type, size) = match &entry.kind {
            TarEntryKind::File { size, .. } => (EntryType::File, *size as i64),
            TarEntryKind::Directory => (EntryType::Directory, 0),
            TarEntryKind::Symlink => (EntryType::Symlink, 0),
            TarEntryKind::HardLink { .. } => match self.file_entry(path) {
                Some((size, _, _)) => (EntryType::File, size as i64),
                None => (EntryType::Other, 0),
            },
            TarEntryKind::Other => (EntryType::Other, 0),
        };
        Ok(RawStat {
            entry_type,
            perms: entry.perms,
            size,
            mtime: entry.mtime,
        })
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        match self.by_path.get(path).map(|i| &self.entries[*i].kind) {
            Some(TarEntryKind::Directory) => return Err(libc::EISDIR),
            Some(TarEntryKind::Other) | Some(TarEntryKind::Symlink) => return Err(libc::EINVAL),
            None => return Err(libc::ENOENT),
            _ => {}
        }
        let (size, data_offset, readable) = self.file_entry(path).ok_or(libc::ENOENT)?;
        if !readable {
            return Err(libc::ENOTSUP);
        }
        if offset >= size || buf.is_empty() {
            return Ok(0);
        }
        let want = (buf.len() as u64).min(size - offset) as usize;
        let at = data_offset + offset;
        match &self.stream {
            TarStream::Plain(source) => source.read_exact_at(at, &mut buf[..want])?,
            TarStream::Gz(gz) => gz_read_at(gz, at, &mut buf[..want])?,
            TarStream::Zst(zr) => zst_read_at(zr, at, &mut buf[..want])?,
        }
        Ok(want)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        let path = normalize(path);
        if !path.is_empty() {
            match self.by_path.get(path).map(|i| &self.entries[*i].kind) {
                Some(TarEntryKind::Directory) => {}
                Some(_) => return Err(libc::ENOTDIR),
                None => return Err(libc::ENOENT),
            }
        }
        let mut out = Vec::new();
        if let Some(kids) = self.children.get(path) {
            for &i in kids {
                let entry = &self.entries[i];
                let name = entry.name.rsplit('/').next().unwrap_or(&entry.name);
                out.push(RawDirEntry {
                    name: name.to_string(),
                    is_dir: entry.kind == TarEntryKind::Directory,
                });
            }
        }
        Ok(out)
    }

    fn image_info_json(&self) -> Option<String> {
        Some(format!(
            "{{\"format\":\"tar\",\"compression\":\"{}\",\"entries\":{},\"uncompressed_size\":{}}}",
            self.compression,
            self.entries.len(),
            self.uncompressed_size
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{detect_format, ImageFormat};

    // ---------------------------------------------------------------
    // Fixture builders
    // ---------------------------------------------------------------

    fn splitmix(mut z: u64) -> u64 {
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// (path, content) pairs appended in order (duplicates allowed).
    fn make_tar(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        for (path, data) in files {
            let mut h = tar::Header::new_gnu();
            h.set_entry_type(tar::EntryType::Regular);
            h.set_mode(0o644);
            h.set_mtime(1_234_567_890);
            h.set_size(data.len() as u64);
            b.append_data(&mut h, path, *data).unwrap();
        }
        b.finish().unwrap();
        b.into_inner().unwrap()
    }

    fn append_dir(b: &mut tar::Builder<Vec<u8>>, path: &str) {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Directory);
        h.set_mode(0o755);
        h.set_mtime(1_234_567_800);
        h.set_size(0);
        b.append_data(&mut h, path, io::empty()).unwrap();
    }

    /// Wrap bytes in a single-member gzip stream (level-6 deflate).
    fn gzip_bytes(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
        out.extend(miniz_oxide::deflate::compress_to_vec(data, 6));
        out.extend(crc32fast::hash(data).to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out
    }

    fn fixture_zst() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/small.tar.zst"
        ))
        .unwrap()
    }

    fn pread_all(b: &TarBackend, path: &str) -> Vec<u8> {
        let st = b.stat(path).unwrap();
        let mut buf = vec![0u8; st.size as usize];
        let n = b.pread(path, &mut buf, 0).unwrap();
        assert_eq!(n, buf.len());
        buf
    }

    // ---------------------------------------------------------------
    // Detection (spec 11 §3: strong magic first, tar heuristic LAST)
    // ---------------------------------------------------------------

    #[test]
    fn detection_chain() {
        let tar = make_tar(&[("a.txt", b"hello")]);
        assert_eq!(detect_format(&tar), ImageFormat::Tar);
        // The heuristic needs a full block.
        assert_eq!(detect_format(&tar[..100]), ImageFormat::Unknown);
        // Strong magics are claimed before the tar heuristic.
        assert_eq!(detect_format(&gzip_bytes(&tar)), ImageFormat::TarGz);
        assert_eq!(detect_format(&fixture_zst()), ImageFormat::TarZst);
        let mut ziphdr = [0u8; 512];
        ziphdr[..4].copy_from_slice(b"PK\x03\x04");
        assert_eq!(detect_format(&ziphdr), ImageFormat::Zip);
        let mut dwhdr = [0u8; 512];
        dwhdr[..6].copy_from_slice(b"DWARFS");
        assert_eq!(detect_format(&dwhdr), ImageFormat::Dwarfs);
        // Garbage, zeros, bad gzip method, corrupted tar checksum.
        assert_eq!(detect_format(&[0u8; 512]), ImageFormat::Unknown);
        assert_eq!(detect_format(&[0xAB; 512]), ImageFormat::Unknown);
        let mut bad_gz = [0u8; 512];
        bad_gz[..3].copy_from_slice(&[0x1f, 0x8b, 7]);
        assert_eq!(detect_format(&bad_gz), ImageFormat::Unknown);
        let mut bad = tar.clone();
        bad[0] ^= 0xFF; // name byte: checksum no longer matches
        assert_eq!(detect_format(&bad), ImageFormat::Unknown);
    }

    // ---------------------------------------------------------------
    // Plain tar semantics
    // ---------------------------------------------------------------

    fn tree_tar() -> Vec<u8> {
        fn file(b: &mut tar::Builder<Vec<u8>>, path: &str, data: &[u8]) {
            let mut h = tar::Header::new_gnu();
            h.set_entry_type(tar::EntryType::Regular);
            h.set_mode(0o640);
            h.set_mtime(1_700_000_000);
            h.set_size(data.len() as u64);
            b.append_data(&mut h, path, data).unwrap();
        }
        let mut b = tar::Builder::new(Vec::new());
        file(&mut b, "hello.txt", b"v1");
        append_dir(&mut b, "dir");
        file(&mut b, "dir/a.txt", &vec![b'A'; 1000]);
        append_dir(&mut b, "dir/sub");
        file(&mut b, "dir/sub/deep.txt", b"deep");
        file(&mut b, "implicit/x/y/z.bin", &[b'Z'; 100]);
        // symlink + hard link
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Symlink);
        h.set_size(0);
        h.set_link_name("hello.txt").unwrap();
        b.append_data(&mut h, "link.txt", io::empty()).unwrap();
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Link);
        h.set_size(0);
        b.append_link(&mut h, "hard.txt", "dir/a.txt").unwrap();
        // "./" normalization and a GNU long name
        file(&mut b, "./dot.txt", b"dot");
        let long = format!("dir/{}", "n".repeat(120));
        file(&mut b, &long, b"long-name-content");
        // duplicate entry: later wins
        file(&mut b, "hello.txt", b"v2-longer");
        b.finish().unwrap();
        b.into_inner().unwrap()
    }

    fn check_tree(b: &TarBackend) {
        // root and synthesized directories
        assert_eq!(b.stat("").unwrap().entry_type, EntryType::Directory);
        assert_eq!(b.stat("implicit").unwrap().entry_type, EntryType::Directory);
        assert_eq!(b.stat("implicit/x/y").unwrap().perms, 0o755);

        // files: explicit metadata, last-entry-wins for duplicates
        let st = b.stat("hello.txt").unwrap();
        assert_eq!(
            (st.entry_type, st.size, st.perms, st.mtime),
            (EntryType::File, 9, 0o640, 1_700_000_000)
        );
        assert_eq!(pread_all(b, "hello.txt"), b"v2-longer");
        assert_eq!(pread_all(b, "dir/a.txt"), vec![b'A'; 1000]);
        assert_eq!(pread_all(b, "implicit/x/y/z.bin"), vec![b'Z'; 100]);
        assert_eq!(pread_all(b, "dot.txt"), b"dot");
        let long = format!("dir/{}", "n".repeat(120));
        assert_eq!(pread_all(b, &long), b"long-name-content");

        // hard link resolves to the target's data
        assert_eq!(b.stat("hard.txt").unwrap().entry_type, EntryType::File);
        assert_eq!(pread_all(b, "hard.txt"), vec![b'A'; 1000]);
        assert_eq!(b.stat("link.txt").unwrap().entry_type, EntryType::Symlink);

        // pread windowing
        let mut buf = [0u8; 4];
        assert_eq!(b.pread("hello.txt", &mut buf, 3).unwrap(), 4);
        assert_eq!(&buf, b"long");
        assert_eq!(b.pread("hello.txt", &mut buf, 9).unwrap(), 0);
        assert_eq!(b.pread("hello.txt", &mut buf, 100).unwrap(), 0);

        // directory listings (direct children only, flags correct)
        let mut root: Vec<(String, bool)> = b
            .read_dir("")
            .unwrap()
            .into_iter()
            .map(|e| (e.name, e.is_dir))
            .collect();
        root.sort();
        assert_eq!(
            root,
            vec![
                ("dir".to_string(), true),
                ("dot.txt".to_string(), false),
                ("hard.txt".to_string(), false),
                ("hello.txt".to_string(), false),
                ("implicit".to_string(), true),
                ("link.txt".to_string(), false),
            ]
        );
        let mut dir: Vec<String> = b
            .read_dir("dir")
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        dir.sort();
        assert_eq!(
            dir,
            vec![
                "a.txt".to_string(),
                long["dir/".len()..].to_string(),
                "sub".to_string()
            ]
        );

        // errno contract
        assert_eq!(b.read_dir("hello.txt").unwrap_err(), libc::ENOTDIR);
        assert_eq!(b.read_dir("nope").unwrap_err(), libc::ENOENT);
        assert_eq!(b.pread("dir", &mut buf, 0).unwrap_err(), libc::EISDIR);
        assert_eq!(b.pread("nope", &mut buf, 0).unwrap_err(), libc::ENOENT);
        assert_eq!(b.pread("link.txt", &mut buf, 0).unwrap_err(), libc::EINVAL);
        assert_eq!(b.stat("nope").unwrap_err(), libc::ENOENT);

        // trailing-slash and leading-slash paths normalize
        assert_eq!(b.stat("/dir/").unwrap().entry_type, EntryType::Directory);
    }

    #[test]
    fn plain_tar_tree_from_memory() {
        let b = TarBackend::from_memory(tree_tar(), TarCompression::None).unwrap();
        assert_eq!(b.name().to_str().unwrap(), "TAR");
        check_tree(&b);
        let info = b.image_info_json().unwrap();
        assert!(info.contains("\"format\":\"tar\"") && info.contains("\"compression\":\"none\""));
    }

    #[test]
    fn plain_tar_tree_from_file_and_region() {
        let dir = tempfile::tempdir().unwrap();
        let tar = tree_tar();
        // Region mount: junk prefix + tar + junk suffix.
        let mut blob = vec![0xEE; 1024];
        blob.extend(&tar);
        blob.extend([0xEE; 512]);
        let path = dir.path().join("blob.bin");
        std::fs::write(&path, &blob).unwrap();
        let path = path.to_str().unwrap();

        // The junk prefix defeats whole-file detection (EINVAL)...
        assert_eq!(
            crate::mount::build_from_file(path, "/w").err(),
            Some(libc::EINVAL)
        );
        // ...while the region mount reads the tar in place.
        let region = crate::mount::build_from_file_at(path, 1024, tar.len() as u64, "/r").unwrap();
        // Exercise through the trait object (the mount dispatch path).
        let b: &dyn Backend = region.backend.as_ref();
        assert_eq!(b.stat("hello.txt").unwrap().size, 9);
        let mut buf = vec![0u8; 9];
        assert_eq!(b.pread("hello.txt", &mut buf, 0).unwrap(), 9);
        assert_eq!(buf, b"v2-longer");
        assert_eq!(b.read_dir("implicit/x").unwrap()[0].name, "y");
    }

    #[test]
    fn plain_tar_whole_file_detection_through_mount() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.tar");
        std::fs::write(&path, tree_tar()).unwrap();
        let mount = crate::mount::build_from_file(path.to_str().unwrap(), "/t").unwrap();
        assert_eq!(mount.backend.name().to_str().unwrap(), "TAR");
        assert_eq!(mount.backend.stat("hello.txt").unwrap().size, 9);
    }

    #[test]
    fn plain_tar_offset_zero_region_mount() {
        // A region spanning the whole file (offset 0): the sniff pass
        // leaves the shared handle past the first block, and the index
        // must still start at the region base.
        let dir = tempfile::tempdir().unwrap();
        let tar = tree_tar();
        let path = dir.path().join("tree.tar");
        std::fs::write(&path, &tar).unwrap();
        let mount =
            crate::mount::build_from_file_at(path.to_str().unwrap(), 0, tar.len() as u64, "/r")
                .unwrap();
        assert_eq!(mount.backend.name().to_str().unwrap(), "TAR");
        assert_eq!(mount.backend.stat("hello.txt").unwrap().size, 9);
        let mut buf = vec![0u8; 9];
        assert_eq!(mount.backend.pread("hello.txt", &mut buf, 0).unwrap(), 9);
    }

    #[test]
    fn corrupt_images_fail_clean() {
        // gzip envelope around non-tar content
        let gz = gzip_bytes(b"this is not a tar stream at all, just bytes");
        assert_eq!(
            TarBackend::from_memory(gz, TarCompression::Gzip).err(),
            Some(libc::EINVAL)
        );
        // zstd envelope around non-tar content (truncated frame)
        assert_eq!(
            TarBackend::from_memory(fixture_zst()[..20].to_vec(), TarCompression::Zstd).err(),
            Some(libc::EINVAL)
        );
        // truncated tar: header claims more data than present
        let tar = make_tar(&[("f.bin", &vec![b'q'; 10_000])]);
        let cut = tar[..tar.len() / 2].to_vec();
        assert_eq!(
            TarBackend::from_memory(cut, TarCompression::None).err(),
            Some(libc::EINVAL)
        );
        // garbage deflate after a valid gzip header
        let mut bad = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
        bad.extend([0x55; 64]);
        assert_eq!(
            TarBackend::from_memory(bad, TarCompression::Gzip).err(),
            Some(libc::EINVAL)
        );
    }

    // ---------------------------------------------------------------
    // tar.gz: checkpointed random access
    // ---------------------------------------------------------------

    /// Pseudo-random (incompressible) content so checkpoints span the stream.
    fn noisy_files(count: u64, size: usize) -> Vec<(String, Vec<u8>)> {
        (0..count)
            .map(|f| {
                let mut data = Vec::with_capacity(size + 8);
                let mut s = f.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
                while data.len() < size {
                    s = splitmix(s);
                    data.extend_from_slice(&s.to_le_bytes());
                }
                data.truncate(size);
                (format!("d{:02}/f{:02}.bin", f / 4, f), data)
            })
            .collect()
    }

    #[test]
    fn gz_random_access_with_tiny_checkpoints() {
        let files = noisy_files(8, 20_000);
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(p, d)| (p.as_str(), d.as_slice()))
            .collect();
        let tar = make_tar(&refs);
        let gz = gzip_bytes(&tar);
        let b = TarBackend::from_source_cfg(
            Source::Memory(Arc::new(gz)),
            TarCompression::Gzip,
            2048, // 2 KiB spacing
            512,  // small input chunks → many capture boundaries
        )
        .unwrap();
        let TarStream::Gz(g) = &b.stream else {
            panic!("expected the gzip stream variant")
        };
        let ncps = g.checkpoints.len();
        assert!(ncps > 20, "expected many checkpoints, got {ncps}");
        eprintln!(
            "[tar-gz] {} checkpoints × ~{} B state for {} B uncompressed",
            ncps,
            std::mem::size_of::<InflateState>(),
            tar.len()
        );

        // Every file: full read + scattered windows in BACKWARD file order
        // (forces checkpoint resumes in both directions).
        for (path, want) in files.iter().rev() {
            assert_eq!(b.stat(path).unwrap().size as usize, want.len());
            assert_eq!(pread_all(&b, path), *want, "full read of {path}");
            for (off, len) in [(0usize, 100usize), (9_999, 500), (19_900, 100), (1, 8)] {
                let mut buf = vec![0u8; len];
                let n = b.pread(path, &mut buf, off as u64).unwrap();
                assert_eq!(n, len);
                assert_eq!(buf, want[off..off + len], "window {off}+{len} of {path}");
            }
        }
        // A second forward pass must hit the shared-cursor fast path.
        for (path, want) in &files {
            assert_eq!(pread_all(&b, path), *want);
        }
    }

    #[test]
    fn gz_default_spacing_multi_checkpoint() {
        // 48 MiB uncompressed → 3 checkpoints at the production 16 MiB spacing.
        let files = noisy_files(6, 8 * 1024 * 1024);
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(p, d)| (p.as_str(), d.as_slice()))
            .collect();
        let tar = make_tar(&refs);
        let gz = gzip_bytes(&tar);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.tar.gz");
        std::fs::write(&path, &gz).unwrap();
        let mount = crate::mount::build_from_file(path.to_str().unwrap(), "/g").unwrap();
        assert_eq!(mount.backend.name().to_str().unwrap(), "TAR.GZ");
        let b = mount.backend;
        for (path, want) in files.iter().rev() {
            let st = b.stat(path).unwrap();
            assert_eq!(st.size as usize, want.len());
            // windows near the spacing boundaries hit resume points
            for off in [0u64, 15 * 1024 * 1024, 16 * 1024 * 1024, 33 * 1024 * 1024] {
                if off as usize + 4096 <= want.len() {
                    let mut buf = vec![0u8; 4096];
                    let n = b.pread(path, &mut buf, off).unwrap();
                    assert_eq!(n, 4096);
                    assert_eq!(buf, want[off as usize..off as usize + 4096]);
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // tar.zst: forward-cursor random access
    // ---------------------------------------------------------------

    #[test]
    fn zst_fixture_mounts_and_reads() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/small.tar.zst");
        let mount = crate::mount::build_from_file(path, "/z").unwrap();
        let b = mount.backend;
        assert_eq!(b.name().to_str().unwrap(), "TAR.ZST");
        // Read the LATER entry first, then an EARLIER one: forces the
        // backward-seek restart path.
        let mut buf = [0u8; 25];
        assert_eq!(b.pread("dir/nested.txt", &mut buf, 0).unwrap(), 25);
        assert_eq!(&buf, b"nested-content-0123456789");
        let mut hello = [0u8; 14];
        assert_eq!(b.pread("hello.txt", &mut hello, 0).unwrap(), 14);
        assert_eq!(&hello, b"hello tar.zst\n");
        let mut x = [0u8; 100];
        assert_eq!(b.pread("hundred_x.bin", &mut x, 0).unwrap(), 100);
        assert!(x.iter().all(|&c| c == b'x'));
        let mut root: Vec<String> = b
            .read_dir("")
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        root.sort();
        assert_eq!(root, vec!["dir", "hello.txt", "hundred_x.bin"]);
    }

    // ---------------------------------------------------------------
    // The 1 GiB random-access RSS budget
    // ---------------------------------------------------------------

    #[cfg(unix)]
    fn peak_rss_bytes() -> u64 {
        let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) }, 0);
        #[cfg(target_os = "linux")]
        {
            ru.ru_maxrss as u64 * 1024 // Linux reports KiB
        }
        #[cfg(not(target_os = "linux"))]
        {
            ru.ru_maxrss as u64 // macOS reports bytes
        }
    }

    /// Streams `len` bytes of the 8-byte-cycle pattern derived from `word`
    /// (keeps the fixture writer at O(1) memory).
    struct WordRead {
        word: u64,
        pos: u64,
        len: u64,
    }

    impl WordRead {
        fn byte(word: u64, off: u64) -> u8 {
            word.to_le_bytes()[(off % 8) as usize]
        }
    }

    impl Read for WordRead {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = ((self.len - self.pos) as usize).min(buf.len());
            for b in &mut buf[..n] {
                *b = Self::byte(self.word, self.pos);
                self.pos += 1;
            }
            Ok(n)
        }
    }

    /// Random-access read of a 1 GiB archive must stay under 64 MiB of
    /// additional peak RSS (spec target; measured via getrusage's peak-RSS
    /// delta — the strongest signal a unit test can observe). Set
    /// TFS_TAR_BIG_GIB=0 to skip, or another value to resize.
    #[test]
    #[cfg(unix)]
    fn gib_archive_random_access_within_rss_budget() {
        let gib: u64 = std::env::var("TFS_TAR_BIG_GIB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        if gib == 0 {
            eprintln!("[tar-big] skipped via TFS_TAR_BIG_GIB=0");
            return;
        }
        const FILES: u64 = 256;
        let file_size = gib * (1 << 30) / FILES;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.tar");
        {
            let out = File::create(&path).unwrap();
            let mut b = tar::Builder::new(out);
            for i in 0..FILES {
                let mut h = tar::Header::new_gnu();
                h.set_entry_type(tar::EntryType::Regular);
                h.set_mode(0o644);
                h.set_size(file_size);
                b.append_data(
                    &mut h,
                    format!("dir{:03}/file{:03}.bin", i / 16, i),
                    WordRead {
                        word: splitmix(i),
                        pos: 0,
                        len: file_size,
                    },
                )
                .unwrap();
            }
            b.finish().unwrap();
        }
        assert!(std::fs::metadata(&path).unwrap().len() > gib * (1 << 30));

        let rss0 = peak_rss_bytes();
        let mount = crate::mount::build_from_file(path.to_str().unwrap(), "/big").unwrap();
        let b = mount.backend;
        assert_eq!(b.name().to_str().unwrap(), "TAR");
        // Stats across the tree (implicit dirs included).
        for i in [0, 1, 128, 255] {
            let st = b
                .stat(&format!("dir{:03}/file{:03}.bin", i / 16, i))
                .unwrap();
            assert_eq!(
                (st.entry_type, st.size),
                (EntryType::File, file_size as i64)
            );
        }
        assert_eq!(b.read_dir("").unwrap().len(), 16);
        // 512 scattered preads, verified byte-for-byte against the pattern.
        let mut s = 0x243F_6A88_85A3_08D3u64;
        let mut rng = move || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            s >> 11
        };
        for _ in 0..512 {
            let f = rng() % FILES;
            let off = (rng() % (file_size - 40_000)) & !7;
            let len = (8 + rng() % 32_768) as usize & !7;
            let mut buf = vec![0u8; len];
            let n = b
                .pread(&format!("dir{:03}/file{:03}.bin", f / 16, f), &mut buf, off)
                .unwrap();
            assert_eq!(n, len);
            let word = splitmix(f);
            for (i, &got) in buf.iter().enumerate() {
                assert_eq!(got, WordRead::byte(word, off + i as u64), "f{f} off {off}");
            }
        }
        let rss1 = peak_rss_bytes();
        let delta = rss1.saturating_sub(rss0);
        eprintln!(
            "[tar-big] {gib} GiB, {FILES} files: peak RSS {} -> {} bytes (delta {:.1} MiB, budget 64 MiB)",
            rss0,
            rss1,
            delta as f64 / 1048576.0
        );
        assert!(
            delta < 64 * 1024 * 1024,
            "RSS delta {delta} exceeds the 64 MiB budget"
        );
    }
}
