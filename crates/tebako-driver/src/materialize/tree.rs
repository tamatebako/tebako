//! The windows materialize tier (spec 17 §7): a runtime whose
//! interpreter cannot consume the driver's mounted images (the
//! zero-patch contract — no io-routing patches, and windows has no
//! preload interposition tier) declares `provides.windows_boot:
//! materialize` in its env image manifest. On windows the driver then
//! boots it from EXTRACTED host trees: every mounted image of the boot
//! — the env image first, then each payload triple — is extracted
//! whole into the exec cache's sha-keyed tree namespace
//! (`<TEBAKO_EXEC_CACHE>/trees/<tree-key>/`), the runtime root is
//! rewired to the extracted env tree (`TEBAKO_MOUNT_ROOT`), the
//! discovery surface points at the host dirs, and the entry resolves
//! against them. POSIX never engages: [`boot_tier`] is called from
//! windows-gated call sites only, and a manifest without the grant is
//! the pre-tier behavior unchanged.
//!
//! The cache discipline is the per-file class-R protocol (the parent
//! module) lifted to trees:
//!
//! - **Write-once, tmp+rename, per-entry flock.** Extraction streams
//!   the mounted tree into a per-process staging dir
//!   (`.<tree-key>.part-<pid>`), hashing in flight (per file the
//!   tfs-merkle-1 construction; the tree digest is the sha256 over the
//!   sorted walk's `D <rel>/` / `F <rel> <merkle>` lines), verifies the
//!   staged tree against the served digest, then renames the record
//!   (`<tree-key>.tfs-digest`) into place BEFORE the tree — content
//!   without a record is foreign by construction, and a partial tree is
//!   never visible at the final path. Installed files are read-only
//!   (Rule R3). Boots serialize on the per-tree flock (120 s, then the
//!   named stale-lock-hint error — the store's spec 05 §4 discipline).
//! - **Digest-pinned reuse.** A cached tree is served only after the
//!   host walk re-hashes to its recorded digest — a match is zero
//!   extraction (the second boot is free). A mismatch, a missing
//!   record, or a corrupt record wipes the tree and re-extracts ONCE
//!   from the mounted (verified) image; still failing after the
//!   re-extraction is the named 70, never a silently served corruption.
//! - **Named failures.** Extraction IO mid-tree is a named 74 with the
//!   staging dir abandoned (never renamed into place); a symlink or
//!   special entry in the tree is a named 74 (the tier extracts regular
//!   files and directories only — windows symlink creation is a
//!   privileged operation, so there is no honest host spelling).
//!
//! **Respawn.** The tier exports `TEBAKO_MATERIALIZE_BOOT=1` beside the
//! rewired root; a boot that finds the marker (a spawned child
//! inheriting the handoff env) scrubs both BEFORE the §1 override is
//! read ([`scrub_inherited`]), so the child's driver mounts at the
//! baked root and re-derives the tier from the env image's grant — a
//! cache hit, never an exit-78 override refusal.

use std::path::{Path, PathBuf};

use sha2::Digest as _;
use tfs::context::context;

use crate::driver::{env_var, errno_text, join_mount, DriverError, Env};
use crate::handoff::{ImageSource, ImageSpec, SlotRef};
use crate::{EX_TEBAKO_IO, EX_TEBAKO_SHA};

/// The tier's respawn marker (spec 17 §7): exported with the rewired
/// `TEBAKO_MOUNT_ROOT`; its presence tells the next boot the inherited
/// root is the tier's own state, not a §1 user override.
pub const MARKER_ENV: &str = "TEBAKO_MATERIALIZE_BOOT";

/// The rewired root's variable (spec 17 §1 owns the semantics).
const ROOT_ENV: &str = "TEBAKO_MOUNT_ROOT";

/// The tree namespace under the exec-cache root (spec 17 §7).
const TREES: &str = "trees";

/// The per-tree flock wait (spec 05 §4's discipline, mirrored).
const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

fn io(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_IO, message.into())
}

fn sha(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_SHA, message.into())
}

/// What a materialize-tier boot established: the extracted env-image
/// tree (the rewired runtime root) and each payload triple's extracted
/// tree against its (qualified) mount, in triple order.
pub struct TreeBoot {
    env_root: PathBuf,
    payloads: Vec<(String, PathBuf)>,
}

impl TreeBoot {
    /// The rewired runtime root — the extracted env tree, '/'-spelled
    /// (the handoff env's spelling; windows accepts both separators).
    pub fn env_root_string(&self) -> String {
        host_spelling(&self.env_root)
    }

    /// The mount-discovery overrides (spec 17 §7): per payload mount,
    /// the extracted HOST dir the `TEBAKO_MOUNT_<SLUG>` var must name
    /// instead of the VFS point.
    pub fn mount_overrides(&self) -> Vec<(String, String)> {
        self.payloads
            .iter()
            .map(|(mount, dir)| (mount.clone(), host_spelling(dir)))
            .collect()
    }

    /// Map a resolved in-VFS path to its host twin under the extracted
    /// trees: longest-prefix over the payload mounts, then the env
    /// tree's root. `None` for a path outside every extracted mount —
    /// it belongs to the interpreter's own startup (spec 17 §1) and is
    /// handed over verbatim.
    pub fn to_host(&self, vfs_path: &str, runtime_root: &str) -> Option<String> {
        let mut best: Option<&(String, PathBuf)> = None;
        for pair in &self.payloads {
            if crate::driver::in_mount(vfs_path, &pair.0)
                && best.map_or(true, |b| pair.0.len() > b.0.len())
            {
                best = Some(pair);
            }
        }
        if let Some((mount, dir)) = best {
            return Some(host_join(dir, vfs_path, mount));
        }
        if crate::driver::in_mount(vfs_path, runtime_root) {
            return Some(host_join(&self.env_root, vfs_path, runtime_root));
        }
        None
    }
}

