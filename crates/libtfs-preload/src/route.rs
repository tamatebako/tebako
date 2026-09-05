//! The routing layer: memfs or host-or-denied, decided against the tfs
//! engine's own answers so the shim's behavior is byte-for-byte the
//! engine's (longest-prefix dispatch, spec 08 host-policy gating).
//!
//! For every intercepted path call the rule is:
//!
//! 1. The engine answers (`Ok`) → the path is memfs; the caller is served
//!    from the VFS and never touches the host.
//! 2. `Err(ENOENT)` → "not ours, pass through" — the host-passthrough
//!    decision, already gated by the installed `host_policy` inside the
//!    engine (spec 08: allowed paths keep the historic ENOENT; denied
//!    paths answered EPERM/EROFS instead). The real libc call runs.
//! 3. `Err(ENODEV)` → nothing is mounted at all: the engine never
//!    consulted the policy, so the route layer runs `host_check`
//!    explicitly (jail-only mode: `TEBAKO_JAIL` without mounts).
//! 4. Any other `Err(e)` → the engine's answer for a memfs path
//!    (EROFS/EISDIR/ENOTDIR/…) or the jail's answer (EPERM/EROFS): the
//!    call fails with `e` and the host is never touched.
//!
//! Pure safe Rust over `tfs::context`; all `unsafe` lives in `crate::sys`.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use tfs::backend::RawStat;
use tfs::context::{context, TebakoCDirent, TEBAKO_FD_FLAG};
use tfs::policy::{HostAccess, HostPolicy, JailSpec};

use crate::spec;

/// The route decision for a read-class path call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathRoute<T> {
    /// Answered by the VFS (the engine's value).
    Vfs(T),
    /// "Not ours, pass through" — the policy allows the host fall-through.
    Host,
    /// Fail with this errno (the jail's answer, or the engine's answer for
    /// a memfs path); the real libc call must NOT run.
    Denied(i32),
}

/// Map an engine path answer to a route decision (rules 1–4 above).
/// `need` is consulted only in the ENODEV (no-mounts) case, where the
/// engine never ran the policy check itself.
fn route_answer<T>(answer: Result<T, i32>, path: &str, need: HostAccess) -> PathRoute<T> {
    match answer {
        Ok(v) => PathRoute::Vfs(v),
        Err(e) if e == libc::ENOENT => PathRoute::Host,
        Err(e) if e == libc::ENODEV => match context().read().unwrap().host_check(path, need) {
            Ok(()) => PathRoute::Host,
            Err(e) => PathRoute::Denied(e),
        },
        Err(e) => PathRoute::Denied(e),
    }
}

/// open/openat routing: the engine dispatches, allocates the
/// `TEBAKO_FD_FLAG` fd, and gates the host fall-through (read vs write
/// need derived from the flags, exactly like the C ABI).
pub fn vfs_open(path: &str, flags: i32) -> PathRoute<i32> {
    let need = if (flags & libc::O_ACCMODE) == libc::O_RDONLY {
        HostAccess::Ro
    } else {
        HostAccess::Rw
    };
    // The guard must drop BEFORE route_answer (its ENODEV branch
    // re-acquires the context): bind the answer in its own block.
    let answer = { context().write().unwrap().open(path, flags) };
    route_answer(answer, path, need)
}

/// stat/lstat routing. The engine has no symlink duality (memfs entries
/// are files or dirs), so lstat == stat.
pub fn vfs_stat(path: &str) -> PathRoute<RawStat> {
    let answer = { context().read().unwrap().stat(path) };
    route_answer(answer, path, HostAccess::Ro)
}

/// fstat on a memfs fd (re-dispatched by the fd's path, like the C ABI).
pub fn vfs_fstat(fd: i32) -> Result<RawStat, i32> {
    context().read().unwrap().fstat(fd)
}

/// opendir routing; the VFS answer is the raw dir-handle id the shim
/// returns as an opaque `DIR *`.
pub fn vfs_opendir(path: &str) -> PathRoute<usize> {
    let answer = { context().write().unwrap().opendir(path) };
    route_answer(answer, path, HostAccess::Ro)
}

/// access/faccessat routing. Read-class: existence/mode answered from the
/// memfs stat. W_OK against a memfs entry is EROFS (payload images are
/// always ro, spec 11); X_OK honors the entry's permission bits.
pub fn vfs_access(path: &str, mode: i32) -> PathRoute<()> {
    let st = context().read().unwrap().stat(path);
    match st {
        Ok(raw) => {
            if mode == libc::F_OK {
                return PathRoute::Vfs(());
            }
            if mode & libc::W_OK != 0 {
                return PathRoute::Denied(libc::EROFS);
            }
            if mode & libc::X_OK != 0 && raw.perms & 0o111 == 0 {
                return PathRoute::Denied(libc::EACCES);
            }
            PathRoute::Vfs(())
        }
        Err(e) => match route_answer::<()>(Err(e), path, HostAccess::Ro) {
            PathRoute::Vfs(()) => unreachable!("an Err never routes to Vfs"),
            other => other,
        },
    }
}

