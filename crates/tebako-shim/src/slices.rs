//! Extension slices at dispatch (spec 03 §2.8's reverse edge + spec 07
//! §2 step 3a's scan, §4's pins, §7's named errors).
//!
//! When the resolved base payload declares `provides.extension_points`,
//! the dispatcher attaches EXTENSION SLICES — content-only payloads whose
//! top-level `augments:` edge names the base — as extra `--tebako-image`
//! triples appended AFTER the app's own mounts (spec 17's multi-mount),
//! each at `<point.mount>/<slice.name>`.
//!
//! Two sources; per slice NAME a pin wins over auto discovery (spec 07
//! §4):
//!
//! - PINS — `TEBAKO_<TOOL>_SLICES` (a comma list, PREPENDED), then the
//!   nearest project file's `slices:`, then the user config's
//!   `defaults.<tool>.slices`. A pin fetches on a store miss (parity with
//!   the runtime fetch: registry → verify → cache) and fails HARD — a
//!   pinned slice that fails a check is SliceIncompatible (exit 65)
//!   naming the slice, the pin's source, and the failed check.
//! - AUTO DISCOVERY — the store-local scan (`payloads/*/*.manifest.yaml`
//!   for an `augments[].payload` naming the base; never the network): the
//!   newest COMPATIBLE installed version per slice name attaches, and a
//!   candidate failing a check is a LOUD skip (journal
//!   `event=slice-skip`, a stderr note when no version of the name
//!   attaches), never an error. `auto_slices: false` kills this scan —
//!   never the pins.
//!
//! The named errors (spec 07 §7): SliceIncompatible (65) for a pinned
//! slice failing a check; NestedAugment (65) when the ROOT payload's own
//! manifest carries `augments:` (a slice dispatched as the root), or one
//! attached slice's mount nests under another's.

use std::collections::BTreeMap;
use std::path::Path;

use tebako_resolve::plan::{
    execute_plan, resolve_fetch_jobs, CommitReport, FetchItem, FetchPlan, StagedArtifact,
    FETCH_JOBS_ENV,
};
use tebako_resolve::registry::{PlatformSelection, SignaturePin};
use tebako_resolve::{FetchedPayload, Fetcher, HttpTransport, Reference, ResolveError, Transport};
use tebako_term::set::ProgressSet;

use crate::config;
use crate::dispatch::MountSpec;
use crate::resolve::{self, Resolution, SlicePin};
use crate::runtime::RuntimeResolution;
use crate::{
    fail, Ctx, ShimError, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST, EX_TEBAKO_SHA, EX_TEBAKO_SIGNATURE,
    EX_TEBAKO_TRUST, EX_TEBAKO_UNAVAILABLE,
};

/// One slice attaching to the dispatch: its image, its mount (below the
/// base's declared point), and its gem inventory (the overlap journal's
/// input).
struct Attach {
    name: String,
    version: String,
    image: std::path::PathBuf,
    mount: String,
    gems: Vec<tpkg::GemVersion>,
}

/// Why one candidate does not attach (spec 07 §2 step 3a's checks). The
/// token rides the journal's `reason=`; the detail is the human clause
/// (stderr for auto skips, the SliceIncompatible body for pins).
struct Reject {
    token: &'static str,
    detail: String,
}

impl Reject {
    fn new(token: &'static str, detail: String) -> Reject {
        Reject { token, detail }
    }
}

/// NestedAugment (spec 07 §7): the ROOT payload's own manifest carries
/// `augments:` — a slice dispatched as the root. Hard error on every
/// dispatch arm (exposed, zero-runtime, …): slices attach through the
/// base's dispatch, never head one. Hoisted out of [`attach`] so it fires
/// before the entrypoint lookup (a data-kind root has none and would
/// otherwise fail with the wrong named error).
pub fn refuse_root_augments(res: &Resolution) -> Result<(), ShimError> {
    if !res.manifest.payload_manifest().augments.is_empty() {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload \"{}\" {} declares `augments:` — an extension slice cannot be the ROOT of a dispatch (NestedAugment)\n  slices attach through the base's dispatch: run the base command, and the slice mounts below the base's declared extension point",
                res.payload_name, res.version
            ),
        );
    }
    Ok(())
}

