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
//! next index name; a plain 401/403 is [`FetchError::AuthRejected`] (the
//! credential layer's remap point, spec 37 §5); a throttling 429/403 is
//! [`FetchError::Throttled`]; everything else is
//! [`FetchError::DownloadFailed`].

use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

#[cfg(feature = "network")]
pub mod netconfig;
#[cfg(feature = "network")]
pub use netconfig::{global as network_config, set_global as set_network_config};

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

#[derive(Debug, Clone)]
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
    /// The server refused the presented (or absent) credential: a plain
    /// 401, or a 403 WITHOUT rate-limit headers (a throttling 403 stays
    /// [`FetchError::Throttled`] — a schedule, not a refusal). Terminal,
    /// never retried: tebako-resolve's credential layer remaps this to
    /// the named [`FetchError::CredentialRequired`] (spec 37 §5).
    AuthRejected { url: String, status: u16 },
    /// The fetch was refused and the credential book holds no usable
    /// credential for the host (spec 37 §5, exit class 69). Data-only:
    /// produced by tebako-resolve's credential-applying transport
    /// wrapper out of [`FetchError::AuthRejected`], never by this
    /// crate's classifiers. `registry` is the book alias whose row
    /// directed the fetch, `looked_for` the env var NAME a matched
    /// entry wanted (both absent for an out-of-book refusal).
    CredentialRequired {
        host: String,
        registry: Option<String>,
        looked_for: Option<String>,
    },
    /// The caller aborted the stream (the fetch pipeline's plan-cancel
    /// path): the progress callback returned `false`. Never retried —
    /// the plan is already unwinding.
    Cancelled(String),
    /// The proxy demanded authentication (HTTP 407). The credentials
    /// ride the proxy URL (`http://user:pass@host:port`) or the
    /// `network.proxy` config value.
    ProxyAuthRequired(String),
    /// The network config failed validation (bad proxy URL, unreadable
    /// or malformed extra-CA PEM, platform+additive conflict).
    #[cfg(feature = "network")]
    NetConfig(netconfig::NetConfigError),
    /// Proxy/TLS-anchor env or config is set but this binary was built
    /// without the `network` feature (the size-gated bootstrap) — the
    /// enterprise-networking knobs are honored by the full toolchain
    /// (`tebako`, `tebako-shim`); the bootstrap resolves from the warm
    /// store those installs produce.
    NetworkingCompiledOut(String),
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
            FetchError::AuthRejected { url, status } => write!(
                f,
                "{status} fetching {url} — the credential was rejected (or one is required and none was presented)"
            ),
            FetchError::CredentialRequired {
                host,
                registry,
                looked_for,
            } => {
                let scope = match registry {
                    Some(alias) => format!("registry '{alias}'"),
                    None => format!("host {host}"),
                };
                let env = match looked_for {
                    Some(var) => format!(" — its credential names the {var} env var, which is not set;"),
                    None => String::new(),
                };
                write!(
                    f,
                    "{host} refused the fetch (401/403) and {scope} has no usable credential{env} register one under `credentials:` in ~/.tebako/config.yaml (CredentialRequired)"
                )
            }
            FetchError::Cancelled(what) => write!(f, "fetch of {what} cancelled"),
            FetchError::ProxyAuthRequired(url) => write!(
                f,
                "proxy authentication required (407) fetching {url} — credentials ride \
                 the proxy URL (http://user:pass@host:port) or network.proxy in \
                 ~/.tebako/config.yaml"
            ),
            FetchError::NetworkingCompiledOut(what) => write!(
                f,
                "{what} is set but this binary has the enterprise-networking feature \
                 compiled out — use the full toolchain (tebako / tebako-shim) to fetch \
                 through proxies or with custom CAs, or pre-seed the store"
            ),
            #[cfg(feature = "network")]
            FetchError::NetConfig(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FetchError {}

#[cfg(feature = "network")]
impl From<netconfig::NetConfigError> for FetchError {
    fn from(e: netconfig::NetConfigError) -> Self {
        FetchError::NetConfig(e)
    }
}

/// The TlsConfig for a chosen root store; the windows-gnu provider swap
/// (ring does not compile under mingw — ureq is built
/// `rustls-no-provider` there) applies to every variant uniformly.
fn tls_config_for(root_certs: ureq::tls::RootCerts) -> ureq::tls::TlsConfig {
    let tls_builder = ureq::tls::TlsConfig::builder().root_certs(root_certs);
    #[cfg(all(windows, target_env = "gnu"))]
    let tls_builder = tls_builder.unversioned_rustls_crypto_provider(std::sync::Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ));
    tls_builder.build()
}

/// With the `network` feature, the effective [`netconfig::NetworkConfig`]
/// (env over the config file's `network:` section) drives the transport:
/// explicit or env proxy, and the trust-anchor choice (bundled webpki /
/// platform verifier / bundled + extra PEMs). Validation failures are
/// named errors at first use — never a silent fallback to direct/plain.
#[cfg(feature = "network")]
fn build_agent(global_timeout: Duration) -> Result<ureq::Agent, FetchError> {
    build_agent_with(global_timeout, true)
}

/// [`build_agent`] with the `https_only` latch as a parameter — `false`
/// is the distribution surface's ([`raw_agent`]; the entry points enforce
/// the https-or-loopback policy themselves, spec 38 §8).
#[cfg(feature = "network")]
fn build_agent_with(global_timeout: Duration, https_only: bool) -> Result<ureq::Agent, FetchError> {
    agent_with_config_full(&netconfig::global(), global_timeout, https_only)
}

/// The one agent constructor (network feature): TLS roots and proxy from
/// `cfg`, the `https_only` latch a parameter.
#[cfg(feature = "network")]
fn agent_with_config_full(
    cfg: &netconfig::NetworkConfig,
    global_timeout: Duration,
    https_only: bool,
) -> Result<ureq::Agent, FetchError> {
    let tls = tls_config_for(netconfig::resolve_roots(cfg)?);
    let mut builder = ureq::Agent::config_builder()
        .tls_config(tls)
        .https_only(https_only)
        .max_redirects(REDIRECT_LIMIT)
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(global_timeout))
        // statuses are mapped by the caller: the rate-limit headers
        // ride the RESPONSE, and ureq's StatusCode error drops them.
        .http_status_as_error(false);
    if let Some(proxy) = netconfig::resolve_proxy(cfg)? {
        builder = builder.proxy(Some(proxy));
    }
    Ok(builder.build().into())
}

/// Testing seam: an agent on an EXPLICIT network config, bypassing the
/// process global (the MITM fixture's scenarios share one process —
/// the OnceLock-cached agent can't serve them all).
#[cfg(feature = "network")]
#[doc(hidden)]
pub fn agent_with_config(
    cfg: &netconfig::NetworkConfig,
    global_timeout: Duration,
) -> Result<ureq::Agent, FetchError> {
    agent_with_config_full(cfg, global_timeout, true)
}

/// The spec 35 §3 TLS probe verdict (tebako doctor's network section).
#[cfg(feature = "network")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsProbe {
    /// The served chain verifies under the effective configuration.
    EffectiveOk,
    /// The effective roots reject the chain but the PLATFORM verifier
    /// accepts it — a TLS-intercepting proxy (or an enterprise root the
    /// bundled store does not carry) is in the path.
    PlatformOnly,
    /// Neither the effective nor the platform roots accept the chain.
    NeitherTrusted,
    /// The host did not answer at all (DNS/connect/timeout/proxy) — a
    /// network condition, not a trust finding.
    Unreachable(String),
}

/// One HTTPS GET against `https://<host>/` under `effective`, and — only
/// when the effective roots reject the chain — a second under the
/// platform verifier. Diagnostic only (spec 35 §3): read-only (the
/// response body is dropped unread), no state changes. The platform leg
/// keeps the effective proxy and drops `extra_ca` (platform+additive is
/// a validation conflict by rule; the comparison is roots-vs-roots).
#[cfg(feature = "network")]
pub fn probe_tls(host: &str, effective: &netconfig::NetworkConfig, timeout: Duration) -> TlsProbe {
    let attempt = |cfg: &netconfig::NetworkConfig| -> Result<(), ureq::Error> {
        let agent = agent_with_config(cfg, timeout)
            .map_err(|e| ureq::Error::Io(std::io::Error::other(e.to_string())))?;
        // http_status_as_error(false) is set on the testing-seam agent:
        // ANY HTTP response means the handshake (the probe's subject)
        // completed. The body is never read.
        agent.get(&format!("https://{host}/")).call().map(|_| ())
    };
    match attempt(effective) {
        Ok(()) => TlsProbe::EffectiveOk,
        Err(e) if !is_cert_rejection(&e) => TlsProbe::Unreachable(format!("{e}")),
        Err(_first_leg) => {
            if effective.tls_roots == netconfig::TlsRoots::Platform {
                return TlsProbe::NeitherTrusted;
            }
            let platform = netconfig::NetworkConfig {
                proxy_url: effective.proxy_url.clone(),
                tls_roots: netconfig::TlsRoots::Platform,
                extra_ca: Vec::new(),
                audit: Vec::new(),
            };
            match attempt(&platform) {
                Ok(()) => TlsProbe::PlatformOnly,
                Err(pe) if is_cert_rejection(&pe) => TlsProbe::NeitherTrusted,
                Err(pe) => TlsProbe::Unreachable(format!("{pe}")),
            }
        }
    }
}

/// ureq surfaces a TLS handshake failure as `Error::Io` with the rustls
/// error as the inner cause (rustls's `From<rustls::Error> for
/// io::Error` is `InvalidData`). The platform verifier's rejections land
/// in the same `InvalidCertificate` shape.
#[cfg(feature = "network")]
fn is_cert_rejection(e: &ureq::Error) -> bool {
    if let ureq::Error::Io(io) = e {
        if let Some(inner) = io.get_ref() {
            if let Some(re) = inner.downcast_ref::<rustls::Error>() {
                return matches!(re, rustls::Error::InvalidCertificate(_));
            }
        }
        return format!("{io}").contains("certificate");
    }
    false
}

