//! tebako-http — in-process HTTPS downloads for the tebako stack.
//!
//! One rule, one client: ureq + rustls (ring provider) with Mozilla's
//! webpki-roots **bundled** — the OS trust store is never consulted
//! unless `TEBAKO_TLS_PLATFORM_ROOTS` is set (env opt-in). HTTPS-only
//! (plain `http://` URLs and redirect downgrades are rejected), redirects
//! bounded at [`REDIRECT_LIMIT`], connect timeout 15 s, global timeout
//! 300 s (the gem's net/http timeouts). `file://` URLs read from disk so
//! `TEBAKO_*_MIRROR=file://...` works with no network stack at all.
//!
//! Error semantics mirror the gem's reader: a missing object (HTTP 404 /
//! ENOENT on `file://`) is [`FetchError::IndexUnavailable`] — try the
//! next index name; everything else is [`FetchError::DownloadFailed`].

use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

/// Redirects followed before giving up (the gem's REDIRECT_LIMIT).
pub const REDIRECT_LIMIT: u32 = 5;
/// Connect timeout (the gem's open_timeout).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Global per-request timeout (the gem's read_timeout).
pub const GLOBAL_TIMEOUT: Duration = Duration::from_secs(300);

/// Set to opt into the OS trust store instead of the bundled
/// webpki-roots (corporate MITM proxies and the like).
pub const PLATFORM_ROOTS_ENV: &str = "TEBAKO_TLS_PLATFORM_ROOTS";

/// Response body cap (ureq's default is 10 MiB; release assets are
/// tens of MB). Still bounded against memory exhaustion.
pub const MAX_BODY_SIZE: u64 = 512 * 1024 * 1024;

#[derive(Debug)]
pub enum FetchError {
    /// The requested object is missing (HTTP 404 / ENOENT on file://);
    /// try the next index name.
    IndexUnavailable(String),
    /// A download failed at the transport or HTTP layer.
    DownloadFailed(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::IndexUnavailable(what) => write!(f, "not found: {what}"),
            FetchError::DownloadFailed(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for FetchError {}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let root_certs = if std::env::var_os(PLATFORM_ROOTS_ENV).is_some() {
            ureq::tls::RootCerts::PlatformVerifier
        } else {
            ureq::tls::RootCerts::WebPki
        };
        let tls = ureq::tls::TlsConfig::builder()
            .root_certs(root_certs)
            .build();
        ureq::Agent::config_builder()
            .tls_config(tls)
            .https_only(true)
            .max_redirects(REDIRECT_LIMIT)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(GLOBAL_TIMEOUT))
            .build()
            .into()
    })
}

fn map_ureq_error(url: &str) -> impl Fn(ureq::Error) -> FetchError + '_ {
    move |e| match e {
        ureq::Error::StatusCode(404) => FetchError::IndexUnavailable(url.to_string()),
        ureq::Error::StatusCode(code) => {
            FetchError::DownloadFailed(format!("{code} fetching {url}"))
        }
        other => FetchError::DownloadFailed(format!("{other} fetching {url}")),
    }
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

/// GET `url` and return the response body. `https://` (redirects
/// followed, HTTPS-only) or `file://`.
pub fn get(url: &str) -> Result<Vec<u8>, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return read_file_url(path);
    }
    if !url.starts_with("https://") {
        return Err(FetchError::DownloadFailed(format!(
            "refusing non-HTTPS URL {url} (https:// and file:// are supported)"
        )));
    }
    let mut response = agent().get(url).call().map_err(map_ureq_error(url))?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .read_to_vec()
        .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))
}

/// [`get`] with a progress hook: `on_progress(bytes_so_far,
/// content_length)` fires per read chunk on the download path (spec 06
/// §5 — the bar is transport-accurate, not estimated). `content_length`
/// is None when the response is chunked/close-delimited. When the hook
/// is None the behavior is [`get`]'s, unchanged.
pub fn get_with_progress(
    url: &str,
    on_progress: Option<&mut dyn FnMut(u64, Option<u64>)>,
) -> Result<Vec<u8>, FetchError> {
    let Some(cb) = on_progress else {
        return get(url);
    };
    if let Some(path) = url.strip_prefix("file://") {
        let bytes = read_file_url(path)?;
        cb(bytes.len() as u64, Some(bytes.len() as u64));
        return Ok(bytes);
    }
    if !url.starts_with("https://") {
        return Err(FetchError::DownloadFailed(format!(
            "refusing non-HTTPS URL {url} (https:// and file:// are supported)"
        )));
    }
    use std::io::Read as _;
    let mut response = agent().get(url).call().map_err(map_ureq_error(url))?;
    let content_length = response.body().content_length();
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .reader();
    let mut body: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 65536];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        cb(body.len() as u64, content_length);
    }
    Ok(body)
}

