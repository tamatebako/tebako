//! The wrapper-exe driver pattern (spec 29) — the boot-tail the
//! standalone runtime exe (`tebako-runtime-launcher`, the repo's one
//! binary home of the pattern) runs around the SHARED spec-17 boot
//! ([`crate::driver::boot`], byte-identical to the linked pattern):
//!
//! 1. boot the handoff (wire parse → env image + payload mounts → jail
//!    → class-R materialization → child-injection and PATH env) exactly
//!    as the linked driver does — the launcher ABI never learns which
//!    pattern a runtime uses (spec 17 §6);
//! 2. answer `--tebako-extract` driver-side (spec 29 §4: dump the
//!    mounted images to disk, exit 0 — BEFORE any interpreter exec;
//!    the unmodified upstream interpreter knows no tebako options);
//! 3. read the env image's `layout.interpreter` declaration (spec 29
//!    §2 — absent key, malformed value, or a path that does not resolve
//!    inside the env mount are named boot errors, exit 65);
//! 4. honor `layout.visibility` (spec 29 §3): `preload` materializes
//!    only the interpreter's load closure (spec 22 §2.1's walk — the
//!    env image stays MOUNTED, the armed preload shim serves its reads)
//!    and requires the image's `preload_shim` grant; `exec-cache`
//!    materializes through the home-tree-aware exec routing (spec 22
//!    §6/§3.2) and bridges the entry token to its host twin (a
//!    host-plain interpreter cannot read the VFS). A declared mechanism
//!    unusable on this host, or an unknown value, is exit 65 naming the
//!    mechanism and the fact — never a silent fallback. Absent the key,
//!    the locked default order applies (preload on POSIX, exec-cache on
//!    windows) and is journaled;
//! 5. compose the interpreter's argv (spec 29 §1):
//!    `[<interpreter>, <args_default…>, <entry>, <user args…>]` — the
//!    entrypoint's declared `args_default` (spec 03 §2.2, read from the
//!    app payload's own manifest) composes between, so a jar entry
//!    `{path: /app/jing.jar, args_default: ["-jar"]}` launches
//!    `java -jar <entry> <user args…>`.
//!
//! The process layer ([`Launch`]'s consumer, the launcher binary) then
//! execs on POSIX (the interpreter REPLACES the wrapper process) or
//! spawns+waits+propagates on windows — spec 29 §1's process semantics.
//!
//! No per-runtime knowledge lives here (spec 29 §7): the interpreter
//! path, the visibility, and the entrypoint's `args_default` are all
//! authored in the images' own manifests and flowed, never hardcoded.

use tfs::context::context;

use crate::driver::{
    boot, env_var, errno_text, join_mount, mounted_manifest_at, qualify_mount, BootOutcome,
    DriverError, Env,
};
use crate::handoff::Handoff;
use crate::layout::ImageLayout;
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

/// The wrapper pattern's baked runtime root. Spec 17 §1's "per-platform
/// baked default owned by the runtime factory" is owned by the launcher
/// crate itself for repacked runtimes — the wrapper IS the runtime exe
/// tebako ships, and the env image's `layout.mount_root` is authored to
/// match (spec 18 C3's pair check is unchanged; a mismatch is exit 78).
/// The spelling follows the ecosystem's one namespace convention;
/// `TEBAKO_MOUNT_ROOT` still overrides at boot when the image grants
/// `mount_root_override` (the shared boot owns that flow).
#[cfg(windows)]
pub const WRAPPER_RUNTIME_ROOT: &str = "A:/t";
#[cfg(not(windows))]
pub const WRAPPER_RUNTIME_ROOT: &str = "/__tfs__";

fn manifest(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_MANIFEST, message.into())
}

fn io(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_IO, message.into())
}

/// What the process layer is asked to do after a successful wrapper
/// boot (spec 29 §1's process semantics live in the launcher binary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The interpreter's materialized HOST path — the exec/spawn target
    /// and argv[0] (spec 29 §1: the interpreter stands at index 0).
    pub program: String,
    /// The interpreter's full argv:
    /// `[<program>, <args_default…>, <entry…>, <user args…>]`.
    pub argv: Vec<String>,
}

