//! Runtime resolution (spec 07 §2.2 + spec 05 §5):
//!
//! the entrypoint's `runtime_requirement` → newest COMPATIBLE runtime
//! already cached (no download) → else download the newest compatible →
//! verify → cache. Zero-runtime entrypoints (no `runtime_requirement`)
//! skip this module entirely. The newest compatible to DOWNLOAD is the
//! release index's pick, not only the config pin's (spec 13 §2a): the
//! newest interpreter version satisfying the constraint AND released for
//! this platform — a readable index with nothing satisfiable is the
//! named platform-availability error (never a bare asset 404); an
//! unreadable or availability-keyless index leaves the pin the target.
//!
//! The download path mirrors tebako-bootstrap's semantics — per-entry
//! flock (120 s), tmp + rename install, `sha256`/`origin` trust markers,
//! read-only image, the spec 18 C2 release-card gate (pre-download
//! contract refusal; tebako-resolve::contract owns the reader),
//! spec 05 §2's shard-first index order (the per-package
//! `<stem>.manifest.json` shard, then the manifest.json monolith, then
//! the SHA256SUMS line index — the fallbacks stay forever, invariant 7),
//! `TEBAKO_RUNTIME_MIRROR` / `TEBAKO_OFFLINE` — reimplemented
//! here rather than linked from the bootstrap crate.
//!
//! Two extensions over the bootstrap's path live here:
//!
//! - **The per-engine download source chain** (spec 05 §2, tebako#567):
//!   config `runtimes: {<engine>: {source:}}` → `TEBAKO_RUNTIME_MIRROR` →
//!   the registered registries' `kind: runtime` entries (the release.ref
//!   derives the base AND the tag) → the product default (ruby only —
//!   a non-ruby engine no channel answers is the named error enumerating
//!   the channels).
//! - **Fetch-time OpenPGP verification** (spec 09 §4, roadmap 80's G1):
//!   the consumed index form's detached `.asc` verifies BEFORE its
//!   digests are trusted; every artifact whose entry DECLARES
//!   `signature:` (spec 13 §2a) verifies against it. Strict: invalid →
//!   71, untrusted signer / pin mismatch → 72; an unsigned release is
//!   first-class (loud + journaled) except under
//!   `TEBAKO_REQUIRE_SIGNED=1` (71).

use std::io::Read;
use std::path::Path;

use tpkg::{RuntimeRequirement, RuntimeRequirements};

use crate::config::{self, RuntimePref};
use crate::versions;
use crate::{
    fail, Ctx, ShimError, EX_TEBAKO_CONTRACT, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST, EX_TEBAKO_SHA,
    EX_TEBAKO_SIGNATURE, EX_TEBAKO_TRUST, EX_TEBAKO_UNAVAILABLE,
};

const DEFAULT_RELEASES_BASE: &str =
    "https://github.com/tamatebako/tebako-runtime-ruby/releases/download";
const LOCK_TIMEOUT_MS: u64 = 120_000;
const LOCK_POLL_MS: u64 = 200;

// The store grammar, the cache scan, and the version machinery moved to
// tpkg (spec 00 §10 — one owner, every consumer flows): the shim's
// resolution/download layers below build on these re-exports.
use tpkg::runtime_store::{
    entry_asset_names, entry_dll_from_index, entry_filename, entry_matches, entry_meta,
    entry_signature, newest_compatible_any, release_index_entry, EntrySignature,
};
pub use tpkg::runtime_store::{
    exe_suffix, newest_compatible, platform_string, scan_all_cached, scan_cached, CachedRuntime,
};

// ---------------------------------------------------------------------
// resolution
// ---------------------------------------------------------------------

#[derive(Debug)]
pub enum RuntimeResolution {
    /// The entrypoint declares no `runtime_requirement` (native /
    /// self-contained): zero runtime payloads mounted.
    Zero,
    // Boxed: `CachedRuntime` is 216 bytes on Windows (Wtf8Buf's extra
    // flag word inflates every PathBuf past clippy's
    // large_enum_variant threshold against the data-less `Zero`).
    Ready(Box<CachedRuntime>),
}

pub fn resolve_runtime(
    requirement: Option<&RuntimeRequirements>,
    allow_download: bool,
    ctx: &Ctx,
) -> Result<RuntimeResolution, ShimError> {
    let Some(reqs) = requirement else {
        return Ok(RuntimeResolution::Zero);
    };
    // The constraints were validated at manifest parse (tpkg::Constraint) —
    // the dispatcher only evaluates them against cached/offered versions.
    // spec 28 §8: a LIST requirement is `any_of` — the newest cache entry
    // matching ANY entry wins (an implementation-narrowed entry matches
    // the runtime's own version line; a language-level entry matches the
    // runtime's `language_version`, falling back to its version on
    // pre-field shards — see tpkg::runtime_store::entry_matches).
    let cached = scan_cached(&ctx.home, reqs.engine());
    if let Some(hit) = newest_compatible_any(&cached, reqs) {
        return Ok(RuntimeResolution::Ready(Box::new(hit)));
    }
    // The abi line (spec 05 §5): a native-extension payload matches only
    // runtimes carrying ITS platform string. A runtime whose release
    // predates the field (abi: None) stays eligible — the compat window,
    // never a match failure of its own. (The validator forbids `abi` on
    // the list form, so at most one entry carries one.)
    let want_abi = reqs.entries().iter().find_map(|r| r.abi.as_deref());
    let abi_note = match want_abi {
        Some(want)
            if cached
                .iter()
                .any(|c| c.abi.as_ref().is_some_and(|have| have != want)) =>
        {
            format!(
                "; the cached abi line(s) ({}) do not match the payload's \"{want}\"",
                cached
                    .iter()
                    .filter_map(|c| c.abi.as_deref())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        _ => String::new(),
    };
    let described = reqs.to_string();

    // No compatible cached runtime. The download target: the release
    // index's pick on the configured preference's line (config.yaml
    // `runtimes:`, spec 07 §4 "runtime preferences") — or, with no
    // preference configured, on the product default line
    // (tebako-resolve::DEFAULT_TEBAKO_VERSION).
    let cfg = config::load_config(&ctx.home)?;
    let pref = cfg.runtimes.get(reqs.engine());
    let cached_note = if cached.is_empty() {
        format!("no cached {} runtimes for this platform", reqs.engine())
    } else {
        format!(
            "cached {} runtimes ({}) do not satisfy \"{}\"{}{}",
            reqs.engine(),
            cached
                .iter()
                .map(|c| c.lang_version.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            described,
            if reqs
                .entries()
                .iter()
                .any(|r| r.constraint.as_str().contains("~>"))
            {
                " — a native-extension payload locks to its ABI line; a newer line needs a new payload build"
            } else {
                ""
            },
            abi_note
        )
    };
    // The download line: the config pin's when one is configured, else
    // the product default (tebako-resolve::DEFAULT_TEBAKO_VERSION — the
    // single owner). A bare `tebako install` never strands on a missing
    // pin: the release index on the default line picks the newest
    // interpreter satisfying the constraint for this platform.
    let (pref_owned, prefless) = match pref {
        Some(p) => {
            let mut p = p.clone();
            if p.tebako.is_empty() {
                // `tebako-shim use --runtime <engine>@<langver>` without
                // `:<tebako>` pins the language version and follows the
                // product default line (config.rs RuntimePref::tebako).
                p.tebako = tebako_resolve::DEFAULT_TEBAKO_VERSION.to_string();
            }
            (p, false)
        }
        None => (
            RuntimePref {
                version: String::new(),
                tebako: tebako_resolve::DEFAULT_TEBAKO_VERSION.to_string(),
                source: None,
            },
            true,
        ),
    };
    let pref = &pref_owned;
    // The pin names the runtime's OWN version line (it names the release
    // asset): satisfied when ANY entry's constraint matches it (spec 28
    // §8 — a payload admitting an implementation carries that
    // implementation's own-line entry alongside any language-level one).
    if !prefless
        && !reqs
            .entries()
            .iter()
            .any(|r| versions::from_validated(&r.constraint).matches(&pref.version))
    {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "runtime preference {}@{} does not satisfy \"{}\": {cached_note}\n  re-pin the preference (`tebako use --runtime {}@<version>`) or rebuild the payload against a newer ABI line",
                reqs.engine(),
                pref.version,
                described,
                reqs.engine()
            ),
        );
    }
    if !allow_download {
        if prefless {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "no compatible runtime for {} \"{}\": {cached_note}\n  and no runtime preference is configured — set `runtimes: {{{}: {{version: …, tebako: …}}}}` in ~/.tebako/config.yaml, or pre-seed the cache",
                    reqs.engine(),
                    described,
                    reqs.engine()
                ),
            );
        }
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "no compatible cached runtime for {} \"{}\" (would download {}@{}): {cached_note}",
                reqs.engine(),
                described,
                reqs.engine(),
                pref.version
            ),
        );
    }
    // The download target is the release index's pick, not only the
    // config pin's: the newest interpreter version that both satisfies
    // the constraint and is released for THIS platform. A readable index
    // with nothing satisfiable fails here with the named
    // platform-availability error; anything unreadable leaves the pin
    // the target (all pin-path behaviors unchanged).
    // The per-engine download source (spec 05 §2's four-channel chain,
    // tebako#567) — computed once per download, threaded through the
    // index probe and the fetch. Every download journals the base and the
    // channel that supplied it.
    let source = runtime_source(reqs, (!prefless).then_some(pref), &cfg, ctx)?;
    // The pick may REDIRECT the source: the registry-first enumeration
    // (spec 05 §2, roadmap 85) names each version's own release (base +
    // tag + signature pin ride the picked row).
    let (target, source) = match index_selected_target(reqs, pref, &source, ctx)? {
        Some(pick) => pick,
        None if !prefless => (pref.clone(), source),
        None => {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "no compatible runtime for {} \"{}\": {cached_note}\n  and no runtime preference is configured, and neither the factory registry nor the default-line release index read — set `runtimes: {{{}: {{version: …, tebako: …}}}}` in ~/.tebako/config.yaml, or pre-seed the cache",
                    reqs.engine(),
                    described,
                    reqs.engine()
                ),
            );
        }
    };
    let rt = download_runtime(reqs.engine(), &target, &source, ctx)?;
    // The downloaded runtime's abi line must satisfy the payload too —
    // the release index carries it (abi: None is the compat window).
    // The single-entry native shape keeps its exact named error.
    if let [req] = reqs.entries() {
        if let (Some(want), Some(have)) = (&req.abi, &rt.abi) {
            if want != have {
                return fail(
                    EX_TEBAKO_UNAVAILABLE,
                    format!(
                        "downloaded runtime {}@{} carries abi \"{have}\" but the payload requires \"{want}\" — the payload was built against a different platform line; rebuild the payload or pin a matching runtime",
                        reqs.engine(), target.version
                    ),
                );
            }
        }
    }
    // spec 28 §8: SOME entry must match the downloaded shard on the
    // shard's own keys — the index's availability row is advisory; a
    // disagreement (implementation / language_version) is the release
    // lying, named, never a guessed boot.
    if !reqs.entries().iter().any(|r| entry_matches(&rt, r)) {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "downloaded runtime {}@{} matches no entry of the requirement \"{}\" — the release index's availability row disagrees with the shard's own keys; report the release",
                reqs.engine(),
                target.version,
                described
            ),
        );
    }
    Ok(RuntimeResolution::Ready(Box::new(rt)))
}

/// A spawned-runtime dependency edge's dispatch-time pick (spec 30 §4):
/// engine + the optional implementation axis + constraint against the
/// store, the env image REQUIRED (the spawned boot mounts it — a pre-era
/// or partial entry can never serve a spawn). Cache first; on a miss the
/// download rides the primary runtime's machinery (the engine's config
/// preference / the default line), and the edge's own filters re-assert
/// on the downloaded pick — a mismatch is a named error, never a guess.
pub fn resolve_runtime_edge(
    engine: &str,
    implementation: Option<&str>,
    constraint: &tpkg::Constraint,
    allow_download: bool,
    ctx: &Ctx,
) -> Result<CachedRuntime, ShimError> {
    let evaluable = versions::from_validated(constraint);
    if let Some(hit) =
        tpkg::runtime_store::resolve_spawned(&ctx.home, engine, implementation, &evaluable)
    {
        return Ok(hit);
    }
    if !allow_download {
        // Name WHY the version-matching cache entries are ineligible —
        // "do not satisfy the constraint" would send the operator chasing
        // the wrong axis (tebako's named-error law).
        let version_ok: Vec<CachedRuntime> = scan_cached(&ctx.home, engine)
            .into_iter()
            .filter(|c| evaluable.matches(&c.lang_version))
            .collect();
        if !version_ok.is_empty() {
            let reasons = version_ok
                .iter()
                .map(|c| {
                    let mut why = Vec::new();
                    if c.image.is_none() {
                        why.push("no verified env image");
                    }
                    if !tpkg::runtime_store::implementation_matches(c, implementation) {
                        why.push("implementation mismatch");
                    }
                    format!(
                        "{} (tebako {}): {}",
                        c.lang_version,
                        c.tebako_version,
                        why.join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "no cached {engine} runtime can serve the spawned edge \"{}\": version-matching entries are ineligible — {reasons}\n  install a satisfying runtime, or loosen the edge",
                    evaluable.source()
                ),
            );
        }
    }
    let req = RuntimeRequirement {
        engine: engine.to_string(),
        constraint: constraint.clone(),
        implementation: implementation.map(str::to_string),
        abi: None,
    };
    let RuntimeResolution::Ready(rt) =
        resolve_runtime(Some(&RuntimeRequirements::one(req)), allow_download, ctx)?
    else {
        unreachable!("a requirement was passed — never Zero");
    };
    if rt.image.is_none() {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "the resolved {engine} runtime {} (tebako {}) carries no verified env image — a spawned runtime needs the image pair; re-install it with `tebako install`",
                rt.lang_version, rt.tebako_version
            ),
        );
    }
    if let (Some(want), Some(have)) = (implementation, rt.implementation.as_deref()) {
        if have != want {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "the resolved {engine} runtime {} (tebako {}) is implementation \"{have}\" but the payload requires \"{want}\" — pin a {want} runtime or rebuild the payload",
                    rt.lang_version, rt.tebako_version
                ),
            );
        }
    }
    Ok(*rt)
}

