//! The OCI distribution client (spec 38): pull-only, no async, pure ureq
//! via tebako-http behind the [`Http`] seam. A pull is: resolve the
//! manifest (the §3 shape law — empty config, exactly one layer whose
//! digest IS the artifact's sha256), then fetch or stream the one layer.
//! Authentication is the distribution-spec Bearer challenge dance
//! (§6): anonymous first, a 401's `WWW-Authenticate` parsed for the
//! realm, a token minted there (Basic credentials from the injected
//! [`CredentialSource`] — the book's tiers 1/2, then the docker config's
//! static `auths` as tier 3), the request retried with the Bearer token.
//! A cached token that draws a 401 is evicted and the dance re-runs
//! ONCE; a second refusal is the named `CredentialRequired`. A token
//! realm on a FOREIGN host is only ever consulted with that host's own
//! tier-2 credential — absent one, the named `OciCrossHostAuthRefused`
//! (never a silent credential leak across hosts, never an anonymous
//! cross-host ride).
//!
//! The transport policy (§8) is construction-level: registry URLs are
//! built `https://` unless the host is the tebako-http loopback
//! carve-out, and a token realm is validated https-or-loopback before
//! any request rides it. There is no insecure-registry spelling.

pub mod auth;
pub mod dockerconfig;
pub mod model;
pub mod push;

use std::io::Write;

use sha2::{Digest, Sha256};
use tebako_http::FetchError;
use tebako_json::{parse as json_parse, Value as JsonValue};

pub use model::{
    payload_tag, signature_tag, Annotations, ArtifactClass, Descriptor, Manifest, ShapeExpectation,
    EMPTY_CONFIG_DIGEST, EMPTY_CONFIG_MT, MANIFEST_MT,
};

