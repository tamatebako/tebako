//! Contract tests for the OCI pull path (spec 38): a mock distribution
//! endpoint behind the [`Transport`] seam answers the manifest/blob GETs,
//! and every leg asserts the spec's wire shape — the tag/digest selector
//! verification, the byte pin's fail-fast (the blob URL is never hit),
//! the §9 named errors, and the store-equivalence law (an OCI install
//! lands byte-identical store records to a `file://` install of the same
//! bytes, the origin marker excepted).
#![cfg(feature = "oci")]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use tebako_http::{FetchError, RawResponse};
use tebako_oci::model::{
    ANNOTATION_RUNTIME_SHARD, ANNOTATION_SIGNATURE_KEYID, ANNOTATION_SIGNATURE_SUBJECT,
    ANNOTATION_TITLE, EMPTY_CONFIG_DIGEST, EMPTY_CONFIG_MT,
};
use tebako_oci::{ArtifactClass, MANIFEST_MT};
use tebako_resolve::{
    execute_plan, sha256_hex, CommitReport, FetchItem, FetchPlan, Fetcher, InstallStatus, OciClass,
    PayloadCache, Reference, RegistryRef, ResolveError, Transport,
};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-resolve-oci-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The OCI credential chain's tier 3 reads `$DOCKER_CONFIG/config.json`
/// (else `~/.docker/…`) — the developer machine's own config (a desktop
/// `credsStore`) must not leak into these tests. Every test points the
/// variable at one shared empty dir (a single value, so the parallel
/// set is benign); the absent file is `Ok(None)`, the anonymous ride.
fn isolate_docker_config() {
    let dir = std::env::temp_dir().join(format!(
        "tebako-resolve-oci-docker-config-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    std::env::set_var("DOCKER_CONFIG", &dir);
}

/// One canned distribution answer.
#[derive(Clone)]
struct Answer {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// The mock registry: URL-keyed answers, every request logged (the
/// fail-fast assertions read the log). Unknown URLs answer 404 with a
/// distribution error body — the same classification input a real
/// registry serves.
struct DistMock {
    answers: HashMap<String, Answer>,
    seen: Mutex<Vec<String>>,
}

impl DistMock {
    fn new() -> Self {
        DistMock {
            answers: HashMap::new(),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn serve(mut self, url: &str, answer: Answer) -> Self {
        self.answers.insert(url.to_string(), answer);
        self
    }

    fn ok(self, url: &str, headers: &[(&str, String)], body: &[u8]) -> Self {
        self.serve(
            url,
            Answer {
                status: 200,
                headers: headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                body: body.to_vec(),
            },
        )
    }

    fn requested(&self, needle: &str) -> bool {
        self.seen.lock().unwrap().iter().any(|u| u.contains(needle))
    }
}

impl Transport for DistMock {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        // The plan pipeline's OCI stream rides `stream_distribution`,
        // whose Transport default buffers through `get` — serve the same
        // map (200 answers only; the distribution classification lives
        // on get_raw).
        self.seen.lock().unwrap().push(url.to_string());
        match self.answers.get(url) {
            Some(a) if a.status == 200 => Ok(a.body.clone()),
            _ => Err(FetchError::IndexUnavailable(format!(
                "{url} (the OCI contract mock serves mapped 200 answers only)"
            ))),
        }
    }

    fn get_raw(
        &self,
        url: &str,
        _accept: Option<&str>,
        _header: Option<(&str, &str)>,
    ) -> Result<RawResponse, FetchError> {
        self.seen.lock().unwrap().push(url.to_string());
        match self.answers.get(url) {
            Some(a) => Ok(RawResponse::new(
                a.status,
                a.headers.clone(),
                a.body.clone(),
            )),
            None => Ok(RawResponse::new(
                404,
                Vec::new(),
                br#"{"errors":[{"code":"MANIFEST_UNKNOWN","message":"mock: unknown url"}]}"#
                    .to_vec(),
            )),
        }
    }
}

/// The spec 38 §3 manifest for one artifact: the canonical empty config,
/// exactly one layer (the blob's digest + size), the served file name in
/// the required title annotation. Returns (body, manifest hex digest).
fn manifest_for(class: ArtifactClass, title: &str, blob: &[u8]) -> (Vec<u8>, String) {
    let layer_hex = sha256_hex(blob);
    let body = format!(
        r#"{{"schemaVersion": 2, "mediaType": "{MANIFEST_MT}",
  "artifactType": "{}",
  "config": {{"mediaType": "{EMPTY_CONFIG_MT}", "digest": "{EMPTY_CONFIG_DIGEST}", "size": 2}},
  "layers": [{{"mediaType": "{}", "digest": "sha256:{layer_hex}", "size": {}}}],
  "annotations": {{"{ANNOTATION_TITLE}": "{title}"}}}}"#,
        class.artifact_type(),
        class.layer_media_type(),
        blob.len(),
    );
    let hex = sha256_hex(body.as_bytes());
    (body.into_bytes(), hex)
}

const REGISTRY_YAML: &str = "schema_version: 1\npayloads:\n  - name: tool\n    kind: app\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: tfs+oci://reg.example/ns/tool:1.0}, entrypoints: [tool]}\n";

/// A mock registry serving `ns/tool`'s payload artifact (`tag` selector)
/// and `ns/tpkg-registry`'s registry index.
fn serving_mock(tag: &str, blob: &[u8]) -> (DistMock, String) {
    let (manifest, manifest_hex) = manifest_for(ArtifactClass::Payload, "tool-1.0.tfs", blob);
    let (reg_manifest, _) = manifest_for(
        ArtifactClass::RegistryIndex,
        "tpkg-registry.yaml",
        REGISTRY_YAML.as_bytes(),
    );
    let blob_hex = sha256_hex(blob);
    let reg_blob_hex = sha256_hex(REGISTRY_YAML.as_bytes());
    let mock = DistMock::new()
        .ok(
            &format!("https://reg.example/v2/ns/tool/manifests/{tag}"),
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/tool/manifests/sha256:{manifest_hex}"),
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/tool/blobs/sha256:{blob_hex}"),
            &[],
            blob,
        )
        .ok(
            "https://reg.example/v2/ns/tpkg-registry/manifests/latest",
            &[],
            &reg_manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/tpkg-registry/blobs/sha256:{reg_blob_hex}"),
            &[],
            REGISTRY_YAML.as_bytes(),
        );
    (mock, manifest_hex)
}

#[test]
fn a_tag_pull_resolves_the_manifest_then_streams_the_layer() {
    isolate_docker_config();
    let blob = b"oci-payload-bytes";
    let (mock, manifest_hex) = serving_mock("1.0", blob);
    let fetcher = Fetcher::with_transport(mock);
    let reference = Reference::parse("tfs+oci://reg.example/ns/tool:1.0").unwrap();
    let got = fetcher.fetch(&reference).unwrap();
    assert_eq!(got.bytes, blob);
    assert_eq!(got.sha256, sha256_hex(blob));
    // spec 38 §5.5: the origin records the digest-pinned form even for
    // a tag pull.
    assert_eq!(
        got.origin,
        format!("tfs+oci://reg.example/ns/tool@sha256:{manifest_hex}")
    );
}

#[test]
fn a_digest_pull_verifies_the_manifest_against_the_pin() {
    isolate_docker_config();
    let blob = b"oci-payload-bytes";
    let (mock, manifest_hex) = serving_mock("1.0", blob);
    let fetcher = Fetcher::with_transport(mock);
    let pinned = Reference::parse(&format!(
        "tfs+oci://reg.example/ns/tool@sha256:{manifest_hex}"
    ))
    .unwrap();
    assert_eq!(fetcher.fetch(&pinned).unwrap().bytes, blob);

    // A registry serving bytes whose hash disagrees with the digest
    // that named them is the verification failure (70 class) — the
    // attack the pin exists against. (An unknown digest selector is a
    // plain 404 → OciManifestNotFound, covered above.)
    let wrong_hex = "0".repeat(64);
    let wrong =
        Reference::parse(&format!("tfs+oci://reg.example/ns/tool@sha256:{wrong_hex}")).unwrap();
    let (mock, _) = serving_mock("1.0", blob);
    let (manifest, _) = manifest_for(ArtifactClass::Payload, "tool-1.0.tfs", blob);
    let mock = mock.ok(
        &format!("https://reg.example/v2/ns/tool/manifests/sha256:{wrong_hex}"),
        &[],
        &manifest,
    );
    let fetcher = Fetcher::with_transport(mock);
    assert!(matches!(
        fetcher.fetch(&wrong).unwrap_err(),
        ResolveError::Sha256Mismatch { .. }
    ));
}

#[test]
fn the_byte_pin_mismatch_never_requests_the_blob() {
    isolate_docker_config();
    let blob = b"oci-payload-bytes";
    let (mock, _) = serving_mock("1.0", blob);
    let mock = std::sync::Arc::new(mock);
    let fetcher = Fetcher::with_transport(SharedMock(mock.clone()));
    let pinned = Reference::parse(&format!(
        "tfs+oci://reg.example/ns/tool:1.0?sha256={}",
        "f".repeat(64)
    ))
    .unwrap();
    assert!(matches!(
        fetcher.fetch(&pinned).unwrap_err(),
        ResolveError::Sha256Mismatch { .. }
    ));
    assert!(mock.requested("/manifests/1.0"));
    assert!(
        !mock.requested("/blobs/"),
        "the blob URL must not be hit on a pin mismatch"
    );
    // and the matching pin pulls normally
    let pinned = Reference::parse(&format!(
        "tfs+oci://reg.example/ns/tool:1.0?sha256={}",
        sha256_hex(blob)
    ))
    .unwrap();
    assert_eq!(fetcher.fetch(&pinned).unwrap().bytes, blob);
}

/// Fetcher::with_transport consumes the transport; the shared wrapper
/// keeps the request log observable.
struct SharedMock(std::sync::Arc<DistMock>);
impl Transport for SharedMock {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.0.get(url)
    }
    fn get_raw(
        &self,
        url: &str,
        accept: Option<&str>,
        header: Option<(&str, &str)>,
    ) -> Result<RawResponse, FetchError> {
        self.0.get_raw(url, accept, header)
    }
}

#[test]
fn a_manifest_unknown_answer_is_the_named_error() {
    isolate_docker_config();
    let fetcher = Fetcher::with_transport(DistMock::new());
    let reference = Reference::parse("tfs+oci://reg.example/ns/absent:1.0").unwrap();
    let err = fetcher.fetch(&reference).unwrap_err();
    assert!(
        matches!(err, ResolveError::OciManifestNotFound { .. }),
        "{err}"
    );
}

#[test]
fn a_blob_unknown_answer_is_the_named_error() {
    isolate_docker_config();
    let blob = b"oci-payload-bytes";
    let (manifest, manifest_hex) = manifest_for(ArtifactClass::Payload, "tool-1.0.tfs", blob);
    let mock = DistMock::new()
        .ok(
            "https://reg.example/v2/ns/tool/manifests/1.0",
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        )
        .serve(
            &format!(
                "https://reg.example/v2/ns/tool/blobs/sha256:{}",
                sha256_hex(blob)
            ),
            Answer {
                status: 404,
                headers: Vec::new(),
                body: br#"{"errors":[{"code":"BLOB_UNKNOWN","message":"gone"}]}"#.to_vec(),
            },
        );
    let fetcher = Fetcher::with_transport(mock);
    let reference = Reference::parse("tfs+oci://reg.example/ns/tool:1.0").unwrap();
    let err = fetcher.fetch(&reference).unwrap_err();
    assert!(matches!(err, ResolveError::OciBlobUnknown { .. }), "{err}");
}

#[test]
fn a_shape_violation_is_the_named_malformed() {
    isolate_docker_config();
    // Two layers — the §3 shape law is exactly one.
    let blob = b"oci-payload-bytes";
    let layer_hex = sha256_hex(blob);
    let body = format!(
        r#"{{"schemaVersion": 2, "mediaType": "{MANIFEST_MT}",
  "artifactType": "application/vnd.tebako.tfs.v1",
  "config": {{"mediaType": "{EMPTY_CONFIG_MT}", "digest": "{EMPTY_CONFIG_DIGEST}", "size": 2}},
  "layers": [{{"mediaType": "application/vnd.tebako.tfs.v1+layer", "digest": "sha256:{layer_hex}", "size": {}}},
             {{"mediaType": "application/vnd.tebako.tfs.v1+layer", "digest": "sha256:{layer_hex}", "size": {}}}],
  "annotations": {{"{ANNOTATION_TITLE}": "tool-1.0.tfs"}}}}"#,
        blob.len(),
        blob.len(),
    );
    let mock = DistMock::new().ok(
        "https://reg.example/v2/ns/tool/manifests/1.0",
        &[],
        body.as_bytes(),
    );
    let fetcher = Fetcher::with_transport(mock);
    let reference = Reference::parse("tfs+oci://reg.example/ns/tool:1.0").unwrap();
    let err = fetcher.fetch(&reference).unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

