//! The spawned-runtime surface (spec 30): a consumer payload's
//! `requires[].kind: runtime` edge is NEVER co-mounted — the depended
//! runtime lives in the store's `runtimes/` area and its exposed
//! entrypoints are spawned as separate processes through the driver's
//! plan. This module is the runtime-side half: the boot captures the
//! edge map from the app payload's manifest ([`capture`]), and the
//! interpreter's spawn hook resolves a bare command name through it
//! ([`plan`] — behind the `tebako_spawn_runtime_plan` FFI).
//!
//! The pieces, all fail-closed:
//!
//! - **The expose map.** Only names the FIRST payload's
//!   `requires[].expose` lists are planned; anything else passes through
//!   untouched (`Ok(None)` — the interpreter's own spawn semantics,
//!   never a silent rewrite). A bare name carrying a path separator is
//!   an explicit path and is never planned.
//! - **Resolution.** The dispatch-time lock (`TEBAKO_SPAWN_LOCK`,
//!   composed by the shim) pins an engine to the exact
//!   `<lang_version>:<tebako_version>` the dispatcher picked; a locked
//!   entry resolves that pair or fails by name. An unlocked edge
//!   resolves cache-only (`resolve_spawned`): a spawn NEVER downloads —
//!   the operator installs ahead (`tebako install`) or dispatches
//!   through the shim.
//! - **The child argv.** `[exe, --tebako-image <triple>…,
//!   --tebako-entry <name>, <user args…>]` — the named entrypoint
//!   resolves in the CHILD against its own env image's
//!   `runtimeProvides.entrypoints` (spec 17 §1's bare-name rule: the
//!   declaration is the single owner; `args_default` composes
//!   child-side, never parent-side).
//! - **Carried mounts (POSIX).** A user argument embedded in this
//!   boot's mounts rides into the child as `--tebako-image` triples for
//!   every decl at that argument's mount point (union members included),
//!   minus decls at the runtime root (the child's own env image owns
//!   that spelling — re-sending it is EEXIST). An argument under the
//!   runtime root, or under a mount with no serializable decl (a memory
//!   mount), is materialized parent-side through the spawn-safe exec
//!   cache and rewritten to the host twin. On windows every embedded
//!   argument materializes and no mounts are carried (the platform has
//!   no interposition to share a namespace through).
//! - **The child env.** The parent's runtime-wiring vars never leak:
//!   the jail trio, the mounts/preload/dll/mount-root/lock vars, the
//!   platform injection var, and every `TEBAKO_MOUNT_*` discovery var
//!   are deleted; the child receives its own `TEBAKO_RUNTIME_IMAGE` and
//!   — when the union of the runtime's and the payload's declared host
//!   needs is non-trivial — the jail trio with source
//!   `spawn-edge:<payload>` (spec 30 §4: the spawned runtime's needs and
//!   the consumer payload's needs BOTH hold; the union is the grant,
//!   never one side silently winning).
//!
//! Spec 32 (the spawned PAYLOAD dependency) widens the surface: an
//! expose-carrying `kind: executable` edge plans the PROVIDER payload's
//! own spec-17 dispatch as the child — the provider's image mounted
//! whole at `/` (the first `--tebako-image` triple, so the bare entry
//! resolves against it), its declared dep mounts alongside, the
//! provider's entrypoint `runtime_requirement` resolving the child's
//! runtime (cache-only, or the payload lock row's nested pair). The
//! child receives a FRESH `TEBAKO_SPAWN_LOCK` composed transitively over
//! the provider's own spawn edges (the spawned child has no loader), the
//! jail union is three-way (consumer payload + provider payload +
//! provider runtime, source `spawn-edge:<consumer>:<provider>`), and the
//! hereditary operator ceiling (`TEBAKO_JAIL_TIGHTENING`, captured at
//! boot, never stripped) intersects every spawned child's recomputed
//! union — a `record` tightening dominates wholesale, mirroring
//! `jail::effective` (spec 32 §4).
//!
//! The runtime facts (the depended env image's entrypoint declarations
//! and host needs) are read by scratch-mounting its image once per
//! store entry and caching the parse — the mount dance is serialized on
//! the facts-cache mutex, so concurrent spawns never race the point.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use tfs::context::context;
use tpkg::jail::HostJail;
use tpkg::runtime_store::{CachedRuntime, SpawnLockEntry};
use tpkg::{Entrypoint, Provides, Requirement};

use crate::driver::{join_mount, manifest, mounted_manifest_at, DriverError, Env};
use crate::handoff::ImageSpec;

/// One spawnable command edge (spec 30 §1 + spec 32 §1): the two
/// spawn-carrying edge kinds, keyed identically by exposed name.
#[derive(Debug, Clone)]
pub(crate) enum SpawnEdge {
    /// A `kind: runtime` edge — the depended runtime's own boot serves
    /// the exposed name.
    Runtime {
        engine: String,
        implementation: Option<String>,
        constraint: tpkg::Constraint,
    },
    /// An expose-carrying `kind: executable` edge (spec 32) — the
    /// PROVIDER payload's own full spec-17 dispatch serves the exposed
    /// name as a child process.
    Payload {
        /// The capability (the edge's `name`).
        name: String,
        /// The by-name provider pin (the AmbiguousProvider escape hatch).
        payload: Option<String>,
        constraint: tpkg::Constraint,
    },
}

/// The boot-captured spawn state (see the module doc). `None` until a
/// boot with images captures the first payload's edges; a plain boot
/// (no images) captures nothing and plans nothing.
struct SpawnState {
    /// Command name → the edge that serves it. Names are unique across
    /// all of the payload's spawn edges (a duplicate is a named 65 at
    /// capture — the payload's self-description is ambiguous).
    exposes: HashMap<String, SpawnEdge>,
    /// The app payload's own name (the jail-source spelling).
    payload_name: String,
    /// The app payload's declared host needs (provides.capabilities.host).
    payload_needs: Option<HostJail>,
    /// The dispatch-time pin (parsed TEBAKO_SPAWN_LOCK).
    lock: Vec<SpawnLockEntry>,
    /// The hereditary operator ceiling (parsed TEBAKO_JAIL_TIGHTENING —
    /// spec 32 §4): the parent dispatch's user tightening, intersected
    /// over every spawned child's recomputed union.
    tightening: Option<HostJail>,
    /// The effective runtime root this boot established (the carried-mount
    /// exclusion and the scratch-mount base).
    runtime_root: String,
    /// The TEBAKO_MOUNT_* keys this boot exported (the launcher tier's
    /// delete list — the FFI recomputes its own from the live env).
    mount_var_keys: Vec<String>,
}

static STATE: RwLock<Option<SpawnState>> = RwLock::new(None);

/// The depended runtime's parsed spawn surface, cached per store entry
/// (the exe's dir — content-addressed by the store layout) so repeated
/// spawns never re-read the image.
struct RuntimeFacts {
    entrypoints: Vec<Entrypoint>,
    needs: Option<HostJail>,
}

static FACTS: Mutex<Option<HashMap<PathBuf, Arc<RuntimeFacts>>>> = Mutex::new(None);

/// The scratch point the facts read mounts the depended env image at —
/// under the runtime root (a nested mount; longest-prefix dispatch keeps
/// it out of the env image's way), unmounted before the read returns.
const SCRATCH_POINT: &str = "__tebako_spawn__";

/// The resolved plan the FFI hands back (spec 30 §2).
#[derive(Debug)]
pub struct SpawnPlan {
    /// The runtime exe (host path — the child's argv[0]).
    pub exe: String,
    /// The full child argv INCLUDING argv[0].
    pub argv: Vec<String>,
    /// Env operations in application order: `(key, Some(value))` sets,
    /// `(key, None)` deletes.
    pub env_ops: Vec<(String, Option<String>)>,
}

/// Which arguments the carried-mount scan covers: the spawn form knows
/// its argv and carries only mounts its arguments touch; the PATH
/// launcher form (spec 30 §3) knows none and carries them all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Carry {
    ForArgs,
    All,
}

