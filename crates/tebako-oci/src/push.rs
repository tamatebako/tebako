//! The publish half of spec 38 (§7): one artifact's placement under its
//! tag — in-process, behind the same [`Http`] seam as the pull path.
//!
//! The flow per artifact:
//!
//! 1. **Write-once.** GET the target tag's manifest (the push scope's
//!    dance on a 401). A 404 is a fresh tag; a 200 whose manifest digest
//!    (the registry's `Docker-Content-Digest`, else the served body's
//!    own hash) equals the digest of the manifest THIS push would place
//!    is the idempotent re-publish skip — nothing uploads; anything else
//!    is the named [`OciError::TagConflict`] (69) — tags are mutable in
//!    the OCI model, tebako treats them as immutable by policy.
//! 2. **The blobs.** The empty config first (the canonical `{}` bytes —
//!    registries validate the manifest's config reference; zot 400s a
//!    manifest whose config blob is absent), then the layer. Each rides
//!    the mount attempt first (the distribution spec's cross-repo
//!    dedup: `POST …/blobs/uploads/?mount=<digest>&from=<repo>`
//!    — a 201 means the blob already stands); a 202 falls through to the
//!    monolithic upload of open question 6's v1: PUT the session's
//!    Location with `?digest=` and the raw bytes (expect 201).
//! 3. **The manifest.** PUT `/v2/<repo>/manifests/<tag>` with
//!    `Content-Type: application/vnd.oci.image.manifest.v1+json`.
//!
//! Every request dances at the `repository:<repo>:pull,push` scope; the
//! re-challenge-once discipline is [`Client::send_with_dance`]'s, the
//! pull path's own. The tag check and the manifest PUT are not atomic
//! (no distribution-spec conditional write exists) — write-once is a
//! publish-side policy, exactly like the git-host release leg's
//! replace-before-upload.
//!
//! Transport resilience is [`retry_push`]'s, tebako-http's one retry
//! law mirrored for the write wire: a throttled answer waits
//! [`tebako_http::throttle_backoff`] (Retry-After honored exactly) for
//! up to [`tebako_http::THROTTLE_ROUNDS`] rounds; a transport failure
//! retries the operation from zero up to [`PUSH_ATTEMPTS`] times with
//! [`PUSH_RETRY_DELAY`] between. Auth-rechallenging alone cannot absorb
//! the broken-pipe class a long layer PUT takes (the first ghcr tag run
//! lost a 200 MB upload to one); every operation the retry wraps is
//! idempotent under it — the tag check and the manifest PUT name the
//! same bytes, and a retried blob leg takes a FRESH upload session
//! (the session a broken PUT stranded is never reused).

use sha2::{Digest, Sha256};

use tebako_http::FetchError;

use crate::model::Manifest;
use crate::{
    push_scope_for, scheme_for, url_host, Annotations, ArtifactClass, Client, CredentialSource,
    Descriptor, Endpoint, Http, OciError, RepoRef, Selector, MANIFEST_MT,
};

/// One publish operation's attempt budget — the range GET's own
/// ([`tebako_http::RANGE_ATTEMPTS`]); the wire crate has one retry law
/// and this is its publish half, not a second schedule.
const PUSH_ATTEMPTS: u32 = tebako_http::RANGE_ATTEMPTS;
/// The delay between publish attempts — [`tebako_http::RANGE_RETRY_DELAY`],
/// same law.
const PUSH_RETRY_DELAY: std::time::Duration = tebako_http::RANGE_RETRY_DELAY;

/// What one [`push_artifact`] placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushOutcome {
    /// The tag the artifact rides.
    pub tag: String,
    /// The artifact's layer descriptor — its digest IS the file's sha256
    /// (spec 38 §3's trust-anchor equivalence).
    pub layer: Descriptor,
    /// The 64-hex sha256 of the manifest bytes as PUT.
    pub manifest_digest: String,
    /// `tfs+oci://<host>/<repo>@sha256:<manifest digest>` — the
    /// digest-pinned spelling (the same origin grammar the pull records).
    pub origin: String,
    /// The tag already named exactly these bytes — the idempotent
    /// re-publish skip; nothing was uploaded.
    pub skipped: bool,
}