/// dlopen routing: a memfs library is materialized to the per-process host
/// cache via the engine's `dlmap2file` (the spec 07 §8 mechanism) and the
/// host path handed to the real dlopen; a host library passes through,
/// policy-gated like any read.
pub fn vfs_dlmap(path: &str) -> PathRoute<std::ffi::CString> {
    let answer = { context().write().unwrap().dlmap2file(path) };
    route_answer(answer, path, HostAccess::Ro)
}

/// fopen routing (read modes only): like dlopen, the consumer needs a
/// real `FILE *` — the engine materializes the memfs original
/// (`dlmap2file`, dlmap-prefix redirect included) and the real fopen
/// opens that copy. Write modes never route here (the caller's real
/// fopen answers for the host path, policy-gated like any write).
/// The trace surface is `open` (spec 25 §2: a stdio consumer is the
/// §4 materialize-candidate signal), never dlopen.
pub fn vfs_fopen(path: &str) -> PathRoute<std::ffi::CString> {
    let answer = { context().write().unwrap().dlmap2file_for_open(path) };
    route_answer(answer, path, HostAccess::Ro)
}

/// realpath routing (spec 07 §8). glibc's realpath(3) walks the path
/// with libc-INTERNAL stat/readlink aliases that LD_PRELOAD cannot
/// interpose, so an un-interposed realpath canonicalizes a VFS path
/// against the HOST root: on usrmerge hosts (`/lib -> usr/lib`) a
/// root-mounted payload's `/lib/…` comes back as `/usr/lib/…`, a spelling
/// the VFS cannot serve — the JDK's `File.getCanonicalFile` canonicalizes
/// every `-jar`/`-cp` classpath entry exactly this way, which dropped the
/// payload's jing jar from the classpath (the dogfood linux-gnu
/// ClassNotFoundException, 2026-09-03; PROGRESS/31).
///
/// The discipline: a path that EXISTS in the VFS is answered with its
/// lexically normalized VFS spelling — the host walk never runs, so no
/// host symlink prefix can leak into it. A path the VFS lacks (a legit
/// host file sitting under a `/` mount, `/etc/…` style) forwards to the
/// real realpath, exactly like the open/stat passthrough. VFS symlink
/// resolution: none (memfs has no symlink duality, and image backends
/// resolve in-image links at lookup), so the normalized spelling IS the
/// honest VFS canonical form.
pub fn vfs_realpath(path: &str) -> PathRoute<std::ffi::CString> {
    // realpath(3) on an empty path answers ENOENT.
    if path.is_empty() {
        return PathRoute::Denied(libc::ENOENT);
    }
    // realpath is absolute-only: a relative input resolves against the
    // cwd first. The kernel never lets a process chdir into the VFS (the
    // VFS has no host directory entries), so the cwd is always host.
    let absolute = if path.starts_with('/') {
        path.to_string()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => format!("{}/{}", cwd.to_string_lossy(), path),
            Err(e) => return PathRoute::Denied(e.raw_os_error().unwrap_or(libc::EIO)),
        }
    };
    let normalized = normalize_lexical(&absolute);
    let st = { context().read().unwrap().stat(&normalized) };
    match route_answer(st, &normalized, HostAccess::Ro) {
        PathRoute::Vfs(_) => match std::ffi::CString::new(normalized) {
            Ok(c) => PathRoute::Vfs(c),
            // Unreachable: `normalized` derives from a NUL-free C string
            // and the transform never introduces one.
            Err(_) => PathRoute::Denied(libc::EINVAL),
        },
        PathRoute::Host => PathRoute::Host,
        PathRoute::Denied(e) => PathRoute::Denied(e),
    }
}

/// The lexical normalizer for the realpath answer — the same discipline
/// as `tfs::context`'s private `normalize` (`.` dropped, `a/..` resolved
/// lexically, `..` at the root clamped, duplicate slashes collapsed). The
/// input is absolute by construction (see the caller), so the result
/// always keeps its leading `/`.
fn normalize_lexical(path: &str) -> String {
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
    format!("/{}", out.join("/"))
}

/// Write-class routing (mkdir/unlink/rename). A path a mount HOLDS is
/// EROFS (payload images are always ro; path-level writes would route
/// through a COW overlay, which the shim never mounts). A path merely
/// COVERED (the image holds nothing there) is a host path — the spec 08
/// passthrough, gated with a write need exactly like an uncovered one:
/// Ok(()) means "pass through to the real call". This is what keeps a
/// `/` mount from outlawing every host write.
pub fn vfs_write_path(path: &str) -> Result<(), i32> {
    let ctx = context().read().unwrap();
    if ctx.path_is_held(path) {
        return Err(libc::EROFS);
    }
    ctx.host_check(path, HostAccess::Rw)
}

/// rename routing: both paths gated, both write-class.
pub fn vfs_rename(old: &str, new: &str) -> Result<(), i32> {
    vfs_write_path(old)?;
    vfs_write_path(new)
}

