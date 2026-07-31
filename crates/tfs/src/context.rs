//! FsContext: the process-global VFS state behind the C ABI.
//!
//! One mount table (legacy single-mount `init*` API on top of it via the
//! compat handle), the fd table (`TEBAKO_FD_FLAG`-tagged descriptors) and
//! the directory-handle table. Mirrors the C++ `FsContext`
//! (`src/c_api/fs_context.cpp`) semantics exactly; see each function's
//! comments for the errno contract.

use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat, WritableBackend};
use crate::exec_closure;
use crate::mount::MountMode;
use crate::policy::{HostAccess, HostPolicy};

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
    /// Mount mode (spec 11 §3; writes on RO mounts fail with EROFS).
    pub mode: MountMode,
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
    /// Host-access policy (spec 08 jails): consulted on every
    /// host-passthrough path decision (a path no memfs mount claims, and
    /// the mount family's image read). Process state, not namespace state:
    /// `unmount()` deliberately does NOT reset it (fail-closed); only the
    /// `tebako_fs_host_policy` C entry replaces it.
    host_policy: HostPolicy,
    /// The open audit-journal file for policy denials (spec 08 §2),
    /// resolved and opened by the policy installer BEFORE the context
    /// guard was taken — a denial is then a bare write(2), never a path
    /// operation under the lock (see `crate::journal` for the deadlock
    /// rationale). Follows the policy: replaced alongside it, left alone
    /// by `unmount()`.
    journal: Option<std::fs::File>,
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
            host_policy: HostPolicy::open(),
            journal: None,
        }
    }

    // ---------------------------------------------------------------
    // Host-access policy (spec 08)
    // ---------------------------------------------------------------

    /// Install the host-access policy, replacing the current one. The
    /// audit-journal file for denials (spec 08 §2) rides alongside —
    /// resolved and opened by the CALLER before the context guard was
    /// taken (`crate::journal::open_journal`), so a denial under the lock
    /// is a bare write(2), never a re-entrant path operation.
    pub fn set_host_policy(&mut self, policy: HostPolicy, journal: Option<std::fs::File>) {
        self.host_policy = policy;
        self.journal = journal;
    }

    /// Gate one host-passthrough path decision against the policy.
    /// Ok(()) = allowed (answer ENOENT, the consumer passes through to the
    /// host fs as today); Err(EPERM)/Err(EROFS) = the jail's answer.
    /// Denials are journaled (spec 08 §2 — the audit journal records the
    /// path, the op class and the policy's source label) via the cached,
    /// pre-opened file: a bare write(2), no path operation under the lock.
    pub fn host_check<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        need: HostAccess,
    ) -> Result<(), i32> {
        let path = path.as_ref();
        match self.host_policy.check(path, need) {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Some(journal) = &self.journal {
                    crate::journal::journal_deny(journal, path, need, self.host_policy.source());
                }
                Err(e)
            }
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

    /// Lexical normalization of a VFS path: `.` components dropped,
    /// `a/../` resolved (no symlink semantics — an image backend keys
    /// its entries by clean path), `..` at the root clamped. The host
    /// resolves `..` at the syscall layer; the mounts must see the same
    /// answer (ruby passes literal `lib/../x.yaml` paths through).
    fn normalize(path: &str) -> String {
        let absolute = path.starts_with('/');
        let mut out: Vec<&str> = Vec::new();
        for component in path.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    out.pop();
                }
                c => out.push(c),
            }
        }
        let joined = out.join("/");
        if absolute {
            format!("/{joined}")
        } else {
            joined
        }
    }

    /// The memfs tail of a dlmap-cache path. `dlmap2file` materializes a
    /// memfs file at `<tmp>/tebako-dl-<hex>/<full memfs path>`; a
    /// consumer that computes paths relative to a materialized binary
    /// (dyld's @rpath probes, a dlopen'd library's own dependencies)
    /// presents paths under the marker directory. Strip it: the
    /// remainder, normalized, is the memfs original. None when no
    /// marker component is present or nothing follows it.
    fn dlmap_tail(path: &str) -> Option<String> {
        let mut components = path.split('/');
        while let Some(component) = components.next() {
            if is_dlmap_marker(component) {
                let tail: Vec<&str> = components.collect();
                if tail.is_empty() {
                    return None;
                }
                return Some(Self::normalize(&format!("/{}", tail.join("/"))));
            }
        }
        None
    }

    // ---------------------------------------------------------------
    // File operations
    // ---------------------------------------------------------------

    /// tebako_fs_open: dispatch + fd allocation. Returns the public fd
    /// (with TEBAKO_FD_FLAG).
    pub fn open(&mut self, path: &str, flags: i32) -> Result<i32, i32> {
        let path = &Self::normalize(path);
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!("[tfs] open: {path}");
        }
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        // dlmap-prefix redirect: a path under the dlmap cache
        // (<tmp>/tebako-dl-<hex>/<memfs path>) names the materialized
        // form of a memfs path. Consumers that compute paths relative to
        // a materialized binary — dyld's @rpath probes, a dlopen'd
        // library's own dependencies — land here. Materialize the
        // original and answer with a REAL host fd: mmap-capable, unlike
        // a token fd (dyld maps what it opens). A tail the mounts do
        // not hold falls through to the literal path's normal routing.
        if let Some(tail) = Self::dlmap_tail(path) {
            if self.find_mount(&tail).is_some() {
                if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                    eprintln!("[tfs] dlmap redirect: {path} -> {tail}");
                }
                let host = self.dlmap2file(&tail)?;
                return real_open(&host, flags);
            }
        }
        let Some(mount) = self.find_mount(path) else {
            // Host-passthrough decision (spec 08): the policy gates the
            // consumer's fall-through to the host fs, for reads AND writes
            // alike — Ok => ENOENT ("not ours, pass through", today's
            // answer); Err => EPERM (outside every grant under deny) or
            // EROFS (write against an ro grant).
            let need = if (flags & libc::O_ACCMODE) == libc::O_RDONLY {
                HostAccess::Ro
            } else {
                HostAccess::Rw
            };
            self.host_check(path, need)?;
            return Err(libc::ENOENT);
        };
        let rel = Self::relative_path(mount, path);
        let st = match mount.backend.stat(rel) {
            Ok(st) => st,
            // A path a mount covers but its image does not hold is a
            // HOST path (spec 08): the policy gates the consumer's
            // fall-through exactly as if no mount claimed it. With the
            // app payload mounted at "/", this is what keeps the host
            // filesystem reachable.
            Err(e) if e == libc::ENOENT => {
                let need = if (flags & libc::O_ACCMODE) == libc::O_RDONLY {
                    HostAccess::Ro
                } else {
                    HostAccess::Rw
                };
                self.host_check(path, need)?;
                return Err(libc::ENOENT);
            }
            Err(e) => return Err(e),
        };
        // Only O_RDONLY is supported: mounted content is read-only
        // (fd-based writes land with the spec 11 §7 write family;
        // path-level writes (pwrite_path & co) are gated by the mount
        // mode).
        if (flags & libc::O_ACCMODE) != libc::O_RDONLY {
            return Err(libc::EROFS);
        }
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
        let path = &Self::normalize(path);
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        let Some(mount) = self.find_mount(path) else {
            // Host-passthrough decision (spec 08), see open().
            self.host_check(path, HostAccess::Ro)?;
            return Err(libc::ENOENT);
        };
        let rel = Self::relative_path(mount, path);
        let entries = match mount.backend.read_dir(rel) {
            Ok(entries) => entries,
            // Covered but not held: a host path (see open()).
            Err(e) if e == libc::ENOENT => {
                self.host_check(path, HostAccess::Ro)?;
                return Err(libc::ENOENT);
            }
            Err(e) => return Err(e),
        };
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

    /// Owned copy of the handle's current entry (additive; the preload
    /// shim's readdir fills a native `struct dirent` from it without
    /// touching raw pointers).
    pub fn dir_current(&self, dir: usize) -> Option<TebakoCDirent> {
        self.dir_table.get(&dir).map(|s| s.current.as_ref().clone())
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
        let path = &Self::normalize(path);
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        // dlmap-prefix redirect (see open()): answer with the memfs
        // original's metadata — the materialized copy open() hands out
        // carries the same content and permissions.
        if let Some(tail) = Self::dlmap_tail(path) {
            if let Some(mount) = self.find_mount(&tail) {
                let rel = Self::relative_path(mount, &tail);
                if let Ok(st) = mount.backend.stat(rel) {
                    return Ok(st);
                }
            }
        }
        let Some(mount) = self.find_mount(path) else {
            // Host-passthrough decision (spec 08), see open().
            self.host_check(path, HostAccess::Ro)?;
            return Err(libc::ENOENT);
        };
        let rel = Self::relative_path(mount, path);
        match mount.backend.stat(rel) {
            // Covered but not held: a host path (see open()).
            Err(e) if e == libc::ENOENT => {
                self.host_check(path, HostAccess::Ro)?;
                Err(libc::ENOENT)
            }
            other => other,
        }
    }

    /// tebako_fs_fstat (re-dispatched by the fd's path, like C++).
    pub fn fstat(&self, fd: i32) -> Result<RawStat, i32> {
        let (_, entry) = self.lookup_fd(fd).ok_or(libc::EBADF)?;
        let path = entry.path.clone();
        self.stat(&path)
    }

    // ---------------------------------------------------------------
    // Write operations (mount-mode gated; spec 11 §3/§4)
    //
    // Path-level writes route to the backend's WritableBackend view:
    // RO mounts fail EROFS (unchanged behavior), mounts whose backend
    // has no write view fail ENOTSUP. fd-based writes (the spec 11 §7
    // write family) are a later, additive milestone.
    // ---------------------------------------------------------------

    /// The writable backend owning `path`, with the in-image path.
    fn writable_for(&self, path: &str) -> Result<(&dyn WritableBackend, String), i32> {
        let path = &Self::normalize(path);
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        let mount = self.find_mount(path).ok_or(libc::ENOENT)?;
        if mount.mode == MountMode::ReadOnly {
            return Err(libc::EROFS);
        }
        let rel = Self::relative_path(mount, path).to_string();
        let w = mount.backend.writable().ok_or(libc::ENOTSUP)?;
        Ok((w, rel))
    }

    /// Write `data` at `offset` in `path` (COW: copy-up into the overlay).
    pub fn pwrite_path(&self, path: &str, data: &[u8], offset: u64) -> Result<usize, i32> {
        let (w, rel) = self.writable_for(path)?;
        w.pwrite(&rel, data, offset)
    }

    /// Truncate `path` to `len` bytes.
    pub fn truncate_path(&self, path: &str, len: u64) -> Result<(), i32> {
        let (w, rel) = self.writable_for(path)?;
        w.truncate(&rel, len)
    }

    /// Create a single directory.
    pub fn mkdir_path(&self, path: &str, perms: u32) -> Result<(), i32> {
        let (w, rel) = self.writable_for(path)?;
        w.mkdir(&rel, perms)
    }

    /// Remove a file, symlink or empty directory (COW: whiteouts the
    /// base entry).
    pub fn remove_path(&self, path: &str) -> Result<(), i32> {
        let (w, rel) = self.writable_for(path)?;
        w.remove(&rel)
    }

    // ---------------------------------------------------------------
    // Utility
    // ---------------------------------------------------------------

    /// tebako_path_is_embedded.
    pub fn path_is_embedded(&self, path: &str) -> bool {
        self.find_mount(path).is_some()
    }

    /// A mount HOLDS `path` (an entry exists in the image) — the
    /// write gate's discriminator. A covered-but-not-held path is a
    /// host path (the spec 08 passthrough, same as open/stat): with a
    /// `/` mount in play, this is what keeps host writes legal.
    pub fn path_is_held(&self, path: &str) -> bool {
        let path = &Self::normalize(path);
        let Some(mount) = self.find_mount(path) else {
            return false;
        };
        mount.backend.stat(Self::relative_path(mount, path)).is_ok()
    }

    /// The mount table in the `TEBAKO_TFS_MOUNTS` grammar
    /// ("image:mount,image:mount,…") — the env a spawned child needs to
    /// re-establish this namespace through the preload shim. Only
    /// file-backed mounts serialize; memory mounts have no image path
    /// and are skipped (a child cannot remount them anyway).
    pub fn mounts_env(&self) -> Option<std::ffi::CString> {
        let mut out = String::new();
        for mount in self.mounts.values() {
            let Some(archive) = &mount.archive_path else {
                continue;
            };
            if !out.is_empty() {
                out.push(',');
            }
            out.push_str(&archive.to_string_lossy());
            out.push(':');
            out.push_str(&mount.mount_point);
        }
        if out.is_empty() {
            None
        } else {
            std::ffi::CString::new(out).ok()
        }
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
        // The destination is a host-WRITE decision (spec 08): the policy
        // must grant it before any memfs content lands on the host.
        self.host_check(dest, HostAccess::Rw)?;
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
    /// dlopen, with per-process cache and tmpdir. The extraction layout
    /// is `<tmp>/tebako-dl-<hex>/<full memfs path>` (a deviation from the
    /// C++ oracle's mount-basename layout, which collided across mounts
    /// sharing a basename): the full path makes the dlmap-prefix
    /// redirect (open/stat) an exact inverse and lets paths computed
    /// relative to a materialized binary resolve back to their memfs
    /// originals.
    pub fn dlmap2file(&mut self, path: &str) -> Result<std::ffi::CString, i32> {
        let path = &Self::normalize(path);
        // dlmap-prefix redirect (see open()): the dlmap spelling of a
        // memfs path materializes the original — stdio (`fopen`) and
        // dlopen consumers of loader-computed paths land here.
        let tail = Self::dlmap_tail(path);
        let effective = tail.as_deref().unwrap_or(path);
        let mut visited = std::collections::HashSet::new();
        let host = self.extract_for_exec(effective, effective, &ClosureDest::Dlcache, &[], &mut visited)?;
        let s = host.to_string_lossy().into_owned();
        std::ffi::CString::new(s).map_err(|_| libc::EIO)
    }

    /// The store-side sibling of dlmap2file (tebako install's
    /// zero-runtime materialization): extract `path` plus its exec
    /// dependency closure to `<dest>/<full memfs path>` and return the
    /// host path of the file itself. No cache, no cleanup — the store
    /// tree is permanent and mirrors the memfs layout so the platform
    /// loader's `@executable_path`-relative probes land on real files.
    pub fn extract_exec_closure(
        &mut self,
        path: &str,
        dest: &std::path::Path,
    ) -> Result<std::path::PathBuf, i32> {
        let mut visited = std::collections::HashSet::new();
        self.extract_for_exec(
            path,
            path,
            &ClosureDest::Store(dest.to_path_buf()),
            &[],
            &mut visited,
        )
    }

    /// One closure-extraction step: materialize `path` under the
    /// destination root, then walk its Mach-O/ELF dependency closure
    /// recursively. `exe` is the original exec target
    /// (`@executable_path` anchor), `chain_rpaths` the rpaths
    /// accumulated down the load chain, `visited` the cycle-breaking
    /// set of memfs paths already extracted in this walk.
    fn extract_for_exec(
        &mut self,
        path: &str,
        exe: &str,
        dest: &ClosureDest,
        chain_rpaths: &[String],
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<std::path::PathBuf, i32> {
        let path = &Self::normalize(path);
        let Some(mount) = self.find_mount(path) else {
            // Host-passthrough decision (spec 08), see open(). The
            // extraction writes themselves are process-internal and
            // not policy-gated.
            self.host_check(path, HostAccess::Ro)?;
            return Err(libc::ENOENT);
        };
        let owner = mount.handle;
        let rel_owned = Self::relative_path(mount, path).to_string();
        let mount = self.mounts.get(&owner).unwrap();

        if matches!(dest, ClosureDest::Dlcache) {
            if let Some(cached) = self.dl_cache.get(path) {
                return Ok(cached.clone());
            }
        }

        if rel_owned.is_empty() {
            return Err(libc::EISDIR);
        }
        let st = match mount.backend.stat(&rel_owned) {
            Ok(st) => st,
            // Covered but not held: a host path (see open()) — the
            // consumer falls back to the host answer.
            Err(e) if e == libc::ENOENT => {
                self.host_check(path, HostAccess::Ro)?;
                return Err(libc::ENOENT);
            }
            Err(e) => return Err(e),
        };
        if st.entry_type != EntryType::File {
            return Err(libc::EISDIR);
        }

        let root = match dest {
            ClosureDest::Dlcache => match &self.dl_tmpdir {
                Some(d) => d.clone(),
                None => {
                    let d = create_dl_tmpdir().ok_or(libc::EIO)?;
                    register_dl_cleanup(&d);
                    self.dl_tmpdir = Some(d.clone());
                    d
                }
            },
            ClosureDest::Store(root) => root.clone(),
        };

        let host_path = root.join(path.trim_start_matches('/'));
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

        if matches!(dest, ClosureDest::Dlcache) {
            self.dl_cache.insert(path.to_string(), host_path.clone());
        }
        // Eager dependency closure: the platform loader's probes are
        // raw syscalls no userland hook can serve (dyld proven on
        // macOS 15), so the image's dylibs must exist as real files
        // under the same root before exec/dlopen runs. Unresolvable
        // names are host/system libraries — the loader answers for
        // them exactly as before.
        visited.insert(path.to_string());
        use std::io::Read as _;
        let mut head = Vec::new();
        let Ok(_) = std::fs::File::open(&host_path)
            .and_then(|f| f.take(exec_closure::HEADER_WINDOW as u64).read_to_end(&mut head))
        else {
            return Ok(host_path);
        };
        let Some(parsed) = exec_closure::parse(&head) else {
            return Ok(host_path);
        };
        let referrer_dir = memfs_dirname(path);
        let exe_dir = memfs_dirname(exe);
        let mut chain: Vec<String> = chain_rpaths.to_vec();
        for rp in &parsed.rpaths {
            if !chain.contains(rp) {
                chain.push(rp.clone());
            }
        }
        for dep in &parsed.deps {
            let Some(memfs) = self.resolve_dep(dep, &referrer_dir, &exe_dir, &parsed.rpaths, &chain)
            else {
                continue;
            };
            if visited.contains(&memfs) {
                continue;
            }
            // The dep's own closure rides its extraction (same exe,
            // same destination).
            let _ = self.extract_for_exec(&memfs, exe, dest, &chain, visited);
        }
        Ok(host_path)
    }

    /// Resolve one dependency name to a memfs path the mounts hold
    /// (dyld semantics, simplified): `@rpath` names try each rpath of
    /// the referencing image then the chain's; `@executable_path` /
    /// `@loader_path` / `$ORIGIN` expand against the exec target and
    /// the referrer; absolute paths are taken verbatim; bare names take
    /// the rpath lookup.
    fn resolve_dep(
        &self,
        name: &str,
        referrer_dir: &str,
        exe_dir: &str,
        own_rpaths: &[String],
        chain_rpaths: &[String],
    ) -> Option<String> {
        let mut candidates: Vec<String> = Vec::new();
        let mut push = |c: String| {
            if !candidates.contains(&c) {
                candidates.push(c);
            }
        };
        if let Some(rest) = name.strip_prefix("@rpath/") {
            for rp in own_rpaths.iter().chain(chain_rpaths) {
                push(Self::normalize(&format!(
                    "{}/{rest}",
                    expand_loader_vars(rp, referrer_dir, exe_dir)
                )));
            }
        } else if name.starts_with('@') || name.contains('$') {
            push(Self::normalize(&expand_loader_vars(name, referrer_dir, exe_dir)));
        } else if name.starts_with('/') {
            push(Self::normalize(name));
        } else {
            for rp in own_rpaths.iter().chain(chain_rpaths) {
                push(Self::normalize(&format!(
                    "{}/{name}",
                    expand_loader_vars(rp, referrer_dir, exe_dir)
                )));
            }
        }
        candidates.into_iter().find(|c| self.held_file(c))
    }

    /// True when `path` is held by a mount as a regular file.
    fn held_file(&self, path: &str) -> bool {
        let Some(mount) = self.find_mount(path) else {
            return false;
        };
        matches!(
            mount.backend.stat(Self::relative_path(mount, path)),
            Ok(st) if st.entry_type == EntryType::File
        )
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

/// A `tebako-dl-<hex>` path component (the create_dl_tmpdir marker).
fn is_dlmap_marker(component: &str) -> bool {
    let Some(hex) = component.strip_prefix("tebako-dl-") else {
        return false;
    };
    !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Where an exec-closure extraction lands.
enum ClosureDest {
    /// The per-process dlmap tmpdir (dlmap2file): cached, cleaned at exit.
    Dlcache,
    /// A permanent store root (tebako install's zero-runtime
    /// materialization): never cleaned, never cached.
    Store(std::path::PathBuf),
}

/// The parent directory of a memfs path (`/a/b/c` → `/a/b`, `/a` → `/`).
fn memfs_dirname(path: &str) -> String {
    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => path[..i].to_string(),
    }
}

/// Expand the loader variables in a dependency or rpath template:
/// `@executable_path` (the exec target's memfs dir), `@loader_path` and
/// `$ORIGIN` (the referencing image's memfs dir).
fn expand_loader_vars(template: &str, referrer_dir: &str, exe_dir: &str) -> String {
    template
        .replace("@executable_path", exe_dir)
        .replace("@loader_path", referrer_dir)
        .replace("$ORIGIN", referrer_dir)
}

/// Open a materialized dlmap copy for real (no TEBAKO_FD_FLAG): the
/// consumer's is-embedded check answers false and treats the fd as a
/// plain host descriptor — which it is. O_CREAT/O_TRUNC are masked:
/// the copy exists by construction and truncating a shared mmap source
/// is never the caller's intent.
fn real_open(path: &std::ffi::CString, flags: i32) -> Result<i32, i32> {
    let flags = flags & !(libc::O_CREAT | libc::O_TRUNC);
    // SAFETY: path is a valid CString that outlives the call; no
    // O_CREAT means the variadic mode argument is not read.
    let fd = unsafe { libc::open(path.as_ptr(), flags) };
    if fd < 0 {
        Err(std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EIO))
    } else {
        Ok(fd)
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
