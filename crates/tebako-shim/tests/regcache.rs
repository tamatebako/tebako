//! The dispatch-time registry cache (spec 04 §2, roadmap 33): remote
//! registry forms resolve through tebako-resolve behind a per-ref cache
//! (24 h TTL, refresh, TEBAKO_OFFLINE = cache-or-named-error); `file://`
//! reads directly. Mock transports only — no network.

mod common;

use std::cell::Cell;
use std::collections::HashMap;

use common::*;
use tebako_resolve::{Fetcher, Transport};
use tebako_shim::regcache::{self, RefreshOutcome, RegistryFreshness};

const REGISTRY_YAML: &str = "schema_version: 1\npayloads:\n  - name: metanorma\n    kind: app\n    default: 1.2.3\n    versions:\n      - version: 1.2.3\n        platforms: universal\n        release: {ref: file:///metanorma-1.2.3.tfs}\n        entrypoints: [metanorma]\n";

/// A counting mock transport: every GET is recorded, answered from the
/// map (unknown URL → 404-class error, like the production mapping).
struct MockTransport {
    answers: HashMap<String, Vec<u8>>,
    hits: std::rc::Rc<Cell<u64>>,
}
impl MockTransport {
    fn with(answers: &[(&str, &str)], hits: std::rc::Rc<Cell<u64>>) -> MockTransport {
        MockTransport {
            answers: answers
                .iter()
                .map(|(u, b)| (u.to_string(), b.as_bytes().to_vec()))
                .collect(),
            hits,
        }
    }
}
impl Transport for MockTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>, tebako_http::FetchError> {
        self.hits.set(self.hits.get() + 1);
        self.answers
            .get(url)
            .cloned()
            .ok_or_else(|| tebako_http::FetchError::IndexUnavailable(url.to_string()))
    }
}

/// The GitHub default-branch form (service contents API): the contents
/// JSON points at a raw download URL.
fn github_fetcher(registry_bytes: &str) -> (Fetcher<MockTransport>, std::rc::Rc<Cell<u64>>) {
    let contents = r#"{"name":"tpkg-registry.yaml","download_url":"https://raw.example/o/r/HEAD/tpkg-registry.yaml"}"#;
    let hits = std::rc::Rc::new(Cell::new(0));
    let t = MockTransport::with(
        &[
            (
                "https://api.github.com/repos/o/r/contents/tpkg-registry.yaml",
                contents,
            ),
            (
                "https://raw.example/o/r/HEAD/tpkg-registry.yaml",
                registry_bytes,
            ),
        ],
        hits.clone(),
    );
    (Fetcher::with_transport(t), hits)
}

const GITHUB_REF: &str = "tfs:github:o/r";

fn cached_file(home: &std::path::Path) -> std::path::PathBuf {
    regcache::registries_dir(home).join(format!(
        "{}.yaml",
        tebako_resolve::sha256_hex(GITHUB_REF.as_bytes())
    ))
}

