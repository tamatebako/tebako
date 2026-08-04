//! The boot orchestration (the safe core; [`crate::ffi`] is the thin C
//! entry over it).
//!
//! Order of operations (spec 17 + spec 18 C3): parse the handoff → mount
//! the env image (whole-file at the runtime root) → **verify the env
//! image's `/lib/tebako/layout.yaml`** (post-mount, before any
//! interpreter handoff — exit 78) → mount each payload triple in order
//! (bare files whole; package files by trailer region) → install
//! the jail policy (after the mounts — spec 08 §3) → resolve and verify
//! the entry → rewrite argv. Any failure unmounts everything: never a
//! partial mount.

use std::path::Path;

use tfs::context::context;

use crate::handoff::{Handoff, ImageSource, SlotRef};
use crate::{
    EX_TEBAKO_IO, EX_TEBAKO_JAIL, EX_TEBAKO_LAYOUT, EX_TEBAKO_MANIFEST, EX_TEBAKO_UNAVAILABLE,
};

/// A named driver failure carrying the loader's exit code (spec 06 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverError {
    pub code: i32,
    pub message: String,
}

impl DriverError {
    pub fn new(code: i32, message: String) -> DriverError {
        DriverError { code, message }
    }
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DriverError {}

fn manifest(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_MANIFEST, message.into())
}

fn unavailable(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_UNAVAILABLE, message.into())
}

fn io(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_IO, message.into())
}

fn jail(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_JAIL, message.into())
}

fn errno_text(e: i32) -> String {
    String::from_utf8_lossy(tfs::errno::strerror(e)).into_owned()
}

/// Environment access, abstracted for tests (`TEBAKO_RUNTIME_IMAGE`,
/// `TEBAKO_JAIL`, `TEBAKO_JAIL_SOURCE`).
pub trait Env {
    fn var(&self, key: &str) -> Option<String>;
}

/// The process environment (the shipped path).
pub struct ProcessEnv;

impl Env for ProcessEnv {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

fn env_var(env: &dyn Env, key: &str) -> Option<String> {
    env.var(key).filter(|s| !s.is_empty())
}

/// What [`boot`] hands back: the rewritten argv
/// (`[<original argv0>, <entry resolved in the VFS>, <user args…>]`,
/// or the input argv unchanged for a plain boot). The program name
/// stays at index 0: the interpreter parses its argv conventionally
/// and takes the entry as the script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootOutcome {
    pub argv: Vec<String>,
}

/// The mount-mode source (spec 17 §1, locked 2026-08-04): mount
/// semantics ride the running package's OWN trailer (the `<self>`
/// manifest block — spelled `self` or as the package's path, the same
/// file), never the argv grammar — the launcher ABI is unchanged and
/// drivers predating this refuse a union package loudly (EEXIST).
/// Consulted only when a triple's mount point is already occupied;
/// `trailer` is the one [`resolve_image`] already parsed from the
/// triple's image file, and the answer is the L2 `mounts:` row for the
/// triple's slot.
pub trait MountModes {
    fn row_for(
        &self,
        spec: &crate::handoff::ImageSpec,
        trailer: Option<&tpkg::Manifest>,
    ) -> Result<Option<tpkg::PackageMount>, DriverError>;
}

/// The shipped source: the L2 `mounts:` block of the trailer the
/// triple's own image file carries. A bare image (no trailer), a
/// whole-file mount, a package without the block, and a slot without a
/// row are all spellings of "exclusive" — payloads handed over without
/// a package manifest (shim dispatch, bare images) are always
/// exclusive (spec 17 §1).
pub struct OwnTrailer;

impl MountModes for OwnTrailer {
    fn row_for(
        &self,
        spec: &crate::handoff::ImageSpec,
        trailer: Option<&tpkg::Manifest>,
    ) -> Result<Option<tpkg::PackageMount>, DriverError> {
        let slot = match &spec.source {
            ImageSource::OwnSlot(n) => *n,
            ImageSource::File(_, SlotRef::Slot(n)) => *n,
            // A whole-file mount is a bare image by construction (a
            // packaged file's `-` is resolve_image's named error
            // already) — nothing declares its mode.
            ImageSource::File(_, SlotRef::Whole) => return Ok(None),
        };
        let Some(trailer) = trailer else {
            return Ok(None);
        };
        let package = trailer.package_manifest().map_err(|e| {
            manifest(format!(
                "invalid L2 package manifest in the mounted package: {e}"
            ))
        })?;
        Ok(package.and_then(|p| p.mounts.into_iter().find(|row| row.slot == slot)))
    }
}