/// spec 33 §4: the on_runtime composition's OWNER pick — the depending
/// runtime's shard-mirrored edge (spec 33 §1) resolved through the
/// spawned-edge machinery (a primary-class resolution: cache-first; a
/// miss rides the primary download machinery; the implementation axis
/// re-asserts), then TWO fail-closed gates of the exit-75 class: the
/// owner's launcher line must implement spec 33's entry rule (tebako
/// 2.5.0 or later) and its declared contract must satisfy the depending
/// runtime's owner_contract — never a guessed-around boot.
pub fn resolve_owner(
    mirror: &tpkg::runtime_store::OnRuntimeMirror,
    allow_download: bool,
    ctx: &Ctx,
) -> Result<CachedRuntime, ShimError> {
    let owner = resolve_runtime_edge(
        &mirror.engine,
        mirror.implementation.as_deref(),
        &mirror.constraint,
        allow_download,
        ctx,
    )?;
    let exe_name = owner
        .exe
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // spec 33 §4's first fail-closed gate: the owner must sit on a
    // launcher line implementing spec 33's entry rule (tebako >= 2.5.0)
    // — an older line mis-joins the entry onto the depending runtime's
    // env image. The shard index entry's own tebako_version meta wins
    // when the release declares it; otherwise the shard's recorded
    // tebako version IS the line.
    let line = tpkg::runtime_store::entry_meta(&owner.dir, &exe_name, "tebako_version")
        .unwrap_or_else(|| owner.tebako_version.clone());
    if versions::compare(&line, "2.5.0") == std::cmp::Ordering::Less {
        return fail(
            EX_TEBAKO_CONTRACT,
            format!(
                "the resolved {} runtime {} (tebako {}) cannot own this composition: launcher line {line} predates tebako 2.5.0, the first line implementing spec 33's entry rule — the composition would mis-join the entry onto the depending runtime's env image\n  install a {} runtime on tebako >= 2.5.0",
                mirror.engine, owner.lang_version, owner.tebako_version, mirror.engine
            ),
        );
    }
    let declared = tpkg::runtime_store::entry_contract_version(&owner.dir, &exe_name);
    let want = mirror.owner_contract();
    let satisfied = declared
        .as_deref()
        .is_some_and(|v| versions::from_validated(&want).matches(v));
    if satisfied {
        return Ok(owner);
    }
    fail(
        EX_TEBAKO_CONTRACT,
        format!(
            "the resolved {} runtime {} (tebako {}) cannot own this composition: it declares {} but the depending runtime requires owner_contract \"{}\" (spec 33 §4)\n  install a {} runtime whose release declares a satisfying contract_version",
            mirror.engine,
            owner.lang_version,
            owner.tebako_version,
            match &declared {
                Some(v) => format!("contract_version {v}"),
                None => "no contract_version (a pre-era release)".to_string(),
            },
            want.as_str(),
            mirror.engine,
        ),
    )
}

/// One availability row of the release index (spec 13 §2a — the locked
/// entry shape declares `{engine}_version` + `platform` +
/// `tebako_version`; spec 28 §8 adds the optional `language_version` /
/// `implementation` keys — absent on indices that predate them, the
/// compat window).
struct ReleasedEntry {
    lang_version: String,
    tebako_version: Option<String>,
    language_version: Option<String>,
    implementation: Option<String>,
}

/// The release index's availability facet for `platform` (spec 13 §2a):
/// every entry released for this platform. `None` when NO entry declares
/// the availability keys at all — an index that predates them is
/// uninformative (the config pin stays the target), never an
/// availability verdict.
fn released_versions(text: &str, engine: &str, platform: &str) -> Option<Vec<ReleasedEntry>> {
    let parsed = tebako_json::parse(text).ok()?;
    let tebako_json::Value::Array(entries) = &parsed else {
        return None;
    };
    let lang_key = format!("{engine}_version");
    let mut keyed = false;
    let mut released = Vec::new();
    for entry in entries {
        let (Some(lang_version), Some(entry_platform)) = (
            entry.find(&lang_key).and_then(|v| v.as_string()),
            entry.find("platform").and_then(|v| v.as_string()),
        ) else {
            continue;
        };
        keyed = true;
        if entry_platform == platform {
            released.push(ReleasedEntry {
                lang_version,
                tebako_version: entry.find("tebako_version").and_then(|v| v.as_string()),
                language_version: entry.find("language_version").and_then(|v| v.as_string()),
                implementation: entry.find("implementation").and_then(|v| v.as_string()),
            });
        }
    }
    keyed.then_some(released)
}

/// The download-target selection on a cache miss (spec 05 §2's order,
/// roadmap 85): the factory's in-repo L3 registry first (range
/// enumeration flows through it, never through a release monolith —
/// post-2026-09-12 release lines ship no monolith at all), then the
/// immutable pre-85 fallback: the release index of the pin's tebako line
/// (`v{pref.tebako}/manifest.json`). Both pick the newest interpreter
/// version that satisfies the requirement and is released for this
/// platform; a registry pick redirects the source to the picked row's
/// own release (base + tag + signature pin ride the row, spec 05 §2).
/// spec 28 §8: a row matches when ANY entry of the `any_of` list matches
/// it — the monolith facet applies the row's `implementation` /
/// `language_version` keys; the registry facet's `runtime_entries` call
/// already filtered the implementation axis, so its row versions match
/// directly. Three outcomes:
///
/// - `Ok(None)` — neither facet was informative (unreadable registry AND
///   an unreadable or availability-keyless index; always in offline
///   mode, which never fetches): the config pin stays the target and
///   every pin-path behavior is unchanged;
/// - `Ok(Some((target, source)))` — the pick (the registry row's own
///   release, or the monolith's pick with the entry's own
///   `tebako_version` when declared, else the pin's line);
/// - `Err` — a facet read fine and NOTHING it releases for this platform
///   satisfies the constraint: the named platform-availability error
///   naming the platform, the constraint, and what IS released (the
///   registry's list carries `(withdrawn)` marks) — or the selected
///   registry row is withdrawn: the named WithdrawnPayload refusal
///   (spec 04 §2, never a silent skip).
fn index_selected_target(
    reqs: &RuntimeRequirements,
    pref: &RuntimePref,
    source: &RuntimeSource,
    ctx: &Ctx,
) -> Result<Option<(RuntimePref, RuntimeSource)>, ShimError> {
    if offline_mode(ctx) {
        return Ok(None);
    }
    if let Some(pick) = registry_selected_target(reqs, source, ctx)? {
        return Ok(Some(pick));
    }
    let platform = platform_string();
    let base = skip_file_scheme(&source.base).to_string();
    let local = base_is_local(&source.base);
    // The release tag the URLs ride: the channel-3 registry pin verbatim
    // (its release.ref names it — the openjdk v2.5.1 shape, where the tag
    // is NOT the rows' tebako line), else the probed line's `v<tebako>`.
    let tag = source.tag_for(&pref.tebako);
    let probe = ctx
        .home
        .join("tmp")
        .join(format!("index-probe.{}", std::process::id()));
    if std::fs::create_dir_all(&probe).is_err() {
        // Uninformative, not fatal: the download path re-creates the
        // store dirs under the install lock and names the IO failure.
        return Ok(None);
    }
    let text = fetch_manifest_text(&base, local, &tag, &probe);
    let _ = std::fs::remove_dir_all(&probe);
    let Some(text) = text else {
        // A registry-pinned tag (channel 3) has no fallback line: the
        // registry named THIS release — an unreadable index there is the
        // named error, never a silent drop to the preference's line.
        if let Some(tag) = &source.tag {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "the registry-derived {} runtime release {} carries no readable release index at {}/{}/manifest.json\n  the registry named this release for \"{}\" — fix the registry entry or pin `runtimes: {{{}: {{source: …}}}}` to override it",
                    reqs.engine(),
                    tag,
                    source.base,
                    tag,
                    reqs,
                    reqs.engine()
                ),
            );
        }
        return Ok(None);
    };
    let Some(released) = released_versions(&text, reqs.engine(), platform) else {
        return Ok(None);
    };
    let row_matches = |e: &ReleasedEntry| {
        reqs.entries().iter().any(|r| {
            let impl_ok = match &r.implementation {
                None => true,
                Some(want) => e
                    .implementation
                    .as_deref()
                    .map_or(true, |have| have == want),
            };
            if !impl_ok {
                return false;
            }
            let line = if r.implementation.is_some() {
                &e.lang_version
            } else {
                e.language_version.as_ref().unwrap_or(&e.lang_version)
            };
            versions::from_validated(&r.constraint).matches(line)
        })
    };
    if let Some(pick) = released.iter().filter(|e| row_matches(e)).max_by(|a, b| {
        versions::compare(&a.lang_version, &b.lang_version).then_with(|| {
            versions::compare(
                a.tebako_version.as_deref().unwrap_or(""),
                b.tebako_version.as_deref().unwrap_or(""),
            )
        })
    }) {
        return Ok(Some((
            RuntimePref {
                version: pick.lang_version.clone(),
                tebako: pick
                    .tebako_version
                    .clone()
                    .unwrap_or_else(|| pref.tebako.clone()),
                source: None,
            },
            source.clone(),
        )));
    }
    let mut known: Vec<&str> = released.iter().map(|e| e.lang_version.as_str()).collect();
    known.sort_by(|a, b| versions::compare(a, b));
    known.dedup();
    let known = if known.is_empty() {
        "nothing".to_string()
    } else {
        known.join(", ")
    };
    fail(
        EX_TEBAKO_UNAVAILABLE,
        format!(
            "no released {} runtime for {platform} satisfies \"{reqs}\"\n  released for {platform}: {known}\n  this payload needs a newer {} than this platform provides yet",
            reqs.engine(),
            reqs.engine()
        ),
    )
}

