//! Port of the gem's RuntimeManager (lib/tebako/runtime_manager.rb):
//! resolution, download, verification and machine-wide caching of the
//! prebuilt tebako runtime packages.
//!
//! Cache layout (rooted at $TEBAKO_HOME or ~/.tebako), identical to the gem:
//!   runtimes/ruby-<ruby-version>-<tebakoabi>-<platform>/
//!     tebako-runtime-<tebakoabi>-<ruby-version>-<platform>[.exe]
//!     sha256    -- digest the installed file was verified against
//!     origin    -- URL the package was downloaded from
//!
//! Installs are serialized per entry with a flock'd lockfile; packages are
//! downloaded to tmp/, SHA256-verified against the release index and moved
//! into place with an atomic rename, so partial downloads never poison the
//! cache.
//!
//! G1 (spec 09 §4/§5): the fetch is signature-verified at the index
//! boundary whenever the release carries detached `.asc` siblings (the
//! per-package shard's, manifest.json's, or SHA256SUMS.txt's — the first
//! TRUSTED form wins; a verified form beats an unsigned one regardless of
//! the index's preference order), and a `signature:` block declared on
//! the entry or a facet (spec 13 §2a) verifies against the downloaded
//! bytes BEFORE the sha256 check. An unverifiable fetch is accepted
//! LOUDLY (stderr + audit journal); TEBAKO_REQUIRE_SIGNED=1 refuses it
//! (exit 71) before any asset downloads. An invalid signature exits 71;
//! an untrusted signer or a changed signer key exits 72.
//!
//! The gem's BootstrapManager half is ported for the RUST bootstrap only
//! (spec 19 §4): [`BootstrapResolver`] resolves the per-triplet
//! tebako-bootstrap published with the product's own releases
//! (tamatebako/tebako — the CLI's own version) into
//!   bootstraps/<version>-<triplet>/
//!     tebako-bootstrap-<version>-<triplet>[.exe]
//!     sha256    -- digest the installed file was verified against
//!     origin    -- URL the asset was downloaded from
//! with the runtime cache's exact discipline (release-index sha256
//! verification, tmp + rename, per-entry flock). The v1 C++
//! tebako-bootstrap download the gem's BootstrapManager fetched stays
//! retired: its argv0-verbatim handoff is rejected by the image-era
//! runtime driver, so the fallback produced silently-broken packages.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sha2::Digest;

use tebako_pkg::{json_parse, JsonValue};
use tpkg::runtime_store::EntrySignature;

use crate::error::{packaging_error, TebakoError};
use crate::fetch::{fetch_bytes, fetch_text, FetchError};
use crate::runner::run_with_capture_v;

const TMP_DIR: &str = "tmp";
const LOCK_FILE: &str = ".install.lock";
const SHA256_FILE: &str = "sha256";
const ORIGIN_FILE: &str = "origin";
const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// The tebako-runtime-ruby release mirror (TEBAKO_RUNTIME_MIRROR
/// overrides).
const DEFAULT_MIRROR: &str = "https://github.com/tamatebako/tebako-runtime-ruby/releases/download";
const MIRROR_ENV_VAR: &str = "TEBAKO_RUNTIME_MIRROR";
/// The cache subdirectory and release identity this resolver consumes.
const CACHE_SUBDIR: &str = "runtimes";
const RELEASE_NAME: &str = "tebako-runtime-ruby";
/// The release index's DERIVED monoliths, in preference order — the
/// fallback chain behind the per-package shard (`<stem>.manifest.json`,
/// tebako#493), consulted for pre-shard releases and kept forever
/// (invariant 7).
const INDEX_FILES: &[&str] = &["manifest.json", "SHA256SUMS.txt"];

/// $TEBAKO_HOME or ~/.tebako (LOCALAPPDATA\tebako on Windows).
pub fn default_cache_root() -> PathBuf {
    if let Ok(home) = std::env::var("TEBAKO_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    if cfg!(windows) {
        if let Ok(lad) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(lad).join("tebako");
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".tebako")
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub ruby_version: Option<String>,
    pub platform: Option<String>,
    pub filename: String,
    pub sha256: String,
    /// The entry's declared detached-signature block (spec 13 §2a):
    /// the signer pin + the `.asc` asset name within the same release.
    /// Absent on the pre-signing keep-forever line and on any
    /// SHA256SUMS-derived entry (the sums form carries no signatures).
    pub signature: Option<EntrySignature>,
    /// The runtime image sibling (item 30b): `<asset>.tfs` from the
    /// manifest's additive `image` key or the SHA256SUMS line.
    pub image: Option<ImageRef>,
    /// The windows ruby DLL sibling (tebako-runtime-ruby#40): `<asset>.dll`
    /// from the manifest's additive `dll` key or the SHA256SUMS line.
    pub dll: Option<DllRef>,
}

/// A resolved runtime image reference (filename + expected sha256 +
/// the declared signature block when the manifest carries one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    pub filename: String,
    pub sha256: String,
    pub signature: Option<EntrySignature>,
}

/// A resolved ruby DLL reference (tebako-runtime-ruby#40): the release
/// asset (`filename` — unique per leg), the PE name it installs under
/// (`install_as` — two same-ABI legs share it; the SHA256SUMS index form
/// carries no PE name, so a sums-derived ref cannot be installed), and the
/// expected sha256 (+ the declared signature block when the manifest
/// carries one — the sums form carries none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DllRef {
    pub filename: String,
    pub install_as: Option<String>,
    pub sha256: String,
    pub signature: Option<EntrySignature>,
}

/// The release index plus its raw card text — the entries and, when the
/// index parsed from a JSON card (the per-package shard or the
/// monolithic manifest.json), the card the spec 18 contract gate reads
/// (tebako#493).
type IndexAndCard = (Vec<IndexEntry>, Option<String>);

/// Whether the consumed release-index form was signature-verified at the
/// fetch boundary (spec 09 §4/§5, roadmap 80 G1): the detached `.asc`
/// sibling of the shard / manifest.json / SHA256SUMS.txt verified
/// TRUSTED against the keyring (the signer's resolved PRIMARY keyid
/// rides along for the audit journal), or no form carried a verifiable
/// signature.
#[derive(Debug)]
enum IndexTrust {
    Unverified,
    Verified { signer: String },
}

impl IndexTrust {
    fn signer(&self) -> Option<&str> {
        match self {
            IndexTrust::Unverified => None,
            IndexTrust::Verified { signer } => Some(signer),
        }
    }
}

/// The fetched release index: the entries, the raw card when the form
/// was a JSON card (the contract gate reads it), and the form's trust
/// outcome (G1).
struct FetchedIndex {
    entries: Vec<IndexEntry>,
    card: Option<String>,
    trust: IndexTrust,
}

/// One release-index form's parse outcome: the form cannot serve the
/// request (fall through to the next form — the pre-G1 IndexUnavailable
/// semantics), or it carries a TORN `signature` block (spec 13 §2a —
/// FATAL, exit 65: an index lying about its trust anchors never falls
/// through to an unsigned form, spec 00 §9).
#[derive(Debug)]
enum ParseFail {
    Unavailable(String),
    Torn(String),
}

/// The outcome of resolving a runtime: the interpreter plus, when the
/// release is image-era, its runtime image reference.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub executable: PathBuf,
    pub image: Option<ImageRef>,
}

/// The resolved windows dll facet of a runtime entry
/// (tebako-runtime-ruby#40): the installed path, its PE name
/// (`install_as`), and the verified sha256 — the pin a press lock
/// carries.
#[derive(Debug, Clone)]
pub struct ResolvedDll {
    pub path: PathBuf,
    pub install_as: String,
    pub sha256: String,
}

#[derive(Debug)]
pub struct Resolver {
    pub cache_root: PathBuf,
    pub mirror: String,
    pub lock_timeout: std::time::Duration,
}

