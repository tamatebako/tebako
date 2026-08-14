//! macOS loader interposition (spec 22 §2, "Phase 1 delivery"): the
//! boot-head self-insertion.
//!
//! dyld honors `__interpose` tuples only from a DYLIB image — tuples in
//! the main executable are silently ignored, and a dylib dlopen'd after
//! launch stays inert (spec 22, verified empirically) — so at the head
//! of the boot, BEFORE any mount, before the jail, and before the
//! interpreter starts, the driver:
//!
//! 1. writes its embedded micro interpose-dylib (`loader_interpose.c`,
//!    built by `build.rs`) to a content-keyed cache path,
//! 2. prepends that path to `DYLD_INSERT_LIBRARIES` and arms the
//!    `TEBAKO_LOADER_INTERPOSED` sentinel,
//! 3. `execv`s itself with the original argv and environ.
//!
//! The sentinel makes the re-exec fire exactly once; because it precedes
//! all mounting there is no double boot, no partial-mount window, and no
//! launcher-ABI change. ANY failure warns loudly on stderr and the boot
//! CONTINUES uninserted — never a hard failure, never a silent fallback.
//!
//! The dylib's route glue mirrors the ELF delivery — the ruby patch
//! `patches/*/dln_c_loader_interpose.patch` in tamatebako/ruby — and
//! binds the exe's `tebako_fs_*` exports (`exports.txt`) via
//! `-undefined dynamic_lookup`: one VFS context in the process, no third
//! artifact.

use std::ffi::{CString, OsStr, OsString};
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};

use libc::c_char;

/// The once-per-process sentinel: set by the inserting parent, seen by
/// the re-exec'd child (which skips this whole path).
pub(crate) const SENTINEL: &str = "TEBAKO_LOADER_INTERPOSED";

/// The dyld insertion variable the dylib path rides.
pub(crate) const INSERT_VAR: &str = "DYLD_INSERT_LIBRARIES";

/// The embedded micro interpose-dylib (`build.rs` compiles
/// `loader_interpose.c` on macOS — this whole module compiles out
/// elsewhere).
static DYLIB_BYTES: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/libtebako_loader_interpose.dylib"
));

/// Lowercase hex sha256 — the content key (the tebako-resolve
/// `sha256_hex` idiom; layering forbids importing it from there).
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

/// The interpose cache dir under the exec cache root — the same root the
/// `tebako_fs_dlmap2file` exec-cache extractions live under (the system
/// temp dir), tebako-namespaced like every sibling entry there
/// (`tebako-dl-<hex>` and friends).
fn cache_dir(base: &Path) -> PathBuf {
    base.join("tebako-interpose")
}

/// The content-keyed dylib path: `<cache dir>/<sha256>.dylib`.
fn dylib_path(dir: &Path, sha: &str) -> PathBuf {
    dir.join(format!("{sha}.dylib"))
}

/// The `DYLD_INSERT_LIBRARIES` value: our dylib FIRST, any existing
/// value preserved after it (colon-separated; an empty existing value is
/// no value). dyld dedupes a path already listed.
fn compose_insert_libraries(dylib: &Path, existing: Option<OsString>) -> OsString {
    let mut out = dylib.as_os_str().to_os_string();
    if let Some(prev) = existing.filter(|v| !v.is_empty()) {
        out.push(":");
        out.push(prev);
    }
    out
}

/// The insertion decision: the sentinel's PRESENCE (any value) means we
/// are the re-exec'd child — skip entirely.
fn should_insert(sentinel: Option<&OsStr>) -> bool {
    sentinel.is_none()
}

/// Install the dylib at its content-keyed path. A present file IS the
/// right bytes: the name keys the content and the tmp-write + rename is
/// atomic, so a crashed writer can only leave its tmp file behind. A
/// lost rename race is fine — the winner wrote identical bytes.
fn ensure_dylib(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.is_file() {
        return Ok(());
    }
    let dir = path.parent().ok_or_else(|| {
        format!(
            "the interpose cache path '{}' has no parent",
            path.display()
        )
    })?;
    std::fs::create_dir_all(dir).map_err(|e| {
        format!(
            "cannot create the interpose cache dir '{}': {e}",
            dir.display()
        )
    })?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dylib".to_string());
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| {
        format!(
            "cannot write the interpose dylib to '{}': {e}",
            tmp.display()
        )
    })?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        if path.is_file() {
            return Ok(()); // another runtime won the race — same content
        }
        return Err(format!(
            "cannot install the interpose dylib at '{}': {e}",
            path.display()
        ));
    }
    Ok(())
}

