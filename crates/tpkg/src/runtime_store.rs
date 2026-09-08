//! The runtime store's entry grammar and cache scan (spec 05 §3) — the
//! SINGLE owner of both (spec 00 §10; spec 30 §1's spawned-runtime edge
//! made the shim's private copy a shared contract: tebako-shim resolves
//! and downloads through it, tebako-driver's spawn interception scans
//! cache-only through it, tebako-cli installs through the shim).
//!
//! The store entry is
//! `runtimes/<engine>-<lv>-<ver>-<triplet>/tebako-runtime-<ver>-<lv>-<triplet>[.exe]`
//! plus the image-era env image and the trust markers; the grammar and
//! the asset-name flow (the cached release index's verbatim `filename`
//! spellings — spec 05 §2, tebako#456's suffix-less windows exes — else
//! the synthesized fallback) live here and nowhere else.
//!
//! Download, the release-index consultation, the contract gate and the
//! trust markers' WRITE side stay in tebako-shim's `runtime` module —
//! this module is the READ side every consumer shares (pure fs + the
//! cached index mirror; no network, no unsafe).

use std::path::{Path, PathBuf};

use crate::versions::{self, Constraint};

/// Runtime-package platform string for asset-name construction.
/// [`crate::Platform`] owns the vocabulary and host detection (spec 03
/// §3); this is the `&'static str` convenience over it.
pub fn platform_string() -> &'static str {
    crate::Platform::host().release_asset_name()
}

pub fn exe_suffix() -> &'static str {
    #[cfg(windows)]
    return ".exe";
    #[cfg(not(windows))]
    return "";
}

// ---------------------------------------------------------------------
// the machine cache scan (spec 05 §3)
// ---------------------------------------------------------------------

/// A cached runtime entry
/// `runtimes/<lang>-<lv>-<ver>-<triplet>/tebako-runtime-<ver>-<lv>-<triplet>[.exe]`.
#[derive(Debug, Clone)]
pub struct CachedRuntime {
    pub engine: String,
    /// Language version (`<lv>`), e.g. `4.0.6`.
    pub lang_version: String,
    /// Tebako (launcher abi) version (`<ver>`), e.g. `0.16.0`.
    pub tebako_version: String,
    pub dir: PathBuf,
    pub exe: PathBuf,
    /// The image-era runtime image, present iff both the `.tfs` and its
    /// `.sha256` trust marker are cached.
    pub image: Option<PathBuf>,
    /// The runtime's own platform string (ruby: `Gem::Platform.local` —
    /// from the release index's `abi` key); `None` for releases that
    /// predate the field (the compat window — eligible, never a match
    /// failure of its own).
    pub abi: Option<String>,
    /// The runtime's implementation (spec 28 §8 — `mri`/`jruby`,
    /// `temurin` for a java engine) from the release index's
    /// `implementation` key; `None` for releases that predate the field
    /// (the same compat-window rule as `abi`: eligible, never a match
    /// failure of its own).
    pub implementation: Option<String>,
    /// The language level this build implements (spec 28 §8 — jruby 9.4 →
    /// `"3.1"`) from the release index's `language_version` key; `None`
    /// for releases that predate the field — a language-level requirement
    /// entry then matches `lang_version` itself (the compat window: for
    /// mri the two are equal by construction).
    pub language_version: Option<String>,
}

/// Parse a cache entry directory name `<lang>-<lv>-<ver>-<triplet>`:
/// the triplet is the known platform suffix, `<lang>` the first segment,
/// `<ver>` the last, `<lv>` everything between (language versions may
/// carry dashes, e.g. prereleases).
fn parse_entry_name(name: &str, platform: &str) -> Option<(String, String, String)> {
    let rest = name.strip_suffix(platform)?.strip_suffix('-')?;
    let (engine, tail) = rest.split_once('-')?;
    let (lv, ver) = tail.rsplit_once('-')?;
    if engine.is_empty() || lv.is_empty() || ver.is_empty() {
        return None;
    }
    Some((engine.to_string(), lv.to_string(), ver.to_string()))
}

fn entry_exe_name(lv: &str, ver: &str, platform: &str) -> String {
    format!("tebako-runtime-{ver}-{lv}-{platform}{}", exe_suffix())
}

/// Synthesized env-image name (spec 05 §3's fallback spelling).
fn synthesized_image_base(lv: &str, ver: &str, platform: &str) -> String {
    format!("tebako-runtime-{ver}-{lv}-{platform}.tfs")
}

/// Match a release-index entry by the identity triple (spec 05 §2):
/// `tebako_version` + `{engine}_version` + `platform` as strings.
/// The runtime factory publishes windows exe assets SUFFIX-LESS, so the
/// entry's `filename` is the ONLY authoritative asset spelling (spec 00
/// §10 SSOT; tebako#456).
pub fn release_index_entry<'m>(
    manifest: &'m tebako_json::Value,
    engine: &str,
    lang_version: &str,
    tebako_version: &str,
    platform: &str,
) -> Option<&'m tebako_json::Value> {
    let tebako_json::Value::Array(entries) = manifest else {
        return None;
    };
    let lang_key = format!("{engine}_version");
    entries.iter().find(|e| {
        [
            ("tebako_version", tebako_version),
            (lang_key.as_str(), lang_version),
            ("platform", platform),
        ]
        .iter()
        .all(|(k, want)| e.find(k).and_then(|v| v.as_string()).as_deref() == Some(*want))
    })
}

/// `filename` of the entry itself (`facet: None`) or of a facet object
/// (`image` / `dll`) — verbatim, including any platform suffix.
pub fn entry_filename(entry: &tebako_json::Value, facet: Option<&str>) -> Option<String> {
    let node = match facet {
        Some(f) => entry.find(f)?,
        None => entry,
    };
    node.find("filename")
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty())
}

/// The exe / env-image names for a cache entry: flow the cached release
/// index verbatim when it names this identity, else the synthesized
/// fallback (`{name}.exe` on windows, `{name}` on posix — spec 05 §2's
/// pre-identity fallback). The download side (tebako-shim) reads the
/// same pair BEFORE the entry exists — same function, same grammar.
pub fn entry_asset_names(
    entry_dir: &Path,
    engine: &str,
    lv: &str,
    ver: &str,
    platform: &str,
) -> (String, String) {
    let flowed = std::fs::read_to_string(entry_dir.join("manifest.json"))
        .ok()
        .and_then(|text| {
            let parsed = tebako_json::parse(&text).ok()?;
            let e = release_index_entry(&parsed, engine, lv, ver, platform)?;
            Some((entry_filename(e, None), entry_filename(e, Some("image"))))
        });
    let exe = flowed
        .as_ref()
        .and_then(|(f, _)| f.clone())
        .unwrap_or_else(|| entry_exe_name(lv, ver, platform));
    let image = flowed
        .as_ref()
        .and_then(|(_, i)| i.clone())
        .unwrap_or_else(|| synthesized_image_base(lv, ver, platform));
    (exe, image)
}

