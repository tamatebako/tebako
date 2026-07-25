//! SquashFS backend via libsquashfs (squashfs-tools-ng, crate `sqfs-sys`).
//!
//! Mirrors the C++ `SquashFSBackend` (libtfs `src/backends/
//! squashfs_backend.cpp`): same mount sequence (superblock → decompressor
//! → directory reader → root-inode probe), same read path (data reader with
//! the fragment table loaded), same eager directory reads.
//!
//! libsquashfs keeps mutable state in its readers, so all calls are
//! serialized through one Mutex (the C++ backend locks the same way). The
//! memory mount owns the image Vec; the memory-backed `sqfs_file_t` borrows
//! from it.

use std::ffi::{CStr, CString};
use std::sync::Mutex;

use sqfs_sys::*;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat};

/// Mounted SquashFS image.
pub struct SquashfsBackend {
    /// Owned memory image (borrowed by the sqfs file; None for file mounts).
    _image: Option<Vec<u8>>,
    inner: Mutex<SqfsInner>,
}

struct SqfsInner {
    file: *mut sqfs_file_t,
    /// Kept alive for ownership: the directory reader and data reader
    /// reference the superblock for their whole lifetime.
    #[allow(dead_code)]
    super_: Box<sqfs_super_t>,
    cmp: *mut sqfs_compressor_t,
    rd: *mut sqfs_dir_reader_t,
    data: *mut sqfs_data_reader_t,
}

// The raw pointers are only ever used through the backend's Mutex.
unsafe impl Send for SqfsInner {}

impl Drop for SqfsInner {
    fn drop(&mut self) {
        unsafe {
            if !self.data.is_null() {
                sqfs_destroy(self.data.cast());
            }
            if !self.rd.is_null() {
                sqfs_destroy(self.rd.cast());
            }
            if !self.cmp.is_null() {
                sqfs_destroy(self.cmp.cast());
            }
            if !self.file.is_null() {
                sqfs_destroy(self.file.cast());
            }
        }
    }
}

impl SqfsInner {
    /// The C++ mount_common sequence. `file` is consumed on success
    /// (owned by the returned inner) and destroyed on failure.
    fn mount_common(file: *mut sqfs_file_t) -> Result<SqfsInner, i32> {
        unsafe {
            // Read superblock.
            let mut super_ = Box::new(std::mem::zeroed::<sqfs_super_t>());
            if sqfs_super_read(super_.as_mut(), file) != 0 {
                sqfs_destroy(file.cast());
                return Err(libc::EIO); // CorruptedArchive
            }

            // Create the decompressor for the archive's compression id
            // (SQFS_COMP_FLAG_UNCOMPRESS — reader only).
            let mut cmp: *mut sqfs_compressor_t = std::ptr::null_mut();
            if sqfs_shim_compressor_create(super_.compression_id, super_.block_size, &mut cmp) != 0
                || cmp.is_null()
            {
                sqfs_destroy(file.cast());
                return Err(libc::ENOTSUP);
            }

            // Create the directory reader.
            let rd = sqfs_dir_reader_create(super_.as_ref(), cmp, file, 0);
            if rd.is_null() {
                sqfs_destroy(cmp.cast());
                sqfs_destroy(file.cast());
                return Err(libc::ENOMEM);
            }

            // Prove the archive metadata is readable by resolving the root
            // inode (catches corruption past the superblock).
            let mut root: *mut sqfs_inode_generic_t = std::ptr::null_mut();
            if sqfs_dir_reader_get_root_inode(rd, &mut root) != 0 {
                sqfs_destroy(rd.cast());
                sqfs_destroy(cmp.cast());
                sqfs_destroy(file.cast());
                return Err(libc::EIO);
            }
            sqfs_free(root.cast());

            // One shared data reader with the fragment table loaded (the
            // table is required for fragment-packed small files).
            let data = sqfs_data_reader_create(file, super_.block_size as usize, cmp, 0);
            if data.is_null() {
                sqfs_destroy(rd.cast());
                sqfs_destroy(cmp.cast());
                sqfs_destroy(file.cast());
                return Err(libc::ENOMEM);
            }
            if sqfs_data_reader_load_fragment_table(data, super_.as_ref()) != 0 {
                sqfs_destroy(data.cast());
                sqfs_destroy(rd.cast());
                sqfs_destroy(cmp.cast());
                sqfs_destroy(file.cast());
                return Err(libc::EIO);
            }

            Ok(SqfsInner {
                file,
                super_,
                cmp,
                rd,
                data,
            })
        }
    }

    /// Resolve an inode by in-image path ("" = root). The caller owns the
    /// result (free with sqfs_free).
    fn lookup_inode(&self, path: &str) -> Result<*mut sqfs_inode_generic_t, i32> {
        unsafe {
            let mut inode: *mut sqfs_inode_generic_t = std::ptr::null_mut();
            if path.is_empty() {
                if sqfs_dir_reader_get_root_inode(self.rd, &mut inode) == 0 {
                    return Ok(inode);
                }
            } else {
                let cpath = CString::new(path).map_err(|_| libc::EINVAL)?;
                if sqfs_dir_reader_find_by_path(
                    self.rd,
                    std::ptr::null(),
                    cpath.as_ptr(),
                    &mut inode,
                ) == 0
                {
                    return Ok(inode);
                }
            }
            Err(libc::ENOENT)
        }
    }