pub struct CacheEntry {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub installed_at: std::time::SystemTime,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    pub fn new() -> Self {
        let mirror = std::env::var(MIRROR_ENV_VAR)
            .ok()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| DEFAULT_MIRROR.to_string());
        Resolver {
            cache_root: default_cache_root(),
            mirror: mirror.trim_end_matches('/').to_string(),
            lock_timeout: LOCK_TIMEOUT,
        }
    }

    /// Resolve a runtime for press (item 30b): the interpreter plus, when
    /// the release index carries an image entry, the runtime image —
    /// downloaded, verified and marked into the same cache entry the
    /// bootstrap consumes at first run. On a cache hit the image metadata
    /// comes from the entry's trusted marker; an entry whose marker is
    /// missing (installed before the image era, or partially wiped) has
    /// its image backfilled from the release index. A windows release
    /// (tebako-runtime-ruby#40) additionally carries the ruby DLL facet:
    /// it installs next to the executable under its PE name (`install_as`)
    /// whenever the index is in hand (the fresh install and the backfill
    /// alike); a contract-complete entry with no `dll` key installs the
    /// executable alone (every POSIX release).
    pub fn resolve_runtime(
        &self,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<Resolved, TebakoError> {
        let dir = self.entry_dir(ruby_version, platform, tebako_version);
        // The entry's own cached index flows the asset spelling verbatim
        // when it names this runtime (spec 05 §2 SSOT; tebako#456 — the
        // factory publishes windows exe assets SUFFIX-LESS, and the
        // bootstrap/shim install them under that spelling); the
        // pre-identity fallback synthesizes `{name}[.exe]`.
        let executable =
            dir.join(self.cached_exe_name(&dir, ruby_version, platform, tebako_version));
        let image_marker = || self.read_image_marker(&dir, ruby_version, platform, tebako_version);
        if executable.is_file() && image_marker().is_some() {
            return Ok(Resolved {
                executable,
                image: image_marker(),
            });
        }
        let entry_ref = self.entry_ref(ruby_version, platform, tebako_version);
        self.with_entry_lock(&dir, &entry_ref, || {
            if !executable.is_file() {
                let (entry, trust, mut signers) =
                    self.install(&dir, ruby_version, platform, tebako_version)?;
                if let Some(signer) = trust.signer() {
                    signers.push(signer.to_string());
                }
                if let Some(image) = entry.image.clone() {
                    signers.extend(self.install_image(&dir, &image, tebako_version)?);
                }
                if let Some(dll) = entry.dll.clone() {
                    signers.extend(self.install_dll(&dir, &dll, tebako_version)?);
                }
                self.journal_verified(&entry_ref, &signers);
                return Ok(());
            }
            // The exe is cached but its image marker is missing (an
            // entry installed before the image era, or a partial
            // wipe): backfill the image from the release index so the
            // entry is complete again.
            if image_marker().is_none() {
                self.offline_check(&entry_ref, tebako_version)?;
                let fetched = self.fetch_index(ruby_version, platform, tebako_version)?;
                let entry =
                    self.find_entry(&fetched.entries, ruby_version, platform, tebako_version)?;
                contract_gate(&entry_ref, fetched.card.as_deref(), &entry.filename)?;
                if matches!(fetched.trust, IndexTrust::Unverified) {
                    // The backfill fetches only the facets — the
                    // unsigned rule keys on THEIR declared
                    // signatures (the exe is already cached).
                    let declared = entry
                        .image
                        .as_ref()
                        .and_then(|i| i.signature.as_ref())
                        .is_some()
                        || entry
                            .dll
                            .as_ref()
                            .and_then(|d| d.signature.as_ref())
                            .is_some();
                    self.unsigned_gate(&entry_ref, declared, crate::install::require_signed())?;
                }
                let mut signers: Vec<String> = fetched
                    .trust
                    .signer()
                    .map(|s| s.to_string())
                    .into_iter()
                    .collect();
                if let Some(image) = entry.image.clone() {
                    signers.extend(self.install_image(&dir, &image, tebako_version)?);
                }
                if let Some(dll) = entry.dll.clone() {
                    signers.extend(self.install_dll(&dir, &dll, tebako_version)?);
                }
                self.journal_verified(&entry_ref, &signers);
            }
            Ok(())
        })?;
        // The install placed the exe under the index spelling and the
        // index rode into the entry — the name flows from it now.
        let executable =
            dir.join(self.cached_exe_name(&dir, ruby_version, platform, tebako_version));
        Ok(Resolved {
            executable,
            image: image_marker(),
        })
    }

    /// The dll facet of an ALREADY-RESOLVED runtime entry (windows,
    /// tebako-runtime-ruby#40), for press's lock: the installed path
    /// (under the PE name), the `install_as`, and the pin — the entry
    /// marker's first token (the verified sha of the installed file),
    /// else the index facet's sha256. `None` for every POSIX entry, for
    /// entries without the facet, and for a facet that never
    /// materialized (a sums-derived ref carries no PE name).
    pub fn resolved_dll(
        &self,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Option<ResolvedDll> {
        let dir = self.entry_dir(ruby_version, platform, tebako_version);
        let dll = self
            .cached_index_entry(&dir, ruby_version, platform, tebako_version)?
            .dll?;
        let install_as = dll.install_as?;
        let path = dir.join(&install_as);
        if !path.is_file() {
            return None;
        }
        let sha256 = fs::read_to_string(dir.join(format!("{install_as}.sha256")))
            .ok()
            .and_then(|body| body.split_whitespace().next().map(str::to_string))
            .filter(|s| !s.is_empty())
            .unwrap_or(dll.sha256);
        Some(ResolvedDll {
            path,
            install_as,
            sha256,
        })
    }

    /// The image's expected filename in a cache entry
    /// (`<asset-minus-exe-suffix>.tfs`).
    fn image_filename(&self, ruby_version: &str, platform: &str, tebako_version: &str) -> String {
        let asset = self.filename(ruby_version, platform, tebako_version);
        let base = asset.strip_suffix(".exe").unwrap_or(&asset);
        format!("{base}.tfs")
    }

    /// Read the entry's trusted image marker (`<image>.sha256`):
    /// Some only when both the image and its marker exist.
    fn read_image_marker(
        &self,
        dir: &Path,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Option<ImageRef> {
        let filename = self.cached_image_name(dir, ruby_version, platform, tebako_version);
        if !dir.join(&filename).is_file() {
            return None;
        }
        let marker = fs::read_to_string(dir.join(format!("{filename}.sha256"))).ok()?;
        let sha256 = marker.split_whitespace().next()?.to_string();
        if sha256.len() == 64 {
            // The trusted marker proves the sha256 gate passed at install
            // — the declared-signature check ran then, never per run.
            Some(ImageRef {
                filename,
                sha256,
                signature: None,
            })
        } else {
            None
        }
    }

    /// Download + verify + install the runtime image (0444 + trusted
    /// markers), sharing the bootstrap's cache layout (item 30b). A
    /// declared `signature:` block verifies against the downloaded bytes
    /// BEFORE the sha256 check (G1, spec 09 §4); the verified signer
    /// rides back for the audit journal. Called with the entry lock
    /// already held.
    fn install_image(
        &self,
        dir: &Path,
        image: &ImageRef,
        tebako_version: &str,
    ) -> Result<Option<String>, TebakoError> {
        let image_path = dir.join(&image.filename);
        let marker = dir.join(format!("{}.sha256", image.filename));
        if image_path.is_file() && marker.is_file() {
            return Ok(None);
        }
        let url = self.package_url(&image.filename, tebako_version);
        let tmp_dir = self.cache_root.join(TMP_DIR);
        let tmp = tmp_dir.join(format!("{}.{}.part", image.filename, std::process::id()));
        let bytes = match fetch_bytes(&url) {
            Ok(bytes) => bytes,
            Err(FetchError::IndexUnavailable(_)) => {
                return Err(packaging_error(122, Some(&format!("{url}: not found"))));
            }
            Err(e @ FetchError::Throttled { .. }) => {
                return Err(packaging_error(122, Some(&e.to_string())));
            }
            Err(FetchError::DownloadFailed(msg)) => {
                return Err(packaging_error(122, Some(&msg)));
            }
            Err(e) => {
                return Err(packaging_error(122, Some(&e.to_string())));
            }
        };
        let signer = image
            .signature
            .as_ref()
            .map(|sig| self.verify_declared(&bytes, &image.filename, sig, tebako_version))
            .transpose()?;
        if let Err(e) = crate::fetch::write_tmp(&tmp, &bytes) {
            let _ = fs::remove_file(&tmp);
            return Err(packaging_error(122, Some(&format!("{e} writing {url}"))));
        }
        let actual = sha256_file_hex(&tmp)
            .ok_or_else(|| packaging_error(121, Some(&format!("cannot hash {}", tmp.display()))))?;
        let expected = image.sha256.to_ascii_lowercase();
        if actual != expected {
            let _ = fs::remove_file(&tmp);
            return Err(packaging_error(
                121,
                Some(&format!(
                    "{}: expected {expected}, got {actual}; download deleted",
                    image.filename
                )),
            ));
        }
        let err = |e: std::io::Error| {
            crate::error::plain_error(format!("{e} installing {}", image_path.display()))
        };
        make_readonly(&tmp).map_err(err)?;
        fs::rename(&tmp, &image_path).map_err(err)?;
        fs::write(&marker, format!("{expected}  {}\n", image.filename)).map_err(err)?;
        fs::write(
            dir.join(format!("{}.origin", image.filename)),
            format!("{url}\n"),
        )
        .map_err(err)?;
        Ok(signer)
    }

    /// Download + verify + install the windows ruby DLL
    /// (tebako-runtime-ruby#40) next to the executable AS `install_as` —
    /// the PE name the exe and the extension .so's import (never the asset
    /// name: assets are unique per leg, two same-ABI legs share the PE
    /// name) — read-only with `<install_as>.sha256`/`<install_as>.origin`
    /// trusted markers, the image's exact discipline (declared signature
    /// before the sha256 check; the verified signer rides back for the
    /// audit journal). A ref without an `install_as` (the SHA256SUMS index
    /// form carries no PE name) is not installable: the dll facet is
    /// manifest-keyed. Called with the entry lock already held.
    fn install_dll(
        &self,
        dir: &Path,
        dll: &DllRef,
        tebako_version: &str,
    ) -> Result<Option<String>, TebakoError> {
        let Some(install_as) = dll.install_as.as_deref() else {
            return Ok(None);
        };
        // The PE name writes a file into the cache entry — a separator
        // would escape it; refuse by name, never install.
        if install_as.contains('/') || install_as.contains('\\') {
            return Err(packaging_error(
                122,
                Some(&format!(
                    "release index dll facet for {} carries an unusable install_as (\"{install_as}\") — the PE name must be a bare file name",
                    dll.filename
                )),
            ));
        }
        let dll_path = dir.join(install_as);
        let marker = dir.join(format!("{install_as}.sha256"));
        if dll_path.is_file() && marker.is_file() {
            return Ok(None);
        }
        let url = self.package_url(&dll.filename, tebako_version);
        let tmp_dir = self.cache_root.join(TMP_DIR);
        let tmp = tmp_dir.join(format!("{}.{}.part", dll.filename, std::process::id()));
        let bytes = match fetch_bytes(&url) {
            Ok(bytes) => bytes,
            Err(FetchError::IndexUnavailable(_)) => {
                return Err(packaging_error(122, Some(&format!("{url}: not found"))));
            }
            Err(e @ FetchError::Throttled { .. }) => {
                return Err(packaging_error(122, Some(&e.to_string())));
            }
            Err(FetchError::DownloadFailed(msg)) => {
                return Err(packaging_error(122, Some(&msg)));
            }
            Err(e) => {
                return Err(packaging_error(122, Some(&e.to_string())));
            }
        };
        let signer = dll
            .signature
            .as_ref()
            .map(|sig| self.verify_declared(&bytes, &dll.filename, sig, tebako_version))
            .transpose()?;
        if let Err(e) = crate::fetch::write_tmp(&tmp, &bytes) {
            let _ = fs::remove_file(&tmp);
            return Err(packaging_error(122, Some(&format!("{e} writing {url}"))));
        }
        let actual = sha256_file_hex(&tmp)
            .ok_or_else(|| packaging_error(121, Some(&format!("cannot hash {}", tmp.display()))))?;
        let expected = dll.sha256.to_ascii_lowercase();
        if actual != expected {
            let _ = fs::remove_file(&tmp);
            return Err(packaging_error(
                121,
                Some(&format!(
                    "{}: expected {expected}, got {actual}; download deleted",
                    dll.filename
                )),
            ));
        }
        let err = |e: std::io::Error| {
            crate::error::plain_error(format!("{e} installing {}", dll_path.display()))
        };
        make_readonly(&tmp).map_err(err)?;
        fs::rename(&tmp, &dll_path).map_err(err)?;
        fs::write(&marker, format!("{expected}  {install_as}\n")).map_err(err)?;
        fs::write(dir.join(format!("{install_as}.origin")), format!("{url}\n")).map_err(err)?;
        Ok(signer)
    }

    /// Extract the runtime package's filesystem layout next to the cached
    /// package (idempotent) and return the layout root.
    pub fn layout(&self, runtime_path: &Path, verbose: bool) -> Result<PathBuf, TebakoError> {
        let layout_dir = runtime_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("layout");
        if layout_dir.join("lib").is_dir() {
            return Ok(layout_dir);
        }
        fs::create_dir_all(&layout_dir).map_err(|e| {
            crate::error::plain_error(format!("cannot create {}: {e}", layout_dir.display()))
        })?;
        run_with_capture_v(
            runtime_path,
            &[
                "--tebako-extract".to_string(),
                layout_dir.to_string_lossy().into_owned(),
            ],
            &[],
            verbose,
        )?;
        Ok(layout_dir)
    }

    /// Cached entries (newest-dir listing like the gem's `entries`).
    pub fn entries(&self) -> Vec<CacheEntry> {
        let base = self.cache_root.join(CACHE_SUBDIR);
        let mut out = Vec::new();
        let Ok(children) = fs::read_dir(&base) else {
            return out;
        };
        let mut names: Vec<String> = children
            .filter_map(|c| c.ok())
            .filter(|c| c.path().is_dir())
            .map(|c| c.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            let path = base.join(&name);
            let installed_at = fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            out.push(CacheEntry {
                name,
                size_bytes: dir_size(&path),
                path,
                installed_at,
            });
        }
        out
    }

    /// Remove cached entries; returns the removed entry names.
    pub fn prune(
        &self,
        all: bool,
        older_than_days: Option<u64>,
    ) -> Result<Vec<String>, TebakoError> {
        if !all && older_than_days.is_none() {
            return Err(crate::error::plain_error(
                "prune requires :all or :older_than_days",
            ));
        }
        let cutoff = older_than_days
            .map(|d| std::time::SystemTime::now() - std::time::Duration::from_secs(d * 86_400));
        let mut removed = Vec::new();
        for entry in self.entries() {
            let drop_it = all || cutoff.is_some_and(|c| entry.installed_at < c);
            if drop_it {
                let _ = fs::remove_dir_all(&entry.path);
                removed.push(entry.name);
            }
        }
        Ok(removed)
    }

    // ---- entry naming ----------------------------------------------------

    fn entry_dir(&self, ruby_version: &str, platform: &str, tebako_version: &str) -> PathBuf {
        self.cache_root
            .join(CACHE_SUBDIR)
            .join(format!("ruby-{ruby_version}-{tebako_version}-{platform}"))
    }

    fn filename(&self, ruby_version: &str, platform: &str, tebako_version: &str) -> String {
        let suffix = if platform.starts_with("windows") {
            ".exe"
        } else {
            ""
        };
        format!("tebako-runtime-{tebako_version}-{ruby_version}-{platform}{suffix}")
    }

    /// The entry of the cache's own copy of the release index
    /// (`manifest.json`) matching this (ruby, platform) — `None` when the
    /// entry predates the cached index or never carried one (the
    /// SHA256SUMS-era fallback spells the names instead, spec 05 §2).
    fn cached_index_entry(
        &self,
        dir: &Path,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Option<IndexEntry> {
        let body = fs::read_to_string(dir.join("manifest.json")).ok()?;
        let index = self.parse_manifest(&body, tebako_version).ok()?;
        index.into_iter().find(|c| {
            c.ruby_version.as_deref() == Some(ruby_version)
                && c.platform.as_deref() == Some(platform)
        })
    }

    /// The exe asset name of a cache entry: the cached release index's
    /// `filename` flowed verbatim when the entry carries its index
    /// (spec 05 §2 SSOT; tebako#456 — the factory publishes windows exe
    /// assets SUFFIX-LESS), else the synthesized fallback.
    fn cached_exe_name(
        &self,
        dir: &Path,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> String {
        self.cached_index_entry(dir, ruby_version, platform, tebako_version)
            .map(|e| e.filename)
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| self.filename(ruby_version, platform, tebako_version))
    }

    /// Same for the env image: the cached entry's `image.filename`
    /// flowed verbatim, else the synthesized `<asset-minus-suffix>.tfs`.
    fn cached_image_name(
        &self,
        dir: &Path,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> String {
        self.cached_index_entry(dir, ruby_version, platform, tebako_version)
            .and_then(|e| e.image)
            .map(|i| i.filename)
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| self.image_filename(ruby_version, platform, tebako_version))
    }

    fn entry_ref(&self, ruby_version: &str, platform: &str, tebako_version: &str) -> String {
        format!("ruby@{ruby_version} (tebako {tebako_version}, {platform})")
    }

    // ---- install pipeline ------------------------------------------------

    fn install(
        &self,
        dir: &Path,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<(IndexEntry, IndexTrust, Vec<String>), TebakoError> {
        let entry_ref = self.entry_ref(ruby_version, platform, tebako_version);
        self.offline_check(&entry_ref, tebako_version)?;
        let fetched = self.fetch_index(ruby_version, platform, tebako_version)?;
        let entry = self.find_entry(&fetched.entries, ruby_version, platform, tebako_version)?;
        // spec 18 C2: the release card gates BEFORE the runtime download.
        contract_gate(&entry_ref, fetched.card.as_deref(), &entry.filename)?;
        // G1 (spec 09 §4): the unsigned rule decides BEFORE any asset
        // downloads — a declared signature on any artifact this run
        // installs counts as signed coverage.
        if matches!(fetched.trust, IndexTrust::Unverified) {
            let declared = entry.signature.is_some()
                || entry
                    .image
                    .as_ref()
                    .and_then(|i| i.signature.as_ref())
                    .is_some()
                || entry
                    .dll
                    .as_ref()
                    .and_then(|d| d.signature.as_ref())
                    .is_some();
            self.unsigned_gate(&entry_ref, declared, crate::install::require_signed())?;
        }
        let url = self.package_url(&entry.filename, tebako_version);
        let (tmp, bytes) = self.download(&url, &entry.filename)?;
        let mut signers = Vec::new();
        if let Some(sig) = &entry.signature {
            match self.verify_declared(&bytes, &entry.filename, sig, tebako_version) {
                Ok(signer) => signers.push(signer),
                Err(e) => {
                    let _ = fs::remove_file(&tmp);
                    return Err(e);
                }
            }
        }
        self.verify(&tmp, entry)?;
        // The exe installs under the index entry's `filename` verbatim
        // (spec 05 §2 SSOT; tebako#456 — the factory publishes windows
        // exe assets SUFFIX-LESS), and the index rides into the cache
        // entry so every consumer flows the spelling from it.
        let executable = dir.join(&entry.filename);
        self.place(&tmp, &executable, entry, &url)?;
        if let Some(card) = &fetched.card {
            let card_path = dir.join("manifest.json");
            fs::write(&card_path, card).map_err(|e| {
                crate::error::plain_error(format!("{e} installing {}", card_path.display()))
            })?;
        }
        let entry = entry.clone();
        Ok((entry, fetched.trust, signers))
    }

    fn offline(&self) -> bool {
        offline_env()
    }

    fn offline_check(&self, entry_ref: &str, tebako_version: &str) -> Result<(), TebakoError> {
        if !self.offline() {
            return Ok(());
        }
        Err(packaging_error(
            123,
            Some(&format!(
                "{} is not cached and downloads are disabled (release index: {}; {}={})",
                entry_ref,
                self.index_urls(tebako_version).join(", "),
                MIRROR_ENV_VAR,
                self.mirror
            )),
        ))
    }

    fn find_entry<'a>(
        &self,
        index: &'a [IndexEntry],
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<&'a IndexEntry, TebakoError> {
        if let Some(entry) = index.iter().find(|c| {
            c.ruby_version.as_deref() == Some(ruby_version)
                && c.platform.as_deref() == Some(platform)
        }) {
            return Ok(entry);
        }
        let combos: Vec<String> = index
            .iter()
            .map(|c| {
                format!(
                    "{}/{}",
                    c.ruby_version.as_deref().unwrap_or("?"),
                    c.platform.as_deref().unwrap_or("?")
                )
            })
            .collect();
        Err(packaging_error(
            120,
            Some(&format!(
                "no package for ruby {ruby_version} on {platform} (tebako {tebako_version}). Available: {}. Use --build-runtime to build the runtime from source instead.",
                combos.join(", ")
            )),
        ))
    }

    fn verify(&self, tmp: &Path, entry: &IndexEntry) -> Result<(), TebakoError> {
        let actual = sha256_file_hex(tmp)
            .ok_or_else(|| packaging_error(121, Some(&format!("cannot hash {}", tmp.display()))))?;
        let expected = entry.sha256.to_ascii_lowercase();
        if actual == expected {
            return Ok(());
        }
        let _ = fs::remove_file(tmp);
        Err(packaging_error(
            121,
            Some(&format!(
                "{}: expected {expected}, got {actual}; download deleted",
                entry.filename
            )),
        ))
    }

    fn place(
        &self,
        tmp: &Path,
        executable: &Path,
        entry: &IndexEntry,
        url: &str,
    ) -> Result<(), TebakoError> {
        let err = |e: std::io::Error| {
            crate::error::plain_error(format!("{e} installing {}", executable.display()))
        };
        make_executable(tmp).map_err(err)?;
        fs::rename(tmp, executable).map_err(err)?;
        let dir = executable.parent().unwrap_or_else(|| Path::new("."));
        fs::write(dir.join(SHA256_FILE), format!("{}\n", entry.sha256)).map_err(err)?;
        fs::write(dir.join(ORIGIN_FILE), format!("{url}\n")).map_err(err)?;
        Ok(())
    }

    /// The release index plus, when it parsed from a JSON card (the
    /// per-package shard or the monolithic manifest.json), the raw card
    /// text (the spec 18 contract gate reads it ahead of the download —
    /// no second fetch of the index), plus the form's trust outcome.
    ///
    /// G1 (spec 09 §4/§5) runs as a trust scan FIRST
    /// ([`Self::fetch_verified_index`]): every index form's detached
    /// `.asc` sibling is probed in the index's preference order and the
    /// first form whose signature verifies TRUSTED is the consumed form
    /// — a verified form beats an unsigned one regardless of preference
    /// order. Only when NO form carries a verifiable signature does the
    /// legacy chain run (the per-package shard `<stem>.manifest.json` —
    /// the sidecar-era authority, one small object carrying exactly this
    /// triple's entry — then the derived monoliths `manifest.json`, then
    /// `SHA256SUMS.txt`; tebako#493, spec 05 §2), and its outcome is
    /// Unverified: the caller applies the unsigned rule. The fallbacks
    /// stay forever: pre-shard releases are immutable and remain
    /// installable (invariant 7).
    fn fetch_index(
        &self,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<FetchedIndex, TebakoError> {
        let mut tried: Vec<String> = Vec::new();
        if let Some(verified) =
            self.fetch_verified_index(ruby_version, platform, tebako_version, &mut tried)?
        {
            return Ok(verified);
        }
        if let Some((entries, card)) =
            self.fetch_shard(ruby_version, platform, tebako_version, &mut tried)?
        {
            return Ok(FetchedIndex {
                entries,
                card,
                trust: IndexTrust::Unverified,
            });
        }
        for name in INDEX_FILES {
            let url = self.index_url(name, tebako_version);
            match fetch_text(&url) {
                Ok(body) => match self.parse_index(name, &body, tebako_version) {
                    Ok(entries) => {
                        let card = (*name == "manifest.json").then_some(body);
                        return Ok(FetchedIndex {
                            entries,
                            card,
                            trust: IndexTrust::Unverified,
                        });
                    }
                    Err(ParseFail::Unavailable(msg)) => tried.push(format!("{url} ({msg})")),
                    Err(ParseFail::Torn(msg)) => return Err(TebakoError::new(msg, 65)),
                },
                Err(FetchError::IndexUnavailable(_)) => tried.push(url),
                Err(e @ FetchError::Throttled { .. }) => {
                    return Err(packaging_error(122, Some(&e.to_string())));
                }
                Err(FetchError::DownloadFailed(msg)) => {
                    return Err(packaging_error(122, Some(&msg)))
                }
                Err(e) => return Err(packaging_error(122, Some(&e.to_string()))),
            }
        }
        Err(packaging_error(
            124,
            Some(&format!(
                "{} release v{} provides no usable package index (tried: {})",
                RELEASE_NAME,
                tebako_version,
                tried.join(", ")
            )),
        ))
    }

    /// The G1 trust scan (spec 09 §4/§5): each index form's detached
    /// `.asc` sibling is probed in the index's preference order (the
    /// per-package shard, then INDEX_FILES); the FIRST form whose
    /// signature verifies TRUSTED against the keyring is the consumed
    /// form — entries AND the contract card come from those verified
    /// bytes. A present `.asc` that does not verify is a hard 71 (the
    /// signed bytes are corrupt or tampered); one whose signer is not
    /// in the keyring is a hard 72 (register the publisher's key
    /// first); neither falls through to an unsigned form. A form
    /// without an `.asc` is skipped here and left for the legacy chain.
    /// `Ok(None)` = nothing verifiable.
    fn fetch_verified_index(
        &self,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
        tried: &mut Vec<String>,
    ) -> Result<Option<FetchedIndex>, TebakoError> {
        let shard =
            format!("tebako-runtime-{tebako_version}-{ruby_version}-{platform}.manifest.json");
        let mut forms: Vec<&str> = vec![shard.as_str()];
        forms.extend(INDEX_FILES);
        let mut keyring: Option<Vec<u8>> = None;
        for name in forms {
            let asc_url = self.index_url(&format!("{name}.asc"), tebako_version);
            let asc = match fetch_bytes(&asc_url) {
                Ok(bytes) => bytes,
                Err(FetchError::IndexUnavailable(_)) => continue,
                Err(e @ FetchError::Throttled { .. }) => {
                    return Err(packaging_error(122, Some(&e.to_string())));
                }
                Err(FetchError::DownloadFailed(msg)) => {
                    return Err(packaging_error(122, Some(&msg)));
                }
                Err(e) => return Err(packaging_error(122, Some(&e.to_string()))),
            };
            let url = self.index_url(name, tebako_version);
            let body = match fetch_text(&url) {
                Ok(body) => body,
                Err(FetchError::IndexUnavailable(_)) => {
                    tried.push(url);
                    continue;
                }
                Err(e @ FetchError::Throttled { .. }) => {
                    return Err(packaging_error(122, Some(&e.to_string())));
                }
                Err(FetchError::DownloadFailed(msg)) => {
                    return Err(packaging_error(122, Some(&msg)));
                }
                Err(e) => return Err(packaging_error(122, Some(&e.to_string()))),
            };
            if keyring.is_none() {
                keyring = Some(self.keyring()?);
            }
            let keyring = keyring.as_deref().expect("built above");
            let signer = match tebako_signer::verify_detached_full(keyring, body.as_bytes(), &asc) {
                Ok(tebako_signer::VerifyOutcome::Trusted(signer)) => {
                    // The signature may issue from a signing subkey —
                    // the identity that journals is the resolved
                    // PRIMARY keyid (spec 09 §9).
                    let signer = signer.to_ascii_lowercase();
                    tebako_signer::primary_keyid_of(keyring, &signer)
                        .map_err(|e| TebakoError::new(format!("{name}.asc: {e}"), 71))?
                        .unwrap_or(signer)
                }
                Ok(tebako_signer::VerifyOutcome::Untrusted(keyid)) => {
                    return Err(TebakoError::new(
                        format!(
                            "the {RELEASE_NAME} release index {name} (v{tebako_version}) is signed by {keyid}, which is not in the trusted keyring — register the publisher's key (~/.tebako/keyring/trusted.pgp), then retry; nothing was cached"
                        ),
                        72,
                    ));
                }
                Ok(tebako_signer::VerifyOutcome::Invalid(keyid)) => {
                    return Err(TebakoError::new(
                        format!(
                            "the {RELEASE_NAME} release index {name} (v{tebako_version}) carries a signature that does not verify (signer {}) — the index or its signature is corrupt; nothing was cached",
                            keyid.unwrap_or_else(|| "unknown".to_string())
                        ),
                        71,
                    ));
                }
                Err(e) => {
                    return Err(TebakoError::new(
                        format!("cannot verify the release index signature {name}.asc: {e}"),
                        71,
                    ));
                }
            };
            let parsed: Result<IndexAndCard, ParseFail> = if name == shard.as_str() {
                self.parse_shard(&body, ruby_version, platform, tebako_version)
                    .map(|entry| (vec![entry], Some(format!("[{body}]"))))
            } else {
                self.parse_index(name, &body, tebako_version)
                    .map(|entries| (entries, (name == "manifest.json").then_some(body)))
            };
            match parsed {
                Ok((entries, card)) => {
                    crate::install::journal(
                        &self.cache_root,
                        &format!("event=runtime-index-verified form={name} signer={signer}"),
                    );
                    return Ok(Some(FetchedIndex {
                        entries,
                        card,
                        trust: IndexTrust::Verified { signer },
                    }));
                }
                // A verified form that cannot serve this request (the
                // shard declares another triple) falls through like any
                // unusable form — a later form may still verify.
                Err(ParseFail::Unavailable(msg)) => {
                    tried.push(format!("{url} ({msg})"));
                    continue;
                }
                Err(ParseFail::Torn(msg)) => return Err(TebakoError::new(msg, 65)),
            }
        }
        Ok(None)
    }

    fn parse_index(
        &self,
        name: &str,
        body: &str,
        tebako_version: &str,
    ) -> Result<Vec<IndexEntry>, ParseFail> {
        if name == "manifest.json" {
            self.parse_manifest(body, tebako_version)
        } else {
            Ok(self.parse_sha256sums(body, tebako_version))
        }
    }

    /// The tebako-runtime-ruby manifest.json is an array of
    /// {tebako_version, ruby_version, platform, filename, sha256, ...}.
    fn parse_manifest(
        &self,
        body: &str,
        tebako_version: &str,
    ) -> Result<Vec<IndexEntry>, ParseFail> {
        let data = json_parse(body)
            .map_err(|_| ParseFail::Unavailable("manifest.json is not valid JSON".to_string()))?;
        let JsonValue::Array(items) = data else {
            return Err(ParseFail::Unavailable(
                "manifest.json is not an array".to_string(),
            ));
        };
        let mut out = Vec::new();
        for e in items.iter().filter(|e| {
            e.find("tebako_version")
                .and_then(|v| v.as_string())
                .as_deref()
                == Some(tebako_version)
        }) {
            if let Some(entry) = entry_from_json(e)? {
                out.push(entry);
            }
        }
        Ok(out)
    }

    /// Preference 1 of the sidecar era (tebako#493): the per-package
    /// shard `<stem>.manifest.json`, the stem being the exe asset name
    /// (`tebako-runtime-<tv>-<rv>-<platform>` — suffix-less, the factory
    /// spelling locked by tebako#456, on windows too). `Ok(None)` when
    /// the release carries no usable shard for the triple (a pre-shard
    /// release, or a shard that cannot serve it) — the caller falls
    /// through to the monoliths with the shard URL recorded in `tried`.
    fn fetch_shard(
        &self,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
        tried: &mut Vec<String>,
    ) -> Result<Option<IndexAndCard>, TebakoError> {
        let stem = format!("tebako-runtime-{tebako_version}-{ruby_version}-{platform}");
        let url = self.index_url(&format!("{stem}.manifest.json"), tebako_version);
        let body = match fetch_text(&url) {
            Ok(body) => body,
            Err(FetchError::IndexUnavailable(_)) => {
                tried.push(url);
                return Ok(None);
            }
            Err(e @ FetchError::Throttled { .. }) => {
                return Err(packaging_error(122, Some(&e.to_string())));
            }
            Err(FetchError::DownloadFailed(msg)) => {
                return Err(packaging_error(122, Some(&msg)));
            }
            Err(e) => {
                return Err(packaging_error(122, Some(&e.to_string())));
            }
        };
        match self.parse_shard(&body, ruby_version, platform, tebako_version) {
            Ok(entry) => {
                // The card the contract gate reads: the shard normalized
                // to the manifest.json array shape — tebako-resolve owns
                // the release-card reader semantics and expects the
                // array (spec 05 §2: the shard is the same entry, served
                // standalone).
                let card = format!("[{body}]");
                Ok(Some((vec![entry], Some(card))))
            }
            Err(ParseFail::Unavailable(msg)) => {
                tried.push(format!("{url} ({msg})"));
                Ok(None)
            }
            Err(ParseFail::Torn(msg)) => Err(TebakoError::new(msg, 65)),
        }
    }

    /// A per-package shard is ONE manifest-entry object (the owning
    /// platform's publish serves its own entry as its own asset, spec 13
    /// §3). The shard was fetched under the name derived from the
    /// requested triple — a shard declaring a different triple cannot
    /// serve this request (IndexUnavailable: fall through to the
    /// monoliths with the URL recorded).
    fn parse_shard(
        &self,
        body: &str,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<IndexEntry, ParseFail> {
        let data = json_parse(body).map_err(|_| {
            ParseFail::Unavailable("the package shard is not valid JSON".to_string())
        })?;
        if !matches!(data, JsonValue::Object(_)) {
            return Err(ParseFail::Unavailable(
                "the package shard is not an object".to_string(),
            ));
        }
        let declares = |key: &str| data.find(key).and_then(|v| v.as_string());
        if declares("tebako_version").as_deref() != Some(tebako_version)
            || declares("ruby_version").as_deref() != Some(ruby_version)
            || declares("platform").as_deref() != Some(platform)
        {
            return Err(ParseFail::Unavailable(format!(
                "the package shard declares {}/{}/{} — requested {}/{}/{}",
                declares("tebako_version").as_deref().unwrap_or("?"),
                declares("ruby_version").as_deref().unwrap_or("?"),
                declares("platform").as_deref().unwrap_or("?"),
                tebako_version,
                ruby_version,
                platform
            )));
        }
        entry_from_json(&data)?.ok_or_else(|| {
            ParseFail::Unavailable("the package shard carries no filename/sha256".to_string())
        })
    }

    /// `<sha256>  <filename>` lines; filenames may carry a `*` prefix.
    /// Runtime lines may name the image sibling (`<asset>.tfs`, item
    /// 30b) or the windows ruby DLL sibling (`<asset>.dll`,
    /// tebako-runtime-ruby#40): they attach to the matching runtime entry
    /// as its `image` / `dll`. The sums form carries no PE name, so a
    /// sums-derived dll ref has no `install_as` (not installable — the
    /// dll facet is manifest-keyed).
    fn parse_sha256sums(&self, body: &str, tebako_version: &str) -> Vec<IndexEntry> {
        let mut out: Vec<IndexEntry> = Vec::new();
        let mut images: Vec<(String, String, String)> = Vec::new(); // (rv, platform, ImageRef parts)
        let mut dlls: Vec<(String, String, String)> = Vec::new(); // (rv, platform, DllRef parts)
        for line in body.lines() {
            let mut parts = line.trim().splitn(2, char::is_whitespace);
            let (Some(sha256), Some(file)) = (parts.next(), parts.next()) else {
                continue;
            };
            let file = file.trim().trim_start_matches('*');
            let prefix = format!("tebako-runtime-{tebako_version}-");
            let Some(rest) = file.strip_prefix(&prefix) else {
                continue;
            };
            // The image sibling: strip .tfs instead of .exe and
            // record for the second pass.
            if let Some(rest) = rest.strip_suffix(".tfs") {
                if let Some((ruby_version, platform)) = split_ruby_platform(rest) {
                    images.push((ruby_version, platform, format!("{file}|{sha256}")));
                }
                continue;
            }
            // The ruby DLL sibling (tebako-runtime-ruby#40): the
            // same treatment.
            if let Some(rest) = rest.strip_suffix(".dll") {
                if let Some((ruby_version, platform)) = split_ruby_platform(rest) {
                    dlls.push((ruby_version, platform, format!("{file}|{sha256}")));
                }
                continue;
            }
            let rest = rest.strip_suffix(".exe").unwrap_or(rest);
            let Some((ruby_version, platform)) = split_ruby_platform(rest) else {
                continue;
            };
            out.push(IndexEntry {
                ruby_version: Some(ruby_version),
                platform: Some(platform),
                filename: file.to_string(),
                sha256: sha256.to_ascii_lowercase(),
                signature: None,
                image: None,
                dll: None,
            });
        }
        for (ruby_version, platform, parts) in images {
            let Some((filename, sha256)) = parts.split_once('|') else {
                continue;
            };
            if let Some(entry) = out.iter_mut().find(|e| {
                e.ruby_version.as_deref() == Some(ruby_version.as_str())
                    && e.platform.as_deref() == Some(platform.as_str())
            }) {
                entry.image = Some(ImageRef {
                    filename: filename.to_string(),
                    sha256: sha256.to_ascii_lowercase(),
                    signature: None,
                });
            }
        }
        for (ruby_version, platform, parts) in dlls {
            let Some((filename, sha256)) = parts.split_once('|') else {
                continue;
            };
            if let Some(entry) = out.iter_mut().find(|e| {
                e.ruby_version.as_deref() == Some(ruby_version.as_str())
                    && e.platform.as_deref() == Some(platform.as_str())
            }) {
                entry.dll = Some(DllRef {
                    filename: filename.to_string(),
                    install_as: None,
                    sha256: sha256.to_ascii_lowercase(),
                    signature: None,
                });
            }
        }
        out
    }

    /// Fetch `url` into the store's tmp/ dir; returns the tmp path AND
    /// the bytes (a declared `signature:` block verifies against the
    /// bytes in memory — before they are ever re-read from disk).
    fn download(&self, url: &str, filename: &str) -> Result<(PathBuf, Vec<u8>), TebakoError> {
        let tmp_dir = self.cache_root.join(TMP_DIR);
        fs::create_dir_all(&tmp_dir).map_err(|e| {
            crate::error::plain_error(format!("{e} creating {}", tmp_dir.display()))
        })?;
        let tmp = tmp_dir.join(format!("{filename}.{}.part", std::process::id()));
        match fetch_bytes(url) {
            Ok(bytes) => {
                if let Err(e) = crate::fetch::write_tmp(&tmp, &bytes) {
                    let _ = fs::remove_file(&tmp);
                    return Err(packaging_error(122, Some(&format!("{e} writing {url}"))));
                }
                Ok((tmp, bytes))
            }
            Err(FetchError::IndexUnavailable(_)) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(122, Some(&format!("{url}: not found"))))
            }
            Err(e @ FetchError::Throttled { .. }) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(122, Some(&e.to_string())))
            }
            Err(FetchError::DownloadFailed(msg)) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(122, Some(&msg)))
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(122, Some(&e.to_string())))
            }
        }
    }

    // ---- trust (spec 09 §4/§5, roadmap 80 G1) ------------------------

    /// The spec 09 §9 zero-interaction keyring: the user's trusted
    /// keyring plus the embedded tamatebako root (and the
    /// TEBAKO_TRUSTED_ROOT dev override). Built lazily: an unsigned
    /// release never touches it.
    fn keyring(&self) -> Result<Vec<u8>, TebakoError> {
        tebako_signer::verification_keyring(&self.cache_root).map_err(|e| {
            TebakoError::new(format!("cannot build the verification keyring: {e}"), 74)
        })
    }

    /// Verify one artifact's declared `signature:` block (spec 13 §2a)
    /// against its downloaded bytes BEFORE the sha256 check: the `.asc`
    /// fetches from the same release, the signer must be in the keyring
    /// AND be the pinned key (a signing subkey pins by its resolved
    /// PRIMARY keyid, spec 09 §9). Returns the primary keyid that
    /// verified (the audit journal's signer identity).
    fn verify_declared(
        &self,
        bytes: &[u8],
        asset: &str,
        sig: &EntrySignature,
        tebako_version: &str,
    ) -> Result<String, TebakoError> {
        // The asc names a sibling asset of the same release — a
        // separator would escape it; refuse by name, never fetch.
        if sig.asc.contains('/') || sig.asc.contains('\\') {
            return Err(TebakoError::new(
                format!(
                    "{asset} declares a signature asset (\"{}\") that is not a bare file name",
                    sig.asc
                ),
                65,
            ));
        }
        let asc_url = self.package_url(&sig.asc, tebako_version);
        let asc = fetch_bytes(&asc_url).map_err(|e| {
            TebakoError::new(
                format!("{asset} declares a signature but {asc_url} did not fetch: {e}"),
                71,
            )
        })?;
        let keyring = self.keyring()?;
        let issuer = match tebako_signer::verify_detached_full(&keyring, bytes, &asc) {
            Ok(tebako_signer::VerifyOutcome::Trusted(issuer)) => issuer.to_ascii_lowercase(),
            Ok(tebako_signer::VerifyOutcome::Untrusted(keyid)) => {
                return Err(TebakoError::new(
                    format!(
                        "{asset} is signed by {keyid}, which is not in the trusted keyring — register the publisher's key (~/.tebako/keyring/trusted.pgp), then retry; nothing was cached"
                    ),
                    72,
                ));
            }
            Ok(tebako_signer::VerifyOutcome::Invalid(keyid)) => {
                return Err(TebakoError::new(
                    format!(
                        "{asset} carries a signature that does not verify (signer {}) — the download or its signature is corrupt; nothing was cached",
                        keyid.unwrap_or_else(|| "unknown".to_string())
                    ),
                    71,
                ));
            }
            Err(e) => {
                return Err(TebakoError::new(
                    format!("cannot verify the declared signature of {asset}: {e}"),
                    71,
                ));
            }
        };
        // The signature may issue from a signing subkey — the identity
        // the pin names is the resolved PRIMARY keyid (spec 09 §9).
        let primary = tebako_signer::primary_keyid_of(&keyring, &issuer)
            .map_err(|e| TebakoError::new(format!("{asset}: {e}"), 71))?;
        let pin = sig.keyid.to_ascii_lowercase();
        if issuer != pin && primary.as_deref() != Some(pin.as_str()) {
            return Err(TebakoError::new(
                format!(
                    "{asset} declares signer {pin} but was signed by {issuer} — the signer key changed; nothing was cached"
                ),
                72,
            ));
        }
        Ok(primary.unwrap_or(issuer))
    }

    /// The spec 09 §5 unsigned rule, applied when the release index
    /// could not be verified and BEFORE any asset downloads: a declared
    /// signature on any artifact this run installs counts as signed
    /// coverage; TEBAKO_REQUIRE_SIGNED=1 refuses the unsigned fetch
    /// (exit 71, nothing cached); otherwise the fetch proceeds LOUDLY
    /// (stderr + the audit journal).
    fn unsigned_gate(
        &self,
        entry_ref: &str,
        declared: bool,
        require_signed: bool,
    ) -> Result<(), TebakoError> {
        if declared {
            return Ok(());
        }
        if require_signed {
            return Err(TebakoError::new(
                format!(
                    "{entry_ref} is unsigned — the release index carries no verifiable signature and no artifact declares one — and TEBAKO_REQUIRE_SIGNED=1 is set — refusing to fetch; nothing was cached"
                ),
                71,
            ));
        }
        eprintln!(
            "tebako: warning: {entry_ref} is unsigned (the release carries no verifiable signature) — installing with sha256 verification only; set TEBAKO_REQUIRE_SIGNED=1 to refuse unsigned runtimes"
        );
        crate::install::journal(
            &self.cache_root,
            &format!(
                "event=unsigned-runtime-fetch runtime_ref={entry_ref} mirror={}",
                self.mirror
            ),
        );
        Ok(())
    }

    /// Journal the verified signers of one runtime fetch (the index
    /// form's plus every declared artifact's), one deduped line.
    fn journal_verified(&self, entry_ref: &str, signers: &[String]) {
        if signers.is_empty() {
            return;
        }
        let mut signers = signers.to_vec();
        signers.sort();
        signers.dedup();
        crate::install::journal(
            &self.cache_root,
            &format!(
                "event=runtime-fetch-verified runtime_ref={entry_ref} mirror={} signer={}",
                self.mirror,
                signers.join(",")
            ),
        );
    }

    // ---- locking ---------------------------------------------------------

    fn with_entry_lock<F>(&self, dir: &Path, entry_ref: &str, f: F) -> Result<(), TebakoError>
    where
        F: FnOnce() -> Result<(), TebakoError>,
    {
        with_install_lock(dir, entry_ref, self.lock_timeout, "runtime", 125, f)
    }

    // ---- urls ------------------------------------------------------------

    fn release_url(&self, tebako_version: &str) -> String {
        format!(
            "{}/v{}",
            self.mirror,
            tebako_version.strip_prefix('v').unwrap_or(tebako_version)
        )
    }

    fn index_url(&self, name: &str, tebako_version: &str) -> String {
        format!("{}/{name}", self.release_url(tebako_version))
    }

    fn index_urls(&self, tebako_version: &str) -> Vec<String> {
        INDEX_FILES
            .iter()
            .map(|n| self.index_url(n, tebako_version))
            .collect()
    }

    fn package_url(&self, filename: &str, tebako_version: &str) -> String {
        format!("{}/{filename}", self.release_url(tebako_version))
    }
}