/// Capture the spawn state at boot (spec 30 §2's setup half). Called on
/// the image-boot path after the mounts are established and before the
/// PATH wiring (the launcher tier reads the map). The FIRST triple is
/// the app payload (spec 17 §1) — its manifest's runtime edges and host
/// needs are the surface. A payload without runtime edges captures an
/// empty map (every later plan passes through); a corrupt manifest
/// stays the named 65 it is everywhere; a malformed TEBAKO_SPAWN_LOCK
/// on a payload WITH edges is the dispatcher's channel lying — a named
/// 65, never a guessed-around boot.
pub(crate) fn capture(
    images: &[ImageSpec],
    env: &dyn Env,
    runtime_root: &str,
    mount_var_keys: Vec<String>,
) -> Result<(), DriverError> {
    let Some(first) = images.first() else {
        return Ok(());
    };
    let Some(manifest_doc) = mounted_manifest_at(&first.mount)? else {
        return Ok(());
    };
    let mut exposes: HashMap<String, SpawnEdge> = HashMap::new();
    for req in &manifest_doc.requires {
        let (expose, edge) = match req {
            Requirement::Runtime {
                engine,
                implementation,
                constraint,
                expose,
            } => (
                expose,
                SpawnEdge::Runtime {
                    engine: engine.clone(),
                    implementation: implementation.clone(),
                    constraint: constraint.clone(),
                },
            ),
            Requirement::Executable {
                name,
                payload,
                constraint,
                expose,
                ..
            } => (
                expose,
                SpawnEdge::Payload {
                    name: name.clone(),
                    payload: payload.clone(),
                    constraint: constraint.clone(),
                },
            ),
            _ => continue,
        };
        for name in expose {
            if name.contains('/') || name.contains('\\') || name.contains(':') {
                return Err(manifest(format!(
                    "requires[].expose name '{name}' is not a bare command name — the spawn surface never shadows a path (spec 30 §1)"
                )));
            }
            if exposes.insert(name.clone(), edge.clone()).is_some() {
                return Err(manifest(format!(
                    "requires[].expose name '{name}' is declared by two spawn edges — the spawn surface is ambiguous (spec 30 §1, spec 32 §1)"
                )));
            }
        }
    }
    let payload_needs = match &manifest_doc.provides {
        Provides::App(app) => app.capabilities.host.clone(),
        _ => None,
    };
    let lock = match (
        exposes.is_empty(),
        crate::driver::env_var(env, tpkg::runtime_store::SPAWN_LOCK_VAR),
    ) {
        (false, Some(value)) => tpkg::runtime_store::parse_spawn_lock(&value).map_err(|e| {
            manifest(format!(
                "{}: {e} — the dispatcher's spawn pin is torn (spec 30 §2)",
                tpkg::runtime_store::SPAWN_LOCK_VAR
            ))
        })?,
        _ => Vec::new(),
    };
    // spec 32 §4 (locked): the hereditary operator ceiling — the parent
    // dispatch's user tightening, intersected over every spawned child's
    // recomputed union. Parsed only when spawn edges exist (a spawn-free
    // boot never plans); a torn value on such a boot is the dispatcher's
    // channel lying — a named 65, never a guessed-around boot.
    let tightening = match (
        exposes.is_empty(),
        crate::driver::env_var(env, tpkg::runtime_store::JAIL_TIGHTENING_VAR),
    ) {
        (false, Some(value)) => {
            Some(HostJail::parse_env_spec(&value).map_err(|e| {
                manifest(format!(
                    "{}: {e} — the dispatcher's jail ceiling is torn (spec 32 §4)",
                    tpkg::runtime_store::JAIL_TIGHTENING_VAR
                ))
            })?)
        }
        _ => None,
    };
    *STATE.write().unwrap() = Some(SpawnState {
        exposes,
        payload_name: manifest_doc.identity.name.clone(),
        payload_needs,
        lock,
        tightening,
        runtime_root: runtime_root.to_string(),
        mount_var_keys,
    });
    Ok(())
}

/// The exposed command names in capture order-ish (the PATH launcher
/// tier's set; spec 30 §3). Empty until a boot captures edges.
pub(crate) fn expose_names() -> Vec<String> {
    STATE
        .read()
        .unwrap()
        .as_ref()
        .map(|s| s.exposes.keys().cloned().collect())
        .unwrap_or_default()
}

/// Plan the spawn of `command` (a bare name) with user arguments `args`
/// (spec 30 §2). `mount_var_keys` names the live TEBAKO_MOUNT_* vars to
/// delete (the FFI scans the process env; the tests name their own).
/// `Ok(None)` is the pass-through: not a bare name, or no payload edge
/// exposes it — the interpreter's own spawn semantics proceed. `Err` is
/// a NAMED error (the message is the whole contract — the FFI hands it
/// to the caller verbatim): the exposed command cannot be planned and
/// the spawn must NOT fall through to a host binary of the same name.
pub fn plan(
    command: &str,
    args: &[String],
    mount_var_keys: &[String],
) -> Result<Option<SpawnPlan>, String> {
    if command.contains('/') || command.contains('\\') || command.contains(':') {
        return Ok(None);
    }
    let (edge, guard) = {
        let guard = STATE.read().unwrap();
        let Some(edge) = guard.as_ref().and_then(|s| s.exposes.get(command).cloned()) else {
            return Ok(None);
        };
        (edge, guard)
    };
    let state = guard.as_ref().unwrap();
    compose_plan(state, command, &edge, args, Carry::ForArgs, mount_var_keys).map(Some)
}

/// The launcher-tier plan (spec 30 §3): the same composition with no
/// known arguments and every serializable mount carried. pub(crate) for
/// `crate::path_env`.
pub(crate) fn plan_launcher(name: &str) -> Result<SpawnPlan, String> {
    let guard = STATE.read().unwrap();
    let (state, edge) = guard
        .as_ref()
        .and_then(|s| s.exposes.get(name).map(|e| (s, e)))
        .ok_or_else(|| format!("spawn launcher '{name}' without a captured expose edge"))?;
    let keys = state.mount_var_keys.clone();
    compose_plan(state, name, edge, &[], Carry::All, &keys)
}

/// The composition core (both forms).
fn compose_plan(
    state: &SpawnState,
    name: &str,
    edge: &SpawnEdge,
    args: &[String],
    carry: Carry,
    mount_var_keys: &[String],
) -> Result<SpawnPlan, String> {
    let home = tpkg::runtime_store::tebako_home(|k| std::env::var(k).ok())
        .map_err(|e| format!("spawn '{name}': {e}"))?;
    match edge {
        SpawnEdge::Runtime {
            engine,
            implementation,
            constraint,
        } => {
            let rt = resolve_edge(
                &home,
                name,
                engine,
                implementation.as_deref(),
                constraint,
                &state.lock,
            )?;
            let facts = runtime_facts(&rt, &state.runtime_root)?;
            if !facts.entrypoints.iter().any(|e| e.name == name) {
                return Err(format!(
                    "spawn '{name}': payload '{}' exposes it for engine '{engine}' but runtime {} {} declares no entrypoint of that name — the payload's expose list outruns the runtime's spawn surface (spec 30 §2)",
                    state.payload_name, rt.lang_version, rt.tebako_version
                ));
            }
            let (triples, args) = carry_mounts(state, args, carry, &[])?;
            let image = rt
                .image
                .as_ref()
                .ok_or_else(|| {
                    format!(
                        "spawn '{name}': runtime {} {} resolved without its env image — an exe-only cache entry cannot boot (spec 30 §2)",
                        rt.lang_version, rt.tebako_version
                    )
                })?
                .to_string_lossy()
                .into_owned();
            let open = HostJail::open();
            let union = tpkg::jail::union(
                facts.needs.as_ref().unwrap_or(&open),
                state.payload_needs.as_ref().unwrap_or(&open),
            );
            let env_ops = child_env_ops(
                state,
                &image,
                union,
                &home,
                &args,
                mount_var_keys,
                format!("spawn-edge:{}", state.payload_name),
            );
            let mut argv = Vec::with_capacity(2 + triples.len() * 2 + 2 + args.len());
            argv.push(rt.exe.to_string_lossy().into_owned());
            for triple in &triples {
                argv.push("--tebako-image".to_string());
                argv.push(triple.clone());
            }
            argv.push("--tebako-entry".to_string());
            argv.push(name.to_string());
            argv.extend(args);
            Ok(SpawnPlan {
                exe: argv[0].clone(),
                argv,
                env_ops,
            })
        }
        SpawnEdge::Payload {
            name: capability,
            payload: pin,
            constraint,
        } => compose_payload_plan(state, name, capability, pin.as_deref(), constraint, args, carry, mount_var_keys, &home),
    }
}

