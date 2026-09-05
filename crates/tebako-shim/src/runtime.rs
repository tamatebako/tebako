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
//! manifest.json-primary / SHA256SUMS-fallback checksum extraction,
//! `TEBAKO_RUNTIME_MIRROR` / `TEBAKO_OFFLINE` — reimplemented
//! here rather than linked: the bootstrap crate drags in rnp, and the
//! shim stays pure-Rust + tebako-http.

use std::io::Read;
use std::path::Path;

use tpkg::RuntimeRequirement;

use crate::config::{self, RuntimePref};
use crate::versions::{self, Constraint};
use crate::{
    fail, Ctx, ShimError, EX_TEBAKO_CONTRACT, EX_TEBAKO_IO, EX_TEBAKO_SHA, EX_TEBAKO_UNAVAILABLE,
};

const DEFAULT_RELEASES_BASE: &str =
    "https://github.com/tamatebako/tebako-runtime-ruby/releases/download";
const LOCK_TIMEOUT_MS: u64 = 120_000;
const LOCK_POLL_MS: u64 = 200;

// The store grammar, the cache scan, and the version machinery moved to
// tpkg (spec 00 §10 — one owner, every consumer flows): the shim's
// resolution/download layers below build on these re-exports.
use tpkg::runtime_store::{entry_asset_names, entry_filename, entry_meta, release_index_entry};
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
    requirement: Option<&RuntimeRequirement>,
    allow_download: bool,
    ctx: &Ctx,
) -> Result<RuntimeResolution, ShimError> {
    let Some(req) = requirement else {
        return Ok(RuntimeResolution::Zero);
    };
    // The constraint was validated at manifest parse (tpkg::Constraint) —
    // the dispatcher only evaluates it against cached/offered versions.
    let constraint = versions::from_validated(&req.constraint);
    let cached = scan_cached(&ctx.home, &req.engine);
    // The abi line (spec 05 §5): a native-extension payload matches only
    // runtimes carrying ITS platform string. A runtime whose release
    // predates the field (abi: None) stays eligible — the compat window,
    // never a match failure of its own.
    let abi_compatible = |c: &CachedRuntime| {
        req.abi.as_ref().map_or(true, |want| {
            c.abi.as_ref().map_or(true, |have| have == want)
        })
    };
    if let Some(hit) = newest_compatible(
        &cached
            .iter()
            .filter(|c| abi_compatible(c))
            .cloned()
            .collect::<Vec<_>>(),
        &constraint,
    ) {
        return Ok(RuntimeResolution::Ready(Box::new(hit)));
    }
    let abi_note = match &req.abi {
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

    // No compatible cached runtime. The download target: the release
    // index's pick on the configured preference's line (config.yaml
    // `runtimes:`, spec 07 §4 "runtime preferences") — or, with no
    // preference configured, on the product default line
    // (tebako-resolve::DEFAULT_TEBAKO_VERSION).
    let cfg = config::load_config(&ctx.home)?;
    let pref = cfg.runtimes.get(&req.engine);
    let cached_note = if cached.is_empty() {
        format!("no cached {} runtimes for this platform", req.engine)
    } else {
        format!(
            "cached {} runtimes ({}) do not satisfy \"{}\"{}{}",
            req.engine,
            cached
                .iter()
                .map(|c| c.lang_version.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            constraint.source(),
            if constraint.source().contains("~>") {
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
            },
            true,
        ),
    };
    let pref = &pref_owned;
    if !prefless && !constraint.matches(&pref.version) {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "runtime preference {}@{} does not satisfy \"{}\": {cached_note}\n  re-pin the preference (`tebako use --runtime {}@<version>`) or rebuild the payload against a newer ABI line",
                req.engine,
                pref.version,
                constraint.source(),
                req.engine
            ),
        );
    }
    if !allow_download {
        if prefless {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "no compatible runtime for {} \"{}\": {cached_note}\n  and no runtime preference is configured — set `runtimes: {{{}: {{version: …, tebako: …}}}}` in ~/.tebako/config.yaml, or pre-seed the cache",
                    req.engine,
                    constraint.source(),
                    req.engine
                ),
            );
        }
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "no compatible cached runtime for {} \"{}\" (would download {}@{}): {cached_note}",
                req.engine,
                constraint.source(),
                req.engine,
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
    let target = match index_selected_target(req, &constraint, pref, ctx)? {
        Some(pick) => pick,
        None if !prefless => pref.clone(),
        None => {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "no compatible runtime for {} \"{}\": {cached_note}\n  and no runtime preference is configured, and the default-line release index did not read — set `runtimes: {{{}: {{version: …, tebako: …}}}}` in ~/.tebako/config.yaml, or pre-seed the cache",
                    req.engine,
                    constraint.source(),
                    req.engine
                ),
            );
        }
    };
    let rt = download_runtime(&req.engine, &target, ctx)?;
    // The downloaded runtime's abi line must satisfy the payload too —
    // the release index carries it (abi: None is the compat window).
    if let (Some(want), Some(have)) = (&req.abi, &rt.abi) {
        if want != have {
            return fail(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "downloaded runtime {}@{} carries abi \"{have}\" but the payload requires \"{want}\" — the payload was built against a different platform line; rebuild the payload or pin a matching runtime",
                    req.engine, target.version
                ),
            );
        }
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
        abi: None,
    };
    let RuntimeResolution::Ready(rt) = resolve_runtime(Some(&req), allow_download, ctx)? else {
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

/// The release index's availability facet for `platform` (spec 13 §2a —
/// the locked entry shape declares `{engine}_version` + `platform` +
/// `tebako_version`): the `(lang_version, tebako_version)` of every
/// entry released for this platform. `None` when NO entry declares the
/// availability keys at all — an index that predates them is
/// uninformative (the config pin stays the target), never an
/// availability verdict.
fn released_versions(
    text: &str,
    engine: &str,
    platform: &str,
) -> Option<Vec<(String, Option<String>)>> {
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
            released.push((
                lang_version,
                entry.find("tebako_version").and_then(|v| v.as_string()),
            ));
        }
    }
    keyed.then_some(released)
}