/// Where an image's bytes come from after trailer probing, plus the
/// parsed trailer when the file is a package (the mount-mode source's
/// input — spec 17 §1).
struct ResolvedImage {
    /// The region to mount.
    region: ResolvedRegion,
    /// The package's trailer, when the image file carries one.
    trailer: Option<tpkg::Manifest>,
}

/// The mount region of a probed image.
enum ResolvedRegion {
    /// A bare file, mounted whole (slot `0` ≡ `-`).
    Whole,
    /// A package file's slot region (offset, size).
    Region(u64, u64),
}

/// Probe a file's tpkg trailer and resolve the slot reference.
fn resolve_image(path: &Path, slot: SlotRef, display: &str) -> Result<ResolvedImage, DriverError> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| unavailable(format!("cannot open image file '{display}': {e}")))?;
    match tpkg::read_from(&mut file) {
        Err(tpkg::TpkgError::NoTrailer) => match slot {
            SlotRef::Whole | SlotRef::Slot(0) => Ok(ResolvedImage {
                region: ResolvedRegion::Whole,
                trailer: None,
            }),
            SlotRef::Slot(n) => Err(manifest(format!(
                "--tebako-image slot {n} is out of range for '{display}' (a bare image file — no slot table; use slot 0 or -)"
            ))),
        },
        Err(e) => Err(manifest(format!(
            "corrupt tpkg manifest trailer in '{display}' ({e}) — re-stitch the package"
        ))),
        Ok(m) => match slot {
            SlotRef::Whole => Err(manifest(format!(
                "--tebako-image slot - names a whole bare image, but '{display}' is a package ({} slot(s)) — use a numeric slot",
                m.slots.len()
            ))),
            SlotRef::Slot(n) => {
                let Some(s) = m.slots.get(n as usize) else {
                    return Err(manifest(format!(
                        "--tebako-image slot {n} is out of range for '{display}' ({} slot(s) in its manifest)",
                        m.slots.len()
                    )));
                };
                if s.format_id == tpkg::TPKG_FORMAT_RUNTIME {
                    return Err(manifest(format!(
                        "--tebako-image slot {n} of '{display}' is a runtime payload slot — payload slots are never mounted"
                    )));
                }
                Ok(ResolvedImage {
                    region: ResolvedRegion::Region(s.offset, s.size),
                    trailer: Some(m),
                })
            }
        },
    }
}

/// One established member of the boot's mount table: the point and a
/// human-readable member description for the union journal (spec 17 §1:
/// the union set — point + members + precedence — is journaled at boot).
struct MountedMember {
    point: String,
    desc: String,
}

/// Map a backend-construction failure into the named boot error
/// (ENOENT is the unavailable image; everything else is IO).
fn build_error(
    built: Result<tfs::context::Mount, i32>,
    what: &str,
) -> Result<tfs::context::Mount, DriverError> {
    built.map_err(|e| {
        if e == libc::ENOENT {
            unavailable(format!("{what}: {}", errno_text(e)))
        } else {
            io(format!("{what}: {}", errno_text(e)))
        }
    })
}

/// The exclusive mount path (the historical behavior): a free point is
/// claimed, an occupied point is the named EEXIST error.
fn mount_exclusive(mount: tfs::context::Mount, what: &str) -> Result<(), DriverError> {
    let mount_point = mount.mount_point.clone();
    context()
        .write()
        .unwrap()
        .mount_checked(mount)
        .map_err(|e| {
            if e == libc::EEXIST {
                manifest(format!("{what}: duplicate mount point"))
            } else {
                io(format!("{what}: {}", errno_text(e)))
            }
        })?;
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "mounted what={what} at={mount_point}"
    );
    Ok(())
}