/// One publish operation under tebako-http's retry law (the range
/// GET's, mirrored for the write wire — the module doc's law): a
/// throttled answer waits [`tebako_http::throttle_backoff`] for up to
/// [`tebako_http::THROTTLE_ROUNDS`] rounds; a transport failure retries
/// THE OPERATION FROM ZERO up to [`PUSH_ATTEMPTS`] times with
/// [`PUSH_RETRY_DELAY`] between. Everything else is terminal by the
/// crate's one law — the named answers (TagConflict, CredentialRequired,
/// the malformed-artifact class) never retry.
fn retry_push<T>(mut op: impl FnMut() -> Result<T, OciError>) -> Result<T, OciError> {
    let mut attempts = 0;
    let mut throttles = 0;
    loop {
        match op() {
            Err(OciError::Transport(FetchError::Throttled {
                retry_after, status, ..
            })) => {
                throttles += 1;
                if throttles >= tebako_http::THROTTLE_ROUNDS {
                    return Err(OciError::Transport(FetchError::DownloadFailed(format!(
                        "still throttled after {} backoff rounds publishing ({status})",
                        tebako_http::THROTTLE_ROUNDS
                    ))));
                }
                std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
            }
            Err(OciError::Transport(FetchError::DownloadFailed(msg))) => {
                attempts += 1;
                if attempts >= PUSH_ATTEMPTS {
                    return Err(OciError::Transport(FetchError::DownloadFailed(format!(
                        "the publish failed after {PUSH_ATTEMPTS} attempts: {msg}"
                    ))));
                }
                std::thread::sleep(PUSH_RETRY_DELAY);
            }
            other => return other,
        }
    }
}

