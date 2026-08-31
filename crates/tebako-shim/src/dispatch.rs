//! Hand-off (spec 07 §2.3): mount payload + ZERO OR MORE runtime payloads
//! + declared dependency mounts, then exec via launcher ABI v1 (spec 06):
//!
//! ```text
//! <runtime> --tebako-image <image>:<slot>:<mount> ...
//!           --tebako-entry <entrypoint> <args...>
//! ```
//!
//! The registry payload is a bare `.tfs` image (no tpkg trailer), so its
//! slot is always `0` (whole image) and its mount point is `/` of the
//! jail namespace; dependency mounts use the consumer-declared `mount:`
//! (spec 03 §2.3 locked MOUNT RULE). Image-era runtimes additionally
//! receive `TEBAKO_RUNTIME_IMAGE` (spec 06 §2).
//!
//! Zero-runtime entrypoints (no `runtime_requirement`) skip runtime
//! resolution entirely: the payload image itself is the program (the
//! self-launching-image contract — a fused bootstrap speaks the same ABI
//! shape minus the runtime prefix).

use std::path::PathBuf;

use tpkg::Requirement;

use crate::resolve::{self, Resolution};
use crate::runtime::{self, RuntimeResolution};
use crate::versions;
use crate::{fail, Ctx, ShimError, EX_TEBAKO_MANIFEST, EX_TEBAKO_UNAVAILABLE};

/// One `--tebako-image` triple (spec 06 §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpec {
    pub image: PathBuf,
    /// Bare registry images always mount as slot 0 (whole image).
    pub slot: u32,
    pub mount: String,
}

impl MountSpec {
    pub fn triple(&self) -> String {
        format!("{}:{}:{}", self.image.display(), self.slot, self.mount)
    }
}

/// The composed hand-off. `argv[0]` is the program; `env` entries are
/// exported on top of the inherited environment.
#[derive(Debug)]
pub struct ExecPlan {
    pub program: PathBuf,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub mounts: Vec<MountSpec>,
    pub runtime: RuntimeResolution,
}

/// Compose the mount set: the payload image at `/`, then each declared
/// dependency (spec 03 §2.3) resolved against the payload cache.
/// Public for the spec 26 §2 check engine (tebako-cli): a check run
/// mounts exactly the composition dispatch would.
pub fn compose_mounts(res: &Resolution, ctx: &Ctx) -> Result<Vec<MountSpec>, ShimError> {
    let mut mounts = vec![MountSpec {
        image: res.record.image.clone(),
        slot: 0,
        mount: "/".to_string(),
    }];
    for req in res.manifest.requires() {
        let dep = dependency_mount(req, &res.tool, ctx)?;
        if let Some(dep) = dep {
            mounts.push(dep);
        }
    }
    Ok(mounts)
}

/// A `requires:` edge → a cached payload image at the consumer-declared
/// mount point. `kind: language` edges are the runtime axis (resolved via
/// the entrypoint's `runtime_requirement`) and are never mounted; edges
/// without a `mount` declare no mount in v1.
fn dependency_mount(
    req: &Requirement,
    tool: &str,
    ctx: &Ctx,
) -> Result<Option<MountSpec>, ShimError> {
    let (kind, name, constraint, mount) = match req {
        Requirement::Language { .. } => return Ok(None),
        Requirement::Toolkit {
            name,
            constraint,
            mount,
            ..
        } => ("toolkit", name, constraint, mount),
        Requirement::Data {
            name,
            constraint,
            mount,
        } => ("data", name, constraint, mount),
    };
    let Some(mount) = mount else {
        return Ok(None);
    };
    let installed = resolve::installed_versions(&ctx.home, name)?;
    // The constraint was validated at manifest parse (tpkg::Constraint) —
    // the dispatcher only evaluates it.
    let constraint = versions::from_validated(constraint);
    let version = installed
        .iter()
        .filter(|v| constraint.matches(v))
        .max_by(|a, b| versions::compare(a, b));
    let Some(version) = version else {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "\"{tool}\" requires {kind} {name} but no satisfying version is installed (installed: {})\n  install the dependency, or run `tebako-shim doctor`",
                if installed.is_empty() {
                    "none".to_string()
                } else {
                    installed.join(", ")
                }
            ),
        );
    };
    tebako_log::log!(
        tebako_log::Level::Debug,
        "shim",
        "dep-mount kind={kind} name={name} version={version} mount={mount}"
    );
    Ok(Some(MountSpec {
        image: crate::manifest::payload_record(&ctx.home, name, version).image,
        slot: 0,
        mount: mount.clone(),
    }))
}