/// spec 32 §2: the spawned-PAYLOAD plan. The provider resolves cache-only
/// (the lock's payload row pinning it when the dispatcher composed one);
/// the exposed entrypoint's `runtime_requirement` resolves the child's
/// runtime (the lock row's nested pair when locked). The child argv
/// mounts the provider image FIRST at `/` (the bare entry resolves
/// against the first triple — spec 17 §1), then the provider's declared
/// dep mounts, then the carried parent mounts (minus `/` and the dep
/// points — the child owns those spellings, re-sending them is EEXIST).
#[allow(clippy::too_many_arguments)]
fn compose_payload_plan(
    state: &SpawnState,
    command: &str,
    capability: &str,
    pin: Option<&str>,
    constraint: &tpkg::Constraint,
    args: &[String],
    carry: Carry,
    mount_var_keys: &[String],
    home: &std::path::Path,
) -> Result<SpawnPlan, String> {
    let (provider, locked_row) =
        resolve_provider(home, command, capability, pin, constraint, &state.lock)?;
    // The exposed name must be a declared, runtime-carrying entrypoint of
    // the provider — validated even when the lock pins the pair (the
    // dispatcher's validation is not the driver's evidence).
    provider_entrypoint(&provider, capability, command)?;
    let expose: Vec<String> = state
        .exposes
        .iter()
        .filter(|(_, e)| {
            matches!(e, SpawnEdge::Payload { name, .. } if name == capability)
        })
        .map(|(n, _)| n.clone())
        .collect();
    let rt = nested_runtime(home, &provider, capability, &expose, locked_row.as_ref())?;
    let facts = runtime_facts(&rt, &state.runtime_root)?;
    let dep_mounts = provider_dep_mounts(home, &provider, &state.lock)?;
    let mut exclude: Vec<String> = dep_mounts
        .iter()
        .filter_map(|t| t.rsplitn(2, ':').next().map(str::to_string))
        .collect();
    exclude.push("/".to_string());
    let (triples, args) = carry_mounts(state, args, carry, &exclude)?;
    let image = rt
        .image
        .as_ref()
        .ok_or_else(|| {
            format!(
                "spawn '{command}': runtime {} {} resolved without its env image — an exe-only cache entry cannot boot (spec 32 §2)",
                rt.lang_version, rt.tebako_version
            )
        })?
        .to_string_lossy()
        .into_owned();
    let mut child_lock: Vec<String> = Vec::new();
    let mut visiting = vec![state.payload_name.clone(), provider.name.clone()];
    compose_child_lock(home, &provider, &state.lock, &mut visiting, &mut child_lock)
        .map_err(|e| format!("spawn '{command}': {e}"))?;
    let provider_needs = match &provider.manifest.provides {
        Provides::App(app) => app.capabilities.host.clone(),
        _ => None,
    };
    let open = HostJail::open();
    let union = tpkg::jail::union(
        &tpkg::jail::union(
            state.payload_needs.as_ref().unwrap_or(&open),
            provider_needs.as_ref().unwrap_or(&open),
        ),
        facts.needs.as_ref().unwrap_or(&open),
    );
    let mut env_ops = child_env_ops(
        state,
        &image,
        union,
        home,
        &args,
        mount_var_keys,
        format!("spawn-edge:{}:{}", state.payload_name, provider.name),
    );
    if !child_lock.is_empty() {
        env_ops.push((
            tpkg::runtime_store::SPAWN_LOCK_VAR.to_string(),
            Some(child_lock.join(";")),
        ));
    }
    let provider_image = provider.image.to_string_lossy().into_owned();
    let mut argv = Vec::with_capacity(2 + (1 + dep_mounts.len() + triples.len()) * 2 + 2 + args.len());
    argv.push(rt.exe.to_string_lossy().into_owned());
    argv.push("--tebako-image".to_string());
    argv.push(format!("{provider_image}:0:/"));
    for triple in dep_mounts.iter().chain(&triples) {
        argv.push("--tebako-image".to_string());
        argv.push(triple.clone());
    }
    argv.push("--tebako-entry".to_string());
    argv.push(command.to_string());
    argv.extend(args);
    Ok(SpawnPlan {
        exe: argv[0].clone(),
        argv,
        env_ops,
    })
}

/// Resolve the edge against the store (spec 30 §2): the dispatch lock
/// pins the exact pair when it names this engine; otherwise the
/// cache-only newest-compatible pick. Never a download.
fn resolve_edge(
    home: &std::path::Path,
    name: &str,
    engine: &str,
    implementation: Option<&str>,
    constraint: &tpkg::Constraint,
    lock: &[SpawnLockEntry],
) -> Result<CachedRuntime, String> {
    if let Some(locked) = lock
        .iter()
        .find(|e| e.payload.is_none() && e.engine == engine)
    {
        return tpkg::runtime_store::resolve_locked(
            home,
            engine,
            implementation,
            &locked.lang_version,
            &locked.tebako_version,
        )
        .ok_or_else(|| {
            format!(
                "spawn '{name}': dispatch-locked {engine}={}:{} has vanished from the store — re-run through the shim or `tebako install` the runtime (spec 30 §2)",
                locked.lang_version, locked.tebako_version
            )
        });
    }
    let constraint = tpkg::versions::from_validated(constraint);
    let hit = tpkg::runtime_store::resolve_spawned(home, engine, implementation, &constraint);
    if let Some(rt) = &hit {
        tebako_log::log!(
            tebako_log::Level::Debug,
            "spawn",
            "event=unlocked-pick name={} engine={} lang_version={} tebako_version={} — no dispatch lock; newest compatible cache entry picked",
            name,
            engine,
            rt.lang_version,
            rt.tebako_version
        );
    }
    hit.ok_or_else(|| {
        format!(
            "spawn '{name}': no cached runtime satisfies engine '{engine}' — a spawn never downloads: `tebako install` the runtime ahead, or dispatch through the shim (spec 30 §2)"
        )
    })
}

/// spec 32 §5: resolve an executable edge's provider payload from the
/// store — CACHE-ONLY (a spawn never downloads). The dispatch lock's
/// payload rows pin first: the row whose provider pin matches the edge's
/// `payload:` pin, or — unpinned — the row whose pinned record declares
/// the capability. A locked row whose record vanished from the store is
/// a named error, never a slide. Unlocked: the pin names the provider
/// (newest satisfying installed version); without it the capability scan
/// answers — zero candidates and more-than-one provider payload are both
/// named errors (AmbiguousProvider escapes via the `payload:` pin).
/// Returns the provider record and the matched lock row (its nested
/// runtime pair pins the child's runtime).
fn resolve_provider(
    home: &std::path::Path,
    command: &str,
    capability: &str,
    pin: Option<&str>,
    constraint: &tpkg::Constraint,
    lock: &[SpawnLockEntry],
) -> Result<(tpkg::payload_store::CachedPayload, Option<SpawnLockEntry>), String> {
    for row in lock {
        let Some((locked_name, locked_version)) = &row.payload else {
            continue;
        };
        if let Some(pin) = pin {
            if locked_name != pin {
                continue;
            }
        }
        let record = tpkg::payload_store::get(home, locked_name, locked_version)
            .map_err(|e| format!("spawn '{command}': {e}"))?
            .ok_or_else(|| {
                format!(
                    "spawn '{command}': dispatch-locked payload {locked_name}@{locked_version} has vanished from the store — re-run through the shim or `tebako install` the payload (spec 32 §5)"
                )
            })?;
        if pin.is_some() || declares_capability(&record, capability) {
            return Ok((record, Some(row.clone())));
        }
    }
    let evaluable = tpkg::versions::from_validated(constraint);
    if let Some(pin) = pin {
        let installed = tpkg::payload_store::installed_versions(home, pin)
            .map_err(|e| format!("spawn '{command}': {e}"))?;
        let version = installed
            .iter()
            .filter(|v| evaluable.matches(v))
            .max_by(|a, b| tpkg::versions::compare(a, b));
        let Some(version) = version else {
            return Err(format!(
                "spawn '{command}': provider payload {pin} has no installed version satisfying '{}' — a spawn never downloads: `tebako install {pin}` ahead, or dispatch through the shim (spec 32 §5)",
                constraint.as_str()
            ));
        };
        let record = tpkg::payload_store::get(home, pin, version)
            .map_err(|e| format!("spawn '{command}': {e}"))?
            .ok_or_else(|| {
                format!(
                    "spawn '{command}': the installed record of provider payload {pin} {version} is incomplete — re-install it with `tebako install {pin}` (spec 32 §5)"
                )
            })?;
        return Ok((record, None));
    }
    let candidates = tpkg::payload_store::find_capability_providers(home, capability, constraint)
        .map_err(|e| format!("spawn '{command}': {e}"))?;
    let mut names: Vec<String> = candidates.iter().map(|p| p.name.clone()).collect();
    names.sort();
    names.dedup();
    match names.len() {
        0 => Err(format!(
            "spawn '{command}': no installed payload provides executable '{capability}' (DependencyNotFound) — a spawn never downloads: `tebako install` a provider ahead, or dispatch through the shim (spec 32 §5)"
        )),
        1 => {
            let provider_name = names.pop().unwrap_or_default();
            candidates
                .into_iter()
                .filter(|p| p.name == provider_name)
                .max_by(|a, b| tpkg::versions::compare(&a.version, &b.version))
                .map(|p| (p, None))
                .ok_or_else(|| {
                    format!("spawn '{command}': provider payload {provider_name} vanished mid-resolution")
                })
        }
        _ => Err(format!(
            "spawn '{command}': executable '{capability}' is provided by more than one installed payload ({}) (AmbiguousProvider) — pin the provider with `payload:` on the edge (spec 32 §1)",
            names.join(", ")
        )),
    }
}

