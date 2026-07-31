//! Runtime resolution (spec 07 §2.2 + spec 05 §5):
//!
//! the entrypoint's `runtime_requirement` → newest COMPATIBLE runtime
//! already cached (no download) → else download the newest compatible →
//! verify → cache. Zero-runtime entrypoints (no `runtime_requirement`)
//! skip this module entirely.
//!
//! The download path mirrors tebako-bootstrap's semantics — per-entry
//! flock (120 s), tmp + rename install, `sha256`/`origin` trust markers,
//! read-only image, manifest.json-primary / SHA256SUMS-fallback checksum
//! extraction, `TEBAKO_RUNTIME_MIRROR` / `TEBAKO_OFFLINE` — reimplemented
//! here rather than linked: the bootstrap crate drags in rnp, and the
//! shim stays pure-Rust + tebako-http.

use std::io::Read;
use std::path::{Path, PathBuf};

use tpkg::RuntimeRequirement;

use crate::config::{self, RuntimePref};
use crate::versions::{self, Constraint};
use crate::{fail, Ctx, ShimError, EX_TEBAKO_IO, EX_TEBAKO_SHA, EX_TEBAKO_UNAVAILABLE};

const DEFAULT_RELEASES_BASE: &str =
    "https://github.com/tamatebako/tebako-runtime-ruby/releases/download";
const LOCK_TIMEOUT_MS: u64 = 120_000;
const LOCK_POLL_MS: u64 = 200;