/// Without the feature (the size-gated bootstrap) the transport is
/// exactly the pre-feature behavior: bundled roots, or the platform
/// verifier via `TEBAKO_TLS_PLATFORM_ROOTS`; no proxy, no extra CAs.
#[cfg(not(feature = "network"))]
fn build_agent(global_timeout: Duration) -> Result<ureq::Agent, FetchError> {
    build_agent_with(global_timeout, true)
}

/// [`build_agent`] with the `https_only` latch as a parameter (the
/// distribution surface's [`raw_agent`] passes `false`).
#[cfg(not(feature = "network"))]
fn build_agent_with(global_timeout: Duration, https_only: bool) -> Result<ureq::Agent, FetchError> {
    let root_certs = if std::env::var_os(PLATFORM_ROOTS_ENV).is_some() {
        ureq::tls::RootCerts::PlatformVerifier
    } else {
        ureq::tls::RootCerts::WebPki
    };
    Ok(ureq::Agent::config_builder()
        .tls_config(tls_config_for(root_certs))
        .https_only(https_only)
        .max_redirects(REDIRECT_LIMIT)
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(global_timeout))
        .http_status_as_error(false)
        .build()
        .into())
}

/// The feature-off guard: a proxy/custom-CA env on a compiled-out binary
/// is a named error at first use, not a connect failure nor (worse) a
/// silently direct fetch.
#[cfg(not(feature = "network"))]
fn network_guard() -> Result<(), FetchError> {
    for var in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if std::env::var_os(var).is_some() {
            return Err(FetchError::NetworkingCompiledOut(var.to_string()));
        }
    }
    if std::env::var_os("TEBAKO_EXTRA_CA").is_some() {
        return Err(FetchError::NetworkingCompiledOut(
            "TEBAKO_EXTRA_CA".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "network")]
fn network_guard() -> Result<(), FetchError> {
    Ok(())
}

fn agent() -> Result<&'static ureq::Agent, FetchError> {
    static AGENT: OnceLock<Result<ureq::Agent, FetchError>> = OnceLock::new();
    AGENT
        .get_or_init(|| build_agent(GLOBAL_TIMEOUT))
        .as_ref()
        .map_err(Clone::clone)
}

/// The upload channel (release assets): 100 MB+ payloads on a degraded
/// backend blow the 300 s global request timeout (the metanorma 1.16.9
/// publish died at it, 2026-08-10; the factory's publish learned the
/// same lesson the same night). Bounded but generous — 30 min covers a
/// 150 MB asset at ~100 KB/s.
fn upload_agent() -> Result<&'static ureq::Agent, FetchError> {
    static UPLOAD_AGENT: OnceLock<Result<ureq::Agent, FetchError>> = OnceLock::new();
    UPLOAD_AGENT
        .get_or_init(|| build_agent(UPLOAD_TIMEOUT))
        .as_ref()
        .map_err(Clone::clone)
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
    if status == 407 {
        return Err(FetchError::ProxyAuthRequired(url.to_string()));
    }
    if status == 429 || (status == 403 && throttle_hint(&response).is_some()) {
        return Err(FetchError::Throttled {
            url: url.to_string(),
            status,
            retry_after: throttle_hint(&response),
        });
    }
    // A plain 401/403 is a CREDENTIAL answer (spec 37 §5): terminal,
    // never retried — the credential layer remaps it to the named
    // CredentialRequired. A 403 carrying rate-limit headers is a
    // schedule and was handled above.
    if status == 401 || status == 403 {
        return Err(FetchError::AuthRejected {
            url: url.to_string(),
            status,
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

/// The ambient GitHub token: `TEBAKO_GITHUB_TOKEN` wins, `GITHUB_TOKEN`
/// is the CI spelling (Actions sets it ambiently). Resolution reads
/// against the GitHub API authenticate when one is present — the
/// anonymous budget is 60 requests/h per egress IP and CI NAT pools
/// share theirs across tenants, so an unauthenticated install in CI is
/// a coin flip against a bucket other tenants already drained.
pub fn github_token_from_env() -> Option<String> {
    std::env::var("TEBAKO_GITHUB_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
        .or_else(|| std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()))
}

/// The one host the ambient token may ride. Browser-URL asset downloads
/// (`github.com/.../releases/download/...`) are pre-signed redirects that
/// need no credential; attaching a bearer to any other host — including a
/// redirect target off GitHub — leaks it. (ureq never forwards auth
/// headers on redirect — `RedirectAuthHeaders::Never` is its default —
/// so the pre-signed redirect target of an api.github.com asset GET
/// never sees the token either.)
///
/// pub: tebako-resolve's credential book (spec 37 §5) ranks the ambient
/// token as its github.com tier — ONE host check, here, never
/// re-implemented.
pub fn carries_ambient_github_token(url: &str) -> bool {
    url.starts_with("https://api.github.com/")
}

/// What one GET needs beyond the URL (spec 04 §3). The caller — the
/// resolve adapter that CHOSE the URL — declares the requirements; this
/// crate never infers them from URL text:
/// - `accept`: a required Accept header. GitHub's asset API serves the
///   asset's JSON metadata unless asked for `application/octet-stream`,
///   and a metadata body is a poisoned cache entry, not an error.
/// - `authenticate`: the fetch is credential-eligible. The ambient
///   bearer still rides ONLY when the URL's host is one we hold a
///   credential for ([`carries_ambient_github_token`]) — `false` is the
///   credential-free declaration for the browser/CDN asset class.
#[derive(Debug, Clone, Copy)]
pub struct GetOptions<'a> {
    pub accept: Option<&'a str>,
    pub authenticate: bool,
}

impl Default for GetOptions<'_> {
    /// `get`'s discipline: API-host reads authenticate when a token is
    /// ambient. Adapters pass `authenticate: false` for URLs that must
    /// stay credential-free.
    fn default() -> Self {
        GetOptions {
            accept: None,
            authenticate: true,
        }
    }
}

// ---------------------------------------------------------------------
// The OCI distribution surface (spec 38): raw-status GETs and the
// loopback carve-out
// ---------------------------------------------------------------------

/// A raw HTTP response for the OCI distribution adapter (spec 38): the
/// status is NOT classified — a 401 carries the `WWW-Authenticate`
/// challenge the caller must read, and distribution error bodies map by
/// their `code` field, not their status class.
#[derive(Debug, Clone)]
pub struct RawResponse {
    pub status: u16,
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawResponse {
    /// Construct a raw response (the distribution adapter's test seams
    /// and tebako-resolve's transport shims build these; the crate's own
    /// [`get_raw`] fills them from the wire).
    pub fn new(status: u16, headers: Vec<(String, String)>, body: Vec<u8>) -> Self {
        RawResponse {
            status,
            headers,
            body,
        }
    }

    /// A response header by case-insensitive name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// The spec 38 §8 loopback carve-out, host-half: `localhost`,
/// `127.0.0.0/8`, `[::1]` (an optional `:port` rides along). The ONLY
/// hosts a plain-HTTP distribution URL may name — the test fixture and
/// local development, never a remote registry.
pub fn is_loopback_host(host: &str) -> bool {
    let (bare, port) = match host.strip_prefix('[') {
        Some(rest) => {
            let mut it = rest.splitn(2, ']');
            let bare = it.next().unwrap_or("");
            match it.next() {
                // the bracket closes the host: only :port may follow
                None => (bare, None),
                Some("") => (bare, None),
                Some(tail) => (bare, tail.strip_prefix(':').or(Some("!"))),
            }
        }
        None => match host.split_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (host, None),
        },
    };
    // A port, when spelled, is digits — anything else is not a
    // host[:port] this carve-out recognizes.
    if port.is_some_and(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    if bare.eq_ignore_ascii_case("localhost") || bare == "::1" {
        return true;
    }
    // 127.0.0.0/8, dotted-quad only (no shorthand forms — a guess would
    // widen the carve-out).
    let parts: Vec<&str> = bare.split('.').collect();
    parts.len() == 4
        && parts[0] == "127"
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The URL-half of the carve-out: a plain-`http://` URL naming a loopback
/// host ([`is_loopback_host`]).
pub fn is_loopback_http_url(url: &str) -> bool {
    match url.strip_prefix("http://") {
        Some(rest) => is_loopback_host(rest.split('/').next().unwrap_or("")),
        None => false,
    }
}

/// The spec 38 §8 transport rule: HTTPS everywhere, plain HTTP to
/// loopback only (no insecure-registry spelling exists).
fn require_https_or_loopback(url: &str) -> Result<(), FetchError> {
    if url.starts_with("https://") || is_loopback_http_url(url) {
        return Ok(());
    }
    Err(FetchError::DownloadFailed(format!(
        "refusing non-HTTPS URL {url} (https://, loopback http://, and file:// are supported)"
    )))
}

/// The distribution-API agent: identical discipline to the one agent
/// (TLS roots, proxy, timeouts, redirects) but without ureq's
/// `https_only` latch — the entry points below enforce the
/// https-or-loopback policy themselves (the latch cannot express the
/// carve-out).
fn raw_agent() -> Result<&'static ureq::Agent, FetchError> {
    static RAW_AGENT: OnceLock<Result<ureq::Agent, FetchError>> = OnceLock::new();
    RAW_AGENT
        .get_or_init(|| build_agent_with(GLOBAL_TIMEOUT, false))
        .as_ref()
        .map_err(Clone::clone)
}

/// GET `url` (https, or loopback http — spec 38 §8) WITHOUT status
/// classification: any 2xx/4xx/5xx answer returns its status, headers,
/// and body so the distribution adapter can read the 401
/// `WWW-Authenticate` challenge and the error body's `code`. Transport
/// failures (connect, TLS, body read) remain [`FetchError`]s.
pub fn get_raw(
    url: &str,
    accept: Option<&str>,
    header: Option<(&str, &str)>,
) -> Result<RawResponse, FetchError> {
    require_https_or_loopback(url)?;
    network_guard()?;
    let response = apply_explicit(url, raw_agent()?.get(url), accept, header)
        .call()
        .map_err(map_ureq_error(url))?;
    into_raw_response(url, response)
}

/// The publish path's write agent (spec 38 §7): the distribution
/// surface's policy (https-or-loopback enforced by the entry points,
/// never the latch) with the upload channel's timeout — monolithic blob
/// PUTs are ≲150 MB.
fn raw_upload_agent() -> Result<&'static ureq::Agent, FetchError> {
    static RAW_UPLOAD_AGENT: OnceLock<Result<ureq::Agent, FetchError>> = OnceLock::new();
    RAW_UPLOAD_AGENT
        .get_or_init(|| build_agent_with(UPLOAD_TIMEOUT, false))
        .as_ref()
        .map_err(Clone::clone)
}

/// Read a raw-agent answer into the unclassified [`RawResponse`] shape
/// (the write verbs and [`get_raw`] share it).
fn into_raw_response(
    url: &str,
    response: ureq::http::Response<ureq::Body>,
) -> Result<RawResponse, FetchError> {
    let status = response.status().as_u16();
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();
    let mut response = response;
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .read_to_vec()
        .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))?;
    Ok(RawResponse {
        status,
        headers,
        body,
    })
}

