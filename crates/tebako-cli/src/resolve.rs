//! Port of the gem's RuntimeManager / BootstrapManager
//! (lib/tebako/runtime_manager.rb, lib/tebako/bootstrap_manager.rb):
//! resolution, download, verification and machine-wide caching of the
//! prebuilt tebako runtime packages and tebako-bootstrap launchers.
//!
//! Cache layout (rooted at $TEBAKO_HOME or ~/.tebako), identical to the gem:
//!   runtimes/ruby-<ruby-version>-<tebakoabi>-<platform>/
//!     tebako-runtime-<tebakoabi>-<ruby-version>-<platform>[.exe]
//!     sha256    -- digest the installed file was verified against
//!     origin    -- URL the package was downloaded from
//!   bootstraps/tebako-bootstrap-<version>-<platform>/
//!     tebako-bootstrap-<version>-<platform>[.exe]
//!     sha256 / origin
//!
//! Installs are serialized per entry with a flock'd lockfile; packages are
//! downloaded to tmp/, SHA256-verified against the release index and moved
//! into place with an atomic rename, so partial downloads never poison the
//! cache.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sha2::Digest;

use tebako_pkg::{json_parse, JsonValue};

use crate::error::{packaging_error, TebakoError};
use crate::fetch::{fetch_bytes, fetch_text, FetchError};
use crate::runner::run_with_capture_v;

const TMP_DIR: &str = "tmp";
const LOCK_FILE: &str = ".install.lock";
const SHA256_FILE: &str = "sha256";
const ORIGIN_FILE: &str = "origin";
const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Which release line the resolver consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    /// tamatebako/tebako-runtime-ruby prebuilt runtimes.
    Runtime,
    /// tamatebako/tebako-bootstrap launchers.
    Bootstrap,
}

/// The default tebako-bootstrap release (TEBAKO_BOOTSTRAP_VERSION
/// overrides); 0.2.0 is the first payload-capable one.
pub const BOOTSTRAP_VERSION: &str = "0.2.0";
/// Fat mode requires a payload-capable bootstrap.
pub const PAYLOAD_MIN_VERSION: &str = "0.2.0";

pub fn default_bootstrap_version() -> String {
    match std::env::var("TEBAKO_BOOTSTRAP_VERSION") {
        Ok(v) if !v.is_empty() => v.strip_prefix('v').unwrap_or(&v).to_string(),
        _ => BOOTSTRAP_VERSION.to_string(),
    }
}

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
    /// The runtime image sibling (item 30b): `<asset>.tfs` from the
    /// manifest's additive `image` key or the SHA256SUMS line.
    pub image: Option<ImageRef>,
}

/// A resolved runtime image reference (filename + expected sha256).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    pub filename: String,
    pub sha256: String,
}

/// The outcome of resolving a runtime: the interpreter plus, when the
/// release is image-era, its runtime image reference.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub executable: PathBuf,
    pub image: Option<ImageRef>,
}

#[derive(Debug)]
pub struct Resolver {
    pub flavor: Flavor,
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

impl Resolver {
    pub fn new(flavor: Flavor) -> Self {
        let (env_var, default_mirror) = match flavor {
            Flavor::Runtime => (
                "TEBAKO_RUNTIME_MIRROR",
                "https://github.com/tamatebako/tebako-runtime-ruby/releases/download",
            ),
            Flavor::Bootstrap => (
                "TEBAKO_BOOTSTRAP_MIRROR",
                "https://github.com/tamatebako/tebako-bootstrap/releases/download",
            ),
        };
        let mirror = std::env::var(env_var)
            .ok()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| default_mirror.to_string());
        Resolver {
            flavor,
            cache_root: default_cache_root(),
            mirror: mirror.trim_end_matches('/').to_string(),
            lock_timeout: LOCK_TIMEOUT,
        }
    }

