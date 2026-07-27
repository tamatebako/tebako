//! `tebako publish` (spec 16 §5 developer side, roadmap 41): the release
//! helper for persona C. One flow, all in-process (no gh CLI, no
//! shell-outs — spec 14 §3):
//!
//! 1. **accept per-triplet payloads** (`--payload <triplet>=<path>` for
//!    native-extension apps, one `--payload <path>` for universal) —
//!    produced by prior `tebako press` runs (one per triplet, the CI
//!    matrix); each must carry an embedded manifest (spec 03 §1 — the
//!    registry's entrypoints/runtime mirror comes from it).
//! 2. **optional sign** (`--sign[=<keyid>]`) via tebako-signer: a
//!    detached OpenPGP signature per artifact, uploaded as
//!    `<artifact>.asc` (the convention the installer verifies); the
//!    registry entry pins `{keyid, asc}`.
//! 3. **upload** to the referenced GitHub release (`--release
//!    tfs:github:<owner>/<repo>[:<tag>]`) via tebako-http + the releases
//!    API — or into a `file://` mirror directory (`--upload-mirror`,
//!    the air-gapped rehearsal + test leg; same layout:
//!    `<mirror>/<tag>/<artifact>`). Uploads replace same-named assets —
//!    re-publish is idempotent.
//! 4. **registry**: generate/update `tpkg-registry.yaml` (upsert the
//!    payload's version entry; a re-published version replaces its entry,
//!    a new payload's default starts at its first version).
//! 5. **optional tap** (`--tap <org/homebrew-tap>`): render the vendored
//!    app-formula template (provenance:
//!    tamatebako/homebrew-tap/templates/app-formula.rb.template) from the
//!    published standalones' digests.
//! 6. **built-in verify**: a clean-temp-cache `tebako install` proof —
//!    fresh TEBAKO_HOME, the just-written registry, the mirror (or live
//!    release) as the source; the shims and the trust anchors must land.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tebako_resolve::registry::SignaturePin;
use tebako_resolve::{
    Fetcher, Registry, RegistryPayload, RegistryPlatforms, RegistryRuntimeRequirement,
    RegistryVersion, ReleaseRef, Transport,
};
use tpkg::Platform;

use crate::error::TebakoError;
use crate::image_manifest;

const EX_USAGE: i32 = 64;
const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_UNAVAILABLE: i32 = 69;
const EX_TEBAKO_SIGNATURE: i32 = 71;
const EX_TEBAKO_IO: i32 = 74;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, code)
}

// ---------------------------------------------------------------------
// options
// ---------------------------------------------------------------------

/// One `--payload` argument: a triplet-bound payload (per-triplet apps)
/// or the universal one (`triplet: None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadInput {
    pub triplet: Option<Platform>,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PublishOptions {
    /// The app name (payload + shim names).
    pub name: String,
    /// The version; derived from the payload file names when omitted.
    pub version: Option<String>,
    /// `tfs:github:<owner>/<repo>[:<tag>]` — the release the artifacts
    /// land in (tag defaults to the version). This is ALSO the ref the
    /// registry entry records, mirror mode or not.
    pub release: String,
    pub payloads: Vec<PayloadInput>,
    /// Standalone binaries (`<triplet>=<path>`) — the tap formula's
    /// artifacts; uploaded alongside the payloads.
    pub standalones: Vec<(Platform, PathBuf)>,
    /// `None` = unsigned; `Some(None)` = the press-local key;
    /// `Some(Some(keyid))` = that key from ~/.tebako/keys.
    pub sign: Option<Option<String>>,
    /// file:// rehearsal: upload into `<mirror>/<tag>/` instead of the
    /// live API (the registry still records the --release ref).
    pub upload_mirror: Option<PathBuf>,
    /// Render the tap formula for `<org/homebrew-tap>`.
    pub tap: Option<String>,
    /// Where the formula is written (`<dir>/Formula/<name>.rb`); stdout
    /// when unset.
    pub tap_dir: Option<PathBuf>,
    pub license: Option<String>,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    /// The registry file to generate/update (default
    /// `./tpkg-registry.yaml`; `-` prints to stdout).
    pub registry_out: Option<String>,
    pub skip_verify: bool,
}

