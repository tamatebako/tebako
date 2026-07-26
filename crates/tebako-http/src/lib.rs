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

/// GET `url` as text (release indexes, manifests).
pub fn get_text(url: &str) -> Result<String, FetchError> {
    let body = get(url)?;
    String::from_utf8(body).map_err(|e| FetchError::DownloadFailed(format!("{e} decoding {url}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_reads_from_disk() {
        let dir = std::env::temp_dir().join(format!("tebako-http-test-{}", std::process::id()));
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
}
