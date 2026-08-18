//! FsContext: the process-global VFS state behind the C ABI.
//!
//! One mount table (legacy single-mount `init*` API on top of it via the
//! compat handle), the fd table (`TEBAKO_FD_FLAG`-tagged descriptors) and
//! the directory-handle table. Mirrors the C++ `FsContext`
//! (`src/c_api/fs_context.cpp`) semantics exactly; see each function's
//! comments for the errno contract.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat, WritableBackend};
use crate::exec_closure;
use crate::mount::MountMode;
use crate::policy::{HostAccess, HostPolicy};

/// Flag bit distinguishing libtfs FDs from host OS FDs.
pub const TEBAKO_FD_FLAG: i32 = 0x4000_0000;
/// Maximum internal FD value.
pub const TEBAKO_FD_MAX: i32 = 0x0FFF_FFFF;

/// The access-mode mask of open flags: `O_ACCMODE` on unix, spelled out
/// on Windows (the libc crate carries no O_ACCMODE there; the three
/// access modes are the whole set).
#[cfg(unix)]
pub(crate) const O_ACCMODE: i32 = libc::O_ACCMODE;
#[cfg(windows)]
pub(crate) const O_ACCMODE: i32 = libc::O_RDONLY | libc::O_WRONLY | libc::O_RDWR;

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

