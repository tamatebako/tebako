//! Service adapters (spec 04 §1): `tfs:github:` / `tfs:gitlab:` / `tfs:bb:`
//! resolve through the host's release/download APIs — the git host's
//! releases ARE the storage (spec 04 §2). All three sit behind the same
//! [`ServiceAdapter`] trait; the transport is injected, so tests answer
//! with canned API bodies (spec 04 §3: `file://` mirrors for tests).
//!
//! - GitHub: releases API first — `repos/{owner}/{repo}/releases/tags/{v}`,
//!   `.tfs` asset by name.
//! - GitLab: `projects/{owner%2Frepo}/releases/{v}`, `assets.links`.
//! - Bitbucket: no releases concept — `repositories/{o}/{r}/downloads`,
//!   `.tfs` files whose name carries the version string. First page of 100;
//!   a `next` page is a named error, never a silent partial listing.

use tebako_http::FetchError;
use tebako_json::{parse as json_parse, Value as JsonValue};

use crate::error::ResolveError;
use crate::reference::Service;
use crate::transport::Transport;

/// One downloadable asset: `(file name, download URL)`.
pub type Asset = (String, String);

/// A service host's release API, behind one trait so the fetcher and the
/// tests treat all three uniformly.
pub trait ServiceAdapter {
    fn service(&self) -> Service;
    /// Candidate `.tfs` assets of `{owner}/{repo}:{version}`.
    fn assets(
        &self,
        transport: &dyn Transport,
        owner: &str,
        repo: &str,
        version: &str,
    ) -> Result<Vec<Asset>, ResolveError>;
}

/// The dispatch table (spec 04 §1): one adapter per service.
pub fn adapter_for(service: Service) -> Box<dyn ServiceAdapter> {
    match service {
        Service::Github => Box::new(GithubAdapter),
        Service::Gitlab => Box::new(GitlabAdapter),
        Service::Bitbucket => Box::new(BitbucketAdapter),
    }
}

fn map_fetch(url: &str, e: FetchError) -> ResolveError {
    match e {
        FetchError::IndexUnavailable(_) => ResolveError::NotFound {
            origin: url.to_string(),
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

/// Keep only the `.tfs` assets; zero matches and ambiguous multi-matches
/// are the caller's named errors.
fn tfs_only(assets: Vec<Asset>) -> Vec<Asset> {
    assets
        .into_iter()
        .filter(|(name, _)| name.ends_with(".tfs"))
        .collect()
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
        Ok(tfs_only(
            assets
                .iter()
                .filter_map(|a| Some((strings(a, "name")?, strings(a, "browser_download_url")?)))
                .collect(),
        ))
    }
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
        Ok(tfs_only(
            links
                .iter()
                .filter_map(|a| Some((strings(a, "name")?, strings(a, "url")?)))
                .collect(),
        ))
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
        let url = format!(
            "https://api.bitbucket.org/2.0/repositories/{owner}/{repo}/downloads?pagelen=100"
        );
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
        let assets = values
            .iter()
            .filter_map(|v| {
                let name = strings(v, "name")?;
                let href = v
                    .find("links")
                    .and_then(|l| l.find("self"))
                    .and_then(|s| strings(s, "href"))?;
                Some((name, href))
            })
            .filter(|(name, _)| name.contains(version))
            .collect();
        Ok(tfs_only(assets))
    }
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
    fn github_lists_tfs_assets() {
        let api = "https://api.github.com/repos/o/r/releases/tags/v1";
        let body = r#"{"assets":[
            {"name":"r-v1.tfs","browser_download_url":"https://dl/r-v1.tfs"},
            {"name":"notes.txt","browser_download_url":"https://dl/notes.txt"}]}"#;
        let t = MockTransport::with(&[(api, body)]);
        let assets = GithubAdapter.assets(&t, "o", "r", "v1").unwrap();
        assert_eq!(
            assets,
            vec![("r-v1.tfs".to_string(), "https://dl/r-v1.tfs".to_string())]
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
}
