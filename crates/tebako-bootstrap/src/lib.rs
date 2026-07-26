//! The tebako bootstrap launcher (Rust port of the C99 `tebako-bootstrap.c`
//! in tebako-bootstrap, v0.2.0 contract, launcher ABI v1).
//!
//! Flow (parity with the C++ bootstrap, never mounts images, never links
//! libtfs):
//! 1. find its own executable path;
//! 2. parse the tpkg manifest trailer at EOF (crates/tpkg);
//! 3. check the trailer's launcher_abi against TEBAKO_BOOTSTRAP_LAUNCHER_ABI;
//! 4. parse runtime_ref "type@version;tebako=<abi>[;sha256=<hex>]";
//! 5. resolve the language runtime — shared cache hit, else a fat-package
//!    payload extraction (SHA256-verified against the ;sha256= parameter of
//!    runtime_ref), else a download from the tebako-runtime-ruby releases
//!    (or $TEBAKO_RUNTIME_MIRROR), SHA256-verified against the release
//!    manifest.json (SHA256SUMS.txt fallback), atomically installed
//!    (tmp + rename) under a per-entry lock;
//! 6. exec the runtime, launcher ABI v1:
//! ```text
//! <runtime> --tebako-image <self>:<slot>:<mount> ...
//!           --tebako-entry <argv0> <user args...>
//! ```
//!
//! Downloads are in-process via crates/tebako-http (ureq + rustls with
//! webpki-roots bundled; HTTPS-only, `file://` mirrors; the OS trust
//! store is opt-in via TEBAKO_TLS_PLATFORM_ROOTS) — no curl anywhere
//! (see README for the size audit).

pub mod platform;
pub mod sha;

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use platform::{
    copy_file, exe_suffix, file_exists, flock_acquire, lock_release, make_executable, mkdir_p,
    os_rename, platform_string, remove_file, write_small_file, EntryLock,
};
use sha::sha256_file_hex;

/// The launcher ABI this bootstrap speaks.
pub const LAUNCHER_ABI: u32 = 1;
/// The bootstrap version string (error messages).
pub const VERSION: &str = "0.2.0";

/// Exit codes (documented in README).
pub const EX_TEBAKO_MANIFEST: u8 = 65;
pub const EX_TEBAKO_ABI: u8 = 66;
pub const EX_TEBAKO_RUNTIME_REF: u8 = 67;
pub const EX_TEBAKO_UNAVAILABLE: u8 = 69;
pub const EX_TEBAKO_SHA: u8 = 70;
/// Trailer signature invalid, or an unsigned package in
/// TEBAKO_REQUIRE_SIGNED=1 mode (item 29; named in README).
pub const EX_TEBAKO_SIGNATURE: u8 = 71;
/// The signer key of a v2-signed package is not in the trusted keyring.
pub const EX_TEBAKO_TRUST: u8 = 72;
pub const EX_TEBAKO_IO: u8 = 74;

const DEFAULT_RELEASES_BASE: &str =
    "https://github.com/tamatebako/tebako-runtime-ruby/releases/download";
const LOCK_TIMEOUT_MS: u64 = 120_000;

/// A named bootstrap error: exit code + full message body (stderr gets
/// "tebako-bootstrap: {message}\n").
#[derive(Debug)]
pub struct BootError {
    pub code: u8,
    pub message: String,
}

impl BootError {
    fn new(code: u8, message: String) -> BootError {
        BootError { code, message }
    }
}

fn fail<T>(code: u8, message: String) -> Result<T, BootError> {
    Err(BootError::new(code, message))
}

fn io_fail<T>(message: String) -> Result<T, BootError> {
    fail(EX_TEBAKO_IO, message)
}

// ---------------------------------------------------------------------
// runtime_ref
// ---------------------------------------------------------------------

/// Parsed runtime_ref: `type@version;tebako=<abi>` (trailing `;key=val`
/// parameters tolerated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRef {
    pub r#type: String,
    pub version: String,
    pub abi: String,
}

