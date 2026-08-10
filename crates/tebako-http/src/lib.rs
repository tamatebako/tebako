//! tebako-http — in-process HTTPS downloads for the tebako stack.
//!
//! One rule, one client: ureq + rustls with Mozilla's webpki-roots
//! **bundled** — the OS trust store is never consulted unless
//! `TEBAKO_TLS_PLATFORM_ROOTS` is set (env opt-in). The rustls crypto
//! provider is ring everywhere except windows-gnu, where ring 0.17 does
//! not compile: there it is aws-lc-rs, set explicitly on the TlsConfig
//! (ureq's `rustls-no-provider` + documented provider-swap pattern).
//! HTTPS-only (plain `http://` URLs and redirect downgrades are
//! rejected), redirects bounded at [`REDIRECT_LIMIT`], connect timeout
//! 15 s, global timeout 300 s (the gem's net/http timeouts). `file://`
//! URLs read from disk so `TEBAKO_*_MIRROR=file://...` works with no
//! network stack at all.
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
/// Upload timeout (the release-asset channel — see [`upload_agent`]).
pub const UPLOAD_TIMEOUT: Duration = Duration::from_secs(1800);

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
    /// The server asked us to slow down (HTTP 429, or 403 with
    /// rate-limit headers). `retry_after` is the server's own hint when
    /// it sent one (Retry-After, or X-RateLimit-Reset with Remaining: 0)
    /// — the caller MUST honor it: a throttled answer is a schedule, not
    /// a failure.
    Throttled {
        url: String,
        status: u16,
        retry_after: Option<std::time::Duration>,
    },
    /// A download failed at the transport or HTTP layer.
    DownloadFailed(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::IndexUnavailable(what) => write!(f, "not found: {what}"),
            FetchError::Throttled {
                status,
                retry_after,
                ..
            } => match retry_after {
                Some(d) => write!(f, "throttled ({status}, retry after {}s)", d.as_secs()),
                None => write!(f, "throttled ({status})"),
            },
            FetchError::DownloadFailed(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for FetchError {}

fn build_agent(global_timeout: Duration) -> ureq::Agent {
    let root_certs = if std::env::var_os(PLATFORM_ROOTS_ENV).is_some() {
        ureq::tls::RootCerts::PlatformVerifier
    } else {
        ureq::tls::RootCerts::WebPki
    };
    let tls_builder = ureq::tls::TlsConfig::builder().root_certs(root_certs);
    // windows-gnu: ureq is built `rustls-no-provider` (ring does not
    // compile under mingw), so the provider must be named explicitly —
    // ureq's documented aws-lc-rs swap.
    #[cfg(all(windows, target_env = "gnu"))]
    let tls_builder = tls_builder.unversioned_rustls_crypto_provider(std::sync::Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ));
    let tls = tls_builder.build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .https_only(true)
        .max_redirects(REDIRECT_LIMIT)
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(global_timeout))
        // statuses are mapped by the caller: the rate-limit headers
        // ride the RESPONSE, and ureq's StatusCode error drops them.
        .http_status_as_error(false)
        .build()
        .into()
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| build_agent(GLOBAL_TIMEOUT))
}

/// The upload channel (release assets): 100 MB+ payloads on a degraded
/// backend blow the 300 s global request timeout (the metanorma 1.16.9
/// publish died at it, 2026-08-10; the factory's publish learned the
/// same lesson the same night). Bounded but generous — 30 min covers a
/// 150 MB asset at ~100 KB/s.
fn upload_agent() -> &'static ureq::Agent {
    static UPLOAD_AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    UPLOAD_AGENT.get_or_init(|| build_agent(UPLOAD_TIMEOUT))
}

/// Seconds from a Retry-After header value (delta-seconds form; the
/// HTTP-date form is GitHub-irrelevant — it always sends seconds).
fn parse_retry_after(value: &str) -> Option<std::time::Duration> {
    let secs: u64 = value.trim().parse().ok()?;
    Some(std::time::Duration::from_secs(secs))
}

/// The throttle schedule a response carries, if any: Retry-After wins;
/// X-RateLimit-Remaining: 0 + X-RateLimit-Reset (epoch seconds) is the
/// primary-limit form. Neither header ⇒ no hint (the caller escalates).
fn throttle_hint_from(get: impl Fn(&str) -> Option<String>) -> Option<std::time::Duration> {
    if let Some(d) = get("retry-after").and_then(|v| parse_retry_after(&v)) {
        return Some(d);
    }
    let remaining = get("x-ratelimit-remaining").and_then(|v| v.trim().parse::<u64>().ok());
    if remaining == Some(0) {
        if let Some(reset) = get("x-ratelimit-reset").and_then(|v| v.trim().parse::<u64>().ok()) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            return Some(std::time::Duration::from_secs(reset.saturating_sub(now)));
        }
    }
    None
}

fn throttle_hint(response: &ureq::http::Response<ureq::Body>) -> Option<std::time::Duration> {
    throttle_hint_from(|name| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    })
}