/// The download-target selection on a cache miss: consult the release
/// index of the pin's tebako line (`v{pref.tebako}/manifest.json`) for
/// the newest interpreter version that both satisfies `constraint` and
/// is released for this platform. Three outcomes:
///
/// - `Ok(None)` — the index did not read or carries no availability
///   keys (and always in offline mode, which never fetches): the config
///   pin stays the target and every pin-path behavior is unchanged;
/// - `Ok(Some(target))` — the index's pick (the entry's own
///   `tebako_version` when declared, else the pin's line);
/// - `Err` — the index read fine and NOTHING released for this platform
///   satisfies the constraint: the named platform-availability error
///   naming the platform, the constraint, and what IS released (a
///   platform that trails the payload's needs — e.g. windows-ucrt64
///   released only through ruby 3.2.x — is a diagnosis, never a 404).
fn index_selected_target(
    req: &RuntimeRequirement,
    constraint: &Constraint,
    pref: &RuntimePref,
    ctx: &Ctx,
) -> Result<Option<RuntimePref>, ShimError> {
    if offline_mode(ctx) {
        return Ok(None);
    }
    let platform = platform_string();
    let base_raw = releases_base(ctx);
    let base = skip_file_scheme(&base_raw).to_string();
    let local = base_is_local(&base_raw);
    let probe = ctx
        .home
        .join("tmp")
        .join(format!("index-probe.{}", std::process::id()));
    if std::fs::create_dir_all(&probe).is_err() {
        // Uninformative, not fatal: the download path re-creates the
        // store dirs under the install lock and names the IO failure.
        return Ok(None);
    }
    let text = fetch_manifest_text(&base, local, &pref.tebako, &probe);
    let _ = std::fs::remove_dir_all(&probe);
    let Some(text) = text else {
        return Ok(None);
    };
    let Some(released) = released_versions(&text, &req.engine, platform) else {
        return Ok(None);
    };
    if let Some((version, tebako)) = released
        .iter()
        .filter(|(version, _)| constraint.matches(version))
        .max_by(|a, b| {
            versions::compare(&a.0, &b.0).then_with(|| {
                versions::compare(a.1.as_deref().unwrap_or(""), b.1.as_deref().unwrap_or(""))
            })
        })
    {
        return Ok(Some(RuntimePref {
            version: version.clone(),
            tebako: tebako.clone().unwrap_or_else(|| pref.tebako.clone()),
        }));
    }
    let mut known: Vec<&str> = released
        .iter()
        .map(|(version, _)| version.as_str())
        .collect();
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
            "no released {} runtime for {platform} satisfies \"{}\"\n  released for {platform}: {known}\n  this payload needs a newer {} than this platform provides yet",
            req.engine,
            constraint.source(),
            req.engine
        ),
    )
}

// ---------------------------------------------------------------------
// download — the bootstrap discipline, reimplemented (see module docs)
// ---------------------------------------------------------------------

fn offline_mode(ctx: &Ctx) -> bool {
    ctx.env_get("TEBAKO_OFFLINE")
        .is_some_and(|v| !v.is_empty() && v != "0")
}

