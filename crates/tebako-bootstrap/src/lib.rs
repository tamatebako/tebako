//! The tebako bootstrap launcher (Rust port of the C99 `tebako-bootstrap.c`
//! in tebako-bootstrap, v0.2.0 contract, launcher ABI v1).
//!
//! Flow (parity with the C++ bootstrap, never mounts images, never links
//! libtfs):
//! 1. find its own executable path;
//! 2. parse the tpkg manifest trailer at EOF (crates/tpkg);
//! 3. check the trailer's launcher_abi against TEBAKO_BOOTSTRAP_LAUNCHER_ABI;
//! 4. parse runtime_ref "type@version;tebako=<abi>[;image][;sha256=<hex>]"
//!    — when the package carries the L2 package manifest (extension block
//!    type 2, spec 02 §5b / spec 03 §6), argv0 selects the entry whose
//!    runtime_ref is used (exact name match, entries[0] fallback —
//!    per-entry refs for suites and multi-runtime packages; the trailer
//!    field stays for v1-era loaders and block-less packages behave
//!    byte-identically). Resolution only — the handoff argv shape is
//!    unchanged (ABI stays 1);
//! 5. resolve the language runtime — shared cache hit, else a fat-package
//!    payload extraction (SHA256-verified against the ;sha256= parameter of
//!    runtime_ref), else a download from the tebako-runtime-ruby releases
//!    (or $TEBAKO_RUNTIME_MIRROR), SHA256-verified against the release
//!    manifest.json (SHA256SUMS.txt fallback), atomically installed
//!    (tmp + rename) under a per-entry lock;
//!
//! 5b. item 30b: when the ref carries the bare `;image` flag, also resolve
//!    the runtime image (`<asset>.tfs`) — downloaded and SHA256-verified
//!    against the same release index (manifest `image` key primary,
//!    SHA256SUMS line fallback), installed READ-ONLY with
//!    `<image>.sha256`/`<image>.origin` trusted markers, never extracted.
//!    A dll-era windows runtime (tebako-runtime-ruby#40) additionally
//!    resolves its ruby DLL under the same flag — installed read-only next
//!    to the exe under the manifest-declared PE name (`install_as`) with
//!    the same marker discipline, so the PE loader resolves the exe's
//!    imports against the exe's own directory;
//!
//! 6. apply the package's jail (spec 08 §2/§4): the `jail:` block of the
//!    type-2 package manifest REQUESTS host access; the user's TEBAKO_JAIL
//!    env TIGHTENS it — the effective policy (manifest request ∩ user
//!    tightening, user wins, never loosens) is exported to the driver as
//!    TEBAKO_JAIL (+ TEBAKO_JAIL_SOURCE for the audit source, and
//!    TEBAKO_JAIL_JOURNAL pointing at this home's journal.log so the
//!    driver's violations land in the same audit journal). A malformed
//!    policy fails closed (exit 73); no policy = byte-identical legacy
//!    behavior (nothing exported);
//! 7. exec the runtime, launcher ABI v1 (byte-identical handoff; an
//!    image-era run additionally exports TEBAKO_RUNTIME_IMAGE):
//! ```text
//! <runtime> --tebako-image <self>:<slot>:<mount> ...
//!           --tebako-entry <argv0> <user args...>
//! ```
//!
//! Downloads are in-process via crates/tebako-http (ureq + rustls with
//! webpki-roots bundled; HTTPS-only, `file://` mirrors; the OS trust
//! store is opt-in via TEBAKO_TLS_PLATFORM_ROOTS) — no curl anywhere
//! (see README for the size audit).
//!
//! Fetches render the spec 06 §5 progress UX (crates/tebako-term) on
//! stderr — `resolving` → `downloading` + live bar → `verifying sha256` →
//! `installing (locked)` → `installed … and shared by every tebako app on
//! this machine`; a cache hit is one quiet `runtime <ref> (cached)` line.
//! stdout is the payload's and is never touched.

pub mod artifact_info;
pub mod platform;
pub mod sha;

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use platform::{
    copy_file, exe_suffix, file_exists, flock_acquire, lock_release, make_executable,
    make_readonly, mkdir_p, os_rename, platform_string, remove_file, write_small_file, EntryLock,
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
/// The jail policy could not be applied: a malformed TEBAKO_JAIL env spec,
/// or an unusable `jail:` block in the package manifest (spec 08 — the
/// policy is fail-closed: an unparseable policy never silently runs open).
pub const EX_TEBAKO_JAIL: u8 = 73;
pub const EX_TEBAKO_IO: u8 = 74;
/// The runtime declares a bootstrap↔runtime contract this bootstrap does
/// not speak — or none at all (roadmap 45 + spec 18 C2): a pre-era
/// release manifest (no contract set declared), a newer contract era, or
/// a contract_version outside this bootstrap's supported range — fail
/// CLOSED before any download, never a guess.
pub const EX_TEBAKO_CONTRACT: u8 = 75;
/// The explicit install verb (TODO.v2-1/12) was refused
/// (`TPKG_FLAG_NO_INSTALL` — the publisher froze the package) or needs
/// the tebako CLI (the manifest read wants the TFS engine the
/// size-capped bootstrap deliberately does not carry).
pub const EX_TEBAKO_INSTALL: u8 = 76;

/// The bootstrap↔runtime contract this bootstrap speaks (spec 17's
/// handoff grammar — co-mount, `--tebako-image` triples, the env image):
/// **2** in the schema vocabulary (docs/spec/schemas/
/// runtime-manifest.yaml: 1 = spec 06, 2 = spec 17; the roadmap-45
/// interim numbering, 1 = "current", is superseded — the factory
/// declares 2 for its spec-17 runtimes from v0.16.0). A declared
/// contract other than this one is a different generation — refused
/// fail-closed either direction, both numbers named (spec 18 C2/S12).
/// Canonical owner: tebako-resolve::contract::SPOKEN_CONTRACT — the
/// size-capped bootstrap cannot link that crate (gix/reqwest would blow
/// the 3 MiB gate); the value is pinned identical by both sides'
/// refusal-message tests.
pub const SUPPORTED_CONTRACT: u32 = 2;

/// The contract era this bootstrap speaks (spec 18: anything undeclared
/// is pre-era — era 1 — and refused by name, never assumed). Canonical
/// owner: tebako-resolve::contract::SPOKEN_ERA (same mirror pin as
/// [`SUPPORTED_CONTRACT`]).
pub const SPOKEN_ERA: u32 = 2;

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

/// The `;image` flag (item 30b): the runtime is image-era — its
/// filesystem image (`<asset>.tfs`) is a separate artifact resolved
/// alongside the executable. Whole-segment match on the `;`-separated
/// runtime_ref parameters.
pub fn runtime_ref_wants_image(ref_: &str) -> bool {
    ref_.split(';').any(|segment| segment == "image")
}

/// Which entry of the package manifest `argv0` selects (spec 07 §2.0:
/// argv0 is the selector). The selector is argv0's file name (both
/// separators honored on every platform, any `.exe` suffix stripped); an
/// exact entry-name match wins, otherwise the package's primary entry
/// (`entries[0]`) — the standalone-download case, where the binary's
/// file name is arbitrary (version/platform suffixes), always runs the
/// primary command.
pub fn select_entry<'a>(pm: &'a tpkg::PackageManifest, argv0: &str) -> &'a tpkg::PackageEntry {
    let name = argv0.rsplit(['/', '\\']).next().unwrap_or(argv0);
    let name = name.strip_suffix(".exe").unwrap_or(name);
    pm.entries
        .iter()
        .find(|e| e.name == name)
        .unwrap_or(&pm.entries[0])
}

/// The package-manifest selection for this invocation: `None` when the
/// package carries no type-2 block (v1 behavior, byte-identical), else
/// the parsed manifest and the entry `argv0` selects. A corrupt block is
/// a named error; an entry naming a slot the container does not carry is
/// a named error (the package is internally inconsistent).
pub fn package_selection(
    m: &tpkg::Manifest,
    argv0: &str,
) -> Result<Option<(tpkg::PackageManifest, tpkg::PackageEntry)>, BootError> {
    match m.package_manifest() {
        Ok(Some(pm)) => {
            let entry = select_entry(&pm, argv0).clone();
            if entry.slot as usize >= m.slots.len() {
                return fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "package manifest entry \"{}\" names slot {} but the package carries {} slot(s) — the package is internally inconsistent, re-stitch it",
                        entry.name,
                        entry.slot,
                        m.slots.len()
                    ),
                );
            }
            Ok(Some((pm, entry.clone())))
        }
        Ok(None) => Ok(None),
        Err(e) => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "invalid package manifest (extension block type 2): {e} — re-stitch the package"
            ),
        ),
    }
}

/// Which runtime_ref this package resolves against (spec 02 §5b / spec 03
/// §6): when the package carries the L2 package manifest (extension block
/// type 2), the SELECTED entry's `runtime_ref` wins — per-entry refs kill
/// the trailer's 128-byte single-field limit (suites, multi-runtime
/// packages); argv0 selects the entry (exact name match, `entries[0]`
/// fallback — [`select_entry`]). A block-less package reads the trailer
/// field exactly as before (byte-identical behavior). Resolution only —
/// the handoff argv is unchanged in shape (launcher ABI stays 1).
pub fn resolution_runtime_ref(m: &tpkg::Manifest, argv0: &str) -> Result<String, BootError> {
    match package_selection(m, argv0)? {
        Some((_, entry)) => Ok(entry.runtime_ref),
        None => Ok(m.runtime_ref_str().unwrap_or_default().to_string()),
    }
}