/// Push one §3 artifact (`bytes` are the served file's raw bytes — the
/// SAME staged bytes the git-host release leg uploads) under
/// `tfs+oci://<host>/<repo>:<tag>`. `annotations` carries the L3 mirror
/// with the title (the served file name) set — the render names a
/// missing title exactly as the read side's shape law does.
pub fn push_artifact<H: Http, C: CredentialSource>(
    client: &Client<'_, H, C>,
    host: &str,
    repo: &str,
    tag: &str,
    class: ArtifactClass,
    annotations: &Annotations,
    bytes: &[u8],
) -> Result<PushOutcome, OciError> {
    let r = RepoRef {
        host,
        repo,
        selector: Selector::Tag(tag),
    };
    let scope = push_scope_for(repo);
    let layer = Descriptor {
        media_type: class.layer_media_type().to_string(),
        digest: format!("sha256:{:x}", Sha256::digest(bytes)),
        size: bytes.len() as u64,
    };
    let manifest = Manifest::render(class, &layer, annotations).map_err(|reason| {
        OciError::ArtifactMalformed {
            origin: r.reference_string(),
            reason,
        }
    })?;
    let manifest_digest = format!("{:x}", Sha256::digest(&manifest));
    let manifest_url = format!(
        "{}://{}/v2/{}/manifests/{}",
        scheme_for(host),
        host,
        repo,
        tag
    );

    // ---- 1. write-once -------------------------------------------------
    let (resp, _) = retry_push(|| {
        let (resp, class) = client.send_with_dance(&r, &scope, &|auth| {
            client.http.get(&manifest_url, Some(MANIFEST_MT), auth)
        })?;
        match resp.status {
            // fresh tag (an unknown repository is a 404 too — NAME_UNKNOWN),
            // or the tag's standing manifest (the idempotence check below)
            404 | 200 => Ok((resp, class)),
            _ => Err(client.classify_error(&manifest_url, Endpoint::Manifest, &r, &resp)),
        }
    })?;
    match resp.status {
        404 => {}
        200 => {
            let existing = resp
                .header("docker-content-digest")
                .map(|d| d.strip_prefix("sha256:").unwrap_or(d).to_string())
                .unwrap_or_else(|| format!("{:x}", Sha256::digest(&resp.body)));
            if existing.eq_ignore_ascii_case(&manifest_digest) {
                return Ok(PushOutcome {
                    tag: tag.to_string(),
                    layer,
                    origin: format!("tfs+oci://{host}/{repo}@sha256:{manifest_digest}"),
                    manifest_digest,
                    skipped: true,
                });
            }
            return Err(OciError::TagConflict {
                origin: format!("tfs+oci://{host}/{repo}"),
                tag: tag.to_string(),
                existing: format!("sha256:{existing}"),
                attempted: format!("sha256:{manifest_digest}"),
            });
        }
        status => unreachable!("the retry classified every other status: {status}"),
    }

    // ---- 2. the blobs: the empty config first (registries validate the
    // manifest's config reference — zot 400s a manifest whose config
    // blob is absent), then the layer. Mount attempt, then the
    // monolithic upload. A transport failure retries the WHOLE leg, so
    // a broken PUT never re-rides its stranded session.
    retry_push(|| {
        push_blob(
            client,
            &r,
            host,
            repo,
            &scope,
            crate::EMPTY_CONFIG_DIGEST,
            b"{}",
        )
    })?;
    let hex = layer
        .digest_hex()
        .expect("the layer digest was built as sha256:<hex>");
    retry_push(|| push_blob(client, &r, host, repo, &scope, &format!("sha256:{hex}"), bytes))?;

    // ---- 3. the manifest placement --------------------------------------
    let (resp, _) = retry_push(|| {
        let (resp, class) = client.send_with_dance(&r, &scope, &|auth| {
            client.http.put(&manifest_url, &manifest, MANIFEST_MT, auth)
        })?;
        match resp.status {
            200..=202 => Ok((resp, class)),
            _ => Err(client.classify_error(&manifest_url, Endpoint::Manifest, &r, &resp)),
        }
    })?;
    // The registry's own echo pins what it stored (the pull path's
    // discipline, mirrored): a served digest that disagrees with the
    // bytes we sent is the named mismatch, never a shrug.
    if let Some(served) = resp.header("docker-content-digest") {
        let served_hex = served.strip_prefix("sha256:").unwrap_or(served);
        if !served_hex.eq_ignore_ascii_case(&manifest_digest) {
            return Err(OciError::Sha256Mismatch {
                origin: r.reference_string(),
                expected: format!("sha256:{manifest_digest}"),
                actual: served.to_string(),
            });
        }
    }

    Ok(PushOutcome {
        tag: tag.to_string(),
        layer,
        origin: format!("tfs+oci://{host}/{repo}@sha256:{manifest_digest}"),
        manifest_digest,
        skipped: false,
    })
}