fn releases_base(ctx: &Ctx) -> String {
    ctx.env_get("TEBAKO_RUNTIME_MIRROR")
        .filter(|v| !v.is_empty())
        .unwrap_or(DEFAULT_RELEASES_BASE)
        .to_string()
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
            Err(tebako_http::FetchError::DownloadFailed(_)) => {
                attempts += 1;
                if attempts >= 3 {
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

/// The release manifest's ruby DLL facet for the exe entry `asset`
/// (tebako-runtime-ruby#40 — the additive `dll` key, windows packages
/// only): `(dll asset filename, install_as, sha256)`. `None` when the
/// entry carries no `dll` key (every POSIX entry) or the key is
/// incomplete — the facet is manifest-keyed: the PE name (`install_as`)
/// exists only there, never derived (the factory's
/// RubyVersion#msys_dll_name is its single owner).
fn dll_from_manifest(text: &str, asset: &str) -> Option<(String, String, String)> {
    let parsed = tebako_json::parse(text).ok()?;
    let tebako_json::Value::Array(entries) = &parsed else {
        return None;
    };
    entries.iter().find_map(|entry| {
        if entry
            .find("filename")
            .and_then(|f| f.as_string())
            .as_deref()
            != Some(asset)
        {
            return None;
        }
        let dll = entry.find("dll")?;
        let filename = dll.find("filename").and_then(|v| v.as_string())?;
        let install_as = dll.find("install_as").and_then(|v| v.as_string())?;
        let sha256 = dll.find("sha256").and_then(|v| v.as_string())?;
        Some((filename, install_as, sha256))
    })
}

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

/// Fetch the release index (`manifest.json`) into the tmp staging dir
/// and return its text. `None` when it does not exist or does not read —
/// the caller decides what that means (spec 18: the pre-era signal).
fn fetch_manifest_text(base: &str, local: bool, abi: &str, tmp_dir: &Path) -> Option<String> {
    let manifest_tmp = tmp_dir.join("manifest.json");
    fetch_url(
        &format!("{base}/v{abi}/manifest.json"),
        local,
        &manifest_tmp,
    )
    .ok()?;
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

/// The expected checksum for an asset: manifest.json primary,
/// SHA256SUMS.txt fallback (the bootstrap's exact order). Returns the
/// optional sha plus the two diagnostic indices so the caller names the
/// failure itself — an absent entry is data, not an error (the v1-era
/// image rule needs it). `manifest_text` is the already-fetched release
/// index when the caller holds it (the contract gate reads it first);
/// `None` fetches it here.
#[allow(clippy::too_many_arguments)]
fn expected_checksum(
    base: &str,
    local: bool,
    abi: &str,
    asset: &str,
    tmp_dir: &Path,
    manifest_text: Option<&str>,
) -> Result<(Option<String>, (usize, usize)), ShimError> {
    let sums_url = format!("{base}/v{abi}/SHA256SUMS.txt");
    let mut expected = None;
    let mut diag_manifest = 1;
    let owned_text;
    let text = match manifest_text {
        Some(text) => {
            diag_manifest = 3;
            text
        }
        None => {
            let manifest_tmp = tmp_dir.join("manifest.json");
            if fetch_url(
                &format!("{base}/v{abi}/manifest.json"),
                local,
                &manifest_tmp,
            )
            .is_ok()
            {
                diag_manifest = 2;
                owned_text = std::fs::read_to_string(&manifest_tmp).ok();
                if owned_text.is_some() {
                    diag_manifest = 3;
                }
                owned_text.as_deref().unwrap_or("")
            } else {
                ""
            }
        }
    };
    if diag_manifest == 3 {
        if let Ok(sha) = sha_from_manifest(text, asset) {
            diag_manifest = 4;
            expected = Some(sha);
        }
    }
    let mut diag_sums = 0;
    if expected.is_none() {
        diag_sums = 1;
        let sums_tmp = tmp_dir.join("SHA256SUMS.txt");
        if fetch_url(&sums_url, local, &sums_tmp).is_ok() {
            diag_sums = 2;
            if let Ok(text) = std::fs::read_to_string(&sums_tmp) {
                diag_sums = 3;
                if let Ok(sha) = sha_from_sums(&text, asset) {
                    diag_sums = 4;
                    expected = Some(sha);
                }
            }
        }
    }
    Ok((expected, (diag_manifest, diag_sums)))
}

/// Download + verify + atomically install one asset into an entry staging
/// dir. Returns the verified sha256.
fn install_asset(
    url: &str,
    local: bool,
    asset: &str,
    tmp_dir: &Path,
    expected: &str,
) -> Result<String, ShimError> {
    let tmp_asset = tmp_dir.join(asset);
    if fetch_url(url, local, &tmp_asset).is_err() {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "runtime download failed\n  url: {url}\n  downloads are in-process (ureq + rustls, webpki-roots) — check the network, or set\n  TEBAKO_RUNTIME_MIRROR to a reachable mirror, or TEBAKO_OFFLINE=1 for cache-only mode"
            ),
        );
    }
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
                "SHA256 mismatch for downloaded runtime {asset} — refusing to install or execute\n  expected: {} (from the release index)\n  actual:   {actual}\n  the download was deleted; the cache was not touched",
                expected.to_lowercase()
            ),
        );
    }
    Ok(actual)
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
        });
    }

    let base_raw = releases_base(ctx);
    let base = skip_file_scheme(&base_raw).to_string();
    let local = base_is_local(&base_raw);

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
        // a contract refusal never downloads a byte of the runtime. No
        // readable manifest is the same pre-era signal (no old-path
        // readers; the SHA256SUMS fallback covers checksums only).
        let manifest_url = format!("{base}/v{}/manifest.json", pref.tebako);
        let manifest_text = fetch_manifest_text(&base, local, &pref.tebako, &tmp_dir).ok_or_else(|| {
            ShimError::new(
                EX_TEBAKO_CONTRACT,
                format!(
                    "runtime \"{runtime_ref}\" is pre-era — no readable release manifest at {manifest_url} — refusing to install or execute\n  the release was built by a pre-contract factory; rebuild it with the current tebako-runtime-ruby (spec 18 C2), or pin a runtime that declares its contract"
                ),
            )
        })?;
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
        let exe_url = format!("{base}/v{}/{asset}", pref.tebako);
        let (exe_sha, (diag_m, diag_s)) = expected_checksum(
            &base,
            local,
            &pref.tebako,
            &asset,
            &tmp_dir,
            Some(&manifest_text),
        )?;
        const DIAG: [&str; 5] = [
            "not tried",
            "download failed",
            "read failed",
            "no matching entry",
            "ok",
        ];
        let expected = exe_sha.ok_or_else(|| {
            ShimError::new(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "no checksum for {asset} in the release\n  tried: {base}/v{abi}/manifest.json ({})\n         {base}/v{abi}/SHA256SUMS.txt ({})",
                    DIAG[diag_m], DIAG[diag_s], abi = pref.tebako
                ),
            )
        })?;
        let actual = install_asset(&exe_url, local, &asset, &tmp_dir, &expected)?;
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
        let (image_sha, _) = expected_checksum(
            &base,
            local,
            &pref.tebako,
            &image_asset,
            &tmp_dir,
            Some(&manifest_text),
        )?;
        let has_image = if let Some(image_expected) = image_sha {
            let image_url = format!("{base}/v{}/{image_asset}", pref.tebako);
            let image_actual =
                install_asset(&image_url, local, &image_asset, &tmp_dir, &image_expected)?;
            make_readonly(&tmp_dir.join(&image_asset));
            let _ = std::fs::write(
                tmp_dir.join(format!("{image_asset}.sha256")),
                format!("{image_actual}  {image_asset}\n"),
            );
            let _ = std::fs::write(
                tmp_dir.join(format!("{image_asset}.origin")),
                format!("runtime_ref={runtime_ref}\nurl={image_url}\nsha256={image_actual}\n"),
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
        if let Some((dll_asset, install_as, dll_expected)) = dll_from_manifest(&manifest_text, &asset)
        {
            if install_as.contains('/') || install_as.contains('\\') {
                return fail(
                    EX_TEBAKO_UNAVAILABLE,
                    format!(
                        "release manifest dll facet for {asset} carries an unusable install_as (\"{install_as}\") — the PE name must be a bare file name — refusing to install or execute"
                    ),
                );
            }
            let dll_url = format!("{base}/v{}/{dll_asset}", pref.tebako);
            let dll_actual = install_asset(&dll_url, local, &install_as, &tmp_dir, &dll_expected)?;
            make_readonly(&tmp_dir.join(&install_as));
            let _ = std::fs::write(
                tmp_dir.join(format!("{install_as}.sha256")),
                format!("{dll_actual}  {install_as}\n"),
            );
            let _ = std::fs::write(
                tmp_dir.join(format!("{install_as}.origin")),
                format!("runtime_ref={runtime_ref}\nurl={dll_url}\nsha256={dll_actual}\n"),
            );
        }
        let _ = std::fs::write(tmp_dir.join("sha256"), format!("{actual}  {asset}\n"));
        let _ = std::fs::write(
            tmp_dir.join("origin"),
            format!("runtime_ref={runtime_ref}\nurl={exe_url}\nsha256={actual}\n"),
        );
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