// ---------------------------------------------------------------------
// jails (spec 08 §2/§4): manifest request ∩ user tightening = effective
// ---------------------------------------------------------------------

/// The jail the package REQUESTS (spec 08 §4): the `jail:` block of the
/// L2 package manifest (ext block type 2, written by `tebako press
/// --jail`). A block-less package requests nothing — the effective policy
/// is then the user's tightening alone (or nothing: byte-identical legacy
/// behavior).
pub fn package_jail(m: &tpkg::Manifest) -> Result<Option<tpkg::HostJail>, BootError> {
    match m.package_manifest() {
        Ok(Some(pm)) => Ok(pm.jail),
        Ok(None) => Ok(None),
        Err(e) => fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "invalid package manifest (extension block type 2): {e} — re-stitch the package"
            ),
        ),
    }
}

/// The user's tightening from the environment: `TEBAKO_JAIL` in the spec
/// 08 §1 env form. Absent/empty = no tightening. A MALFORMED spec is a
/// named error (EX_TEBAKO_JAIL) — a security policy that cannot be parsed
/// must never silently run open (fail-closed).
pub fn user_jail_from_env() -> Result<Option<tpkg::HostJail>, BootError> {
    let spec = std::env::var("TEBAKO_JAIL").unwrap_or_default();
    if spec.trim().is_empty() {
        return Ok(None);
    }
    tpkg::HostJail::parse_env_spec(&spec)
        .map(Some)
        .map_err(|e| BootError::new(EX_TEBAKO_JAIL, format!("cannot apply the jail policy: {e}")))
}

/// The effective jail exported to the runtime driver: spec, source label
/// (for TEBAKO_JAIL_SOURCE and the audit journal), and the journal file
/// the driver's violations land in (TEBAKO_JAIL_JOURNAL).
pub struct JailEnv {
    pub spec: String,
    pub source: &'static str,
    pub journal: PathBuf,
}

/// Compose the effective jail (spec 08 §2, locked: the package's request ∩
/// the user's tightening — the user TIGHTENS, never loosens) and render it
/// for the handoff. `argv` includes argv[0]; with `argument_files:
/// auto-allowed` the user args naming existing files become read-only
/// grants ("the input file you hand the command is allowed even under
/// deny"). `Ok(None)` when no policy applies — nothing is exported and the
/// run is byte-identical to the pre-jails behavior.
pub fn prepare_jail(
    m: &tpkg::Manifest,
    user: Option<&tpkg::HostJail>,
    argv: &[String],
    home: &Path,
) -> Result<Option<JailEnv>, BootError> {
    let package = package_jail(m)?;
    let Some((jail, source)) = tpkg::jail::effective(package.as_ref(), user) else {
        return Ok(None);
    };
    if jail.is_trivially_open() {
        return Ok(None);
    }
    let arg_files = if jail.argument_files.auto {
        tpkg::jail::resolve_argument_files(&argv[1..])
    } else {
        Vec::new()
    };
    Ok(Some(JailEnv {
        spec: jail.to_env_spec(&arg_files),
        source,
        journal: home.join("journal.log"),
    }))
}

// ---------------------------------------------------------------------
// cache layout
// ---------------------------------------------------------------------

/// The store layout version this bootstrap writes and reads (spec 18
/// C13). Canonical owner: tebako-resolve::store::STORE_LAYOUT_VERSION —
/// the size-capped bootstrap cannot link that crate (gix/reqwest would
/// blow the 3 MiB gate); the value and the refusal/migration message
/// texts are pinned identical by both sides' tests.
const STORE_LAYOUT_VERSION: u32 = 1;

/// spec 18 C13/S41/S42: `~/.tebako/layout-version` — checked once per
/// process (a run has exactly one home). A newer stamp is the upgrade
/// refusal; a pre-versioning store is stamped and the named migration
/// announced on stderr; a new store is born stamped. Mirrors
/// tebako-resolve::store (the canonical owner — see the constant).
fn store_layout_check(root: &Path) -> Result<(), BootError> {
    static DONE: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    let result = DONE.get_or_init(|| store_layout_check_once(root)).clone();
    match result {
        Ok(()) => Ok(()),
        Err(message) => fail(EX_TEBAKO_IO, message),
    }
}

