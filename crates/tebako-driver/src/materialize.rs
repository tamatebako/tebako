//! Declarative boot materialization (spec 22 §4 class R): an image's
//! manifest MAY declare `materialize: [paths]` — absolute in-image paths
//! of regular files a C library must read through its OWN IO (the
//! OpenSSL CA cert is the canonical entry: the path must exist on the
//! host, the interpreter's patched IO never gets asked). The driver
//! extracts the declared paths after the mounts are established, before
//! the interpreter handoff — in both boot shapes (the standalone
//! env-image boot and the `--tebako-image` grammar).
//!
//! A declared cert (the `ssl/cert.pem` convention — the runtime env
//! image's OpenSSL trust store) additionally rides the handoff env: the
//! driver, the single owner of the materialized host path, exports
//! `SSL_CERT_FILE` pointing at the host copy (issue #437). The export is
//! deliberate, never a stomp: an unset/empty value is set, a stale
//! in-VFS spelling under the effective runtime mount root (inherited
//! from an outer tebako process — ruby's patched IO resolves it,
//! libcrypto's native CRT IO cannot) is rewritten, and a real host path
//! is the user's own configuration, which always wins. POSIX images
//! declare no cert, so nothing is set there — the platform default
//! store keeps flowing through the jail.
//!
//! Each declared path `P` lands at
//! `<TEBAKO_EXEC_CACHE>/resources/<image-key>/<P>` (the §6 convention):
//! the exec cache is the namespace every materialization of this boot
//! shares, and the image key is the same segregation idiom as the
//! cache root itself (the store sidecar's sha prefix, else the path key
//! — exec_cache.rs). The copy is whole-file, read-only, and verified
//! (Rule R3):
//!
//! - **Extraction** streams the mounted file to a per-process `part`
//!   staging file, hashing in flight with the tfs-merkle-1 file
//!   construction (`tpkg::merkle::FileHasher`), then re-reads the staged
//!   copy and refuses to install a copy that does not hash to the bytes
//!   the image served (exit 70). The digest record
//!   (`<P>.tfs-digest`) is renamed into place BEFORE the content file,
//!   so a crash never leaves content without its record — and content
//!   without a record is foreign by construction.
//! - **Reuse** (the write-once case): an earlier boot's copy is served
//!   only after it re-hashes to its recorded digest. A mismatch, a
//!   missing record, or a corrupt record is the cache tampered or
//!   corrupt — a named 70 (`EX_TEBAKO_SHA`, spec 06 §4's mismatch code),
//!   never a silently served corruption. The digest's trust chains to
//!   the image itself: verification of the image happens at
//!   fetch/install (spec 09), the record pins the cache copy to what
//!   the image served, and the per-boot rehash pins the copy to the
//!   record.
//! - A declared path absent from the image, or not a regular file, is
//!   the manifest lying — a named 65 (`EX_TEBAKO_MANIFEST`), never a
//!   skipped entry.
//!
//! Concurrent boots of one image race benignly: both stage distinct
//! `part` files and rename identical content (the image determines the
//! bytes), so the last rename wins with the same bytes. The namespace
//! is persistent across boots; stale `part` files are tmp-domain litter
//! the OS reaps.
//!
//! ## The trust bridge (spec 17 §2.3)
//!
//! When the dispatcher conveys a netconfig verdict into the handoff env
//! (`TEBAKO_TLS_PLATFORM_ROOTS` / `TEBAKO_EXTRA_CA` — tebako-http's
//! netconfig owns both spellings; env inheritance is the wire), the
//! declared cert materializes a MERGED bundle instead of the image's
//! bare `cert.pem`: the image roots + the enumerated platform store
//! (rustls-native-certs — the driver is not size-gated) in platform
//! mode, or the image roots + the `extra_ca` PEMs in additive mode. The
//! bundle lives beside the bare copy under the same resources namespace
//! (`ssl/cert-merged-<sha256 of the merge inputs>.pem` — content-keyed,
//! so a changed verdict never reads a stale merge) and carries the same
//! digest record + write-once + per-boot-rehash discipline. Fail closed
//! throughout: a malformed or unreadable `extra_ca` PEM, an
//! unenumerable (or empty) platform store, and a both-vars-set env are
//! named 65s — never a silent fall back to the image bundle alone, and
//! there is no verify-off spelling. With no verdict conveyed nothing
//! changes: the bare cert's bytes and the exported path are the
//! pre-bridge ones exactly.

use std::path::{Path, PathBuf};

use tfs::context::context;
use tpkg::merkle::MerkleDigest;

use crate::driver::{env_var, errno_text, join_mount, DriverError, Env};
use crate::handoff::{ImageSource, ImageSpec};
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST, EX_TEBAKO_SHA};

/// The digest record's suffix next to an extracted file (cache
/// bookkeeping — not a consumption path; spec 22 §6).
const RECORD_SUFFIX: &str = ".tfs-digest";

fn manifest(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_MANIFEST, message.into())
}

fn sha(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_SHA, message.into())
}

fn io(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_IO, message.into())
}

/// The handoff env variable the cert convention exports (issue #437).
const CERT_ENV: &str = "SSL_CERT_FILE";

/// The cert convention: a declared `materialize:` entry naming the
/// image's OpenSSL trust store ends with this path (component-aligned —
/// `/ssl/cert.pem` itself or any deeper spelling such as
/// `/foo/ssl/cert.pem`).
const CERT_SUFFIX: &str = "ssl/cert.pem";

