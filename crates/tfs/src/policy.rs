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

use std::fmt;
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Who installed the policy (`manifest`, `user`, `manifest+user`,
    /// `TEBAKO_JAIL`, …), recorded in the audit journal on every denial
    /// (spec 08 §2: violations are logged with path + syscall class).
    source: String,
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
            source: String::new(),
        }
    }

    /// Record who installed the policy (the audit-journal `source=` field,
    /// spec 08 §2): the bootstrap/dispatch surfaces label the composition
    /// (`manifest`, `user`, `manifest+user`), direct installers name their
    /// channel (`TEBAKO_JAIL`, `tebako_fs_host_policy`, …).
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// The installer's label (empty when never attributed).
    pub fn source(&self) -> &str {
        &self.source
    }

    /// True when this policy can never deny anything (open default with no
    /// mounts — argument files only ever ALLOW). Installers skip opening
    /// the audit journal for it: there would never be a violation to log.
    pub fn never_denies(&self) -> bool {
        self.default_open && self.mounts.is_empty()
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
            source: String::new(),
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

    /// Serialize the policy to the canonical `TEBAKO_JAIL` env form (the
    /// inverse of [`JailSpec::parse`]). Because the policy is bound, every
    /// host path is already canonical — a consumer that re-parses and
    /// re-binds the string gets an identical policy.
    pub fn to_env_spec(&self) -> String {
        let mut out = String::from(if self.default_open { "open" } else { "deny" });
        for m in &self.mounts {
            out.push(';');
            out.push_str(&m.host.to_string_lossy());
            out.push(':');
            out.push_str(&m.mount);
            out.push(':');
            out.push_str(match m.access {
                HostAccess::Ro => "ro",
                HostAccess::Rw => "rw",
            });
        }
        for f in &self.arg_files {
            out.push(';');
            out.push('@');
            out.push_str(&f.to_string_lossy());
        }
        out
    }
}

/// The parsed form of a `TEBAKO_JAIL` / `--jail` spec string (spec 08 §1,
/// env encoding shared by the preload shim and the dispatch surfaces).
///
/// Grammar (`;`-separated directives, order-free):
///
/// ```text
/// jail      = directive *( ";" directive )
/// directive = "open" | "deny"          # namespace default (default: open)
///           | host ":" mount ":" mode  # docker -v grant; mount absolute
///           | "@" path                 # argument file (read-only grant)
/// mode      = "ro" | "rw"
/// ```
///
/// Example: `deny;/home/u/src:/work:rw;@/home/u/input.csv`.
///
/// Bind with [`HostPolicy::bind`]; serialize a bound policy back with
/// [`HostPolicy::to_env_spec`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JailSpec {
    /// Namespace default: `true` = open (today's behavior), `false` = deny.
    pub default_open: bool,
    /// Host-mount grants (docker `-v` semantics).
    pub mounts: Vec<HostMountSpec>,
    /// Argument files: read-only grants, honored even under deny.
    pub arg_files: Vec<PathBuf>,
}

/// A named, human-readable jail-spec parse error (the offending token is
/// always quoted; spec 14 §3's "named errors on malformed input").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JailSpecError(pub String);

impl fmt::Display for JailSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid jail spec: {}", self.0)
    }
}

impl std::error::Error for JailSpecError {}