fn store_layout_check_once(root: &Path) -> Result<(), String> {
    let stamp = root.join("layout-version");
    let write_stamp = || std::fs::write(&stamp, format!("{STORE_LAYOUT_VERSION}\n"));
    let migrate = || {
        eprintln!(
            "tebako-bootstrap: note: migrated the tebako store at {} to layout {STORE_LAYOUT_VERSION} (stamped layout-version; the store predates layout versioning — spec 18 C13)",
            root.display()
        );
    };
    match std::fs::read_to_string(&stamp) {
        Ok(text) => {
            let found: u32 = text.trim().parse().map_err(|_| {
                format!(
                    "the tebako store at {} carries an unreadable layout-version ({:?}) — remove the file and rerun, or clear the store",
                    root.display(),
                    text.trim().chars().take(40).collect::<String>()
                )
            })?;
            if found > STORE_LAYOUT_VERSION {
                return Err(format!(
                    "the tebako store at {} was created by a newer tebako (layout {found}); this build speaks layout {STORE_LAYOUT_VERSION} — upgrade tebako",
                    root.display()
                ));
            }
            if found < STORE_LAYOUT_VERSION {
                write_stamp().map_err(|e| format!("{e} (writing {})", stamp.display()))?;
                migrate();
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if root.exists() {
                write_stamp().map_err(|e| format!("{e} (writing {})", stamp.display()))?;
                migrate();
            } else {
                let _ = std::fs::create_dir_all(root);
                write_stamp().map_err(|e| format!("{e} (writing {})", stamp.display()))?;
            }
            Ok(())
        }
        Err(e) => Err(format!("{e} (reading {})", stamp.display())),
    }
}

fn cache_root() -> Result<PathBuf, BootError> {
    let root = cache_root_path()?;
    store_layout_check(&root)?;
    Ok(root)
}

fn cache_root_path() -> Result<PathBuf, BootError> {
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
    // RFC 8089 drive recovery included: `file:///C:/x` strips to `/C:/x`,
    // which is not a windows path — file_path_from_url hands back `C:/x`.
    // Unix remainders pass through unchanged.
    tebako_http::file_path_from_url(base.strip_prefix("file://").unwrap_or(base))
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
    // missing object (404) fails fast. Throttling is a schedule, not a
    // failure: the server's hint is honored, bounded by THROTTLE_ROUNDS.
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

// ---------------------------------------------------------------------
// progress UX (spec 06 §5, locked): phases + bar on stderr, never stdout
// ---------------------------------------------------------------------

/// The quiet cache-hit line: `runtime ruby-3.4.2 (cached)`.
pub fn cached_line(r#type: &str, version: &str) -> String {
    format!("runtime {type}-{version} (cached)")
}

/// The benefit line (spec 06 §5): on completion the user SEES what
/// landed, where it landed, and that it is shared.
pub fn installed_line(name: &str, size: u64, dir: &Path) -> String {
    format!(
        "installed {name} ({}) — cached at {} and shared by every tebako app on this machine",
        tebako_term::human_bytes(size),
        dir.display()
    )
}

/// The progress surface for one bootstrap run: a tebako-term renderer
/// over stderr plus the once-per-run `resolving <runtime_ref>` phase.
struct BootUx {
    prog: tebako_term::Progress<std::io::Stderr>,
    resolving_announced: bool,
}

impl BootUx {
    fn new() -> BootUx {
        BootUx {
            prog: tebako_term::Progress::stderr(),
            resolving_announced: false,
        }
    }

    /// `resolving <runtime_ref>` — once per run, ahead of the first fetch.
    fn resolving(&mut self, runtime_ref: &str) {
        if !self.resolving_announced {
            self.prog.phase(&format!("resolving {runtime_ref}"));
            self.resolving_announced = true;
        }
    }

    /// The quiet cache-hit line.
    fn cached(&mut self, rr: &RuntimeRef) {
        self.prog.line(&cached_line(&rr.r#type, &rr.version));
    }
}

/// Fetch the runtime executable or image with the spec 06 §5 progress:
/// `downloading <asset> (<size>)` plus the live bar (transport-fed via
/// tebako-http's on_progress; one truthful tick for instant local-mirror
/// copies). Same curl --retry 3 parity as fetch_url.
#[allow(clippy::result_unit_err)] // C-style -1 error by design
fn fetch_asset(
    url: &str,
    local: bool,
    out: &Path,
    asset: &str,
    prog: &mut tebako_term::Progress<std::io::Stderr>,
) -> Result<(), ()> {
    if local {
        prog.download_begin(asset);
        return match copy_file(Path::new(url), out) {
            Ok(()) => {
                let n = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
                prog.download_tick(n, Some(n));
                prog.download_end();
                Ok(())
            }
            Err(_) => {
                prog.download_abort();
                Err(())
            }
        };
    }
    let mut attempts = 0;
    let mut throttles = 0;
    loop {
        prog.download_begin(asset);
        let result = {
            let mut tick = |so_far: u64, total: Option<u64>| prog.download_tick(so_far, total);
            tebako_http::get_with_progress(url, Some(&mut tick))
        };
        match result {
            Ok(bytes) => {
                prog.download_end();
                return std::fs::write(out, bytes).map_err(|_| ());
            }
            Err(tebako_http::FetchError::IndexUnavailable(_)) => {
                prog.download_abort();
                return Err(());
            }
            Err(tebako_http::FetchError::Throttled { retry_after, .. }) => {
                prog.download_abort();
                throttles += 1;
                if throttles >= tebako_http::THROTTLE_ROUNDS {
                    return Err(());
                }
                std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
            }
            Err(tebako_http::FetchError::DownloadFailed(_)) => {
                prog.download_abort();
                attempts += 1;
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

/// The integer field `key` of the release-manifest entry for `asset`
/// (the same bounding as [`sha_from_manifest_json`]): `None` when the
/// entry or the field is absent, unparseable, or zero (a zero contract
/// declares nothing). The contract keys are entry-level and precede the
/// nested `image` object in the factory's writer — the bounding assumes
/// no nested object before them.
fn int_field_of_entry(text: &str, asset: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{asset}\"");
    let p = text.find(&needle)?;
    let obj = text[..p].rfind('{')?;
    let end = text[p..].find('}').map(|e| p + e)?;
    let body = &text[obj..end];
    let quoted = format!("\"{key}\"");
    let k = body.find(&quoted)?;
    let after = &body[k + quoted.len()..];
    let after = after.trim_start_matches([':', ' ', '\t', '\n', '\r']);
    let digits: String = after
        .bytes()
        .take_while(|b| b.is_ascii_digit())
        .map(char::from)
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok().filter(|v| *v > 0)
}

/// The `contract_version` of the release-manifest entry for `asset`
/// (roadmap 45): the same bounding as [`sha_from_manifest_json`], reading
/// the object's integer "contract_version" when present. `None` when the
/// field is absent or unparseable — spec 18 reads that as the pre-era
/// signal (see [`contract_gate`]).
pub fn contract_from_manifest_json(text: &str, asset: &str) -> Option<u32> {
    int_field_of_entry(text, asset, "contract_version")
}

/// The `contract_era` of the release-manifest entry for `asset` (spec 18
/// C2): `None` when absent, unparseable, or zero.
pub fn era_from_manifest_json(text: &str, asset: &str) -> Option<u32> {
    int_field_of_entry(text, asset, "contract_era")
}

/// The `mount_root` of the release-manifest entry for `asset` (spec 18
/// C2): `None` when absent or not a non-empty string. Declared, never
/// guessed — the reader invents no fallback value.
pub fn mount_root_from_manifest_json(text: &str, asset: &str) -> Option<String> {
    let needle = format!("\"{asset}\"");
    let p = text.find(&needle)?;
    let obj = text[..p].rfind('{')?;
    let end = text[p..].find('}').map(|e| p + e)?;
    let body = &text[obj..end];
    let k = body.find("\"mount_root\"")?;
    let after = &body[k + 12..];
    let after = after.trim_start_matches([':', ' ', '\t', '\n', '\r']);
    if !after.starts_with('"') {
        return None;
    }
    let inner = &after[1..];
    let endq = inner.find('"')?;
    let value = &inner[..endq];
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Whether the manifest carries an entry for `asset` at all (the gate's
/// "not a contract question" case — the checksum path names a missing
/// entry).
fn contract_entry_present(text: &str, asset: &str) -> bool {
    text.contains(&format!("\"{asset}\""))
}

/// roadmap 45 negotiation failure (exe and image paths share the message).
fn contract_mismatch_error(runtime_ref: &str, declared: u32) -> BootError {
    BootError::new(
        EX_TEBAKO_CONTRACT,
        format!(
            "runtime \"{runtime_ref}\" declares contract_version {declared}, but this tebako-bootstrap speaks contract {SUPPORTED_CONTRACT} — refusing to install or execute\n  the runtime and this bootstrap are from different contract generations; upgrade tebako (or pin an older runtime) and retry"
        ),
    )
}

/// Fail-closed contract gate (roadmap 45 + spec 18 C2/S11/S12), run
/// BEFORE any asset download: the release manifest's entry for `asset`
/// must DECLARE its contract set (contract_era / contract_version /
/// mount_root) — anything undeclared is pre-era (era 1) and refused by
/// name, never assumed (no old-path readers; the manifest itself is the
/// release card, and an entry-less asset is undeclared by definition).
/// A declared era or contract_version newer than this bootstrap speaks
/// is refused with both numbers named. Returns the negotiation error to
/// propagate, or `None` when the entry is acceptable.
fn contract_gate(runtime_ref: &str, manifest_text: &str, asset: &str) -> Option<BootError> {
    let pre_era = |detail: &str| {
        BootError::new(
            EX_TEBAKO_CONTRACT,
            format!(
                "runtime \"{runtime_ref}\" is pre-era — its release manifest entry declares no contract set ({detail}) — refusing to install or execute\n  the release was built by a pre-contract factory; rebuild it with the current tebako-runtime-ruby (spec 18 C2), or pin a runtime that declares its contract"
            ),
        )
    };
    if !contract_entry_present(manifest_text, asset) {
        return Some(pre_era(&format!("no entry for {asset}")));
    }
    let era = era_from_manifest_json(manifest_text, asset);
    let contract = contract_from_manifest_json(manifest_text, asset);
    let mount_root = mount_root_from_manifest_json(manifest_text, asset);
    let mut missing: Vec<&str> = Vec::new();
    if era.is_none() {
        missing.push("contract_era");
    }
    if contract.is_none() {
        missing.push("contract_version");
    }
    if mount_root.is_none() {
        missing.push("mount_root");
    }
    if !missing.is_empty() {
        return Some(pre_era(&format!("missing: {}", missing.join(", "))));
    }
    let era = era.unwrap_or(0);
    if era > SPOKEN_ERA {
        return Some(BootError::new(
            EX_TEBAKO_CONTRACT,
            format!(
                "runtime \"{runtime_ref}\" speaks contract era {era}, but this tebako-bootstrap speaks era {SPOKEN_ERA} — refusing to install or execute\n  upgrade tebako (or pin an older runtime) and retry"
            ),
        ));
    }
    let declared = contract.unwrap_or(0);
    if declared != SUPPORTED_CONTRACT {
        return Some(contract_mismatch_error(runtime_ref, declared));
    }
    None
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

/// manifest.json, the image entry (item 30b): locate "<image_asset>"
/// (the .tfs filename — it appears only inside the package entry's
/// additive `image` key) and read the NEXT "sha256" after it.
#[allow(clippy::result_unit_err)] // C-style -1 error by design
pub fn sha_from_manifest_image(text: &str, image_asset: &str) -> Result<String, ()> {
    let needle = format!("\"{image_asset}\"");
    let p = text.find(&needle).ok_or(())?;
    let body = &text[p + needle.len()..];
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

/// manifest.json, the dll facet (tebako-runtime-ruby#40): locate
/// `"<dll_asset>"` (the .dll filename — it appears only inside the exe
/// entry's additive `dll` key) and read the NEXT `"install_as"` value,
/// bounded by the dll object's closing brace (never a later entry's
/// key). The PE name is declared, never derived — the factory's
/// RubyVersion#msys_dll_name is its single owner.
#[allow(clippy::result_unit_err)] // C-style -1 error by design
pub fn dll_install_as_from_manifest(text: &str, dll_asset: &str) -> Result<String, ()> {
    let needle = format!("\"{dll_asset}\"");
    let p = text.find(&needle).ok_or(())?;
    let rest = &text[p + needle.len()..];
    let close = rest.find('}').ok_or(())?;
    let body = &rest[..close];
    let k = body.find("\"install_as\"").ok_or(())?;
    let after = &body[k + 12..];
    let after = after.trim_start_matches([':', ' ', '\t', '\n', '\r']);
    if !after.starts_with('"') {
        return Err(());
    }
    let inner = &after[1..];
    let endq = inner.find('"').ok_or(())?;
    let value = &inner[..endq];
    if value.is_empty() {
        Err(())
    } else {
        Ok(value.to_string())
    }
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
    /// The asset name without the platform executable suffix; the runtime
    /// image is `<asset_base>.tfs` (item 30b).
    asset_base: String,
    entry: String,
}

fn install_payload(
    runtime_ref: &str,
    rr: &RuntimeRef,
    self_path: &Path,
    slot: &tpkg::Slot,
    layout: &CacheLayout,
    ux: &mut BootUx,
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
        ux.cached(rr);
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

// ---------------------------------------------------------------------
// chain of trust (item 29): trailer signature + per-slot sha256
// ---------------------------------------------------------------------

/// The tamatebako release root-of-trust fingerprint, compile-time
/// embedded in the bootstrap (item 29 point 1: the root fingerprint is
/// published on tebako.org AND embedded in the artifacts). Empty until
/// the release key ceremony fills it at release time;
/// `TEBAKO_TRUSTED_ROOT` (a fingerprint) extends/overrides it for
/// development.
pub const EMBEDDED_ROOT_FINGERPRINT: &str = "";

/// A trusted root: fingerprint plus optionally-bundled public key bytes
/// (an env override may point at an armored public key file).
#[cfg(feature = "openpgp-verify")]
struct TrustedRoot {
    fingerprint: String,
    public_key: Option<Vec<u8>>,
}

/// The trusted roots in effect: the embedded root (fingerprint only — its
/// public key must reach the trusted keyring via the normal channel), and
/// the `TEBAKO_TRUSTED_ROOT` override — a fingerprint (public key then
/// expected in the trusted keyring) or a path to an armored public key
/// file. A fingerprint never suffices on its own: the trailer signature
/// is always cryptographically verified against the root's public key.
#[cfg(feature = "openpgp-verify")]
fn trusted_roots() -> Vec<TrustedRoot> {
    let mut roots = Vec::new();
    if !EMBEDDED_ROOT_FINGERPRINT.is_empty() {
        roots.push(TrustedRoot {
            fingerprint: EMBEDDED_ROOT_FINGERPRINT.to_uppercase(),
            public_key: None,
        });
    }
    if let Ok(v) = std::env::var("TEBAKO_TRUSTED_ROOT") {
        let v = v.trim();
        if !v.is_empty() {
            let path = Path::new(v);
            if path.is_file() {
                if let Ok(bytes) = std::fs::read(path) {
                    let fp = (|| {
                        let ctx = rnp::Context::new().ok()?;
                        ctx.load_keys(rnp::KeyringFormat::Gpg, &bytes, rnp::LoadSaveFlags::PUBLIC)
                            .ok()?;
                        let mut ids = ctx.identifiers(rnp::IdentifierKind::Fingerprint).ok()?;
                        ids.next()
                    })();
                    if let Some(fp) = fp {
                        roots.push(TrustedRoot {
                            fingerprint: fp.to_uppercase(),
                            public_key: Some(bytes),
                        });
                    }
                }
            } else {
                roots.push(TrustedRoot {
                    fingerprint: v.to_uppercase(),
                    public_key: None,
                });
            }
        }
    }
    roots
}

/// Forward trust through the successor-statement chain (item 29
/// rotation): `$TEBAKO_HOME/keyring/successors/` holds successor
/// statements (`*.asc`) and the successor public keys (`<fingerprint>.pub`)
/// they authorize. When every statement verifies from a trusted root to
/// `signer_fp`, the signer's public key is registered in the trusted
/// keyring and the trailer signature is re-verified against it.
/// Returns Ok(true) only when the final signature verification is Trusted.
#[cfg(feature = "openpgp-verify")]
fn forward_trust_from_successors(
    home: &Path,
    roots: &[TrustedRoot],
    signer_fp: &str,
    region: &[u8],
    signature: &[u8],
    signer_keyid: &[u8; 8],
) -> Result<bool, BootError> {
    let dir = home.join("keyring").join("successors");
    let rd = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(BootError::new(
                EX_TEBAKO_IO,
                format!("cannot read {}: {e}", dir.display()),
            ))
        }
    };

    let mut statements: Vec<Vec<u8>> = Vec::new();
    let mut extended: Vec<u8> = tebako_signer::trusted_keyring_bytes(home)
        .map_err(|e| BootError::new(EX_TEBAKO_IO, e.to_string()))?;
    for root in roots {
        if let Some(pk) = &root.public_key {
            extended.extend_from_slice(pk);
        }
    }
    for entry in rd.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if name.ends_with(".asc") {
            if let Ok(bytes) = std::fs::read(&path) {
                statements.push(bytes);
            }
        } else if name.ends_with(".pub") {
            if let Ok(bytes) = std::fs::read(&path) {
                // the trusted keyring is binary; successor public keys may
                // be armored — dearmor before concatenating
                let bytes = rnp::dearmor_bytes(&bytes).unwrap_or(bytes);
                extended.extend_from_slice(&bytes);
            }
        }
    }
    if statements.is_empty() || roots.is_empty() {
        return Ok(false);
    }

    for root in roots {
        // the signer may be any link in the rotation chain, not only its
        // tip: walk the verified path once and test membership (statement
        // order in the directory is irrelevant — the walk matches
        // predecessors; a broken link simply ends the reachable path).
        let path = tebako_signer::successor_chain_path(&root.fingerprint, &extended, &statements);
        if !path.iter().any(|fp| fp.eq_ignore_ascii_case(signer_fp)) {
            continue;
        }
        // rotation proven: register the signer's public key
        // (distributed alongside the statements) and re-verify
        let pub_path = dir.join(format!("{}.pub", signer_fp.to_uppercase()));
        let Ok(public_key) = std::fs::read(&pub_path) else {
            continue;
        };
        tebako_signer::register_trusted(home, &public_key)
            .map_err(|e| BootError::new(EX_TEBAKO_IO, e.to_string()))?;
        let keyring = tebako_signer::trusted_keyring_bytes(home)
            .map_err(|e| BootError::new(EX_TEBAKO_IO, e.to_string()))?;
        let outcome = tebako_signer::verify_detached(&keyring, region, signature, signer_keyid)
            .map_err(|e| BootError::new(EX_TEBAKO_SIGNATURE, e.to_string()))?;
        if matches!(outcome, tebako_signer::VerifyOutcome::Trusted(_)) {
            return Ok(true);
        }
    }
    Ok(false)
}

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

/// OpenPGP verification of the v2 signed trailer (feature
/// `openpgp-verify`): trusted keyring → embedded/dev roots → successor
/// rotation chain. Failures are named exits (EX_TEBAKO_SIGNATURE /
/// EX_TEBAKO_TRUST); every success path is journaled.
#[cfg(feature = "openpgp-verify")]
fn verify_v2_signature(
    self_path: &Path,
    m: &tpkg::Manifest,
    home: &Path,
    region: &[u8],
    keyid_hex: &str,
) -> Result<(), BootError> {
    let v2 = m.v2.as_ref().expect("v2 presence checked by caller");
    let keyring = tebako_signer::trusted_keyring_bytes(home)
        .map_err(|e| BootError::new(EX_TEBAKO_IO, e.to_string()))?;
    let outcome = tebako_signer::verify_detached(&keyring, region, &v2.signature, &v2.signer_keyid)
        .map_err(|e| BootError::new(EX_TEBAKO_SIGNATURE, e.to_string()))?;
    match outcome {
        tebako_signer::VerifyOutcome::Trusted(_) => {
            journal(
                home,
                &format!(
                    "event=v2-trusted package={} signer={keyid_hex}",
                    self_path.display()
                ),
            );
        }
        tebako_signer::VerifyOutcome::Untrusted(_) => {
            // Before the named trust error: the signer may be the
            // embedded/dev trusted root, or reach trust through the
            // successor-statement rotation chain.
            let signer_fp =
                tebako_signer::signature_issuer_fingerprint(&v2.signature).unwrap_or_default();
            let roots = trusted_roots();
            let mut root_verified = false;
            for root in &roots {
                if !root.fingerprint.eq_ignore_ascii_case(&signer_fp) {
                    continue;
                }
                // fingerprint matches a trusted root: the signature must
                // still cryptographically verify against the root's public
                // key (keyring, or bundled with the override)
                let mut ring = tebako_signer::trusted_keyring_bytes(home)
                    .map_err(|e| BootError::new(EX_TEBAKO_IO, e.to_string()))?;
                if let Some(pk) = &root.public_key {
                    ring.extend_from_slice(pk);
                }
                let outcome =
                    tebako_signer::verify_detached(&ring, region, &v2.signature, &v2.signer_keyid)
                        .map_err(|e| BootError::new(EX_TEBAKO_SIGNATURE, e.to_string()))?;
                if matches!(outcome, tebako_signer::VerifyOutcome::Trusted(_)) {
                    journal(
                        home,
                        &format!(
                            "event=v2-trusted-root package={} signer={signer_fp}",
                            self_path.display()
                        ),
                    );
                    root_verified = true;
                }
                break;
            }
            if !root_verified
                && forward_trust_from_successors(
                    home,
                    &roots,
                    &signer_fp,
                    region,
                    &v2.signature,
                    &v2.signer_keyid,
                )?
            {
                journal(
                    home,
                    &format!(
                        "event=v2-trusted-forwarded package={} signer={signer_fp}",
                        self_path.display()
                    ),
                );
                root_verified = true;
            }
            if !root_verified {
                return fail(
                    EX_TEBAKO_TRUST,
                    format!(
                        "the signer of {} is not in the trusted keyring — refusing to execute\n  signer keyid: {keyid_hex}\n  keyring: {}\n  register the signer's public key with tebako-pkg (TOFU) if you trust it",
                        self_path.display(),
                        tebako_signer::trusted_keyring_path(home).display()
                    ),
                );
            }
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
    Ok(())
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
            home,
            &format!("event=legacy-v1-accepted package={}", self_path.display()),
        );
        return Ok(());
    };

    // -- trailer region (shared by both trust modes) ---------------------
    let keyid_hex = v2.signer_keyid_hex();
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

    #[cfg(feature = "openpgp-verify")]
    verify_v2_signature(self_path, m, home, &region, &keyid_hex)?;

    // -- unverified-first (openpgp-verify disabled) ----------------------
    #[cfg(not(feature = "openpgp-verify"))]
    {
        if require_signed_mode() {
            return fail(
                EX_TEBAKO_SIGNATURE,
                format!(
                    "{} is a signed package, but this tebako-bootstrap was built WITHOUT OpenPGP verification (unverified-first) and cannot honor TEBAKO_REQUIRE_SIGNED=1\n  signer keyid: {keyid_hex}\n  run a verification-enabled bootstrap (roadmap 72 crypto toolkit), or unset TEBAKO_REQUIRE_SIGNED to proceed unverified",
                    self_path.display()
                ),
            );
        }
        eprintln!(
            "tebako-bootstrap: WARNING: {} is a signed package executed UNVERIFIED — this bootstrap was built without OpenPGP verification\n  signer keyid: {keyid_hex}\n  — only run packages from sources you trust",
            self_path.display()
        );
        journal(
            home,
            &format!(
                "event=v2-unverified-accepted package={} signer={keyid_hex}",
                self_path.display()
            ),
        );
    }

    // -- per-slot sha256 (integrity anchor; verified when the feature is
    // enabled, unverified-but-corruption-evident when it is not) ---------
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
                home,
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
            home,
            &format!(
                "event=trusted-cache-write-failed path={} error={e}",
                marker_path.display()
            ),
        );
    }
    journal(
        home,
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
) -> Result<(PathBuf, Option<PathBuf>), BootError> {
    let platform = platform_string();
    let asset_base = format!("tebako-runtime-{}-{}-{platform}", rr.abi, rr.version);
    let layout = CacheLayout {
        root: cache_root()?,
        entry_dir: PathBuf::new(),
        exe_path: PathBuf::new(),
        asset: format!("{asset_base}{}", exe_suffix()),
        asset_base,
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

    let mut ux = BootUx::new();

    // The interpreter: cache hit / fat payload slot / download+verify.
    let exe = if file_exists(&layout.exe_path) {
        ux.cached(rr);
        layout.exe_path.clone()
    } else {
        let mut payload_exe = None;
        // fat package: the runtime rides along as a payload slot.
        for slot in &m.slots {
            if slot.format_id == tpkg::TPKG_FORMAT_RUNTIME {
                payload_exe = Some(install_payload(
                    runtime_ref,
                    rr,
                    self_path,
                    slot,
                    &layout,
                    &mut ux,
                )?);
                break;
            }
        }
        match payload_exe {
            Some(exe) => exe,
            None => download_executable(runtime_ref, rr, &layout, &mut ux)?,
        }
    };

    // item 30b: the `;image` flag resolves the runtime image alongside.
    let image = if runtime_ref_wants_image(runtime_ref) {
        Some(resolve_image(runtime_ref, rr, &layout, &mut ux)?)
    } else {
        None
    };
    // tebako-runtime-ruby#40: a dll-era (windows) runtime also needs its
    // ruby DLL next to the exe — it rides the same image-era release
    // card, resolved alongside; a no-op when the cached release index
    // declares no dll facet (every POSIX release, pre-#40 windows
    // releases). No handoff: the PE loader finds the DLL on its own.
    if runtime_ref_wants_image(runtime_ref) {
        resolve_dll(runtime_ref, rr, &layout, &mut ux)?;
    }
    Ok((exe, image))
}

fn download_executable(
    runtime_ref: &str,
    rr: &RuntimeRef,
    layout: &CacheLayout,
    ux: &mut BootUx,
) -> Result<PathBuf, BootError> {
    let (root, entry_dir, asset, entry) = (
        &layout.root,
        &layout.entry_dir,
        &layout.asset,
        &layout.entry,
    );

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

    ux.resolving(runtime_ref);

    let Some(ins) = begin_entry_install(root, entry, exe_path, asset, runtime_ref)? else {
        ux.cached(rr);
        return Ok(exe_path.clone());
    };

    // spec 18 C2: the release card gates BEFORE any asset download —
    // a contract refusal never downloads a byte of the runtime. No
    // readable manifest at all is the same pre-era signal (no old-path
    // readers; the SHA256SUMS fallback covers checksums only).
    let manifest_tmp = ins.tmp_dir.join("manifest.json");
    let manifest_text = if fetch_url(&manifest_url, local, &manifest_tmp).is_ok() {
        std::fs::read_to_string(&manifest_tmp).ok()
    } else {
        None
    };
    let Some(manifest_text) = manifest_text else {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return fail(
            EX_TEBAKO_CONTRACT,
            format!(
                "runtime \"{runtime_ref}\" is pre-era — no readable release manifest at {manifest_url} — refusing to install or execute\n  the release was built by a pre-contract factory; rebuild it with the current tebako-runtime-ruby (spec 18 C2), or pin a runtime that declares its contract"
            ),
        );
    };
    if let Some(e) = contract_gate(runtime_ref, &manifest_text, asset) {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return Err(e);
    }

    if fetch_asset(&asset_url, local, &ins.tmp_asset, asset, &mut ux.prog).is_err() {
        cleanup_tmp_entry(&ins.tmp_dir, asset);
        lock_release(ins.lock);
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "cannot resolve runtime \"{runtime_ref}\": download failed\n  url: {asset_url}\n  downloads are in-process (ureq + rustls, webpki-roots) — check the network, or set\n  TEBAKO_RUNTIME_MIRROR to a reachable mirror, or TEBAKO_OFFLINE=1 for cache-only mode"
            ),
        );
    }

    ux.prog.phase("verifying sha256");

    // expected checksum: the (already gated) release manifest primary,
    // SHA256SUMS.txt fallback.
    const DIAG_NAMES: [&str; 5] = [
        "not tried",
        "download failed",
        "read failed",
        "no matching entry",
        "ok",
    ];
    let mut expected: Option<String> = None;
    let mut diag_manifest = 3;
    if let Ok(sha) = sha_from_manifest_json(&manifest_text, asset) {
        diag_manifest = 4;
        expected = Some(sha);
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
    ux.prog.phase("installing (locked)");
    let installed = publish_entry(ins, entry_dir, exe_path, asset, &actual, &origin)?;
    let size = std::fs::metadata(&installed).map(|m| m.len()).unwrap_or(0);
    ux.prog.line(&installed_line(entry, size, entry_dir));
    Ok(installed)
}

// ---------------------------------------------------------------------
// runtime image resolution (item 30b, the `;image` flag)
// ---------------------------------------------------------------------

/// Resolve the runtime image (`<asset_base>.tfs`) into the executable's
/// cache entry: download (same mirror/offline rules), verify against the
/// release index (manifest.json `image` key primary, SHA256SUMS line
/// fallback), install read-only with `<image>.sha256`/`<image>.origin`
/// trusted markers. The image is never extracted into the cache.
fn resolve_image(
    runtime_ref: &str,
    rr: &RuntimeRef,
    layout: &CacheLayout,
    ux: &mut BootUx,
) -> Result<PathBuf, BootError> {
    let (root, entry_dir, entry) = (&layout.root, &layout.entry_dir, &layout.entry);
    let image_asset = format!("{}.tfs", layout.asset_base);
    let image_path = entry_dir.join(&image_asset);
    let marker = entry_dir.join(format!("{image_asset}.sha256"));

    if file_exists(&image_path) && file_exists(&marker) {
        return Ok(image_path);
    }

    let base_raw = releases_base();
    let base = skip_file_scheme(&base_raw).to_string();
    let local = base_is_local(&base_raw);
    let image_url = format!("{base}/v{}/{image_asset}", rr.abi);
    let manifest_url = format!("{base}/v{}/manifest.json", rr.abi);
    let sums_url = format!("{base}/v{}/SHA256SUMS.txt", rr.abi);

    if offline_mode() {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "cannot resolve runtime image \"{runtime_ref}\": not present in the cache and TEBAKO_OFFLINE is set\n  cache entry: {}\n  would fetch: {image_url}\n  unset TEBAKO_OFFLINE, or set TEBAKO_RUNTIME_MIRROR to a reachable mirror",
                entry_dir.display()
            ),
        );
    }

    ux.resolving(runtime_ref);

    // Serialize against other bootstraps installing this entry (the same
    // lock file the executable install uses).
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

    // re-check under the lock
    if file_exists(&image_path) && file_exists(&marker) {
        lock_release(lock);
        return Ok(image_path);
    }

    let tmp_dir = root
        .join("tmp")
        .join(format!("{entry}.{}.image", std::process::id()));
    let tmp_image = tmp_dir.join(&image_asset);
    cleanup_tmp_entry(&tmp_dir, &image_asset);
    if let Err(e) = std::fs::create_dir(&tmp_dir) {
        lock_release(lock);
        return Err(BootError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", tmp_dir.display()),
        ));
    }

    let fail_image = |lock: EntryLock, e: BootError| -> BootError {
        cleanup_tmp_entry(&tmp_dir, &image_asset);
        lock_release(lock);
        e
    };

    // spec 18 C2: the release card gates BEFORE the image download (the
    // image is additive metadata of the exe's package entry — the entry
    // anchored by the executable's filename governs it, the same gate
    // the executable path applies). No readable manifest is the same
    // pre-era signal.
    let manifest_tmp = tmp_dir.join("manifest.json");
    let manifest_text = if fetch_url(&manifest_url, local, &manifest_tmp).is_ok() {
        std::fs::read_to_string(&manifest_tmp).ok()
    } else {
        None
    };
    let Some(manifest_text) = manifest_text else {
        return Err(fail_image(
            lock,
            BootError::new(
                EX_TEBAKO_CONTRACT,
                format!(
                    "runtime \"{runtime_ref}\" is pre-era — no readable release manifest at {manifest_url} — refusing to install or execute\n  the release was built by a pre-contract factory; rebuild it with the current tebako-runtime-ruby (spec 18 C2), or pin a runtime that declares its contract"
                ),
            ),
        ));
    };
    if let Some(e) = contract_gate(runtime_ref, &manifest_text, &layout.asset) {
        return Err(fail_image(lock, e));
    }

    if fetch_asset(&image_url, local, &tmp_image, &image_asset, &mut ux.prog).is_err() {
        return Err(fail_image(
            lock,
            BootError::new(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "cannot resolve runtime image \"{runtime_ref}\": download failed\n  url: {image_url}\n  downloads are in-process (ureq + rustls, webpki-roots) — check the network, or set\n  TEBAKO_RUNTIME_MIRROR to a reachable mirror, or TEBAKO_OFFLINE=1 for cache-only mode"
                ),
            ),
        ));
    }

    ux.prog.phase("verifying sha256");

    // expected checksum: the (already gated) manifest's `image` key
    // primary, SHA256SUMS line fallback (the same sources the
    // executable's checksum uses).
    const DIAG_NAMES: [&str; 5] = [
        "not tried",
        "download failed",
        "read failed",
        "no matching entry",
        "ok",
    ];
    let mut expected: Option<String> = None;
    let mut diag_manifest = 3;
    if let Ok(sha) = sha_from_manifest_image(&manifest_text, &image_asset) {
        diag_manifest = 4;
        expected = Some(sha);
    }
    let mut diag_sums = 0;
    if expected.is_none() {
        diag_sums = 1;
        let sums_tmp = tmp_dir.join("SHA256SUMS.txt");
        if fetch_url(&sums_url, local, &sums_tmp).is_ok() {
            diag_sums = 2;
            if let Ok(text) = std::fs::read_to_string(&sums_tmp) {
                diag_sums = 3;
                if let Ok(sha) = sha_from_sums(&text, &image_asset) {
                    diag_sums = 4;
                    expected = Some(sha);
                }
            }
        }
    }
    let Some(expected) = expected else {
        return Err(fail_image(
            lock,
            BootError::new(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "cannot resolve runtime image \"{runtime_ref}\": no checksum for {image_asset} in the release\n  tried: {manifest_url} ({})\n         {sums_url} ({})",
                    DIAG_NAMES[diag_manifest], DIAG_NAMES[diag_sums]
                ),
            ),
        ));
    };

    let actual = match sha256_file_hex(&tmp_image) {
        Ok(a) => a,
        Err(e) => {
            return Err(fail_image(
                lock,
                BootError::new(
                    EX_TEBAKO_IO,
                    format!("cannot hash downloaded file {}: {e}", tmp_image.display()),
                ),
            ));
        }
    };

    let expected = expected.to_lowercase();
    if expected != actual {
        return Err(fail_image(
            lock,
            BootError::new(
                EX_TEBAKO_SHA,
                format!(
                    "SHA256 mismatch for downloaded runtime image {image_asset} — refusing to install or execute\n  expected: {expected} (from {manifest_url})\n  actual:   {actual}\n  the download was deleted; the cache was not touched"
                ),
            ),
        ));
    }

    // Install: the image is immutable (0444) with trusted markers.
    ux.prog.phase("installing (locked)");
    make_readonly(&tmp_image);
    if let Err(e) = os_rename(&tmp_image, &image_path) {
        return Err(fail_image(
            lock,
            BootError::new(
                EX_TEBAKO_IO,
                format!(
                    "cannot install runtime image into the cache ({} -> {}): {e}",
                    tmp_image.display(),
                    image_path.display()
                ),
            ),
        ));
    }
    let _ = write_small_file(&marker, &format!("{actual}  {image_asset}\n"));
    let _ = write_small_file(
        &entry_dir.join(format!("{image_asset}.origin")),
        &format!("runtime_ref={runtime_ref}\nurl={image_url}\nsha256={actual}\n"),
    );
    cleanup_tmp_entry(&tmp_dir, &image_asset);
    lock_release(lock);
    let size = std::fs::metadata(&image_path).map(|m| m.len()).unwrap_or(0);
    ux.prog.line(&installed_line(&image_asset, size, entry_dir));
    Ok(image_path)
}

// ---------------------------------------------------------------------
// runtime ruby DLL resolution (tebako-runtime-ruby#40, windows runtimes)
// ---------------------------------------------------------------------

/// Resolve the windows ruby DLL (`<asset_base>.dll`) into the
/// executable's cache entry when the release declares the additive `dll`
/// facet: download (same mirror/offline rules as the image), verify
/// against the declared sha256, install read-only AS `install_as` — the
/// PE name the exe and the extension .so's import, never the asset name
/// (assets are unique per leg; two same-ABI legs share the PE name) —
/// with `<install_as>.sha256`/`<install_as>.origin` trusted markers. The
/// PE loader resolves the exe's imports against the exe's own directory
/// first, so the DLL must sit next to the exe. `Ok(None)` when the
/// release declares no dll facet (every POSIX release, pre-#40 windows
/// releases) — the facet is declared, never derived.
///
/// The facet source is the entry's cached release index (the verified
/// lean install leaves manifest.json in the entry), so a cached run
/// never re-fetches the index — a run stays a run, offline-safe. An
/// entry without a cached index (the fat-payload path never fetches one)
/// declares nothing here.
fn resolve_dll(
    runtime_ref: &str,
    rr: &RuntimeRef,
    layout: &CacheLayout,
    ux: &mut BootUx,
) -> Result<Option<PathBuf>, BootError> {
    let (root, entry_dir, entry) = (&layout.root, &layout.entry_dir, &layout.entry);
    let dll_asset = format!("{}.dll", layout.asset_base);

    let Ok(manifest_text) = std::fs::read_to_string(entry_dir.join("manifest.json")) else {
        return Ok(None);
    };
    let Ok(install_as) = dll_install_as_from_manifest(&manifest_text, &dll_asset) else {
        return Ok(None);
    };
    let dll_path = entry_dir.join(&install_as);
    let marker = entry_dir.join(format!("{install_as}.sha256"));
    if file_exists(&dll_path) && file_exists(&marker) {
        return Ok(Some(dll_path));
    }
    let Ok(expected) = sha_from_manifest_image(&manifest_text, &dll_asset) else {
        return Ok(None); // an incomplete facet is no facet — never guess
    };
    if install_as.contains('/') || install_as.contains('\\') {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "release manifest dll facet for {} carries an unusable install_as (\"{install_as}\") — the PE name must be a bare file name — refusing to install or execute",
                layout.asset
            ),
        );
    }

    let base_raw = releases_base();
    let base = skip_file_scheme(&base_raw).to_string();
    let local = base_is_local(&base_raw);
    let dll_url = format!("{base}/v{}/{dll_asset}", rr.abi);

    if offline_mode() {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "cannot resolve runtime dll \"{runtime_ref}\": not present in the cache and TEBAKO_OFFLINE is set\n  cache entry: {}\n  would fetch: {dll_url}\n  unset TEBAKO_OFFLINE, or set TEBAKO_RUNTIME_MIRROR to a reachable mirror",
                entry_dir.display()
            ),
        );
    }

    ux.resolving(runtime_ref);

    // Serialize against other bootstraps installing this entry (the same
    // lock file the executable and image installs use).
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

    // re-check under the lock
    if file_exists(&dll_path) && file_exists(&marker) {
        lock_release(lock);
        return Ok(Some(dll_path));
    }

    let tmp_dir = root
        .join("tmp")
        .join(format!("{entry}.{}.dll", std::process::id()));
    let tmp_dll = tmp_dir.join(&dll_asset);
    cleanup_tmp_entry(&tmp_dir, &dll_asset);
    if let Err(e) = std::fs::create_dir(&tmp_dir) {
        lock_release(lock);
        return Err(BootError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", tmp_dir.display()),
        ));
    }

    let fail_dll = |lock: EntryLock, e: BootError| -> BootError {
        cleanup_tmp_entry(&tmp_dir, &dll_asset);
        lock_release(lock);
        e
    };

    if fetch_asset(&dll_url, local, &tmp_dll, &dll_asset, &mut ux.prog).is_err() {
        return Err(fail_dll(
            lock,
            BootError::new(
                EX_TEBAKO_UNAVAILABLE,
                format!(
                    "cannot resolve runtime dll \"{runtime_ref}\": download failed\n  url: {dll_url}\n  downloads are in-process (ureq + rustls, webpki-roots) — check the network, or set\n  TEBAKO_RUNTIME_MIRROR to a reachable mirror, or TEBAKO_OFFLINE=1 for cache-only mode"
                ),
            ),
        ));
    }

    ux.prog.phase("verifying sha256");

    let actual = match sha256_file_hex(&tmp_dll) {
        Ok(a) => a,
        Err(e) => {
            return Err(fail_dll(
                lock,
                BootError::new(
                    EX_TEBAKO_IO,
                    format!("cannot hash downloaded file {}: {e}", tmp_dll.display()),
                ),
            ));
        }
    };

    let expected = expected.to_lowercase();
    if expected != actual {
        return Err(fail_dll(
            lock,
            BootError::new(
                EX_TEBAKO_SHA,
                format!(
                    "SHA256 mismatch for downloaded runtime dll {dll_asset} — refusing to install or execute\n  expected: {expected} (from the cached release manifest)\n  actual:   {actual}\n  the download was deleted; the cache was not touched"
                ),
            ),
        ));
    }

    // Install: the dll is immutable (0444) with trusted markers, AS
    // install_as next to the exe.
    ux.prog.phase("installing (locked)");
    make_readonly(&tmp_dll);
    if let Err(e) = os_rename(&tmp_dll, &dll_path) {
        return Err(fail_dll(
            lock,
            BootError::new(
                EX_TEBAKO_IO,
                format!(
                    "cannot install the runtime dll into the cache ({} -> {}): {e}",
                    tmp_dll.display(),
                    dll_path.display()
                ),
            ),
        ));
    }
    let _ = write_small_file(&marker, &format!("{actual}  {install_as}\n"));
    let _ = write_small_file(
        &entry_dir.join(format!("{install_as}.origin")),
        &format!("runtime_ref={runtime_ref}\nurl={dll_url}\nsha256={actual}\n"),
    );
    cleanup_tmp_entry(&tmp_dir, &dll_asset);
    lock_release(lock);
    let size = std::fs::metadata(&dll_path).map(|m| m.len()).unwrap_or(0);
    ux.prog.line(&installed_line(&install_as, size, entry_dir));
    Ok(Some(dll_path))
}