/// True when `fd` is a memfs descriptor (the `TEBAKO_FD_FLAG` bit; a pure
/// bit test, no engine state — host fds never carry bit 30).
pub fn is_memfs_fd(fd: i32) -> bool {
    (fd & TEBAKO_FD_FLAG) != 0
}

/// True when `id` is a live memfs dir handle (the registry-membership
/// test, same as `tebako_fs_dir_is_embedded`).
pub fn dir_is_embedded(id: usize) -> bool {
    context().read().unwrap().dir_is_embedded(id)
}

pub fn vfs_read(fd: i32, buf: &mut [u8]) -> Result<usize, i32> {
    context().write().unwrap().read(fd, buf)
}

pub fn vfs_pread(fd: i32, buf: &mut [u8], offset: i64) -> Result<usize, i32> {
    context().read().unwrap().pread(fd, buf, offset)
}

pub fn vfs_lseek(fd: i32, offset: i64, whence: i32) -> Result<i64, i32> {
    context().write().unwrap().lseek(fd, offset, whence)
}

pub fn vfs_close(fd: i32) -> Result<(), i32> {
    context().write().unwrap().close(fd)
}

/// The dup class (tebako#534): the engine clones the open-file
/// description (the offset is SHARED) onto a fresh flagged fd; `min` is
/// the fcntl(F_DUPFD) floor (0 for plain dup).
pub fn vfs_dup(fd: i32, min: i32) -> Result<i32, i32> {
    context().write().unwrap().dup(fd, min)
}

/// dup2 onto a memfs-flagged target number (the shim answers ENOTSUP
/// for a host-numbered target before routing here): the engine
/// atomically closes a live target and rebinds the number to the
/// source's description.
pub fn vfs_dup2(old: i32, new: i32) -> Result<i32, i32> {
    context().write().unwrap().dup2(old, new)
}

pub fn vfs_closedir(id: usize) -> Result<(), i32> {
    context().write().unwrap().closedir(id)
}

