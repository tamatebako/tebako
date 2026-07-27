//! HostDir backend: a host directory exposed as a TFS backend, plus write
//! support — the first overlay store for the COW composite (spec 11 §4).
//!
//! Semantics:
//! - lstat-style metadata (symlinks stat as `Symlink`, never followed);
//!   pread on a symlink is EINVAL, consistent with the tar backend
//! - permissions/mtime come from the host filesystem; host errnos pass
//!   through unchanged (ENOENT/EEXIST/ENOTEMPTY/ENOTDIR — named errors,
//!   no synthesized fallbacks)
//! - writes are positioned (`pwrite`-style); parent directories of a
//!   written file materialize on demand (overlay semantics)
//! - paths with `..` components are rejected (the backend never escapes
//!   its root)
//!
//! Used standalone the backend is a plain read view of a host directory;
//! [`CowBackend`](crate::backends_cow::CowBackend) adds the whiteout
//! journal and the fall-through/shadow logic.

use std::ffi::CStr;
use std::fs::{self, File, OpenOptions};
use std::io;
#[cfg(not(unix))]
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat, WritableBackend};

/// A mounted host directory.
pub struct HostDirBackend {
    root: PathBuf,
}

/// Map a host I/O error to its errno (raw OS error preferred — the honest
/// contract; EIO only when the OS gave us nothing).
pub(crate) fn io_errno(e: &io::Error) -> i32 {
    e.raw_os_error().unwrap_or(libc::EIO)
}

/// Validate an in-image path and join it under `root` (`""` maps to the
/// root itself). EINVAL for `..`/prefix components — the escape hatch.
pub(crate) fn join_under(root: &Path, path: &str) -> Result<PathBuf, i32> {
    let mut out = root.to_path_buf();
    for comp in Path::new(path).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir | Component::RootDir => {}
            _ => return Err(libc::EINVAL),
        }
    }
    Ok(out)
}

/// Permission bits and mtime from host metadata (platform-tolerant).
fn meta_perms(md: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        md.permissions().mode() & 0o7777
    }
    #[cfg(not(unix))]
    {
        if md.is_dir() {
            0o755
        } else {
            0o644
        }
    }
}