/// The full dispatch (spec 07 §2): resolve payload version → resolve
/// runtime → compose the mount set → the ABI v1 exec plan.
pub fn dispatch(tool: &str, user_args: &[String], ctx: &Ctx) -> Result<ExecPlan, ShimError> {
    let res = resolve::resolve(tool, ctx)?;
    let (flags, args) = parse_jail_flags(user_args)?;
    let jail_env = compose_jail_env(&res, &flags, &args, ctx)?;
    plan(&res, &args, ctx, true, jail_env)
}

/// The plan behind [`dispatch`], parameterized so `which` can resolve
/// without downloading (`allow_download = false`).
pub fn plan(
    res: &Resolution,
    user_args: &[String],
    ctx: &Ctx,
    allow_download: bool,
    jail_env: Vec<(String, String)>,
) -> Result<ExecPlan, ShimError> {
    let entry = res.manifest.entrypoint(&res.tool).ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload \"{}\" {} declares no entrypoint \"{}\"",
                res.payload_name, res.version, res.tool
            ),
        )
    })?;
    let mounts = compose_mounts(res, ctx)?;
    let runtime =
        runtime::resolve_runtime(entry.runtime_requirement.as_ref(), allow_download, ctx)?;
    tebako_log::log!(
        tebako_log::Level::Debug,
        "shim",
        "dispatch tool={} payload={}@{} source={:?} mounts={} runtime={}",
        res.tool,
        res.payload_name,
        res.version,
        res.source,
        mounts.len(),
        match &runtime {
            RuntimeResolution::Ready(rt) => format!(
                "{} {} (tebako {})",
                rt.engine, rt.lang_version, rt.tebako_version
            ),
            RuntimeResolution::Zero => "zero".to_string(),
        }
    );

    let mut argv: Vec<String> = Vec::new();
    let mut env: Vec<(String, String)> = Vec::new();
    let program = match &runtime {
        RuntimeResolution::Ready(rt) => {
            argv.push(rt.exe.to_string_lossy().into_owned());
            for m in &mounts {
                argv.push("--tebako-image".to_string());
                argv.push(m.triple());
            }
            argv.push("--tebako-entry".to_string());
            argv.push(entry.path.clone());
            if let Some(image) = &rt.image {
                // spec 06 §2: image-era drivers mount the env image; v1
                // runtimes ignore it (graceful degradation).
                env.push((
                    "TEBAKO_RUNTIME_IMAGE".to_string(),
                    image.to_string_lossy().into_owned(),
                ));
            }
            rt.exe.clone()
        }
        RuntimeResolution::Zero => {
            // Zero-runtime: the install-time materialization is the
            // program (a run never materializes — install is the
            // explicit verb). The child runs from the store tree (host
            // paths) — it needs NO VFS mounts and NO preload shim. The
            // preload shim's env (LD_PRELOAD / DYLD_INSERT_LIBRARIES)
            // would otherwise be inherited from the parent runtime and
            // intercept the child's own IO (the openjdk JVM's boot
            // classpath failure, dogfood-found 2026-08-12).
            let entry_host = res.record.tree.join(entry.path.trim_start_matches('/'));
            if !entry_host.is_file() {
                return fail(
                    EX_TEBAKO_UNAVAILABLE,
                    format!(
                        "zero-runtime entrypoint \"{}\" of \"{}\" {} is not materialized at {}\n  materialize it with `tebako install {}`",
                        res.tool,
                        res.payload_name,
                        res.version,
                        entry_host.display(),
                        res.payload_name,
                    ),
                );
            }
            argv.push(entry_host.to_string_lossy().into_owned());
            // tebako#503: with no runtime there is no driver, so the shim
            // stays the sole composer of the entrypoint's declared
            // `args_default` here — as leading args after the entry host
            // path. The runtime path's driver composes them between the
            // interpreter and the entry instead (spec 17 §1).
            argv.extend(entry.args_default.iter().cloned());
            entry_host
        }
    };
    argv.extend(user_args.iter().cloned());

    // spec 08: the dispatcher's computed jail env always wins over every
    // manifest/host value (spec 07 §9 env composition).
    env.extend(jail_env);

    Ok(ExecPlan {
        program,
        argv,
        env,
        mounts,
        runtime,
    })
}