// ---------------------------------------------------------------------
// exec handoff (launcher ABI v1)
// ---------------------------------------------------------------------

/// The launcher ABI v1 handoff argv — byte-identical on every platform:
/// nargv[0] is the runtime, then one `--tebako-image <self>:<slot>:<mount>`
/// pair per mounted slot, then `--tebako-entry <entry> <user args...>`.
///
/// Suite mount rule (spec 03 §6): the SELECTED entry's slot mounts, slots
/// referenced by OTHER package-manifest entries do not (suite member
/// images are mutually exclusive — they share one mount point by
/// construction), and slots no entry references (extra `--image` payloads)
/// mount as always. Without a package manifest every non-runtime slot
/// mounts and `--tebako-entry` is argv0 verbatim — the v1 behavior.
/// Public for the integration tests; the flow uses it once per run.
pub fn handoff_argv(
    runtime: &Path,
    self_path: &Path,
    m: &tpkg::Manifest,
    selection: Option<&(tpkg::PackageManifest, tpkg::PackageEntry)>,
    argv: &[String],
) -> Vec<String> {
    let mut nargv: Vec<String> = vec![runtime.to_string_lossy().into_owned()];
    for (s, slot) in m.slots.iter().enumerate() {
        if slot.format_id == tpkg::TPKG_FORMAT_RUNTIME {
            continue; // runtime payload: installed into the cache, never mounted
        }
        if let Some((pm, selected)) = selection {
            if s != selected.slot as usize && pm.entries.iter().any(|e| e.slot as usize == s) {
                continue; // another suite member's image — not mounted
            }
        }
        nargv.push("--tebako-image".to_string());
        nargv.push(format!(
            "{}:{s}:{}",
            self_path.display(),
            slot.mount_point_str().unwrap_or_default()
        ));
    }
    nargv.push("--tebako-entry".to_string());
    nargv.push(match selection {
        Some((_, entry)) => entry.entrypoint.clone(),
        None => argv
            .first()
            .cloned()
            .unwrap_or_else(|| self_path.to_string_lossy().into_owned()),
    });
    nargv.extend(argv.iter().skip(1).cloned());
    nargv
}