/// `https://github.com/<owner>/<repo>/releases/download` →
/// `(owner, repo)` — the base shape a factory registry ref derives from
/// (spec 05 §2: the in-repo L3 registry of the SAME project the download
/// base names). Any other base shape cannot name a registry: None.
fn github_owner_repo(base: &str) -> Option<(String, String)> {
    let rest = base.strip_prefix("https://github.com/")?;
    let rest = rest.strip_suffix("/releases/download")?;
    let (owner, repo) = rest.split_once('/')?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// The row's tebako line from its declared artifact's stem (spec 05 §2
/// SSOT — the factory's `tebako-runtime-<tebako>-<lang>-<platform>`
/// spelling is flowed, never re-derived elsewhere): strip the
/// `tebako-runtime-` prefix, the `.tfs` payload suffix, and the
/// `-<lang>-<release-asset-platform>` tail; what remains is the line.
/// `None` when the artifact does not carry the spelling (the caller
/// falls back to the release tag).
fn tebako_line_from_artifact(
    artifact: &str,
    lang_version: &str,
    host: tpkg::Platform,
) -> Option<String> {
    let stem = artifact.strip_suffix(".tfs").unwrap_or(artifact);
    let rest = stem.strip_prefix("tebako-runtime-")?;
    let suffix = format!("-{lang_version}-{}", host.release_asset_name());
    let line = rest.strip_suffix(&suffix)?;
    (!line.is_empty()).then(|| line.to_string())
}

/// A composite line-id row's split into (lang_version, tebako line),
/// cross-validated against the row's declared artifact. Factory
/// registries key runtime rows EITHER by the bare language version
/// (openjdk: `21.0.12`) OR by the composite line id `<lang>-<tebako>`
/// (ruby: `4.0.6-0.16.23` — the same interpreter version ships on
/// several tebako lines, and registry versions are unique per payload,
/// so the line id is the only collision-free key). The artifact stem is
/// the SSOT witness (spec 05 §2): the row parses as a composite exactly
/// when ONE dash-split of the row version spells the stem
/// `tebako-runtime-<tebako>-<lang>-<platform>` — zero matching splits
/// (a bare-version row) or several (a genuinely ambiguous id) are both
/// `None`, and the caller keeps the bare-row behavior.
fn split_line_id(
    artifact: &str,
    row_version: &str,
    host: tpkg::Platform,
) -> Option<(String, String)> {
    let stem = artifact.strip_suffix(".tfs").unwrap_or(artifact);
    let platform = host.release_asset_name();
    let mut found = None;
    for (i, _) in row_version.match_indices('-') {
        let (lang, tebako) = (&row_version[..i], &row_version[i + 1..]);
        if lang.is_empty() || tebako.is_empty() {
            continue;
        }
        if stem == format!("tebako-runtime-{tebako}-{lang}-{platform}") {
            if found.is_some() {
                return None;
            }
            found = Some((lang.to_string(), tebako.to_string()));
        }
    }
    found
}

/// spec 05 §2's registry facet of the download-target selection
/// (roadmap 85): derive the factory's in-repo L3 registry
/// (`tfs:github:<owner>/<repo>`) from the download base — every channel,
/// uniform — and enumerate its `kind: runtime` entries for the engine.
/// An unreadable registry is journaled and uninformative (`Ok(None)` —
/// the monolith probe decides, invariant 7's fallback); a registry with
/// no runtime entries for the engine is uninformative the same way. A
/// READABLE, informative registry is authoritative: host-covered rows
/// (`platforms:` selects this host) matching any constraint entry
/// compete newest-first on (version, tebako line); a selected withdrawn
/// row is the named WithdrawnPayload refusal; nothing satisfying is the
/// named platform-availability error listing the host-covered rows with
/// their `(withdrawn)` marks.
fn registry_selected_target(
    reqs: &RuntimeRequirements,
    source: &RuntimeSource,
    ctx: &Ctx,
) -> Result<Option<(RuntimePref, RuntimeSource)>, ShimError> {
    let Some((owner, repo)) = github_owner_repo(&source.base) else {
        return Ok(None);
    };
    let engine = reqs.engine();
    let reg_ref = format!("tfs:github:{owner}/{repo}");
    let registry = match crate::regcache::registry_for(&ctx.home, &reg_ref, ctx) {
        Ok(r) => r,
        Err(e) => {
            journal(
                &ctx.home,
                &format!(
                    "event=runtime-index-registry-error engine={engine} registry={reg_ref} error={e:?}"
                ),
            );
            return Ok(None);
        }
    };
    let implementation = reqs
        .entries()
        .iter()
        .find_map(|r| r.implementation.as_deref());
    let entries = registry.runtime_entries(engine, implementation);
    if entries.is_empty() {
        return Ok(None);
    }
    let host = tpkg::Platform::host();
    let platform = platform_string();
    /// One host-covered, constraint-satisfying registry row.
    struct Candidate {
        payload: String,
        lang_version: String,
        tebako: String,
        base: String,
        tag: String,
        signer_pin: Option<String>,
        withdrawn: bool,
        /// The registry's own version key (the composite line id when
        /// the row is one) — the withdrawn refusal names what the
        /// registry shows, not the split-down pref.
        row_version: String,
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    // Every host-covered row, satisfying or not — the availability
    // error's "released for <platform>" listing (withdrawn rows marked).
    let mut declared: Vec<(String, bool)> = Vec::new();
    for entry in entries {
        for row in &entry.versions {
            let Some(selection) = row.select(host) else {
                continue;
            };
            declared.push((row.version.clone(), row.is_withdrawn()));
            let satisfied = reqs
                .entries()
                .iter()
                .any(|r| versions::from_validated(&r.constraint).matches(&row.version));
            if !satisfied {
                continue;
            }
            // The row's release.ref names where THIS version lives —
            // github service refs only (the runtime fetch grammar); any
            // other class is journaled and skipped, never guessed.
            let (base, tag) = match tebako_resolve::Reference::parse(&row.release.r#ref) {
                Ok(tebako_resolve::Reference::Service {
                    service: tebako_resolve::Service::Github,
                    owner,
                    repo,
                    version: tag,
                    ..
                }) => (
                    format!("https://github.com/{owner}/{repo}/releases/download"),
                    tag,
                ),
                Ok(other) => {
                    journal(
                        &ctx.home,
                        &format!(
                            "event=runtime-index-registry-skip engine={engine} registry={reg_ref} entry={} version={} reason=release-ref-class-{}-not-a-download-base",
                            entry.name,
                            row.version,
                            match &other {
                                tebako_resolve::Reference::Service { service, .. } =>
                                    service.name(),
                                tebako_resolve::Reference::Git { .. } => "git",
                                tebako_resolve::Reference::Https { .. } => "https",
                                tebako_resolve::Reference::File { .. } => "file",
                            }
                        ),
                    );
                    continue;
                }
                Err(e) => {
                    journal(
                        &ctx.home,
                        &format!(
                            "event=runtime-index-registry-skip engine={engine} registry={reg_ref} entry={} version={} reason=bad-release-ref error={e}",
                            entry.name, row.version
                        ),
                    );
                    continue;
                }
            };
            // The row's tebako line: the declared artifact's stem (the
            // openjdk v2.5.1 shape — tag v2.5.1, rows riding tebako
            // 2.5.0), else the tag minus its 'v' (the factory convention
            // `v<tebako>`). A COMPOSITE row version (`<lang>-<tebako>`,
            // the ruby factory's collision-free key) splits against the
            // artifact witness first — the pick's pref names the bare
            // language version or every downstream spelling double-
            // suffixes (`tebako-runtime-0.16.23-4.0.6-0.16.23-…`).
            let from_tag = tag.strip_prefix('v').unwrap_or(&tag).to_string();
            let (lang_version, tebako) = match &selection {
                tebako_resolve::registry::PlatformSelection::Selected { artifact, .. } => {
                    match split_line_id(artifact, &row.version, host) {
                        Some((lang, line)) => (lang, line),
                        None => (
                            row.version.clone(),
                            tebako_line_from_artifact(artifact, &row.version, host)
                                .unwrap_or(from_tag),
                        ),
                    }
                }
                tebako_resolve::registry::PlatformSelection::Universal => {
                    (row.version.clone(), from_tag)
                }
            };
            candidates.push(Candidate {
                payload: entry.name.clone(),
                lang_version,
                tebako,
                base,
                tag,
                signer_pin: row.signature.as_ref().map(|s| s.keyid.clone()),
                withdrawn: row.is_withdrawn(),
                row_version: row.version.clone(),
            });
        }
    }
    let pick = candidates.into_iter().max_by(|a, b| {
        versions::compare(&a.lang_version, &b.lang_version)
            .then_with(|| versions::compare(&a.tebako, &b.tebako))
    });
    let Some(pick) = pick else {
        declared.sort_by(|a, b| versions::compare(&a.0, &b.0));
        declared.dedup();
        let known = if declared.is_empty() {
            "nothing".to_string()
        } else {
            declared
                .iter()
                .map(|(v, w)| {
                    if *w {
                        format!("{v} (withdrawn)")
                    } else {
                        v.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "no released {engine} runtime for {platform} satisfies \"{reqs}\"\n  released for {platform}: {known}\n  this payload needs a newer {engine} than this platform provides yet"
            ),
        );
    };
    // spec 04 §2: the SELECTED row's withdrawal is a named refusal —
    // never a silent skip to an older row, never a fallback to it.
    if pick.withdrawn {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            tebako_resolve::RegistryError::Withdrawn {
                payload: pick.payload,
                version: pick.row_version,
            }
            .to_string(),
        );
    }
    Ok(Some((
        RuntimePref {
            version: pick.lang_version,
            tebako: pick.tebako,
            source: None,
        },
        RuntimeSource {
            base: pick.base,
            tag: Some(pick.tag),
            channel: source.channel,
            signer_pin: pick.signer_pin,
        },
    )))
}

// ---------------------------------------------------------------------
// download — the bootstrap discipline, reimplemented (see module docs)
// ---------------------------------------------------------------------

fn offline_mode(ctx: &Ctx) -> bool {
    ctx.env_get("TEBAKO_OFFLINE")
        .is_some_and(|v| !v.is_empty() && v != "0")
}

/// The per-engine download source (spec 05 §2's chain — tebako#567): the
/// release download base, the tag the URLs ride, the chain channel that
/// supplied them (journaled per fetch), and — channel 3 only — the
/// registry entry's signature pin (spec 09 §9).
#[derive(Debug, Clone)]
struct RuntimeSource {
    /// The release download base (`{base}/{tag}/<asset>` URLs).
    base: String,
    /// The pinned release tag. `None` = `v<tebako>` of the line being
    /// read (the factory convention: the probed line for the index, the
    /// pick's own line for the assets). `Some` pins ONE tag for every
    /// fetch — channel 3's registry-derived form, where the registry
    /// names the release and a row's `tebako_version` stays the identity
    /// line (the openjdk v2.5.1 shape: tag v2.5.1, rows on tebako 2.5.0).
    tag: Option<String>,
    /// The chain channel that supplied this source (spec 05 §2's journal
    /// requirement): `config-source` / `mirror-env` / `registry` /
    /// `default`.
    channel: &'static str,
    /// Channel 3's registry signature pin (spec 09 §9): the PRIMARY keyid
    /// the verified signer of the index/artifacts must resolve to.
    signer_pin: Option<String>,
}

impl RuntimeSource {
    /// The tag a fetch on tebako line `tebako_line` rides.
    fn tag_for(&self, tebako_line: &str) -> String {
        self.tag
            .clone()
            .unwrap_or_else(|| format!("v{tebako_line}"))
    }
}

/// Append one line to the audit journal (`~/.tebako/journal.log`) —
/// best-effort, like every journal write: the answer never depends on
/// the record.
pub(crate) fn journal(home: &Path, line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("journal.log"))
    {
        use std::io::Write as _;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = f.write_all(format!("{now} {line}\n").as_bytes());
    }
}

/// `TEBAKO_REQUIRE_SIGNED=1` (spec 09 §4): set, non-empty, not "0".
fn require_signed(ctx: &Ctx) -> bool {
    ctx.env_get("TEBAKO_REQUIRE_SIGNED")
        .is_some_and(|v| !v.is_empty() && v != "0")
}

/// The per-engine download-source chain (spec 05 §2, tebako#567) — first
/// hit wins:
///
/// 1. the config pin's `source:` (a differing `TEBAKO_RUNTIME_MIRROR` is
///    shadowed — loud + journaled);
/// 2. `TEBAKO_RUNTIME_MIRROR` (the operator's global override);
/// 3. the registered registries' `kind: runtime` entries whose `engine`
///    (+ `implementation` when the edge names one) matches, with a
///    version satisfying the edge — the base AND the tag derive from
///    that version's `release.ref` (the zero-config path);
/// 4. the product default — the ruby factory, RUBY ONLY: a non-ruby
///    engine no channel answers is the named error enumerating the
///    channels (the #567 closeout).
fn runtime_source(
    reqs: &RuntimeRequirements,
    config_pref: Option<&RuntimePref>,
    cfg: &config::UserConfig,
    ctx: &Ctx,
) -> Result<RuntimeSource, ShimError> {
    let engine = reqs.engine();
    let mirror = ctx
        .env_get("TEBAKO_RUNTIME_MIRROR")
        .filter(|v| !v.is_empty());
    // Channel 1: the config pin's declared source.
    if let Some(base) = config_pref.and_then(|p| p.source.as_deref()) {
        if let Some(mirror) = mirror.filter(|m| *m != base) {
            // The pin shadows the operator's mirror for this engine —
            // never silently (spec 05 §2).
            eprintln!(
                "tebako-shim: warning: the {engine} runtime source: pin ({base}) shadows TEBAKO_RUNTIME_MIRROR ({mirror}) for this engine"
            );
            journal(
                &ctx.home,
                &format!(
                    "event=runtime-source-shadow engine={engine} source={base} shadowed_mirror={mirror}"
                ),
            );
        }
        return Ok(RuntimeSource {
            base: base.to_string(),
            tag: None,
            channel: "config-source",
            signer_pin: None,
        });
    }
    // Channel 2: the operator's global mirror.
    if let Some(mirror) = mirror {
        return Ok(RuntimeSource {
            base: mirror.to_string(),
            tag: None,
            channel: "mirror-env",
            signer_pin: None,
        });
    }
    // Channel 3: the registered registries (the zero-config path).
    if let Some(source) = registry_derived_source(reqs, cfg, ctx)? {
        return Ok(source);
    }
    // Channel 4: the product default — the ruby factory hosts ruby only.
    if engine == "ruby" {
        return Ok(RuntimeSource {
            base: DEFAULT_RELEASES_BASE.to_string(),
            tag: None,
            channel: "default",
            signer_pin: None,
        });
    }
    fail(
        EX_TEBAKO_UNAVAILABLE,
        format!(
            "no download source for {engine} runtimes — every channel of the per-engine chain (spec 05 §2) came up empty:\n  1. config.yaml `runtimes: {{{engine}: {{source: …}}}}` — not set\n  2. TEBAKO_RUNTIME_MIRROR — not set\n  3. the registered registries — none lists a `kind: runtime` entry for engine \"{engine}\" with a version satisfying \"{reqs}\"\n  4. the product default — hosts ruby runtimes only\n  register the registry that publishes the {engine} runtime (`tebako add-registry`), or pin a source"
        ),
    )
}

/// Channel 3: the registry-derived source. Scans the configured
/// registries IN ORDER; the first `kind: runtime` entry matching the
/// engine (+ the edge's implementation when named) with a version
/// satisfying the requirement answers — the base + tag derive from that
/// version's `release.ref`. Only GitHub service refs derive a
/// `{base}/{tag}` download root (the runtime fetch grammar); other ref
/// classes skip with a journal note. A registry that does not resolve is
/// journaled and skipped — it cannot answer, and a later channel still
/// can (the failure is named in the no-channel error's enumeration).
/// spec 04 §2: the pick is status-blind — a SELECTED withdrawn row is
/// the named WithdrawnPayload refusal, never a silent skip.
fn registry_derived_source(
    reqs: &RuntimeRequirements,
    cfg: &config::UserConfig,
    ctx: &Ctx,
) -> Result<Option<RuntimeSource>, ShimError> {
    let engine = reqs.engine();
    let implementation = reqs
        .entries()
        .iter()
        .find_map(|r| r.implementation.as_deref());
    for reg_ref in &cfg.registries {
        let registry = match crate::regcache::registry_for(&ctx.home, reg_ref, ctx) {
            Ok(r) => r,
            Err(e) => {
                journal(
                    &ctx.home,
                    &format!(
                        "event=runtime-source-registry-error engine={engine} registry={reg_ref} error={e:?}"
                    ),
                );
                continue;
            }
        };
        for entry in registry.runtime_entries(engine, implementation) {
            // The newest registry version satisfying ANY entry of the
            // `any_of` requirement answers where this engine lives.
            let pick = entry
                .versions
                .iter()
                .filter(|v| {
                    reqs.entries()
                        .iter()
                        .any(|r| versions::from_validated(&r.constraint).matches(&v.version))
                })
                .max_by(|a, b| versions::compare(&a.version, &b.version));
            let Some(version) = pick else { continue };
            if version.is_withdrawn() {
                return fail(
                    EX_TEBAKO_UNAVAILABLE,
                    tebako_resolve::RegistryError::Withdrawn {
                        payload: entry.name.clone(),
                        version: version.version.clone(),
                    }
                    .to_string(),
                );
            }
            let derived = match tebako_resolve::Reference::parse(&version.release.r#ref) {
                Ok(tebako_resolve::Reference::Service {
                    service: tebako_resolve::Service::Github,
                    owner,
                    repo,
                    version: tag,
                    ..
                }) => Some(RuntimeSource {
                    base: format!("https://github.com/{owner}/{repo}/releases/download"),
                    tag: Some(tag),
                    channel: "registry",
                    signer_pin: version.signature.as_ref().map(|s| s.keyid.clone()),
                }),
                // A non-GitHub release.ref cannot spell the `{base}/{tag}`
                // download root the runtime fetch rides — journaled, never
                // guessed.
                Ok(other) => {
                    journal(
                        &ctx.home,
                        &format!(
                            "event=runtime-source-registry-skip engine={engine} registry={reg_ref} entry={} version={} reason=release-ref-class-{}-not-a-download-base",
                            entry.name,
                            version.version,
                            match &other {
                                tebako_resolve::Reference::Service { service, .. } =>
                                    service.name(),
                                tebako_resolve::Reference::Git { .. } => "git",
                                tebako_resolve::Reference::Https { .. } => "https",
                                tebako_resolve::Reference::File { .. } => "file",
                            }
                        ),
                    );
                    None
                }
                Err(e) => {
                    journal(
                        &ctx.home,
                        &format!(
                            "event=runtime-source-registry-skip engine={engine} registry={reg_ref} entry={} version={} reason=bad-release-ref error={e}",
                            entry.name, version.version
                        ),
                    );
                    None
                }
            };
            if derived.is_some() {
                return Ok(derived);
            }
        }
    }
    Ok(None)
}

fn base_is_local(base: &str) -> bool {
    !(base.starts_with("http://") || base.starts_with("https://"))
}

fn skip_file_scheme(base: &str) -> &str {
    // RFC 8089 drive recovery included: `file:///C:/x` strips to `/C:/x`,
    // which is not a windows path — file_path_from_url hands back `C:/x`.
    // Unix remainders pass through unchanged.
    tebako_http::file_path_from_url(base.strip_prefix("file://").unwrap_or(base))
}

fn file_exists(path: &Path) -> bool {
    path.exists()
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = path;
}

fn make_readonly(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444));
    }
    #[cfg(not(unix))]
    let _ = path;
}