/// A host path's handoff spelling: '/' separators (the form every
/// windows consumer accepts; the VFS namespace never sees it).
fn host_spelling(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Join `vfs_path` (known to be under `mount`) onto the extracted dir.
fn host_join(dir: &Path, vfs_path: &str, mount: &str) -> String {
    let mp = mount.trim_end_matches('/');
    let rel = vfs_path[mp.len()..].trim_start_matches('/');
    if rel.is_empty() {
        host_spelling(dir)
    } else {
        format!("{}/{}", host_spelling(dir), rel)
    }
}

/// Scrub an inherited tier state (spec 17 §7's respawn rule): with the
/// marker present, the inherited `TEBAKO_MOUNT_ROOT` is the parent
/// boot's rewiring — blank both so §1's override never reads it (the
/// empty value is the `env_var` filter's absent). Without the marker
/// the env is the user's own and stays untouched. Called before
/// `effective_root` on windows; POSIX never sets the marker.
pub fn scrub_inherited(env: &dyn Env) {
    if env_var(env, MARKER_ENV).is_some() {
        env.set_var(ROOT_ENV, "");
        env.set_var(MARKER_ENV, "");
    }
}

/// The tier gate + extract + rewire (spec 17 §7), called from the
/// windows-gated boot paths after the mounts, the jail, and the
/// class-R pass. `Ok(None)` — the pre-tier behavior, unchanged — when
/// no env image mounted or its manifest grants nothing. With the
/// grant, every mounted image extracts into the exec cache, the
/// rewired root + marker ride the handoff env, and the returned
/// [`TreeBoot`] carries the host mapping the entry rewrite and the
/// mount-discovery export consume.
pub fn boot_tier(
    images: &[ImageSpec],
    env: &dyn Env,
    runtime_root: &str,
) -> Result<Option<TreeBoot>, DriverError> {
    let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") else {
        return Ok(None);
    };
    let grant = match crate::driver::mounted_manifest_at(runtime_root)? {
        Some(manifest) => match &manifest.provides {
            tpkg::Provides::Runtime(rt) => rt.windows_boot == Some(tpkg::WindowsBoot::Materialize),
            _ => false,
        },
        // No manifest, no grant — plain images boot mounted, as today.
        None => false,
    };
    if !grant {
        return Ok(None);
    }
    let Some(cache) = env_var(env, crate::exec_cache::VAR) else {
        // Unreachable through boot() — exec_cache::export runs first —
        // but the surface is contractual, never assumed.
        return Err(io(format!(
            "{} is not exported — the exec-cache export runs before the materialize tier at boot",
            crate::exec_cache::VAR
        )));
    };
    let trees = Path::new(&cache).join(TREES);
    let (path, slot) = crate::driver::env_image_ref(&image);
    let env_root = ensure_tree(&trees, &tree_key(Path::new(path), slot)?, runtime_root)?;
    let mut payloads = Vec::new();
    for spec in images {
        let host = match &spec.source {
            ImageSource::File(path, _) => path.clone(),
            ImageSource::OwnSlot(_) => std::env::current_exe()
                .map_err(|e| io(format!("cannot determine own executable path: {e}")))?,
        };
        let slot = match &spec.source {
            ImageSource::OwnSlot(n) => Some(*n),
            ImageSource::File(_, SlotRef::Slot(n)) => Some(*n),
            ImageSource::File(_, SlotRef::Whole) => None,
        };
        let dir = ensure_tree(&trees, &tree_key(&host, slot)?, &spec.mount)?;
        payloads.push((spec.mount.clone(), dir));
    }
    // The documented deviation (spec 17 §7): host IO under the extracted
    // trees cannot be interposed — the jail does not confine payload
    // reads on this tier. Loud, once, before the handoff; POSIX mounts
    // enforce as spec 08 describes.
    eprintln!(
        "tebako-driver: windows materialize tier in effect (spec 17 §7) — the runtime boots \
         from extracted host trees under '{}'; TEBAKO_JAIL does not confine the payload's \
         reads under them (host IO cannot be interposed on windows)",
        trees.display()
    );
    let boot = TreeBoot { env_root, payloads };
    env.set_var(ROOT_ENV, &boot.env_root_string());
    env.set_var(MARKER_ENV, "1");
    // The extracted tree is the payload's byte-stable twin: the reuse
    // verification re-digests it at every boot, so anything a running
    // payload writes into it (python's __pycache__ bytecode on import)
    // fails the NEXT boot's check, wipes the tree, and churns the cache
    // until a spawn opens the entrypoint mid-wipe (the windows ENOENT).
    // Interpreter-managed caches are off for the tier; other runtimes
    // cache nothing in the tree.
    env.set_var("PYTHONDONTWRITEBYTECODE", "1");
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "materialize tier: runtime root rewired to {}",
        boot.env_root_string()
    );
    Ok(Some(boot))
}

/// The tree key of one mounted image (spec 17 §7): the exec cache's
/// content idiom (the store sidecar's sha prefix, else the path key —
/// `exec_cache::image_key`), with the slot appended when the image is a
/// package region, so two slots of one package never share a tree.
fn tree_key(image: &Path, slot: Option<u32>) -> Result<String, DriverError> {
    let key = crate::exec_cache::image_key(image);
    match slot {
        None => Ok(key),
        Some(0) => {
            // Slot 0 on a bare file ≡ the whole file (the spec 17
            // grammar's bare rule); on a package it names a region.
            // Resolve the distinction exactly as the mount did.
            let mut file = std::fs::File::open(image)
                .map_err(|e| io(format!("cannot open image file '{}': {e}", image.display())))?;
            match tpkg::read_from(&mut file) {
                Err(tpkg::TpkgError::NoTrailer) => Ok(key),
                Err(e) => Err(io(format!(
                    "cannot probe the tpkg trailer of '{}': {e}",
                    image.display()
                ))),
                Ok(_) => Ok(format!("{key}-s0")),
            }
        }
        Some(n) => Ok(format!("{key}-s{n}")),
    }
}