/// Materialize every mounted image's declared `materialize:` paths (see
/// the module doc). Called per boot after the mounts and the jail,
/// before the interpreter handoff. The env image's own declarations
/// come first (the runtime's resources — the cert case), then each
/// payload triple's in order. When a declared cert materialized, the
/// trust bridge (spec 17 §2.3) merges the conveyed netconfig verdict
/// into it and the result's host path rides the handoff env
/// ([`export_cert`]).
pub fn extract(images: &[ImageSpec], env: &dyn Env, runtime_root: &str) -> Result<(), DriverError> {
    let Some(cache) = env_var(env, crate::exec_cache::VAR) else {
        // Unreachable through boot() — exec_cache::export runs first on
        // both paths — but the surface is contractual, never assumed.
        return Err(io(format!(
            "{} is not exported — the exec-cache export runs before materialization at boot",
            crate::exec_cache::VAR
        )));
    };
    // The first declared cert wins: the env image's (the runtime's own
    // trust store) ahead of any payload's, then triple order.
    let mut cert = None;
    if let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") {
        cert = extract_image(&cache, Path::new(&image), runtime_root)?;
    }
    for spec in images {
        let host = match &spec.source {
            ImageSource::File(path, _) => path.clone(),
            ImageSource::OwnSlot(_) => std::env::current_exe()
                .map_err(|e| io(format!("cannot determine own executable path: {e}")))?,
        };
        cert = cert.or(extract_image(&cache, &host, &spec.mount)?);
    }
    if let Some(hit) = cert {
        let cert = bridge_cert(env, &hit, enumerate_platform_roots)?;
        export_cert(env, runtime_root, &cert);
    } else if trust_mode(env)?.is_some() {
        // The POSIX no-op case (no image declares a cert): the verdict
        // has nothing to merge into — the runtime's TLS stacks read the
        // platform defaults, which platform mode already is. Noted, never
        // silent at debug.
        tebako_log::log!(
            tebako_log::Level::Debug,
            "driver",
            "trust verdict conveyed but no image declares a cert — nothing to merge"
        );
    }
    Ok(())
}

/// A materialized declared cert: its host path, and the image's
/// resources dir (the trust bridge's merged bundle lives beside the
/// bare copy).
struct CertHit {
    host: PathBuf,
    dir: PathBuf,
}

/// The spec 17 §2.3 hook on the winning cert: with a conveyed verdict,
/// the merged bundle's host path; without one, the bare cert's
/// (byte-identical pre-bridge behavior).
fn bridge_cert(
    env: &dyn Env,
    hit: &CertHit,
    enumerate_platform: fn() -> Result<Vec<Vec<u8>>, DriverError>,
) -> Result<PathBuf, DriverError> {
    match trust_mode(env)? {
        Some(mode) => merge_bundle(hit, &mode, enumerate_platform),
        None => Ok(hit.host.clone()),
    }
}

/// One mounted image's declarations. No manifest declares nothing
/// (plain images mount fine — the pre-manifest era and the boot-smoke
/// fixture case); a corrupt one is the image lying about its
/// self-description (the shared named 65). Returns the image's first
/// declared cert when one materialized.
fn extract_image(cache: &str, image: &Path, mount: &str) -> Result<Option<CertHit>, DriverError> {
    let Some(manifest) = crate::driver::mounted_manifest_at(mount)? else {
        return Ok(None);
    };
    if manifest.materialize.is_empty() {
        return Ok(None);
    }
    let dir = Path::new(cache)
        .join("resources")
        .join(crate::exec_cache::image_key(image));
    let mut cert = None;
    for declared in &manifest.materialize {
        let target = extract_one(&dir, mount, declared)?;
        if cert.is_none() && is_cert(declared) {
            cert = Some(CertHit {
                host: target,
                dir: dir.clone(),
            });
        }
    }
    Ok(cert)
}

/// The cert convention's name check (see [`CERT_SUFFIX`]).
fn is_cert(declared: &str) -> bool {
    let path = declared.trim_start_matches('/');
    path == CERT_SUFFIX || path.ends_with(&format!("/{CERT_SUFFIX}"))
}

/// Is `value` lexically under `root` (the stale in-VFS spelling check)?
/// Separator-insensitive (`\` reads as `/`); a drive-qualified root (the
/// windows spelling) compares ASCII-case-insensitively — the windows
/// filesystem's own rule — a POSIX root exactly.
fn under_root(value: &str, root: &str) -> bool {
    let value = value.replace('\\', "/");
    let root = root.replace('\\', "/");
    let root = root.trim_end_matches('/');
    if root.is_empty() {
        return false;
    }
    let (value, root) = match crate::driver::vfs_drive(root) {
        Some(_) => (value.to_ascii_lowercase(), root.to_ascii_lowercase()),
        None => (value, root.to_string()),
    };
    value == root || value.starts_with(&format!("{root}/"))
}

/// The `SSL_CERT_FILE` decision, pure (issue #437): the materialized
/// host path when the incoming value is absent (empty reads as absent —
/// the `env_var` filter) or points under the effective runtime mount
/// root (a stale in-VFS spelling); `None` — leave untouched — when the
/// incoming value is a real host path (the user's own configuration
/// always wins).
fn cert_env(current: Option<String>, cert_host: &str, effective_root: &str) -> Option<String> {
    match current {
        None => Some(cert_host.to_string()),
        Some(value) if under_root(&value, effective_root) => Some(cert_host.to_string()),
        Some(_) => None,
    }
}

/// Export the materialized cert's host path per [`cert_env`]'s decision.
fn export_cert(env: &dyn Env, runtime_root: &str, cert_host: &Path) {
    let cert_host = cert_host.to_string_lossy();
    if let Some(value) = cert_env(env_var(env, CERT_ENV), &cert_host, runtime_root) {
        tebako_log::log!(
            tebako_log::Level::Debug,
            "driver",
            "exported {CERT_ENV}={value}"
        );
        env.set_var(CERT_ENV, &value);
    }
}

// ---------------------------------------------------------------------
// The trust bridge (spec 17 §2.3) — see the module doc.
// ---------------------------------------------------------------------

