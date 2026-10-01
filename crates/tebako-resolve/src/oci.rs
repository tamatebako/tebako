//! The OCI distribution adapter (spec 38 — the `oci` feature): the
//! bridge between tebako-oci's client and this crate's seams. The
//! [`Transport`] trait carries the bytes (the production transport rides
//! tebako-http's distribution surface; mocks key on URLs), and the
//! credential chain is spec 38 §6's: tier 1 (the directing registry's
//! alias, confined to the registry host), tier 2 (the exact host), tier
//! 3 the docker config's static `auths` (OCI-class only, journaled
//! `token:docker-config:<host>`), then anonymous. A challenge realm on
//! a FOREIGN host answers only to its own tier-2 entry — the client's
//! named `OciCrossHostAuthRefused` otherwise.
//!
//! Every OciError maps to exactly one spec 38 §9 named [`ResolveError`]
//! here — the pull path's single mapping site.

use std::sync::{Mutex, OnceLock};

use tebako_http::FetchError;
use tebako_oci::dockerconfig::{DockerConfig, DockerConfigError};
use tebako_oci::{
    ArtifactClass, BasicCred, Client, CredentialSource, Http, OciError, RepoRef, Selector,
    ShapeExpectation,
};

use crate::credentials::CredentialBook;
use crate::error::ResolveError;
use crate::plan::ItemFail;
use crate::reference::Reference;
use crate::transport::Transport;

/// The client's view of a parsed `Reference::Oci` (digest wins
/// defensively — the parser never produces tag+digest).
fn repo_ref<'a>(
    host: &'a str,
    repo: &'a str,
    tag: Option<&'a str>,
    digest: Option<&'a str>,
) -> RepoRef<'a> {
    let selector = match (tag, digest) {
        (_, Some(d)) => Selector::Digest(d),
        (Some(t), None) => Selector::Tag(t),
        (None, None) => Selector::Default,
    };
    RepoRef {
        host,
        repo,
        selector,
    }
}

// ---------------------------------------------------------------------
// The transport bridge
// ---------------------------------------------------------------------

/// [`tebako_oci::Http`] over this crate's [`Transport`] — the decided
/// credential header attaches verbatim; confinement was decided by the
/// credential chain before the header existed.
struct OciTransport<'a, T: Transport>(&'a T);

