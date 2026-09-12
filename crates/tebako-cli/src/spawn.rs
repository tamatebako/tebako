//! The press-time spawned-edge walk (spec 23 §13.6, spec 30 §2, spec 32
//! §6): `tebako press` reads the app image's L1 `requires:` and composes
//! the lock's `spawned[]` rows itself — one row per `kind: runtime` edge
//! (spec 30) and per expose-carrying `kind: executable` edge (spec 32),
//! in manifest order — resolving each edge exactly as managed dispatch
//! would (the runtime pair through the machine runtime store, the
//! provider payload through the registered registries into the payload
//! cache) and pinning the carried bytes' digests into the row.
//!
//! Press is NOT install: the resolved bytes land in the trust-anchored
//! caches, but no mirrors, shims, or materialized trees are written —
//! those are the install verb's surface (spec 07 §2). `carry` rides the
//! preset's defaults; a row the preset leaves SHARED splits by kind: a
//! shared runtime row is a named press error (press resolves runtimes
//! through the machine store and records no replayable `source:` — the
//! error advises `--mode=self-contained`; the hand-authored `spawned[]`
//! block stays the escape hatch, spec 23 §13.6), while a shared payload
//! row records its registry `source:` (no current preset produces one).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tebako_resolve::registry::RegistryPlatforms;
use tebako_resolve::{Fetcher, PayloadCache, ResolveError, Transport};
use tpkg::{ComposePreset, Platform};

use crate::compose;
use crate::error::TebakoError;
use crate::image_manifest;
use crate::install;

// The spec 06 §4 named set (the codes compose.rs/install.rs already own;
// the spawn walk's errors are manifest/resolution failures of the same
// family).
const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_SHA: i32 = 70;
const EX_TEBAKO_IO: i32 = 74;

fn err(message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, EX_TEBAKO_MANIFEST)
}

/// The walk's product: the lock's `spawned[]` rows (in the app
/// manifest's `requires:` order) plus the carried bytes as press image
/// specs (`(path, mount, format_id)` — the mount is always "", a carried
/// spawned artifact is never mounted; its slot is the spec's index in
/// press's image list).
#[derive(Debug, Default)]
pub struct SpawnedPlan {
    pub rows: Vec<tpkg::LockedSpawned>,
    pub images: Vec<(PathBuf, String, u32)>,
}

/// One spawn-bearing edge of the app manifest (the walk's unit).
enum SpawnEdge<'m> {
    /// A `kind: runtime` edge (spec 30 §1) — the row mirrors the edge
    /// whether or not it exposes names.
    Runtime {
        engine: &'m str,
        implementation: Option<&'m str>,
        constraint: &'m tpkg::Constraint,
        expose: &'m [String],
    },
    /// An expose-carrying `kind: executable` edge (spec 32 §1/§6).
    Executable {
        name: &'m str,
        payload: Option<&'m str>,
        constraint: &'m tpkg::Constraint,
        expose: &'m [String],
    },
}