/// One manifest entry's JSON → the resolver's index entry: the exe
/// identity (`filename`, `sha256` — both mandatory, anything less is no
/// entry) plus the additive facets (the env image; the windows ruby DLL,
/// tebako-runtime-ruby#40 — absent on every POSIX entry, ignored by
/// consumers that predate it) and the declared signature blocks (spec 13
/// §2a — a PRESENT but torn block is fatal, never a silent downgrade to
/// unsigned). Shared by the monolithic manifest's array items and the
/// per-package shard (tebako#493).
fn entry_from_json(e: &JsonValue) -> Result<Option<IndexEntry>, ParseFail> {
    let Some(filename) = e.find("filename").and_then(|v| v.as_string()) else {
        return Ok(None);
    };
    let Some(sha256) = e.find("sha256").and_then(|v| v.as_string()) else {
        return Ok(None);
    };
    let image = match e.find("image") {
        Some(img) => {
            let filename = img.find("filename").and_then(|v| v.as_string());
            let sha256 = img.find("sha256").and_then(|v| v.as_string());
            match (filename, sha256) {
                (Some(filename), Some(sha256)) => Some(ImageRef {
                    filename,
                    sha256: sha256.to_ascii_lowercase(),
                    signature: declared_signature(e, Some("image"))?,
                }),
                _ => None,
            }
        }
        None => None,
    };
    let dll = match e.find("dll") {
        Some(d) => {
            let filename = d.find("filename").and_then(|v| v.as_string());
            let sha256 = d.find("sha256").and_then(|v| v.as_string());
            match (filename, sha256) {
                (Some(filename), Some(sha256)) => Some(DllRef {
                    filename,
                    install_as: d.find("install_as").and_then(|v| v.as_string()),
                    sha256: sha256.to_ascii_lowercase(),
                    signature: declared_signature(e, Some("dll"))?,
                }),
                _ => None,
            }
        }
        None => None,
    };
    Ok(Some(IndexEntry {
        ruby_version: e.find("ruby_version").and_then(|v| v.as_string()),
        platform: e.find("platform").and_then(|v| v.as_string()),
        filename,
        sha256: sha256.to_ascii_lowercase(),
        signature: declared_signature(e, None)?,
        image,
        dll,
    }))
}