/// The capability a cached payload declares (spec 32 §1): exact-name
/// match against `provides.entrypoints[].name` ∪
/// `provides.executables[].name`.
fn declares_capability(record: &tpkg::payload_store::CachedPayload, capability: &str) -> bool {
    match &record.manifest.provides {
        Provides::App(app) => app.entrypoints.iter().any(|e| e.name == capability),
        Provides::Toolkit(tk) => tk.executables.iter().any(|e| e.name == capability),
        _ => false,
    }
}

/// The provider's entrypoint an exposed name dispatches to (spec 32 §1):
/// declared in the provider's `provides.entrypoints` and CARRYING
/// `runtime_requirement` — a runtime-less match (a toolkit executable, a
/// native entrypoint) is a named error, never an exec-tier fallback.
fn provider_entrypoint<'p>(
    provider: &'p tpkg::payload_store::CachedPayload,
    capability: &str,
    exposed: &str,
) -> Result<&'p Entrypoint, String> {
    let Provides::App(app) = &provider.manifest.provides else {
        return Err(format!(
            "executable edge '{capability}': provider payload {} {} is not an app payload — it declares no entrypoints to spawn (spec 32 §1)",
            provider.name, provider.version
        ));
    };
    let Some(entry) = app.entrypoints.iter().find(|e| e.name == exposed) else {
        return Err(format!(
            "executable edge '{capability}': provider payload {} {} declares no entrypoint '{exposed}' — the expose list outruns the provider's declaration (spec 32 §1)",
            provider.name, provider.version
        ));
    };
    if entry.runtime_requirement.is_none() {
        return Err(format!(
            "executable edge '{capability}': the provider's entrypoint '{exposed}' carries no runtime_requirement — a runtime-less entry has no spawn form, its surface is the exec tier (spec 32 §1)"
        ));
    }
    Ok(entry)
}

/// The child's runtime for a spawned payload (spec 32 §2/§5): the lock
/// row's nested pair when the dispatcher pinned one (a vanished pair is
/// a named error, never a slide); otherwise each exposed entrypoint's
/// own `runtime_requirement` resolves cache-only and every exposed name
/// must agree on the SAME pair — a disagreement is a named error (split
/// the edge per runtime).
fn nested_runtime(
    home: &std::path::Path,
    provider: &tpkg::payload_store::CachedPayload,
    capability: &str,
    expose: &[String],
    locked_row: Option<&SpawnLockEntry>,
) -> Result<CachedRuntime, String> {
    if let Some(row) = locked_row {
        return tpkg::runtime_store::resolve_locked(
            home,
            &row.engine,
            None,
            &row.lang_version,
            &row.tebako_version,
        )
        .ok_or_else(|| {
            format!(
                "executable edge '{capability}': dispatch-locked {}={}:{} (nested in the {}@{} row) has vanished from the store — re-run through the shim or `tebako install` the runtime (spec 32 §5)",
                row.engine, row.lang_version, row.tebako_version, provider.name, provider.version
            )
        });
    }
    let mut picked: Option<CachedRuntime> = None;
    for exposed in expose {
        let entry = provider_entrypoint(provider, capability, exposed)?;
        let req = entry
            .runtime_requirement
            .as_ref()
            .expect("provider_entrypoint post-asserts runtime_requirement");
        let constraint = tpkg::versions::from_validated(&req.constraint);
        let rt = tpkg::runtime_store::resolve_spawned(home, &req.engine, None, &constraint)
            .ok_or_else(|| {
                format!(
                    "executable edge '{capability}': no cached runtime satisfies engine '{}' ('{}') for provider {} {} — a spawn never downloads: `tebako install` the runtime ahead, or dispatch through the shim (spec 32 §2)",
                    req.engine, req.constraint.as_str(), provider.name, provider.version
                )
            })?;
        if let (Some(want), Some(got)) = (&req.abi, &rt.abi) {
            if want != got {
                return Err(format!(
                    "executable edge '{capability}': the exposed entry '{exposed}' requires abi '{want}' but the cached {} runtime is '{got}' — a spawn never downloads: `tebako install` a matching runtime (spec 32 §2)",
                    req.engine
                ));
            }
        }
        match &picked {
            None => picked = Some(rt),
            Some(p)
                if p.engine == rt.engine
                    && p.lang_version == rt.lang_version
                    && p.tebako_version == rt.tebako_version => {}
            Some(p) => {
                return Err(format!(
                    "executable edge '{capability}': the exposed entries disagree on the runtime pair ({} {} tebako {} vs {} {} tebako {}) — one payload row nests ONE pair (spec 32 §5); split the edge per runtime",
                    p.engine, p.lang_version, p.tebako_version, rt.engine, rt.lang_version, rt.tebako_version
                ));
            }
        }
    }
    picked.ok_or_else(|| format!("executable edge '{capability}' exposes no entries"))
}

/// The provider payload's declared dep mounts (spec 32 §2): each
/// mount-carrying `requires` edge of the provider's manifest mirror
/// resolves cache-only against the payload store into a
/// `<image>:0:<mount>` triple for the child. `kind: language`/`runtime`
/// edges never mount (the runtime axis; the spawn surface); a miss is a
/// named error, never a skipped mount (a partial mount is forbidden).
fn provider_dep_mounts(
    home: &std::path::Path,
    provider: &tpkg::payload_store::CachedPayload,
    lock: &[SpawnLockEntry],
) -> Result<Vec<String>, String> {
    let mut triples = Vec::new();
    for req in &provider.manifest.requires {
        let (dep, mount) = match req {
            Requirement::Executable {
                name,
                payload,
                constraint,
                mount: Some(mount),
                ..
            } => (
                resolve_provider(home, name, name, payload.as_deref(), constraint, lock)?.0,
                mount,
            ),
            Requirement::Toolkit {
                name,
                constraint,
                mount: Some(mount),
                ..
            }
            | Requirement::Data {
                name,
                constraint,
                mount: Some(mount),
            } => {
                let evaluable = tpkg::versions::from_validated(constraint);
                let installed = tpkg::payload_store::installed_versions(home, name)
                    .map_err(|e| format!("provider '{}': {e}", provider.name))?;
                let version = installed
                    .iter()
                    .filter(|v| evaluable.matches(v))
                    .max_by(|a, b| tpkg::versions::compare(a, b));
                let Some(version) = version else {
                    return Err(format!(
                        "provider '{}' requires payload {name} but no satisfying version is installed — a spawn never downloads: `tebako install {name}` ahead (spec 32 §2)",
                        provider.name
                    ));
                };
                let record = tpkg::payload_store::get(home, name, version)
                    .map_err(|e| format!("provider '{}': {e}", provider.name))?
                    .ok_or_else(|| {
                        format!(
                            "provider '{}': the installed record of {name} {version} is incomplete — re-install it with `tebako install {name}` (spec 32 §2)",
                            provider.name
                        )
                    })?;
                (record, mount)
            }
            _ => continue,
        };
        triples.push(format!("{}:0:{}", dep.image.to_string_lossy(), mount));
    }
    Ok(triples)
}