fn sha256_file_hex(path: &Path) -> std::io::Result<String> {
    use sha2::Digest as _;
    let mut f = std::fs::File::open(path)?;
    let mut h = sha2::Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let digest = h.finalize();
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    Ok(s)
}

// -- per-entry install lock (mirrors the bootstrap's flock discipline) --

struct EntryLock(std::fs::File);

#[cfg(unix)]
fn flock_acquire(path: &Path, timeout_ms: u64) -> std::io::Result<EntryLock> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&f);
    loop {
        let rc = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if rc == 0 {
            return Ok(EntryLock(f));
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EWOULDBLOCK)
            && err.kind() != std::io::ErrorKind::Interrupted
        {
            return Err(err);
        }
        if err.kind() != std::io::ErrorKind::Interrupted && std::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "lock timeout",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_POLL_MS));
    }
}

/// Windows: LockFileEx on one byte at offset 0 of the lock file — the
/// same semantics as the unix flock (exclusive, non-blocking attempts on
/// a poll until the timeout; the kernel releases a crashed holder's lock
/// when the handle dies). The shape the bootstrap's platform.rs and
/// tebako-resolve's cache.rs already use.
#[cfg(windows)]
fn flock_acquire(path: &Path, timeout_ms: u64) -> std::io::Result<EntryLock> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::{ERROR_IO_PENDING, ERROR_LOCK_VIOLATION};
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;

    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        let mut ov = OVERLAPPED::default();
        let ok = unsafe {
            LockFileEx(
                f.as_raw_handle(),
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                1,
                0,
                &mut ov,
            )
        };
        if ok != 0 {
            return Ok(EntryLock(f));
        }
        let err = std::io::Error::last_os_error();
        let raw = err.raw_os_error().unwrap_or(0);
        if raw != ERROR_LOCK_VIOLATION as i32 && raw != ERROR_IO_PENDING as i32 {
            return Err(err);
        }
        if std::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "lock timeout",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_POLL_MS));
    }
}

fn lock_release(lock: EntryLock) {
    #[cfg(unix)]
    {
        let fd = std::os::unix::io::AsRawFd::as_raw_fd(&lock.0);
        unsafe {
            libc::flock(fd, libc::LOCK_UN);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::Storage::FileSystem::UnlockFileEx;
        use windows_sys::Win32::System::IO::OVERLAPPED;
        let mut ov = OVERLAPPED::default();
        unsafe {
            UnlockFileEx(lock.0.as_raw_handle(), 0, 1, 0, &mut ov);
        }
        // dropping the file closes the handle, releasing the lock regardless
    }
}

/// Fetch one URL (in-process HTTP; `file://`/local mirrors are copies).
/// curl --retry 3 parity: transient failures get three attempts.
#[allow(clippy::result_unit_err)]
fn fetch_url(url: &str, local: bool, out: &Path) -> Result<(), ()> {
    if local {
        return std::fs::copy(Path::new(url), out)
            .map(|_| ())
            .map_err(|_| ());
    }
    let mut attempts = 0;
    let mut throttles = 0;
    loop {
        match tebako_http::get(url) {
            Ok(bytes) => return std::fs::write(out, bytes).map_err(|_| ()),
            Err(tebako_http::FetchError::IndexUnavailable(_)) => return Err(()),
            Err(tebako_http::FetchError::Throttled { retry_after, .. }) => {
                throttles += 1;
                if throttles >= tebako_http::THROTTLE_ROUNDS {
                    return Err(());
                }
                std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
            }
            Err(e) => {
                attempts += 1;
                if attempts >= 3 {
                    eprintln!("tebako-shim: download failed: {e}");
                    return Err(());
                }
            }
        }
    }
}

/// The expected sha256 of `asset` in a release manifest (spec 13's
/// machine-readable index — the additive image-era `image` key
/// included): a per-entry lookup — the entry's `filename` answers its
/// `sha256`; the nested `image.filename` answers the image's own
/// `sha256`. An absent asset is no answer (the SHA256SUMS fallback
/// decides; the v1-era image rule needs the miss).
#[allow(clippy::result_unit_err)]
fn sha_from_manifest(text: &str, asset: &str) -> Result<String, ()> {
    let parsed = tebako_json::parse(text).map_err(|_| ())?;
    let tebako_json::Value::Array(entries) = &parsed else {
        return Err(());
    };
    for entry in entries {
        if entry
            .find("filename")
            .and_then(|f| f.as_string())
            .as_deref()
            == Some(asset)
        {
            return entry.find("sha256").and_then(|s| s.as_string()).ok_or(());
        }
        if let Some(image) = entry.find("image") {
            if image
                .find("filename")
                .and_then(|f| f.as_string())
                .as_deref()
                == Some(asset)
            {
                return image.find("sha256").and_then(|s| s.as_string()).ok_or(());
            }
        }
    }
    Err(())
}

// The release manifest's ruby DLL facet (tebako-runtime-ruby#40 — the
// additive `dll` key, windows packages only) is parsed in tpkg
// (`runtime_store::entry_dll_from_index`, the single grammar owner): the
// shim reads it at download, the press's spawned-row walk from the
// cached index. The PE name (`install_as`) exists only in the manifest,
// never derived (the factory's RubyVersion#msys_dll_name is its single
// owner).

/// SHA256SUMS.txt fallback: "<64hex><spaces>[*]<filename>" per line.
#[allow(clippy::result_unit_err)]
fn sha_from_sums(text: &str, asset: &str) -> Result<String, ()> {
    for line in text.lines() {
        let line = line.trim_end_matches(['\r', ' ', '\t']);
        if line.len() > 66 && line[..64].bytes().all(|b| b.is_ascii_hexdigit()) {
            let name = line[64..].trim_start_matches([' ', '\t']);
            let name = name.strip_prefix('*').unwrap_or(name);
            let name = name.trim_end_matches([' ', '\t']);
            if name == asset {
                return Ok(line[..64].to_string());
            }
        }
    }
    Err(())
}

/// Fetch the release index (`manifest.json`) at release tag `tag` into
/// the tmp staging dir and return its text. `None` when it does not
/// exist or does not read — the caller decides what that means (spec 18:
/// the pre-era signal).
fn fetch_manifest_text(base: &str, local: bool, tag: &str, tmp_dir: &Path) -> Option<String> {
    let manifest_tmp = tmp_dir.join("manifest.json");
    fetch_url(&format!("{base}/{tag}/manifest.json"), local, &manifest_tmp).ok()?;
    std::fs::read_to_string(&manifest_tmp).ok()
}

/// spec 18 C2 pre-download gate (S11/S12): the release manifest's entry
/// for the runtime exe must declare its contract set — tebako-resolve's
/// reader owns the semantics (the shim links it); the refusal is exit 75
/// with both sides named. An entry-less asset is undeclared by
/// definition (no old-path readers) — the refusal names the identity
/// triple alongside the asset spelling tried (spec 05 §2; tebako#456).
#[allow(clippy::too_many_arguments)]
fn contract_gate(
    runtime_ref: &str,
    manifest_text: &str,
    asset: &str,
    engine: &str,
    lang_version: &str,
    tebako_version: &str,
    platform: &str,
) -> Result<(), ShimError> {
    match tebako_resolve::contract::gate(manifest_text, asset) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => fail(
            EX_TEBAKO_CONTRACT,
            format!(
                "runtime \"{runtime_ref}\" is pre-era — its release manifest entry declares no contract set (no entry for tebako_version={tebako_version} {engine}_version={lang_version} platform={platform} (asset spelling {asset})) — refusing to install or execute\n  the release was built by a pre-contract factory; rebuild it with the current tebako-runtime-ruby (spec 18 C2), or pin a runtime that declares its contract"
            ),
        ),
        Err(e) => fail(
            EX_TEBAKO_CONTRACT,
            format!("runtime \"{runtime_ref}\": {e}"),
        ),
    }
}

/// Which release-index form the G1 verification (spec 09 §4) trusted for
/// this fetch — the digests come from THAT form and no other (an
/// unverified form's digests are never consulted once a form verified).
enum IndexTrust {
    /// No form carried a verifiable signature — the pre-signing
    /// keep-forever line: manifest primary, SHA256SUMS fallback (the
    /// behavior that predates G1), the unsigned rule applies.
    Unverified,
    /// manifest.json verified (the signer's resolved PRIMARY keyid).
    VerifiedManifest(String),
    /// Only SHA256SUMS.txt verified — digests from it; the contract card
    /// has no trusted carrier (the caller refuses 75).
    VerifiedSums {
        text: String,
        #[allow(dead_code)]
        signer: String,
    },
}

impl IndexTrust {
    /// The verified signer (resolved PRIMARY keyid), when any form
    /// verified.
    fn signer(&self) -> Option<&str> {
        match self {
            IndexTrust::Unverified => None,
            IndexTrust::VerifiedManifest(signer) => Some(signer),
            IndexTrust::VerifiedSums { signer, .. } => Some(signer),
        }
    }
}

/// The fetch coordinates an expected-checksum lookup serves: the
/// asset-in-release identity plus the already-acquired index card and
/// its trust verdict.
struct ChecksumQuery<'a> {
    base: &'a str,
    local: bool,
    tag: &'a str,
    asset: &'a str,
    tmp_dir: &'a Path,
    index_text: &'a str,
    /// Marks which card slot the diagnostics report against.
    from_shard: bool,
    trust: &'a IndexTrust,
}

/// The lookup's answer: the optional sha plus the three diagnostic
/// indices (shard, manifest, sums) so the caller names the failure
/// itself — an absent entry is data, not an error (the v1-era image
/// rule needs it).
struct ChecksumAnswer {
    sha: Option<String>,
    diags: (usize, usize, usize),
}