/// One blob's placement (spec 38 §7 step 2): the mount attempt first
/// (the distribution spec's cross-repo dedup — a 201 means the blob
/// already stands); a 202 falls through to the monolithic upload of
/// open question 6's v1: PUT the session's Location with `?digest=`
/// and the raw bytes (expect 201). `digest` is the full `sha256:<hex>`
/// spelling.
fn push_blob<H: Http, C: CredentialSource>(
    client: &Client<'_, H, C>,
    r: &RepoRef<'_>,
    host: &str,
    repo: &str,
    scope: &str,
    digest: &str,
    bytes: &[u8],
) -> Result<(), OciError> {
    let mount_url = format!(
        "{}://{}/v2/{}/blobs/uploads/?mount={}&from={}",
        scheme_for(host),
        host,
        repo,
        digest,
        crate::urlencode(repo)
    );
    let (resp, _) = client.send_with_dance(r, scope, &|auth| {
        client.http.post(&mount_url, &[], None, auth)
    })?;
    match resp.status {
        // 201 Created — the blob mounted (the dedup hit)
        201 => Ok(()),
        // 202 Accepted — no mount; the session's Location takes the upload
        202 => {
            let location = resp
                .header("location")
                .ok_or_else(|| OciError::ArtifactMalformed {
                    origin: r.reference_string(),
                    reason: "the blob upload session's 202 carries no Location header".to_string(),
                })?;
            let upload = resolve_location(host, location)?;
            let sep = if upload.contains('?') { '&' } else { '?' };
            let upload = format!("{upload}{sep}digest={digest}");
            let (resp, _) = client.send_with_dance(r, scope, &|auth| {
                client
                    .http
                    .put(&upload, bytes, "application/octet-stream", auth)
            })?;
            match resp.status {
                201 => Ok(()),
                _ => Err(client.classify_error(&upload, Endpoint::Blob, r, &resp)),
            }
        }
        _ => Err(client.classify_error(&mount_url, Endpoint::Blob, r, &resp)),
    }
}

