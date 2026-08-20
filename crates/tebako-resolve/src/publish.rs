//! Publishing (spec 16 §5, roadmap 41): the write side of the git host's
//! release API and the registry-side half of `tebako publish`.
//!
//! The resolver's adapters READ the GitHub releases API; this module
//! WRITES it — create-or-reuse the release for a tag, then replace each
//! asset (delete-then-upload: the API's replace semantics, so re-running
//! publish for the same version is idempotent) — plus the contents-API
//! commit that bumps a Homebrew tap formula. All in-process HTTP through
//! [`PublishTransport`] (production: [`HttpPublishTransport`] over
//! tebako-http; tests plug a mock — no `gh` CLI anywhere, spec 14 §3).
//!
//! [`upsert_entry`] is the registry half: build a version entry from the
//! published artifacts and merge it into the `tpkg-registry.yaml` model
//! (spec 04 §2) — the model round-trips losslessly, so re-publishing a
//! version replaces exactly that entry and leaves every other payload and
//! version untouched.

use std::collections::BTreeMap;

use base64::Engine as _;
use tebako_http::FetchError;
use tebako_json::{escape as json_escape, parse as json_parse, Value as JsonValue};
use tpkg::{PayloadKind, Platform};

use crate::error::RegistryError;
use crate::registry::{
    PlatformArtifact, Registry, RegistryPayload, RegistryPlatforms, RegistryRuntimeRequirement,
    RegistryVersion, ReleaseRef, SignaturePin,
};

// ---------------------------------------------------------------------
// The transport seam
// ---------------------------------------------------------------------

/// The release-API write transport. Production code goes through
/// [`HttpPublishTransport`] (tebako-http); tests plug a mock. Mirrors
/// [`tebako_http::request`]: any completed HTTP exchange is `Ok((status,
/// body))`; only transport failures are `Err`.
pub trait PublishTransport {
    fn request(
        &self,
        method: &str,
        url: &str,
        token: &str,
        content_type: Option<&str>,
        body: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>), ReleaseError>;
}

impl<T: PublishTransport + ?Sized> PublishTransport for &T {
    fn request(
        &self,
        method: &str,
        url: &str,
        token: &str,
        content_type: Option<&str>,
        body: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>), ReleaseError> {
        (**self).request(method, url, token, content_type, body)
    }
}

/// The production transport: tebako-http with the GitHub API headers
/// (bearer token, JSON accept, a named user agent).
#[derive(Debug, Default, Clone, Copy)]
pub struct HttpPublishTransport;

impl PublishTransport for HttpPublishTransport {
    fn request(
        &self,
        method: &str,
        url: &str,
        token: &str,
        content_type: Option<&str>,
        body: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>), ReleaseError> {
        let mut headers: Vec<(&str, &str)> = vec![
            ("Authorization", token),
            ("Accept", "application/vnd.github+json"),
            ("User-Agent", "tebako-publish (tebako-rs)"),
            ("X-GitHub-Api-Version", "2022-11-28"),
        ];
        if let Some(ct) = content_type {
            headers.push(("Content-Type", ct));
        }
        tebako_http::request(method, url, &headers, body).map_err(|e| match e {
            FetchError::IndexUnavailable(origin) => ReleaseError::NotFound { origin },
            FetchError::DownloadFailed(reason) => ReleaseError::Transport { reason },
        })
    }
}

/// Named publish-API errors (spec 00 invariant 9).
#[derive(Debug)]
pub enum ReleaseError {
    /// A transport failure before any HTTP status came back.
    Transport { reason: String },
    /// 401/403 — the token is missing, invalid, or under-scoped.
    Auth { origin: String },
    /// 404 where the object was expected to exist.
    NotFound { origin: String },
    /// Any other non-2xx status, or an unusable 2xx body.
    Api {
        origin: String,
        status: u16,
        reason: String,
    },
}

