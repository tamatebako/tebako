//! FsContext: the process-global VFS state behind the C ABI.
//!
//! One mount table (legacy single-mount `init*` API on top of it via the
//! compat handle), the fd table (`TEBAKO_FD_FLAG`-tagged descriptors) and
//! the directory-handle table. Mirrors the C++ `FsContext`
//! (`src/c_api/fs_context.cpp`) semantics exactly; see each function's
//! comments for the errno contract.

use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat};

/// Flag bit distinguishing libtfs FDs from host OS FDs.
pub const TEBAKO_FD_FLAG: i32 = 0x4000_0000;
/// Maximum internal FD value.
pub const TEBAKO_FD_MAX: i32 = 0x0FFF_FFFF;

/// Directory entry type constant (POSIX DT_REG).
pub const DT_REG: u8 = 8;
/// Directory entry type constant (POSIX DT_DIR).
pub const DT_DIR: u8 = 4;

/// `struct tebako_c_dirent` from the C API header, POSIX-dirent flavored.
#[repr(C)]
#[derive(Clone)]
pub struct TebakoCDirent {
    /// Entry name (NUL-terminated, truncated to 255 bytes like the C++ side).
    pub d_name: [libc::c_char; 256],
    /// DT_REG or DT_DIR.
    pub d_type: u8,
}

impl Default for TebakoCDirent {
    fn default() -> Self {
        TebakoCDirent {
            d_name: [0; 256],
            d_type: 0,
        }
    }
}

impl TebakoCDirent {
    fn fill_from(&mut self, entry: &RawDirEntry) {
        // strncpy semantics: zero-fill, then copy at most 255 bytes.
        self.d_name = [0; 256];
        let bytes = entry.name.as_bytes();
        let n = bytes.len().min(255);
        for (i, &b) in bytes[..n].iter().enumerate() {
            self.d_name[i] = b as libc::c_char;
        }
        self.d_type = if entry.is_dir { DT_DIR } else { DT_REG };
    }
}

/// One mounted archive.
pub struct Mount {
    /// Mount handle (never reused within a process run).
    pub handle: i32,
    /// Virtual mount point (e.g. "/__tebako__").
    pub mount_point: String,
    /// NUL-terminated mount point, boxed for a pointer stable across
    /// BTreeMap moves (tebako_get_mount_point's "valid until unmount").
    pub mount_point_c: Box<std::ffi::CString>,
    /// Archive path on disk, when mounted from a file.
    pub archive_path: Option<Box<std::ffi::CString>>,
    /// The backend.
    pub backend: Box<dyn Backend>,
}

/// One open file descriptor.
pub struct FdEntry {
    /// In-image path (for fstat re-dispatch, like the C++ implementation).
    pub path: String,
    /// File size at open time.
    pub size: u64,
    /// Current position.
    pub pos: u64,
    /// Owning mount handle.
    pub owner: i32,
}

/// One open directory handle.
pub struct DirState {
    /// Snapshot of the directory's entries at opendir time.
    pub entries: Vec<RawDirEntry>,
    /// Ordinal of the entry the next readdir returns.
    pub position: usize,
    /// Owning mount handle.
    #[allow(dead_code)]
    pub owner: i32,
    /// Current entry in ABI form; the pointer returned by readdir points
    /// here and stays valid until the next readdir/closedir (boxed so the
    /// address is stable across BTreeMap moves).
    pub current: Box<TebakoCDirent>,
}

/// The process-global context state (behind a lock; see [`context`]).
pub struct FsContext {
    mounts: BTreeMap<i32, Mount>,
    fd_table: BTreeMap<i32, FdEntry>,
    dir_table: BTreeMap<usize, DirState>,
    next_handle: i32,
    next_fd: i32,
    next_dir_id: usize,
    /// The mount made through the legacy single-mount `init*` API; the
    /// compat getters (`tebako_get_mount_point`, ...) report on it.
    compat_handle: Option<i32>,
    /// Per-process temp dir for dlmap2file extractions (created lazily).
    dl_tmpdir: Option<std::path::PathBuf>,
    /// dlmap2file cache: memfs path -> extracted host path. Extractions
    /// live for the process run and are removed at teardown (atexit).
    dl_cache: BTreeMap<String, std::path::PathBuf>,
}