/// The expected checksum for an asset: the VERIFIED index form's digests
/// when a form verified (spec 09 §4 — never an unverified fallback);
/// otherwise the consumed release card (shard or monolith — already in
/// hand from the acquisition) primary, SHA256SUMS.txt fallback (the
/// bootstrap's exact order).
fn expected_checksum(q: &ChecksumQuery) -> Result<ChecksumAnswer, ShimError> {
    // The verified chains read the consumed form's digests and nothing
    // else — an unverified form is never consulted once a form verified.
    match q.trust {
        IndexTrust::VerifiedManifest(_) => {
            let expected = sha_from_manifest(q.index_text, q.asset).ok();
            let diag = if expected.is_some() { 4 } else { 3 };
            return Ok(ChecksumAnswer {
                sha: expected,
                diags: if q.from_shard {
                    (diag, 0, 0)
                } else {
                    (0, diag, 0)
                },
            });
        }
        IndexTrust::VerifiedSums { text, .. } => {
            let expected = sha_from_sums(text, q.asset).ok();
            let diag = if expected.is_some() { 4 } else { 3 };
            return Ok(ChecksumAnswer {
                sha: expected,
                diags: (0, 0, diag),
            });
        }
        IndexTrust::Unverified => {}
    }
    let mut expected = sha_from_manifest(q.index_text, q.asset).ok();
    let diag_card = if expected.is_some() { 4 } else { 3 };
    let (diag_shard, diag_manifest) = if q.from_shard {
        (diag_card, 0)
    } else {
        (0, diag_card)
    };
    let mut diag_sums = 0;
    if expected.is_none() {
        diag_sums = 1;
        let sums_tmp = q.tmp_dir.join("SHA256SUMS.txt");
        if fetch_url(
            &format!("{}/{}/SHA256SUMS.txt", q.base, q.tag),
            q.local,
            &sums_tmp,
        )
        .is_ok()
        {
            diag_sums = 2;
            if let Ok(text) = std::fs::read_to_string(&sums_tmp) {
                diag_sums = 3;
                if let Ok(sha) = sha_from_sums(&text, q.asset) {
                    diag_sums = 4;
                    expected = Some(sha);
                }
            }
        }
    }
    Ok(ChecksumAnswer {
        sha: expected,
        diags: (diag_shard, diag_manifest, diag_sums),
    })
}

/// The G1 fetch-time verification context (spec 09 §4): the keyring this
/// fetch verifies against — the user's trusted keyring + the embedded
/// first-party root + the `TEBAKO_TRUSTED_ROOT` dev override (the CLI's
/// payload-install keyring, mirrored key-for-key) — plus the channel-3
/// registry signature pin (spec 09 §9) when the source came from a
/// registry.
struct FetchTrust {
    keyring: Vec<u8>,
    signer_pin: Option<String>,
}

impl FetchTrust {
    fn build(source: &RuntimeSource, ctx: &Ctx) -> Result<FetchTrust, ShimError> {
        let mut keyring = tebako_signer::trusted_keyring_bytes(&ctx.home).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot read the trusted keyring: {e}"),
            )
        })?;
        let root = tebako_signer::dearmor_bytes(tebako_signer::ROOT_PUBLIC_KEY.as_bytes())
            .map_err(|e| {
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
        Ok(FetchTrust {
            keyring,
            signer_pin: source.signer_pin.clone(),
        })
    }

    /// Verify one detached-signature pair (spec 09 §4's strict rule):
    /// Invalid → 71; Untrusted (the signer is not in the trusted
    /// keyring) → 72; Trusted → the declared keyid (the entry's
    /// `signature.keyid`, the PRIMARY per spec 13 §2a) and the channel-3
    /// registry pin re-assert — a mismatch is SignerKeyChanged, 72.
    /// Returns the resolved PRIMARY keyid of the verified signer.
    fn verify_detached(
        &self,
        what: &str,
        bytes: &[u8],
        asc: &[u8],
        declared_keyid: Option<&str>,
    ) -> Result<String, ShimError> {
        let outcome =
            tebako_signer::verify_detached_full(&self.keyring, bytes, asc).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_SIGNATURE,
                    format!("cannot verify the signature on {what}: {e}"),
                )
            })?;
        let issuer = match outcome {
            tebako_signer::VerifyOutcome::Trusted(keyid) => keyid,
            tebako_signer::VerifyOutcome::Untrusted(keyid) => {
                return fail(
                    EX_TEBAKO_TRUST,
                    format!(
                        "{what} is signed by {keyid}, which is not in the trusted keyring — refusing to install or execute\n  if you trust this signer, register its public key with `tebako key import`"
                    ),
                );
            }
            tebako_signer::VerifyOutcome::Invalid(keyid) => {
                return fail(
                    EX_TEBAKO_SIGNATURE,
                    format!(
                        "invalid signature on {what}{} — refusing to install or execute\n  the download was deleted; the cache was not touched",
                        keyid.map(|k| format!(" (issuer {k})")).unwrap_or_default()
                    ),
                );
            }
        };
        let primary = tebako_signer::primary_keyid_of(&self.keyring, &issuer)
            .map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_SIGNATURE,
                    format!("cannot resolve the signer of {what}: {e}"),
                )
            })?
            .unwrap_or_else(|| issuer.clone());
        // The entry pin (spec 13 §2a) and the channel-3 registry pin
        // (spec 09 §9) both name PRIMARY keyids; a signature issuing
        // from a signing SUBKEY resolves to its primary before the
        // compare.
        for (pin, origin) in [
            (declared_keyid, "the release index entry declares"),
            (self.signer_pin.as_deref(), "the registry pins"),
        ] {
            if let Some(want) = pin {
                let want = want.to_lowercase();
                if issuer != want && primary != want {
                    return fail(
                        EX_TEBAKO_TRUST,
                        format!(
                            "{what} is signed by {primary}, but {origin} {want} — the signer key changed (spec 09 §9); refusing to install or execute"
                        ),
                    );
                }
            }
        }
        Ok(primary)
    }

    /// The per-artifact leg (spec 09 §4): fetch the declared `.asc` (a
    /// DECLARED asc that does not fetch → 71) and verify the downloaded
    /// bytes BEFORE the sha256 check. Returns the verified signer's
    /// resolved PRIMARY keyid.
    fn verify_asset(
        &self,
        dir_url: &str,
        local: bool,
        tmp_dir: &Path,
        asset_path: &Path,
        asset: &str,
        declared: &EntrySignature,
    ) -> Result<String, ShimError> {
        // The asc names an exact asset within the same release — a bare
        // file name, never a path (spec 13 §2a).
        if declared.asc.contains('/') || declared.asc.contains('\\') {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "the release index's signature.asc for {asset} (\"{}\") is not a bare asset name — the release is malformed",
                    declared.asc
                ),
            );
        }
        let asc_url = format!("{dir_url}/{}", declared.asc);
        let asc_tmp = tmp_dir.join(&declared.asc);
        if fetch_url(&asc_url, local, &asc_tmp).is_err() {
            return fail(
                EX_TEBAKO_SIGNATURE,
                format!(
                    "the release index declares signature \"{}\" for {asset} but it did not fetch from {asc_url} — a declared signature that does not fetch is refused (spec 09 §4)",
                    declared.asc
                ),
            );
        }
        let asc = std::fs::read(&asc_tmp).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!(
                    "cannot read the fetched signature {}: {e}",
                    asc_tmp.display()
                ),
            )
        })?;
        let bytes = std::fs::read(asset_path).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot read the downloaded {}: {e}", asset_path.display()),
            )
        })?;
        self.verify_detached(asset, &bytes, &asc, Some(&declared.keyid))
    }
}

/// What the shard-first index acquisition (spec 05 §2, roadmap 85)
/// consumed: the release-card text (a consumed shard NORMALIZED to the
/// one-entry array shape every card reader speaks — the store's cached
/// `manifest.json`, `release_index_entry`, `sha_from_manifest`), the
/// trust verdict of the consumed form, and which form served.
struct AcquiredIndex {
    text: String,
    trust: IndexTrust,
    from_shard: bool,
}

/// The index-trust resolution + acquisition (spec 09 §4 + spec 05 §2's
/// shard-first order): the trust scan probes each form's detached `.asc`
/// in preference order — `<stem>.manifest.json` → `manifest.json` →
/// `SHA256SUMS.txt` — and the FIRST form whose asc verifies over its
/// served bytes is the consumed form, its digests alone trusted from
/// that point (strict: Invalid → 71, an untrusted signer → 72). An asc
/// whose own body does not fetch skips the form; a verified but
/// triple-mismatched shard falls through to the next form with its URL
/// recorded for the failure message. With no verifiable form the
/// unsigned chain consumes the shard when it reads AND names the
/// requested identity triple, else the monolith; no readable card form
/// at all is the spec 18 C2 pre-era refusal (75) naming every URL tried.
#[allow(clippy::too_many_arguments)]
fn acquire_index(
    trust: &FetchTrust,
    base: &str,
    local: bool,
    tag: &str,
    stem: &str,
    engine: &str,
    lang_version: &str,
    tebako_version: &str,
    platform: &str,
    runtime_ref: &str,
    tmp_dir: &Path,
) -> Result<AcquiredIndex, ShimError> {
    let dir_url = format!("{base}/{tag}");
    let shard_name = format!("{stem}.manifest.json");
    let shard_url = format!("{dir_url}/{shard_name}");
    let manifest_url = format!("{dir_url}/manifest.json");
    let fetch_opt = |name: &str| -> Option<Vec<u8>> {
        let tmp = tmp_dir.join(name);
        fetch_url(&format!("{dir_url}/{name}"), local, &tmp).ok()?;
        std::fs::read(&tmp).ok()
    };
    // The shard serves only when it names the requested identity triple
    // (spec 05 §2: a triple-mismatched shard is not this package's card).
    // The consumed text normalizes to the array shape at the boundary.
    let shard_card = |body: &[u8]| -> Option<String> {
        let body = String::from_utf8(body.to_vec()).ok()?;
        let card = format!("[{}]", body.trim_end());
        let parsed = tebako_json::parse(&card).ok()?;
        release_index_entry(&parsed, engine, lang_version, tebako_version, platform)?;
        Some(card)
    };
    // Why the shard was not consumed (recorded for the failure message).
    let mut shard_note: Option<String> = None;

    // The trust scan — the first TRUSTED form wins.
    if let Some(asc) = fetch_opt(&format!("{shard_name}.asc")) {
        match fetch_opt(&shard_name) {
            Some(body) => {
                let signer = trust.verify_detached(&shard_name, &body, &asc, None)?;
                match shard_card(&body) {
                    Some(card) => {
                        return Ok(AcquiredIndex {
                            text: card,
                            trust: IndexTrust::VerifiedManifest(signer),
                            from_shard: true,
                        });
                    }
                    None => {
                        shard_note = Some(format!(
                            "the shard {shard_url} verified but names another identity triple"
                        ));
                    }
                }
            }
            None => {
                shard_note = Some(format!(
                    "the shard's signature fetched but {shard_url} did not"
                ));
            }
        }
    }
    if let Some(asc) = fetch_opt("manifest.json.asc") {
        // A dangling asc over an unfetchable body is not a verified form
        // — the next form is probed.
        if let Some(body) = fetch_opt("manifest.json") {
            let signer = trust.verify_detached("manifest.json", &body, &asc, None)?;
            let text = String::from_utf8(body).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_MANIFEST,
                    format!("the verified manifest.json at {manifest_url} is not UTF-8: {e}"),
                )
            })?;
            return Ok(AcquiredIndex {
                text,
                trust: IndexTrust::VerifiedManifest(signer),
                from_shard: false,
            });
        }
    }
    if let Some(asc) = fetch_opt("SHA256SUMS.txt.asc") {
        if let Some(sums) = fetch_opt("SHA256SUMS.txt").and_then(|b| String::from_utf8(b).ok()) {
            let signer = trust.verify_detached("SHA256SUMS.txt", sums.as_bytes(), &asc, None)?;
            return Ok(AcquiredIndex {
                text: String::new(),
                trust: IndexTrust::VerifiedSums { text: sums, signer },
                from_shard: false,
            });
        }
    }

    // The unsigned chain (the pre-signing keep-forever line): shard
    // first, the monolith next — a skipped shard (its own asc proved it
    // is not this package's card) is not reconsidered here.
    if shard_note.is_none() {
        if let Some(body) = fetch_opt(&shard_name) {
            match shard_card(&body) {
                Some(card) => {
                    return Ok(AcquiredIndex {
                        text: card,
                        trust: IndexTrust::Unverified,
                        from_shard: true,
                    });
                }
                None => {
                    shard_note = Some(format!(
                        "the shard {shard_url} does not name the requested identity triple ({engine}_version={lang_version} tebako_version={tebako_version} platform={platform})"
                    ));
                }
            }
        }
    }
    if let Some(bytes) = fetch_opt("manifest.json") {
        if let Ok(text) = String::from_utf8(bytes) {
            return Ok(AcquiredIndex {
                text,
                trust: IndexTrust::Unverified,
                from_shard: false,
            });
        }
    }
    let mut msg = format!(
        "runtime \"{runtime_ref}\" is pre-era — no readable release index for it\n  tried: {shard_url}\n         {manifest_url}\n  the release was built by a pre-contract factory; rebuild it with the current tebako-runtime-ruby (spec 18 C2), or pin a runtime that declares its contract"
    );
    if let Some(note) = &shard_note {
        msg.push_str(&format!("\n  shard: {note}"));
    }
    fail(EX_TEBAKO_CONTRACT, msg)
}