impl std::fmt::Display for ReleaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseError::Transport { reason } => write!(f, "release API request failed: {reason}"),
            ReleaseError::Auth { origin } => write!(
                f,
                "release API refused the credentials for {origin} (check GITHUB_TOKEN: a token with contents+release write scope is required)"
            ),
            ReleaseError::NotFound { origin } => write!(f, "not found: {origin}"),
            ReleaseError::Api {
                origin,
                status,
                reason,
            } => write!(f, "release API error {status} for {origin}: {reason}"),
        }
    }
}

impl std::error::Error for ReleaseError {}

// ---------------------------------------------------------------------
// The GitHub releases client
// ---------------------------------------------------------------------

const API: &str = "https://api.github.com";
const UPLOADS: &str = "https://uploads.github.com";

/// The outcome of one [`GithubReleaseClient::publish`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedRelease {
    /// The release tag (`v<version>`).
    pub tag: String,
    /// Every asset uploaded (in argument order).
    pub uploaded: Vec<String>,
    /// The subset of `uploaded` that replaced a same-named existing asset
    /// (the idempotent re-publish path).
    pub replaced: Vec<String>,
}

/// The write side of one repository's releases API.
pub struct GithubReleaseClient<'a, T: PublishTransport> {
    pub transport: &'a T,
    pub owner: &'a str,
    pub repo: &'a str,
    /// The `Authorization: Bearer …` header value (token included).
    pub token: &'a str,
}

/// One release as the API reports it: id plus its `(asset id, name)` list.
struct ReleaseState {
    id: u64,
    assets: Vec<(u64, String)>,
}

impl<T: PublishTransport> GithubReleaseClient<'_, T> {
    /// Publish `assets` to the release for `tag`, creating the release
    /// when missing and replacing same-named assets when it already
    /// exists (idempotent re-publish, locked: delete-then-upload per
    /// asset — the release never carries a half-updated asset name).
    pub fn publish(
        &self,
        tag: &str,
        assets: &[(String, Vec<u8>)],
    ) -> Result<PublishedRelease, ReleaseError> {
        let release = match self.release_by_tag(tag)? {
            Some(state) => state,
            None => self.create_release(tag)?,
        };
        let mut uploaded = Vec::new();
        let mut replaced = Vec::new();
        for (name, bytes) in assets {
            if let Some((asset_id, _)) = release.assets.iter().find(|(_, n)| n == name) {
                self.delete_asset(*asset_id)?;
                replaced.push(name.clone());
            }
            self.upload_asset(release.id, name, bytes)?;
            uploaded.push(name.clone());
        }
        Ok(PublishedRelease {
            tag: tag.to_string(),
            uploaded,
            replaced,
        })
    }

    fn release_by_tag(&self, tag: &str) -> Result<Option<ReleaseState>, ReleaseError> {
        let url = format!(
            "{API}/repos/{}/{}/releases/tags/{}",
            self.owner, self.repo, tag
        );
        let (status, body) = self.transport.request("GET", &url, self.token, None, None)?;
        match status {
            200 => parse_release(&url, &body).map(Some),
            404 => Ok(None),
            other => Err(status_error(&url, other, "fetching the release")),
        }
    }

    fn create_release(&self, tag: &str) -> Result<ReleaseState, ReleaseError> {
        let url = format!("{API}/repos/{}/{}/releases", self.owner, self.repo);
        let json = format!(
            "{{\"tag_name\":\"{}\",\"name\":\"{}\"}}",
            json_escape(tag),
            json_escape(tag)
        );
        let (status, body) =
            self.transport
                .request("POST", &url, self.token, Some("application/json"), Some(json.as_bytes()))?;
        match status {
            200 | 201 => parse_release(&url, &body),
            other => Err(status_error(&url, other, "creating the release")),
        }
    }

    fn delete_asset(&self, asset_id: u64) -> Result<(), ReleaseError> {
        let url = format!(
            "{API}/repos/{}/{}/releases/assets/{asset_id}",
            self.owner, self.repo
        );
        let (status, _) = self.transport.request("DELETE", &url, self.token, None, None)?;
        match status {
            204 => Ok(()),
            other => Err(status_error(&url, other, "replacing the asset")),
        }
    }

    fn upload_asset(&self, release_id: u64, name: &str, bytes: &[u8]) -> Result<(), ReleaseError> {
        let url = format!(
            "{UPLOADS}/repos/{}/{}/releases/{release_id}/assets?name={name}",
            self.owner, self.repo
        );
        let (status, _) = self.transport.request(
            "POST",
            &url,
            self.token,
            Some("application/octet-stream"),
            Some(bytes),
        )?;
        match status {
            200 | 201 => Ok(()),
            other => Err(status_error(&url, other, "uploading the asset")),
        }
    }
}