/// The netconfig verdict vars (spec 04's network grammar). tebako-http's
/// netconfig owns the semantics and the export; the driver only READS
/// the spellings the dispatcher conveyed — env inheritance is the wire.
const TRUST_PLATFORM_ENV: &str = "TEBAKO_TLS_PLATFORM_ROOTS";
const TRUST_EXTRA_CA_ENV: &str = "TEBAKO_EXTRA_CA";

/// The conveyed verdict, parsed from the boot env.
#[derive(Debug)]
enum TrustMode {
    /// `TEBAKO_TLS_PLATFORM_ROOTS` — merge in the enumerated OS store.
    Platform,
    /// `TEBAKO_EXTRA_CA` — merge in these PEM files, in list order.
    ExtraCa(Vec<PathBuf>),
}

/// The verdict read, mirroring netconfig's own reads exactly: the
/// platform marker is PRESENCE-based (`var_os().is_some()` — a
/// set-but-empty value still selects the platform store), the extra-ca
/// list is empty-filtered (a set-but-empty value declares nothing).
/// Both at once is the netconfig layer's named error — the driver
/// still fails closed (65) rather than pick a winner: a
/// hand-constructed env could present both, and an ambiguous trust
/// verdict is never resolved by guessing.
fn trust_mode(env: &dyn Env) -> Result<Option<TrustMode>, DriverError> {
    let platform = env.var(TRUST_PLATFORM_ENV).is_some();
    let extra: Vec<PathBuf> = env_var(env, TRUST_EXTRA_CA_ENV)
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    match (platform, extra.is_empty()) {
        (true, false) => Err(manifest(format!(
            "{TRUST_PLATFORM_ENV} and {TRUST_EXTRA_CA_ENV} are both set — the trust verdict is ambiguous (the netconfig layer refuses this combination: the OS verifier cannot take added roots); unset one of them"
        ))),
        (true, true) => Ok(Some(TrustMode::Platform)),
        (false, false) => Ok(Some(TrustMode::ExtraCa(extra))),
        (false, true) => Ok(None),
    }
}

/// The merged bundle (spec 17 §2.3): the image's bare cert + the
/// conveyed trust material, written as ONE PEM file beside the bare
/// copy, content-keyed by the merge inputs
/// (`cert-merged-<sha256(mode ‖ bare bytes ‖ inputs)>.pem`) so a changed
/// verdict never reads a stale merge. An earlier boot's bundle is served
/// only after it re-hashes to its recorded digest (the write-once
/// discipline of every extraction). Fail closed: a malformed or
/// unreadable `extra_ca` PEM, an unenumerable or empty platform store,
/// is a named 65 — never the image bundle alone.
fn merge_bundle(
    hit: &CertHit,
    mode: &TrustMode,
    enumerate_platform: fn() -> Result<Vec<Vec<u8>>, DriverError>,
) -> Result<PathBuf, DriverError> {
    // The bare copy is the verified, read-only materialization — reading
    // it back is the image's bytes by the record's construction.
    let image_pem = std::fs::read(&hit.host).map_err(|e| {
        io(format!(
            "cannot read the materialized cert '{}': {e}",
            hit.host.display()
        ))
    })?;
    use sha2::Digest as _;
    let mut key = sha2::Sha256::new();
    let mut extras: Vec<Vec<u8>> = Vec::new();
    match mode {
        TrustMode::Platform => {
            key.update(b"platform\n");
            for der in enumerate_platform()? {
                key.update(&der);
                extras.push(pem_block(&der));
            }
        }
        TrustMode::ExtraCa(paths) => {
            key.update(b"extra-ca\n");
            for path in paths {
                let bytes = read_extra_ca(path)?;
                key.update(&bytes);
                extras.push(bytes);
            }
        }
    }
    key.update(&image_pem);
    let key = crate::exec_cache::hex(&key.finalize());
    let parent = hit.host.parent().ok_or_else(|| {
        io(format!(
            "the materialized cert '{}' has no parent directory",
            hit.host.display()
        ))
    })?;
    let target = parent.join(format!("cert-merged-{key}.pem"));
    let label = "the merged cert bundle (spec 17 §2.3)";
    if target.exists() {
        verify_recorded(&target, &hit.dir, label)?;
        return Ok(target);
    }
    let mut merged = image_pem;
    if !merged.ends_with(b"\n") {
        merged.push(b'\n');
    }
    for extra in &extras {
        merged.extend_from_slice(extra);
        if !merged.ends_with(b"\n") {
            merged.push(b'\n');
        }
    }
    install_synthesized(&target, &merged, label)?;
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "trust bridge merged {} extra block(s) into {}",
        extras.len(),
        target.display()
    );
    Ok(target)
}

/// The platform trust store, enumerated (spec 17 §2.3's platform mode) —
/// DER blocks in the store's own order. Fail closed: any enumeration
/// error, or an empty store, is a named 65 (an enterprise MITM CA that
/// did not land must never degrade into the image bundle alone).
fn enumerate_platform_roots() -> Result<Vec<Vec<u8>>, DriverError> {
    // rustls-native-certs' env-aware leg honors SSL_CERT_FILE /
    // SSL_CERT_DIR, but this enumeration must name the OS STORE: an
    // inherited SSL_CERT_FILE here is typically the stale in-VFS
    // spelling (the cert convention's own rewrite case), which native
    // IO cannot read — a false enumeration failure. Scrub the pair for
    // the call, restore after; a real user host-path SSL_CERT_FILE wins
    // at export precedence regardless (cert_env), so it is never the
    // merge's input.
    let saved: Vec<(&str, Option<std::ffi::OsString>)> = ["SSL_CERT_FILE", "SSL_CERT_DIR"]
        .into_iter()
        .map(|k| (k, std::env::var_os(k)))
        .collect();
    for (k, _) in &saved {
        std::env::remove_var(k);
    }
    let result = rustls_native_certs::load_native_certs();
    for (k, v) in saved {
        if let Some(v) = v {
            std::env::set_var(k, v);
        }
    }
    if !result.errors.is_empty() {
        return Err(manifest(format!(
            "{TRUST_PLATFORM_ENV} is set but the platform trust store could not be enumerated: {} ({} error(s)) — refusing to boot with the image bundle alone",
            result.errors[0],
            result.errors.len()
        )));
    }
    if result.certs.is_empty() {
        return Err(manifest(format!(
            "{TRUST_PLATFORM_ENV} is set but the platform trust store enumerated zero certificates — refusing to boot with the image bundle alone"
        )));
    }
    Ok(result
        .certs
        .iter()
        .map(|c| c.as_ref().to_vec())
        .collect::<Vec<Vec<u8>>>())
}