/// Parse a runtime_ref (exact C++ `parse_runtime_ref` semantics; the
/// components become path/URL parts, so `/\\ \t\r\n` are refused).
pub fn parse_runtime_ref(ref_: &str) -> Result<RuntimeRef, BootError> {
    let err = || {
        BootError::new(
            EX_TEBAKO_RUNTIME_REF,
            format!(
                "cannot parse runtime_ref \"{ref_}\" — expected \"<type>@<version>;tebako=<abi>\""
            ),
        )
    };
    let Some(at) = ref_.find('@') else {
        return Err(err());
    };
    if at == 0 {
        return Err(err());
    }
    let Some(semi) = ref_[at + 1..].find(";tebako=") else {
        return Err(err());
    };
    let semi = at + 1 + semi;
    if semi == at + 1 {
        return Err(err());
    }
    let abiv = &ref_[semi + 8..];
    if abiv.is_empty() {
        return Err(err());
    }
    let ty = &ref_[..at];
    let ver = &ref_[at + 1..semi];
    let abi = abiv.split(';').next().unwrap_or("");
    if ty.is_empty() || ver.is_empty() || abi.is_empty() {
        return Err(err());
    }
    for part in [ty, ver, abi] {
        if part
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ' ' | '\t' | '\r' | '\n'))
        {
            return Err(err());
        }
    }
    Ok(RuntimeRef {
        r#type: ty.to_string(),
        version: ver.to_string(),
        abi: abi.to_string(),
    })
}

/// Extract the trailing `;sha256=<64 lowercase hex>` parameter (the fat
/// package's payload checksum). Err(()) when absent or malformed.
#[allow(clippy::result_unit_err)] // C-style -1 error by design
pub fn runtime_ref_sha256(ref_: &str) -> Result<String, ()> {
    let Some(p) = ref_.find(";sha256=") else {
        return Err(());
    };
    let hex = &ref_[p + 8..];
    if hex.len() < 64 {
        return Err(());
    }
    let hex = &hex[..64];
    if !hex
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(());
    }
    let rest = &ref_[p + 8 + 64..];
    if rest.is_empty() || rest.starts_with(';') {
        Ok(hex.to_lowercase())
    } else {
        Err(())
    }
}

// ---------------------------------------------------------------------
// cache layout
// ---------------------------------------------------------------------

fn cache_root() -> Result<PathBuf, BootError> {
    if let Ok(home) = std::env::var("TEBAKO_HOME") {
        if !home.is_empty() {
            return Ok(PathBuf::from(home));
        }
    }
    #[cfg(windows)]
    {
        if let Ok(home) = std::env::var("LOCALAPPDATA") {
            if !home.is_empty() {
                return Ok(PathBuf::from(home).join("tebako"));
            }
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            if !home.is_empty() {
                return Ok(PathBuf::from(home).join(".tebako"));
            }
        }
        io_fail("cannot determine tebako cache root (set TEBAKO_HOME)".into())
    }
    #[cfg(not(windows))]
    {
        match std::env::var("HOME") {
            Ok(home) if !home.is_empty() => Ok(PathBuf::from(home).join(".tebako")),
            _ => io_fail("cannot determine tebako cache root (set TEBAKO_HOME)".into()),
        }
    }
}

fn offline_mode() -> bool {
    std::env::var("TEBAKO_OFFLINE").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn releases_base() -> String {
    std::env::var("TEBAKO_RUNTIME_MIRROR")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASES_BASE.to_string())
}

fn base_is_local(base: &str) -> bool {
    !(base.starts_with("http://") || base.starts_with("https://"))
}

fn skip_file_scheme(base: &str) -> &str {
    base.strip_prefix("file://").unwrap_or(base)
}

// ---------------------------------------------------------------------
// per-entry install (lock + tmp staging + atomic publish)
// ---------------------------------------------------------------------

struct EntryInstall {
    lock: EntryLock,
    tmp_dir: PathBuf,
    tmp_asset: PathBuf,
}

/// Sweep leftover tmp files from a crashed run (and the tmp dir itself).
fn cleanup_tmp_entry(dir: &Path, asset: &str) {
    let _ = remove_file(&dir.join(asset));
    let _ = remove_file(&dir.join("manifest.json"));
    let _ = remove_file(&dir.join("SHA256SUMS.txt"));
    let _ = remove_file(&dir.join("sha256"));
    let _ = remove_file(&dir.join("origin"));
    let _ = std::fs::remove_dir(dir);
}