    /// Resolve (download/verify/cache when missing) the package for
    /// `ruby_version` + `platform` at `tebako_version`; returns the cached
    /// executable path. For the Bootstrap flavor `ruby_version` is the
    /// bootstrap version and `tebako_version` equals it.
    pub fn resolve(
        &self,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<PathBuf, TebakoError> {
        let dir = self.entry_dir(ruby_version, platform, tebako_version);
        let executable = dir.join(self.filename(ruby_version, platform, tebako_version));
        if executable.is_file() {
            return Ok(executable);
        }
        self.with_entry_lock(
            &dir,
            &self.entry_ref(ruby_version, platform, tebako_version),
            || {
                if !executable.is_file() {
                    self.install(&executable, ruby_version, platform, tebako_version)?;
                }
                Ok(())
            },
        )?;
        Ok(executable)
    }

    /// Resolve a runtime for press (item 30b): the interpreter plus, when
    /// the release index carries an image entry, the runtime image —
    /// downloaded, verified and marked into the same cache entry the
    /// bootstrap consumes at first run. On a cache hit the image metadata
    /// comes from the entry's trusted marker.
    pub fn resolve_runtime(
        &self,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<Resolved, TebakoError> {
        let dir = self.entry_dir(ruby_version, platform, tebako_version);
        let executable = dir.join(self.filename(ruby_version, platform, tebako_version));
        if executable.is_file() {
            return Ok(Resolved {
                executable,
                image: self.read_image_marker(&dir, ruby_version, platform, tebako_version),
            });
        }
        self.with_entry_lock(
            &dir,
            &self.entry_ref(ruby_version, platform, tebako_version),
            || {
                if executable.is_file() {
                    return Ok(());
                }
                let entry = self.install(&executable, ruby_version, platform, tebako_version)?;
                if let Some(image) = entry.image.clone() {
                    self.install_image(&dir, &image, tebako_version)?;
                }
                Ok(())
            },
        )?;
        Ok(Resolved {
            executable,
            image: self.read_image_marker(&dir, ruby_version, platform, tebako_version),
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
        let filename = self.image_filename(ruby_version, platform, tebako_version);
        if !dir.join(&filename).is_file() {
            return None;
        }
        let marker = fs::read_to_string(dir.join(format!("{filename}.sha256"))).ok()?;
        let sha256 = marker.split_whitespace().next()?.to_string();
        if sha256.len() == 64 {
            Some(ImageRef { filename, sha256 })
        } else {
            None
        }
    }

    /// Download + verify + install the runtime image (0444 + trusted
    /// markers), sharing the bootstrap's cache layout (item 30b). Called
    /// with the entry lock already held.
    fn install_image(
        &self,
        dir: &Path,
        image: &ImageRef,
        tebako_version: &str,
    ) -> Result<(), TebakoError> {
        let image_path = dir.join(&image.filename);
        let marker = dir.join(format!("{}.sha256", image.filename));
        if image_path.is_file() && marker.is_file() {
            return Ok(());
        }
        let url = self.package_url(&image.filename, tebako_version);
        let tmp_dir = self.cache_root.join(TMP_DIR);
        let tmp = tmp_dir.join(format!("{}.{}.part", image.filename, std::process::id()));
        match fetch_bytes(&url) {
            Ok(bytes) => {
                if let Err(e) = crate::fetch::write_tmp(&tmp, &bytes) {
                    let _ = fs::remove_file(&tmp);
                    return Err(packaging_error(122, Some(&format!("{e} writing {url}"))));
                }
            }
            Err(FetchError::IndexUnavailable(_)) => {
                let _ = fs::remove_file(&tmp);
                return Err(packaging_error(122, Some(&format!("{url}: not found"))));
            }
            Err(FetchError::DownloadFailed(msg)) => {
                let _ = fs::remove_file(&tmp);
                return Err(packaging_error(122, Some(&msg)));
            }
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
        let mut perms = fs::metadata(&tmp).map_err(err)?.permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&tmp, perms).map_err(err)?;
        fs::rename(&tmp, &image_path).map_err(err)?;
        fs::write(&marker, format!("{expected}  {}\n", image.filename)).map_err(err)?;
        fs::write(
            dir.join(format!("{}.origin", image.filename)),
            format!("{url}\n"),
        )
        .map_err(err)?;
        Ok(())
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
        let base = self.cache_root.join(self.cache_subdir());
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

    // ---- flavor specifics ------------------------------------------------

    fn cache_subdir(&self) -> &'static str {
        match self.flavor {
            Flavor::Runtime => "runtimes",
            Flavor::Bootstrap => "bootstraps",
        }
    }

    fn index_files(&self) -> &'static [&'static str] {
        match self.flavor {
            Flavor::Runtime => &["manifest.json", "SHA256SUMS.txt"],
            Flavor::Bootstrap => &["manifest.json", "SHA256SUMS"],
        }
    }

    fn release_name(&self) -> &'static str {
        match self.flavor {
            Flavor::Runtime => "tebako-runtime-ruby",
            Flavor::Bootstrap => "tebako-bootstrap",
        }
    }

    fn mirror_env_var(&self) -> &'static str {
        match self.flavor {
            Flavor::Runtime => "TEBAKO_RUNTIME_MIRROR",
            Flavor::Bootstrap => "TEBAKO_BOOTSTRAP_MIRROR",
        }
    }

    fn entry_dir(&self, ruby_version: &str, platform: &str, tebako_version: &str) -> PathBuf {
        match self.flavor {
            Flavor::Runtime => self
                .cache_root
                .join("runtimes")
                .join(format!("ruby-{ruby_version}-{tebako_version}-{platform}")),
            Flavor::Bootstrap => self
                .cache_root
                .join("bootstraps")
                .join(format!("tebako-bootstrap-{tebako_version}-{platform}")),
        }
    }

    fn filename(&self, ruby_version: &str, platform: &str, tebako_version: &str) -> String {
        let suffix = if platform.starts_with("windows") {
            ".exe"
        } else {
            ""
        };
        match self.flavor {
            Flavor::Runtime => {
                format!("tebako-runtime-{tebako_version}-{ruby_version}-{platform}{suffix}")
            }
            Flavor::Bootstrap => format!("tebako-bootstrap-{tebako_version}-{platform}{suffix}"),
        }
    }

    fn entry_ref(&self, ruby_version: &str, platform: &str, tebako_version: &str) -> String {
        match self.flavor {
            Flavor::Runtime => format!("ruby@{ruby_version} (tebako {tebako_version}, {platform})"),
            Flavor::Bootstrap => format!("tebako-bootstrap@{tebako_version} ({platform})"),
        }
    }

    // ---- install pipeline ------------------------------------------------

    fn install(
        &self,
        executable: &Path,
        ruby_version: &str,
        platform: &str,
        tebako_version: &str,
    ) -> Result<IndexEntry, TebakoError> {
        let entry_ref = self.entry_ref(ruby_version, platform, tebako_version);
        self.offline_check(&entry_ref, tebako_version)?;
        let index = self.fetch_index(tebako_version)?;
        let entry = self.find_entry(&index, ruby_version, platform, tebako_version)?;
        let url = self.package_url(&entry.filename, tebako_version);
        let tmp = self.download(&url, &entry.filename)?;
        self.verify(&tmp, entry)?;
        self.place(&tmp, executable, entry, &url)?;
        Ok(entry.clone())
    }

    fn offline(&self) -> bool {
        std::env::var("TEBAKO_OFFLINE")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false)
    }