/// The wrapper boot's directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootAction {
    /// `--tebako-extract` answered driver-side (spec 29 §4): the mounted
    /// images were dumped to `dest`; the process exits 0 without ever
    /// exec'ing the interpreter. `skipped_symlinks` is the extraction's
    /// own accounting (no backend carries readlink today).
    Extracted {
        dest: String,
        skipped_symlinks: usize,
    },
    /// Exec (POSIX) or spawn+wait+propagate (windows) this interpreter.
    Launch(Launch),
}

/// The kernel-visibility mechanism this boot honors (spec 29 §3) — the
/// set reachable in this build: tier 1 (`preload`, POSIX interposition)
/// and tier 2b (`exec-cache`, the materialized home tree). Tier 2a
/// (`seccomp-notify`) is named-and-refused until it lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mechanism {
    Preload,
    ExecCache,
}

impl Mechanism {
    fn name(self) -> &'static str {
        match self {
            Mechanism::Preload => "preload",
            Mechanism::ExecCache => "exec-cache",
        }
    }
}

/// The visibility decision (spec 29 §3): a declaration is honored or
/// fails closed (never a silent fallback); absent it, the locked default
/// order applies — preload on POSIX (tier 1), exec-cache on windows
/// (tier 2b, the only mechanism with no interposition API) — journaled
/// at boot either way. `preload` needs the image's `preload_shim` grant
/// (schema_minor 2): without the shim the interposition cannot arm and
/// the mounted env image would be invisible to the interpreter.
fn mechanism(layout: &ImageLayout, image: &str) -> Result<Mechanism, DriverError> {
    let declared = layout.visibility.as_deref();
    let mech = match declared {
        Some("preload") => {
            if cfg!(windows) {
                return Err(manifest(
                    "layout.visibility 'preload' is unusable on this host: windows has no interposition API (spec 29 §3) — the factory validates the declaration against the runtime's platform list at press",
                ));
            }
            if layout.preload_shim.is_none() {
                return Err(manifest(format!(
                    "layout.visibility 'preload' but env image '{image}' declares no preload_shim — the interposition cannot arm (spec 29 §3): ship libtfs-preload in the image or declare exec-cache"
                )));
            }
            Mechanism::Preload
        }
        Some("exec-cache") => Mechanism::ExecCache,
        Some("seccomp-notify") => {
            return Err(manifest(
                "layout.visibility 'seccomp-notify' is not reachable in this driver build (the linux tier-2a surface is unimplemented — spec 29 §3) — the factory must not declare it for this runtime",
            ));
        }
        Some(other) => {
            return Err(manifest(format!(
                "layout.visibility '{other}' is not a known mechanism (preload | seccomp-notify | exec-cache — spec 29 §3)"
            )));
        }
        None => {
            if cfg!(windows) {
                Mechanism::ExecCache
            } else if layout.preload_shim.is_some() {
                Mechanism::Preload
            } else {
                // The POSIX default is tier 1 — which the shim grant
                // gates: without it there is no interposition to arm,
                // and a silent tier-2b slide is the forbidden fallback.
                return Err(manifest(format!(
                    "env image '{image}' declares no layout.visibility and no preload_shim — the default mechanism (preload, spec 29 §3) cannot arm: ship libtfs-preload in the image or declare exec-cache"
                )));
            }
        }
    };
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "event=visibility mechanism={} image={} source={}",
        mech.name(),
        image,
        if declared.is_some() {
            "declared"
        } else {
            "default"
        }
    );
    Ok(mech)
}

