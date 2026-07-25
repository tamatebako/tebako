//! DwarFS backend via the external [`dwarfs-t`] crate (dwarfs-rs).
//!
//! The backend is a thin, safe adapter: every call maps 1:1 onto the
//! `dwarfs_t::Filesystem` API (itself a safe wrapper over the `dwarfs_c_*` C
//! ABI), and every error is its raw errno (`DwarfsError::errno()`), so the
//! C ABI errno contract is preserved end-to-end.
//!
//! Backend name is "DwarFS" — the C++ libtfs reports exactly that and the
//! contract tests assert it.

use std::path::Path;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat};

/// Mounted DwarFS image.
pub struct DwarfsBackend {
    fs: dwarfs_t::Filesystem,
}

fn map_entry_type(t: dwarfs_t::FileType) -> EntryType {
    match t {
        dwarfs_t::FileType::Regular => EntryType::File,
        dwarfs_t::FileType::Directory => EntryType::Directory,
        dwarfs_t::FileType::Symlink => EntryType::Symlink,
        _ => EntryType::Other,
    }
}

impl DwarfsBackend {
    /// Open a DwarFS image from a file on disk.
    pub fn from_file(path: &Path) -> Result<DwarfsBackend, i32> {
        let fs = dwarfs_t::Filesystem::open(path).map_err(|e| e.errno())?;
        Ok(DwarfsBackend { fs })
    }

    /// Open a DwarFS image from an in-memory buffer (copied by dwarfs-rs).
    pub fn from_memory(data: &[u8]) -> Result<DwarfsBackend, i32> {
        let fs = dwarfs_t::Filesystem::open_memory(data).map_err(|e| e.errno())?;
        Ok(DwarfsBackend { fs })
    }

    /// Open a DwarFS image from a region of a file.
    pub fn from_file_at(path: &Path, offset: i64, length: u64) -> Result<DwarfsBackend, i32> {
        let fs = dwarfs_t::Filesystem::open_region(path, offset, length).map_err(|e| e.errno())?;
        Ok(DwarfsBackend { fs })
    }
}

impl Backend for DwarfsBackend {
    fn name(&self) -> &'static std::ffi::CStr {
        c"DwarFS"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let meta = self.fs.stat(path).map_err(|e| e.errno())?;
        Ok(RawStat {
            entry_type: map_entry_type(meta.file_type),
            perms: meta.mode & 0o7777,
            size: meta.size as i64,
            mtime: meta.mtime,
        })
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        self.fs
            .pread(path, buf, offset as i64)
            .map_err(|e| e.errno())
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        let iter = self.fs.read_dir(path).map_err(|e| e.errno())?;
        let mut out = Vec::new();
        for entry in iter {
            let entry = entry.map_err(|e| e.errno())?;
            out.push(RawDirEntry {
                name: entry.name,
                is_dir: entry.file_type == dwarfs_t::FileType::Directory,
            });
        }
        Ok(out)
    }

    fn image_info_json(&self) -> Option<String> {
        self.fs.image_info_json().ok()
    }
}