// ---------------------------------------------------------------------
// Jail flags (spec 08 §2 — the dispatcher's tightening surface)
// ---------------------------------------------------------------------

/// The dispatcher's jail flags: `--jail <spec>` (`open` | `deny` |
/// `deny:arg` | a YAML file | the TEBAKO_JAIL env grammar), `--mount
/// <host:mount:ro|rw>` (repeatable), `--no-host`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JailFlags {
    pub jail: Option<String>,
    pub mounts: Vec<String>,
    pub no_host: bool,
}

/// Split the dispatcher's flags from the payload's argv. Only the exact
/// known tokens are consumed; anything else — unknown flags included —
/// stops the scan and rides to the payload verbatim (a shim's whole job
/// is forwarding argv). `--` ends the scan; everything after it is the
/// payload's (the escape hatch for payload args literally named
/// `--jail`/`--mount`/`--no-host`).
pub fn parse_jail_flags(args: &[String]) -> Result<(JailFlags, Vec<String>), ShimError> {
    let mut flags = JailFlags::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        match flag {
            "--" => return Ok((flags, args[i + 1..].to_vec())),
            "--jail" | "--mount" => {
                let value = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        args.get(i).cloned().ok_or_else(|| {
                            ShimError::new(
                                crate::EX_USAGE,
                                format!("option '{flag}' requires a value"),
                            )
                        })?
                    }
                };
                if flag == "--jail" {
                    flags.jail = Some(value);
                } else {
                    flags.mounts.push(value);
                }
            }
            "--no-host" => {
                if inline.is_some() {
                    return fail(crate::EX_USAGE, "option '--no-host' takes no value");
                }
                flags.no_host = true;
            }
            _ => {
                // A non-flag token or an unknown flag: the payload's,
                // verbatim (a shim's whole job is forwarding argv; unknown
                // flags belong to payload CLIs).
                return Ok((flags, args[i..].to_vec()));
            }
        }
        i += 1;
    }
    Ok((flags, Vec::new()))
}

/// Compose the dispatch-time jail env (spec 08 §2/§4): the payload
/// mirror's `capabilities.host` REQUEST ∩ the user's tightening flags =
/// the effective jail, exported as TEBAKO_JAIL with the audit source
/// (TEBAKO_JAIL_SOURCE) and the journal pointer (TEBAKO_JAIL_JOURNAL →
/// this home's journal.log, the bootstrap's convention). With
/// `argument_files: auto-allowed` the payload args naming existing files
/// become read-only grants. Empty when no policy applies (byte-identical
/// legacy dispatch).
pub fn compose_jail_env(
    res: &Resolution,
    flags: &JailFlags,
    args: &[String],
    ctx: &Ctx,
) -> Result<Vec<(String, String)>, ShimError> {
    let request = res.manifest.host_jail();
    let user =
        tpkg::HostJail::from_dispatch_flags(flags.jail.as_deref(), &flags.mounts, flags.no_host)
            .map_err(|e| ShimError::new(crate::EX_USAGE, format!("{e}")))?;
    let Some((jail, source)) = tpkg::jail::effective(request, user.as_ref()) else {
        return Ok(Vec::new());
    };
    if jail.is_trivially_open() {
        return Ok(Vec::new());
    }
    let arg_files = if jail.argument_files.auto {
        tpkg::jail::resolve_argument_files(args)
    } else {
        Vec::new()
    };
    Ok(vec![
        ("TEBAKO_JAIL".to_string(), jail.to_env_spec(&arg_files)),
        ("TEBAKO_JAIL_SOURCE".to_string(), source.to_string()),
        (
            "TEBAKO_JAIL_JOURNAL".to_string(),
            ctx.home.join("journal.log").to_string_lossy().into_owned(),
        ),
    ])
}