/// Mount one payload triple's image at its point (spec 17 §1 mount
/// modes): a free point is claimed exclusively; an occupied point is
/// governed by the L2 `mounts:` row the mode source answers for the
/// triple's slot — `union` merges the image over the incumbents with
/// the declared precedence (journaled at boot), anything else (no row,
/// `exclusive`) is the named EEXIST error it always was.
fn mount_at_point(
    mount: tfs::context::Mount,
    spec: &crate::handoff::ImageSpec,
    trailer: Option<&tpkg::Manifest>,
    what: &str,
    new_desc: &str,
    modes: &dyn MountModes,
    mounted: &mut Vec<MountedMember>,
) -> Result<(), DriverError> {
    if !context().read().unwrap().mount_point_taken(&spec.mount) {
        mount_exclusive(mount, what)?;
        mounted.push(MountedMember {
            point: spec.mount.clone(),
            desc: new_desc.to_string(),
        });
        return Ok(());
    }
    let Some(row) = modes.row_for(spec, trailer)? else {
        return Err(manifest(format!("{what}: duplicate mount point")));
    };
    match row.mode {
        tpkg::MountMode::Union => {
            context()
                .write()
                .unwrap()
                .mount_union(mount)
                .map_err(|e| io(format!("{what}: {}", errno_text(e))))?;
            let precedence = row
                .precedence
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_string());
            let incumbents = mounted
                .iter()
                .filter(|m| m.point == spec.mount)
                .map(|m| m.desc.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            tebako_log::log!(
                tebako_log::Level::Debug,
                "driver",
                "union mount point={} members=[{}; {}] precedence={}",
                spec.mount,
                incumbents,
                new_desc,
                precedence
            );
            if !mounted
                .iter()
                .any(|m| m.point == spec.mount && m.desc == new_desc)
            {
                mounted.push(MountedMember {
                    point: spec.mount.clone(),
                    desc: new_desc.to_string(),
                });
            }
            Ok(())
        }
        // `exclusive` — and every spelling the L2 validation let through
        // without a row — keeps the named EEXIST error. The reserved
        // `cow`/`enc` spellings never reach here: the L2 block fails
        // tpkg's validation and the mode source answered with the named
        // error already.
        _ => Err(manifest(format!("{what}: duplicate mount point"))),
    }
}

/// The env image (`TEBAKO_RUNTIME_IMAGE`): a bare `.tfs`, mounted whole
/// at the runtime root. Records `runtime_root` in `mounted`.
fn mount_env_image(
    env: &dyn Env,
    runtime_root: &str,
    mounted: &mut Vec<MountedMember>,
) -> Result<(), DriverError> {
    let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") else {
        return Ok(());
    };
    mount_exclusive(
        build_error(
            tfs::mount::build_from_file(&image, runtime_root),
            &format!("failed to mount the runtime filesystem image from '{image}'"),
        )?,
        &format!("failed to mount the runtime filesystem image from '{image}'"),
    )?;
    mounted.push(MountedMember {
        point: runtime_root.to_string(),
        desc: format!("env image '{image}'"),
    });
    Ok(())
}

/// Read a small text file through the mounted VFS (never the host fs) —
/// the layout declaration lives INSIDE the env image (spec 18 C3).
fn read_mounted_text(path: &str) -> Result<String, i32> {
    let mut ctx = context().write().unwrap();
    let fd = ctx.open(path, libc::O_RDONLY)?;
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = match ctx.read(fd, &mut buf) {
            Ok(n) => n,
            Err(e) => {
                let _ = ctx.close(fd);
                return Err(e);
            }
        };
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
        if out.len() > crate::layout::LAYOUT_MAX_BYTES {
            let _ = ctx.close(fd);
            return Err(libc::EFBIG);
        }
    }
    ctx.close(fd)?;
    String::from_utf8(out).map_err(|_| libc::EINVAL)
}

/// The env-image layout check (spec 18 C3): after the env image mounts
/// and BEFORE any interpreter handoff, `/lib/tebako/layout.yaml` inside
/// it is verified against this exe's expectations — fail-closed, exit 78
/// (S17 absent → era-1 refusal; S18 newer → upgrade refusal; S19
/// mount_root mismatch). A boot without `TEBAKO_RUNTIME_IMAGE` mounts no
/// env image and has no pair to check.
fn check_env_layout(env: &dyn Env, runtime_root: &str) -> Result<(), DriverError> {
    let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") else {
        return Ok(());
    };
    let path = join_mount(runtime_root, crate::layout::LAYOUT_IMAGE_PATH);
    let text = read_mounted_text(&path).map_err(|_| {
        DriverError::new(
            EX_TEBAKO_LAYOUT,
            format!(
                "env image '{image}' declares no /lib/tebako/layout.yaml — pre-era image (era 1): rebuild the runtime with the current factory (spec 18 C3)"
            ),
        )
    })?;
    crate::layout::ImageLayout::check(&text, runtime_root, &image).map(|_| ())
}

