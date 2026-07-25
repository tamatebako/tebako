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

    /// Read from a regular file at `offset` (pread primitive).
    /// Callers clamp to EOF; backends may read less.
    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32>;

    /// List a directory's direct children (no `.`/`..`).
    /// `Err(ENOTDIR)` when `path` is not a directory.
    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32>;

    /// Image-level metadata as JSON (item 24's `image_info_json`), when the
    /// backend exposes it. Default: None (backend has no image metadata
    /// surface).
    fn image_info_json(&self) -> Option<String> {
        None
    }
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
    /// Unrecognized.
    Unknown,
}

/// Sniff the archive format from its first bytes.
pub fn detect_format(magic: &[u8]) -> ImageFormat {
    if magic.starts_with(b"PK\x03\x04") || magic.starts_with(b"PK\x05\x06") {
        ImageFormat::Zip
    } else if magic.starts_with(b"DWARFS") {
        ImageFormat::Dwarfs
    } else if magic.starts_with(b"hsqs") {
        ImageFormat::Squashfs
    } else {
        ImageFormat::Unknown
    }
}