/// One DER block's PEM spelling (RFC 7468: `CERTIFICATE` armor,
/// 64-column base64, LF endings — the OpenSSL bundle grammar the image's
/// `cert.pem` already speaks).
fn pem_block(der: &[u8]) -> Vec<u8> {
    use base64ct::Encoding as _;
    let b64 = base64ct::Base64::encode_string(der);
    let mut out = Vec::with_capacity(b64.len() + 64);
    out.extend_from_slice(b"-----BEGIN CERTIFICATE-----\n");
    for line in b64.as_bytes().chunks(64) {
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    out.extend_from_slice(b"-----END CERTIFICATE-----\n");
    out
}

/// Read + validate one `extra_ca` file — tebako-http's parse rule
/// mirrored at the consumption point (the loader plane validates at
/// fetch, the runtime at boot: different moments, same rule): every
/// certificate block parses, and at least one exists. The bytes return
/// verbatim — the merged bundle carries the file's own spelling. A
/// malformed or unreadable file is a named 65 (fail closed).
fn read_extra_ca(path: &Path) -> Result<Vec<u8>, DriverError> {
    let bytes = std::fs::read(path).map_err(|e| {
        manifest(format!(
            "cannot read the extra CA '{}' conveyed by {TRUST_EXTRA_CA_ENV}: {e}",
            path.display()
        ))
    })?;
    let mut count = 0usize;
    let mut cursor = std::io::Cursor::new(&bytes);
    for item in rustls_pemfile::certs(&mut cursor) {
        item.map_err(|e| {
            manifest(format!(
                "the extra CA '{}' conveyed by {TRUST_EXTRA_CA_ENV} does not parse: {e} — expected PEM (-----BEGIN CERTIFICATE-----)",
                path.display()
            ))
        })?;
        count += 1;
    }
    if count == 0 {
        return Err(manifest(format!(
            "no certificate block parses from the extra CA '{}' conveyed by {TRUST_EXTRA_CA_ENV} — expected PEM (-----BEGIN CERTIFICATE-----)",
            path.display()
        )));
    }
    Ok(bytes)
}

/// Install synthesized bytes under the same write-once discipline as an
/// extraction (the trust bridge's merged bundle never rode the VFS):
/// stage, refuse a staged copy that does not hash to the intended bytes
/// (70), then the shared record-first tail.
fn install_synthesized(target: &Path, bytes: &[u8], label: &str) -> Result<(), DriverError> {
    let parent = target.parent().ok_or_else(|| {
        io(format!(
            "the materialization target '{}' has no parent directory",
            target.display()
        ))
    })?;
    std::fs::create_dir_all(parent).map_err(|e| {
        io(format!(
            "cannot create the resources dir '{}': {e}",
            parent.display()
        ))
    })?;
    let tmp = parent.join(format!(
        ".{}.part-{}",
        target
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
        std::process::id()
    ));
    std::fs::write(&tmp, bytes)
        .map_err(|e| io(format!("cannot stage '{}': {e}", tmp.display())))?;
    let want = {
        let mut hasher = tpkg::merkle::FileHasher::new();
        hasher.update(bytes);
        hasher.finish()
    };
    if hash_host_file(&tmp)? != want {
        let _ = std::fs::remove_file(&tmp);
        return Err(sha(format!(
            "the staged copy of {label} does not match the synthesized bytes — installation is not faithful"
        )));
    }
    install_staged(target, &tmp, &want)
}

/// The install tail shared by extraction and synthesized content: the
/// digest record lands FIRST (content without a record is foreign by
/// construction — a crash between the renames leaves a harmless
/// record-only state the next pass overwrites), then the content, then
/// read-only (Rule R3).
fn install_staged(target: &Path, tmp: &Path, served: &MerkleDigest) -> Result<(), DriverError> {
    let parent = target.parent().ok_or_else(|| {
        io(format!(
            "the materialization target '{}' has no parent directory",
            target.display()
        ))
    })?;
    let record = format!("{}\n", tpkg::merkle::render_tree_hash(served));
    let record_tmp = parent.join(format!(
        ".{}{RECORD_SUFFIX}.part-{}",
        target
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
        std::process::id()
    ));
    std::fs::write(&record_tmp, &record).map_err(|e| {
        io(format!(
            "cannot stage the digest record '{}': {e}",
            record_tmp.display()
        ))
    })?;
    std::fs::rename(&record_tmp, record_path(target)).map_err(|e| {
        io(format!(
            "cannot install the digest record for '{}': {e}",
            target.display()
        ))
    })?;
    std::fs::rename(tmp, target).map_err(|e| {
        io(format!(
            "cannot install the materialized '{}': {e}",
            target.display()
        ))
    })?;
    // Rule R3: read-only. After the rename so the staging writes never
    // race the attribute.
    let mut perms = std::fs::metadata(target)
        .map_err(|e| {
            io(format!(
                "cannot stat the materialized '{}': {e}",
                target.display()
            ))
        })?
        .permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(target, perms).map_err(|e| {
        io(format!(
            "cannot make the materialized '{}' read-only: {e}",
            target.display()
        ))
    })?;
    Ok(())
}

/// The record path of an extraction target (`<target>.tfs-digest`).
fn record_path(target: &Path) -> PathBuf {
    let mut p = target.as_os_str().to_os_string();
    p.push(RECORD_SUFFIX);
    PathBuf::from(p)
}

/// The merkle file digest of a host file, streamed.
fn hash_host_file(path: &Path) -> Result<MerkleDigest, DriverError> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| io(format!("cannot read '{}': {e}", path.display())))?;
    let mut hasher = tpkg::merkle::FileHasher::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)
            .map_err(|e| io(format!("cannot read '{}': {e}", path.display())))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish())
}