/// The spawned payload child's FRESH spawn lock (spec 32 §5): composed
/// over the provider's own spawn edges, transitively — the child has no
/// loader, so the pins compose at the parent's plan time. A runtime edge
/// reuses the parent lock's row for the engine when one rides (the same
/// artifact the parent resolved), else resolves cache-only; an
/// expose-carrying executable edge resolves its provider (the parent
/// lock's payload rows pin the whole closure the dispatcher composed)
/// and recurses. Identical rows dedupe; `visiting` is the payload-name
/// cycle guard — a cycle through spawn edges is a named error, never a
/// recursion trap.
fn compose_child_lock(
    home: &std::path::Path,
    provider: &tpkg::payload_store::CachedPayload,
    parent_lock: &[SpawnLockEntry],
    visiting: &mut Vec<String>,
    rows: &mut Vec<String>,
) -> Result<(), String> {
    for edge in &provider.manifest.requires {
        match edge {
            Requirement::Runtime {
                engine,
                implementation,
                constraint,
                ..
            } => {
                let row = match parent_lock
                    .iter()
                    .find(|e| e.payload.is_none() && &e.engine == engine)
                {
                    Some(locked) => tpkg::runtime_store::spawn_lock_entry(
                        engine,
                        &locked.lang_version,
                        &locked.tebako_version,
                    ),
                    None => {
                        let evaluable = tpkg::versions::from_validated(constraint);
                        let rt = tpkg::runtime_store::resolve_spawned(
                            home,
                            engine,
                            implementation.as_deref(),
                            &evaluable,
                        )
                        .ok_or_else(|| {
                            format!(
                                "provider '{}' requires runtime '{engine}' but no cached runtime satisfies it — a spawn never downloads: `tebako install` the runtime ahead (spec 32 §5)",
                                provider.name
                            )
                        })?;
                        tpkg::runtime_store::spawn_lock_entry(
                            engine,
                            &rt.lang_version,
                            &rt.tebako_version,
                        )
                    }
                };
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
                let (nested, locked_row) =
                    resolve_provider(home, name, name, payload.as_deref(), constraint, parent_lock)?;
                if visiting.iter().any(|p| p == &nested.name) {
                    return Err(format!(
                        "spawn dependency cycle through provider payload '{}' ({}): the executable edges form a cycle — break it (spec 32 §2)",
                        nested.name,
                        visiting.join(" -> ")
                    ));
                }
                let pair = nested_runtime(home, &nested, name, expose, locked_row.as_ref())?;
                let row = tpkg::runtime_store::spawn_lock_payload_entry(
                    &nested.name,
                    &nested.version,
                    &pair.engine,
                    &pair.lang_version,
                    &pair.tebako_version,
                );
                if !rows.contains(&row) {
                    rows.push(row);
                }
                visiting.push(nested.name.clone());
                compose_child_lock(home, &nested, parent_lock, visiting, rows)?;
                visiting.pop();
            }
            _ => {}
        }
    }
    Ok(())
}

/// The depended runtime's spawn surface, cached per store entry. A cache
/// miss scratch-mounts the env image under the runtime root, reads its
/// in-image manifest through the VFS, and unmounts — the facts-cache
/// mutex serializes the dance so concurrent spawns never race the point.
fn runtime_facts(rt: &CachedRuntime, runtime_root: &str) -> Result<Arc<RuntimeFacts>, String> {
    let key = rt.dir.clone();
    let mut guard = FACTS.lock().unwrap();
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some(facts) = cache.get(&key) {
        return Ok(Arc::clone(facts));
    }
    let image = rt.image.as_ref().ok_or_else(|| {
        format!(
            "runtime {} {} has no cached env image — cannot read its spawn surface",
            rt.lang_version, rt.tebako_version
        )
    })?;
    let point = join_mount(runtime_root, SCRATCH_POINT);
    let mount = tfs::mount::build_from_file(&image.to_string_lossy(), &point).map_err(|e| {
        format!(
            "cannot mount the env image '{}' for the spawn-surface read: {}",
            image.display(),
            crate::driver::errno_text(e)
        )
    })?;
    let handle = context()
        .write()
        .unwrap()
        .mount_checked(mount)
        .map_err(|e| {
            format!(
                "cannot mount the env image '{}' at '{point}': {}",
                image.display(),
                crate::driver::errno_text(e)
            )
        })?;
    let text = crate::driver::read_mounted_text(&join_mount(&point, tpkg::PAYLOAD_MANIFEST_PATH));
    let _ = context().write().unwrap().unmount_handle(handle);
    let text = text.map_err(|e| {
        format!(
            "the env image '{}' carries no readable {} — a runtime's spawn surface is declared in its image manifest (spec 30 §2): {}",
            image.display(),
            tpkg::PAYLOAD_MANIFEST_PATH,
            crate::driver::errno_text(e)
        )
    })?;
    let manifest_doc = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
        format!(
            "corrupt {} in the env image '{}' — the runtime's self-description lies: {e}",
            tpkg::PAYLOAD_MANIFEST_PATH,
            image.display()
        )
    })?;
    let Provides::Runtime(runtime) = &manifest_doc.provides else {
        return Err(format!(
            "the env image '{}' is not a runtime payload — its spawn surface cannot resolve (spec 30 §2)",
            image.display()
        ));
    };
    let facts = Arc::new(RuntimeFacts {
        entrypoints: runtime.entrypoints.clone(),
        needs: runtime.capabilities.host.clone(),
    });
    cache.insert(key, Arc::clone(&facts));
    Ok(facts)
}

/// The carried-mount scan (see the module doc): the child triples and
/// the rewritten argument vector. POSIX carries the mounts an argument
/// touches (all decls at the argument's point, establishment order,
/// deduped, runtime-root decls excluded); an argument under the runtime
/// root, under an EXCLUDED point (`exclude` — the spawned payload plan's
/// `/` and the provider's dep mount points, which the CHILD's own mounts
/// own), or under an unserializable mount materializes parent-side.
/// Windows materializes every embedded argument and carries nothing.
fn carry_mounts(
    state: &SpawnState,
    args: &[String],
    carry: Carry,
    exclude: &[String],
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut rewritten: Vec<String> = args.to_vec();
    let mut decls_out: Vec<tfs::mount_spec::MountDecl> = Vec::new();
    {
        let ctx = context().read().unwrap();
        let decls = ctx.mount_decls();
        #[cfg(not(windows))]
        {
            let mut materialize_idx: Vec<usize> = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                let Some(point) = ctx.mount_point_of(arg) else {
                    continue;
                };
                let at_point: Vec<_> = decls.iter().filter(|d| d.mount == point).collect();
                if point == state.runtime_root
                    || exclude.iter().any(|p| p == &point)
                    || at_point.is_empty()
                {
                    // The child's own env image owns the root spelling;
                    // an excluded point is the child's own mount (the
                    // spawned payload's `/` and dep points); a memory
                    // mount has no image to carry. All three answer with
                    // the materialized host twin.
                    materialize_idx.push(i);
                    continue;
                }
                for d in at_point {
                    if !decls_out
                        .iter()
                        .any(|e| e.image == d.image && e.slot == d.slot && e.mount == d.mount)
                    {
                        decls_out.push(d.clone());
                    }
                }
            }
            drop(ctx);
            for i in materialize_idx {
                rewritten[i] = materialize_arg(&args[i])?;
            }
        }
        #[cfg(windows)]
        {
            let mut materialize_idx: Vec<usize> = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                if ctx.path_is_embedded(arg) {
                    materialize_idx.push(i);
                }
            }
            drop(ctx);
            for i in materialize_idx {
                rewritten[i] = materialize_arg(&args[i])?;
            }
        }
    }
    if carry == Carry::All {
        // The PATH launcher knows no arguments: every serializable mount
        // (minus the runtime root and the excluded points — the child's
        // own spellings) rides — the establishment order, deduped.
        let ctx = context().read().unwrap();
        for d in ctx.mount_decls() {
            if d.mount == state.runtime_root || exclude.contains(&d.mount) {
                continue;
            }
            if !decls_out
                .iter()
                .any(|e| e.image == d.image && e.slot == d.slot && e.mount == d.mount)
            {
                decls_out.push(d);
            }
        }
    }
    let triples = decls_out
        .iter()
        .map(|d| {
            format!(
                "{}:{}:{}",
                d.image,
                d.slot
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                d.mount
            )
        })
        .collect();
    Ok((triples, rewritten))
}

/// The spawn-safe materialization of one embedded argument (spec 22 §2's
/// spawn routing: a home-annotated mount extracts whole — a JVM's
/// self-located prefix never strands).
fn materialize_arg(arg: &str) -> Result<String, String> {
    context()
        .write()
        .unwrap()
        .exec_materialize_for_spawn(arg)
        .map(|c| c.to_string_lossy().into_owned())
        .map_err(|e| {
            format!(
                "cannot materialize the embedded argument '{arg}' for the spawned child: {}",
                crate::driver::errno_text(e)
            )
        })
}