/// POST `body` to `url` (https, or loopback http — spec 38 §8) WITHOUT
/// status classification: the publish path's blob-mount/upload-session
/// open (spec 38 §7), where 201 (mounted), 202 (session opened, the
/// `Location` header its handle), and 401 (the dance cue) are all
/// answers the caller reads itself. The caller-decided credential
/// header attaches verbatim; ureq never forwards it on redirect
/// (`RedirectAuthHeaders::Never`).
pub fn post_raw(
    url: &str,
    body: &[u8],
    content_type: Option<&str>,
    header: Option<(&str, &str)>,
) -> Result<RawResponse, FetchError> {
    require_https_or_loopback(url)?;
    network_guard()?;
    let mut req = raw_upload_agent()?.post(url);
    if let Some(content_type) = content_type {
        req = req.header("Content-Type", content_type);
    }
    if let Some((name, value)) = header {
        req = req.header(name, value);
    }
    let response = req.send(body).map_err(map_ureq_error(url))?;
    into_raw_response(url, response)
}

/// PUT `body` to `url` (https, or loopback http — spec 38 §8) WITHOUT
/// status classification: the publish path's monolithic blob upload and
/// manifest placement (spec 38 §7). The same discipline as
/// [`post_raw`].
pub fn put_raw(
    url: &str,
    body: &[u8],
    content_type: &str,
    header: Option<(&str, &str)>,
) -> Result<RawResponse, FetchError> {
    require_https_or_loopback(url)?;
    network_guard()?;
    let mut req = raw_upload_agent()?
        .put(url)
        .header("Content-Type", content_type);
    if let Some((name, value)) = header {
        req = req.header(name, value);
    }
    let response = req.send(body).map_err(map_ureq_error(url))?;
    into_raw_response(url, response)
}

/// The distribution blob stream (spec 38 §5): the ordinary classified
/// streaming GET (a 401 is [`FetchError::AuthRejected`] — the adapter's
/// re-challenge cue; a 404 the missing-blob answer) with the
/// https-or-loopback policy of [`get_raw`] in place of the https-only
/// rule. The caller-decided credential header attaches verbatim; ureq
/// never forwards it on redirect (`RedirectAuthHeaders::Never`).
pub fn stream_raw(
    url: &str,
    accept: Option<&str>,
    header: Option<(&str, &str)>,
    writer: &mut dyn std::io::Write,
    on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
) -> Result<u64, FetchError> {
    require_https_or_loopback(url)?;
    network_guard()?;
    let response = apply_explicit(url, raw_agent()?.get(url), accept, header)
        .call()
        .map_err(map_ureq_error(url))?;
    let response = classify(response, url)?;
    stream_response(url, response, writer, on_progress)
}

// ---------------------------------------------------------------------
// The range-fetch surface (spec 39 §2's transport half): positioned
// Range GETs for the lazy mount-source byte layer
// ---------------------------------------------------------------------

/// One attempt budget for a single range GET (the gem's download
/// discipline — tebako-resolve's DOWNLOAD_ATTEMPTS mirrors this value).
pub const RANGE_ATTEMPTS: u32 = 3;
/// Delay between range attempts (the gem's retry delay).
pub const RANGE_RETRY_DELAY: Duration = Duration::from_secs(1);

/// One half-open byte range `[offset, offset + len)` of a remote object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub offset: u64,
    pub len: u64,
}

impl ByteRange {
    /// The block-group indexing of spec 39 §3: group `index` of
    /// `group_size`-byte groups over an object of `size_bytes` — the
    /// final group is SHORT when `size_bytes` is not a multiple.
    /// `index` beyond the last group is a zero-length range (the
    /// caller's size accounting bug — [`get_range`] refuses it by name).
    pub fn group(index: u64, group_size: u64, size_bytes: u64) -> ByteRange {
        let offset = index.saturating_mul(group_size);
        let len = group_size.min(size_bytes.saturating_sub(offset));
        ByteRange { offset, len }
    }

    /// The group count of `size_bytes` under `group_size` (0 for an
    /// empty object).
    pub fn group_count(size_bytes: u64, group_size: u64) -> u64 {
        if group_size == 0 {
            return 0;
        }
        size_bytes / group_size + u64::from(size_bytes % group_size != 0)
    }

    /// The wire spelling (`bytes=<offset>-<end>`, the inclusive-end
    /// form). A zero-length range has no spelling — the caller refuses
    /// it before the wire.
    fn wire(&self) -> String {
        format!("bytes={}-{}", self.offset, self.offset + self.len - 1)
    }
}

/// The parsed `Content-Range: bytes <start>-<end>/<total>` of a 206
/// (inclusive end, as the wire spells it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub start: u64,
    pub end: u64,
    pub total: u64,
}

/// Parse a `Content-Range` value: exactly the `bytes <start>-<end>/<total>`
/// form (a 206's answer — the `bytes */<total>` form belongs to a 416,
/// which never reaches here). `start > end` or `end >= total` is no
/// Content-Range at all, never a clamp.
fn parse_content_range(value: &str) -> Option<ContentRange> {
    let rest = value.trim().strip_prefix("bytes")?.trim_start();
    let (span, total) = rest.split_once('/')?;
    let (start, end) = span.split_once('-')?;
    let start: u64 = start.trim().parse().ok()?;
    let end: u64 = end.trim().parse().ok()?;
    let total: u64 = total.trim().parse().ok()?;
    if start > end || end >= total {
        return None;
    }
    Some(ContentRange { start, end, total })
}

/// The bytes one range GET returned, with the object's total size and
/// the validator (`ETag`) the response carried — the caller's later
/// `If-Range` input.
#[derive(Debug, Clone)]
pub struct RangeBody {
    pub bytes: Vec<u8>,
    pub total: u64,
    pub etag: Option<String>,
}

/// The range GET's answer: the requested window, or the whole
/// representation (the eager-fallback signal).
#[derive(Debug, Clone)]
pub enum RangeAnswer {
    /// 206 with a Content-Range validated against the request (exact
    /// start, exact length, total).
    Partial(RangeBody),
    /// 200 — the server ignored the Range header (no range support) or
    /// answered an `If-Range` validator mismatch with the whole
    /// representation: spec 39 §3's loud-eager-fallback signal. Never an
    /// error, never silently treated as the window; the caller owns the
    /// fallback law.
    Full(RangeBody),
}

/// Read one response body fully (the range window — group-scale, far
/// under [`MAX_BODY_SIZE`]), classifying a mid-stream failure as
/// retryable by the caller.
fn read_range_body(
    url: &str,
    response: &mut ureq::http::Response<ureq::Body>,
) -> Result<Vec<u8>, FetchError> {
    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .read_to_vec()
        .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))
}