/// The credential presentation on one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth<'a> {
    /// No `Authorization` header.
    Anonymous,
    /// `Authorization: Bearer <token>`.
    Bearer(&'a str),
    /// `Authorization: Basic <base64(user:pass)>` — `&str` is the
    /// `user:pass` pair.
    Basic(&'a str),
}

/// The `Authorization` header [`Auth`] spells, if any. The one
/// construction site — transports attach it verbatim.
pub fn authorization_header(auth: Auth<'_>) -> Option<(&'static str, String)> {
    match auth {
        Auth::Anonymous => None,
        Auth::Bearer(token) => Some(("Authorization", format!("Bearer {token}"))),
        Auth::Basic(user_pass) => Some((
            "Authorization",
            format!("Basic {}", base64_encode(user_pass.as_bytes())),
        )),
    }
}

/// Standard base64 encode (the Basic credential's pair). Strict alphabet,
/// canonical padding — the mirror of [`dockerconfig`]'s decoder.
pub fn base64_encode(bytes: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = |i: usize| chunk.get(i).copied().unwrap_or(0) as u32;
        let acc = (b(0) << 16) | (b(1) << 8) | b(2);
        out.push(ALPHA[(acc >> 18) as usize & 63] as char);
        out.push(ALPHA[(acc >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHA[(acc >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHA[acc as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// The transport seam: what the client needs from HTTP. tebako-resolve
/// implements it over its `Transport` trait (the real one delegating to
/// tebako-http's distribution surface); tests drive a script.
pub trait Http {
    /// A buffered, UNCLASSIFIED GET: any status returns with headers and
    /// body so the caller can read a 401's challenge and a distribution
    /// error body's `code`.
    fn get(
        &self,
        url: &str,
        accept: Option<&str>,
        auth: Auth<'_>,
    ) -> Result<tebako_http::RawResponse, FetchError>;
    /// A buffered, UNCLASSIFIED POST for the publish path (spec 38 §7):
    /// the blob-mount/upload-session open. Any status returns like
    /// [`Http::get`] — the push flow reads 201 (mounted) / 202 (upload
    /// session opened, the Location header its handle) / 401 (the dance
    /// cue) itself.
    fn post(
        &self,
        url: &str,
        body: &[u8],
        content_type: Option<&str>,
        auth: Auth<'_>,
    ) -> Result<tebako_http::RawResponse, FetchError>;
    /// A buffered, UNCLASSIFIED PUT for the publish path (spec 38 §7):
    /// the monolithic blob upload and the manifest placement. Any status
    /// returns like [`Http::get`].
    fn put(
        &self,
        url: &str,
        body: &[u8],
        content_type: &str,
        auth: Auth<'_>,
    ) -> Result<tebako_http::RawResponse, FetchError>;
    /// A classified streaming GET (a 401 is [`FetchError::AuthRejected`]
    /// — the re-challenge cue): bytes to `writer`, progress to
    /// `on_progress` (return `false` to abort).
    fn stream(
        &self,
        url: &str,
        auth: Auth<'_>,
        writer: &mut dyn Write,
        on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
    ) -> Result<u64, FetchError>;
}

/// A `user:pass` pair plus its credential CLASS (the token cache's
/// fourth key, §6: `anonymous`, `token:env:<VAR>`,
/// `token:docker-config:<host>` — different classes never share a token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicCred {
    pub user_pass: String,
    pub class: String,
}

/// The credential chain the client consults (the adapter implements it
/// over the tebako credential book + the docker config).
pub trait CredentialSource {
    /// The registry host's own credential: tier 1 confined to this host,
    /// tier 2 exact-host, tier 3 the docker config's static entry. `None`
    /// = anonymous.
    fn registry_credential(&self) -> Result<Option<BasicCred>, OciError>;
    /// A FOREIGN token realm's credential: the realm host's own tier-2
    /// entry only. `None` = no entry — the client refuses the cross-host
    /// ride by name (`OciCrossHostAuthRefused`).
    fn realm_credential(&self, realm_host: &str) -> Result<Option<BasicCred>, OciError>;
}

/// The client's failure classes — every one maps to a spec 38 §9 named
/// error in the tebako-resolve adapter.
#[derive(Debug)]
pub enum OciError {
    /// A transport-layer failure (connect, TLS, classified HTTP).
    Transport(FetchError),
    /// A token realm spelled neither https nor loopback http (§8).
    InsecureTransport { url: String },
    /// The named manifest/tag/repository is absent (NAME_UNKNOWN,
    /// MANIFEST_UNKNOWN, TAG_INVALID, or a bare 404 on the manifest
    /// endpoint).
    ManifestNotFound { origin: String },
    /// The named blob is absent (BLOB_UNKNOWN, or a bare 404 on the
    /// blob endpoint).
    BlobUnknown { origin: String },
    /// The manifest violates the §3 shape law (not parseable as an OCI
    /// image manifest, a foreign `artifactType`, ≠1 layer, a
    /// non-canonical config, a missing required annotation).
    ArtifactMalformed { origin: String, reason: String },
    /// The Bearer challenge or the token endpoint's answer is unusable:
    /// no Bearer challenge (a Basic-only registry), a malformed
    /// challenge, a token response without a token.
    TokenChallengeInvalid { reason: String },
    /// The token realm lives on a foreign host and the credential chain
    /// holds no tier-2 entry for THAT host (§6 — the deliberate sharp
    /// edge: even an anonymous cross-host ride is refused).
    CrossHostAuthRefused {
        registry_host: String,
        realm_host: String,
    },
    /// The registry or token endpoint refused the presented (or absent)
    /// credential: a terminal 401 after the re-challenge, or the token
    /// endpoint's own 401/403.
    CredentialRequired {
        host: String,
        looked_for: Option<String>,
    },
    /// The docker config consult failed by name (malformed config, or a
    /// credential helper covering the host).
    DockerConfig(dockerconfig::DockerConfigError),
    /// The bytes do not match the digest that named them (the manifest
    /// body vs the `@sha256:` selector or the `Docker-Content-Digest`
    /// header; the blob bytes vs the layer descriptor).
    Sha256Mismatch {
        origin: String,
        expected: String,
        actual: String,
    },
    /// Publish (spec 38 §7's write-once): the tag already names a
    /// DIFFERENT manifest digest. Re-publish of identical bytes is the
    /// idempotent skip (never this error); moving a tag is refused.
    /// Exit class 69 (`OciTagConflict`).
    TagConflict {
        origin: String,
        tag: String,
        existing: String,
        attempted: String,
    },
}

impl std::fmt::Display for OciError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OciError::Transport(e) => write!(f, "{e}"),
            OciError::InsecureTransport { url } => write!(
                f,
                "OciInsecureTransport: the token realm {url} is neither https:// nor a loopback http:// URL — plain-HTTP registries do not exist (spec 38 §8)"
            ),
            OciError::ManifestNotFound { origin } => write!(
                f,
                "OciManifestNotFound: {origin} names no manifest on the registry (repository, tag, or digest unknown)"
            ),
            OciError::BlobUnknown { origin } => write!(
                f,
                "OciBlobUnknown: the registry does not hold the blob {origin} names"
            ),
            OciError::ArtifactMalformed { origin, reason } => write!(
                f,
                "OciArtifactMalformed: {origin}: {reason}"
            ),
            OciError::TokenChallengeInvalid { reason } => write!(
                f,
                "OciTokenChallengeInvalid: {reason}"
            ),
            OciError::CrossHostAuthRefused {
                registry_host,
                realm_host,
            } => write!(
                f,
                "OciCrossHostAuthRefused: {registry_host}'s token realm lives on the foreign host {realm_host}, and no tier-2 `credentials:` entry covers {realm_host} — credentials never leak across hosts; add an entry for {realm_host} (even a registry whose anonymous tokens ride a foreign auth host, like docker.io's auth.docker.io, needs one)"
            ),
            OciError::CredentialRequired { host, looked_for } => {
                write!(
                    f,
                    "CredentialRequired: {host} refused the presented (or absent) credential"
                )?;
                if let Some(class) = looked_for {
                    write!(f, " (presented class: {class})")?;
                }
                Ok(())
            }
            OciError::DockerConfig(e) => write!(f, "{e}"),
            OciError::Sha256Mismatch {
                origin,
                expected,
                actual,
            } => write!(
                f,
                "sha256 mismatch for {origin}: expected {expected}, got {actual}"
            ),
            OciError::TagConflict {
                origin,
                tag,
                existing,
                attempted,
            } => write!(
                f,
                "OciTagConflict: {origin} tag '{tag}' already points at {existing} — pushing {attempted} would move the tag (write-once, spec 38 §7); re-publish of identical bytes is an idempotent skip, otherwise publish a new version or a new tag"
            ),
        }
    }
}

impl std::error::Error for OciError {}

impl From<FetchError> for OciError {
    fn from(e: FetchError) -> Self {
        OciError::Transport(e)
    }
}

impl From<dockerconfig::DockerConfigError> for OciError {
    fn from(e: dockerconfig::DockerConfigError) -> Self {
        OciError::DockerConfig(e)
    }
}

/// How a reference selects its manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selector<'a> {
    /// `:tag`.
    Tag(&'a str),
    /// `@sha256:<64 hex>` (the hex half only).
    Digest(&'a str),
    /// No selector spelled — the `latest` tag.
    Default,
}

/// A repository on a registry host plus its selector — the client's
/// view of a parsed `tfs+oci://` reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepoRef<'a> {
    pub host: &'a str,
    pub repo: &'a str,
    pub selector: Selector<'a>,
}

impl RepoRef<'_> {
    /// The `tfs+oci://` spelling of this reference (error contexts).
    pub fn reference_string(&self) -> String {
        match self.selector {
            Selector::Tag(tag) => format!("tfs+oci://{}/{}:{}", self.host, self.repo, tag),
            Selector::Digest(hex) => {
                format!("tfs+oci://{}/{}@sha256:{}", self.host, self.repo, hex)
            }
            Selector::Default => format!("tfs+oci://{}/{}", self.host, self.repo),
        }
    }

    /// The manifest name in the URL: the tag, the `sha256:<hex>`
    /// digest, or `latest`.
    fn manifest_name(&self) -> String {
        match self.selector {
            Selector::Tag(tag) => tag.to_string(),
            Selector::Digest(hex) => format!("sha256:{hex}"),
            Selector::Default => "latest".to_string(),
        }
    }
}

/// The pull scope the pull path's requests dance for.
fn scope_for(repo: &str) -> String {
    format!("repository:{repo}:pull")
}

/// The publish path's scope (spec 38 §7): writes dance for pull,push —
/// the mount probe reads (cross-repo mount source), the upload writes.
pub(crate) fn push_scope_for(repo: &str) -> String {
    format!("repository:{repo}:pull,push")
}

/// `http` iff the host is the tebako-http loopback carve-out (the test
/// fixture, local development), else `https` — there is no
/// insecure-registry spelling (§8).
fn scheme_for(host: &str) -> &'static str {
    if tebako_http::is_loopback_host(host) {
        "http"
    } else {
        "https"
    }
}

/// The `<host>` half of a URL (the authority, up to the first `/`).
fn url_host(url: &str) -> &str {
    let rest = url.split("://").nth(1).unwrap_or(url);
    rest.split('/').next().unwrap_or(rest)
}

/// RFC 3986 unreserved-only percent-encoding (query parameter values).
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// A resolved manifest: the artifact class, the manifest's own digest
/// (the origin's trust anchor), the ONE layer (whose digest is the
/// artifact's sha256), and the annotation mirror.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub class: ArtifactClass,
    /// The 64-hex sha256 of the manifest bytes as served.
    pub manifest_digest: String,
    pub layer: Descriptor,
    pub annotations: Annotations,
    /// `tfs+oci://<host>/<repo>@sha256:<manifest-digest>` — the
    /// digest-pinned spelling (the store origin).
    pub origin: String,
}

/// The client: stateless but for the injected seams (the per-process
/// token cache lives in [`auth`]).
pub struct Client<'a, H: Http, C: CredentialSource> {
    http: &'a H,
    creds: &'a C,
}