/// The dispatcher's slice pass (spec 07 §2 step 3a): the mounts to append
/// after the app's own, in pin order then store-scan order.
pub fn attach(
    res: &Resolution,
    runtime: &RuntimeResolution,
    allow_download: bool,
    ctx: &Ctx,
) -> Result<Vec<MountSpec>, ShimError> {
    let base = res.manifest.payload_manifest();

    let points: &[tpkg::ExtensionPoint] = match &base.provides {
        tpkg::Provides::App(p) => &p.extension_points,
        _ => &[],
    };
    let base_gems: &[tpkg::GemVersion] = match &base.provides {
        tpkg::Provides::App(p) => p.gems.as_deref().unwrap_or(&[]),
        tpkg::Provides::Data(p) => p.gems.as_deref().unwrap_or(&[]),
        _ => &[],
    };
    let cfg = config::load_config(&ctx.home)?;
    let pins = resolve::slice_pins(&res.tool, ctx, &cfg)?;
    let auto = resolve::auto_slices_enabled(ctx, &cfg)?;

    // The §2 step-3a gate: the base declares extension points. Without
    // them the auto scan never runs; pins then fail their known-point
    // check one by one (SliceIncompatible), which only happens below.
    if points.is_empty() && pins.is_empty() {
        return Ok(Vec::new());
    }

    // Zero-runtime dispatch: no driver, no VFS — slices cannot attach.
    // Loud once (journal + stderr), never an error: the base runs exactly
    // as unextended.
    let RuntimeResolution::Ready(rt) = runtime else {
        crate::runtime::journal(
            &ctx.home,
            &format!(
                "event=slice-skip base={} reason=zero-runtime",
                res.payload_name
            ),
        );
        eprintln!(
            "tebako-shim: note: \"{}\" {} has extension surface (declared points or pins), but this dispatch has no runtime (no VFS) — extension slices do not attach",
            res.payload_name, res.version
        );
        return Ok(Vec::new());
    };

    let mut attached: Vec<Attach> = Vec::new();

    // Pins first — the pin contract is fail-hard (spec 07 §7).
    for pin in &pins {
        attached.push(attach_pin(res, points, rt, pin, allow_download, &cfg, ctx)?);
    }

    // Then auto discovery (the store scan), skipping names a pin owns.
    if auto && !points.is_empty() {
        let pinned: std::collections::BTreeSet<&str> =
            pins.iter().map(|p| p.name.as_str()).collect();
        auto_scan(res, points, rt, &pinned, &mut attached, ctx);
    }

    // The overlap journal (spec 07 §2 step 3a): base and slice both
    // carrying the same gem name is informational, never an error.
    for a in &attached {
        for gem in &a.gems {
            if base_gems.iter().any(|g| g.name == gem.name) {
                crate::runtime::journal(
                    &ctx.home,
                    &format!(
                        "event=slice-overlap slice={}@{} gem={}",
                        a.name, a.version, gem.name
                    ),
                );
            }
        }
    }

    // NestedAugment (spec 07 §7): no attached slice's mount nests under
    // another's. (Nesting under the APP's mounts — the app's `/` — is
    // the multi-mount design, not this error.)
    for (i, a) in attached.iter().enumerate() {
        for b in attached.iter().skip(i + 1) {
            let nested = format!("{}/", a.mount);
            let nestee = format!("{}/", b.mount);
            if b.mount.starts_with(&nested) || a.mount.starts_with(&nestee) {
                return fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "extension slice \"{}\" mounts at {}, which nests under slice \"{}\" at {} (NestedAugment) — slices mount side by side below the base's points; re-mount one at its own point",
                        b.name, b.mount, a.name, a.mount
                    ),
                );
            }
        }
    }

    for a in &attached {
        crate::runtime::journal(
            &ctx.home,
            &format!(
                "event=slice-augment slice={}@{} base={} mount={}",
                a.name, a.version, res.payload_name, a.mount
            ),
        );
    }

    Ok(attached
        .into_iter()
        .map(|a| MountSpec {
            image: a.image,
            slot: 0,
            mount: a.mount,
        })
        .collect())
}