    fn offline_check(&self, entry_ref: &str, tebako_version: &str) -> Result<(), TebakoError> {
        if !self.offline() {
            return Ok(());
        }
        let code = match self.flavor {
            Flavor::Runtime => 123,
            Flavor::Bootstrap => 132,
        };
        Err(packaging_error(
            code,
            Some(&format!(
                "{} is not cached and downloads are disabled (release index: {}; {}={})",
                entry_ref,
                self.index_urls(tebako_version).join(", "),
                self.mirror_env_var(),
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
        match self.flavor {
            Flavor::Runtime => {
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
            Flavor::Bootstrap => {
                if let Some(entry) = index
                    .iter()
                    .find(|c| c.platform.as_deref() == Some(platform))
                {
                    return Ok(entry);
                }
                let platforms: Vec<String> = index
                    .iter()
                    .map(|c| c.platform.clone().unwrap_or_else(|| "?".to_string()))
                    .collect();
                Err(packaging_error(
                    131,
                    Some(&format!(
                        "no tebako-bootstrap package for {platform} (bootstrap {tebako_version}). Available: {}.",
                        platforms.join(", ")
                    )),
                ))
            }
        }
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
        let mut perms = fs::metadata(tmp).map_err(err)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(tmp, perms).map_err(err)?;
        fs::rename(tmp, executable).map_err(err)?;
        let dir = executable.parent().unwrap_or_else(|| Path::new("."));
        fs::write(dir.join(SHA256_FILE), format!("{}\n", entry.sha256)).map_err(err)?;
        fs::write(dir.join(ORIGIN_FILE), format!("{url}\n")).map_err(err)?;
        Ok(())
    }

    fn fetch_index(&self, tebako_version: &str) -> Result<Vec<IndexEntry>, TebakoError> {
        let mut tried: Vec<String> = Vec::new();
        for name in self.index_files() {
            let url = self.index_url(name, tebako_version);
            match fetch_text(&url) {
                Ok(body) => match self.parse_index(name, &body, tebako_version) {
                    Ok(entries) => return Ok(entries),
                    Err(FetchError::IndexUnavailable(_)) => tried.push(url),
                    Err(FetchError::DownloadFailed(msg)) => {
                        return Err(packaging_error(122, Some(&msg)));
                    }
                },
                Err(FetchError::IndexUnavailable(_)) => tried.push(url),
                Err(FetchError::DownloadFailed(msg)) => {
                    return Err(packaging_error(122, Some(&msg)))
                }
            }
        }
        Err(packaging_error(
            124,
            Some(&format!(
                "{} release v{} provides no usable package index (tried: {})",
                self.release_name(),
                tebako_version,
                tried.join(", ")
            )),
        ))
    }

    fn parse_index(
        &self,
        name: &str,
        body: &str,
        tebako_version: &str,
    ) -> Result<Vec<IndexEntry>, FetchError> {
        if name == "manifest.json" {
            self.parse_manifest(body, tebako_version)
        } else {
            Ok(self.parse_sha256sums(body, tebako_version))
        }
    }

    /// Runtime: the tebako-runtime-ruby manifest.json is an array of
    /// {tebako_version, ruby_version, platform, filename, sha256, ...}.
    /// Bootstrap: the tebako-bootstrap manifest.json is an object
    /// {name, version, assets: [{platform, file, sha256}]}.
    fn parse_manifest(
        &self,
        body: &str,
        tebako_version: &str,
    ) -> Result<Vec<IndexEntry>, FetchError> {
        let data = json_parse(body).map_err(|_| {
            FetchError::IndexUnavailable("manifest.json is not valid JSON".to_string())
        })?;
        match self.flavor {
            Flavor::Runtime => {
                let JsonValue::Array(items) = data else {
                    return Err(FetchError::IndexUnavailable(
                        "manifest.json is not an array".to_string(),
                    ));
                };
                Ok(items
                    .iter()
                    .filter(|e| {
                        e.find("tebako_version")
                            .and_then(|v| v.as_string())
                            .as_deref()
                            == Some(tebako_version)
                            && e.find("sha256").and_then(|v| v.as_string()).is_some()
                            && e.find("filename").and_then(|v| v.as_string()).is_some()
                    })
                    .map(|e| IndexEntry {
                        ruby_version: e.find("ruby_version").and_then(|v| v.as_string()),
                        platform: e.find("platform").and_then(|v| v.as_string()),
                        filename: e
                            .find("filename")
                            .and_then(|v| v.as_string())
                            .unwrap_or_default(),
                        sha256: e
                            .find("sha256")
                            .and_then(|v| v.as_string())
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                        image: e.find("image").and_then(|img| {
                            Some(ImageRef {
                                filename: img.find("filename").and_then(|v| v.as_string())?,
                                sha256: img
                                    .find("sha256")
                                    .and_then(|v| v.as_string())?
                                    .to_ascii_lowercase(),
                            })
                        }),
                    })
                    .collect())
            }
            Flavor::Bootstrap => {
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
                    .map(|a| IndexEntry {
                        ruby_version: None,
                        platform: a.find("platform").and_then(|v| v.as_string()),
                        filename: a
                            .find("file")
                            .and_then(|v| v.as_string())
                            .unwrap_or_default(),
                        sha256: a
                            .find("sha256")
                            .and_then(|v| v.as_string())
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                        image: None,
                    })
                    .collect())
            }
        }
    }

    /// `<sha256>  <filename>` lines; filenames may carry a `*` prefix.
    /// Runtime lines may name the image sibling (`<asset>.tfs`, item
    /// 30b): they attach to the matching runtime entry as its `image`.
    fn parse_sha256sums(&self, body: &str, tebako_version: &str) -> Vec<IndexEntry> {
        let mut out: Vec<IndexEntry> = Vec::new();
        let mut images: Vec<(String, String, String)> = Vec::new(); // (rv, platform, ImageRef parts)
        for line in body.lines() {
            let mut parts = line.trim().splitn(2, char::is_whitespace);
            let (Some(sha256), Some(file)) = (parts.next(), parts.next()) else {
                continue;
            };
            let file = file.trim().trim_start_matches('*');
            match self.flavor {
                Flavor::Runtime => {
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
                    let rest = rest.strip_suffix(".exe").unwrap_or(rest);
                    let Some((ruby_version, platform)) = split_ruby_platform(rest) else {
                        continue;
                    };
                    out.push(IndexEntry {
                        ruby_version: Some(ruby_version),
                        platform: Some(platform),
                        filename: file.to_string(),
                        sha256: sha256.to_ascii_lowercase(),
                        image: None,
                    });
                }
                Flavor::Bootstrap => {
                    let prefix = format!("tebako-bootstrap-{tebako_version}-");
                    let Some(rest) = file.strip_prefix(&prefix) else {
                        continue;
                    };
                    let platform = rest.strip_suffix(".exe").unwrap_or(rest);
                    if platform.is_empty() {
                        continue;
                    }
                    out.push(IndexEntry {
                        ruby_version: None,
                        platform: Some(platform.to_string()),
                        filename: file.to_string(),
                        sha256: sha256.to_ascii_lowercase(),
                        image: None,
                    });
                }
            }
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
                });
            }
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
                    return Err(packaging_error(122, Some(&format!("{e} writing {url}"))));
                }
                Ok(tmp)
            }
            Err(FetchError::IndexUnavailable(_)) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(122, Some(&format!("{url}: not found"))))
            }
            Err(FetchError::DownloadFailed(msg)) => {
                let _ = fs::remove_file(&tmp);
                Err(packaging_error(122, Some(&msg)))
            }
        }
    }

    // ---- locking ---------------------------------------------------------

    fn with_entry_lock<F>(&self, dir: &Path, entry_ref: &str, f: F) -> Result<(), TebakoError>
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
            .map_err(|e| {
                crate::error::plain_error(format!("{e} opening {}", lock_path.display()))
            })?;
        self.acquire_lock(&lock, entry_ref, &lock_path)?;
        let result = f();
        flock(&lock, libc::LOCK_UN);
        result
    }

    fn acquire_lock(
        &self,
        lock: &fs::File,
        entry_ref: &str,
        lock_path: &Path,
    ) -> Result<(), TebakoError> {
        let deadline = std::time::Instant::now() + self.lock_timeout;
        loop {
            if flock(lock, libc::LOCK_EX | libc::LOCK_NB) {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err(packaging_error(
                    125,
                    Some(&format!(
                        "{entry_ref}: another process is installing this runtime (no lock after {}s; lockfile: {})",
                        self.lock_timeout.as_secs(),
                        lock_path.display()
                    )),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
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
        self.index_files()
            .iter()
            .map(|n| self.index_url(n, tebako_version))
            .collect()
    }

    fn package_url(&self, filename: &str, tebako_version: &str) -> String {
        format!("{}/{filename}", self.release_url(tebako_version))
    }
}

/// flock(2) wrapper; returns true when the operation succeeded.
fn flock(file: &fs::File, op: i32) -> bool {
    use std::os::unix::io::AsRawFd;
    unsafe { libc::flock(file.as_raw_fd(), op) == 0 }
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
        let r = Resolver::new(Flavor::Runtime);
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
    fn sha256sums_bootstrap_entries() {
        let r = Resolver::new(Flavor::Bootstrap);
        let body = "aaa111  tebako-bootstrap-0.2.0-macos-arm64\n\
                    bbb222  tebako-bootstrap-0.2.0-windows-x86_64.exe\n";
        let entries = r.parse_sha256sums(body, "0.2.0");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].platform.as_deref(), Some("macos-arm64"));
        assert_eq!(entries[1].platform.as_deref(), Some("windows-x86_64"));
    }

    #[test]
    fn manifest_runtime_array() {
        let r = Resolver::new(Flavor::Runtime);
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
        let r = Resolver::new(Flavor::Runtime);
        let err = r.parse_manifest("{\"assets\":[]}", "0.15.9").unwrap_err();
        assert!(matches!(err, FetchError::IndexUnavailable(_)));
    }

    #[test]
    fn manifest_bootstrap_object() {
        let r = Resolver::new(Flavor::Bootstrap);
        let body = r#"{"name":"tebako-bootstrap","version":"0.2.0","assets":[
          {"platform":"macos-arm64","file":"tebako-bootstrap-0.2.0-macos-arm64","sha256":"DEF"}
        ]}"#;
        let entries = r.parse_manifest(body, "0.2.0").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].platform.as_deref(), Some("macos-arm64"));
        assert_eq!(entries[0].sha256, "def");
    }
}