/// begin_entry_install: lock + re-check + staging dir.
/// Ok(Some) = lock held, proceed; Ok(None) = entry appeared meanwhile
/// (use the cache); Err = named failure.
fn begin_entry_install(
    root: &Path,
    entry: &str,
    exe_path: &Path,
    asset: &str,
    runtime_ref: &str,
) -> Result<Option<EntryInstall>, BootError> {
    let locks = root.join("locks");
    mkdir_p(&locks).map_err(|e| {
        BootError::new(
            EX_TEBAKO_IO,
            format!(
                "cannot create tebako cache directories under {}: {e}",
                root.display()
            ),
        )
    })?;
    mkdir_p(&root.join("tmp")).map_err(|e| {
        BootError::new(
            EX_TEBAKO_IO,
            format!(
                "cannot create tebako cache directories under {}: {e}",
                root.display()
            ),
        )
    })?;
    mkdir_p(&root.join("runtimes")).map_err(|e| {
        BootError::new(
            EX_TEBAKO_IO,
            format!(
                "cannot create tebako cache directories under {}: {e}",
                root.display()
            ),
        )
    })?;

    let lock_path = locks.join(format!("{entry}.lock"));
    let lock = flock_acquire(&lock_path, LOCK_TIMEOUT_MS).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut {
            BootError::new(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "timed out after {}s waiting for another tebako bootstrap to finish installing \"{runtime_ref}\"\n  lock: {}\n  if no other tebako process is running, remove the stale lock file",
                    LOCK_TIMEOUT_MS / 1000,
                    lock_path.display()
                ),
            )
        } else {
            BootError::new(
                EX_TEBAKO_IO,
                format!("cannot acquire install lock {}: {e}", lock_path.display()),
            )
        }
    })?;

    // re-check under the lock: another process may have installed it.
    if file_exists(exe_path) {
        lock_release(lock);
        return Ok(None);
    }

    let tmp_dir = root
        .join("tmp")
        .join(format!("{entry}.{}", std::process::id()));
    let tmp_asset = tmp_dir.join(asset);
    cleanup_tmp_entry(&tmp_dir, asset);
    if let Err(e) = std::fs::create_dir(&tmp_dir) {
        lock_release(lock);
        return Err(BootError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", tmp_dir.display()),
        ));
    }
    Ok(Some(EntryInstall {
        lock,
        tmp_dir,
        tmp_asset,
    }))
}

/// publish_entry: executable bit, sha256/origin metadata, atomic rename,
/// lock release.
fn publish_entry(
    ins: EntryInstall,
    entry_dir: &Path,
    exe_path: &Path,
    asset: &str,
    sha_hex: &str,
    origin: &str,
) -> Result<PathBuf, BootError> {
    make_executable(&ins.tmp_asset);
    let _ = write_small_file(
        &ins.tmp_dir.join("sha256"),
        &format!("{sha_hex}  {asset}\n"),
    );
    let _ = write_small_file(&ins.tmp_dir.join("origin"), origin);

    let rc = if file_exists(entry_dir) {
        // directory exists without the executable — an interrupted manual
        // edit; never delete user state behind its back.
        fail::<()>(
            EX_TEBAKO_IO,
            format!(
                "cache entry {} exists but is incomplete (missing {asset})\n  remove that directory and run again",
                entry_dir.display()
            ),
        )
        .err()
        .unwrap()
    } else if let Err(e) = os_rename(&ins.tmp_dir, entry_dir) {
        BootError::new(
            EX_TEBAKO_IO,
            format!(
                "cannot install runtime into the cache ({} -> {}): {e}",
                ins.tmp_dir.display(),
                entry_dir.display()
            ),
        )
    } else {
        lock_release(ins.lock);
        return Ok(exe_path.to_path_buf());
    };
    cleanup_tmp_entry(&ins.tmp_dir, asset);
    lock_release(ins.lock);
    Err(rc)
}

// ---------------------------------------------------------------------
// fetch (tebako-http in-process for http(s), copy for local mirrors)
// ---------------------------------------------------------------------

#[allow(clippy::result_unit_err)] // C-style -1 error by design
fn fetch_url(url: &str, local: bool, out: &Path) -> Result<(), ()> {
    if local {
        return copy_file(Path::new(url), out).map_err(|_| ());
    }
    // curl --retry 3 parity: transient failures get three attempts; a
    // missing object (404) fails fast.
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

// ---------------------------------------------------------------------
// release checksum extraction (manifest.json / SHA256SUMS.txt)
// ---------------------------------------------------------------------

/// manifest.json: array of {"filename": ..., "sha256": ...} objects; locate
/// the asset name, bound the enclosing object, read that object's "sha256".
#[allow(clippy::result_unit_err)] // C-style -1 error by design
pub fn sha_from_manifest_json(text: &str, asset: &str) -> Result<String, ()> {
    let needle = format!("\"{asset}\"");
    let p = text.find(&needle).ok_or(())?;
    let obj = text[..p].rfind('{').ok_or(())?;
    let end = text[p..].find('}').map(|e| p + e).ok_or(())?;
    let body = &text[obj..end];
    let k = body.find("\"sha256\"").ok_or(())?;
    let after = &body[k + 8..];
    let after = after.trim_start_matches([':', ' ', '\t', '\n', '\r']);
    if !after.starts_with('"') {
        return Err(());
    }
    let hex = &after[1..];
    let endq = hex.find('"').ok_or(())?;
    let hex = &hex[..endq];
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(hex.to_string())
    } else {
        Err(())
    }
}

