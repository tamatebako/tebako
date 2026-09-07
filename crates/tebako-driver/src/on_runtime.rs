//! Runtime-on-runtime composition (spec 33) — the driver side. The
//! loader composes the handoff (the OWNER's exe receives the wire, its
//! env image rides `TEBAKO_RUNTIME_IMAGE`, the depending runtime's env
//! image leads the `--tebako-image` triples at its declared mount); the
//! driver discovers the composition from that first triple's mounted
//! manifest (the `on_runtime` block), never from a new wire token:
//!
//! 1. [`discover`] reads the first triple's in-image manifest after the
//!    mounts: a `kind: runtime` payload carrying `provides.on_runtime`
//!    is the depending env image. The shard-lie cross-check fires here —
//!    the loader composed the triple from the release-index mirror
//!    (`on_runtime.mount`), and the mounted manifest is the authority: a
//!    declared mount that does not qualify to the triple's point is the
//!    release lying, a named 65, never a guessed-around composition.
//! 2. The `{mount}` placeholder expands to the effective (qualified)
//!    mount; an unknown placeholder or a post-expansion escape of the
//!    mount is a named boot error 65 (spec 33 §2).
//! 3. The entry surface (spec 33 §1/§3): a path entry resolves against
//!    the first triple AFTER the depending image (ordinarily the app
//!    payload), or against the depending mount itself on the self-boot
//!    smoke form; a bare name resolves against the DEPENDING runtime's
//!    `provides.entrypoints` — the owner's own entrypoints stay
//!    spawn-only — and entry-directed `args_default` come from the
//!    depending runtime's manifest (the single-owner rule), never from
//!    the app payload's. There is NO direct-program form on this boot:
//!    every entry composes through the template.
//!
//! The argv the boot composes is `[argv0, template…, args_default…,
//! entry, user args…]` (spec 33 §3); the wrapper tail then swaps in the
//! owner's materialized interpreter and bridges the template's VFS
//! tokens plus the entry under exec-cache.

use tpkg::Provides;

use crate::driver::{
    join_mount, manifest, mounted_manifest_at, qualify_mount, DriverError, MountedMember,
};
use crate::handoff::ImageSpec;

/// The discovered composition (spec 33 §2's block, boot-resolved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnRuntime {
    /// The depending env image's effective mount (drive-qualified on
    /// windows) — the `{mount}` expansion value.
    pub mount: String,
    /// The expanded `argv_template` (placeholder-expanded, validated).
    pub template: Vec<String>,
}

/// The BootOutcome mirror of the discovery: what the wrapper tail needs
/// to compose and bridge around the boot's rewritten argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnRuntimeMeta {
    /// The expanded template's length — argv indices `1..=template_len`
    /// are the template tokens.
    pub template_len: usize,
    /// The depending env image's effective mount (the `{mount}`
    /// expansion value) — the template bridge's splice key.
    pub mount: String,
}

/// Discover the composition from the FIRST triple's mounted manifest
/// (spec 33 §1: the depending env image rides first). `Ok(None)` on
/// every non-composition shape — no triples, an unreadable/absent
/// manifest, a non-runtime payload, a runtime without the block (every
/// runtime predating this spec); a corrupt manifest stays the named 65
/// it is everywhere. Runs after the mounts (the manifest is read through
/// the VFS) and before the entry resolution.
pub(crate) fn discover(
    images: &[ImageSpec],
    runtime_root: &str,
) -> Result<Option<OnRuntime>, DriverError> {
    let Some(first) = images.first() else {
        return Ok(None);
    };
    let Some(doc) = mounted_manifest_at(&first.mount)? else {
        return Ok(None);
    };
    let Provides::Runtime(rt) = &doc.provides else {
        return Ok(None);
    };
    let Some(on) = &rt.on_runtime else {
        return Ok(None);
    };
    // The shard-lie cross-check (spec 33 §1): the in-image declaration
    // is the authority; the triple's point is the loader's composition
    // from the release-index mirror. A disagreement is the release lying.
    let declared = qualify_mount(&on.mount, runtime_root);
    if declared != first.mount {
        return Err(manifest(format!(
            "the depending runtime's on_runtime.mount declares '{}' but its env image is mounted at '{}' — the release index's on_runtime mirror contradicts the in-image manifest (spec 33 §1); the release is lying",
            on.mount, first.mount
        )));
    }
    let template = expand(&on.argv_template, &first.mount)?;
    Ok(Some(OnRuntime {
        mount: first.mount.clone(),
        template,
    }))
}