#[test]
fn the_registry_index_pulls_as_a_registry_class_artifact() {
    isolate_docker_config();
    let blob = b"oci-payload-bytes";
    let (mock, _) = serving_mock("1.0", blob);
    let fetcher = Fetcher::with_transport(mock);
    let r = RegistryRef::parse("tfs+oci://reg.example/ns/tpkg-registry:latest").unwrap();
    let registry = fetcher.resolve_registry(&r).unwrap();
    assert_eq!(registry.payloads[0].name, "tool");

    // spec 38 §4: the re-serve digest flows out of the pull itself.
    let (mock, _) = serving_mock("1.0", blob);
    let fetcher = Fetcher::with_transport(mock);
    let (bytes, digest) = fetcher.fetch_registry_with_oci_digest(&r).unwrap();
    assert_eq!(bytes, REGISTRY_YAML.as_bytes());
    let digest = digest.expect("an OCI registry pull records its manifest digest");
    assert_eq!(digest.len(), 64);
    // and the re-serve check resolves the same digest
    let (mock, _) = serving_mock("1.0", blob);
    let fetcher = Fetcher::with_transport(mock);
    assert_eq!(
        fetcher.oci_manifest_digest(&r).unwrap().as_deref(),
        Some(digest.as_str())
    );
}

#[test]
fn the_store_records_match_a_file_install_byte_for_byte() {
    isolate_docker_config();
    let blob = b"oci-payload-bytes";
    let dir = scratch("store");

    // The OCI install.
    let (mock, _) = serving_mock("1.0", blob);
    let fetcher = Fetcher::with_transport(mock);
    let reference = Reference::parse("tfs+oci://reg.example/ns/tool:1.0").unwrap();
    let oci_cache = PayloadCache::with_root(dir.join("oci"));
    let (entry, status) = oci_cache
        .install("tool", "1.0", None, || fetcher.fetch(&reference))
        .unwrap();
    assert_eq!(status, InstallStatus::Installed);
    assert_eq!(fs::read(&entry.path).unwrap(), blob);

    // The file:// install of the same bytes.
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tool.tfs"), blob).unwrap();
    let file_ref = Reference::parse(&tebako_http::file_url(&mirror.join("tool.tfs"))).unwrap();
    let file_fetcher = Fetcher::new();
    let file_cache = PayloadCache::with_root(dir.join("file"));
    let (file_entry, _) = file_cache
        .install("tool", "1.0", None, || file_fetcher.fetch(&file_ref))
        .unwrap();

    // Store equivalence: the payload bytes and the trust-anchor sidecar
    // are identical; only the origin marker differs (and the OCI origin
    // is the digest-pinned form, spec 38 §5.5).
    assert_eq!(
        fs::read(&entry.path).unwrap(),
        fs::read(&file_entry.path).unwrap()
    );
    let sidecar = |root: &str| {
        fs::read_to_string(dir.join(root).join("payloads/tool/1.0.tfs.sha256")).unwrap()
    };
    assert_eq!(sidecar("oci"), sidecar("file"));
    let origin = fs::read_to_string(dir.join("oci/payloads/tool/1.0.tfs.origin")).unwrap();
    assert!(
        origin
            .trim()
            .starts_with("tfs+oci://reg.example/ns/tool@sha256:"),
        "{origin}"
    );
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// The blksum sidecar over OCI (spec 39 §8)
// ---------------------------------------------------------------------

/// A mock registry serving the blksum sidecar sibling of `ns/tool`'s
/// image blob: the sibling digest-tag manifest (blksum class) + the
/// sidecar blob itself. Returns the mock, the image's hex digest, and
/// the blksum document the sidecar carries.
fn blksum_mock(image: &[u8]) -> (DistMock, String, tpkg::lazy::Blksum) {
    let image_hex = sha256_hex(image);
    let blksum = tpkg::lazy::Blksum::from_image_bytes(image);
    let sidecar = blksum.render();
    let (manifest, manifest_hex) = manifest_for(
        ArtifactClass::Blksum,
        &tebako_oci::blksum_tag(&image_hex),
        sidecar.as_bytes(),
    );
    let sidecar_hex = sha256_hex(sidecar.as_bytes());
    let mock = DistMock::new()
        .ok(
            &format!(
                "https://reg.example/v2/ns/tool/manifests/{}",
                tebako_oci::blksum_tag(&image_hex)
            ),
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/tool/blobs/sha256:{sidecar_hex}"),
            &[],
            sidecar.as_bytes(),
        );
    (mock, image_hex, blksum)
}

#[test]
fn the_blksum_sidecar_resolves_through_the_sibling_digest_tag() {
    isolate_docker_config();
    let image = b"the lazy env image bytes, more than one group would need";
    let (mock, image_hex, published) = blksum_mock(image);
    let mock = std::sync::Arc::new(mock);
    let got = tebako_resolve::oci::fetch_blksum_sidecar(
        &SharedMock(mock.clone()),
        "reg.example",
        "ns/tool",
        &image_hex,
        None,
    )
    .unwrap();
    let Some((blksum, origin)) = got else {
        panic!("the published sidecar must resolve")
    };
    assert_eq!(blksum.sha256, published.sha256);
    assert_eq!(blksum.size_bytes, image.len() as u64);
    assert_eq!(blksum.group_count(), published.group_count());
    assert!(
        origin.starts_with("tfs+oci://reg.example/ns/tool@sha256:"),
        "{origin}"
    );
    assert!(mock.requested(&format!("/manifests/sha256-{image_hex}.blksum.json")));
}

#[test]
fn a_missing_blksum_sidecar_is_the_loud_fallback_signal() {
    isolate_docker_config();
    let got = tebako_resolve::oci::fetch_blksum_sidecar(
        &DistMock::new(),
        "reg.example",
        "ns/tool",
        &"a".repeat(64),
        None,
    )
    .unwrap();
    assert!(
        got.is_none(),
        "blksum-missing is Ok(None) — the spec 39 §3 loud eager fallback is the caller's law"
    );
}

#[test]
fn a_blksum_tag_naming_a_non_blksum_artifact_is_malformed() {
    isolate_docker_config();
    let image_hex = "b".repeat(64);
    let blob = b"not a blksum at all";
    let (manifest, _) = manifest_for(ArtifactClass::Payload, "tool-1.0.tfs", blob);
    let mock = DistMock::new().ok(
        &format!("https://reg.example/v2/ns/tool/manifests/sha256-{image_hex}.blksum.json"),
        &[],
        &manifest,
    );
    let err = tebako_resolve::oci::fetch_blksum_sidecar(
        &mock,
        "reg.example",
        "ns/tool",
        &image_hex,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

#[test]
fn a_sidecar_pinning_a_different_image_is_the_named_mismatch() {
    isolate_docker_config();
    // The sidecar was published for image A but rides image B's sibling
    // tag — a torn publish, never a silent serve.
    let image_a = b"image A bytes";
    let blksum_a = tpkg::lazy::Blksum::from_image_bytes(image_a);
    let sidecar = blksum_a.render();
    let image_b_hex = "c".repeat(64);
    let (manifest, _) = manifest_for(
        ArtifactClass::Blksum,
        &tebako_oci::blksum_tag(&image_b_hex),
        sidecar.as_bytes(),
    );
    let sidecar_hex = sha256_hex(sidecar.as_bytes());
    let mock = DistMock::new()
        .ok(
            &format!("https://reg.example/v2/ns/tool/manifests/sha256-{image_b_hex}.blksum.json"),
            &[],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/tool/blobs/sha256:{sidecar_hex}"),
            &[],
            sidecar.as_bytes(),
        );
    let err = tebako_resolve::oci::fetch_blksum_sidecar(
        &mock,
        "reg.example",
        "ns/tool",
        &image_b_hex,
        None,
    )
    .unwrap_err();
    assert!(matches!(err, ResolveError::Sha256Mismatch { .. }), "{err}");
}

#[test]
fn a_torn_sidecar_document_is_malformed_by_name() {
    isolate_docker_config();
    let image_hex = "d".repeat(64);
    let sidecar = b"this is not a blksum document";
    let (manifest, _) = manifest_for(
        ArtifactClass::Blksum,
        &tebako_oci::blksum_tag(&image_hex),
        sidecar,
    );
    let sidecar_hex = sha256_hex(sidecar);
    let mock = DistMock::new()
        .ok(
            &format!("https://reg.example/v2/ns/tool/manifests/sha256-{image_hex}.blksum.json"),
            &[],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/tool/blobs/sha256:{sidecar_hex}"),
            &[],
            sidecar,
        );
    let err = tebako_resolve::oci::fetch_blksum_sidecar(
        &mock,
        "reg.example",
        "ns/tool",
        &image_hex,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

// ---------------------------------------------------------------------
// The runtime-bundle lane (spec 38 §5.4)
// ---------------------------------------------------------------------

/// JSON-string escape for annotation values embedded in a manifest body
/// (the shard annotation is JSON-in-JSON).
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// [`manifest_for`] plus extra annotations (the spec 38 §5.4 shard ride,
/// the §3 signature cross-checks).
fn manifest_annotated(
    class: ArtifactClass,
    title: &str,
    blob: &[u8],
    extra: &[(&str, &str)],
) -> (Vec<u8>, String) {
    let layer_hex = sha256_hex(blob);
    let extras = extra
        .iter()
        .map(|(k, v)| format!(r#", "{k}": "{}""#, json_escape(v)))
        .collect::<String>();
    let body = format!(
        r#"{{"schemaVersion": 2, "mediaType": "{MANIFEST_MT}",
  "artifactType": "{}",
  "config": {{"mediaType": "{EMPTY_CONFIG_MT}", "digest": "{EMPTY_CONFIG_DIGEST}", "size": 2}},
  "layers": [{{"mediaType": "{}", "digest": "sha256:{layer_hex}", "size": {}}}],
  "annotations": {{"{ANNOTATION_TITLE}": "{title}"{extras}}}}}"#,
        class.artifact_type(),
        class.layer_media_type(),
        blob.len(),
    );
    let hex = sha256_hex(body.as_bytes());
    (body.into_bytes(), hex)
}

/// A spec 36 §3 shard (the annotation's payload is opaque to the OCI
/// lane — the shim feeds it to spec 36 §4's chain verbatim).
const SHARD_JSON: &str = concat!(
    r#"{"schema_version":1,"name":"ruby","version":"3.3.9","tebako":"0.16.32","#,
    r#""triplet":"aarch64-macos","bundle":{"filename":"#,
    r#""tebako-runtime-0.16.32-ruby-3.3.9-aarch64-macos.tar.gz","sha256":""#,
    r#""0000000000000000000000000000000000000000000000000000000000000000"}}"#
);

/// A mock registry serving `ns/runtime`'s runtime-bundle artifact: the
/// manifest under the RuntimeBundle class with the required
/// `org.tebako.runtime.shard` annotation, plus the bundle blob.
fn bundle_mock(tag: &str, blob: &[u8], shard: &str) -> (DistMock, String) {
    let (manifest, manifest_hex) = manifest_annotated(
        ArtifactClass::RuntimeBundle,
        "tebako-runtime-0.16.32-ruby-3.3.9-aarch64-macos.tar.gz",
        blob,
        &[(ANNOTATION_RUNTIME_SHARD, shard)],
    );
    let blob_hex = sha256_hex(blob);
    let mock = DistMock::new()
        .ok(
            &format!("https://reg.example/v2/ns/runtime/manifests/{tag}"),
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/runtime/manifests/sha256:{manifest_hex}"),
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/runtime/blobs/sha256:{blob_hex}"),
            &[],
            blob,
        );
    (mock, manifest_hex)
}

#[test]
fn the_runtime_bundle_card_replaces_the_shard_fetch() {
    isolate_docker_config();
    let blob = b"the runtime bundle tar.gz bytes";
    let (mock, manifest_hex) = bundle_mock("3.3.9", blob, SHARD_JSON);
    let reference = Reference::parse("tfs+oci://reg.example/ns/runtime:3.3.9").unwrap();
    let card = tebako_resolve::oci::resolve_runtime_bundle(&mock, &reference, None).unwrap();
    let blob_hex = sha256_hex(blob);
    assert_eq!(card.shard, SHARD_JSON);
    assert_eq!(card.layer_digest, format!("sha256:{blob_hex}"));
    assert_eq!(card.layer_sha256, blob_hex);
    assert_eq!(card.layer_size, blob.len() as u64);
    assert_eq!(card.manifest_digest, manifest_hex);
    // spec 38 §5.5: the origin is the digest-pinned form even for a tag
    // pull — the plan re-resolves by it (the tag's mutability never
    // reaches the stream).
    assert_eq!(
        card.origin,
        format!("tfs+oci://reg.example/ns/runtime@sha256:{manifest_hex}")
    );
}

#[test]
fn a_bundle_manifest_without_the_shard_annotation_is_a_torn_publish() {
    isolate_docker_config();
    let blob = b"bundle bytes";
    let reference = Reference::parse("tfs+oci://reg.example/ns/runtime:3.3.9").unwrap();

    // The RuntimeBundle class without the shard annotation: no best-
    // effort read, malformed by name.
    let (manifest, manifest_hex) = manifest_for(
        ArtifactClass::RuntimeBundle,
        "tebako-runtime-0.16.32-ruby-3.3.9-aarch64-macos.tar.gz",
        blob,
    );
    let mock = DistMock::new().ok(
        "https://reg.example/v2/ns/runtime/manifests/3.3.9",
        &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
        &manifest,
    );
    let err = tebako_resolve::oci::resolve_runtime_bundle(&mock, &reference, None).unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );

    // An annotation that is whitespace-only is no shard ride either.
    let (manifest, manifest_hex) = manifest_annotated(
        ArtifactClass::RuntimeBundle,
        "tebako-runtime-0.16.32-ruby-3.3.9-aarch64-macos.tar.gz",
        blob,
        &[(ANNOTATION_RUNTIME_SHARD, "  ")],
    );
    let mock = DistMock::new().ok(
        "https://reg.example/v2/ns/runtime/manifests/3.3.9",
        &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
        &manifest,
    );
    let err = tebako_resolve::oci::resolve_runtime_bundle(&mock, &reference, None).unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

#[test]
fn a_payload_class_manifest_at_a_bundle_reference_is_malformed() {
    isolate_docker_config();
    let blob = b"payload bytes";
    let (manifest, manifest_hex) = manifest_for(ArtifactClass::Payload, "tool-1.0.tfs", blob);
    let mock = DistMock::new().ok(
        "https://reg.example/v2/ns/runtime/manifests/3.3.9",
        &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
        &manifest,
    );
    let reference = Reference::parse("tfs+oci://reg.example/ns/runtime:3.3.9").unwrap();
    let err = tebako_resolve::oci::resolve_runtime_bundle(&mock, &reference, None).unwrap_err();
    // §3's shape law per lane: a payload artifact is not a runtime
    // bundle, malformed by name — never a class confusion.
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

#[test]
fn the_bundle_byte_pin_mismatch_fails_before_any_blob_request() {
    isolate_docker_config();
    let blob = b"bundle bytes";
    let (mock, _) = bundle_mock("3.3.9", blob, SHARD_JSON);
    let mock = std::sync::Arc::new(mock);
    let reference = Reference::parse(&format!(
        "tfs+oci://reg.example/ns/runtime:3.3.9?sha256={}",
        "f".repeat(64)
    ))
    .unwrap();
    let err =
        tebako_resolve::oci::resolve_runtime_bundle(&SharedMock(mock.clone()), &reference, None)
            .unwrap_err();
    assert!(matches!(err, ResolveError::Sha256Mismatch { .. }), "{err}");
    assert!(mock.requested("/manifests/3.3.9"));
    assert!(
        !mock.requested("/blobs/"),
        "the blob URL must not be hit on a pin mismatch"
    );
}

#[test]
fn a_non_sha256_layer_digest_is_malformed_by_name() {
    isolate_docker_config();
    let blob = b"bundle bytes";
    let body = format!(
        r#"{{"schemaVersion": 2, "mediaType": "{MANIFEST_MT}",
  "artifactType": "{}",
  "config": {{"mediaType": "{EMPTY_CONFIG_MT}", "digest": "{EMPTY_CONFIG_DIGEST}", "size": 2}},
  "layers": [{{"mediaType": "{}", "digest": "sha512:{}", "size": {}}}],
  "annotations": {{"{ANNOTATION_TITLE}": "bundle.tar.gz", "{ANNOTATION_RUNTIME_SHARD}": "{}"}}}}"#,
        ArtifactClass::RuntimeBundle.artifact_type(),
        ArtifactClass::RuntimeBundle.layer_media_type(),
        "a".repeat(128),
        blob.len(),
        json_escape(SHARD_JSON),
    );
    let mock = DistMock::new().ok(
        "https://reg.example/v2/ns/runtime/manifests/3.3.9",
        &[],
        body.as_bytes(),
    );
    let reference = Reference::parse("tfs+oci://reg.example/ns/runtime:3.3.9").unwrap();
    let err = tebako_resolve::oci::resolve_runtime_bundle(&mock, &reference, None).unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

// ---------------------------------------------------------------------
// The signature sibling over OCI (spec 38 §3)
// ---------------------------------------------------------------------

/// A mock registry serving the signature sibling of `ns/runtime`'s
/// bundle blob: the deterministic digest-tag manifest (Signature class,
/// the subject/keyid annotations) plus the `.asc` blob. `subject_hex`
/// is what the manifest DECLARES; `tag_hex` is the blob the sibling tag
/// names — a torn publish separates the two.
fn signature_mock(subject_hex: &str, tag_hex: &str, keyid: Option<&str>, asc: &[u8]) -> DistMock {
    let tag = tebako_oci::signature_tag(tag_hex);
    let subject = format!("sha256:{subject_hex}");
    let mut extra = vec![(ANNOTATION_SIGNATURE_SUBJECT, subject.as_str())];
    if let Some(k) = keyid {
        extra.push((ANNOTATION_SIGNATURE_KEYID, k));
    }
    let (manifest, manifest_hex) = manifest_annotated(ArtifactClass::Signature, &tag, asc, &extra);
    let asc_hex = sha256_hex(asc);
    DistMock::new()
        .ok(
            &format!("https://reg.example/v2/ns/runtime/manifests/{tag}"),
            &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
            &manifest,
        )
        .ok(
            &format!("https://reg.example/v2/ns/runtime/blobs/sha256:{asc_hex}"),
            &[],
            asc,
        )
}

#[test]
fn the_signature_sibling_resolves_by_its_digest_tag() {
    isolate_docker_config();
    let blob_hex = "b".repeat(64);
    let asc = b"-----BEGIN PGP SIGNATURE-----\nmock armor\n";
    let mock = signature_mock(&blob_hex, &blob_hex, Some("0123456789abcdef"), asc);
    // The declared keyid matches case-insensitively (the release's
    // spelling is lowercase, the annotation rides the publish).
    let (bytes, origin) = tebako_resolve::oci::fetch_signature(
        &mock,
        "reg.example",
        "ns/runtime",
        &blob_hex,
        Some("0123456789ABCDEF"),
        None,
    )
    .unwrap();
    assert_eq!(bytes, asc);
    assert!(
        origin.starts_with("tfs+oci://reg.example/ns/runtime@sha256:"),
        "{origin}"
    );
}

#[test]
fn a_signature_naming_another_subject_is_a_torn_publish() {
    isolate_docker_config();
    let tag_hex = "b".repeat(64);
    let other_hex = "c".repeat(64);
    let mock = signature_mock(&other_hex, &tag_hex, None, b"asc bytes");
    let err = tebako_resolve::oci::fetch_signature(
        &mock,
        "reg.example",
        "ns/runtime",
        &tag_hex,
        None,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

#[test]
fn a_signature_keyid_disagreeing_with_the_declaration_is_a_torn_publish() {
    isolate_docker_config();
    let blob_hex = "b".repeat(64);
    let mock = signature_mock(&blob_hex, &blob_hex, Some("ffffffffffffffff"), b"asc bytes");
    let err = tebako_resolve::oci::fetch_signature(
        &mock,
        "reg.example",
        "ns/runtime",
        &blob_hex,
        Some("0123456789abcdef"),
        None,
    )
    .unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
}

#[test]
fn a_declared_signature_sibling_that_names_nothing_is_manifest_not_found() {
    isolate_docker_config();
    let err = tebako_resolve::oci::fetch_signature(
        &DistMock::new(),
        "reg.example",
        "ns/runtime",
        &"b".repeat(64),
        None,
        None,
    )
    .unwrap_err();
    // The caller maps a DECLARED signature that does not fetch to the
    // 71 refusal (spec 09 §4) — the adapter surfaces the plain 404.
    assert!(
        matches!(err, ResolveError::OciManifestNotFound { .. }),
        "{err}"
    );
}

#[test]
fn the_plan_streams_a_runtime_bundle_item_under_its_own_class_law() {
    isolate_docker_config();
    let blob = b"the runtime bundle tar.gz bytes, streamed through the plan";
    let (mock, manifest_hex) = bundle_mock("3.3.9", blob, SHARD_JSON);
    let dir = scratch("plan-bundle");
    let dest = dir.join("bundle.tar.gz");
    let dest2 = dest.clone();
    let origin_note = dir.join("origin");
    let origin_note2 = origin_note.clone();
    // The plan item re-resolves by the card's manifest digest — the
    // tag's mutability never reaches the stream (spec 38 §5.4).
    let item = FetchItem {
        display: "tebako-runtime-0.16.32-ruby-3.3.9-aarch64-macos.tar.gz".to_string(),
        reference: Reference::parse(&format!(
            "tfs+oci://reg.example/ns/runtime@sha256:{manifest_hex}"
        ))
        .unwrap(),
        sha256_pin: Some(sha256_hex(blob)),
        size_hint: Some(blob.len() as u64),
        tmp_dir: dir.join("tmp"),
        registry_alias: None,
        oci_class: OciClass::RuntimeBundle,
        lazy: false,
        commit: Box::new(move |staged| {
            fs::rename(staged.tmp, &dest2).unwrap();
            fs::write(&origin_note2, staged.origin).unwrap();
            Ok(CommitReport { line: None })
        }),
    };
    execute_plan::<_, Vec<u8>>(&mock, FetchPlan::new("runtime test", vec![item]), 1, None).unwrap();
    assert_eq!(fs::read(&dest).unwrap(), blob);
    assert_eq!(
        fs::read_to_string(&origin_note).unwrap(),
        format!("tfs+oci://reg.example/ns/runtime@sha256:{manifest_hex}")
    );

    // The class law on the same arm: a PAYLOAD item pointed at the
    // bundle-class manifest is malformed by name, and nothing lands.
    let item = FetchItem {
        display: "wrong-class.tfs".to_string(),
        reference: Reference::parse(&format!(
            "tfs+oci://reg.example/ns/runtime@sha256:{manifest_hex}"
        ))
        .unwrap(),
        sha256_pin: None,
        size_hint: None,
        tmp_dir: dir.join("tmp"),
        registry_alias: None,
        oci_class: OciClass::Payload,
        lazy: false,
        commit: Box::new(|_| Ok(CommitReport { line: None })),
    };
    let err =
        execute_plan::<_, Vec<u8>>(&mock, FetchPlan::new("runtime test", vec![item]), 1, None)
            .unwrap_err();
    assert!(
        matches!(err, ResolveError::OciArtifactMalformed { .. }),
        "{err}"
    );
    let _ = fs::remove_dir_all(&dir);
}