/// A metadata key of the cached release index's entry for the exe
/// `exe_name` (`abi`, `implementation`, …): `None` when the entry or the
/// key is absent (pre-field releases — the compat window).
pub fn entry_meta(entry_dir: &Path, exe_name: &str, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(entry_dir.join("manifest.json")).ok()?;
    let parsed = tebako_json::parse(&text).ok()?;
    let tebako_json::Value::Array(entries) = &parsed else {
        return None;
    };
    entries.iter().find_map(|entry| {
        (entry
            .find("filename")
            .and_then(|f| f.as_string())
            .as_deref()
            == Some(exe_name))
        .then(|| entry.find(key).and_then(|a| a.as_string()))
        .flatten()
    })
}

/// The cached index entry's `contract_version` (spec 18) as a version
/// string — the factory index writes it as a JSON NUMBER; the string
/// spelling reads too. `None` when the mirror, the entry, or the key is
/// absent (the pre-era signal — the caller's negotiation decides what
/// absence means; spec 33 §4 fails it closed).
pub fn entry_contract_version(entry_dir: &Path, exe_name: &str) -> Option<String> {
    let text = std::fs::read_to_string(entry_dir.join("manifest.json")).ok()?;
    let parsed = tebako_json::parse(&text).ok()?;
    let tebako_json::Value::Array(entries) = &parsed else {
        return None;
    };
    entries.iter().find_map(|entry| {
        (entry
            .find("filename")
            .and_then(|f| f.as_string())
            .as_deref()
            == Some(exe_name))
        .then(|| {
            entry.find("contract_version").and_then(|v| match v {
                tebako_json::Value::Number(n) => Some(n.clone()),
                tebako_json::Value::String(s) => Some(s.clone()),
                _ => None,
            })
        })
        .flatten()
    })
}

// ---------------------------------------------------------------------
// spec 33 §1: the on_runtime release-index mirror
// ---------------------------------------------------------------------

/// The `on_runtime` key of a cached release-index entry (spec 33 §1):
/// the depending runtime's owner edge + mount + owner-contract, flowed
/// verbatim from the factory's release index. The in-image L1 block
/// stays the sole authority — the DRIVER cross-checks this mirror
/// against it at boot (a disagreement is "the release is lying", the
/// named boot error 65); the loader side (no image reader) plans the
/// composition from this mirror alone.
#[derive(Debug, Clone)]
pub struct OnRuntimeMirror {
    /// The owner runtime's engine (`java`, …).
    pub engine: String,
    /// The owner runtime's implementation requirement (`graalvm`, …);
    /// `None` = any implementation of the engine satisfies the edge.
    pub implementation: Option<String>,
    /// The owner runtime's language-version constraint.
    pub constraint: crate::manifest::Constraint,
    /// Where the depending runtime's env image mounts on the owner's
    /// boot (the `--tebako-image <dep>:-:<mount>` triple's point).
    pub mount: String,
    /// The declared owner-contract constraint; `None` = the spec 33 §2
    /// default, applied at negotiation, never written back.
    pub owner_contract: Option<crate::manifest::Constraint>,
}

impl OnRuntimeMirror {
    /// The effective owner-contract constraint (the declared one, else
    /// [`crate::manifest::OnRuntime::DEFAULT_OWNER_CONTRACT`]).
    pub fn owner_contract(&self) -> crate::manifest::Constraint {
        self.owner_contract.clone().unwrap_or_else(|| {
            crate::manifest::Constraint::new(crate::manifest::OnRuntime::DEFAULT_OWNER_CONTRACT)
                .unwrap()
        })
    }
}

/// The `on_runtime` mirror of the cached index entry for the exe
/// `exe_name` (spec 33 §1): `None` when the mirror file, the entry, or
/// the key is absent (every pre-spec-33 runtime — no composition, the
/// same compat window as [`entry_meta`]). A PRESENT but malformed key
/// is the named error: the shard the resolution already trusted is
/// lying — never a guessed composition.
pub fn on_runtime_mirror(
    entry_dir: &Path,
    exe_name: &str,
) -> Result<Option<OnRuntimeMirror>, String> {
    let Ok(text) = std::fs::read_to_string(entry_dir.join("manifest.json")) else {
        return Ok(None);
    };
    let Ok(parsed) = tebako_json::parse(&text) else {
        return Ok(None);
    };
    let tebako_json::Value::Array(entries) = &parsed else {
        return Ok(None);
    };
    let Some(entry) = entries
        .iter()
        .find(|e| e.find("filename").and_then(|f| f.as_string()).as_deref() == Some(exe_name))
    else {
        return Ok(None);
    };
    let Some(key) = entry.find("on_runtime") else {
        return Ok(None);
    };
    let bad = |why: String| {
        format!("the cached release index's on_runtime for {exe_name}: {why} (spec 33 §1)")
    };
    if !matches!(key, tebako_json::Value::Object(_)) {
        return Err(bad("must be a map".to_string()));
    }
    let required = |name: &str| -> Result<String, String> {
        key.find(name)
            .and_then(|v| v.as_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| bad(format!("{name} is required")))
    };
    let parse_constraint =
        |name: &str, src: String| -> Result<crate::manifest::Constraint, String> {
            crate::manifest::Constraint::new(&src)
                .map_err(|e| bad(format!("{name} is not a constraint: {e}")))
        };
    let engine = required("engine")?;
    let implementation = key
        .find("implementation")
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty());
    let constraint = parse_constraint("constraint", required("constraint")?)?;
    let mount = required("mount")?;
    let owner_contract = key
        .find("owner_contract")
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty())
        .map(|s| parse_constraint("owner_contract", s))
        .transpose()?;
    Ok(Some(OnRuntimeMirror {
        engine,
        implementation,
        constraint,
        mount,
        owner_contract,
    }))
}

