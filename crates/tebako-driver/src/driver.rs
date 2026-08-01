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

/// Where an image's bytes come from after trailer probing.
enum ResolvedImage {
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
            SlotRef::Whole | SlotRef::Slot(0) => Ok(ResolvedImage::Whole),
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
                Ok(ResolvedImage::Region(s.offset, s.size))
            }
        },
    }
}

fn mount_built(built: Result<tfs::context::Mount, i32>, what: &str) -> Result<(), DriverError> {
    let mount = built.map_err(|e| {
        if e == libc::ENOENT {
            unavailable(format!("{what}: {}", errno_text(e)))
        } else {
            io(format!("{what}: {}", errno_text(e)))
        }
    })?;
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

/// The env image (`TEBAKO_RUNTIME_IMAGE`): a bare `.tfs`, mounted whole
/// at the runtime root. Records `runtime_root` in `mounted`.
fn mount_env_image(
    env: &dyn Env,
    runtime_root: &str,
    mounted: &mut Vec<String>,
) -> Result<(), DriverError> {
    let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") else {
        return Ok(());
    };
    mount_built(
        tfs::mount::build_from_file(&image, runtime_root),
        &format!("failed to mount the runtime filesystem image from '{image}'"),
    )?;
    mounted.push(runtime_root.to_string());
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
    mounted: &mut Vec<String>,
) -> Result<(), DriverError> {
    match &spec.source {
        ImageSource::OwnSlot(n) => {
            let exe = std::env::current_exe()
                .map_err(|e| io(format!("cannot determine own executable path: {e}")))?;
            let display = exe.display().to_string();
            match resolve_image(&exe, SlotRef::Slot(*n), &display)? {
                ResolvedImage::Whole => Err(manifest(
                    "the running executable carries no tpkg trailer — <self> slots require a stitched package",
                )),
                ResolvedImage::Region(offset, size) => {
                    mount_built(
                        tfs::mount::build_from_file_at(&display, offset, size, &spec.mount),
                        &format!("failed to mount own slot {n} at '{}'", spec.mount),
                    )?;
                    mounted.push(spec.mount.clone());
                    Ok(())
                }
            }
        }
        ImageSource::File(path, slot) => {
            let display = path.display().to_string();
            match resolve_image(path, *slot, &display)? {
                ResolvedImage::Whole => {
                    mount_built(
                        tfs::mount::build_from_file(&display, &spec.mount),
                        &format!("failed to mount image '{display}' at '{}'", spec.mount),
                    )?;
                }
                ResolvedImage::Region(offset, size) => {
                    mount_built(
                        tfs::mount::build_from_file_at(&display, offset, size, &spec.mount),
                        &format!("failed to mount image '{display}' at '{}'", spec.mount),
                    )?;
                }
            }
            mounted.push(spec.mount.clone());
            Ok(())
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
    mounted: &[String],
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
    if mounted.iter().any(|mp| in_mount(&resolved, mp)) {
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
        let mut mounted: Vec<String> = Vec::new();
        mount_env_image(env, runtime_root, &mut mounted)?;
        // The env image's pair-check runs post-mount, before any payload
        // or interpreter touch (spec 18 C3 — exit 78).
        check_env_layout(env, runtime_root)?;
        for spec in &h.images {
            mount_image(spec, &mut mounted)?;
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