/// Which endpoint an error body came from (the bare-404 fallback maps
/// by endpoint kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endpoint {
    Manifest,
    Blob,
}

impl<'a, H: Http, C: CredentialSource> Client<'a, H, C> {
    pub fn new(http: &'a H, creds: &'a C) -> Self {
        Client { http, creds }
    }

    /// The registry credential's class (the preemptive-token cache key).
    fn registry_class(&self) -> Result<String, OciError> {
        Ok(self
            .creds
            .registry_credential()?
            .map(|c| c.class)
            .unwrap_or_else(|| "anonymous".to_string()))
    }

    fn manifest_url(&self, r: &RepoRef<'_>) -> String {
        format!(
            "{}://{}/v2/{}/manifests/{}",
            scheme_for(r.host),
            r.host,
            r.repo,
            r.manifest_name()
        )
    }

    fn blob_url(&self, r: &RepoRef<'_>, layer: &Descriptor) -> Result<String, OciError> {
        let hex = layer
            .digest_hex()
            .ok_or_else(|| OciError::ArtifactMalformed {
                origin: r.reference_string(),
                reason: format!("the layer digest '{}' is not a sha256 digest", layer.digest),
            })?;
        Ok(format!(
            "{}://{}/v2/{}/blobs/sha256:{}",
            scheme_for(r.host),
            r.host,
            r.repo,
            hex
        ))
    }

    /// The §6 dance: parse the 401's Bearer challenge, answer it with a
    /// token (cache first, then the realm). `default_scope` is the scope
    /// requested when the challenge carries none (pull on the read path,
    /// pull,push on the publish path); a challenge's own scope is honored
    /// verbatim — the registry knows the action the rejected call needed.
    fn dance(
        &self,
        r: &RepoRef<'_>,
        www_authenticate: Option<&str>,
        default_scope: &str,
    ) -> Result<String, OciError> {
        let header = www_authenticate.ok_or_else(|| OciError::TokenChallengeInvalid {
            reason: "the 401 carries no WWW-Authenticate header".to_string(),
        })?;
        let challenge = auth::parse_challenge(header)
            .map_err(|reason| OciError::TokenChallengeInvalid { reason })?
            .ok_or_else(|| OciError::TokenChallengeInvalid {
                reason: format!(
                    "the 401's challenge carries no Bearer scheme ('{header}') — a Basic-only registry is not the distribution spec's shape"
                ),
            })?;
        let scope = challenge
            .scope
            .clone()
            .unwrap_or_else(|| default_scope.to_string());
        // The realm's transport and host discipline (§6/§8): https or
        // loopback only, and a FOREIGN realm host answers only to its
        // own tier-2 credential — absent one, the named refusal (even
        // for what would be an anonymous ride).
        if !(challenge.realm.starts_with("https://")
            || tebako_http::is_loopback_http_url(&challenge.realm))
        {
            return Err(OciError::InsecureTransport {
                url: challenge.realm.clone(),
            });
        }
        let realm_host = url_host(&challenge.realm);
        let cred = if realm_host.eq_ignore_ascii_case(r.host) {
            self.creds.registry_credential()?
        } else {
            match self.creds.realm_credential(realm_host)? {
                Some(cred) => Some(cred),
                None => {
                    return Err(OciError::CrossHostAuthRefused {
                        registry_host: r.host.to_string(),
                        realm_host: realm_host.to_string(),
                    })
                }
            }
        };
        let class = cred
            .as_ref()
            .map(|c| c.class.clone())
            .unwrap_or_else(|| "anonymous".to_string());
        if let Some(token) = auth::cached_token(&challenge, Some(&scope), &class) {
            return Ok(token);
        }
        // Mint: GET the realm with service/scope, Basic when a
        // credential answered.
        let sep = if challenge.realm.contains('?') {
            '&'
        } else {
            '?'
        };
        let mut url = challenge.realm.clone();
        if let Some(service) = &challenge.service {
            url.push_str(&format!("{sep}service={}", urlencode(service)));
        }
        url.push_str(&format!(
            "{}scope={}",
            if url.contains('?') { '&' } else { '?' },
            urlencode(&scope)
        ));
        let presentation = match &cred {
            Some(cred) => Auth::Basic(&cred.user_pass),
            None => Auth::Anonymous,
        };
        let resp = self.http.get(&url, None, presentation)?;
        match resp.status {
            200 => {}
            401 | 403 => {
                return Err(OciError::CredentialRequired {
                    host: realm_host.to_string(),
                    looked_for: cred.map(|c| c.class),
                })
            }
            status => {
                return Err(OciError::TokenChallengeInvalid {
                    reason: format!("the token endpoint {url} answered HTTP {status}"),
                })
            }
        }
        let token = std::str::from_utf8(&resp.body)
            .ok()
            .and_then(|text| json_parse(text).ok())
            .and_then(|doc| {
                doc.find("token")
                    .or_else(|| doc.find("access_token"))
                    .and_then(|t| t.as_string())
            })
            .filter(|t| !t.is_empty())
            .ok_or_else(|| OciError::TokenChallengeInvalid {
                reason: format!("the token endpoint {url} answered 200 with no token"),
            })?;
        auth::store_token(r.host, &challenge, Some(&scope), &class, &token);
        Ok(token)
    }