/// The insertion proper, split from [`self_insert`] for tests (the
/// cache root is a parameter; the execv never happens on any failure
/// path). Every step BEFORE the execv returns the failure as the reason;
/// the execv itself returns only on failure, which becomes the reason.
fn try_insert(cache_base: &Path, c_argv: *const *const c_char) -> Result<(), String> {
    if c_argv.is_null() {
        return Err("the process argv is unavailable".to_string());
    }
    let sha = sha256_hex(DYLIB_BYTES);
    let dir = cache_dir(cache_base);
    let path = dylib_path(&dir, &sha);
    ensure_dylib(&path, DYLIB_BYTES)?;
    std::env::set_var(
        INSERT_VAR,
        compose_insert_libraries(&path, std::env::var_os(INSERT_VAR)),
    );
    std::env::set_var(SENTINEL, "1");
    let exe = std::env::current_exe()
        .map_err(|e| format!("cannot determine own executable path: {e}"))?;
    let exe_c = CString::new(exe.as_os_str().as_bytes()).map_err(|_| {
        format!(
            "own executable path '{}' contains a NUL byte",
            exe.display()
        )
    })?;
    // execv replaces the process on success — it only ever returns the
    // failure. The original argv/environ ride along unchanged (the two
    // variables above are the only environ delta).
    unsafe { libc::execv(exe_c.as_ptr(), c_argv) };
    Err(format!(
        "execv of '{}' failed: {}",
        exe.display(),
        std::io::Error::last_os_error()
    ))
}

/// The boot-head self-insertion: re-exec once with the interpose dylib
/// inserted, or warn loudly and boot uninserted. `c_argv` is the
/// process's original C argv (NULL-terminated — the FFI contract of the
/// boot entries in [`crate::ffi`]).
pub(crate) fn self_insert(c_argv: *mut *mut c_char) {
    if !should_insert(std::env::var_os(SENTINEL).as_deref()) {
        return; // we are the re-exec'd child — the insertion fired exactly once
    }
    if let Err(reason) = try_insert(&std::env::temp_dir(), c_argv as *const *const c_char) {
        eprintln!("tebako: loader interposition unavailable: {reason}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_matches_the_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_dylib_content_key_is_stable_and_byte_sensitive() {
        assert_eq!(sha256_hex(DYLIB_BYTES), sha256_hex(DYLIB_BYTES));
        assert_ne!(sha256_hex(DYLIB_BYTES), sha256_hex(b"not the dylib"));
    }

    #[test]
    fn cache_paths_are_namespaced_and_content_keyed() {
        let dir = cache_dir(Path::new("/exec-cache"));
        assert_eq!(dir, Path::new("/exec-cache/tebako-interpose"));
        assert_eq!(
            dylib_path(&dir, "ab12cd"),
            Path::new("/exec-cache/tebako-interpose/ab12cd.dylib")
        );
    }

    #[test]
    fn the_insert_list_prepends_and_preserves_any_existing_value() {
        let dylib = Path::new("/cache/tebako-interpose/ab12.dylib");
        assert_eq!(
            compose_insert_libraries(dylib, None),
            OsString::from("/cache/tebako-interpose/ab12.dylib")
        );
        assert_eq!(
            compose_insert_libraries(dylib, Some(OsString::from(""))),
            OsString::from("/cache/tebako-interpose/ab12.dylib"),
            "an empty existing value is no value"
        );
        assert_eq!(
            compose_insert_libraries(dylib, Some(OsString::from("/a/x.dylib:/b/y.dylib"))),
            OsString::from("/cache/tebako-interpose/ab12.dylib:/a/x.dylib:/b/y.dylib")
        );
    }

    #[test]
    fn the_sentinel_decision_is_presence_not_truthiness() {
        assert!(should_insert(None));
        assert!(!should_insert(Some(OsStr::new("1"))));
        assert!(!should_insert(Some(OsStr::new(""))));
    }

    #[test]
    fn ensure_dylib_writes_once_and_keeps_the_first_bytes() {
        let base = std::env::temp_dir().join(format!(
            "tebako-driver-interpose-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let path = dylib_path(&cache_dir(&base), "deadbeef");
        ensure_dylib(&path, b"dylib-bytes").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"dylib-bytes");
        // Content-keyed: a present file is the right bytes — never
        // rewritten.
        ensure_dylib(&path, b"other-bytes").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"dylib-bytes");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_cache_write_failure_is_a_named_reason_never_a_panic() {
        // The cache base is a FILE: the cache dir cannot be created
        // underneath it. try_insert must answer the named reason BEFORE
        // any execv (a reached execv would replace this test process).
        let base = std::env::temp_dir().join(format!(
            "tebako-driver-interpose-fail-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let file = base.join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();
        let arg = CString::new("probe").unwrap();
        let argv: [*mut c_char; 2] = [arg.as_ptr() as *mut c_char, std::ptr::null_mut()];
        let err = try_insert(&file, argv.as_ptr() as *const *const c_char).unwrap_err();
        assert!(
            err.contains("interpose cache dir"),
            "the reason names the failed step: {err}"
        );
        assert!(
            std::env::var_os(SENTINEL).is_none(),
            "a failed insertion arms no sentinel"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
