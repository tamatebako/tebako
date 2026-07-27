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
    let registry = regcache::registry_for_with(
        &home,
        &format!("file://{}", reg.display()),
        &fetcher,
        false,
        now(),
    )
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
    let outcome =
        regcache::refresh_with(&home, &format!("file://{}", reg.display()), &fetcher).unwrap();
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