fn fetched_at_file(home: &std::path::Path) -> std::path::PathBuf {
    regcache::registries_dir(home).join(format!(
        "{}.fetched-at",
        tebako_resolve::sha256_hex(GITHUB_REF.as_bytes())
    ))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn file_refs_read_directly_and_never_touch_the_cache() {
    let tmp = TempDir::new("regcache-file");
    let home = tmp.path().join("home");
    let reg = tmp.path().join("tpkg-registry.yaml");
    std::fs::write(&reg, REGISTRY_YAML).unwrap();

    let (fetcher, _) = github_fetcher("unused");
    let registry =
        regcache::registry_for_with(&home, &tebako_http::file_url(&reg), &fetcher, false, now())
            .unwrap();
    assert_eq!(registry.payloads.len(), 1);
    // no cache directory was created and nothing was fetched
    assert!(!regcache::registries_dir(&home).exists());
    // a plain hand-authored path resolves the same way
    let registry =
        regcache::registry_for_with(&home, &reg.display().to_string(), &fetcher, false, now())
            .unwrap();
    assert_eq!(registry.payloads.len(), 1);
}

#[test]
fn remote_ref_fetches_once_then_serves_the_fresh_cache() {
    let tmp = TempDir::new("regcache-remote");
    let home = tmp.path().join("home");
    let (fetcher, hits) = github_fetcher(REGISTRY_YAML);

    // miss → fetch + publish
    let registry = regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    assert_eq!(
        registry.payload("metanorma").unwrap().default.as_deref(),
        Some("1.2.3")
    );
    assert!(
        cached_file(&home).is_file(),
        "the fetch populated the cache"
    );
    assert!(fetched_at_file(&home).is_file());
    let hits_after_first = hits.get();

    // fresh → the cache answers; no second fetch
    let registry = regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    assert_eq!(registry.payloads.len(), 1);
    assert_eq!(hits.get(), hits_after_first);
}

#[test]
fn stale_cache_refetches_and_renews() {
    let tmp = TempDir::new("regcache-stale");
    let home = tmp.path().join("home");
    let (fetcher, hits) = github_fetcher(REGISTRY_YAML);

    regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    let hits_after_first = hits.get();
    // backdate the cache beyond the TTL
    std::fs::write(
        fetched_at_file(&home),
        format!("{}\n", now() - regcache::REGISTRY_TTL_SECS - 60),
    )
    .unwrap();

    regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    assert!(hits.get() > hits_after_first, "a stale cache refetches");
    // the fetched-at marker was renewed
    let at: u64 = std::fs::read_to_string(fetched_at_file(&home))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(now() - at < 60, "renewed fetched-at");
}

#[test]
fn offline_is_cache_or_named_error() {
    let tmp = TempDir::new("regcache-offline");
    let home = tmp.path().join("home");
    let (fetcher, hits) = github_fetcher(REGISTRY_YAML);

    // no cache → the named error (and NO fetch attempt)
    let err = regcache::registry_for_with(&home, GITHUB_REF, &fetcher, true, now()).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("TEBAKO_OFFLINE"), "{}", err.message);
    assert!(err.message.contains("update-registries"), "{}", err.message);
    assert_eq!(hits.get(), 0);

    // prime a STALE cache (older than the TTL): offline reads it anyway
    regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    std::fs::write(fetched_at_file(&home), "1000\n").unwrap();
    let before = hits.get();
    let registry = regcache::registry_for_with(&home, GITHUB_REF, &fetcher, true, now()).unwrap();
    assert_eq!(registry.payloads.len(), 1);
    assert_eq!(hits.get(), before, "offline never fetches");
}

#[test]
fn refresh_force_renews_and_reports_outcomes() {
    let tmp = TempDir::new("regcache-refresh");
    let home = tmp.path().join("home");
    let (fetcher, hits) = github_fetcher(REGISTRY_YAML);

    let outcome = regcache::refresh_with(&home, GITHUB_REF, &fetcher).unwrap();
    assert_eq!(outcome, RefreshOutcome::Refreshed);
    assert!(cached_file(&home).is_file());
    let before = hits.get();

    let outcome = regcache::refresh_with(&home, GITHUB_REF, &fetcher).unwrap();
    assert_eq!(outcome, RefreshOutcome::Refreshed);
    assert!(hits.get() > before, "refresh always fetches");

    // file:// refs skip with a distinct outcome
    let reg = tmp.path().join("tpkg-registry.yaml");
    std::fs::write(&reg, REGISTRY_YAML).unwrap();
    let outcome = regcache::refresh_with(&home, &tebako_http::file_url(&reg), &fetcher).unwrap();
    assert_eq!(outcome, RefreshOutcome::LocalSkipped);

    // a fetched registry that no longer parses is a named error and the
    // old cache is not clobbered
    let (bad, _) = github_fetcher("schema_version: 99\n");
    let err = regcache::refresh_with(&home, GITHUB_REF, &bad).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(std::fs::read_to_string(cached_file(&home))
        .unwrap()
        .contains("metanorma"));
}

#[test]
fn prime_writes_the_cache_and_freshness_reports() {
    let tmp = TempDir::new("regcache-prime");
    let home = tmp.path().join("home");

    assert_eq!(
        regcache::freshness_at(&home, GITHUB_REF, now()),
        RegistryFreshness::Missing
    );

    regcache::prime(&home, GITHUB_REF, REGISTRY_YAML.as_bytes()).unwrap();
    assert!(cached_file(&home).is_file());
    match regcache::freshness_at(&home, GITHUB_REF, now()) {
        RegistryFreshness::Fresh(age) => assert!(age < 60),
        other => panic!("expected Fresh, got {other:?}"),
    }
    // stale beyond the TTL
    std::fs::write(
        fetched_at_file(&home),
        format!("{}\n", now() - regcache::REGISTRY_TTL_SECS - 3600),
    )
    .unwrap();
    match regcache::freshness_at(&home, GITHUB_REF, now()) {
        RegistryFreshness::Stale(age) => assert!(age >= regcache::REGISTRY_TTL_SECS),
        other => panic!("expected Stale, got {other:?}"),
    }
    // local + bad refs
    assert_eq!(
        regcache::freshness(&home, "file:///x/tpkg-registry.yaml"),
        RegistryFreshness::Local
    );
    assert!(matches!(
        regcache::freshness(&home, "not-a-ref"),
        RegistryFreshness::BadRef(_)
    ));

    // priming a file:// ref is a no-op (no new cache files beyond the
    // github ref's .yaml + .fetched-at)
    regcache::prime(&home, "file:///x/tpkg-registry.yaml", b"ignored").unwrap();
    assert_eq!(
        regcache::registries_dir(&home)
            .read_dir()
            .map(|d| d.count())
            .unwrap_or(0),
        2
    );
}