/// The child env operations (see the module doc): deletes first, then
/// the runtime image and — when the effective jail is non-trivial — the
/// jail trio with `source`. `union` is the pre-ceiling union the caller
/// composed (two-way for a spawned runtime, three-way for a spawned
/// payload); the hereditary ceiling (spec 32 §4) applies over it: a
/// `record` tightening dominates wholesale, otherwise the tightening
/// intersects the union. `TEBAKO_JAIL_TIGHTENING` itself is never
/// deleted — heredity rides plain inheritance.
fn child_env_ops(
    state: &SpawnState,
    image: &str,
    union: HostJail,
    home: &std::path::Path,
    args: &[String],
    mount_var_keys: &[String],
    source: String,
) -> Vec<(String, Option<String>)> {
    let mut ops: Vec<(String, Option<String>)> = Vec::new();
    // The parent's runtime wiring never leaks into the child — the
    // child boots its own env image and rebuilds everything from it.
    for key in [
        "TEBAKO_JAIL",
        "TEBAKO_JAIL_SOURCE",
        "TEBAKO_JAIL_JOURNAL",
        crate::injection::MOUNTS_VAR,
        crate::injection::PRELOAD_SHIM_VAR,
        crate::injection::RUNTIME_DLL_VAR,
        "TEBAKO_MOUNT_ROOT",
        tpkg::runtime_store::SPAWN_LOCK_VAR,
        // The platform's injection var: a foreign preload in the child
        // would bind the PARENT's shim symbols. Deleted on every
        // platform — a var that was never set deletes to nothing.
        "LD_PRELOAD",
        "DYLD_INSERT_LIBRARIES",
    ] {
        ops.push((key.to_string(), None));
    }
    for key in mount_var_keys {
        ops.push((key.clone(), None));
    }
    ops.push(("TEBAKO_RUNTIME_IMAGE".to_string(), Some(image.to_string())));
    let effective = match &state.tightening {
        Some(t) if t.record => t.clone(),
        Some(t) => tpkg::jail::intersect(&union, t),
        None => union,
    };
    if !effective.is_trivially_open() {
        let resolved = if effective.argument_files.auto {
            tpkg::jail::resolve_argument_files(args)
        } else {
            Vec::new()
        };
        ops.push((
            "TEBAKO_JAIL".to_string(),
            Some(effective.to_env_spec(&resolved)),
        ));
        ops.push(("TEBAKO_JAIL_SOURCE".to_string(), Some(source)));
        ops.push((
            "TEBAKO_JAIL_JOURNAL".to_string(),
            Some(home.join("journal.log").to_string_lossy().into_owned()),
        ));
    }
    ops
}

/// Test support: drop the captured state and the facts cache (the boot
/// fixtures' reset discipline — a fresh boot must not inherit a previous
/// boot's surface).
#[cfg(test)]
pub(crate) fn reset() {
    *STATE.write().unwrap() = None;
    *FACTS.lock().unwrap() = None;
}

