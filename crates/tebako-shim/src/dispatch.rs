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
    mounts_for(&res.record.image, res.manifest.requires(), &res.tool, ctx)
}

/// The mount set of one payload: its image at `/`, then each declared
/// dependency (spec 03 §2.3) resolved against the payload cache. Shared
/// by dispatch and by the spec 32 §2 child composition (the PROVIDER's
/// mounts in the spawned child's plan).
fn mounts_for(
    image: &std::path::Path,
    requires: &[Requirement],
    tool: &str,
    ctx: &Ctx,
) -> Result<Vec<MountSpec>, ShimError> {
    let mut mounts = vec![MountSpec {
        image: image.to_path_buf(),
        slot: 0,
        mount: "/".to_string(),
    }];
    for req in requires {
        let dep = dependency_mount(req, tool, ctx)?;
        if let Some(dep) = dep {
            mounts.push(dep);
        }
    }
    Ok(mounts)
}

/// A `requires:` edge → a cached payload image at the consumer-declared
/// mount point. `kind: language` edges are the runtime axis (resolved via
/// the entrypoint's `runtime_requirement`) and are never mounted; a
/// `runtime` edge (spec 30) is a SPAWNED dependency — resolved at
/// dispatch into the spawn lock, never co-mounted into this stack; an
/// `executable` edge (spec 03 §8 + spec 32) co-mounts only on its
/// `mount` axis (its `expose` axis is the spawn surface); edges without
/// a `mount` declare no mount in v1.
fn dependency_mount(
    req: &Requirement,
    tool: &str,
    ctx: &Ctx,
) -> Result<Option<MountSpec>, ShimError> {
    let (kind, name, constraint, mount) = match req {
        Requirement::Language { .. } | Requirement::Runtime { .. } => return Ok(None),
        Requirement::Executable {
            name,
            payload,
            constraint,
            mount,
            ..
        } => {
            let Some(mount) = mount else {
                return Ok(None);
            };
            // spec 32 §1: the pin names the provider directly; without it
            // the capability scan answers (DependencyNotFound /
            // AmbiguousProvider). Cache-only — install is the explicit
            // verb (the shim's dep-mount posture).
            let provider =
                resolve_provider_payload(tool, name, payload.as_deref(), constraint, ctx)?;
            tebako_log::log!(
                tebako_log::Level::Debug,
                "shim",
                "dep-mount kind=executable name={} provider={} version={} mount={mount}",
                name,
                provider.name,
                provider.version
            );
            return Ok(Some(MountSpec {
                image: provider.image.clone(),
                slot: 0,
                mount: mount.clone(),
            }));
        }
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

/// spec 32 §1/§5: resolve an executable edge's provider payload from the
/// store — CACHE-ONLY at dispatch (install is the explicit verb; the
/// shim's existing dep-mount posture). The `payload:` pin names the
/// provider by name; without it the capability scan answers: zero
/// candidates is `DependencyNotFound`, more than one provider payload is
/// `AmbiguousProvider` (spec 03 §8 — pin with `payload:`). Among one
/// provider's matching versions the newest wins.
fn resolve_provider_payload(
    tool: &str,
    name: &str,
    pin: Option<&str>,
    constraint: &tpkg::Constraint,
    ctx: &Ctx,
) -> Result<tpkg::payload_store::CachedPayload, ShimError> {
    let evaluable = versions::from_validated(constraint);
    let store_err = |e: String| ShimError::new(EX_TEBAKO_MANIFEST, e);
    if let Some(pin) = pin {
        let installed = resolve::installed_versions(&ctx.home, pin)?;
        let version = installed
            .iter()
            .filter(|v| evaluable.matches(v))
            .max_by(|a, b| versions::compare(a, b));
        let Some(version) = version else {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "\"{tool}\" requires executable {name} (provider payload {pin}) but no satisfying version is installed (installed: {})\n  install it with `tebako install {pin}`",
                    if installed.is_empty() {
                        "none".to_string()
                    } else {
                        installed.join(", ")
                    }
                ),
            );
        };
        return tpkg::payload_store::get(&ctx.home, pin, version)
            .map_err(store_err)?
            .ok_or_else(|| {
                ShimError::new(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "the installed record of provider payload {pin} {version} is incomplete (image or trust anchor missing)\n  re-install it with `tebako install {pin}`"
                    ),
                )
            });
    }
    let candidates = tpkg::payload_store::find_capability_providers(&ctx.home, name, constraint)
        .map_err(store_err)?;
    let mut names: Vec<String> = candidates.iter().map(|p| p.name.clone()).collect();
    names.sort();
    names.dedup();
    match names.len() {
        0 => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "\"{tool}\" requires executable {name} but no installed payload provides it (DependencyNotFound)\n  install a provider with `tebako install <payload>`"
            ),
        ),
        1 => {
            let provider_name = names.pop().unwrap_or_default();
            candidates
                .into_iter()
                .filter(|p| p.name == provider_name)
                .max_by(|a, b| versions::compare(&a.version, &b.version))
                .ok_or_else(|| {
                    ShimError::new(
                        EX_TEBAKO_MANIFEST,
                        format!("provider payload {provider_name} vanished mid-resolution"),
                    )
                })
        }
        _ => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "executable \"{name}\" is provided by more than one installed payload ({}) (AmbiguousProvider)\n  pin the provider with `payload:` on the edge (spec 03 §8)",
                names.join(", ")
            ),
        ),
    }
}