/// SHA256SUMS.txt fallback: "<64hex><spaces>[*]<filename>" per line.
#[allow(clippy::result_unit_err)] // C-style -1 error by design
pub fn sha_from_sums(text: &str, asset: &str) -> Result<String, ()> {
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

// ---------------------------------------------------------------------
// fat package: install the runtime payload slot
// ---------------------------------------------------------------------

/// Copy `size` bytes at `offset` of `self_file` into `dst`.
fn extract_payload(self_file: &Path, offset: u64, size: u64, dst: &Path) -> Result<(), i32> {
    let mut f = std::fs::File::open(self_file).map_err(|_| -1)?;
    let fsize = f.metadata().map_err(|_| -1)?.len() as i64;
    if fsize < 0 {
        return Err(-1);
    }
    if offset as i64 > fsize || size as i64 > fsize - offset as i64 {
        return Err(-2);
    }
    f.seek(SeekFrom::Start(offset)).map_err(|_| -1)?;
    let mut out = std::fs::File::create(dst).map_err(|_| -1)?;
    let mut left = size;
    let mut buf = [0u8; 65536];
    while left > 0 {
        let chunk = left.min(buf.len() as u64) as usize;
        if f.read_exact(&mut buf[..chunk]).is_err() {
            let _ = remove_file(dst);
            return Err(-1);
        }
        if out.write_all(&buf[..chunk]).is_err() {
            let _ = remove_file(dst);
            return Err(-1);
        }
        left -= chunk as u64;
    }
    out.flush().map_err(|_| -1)?;
    Ok(())
}

/// The cache layout for one runtime entry.
struct CacheLayout {
    root: PathBuf,
    entry_dir: PathBuf,
    exe_path: PathBuf,
    asset: String,
    entry: String,
}

fn install_payload(
    runtime_ref: &str,
    self_path: &Path,
    slot: &tpkg::Slot,
    layout: &CacheLayout,
) -> Result<PathBuf, BootError> {
    let (root, entry_dir, exe_path, asset, entry) = (
        &layout.root,
        &layout.entry_dir,
        &layout.exe_path,
        &layout.asset,
        &layout.entry,
    );
    let expected = runtime_ref_sha256(runtime_ref).map_err(|_| {
        BootError::new(
            EX_TEBAKO_RUNTIME_REF,
            format!(
                "fat package carries no usable payload checksum — runtime_ref \"{runtime_ref}\"\n  expected \"<type>@<version>;tebako=<abi>;sha256=<64 lowercase hex>\""
            ),
        )
    })?;

    let Some(ins) = begin_entry_install(root, entry, exe_path, asset, runtime_ref)? else {
        return Ok(exe_path.clone());
    };

    if let Err(rc) = extract_payload(self_path, slot.offset, slot.size, &ins.tmp_asset) {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return if rc == -2 {
            fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "corrupt tebako manifest trailer in {} (runtime payload slot outside file bounds) — re-stitch the package",
                    self_path.display()
                ),
            )
        } else {
            io_fail(format!(
                "cannot extract the runtime payload from {}",
                self_path.display()
            ))
        };
    }

    let actual = match sha256_file_hex(&ins.tmp_asset) {
        Ok(a) => a,
        Err(e) => {
            cleanup_tmp_entry(&ins.tmp_dir, asset);
            lock_release(ins.lock);
            return Err(BootError::new(
                EX_TEBAKO_IO,
                format!("cannot hash extracted payload: {e}"),
            ));
        }
    };

    if expected != actual {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return fail(
            EX_TEBAKO_SHA,
            format!(
                "SHA256 mismatch for the runtime payload of {} — refusing to install or execute\n  expected: {expected} (from the package's runtime_ref)\n  actual:   {actual}\n  the cache was not touched",
                self_path.display()
            ),
        );
    }

    let origin = format!(
        "runtime_ref={runtime_ref}\npayload={}\nsha256={actual}\n",
        self_path.display()
    );
    publish_entry(ins, entry_dir, exe_path, asset, &actual, &origin)
}