    /// One request with the full dance discipline, any verb: a
    /// preemptive cached token when one exists, the dance on a 401, ONE
    /// re-challenge (evict + re-dance) when a token draws a 401, and the
    /// terminal 401 mapped to `CredentialRequired` by the caller (which
    /// knows the endpoint kind). `scope` is the token scope this call
    /// dances for (pull on the read path, pull,push on publish). Returns
    /// the response plus the credential class in play.
    pub(crate) fn send_with_dance(
        &self,
        r: &RepoRef<'_>,
        scope: &str,
        send: &dyn Fn(Auth<'_>) -> Result<tebako_http::RawResponse, FetchError>,
    ) -> Result<(tebako_http::RawResponse, String), OciError> {
        let class = self.registry_class()?;
        let mut token = auth::cached_token_for_host(r.host, scope, &class);
        let mut dances = 0u8;
        loop {
            let presentation = match &token {
                Some(token) => Auth::Bearer(token),
                None => Auth::Anonymous,
            };
            let resp = send(presentation)?;
            if resp.status != 401 {
                return Ok((resp, class));
            }
            match dances {
                0 | 1 => {
                    if token.is_some() {
                        auth::evict_token_for_host(r.host, scope, &class);
                    }
                    token = Some(self.dance(r, resp.header("www-authenticate"), scope)?);
                    dances += 1;
                }
                _ => {
                    return Err(OciError::CredentialRequired {
                        host: r.host.to_string(),
                        looked_for: Some(class),
                    })
                }
            }
        }
    }

    /// A buffered GET with the full dance discipline — the pull path's
    /// form of [`Client::send_with_dance`] at the pull scope.
    fn get_with_dance(
        &self,
        url: &str,
        accept: Option<&str>,
        r: &RepoRef<'_>,
    ) -> Result<(tebako_http::RawResponse, String), OciError> {
        self.send_with_dance(r, &scope_for(r.repo), &|auth| {
            self.http.get(url, accept, auth)
        })
    }

    /// The distribution error-body map (§5): the `code` field names the
    /// failure; a bare status falls back by endpoint kind; 429 rides the
    /// transport's throttling schedule.
    fn classify_error(
        &self,
        url: &str,
        endpoint: Endpoint,
        r: &RepoRef<'_>,
        resp: &tebako_http::RawResponse,
    ) -> OciError {
        let origin = r.reference_string();
        let code = std::str::from_utf8(&resp.body)
            .ok()
            .and_then(|text| json_parse(text).ok())
            .and_then(|doc| match doc.find("errors") {
                Some(JsonValue::Array(errors)) => errors
                    .iter()
                    .find_map(|e| e.find("code").and_then(|c| c.as_string())),
                _ => None,
            });
        match code.as_deref() {
            Some("NAME_UNKNOWN") | Some("MANIFEST_UNKNOWN") | Some("TAG_INVALID") => {
                return OciError::ManifestNotFound { origin }
            }
            Some("BLOB_UNKNOWN") => return OciError::BlobUnknown { origin },
            Some("UNAUTHORIZED") | Some("DENIED") => {
                return OciError::CredentialRequired {
                    host: r.host.to_string(),
                    looked_for: None,
                }
            }
            _ => {}
        }
        match resp.status {
            404 => match endpoint {
                Endpoint::Manifest => OciError::ManifestNotFound { origin },
                Endpoint::Blob => OciError::BlobUnknown { origin },
            },
            429 => OciError::Transport(FetchError::Throttled {
                url: url.to_string(),
                status: 429,
                retry_after: resp
                    .header("retry-after")
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .map(std::time::Duration::from_secs),
            }),
            status => OciError::Transport(FetchError::DownloadFailed(format!(
                "{url} answered HTTP {status}"
            ))),
        }
    }

    /// Resolve the manifest: fetch (with the dance), verify the bytes
    /// against the digest that named them (the `@sha256:` selector and
    /// the `Docker-Content-Digest` header), then apply the §3 shape
    /// law. The ONE layer's descriptor comes back inside the
    /// [`Artifact`].
    pub fn resolve(&self, r: &RepoRef<'_>, expect: ShapeExpectation) -> Result<Artifact, OciError> {
        let url = self.manifest_url(r);
        let (resp, class) = self.get_with_dance(&url, Some(MANIFEST_MT), r)?;
        if resp.status == 401 {
            return Err(OciError::CredentialRequired {
                host: r.host.to_string(),
                looked_for: Some(class),
            });
        }
        if resp.status != 200 {
            return Err(self.classify_error(&url, Endpoint::Manifest, r, &resp));
        }
        // Step 1 of the §5 verification: the manifest bytes hash to the
        // digest that named them — the selector pin first, then the
        // registry's own Docker-Content-Digest when served.
        let actual = format!("{:x}", Sha256::digest(&resp.body));
        if let Selector::Digest(pin) = r.selector {
            if !actual.eq_ignore_ascii_case(pin) {
                return Err(OciError::Sha256Mismatch {
                    origin: r.reference_string(),
                    expected: pin.to_string(),
                    actual,
                });
            }
        }
        if let Some(served) = resp.header("docker-content-digest") {
            let served_hex = served.strip_prefix("sha256:").unwrap_or(served);
            if !actual.eq_ignore_ascii_case(served_hex) {
                return Err(OciError::Sha256Mismatch {
                    origin: r.reference_string(),
                    expected: served.to_string(),
                    actual,
                });
            }
        }
        let manifest =
            Manifest::parse(&resp.body).map_err(|reason| OciError::ArtifactMalformed {
                origin: r.reference_string(),
                reason,
            })?;
        let layer = manifest
            .validate_shape(expect)
            .map_err(|reason| OciError::ArtifactMalformed {
                origin: r.reference_string(),
                reason,
            })?
            .clone();
        let class_of = match manifest
            .artifact_type
            .as_deref()
            .and_then(ArtifactClass::from_artifact_type)
        {
            Some(class) => class,
            None => {
                return Err(OciError::ArtifactMalformed {
                    origin: r.reference_string(),
                    reason: "the manifest carries no tebako artifactType".to_string(),
                })
            }
        };
        Ok(Artifact {
            class: class_of,
            origin: format!("tfs+oci://{}/{}@sha256:{}", r.host, r.repo, actual),
            manifest_digest: actual,
            layer,
            annotations: manifest.annotations,
        })
    }

    /// Verify streamed/buffered blob bytes against the layer descriptor
    /// (the descriptor's digest IS the artifact's sha256, §3).
    fn verify_blob(
        &self,
        r: &RepoRef<'_>,
        layer: &Descriptor,
        actual_hex: &str,
    ) -> Result<(), OciError> {
        let expected = layer
            .digest_hex()
            .ok_or_else(|| OciError::ArtifactMalformed {
                origin: r.reference_string(),
                reason: format!("the layer digest '{}' is not a sha256 digest", layer.digest),
            })?;
        if !actual_hex.eq_ignore_ascii_case(expected) {
            return Err(OciError::Sha256Mismatch {
                origin: r.reference_string(),
                expected: layer.digest.clone(),
                actual: actual_hex.to_string(),
            });
        }
        Ok(())
    }

    /// Fetch the artifact's one layer buffered (the small files: a
    /// registry index, a detached signature). The bytes verify against
    /// the layer digest before they return.
    pub fn fetch_blob(&self, r: &RepoRef<'_>, layer: &Descriptor) -> Result<Vec<u8>, OciError> {
        let url = self.blob_url(r, layer)?;
        let (resp, class) = self.get_with_dance(&url, None, r)?;
        if resp.status == 401 {
            return Err(OciError::CredentialRequired {
                host: r.host.to_string(),
                looked_for: Some(class),
            });
        }
        if resp.status != 200 {
            return Err(self.classify_error(&url, Endpoint::Blob, r, &resp));
        }
        self.verify_blob(r, layer, &format!("{:x}", Sha256::digest(&resp.body)))?;
        Ok(resp.body)
    }

    /// Stream the artifact's one layer to `writer`, hashing as it goes;
    /// the byte stream verifies against the layer digest. The blob GET
    /// preemptively attaches a cached token (no probe 401); a refusal
    /// harvests the challenge with a raw GET, dances, and retries —
    /// one re-challenge, then the named `CredentialRequired`.
    pub fn stream_blob(
        &self,
        r: &RepoRef<'_>,
        layer: &Descriptor,
        writer: &mut dyn Write,
        on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
    ) -> Result<u64, OciError> {
        let url = self.blob_url(r, layer)?;
        let scope = scope_for(r.repo);
        let class = self.registry_class()?;
        let mut token = auth::cached_token_for_host(r.host, &scope, &class);
        let mut dances = 0u8;
        // One unified progress binding, reborrowed per attempt (the
        // retry loop cannot reborrow through the Option).
        let mut noop = |_: u64, _: Option<u64>| true;
        let progress: &mut dyn FnMut(u64, Option<u64>) -> bool = match on_progress {
            Some(p) => p,
            None => &mut noop,
        };
        loop {
            let presentation = match &token {
                Some(token) => Auth::Bearer(token),
                None => Auth::Anonymous,
            };
            let mut hasher = Sha256::new();
            let mut hashing = HashingWriter {
                inner: &mut *writer,
                hasher: &mut hasher,
            };
            match self
                .http
                .stream(&url, presentation, &mut hashing, Some(&mut *progress))
            {
                Ok(n) => {
                    let _ = hashing; // end the &mut hasher borrow
                    self.verify_blob(r, layer, &format!("{:x}", hasher.finalize()))?;
                    return Ok(n);
                }
                Err(FetchError::AuthRejected { .. }) => match dances {
                    0 | 1 => {
                        if token.is_some() {
                            auth::evict_token_for_host(r.host, &scope, &class);
                        }
                        // Harvest the challenge with a raw (unclassified)
                        // GET: the 401's body is the small error JSON,
                        // never the blob.
                        let probe = self.http.get(&url, None, Auth::Anonymous)?;
                        token = Some(self.dance(r, probe.header("www-authenticate"), &scope)?);
                        dances += 1;
                    }
                    _ => {
                        return Err(OciError::CredentialRequired {
                            host: r.host.to_string(),
                            looked_for: Some(class),
                        })
                    }
                },
                Err(e) => return Err(OciError::Transport(e)),
            }
        }
    }
}

/// The stream's hashing wrapper: every byte the transport writes also
/// feeds the artifact-digest verification.
struct HashingWriter<'a> {
    inner: &'a mut dyn Write,
    hasher: &'a mut Sha256,
}