/// The provider's entrypoint an exposed name dispatches to (spec 32 §1):
/// declared in the provider's `provides.entrypoints` and CARRYING
/// `runtime_requirement` — a runtime-less match (a toolkit executable, a
/// native entrypoint) is a named resolution error, never an exec-tier
/// fallback.
fn provider_spawn_entrypoint<'m>(
    provider: &'m tpkg::payload_store::CachedPayload,
    edge_name: &str,
    exposed: &str,
) -> Result<&'m tpkg::Entrypoint, ShimError> {
    let tpkg::Provides::App(app) = &provider.manifest.provides else {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "executable edge \"{edge_name}\": provider payload {} {} is not an app payload — it declares no entrypoints to spawn (spec 32 §1)",
                provider.name, provider.version
            ),
        );
    };
    let entry = app.entrypoints.iter().find(|e| e.name == exposed);
    let Some(entry) = entry else {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "executable edge \"{edge_name}\": provider payload {} {} declares no entrypoint \"{exposed}\" — the expose list outruns the provider's declaration (spec 32 §7)",
                provider.name, provider.version
            ),
        );
    };
    if entry.runtime_requirement.is_none() {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "executable edge \"{edge_name}\": the provider's entrypoint \"{exposed}\" carries no runtime_requirement — a runtime-less entry has no spawn form, its surface is the exec tier (spec 32 §0/§1)",
            ),
        );
    }
    Ok(entry)
}

/// The runtime pair a payload row nests (spec 32 §5/§6): the provider's
/// resolved runtime, driven by the exposed entrypoints' own
/// `runtime_requirement`s. Every exposed name of the edge must resolve
/// to the SAME pair — a disagreement is a named error (split the edges).
fn provider_spawn_pair(
    provider: &tpkg::payload_store::CachedPayload,
    edge_name: &str,
    expose: &[String],
    allow_download: bool,
    ctx: &Ctx,
) -> Result<tpkg::runtime_store::CachedRuntime, ShimError> {
    let mut picked: Option<tpkg::runtime_store::CachedRuntime> = None;
    for exposed in expose {
        let entry = provider_spawn_entrypoint(provider, edge_name, exposed)?;
        let req = entry
            .runtime_requirement
            .as_ref()
            .expect("provider_spawn_entrypoint post-asserts runtime_requirement");
        let rt = match runtime::resolve_runtime(Some(req), allow_download, ctx)? {
            runtime::RuntimeResolution::Ready(rt) => *rt,
            runtime::RuntimeResolution::Zero => {
                unreachable!("a requirement was passed — never Zero")
            }
        };
        if rt.image.is_none() {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "the resolved {} runtime {} (tebako {}) carries no verified env image — a spawned payload needs the image pair; re-install it with `tebako install`",
                    req.engine, rt.lang_version, rt.tebako_version
                ),
            );
        }
        match &picked {
            None => picked = Some(rt),
            Some(p)
                if p.lang_version == rt.lang_version
                    && p.tebako_version == rt.tebako_version
                    && p.engine == rt.engine => {}
            Some(p) => {
                return fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "executable edge \"{edge_name}\": the exposed entries disagree on the runtime pair ({} {} tebako {} vs {} {} tebako {}) — one payload row nests ONE pair (spec 32 §5); split the edge per runtime",
                        p.engine, p.lang_version, p.tebako_version, rt.engine, rt.lang_version, rt.tebako_version
                    ),
                );
            }
        }
    }
    picked.ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("executable edge \"{edge_name}\" exposes no entries"),
        )
    })
}