/// The `--tebako-extract` scan (spec 29 §4): the runtime-side option
/// rides the interpreter's OWN argument position (spec 06 §1) — the
/// args before `--tebako-entry`, never the user's (a post-entry token
/// is the app's argument, exactly as the linked interpreter would parse
/// it). The value is the next token; a bare flag is a named error.
fn extract_dest(h: &Handoff) -> Result<Option<String>, DriverError> {
    let Some(pos) = h
        .interpreter_args
        .iter()
        .position(|a| a == "--tebako-extract")
    else {
        return Ok(None);
    };
    h.interpreter_args
        .get(pos + 1)
        .cloned()
        .map(Some)
        .ok_or_else(|| manifest("--tebako-extract shall name a destination directory"))
}

/// Resolve the declared interpreter to its effective VFS path (spec 29
/// §2): the value is POSIX-absolute in-image on every platform (no
/// drive qualifier, no `..`), joined under the env image's mount (the
/// effective runtime root — drive-qualified by construction), and must
/// resolve inside that mount. Every refusal is exit 65 naming the key
/// or the path and the mount.
fn interpreter_vfs_path(
    layout: &ImageLayout,
    root: &str,
    image: &str,
) -> Result<String, DriverError> {
    let declared = layout.interpreter.as_deref().ok_or_else(|| {
        manifest(format!(
            "env image '{image}' declares no layout.interpreter — the wrapper pattern requires it (spec 29 §2); a layout block without interpreter is the LINKED pattern"
        ))
    })?;
    let well_formed = declared.len() > 1
        && declared.starts_with('/')
        && !declared
            .split('/')
            .any(|c| c == ".." || c.contains(':') || c.contains('\\'));
    if !well_formed {
        return Err(manifest(format!(
            "layout.interpreter '{declared}' is not a usable in-image path (want POSIX-absolute, no drive qualifier, no '..') — the env image's declaration lies (spec 29 §2)"
        )));
    }
    let vfs = join_mount(root, declared);
    let mut ctx = context().write().unwrap();
    match ctx.open(&vfs, libc::O_RDONLY) {
        Ok(fd) => {
            let _ = ctx.close(fd);
            Ok(vfs)
        }
        Err(e) => Err(manifest(format!(
            "layout.interpreter '{declared}' does not resolve at '{vfs}' inside the env image mounted at '{root}' ({}) — the env image's declaration lies (spec 29 §2)",
            errno_text(e)
        ))),
    }
}

/// The entrypoint's declared `args_default` (spec 03 §2.2), read from
/// the app payload's own manifest — the FIRST `--tebako-image` triple's
/// mount (spec 17 §1's entry base). Composed between the interpreter
/// and the entry (spec 29 §1). No manifest, a non-app payload, or no
/// entrypoint declaring this path all mean an empty list — the
/// positional-entry form — never an error; a corrupt manifest stays the
/// named 65 it is everywhere (the image lying).
fn args_default(h: &Handoff, outcome: &BootOutcome) -> Result<Vec<String>, DriverError> {
    let entry_case = h.entry.as_deref().is_some_and(|e| e.contains('/'));
    let (Some(first), Some(entry), true) = (h.images.first(), h.entry.as_deref(), entry_case)
    else {
        return Ok(Vec::new());
    };
    let mount = qualify_mount(&first.mount, &outcome.runtime_root);
    let Some(manifest_doc) = mounted_manifest_at(&mount)? else {
        return Ok(Vec::new());
    };
    let spelled = format!("/{}", entry.trim_start_matches('/'));
    if let tpkg::Provides::App(app) = &manifest_doc.provides {
        if let Some(ep) = app.entrypoints.iter().find(|ep| ep.path == spelled) {
            return Ok(ep.args_default.clone());
        }
    }
    Ok(Vec::new())
}

/// The composition (spec 29 §1): the interpreter at index 0, then the
/// entrypoint's `args_default`, then the boot's rewritten argv sans the
/// original program name — `[entry resolved, user args…]` in the entry
/// case, the interpreter's own args in the bare/smoke forms, the user
/// args in the interpreter-keyword form (the keyword itself was dropped
/// by the boot, spec 17 §1).
fn compose(program: String, defaults: Vec<String>, outcome: &BootOutcome) -> Launch {
    let rest = outcome.argv.get(1..).unwrap_or(&[]);
    let mut argv = Vec::with_capacity(1 + defaults.len() + rest.len());
    argv.push(program.clone());
    argv.extend(defaults);
    argv.extend(rest.iter().cloned());
    Launch { program, argv }
}