/// readdir on a memfs handle: Ok(Some(entry)) / Ok(None) at end / Err.
/// The entry is an owned copy (the engine's `current` buffer is reused by
/// the next readdir).
pub fn vfs_readdir(id: usize) -> Result<Option<TebakoCDirent>, i32> {
    let mut ctx = context().write().unwrap();
    match ctx.readdir_abi(id) {
        Ok(true) => Ok(ctx.dir_current(id)),
        Ok(false) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Resolve an openat/faccessat path for ROUTING: absolute paths and
/// AT_FDCWD-relative paths route as given (a relative path can never be
/// memfs — mount points are absolute — and the policy canonicalizes it
/// against the cwd); a dirfd-relative path joins the dirfd's own path
/// (`base`, resolved by the sys layer) so a mount point under it is found.
/// The real libc call always receives the ORIGINAL (dirfd, path) pair.
///
/// A memfs dirfd is ENOTDIR (memfs fds are regular files only — opening a
/// memfs directory fails EISDIR at open time). An unresolvable dirfd keeps
/// the relative path: under a deny policy the cwd-relative check then
/// fails closed (EPERM) for anything outside the grants.
pub fn resolve_at(dirfd: i32, path: &str, base: Option<PathBuf>) -> Result<String, i32> {
    match resolve_at_strict(dirfd, path, base)? {
        AtRoute::Routed(p) => Ok(p),
        // Unknown negative dirfd: keep the relative path (the real call
        // answers EBADF); the pre-strict behavior is preserved.
        AtRoute::Real => Ok(path.to_string()),
    }
}

/// The *at-family route resolution (roadmap 39). CRITICAL discipline:
/// `AT_FDCWD` (-100) has the `TEBAKO_FD_FLAG` bit set, so the fd branch of
/// every *at shim MUST gate on `dirfd >= 0` — a bare
/// `tebako_fd_is_embedded`-style bit test misroutes AT_FDCWD-relative
/// paths into the memfs fd table (the exact bug class that broke runtime
/// builds; pinned by `at_fdcwd_is_not_a_memfs_fd` below).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtRoute {
    /// Route this resolved path through the engine/policy.
    Routed(String),
    /// Pass the original (dirfd, path) pair to the real libc call
    /// verbatim (unknown negative dirfd — the kernel answers EBADF).
    Real,
}

/// Resolve an *at-family path. Absolute and AT_FDCWD-relative paths route
/// as given; a memfs dirfd is ENOTDIR; a valid host dirfd joins `base`
/// (its own path, resolved by the sys layer); any other negative dirfd
/// passes through untouched.
pub fn resolve_at_strict(dirfd: i32, path: &str, base: Option<PathBuf>) -> Result<AtRoute, i32> {
    if path.starts_with('/') || dirfd == libc::AT_FDCWD {
        return Ok(AtRoute::Routed(path.to_string()));
    }
    if dirfd < 0 {
        return Ok(AtRoute::Real);
    }
    if is_memfs_fd(dirfd) {
        return Err(libc::ENOTDIR);
    }
    Ok(AtRoute::Routed(match base {
        Some(b) => b.join(path).to_string_lossy().into_owned(),
        None => path.to_string(),
    }))
}

/// telldir on a memfs handle: ordinal of the entry the next readdir
/// returns (index-based cookies, the engine's contract).
pub fn vfs_telldir(id: usize) -> Result<i64, i32> {
    context().read().unwrap().telldir(id)
}

/// seekdir on a memfs handle (cookies are telldir ordinals; past-the-end
/// clamps to end-of-directory).
pub fn vfs_seekdir(id: usize, pos: i64) -> Result<(), i32> {
    context().write().unwrap().seekdir(id, pos)
}

/// rewinddir on a memfs handle: back to the first entry.
pub fn vfs_rewinddir(id: usize) -> Result<(), i32> {
    context().write().unwrap().rewinddir(id)
}

/// execve/posix_spawn of a MEMFS path (roadmap 39): materialize through
/// the engine's exec answer (`exec_materialize` — a home-layout mount
/// (the in-image manifest's `java_home` annotation) extracts WHOLE once
/// per process so the tool's self-relative data files (lib/modules,
/// lib/jvm.cfg) exist next to the binary; any other mount rides the
/// dlmap2file closure walk) and force the exec bit (zip-family backends
/// honestly report 0644). The routing is the dlopen rule: memfs → the
/// host copy; host-or-denied → the route's answer (an allowed host path
/// execs the original, a denied one fails EPERM/EROFS).
pub fn vfs_materialize_exec(path: &str) -> PathRoute<std::ffi::CString> {
    materialize_exec_route(path, false)
}

/// The posix_spawn/posix_spawnp surface (spec 25 §2, phase T2): the same
/// routing, emitted as a `spawn` trace event — the op token marks the
/// child-creating surface for the stream's process-tree regrouping.
pub fn vfs_materialize_spawn(path: &str) -> PathRoute<std::ffi::CString> {
    materialize_exec_route(path, true)
}

/// The shared exec/spawn route: the engine answers with the trace op the
/// syscall surface owns (`exec` for execve, `spawn` for posix_spawn).
fn materialize_exec_route(path: &str, spawn: bool) -> PathRoute<std::ffi::CString> {
    let answer = {
        let mut ctx = context().write().unwrap();
        if spawn {
            ctx.exec_materialize_for_spawn(path)
        } else {
            ctx.exec_materialize(path)
        }
    };
    match answer {
        Ok(host) => match ensure_exec_bit(&host) {
            Ok(()) => PathRoute::Vfs(host),
            Err(e) => PathRoute::Denied(e),
        },
        Err(e) => route_answer::<std::ffi::CString>(Err(e), path, HostAccess::Ro),
    }
}

/// OR 0111 into the materialized copy's mode (dlmap2file preserves the
/// image's perms, which may be 0644 — the kernel refuses those for exec).
fn ensure_exec_bit(host: &std::ffi::CString) -> Result<(), i32> {
    use std::os::unix::fs::PermissionsExt as _;
    let path = PathBuf::from(host.to_string_lossy().into_owned());
    let mode = std::fs::metadata(&path)
        .map_err(|e| e.raw_os_error().unwrap_or(libc::EIO))?
        .permissions()
        .mode();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode | 0o111))
        .map_err(|e| e.raw_os_error().unwrap_or(libc::EIO))
}

// ---------------------------------------------------------------------
// Shim initialization (the library constructor's payload)
// ---------------------------------------------------------------------

/// Read `TEBAKO_TFS_MOUNTS` + `TEBAKO_JAIL` and establish the namespace:
/// mount each declared image through the engine (in-process), then install
/// the host policy. Mounts come FIRST — the mount family's image read is
/// itself policy-gated once a policy is active (spec 08 §3).
///
/// Both vars absent/empty → the shim is fully inert (every intercepted
/// call passes through). Idempotent: later calls return the first result.
pub fn initialize() -> Result<(), String> {
    static RESULT: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    RESULT.get_or_init(init_inner).clone()
}