/// The §2 step-3a checks against ONE candidate slice manifest, in check
/// order: an `augments:` edge naming the base → the base DECLARES the
/// named point → the edge's constraint vs the base's RESOLVED version →
/// the slice's language edges vs the RESOLVED runtime. `Ok` carries the
/// point (the mount root).
fn check_candidate(
    base_name: &str,
    base_version: &str,
    points: &[tpkg::ExtensionPoint],
    rt: &tpkg::runtime_store::CachedRuntime,
    slice: &tpkg::PayloadManifest,
) -> Result<tpkg::ExtensionPoint, Reject> {
    let Some(edge) = slice.augments.iter().find(|e| e.payload == base_name) else {
        return Err(Reject::new(
            "no-edge",
            format!("declares no `augments:` edge naming \"{base_name}\""),
        ));
    };
    let Some(point) = points.iter().find(|p| p.name == edge.extension_point) else {
        return Err(Reject::new(
            "unknown-point",
            format!(
                "names extension point \"{}\", which \"{base_name}\" does not declare",
                edge.extension_point
            ),
        ));
    };
    if !tpkg::versions::from_validated(&edge.constraint).matches(base_version) {
        return Err(Reject::new(
            "base-version",
            format!(
                "constraint \"{}\" does not match the resolved {base_name} {base_version}",
                edge.constraint.as_str()
            ),
        ));
    }
    for req in &slice.requires {
        if let tpkg::Requirement::Language {
            engine, constraint, ..
        } = req
        {
            if engine != &rt.engine {
                return Err(Reject::new(
                    "runtime",
                    format!(
                        "language edge engine \"{engine}\" ≠ the resolved runtime's \"{}\"",
                        rt.engine
                    ),
                ));
            }
            // The language edge vs the RESOLVED runtime (spec 07 §2 step
            // 3a) — entry_matches owns the version-line rule (and the abi
            // line, when both sides carry one).
            let check = tpkg::RuntimeRequirement {
                engine: engine.clone(),
                constraint: constraint.clone(),
                implementation: None,
                abi: None,
            };
            if !tpkg::runtime_store::entry_matches(rt, &check) {
                return Err(Reject::new(
                    "runtime",
                    format!(
                        "language edge \"{engine} {}\" is not satisfied by the resolved runtime {} {}",
                        constraint.as_str(),
                        rt.engine,
                        rt.lang_version
                    ),
                ));
            }
        }
    }
    Ok(point.clone())
}

/// A slice manifest's gem inventory, whichever provides kind carries it.
fn slice_gems(m: &tpkg::PayloadManifest) -> Vec<tpkg::GemVersion> {
    match &m.provides {
        tpkg::Provides::App(p) => p.gems.clone().unwrap_or_default(),
        tpkg::Provides::Data(p) => p.gems.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// The mount one attached slice takes: BELOW the base's declared point
/// (spec 03 §2.8 — `<mount>/<slice.name>`).
fn slice_mount(point: &tpkg::ExtensionPoint, slice_name: &str) -> String {
    format!("{}/{}", point.mount.trim_end_matches('/'), slice_name)
}

/// The pin path (spec 07 §4/§7): a pin names `<slice>@<version>`
/// EXACTLY — a different installed version never substitutes. Store miss
/// fetches (parity with the runtime fetch) when downloads are allowed;
/// every check failure is SliceIncompatible naming the slice, the pin's
/// source, and the failed check.
fn attach_pin(
    res: &Resolution,
    points: &[tpkg::ExtensionPoint],
    rt: &tpkg::runtime_store::CachedRuntime,
    pin: &SlicePin,
    allow_download: bool,
    cfg: &config::UserConfig,
    ctx: &Ctx,
) -> Result<Attach, ShimError> {
    let incompatible = |reason: &Reject| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!(
                "extension slice {}@{} (pinned in {}) is incompatible with {} {}: {} (SliceIncompatible)",
                pin.name, pin.version, pin.source, res.payload_name, res.version, reason.detail
            ),
        )
    };

    let cached = match tpkg::payload_store::get(&ctx.home, &pin.name, &pin.version) {
        Ok(Some(cached)) => cached,
        Ok(None) => {
            // TEBAKO_OFFLINE=1 = cache-or-named-error (spec 05's shape),
            // regardless of the caller's download allowance.
            if !allow_download || crate::runtime::offline_mode(ctx) {
                return fail(
                    EX_TEBAKO_UNAVAILABLE,
                    format!(
                        "pinned slice {}@{} (from {}) is not cached and this dispatch may not download (TEBAKO_OFFLINE)\n  install it first: `tebako install {}@{}`",
                        pin.name, pin.version, pin.source, pin.name, pin.version
                    ),
                );
            }
            fetch_slice(&ctx.home, cfg, pin, ctx)?;
            match tpkg::payload_store::get(&ctx.home, &pin.name, &pin.version) {
                Ok(Some(cached)) => cached,
                Ok(None) => {
                    return fail(
                        EX_TEBAKO_UNAVAILABLE,
                        format!(
                            "the fetch of pinned slice {}@{} did not produce a cache record — nothing was installed",
                            pin.name, pin.version
                        ),
                    )
                }
                Err(e) => return Err(mirror_error(&ctx.home, pin, &e)),
            }
        }
        Err(e) => return Err(mirror_error(&ctx.home, pin, &e)),
    };

    match check_candidate(
        &res.payload_name,
        &res.version,
        points,
        rt,
        &cached.manifest,
    ) {
        Ok(point) => Ok(Attach {
            name: pin.name.clone(),
            version: pin.version.clone(),
            image: cached.image,
            mount: slice_mount(&point, &pin.name),
            gems: slice_gems(&cached.manifest),
        }),
        Err(reason) => Err(incompatible(&reason)),
    }
}

