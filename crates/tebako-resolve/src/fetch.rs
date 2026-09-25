//! The fetcher: resolve a [`Reference`] to payload bytes (spec 04 §3).
//! HTTPS and `file://` go through the injected [`Transport`]; service
//! references through the matching [`ServiceAdapter`]; `tfs+git:` through
//! the gix adapter. A digest pin on the reference is verified here — the
//! fetch boundary — so a mismatch is a named sha error before any cache
//! sees the bytes (spec 04 §3: nothing enters the cache).

use sha2::Digest;
use tebako_http::FetchError;

use crate::credentials::{self, CredentialBook, Decision};
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

/// The fetch error mapping, shared by plain and asset GETs: a missing
/// object is `NotFound` (the next-index walk), everything else
/// `DownloadFailed`; the named networking failures ride their own
/// messages, never NotFound.
fn map_fetch_error(url: &str, e: FetchError) -> ResolveError {
    match e {
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
        // The credential layer remaps AuthRejected before this point;
        // an unwrapped sighting is still the credential refusal, never a
        // retried download failure.
        FetchError::AuthRejected { .. } => ResolveError::CredentialRequired {
            registry: None,
            host: credentials::url_host(url).unwrap_or(url).to_string(),
            looked_for: None,
        },
        FetchError::CredentialRequired {
            registry,
            host,
            looked_for,
        } => ResolveError::CredentialRequired {
            registry,
            host,
            looked_for,
        },
        // The plan-cancel abort never reaches the buffered fetcher; map
        // it like a transport failure if a caller ever sees one.
        FetchError::Cancelled(reason) => ResolveError::DownloadFailed {
            origin: url.to_string(),
            reason,
        },
        // TODO.v2-1/33's named networking failures ride their own
        // messages; never NotFound (no next-index walk).
        FetchError::ProxyAuthRequired(_) | FetchError::NetworkingCompiledOut(_) => {
            ResolveError::DownloadFailed {
                origin: url.to_string(),
                reason: e.to_string(),
            }
        }
        #[cfg(feature = "network")]
        FetchError::NetConfig(_) => ResolveError::DownloadFailed {
            origin: url.to_string(),
            reason: e.to_string(),
        },
    }
}

// ---------------------------------------------------------------------
// the credential-applying transport (spec 37 §5)
// ---------------------------------------------------------------------

/// One URL's decided credential ride: the header to attach (None =
/// anonymous) and the env var NAME the decision wanted when one was
/// matched-or-attached (the `CredentialRequired` remap's `looked_for`).
struct CredOutcome {
    header: Option<(&'static str, String)>,
    looked_for: Option<String>,
}

impl CredOutcome {
    fn header_ref(&self) -> Option<(&str, &str)> {
        self.header
            .as_ref()
            .map(|(name, value)| (*name, value.as_str()))
    }
}

/// The credential-applying transport wrapper (spec 37 §5): every fetch
/// the fetcher or the plan executor runs goes through here — the book
/// decides the credential for the URL (two tiers + the ambient rule,
/// locked confinement), the decided header rides
/// [`Transport::get_with_header`] and its requirements/stream forms
/// verbatim, the service's 401/403 refusal remaps to the named
/// `CredentialRequired`, and the decision journals (host + credential
/// CLASS, value redacted — non-empty books only, https URLs only; a
/// journal error never fails the fetch). The adapters stay UNAWARE:
/// they receive this as `&dyn Transport`.
pub struct CredTransport<'a, T: Transport + ?Sized> {
    inner: &'a T,
    book: CredentialBook,
    /// The tier-1 key: the alias of the registry whose row directed the
    /// fetch (None = the anonymous scope — direct references).
    alias: Option<String>,
    /// The service class of the fetch context (the header shape; None
    /// for plain https/file fetches — `Authorization: Bearer`).
    service: Option<Service>,
    /// The explicit host of the `tfs+<svc>://host/…` form, when the
    /// context is a service one — the `authenticated()` host set's
    /// parameter.
    service_host: Option<String>,
    journal_home: Option<std::path::PathBuf>,
}

