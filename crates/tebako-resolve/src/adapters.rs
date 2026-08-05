//! Service adapters (spec 04 §1): `tfs:github:` / `tfs:gitlab:` / `tfs:bb:`
//! resolve through the host's release/download APIs — the git host's
//! releases ARE the storage (spec 04 §2). All three sit behind the same
//! [`ServiceAdapter`] trait; the transport is injected, so tests answer
//! with canned API bodies (spec 04 §3: `file://` mirrors for tests).
//!
//! - GitHub: releases API first — `repos/{owner}/{repo}/releases/tags/{v}`.
//! - GitLab: `projects/{owner%2Frepo}/releases/{v}`, `assets.links`.
//! - Bitbucket: no releases concept — `repositories/{o}/{r}/downloads`,
//!   files whose name carries the version string. First page of 100;
//!   a `next` page is a named error, never a silent partial listing.
//!
//! Adapters list EVERY asset of the release; the multi-artifact selection
//! rule (spec 04 §1, locked) is split between [`select_candidate`] (no
//! `#`: the `.tfs` candidate class — one is used, zero is `AssetNotFound`,
//! more than one is `AmbiguousAssets` listing every candidate) and
//! [`ServiceAdapter::asset_named`] (`#artifact`: exactly that asset,
//! missing → `AssetNotFound` naming it). The adapter NEVER auto-picks by
//! host triplet.
//!
//! Each adapter also reads the registry file (spec 04 §2):
//! `tpkg-registry.yaml` from the repo's DEFAULT-BRANCH ROOT via the
//! service contents API — exactly one location, no fallback chain.

use tebako_http::FetchError;
use tebako_json::{parse as json_parse, Value as JsonValue};

use crate::error::ResolveError;
use crate::reference::Service;
use crate::transport::Transport;

/// One downloadable asset: `(file name, download URL)`.
pub type Asset = (String, String);

/// The registry file name at the default-branch root (spec 04 §2).
pub const REGISTRY_FILE: &str = "tpkg-registry.yaml";

