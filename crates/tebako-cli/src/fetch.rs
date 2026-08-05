//! Download machinery (a port of the gem's net/http reader in
//! lib/tebako/runtime_manager.rb) on crates/tebako-http: in-process
//! HTTPS with webpki-roots bundled, redirects bounded at 5, `file://`
//! mirrors supported. A 404 maps to `IndexUnavailable` (try the next
//! index file), everything else is `DownloadFailed`; the caller retries
//! non-404 failures up to DOWNLOAD_ATTEMPTS times with a fixed delay.

use std::io::Write;
use std::path::PathBuf;

pub use tebako_http::FetchError;

pub const DOWNLOAD_ATTEMPTS: u32 = 3;
pub const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

/// Throttling is a schedule, not a failure: the round count + backoff
/// policy live in tebako-http (THROTTLE_ROUNDS / throttle_backoff), the
/// SSOT every crate's retry loop shares.
pub const THROTTLE_ATTEMPTS: u32 = tebako_http::THROTTLE_ROUNDS;

/// One attempt at reading `url` (redirects followed inside the client).
pub fn read_url(url: &str) -> Result<Vec<u8>, FetchError> {
    tebako_http::get(url)
}

/// `with_retries`: IndexUnavailable returns immediately (try the next
/// index name); Throttled honors the server's schedule (bounded by
/// THROTTLE_ATTEMPTS); other failures retry up to DOWNLOAD_ATTEMPTS with
/// a fixed delay.
pub fn with_retries<F>(url: &str, mut f: F) -> Result<Vec<u8>, FetchError>
where
    F: FnMut() -> Result<Vec<u8>, FetchError>,
{
    let mut attempts = 0;
    let mut throttles = 0;
    loop {
        match f() {
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
                if throttles >= THROTTLE_ATTEMPTS {
                    return Err(FetchError::DownloadFailed(format!(
                        "still throttled after {THROTTLE_ATTEMPTS} backoff rounds fetching {url} ({status})"
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

pub fn fetch_text(url: &str) -> Result<String, FetchError> {
    let body = with_retries(url, || read_url(url))?;
    String::from_utf8(body).map_err(|e| FetchError::DownloadFailed(format!("{e} decoding {url}")))
}

pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, FetchError> {
    with_retries(url, || read_url(url))
}

/// Write `bytes` to `path` (helper kept here so download tmp handling
/// lives in one place).
pub fn write_tmp(path: &PathBuf, bytes: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)
}