/// One registered library alias (spec 22 §2.1, phase W2): the declared
/// bare name (the manifest's own spelling — the match is
/// case-insensitive, the spelling is preserved for the journal) and the
/// alias target's boot-materialized absolute host path.
struct DlAlias {
    name: String,
    host: std::path::PathBuf,
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
    /// The home-layout verdict per mount handle (the in-image manifest's
    /// `identity.annotations.java_home`), memoized on first exec probe —
    /// see `exec_materialize`.
    home_memos: BTreeMap<i32, bool>,
    /// The home mounts whose whole tree already materialized into the
    /// dl tmpdir this process run (extract once per mount).
    home_trees: BTreeSet<i32>,
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
    /// The boot's registered library-alias table (spec 22 §2.1, phase
    /// W2 — the windows bare-name rule's covered surface): declared bare
    /// name → the target's boot-materialized absolute host path.
    /// Installed by the driver after the class-L extraction pass
    /// (windows boots only — the registration channel itself is
    /// platform-neutral); the DEFAULT empty table answers every name
    /// HOST, the bare-name rule's default. Process state like the
    /// policy: `unmount()` deliberately leaves it.
    dlaliases: Vec<DlAlias>,
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
            home_memos: BTreeMap::new(),
            home_trees: BTreeSet::new(),
            host_policy: HostPolicy::open(),
            journal: None,
            dlaliases: Vec::new(),
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
    /// Under the record policy (spec 23 §8) ALLOWS are journaled too
    /// (`event=jail-allow`) — the "perm all and monitor" trail the
    /// `tfs needs` generator turns into a draft `needs:` block.
    pub fn host_check<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        need: HostAccess,
    ) -> Result<(), i32> {
        let path = path.as_ref();
        match self.host_policy.check(path, need) {
            Ok(()) => {
                if self.host_policy.is_record() {
                    if let Some(journal) = &self.journal {
                        crate::journal::journal_allow(
                            journal,
                            path,
                            need,
                            self.host_policy.source(),
                        );
                    }
                }
                Ok(())
            }
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

    /// Multi-mount API, union mode (spec 17 §1 / spec 03 §6): `mount`'s
    /// point is already taken — merge its backend over the incumbent's
    /// as a UNION composite (the new image shadows; members stay
    /// read-only). The incumbent keeps its handle; the union view
    /// replaces its backend in place, so established fds keep their
    /// owner. `Err(EINVAL)` on an empty point, `Err(ENODEV)` when the
    /// point is free — a union needs an incumbent (a lone image is a
    /// plain exclusive mount).
    pub fn mount_union(&mut self, mount: Mount) -> Result<i32, i32> {
        if mount.mount_point.is_empty() {
            return Err(libc::EINVAL);
        }
        let Some(handle) = self
            .mounts
            .values()
            .find(|m| m.mount_point == mount.mount_point)
            .map(|m| m.handle)
        else {
            return Err(libc::ENODEV);
        };
        let mut incumbent = self.mounts.remove(&handle).ok_or(libc::ENODEV)?;
        let union =
            crate::backends_union::UnionBackend::new(vec![incumbent.backend, mount.backend])?;
        incumbent.backend = Box::new(union);
        self.mounts.insert(handle, incumbent);
        Ok(handle)
    }

    /// The mount with `handle`, when it exists (boot-time introspection:
    /// the driver journals the union set it established).
    pub fn mount_by_handle(&self, handle: i32) -> Option<&Mount> {
        self.mounts.get(&handle)
    }

    /// Unmount a single mount by handle: force-close only its own fds and
    /// dir handles (they fail with EBADF afterwards), drop the mount, and
    /// release the mount point. Handles are never reused. The dlmap cache
    /// is flushed whole (entries carry no owner; serving an extraction of
    /// the removed mount's image afterwards would be a stale leak).
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
        self.dl_cache.clear();
        Ok(())
    }

    /// Unmount everything; all fds and dir handles become invalid. The
    /// dlmap cache dies with the mount table: its entries are a function
    /// of THESE mounts' images, and serving them against a later table
    /// would be a stale-extraction leak (the extracted FILES linger in
    /// the per-process tmpdir until the exit cleanup — the map is what
    /// must not outlive the mounts).
    pub fn unmount(&mut self) {
        self.mounts.clear();
        self.fd_table.clear();
        self.dir_table.clear();
        self.next_fd = 1;
        self.next_dir_id = 1;
        self.compat_handle = None;
        self.dl_cache.clear();
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

    /// The host-side tail of a memfs path under an extraction root (the
    /// dlmap/exec-cache layout `<root>/<tail>`). A drive-letter memfs
    /// root (msys: `A:/t`) makes the raw join operand drive-prefixed —
    /// `Path::join` REPLACES the root on such an operand and the
    /// extraction would target the nonexistent `A:` drive, failing EIO
    /// (the msys native-extension extraction failure class the runtime
    /// factory worked around C-side, `tfs_dlmap_extract`). A
    /// drive-letter first component flattens its colon so the tail is a
    /// RELATIVE host path. Every other shape is byte-stable: `:` is a
    /// legal POSIX name and the layout is pinned. For a flattened tail
    /// the dlmap-prefix redirect (`dlmap_tail`) names a memfs path the
    /// mounts do not hold, so it falls through to the host — the
    /// extracted file answers from the host, exactly the factory
    /// helper's contract.
    fn host_tail(path: &str) -> String {
        let tail = path.trim_start_matches('/');
        let mut parts = tail.splitn(2, '/');
        let first = parts.next().unwrap_or("");
        let rest = parts.next();
        let first = if first.len() == 2
            && first.ends_with(':')
            && first
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
        {
            format!("{}_", &first[..1])
        } else {
            first.to_string()
        };
        match rest {
            Some(rest) => format!("{first}/{rest}"),
            None => first,
        }
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
            let need = if (flags & O_ACCMODE) == libc::O_RDONLY {
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
            // A write naming a file the image does not hold: the held-tree
            // rule decides. An ancestor the image DOES hold means the write
            // lands in image territory — EROFS (never a host passthrough
            // with the wrong errno; on msys the host answer for a path on
            // a nonexistent drive is EBADF, which bundler's ProcessLock
            // does not tolerate — the 2026-08-01 boot-smoke class). Only
            // with NO held ancestor is the path a host path (spec 08):
            // the policy gates the consumer's fall-through exactly as if
            // no mount claimed it. With the app payload mounted at "/",
            // this is what keeps the host filesystem reachable.
            Err(e) if e == libc::ENOENT => {
                let accmode = flags & O_ACCMODE;
                if accmode != libc::O_RDONLY && self.path_is_held(path) {
                    // The spec 24 §5 write gate: EROFS, journaled.
                    self.journal_write_denial(path, &mount.mount_point);
                    return Err(libc::EROFS);
                }
                let need = if accmode == libc::O_RDONLY {
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
        // mode). A write open of an in-image file is a write into a held
        // tree by construction — the spec 24 §5 gate: EROFS, journaled.
        if (flags & O_ACCMODE) != libc::O_RDONLY {
            self.journal_write_denial(path, &mount.mount_point);
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
    //
    // The spec 24 §5 write gate: a denied write into a HELD tree is
    // journaled `event=vfs-deny op=write` (best-effort — only when a
    // journal rides the installed policy; the EROFS answer never
    // depends on it). Two denials journal: an RO mount's EROFS on a
    // held path, and a gated COW mount's out-of-area EROFS.
    // ---------------------------------------------------------------

    /// Journal one write-gate denial (spec 24 §5) — best-effort, silent
    /// when no journal is installed.
    fn journal_write_denial(&self, path: &str, mount_point: &str) {
        if let Some(journal) = &self.journal {
            crate::journal::journal_vfs_deny(
                journal,
                std::path::Path::new(path),
                HostAccess::Rw,
                mount_point,
                None,
            );
        }
    }

    /// The writable backend owning `path`, with its mount and the
    /// in-image path.
    fn writable_for(&self, path: &str) -> Result<(&Mount, &dyn WritableBackend, String), i32> {
        let path = &Self::normalize(path);
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        let mount = self.find_mount(path).ok_or(libc::ENOENT)?;
        if mount.mode == MountMode::ReadOnly {
            // The write gate (spec 24 §5): a write into a held tree is
            // EROFS and journaled. A covered-but-not-held path is host
            // territory — the same EROFS (the path write family does no
            // passthrough) but never a vfs-deny line.
            if self.path_is_held(path) {
                self.journal_write_denial(path, &mount.mount_point);
            }
            return Err(libc::EROFS);
        }
        let rel = Self::relative_path(mount, path).to_string();
        let w = mount.backend.writable().ok_or(libc::ENOTSUP)?;
        Ok((mount, w, rel))
    }

    /// Dispatch one write verb, journaling the gated COW mount's
    /// out-of-area EROFS (spec 24 §5 — on a COW mount EROFS comes only
    /// from the declared-area gate; the overlay's own host failures
    /// surface as their own errnos).
    fn write_gated<T>(&self, mount: &Mount, path: &str, result: Result<T, i32>) -> Result<T, i32> {
        match result {
            Err(e) if e == libc::EROFS && mount.mode == MountMode::Cow => {
                self.journal_write_denial(path, &mount.mount_point);
                Err(e)
            }
            r => r,
        }
    }

    /// Write `data` at `offset` in `path` (COW: copy-up into the overlay).
    pub fn pwrite_path(&self, path: &str, data: &[u8], offset: u64) -> Result<usize, i32> {
        let (mount, w, rel) = self.writable_for(path)?;
        self.write_gated(mount, path, w.pwrite(&rel, data, offset))
    }

    /// Truncate `path` to `len` bytes.
    pub fn truncate_path(&self, path: &str, len: u64) -> Result<(), i32> {
        let (mount, w, rel) = self.writable_for(path)?;
        self.write_gated(mount, path, w.truncate(&rel, len))
    }

    /// Create a single directory.
    pub fn mkdir_path(&self, path: &str, perms: u32) -> Result<(), i32> {
        let (mount, w, rel) = self.writable_for(path)?;
        self.write_gated(mount, path, w.mkdir(&rel, perms))
    }

    /// Remove a file, symlink or empty directory (COW: whiteouts the
    /// base entry).
    pub fn remove_path(&self, path: &str) -> Result<(), i32> {
        let (mount, w, rel) = self.writable_for(path)?;
        self.write_gated(mount, path, w.remove(&rel))
    }

    // ---------------------------------------------------------------
    // Utility
    // ---------------------------------------------------------------

    /// tebako_path_is_embedded.
    pub fn path_is_embedded(&self, path: &str) -> bool {
        self.find_mount(path).is_some()
    }

    /// tebako_fs_mount_of: the mount point of the longest-prefix mount
    /// covering `path` — the coverage answer behind `path_is_embedded`,
    /// in string form (spec 22 phase 1's mount-decision helper: an
    /// interposed load names the mount that owns the path). Same lexical
    /// discipline as the coverage check — no normalization; the caller's
    /// spelling is the dispatch input, so a Some here is exactly the
    /// `path_is_embedded` == true case.
    pub fn mount_point_of(&self, path: &str) -> Option<String> {
        self.find_mount(path).map(|m| m.mount_point.clone())
    }

    /// A mount HOLDS `path` — the write gate's discriminator. An entry
    /// existing at `path` in the image is held; so is a path whose
    /// deepest EXISTING in-image ancestor is held (a write into a held
    /// tree writes into the image's territory — EROFS, never a silent
    /// host passthrough with the wrong errno). A covered path with NO
    /// existing in-image ancestor is a host path (the spec 08
    /// passthrough, same as open/stat): with a `/` mount in play, this
    /// is what keeps host writes legal.
    pub fn path_is_held(&self, path: &str) -> bool {
        let path = &Self::normalize(path);
        let Some(mount) = self.find_mount(path) else {
            return false;
        };
        let mut relative = Self::relative_path(mount, path).to_string();
        loop {
            let held_here = mount.backend.has_entry_or_children(&relative);
            tebako_log::log!(
                tebako_log::Level::Trace,
                "tfs",
                "path_is_held path={path} probe={relative} held_here={held_here}"
            );
            if held_here {
                return true;
            }
            let Some((parent, _)) = relative.rsplit_once('/') else {
                return false;
            };
            relative = parent.to_string();
        }
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
    /// Returns the count of SKIPPED symlinks (no backend carries
    /// readlink today — skipped, counted, never silent).
    pub fn extract_all(&mut self, dest: &std::path::Path) -> Result<usize, i32> {
        if self.mounts.is_empty() {
            return Err(libc::ENODEV);
        }
        // The destination is a host-WRITE decision (spec 08): the policy
        // must grant it before any memfs content lands on the host.
        self.host_check(dest, HostAccess::Rw)?;
        if std::fs::create_dir_all(dest).is_err() {
            return Err(libc::EIO);
        }
        let mut skipped = 0usize;
        if self.mounts.len() == 1 {
            // Single mount: historic behavior — tree directly into dest.
            let mount = self.mounts.values().next().unwrap();
            extract_dir_recursive(mount.backend.as_ref(), "", dest, &mut skipped)?;
        } else {
            let mounts: Vec<&Mount> = self.mounts.values().collect();
            for mount in mounts {
                let subtree = dest.join(mount_point_basename(&mount.mount_point));
                if std::fs::create_dir_all(&subtree).is_err() {
                    return Err(libc::EIO);
                }
                extract_dir_recursive(mount.backend.as_ref(), "", &subtree, &mut skipped)?;
            }
        }
        Ok(skipped)
    }

    // ---------------------------------------------------------------
    // Library aliases (spec 22 §2.1, phase W2)
    // ---------------------------------------------------------------

    /// Install the boot's library-alias table: plain (declared bare
    /// name, boot-materialized absolute host path) pairs from the
    /// driver's class-L extraction pass — the channel's only writer
    /// (windows boots gate the call site; the channel itself is
    /// platform-neutral). The driver registers only manifest-validated
    /// declarations, so the pairs are trusted as-is — tfs stays
    /// tpkg-free. Replaces any previous table; the default empty table
    /// answers every name HOST.
    pub fn register_dlaliases(&mut self, pairs: Vec<(String, std::path::PathBuf)>) {
        self.dlaliases = pairs
            .into_iter()
            .map(|(name, host)| DlAlias { name, host })
            .collect();
    }

    /// tebako_fs_dlalias2file: the bare-name alias verdict at LOAD time
    /// — the covered surface's decision point (the patched msys `dln.c`
    /// calls this for a presented name; spec 22 §2.1, phase W2).
    ///
    /// - A BARE name (no path separator, no drive qualifier — the byte
    ///   grammar is this entry's own test, the same rule the manifest's
    ///   `library_aliases:` validation enforces) matching a registered
    ///   alias VERBATIM, case-insensitively (never extension-completed:
    ///   `foo` does not match `foo.dll`) answers the alias's
    ///   boot-materialized absolute host path — the `alias` verdict.
    /// - A path-carrying name never reaches the alias rule (the path
    ///   surface is Rule L1's): ENOENT, and NO verdict exists — nothing
    ///   is journaled (the spec journals bare-name verdicts only).
    /// - An undeclared bare name is HOST by default: ENOENT, the
    ///   `host` verdict — the consumer passes the presented name to the
    ///   OS loader untouched.
    /// - A registered alias whose materialized path VANISHED under the
    ///   process is the cache tampered with: EIO — deliberately NOT
    ///   ENOENT, so the C side raises the §5 verdict LoadError instead
    ///   of falling through to a host shadow (the verdict was `alias`;
    ///   the failure is post-verdict).
    ///
    /// The verdict line (`event=lib-load name=<n> verdict=host|alias`)
    /// is journaled where the verdict is MADE — here — under the RECORD
    /// policy only (spec 23 §8's discovery instrument; production stays
    /// silent), on the pre-opened audit file: a bare write(2), never a
    /// path operation under the lock (the journal module's fd
    /// discipline). The tamper failure's reporting channel is the §5
    /// LoadError, not the journal. The covered path's existence probe
    /// under the guard is safe: a non-empty table exists only on
    /// windows boots, where no syscall interposition can re-enter.
    pub fn dlalias2file(&self, name: &str) -> Result<std::ffi::CString, i32> {
        if name.bytes().any(|b| b == b'/' || b == b'\\' || b == b':') {
            // Not a bare name — the alias rule never engages; no
            // bare-name verdict exists, so nothing is journaled.
            return Err(libc::ENOENT);
        }
        let Some(alias) = self
            .dlaliases
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
        else {
            self.journal_lib_load(name, "host");
            return Err(libc::ENOENT);
        };
        self.journal_lib_load(name, "alias");
        if !alias.host.exists() {
            return Err(libc::EIO);
        }
        std::ffi::CString::new(alias.host.to_string_lossy().into_owned()).map_err(|_| libc::EIO)
    }

    /// One lib-load verdict line on the audit journal, under the record
    /// policy only — the helper keeps `dlalias2file` to one journal call
    /// site per verdict.
    fn journal_lib_load(&self, name: &str, verdict: &str) {
        if self.host_policy.is_record() {
            if let Some(journal) = &self.journal {
                crate::journal::journal_lib_load(journal, name, verdict);
            }
        }
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
    /// originals. A drive-letter memfs root (msys `A:/t`) flattens its
    /// colon in the tail (`host_tail`) so the host join stays relative —
    /// the redirect inverse degrades to host-serve there.
    pub fn dlmap2file(&mut self, path: &str) -> Result<std::ffi::CString, i32> {
        let path = &Self::normalize(path);
        // dlmap-prefix redirect (see open()): the dlmap spelling of a
        // memfs path materializes the original — stdio (`fopen`) and
        // dlopen consumers of loader-computed paths land here.
        let tail = Self::dlmap_tail(path);
        let effective = tail.as_deref().unwrap_or(path);
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!("[tfs] dlmap2file: {path} (effective {effective})");
        }
        let mut visited = std::collections::HashSet::new();
        let host = self.extract_for_exec(
            effective,
            effective,
            &ClosureDest::Dlcache,
            &[],
            &mut visited,
        )?;
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!("[tfs] dlmap2file: {effective} -> {}", host.display());
        }
        let s = host.to_string_lossy().into_owned();
        std::ffi::CString::new(s).map_err(|_| libc::EIO)
    }

    /// The per-process dl tmpdir, created and cleanup-registered on
    /// first use (the dlmap2file root). Exposed for the driver's PATH
    /// launchers (spec 22 §3.2): the self-injecting wrappers live under
    /// the same root, so the process-exit cleanup takes them with the
    /// extractions.
    pub fn ensure_dl_tmpdir(&mut self) -> Result<std::path::PathBuf, i32> {
        ensure_dl_tmpdir(&mut self.dl_tmpdir)
    }

    /// The exec surface's answer for a memfs path (the preload's
    /// execve/posix_spawn routing): a mount whose in-image manifest
    /// declares the home annotation (`identity.annotations.java_home` —
    /// the payload root IS a tool home, spec 03's free-form annotations)
    /// materializes WHOLE once per process and the answer is the host
    /// twin of `path` inside that tree. A home's data files
    /// (lib/modules, lib/jvm.cfg) never ride a linked-library closure,
    /// so the closure walk's answer boots a java that cannot find its
    /// boot class path (the metanorma dogfood's jing failure). Any
    /// other mount answers via dlmap2file's closure walk, unchanged.
    pub fn exec_materialize(&mut self, path: &str) -> Result<std::ffi::CString, i32> {
        let normalized = Self::normalize(path);
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!("[tfs] exec_materialize: path={path} normalized={normalized}");
        }
        let probe = self.find_mount(&normalized).map(|mount| {
            (
                mount.handle,
                Self::relative_path(mount, &normalized).to_string(),
            )
        });
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!("[tfs] exec_materialize: path={path} probe={probe:?}");
        }
        let Some((handle, rel)) = probe.filter(|(handle, _)| self.mount_is_home(*handle)) else {
            if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                eprintln!(
                    "[tfs] exec_materialize: path={path} -> dlmap2file fallback (no home mount)"
                );
            }
            return self.dlmap2file(path);
        };
        if rel.is_empty() {
            return Err(libc::EISDIR);
        }
        let root = self.home_tree_root(handle)?;
        if self.home_trees.insert(handle) {
            let mount = self.mounts.get(&handle).ok_or(libc::ENODEV)?;
            let mut skipped = 0usize;
            extract_dir_recursive(mount.backend.as_ref(), "", &root, &mut skipped)?;
        }
        let host = root.join(&rel);
        std::ffi::CString::new(host.to_string_lossy().into_owned()).map_err(|_| libc::EIO)
    }

    /// The mount's home-layout verdict, memoized per handle: the
    /// in-image manifest carries `identity.annotations.java_home` with
    /// a string value (the whole mount tree materializes — the shim's
    /// install-time reading of the same annotation, #388). The read is
    /// a tolerant value walk, never the validating model parse: a
    /// schema newer than this runtime must still answer (spec 03's
    /// compat rule — consumers ignore what they predate). An absent,
    /// unreadable, or malformed manifest answers false: the closure
    /// walk stays the default.
    fn mount_is_home(&mut self, handle: i32) -> bool {
        if let Some(verdict) = self.home_memos.get(&handle) {
            return *verdict;
        }
        let mount = self.mounts.get(&handle);
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!(
                "[tfs] mount_is_home: handle={} mount_present={}",
                handle,
                mount.is_some()
            );
        }
        let verdict = mount
            .and_then(|mount| {
                let text = read_backend_file(
                    mount.backend.as_ref(),
                    tpkg::PAYLOAD_MANIFEST_PATH.trim_start_matches('/'),
                );
                if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                    eprintln!(
                        "[tfs] mount_is_home: handle={} manifest_read={}",
                        handle,
                        text.is_some()
                    );
                    if let Some(ref t) = text {
                        eprintln!(
                            "[tfs] mount_is_home: handle={} manifest_len={} first100={:?}",
                            handle,
                            t.len(),
                            &t[..t.len().min(100)]
                        );
                    }
                }
                text
            })
            .and_then(|text| serde_yml::from_str::<serde_yml::Value>(&text).ok())
            .and_then(|yaml| {
                let result = yaml
                    .get("identity")?
                    .get("annotations")?
                    .get("java_home")?
                    .as_str()
                    .map(str::to_owned);
                if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                    eprintln!(
                        "[tfs] mount_is_home: handle={} java_home={:?}",
                        handle, result
                    );
                }
                result
            })
            .is_some();
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!("[tfs] mount_is_home: handle={} verdict={}", handle, verdict);
        }
        self.home_memos.insert(handle, verdict);
        verdict
    }

    /// The per-process whole-tree root for a home mount
    /// (`<dl tmpdir>/tebako-home-<handle>`), creating the dl tmpdir on
    /// first use — the same lifecycle as dlmap2file's cache (registered
    /// for cleanup at exit).
    fn home_tree_root(&mut self, handle: i32) -> Result<std::path::PathBuf, i32> {
        if self.dl_tmpdir.is_none() {
            let dir = create_dl_tmpdir().ok_or(libc::EIO)?;
            register_dl_cleanup(&dir);
            self.dl_tmpdir = Some(dir);
        }
        let tmp = self.dl_tmpdir.as_ref().ok_or(libc::EIO)?;
        Ok(tmp.join(format!("tebako-home-{handle}")))
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
    /// destination root, then walk its Mach-O/ELF/PE dependency closure
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
            ClosureDest::Dlcache => ensure_dl_tmpdir(&mut self.dl_tmpdir)?,
            ClosureDest::Store(root) => root.clone(),
        };

        let host_path = root.join(Self::host_tail(path));
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
        let Some(parsed) = (|| {
            use std::io::Read as _;
            let mut head = Vec::new();
            std::fs::File::open(&host_path)
                .and_then(|f| {
                    f.take(exec_closure::HEADER_WINDOW as u64)
                        .read_to_end(&mut head)
                })
                .ok()?;
            // Incident 13: a PE import directory is section-resident
            // (.rdata), past the header window in a multi-MiB module (a
            // -static-libstdc++ libsass.so) — a windowed parse silently
            // answers an empty closure and the OS load then 126s on the
            // vendored siblings that never materialized. A dlmap target
            // is a load module by construction, so PE images are parsed
            // WHOLE; the ELF/Mach-O window stands (their load tables
            // ride the headers).
            if head.starts_with(b"MZ") {
                if let Ok(full) = std::fs::read(&host_path) {
                    head = full;
                }
            }
            exec_closure::parse(&head)
        })() else {
            if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                eprintln!("[tfs] closure: {path} — header parse unsupported, no dep walk");
            }
            return Ok(host_path);
        };
        if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
            eprintln!(
                "[tfs] closure: {path} format={:?} deps={:?}",
                parsed.format, parsed.deps
            );
        }
        let referrer_dir = memfs_dirname(path);
        let exe_dir = memfs_dirname(exe);
        let mut chain: Vec<String> = chain_rpaths.to_vec();
        for rp in &parsed.rpaths {
            if !chain.contains(rp) {
                chain.push(rp.clone());
            }
        }
        for dep in &parsed.deps {
            let resolved = match parsed.format {
                exec_closure::ImageFormat::Pe => self.resolve_pe_dep(dep, &referrer_dir),
                exec_closure::ImageFormat::MachO | exec_closure::ImageFormat::Elf => {
                    self.resolve_dep(dep, &referrer_dir, &exe_dir, &parsed.rpaths, &chain)
                }
            };
            let Some(memfs) = resolved else {
                if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                    eprintln!(
                        "[tfs] closure dep: {dep} — not held at the importer's dir (host/system)"
                    );
                }
                continue;
            };
            if visited.contains(&memfs) {
                continue;
            }
            // The dep's own closure rides its extraction (same exe,
            // same destination).
            if let Err(e) = self.extract_for_exec(&memfs, exe, dest, &chain, visited) {
                // The OS load fails on its own with the loader's error;
                // the trace names the dep the walk could not serve.
                if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                    eprintln!("[tfs] closure dep: {dep} -> {memfs} — extraction failed errno={e}");
                }
            } else if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
                eprintln!("[tfs] closure dep: {dep} -> {memfs} materialized");
            }
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
            push(Self::normalize(&expand_loader_vars(
                name,
                referrer_dir,
                exe_dir,
            )));
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

    /// Resolve one PE import name (spec 22 §2.1 — no rpath exists on
    /// PE). API-set contracts (`api-ms-win-*` / `ext-ms-win-*`) are
    /// pseudo-modules the OS resolves internally — host surface by
    /// construction, skipped unconditionally. The runtime's own DLL
    /// (the PE name the handoff env names via `TEBAKO_RUNTIME_DLL`) is
    /// never materialized from a payload: the OS's basename-reuse rule
    /// binds the already-loaded copy, and a vendored copy would be a
    /// dead file written for no binding. A bare name resolves against
    /// the IMPORTING image's own in-image directory only (the $ORIGIN
    /// analogue — never a cross-mount basename probe); a
    /// separator-carrying name resolves verbatim when rooted,
    /// referrer-relative otherwise, normalized. A name the mounts do
    /// not hold at that candidate is a HOST import (None) — the OS
    /// loader answers for it exactly as before.
    fn resolve_pe_dep(&self, name: &str, referrer_dir: &str) -> Option<String> {
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("api-ms-win-") || lower.starts_with("ext-ms-win-") {
            return None;
        }
        if runtime_dll_name().as_deref() == Some(lower.as_str()) {
            return None;
        }
        // The windows loader's separator is '\\' as often as '/'; the
        // memfs tree spells only '/'.
        let name = &name.replace('\\', "/");
        let rooted = name.starts_with('/')
            || (name.len() >= 3
                && name.as_bytes()[0].is_ascii_alphabetic()
                && name.as_bytes()[1] == b':'
                && name.as_bytes()[2] == b'/');
        let candidate = if rooted {
            Self::normalize(name)
        } else {
            // Bare and relative names alike anchor at the importer's
            // own directory — the one and only PE candidate.
            Self::normalize(&format!("{referrer_dir}/{name}"))
        };
        self.held_file(&candidate).then_some(candidate)
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

/// The runtime's own windows DLL name (the PE name the factory owns —
/// spec 22 §2.1's closure-walk exclusion), named by the handoff env,
/// lowercased for the windows loader's case-insensitive comparison.
/// None when the leg never names one (every POSIX leg; windows legs
/// before the driver wires the flow) — no exclusion then.
fn runtime_dll_name() -> Option<String> {
    std::env::var("TEBAKO_RUNTIME_DLL")
        .ok()
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase())
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

/// The per-process dl tmpdir behind `Context::ensure_dl_tmpdir`, as a
/// field-disjoint free function — the exec walk holds an immutable
/// borrow of `self.mounts` while the tmpdir slot rotates.
fn ensure_dl_tmpdir(slot: &mut Option<std::path::PathBuf>) -> Result<std::path::PathBuf, i32> {
    match slot {
        Some(d) => Ok(d.clone()),
        None => {
            let d = create_dl_tmpdir().ok_or(libc::EIO)?;
            register_dl_cleanup(&d);
            *slot = Some(d.clone());
            Ok(d)
        }
    }
}

/// Create the per-process temporary directory for dlmap2file extractions
/// (mirrors the legacy C++ semantics: a unique subdirectory of the system
/// temp dir; a handful of attempts before giving up). spec 22 §6: when
/// the driver named `TEBAKO_EXEC_CACHE`, the leaf lands UNDER it — the
/// `tebako-dl-<hex>` marker keeps the dlmap-prefix redirect and the
/// exit cleanup untouched.
fn create_dl_tmpdir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("TEBAKO_EXEC_CACHE")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    // The named exec-cache root may not exist yet — the driver names it,
    // the first materialization creates it.
    std::fs::create_dir_all(&base).ok()?;
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
    skipped_symlinks: &mut usize,
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
            extract_dir_recursive(backend, &child_rel, &child_host, skipped_symlinks)?;
        } else if extract_file(backend, &child_rel, &child_host)? == ExtractStep::SkippedSymlink {
            *skipped_symlinks += 1;
        }
    }
    Ok(())
}

