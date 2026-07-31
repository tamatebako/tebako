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

/// The canonical file:// URL for a local path (forward slashes, the
/// third slash before an absolute unix path or a Windows drive path:
/// `file:///tmp/x`, `file:///C:/x`). `format!("file://{path}")` is only
/// accidentally right on unix; this is the one constructor, so nobody
/// hand-rolls it again.
pub fn file_url(path: &std::path::Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

/// The filesystem path a `file://` remainder names. RFC 8089: the third
/// slash separates the (empty) authority from the path — so on Windows
/// `/C:/x` is not a path at all; the drive path is `C:/x`. Unix
/// remainders begin at that slash and pass through unchanged.
pub fn file_path_from_url(remainder: &str) -> &str {
    #[cfg(windows)]
    {
        let b = remainder.as_bytes();
        if b.len() > 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' && b[3] == b'/'
        {
            return &remainder[1..];
        }
    }
    remainder
}

/// GET `url` and return the response body. `https://` (redirects
/// followed, HTTPS-only) or `file://`.
pub fn get(url: &str) -> Result<Vec<u8>, FetchError> {
    get_bearer(url, None)
}

/// [`get`] with an optional bearer token. The releases-API reads
/// authenticate: GitHub's unauthenticated tag-lookup lags (or 404s
/// outright) on fresh public releases, and the anonymous rate limit is
/// tight on CI egress IPs.
pub fn get_bearer(url: &str, bearer: Option<&str>) -> Result<Vec<u8>, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return read_file_url(file_path_from_url(path));
    }
    if !url.starts_with("https://") {
        return Err(FetchError::DownloadFailed(format!(
            "refusing non-HTTPS URL {url} (https:// and file:// are supported)"
        )));
    }
    let mut req = agent().get(url);
    if let Some(token) = bearer {
        req = req.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = req.call().map_err(map_ureq_error(url))?;
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
        let bytes = read_file_url(file_path_from_url(path))?;
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

fn require_https(url: &str) -> Result<(), FetchError> {
    if !url.starts_with("https://") {
        return Err(FetchError::DownloadFailed(format!(
            "refusing non-HTTPS URL {url} (https:// and file:// are supported)"
        )));
    }
    Ok(())
}

/// POST `body` to `url` with an optional bearer token (the release-upload
/// half of the publish channel, spec 16). HTTPS-only — uploads never ride
/// `file://`. Returns the response body; HTTP errors map like [`get`]'s.
pub fn post(
    url: &str,
    body: &[u8],
    content_type: &str,
    bearer: Option<&str>,
) -> Result<Vec<u8>, FetchError> {
    require_https(url)?;
    let mut req = agent()
        .post(url)
        .header("Content-Type", content_type)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tebako");
    if let Some(token) = bearer {
        req = req.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = req.send(body).map_err(map_ureq_error(url))?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .read_to_vec()
        .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))
}

/// DELETE `url` with an optional bearer token (asset replacement on
/// idempotent re-publish). HTTPS-only; 404 is success (the asset is
/// already gone — replacement stays idempotent).
pub fn delete(url: &str, bearer: Option<&str>) -> Result<(), FetchError> {
    require_https(url)?;
    let mut req = agent()
        .delete(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tebako");
    if let Some(token) = bearer {
        req = req.header("Authorization", &format!("Bearer {token}"));
    }
    match req.call() {
        Ok(_) => Ok(()),
        Err(ureq::Error::StatusCode(404)) => Ok(()),
        Err(e) => Err(map_ureq_error(url)(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_constructor_is_canonical_on_both_platforms() {
        // unix absolute: the third slash comes from the path itself
        assert_eq!(file_url(std::path::Path::new("/tmp/x")), "file:///tmp/x");
        // a drive path (or any non-/ path) gets the slash spelled
        #[cfg(windows)]
        assert_eq!(
            file_url(std::path::Path::new(r"C:/Users/x")),
            "file:///C:/Users/x"
        );
    }

    #[test]
    fn the_constructor_round_trips_through_get() {
        let dir =
            std::env::temp_dir().join(format!("tebako-http-test-roundtrip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("index.txt");
        std::fs::write(&file, b"hello").unwrap();
        assert_eq!(get(&file_url(&file)).unwrap(), b"hello");
    }

    #[test]
    fn file_url_reads_from_disk() {
        let dir =
            std::env::temp_dir().join(format!("tebako-http-test-file-url-{}", std::process::id()));
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
        let dir =
            std::env::temp_dir().join(format!("tebako-http-test-progress-{}", std::process::id()));
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
}