/// GET `url` as text (release indexes, manifests).
pub fn get_text(url: &str) -> Result<String, FetchError> {
    let body = get(url)?;
    String::from_utf8(body).map_err(|e| FetchError::DownloadFailed(format!("{e} decoding {url}")))
}

/// The write side of the release APIs (roadmap 41 — `tebako publish`):
/// send `method` (GET/POST/PUT/DELETE) to `url` with `headers` and an
/// optional body, returning the status code and the response body. Any
/// completed HTTP exchange is `Ok` — non-2xx statuses come back as
/// `(code, [])` (ureq reports them without a body) for the caller's named
/// errors; only transport failures are `Err`. HTTPS-only like [`get`];
/// `file://` is supported for GET only (tests and air-gapped mirrors).
pub fn request(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<(u16, Vec<u8>), FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return match method {
            "GET" => match std::fs::read(path) {
                Ok(bytes) => Ok((200, bytes)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((404, Vec::new())),
                Err(e) => Err(FetchError::DownloadFailed(format!("{e} reading {path}"))),
            },
            _ => Err(FetchError::DownloadFailed(format!(
                "refusing {method} on file:// URL {url} (GET only)"
            ))),
        };
    }
    if !url.starts_with("https://") {
        return Err(FetchError::DownloadFailed(format!(
            "refusing non-HTTPS URL {url} (https:// and file:// are supported)"
        )));
    }
    let agent = agent();
    // ureq's typestate splits body-less methods (WithoutBody) from
    // body-carrying ones (WithBody); build each arm with its own headers.
    let result = match method {
        "GET" | "DELETE" => {
            let builder = match method {
                "GET" => agent.get(url),
                _ => agent.delete(url),
            };
            let builder = headers
                .iter()
                .fold(builder, |b, (name, value)| b.header(*name, *value));
            match body {
                None => builder.call(),
                Some(_) => {
                    return Err(FetchError::DownloadFailed(format!(
                        "{method} carries no body ({url})"
                    )))
                }
            }
        }
        "POST" | "PUT" => {
            let builder = match method {
                "POST" => agent.post(url),
                _ => agent.put(url),
            };
            let builder = headers
                .iter()
                .fold(builder, |b, (name, value)| b.header(*name, *value));
            builder.send(body.unwrap_or(&[]))
        }
        other => {
            return Err(FetchError::DownloadFailed(format!(
                "unsupported HTTP method {other}"
            )))
        }
    };
    match result {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let body = response
                .body_mut()
                .with_config()
                .limit(MAX_BODY_SIZE)
                .read_to_vec()
                .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))?;
            Ok((status, body))
        }
        Err(ureq::Error::StatusCode(code)) => Ok((code, Vec::new())),
        Err(other) => Err(FetchError::DownloadFailed(format!(
            "{other} fetching {url}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_reads_from_disk() {
        let dir = std::env::temp_dir().join(format!("tebako-http-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("index.txt");
        std::fs::write(&file, b"hello").unwrap();
        assert_eq!(
            get(&format!("file://{}", file.display())).unwrap(),
            b"hello"
        );
        let missing = dir.join("missing.txt");
        assert!(matches!(
            get(&format!("file://{}", missing.display())),
            Err(FetchError::IndexUnavailable(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plain_http_is_refused() {
        assert!(matches!(
            get("http://example.com/"),
            Err(FetchError::DownloadFailed(_))
        ));
    }

    #[test]
    fn progress_hook_fires_once_for_file_urls() {
        let dir = std::env::temp_dir().join(format!("tebako-http-progress-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("asset.bin");
        std::fs::write(&file, b"hello progress").unwrap();
        let url = format!("file://{}", file.display());

        let mut calls: Vec<(u64, Option<u64>)> = Vec::new();
        let body = get_with_progress(&url, Some(&mut |so_far, total| calls.push((so_far, total))))
            .unwrap();
        assert_eq!(body, b"hello progress");
        assert_eq!(calls, vec![(14, Some(14))]);

        // None is get()'s behavior, unchanged.
        assert_eq!(get_with_progress(&url, None).unwrap(), get(&url).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn request_gets_file_urls_and_refuses_writes_and_plain_http() {
        let dir = std::env::temp_dir().join(format!("tebako-http-req-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("api.json");
        std::fs::write(&file, b"{}").unwrap();
        let url = format!("file://{}", file.display());
        assert_eq!(request("GET", &url, &[], None).unwrap(), (200, b"{}".to_vec()));
        assert_eq!(
            request("GET", &format!("file://{}/missing", dir.display()), &[], None).unwrap(),
            (404, Vec::new())
        );
        assert!(matches!(
            request("POST", &url, &[], Some(b"x")),
            Err(FetchError::DownloadFailed(_))
        ));
        assert!(matches!(
            request("POST", "http://example.com/", &[], Some(b"x")),
            Err(FetchError::DownloadFailed(_))
        ));
        assert!(matches!(
            request("PATCH", &url, &[], None),
            Err(FetchError::DownloadFailed(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