impl FsContext {
    const fn new() -> Self {
        FsContext {
            mounts: BTreeMap::new(),
            fd_table: BTreeMap::new(),
            dir_table: BTreeMap::new(),
            next_handle: 0,
            next_fd: 1,
            next_dir_id: 1,
            compat_handle: None,
            dl_tmpdir: None,
            dl_cache: BTreeMap::new(),
        }
    }

    // ---------------------------------------------------------------
    // Mount management
    // ---------------------------------------------------------------

    /// Legacy single-mount init: fails with EEXIST when anything is mounted.
    pub fn init_mount(&mut self, mount: Mount) -> Result<(), i32> {
        if !self.mounts.is_empty() {
            return Err(libc::EEXIST);
        }
        let handle = self.insert_mount(mount);
        self.compat_handle = Some(handle);
        Ok(())
    }

    /// Insert a mount; returns its handle.
    pub fn insert_mount(&mut self, mount: Mount) -> i32 {
        let handle = self.next_handle;
        self.next_handle += 1;
        let mount = Mount { handle, ..mount };
        self.mounts.insert(mount.handle, mount);
        handle
    }

    /// Multi-mount API: validate + insert, with the C++ errno contract.
    /// `Err` on taken mount point (EEXIST) or empty mount point (EINVAL).
    pub fn mount_checked(&mut self, mount: Mount) -> Result<i32, i32> {
        if mount.mount_point.is_empty() {
            return Err(libc::EINVAL);
        }
        if self.mount_point_taken(&mount.mount_point) {
            return Err(libc::EEXIST);
        }
        Ok(self.insert_mount(mount))
    }

    /// Unmount a single mount by handle: force-close only its own fds and
    /// dir handles (they fail with EBADF afterwards), drop the mount, and
    /// release the mount point. Handles are never reused.
    pub fn unmount_handle(&mut self, handle: i32) -> Result<(), i32> {
        if !self.mounts.contains_key(&handle) {
            return Err(libc::ENODEV);
        }
        self.fd_table.retain(|_, e| e.owner != handle);
        self.dir_table.retain(|_, e| e.owner != handle);
        self.mounts.remove(&handle);
        if self.compat_handle == Some(handle) {
            self.compat_handle = None;
        }
        Ok(())
    }

    /// Unmount everything; all fds and dir handles become invalid.
    pub fn unmount(&mut self) {
        self.mounts.clear();
        self.fd_table.clear();
        self.dir_table.clear();
        self.next_fd = 1;
        self.next_dir_id = 1;
        self.compat_handle = None;
    }

    pub fn is_mounted(&self) -> bool {
        !self.mounts.is_empty()
    }

    /// True when `mount_point` is already taken.
    pub fn mount_point_taken(&self, mount_point: &str) -> bool {
        self.mounts.values().any(|m| m.mount_point == mount_point)
    }

    /// Longest-prefix dispatch: the mount owning `path`, if any.
    fn find_mount(&self, path: &str) -> Option<&Mount> {
        self.mounts
            .values()
            .filter(|m| path_is_in_mount(path, &m.mount_point))
            .max_by_key(|m| m.mount_point.len())
    }