impl JailSpec {
    /// Parse a `TEBAKO_JAIL` / `--jail` spec. Errors on: empty spec,
    /// conflicting or duplicated `open`/`deny`, unknown access modes,
    /// non-absolute mount points, empty hosts/mounts/argument files, and
    /// unrecognised directive shapes.
    pub fn parse(spec: &str) -> Result<Self, JailSpecError> {
        let err = |msg: String| JailSpecError(format!("{msg} in {spec:?}"));
        if spec.trim().is_empty() {
            return Err(JailSpecError("empty spec".to_string()));
        }
        let mut default_open: Option<bool> = None;
        let mut mounts = Vec::new();
        let mut arg_files = Vec::new();
        for token in spec.split(';') {
            if token.is_empty() {
                return Err(err("empty directive (stray ';')".to_string()));
            }
            match token {
                "open" | "deny" => {
                    let value = token == "open";
                    if default_open.is_some() {
                        return Err(err(format!(
                            "duplicate/conflicting default directive {token:?}"
                        )));
                    }
                    default_open = Some(value);
                }
                _ if token.starts_with('@') => {
                    let path = &token[1..];
                    if path.is_empty() {
                        return Err(err("empty argument file".to_string()));
                    }
                    arg_files.push(PathBuf::from(path));
                }
                _ => {
                    // host:mount:access — split from the RIGHT so host paths
                    // containing ':' survive; mount must be absolute.
                    let Some((head, access)) = token.rsplit_once(':') else {
                        return Err(err(format!(
                            "directive {token:?} is not open|deny, @file, or host:mount:ro|rw"
                        )));
                    };
                    let access = match access {
                        "ro" => HostAccess::Ro,
                        "rw" => HostAccess::Rw,
                        other => {
                            return Err(err(format!("unknown access mode {other:?} (want ro|rw)")))
                        }
                    };
                    let Some((host, mount)) = head.rsplit_once(':') else {
                        return Err(err(format!(
                            "grant {token:?} needs the host:mount:ro|rw shape"
                        )));
                    };
                    if host.is_empty() {
                        return Err(err(format!("empty host in grant {token:?}")));
                    }
                    if !mount.starts_with('/') {
                        return Err(err(format!("mount point {mount:?} is not absolute")));
                    }
                    mounts.push(HostMountSpec {
                        host: PathBuf::from(host),
                        mount: mount.to_string(),
                        access,
                    });
                }
            }
        }
        Ok(JailSpec {
            default_open: default_open.unwrap_or(true),
            mounts,
            arg_files,
        })
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

    // ---------------------------------------------------------------
    // JailSpec (TEBAKO_JAIL / --jail env form) + to_env_spec
    // ---------------------------------------------------------------

    #[test]
    fn jail_spec_parses_all_directive_kinds() {
        let s =
            JailSpec::parse("deny;/home/u/src:/work:rw;@/home/u/in.csv;/data:/data:ro").unwrap();
        assert!(!s.default_open);
        assert_eq!(
            s.mounts,
            vec![
                HostMountSpec {
                    host: PathBuf::from("/home/u/src"),
                    mount: "/work".to_string(),
                    access: HostAccess::Rw,
                },
                HostMountSpec {
                    host: PathBuf::from("/data"),
                    mount: "/data".to_string(),
                    access: HostAccess::Ro,
                },
            ]
        );
        assert_eq!(s.arg_files, vec![PathBuf::from("/home/u/in.csv")]);
    }

    #[test]
    fn jail_spec_defaults_to_open_without_a_default_directive() {
        let s = JailSpec::parse("/h:/w:ro").unwrap();
        assert!(s.default_open);
        assert_eq!(s.mounts.len(), 1);
    }

    #[test]
    fn jail_spec_host_may_contain_colons() {
        // Split from the right: only the last two ':' delimit mount+access.
        let s = JailSpec::parse("/Volumes/a:b/work:/w:rw").unwrap();
        assert_eq!(s.mounts[0].host, PathBuf::from("/Volumes/a:b/work"));
        assert_eq!(s.mounts[0].mount, "/w");
        assert_eq!(s.mounts[0].access, HostAccess::Rw);
    }

    #[test]
    fn jail_spec_rejects_malformed_input_with_named_errors() {
        for (spec, frag) in [
            ("", "empty spec"),
            ("   ", "empty spec"),
            ("open;deny", "duplicate/conflicting"),
            ("deny;deny", "duplicate/conflicting"),
            ("deny;;/h:/w:ro", "empty directive"),
            ("/h:/w:xx", "unknown access mode"),
            ("/h:w:ro", "not absolute"),
            ("/h:/w", "unknown access mode"),
            ("frob", "host:mount:ro|rw"),
            (":/w:ro", "empty host"),
            ("@", "empty argument file"),
        ] {
            let e = JailSpec::parse(spec).unwrap_err();
            assert!(
                e.0.contains(frag),
                "spec {spec:?}: error {e:?} should mention {frag:?}"
            );
        }
        // The offending spec is always quoted for the user.
        assert!(JailSpec::parse("frob").unwrap_err().0.contains("\"frob\""));
    }

    #[test]
    fn jail_spec_bind_env_round_trip() {
        let tree = Tree::new("jailrt");
        let spec = JailSpec::parse(&format!(
            "deny;{}:/work:rw;{}:/ro:ro;@{}",
            tree.work.display(),
            tree.rodir.display(),
            tree.input.display()
        ))
        .unwrap();
        let policy = HostPolicy::bind(spec.default_open, spec.mounts, spec.arg_files).unwrap();
        let env = policy.to_env_spec();
        let spec2 = JailSpec::parse(&env).unwrap();
        let policy2 = HostPolicy::bind(spec2.default_open, spec2.mounts, spec2.arg_files).unwrap();
        assert_eq!(policy, policy2, "env round trip: {env}");
        assert!(env.starts_with("deny;"), "{env}");
        assert!(env.contains(":/work:rw"), "{env}");
        assert!(env.contains(":/ro:ro"), "{env}");
        assert!(env.contains(";@"), "{env}");
    }

    #[test]
    fn open_policy_serializes_to_open() {
        assert_eq!(HostPolicy::open().to_env_spec(), "open");
    }
}