/// A service host's release API, behind one trait so the fetcher and the
/// tests treat all three uniformly.
pub trait ServiceAdapter {
    fn service(&self) -> Service;
    /// Every asset of `{owner}/{repo}:{version}` (unfiltered — the
    /// selection rule is the caller's, [`select_asset`]).
    fn assets(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
        version: &str,
    ) -> Result<Vec<Asset>, ResolveError>;
    /// One asset by exact name (the `#artifact` selector). Default: the
    /// release's asset list. Bitbucket overrides it: its downloads API
    /// has no releases, so `assets()` filters by the version string —
    /// but an explicitly named artifact (`#tpkg-registry.yaml`) must
    /// match regardless of that filter.
    fn asset_named(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
        version: &str,
        name: &str,
    ) -> Result<Option<Asset>, ResolveError> {
        Ok(self
            .assets(transport, owner, repo, version)?
            .into_iter()
            .find(|(n, _)| n == name))
    }
    /// `tpkg-registry.yaml` from the repo's default-branch root (spec 04
    /// §2 registry resolution, first form).
    fn registry_file(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<u8>, ResolveError>;
}

/// The dispatch table (spec 04 §1): one adapter per service.
pub fn adapter_for(service: Service) -> Box<dyn ServiceAdapter> {
    match service {
        Service::Github => Box::new(GithubAdapter),
        Service::Gitlab => Box::new(GitlabAdapter),
        Service::Bitbucket => Box::new(BitbucketAdapter),
    }
}

/// The multi-artifact selection rule (spec 04 §1, locked — no magic):
/// without a `#artifact` fragment the candidate class is `.tfs` images —
/// exactly one is used, zero is `AssetNotFound`, more than one is
/// `AmbiguousAssets` naming every candidate so the user re-runs with
/// `#name`. (The `#artifact` case is [`ServiceAdapter::asset_named`].)
pub fn select_candidate(
    service: Service,
    owner: &str,
    repo: &str,
    version: &str,
    assets: Vec<Asset>,
) -> Result<Asset, ResolveError> {
    let candidates: Vec<Asset> = assets
        .into_iter()
        .filter(|(name, _)| name.ends_with(".tfs"))
        .collect();
    match candidates.as_slice() {
        [] => Err(ResolveError::AssetNotFound {
            service,
            owner: owner.to_string(),
            repo: repo.to_string(),
            version: version.to_string(),
            artifact: None,
        }),
        [single] => Ok(single.clone()),
        many => Err(ResolveError::AmbiguousAssets {
            service,
            owner: owner.to_string(),
            repo: repo.to_string(),
            version: version.to_string(),
            assets: many.iter().map(|(n, _)| n.clone()).collect(),
        }),
    }
}

fn map_fetch(url: &str, e: FetchError) -> ResolveError {
    match e {
        FetchError::IndexUnavailable(_) => ResolveError::NotFound {
            origin: url.to_string(),
        },
        FetchError::Throttled { .. } => ResolveError::DownloadFailed {
            origin: url.to_string(),
            reason: e.to_string(),
        },
        FetchError::DownloadFailed(msg) => ResolveError::DownloadFailed {
            origin: url.to_string(),
            reason: msg,
        },
    }
}

fn get_json(
    transport: &dyn Transport,
    service: Service,
    url: &str,
) -> Result<JsonValue, ResolveError> {
    let body = transport.get(url).map_err(|e| map_fetch(url, e))?;
    let text = String::from_utf8(body).map_err(|e| ResolveError::ServiceFailed {
        service,
        reason: format!("{e} decoding {url}"),
    })?;
    json_parse(&text).map_err(|e| ResolveError::ServiceFailed {
        service,
        reason: format!("invalid JSON from {url}: {e}"),
    })
}

fn strings(v: &JsonValue, key: &str) -> Option<String> {
    v.find(key).and_then(|x| x.as_string())
}

/// Raw GET through the transport with the crate's error mapping (404 →
/// [`ResolveError::NotFound`], anything else → [`ResolveError::DownloadFailed`]).
fn get_bytes(transport: &dyn Transport, url: &str) -> Result<Vec<u8>, ResolveError> {
    transport.get(url).map_err(|e| map_fetch(url, e))
}

// ---- GitHub ---------------------------------------------------------------

/// `tfs:github:` — GitHub releases API first.
pub struct GithubAdapter;

impl ServiceAdapter for GithubAdapter {
    fn service(&self) -> Service {
        Service::Github
    }

    fn assets(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
        version: &str,
    ) -> Result<Vec<Asset>, ResolveError> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{version}");
        let doc = get_json(transport, Service::Github, &url)?;
        let Some(JsonValue::Array(assets)) = doc.find("assets") else {
            return Err(ResolveError::ServiceFailed {
                service: Service::Github,
                reason: format!("{url} has no assets array"),
            });
        };
        Ok(assets
            .iter()
            .filter_map(|a| Some((strings(a, "name")?, strings(a, "browser_download_url")?)))
            .collect())
    }

    /// The contents API names the default branch implicitly; the file's
    /// bytes come INLINE in `content` (base64 — always fresh at request
    /// time). The `download_url` (raw.githubusercontent.com) is only the
    /// fallback: the raw CDN caches for minutes after a push, and an
    /// install that reads a STALE registry fails its sha check against
    /// the fresh release assets (proven live: a registry updated 2
    /// minutes before an install resolved the old pin).
    fn registry_file(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<u8>, ResolveError> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/contents/{REGISTRY_FILE}");
        let doc = get_json(transport, Service::Github, &url)?;
        if let Some(content) = strings(&doc, "content") {
            if let Some(bytes) = base64_decode(&content) {
                return Ok(bytes);
            }
        }
        let Some(download) = strings(&doc, "download_url") else {
            return Err(ResolveError::ServiceFailed {
                service: Service::Github,
                reason: format!(
                    "{url} carries neither content nor download_url (is {REGISTRY_FILE} a file?)"
                ),
            });
        };
        get_bytes(transport, &download)
    }
}