/// The declared signature block of one entry facet (spec 13 §2a) — the
/// tpkg helper's torn-block error is fatal here (`ParseFail::Torn`).
fn declared_signature(
    entry: &JsonValue,
    facet: Option<&str>,
) -> Result<Option<EntrySignature>, ParseFail> {
    tpkg::runtime_store::entry_signature(entry, facet).map_err(ParseFail::Torn)
}

// ---------------------------------------------------------------------
// The Rust bootstrap store (spec 19 §4)
// ---------------------------------------------------------------------

/// The product's own release mirror — the per-triplet Rust
/// tebako-bootstrap assets publish here (TEBAKO_BOOTSTRAP_MIRROR
/// overrides; tests use file://).
const DEFAULT_BOOTSTRAP_MIRROR: &str = "https://github.com/tamatebako/tebako/releases/download";
const BOOTSTRAP_MIRROR_ENV_VAR: &str = "TEBAKO_BOOTSTRAP_MIRROR";
/// The store subdirectory the bootstrap resolves into
/// (`bootstraps/<version>-<triplet>/`, spec 19 §4).
const BOOTSTRAP_CACHE_SUBDIR: &str = "bootstraps";
/// The product release's DERIVED monoliths, in preference order — the
/// fallback chain behind the per-asset sidecar
/// (`tebako-bootstrap-<version>-<platform>[.exe].sha256`, tebako#493),
/// consulted for pre-sidecar releases and kept forever (invariant 7).
/// The manifest's top-level `assets` array is exactly the bootstrap set
/// and SHA256SUMS carries one line per tool asset (both authored by
/// .github/workflows/lib/finalize.sh).
const BOOTSTRAP_INDEX_FILES: &[&str] = &["manifest.json", "SHA256SUMS"];

/// One bootstrap asset in the release index (platform + filename +
/// expected sha256).
#[derive(Debug, Clone)]
pub struct BootstrapEntry {
    pub platform: String,
    pub filename: String,
    pub sha256: String,
}

/// Resolves the Rust tebako-bootstrap a press stitches with (spec 19
/// §4): the asset published with the product release matching the CLI's
/// own version (`env!("CARGO_PKG_VERSION")` — the bootstrap and the CLI
/// ship in one release, so the versions never drift). A store hit
/// returns immediately (no lock, no network); a miss downloads the
/// asset sha256-verified against the release index into
/// `bootstraps/<version>-<triplet>/` under the entry lock, tmp + rename
/// so a partial download stays invisible. `TEBAKO_OFFLINE=1` is
/// cache-or-named-error (138).
#[derive(Debug)]
pub struct BootstrapResolver {
    pub cache_root: PathBuf,
    pub mirror: String,
    pub version: String,
    pub offline: bool,
    pub lock_timeout: std::time::Duration,
}