/// The record's manifest mirror is unreadable (spec 07 §7's pin path):
/// missing → the record came from a fetch path that writes no mirror
/// (exit 69, the remedy named); damaged → the record fails closed (65).
fn mirror_error(home: &Path, pin: &SlicePin, detail: &str) -> ShimError {
    let mirror = tpkg::payload_store::manifest_mirror_path(home, &pin.name, &pin.version);
    if mirror.is_file() {
        ShimError::new(EX_TEBAKO_MANIFEST, detail.to_string())
    } else {
        ShimError::new(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "pinned slice {}@{} (from {}) is cached but has no manifest mirror ({}) — the dispatch-time checks need it\n  repair the record with `tebako install {}@{}` (the install path writes the mirror)",
                pin.name,
                pin.version,
                pin.source,
                mirror.display(),
                pin.name,
                pin.version
            ),
        )
    }
}

/// The §2 step-3a store scan: every installed payload version whose
/// manifest mirror carries an `augments:` edge naming the base is a
/// candidate; per slice NAME the newest compatible version attaches. All
/// failures are SOFT (journal `event=slice-skip`; a stderr note when no
/// version of a name attaches) — auto discovery never fails a dispatch
/// (spec 07 §7).
fn auto_scan(
    res: &Resolution,
    points: &[tpkg::ExtensionPoint],
    rt: &tpkg::runtime_store::CachedRuntime,
    pinned: &std::collections::BTreeSet<&str>,
    attached: &mut Vec<Attach>,
    ctx: &Ctx,
) {
    let payloads_dir = ctx.home.join("payloads");
    let Ok(rd) = std::fs::read_dir(&payloads_dir) else {
        return; // no store: no candidates
    };
    // name → the newest attaching candidate (and whether any candidate
    // failed, for the stderr note).
    let mut winners: BTreeMap<String, (String, tpkg::PayloadManifest, tpkg::ExtensionPoint)> =
        BTreeMap::new();
    let mut failed: BTreeMap<String, (String, String)> = BTreeMap::new(); // name → (version, detail)
    for entry in rd.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if pinned.contains(name.as_str()) {
            continue; // a pin owns this name (spec 07 §4)
        }
        let versions = match tpkg::payload_store::installed_versions(&ctx.home, &name) {
            Ok(v) => v,
            Err(e) => {
                crate::runtime::journal(
                    &ctx.home,
                    &format!("event=slice-skip slice={name} reason=store-unreadable detail={e}"),
                );
                continue;
            }
        };
        for version in versions {
            // A partial record (image without trust anchor) is invisible
            // (spec 05 §3).
            if !tpkg::payload_store::sha_marker_path(&ctx.home, &name, &version).is_file() {
                continue;
            }
            let mirror = tpkg::payload_store::manifest_mirror_path(&ctx.home, &name, &version);
            // No mirror: nothing declares this record a slice — not a
            // candidate, no noise (fetch-only records carry no mirror).
            let Ok(text) = std::fs::read_to_string(&mirror) else {
                continue;
            };
            let slice = match tpkg::PayloadManifest::from_yaml(&text) {
                Ok(m) => m,
                Err(e) => {
                    crate::runtime::journal(
                        &ctx.home,
                        &format!(
                            "event=slice-skip slice={name}@{version} reason=corrupt-mirror detail={e}"
                        ),
                    );
                    continue;
                }
            };
            if !slice.augments.iter().any(|e| e.payload == res.payload_name) {
                continue; // not a slice of this base — the common case
            }
            match check_candidate(&res.payload_name, &res.version, points, rt, &slice) {
                Ok(point) => {
                    let replace = match winners.get(&name) {
                        None => true,
                        Some((have, _, _)) => {
                            tpkg::versions::compare(&version, have) == std::cmp::Ordering::Greater
                        }
                    };
                    if replace {
                        winners.insert(name.clone(), (version, slice, point));
                    }
                }
                Err(reject) => {
                    crate::runtime::journal(
                        &ctx.home,
                        &format!(
                            "event=slice-skip slice={name}@{version} reason={} detail={}",
                            reject.token, reject.detail
                        ),
                    );
                    failed.insert(name.clone(), (version, reject.detail));
                }
            }
        }
    }
    for (name, (_, detail)) in &failed {
        if !winners.contains_key(name) {
            eprintln!(
                "tebako-shim: note: extension slice \"{name}\" does not attach to {} {} — {detail}",
                res.payload_name, res.version
            );
        }
    }
    for (name, (version, slice, point)) in winners {
        attached.push(Attach {
            mount: slice_mount(&point, &name),
            image: tpkg::payload_store::image_path(&ctx.home, &name, &version),
            gems: slice_gems(&slice),
            name,
            version,
        });
    }
}