    /// Strip the mount point: the in-image path for `path` under `mount`
    /// ("" for the mount root).
    fn relative_path<'a>(mount: &Mount, path: &'a str) -> &'a str {
        let rest = &path[mount.mount_point.len()..];
        rest.trim_start_matches('/')
    }

    // ---------------------------------------------------------------
    // File operations
    // ---------------------------------------------------------------

    /// tebako_fs_open: dispatch + fd allocation. Returns the public fd
    /// (with TEBAKO_FD_FLAG).
    pub fn open(&mut self, path: &str, flags: i32) -> Result<i32, i32> {
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        // Only O_RDONLY is supported.
        if (flags & libc::O_ACCMODE) != libc::O_RDONLY {
            return Err(libc::EROFS);
        }
        let mount = self.find_mount(path).ok_or(libc::ENOENT)?;
        let rel = Self::relative_path(mount, path);
        let st = mount.backend.stat(rel)?;
        match st.entry_type {
            EntryType::File => {}
            // C++ maps NotAFile -> EISDIR for any non-regular open.
            _ => return Err(libc::EISDIR),
        }
        let owner = mount.handle;
        let fd = self.next_fd;
        if fd > TEBAKO_FD_MAX {
            return Err(libc::EMFILE);
        }
        self.next_fd += 1;
        self.fd_table.insert(
            fd,
            FdEntry {
                path: path.to_string(),
                size: st.size.max(0) as u64,
                pos: 0,
                owner,
            },
        );
        Ok(fd | TEBAKO_FD_FLAG)
    }

    fn lookup_fd(&self, fd: i32) -> Option<(i32, &FdEntry)> {
        if (fd & TEBAKO_FD_FLAG) == 0 {
            return None;
        }
        let internal = fd & !TEBAKO_FD_FLAG;
        self.fd_table.get(&internal).map(|e| (internal, e))
    }

    fn lookup_fd_mut(&mut self, fd: i32) -> Option<(i32, &mut FdEntry)> {
        if (fd & TEBAKO_FD_FLAG) == 0 {
            return None;
        }
        let internal = fd & !TEBAKO_FD_FLAG;
        self.fd_table.get_mut(&internal).map(|e| (internal, e))
    }

    /// tebako_fs_read.
    pub fn read(&mut self, fd: i32, buf: &mut [u8]) -> Result<usize, i32> {
        let (internal, path, owner, pos, size) = {
            let (internal, entry) = self.lookup_fd(fd).ok_or(libc::EBADF)?;
            (
                internal,
                entry.path.clone(),
                entry.owner,
                entry.pos,
                entry.size,
            )
        };
        let mount = self.mounts.get(&owner).ok_or(libc::EBADF)?;
        let rel = Self::relative_path(mount, &path).to_string();
        if pos >= size || buf.is_empty() {
            return Ok(0);
        }
        let want = std::cmp::min(buf.len() as u64, size - pos) as usize;
        let n = mount.backend.pread(&rel, &mut buf[..want], pos)?;
        let entry = self.fd_table.get_mut(&internal).ok_or(libc::EBADF)?;
        entry.pos += n as u64;
        Ok(n)
    }

    /// tebako_fs_pread (position untouched).
    pub fn pread(&self, fd: i32, buf: &mut [u8], offset: i64) -> Result<usize, i32> {
        if offset < 0 {
            return Err(libc::EINVAL);
        }
        let (_, entry) = self.lookup_fd(fd).ok_or(libc::EBADF)?;
        let mount = self.mounts.get(&entry.owner).ok_or(libc::EBADF)?;
        let rel = Self::relative_path(mount, &entry.path);
        let offset = offset as u64;
        if offset >= entry.size || buf.is_empty() {
            return Ok(0);
        }
        let want = std::cmp::min(buf.len() as u64, entry.size - offset) as usize;
        mount.backend.pread(rel, &mut buf[..want], offset)
    }

    /// tebako_fs_lseek.
    pub fn lseek(&mut self, fd: i32, offset: i64, whence: i32) -> Result<i64, i32> {
        let (_, entry) = self.lookup_fd_mut(fd).ok_or(libc::EBADF)?;
        let size = entry.size as i64;
        let target = match whence {
            libc::SEEK_SET => offset,
            libc::SEEK_CUR => entry.pos as i64 + offset,
            libc::SEEK_END => size + offset,
            _ => return Err(libc::EINVAL),
        };
        if !(0..=size).contains(&target) {
            return Err(libc::EINVAL);
        }
        entry.pos = target as u64;
        Ok(target)
    }

    /// tebako_fs_close.
    pub fn close(&mut self, fd: i32) -> Result<(), i32> {
        if (fd & TEBAKO_FD_FLAG) == 0 {
            return Err(libc::EBADF);
        }
        match self.fd_table.remove(&(fd & !TEBAKO_FD_FLAG)) {
            Some(_) => Ok(()),
            None => Err(libc::EBADF),
        }
    }

    // ---------------------------------------------------------------
    // Directory operations
    // ---------------------------------------------------------------

    /// tebako_fs_opendir. Returns the raw dir-handle id.
    pub fn opendir(&mut self, path: &str) -> Result<usize, i32> {
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        let mount = self.find_mount(path).ok_or(libc::ENOENT)?;
        let rel = Self::relative_path(mount, path);
        let entries = mount.backend.read_dir(rel)?;
        let owner = mount.handle;
        let id = self.next_dir_id;
        self.next_dir_id += 1;
        self.dir_table.insert(
            id,
            DirState {
                entries,
                position: 0,
                owner,
                current: Box::default(),
            },
        );
        Ok(id)
    }

    /// tebako_fs_readdir: fill the handle's current-entry buffer; Ok(false)
    /// at end of directory.
    pub fn readdir_abi(&mut self, dir: usize) -> Result<bool, i32> {
        let state = self.dir_table.get_mut(&dir).ok_or(libc::EBADF)?;
        if state.position >= state.entries.len() {
            return Ok(false);
        }
        let entry = state.entries[state.position].clone();
        state.position += 1;
        state.current.fill_from(&entry);
        Ok(true)
    }

    /// Pointer to the handle's current-entry buffer (NULL when unknown).
    pub fn dir_current_ptr(&self, dir: usize) -> *const TebakoCDirent {
        self.dir_table
            .get(&dir)
            .map(|s| std::ptr::from_ref(s.current.as_ref()))
            .unwrap_or(std::ptr::null())
    }

    /// tebako_fs_closedir.
    pub fn closedir(&mut self, dir: usize) -> Result<(), i32> {
        match self.dir_table.remove(&dir) {
            Some(_) => Ok(()),
            None => Err(libc::EBADF),
        }
    }

    /// tebako_fs_dir_is_embedded: registry-membership test.
    pub fn dir_is_embedded(&self, dir: usize) -> bool {
        self.dir_table.contains_key(&dir)
    }

    /// tebako_fs_telldir: ordinal of the entry the next readdir returns.
    pub fn telldir(&self, dir: usize) -> Result<i64, i32> {
        let state = self.dir_table.get(&dir).ok_or(libc::EBADF)?;
        Ok(state.position as i64)
    }

    /// tebako_fs_rewinddir.
    pub fn rewinddir(&mut self, dir: usize) -> Result<(), i32> {
        let state = self.dir_table.get_mut(&dir).ok_or(libc::EBADF)?;
        state.position = 0;
        Ok(())
    }

    /// tebako_fs_seekdir (index-based cookies; seeking past the end leaves
    /// the stream at end-of-directory).
    pub fn seekdir(&mut self, dir: usize, pos: i64) -> Result<(), i32> {
        if pos < 0 {
            return Err(libc::EINVAL);
        }
        let state = self.dir_table.get_mut(&dir).ok_or(libc::EBADF)?;
        state.position = (pos as usize).min(state.entries.len());
        Ok(())
    }

    // ---------------------------------------------------------------
    // Metadata
    // ---------------------------------------------------------------

    /// tebako_fs_stat.
    pub fn stat(&self, path: &str) -> Result<RawStat, i32> {
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        let mount = self.find_mount(path).ok_or(libc::ENOENT)?;
        let rel = Self::relative_path(mount, path);
        mount.backend.stat(rel)
    }

    /// tebako_fs_fstat (re-dispatched by the fd's path, like C++).
    pub fn fstat(&self, fd: i32) -> Result<RawStat, i32> {
        let (_, entry) = self.lookup_fd(fd).ok_or(libc::EBADF)?;
        let path = entry.path.clone();
        self.stat(&path)
    }

    // ---------------------------------------------------------------
    // Utility
    // ---------------------------------------------------------------

    /// tebako_path_is_embedded.
    pub fn path_is_embedded(&self, path: &str) -> bool {
        self.find_mount(path).is_some()
    }

    // ---------------------------------------------------------------
    // Extraction
    // ---------------------------------------------------------------

    /// tebako_fs_extract_all: one mount extracts directly into `dest`;
    /// multiple mounts each extract into `<dest>/<mount-point-basename>`.
    pub fn extract_all(&mut self, dest: &std::path::Path) -> Result<(), i32> {
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        if std::fs::create_dir_all(dest).is_err() {
            return Err(libc::EIO);
        }
        if self.mounts.len() == 1 {
            // Single mount: historic behavior — tree directly into dest.
            let mount = self.mounts.values().next().unwrap();
            extract_dir_recursive(mount.backend.as_ref(), "", dest)?;
        } else {
            let mounts: Vec<&Mount> = self.mounts.values().collect();
            for mount in mounts {
                let subtree = dest.join(mount_point_basename(&mount.mount_point));
                if std::fs::create_dir_all(&subtree).is_err() {
                    return Err(libc::EIO);
                }
                extract_dir_recursive(mount.backend.as_ref(), "", &subtree)?;
            }
        }
        Ok(())
    }

    // ---------------------------------------------------------------
    // dlmap2file
    // ---------------------------------------------------------------

    /// tebako_fs_dlmap2file: extract a memfs file to a host path for
    /// dlopen, with per-process cache and tmpdir (legacy semantics).
    pub fn dlmap2file(&mut self, path: &str) -> Result<std::ffi::CString, i32> {
        let mount = self.find_mount(path).ok_or(libc::ENOENT)?;
        let owner = mount.handle;
        let mount_point = mount.mount_point.clone();
        let rel_owned = Self::relative_path(mount, path).to_string();
        let mount = self.mounts.get(&owner).unwrap();

        if let Some(cached) = self.dl_cache.get(path) {
            let s = cached.to_string_lossy().into_owned();
            return std::ffi::CString::new(s).map_err(|_| libc::EIO);
        }

        if rel_owned.is_empty() {
            return Err(libc::EISDIR);
        }
        let st = mount.backend.stat(&rel_owned)?;
        if st.entry_type != EntryType::File {
            return Err(libc::EISDIR);
        }

        let tmpdir = match &self.dl_tmpdir {
            Some(d) => d.clone(),
            None => {
                let d = create_dl_tmpdir().ok_or(libc::EIO)?;
                register_dl_cleanup(&d);
                self.dl_tmpdir = Some(d.clone());
                d
            }
        };

        let host_path = tmpdir
            .join(mount_point_basename(&mount_point))
            .join(&rel_owned);
        if let Some(parent) = host_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| libc::EIO)?;
        }

        // Stream the file out in chunks.
        let mut out = std::fs::File::create(&host_path).map_err(|_| libc::EIO)?;
        let mut offset = 0u64;
        let mut buf = vec![0u8; 8192];
        loop {
            let n = mount.backend.pread(&rel_owned, &mut buf, offset)?;
            if n == 0 {
                break;
            }
            use std::io::Write as _;
            out.write_all(&buf[..n]).map_err(|_| {
                let _ = std::fs::remove_file(&host_path);
                libc::EIO
            })?;
            offset += n as u64;
        }
        drop(out);

        // Permissions, best effort (dlopen needs a readable file).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(&host_path, std::fs::Permissions::from_mode(st.perms));
        }

        self.dl_cache.insert(path.to_string(), host_path.clone());
        let s = host_path.to_string_lossy().into_owned();
        std::ffi::CString::new(s).map_err(|_| libc::EIO)
    }

    /// Compat getters (report on the legacy init mount).
    pub fn compat_mount_point(&self) -> Option<&std::ffi::CString> {
        let handle = self.compat_handle?;
        let mount = self.mounts.get(&handle)?;
        Some(mount.mount_point_c.as_ref())
    }

    pub fn compat_archive_path(&self) -> Option<&std::ffi::CString> {
        let handle = self.compat_handle?;
        self.mounts.get(&handle)?.archive_path.as_deref()
    }

    pub fn compat_backend_name(&self) -> Option<&'static std::ffi::CStr> {
        let handle = self.compat_handle?;
        Some(self.mounts.get(&handle)?.backend.name())
    }
}