/// What a publish produced.
#[derive(Debug)]
pub struct PublishOutcome {
    pub name: String,
    pub version: String,
    pub tag: String,
    /// `(artifact name, sha256)` of every uploaded artifact.
    pub artifacts: Vec<(String, String)>,
    /// Uploaded `.asc` names (when signed).
    pub ascs: Vec<String>,
    /// The signer keyid (when signed).
    pub signer: Option<String>,
    pub registry_path: Option<PathBuf>,
    /// The rendered formula text (when --tap).
    pub formula: Option<String>,
    pub formula_path: Option<PathBuf>,
    /// The built-in verify's summary line (unless --skip-verify).
    pub verified: Option<String>,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------
// the release reference + artifact naming
// ---------------------------------------------------------------------

/// `tfs:github:<owner>/<repo>[:<tag>]` → (owner, repo, tag?). The publish
/// write leg is GitHub-only (the service adapters' read legs cover the
/// rest; their write legs are a later milestone).
fn parse_release_ref(release: &str) -> Result<(String, String, Option<String>), TebakoError> {
    for (prefix, service) in [("tfs:gitlab:", "gitlab"), ("tfs:bb:", "bitbucket")] {
        if release.starts_with(prefix) {
            return Err(err(
                EX_USAGE,
                format!(
                    "publish upload is implemented for GitHub releases (and file:// mirrors) — the {service} write leg is a later milestone"
                ),
            ));
        }
    }
    let Some(rest) = release.strip_prefix("tfs:github:") else {
        return Err(err(
            EX_USAGE,
            format!("invalid --release '{release}' — expected tfs:github:<owner>/<repo>[:<tag>]"),
        ));
    };
    let (owner, repo_tag) = rest.rsplit_once('/').ok_or_else(|| {
        err(
            EX_USAGE,
            format!("invalid --release '{release}' — expected tfs:github:<owner>/<repo>[:<tag>]"),
        )
    })?;
    let (repo, tag) = match repo_tag.split_once(':') {
        Some((r, t)) => (r.to_string(), Some(t.to_string())),
        None => (repo_tag.to_string(), None),
    };
    for (what, value) in [("owner", &owner.to_string()), ("repo", &repo)] {
        if value.is_empty()
            || value
                .chars()
                .any(|c| matches!(c, '?' | '#' | '@' | ' ' | '\t'))
        {
            return Err(err(
                EX_USAGE,
                format!("invalid --release '{release}' — bad {what} '{value}'"),
            ));
        }
    }
    if let Some(t) = &tag {
        if t.is_empty() || t.contains(['?', '#', ' ']) {
            return Err(err(
                EX_USAGE,
                format!("invalid --release '{release}' — bad tag '{t}'"),
            ));
        }
    }
    Ok((owner.to_string(), repo, tag))
}

/// The version from a payload file name: `<name>-<version>.tfs` or
/// `<name>-<version>-<release_asset_name>.tfs`.
fn version_from_artifact(name: &str, file: &Path) -> Option<String> {
    let base = file.file_name()?.to_string_lossy().into_owned();
    let base = base.strip_suffix(".tfs")?;
    let base = Platform::ALL
        .iter()
        .find_map(|p| base.strip_suffix(&format!("-{}", p.release_asset_name())))
        .unwrap_or(base);
    base.strip_prefix(&format!("{name}-")).map(str::to_string)
}

/// The payload's upload name (the locked release-asset convention).
fn payload_artifact_name(name: &str, version: &str, triplet: Option<Platform>) -> String {
    match triplet {
        Some(p) => format!("{name}-{version}-{}.tfs", p.release_asset_name()),
        None => format!("{name}-{version}.tfs"),
    }
}

/// A standalone binary's upload name (no `.tfs` — the tap formula's urls).
fn standalone_artifact_name(name: &str, version: &str, triplet: Platform) -> String {
    format!("{name}-{version}-{}", triplet.release_asset_name())
}

// ---------------------------------------------------------------------
// the upload stores
// ---------------------------------------------------------------------

/// The upload half of a release: idempotent asset placement (replace on
/// re-publish).
trait ReleaseStore {
    fn ensure_release(&self, owner: &str, repo: &str, tag: &str) -> Result<(), TebakoError>;
    fn upload_asset(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), TebakoError>;
}

/// The `file://` mirror store: `<root>/<tag>/<artifact>` (the rehearsal +
/// test leg; the registry still records the GitHub release ref).
struct MirrorStore {
    root: PathBuf,
}

impl ReleaseStore for MirrorStore {
    fn ensure_release(&self, _owner: &str, _repo: &str, tag: &str) -> Result<(), TebakoError> {
        let dir = self.root.join(tag);
        std::fs::create_dir_all(&dir).map_err(|e| {
            err(
                EX_TEBAKO_IO,
                format!("cannot create the release mirror {}: {e}", dir.display()),
            )
        })
    }

    fn upload_asset(
        &self,
        _owner: &str,
        _repo: &str,
        tag: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), TebakoError> {
        let dir = self.root.join(tag);
        let tmp = dir.join(format!(".{name}.{}.part", std::process::id()));
        let dst = dir.join(name);
        std::fs::write(&tmp, bytes)
            .map_err(|e| err(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display())))?;
        // replace on re-publish — idempotent by construction
        std::fs::rename(&tmp, &dst).map_err(|e| {
            err(
                EX_TEBAKO_IO,
                format!("cannot install {}: {e}", dst.display()),
            )
        })
    }
}

/// The GitHub releases store (live API via tebako-http; token from
/// `TEBAKO_GITHUB_TOKEN` or `GITHUB_TOKEN`). Never constructed in tests.
struct GithubStore {
    token: String,
}