/// Stream one mounted regular file to a staging file, hashing in
/// flight; the returned digest commits the bytes the image served.
fn stream_out(vfs: &str, tmp: &Path) -> Result<MerkleDigest, DriverError> {
    let mut ctx = context().write().unwrap();
    let fd = ctx.open(vfs, libc::O_RDONLY).map_err(|e| {
        io(format!(
            "cannot read '{vfs}' from the mounted image: {}",
            errno_text(e)
        ))
    })?;
    let result = (|| {
        let mut out = std::fs::File::create(tmp)
            .map_err(|e| io(format!("cannot stage '{}': {e}", tmp.display())))?;
        let mut hasher = tpkg::merkle::FileHasher::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = ctx.read(fd, &mut buf).map_err(|e| {
                io(format!(
                    "cannot read '{vfs}' from the mounted image: {}",
                    errno_text(e)
                ))
            })?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            std::io::Write::write_all(&mut out, &buf[..n])
                .map_err(|e| io(format!("cannot stage '{}': {e}", tmp.display())))?;
        }
        Ok(hasher.finish())
    })();
    let _ = ctx.close(fd);
    result
}

/// Extract one declared path: whole-file, read-only, verified (the
/// module doc's protocol). `dir` is the image's resources namespace.
/// Returns the materialized host path (fresh or verified cache hit).
fn extract_one(dir: &Path, mount: &str, declared: &str) -> Result<PathBuf, DriverError> {
    let vfs = join_mount(mount, declared);
    // Whole files only: a declared-but-absent or non-file path is the
    // manifest lying (Rule R3) — a named 65, never a skipped entry.
    let stat = context().read().unwrap().stat(&vfs).map_err(|e| {
        if e == libc::ENOENT {
            manifest(format!(
                "the image mounted at '{mount}' declares materialize '{declared}' but '{vfs}' is absent from the image — the payload's self-description lies"
            ))
        } else {
            io(format!("cannot stat '{vfs}' in the mounted image: {}", errno_text(e)))
        }
    })?;
    if stat.entry_type != tfs::EntryType::File {
        return Err(manifest(format!(
            "the image mounted at '{mount}' declares materialize '{declared}' but '{vfs}' is not a regular file (materialize lists whole files) — the payload's self-description lies"
        )));
    }
    // The manifest grammar (validated at parse: absolute, no '..'
    // components) makes this join namespace-safe by construction.
    let target = dir.join(declared.trim_start_matches('/'));
    if target.exists() {
        verify_recorded(&target, dir, declared)?;
        return Ok(target);
    }
    let parent = target.parent().ok_or_else(|| {
        io(format!(
            "the materialization target '{}' has no parent directory",
            target.display()
        ))
    })?;
    std::fs::create_dir_all(parent).map_err(|e| {
        io(format!(
            "cannot create the resources dir '{}': {e}",
            parent.display()
        ))
    })?;
    let tmp = parent.join(format!(
        ".{}.part-{}",
        target
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
        std::process::id()
    ));
    let served = stream_out(&vfs, &tmp)?;
    // The staged copy must carry exactly the bytes the image served —
    // the record describes the copy, or the boot never installs it.
    if hash_host_file(&tmp)? != served {
        let _ = std::fs::remove_file(&tmp);
        return Err(sha(format!(
            "the staged copy of '{declared}' does not match the bytes the image served — extraction is not faithful"
        )));
    }
    install_staged(&target, &tmp, &served)?;
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "materialized declared={declared} at={}",
        target.display()
    );
    Ok(target)
}