/// The pin's fetch-on-miss (spec 07 §4 — parity with the runtime fetch):
/// the pinned `<slice>@<version>` resolves through the registered
/// registries (first hit wins, the ambiguity named), fetches, verifies
/// (the registry's sha256 pin through the cache install + the optional
/// OpenPGP signature BEFORE the bytes land), and installs. Offline was
/// refused by the caller; a registry that does not list the pin is the
/// named not-listed error.
fn fetch_slice(
    home: &Path,
    cfg: &config::UserConfig,
    pin: &SlicePin,
    ctx: &Ctx,
) -> Result<(), ShimError> {
    let mut hits = Vec::new();
    for reg_ref in &cfg.registries {
        let registry = crate::regcache::registry_for(home, reg_ref, ctx)?;
        if let Some(payload) = registry.payload(&pin.name) {
            if let Some(entry) = payload.versions.iter().find(|v| v.version == pin.version) {
                hits.push((reg_ref.clone(), entry.clone()));
            }
        }
    }
    let (_reg_ref, entry) = match hits.len() {
        0 => {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "pinned slice {}@{} (from {}): no registered registry lists it\n  add the registry (`tebako add-registry <ref>`), or install the slice explicitly: `tebako install {}@{}`",
                    pin.name, pin.version, pin.source, pin.name, pin.version
                ),
            )
        }
        1 => hits.remove(0),
        n => {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "pinned slice {}@{} (from {}) is listed by {n} registered registries (AmbiguousRegistries):\n{}\n  install it explicitly first: `tebako install {}@{}`",
                    pin.name,
                    pin.version,
                    pin.source,
                    hits.iter()
                        .map(|(r, _)| format!("    - {r}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    pin.name,
                    pin.version
                ),
            )
        }
    };
    // The SELECTED row's `status: withdrawn` is a named refusal (spec 04
    // §2), never a silent fallback.
    if entry.is_withdrawn() {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "pinned slice {}@{} is withdrawn in its registry — the release was yanked; pick another version",
                pin.name, pin.version
            ),
        );
    }

    let mut reference = Reference::parse(&entry.release.r#ref).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("registry release ref for {}@{}: {e}", pin.name, pin.version),
        )
    })?;
    let expected_sha256 = match entry.select(tpkg::Platform::host()) {
        Some(PlatformSelection::Universal) => None,
        Some(PlatformSelection::Selected { artifact, sha256 }) => {
            if let Reference::Service { artifact: slot, .. } = &mut reference {
                *slot = Some(artifact.to_string());
            }
            Some(sha256.to_string())
        }
        None => {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "slice {}@{} is not published for the host triplet {}",
                    pin.name,
                    pin.version,
                    tpkg::Platform::host()
                ),
            )
        }
    };

    let fetcher = Fetcher::new();
    // The fetch pipeline (spec 05 §6): the slice streams to the store's
    // tmp with its sha256 computed inline; the commit verifies the
    // declared signature against the staged bytes FIRST (spec 09 §4's
    // order) and lands through the cache's staged install (the pin
    // re-checks there).
    let cache = tebako_resolve::PayloadCache::with_root(home);
    let sink = SliceSink::default();
    let item = FetchItem {
        display: format!("{}@{}", pin.name, pin.version),
        reference: reference.clone(),
        sha256_pin: None,
        size_hint: None,
        tmp_dir: home.join("tmp"),
        commit: Box::new(|staged: &StagedArtifact| {
            if let Err(e) = verify_slice_signature_staged(
                ctx,
                &fetcher,
                staged,
                &reference,
                entry.signature.as_ref(),
            ) {
                return Err(sink.fail(e));
            }
            match cache.install_staged(
                &pin.name,
                &pin.version,
                expected_sha256.as_deref(),
                staged.tmp,
                staged.sha256,
                staged.origin,
            ) {
                Ok((entry, _)) => {
                    sink.add_entry(entry);
                    Ok(CommitReport { line: None })
                }
                Err(e) => Err(sink.fail(map_fetch(e))),
            }
        }),
    };
    let jobs = resolve_fetch_jobs(
        ctx.env_get(FETCH_JOBS_ENV).map(str::to_string),
        cfg.fetch_jobs,
    )
    .map_err(|e| ShimError::new(EX_TEBAKO_MANIFEST, e.to_string()))?;
    let transport = HttpTransport;
    let progress = ProgressSet::stderr();
    let result = execute_plan(
        &transport,
        FetchPlan::new(format!("slice {}@{}", pin.name, pin.version), vec![item]),
        jobs,
        Some(&progress),
    );
    if let Some(e) = sink.take_error() {
        return Err(e);
    }
    result.map_err(map_fetch)?;
    let entry = sink.take_entry().ok_or_else(|| {
        ShimError::new(EX_TEBAKO_IO, "the slice fetch plan ended without a commit")
    })?;
    crate::runtime::journal(
        home,
        &format!(
            "event=payload-installed name={} version={} sha256={} origin={}",
            pin.name,
            pin.version,
            entry.sha256,
            entry.origin.as_deref().unwrap_or("")
        ),
    );
    Ok(())
}