impl GithubStore {
    fn release_api_url(owner: &str, repo: &str, tag: &str) -> String {
        format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}")
    }

    fn create_release_url(owner: &str, repo: &str) -> String {
        format!("https://api.github.com/repos/{owner}/{repo}/releases")
    }

    fn create_release_body(tag: &str) -> String {
        format!(
            "{{\"tag_name\":\"{}\",\"name\":\"{}\"}}",
            tebako_json::escape(tag),
            tebako_json::escape(tag)
        )
    }

    /// The asset upload URL from a release's `upload_url` template.
    fn upload_url(upload_url_template: &str, name: &str) -> String {
        let base = upload_url_template
            .split('{')
            .next()
            .unwrap_or(upload_url_template);
        format!("{base}?name={}", tebako_json::escape(name))
    }

    fn delete_asset_url(owner: &str, repo: &str, asset_id: u64) -> String {
        format!("https://api.github.com/repos/{owner}/{repo}/releases/assets/{asset_id}")
    }

    /// The release document `{id, upload_url, assets: [{name, id}]}`,
    /// or None on 404.
    fn get_release(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> Result<Option<ReleaseDoc>, TebakoError> {
        let url = Self::release_api_url(owner, repo, tag);
        let text = match tebako_http::get_text(&url) {
            Ok(t) => t,
            Err(tebako_http::FetchError::IndexUnavailable(_)) => return Ok(None),
            Err(e) => {
                return Err(err(
                    EX_TEBAKO_UNAVAILABLE,
                    format!("cannot read the GitHub release {owner}/{repo}:{tag}: {e}"),
                ))
            }
        };
        match ReleaseDoc::parse(&text) {
            Some(doc) => Ok(Some(doc)),
            None => Err(err(
                EX_TEBAKO_UNAVAILABLE,
                format!("unexpected GitHub release document from {url}"),
            )),
        }
    }
}

/// The bits of a GitHub release document publish needs.
struct ReleaseDoc {
    upload_url: String,
    /// `(asset name, asset id)` for replacement on re-publish.
    assets: Vec<(String, u64)>,
}

impl ReleaseDoc {
    fn parse(text: &str) -> Option<ReleaseDoc> {
        let doc = tebako_json::parse(text).ok()?;
        let upload_url = doc.find("upload_url")?.as_string()?;
        let mut assets = Vec::new();
        if let Some(tebako_json::Value::Array(items)) = doc.find("assets") {
            for a in items {
                let name = a.find("name")?.as_string()?;
                let id = a.find("id")?.as_u64()?;
                assets.push((name, id));
            }
        }
        Some(ReleaseDoc { upload_url, assets })
    }
}

impl ReleaseStore for GithubStore {
    fn ensure_release(&self, owner: &str, repo: &str, tag: &str) -> Result<(), TebakoError> {
        if self.get_release(owner, repo, tag)?.is_some() {
            return Ok(());
        }
        tebako_http::post(
            &Self::create_release_url(owner, repo),
            Self::create_release_body(tag).as_bytes(),
            "application/json",
            Some(&self.token),
        )
        .map_err(|e| {
            err(
                EX_TEBAKO_UNAVAILABLE,
                format!("cannot create the GitHub release {owner}/{repo}:{tag}: {e}"),
            )
        })?;
        Ok(())
    }

    fn upload_asset(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), TebakoError> {
        let release = self.get_release(owner, repo, tag)?.ok_or_else(|| {
            err(
                EX_TEBAKO_UNAVAILABLE,
                format!("the GitHub release {owner}/{repo}:{tag} vanished mid-publish"),
            )
        })?;
        // replace on re-publish: delete a same-named asset first
        if let Some((_, id)) = release.assets.iter().find(|(n, _)| n == name) {
            tebako_http::delete(&Self::delete_asset_url(owner, repo, *id), Some(&self.token))
                .map_err(|e| {
                    err(
                        EX_TEBAKO_UNAVAILABLE,
                        format!("cannot replace the release asset {name}: {e}"),
                    )
                })?;
        }
        tebako_http::post(
            &Self::upload_url(&release.upload_url, name),
            bytes,
            "application/octet-stream",
            Some(&self.token),
        )
        .map_err(|e| {
            err(
                EX_TEBAKO_UNAVAILABLE,
                format!("cannot upload the release asset {name}: {e}"),
            )
        })?;
        Ok(())
    }
}

// ---------------------------------------------------------------------
// the mirror transport (verify against a file:// release mirror)
// ---------------------------------------------------------------------

/// A transport answering the release's API shape from a mirror directory
/// — the verify leg resolves the REAL registry (service ref, per-triplet
/// selection, sha pins) with zero network.
struct MirrorTransport {
    api_url: String,
    dir: PathBuf,
}

impl MirrorTransport {
    fn for_release(owner: &str, repo: &str, tag: &str, mirror: &Path) -> MirrorTransport {
        MirrorTransport {
            api_url: GithubStore::release_api_url(owner, repo, tag),
            dir: mirror.join(tag),
        }
    }
}