/// Test support: install a state directly (the pure planner's unit
/// tests run without a boot).
#[cfg(test)]
pub(crate) fn install_state(
    exposes: HashMap<String, SpawnEdge>,
    payload_name: &str,
    payload_needs: Option<HostJail>,
    lock: Vec<SpawnLockEntry>,
    tightening: Option<HostJail>,
    runtime_root: &str,
) {
    *STATE.write().unwrap() = Some(SpawnState {
        exposes,
        payload_name: payload_name.to_string(),
        payload_needs,
        lock,
        tightening,
        runtime_root: runtime_root.to_string(),
        mount_var_keys: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::sync::MutexGuard;

    // STATE, FACTS, the tfs context, and the process env (TEBAKO_HOME)
    // are all process-global — the suite serializes.
    static LOCK: Mutex<()> = Mutex::new(());

    struct Guard {
        _guard: MutexGuard<'static, ()>,
        home: PathBuf,
        prior_home: Option<String>,
    }

    fn guard(tag: &str) -> Guard {
        let g = LOCK.lock().unwrap();
        reset();
        context().write().unwrap().unmount();
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home =
            std::env::temp_dir().join(format!("tebako-spawn-{tag}-{}-{uniq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let prior_home = std::env::var("TEBAKO_HOME").ok();
        std::env::set_var("TEBAKO_HOME", &home);
        Guard {
            _guard: g,
            home,
            prior_home,
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            match &self.prior_home {
                Some(v) => std::env::set_var("TEBAKO_HOME", v),
                None => std::env::remove_var("TEBAKO_HOME"),
            }
            reset();
            context().write().unwrap().unmount();
            let _ = std::fs::remove_dir_all(&self.home);
        }
    }

    /// A runtime-manifest fixture (spec 03 §2.2 + spec 30 §2's additive
    /// entrypoints). `entrypoints`/`host` are YAML fragments.
    fn runtime_manifest(engine: &str, entrypoints: &str, host: &str) -> String {
        let z = "0".repeat(64);
        format!(
            "identity:\n  schema_version: 1\n  kind: runtime\n  name: {engine}\n  version: \"21.0.12\"\n  \
             producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-13T00:00:00Z\"\n  \
             digest: {{tree_hash: \"sha256:{z}\", blob_sha256: {z}}}\n  \
             signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  provides: {{engine: {engine}, version: \"21.0.12\", abi_line: \"21\", platform: aarch64-macos}}\n  \
             built_from: {{src_sha256: {z}, patch_set: base}}\n\
             {entrypoints}  capabilities: {{exec: true, read: true, runtime: true{host}}}\n"
        )
    }

    /// A real zip image carrying the manifest (the scratch-mount read's
    /// target — tfs mounts it for real).
    fn build_image(path: &Path, manifest: &str) {
        let file = std::fs::File::create(path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zw.add_directory("__tpkg__/", opts).unwrap();
        zw.start_file("__tpkg__/manifest.yaml", opts).unwrap();
        zw.write_all(manifest.as_bytes()).unwrap();
        zw.finish().unwrap();
    }

    /// A store entry: exe + image pair + trust sidecar (the spawned
    /// resolution's eligibility rule — image required).
    fn store_entry(home: &Path, engine: &str, lv: &str, ver: &str, manifest: &str) {
        let platform = tpkg::runtime_store::platform_string();
        let dir = home
            .join("runtimes")
            .join(format!("{engine}-{lv}-{ver}-{platform}"));
        std::fs::create_dir_all(&dir).unwrap();
        #[cfg(not(windows))]
        let exe = format!("tebako-runtime-{ver}-{lv}-{platform}");
        #[cfg(windows)]
        let exe = format!("tebako-runtime-{ver}-{lv}-{platform}.exe");
        std::fs::write(dir.join(exe), b"exe").unwrap();
        let image = format!("tebako-runtime-{ver}-{lv}-{platform}.tfs");
        build_image(&dir.join(&image), manifest);
        std::fs::write(dir.join(format!("{image}.sha256")), b"x").unwrap();
    }

    /// An installed PAYLOAD record (spec 32 §5's resolution target): the
    /// image (a plain file — the plan never mounts it, the mirror is the
    /// plan-time manifest source), the trust anchor, the manifest mirror.
    fn seed_payload(home: &Path, name: &str, version: &str, manifest: &str) {
        let dir = tpkg::payload_store::payload_dir(home, name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{version}.tfs")), b"image").unwrap();
        std::fs::write(dir.join(format!("{version}.tfs.sha256")), "0\n").unwrap();
        std::fs::write(dir.join(format!("{version}.manifest.yaml")), manifest).unwrap();
    }

    /// An app-payload manifest mirror fixture. `entrypoints` and
    /// `requires` are YAML fragments (requires may be empty).
    fn app_manifest(name: &str, version: &str, entrypoints: &str, requires: &str) -> String {
        let z = "0".repeat(64);
        format!(
            "identity:\n  schema_version: 1\n  schema_minor: 5\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  \
             producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-09-05T00:00:00Z\"\n  \
             digest: {{tree_hash: \"sha256:{z}\", blob_sha256: {z}}}\n  \
             signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  entrypoints: [{entrypoints}]\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n\
             {requires}"
        )
    }

    fn edge(constraint: &str) -> SpawnEdge {
        SpawnEdge::Runtime {
            engine: "java".to_string(),
            implementation: None,
            constraint: tpkg::Constraint::new(constraint).unwrap(),
        }
    }

    fn payload_edge(pin: Option<&str>, constraint: &str) -> SpawnEdge {
        SpawnEdge::Payload {
            name: "xml2rfc".to_string(),
            payload: pin.map(str::to_string),
            constraint: tpkg::Constraint::new(constraint).unwrap(),
        }
    }

    fn one_expose() -> HashMap<String, SpawnEdge> {
        HashMap::from([("java".to_string(), edge(">= 21"))])
    }

    fn state_with(exposes: HashMap<String, SpawnEdge>) {
        install_state(exposes, "app", None, Vec::new(), None, "/__tfs__");
    }

    /// The env-op for a key, LAST write wins (the ops apply in order:
    /// the unconditional deletes lead, the sets follow).
    fn op<'p>(plan: &'p SpawnPlan, key: &str) -> Option<&'p Option<String>> {
        plan.env_ops
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    #[test]
    fn paths_and_unexposed_names_pass_through() {
        let _g = guard("pass");
        state_with(one_expose());
        assert!(plan("/usr/bin/java", &[], &[]).unwrap().is_none());
        assert!(plan("bin\\java", &[], &[]).unwrap().is_none());
        assert!(plan("javac", &[], &[]).unwrap().is_none());
        // No captured state at all: everything passes through.
        reset();
        assert!(plan("java", &[], &[]).unwrap().is_none());
    }

    #[test]
    fn the_happy_path_plans_the_child_boot() {
        let g = guard("happy");
        store_entry(
            &g.home,
            "java",
            "21.0.12",
            "0.3.0",
            &runtime_manifest("java", "  entrypoints: [{name: java, path: /bin/java}]\n", ""),
        );
        state_with(one_expose());
        let plan = plan(
            "java",
            &["-version".to_string()],
            &["TEBAKO_MOUNT_TOOLS".to_string()],
        )
        .unwrap()
        .expect("planned");
        // argv: [exe, --tebako-entry, java, -version] — no mounts exist
        // in this context, so no triples ride.
        assert_eq!(plan.argv.len(), 4, "{:?}", plan.argv);
        assert!(
            plan.argv[0].contains("tebako-runtime-0.3.0-21.0.12"),
            "{:?}",
            plan.argv
        );
        assert_eq!(plan.exe, plan.argv[0]);
        assert_eq!(plan.argv[1], "--tebako-entry");
        assert_eq!(plan.argv[2], "java");
        assert_eq!(plan.argv[3], "-version");
        // The child gets its own runtime image; the parent's wiring is
        // deleted (the mount-var key included); no jail SET op — the
        // union of two absent needs is trivially open (the delete still
        // rides, clearing an inherited one).
        let image = op(&plan, "TEBAKO_RUNTIME_IMAGE")
            .and_then(|v| v.as_ref())
            .expect("image set");
        assert!(image.ends_with(".tfs"), "{image}");
        assert_eq!(op(&plan, "TEBAKO_MOUNT_TOOLS"), Some(&None));
        assert_eq!(op(&plan, "TEBAKO_JAIL"), Some(&None));
        assert_eq!(op(&plan, tpkg::runtime_store::SPAWN_LOCK_VAR), Some(&None));
        assert_eq!(op(&plan, crate::injection::MOUNTS_VAR), Some(&None));
        assert_eq!(op(&plan, "TEBAKO_MOUNT_ROOT"), Some(&None));
        // …and the source/journal keys carry only their deletes.
        assert_eq!(op(&plan, "TEBAKO_JAIL_SOURCE"), Some(&None));
        assert_eq!(op(&plan, "TEBAKO_JAIL_JOURNAL"), Some(&None));
    }

    #[test]
    fn the_cache_only_rule_is_named() {
        let _g = guard("miss");
        state_with(one_expose());
        let err = plan("java", &[], &[]).unwrap_err();
        assert!(err.contains("never downloads"), "{err}");
        assert!(err.contains("tebako install"), "{err}");
    }

    #[test]
    fn the_dispatch_lock_pins_and_vanishes_by_name() {
        let g = guard("lock");
        // Two cache entries; the lock pins the OLDER pair.
        store_entry(
            &g.home,
            "java",
            "21.0.12",
            "0.3.0",
            &runtime_manifest("java", "  entrypoints: [{name: java, path: /bin/java}]\n", ""),
        );
        store_entry(
            &g.home,
            "java",
            "21.0.11",
            "0.3.0",
            &runtime_manifest("java", "  entrypoints: [{name: java, path: /bin/java}]\n", ""),
        );
        install_state(
            one_expose(),
            "app",
            None,
            vec![SpawnLockEntry {
                engine: "java".to_string(),
                lang_version: "21.0.11".to_string(),
                tebako_version: "0.3.0".to_string(),
                payload: None,
            }],
            None,
            "/__tfs__",
        );
        let pinned = plan("java", &[], &[]).unwrap().expect("planned");
        assert!(pinned.exe.contains("21.0.11"), "{}", pinned.exe);

        // The pinned pair gone from the store: a named error, never a
        // slide onto the newer entry.
        install_state(
            one_expose(),
            "app",
            None,
            vec![SpawnLockEntry {
                engine: "java".to_string(),
                lang_version: "21.0.99".to_string(),
                tebako_version: "0.3.0".to_string(),
                payload: None,
            }],
            None,
            "/__tfs__",
        );
        let err = plan("java", &[], &[]).unwrap_err();
        assert!(err.contains("dispatch-locked"), "{err}");
        assert!(err.contains("21.0.99"), "{err}");
    }

    #[test]
    fn an_undeclared_expose_is_a_named_error() {
        let g = guard("undeclared");
        // The runtime declares `java` only; the payload exposes `jing`.
        store_entry(
            &g.home,
            "java",
            "21.0.12",
            "0.3.0",
            &runtime_manifest("java", "  entrypoints: [{name: java, path: /bin/java}]\n", ""),
        );
        let mut exposes = one_expose();
        exposes.insert("jing".to_string(), edge(">= 21"));
        state_with(exposes);
        let err = plan("jing", &[], &[]).unwrap_err();
        assert!(err.contains("outruns"), "{err}");
        assert!(err.contains("jing"), "{err}");
    }

    #[test]
    fn the_jail_union_rides_the_child_env() {
        let g = guard("jail");
        // The runtime needs /usr/share read-only…
        store_entry(
            &g.home,
            "java",
            "21.0.12",
            "0.3.0",
            &runtime_manifest(
                "java",
                "  entrypoints: [{name: java, path: /bin/java}]\n",
                ", host: {default: deny, mounts: [{host: /usr/share, mount: /usr/share, access: ro}]}",
            ),
        );
        // …and the payload needs /data read-write. Both hold (the
        // union), and the source names the spawn edge.
        let payload_needs = HostJail::from_yaml(
            "default: deny\nmounts:\n - {host: /data, mount: /data, access: rw}\n",
        )
        .unwrap();
        install_state(
            one_expose(),
            "metanorma",
            Some(payload_needs),
            Vec::new(),
            None,
            "/__tfs__",
        );
        let plan = plan("java", &[], &[]).unwrap().expect("planned");
        let jail = op(&plan, "TEBAKO_JAIL")
            .and_then(|v| v.as_ref())
            .expect("jail set");
        let parsed = tpkg::jail::HostJail::parse_env_spec(jail).unwrap();
        assert!(!parsed.default_open, "{jail}");
        assert_eq!(parsed.mounts.len(), 2, "{jail}");
        assert!(
            jail.contains("/usr/share") && jail.contains("/data"),
            "{jail}"
        );
        assert_eq!(
            op(&plan, "TEBAKO_JAIL_SOURCE").and_then(|v| v.as_deref()),
            Some("spawn-edge:metanorma")
        );
        assert!(op(&plan, "TEBAKO_JAIL_JOURNAL")
            .and_then(|v| v.as_ref())
            .is_some_and(|j| j.ends_with("journal.log")));
    }

    #[test]
    fn the_facts_cache_balances_the_scratch_mount() {
        let g = guard("facts");
        store_entry(
            &g.home,
            "java",
            "21.0.12",
            "0.3.0",
            &runtime_manifest("java", "  entrypoints: [{name: java, path: /bin/java}]\n", ""),
        );
        state_with(one_expose());
        // Two plans of the same edge both succeed — the scratch mount
        // is mounted and unmounted once, the parse cached.
        plan("java", &[], &[]).unwrap().expect("first");
        plan("java", &[], &[]).unwrap().expect("second");
    }

    /// The spec 32 fixtures: a python runtime pair and the xml2rfc
    /// provider payload whose entrypoint requires it.
    fn seed_python(home: &Path) {
        store_entry(
            home,
            "python",
            "3.12.3",
            "0.3.0",
            &runtime_manifest("python", "  entrypoints: []\n", ""),
        );
    }

    fn seed_xml2rfc(home: &Path, requires: &str) {
        seed_payload(
            home,
            "xml2rfc",
            "3.34.0",
            &app_manifest(
                "xml2rfc",
                "3.34.0",
                "{name: xml2rfc, path: /bin/xml2rfc, runtime_requirement: {engine: python, constraint: \">= 3.10\"}}",
                requires,
            ),
        );
    }

    fn xml2rfc_expose() -> HashMap<String, SpawnEdge> {
        HashMap::from([("xml2rfc".to_string(), payload_edge(None, ">= 3.34"))])
    }

    #[test]
    fn the_payload_edge_plans_the_providers_own_dispatch() {
        let g = guard("payload-plan");
        seed_python(&g.home);
        seed_xml2rfc(&g.home, "");
        install_state(xml2rfc_expose(), "metanorma", None, Vec::new(), None, "/__tfs__");
        let plan = plan("xml2rfc", &["--version".to_string()], &[])
            .unwrap()
            .expect("planned");
        // argv: [python-exe, --tebako-image <provider.tfs>:0:/,
        // --tebako-entry xml2rfc, --version] — the provider image is the
        // FIRST triple (the bare entry resolves against it, spec 17 §1).
        assert_eq!(plan.argv.len(), 6, "{:?}", plan.argv);
        assert!(
            plan.argv[0].contains("tebako-runtime-0.3.0-3.12.3"),
            "{:?}",
            plan.argv
        );
        assert_eq!(plan.argv[1], "--tebako-image");
        assert!(
            plan.argv[2].ends_with("payloads/xml2rfc/3.34.0.tfs:0:/"),
            "{:?}",
            plan.argv
        );
        assert_eq!(plan.argv[3], "--tebako-entry");
        assert_eq!(plan.argv[4], "xml2rfc");
        // The child's env image is the PYTHON runtime's; the spawn lock
        // rides only its delete (the provider declares no spawn edges).
        let image = op(&plan, "TEBAKO_RUNTIME_IMAGE")
            .and_then(|v| v.as_ref())
            .expect("image set");
        assert!(
            image.contains("python-3.12.3-0.3.0") && image.ends_with(".tfs"),
            "{image}"
        );
        assert_eq!(op(&plan, tpkg::runtime_store::SPAWN_LOCK_VAR), Some(&None));
    }

    #[test]
    fn the_payload_lock_row_pins_and_vanishes_by_name() {
        let g = guard("payload-lock");
        seed_python(&g.home);
        seed_xml2rfc(&g.home, "");
        let locked = SpawnLockEntry {
            engine: "python".to_string(),
            lang_version: "3.12.3".to_string(),
            tebako_version: "0.3.0".to_string(),
            payload: Some(("xml2rfc".to_string(), "3.34.0".to_string())),
        };
        install_state(
            xml2rfc_expose(),
            "metanorma",
            None,
            vec![locked],
            None,
            "/__tfs__",
        );
        let planned = plan("xml2rfc", &[], &[]).unwrap().expect("planned");
        assert!(planned.exe.contains("3.12.3"), "{}", planned.exe);

        // The pinned provider record gone from the store: a named error,
        // never a slide onto another version.
        let vanished = SpawnLockEntry {
            payload: Some(("xml2rfc".to_string(), "9.9.9".to_string())),
            ..SpawnLockEntry {
                engine: "python".to_string(),
                lang_version: "3.12.3".to_string(),
                tebako_version: "0.3.0".to_string(),
                payload: None,
            }
        };
        install_state(
            xml2rfc_expose(),
            "metanorma",
            None,
            vec![vanished],
            None,
            "/__tfs__",
        );
        let err = plan("xml2rfc", &[], &[]).unwrap_err();
        assert!(err.contains("dispatch-locked payload"), "{err}");
        assert!(err.contains("xml2rfc@9.9.9"), "{err}");
    }

    #[test]
    fn the_payload_edge_names_its_resolution_failures() {
        let g = guard("payload-errors");
        install_state(xml2rfc_expose(), "metanorma", None, Vec::new(), None, "/__tfs__");
        // No provider installed: DependencyNotFound, never a download.
        let err = plan("xml2rfc", &[], &[]).unwrap_err();
        assert!(err.contains("DependencyNotFound"), "{err}");
        assert!(err.contains("never downloads"), "{err}");
        // Two providers of the same capability: AmbiguousProvider.
        seed_xml2rfc(&g.home, "");
        seed_payload(
            &g.home,
            "xml2rfc-alt",
            "3.35.0",
            &app_manifest(
                "xml2rfc-alt",
                "3.35.0",
                "{name: xml2rfc, path: /bin/xml2rfc, runtime_requirement: {engine: python, constraint: \">= 3.10\"}}",
                "",
            ),
        );
        let err = plan("xml2rfc", &[], &[]).unwrap_err();
        assert!(err.contains("AmbiguousProvider"), "{err}");
        assert!(err.contains("xml2rfc-alt"), "{err}");
        // …and the pin is the escape hatch.
        let mut exposes = HashMap::new();
        exposes.insert("xml2rfc".to_string(), payload_edge(Some("xml2rfc"), ">= 3.34"));
        install_state(exposes, "metanorma", None, Vec::new(), None, "/__tfs__");
        let err = plan("xml2rfc", &[], &[]).unwrap_err();
        assert!(err.contains("no cached runtime"), "{err}");
        // A runtime-less provider entrypoint has no spawn form.
        seed_python(&g.home);
        seed_payload(
            &g.home,
            "xml2rfc",
            "3.35.0",
            &app_manifest(
                "xml2rfc",
                "3.35.0",
                "{name: xml2rfc, path: /bin/xml2rfc}",
                "",
            ),
        );
        let err = plan("xml2rfc", &[], &[]).unwrap_err();
        assert!(err.contains("runtime_requirement"), "{err}");
    }

    #[test]
    fn the_payload_child_carries_a_fresh_transitive_lock() {
        let g = guard("payload-child-lock");
        seed_python(&g.home);
        // The provider's OWN spawn surface: a java runtime edge. The
        // child has no loader — the parent's dispatch lock row for java
        // flows into the child's fresh lock verbatim.
        seed_xml2rfc(
            &g.home,
            "requires:\n  - kind: runtime\n    engine: java\n    constraint: \">= 21\"\n    expose: [java]\n",
        );
        let parent_lock = vec![SpawnLockEntry {
            engine: "java".to_string(),
            lang_version: "21.0.11".to_string(),
            tebako_version: "0.3.0".to_string(),
            payload: None,
        }];
        install_state(
            xml2rfc_expose(),
            "metanorma",
            None,
            parent_lock,
            None,
            "/__tfs__",
        );
        let plan = plan("xml2rfc", &[], &[]).unwrap().expect("planned");
        let lock = op(&plan, tpkg::runtime_store::SPAWN_LOCK_VAR)
            .and_then(|v| v.as_ref())
            .expect("child lock set");
        assert_eq!(lock, "java=21.0.11:0.3.0", "{lock}");
    }

    #[test]
    fn a_spawn_edge_cycle_is_a_named_error() {
        let g = guard("payload-cycle");
        seed_python(&g.home);
        // xml2rfc spawns toolb; toolb-pkg spawns xml2rfc back.
        seed_xml2rfc(
            &g.home,
            "requires:\n  - kind: executable\n    name: toolb\n    payload: toolb-pkg\n    constraint: \">= 1.0\"\n    expose: [toolb]\n",
        );
        seed_payload(
            &g.home,
            "toolb-pkg",
            "1.2.0",
            &app_manifest(
                "toolb-pkg",
                "1.2.0",
                "{name: toolb, path: /bin/toolb, runtime_requirement: {engine: python, constraint: \">= 3.10\"}}",
                "requires:\n  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.34\"\n    expose: [xml2rfc]\n",
            ),
        );
        install_state(xml2rfc_expose(), "metanorma", None, Vec::new(), None, "/__tfs__");
        let err = plan("xml2rfc", &[], &[]).unwrap_err();
        assert!(err.contains("cycle"), "{err}");
        assert!(err.contains("xml2rfc"), "{err}");
    }

    #[test]
    fn the_hereditary_ceiling_tightens_every_spawned_child() {
        let g = guard("ceiling");
        let ceiling = HostJail::from_yaml("default: deny\n").unwrap();
        // A spawned RUNTIME child: the union of two absent needs is
        // trivially open, yet the ceiling forces the deny jail.
        store_entry(
            &g.home,
            "java",
            "21.0.12",
            "0.3.0",
            &runtime_manifest("java", "  entrypoints: [{name: java, path: /bin/java}]\n", ""),
        );
        install_state(
            one_expose(),
            "app",
            None,
            Vec::new(),
            Some(ceiling.clone()),
            "/__tfs__",
        );
        let plan_java = plan("java", &[], &[]).unwrap().expect("planned");
        let jail = op(&plan_java, "TEBAKO_JAIL")
            .and_then(|v| v.as_ref())
            .expect("ceiling forces the jail");
        assert!(
            !tpkg::jail::HostJail::parse_env_spec(jail)
                .unwrap()
                .default_open,
            "{jail}"
        );
        // A spawned PAYLOAD child: same ceiling, three-way source.
        seed_python(&g.home);
        seed_xml2rfc(&g.home, "");
        install_state(
            xml2rfc_expose(),
            "metanorma",
            None,
            Vec::new(),
            Some(ceiling),
            "/__tfs__",
        );
        let plan = plan("xml2rfc", &[], &[]).unwrap().expect("planned");
        let jail = op(&plan, "TEBAKO_JAIL")
            .and_then(|v| v.as_ref())
            .expect("ceiling forces the jail");
        assert!(
            !tpkg::jail::HostJail::parse_env_spec(jail)
                .unwrap()
                .default_open,
            "{jail}"
        );
        assert_eq!(
            op(&plan, "TEBAKO_JAIL_SOURCE").and_then(|v| v.as_deref()),
            Some("spawn-edge:metanorma:xml2rfc")
        );
    }
}