/// The slice plan commit's report channel (the closure runs on the
/// pipeline's worker thread): the installed entry and the FIRST precise
/// named error — the pipeline's own [`ResolveError`] is only the cancel
/// marker; this error's exit code rides back verbatim.
#[derive(Default)]
struct SliceSink {
    entry: std::sync::Mutex<Option<tebako_resolve::CacheEntry>>,
    error: std::sync::Mutex<Option<ShimError>>,
}

impl SliceSink {
    fn add_entry(&self, entry: tebako_resolve::CacheEntry) {
        *self.entry.lock().unwrap_or_else(|e| e.into_inner()) = Some(entry);
    }

    fn fail(&self, e: ShimError) -> ResolveError {
        let mut slot = self.error.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = Some(e);
        }
        drop(slot);
        ResolveError::Commit {
            reason: "the slice's install commit failed".to_string(),
        }
    }

    fn take_error(&self) -> Option<ShimError> {
        self.error.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    fn take_entry(&self) -> Option<tebako_resolve::CacheEntry> {
        self.entry.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

/// [`verify_slice_signature`] over the pipeline's staged tmp file
/// (spec 05 §6): the bytes streamed to disk with the sha256 computed
/// inline; the signature pass reads the staged file once — never after
/// the rename. An unsigned entry reads nothing.
fn verify_slice_signature_staged<T: Transport>(
    ctx: &Ctx,
    fetcher: &Fetcher<T>,
    staged: &StagedArtifact,
    reference: &Reference,
    signature: Option<&SignaturePin>,
) -> Result<(), ShimError> {
    let bytes = if signature.is_some() {
        std::fs::read(staged.tmp).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!(
                    "cannot read the staged {} for the signature check: {e}",
                    staged.tmp.display()
                ),
            )
        })?
    } else {
        Vec::new()
    };
    let fetched = FetchedPayload {
        bytes,
        origin: staged.origin.to_string(),
        sha256: staged.sha256.to_string(),
    };
    verify_slice_signature(ctx, fetcher, &fetched, reference, signature)
}