impl Transport for MirrorTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>, tebako_http::FetchError> {
        if url == self.api_url {
            let mut assets = String::from("[");
            let mut first = true;
            let entries = std::fs::read_dir(&self.dir).map_err(|e| {
                tebako_http::FetchError::IndexUnavailable(format!("{}: {e}", self.dir.display()))
            })?;
            let mut names: Vec<String> = entries
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('.'))
                .collect();
            names.sort();
            for name in names {
                if !first {
                    assets.push(',');
                }
                first = false;
                assets.push_str(&format!(
                    "{{\"name\":\"{}\",\"browser_download_url\":\"file://{}/{}\"}}",
                    tebako_json::escape(&name),
                    self.dir.display(),
                    tebako_json::escape(&name)
                ));
            }
            assets.push(']');
            return Ok(format!("{{\"assets\":{assets}}}").into_bytes());
        }
        if url.starts_with("file://") {
            return tebako_http::get(url);
        }
        Err(tebako_http::FetchError::IndexUnavailable(url.to_string()))
    }
}

// ---------------------------------------------------------------------
// the tap formula
// ---------------------------------------------------------------------

const TAP_TEMPLATE: &str = include_str!("../templates/app-formula.rb.template");

/// The platforms the vendored template carries sha slots for.
const TAP_PLATFORMS: [Platform; 4] = [
    Platform::Aarch64Macos,
    Platform::X86_64Macos,
    Platform::Aarch64LinuxGnu,
    Platform::X86_64LinuxGnu,
];

fn camelize(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Render the vendored app-formula template (provenance:
/// tamatebako/homebrew-tap/templates/app-formula.rb.template, kept
/// verbatim). Every template sha slot needs a standalone — a missing one
/// is a named error, never a placeholder digest.
fn render_formula(
    opts: &PublishOptions,
    owner: &str,
    repo: &str,
    version: &str,
    standalones: &[(Platform, String)],
) -> Result<String, TebakoError> {
    let mut shas: BTreeMap<Platform, String> = BTreeMap::new();
    for (p, sha) in standalones {
        shas.insert(*p, sha.clone());
    }
    let missing: Vec<&str> = TAP_PLATFORMS
        .iter()
        .filter(|p| !shas.contains_key(p))
        .map(|p| p.release_asset_name())
        .collect();
    if !missing.is_empty() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "--tap needs a standalone for every template platform — missing: {} (--standalone <triplet>=<path>)",
                missing.join(", ")
            ),
        ));
    }
    let out = TAP_TEMPLATE
        .replace("@@CAMELAPP@@", &camelize(&opts.name))
        .replace(
            "@@APP_DESC@@",
            &opts
                .desc
                .clone()
                .unwrap_or_else(|| format!("{} (tebako-packaged)", opts.name)),
        )
        .replace(
            "@@APP_HOMEPAGE@@",
            &opts
                .homepage
                .clone()
                .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}")),
        )
        .replace("@@APP_VERSION@@", version)
        .replace(
            "@@APP_LICENSE_SPDX@@",
            &opts
                .license
                .clone()
                .unwrap_or_else(|| "UNLICENSED".to_string()),
        )
        .replace(
            "@@RELEASE_BASE_URL@@",
            &format!("https://github.com/{owner}/{repo}/releases/download"),
        )
        .replace("@@APP@@", &opts.name)
        .replace("@@SHA256_MACOS_ARM64@@", &shas[&Platform::Aarch64Macos])
        .replace("@@SHA256_MACOS_X86_64@@", &shas[&Platform::X86_64Macos])
        .replace(
            "@@SHA256_LINUX_GNU_ARM64@@",
            &shas[&Platform::Aarch64LinuxGnu],
        )
        .replace(
            "@@SHA256_LINUX_GNU_X86_64@@",
            &shas[&Platform::X86_64LinuxGnu],
        );
    if out
        .lines()
        .any(|line| !line.trim_start().starts_with('#') && line.contains("@@"))
    {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            "the tap template carries an @@PLACEHOLDER@@ this build does not fill — the vendored template drifted",
        ));
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// publish
// ---------------------------------------------------------------------

fn resolve_signing_key(
    home: &Path,
    sign: &Option<Option<String>>,
) -> Result<Option<tebako_signer::PressKey>, TebakoError> {
    match sign {
        None => Ok(None),
        Some(None) => tebako_signer::press_local_key(home)
            .map(Some)
            .map_err(|e| err(EX_TEBAKO_SIGNATURE, format!("the press-local key: {e}"))),
        Some(Some(keyid)) => match tebako_signer::secret_key_by_keyid(home, keyid)
            .map_err(|e| err(EX_TEBAKO_SIGNATURE, format!("cannot read the key store: {e}")))?
        {
            Some(key) => Ok(Some(key)),
            None => Err(err(
                EX_TEBAKO_SIGNATURE,
                format!(
                    "no secret key with keyid {keyid} under {}/keys — generate one, or use bare --sign for the press-local key",
                    home.display()
                ),
            )),
        },
    }
}

/// The production publish (`shim_binary: None` resolves the dispatcher
/// like install does).
pub fn publish(
    opts: &PublishOptions,
    home: &Path,
    cwd: &Path,
) -> Result<PublishOutcome, TebakoError> {
    publish_full(opts, home, cwd, None)
}