/// Download + verify + atomically install one asset into an entry staging
/// dir. The spec 09 §4 order: the DECLARED signature verifies first
/// (invalid → 71, untrusted signer / pin mismatch → 72), then the sha256
/// (70). `fetch_name` is the release-asset spelling (the URL's last
/// segment); `install_name` the name it stages under (the windows dll
/// facet's PE name — everywhere else the two are one). Returns the
/// verified sha256 plus the verified signer's PRIMARY keyid when a
/// signature was declared and verified.
#[allow(clippy::too_many_arguments)]
fn install_asset(
    dir_url: &str,
    local: bool,
    fetch_name: &str,
    install_name: &str,
    tmp_dir: &Path,
    expected: &str,
    sig: Option<(&EntrySignature, &FetchTrust)>,
) -> Result<(String, Option<String>), ShimError> {
    let url = format!("{dir_url}/{fetch_name}");
    let tmp_asset = tmp_dir.join(install_name);
    if fetch_url(&url, local, &tmp_asset).is_err() {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "runtime download failed\n  url: {url}\n  downloads are in-process (ureq + rustls, webpki-roots) — check the network, or set\n  TEBAKO_RUNTIME_MIRROR to a reachable mirror, or TEBAKO_OFFLINE=1 for cache-only mode"
            ),
        );
    }
    let signer = match sig {
        Some((declared, trust)) => {
            Some(trust.verify_asset(dir_url, local, tmp_dir, &tmp_asset, fetch_name, declared)?)
        }
        None => None,
    };
    let actual = sha256_file_hex(&tmp_asset).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot hash downloaded file {}: {e}", tmp_asset.display()),
        )
    })?;
    if expected.to_lowercase() != actual {
        let _ = std::fs::remove_file(&tmp_asset);
        return fail(
            EX_TEBAKO_SHA,
            format!(
                "SHA256 mismatch for downloaded runtime {fetch_name} — refusing to install or execute\n  expected: {} (from the release index)\n  actual:   {actual}\n  the download was deleted; the cache was not touched",
                expected.to_lowercase()
            ),
        );
    }
    Ok((actual, signer))
}

/// What the staging step produced: a fresh install (the staged names)
/// or the discovery that another installer published the entry under
/// the flowed spelling while the index was being fetched (use the
/// cache as-is).
enum StageOutcome {
    Raced {
        asset: String,
        image_asset: String,
    },
    Installed {
        has_image: bool,
        asset: String,
        image_asset: String,
    },
}

/// Download the preferred runtime into the shared cache with the
/// bootstrap's install discipline: per-entry flock (120 s), re-check
/// under the lock, tmp staging, sha256-verified, tmp + rename publish,
/// trust markers, read-only image.
fn download_runtime(
    engine: &str,
    pref: &RuntimePref,
    source: &RuntimeSource,
    ctx: &Ctx,
) -> Result<CachedRuntime, ShimError> {
    let platform = platform_string();
    let entry = format!("{engine}-{}-{}-{platform}", pref.version, pref.tebako);
    let runtime_ref = format!("{engine}@{};tebako={};image", pref.version, pref.tebako);
    let root = ctx.home.clone();
    let entry_dir = root.join("runtimes").join(&entry);
    // The entry's own cached release index flows the asset spellings
    // verbatim when it names this identity (spec 05 §2 SSOT; tebako#456
    // — the factory publishes windows exe assets SUFFIX-LESS); the
    // pre-identity fallback synthesizes `{name}.exe` / `{name}.tfs`.
    let (asset, image_asset) =
        entry_asset_names(&entry_dir, engine, &pref.version, &pref.tebako, platform);
    let exe_path = entry_dir.join(&asset);

    if file_exists(&exe_path) {
        // Raced with another installer; use the cache.
        return Ok(CachedRuntime {
            engine: engine.to_string(),
            lang_version: pref.version.clone(),
            tebako_version: pref.tebako.clone(),
            dir: entry_dir.clone(),
            exe: exe_path,
            image: entry_dir
                .join(&image_asset)
                .is_file()
                .then(|| entry_dir.join(&image_asset)),
            abi: entry_meta(&entry_dir, &asset, "abi"),
            implementation: entry_meta(&entry_dir, &asset, "implementation"),
            language_version: entry_meta(&entry_dir, &asset, "language_version"),
        });
    }

    let base = skip_file_scheme(&source.base).to_string();
    let local = base_is_local(&source.base);
    // The release tag every fetch of this download rides (spec 05 §2):
    // channel 3 pins the registry-named tag; the other channels ride the
    // pick's own tebako line (the factory's `v<tebako>` convention).
    let tag = source.tag_for(&pref.tebako);
    let dir_url = format!("{base}/{tag}");

    if offline_mode(ctx) {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "cannot resolve runtime \"{runtime_ref}\": not present in the cache and TEBAKO_OFFLINE is set\n  cache entry: {}\n  unset TEBAKO_OFFLINE, or set TEBAKO_RUNTIME_MIRROR to a reachable mirror",
                entry_dir.display()
            ),
        );
    }

    let locks = root.join("locks");
    for dir in [&locks, &root.join("tmp"), &root.join("runtimes")] {
        std::fs::create_dir_all(dir).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!(
                    "cannot create tebako cache directories under {}: {e}",
                    dir.display()
                ),
            )
        })?;
    }
    let lock_path = locks.join(format!("{entry}.lock"));
    let lock = flock_acquire(&lock_path, LOCK_TIMEOUT_MS).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut {
            ShimError::new(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "timed out after {}s waiting for another tebako process to finish installing \"{runtime_ref}\"\n  lock: {}\n  if no other tebako process is running, remove the stale lock file",
                    LOCK_TIMEOUT_MS / 1000,
                    lock_path.display()
                ),
            )
        } else {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot acquire install lock {}: {e}", lock_path.display()),
            )
        }
    })?;

    // re-check under the lock: another process may have installed it.
    if file_exists(&exe_path) {
        lock_release(lock);
        return Ok(CachedRuntime {
            engine: engine.to_string(),
            lang_version: pref.version.clone(),
            tebako_version: pref.tebako.clone(),
            dir: entry_dir.clone(),
            exe: exe_path,
            image: entry_dir
                .join(&image_asset)
                .is_file()
                .then(|| entry_dir.join(&image_asset)),
            abi: entry_meta(&entry_dir, &asset, "abi"),
            implementation: entry_meta(&entry_dir, &asset, "implementation"),
            language_version: entry_meta(&entry_dir, &asset, "language_version"),
        });
    }

    let tmp_dir = root
        .join("tmp")
        .join(format!("{entry}.{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let result = std::fs::create_dir(&tmp_dir).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", tmp_dir.display()),
        )
    });
    let fallback_asset = asset.clone();
    let fallback_image = image_asset.clone();
    let result: Result<StageOutcome, ShimError> = result.and_then(|()| {
        // spec 18 C2: the release card gates BEFORE any asset download —
        // a contract refusal never downloads a byte of the runtime. The
        // card's acquisition is spec 05 §2's shard-first order (roadmap
        // 85): the per-package shard `<stem>.manifest.json`, then the
        // manifest.json monolith — the trust scan (G1, spec 09 §4)
        // verifies each form's detached asc BEFORE its bytes are
        // consumed, and no readable card form is the pre-era signal (75)
        // naming every URL tried.
        let stem = format!("tebako-runtime-{}-{}-{platform}", pref.tebako, pref.version);
        let trust = FetchTrust::build(source, ctx)?;
        let acquired = acquire_index(
            &trust,
            &base,
            local,
            &tag,
            &stem,
            engine,
            &pref.version,
            &pref.tebako,
            platform,
            &runtime_ref,
            &tmp_dir,
        )?;
        let from_shard = acquired.from_shard;
        let index_trust = acquired.trust;
        let manifest_text = acquired.text;
        if from_shard {
            // The store's cached card keeps the array shape every card
            // reader speaks (entry_asset_names / entry_meta / the
            // contract re-reads parse manifest.json as an array); the
            // raw shard file is not part of the store entry.
            let _ = std::fs::write(tmp_dir.join("manifest.json"), &manifest_text);
            let _ = std::fs::remove_file(tmp_dir.join(format!("{stem}.manifest.json")));
        }
        // The entry's `filename` is the ONLY authoritative asset
        // spelling (spec 05 §2 SSOT; tebako#456): match the identity
        // triple (`tebako_version` + `{engine}_version` + `platform`)
        // and flow `filename` / `image.filename` verbatim; the
        // pre-identity fallback keeps the synthesized spelling.
        let parsed = tebako_json::parse(&manifest_text).ok();
        let entry_match = parsed
            .as_ref()
            .and_then(|m| release_index_entry(m, engine, &pref.version, &pref.tebako, platform));
        let asset = entry_match
            .and_then(|e| entry_filename(e, None))
            .unwrap_or(fallback_asset);
        let image_asset = entry_match
            .and_then(|e| entry_filename(e, Some("image")))
            .unwrap_or(fallback_image);
        // Late re-check under the flowed spelling: another installer may
        // have published the entry while the index was being fetched.
        if file_exists(&entry_dir.join(&asset)) {
            return Ok(StageOutcome::Raced { asset, image_asset });
        }

        // The declared per-artifact signatures (spec 13 §2a): a PRESENT
        // but torn block is the named error — never a silent downgrade
        // to the unsigned rule.
        let declared_sig = |facet: Option<&str>| -> Result<Option<EntrySignature>, ShimError> {
            entry_match
                .map(|e| entry_signature(e, facet))
                .transpose()
                .map(Option::flatten)
                .map_err(|m| {
                    ShimError::new(
                        EX_TEBAKO_MANIFEST,
                        format!("runtime \"{runtime_ref}\": the release index's {m}"),
                    )
                })
        };
        let exe_sig = declared_sig(None)?;
        let image_sig = declared_sig(Some("image"))?;
        let dll_sig = declared_sig(Some("dll"))?;

        // The unsigned rule (spec 09 §4): no verified index form AND no
        // declared per-artifact signature — the pre-signing keep-forever
        // line. Loud + journaled on EVERY fetch; refused (71) under
        // TEBAKO_REQUIRE_SIGNED=1 — before a byte of the runtime moves.
        let any_declared = exe_sig.is_some() || image_sig.is_some() || dll_sig.is_some();
        if index_trust.signer().is_none() && !any_declared {
            if require_signed(ctx) {
                return fail(
                    EX_TEBAKO_SIGNATURE,
                    format!(
                        "runtime \"{runtime_ref}\" is unsigned — no signed release index form and no declared per-artifact signature — and TEBAKO_REQUIRE_SIGNED=1 is set: refusing to install or execute\n  unset TEBAKO_REQUIRE_SIGNED to accept the unsigned fetch (it is journaled), or pin a signed runtime release"
                    ),
                );
            }
            eprintln!(
                "tebako-shim: warning: runtime \"{runtime_ref}\" is unsigned — no verifiable signature on the release; the sha256 digest is the only integrity anchor (spec 09 §4's pre-signing keep-forever line)"
            );
            journal(
                &ctx.home,
                &format!(
                    "event=unsigned-runtime-fetch runtime_ref={runtime_ref} base={} channel={}",
                    source.base, source.channel
                ),
            );
        }

        // The contract card rides the CONSUMED index form: a sums-only
        // verified fetch has no trusted card carrier — the spec 18 C2
        // refusal (75), never a card read from an unverified manifest.
        if matches!(index_trust, IndexTrust::VerifiedSums { .. }) {
            return fail(
                EX_TEBAKO_CONTRACT,
                format!(
                    "runtime \"{runtime_ref}\": only SHA256SUMS.txt verified — manifest.json is unsigned, so no trusted release contract card exists — refusing to install or execute (spec 09 §4 + spec 18 C2)\n  a spec-conformant factory signs every index form (spec 13 §2a); report the release"
                ),
            );
        }
        contract_gate(
            &runtime_ref,
            &manifest_text,
            &asset,
            engine,
            &pref.version,
            &pref.tebako,
            platform,
        )?;

        // executable
        let exe = expected_checksum(&ChecksumQuery {
            base: &base,
            local,
            tag: &tag,
            asset: &asset,
            tmp_dir: &tmp_dir,
            index_text: &manifest_text,
            from_shard,
            trust: &index_trust,
        })?;
        let (diag_sh, diag_m, diag_s) = exe.diags;
        const DIAG: [&str; 5] = [
            "not tried",
            "download failed",
            "read failed",
            "no matching entry",
            "ok",
        ];
        let expected = exe.sha.ok_or_else(|| {
            ShimError::new(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "no checksum for {asset} in the release\n  tried: {dir_url}/{stem}.manifest.json ({})\n         {dir_url}/manifest.json ({})\n         {dir_url}/SHA256SUMS.txt ({})",
                    DIAG[diag_sh], DIAG[diag_m], DIAG[diag_s]
                ),
            )
        })?;
        let mut signers: Vec<String> = index_trust
            .signer()
            .map(str::to_string)
            .into_iter()
            .collect();
        let (actual, exe_signer) = install_asset(
            &dir_url,
            local,
            &asset,
            &asset,
            &tmp_dir,
            &expected,
            exe_sig.as_ref().map(|s| (s, &trust)),
        )?;
        signers.extend(exe_signer);
        make_executable(&tmp_dir.join(&asset));
        // image-era runtime image: same mirror/offline/verify rules —
        // when the release index carries it. The image is optional only
        // in an otherwise contract-complete release (an entry with the
        // contract set but no `image` key: the exe's embedded image
        // serves and it installs alone — the s14 sums-fallback shape).
        // The contract gate is the same one (the exe entry governs its
        // additive image too).
        contract_gate(
            &runtime_ref,
            &manifest_text,
            &asset,
            engine,
            &pref.version,
            &pref.tebako,
            platform,
        )?;
        let image = expected_checksum(&ChecksumQuery {
            base: &base,
            local,
            tag: &tag,
            asset: &image_asset,
            tmp_dir: &tmp_dir,
            index_text: &manifest_text,
            from_shard,
            trust: &index_trust,
        })?;
        let has_image = if let Some(image_expected) = image.sha {
            let (image_actual, image_signer) = install_asset(
                &dir_url,
                local,
                &image_asset,
                &image_asset,
                &tmp_dir,
                &image_expected,
                image_sig.as_ref().map(|s| (s, &trust)),
            )?;
            signers.extend(image_signer);
            make_readonly(&tmp_dir.join(&image_asset));
            let _ = std::fs::write(
                tmp_dir.join(format!("{image_asset}.sha256")),
                format!("{image_actual}  {image_asset}\n"),
            );
            let _ = std::fs::write(
                tmp_dir.join(format!("{image_asset}.origin")),
                format!("runtime_ref={runtime_ref}\nurl={dir_url}/{image_asset}\nsha256={image_actual}\n"),
            );
            true
        } else {
            false
        };
        // windows dll-era runtimes (tebako-runtime-ruby#40): the exe
        // imports the ruby core DLL — the release manifest's additive
        // `dll` key names the asset and the PE name (`install_as`) it
        // installs under next to the exe (never the asset name: assets
        // are unique per leg, two same-ABI legs share the PE name). Same
        // mirror/offline/verify rules as the image, the same contract
        // gate (the exe entry governs its additive facets); a
        // contract-complete entry with no `dll` key installs the exe
        // alone (every POSIX release).
        if let Some(dll_facet) = entry_dll_from_index(&manifest_text, &asset) {
            let tpkg::runtime_store::EntryDll {
                filename: dll_asset,
                install_as,
                sha256: dll_expected,
            } = dll_facet;
            if install_as.contains('/') || install_as.contains('\\') {
                return fail(
                    EX_TEBAKO_UNAVAILABLE,
                    format!(
                        "release manifest dll facet for {asset} carries an unusable install_as (\"{install_as}\") — the PE name must be a bare file name — refusing to install or execute"
                    ),
                );
            }
            let (dll_actual, dll_signer) = install_asset(
                &dir_url,
                local,
                &dll_asset,
                &install_as,
                &tmp_dir,
                &dll_expected,
                dll_sig.as_ref().map(|s| (s, &trust)),
            )?;
            signers.extend(dll_signer);
            make_readonly(&tmp_dir.join(&install_as));
            let _ = std::fs::write(
                tmp_dir.join(format!("{install_as}.sha256")),
                format!("{dll_actual}  {install_as}\n"),
            );
            let _ = std::fs::write(
                tmp_dir.join(format!("{install_as}.origin")),
                format!("runtime_ref={runtime_ref}\nurl={dir_url}/{dll_asset}\nsha256={dll_actual}\n"),
            );
        }
        let _ = std::fs::write(tmp_dir.join("sha256"), format!("{actual}  {asset}\n"));
        let _ = std::fs::write(
            tmp_dir.join("origin"),
            format!("runtime_ref={runtime_ref}\nurl={dir_url}/{asset}\nsha256={actual}\n"),
        );
        // spec 05 §2's journal rule: every download records the base, the
        // channel that supplied it, and the verification strength.
        if !signers.is_empty() {
            signers.sort();
            signers.dedup();
            journal(
                &ctx.home,
                &format!(
                    "event=runtime-fetch-verified runtime_ref={runtime_ref} base={} channel={} signer={}",
                    source.base,
                    source.channel,
                    signers.join(",")
                ),
            );
        }
        Ok(StageOutcome::Installed {
            has_image,
            asset,
            image_asset,
        })
    });

    match result {
        Ok(StageOutcome::Raced { asset, image_asset }) => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            lock_release(lock);
            Ok(CachedRuntime {
                engine: engine.to_string(),
                lang_version: pref.version.clone(),
                tebako_version: pref.tebako.clone(),
                exe: entry_dir.join(&asset),
                image: entry_dir
                    .join(&image_asset)
                    .is_file()
                    .then(|| entry_dir.join(&image_asset)),
                abi: entry_meta(&entry_dir, &asset, "abi"),
                implementation: entry_meta(&entry_dir, &asset, "implementation"),
                language_version: entry_meta(&entry_dir, &asset, "language_version"),
                dir: entry_dir,
            })
        }
        Ok(StageOutcome::Installed {
            has_image,
            asset,
            image_asset,
        }) => {
            if file_exists(&entry_dir) {
                let _ = std::fs::remove_dir_all(&tmp_dir);
                lock_release(lock);
                return fail(
                    EX_TEBAKO_IO,
                    format!(
                        "cache entry {} exists but is incomplete (missing {asset})\n  remove that directory and run again",
                        entry_dir.display()
                    ),
                );
            }
            if let Err(e) = std::fs::rename(&tmp_dir, &entry_dir) {
                let _ = std::fs::remove_dir_all(&tmp_dir);
                lock_release(lock);
                return fail(
                    EX_TEBAKO_IO,
                    format!(
                        "cannot install runtime into the cache ({} -> {}): {e}",
                        tmp_dir.display(),
                        entry_dir.display()
                    ),
                );
            }
            lock_release(lock);
            Ok(CachedRuntime {
                engine: engine.to_string(),
                lang_version: pref.version.clone(),
                tebako_version: pref.tebako.clone(),
                exe: entry_dir.join(&asset),
                image: has_image.then(|| entry_dir.join(&image_asset)),
                abi: entry_meta(&entry_dir, &asset, "abi"),
                implementation: entry_meta(&entry_dir, &asset, "implementation"),
                language_version: entry_meta(&entry_dir, &asset, "language_version"),
                dir: entry_dir,
            })
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            lock_release(lock);
            Err(e)
        }
    }
}