/// The write-once case: serve an earlier boot's copy only after it
/// re-hashes to its recorded digest. Anything else — a missing/corrupt
/// record, foreign content, a hash mismatch — is the cache tampered or
/// corrupt: a named 70, never a silently served corruption.
fn verify_recorded(target: &Path, dir: &Path, declared: &str) -> Result<(), DriverError> {
    if !target.is_file() {
        return Err(sha(format!(
            "the materialized '{declared}' in the exec cache is not a regular file — the cache is tampered or corrupt; remove '{}' to force re-extraction",
            dir.display()
        )));
    }
    let Ok(record) = std::fs::read_to_string(record_path(target)) else {
        return Err(sha(format!(
            "the materialized '{declared}' in the exec cache carries no digest record — refusing to serve unverified content; remove '{}' to force re-extraction",
            dir.display()
        )));
    };
    let Some(want) = tpkg::merkle::parse_tree_hash(record.trim()) else {
        return Err(sha(format!(
            "the digest record of '{declared}' in the exec cache is corrupt — remove '{}' to force re-extraction",
            dir.display()
        )));
    };
    let got = hash_host_file(target)?;
    if got != want {
        return Err(sha(format!(
            "the materialized '{declared}' in the exec cache fails verification against its recorded digest — the cache is tampered or corrupt; remove '{}' to force re-extraction",
            dir.display()
        )));
    }
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "materialize cache hit declared={declared} at={}",
        target.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn temp(tag: &str) -> PathBuf {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tebako-driver-materialize-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct MapEnv(RefCell<HashMap<String, String>>);

    impl Env for MapEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn set_var(&self, key: &str, value: &str) {
            self.0
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        }
    }

    #[test]
    fn the_record_path_appends_its_suffix() {
        assert_eq!(
            record_path(Path::new("/r/lib/cacert.pem")),
            Path::new("/r/lib/cacert.pem.tfs-digest")
        );
        // A suffix-less name gains one; an existing extension is kept.
        assert_eq!(
            record_path(Path::new("/r/ICUDATA")),
            Path::new("/r/ICUDATA.tfs-digest")
        );
    }

    #[test]
    fn the_record_round_trips_and_detects_tampering() {
        let dir = temp("record");
        let target = dir.join("cert.pem");
        std::fs::write(&target, b"CERT\n").unwrap();
        let digest = hash_host_file(&target).unwrap();
        std::fs::write(
            record_path(&target),
            format!("{}\n", tpkg::merkle::render_tree_hash(&digest)),
        )
        .unwrap();
        // A faithful copy with its record verifies.
        verify_recorded(&target, &dir, "/cert.pem").unwrap();
        // A tampered copy fails verification by name (70).
        std::fs::write(&target, b"FORGED\n").unwrap();
        let err = verify_recorded(&target, &dir, "/cert.pem").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA, "{}", err.message);
        assert!(err.message.contains("/cert.pem"), "{}", err.message);
        // A corrupt record is the same family.
        std::fs::write(record_path(&target), b"not a digest\n").unwrap();
        let err = verify_recorded(&target, &dir, "/cert.pem").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA, "{}", err.message);
        // Content without a record is foreign — never served.
        std::fs::remove_file(record_path(&target)).unwrap();
        let err = verify_recorded(&target, &dir, "/cert.pem").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA, "{}", err.message);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_without_the_exec_cache_export_is_a_named_error() {
        // boot() always exports first; the surface is contractual.
        let env = MapEnv(RefCell::new(HashMap::new()));
        let err = extract(&[], &env, "/__tfs__").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_IO, "{}", err.message);
        assert!(
            err.message.contains(crate::exec_cache::VAR),
            "{}",
            err.message
        );
    }

    #[test]
    fn extract_without_declarations_creates_nothing() {
        let dir = temp("no-decl");
        let env = MapEnv(RefCell::new(HashMap::from([(
            crate::exec_cache::VAR.to_string(),
            dir.to_string_lossy().into_owned(),
        )])));
        // No env image, no payload images: nothing to consult, nothing
        // created — and no cert declared, so the env stays untouched
        // (the POSIX no-op).
        extract(&[], &env, "/__tfs__").unwrap();
        assert!(!dir.join("resources").exists());
        assert!(!env.0.borrow().contains_key(CERT_ENV));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn env_with(pairs: &[(&str, &str)]) -> MapEnv {
        MapEnv(RefCell::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ))
    }

    #[test]
    fn the_cert_convention_names_component_aligned_paths() {
        assert!(is_cert("/ssl/cert.pem"));
        assert!(is_cert("ssl/cert.pem"));
        assert!(is_cert("/foo/ssl/cert.pem"));
        // Not component-aligned — no false positive on a prefix.
        assert!(!is_cert("/my-ssl/cert.pem"));
        assert!(!is_cert("/ssl/cert.pem.bak"));
        assert!(!is_cert("/lib/libcrypto.so"));
    }

    #[test]
    fn under_root_is_separator_and_drive_case_insensitive() {
        // POSIX spellings compare exactly.
        assert!(under_root("/__tfs__/ssl/cert.pem", "/__tfs__"));
        assert!(!under_root("/__TFS__/ssl/cert.pem", "/__tfs__"));
        assert!(!under_root("/etc/ssl/cert.pem", "/__tfs__"));
        // A drive-qualified root (the windows spelling) compares
        // ASCII-case-insensitively and reads `\` as `/`.
        assert!(under_root("A:/t/ssl/cert.pem", "A:/t"));
        assert!(under_root("a:/t/ssl/cert.pem", "A:/t"));
        assert!(under_root("A:\\t\\ssl\\cert.pem", "A:/t"));
        assert!(!under_root("C:/Users/x/cert.pem", "A:/t"));
        // Boundary discipline: the root itself is under itself; a
        // sibling with a shared prefix is not.
        assert!(under_root("A:/t", "A:/t"));
        assert!(!under_root("/__tfs__x/ssl/cert.pem", "/__tfs__"));
        assert!(!under_root("A:/tx/ssl/cert.pem", "A:/t"));
    }

    #[test]
    fn cert_env_sets_rewrites_or_defers() {
        let host = "/tmp/tebako-exec-k/resources/i/ssl/cert.pem";
        // Unset → set to the materialized host path.
        assert_eq!(cert_env(None, host, "/__tfs__").as_deref(), Some(host));
        // Under the effective root → the stale in-VFS spelling rewrites,
        // POSIX and windows spellings alike.
        assert_eq!(
            cert_env(Some("/__tfs__/ssl/cert.pem".to_string()), host, "/__tfs__").as_deref(),
            Some(host)
        );
        assert_eq!(
            cert_env(Some("A:/t/ssl/cert.pem".to_string()), host, "A:/t").as_deref(),
            Some(host)
        );
        // A real host path outside the root is the user's own setting —
        // untouched.
        assert_eq!(
            cert_env(Some("/etc/ssl/cert.pem".to_string()), host, "/__tfs__"),
            None
        );
        assert_eq!(
            cert_env(Some("C:/Users/x/cert.pem".to_string()), host, "A:/t"),
            None
        );
    }

    #[test]
    fn export_cert_applies_the_decision_to_the_env() {
        // Unset → set.
        let env = env_with(&[]);
        export_cert(&env, "/__tfs__", Path::new("/cache/ssl/cert.pem"));
        assert_eq!(
            env.0.borrow().get(CERT_ENV).map(String::as_str),
            Some("/cache/ssl/cert.pem")
        );
        // Empty reads as unset (the env_var filter) → set.
        let env = env_with(&[(CERT_ENV, "")]);
        export_cert(&env, "/__tfs__", Path::new("/cache/ssl/cert.pem"));
        assert_eq!(
            env.0.borrow().get(CERT_ENV).map(String::as_str),
            Some("/cache/ssl/cert.pem")
        );
        // A stale in-VFS spelling inherited from an outer tebako process
        // rewrites to this boot's materialized copy.
        let env = env_with(&[(CERT_ENV, "A:/t/ssl/cert.pem")]);
        export_cert(&env, "A:/t", Path::new("C:/cache/ssl/cert.pem"));
        assert_eq!(
            env.0.borrow().get(CERT_ENV).map(String::as_str),
            Some("C:/cache/ssl/cert.pem")
        );
        // A real host path wins.
        let env = env_with(&[(CERT_ENV, "/etc/ssl/cert.pem")]);
        export_cert(&env, "/__tfs__", Path::new("/cache/ssl/cert.pem"));
        assert_eq!(
            env.0.borrow().get(CERT_ENV).map(String::as_str),
            Some("/etc/ssl/cert.pem")
        );
    }

    // -----------------------------------------------------------------
    // the trust bridge (spec 17 §2.3)
    // -----------------------------------------------------------------

    /// A materialized bare cert fixture: `<tmp>/ssl/cert.pem` + its
    /// CertHit (the resources dir is the temp root).
    fn cert_hit(tag: &str) -> (PathBuf, CertHit) {
        let dir = temp(tag);
        let ssl = dir.join("ssl");
        std::fs::create_dir_all(&ssl).unwrap();
        let host = ssl.join("cert.pem");
        std::fs::write(&host, pem_block(b"image-der-root")).unwrap();
        (dir.clone(), CertHit { host, dir })
    }

    fn never_enumerate() -> Result<Vec<Vec<u8>>, DriverError> {
        panic!("extra_ca mode never enumerates the platform store")
    }

    #[test]
    fn the_trust_mode_read_mirrors_netconfig() {
        // Neither var → no verdict.
        assert!(trust_mode(&env_with(&[])).unwrap().is_none());
        // The platform marker is PRESENCE-based — a set-but-empty value
        // still selects the platform store (netconfig's own read).
        let env = env_with(&[(TRUST_PLATFORM_ENV, "")]);
        assert!(matches!(
            trust_mode(&env).unwrap(),
            Some(TrustMode::Platform)
        ));
        let env = env_with(&[(TRUST_PLATFORM_ENV, "1")]);
        assert!(matches!(
            trust_mode(&env).unwrap(),
            Some(TrustMode::Platform)
        ));
        // The extra-ca list is empty-filtered (set-but-empty declares
        // nothing) and splits on the OS path-list separator.
        let env = env_with(&[(TRUST_EXTRA_CA_ENV, "")]);
        assert!(trust_mode(&env).unwrap().is_none());
        let list = std::env::join_paths([Path::new("/a.pem"), Path::new("/b.pem")])
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let env = env_with(&[(TRUST_EXTRA_CA_ENV, list.as_str())]);
        match trust_mode(&env).unwrap() {
            Some(TrustMode::ExtraCa(paths)) => {
                assert_eq!(
                    paths,
                    vec![PathBuf::from("/a.pem"), PathBuf::from("/b.pem")]
                )
            }
            other => panic!("expected ExtraCa, got {other:?}"),
        }
        // Both at once: fail closed (65), never a guessed winner.
        let env = env_with(&[(TRUST_PLATFORM_ENV, "1"), (TRUST_EXTRA_CA_ENV, "/a.pem")]);
        let err = trust_mode(&env).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{}", err.message);
        assert!(err.message.contains(TRUST_PLATFORM_ENV), "{}", err.message);
        assert!(err.message.contains(TRUST_EXTRA_CA_ENV), "{}", err.message);
    }

    #[test]
    fn pem_block_renders_the_rfc7468_shape() {
        use base64ct::Encoding as _;
        // 48 bytes of DER → 64 base64 chars on one line; 72 bytes → a
        // 64-char line + a 32-char line (the 64-column wrap).
        let pem = String::from_utf8(pem_block(&[7u8; 48])).unwrap();
        assert_eq!(
            pem,
            format!(
                "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
                base64ct::Base64::encode_string(&[7u8; 48])
            ),
            "{pem}"
        );
        let pem = String::from_utf8(pem_block(&[9u8; 72])).unwrap();
        let lines: Vec<&str> = pem.lines().collect();
        assert_eq!(lines[0], "-----BEGIN CERTIFICATE-----");
        assert_eq!(lines[1].len(), 64);
        assert_eq!(lines[2].len(), 32);
        assert_eq!(lines[3], "-----END CERTIFICATE-----");
        // The block round-trips through the parser the additive mode
        // validates with.
        let parsed: Vec<_> = rustls_pemfile::certs(&mut std::io::Cursor::new(&pem))
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].as_ref(), &[9u8; 72][..]);
    }

    #[test]
    fn the_merged_bundle_keys_by_its_inputs_and_verifies() {
        let (dir, hit) = cert_hit("merge-extra");
        let ca = dir.join("corp.pem");
        let ca_bytes = pem_block(b"corp-der-root");
        std::fs::write(&ca, &ca_bytes).unwrap();
        let mode = TrustMode::ExtraCa(vec![ca]);
        let merged = merge_bundle(&hit, &mode, never_enumerate).unwrap();
        // Content-keyed name beside the bare copy.
        let name = merged.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("cert-merged-") && name.ends_with(".pem"),
            "{name}"
        );
        assert_eq!(merged.parent(), hit.host.parent());
        // The merge: image roots first, then the extra CA, verbatim.
        let want = [pem_block(b"image-der-root"), ca_bytes].concat();
        assert_eq!(std::fs::read(&merged).unwrap(), want);
        // Read-only, digest-recorded (Rule R3 + the write-once record).
        assert!(std::fs::metadata(&merged).unwrap().permissions().readonly());
        assert!(record_path(&merged).is_file());
        // Reuse: same inputs → the same path, served after rehash.
        let again = merge_bundle(&hit, &mode, never_enumerate).unwrap();
        assert_eq!(again, merged);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn changed_inputs_key_a_different_bundle() {
        let (dir, hit) = cert_hit("merge-keyed");
        let ca_a = dir.join("a.pem");
        let ca_b = dir.join("b.pem");
        std::fs::write(&ca_a, pem_block(b"corp-a")).unwrap();
        std::fs::write(&ca_b, pem_block(b"corp-b")).unwrap();
        let first = merge_bundle(
            &hit,
            &TrustMode::ExtraCa(vec![ca_a.clone()]),
            never_enumerate,
        )
        .unwrap();
        let second = merge_bundle(&hit, &TrustMode::ExtraCa(vec![ca_b]), never_enumerate).unwrap();
        assert_ne!(first, second);
        // Order is an input too.
        let ab = merge_bundle(
            &hit,
            &TrustMode::ExtraCa(vec![ca_a.clone(), dir.join("b.pem")]),
            never_enumerate,
        )
        .unwrap();
        let ba = merge_bundle(
            &hit,
            &TrustMode::ExtraCa(vec![dir.join("b.pem"), ca_a]),
            never_enumerate,
        )
        .unwrap();
        assert_ne!(ab, ba);
        // Platform mode keys differently from additive mode on the same
        // bytes (the mode tag is an input).
        fn one_root() -> Result<Vec<Vec<u8>>, DriverError> {
            Ok(vec![b"corp-a".to_vec()])
        }
        let platform = merge_bundle(&hit, &TrustMode::Platform, one_root).unwrap();
        assert_ne!(platform, first);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn platform_mode_merges_the_enumerated_store_as_pem() {
        let (dir, hit) = cert_hit("merge-platform");
        fn two_roots() -> Result<Vec<Vec<u8>>, DriverError> {
            Ok(vec![b"der-root-a".to_vec(), b"der-root-b".to_vec()])
        }
        let merged = merge_bundle(&hit, &TrustMode::Platform, two_roots).unwrap();
        let want = [
            pem_block(b"image-der-root"),
            pem_block(b"der-root-a"),
            pem_block(b"der-root-b"),
        ]
        .concat();
        assert_eq!(std::fs::read(&merged).unwrap(), want);
        // Every block parses back as a certificate.
        let text = std::fs::read(&merged).unwrap();
        let count = rustls_pemfile::certs(&mut std::io::Cursor::new(&text)).count();
        assert_eq!(count, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_bridge_fails_closed_on_a_bad_extra_ca() {
        let (dir, hit) = cert_hit("merge-badca");
        // Unreadable → 65, naming the file and the var.
        let missing = dir.join("missing.pem");
        let err = merge_bundle(
            &hit,
            &TrustMode::ExtraCa(vec![missing.clone()]),
            never_enumerate,
        )
        .unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{}", err.message);
        assert!(err.message.contains("missing.pem"), "{}", err.message);
        assert!(err.message.contains(TRUST_EXTRA_CA_ENV), "{}", err.message);
        // No parseable certificate block → 65.
        let junk = dir.join("junk.pem");
        std::fs::write(&junk, b"not a pem\n").unwrap();
        let err = merge_bundle(&hit, &TrustMode::ExtraCa(vec![junk]), never_enumerate).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{}", err.message);
        assert!(
            err.message.contains("no certificate block"),
            "{}",
            err.message
        );
        // Nothing installed on the failure path.
        assert_eq!(
            std::fs::read_dir(hit.host.parent().unwrap())
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("cert-merged-"))
                .count(),
            0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_bridge_fails_closed_on_a_broken_platform_store() {
        let (dir, hit) = cert_hit("merge-badstore");
        fn broken() -> Result<Vec<Vec<u8>>, DriverError> {
            Err(manifest("the store is a maze of twisty passages"))
        }
        let err = merge_bundle(&hit, &TrustMode::Platform, broken).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{}", err.message);
        assert!(err.message.contains("twisty passages"), "{}", err.message);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tampered_merged_bundle_is_a_named_70() {
        let (dir, hit) = cert_hit("merge-tamper");
        let ca = dir.join("corp.pem");
        std::fs::write(&ca, pem_block(b"corp-der-root")).unwrap();
        let mode = TrustMode::ExtraCa(vec![ca]);
        let merged = merge_bundle(&hit, &mode, never_enumerate).unwrap();
        let mut perms = std::fs::metadata(&merged).unwrap().permissions();
        // Deliberate: the tamper case needs the read-only bundle
        // writable again (the same allow as boot.rs's tamper test).
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        std::fs::set_permissions(&merged, perms).unwrap();
        std::fs::write(&merged, b"FORGED\n").unwrap();
        let err = merge_bundle(&hit, &mode, never_enumerate).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA, "{}", err.message);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bridge_cert_defers_without_a_verdict() {
        let (dir, hit) = cert_hit("bridge-none");
        let env = env_with(&[]);
        let got = bridge_cert(&env, &hit, never_enumerate).unwrap();
        assert_eq!(got, hit.host);
        // No merged bundle materialized.
        assert!(!hit.host.parent().unwrap().read_dir().unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("cert-merged-")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_with_a_verdict_but_no_declared_cert_is_a_noop() {
        // The POSIX case: no image declares a cert, so the verdict has
        // nothing to merge into — no export, no error (noted at debug).
        let dir = temp("verdict-no-cert");
        let cache = dir.to_string_lossy().into_owned();
        let env = env_with(&[
            (crate::exec_cache::VAR, cache.as_str()),
            (TRUST_PLATFORM_ENV, "1"),
        ]);
        extract(&[], &env, "/__tfs__").unwrap();
        assert!(!dir.join("resources").exists());
        assert!(!env.0.borrow().contains_key(CERT_ENV));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