/// One store entry dir → a [`CachedRuntime`] when it is well-formed
/// (parseable name for this platform, exe present); `None` otherwise.
/// Lenient by design: malformed entries are invisible to resolution
/// (doctor reports them).
fn scan_entry(
    entry_dir: &Path,
    name: &str,
    platform: &str,
) -> Option<(String, String, String, CachedRuntime)> {
    let (lang, lv, ver) = parse_entry_name(name, platform)?;
    let (exe_name, image_base) = entry_asset_names(entry_dir, &lang, &lv, &ver, platform);
    let exe = entry_dir.join(&exe_name);
    if !exe.is_file() {
        return None;
    }
    let image = entry_dir.join(&image_base);
    let image = if image.is_file() && entry_dir.join(format!("{image_base}.sha256")).is_file() {
        Some(image)
    } else {
        None
    };
    let rt = CachedRuntime {
        engine: lang.clone(),
        lang_version: lv.clone(),
        tebako_version: ver.clone(),
        dir: entry_dir.to_path_buf(),
        exe,
        image,
        abi: entry_meta(entry_dir, &exe_name, "abi"),
        implementation: entry_meta(entry_dir, &exe_name, "implementation"),
        language_version: entry_meta(entry_dir, &exe_name, "language_version"),
    };
    Some((lang, lv, ver, rt))
}

/// Scan `~/.tebako/runtimes/` for cached runtimes of `engine` on this
/// platform.
pub fn scan_cached(home: &Path, engine: &str) -> Vec<CachedRuntime> {
    let platform = platform_string();
    let dir = home.join("runtimes");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let entry_dir = entry.path();
        if !entry_dir.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((lang, .., rt)) = scan_entry(&entry_dir, &name, platform) else {
            continue;
        };
        if lang == engine {
            out.push(rt);
        }
    }
    out
}

/// Scan `~/.tebako/runtimes/` for cached runtimes of EVERY engine on
/// this platform — the info surface's machine view (resolution itself
/// always asks per engine).
pub fn scan_all_cached(home: &Path) -> Vec<CachedRuntime> {
    let platform = platform_string();
    let dir = home.join("runtimes");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let entry_dir = entry.path();
        if !entry_dir.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some((.., rt)) = scan_entry(&entry_dir, &name, platform) {
            out.push(rt);
        }
    }
    out
}

/// The newest cached runtime satisfying `constraint` (spec 05 §5:
/// range → any newer within range; abi-line `~>` → the locked line).
/// Two cache entries may share the language version (different tebako
/// builds): the tie breaks on the tebako version, newer first — an
/// arbitrary pick would let a stale runtime shadow a fresh one.
pub fn newest_compatible(
    cached: &[CachedRuntime],
    constraint: &Constraint,
) -> Option<CachedRuntime> {
    cached
        .iter()
        .filter(|c| constraint.matches(&c.lang_version))
        .max_by(|a, b| {
            versions::compare(&a.lang_version, &b.lang_version)
                .then_with(|| versions::compare(&a.tebako_version, &b.tebako_version))
        })
        .cloned()
}

/// The implementation-axis match (spec 28 §8; spec 30 §1): an edge
/// naming an implementation matches only cache entries declaring THAT
/// implementation; an edge omitting it matches any. A cache entry whose
/// release index predates the key (`implementation: None`) stays
/// eligible — the compat window, never a match failure of its own.
pub fn implementation_matches(cached: &CachedRuntime, want: Option<&str>) -> bool {
    match want {
        None => true,
        Some(w) => cached
            .implementation
            .as_deref()
            .map_or(true, |have| have == w),
    }
}

/// The spawned-runtime edge pick (spec 30 §1/§2): engine + optional
/// implementation + constraint against the cache, requiring the env
/// image (the spec-29 wrapper mounts it — an entry without the verified
/// image pair is pre-era or partial and can never serve a spawn).
/// Newest-compatible wins; `None` is data for the caller's named error
/// (never a guess).
pub fn resolve_spawned(
    home: &Path,
    engine: &str,
    implementation: Option<&str>,
    constraint: &Constraint,
) -> Option<CachedRuntime> {
    let cached: Vec<CachedRuntime> = scan_cached(home, engine)
        .into_iter()
        .filter(|c| c.image.is_some() && implementation_matches(c, implementation))
        .collect();
    newest_compatible(&cached, constraint)
}

/// One requirement entry against one cache entry (spec 28 §8): the
/// implementation axis (the compat-window rule of
/// [`implementation_matches`]), the abi axis re-asserted (both sides
/// `Option` — a pre-field cache entry stays eligible, never a match
/// failure of its own), then the version line: an
/// implementation-narrowed entry matches the runtime's OWN version; a
/// language-level entry matches the runtime's `language_version`,
/// falling back to its version on pre-field shards.
pub fn entry_matches(cached: &CachedRuntime, req: &crate::manifest::RuntimeRequirement) -> bool {
    if !implementation_matches(cached, req.implementation.as_deref()) {
        return false;
    }
    if let (Some(want), Some(have)) = (&req.abi, &cached.abi) {
        if want != have {
            return false;
        }
    }
    let line = if req.implementation.is_some() {
        &cached.lang_version
    } else {
        cached
            .language_version
            .as_deref()
            .unwrap_or(&cached.lang_version)
    };
    versions::from_validated(&req.constraint).matches(line)
}

/// The newest cached runtime matching ANY entry of `reqs` (spec 28 §8's
/// `any_of` pick — the entries are OR-ed; newest by (language version,
/// tebako version) as in [`newest_compatible`]).
pub fn newest_compatible_any(
    cached: &[CachedRuntime],
    reqs: &crate::manifest::RuntimeRequirements,
) -> Option<CachedRuntime> {
    cached
        .iter()
        .filter(|c| reqs.entries().iter().any(|r| entry_matches(c, r)))
        .max_by(|a, b| {
            versions::compare(&a.lang_version, &b.lang_version)
                .then_with(|| versions::compare(&a.tebako_version, &b.tebako_version))
        })
        .cloned()
}

/// The `any_of` spawn pick (spec 28 §8 over spec 30 §1's rule): the
/// newest cache entry matching ANY entry of the requirement list, the
/// env image still required. Single-entry requirements answer exactly
/// what [`resolve_spawned`] answers for the same axes on pre-field
/// shards.
pub fn resolve_spawned_any(
    home: &Path,
    reqs: &crate::manifest::RuntimeRequirements,
) -> Option<CachedRuntime> {
    let cached: Vec<CachedRuntime> = scan_cached(home, reqs.engine())
        .into_iter()
        .filter(|c| c.image.is_some())
        .collect();
    newest_compatible_any(&cached, reqs)
}