/// Unix: execv(3) replaces the bootstrap — never returns on success.
#[cfg(unix)]
fn exec_runtime(
    runtime: &Path,
    image: Option<&Path>,
    self_path: &Path,
    m: &tpkg::Manifest,
    selection: Option<&(tpkg::PackageManifest, tpkg::PackageEntry)>,
    argv: &[String],
    jail: Option<&JailEnv>,
) -> BootError {
    use std::os::unix::process::CommandExt;

    let nargv = handoff_argv(runtime, self_path, m, selection, argv);
    let mut cmd = std::process::Command::new(runtime);
    cmd.args(&nargv[1..]);
    if let Some(image) = image {
        // item 30b: the runtime image rides the environment; image-era
        // drivers mount it instead of an embedded image, v1 drivers
        // ignore it. The handoff options themselves are unchanged.
        cmd.env("TEBAKO_RUNTIME_IMAGE", image);
    }
    if let Some(jail) = jail {
        // spec 08: the effective jail (manifest request ∩ user
        // tightening) reaches the driver through its policy env; the
        // driver's violations journal into this home's journal.log.
        cmd.env("TEBAKO_JAIL", &jail.spec);
        cmd.env("TEBAKO_JAIL_SOURCE", jail.source);
        cmd.env("TEBAKO_JAIL_JOURNAL", &jail.journal);
    }
    let err = cmd.exec();
    BootError::new(
        EX_TEBAKO_IO,
        format!("cannot execute runtime {}: {err}", runtime.display()),
    )
}