/// Read a small in-image file whole (the manifest probe): pread chunks
/// to EOF. Any backend error — the absent file included — answers None
/// (the probe's caller decides what absence means; it is never an exec
/// failure of its own).
fn read_backend_file(backend: &dyn Backend, rel: &str) -> Option<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut offset = 0u64;
    loop {
        match backend.pread(rel, &mut chunk, offset) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                offset += n as u64;
            }
            Err(_) => return None,
        }
    }
    String::from_utf8(buf).ok()
}

/// What one walk step did with a non-directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtractStep {
    /// File content written to the host.
    Written,
    /// A symlink the walk cannot materialize (no backend carries
    /// readlink today — the C ABI has no target surface). Skipped,
    /// counted, never silent: the CLI reports the count.
    SkippedSymlink,
}

/// Stream one file out of a backend onto the host (permissions and
/// modification time preserved, best effort).
fn extract_file(
    backend: &dyn Backend,
    rel: &str,
    host: &std::path::Path,
) -> Result<ExtractStep, i32> {
    use std::io::Write as _;
    #[cfg_attr(windows, allow(unused_variables))]
    let st = backend.stat(rel)?;
    if st.entry_type == EntryType::Symlink {
        return Ok(ExtractStep::SkippedSymlink);
    }
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
    Ok(ExtractStep::Written)
}