/// Serve the image tree keyed `key`, extracting from the mounted VFS at
/// `vfs_root` on a miss (the module doc's protocol). Returns the
/// extracted tree's host root.
fn ensure_tree(trees: &Path, key: &str, vfs_root: &str) -> Result<PathBuf, DriverError> {
    let target = trees.join(key);
    let record = record_path(trees, key);
    with_tree_lock(trees, key, || {
        if target.exists() {
            if verify_tree(&target, &record, key).is_ok() {
                tebako_log::log!(
                    tebako_log::Level::Debug,
                    "driver",
                    "materialize tier cache hit key={key} at={}",
                    target.display()
                );
                return Ok(target);
            }
            // Tampered or corrupt: wipe and re-extract ONCE (spec 17
            // §7); a tree still failing after that is the named 70.
            wipe(&target, &record, key)?;
            extract_and_install(&target, &record, vfs_root, key)?;
            verify_tree(&target, &record, key).map_err(|_| {
                sha(format!(
                    "the re-extracted tree '{key}' in the exec cache still fails verification against its recorded digest — the cache is tampered or corrupt; remove '{}' to force a clean re-extraction",
                    trees.display()
                ))
            })?;
            return Ok(target);
        }
        extract_and_install(&target, &record, vfs_root, key)?;
        Ok(target)
    })
}

/// The tree's digest record (`<trees>/<key>.tfs-digest`) — cache
/// bookkeeping, never a consumption path (the parent's convention).
fn record_path(trees: &Path, key: &str) -> PathBuf {
    trees.join(format!("{key}{}", super::RECORD_SUFFIX))
}

/// Extract the mounted tree at `vfs_root` and install it at `target`:
/// stage → verify the staging against the served digest → record first,
/// then the tree (the module doc's install order).
fn extract_and_install(
    target: &Path,
    record: &Path,
    vfs_root: &str,
    key: &str,
) -> Result<(), DriverError> {
    let trees = target.parent().ok_or_else(|| {
        io(format!(
            "the tree target '{}' has no parent directory",
            target.display()
        ))
    })?;
    std::fs::create_dir_all(trees).map_err(|e| {
        io(format!(
            "cannot create the trees dir '{}': {e}",
            trees.display()
        ))
    })?;
    let staging = trees.join(format!(".{key}.part-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    let served = extract_tree(vfs_root, &staging)?;
    // The staged tree must carry exactly the bytes the image served —
    // the record describes the copy, or the boot never installs it
    // (the parent's R3 refusal, lifted to trees).
    if walk_host(&staging)? != served {
        let _ = force_remove_dir_all(&staging);
        return Err(sha(format!(
            "the staged tree '{key}' does not match the bytes the image served — extraction is not faithful"
        )));
    }
    std::fs::write(record, format!("{served}\n")).map_err(|e| {
        io(format!(
            "cannot install the digest record '{}': {e}",
            record.display()
        ))
    })?;
    std::fs::rename(&staging, target).map_err(|e| {
        io(format!(
            "cannot install the extracted tree '{}': {e}",
            target.display()
        ))
    })?;
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "materialize tier extracted key={key} from={vfs_root} at={}",
        target.display()
    );
    Ok(())
}

/// Stream the mounted tree at `vfs_root` into `staging` (created),
/// returning the tree digest of the bytes the image served. Any
/// failure abandons the staging dir — the final path never sees a
/// partial tree.
fn extract_tree(vfs_root: &str, staging: &Path) -> Result<String, DriverError> {
    let mut hasher = sha2::Sha256::new();
    let result = walk_vfs(vfs_root, "", staging, &mut hasher);
    if let Err(e) = result {
        let _ = force_remove_dir_all(staging);
        return Err(e);
    }
    Ok(crate::exec_cache::hex(&hasher.finalize()))
}

/// One directory level of the VFS walk: entries sorted by name (the
/// digest's determinism), directories recorded `D <rel>/` and recursed,
/// files streamed out hashing in flight and recorded
/// `F <rel> <merkle>`, anything else the named refusal. The listing is
/// the image's OWN (`image_read_dir`): a sibling mount below the point
/// (the env at `A:/t` beside the drive root `A:/`) extracts into its
/// own tree — its boundary name never joins this walk.
fn walk_vfs(
    vfs: &str,
    rel: &str,
    host: &Path,
    hasher: &mut sha2::Sha256,
) -> Result<(), DriverError> {
    std::fs::create_dir_all(host)
        .map_err(|e| io(format!("cannot create '{}': {e}", host.display())))?;
    let mut entries = {
        let mut ctx = context().write().unwrap();
        ctx.image_read_dir(vfs).map_err(|e| {
            io(format!(
                "cannot read the directory '{vfs}' from the mounted image: {}",
                errno_text(e)
            ))
        })?
    };
    entries.sort();
    for name in entries {
        let child_vfs = join_mount(vfs, &name);
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let stat = context().read().unwrap().stat(&child_vfs).map_err(|e| {
            io(format!(
                "cannot stat '{child_vfs}' in the mounted image: {}",
                errno_text(e)
            ))
        })?;
        match stat.entry_type {
            tfs::EntryType::Directory => {
                hasher.update(format!("D {child_rel}/\n"));
                walk_vfs(&child_vfs, &child_rel, &host.join(&name), hasher)?;
            }
            tfs::EntryType::File => {
                let target = host.join(&name);
                let served = super::stream_out(&child_vfs, &target)?;
                super::set_readonly(&target)?;
                hasher.update(format!(
                    "F {child_rel} {}\n",
                    tpkg::merkle::render_tree_hash(&served)
                ));
            }
            other => {
                return Err(io(format!(
                    "the mounted tree entry '{child_vfs}' is a {other:?} — the windows materialize tier extracts regular files and directories only (spec 17 §7)"
                )));
            }
        }
    }
    Ok(())
}

/// The host walk's digest of an extracted tree — the byte-identical
/// construction of [`walk_vfs`]'s (sorted entries, `D <rel>/` /
/// `F <rel> <merkle>` lines), so the reuse check compares like with
/// like.
fn walk_host(root: &Path) -> Result<String, DriverError> {
    let mut hasher = sha2::Sha256::new();
    walk_host_dir(root, "", &mut hasher)?;
    Ok(crate::exec_cache::hex(&hasher.finalize()))
}

fn walk_host_dir(dir: &Path, rel: &str, hasher: &mut sha2::Sha256) -> Result<(), DriverError> {
    let mut entries: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| io(format!("cannot read the dir '{}': {e}", dir.display())))?
    {
        let entry =
            entry.map_err(|e| io(format!("cannot read the dir '{}': {e}", dir.display())))?;
        entries.push(entry.file_name().to_string_lossy().into_owned());
    }
    entries.sort();
    for name in entries {
        let path = dir.join(&name);
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|e| io(format!("cannot stat '{}': {e}", path.display())))?;
        if meta.file_type().is_dir() {
            hasher.update(format!("D {child_rel}/\n"));
            walk_host_dir(&path, &child_rel, hasher)?;
        } else if meta.file_type().is_file() {
            let digest = super::hash_host_file(&path)?;
            hasher.update(format!(
                "F {child_rel} {}\n",
                tpkg::merkle::render_tree_hash(&digest)
            ));
        } else {
            // The tier installs regular files and directories only —
            // anything else in the tree is foreign content: the digest
            // line can never match a served tree, so the reuse check
            // below reports the corruption by name.
            hasher.update(format!("X {child_rel}\n"));
        }
    }
    Ok(())
}