// ---------------------------------------------------------------------
// chain of trust (item 29): trailer signature + per-slot sha256
// ---------------------------------------------------------------------

fn require_signed_mode() -> bool {
    std::env::var("TEBAKO_REQUIRE_SIGNED").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Append one line to the audit journal ($TEBAKO_HOME/journal.log).
/// Best-effort: journaling never fails the run.
fn journal(home: &Path, line: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(home);
    let path = home.join("journal.log");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{now} {line}");
    }
}

/// SHA-256 of `size` bytes at `offset` of `path` (streaming, one pass).
fn sha256_region(path: &Path, offset: u64, size: u64) -> Result<[u8; 32], BootError> {
    use sha2::Digest;

    let mut f = std::fs::File::open(path).map_err(|e| BootError {
        code: EX_TEBAKO_IO,
        message: format!("cannot open {} for slot hashing: {e}", path.display()),
    })?;
    f.seek(SeekFrom::Start(offset)).map_err(|e| BootError {
        code: EX_TEBAKO_IO,
        message: format!("cannot seek in {}: {e}", path.display()),
    })?;
    let mut h = sha2::Sha256::new();
    let mut buf = [0u8; 65536];
    let mut left = size;
    while left > 0 {
        let chunk = left.min(buf.len() as u64) as usize;
        f.read_exact(&mut buf[..chunk]).map_err(|e| BootError {
            code: EX_TEBAKO_IO,
            message: format!("cannot read {} for slot hashing: {e}", path.display()),
        })?;
        h.update(&buf[..chunk]);
        left -= chunk as u64;
    }
    Ok(h.finalize().into())
}

fn sha256_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for &b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

/// Verify the chain of trust of the package at `self_path` (item 29):
///
/// - v2-signed trailer: the OpenPGP signature must verify against the
///   trusted keyring (`EX_TEBAKO_SIGNATURE` when invalid,
///   `EX_TEBAKO_TRUST` when the signer key is not registered), then every
///   slot's SHA-256 is checked against the trailer's digest array
///   (`EX_TEBAKO_SHA` on mismatch, streaming one pass at install; a
///   trusted-cache marker avoids re-hashing unchanged packages).
/// - v1 (legacy unsigned) trailer: accepted with a loud stderr warning
///   and an audit-journal record — unless TEBAKO_REQUIRE_SIGNED=1, which
///   turns it into a hard `EX_TEBAKO_SIGNATURE` failure.
pub fn verify_chain(self_path: &Path, m: &tpkg::Manifest) -> Result<(), BootError> {
    let home = cache_root()?;
    verify_chain_with_home(self_path, m, &home)
}