impl Default for BootstrapResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BootstrapResolver {
    pub fn new() -> Self {
        let mirror = std::env::var(BOOTSTRAP_MIRROR_ENV_VAR)
            .ok()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| DEFAULT_BOOTSTRAP_MIRROR.to_string());
        BootstrapResolver {
            cache_root: default_cache_root(),
            mirror: mirror.trim_end_matches('/').to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            offline: offline_env(),
            lock_timeout: LOCK_TIMEOUT,
        }
    }

    /// The bootstrap binary for `platform` (a release-asset triplet like
    /// `macos-arm64`): the cached store entry when present, else the
    /// downloaded + verified + installed one.
    pub fn resolve(&self, platform: &str) -> Result<PathBuf, TebakoError> {
        let dir = self.entry_dir(platform);
        let binary = dir.join(self.filename(platform));
        if binary.is_file() {
            return Ok(binary);
        }
        with_install_lock(
            &dir,
            &self.entry_ref(platform),
            self.lock_timeout,
            "bootstrap",
            142,
            || {
                // Re-check under the lock: a concurrent press may have
                // installed the entry while we waited.
                if binary.is_file() {
                    return Ok(());
                }
                self.install(&binary, platform)
            },
        )?;
        Ok(binary)
    }

    // ---- entry naming ----------------------------------------------------

    fn entry_dir(&self, platform: &str) -> PathBuf {
        self.cache_root
            .join(BOOTSTRAP_CACHE_SUBDIR)
            .join(format!("{}-{platform}", self.version))
    }

    fn filename(&self, platform: &str) -> String {
        let suffix = if platform.starts_with("windows") {
            ".exe"
        } else {
            ""
        };
        format!("tebako-bootstrap-{}-{platform}{suffix}", self.version)
    }

    fn entry_ref(&self, platform: &str) -> String {
        format!("tebako-bootstrap@{} ({platform})", self.version)
    }

    // ---- install pipeline ------------------------------------------------

    fn install(&self, binary: &Path, platform: &str) -> Result<(), TebakoError> {
        let entry_ref = self.entry_ref(platform);
        self.offline_check(&entry_ref)?;
        let index = self.fetch_index(platform)?;
        let entry = self.find_entry(&index, platform)?;
        let url = self.asset_url(&entry.filename);
        let tmp = self.download(&url, &entry.filename)?;
        self.verify(&tmp, entry)?;
        self.place(&tmp, binary, entry, &url)?;
        Ok(())
    }

    fn offline_check(&self, entry_ref: &str) -> Result<(), TebakoError> {
        if !self.offline {
            return Ok(());
        }
        Err(packaging_error(
            138,
            Some(&format!(
                "{} is not cached and downloads are disabled (release index: {}; {}={})",
                entry_ref,
                self.index_urls().join(", "),
                BOOTSTRAP_MIRROR_ENV_VAR,
                self.mirror
            )),
        ))
    }

    fn find_entry<'a>(
        &self,
        index: &'a [BootstrapEntry],
        platform: &str,
    ) -> Result<&'a BootstrapEntry, TebakoError> {
        if let Some(entry) = index.iter().find(|e| e.platform == platform) {
            return Ok(entry);
        }
        let combos: Vec<String> = index.iter().map(|e| e.platform.clone()).collect();
        Err(packaging_error(
            137,
            Some(&format!(
                "no tebako-bootstrap asset for {platform} (tebako {}). Available: {}. Set --bootstrap or $TEBAKO_BOOTSTRAP to a local Rust tebako-bootstrap instead.",
                self.version,
                combos.join(", ")
            )),
        ))
    }

    fn verify(&self, tmp: &Path, entry: &BootstrapEntry) -> Result<(), TebakoError> {
        let actual = sha256_file_hex(tmp)
            .ok_or_else(|| packaging_error(139, Some(&format!("cannot hash {}", tmp.display()))))?;
        let expected = entry.sha256.to_ascii_lowercase();
        if actual == expected {
            return Ok(());
        }
        let _ = fs::remove_file(tmp);
        Err(packaging_error(
            139,
            Some(&format!(
                "{}: expected {expected}, got {actual}; download deleted",
                entry.filename
            )),
        ))
    }

    fn place(
        &self,
        tmp: &Path,
        binary: &Path,
        entry: &BootstrapEntry,
        url: &str,
    ) -> Result<(), TebakoError> {
        let err = |e: std::io::Error| {
            crate::error::plain_error(format!("{e} installing {}", binary.display()))
        };
        make_executable(tmp).map_err(err)?;
        fs::rename(tmp, binary).map_err(err)?;
        let dir = binary.parent().unwrap_or_else(|| Path::new("."));
        fs::write(dir.join(SHA256_FILE), format!("{}\n", entry.sha256)).map_err(err)?;
        fs::write(dir.join(ORIGIN_FILE), format!("{url}\n")).map_err(err)?;
        Ok(())
    }

    /// The release index for `platform`, in preference order
    /// (tebako#493; spec 05 §2): the per-asset sidecar
    /// `tebako-bootstrap-<version>-<platform>[.exe].sha256` — the
    /// sidecar-era authority, one small file carrying exactly this
    /// platform's pin — then the derived monoliths (`manifest.json`,
    /// then `SHA256SUMS`). The fallbacks stay forever: pre-sidecar
    /// releases are immutable and remain installable (invariant 7).
    fn fetch_index(&self, platform: &str) -> Result<Vec<BootstrapEntry>, TebakoError> {
        let mut tried: Vec<String> = Vec::new();
        if let Some(entry) = self.fetch_sidecar(platform, &mut tried)? {
            return Ok(vec![entry]);
        }
        for name in BOOTSTRAP_INDEX_FILES {
            let url = self.index_url(name);
            match fetch_text(&url) {
                Ok(body) => match self.parse_index(name, &body) {
                    Ok(entries) => return Ok(entries),
                    Err(FetchError::IndexUnavailable(_)) => tried.push(url),
                    Err(e @ FetchError::Throttled { .. }) => {
                        return Err(packaging_error(140, Some(&e.to_string())));
                    }
                    Err(FetchError::DownloadFailed(msg)) => {
                        return Err(packaging_error(140, Some(&msg)));
                    }
                    Err(e) => {
                        return Err(packaging_error(140, Some(&e.to_string())));
                    }
                },
                Err(FetchError::IndexUnavailable(_)) => tried.push(url),
                Err(e @ FetchError::Throttled { .. }) => {
                    return Err(packaging_error(140, Some(&e.to_string())));
                }
                Err(FetchError::DownloadFailed(msg)) => {
                    return Err(packaging_error(140, Some(&msg)))
                }
                Err(e) => return Err(packaging_error(140, Some(&e.to_string()))),
            }
        }
        Err(packaging_error(
            141,
            Some(&format!(
                "the tebako release v{} provides no usable bootstrap index (tried: {})",
                self.version,
                tried.join(", ")
            )),
        ))
    }

    fn parse_index(&self, name: &str, body: &str) -> Result<Vec<BootstrapEntry>, FetchError> {
        if name == "manifest.json" {
            self.parse_manifest(body)
        } else {
            Ok(self.parse_sha256sums(body))
        }
    }

    /// Preference 1 of the sidecar era (tebako#493): the per-asset
    /// sidecar `tebako-bootstrap-<version>-<platform>[.exe].sha256`
    /// (authored by finalize.sh — the same frag sha that feeds
    /// SHA256SUMS). Windows assets carry `.exe` (the product's real
    /// spelling — that candidate goes first there; the suffix-less
    /// variant is the defensive second). `Ok(None)` when the release
    /// carries no usable sidecar for the platform (a pre-sidecar
    /// release, or one that cannot serve it) — the caller falls through
    /// to the monoliths with the sidecar URL recorded in `tried`.
    fn fetch_sidecar(
        &self,
        platform: &str,
        tried: &mut Vec<String>,
    ) -> Result<Option<BootstrapEntry>, TebakoError> {
        let stem = format!("tebako-bootstrap-{}-{platform}", self.version);
        let mut names = vec![format!("{stem}.sha256")];
        if platform.starts_with("windows") {
            names.insert(0, format!("{stem}.exe.sha256"));
        }
        for name in names {
            let url = self.index_url(&name);
            match fetch_text(&url) {
                Ok(body) => match self.parse_sidecar(&body, platform) {
                    Ok(entry) => return Ok(Some(entry)),
                    Err(FetchError::IndexUnavailable(_)) => tried.push(url),
                    Err(e @ FetchError::Throttled { .. }) => {
                        return Err(packaging_error(140, Some(&e.to_string())));
                    }
                    Err(FetchError::DownloadFailed(msg)) => {
                        return Err(packaging_error(140, Some(&msg)));
                    }
                    Err(e) => {
                        return Err(packaging_error(140, Some(&e.to_string())));
                    }
                },
                Err(FetchError::IndexUnavailable(_)) => tried.push(url),
                Err(e @ FetchError::Throttled { .. }) => {
                    return Err(packaging_error(140, Some(&e.to_string())));
                }
                Err(FetchError::DownloadFailed(msg)) => {
                    return Err(packaging_error(140, Some(&msg)));
                }
                Err(e) => return Err(packaging_error(140, Some(&e.to_string()))),
            }
        }
        Ok(None)
    }

    /// A per-asset sidecar is one coreutils line, `"<sha256>
    /// <filename>"` (a `*` prefix rides along). The filename is the
    /// asset's authoritative spelling. The sidecar was fetched under the
    /// name derived from the requested platform — one naming a different
    /// asset or platform cannot serve this request (IndexUnavailable:
    /// fall through to the monoliths with the URL recorded).
    fn parse_sidecar(&self, body: &str, platform: &str) -> Result<BootstrapEntry, FetchError> {
        let unusable = || {
            FetchError::IndexUnavailable(
                "the bootstrap sidecar is not a \"<sha256>  <filename>\" line".to_string(),
            )
        };
        let mut lines = body.lines().filter(|l| !l.trim().is_empty());
        let (Some(line), None) = (lines.next(), lines.next()) else {
            return Err(unusable());
        };
        let mut parts = line.trim().splitn(2, char::is_whitespace);
        let (Some(sha256), Some(file)) = (parts.next(), parts.next()) else {
            return Err(unusable());
        };
        let file = file.trim().trim_start_matches('*');
        let stem = format!("tebako-bootstrap-{}-{platform}", self.version);
        if file != stem && file != format!("{stem}.exe") {
            return Err(FetchError::IndexUnavailable(format!(
                "the bootstrap sidecar names {file} — requested {stem}"
            )));
        };
        Ok(BootstrapEntry {
            platform: platform.to_string(),
            filename: file.to_string(),
            sha256: sha256.to_ascii_lowercase(),
        })
    }

    /// The product release's manifest.json is an object whose top-level
    /// `assets` array is exactly the bootstrap set ({platform, file,
    /// sha256, size_bytes} — authored by finalize.sh; the per-tool
    /// `tools` map never leaks into it).
    fn parse_manifest(&self, body: &str) -> Result<Vec<BootstrapEntry>, FetchError> {
        let data = json_parse(body).map_err(|_| {
            FetchError::IndexUnavailable("manifest.json is not valid JSON".to_string())
        })?;
        let Some(JsonValue::Array(assets)) = data.find("assets").cloned() else {
            return Err(FetchError::IndexUnavailable(
                "manifest.json has no assets array".to_string(),
            ));
        };
        Ok(assets
            .iter()
            .filter(|a| {
                a.find("platform").and_then(|v| v.as_string()).is_some()
                    && a.find("file").and_then(|v| v.as_string()).is_some()
                    && a.find("sha256").and_then(|v| v.as_string()).is_some()
            })
            .map(|a| BootstrapEntry {
                platform: a
                    .find("platform")
                    .and_then(|v| v.as_string())
                    .unwrap_or_default(),
                filename: a
                    .find("file")
                    .and_then(|v| v.as_string())
                    .unwrap_or_default(),
                sha256: a
                    .find("sha256")
                    .and_then(|v| v.as_string())
                    .unwrap_or_default()
                    .to_ascii_lowercase(),
            })
            .collect())
    }

    /// `<sha256>  <filename>` lines; filenames may carry a `*` prefix.
    /// Only the `tebako-bootstrap-<version>-` lines are bootstrap assets
    /// — the product's SHA256SUMS carries every tool's lines (tfs,
    /// tebako-pkg, tebako, tebako-shim, the link-unit tarballs).
    fn parse_sha256sums(&self, body: &str) -> Vec<BootstrapEntry> {
        let mut out: Vec<BootstrapEntry> = Vec::new();
        let prefix = format!("tebako-bootstrap-{}-", self.version);
        for line in body.lines() {
            let mut parts = line.trim().splitn(2, char::is_whitespace);
            let (Some(sha256), Some(file)) = (parts.next(), parts.next()) else {
                continue;
            };
            let file = file.trim().trim_start_matches('*');
            let Some(rest) = file.strip_prefix(&prefix) else {
                continue;
            };
            let platform = rest.strip_suffix(".exe").unwrap_or(rest);
            if platform.is_empty() {
                continue;
            }
            out.push(BootstrapEntry {
                platform: platform.to_string(),
                filename: file.to_string(),
                sha256: sha256.to_ascii_lowercase(),
            });
        }
        out
    }

    fn download(&self, url: &str, filename: &str) -> Result<PathBuf, TebakoError> {
        let tmp_dir = self.cache_root.join(TMP_DIR);
        fs::create_dir_all(&tmp_dir).map_err(|e| {
            crate::error::plain_error(format!("{e} creating {}", tmp_dir.display()))
        })?;
        let tmp = tmp_dir.join(format!("{filename}.{}.part", std::process::id()));
        match fetch_bytes(url) {
            Ok(bytes) => {
                if let Err(e) = crate::fetch::write_tmp(&tmp, &bytes) {
                    let _ = fs::remove_file(&tmp);
                    return Err(packaging_error(140, Some(&format!("{e} writing {url}"))));
                }
                Ok(tmp)
            }
            Err(FetchError::IndexUnavailable(_)) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(140, Some(&format!("{url}: not found"))))
            }
            Err(e @ FetchError::Throttled { .. }) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(140, Some(&e.to_string())))
            }
            Err(FetchError::DownloadFailed(msg)) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(140, Some(&msg)))
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(140, Some(&e.to_string())))
            }
        }
    }

    // ---- urls ------------------------------------------------------------

    fn release_url(&self) -> String {
        format!(
            "{}/v{}",
            self.mirror,
            self.version.strip_prefix('v').unwrap_or(&self.version)
        )
    }

    fn index_url(&self, name: &str) -> String {
        format!("{}/{name}", self.release_url())
    }

    fn index_urls(&self) -> Vec<String> {
        BOOTSTRAP_INDEX_FILES
            .iter()
            .map(|n| self.index_url(n))
            .collect()
    }

    fn asset_url(&self, filename: &str) -> String {
        format!("{}/{filename}", self.release_url())
    }
}

/// TEBAKO_OFFLINE (1/true/yes, case-insensitive): every resolution is
/// cache-or-named-error, never a silent fetch.
fn offline_env() -> bool {
    std::env::var("TEBAKO_OFFLINE")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

/// The store's per-entry install lock (the `.install.lock` file inside
/// the entry directory): exclusive flock with a bounded wait, then `f`.
/// A contended lock past `timeout` is the named error `code` naming the
/// artifact kind (`noun`) — 125 for runtimes, 142 for bootstraps.
fn with_install_lock<F>(
    dir: &Path,
    entry_ref: &str,
    timeout: std::time::Duration,
    noun: &str,
    code: i32,
    f: F,
) -> Result<(), TebakoError>
where
    F: FnOnce() -> Result<(), TebakoError>,
{
    fs::create_dir_all(dir)
        .map_err(|e| crate::error::plain_error(format!("{e} creating {}", dir.display())))?;
    let lock_path = dir.join(LOCK_FILE);
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| crate::error::plain_error(format!("{e} opening {}", lock_path.display())))?;
    acquire_install_lock(&lock, entry_ref, &lock_path, timeout, noun, code)?;
    let result = f();
    flock(&lock, LOCK_UN);
    result
}

fn acquire_install_lock(
    lock: &fs::File,
    entry_ref: &str,
    lock_path: &Path,
    timeout: std::time::Duration,
    noun: &str,
    code: i32,
) -> Result<(), TebakoError> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if flock(lock, LOCK_EX | LOCK_NB) {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(packaging_error(
                code,
                Some(&format!(
                    "{entry_ref}: another process is installing this {noun} (no lock after {}s; lockfile: {})",
                    timeout.as_secs(),
                    lock_path.display()
                )),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// 0755 (installed executable): unix mode bits; a Windows executable
/// needs no bit (PATHEXT makes the .exe runnable), so this is a no-op
/// there. Mirrors tebako-shim's runtime install helpers.
fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// 0444 (read-only artifact): unix mode bits; the Windows form is the
/// readonly attribute (mode bits do not exist there).
fn make_readonly(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o444);
        fs::set_permissions(path, perms)
    }
    #[cfg(not(unix))]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(path, perms)
    }
}

/// Lock ops for [`flock`]. The unix values are libc's flock(2) ops; the
/// Windows port re-declares the three the install locks use (EX/NB map
/// onto LockFileEx flags, UN onto UnlockFileEx) — call sites stay
/// platform-free.
#[cfg(unix)]
pub(crate) use libc::{LOCK_EX, LOCK_NB, LOCK_UN};
#[cfg(windows)]
pub(crate) const LOCK_EX: i32 = 2;
#[cfg(windows)]
pub(crate) const LOCK_NB: i32 = 4;
#[cfg(windows)]
pub(crate) const LOCK_UN: i32 = 8;

/// spec 18 C2 pre-download gate (S11/S12): the runtime release's
/// manifest.json entry for the resolved asset must declare its contract
/// set (contract_era / contract_version / mount_root) — anything less is
/// pre-era and refused by name, before a byte of the runtime downloads.
/// tebako-resolve owns the reader semantics; the press surfaces the
/// refusal as exit 75 (the loader family's shared contract code).
fn contract_gate(entry_ref: &str, card: Option<&str>, asset: &str) -> Result<(), TebakoError> {
    let Some(text) = card else {
        return Err(TebakoError::new(
            format!(
                "{entry_ref} is pre-era — the release provides no readable manifest.json (a checksum-only index declares no contract set) — rebuild with the current factory (spec 18 C2), or pin a runtime that declares its contract"
            ),
            75,
        ));
    };
    match tebako_resolve::contract::gate(text, asset) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(TebakoError::new(
            format!(
                "{entry_ref} is pre-era — its release manifest declares no contract set (no entry for {asset}) — rebuild with the current factory (spec 18 C2)"
            ),
            75,
        )),
        Err(e) => Err(TebakoError::new(format!("{entry_ref}: {e}"), 75)),
    }
}

/// flock(2) wrapper; returns true when the operation succeeded.
/// pub(crate): the RuntimeSdk lock (src/sdk.rs) reuses it — the crate's
/// only lock call sites stay here.
#[cfg(unix)]
pub(crate) fn flock(file: &fs::File, op: i32) -> bool {
    use std::os::unix::io::AsRawFd;
    unsafe { libc::flock(file.as_raw_fd(), op) == 0 }
}

/// Windows: LockFileEx on one byte at offset 0 of the lock file — the
/// same semantics as the unix flock (exclusive/shared, non-blocking on
/// LOCK_NB; the kernel releases a crashed holder's lock when the handle
/// dies). The shape tebako-shim's runtime.rs and tebako-bootstrap's
/// platform.rs already use; the Win32 FFI is quarantined in this
/// function (the crate's lock boundary).
#[cfg(windows)]
pub(crate) fn flock(file: &fs::File, op: i32) -> bool {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, UnlockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;

    let mut ov = OVERLAPPED::default();
    if op & LOCK_UN != 0 {
        return unsafe { UnlockFileEx(file.as_raw_handle(), 0, 1, 0, &mut ov) != 0 };
    }
    let mut flags = 0;
    if op & LOCK_EX != 0 {
        flags |= LOCKFILE_EXCLUSIVE_LOCK;
    }
    if op & LOCK_NB != 0 {
        flags |= LOCKFILE_FAIL_IMMEDIATELY;
    }
    unsafe { LockFileEx(file.as_raw_handle(), flags, 0, 1, 0, &mut ov) != 0 }
}

/// Split `<x.y.z>-<platform>` (ruby version is exactly three numeric
/// components, the platform is the rest).
fn split_ruby_platform(rest: &str) -> Option<(String, String)> {
    let bytes = rest.as_bytes();
    let mut dots = 0;
    let mut end = 0;
    while end < bytes.len() {
        let c = bytes[end] as char;
        if c.is_ascii_digit() {
            end += 1;
        } else if c == '.' && dots < 2 {
            dots += 1;
            end += 1;
        } else {
            break;
        }
    }
    if dots != 2 || end == 0 || end >= bytes.len() || bytes[end] != b'-' {
        return None;
    }
    let ruby_version = rest[..end].to_string();
    let platform = rest[end + 1..].to_string();
    if platform.is_empty() {
        return None;
    }
    Some((ruby_version, platform))
}

pub fn sha256_file_hex(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&bytes);
    Some(hex_lower(&hasher.finalize()))
}

pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    let Ok(children) = fs::read_dir(path) else {
        return 0;
    };
    for child in children.filter_map(|c| c.ok()) {
        let p = child.path();
        if p.is_dir() {
            total += dir_size(&p);
        } else if let Ok(m) = p.metadata() {
            total += m.len();
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_ruby_platform_parses_runtime_names() {
        assert_eq!(
            split_ruby_platform("3.3.7-macos-arm64"),
            Some(("3.3.7".to_string(), "macos-arm64".to_string()))
        );
        assert_eq!(
            split_ruby_platform("3.1.6-linux-gnu-x86_64"),
            Some(("3.1.6".to_string(), "linux-gnu-x86_64".to_string()))
        );
        assert_eq!(split_ruby_platform("3.3-macos-arm64"), None);
        assert_eq!(split_ruby_platform("3.3.7"), None);
        assert_eq!(split_ruby_platform("3.3.7-"), None);
    }

    #[test]
    fn sha256sums_runtime_entries() {
        let r = Resolver::new();
        let body = "abc123  tebako-runtime-0.15.9-3.3.7-macos-arm64\n\
                    def456  *tebako-runtime-0.15.9-3.1.6-linux-gnu-x86_64\n\
                    000000  tebako-runtime-0.15.8-3.3.7-macos-arm64\n\
                    junk line\n";
        let entries = r.parse_sha256sums(body, "0.15.9");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ruby_version.as_deref(), Some("3.3.7"));
        assert_eq!(entries[0].platform.as_deref(), Some("macos-arm64"));
        assert_eq!(
            entries[0].filename,
            "tebako-runtime-0.15.9-3.3.7-macos-arm64"
        );
        assert_eq!(
            entries[1].filename,
            "tebako-runtime-0.15.9-3.1.6-linux-gnu-x86_64"
        );
        assert_eq!(entries[1].sha256, "def456");
    }

    #[test]
    fn manifest_runtime_array() {
        let r = Resolver::new();
        let body = r#"[
          {"tebako_version":"0.15.9","ruby_version":"3.3.7","platform":"macos-arm64",
           "filename":"tebako-runtime-0.15.9-3.3.7-macos-arm64","sha256":"ABC","size_bytes":12},
          {"tebako_version":"0.15.8","ruby_version":"3.3.7","platform":"macos-arm64",
           "filename":"old","sha256":"zzz"}
        ]"#;
        let entries = r.parse_manifest(body, "0.15.9").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sha256, "abc");
    }

    #[test]
    fn manifest_runtime_rejects_object() {
        let r = Resolver::new();
        let err = r.parse_manifest("{\"assets\":[]}", "0.15.9").unwrap_err();
        assert!(matches!(err, ParseFail::Unavailable(_)));
    }

    #[test]
    fn contract_gate_classes() {
        let asset = "tebako-runtime-0.16.1-3.4.2-macos-arm64";
        let card = |contract_bits: &str| {
            format!(
                "[{{{contract_bits}\"filename\":\"{asset}\",\"sha256\":\"604e87a1b1d74a6868b35ecdbb11c4e3db01b23286cea9f078636fdf246172b8\"}}]"
            )
        };
        let full = "\"contract_era\":2,\"contract_version\":2,\"mount_root\":\"/__tfs__\",";
        // accept: the era-2 factory shape
        contract_gate("ruby@3.4.2", Some(&card(full)), asset).unwrap();
        // pre-era: no contract fields (exit 75, named)
        let err = contract_gate("ruby@3.4.2", Some(&card("")), asset).unwrap_err();
        assert_eq!(err.code, 75);
        assert!(err.message.contains("pre-era"), "{}", err.message);
        // pre-era: no readable card at all (a sums-only index)
        let err = contract_gate("ruby@3.4.2", None, asset).unwrap_err();
        assert_eq!(err.code, 75);
        assert!(err.message.contains("pre-era"), "{}", err.message);
        // newer contract_version: both numbers named
        let newer = "\"contract_era\":2,\"contract_version\":3,\"mount_root\":\"/__tfs__\",";
        let err = contract_gate("ruby@3.4.2", Some(&card(newer)), asset).unwrap_err();
        assert_eq!(err.code, 75);
        assert!(
            err.message.contains("contract_version 3"),
            "{}",
            err.message
        );
        assert!(err.message.contains("speaks contract 2"), "{}", err.message);
        // the entry itself missing: pre-era, not a silent pass
        let err = contract_gate("ruby@3.4.2", Some(&card(full)), "nope").unwrap_err();
        assert_eq!(err.code, 75);
        assert!(err.message.contains("pre-era"), "{}", err.message);
    }

    // ---- tebako-runtime-ruby#40: the windows ruby DLL facet -------------

    #[test]
    fn sha256sums_dll_lines_attach_to_the_runtime_entry() {
        let r = Resolver::new();
        let body = "aaa111  tebako-runtime-0.16.3-3.3.12-windows-ucrt64.exe\n\
                    bbb222  tebako-runtime-0.16.3-3.3.12-windows-ucrt64.tfs\n\
                    ccc333  tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll\n\
                    ddd444  tebako-runtime-0.16.3-3.4.2-macos-arm64\n";
        let entries = r.parse_sha256sums(body, "0.16.3");
        assert_eq!(entries.len(), 2);
        let win = &entries[0];
        assert_eq!(win.platform.as_deref(), Some("windows-ucrt64"));
        assert_eq!(
            win.image.as_ref().map(|i| i.filename.as_str()),
            Some("tebako-runtime-0.16.3-3.3.12-windows-ucrt64.tfs")
        );
        let dll = win.dll.as_ref().expect("the dll line attaches");
        assert_eq!(
            dll.filename,
            "tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll"
        );
        assert_eq!(dll.sha256, "ccc333");
        // the sums form carries no PE name — not installable from here
        assert_eq!(dll.install_as, None);
        // a POSIX entry carries no facet
        assert_eq!(entries[1].image, None);
        assert_eq!(entries[1].dll, None);
    }

    #[test]
    fn manifest_runtime_dll_facet() {
        let r = Resolver::new();
        let body = r#"[
          {"tebako_version":"0.16.3","ruby_version":"3.3.12","platform":"windows-ucrt64",
           "filename":"tebako-runtime-0.16.3-3.3.12-windows-ucrt64.exe","sha256":"AAA",
           "image":{"filename":"tebako-runtime-0.16.3-3.3.12-windows-ucrt64.tfs","sha256":"BBB","size_bytes":5},
           "dll":{"filename":"tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll",
                  "install_as":"x64-ucrt-ruby330.dll","sha256":"CCC","size_bytes":7}},
          {"tebako_version":"0.16.3","ruby_version":"3.3.12","platform":"macos-arm64",
           "filename":"tebako-runtime-0.16.3-3.3.12-macos-arm64","sha256":"DDD",
           "image":{"filename":"tebako-runtime-0.16.3-3.3.12-macos-arm64.tfs","sha256":"EEE","size_bytes":5}}
        ]"#;
        let entries = r.parse_manifest(body, "0.16.3").unwrap();
        assert_eq!(entries.len(), 2);
        let dll = entries[0].dll.as_ref().expect("the dll key parses");
        assert_eq!(
            dll.filename,
            "tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll"
        );
        assert_eq!(dll.install_as.as_deref(), Some("x64-ucrt-ruby330.dll"));
        assert_eq!(dll.sha256, "ccc");
        // the POSIX entry carries no dll facet (the additive-key compat rule)
        assert_eq!(entries[1].dll, None);
    }

    /// A scratch (cache root, release mirror) pair in the factory's
    /// era-2 shape: exe + image (+ optional dll, `tamper_dll` poisons
    /// its declared sha) and a manifest.json declaring the contract set.
    fn dll_mirror(tag: &str, with_dll: bool, tamper_dll: bool) -> (PathBuf, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("tebako-resolve-dll-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = dir.join("home");
        let release = dir.join("mirror").join("v0.16.3");
        fs::create_dir_all(&release).unwrap();
        let exe = "tebako-runtime-0.16.3-3.3.12-windows-ucrt64.exe";
        let image = "tebako-runtime-0.16.3-3.3.12-windows-ucrt64.tfs";
        let dll = "tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll";
        fs::write(release.join(exe), b"fake runtime exe\n").unwrap();
        fs::write(release.join(image), b"fake env image\n").unwrap();
        let mut manifest = format!(
            "[{{\"tebako_version\":\"0.16.3\",\"contract_era\":2,\"contract_version\":2,\"mount_root\":\"/__tfs__\",\"ruby_version\":\"3.3.12\",\"platform\":\"windows-ucrt64\",\"filename\":\"{exe}\",\"sha256\":\"{}\",\"image\":{{\"filename\":\"{image}\",\"sha256\":\"{}\"}}",
            sha256_file_hex(&release.join(exe)).unwrap(),
            sha256_file_hex(&release.join(image)).unwrap(),
        );
        if with_dll {
            fs::write(release.join(dll), b"fake ruby dll\n").unwrap();
            let declared = if tamper_dll {
                "f".repeat(64)
            } else {
                sha256_file_hex(&release.join(dll)).unwrap()
            };
            manifest.push_str(&format!(
                ",\"dll\":{{\"filename\":\"{dll}\",\"install_as\":\"x64-ucrt-ruby330.dll\",\"sha256\":\"{declared}\",\"size_bytes\":14}}"
            ));
        }
        manifest.push_str("}]\n");
        fs::write(release.join("manifest.json"), manifest).unwrap();
        (cache, dir.join("mirror"))
    }

    fn dll_resolver(cache: &Path, mirror: &Path) -> Resolver {
        Resolver {
            cache_root: cache.to_path_buf(),
            mirror: format!("file://{}", mirror.display()),
            lock_timeout: LOCK_TIMEOUT,
        }
    }

    fn dll_entry_dir(cache: &Path) -> PathBuf {
        cache
            .join("runtimes")
            .join("ruby-3.3.12-0.16.3-windows-ucrt64")
    }

    #[test]
    fn resolve_runtime_installs_the_dll_as_install_as_with_markers() {
        let (cache, mirror) = dll_mirror("install", true, false);
        let r = dll_resolver(&cache, &mirror);
        let resolved = r
            .resolve_runtime("3.3.12", "windows-ucrt64", "0.16.3")
            .unwrap();
        assert!(resolved.executable.is_file());
        let dir = dll_entry_dir(&cache);
        // the env image landed too
        assert!(dir
            .join("tebako-runtime-0.16.3-3.3.12-windows-ucrt64.tfs")
            .is_file());
        // the dll materializes under its PE name — never the asset name
        let dll = dir.join("x64-ucrt-ruby330.dll");
        assert!(dll.is_file(), "{}", dll.display());
        assert!(
            !dir.join("tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll")
                .exists(),
            "the asset name is not the install name"
        );
        let marker = fs::read_to_string(dir.join("x64-ucrt-ruby330.dll.sha256")).unwrap();
        assert_eq!(
            marker,
            format!("{}  x64-ucrt-ruby330.dll\n", sha256_file_hex(&dll).unwrap())
        );
        let origin = fs::read_to_string(dir.join("x64-ucrt-ruby330.dll.origin")).unwrap();
        assert_eq!(
            origin,
            format!(
                "file://{}/v0.16.3/tebako-runtime-0.16.3-3.3.12-windows-ucrt64.dll\n",
                mirror.display()
            )
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(dll.metadata().unwrap().permissions().mode() & 0o777, 0o444);
        }
        // a cache hit needs no mirror at all (a run is a run, offline-safe)
        fs::remove_dir_all(&mirror).unwrap();
        r.resolve_runtime("3.3.12", "windows-ucrt64", "0.16.3")
            .unwrap();
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_runtime_flows_the_index_spelling() {
        // tebako#456 (spec 05 §2 SSOT): the exe installs under the index
        // entry's `filename` VERBATIM — the factory publishes the windows
        // exe SUFFIX-LESS — the index rides into the cache entry, and a
        // cache hit (mirror gone) finds the runtime by the flowed
        // spelling again.
        let dir = std::env::temp_dir().join(format!("tebako-resolve-flow-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = dir.join("home");
        let mirror = dir.join("mirror");
        let release = mirror.join("v0.16.9");
        fs::create_dir_all(&release).unwrap();
        let exe = "tebako-runtime-0.16.9-3.3.12-windows-ucrt64";
        let image = "tebako-runtime-0.16.9-3.3.12-windows-ucrt64.tfs";
        fs::write(release.join(exe), b"fake runtime exe\n").unwrap();
        fs::write(release.join(image), b"fake env image\n").unwrap();
        fs::write(
            release.join("manifest.json"),
            format!(
                "[{{\"tebako_version\":\"0.16.9\",\"contract_era\":2,\"contract_version\":2,\"mount_root\":\"A:/t\",\"ruby_version\":\"3.3.12\",\"platform\":\"windows-ucrt64\",\"filename\":\"{exe}\",\"sha256\":\"{}\",\"image\":{{\"filename\":\"{image}\",\"sha256\":\"{}\"}}}}]\n",
                sha256_file_hex(&release.join(exe)).unwrap(),
                sha256_file_hex(&release.join(image)).unwrap(),
            ),
        )
        .unwrap();
        let r = Resolver {
            cache_root: cache.clone(),
            mirror: format!("file://{}", mirror.display()),
            lock_timeout: LOCK_TIMEOUT,
        };

        let resolved = r
            .resolve_runtime("3.3.12", "windows-ucrt64", "0.16.9")
            .unwrap();
        assert_eq!(
            resolved.executable.file_name().unwrap().to_string_lossy(),
            exe,
            "the cache holds the exe under the index spelling, verbatim"
        );
        let entry_dir = cache
            .join("runtimes")
            .join("ruby-3.3.12-0.16.9-windows-ucrt64");
        assert!(entry_dir.join(exe).is_file());
        assert!(entry_dir.join(image).is_file());
        assert!(
            entry_dir.join("manifest.json").is_file(),
            "the index rides into the cache entry"
        );

        // a cache hit flows the cached index: the mirror is gone, the
        // exe is found under the flowed spelling (no re-download).
        fs::remove_dir_all(&mirror).unwrap();
        let hit = r
            .resolve_runtime("3.3.12", "windows-ucrt64", "0.16.9")
            .unwrap();
        assert_eq!(hit.executable, resolved.executable);
        assert!(hit.image.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_runtime_without_the_dll_key_installs_the_exe_alone() {
        let (cache, mirror) = dll_mirror("nodll", false, false);
        let r = dll_resolver(&cache, &mirror);
        r.resolve_runtime("3.3.12", "windows-ucrt64", "0.16.3")
            .unwrap();
        let dir = dll_entry_dir(&cache);
        assert!(dir
            .join("tebako-runtime-0.16.3-3.3.12-windows-ucrt64.exe")
            .is_file());
        assert!(!dir.join("x64-ucrt-ruby330.dll").exists());
        assert!(
            !fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().ends_with(".dll")),
            "no dll facet declared, no dll installed"
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_runtime_wrong_dll_sha_is_a_named_error() {
        let (cache, mirror) = dll_mirror("badsha", true, true);
        let r = dll_resolver(&cache, &mirror);
        let err = r
            .resolve_runtime("3.3.12", "windows-ucrt64", "0.16.3")
            .unwrap_err();
        assert_eq!(err.code, 121);
        assert!(err.message.contains("download deleted"), "{}", err.message);
        let dir = dll_entry_dir(&cache);
        assert!(!dir.join("x64-ucrt-ruby330.dll").exists());
        assert!(
            !fs::read_dir(cache.join(TMP_DIR))
                .unwrap()
                .filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().contains(".dll")),
            "the failed download was deleted"
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_runtime_dll_traversal_install_as_is_a_named_error() {
        let (cache, mirror) = dll_mirror("traversal", true, false);
        // poison the facet's PE name: a separator would escape the entry
        let manifest_path = mirror.join("v0.16.3").join("manifest.json");
        let text = fs::read_to_string(&manifest_path)
            .unwrap()
            .replace("x64-ucrt-ruby330.dll", "../evil.dll");
        fs::write(&manifest_path, text).unwrap();
        let r = dll_resolver(&cache, &mirror);
        let err = r
            .resolve_runtime("3.3.12", "windows-ucrt64", "0.16.3")
            .unwrap_err();
        assert_eq!(err.code, 122);
        assert!(err.message.contains("bare file name"), "{}", err.message);
        assert!(!cache.join("runtimes").join("evil.dll").exists());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    // ---- tebako#493: the sidecar-era index (shard-first) ---------------

    /// A scratch (cache root, release mirror) pair in the sidecar era's
    /// shape: the suffix-less windows exe + image + dll payload assets
    /// and the per-package shard `<stem>.manifest.json` — no monoliths.
    /// `tamper_manifest` additionally writes a derived manifest.json
    /// whose declared exe sha is POISONED (the shard must win).
    fn shard_mirror(tag: &str, tamper_manifest: bool) -> (PathBuf, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("tebako-resolve-shard-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = dir.join("home");
        let release = dir.join("mirror").join("v0.16.17");
        fs::create_dir_all(&release).unwrap();
        let exe = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64";
        let image = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64.tfs";
        let dll = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64.dll";
        fs::write(release.join(exe), b"fake runtime exe\n").unwrap();
        fs::write(release.join(image), b"fake env image\n").unwrap();
        fs::write(release.join(dll), b"fake ruby dll\n").unwrap();
        let shard = format!(
            "{{\"tebako_version\":\"0.16.17\",\"contract_era\":2,\"contract_version\":2,\"mount_root\":\"A:/t\",\"ruby_version\":\"3.3.12\",\"platform\":\"windows-ucrt64\",\"filename\":\"{exe}\",\"sha256\":\"{}\",\"image\":{{\"filename\":\"{image}\",\"sha256\":\"{}\"}},\"dll\":{{\"filename\":\"{dll}\",\"install_as\":\"x64-ucrt-ruby330.dll\",\"sha256\":\"{}\"}}}}\n",
            sha256_file_hex(&release.join(exe)).unwrap(),
            sha256_file_hex(&release.join(image)).unwrap(),
            sha256_file_hex(&release.join(dll)).unwrap(),
        );
        fs::write(release.join(format!("{exe}.manifest.json")), shard).unwrap();
        if tamper_manifest {
            let manifest = format!(
                "[{{\"tebako_version\":\"0.16.17\",\"contract_era\":2,\"contract_version\":2,\"mount_root\":\"A:/t\",\"ruby_version\":\"3.3.12\",\"platform\":\"windows-ucrt64\",\"filename\":\"{exe}\",\"sha256\":\"{}\"}}]\n",
                "f".repeat(64)
            );
            fs::write(release.join("manifest.json"), manifest).unwrap();
        }
        (cache, dir.join("mirror"))
    }

    fn shard_entry_dir(cache: &Path) -> PathBuf {
        cache
            .join("runtimes")
            .join("ruby-3.3.12-0.16.17-windows-ucrt64")
    }

    #[test]
    fn shard_parses_the_single_entry_object() {
        let r = Resolver::new();
        let body = r#"{"tebako_version":"0.16.17","contract_era":2,"contract_version":2,"mount_root":"A:/t","ruby_version":"3.3.12","platform":"windows-ucrt64","filename":"tebako-runtime-0.16.17-3.3.12-windows-ucrt64","sha256":"ABC","image":{"filename":"i.tfs","sha256":"DEF"},"dll":{"filename":"d.dll","install_as":"x.dll","sha256":"012"}}"#;
        let e = r
            .parse_shard(body, "3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap();
        assert_eq!(e.filename, "tebako-runtime-0.16.17-3.3.12-windows-ucrt64");
        assert_eq!(e.sha256, "abc");
        assert_eq!(e.image.as_ref().map(|i| i.sha256.as_str()), Some("def"));
        assert_eq!(
            e.dll.as_ref().and_then(|d| d.install_as.as_deref()),
            Some("x.dll")
        );
        // an array is the monolith's shape — rejected as a shard
        let err = r
            .parse_shard("[{}]", "3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap_err();
        assert!(matches!(err, ParseFail::Unavailable(_)));
        // a triple mismatch cannot serve the request
        let err = r
            .parse_shard(body, "3.4.10", "windows-ucrt64", "0.16.17")
            .unwrap_err();
        assert!(matches!(err, ParseFail::Unavailable(_)));
        // unparseable
        let err = r
            .parse_shard("{", "3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap_err();
        assert!(matches!(err, ParseFail::Unavailable(_)));
    }

    #[test]
    fn resolve_runtime_installs_from_the_shard_without_a_monolith() {
        let (cache, mirror) = shard_mirror("only", false);
        let r = dll_resolver(&cache, &mirror);
        let resolved = r
            .resolve_runtime("3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap();
        let dir = shard_entry_dir(&cache);
        // the suffix-less exe spelling flows from the shard verbatim
        assert_eq!(
            resolved.executable.file_name().unwrap().to_string_lossy(),
            "tebako-runtime-0.16.17-3.3.12-windows-ucrt64"
        );
        assert!(dir
            .join("tebako-runtime-0.16.17-3.3.12-windows-ucrt64.tfs")
            .is_file());
        assert!(dir.join("x64-ucrt-ruby330.dll").is_file());
        // the wrapped shard rode into the cache entry as the card — the
        // manifest array shape, readable by parse_manifest
        let card = fs::read_to_string(dir.join("manifest.json")).unwrap();
        assert!(
            card.starts_with('['),
            "the cached card is the wrapped shard (the array shape)"
        );
        let entry = r
            .cached_index_entry(&dir, "3.3.12", "windows-ucrt64", "0.16.17")
            .expect("the cached card re-reads");
        assert_eq!(
            entry.filename,
            resolved.executable.file_name().unwrap().to_string_lossy()
        );
        // a cache hit needs no mirror at all (a run is a run)
        fs::remove_dir_all(&mirror).unwrap();
        r.resolve_runtime("3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap();
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_runtime_the_shard_wins_over_a_tampered_monolith() {
        // the monolith's poisoned exe sha would fail the install with
        // 121 — the shard is read FIRST, so the honest sha gates
        let (cache, mirror) = shard_mirror("precedence", true);
        let r = dll_resolver(&cache, &mirror);
        r.resolve_runtime("3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap();
        assert!(shard_entry_dir(&cache)
            .join("tebako-runtime-0.16.17-3.3.12-windows-ucrt64")
            .is_file());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_runtime_a_mismatched_shard_falls_through_to_the_monolith() {
        let (cache, mirror) = shard_mirror("mismatch", false);
        let release = mirror.join("v0.16.17");
        let exe = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64";
        let shard_path = release.join(format!("{exe}.manifest.json"));
        let text = fs::read_to_string(&shard_path)
            .unwrap()
            .replace("\"ruby_version\":\"3.3.12\"", "\"ruby_version\":\"3.4.10\"");
        fs::write(&shard_path, text).unwrap();
        // the honest derived monolith names the requested triple
        let image = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64.tfs";
        let dll = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64.dll";
        fs::write(
            release.join("manifest.json"),
            format!(
                "[{{\"tebako_version\":\"0.16.17\",\"contract_era\":2,\"contract_version\":2,\"mount_root\":\"A:/t\",\"ruby_version\":\"3.3.12\",\"platform\":\"windows-ucrt64\",\"filename\":\"{exe}\",\"sha256\":\"{}\",\"image\":{{\"filename\":\"{image}\",\"sha256\":\"{}\"}},\"dll\":{{\"filename\":\"{dll}\",\"install_as\":\"x64-ucrt-ruby330.dll\",\"sha256\":\"{}\"}}}}]\n",
                sha256_file_hex(&release.join(exe)).unwrap(),
                sha256_file_hex(&release.join(image)).unwrap(),
                sha256_file_hex(&release.join(dll)).unwrap(),
            ),
        )
        .unwrap();
        let r = dll_resolver(&cache, &mirror);
        r.resolve_runtime("3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap();
        assert!(shard_entry_dir(&cache).join(exe).is_file());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_runtime_a_corrupt_shard_falls_through_to_the_monolith() {
        let (cache, mirror) = shard_mirror("corrupt", false);
        let release = mirror.join("v0.16.17");
        let exe = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64";
        fs::write(release.join(format!("{exe}.manifest.json")), "{").unwrap();
        let image = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64.tfs";
        fs::write(
            release.join("manifest.json"),
            format!(
                "[{{\"tebako_version\":\"0.16.17\",\"contract_era\":2,\"contract_version\":2,\"mount_root\":\"A:/t\",\"ruby_version\":\"3.3.12\",\"platform\":\"windows-ucrt64\",\"filename\":\"{exe}\",\"sha256\":\"{}\",\"image\":{{\"filename\":\"{image}\",\"sha256\":\"{}\"}}}}]\n",
                sha256_file_hex(&release.join(exe)).unwrap(),
                sha256_file_hex(&release.join(image)).unwrap(),
            ),
        )
        .unwrap();
        let r = dll_resolver(&cache, &mirror);
        r.resolve_runtime("3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap();
        assert!(shard_entry_dir(&cache).join(exe).is_file());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_runtime_without_any_index_names_every_tried_url() {
        let (cache, mirror) = shard_mirror("noindex", false);
        let release = mirror.join("v0.16.17");
        let exe = "tebako-runtime-0.16.17-3.3.12-windows-ucrt64";
        fs::remove_file(release.join(format!("{exe}.manifest.json"))).unwrap();
        let r = dll_resolver(&cache, &mirror);
        let err = r
            .resolve_runtime("3.3.12", "windows-ucrt64", "0.16.17")
            .unwrap_err();
        assert_eq!(err.code, 124);
        // the shard URL is named first, then the monoliths
        assert!(
            err.message
                .contains("tebako-runtime-0.16.17-3.3.12-windows-ucrt64.manifest.json"),
            "{}",
            err.message
        );
        assert!(err.message.contains("manifest.json"), "{}", err.message);
        assert!(err.message.contains("SHA256SUMS.txt"), "{}", err.message);
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    // ---- spec 19 §4: the Rust bootstrap store ---------------------------

    /// A scratch (cache root, release mirror) pair in the product
    /// release's shape (finalize.sh): the tebako-bootstrap asset plus
    /// both index files — manifest.json's top-level `assets` IS the
    /// bootstrap set (a `tools` entry rides along and must not leak in)
    /// and SHA256SUMS carries every tool's lines. `tamper` poisons the
    /// declared sha.
    fn boot_mirror(tag: &str, tamper: bool) -> (PathBuf, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("tebako-resolve-boot-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = dir.join("home");
        let release = dir.join("mirror").join("v0.1.8");
        fs::create_dir_all(&release).unwrap();
        let asset = "tebako-bootstrap-0.1.8-macos-arm64";
        fs::write(release.join(asset), b"fake rust bootstrap\n").unwrap();
        let declared = if tamper {
            "f".repeat(64)
        } else {
            sha256_file_hex(&release.join(asset)).unwrap()
        };
        let manifest = format!(
            r#"{{"name":"tebako-rs","version":"0.1.8","assets":[{{"platform":"macos-arm64","file":"{asset}","sha256":"{declared}","size_bytes":19}}],"tools":{{"tfs":[{{"platform":"macos-arm64","file":"tfs-0.1.8-macos-arm64","sha256":"{}","size_bytes":5}}]}}}}"#,
            "0".repeat(64)
        );
        fs::write(release.join("manifest.json"), manifest).unwrap();
        let sums = format!(
            "{declared}  {asset}\n{}  tfs-0.1.8-macos-arm64\n{}  tebako-0.1.8-macos-arm64\n{}  tebako-bootstrap-0.1.7-macos-arm64\n{}  link-unit-0.1.8-macos-arm64.tar.gz\n",
            "0".repeat(64),
            "1".repeat(64),
            "2".repeat(64),
            "3".repeat(64)
        );
        fs::write(release.join("SHA256SUMS"), sums).unwrap();
        (cache, dir.join("mirror"))
    }

    fn boot_resolver(cache: &Path, mirror: &Path, offline: bool) -> BootstrapResolver {
        BootstrapResolver {
            cache_root: cache.to_path_buf(),
            mirror: format!("file://{}", mirror.display()),
            version: "0.1.8".to_string(),
            offline,
            lock_timeout: LOCK_TIMEOUT,
        }
    }

    fn boot_entry_dir(cache: &Path) -> PathBuf {
        cache.join("bootstraps").join("0.1.8-macos-arm64")
    }

    #[test]
    fn bootstrap_manifest_object_parses_the_assets_array() {
        let (cache, mirror) = boot_mirror("parse-manifest", false);
        let r = boot_resolver(&cache, &mirror, false);
        let body = fs::read_to_string(mirror.join("v0.1.8").join("manifest.json")).unwrap();
        let entries = r.parse_manifest(&body).unwrap();
        assert_eq!(entries.len(), 1, "the tools map never leaks in");
        assert_eq!(entries[0].platform, "macos-arm64");
        assert_eq!(entries[0].filename, "tebako-bootstrap-0.1.8-macos-arm64");
        // the array shape is mandatory — an object manifest is unusable
        let err = r.parse_manifest("[{\"platform\":\"x\"}]").unwrap_err();
        assert!(matches!(err, FetchError::IndexUnavailable(_)));
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn bootstrap_sha256sums_keeps_only_the_versioned_bootstrap_lines() {
        let (cache, mirror) = boot_mirror("parse-sums", false);
        let r = boot_resolver(&cache, &mirror, false);
        let body = fs::read_to_string(mirror.join("v0.1.8").join("SHA256SUMS")).unwrap();
        let entries = r.parse_sha256sums(&body);
        // tfs-/tebako-/link-unit- lines and the OTHER version's bootstrap
        // line are all filtered out
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].platform, "macos-arm64");
        assert_eq!(entries[0].filename, "tebako-bootstrap-0.1.8-macos-arm64");
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_installs_from_the_manifest_with_markers() {
        let (cache, mirror) = boot_mirror("install", false);
        let r = boot_resolver(&cache, &mirror, false);
        let path = r.resolve("macos-arm64").unwrap();
        let dir = boot_entry_dir(&cache);
        assert_eq!(path, dir.join("tebako-bootstrap-0.1.8-macos-arm64"));
        assert!(path.is_file());
        assert_eq!(
            fs::read_to_string(dir.join("sha256")).unwrap(),
            format!("{}\n", sha256_file_hex(&path).unwrap())
        );
        assert_eq!(
            fs::read_to_string(dir.join("origin")).unwrap(),
            format!(
                "file://{}/v0.1.8/tebako-bootstrap-0.1.8-macos-arm64\n",
                mirror.display()
            )
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o755);
        }
        // a cache hit needs no mirror at all (a run is a run, offline-safe)
        fs::remove_dir_all(&mirror).unwrap();
        let offline_hit = boot_resolver(&cache, &mirror, true);
        assert_eq!(offline_hit.resolve("macos-arm64").unwrap(), path);
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_falls_back_to_sha256sums() {
        let (cache, mirror) = boot_mirror("sums", false);
        fs::remove_file(mirror.join("v0.1.8").join("manifest.json")).unwrap();
        let r = boot_resolver(&cache, &mirror, false);
        let path = r.resolve("macos-arm64").unwrap();
        assert!(path.is_file());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_wrong_sha_is_a_named_error() {
        let (cache, mirror) = boot_mirror("badsha", true);
        let r = boot_resolver(&cache, &mirror, false);
        let err = r.resolve("macos-arm64").unwrap_err();
        assert_eq!(err.code, 139);
        assert!(err.message.contains("download deleted"), "{}", err.message);
        assert!(!boot_entry_dir(&cache)
            .join("tebako-bootstrap-0.1.8-macos-arm64")
            .exists());
        assert!(
            !fs::read_dir(cache.join(TMP_DIR))
                .unwrap()
                .filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().contains("tebako-bootstrap")),
            "the failed download was deleted"
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_offline_miss_is_a_named_error() {
        let (cache, mirror) = boot_mirror("offline", false);
        let r = boot_resolver(&cache, &mirror, true);
        let err = r.resolve("macos-arm64").unwrap_err();
        assert_eq!(err.code, 138);
        assert!(err.message.contains("not cached"), "{}", err.message);
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_unknown_platform_is_a_named_error() {
        let (cache, mirror) = boot_mirror("noplatform", false);
        let r = boot_resolver(&cache, &mirror, false);
        let err = r.resolve("plan9-s390x").unwrap_err();
        assert_eq!(err.code, 137);
        assert!(
            err.message.contains("macos-arm64"),
            "the available list is named: {}",
            err.message
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_without_an_index_is_a_named_error() {
        let (cache, mirror) = boot_mirror("noindex", false);
        let release = mirror.join("v0.1.8");
        fs::remove_file(release.join("manifest.json")).unwrap();
        fs::remove_file(release.join("SHA256SUMS")).unwrap();
        let r = boot_resolver(&cache, &mirror, false);
        let err = r.resolve("macos-arm64").unwrap_err();
        assert_eq!(err.code, 141);
        assert!(
            err.message.contains("no usable bootstrap index"),
            "{}",
            err.message
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    // ---- tebako#493: the sidecar-era index (sidecar-first) --------------

    #[test]
    fn sidecar_parses_one_coreutils_line() {
        let r = BootstrapResolver {
            cache_root: PathBuf::new(),
            mirror: String::new(),
            version: "0.1.8".to_string(),
            offline: false,
            lock_timeout: LOCK_TIMEOUT,
        };
        let e = r
            .parse_sidecar(
                "ABC123  tebako-bootstrap-0.1.8-macos-arm64\n",
                "macos-arm64",
            )
            .unwrap();
        assert_eq!(e.filename, "tebako-bootstrap-0.1.8-macos-arm64");
        assert_eq!(e.sha256, "abc123");
        assert_eq!(e.platform, "macos-arm64");
        // a `*` prefix rides along (coreutils binary marker)
        let e = r
            .parse_sidecar(
                "abc123  *tebako-bootstrap-0.1.8-macos-arm64\n",
                "macos-arm64",
            )
            .unwrap();
        assert_eq!(e.filename, "tebako-bootstrap-0.1.8-macos-arm64");
        // the .exe spelling validates against the windows platform
        let e = r
            .parse_sidecar(
                "abc  tebako-bootstrap-0.1.8-windows-ucrt64.exe\n",
                "windows-ucrt64",
            )
            .unwrap();
        assert_eq!(e.filename, "tebako-bootstrap-0.1.8-windows-ucrt64.exe");
        // a different asset or platform cannot serve the request
        for body in [
            "abc  tebako-bootstrap-0.1.8-linux-gnu-x86_64\n",
            "abc  tfs-0.1.8-macos-arm64\n",
            "abc  tebako-bootstrap-0.1.7-macos-arm64\n",
            "garbage\n",
            "abc  tebako-bootstrap-0.1.8-macos-arm64\ndef  tebako-bootstrap-0.1.8-macos-arm64\n",
            "\n",
        ] {
            assert!(
                matches!(
                    r.parse_sidecar(body, "macos-arm64"),
                    Err(FetchError::IndexUnavailable(_))
                ),
                "{body:?}"
            );
        }
    }

    /// Write the per-asset sidecar into a boot_mirror release (the
    /// finalize.sh shape: `"<sha>  <asset>\n"`).
    fn write_sidecar(release: &Path, asset: &str) {
        let sha = sha256_file_hex(&release.join(asset)).unwrap();
        fs::write(
            release.join(format!("{asset}.sha256")),
            format!("{sha}  {asset}\n"),
        )
        .unwrap();
    }

    #[test]
    fn resolve_bootstrap_installs_from_the_sidecar_without_a_monolith() {
        let (cache, mirror) = boot_mirror("sidecar-only", false);
        let release = mirror.join("v0.1.8");
        fs::remove_file(release.join("manifest.json")).unwrap();
        fs::remove_file(release.join("SHA256SUMS")).unwrap();
        write_sidecar(&release, "tebako-bootstrap-0.1.8-macos-arm64");
        let r = boot_resolver(&cache, &mirror, false);
        let path = r.resolve("macos-arm64").unwrap();
        assert!(path.is_file());
        assert_eq!(
            fs::read_to_string(boot_entry_dir(&cache).join("sha256")).unwrap(),
            format!("{}\n", sha256_file_hex(&path).unwrap())
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_the_sidecar_wins_over_a_tampered_monolith() {
        // boot_mirror(tamper=true) poisons the monoliths' declared sha —
        // the sidecar is read FIRST, so the honest sha gates
        let (cache, mirror) = boot_mirror("sidecar-precedence", true);
        write_sidecar(&mirror.join("v0.1.8"), "tebako-bootstrap-0.1.8-macos-arm64");
        let r = boot_resolver(&cache, &mirror, false);
        let path = r.resolve("macos-arm64").unwrap();
        assert!(path.is_file());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn resolve_bootstrap_a_mismatched_sidecar_falls_through_to_the_monolith() {
        let (cache, mirror) = boot_mirror("sidecar-mismatch", false);
        let release = mirror.join("v0.1.8");
        // the sidecar names a DIFFERENT platform's asset
        fs::write(
            release.join("tebako-bootstrap-0.1.8-macos-arm64.sha256"),
            format!(
                "{}  tebako-bootstrap-0.1.8-linux-gnu-x86_64\n",
                "0".repeat(64)
            ),
        )
        .unwrap();
        let r = boot_resolver(&cache, &mirror, false);
        let path = r.resolve("macos-arm64").unwrap();
        assert!(path.is_file());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    // ---- roadmap 80 G1: the signed runtime fetch (spec 09 §4/§5) -------

    const G1_TAG: &str = "0.16.30";
    const G1_RUBY: &str = "3.3.12";
    const G1_PLATFORM: &str = "macos-arm64";

    fn g1_exe() -> String {
        format!("tebako-runtime-{G1_TAG}-{G1_RUBY}-{G1_PLATFORM}")
    }

    fn g1_image() -> String {
        format!("{}.tfs", g1_exe())
    }

    fn g1_dll() -> String {
        format!("{}.dll", g1_exe())
    }

    fn g1_entry_dir(cache: &Path) -> PathBuf {
        cache
            .join("runtimes")
            .join(format!("ruby-{G1_RUBY}-{G1_TAG}-{G1_PLATFORM}"))
    }

    fn g1_journal(cache: &Path) -> String {
        fs::read_to_string(cache.join("journal.log")).unwrap_or_default()
    }

    /// A scratch (cache root, release mirror, press key) triple: the
    /// exe/image/dll assets at v0.16.30 and a fresh press-local key that
    /// is NOT yet in the cache's trusted keyring — each test composes
    /// its own trust shape.
    fn g1_mirror(tag: &str) -> (PathBuf, PathBuf, tebako_signer::PressKey) {
        let dir =
            std::env::temp_dir().join(format!("tebako-resolve-g1-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = dir.join("home");
        let release = dir.join("mirror").join(format!("v{G1_TAG}"));
        fs::create_dir_all(&release).unwrap();
        let key = tebako_signer::press_local_key(&dir.join("press")).unwrap();
        fs::write(release.join(g1_exe()), b"fake runtime exe\n").unwrap();
        fs::write(release.join(g1_image()), b"fake env image\n").unwrap();
        fs::write(release.join(g1_dll()), b"fake ruby dll\n").unwrap();
        (cache, dir.join("mirror"), key)
    }

    /// Write one detached `.asc` sibling for a release asset, signed by
    /// `key` over the asset's CURRENT bytes.
    fn g1_sign_file(release: &Path, key: &tebako_signer::PressKey, name: &str) {
        let bytes = fs::read(release.join(name)).unwrap();
        let asc = tebako_signer::sign_detached(&bytes, &key.secret_key, &key.fingerprint).unwrap();
        fs::write(release.join(format!("{name}.asc")), asc).unwrap();
    }

    /// Write the release's era-2 manifest.json (the contract set +
    /// exe/image/dll facets). `signed` attaches the declared `signature:`
    /// blocks (spec 13 §2a — entry + image + dll) and writes every
    /// `<asset>.asc` sibling, all by `key`.
    fn g1_write_manifest(release: &Path, key: &tebako_signer::PressKey, signed: bool) {
        let keyid = hex_lower(&key.keyid);
        let block = |name: &str| -> String {
            if !signed {
                return String::new();
            }
            g1_sign_file(release, key, name);
            format!(r#","signature":{{"keyid":"{keyid}","asc":"{name}.asc"}}"#)
        };
        let manifest = format!(
            r#"[{{"tebako_version":"{G1_TAG}","contract_era":2,"contract_version":2,"mount_root":"/__tfs__","ruby_version":"{G1_RUBY}","platform":"{G1_PLATFORM}","filename":"{}","sha256":"{}"{},"image":{{"filename":"{}","sha256":"{}"{}}},"dll":{{"filename":"{}","install_as":"libruby-x.dll","sha256":"{}"{}}}}}]"#,
            g1_exe(),
            sha256_file_hex(&release.join(g1_exe())).unwrap(),
            block(&g1_exe()),
            g1_image(),
            sha256_file_hex(&release.join(g1_image())).unwrap(),
            block(&g1_image()),
            g1_dll(),
            sha256_file_hex(&release.join(g1_dll())).unwrap(),
            block(&g1_dll()),
        );
        fs::write(release.join("manifest.json"), manifest).unwrap();
    }

    #[test]
    fn g1_signed_release_verifies_and_journals_every_signer() {
        let (cache, mirror, key) = g1_mirror("happy");
        let release = mirror.join(format!("v{G1_TAG}"));
        g1_write_manifest(&release, &key, true);
        g1_sign_file(&release, &key, "manifest.json");
        tebako_signer::register_trusted(&cache, &key.public_key).unwrap();
        let r = dll_resolver(&cache, &mirror);
        let resolved = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap();
        assert!(resolved.executable.is_file());
        let dir = g1_entry_dir(&cache);
        assert!(dir.join(g1_image()).is_file());
        assert!(dir.join("libruby-x.dll").is_file());
        let journal = g1_journal(&cache);
        let keyid = hex_lower(&key.keyid);
        assert!(
            journal.contains(&format!(
                "event=runtime-index-verified form=manifest.json signer={keyid}"
            )),
            "{journal}"
        );
        assert!(
            journal.contains(&format!(
                "event=runtime-fetch-verified runtime_ref=ruby@{G1_RUBY} (tebako {G1_TAG}, {G1_PLATFORM})"
            )),
            "{journal}"
        );
        assert!(journal.contains(&format!("signer={keyid}")), "{journal}");
        assert!(!journal.contains("unsigned-runtime-fetch"), "{journal}");
        // a cache hit re-resolves without the mirror (a run is a run)
        fs::remove_dir_all(&mirror).unwrap();
        r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap();
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_untrusted_index_signer_is_a_named_72() {
        let (cache, mirror, key) = g1_mirror("untrusted");
        let release = mirror.join(format!("v{G1_TAG}"));
        g1_write_manifest(&release, &key, true);
        g1_sign_file(&release, &key, "manifest.json");
        // the signing key is NOT registered — nothing trusts it
        let r = dll_resolver(&cache, &mirror);
        let err = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap_err();
        assert_eq!(err.code, 72);
        assert!(
            err.message.contains("not in the trusted keyring"),
            "{}",
            err.message
        );
        assert!(
            err.message.contains(&hex_lower(&key.keyid)),
            "the untrusted signer is named: {}",
            err.message
        );
        assert!(!g1_entry_dir(&cache).join(g1_exe()).exists());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_tampered_exe_fails_the_signature_before_the_sha() {
        let (cache, mirror, key) = g1_mirror("tampered-exe");
        let release = mirror.join(format!("v{G1_TAG}"));
        g1_write_manifest(&release, &key, true);
        g1_sign_file(&release, &key, "manifest.json");
        tebako_signer::register_trusted(&cache, &key.public_key).unwrap();
        // tamper the exe AFTER its .asc was written — the manifest's
        // declared sha now also mismatches, but the signature check runs
        // FIRST (spec 09 §4)
        fs::write(release.join(g1_exe()), b"evil exe\n").unwrap();
        let r = dll_resolver(&cache, &mirror);
        let err = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap_err();
        assert_eq!(err.code, 71);
        assert!(err.message.contains("does not verify"), "{}", err.message);
        assert!(!g1_entry_dir(&cache).join(g1_exe()).exists());
        let journal = g1_journal(&cache);
        assert!(!journal.contains("runtime-fetch-verified"), "{journal}");
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_a_declared_signature_whose_asc_is_missing_is_a_named_71() {
        let (cache, mirror, key) = g1_mirror("missing-asc");
        let release = mirror.join(format!("v{G1_TAG}"));
        g1_write_manifest(&release, &key, true);
        g1_sign_file(&release, &key, "manifest.json");
        tebako_signer::register_trusted(&cache, &key.public_key).unwrap();
        // the exe's declared .asc vanishes from the release
        fs::remove_file(release.join(format!("{}.asc", g1_exe()))).unwrap();
        let r = dll_resolver(&cache, &mirror);
        let err = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap_err();
        assert_eq!(err.code, 71);
        assert!(err.message.contains("did not fetch"), "{}", err.message);
        assert!(
            err.message.contains(&format!("{}.asc", g1_exe())),
            "the missing asset is named: {}",
            err.message
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_a_changed_signer_key_is_a_named_72() {
        let (cache, mirror, key) = g1_mirror("key-pin");
        let release = mirror.join(format!("v{G1_TAG}"));
        g1_write_manifest(&release, &key, true);
        tebako_signer::register_trusted(&cache, &key.public_key).unwrap();
        // the manifest's pin no longer names the signing key (the index
        // itself is honestly re-signed — the LIE is inside the card)
        let manifest_path = release.join("manifest.json");
        let pinned = fs::read_to_string(&manifest_path)
            .unwrap()
            .replace(&hex_lower(&key.keyid), &"0".repeat(16));
        fs::write(&manifest_path, pinned).unwrap();
        g1_sign_file(&release, &key, "manifest.json");
        let r = dll_resolver(&cache, &mirror);
        let err = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap_err();
        assert_eq!(err.code, 72);
        assert!(
            err.message.contains("the signer key changed"),
            "{}",
            err.message
        );
        assert!(!g1_entry_dir(&cache).join(g1_exe()).exists());
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_an_unsigned_release_installs_loudly() {
        let (cache, mirror, _key) = g1_mirror("unsigned");
        let release = mirror.join(format!("v{G1_TAG}"));
        // no signature blocks, no .asc siblings — the pre-signing shape
        g1_write_manifest(&release, &_key, false);
        let r = dll_resolver(&cache, &mirror);
        let resolved = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap();
        assert!(resolved.executable.is_file());
        let journal = g1_journal(&cache);
        assert!(
            journal.contains(&format!(
                "event=unsigned-runtime-fetch runtime_ref=ruby@{G1_RUBY} (tebako {G1_TAG}, {G1_PLATFORM})"
            )),
            "{journal}"
        );
        assert!(!journal.contains("runtime-fetch-verified"), "{journal}");
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_unsigned_gate_matrix() {
        let dir =
            std::env::temp_dir().join(format!("tebako-resolve-g1-gate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let r = Resolver {
            cache_root: dir.join("home"),
            mirror: "file:///nowhere".to_string(),
            lock_timeout: LOCK_TIMEOUT,
        };
        // a declared signature counts as signed coverage, always
        r.unsigned_gate("ruby@x", true, true).unwrap();
        r.unsigned_gate("ruby@x", true, false).unwrap();
        // unsigned + not required: a loud pass (stderr + the journal)
        r.unsigned_gate("ruby@x", false, false).unwrap();
        let journal = g1_journal(&r.cache_root);
        assert!(
            journal.contains("event=unsigned-runtime-fetch runtime_ref=ruby@x"),
            "{journal}"
        );
        // unsigned + TEBAKO_REQUIRE_SIGNED: refused before any download
        let err = r.unsigned_gate("ruby@x", false, true).unwrap_err();
        assert_eq!(err.code, 71);
        assert!(
            err.message.contains("TEBAKO_REQUIRE_SIGNED=1"),
            "{}",
            err.message
        );
        assert!(
            err.message.contains("nothing was cached"),
            "{}",
            err.message
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn g1_a_tampered_index_is_a_named_71() {
        let (cache, mirror, key) = g1_mirror("tampered-index");
        let release = mirror.join(format!("v{G1_TAG}"));
        g1_write_manifest(&release, &key, true);
        g1_sign_file(&release, &key, "manifest.json");
        tebako_signer::register_trusted(&cache, &key.public_key).unwrap();
        // tamper the index AFTER its .asc was written
        let manifest_path = release.join("manifest.json");
        let body = fs::read_to_string(&manifest_path)
            .unwrap()
            .replace("3.3.12", "3.3.99");
        fs::write(&manifest_path, body).unwrap();
        let r = dll_resolver(&cache, &mirror);
        let err = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap_err();
        assert_eq!(err.code, 71);
        assert!(err.message.contains("does not verify"), "{}", err.message);
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_a_verified_form_beats_an_unsigned_one_regardless_of_preference() {
        let (cache, mirror, key) = g1_mirror("verified-wins");
        let release = mirror.join(format!("v{G1_TAG}"));
        g1_write_manifest(&release, &key, true);
        g1_sign_file(&release, &key, "manifest.json");
        tebako_signer::register_trusted(&cache, &key.public_key).unwrap();
        // the shard (the preference-1 form) is UNSIGNED and POISONED —
        // its image sha would fail the install with 121 if it were read
        let shard = format!(
            r#"{{"tebako_version":"{G1_TAG}","contract_era":2,"contract_version":2,"mount_root":"/__tfs__","ruby_version":"{G1_RUBY}","platform":"{G1_PLATFORM}","filename":"{}","sha256":"{}","image":{{"filename":"{}","sha256":"{}"}}}}"#,
            g1_exe(),
            sha256_file_hex(&release.join(g1_exe())).unwrap(),
            g1_image(),
            "f".repeat(64),
        );
        fs::write(release.join(format!("{}.manifest.json", g1_exe())), shard).unwrap();
        let r = dll_resolver(&cache, &mirror);
        r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap();
        let journal = g1_journal(&cache);
        assert!(
            journal.contains("event=runtime-index-verified form=manifest.json"),
            "{journal}"
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }

    #[test]
    fn g1_a_signed_sums_only_release_still_fails_the_contract_gate() {
        let (cache, mirror, key) = g1_mirror("sums-only");
        let release = mirror.join(format!("v{G1_TAG}"));
        // no manifest.json, no shard — SHA256SUMS.txt carries the entry,
        // and it IS signed (the verified index form)
        let sums = format!(
            "{}  {}\n{}  {}\n",
            sha256_file_hex(&release.join(g1_exe())).unwrap(),
            g1_exe(),
            sha256_file_hex(&release.join(g1_image())).unwrap(),
            g1_image(),
        );
        fs::write(release.join("SHA256SUMS.txt"), sums).unwrap();
        g1_sign_file(&release, &key, "SHA256SUMS.txt");
        tebako_signer::register_trusted(&cache, &key.public_key).unwrap();
        let r = dll_resolver(&cache, &mirror);
        let err = r.resolve_runtime(G1_RUBY, G1_PLATFORM, G1_TAG).unwrap_err();
        // the sums form carries no contract card — pre-era, exit 75
        assert_eq!(err.code, 75);
        assert!(err.message.contains("pre-era"), "{}", err.message);
        let journal = g1_journal(&cache);
        assert!(
            journal.contains("event=runtime-index-verified form=SHA256SUMS.txt"),
            "{journal}"
        );
        let _ = fs::remove_dir_all(cache.parent().unwrap());
    }
}