/// One range attempt. The retry loop owns the schedule; everything here
/// is ONE shot — a mid-stream failure abandons the attempt so the loop
/// restarts THE RANGE from zero (a partial window would fail its digest
/// anyway — spec 39 §6, the pipeline's artifact rule at group scale).
fn range_attempt(
    url: &str,
    range: ByteRange,
    if_range: Option<&str>,
    header: Option<(&str, &str)>,
) -> Result<RangeAnswer, FetchError> {
    let mut req = raw_agent()?.get(url).header("Range", range.wire());
    if let Some(validator) = if_range {
        req = req.header("If-Range", validator);
    }
    let response = apply_explicit(url, req, None, header)
        .call()
        .map_err(map_ureq_error(url))?;
    let status = response.status().as_u16();
    let etag = |r: &ureq::http::Response<ureq::Body>| {
        r.headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    match status {
        200 => {
            let etag = etag(&response);
            let mut response = response;
            let bytes = read_range_body(url, &mut response)?;
            Ok(RangeAnswer::Full(RangeBody {
                total: bytes.len() as u64,
                bytes,
                etag,
            }))
        }
        206 => {
            let served = response
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(parse_content_range);
            let mut response = response;
            let Some(served) = served else {
                drain_body(&mut response);
                return Err(FetchError::DownloadFailed(format!(
                    "206 from {url} carries no valid Content-Range"
                )));
            };
            if served.start != range.offset || served.end - served.start + 1 != range.len {
                drain_body(&mut response);
                return Err(FetchError::DownloadFailed(format!(
                    "206 from {url} serves bytes {}-{} but the request was {range:?}",
                    served.start, served.end
                )));
            }
            let etag = etag(&response);
            let bytes = read_range_body(url, &mut response)?;
            // A cleanly-delimited body that ends short of the window is
            // as failed as a dropped connection: retry the range from
            // zero, never serve a short group.
            if bytes.len() as u64 != range.len {
                return Err(FetchError::DownloadFailed(format!(
                    "truncated range body from {url}: {} of {} bytes",
                    bytes.len(),
                    range.len
                )));
            }
            Ok(RangeAnswer::Partial(RangeBody {
                bytes,
                total: served.total,
                etag,
            }))
        }
        _ => {
            let mut response = response;
            drain_body(&mut response);
            match classify(response, url) {
                Err(e) => Err(e),
                Ok(_) => Err(FetchError::DownloadFailed(format!(
                    "{status} fetching {url} — a range GET answers 206, 200, or an error"
                ))),
            }
        }
    }
}

/// Read and discard the body of a response the caller is about to
/// reject (bounded): a fully-consumed body returns the connection to
/// the agent's pool, while a dropped one forces a close — with unread
/// bytes still in flight, a RST, and the next attempt's pooled
/// connection then races its own teardown. Error bodies are small;
/// one that outgrows the cap simply closes the connection instead.
fn drain_body(response: &mut ureq::http::Response<ureq::Body>) {
    const DRAIN_CAP: u64 = 4 * 1024 * 1024;
    let _ = response
        .body_mut()
        .with_config()
        .limit(DRAIN_CAP)
        .read_to_vec();
}

/// GET one byte range of `url` (spec 39 — the lazy mount-source's
/// transport): `Range: bytes=<offset>-<end>` is emitted; an `If-Range`
/// validator (the ETag the caller holds from an earlier answer) rides
/// along when supplied; the caller-decided credential header attaches
/// verbatim — confinement was decided upstream, exactly like whole-file
/// GETs (spec 39 §6). `https://`, loopback `http://` (the §10 fixture's
/// carve-out), and `file://` (the test/airgap spelling — the range is
/// sliced from disk, pread-clipped at EOF).
///
/// The answer distinguishes 206 ([`RangeAnswer::Partial`], Content-Range
/// validated) from 200 ([`RangeAnswer::Full`] — the no-range-support /
/// validator-mismatch eager-fallback signal). Every other status
/// classifies exactly like [`get`].
///
/// Retry law (tebako-http's own, unreinvented): a throttled answer waits
/// [`throttle_backoff`] (Retry-After honored exactly) for up to
/// [`THROTTLE_ROUNDS`]; a transport failure — a dropped or truncated
/// body included — retries THE RANGE FROM ZERO up to [`RANGE_ATTEMPTS`]
/// times with [`RANGE_RETRY_DELAY`] between. A zero-length range is a
/// named refusal, never a wire request.
pub fn get_range(
    url: &str,
    range: ByteRange,
    if_range: Option<&str>,
    header: Option<(&str, &str)>,
) -> Result<RangeAnswer, FetchError> {
    if range.len == 0 {
        return Err(FetchError::DownloadFailed(format!(
            "a zero-length range (offset {}) is not a fetch — the caller's group indexing is off",
            range.offset
        )));
    }
    if let Some(path) = url.strip_prefix("file://") {
        let bytes = read_file_url(file_path_from_url(path))?;
        let total = bytes.len() as u64;
        let start = (range.offset as usize).min(bytes.len());
        let end = (start + (range.len as usize).min(bytes.len() - start)).min(bytes.len());
        return Ok(RangeAnswer::Partial(RangeBody {
            bytes: bytes[start..end].to_vec(),
            total,
            etag: None,
        }));
    }
    require_https_or_loopback(url)?;
    network_guard()?;
    let mut attempts = 0;
    let mut throttles = 0;
    loop {
        match range_attempt(url, range, if_range, header) {
            Ok(answer) => return Ok(answer),
            Err(FetchError::Throttled {
                retry_after,
                status,
                ..
            }) => {
                throttles += 1;
                if throttles >= THROTTLE_ROUNDS {
                    return Err(FetchError::DownloadFailed(format!(
                        "still throttled after {THROTTLE_ROUNDS} backoff rounds fetching {url} ({status})"
                    )));
                }
                std::thread::sleep(throttle_backoff(throttles, retry_after));
            }
            Err(FetchError::DownloadFailed(msg)) => {
                attempts += 1;
                if attempts >= RANGE_ATTEMPTS {
                    return Err(FetchError::DownloadFailed(format!(
                        "failed to fetch the range {range:?} of {url} after {RANGE_ATTEMPTS} attempts: {msg}"
                    )));
                }
                std::thread::sleep(RANGE_RETRY_DELAY);
            }
            // Everything else is terminal by the crate's one law:
            // IndexUnavailable (try the next name), AuthRejected (the
            // credential layer's remap point), Cancelled (the plan is
            // already unwinding), the config answers (never retried).
            Err(e) => return Err(e),
        }
    }
}

/// The one request-shaping path: the declared Accept, plus the ambient
/// bearer iff the caller allows credentials AND the host is the GitHub
/// API. Every GET entry point builds through here so no path can drift.
fn apply_options<'a>(
    url: &str,
    mut req: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    opts: &GetOptions<'a>,
) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
    if let Some(accept) = opts.accept {
        req = req.header("Accept", accept);
    }
    if opts.authenticate && carries_ambient_github_token(url) {
        if let Some(token) = github_token_from_env() {
            req = req.header("Authorization", &format!("Bearer {token}"));
        }
    }
    req
}

/// GET `url` and return the response body. `https://` (redirects
/// followed, HTTPS-only) or `file://`. A `TEBAKO_GITHUB_TOKEN` /
/// `GITHUB_TOKEN` env authenticates `api.github.com` reads (see
/// [`github_token_from_env`]); every other host rides anonymous.
pub fn get(url: &str) -> Result<Vec<u8>, FetchError> {
    get_with_options(url, &GetOptions::default())
}

/// [`get`] honoring explicit fetch requirements ([`GetOptions`]) — the
/// resolve adapter's asset descriptors ride this entry point.
pub fn get_with_options(url: &str, opts: &GetOptions) -> Result<Vec<u8>, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return read_file_url(file_path_from_url(path));
    }
    require_https(url)?;
    network_guard()?;
    // http_status_as_error(false): statuses are mapped HERE, because the
    // throttle schedule lives in the response headers (ureq's StatusCode
    // error drops them).
    let response = apply_options(url, agent()?.get(url), opts)
        .call()
        .map_err(map_ureq_error(url))?;
    let mut response = classify(response, url)?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .read_to_vec()
        .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))
}

/// [`get`] with an EXPLICIT credential header decided by the caller
/// (spec 37 §5's credential layer): the header attaches verbatim and
/// wins over the ambient bearer — confinement was already decided by
/// the caller, and ureq never forwards auth headers on redirect
/// (`RedirectAuthHeaders::Never` is the default), so a redirect target
/// never sees it either. `None` rides anonymous.
pub fn get_with_explicit(
    url: &str,
    accept: Option<&str>,
    header: Option<(&str, &str)>,
) -> Result<Vec<u8>, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return read_file_url(file_path_from_url(path));
    }
    require_https(url)?;
    network_guard()?;
    let response = apply_explicit(url, agent()?.get(url), accept, header)
        .call()
        .map_err(map_ureq_error(url))?;
    let mut response = classify(response, url)?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .read_to_vec()
        .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))
}

/// The explicit-header request shaping ([`get_with_explicit`],
/// [`stream_to_writer_explicit`]): the declared Accept, then the
/// caller-decided credential header verbatim. The ambient bearer is
/// NEVER consulted here — the caller's decision already accounted for
/// it (spec 37 §5's two-tier lookup ranks the ambient token).
fn apply_explicit<'a>(
    _url: &str,
    mut req: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    accept: Option<&'a str>,
    header: Option<(&'a str, &'a str)>,
) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
    if let Some(accept) = accept {
        req = req.header("Accept", accept);
    }
    if let Some((name, value)) = header {
        req = req.header(name, value);
    }
    req
}

/// [`get`] with an explicit bearer token (the publish/verify channel —
/// the caller holds the credential; the ambient-token rule does not
/// apply). The releases-API reads authenticate: GitHub's unauthenticated
/// tag-lookup lags (or 404s outright) on fresh public releases, and the
/// anonymous rate limit is tight on CI egress IPs.
pub fn get_bearer(url: &str, bearer: Option<&str>) -> Result<Vec<u8>, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return read_file_url(file_path_from_url(path));
    }
    require_https(url)?;
    network_guard()?;
    let mut req = agent()?.get(url);
    if let Some(token) = bearer {
        req = req.header("Authorization", &format!("Bearer {token}"));
    }
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
    get_with_progress_and_options(url, on_progress, &GetOptions::default())
}

/// [`get_with_progress`] honoring explicit fetch requirements
/// ([`GetOptions`]). The progress path shapes the request through the
/// same [`apply_options`] as every other GET — a download must not ride
/// anonymous (or headerless) just because a progress hook is attached.
pub fn get_with_progress_and_options(
    url: &str,
    on_progress: Option<&mut dyn FnMut(u64, Option<u64>)>,
    opts: &GetOptions,
) -> Result<Vec<u8>, FetchError> {
    let Some(cb) = on_progress else {
        return get_with_options(url, opts);
    };
    if let Some(path) = url.strip_prefix("file://") {
        let bytes = read_file_url(file_path_from_url(path))?;
        cb(bytes.len() as u64, Some(bytes.len() as u64));
        return Ok(bytes);
    }
    require_https(url)?;
    use std::io::Read as _;
    network_guard()?;
    let response = apply_options(url, agent()?.get(url), opts)
        .call()
        .map_err(map_ureq_error(url))?;
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

/// The STREAMING GET (spec 05 §6): the response body flows chunk by
/// chunk into `writer` — the fetch pipeline's hashing writer turns this
/// into one pass with constant memory; artifact bytes never materialize
/// whole in memory. `on_progress` fires per chunk with
/// `(bytes_so_far, content_length)`; returning `false` ABORTS the
/// download (the plan-cancel path) with a [`FetchError::Cancelled`].
/// Returns the total bytes written.
pub fn stream_to_writer(
    url: &str,
    opts: &GetOptions,
    writer: &mut dyn std::io::Write,
    on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
) -> Result<u64, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return stream_file_url(path, url, writer, on_progress);
    }
    require_https(url)?;
    network_guard()?;
    let response = apply_options(url, agent()?.get(url), opts)
        .call()
        .map_err(map_ureq_error(url))?;
    let response = classify(response, url)?;
    stream_response(url, response, writer, on_progress)
}