impl<'a, T: Transport + ?Sized> CredTransport<'a, T> {
    pub fn new(
        inner: &'a T,
        book: CredentialBook,
        alias: Option<String>,
        service: Option<Service>,
        service_host: Option<String>,
    ) -> Self {
        CredTransport {
            inner,
            book,
            alias,
            service,
            service_host,
            journal_home: credentials::journal_home(),
        }
    }

    /// Decide the credential for `url` and journal the decision.
    fn decide(&self, url: &str) -> CredOutcome {
        let (header, class, looked_for) =
            match self.book.decide(self.alias.as_deref(), url, self.service) {
                Decision::Attach {
                    header_name,
                    header_value,
                    class,
                } => {
                    // The class spells `token:env:<NAME>`; the refusal remap
                    // names the same var.
                    let env = class.strip_prefix("token:env:").map(str::to_string);
                    (Some((header_name, header_value)), class, env)
                }
                Decision::Anonymous { looked_for } => (None, "anonymous".to_string(), looked_for),
            };
        // The mixed-federation audit trail: book-empty machines journal
        // NOTHING new (test-noise control); a non-empty book journals
        // every remote (https) fetch, credential class only.
        if !self.book.is_empty() && url.starts_with("https://") {
            if let (Some(home), Some(host)) = (&self.journal_home, credentials::url_host(url)) {
                credentials::journal_fetch(home, host, &class);
            }
        }
        CredOutcome { header, looked_for }
    }

    /// The service's refusal becomes the named `CredentialRequired`
    /// (spec 37 §5, exit class 69) — naming the host, the directing
    /// registry's alias when one scoped the fetch, and the env var the
    /// decision wanted.
    fn remap(&self, url: &str, outcome: &CredOutcome, e: FetchError) -> FetchError {
        match e {
            FetchError::AuthRejected { .. } => FetchError::CredentialRequired {
                host: credentials::url_host(url).unwrap_or(url).to_string(),
                registry: self.alias.clone(),
                looked_for: outcome.looked_for.clone(),
            },
            other => other,
        }
    }
}

impl<T: Transport + ?Sized> Transport for CredTransport<'_, T> {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let outcome = self.decide(url);
        self.inner
            .get_with_header(url, outcome.header_ref())
            .map_err(|e| self.remap(url, &outcome, e))
    }

    fn get_asset(
        &self,
        url: &str,
        accept: Option<&str>,
        authenticate: bool,
    ) -> Result<Vec<u8>, FetchError> {
        let outcome = self.decide(url);
        self.inner
            .get_asset_with_header(url, accept, authenticate, outcome.header_ref())
            .map_err(|e| self.remap(url, &outcome, e))
    }

    fn authenticated(&self) -> bool {
        // True when the book could authenticate this context's hosts
        // (an applicable tier-1/tier-2 entry with its env var SET — an
        // entry without its token must not steer to the API URLs) or
        // the inner transport authenticates (the ambient token; the
        // pre-book semantics on a book-empty machine).
        let hosts = match self.service {
            Some(service) => crate::adapters::service_hosts(service, self.service_host.as_deref()),
            None => Default::default(),
        };
        self.book.can_authenticate(&hosts) || self.inner.authenticated()
    }

    fn stream(
        &self,
        url: &str,
        writer: &mut dyn std::io::Write,
        on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
    ) -> Result<u64, FetchError> {
        let outcome = self.decide(url);
        self.inner
            .stream_with_header(url, outcome.header_ref(), writer, on_progress)
            .map_err(|e| self.remap(url, &outcome, e))
    }

    fn stream_asset(
        &self,
        url: &str,
        accept: Option<&str>,
        authenticate: bool,
        writer: &mut dyn std::io::Write,
        on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
    ) -> Result<u64, FetchError> {
        let outcome = self.decide(url);
        self.inner
            .stream_asset_with_header(
                url,
                accept,
                authenticate,
                outcome.header_ref(),
                writer,
                on_progress,
            )
            .map_err(|e| self.remap(url, &outcome, e))
    }
}

