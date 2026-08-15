//! The boot orchestration (the safe core; [`crate::ffi`] is the thin C
//! entry over it).
//!
//! Order of operations (spec 17 + spec 18 C3): parse the handoff → mount
//! the env image (whole-file at the runtime root) → **verify the env
//! image's `/lib/tebako/layout.yaml`** (post-mount, before any
//! interpreter handoff — exit 78) → mount each payload triple in order
//! (bare files whole; package files by trailer region) → install
//! the jail policy (after the mounts — spec 08 §3) → materialize each
//! mounted image's declared `materialize:` resources into the exec
//! cache (spec 22 §4 class R) → on windows, boot-materialize every
//! co-mounted image's declared `library_aliases:` and join the
//! materialized dirs to the PATH lead (spec 22 §2.1 — the raw
//! LoadLibrary surface) → resolve and verify
//! the entry → rewrite argv. Any failure unmounts everything: never a
//! partial mount.

use std::path::Path;

use tfs::context::context;

use crate::handoff::{Handoff, ImageSource, ImageSpec, SlotRef};
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

/// The run-time mount-root override (`TEBAKO_MOUNT_ROOT`, spec 17 §1):
/// the per-platform compiled-in root is the DEFAULT, never the only
/// spelling — the root is configurable per platform (the factory bakes
/// `/__tfs__` on POSIX, `A:/t` on windows) and overridable at boot. An
/// override is form-validated here (fail-closed, exit 65) and gated on
/// the env image's layout permission post-mount (exit 78): a runtime
/// whose rbconfig cannot follow the override refuses it by name instead
/// of booting a broken interpreter.
fn effective_root(declared: &str, env: &dyn Env) -> Result<String, DriverError> {
    let Some(root) = env_var(env, "TEBAKO_MOUNT_ROOT") else {
        return Ok(declared.to_string());
    };
    let b = root.as_bytes();
    let drive_qualified =
        b.len() >= 4 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/';
    let posix_absolute = b.len() >= 2 && b[0] == b'/';
    let well_formed = (drive_qualified || posix_absolute)
        && !root.ends_with('/')
        && !root.split('/').any(|component| component == "..");
    if well_formed {
        Ok(root)
    } else {
        Err(manifest(format!(
            "TEBAKO_MOUNT_ROOT '{root}' is not a usable mount root (want an absolute path — '/…' or drive-qualified 'X:/…' — with no trailing slash and no '..')"
        )))
    }
}

pub(crate) fn errno_text(e: i32) -> String {
    String::from_utf8_lossy(tfs::errno::strerror(e)).into_owned()
}

/// Environment access, abstracted for tests (`TEBAKO_RUNTIME_IMAGE`,
/// `TEBAKO_JAIL`, `TEBAKO_JAIL_SOURCE`).
pub trait Env {
    fn var(&self, key: &str) -> Option<String>;
    /// Export into the interpreter's environment — the driver owns the
    /// handoff env (the spec 22 §6 surface: `TEBAKO_EXEC_CACHE`, the
    /// mount-discovery vars, the child-injection vars).
    fn set_var(&self, key: &str, value: &str);
}

/// The process environment (the shipped path).
pub struct ProcessEnv;

impl Env for ProcessEnv {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
    fn set_var(&self, key: &str, value: &str) {
        std::env::set_var(key, value);
    }
}