#[test]
fn registry_default_resolves_through_a_remote_registry() {
    let tmp = TempDir::new("regcache-default");
    let home = tmp.path().join("home");
    let (fetcher, _hits) = github_fetcher(REGISTRY_YAML);
    // the chain-level lookup with the injected fetcher (the production
    // path uses Fetcher::new — same code, live transport)
    let registry = regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    let payload = registry.payload("metanorma").unwrap();
    assert_eq!(payload.default.as_deref(), Some("1.2.3"));
}

/// A newer registry revision (the fetch side of the stale→fresh proof).
const REGISTRY_YAML_V2: &str = "schema_version: 1\npayloads:\n  - name: metanorma\n    kind: app\n    default: 1.2.4\n    versions:\n      - version: 1.2.4\n        platforms: universal\n        release: {ref: file:///metanorma-1.2.4.tfs}\n        entrypoints: [metanorma]\n";

/// A transport whose every GET fails (the airgapped day-2 shape).
fn failing_fetcher() -> (Fetcher<MockTransport>, std::rc::Rc<Cell<u64>>) {
    let hits = std::rc::Rc::new(Cell::new(0));
    (
        Fetcher::with_transport(MockTransport::with(&[], hits.clone())),
        hits,
    )
}

/// Backdate the cache's fetched-at marker beyond the TTL.
fn backdate_beyond_ttl(home: &std::path::Path) {
    std::fs::write(
        fetched_at_file(home),
        format!("{}\n", now() - regcache::REGISTRY_TTL_SECS - 60),
    )
    .unwrap();
}

#[test]
fn stale_cache_and_a_failed_fetch_serves_the_stale_bytes_loud() {
    // spec 05 §4 (roadmap 86): cache PRESENT but stale + the refresh
    // fetch failed → the stale cache SERVES, loud (stderr + journal) —
    // the trust anchor is the artifact's .sha256/.asc at fetch time,
    // never the registry's freshness.
    let tmp = TempDir::new("regcache-stale-serve");
    let home = tmp.path().join("home");
    let (fetcher, _) = github_fetcher(REGISTRY_YAML);
    regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    backdate_beyond_ttl(&home);

    let (failing, hits) = failing_fetcher();
    let registry = regcache::registry_for_with(&home, GITHUB_REF, &failing, false, now()).unwrap();
    // …the STALE bytes served (the primed 1.2.3 registry)…
    assert_eq!(
        registry.payload("metanorma").unwrap().default.as_deref(),
        Some("1.2.3")
    );
    assert!(hits.get() > 0, "the refresh was attempted first");
    // …LOUD: the journal line names the event, the ref, the failure…
    let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
    assert!(journal.contains("event=stale-registry-serve"), "{journal}");
    assert!(journal.contains("ref=tfs:github:o/r"), "{journal}");
    assert!(journal.contains("error="), "{journal}");
    // …and the cache is neither clobbered nor renewed.
    let at: u64 = std::fs::read_to_string(fetched_at_file(&home))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        now().saturating_sub(at) > regcache::REGISTRY_TTL_SECS,
        "the stale marker stays stale"
    );
    assert!(std::fs::read_to_string(cached_file(&home))
        .unwrap()
        .contains("1.2.3"));
}

#[test]
fn missing_cache_and_a_failed_fetch_is_the_named_error() {
    // Cache ABSENT + fetch failed: unchanged — the named resolution
    // error, no stale-serve journal line.
    let tmp = TempDir::new("regcache-absent-fail");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let (failing, hits) = failing_fetcher();
    let err = regcache::registry_for_with(&home, GITHUB_REF, &failing, false, now()).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(
        err.message.contains("cannot resolve registry"),
        "{}",
        err.message
    );
    assert!(hits.get() > 0, "the fetch was attempted");
    assert!(
        !home.join("journal.log").exists(),
        "no stale-serve journal line without a cache"
    );
}

