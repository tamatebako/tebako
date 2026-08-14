//! Declarative boot materialization (spec 22 §4 class R): an image's
//! manifest MAY declare `materialize: [paths]` — absolute in-image paths
//! of regular files a C library must read through its OWN IO (the
//! OpenSSL CA cert is the canonical entry: the path must exist on the
//! host, the interpreter's patched IO never gets asked). The driver
//! extracts the declared paths after the mounts are established, before
//! the interpreter handoff — in both boot shapes (the standalone
//! env-image boot and the `--tebako-image` grammar).
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

/// Materialize every mounted image's declared `materialize:` paths (see
/// the module doc). Called per boot after the mounts and the jail,
/// before the interpreter handoff. The env image's own declarations
/// come first (the runtime's resources — the cert case), then each
/// payload triple's in order.
pub fn extract(images: &[ImageSpec], env: &dyn Env, runtime_root: &str) -> Result<(), DriverError> {
    let Some(cache) = env_var(env, crate::exec_cache::VAR) else {
        // Unreachable through boot() — exec_cache::export runs first on
        // both paths — but the surface is contractual, never assumed.
        return Err(io(format!(
            "{} is not exported — the exec-cache export runs before materialization at boot",
            crate::exec_cache::VAR
        )));
    };
    if let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") {
        extract_image(&cache, Path::new(&image), runtime_root)?;
    }
    for spec in images {
        let host = match &spec.source {
            ImageSource::File(path, _) => path.clone(),
            ImageSource::OwnSlot(_) => std::env::current_exe()
                .map_err(|e| io(format!("cannot determine own executable path: {e}")))?,
        };
        extract_image(&cache, &host, &spec.mount)?;
    }
    Ok(())
}

/// One mounted image's declarations. No manifest declares nothing
/// (plain images mount fine — the pre-manifest era and the boot-smoke
/// fixture case); a corrupt one is the image lying about its
/// self-description (the shared named 65).
fn extract_image(cache: &str, image: &Path, mount: &str) -> Result<(), DriverError> {
    let Some(manifest) = crate::driver::mounted_manifest_at(mount)? else {
        return Ok(());
    };
    if manifest.materialize.is_empty() {
        return Ok(());
    }
    let dir = Path::new(cache)
        .join("resources")
        .join(crate::exec_cache::image_key(image));
    for declared in &manifest.materialize {
        extract_one(&dir, mount, declared)?;
    }
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
fn extract_one(dir: &Path, mount: &str, declared: &str) -> Result<(), DriverError> {
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
        return verify_recorded(&target, dir, declared);
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
    // The record lands FIRST: content without a record is then foreign
    // by construction (a crash between the renames leaves a harmless
    // record-only state the next extraction overwrites).
    let record = format!("{}\n", tpkg::merkle::render_tree_hash(&served));
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
    std::fs::rename(&record_tmp, record_path(&target)).map_err(|e| {
        io(format!(
            "cannot install the digest record for '{}': {e}",
            target.display()
        ))
    })?;
    std::fs::rename(&tmp, &target).map_err(|e| {
        io(format!(
            "cannot install the materialized '{}': {e}",
            target.display()
        ))
    })?;
    // Rule R3: read-only. After the rename so the staging writes never
    // race the attribute.
    let mut perms = std::fs::metadata(&target)
        .map_err(|e| {
            io(format!(
                "cannot stat the materialized '{}': {e}",
                target.display()
            ))
        })?
        .permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&target, perms).map_err(|e| {
        io(format!(
            "cannot make the materialized '{}' read-only: {e}",
            target.display()
        ))
    })?;
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "materialized declared={declared} at={}",
        target.display()
    );
    Ok(())
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
        // created.
        extract(&[], &env, "/__tfs__").unwrap();
        assert!(!dir.join("resources").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