/// The full publish flow; `shim_binary` overrides the dispatcher binary
/// the verify install links (tests).
pub fn publish_full(
    opts: &PublishOptions,
    home: &Path,
    cwd: &Path,
    shim_binary: Option<&Path>,
) -> Result<PublishOutcome, TebakoError> {
    let mut notes = Vec::new();

    // ---- 1. inputs --------------------------------------------------
    let (owner, repo, ref_tag) = parse_release_ref(&opts.release)?;
    if opts.payloads.is_empty() {
        return Err(err(
            EX_USAGE,
            "publish needs at least one --payload (<triplet>=<path>, or <path> for universal)",
        ));
    }
    let universal = opts.payloads[0].triplet.is_none();
    if universal && opts.payloads.len() > 1 {
        return Err(err(
            EX_USAGE,
            "a universal payload is one artifact — do not mix it with per-triplet payloads",
        ));
    }
    if !universal && opts.payloads.iter().any(|p| p.triplet.is_none()) {
        return Err(err(
            EX_USAGE,
            "do not mix per-triplet and universal payloads in one publish",
        ));
    }
    {
        let mut triplets: Vec<Platform> = opts.payloads.iter().filter_map(|p| p.triplet).collect();
        triplets.sort();
        if triplets.windows(2).any(|w| w[0] == w[1]) {
            return Err(err(EX_USAGE, "duplicate payload triplet"));
        }
        if let Some(p) = triplets.iter().find(|p| p.is_reserved()) {
            return Err(err(
                EX_USAGE,
                format!("the reserved triplet {p} is not publishable in v1"),
            ));
        }
    }

    let version = match &opts.version {
        Some(v) => v.clone(),
        None => {
            let mut versions = opts
                .payloads
                .iter()
                .map(|p| version_from_artifact(&opts.name, &p.path))
                .collect::<Vec<_>>();
            versions.dedup();
            match versions.as_slice() {
                [Some(v)] => v.clone(),
                _ => {
                    return Err(err(
                        EX_USAGE,
                        format!(
                            "cannot derive the version from the payload file names — pass --version (expected {name}-<version>[-<platform>].tfs)",
                            name = opts.name
                        ),
                    ))
                }
            }
        }
    };
    let tag = ref_tag.unwrap_or_else(|| version.clone());

    // ---- 2. payloads: bytes, digests, the embedded manifest ----------
    let mut artifacts: Vec<(String, String, Vec<u8>)> = Vec::new(); // (upload name, sha, bytes)
    for input in &opts.payloads {
        if !input.path.is_file() {
            return Err(err(
                EX_USAGE,
                format!("payload not found: {}", input.path.display()),
            ));
        }
        let bytes = std::fs::read(&input.path).map_err(|e| {
            err(
                EX_TEBAKO_IO,
                format!("cannot read {}: {e}", input.path.display()),
            )
        })?;
        let name = payload_artifact_name(&opts.name, &version, input.triplet);
        let sha = tebako_resolve::sha256_hex(&bytes);
        artifacts.push((name, sha, bytes));
    }
    let manifest_text = image_manifest::read_embedded_manifest(&opts.payloads[0].path)?
        .ok_or_else(|| {
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "{} carries no embedded manifest (/__tpkg__/manifest.yaml) — the registry's entrypoint mirror comes from it; press/mkimage embeds one",
                    opts.payloads[0].path.display()
                ),
            )
        })?;
    let embedded = tpkg::PayloadManifest::from_yaml(&manifest_text).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the embedded manifest of {} does not parse: {e}",
                opts.payloads[0].path.display()
            ),
        )
    })?;
    if embedded.identity.name != opts.name || embedded.identity.version != version {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the embedded manifest declares {} {} but publish names {} {}",
                embedded.identity.name, embedded.identity.version, opts.name, version
            ),
        ));
    }
    let tpkg::Provides::App(app) = &embedded.provides else {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "publish ships apps — {} is a {:?} payload",
                opts.payloads[0].path.display(),
                embedded.identity.kind
            ),
        ));
    };
    let entrypoints: Vec<String> = app.entrypoints.iter().map(|e| e.name.clone()).collect();
    let runtime_requirement = app
        .entrypoints
        .first()
        .and_then(|e| e.runtime_requirement.as_ref())
        .map(|r| RegistryRuntimeRequirement {
            engine: r.engine.clone(),
            constraint: r.constraint.as_str().to_string(),
        });
    for input in opts.payloads.iter().skip(1) {
        // every triplet's manifest must agree (the registry mirrors ONE set)
        if let Some(text) = image_manifest::read_embedded_manifest(&input.path)? {
            let other = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
                err(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "the embedded manifest of {} does not parse: {e}",
                        input.path.display()
                    ),
                )
            })?;
            let other_eps: Vec<String> = match &other.provides {
                tpkg::Provides::App(a) => a.entrypoints.iter().map(|e| e.name.clone()).collect(),
                _ => Vec::new(),
            };
            if other_eps != entrypoints {
                return Err(err(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "{} declares different entrypoints than the first payload — per-triplet variants of one app must agree",
                        input.path.display()
                    ),
                ));
            }
        }
    }

    // ---- 3. standalones ---------------------------------------------
    let mut standalone_uploads: Vec<(String, String, Vec<u8>)> = Vec::new();
    for (triplet, path) in &opts.standalones {
        if triplet.is_reserved() {
            return Err(err(
                EX_USAGE,
                format!("the reserved triplet {triplet} is not publishable in v1"),
            ));
        }
        let bytes = std::fs::read(path)
            .map_err(|e| err(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())))?;
        let name = standalone_artifact_name(&opts.name, &version, *triplet);
        let sha = tebako_resolve::sha256_hex(&bytes);
        standalone_uploads.push((name, sha, bytes));
    }

    // ---- 4. sign ------------------------------------------------------
    let key = resolve_signing_key(home, &opts.sign)?;
    let mut ascs: Vec<(String, Vec<u8>)> = Vec::new();
    if let Some(key) = &key {
        for (name, _, bytes) in artifacts.iter().chain(standalone_uploads.iter()) {
            let asc = tebako_signer::sign_detached(bytes, &key.secret_key, &key.fingerprint)
                .map_err(|e| err(EX_TEBAKO_SIGNATURE, format!("cannot sign {name}: {e}")))?;
            ascs.push((format!("{name}.asc"), asc));
        }
    }

    // ---- 5. upload ----------------------------------------------------
    let store: Box<dyn ReleaseStore> = match &opts.upload_mirror {
        Some(mirror) => Box::new(MirrorStore {
            root: mirror.clone(),
        }),
        None => {
            let token = std::env::var("TEBAKO_GITHUB_TOKEN")
                .ok()
                .filter(|t| !t.is_empty())
                .or_else(|| std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()))
                .ok_or_else(|| {
                    err(
                        EX_USAGE,
                        "uploading to the live GitHub API needs TEBAKO_GITHUB_TOKEN (or GITHUB_TOKEN); --upload-mirror <dir> rehearses file://-only",
                    )
                })?;
            Box::new(GithubStore { token })
        }
    };
    store.ensure_release(&owner, &repo, &tag)?;
    for (name, _, bytes) in artifacts.iter().chain(standalone_uploads.iter()) {
        store.upload_asset(&owner, &repo, &tag, name, bytes)?;
    }
    for (name, asc) in &ascs {
        store.upload_asset(&owner, &repo, &tag, name, asc)?;
    }

    // ---- 6. the registry ----------------------------------------------
    let release_ref = format!("tfs:github:{owner}/{repo}:{tag}");
    let platforms = if universal {
        RegistryPlatforms::Universal
    } else {
        RegistryPlatforms::PerTriplet(
            opts.payloads
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    (
                        p.triplet.expect("per-triplet checked"),
                        tebako_resolve::registry::PlatformArtifact {
                            artifact: artifacts[i].0.clone(),
                            sha256: artifacts[i].1.clone(),
                        },
                    )
                })
                .collect(),
        )
    };
    let signature = key.as_ref().map(|key| SignaturePin {
        keyid: key.keyid_hex(),
        // universal: the exact asc; per-triplet: the convention asc of the
        // first artifact (the installer derives <selected-artifact>.asc).
        asc: format!("{}.asc", artifacts[0].0),
    });
    let version_entry = RegistryVersion {
        version: version.clone(),
        platforms,
        release: ReleaseRef {
            r#ref: release_ref.clone(),
        },
        signature,
        runtime_requirement,
        entrypoints: entrypoints.clone(),
    };

    let registry_out = opts
        .registry_out
        .clone()
        .unwrap_or_else(|| cwd.join("tpkg-registry.yaml").display().to_string());
    let mut registry = if registry_out == "-" {
        Registry {
            schema_version: 1,
            payloads: Vec::new(),
        }
    } else {
        let path = Path::new(&registry_out);
        match std::fs::read_to_string(path) {
            Ok(text) => Registry::from_yaml(&text).map_err(|e| {
                err(
                    EX_TEBAKO_MANIFEST,
                    format!("cannot parse {}: {e}", path.display()),
                )
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Registry {
                schema_version: 1,
                payloads: Vec::new(),
            },
            Err(e) => {
                return Err(err(
                    EX_TEBAKO_IO,
                    format!("cannot read {}: {e}", path.display()),
                ))
            }
        }
    };
    upsert_registry(&mut registry, &opts.name, version_entry, &mut notes);
    let registry_text = registry.to_yaml().map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("cannot serialize the registry: {e}"),
        )
    })?;
    let registry_path = if registry_out == "-" {
        println!("{registry_text}");
        None
    } else {
        let path = PathBuf::from(&registry_out);
        write_atomic(&path, registry_text.as_bytes())?;
        Some(path)
    };

    // ---- 7. the tap formula -------------------------------------------
    let mut formula = None;
    let mut formula_path = None;
    if opts.tap.is_some() {
        let shas: Vec<(Platform, String)> = opts
            .standalones
            .iter()
            .enumerate()
            .map(|(i, (p, _))| (*p, standalone_uploads[i].1.clone()))
            .collect();
        let rendered = render_formula(opts, &owner, &repo, &version, &shas)?;
        if let Some(dir) = &opts.tap_dir {
            let formula_dir = dir.join("Formula");
            std::fs::create_dir_all(&formula_dir).map_err(|e| {
                err(
                    EX_TEBAKO_IO,
                    format!("cannot create {}: {e}", formula_dir.display()),
                )
            })?;
            let path = formula_dir.join(format!("{}.rb", opts.name));
            write_atomic(&path, rendered.as_bytes())?;
            formula_path = Some(path);
        }
        formula = Some(rendered);
    }

    // ---- 8. the built-in verify ----------------------------------------
    let mut verified = None;
    if !opts.skip_verify {
        let verify_registry_text;
        let registry_ref = match &registry_path {
            Some(path) => format!("file://{}", path.display()),
            None => {
                // the registry went to stdout — the verify reads a temp copy
                let tmp = std::env::temp_dir().join(format!(
                    "tebako-publish-registry-{}.yaml",
                    std::process::id()
                ));
                std::fs::write(&tmp, &registry_text).map_err(|e| {
                    err(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
                })?;
                verify_registry_text = tmp;
                format!("file://{}", verify_registry_text.display())
            }
        };
        let line = verify_install(
            opts,
            &version,
            &registry_ref,
            &tag,
            key.as_ref(),
            shim_binary,
            universal,
        )?;
        verified = Some(line);
    }

    Ok(PublishOutcome {
        name: opts.name.clone(),
        version,
        tag,
        artifacts: artifacts
            .iter()
            .map(|(n, s, _)| (n.clone(), s.clone()))
            .chain(
                standalone_uploads
                    .iter()
                    .map(|(n, s, _)| (n.clone(), s.clone())),
            )
            .collect(),
        ascs: ascs.iter().map(|(n, _)| n.clone()).collect(),
        signer: key.as_ref().map(|k| k.keyid_hex()),
        registry_path,
        formula,
        formula_path,
        verified,
        notes,
    })
}

