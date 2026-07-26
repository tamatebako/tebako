//! Download machinery (a port of the gem's net/http reader in
//! lib/tebako/runtime_manager.rb) backed by the curl CLI, the same
//! approach as tebako-bootstrap's platform.rs: redirects are followed by
//! curl itself (limit 5), `file://` URLs read from disk, a 404 maps to
//! `IndexUnavailable` (try the next index file), everything else is
//! `DownloadFailed`; the caller retries non-404 failures up to
//! DOWNLOAD_ATTEMPTS times with a fixed delay.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const DOWNLOAD_ATTEMPTS: u32 = 3;
pub const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Debug)]
pub enum FetchError {
    /// The requested object is missing (HTTP 404 / ENOENT on file://);
    /// try the next index name.
    IndexUnavailable(String),
    /// A download failed at the transport layer (after redirects).
    DownloadFailed(String),
}

fn read_file_url(path: &str) -> Result<Vec<u8>, FetchError> {
    std::fs::read(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            FetchError::IndexUnavailable(path.to_string())
        } else {
            FetchError::DownloadFailed(format!("{e} reading {path}"))
        }
    })
}

fn curl_fetch(url: &str) -> Result<Vec<u8>, FetchError> {
    // curl writes the body to a temp file (release assets are tens of MB)
    // and the status line carries the final HTTP status after redirects.
    let tmp = std::env::temp_dir().join(format!(
        "tebako-fetch-{}-{}.part",
        std::process::id(),
        nonce()
    ));
    let out = Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--max-redirs",
            "5",
            "--connect-timeout",
            "15",
            "--max-time",
            "300",
            "-o",
        ])
        .arg(&tmp)
        .args(["-w", "%{http_code}", "--"])
        .arg(url)
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(FetchError::DownloadFailed(format!(
                "curl failed to start: {e}"
            )));
        }
    };
    let http_code = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let curl_err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if !out.status.success() {
        let _ = std::fs::remove_file(&tmp);
        // curl -f is not used, so a nonzero exit is a transport failure;
        // the HTTP status (when one was received) rides along for context.
        let detail = if curl_err.is_empty() {
            format!("curl exit {} fetching {url}", status_code(&out.status))
        } else {
            format!("{curl_err} fetching {url}")
        };
        let _ = http_code;
        return Err(FetchError::DownloadFailed(detail));
    }
    match http_code.chars().next() {
        Some('2') => read_tmp(&tmp, url),
        // A status of 404 means the object does not exist; anything else
        // is a server/proxy failure worth retrying.
        _ if http_code == "404" => {
            let _ = std::fs::remove_file(&tmp);
            Err(FetchError::IndexUnavailable(url.to_string()))
        }
        _ => {
            let _ = std::fs::remove_file(&tmp);
            if http_code == "000" {
                // Should be unreachable (curl exits nonzero then), but keep
                // a sane message if curl ever returns success with no HTTP.
                Err(FetchError::DownloadFailed(format!(
                    "no HTTP response fetching {url}"
                )))
            } else {
                Err(FetchError::DownloadFailed(format!(
                    "{http_code} fetching {url}"
                )))
            }
        }
    }
}

fn status_code(status: &std::process::ExitStatus) -> String {
    status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "signal".to_string())
}

fn read_tmp(tmp: &Path, url: &str) -> Result<Vec<u8>, FetchError> {
    let body = std::fs::read(tmp);
    let _ = std::fs::remove_file(tmp);
    body.map_err(|e| FetchError::DownloadFailed(format!("{e} reading download of {url}")))
}

fn nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// One attempt at reading `url` (redirects followed inside curl).
pub fn read_url(url: &str) -> Result<Vec<u8>, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return read_file_url(path);
    }
    curl_fetch(url)
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
