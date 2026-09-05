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
        let platform = platform_string();
        let dir = home
            .join("runtimes")
            .join(format!("java-{lv}-{ver}-{platform}"));
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
}