/// Upsert the payload's version entry: a re-published version REPLACES
/// its entry (idempotent re-publish), a new version appends, a new
/// payload's default starts at its first version.
fn upsert_registry(
    registry: &mut Registry,
    name: &str,
    entry: RegistryVersion,
    notes: &mut Vec<String>,
) {
    let version = entry.version.clone();
    match registry.payloads.iter_mut().find(|p| p.name == name) {
        Some(payload) => {
            match payload.versions.iter_mut().find(|v| v.version == version) {
                Some(existing) => {
                    *existing = entry;
                    notes.push(format!(
                        "replaced the registry entry for {name} {version} (re-publish)"
                    ));
                }
                None => payload.versions.push(entry),
            }
            if payload.default.is_none() {
                payload.default = Some(version);
            }
        }
        None => registry.payloads.push(RegistryPayload {
            name: name.to_string(),
            kind: tpkg::PayloadKind::App,
            versions: vec![entry],
            default: Some(version),
        }),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), TebakoError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", dir.display()),
        )
    })?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
        std::process::id()
    ));
    std::fs::write(&tmp, bytes)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })
}

/// Verify temp homes are unique per call (parallel tests share the
/// process's temp dir).
static VERIFY_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The built-in verify: a clean-temp-cache `tebako install` of the
/// just-published payload — fresh TEBAKO_HOME, the just-written registry,
/// the mirror (or the live release) as the artifact source. Signed
/// publishes register the signer's public key in the temp keyring first
/// (the install's strict signature path is part of the proof).
#[allow(clippy::too_many_arguments)]
fn verify_install(
    opts: &PublishOptions,
    version: &str,
    registry_ref: &str,
    tag: &str,
    key: Option<&tebako_signer::PressKey>,
    shim_binary: Option<&Path>,
    universal: bool,
) -> Result<String, TebakoError> {
    let tmp = std::env::temp_dir().join(format!(
        "tebako-publish-verify-{}-{}-{}",
        opts.name,
        std::process::id(),
        VERIFY_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    let home = tmp.join("home");
    std::fs::create_dir_all(&home).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot create the verify cache {}: {e}", home.display()),
        )
    })?;
    let result = verify_install_at(
        opts,
        &home,
        version,
        registry_ref,
        tag,
        key,
        shim_binary,
        universal,
    );
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