fn mount_image(
    spec: &crate::handoff::ImageSpec,
    mounted: &mut Vec<MountedMember>,
    modes: &dyn MountModes,
) -> Result<(), DriverError> {
    match &spec.source {
        ImageSource::OwnSlot(n) => {
            let exe = std::env::current_exe()
                .map_err(|e| io(format!("cannot determine own executable path: {e}")))?;
            let display = exe.display().to_string();
            let resolved = resolve_image(&exe, SlotRef::Slot(*n), &display)?;
            match resolved.region {
                ResolvedRegion::Whole => Err(manifest(
                    "the running executable carries no tpkg trailer — <self> slots require a stitched package",
                )),
                ResolvedRegion::Region(offset, size) => {
                    let what = format!("failed to mount own slot {n} at '{}'", spec.mount);
                    let mount = build_error(
                        tfs::mount::build_from_file_at(&display, offset, size, &spec.mount),
                        &what,
                    )?;
                    mount_at_point(
                        mount,
                        spec,
                        resolved.trailer.as_ref(),
                        &what,
                        &format!("own slot {n}"),
                        modes,
                        mounted,
                    )
                }
            }
        }
        ImageSource::File(path, slot) => {
            let display = path.display().to_string();
            let resolved = resolve_image(path, *slot, &display)?;
            let what = format!("failed to mount image '{display}' at '{}'", spec.mount);
            let mount = match resolved.region {
                ResolvedRegion::Whole => {
                    build_error(tfs::mount::build_from_file(&display, &spec.mount), &what)?
                }
                ResolvedRegion::Region(offset, size) => build_error(
                    tfs::mount::build_from_file_at(&display, offset, size, &spec.mount),
                    &what,
                )?,
            };
            mount_at_point(
                mount,
                spec,
                resolved.trailer.as_ref(),
                &what,
                &format!("'{display}'"),
                modes,
                mounted,
            )
        }
    }
}

/// `TEBAKO_JAIL` → the host policy, installed AFTER the mounts (spec 08
/// §3: the mount family's image read is itself policy-gated once a
/// policy is active). Malformed policy fails closed (exit 73).
fn apply_jail(env: &dyn Env) -> Result<(), DriverError> {
    let Some(spec_str) = env_var(env, "TEBAKO_JAIL") else {
        return Ok(());
    };
    let spec =
        tfs::policy::JailSpec::parse(&spec_str).map_err(|e| jail(format!("TEBAKO_JAIL: {e}")))?;
    let policy = tfs::policy::HostPolicy::bind(spec.default_open, spec.mounts, spec.arg_files)
        .map_err(|e| {
            jail(format!(
                "TEBAKO_JAIL: cannot bind policy: {}",
                errno_text(e)
            ))
        })?;
    let source = env_var(env, "TEBAKO_JAIL_SOURCE").unwrap_or_else(|| "TEBAKO_JAIL".to_string());
    let journal = if policy.never_denies() {
        None
    } else {
        tfs::journal::open_journal()
    };
    context()
        .write()
        .unwrap()
        .set_host_policy(policy.with_source(source), journal);
    Ok(())
}

fn in_mount(path: &str, mount_point: &str) -> bool {
    let mp = mount_point.trim_end_matches('/');
    mp.is_empty() || path == mp || path.starts_with(&format!("{mp}/"))
}

/// Join the entry onto its mount: mount `/` + `/bin/app` → `/bin/app`;
/// mount `/opt` + `bin/app` → `/opt/bin/app`.
fn join_mount(mount_point: &str, entry: &str) -> String {
    format!(
        "{}/{}",
        mount_point.trim_end_matches('/'),
        entry.trim_start_matches('/')
    )
}

/// Resolve the entry against the first image's mount (the app payload —
/// spec 17 §1) and verify it exists in the mounted tree — but only
/// against mounts THIS boot established (an entry outside them belongs
/// to the interpreter's own startup, not to the handoff).
fn resolve_entry(
    h: &Handoff,
    runtime_root: &str,
    mounted: &[MountedMember],
) -> Result<String, DriverError> {
    let entry = h
        .entry
        .as_deref()
        .ok_or_else(|| manifest("--tebako-entry is required when --tebako-image is given"))?;
    let base = h
        .images
        .first()
        .map(|i| i.mount.as_str())
        .unwrap_or(runtime_root);
    let resolved = join_mount(base, entry);
    if mounted.iter().any(|m| in_mount(&resolved, &m.point)) {
        let mut ctx = context().write().unwrap();
        match ctx.open(&resolved, libc::O_RDONLY) {
            Ok(fd) => {
                let _ = ctx.close(fd);
            }
            Err(_) => {
                return Err(manifest(format!(
                    "entrypoint '{entry}' not found at '{resolved}' in the mounted tree"
                )));
            }
        }
    }
    Ok(resolved)
}