/// Materialize the interpreter per the mechanism (spec 29 §3 → the
/// spec-22 machinery, reused): `preload` walks only the exe's load
/// closure (the mounted env image serves everything else through the
/// armed shim); `exec-cache` routes through the home-tree decision (a
/// home-annotated mount extracts whole, anything else answers the
/// closure mirror). The trace op names the process semantics (exec on
/// POSIX, spawn on windows — spec 22 §2's op split).
fn materialize(vfs: &str, mech: Mechanism, root: &str) -> Result<String, DriverError> {
    let result = {
        let mut ctx = context().write().unwrap();
        match mech {
            Mechanism::Preload => ctx.dlmap2file(vfs),
            Mechanism::ExecCache if cfg!(windows) => ctx.exec_materialize_for_spawn(vfs),
            Mechanism::ExecCache => ctx.exec_materialize(vfs),
        }
    };
    result
        .map(|c| c.to_string_lossy().into_owned())
        .map_err(|e| {
            if e == libc::ENOENT {
                manifest(format!(
                    "the interpreter at '{vfs}' does not resolve inside the env image mounted at '{root}' ({})",
                    errno_text(e)
                ))
            } else {
                io(format!(
                    "cannot materialize the interpreter '{vfs}' (mechanism {}): {}",
                    mech.name(),
                    errno_text(e)
                ))
            }
        })
}

/// The entry token under `exec-cache` (spec 29 §3's host-plain child):
/// the interpreter cannot read the VFS, so an entry inside this boot's
/// mounts is bridged to its materialized host twin (the same exec
/// routing — a home mount answers whole-tree, anything else the file
/// plus its spec 22 §3 Rule E4 siblings). An entry outside the mounts
/// passes through verbatim — it belongs to the interpreter's own
/// startup (spec 17 §1) and answers with its own honest error (the
/// spec 22 §3.2 argv bridge's discipline: never a silent rewrite of
/// host tokens). Under `preload` the armed shim serves the VFS spelling
/// — no bridge. `entry_index` is the entry's position in
/// `launch.argv` (after program + defaults) in the entry case.
fn bridge_entry(
    launch: &mut Launch,
    mech: Mechanism,
    entry_index: Option<usize>,
) -> Result<(), DriverError> {
    if mech != Mechanism::ExecCache {
        return Ok(());
    }
    let Some(index) = entry_index else {
        return Ok(());
    };
    let Some(token) = launch.argv.get(index).cloned() else {
        return Ok(());
    };
    if !context().read().unwrap().path_is_embedded(&token) {
        return Ok(());
    }
    let host = {
        let mut ctx = context().write().unwrap();
        if cfg!(windows) {
            ctx.exec_materialize_for_spawn(&token)
        } else {
            ctx.exec_materialize(&token)
        }
    };
    launch.argv[index] = host
        .map(|c| c.to_string_lossy().into_owned())
        .map_err(|e| {
            if e == libc::ENOENT {
                manifest(format!(
                    "the entrypoint '{token}' does not resolve inside the mounted tree"
                ))
            } else {
                io(format!(
                    "cannot materialize the entrypoint '{token}' for the host-plain interpreter: {}",
                    errno_text(e)
                ))
            }
        })?;
    Ok(())
}

/// The wrapper boot (spec 29): the shared spec-17 boot, then the
/// boot-tail above. `argv` is the process argv with argv[0] at index 0;
/// `runtime_root` is the launcher's baked root ([`WRAPPER_RUNTIME_ROOT`]
/// — overridable via `TEBAKO_MOUNT_ROOT` when the image grants it);
/// `env` is the process environment. Any failure unmounts everything
/// (the shared boot's rule — extended to the wrapper's own tail: a
/// refused interpreter declaration or visibility never leaves a partial
/// mount behind either) and carries the loader's named exit code.
pub fn run(argv: &[String], runtime_root: &str, env: &dyn Env) -> Result<BootAction, DriverError> {
    let h = Handoff::parse(argv)?;
    let outcome = boot(argv, runtime_root, env)?;
    match tail(&h, &outcome, env) {
        Ok(action) => Ok(action),
        Err(e) => {
            context().write().unwrap().unmount();
            Err(e)
        }
    }
}