/// Exec the plan, replacing the process (unix). Never returns on success.
///
/// Zero-runtime dispatches scrub the preload shim's env (`LD_PRELOAD`,
/// `DYLD_INSERT_LIBRARIES`): the child runs from the store tree (host
/// paths), not the VFS, and the inherited shim would intercept its IO
/// (the openjdk JVM's boot classpath failure, dogfood-found 2026-08-12).
#[cfg(unix)]
pub fn exec(plan: &ExecPlan) -> ShimError {
    use std::os::unix::process::CommandExt as _;
    let mut cmd = std::process::Command::new(&plan.program);
    cmd.args(&plan.argv[1..]);
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    if matches!(plan.runtime, RuntimeResolution::Zero) {
        cmd.env_remove("LD_PRELOAD");
        cmd.env_remove("DYLD_INSERT_LIBRARIES");
        cmd.env_remove("DYLD_PRINT_LIBRARIES");
        cmd.env_remove("TEBAKO_TFS_MOUNTS");
    }
    let err = cmd.exec();
    ShimError::new(
        crate::EX_TEBAKO_IO,
        format!("cannot execute {}: {err}", plan.program.display()),
    )
}

/// Windows has no execve(2): spawn the child, wait, and exit with its
/// code (the same contract the bootstrap's spawn_handoff implements for
/// the runtime handoff — the user sees the program's own exit code).
/// Never returns on success.
#[cfg(windows)]
pub fn exec(plan: &ExecPlan) -> ShimError {
    install_ctrl_swallow();
    let mut cmd = std::process::Command::new(&plan.program);
    cmd.args(&plan.argv[1..]);
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    if matches!(plan.runtime, RuntimeResolution::Zero) {
        cmd.env_remove("LD_PRELOAD");
        cmd.env_remove("DYLD_INSERT_LIBRARIES");
        cmd.env_remove("DYLD_PRINT_LIBRARIES");
        cmd.env_remove("TEBAKO_TFS_MOUNTS");
    }
    match cmd.spawn().and_then(|mut child| child.wait()) {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(e) => ShimError::new(
            crate::EX_TEBAKO_IO,
            format!("cannot execute {}: {e}", plan.program.display()),
        ),
    }
}

/// Console Ctrl handling for the spawn handoff (the bootstrap's rule,
/// same reasoning): the child shares our console process group, so
/// CTRL_C/CTRL_BREAK reach it directly (the payload sees its normal
/// SIGINT); the shim must outlive the child to propagate its exit code,
/// so its own copy of those events is swallowed.
#[cfg(windows)]
unsafe extern "system" fn ctrl_swallow(ctrl_type: u32) -> windows_sys::core::BOOL {
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    match ctrl_type {
        CTRL_C_EVENT | CTRL_BREAK_EVENT => TRUE,
        // CLOSE/LOGOFF/SHUTDOWN keep the default processing (terminate).
        _ => FALSE,
    }
}

#[cfg(windows)]
fn install_ctrl_swallow() {
    // Best-effort: without the handler a console Ctrl event kills the
    // shim before the child is reaped, but the handoff itself still
    // works — never fail an exec over it.
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    unsafe {
        SetConsoleCtrlHandler(Some(ctrl_swallow), 1);
    }
}