#[test]
fn stale_cache_and_a_successful_fetch_serves_the_fresh_bytes() {
    // The healthy path is unchanged: a stale cache refetches and the
    // FRESH bytes serve (no stale-serve journal line).
    let tmp = TempDir::new("regcache-stale-fresh");
    let home = tmp.path().join("home");
    let (fetcher, _) = github_fetcher(REGISTRY_YAML);
    regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    backdate_beyond_ttl(&home);

    let (fetcher, _) = github_fetcher(REGISTRY_YAML_V2);
    let registry = regcache::registry_for_with(&home, GITHUB_REF, &fetcher, false, now()).unwrap();
    assert_eq!(
        registry.payload("metanorma").unwrap().default.as_deref(),
        Some("1.2.4"),
        "the refreshed bytes serve"
    );
    assert!(!home.join("journal.log").exists());
}

// ---------------------------------------------------------------------
// spec 38 §4 — the OCI registry forms' TTL mapping
// ---------------------------------------------------------------------

mod oci {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use tebako_http::{FetchError, RawResponse};

    /// The tier-3 docker consult reads `$DOCKER_CONFIG` — the developer
    /// machine's own config must not leak in (a shared empty dir; the
    /// absent file is the anonymous ride).
    fn isolate_docker_config() {
        let dir = std::env::temp_dir().join(format!(
            "tebako-shim-regcache-oci-docker-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("DOCKER_CONFIG", &dir);
    }

    /// The spec 38 §3 registry-class manifest for `yaml` (the contract
    /// oracle — the media types and the canonical empty config asserted
    /// verbatim). Returns (body, manifest hex digest).
    fn registry_manifest(yaml: &str) -> (Vec<u8>, String) {
        let layer_hex = tebako_resolve::sha256_hex(yaml.as_bytes());
        let body = format!(
            r#"{{"schemaVersion": 2, "mediaType": "application/vnd.oci.image.manifest.v1+json",
  "artifactType": "application/vnd.tebako.registry.v1",
  "config": {{"mediaType": "application/vnd.oci.empty.v1+json", "digest": "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fcd02fe2d1a42cb2d57b9b8b37d", "size": 2}},
  "layers": [{{"mediaType": "application/vnd.tebako.registry.v1+yaml", "digest": "sha256:{layer_hex}", "size": {}}}],
  "annotations": {{"org.opencontainers.image.title": "tpkg-registry.yaml"}}}}"#,
            yaml.len(),
        );
        let hex = tebako_resolve::sha256_hex(body.as_bytes());
        (body.into_bytes(), hex)
    }

    /// A mock registry whose served registry index can MOVE (the tag
    /// points at whatever `yaml` currently is): manifest and blob hits
    /// are counted separately — the re-serve law is "the digest check
    /// saves the blob pull".
    struct OciEndpoint {
        state: Rc<RefCell<OciState>>,
    }
    struct OciState {
        yaml: String,
        manifest_hits: u64,
        blob_hits: u64,
    }

    impl OciEndpoint {
        fn new(yaml: &str) -> OciEndpoint {
            OciEndpoint {
                state: Rc::new(RefCell::new(OciState {
                    yaml: yaml.to_string(),
                    manifest_hits: 0,
                    blob_hits: 0,
                })),
            }
        }
        fn set_yaml(&self, yaml: &str) {
            self.state.borrow_mut().yaml = yaml.to_string();
        }
        fn hits(&self) -> (u64, u64) {
            let st = self.state.borrow();
            (st.manifest_hits, st.blob_hits)
        }
    }

    impl Clone for OciEndpoint {
        fn clone(&self) -> OciEndpoint {
            OciEndpoint {
                state: self.state.clone(),
            }
        }
    }

    impl Transport for OciEndpoint {
        fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            Err(FetchError::IndexUnavailable(format!(
                "{url} (the OCI mock serves get_raw only)"
            )))
        }