/// Expand `{mount}` against the effective mount and validate the result
/// (spec 33 §2): an unknown placeholder (any surviving brace) or a
/// mount-prefixed element escaping the mount through `..` is a named
/// boot error 65 naming the template. Elements not addressing the mount
/// (flags, class names) pass verbatim.
fn expand(template: &[String], mount: &str) -> Result<Vec<String>, DriverError> {
    let mut out = Vec::with_capacity(template.len());
    for token in template {
        let expanded = token.replace("{mount}", mount);
        if let Some(start) = expanded.find('{') {
            let end = expanded[start..]
                .find('}')
                .map(|i| start + i + 1)
                .unwrap_or(start + 1);
            let placeholder = &expanded[start..end.min(expanded.len())];
            return Err(manifest(format!(
                "on_runtime.argv_template names an unknown placeholder '{placeholder}' — the single placeholder is {{mount}} (spec 33 §2)"
            )));
        }
        if expanded.contains('}') {
            return Err(manifest(format!(
                "on_runtime.argv_template element '{token}' carries a stray '}}' — the single placeholder is {{mount}} (spec 33 §2)"
            )));
        }
        if let Some(rest) = expanded.strip_prefix(mount) {
            if rest.split('/').any(|c| c == "..") {
                return Err(manifest(format!(
                    "on_runtime.argv_template element '{token}' escapes the depending runtime's mount '{mount}' after expansion — a named boot error, never a host path (spec 33 §2)"
                )));
            }
        }
        out.push(expanded);
    }
    Ok(out)
}

/// The entry-resolution base (spec 33 §1's amendment): the first triple
/// after the depending image — ordinarily the app payload — or, with no
/// payload triples, the depending runtime's own mount (the self-boot
/// smoke form).
pub(crate) fn entry_base<'m>(on: &'m OnRuntime, app_images: &'m [ImageSpec]) -> &'m str {
    app_images
        .first()
        .map(|i| i.mount.as_str())
        .unwrap_or(&on.mount)
}

/// The bare-name surface (spec 33 §3): `name` resolves against the
/// DEPENDING runtime's `provides.entrypoints`, verified against the
/// mounted tree; the owner's own entrypoints are never consulted (they
/// stay spawn-only through spec 30 edges). Returns the resolved VFS path
/// and the declaration's `args_default`.
pub(crate) fn dep_entrypoint(
    name: &str,
    on: &OnRuntime,
    mounted: &[MountedMember],
) -> Result<(String, Vec<String>), DriverError> {
    let doc = mounted_manifest_at(&on.mount)?.ok_or_else(|| {
        manifest(format!(
            "--tebako-entry '{name}' names a depending-runtime entrypoint but the manifest at '{}' is gone — the composition's discovery read it moments ago (spec 33 §3)",
            on.mount
        ))
    })?;
    let Provides::Runtime(rt) = &doc.provides else {
        return Err(manifest(format!(
            "--tebako-entry '{name}': the image mounted at '{}' is not the depending runtime (spec 33 §3)",
            on.mount
        )));
    };
    let ep = rt
        .entrypoints
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| {
            manifest(format!(
                "--tebako-entry '{name}': the depending runtime declares no entrypoint of that name — on a runtime-on-runtime boot the bare-name surface is the depending runtime's (spec 33 §3); the owner's own entrypoints stay spawnable through spec 30 edges"
            ))
        })?;
    let resolved = join_mount(&on.mount, &ep.path);
    if mounted
        .iter()
        .any(|m| crate::driver::in_mount(&resolved, &m.point))
    {
        let mut ctx = tfs::context::context().write().unwrap();
        match ctx.open(&resolved, libc::O_RDONLY) {
            Ok(fd) => {
                let _ = ctx.close(fd);
            }
            Err(_) => {
                return Err(manifest(format!(
                    "depending-runtime entrypoint '{name}' resolves to '{resolved}' but the path is absent from the mounted tree — the image's declaration lies (spec 33 §3)"
                )));
            }
        }
    }
    Ok((resolved, ep.args_default.clone()))
}

/// The path-form entry's `args_default` (spec 33 §3 — entry-directed,
/// the depending runtime's manifest the single owner): the dep
/// runtime's entrypoint declaring that path, else none (never an
/// error).
pub(crate) fn dep_args_default(on: &OnRuntime, entry: &str) -> Result<Vec<String>, DriverError> {
    let Some(doc) = mounted_manifest_at(&on.mount)? else {
        return Ok(Vec::new());
    };
    let Provides::Runtime(rt) = &doc.provides else {
        return Ok(Vec::new());
    };
    let spelled = format!("/{}", entry.trim_start_matches('/'));
    Ok(rt
        .entrypoints
        .iter()
        .find(|e| e.path == spelled)
        .map(|e| e.args_default.clone())
        .unwrap_or_default())
}
