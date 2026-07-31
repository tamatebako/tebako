//! The backend seam: one trait per archive format, plus image-magic
//! detection. Mirrors the C++ `FileSystem`/`Backend` interface but with
//! errno-valued errors (the C ABI boundary speaks errno).
//!
//! Backends are stateless lookup engines: the fd/dir tables live in the
//! context, and every backend operation is path-addressed. All `unsafe`
//! stays in the backend modules (a pure-Rust backend like ZIP has none).

/// Entry type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
    /// Anything else (device, fifo, socket, unknown).
    Other,
}

/// Stat-equivalent info returned by [`Backend::stat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawStat {
    /// Entry type.
    pub entry_type: EntryType,
    /// Permission bits (e.g. 0o644).
    pub perms: u32,
    /// File size in bytes (regular files).
    pub size: i64,
    /// Modification time, seconds since the epoch.
    pub mtime: i64,
}

/// One directory entry returned by [`Backend::read_dir`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDirEntry {
    /// Entry name (never `.` or `..`).
    pub name: String,
    /// True for a directory.
    pub is_dir: bool,
}

/// A mounted archive image. Errors are raw errno values ready for the
/// thread-local errno channel.
///
/// `path` arguments are relative to the mount root, use `/` separators and
/// have no leading slash (`""` denotes the root directory).
pub trait Backend: Send + Sync {
    /// Human-readable backend name as a NUL-terminated static string
    /// (e.g. `c"ZIP"`), safe to hand straight to the C ABI.
    fn name(&self) -> &'static std::ffi::CStr;

    /// Look up an entry. `Err(ENOENT)` when missing.
    fn stat(&self, path: &str) -> Result<RawStat, i32>;

    /// The write gate's territory check: the image holds an entry at
    /// `path` OR any content under it. Implied parents count — a write
    /// into an implied directory still writes into the image's tree
    /// (zip: entries exist under `path/` without an explicit dir entry;
    /// dwarfs/squashfs/tar: the explicit entries already make stat answer
    /// this). Default: the stat answer; zip overrides with a prefix scan.
    fn has_entry_or_children(&self, path: &str) -> bool {
        self.stat(path).is_ok()
    }

    /// Read from a regular file at `offset` (pread primitive).
    /// Callers clamp to EOF; backends may read less.
    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32>;

    /// Read a symlink's target. Default: `Err(ENOTSUP)` — the backend
    /// cannot resolve targets (or `path` is not a symlink). Additive and
    /// NOT part of the C ABI: consumers are the in-process transforms
    /// (the tree-hash walker of spec 03 §7, spec 10's EncBackend).
    fn read_link(&self, path: &str) -> Result<String, i32> {
        let _ = path;
        Err(libc::ENOTSUP)
    }

    /// List a directory's direct children (no `.`/`..`).
    /// `Err(ENOTDIR)` when `path` is not a directory.
    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32>;

    /// Image-level metadata as JSON (item 24's `image_info_json`), when the
    /// backend exposes it. Default: None (backend has no image metadata
    /// surface).
    fn image_info_json(&self) -> Option<String> {
        None
    }

    /// The writable view of this backend, when it is one of the composite
    /// write-capable backends (COW overlay, host directory). Default: None
    /// — every FORMAT backend is read-only forever (spec 00 invariant 5:
    /// backends never learn to write; write support exists ONLY in the
    /// Rust-TFS composite layer, spec 11 §4).
    fn writable(&self) -> Option<&dyn WritableBackend> {
        None
    }
}

/// The write seam (spec 11 §4): positioned writes, directory creation and
/// removal. Only composite/overlay backends implement this; format
/// backends stay read-only. Errors are raw errno values.
///
/// `path` arguments follow the [`Backend`] convention (relative to the
/// mount root, `/`-separated, `""` is the root).
pub trait WritableBackend: Backend {
    /// Write `data` at `offset` in `path`, creating the file (and its
    /// parent directories, overlay-style) when missing. Never truncates;
    /// returns the number of bytes written.
    fn pwrite(&self, path: &str, data: &[u8], offset: u64) -> Result<usize, i32>;