        fn get_raw(
            &self,
            url: &str,
            _accept: Option<&str>,
            _header: Option<(&str, &str)>,
        ) -> Result<RawResponse, FetchError> {
            let mut st = self.state.borrow_mut();
            let (manifest, manifest_hex) = registry_manifest(&st.yaml);
            let blob_hex = tebako_resolve::sha256_hex(st.yaml.as_bytes());
            let base = "https://reg.example/v2/ns/tpkg-registry";
            if url == format!("{base}/manifests/latest")
                || url == format!("{base}/manifests/sha256:{manifest_hex}")
            {
                st.manifest_hits += 1;
                return Ok(RawResponse::new(
                    200,
                    vec![(
                        "Docker-Content-Digest".to_string(),
                        format!("sha256:{manifest_hex}"),
                    )],
                    manifest,
                ));
            }
            if url == format!("{base}/blobs/sha256:{blob_hex}") {
                st.blob_hits += 1;
                return Ok(RawResponse::new(
                    200,
                    Vec::new(),
                    st.yaml.clone().into_bytes(),
                ));
            }
            Ok(RawResponse::new(
                404,
                Vec::new(),
                br#"{"errors":[{"code":"MANIFEST_UNKNOWN"}]}"#.to_vec(),
            ))
        }
    }

    const OCI_REF: &str = "tfs+oci://reg.example/ns/tpkg-registry:latest";

    #[test]
    fn a_tag_ref_stale_cache_re_serves_when_the_digest_still_matches() {
        isolate_docker_config();
        let tmp = TempDir::new("regcache-oci-reserve");
        let home = tmp.path().join("home");
        let ep = OciEndpoint::new(REGISTRY_YAML);
        let fetcher = Fetcher::with_transport(ep.clone());
        let t0 = now();

        // miss → full pull (manifest + blob)
        regcache::registry_for_with(&home, OCI_REF, &fetcher, false, t0).unwrap();
        assert_eq!(ep.hits(), (1, 1));
        // fresh within the TTL: no requests at all
        regcache::registry_for_with(&home, OCI_REF, &fetcher, false, t0 + 3600).unwrap();
        assert_eq!(ep.hits(), (1, 1));
        // stale, digest unchanged: ONE manifest read, the blob pull is
        // saved, the cache serves (and the freshness marker renews)
        let registry =
            regcache::registry_for_with(&home, OCI_REF, &fetcher, false, t0 + 25 * 3600).unwrap();
        assert_eq!(ep.hits(), (2, 1));
        assert_eq!(
            registry.payload("metanorma").unwrap().default.as_deref(),
            Some("1.2.3")
        );
        // the renewed marker (real time — the touch wall-clocks) makes
        // the next in-TTL read request-free
        regcache::registry_for_with(&home, OCI_REF, &fetcher, false, now() + 3600).unwrap();
        assert_eq!(ep.hits(), (2, 1));
    }

    #[test]
    fn a_tag_ref_stale_cache_fetches_fresh_when_the_tag_moved() {
        isolate_docker_config();
        let tmp = TempDir::new("regcache-oci-moved");
        let home = tmp.path().join("home");
        let ep = OciEndpoint::new(REGISTRY_YAML);
        let fetcher = Fetcher::with_transport(ep.clone());
        let t0 = now();

        regcache::registry_for_with(&home, OCI_REF, &fetcher, false, t0).unwrap();
        assert_eq!(ep.hits(), (1, 1));

        // the tag moved: the digest check disagrees with the sidecar →
        // the full pull runs (its own manifest read + the blob)
        ep.set_yaml(REGISTRY_YAML_V2);
        let registry =
            regcache::registry_for_with(&home, OCI_REF, &fetcher, false, t0 + 25 * 3600).unwrap();
        assert_eq!(ep.hits(), (3, 2));
        assert_eq!(
            registry.payload("metanorma").unwrap().default.as_deref(),
            Some("1.2.4"),
            "the moved tag's bytes serve"
        );
        // and the NEW digest is the sidecar now: a further stale read
        // re-serves without a blob pull
        regcache::registry_for_with(&home, OCI_REF, &fetcher, false, t0 + 50 * 3600).unwrap();
        assert_eq!(ep.hits(), (4, 2));
    }

    #[test]
    fn a_digest_pulled_ref_is_cached_forever() {
        isolate_docker_config();
        let tmp = TempDir::new("regcache-oci-pinned");
        let home = tmp.path().join("home");
        let ep = OciEndpoint::new(REGISTRY_YAML);
        let (_, manifest_hex) = registry_manifest(REGISTRY_YAML);
        let pinned = format!("tfs+oci://reg.example/ns/tpkg-registry@sha256:{manifest_hex}");
        let fetcher = Fetcher::with_transport(ep.clone());
        let t0 = now();

        regcache::registry_for_with(&home, &pinned, &fetcher, false, t0).unwrap();
        assert_eq!(ep.hits(), (1, 1));
        // far past the TTL — even with the tag moved — the pinned cache
        // serves with ZERO requests (pinned-immutable, spec 38 §4)
        ep.set_yaml(REGISTRY_YAML_V2);
        let registry =
            regcache::registry_for_with(&home, &pinned, &fetcher, false, t0 + 365 * 86400).unwrap();
        assert_eq!(ep.hits(), (1, 1));
        assert_eq!(
            registry.payload("metanorma").unwrap().default.as_deref(),
            Some("1.2.3")
        );
        // and doctor never reports it stale
        assert!(matches!(
            regcache::freshness_at(&home, &pinned, t0 + 365 * 86400),
            RegistryFreshness::Fresh(_)
        ));
    }
}
