//! COW: the composite backend (spec 11 §4 — the transforms law: write
//! support exists ONLY here in the Rust TFS; format backends stay
//! read-only forever).
//!
//! [`CowBackend`] stacks a writable overlay over a read-only base:
//! - **reads** fall through to the base unless the overlay shadows the
//!   path (an overlay entry always wins over a base entry of the same
//!   name, directory listings merge overlay + base minus whiteouts)
//! - **writes/deletes/attr changes** land in the overlay only; modifying
//!   a base file copies it up first (the base image stays byte-identical)
//! - **whiteouts** hide base entries: a small journal records deleted
//!   paths; a whiteout on a directory hides its whole subtree. Whiteouts
//!   mask the BASE only — an overlay entry of the same name always wins
//!   (upper-replaces-whiteout, exactly like overlayfs), so recreating a
//!   deleted path needs no journal surgery and the delete record stands.
//!   The journal is the complete delete-side change record (the
//!   create/modify side IS the overlay directory tree).
//!
//! The overlay is a [`HostDirBackend`] — a plain host directory, so the
//! composite is disposable by deleting the directory, and the journal
//! lives inside it (`.tfs-whiteouts`) keeping the overlay self-contained.
//! The journal file itself is hidden from the merged view.
//!
//! ## Journal format (v1)
//!
//! ```text
//! TFS-WHITEOUTS 1\n
//! W <escaped-path>\n      one per whiteout, sorted, deduplicated
//! ```
//!
//! Escaping: `%` → `%25`, `\n` → `%0A`, `\r` → `%0D`. Parsing is strict:
//! wrong magic, unknown record tags and bad escapes are EINVAL (a lost
//! whiteout exposes deleted base content — never tolerated silently).
//! Rewrites are atomic (temp file + rename).

use std::collections::BTreeSet;
use std::ffi::CStr;
use std::io;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat, WritableBackend};
use crate::backends_hostdir::{io_errno, HostDirBackend};

/// The whiteout journal file, at the overlay root (hidden from the
/// merged view).
pub const JOURNAL_FILE: &str = ".tfs-whiteouts";
const JOURNAL_MAGIC: &str = "TFS-WHITEOUTS 1";

// ===================================================================
// Journal (de)serialization
// ===================================================================

fn escape(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            '%' => out.push_str("%25"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(text: &str) -> Result<String, i32> {
    if !text.contains('%') {
        return Ok(text.to_string());
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let h1 = chars.next().ok_or(libc::EINVAL)?;
        let h2 = chars.next().ok_or(libc::EINVAL)?;
        let hex = [h1, h2].iter().collect::<String>();
        let byte = u8::from_str_radix(&hex, 16).map_err(|_| libc::EINVAL)?;
        out.push(byte as char);
    }
    Ok(out)
}

/// Serialize the whiteout set (sorted, deduplicated by BTreeSet).
pub fn serialize_whiteouts(set: &BTreeSet<String>) -> String {
    let mut out = String::from(JOURNAL_MAGIC);
    out.push('\n');
    for path in set {
        out.push_str("W ");
        out.push_str(&escape(path));
        out.push('\n');
    }
    out
}

/// Parse a journal body (strict: EINVAL on any malformed line).
pub fn parse_whiteouts(text: &str) -> Result<BTreeSet<String>, i32> {
    let mut lines = text.lines();
    if lines.next() != Some(JOURNAL_MAGIC) {
        return Err(libc::EINVAL);
    }
    let mut set = BTreeSet::new();
    for line in lines {
        let rest = line.strip_prefix("W ").ok_or(libc::EINVAL)?;
        let path = unescape(rest)?;
        if path.is_empty() || path.starts_with('/') || path.ends_with('/') || path.contains('\0') {
            return Err(libc::EINVAL);
        }
        set.insert(path);
    }
    Ok(set)
}

/// Load the journal from disk (missing file → empty set).
fn load_journal(path: &PathBuf) -> Result<BTreeSet<String>, i32> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse_whiteouts(&text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(e) => Err(io_errno(&e)),
    }
}

/// Atomically rewrite the journal (temp file + rename).
fn store_journal(path: &PathBuf, set: &BTreeSet<String>) -> Result<(), i32> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serialize_whiteouts(set)).map_err(|e| io_errno(&e))?;
    std::fs::rename(&tmp, path).map_err(|e| io_errno(&e))
}

// ===================================================================
// The composite backend
// ===================================================================