/// Compose the lock's `spawned[]` rows for one press (spec 23 §13.6).
/// `home` is LAZY: a spawn-less app image (no embedded manifest, or no
/// spawn-bearing edges) composes no rows and never resolves the store
/// home — the plain press's shape is unchanged. `first_slot` is the
/// image-list index the walk's carried slots continue from (the app,
/// the carried compose slices, and the carried runtime pair sit below
/// it).
pub fn resolve_spawned_edges<H, T>(
    home: H,
    fetcher: &Fetcher<T>,
    app_image: &Path,
    app_name: &str,
    preset: ComposePreset,
    host: Platform,
    first_slot: u32,
) -> Result<SpawnedPlan, TebakoError>
where
    H: FnOnce() -> Result<PathBuf, TebakoError>,
    T: Transport,
{
    let Some(text) = image_manifest::read_embedded_manifest(app_image)? else {
        return Ok(SpawnedPlan::default());
    };
    let manifest = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
        err(format!(
            "the embedded manifest of {} does not parse: {e}",
            app_image.display()
        ))
    })?;

    let mut edges: Vec<SpawnEdge> = Vec::new();
    for requirement in &manifest.requires {
        // spec 03 §2.3 (schema_minor 9): an edge whose triplets: list
        // does not cover the press's target host contributes NO spawned
        // row and NO exposed name — the skip is loud, never an error.
        if !requirement.covers_host(host) {
            eprintln!("tebako: note: {}", requirement.platform_skip_note(host));
            continue;
        }
        match requirement {
            tpkg::Requirement::Runtime {
                engine,
                implementation,
                constraint,
                expose,
                ..
            } => {
                // The lock's own validator refuses two rows for one
                // engine+implementation — press refuses to compose them
                // first, naming the edges.
                if edges.iter().any(|e| {
                    matches!(e, SpawnEdge::Runtime { engine: e2, implementation: i2, .. }
                        if *e2 == engine && *i2 == implementation.as_deref())
                }) {
                    return Err(err(format!(
                        "the app payload declares two spawned runtime edges on engine '{engine}'{} — one lock.spawned[] row per engine+implementation (spec 23 §13.6)",
                        implementation
                            .as_deref()
                            .map(|i| format!(" (implementation '{i}')"))
                            .unwrap_or_default()
                    )));
                }
                edges.push(SpawnEdge::Runtime {
                    engine,
                    implementation: implementation.as_deref(),
                    constraint,
                    expose,
                });
            }
            tpkg::Requirement::Executable {
                name,
                payload,
                constraint,
                expose,
                ..
            } if !expose.is_empty() => {
                edges.push(SpawnEdge::Executable {
                    name,
                    payload: payload.as_deref(),
                    constraint,
                    expose,
                });
            }
            _ => {}
        }
    }
    if edges.is_empty() {
        return Ok(SpawnedPlan::default());
    }

    // The expose × own-entrypoint collision refusal (spec 30 §3, spec 32
    // §1) is the manifest validator's — `PayloadManifest::from_yaml`
    // already refused a colliding expose list above; the walk never sees
    // one.

    let home = home()?;
    let ctx = tebako_shim::Ctx {
        home: home.clone(),
        cwd: std::env::current_dir().map_err(|e| TebakoError::new(e.to_string(), EX_TEBAKO_IO))?,
        env: std::env::vars().collect(),
    };

    let mut plan = SpawnedPlan::default();
    let mut next_slot = first_slot;
    // One row per provider payload (the lock's validator refuses two) —
    // checked AFTER provider resolution: two edges naming different
    // capabilities may still resolve to the one provider.
    let mut seen_providers: Vec<(String, String)> = Vec::new();
    for edge in &edges {
        match edge {
            SpawnEdge::Runtime {
                engine,
                implementation,
                constraint,
                expose,
            } => {
                let row = spawned_runtime_row(
                    &ctx,
                    engine,
                    *implementation,
                    constraint,
                    expose,
                    preset,
                    &mut plan,
                    &mut next_slot,
                )?;
                plan.rows.push(tpkg::LockedSpawned::Runtime(row));
            }
            SpawnEdge::Executable {
                name,
                payload,
                constraint,
                expose,
            } => {
                let provider = match payload {
                    Some(p) => p.to_string(),
                    None => compose::compose_capability_provider(
                        &home, fetcher, app_name, name, constraint,
                    )?,
                };
                if let Some((first, _)) = seen_providers.iter().find(|(_, p)| p == &provider) {
                    return Err(err(format!(
                        "the executable edges \"{first}\" and \"{name}\" both resolve to the provider payload '{provider}' — one lock.spawned[] row per provider payload (spec 23 §13.6); merge the expose lists into one edge"
                    )));
                }
                seen_providers.push((name.to_string(), provider.clone()));
                let row = spawned_payload_row(
                    &home,
                    &ctx,
                    fetcher,
                    &manifest,
                    name,
                    &provider,
                    constraint,
                    expose,
                    preset,
                    host,
                    &mut plan,
                    &mut next_slot,
                )?;
                plan.rows.push(tpkg::LockedSpawned::Payload(row));
            }
        }
    }
    Ok(plan)
}