/// The boot (spec 17 §1–§3). `argv` is the process argv WITHOUT argv[0]
/// semantics preserved: the parser scans from index 0 (callers pass the
/// full argv; non-loader leading args end the scan — the plain-boot
/// case). `runtime_root` is the mount point the interpreter was compiled
/// against (ruby: `/__tfs__`, `A:/t` on windows).
pub fn boot(
    argv: &[String],
    runtime_root: &str,
    env: &dyn Env,
) -> Result<BootOutcome, DriverError> {
    boot_with_mount_modes(argv, runtime_root, env, &OwnTrailer)
}

/// The boot with an explicit mount-mode source — the shipped [`boot`]
/// reads the modes from the running package's OWN trailer ([`OwnTrailer`],
/// spec 17 §1); the tests substitute a stub. The argv grammar is
/// identical either way: mount semantics never ride the handoff.
pub fn boot_with_mount_modes(
    argv: &[String],
    runtime_root: &str,
    env: &dyn Env,
    modes: &dyn MountModes,
) -> Result<BootOutcome, DriverError> {
    let h = Handoff::parse(argv)?;

    // Plain boot: no loader args at all. The interpreter runs its own
    // argv; the env image still mounts when handed (image-era standalone
    // mode), and the jail still applies.
    if h.images.is_empty() && h.entry.is_none() {
        let result = (|| {
            let mut mounted = Vec::new();
            mount_env_image(env, runtime_root, &mut mounted)?;
            check_env_layout(env, runtime_root)?;
            apply_jail(env)?;
            Ok(BootOutcome {
                argv: argv.to_vec(),
            })
        })();
        if result.is_err() {
            context().write().unwrap().unmount();
        }
        return result;
    }

    let result = (|| {
        let mut mounted: Vec<MountedMember> = Vec::new();
        mount_env_image(env, runtime_root, &mut mounted)?;
        // The env image's pair-check runs post-mount, before any payload
        // or interpreter touch (spec 18 C3 — exit 78).
        check_env_layout(env, runtime_root)?;
        for spec in &h.images {
            mount_image(spec, &mut mounted, modes)?;
        }
        apply_jail(env)?;
        let rewritten = match h.entry.as_deref() {
            // No entry: the interpreter starts with its own args (the
            // bare `--tebako-image` invocation — the deploy-driver
            // smoke; v1 behavior).
            None => {
                let mut v = Vec::with_capacity(h.interpreter_args.len() + 1);
                if let Some(program) = argv.first() {
                    v.push(program.clone());
                }
                v.extend(h.interpreter_args.iter().cloned());
                v
            }
            // The interpreter keyword (a bare name, never a path): the
            // CLI's deploy shims re-enter the interpreter itself
            // (`--tebako-entry ruby`); the keyword is dropped.
            Some(keyword) if !keyword.contains('/') => {
                let mut v = Vec::with_capacity(h.user_args.len() + 1);
                if let Some(program) = argv.first() {
                    v.push(program.clone());
                }
                v.extend(h.user_args.iter().cloned());
                v
            }
            Some(_) => {
                let resolved = resolve_entry(&h, runtime_root, &mounted)?;
                // The rewritten argv keeps the interpreter's convention:
                // argv[0] (the program name) first, then the resolved
                // entry as the script, then the user's args verbatim.
                // Dropping argv[0] makes the interpreter treat the entry
                // as its own name and the first user arg as the script.
                let mut v = Vec::with_capacity(h.user_args.len() + 2);
                if let Some(program) = argv.first() {
                    v.push(program.clone());
                }
                v.push(resolved);
                v.extend(h.user_args.iter().cloned());
                v
            }
        };
        Ok(BootOutcome { argv: rewritten })
    })();
    if result.is_err() {
        // Never a partial mount (spec 17 §1): one bad slot aborts the
        // whole namespace.
        context().write().unwrap().unmount();
    }
    result
}
