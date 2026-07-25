//! ZIP backend via the pure-Rust `zip` crate (see Cargo.toml for why not
//! libzip-sys). Read-only, whole-archive name index built at mount time.
//!
//! Semantics mirrored from the C++ `ZipBackend`:
//! - files report mode `S_IFREG|0644`, directories `S_IFDIR|0755`
//!   (ZIP does not reliably store POSIX permissions)
//! - a directory exists when it has an explicit `name/` entry or any entry
//!   below `name/`
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

    /// True when `path` names a directory (explicit entry or prefix).
    fn is_dir(&self, path: &str) -> bool {
        if path.is_empty() {
            return true;
        }
        let prefix = format!("{path}/");
        self.names.iter().any(|n| n.starts_with(&prefix))
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
            return Ok(RawStat {
                entry_type: EntryType::Directory,
                perms: 0o755,
                size: 0,
                mtime: 0,
            });
        }
        if let Some(index) = self.find_file(path) {
            return self.stat_file(index);
        }
        if self.is_dir(path) {
            return Ok(RawStat {
                entry_type: EntryType::Directory,
                perms: 0o755,
                size: 0,
                mtime: 0,
            });
        }
        Err(libc::ENOENT)
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        let Some(index) = self.find_file(path) else {
            return Err(if self.is_dir(path) {
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
        if !path.is_empty() && !self.is_dir(path) {
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
            if rest.is_empty() || rest == "/" {
                continue; // the directory's own entry
            }
            // NB: check for '/' before trimming it: "subdir/" is a directory.
            let (child, is_dir) = match rest.find('/') {
                Some(i) => (&rest[..i], true),
                None => (rest, false),
            };
            if child.is_empty() {
                continue;
            }
            if !out.iter().any(|e| e.name == child) {
                out.push(RawDirEntry {
                    name: child.to_string(),
                    is_dir,
                });
            }
        }
        Ok(out)
    }
}

/// Convert a ZIP DOS timestamp to unix seconds (interpreted as UTC).
///
/// ZIP stores local time with no zone; the C++ backend (libzip) interprets
/// it in the process timezone. The difference is at most the UTC offset and
/// does not affect the C ABI contract tests, which never compare mtimes.
fn dos_to_unix(dt: zip::DateTime) -> i64 {
    let days = days_from_civil(dt.year() as i64, dt.month() as u32, dt.day() as u32);
    days * 86_400
        + i64::from(dt.hour()) * 3600
        + i64::from(dt.minute()) * 60
        + i64::from(dt.second())
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = ((m + 9) % 12) as i64;
    let doy = (153 * mp + 2) / 5 + i64::from(d - 1);
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_epoch() {
        // 1980-01-01 00:00:00 UTC = 315532800
        let dt = zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(dos_to_unix(dt), 315_532_800);
        // 2000-02-29 12:00:00 UTC = 951825600 (leap day sanity)
        let dt = zip::DateTime::from_date_and_time(2000, 2, 29, 12, 0, 0).unwrap();
        assert_eq!(dos_to_unix(dt), 951_825_600);
    }
}
