//! Host-access policy (spec 08 — jails).
//!
//! The TFS ABI mediates memfs paths; a path no mount claims is handed back
//! to the consumer with ENOENT, and the consumer falls through to the host
//! filesystem — today unrestricted. The host policy is the gate on that
//! fall-through: every host-passthrough path decision in the context
//! (open/stat/opendir/dlmap2file, extract_all's destination, and the
//! mount-family's image read) consults it. Denied paths fail EPERM, writes
//! against a read-only grant fail EROFS, and allowed paths keep today's
//! answer (ENOENT = "not ours, pass through"), so bootstrap, the runtime
//! driver, `tebako run` and `tfs` enforce identically with no per-app work.
//!
//! Docker `-v` semantics: `default_open` is the namespace default; mounts
//! are (host dir, virtual point, ro|rw) grants matched by longest host
//! prefix on path-component boundaries; argument files are allowed for
//! reading even under deny. Host paths are realpath-canonicalized at bind
//! time and RE-CANONICALIZED on every check, so a symlink swapped in after
//! the policy was installed resolves to its target and escapes fail.
//!
//! The policy is about HOST paths only; memfs mounts are unaffected. It is
//! process state, not namespace state: `unmount()` does not reset it
//! (fail-closed), and installing a new policy replaces the old one — the
//! caller (bootstrap / `tebako run`) is the policy owner. Payloads with raw
//! C-ABI access are outside this threat model: native code can always
//! syscall the host directly; the jail covers IO routed through TFS.
//!
//! Pure safe Rust; errno values are the crate's error convention.

use std::path::{Component, Path, PathBuf};

/// Access mode: a mount's grant bit, and the level an IO route requests.
///
/// A request is satisfiable when the grant is at least as wide: `Ro` asks
/// read, `Rw` asks write (requires an `Rw` grant; against an `Ro` grant the
/// answer is EROFS). The C ABI maps this as 0 = ro, 1 = rw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostAccess {
    /// Read-only.
    Ro,
    /// Read-write.
    Rw,
}

/// One host-mount grant, as bound (host side already canonicalized).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostMount {
    /// Host directory, realpath-canonicalized at bind time.
    pub host: PathBuf,
    /// Virtual mount point the host dir is exposed at (e.g. "/work").
    /// Carried for the dispatch layer (spec 07) and the future
    /// HostDirBackend; enforcement here matches on `host` prefixes.
    pub mount: String,
    /// Grant bit.
    pub access: HostAccess,
}

/// Bind-time form of a [`HostMount`] (host side not yet canonicalized).
#[derive(Debug, Clone)]
pub struct HostMountSpec {
    /// Host directory as handed in (may contain `.`/`..`/symlinks).
    pub host: PathBuf,
    /// Virtual mount point; must be absolute (EINVAL otherwise).
    pub mount: String,
    /// Grant bit.
    pub access: HostAccess,
}

/// The host-access policy of a process (spec 08 §1).
///
/// The default is `open` with no mounts and no argument files — byte-for
/// byte today's behavior, so consumers that never install a policy see zero
/// change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPolicy {
    default_open: bool,
    mounts: Vec<HostMount>,
    arg_files: Vec<PathBuf>,
}

impl Default for HostPolicy {
    /// The initial policy is OPEN (today's behavior), never deny.
    fn default() -> Self {
        Self::open()
    }
}

impl HostPolicy {
    /// The open policy (today's behavior; the context's initial state).
    pub const fn open() -> Self {
        HostPolicy {
            default_open: true,
            mounts: Vec::new(),
            arg_files: Vec::new(),
        }
    }

    /// Bind a policy: canonicalize every host path (mount sources and
    /// argument files) NOW, so later checks compare canonical forms.
    ///
    /// Errors: EINVAL for a non-absolute virtual mount point; the
    /// canonicalization errno (ENOENT for a missing mount source or
    /// argument file, ENOTDIR/ELOOP/… as reported) otherwise.
    pub fn bind(
        default_open: bool,
        mounts: Vec<HostMountSpec>,
        arg_files: Vec<PathBuf>,
    ) -> Result<Self, i32> {
        let mut bound_mounts = Vec::with_capacity(mounts.len());
        for spec in mounts {
            if !spec.mount.starts_with('/') {
                return Err(libc::EINVAL);
            }
            let host = canonicalize(&spec.host)?;
            bound_mounts.push(HostMount {
                host,
                mount: spec.mount,
                access: spec.access,
            });
        }
        let mut bound_files = Vec::with_capacity(arg_files.len());
        for f in &arg_files {
            bound_files.push(canonicalize(f)?);
        }
        Ok(HostPolicy {
            default_open,
            mounts: bound_mounts,
            arg_files: bound_files,
        })
    }

