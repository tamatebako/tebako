//! End-to-end legs through the public API, no network: `file://` mirrors
//! (spec 04 §3) and a local git fixture cloned by the real gix adapter.

use std::fs;
use std::path::PathBuf;

use tebako_resolve::{
    fetch_and_cache, sha256_hex, Fetcher, InstallStatus, PayloadCache, Reference, ResolveError,
};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-resolve-it-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A canonical file:// URL for a local path: forward slashes, with the
/// third slash before an absolute unix path or the drive path on
/// Windows (`file:///tmp/x` / `file:///C:/x`). `format!("file://{}")`
/// over a Display path only looks right on unix.
fn file_url(path: &std::path::Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

#[test]
fn file_mirror_fetch_and_cache() {
    let dir = scratch("file");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tool.tfs"), b"payload-bytes").unwrap();
    let cache = PayloadCache::with_root(dir.join("cache"));

    let reference = Reference::parse(&format!("{}/tool.tfs", file_url(&mirror))).unwrap();
    let (entry, status) = fetch_and_cache(&cache, &reference, "tool", "1.2.3", None).unwrap();
    assert_eq!(status, InstallStatus::Installed);
    assert_eq!(fs::read(&entry.path).unwrap(), b"payload-bytes");
    assert_eq!(entry.sha256, sha256_hex(b"payload-bytes"));

    let (_entry, status) = fetch_and_cache(&cache, &reference, "tool", "1.2.3", None).unwrap();
    assert_eq!(status, InstallStatus::Hit);

    // A registry-supplied anchor matching the artifact also installs.
    let (entry, status) = fetch_and_cache(
        &cache,
        &reference,
        "tool",
        "2.0",
        Some(&sha256_hex(b"payload-bytes")),
    )
    .unwrap();
    assert_eq!(status, InstallStatus::Installed);
    assert!(entry.path.ends_with("payloads/tool/2.0.tfs"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn pinned_mirror_mismatch_caches_nothing() {
    let dir = scratch("mismatch");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tool.tfs"), b"payload-bytes").unwrap();
    let cache = PayloadCache::with_root(dir.join("cache"));

    let reference = Reference::parse(&format!(
        "{}/tool.tfs?sha256={}",
        file_url(&mirror),
        "0".repeat(64)
    ))
    .unwrap();
    let err = fetch_and_cache(&cache, &reference, "tool", "1.0", None).unwrap_err();
    assert!(matches!(err, ResolveError::Sha256Mismatch { .. }));
    assert!(!dir.join("cache/payloads/tool/1.0.tfs").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn git_reference_fetches_through_the_real_adapter() {
    let dir = scratch("git");
    let fixture = dir.join("fixture.git");
    // A bare repo holding two payload images on a versioned branch.
    let repo = gix::init_bare(&fixture).unwrap();
    drop(repo);
    // gix commit needs an identity; CI runners have no ambient git
    // config. Pin it in-repo (append — init already wrote [core]) and
    // re-open so the config is re-read.
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .append(true)
        .open(fixture.join("config"))
        .unwrap()
        .write_all(b"[user]\n\tname = tebako-test\n\temail = tebako-test@example.org\n")
        .unwrap();
    let repo = gix::open(&fixture).unwrap();
    let blob = repo.write_blob(b"git-payload").unwrap();
    let empty = gix::hash::ObjectId::empty_tree(repo.object_hash());
    let tree = {
        let mut ed = repo.edit_tree(empty).unwrap();
        ed.upsert("images/tool.tfs", gix::object::tree::EntryKind::Blob, blob)
            .unwrap();
        ed.write().unwrap()
    };
    repo.commit("refs/heads/v1", "v1", tree, gix::commit::NO_PARENT_IDS)
        .unwrap();

    let reference = Reference::Git {
        url: fixture.to_string_lossy().into_owned(),
        git_ref: Some("v1".to_string()),
        path: Some("images/tool.tfs".to_string()),
        sha256: Some(sha256_hex(b"git-payload")),
    };
    let fetcher = Fetcher::new();
    let got = fetcher.fetch(&reference).unwrap();
    assert_eq!(got.bytes, b"git-payload");

    let cache = PayloadCache::with_root(dir.join("cache"));
    let (entry, status) = fetch_and_cache(&cache, &reference, "tool", "1.0", None).unwrap();
    assert_eq!(status, InstallStatus::Installed);
    assert_eq!(fs::read(&entry.path).unwrap(), b"git-payload");
    // The origin marker records the full reference, pin included.
    let origin = fs::read_to_string(dir.join("cache/payloads/tool/1.0.tfs.origin")).unwrap();
    assert!(origin.contains("tfs+git://") && origin.contains("#images/tool.tfs"));
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// Registry resolution (spec 04 §2) — one location per form, no fallback.
// ---------------------------------------------------------------------

use std::collections::HashMap;
use tebako_http::FetchError;
use tebako_resolve::{RegistryRef, Transport};

struct MockTransport {
    answers: HashMap<String, Vec<u8>>,
}
impl MockTransport {
    fn with(answers: &[(&str, &[u8])]) -> Self {
        MockTransport {
            answers: answers
                .iter()
                .map(|(u, b)| (u.to_string(), b.to_vec()))
                .collect(),
        }
    }
}
impl Transport for MockTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.answers
            .get(url)
            .cloned()
            .ok_or_else(|| FetchError::IndexUnavailable(url.to_string()))
    }
}

const REGISTRY_YAML: &str = "schema_version: 1\npayloads:\n  - name: tool\n    kind: app\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: tfs:github:o/tool:1.0}, entrypoints: [tool]}\n";

#[test]
fn file_mirror_registry_resolves_and_parses() {
    let dir = scratch("regfile");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tpkg-registry.yaml"), REGISTRY_YAML).unwrap();

    let r = RegistryRef::parse(&format!("{}/tpkg-registry.yaml", file_url(&mirror))).unwrap();
    let registry = Fetcher::new().resolve_registry(&r).unwrap();
    assert_eq!(registry.payloads.len(), 1);
    assert_eq!(registry.payloads[0].name, "tool");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn default_branch_form_resolves_through_the_contents_api() {
    let api = "https://api.github.com/repos/o/r/contents/tpkg-registry.yaml";
    let doc =
        r#"{"name":"tpkg-registry.yaml","download_url":"https://raw/o/r/HEAD/tpkg-registry.yaml"}"#;
    let t = MockTransport::with(&[
        (api, doc.as_bytes()),
        (
            "https://raw/o/r/HEAD/tpkg-registry.yaml",
            REGISTRY_YAML.as_bytes(),
        ),
    ]);
    let fetcher = Fetcher::with_transport(t);
    let r = RegistryRef::parse("tfs:github:o/r").unwrap();
    let registry = fetcher.resolve_registry(&r).unwrap();
    assert_eq!(registry.payloads[0].name, "tool");

    // the pinned form verifies the registry file itself
    let pinned = RegistryRef::parse(&format!(
        "tfs:github:o/r?sha256={}",
        sha256_hex(REGISTRY_YAML.as_bytes())
    ))
    .unwrap();
    assert!(fetcher.resolve_registry(&pinned).is_ok());
    let wrong = RegistryRef::parse(&format!("tfs:github:o/r?sha256={}", "0".repeat(64))).unwrap();
    assert!(matches!(
        fetcher.resolve_registry(&wrong).unwrap_err(),
        ResolveError::Sha256Mismatch { .. }
    ));

    // 404 at the contents API is the named NotFound, never a fallback.
    let t = MockTransport::with(&[]);
    let fetcher = Fetcher::with_transport(t);
    let err = fetcher.resolve_registry(&r).unwrap_err();
    assert!(matches!(err, ResolveError::NotFound { .. }));
}

#[test]
fn release_artifact_form_uses_the_artifact_selector() {
    let api = "https://api.github.com/repos/o/r/releases/tags/v9";
    let body = r#"{"assets":[
        {"name":"r-v9.tfs","browser_download_url":"https://dl/r-v9.tfs"},
        {"name":"tpkg-registry.yaml","browser_download_url":"https://dl/tpkg-registry.yaml"}]}"#;
    let t = MockTransport::with(&[
        (api, body.as_bytes()),
        ("https://dl/tpkg-registry.yaml", REGISTRY_YAML.as_bytes()),
    ]);
    let fetcher = Fetcher::with_transport(t);
    let r = RegistryRef::parse("tfs:github:o/r:v9#tpkg-registry.yaml").unwrap();
    let registry = fetcher.resolve_registry(&r).unwrap();
    assert_eq!(registry.payloads[0].name, "tool");

    // a release without the registry file → AssetNotFound naming it
    let body = r#"{"assets":[{"name":"r-v9.tfs","browser_download_url":"https://dl/r-v9.tfs"}]}"#;
    let t = MockTransport::with(&[(api, body.as_bytes())]);
    let fetcher = Fetcher::with_transport(t);
    let err = fetcher.resolve_registry(&r).unwrap_err();
    assert!(matches!(
        err,
        ResolveError::AssetNotFound {
            artifact: Some(_),
            ..
        }
    ));
}

#[test]
fn git_blob_form_resolves_through_the_real_git_adapter() {
    let dir = scratch("reggit");
    let fixture = dir.join("registry.git");
    let mut repo = gix::init_bare(&fixture).unwrap();
    // The commit needs an author/committer; never rely on ambient git
    // config (CI runners have none, and gix's defaults have drifted —
    // AuthorMissing). Pin the identity in the fixture repo itself (the
    // guard writes back on drop).
    {
        let mut config = repo.config_snapshot_mut();
        config.set_raw_value("user.name", "tebako-test").unwrap();
        config
            .set_raw_value("user.email", "tebako-test@localhost")
            .unwrap();
    }
    let blob = repo.write_blob(REGISTRY_YAML.as_bytes()).unwrap();
    let empty = gix::hash::ObjectId::empty_tree(repo.object_hash());
    let tree = {
        let mut ed = repo.edit_tree(empty).unwrap();
        ed.upsert(
            "meta/tpkg-registry.yaml",
            gix::object::tree::EntryKind::Blob,
            blob,
        )
        .unwrap();
        ed.write().unwrap()
    };
    repo.commit("refs/heads/main", "main", tree, gix::commit::NO_PARENT_IDS)
        .unwrap();

    // The url field takes an absolute local path verbatim (the air-gapped
    // mirror form); the string grammar reserves `tfs+git://` for host paths.
    let r = RegistryRef::GitBlob(Reference::Git {
        url: fixture.to_string_lossy().into_owned(),
        git_ref: Some("main".to_string()),
        path: Some("meta/tpkg-registry.yaml".to_string()),
        sha256: None,
    });
    let registry = Fetcher::new().resolve_registry(&r).unwrap();
    assert_eq!(registry.payloads[0].name, "tool");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn unparsable_registry_is_a_named_error() {
    let dir = scratch("regbad");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tpkg-registry.yaml"), "schema_version: 99\n").unwrap();
    let r = RegistryRef::parse(&format!("{}/tpkg-registry.yaml", file_url(&mirror))).unwrap();
    let err = Fetcher::new().resolve_registry(&r).unwrap_err();
    assert!(matches!(err, ResolveError::Registry(_)));
    assert!(err.to_string().contains("schema_version 99"));
    let _ = fs::remove_dir_all(&dir);
}