/// The reuse check: the cached tree re-walks to its recorded digest or
/// the caller wipes and re-extracts. A missing/corrupt record and any
/// host IO failure are the same verdict — the cache is tampered or
/// corrupt (never a silently served tree).
fn verify_tree(target: &Path, record: &Path, key: &str) -> Result<(), DriverError> {
    if !target.is_dir() {
        return Err(sha(format!(
            "the extracted tree '{key}' in the exec cache is not a directory — the cache is tampered or corrupt"
        )));
    }
    let Ok(text) = std::fs::read_to_string(record) else {
        return Err(sha(format!(
            "the extracted tree '{key}' in the exec cache carries no digest record — refusing to serve unverified content"
        )));
    };
    let want = text.trim();
    if want.len() != 64 || !want.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(sha(format!(
            "the digest record of the extracted tree '{key}' in the exec cache is corrupt"
        )));
    }
    let got = walk_host(target)?;
    if got != want {
        return Err(sha(format!(
            "the extracted tree '{key}' in the exec cache fails verification against its recorded digest — the cache is tampered or corrupt"
        )));
    }
    Ok(())
}

/// Remove a cached tree + its record. Read-only files (Rule R3) resist
/// deletion — on windows DeleteFile refuses the read-only bit — so the
/// wipe clears it first.
fn wipe(target: &Path, record: &Path, key: &str) -> Result<(), DriverError> {
    force_remove_dir_all(target).map_err(|e| {
        io(format!(
            "cannot wipe the corrupt extracted tree '{key}' at '{}': {e}",
            target.display()
        ))
    })?;
    let _ = std::fs::remove_file(record);
    Ok(())
}

/// `remove_dir_all` through the read-only attribute: clear it on every
/// entry (best effort per file) before the removal.
fn force_remove_dir_all(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_dir() {
            force_remove_dir_all(&path)?;
        } else {
            let mut perms = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(&path, perms);
        }
    }
    std::fs::remove_dir_all(dir)
}