/// Build the mount for one `TEBAKO_TFS_MOUNTS` declaration. The slot form
/// (`image:slot:mount`, spec 17 §2.1) resolves the slot's byte region
/// against the file's tpkg trailer first, so a packaged payload mounts its
/// region — never the whole package file (whose trailer the format sniff
/// would reject). The established mount carries the slot identity so a
/// re-export to a spawned child keeps the slot form.
fn build_decl(d: &spec::MountDecl) -> Result<tfs::context::Mount, String> {
    let mount_error = |e: i32| {
        format!(
            "TEBAKO_TFS_MOUNTS: cannot mount {} at {}: {}",
            d.image,
            d.mount,
            errno_text(e)
        )
    };
    let Some(slot) = d.slot else {
        return tfs::mount::build_from_file(&d.image, &d.mount).map_err(mount_error);
    };
    let mut file = std::fs::File::open(&d.image)
        .map_err(|e| mount_error(e.raw_os_error().unwrap_or(libc::EIO)))?;
    match tpkg::resolve_slot_region(&mut file, slot).map_err(|e| {
        format!(
            "TEBAKO_TFS_MOUNTS: cannot mount slot {slot} of {} at {}: {e}",
            d.image, d.mount
        )
    })? {
        // Slot 0 on a bare image IS the whole file; the mount stays
        // slot-less, so the re-exported form is the two-field spelling
        // (spec 17 §2.1's emit rule).
        tpkg::SlotRegion::Whole => {
            tfs::mount::build_from_file(&d.image, &d.mount).map_err(mount_error)
        }
        tpkg::SlotRegion::Region { offset, len } => {
            let mut mount = tfs::mount::build_from_file_at(&d.image, offset, len, &d.mount)
                .map_err(mount_error)?;
            mount.slot = Some(slot);
            Ok(mount)
        }
    }
}