/// Mount-point membership with path-component boundaries
/// (mirrors the C++ `path_is_in_mount`).
fn path_is_in_mount(path: &str, mount: &str) -> bool {
    if mount.is_empty() || path.len() < mount.len() || !path.starts_with(mount) {
        return false;
    }
    if path.len() == mount.len() {
        return true;
    }
    if mount.ends_with('/') {
        return true;
    }
    path.as_bytes()[mount.len()] == b'/'
}

/// Basename of a mount point for per-mount extraction subtrees
/// (mirrors the C++ `mount_point_basename`): strips trailing slashes and
/// takes the last component; "root" when nothing usable remains.
fn mount_point_basename(mount_point: &str) -> &str {
    let mp = mount_point.trim_end_matches('/');
    let base = mp.rsplit('/').next().unwrap_or(mp);
    if base.is_empty() {
        "root"
    } else {
        base
    }
}

/// Create the per-process temporary directory for dlmap2file extractions
/// (mirrors the legacy C++ semantics: a unique subdirectory of the system
/// temp dir; a handful of attempts before giving up).
fn create_dl_tmpdir() -> Option<std::path::PathBuf> {
    let base = std::env::temp_dir();
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    for _ in 0..16 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let dir = base.join(format!("tebako-dl-{:x}", (seed >> 33) as u64));
        if std::fs::create_dir(&dir).is_ok() {
            return Some(dir);
        }
    }
    None
}