/// The upload session's Location resolved for the PUT: a root-relative
/// path rides the registry's own scheme+host; an absolute URL is
/// accepted only when it names the SAME host — the push credential
/// never rides cross-host (spec 38 §6's confinement applied to the
/// upload session, the §8 redirect rule's twin).
fn resolve_location(host: &str, location: &str) -> Result<String, OciError> {
    if location.starts_with('/') {
        return Ok(format!("{}://{host}{location}", scheme_for(host)));
    }
    if location.starts_with("https://") || location.starts_with("http://") {
        let location_host = url_host(location);
        if location_host.eq_ignore_ascii_case(host) {
            return Ok(location.to_string());
        }
        return Err(OciError::ArtifactMalformed {
            origin: location.to_string(),
            // a POLICY refusal, not a transport failure — the publish
            // retry law must never re-fire it (the mount POST's answer
            // was complete; the registry served a Location we refuse)
            reason: format!(
                "the registry's upload Location names the foreign host {location_host} — the push credential never rides cross-host"
            ),
        });
    }
    Err(OciError::ArtifactMalformed {
        origin: location.to_string(),
        reason:
            "the blob upload session's Location is neither an absolute URL nor a root-relative path"
                .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth;
    use crate::tests::{token_guard, MockCreds, MockHttp};

    const HOST: &str = "registry.example";
    const REPO: &str = "tebako-packages/tool";
    const TAG: &str = "1.0";
    const BYTES: &[u8] = b"the staged payload bytes";

    fn annotations() -> Annotations {
        Annotations {
            title: Some("tool-1.0.tfs".to_string()),
            name: Some("tool".to_string()),
            version: Some("1.0".to_string()),
            kind: Some("app".to_string()),
            triplet: Some("universal".to_string()),
            ..Annotations::default()
        }
    }

    fn layer() -> Descriptor {
        Descriptor {
            media_type: ArtifactClass::Payload.layer_media_type().to_string(),
            digest: format!("sha256:{:x}", Sha256::digest(BYTES)),
            size: BYTES.len() as u64,
        }
    }

    fn manifest_bytes() -> Vec<u8> {
        Manifest::render(ArtifactClass::Payload, &layer(), &annotations()).unwrap()
    }

    fn manifest_hex() -> String {
        format!("{:x}", Sha256::digest(manifest_bytes()))
    }

    fn challenge(realm: &str) -> String {
        format!(r#"Bearer realm="{realm}",service="{HOST}""#)
    }

    /// The happy path's script, monolithic arm: fresh tag, no mount on
    /// either blob, the upload sessions' Locations take the config and
    /// the layer, the manifest lands.
    fn script_monolithic(http: &MockHttp) {
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post(
            "/blobs/uploads/?mount=",
            202,
            &[("Location", format!("/v2/{REPO}/blobs/uploads/session-0"))],
            b"",
        );
        http.push_put("/blobs/uploads/session-0?digest=sha256:", 201, &[], b"");
        http.push_post(
            "/blobs/uploads/?mount=",
            202,
            &[("Location", format!("/v2/{REPO}/blobs/uploads/session-1"))],
            b"",
        );
        http.push_put("/blobs/uploads/session-1?digest=sha256:", 201, &[], b"");
        http.push_put("/manifests/1.0", 201, &[], b"");
    }

    #[test]
    fn a_fresh_tag_pushes_blob_then_manifest() {
        let _guard = token_guard();
        let http = MockHttp::new();
        script_monolithic(&http);
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let outcome = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        assert!(!outcome.skipped);
        assert_eq!(outcome.tag, TAG);
        assert_eq!(outcome.layer, layer());
        assert_eq!(outcome.manifest_digest, manifest_hex());
        assert_eq!(
            outcome.origin,
            format!("tfs+oci://{HOST}/{REPO}@sha256:{}", manifest_hex())
        );
        let writes = http.writes();
        assert_eq!(writes.len(), 5, "{writes:?}");
        // the mount attempts carry no body; the config PUT the canonical
        // `{}` bytes (zot validates the manifest's config reference);
        // the blob PUT the raw bytes; the manifest PUT the rendered
        // manifest
        assert!(writes[0].1.is_empty());
        assert_eq!(writes[1].1, b"{}");
        assert!(
            writes[1].0.contains(crate::EMPTY_CONFIG_DIGEST),
            "{}",
            writes[1].0
        );
        assert!(writes[2].1.is_empty());
        assert_eq!(writes[3].1, BYTES);
        assert_eq!(writes[4].1, manifest_bytes());
        // the blob PUT rode the session's Location with the digest query
        assert!(
            writes[3].0.starts_with(&format!(
                "https://{HOST}/v2/{REPO}/blobs/uploads/session-1?digest="
            )),
            "{}",
            writes[3].0
        );
    }

    #[test]
    fn a_mount_hit_skips_the_blob_upload() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_put("/manifests/1.0", 201, &[], b"");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        let writes = http.writes();
        assert_eq!(
            writes.len(),
            3,
            "the mounts deduped both blob PUTs: {writes:?}"
        );
        assert!(writes[2].0.contains("/manifests/1.0"), "{}", writes[2].0);
    }

    #[test]
    fn an_identical_republish_is_the_idempotent_skip() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            200,
            &[(
                "Docker-Content-Digest",
                format!("sha256:{}", manifest_hex()),
            )],
            &manifest_bytes(),
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let outcome = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        assert!(outcome.skipped);
        assert_eq!(http.writes().len(), 0, "nothing uploaded");
    }

    #[test]
    fn a_different_digest_at_the_tag_is_the_named_conflict() {
        let _guard = token_guard();
        let other = format!("sha256:{}", "9".repeat(64));
        // the registry's own digest header disagrees
        let http = MockHttp::new();
        http.push_get(
            "/manifests/1.0",
            200,
            &[("Docker-Content-Digest", other.clone())],
            &manifest_bytes(),
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap_err();
        assert!(
            matches!(
                &err,
                OciError::TagConflict { tag, existing, attempted, .. }
                if tag == TAG && existing == &other && attempted == &format!("sha256:{}", manifest_hex())
            ),
            "{err:?}"
        );
        assert!(err.to_string().contains("OciTagConflict"), "{err}");
        assert_eq!(http.writes().len(), 0, "the conflict uploads nothing");
        // no digest header: the served body's own hash decides
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 200, &[], b"{}");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap_err();
        assert!(matches!(err, OciError::TagConflict { .. }), "{err:?}");
    }

    #[test]
    fn the_push_dances_for_the_pull_push_scope() {
        let _guard = token_guard();
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
        http.push_get("/token?", 200, &[], br#"{"token":"tok-p"}"#);
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_put("/manifests/1.0", 201, &[], b"");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        let log = http.requests();
        let token_request = &log[1];
        assert!(
            token_request
                .0
                .contains("scope=repository%3Atebako-packages%2Ftool%3Apull%2Cpush"),
            "the push scope rode the token request: {}",
            token_request.0
        );
        assert_eq!(log[2].1, "Bearer tok-p", "the fresh token rides the retry");
        // the token cached under the push scope preempts the writes' 401s
        assert_eq!(log[3].1, "Bearer tok-p");
        assert_eq!(log[4].1, "Bearer tok-p");
        assert_eq!(log[5].1, "Bearer tok-p");
    }

    #[test]
    fn a_stale_push_token_rechallenges_exactly_once() {
        let _guard = token_guard();
        let ch = auth::BearerChallenge {
            realm: format!("https://{HOST}/token"),
            service: Some(HOST.to_string()),
            scope: None,
        };
        auth::store_token(HOST, &ch, Some(&push_scope_for(REPO)), "anonymous", "stale");
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
        http.push_get("/token?", 200, &[], br#"{"token":"fresh"}"#);
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_put("/manifests/1.0", 201, &[], b"");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        let log = http.requests();
        assert_eq!(log[0].1, "Bearer stale");
        assert_eq!(log[2].1, "Bearer fresh");
    }

    #[test]
    fn a_twice_refused_push_is_credential_required() {
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
        let err = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap_err();
        assert!(
            matches!(err, OciError::CredentialRequired { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn a_cross_host_upload_location_is_refused_before_any_put() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post(
            "/blobs/uploads/?mount=",
            202,
            &[("Location", "https://evil.example/v2/capture".to_string())],
            b"",
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap_err();
        assert!(matches!(err, OciError::ArtifactMalformed { .. }), "{err:?}");
        assert!(err.to_string().contains("cross-host"), "{err}");
        assert_eq!(http.writes().len(), 1, "only the mount POST went out");
    }

    #[test]
    fn an_upload_session_without_a_location_is_named() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 202, &[], b"");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap_err();
        assert!(matches!(err, OciError::ArtifactMalformed { .. }), "{err:?}");
        assert!(err.to_string().contains("Location"), "{err}");
    }

    #[test]
    fn a_same_host_absolute_location_rides() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post(
            "/blobs/uploads/?mount=",
            202,
            &[(
                "Location",
                format!("https://{HOST}/v2/{REPO}/blobs/uploads/abs-1?_state=xyz"),
            )],
            b"",
        );
        http.push_put(
            "/blobs/uploads/abs-1?_state=xyz&digest=sha256:",
            201,
            &[],
            b"",
        );
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_put("/manifests/1.0", 201, &[], b"");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        let writes = http.writes();
        assert!(
            writes[1].0.contains("?_state=xyz&digest=sha256:"),
            "the digest query appended to the existing query: {}",
            writes[1].0
        );
    }

    #[test]
    fn the_manifest_echo_digest_is_verified() {
        let _guard = token_guard();
        let http = MockHttp::new();
        // the manifest PUT answers 201 but echoes a digest that disagrees
        http.push_get("/manifests/2.0", 404, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_put(
            "/manifests/2.0",
            201,
            &[(
                "Docker-Content-Digest",
                format!("sha256:{}", "f".repeat(64)),
            )],
            b"",
        );
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = push_artifact(
            &client,
            HOST,
            REPO,
            "2.0",
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap_err();
        assert!(matches!(err, OciError::Sha256Mismatch { .. }), "{err:?}");
    }

    #[test]
    fn the_push_scope_spelling_is_pull_push() {
        assert_eq!(push_scope_for("o/r"), "repository:o/r:pull,push");
    }

    // ---- the transport-retry law (the module doc's): a throttled answer
    // waits the server's own schedule; a transport failure retries the
    // operation from zero, a blob leg on a FRESH upload session.

    #[test]
    fn a_broken_layer_put_retries_the_leg_on_a_fresh_session() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 404, &[], b"");
        // the well-known empty config mounts (no PUT — the fail_next_put
        // below then lands on the LAYER leg, the 200 MB class the retry
        // law exists for)
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_post(
            "/blobs/uploads/?mount=",
            202,
            &[("Location", format!("/v2/{REPO}/blobs/uploads/session-1"))],
            b"",
        );
        http.fail_next_put(FetchError::DownloadFailed("broken pipe".to_string()));
        // the retried leg takes a FRESH session — the PUT's stranded one
        // is never re-ridden
        http.push_post(
            "/blobs/uploads/?mount=",
            202,
            &[("Location", format!("/v2/{REPO}/blobs/uploads/session-2"))],
            b"",
        );
        http.push_put("/blobs/uploads/session-2?digest=sha256:", 201, &[], b"");
        http.push_put("/manifests/1.0", 201, &[], b"");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let outcome = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        assert!(!outcome.skipped);
        let writes = http.writes();
        // the two mount probes, the broken PUT, the fresh mount, the
        // layer, the manifest
        assert_eq!(writes.len(), 6, "{writes:?}");
        assert!(writes[2].0.contains("session-1"), "{}", writes[2].0);
        assert_eq!(writes[2].1, BYTES);
        assert!(writes[3].1.is_empty());
        assert!(writes[4].0.contains("session-2"), "{}", writes[4].0);
        assert_eq!(writes[4].1, BYTES);
    }

    #[test]
    fn a_throttled_manifest_put_waits_the_servers_schedule_and_retries() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        http.push_post(
            "/blobs/uploads/?mount=",
            202,
            &[("Location", format!("/v2/{REPO}/blobs/uploads/session-1"))],
            b"",
        );
        http.push_put("/blobs/uploads/session-1?digest=sha256:", 201, &[], b"");
        // Retry-After: 0 keeps the test off the clock while the backoff
        // law still runs (the hint is honored exactly)
        http.push_put("/manifests/1.0", 429, &[("Retry-After", "0".to_string())], b"");
        http.push_put("/manifests/1.0", 201, &[], b"");
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let outcome = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        assert!(!outcome.skipped);
        let writes = http.writes();
        assert_eq!(writes.len(), 5, "{writes:?}");
        assert_eq!(writes[3].1, manifest_bytes());
        assert_eq!(writes[4].1, manifest_bytes());
    }

    #[test]
    fn a_transport_failure_exhausts_the_publish_attempts() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.push_get("/manifests/1.0", 404, &[], b"");
        http.push_post("/blobs/uploads/?mount=", 201, &[], b"");
        for session in ["session-1", "session-2", "session-3"] {
            http.push_post(
                "/blobs/uploads/?mount=",
                202,
                &[("Location", format!("/v2/{REPO}/blobs/uploads/{session}"))],
                b"",
            );
            http.fail_next_put(FetchError::DownloadFailed(format!(
                "broken pipe on {session}"
            )));
        }
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let err = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap_err();
        assert!(
            matches!(err, OciError::Transport(FetchError::DownloadFailed(_))),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("after 3 attempts"), "{msg}");
        // the LAST failure is the reported one
        assert!(msg.contains("session-3"), "{msg}");
    }

    #[test]
    fn the_tag_check_get_retries_a_transport_failure() {
        let _guard = token_guard();
        let http = MockHttp::new();
        http.fail_next_get(FetchError::DownloadFailed("connection reset".to_string()));
        script_monolithic(&http);
        let creds = MockCreds::anonymous();
        let client = Client::new(&http, &creds);
        let outcome = push_artifact(
            &client,
            HOST,
            REPO,
            TAG,
            ArtifactClass::Payload,
            &annotations(),
            BYTES,
        )
        .unwrap();
        assert!(!outcome.skipped);
        assert_eq!(http.writes().len(), 5);
    }
}