/// The fetch-time signature rule for a pinned slice — the install path's
/// consumer policy mirrored (spec 09 §4): an entry with no signature is
/// the v1-legacy rule (loud warn + audit line; `TEBAKO_REQUIRE_SIGNED=1`
/// hard-fails, 71); a signed artifact verifies strict (Invalid → 71,
/// Untrusted → 72, a signer-keyid change → 72).
fn verify_slice_signature<T: Transport>(
    ctx: &Ctx,
    fetcher: &Fetcher<T>,
    fetched: &FetchedPayload,
    reference: &Reference,
    signature: Option<&SignaturePin>,
) -> Result<(), ShimError> {
    let Some(sig) = signature else {
        if crate::runtime::require_signed(ctx) {
            return fail(
                EX_TEBAKO_SIGNATURE,
                format!(
                    "{} is unsigned and TEBAKO_REQUIRE_SIGNED=1 is set — refusing to install",
                    fetched.origin
                ),
            );
        }
        eprintln!(
            "tebako-shim: WARNING: {} is unsigned\n  — accepted for compatibility (the registry entry carries no signature); ask the publisher to sign the release",
            fetched.origin
        );
        crate::runtime::journal(
            &ctx.home,
            &format!("event=legacy-unsigned-accepted origin={}", fetched.origin),
        );
        return Ok(());
    };

    let asc_ref = signature_reference(sig, reference)?;
    let asc = fetcher.fetch(&asc_ref).map_err(map_fetch)?;
    let mut keyring = shim_verification_keyring(ctx)?;
    let mut retrieved = false;
    let issuer = loop {
        let outcome = tebako_signer::verify_detached_full(&keyring, &fetched.bytes, &asc.bytes)
            .map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_SIGNATURE,
                    format!("cannot verify the signature on {}: {e}", fetched.origin),
                )
            })?;
        match outcome {
            tebako_signer::VerifyOutcome::Trusted(keyid) => break keyid,
            tebako_signer::VerifyOutcome::Untrusted(keyid) if !retrieved => {
                retrieved = true;
                // spec 09 §10: a signer ON THE TAMATEBAKO ROOT CHAIN is
                // retrieved from the trust-anchor channel and admitted
                // through the chain (never TOFU) — then the verification
                // re-runs against the widened keyring.
                match retrieve_slice_signer(ctx, sig)? {
                    Some(retrieval) => {
                        crate::runtime::journal(
                            &ctx.home,
                            &format!(
                                "event=key-retrieval keyid={} fingerprint={} source={} basis={}",
                                retrieval.keyid,
                                retrieval.fingerprint,
                                retrieval.source_url,
                                retrieval.basis.journal_label()
                            ),
                        );
                        keyring = shim_verification_keyring(ctx)?;
                    }
                    None => {
                        return fail(
                            EX_TEBAKO_TRUST,
                            format!(
                                "{} is signed by {keyid}, which is not in the trusted keyring — register the publisher's key (~/.tebako/keyring/trusted.pgp), then retry; nothing was cached",
                                fetched.origin
                            ),
                        );
                    }
                }
            }
            tebako_signer::VerifyOutcome::Untrusted(keyid) => {
                return fail(
                    EX_TEBAKO_TRUST,
                    format!(
                        "{} is signed by {keyid}, which is not in the trusted keyring — register the publisher's key (~/.tebako/keyring/trusted.pgp), then retry; nothing was cached",
                        fetched.origin
                    ),
                );
            }
            tebako_signer::VerifyOutcome::Invalid(keyid) => {
                return fail(
                    EX_TEBAKO_SIGNATURE,
                    format!(
                        "signature verification failed for {} (signer {}) — the payload or its signature is corrupt; nothing was cached",
                        fetched.origin,
                        keyid.unwrap_or_else(|| "unknown".to_string())
                    ),
                );
            }
        }
    };
    // The registry pin names the signing key's PRIMARY keyid (spec 09
    // §9): a signature issuing from a signing subkey resolves to its
    // primary through the keyring before the compare.
    let issuer = issuer.to_ascii_lowercase();
    let pin = sig.keyid.to_ascii_lowercase();
    let primary = tebako_signer::primary_keyid_of(&keyring, &issuer)
        .map_err(|e| ShimError::new(EX_TEBAKO_SIGNATURE, e.to_string()))?;
    if issuer != pin && primary.as_deref() != Some(pin.as_str()) {
        let primary_note = primary
            .as_deref()
            .map_or_else(String::new, |p| format!(" (primary {p})"));
        return fail(
            EX_TEBAKO_TRUST,
            format!(
                "{} is signed by {issuer}{primary_note} but the registry pins {pin} — the signer key changed (SignerKeyChanged); refusing to install; nothing was cached",
                fetched.origin
            ),
        );
    }
    crate::runtime::journal(
        &ctx.home,
        &format!(
            "event=payload-signature-trusted origin={} signer={issuer}",
            fetched.origin
        ),
    );
    Ok(())
}

