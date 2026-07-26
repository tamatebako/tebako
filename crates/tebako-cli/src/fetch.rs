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

/// One attempt at reading `url` (redirects followed inside the client).
pub fn read_url(url: &str) -> Result<Vec<u8>, FetchError> {
    tebako_http::get(url)
}

/// `with_retries`: every failure except IndexUnavailable is retried up to
/// DOWNLOAD_ATTEMPTS times, then wrapped as DownloadFailed.
pub fn with_retries<F>(url: &str, mut f: F) -> Result<Vec<u8>, FetchError>
where
    F: FnMut() -> Result<Vec<u8>, FetchError>,
{
    let mut attempts = 0;
    loop {
        attempts += 1;
        match f() {
            Ok(body) => return Ok(body),
            Err(FetchError::IndexUnavailable(msg)) => {
                return Err(FetchError::IndexUnavailable(msg))
            }
            Err(FetchError::DownloadFailed(msg)) => {
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