fn meta_mtime(md: &fs::Metadata) -> i64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        md.mtime()
    }
    #[cfg(not(unix))]
    {
        md.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

impl HostDirBackend {
    /// Expose `root` (must be an existing directory).
    pub fn new(root: &Path) -> Result<HostDirBackend, i32> {
        let md = fs::metadata(root).map_err(|e| io_errno(&e))?;
        if !md.is_dir() {
            return Err(libc::ENOTDIR);
        }
        let root = fs::canonicalize(root).map_err(|e| io_errno(&e))?;
        Ok(HostDirBackend { root })
    }

    /// The (canonicalized) host root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create all missing parents of an in-image path (overlay
    /// materialization; idempotent).
    pub fn mkdir_parents(&self, path: &str) -> Result<(), i32> {
        let host = join_under(&self.root, path)?;
        fs::create_dir_all(&host).map_err(|e| io_errno(&e))
    }

    /// Best-effort permission set (used by copy-up; unix only).
    #[cfg(unix)]
    pub fn set_perms(&self, path: &str, perms: u32) {
        use std::os::unix::fs::PermissionsExt as _;
        if let Ok(host) = join_under(&self.root, path) {
            let _ = fs::set_permissions(&host, fs::Permissions::from_mode(perms));
        }
    }

    fn host(&self, path: &str) -> Result<PathBuf, i32> {
        join_under(&self.root, path)
    }
}

/// Normalize an in-image path: no leading or trailing `/`, `""` for root.
fn normalize(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

impl Backend for HostDirBackend {
    fn name(&self) -> &'static CStr {
        c"HOSTDIR"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let host = self.host(normalize(path))?;
        let md = fs::symlink_metadata(&host).map_err(|e| io_errno(&e))?;
        let ft = md.file_type();
        let entry_type = if ft.is_file() {
            EntryType::File
        } else if ft.is_dir() {
            EntryType::Directory
        } else if ft.is_symlink() {
            EntryType::Symlink
        } else {
            EntryType::Other
        };
        Ok(RawStat {
            entry_type,
            perms: meta_perms(&md),
            size: if ft.is_file() { md.len() as i64 } else { 0 },
            mtime: meta_mtime(&md),
        })
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let host = self.host(normalize(path))?;
        let md = fs::symlink_metadata(&host).map_err(|e| io_errno(&e))?;
        let ft = md.file_type();
        if ft.is_dir() {
            return Err(libc::EISDIR);
        }
        if !ft.is_file() {
            return Err(libc::EINVAL); // symlinks and special files: no content
        }
        let size = md.len();
        if offset >= size || buf.is_empty() {
            return Ok(0);
        }
        let want = (buf.len() as u64).min(size - offset) as usize;
        let file = File::open(&host).map_err(|e| io_errno(&e))?;
        let buf = &mut buf[..want];
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileExt as _;
            let mut abs = offset;
            let mut rest = buf;
            while !rest.is_empty() {
                let n = file.read_at(rest, abs).map_err(|e| io_errno(&e))?;
                if n == 0 {
                    return Err(libc::EIO);
                }
                abs += n as u64;
                rest = &mut rest[n..];
            }
            Ok(want)
        }
        #[cfg(not(unix))]
        {
            let mut file = file;
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| io_errno(&e))?;
            file.read_exact(buf).map_err(|e| io_errno(&e))?;
            Ok(want)
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        let host = self.host(normalize(path))?;
        let md = fs::symlink_metadata(&host).map_err(|e| io_errno(&e))?;
        if !md.is_dir() {
            return Err(libc::ENOTDIR);
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(&host).map_err(|e| io_errno(&e))? {
            let entry = entry.map_err(|e| io_errno(&e))?;
            // Non-UTF-8 host names are not exposable through the ABI.
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            out.push(RawDirEntry { name, is_dir });
        }
        Ok(out)
    }

    fn read_link(&self, path: &str) -> Result<String, i32> {
        let host = self.host(normalize(path))?;
        let target = fs::read_link(&host).map_err(|e| io_errno(&e))?;
        target.to_str().map(str::to_string).ok_or(libc::EINVAL)
    }

    fn writable(&self) -> Option<&dyn WritableBackend> {
        Some(self)
    }
}

impl WritableBackend for HostDirBackend {
    fn pwrite(&self, path: &str, data: &[u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        if path.is_empty() {
            return Err(libc::EISDIR);
        }
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                let host_parent = join_under(&self.root, parent.to_str().ok_or(libc::EINVAL)?)?;
                fs::create_dir_all(&host_parent).map_err(|e| io_errno(&e))?;
            }
        }
        let host = self.host(path)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&host)
            .map_err(|e| io_errno(&e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileExt as _;
            file.write_all_at(data, offset).map_err(|e| io_errno(&e))?;
        }
        #[cfg(not(unix))]
        {
            use std::io::Write as _;
            let mut file = file;
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| io_errno(&e))?;
            file.write_all(data).map_err(|e| io_errno(&e))?;
        }
        Ok(data.len())
    }

    fn truncate(&self, path: &str, len: u64) -> Result<(), i32> {
        let host = self.host(normalize(path))?;
        // No create: truncate requires an existing file (ENOENT otherwise).
        let file = OpenOptions::new()
            .write(true)
            .open(&host)
            .map_err(|e| io_errno(&e))?;
        file.set_len(len).map_err(|e| io_errno(&e))
    }

    fn mkdir(&self, path: &str, perms: u32) -> Result<(), i32> {
        let path = normalize(path);
        if path.is_empty() {
            return Err(libc::EEXIST);
        }
        let host = self.host(path)?;
        fs::create_dir(&host).map_err(|e| io_errno(&e))?;
        #[cfg(unix)]
        self.set_perms(path, perms);
        #[cfg(not(unix))]
        let _ = perms;
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), i32> {
        let host = self.host(normalize(path))?;
        let md = fs::symlink_metadata(&host).map_err(|e| io_errno(&e))?;
        if md.is_dir() {
            fs::remove_dir(&host).map_err(|e| io_errno(&e))
        } else {
            fs::remove_file(&host).map_err(|e| io_errno(&e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pread_all(b: &HostDirBackend, path: &str) -> Vec<u8> {
        let st = b.stat(path).unwrap();
        let mut buf = vec![0u8; st.size as usize];
        let n = b.pread(path, &mut buf, 0).unwrap();
        assert_eq!(n, buf.len());
        buf
    }

    #[test]
    fn hostdir_read_and_write_ops() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/f.txt"), b"host-content").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("f.txt", dir.path().join("sub/l.txt")).unwrap();

        let b = HostDirBackend::new(dir.path()).unwrap();
        assert_eq!(b.name().to_str().unwrap(), "HOSTDIR");

        // stat / pread / read_dir
        let st = b.stat("sub/f.txt").unwrap();
        assert_eq!((st.entry_type, st.size), (EntryType::File, 12));
        let mut buf = [0u8; 7];
        assert_eq!(b.pread("sub/f.txt", &mut buf, 5).unwrap(), 7);
        assert_eq!(&buf, b"content");
        assert_eq!(b.pread("sub/f.txt", &mut buf, 12).unwrap(), 0);
        assert_eq!(b.pread("sub", &mut buf, 0).unwrap_err(), libc::EISDIR);
        assert_eq!(b.pread("missing", &mut buf, 0).unwrap_err(), libc::ENOENT);
        #[cfg(unix)]
        {
            assert_eq!(b.stat("sub/l.txt").unwrap().entry_type, EntryType::Symlink);
            assert_eq!(b.pread("sub/l.txt", &mut buf, 0).unwrap_err(), libc::EINVAL);
        }
        assert!(b
            .read_dir("")
            .unwrap()
            .iter()
            .any(|e| e.name == "sub" && e.is_dir));
        assert_eq!(b.read_dir("sub/f.txt").unwrap_err(), libc::ENOTDIR);
        assert_eq!(b.read_dir("missing").unwrap_err(), libc::ENOENT);
        assert_eq!(b.stat("").unwrap().entry_type, EntryType::Directory);
        assert_eq!(b.stat("../outside").unwrap_err(), libc::EINVAL);

        // writes land with parents materialized on demand
        let w = b.writable().unwrap();
        assert_eq!(w.pwrite("new/deep/x.bin", b"xyz", 0).unwrap(), 3);
        assert_eq!(pread_all(&b, "new/deep/x.bin"), b"xyz");
        assert_eq!(w.pwrite("new/deep/x.bin", b"Q", 1).unwrap(), 1);
        assert_eq!(pread_all(&b, "new/deep/x.bin"), b"xQz");
        w.truncate("new/deep/x.bin", 2).unwrap();
        assert_eq!(pread_all(&b, "new/deep/x.bin"), b"xQ");
        assert_eq!(w.truncate("missing", 1).unwrap_err(), libc::ENOENT);
        assert_eq!(w.pwrite("", b"nope", 0).unwrap_err(), libc::EISDIR);

        // mkdir / remove with POSIX-flavored errnos
        w.mkdir("newdir", 0o750).unwrap();
        assert_eq!(b.stat("newdir").unwrap().entry_type, EntryType::Directory);
        #[cfg(unix)]
        assert_eq!(b.stat("newdir").unwrap().perms, 0o750);
        assert_eq!(w.mkdir("newdir", 0o750).unwrap_err(), libc::EEXIST);
        assert_eq!(w.mkdir("no/parent/dir", 0o750).unwrap_err(), libc::ENOENT);
        w.remove("newdir").unwrap();
        assert_eq!(w.remove("newdir").unwrap_err(), libc::ENOENT);
        assert_eq!(w.remove("new/deep").unwrap_err(), libc::ENOTEMPTY);
        w.remove("new/deep/x.bin").unwrap();
        w.remove("new/deep").unwrap();
    }

    #[test]
    fn hostdir_new_requires_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(HostDirBackend::new(&file).err(), Some(libc::ENOTDIR));
        assert_eq!(
            HostDirBackend::new(&dir.path().join("missing")).err(),
            Some(libc::ENOENT)
        );
    }
}