/// The home-parameterized core of [`verify_chain`] (tests inject a temp
/// home; production resolves it through `cache_root()`).
pub fn verify_chain_with_home(
    self_path: &Path,
    m: &tpkg::Manifest,
    home: &Path,
) -> Result<(), BootError> {
    let Some(v2) = &m.v2 else {
        // v1 legacy rule (item 29 point 8)
        if require_signed_mode() {
            return fail(
                EX_TEBAKO_SIGNATURE,
                format!(
                    "{} carries an unsigned v1 (legacy) tpkg trailer and TEBAKO_REQUIRE_SIGNED=1 is set — refusing to execute\n  re-bundle the package to sign it, or unset TEBAKO_REQUIRE_SIGNED",
                    self_path.display()
                ),
            );
        }
        eprintln!(
            "tebako-bootstrap: WARNING: {} carries an unsigned v1 (legacy) tpkg trailer\n  — accepted for compatibility; re-bundle the package for integrity protection",
            self_path.display()
        );
        journal(
            &home,
            &format!("event=legacy-v1-accepted package={}", self_path.display()),
        );
        return Ok(());
    };

    // -- signature verification -----------------------------------------
    let keyring = tebako_signer::trusted_keyring_bytes(&home)
        .map_err(|e| BootError::new(EX_TEBAKO_IO, e.to_string()))?;

    let trailer = {
        let mut f = std::fs::File::open(self_path).map_err(|e| BootError {
            code: EX_TEBAKO_IO,
            message: format!(
                "cannot open {} for signature verification: {e}",
                self_path.display()
            ),
        })?;
        let tlen = tpkg::trailer_len(m);
        f.seek(SeekFrom::End(-(tlen as i64)))
            .map_err(|e| BootError {
                code: EX_TEBAKO_IO,
                message: format!("cannot seek trailer of {}: {e}", self_path.display()),
            })?;
        let mut buf = vec![0u8; tlen as usize];
        f.read_exact(&mut buf).map_err(|e| BootError {
            code: EX_TEBAKO_IO,
            message: format!("cannot read trailer of {}: {e}", self_path.display()),
        })?;
        buf
    };
    let region = tpkg::v2_signed_region(&trailer).map_err(|_| BootError {
        code: EX_TEBAKO_MANIFEST,
        message: format!(
            "corrupt tebako manifest trailer in {} (bad v2 extension bounds) — re-stitch the package",
            self_path.display()
        ),
    })?;

    let keyid_hex = v2.signer_keyid_hex();
    let outcome = tebako_signer::verify_detached(&keyring, region, &v2.signature, &v2.signer_keyid)
        .map_err(|e| BootError::new(EX_TEBAKO_SIGNATURE, e.to_string()))?;
    match outcome {
        tebako_signer::VerifyOutcome::Trusted(_) => {
            journal(
                &home,
                &format!(
                    "event=v2-trusted package={} signer={keyid_hex}",
                    self_path.display()
                ),
            );
        }
        tebako_signer::VerifyOutcome::Untrusted(_) => {
            return fail(
                EX_TEBAKO_TRUST,
                format!(
                    "the signer of {} is not in the trusted keyring — refusing to execute\n  signer keyid: {keyid_hex}\n  keyring: {}\n  register the signer's public key with tebako-pkg (TOFU) if you trust it",
                    self_path.display(),
                    tebako_signer::trusted_keyring_path(&home).display()
                ),
            );
        }
        tebako_signer::VerifyOutcome::Invalid(_) => {
            return fail(
                EX_TEBAKO_SIGNATURE,
                format!(
                    "the trailer signature of {} is INVALID — the package or its trailer was tampered with, refusing to execute\n  signer keyid: {keyid_hex}",
                    self_path.display()
                ),
            );
        }
    }

    // -- per-slot sha256 (trusted-cache marker) --------------------------
    let marker_key = {
        use sha2::Digest;
        sha256_hex(&sha2::Sha256::digest(region))
    };
    let marker_path = home
        .join("trusted-cache")
        .join(format!("{marker_key}.marker"));
    let meta = std::fs::metadata(self_path).map_err(|e| BootError {
        code: EX_TEBAKO_IO,
        message: format!("cannot stat {}: {e}", self_path.display()),
    })?;
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Ok(marker) = std::fs::read_to_string(&marker_path) {
        let mut parts = marker.split_whitespace();
        let ok = parts.next() == Some(&*size.to_string())
            && parts.next() == Some(&*mtime.to_string())
            && parts.next() == Some(keyid_hex.as_str());
        if ok {
            journal(
                &home,
                &format!(
                    "event=v2-slots-cache-hit package={} signer={keyid_hex}",
                    self_path.display()
                ),
            );
            return Ok(());
        }
    }

    for (i, slot) in m.slots.iter().enumerate() {
        let digest = sha256_region(self_path, slot.offset, slot.size)?;
        if digest != v2.slot_digests[i] {
            return fail(
                EX_TEBAKO_SHA,
                format!(
                    "SHA256 mismatch for slot {i} ({}) of {} — refusing to install or execute\n  expected: {} (from the signed trailer)\n  actual:   {}\n  the package content was tampered with after signing",
                    slot.mount_point_str().unwrap_or_default(),
                    self_path.display(),
                    sha256_hex(&v2.slot_digests[i]),
                    sha256_hex(&digest)
                ),
            );
        }
    }

    // publish the trusted-cache marker (best-effort)
    let _ = std::fs::create_dir_all(home.join("trusted-cache"));
    if let Err(e) = std::fs::write(&marker_path, format!("{size} {mtime} {keyid_hex}\n")) {
        journal(
            &home,
            &format!(
                "event=trusted-cache-write-failed path={} error={e}",
                marker_path.display()
            ),
        );
    }
    journal(
        &home,
        &format!(
            "event=v2-slots-verified package={} signer={keyid_hex}",
            self_path.display()
        ),
    );
    Ok(())
}

// ---------------------------------------------------------------------
// runtime resolution
// ---------------------------------------------------------------------