    /// Truncate `path` to `len` bytes. `Err(ENOENT)` when the file does
    /// not exist.
    fn truncate(&self, path: &str, len: u64) -> Result<(), i32>;

    /// Create a single directory (no implicit parents). `Err(EEXIST)`
    /// when it exists, `Err(ENOENT)` when the parent does not.
    fn mkdir(&self, path: &str, perms: u32) -> Result<(), i32>;

    /// Remove a file, symlink or EMPTY directory (`Err(ENOTEMPTY)`
    /// otherwise). `Err(ENOENT)` when missing.
    fn remove(&self, path: &str) -> Result<(), i32>;
}

/// Detected archive format (magic sniffing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// ZIP archive ("PK\x03\x04", or "PK\x05\x06" for an empty archive).
    Zip,
    /// DwarFS image ("DWARFS").
    Dwarfs,
    /// SquashFS image ("hsqs").
    Squashfs,
    /// tar stream (no strong magic — header-checksum heuristic, LAST).
    Tar,
    /// gzip-wrapped tar ("\x1f\x8b" + deflate method byte).
    TarGz,
    /// zstd-frame-wrapped tar (frame magic 0xFD2FB528, little-endian).
    TarZst,
    /// Unrecognized.
    Unknown,
}

/// Sniff the archive format from its first bytes (spec 11 §3: strong magic
/// first, the weak tar heuristic LAST). `magic` should carry at least one
/// 512-byte tar block for the tar heuristic to fire.
pub fn detect_format(magic: &[u8]) -> ImageFormat {
    if magic.starts_with(b"PK\x03\x04") || magic.starts_with(b"PK\x05\x06") {
        ImageFormat::Zip
    } else if magic.starts_with(b"DWARFS") {
        ImageFormat::Dwarfs
    } else if magic.starts_with(b"hsqs") {
        ImageFormat::Squashfs
    } else if magic.len() >= 3 && magic[0] == 0x1f && magic[1] == 0x8b && magic[2] == 8 {
        // gzip envelope; the only gzip payload TFS mounts is tar.
        ImageFormat::TarGz
    } else if magic.starts_with(b"\x28\xb5\x2f\xfd") {
        // zstd frame; skippable frames (0x184D2A5*) are not images.
        ImageFormat::TarZst
    } else if looks_like_tar_header(magic) {
        ImageFormat::Tar
    } else {
        ImageFormat::Unknown
    }
}

/// ustar header-checksum heuristic: a valid first block whose stored
/// checksum matches the computed one (checksum field itself counted as
/// spaces, POSIX 1003.1-1988). Weak by design — runs last in the chain.
fn looks_like_tar_header(block: &[u8]) -> bool {
    const BLOCK: usize = 512;
    const SUM_FIELD: std::ops::Range<usize> = 148..156;
    if block.len() < BLOCK {
        return false;
    }
    let h = &block[..BLOCK];
    if h.iter().all(|&b| b == 0) {
        return false;
    }
    let field = &h[SUM_FIELD];
    let mut i = 0;
    while i < field.len() && (field[i] == b' ' || field[i] == 0) {
        i += 1;
    }
    let start = i;
    let mut stored: u64 = 0;
    while i < field.len() && (b'0'..=b'7').contains(&field[i]) {
        stored = stored * 8 + u64::from(field[i] - b'0');
        i += 1;
    }
    if i == start {
        return false; // no octal digits in the checksum field
    }
    let mut computed: u64 = 0;
    for (j, &b) in h.iter().enumerate() {
        computed += if SUM_FIELD.contains(&j) {
            u64::from(b' ')
        } else {
            u64::from(b)
        };
    }
    computed == stored
}