#[allow(clippy::too_many_arguments)]
fn verify_install_at(
    opts: &PublishOptions,
    home: &Path,
    version: &str,
    registry_ref: &str,
    tag: &str,
    key: Option<&tebako_signer::PressKey>,
    shim_binary: Option<&Path>,
    universal: bool,
) -> Result<String, TebakoError> {
    if let Some(key) = key {
        tebako_signer::register_trusted(home, &key.public_key)
            .map_err(|e| err(EX_TEBAKO_SIGNATURE, format!("the verify keyring: {e}")))?;
    }
    let (owner, repo, _) = parse_release_ref(&opts.release)?;
    let host = if universal {
        None // the actual host — universal payloads install anywhere
    } else {
        // proof on the host's triplet when published, else the first one
        let actual = Platform::from_release_asset_name(&crate::options::host_platform()?);
        let first = opts.payloads[0].triplet;
        Some(
            actual
                .filter(|a| opts.payloads.iter().any(|p| p.triplet == Some(*a)))
                .unwrap_or(first.expect("per-triplet checked")),
        )
    };
    let line = match &opts.upload_mirror {
        Some(mirror) => {
            let fetcher =
                Fetcher::with_transport(MirrorTransport::for_release(&owner, &repo, tag, mirror));
            verify_with(
                opts,
                home,
                version,
                registry_ref,
                host,
                shim_binary,
                &fetcher,
            )?
        }
        None => {
            let fetcher = Fetcher::new();
            verify_with(
                opts,
                home,
                version,
                registry_ref,
                host,
                shim_binary,
                &fetcher,
            )?
        }
    };
    Ok(line)
}