/// A reference fetcher over an injected transport.
pub struct Fetcher<T: Transport> {
    pub(crate) transport: T,
    /// The credential book this fetcher decides through (spec 37 §5):
    /// `None` = the process-global book at FETCH time (production — the
    /// book's one install point is tebako-shim's config load, which can
    /// run after the fetcher is constructed); `Some` = the explicit
    /// injection (tests — the global never moves mid-test).
    pub(crate) book: Option<CredentialBook>,
}

impl Fetcher<HttpTransport> {
    /// The production fetcher (tebako-http transport).
    pub fn new() -> Self {
        Fetcher {
            transport: HttpTransport,
            book: None,
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
        Fetcher {
            transport,
            book: None,
        }
    }

    /// Decide through an explicit credential book instead of the
    /// process-global one (tests — the global book's one install point
    /// is tebako-shim's config load).
    pub fn with_credential_book(mut self, book: CredentialBook) -> Self {
        self.book = Some(book);
        self
    }

    /// The book this fetch decides through (spec 37 §5): the explicit
    /// injection when one is set, else the process-global book read NOW
    /// (a config load between construction and fetch must count).
    pub(crate) fn effective_book(&self) -> CredentialBook {
        self.book.clone().unwrap_or_else(credentials::book)
    }

    /// The injected transport — the fetch pipeline (spec 05 §6) streams
    /// through the SAME seam the one-shot [`Fetcher::fetch`] reads, so a
    /// test mock answers plan GETs too.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Fetch and (when the reference carries a pin) sha256-verify. The
    /// anonymous scope: no registry directs the fetch (direct
    /// references, the ref-form install).
    pub fn fetch(&self, reference: &Reference) -> Result<FetchedPayload, ResolveError> {
        self.fetch_scoped(reference, None)
    }

    /// [`Fetcher::fetch`] scoped by the alias of the registry whose row
    /// directed the fetch — spec 37 §5's tier-1 key. The alias threads
    /// from the registry hit through the install plan to here.
    pub fn fetch_scoped(
        &self,
        reference: &Reference,
        alias: Option<&str>,
    ) -> Result<FetchedPayload, ResolveError> {
        let (bytes, origin) = match reference {
            Reference::Https { url, .. } => (self.scoped_get(alias, None, None, url)?, url.clone()),
            Reference::File { path, .. } => {
                // the canonical constructor — a windows drive path needs
                // the third slash (file:///C:/x), and the mock transports
                // key on exactly this form
                let url = tebako_http::file_url(std::path::Path::new(path));
                (self.scoped_get(alias, None, None, &url)?, url)
            }
            Reference::Service { service, host, .. } => self.fetch_service(
                &self.scoped_transport(alias, Some(*service), host.clone()),
                reference,
            )?,
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
                    return Err(ResolveError::GitAdapterDisabled { url: url.clone() });
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

    /// One plain GET through the credential wrapper (spec 37 §5).
    fn scoped_get(
        &self,
        alias: Option<&str>,
        service: Option<Service>,
        service_host: Option<String>,
        url: &str,
    ) -> Result<Vec<u8>, ResolveError> {
        self.scoped_transport(alias, service, service_host)
            .get(url)
            .map_err(|e| map_fetch_error(url, e))
    }

    /// The credential-applying view of this fetcher's transport (spec
    /// 37 §5) — the wrapper the fetcher's own fetches and the registry
    /// resolution pass to the adapters.
    pub(crate) fn scoped_transport(
        &self,
        alias: Option<&str>,
        service: Option<Service>,
        service_host: Option<String>,
    ) -> CredTransport<'_, T> {
        CredTransport::new(
            &self.transport,
            self.effective_book(),
            alias.map(str::to_string),
            service,
            service_host,
        )
    }

    /// Apply the multi-artifact selection rule (spec 04 §1, locked):
    /// `#artifact` → [`ServiceAdapter::asset_named`] (missing is
    /// `AssetNotFound` naming it); no `#` →
    /// [`adapters::select_candidate`] over the release's `.tfs` assets.
    /// Never a guess (spec 00 invariant 9). `scoped` is the
    /// credential-applying view carrying the directing registry's alias
    /// (spec 37 §5); `reference` is the Service arm this fetch serves.
    fn fetch_service(
        &self,
        scoped: &CredTransport<'_, T>,
        reference: &Reference,
    ) -> Result<(Vec<u8>, String), ResolveError> {
        let Reference::Service {
            service,
            host,
            owner,
            repo,
            version,
            artifact,
            ..
        } = reference
        else {
            unreachable!("fetch_service serves the Service arm only")
        };
        let adapter = crate::adapters::adapter_for_host(*service, host.as_deref())?;
        let asset = match artifact {
            Some(name) => adapter
                .asset_named(scoped, owner, repo, version, name)?
                .ok_or_else(|| ResolveError::AssetNotFound {
                    service: *service,
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    version: version.to_string(),
                    artifact: Some(name.to_string()),
                })?,
            None => {
                let assets = adapter.assets(scoped, owner, repo, version)?;
                crate::adapters::select_candidate(*service, owner, repo, version, assets)?
            }
        };
        let url = asset.url.clone();
        // An asset fetch honors the requirements the adapter declared on
        // the descriptor (accept / authenticate) — the URL alone carries
        // nothing (spec 04 §3); the credential rides the wrapper (spec
        // 37 §5).
        let bytes = scoped
            .get_asset(&asset.url, asset.accept.as_deref(), asset.authenticate)
            .map_err(|e| map_fetch_error(&asset.url, e))?;
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
            host: None,
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
            host: None,
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

    /// Records the fetch requirements the asset descriptor declared —
    /// the wiring the URL string alone cannot carry (spec 04 §3).
    struct ReqTransport {
        seen: std::sync::Mutex<Vec<(String, Option<String>, bool)>>,
    }
    impl Transport for ReqTransport {
        fn get(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
            // the release-INDEX read
            Ok(br#"{"assets":[{"name":"r-v1.tfs","url":"https://api.github.com/repos/o/r/releases/assets/11","browser_download_url":"https://dl/r-v1.tfs"}]}"#
                .to_vec())
        }
        fn get_asset(
            &self,
            url: &str,
            accept: Option<&str>,
            authenticate: bool,
        ) -> Result<Vec<u8>, FetchError> {
            self.seen.lock().unwrap().push((
                url.to_string(),
                accept.map(str::to_string),
                authenticate,
            ));
            Ok(b"img".to_vec())
        }
        fn authenticated(&self) -> bool {
            true
        }
    }

    #[test]
    fn service_fetch_honors_the_asset_descriptor_requirements() {
        let t = ReqTransport {
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let f = Fetcher::with_transport(t);
        let r = Reference::Service {
            service: Service::Github,
            host: None,
            owner: "o".into(),
            repo: "r".into(),
            version: "v1".into(),
            artifact: None,
            sha256: None,
        };
        let got = f.fetch(&r).unwrap();
        assert_eq!(got.bytes, b"img");
        assert_eq!(
            got.origin,
            "https://api.github.com/repos/o/r/releases/assets/11"
        );
        let seen = f.transport.seen.lock().unwrap();
        assert_eq!(
            seen.as_slice(),
            &[(
                "https://api.github.com/repos/o/r/releases/assets/11".to_string(),
                Some("application/octet-stream".to_string()),
                true
            )]
        );
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

    // spec 37 §5 — the credential layer at the fetch boundary

    /// Records the credential header the wrapper decided (spec 37 §5)
    /// and refuses with the service's 401 when the expected Bearer is
    /// not attached.
    type SeenHeaders = std::sync::Mutex<Vec<(String, Option<(String, String)>)>>;

    struct CredRecordingTransport {
        seen: SeenHeaders,
        /// The header VALUE the private endpoint demands; None = the
        /// endpoint answers anything (anonymous-friendly).
        demand: Option<String>,
        body: Vec<u8>,
    }
    impl Transport for CredRecordingTransport {
        fn get(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
            unreachable!("the credential wrapper rides get_with_header")
        }
        fn get_with_header(
            &self,
            url: &str,
            header: Option<(&str, &str)>,
        ) -> Result<Vec<u8>, FetchError> {
            self.seen.lock().unwrap().push((
                url.to_string(),
                header.map(|(n, v)| (n.to_string(), v.to_string())),
            ));
            match (&self.demand, header) {
                (Some(want), Some((_, got))) if want == got => Ok(self.body.clone()),
                (Some(_), _) => Err(FetchError::AuthRejected {
                    url: url.to_string(),
                    status: 401,
                }),
                (None, _) => Ok(self.body.clone()),
            }
        }
    }

    const CRED_VAR: &str = "TEBAKO_TEST_FETCH_CREDENTIAL";

    fn scoped_book() -> CredentialBook {
        CredentialBook {
            tier1: vec![crate::credentials::Tier1Entry {
                alias: "nist".to_string(),
                token_env: CRED_VAR.to_string(),
                allowed_hosts: std::collections::BTreeSet::from([
                    "api.github.com".to_string(),
                    "github.com".to_string(),
                ]),
            }],
            tier2: vec![("ghe.corp.internal".to_string(), CRED_VAR.to_string())],
            ref_index: Vec::new(),
        }
    }

    #[test]
    fn a_scoped_fetch_attaches_the_decided_bearer_verbatim() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var(CRED_VAR, "sekrit");
        let t = CredRecordingTransport {
            seen: std::sync::Mutex::new(Vec::new()),
            demand: Some("Bearer sekrit".to_string()),
            body: b"registry".to_vec(),
        };
        let f = Fetcher::with_transport(t).with_credential_book(scoped_book());
        let r = Reference::Https {
            url: "https://api.github.com/repos/acme/priv/contents/tpkg-registry.yaml".into(),
            sha256: None,
        };
        let got = f.fetch_scoped(&r, Some("nist")).unwrap();
        assert_eq!(got.bytes, b"registry");
        let seen = f.transport.seen.lock().unwrap();
        assert_eq!(
            seen.as_slice(),
            &[(
                "https://api.github.com/repos/acme/priv/contents/tpkg-registry.yaml".to_string(),
                Some(("Authorization".to_string(), "Bearer sekrit".to_string()))
            )]
        );
        std::env::remove_var(CRED_VAR);
    }

    #[test]
    fn confinement_keeps_the_tier1_token_off_other_hosts() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var(CRED_VAR, "sekrit");
        let t = CredRecordingTransport {
            seen: std::sync::Mutex::new(Vec::new()),
            demand: None,
            body: b"blob".to_vec(),
        };
        let f = Fetcher::with_transport(t).with_credential_book(scoped_book());
        let r = Reference::Https {
            url: "https://other.example.com/x.tfs".into(),
            sha256: None,
        };
        f.fetch_scoped(&r, Some("nist")).unwrap();
        let seen = f.transport.seen.lock().unwrap();
        assert_eq!(seen.as_slice()[0].1, None);
        std::env::remove_var(CRED_VAR);
    }

    #[test]
    fn the_tier2_host_entry_answers_without_an_alias() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var(CRED_VAR, "ghe-sekrit");
        let t = CredRecordingTransport {
            seen: std::sync::Mutex::new(Vec::new()),
            demand: Some("Bearer ghe-sekrit".to_string()),
            body: b"blob".to_vec(),
        };
        let f = Fetcher::with_transport(t).with_credential_book(scoped_book());
        let r = Reference::Https {
            url: "https://ghe.corp.internal/api/v3/repos/o/r/tarball/v1".into(),
            sha256: None,
        };
        let got = f.fetch(&r).unwrap();
        assert_eq!(got.bytes, b"blob");
        std::env::remove_var(CRED_VAR);
    }

    #[test]
    fn a_refusal_with_a_matched_but_unset_env_is_the_named_credential_required() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::remove_var(CRED_VAR);
        let t = CredRecordingTransport {
            seen: std::sync::Mutex::new(Vec::new()),
            demand: Some("Bearer sekrit".to_string()),
            body: Vec::new(),
        };
        let f = Fetcher::with_transport(t).with_credential_book(scoped_book());
        let r = Reference::Https {
            url: "https://api.github.com/repos/acme/priv/contents/tpkg-registry.yaml".into(),
            sha256: None,
        };
        let err = f.fetch_scoped(&r, Some("nist")).unwrap_err();
        let ResolveError::CredentialRequired {
            registry,
            host,
            looked_for,
        } = &err
        else {
            panic!("expected CredentialRequired, got {err:?}")
        };
        assert_eq!(registry.as_deref(), Some("nist"));
        assert_eq!(host, "api.github.com");
        assert_eq!(looked_for.as_deref(), Some(CRED_VAR));
        let text = err.to_string();
        assert!(text.contains("(CredentialRequired)"), "{text}");
        assert!(text.contains(CRED_VAR), "{text}");
        assert!(text.contains("registry 'nist'"), "{text}");
        assert!(text.contains("credentials:"), "{text}");
    }

    #[test]
    fn an_unscoped_refusal_names_the_host_without_an_env() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::remove_var(CRED_VAR);
        let t = CredRecordingTransport {
            seen: std::sync::Mutex::new(Vec::new()),
            demand: Some("Bearer sekrit".to_string()),
            body: Vec::new(),
        };
        let f = Fetcher::with_transport(t).with_credential_book(CredentialBook::default());
        let r = Reference::Https {
            url: "https://priv.example.com/x.tfs".into(),
            sha256: None,
        };
        let err = f.fetch(&r).unwrap_err();
        let ResolveError::CredentialRequired {
            registry,
            host,
            looked_for,
        } = &err
        else {
            panic!("expected CredentialRequired, got {err:?}")
        };
        assert_eq!(*registry, None);
        assert_eq!(host, "priv.example.com");
        assert_eq!(*looked_for, None);
    }

    #[test]
    fn the_decision_journals_the_credential_class_never_the_value() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let home =
            std::env::temp_dir().join(format!("tebako-resolve-credjournal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        crate::credentials::set_journal_home(Some(home.clone()));
        std::env::set_var(CRED_VAR, "sekrit");
        let t = CredRecordingTransport {
            seen: std::sync::Mutex::new(Vec::new()),
            demand: Some("Bearer sekrit".to_string()),
            body: b"registry".to_vec(),
        };
        let f = Fetcher::with_transport(t).with_credential_book(scoped_book());
        let r = Reference::Https {
            url: "https://api.github.com/repos/acme/priv/contents/tpkg-registry.yaml".into(),
            sha256: None,
        };
        f.fetch_scoped(&r, Some("nist")).unwrap();
        std::env::remove_var(CRED_VAR);
        crate::credentials::set_journal_home(None);
        let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
        assert!(
            journal.contains(
                "event=fetch host=api.github.com credential=token:env:TEBAKO_TEST_FETCH_CREDENTIAL"
            ),
            "{journal}"
        );
        assert!(!journal.contains("sekrit"), "{journal}");
        let _ = std::fs::remove_dir_all(&home);
    }
}