/// `CowBackend { base, overlay }` — stacking, not a format (spec 11 §4).
pub struct CowBackend {
    base: Box<dyn Backend>,
    overlay: HostDirBackend,
    /// Hidden base paths (the whiteout set; the journal on disk is the
    /// persistent form, rewritten atomically on every change).
    whiteouts: RwLock<BTreeSet<String>>,
    journal_path: PathBuf,
}

impl CowBackend {
    /// Stack `overlay` over `base`, loading (or creating) the whiteout
    /// journal inside the overlay directory.
    pub fn new(base: Box<dyn Backend>, overlay: HostDirBackend) -> Result<CowBackend, i32> {
        let journal_path = overlay.root().join(JOURNAL_FILE);
        let whiteouts = load_journal(&journal_path)?;
        if whiteouts.is_empty() {
            // Eagerly create the journal: the change record exists from
            // the first mount even before any delete.
            store_journal(&journal_path, &whiteouts)?;
        }
        Ok(CowBackend {
            base,
            overlay,
            whiteouts: RwLock::new(whiteouts),
            journal_path,
        })
    }

    /// The read-only base backend.
    pub fn base(&self) -> &dyn Backend {
        self.base.as_ref()
    }

    /// The overlay backend.
    pub fn overlay(&self) -> &HostDirBackend {
        &self.overlay
    }

    /// The current whiteout set (snapshot).
    pub fn whiteouts(&self) -> BTreeSet<String> {
        self.whiteouts.read().unwrap().clone()
    }

    /// True when `path` is hidden by a whiteout (a whiteout on a
    /// directory hides its whole subtree).
    fn is_hidden(&self, path: &str) -> bool {
        let set = self.whiteouts.read().unwrap();
        let mut p = path;
        loop {
            if set.contains(p) {
                return true;
            }
            match p.rfind('/') {
                Some(i) => p = &p[..i],
                None => return false,
            }
        }
    }

    fn add_whiteout(&self, path: &str) -> Result<(), i32> {
        let mut set = self.whiteouts.write().unwrap();
        if set.insert(path.to_string()) {
            store_journal(&self.journal_path, &set)?;
        }
        Ok(())
    }