    /// Gate one host-passthrough path decision. `need` is the access the IO
    /// route asks for (`Rw` for any write-class operation).
    ///
    /// Ok(()) means "allowed" — the caller answers ENOENT and the consumer
    /// passes through to the host fs, exactly like today. Err(EPERM) when no
    /// grant covers the path under a deny default; Err(EROFS) for a write
    /// against an ro grant.
    pub fn check(&self, path: &Path, need: HostAccess) -> Result<(), i32> {
        // Open policy with no mounts: today's behavior, zero overhead.
        if self.default_open && self.mounts.is_empty() {
            return Ok(());
        }
        // Re-validate realpath on each open: the target is canonicalized at
        // check time, so symlink swaps after bind resolve to their target.
        let canon = canonicalize_lenient(path);

        // Argument files: exact (canonical) match, read grant only, even
        // under deny — "the input file you hand the command is allowed".
        if need == HostAccess::Ro && self.arg_files.contains(&canon) {
            return Ok(());
        }

        // Longest host-prefix match; Path::starts_with matches on whole
        // path components, so "/work" never matches "/workshop".
        let mut best: Option<&HostMount> = None;
        for m in &self.mounts {
            let longer = match best {
                Some(b) => b.host.as_os_str().len() < m.host.as_os_str().len(),
                None => true,
            };
            if longer && canon.starts_with(&m.host) {
                best = Some(m);
            }
        }
        if let Some(m) = best {
            // A mount's grant bit applies even under an open default (an ro
            // bind is ro in an otherwise open namespace, docker-style).
            if need == HostAccess::Rw && m.access == HostAccess::Ro {
                return Err(libc::EROFS);
            }
            return Ok(());
        }

        if self.default_open {
            return Ok(());
        }
        Err(libc::EPERM)
    }
}

/// Canonicalize a bind-time path; the errno channel speaks raw OS errors.
fn canonicalize(path: &Path) -> Result<PathBuf, i32> {
    std::fs::canonicalize(path).map_err(|e| e.raw_os_error().unwrap_or(libc::ENOENT))
}

/// Canonicalize for a check: strict realpath when the path exists; when it
/// does not (a write-create target, or a read that will ENOENT on the host
/// anyway), resolve the deepest ancestor that DOES exist and re-append the
/// missing tail with lexical `.`/`..` normalization. The resolved ancestor
/// chain is fully symlink-expanded either way, so escapes are caught; a
/// nonexistent tail cannot contain a symlink.
fn canonicalize_lenient(path: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(path) {
        return c;
    }
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = path;
    loop {
        let Some(parent) = cur.parent() else {
            // No existing ancestor at all (relative path with no cwd
            // anchor): fall back to the lexical form; prefix matching then
            // simply fails, which denies under a deny default.
            return normalize_lexically(path);
        };
        // Paths ending in ".." have no file_name; keep the component
        // verbatim so lexical normalization can still apply it.
        tail.push(
            cur.file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(".."))
                .to_os_string(),
        );
        cur = parent;
        if let Ok(mut base) = std::fs::canonicalize(cur) {
            for comp in tail.iter().rev() {
                base.push(comp);
            }
            return normalize_lexically(&base);
        }
    }
}