pub(crate) fn env_var(env: &dyn Env, key: &str) -> Option<String> {
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
/// the layout declaration lives INSIDE the env image (spec 18 C3), and so
/// does each payload's own manifest (spec 22 §3.2).
pub(crate) fn read_mounted_text(path: &str) -> Result<String, i32> {
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

/// The mounted image's own manifest, when readable: no manifest
/// declares nothing (plain images mount fine); a corrupt one is the
/// image lying about its self-description — a named 65. Shared by the
/// path_env bin-dir export (spec 22 §3.2) and the class-R
/// materialization (spec 22 §4).
pub(crate) fn mounted_manifest_at(
    mount: &str,
) -> Result<Option<tpkg::PayloadManifest>, DriverError> {
    let path = join_mount(mount, tpkg::PAYLOAD_MANIFEST_PATH);
    let Ok(text) = read_mounted_text(&path) else {
        return Ok(None);
    };
    tpkg::PayloadManifest::from_yaml(&text)
        .map(Some)
        .map_err(|e| {
            manifest(format!(
                "corrupt {} in the image mounted at '{mount}' ({e}) — the payload's self-description lies",
                tpkg::PAYLOAD_MANIFEST_PATH
            ))
        })
}

/// The env-image layout check (spec 18 C3): after the env image mounts
/// and BEFORE any interpreter handoff, `/lib/tebako/layout.yaml` inside
/// it is verified against this exe's expectations — fail-closed, exit 78
/// (S17 absent → era-1 refusal; S18 newer → upgrade refusal; S19
/// mount_root mismatch). The image's declared root is checked against
/// the exe's BAKED root (`baked_root`); a `TEBAKO_MOUNT_ROOT` override
/// (`effective_root` ≠ `baked_root`) additionally requires the image's
/// `mount_root_override` permission — a runtime whose rbconfig predates
/// the override era refuses by name rather than booting an interpreter
/// whose load paths point at an unmounted root. A boot without
/// `TEBAKO_RUNTIME_IMAGE` mounts no env image and has no pair to check
/// (None). On success the parsed declaration returns — its additive
/// grants drive the child injection (spec 22 §3).
fn check_env_layout(
    env: &dyn Env,
    baked_root: &str,
    effective_root: &str,
) -> Result<Option<crate::layout::ImageLayout>, DriverError> {
    let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") else {
        return Ok(None);
    };
    let path = join_mount(effective_root, crate::layout::LAYOUT_IMAGE_PATH);
    let text = read_mounted_text(&path).map_err(|_| {
        DriverError::new(
            EX_TEBAKO_LAYOUT,
            format!(
                "env image '{image}' declares no /lib/tebako/layout.yaml — pre-era image (era 1): rebuild the runtime with the current factory (spec 18 C3)"
            ),
        )
    })?;
    let declaration = crate::layout::ImageLayout::check(&text, baked_root, &image)?;
    if effective_root != baked_root && !declaration.mount_root_override {
        return Err(DriverError::new(
            EX_TEBAKO_LAYOUT,
            format!(
                "TEBAKO_MOUNT_ROOT override '{effective_root}' refused: env image '{image}' grants no mount_root_override (its rbconfig is pinned to '{baked_root}') — rebuild the runtime with the current factory"
            ),
        ));
    }
    Ok(Some(declaration))
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
    let policy =
        tfs::policy::HostPolicy::bind(spec.default, spec.mounts, spec.arg_files).map_err(|e| {
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
pub(crate) fn join_mount(mount_point: &str, entry: &str) -> String {
    format!(
        "{}/{}",
        mount_point.trim_end_matches('/'),
        entry.trim_start_matches('/')
    )
}

/// The VFS drive of a runtime root: `A:/t` → `Some("A:")`;
/// `/__tfs__` → `None` (POSIX — no drive qualification).
fn vfs_drive(runtime_root: &str) -> Option<&str> {
    let b = runtime_root.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        Some(&runtime_root[..2])
    } else {
        None
    }
}

/// Windows (spec 17 §1): a declared mount is a POSIX absolute path in
/// the VFS namespace (`/`, `/t`, `/opt/x`); on windows the
/// namespace presents on its own drive — the drive of the runtime root
/// (`A:/t`, short by owner decision: MAX_PATH headroom on every in-image
/// path). The driver therefore mounts every declared point at
/// `<drive><mount>`. Ruby's C-level path expansion re-roots
/// drive-relative paths (`/...`) onto the process cwd drive; only
/// drive-qualified paths are stable across expansion, so qualifying is
/// what keeps payload paths inside the VFS. POSIX roots carry no
/// drive: the mount is used as declared. A relative mount (the grammar
/// admits it) is never qualified.
fn qualify_mount(mount: &str, runtime_root: &str) -> String {
    match vfs_drive(runtime_root) {
        Some(drive) if mount.starts_with('/') => format!("{drive}{mount}"),
        _ => mount.to_string(),
    }
}

/// The mechanical slug of a declared mount (spec 22 §6; v2-1/20): the
/// drive qualifier dropped (`A:/tools/x` slugs like `/tools/x`),
/// alphanumerics uppercased, everything else an underscore, edge
/// underscores trimmed; the root mount slugs ROOT.
fn mount_slug(mount: &str) -> String {
    let declared = match vfs_drive(mount) {
        Some(d) => &mount[d.len()..],
        None => mount,
    };
    let slug: String = declared
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = slug.trim_matches('_');
    if trimmed.is_empty() {
        "ROOT".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The mount-discovery surface (spec 22 §6; v2-1/20): per payload image
/// this boot mounts, `TEBAKO_MOUNT_<SLUG>=<physical point>` rides the
/// handoff env — the portable way to reference a co-mounted payload's
/// files (the value is the QUALIFIED point on windows, re-rooting-proof).
/// Union members share their point and get one var; two DIFFERENT
/// physical points slugging alike is an authoring ambiguity — a named
/// error, never a silent winner.
///
/// The root mount (`/`) exports NOTHING: its slug would spell
/// `TEBAKO_MOUNT_ROOT`, which is the mount-root OVERRIDE var (spec 17
/// §1) — exporting it would make every child runtime read an override.
/// The app-at-/ flow needs no discovery var by construction (the
/// rewritten entry + `__dir__` qualify automatically — v2-1/20).
fn export_mount_vars(images: &[ImageSpec], env: &dyn Env) -> Result<(), DriverError> {
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for spec in images {
        let slug = mount_slug(&spec.mount);
        if slug == "ROOT" {
            continue;
        }
        match seen.get(&slug) {
            Some(point) if point != &spec.mount => {
                return Err(manifest(format!(
                    "mount points '{point}' and '{}' both derive TEBAKO_MOUNT_{slug} — the discovery surface is ambiguous; rename one mount",
                    spec.mount
                )));
            }
            Some(_) => continue,
            None => {
                seen.insert(slug.clone(), spec.mount.clone());
                env.set_var(&format!("TEBAKO_MOUNT_{slug}"), &spec.mount);
            }
        }
    }
    Ok(())
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
/// against (ruby: `/__tfs__` on POSIX, `A:/t` on windows — per-platform
/// baked defaults, overridable at boot via `TEBAKO_MOUNT_ROOT`; spec 17
/// §1).
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
    // The root this boot actually uses: the TEBAKO_MOUNT_ROOT override
    // when handed (form-validated here; the layout permission is gated
    // post-mount), else the exe's compiled-in value. Everything
    // downstream — the env-image mount, the drive qualification, the
    // entry resolution, the io-routing patches via tebako_mount_point —
    // sees the effective root and nothing else.
    let effective = effective_root(runtime_root, env)?;
    let baked_root = runtime_root;
    let runtime_root = effective.as_str();
    crate::ffi::set_mount_point(runtime_root);
    let mut h = Handoff::parse(argv)?;
    // The exec cache (spec 22 §6) is named before anything can
    // materialize: both boot paths below export it to the handoff env.
    crate::exec_cache::export(env);
    // Windows: qualify the declared mounts onto the VFS drive (spec 17
    // §1) before any mount/entry use — the mount table, the union-mode
    // rows, and the entry resolution all see the physical points.
    for spec in &mut h.images {
        spec.mount = qualify_mount(&spec.mount, runtime_root);
    }

    // Plain boot: no loader args at all. The interpreter runs its own
    // argv; the env image still mounts when handed (image-era standalone
    // mode), and the jail still applies.
    if h.images.is_empty() && h.entry.is_none() {
        let result = (|| {
            let mut mounted = Vec::new();
            mount_env_image(env, runtime_root, &mut mounted)?;
            let declaration = check_env_layout(env, baked_root, runtime_root)?;
            apply_jail(env)?;
            // The env image's own declared resources materialize before
            // the interpreter runs (spec 22 §4 class R — the cert case).
            crate::materialize::extract(&h.images, env, runtime_root)?;
            // Windows Class L (spec 22 §2.1): boot-materialize every
            // declared library alias and lead PATH with the
            // materialized dirs — the raw LoadLibrary surface's
            // interception-free answer.
            #[cfg(windows)]
            {
                let alias_dirs = crate::alias::extract(&h.images, env, runtime_root)?;
                crate::alias::export_path(env, &alias_dirs);
            }
            // The standalone interpreter spawns too — arm its children
            // the same way (spec 22 §3).
            crate::injection::export(env, declaration.as_ref(), runtime_root)?;
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
        let declaration = check_env_layout(env, baked_root, runtime_root)?;
        for spec in &h.images {
            mount_image(spec, &mut mounted, modes)?;
        }
        apply_jail(env)?;
        // Declared resources land in the exec cache after the mounts and
        // the jail, before any handoff (spec 22 §4 class R — Rule R3
        // fails the boot by name).
        crate::materialize::extract(&h.images, env, runtime_root)?;
        // Windows Class L (spec 22 §2.1, phase W2): boot-materialize
        // every co-mounted image's declared library aliases (the app
        // payload's included) and join the materialized dirs to the PATH
        // lead — BEFORE path_env::export prepends the §3.2 bin dirs in
        // front, so the exec surface's locked lead order stays
        // byte-stable (… launcher → bins → alias dirs → inherited).
        #[cfg(windows)]
        {
            let alias_dirs = crate::alias::extract(&h.images, env, runtime_root)?;
            crate::alias::export_path(env, &alias_dirs);
        }
        // The mounts are established — publish the discovery surface
        // (spec 22 §6; v2-1/20), arm the children (spec 22 §3), and wire
        // the dependency bins onto PATH (spec 22 §3.2 — the launcher
        // tier embeds the shim's materialized copy when one is
        // delivered, so injection runs first).
        export_mount_vars(&h.images, env)?;
        let shim_host = crate::injection::export(env, declaration.as_ref(), runtime_root)?;
        crate::path_env::export(&h.images, env, shim_host.as_deref())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vfs_drive_reads_the_root_drive() {
        assert_eq!(vfs_drive("A:/t"), Some("A:"));
        assert_eq!(vfs_drive("a:/t"), Some("a:"));
        assert_eq!(vfs_drive("A:"), Some("A:"));
        assert_eq!(vfs_drive("/__tfs__"), None);
        assert_eq!(vfs_drive("//share/x"), None);
        assert_eq!(vfs_drive("1:/x"), None);
        assert_eq!(vfs_drive(""), None);
    }

    #[test]
    fn declared_mounts_qualify_onto_the_vfs_drive() {
        // The uniform namespace (spec 17 §1): the POSIX absolute
        // namespace presents on the runtime root's drive.
        assert_eq!(qualify_mount("/", "A:/t"), "A:/");
        assert_eq!(qualify_mount("/t", "A:/t"), "A:/t");
        assert_eq!(qualify_mount("/opt/x", "A:/t"), "A:/opt/x");
    }

    #[test]
    fn posix_roots_never_qualify() {
        assert_eq!(qualify_mount("/", "/__tfs__"), "/");
        assert_eq!(qualify_mount("/__tfs__", "/__tfs__"), "/__tfs__");
        assert_eq!(qualify_mount("/opt/x", "/__tfs__"), "/opt/x");
    }

    #[test]
    fn relative_mounts_are_never_qualified() {
        assert_eq!(qualify_mount("rel", "A:/t"), "rel");
        assert_eq!(qualify_mount("rel", "/__tfs__"), "rel");
    }

    struct MapEnv(std::cell::RefCell<std::collections::HashMap<String, String>>);

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
        MapEnv(std::cell::RefCell::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ))
    }

    #[test]
    fn no_override_keeps_the_compiled_in_root() {
        assert_eq!(
            effective_root("/__tfs__", &env_with(&[])).unwrap(),
            "/__tfs__"
        );
        // An empty value is no override (the env_var filter).
        assert_eq!(
            effective_root("A:/t", &env_with(&[("TEBAKO_MOUNT_ROOT", "")])).unwrap(),
            "A:/t"
        );
    }

    #[test]
    fn a_well_formed_override_wins() {
        let env = env_with(&[("TEBAKO_MOUNT_ROOT", "/rt")]);
        assert_eq!(effective_root("/__tfs__", &env).unwrap(), "/rt");
        // The drive-qualified form (the windows spelling) is accepted and
        // its drive governs the qualification downstream.
        let env = env_with(&[("TEBAKO_MOUNT_ROOT", "B:/rt")]);
        assert_eq!(effective_root("A:/t", &env).unwrap(), "B:/rt");
    }

    #[test]
    fn a_malformed_override_is_a_named_error() {
        for bad in [
            "relative/x",
            "/",
            "A:/",
            "/trail/",
            "/has/../dot",
            "A:\\\\win",
        ] {
            let err =
                effective_root("/__tfs__", &env_with(&[("TEBAKO_MOUNT_ROOT", bad)])).unwrap_err();
            assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{bad}");
            assert!(err.message.contains("TEBAKO_MOUNT_ROOT"), "{bad}");
        }
    }

    #[test]
    fn mount_slug_is_mechanical_and_drive_neutral() {
        assert_eq!(mount_slug("/"), "ROOT");
        assert_eq!(mount_slug("A:/"), "ROOT");
        assert_eq!(mount_slug("/tools/inkscape"), "TOOLS_INKSCAPE");
        assert_eq!(mount_slug("A:/tools/inkscape"), "TOOLS_INKSCAPE");
        assert_eq!(mount_slug("/a-b/c.d"), "A_B_C_D");
        assert_eq!(mount_slug("/x/"), "X");
        assert_eq!(mount_slug("rel"), "REL");
    }

    fn image_spec(mount: &str) -> ImageSpec {
        ImageSpec {
            source: ImageSource::File(std::path::PathBuf::from("/x/img.tfs"), SlotRef::Whole),
            mount: mount.to_string(),
        }
    }

    #[test]
    fn mount_vars_export_per_image_with_the_physical_value() {
        let env = env_with(&[]);
        export_mount_vars(&[image_spec("/"), image_spec("/tools/jdk")], &env).unwrap();
        let m = env.0.borrow();
        // The root mount exports nothing — TEBAKO_MOUNT_ROOT is the
        // spec-17 mount-root override, never a discovery var.
        assert!(!m.contains_key("TEBAKO_MOUNT_ROOT"));
        assert_eq!(
            m.get("TEBAKO_MOUNT_TOOLS_JDK").map(String::as_str),
            Some("/tools/jdk")
        );
    }

    #[test]
    fn union_members_at_one_point_share_one_var() {
        let env = env_with(&[]);
        export_mount_vars(&[image_spec("/opt/x"), image_spec("/opt/x")], &env).unwrap();
        assert_eq!(
            env.0.borrow().get("TEBAKO_MOUNT_OPT_X").map(String::as_str),
            Some("/opt/x")
        );
    }

    #[test]
    fn two_different_points_slugging_alike_is_a_named_error() {
        let env = env_with(&[]);
        let err = export_mount_vars(&[image_spec("/a-b"), image_spec("/a/b")], &env).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST);
        assert!(err.message.contains("TEBAKO_MOUNT_A_B"), "{}", err.message);
        assert!(err.message.contains("/a-b"), "{}", err.message);
        assert!(err.message.contains("/a/b"), "{}", err.message);
    }
}