// ---------------------------------------------------------------------
// unit tests: the per-engine download-source chain (spec 05 §2, #567)
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-shim-runtime-test-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_ctx(home: &Path) -> Ctx {
        Ctx {
            home: home.to_path_buf(),
            cwd: home.to_path_buf(),
            env: std::collections::BTreeMap::new(),
        }
    }

    fn reqs(engine: &str, constraint: &str) -> RuntimeRequirements {
        RuntimeRequirements::one(RuntimeRequirement {
            engine: engine.to_string(),
            constraint: tpkg::Constraint::new(constraint).unwrap(),
            implementation: None,
            abi: None,
        })
    }

    fn pref(version: &str, tebako: &str, source: Option<&str>) -> RuntimePref {
        RuntimePref {
            version: version.to_string(),
            tebako: tebako.to_string(),
            source: source.map(str::to_string),
        }
    }

    /// Write a registry file and return its `file://` ref.
    fn registry_ref(home: &Path, name: &str, yaml: &str) -> String {
        let path = home.join(name);
        std::fs::write(&path, yaml).unwrap();
        tebako_http::file_url(&path)
    }

    const OPENJDK_REGISTRY: &str = r#"
schema_version: 1
payloads:
  - name: tebako-runtime-openjdk
    kind: runtime
    engine: java
    implementation: temurin
    versions:
      - version: '21.0.11'
        platforms: universal
        release: {ref: 'tfs:github:tamatebako/tebako-runtime-openjdk:v2.5.0'}
      - version: '21.0.12'
        platforms: universal
        release: {ref: 'tfs:github:tamatebako/tebako-runtime-openjdk:v2.5.1'}
        signature: {keyid: 'efc3c250f7862a48', asc: 'openjdk-21.0.12-universal.tfs.asc'}
