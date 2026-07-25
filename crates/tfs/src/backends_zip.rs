//! ZIP backend via the pure-Rust `zip` crate (see Cargo.toml for why not
//! libzip-sys). Read-only, whole-archive name index built at mount time.
//!
//! Semantics mirrored from the C++ `ZipBackend` (verified against the
//! tebakofs oracle):
//! - files report mode `S_IFREG|0644`, directories `S_IFDIR|0755`
//!   (ZIP does not reliably store POSIX permissions)
//! - directories are EXPLICIT-ENTRY-ONLY through the C ABI: a non-root
//!   directory stats/opens only via its `name/` entry. Implicit parents of
//!   deeper entries are never synthesized — a zip without directory
//!   entries lists nothing at the root and ENOENTs implicit paths
//!   (exactly the C++ behavior; real fixtures carry explicit entries).
//! - mtime comes from the entry's DOS timestamp (interpreted as UTC — see
//!   `dos_to_unix`; contract tests do not compare mtimes)

use std::fs::File;
use std::io::{self, Cursor, Read, Seek};
use std::sync::Mutex;

use zip::ZipArchive;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat};

/// Reader source for the archive: a file on disk or an in-memory image.
enum ZipSource {
    File(File),
    Memory(Cursor<Vec<u8>>),
}

impl Read for ZipSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            ZipSource::File(f) => f.read(buf),
            ZipSource::Memory(c) => c.read(buf),
        }
    }
}

impl Seek for ZipSource {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match self {
            ZipSource::File(f) => f.seek(pos),
            ZipSource::Memory(c) => c.seek(pos),
        }
    }
}

/// Mounted ZIP archive.
pub struct ZipBackend {
    archive: Mutex<ZipArchive<ZipSource>>,
    /// Entry names, in archive order (index built once at mount).
    names: Vec<String>,
}

impl ZipBackend {
    /// Open a ZIP archive from a file on disk.
    pub fn from_file(file: File) -> Result<ZipBackend, i32> {
        Self::open(ZipSource::File(file))
    }

    /// Open a ZIP archive from an in-memory image (owned).
    pub fn from_memory(data: Vec<u8>) -> Result<ZipBackend, i32> {
        Self::open(ZipSource::Memory(Cursor::new(data)))
    }

    fn open(source: ZipSource) -> Result<ZipBackend, i32> {
        let mut archive = ZipArchive::new(source).map_err(|_| libc::EINVAL)?;
        let mut names = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|_| libc::EINVAL)?;
            names.push(entry.name().to_string());
        }
        Ok(ZipBackend {
            archive: Mutex::new(archive),
            names,
        })
    }

    /// The raw entry named `path`, if present.
    fn find_file(&self, path: &str) -> Option<usize> {
        self.names
            .iter()
            .position(|n| n.trim_end_matches('/') == path && !n.ends_with('/'))
    }

    /// The explicit directory entry `path/` (C++ zip semantics: only
    /// explicit entries make a non-root directory addressable through the
    /// C ABI — implicit parents of deeper entries are NOT synthesized).
    fn find_explicit_dir(&self, path: &str) -> Option<usize> {
        let want = format!("{path}/");
        self.names.iter().position(|n| *n == want)
    }

    /// stat helper for a found file entry.
    fn stat_file(&self, index: usize) -> Result<RawStat, i32> {
        let mut archive = self.archive.lock().map_err(|_| libc::EIO)?;
        let entry = archive.by_index(index).map_err(|_| libc::EIO)?;
        Ok(RawStat {
            entry_type: EntryType::File,
            perms: 0o644,
            size: entry.size() as i64,
            mtime: entry.last_modified().map(dos_to_unix).unwrap_or(0),
        })
    }

    /// stat helper for an explicit directory entry (the dir entry carries
    /// its own DOS timestamp, like the C++ zip iterator reports).
    fn stat_explicit_dir(&self, index: usize) -> Result<RawStat, i32> {
        let mut archive = self.archive.lock().map_err(|_| libc::EIO)?;
        let entry = archive.by_index(index).map_err(|_| libc::EIO)?;
        Ok(RawStat {
            entry_type: EntryType::Directory,
            perms: 0o755,
            size: 0,
            mtime: entry.last_modified().map(dos_to_unix).unwrap_or(0),
        })
    }

    fn stat_root() -> RawStat {
        RawStat {
            entry_type: EntryType::Directory,
            perms: 0o755,
            size: 0,
            mtime: 0,
        }
    }
}

