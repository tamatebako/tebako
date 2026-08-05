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
}

/// The real transport: tebako-http with the gem's retry discipline.
#[derive(Debug, Default, Clone, Copy)]
pub struct HttpTransport;

impl Transport for HttpTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let mut attempts = 0;
        let mut throttles = 0;
        loop {
            match tebako_http::get(url) {
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
        let got = HttpTransport
            .get(&format!("file://{}", file.display()))
            .unwrap();
        assert_eq!(got, b"bytes");
        assert!(matches!(
            HttpTransport.get(&format!("file://{}/missing", dir.display())),
            Err(FetchError::IndexUnavailable(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