/// The zero-interaction keyring (spec 09 §9): the user's trusted
/// keyring + the embedded tamatebako root + the TEBAKO_TRUSTED_ROOT
/// dev override's bundled key.
fn shim_verification_keyring(ctx: &Ctx) -> Result<Vec<u8>, ShimError> {
    let mut keyring = tebako_signer::trusted_keyring_bytes(&ctx.home).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot read the trusted keyring: {e}"),
        )
    })?;
    let root =
        tebako_signer::dearmor_bytes(tebako_signer::ROOT_PUBLIC_KEY.as_bytes()).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("the embedded root key does not dearmor: {e}"),
            )
        })?;
    keyring.extend_from_slice(&root);
    if let Some(extra) = tebako_signer::trusted_root_override_key(
        ctx.env_get("TEBAKO_TRUSTED_ROOT").map(str::to_string),
    ) {
        keyring.extend_from_slice(&extra);
    }
    Ok(keyring)
}

/// The spec 09 §10 ceremony for a slice's pinned signer: fetch the key
/// from the trust-anchor channel, admit it ONLY through the tamatebako
/// root chain (never TOFU), and register it into the trusted keyring.
/// `Ok(None)` = offline (the pre-ceremony untrusted outcome stands);
/// the named `KeyRetrieval` failure journals
/// `event=key-retrieval-failed` and exits 72 (the trust class).
fn retrieve_slice_signer(
    ctx: &Ctx,
    sig: &SignaturePin,
) -> Result<Option<tebako_signer::KeyRetrieval>, ShimError> {
    let offline = crate::runtime::offline_mode(ctx);
    let fetch = |url: &str| tebako_http::get(url).map_err(|e| e.to_string());
    match tebako_signer::retrieve_signer_key(&ctx.home, &sig.keyid, offline, &fetch) {
        Ok(retrieval) => Ok(retrieval),
        Err(e) => {
            crate::runtime::journal(
                &ctx.home,
                &format!("event=key-retrieval-failed keyid={} reason={e}", sig.keyid),
            );
            Err(ShimError::new(EX_TEBAKO_TRUST, e.to_string()))
        }
    }
}

/// The `.asc` of a signature pin: a full reference, or an asset name
/// within the same release (`<artifact>.asc` by convention — the asc
/// follows the SELECTED artifact, the install path's rule).
fn signature_reference(sig: &SignaturePin, release: &Reference) -> Result<Reference, ShimError> {
    if looks_like_reference(&sig.asc) {
        return Reference::parse(&sig.asc).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!("signature.asc does not parse: {e}"),
            )
        });
    }
    let mut reference = release.clone();
    match &mut reference {
        Reference::Service {
            artifact, sha256, ..
        } => {
            // The payload's own pin must not leak onto the signature asset.
            *sha256 = None;
            *artifact = Some(match artifact {
                Some(selected) => format!("{selected}.asc"),
                None => sig.asc.clone(),
            });
            Ok(reference)
        }
        _ => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "signature.asc '{}' is an asset name but the release is not a service release — name a full reference instead",
                sig.asc
            ),
        ),
    }
}

/// The reference-shape test (`tfs:` scheme or any `://` URL), the install
/// path's rule.
fn looks_like_reference(target: &str) -> bool {
    target.contains("://") || target.starts_with("tfs:")
}

/// A fetch/install failure of a pinned slice: the shim's code table (the
/// regcache mapping's twin).
fn map_fetch(e: ResolveError) -> ShimError {
    let code = match &e {
        ResolveError::Sha256Mismatch { .. } => EX_TEBAKO_SHA,
        ResolveError::LockTimeout { .. } | ResolveError::CacheIo { .. } => EX_TEBAKO_IO,
        ResolveError::Registry(_) | ResolveError::Reference(_) => EX_TEBAKO_MANIFEST,
        _ => EX_TEBAKO_UNAVAILABLE,
    };
    ShimError::new(code, format!("cannot fetch the pinned slice: {e}"))
}