fn resolve_runtime(
    runtime_ref: &str,
    rr: &RuntimeRef,
    self_path: &Path,
    m: &tpkg::Manifest,
) -> Result<PathBuf, BootError> {
    let platform = platform_string();
    let layout = CacheLayout {
        root: cache_root()?,
        entry_dir: PathBuf::new(),
        exe_path: PathBuf::new(),
        asset: format!(
            "tebako-runtime-{}-{}-{platform}{}",
            rr.abi,
            rr.version,
            exe_suffix()
        ),
        entry: format!("{}-{}-{}-{platform}", rr.r#type, rr.version, rr.abi),
    };
    let layout = CacheLayout {
        entry_dir: layout.root.join("runtimes").join(&layout.entry),
        exe_path: layout
            .root
            .join("runtimes")
            .join(&layout.entry)
            .join(&layout.asset),
        ..layout
    };
    let (root, entry_dir, exe_path, asset, entry) = (
        &layout.root,
        &layout.entry_dir,
        &layout.exe_path,
        &layout.asset,
        &layout.entry,
    );

    // cache hit
    if file_exists(exe_path) {
        return Ok(exe_path.clone());
    }

    // fat package: the runtime rides along as a payload slot.
    for slot in &m.slots {
        if slot.format_id == tpkg::TPKG_FORMAT_RUNTIME {
            return install_payload(runtime_ref, self_path, slot, &layout);
        }
    }

    let exe_path = &layout.exe_path;
    let base_raw = releases_base();
    let base = skip_file_scheme(&base_raw).to_string();
    let local = base_is_local(&base_raw);
    let asset_url = format!("{base}/v{}/{asset}", rr.abi);
    let manifest_url = format!("{base}/v{}/manifest.json", rr.abi);
    let sums_url = format!("{base}/v{}/SHA256SUMS.txt", rr.abi);

    if offline_mode() {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "cannot resolve runtime \"{runtime_ref}\": not present in the cache and TEBAKO_OFFLINE is set\n  cache entry: {}\n  would fetch: {asset_url}\n  unset TEBAKO_OFFLINE, or set TEBAKO_RUNTIME_MIRROR to a reachable mirror",
                entry_dir.display()
            ),
        );
    }

    let Some(ins) = begin_entry_install(root, entry, exe_path, asset, runtime_ref)? else {
        return Ok(exe_path.clone());
    };

    if fetch_url(&asset_url, local, &ins.tmp_asset).is_err() {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "cannot resolve runtime \"{runtime_ref}\": download failed\n  url: {asset_url}\n  downloads are in-process (ureq + rustls, webpki-roots) — check the network, or set\n  TEBAKO_RUNTIME_MIRROR to a reachable mirror, or TEBAKO_OFFLINE=1 for cache-only mode"
            ),
        );
    }

    // expected checksum: manifest.json primary, SHA256SUMS.txt fallback.
    const DIAG_NAMES: [&str; 5] = [
        "not tried",
        "download failed",
        "read failed",
        "no matching entry",
        "ok",
    ];
    let mut expected: Option<String> = None;
    let mut diag_manifest = 1;
    let manifest_tmp = ins.tmp_dir.join("manifest.json");
    if fetch_url(&manifest_url, local, &manifest_tmp).is_ok() {
        diag_manifest = 2;
        if let Ok(text) = std::fs::read_to_string(&manifest_tmp) {
            diag_manifest = 3;
            if let Ok(sha) = sha_from_manifest_json(&text, asset) {
                diag_manifest = 4;
                expected = Some(sha);
            }
        }
    }
    let mut diag_sums = 0;
    if expected.is_none() {
        diag_sums = 1;
        let sums_tmp = ins.tmp_dir.join("SHA256SUMS.txt");
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
    let Some(expected) = expected else {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "cannot resolve runtime \"{runtime_ref}\": no checksum for {asset} in the release\n  tried: {manifest_url} ({})\n         {sums_url} ({})",
                DIAG_NAMES[diag_manifest], DIAG_NAMES[diag_sums]
            ),
        );
    };

    let actual = match sha256_file_hex(&ins.tmp_asset) {
        Ok(a) => a,
        Err(e) => {
            cleanup_tmp_entry(&ins.tmp_dir, asset);
            lock_release(ins.lock);
            return Err(BootError::new(
                EX_TEBAKO_IO,
                format!(
                    "cannot hash downloaded file {}: {e}",
                    ins.tmp_asset.display()
                ),
            ));
        }
    };

    let expected = expected.to_lowercase();
    if expected != actual {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return fail(
            EX_TEBAKO_SHA,
            format!(
                "SHA256 mismatch for downloaded runtime {asset} — refusing to install or execute\n  expected: {expected} (from {manifest_url})\n  actual:   {actual}\n  the download was deleted; the cache was not touched"
            ),
        );
    }

    let origin = format!("runtime_ref={runtime_ref}\nurl={asset_url}\nsha256={actual}\n");
    publish_entry(ins, entry_dir, exe_path, asset, &actual, &origin)
}