impl Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    // -------------------------------------------------------------
    // The scripted seams
    // -------------------------------------------------------------

    pub(crate) struct ScriptedGet {
        pub(crate) url_part: &'static str,
        pub(crate) status: u16,
        pub(crate) headers: Vec<(String, String)>,
        pub(crate) body: Vec<u8>,
    }

    enum ScriptedStream {
        AuthRejected,
        Bytes(Vec<u8>),
    }

    /// One scripted write answer plus the received body (the publish
    /// tests assert the manifest PUT's bytes).
    pub(crate) struct ScriptedWrite {
        pub(crate) url_part: &'static str,
        pub(crate) status: u16,
        pub(crate) headers: Vec<(String, String)>,
        pub(crate) body: Vec<u8>,
    }

    pub(crate) struct MockHttp {
        gets: Mutex<VecDeque<ScriptedGet>>,
        streams: Mutex<VecDeque<ScriptedStream>>,
        posts: Mutex<VecDeque<ScriptedWrite>>,
        puts: Mutex<VecDeque<ScriptedWrite>>,
        /// (url, authorization-header-or-"anonymous")
        log: Mutex<Vec<(String, String)>>,
        /// (url, request body) of every POST/PUT, in order.
        write_log: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl MockHttp {
        pub(crate) fn new() -> Self {
            MockHttp {
                gets: Mutex::new(VecDeque::new()),
                streams: Mutex::new(VecDeque::new()),
                posts: Mutex::new(VecDeque::new()),
                puts: Mutex::new(VecDeque::new()),
                log: Mutex::new(Vec::new()),
                write_log: Mutex::new(Vec::new()),
            }
        }

        pub(crate) fn push_get(
            &self,
            url_part: &'static str,
            status: u16,
            headers: &[(&str, String)],
            body: &[u8],
        ) {
            self.gets.lock().unwrap().push_back(ScriptedGet {
                url_part,
                status,
                headers: headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                body: body.to_vec(),
            });
        }

        pub(crate) fn push_post(
            &self,
            url_part: &'static str,
            status: u16,
            headers: &[(&str, String)],
            body: &[u8],
        ) {
            self.posts.lock().unwrap().push_back(ScriptedWrite {
                url_part,
                status,
                headers: headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                body: body.to_vec(),
            });
        }

        pub(crate) fn push_put(
            &self,
            url_part: &'static str,
            status: u16,
            headers: &[(&str, String)],
            body: &[u8],
        ) {
            self.puts.lock().unwrap().push_back(ScriptedWrite {
                url_part,
                status,
                headers: headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                body: body.to_vec(),
            });
        }

        fn record(&self, url: &str, auth: Auth<'_>) {
            let presented = match authorization_header(auth) {
                Some((_, v)) => v,
                None => "anonymous".to_string(),
            };
            self.log.lock().unwrap().push((url.to_string(), presented));
        }

        fn record_write(&self, url: &str, body: &[u8]) {
            self.write_log
                .lock()
                .unwrap()
                .push((url.to_string(), body.to_vec()));
        }

        pub(crate) fn requests(&self) -> Vec<(String, String)> {
            self.log.lock().unwrap().clone()
        }

        pub(crate) fn writes(&self) -> Vec<(String, Vec<u8>)> {
            self.write_log.lock().unwrap().clone()
        }

        fn write(
            &self,
            queue: &Mutex<VecDeque<ScriptedWrite>>,
            verb: &str,
            url: &str,
            body: &[u8],
            auth: Auth<'_>,
        ) -> Result<tebako_http::RawResponse, FetchError> {
            self.record(url, auth);
            self.record_write(url, body);
            let next = queue
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| panic!("no scripted {verb} left for {url}"));
            assert!(
                url.contains(next.url_part),
                "scripted {verb} for '{}' got '{url}'",
                next.url_part
            );
            Ok(tebako_http::RawResponse::new(
                next.status,
                next.headers,
                next.body,
            ))
        }
    }

    impl Http for MockHttp {
        fn get(
            &self,
            url: &str,
            _accept: Option<&str>,
            auth: Auth<'_>,
        ) -> Result<tebako_http::RawResponse, FetchError> {
            self.record(url, auth);
            let next = self
                .gets
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| panic!("no scripted GET left for {url}"));
            assert!(
                url.contains(next.url_part),
                "scripted GET for '{}' got '{url}'",
                next.url_part
            );
            Ok(tebako_http::RawResponse::new(
                next.status,
                next.headers,
                next.body,
            ))
        }

        fn post(
            &self,
            url: &str,
            body: &[u8],
            _content_type: Option<&str>,
            auth: Auth<'_>,
        ) -> Result<tebako_http::RawResponse, FetchError> {
            self.write(&self.posts, "POST", url, body, auth)
        }

        fn put(
            &self,
            url: &str,
            body: &[u8],
            _content_type: &str,
            auth: Auth<'_>,
        ) -> Result<tebako_http::RawResponse, FetchError> {
            self.write(&self.puts, "PUT", url, body, auth)
        }

        fn stream(
            &self,
            url: &str,
            auth: Auth<'_>,
            writer: &mut dyn Write,
            mut on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
        ) -> Result<u64, FetchError> {
            self.record(url, auth);
            let next = self
                .streams
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| panic!("no scripted stream left for {url}"));
            match next {
                ScriptedStream::AuthRejected => Err(FetchError::AuthRejected {
                    url: url.to_string(),
                    status: 401,
                }),
                ScriptedStream::Bytes(bytes) => {
                    writer.write_all(&bytes).unwrap();
                    if let Some(progress) = on_progress.as_mut() {
                        assert!(progress(bytes.len() as u64, Some(bytes.len() as u64)));
                    }
                    Ok(bytes.len() as u64)
                }
            }
        }
    }

    pub(crate) struct MockCreds {
        registry: Option<BasicCred>,
        realms: HashMap<String, BasicCred>,
    }

    impl MockCreds {
        pub(crate) fn anonymous() -> Self {
            MockCreds {
                registry: None,
                realms: HashMap::new(),
            }
        }
    }

    impl CredentialSource for MockCreds {
        fn registry_credential(&self) -> Result<Option<BasicCred>, OciError> {
            Ok(self.registry.clone())
        }

        fn realm_credential(&self, realm_host: &str) -> Result<Option<BasicCred>, OciError> {
            Ok(self.realms.get(realm_host).cloned())
        }
    }

    // -------------------------------------------------------------
    // Fixtures
    // -------------------------------------------------------------

    /// Serialize the dance-asserting tests on the process-global token
    /// cache (and start each cold).
    pub(crate) fn token_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = auth::TEST_TOKEN_LOCK.lock().unwrap();
        auth::clear_tokens();
        guard
    }

    const HOST: &str = "registry.example";
    const REPO: &str = "tebako-packages/tool";
    const BLOB: &[u8] = b"the payload bytes";

    fn blob_hex() -> String {
        format!("{:x}", Sha256::digest(BLOB))
    }

    fn manifest_bytes() -> Vec<u8> {
        format!(
            r#"{{"schemaVersion": 2, "mediaType": "{MANIFEST_MT}",
  "artifactType": "{}",
  "config": {{"mediaType": "{}", "digest": "{}", "size": 2}},
  "layers": [{{"mediaType": "{}", "digest": "sha256:{}", "size": {}}}],
  "annotations": {{"{}": "tool-1.0.tfs", "{}": "tool"}}}}"#,
            ArtifactClass::Payload.artifact_type(),
            model::EMPTY_CONFIG_MT,
            model::EMPTY_CONFIG_DIGEST,
            ArtifactClass::Payload.layer_media_type(),
            blob_hex(),
            BLOB.len(),
            model::ANNOTATION_TITLE,
            model::ANNOTATION_NAME,
        )
        .into_bytes()
    }

    fn challenge(realm: &str) -> String {
        format!(r#"Bearer realm="{realm}",service="{HOST}""#)
    }

    fn repo(selector: Selector<'_>) -> RepoRef<'_> {
        RepoRef {
            host: HOST,
            repo: REPO,
            selector,
        }
    }

    fn resolve_ok(expect: ShapeExpectation) -> (MockHttp, Artifact) {
        let http = MockHttp::new();
        let manifest = manifest_bytes();
        let manifest_hex = format!("{:x}", Sha256::digest(&manifest));
        http.push_get(
            "/manifests/1.0",
            401,
            &[(
                "WWW-Authenticate",
                challenge(&format!("https://{HOST}/token")),
            )],
            b"",
        );
        http.push_get("/token?", 200, &[], br#"{"token":"tok-1"}"#);
        http.push_get(
            "/manifests/1.0",
            200,
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let artifact = client.resolve(&repo(Selector::Tag("1.0")), expect).unwrap();
        (http, artifact)
    }

    // -------------------------------------------------------------
    // The contract tests
    // -------------------------------------------------------------

    #[test]
    fn the_happy_path_dances_once_and_pins_the_origin() {
        let _guard = token_guard();
        let (http, artifact) = resolve_ok(ShapeExpectation::Class(ArtifactClass::Payload));
        assert_eq!(artifact.class, ArtifactClass::Payload);
        assert_eq!(artifact.layer.digest, format!("sha256:{}", blob_hex()));
        assert_eq!(
            artifact.origin,
            format!(
                "tfs+oci://{HOST}/{REPO}@sha256:{}",
                artifact.manifest_digest
            )
        );
        assert_eq!(artifact.annotations.title.as_deref(), Some("tool-1.0.tfs"));
        let log = http.requests();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].1, "anonymous");
        assert!(
            log[1].0.starts_with(&format!("https://{HOST}/token?")),
            "{}",
            log[1].0
        );
        assert!(
            log[1]
                .0
                .contains("scope=repository%3Atebako-packages%2Ftool%3Apull"),
            "{}",
            log[1].0
        );
        assert_eq!(log[2].1, "Bearer tok-1");
    }

    #[test]
    fn a_public_registry_never_dances() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/latest", 200, &[], &manifest_bytes());
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let artifact = client
            .resolve(&repo(Selector::Default), ShapeExpectation::AnyTebako)
            .unwrap();
        assert_eq!(artifact.class, ArtifactClass::Payload);
        let log = http.requests();
        assert_eq!(log.len(), 1);
        assert!(log[0].0.ends_with("/manifests/latest"), "{}", log[0].0);
    }

    #[test]
    fn the_error_body_code_maps_by_name() {
        let _guard = token_guard();
        for (code, want) in [
            ("MANIFEST_UNKNOWN", "OciManifestNotFound"),
            ("NAME_UNKNOWN", "OciManifestNotFound"),
            ("TAG_INVALID", "OciManifestNotFound"),
            ("UNAUTHORIZED", "CredentialRequired"),
            ("DENIED", "CredentialRequired"),
        ] {
            let http = MockHttp::new();
            http.push_get(
                "/manifests/",
                404,
                &[],
                format!(r#"{{"errors":[{{"code":"{code}"}}]}}"#).as_bytes(),
            );
            let creds = MockCreds::anonymous();
            let client = Client::new(&http, &creds);
            let err = client
                .resolve(&repo(Selector::Tag("9.9")), ShapeExpectation::AnyTebako)
                .unwrap_err();
            assert!(
                err.to_string().contains(want),
                "code {code}: expected {want} in {err}"
            );
        }
        // a bare 404 (no error body) falls back by endpoint kind
        let http = MockHttp::new();
        http.push_get("/manifests/", 404, &[], b"not found");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("9.9")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(matches!(err, OciError::ManifestNotFound { .. }), "{err:?}");
        // 429 rides the throttling schedule
        let http = MockHttp::new();
        http.push_get(
            "/manifests/",
            429,
            &[("Retry-After", "17".to_string())],
            b"",
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("9.9")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(
            matches!(
                err,
                OciError::Transport(FetchError::Throttled {
                    status: 429,
                    retry_after: Some(d),
                    ..
                }) if d == std::time::Duration::from_secs(17)
            ),
            "{err:?}"
        );
    }

    #[test]
    fn a_shape_violation_is_artifact_malformed() {
        let _guard = token_guard();
        let http = MockHttp::new();
        let bad = String::from_utf8(manifest_bytes())
            .unwrap()
            .replace("tool-1.0.tfs", "");
        http.push_get("/manifests/1.0", 200, &[], bad.as_bytes());
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(matches!(err, OciError::ArtifactMalformed { .. }), "{err:?}");
        assert!(err.to_string().contains("OciArtifactMalformed"), "{err}");
    }

    #[test]
    fn the_digest_pins_verify_the_manifest_bytes() {
        let _guard = token_guard();
        // the @sha256: selector pin disagrees with the served bytes
        let http = MockHttp::new();
        http.push_get("/manifests/sha256:", 200, &[], &manifest_bytes());
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(
                &repo(Selector::Digest(&"0".repeat(64))),
                ShapeExpectation::AnyTebako,
            )
            .unwrap_err();
        assert!(matches!(err, OciError::Sha256Mismatch { .. }), "{err:?}");
        // the registry's own Docker-Content-Digest disagrees
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            200,
            &[(
                "Docker-Content-Digest",
                format!("sha256:{}", "f".repeat(64)),
            )],
            &manifest_bytes(),
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(matches!(err, OciError::Sha256Mismatch { .. }), "{err:?}");
    }

    #[test]
    fn a_cross_host_realm_without_a_tier2_entry_is_the_named_refusal() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            401,
            &[(
                "WWW-Authenticate",
                challenge("https://auth.docker.io/token"),
            )],
            b"",
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(
            matches!(
                err,
                OciError::CrossHostAuthRefused { ref realm_host, .. } if realm_host == "auth.docker.io"
            ),
            "{err:?}"
        );
        assert!(err.to_string().contains("OciCrossHostAuthRefused"), "{err}");
        // and no request ever rode the foreign realm
        assert_eq!(http.requests().len(), 1);
    }

    #[test]
    fn a_cross_host_realm_with_a_tier2_entry_uses_it() {
        let _guard = token_guard();
        let mut creds = MockCreds::anonymous();
        creds.realms.insert(
            "auth.corp.example".to_string(),
            BasicCred {
                user_pass: "svc:s3cret".to_string(),
                class: "token:env:CORP_TOKEN".to_string(),
            },
        );
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            401,
            &[(
                "WWW-Authenticate",
                challenge("https://auth.corp.example/token"),
            )],
            b"",
        );
        http.push_get("/token?", 200, &[], br#"{"access_token":"tok-x"}"#);
        http.push_get("/manifests/1.0", 200, &[], &manifest_bytes());
        let client = Client::new(&http, &creds);
        client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap();
        let log = http.requests();
        assert_eq!(
            log[1].1,
            format!("Basic {}", base64_encode(b"svc:s3cret")),
            "the realm got its own tier-2 credential"
        );
        assert_eq!(log[2].1, "Bearer tok-x");
    }

    #[test]
    fn a_stale_cached_token_rechallenges_exactly_once() {
        let _guard = token_guard();
        // the cache holds a stale token for this host/scope/class
        let ch = auth::BearerChallenge {
            realm: format!("https://{HOST}/token"),
            service: Some(HOST.to_string()),
            scope: None,
        };
        auth::store_token(HOST, &ch, Some(&scope_for(REPO)), "anonymous", "stale");
        let http = MockHttp::new();
        // the preemptive stale token draws a 401 (re-challenge cue)…
        http.push_get(
            "/manifests/1.0",
            401,
            &[(
                "WWW-Authenticate",
                challenge(&format!("https://{HOST}/token")),
            )],
            b"",
        );
        // …the dance re-mints…
        http.push_get("/token?", 200, &[], br#"{"token":"fresh"}"#);
        // …and the fresh token rides
        http.push_get("/manifests/1.0", 200, &[], &manifest_bytes());
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap();
        let log = http.requests();
        assert_eq!(log.len(), 3, "{log:?}");
        assert_eq!(log[0].1, "Bearer stale");
        assert_eq!(log[2].1, "Bearer fresh");
    }

    #[test]
    fn a_twice_refused_token_is_credential_required() {
        let _guard = token_guard();
        let http = MockHttp::new();
        for _ in 0..2 {
            http.push_get(
                "/manifests/1.0",
                401,
                &[(
                    "WWW-Authenticate",
                    challenge(&format!("https://{HOST}/token")),
                )],
                b"",
            );
            http.push_get("/token?", 200, &[], br#"{"token":"tok"}"#);
        }
        http.push_get(
            "/manifests/1.0",
            401,
            &[(
                "WWW-Authenticate",
                challenge(&format!("https://{HOST}/token")),
            )],
            b"",
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(
            matches!(err, OciError::CredentialRequired { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn a_basic_only_challenge_is_token_challenge_invalid() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            401,
            &[("WWW-Authenticate", r#"Basic realm="registry""#.to_string())],
            b"",
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(
            matches!(err, OciError::TokenChallengeInvalid { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn an_insecure_realm_is_refused_before_any_request() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            401,
            &[(
                "WWW-Authenticate",
                challenge("http://auth.corp.example/token"),
            )],
            b"",
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap_err();
        assert!(matches!(err, OciError::InsecureTransport { .. }), "{err:?}");
        assert!(err.to_string().contains("OciInsecureTransport"), "{err}");
        assert_eq!(http.requests().len(), 1);
    }

    #[test]
    fn the_blob_fetch_verifies_the_layer_digest() {
        let _guard = token_guard();
        let (http, artifact) = resolve_ok(ShapeExpectation::Class(ArtifactClass::Payload));
        http.push_get("/blobs/sha256:", 200, &[], BLOB);
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let bytes = client
            .fetch_blob(&repo(Selector::Tag("1.0")), &artifact.layer)
            .unwrap();
        assert_eq!(bytes, BLOB);
        // the resolve dance's token rode the blob fetch preemptively
        let log = http.requests();
        assert_eq!(log.last().unwrap().1, "Bearer tok-1");
        // a corrupt blob is the named mismatch (a cold token cache, so
        // the second resolve's dance re-mints as scripted)
        auth::clear_tokens();
        let (http, artifact) = resolve_ok(ShapeExpectation::Class(ArtifactClass::Payload));
        http.push_get("/blobs/sha256:", 200, &[], b"corrupt");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = client
            .fetch_blob(&repo(Selector::Tag("1.0")), &artifact.layer)
            .unwrap_err();
        assert!(matches!(err, OciError::Sha256Mismatch { .. }), "{err:?}");
    }

    #[test]
    fn the_blob_stream_hashes_as_it_goes_and_dances_on_refusal() {
        let _guard = token_guard();
        let (http, artifact) = resolve_ok(ShapeExpectation::Class(ArtifactClass::Payload));
        // first stream attempt: refused; the harvest GET carries the
        // challenge; the retry rides the fresh token
        http.streams
            .lock()
            .unwrap()
            .push_back(ScriptedStream::AuthRejected);
        http.push_get(
            "/blobs/sha256:",
            401,
            &[(
                "WWW-Authenticate",
                challenge(&format!("https://{HOST}/token")),
            )],
            b"",
        );
        // the probe's challenge mints a fresh token…
        http.push_get("/token?", 200, &[], br#"{"token":"tok-2"}"#);
        http.streams
            .lock()
            .unwrap()
            .push_back(ScriptedStream::Bytes(BLOB.to_vec()));
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let mut out = Vec::new();
        let n = client
            .stream_blob(&repo(Selector::Tag("1.0")), &artifact.layer, &mut out, None)
            .unwrap();
        assert_eq!(n, BLOB.len() as u64);
        assert_eq!(out, BLOB);
    }

    #[test]
    fn basic_credentials_ride_the_same_host_realm() {
        let _guard = token_guard();
        let creds = MockCreds {
            registry: Some(BasicCred {
                user_pass: "robot:tok".to_string(),
                class: "token:docker-config:registry.example".to_string(),
            }),
            realms: HashMap::new(),
        };
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            401,
            &[(
                "WWW-Authenticate",
                challenge(&format!("https://{HOST}/token")),
            )],
            b"",
        );
        http.push_get("/token?", 200, &[], br#"{"token":"tok-b"}"#);
        http.push_get("/manifests/1.0", 200, &[], &manifest_bytes());
        let client = Client::new(&http, &creds);
        client
            .resolve(&repo(Selector::Tag("1.0")), ShapeExpectation::AnyTebako)
            .unwrap();
        let log = http.requests();
        assert_eq!(log[1].1, format!("Basic {}", base64_encode(b"robot:tok")));
    }

    proptest::proptest! {
        #[test]
        fn base64_round_trips_through_the_docker_decoder(bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..256)) {
            let encoded = base64_encode(&bytes);
            assert_eq!(
                dockerconfig::base64_decode(encoded.as_bytes()),
                Some(bytes)
            );
        }

        #[test]
        fn urlencode_stays_in_the_unreserved_alphabet(value in ".*") {
            let encoded = urlencode(&value);
            assert!(
                encoded
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-._~%".contains(&c)),
                "{encoded}"
            );
        }
    }
}
