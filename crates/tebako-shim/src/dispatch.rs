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

use crate::manifest::Require;
use crate::resolve::{self, Resolution};
use crate::runtime::{self, RuntimeResolution};
use crate::versions;
use crate::{fail, Ctx, ShimError, EX_TEBAKO_MANIFEST};

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

/// Compose the mount set: the entrypoint's OWN payload slot at `/` (spec
/// 03 §6 suites: entry `slot` selects the image inside a multi-entry
/// package; simple apps stay at slot 0, whole image), then each declared
/// dependency (spec 03 §2.3) resolved against the payload cache.
fn compose_mounts(
    res: &Resolution,
    entry: &crate::manifest::Entrypoint,
    ctx: &Ctx,
) -> Result<Vec<MountSpec>, ShimError> {
    let mut mounts = vec![MountSpec {
        image: res.record.image.clone(),
        slot: entry.slot,
        mount: "/".to_string(),
    }];
    for req in &res.manifest.requires {
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
fn dependency_mount(req: &Require, tool: &str, ctx: &Ctx) -> Result<Option<MountSpec>, ShimError> {
    if req.kind == "language" {
        return Ok(None);
    }
    let (Some(name), Some(mount)) = (&req.name, &req.mount) else {
        return Ok(None);
    };
    let installed = resolve::installed_versions(&ctx.home, name)?;
    let constraint = match &req.constraint {
        Some(c) => versions::parse_constraint(c)?,
        None => versions::parse_constraint(">= 0")?,
    };
    let version = installed
        .iter()
        .filter(|v| constraint.matches(v))
        .max_by(|a, b| versions::compare(a, b));
    let Some(version) = version else {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "\"{tool}\" requires {} {name} but no satisfying version is installed (installed: {})\n  install the dependency, or run `tebako-shim doctor`",
                req.kind,
                if installed.is_empty() {
                    "none".to_string()
                } else {
                    installed.join(", ")
                }
            ),
        );
    };
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
    plan(&res, user_args, ctx, true)
}

/// The plan behind [`dispatch`], parameterized so `which` can resolve
/// without downloading (`allow_download = false`).
pub fn plan(
    res: &Resolution,
    user_args: &[String],
    ctx: &Ctx,
    allow_download: bool,
) -> Result<ExecPlan, ShimError> {
    let entry = res
        .manifest
        .entrypoint(&res.tool)
        .ok_or_else(|| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "payload \"{}\" {} declares no entrypoint \"{}\"",
                    res.payload_name, res.version, res.tool
                ),
            )
        })?
        .clone();
    let mounts = compose_mounts(res, &entry, ctx)?;
    let runtime =
        runtime::resolve_runtime(entry.runtime_requirement.as_ref(), allow_download, ctx)?;

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
            // Zero-runtime: the payload image is the program. Dependency
            // mounts still ride the ABI shape (a self-launching image
            // consumes them); with no dependencies the argv is just
            // entry + user args.
            argv.push(res.record.image.to_string_lossy().into_owned());
            for m in mounts.iter().skip(1) {
                argv.push("--tebako-image".to_string());
                argv.push(m.triple());
            }
            argv.push("--tebako-entry".to_string());
            argv.push(entry.path.clone());
            res.record.image.clone()
        }
    };
    argv.extend(entry.args_default.iter().cloned());
    argv.extend(user_args.iter().cloned());

    Ok(ExecPlan {
        program,
        argv,
        env,
        mounts,
        runtime,
    })
}

/// Exec the plan, replacing the process (unix). Never returns on success.
#[cfg(unix)]
pub fn exec(plan: &ExecPlan) -> ShimError {
    use std::os::unix::process::CommandExt as _;
    let mut cmd = std::process::Command::new(&plan.program);
    cmd.args(&plan.argv[1..]);
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    let err = cmd.exec();
    ShimError::new(
        crate::EX_TEBAKO_IO,
        format!("cannot execute {}: {err}", plan.program.display()),
    )
}

#[cfg(not(unix))]
pub fn exec(plan: &ExecPlan) -> ShimError {
    // The Windows exec port lands with the windows CI leg (spec 06 §3
    // status); fail cleanly rather than misbehave.
    ShimError::new(
        crate::EX_TEBAKO_IO,
        format!(
            "cannot execute {}: exec is not implemented on this platform in v1",
            plan.program.display()
        ),
    )
}