/// Normalize an in-image path: no leading or trailing `/`, `""` for root.
fn normalize(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

impl Backend for ZipBackend {
    fn name(&self) -> &'static std::ffi::CStr {
        c"ZIP"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let path = normalize(path);
        if path.is_empty() {
            return Ok(Self::stat_root());
        }
        if let Some(index) = self.find_file(path) {
            return self.stat_file(index);
        }
        // C++ semantics: a directory stats only via its explicit "path/"
        // entry (exists() is explicit-only through the C ABI).
        if let Some(index) = self.find_explicit_dir(path) {
            return self.stat_explicit_dir(index);
        }
        Err(libc::ENOENT)
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        let Some(index) = self.find_file(path) else {
            return Err(if self.find_explicit_dir(path).is_some() {
                libc::EISDIR
            } else {
                libc::ENOENT
            });
        };
        let mut archive = self.archive.lock().map_err(|_| libc::EIO)?;
        let mut entry = archive.by_index(index).map_err(|_| libc::EIO)?;

        let size = entry.size();
        if offset >= size || buf.is_empty() {
            return Ok(0);
        }
        // v1: read through the entry sequentially up to the wanted window
        // (zip entries are deflate streams; random access needs decompressing
        // the prefix anyway).
        let want = std::cmp::min(buf.len() as u64, size - offset) as usize;
        let mut skip = vec![0u8; offset as usize];
        entry.read_exact(&mut skip).map_err(|_| libc::EIO)?;
        entry.read_exact(&mut buf[..want]).map_err(|_| libc::EIO)?;
        Ok(want)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        let path = normalize(path);
        if !path.is_empty() && self.find_explicit_dir(path).is_none() {
            // C++ list_directory: an explicit "path/" entry is required to
            // open a non-root directory; an explicit file is ENOTDIR.
            return Err(if self.find_file(path).is_some() {
                libc::ENOTDIR
            } else {
                libc::ENOENT
            });
        }
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };
        let mut out: Vec<RawDirEntry> = Vec::new();
        for name in &self.names {
            let Some(rest) = name.strip_prefix(&prefix) else {
                continue;
            };
            if rest.is_empty() {
                continue; // the directory's own entry
            }
            // C++ zip iterator semantics: yield only EXPLICIT entries at
            // this level — files (no '/'), and directories only via their
            // trailing-slash entry ("name/"). Deeper content is skipped;
            // implicit parents are never synthesized.
            match rest.find('/') {
                None => {
                    if !out.iter().any(|e| e.name == rest) {
                        out.push(RawDirEntry {
                            name: rest.to_string(),
                            is_dir: false,
                        });
                    }
                }
                Some(i) if i == rest.len() - 1 => {
                    let child = &rest[..i];
                    if !child.is_empty() && !out.iter().any(|e| e.name == child) {
                        out.push(RawDirEntry {
                            name: child.to_string(),
                            is_dir: true,
                        });
                    }
                }
                _ => continue,
            }
        }
        Ok(out)
    }
}

/// Convert a ZIP DOS timestamp to unix seconds in the LOCAL timezone
/// (matching libzip, and therefore the C++ oracle: ZIP stores local time
/// with no zone). Uses libc mktime so DST rules match the C library.
fn dos_to_unix(dt: zip::DateTime) -> i64 {
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    tm.tm_year = i32::from(dt.year()) - 1900;
    tm.tm_mon = i32::from(dt.month()) - 1;
    tm.tm_mday = i32::from(dt.day());
    tm.tm_hour = i32::from(dt.hour());
    tm.tm_min = i32::from(dt.minute());
    tm.tm_sec = i32::from(dt.second());
    tm.tm_isdst = -1; // let mktime figure out DST
    unsafe { libc::mktime(&mut tm) as i64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_time_is_local() {
        // The conversion must agree with libc mktime for the same civil
        // time (local timezone semantics, matching libzip).
        let dt = zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap();
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        tm.tm_year = 80;
        tm.tm_mday = 1;
        tm.tm_isdst = -1;
        let expected = unsafe { libc::mktime(&mut tm) as i64 };
        assert_eq!(dos_to_unix(dt), expected);

        let dt = zip::DateTime::from_date_and_time(2000, 2, 29, 12, 0, 0).unwrap();
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        tm.tm_year = 100;
        tm.tm_mon = 1;
        tm.tm_mday = 29;
        tm.tm_hour = 12;
        tm.tm_isdst = -1;
        let expected = unsafe { libc::mktime(&mut tm) as i64 };
        assert_eq!(dos_to_unix(dt), expected);
    }
}