/// The install proof over an injected fetcher (mirror or live).
fn verify_with<T: Transport>(
    opts: &PublishOptions,
    home: &Path,
    version: &str,
    registry_ref: &str,
    host: Option<Platform>,
    shim_binary: Option<&Path>,
    fetcher: &Fetcher<T>,
) -> Result<String, TebakoError> {
    crate::install::add_registry_with(home, registry_ref, fetcher)?;
    // the just-published version explicitly (the registry default may
    // point at an older line)
    let target = format!("{}@{version}", opts.name);
    let outcome = crate::install::install_with(home, &target, host, shim_binary, fetcher)?;
    let signed = match &outcome.signer {
        Some(s) => format!(", signed by {s}"),
        None => String::new(),
    };
    Ok(format!(
        "verified: clean-cache install of {} {} ({} shim(s): {}{})",
        opts.name,
        version,
        outcome.commands.len(),
        outcome.commands.join(", "),
        signed
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_urls_and_bodies() {
        assert_eq!(
            GithubStore::release_api_url("acme", "app", "1.0"),
            "https://api.github.com/repos/acme/app/releases/tags/1.0"
        );
        assert_eq!(
            GithubStore::create_release_body("v1.2.3"),
            "{\"tag_name\":\"v1.2.3\",\"name\":\"v1.2.3\"}"
        );
        assert_eq!(
            GithubStore::upload_url(
                "https://uploads.github.com/repos/acme/app/releases/42/assets{?name,label}",
                "app-1.0.tfs"
            ),
            "https://uploads.github.com/repos/acme/app/releases/42/assets?name=app-1.0.tfs"
        );
        assert_eq!(
            GithubStore::delete_asset_url("acme", "app", 42),
            "https://api.github.com/repos/acme/app/releases/assets/42"
        );
    }

    #[test]
    fn release_doc_parses_the_bits_publish_needs() {
        let doc = ReleaseDoc::parse(
            r#"{"id":42,"upload_url":"https://uploads.github.com/x{?name,label}",
               "assets":[{"name":"app-1.0.tfs","id":7},{"name":"app-1.0.tfs.asc","id":8}]}"#,
        )
        .unwrap();
        assert_eq!(doc.upload_url, "https://uploads.github.com/x{?name,label}");
        assert_eq!(
            doc.assets,
            vec![
                ("app-1.0.tfs".to_string(), 7),
                ("app-1.0.tfs.asc".to_string(), 8)
            ]
        );
        assert!(ReleaseDoc::parse("not json").is_none());
    }

    #[test]
    fn camelize_names() {
        assert_eq!(camelize("metanorma"), "Metanorma");
        assert_eq!(camelize("my-app"), "MyApp");
        assert_eq!(camelize("my_app-2"), "MyApp2");
    }

    #[test]
    fn release_ref_forms() {
        let (o, r, t) = parse_release_ref("tfs:github:acme/app").unwrap();
        assert_eq!(
            (o.as_str(), r.as_str(), t.as_deref()),
            ("acme", "app", None)
        );
        let (o, r, t) = parse_release_ref("tfs:github:acme/app:v1.0").unwrap();
        assert_eq!(
            (o.as_str(), r.as_str(), t.as_deref()),
            ("acme", "app", Some("v1.0"))
        );
        for (bad, needle) in [
            ("tfs:gitlab:acme/app", "later milestone"),
            ("tfs:bb:acme/app", "later milestone"),
            ("https://github.com/acme/app", "tfs:github:"),
            ("tfs:github:acme", "<owner>/<repo>"),
        ] {
            let msg = parse_release_ref(bad).unwrap_err().message;
            assert!(msg.contains(needle), "{bad}: expected '{needle}' in {msg}");
        }
    }

    #[test]
    fn artifact_version_derivation() {
        let p = Path::new("/tmp/app-1.2.3.tfs");
        assert_eq!(version_from_artifact("app", p).as_deref(), Some("1.2.3"));
        let p = Path::new("/tmp/app-1.2.3-macos-arm64.tfs");
        assert_eq!(version_from_artifact("app", p).as_deref(), Some("1.2.3"));
        let p = Path::new("/tmp/mystery.bin");
        assert_eq!(version_from_artifact("app", p), None);
        // a name with dashes keeps the version intact
        let p = Path::new("/tmp/my-app-2.0-linux-gnu-x86_64.tfs");
        assert_eq!(version_from_artifact("my-app", p).as_deref(), Some("2.0"));
    }
}