"#;

    #[test]
    fn config_source_wins_and_shadows_a_differing_mirror() {
        let home = temp_home("chain-source");
        let mut ctx = test_ctx(&home);
        ctx.env.insert(
            "TEBAKO_RUNTIME_MIRROR".to_string(),
            "https://mirror.invalid/releases".to_string(),
        );
        let p = pref(
            "21.0.12",
            "2.5.0",
            Some("https://pinned.example.com/releases"),
        );
        let cfg = UserConfig::default();
        let source = runtime_source(&reqs("java", ">= 21"), Some(&p), &cfg, &ctx).unwrap();
        assert_eq!(source.base, "https://pinned.example.com/releases");
        assert_eq!(source.channel, "config-source");
        assert_eq!(source.tag, None, "a bare base pin rides the tebako line");
        let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
        assert!(
            journal.contains("event=runtime-source-shadow engine=java"),
            "{journal}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_mirror_env_is_channel_2() {
        let home = temp_home("chain-mirror");
        let mut ctx = test_ctx(&home);
        ctx.env.insert(
            "TEBAKO_RUNTIME_MIRROR".to_string(),
            "https://mirror.invalid/releases".to_string(),
        );
        let p = pref("21.0.12", "2.5.0", None);
        let cfg = UserConfig::default();
        let source = runtime_source(&reqs("java", ">= 21"), Some(&p), &cfg, &ctx).unwrap();
        assert_eq!(source.base, "https://mirror.invalid/releases");
        assert_eq!(source.channel, "mirror-env");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_registry_channel_derives_base_tag_and_pin() {
        // The golden openjdk v2.5.1 shape: the registry version's
        // release.ref names the TAG (v2.5.1) while the rows ride their
        // own tebako line (2.5.0) — the tag is decoupled from the
        // identity line.
        let home = temp_home("chain-registry");
        let ctx = test_ctx(&home);
        let reg = registry_ref(&home, "tpkg-registry.yaml", OPENJDK_REGISTRY);
        let cfg = UserConfig {
            registries: vec![reg],
            ..UserConfig::default()
        };
        let source = runtime_source(&reqs("java", ">= 21"), None, &cfg, &ctx).unwrap();
        assert_eq!(source.channel, "registry");
        assert_eq!(
            source.base,
            "https://github.com/tamatebako/tebako-runtime-openjdk/releases/download"
        );
        assert_eq!(
            source.tag.as_deref(),
            Some("v2.5.1"),
            "the newest SATISFYING version's tag"
        );
        assert_eq!(source.signer_pin.as_deref(), Some("efc3c250f7862a48"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_registry_channel_matches_the_constraint_and_the_implementation() {
        let home = temp_home("chain-registry-match");
        let ctx = test_ctx(&home);
        let reg = registry_ref(&home, "tpkg-registry.yaml", OPENJDK_REGISTRY);
        let cfg = UserConfig {
            registries: vec![reg],
            ..UserConfig::default()
        };
        // A constraint no registry version satisfies: the channel does
        // not answer (the named no-channel error for a non-ruby engine).
        let err = runtime_source(&reqs("java", ">= 25"), None, &cfg, &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        assert!(err.message.contains("no download source"), "{err:?}");
        // An implementation the registry entry does not carry: same.
        let req = RuntimeRequirements::one(RuntimeRequirement {
            engine: "java".to_string(),
            constraint: tpkg::Constraint::new(">= 21").unwrap(),
            implementation: Some("graalvm".to_string()),
            abi: None,
        });
        let err = runtime_source(&req, None, &cfg, &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_engine_less_runtime_entry_stays_invisible_to_the_chain() {
        // The openjdk golden gap: a `kind: runtime` entry without the
        // payload-level `engine:` key is pre-discovery legacy — resolvable
        // by name, never an edge answer.
        let home = temp_home("chain-engineless");
        let ctx = test_ctx(&home);
        let reg = registry_ref(
            &home,
            "tpkg-registry.yaml",
            r#"
schema_version: 1
payloads:
  - name: tebako-runtime-openjdk
    kind: runtime
    versions:
      - version: '21.0.12'
        platforms: universal
        release: {ref: 'tfs:github:tamatebako/tebako-runtime-openjdk:v2.5.1'}
"#,
        );
        let cfg = UserConfig {
            registries: vec![reg],
            ..UserConfig::default()
        };
        let err = runtime_source(&reqs("java", ">= 21"), None, &cfg, &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        assert!(err.message.contains("kind: runtime"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_non_github_release_ref_is_journaled_and_skipped() {
        let home = temp_home("chain-nongithub");
        let ctx = test_ctx(&home);
        let reg = registry_ref(
            &home,
            "tpkg-registry.yaml",
            r#"
schema_version: 1
payloads:
  - name: tebako-runtime-python
    kind: runtime
    engine: python
    versions:
      - version: '3.13.5'
        platforms: universal
        release: {ref: 'tfs:gitlab:acme/tebako-runtime-python:v0.3.0'}
"#,
        );
        let cfg = UserConfig {
            registries: vec![reg],
            ..UserConfig::default()
        };
        let err = runtime_source(&reqs("python", ">= 3.13"), None, &cfg, &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
        assert!(
            journal.contains("event=runtime-source-registry-skip engine=python"),
            "{journal}"
        );
        assert!(journal.contains("gitlab"), "{journal}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_default_channel_serves_ruby_only() {
        let home = temp_home("chain-default");
        let ctx = test_ctx(&home);
        let cfg = UserConfig::default();
        let source = runtime_source(&reqs("ruby", ">= 3.3"), None, &cfg, &ctx).unwrap();
        assert_eq!(source.channel, "default");
        assert_eq!(source.base, DEFAULT_RELEASES_BASE);
        assert_eq!(source.tag, None);
        let err = runtime_source(&reqs("java", ">= 21"), None, &cfg, &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        for channel in [
            "source:",
            "TEBAKO_RUNTIME_MIRROR",
            "kind: runtime",
            "hosts ruby runtimes only",
        ] {
            assert!(err.message.contains(channel), "{channel} — {err:?}");
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn tag_for_rides_the_line_unless_pinned() {
        let unpinned = RuntimeSource {
            base: "https://x".to_string(),
            tag: None,
            channel: "default",
            signer_pin: None,
        };
        assert_eq!(unpinned.tag_for("0.16.22"), "v0.16.22");
        let pinned = RuntimeSource {
            base: "https://x".to_string(),
            tag: Some("v2.5.1".to_string()),
            channel: "registry",
            signer_pin: None,
        };
        assert_eq!(pinned.tag_for("2.5.0"), "v2.5.1");
    }

    // ---- the registry facet of the index selection (spec 05 §2, roadmap 85) ----

    fn sha64(c: char) -> String {
        c.to_string().repeat(64)
    }

    /// A shard-era factory registry (the openjdk shape): two versions
    /// riding tebako line 2.5.0, the newer published under release tag
    /// v2.5.1 (the tag is NOT the rows' tebako line), host-covered
    /// per-triplet rows; `withdraw_newer` yanks 21.0.12.
    fn shard_era_registry(withdraw_newer: bool) -> String {
        let host = tpkg::Platform::host();
        let triplet = host.as_triplet();
        let asset = host.release_asset_name();
        let status = if withdraw_newer {
            "        status: withdrawn\n"
        } else {
            ""
        };
        format!(
            "schema_version: 1\npayloads:\n  - name: tebako-runtime-openjdk\n    kind: runtime\n    engine: java\n    implementation: temurin\n    versions:\n      - version: '21.0.11'\n        platforms:\n          {triplet}: {{artifact: tebako-runtime-2.5.0-21.0.11-{asset}.tfs, sha256: '{}'}}\n        release: {{ref: 'tfs:github:acme/tebako-runtime-openjdk:v2.5.0'}}\n      - version: '21.0.12'\n{status}        platforms:\n          {triplet}: {{artifact: tebako-runtime-2.5.0-21.0.12-{asset}.tfs, sha256: '{}'}}\n        release: {{ref: 'tfs:github:acme/tebako-runtime-openjdk:v2.5.1'}}\n        signature: {{keyid: 'efc3c250f7862a48', asc: 'tebako-runtime-2.5.0-21.0.12-{asset}.tfs.asc'}}\n",
            sha64('a'),
            sha64('b'),
        )
    }

    fn github_source() -> RuntimeSource {
        RuntimeSource {
            base: "https://github.com/acme/tebako-runtime-openjdk/releases/download".to_string(),
            tag: Some("v2.5.1".to_string()),
            channel: "registry",
            signer_pin: None,
        }
    }

    #[test]
    fn github_owner_repo_parses_only_the_release_download_base() {
        assert_eq!(
            github_owner_repo("https://github.com/acme/tebako-runtime-openjdk/releases/download"),
            Some(("acme".to_string(), "tebako-runtime-openjdk".to_string()))
        );
        assert_eq!(github_owner_repo("https://mirror.invalid/releases"), None);
        assert_eq!(github_owner_repo("https://github.com/acme/repo"), None);
        assert_eq!(
            github_owner_repo("https://github.com/a/b/c/releases/download"),
            None
        );
        assert_eq!(github_owner_repo("file:///tmp/mirror"), None);
    }

    #[test]
    fn tebako_line_from_artifact_reads_the_stem() {
        let host = tpkg::Platform::host();
        let asset = host.release_asset_name();
        assert_eq!(
            tebako_line_from_artifact(
                &format!("tebako-runtime-2.5.0-21.0.12-{asset}.tfs"),
                "21.0.12",
                host
            ),
            Some("2.5.0".to_string())
        );
        // a non-factory spelling carries no line
        assert_eq!(
            tebako_line_from_artifact("openjdk-21.tar.gz", "21.0.12", host),
            None
        );
    }

    #[test]
    fn the_registry_facet_picks_the_newest_satisfying_row() {
        let home = temp_home("regfacet-pick");
        let ctx = test_ctx(&home);
        crate::regcache::prime(
            &home,
            "tfs:github:acme/tebako-runtime-openjdk",
            shard_era_registry(false).as_bytes(),
        )
        .unwrap();
        let (pref, source) =
            registry_selected_target(&reqs("java", ">= 21"), &github_source(), &ctx)
                .unwrap()
                .expect("an informative registry picks");
        assert_eq!(pref.version, "21.0.12");
        // the tebako line flows from the artifact stem — NOT the tag
        // (the openjdk v2.5.1 shape: tag v2.5.1, rows on tebako 2.5.0)
        assert_eq!(pref.tebako, "2.5.0");
        assert_eq!(
            source.base,
            "https://github.com/acme/tebako-runtime-openjdk/releases/download"
        );
        assert_eq!(source.tag.as_deref(), Some("v2.5.1"));
        assert_eq!(source.signer_pin.as_deref(), Some("efc3c250f7862a48"));
        assert_eq!(source.channel, "registry");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn split_line_id_parses_only_the_witnessed_composite() {
        let host = tpkg::Platform::host();
        let asset = host.release_asset_name();
        // the ruby factory shape: composite row version, artifact stem
        // witnesses the (lang, tebako) split
        assert_eq!(
            split_line_id(
                &format!("tebako-runtime-0.16.23-4.0.6-{asset}.tfs"),
                "4.0.6-0.16.23",
                host
            ),
            Some(("4.0.6".to_string(), "0.16.23".to_string()))
        );
        // a bare row version never splits (the openjdk shape)
        assert_eq!(
            split_line_id(
                &format!("tebako-runtime-2.5.0-21.0.12-{asset}.tfs"),
                "21.0.12",
                host
            ),
            None
        );
        // a composite claim the artifact does not witness never splits
        assert_eq!(
            split_line_id(
                &format!("tebako-runtime-0.16.23-4.0.6-{asset}.tfs"),
                "4.0.6-0.16.22",
                host
            ),
            None
        );
        // a single-dash version splits uniquely when the stem witnesses it
        assert_eq!(
            split_line_id(&format!("tebako-runtime-2-1-{asset}.tfs"), "1-2", host),
            Some(("1".to_string(), "2".to_string()))
        );
    }

    /// The ruby factory's registry shape (composite line-id rows) — the
    /// pick must name the BARE language version in the pref, or the
    /// download recomposes a double-suffixed asset spelling and the
    /// contract gate answers pre-era (hello-runtimes dogfood, 2026-09-14).
    fn line_id_registry() -> String {
        let host = tpkg::Platform::host();
        let triplet = host.as_triplet();
        let asset = host.release_asset_name();
        format!(
            "schema_version: 1\npayloads:\n  - name: ruby\n    kind: runtime\n    engine: ruby\n    versions:\n      - version: '3.3.12-0.16.23'\n        platforms:\n          {triplet}: {{artifact: tebako-runtime-0.16.23-3.3.12-{asset}, sha256: '{}'}}\n        release: {{ref: 'tfs:github:acme/tebako-runtime-ruby:v0.16.23'}}\n      - version: '4.0.6-0.16.23'\n        platforms:\n          {triplet}: {{artifact: tebako-runtime-0.16.23-4.0.6-{asset}, sha256: '{}'}}\n        release: {{ref: 'tfs:github:acme/tebako-runtime-ruby:v0.16.23'}}\n",
            sha64('c'),
            sha64('d'),
        )
    }

    #[test]
    fn the_registry_facet_splits_a_composite_line_id_row() {
        let home = temp_home("regfacet-lineid");
        let ctx = test_ctx(&home);
        crate::regcache::prime(
            &home,
            "tfs:github:acme/tebako-runtime-ruby",
            line_id_registry().as_bytes(),
        )
        .unwrap();
        let source = RuntimeSource {
            base: "https://github.com/acme/tebako-runtime-ruby/releases/download".to_string(),
            tag: None,
            channel: "default",
            signer_pin: None,
        };
        let (pref, source) =
            registry_selected_target(&reqs("ruby", ">= 3.3, < 5.0"), &source, &ctx)
                .unwrap()
                .expect("an informative registry picks");
        // the bare language version + the line — the download composes
        // tebako-runtime-0.16.23-4.0.6-<platform> from these
        assert_eq!(pref.version, "4.0.6");
        assert_eq!(pref.tebako, "0.16.23");
        assert_eq!(source.tag.as_deref(), Some("v0.16.23"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_registry_facet_matches_composite_rows_against_the_constraint() {
        // the constraint math rides the COMPOSITE row key: ">= 4.0"
        // satisfies 4.0.6-0.16.23 but not 3.3.12-0.16.23
        let home = temp_home("regfacet-lineid-range");
        let ctx = test_ctx(&home);
        crate::regcache::prime(
            &home,
            "tfs:github:acme/tebako-runtime-ruby",
            line_id_registry().as_bytes(),
        )
        .unwrap();
        let source = RuntimeSource {
            base: "https://github.com/acme/tebako-runtime-ruby/releases/download".to_string(),
            tag: None,
            channel: "default",
            signer_pin: None,
        };
        let (pref, _) = registry_selected_target(&reqs("ruby", ">= 4.0"), &source, &ctx)
            .unwrap()
            .expect("4.0.6-0.16.23 satisfies >= 4.0");
        assert_eq!(pref.version, "4.0.6");
        let err = registry_selected_target(&reqs("ruby", ">= 5.0"), &source, &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        assert!(err.message.contains("4.0.6-0.16.23"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_registry_facet_refuses_a_withdrawn_pick_by_name() {
        let home = temp_home("regfacet-withdrawn");
        let ctx = test_ctx(&home);
        crate::regcache::prime(
            &home,
            "tfs:github:acme/tebako-runtime-openjdk",
            shard_era_registry(true).as_bytes(),
        )
        .unwrap();
        let err =
            registry_selected_target(&reqs("java", ">= 21"), &github_source(), &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        assert!(err.message.contains("WithdrawnPayload"), "{err:?}");
        assert!(err.message.contains("21.0.12"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_withdrawn_non_selected_row_is_inert() {
        let home = temp_home("regfacet-inert");
        let ctx = test_ctx(&home);
        crate::regcache::prime(
            &home,
            "tfs:github:acme/tebako-runtime-openjdk",
            shard_era_registry(true).as_bytes(),
        )
        .unwrap();
        // the constraint excludes the withdrawn 21.0.12 — the healthy
        // 21.0.11 picks without a murmur
        let (pref, _) =
            registry_selected_target(&reqs("java", ">= 21, < 21.0.12"), &github_source(), &ctx)
                .unwrap()
                .expect("the healthy row picks");
        assert_eq!(pref.version, "21.0.11");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_registry_facet_availability_error_marks_withdrawn_rows() {
        let home = temp_home("regfacet-avail");
        let ctx = test_ctx(&home);
        crate::regcache::prime(
            &home,
            "tfs:github:acme/tebako-runtime-openjdk",
            shard_era_registry(true).as_bytes(),
        )
        .unwrap();
        let err =
            registry_selected_target(&reqs("java", ">= 25"), &github_source(), &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        let platform = platform_string();
        assert!(
            err.message.contains(&format!(
                "no released java runtime for {platform} satisfies"
            )),
            "{err:?}"
        );
        assert!(
            err.message.contains(&format!(
                "released for {platform}: 21.0.11, 21.0.12 (withdrawn)"
            )),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_registry_facet_is_uninformative_without_runtime_entries() {
        let home = temp_home("regfacet-empty");
        let ctx = test_ctx(&home);
        crate::regcache::prime(
            &home,
            "tfs:github:acme/tebako-runtime-openjdk",
            b"schema_version: 1\npayloads: []\n",
        )
        .unwrap();
        assert!(
            registry_selected_target(&reqs("java", ">= 21"), &github_source(), &ctx)
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_registry_facet_ignores_a_non_github_base() {
        let home = temp_home("regfacet-nongh");
        let ctx = test_ctx(&home);
        let source = RuntimeSource {
            base: "https://mirror.invalid/releases".to_string(),
            tag: None,
            channel: "mirror-env",
            signer_pin: None,
        };
        assert!(
            registry_selected_target(&reqs("java", ">= 21"), &source, &ctx)
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_unreadable_registry_is_journaled_and_uninformative() {
        let home = temp_home("regfacet-unreadable");
        let mut ctx = test_ctx(&home);
        // nothing primed + offline: the registry cannot read
        ctx.env
            .insert("TEBAKO_OFFLINE".to_string(), "1".to_string());
        assert!(
            registry_selected_target(&reqs("java", ">= 21"), &github_source(), &ctx)
                .unwrap()
                .is_none()
        );
        let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
        assert!(
            journal.contains("event=runtime-index-registry-error engine=java"),
            "{journal}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn channel_3_refuses_a_withdrawn_pick_by_name() {
        let home = temp_home("chain-registry-withdrawn");
        let ctx = test_ctx(&home);
        let reg = registry_ref(&home, "tpkg-registry.yaml", &shard_era_registry(true));
        let cfg = UserConfig {
            registries: vec![reg],
            ..UserConfig::default()
        };
        let err = runtime_source(&reqs("java", ">= 21"), None, &cfg, &ctx).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        assert!(err.message.contains("WithdrawnPayload"), "{err:?}");
        assert!(err.message.contains("21.0.12"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }
}