// ---------------------------------------------------------------------
// the spawn lock (spec 30 §3) — the dispatch-time pin the shim exports
// and the driver honors at spawn
// ---------------------------------------------------------------------

/// The `TEBAKO_SPAWN_LOCK` channel's variable name (spec 30 §3).
pub const SPAWN_LOCK_VAR: &str = "TEBAKO_SPAWN_LOCK";

/// The `TEBAKO_JAIL_TIGHTENING` channel's variable name (spec 32 §4):
/// the dispatch surface's USER tightening (`--jail` / `--no-host` /
/// `--mount`) exported as an env spec so every spawned child re-applies
/// it as the hereditary ceiling over its own recomputed union — a
/// spawned child never holds a grant the operator denied the parent.
pub const JAIL_TIGHTENING_VAR: &str = "TEBAKO_JAIL_TIGHTENING";

/// One locked entry of the dispatch-time spawn pin (spec 30 §3, spec 32
/// §5). Two MECE row shapes share the channel:
///
/// - **runtime row** — `payload: None`: `engine` resolves to exactly
///   `<lang_version>` of tebako `<tebako_version>`.
/// - **payload row** — `payload: Some((name, version))`: the pinned
///   PROVIDER payload of an expose-carrying `kind: executable` edge; the
///   entry's engine/version triple then nests the provider's OWN resolved
///   runtime pair exactly as a runtime row spells it.
///
/// Either way the versions are the dispatcher's picks, so a payload's
/// spawned children run the SAME artifacts the dispatch resolved (never
/// a newer cache arrival mid-run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnLockEntry {
    pub engine: String,
    pub lang_version: String,
    pub tebako_version: String,
    /// The payload row's provider pin (spec 32 §5): `(name, version)`.
    pub payload: Option<(String, String)>,
}

/// The wire form of one runtime lock entry:
/// `engine=lang_version:tebako_version`. The alphabets
/// (`[A-Za-z0-9._-]` — store entry names) keep the `=` `:` `;`
/// delimiters unambiguous.
pub fn spawn_lock_entry(engine: &str, lang_version: &str, tebako_version: &str) -> String {
    format!("{engine}={lang_version}:{tebako_version}")
}

/// The wire form of one payload lock entry (spec 32 §5):
/// `payload@payload_version=engine=lang_version:tebako_version` — the
/// `@`-in-subject form is the MECE discriminator (`@` appears in neither
/// engine names nor the runtime row's subject); the value nests the
/// provider's resolved runtime pair exactly as a runtime row spells it.
pub fn spawn_lock_payload_entry(
    payload: &str,
    payload_version: &str,
    engine: &str,
    lang_version: &str,
    tebako_version: &str,
) -> String {
    format!("{payload}@{payload_version}={engine}={lang_version}:{tebako_version}")
}

/// Parse the lock value: `;`-joined [`spawn_lock_entry`] and
/// [`spawn_lock_payload_entry`] forms. An empty value is no lock; a
/// malformed entry fails the whole parse (the channel is machine-written
/// — a torn value is a bug to surface, never to guess around).
pub fn parse_spawn_lock(value: &str) -> Result<Vec<SpawnLockEntry>, String> {
    let mut out = Vec::new();
    for raw in value.split(';') {
        let entry = raw.trim();
        if entry.is_empty() {
            continue;
        }
        let (subject, versions) = entry
            .split_once('=')
            .ok_or_else(|| format!("spawn-lock entry {entry:?} lacks '='"))?;
        // The `@`-in-subject discriminator (spec 32 §5): a subject
        // carrying `@` is a payload row, and its value nests the
        // provider's runtime pair (`engine=<lv>:<tv>`).
        let (payload, engine, pair) = if subject.contains('@') {
            let (name, pversion) = subject
                .split_once('@')
                .ok_or_else(|| format!("spawn-lock entry {entry:?} has a torn payload subject"))?;
            let (engine, pair) = versions
                .split_once('=')
                .ok_or_else(|| format!("spawn-lock entry {entry:?} lacks the nested engine"))?;
            (Some((name.to_string(), pversion.to_string())), engine, pair)
        } else {
            (None, subject, versions)
        };
        let (lv, tv) = pair
            .split_once(':')
            .ok_or_else(|| format!("spawn-lock entry {entry:?} lacks '<lv>:<tv>'"))?;
        if engine.is_empty() || lv.is_empty() || tv.is_empty() {
            return Err(format!("spawn-lock entry {entry:?} has an empty segment"));
        }
        if let Some((name, pversion)) = &payload {
            if name.is_empty() || pversion.is_empty() {
                return Err(format!(
                    "spawn-lock entry {entry:?} has an empty payload segment"
                ));
            }
        }
        out.push(SpawnLockEntry {
            engine: engine.to_string(),
            lang_version: lv.to_string(),
            tebako_version: tv.to_string(),
            payload,
        });
    }
    Ok(out)
}

/// The locked-entry pick (spec 30 §3): exactly the dispatcher-pinned
/// (lang_version, tebako_version) for `engine`, image required, the
/// implementation axis still applied. `None` means the locked entry
/// vanished from the store — data for the caller's named error.
pub fn resolve_locked(
    home: &Path,
    engine: &str,
    implementation: Option<&str>,
    lang_version: &str,
    tebako_version: &str,
) -> Option<CachedRuntime> {
    scan_cached(home, engine).into_iter().find(|c| {
        c.lang_version == lang_version
            && c.tebako_version == tebako_version
            && c.image.is_some()
            && implementation_matches(c, implementation)
    })
}

// ---------------------------------------------------------------------
// the store root (spec 00 §8) — the home resolution grammar
// ---------------------------------------------------------------------