fn status_error(url: &str, status: u16, what: &str) -> ReleaseError {
    match status {
        401 | 403 => ReleaseError::Auth {
            origin: url.to_string(),
        },
        404 => ReleaseError::NotFound {
            origin: url.to_string(),
        },
        other => ReleaseError::Api {
            origin: url.to_string(),
            status: other,
            reason: format!("unexpected status {what}"),
        },
    }
}

fn parse_release(url: &str, body: &[u8]) -> Result<ReleaseState, ReleaseError> {
    let text = String::from_utf8(body.to_vec()).map_err(|e| ReleaseError::Api {
        origin: url.to_string(),
        status: 200,
        reason: format!("{e} decoding the release document"),
    })?;
    let doc = json_parse(&text).map_err(|e| ReleaseError::Api {
        origin: url.to_string(),
        status: 200,
        reason: format!("invalid JSON: {e}"),
    })?;
    let id = doc
        .find("id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ReleaseError::Api {
            origin: url.to_string(),
            status: 200,
            reason: "the release document carries no id".to_string(),
        })?;
    let mut assets = Vec::new();
    if let Some(JsonValue::Array(items)) = doc.find("assets") {
        for item in items {
            let (Some(aid), Some(name)) = (
                item.find("id").and_then(|v| v.as_u64()),
                item.find("name").and_then(|v| v.as_string()),
            ) else {
                continue;
            };
            assets.push((aid, name));
        }
    }
    Ok(ReleaseState { id, assets })
}

// ---------------------------------------------------------------------
// The contents API (the Homebrew tap bump)
// ---------------------------------------------------------------------

/// What a tap formula commit did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    Created,
    Updated,
}

/// Create or update `path` in `{owner}/{repo}` through the GitHub
/// contents API (the tap formula bump — in-process, no git CLI). Update
/// reads the existing file's blob sha first (the API requires it).
pub fn commit_file<T: PublishTransport>(
    transport: &T,
    owner: &str,
    repo: &str,
    path: &str,
    content: &[u8],
    message: &str,
    token: &str,
) -> Result<CommitOutcome, ReleaseError> {
    let url = format!("{API}/repos/{owner}/{repo}/contents/{path}");
    let (status, body) = transport.request("GET", &url, token, None, None)?;
    let existing_sha = match status {
        200 => {
            let text = String::from_utf8(body).map_err(|e| ReleaseError::Api {
                origin: url.clone(),
                status: 200,
                reason: format!("{e} decoding the contents document"),
            })?;
            let doc = json_parse(&text).map_err(|e| ReleaseError::Api {
                origin: url.clone(),
                status: 200,
                reason: format!("invalid JSON: {e}"),
            })?;
            doc.find("sha").and_then(|v| v.as_string()).ok_or_else(|| {
                ReleaseError::Api {
                    origin: url.clone(),
                    status: 200,
                    reason: "the contents document carries no sha".to_string(),
                }
            })?
        }
        404 => String::new(),
        other => return Err(status_error(&url, other, "reading the file")),
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(content);
    let json = if existing_sha.is_empty() {
        format!(
            "{{\"message\":\"{}\",\"content\":\"{encoded}\"}}",
            json_escape(message)
        )
    } else {
        format!(
            "{{\"message\":\"{}\",\"content\":\"{encoded}\",\"sha\":\"{}\"}}",
            json_escape(message),
            json_escape(&existing_sha)
        )
    };
    let (status, _) = transport.request(
        "PUT",
        &url,
        token,
        Some("application/json"),
        Some(json.as_bytes()),
    )?;
    match (status, existing_sha.is_empty()) {
        (200 | 201, true) => Ok(CommitOutcome::Created),
        (200 | 201, false) => Ok(CommitOutcome::Updated),
        (other, _) => Err(status_error(&url, other, "committing the file")),
    }
}

// ---------------------------------------------------------------------
// Naming (spec 16 §2's artifact forms)
// ---------------------------------------------------------------------

/// The GitHub release tag for a version: `v`-prefixed unless the version
/// already carries the prefix (the brew template downloads from
/// `…/releases/download/v<version>/…`; the registry's release ref carries
/// exactly this tag).
pub fn release_tag(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    }
}