/// [`stream_to_writer`] with an EXPLICIT credential header decided by
/// the caller (spec 37 §5's credential layer — the same rule as
/// [`get_with_explicit`]): the header attaches verbatim and wins over
/// the ambient bearer; `None` rides anonymous.
pub fn stream_to_writer_explicit(
    url: &str,
    accept: Option<&str>,
    header: Option<(&str, &str)>,
    writer: &mut dyn std::io::Write,
    on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
) -> Result<u64, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return stream_file_url(path, url, writer, on_progress);
    }
    require_https(url)?;
    network_guard()?;
    let response = apply_explicit(url, agent()?.get(url), accept, header)
        .call()
        .map_err(map_ureq_error(url))?;
    let response = classify(response, url)?;
    stream_response(url, response, writer, on_progress)
}

/// The `file://` half of the streaming GETs (a local read streams like
/// a remote body; credential headers are meaningless on it).
fn stream_file_url(
    path: &str,
    url: &str,
    writer: &mut dyn std::io::Write,
    mut on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
) -> Result<u64, FetchError> {
    let mut file = std::fs::File::open(file_path_from_url(path)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            FetchError::IndexUnavailable(path.to_string())
        } else {
            FetchError::DownloadFailed(format!("{e} reading {path}"))
        }
    })?;
    let total = file.metadata().ok().map(|m| m.len());
    let mut written = 0u64;
    let mut chunk = [0u8; 65536];
    use std::io::Read as _;
    loop {
        let n = file
            .read(&mut chunk)
            .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {path}")))?;
        if n == 0 {
            break;
        }
        writer
            .write_all(&chunk[..n])
            .map_err(|e| FetchError::DownloadFailed(format!("{e} writing {url}")))?;
        written += n as u64;
        if let Some(cb) = on_progress.as_deref_mut() {
            if !cb(written, total) {
                return Err(FetchError::Cancelled(url.to_string()));
            }
        }
    }
    Ok(written)
}