    /// stat for a looked-up inode (consumes the inode).
    unsafe fn stat_inode(inode: *mut sqfs_inode_generic_t) -> Result<RawStat, i32> {
        unsafe {
            let base = &*(inode as *const sqfs_inode_t);
            let entry_type = match base.r#type {
                SQFS_INODE_DIR | SQFS_INODE_EXT_DIR => EntryType::Directory,
                SQFS_INODE_FILE | SQFS_INODE_EXT_FILE => EntryType::File,
                SQFS_INODE_SLINK | SQFS_INODE_EXT_SLINK => EntryType::Symlink,
                _ => EntryType::Other,
            };
            let mut size = 0u64;
            if entry_type == EntryType::File && sqfs_inode_get_file_size(inode, &mut size) != 0 {
                sqfs_free(inode.cast());
                return Err(libc::EIO);
            }
            let st = RawStat {
                entry_type,
                perms: u32::from(base.mode),
                size: size as i64,
                mtime: i64::from(base.mod_time),
            };
            sqfs_free(inode.cast());
            Ok(st)
        }
    }
}

/// Normalize an in-image path: no leading or trailing `/`, `""` for root.
fn normalize(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

impl SquashfsBackend {
    /// Open a SquashFS image from a file on disk.
    pub fn from_file(path: &str) -> Result<SquashfsBackend, i32> {
        let cpath = CString::new(path).map_err(|_| libc::EINVAL)?;
        let file = unsafe { sqfs_open_file(cpath.as_ptr(), SQFS_FILE_OPEN_READ_ONLY) };
        if file.is_null() {
            return Err(libc::ENOENT);
        }
        let inner = SqfsInner::mount_common(file)?;
        Ok(SquashfsBackend {
            _image: None,
            inner: Mutex::new(inner),
        })
    }

    /// Open a SquashFS image from memory (the image is OWNED here; the
    /// memory-backed sqfs file borrows from it).
    pub fn from_memory(data: Vec<u8>) -> Result<SquashfsBackend, i32> {
        if data.is_empty() {
            return Err(libc::EINVAL);
        }
        let file = unsafe { sqfs_memory_file_create(data.as_ptr().cast(), data.len()) };
        if file.is_null() {
            return Err(libc::ENOMEM);
        }
        let inner = SqfsInner::mount_common(file)?;
        Ok(SquashfsBackend {
            _image: Some(data),
            inner: Mutex::new(inner),
        })
    }
}

impl Backend for SquashfsBackend {
    fn name(&self) -> &'static CStr {
        c"SquashFS"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let path = normalize(path);
        let inner = self.inner.lock().map_err(|_| libc::EIO)?;
        let inode = inner.lookup_inode(path)?;
        unsafe { SqfsInner::stat_inode(inode) }
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        let inner = self.inner.lock().map_err(|_| libc::EIO)?;
        let inode = inner.lookup_inode(path)?;
        unsafe {
            let base = &*(inode as *const sqfs_inode_t);
            if base.r#type == SQFS_INODE_DIR || base.r#type == SQFS_INODE_EXT_DIR {
                sqfs_free(inode.cast());
                return Err(libc::EISDIR);
            }
            if base.r#type != SQFS_INODE_FILE && base.r#type != SQFS_INODE_EXT_FILE {
                sqfs_free(inode.cast());
                return Err(libc::EINVAL);
            }
            let mut size = 0u64;
            if sqfs_inode_get_file_size(inode, &mut size) != 0 {
                sqfs_free(inode.cast());
                return Err(libc::EIO);
            }
            if offset >= size || buf.is_empty() {
                sqfs_free(inode.cast());
                return Ok(0);
            }
            let want = std::cmp::min(buf.len() as u64, size - offset) as sqfs_u32;
            let n = sqfs_data_reader_read(inner.data, inode, offset, buf.as_mut_ptr().cast(), want);
            sqfs_free(inode.cast());
            if n < 0 {
                return Err(libc::EIO);
            }
            Ok(n as usize)
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        let path = normalize(path);
        let inner = self.inner.lock().map_err(|_| libc::EIO)?;
        let inode = inner.lookup_inode(path)?;
        unsafe {
            let base = &*(inode as *const sqfs_inode_t);
            if base.r#type != SQFS_INODE_DIR && base.r#type != SQFS_INODE_EXT_DIR {
                sqfs_free(inode.cast());
                return Err(libc::ENOTDIR);
            }
            if sqfs_dir_reader_open_dir(inner.rd, inode, 0) != 0 {
                sqfs_free(inode.cast());
                return Err(libc::EIO);
            }
            let mut out = Vec::new();
            loop {
                let mut entry: *mut sqfs_dir_entry_t = std::ptr::null_mut();
                if sqfs_dir_reader_read(inner.rd, &mut entry) != 0 {
                    break;
                }
                let e = &*entry;
                let name_len = usize::from(e.size) + 1; // stored off-by-one
                let name_bytes = std::slice::from_raw_parts(e.name.as_ptr(), name_len);
                let name = String::from_utf8_lossy(name_bytes).into_owned();
                let is_dir = e.r#type == SQFS_INODE_DIR || e.r#type == SQFS_INODE_EXT_DIR;
                out.push(RawDirEntry { name, is_dir });
                sqfs_free(entry.cast());
            }
            sqfs_free(inode.cast());
            Ok(out)
        }
    }
}