/// The payload asset name (spec 16 §2): per-triplet
/// `<name>-<version>-<asset-triplet>.tfs`, universal `<name>-<version>.tfs`.
pub fn artifact_name(name: &str, version: &str, platform: Option<Platform>) -> String {
    match platform {
        Some(p) => format!("{name}-{version}-{}.tfs", p.release_asset_name()),
        None => format!("{name}-{version}.tfs"),
    }
}

/// The standalone-binary asset name (the brew formula's download form):
/// `<name>-<version>-<asset-triplet>`, no extension.
pub fn binary_asset_name(name: &str, version: &str, platform: Platform) -> String {
    format!("{name}-{version}-{}", platform.release_asset_name())
}

// ---------------------------------------------------------------------
// Registry entry generation (spec 04 §2 — the lossless model upsert)
// ---------------------------------------------------------------------

/// Everything publish computed for one version of one payload.
#[derive(Debug, Clone)]
pub struct EntrySpec {
    pub name: String,
    pub kind: PayloadKind,
    pub version: String,
    /// Universal payloads: the artifact name and its sha256 (the pin rides
    /// the release ref — the model's universal digest channel).
    pub universal: Option<(String, String)>,
    /// Per-triplet payloads (triplet → artifact + sha256).
    pub per_triplet: BTreeMap<Platform, PlatformArtifact>,
    /// `tfs:github:owner/repo:<tag>` (no pin; universal adds its own).
    pub release_ref: String,
    /// The signature pin (spec 09, opt-in). Per-triplet publishes sign
    /// every artifact; the version-level block names one of them (the
    /// model carries one asc per version — per-platform pins are a spec 04
    /// model extension, not roadmap 41).
    pub signature: Option<SignaturePin>,
    pub runtime_requirement: Option<RegistryRuntimeRequirement>,
    pub entrypoints: Vec<String>,
    /// Publish bumps the payload default to the version it just shipped.
    pub set_default: bool,
}