/// The tebako home resolution (spec 00 §8): `$TEBAKO_HOME` >
/// platform default (`~/.tebako`; windows: `%LOCALAPPDATA%\tebako` >
/// `%USERPROFILE%\.tebako`). The SINGLE owner of the grammar (spec 00
/// §10) — tebako-shim's dispatcher and tebako-driver's spawn
/// interception both resolve through here; `get` reads the caller's
/// environment (tests inject a map).
pub fn tebako_home(get: impl Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    if let Some(home) = get("TEBAKO_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(home));
    }
    #[cfg(windows)]
    {
        if let Some(home) = get("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(home).join("tebako"));
        }
        if let Some(home) = get("USERPROFILE").filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(home).join(".tebako"));
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = get("HOME").filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(home).join(".tebako"));
        }
    }
    Err("cannot determine tebako home (set TEBAKO_HOME)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn entry_name_parses_engine_lv_ver() {
        let platform = "aarch64-macos";
        let (engine, lv, ver) =
            parse_entry_name("ruby-4.0.6-0.16.6-aarch64-macos", platform).unwrap();
        assert_eq!(
            (engine.as_str(), lv.as_str(), ver.as_str()),
            ("ruby", "4.0.6", "0.16.6")
        );
        // Language versions may carry dashes (prereleases).
        let (engine, lv, ver) =
            parse_entry_name("java-21-rc1-0.3.0-aarch64-macos", platform).unwrap();
        assert_eq!(
            (engine.as_str(), lv.as_str(), ver.as_str()),
            ("java", "21-rc1", "0.3.0")
        );
        // Other platforms' entries are invisible.
        assert!(parse_entry_name("ruby-4.0.6-0.16.6-x86_64-linux", platform).is_none());
        // Empty segments are malformed.
        assert!(parse_entry_name("ruby--0.16.6-aarch64-macos", platform).is_none());
        assert!(parse_entry_name("-3.3.12-0.16.6-aarch64-macos", platform).is_none());
    }

    #[test]
    fn exe_and_image_names_follow_the_platform() {
        let exe = entry_exe_name("4.0.6", "0.16.6", "aarch64-macos");
        let image = synthesized_image_base("4.0.6", "0.16.6", "aarch64-macos");
        #[cfg(windows)]
        assert!(exe.ends_with(".exe"));
        #[cfg(not(windows))]
        assert!(!exe.ends_with(".exe"));
        assert!(exe.starts_with("tebako-runtime-0.16.6-4.0.6-aarch64-macos"));
        assert_eq!(image, "tebako-runtime-0.16.6-4.0.6-aarch64-macos.tfs");
    }

    /// A fixture store entry: exe + optional image pair + optional
    /// release-index mirror carrying the given per-entry keys.
    fn fixture_entry(
        home: &Path,
        lv: &str,
        ver: &str,
        with_image: bool,
        index_entry: Option<String>,
    ) {
        fixture_entry_engine(home, "java", lv, ver, with_image, index_entry)
    }

    fn fixture_entry_engine(
        home: &Path,
        engine: &str,
        lv: &str,
        ver: &str,
        with_image: bool,
        index_entry: Option<String>,
    ) {
        let platform = platform_string();
        let dir = home
            .join("runtimes")
            .join(format!("{engine}-{lv}-{ver}-{platform}"));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = entry_exe_name(lv, ver, platform);
        std::fs::write(dir.join(&exe), b"exe").unwrap();
        if with_image {
            let image = synthesized_image_base(lv, ver, platform);
            std::fs::write(dir.join(&image), b"image").unwrap();
            std::fs::write(dir.join(format!("{image}.sha256")), b"x").unwrap();
        }
        if let Some(entry) = index_entry {
            std::fs::write(dir.join("manifest.json"), format!("[{entry}]")).unwrap();
        }
    }

    #[test]
    fn the_scan_flows_implementation_and_abi_from_the_cached_index() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-runtime-store-scan-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let platform = platform_string();
        let exe = entry_exe_name("21.0.12", "0.3.0", platform);
        fixture_entry(
            &tmp,
            "21.0.12",
            "0.3.0",
            true,
            Some(format!(
                "{{\"filename\": \"{exe}\", \"abi\": \"x\", \"implementation\": \"temurin\"}}"
            )),
        );
        // An entry predating the keys: both read as None (the compat
        // window — eligible, never a match failure).
        fixture_entry(&tmp, "21.0.11", "0.3.0", true, None);
        let cached = scan_cached(&tmp, "java");
        assert_eq!(cached.len(), 2);
        let new = cached.iter().find(|c| c.lang_version == "21.0.12").unwrap();
        assert_eq!(new.implementation.as_deref(), Some("temurin"));
        assert_eq!(new.abi.as_deref(), Some("x"));
        let old = cached.iter().find(|c| c.lang_version == "21.0.11").unwrap();
        assert_eq!(old.implementation, None);
        assert_eq!(old.abi, None);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_spawned_requires_the_image_and_honors_the_axes() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-runtime-store-resolve-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let platform = platform_string();
        // temurin 21.0.12 with image; pre-key 21.0.11 with image;
        // temurin 22 WITHOUT the image pair (never serves a spawn).
        fixture_entry(
            &tmp,
            "21.0.12",
            "0.3.0",
            true,
            Some(format!(
                "{{\"filename\": \"{}\", \"implementation\": \"temurin\"}}",
                entry_exe_name("21.0.12", "0.3.0", platform)
            )),
        );
        fixture_entry(&tmp, "21.0.11", "0.3.0", true, None);
        fixture_entry(
            &tmp,
            "22",
            "0.3.0",
            false,
            Some(format!(
                "{{\"filename\": \"{}\", \"implementation\": \"temurin\"}}",
                entry_exe_name("22", "0.3.0", platform)
            )),
        );
        let ge21 = crate::Constraint::new(">= 21").unwrap();
        let c = versions::from_validated(&ge21);
        // Newest compatible WITH an image: 21.0.12 (22 lacks the image).
        let pick = resolve_spawned(&tmp, "java", None, &c).unwrap();
        assert_eq!(pick.lang_version, "21.0.12");
        // The implementation axis narrows; a named one matching wins.
        let pick = resolve_spawned(&tmp, "java", Some("temurin"), &c).unwrap();
        assert_eq!(pick.lang_version, "21.0.12");
        // A named implementation nobody declares: the pre-key entry is
        // the compat window and wins as the only eligible candidate.
        let pick = resolve_spawned(&tmp, "java", Some("zulu"), &c).unwrap();
        assert_eq!(pick.lang_version, "21.0.11");
        // An unknown engine is no answer.
        assert!(resolve_spawned(&tmp, "python", None, &c).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// spec 28 §8 fixtures: a truffleruby shard (the implementation +
    /// language_version + abi keys) and a pre-field mri shard (no keys —
    /// the compat window), plus an imageless newest shard that never
    /// serves a spawn.
    fn spec28_shards(home: &Path) {
        let platform = platform_string();
        fixture_entry_engine(
            home,
            "ruby",
            "34.0.1",
            "0.4.0",
            true,
            Some(format!(
                "{{\"filename\": \"{}\", \"implementation\": \"truffleruby\", \"language_version\": \"3.4\", \"abi\": \"arm64-darwin-23\"}}",
                entry_exe_name("34.0.1", "0.4.0", platform)
            )),
        );
        fixture_entry_engine(home, "ruby", "3.4.2", "0.4.0", true, None);
        fixture_entry_engine(
            home,
            "ruby",
            "35.0.0",
            "0.4.0",
            false,
            Some(format!(
                "{{\"filename\": \"{}\", \"implementation\": \"truffleruby\", \"language_version\": \"3.5\"}}",
                entry_exe_name("35.0.0", "0.4.0", platform)
            )),
        );
    }

    fn req28(
        implementation: Option<&str>,
        constraint: &str,
    ) -> crate::manifest::RuntimeRequirement {
        crate::manifest::RuntimeRequirement {
            engine: "ruby".to_string(),
            constraint: crate::Constraint::new(constraint).unwrap(),
            implementation: implementation.map(str::to_string),
            abi: None,
        }
    }

    #[test]
    fn entry_matches_reads_the_language_version_line() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-runtime-store-entry-matches-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        spec28_shards(&tmp);
        let cached = scan_cached(&tmp, "ruby");
        let truffle = cached.iter().find(|c| c.lang_version == "34.0.1").unwrap();
        assert_eq!(truffle.implementation.as_deref(), Some("truffleruby"));
        assert_eq!(truffle.language_version.as_deref(), Some("3.4"));
        let mri = cached.iter().find(|c| c.lang_version == "3.4.2").unwrap();
        assert_eq!(mri.language_version, None);

        // A language-level entry matches the shard's language_version —
        // truffleruby 34.0.1 speaks ruby 3.4…
        let language = req28(None, ">= 3.3, < 5.0");
        assert!(entry_matches(truffle, &language));
        assert!(entry_matches(mri, &language));
        // …and never the shard's OWN version when language_version
        // exists: 34.0.1 is not "< 5.0", yet the entry matched above;
        // ">= 34" reads the language line and fails.
        assert!(!entry_matches(truffle, &req28(None, ">= 34")));
        // On a pre-field shard the fallback IS the own version (for mri
        // the two lines are one).
        assert!(entry_matches(mri, &req28(None, "~> 3.4")));

        // An implementation-narrowed entry matches the OWN version line…
        assert!(entry_matches(truffle, &req28(Some("truffleruby"), "~> 34.0")));
        // …never the language line…
        assert!(!entry_matches(
            truffle,
            &req28(Some("truffleruby"), ">= 3.3, < 5.0")
        ));
        // …and filters the implementation axis (mri declares none — the
        // compat window keeps it eligible on its own line, but "~> 34.0"
        // is not 3.4.2; a named implementation on a DECLARING shard must
        // agree).
        assert!(!entry_matches(mri, &req28(Some("truffleruby"), "~> 34.0")));
        assert!(!entry_matches(
            truffle,
            &req28(Some("jruby"), ">= 3.3, < 5.0")
        ));
        // The compat window: a named implementation nobody declares stays
        // eligible on a pre-key shard (implementation_matches's rule).
        assert!(entry_matches(mri, &req28(Some("jruby"), "~> 3.4")));

        // The abi axis re-asserts: only the shard carrying the payload's
        // platform string matches (both sides Some); a pre-field shard
        // (abi: None) stays eligible.
        let native = |abi: &str| crate::manifest::RuntimeRequirement {
            abi: Some(abi.to_string()),
            ..req28(Some("truffleruby"), "~> 34.0")
        };
        assert!(entry_matches(truffle, &native("arm64-darwin-23")));
        assert!(!entry_matches(truffle, &native("x86_64-linux-gnu")));
        // A pre-key shard stays eligible through the abi axis too: the
        // same axes on mri's OWN line ("~> 3.4" reads 3.4.2 — the
        // implementation-named compat window plus abi: None both stay
        // eligible) match mri and never the truffleruby shard, whose
        // own line is 34.0.1.
        let native_mri = |abi: &str| crate::manifest::RuntimeRequirement {
            abi: Some(abi.to_string()),
            ..req28(Some("truffleruby"), "~> 3.4")
        };
        assert!(entry_matches(mri, &native_mri("arm64-darwin-23")));
        assert!(!entry_matches(truffle, &native_mri("arm64-darwin-23")));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_spawned_any_picks_the_newest_match_across_the_any_of_entries() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-runtime-store-spawned-any-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        spec28_shards(&tmp);
        // any_of: the language level OR truffleruby's own line — both
        // imaged shards admit (truffleruby via either entry, mri via the
        // language entry); newest by (language version, tebako version)
        // wins, and the imageless 35.0.0 never serves a spawn.
        let list = crate::manifest::RuntimeRequirements::many(vec![
            req28(None, ">= 3.3, < 5.0"),
            req28(Some("truffleruby"), "~> 34.0"),
        ]);
        let pick = resolve_spawned_any(&tmp, &list).unwrap();
        assert_eq!(pick.lang_version, "34.0.1");
        // A requirement no entry satisfies is no answer (never a guess).
        let none = crate::manifest::RuntimeRequirements::many(vec![
            req28(None, ">= 5.0"),
            req28(Some("jruby"), "~> 9.5"),
        ]);
        assert!(resolve_spawned_any(&tmp, &none).is_none());
        // The single-entry form diverges from resolve_spawned exactly
        // where spec 28 §8 says it must: the language-level entry reads
        // the shard's language_version line, so "3.4" on truffleruby
        // 34.0.1 matches "~> 3.4" and the newest MATCH is the
        // truffleruby shard — while resolve_spawned reads the own line
        // only and stays on mri 3.4.2.
        let single = crate::manifest::RuntimeRequirements::one(req28(None, "~> 3.4"));
        let pick = resolve_spawned_any(&tmp, &single).unwrap();
        assert_eq!(pick.lang_version, "34.0.1");
        let c = versions::from_validated(&crate::Constraint::new("~> 3.4").unwrap());
        let own_line = resolve_spawned(&tmp, "ruby", None, &c).unwrap();
        assert_eq!(own_line.lang_version, "3.4.2");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn newest_compatible_ties_break_on_the_tebako_version() {
        let mk = |lv: &str, ver: &str| CachedRuntime {
            engine: "ruby".to_string(),
            lang_version: lv.to_string(),
            tebako_version: ver.to_string(),
            dir: PathBuf::new(),
            exe: PathBuf::new(),
            image: None,
            abi: None,
            implementation: None,
            language_version: None,
        };
        let cached = vec![
            mk("3.3.12", "0.16.5"),
            mk("3.3.12", "0.16.6"),
            mk("3.2.9", "9.9.9"),
        ];
        let validated = crate::Constraint::new("~> 3.3.0").unwrap();
        let c = versions::from_validated(&validated);
        let pick = newest_compatible(&cached, &c).unwrap();
        assert_eq!(pick.tebako_version, "0.16.6");
        assert!(versions::compare(pick.lang_version.as_str(), "3.3.11") == Ordering::Greater);
    }

    #[test]
    fn spawn_lock_round_trips_and_rejects_torn_values() {
        let wire = format!(
            "{};{}",
            spawn_lock_entry("java", "21.0.12", "0.3.0"),
            spawn_lock_entry("ruby", "3.3.12", "0.16.17")
        );
        assert_eq!(wire, "java=21.0.12:0.3.0;ruby=3.3.12:0.16.17");
        let entries = parse_spawn_lock(&wire).unwrap();
        assert_eq!(
            entries,
            vec![
                SpawnLockEntry {
                    engine: "java".to_string(),
                    lang_version: "21.0.12".to_string(),
                    tebako_version: "0.3.0".to_string(),
                    payload: None,
                },
                SpawnLockEntry {
                    engine: "ruby".to_string(),
                    lang_version: "3.3.12".to_string(),
                    tebako_version: "0.16.17".to_string(),
                    payload: None,
                },
            ]
        );
        // Empty is no lock; blank segments tolerate trailing ';'.
        assert_eq!(parse_spawn_lock("").unwrap(), vec![]);
        assert_eq!(parse_spawn_lock("java=21:0.3.0;").unwrap().len(), 1);
        // Torn values fail the whole parse — never a guessed half-lock.
        assert!(parse_spawn_lock("java-21").is_err());
        assert!(parse_spawn_lock("java=21").is_err());
        assert!(parse_spawn_lock("=21:0.3.0").is_err());
        assert!(parse_spawn_lock("java=:0.3.0").is_err());
    }

    #[test]
    fn spawn_lock_payload_rows_round_trip_and_reject_torn_values() {
        // spec 32 §5: the payload row nests the provider's resolved
        // runtime pair; the `@`-in-subject form is the MECE
        // discriminator against the runtime row's bare-engine subject.
        let wire = format!(
            "{};{}",
            spawn_lock_entry("java", "21.0.12", "0.3.0"),
            spawn_lock_payload_entry("xml2rfc", "3.34.0", "python", "3.13.15", "2.1.10")
        );
        assert_eq!(
            wire,
            "java=21.0.12:0.3.0;xml2rfc@3.34.0=python=3.13.15:2.1.10"
        );
        let entries = parse_spawn_lock(&wire).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].payload, None);
        assert_eq!(
            entries[1].payload,
            Some(("xml2rfc".to_string(), "3.34.0".to_string()))
        );
        assert_eq!(entries[1].engine, "python");
        assert_eq!(entries[1].lang_version, "3.13.15");
        assert_eq!(entries[1].tebako_version, "2.1.10");
        // Torn payload rows fail the whole parse — never a guessed half-lock.
        assert!(parse_spawn_lock("xml2rfc@3.34.0=3.13.15:2.1.10").is_err());
        assert!(parse_spawn_lock("xml2rfc@=python=3.13.15:2.1.10").is_err());
        assert!(parse_spawn_lock("@3.34.0=python=3.13.15:2.1.10").is_err());
        assert!(parse_spawn_lock("xml2rfc@3.34.0=python=:2.1.10").is_err());
    }

    #[test]
    fn resolve_locked_pins_the_exact_pair() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-runtime-store-locked-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let platform = platform_string();
        fixture_entry(
            &tmp,
            "21.0.12",
            "0.3.0",
            true,
            Some(format!(
                "{{\"filename\": \"{}\", \"implementation\": \"temurin\"}}",
                entry_exe_name("21.0.12", "0.3.0", platform)
            )),
        );
        fixture_entry(&tmp, "21.0.11", "0.3.0", true, None);
        // The exact pin wins over the newer compatible entry's pull.
        let pick = resolve_locked(&tmp, "java", None, "21.0.11", "0.3.0").unwrap();
        assert_eq!(pick.lang_version, "21.0.11");
        // A vanished lock entry is no answer (the caller's named error).
        assert!(resolve_locked(&tmp, "java", None, "21.0.10", "0.3.0").is_none());
        // The implementation axis still applies to the pinned entry.
        assert!(resolve_locked(&tmp, "java", Some("temurin"), "21.0.12", "0.3.0").is_some());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tebako_home_prefers_the_env_var_then_the_platform_default() {
        let home = tebako_home(|k| (k == "TEBAKO_HOME").then(|| "/x/tebako".to_string())).unwrap();
        assert_eq!(home, PathBuf::from("/x/tebako"));
        // An empty override is no override.
        let home = tebako_home(|k| match k {
            "TEBAKO_HOME" => Some(String::new()),
            #[cfg(not(windows))]
            "HOME" => Some("/u".to_string()),
            #[cfg(windows)]
            "LOCALAPPDATA" => Some("C:/App/Local".to_string()),
            _ => None,
        })
        .unwrap();
        #[cfg(not(windows))]
        assert_eq!(home, PathBuf::from("/u/.tebako"));
        #[cfg(windows)]
        assert_eq!(home, PathBuf::from("C:/App/Local\\tebako"));
        // Nothing resolveable is a named error, never a guess.
        assert!(tebako_home(|_| None).is_err());
    }

    #[test]
    fn entry_contract_version_reads_the_numeric_and_string_spellings() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-contract-version-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let platform = platform_string();
        let exe = entry_exe_name("25.0.4.1", "2.4.1", platform);
        // the factory index's spelling: a JSON NUMBER
        fixture_entry(
            &tmp,
            "25.0.4.1",
            "2.4.1",
            true,
            Some(format!(
                "{{\"filename\": \"{exe}\", \"contract_version\": 2}}"
            )),
        );
        let dir = tmp
            .join("runtimes")
            .join(format!("java-25.0.4.1-2.4.1-{platform}"));
        assert_eq!(entry_contract_version(&dir, &exe).as_deref(), Some("2"));
        // the string spelling reads too
        fixture_entry(
            &tmp,
            "21.0.12",
            "0.3.0",
            true,
            Some(format!(
                "{{\"filename\": \"{}\", \"contract_version\": \"1\"}}",
                entry_exe_name("21.0.12", "0.3.0", platform)
            )),
        );
        let dir2 = tmp
            .join("runtimes")
            .join(format!("java-21.0.12-0.3.0-{platform}"));
        let exe2 = entry_exe_name("21.0.12", "0.3.0", platform);
        assert_eq!(entry_contract_version(&dir2, &exe2).as_deref(), Some("1"));
        // absent key, absent entry, absent mirror: the pre-era None
        let exe3 = entry_exe_name("21.0.11", "0.3.0", platform);
        fixture_entry(&tmp, "21.0.11", "0.3.0", true, None);
        let dir3 = tmp
            .join("runtimes")
            .join(format!("java-21.0.11-0.3.0-{platform}"));
        assert_eq!(entry_contract_version(&dir3, &exe3), None);
        assert_eq!(entry_contract_version(&dir3, "never-released"), None);
        assert_eq!(entry_contract_version(&tmp.join("runtimes"), &exe3), None);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------
    // spec 33 §1: the on_runtime release-index mirror
    // -----------------------------------------------------------------

    #[test]
    fn on_runtime_mirror_parses_the_full_shape() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-on-runtime-mirror-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let platform = platform_string();
        let exe = entry_exe_name("34.0.1", "2.4.0", platform);
        // the truffleruby shape: the DEPENDING runtime is a ruby engine
        // whose on_runtime edge names the java owner
        fixture_entry_engine(
            &tmp,
            "ruby",
            "34.0.1",
            "2.4.0",
            true,
            Some(format!(
                "{{\"filename\": \"{exe}\", \"on_runtime\": {{\"engine\": \"java\", \"implementation\": \"graalvm\", \"constraint\": \">= 24\", \"mount\": \"/__runners__/truffleruby\", \"owner_contract\": \">= 2\"}}}}"
            )),
        );
        let mirror = on_runtime_mirror(
            &tmp.join("runtimes")
                .join(format!("ruby-34.0.1-2.4.0-{platform}")),
            &exe,
        )
        .unwrap()
        .expect("the mirror parses");
        assert_eq!(mirror.engine, "java");
        assert_eq!(mirror.implementation.as_deref(), Some("graalvm"));
        assert_eq!(mirror.constraint.as_str(), ">= 24");
        assert_eq!(mirror.mount, "/__runners__/truffleruby");
        assert_eq!(mirror.owner_contract().as_str(), ">= 2");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn on_runtime_mirror_minimal_and_absent() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-on-runtime-min-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let platform = platform_string();
        let exe = entry_exe_name("9.4.8.0", "2.4.0", platform);
        // minimal: no implementation, no owner_contract — the ">= 2"
        // default applies at negotiation, never written back
        // (the jruby shape: a ruby engine riding a java owner)
        fixture_entry_engine(
            &tmp,
            "ruby",
            "9.4.8.0",
            "2.4.0",
            true,
            Some(format!(
                "{{\"filename\": \"{exe}\", \"on_runtime\": {{\"engine\": \"java\", \"constraint\": \">= 21\", \"mount\": \"/__runners__/jruby\"}}}}"
            )),
        );
        let dir = tmp
            .join("runtimes")
            .join(format!("ruby-9.4.8.0-2.4.0-{platform}"));
        let mirror = on_runtime_mirror(&dir, &exe)
            .unwrap()
            .expect("minimal parses");
        assert_eq!(mirror.implementation, None);
        assert_eq!(
            mirror.owner_contract().as_str(),
            crate::manifest::OnRuntime::DEFAULT_OWNER_CONTRACT
        );
        // the key absent: no composition (every pre-spec-33 runtime)
        fixture_entry(&tmp, "21.0.12", "0.3.0", true, None);
        let platform2 = platform_string();
        let exe2 = entry_exe_name("21.0.12", "0.3.0", platform2);
        assert!(on_runtime_mirror(
            &tmp.join("runtimes")
                .join(format!("java-21.0.12-0.3.0-{platform2}")),
            &exe2
        )
        .unwrap()
        .is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn on_runtime_mirror_malformed_is_a_named_error() {
        let tmp = std::env::temp_dir().join(format!(
            "tpkg-on-runtime-bad-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        let platform = platform_string();
        // the key not a map
        let exe = entry_exe_name("9.4.8.0", "2.4.0", platform);
        fixture_entry(
            &tmp,
            "9.4.8.0",
            "2.4.0",
            true,
            Some(format!(
                "{{\"filename\": \"{exe}\", \"on_runtime\": \"garbage\"}}"
            )),
        );
        let dir = tmp
            .join("runtimes")
            .join(format!("java-9.4.8.0-2.4.0-{platform}"));
        let err = on_runtime_mirror(&dir, &exe).unwrap_err();
        assert!(err.contains("on_runtime"), "{err}");
        let _ = std::fs::remove_dir_all(&tmp);

        // engine missing
        let tmp2 = std::env::temp_dir().join(format!(
            "tpkg-on-runtime-bad2-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        fixture_entry(
            &tmp2,
            "9.4.8.0",
            "2.4.0",
            true,
            Some(format!(
                "{{\"filename\": \"{exe}\", \"on_runtime\": {{\"constraint\": \">= 21\", \"mount\": \"/x\"}}}}"
            )),
        );
        let dir2 = tmp2
            .join("runtimes")
            .join(format!("java-9.4.8.0-2.4.0-{platform}"));
        let err = on_runtime_mirror(&dir2, &exe).unwrap_err();
        assert!(err.contains("engine"), "{err}");
        let _ = std::fs::remove_dir_all(&tmp2);

        // a constraint that is not a constraint
        let tmp3 = std::env::temp_dir().join(format!(
            "tpkg-on-runtime-bad3-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        fixture_entry(
            &tmp3,
            "9.4.8.0",
            "2.4.0",
            true,
            Some(format!(
                "{{\"filename\": \"{exe}\", \"on_runtime\": {{\"engine\": \"java\", \"constraint\": \"bogus\", \"mount\": \"/x\"}}}}"
            )),
        );
        let dir3 = tmp3
            .join("runtimes")
            .join(format!("java-9.4.8.0-2.4.0-{platform}"));
        let err = on_runtime_mirror(&dir3, &exe).unwrap_err();
        assert!(err.contains("constraint"), "{err}");
        let _ = std::fs::remove_dir_all(&tmp3);
    }
}