/// spec 30 §4 + spec 32 §5: append the spawn-lock rows of one manifest's
/// spawn edges, TRANSITIVELY — a runtime edge resolves its pair into a
/// runtime row; an expose-carrying executable edge resolves its provider
/// (cache-only) and the provider's runtime pair into a payload row, then
/// recurses into the provider's OWN spawn edges (the spawned child has
/// no loader, so the transitive pins compose here). Identical rows
/// dedupe; `visiting` is the payload-name cycle guard — a cycle through
/// spawn edges is a named error, never a recursion trap.
fn compose_spawn_lock(
    requires: &[Requirement],
    allow_download: bool,
    ctx: &Ctx,
    visiting: &mut Vec<String>,
    rows: &mut Vec<String>,
) -> Result<(), ShimError> {
    for edge in requires {
        match edge {
            Requirement::Runtime {
                engine,
                implementation,
                constraint,
                ..
            } => {
                let rt = runtime::resolve_runtime_edge(
                    engine,
                    implementation.as_deref(),
                    constraint,
                    allow_download,
                    ctx,
                )?;
                let row = tpkg::runtime_store::spawn_lock_entry(
                    engine,
                    &rt.lang_version,
                    &rt.tebako_version,
                );
                if !rows.contains(&row) {
                    rows.push(row);
                }
            }
            Requirement::Executable {
                name,
                payload,
                constraint,
                expose,
                ..
            } if !expose.is_empty() => {
                let provider =
                    resolve_provider_payload(name, name, payload.as_deref(), constraint, ctx)?;
                if visiting.iter().any(|p| p == &provider.name) {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        format!(
                            "spawn dependency cycle through provider payload \"{}\" ({}): the executable edges form a cycle — break it (spec 32 §2)",
                            provider.name,
                            visiting.join(" -> ")
                        ),
                    );
                }
                let pair = provider_spawn_pair(&provider, name, expose, allow_download, ctx)?;
                let row = tpkg::runtime_store::spawn_lock_payload_entry(
                    &provider.name,
                    &provider.version,
                    &pair.engine,
                    &pair.lang_version,
                    &pair.tebako_version,
                );
                if !rows.contains(&row) {
                    rows.push(row);
                }
                visiting.push(provider.name.clone());
                compose_spawn_lock(
                    &provider.manifest.requires,
                    allow_download,
                    ctx,
                    visiting,
                    rows,
                )?;
                visiting.pop();
            }
            _ => {}
        }
    }
    Ok(())
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
    // spec 30 §3: an exposed name dispatches the RUNTIME's own boot. The
    // consumer payload is never mounted (nothing of it exists in the
    // child's VFS); the bare entry name resolves against the runtime's
    // own embedded manifest (spec 17 §1's bare-name rule), so its
    // args_default compose child-side.
    if let Some(Requirement::Runtime {
        engine,
        implementation,
        constraint,
        ..
    }) = &res.exposed
    {
        let rt = runtime::resolve_runtime_edge(
            engine,
            implementation.as_deref(),
            constraint,
            allow_download,
            ctx,
        )?;
        let image = rt
            .image
            .clone()
            .expect("resolve_runtime_edge post-asserts the env image");
        let mut argv = vec![
            rt.exe.to_string_lossy().into_owned(),
            "--tebako-entry".to_string(),
            res.tool.clone(),
        ];
        argv.extend(user_args.iter().cloned());
        let mut env = vec![(
            "TEBAKO_RUNTIME_IMAGE".to_string(),
            image.to_string_lossy().into_owned(),
        )];
        // The dispatcher's jail (the consumer payload's needs ∩ the
        // user's tightening) always wins (spec 08 §2).
        env.extend(jail_env);
        return Ok(ExecPlan {
            program: rt.exe.clone(),
            argv,
            env,
            mounts: Vec::new(),
            runtime: RuntimeResolution::Ready(Box::new(rt)),
        });
    }
    // spec 32 §2/§3: an exposed name of an EXECUTABLE edge dispatches
    // the PROVIDER payload's own managed dispatch as the child — its own
    // runtime pair, its own image co-mounted at / with its own dependency
    // mounts, the entry resolved against the provider image's
    // entrypoints (spec 17 §1's app-payload rule). The consumer payload
    // is never mounted in the child.
    if let Some(Requirement::Executable {
        name,
        payload,
        constraint,
        expose,
        ..
    }) = &res.exposed
    {
        let provider =
            resolve_provider_payload(&res.tool, name, payload.as_deref(), constraint, ctx)?;
        // Validate the exposed entry (named errors for the undeclared /
        // runtime-less classes) before composing around it.
        provider_spawn_entrypoint(&provider, name, &res.tool)?;
        let pair = provider_spawn_pair(&provider, name, expose, allow_download, ctx)?;
        let image = pair
            .image
            .clone()
            .expect("provider_spawn_pair post-asserts the env image");
        let mounts = mounts_for(&provider.image, &provider.manifest.requires, &res.tool, ctx)?;
        let mut argv = vec![pair.exe.to_string_lossy().into_owned()];
        for m in &mounts {
            argv.push("--tebako-image".to_string());
            argv.push(m.triple());
        }
        argv.push("--tebako-entry".to_string());
        argv.push(res.tool.clone());
        argv.extend(user_args.iter().cloned());
        let mut env = vec![(
            "TEBAKO_RUNTIME_IMAGE".to_string(),
            image.to_string_lossy().into_owned(),
        )];
        // The child's fresh spawn lock (spec 32 §2): the provider's OWN
        // spawn edges, resolved transitively — the parent's lock never
        // leaks (the child's driver strips and rebuilds; here the shim
        // composes the whole child env, so only the fresh rows are set).
        let mut rows = Vec::new();
        let mut visiting = vec![res.payload_name.clone(), provider.name.clone()];
        compose_spawn_lock(
            &provider.manifest.requires,
            allow_download,
            ctx,
            &mut visiting,
            &mut rows,
        )?;
        if !rows.is_empty() {
            env.push((
                tpkg::runtime_store::SPAWN_LOCK_VAR.to_string(),
                rows.join(";"),
            ));
        }
        // The dispatcher's jail (the consumer payload's needs ∩ the
        // user's tightening) always wins (spec 08 §2).
        env.extend(jail_env);
        return Ok(ExecPlan {
            program: pair.exe.clone(),
            argv,
            env,
            mounts,
            runtime: RuntimeResolution::Ready(Box::new(pair)),
        });
    }
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

    // spec 30 §4 + spec 32 §5: the payload's spawn edges resolve at
    // dispatch (runtimes download per the caller; provider payloads are
    // cache-only — install is the explicit verb) and pin the driver's
    // spawn-time picks via TEBAKO_SPAWN_LOCK — runtime rows
    // (`engine=lang_version:tebako_version`) and payload rows
    // (`provider@version=engine=lv:tv`), `;`-joined, manifest order,
    // composed TRANSITIVELY (a provider payload's own spawn edges join
    // the one lock — the spawned child has no loader). A payload without
    // spawn edges exports nothing.
    let mut spawn_lock = Vec::new();
    let mut visiting = vec![res.payload_name.clone()];
    compose_spawn_lock(
        res.manifest.requires(),
        allow_download,
        ctx,
        &mut visiting,
        &mut spawn_lock,
    )?;
    if !spawn_lock.is_empty() {
        env.push((
            tpkg::runtime_store::SPAWN_LOCK_VAR.to_string(),
            spawn_lock.join(";"),
        ));
    }

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
    let mut out = Vec::new();
    if let Some(user) = &user {
        // spec 32 §4 (locked): operator tightening is HEREDITARY — the
        // parent's user directives ride every spawned child as the
        // ceiling over the child's own recomputed union (a spawned child
        // never holds a grant the operator denied the parent). The
        // driver's spawn plan intersects it in; it inherits onward to
        // deeper spawns (the plan's env-op block never strips it).
        out.push((
            tpkg::runtime_store::JAIL_TIGHTENING_VAR.to_string(),
            user.to_env_spec(&[]),
        ));
    }
    let Some((jail, source)) = tpkg::jail::effective(request, user.as_ref()) else {
        return Ok(out);
    };
    if jail.is_trivially_open() {
        return Ok(out);
    }
    let arg_files = if jail.argument_files.auto {
        tpkg::jail::resolve_argument_files(args)
    } else {
        Vec::new()
    };
    out.extend([
        ("TEBAKO_JAIL".to_string(), jail.to_env_spec(&arg_files)),
        ("TEBAKO_JAIL_SOURCE".to_string(), source.to_string()),
        (
            "TEBAKO_JAIL_JOURNAL".to_string(),
            ctx.home.join("journal.log").to_string_lossy().into_owned(),
        ),
    ]);
    Ok(out)
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