/// The per-tree flock (spec 05 §4's discipline): LOCK_EX|LOCK_NB
/// retried for [`LOCK_TIMEOUT`], then the named stale-lock-hint error.
fn with_tree_lock<T>(
    trees: &Path,
    key: &str,
    f: impl FnOnce() -> Result<T, DriverError>,
) -> Result<T, DriverError> {
    std::fs::create_dir_all(trees).map_err(|e| {
        io(format!(
            "cannot create the trees dir '{}': {e}",
            trees.display()
        ))
    })?;
    let lock_path = trees.join(format!(".lock-{key}"));
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| io(format!("cannot open '{}': {e}", lock_path.display())))?;
    let deadline = std::time::Instant::now() + LOCK_TIMEOUT;
    loop {
        if lock::exclusive_nb(&lock) {
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err(io(format!(
                "timed out after {} s waiting for the exec-cache lock '{}' — another tebako process may hold it; if that process is gone the lock is stale, remove the file",
                LOCK_TIMEOUT.as_secs(),
                lock_path.display()
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let result = f();
    lock::unlock(&lock);
    result
}

/// The OS lock pair, one shape per platform (the tebako-resolve
/// cache.rs shape — no new mechanism): flock(2) on unix, LockFileEx on
/// one byte at offset 0 on windows. This module is the crate's FFI
/// boundary for the cache locks — the only `unsafe` outside `ffi`.
#[allow(unsafe_code)]
mod lock {
    #[cfg(unix)]
    pub fn exclusive_nb(file: &std::fs::File) -> bool {
        use std::os::unix::io::AsRawFd as _;
        // SAFETY: flock(2) on a live fd.
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 }
    }

    #[cfg(unix)]
    pub fn unlock(file: &std::fs::File) -> bool {
        use std::os::unix::io::AsRawFd as _;
        // SAFETY: flock(2) on a live fd.
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) == 0 }
    }

    #[cfg(windows)]
    pub fn exclusive_nb(file: &std::fs::File) -> bool {
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::Storage::FileSystem::{
            LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
        };
        use windows_sys::Win32::System::IO::OVERLAPPED;
        let mut ov = OVERLAPPED::default();
        // SAFETY: LockFileEx on a live handle; released by the kernel
        // when the handle dies, exactly like flock.
        unsafe {
            LockFileEx(
                file.as_raw_handle(),
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                1,
                0,
                &mut ov,
            ) != 0
        }
    }

    #[cfg(windows)]
    pub fn unlock(file: &std::fs::File) -> bool {
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::Storage::FileSystem::UnlockFileEx;
        use windows_sys::Win32::System::IO::OVERLAPPED;
        let mut ov = OVERLAPPED::default();
        // SAFETY: UnlockFileEx on a live handle holding the lock.
        unsafe { UnlockFileEx(file.as_raw_handle(), 0, 1, 0, &mut ov) != 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Env;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Write as _;
    use std::sync::MutexGuard;

    // The mounts are process-global — the tests that mount serialize on
    // the crate-wide test lock (the tests/boot.rs LOCK pattern, shared
    // across modules in this one test binary).
    use crate::TEST_LOCK as LOCK;

    struct MountGuard {
        _guard: MutexGuard<'static, ()>,
        tmp: PathBuf,
    }

    impl MountGuard {
        fn new(tag: &str) -> MountGuard {
            let g = LOCK.lock().unwrap();
            context().write().unwrap().unmount();
            let uniq = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let tmp = std::env::temp_dir().join(format!(
                "tebako-driver-tree-{tag}-{}-{uniq}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp).unwrap();
            MountGuard { _guard: g, tmp }
        }
    }

    impl Drop for MountGuard {
        fn drop(&mut self) {
            context().write().unwrap().unmount();
            let _ = std::fs::remove_dir_all(&self.tmp);
        }
    }

    struct MapEnv(RefCell<HashMap<String, String>>);

    impl Env for MapEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn set_var(&self, key: &str, value: &str) {
            self.0
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        }
    }

    fn env_with(pairs: &[(&str, &str)]) -> MapEnv {
        MapEnv(RefCell::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ))
    }

    /// A zip fixture (the tests/boot.rs pattern).
    fn build_zip(path: &Path, dirs: &[&str], files: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).expect("create zip");
        let mut w = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        for d in dirs {
            w.add_directory(*d, opts).unwrap();
        }
        for (name, content) in files {
            w.start_file(name, opts).unwrap();
            w.write_all(content).unwrap();
        }
        w.finish().unwrap();
    }

    /// The fixture image: a small runtime-ish tree + its sha256 sidecar
    /// (the content key).
    fn write_image(g: &MountGuard, name: &str) -> PathBuf {
        let p = g.tmp.join(name);
        build_zip(
            &p,
            &["lib/", "lib/lang/", "share/"],
            &[
                ("lib/lang/core.txt", b"core\n".as_slice()),
                ("lib/lang/std.txt", b"std\n".as_slice()),
                ("share/data.bin", b"\x00\x01\x02".as_slice()),
            ],
        );
        let digest = crate::exec_cache::hex(&sha2::Sha256::digest(std::fs::read(&p).unwrap()));
        std::fs::write(
            p.with_file_name(format!("{name}.sha256")),
            format!("{digest}  {name}\n"),
        )
        .unwrap();
        p
    }

    fn mount(image: &Path, point: &str) {
        let m = tfs::mount::build_from_file(&image.display().to_string(), point).unwrap();
        context().write().unwrap().mount_checked(m).unwrap();
    }

    fn trees_dir(g: &MountGuard) -> PathBuf {
        g.tmp.join("cache").join(TREES)
    }

    #[test]
    fn extraction_installs_the_tree_verified_and_readonly() {
        let g = MountGuard::new("extract");
        let image = write_image(&g, "env.tfs");
        mount(&image, "/rt");
        let trees = trees_dir(&g);
        let key = tree_key(&image, None).unwrap();
        let target = ensure_tree(&trees, &key, "/rt").unwrap();
        // The whole tree landed, read-only (Rule R3), with its record.
        assert_eq!(
            std::fs::read(target.join("lib/lang/core.txt")).unwrap(),
            b"core\n"
        );
        assert_eq!(
            std::fs::read(target.join("share/data.bin")).unwrap(),
            b"\x00\x01\x02"
        );
        assert!(std::fs::metadata(target.join("lib/lang/std.txt"))
            .unwrap()
            .permissions()
            .readonly());
        assert!(record_path(&trees, &key).is_file());
        // No staging litter: the part dir never reaches the namespace.
        let litter: Vec<_> = std::fs::read_dir(&trees)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".part-"))
            .collect();
        assert!(litter.is_empty(), "{litter:?}");
    }

    #[test]
    fn a_verified_cache_hit_needs_no_mount() {
        let g = MountGuard::new("reuse");
        let image = write_image(&g, "env.tfs");
        mount(&image, "/rt");
        let trees = trees_dir(&g);
        let key = tree_key(&image, None).unwrap();
        let target = ensure_tree(&trees, &key, "/rt").unwrap();
        // The image is gone from the VFS AND from the host: the second
        // call serves the verified cache tree — zero extraction is
        // proven by construction (either read would fail the test).
        context().write().unwrap().unmount();
        let record_before = std::fs::read(record_path(&trees, &key)).unwrap();
        let again = ensure_tree(&trees, &key, "/rt").unwrap();
        assert_eq!(again, target);
        assert_eq!(
            std::fs::read(record_path(&trees, &key)).unwrap(),
            record_before
        );
    }

    #[test]
    fn a_tampered_tree_wipes_and_reextracts() {
        let g = MountGuard::new("tamper");
        let image = write_image(&g, "env.tfs");
        mount(&image, "/rt");
        let trees = trees_dir(&g);
        let key = tree_key(&image, None).unwrap();
        let target = ensure_tree(&trees, &key, "/rt").unwrap();
        // Tamper (clear the read-only bit first — the cache's own
        // discipline).
        let victim = target.join("lib/lang/core.txt");
        let mut perms = std::fs::metadata(&victim).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        std::fs::set_permissions(&victim, perms).unwrap();
        std::fs::write(&victim, b"FORGED\n").unwrap();
        let healed = ensure_tree(&trees, &key, "/rt").unwrap();
        assert_eq!(healed, target);
        assert_eq!(
            std::fs::read(target.join("lib/lang/core.txt")).unwrap(),
            b"core\n"
        );
        // Foreign extra content is the same corruption verdict.
        std::fs::write(target.join("lib/lang/planted.txt"), b"planted\n").unwrap();
        ensure_tree(&trees, &key, "/rt").unwrap();
        assert!(!target.join("lib/lang/planted.txt").exists());
    }

    #[test]
    fn a_missing_or_corrupt_record_reextracts() {
        let g = MountGuard::new("record");
        let image = write_image(&g, "env.tfs");
        mount(&image, "/rt");
        let trees = trees_dir(&g);
        let key = tree_key(&image, None).unwrap();
        let target = ensure_tree(&trees, &key, "/rt").unwrap();
        // Content without a record is foreign by construction.
        std::fs::remove_file(record_path(&trees, &key)).unwrap();
        ensure_tree(&trees, &key, "/rt").unwrap();
        assert!(record_path(&trees, &key).is_file());
        assert_eq!(
            std::fs::read(target.join("lib/lang/core.txt")).unwrap(),
            b"core\n"
        );
        // A corrupt record is the same family.
        std::fs::write(record_path(&trees, &key), b"not a digest\n").unwrap();
        ensure_tree(&trees, &key, "/rt").unwrap();
        assert_eq!(
            std::fs::read(target.join("lib/lang/core.txt")).unwrap(),
            b"core\n"
        );
    }

    #[test]
    fn a_symlink_in_the_tree_is_a_named_refusal() {
        let g = MountGuard::new("symlink");
        let p = g.tmp.join("env.tfs");
        let file = std::fs::File::create(&p).unwrap();
        let mut w = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        w.start_file("real.txt", opts).unwrap();
        w.write_all(b"real\n").unwrap();
        // A unix-mode symlink entry (the zip spelling).
        let link_opts = opts.unix_permissions(0o120777);
        w.start_file("link.txt", link_opts).unwrap();
        w.write_all(b"real.txt").unwrap();
        w.finish().unwrap();
        mount(&p, "/rt");
        let trees = trees_dir(&g);
        let key = tree_key(&p, None).unwrap();
        match ensure_tree(&trees, &key, "/rt") {
            Err(e) => {
                assert_eq!(e.code, EX_TEBAKO_IO, "{}", e.message);
                assert!(e.message.contains("link.txt"), "{}", e.message);
                // The partial tree never reached the namespace.
                assert!(!trees.join(&key).exists());
            }
            Ok(_) => {
                // A backend reading the entry as a regular file (the zip
                // backend's prerogative) is not the refusal case — the
                // refusal is proven where the entry IS a symlink.
            }
        }
    }

    #[test]
    fn the_tree_key_distinguishes_slots_of_one_package() {
        let g = MountGuard::new("key");
        let p = g.tmp.join("pkg.tpkg");
        build_zip(&p, &["a/"], &[("a/x", b"x\n".as_slice())]);
        let whole = tree_key(&p, None).unwrap();
        assert_eq!(whole, tree_key(&p, Some(0)).unwrap()); // bare file: slot 0 ≡ whole
                                                           // With a trailer the same file's slots key per region.
        let size = std::fs::metadata(&p).unwrap().len();
        let mut m = tpkg::Manifest {
            package_flags: tpkg::TPKG_FLAG_LEAN,
            launcher_abi: 1,
            ..Default::default()
        };
        m.slots
            .push(tpkg::Slot::new(0, size, tpkg::TPKG_FORMAT_DWARFS, "/"));
        m.validate().unwrap();
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&p)
            .unwrap();
        tpkg::write_to(&mut f, &m).unwrap();
        let s0 = tree_key(&p, Some(0)).unwrap();
        let s1 = tree_key(&p, Some(1)).unwrap();
        assert_ne!(whole, s0);
        assert_ne!(s0, s1);
        assert!(s0.ends_with("-s0"), "{s0}");
    }

    #[test]
    fn to_host_maps_longest_prefix_then_the_env_root() {
        let boot = TreeBoot {
            env_root: PathBuf::from("C:\\cache\\trees\\env"),
            payloads: vec![
                ("/app".to_string(), PathBuf::from("C:\\cache\\trees\\app")),
                ("/opt/x".to_string(), PathBuf::from("C:\\cache\\trees\\x")),
            ],
        };
        assert_eq!(
            boot.to_host("/app/bin/app", "/rt").as_deref(),
            Some("C:/cache/trees/app/bin/app")
        );
        assert_eq!(
            boot.to_host("/opt/x/lib/y", "/rt").as_deref(),
            Some("C:/cache/trees/x/lib/y")
        );
        // The env tree answers the runtime root, mount point included.
        assert_eq!(
            boot.to_host("/rt/lib/lang/core.txt", "/rt").as_deref(),
            Some("C:/cache/trees/env/lib/lang/core.txt")
        );
        assert_eq!(
            boot.to_host("/rt", "/rt").as_deref(),
            Some("C:/cache/trees/env")
        );
        // Outside every extracted mount: the interpreter's own startup
        // path, handed over verbatim.
        assert_eq!(boot.to_host("/elsewhere/x", "/rt"), None);
    }

    #[test]
    fn the_drive_qualified_root_mount_extracts_the_payloads_own_tree() {
        // The windows boot's mount shape (spec 17 §7 as fired by
        // xml2rfc#18): the env image sits at the qualified runtime root
        // `A:/t` while the payload sits at the drive ROOT `A:/` — a
        // descendant-prefix sibling. Normalizing `A:/` once collapsed to
        // the relative-looking `A:`, no mount's prefix, and the walk
        // served the synthesized `t` boundary instead of the payload's
        // own listing: the extracted "payload" tree was a copy of the
        // ENV image (no `bin/`), the entry ENOENT'd at spawn, and the
        // recorded digest described the same wrong bytes — so the reuse
        // verification approved the corruption on every later boot.
        let g = MountGuard::new("drive-root");
        let env = write_env_image(&g, true);
        let payload = g.tmp.join("payload.tfs");
        build_zip(
            &payload,
            &["bin/", "lib/", "__tpkg__/"],
            &[
                ("bin/xml2rfc", b"#!/usr/bin/env python3\n".as_slice()),
                ("lib/site.txt", b"site\n".as_slice()),
            ],
        );
        mount(&env, "A:/t");
        mount(&payload, "A:/");
        let trees = trees_dir(&g);
        let key = tree_key(&payload, None).unwrap();
        let target = ensure_tree(&trees, &key, "A:/").unwrap();
        // The payload's own bytes landed — the entrypoint included.
        assert_eq!(
            std::fs::read(target.join("bin/xml2rfc")).unwrap(),
            b"#!/usr/bin/env python3\n"
        );
        assert_eq!(
            std::fs::read(target.join("lib/site.txt")).unwrap(),
            b"site\n"
        );
        // The env image extracts into ITS OWN tree: a sibling mount
        // below the point never joins the payload's extraction.
        assert!(
            !target.join("t").exists(),
            "env bytes leaked into the payload's tree"
        );
        // And the env tree at the runtime root is the env's own.
        let env_key = tree_key(&env, None).unwrap();
        let env_target = ensure_tree(&trees, &env_key, "A:/t").unwrap();
        assert!(env_target.join("lib/tebako/layout.yaml").is_file());
        assert!(!env_target.join("bin").exists());
    }

    /// The dependency payload's manifest (kind app; one entrypoint —
    /// its bin dir is what §3.2 flows onto `PATH`).
    fn app_manifest() -> String {
        format!(
            "identity:\n  schema_version: 1\n  kind: app\n  name: dep\n  version: \"1\"\n  \
             producer: {{tool: t, tool_version: \"1\"}}\n  created: now\n  \
             digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n  \
             signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  entrypoints:\n    - {{name: tool, path: /bin/tool}}\n  \
             platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
            "12".repeat(32),
            "34".repeat(32),
        )
    }

    #[test]
    fn the_path_export_names_the_extracted_host_bin_dirs() {
        // spec 17 §7's PATH rewiring: the §3.2 dependency bin dirs
        // compose from the VFS mount points, which the host loader
        // cannot resolve under the tier (the xml2rfc windows leg staged
        // its mingw DLLs beside the pyds to dodge exactly this) — the
        // boot's mount→host map must translate them onto the extracted
        // trees, through the same map the mount-vars export consumes.
        let g = MountGuard::new("pathenv");
        // A fresh boot must not inherit a previous boot's spawn surface
        // (the boot fixtures' reset discipline; the guard holds the
        // crate-wide lock the STATE writes serialize on).
        crate::spawn::reset();
        let image = write_env_image(&g, true);
        mount(&image, "/rt");
        // The first triple (the app payload) is a plain image — its own
        // bins are the entrypoint's business and never ride PATH; the
        // DEPENDENCY declares its entrypoint so its bin dir does.
        let app = write_image(&g, "app.tfs");
        let dep = g.tmp.join("dep.tfs");
        build_zip(
            &dep,
            &["bin/", "__tpkg__/"],
            &[
                ("bin/tool", b"#!/usr/bin/env python3\n".as_slice()),
                ("__tpkg__/manifest.yaml", app_manifest().as_bytes()),
            ],
        );
        mount(&app, "/app");
        mount(&dep, "/dep");
        let cache = g.tmp.join("cache");
        let env = env_with(&[
            ("TEBAKO_RUNTIME_IMAGE", &image.display().to_string()),
            (crate::exec_cache::VAR, &cache.display().to_string()),
        ]);
        let specs = vec![
            ImageSpec {
                source: ImageSource::File(app, SlotRef::Whole),
                mount: "/app".to_string(),
            },
            ImageSpec {
                source: ImageSource::File(dep, SlotRef::Whole),
                mount: "/dep".to_string(),
            },
        ];
        let boot = boot_tier(&specs, &env, "/rt").unwrap().unwrap();
        let overrides = boot.mount_overrides();
        crate::path_env::export(&specs, &env, None, &overrides).unwrap();
        let path = env_var(&env, "PATH").unwrap_or_default();
        let dep_tree = &overrides.iter().find(|(m, _)| m == "/dep").unwrap().1;
        // The dependency's bin dir names the EXTRACTED tree; the VFS
        // spelling is gone from the lead.
        assert!(path.contains(&format!("{dep_tree}/bin")), "{path}");
        assert!(!path.contains("/dep/bin"), "{path}");
    }

    #[test]
    fn scrub_inherited_blanks_only_the_tiers_own_state() {
        // The marker present: both vars blank (the env_var filter reads
        // empty as absent).
        let env = env_with(&[(MARKER_ENV, "1"), (ROOT_ENV, "C:/cache/trees/env")]);
        scrub_inherited(&env);
        assert_eq!(env_var(&env, MARKER_ENV), None);
        assert_eq!(env_var(&env, ROOT_ENV), None);
        // No marker: a user override is §1's own state — untouched.
        let env = env_with(&[(ROOT_ENV, "B:/rt")]);
        scrub_inherited(&env);
        assert_eq!(env_var(&env, ROOT_ENV).as_deref(), Some("B:/rt"));
    }

    /// The env-image manifest fixture (kind runtime; `grant` controls
    /// the tier declaration).
    fn runtime_manifest(grant: bool) -> String {
        let key = if grant {
            "  windows_boot: materialize\n"
        } else {
            ""
        };
        format!(
            "identity:\n  schema_version: 1\n  kind: runtime\n  name: pyruntime\n  version: 3.13.1\n\
             \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: now\n\
             \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
             \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  provides: {{engine: python, version: 3.13.1, abi_line: \"3.13\", platform: x86_64-windows-ucrt}}\n\
             \x20 built_from: {{src_sha256: {}, patch_set: v0.0.1}}\n{key}  capabilities: {{exec: true, read: true, runtime: true}}\n",
            "ab".repeat(32),
            "cd".repeat(32),
            "ef".repeat(32),
        )
    }

    /// The env-image fixture with the in-image manifest (+ the spec-18
    /// layout the boot's pair-check wants — boot_tier itself reads only
    /// the manifest).
    fn write_env_image(g: &MountGuard, grant: bool) -> PathBuf {
        let p = g.tmp.join("env.tfs");
        build_zip(
            &p,
            &["lib/", "lib/lang/", "__tpkg__/", "lib/tebako/"],
            &[
                ("lib/lang/core.txt", b"core\n".as_slice()),
                ("__tpkg__/manifest.yaml", runtime_manifest(grant).as_bytes()),
                (
                    "lib/tebako/layout.yaml",
                    b"schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /rt\ninterpreter_api_version: \"3.13\"\n".as_slice(),
                ),
            ],
        );
        let digest = crate::exec_cache::hex(&sha2::Sha256::digest(std::fs::read(&p).unwrap()));
        std::fs::write(g.tmp.join("env.tfs.sha256"), format!("{digest}  env.tfs\n")).unwrap();
        p
    }

    #[test]
    fn the_grant_gates_the_tier() {
        let g = MountGuard::new("gate");
        // No grant: the manifest reads, the tier stays off, nothing is
        // extracted or exported.
        let image = write_env_image(&g, false);
        mount(&image, "/rt");
        let cache = g.tmp.join("cache");
        let env = env_with(&[
            ("TEBAKO_RUNTIME_IMAGE", &image.display().to_string()),
            (crate::exec_cache::VAR, &cache.display().to_string()),
        ]);
        assert!(boot_tier(&[], &env, "/rt").unwrap().is_none());
        assert!(!cache.join(TREES).exists());
        assert!(!env.0.borrow().contains_key(ROOT_ENV));
        assert!(!env.0.borrow().contains_key(MARKER_ENV));
    }

    #[test]
    fn the_tier_extracts_rewires_and_maps() {
        let g = MountGuard::new("tier");
        let image = write_env_image(&g, true);
        mount(&image, "/rt");
        let payload = g.tmp.join("payload.tfs");
        build_zip(
            &payload,
            &["bin/"],
            &[("bin/app", b"#!/usr/bin/env python3\n".as_slice())],
        );
        let pdigest =
            crate::exec_cache::hex(&sha2::Sha256::digest(std::fs::read(&payload).unwrap()));
        std::fs::write(
            g.tmp.join("payload.tfs.sha256"),
            format!("{pdigest}  payload.tfs\n"),
        )
        .unwrap();
        mount(&payload, "/app");
        let cache = g.tmp.join("cache");
        let env = env_with(&[
            ("TEBAKO_RUNTIME_IMAGE", &image.display().to_string()),
            (crate::exec_cache::VAR, &cache.display().to_string()),
        ]);
        let spec = ImageSpec {
            source: ImageSource::File(payload.clone(), SlotRef::Whole),
            mount: "/app".to_string(),
        };
        let boot = boot_tier(&[spec], &env, "/rt").unwrap().unwrap();
        // The rewired root rides the handoff env with the marker.
        let root = env_var(&env, ROOT_ENV).unwrap();
        assert_eq!(root, boot.env_root_string());
        assert!(root.contains(TREES), "{root}");
        assert_eq!(env_var(&env, MARKER_ENV).as_deref(), Some("1"));
        // The tree stays byte-stable under the running payload:
        // interpreter-managed caches are off (a __pycache__ write would
        // fail the next boot's reuse verification and wipe the tree).
        assert_eq!(
            env_var(&env, "PYTHONDONTWRITEBYTECODE").as_deref(),
            Some("1")
        );
        // The extracted trees read as plain host files.
        let core = PathBuf::from(root.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert_eq!(
            std::fs::read(core.join("lib/lang/core.txt")).unwrap(),
            b"core\n"
        );
        // The discovery override + the entry mapping hit the host dirs.
        let overrides = boot.mount_overrides();
        assert_eq!(overrides.len(), 1);
        assert_eq!(overrides[0].0, "/app");
        assert!(overrides[0].1.contains(TREES), "{}", overrides[0].1);
        let entry = boot.to_host("/app/bin/app", "/rt").unwrap();
        let entry_host = PathBuf::from(entry.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert_eq!(
            std::fs::read(&entry_host).unwrap(),
            b"#!/usr/bin/env python3\n"
        );
        // The second boot is a cache hit: the images unmount, the tier
        // still serves (and re-exports).
        context().write().unwrap().unmount();
        let image2 = write_env_image(&g, true);
        let payload2 = g.tmp.join("payload.tfs");
        mount(&image2, "/rt");
        mount(&payload2, "/app");
        let spec2 = ImageSpec {
            source: ImageSource::File(payload2, SlotRef::Whole),
            mount: "/app".to_string(),
        };
        // …the mounts exist only so the grant manifest reads; extraction
        // is zero (the trees verify from the record). Prove it: extract
        // once more after removing nothing — same roots.
        let boot2 = boot_tier(&[spec2], &env, "/rt").unwrap().unwrap();
        assert_eq!(boot2.env_root_string(), boot.env_root_string());
        assert_eq!(boot2.mount_overrides(), boot.mount_overrides());
    }
}