impl<T: Transport> Http for OciTransport<'_, T> {
    fn get(
        &self,
        url: &str,
        accept: Option<&str>,
        auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, FetchError> {
        let header = tebako_oci::authorization_header(auth);
        self.0
            .get_raw(url, accept, header.as_ref().map(|(k, v)| (*k, v.as_str())))
    }

    fn stream(
        &self,
        url: &str,
        auth: tebako_oci::Auth<'_>,
        writer: &mut dyn std::io::Write,
        on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
    ) -> Result<u64, FetchError> {
        let header = tebako_oci::authorization_header(auth);
        self.0.stream_distribution(
            url,
            None,
            header.as_ref().map(|(k, v)| (*k, v.as_str())),
            writer,
            on_progress,
        )
    }

    fn post(
        &self,
        url: &str,
        body: &[u8],
        content_type: Option<&str>,
        auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, FetchError> {
        let header = tebako_oci::authorization_header(auth);
        self.0.post_raw(
            url,
            body,
            content_type,
            header.as_ref().map(|(k, v)| (*k, v.as_str())),
        )
    }

    fn put(
        &self,
        url: &str,
        body: &[u8],
        content_type: &str,
        auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, FetchError> {
        let header = tebako_oci::authorization_header(auth);
        self.0.put_raw(
            url,
            body,
            content_type,
            header.as_ref().map(|(k, v)| (*k, v.as_str())),
        )
    }

    fn get_range(
        &self,
        url: &str,
        range: tebako_http::ByteRange,
        if_range: Option<&str>,
        auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, FetchError> {
        let header = tebako_oci::authorization_header(auth);
        self.0.get_range_raw(
            url,
            range,
            if_range,
            header.as_ref().map(|(k, v)| (*k, v.as_str())),
        )
    }
}

// ---------------------------------------------------------------------
// The credential chain (spec 38 §6)
// ---------------------------------------------------------------------

/// [`tebako_oci::CredentialSource`] over the credential book + the docker
/// config. `looked_for` records a MATCHED entry whose env var was unset
/// (or carried no `user:password` pair) — the anonymous ride continues
/// (a matched-but-absent credential behaves as absent, never a username
/// guess), and a later refusal names what it looked for.
pub struct OciBook<'a> {
    book: CredentialBook,
    alias: Option<&'a str>,
    registry_host: &'a str,
    docker: OnceLock<Result<Option<DockerConfig>, DockerConfigError>>,
    /// Test seam: the docker consult without touching `$HOME`.
    docker_override: Option<Result<Option<DockerConfig>, DockerConfigError>>,
    looked_for: Mutex<Option<String>>,
    journaled: Mutex<bool>,
}

impl<'a> OciBook<'a> {
    pub fn new(book: CredentialBook, alias: Option<&'a str>, registry_host: &'a str) -> Self {
        OciBook {
            book,
            alias,
            registry_host,
            docker: OnceLock::new(),
            docker_override: None,
            looked_for: Mutex::new(None),
            journaled: Mutex::new(false),
        }
    }

    /// The test constructor: an explicit docker consult result.
    #[doc(hidden)]
    pub fn with_docker(
        book: CredentialBook,
        alias: Option<&'a str>,
        registry_host: &'a str,
        docker: Result<Option<DockerConfig>, DockerConfigError>,
    ) -> Self {
        OciBook {
            docker_override: Some(docker),
            ..OciBook::new(book, alias, registry_host)
        }
    }

    fn docker(&self) -> &Result<Option<DockerConfig>, DockerConfigError> {
        self.docker.get_or_init(|| match &self.docker_override {
            Some(o) => o.clone(),
            None => DockerConfig::load(),
        })
    }

    /// What a refusal should name (a matched entry's missing or
    /// malformed token), if anything was looked for.
    fn take_looked_for(&self) -> Option<String> {
        self.looked_for
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// One book entry's env read: the `user:password` pair as a
    /// [`BasicCred`], or None with the expectation recorded (a matched
    /// entry whose var is unset or carries no `:` — spec 38 §6: the
    /// value MUST carry the pair; never a username guess).
    fn env_pair(&self, token_env: &str) -> Option<BasicCred> {
        match std::env::var(token_env).ok().filter(|v| !v.is_empty()) {
            Some(value) if value.contains(':') => Some(BasicCred {
                user_pass: value,
                class: format!("token:env:{token_env}"),
            }),
            Some(_) => {
                *self.looked_for.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("{token_env} (expected a user:password pair)"));
                None
            }
            None => {
                *self.looked_for.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(token_env.to_string());
                None
            }
        }
    }

    /// The fetch journal's one line per pull (host + credential class,
    /// values redacted — spec 37 §5's audit trail, the OCI classes
    /// included).
    fn journal(&self, class: &str) {
        let mut done = self.journaled.lock().unwrap_or_else(|e| e.into_inner());
        if *done {
            return;
        }
        if let Some(home) = crate::credentials::journal_home() {
            crate::credentials::journal_fetch(&home, self.registry_host, class);
            *done = true;
        }
    }
}

impl CredentialSource for OciBook<'_> {
    /// The registry host's credential: tier 1 confined to this host,
    /// tier 2 exact-host, tier 3 the docker config's static entry.
    fn registry_credential(&self) -> Result<Option<BasicCred>, OciError> {
        if let Some(alias) = self.alias {
            if let Some(entry) = self.book.tier1.iter().find(|e| e.alias == alias) {
                if entry.allowed_hosts.contains(self.registry_host) {
                    let cred = self.env_pair(&entry.token_env);
                    if let Some(cred) = &cred {
                        self.journal(&cred.class);
                    }
                    return Ok(cred);
                }
                // Confined out: the credential does not follow the ref —
                // the host chain answers (spec 37 §5's fall-through).
            }
        }
        if let Some((_, token_env)) = self
            .book
            .tier2
            .iter()
            .find(|(h, _)| h == self.registry_host)
        {
            let cred = self.env_pair(token_env);
            if let Some(cred) = &cred {
                self.journal(&cred.class);
            }
            return Ok(cred);
        }
        match self.docker() {
            Err(e) => Err(OciError::DockerConfig(e.clone())),
            Ok(Some(config)) => {
                let pair = config
                    .credential_for(self.registry_host)
                    .map_err(OciError::DockerConfig)?;
                Ok(pair.map(|user_pass| {
                    let class = format!("token:docker-config:{}", self.registry_host);
                    self.journal(&class);
                    BasicCred { user_pass, class }
                }))
            }
            Ok(None) => Ok(None),
        }
    }

    /// A FOREIGN token realm's credential: the realm host's own tier-2
    /// entry, nothing else (spec 38 §6's confinement — no tier 1, no
    /// docker fallback across hosts).
    fn realm_credential(&self, realm_host: &str) -> Result<Option<BasicCred>, OciError> {
        if let Some((_, token_env)) = self.book.tier2.iter().find(|(h, _)| h == realm_host) {
            return Ok(self.env_pair(token_env));
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------
// The §9 error mapping (the single site)
// ---------------------------------------------------------------------

/// OciError → the spec 38 §9 named [`ResolveError`]. A terminal
/// `CredentialRequired` names the book's looked-for expectation when
/// the client's own class carries none (an anonymous ride that was
/// refused while a matched entry's token was absent).
fn named(alias: Option<&str>, looked_for: Option<String>, err: OciError) -> ResolveError {
    match err {
        OciError::Transport(e) => crate::fetch::map_fetch_error("the OCI registry", e),
        OciError::InsecureTransport { url } => ResolveError::OciInsecureTransport { url },
        OciError::ManifestNotFound { origin } => ResolveError::OciManifestNotFound { origin },
        OciError::BlobUnknown { origin } => ResolveError::OciBlobUnknown { origin },
        OciError::ArtifactMalformed { origin, reason } => {
            ResolveError::OciArtifactMalformed { origin, reason }
        }
        OciError::TokenChallengeInvalid { reason } => {
            ResolveError::OciTokenChallengeInvalid { reason }
        }
        OciError::CrossHostAuthRefused {
            registry_host,
            realm_host,
        } => ResolveError::OciCrossHostAuthRefused {
            registry_host,
            realm_host,
        },
        OciError::CredentialRequired {
            host,
            looked_for: lf,
        } => ResolveError::CredentialRequired {
            registry: alias.map(str::to_string),
            host,
            looked_for: lf.filter(|c| c != "anonymous").or(looked_for),
        },
        OciError::DockerConfig(DockerConfigError::Malformed { path, reason }) => {
            ResolveError::DockerConfigMalformed { path, reason }
        }
        OciError::DockerConfig(DockerConfigError::HelperUnsupported { host, helper }) => {
            ResolveError::DockerCredentialHelperUnsupported { host, helper }
        }
        OciError::TagConflict {
            origin,
            tag,
            existing,
            attempted,
        } => ResolveError::OciTagConflict {
            origin,
            tag,
            existing,
            attempted,
        },
        OciError::Sha256Mismatch {
            origin,
            expected,
            actual,
        } => ResolveError::Sha256Mismatch {
            origin,
            expected,
            actual,
        },
    }
}

// ---------------------------------------------------------------------
// The pull entry points
// ---------------------------------------------------------------------

/// Resolve + pull a small artifact buffered (a registry index, a
/// detached signature, a direct small fetch): the manifest first (the
/// §3 shape law under `expect`), the byte pin fail-fast against the
/// layer descriptor, then the one layer — verified against its digest
/// by the client. Returns the bytes and the digest-pinned origin
/// (`tfs+oci://<host>/<repo>@sha256:<manifest digest>`, spec 38 §5.5).
/// Throttled answers ride tebako-http's backoff schedule, unchanged.
pub fn fetch_artifact<T: Transport>(
    transport: &T,
    reference: &Reference,
    alias: Option<&str>,
    expect: ShapeExpectation,
) -> Result<(Vec<u8>, String), ResolveError> {
    let Reference::Oci {
        host,
        repo,
        tag,
        digest,
        sha256,
    } = reference
    else {
        return Err(ResolveError::DownloadFailed {
            origin: reference.to_string(),
            reason: "the OCI adapter serves tfs+oci: references only".to_string(),
        });
    };
    let book = OciBook::new(crate::credentials::book(), alias, host);
    let oci_transport = OciTransport(transport);
    let client = Client::new(&oci_transport, &book);
    let r = repo_ref(host, repo, tag.as_deref(), digest.as_deref());
    let mut throttles = 0;
    loop {
        match pull_buffered(&client, &r, expect, sha256.as_deref()) {
            Ok(done) => return Ok(done),
            Err(OciError::Transport(FetchError::Throttled { retry_after, .. })) => {
                throttles += 1;
                if throttles >= tebako_http::THROTTLE_ROUNDS {
                    return Err(ResolveError::DownloadFailed {
                        origin: r.reference_string(),
                        reason: format!(
                            "still throttled after {} backoff rounds",
                            tebako_http::THROTTLE_ROUNDS
                        ),
                    });
                }
                std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
            }
            Err(e) => return Err(named(alias, book.take_looked_for(), e)),
        }
    }
}

/// One buffered pull attempt (the manifest + the one layer).
fn pull_buffered<H: Http, C: CredentialSource>(
    client: &Client<'_, H, C>,
    r: &RepoRef<'_>,
    expect: ShapeExpectation,
    byte_pin: Option<&str>,
) -> Result<(Vec<u8>, String), OciError> {
    let artifact = client.resolve(r, expect)?;
    check_byte_pin(r, byte_pin, &artifact.layer)?;
    let bytes = client.fetch_blob(r, &artifact.layer)?;
    Ok((bytes, artifact.origin))
}

/// The spec 38 §5 fail-fast: a `?sha256=` byte pin names the LAYER
/// blob's digest — checked against the layer descriptor before any
/// blob byte moves.
fn check_byte_pin(
    r: &RepoRef<'_>,
    byte_pin: Option<&str>,
    layer: &tebako_oci::Descriptor,
) -> Result<(), OciError> {
    let Some(pin) = byte_pin else { return Ok(()) };
    if Some(pin) == layer.digest_hex() {
        return Ok(());
    }
    Err(OciError::Sha256Mismatch {
        origin: r.reference_string(),
        expected: pin.to_string(),
        actual: layer.digest.clone(),
    })
}

/// The registry-index pull (spec 38 §4): a registry-class artifact,
/// shape-checked as such.
pub fn fetch_registry_file<T: Transport>(
    transport: &T,
    reference: &Reference,
    alias: Option<&str>,
) -> Result<(Vec<u8>, String), ResolveError> {
    fetch_artifact(
        transport,
        reference,
        alias,
        ShapeExpectation::Class(ArtifactClass::RegistryIndex),
    )
}

// ---------------------------------------------------------------------
// The push entry point (spec 38 §7)
// ---------------------------------------------------------------------

/// Place one §3 artifact under `tfs+oci://<host>/<repo>:<tag>` — the
/// CLI's `--oci` publish leg. `bytes` are the SAME staged bytes the
/// git-host release leg uploads (the layer digest IS the file's sha256
/// trust anchor); `annotations` carries the L3 mirror with the title
/// set. The client checks the tag write-once (a same-bytes re-publish
/// is the idempotent skip, anything else the named 69), mounts or
/// uploads the blob, and places the manifest — every request dancing at
/// the `repository:<repo>:pull,push` scope. No registry alias rides a
/// command-line reference (the credential chain's tier 1 has no name to
/// match). Throttled answers ride tebako-http's backoff schedule,
/// unchanged.
pub fn push_artifact<T: Transport>(
    transport: &T,
    host: &str,
    repo: &str,
    tag: &str,
    class: ArtifactClass,
    annotations: &tebako_oci::Annotations,
    bytes: &[u8],
) -> Result<tebako_oci::push::PushOutcome, ResolveError> {
    let book = OciBook::new(crate::credentials::book(), None, host);
    let oci_transport = OciTransport(transport);
    let client = Client::new(&oci_transport, &book);
    let mut throttles = 0;
    loop {
        match tebako_oci::push::push_artifact(&client, host, repo, tag, class, annotations, bytes) {
            Ok(outcome) => return Ok(outcome),
            Err(OciError::Transport(FetchError::Throttled { retry_after, .. })) => {
                throttles += 1;
                if throttles >= tebako_http::THROTTLE_ROUNDS {
                    return Err(ResolveError::DownloadFailed {
                        origin: format!("tfs+oci://{host}/{repo}:{tag}"),
                        reason: format!(
                            "still throttled after {} backoff rounds",
                            tebako_http::THROTTLE_ROUNDS
                        ),
                    });
                }
                std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
            }
            Err(e) => return Err(named(None, book.take_looked_for(), e)),
        }
    }
}

/// The manifest digest a reference resolves to right now (spec 38 §4's
/// dispatch-cache re-serve check): the manifest read only — the blob
/// pull is what the check saves.
pub fn resolve_manifest_digest<T: Transport>(
    transport: &T,
    reference: &Reference,
    alias: Option<&str>,
) -> Result<String, ResolveError> {
    let Reference::Oci {
        host,
        repo,
        tag,
        digest,
        ..
    } = reference
    else {
        return Err(ResolveError::DownloadFailed {
            origin: reference.to_string(),
            reason: "the OCI adapter serves tfs+oci: references only".to_string(),
        });
    };
    let book = OciBook::new(crate::credentials::book(), alias, host);
    let oci_transport = OciTransport(transport);
    let client = Client::new(&oci_transport, &book);
    let r = repo_ref(host, repo, tag.as_deref(), digest.as_deref());
    client
        .resolve(&r, ShapeExpectation::Class(ArtifactClass::RegistryIndex))
        .map(|a| a.manifest_digest)
        .map_err(|e| named(alias, book.take_looked_for(), e))
}

/// The spec 39 §8 blksum sidecar over OCI: resolve the sibling
/// digest-tag (`sha256-<image sha>.blksum.json` — spec 38 §3's rule)
/// in `<host>/<repo>`, fetch the one layer (its digest IS the sidecar
/// document's sha256 — the seed descriptor's `blksum_sha256` pin binds
/// it with no second file), and parse the blksum document. Returns the
/// document and the digest-pinned origin. `Ok(None)` is the spec 39 §3
/// blksum-missing signal — the sibling tag names nothing, and the loud
/// eager fallback is the CALLER's law, never an error here. A sidecar
/// that pins a DIFFERENT image than the tag names is a torn publish:
/// the named sha mismatch, never a silent serve.
pub fn fetch_blksum_sidecar<T: Transport>(
    transport: &T,
    host: &str,
    repo: &str,
    image_blob_sha256: &str,
    alias: Option<&str>,
) -> Result<Option<(tpkg::lazy::Blksum, String)>, ResolveError> {
    let book = OciBook::new(crate::credentials::book(), alias, host);
    let oci_transport = OciTransport(transport);
    let client = Client::new(&oci_transport, &book);
    let tag = tebako_oci::blksum_tag(image_blob_sha256);
    let r = repo_ref(host, repo, Some(&tag), None);
    let mut throttles = 0;
    loop {
        match pull_blksum(&client, &r, image_blob_sha256) {
            Ok(done) => return Ok(done),
            Err(OciError::Transport(FetchError::Throttled { retry_after, .. })) => {
                throttles += 1;
                if throttles >= tebako_http::THROTTLE_ROUNDS {
                    return Err(ResolveError::DownloadFailed {
                        origin: r.reference_string(),
                        reason: format!(
                            "still throttled after {} backoff rounds",
                            tebako_http::THROTTLE_ROUNDS
                        ),
                    });
                }
                std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
            }
            Err(e) => return Err(named(alias, book.take_looked_for(), e)),
        }
    }
}

/// One blksum sidecar pull attempt (the sibling-tag manifest + the one
/// layer + the strict document checks).
fn pull_blksum<H: Http, C: CredentialSource>(
    client: &Client<'_, H, C>,
    r: &RepoRef<'_>,
    image_blob_sha256: &str,
) -> Result<Option<(tpkg::lazy::Blksum, String)>, OciError> {
    let artifact = match client.resolve(r, ShapeExpectation::Class(ArtifactClass::Blksum)) {
        Ok(artifact) => artifact,
        Err(OciError::ManifestNotFound { .. }) => return Ok(None),
        Err(e) => return Err(e),
    };
    let bytes = client.fetch_blob(r, &artifact.layer)?;
    let text = String::from_utf8(bytes).map_err(|_| OciError::ArtifactMalformed {
        origin: artifact.origin.clone(),
        reason: "the blksum sidecar is not UTF-8".to_string(),
    })?;
    let blksum =
        tpkg::lazy::Blksum::parse(&text).map_err(|e| OciError::ArtifactMalformed {
            origin: artifact.origin.clone(),
            reason: format!("the blksum sidecar document: {e}"),
        })?;
    if !blksum.sha256.eq_ignore_ascii_case(image_blob_sha256) {
        return Err(OciError::Sha256Mismatch {
            origin: artifact.origin,
            expected: image_blob_sha256.to_string(),
            actual: blksum.sha256,
        });
    }
    Ok(Some((blksum, artifact.origin)))
}

/// The plan-pipeline pull (spec 38 §5.2): the payload artifact's one
/// layer streams through the pipeline's HashWriter — the client
/// verifies the stream against the layer digest inline, the pipeline
/// verifies any byte pin against its own hash, and the commit closure
/// owns install semantics exactly as every other class. Transport
/// answers surface as [`ItemFail::Transport`] (the executor's
/// retry/throttle discipline owns them); everything else is the named
/// error.
pub(crate) fn stream_artifact<T: Transport>(
    transport: &T,
    reference: &Reference,
    alias: Option<&str>,
    writer: &mut dyn std::io::Write,
    tick: &mut dyn FnMut(u64, Option<u64>) -> bool,
) -> Result<(u64, String), ItemFail> {
    let Reference::Oci {
        host,
        repo,
        tag,
        digest,
        sha256,
    } = reference
    else {
        return Err(ItemFail::Named(ResolveError::DownloadFailed {
            origin: reference.to_string(),
            reason: "the OCI adapter serves tfs+oci: references only".to_string(),
        }));
    };
    let book = OciBook::new(crate::credentials::book(), alias, host);
    let oci_transport = OciTransport(transport);
    let client = Client::new(&oci_transport, &book);
    let r = repo_ref(host, repo, tag.as_deref(), digest.as_deref());
    let map = |e: OciError| match e {
        OciError::Transport(f) => ItemFail::Transport(f),
        other => ItemFail::Named(named(alias, book.take_looked_for(), other)),
    };
    let artifact = client
        .resolve(&r, ShapeExpectation::Class(ArtifactClass::Payload))
        .map_err(map)?;
    check_byte_pin(&r, sha256.as_deref(), &artifact.layer).map_err(map)?;
    let n = client
        .stream_blob(&r, &artifact.layer, writer, Some(tick))
        .map_err(map)?;
    Ok((n, artifact.origin))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::Tier1Entry;
    use std::collections::BTreeSet;

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    const VAR: &str = "TEBAKO_TEST_OCI_CRED";
    const VAR2: &str = "TEBAKO_TEST_OCI_CRED_TWO";

    fn book_with(host: &str, env: &str) -> CredentialBook {
        CredentialBook {
            tier1: vec![],
            tier2: vec![(host.to_string(), env.to_string())],
            ref_index: vec![],
        }
    }

    #[test]
    fn tier1_confines_to_the_registry_host() {
        let _guard = env_guard();
        std::env::set_var(VAR, "user:pass");
        let book = CredentialBook {
            tier1: vec![Tier1Entry {
                alias: "corp".to_string(),
                token_env: VAR.to_string(),
                allowed_hosts: BTreeSet::from(["ghcr.io".to_string()]),
            }],
            tier2: vec![],
            ref_index: vec![],
        };
        // confined-in: the credential answers
        let src = OciBook::with_docker(book.clone(), Some("corp"), "ghcr.io", Ok(None));
        let cred = src.registry_credential().unwrap().unwrap();
        assert_eq!(cred.user_pass, "user:pass");
        assert_eq!(cred.class, format!("token:env:{VAR}"));
        // confined-out: the credential does not follow the ref
        let src = OciBook::with_docker(book, Some("corp"), "harbor.other.example", Ok(None));
        assert_eq!(src.registry_credential().unwrap(), None);
        std::env::remove_var(VAR);
    }

    #[test]
    fn the_chain_runs_tier2_then_docker_then_anonymous() {
        let _guard = env_guard();
        std::env::set_var(VAR, "u:p");
        // tier 2 answers before the docker consult
        let dir = std::env::temp_dir().join(format!("tebako-oci-book-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = dir.join("config.json");
        // ghcr.io's docker pair decodes to "docker:pair" (ZG9ja2VyOnBhaXI=)
        std::fs::write(
            &cfg_path,
            r#"{"auths":{"ghcr.io":{"auth":"ZG9ja2VyOnBhaXI="}}}"#,
        )
        .unwrap();
        let docker = DockerConfig::load_from(cfg_path.clone()).unwrap();
        let src = OciBook::with_docker(book_with("ghcr.io", VAR), None, "ghcr.io", Ok(docker));
        let cred = src.registry_credential().unwrap().unwrap();
        assert_eq!(cred.user_pass, "u:p", "tier 2 wins over the docker entry");
        std::env::remove_var(VAR);
        // no book entry: the docker tier answers, journaled by its class
        let docker = DockerConfig::load_from(cfg_path.clone()).unwrap();
        let src = OciBook::with_docker(CredentialBook::default(), None, "ghcr.io", Ok(docker));
        let cred = src.registry_credential().unwrap().unwrap();
        assert_eq!(cred.user_pass, "docker:pair");
        assert_eq!(cred.class, "token:docker-config:ghcr.io");
        // neither: anonymous
        let src = OciBook::with_docker(CredentialBook::default(), None, "other.example", Ok(None));
        assert_eq!(src.registry_credential().unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_covering_helper_and_a_malformed_config_are_named() {
        let _guard = env_guard();
        let src = OciBook::with_docker(
            CredentialBook::default(),
            None,
            "ghcr.io",
            Err(DockerConfigError::HelperUnsupported {
                host: "ghcr.io".to_string(),
                helper: "desktop".to_string(),
            }),
        );
        let err = src.registry_credential().unwrap_err();
        assert!(
            matches!(
                err,
                OciError::DockerConfig(DockerConfigError::HelperUnsupported { .. })
            ),
            "{err:?}"
        );
        let mapped = named(None, None, err);
        assert!(
            matches!(
                mapped,
                ResolveError::DockerCredentialHelperUnsupported { .. }
            ),
            "{mapped:?}"
        );
        assert!(
            mapped
                .to_string()
                .contains("DockerCredentialHelperUnsupported"),
            "{mapped}"
        );
        let src = OciBook::with_docker(
            CredentialBook::default(),
            None,
            "ghcr.io",
            Err(DockerConfigError::Malformed {
                path: std::path::PathBuf::from("/x/config.json"),
                reason: "bad json".to_string(),
            }),
        );
        let mapped = named(None, None, src.registry_credential().unwrap_err());
        assert!(
            matches!(mapped, ResolveError::DockerConfigMalformed { .. }),
            "{mapped:?}"
        );
    }

    #[test]
    fn a_matched_entry_without_a_pair_is_looked_for_never_guessed() {
        let _guard = env_guard();
        // unset env: the match happened, the token is absent
        std::env::remove_var(VAR2);
        let src = OciBook::with_docker(book_with("ghcr.io", VAR2), None, "ghcr.io", Ok(None));
        assert_eq!(src.registry_credential().unwrap(), None);
        assert_eq!(src.take_looked_for().as_deref(), Some(VAR2));
        // a value without ':' is never a username guess
        std::env::set_var(VAR2, "just-a-token");
        let src = OciBook::with_docker(book_with("ghcr.io", VAR2), None, "ghcr.io", Ok(None));
        assert_eq!(src.registry_credential().unwrap(), None);
        assert!(src
            .take_looked_for()
            .unwrap()
            .contains("expected a user:password pair"));
        // …and a refusal names the expectation
        let mapped = named(
            Some("corp"),
            src.take_looked_for(),
            OciError::CredentialRequired {
                host: "ghcr.io".to_string(),
                looked_for: Some("anonymous".to_string()),
            },
        );
        assert!(
            matches!(
                &mapped,
                ResolveError::CredentialRequired { registry, looked_for, .. }
                if registry.as_deref() == Some("corp")
                    && looked_for.as_deref().is_some_and(|l| l.contains("user:password"))
            ),
            "{mapped:?}"
        );
        std::env::remove_var(VAR2);
    }

    #[test]
    fn the_realm_consults_only_its_own_tier2_entry() {
        let _guard = env_guard();
        std::env::set_var(VAR, "realm:pair");
        let book = book_with("auth.docker.io", VAR);
        let src = OciBook::with_docker(book, Some("corp"), "registry-1.docker.io", Ok(None));
        let cred = src.realm_credential("auth.docker.io").unwrap().unwrap();
        assert_eq!(cred.user_pass, "realm:pair");
        // no entry for the realm host: None — the client refuses by name
        assert_eq!(src.realm_credential("other.example").unwrap(), None);
        std::env::remove_var(VAR);
    }

    #[test]
    fn the_named_mapping_covers_every_oci_error() {
        let cases: Vec<(OciError, &str)> = vec![
            (
                OciError::InsecureTransport {
                    url: "http://x".into(),
                },
                "OciInsecureTransport",
            ),
            (
                OciError::ManifestNotFound { origin: "o".into() },
                "OciManifestNotFound",
            ),
            (
                OciError::BlobUnknown { origin: "o".into() },
                "OciBlobUnknown",
            ),
            (
                OciError::ArtifactMalformed {
                    origin: "o".into(),
                    reason: "r".into(),
                },
                "OciArtifactMalformed",
            ),
            (
                OciError::TokenChallengeInvalid { reason: "r".into() },
                "OciTokenChallengeInvalid",
            ),
            (
                OciError::CrossHostAuthRefused {
                    registry_host: "a".into(),
                    realm_host: "b".into(),
                },
                "OciCrossHostAuthRefused",
            ),
            (
                OciError::Sha256Mismatch {
                    origin: "o".into(),
                    expected: "e".into(),
                    actual: "a".into(),
                },
                "sha256 mismatch",
            ),
            (
                OciError::TagConflict {
                    origin: "o".into(),
                    tag: "1.0".into(),
                    existing: "sha256:aa".into(),
                    attempted: "sha256:bb".into(),
                },
                "OciTagConflict",
            ),
        ];
        for (err, needle) in cases {
            let mapped = named(None, None, err);
            assert!(
                mapped.to_string().contains(needle),
                "expected '{needle}' in: {mapped}"
            );
        }
    }
}