    /// Merged-view stat without the journal-name filter (internal).
    /// The overlay always shadows — including over a whiteout (an upper
    /// entry replaces the delete marker, exactly like overlayfs);
    /// whiteouts hide BASE entries only.
    fn stat_merged(&self, path: &str) -> Result<RawStat, i32> {
        if path.is_empty() {
            return Ok(RawStat {
                entry_type: EntryType::Directory,
                perms: 0o755,
                size: 0,
                mtime: 0,
            });
        }
        match self.overlay.stat(path) {
            Ok(st) => Ok(st),
            Err(libc::ENOENT) => {
                if self.is_hidden(path) {
                    Err(libc::ENOENT)
                } else {
                    self.base.stat(path)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Materialize the parent directories of `path` into the overlay
    /// (overlay-style writes: parents appear on demand). A merged parent
    /// that exists and is not a directory is ENOTDIR.
    fn ensure_overlay_parent(&self, path: &str) -> Result<(), i32> {
        let Some(i) = path.rfind('/') else {
            return Ok(()); // root-level entry
        };
        let parent = &path[..i];
        match self.stat_merged(parent) {
            Ok(st) if st.entry_type != EntryType::Directory => Err(libc::ENOTDIR),
            Ok(_) | Err(libc::ENOENT) => self.overlay.mkdir_parents(parent),
            Err(e) => Err(e),
        }
    }

    /// POSIX mkdir's parent rule: the merged parent must EXIST as a
    /// directory (ENOENT/ENOTDIR otherwise); it is then materialized into
    /// the overlay so the single-level mkdir succeeds.
    fn require_merged_parent(&self, path: &str) -> Result<(), i32> {
        let Some(i) = path.rfind('/') else {
            return Ok(());
        };
        let parent = &path[..i];
        match self.stat_merged(parent) {
            Ok(st) if st.entry_type == EntryType::Directory => self.overlay.mkdir_parents(parent),
            Ok(_) => Err(libc::ENOTDIR),
            Err(e) => Err(e),
        }
    }

    /// Copy a base entry into the overlay so a write can land (overlayfs
    /// copy-up). New files only materialize their parents; directories
    /// materialize shallowly (children copy up on demand).
    fn copy_up(&self, path: &str) -> Result<(), i32> {
        if self.overlay.stat(path).is_ok() {
            return Ok(());
        }
        match self.base.stat(path) {
            Ok(st) => {
                self.ensure_overlay_parent(path)?;
                match st.entry_type {
                    EntryType::File => {
                        // Stream the base content into a fresh overlay file.
                        self.overlay.pwrite(path, &[], 0)?;
                        let mut off = 0u64;
                        let mut buf = vec![0u8; 8192];
                        loop {
                            let n = self.base.pread(path, &mut buf, off)?;
                            if n == 0 {
                                break;
                            }
                            let mut w = 0usize;
                            while w < n {
                                w += self.overlay.pwrite(path, &buf[w..n], off + w as u64)?;
                            }
                            off += n as u64;
                        }
                        #[cfg(unix)]
                        self.overlay.set_perms(path, st.perms);
                        Ok(())
                    }
                    EntryType::Directory => self.overlay.mkdir_parents(path),
                    // Symlinks/special files: no copyable content.
                    _ => Err(libc::EINVAL),
                }
            }
            Err(libc::ENOENT) => {
                // New file: only the parents need to exist in the overlay.
                self.ensure_overlay_parent(path)
            }
            Err(e) => Err(e),
        }
    }
}

/// Normalize an in-image path: no leading or trailing `/`, `""` for root.
fn normalize(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

/// Join a child name onto a directory path (`""` root aware).
fn join_path(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

impl Backend for CowBackend {
    fn name(&self) -> &'static CStr {
        c"COW"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let path = normalize(path);
        if path == JOURNAL_FILE {
            return Err(libc::ENOENT); // the journal is not VFS content
        }
        self.stat_merged(path)
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        if path == JOURNAL_FILE {
            return Err(libc::ENOENT);
        }
        // Overlay first; a whiteout hides only the base entry.
        match self.overlay.pread(path, buf, offset) {
            Err(libc::ENOENT) => {
                if self.is_hidden(path) {
                    Err(libc::ENOENT)
                } else {
                    self.base.pread(path, buf, offset)
                }
            }
            r => r,
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        let path = normalize(path);
        if path == JOURNAL_FILE {
            return Err(libc::ENOTDIR);
        }
        let overlay_entries = self.overlay.read_dir(path);
        let base_entries = self.base.read_dir(path);
        let mut out: Vec<RawDirEntry> = match overlay_entries {
            Ok(entries) => entries,
            // The overlay has no such directory: the base answers alone,
            // unless a whiteout hides the path.
            Err(libc::ENOENT) => {
                if self.is_hidden(path) {
                    return Err(libc::ENOENT);
                }
                return match base_entries {
                    Ok(entries) => Ok(entries
                        .into_iter()
                        .filter(|e| !self.is_hidden(&join_path(path, &e.name)))
                        .collect()),
                    Err(e) => Err(e),
                };
            }
            // The overlay's answer is definitive (ENOTDIR for a shadowing
            // file, host errors).
            Err(e) => return Err(e),
        };
        if path.is_empty() {
            out.retain(|e| e.name != JOURNAL_FILE);
        }
        match base_entries {
            Ok(entries) => {
                for e in entries {
                    if out.iter().any(|o| o.name == e.name) {
                        continue; // overlay shadows the base entry
                    }
                    if self.is_hidden(&join_path(path, &e.name)) {
                        continue; // whiteout hides the base entry
                    }
                    out.push(e);
                }
                Ok(out)
            }
            // Base has nothing here, or an overlay directory shadows a
            // base file: the overlay listing is the merged view.
            Err(libc::ENOENT) | Err(libc::ENOTDIR) => Ok(out),
            Err(e) => Err(e),
        }
    }

    fn read_link(&self, path: &str) -> Result<String, i32> {
        let path = normalize(path);
        // Same shadowing rule as reads: the overlay wins, then the base.
        match self.overlay.read_link(path) {
            Err(libc::ENOENT) => {
                if self.is_hidden(path) {
                    Err(libc::ENOENT)
                } else {
                    self.base.read_link(path)
                }
            }
            r => r,
        }
    }

    fn writable(&self) -> Option<&dyn WritableBackend> {
        Some(self)
    }
}

impl WritableBackend for CowBackend {
    fn pwrite(&self, path: &str, data: &[u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        if path.is_empty() {
            return Err(libc::EISDIR);
        }
        if path == JOURNAL_FILE {
            return Err(libc::EPERM); // the journal is the audit delta, not content
        }
        // A whiteouted path recreates FRESH (no base copy-up): the delete
        // stands in the journal; the new overlay entry takes over the name.
        if !self.whiteouts.read().unwrap().contains(path) {
            self.copy_up(path)?;
        } else {
            self.ensure_overlay_parent(path)?;
        }
        self.overlay.pwrite(path, data, offset)
    }

    fn truncate(&self, path: &str, len: u64) -> Result<(), i32> {
        let path = normalize(path);
        if path.is_empty() || path == JOURNAL_FILE {
            return Err(libc::EINVAL);
        }
        // truncate requires an existing file (merged view), then lands in
        // the overlay (copy-up first for base files).
        if self.stat_merged(path)?.entry_type != EntryType::File {
            return Err(libc::EINVAL);
        }
        self.copy_up(path)?;
        self.overlay.truncate(path, len)
    }

    fn mkdir(&self, path: &str, perms: u32) -> Result<(), i32> {
        let path = normalize(path);
        if path.is_empty() || path == JOURNAL_FILE {
            return Err(libc::EEXIST);
        }
        match self.stat_merged(path) {
            Ok(_) => return Err(libc::EEXIST),
            Err(libc::ENOENT) => {}
            Err(e) => return Err(e),
        }
        self.require_merged_parent(path)?;
        self.overlay.mkdir(path, perms)
    }

    fn remove(&self, path: &str) -> Result<(), i32> {
        let path = normalize(path);
        if path.is_empty() || path == JOURNAL_FILE {
            return Err(libc::EINVAL);
        }
        // The merged view answers existence (hidden → ENOENT) and, for
        // directories, emptiness (rmdir semantics: ENOTEMPTY).
        let st = self.stat_merged(path)?;
        if st.entry_type == EntryType::Directory && !self.read_dir(path)?.is_empty() {
            return Err(libc::ENOTEMPTY);
        }
        if self.overlay.stat(path).is_ok() {
            self.overlay.remove(path)?;
        }
        if self.base.stat(path).is_ok() {
            // The delete must hide the base entry too (the name is gone
            // from the merged view entirely).
            self.add_whiteout(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends_tar::{TarBackend, TarCompression};
    use crate::mount::{self, MountMode, TEBAKO_MOUNT_COW, TEBAKO_MOUNT_RW};
    use std::ffi::CString;
    use std::fs::File;

    fn append_file(b: &mut tar::Builder<Vec<u8>>, path: &str, data: &[u8], mode: u32) {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Regular);
        h.set_mode(mode);
        h.set_mtime(1_700_000_000);
        h.set_size(data.len() as u64);
        b.append_data(&mut h, path, data).unwrap();
    }

    /// The base image used across the COW tests.
    fn make_base_tar() -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        append_file(&mut b, "etc/motd", b"base-motd\n", 0o644);
        append_file(&mut b, "etc/deep/nested.txt", b"nested\n", 0o640);
        append_file(&mut b, "bin/tool", b"tool-binary", 0o755);
        append_file(&mut b, "todelete.txt", b"delete me", 0o644);
        append_file(&mut b, "delsub/a.txt", b"aaa", 0o644);
        append_file(&mut b, "delsub/b.txt", b"bbb", 0o644);
        b.finish().unwrap();
        b.into_inner().unwrap()
    }

    fn cow() -> (tempfile::TempDir, CowBackend) {
        let dir = tempfile::tempdir().unwrap();
        let base =
            Box::new(TarBackend::from_memory(make_base_tar(), TarCompression::None).unwrap());
        let overlay = HostDirBackend::new(dir.path()).unwrap();
        let cow = CowBackend::new(base, overlay).unwrap();
        (dir, cow)
    }

    fn pread_all(b: &dyn Backend, path: &str) -> Vec<u8> {
        let st = b.stat(path).unwrap();
        let mut buf = vec![0u8; st.size as usize];
        let n = b.pread(path, &mut buf, 0).unwrap();
        assert_eq!(n, buf.len());
        buf
    }

    fn names(b: &dyn Backend, path: &str) -> Vec<String> {
        let mut v: Vec<String> = b
            .read_dir(path)
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        v.sort();
        v
    }

    // ---------------------------------------------------------------
    // Layering basics + the transforms law (base never learns to write)
    // ---------------------------------------------------------------

    #[test]
    fn cow_layers_and_journal_created() {
        let (dir, cow) = cow();
        assert_eq!(cow.name().to_str().unwrap(), "COW");
        // Base reads pass through with base metadata.
        let st = cow.stat("etc/motd").unwrap();
        assert_eq!((st.entry_type, st.perms), (EntryType::File, 0o644));
        assert_eq!(pread_all(&cow, "etc/motd"), b"base-motd\n");
        assert_eq!(names(&cow, "etc"), vec!["deep", "motd"]);
        assert_eq!(
            names(&cow, ""),
            vec!["bin", "delsub", "etc", "todelete.txt"]
        );
        // The journal exists from mount time, is hidden from the VFS...
        let journal = dir.path().join(JOURNAL_FILE);
        assert!(journal.exists());
        assert_eq!(
            parse_whiteouts(&std::fs::read_to_string(&journal).unwrap()).unwrap(),
            BTreeSet::new()
        );
        assert_eq!(cow.stat(JOURNAL_FILE).unwrap_err(), libc::ENOENT);
        assert!(!names(&cow, "").contains(&JOURNAL_FILE.to_string()));
        assert_eq!(
            cow.pread(JOURNAL_FILE, &mut [0u8; 4], 0).unwrap_err(),
            libc::ENOENT
        );
        // ...and the write view is only on the composite (spec 00 inv. 5).
        assert!(cow.writable().is_some());
        assert!(cow.base().writable().is_none());
    }

    #[test]
    fn cow_write_new_file_lands_in_overlay() {
        let (dir, cow) = cow();
        let w = cow.writable().unwrap();
        assert_eq!(w.pwrite("var/log/app.log", b"line1\n", 0).unwrap(), 6);
        assert_eq!(pread_all(&cow, "var/log/app.log"), b"line1\n");
        // The base does not have it; the overlay host directory does.
        assert_eq!(
            cow.base().stat("var/log/app.log").unwrap_err(),
            libc::ENOENT
        );
        assert_eq!(
            std::fs::read(dir.path().join("var/log/app.log")).unwrap(),
            b"line1\n"
        );
        assert!(names(&cow, "").contains(&"var".to_string()));
        assert_eq!(names(&cow, "var/log"), vec!["app.log"]);
    }

    #[test]
    fn cow_modify_base_file_shadow_wins() {
        let (dir, cow) = cow();
        let w = cow.writable().unwrap();
        // Positioned write without truncate: copy-up preserves the rest.
        w.pwrite("etc/motd", b"NEW", 0).unwrap();
        assert_eq!(pread_all(&cow, "etc/motd"), b"NEWe-motd\n");
        // The base still serves the original bytes.
        assert_eq!(pread_all(cow.base(), "etc/motd"), b"base-motd\n");
        // The overlay carries the shadow with the base permissions.
        let shadow = dir.path().join("etc/motd");
        assert_eq!(std::fs::read(&shadow).unwrap(), b"NEWe-motd\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&shadow).unwrap().permissions().mode() & 0o777,
                0o644
            );
        }
        w.truncate("etc/motd", 3).unwrap();
        assert_eq!(pread_all(&cow, "etc/motd"), b"NEW");
        assert_eq!(pread_all(cow.base(), "etc/motd"), b"base-motd\n");
    }

    #[test]
    fn cow_delete_base_file_whiteout_hides() {
        let (dir, cow) = cow();
        let w = cow.writable().unwrap();
        w.remove("todelete.txt").unwrap();
        assert_eq!(cow.stat("todelete.txt").unwrap_err(), libc::ENOENT);
        assert_eq!(
            cow.pread("todelete.txt", &mut [0u8; 4], 0).unwrap_err(),
            libc::ENOENT
        );
        // The base entry is untouched; the journal records the whiteout.
        assert_eq!(
            cow.base().stat("todelete.txt").unwrap().entry_type,
            EntryType::File
        );
        let journal = std::fs::read_to_string(dir.path().join(JOURNAL_FILE)).unwrap();
        assert!(journal.contains("W todelete.txt\n"), "journal: {journal:?}");
        assert_eq!(
            parse_whiteouts(&journal).unwrap(),
            BTreeSet::from(["todelete.txt".to_string()])
        );
        assert!(!names(&cow, "").contains(&"todelete.txt".to_string()));
        // Deleting again is ENOENT; deleting a nonexistent path is ENOENT.
        assert_eq!(w.remove("todelete.txt").unwrap_err(), libc::ENOENT);
        assert_eq!(w.remove("nonexistent").unwrap_err(), libc::ENOENT);
    }

    #[test]
    fn cow_delete_dir_hides_subtree_and_recreate() {
        let (_dir, cow) = cow();
        let w = cow.writable().unwrap();
        // rmdir semantics: non-empty dirs refuse (base-only children count).
        assert_eq!(w.remove("delsub").unwrap_err(), libc::ENOTEMPTY);
        w.remove("delsub/a.txt").unwrap();
        w.remove("delsub/b.txt").unwrap();
        w.remove("delsub").unwrap();
        // A whiteout on a directory hides its whole base subtree.
        assert_eq!(cow.stat("delsub").unwrap_err(), libc::ENOENT);
        assert_eq!(
            cow.base().stat("delsub/a.txt").unwrap().entry_type,
            EntryType::File
        );
        // Writes materialize parents on demand (overlay-style): recreating
        // under the deleted directory works without an explicit mkdir...
        w.pwrite("delsub/new.txt", b"new-content", 0).unwrap();
        assert_eq!(pread_all(&cow, "delsub/new.txt"), b"new-content");
        // ...the overlay entry takes the name over the STANDING whiteout
        // (the delete record stays in the journal; base children of the
        // whiteouted directory stay hidden).
        assert!(cow.whiteouts().contains("delsub"));
        assert_eq!(cow.stat("delsub").unwrap().entry_type, EntryType::Directory);
        assert_eq!(cow.stat("delsub/a.txt").unwrap_err(), libc::ENOENT);
        assert_eq!(names(&cow, "delsub"), vec!["new.txt"]);
        // mkdir over the recreated directory is EEXIST; a second delete
        // removes the overlay entry while the base stays whiteouted.
        assert_eq!(w.mkdir("delsub", 0o755).unwrap_err(), libc::EEXIST);
        w.remove("delsub/new.txt").unwrap();
        w.remove("delsub").unwrap();
        assert_eq!(cow.stat("delsub").unwrap_err(), libc::ENOENT);
        assert_eq!(
            cow.base().stat("delsub/a.txt").unwrap().entry_type,
            EntryType::File
        );
    }

    #[test]
    fn cow_unmount_leaves_base_byte_identical() {
        let dir = tempfile::tempdir().unwrap();
        let tar_path = dir.path().join("base.tar");
        std::fs::write(&tar_path, make_base_tar()).unwrap();
        let before = std::fs::read(&tar_path).unwrap();

        let overlay_dir = tempfile::tempdir().unwrap();
        {
            let base = Box::new(
                TarBackend::from_file(File::open(&tar_path).unwrap(), TarCompression::None)
                    .unwrap(),
            );
            let overlay = HostDirBackend::new(overlay_dir.path()).unwrap();
            let cow = CowBackend::new(base, overlay).unwrap();
            let w = cow.writable().unwrap();
            w.pwrite("etc/motd", b"rewritten entirely", 0).unwrap();
            w.pwrite("brand/new.file", b"new", 0).unwrap();
            w.remove("todelete.txt").unwrap();
            w.truncate("bin/tool", 4).unwrap();
            drop(cow);
        }
        let after = std::fs::read(&tar_path).unwrap();
        assert_eq!(before, after, "the base image must be byte-identical");
    }

    #[test]
    fn cow_journal_persists_across_remounts() {
        let dir = tempfile::tempdir().unwrap();
        {
            let (_d, cow) = {
                let base = Box::new(
                    TarBackend::from_memory(make_base_tar(), TarCompression::None).unwrap(),
                );
                let overlay = HostDirBackend::new(dir.path()).unwrap();
                ((), CowBackend::new(base, overlay).unwrap())
            };
            let w = cow.writable().unwrap();
            w.remove("todelete.txt").unwrap();
            w.pwrite("kept.txt", b"kept", 0).unwrap();
        }
        // Re-stack on the same overlay directory.
        let base =
            Box::new(TarBackend::from_memory(make_base_tar(), TarCompression::None).unwrap());
        let overlay = HostDirBackend::new(dir.path()).unwrap();
        let cow = CowBackend::new(base, overlay).unwrap();
        assert_eq!(cow.stat("todelete.txt").unwrap_err(), libc::ENOENT);
        assert_eq!(pread_all(&cow, "kept.txt"), b"kept");
        assert!(cow.whiteouts().contains("todelete.txt"));
    }

    // ---------------------------------------------------------------
    // The whiteout journal format
    // ---------------------------------------------------------------

    #[test]
    fn journal_serialize_parse_roundtrip() {
        let set: BTreeSet<String> = [
            "a.txt",
            "dir/b file.txt",
            "per%cent",
            "new\nline",
            "carr\rret",
            "uni-λ",
            "deep/very/long/path",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let text = serialize_whiteouts(&set);
        assert!(text.starts_with("TFS-WHITEOUTS 1\n"));
        assert_eq!(parse_whiteouts(&text).unwrap(), set);
        // Deterministic (sorted) output.
        assert_eq!(serialize_whiteouts(&set), text);
        // Empty set: header only.
        assert_eq!(
            parse_whiteouts("TFS-WHITEOUTS 1\n").unwrap(),
            BTreeSet::new()
        );

        // Strict parsing: named error on every malformed form.
        assert_eq!(
            parse_whiteouts("TFS-WHITEOUTS 2\n").unwrap_err(),
            libc::EINVAL
        );
        assert_eq!(parse_whiteouts("X a\n").unwrap_err(), libc::EINVAL);
        assert_eq!(
            parse_whiteouts("TFS-WHITEOUTS 1\nX a\n").unwrap_err(),
            libc::EINVAL
        );
        assert_eq!(
            parse_whiteouts("TFS-WHITEOUTS 1\nW bad%zz\n").unwrap_err(),
            libc::EINVAL
        );
        assert_eq!(
            parse_whiteouts("TFS-WHITEOUTS 1\nW trailing%\n").unwrap_err(),
            libc::EINVAL
        );
        assert_eq!(
            parse_whiteouts("TFS-WHITEOUTS 1\nW /absolute\n").unwrap_err(),
            libc::EINVAL
        );
        assert_eq!(
            parse_whiteouts("TFS-WHITEOUTS 1\nW \n").unwrap_err(),
            libc::EINVAL
        );

        // Escapes round-trip byte-exactly.
        assert_eq!(unescape(&escape("100%\n\r")).unwrap(), "100%\n\r");
    }

    // ---------------------------------------------------------------
    // Mount-mode wiring (spec 11 §3): RO unchanged, COW through the
    // mount entry points and the C ABI, RW honestly ENOTSUP.
    // ---------------------------------------------------------------

    #[test]
    fn mount_modes_through_builders() {
        let dir = tempfile::tempdir().unwrap();
        let tar_path = dir.path().join("base.tar");
        std::fs::write(&tar_path, make_base_tar()).unwrap();
        let path = tar_path.to_str().unwrap();

        // RO (default) — unchanged behavior.
        let ro = mount::build_from_file(path, "/ro").unwrap();
        assert_eq!(ro.mode, MountMode::ReadOnly);
        assert_eq!(ro.backend.name().to_str().unwrap(), "TAR");

        // COW stacks the composite; overlay dir is created when missing.
        let overlay = dir.path().join("overlay");
        let cow = mount::build_from_file_with_mode(
            path,
            "/cow",
            MountMode::Cow,
            Some(overlay.to_str().unwrap()),
        )
        .unwrap();
        assert_eq!(cow.mode, MountMode::Cow);
        assert_eq!(cow.backend.name().to_str().unwrap(), "COW");
        assert_eq!(cow.backend.stat("etc/motd").unwrap().size, 10);
        assert!(overlay.join(JOURNAL_FILE).exists());

        // COW over a memory image works too.
        let cow_mem = mount::build_from_memory_with_mode(
            &make_base_tar(),
            "/cow-mem",
            MountMode::Cow,
            Some(tempfile::tempdir().unwrap().path().to_str().unwrap()),
        )
        .unwrap();
        assert_eq!(cow_mem.backend.name().to_str().unwrap(), "COW");

        // RW is honestly ENOTSUP; an overlay without COW is EINVAL; COW
        // without an overlay dir is EINVAL.
        assert_eq!(
            mount::build_from_file_with_mode(path, "/rw", MountMode::ReadWrite, None).err(),
            Some(libc::ENOTSUP)
        );
        assert_eq!(
            mount::build_from_file_with_mode(
                path,
                "/bad1",
                MountMode::ReadOnly,
                Some(overlay.to_str().unwrap())
            )
            .err(),
            Some(libc::EINVAL)
        );
        assert_eq!(
            mount::build_from_file_with_mode(path, "/bad2", MountMode::Cow, None).err(),
            Some(libc::EINVAL)
        );
    }

    #[test]
    fn mount_modes_through_cabi_and_context_writes() {
        use crate::c_api::*;
        use crate::context::context;

        let dir = tempfile::tempdir().unwrap();
        let tar_path = dir.path().join("base.tar");
        std::fs::write(&tar_path, make_base_tar()).unwrap();
        let c_tar = CString::new(tar_path.to_str().unwrap()).unwrap();
        let c_mp_cow = CString::new("/cow-abi").unwrap();
        let c_mp_ro = CString::new("/ro-abi").unwrap();
        let c_overlay = CString::new(dir.path().join("ov").to_str().unwrap()).unwrap();

        unsafe {
            // COW mount through the additive with_mode entry point.
            let mut h_cow = -1;
            assert_eq!(
                tebako_fs_mount_from_file_with_mode(
                    c_tar.as_ptr(),
                    c_mp_cow.as_ptr(),
                    TEBAKO_MOUNT_COW as libc::c_int,
                    c_overlay.as_ptr(),
                    &mut h_cow,
                ),
                0
            );
            assert_eq!(tebako_get_errno(), 0);
            assert!(h_cow >= 0);

            // RO mount through the legacy entry (delegates to RO with_mode).
            let mut h_ro = -1;
            assert_eq!(
                tebako_fs_mount_from_file(c_tar.as_ptr(), c_mp_ro.as_ptr(), &mut h_ro),
                0
            );

            // Context-level writes: COW accepts, RO refuses with EROFS
            // (RO behavior unchanged).
            let ctx = context().read().unwrap();
            assert_eq!(ctx.pwrite_path("/cow-abi/newfile.txt", b"abc", 0), Ok(3));
            let st = ctx.stat("/cow-abi/newfile.txt").unwrap();
            assert_eq!(st.size, 3);
            assert_eq!(ctx.mkdir_path("/cow-abi/made", 0o755), Ok(()));
            assert_eq!(ctx.remove_path("/cow-abi/todelete.txt"), Ok(()));
            assert_eq!(ctx.stat("/cow-abi/todelete.txt").unwrap_err(), libc::ENOENT);
            assert_eq!(
                ctx.pwrite_path("/ro-abi/x", b"a", 0).unwrap_err(),
                libc::EROFS
            );
            assert_eq!(ctx.mkdir_path("/ro-abi/x", 0o755).unwrap_err(), libc::EROFS);
            assert_eq!(
                ctx.remove_path("/ro-abi/etc/motd").unwrap_err(),
                libc::EROFS
            );
            assert_eq!(
                ctx.truncate_path("/ro-abi/etc/motd", 0).unwrap_err(),
                libc::EROFS
            );
            drop(ctx);

            // Write opens stay EROFS on every mount (fd write family: later).
            // (Separate guard: a write lock while holding the read lock
            // would deadlock.)
            assert_eq!(
                context()
                    .write()
                    .unwrap()
                    .open("/cow-abi/etc/motd", libc::O_WRONLY)
                    .unwrap_err(),
                libc::EROFS
            );

            // The overlay holds the change record.
            assert!(dir.path().join("ov/newfile.txt").exists());
            assert!(dir.path().join("ov").join(JOURNAL_FILE).exists());

            // Flag validation through the ABI.
            let mut h = -1;
            assert_eq!(
                tebako_fs_mount_from_file_with_mode(
                    c_tar.as_ptr(),
                    c_mp_cow.as_ptr(),
                    TEBAKO_MOUNT_RW as libc::c_int,
                    std::ptr::null(),
                    &mut h,
                ),
                -1
            );
            assert_eq!(tebako_get_errno(), libc::ENOTSUP);
            let c_mp_dup = CString::new("/cow-abi-2").unwrap();
            assert_eq!(
                tebako_fs_mount_from_file_with_mode(
                    c_tar.as_ptr(),
                    c_mp_dup.as_ptr(),
                    TEBAKO_MOUNT_COW as libc::c_int,
                    std::ptr::null(), // COW without an overlay
                    &mut h,
                ),
                -1
            );
            assert_eq!(tebako_get_errno(), libc::EINVAL);
            assert_eq!(
                tebako_fs_mount_from_file_with_mode(
                    c_tar.as_ptr(),
                    c_mp_dup.as_ptr(),
                    99,
                    std::ptr::null(),
                    &mut h,
                ),
                -1
            );
            assert_eq!(tebako_get_errno(), libc::EINVAL);

            assert_eq!(tebako_fs_unmount_handle(h_cow), 0);
            assert_eq!(tebako_fs_unmount_handle(h_ro), 0);
        }
    }
}
