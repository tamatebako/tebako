//! The fetcher: resolve a [`Reference`] to payload bytes (spec 04 §3).
//! HTTPS and `file://` go through the injected [`Transport`]; service
//! references through the matching [`ServiceAdapter`]; `tfs+git:` through
//! the gix adapter. A digest pin on the reference is verified here — the
//! fetch boundary — so a mismatch is a named sha error before any cache
//! sees the bytes (spec 04 §3: nothing enters the cache).

use sha2::Digest;
use tebako_http::FetchError;

use crate::adapters::adapter_for;
use crate::error::ResolveError;
#[cfg(feature = "git")]
use crate::git;
use crate::reference::{Reference, Service};
use crate::transport::{HttpTransport, Transport};

/// A fetched payload: the bytes, where they came from, and their digest
/// (always computed — the cache marker is the trust anchor, spec 05 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPayload {
    pub bytes: Vec<u8>,
    /// The concrete origin (download URL / file path / git coordinates) —
    /// written to the cache's `.origin` marker.
    pub origin: String,
    /// Lowercase sha256 of `bytes`.
    pub sha256: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hex_digest(&hasher.finalize())
}

/// Lowercase hex of a digest (the seed path streams its hash).
pub(crate) fn hex_digest(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

/// A reference fetcher over an injected transport.
pub struct Fetcher<T: Transport> {
    pub(crate) transport: T,
}

impl Fetcher<HttpTransport> {
    /// The production fetcher (tebako-http transport).
    pub fn new() -> Self {
        Fetcher {
            transport: HttpTransport,
        }
    }
}

impl Default for Fetcher<HttpTransport> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Transport> Fetcher<T> {
    pub fn with_transport(transport: T) -> Self {
        Fetcher { transport }
    }

    /// Fetch and (when the reference carries a pin) sha256-verify.
    pub fn fetch(&self, reference: &Reference) -> Result<FetchedPayload, ResolveError> {
        let (bytes, origin) = match reference {
            Reference::Https { url, .. } => (self.get(url)?, url.clone()),
            Reference::File { path, .. } => {
                // the canonical constructor — a windows drive path needs
                // the third slash (file:///C:/x), and the mock transports
                // key on exactly this form
                let url = tebako_http::file_url(std::path::Path::new(path));
                (self.get(&url)?, url)
            }
            Reference::Service {
                service,
                owner,
                repo,
                version,
                artifact,
                ..
            } => self.fetch_service(*service, owner, repo, version, artifact.as_deref())?,
            Reference::Git {
                url, git_ref, path, ..
            } => {
                let Some(path) = path else {
                    return Err(ResolveError::GitPathRequired { url: url.clone() });
                };
                #[cfg(feature = "git")]
                {
                    let bytes = git::fetch_blob(url, git_ref.as_deref(), path)?;
                    (bytes, reference.to_string())
                }
                #[cfg(not(feature = "git"))]
                {
                    let _ = (git_ref, path);
                    return Err(ResolveError::Git {
                        url: url.clone(),
                        reason: "this build was compiled without the 'git' feature — tfs+git: references are unsupported".to_string(),
                    });
                }
            }
        };
        let sha256 = sha256_hex(&bytes);
        if let Some(expected) = reference.sha256() {
            if sha256 != expected {
                return Err(ResolveError::Sha256Mismatch {
                    origin,
                    expected: expected.to_string(),
                    actual: sha256,
                });
            }
        }
        Ok(FetchedPayload {
            bytes,
            origin,
            sha256,
        })
    }

    fn get(&self, url: &str) -> Result<Vec<u8>, ResolveError> {
        self.transport.get(url).map_err(|e| match e {
            FetchError::IndexUnavailable(_) => ResolveError::NotFound {
                origin: url.to_string(),
            },
            FetchError::Throttled { .. } => ResolveError::DownloadFailed {
                origin: url.to_string(),
                reason: e.to_string(),
            },
            FetchError::DownloadFailed(reason) => ResolveError::DownloadFailed {
                origin: url.to_string(),
                reason,
            },
        })
    }

    /// Apply the multi-artifact selection rule (spec 04 §1, locked):
    /// `#artifact` → [`ServiceAdapter::asset_named`] (missing is
    /// `AssetNotFound` naming it); no `#` →
    /// [`adapters::select_candidate`] over the release's `.tfs` assets.
    /// Never a guess (spec 00 invariant 9).
    fn fetch_service(
        &self,
        service: Service,
        owner: &str,
        repo: &str,
        version: &str,
        artifact: Option<&str>,
    ) -> Result<(Vec<u8>, String), ResolveError> {
        let adapter = adapter_for(service);
        let (_, url) = match artifact {
            Some(name) => adapter
                .asset_named(&self.transport, owner, repo, version, name)?
                .ok_or_else(|| ResolveError::AssetNotFound {
                    service,
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    version: version.to_string(),
                    artifact: Some(name.to_string()),
                })?,
            None => {
                let assets = adapter.assets(&self.transport, owner, repo, version)?;
                crate::adapters::select_candidate(service, owner, repo, version, assets)?
            }
        };
        let bytes = self.get(&url)?;
        Ok((bytes, url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::Service;
    use std::collections::HashMap;

    pub struct MockTransport {
        pub answers: HashMap<String, Vec<u8>>,
    }
    impl MockTransport {
        pub fn with(answers: &[(&str, &[u8])]) -> Self {
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

    #[test]
    fn https_fetch_verifies_the_pin() {
        let t = MockTransport::with(&[("https://cdn/t.tfs", b"payload")]);
        let f = Fetcher::with_transport(t);
        let good = Reference::Https {
            url: "https://cdn/t.tfs".into(),
            sha256: Some(sha256_hex(b"payload")),
        };
        assert_eq!(f.fetch(&good).unwrap().bytes, b"payload");

        let bad = Reference::Https {
            url: "https://cdn/t.tfs".into(),
            sha256: Some("0".repeat(64)),
        };
        let err = f.fetch(&bad).unwrap_err();
        assert!(matches!(err, ResolveError::Sha256Mismatch { .. }));
        assert!(err.to_string().contains("expected 0000"));

        let missing = Reference::Https {
            url: "https://cdn/missing.tfs".into(),
            sha256: None,
        };
        assert!(matches!(
            f.fetch(&missing).unwrap_err(),
            ResolveError::NotFound { .. }
        ));
    }

    #[test]
    fn service_fetch_picks_the_single_tfs_asset() {
        let api = "https://api.github.com/repos/o/r/releases/tags/v1";
        let body =
            r#"{"assets":[{"name":"r-v1.tfs","browser_download_url":"https://dl/r-v1.tfs"}]}"#;
        let t = MockTransport::with(&[(api, body.as_bytes()), ("https://dl/r-v1.tfs", b"img")]);
        let f = Fetcher::with_transport(t);
        let r = Reference::Service {
            service: Service::Github,
            owner: "o".into(),
            repo: "r".into(),
            version: "v1".into(),
            artifact: None,
            sha256: None,
        };
        let got = f.fetch(&r).unwrap();
        assert_eq!(got.bytes, b"img");
        assert_eq!(got.origin, "https://dl/r-v1.tfs");
    }

    #[test]
    fn service_fetch_with_artifact_takes_exactly_that_asset() {
        let api = "https://api.github.com/repos/o/r/releases/tags/v1";
        let body = r#"{"assets":[
            {"name":"r-linux-v1.tfs","browser_download_url":"https://dl/linux.tfs"},
            {"name":"r-macos-v1.tfs","browser_download_url":"https://dl/macos.tfs"}]}"#;
        let t =
            MockTransport::with(&[(api, body.as_bytes()), ("https://dl/macos.tfs", b"mac-img")]);
        let f = Fetcher::with_transport(t);
        let r = Reference::parse("tfs:github:o/r:v1#r-macos-v1.tfs").unwrap();
        let got = f.fetch(&r).unwrap();
        assert_eq!(got.bytes, b"mac-img");
        assert_eq!(got.origin, "https://dl/macos.tfs");

        // a missing #artifact is the named AssetNotFound
        let r = Reference::parse("tfs:github:o/r:v1#r-windows-v1.tfs").unwrap();
        let err = f.fetch(&r).unwrap_err();
        assert!(matches!(
            err,
            ResolveError::AssetNotFound {
                artifact: Some(_),
                ..
            }
        ));
        assert!(err.to_string().contains("'r-windows-v1.tfs'"));
    }

    #[test]
    fn ambiguous_assets_are_a_named_error_listing_every_candidate() {
        let api = "https://api.github.com/repos/o/r/releases/tags/v1";
        let body = r#"{"assets":[
            {"name":"r-macos-v1.tfs","browser_download_url":"https://dl/a.tfs"},
            {"name":"r-linux-v1.tfs","browser_download_url":"https://dl/b.tfs"}]}"#;
        let t = MockTransport::with(&[(api, body.as_bytes())]);
        let f = Fetcher::with_transport(t);
        let r = Reference::Service {
            service: Service::Github,
            owner: "o".into(),
            repo: "r".into(),
            version: "v1".into(),
            artifact: None,
            sha256: None,
        };
        let err = f.fetch(&r).unwrap_err();
        let ResolveError::AmbiguousAssets { assets, .. } = &err else {
            panic!("expected AmbiguousAssets, got {err:?}")
        };
        assert_eq!(assets, &["r-macos-v1.tfs", "r-linux-v1.tfs"]);
        assert!(err.to_string().contains("r-macos-v1.tfs"));
    }

    #[test]
    fn git_without_path_is_a_named_error() {
        let t = MockTransport::with(&[]);
        let f = Fetcher::with_transport(t);
        let r = Reference::Git {
            url: "h/registry.git".into(),
            git_ref: None,
            path: None,
            sha256: None,
        };
        assert!(matches!(
            f.fetch(&r).unwrap_err(),
            ResolveError::GitPathRequired { .. }
        ));
    }
}