/// Windows has no execve(2): the runtime is spawned as a child process,
/// waited on, and the bootstrap exits with the child's exit code
/// (platform::spawn_handoff) — the exit-code contract holds: the user
/// sees the runtime's code, and the loader errors (65–74) still
/// originate loader-side before this point. Never returns on success;
/// the spawn/wait failure maps onto the same EX_TEBAKO_IO message body
/// as the unix exec failure.
#[cfg(windows)]
fn exec_runtime(
    runtime: &Path,
    image: Option<&Path>,
    self_path: &Path,
    m: &tpkg::Manifest,
    selection: Option<&(tpkg::PackageManifest, tpkg::PackageEntry)>,
    argv: &[String],
    jail: Option<&JailEnv>,
) -> BootError {
    let nargv = handoff_argv(runtime, self_path, m, selection, argv);
    let err = platform::spawn_handoff(runtime, &nargv[1..], image, jail);
    BootError::new(
        EX_TEBAKO_IO,
        format!("cannot execute runtime {}: {err}", runtime.display()),
    )
}

// ---------------------------------------------------------------------
// main flow
// ---------------------------------------------------------------------

/// Run the bootstrap. `argv` includes argv[0]. Never returns Ok on any
/// platform (unix exec replaces the process; the Windows spawn handoff
/// exits with the child's code) — the return is a named error with its
/// exit code.
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

    // The explicit install verb (TODO.v2-1/12): a run is a run — payload
    // slices land in the local store only when asked. The manifest read
    // needs the TFS engine the size-capped bootstrap deliberately does
    // not carry, so the verb refuses (frozen package) or guides to the
    // CLI. Placed after the chain + ABI gates: an install attempt is
    // verified exactly like a run.
    if argv.get(1).map(String::as_str) == Some("--tebako-install") {
        if m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL != 0 {
            return fail(
                EX_TEBAKO_INSTALL,
                format!(
                    "{} was built non-installable (TPKG_FLAG_NO_INSTALL — the publisher froze it); it runs standalone",
                    self_path.display()
                ),
            );
        }
        return fail(
            EX_TEBAKO_INSTALL,
            format!(
                "to install this package's slices into the local store, run:\n  tebako install {}\n(the manifest read needs the TFS engine the bootstrap deliberately does not carry — the CLI does it for you)",
                self_path.display()
            ),
        );
    }

    // argv0 entry selection (spec 03 §6 / spec 07 §2.0): the type-2
    // package manifest's entries map command names to per-entry runtime
    // refs; argv0 picks one (entries[0] fallback), v1 packages see no
    // selection at all.
    let argv0 = argv.first().map(String::as_str).unwrap_or("");
    let selection = package_selection(&m, argv0)?;
    let runtime_ref = match &selection {
        Some((_, entry)) => entry.runtime_ref.clone(),
        None => m.runtime_ref_str().unwrap_or_default().to_string(),
    };
    if runtime_ref.is_empty() {
        return fail(
            EX_TEBAKO_RUNTIME_REF,
            "package has no runtime_ref (classic bundle?) — nothing for the bootstrap to resolve"
                .into(),
        );
    }
    let rr = parse_runtime_ref(&runtime_ref)?;

    // Jails (spec 08 §2/§4): the package's `jail:` request ∩ the user's
    // TEBAKO_JAIL tightening = the effective policy, exported to the
    // driver as TEBAKO_JAIL (+ TEBAKO_JAIL_SOURCE / TEBAKO_JAIL_JOURNAL)
    // at handoff. Fail-closed: a malformed policy never runs open.
    let jail = {
        let user = user_jail_from_env()?;
        let home = cache_root()?;
        prepare_jail(&m, user.as_ref(), argv, &home)?
    };

    let (runtime, image) = resolve_runtime(&runtime_ref, &rr, &self_path, &m)?;
    Err(exec_runtime(
        &runtime,
        image.as_deref(),
        &self_path,
        &m,
        selection.as_ref(),
        argv,
        jail.as_ref(),
    ))
}