/// Register process-teardown removal of the dl tmpdir (the C++ FsContext
/// destructor semantics), once per process.
fn register_dl_cleanup(dir: &std::path::Path) {
    use std::sync::{Mutex, Once};
    static CLEANUP_DIR: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        *CLEANUP_DIR.lock().unwrap() = Some(dir.to_path_buf());
        extern "C" fn cleanup() {
            if let Some(d) = CLEANUP_DIR.lock().unwrap().take() {
                let _ = std::fs::remove_dir_all(d);
            }
        }
        unsafe { libc::atexit(cleanup) };
    });
}

/// Recursively extract a backend tree (`rel_dir` is in-image, "" = root)
/// into `host_dir` (created). Errors are errno values (EIO on host
/// failures).
fn extract_dir_recursive(
    backend: &dyn Backend,
    rel_dir: &str,
    host_dir: &std::path::Path,
) -> Result<(), i32> {
    std::fs::create_dir_all(host_dir).map_err(|_| libc::EIO)?;
    for entry in backend.read_dir(rel_dir)? {
        let child_rel = if rel_dir.is_empty() {
            entry.name.clone()
        } else {
            format!("{rel_dir}/{}", entry.name)
        };
        let child_host = host_dir.join(&entry.name);
        if entry.is_dir {
            extract_dir_recursive(backend, &child_rel, &child_host)?;
        } else {
            extract_file(backend, &child_rel, &child_host)?;
        }
    }
    Ok(())
}

/// Stream one file out of a backend onto the host (permissions and
/// modification time preserved, best effort).
fn extract_file(backend: &dyn Backend, rel: &str, host: &std::path::Path) -> Result<(), i32> {
    use std::io::Write as _;
    let st = backend.stat(rel)?;
    let mut out = std::fs::File::create(host).map_err(|_| libc::EIO)?;
    let mut offset = 0u64;
    let mut buf = vec![0u8; 8192];
    loop {
        let n = backend.pread(rel, &mut buf, offset)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|_| libc::EIO)?;
        offset += n as u64;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(host, std::fs::Permissions::from_mode(st.perms));
        let _ = out.set_modified(
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(st.mtime.max(0) as u64),
        );
    }
    Ok(())
}

/// The process-global context. Public C API functions lock it for the
/// duration of each call (exactly like the C++ implementation's mutex).
pub fn context() -> &'static RwLock<FsContext> {
    static CONTEXT: RwLock<FsContext> = RwLock::new(FsContext::new());
    &CONTEXT
}