/// Lexical `.`/`..` normalization (no symlink resolution — the input's
/// existing prefix is already canonical).
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                // "/" stays "/"; a leading ".." of a relative path is kept.
                if !out.pop() && !out.has_root() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A temp directory that removes itself on drop (unique per instance).
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "tfs-policy-test-{tag}-{}-{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).unwrap();
            // Canonicalize once: macOS temp dirs live behind /var ->
            // /private/var, and the policy compares canonical forms.
            TempDir(std::fs::canonicalize(&dir).unwrap())
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The shared fixture tree:
    ///
    /// ```text
    /// <tmp>/work/hello.txt
    /// <tmp>/work/sub/
    /// <tmp>/sibling/secret.txt
    /// <tmp>/rodir/ro.txt
    /// <tmp>/input.txt
    /// ```
    struct Tree {
        _tmp: TempDir,
        work: PathBuf,
        sibling: PathBuf,
        rodir: PathBuf,
        input: PathBuf,
    }

    impl Tree {
        fn new(tag: &str) -> Self {
            let tmp = TempDir::new(tag);
            let work = tmp.0.join("work");
            let sibling = tmp.0.join("sibling");
            let rodir = tmp.0.join("rodir");
            std::fs::create_dir_all(work.join("sub")).unwrap();
            std::fs::create_dir_all(&sibling).unwrap();
            std::fs::create_dir_all(&rodir).unwrap();
            std::fs::write(work.join("hello.txt"), b"hi").unwrap();
            std::fs::write(sibling.join("secret.txt"), b"secret").unwrap();
            std::fs::write(rodir.join("ro.txt"), b"ro").unwrap();
            let input = tmp.0.join("input.txt");
            std::fs::write(&input, b"input").unwrap();
            Tree {
                _tmp: tmp,
                work,
                sibling,
                rodir,
                input,
            }
        }

        fn spec(&self, host: &Path, mount: &str, access: HostAccess) -> HostMountSpec {
            HostMountSpec {
                host: host.to_path_buf(),
                mount: mount.to_string(),
                access,
            }
        }
    }

    #[test]
    fn open_policy_allows_everything() {
        let tree = Tree::new("open");
        let p = HostPolicy::open();
        assert_eq!(
            p.check(&tree.sibling.join("secret.txt"), HostAccess::Ro),
            Ok(())
        );
        assert_eq!(
            p.check(&tree.sibling.join("secret.txt"), HostAccess::Rw),
            Ok(())
        );
        assert_eq!(p.check(Path::new("/"), HostAccess::Ro), Ok(()));
    }

    #[test]
    fn default_policy_is_open() {
        // HostPolicy::default() must equal open() — consumers that never
        // install a policy see zero behavior change.
        let p = HostPolicy::default();
        assert_eq!(p.check(Path::new("/anything"), HostAccess::Rw), Ok(()));
    }

    #[test]
    fn deny_without_grants_denies_everything() {
        let tree = Tree::new("deny");
        let p = HostPolicy::bind(false, vec![], vec![]).unwrap();
        assert_eq!(
            p.check(&tree.work.join("hello.txt"), HostAccess::Ro),
            Err(libc::EPERM)
        );
        assert_eq!(p.check(Path::new("/"), HostAccess::Ro), Err(libc::EPERM));
        assert_eq!(p.check(&tree.work, HostAccess::Rw), Err(libc::EPERM));
    }

    #[test]
    fn scoped_mount_rw_allows_inside_denies_sibling() {
        let tree = Tree::new("scoped");
        let p = HostPolicy::bind(
            false,
            vec![tree.spec(&tree.work, "/work", HostAccess::Rw)],
            vec![],
        )
        .unwrap();
        assert_eq!(
            p.check(&tree.work.join("hello.txt"), HostAccess::Ro),
            Ok(())
        );
        assert_eq!(p.check(&tree.work.join("new.txt"), HostAccess::Rw), Ok(()));
        assert_eq!(
            p.check(&tree.sibling.join("secret.txt"), HostAccess::Ro),
            Err(libc::EPERM)
        );
        assert_eq!(p.check(&tree.sibling, HostAccess::Rw), Err(libc::EPERM));
    }

    #[test]
    fn ro_mount_refuses_writes_with_erofs() {
        let tree = Tree::new("ro");
        let p = HostPolicy::bind(
            false,
            vec![tree.spec(&tree.rodir, "/ro", HostAccess::Ro)],
            vec![],
        )
        .unwrap();
        assert_eq!(p.check(&tree.rodir.join("ro.txt"), HostAccess::Ro), Ok(()));
        assert_eq!(
            p.check(&tree.rodir.join("ro.txt"), HostAccess::Rw),
            Err(libc::EROFS)
        );
        // A new file in the ro tree is equally refused.
        assert_eq!(
            p.check(&tree.rodir.join("new.txt"), HostAccess::Rw),
            Err(libc::EROFS)
        );
    }

    #[test]
    fn arg_file_allowed_even_under_deny() {
        let tree = Tree::new("arg");
        let p = HostPolicy::bind(false, vec![], vec![tree.input.clone()]).unwrap();
        assert_eq!(p.check(&tree.input, HostAccess::Ro), Ok(()));
        // …but only for reading, and only the file itself (not its dir).
        assert_eq!(p.check(&tree.input, HostAccess::Rw), Err(libc::EPERM));
        assert_eq!(
            p.check(&tree.sibling.join("secret.txt"), HostAccess::Ro),
            Err(libc::EPERM)
        );
    }

    #[test]
    fn prefix_match_respects_component_boundaries() {
        let tree = Tree::new("boundary");
        let workshop = tree.work.with_file_name("workshop");
        std::fs::create_dir_all(&workshop).unwrap();
        std::fs::write(workshop.join("x.txt"), b"x").unwrap();
        let p = HostPolicy::bind(
            false,
            vec![tree.spec(&tree.work, "/work", HostAccess::Rw)],
            vec![],
        )
        .unwrap();
        assert_eq!(
            p.check(&workshop.join("x.txt"), HostAccess::Ro),
            Err(libc::EPERM)
        );
    }

    #[test]
    fn longest_prefix_wins() {
        let tree = Tree::new("longest");
        let inner = tree.work.join("sub");
        let p = HostPolicy::bind(
            false,
            vec![
                tree.spec(&tree.work, "/work", HostAccess::Ro),
                tree.spec(&inner, "/work/sub", HostAccess::Rw),
            ],
            vec![],
        )
        .unwrap();
        // Under the inner rw mount: writes allowed.
        assert_eq!(p.check(&inner.join("new.txt"), HostAccess::Rw), Ok(()));
        // Still inside the outer ro mount: EROFS.
        assert_eq!(
            p.check(&tree.work.join("hello.txt"), HostAccess::Rw),
            Err(libc::EROFS)
        );
    }

    #[test]
    fn open_default_with_ro_bind_still_enforces_the_bit() {
        let tree = Tree::new("openro");
        let p = HostPolicy::bind(
            true,
            vec![tree.spec(&tree.rodir, "/ro", HostAccess::Ro)],
            vec![],
        )
        .unwrap();
        // Outside the bind: open.
        assert_eq!(
            p.check(&tree.sibling.join("secret.txt"), HostAccess::Rw),
            Ok(())
        );
        // Inside the ro bind: the bit applies even under an open default.
        assert_eq!(
            p.check(&tree.rodir.join("ro.txt"), HostAccess::Rw),
            Err(libc::EROFS)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_denied() {
        let tree = Tree::new("escape");
        let p = HostPolicy::bind(
            false,
            vec![tree.spec(&tree.work, "/work", HostAccess::Rw)],
            vec![],
        )
        .unwrap();
        // A symlink inside the granted tree pointing outside it.
        std::os::unix::fs::symlink(&tree.sibling, tree.work.join("evil")).unwrap();
        assert_eq!(
            p.check(&tree.work.join("evil").join("secret.txt"), HostAccess::Ro),
            Err(libc::EPERM)
        );
        // Same for a file-level symlink.
        std::os::unix::fs::symlink(tree.sibling.join("secret.txt"), tree.work.join("link.txt"))
            .unwrap();
        assert_eq!(
            p.check(&tree.work.join("link.txt"), HostAccess::Ro),
            Err(libc::EPERM)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_swap_after_bind_is_caught_on_revalidation() {
        let tree = Tree::new("swap");
        let target = tree.work.join("data");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("f.txt"), b"f").unwrap();
        let p = HostPolicy::bind(
            false,
            vec![tree.spec(&tree.work, "/work", HostAccess::Rw)],
            vec![],
        )
        .unwrap();
        assert_eq!(p.check(&target.join("f.txt"), HostAccess::Ro), Ok(()));
        // Swap the real dir for a symlink to the sibling: the next check
        // re-canonicalizes and the path resolves outside the grant.
        std::fs::remove_dir_all(&target).unwrap();
        std::os::unix::fs::symlink(&tree.sibling, &target).unwrap();
        assert_eq!(
            p.check(&target.join("secret.txt"), HostAccess::Ro),
            Err(libc::EPERM)
        );
    }

    #[test]
    fn nonexistent_paths_resolve_through_existing_ancestors() {
        let tree = Tree::new("nonexistent");
        let p = HostPolicy::bind(
            false,
            vec![tree.spec(&tree.work, "/work", HostAccess::Rw)],
            vec![],
        )
        .unwrap();
        // Write-create deep inside the rw mount (parent chain partially
        // missing): allowed.
        assert_eq!(
            p.check(&tree.work.join("a/b/c.txt"), HostAccess::Rw),
            Ok(())
        );
        // Dot-dot in the missing tail is applied lexically, staying inside.
        assert_eq!(
            p.check(&tree.work.join("a/../b.txt"), HostAccess::Rw),
            Ok(())
        );
        // The same shape outside every grant: denied.
        assert_eq!(
            p.check(&tree.sibling.join("a/b/c.txt"), HostAccess::Rw),
            Err(libc::EPERM)
        );
    }

    #[test]
    fn bind_canonicalizes_mount_sources() {
        let tree = Tree::new("bindcanon");
        let dotted = tree.work.join("sub").join("..");
        let p = HostPolicy::bind(
            false,
            vec![tree.spec(&dotted, "/work", HostAccess::Rw)],
            vec![],
        )
        .unwrap();
        assert_eq!(p.mounts[0].host, tree.work);
        assert_eq!(
            p.check(&tree.work.join("hello.txt"), HostAccess::Rw),
            Ok(())
        );
    }

    #[test]
    fn bind_rejects_relative_mount_points_and_missing_sources() {
        let tree = Tree::new("binderr");
        assert_eq!(
            HostPolicy::bind(
                false,
                vec![tree.spec(&tree.work, "work", HostAccess::Rw)],
                vec![]
            ),
            Err(libc::EINVAL)
        );
        let missing = tree.work.join("no-such-dir");
        let err = HostPolicy::bind(
            false,
            vec![tree.spec(&missing, "/work", HostAccess::Rw)],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, libc::ENOENT);
        let err = HostPolicy::bind(false, vec![], vec![missing.clone()]).unwrap_err();
        assert_eq!(err, libc::ENOENT);
    }
}