#[cfg(test)]
mod store_layout_tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tebako-boot-store-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_new_store_is_born_stamped() {
        let home = dir("new");
        assert_eq!(store_layout_check_once(&home), Ok(()));
        assert_eq!(
            std::fs::read_to_string(home.join("layout-version")).unwrap(),
            format!("{STORE_LAYOUT_VERSION}\n")
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_pre_versioning_store_is_stamped() {
        let home = dir("legacy");
        std::fs::create_dir_all(home.join("runtimes")).unwrap();
        assert_eq!(store_layout_check_once(&home), Ok(()));
        assert_eq!(
            std::fs::read_to_string(home.join("layout-version")).unwrap(),
            format!("{STORE_LAYOUT_VERSION}\n")
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_newer_stamp_is_the_upgrade_refusal() {
        let home = dir("newer");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("layout-version"), "99\n").unwrap();
        let err = store_layout_check_once(&home).unwrap_err();
        // The canonical message text — pinned identical to
        // tebako-resolve::store's Newer refusal (the mirror rule).
        assert!(err.contains("layout 99"), "{err}");
        assert!(err.contains("speaks layout 1"), "{err}");
        assert!(err.contains("upgrade tebako"), "{err}");
        assert_eq!(
            std::fs::read_to_string(home.join("layout-version")).unwrap(),
            "99\n",
            "no silent downgrade"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_corrupt_stamp_is_named() {
        let home = dir("corrupt");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("layout-version"), "banana\n").unwrap();
        let err = store_layout_check_once(&home).unwrap_err();
        assert!(err.contains("banana"), "{err}");
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[cfg(test)]
mod dll_tests {
    use super::*;

    const EXE: &str = "tebako-runtime-0.16.3-3.3.12-windows-ucrt64.exe";
    const DLL: &str = "tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll";
    const INSTALL_AS: &str = "x64-ucrt-ruby330.dll";
    const RUNTIME_REF: &str = "ruby@3.3.12;tebako=0.16.3;image";

    fn dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tebako-boot-dll-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// The era-2 factory manifest shape with the given `dll` key body
    /// appended to the exe entry ("" = no facet, the POSIX shape).
    fn manifest(dll_key: &str) -> String {
        format!(
            "[{{\"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"filename\": \"{EXE}\", \"sha256\": \"{}\", \"image\": {{\"filename\": \"tebako-runtime-0.16.3-3.3.12-windows-ucrt64.tfs\", \"sha256\": \"{}\"}}{dll_key}}}]\n",
            "a".repeat(64),
            "b".repeat(64),
        )
    }

    fn dll_key(install_as: &str, sha256: &str) -> String {
        format!(
            ", \"dll\": {{\"filename\": \"{DLL}\", \"install_as\": \"{install_as}\", \"sha256\": \"{sha256}\", \"size_bytes\": 14}}"
        )
    }

    /// A cache entry (exe + optional cached release index) in the shape
    /// the lean install leaves behind (its tmp/ dir included — the exe
    /// install creates it before the dll resolution runs).
    fn dll_layout(home: &Path, manifest_text: Option<&str>) -> (CacheLayout, RuntimeRef) {
        let entry = "ruby-3.3.12-0.16.3-windows-ucrt64";
        let asset_base = "tebako-runtime-0.16.3-3.3.12-windows-ucrt64";
        let asset = format!("{asset_base}.exe");
        let entry_dir = home.join("runtimes").join(entry);
        std::fs::create_dir_all(&entry_dir).unwrap();
        std::fs::create_dir_all(home.join("tmp")).unwrap();
        std::fs::write(entry_dir.join(&asset), b"fake runtime exe\n").unwrap();
        if let Some(text) = manifest_text {
            std::fs::write(entry_dir.join("manifest.json"), text).unwrap();
        }
        (
            CacheLayout {
                root: home.to_path_buf(),
                entry_dir: entry_dir.clone(),
                exe_path: entry_dir.join(&asset),
                asset,
                asset_base: asset_base.to_string(),
                entry: entry.to_string(),
            },
            RuntimeRef {
                r#type: "ruby".to_string(),
                version: "3.3.12".to_string(),
                abi: "0.16.3".to_string(),
            },
        )
    }

    #[test]
    fn dll_install_as_parses_from_the_dll_facet() {
        let text = manifest(&dll_key(INSTALL_AS, &"c".repeat(64)));
        assert_eq!(
            dll_install_as_from_manifest(&text, DLL),
            Ok(INSTALL_AS.to_string())
        );
        // the facet's sha rides the same next-sha256 reader the image uses
        assert_eq!(sha_from_manifest_image(&text, DLL), Ok("c".repeat(64)));
    }

    #[test]
    fn dll_facet_absent_or_incomplete_is_no_answer() {
        // no dll key at all (every POSIX release, pre-#40 windows)
        assert_eq!(dll_install_as_from_manifest(&manifest(""), DLL), Err(()));
        // the dll key without install_as — never guess the PE name
        let text = manifest(&format!(
            ", \"dll\": {{\"filename\": \"{DLL}\", \"sha256\": \"{}\"}}",
            "c".repeat(64)
        ));
        assert_eq!(dll_install_as_from_manifest(&text, DLL), Err(()));
        // an empty install_as is no name
        let text = manifest(&dll_key("", &"c".repeat(64)));
        assert_eq!(dll_install_as_from_manifest(&text, DLL), Err(()));
    }

    #[test]
    fn dll_install_as_is_bounded_to_its_own_entry() {
        // the first entry's dll object has no install_as; the second
        // entry's must not leak across the object boundary
        let text = format!(
            "[{{\"filename\": \"{EXE}\", \"sha256\": \"{a}\", \"dll\": {{\"filename\": \"{DLL}\", \"sha256\": \"{b}\"}}}}, {{\"filename\": \"other.exe\", \"sha256\": \"{a}\", \"dll\": {{\"filename\": \"other.dll\", \"install_as\": \"x64-ucrt-ruby340.dll\", \"sha256\": \"{b}\"}}}}]",
            a = "a".repeat(64),
            b = "b".repeat(64),
        );
        assert_eq!(dll_install_as_from_manifest(&text, DLL), Err(()));
    }

    /// The env-using flow (TEBAKO_RUNTIME_MIRROR / TEBAKO_OFFLINE) lives
    /// in ONE test so the process-wide variables never race a sibling.
    #[test]
    fn resolve_dll_flow() {
        let home = dir("flow");
        let mirror = home.join("mirror").join("v0.16.3");
        std::fs::create_dir_all(&mirror).unwrap();
        std::fs::write(mirror.join(DLL), b"fake ruby dll\n").unwrap();
        let dll_sha = sha256_file_hex(&mirror.join(DLL)).unwrap();
        let text = manifest(&dll_key(INSTALL_AS, &dll_sha));
        let (layout, rr) = dll_layout(&home, Some(&text));
        std::env::set_var(
            "TEBAKO_RUNTIME_MIRROR",
            format!("file://{}", home.join("mirror").display()),
        );

        // fresh install: the dll lands AS install_as with trusted markers
        let mut ux = BootUx::new();
        let got = resolve_dll(RUNTIME_REF, &rr, &layout, &mut ux).unwrap();
        let dll_path = layout.entry_dir.join(INSTALL_AS);
        assert_eq!(got, Some(dll_path.clone()));
        assert!(dll_path.is_file());
        assert!(
            !layout.entry_dir.join(DLL).exists(),
            "the asset name is not the install name"
        );
        let marker =
            std::fs::read_to_string(layout.entry_dir.join(format!("{INSTALL_AS}.sha256"))).unwrap();
        assert_eq!(marker, format!("{dll_sha}  {INSTALL_AS}\n"));
        let origin =
            std::fs::read_to_string(layout.entry_dir.join(format!("{INSTALL_AS}.origin"))).unwrap();
        assert!(origin.contains(&format!("/v0.16.3/{DLL}")), "{origin}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                dll_path.metadata().unwrap().permissions().mode() & 0o777,
                0o444
            );
        }

        // a cached run needs no mirror at all (offline-safe re-resolution)
        std::fs::remove_dir_all(home.join("mirror")).unwrap();
        let mut ux = BootUx::new();
        let got = resolve_dll(RUNTIME_REF, &rr, &layout, &mut ux).unwrap();
        assert_eq!(got, Some(dll_path.clone()));

        // a declared-but-missing dll under TEBAKO_OFFLINE is the named error
        std::fs::remove_file(&dll_path).unwrap();
        std::fs::remove_file(layout.entry_dir.join(format!("{INSTALL_AS}.sha256"))).unwrap();
        std::env::set_var("TEBAKO_OFFLINE", "1");
        let mut ux = BootUx::new();
        let err = resolve_dll(RUNTIME_REF, &rr, &layout, &mut ux).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        assert!(err.message.contains("TEBAKO_OFFLINE"), "{}", err.message);
        std::env::remove_var("TEBAKO_OFFLINE");

        // a wrong declared sha is exit 70; the download is deleted
        std::fs::create_dir_all(&mirror).unwrap();
        std::fs::write(mirror.join(DLL), b"fake ruby dll\n").unwrap();
        std::fs::write(
            layout.entry_dir.join("manifest.json"),
            manifest(&dll_key(INSTALL_AS, &"f".repeat(64))),
        )
        .unwrap();
        let mut ux = BootUx::new();
        let err = resolve_dll(RUNTIME_REF, &rr, &layout, &mut ux).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA);
        assert!(!dll_path.exists());
        assert!(
            !layout
                .entry_dir
                .join(format!("{INSTALL_AS}.sha256"))
                .exists(),
            "a failed install leaves no trust marker"
        );

        // an install_as with a path separator is refused by name
        std::fs::write(
            layout.entry_dir.join("manifest.json"),
            manifest(&dll_key("../evil.dll", &dll_sha)),
        )
        .unwrap();
        let mut ux = BootUx::new();
        let err = resolve_dll(RUNTIME_REF, &rr, &layout, &mut ux).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_UNAVAILABLE);
        assert!(err.message.contains("bare file name"), "{}", err.message);
        assert!(!home.join("runtimes").join("evil.dll").exists());

        // no dll key declared (every POSIX release): a quiet no-op
        let home2 = dir("nofacet");
        let (layout2, rr2) = dll_layout(&home2, Some(&manifest("")));
        let mut ux = BootUx::new();
        assert!(resolve_dll(RUNTIME_REF, &rr2, &layout2, &mut ux)
            .unwrap()
            .is_none());
        // no cached release index at all (the fat-payload path): a no-op
        let home3 = dir("nomanifest");
        let (layout3, rr3) = dll_layout(&home3, None);
        let mut ux = BootUx::new();
        assert!(resolve_dll(RUNTIME_REF, &rr3, &layout3, &mut ux)
            .unwrap()
            .is_none());

        std::env::remove_var("TEBAKO_RUNTIME_MIRROR");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&home2);
        let _ = std::fs::remove_dir_all(&home3);
    }
}