/// The process-global context. Public C API functions lock it for the
/// duration of each call (exactly like the C++ implementation's mutex).
pub fn context() -> &'static RwLock<FsContext> {
    static CONTEXT: RwLock<FsContext> = RwLock::new(FsContext::new());
    &CONTEXT
}

/// Test-only serialization for tests that touch the process-global
/// context (`context()` or the `tebako_fs_*` C API): hold the guard for
/// the test's whole body; acquiring resets the mount table so each
/// holder starts empty. It must be ONE lock crate-wide — a
/// module-private lock serializes a module only against itself, and a
/// global `unmount()` taken under it then deletes other modules' mounts
/// mid-test (backends_cow's C-ABI test lost `h_cow` to backends_union's
/// private LOCK exactly so — ubuntu --no-default-features, tebako run
/// 31718292665).
#[cfg(test)]
pub(crate) fn lock_global_context() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let g = LOCK.lock().unwrap();
    context().write().unwrap().unmount();
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// A zip holding ONLY "data/secret.txt" — no explicit "data/" entry:
    /// the zip backend's implied-parent case (production dwarfs images
    /// always carry dir entries; zips usually do not).
    fn fixture_zip(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("img.zip");
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("data/secret.txt", options).unwrap();
        writer.write_all(b"hush").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn path_is_held_walks_to_the_deepest_existing_in_image_ancestor() {
        let dir = std::env::temp_dir().join(format!("tfs-held-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let image = fixture_zip(&dir);
        let mut ctx = FsContext::new();
        let mount = crate::mount::build_from_file(image.to_str().unwrap(), "/tfs").unwrap();
        ctx.mount_checked(mount).unwrap();

        // an implied parent of real content is territory
        assert!(ctx.path_is_held("/tfs/data"));
        // a NEW path inside a held tree: the deepest existing ancestor
        // ("data", via its children) is in-image — EROFS, never a silent
        // host passthrough with the wrong errno
        assert!(ctx.path_is_held("/tfs/data/newdir"));
        assert!(ctx.path_is_held("/tfs/data/deeper/still-new"));
        // the mount root itself holds
        assert!(ctx.path_is_held("/tfs"));
        // a covered path with NO in-image ancestor stays a host path
        assert!(!ctx.path_is_held("/tfs/elsewhere/x"));
        // outside every mount
        assert!(!ctx.path_is_held("/elsewhere"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_gate_denials_journal_vfs_deny() {
        // Spec 24 §5: writes into held trees are EROFS and journaled
        // `event=vfs-deny op=write path=<p> mount=<mp>` — across the RO
        // mount, the write-open path, and the gated COW's out-of-area
        // gate; allowed and ungated writes journal nothing.
        let dir = tempfile::tempdir().unwrap();
        let image = fixture_zip(dir.path());
        let log = dir.path().join("journal.log");
        let journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();

        let mut ctx = FsContext::new();
        ctx.set_host_policy(HostPolicy::open(), Some(journal));

        // The RO mount: write opens and path-level writes on held paths.
        let ro = crate::mount::build_from_file(image.to_str().unwrap(), "/ro").unwrap();
        ctx.mount_checked(ro).unwrap();
        assert_eq!(
            ctx.open("/ro/data/secret.txt", libc::O_WRONLY).unwrap_err(),
            libc::EROFS
        );
        assert_eq!(
            ctx.open("/ro/data/new.txt", libc::O_WRONLY).unwrap_err(),
            libc::EROFS
        );
        assert_eq!(
            ctx.pwrite_path("/ro/data/secret.txt", b"x", 0).unwrap_err(),
            libc::EROFS
        );
        // A covered-but-not-held path-level write: the same EROFS (the
        // path write family does no passthrough) but NO vfs-deny line —
        // it is host territory, not the image's.
        assert_eq!(
            ctx.pwrite_path("/ro/elsewhere/x", b"x", 0).unwrap_err(),
            libc::EROFS
        );

        // The gated COW mount: in-area writes land (no line), out-of-area
        // writes are EROFS and journaled.
        let store = dir.path().join("store");
        let cow = crate::mount::build_from_file_with_mode(
            image.to_str().unwrap(),
            "/cow",
            crate::mount::MountMode::Cow,
            Some(&crate::mount::Overlay::gated(
                store.to_str().unwrap(),
                vec!["/data".to_string()],
            )),
        )
        .unwrap();
        ctx.mount_checked(cow).unwrap();
        ctx.pwrite_path("/cow/data/secret.txt", b"x", 0).unwrap();
        assert_eq!(
            ctx.pwrite_path("/cow/other.txt", b"x", 0).unwrap_err(),
            libc::EROFS
        );

        let text = std::fs::read_to_string(&log).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        // Two write-open denials + one RO path-write denial + one gated
        // out-of-area denial; the unheld and in-area writes journal none.
        assert_eq!(lines.len(), 4, "{text}");
        assert!(
            text.contains("event=vfs-deny op=write path=/ro/data/secret.txt mount=/ro"),
            "{text}"
        );
        assert!(
            text.contains("event=vfs-deny op=write path=/ro/data/new.txt mount=/ro"),
            "{text}"
        );
        assert!(
            text.contains("event=vfs-deny op=write path=/cow/other.txt mount=/cow"),
            "{text}"
        );
        assert!(!text.contains("elsewhere"), "{text}");
        assert!(!text.contains("/cow/data"), "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn extract_all_skips_symlinks_counted_and_lands_the_real_files() {
        // A hostdir mount carrying a symlink: the walk skips it, COUNTED
        // (no backend carries readlink — the C ABI has no target
        // surface), and the real files all land. 2026-08-11: the walk
        // died on exactly this with EINVAL in release builds (the
        // openjdk linux leg's mute failure).
        let dir = std::env::temp_dir().join(format!("tfs-symlink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("lib")).unwrap();
        std::fs::write(img.join("lib/real.txt"), b"data").unwrap();
        std::os::unix::fs::symlink("real.txt", img.join("lib/link.txt")).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        let backend = crate::backends_hostdir::HostDirBackend::new(&img).unwrap();
        let mount = Mount {
            handle: 0,
            mount_point: "/tfs".to_string(),
            mount_point_c: Box::new(std::ffi::CString::new("/tfs").unwrap()),
            archive_path: None,
            backend: Box::new(backend),
            mode: crate::mount::MountMode::ReadOnly,
        };
        ctx.mount_checked(mount).unwrap();
        let skipped = ctx.extract_all(&dest).unwrap();
        assert_eq!(skipped, 1, "the symlink is counted");
        assert!(dest.join("lib/real.txt").is_file(), "the real file lands");
        assert!(
            !dest.join("lib/link.txt").exists(),
            "the symlink is not materialized"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_policy_journals_allows_and_open_policy_does_not() {
        // spec 23 §8: under a record policy every ALLOWED host access is
        // journaled (event=jail-allow) — the `tfs needs` generator's input.
        // Under the default open policy allows stay silent (no noise).
        let dir = std::env::temp_dir().join(format!("tfs-record-ctx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("journal.log");
        let journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();

        let mut ctx = FsContext::new();
        ctx.set_host_policy(
            crate::policy::HostPolicy::bind(crate::policy::PolicyDefault::Record, vec![], vec![])
                .unwrap(),
            Some(journal),
        );
        assert_eq!(ctx.host_check("/etc/hosts", HostAccess::Ro), Ok(()));
        assert_eq!(ctx.host_check("/tmp/x", HostAccess::Rw), Ok(()));
        drop(ctx);

        let text = std::fs::read_to_string(&log).unwrap();
        let mut lines = text.lines();
        assert!(
            lines
                .next()
                .unwrap()
                .contains("event=jail-allow path=/etc/hosts op=read"),
            "{text}"
        );
        assert!(
            lines
                .next()
                .unwrap()
                .contains("event=jail-allow path=/tmp/x op=write"),
            "{text}"
        );
        assert!(lines.next().is_none(), "{text}");

        // The open policy journals no allows (today's behavior, no noise).
        let journal2 = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        let mut ctx2 = FsContext::new();
        ctx2.set_host_policy(crate::policy::HostPolicy::open(), Some(journal2));
        assert_eq!(ctx2.host_check("/etc/hosts", HostAccess::Ro), Ok(()));
        drop(ctx2);
        let text2 = std::fs::read_to_string(&log).unwrap();
        assert_eq!(
            text2.lines().count(),
            2,
            "open policy journals nothing: {text2}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A fresh context with one registered alias whose materialized host
    /// copy is a real temp file (the covered verdict re-probes the host
    /// path — the tamper case's discriminator).
    fn ctx_with_one_alias(dir: &std::path::Path) -> (FsContext, std::path::PathBuf) {
        std::fs::create_dir_all(dir).unwrap();
        let host = dir.join("libfoo-3.dll");
        std::fs::write(&host, b"pe").unwrap();
        let mut ctx = FsContext::new();
        ctx.register_dlaliases(vec![("libfoo-3.dll".to_string(), host.clone())]);
        (ctx, host)
    }

    #[test]
    fn dlalias2file_matches_verbatim_case_insensitively() {
        let dir = std::env::temp_dir().join(format!("tfs-dlalias-match-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (ctx, host) = ctx_with_one_alias(&dir);
        let want = host.to_string_lossy().into_owned();
        // The windows loader's own comparison: case-insensitive…
        assert_eq!(
            ctx.dlalias2file("LIBFOO-3.DLL").unwrap().to_str().unwrap(),
            want
        );
        assert_eq!(
            ctx.dlalias2file("LibFoo-3.Dll").unwrap().to_str().unwrap(),
            want
        );
        // …but verbatim: never extension-completed, never a basename
        // probe, never a prefix.
        assert_eq!(ctx.dlalias2file("libfoo-3"), Err(libc::ENOENT));
        assert_eq!(ctx.dlalias2file("libfoo-3.dll.dll"), Err(libc::ENOENT));
        // An undeclared bare name is host-by-default — the rule's whole
        // point.
        assert_eq!(ctx.dlalias2file("user32"), Err(libc::ENOENT));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dlalias2file_only_bare_names_reach_the_rule() {
        let dir = std::env::temp_dir().join(format!("tfs-dlalias-bare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (ctx, _host) = ctx_with_one_alias(&dir);
        // A separator or a drive qualifier makes the name path surface
        // (Rule L1) — never an alias, even when the basename matches.
        assert_eq!(
            ctx.dlalias2file("/vendor/lib/libfoo-3.dll"),
            Err(libc::ENOENT)
        );
        assert_eq!(ctx.dlalias2file("lib\\libfoo-3.dll"), Err(libc::ENOENT));
        assert_eq!(ctx.dlalias2file("C:\\lib\\libfoo-3.dll"), Err(libc::ENOENT));
        assert_eq!(ctx.dlalias2file("C:libfoo-3.dll"), Err(libc::ENOENT));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dlalias2file_empty_table_answers_everything_host() {
        // The default state (no driver registration — every POSIX boot):
        // every name is HOST, no probe, no journal.
        let ctx = FsContext::new();
        assert_eq!(ctx.dlalias2file("user32"), Err(libc::ENOENT));
        assert_eq!(ctx.dlalias2file("libfoo-3.dll"), Err(libc::ENOENT));
    }

    #[test]
    fn dlalias2file_vanished_materialization_is_eio_never_enoent() {
        // A registered alias whose materialized copy vanished under the
        // process is the cache tampered with: EIO — deliberately NOT
        // ENOENT, so the C side raises the §5 verdict LoadError instead
        // of falling through to a host shadow.
        let dir = std::env::temp_dir().join(format!("tfs-dlalias-eio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (ctx, host) = ctx_with_one_alias(&dir);
        std::fs::remove_file(&host).unwrap();
        assert_eq!(ctx.dlalias2file("libfoo-3.dll"), Err(libc::EIO));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dlalias2file_journals_verdicts_under_record_only() {
        // The lib-load verdict line is the record mode's discovery
        // instrument (spec 23 §8's idiom): under a record policy both
        // verdicts journal; under the default open policy nothing does
        // (production stays silent — the tamper case's reporting channel
        // is the §5 LoadError, not the journal).
        let dir = std::env::temp_dir().join(format!("tfs-dlalias-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("journal.log");
        let journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();

        let (mut ctx, host) = ctx_with_one_alias(&dir);
        ctx.set_host_policy(
            crate::policy::HostPolicy::bind(crate::policy::PolicyDefault::Record, vec![], vec![])
                .unwrap(),
            Some(journal),
        );
        let covered = ctx.dlalias2file("libfoo-3.dll").unwrap();
        assert_eq!(covered.to_str().unwrap(), host.to_string_lossy().as_ref());
        assert_eq!(ctx.dlalias2file("user32"), Err(libc::ENOENT));
        // A path-carrying name has no bare-name verdict at all — the
        // spec journals bare-name verdicts only.
        assert_eq!(
            ctx.dlalias2file("/vendor/lib/libfoo-3.dll"),
            Err(libc::ENOENT)
        );
        drop(ctx);
        let text = std::fs::read_to_string(&log).unwrap();
        let mut lines = text.lines();
        assert!(
            lines
                .next()
                .unwrap()
                .contains("event=lib-load name=libfoo-3.dll verdict=alias"),
            "{text}"
        );
        assert!(
            lines
                .next()
                .unwrap()
                .contains("event=lib-load name=user32 verdict=host"),
            "{text}"
        );
        assert!(lines.next().is_none(), "{text}");

        // The open policy journals no verdicts (the jail-allow precedent:
        // no noise outside record mode).
        let journal2 = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        let (mut ctx2, _host2) = ctx_with_one_alias(&dir);
        ctx2.set_host_policy(crate::policy::HostPolicy::open(), Some(journal2));
        assert!(ctx2.dlalias2file("libfoo-3.dll").is_ok());
        assert_eq!(ctx2.dlalias2file("user32"), Err(libc::ENOENT));
        drop(ctx2);
        let text2 = std::fs::read_to_string(&log).unwrap();
        assert_eq!(
            text2.lines().count(),
            2,
            "open policy journals nothing: {text2}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A zip with explicit dir entries (the zip backend's readdir is
    /// explicit-only) carrying a fake tool home: bin/tool + lib/modules,
    /// and the in-image manifest when `with_annotation` (the tolerant
    /// exec probe reads only identity.annotations.java_home).
    fn fixture_home_zip(dir: &std::path::Path, with_annotation: bool) -> std::path::PathBuf {
        let name = if with_annotation {
            "home.zip"
        } else {
            "plain.zip"
        };
        let path = dir.join(name);
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for d in ["__tpkg__/", "bin/", "lib/"] {
            writer.add_directory(d, options).unwrap();
        }
        if with_annotation {
            writer
                .start_file("__tpkg__/manifest.yaml", options)
                .unwrap();
            writer
                .write_all(b"identity:\n  annotations:\n    java_home: \"/\"\n")
                .unwrap();
        }
        writer.start_file("bin/tool", options).unwrap();
        writer.write_all(b"#!/bin/fake\n").unwrap();
        writer.start_file("lib/modules", options).unwrap();
        writer.write_all(b"jimage-bytes").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn exec_materialize_lands_the_whole_tree_for_a_home_layout_mount() {
        let dir = std::env::temp_dir().join(format!("tfs-home-exec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let image = fixture_home_zip(&dir, true);
        let mut ctx = FsContext::new();
        let mount = crate::mount::build_from_file(image.to_str().unwrap(), "/tfs").unwrap();
        ctx.mount_checked(mount).unwrap();

        let answer = ctx.exec_materialize("/tfs/bin/tool").unwrap();
        let host = std::path::PathBuf::from(answer.to_string_lossy().into_owned());
        assert!(host.is_file(), "the exec twin lands: {host:?}");
        assert_eq!(std::fs::read(&host).unwrap(), b"#!/bin/fake\n");
        assert!(
            host.to_string_lossy().contains("tebako-home-"),
            "home-layout answers from the whole-tree root, not the closure cache: {host:?}"
        );
        // The point of the branch: the home's data files exist next to
        // the binary (lib/modules — never in a linked closure).
        let root = host.parent().unwrap().parent().unwrap();
        assert_eq!(
            std::fs::read(root.join("lib/modules")).unwrap(),
            b"jimage-bytes"
        );

        // Idempotent within the process: a second exec reuses the tree.
        let again = ctx.exec_materialize("/tfs/bin/tool").unwrap();
        assert_eq!(again, answer);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_exec_cache_env_rebases_the_dl_tmpdir() {
        // spec 22 §6: when the driver named TEBAKO_EXEC_CACHE, the
        // closure walk's extractions live UNDER it — in the same
        // tebako-dl-<hex> per-process leaf (the dlmap-prefix redirect
        // and the exit cleanup are untouched).
        let base =
            std::env::temp_dir().join(format!("tebako-exec-cache-ut-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("TEBAKO_EXEC_CACHE", &base);
        let dir = create_dl_tmpdir();
        std::env::remove_var("TEBAKO_EXEC_CACHE");
        let dir = dir.expect("the tmpdir is created");
        assert!(
            dir.starts_with(&base),
            "{dir:?} is not under the named exec cache {base:?}"
        );
        assert!(
            dir.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("tebako-dl-"),
            "the per-process leaf keeps its marker: {dir:?}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn exec_materialize_keeps_the_closure_walk_for_a_plain_mount() {
        let dir = std::env::temp_dir().join(format!("tfs-plain-exec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let image = fixture_home_zip(&dir, false);
        let mut ctx = FsContext::new();
        let mount = crate::mount::build_from_file(image.to_str().unwrap(), "/tfs").unwrap();
        ctx.mount_checked(mount).unwrap();

        let answer = ctx.exec_materialize("/tfs/bin/tool").unwrap();
        let host = std::path::PathBuf::from(answer.to_string_lossy().into_owned());
        assert!(host.is_file(), "the closure-walk twin lands: {host:?}");
        assert!(
            !host.to_string_lossy().contains("tebako-home-"),
            "a manifest-less mount keeps the closure walk: {host:?}"
        );
        // No whole-tree: the home's data file never materializes.
        let root = host.parent().unwrap().parent().unwrap();
        assert!(!root.join("lib/modules").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------------------------------------------------------------
    // PE exec-closure walk (spec 22 §2.1)
    // ---------------------------------------------------------------

    /// A minimal PE32+ image with an import directory naming `imports`
    /// and a delay-load import directory naming `delay_imports` (the
    /// same byte construction as exec_closure's pe64_fixture — fixture
    /// builders stay local to their test module, like macho64_fixture).
    fn pe64_fixture(imports: &[&str], delay_imports: &[&str]) -> Vec<u8> {
        pe64_fixture_deep(0, imports, delay_imports)
    }

    /// The same fixture with `pad` zero bytes between the headers and
    /// the section body: the import directory's FILE offset lands at
    /// 0x200 + pad — a multi-MiB module's .rdata behind a big .text
    /// (incident 13's libsass.so).
    fn pe64_fixture_deep(pad: usize, imports: &[&str], delay_imports: &[&str]) -> Vec<u8> {
        const HEADERS: usize = 0x200;
        const SECTION_RVA: u32 = 0x1000;
        let raw_off = HEADERS + pad;
        let import_dir_size = (imports.len() + 1) * 20;
        // Section body: the import descriptors (+ all-zero terminator),
        // then the import name strings, then the delay-load descriptors
        // (+ terminator) and their name strings.
        let mut section = vec![0u8; import_dir_size];
        let mut import_name_rvas = Vec::new();
        for name in imports {
            import_name_rvas.push(SECTION_RVA + section.len() as u32);
            section.extend_from_slice(name.as_bytes());
            section.push(0);
        }
        let delay_base = section.len();
        let delay_dir_rva = SECTION_RVA + delay_base as u32;
        let mut delay_name_rvas = Vec::new();
        if !delay_imports.is_empty() {
            section.resize(section.len() + (delay_imports.len() + 1) * 20, 0);
            for name in delay_imports {
                delay_name_rvas.push(SECTION_RVA + section.len() as u32);
                section.extend_from_slice(name.as_bytes());
                section.push(0);
            }
        }
        let mut out = vec![0u8; HEADERS];
        out[0..2].copy_from_slice(b"MZ");
        out[0x3C..0x40].copy_from_slice(&0x80_u32.to_le_bytes()); // e_lfanew
        out[0x80..0x84].copy_from_slice(b"PE\0\0");
        let coff = 0x84;
        out[coff..coff + 2].copy_from_slice(&0x8664_u16.to_le_bytes()); // AMD64
        out[coff + 2..coff + 4].copy_from_slice(&1_u16.to_le_bytes()); // sections
        out[coff + 16..coff + 18].copy_from_slice(&240_u16.to_le_bytes()); // opt hdr
        let opt = coff + 20;
        out[opt..opt + 2].copy_from_slice(&0x20B_u16.to_le_bytes()); // PE32+
        out[opt + 60..opt + 64].copy_from_slice(&(HEADERS as u32).to_le_bytes());
        out[opt + 108..opt + 112].copy_from_slice(&16_u32.to_le_bytes()); // dirs count
        let dirs = opt + 112;
        out[dirs + 8..dirs + 12].copy_from_slice(&SECTION_RVA.to_le_bytes());
        out[dirs + 12..dirs + 16].copy_from_slice(&(import_dir_size as u32).to_le_bytes());
        // Delay-load import directory (index 13) — present, never read.
        if !delay_imports.is_empty() {
            let d = dirs + 13 * 8;
            out[d..d + 4].copy_from_slice(&delay_dir_rva.to_le_bytes());
            out[d + 4..d + 8]
                .copy_from_slice(&(((delay_imports.len() + 1) * 20) as u32).to_le_bytes());
        }
        // The one section header: .rdata, RVA 0x1000 → file raw_off.
        let sec = opt + 240;
        out[sec..sec + 6].copy_from_slice(b".rdata");
        out[sec + 8..sec + 12].copy_from_slice(&(section.len() as u32).to_le_bytes());
        out[sec + 12..sec + 16].copy_from_slice(&SECTION_RVA.to_le_bytes());
        out[sec + 16..sec + 20].copy_from_slice(&(section.len() as u32).to_le_bytes());
        out[sec + 20..sec + 24].copy_from_slice(&(raw_off as u32).to_le_bytes());
        out.resize(raw_off, 0);
        out.extend_from_slice(&section);
        for (i, rva) in import_name_rvas.iter().enumerate() {
            let at = raw_off + i * 20;
            out[at..at + 4].copy_from_slice(&1_u32.to_le_bytes()); // OriginalFirstThunk
            out[at + 12..at + 16].copy_from_slice(&rva.to_le_bytes()); // Name
            out[at + 16..at + 20].copy_from_slice(&1_u32.to_le_bytes()); // FirstThunk
        }
        for (i, rva) in delay_name_rvas.iter().enumerate() {
            let at = raw_off + delay_base + i * 20;
            out[at..at + 4].copy_from_slice(&1_u32.to_le_bytes());
            out[at + 12..at + 16].copy_from_slice(&rva.to_le_bytes());
            out[at + 16..at + 20].copy_from_slice(&1_u32.to_le_bytes());
        }
        out
    }

    /// Mount a host-dir fixture tree at `point` on a fresh context.
    fn mount_hostdir(ctx: &mut FsContext, dir: &std::path::Path, point: &str) {
        let backend = crate::backends_hostdir::HostDirBackend::new(dir).unwrap();
        let mount = Mount {
            handle: 0,
            mount_point: point.to_string(),
            mount_point_c: Box::new(std::ffi::CString::new(point).unwrap()),
            archive_path: None,
            backend: Box::new(backend),
            mode: crate::mount::MountMode::ReadOnly,
        };
        ctx.mount_checked(mount).unwrap();
    }

    #[test]
    fn pe_closure_walk_materializes_the_importer_dir_tree() {
        // spec 22 §2.1: bare imports resolve against the IMPORTING
        // image's own in-image directory (the $ORIGIN analogue);
        // separator-carrying names resolve referrer-relative, or
        // verbatim when rooted ('/' or '\', normalized); the closure
        // recurses with a visited set. Everything lands at its mirrored
        // path under the destination root.
        let dir = std::env::temp_dir().join(format!("tfs-pe-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("bin/sub")).unwrap();
        std::fs::write(img.join("rooted.dll"), pe64_fixture(&[], &[])).unwrap();
        std::fs::write(
            img.join("bin/tool.dll"),
            pe64_fixture(
                &[
                    "sibling.dll",
                    "sub/helper.dll",
                    "sub\\nested.dll",
                    "/tfs/rooted.dll",
                ],
                &[],
            ),
        )
        .unwrap();
        std::fs::write(
            img.join("bin/sibling.dll"),
            pe64_fixture(&["deeper.dll"], &[]),
        )
        .unwrap();
        std::fs::write(img.join("bin/deeper.dll"), pe64_fixture(&[], &[])).unwrap();
        std::fs::write(img.join("bin/sub/helper.dll"), pe64_fixture(&[], &[])).unwrap();
        std::fs::write(img.join("bin/sub/nested.dll"), pe64_fixture(&[], &[])).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img, "/tfs");
        let host = ctx
            .extract_exec_closure("/tfs/bin/tool.dll", &dest)
            .unwrap();

        assert_eq!(host, dest.join("tfs/bin/tool.dll"));
        for p in [
            "tfs/bin/sibling.dll",    // bare, the importer's own dir
            "tfs/bin/deeper.dll",     // transitive (sibling's bare import)
            "tfs/bin/sub/helper.dll", // referrer-relative
            "tfs/bin/sub/nested.dll", // '\' separator, referrer-relative
            "tfs/rooted.dll",         // rooted, verbatim
        ] {
            assert!(dest.join(p).is_file(), "the closure lands at {p}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_tail_flattens_a_drive_letter_root() {
        // The msys memfs root carries a drive letter (A:/t): the
        // extraction tail must be a RELATIVE host path, or Path::join
        // replaces the cache root and the write targets the nonexistent
        // drive (EIO). Only the drive-letter first component flattens.
        assert_eq!(
            FsContext::host_tail("A:/t/lib/ruby/x.so"),
            "A_/t/lib/ruby/x.so"
        );
        assert_eq!(FsContext::host_tail("/A:/t/lib/x.so"), "A_/t/lib/x.so");
        assert_eq!(FsContext::host_tail("A:"), "A_");
        // Every other shape is byte-stable — ':' is a legal POSIX name
        // and the extraction layout is pinned.
        assert_eq!(
            FsContext::host_tail("/__tfs__/lib/x.so"),
            "__tfs__/lib/x.so"
        );
        assert_eq!(
            FsContext::host_tail("/__tfs__/lib/we:ird.so"),
            "__tfs__/lib/we:ird.so"
        );
        assert_eq!(FsContext::host_tail("foo:/bar"), "foo:/bar");
    }

    #[test]
    fn dlmap2file_of_a_drive_letter_mount_stays_under_the_dl_root() {
        // The msys shape end to end: a mount at A:/t, a dlmap of an
        // in-image library — the materialized copy lands UNDER the
        // per-process dl tmpdir (never on an A: drive) and exists.
        let dir = std::env::temp_dir().join(format!("tfs-dlmap-drive-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("lib")).unwrap();
        std::fs::write(img.join("lib/x.so"), pe64_fixture(&[], &[])).unwrap();

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img, "A:/t");
        let host = ctx.dlmap2file("A:/t/lib/x.so").unwrap();
        let host = host.to_string_lossy().into_owned();

        assert!(host.contains("tebako-dl-"), "under the dl root: {host}");
        assert!(!host.starts_with("A:"), "never the drive itself: {host}");
        assert!(
            std::path::Path::new(&host).is_file(),
            "the materialized copy exists: {host}"
        );
        assert!(
            host.ends_with("A_/t/lib/x.so"),
            "the flattened tail mirrors the memfs path: {host}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pe_closure_with_a_deep_import_directory_still_walks() {
        // Incident 13: the import directory past the 1-MiB header
        // window (a multi-MiB module's .rdata — a -static-libstdc++
        // libsass.so) must still walk. A windowed parse silently
        // answers no imports, the importer materializes ALONE, and the
        // OS load then misses the vendored siblings (the msys 126).
        let dir = std::env::temp_dir().join(format!("tfs-pe-deep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("bin")).unwrap();
        std::fs::write(
            img.join("bin/tool.dll"),
            pe64_fixture_deep(exec_closure::HEADER_WINDOW + 0x400, &["sibling.dll"], &[]),
        )
        .unwrap();
        std::fs::write(img.join("bin/sibling.dll"), pe64_fixture(&[], &[])).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img, "/tfs");
        let host = ctx
            .extract_exec_closure("/tfs/bin/tool.dll", &dest)
            .unwrap();

        assert_eq!(host, dest.join("tfs/bin/tool.dll"));
        assert!(
            dest.join("tfs/bin/sibling.dll").is_file(),
            "the deep import table's sibling materializes beside the importer"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dlmap2file_drive_root_closure_materializes_siblings() {
        // Incident 13 round 4 repro — the msys dogfood's exact shape:
        // the payload mounted at the DRIVE ROOT (A:/), the importer
        // dlopen'd by full drive-spelled path, the import directory
        // past the 1-MiB header window, the vendored ucrt sibling
        // beside it in-image. The closure walk must materialize the
        // sibling next to the importer or the OS load 126s.
        let dir = std::env::temp_dir().join(format!("tfs-dlmap-deep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let sassc = dir.join("img/probe/gemhome/gems/sassc-2.4.0/lib/sassc");
        std::fs::create_dir_all(&sassc).unwrap();
        std::fs::write(
            sassc.join("libsass.so"),
            pe64_fixture_deep(
                exec_closure::HEADER_WINDOW + 0x400,
                &["libgcc_s_seh-1.dll"],
                &[],
            ),
        )
        .unwrap();
        std::fs::write(sassc.join("libgcc_s_seh-1.dll"), pe64_fixture(&[], &[])).unwrap();

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &dir.join("img"), "A:/");
        let host = ctx
            .dlmap2file("A:/probe/gemhome/gems/sassc-2.4.0/lib/sassc/libsass.so")
            .unwrap();
        let host = host.to_string_lossy().into_owned();
        let sibling = std::path::Path::new(&host).with_file_name("libgcc_s_seh-1.dll");
        assert!(
            sibling.is_file(),
            "the vendored sibling materializes beside the importer: {sibling:?} (importer at {host})"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pe_closure_bare_import_never_probes_beyond_the_importers_dir() {
        // spec 22 §2.1: importer-dir-only — a bare import the mounts do
        // not hold AT the importer's own directory is a HOST import (the
        // OS loader answers for it). Never a sibling-directory guess,
        // never a cross-mount basename probe: /tfs-b/bin/lonely.dll
        // (the same in-image relative dir in a SECOND image) must not
        // serve /tfs-a/bin/tool.dll's import.
        let dir = std::env::temp_dir().join(format!("tfs-pe-neg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img_a = dir.join("img-a");
        let img_b = dir.join("img-b");
        std::fs::create_dir_all(img_a.join("bin")).unwrap();
        std::fs::create_dir_all(img_a.join("lib")).unwrap();
        std::fs::create_dir_all(img_b.join("bin")).unwrap();
        std::fs::write(
            img_a.join("bin/tool.dll"),
            pe64_fixture(&["lonely.dll"], &[]),
        )
        .unwrap();
        // The wrong directory of the SAME image…
        std::fs::write(img_a.join("lib/lonely.dll"), pe64_fixture(&[], &[])).unwrap();
        // …and the SAME relative directory of another image.
        std::fs::write(img_b.join("bin/lonely.dll"), pe64_fixture(&[], &[])).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img_a, "/tfs-a");
        mount_hostdir(&mut ctx, &img_b, "/tfs-b");
        let host = ctx
            .extract_exec_closure("/tfs-a/bin/tool.dll", &dest)
            .unwrap();

        assert!(host.is_file());
        assert!(
            !dest.join("tfs-a/bin/lonely.dll").exists(),
            "the bare import is not satisfied from the other image's bin/"
        );
        assert!(
            !dest.join("tfs-a/lib/lonely.dll").exists(),
            "the bare import is not satisfied from a sibling directory"
        );
        assert!(!dest.join("tfs-b/bin/lonely.dll").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pe_closure_skips_api_set_contracts_even_when_held() {
        // spec 22 §2.1: api-ms-win-* / ext-ms-win-* pseudo-modules are
        // host surface by construction — skipped unconditionally, even
        // when the image HOLDS a file by that name next to the importer
        // (a vendored trap file must never materialize). The comparison
        // is case-insensitive, like the windows loader's own.
        let dir = std::env::temp_dir().join(format!("tfs-pe-apiset-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("bin")).unwrap();
        std::fs::write(
            img.join("bin/tool.dll"),
            pe64_fixture(
                &[
                    "api-ms-win-core-file-l1-1-0.dll",
                    "EXT-MS-WIN-NTUSER-STRING-L1-1-0.DLL",
                    "real.dll",
                ],
                &[],
            ),
        )
        .unwrap();
        std::fs::write(
            img.join("bin/api-ms-win-core-file-l1-1-0.dll"),
            pe64_fixture(&[], &[]),
        )
        .unwrap();
        std::fs::write(
            img.join("bin/EXT-MS-WIN-NTUSER-STRING-L1-1-0.DLL"),
            pe64_fixture(&[], &[]),
        )
        .unwrap();
        std::fs::write(img.join("bin/real.dll"), pe64_fixture(&[], &[])).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img, "/tfs");
        ctx.extract_exec_closure("/tfs/bin/tool.dll", &dest)
            .unwrap();

        assert!(dest.join("tfs/bin/real.dll").is_file());
        assert!(!dest
            .join("tfs/bin/api-ms-win-core-file-l1-1-0.dll")
            .exists());
        assert!(!dest
            .join("tfs/bin/EXT-MS-WIN-NTUSER-STRING-L1-1-0.DLL")
            .exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pe_closure_ignores_delay_load_imports() {
        // spec 22 §2.1: delay-load is out of phase W — the delay
        // directory is never read, so a planted delayed.dll never
        // materializes (an honest OS failure at load, never a guess).
        let dir = std::env::temp_dir().join(format!("tfs-pe-delay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("bin")).unwrap();
        std::fs::write(
            img.join("bin/tool.dll"),
            pe64_fixture(&[], &["delayed.dll"]),
        )
        .unwrap();
        std::fs::write(img.join("bin/delayed.dll"), pe64_fixture(&[], &[])).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img, "/tfs");
        ctx.extract_exec_closure("/tfs/bin/tool.dll", &dest)
            .unwrap();

        assert!(dest.join("tfs/bin/tool.dll").is_file());
        assert!(!dest.join("tfs/bin/delayed.dll").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pe_closure_never_materializes_the_runtime_dll() {
        // spec 22 §2.1: the runtime's own ruby DLL (the PE name the
        // handoff env names via TEBAKO_RUNTIME_DLL) is never
        // materialized from a payload — the OS's basename-reuse rule
        // binds the already-loaded copy, so a vendored copy would be a
        // dead file written for no binding. Case-insensitive, like the
        // windows loader's own comparison.
        std::env::set_var("TEBAKO_RUNTIME_DLL", "x64-ucrt-ruby999.dll");
        let dir = std::env::temp_dir().join(format!("tfs-pe-rubydll-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("bin")).unwrap();
        std::fs::write(
            img.join("bin/tool.dll"),
            pe64_fixture(
                &["x64-ucrt-ruby999.dll", "X64-UCRT-RUBY999.DLL", "real.dll"],
                &[],
            ),
        )
        .unwrap();
        std::fs::write(img.join("bin/x64-ucrt-ruby999.dll"), pe64_fixture(&[], &[])).unwrap();
        std::fs::write(img.join("bin/real.dll"), pe64_fixture(&[], &[])).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img, "/tfs");
        ctx.extract_exec_closure("/tfs/bin/tool.dll", &dest)
            .unwrap();
        std::env::remove_var("TEBAKO_RUNTIME_DLL");

        assert!(dest.join("tfs/bin/real.dll").is_file());
        assert!(
            !dest.join("tfs/bin/x64-ucrt-ruby999.dll").exists(),
            "the runtime's own DLL stays off disk even when a payload vendors it"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pe_closure_treats_a_malformed_image_as_dependency_free() {
        // The named answer for a malformed PE is no-dependencies (the
        // parse answers None): the image itself still materializes and
        // the OS loader answers for its imports — never a panic, never
        // a parse error surfaced on the extraction path.
        let dir = std::env::temp_dir().join(format!("tfs-pe-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = dir.join("img");
        std::fs::create_dir_all(img.join("bin")).unwrap();
        let mut bad = pe64_fixture(&["sibling.dll"], &[]);
        bad[0x80..0x84].copy_from_slice(b"PX\0\0"); // a broken PE signature
        std::fs::write(img.join("bin/tool.dll"), &bad).unwrap();
        std::fs::write(img.join("bin/sibling.dll"), pe64_fixture(&[], &[])).unwrap();
        let dest = dir.join("out");

        let mut ctx = FsContext::new();
        mount_hostdir(&mut ctx, &img, "/tfs");
        let host = ctx
            .extract_exec_closure("/tfs/bin/tool.dll", &dest)
            .unwrap();

        assert!(host.is_file(), "the malformed image itself still lands");
        assert!(
            !dest.join("tfs/bin/sibling.dll").exists(),
            "no dependencies are read from a malformed image"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