/// The read loop of the streaming GETs: the classified response body
/// flows chunk by chunk into `writer`, ticking progress and aborting
/// with [`FetchError::Cancelled`] when the callback answers `false`.
fn stream_response(
    url: &str,
    mut response: ureq::http::Response<ureq::Body>,
    writer: &mut dyn std::io::Write,
    mut on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
) -> Result<u64, FetchError> {
    use std::io::Read as _;
    let content_length = response.body().content_length();
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_SIZE)
        .reader();
    let mut written = 0u64;
    let mut chunk = [0u8; 65536];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| FetchError::DownloadFailed(format!("{e} reading {url}")))?;
        if n == 0 {
            break;
        }
        writer
            .write_all(&chunk[..n])
            .map_err(|e| FetchError::DownloadFailed(format!("{e} writing {url}")))?;
        written += n as u64;
        if let Some(cb) = on_progress.as_deref_mut() {
            if !cb(written, content_length) {
                return Err(FetchError::Cancelled(url.to_string()));
            }
        }
    }
    Ok(written)
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
    network_guard()?;
    let mut req = upload_agent()?
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
    network_guard()?;
    let mut req = agent()?
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
    fn the_loopback_carve_out_names_exactly_the_spec_38_hosts() {
        for good in [
            "localhost",
            "localhost:5000",
            "LOCALHOST",
            "127.0.0.1",
            "127.0.0.1:5000",
            "127.42.13.7",
            "[::1]",
            "[::1]:5000",
        ] {
            assert!(is_loopback_host(good), "{good}");
        }
        for bad in [
            "example.com",
            "localhost.evil.example",
            "128.0.0.1",
            "127.0.0.1.evil.example",
            "::1",           // IPv6 rides the bracketed form
            "127.0.0.1:abc", // a port is digits
            "[::1]x",        // the bracket closes the host
            "",
        ] {
            assert!(!is_loopback_host(bad), "{bad}");
        }
        assert!(is_loopback_http_url("http://127.0.0.1:5000/v2/x"));
        assert!(is_loopback_http_url("http://localhost/fixtures/tool"));
        assert!(!is_loopback_http_url("https://127.0.0.1:5000/v2/x")); // http-only helper
        assert!(!is_loopback_http_url("http://registry.corp.internal/v2/x"));
        assert!(!is_loopback_http_url("file:///tmp/x"));
    }

    #[test]
    fn the_distribution_surface_refuses_non_https_off_loopback() {
        assert!(matches!(
            get_raw("http://registry.corp.internal/v2/", None, None),
            Err(FetchError::DownloadFailed(_))
        ));
        let mut sink: Vec<u8> = Vec::new();
        assert!(matches!(
            stream_raw(
                "http://registry.corp.internal/v2/x/blobs/sha256:ab",
                None,
                None,
                &mut sink,
                None
            ),
            Err(FetchError::DownloadFailed(_))
        ));
        // the policy check fires before any network: a loopback URL passes
        // require_https_or_loopback and fails (if at all) at connect.
        assert!(require_https_or_loopback("http://127.0.0.1:9/v2/").is_ok());
        assert!(require_https_or_loopback("https://ghcr.io/v2/").is_ok());
    }

    #[test]
    fn raw_response_headers_lookup_case_insensitively() {
        let r = RawResponse {
            status: 401,
            headers: vec![(
                "WWW-Authenticate".to_string(),
                "Bearer realm=\"https://h/token\"".to_string(),
            )],
            body: Vec::new(),
        };
        assert_eq!(r.status, 401);
        assert_eq!(
            r.header("www-authenticate"),
            Some("Bearer realm=\"https://h/token\"")
        );
        assert_eq!(r.header("content-type"), None);
    }

    #[test]
    fn plain_http_is_refused() {
        assert!(matches!(
            get("http://example.com/"),
            Err(FetchError::DownloadFailed(_))
        ));
    }

    #[test]
    fn ambient_token_is_carried_only_to_the_github_api_host() {
        assert!(carries_ambient_github_token(
            "https://api.github.com/repos/o/r/releases/tags/v1"
        ));
        // asset downloads and every other host stay anonymous (a bearer
        // sent off api.github.com is a leaked credential)
        assert!(!carries_ambient_github_token(
            "https://github.com/o/r/releases/download/v1/a.tfs"
        ));
        assert!(!carries_ambient_github_token(
            "https://objects.githubusercontent.com/o/r/a.tfs"
        ));
        assert!(!carries_ambient_github_token(
            "https://api.github.com.evil.example/phish"
        ));
        assert!(!carries_ambient_github_token("file:///tmp/x"));
    }

    #[test]
    fn ambient_token_env_precedence_and_request_shaping() {
        let saved: Vec<(&str, Option<String>)> = ["TEBAKO_GITHUB_TOKEN", "GITHUB_TOKEN"]
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();
        std::env::remove_var("TEBAKO_GITHUB_TOKEN");
        std::env::remove_var("GITHUB_TOKEN");
        assert_eq!(github_token_from_env(), None);

        std::env::set_var("GITHUB_TOKEN", "ci-token");
        assert_eq!(github_token_from_env().as_deref(), Some("ci-token"));

        std::env::set_var("TEBAKO_GITHUB_TOKEN", "tebako-token");
        assert_eq!(github_token_from_env().as_deref(), Some("tebako-token"));

        std::env::set_var("TEBAKO_GITHUB_TOKEN", "");
        assert_eq!(github_token_from_env().as_deref(), Some("ci-token"));

        // apply_options shapes the request: the declared Accept always
        // lands; the ambient bearer lands iff `authenticate` AND the
        // api.github.com host. (No request is sent — headers are read
        // off the builder.)
        let builder = || -> ureq::Agent { ureq::Agent::config_builder().build().into() };
        let asset_api = "https://api.github.com/repos/o/r/releases/assets/1";
        let req = apply_options(
            asset_api,
            builder().get(asset_api),
            &GetOptions {
                accept: Some("application/octet-stream"),
                authenticate: true,
            },
        );
        let headers = req.headers_ref().unwrap();
        assert_eq!(headers["Accept"], "application/octet-stream");
        assert_eq!(headers["Authorization"], "Bearer ci-token");

        // the credential-free declaration: no bearer even on the API host
        let req = apply_options(
            asset_api,
            builder().get(asset_api),
            &GetOptions {
                accept: None,
                authenticate: false,
            },
        );
        assert!(!req.headers_ref().unwrap().contains_key("Authorization"));

        // and credential-eligibility never extends past the API host
        let cdn = "https://github.com/o/r/releases/download/v1/a.tfs";
        let req = apply_options(cdn, builder().get(cdn), &GetOptions::default());
        assert!(!req.headers_ref().unwrap().contains_key("Authorization"));

        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
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

    #[test]
    fn stream_to_writer_flows_chunked_with_progress_and_totals() {
        let dir =
            std::env::temp_dir().join(format!("tebako-http-test-stream-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // > one 64 KiB chunk, so the chunked path exercises twice
        let payload: Vec<u8> = (0..150_000u32).map(|i| (i % 251) as u8).collect();
        let file = dir.join("asset.bin");
        std::fs::write(&file, &payload).unwrap();
        let url = file_url(&file);

        let mut out: Vec<u8> = Vec::new();
        let mut calls: Vec<(u64, Option<u64>)> = Vec::new();
        let written = stream_to_writer(
            &url,
            &GetOptions::default(),
            &mut out,
            Some(&mut |so_far, total| {
                calls.push((so_far, total));
                true
            }),
        )
        .unwrap();
        assert_eq!(written, payload.len() as u64);
        assert_eq!(out, payload);
        assert!(calls.len() >= 2, "{calls:?}");
        assert_eq!(calls.last().copied(), Some((150_000, Some(150_000))));

        // no hook: same bytes
        let mut out2: Vec<u8> = Vec::new();
        let written = stream_to_writer(&url, &GetOptions::default(), &mut out2, None).unwrap();
        assert_eq!(written, payload.len() as u64);
        assert_eq!(out2, payload);

        // a missing file is the missing-object error
        let missing = file_url(&dir.join("nope.bin"));
        let mut sink: Vec<u8> = Vec::new();
        assert!(matches!(
            stream_to_writer(&missing, &GetOptions::default(), &mut sink, None),
            Err(FetchError::IndexUnavailable(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stream_to_writer_abort_is_the_named_cancel() {
        let dir =
            std::env::temp_dir().join(format!("tebako-http-test-cancel-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let payload: Vec<u8> = (0..150_000u32).map(|i| (i % 251) as u8).collect();
        let file = dir.join("asset.bin");
        std::fs::write(&file, &payload).unwrap();
        let url = file_url(&file);

        let mut out: Vec<u8> = Vec::new();
        let err = stream_to_writer(
            &url,
            &GetOptions::default(),
            &mut out,
            Some(&mut |so_far, _| so_far < 65536),
        )
        .unwrap_err();
        assert!(matches!(err, FetchError::Cancelled(_)), "{err:?}");
        // the writer holds exactly the bytes up to the abort
        assert_eq!(out.len(), 65536);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ---------------------------------------------------------------------
// The range-fetch surface's tests (spec 39 §10): pure range/Content-Range
// math, then the contract tier against a hand-rolled std::net responder
// (no test-only dependencies — the spec 38 §8 loopback carve-out is what
// lets the fixture ride plain HTTP).
// ---------------------------------------------------------------------

#[cfg(test)]
mod range_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    fn patterned(n: usize) -> Vec<u8> {
        (0..n as u32).map(|i| (i % 251) as u8).collect()
    }

    // ------------------------- unit legs (pure) -------------------------

    #[test]
    fn group_math_covers_exact_short_and_empty_objects() {
        assert_eq!(
            ByteRange::group(0, 256, 1024),
            ByteRange {
                offset: 0,
                len: 256
            }
        );
        assert_eq!(
            ByteRange::group(3, 256, 1024),
            ByteRange {
                offset: 768,
                len: 256
            }
        );
        // 1000 = 3 × 256 + 232 — the final group is short
        assert_eq!(
            ByteRange::group(3, 256, 1000),
            ByteRange {
                offset: 768,
                len: 232
            }
        );
        // beyond the last group: the zero-length range get_range refuses
        assert_eq!(
            ByteRange::group(4, 256, 1000),
            ByteRange {
                offset: 1024,
                len: 0
            }
        );
        assert_eq!(ByteRange::group_count(1024, 256), 4);
        assert_eq!(ByteRange::group_count(1000, 256), 4);
        assert_eq!(ByteRange::group_count(0, 256), 0);
        assert_eq!(ByteRange::group_count(7, 0), 0);
    }

    #[test]
    fn the_wire_spelling_is_the_inclusive_end_form() {
        assert_eq!(ByteRange { offset: 8, len: 8 }.wire(), "bytes=8-15");
        assert_eq!(ByteRange { offset: 0, len: 1 }.wire(), "bytes=0-0");
    }

    #[test]
    fn content_range_parses_only_the_206_form() {
        assert_eq!(
            parse_content_range("bytes 0-99/200"),
            Some(ContentRange {
                start: 0,
                end: 99,
                total: 200
            })
        );
        assert_eq!(
            parse_content_range("bytes 8-15/1000"),
            Some(ContentRange {
                start: 8,
                end: 15,
                total: 1000
            })
        );
        for bad in [
            "bytes */1234",  // the 416 form, never a 206's
            "0-1/2",         // no unit
            "items 0-1/2",   // the wrong unit
            "bytes 5-3/10",  // start past end
            "bytes 0-10/10", // end past the object
            "bytes 0-1",     // no total
            "bytes a-b/c",
            "",
        ] {
            assert_eq!(parse_content_range(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_zero_length_range_is_refused_before_any_io() {
        let err = get_range(
            "https://example.invalid/image.tfs",
            ByteRange { offset: 0, len: 0 },
            None,
            None,
        )
        .unwrap_err();
        let FetchError::DownloadFailed(msg) = err else {
            panic!("expected DownloadFailed: {err:?}")
        };
        assert!(msg.contains("zero-length"), "{msg}");
    }

    #[test]
    fn a_plain_http_remote_is_refused_before_the_network() {
        let err = get_range(
            "http://registry.corp.internal/image.tfs",
            ByteRange { offset: 0, len: 4 },
            None,
            None,
        )
        .unwrap_err();
        let FetchError::DownloadFailed(msg) = err else {
            panic!("expected DownloadFailed: {err:?}")
        };
        assert!(msg.contains("non-HTTPS"), "{msg}");
    }

    #[test]
    fn file_urls_slice_from_disk_clipped_at_eof() {
        let dir =
            std::env::temp_dir().join(format!("tebako-http-test-range-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let payload = patterned(1000);
        let file = dir.join("image.tfs");
        std::fs::write(&file, &payload).unwrap();
        let url = file_url(&file);

        // a middle window
        let answer = get_range(&url, ByteRange { offset: 8, len: 8 }, None, None).unwrap();
        let RangeAnswer::Partial(body) = answer else {
            panic!("expected Partial")
        };
        assert_eq!(body.bytes.as_slice(), &payload[8..16]);
        assert_eq!(body.total, 1000);
        assert_eq!(body.etag, None);

        // the short final group
        let answer = get_range(&url, ByteRange::group(3, 256, 1000), None, None).unwrap();
        let RangeAnswer::Partial(body) = answer else {
            panic!("expected Partial")
        };
        assert_eq!(body.bytes.as_slice(), &payload[768..]);

        // a range spilling past EOF clips at the object's end
        let answer = get_range(
            &url,
            ByteRange {
                offset: 900,
                len: 500,
            },
            None,
            None,
        )
        .unwrap();
        let RangeAnswer::Partial(body) = answer else {
            panic!("expected Partial")
        };
        assert_eq!(body.bytes.as_slice(), &payload[900..]);
        assert_eq!(body.total, 1000);

        // a missing file is the missing-object error
        let missing = file_url(&dir.join("nope.tfs"));
        let err = get_range(&missing, ByteRange { offset: 0, len: 8 }, None, None).unwrap_err();
        assert!(matches!(err, FetchError::IndexUnavailable(_)), "{err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ------------------- the contract fixture (§10) -------------------

    /// What the fixture heard on one connection.
    #[derive(Debug, Clone)]
    struct Heard {
        path: String,
        range: Option<String>,
        if_range: Option<String>,
    }

    /// One programmed answer. `content-length` always spells the FULL
    /// body length; `truncate_to` delivers only a prefix before the
    /// close — the dropped-connection injection (the header's lie is
    /// what forces the client's mid-stream failure).
    #[derive(Clone)]
    struct Reply {
        status: u16,
        reason: &'static str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        truncate_to: Option<usize>,
    }

    impl Reply {
        fn whole(status: u16, reason: &'static str, body: Vec<u8>) -> Reply {
            Reply {
                status,
                reason,
                headers: Vec::new(),
                body,
                truncate_to: None,
            }
        }

        fn header(mut self, name: &str, value: String) -> Reply {
            self.headers.push((name.to_string(), value));
            self
        }
    }

    type Handler = Arc<dyn Fn(usize, &Heard) -> Reply + Send + Sync>;

    /// The spec 39 §10 contract server: an accept loop on a loopback
    /// port, one thread per connection, each connection speaking proper
    /// HTTP/1.1 keep-alive (the client's pooled agent then never meets a
    /// silently-closed socket — only the truncation injection closes,
    /// which is exactly the mid-stream failure it means to be). Requests
    /// are logged for the assertions (Range / If-Range spellings, hit
    /// counts — the retry law's evidence).
    struct RangeServer {
        url: String,
        heard: Arc<Mutex<Vec<Heard>>>,
        stop: Arc<AtomicBool>,
        join: Option<std::thread::JoinHandle<()>>,
        listener: Option<TcpListener>,
    }

    /// Bound fixture listeners are NEVER closed mid-process: a closed
    /// listener's port can be handed to the next fixture while the
    /// shared agent's pool still holds idle keep-alive connections
    /// keyed to it — the next fixture's requests would ghost into the
    /// old fixture's lingering connection threads and the hit-count
    /// evidence would lie. Parking every listener here keeps each
    /// fixture's host:port unique for the process's life.
    fn graveyard() -> &'static Mutex<Vec<TcpListener>> {
        static GRAVEYARD: OnceLock<Mutex<Vec<TcpListener>>> = OnceLock::new();
        GRAVEYARD.get_or_init(|| Mutex::new(Vec::new()))
    }

    impl RangeServer {
        fn start(handler: Handler) -> RangeServer {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let port = listener.local_addr().unwrap().port();
            let heard = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let thread_heard = Arc::clone(&heard);
            let thread_stop = Arc::clone(&stop);
            let accept = listener.try_clone().unwrap();
            let join = std::thread::spawn(move || loop {
                // The stop check leads the iteration: a BLOCKING accept
                // wakes only when a connection lands, and Drop's knock
                // is that connection — checking only on the WouldBlock
                // arm would accept the knock and block in accept()
                // again, wedging Drop's join forever (the windows CI
                // stall: a cloned listener there does not keep the
                // original's nonblocking mode).
                if thread_stop.load(Ordering::SeqCst) {
                    break;
                }
                match accept.accept() {
                    Ok((stream, _)) => {
                        let conn_handler = Arc::clone(&handler);
                        let conn_heard = Arc::clone(&thread_heard);
                        std::thread::spawn(move || serve(stream, &conn_handler, &conn_heard));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            });
            RangeServer {
                url: format!("http://127.0.0.1:{port}/image.tfs"),
                heard,
                stop,
                join: Some(join),
                listener: Some(listener),
            }
        }

        fn hits(&self) -> usize {
            self.heard.lock().unwrap().len()
        }

        fn heard(&self) -> Vec<Heard> {
            self.heard.lock().unwrap().clone()
        }
    }

    impl Drop for RangeServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            // Knock on the door so a blocking accept() wakes and sees
            // the flag; the connection threads are detached and end
            // with the client's sockets. Bounded: an untimed connect
            // has no business in a teardown path.
            if let Some(addr) = self
                .url
                .strip_prefix("http://")
                .and_then(|rest| rest.split('/').next())
                .and_then(|authority| authority.parse::<std::net::SocketAddr>().ok())
            {
                let _ = TcpStream::connect_timeout(&addr, Duration::from_secs(2));
            }
            if let Some(join) = self.join.take() {
                // A wedged accept thread must never wedge the harness:
                // join behind a timeout and leak the thread on expiry
                // (the test process reaps it at exit, like the
                // graveyard leaks the listener).
                let (done_tx, done_rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let _ = join.join();
                    let _ = done_tx.send(());
                });
                let _ = done_rx.recv_timeout(Duration::from_secs(10));
            }
            // The listener never closes — see graveyard().
            if let Some(listener) = self.listener.take() {
                graveyard().lock().unwrap().push(listener);
            }
        }
    }

    /// One connection's keep-alive loop: read a header block, log it,
    /// answer it, loop — until the client goes away (EOF, timeout) or
    /// the reply is the truncation injection (the only answer that
    /// closes, mid-body, content-length still spelling the full length).
    fn serve(mut stream: TcpStream, handler: &Handler, heard: &Arc<Mutex<Vec<Heard>>>) {
        // A socket accepted from a nonblocking listener inherits that
        // mode on some platforms (BSD/macOS) — pin it explicitly or the
        // keep-alive read below can WouldBlock-exit the thread mid-test.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut pending: Vec<u8> = Vec::new();
        loop {
            let head_end = loop {
                if let Some(pos) = pending.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
                let mut buf = [0u8; 4096];
                match stream.read(&mut buf) {
                    // the Drop knock opens and closes without a byte
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        pending.extend_from_slice(&buf[..n]);
                        if pending.len() > 64 * 1024 {
                            return;
                        }
                    }
                }
            };
            let request: Vec<u8> = pending.drain(..head_end).collect();
            let text = String::from_utf8_lossy(&request);
            let mut lines = text.split("\r\n");
            let path = lines
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/")
                .to_string();
            let mut range = None;
            let mut if_range = None;
            for line in lines {
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("range") {
                        range = Some(value.trim().to_string());
                    } else if name.eq_ignore_ascii_case("if-range") {
                        if_range = Some(value.trim().to_string());
                    }
                }
            }
            let heard_one = Heard {
                path,
                range,
                if_range,
            };
            let hit = {
                let mut log = heard.lock().unwrap();
                log.push(heard_one.clone());
                log.len() - 1
            };
            let reply = handler(hit, &heard_one);
            let closing = reply.truncate_to.is_some();
            let mut head = format!(
                "HTTP/1.1 {} {}\r\ncontent-length: {}\r\n",
                reply.status,
                reply.reason,
                reply.body.len()
            );
            if closing {
                head.push_str("connection: close\r\n");
            }
            for (name, value) in &reply.headers {
                head.push_str(name);
                head.push_str(": ");
                head.push_str(value);
                head.push_str("\r\n");
            }
            head.push_str("\r\n");
            if stream.write_all(head.as_bytes()).is_err() {
                return;
            }
            let body = match reply.truncate_to {
                Some(n) => &reply.body[..n.min(reply.body.len())],
                None => &reply.body[..],
            };
            if stream.write_all(body).is_err() {
                return;
            }
            let _ = stream.flush();
            if closing {
                return;
            }
        }
    }

    /// The reference range responder: 206 with the exact window when a
    /// Range header rides (and any If-Range validator is current), 200
    /// with the whole object otherwise — the validator-mismatch law
    /// included, so the eager-fallback leg rides the same responder.
    fn range_responder(data: Arc<Vec<u8>>, etag: &str) -> Handler {
        let etag = etag.to_string();
        Arc::new(move |_, heard| {
            let stale = matches!(&heard.if_range, Some(v) if *v != etag);
            match (&heard.range, stale) {
                (Some(range), false) => {
                    let span = range.strip_prefix("bytes=").unwrap();
                    let (start, end) = span.split_once('-').unwrap();
                    let start: usize = start.parse().unwrap();
                    let end: usize = end.parse().unwrap();
                    assert!(end < data.len());
                    Reply::whole(206, "Partial Content", data[start..=end].to_vec())
                        .header(
                            "content-range",
                            format!("bytes {start}-{end}/{}", data.len()),
                        )
                        .header("etag", etag.clone())
                }
                _ => Reply::whole(200, "OK", data.as_ref().clone()).header("etag", etag.clone()),
            }
        })
    }

    // ----------------------- contract legs (§10) -----------------------

    #[test]
    fn a_206_answers_the_exact_window_with_total_and_etag() {
        let data = Arc::new(patterned(64));
        let server = RangeServer::start(range_responder(Arc::clone(&data), "\"v1\""));
        let answer = get_range(&server.url, ByteRange { offset: 8, len: 8 }, None, None).unwrap();
        let RangeAnswer::Partial(body) = answer else {
            panic!("expected Partial")
        };
        assert_eq!(body.bytes.as_slice(), &data[8..16]);
        assert_eq!(body.total, 64);
        assert_eq!(body.etag.as_deref(), Some("\"v1\""));
        assert_eq!(server.hits(), 1);
        let heard = server.heard();
        assert_eq!(heard[0].path, "/image.tfs");
        assert_eq!(heard[0].range.as_deref(), Some("bytes=8-15"));
    }

    #[test]
    fn a_short_final_group_comes_back_exact() {
        let data = Arc::new(patterned(1000));
        let server = RangeServer::start(range_responder(Arc::clone(&data), "\"v1\""));
        let group = ByteRange::group(3, 256, 1000);
        let answer = get_range(&server.url, group, None, None).unwrap();
        let RangeAnswer::Partial(body) = answer else {
            panic!("expected Partial")
        };
        assert_eq!(body.bytes.as_slice(), &data[768..]);
        assert_eq!(body.total, 1000);
        assert_eq!(server.heard()[0].range.as_deref(), Some("bytes=768-999"));
    }

    #[test]
    fn a_200_answer_is_the_eager_fallback_signal() {
        let data = Arc::new(patterned(300));
        let whole = Arc::clone(&data);
        let server = RangeServer::start(Arc::new(move |_, _| {
            Reply::whole(200, "OK", whole.as_ref().clone()).header("etag", "\"v1\"".to_string())
        }));
        let answer = get_range(&server.url, ByteRange { offset: 8, len: 8 }, None, None).unwrap();
        let RangeAnswer::Full(body) = answer else {
            panic!("expected Full — a 200 must never pose as the window")
        };
        assert_eq!(body.bytes.as_slice(), data.as_slice());
        assert_eq!(body.total, 300);
        assert_eq!(body.etag.as_deref(), Some("\"v1\""));
        assert_eq!(server.hits(), 1);
    }

    #[test]
    fn if_range_revalidates_and_a_stale_validator_falls_back() {
        let data = Arc::new(patterned(64));
        let server = RangeServer::start(range_responder(Arc::clone(&data), "\"v2\""));
        let range = ByteRange { offset: 0, len: 16 };

        let answer = get_range(&server.url, range, Some("\"v2\""), None).unwrap();
        let RangeAnswer::Partial(body) = answer else {
            panic!("a current validator answers Partial")
        };
        assert_eq!(body.bytes.as_slice(), &data[0..16]);

        let answer = get_range(&server.url, range, Some("\"v1\""), None).unwrap();
        let RangeAnswer::Full(body) = answer else {
            panic!("a stale validator answers the whole object")
        };
        assert_eq!(body.bytes.as_slice(), data.as_slice());

        let heard = server.heard();
        assert_eq!(heard.len(), 2);
        assert_eq!(heard[0].if_range.as_deref(), Some("\"v2\""));
        assert_eq!(heard[1].if_range.as_deref(), Some("\"v1\""));
    }

    #[test]
    fn a_throttled_answer_waits_the_hint_then_retries() {
        let data = Arc::new(patterned(64));
        let responder = range_responder(Arc::clone(&data), "\"v1\"");
        let server = RangeServer::start(Arc::new(move |hit, heard| {
            if hit == 0 {
                Reply::whole(429, "Too Many Requests", Vec::new())
                    .header("retry-after", "0".to_string())
            } else {
                responder(hit, heard)
            }
        }));
        let answer = get_range(&server.url, ByteRange { offset: 8, len: 8 }, None, None).unwrap();
        assert!(matches!(answer, RangeAnswer::Partial(_)));
        assert_eq!(server.hits(), 2);
    }

    #[test]
    fn a_persistent_500_fails_after_the_attempt_budget() {
        let server = RangeServer::start(Arc::new(|_, _| {
            Reply::whole(500, "Internal Server Error", b"boom".to_vec())
        }));
        let err = get_range(&server.url, ByteRange { offset: 8, len: 8 }, None, None).unwrap_err();
        assert!(matches!(err, FetchError::DownloadFailed(_)), "{err:?}");
        assert_eq!(server.hits(), RANGE_ATTEMPTS as usize, "{err:?}");
    }

    #[test]
    fn a_truncated_body_retries_the_range_from_zero() {
        let data = Arc::new(patterned(64));
        let responder = range_responder(Arc::clone(&data), "\"v1\"");
        let server = RangeServer::start(Arc::new(move |hit, heard| {
            let mut reply = responder(hit, heard);
            if hit == 0 {
                // The header keeps the full length; only half the window
                // reaches the wire — a dropped mid-stream body.
                reply.truncate_to = Some(4);
            }
            reply
        }));
        let answer = get_range(&server.url, ByteRange { offset: 8, len: 8 }, None, None).unwrap();
        let RangeAnswer::Partial(body) = answer else {
            panic!("expected Partial after the retry")
        };
        assert_eq!(body.bytes.as_slice(), &data[8..16]);
        assert_eq!(server.hits(), 2);
    }

    #[test]
    fn a_wrong_content_range_is_retried_then_named() {
        let server = RangeServer::start(Arc::new(|_, _| {
            // Off-by-one start, every time: never the requested window.
            Reply::whole(206, "Partial Content", patterned(8))
                .header("content-range", "bytes 9-16/64".to_string())
        }));
        let err = get_range(&server.url, ByteRange { offset: 8, len: 8 }, None, None).unwrap_err();
        let FetchError::DownloadFailed(msg) = err else {
            panic!("expected DownloadFailed: {err:?}")
        };
        assert!(msg.contains("bytes 9-16"), "{msg}");
        assert_eq!(server.hits(), RANGE_ATTEMPTS as usize, "{msg}");
    }

    #[test]
    fn a_404_is_the_missing_object_error_not_a_retry() {
        let server = RangeServer::start(Arc::new(|_, _| {
            Reply::whole(404, "Not Found", b"no such object".to_vec())
        }));
        let err = get_range(&server.url, ByteRange { offset: 0, len: 8 }, None, None).unwrap_err();
        assert!(matches!(err, FetchError::IndexUnavailable(_)), "{err:?}");
        assert_eq!(server.hits(), 1);
    }

    #[test]
    fn a_401_is_the_credential_error_not_a_retry() {
        let server = RangeServer::start(Arc::new(|_, _| {
            Reply::whole(401, "Unauthorized", b"credentials required".to_vec())
        }));
        let err = get_range(&server.url, ByteRange { offset: 0, len: 8 }, None, None).unwrap_err();
        assert!(matches!(err, FetchError::AuthRejected { .. }), "{err:?}");
        assert_eq!(server.hits(), 1);
    }

    // ---------------- the write-verbs fixture (spec 38 §7) ----------------

    /// What the write fixture heard on one request: the method line's
    /// two halves, the headers the publish path sets, and the exact body
    /// bytes (the range fixture never reads bodies — writes need them).
    #[derive(Debug, Clone)]
    struct WriteHeard {
        method: String,
        path: String,
        content_type: Option<String>,
        auth: Option<String>,
        body: Vec<u8>,
    }

    /// A minimal write server: one request per connection (answered
    /// `connection: close` — the shared upload agent pools keep-alive
    /// sockets, so the listener rides the same graveyard as the range
    /// fixture's and its port is never recycled under a stale pooled
    /// connection), the request body read to its content-length, replies
    /// scripted 1:1 (the last one repeating when the script runs dry).
    struct WriteServer {
        url: String,
        heard: Arc<Mutex<Vec<WriteHeard>>>,
        stop: Arc<AtomicBool>,
        join: Option<std::thread::JoinHandle<()>>,
        listener: Option<TcpListener>,
    }

    impl WriteServer {
        fn start(replies: Vec<Reply>) -> WriteServer {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let port = listener.local_addr().unwrap().port();
            let heard = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let thread_heard = Arc::clone(&heard);
            let thread_stop = Arc::clone(&stop);
            let accept = listener.try_clone().unwrap();
            let join = std::thread::spawn(move || loop {
                if thread_stop.load(Ordering::SeqCst) {
                    break;
                }
                match accept.accept() {
                    Ok((stream, _)) => {
                        let conn_heard = Arc::clone(&thread_heard);
                        let conn_replies = Arc::new(replies.clone());
                        std::thread::spawn(move || serve_write(stream, &conn_replies, &conn_heard));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            });
            WriteServer {
                url: format!("http://127.0.0.1:{port}"),
                heard,
                stop,
                join: Some(join),
                listener: Some(listener),
            }
        }

        fn heard(&self) -> Vec<WriteHeard> {
            self.heard.lock().unwrap().clone()
        }
    }

    impl Drop for WriteServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(addr) = self
                .url
                .strip_prefix("http://")
                .and_then(|authority| authority.parse::<std::net::SocketAddr>().ok())
            {
                let _ = TcpStream::connect_timeout(&addr, Duration::from_secs(2));
            }
            if let Some(join) = self.join.take() {
                let (done_tx, done_rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let _ = join.join();
                    let _ = done_tx.send(());
                });
                let _ = done_rx.recv_timeout(Duration::from_secs(10));
            }
            if let Some(listener) = self.listener.take() {
                graveyard().lock().unwrap().push(listener);
            }
        }
    }

    /// One request on one connection: head, then exactly content-length
    /// body bytes, logged, answered, closed. A request without a
    /// content-length is read as body-less (the write verbs always send
    /// one — ureq sizes a `&[u8]` body).
    fn serve_write(mut stream: TcpStream, replies: &[Reply], heard: &Arc<Mutex<Vec<WriteHeard>>>) {
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut pending: Vec<u8> = Vec::new();
        let head_end = loop {
            if let Some(pos) = pending.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
            let mut buf = [0u8; 4096];
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    if pending.len() > 1024 * 1024 {
                        return;
                    }
                }
            }
        };
        let request: Vec<u8> = pending.drain(..head_end).collect();
        let text = String::from_utf8_lossy(&request);
        let mut lines = text.split("\r\n");
        let request_line = lines.next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("/").to_string();
        let mut content_type = None;
        let mut auth = None;
        let mut content_length = 0usize;
        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-type") {
                    content_type = Some(value.trim().to_string());
                } else if name.eq_ignore_ascii_case("authorization") {
                    auth = Some(value.trim().to_string());
                } else if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        while pending.len() < content_length {
            let mut buf = [0u8; 8192];
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => pending.extend_from_slice(&buf[..n]),
            }
        }
        let body: Vec<u8> = pending.drain(..content_length).collect();
        let hit = {
            let mut log = heard.lock().unwrap();
            log.push(WriteHeard {
                method,
                path,
                content_type,
                auth,
                body,
            });
            log.len() - 1
        };
        let reply = &replies[hit.min(replies.len() - 1)];
        let mut head = format!(
            "HTTP/1.1 {} {}\r\ncontent-length: {}\r\nconnection: close\r\n",
            reply.status,
            reply.reason,
            reply.body.len()
        );
        for (name, value) in &reply.headers {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        if stream.write_all(head.as_bytes()).is_err() {
            return;
        }
        let _ = stream.write_all(&reply.body);
        let _ = stream.flush();
    }

    // --------------------- write-verb legs (spec 38 §7) ---------------------

    #[test]
    fn post_raw_delivers_the_body_type_and_credential_header() {
        let server = WriteServer::start(vec![Reply::whole(202, "Accepted", Vec::new())
            .header("location", "/v2/repo/blobs/uploads/abc".to_string())]);
        let url = format!(
            "{}/v2/repo/blobs/uploads/?mount=sha256:aa&from=repo",
            server.url
        );
        let response = post_raw(&url, b"", None, Some(("Authorization", "Bearer tok"))).unwrap();
        assert_eq!(response.status, 202);
        assert_eq!(
            response.header("location"),
            Some("/v2/repo/blobs/uploads/abc")
        );
        let heard = server.heard();
        assert_eq!(heard.len(), 1);
        assert_eq!(heard[0].method, "POST");
        assert_eq!(
            heard[0].path,
            "/v2/repo/blobs/uploads/?mount=sha256:aa&from=repo"
        );
        assert_eq!(heard[0].auth.as_deref(), Some("Bearer tok"));
        assert!(heard[0].body.is_empty());
    }

    #[test]
    fn post_raw_carries_a_content_type_when_given_one() {
        let server = WriteServer::start(vec![Reply::whole(200, "OK", Vec::new())]);
        let url = format!("{}/v2/x", server.url);
        post_raw(&url, b"{}", Some("application/json"), None).unwrap();
        let heard = server.heard();
        assert_eq!(heard[0].content_type.as_deref(), Some("application/json"));
        assert_eq!(heard[0].body, b"{}");
        assert_eq!(heard[0].auth, None);
    }

    #[test]
    fn put_raw_delivers_the_manifest_bytes_and_status_unclassified() {
        let server = WriteServer::start(vec![Reply::whole(201, "Created", Vec::new())
            .header("docker-content-digest", "sha256:bb".to_string())]);
        let url = format!("{}/v2/repo/manifests/1.0", server.url);
        let response = put_raw(
            &url,
            b"{\"schemaVersion\":2}",
            "application/vnd.oci.image.manifest.v1+json",
            Some(("Authorization", "Bearer tok")),
        )
        .unwrap();
        assert_eq!(response.status, 201);
        assert_eq!(response.header("docker-content-digest"), Some("sha256:bb"));
        let heard = server.heard();
        assert_eq!(heard.len(), 1);
        assert_eq!(heard[0].method, "PUT");
        assert_eq!(heard[0].path, "/v2/repo/manifests/1.0");
        assert_eq!(
            heard[0].content_type.as_deref(),
            Some("application/vnd.oci.image.manifest.v1+json")
        );
        assert_eq!(heard[0].body, b"{\"schemaVersion\":2}");
    }

    #[test]
    fn the_write_verbs_refuse_a_plain_http_remote() {
        let err = post_raw("http://registry.example/v2/x", b"", None, None).unwrap_err();
        let FetchError::DownloadFailed(msg) = err else {
            panic!("expected DownloadFailed: {err:?}")
        };
        assert!(msg.contains("refusing non-HTTPS URL"), "{msg}");
        let err = put_raw("http://registry.example/v2/x", b"", "text/plain", None).unwrap_err();
        assert!(matches!(err, FetchError::DownloadFailed(_)), "{err:?}");
    }

    #[test]
    fn the_write_verbs_surface_a_401_unclassified() {
        let server = WriteServer::start(vec![Reply::whole(401, "Unauthorized", Vec::new())
            .header(
                "www-authenticate",
                "Bearer realm=\"https://auth.example/token\"".to_string(),
            )]);
        let url = format!("{}/v2/x", server.url);
        let response = put_raw(&url, b"{}", "application/json", None).unwrap();
        assert_eq!(response.status, 401);
        assert!(response
            .header("www-authenticate")
            .unwrap()
            .contains("Bearer realm="));
    }
}
