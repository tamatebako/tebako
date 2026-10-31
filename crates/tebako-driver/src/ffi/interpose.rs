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
//! The re-exec'd child (4.) scrubs the micro dylib's entry and the
//! sentinel back OUT of the env before the boot proceeds: the dylib is
//! bound to this exe's own `tebako_fs_*` exports — meaningless in any
//! spawned child and LETHAL in one whose Mach-O slice cannot load it
//! (dyld terminates an arm64e target over an inherited arm64-only
//! insertion; tebako#448). The already-loaded dylib stays live in THIS
//! process — dyld only consults the var at exec.
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

/// The once-per-process sentinel: set by the inserting parent, seen and
/// scrubbed by the re-exec'd child (which skips the insertion itself).
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

/// The running OS's major version (`kern.osproductversion` — 15 for the
/// AMFI-tightened generation). Read through libc, never a shell-out.
fn macos_major() -> Option<u64> {
    let mut buf = [0u8; 32];
    let mut size = buf.len();
    // SAFETY: the named sysctl is a string; buf is sized for it, size is
    // in/out, and no new value is written (the null oldp form).
    let rc = unsafe {
        libc::sysctlbyname(
            c"kern.osproductversion".as_ptr(),
            buf.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(size);
    let s = std::str::from_utf8(&buf[..end]).ok()?;
    s.split('.').next()?.parse().ok()
}

/// Does this exe's code signature carry a CMS blob? The superblob's
/// index holds a CSSLOT_CMS_SIGNATURE (5) entry exactly when the
/// signature is a real one; the linker's adhoc signature is a code
/// directory alone. FAT headers resolve the CURRENT arch's slice;
/// anything unreadable answers true (the insert attempt is the
/// pre-macOS-15 behavior — never a silent degradation there).
fn exe_carries_cms_blob(exe: &Path) -> bool {
    // true ONLY on a positive CMS-slot find — every other shape (no
    // signature, a bare adhoc code directory, an unreadable file) has no
    // CMS blob, and the gate's skip is the safe answer on the AMFI
    // generations (the insert re-exec would be killed, not warned).
    let Ok(bytes) = std::fs::read(exe) else {
        return false;
    };
    let read_u32 = |off: usize, be: bool| -> Option<u32> {
        let b = bytes.get(off..off + 4)?;
        Some(if be {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    let base = match bytes.first_chunk::<4>() {
        // FAT (the big-endian magics): the current arch's slice.
        Some([0xca, 0xfe, 0xba, 0xbe]) | Some([0xca, 0xfe, 0xba, 0xbf]) => {
            let n = read_u32(4, true).unwrap_or(0) as usize;
            let want: u32 = if cfg!(target_arch = "aarch64") {
                0x0100_000C // CPU_TYPE_ARM64
            } else {
                0x0100_0007 // CPU_TYPE_X86_64
            };
            let mut off = None;
            for i in 0..n {
                let e = 8 + i * 20;
                if read_u32(e, true) == Some(want) {
                    off = read_u32(e + 8, true);
                    break;
                }
            }
            match off {
                Some(o) => o as usize,
                None => return false,
            }
        }
        // Thin 64-bit LE (stored CF FA ED FE) and 32-bit LE (stored CE FA
        // ED FE) — the raw bytes, never a transposed read.
        Some([0xcf, 0xfa, 0xed, 0xfe]) | Some([0xce, 0xfa, 0xed, 0xfe]) => 0,
        _ => return false,
    };
    // The mach-o header: ncmds at base+16 (LE), load commands from
    // base+32 (64-bit) or base+28 (32-bit). Find LC_CODE_SIGNATURE
    // (0x1d); its dataoff is the signature blob's file offset.
    let Some(ncmds) = read_u32(base + 16, false) else {
        return false;
    };
    let is64 = bytes[base..base + 4] == [0xcf, 0xfa, 0xed, 0xfe];
    let mut lc = base + if is64 { 32 } else { 28 };
    for _ in 0..ncmds {
        let (Some(cmd), Some(size)) = (read_u32(lc, false), read_u32(lc + 4, false)) else {
            return false;
        };
        if cmd == 0x1d {
            let Some(dataoff) = read_u32(lc + 8, false) else {
                return false;
            };
            let soff = dataoff as usize;
            // An adhoc signature is a BARE code directory (magic
            // 0xFADE0CC0) — no superblob, no CMS slot. A real signature's
            // superblob (magic 0xFADE0CC2) indexes it at slot type 5.
            if read_u32(soff, true) != Some(0xFADE0CC2) {
                return false;
            }
            let Some(count) = read_u32(soff + 8, true) else {
                return false;
            };
            for i in 0..count {
                if let Some(5) = read_u32(soff + 12 + (i as usize) * 8, true) {
                    return true;
                }
            }
            return false;
        }
        lc += size as usize;
    }
    false // no LC_CODE_SIGNATURE at all — unsigned, no CMS blob
}

/// The macOS-15 AMFI gate: with an insertion armed, AMFI kills an
/// exec'd image whose code signature carries no CMS blob — "no CMS
/// blob … Unrecoverable CT signature issue", SIGSEGV, never reaching
/// main (the macos-15 leg's silent death, proven by the unified log).
/// The adhoc link signature has no CMS blob, so an UNSIGNED exe (a dev
/// build, a CI test binary) must not re-exec into it; the boot's
/// documented fallback (warn loudly, boot uninserted) fires instead.
/// The signed production runtime carries the CMS blob and is untouched.
fn amfi_kills_this_exe(exe: &Path) -> bool {
    macos_major().is_some_and(|v| v >= 15) && !exe_carries_cms_blob(exe)
}

/// The cache-dir marker every inserted micro-dylib path carries
/// (`<temp>/tebako-interpose/<sha256>.dylib`). Any tebako version's entry
/// matches — an ancestor runtime's micro is as foreign to a child as ours.
fn is_micro_dylib_entry(entry: &OsStr) -> bool {
    let bytes = entry.as_bytes();
    bytes
        .windows(b"tebako-interpose/".len())
        .any(|w| w == b"tebako-interpose/")
        && bytes.ends_with(b".dylib")
}

/// The re-exec'd child's env cleanup (tebako#448's root fix): the micro
/// dylib is bound to THIS exe's own `tebako_fs_*` exports — meaningless
/// in any child (dynamic_lookup finds nothing there) and LETHAL in one
/// whose selected Mach-O slice cannot load it: dyld TERMINATES an arm64e
/// target over an inherited arm64-only insertion, and the interpreter of
/// an exec'd SCRIPT is exactly such a target (deploy's `o/p/ruby` shim
/// script execs /bin/sh — fat x86_64+arm64e — under the armed var).
///
/// dyld consults DYLD_INSERT_LIBRARIES only at exec: pruning it here
/// unloads nothing in THIS process (the dylib stays loaded, the tuples
/// stay live) but keeps the micro out of every child's env. Any
/// non-tebako entries ride through verbatim. The sentinel is cleared with
/// it — a respawned tebako child self-inserts at its own boot head
/// instead of inheriting a dylib bound to the wrong exe's symbols.
/// Injection (spec 22 §3) later points the var at the self-contained
/// preload shim when the env image declares one — that delivery is
/// unaffected.
fn scrub_insertion_for_children() {
    match scrub_insert_value(std::env::var_os(INSERT_VAR)) {
        Some(v) => std::env::set_var(INSERT_VAR, v),
        None => std::env::remove_var(INSERT_VAR),
    }
    std::env::remove_var(SENTINEL);
}

/// The scrub's pure decision: the insert list minus every micro-dylib
/// entry, or `None` when nothing (else) is listed — the var then comes
/// out of the env entirely rather than riding on as an empty string.
fn scrub_insert_value(existing: Option<OsString>) -> Option<OsString> {
    let v = existing?;
    let entries: Vec<OsString> = v
        .as_bytes()
        .split(|b| *b == b':')
        .filter(|e| !e.is_empty())
        .map(OsStr::from_bytes)
        .filter(|e| !is_micro_dylib_entry(e))
        .map(OsString::from)
        .collect();
    if entries.is_empty() {
        return None;
    }
    let mut out = entries[0].clone();
    for e in &entries[1..] {
        out.push(":");
        out.push(e);
    }
    Some(out)
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
        // We are the re-exec'd child — the insertion fired exactly once.
        // The dylib is bound to THIS exe's exports; it must not ride the
        // env into any child we spawn (tebako#448).
        scrub_insertion_for_children();
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        if amfi_kills_this_exe(&exe) {
            eprintln!(
                "tebako: loader interposition skipped: this exe's adhoc signature carries no CMS blob, and macOS 15+'s AMFI kills an insert re-exec of it — booting uninserted (a signed runtime is unaffected)"
            );
            return;
        }
    }
    if let Err(reason) = try_insert(&std::env::temp_dir(), c_argv as *const *const c_char) {
        eprintln!("tebako: loader interposition unavailable: {reason}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

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
    fn the_cms_parse_reads_this_adhoc_test_binary_as_uncarrying() {
        // The linker's adhoc signature is a bare code directory — the
        // parse must answer NO CMS slot (the first draft's transposed
        // magic read answered true for everything and masked the gate,
        // and the superblob-magic mismatch read it as signed).
        let exe = std::env::current_exe().unwrap();
        assert!(
            !exe_carries_cms_blob(&exe),
            "a cargo-built test binary is adhoc — no CMS slot"
        );
        // The positive half, when the host can sign: a copy signed with
        // a real identity carries the CMS slot.
        if Command::new("codesign").arg("--version").output().is_ok()
            && !Command::new("security")
                .args(["find-identity", "-v", "-p", "codesigning"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("Developer ID"))
                .unwrap_or(false)
        {
            return; // no identity on this host — the negative half stands alone
        }
        let dir = std::env::temp_dir().join(format!("cms-fixture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let signed = dir.join("signed-fixture");
        std::fs::copy(&exe, &signed).unwrap();
        let status = Command::new("codesign")
            .args(["--force", "--sign", "Developer ID"])
            .arg(&signed)
            .status()
            .unwrap();
        if status.success() {
            assert!(
                exe_carries_cms_blob(&signed),
                "a real signature carries the CMS slot"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sentinel_decision_is_presence_not_truthiness() {
        assert!(should_insert(None));
        assert!(!should_insert(Some(OsStr::new("1"))));
        assert!(!should_insert(Some(OsStr::new(""))));
    }

    #[test]
    fn the_micro_entry_marker_matches_only_the_cached_dylib_shape() {
        assert!(is_micro_dylib_entry(OsStr::new(
            "/tmp/tebako-interpose/ab12cd.dylib"
        )));
        assert!(is_micro_dylib_entry(OsStr::new(
            "/var/folders/x/T/tebako-interpose/00ff.dylib"
        )));
        assert!(!is_micro_dylib_entry(OsStr::new("/a/x.dylib")));
        assert!(!is_micro_dylib_entry(OsStr::new(
            // the marker substring alone is not enough — no .dylib tail
            "/tmp/tebako-interpose/README"
        )));
        assert!(!is_micro_dylib_entry(OsStr::new(
            // a .dylib whose dir merely ends in the marker word
            "/opt/tebako-interpose-other/x.dylib"
        )));
        assert!(!is_micro_dylib_entry(OsStr::new("")));
    }

    #[test]
    fn the_scrub_drops_only_the_micro_entries() {
        let micro = "/var/folders/x/T/tebako-interpose/ab12cd.dylib";
        // No var at all → still no var.
        assert_eq!(scrub_insert_value(None), None);
        // Only the micro → the var comes out entirely.
        assert_eq!(scrub_insert_value(Some(OsString::from(micro))), None);
        // Micro first (the self-insertion shape) → the rest rides through.
        assert_eq!(
            scrub_insert_value(Some(OsString::from(format!(
                "{micro}:/a/x.dylib:/b/y.dylib"
            )))),
            Some(OsString::from("/a/x.dylib:/b/y.dylib"))
        );
        // Micros from several ancestor runtimes all go (any cache root,
        // any content key); order of the survivors is preserved.
        let older = "/tmp/tebako-interpose/00ff11.dylib";
        assert_eq!(
            scrub_insert_value(Some(OsString::from(format!(
                "/a/x.dylib:{micro}:{older}:/b/y.dylib"
            )))),
            Some(OsString::from("/a/x.dylib:/b/y.dylib"))
        );
        // An unrelated list is byte-identical on the far side.
        assert_eq!(
            scrub_insert_value(Some(OsString::from("/a/x.dylib:/b/y.dylib"))),
            Some(OsString::from("/a/x.dylib:/b/y.dylib"))
        );
        // Empty segments carry nothing (the compose side never emits
        // them, but an inherited foreign value might).
        assert_eq!(
            scrub_insert_value(Some(OsString::from(format!("{micro}::/a/x.dylib")))),
            Some(OsString::from("/a/x.dylib"))
        );
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