fn init_inner() -> Result<(), String> {
    // The trace bus (spec 25 §2): `TEBAKO_TRACE` arms the channel for ANY
    // tfs consumer — the preload delivery included; the driver is not the
    // only armer. Arm FIRST, before any mount, so the mount decisions land
    // on the stream. A failed arm is arm()'s loud-note disarm (law 1: the
    // run proceeds), never an init error. A spawned/exec'd child re-arms
    // from the inherited env at its own constructor (append-mode channel —
    // §2's re-derivation clause).
    if let Ok(path) = std::env::var(tfs::trace::TRACE_ENV) {
        if !path.trim().is_empty() {
            tfs::trace::arm(std::path::Path::new(&path));
        }
    }
    let mounts_spec = std::env::var("TEBAKO_TFS_MOUNTS").unwrap_or_default();
    if !mounts_spec.trim().is_empty() {
        let decls =
            spec::parse_mounts(&mounts_spec).map_err(|e| format!("TEBAKO_TFS_MOUNTS: {e}"))?;
        for d in &decls {
            let mount = build_decl(d)?;
            let mut guard = context().write().unwrap();
            // A repeated mount point is a UNION member, not an error:
            // the driver serializes a union as the incumbent followed by
            // its members at the same point (spec 17 §2.1) — layer the
            // later declaration over the incumbent exactly as the
            // parent's mount_union did. Decide union-vs-exclusive BEFORE
            // building: build_decl does host IO (the slot form opens the
            // package and the backend reader parses the region), and on
            // ELF targets the shim's own IO re-enters the interposed libc
            // symbols → the route layer → this same context lock, so a
            // second build_decl under the guard self-deadlocked the
            // constructor (the packed-mn linux-gnu hang, 2026-08-29).
            // mount_union/mount_checked are pure in-memory composition —
            // no IO — safe under the guard.
            let result = if guard.mount_point_taken(&mount.mount_point) {
                guard.mount_union(mount).map(|_| ())
            } else {
                guard.mount_checked(mount).map(|_| ())
            };
            result.map_err(|e| {
                format!(
                    "TEBAKO_TFS_MOUNTS: cannot mount {} at {}: {}",
                    d.image,
                    d.mount,
                    errno_text(e)
                )
            })?;
        }
    }
    let jail_spec = std::env::var("TEBAKO_JAIL").unwrap_or_default();
    if !jail_spec.trim().is_empty() {
        let spec = JailSpec::parse(&jail_spec).map_err(|e| format!("TEBAKO_JAIL: {e}"))?;
        let policy = HostPolicy::bind(spec.default, spec.mounts, spec.arg_files)
            .map_err(|e| format!("TEBAKO_JAIL: cannot bind policy: {}", errno_text(e)))?;
        // The audit-journal source label: whoever exported TEBAKO_JAIL
        // names the composition (TEBAKO_JAIL_SOURCE); a direct env install
        // is attributed to the variable itself. The journal file opens
        // HERE, before the context guard (see `tfs::journal`).
        let source = std::env::var("TEBAKO_JAIL_SOURCE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "TEBAKO_JAIL".to_string());
        let journal = if policy.never_denies() {
            None
        } else {
            tfs::journal::open_journal()
        };
        context()
            .write()
            .unwrap()
            .set_host_policy(policy.with_source(source), journal);
    }
    Ok(())
}

/// Borrowed engine error text, lossy (for init messages).
pub fn errno_text(e: i32) -> String {
    String::from_utf8_lossy(tfs::errno::strerror(e)).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole route matrix in ONE test function: the engine context
    /// (mounts, policy) is process-global, so context-touching assertions
    /// must not race parallel tests.
    #[test]
    fn route_matrix() {
        // The 2026-09-02 discipline (supersedes the 08-21 per-call wraps):
        // the WHOLE matrix runs inside ONE engine_call — the production
        // re-entrancy guard. Route calls inside then behave exactly like a
        // production shim entry: the engine's own host IO (mount reads,
        // extraction writes, the policy layer's realpath re-validation —
        // canonicalize_lenient, policy.rs) re-enters the interposed symbols
        // in-process on linux, sees IN_ENGINE armed, and forwards to the
        // host. Unguarded direct entries self-deadlock the context RwLock
        // (write held → canonicalize → interposed realpath → vfs_realpath →
        // read request) — the 08-21 and 09-02 ubuntu CI hangs; macOS test
        // binaries do not interpose in-process, so only ubuntu CI saw them.
        crate::sys::engine_call(route_matrix_body);
    }

    fn route_matrix_body() {
        // ---- fixture: a zip image in a private temp dir ----
        let dir = std::env::temp_dir().join(format!("libtfs-preload-route-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("img.zip");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zw = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);
            // Explicit directory entries: the zip backend addresses only
            // explicit "path/" dirs (C++ semantics).
            zw.add_directory("bin/", opts).unwrap();
            zw.add_directory("data/", opts).unwrap();
            zw.add_directory("dir/", opts).unwrap();
            zw.start_file("bin/tool", opts).unwrap();
            use std::io::Write as _;
            zw.write_all(b"#!/bin/false\n").unwrap();
            zw.start_file("data/secret.txt", opts.unix_permissions(0o644))
                .unwrap();
            zw.write_all(b"hello-vfs").unwrap();
            zw.start_file("dir/a.txt", opts.unix_permissions(0o644))
                .unwrap();
            zw.write_all(b"a").unwrap();
            zw.start_file("dir/b.txt", opts.unix_permissions(0o644))
                .unwrap();
            zw.write_all(b"b").unwrap();
            zw.finish().unwrap();
        }
        let mp = format!("/tfsroute{}", std::process::id());
        let m = tfs::mount::build_from_file(zip_path.to_str().unwrap(), &mp).unwrap();
        let handle = context().write().unwrap().mount_checked(m).unwrap();

        let secret = format!("{mp}/data/secret.txt");

        // ---- memfs read family ----
        let PathRoute::Vfs(fd) = vfs_open(&secret, libc::O_RDONLY) else {
            panic!("memfs open should route Vfs");
        };
        assert!(is_memfs_fd(fd));
        let mut buf = [0u8; 64];
        let n = vfs_read(fd, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello-vfs");
        assert_eq!(vfs_read(fd, &mut buf).unwrap(), 0);
        assert_eq!(vfs_lseek(fd, 0, libc::SEEK_SET).unwrap(), 0);
        let n = vfs_pread(fd, &mut buf[..4], 1).unwrap();
        assert_eq!(&buf[..n], b"ello");
        let st = vfs_fstat(fd).unwrap();
        assert_eq!(st.size, 9);
        vfs_close(fd).unwrap();
        assert_eq!(vfs_close(fd), Err(libc::EBADF));

        // ---- stat / access / opendir ----
        let PathRoute::Vfs(st) = vfs_stat(&secret) else {
            panic!("memfs stat should route Vfs");
        };
        assert_eq!(st.size, 9);
        assert_eq!(vfs_access(&secret, libc::F_OK), PathRoute::Vfs(()));
        assert_eq!(vfs_access(&secret, libc::R_OK), PathRoute::Vfs(()));
        assert_eq!(
            vfs_access(&secret, libc::W_OK),
            PathRoute::Denied(libc::EROFS)
        );
        let tool = format!("{mp}/bin/tool");
        // The zip backend honestly hardcodes perms (0o644 files / 0o755
        // dirs — ZIP does not reliably store POSIX permissions), so a
        // FILE is never X_OK but a directory (the mount root) is. (Only
        // explicit dir entries are addressable in a zip — C++ semantics —
        // so this checks the root rather than "bin".)
        assert_eq!(
            vfs_access(&tool, libc::X_OK),
            PathRoute::Denied(libc::EACCES)
        );
        assert_eq!(vfs_access(&mp, libc::X_OK), PathRoute::Vfs(()));
        assert_eq!(
            vfs_access(&secret, libc::X_OK),
            PathRoute::Denied(libc::EACCES)
        );

        // ---- realpath routing (spec 07 §8 — the JDK
        // canonicalization gate; the dogfood linux-gnu jing
        // ClassNotFound). A path the VFS HAS answers with its normalized
        // VFS spelling — never the host walk, whose symlink prefixes
        // (usrmerge /lib→usr/lib) are the leak. The re-entrancy guard
        // discipline lives at the top of route_matrix now (one
        // engine_call for the whole body); these calls are plain route
        // entries, exactly what a production shim does inside its guard.
        let PathRoute::Vfs(rp) = vfs_realpath(&secret) else {
            panic!("memfs realpath should route Vfs");
        };
        assert_eq!(rp.to_str().unwrap(), secret);
        // Lexical normalization rides (`.`/`..`/duplicate slashes).
        let PathRoute::Vfs(rp) = vfs_realpath(&format!("{mp}//data/./sub/../secret.txt")) else {
            panic!("memfs realpath with dot components should route Vfs");
        };
        assert_eq!(rp.to_str().unwrap(), secret);
        // A mount root itself canonicalizes to its own spelling.
        let PathRoute::Vfs(rp) = vfs_realpath(&format!("{mp}/")) else {
            panic!("the mount root's realpath should route Vfs");
        };
        assert_eq!(rp.to_str().unwrap(), mp);
        // Covered-but-missing stays Host (a legit host file under a `/`
        // mount must still reach the host realpath — /etc/localtime
        // discipline).
        assert_eq!(
            vfs_realpath(&format!("{mp}/data/nope.txt")),
            PathRoute::Host
        );
        // Uncovered spellings forward, and the empty path is glibc's
        // ENOENT.
        assert_eq!(vfs_realpath("/etc"), PathRoute::Host);
        assert_eq!(vfs_realpath(""), PathRoute::Denied(libc::ENOENT));

        let PathRoute::Vfs(dir_id) = vfs_opendir(&format!("{mp}/dir")) else {
            panic!("memfs opendir should route Vfs");
        };
        assert!(dir_is_embedded(dir_id));
        let mut names = Vec::new();
        while let Some(e) = vfs_readdir(dir_id).unwrap() {
            let end = e
                .d_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(e.d_name.len());
            names.push(
                e.d_name[..end]
                    .iter()
                    .map(|&c| c as u8 as char)
                    .collect::<String>(),
            );
        }
        assert_eq!(names, ["a.txt", "b.txt"]);
        vfs_closedir(dir_id).unwrap();
        assert!(!dir_is_embedded(dir_id));

        // ---- host passthrough (open policy), incl. mount-claimed-missing ----
        // These hold the context lock while the policy layer realpath
        // re-validates the host path — with realpath interposed, that
        // re-enters this crate in-process; the whole-test engine_call
        // (top of route_matrix) is what forwards that re-entry to the
        // host instead of self-deadlocking the context lock.
        assert_eq!(
            vfs_open("/etc/definitely-host", libc::O_RDONLY),
            PathRoute::Host
        );
        assert_eq!(vfs_stat("/etc/definitely-host"), PathRoute::Host);
        assert_eq!(
            vfs_open(&format!("{mp}/missing"), libc::O_RDONLY),
            PathRoute::Host,
            "a mount-claimed path absent from the image keeps the ENOENT pass-through"
        );
        assert_eq!(vfs_write_path("/tmp/libtfs-preload-route-write"), Ok(()));

        // ---- exec/spawn trace surfaces (spec 25 §2, phase T2) ----
        // execve and posix_spawn share the engine's routing answer; the
        // trace op names the syscall surface. The bus is process-global:
        // armed for this block only, disarmed right after.
        let capture = dir.join("trace.jsonl");
        assert!(tfs::trace::arm(&capture), "the bus arms");
        // The extraction's host writes re-enter the shims in-process on
        // linux — safe here because the whole test rides the one
        // engine_call at the top of route_matrix (the 2026-08-21 ubuntu
        // CI hang was the unguarded form of exactly this entry).
        let PathRoute::Vfs(exec_host) = vfs_materialize_exec(&tool) else {
            panic!("memfs exec should route Vfs");
        };
        let PathRoute::Vfs(spawn_host) = vfs_materialize_spawn(&tool) else {
            panic!("memfs spawn should route Vfs");
        };
        assert_eq!(exec_host, spawn_host, "one routing answer, two surfaces");
        assert_eq!(
            vfs_materialize_spawn("/etc/definitely-host"),
            PathRoute::Host
        );
        tfs::trace::disarm();
        let text = std::fs::read_to_string(&capture).unwrap();
        let exec_line = text
            .lines()
            .find(|l| l.contains("\"op\":\"exec\""))
            .unwrap_or_else(|| panic!("an exec event was traced: {text}"));
        assert!(exec_line.contains("\"verdict\":\"routed:"), "{exec_line}");
        let spawn_lines: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("\"op\":\"spawn\""))
            .collect();
        assert_eq!(
            spawn_lines.len(),
            2,
            "the memfs spawn and the host-passthrough spawn: {text}"
        );
        assert!(
            spawn_lines[0].contains("\"verdict\":\"routed:"),
            "{spawn_lines:?}"
        );
        assert!(
            spawn_lines[1].contains("\"verdict\":\"host\""),
            "the host fallthrough carries the exec row's host verdict: {spawn_lines:?}"
        );

        // ---- write-class gating: memfs is EROFS ----
        assert_eq!(vfs_write_path(&secret), Err(libc::EROFS));
        assert_eq!(vfs_rename("/tmp/a-x", &secret), Err(libc::EROFS));

        // ---- dir streams: rewind/tell/seek (roadmap 39) ----
        let PathRoute::Vfs(dir_id) = vfs_opendir(&format!("{mp}/dir")) else {
            panic!("memfs opendir should route Vfs");
        };
        assert!(vfs_readdir(dir_id).unwrap().is_some());
        assert_eq!(vfs_telldir(dir_id), Ok(1));
        assert!(vfs_readdir(dir_id).unwrap().is_some());
        assert_eq!(vfs_telldir(dir_id), Ok(2));
        assert!(vfs_readdir(dir_id).unwrap().is_none()); // end of stream
        vfs_rewinddir(dir_id).unwrap();
        assert_eq!(vfs_telldir(dir_id), Ok(0));
        assert!(vfs_readdir(dir_id).unwrap().is_some());
        vfs_seekdir(dir_id, 2).unwrap();
        assert!(vfs_readdir(dir_id).unwrap().is_none());
        assert_eq!(vfs_seekdir(dir_id, -1), Err(libc::EINVAL));
        vfs_closedir(dir_id).unwrap();
        assert_eq!(vfs_telldir(dir_id), Err(libc::EBADF));

        // ---- deny policy: host denied, memfs unaffected ----
        context().write().unwrap().set_host_policy(
            HostPolicy::bind(tfs::policy::PolicyDefault::Deny, vec![], vec![]).unwrap(),
            None,
        );
        assert_eq!(
            vfs_open("/etc/definitely-host", libc::O_RDONLY),
            PathRoute::Denied(libc::EPERM)
        );
        assert_eq!(
            vfs_stat("/etc/definitely-host"),
            PathRoute::Denied(libc::EPERM)
        );
        assert_eq!(
            vfs_opendir("/etc/definitely-host"),
            PathRoute::Denied(libc::EPERM)
        );
        assert_eq!(
            vfs_write_path("/tmp/libtfs-preload-route-write"),
            Err(libc::EPERM)
        );
        assert_eq!(
            vfs_dlmap("/etc/definitely-host"),
            PathRoute::Denied(libc::EPERM)
        );
        let PathRoute::Vfs(fd) = vfs_open(&secret, libc::O_RDONLY) else {
            panic!("memfs is unaffected by a deny jail (spec 08 §3)");
        };
        vfs_close(fd).unwrap();

        // ---- jail-only mode: no mounts, ENODEV routes through host_check ----
        context().write().unwrap().unmount_handle(handle).unwrap();
        assert_eq!(
            vfs_open("/etc/definitely-host", libc::O_RDONLY),
            PathRoute::Denied(libc::EPERM)
        );
        context()
            .write()
            .unwrap()
            .set_host_policy(HostPolicy::open(), None);
        assert_eq!(
            vfs_open("/etc/definitely-host", libc::O_RDONLY),
            PathRoute::Host
        );

        // ---- resolve_at ----
        assert_eq!(resolve_at(99, "/abs/p", None).unwrap(), "/abs/p");
        assert_eq!(resolve_at(libc::AT_FDCWD, "rel/p", None).unwrap(), "rel/p");
        assert_eq!(
            resolve_at(5, "rel/p", Some(PathBuf::from(&mp))).unwrap(),
            format!("{mp}/rel/p")
        );
        let flagged = 7 | TEBAKO_FD_FLAG;
        assert_eq!(resolve_at(flagged, "rel/p", None), Err(libc::ENOTDIR));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The AT_FDCWD regression pin (roadmap 39): `AT_FDCWD` is -100, whose
    /// two's-complement form has the `TEBAKO_FD_FLAG` bit set — a bare
    /// `is_memfs_fd` bit test therefore lies for every *at shim. The fd
    /// branch gates on `dirfd >= 0` instead: AT_FDCWD paths route
    /// cwd-relative, never into the memfs fd table.
    #[test]
    fn at_fdcwd_is_not_a_memfs_fd() {
        // The trap itself: the bit IS set (this is exactly what made a
        // naive `tebako_fd_is_embedded(dirfd)` branch misroute).
        assert_eq!(libc::AT_FDCWD & TEBAKO_FD_FLAG, TEBAKO_FD_FLAG);
        assert!(is_memfs_fd(libc::AT_FDCWD), "the bit test alone lies");
        // …so the *at resolution special-cases AT_FDCWD BEFORE any bit
        // test: cwd-relative routing, never ENOTDIR.
        assert_eq!(
            resolve_at_strict(libc::AT_FDCWD, "rel/x", None).unwrap(),
            AtRoute::Routed("rel/x".to_string())
        );
        // Other negative dirfds pass through to the real call (the kernel
        // answers EBADF) — never near the fd table either.
        assert_eq!(resolve_at_strict(-5, "rel/x", None).unwrap(), AtRoute::Real);
        // A genuine memfs fd is still ENOTDIR (memfs fds are regular
        // files; a memfs directory can never be fd-opened).
        let flagged = 7 | TEBAKO_FD_FLAG;
        assert_eq!(
            resolve_at_strict(flagged, "rel/x", None),
            Err(libc::ENOTDIR)
        );
        // Absolute paths ignore the dirfd entirely.
        assert_eq!(
            resolve_at_strict(libc::AT_FDCWD, "/abs/x", None).unwrap(),
            AtRoute::Routed("/abs/x".to_string())
        );
    }
}
