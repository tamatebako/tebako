//! The fetch transport seam. Production code goes through [`HttpTransport`]
//! (tebako-http: in-process ureq + rustls, webpki-roots bundled, `file://`
//! mirrors); tests plug a mock behind the same [`Transport`] trait.
//!
//! Error semantics mirror the gem's reader (spec 04 §3, tebako-http):
//! a missing object is `IndexUnavailable`, everything else
//! `DownloadFailed`; non-404 failures are retried up to
//! [`DOWNLOAD_ATTEMPTS`] times with a fixed delay, mirroring
//! tebako-cli's fetch machinery.

use tebako_http::FetchError;

/// One attempt budget for a single GET (the gem's retry discipline).
pub const DOWNLOAD_ATTEMPTS: u32 = 3;
/// Delay between attempts (tebako-cli::fetch::RETRY_DELAY).
pub const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

/// GET `url` (`https://` or `file://`) and return the body.
pub trait Transport {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError>;

    /// GET an asset honoring the fetch requirements the adapter declared
    /// on it (spec 04 §3): `accept` is a required Accept header (the
    /// GitHub asset API serves JSON metadata without
    /// `application/octet-stream` — a poisoned cache entry, not an
    /// error); `authenticate` marks the fetch credential-eligible (the
    /// transport attaches whatever it holds for the URL's host — and
    /// nothing otherwise). The default is a plain GET: test mocks key on
    /// the URL only.
    fn get_asset(
        &self,
        url: &str,
        accept: Option<&str>,
        authenticate: bool,
    ) -> Result<Vec<u8>, FetchError> {
        let _ = (accept, authenticate);
        self.get(url)
    }

    /// Whether this transport authenticates to the service APIs (an
    /// ambient GitHub token). Adapters consult it to choose asset URLs:
    /// authenticated transports get the API asset URL — private repos
    /// answer 404 on the anonymous browser URL — everyone else gets
    /// `browser_download_url` (the CDN path, no rate budget spent).
    fn authenticated(&self) -> bool {
        false
    }
}

/// The real transport: tebako-http with the gem's retry discipline.
#[derive(Debug, Default, Clone, Copy)]
pub struct HttpTransport;

impl Transport for HttpTransport {
    fn authenticated(&self) -> bool {
        tebako_http::github_token_from_env().is_some()
    }

    fn get_asset(
        &self,
        url: &str,
        accept: Option<&str>,
        authenticate: bool,
    ) -> Result<Vec<u8>, FetchError> {
        self.get_with_retry(url, || {
            tebako_http::get_with_options(
                url,
                &tebako_http::GetOptions {
                    accept,
                    authenticate,
                },
            )
        })
    }

    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.get_with_retry(url, || tebako_http::get(url))
    }
}

impl HttpTransport {
    /// One GET through tebako-http under the gem's retry discipline
    /// (throttle schedule honored; download failures retried
    /// [`DOWNLOAD_ATTEMPTS`] times; deterministic configuration answers
    /// surface verbatim, never retried).
    fn get_with_retry(
        &self,
        url: &str,
        attempt: impl Fn() -> Result<Vec<u8>, FetchError>,
    ) -> Result<Vec<u8>, FetchError> {
        let mut attempts = 0;
        let mut throttles = 0;
        loop {
            match attempt() {
                Ok(body) => return Ok(body),
                Err(FetchError::IndexUnavailable(msg)) => {
                    return Err(FetchError::IndexUnavailable(msg))
                }
                Err(FetchError::Throttled {
                    retry_after,
                    status,
                    ..
                }) => {
                    throttles += 1;
                    if throttles >= tebako_http::THROTTLE_ROUNDS {
                        return Err(FetchError::DownloadFailed(format!(
                            "still throttled after {} backoff rounds fetching {url} ({status})",
                            tebako_http::THROTTLE_ROUNDS
                        )));
                    }
                    std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
                }
                Err(FetchError::DownloadFailed(msg)) => {
                    attempts += 1;
                    if attempts >= DOWNLOAD_ATTEMPTS {
                        return Err(FetchError::DownloadFailed(format!(
                            "failed to download {url} after {DOWNLOAD_ATTEMPTS} attempts: {msg}"
                        )));
                    }
                    std::thread::sleep(RETRY_DELAY);
                }
                // TODO.v2-1/33's named networking failures are
                // deterministic configuration answers — retried never,
                // surfaced verbatim.
                Err(
                    e @ (FetchError::ProxyAuthRequired(_) | FetchError::NetworkingCompiledOut(_)),
                ) => {
                    return Err(e);
                }
                #[cfg(feature = "network")]
                Err(e @ FetchError::NetConfig(_)) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_urls_round_trip_through_the_real_transport() {
        let dir = std::env::temp_dir().join(format!("tebako-resolve-t-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("payload.tfs");
        std::fs::write(&file, b"bytes").unwrap();
        let got = HttpTransport.get(&tebako_http::file_url(&file)).unwrap();
        assert_eq!(got, b"bytes");
        assert!(matches!(
            HttpTransport.get(&tebako_http::file_url(&dir.join("missing"))),
            Err(FetchError::IndexUnavailable(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