/// The throttle schedule policy, shared by every retry loop in the
/// ecosystem — GitHub's own rules, verbatim:
/// 1. `retry-after` present ⇒ wait exactly that long.
/// 2. `x-ratelimit-remaining: 0` ⇒ wait until `x-ratelimit-reset`
///    (epoch seconds).
/// 3. Otherwise (a bare 403/429) ⇒ wait at least one minute, then
///    exponentially longer between retries (60s × 2ⁿ), and give up
///    after THROTTLE_ROUNDS — continuing to fire while limited gets
///    the integration BANNED, so every wait is honored in full and no
///    retry ever fires early.
pub const THROTTLE_ROUNDS: u32 = 6;

pub fn throttle_backoff(attempt: u32, hint: Option<std::time::Duration>) -> std::time::Duration {
    hint.unwrap_or_else(|| {
        // the hint-less exponential: 60, 120, 240, 480, 960 s
        let shift = (attempt.max(1) - 1).min(4);
        std::time::Duration::from_secs(60 << shift)
    })
}

/// The one status→error mapping (http_status_as_error is false so the
/// throttle headers survive to here): 2xx hands the response back, 404
/// is IndexUnavailable, 429 / 403-with-rate-limit-headers is Throttled
/// with the server's own schedule, anything else is DownloadFailed.
fn classify(
    response: ureq::http::Response<ureq::Body>,
    url: &str,
) -> Result<ureq::http::Response<ureq::Body>, FetchError> {
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(response);
    }
    if status == 404 {
        return Err(FetchError::IndexUnavailable(url.to_string()));
    }
    if status == 429 || (status == 403 && throttle_hint(&response).is_some()) {
        return Err(FetchError::Throttled {
            url: url.to_string(),
            status,
            retry_after: throttle_hint(&response),
        });
    }
    Err(FetchError::DownloadFailed(format!(
        "{status} fetching {url}"
    )))
}

fn map_ureq_error(url: &str) -> impl Fn(ureq::Error) -> FetchError + '_ {
    move |e| FetchError::DownloadFailed(format!("{e} fetching {url}"))
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
    // http_status_as_error(false): statuses are mapped HERE, because the
    // throttle schedule lives in the response headers (ureq's StatusCode
    // error drops them).
    let response = req.call().map_err(map_ureq_error(url))?;
    let mut response = classify(response, url)?;
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
    let response = agent().get(url).call().map_err(map_ureq_error(url))?;
    let mut response = classify(response, url)?;
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
    let mut req = upload_agent()
        .post(url)
        .header("Content-Type", content_type)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tebako");
    if let Some(token) = bearer {
        req = req.header("Authorization", &format!("Bearer {token}"));
    }
    let response = req.send(body).map_err(map_ureq_error(url))?;
    let mut response = classify(response, url)?;
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
        Ok(response) => match classify(response, url) {
            Ok(_) => Ok(()),
            // 404 is success on delete: the asset is already gone —
            // replacement stays idempotent.
            Err(FetchError::IndexUnavailable(_)) => Ok(()),
            Err(e) => Err(e),
        },
        Err(e) => Err(map_ureq_error(url)(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_parses_delta_seconds() {
        assert_eq!(
            parse_retry_after("60"),
            Some(std::time::Duration::from_secs(60))
        );
        assert_eq!(
            parse_retry_after(" 5 "),
            Some(std::time::Duration::from_secs(5))
        );
        assert_eq!(parse_retry_after("soon"), None);
        assert_eq!(parse_retry_after(""), None);
    }

    #[test]
    fn throttle_hint_prefers_retry_after() {
        let h = throttle_hint_from(|name| match name {
            "retry-after" => Some("42".to_string()),
            "x-ratelimit-remaining" => Some("0".to_string()),
            "x-ratelimit-reset" => Some("9999999999".to_string()),
            _ => None,
        });
        assert_eq!(h, Some(std::time::Duration::from_secs(42)));
    }

    #[test]
    fn throttle_hint_falls_back_to_ratelimit_reset() {
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 120;
        let h = throttle_hint_from(|name| match name {
            "x-ratelimit-remaining" => Some("0".to_string()),
            "x-ratelimit-reset" => Some(future.to_string()),
            _ => None,
        });
        let d = h.expect("reset-based hint");
        assert!(d.as_secs() > 100 && d.as_secs() <= 120, "{}", d.as_secs());
    }

    #[test]
    fn throttle_hint_absent_without_headers() {
        assert_eq!(throttle_hint_from(|_| None), None);
        // remaining > 0 is not throttling
        let h = throttle_hint_from(|name| match name {
            "x-ratelimit-remaining" => Some("57".to_string()),
            _ => None,
        });
        assert_eq!(h, None);
    }

    #[test]
    fn hintless_backoff_is_the_documented_one_minute_then_exponential() {
        assert_eq!(
            throttle_backoff(1, None),
            std::time::Duration::from_secs(60)
        );
        assert_eq!(
            throttle_backoff(2, None),
            std::time::Duration::from_secs(120)
        );
        assert_eq!(
            throttle_backoff(3, None),
            std::time::Duration::from_secs(240)
        );
        assert_eq!(
            throttle_backoff(5, None),
            std::time::Duration::from_secs(960)
        );
        // the hint always wins
        assert_eq!(
            throttle_backoff(3, Some(std::time::Duration::from_secs(17))),
            std::time::Duration::from_secs(17)
        );
    }

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