/// Merge one published version into the registry model: upsert the
/// payload (kind must agree), replace a same-version entry wholesale
/// (idempotent re-publish) or append a new one, and bump the default when
/// asked. The result is re-validated, so what lands on disk always parses
/// back — the round-trip the model guarantees.
pub fn upsert_entry(registry: &mut Registry, spec: &EntrySpec) -> Result<(), RegistryError> {
    let invalid = |reason: String| RegistryError::Invalid { reason };
    if spec.universal.is_some() == !spec.per_triplet.is_empty() {
        return Err(invalid(format!(
            "payload '{}' {} must publish either a universal artifact or a per-triplet map, not both",
            spec.name, spec.version
        )));
    }
    let (platforms, release_ref) = match &spec.universal {
        // The artifact name is the release's single .tfs — needed by the
        // caller's asset list, not mirrored per-platform.
        Some((_artifact, sha256)) => (
            RegistryPlatforms::Universal,
            format!("{}?sha256={sha256}", spec.release_ref),
        ),
        None => (
            RegistryPlatforms::PerTriplet(spec.per_triplet.clone()),
            spec.release_ref.clone(),
        ),
    };
    let version_entry = RegistryVersion {
        version: spec.version.clone(),
        platforms,
        release: ReleaseRef { r#ref: release_ref },
        signature: spec.signature.clone(),
        runtime_requirement: spec.runtime_requirement.clone(),
        entrypoints: spec.entrypoints.clone(),
    };

    let payload = match registry
        .payloads
        .iter_mut()
        .find(|p| p.name == spec.name)
    {
        Some(p) => {
            if p.kind != spec.kind {
                return Err(invalid(format!(
                    "payload '{}' is registered as kind {:?} but publish declares {:?}",
                    spec.name, p.kind, spec.kind
                )));
            }
            p
        }
        None => {
            registry.payloads.push(RegistryPayload {
                name: spec.name.clone(),
                kind: spec.kind,
                versions: Vec::new(),
                default: None,
            });
            registry.payloads.last_mut().expect("just pushed")
        }
    };
    match payload
        .versions
        .iter()
        .position(|v| v.version == spec.version)
    {
        Some(pos) => payload.versions[pos] = version_entry,
        None => payload.versions.push(version_entry),
    }
    if spec.set_default {
        payload.default = Some(spec.version.clone());
    }
    registry
        .validate()
        .map_err(|e| invalid(format!("the published registry would be invalid: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // ---- naming ----------------------------------------------------------

    #[test]
    fn naming_follows_the_spec_16_forms() {
        assert_eq!(release_tag("1.2.3"), "v1.2.3");
        assert_eq!(release_tag("v1.2.3"), "v1.2.3");
        assert_eq!(
            artifact_name("metanorma", "1.2.3", Some(Platform::Aarch64Macos)),
            "metanorma-1.2.3-macos-arm64.tfs"
        );
        assert_eq!(
            artifact_name("metanorma", "1.2.3", None),
            "metanorma-1.2.3.tfs"
        );
        assert_eq!(
            binary_asset_name("metanorma", "1.2.3", Platform::X86_64LinuxGnu),
            "metanorma-1.2.3-linux-gnu-x86_64"
        );
    }

    // ---- upsert_entry: the registry round-trip ---------------------------

    fn spec_for(version: &str, sha: &str) -> EntrySpec {
        let mut per_triplet = BTreeMap::new();
        per_triplet.insert(
            Platform::Aarch64Macos,
            PlatformArtifact {
                artifact: format!("app-{version}-macos-arm64.tfs"),
                sha256: sha.to_string(),
            },
        );
        per_triplet.insert(
            Platform::X86_64LinuxGnu,
            PlatformArtifact {
                artifact: format!("app-{version}-linux-gnu-x86_64.tfs"),
                sha256: sha.to_string(),
            },
        );
        EntrySpec {
            name: "app".to_string(),
            kind: PayloadKind::App,
            version: version.to_string(),
            universal: None,
            per_triplet,
            release_ref: format!("tfs:github:o/app:{}", release_tag(version)),
            signature: Some(SignaturePin {
                keyid: "0123456789abcdef".to_string(),
                asc: format!("app-{version}-macos-arm64.tfs.asc"),
            }),
            runtime_requirement: Some(RegistryRuntimeRequirement {
                engine: "ruby".to_string(),
                constraint: "~> 3.3.0".to_string(),
            }),
            entrypoints: vec!["app".to_string()],
            set_default: true,
        }
    }

    #[test]
    fn upsert_generates_entries_that_round_trip_losslessly() {
        let mut registry = Registry {
            schema_version: 1,
            payloads: vec![],
        };
        upsert_entry(&mut registry, &spec_for("1.0", &"a".repeat(64))).unwrap();
        upsert_entry(&mut registry, &spec_for("1.1", &"b".repeat(64))).unwrap();

        let payload = registry.payload("app").unwrap();
        assert_eq!(payload.versions.len(), 2);
        assert_eq!(payload.default.as_deref(), Some("1.1"));
        let v = payload.version("1.1").unwrap();
        assert_eq!(
            v.select(Platform::Aarch64Macos),
            Some(crate::registry::PlatformSelection::Selected {
                artifact: "app-1.1-macos-arm64.tfs",
                sha256: &"b".repeat(64),
            })
        );
        assert_eq!(v.release.r#ref, "tfs:github:o/app:v1.1");
        assert_eq!(
            v.signature.as_ref().unwrap().asc,
            "app-1.1-macos-arm64.tfs.asc"
        );

        // re-publish 1.0 with new digests: the entry is REPLACED in place,
        // the 1.1 entry and the rest of the model untouched
        upsert_entry(&mut registry, &spec_for("1.0", &"c".repeat(64))).unwrap();
        let payload = registry.payload("app").unwrap();
        assert_eq!(payload.versions.len(), 2);
        assert_eq!(
            payload.version("1.0").unwrap().published_triplets(),
            vec![Platform::Aarch64Macos, Platform::X86_64LinuxGnu]
        );
        assert_eq!(
            payload.version("1.1").unwrap().signature,
            spec_for("1.1", &"b".repeat(64)).signature
        );
        assert_eq!(payload.default.as_deref(), Some("1.0"));

        let yaml = registry.to_yaml().unwrap();
        let again = Registry::from_yaml(&yaml).unwrap();
        assert_eq!(registry, again);
    }

    #[test]
    fn upsert_universal_pins_the_release_ref() {
        let mut registry = Registry {
            schema_version: 1,
            payloads: vec![],
        };
        let mut spec = spec_for("2.0", &"d".repeat(64));
        spec.universal = Some(("app-2.0.tfs".to_string(), "d".repeat(64)));
        spec.per_triplet = BTreeMap::new();
        spec.set_default = false;
        upsert_entry(&mut registry, &spec).unwrap();
        let v = registry.payload("app").unwrap().version("2.0").unwrap();
        assert!(matches!(v.platforms, RegistryPlatforms::Universal));
        assert_eq!(
            v.release.r#ref,
            format!("tfs:github:o/app:v2.0?sha256={}", "d".repeat(64))
        );
        assert!(registry.payload("app").unwrap().default.is_none());

        let yaml = registry.to_yaml().unwrap();
        assert_eq!(Registry::from_yaml(&yaml).unwrap(), registry);
    }

    #[test]
    fn upsert_errors_are_named() {
        let mut registry = Registry {
            schema_version: 1,
            payloads: vec![],
        };
        let mut both = spec_for("1.0", &"a".repeat(64));
        both.universal = Some(("app-1.0.tfs".to_string(), "a".repeat(64)));
        let err = upsert_entry(&mut registry, &both).unwrap_err();
        assert!(err.to_string().contains("not both"), "{err}");

        upsert_entry(&mut registry, &spec_for("1.0", &"a".repeat(64))).unwrap();
        let mut wrong_kind = spec_for("1.1", &"b".repeat(64));
        wrong_kind.kind = PayloadKind::Data;
        wrong_kind.entrypoints = Vec::new();
        let err = upsert_entry(&mut registry, &wrong_kind).unwrap_err();
        assert!(err.to_string().contains("kind"), "{err}");
    }

    // ---- the releases client over a mock transport -----------------------

    #[derive(Default)]
    struct MockGithub {
        /// tag → (release id, assets: name → (asset id, bytes))
        releases: Mutex<BTreeMap<String, (u64, BTreeMap<String, (u64, Vec<u8>)>)>>,
        next_id: Mutex<u64>,
        deleted: Mutex<Vec<u64>>,
        /// contents path → bytes
        contents: Mutex<BTreeMap<String, Vec<u8>>>,
        put_bodies: Mutex<Vec<String>>,
        fail_status: Mutex<Option<u16>>,
    }

    impl MockGithub {
        fn new() -> Self {
            let m = MockGithub::default();
            *m.next_id.lock().unwrap() = 100;
            m
        }

        fn alloc(&self) -> u64 {
            let mut next = self.next_id.lock().unwrap();
            *next += 1;
            *next
        }

        fn release_json(&self, tag: &str) -> Option<(u16, Vec<u8>)> {
            let releases = self.releases.lock().unwrap();
            let (id, assets) = releases.get(tag)?;
            let assets_json = assets
                .iter()
                .map(|(name, (aid, _))| {
                    format!(
                        "{{\"id\":{aid},\"name\":\"{}\",\"browser_download_url\":\"https://dl/{name}\"}}",
                        json_escape(name)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            Some((
                200,
                format!("{{\"id\":{id},\"assets\":[{assets_json}]}}").into_bytes(),
            ))
        }
    }

    impl PublishTransport for MockGithub {
        fn request(
            &self,
            method: &str,
            url: &str,
            _token: &str,
            _content_type: Option<&str>,
            body: Option<&[u8]>,
        ) -> Result<(u16, Vec<u8>), ReleaseError> {
            if let Some(status) = *self.fail_status.lock().unwrap() {
                return Ok((status, Vec::new()));
            }
            let base = format!("{API}/repos/o/app");
            if method == "GET" && url.starts_with(&format!("{base}/releases/tags/")) {
                let tag = url.trim_start_matches(&format!("{base}/releases/tags/"));
                return Ok(self.release_json(tag).unwrap_or((404, Vec::new())));
            }
            if method == "POST" && url == format!("{base}/releases") {
                let doc = json_parse(std::str::from_utf8(body.unwrap()).unwrap()).unwrap();
                let tag = doc.find("tag_name").unwrap().as_string().unwrap();
                let id = self.alloc();
                self.releases
                    .lock()
                    .unwrap()
                    .insert(tag, (id, BTreeMap::new()));
                return Ok((
                    201,
                    format!("{{\"id\":{id},\"assets\":[]}}").into_bytes(),
                ));
            }
            if method == "DELETE" && url.starts_with(&format!("{base}/releases/assets/")) {
                let id: u64 = url
                    .trim_start_matches(&format!("{base}/releases/assets/"))
                    .parse()
                    .unwrap();
                self.deleted.lock().unwrap().push(id);
                let mut releases = self.releases.lock().unwrap();
                for (_, assets) in releases.values_mut() {
                    assets.retain(|_, (aid, _)| *aid != id);
                }
                return Ok((204, Vec::new()));
            }
            if method == "POST"
                && url.starts_with(&format!("{UPLOADS}/repos/o/app/releases/"))
            {
                let rest = url.trim_start_matches(&format!("{UPLOADS}/repos/o/app/releases/"));
                let (id_part, query) = rest.split_once('/').unwrap();
                let release_id: u64 = id_part.parse().unwrap();
                let name = query
                    .trim_start_matches("assets?name=")
                    .to_string();
                let asset_id = self.alloc();
                let mut releases = self.releases.lock().unwrap();
                let (_, assets) = releases
                    .values_mut()
                    .find(|(rid, _)| *rid == release_id)
                    .expect("release exists");
                assets.insert(name, (asset_id, body.unwrap().to_vec()));
                return Ok((201, b"{}".to_vec()));
            }
            if method == "GET" && url.starts_with(&format!("{base}/contents/")) {
                let path = url.trim_start_matches(&format!("{base}/contents/"));
                let contents = self.contents.lock().unwrap();
                return Ok(match contents.get(path) {
                    Some(bytes) => (
                        200,
                        format!("{{\"sha\":\"blobsha-{path}\",\"size\":{}}}", bytes.len())
                            .into_bytes(),
                    ),
                    None => (404, Vec::new()),
                });
            }
            if method == "PUT" && url.starts_with(&format!("{base}/contents/")) {
                let path = url.trim_start_matches(&format!("{base}/contents/"));
                let text = std::str::from_utf8(body.unwrap()).unwrap().to_string();
                self.put_bodies.lock().unwrap().push(text.clone());
                let doc = json_parse(&text).unwrap();
                let encoded = doc.find("content").unwrap().as_string().unwrap();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .unwrap();
                self.contents.lock().unwrap().insert(path.to_string(), bytes);
                return Ok((200, b"{}".to_vec()));
            }
            panic!("unexpected request {method} {url}");
        }
    }

    fn client<'a>(mock: &'a MockGithub) -> GithubReleaseClient<'a, MockGithub> {
        GithubReleaseClient {
            transport: mock,
            owner: "o",
            repo: "app",
            token: "Bearer test",
        }
    }

    #[test]
    fn publish_creates_the_release_and_uploads_every_asset() {
        let mock = MockGithub::new();
        let assets = vec![
            ("app-1.0.tfs".to_string(), b"payload".to_vec()),
            ("SHA256SUMS".to_string(), b"sums".to_vec()),
        ];
        let outcome = client(&mock).publish("v1.0", &assets).unwrap();
        assert_eq!(outcome.tag, "v1.0");
        assert_eq!(outcome.uploaded, vec!["app-1.0.tfs", "SHA256SUMS"]);
        assert!(outcome.replaced.is_empty());
        let releases = mock.releases.lock().unwrap();
        let (_, assets) = releases.get("v1.0").unwrap();
        assert_eq!(assets.get("app-1.0.tfs").unwrap().1, b"payload");
        assert_eq!(assets.get("SHA256SUMS").unwrap().1, b"sums");
    }

    #[test]
    fn republish_replaces_same_named_assets_idempotently() {
        let mock = MockGithub::new();
        let v1 = vec![("app-1.0.tfs".to_string(), b"old".to_vec())];
        client(&mock).publish("v1.0", &v1).unwrap();

        let v2 = vec![
            ("app-1.0.tfs".to_string(), b"new".to_vec()),
            ("app-1.0.tfs.asc".to_string(), b"asc".to_vec()),
        ];
        let outcome = client(&mock).publish("v1.0", &v2).unwrap();
        assert_eq!(outcome.replaced, vec!["app-1.0.tfs"]);
        assert_eq!(mock.deleted.lock().unwrap().len(), 1);

        let releases = mock.releases.lock().unwrap();
        let (_, assets) = releases.get("v1.0").unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets.get("app-1.0.tfs").unwrap().1, b"new");
        assert_eq!(assets.get("app-1.0.tfs.asc").unwrap().1, b"asc");
    }

    #[test]
    fn api_failures_are_named_errors() {
        let mock = MockGithub::new();
        *mock.fail_status.lock().unwrap() = Some(403);
        let err = client(&mock)
            .publish("v1.0", &[("a".to_string(), b"b".to_vec())])
            .unwrap_err();
        assert!(matches!(err, ReleaseError::Auth { .. }), "{err:?}");

        *mock.fail_status.lock().unwrap() = Some(500);
        let err = client(&mock)
            .publish("v1.0", &[("a".to_string(), b"b".to_vec())])
            .unwrap_err();
        assert!(matches!(err, ReleaseError::Api { status: 500, .. }), "{err:?}");
    }

    #[test]
    fn commit_file_creates_then_updates_with_the_blob_sha() {
        let mock = MockGithub::new();
        let outcome = commit_file(
            &mock,
            "o",
            "app",
            "Formula/app.rb",
            b"formula-v1",
            "app 1.0",
            "Bearer test",
        )
        .unwrap();
        assert_eq!(outcome, CommitOutcome::Created);
        let bodies = mock.put_bodies.lock().unwrap();
        assert!(!bodies[0].contains("\"sha\""), "create carries no sha");
        drop(bodies);

        let outcome = commit_file(
            &mock,
            "o",
            "app",
            "Formula/app.rb",
            b"formula-v2",
            "app 1.1",
            "Bearer test",
        )
        .unwrap();
        assert_eq!(outcome, CommitOutcome::Updated);
        let bodies = mock.put_bodies.lock().unwrap();
        assert!(
            bodies[1].contains("\"sha\":\"blobsha-Formula/app.rb\""),
            "update carries the blob sha: {}",
            bodies[1]
        );
        assert_eq!(
            mock.contents.lock().unwrap().get("Formula/app.rb").unwrap(),
            b"formula-v2"
        );
    }
}