/// Runtime-package platform string; must match tebako-runtime-ruby's
/// asset naming (mirrors the bootstrap's platform_string).
pub fn platform_string() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x86_64";
    #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
    return "linux-gnu-x86_64";
    #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "aarch64"))]
    return "linux-gnu-arm64";
    #[cfg(all(target_os = "linux", target_env = "musl", target_arch = "x86_64"))]
    return "linux-musl-x86_64";
    #[cfg(all(target_os = "linux", target_env = "musl", target_arch = "aarch64"))]
    return "linux-musl-arm64";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x86_64";
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"),
        all(target_os = "linux", target_env = "gnu", target_arch = "aarch64"),
        all(target_os = "linux", target_env = "musl", target_arch = "x86_64"),
        all(target_os = "linux", target_env = "musl", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    compile_error!("unsupported platform");
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

/// Scan `~/.tebako/runtimes/` for cached runtimes of `engine` on this
/// platform. Lenient by design: malformed entries are invisible to
/// resolution (doctor reports them).
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
        let Some((lang, lv, ver)) = parse_entry_name(&name, platform) else {
            continue;
        };
        if lang != engine {
            continue;
        }
        let exe = entry_dir.join(entry_exe_name(&lv, &ver, platform));
        if !exe.is_file() {
            continue;
        }
        let image_base = format!("tebako-runtime-{ver}-{lv}-{platform}.tfs");
        let image = entry_dir.join(&image_base);
        let image = if image.is_file() && entry_dir.join(format!("{image_base}.sha256")).is_file() {
            Some(image)
        } else {
            None
        };
        out.push(CachedRuntime {
            engine: lang,
            lang_version: lv,
            tebako_version: ver,
            dir: entry_dir,
            exe,
            image,
        });
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

// ---------------------------------------------------------------------
// resolution
// ---------------------------------------------------------------------

#[derive(Debug)]
pub enum RuntimeResolution {
    /// The entrypoint declares no `runtime_requirement` (native /
    /// self-contained): zero runtime payloads mounted.
    Zero,
    Ready(CachedRuntime),
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
    if let Some(hit) = newest_compatible(&cached, &constraint) {
        return Ok(RuntimeResolution::Ready(hit));
    }

    // No compatible cached runtime. The download fallback needs an exact
    // ref: the user's runtime preference for the engine (config.yaml
    // `runtimes:`, spec 07 §4 "runtime preferences") until the runtime
    // registry ships.
    let cfg = config::load_config(&ctx.home)?;
    let pref = cfg.runtimes.get(&req.engine);
    let cached_note = if cached.is_empty() {
        format!("no cached {} runtimes for this platform", req.engine)
    } else {
        format!(
            "cached {} runtimes ({}) do not satisfy \"{}\"{}",
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
            }
        )
    };
    let Some(pref) = pref else {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "no compatible runtime for {} \"{}\": {cached_note}\n  and no runtime preference is configured — set `runtimes: {{{}: {{version: …, tebako: …}}}}` in ~/.tebako/config.yaml, or pre-seed the cache",
                req.engine,
                constraint.source(),
                req.engine
            ),
        );
    };
    if !constraint.matches(&pref.version) {
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
    let rt = download_runtime(&req.engine, pref, ctx)?;
    Ok(RuntimeResolution::Ready(rt))
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
    base.strip_prefix("file://").unwrap_or(base)
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

fn cleanup_tmp_entry(dir: &Path, asset: &str) {
    let _ = std::fs::remove_file(dir.join(asset));
    let _ = std::fs::remove_file(dir.join("manifest.json"));
    let _ = std::fs::remove_file(dir.join("SHA256SUMS.txt"));
    let _ = std::fs::remove_dir(dir);
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
    loop {
        attempts += 1;
        match tebako_http::get(url) {
            Ok(bytes) => return std::fs::write(out, bytes).map_err(|_| ()),
            Err(tebako_http::FetchError::IndexUnavailable(_)) => return Err(()),
            Err(tebako_http::FetchError::DownloadFailed(_)) => {
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

/// The expected checksum for an asset: manifest.json primary,
/// SHA256SUMS.txt fallback (the bootstrap's exact order). Returns the
/// optional sha plus the two diagnostic indices so the caller names the
/// failure itself — an absent entry is data, not an error (the v1-era
/// image rule needs it).
fn expected_checksum(
    base: &str,
    local: bool,
    abi: &str,
    asset: &str,
    tmp_dir: &Path,
) -> Result<(Option<String>, (usize, usize)), ShimError> {
    let manifest_url = format!("{base}/v{abi}/manifest.json");
    let sums_url = format!("{base}/v{abi}/SHA256SUMS.txt");
    let mut expected = None;
    let mut diag_manifest = 1;
    let manifest_tmp = tmp_dir.join("manifest.json");
    if fetch_url(&manifest_url, local, &manifest_tmp).is_ok() {
        diag_manifest = 2;
        if let Ok(text) = std::fs::read_to_string(&manifest_tmp) {
            diag_manifest = 3;
            if let Ok(sha) = sha_from_manifest(&text, asset) {
                diag_manifest = 4;
                expected = Some(sha);
            }
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
    let asset = entry_exe_name(&pref.version, &pref.tebako, platform);
    let exe_path = entry_dir.join(&asset);
    let image_asset = format!(
        "tebako-runtime-{}-{}-{platform}.tfs",
        pref.tebako, pref.version
    );

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
        });
    }

    let tmp_dir = root
        .join("tmp")
        .join(format!("{entry}.{}", std::process::id()));
    cleanup_tmp_entry(&tmp_dir, &asset);
    let result = std::fs::create_dir(&tmp_dir).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", tmp_dir.display()),
        )
    });
    let result = result.and_then(|()| {
        // executable
        let exe_url = format!("{base}/v{}/{asset}", pref.tebako);
        let (exe_sha, (diag_m, diag_s)) =
            expected_checksum(&base, local, &pref.tebako, &asset, &tmp_dir)?;
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
        // when the release index carries it. A pre-image (v1-era) release
        // has no image entry: the runtime's embedded image serves (the
        // bootstrap's graceful-degradation rule), and the exe installs
        // alone rather than hard-failing.
        let (image_sha, _) = expected_checksum(&base, local, &pref.tebako, &image_asset, &tmp_dir)?;
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
        let _ = std::fs::write(tmp_dir.join("sha256"), format!("{actual}  {asset}\n"));
        let _ = std::fs::write(
            tmp_dir.join("origin"),
            format!("runtime_ref={runtime_ref}\nurl={exe_url}\nsha256={actual}\n"),
        );
        Ok((actual, has_image))
    });

    match result {
        Ok((_, has_image)) => {
            if file_exists(&entry_dir) {
                cleanup_tmp_entry(&tmp_dir, &asset);
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
                cleanup_tmp_entry(&tmp_dir, &asset);
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
                exe: exe_path,
                image: has_image.then(|| entry_dir.join(&image_asset)),
                dir: entry_dir,
            })
        }
        Err(e) => {
            cleanup_tmp_entry(&tmp_dir, &asset);
            lock_release(lock);
            Err(e)
        }
    }
}