/// The boot-tail proper (everything after the shared boot): extract,
/// interpreter declaration, visibility, materialization, composition.
/// Split from [`run`] so the unmount-on-failure rule lives in exactly
/// one place.
fn tail(h: &Handoff, outcome: &BootOutcome, env: &dyn Env) -> Result<BootAction, DriverError> {
    // `--tebako-extract` is answered BEFORE the interpreter is executed
    // (spec 29 §4) — including before its declaration is consulted.
    if let Some(dest) = extract_dest(h)? {
        let skipped = context()
            .write()
            .unwrap()
            .extract_all(std::path::Path::new(&dest))
            .map_err(|e| {
                if e == libc::ENODEV {
                    manifest("--tebako-extract with no images mounted — nothing to dump")
                } else {
                    io(format!(
                        "--tebako-extract: cannot dump the mounted images to '{dest}': {}",
                        errno_text(e)
                    ))
                }
            })?;
        return Ok(BootAction::Extracted {
            dest,
            skipped_symlinks: skipped,
        });
    }
    let layout = outcome.layout.clone().ok_or_else(|| {
        manifest(
            "layout.interpreter cannot resolve: no env image mounted (TEBAKO_RUNTIME_IMAGE unset) — the wrapper pattern's interpreter lives in the env image (spec 29 §2)",
        )
    })?;
    let image = env_var(env, "TEBAKO_RUNTIME_IMAGE").unwrap_or_else(|| "-".to_string());
    let vfs = interpreter_vfs_path(&layout, &outcome.runtime_root, &image)?;
    let mech = mechanism(&layout, &image)?;
    let program = materialize(&vfs, mech, &outcome.runtime_root)?;
    let defaults = args_default(h, outcome)?;
    let entry_index = if h.entry.as_deref().is_some_and(|e| e.contains('/')) {
        // [program, defaults…, ENTRY, user args…] — fixed at composition.
        Some(1 + defaults.len())
    } else {
        None
    };
    let mut launch = compose(program, defaults, outcome);
    bridge_entry(&mut launch, mech, entry_index)?;
    Ok(BootAction::Launch(launch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_view(interpreter: Option<&str>, visibility: Option<&str>, shim: bool) -> ImageLayout {
        ImageLayout {
            schema_version: 1,
            era: 2,
            image_layout: 1,
            mount_root: "/__tfs__".to_string(),
            interpreter_api_version: "21".to_string(),
            mount_root_override: false,
            preload_shim: shim.then(|| "lib/tebako/libtfs_preload.so".to_string()),
            runtime_dll: None,
            interpreter: interpreter.map(str::to_string),
            visibility: visibility.map(str::to_string),
        }
    }

    #[test]
    fn declared_mechanisms_are_honored_or_refused_by_name() {
        // exec-cache: universal, no shim needed
        assert_eq!(
            mechanism(
                &layout_view(Some("/bin/java"), Some("exec-cache"), false),
                "/e.tfs"
            )
            .unwrap(),
            Mechanism::ExecCache
        );
        // an unknown value is a named 65 naming the key's value
        let err = mechanism(
            &layout_view(Some("/bin/java"), Some("fuse"), false),
            "/e.tfs",
        )
        .unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err}");
        assert!(err.message.contains("'fuse'"), "{err}");
        assert!(err.message.contains("layout.visibility"), "{err}");
        // seccomp-notify is named-and-refused until the tier-2a surface lands
        let err = mechanism(
            &layout_view(Some("/bin/java"), Some("seccomp-notify"), false),
            "/e.tfs",
        )
        .unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err}");
        assert!(err.message.contains("seccomp-notify"), "{err}");
    }

    #[cfg(not(windows))]
    #[test]
    fn preload_requires_the_shim_grant() {
        // declared preload + the shim declared → honored
        assert_eq!(
            mechanism(
                &layout_view(Some("/bin/java"), Some("preload"), true),
                "/e.tfs"
            )
            .unwrap(),
            Mechanism::Preload
        );
        // declared preload without the shim → 65 naming mechanism + fact
        let err = mechanism(
            &layout_view(Some("/bin/java"), Some("preload"), false),
            "/e.tfs",
        )
        .unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err}");
        assert!(err.message.contains("preload"), "{err}");
        assert!(err.message.contains("preload_shim"), "{err}");
        // the POSIX default needs the grant too — never a silent slide
        // to tier 2b (spec 29 §3's no-silent-fallback law)
        let err = mechanism(&layout_view(Some("/bin/java"), None, false), "/e.tfs").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err}");
        assert!(err.message.contains("preload_shim"), "{err}");
        // …and takes tier 1 when the image delivers the shim
        assert_eq!(
            mechanism(&layout_view(Some("/bin/java"), None, true), "/e.tfs").unwrap(),
            Mechanism::Preload
        );
    }

    #[test]
    fn the_interpreter_declaration_is_form_checked() {
        let l = layout_view(Some("bin/java"), None, false);
        let err = interpreter_vfs_path(&l, "/__tfs__", "/e.tfs").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err}");
        assert!(err.message.contains("layout.interpreter"), "{err}");
        assert!(err.message.contains("bin/java"), "{err}");
        for bad in ["/", "/../x", "/a/../../b", "/C:/x", "/a\\b"] {
            let l = layout_view(Some(bad), None, false);
            let err = interpreter_vfs_path(&l, "/__tfs__", "/e.tfs").unwrap_err();
            assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{bad}: {err}");
        }
    }

    #[test]
    fn the_absent_interpreter_key_is_named() {
        let l = layout_view(None, None, false);
        let err = interpreter_vfs_path(&l, "/__tfs__", "/e.tfs").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err}");
        assert!(err.message.contains("layout.interpreter"), "{err}");
        assert!(err.message.contains("LINKED"), "{err}");
    }

    #[test]
    fn compose_puts_the_interpreter_at_index_zero() {
        let outcome = BootOutcome {
            argv: vec![
                "wrapper".to_string(),
                "/bin/app".to_string(),
                "-x".to_string(),
            ],
            layout: None,
            runtime_root: "/__tfs__".to_string(),
        };
        let launch = compose(
            "/cache/java".to_string(),
            vec!["-jar".to_string()],
            &outcome,
        );
        assert_eq!(launch.program, "/cache/java");
        assert_eq!(launch.argv, vec!["/cache/java", "-jar", "/bin/app", "-x"]);
        // no defaults, no entry (the bare form): the interpreter's own args
        let outcome = BootOutcome {
            argv: vec!["wrapper".to_string(), "--version".to_string()],
            layout: None,
            runtime_root: "/__tfs__".to_string(),
        };
        let launch = compose("/cache/java".to_string(), vec![], &outcome);
        assert_eq!(launch.argv, vec!["/cache/java", "--version"]);
    }

    #[test]
    fn extract_scans_the_interpreters_own_args_only() {
        let h = Handoff {
            interpreter_args: vec!["--tebako-extract".to_string(), "dest".to_string()],
            ..Handoff::default()
        };
        assert_eq!(extract_dest(&h).unwrap().as_deref(), Some("dest"));
        // a post-entry token is the app's argument — never answered here
        let h = Handoff {
            entry: Some("/bin/app".to_string()),
            user_args: vec!["--tebako-extract".to_string(), "dest".to_string()],
            ..Handoff::default()
        };
        assert_eq!(extract_dest(&h).unwrap(), None);
        // a bare flag is a named error
        let h = Handoff {
            interpreter_args: vec!["--tebako-extract".to_string()],
            ..Handoff::default()
        };
        let err = extract_dest(&h).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err}");
    }
}