// ---------------------------------------------------------------------
// exec handoff (launcher ABI v1)
// ---------------------------------------------------------------------

#[cfg(unix)]
fn exec_runtime(
    runtime: &Path,
    self_path: &Path,
    m: &tpkg::Manifest,
    argv: &[String],
) -> BootError {
    use std::os::unix::process::CommandExt;

    let mut nargv: Vec<String> = vec![runtime.to_string_lossy().into_owned()];
    for (s, slot) in m.slots.iter().enumerate() {
        if slot.format_id == tpkg::TPKG_FORMAT_RUNTIME {
            continue; // runtime payload: installed into the cache, never mounted
        }
        nargv.push("--tebako-image".to_string());
        nargv.push(format!(
            "{}:{s}:{}",
            self_path.display(),
            slot.mount_point_str().unwrap_or_default()
        ));
    }
    nargv.push("--tebako-entry".to_string());
    nargv.push(
        argv.first()
            .cloned()
            .unwrap_or_else(|| self_path.to_string_lossy().into_owned()),
    );
    nargv.extend(argv.iter().skip(1).cloned());

    let err = std::process::Command::new(runtime).args(&nargv[1..]).exec();
    BootError::new(
        EX_TEBAKO_IO,
        format!("cannot execute runtime {}: {err}", runtime.display()),
    )
}

#[cfg(not(unix))]
fn exec_runtime(
    runtime: &Path,
    _self_path: &Path,
    _m: &tpkg::Manifest,
    _argv: &[String],
) -> BootError {
    // The Windows exec port lands with the windows CI leg (item 22 v1 ships
    // macOS/Linux); fail cleanly rather than misbehave.
    BootError::new(
        EX_TEBAKO_IO,
        format!(
            "cannot execute runtime {}: exec is not implemented on this platform in v1",
            runtime.display()
        ),
    )
}

// ---------------------------------------------------------------------
// main flow
// ---------------------------------------------------------------------

/// Run the bootstrap. `argv` includes argv[0]. Returns Ok on exec failure
/// or a named error with its exit code (exec replaces the process on
/// success on unix).
pub fn run(argv: &[String]) -> Result<std::convert::Infallible, BootError> {
    let self_path = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|_| BootError::new(EX_TEBAKO_IO, "cannot determine own executable path".into()))?;

    let mut f = std::fs::File::open(&self_path).map_err(|e| {
        BootError::new(
            EX_TEBAKO_IO,
            format!("cannot open own executable {}: {e}", self_path.display()),
        )
    })?;
    let m = match tpkg::read_from(&mut f) {
        Ok(m) => m,
        Err(tpkg::TpkgError::NoTrailer) => {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "{} carries no tebako manifest trailer —\n  a bare tebako-bootstrap only becomes runnable when stitched into a\n  three-part package (tebakofs bundle --bootstrap … --image …)",
                    self_path.display()
                ),
            );
        }
        Err(e) => {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "corrupt tebako manifest trailer in {} ({}) — re-stitch the package",
                    self_path.display(),
                    tpkg::strerror(e.code())
                ),
            );
        }
    };

    // Chain of trust (item 29): verify the trailer signature and the
    // per-slot digests before anything is extracted or mounted.
    verify_chain(&self_path, &m)?;

    if m.launcher_abi > LAUNCHER_ABI {
        return fail(
            EX_TEBAKO_ABI,
            format!(
                "package requires launcher ABI {} but this tebako-bootstrap {VERSION} supports ABI {LAUNCHER_ABI} —\n  refresh the runtime via tebako cache, or re-bundle with a current tebako-bootstrap",
                m.launcher_abi
            ),
        );
    }

    let runtime_ref = m.runtime_ref_str().unwrap_or_default().to_string();
    if runtime_ref.is_empty() {
        return fail(
            EX_TEBAKO_RUNTIME_REF,
            "package has no runtime_ref (classic bundle?) — nothing for the bootstrap to resolve"
                .into(),
        );
    }
    let rr = parse_runtime_ref(&runtime_ref)?;

    let runtime = resolve_runtime(&runtime_ref, &rr, &self_path, &m)?;
    Err(exec_runtime(&runtime, &self_path, &m, argv))
}