/// Standard base64 with arbitrary whitespace (the contents API wraps at
/// 60 columns); `None` on malformed input (the caller falls back).
fn base64_decode(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    fn value(byte: u8) -> Option<u32> {
        ALPHABET.iter().position(|&c| c == byte).map(|p| p as u32)
    }
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut padding = 0;
    for byte in text.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if byte == b'=' {
            padding += 1;
            continue;
        }
        if padding > 0 {
            return None; // content after '='
        }
        acc = (acc << 6) | value(byte)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

// ---- GitLab ---------------------------------------------------------------

/// `tfs:gitlab:` — GitLab releases API (`assets.links` direct links only;
/// the auto-generated source archives are never payload candidates).
pub struct GitlabAdapter;

impl ServiceAdapter for GitlabAdapter {
    fn service(&self) -> Service {
        Service::Gitlab
    }

    fn assets(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
        version: &str,
    ) -> Result<Vec<Asset>, ResolveError> {
        // Nested groups ride in `owner`; '/' must be percent-encoded.
        let project = format!("{owner}/{repo}").replace('/', "%2F");
        let url = format!("https://gitlab.com/api/v4/projects/{project}/releases/{version}");
        let doc = get_json(transport, Service::Gitlab, &url)?;
        let links = doc
            .find("assets")
            .and_then(|a| a.find("links"))
            .and_then(|l| match l {
                JsonValue::Array(items) => Some(items),
                _ => None,
            });
        let Some(links) = links else {
            return Err(ResolveError::ServiceFailed {
                service: Service::Gitlab,
                reason: format!("{url} has no assets.links array"),
            });
        };
        Ok(links
            .iter()
            .filter_map(|a| Some((strings(a, "name")?, strings(a, "url")?)))
            .collect())
    }

    /// The repository-files API serves raw bytes; no `ref` parameter means
    /// the project's default branch.
    fn registry_file(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<u8>, ResolveError> {
        let project = format!("{owner}/{repo}").replace('/', "%2F");
        let url = format!(
            "https://gitlab.com/api/v4/projects/{project}/repository/files/{REGISTRY_FILE}/raw"
        );
        get_bytes(transport, &url)
    }
}

// ---- Bitbucket ------------------------------------------------------------

/// `tfs:bb:` — Bitbucket has no releases; the downloads API lists files
/// and the version string selects among them (`<anything>-<v>.tfs`).
pub struct BitbucketAdapter;

impl ServiceAdapter for BitbucketAdapter {
    fn service(&self) -> Service {
        Service::Bitbucket
    }

    fn assets(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
        version: &str,
    ) -> Result<Vec<Asset>, ResolveError> {
        Ok(downloads(transport, owner, repo)?
            .into_iter()
            .filter(|(name, _)| name.contains(version))
            .collect())
    }

    /// The version-string filter is the releases emulation for the no-`#`
    /// candidate class; an explicitly named artifact matches exactly,
    /// version string or not (`#tpkg-registry.yaml` carries no version).
    fn asset_named(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
        _version: &str,
        name: &str,
    ) -> Result<Option<Asset>, ResolveError> {
        Ok(downloads(transport, owner, repo)?
            .into_iter()
            .find(|(n, _)| n == name))
    }

    /// Bitbucket's src API needs an explicit ref: read the repo's
    /// `mainbranch` first, then the file at that branch.
    fn registry_file(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<u8>, ResolveError> {
        let meta_url = format!("https://api.bitbucket.org/2.0/repositories/{owner}/{repo}");
        let doc = get_json(transport, Service::Bitbucket, &meta_url)?;
        let branch = doc
            .find("mainbranch")
            .and_then(|b| strings(b, "name"))
            .ok_or_else(|| ResolveError::ServiceFailed {
                service: Service::Bitbucket,
                reason: format!("{meta_url} has no mainbranch.name"),
            })?;
        let url = format!(
            "https://api.bitbucket.org/2.0/repositories/{owner}/{repo}/src/{branch}/{REGISTRY_FILE}"
        );
        get_bytes(transport, &url)
    }
}

/// The downloads listing (first page of 100; a `next` page is a named
/// error, never a silent partial listing).
fn downloads(
    transport: &dyn Transport,
    owner: &str,
    repo: &str,
) -> Result<Vec<Asset>, ResolveError> {
    let url =
        format!("https://api.bitbucket.org/2.0/repositories/{owner}/{repo}/downloads?pagelen=100");
    let doc = get_json(transport, Service::Bitbucket, &url)?;
    if doc.find("next").is_some() {
        return Err(ResolveError::ServiceFailed {
            service: Service::Bitbucket,
            reason: format!(
                "{owner}/{repo} has more than one downloads page; narrow the version string"
            ),
        });
    }
    let Some(JsonValue::Array(values)) = doc.find("values") else {
        return Err(ResolveError::ServiceFailed {
            service: Service::Bitbucket,
            reason: format!("{url} has no values array"),
        });
    };
    Ok(values
        .iter()
        .filter_map(|v| {
            let name = strings(v, "name")?;
            let href = v
                .find("links")
                .and_then(|l| l.find("self"))
                .and_then(|s| strings(s, "href"))?;
            Some((name, href))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockTransport {
        answers: HashMap<String, Vec<u8>>,
    }
    impl MockTransport {
        fn with(answers: &[(&str, &str)]) -> Self {
            MockTransport {
                answers: answers
                    .iter()
                    .map(|(u, b)| (u.to_string(), b.as_bytes().to_vec()))
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

    #[test]
    fn github_lists_every_asset_unfiltered() {
        let api = "https://api.github.com/repos/o/r/releases/tags/v1";
        let body = r#"{"assets":[
            {"name":"r-v1.tfs","browser_download_url":"https://dl/r-v1.tfs"},
            {"name":"notes.txt","browser_download_url":"https://dl/notes.txt"}]}"#;
        let t = MockTransport::with(&[(api, body)]);
        // the adapter lists everything; .tfs filtering is the selection rule's
        let assets = GithubAdapter.assets(&t, "o", "r", "v1").unwrap();
        assert_eq!(
            assets,
            vec![
                ("r-v1.tfs".to_string(), "https://dl/r-v1.tfs".to_string()),
                ("notes.txt".to_string(), "https://dl/notes.txt".to_string()),
            ]
        );
        assert!(matches!(
            GithubAdapter.assets(&t, "o", "missing", "v1"),
            Err(ResolveError::NotFound { .. })
        ));
    }

    #[test]
    fn gitlab_encodes_nested_groups_and_reads_links() {
        let api = "https://gitlab.com/api/v4/projects/g%2Fsub%2Fr/releases/v2";
        let body = r#"{"assets":{"links":[
            {"name":"r-v2.tfs","url":"https://gl/dl/r-v2.tfs"}],
            "sources":[{"url":"https://gl/src.tgz"}]}}"#;
        let t = MockTransport::with(&[(api, body)]);
        let assets = GitlabAdapter.assets(&t, "g/sub", "r", "v2").unwrap();
        assert_eq!(
            assets,
            vec![("r-v2.tfs".to_string(), "https://gl/dl/r-v2.tfs".to_string())]
        );
    }

    #[test]
    fn bitbucket_matches_version_in_download_names() {
        let api = "https://api.bitbucket.org/2.0/repositories/o/r/downloads?pagelen=100";
        let body = r#"{"values":[
            {"name":"tool-1.0.tfs","links":{"self":{"href":"https://bb/dl/tool-1.0.tfs"}}},
            {"name":"tool-2.0.tfs","links":{"self":{"href":"https://bb/dl/tool-2.0.tfs"}}}]}"#;
        let t = MockTransport::with(&[(api, body)]);
        let assets = BitbucketAdapter.assets(&t, "o", "r", "1.0").unwrap();
        assert_eq!(
            assets,
            vec![(
                "tool-1.0.tfs".to_string(),
                "https://bb/dl/tool-1.0.tfs".to_string()
            )]
        );
    }

    #[test]
    fn bitbucket_refuses_to_silently_truncate_pages() {
        let api = "https://api.bitbucket.org/2.0/repositories/o/r/downloads?pagelen=100";
        let t = MockTransport::with(&[(api, r#"{"next":"https://…","values":[]}"#)]);
        assert!(matches!(
            BitbucketAdapter.assets(&t, "o", "r", "1.0"),
            Err(ResolveError::ServiceFailed { .. })
        ));
    }

    // ---- the selection rule (spec 04 §1, locked) --------------------------

    fn assets(names: &[&str]) -> Vec<Asset> {
        names
            .iter()
            .map(|n| (n.to_string(), format!("https://dl/{n}")))
            .collect()
    }

    #[test]
    fn select_without_fragment_picks_the_single_tfs_candidate() {
        let got = select_candidate(
            Service::Github,
            "o",
            "r",
            "v1",
            assets(&["notes.txt", "r-v1.tfs", "r-v1.tfs.asc"]),
        )
        .unwrap();
        assert_eq!(got.0, "r-v1.tfs");
    }

    #[test]
    fn select_without_fragment_zero_candidates_is_asset_not_found() {
        let err =
            select_candidate(Service::Github, "o", "r", "v1", assets(&["notes.txt"])).unwrap_err();
        assert!(
            matches!(err, ResolveError::AssetNotFound { artifact: None, .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains("no .tfs asset"));
    }

    #[test]
    fn select_without_fragment_many_candidates_is_ambiguous_and_lists_them() {
        let err = select_candidate(
            Service::Gitlab,
            "o",
            "r",
            "v1",
            assets(&["r-linux-v1.tfs", "r-macos-v1.tfs", "notes.txt"]),
        )
        .unwrap_err();
        let ResolveError::AmbiguousAssets { assets, .. } = err else {
            panic!("expected AmbiguousAssets, got {err:?}")
        };
        assert_eq!(assets, vec!["r-linux-v1.tfs", "r-macos-v1.tfs"]);
        let msg = ResolveError::AmbiguousAssets {
            service: Service::Gitlab,
            owner: "o".into(),
            repo: "r".into(),
            version: "v1".into(),
            assets,
        }
        .to_string();
        assert!(msg.contains("r-linux-v1.tfs") && msg.contains("r-macos-v1.tfs"));
        assert!(
            msg.contains("#name"),
            "the hint names the re-run form: {msg}"
        );
    }

    #[test]
    fn asset_named_takes_exactly_that_asset() {
        // …even when it is not a .tfs file (the registry release form fetches
        // tpkg-registry.yaml; signature sidecars come along the same path)
        let api = "https://api.github.com/repos/o/r/releases/tags/v1";
        let body = r#"{"assets":[
            {"name":"r-v1.tfs","browser_download_url":"https://dl/r-v1.tfs"},
            {"name":"tpkg-registry.yaml","browser_download_url":"https://dl/tpkg-registry.yaml"}]}"#;
        let t = MockTransport::with(&[(api, body)]);
        let got = GithubAdapter
            .asset_named(&t, "o", "r", "v1", "tpkg-registry.yaml")
            .unwrap();
        assert_eq!(
            got,
            Some((
                "tpkg-registry.yaml".to_string(),
                "https://dl/tpkg-registry.yaml".to_string()
            ))
        );
        assert_eq!(
            GithubAdapter
                .asset_named(&t, "o", "r", "v1", "r-windows-v1.tfs")
                .unwrap(),
            None
        );
    }

    #[test]
    fn bitbucket_asset_named_ignores_the_version_filter() {
        // the downloads API has no releases: the version-string filter is
        // the no-# candidate emulation only, an explicit #artifact matches
        // regardless (#tpkg-registry.yaml carries no version)
        let api = "https://api.bitbucket.org/2.0/repositories/o/r/downloads?pagelen=100";
        let body = r#"{"values":[
            {"name":"tool-1.0.tfs","links":{"self":{"href":"https://bb/dl/tool-1.0.tfs"}}},
            {"name":"tpkg-registry.yaml","links":{"self":{"href":"https://bb/dl/tpkg-registry.yaml"}}}]}"#;
        let t = MockTransport::with(&[(api, body)]);
        let got = BitbucketAdapter
            .asset_named(&t, "o", "r", "1.0", "tpkg-registry.yaml")
            .unwrap();
        assert_eq!(
            got,
            Some((
                "tpkg-registry.yaml".to_string(),
                "https://bb/dl/tpkg-registry.yaml".to_string()
            ))
        );
        assert_eq!(
            BitbucketAdapter
                .asset_named(&t, "o", "r", "1.0", "tool-9.9.tfs")
                .unwrap(),
            None
        );
    }

    // ---- the registry file (spec 04 §2, default-branch form) --------------

    #[test]
    fn github_registry_file_follows_the_contents_api_download_url() {
        let api = "https://api.github.com/repos/o/r/contents/tpkg-registry.yaml";
        let body = r#"{"name":"tpkg-registry.yaml","download_url":"https://raw/o/r/HEAD/tpkg-registry.yaml"}"#;
        let t = MockTransport::with(&[
            (api, body),
            (
                "https://raw/o/r/HEAD/tpkg-registry.yaml",
                "schema_version: 1\npayloads: []\n",
            ),
        ]);
        let got = GithubAdapter.registry_file(&t, "o", "r").unwrap();
        assert_eq!(got, b"schema_version: 1\npayloads: []\n");

        // a contents answer without download_url is a named service error
        let t = MockTransport::with(&[(api, r#"[{"name":"tpkg-registry.yaml"}]"#)]);
        assert!(matches!(
            GithubAdapter.registry_file(&t, "o", "r"),
            Err(ResolveError::ServiceFailed { .. })
        ));
        // 404 → NotFound
        assert!(matches!(
            GithubAdapter.registry_file(&t, "o", "missing"),
            Err(ResolveError::NotFound { .. })
        ));
    }

    #[test]
    fn github_registry_file_prefers_the_inline_content_over_the_cdn() {
        // The raw download_url (CDN) caches for minutes after a push —
        // the inline base64 `content` is fresh at request time. Both
        // present: the inline bytes win.
        let api = "https://api.github.com/repos/o/r/contents/tpkg-registry.yaml";
        let body = "{\"name\":\"tpkg-registry.yaml\",\"content\":\"ZnJlc2ggcmVnaXN0cnkK\",\"download_url\":\"https://raw/o/r/HEAD/tpkg-registry.yaml\"}";
        let t = MockTransport::with(&[
            (api, body),
            (
                "https://raw/o/r/HEAD/tpkg-registry.yaml",
                "stale registry\n",
            ),
        ]);
        let got = GithubAdapter.registry_file(&t, "o", "r").unwrap();
        assert_eq!(got, b"fresh registry\n");
    }

    #[test]
    fn base64_decode_handles_wrapped_and_padded_text() {
        assert_eq!(
            base64_decode("aGVsbG8sIHdvcmxk"),
            Some(b"hello, world".to_vec())
        );
        assert_eq!(base64_decode("aGVs\nbG8s\n"), Some(b"hello,".to_vec()));
        assert_eq!(base64_decode(""), Some(Vec::new()));
        assert_eq!(base64_decode("aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(base64_decode("aGVsbG8"), Some(b"hello".to_vec()));
        assert_eq!(base64_decode("####"), None);
        assert_eq!(base64_decode("aGVs==aA"), None);
    }

    #[test]
    fn gitlab_registry_file_reads_raw_at_the_default_branch() {
        let api =
            "https://gitlab.com/api/v4/projects/g%2Fsub%2Fr/repository/files/tpkg-registry.yaml/raw";
        let t = MockTransport::with(&[(api, "schema_version: 1\n")]);
        assert_eq!(
            GitlabAdapter.registry_file(&t, "g/sub", "r").unwrap(),
            b"schema_version: 1\n"
        );
    }

    #[test]
    fn bitbucket_registry_file_resolves_mainbranch_then_src() {
        let meta = "https://api.bitbucket.org/2.0/repositories/o/r";
        let src = "https://api.bitbucket.org/2.0/repositories/o/r/src/trunk/tpkg-registry.yaml";
        let t = MockTransport::with(&[
            (meta, r#"{"mainbranch":{"name":"trunk"}}"#),
            (src, "schema_version: 1\n"),
        ]);
        assert_eq!(
            BitbucketAdapter.registry_file(&t, "o", "r").unwrap(),
            b"schema_version: 1\n"
        );

        let t = MockTransport::with(&[(meta, r#"{"slug":"r"}"#)]);
        assert!(matches!(
            BitbucketAdapter.registry_file(&t, "o", "r"),
            Err(ResolveError::ServiceFailed { .. })
        ));
    }
}