/// The shared-runtime row refusal (spec 23 §13.6): press resolves
/// spawned runtimes through the machine store and records no replayable
/// `source:`, so a shared row could not reproduce on another machine.
fn gate_carried_runtime(what: &str, preset: ComposePreset) -> Result<(), TebakoError> {
    if preset.default_carry(true) {
        return Ok(());
    }
    Err(err(format!(
        "{what} rides the shared-runtime preset, but press resolves spawned runtimes through the machine store and a shared row would record no replayable `source:` (spec 23 §13.6) — press --mode=self-contained to carry the pair, or hand-author the lock's spawned[] row"
    )))
}

/// The expose cross-check against the runtime image's OWN embedded
/// manifest (spec 30 §2's install-time class, mirrored at press — the
/// shim layer deliberately cannot read images; press can).
fn check_expose_against_runtime(
    engine: &str,
    rt: &tpkg::runtime_store::CachedRuntime,
    expose: &[String],
) -> Result<(), TebakoError> {
    if expose.is_empty() {
        return Ok(());
    }
    let image = rt.image.as_ref().ok_or_else(|| {
        err(format!(
            "the cached {engine} runtime {} (tebako {}) carries no verified env image — the expose list cannot be verified",
            rt.lang_version, rt.tebako_version
        ))
    })?;
    let text = image_manifest::read_embedded_manifest(image)?.ok_or_else(|| {
        err(format!(
            "the {engine} runtime image {} carries no embedded manifest — the expose list cannot be verified",
            image.display()
        ))
    })?;
    let manifest = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
        err(format!(
            "the embedded manifest of {} does not parse: {e}",
            image.display()
        ))
    })?;
    let tpkg::Provides::Runtime(provides) = &manifest.provides else {
        return Err(err(format!(
            "{} is not a runtime payload — a spawn edge can only expose a runtime's entrypoints",
            image.display()
        )));
    };
    for name in expose {
        if !provides.entrypoints.iter().any(|e| &e.name == name) {
            return Err(err(format!(
                "expose: the {engine} runtime {} declares no entrypoint \"{name}\" (declared: {}) — fix the payload's expose list or the runtime's spawn surface",
                rt.lang_version,
                provides
                    .entrypoints
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }
    Ok(())
}

/// Compose one §13.6 runtime row (spec 30 §2): the edge resolved through
/// the machine store (download allowed — the same posture as install's
/// pre-staging), the expose list cross-checked, the pair carried with
/// its digest pins.
#[allow(clippy::too_many_arguments)]
fn spawned_runtime_row(
    ctx: &tebako_shim::Ctx,
    engine: &str,
    implementation: Option<&str>,
    constraint: &tpkg::Constraint,
    expose: &[String],
    preset: ComposePreset,
    plan: &mut SpawnedPlan,
    next_slot: &mut u32,
) -> Result<tpkg::LockedSpawnedRuntime, TebakoError> {
    let rt =
        tebako_shim::runtime::resolve_runtime_edge(engine, implementation, constraint, true, ctx)
            .map_err(install::map_shim)?;
    check_expose_against_runtime(engine, &rt, expose)?;
    gate_carried_runtime(
        &format!(
            "the app payload's spawned runtime edge on engine '{engine}' ({})",
            constraint.as_str()
        ),
        preset,
    )?;
    carried_runtime_row(
        &rt,
        engine,
        implementation,
        constraint,
        expose.to_vec(),
        plan,
        next_slot,
    )
}

/// Assemble one carried §13.6 runtime row from a resolved store entry:
/// hash-pins the exe + env image (+ the windows dll facet when the
/// cached release index declares one — tebako-runtime-ruby#40) and
/// stages the bytes as slots in press's image list, exe first, then the
/// image, then the dll (the packed-mn oracle's slot order).
fn carried_runtime_row(
    rt: &tpkg::runtime_store::CachedRuntime,
    engine: &str,
    implementation: Option<&str>,
    constraint: &tpkg::Constraint,
    expose: Vec<String>,
    plan: &mut SpawnedPlan,
    next_slot: &mut u32,
) -> Result<tpkg::LockedSpawnedRuntime, TebakoError> {
    let hash = |path: &Path| -> Result<String, TebakoError> {
        crate::resolve::sha256_file_hex(path).ok_or_else(|| {
            TebakoError::new(
                format!(
                    "cannot hash the spawned runtime artifact {}",
                    path.display()
                ),
                EX_TEBAKO_IO,
            )
        })
    };
    let exe_sha = hash(&rt.exe)?;
    let exe_slot = *next_slot;
    *next_slot += 1;
    plan.images
        .push((rt.exe.clone(), String::new(), tpkg::TPKG_FORMAT_AUTO));
    let image = rt.image.clone().ok_or_else(|| {
        err(format!(
            "the cached {engine} runtime {} (tebako {}) carries no verified env image — a spawned row needs the pair",
            rt.lang_version, rt.tebako_version
        ))
    })?;
    let image_sha = hash(&image)?;
    let image_slot = *next_slot;
    *next_slot += 1;
    plan.images
        .push((image, String::new(), tpkg::TPKG_FORMAT_DWARFS));
    // The dll facet rides the cached index mirror's grammar (tpkg owns
    // it); a carried-staged cache entry holds no index and records none.
    let exe_name = rt
        .exe
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dll = match tpkg::runtime_store::entry_dll(&rt.dir, &exe_name) {
        Some(d) => {
            let dll_path = rt.dir.join(&d.install_as);
            if !dll_path.is_file() {
                return Err(err(format!(
                    "the cached {engine} runtime entry {} is corrupt: its release index declares the dll facet \"{}\" (install_as \"{}\") but the store holds no such file",
                    rt.dir.display(),
                    d.filename,
                    d.install_as
                )));
            }
            let dll_sha = hash(&dll_path)?;
            if dll_sha != d.sha256 {
                return Err(TebakoError::new(
                    format!(
                        "the cached {engine} runtime dll {} hashes to {dll_sha} but the release index pins {} — the store entry is damaged; reinstall the runtime",
                        dll_path.display(),
                        d.sha256
                    ),
                    EX_TEBAKO_SHA,
                ));
            }
            let slot = *next_slot;
            *next_slot += 1;
            plan.images
                .push((dll_path, String::new(), tpkg::TPKG_FORMAT_AUTO));
            Some(tpkg::LockedSpawnedArtifact {
                slot: Some(slot),
                sha256: tpkg::DigestPin::One(d.sha256),
                install_as: Some(d.install_as),
            })
        }
        None => None,
    };
    Ok(tpkg::LockedSpawnedRuntime {
        engine: engine.to_string(),
        implementation: implementation.map(str::to_string),
        constraint: constraint.clone(),
        expose,
        version: rt.lang_version.clone(),
        tebako: rt.tebako_version.clone(),
        carry: true,
        exe: tpkg::LockedSpawnedArtifact {
            slot: Some(exe_slot),
            sha256: tpkg::DigestPin::One(exe_sha),
            install_as: None,
        },
        image: tpkg::LockedSpawnedArtifact {
            slot: Some(image_slot),
            sha256: tpkg::DigestPin::One(image_sha),
            install_as: None,
        },
        dll,
        source: None,
    })
}

/// Compose one §13.6 payload row (spec 32 §6): the provider payload
/// resolved through the registries into the payload cache (the compose
/// closure's fetch → verify → cache tail), the expose list cross-checked
/// against the provider's embedded manifest, and the provider's OWN
/// `kind: language` edge resolved as the nested runtime row (its
/// constraint mirrored verbatim, its expose empty).
#[allow(clippy::too_many_arguments)]
fn spawned_payload_row<T: Transport>(
    home: &Path,
    ctx: &tebako_shim::Ctx,
    fetcher: &Fetcher<T>,
    app: &tpkg::PayloadManifest,
    edge_name: &str,
    provider: &str,
    constraint: &tpkg::Constraint,
    expose: &[String],
    preset: ComposePreset,
    host: Platform,
    plan: &mut SpawnedPlan,
    next_slot: &mut u32,
) -> Result<tpkg::LockedSpawnedPayload, TebakoError> {
    let consumer = format!("{} {}", app.identity.name, app.identity.version);

    // Registry resolution (the compose closure's tail): one registry
    // carrying the provider, the newest satisfying version, fetched +
    // verified + cached. Press is not install — no mirrors, no shims.
    let mut found = install::find_in_registries(home, fetcher, provider)?;
    let (reg_ref, registry_payload) = match found.len() {
        0 => {
            return Err(err(format!(
                "{consumer} requires executable {edge_name} but its provider payload '{provider}' is not carried by any registered registry — register one with: tebako add-registry <ref>"
            )));
        }
        1 => found.pop().expect("len == 1 checked"),
        n => {
            return Err(err(format!(
                "{consumer} requires executable {edge_name} and its provider payload '{provider}' is listed by {n} registered registries (AmbiguousRegistries):\n{}\n  narrow it with an explicit registry set",
                found
                    .iter()
                    .map(|(r, _)| format!("    - {r}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )));
        }
    };
    let eval = tpkg::versions::from_validated(constraint);
    let version = registry_payload
        .versions
        .iter()
        .map(|v| v.version.as_str())
        .filter(|v| eval.matches(v))
        .max_by(|a, b| tpkg::versions::compare(a, b))
        .map(str::to_string)
        .ok_or_else(|| {
            err(format!(
                "{consumer} requires executable {edge_name} but no published version of '{provider}' satisfies '{}' (available: {})",
                constraint.as_str(),
                registry_payload
                    .versions
                    .iter()
                    .map(|v| v.version.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
    let entry = registry_payload.version(&version).expect("selected above");
    let install_plan =
        install::plan_from_registry_entry(&reg_ref, &registry_payload, Some(&version), Some(host))?;
    let cache = PayloadCache::with_root(home);
    let cached = match cache
        .get(&install_plan.name, &install_plan.version)
        .map_err(install::map_resolve)?
    {
        Some(cached) => cached,
        None => {
            if tebako_resolve::cache::offline() {
                return Err(install::map_resolve(ResolveError::Offline {
                    what: format!("payload {}@{}", install_plan.name, install_plan.version),
                }));
            }
            let fetched = fetcher
                .fetch(&install_plan.reference)
                .map_err(install::map_resolve)?;
            let (cached, _status) = cache
                .install(
                    &install_plan.name,
                    &install_plan.version,
                    install_plan.expected_sha256.as_deref(),
                    || Ok(fetched),
                )
                .map_err(install::map_resolve)?;
            cached
        }
    };

    // The lock's pin (spec 23 §13.3): the single universal digest, or the
    // host triplet's row — press verifies the host's bytes.
    let pin = match &entry.platforms {
        RegistryPlatforms::Universal => tpkg::DigestPin::One(cached.sha256.clone()),
        RegistryPlatforms::PerTriplet(_) => tpkg::DigestPin::PerTriplet(BTreeMap::from([(
            host.release_asset_name().to_string(),
            cached.sha256.clone(),
        )])),
    };

    // The provider's embedded manifest (tier 1, authoritative) drives the
    // expose cross-check and the nested runtime pick; the identity
    // cross-check is the compose closure's (the registry must agree with
    // the bytes it names).
    let text = image_manifest::read_embedded_manifest(&cached.path)?.ok_or_else(|| {
        err(format!(
            "the spawned provider image {} carries no embedded manifest — the expose list cannot be verified",
            cached.path.display()
        ))
    })?;
    let provider_manifest = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
        err(format!(
            "the embedded manifest of {} does not parse: {e}",
            cached.path.display()
        ))
    })?;
    if provider_manifest.identity.name != install_plan.name
        || provider_manifest.identity.version != install_plan.version
    {
        return Err(err(format!(
            "the embedded manifest declares {} {} but the spawned edge resolved {} {} — the registry entry is inconsistent with the payload it names",
            provider_manifest.identity.name,
            provider_manifest.identity.version,
            install_plan.name,
            install_plan.version
        )));
    }

    // The expose cross-check (spec 32 §6/§7 — install's mirror-check
    // class, reading the provider's embedded manifest): each exposed name
    // must resolve to a declared entrypoint CARRYING a runtime
    // requirement (a runtime-less entry has no spawn form — spec 32
    // §0/§1). A toolkit executable and a runtime's own entrypoint are
    // runtime-less by construction.
    enum Found<'p> {
        Entrypoint(&'p tpkg::Entrypoint),
        RuntimeLess,
        Undeclared,
    }
    let find = |exposed: &str| -> Found {
        match &provider_manifest.provides {
            tpkg::Provides::App(app) => match app.entrypoints.iter().find(|e| e.name == exposed) {
                Some(ep) => Found::Entrypoint(ep),
                None => Found::Undeclared,
            },
            tpkg::Provides::Runtime(r) => match r.entrypoints.iter().find(|e| e.name == exposed) {
                Some(ep) => Found::Entrypoint(ep),
                None => Found::Undeclared,
            },
            tpkg::Provides::Toolkit(t) => {
                if t.executables.iter().any(|e| e.name == exposed) {
                    Found::RuntimeLess
                } else {
                    Found::Undeclared
                }
            }
            _ => Found::Undeclared,
        }
    };
    let declared: Vec<&str> = match &provider_manifest.provides {
        tpkg::Provides::App(app) => app.entrypoints.iter().map(|e| e.name.as_str()).collect(),
        tpkg::Provides::Runtime(r) => r.entrypoints.iter().map(|e| e.name.as_str()).collect(),
        tpkg::Provides::Toolkit(t) => t.executables.iter().map(|e| e.name.as_str()).collect(),
        _ => Vec::new(),
    };
    let mut exposed_eps: Vec<&tpkg::Entrypoint> = Vec::new();
    for exposed in expose {
        let ep = match find(exposed) {
            Found::Entrypoint(ep) => ep,
            Found::RuntimeLess => {
                return Err(err(format!(
                    "{consumer} requires executable {edge_name}: the provider's entrypoint \"{exposed}\" carries no runtime_requirement — a runtime-less entry has no spawn form, its surface is the exec tier (spec 32 §0/§1)"
                )));
            }
            Found::Undeclared => {
                return Err(err(format!(
                    "{consumer} requires executable {edge_name} and exposes \"{exposed}\" but the provider payload {provider} {version} declares no such entrypoint (declared: {}) — fix the expose list or the provider's manifest (spec 32 §7)",
                    declared.join(", ")
                )));
            }
        };
        if ep.runtime_requirement.is_none() {
            return Err(err(format!(
                "{consumer} requires executable {edge_name}: the provider's entrypoint \"{exposed}\" carries no runtime_requirement — a runtime-less entry has no spawn form, its surface is the exec tier (spec 32 §0/§1)"
            )));
        }
        exposed_eps.push(ep);
    }

    // One nested runtime per row: every exposed entrypoint must agree on
    // the engine, and on the implementation axis of its first
    // requirement entry (spec 28 §8's pre-stage convention).
    let first_reqs = exposed_eps[0]
        .runtime_requirement
        .as_ref()
        .expect("checked above");
    let engine = first_reqs.engine().to_string();
    let implementation = first_reqs.entries()[0].implementation.clone();
    for ep in &exposed_eps[1..] {
        let reqs = ep.runtime_requirement.as_ref().expect("checked above");
        if reqs.engine() != engine {
            return Err(err(format!(
                "the provider payload {provider} {version}'s exposed entrypoints disagree on the runtime engine ({engine} vs {}) — one spawned row carries one nested runtime (spec 32 §6); split the expose list across edges",
                reqs.engine()
            )));
        }
        if reqs.entries()[0].implementation != implementation {
            return Err(err(format!(
                "the provider payload {provider} {version}'s exposed entrypoints disagree on the runtime implementation — one spawned row carries one nested runtime (spec 32 §6); split the expose list across edges"
            )));
        }
    }

    // The nested row's constraint mirrors the provider's OWN
    // kind: language edge for the engine, verbatim (spec 32 §6 — the
    // validate cross-check asserts it against the provider's L1 edge).
    let language = provider_manifest
        .requires
        .iter()
        .find_map(|r| match r {
            tpkg::Requirement::Language {
                engine: e,
                constraint,
                ..
            } if *e == engine => Some(constraint),
            _ => None,
        })
        .ok_or_else(|| {
            err(format!(
                "the provider payload {provider} {version} declares no kind: language edge for engine '{engine}' — the spawned row's nested runtime constraint has no source (spec 32 §6)"
            ))
        })?;

    let rt = tebako_shim::runtime::resolve_runtime_edge(
        &engine,
        implementation.as_deref(),
        language,
        true,
        ctx,
    )
    .map_err(install::map_shim)?;
    for ep in &exposed_eps {
        let reqs = ep.runtime_requirement.as_ref().expect("checked above");
        if !reqs
            .entries()
            .iter()
            .any(|r| tpkg::runtime_store::entry_matches(&rt, r))
        {
            return Err(err(format!(
                "the nested runtime pick ({engine} {} / tebako {}) satisfies the provider's language edge but not the exposed entrypoint \"{}\"'s runtime_requirement ({reqs}) — pin a tighter kind: language edge in the provider's manifest",
                rt.lang_version, rt.tebako_version, ep.name
            )));
        }
    }

    gate_carried_runtime(
        &format!("the spawned payload provider '{provider}'s nested {engine} runtime"),
        preset,
    )?;

    // Slot order (the packed-mn oracle): the provider image first, then
    // the nested pair (exe, image, dll).
    let image_carry = preset.default_carry(false);
    let image_artifact = if image_carry {
        let slot = *next_slot;
        *next_slot += 1;
        plan.images
            .push((cached.path.clone(), String::new(), tpkg::TPKG_FORMAT_AUTO));
        tpkg::LockedSpawnedArtifact {
            slot: Some(slot),
            sha256: pin,
            install_as: None,
        }
    } else {
        // No current preset produces a shared payload row (the image
        // carries under both) — the shape stays expressible: the pin
        // stands and `source:` records the fetch coordinates.
        tpkg::LockedSpawnedArtifact {
            slot: None,
            sha256: pin,
            install_as: None,
        }
    };
    let nested = carried_runtime_row(
        &rt,
        &engine,
        implementation.as_deref(),
        language,
        Vec::new(),
        plan,
        next_slot,
    )?;
    Ok(tpkg::LockedSpawnedPayload {
        payload: provider.to_string(),
        constraint: constraint.clone(),
        expose: expose.to_vec(),
        version: install_plan.version.clone(),
        carry: image_carry,
        image: image_artifact,
        runtime: nested,
        source: if image_carry {
            None
        } else {
            Some(install_plan.reference.to_string())
        },
    })
}
