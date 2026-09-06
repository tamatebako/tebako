//! Enterprise networking configuration (TODO.v2-1/33, spec 04 amendment):
//! proxies and custom trust anchors for the loader's downloads.
//!
//! One client, one rule still holds — every fetch in the stack rides the
//! [`crate`] agents, so the effective [`NetworkConfig`] resolves ONCE per
//! process from three layers, first hit wins **per key**:
//!
//! 1. process environment (`HTTPS_PROXY` / `HTTP_PROXY` / `ALL_PROXY` /
//!    `NO_PROXY`, `TEBAKO_TLS_PLATFORM_ROOTS`, `TEBAKO_EXTRA_CA`),
//! 2. the `network:` section of `~/.tebako/config.yaml` (installed by the
//!    binaries at startup via [`set_global`]),
//! 3. direct connection with the bundled Mozilla roots (the default,
//!    unchanged).
//!
//! Proxy grammar is ureq's own (CONNECT over http/https; URL-embedded
//! credentials; NO_PROXY comma list with exact / `.suffix` / `*` entries;
//! localhost always direct). SOCKS is refused with a named error — v1 of
//! the feature is CONNECT only. TLS roots are never *less* than the
//! bundled set: the choice is webpki-roots (default), the platform
//! verifier (`tls_roots: platform` / `TEBAKO_TLS_PLATFORM_ROOTS`), or
//! **additive** (`extra_ca:` / `TEBAKO_EXTRA_CA` — corporate PEMs parsed
//! INTO the bundled store). Platform + additive is a named error (the
//! platform verifier cannot take extra roots); a verify-off spelling does
//! not exist.
//!
//! Every resolution produces audit lines ([`NetworkConfig::audit`]) — the
//! installing binary journals them (the loud record the trust story
//! requires).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use ureq::tls::{Certificate, RootCerts};

/// OS-path-list-separated set of PEM files to add to the bundled root
/// store (`:`-separated on unix, `;` on windows — `std::env::split_paths`).
pub const EXTRA_CA_ENV: &str = "TEBAKO_EXTRA_CA";

/// Which root store the TLS layer trusts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TlsRoots {
    /// Mozilla's webpki-roots, bundled (default; the MITM-safe choice).
    #[default]
    WebPki,
    /// The OS trust store via the platform verifier (GPO/MDM-pushed
    /// enterprise roots ride this).
    Platform,
}

impl fmt::Display for TlsRoots {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsRoots::WebPki => write!(f, "webpki"),
            TlsRoots::Platform => write!(f, "platform"),
        }
    }
}

/// The effective network configuration for this process.
#[derive(Debug, Clone, Default)]
pub struct NetworkConfig {
    /// Explicit proxy URL (`<scheme>://[user[:pass]@]host[:port]`) from
    /// the config file. When None, ureq's env grammar still applies at
    /// agent build (`Proxy::try_from_env`), so "no config proxy" never
    /// means "no proxy".
    pub proxy_url: Option<String>,
    /// The trust-anchor choice.
    pub tls_roots: TlsRoots,
    /// Extra CA PEM files to add to the bundled store.
    pub extra_ca: Vec<PathBuf>,
    /// Human-readable record of what resolved from where (journaled by
    /// the installer; empty for the pure-default resolution).
    pub audit: Vec<String>,
}

/// Named errors on the network-config path — never a silent fallback.
#[derive(Debug, Clone)]
pub enum NetConfigError {
    /// The configured proxy URL does not parse.
    ProxyUrlInvalid(String),
    /// The configured proxy names a SOCKS scheme — v1 of the feature is
    /// CONNECT-only (ureq is built without the socks transport).
    ProxySchemeUnsupported(String),
    /// `extra_ca` combined with the platform verifier — the platform
    /// verifier cannot take added roots; pick one trust story.
    ExtraCaWithPlatformRoots,
    /// An `extra_ca` path does not read.
    ExtraCaUnreadable { path: PathBuf, why: String },
    /// An `extra_ca` file holds no parseable certificate block.
    ExtraCaMalformed { path: PathBuf, why: String },
}

impl fmt::Display for NetConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetConfigError::ProxyUrlInvalid(what) => write!(
                f,
                "invalid proxy URL `{what}` — expected <scheme>://[user[:pass]@]host[:port] \
                 (http/https CONNECT)"
            ),
            NetConfigError::ProxySchemeUnsupported(what) => write!(
                f,
                "proxy scheme in `{what}` is not supported — tebako rides http/https \
                 CONNECT proxies only (no SOCKS)"
            ),
            NetConfigError::ExtraCaWithPlatformRoots => write!(
                f,
                "tls_roots: platform cannot combine with extra_ca — the OS verifier \
                 trusts exactly the OS store; either push the CA into the OS store \
                 (GPO/MDM) or use the bundled roots plus extra_ca"
            ),
            NetConfigError::ExtraCaUnreadable { path, why } => {
                write!(f, "cannot read extra CA {}: {why}", path.display())
            }
            NetConfigError::ExtraCaMalformed { path, why } => write!(
                f,
                "no certificate block parses from extra CA {}: {why} — expected PEM \
                 (-----BEGIN CERTIFICATE-----)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for NetConfigError {}

static GLOBAL: OnceLock<RwLock<NetworkConfig>> = OnceLock::new();

fn slot() -> &'static RwLock<NetworkConfig> {
    GLOBAL.get_or_init(|| RwLock::new(NetworkConfig::from_env()))
}

/// The process-wide effective config. Defaults to
/// [`NetworkConfig::from_env`] until a binary installs the
/// config-file-resolved one at startup.
pub fn global() -> NetworkConfig {
    slot().read().expect("network config lock").clone()
}

/// Install the effective config (env over `network:` per key, already
/// merged by the caller — see tebako-shim's config). Must run before the
/// first fetch; agent construction caches the resolved transport.
pub fn set_global(cfg: NetworkConfig) {
    *slot().write().expect("network config lock") = cfg;
}

/// The audit spelling of a proxy URL: credentials never touch the
/// journal (spec 11 §11's log discipline applies to proxy auth too) —
/// userinfo is replaced by `***`.
fn redact_proxy_url(url: &str) -> String {
    match ureq::Proxy::new(url) {
        Ok(p) if p.username().is_some() => {
            let scheme = match p.protocol() {
                ureq::ProxyProtocol::Https => "https",
                _ => "http",
            };
            format!("{scheme}://***@{}:{}", p.host(), p.port())
        }
        _ => url.to_string(),
    }
}

impl NetworkConfig {
    /// The env-only resolution (the default before any config install).
    /// The proxy URL stays None here — env proxies are read by ureq at
    /// agent build so NO_PROXY rides its own tested grammar.
    pub fn from_env() -> Self {
        let tls_roots = if std::env::var_os(crate::PLATFORM_ROOTS_ENV).is_some() {
            TlsRoots::Platform
        } else {
            TlsRoots::WebPki
        };
        let extra_ca: Vec<PathBuf> = std::env::var_os(EXTRA_CA_ENV)
            .map(|v| std::env::split_paths(&v).collect())
            .unwrap_or_default();
        let mut audit = Vec::new();
        if tls_roots == TlsRoots::Platform {
            audit.push(format!(
                "tls_roots=platform (env {})",
                crate::PLATFORM_ROOTS_ENV
            ));
        }
        if !extra_ca.is_empty() {
            audit.push(format!(
                "extra_ca={} file(s) (env {EXTRA_CA_ENV})",
                extra_ca.len()
            ));
        }
        NetworkConfig {
            proxy_url: None,
            tls_roots,
            extra_ca,
            audit,
        }
    }

    /// Merge a config-file `network:` section UNDER the environment (env
    /// wins per key; this is the binaries' startup call shape).
    /// `proxy_url`/`tls_roots`/`extra_ca` carry the file's values;
    /// None/empty means "not set in the file".
    pub fn merge_file(
        mut self,
        proxy_url: Option<String>,
        tls_roots: Option<TlsRoots>,
        extra_ca: Vec<PathBuf>,
        source: &Path,
    ) -> Self {
        if self.proxy_url.is_none() && proxy_url.is_some() {
            self.audit.push(format!(
                "proxy={} ({})",
                redact_proxy_url(proxy_url.as_deref().unwrap_or("")),
                source.display()
            ));
            self.proxy_url = proxy_url;
        }
        if std::env::var_os(crate::PLATFORM_ROOTS_ENV).is_none()
            && tls_roots == Some(TlsRoots::Platform)
        {
            self.audit
                .push(format!("tls_roots=platform ({})", source.display()));
            self.tls_roots = TlsRoots::Platform;
        }
        if self.extra_ca.is_empty() && !extra_ca.is_empty() {
            self.audit.push(format!(
                "extra_ca={} file(s) ({})",
                extra_ca.len(),
                source.display()
            ));
            self.extra_ca = extra_ca;
        }
        self
    }

    /// Validate cross-field invariants (the fail-closed half of the
    /// trust story) before any fetch rides this config.
    pub fn validate(&self) -> Result<(), NetConfigError> {
        if self.tls_roots == TlsRoots::Platform && !self.extra_ca.is_empty() {
            return Err(NetConfigError::ExtraCaWithPlatformRoots);
        }
        if let Some(url) = &self.proxy_url {
            // compose_proxy owns the grammar (ureq's parser) and refuses
            // socks by name; a config-sourced URL parses at startup so a
            // bad value dies there, not mid-fetch.
            compose_proxy(url, None)?;
        }
        Ok(())
    }
}

/// Compose a Proxy from an explicit URL plus a NO_PROXY expression (the
/// config-file path): ureq's parser owns the URL grammar, ureq's builder
/// owns the NO_PROXY grammar. Pure — the env read happens in
/// [`resolve_proxy`].
fn compose_proxy(url: &str, no_proxy_expr: Option<&str>) -> Result<ureq::Proxy, NetConfigError> {
    let parsed = ureq::Proxy::new(url)
        .map_err(|e| NetConfigError::ProxyUrlInvalid(format!("{url}: {e}")))?;
    // ureq parses socks schemes without its (disabled) transport — refuse
    // them by name here instead of dying at connect time.
    match parsed.protocol() {
        ureq::ProxyProtocol::Http | ureq::ProxyProtocol::Https => {}
        other => {
            return Err(NetConfigError::ProxySchemeUnsupported(format!(
                "{url} ({other:?})"
            )))
        }
    }
    let mut builder = ureq::Proxy::builder(parsed.protocol())
        .host(parsed.host())
        .port(parsed.port());
    if let Some(u) = parsed.username() {
        builder = builder.username(u);
    }
    if let Some(p) = parsed.password() {
        builder = builder.password(p);
    }
    if let Some(expr) = no_proxy_expr {
        for entry in expr.split(',') {
            let entry = entry.trim();
            if !entry.is_empty() {
                builder = builder.no_proxy(entry);
            }
        }
    }
    builder
        .build()
        .map_err(|e| NetConfigError::ProxyUrlInvalid(format!("{url}: {e}")))
}

/// Build the ureq proxy for a config: the explicit URL wins (with the
/// NO_PROXY env grammar applied on top), else ureq's own env read (which
/// carries NO_PROXY natively). None = direct.
pub(crate) fn resolve_proxy(cfg: &NetworkConfig) -> Result<Option<ureq::Proxy>, NetConfigError> {
    if let Some(url) = &cfg.proxy_url {
        let no_proxy = std::env::var_os("NO_PROXY")
            .or_else(|| std::env::var_os("no_proxy"))
            .map(|v| v.to_string_lossy().into_owned());
        return compose_proxy(url, no_proxy.as_deref()).map(Some);
    }
    Ok(ureq::Proxy::try_from_env())
}

/// The root-certs choice for the agent builder. Additive mode parses
/// every extra PEM into the SAME store as the bundled Mozilla set —
/// fail-closed per file, named per failure.
pub(crate) fn resolve_roots(cfg: &NetworkConfig) -> Result<RootCerts, NetConfigError> {
    cfg.validate()?;
    match (cfg.tls_roots, cfg.extra_ca.is_empty()) {
        (TlsRoots::Platform, _) => Ok(RootCerts::PlatformVerifier),
        (TlsRoots::WebPki, true) => Ok(RootCerts::WebPki),
        (TlsRoots::WebPki, false) => {
            let mut certs: Vec<Certificate<'static>> = webpki_root_certs::TLS_SERVER_ROOT_CERTS
                .iter()
                .map(|der| Certificate::from_der(der.as_ref()))
                .collect();
            for path in &cfg.extra_ca {
                certs.extend(load_pem_certs(path)?);
            }
            Ok(RootCerts::new_with_certs(&certs))
        }
    }
}

/// Every certificate block in one PEM file (a corporate export often
/// carries the root AND intermediates — all land; rustls picks).
fn load_pem_certs(path: &Path) -> Result<Vec<Certificate<'static>>, NetConfigError> {
    let bytes = std::fs::read(path).map_err(|e| NetConfigError::ExtraCaUnreadable {
        path: path.to_path_buf(),
        why: e.to_string(),
    })?;
    let mut certs = Vec::new();
    let mut cursor = std::io::Cursor::new(&bytes);
    for item in rustls_pemfile::certs(&mut cursor) {
        let der = item.map_err(|e| NetConfigError::ExtraCaMalformed {
            path: path.to_path_buf(),
            why: e.to_string(),
        })?;
        certs.push(Certificate::from_der(der.as_ref()).to_owned());
    }
    if certs.is_empty() {
        return Err(NetConfigError::ExtraCaMalformed {
            path: path.to_path_buf(),
            why: "no -----BEGIN CERTIFICATE----- block found".to_string(),
        });
    }
    Ok(certs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real self-signed test root (generated 2026-09-06:
    /// `openssl req -x509 -newkey rsa:2048 -nodes -days 3650
    /// -subj "/CN=Tebako Test Root CA"`).
    const TEST_CA_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIDHTCCAgWgAwIBAgIUbscliGH/N5CSPJBThlIWafxIFaYwDQYJKoZIhvcNAQEL\n\
BQAwHjEcMBoGA1UEAwwTVGViYWtvIFRlc3QgUm9vdCBDQTAeFw0yNjA5MDYwNzA0\n\
MDhaFw0zNjA5MDMwNzA0MDhaMB4xHDAaBgNVBAMME1RlYmFrbyBUZXN0IFJvb3Qg\n\
Q0EwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDFjAZxPs40rR5cpKUk\n\
JJqGJRq4fpcEugehFXa7xfG1VW80d2DBXYmypZ9IknxFk9Nnq6OLP0IGZagbvpbs\n\
k7DQXoCuLOuwkpH+kUWzL8ZMSXqX5uHm492imHQXDgXVc8vaoELSEjHlxepyRu3B\n\
vCP/fYnDSUGpmwxQAWEtLqD9/fqm8fu0yPsjyzXpr10+ijw0VimRsNIXn4CkBSo9\n\
JrExuShhvHKV2UN12dakX4+bG0mLaPCPfSE5ICc0hWtMIHx/AZSUoGB94VL+g+JJ\n\
ss0rnGaxjGV8lJPTmTrYOdikt/NGqgXFprJVPikSQ4WzwFN3Dx9matVwco43d/ms\n\
ZlPjAgMBAAGjUzBRMB0GA1UdDgQWBBTGY0fUF9M9TziFyymP20OKC5r88jAfBgNV\n\
HSMEGDAWgBTGY0fUF9M9TziFyymP20OKC5r88jAPBgNVHRMBAf8EBTADAQH/MA0G\n\
CSqGSIb3DQEBCwUAA4IBAQBhU7Ow+gGNxnckDmjPHlIYdfoAwjCsXKmzBxL7ewQC\n\
pAQnc8UxLi3iqg5pngkPEOuP6bNyUUSZVg4TSL7RQVodSlh3s/yD1pSBh0NCnD+7\n\
irDpdk5uGBy5txlh/NJvVOxlxQ2oGjpgQiQ+4DY12tbFHB8v1FkZyIe82iWHeZbz\n\
V2o75fAjIdeSQHX6cr099a4GcZiKHGRsIk49xFzuUN/f1XNylwGh7EvF86I4CNvA\n\
RXSnOc5tBLxNY2iSg6ULMM+s4Q+VZknK9Vk84Xbq0oF9wdgnLR41BnxmrpfzDL/M\n\
wVYzxWmIj2VzW7jBacDLSIXvtFG6/Q7Zi5uJkatP7H6F\n\
-----END CERTIFICATE-----\n";

    #[test]
    fn default_resolution_is_webpki_and_direct() {
        let cfg = NetworkConfig::default();
        assert_eq!(cfg.tls_roots, TlsRoots::WebPki);
        assert!(cfg.proxy_url.is_none());
        assert!(cfg.extra_ca.is_empty());
        assert!(cfg.validate().is_ok());
        assert!(matches!(resolve_roots(&cfg).unwrap(), RootCerts::WebPki));
    }

    #[test]
    fn platform_plus_extra_ca_is_a_named_error() {
        let cfg = NetworkConfig {
            tls_roots: TlsRoots::Platform,
            extra_ca: vec![PathBuf::from("/tmp/whatever.pem")],
            ..Default::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(NetConfigError::ExtraCaWithPlatformRoots)
        ));
    }

    #[test]
    fn bad_proxy_url_is_a_named_error() {
        let cfg = NetworkConfig {
            proxy_url: Some("http://[::1".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(NetConfigError::ProxyUrlInvalid(_))
        ));
    }

    #[test]
    fn socks_proxy_url_is_refused() {
        // v1 is CONNECT-only; a socks scheme must surface, never silently
        // "work" via a wrong transport.
        assert!(compose_proxy("socks5://proxy.example.com:1080", None).is_err());
    }

    #[test]
    fn explicit_proxy_rebuilds_with_host_port_and_auth() {
        let proxy = compose_proxy("http://user:pass@proxy.example.com:3128", None).unwrap();
        assert_eq!(proxy.host(), "proxy.example.com");
        assert_eq!(proxy.port(), 3128);
        assert_eq!(proxy.username(), Some("user"));
        assert_eq!(proxy.password(), Some("pass"));
    }

    #[test]
    fn compose_proxy_applies_the_no_proxy_grammar() {
        // The grammar itself is ureq's (upstream property-tested); what
        // we pin here is that OUR composition actually attaches it.
        let proxy = compose_proxy(
            "http://proxy.example.com:3128",
            Some("internal.example.com, *.corp, *."),
        )
        .unwrap();
        let bypass: ureq::http::Uri = "https://internal.example.com/x".parse().unwrap();
        let through: ureq::http::Uri = "https://github.com/x".parse().unwrap();
        assert!(proxy.is_no_proxy(&bypass));
        assert!(!proxy.is_no_proxy(&through));
    }

    #[test]
    fn extra_ca_unreadable_and_malformed_are_named() {
        let dir = std::env::temp_dir().join(format!("tebako-http-netcfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("missing.pem");
        let err = load_pem_certs(&missing).unwrap_err();
        assert!(matches!(err, NetConfigError::ExtraCaUnreadable { .. }));
        let junk = dir.join("junk.pem");
        std::fs::write(&junk, b"not a pem\n").unwrap();
        let err = load_pem_certs(&junk).unwrap_err();
        assert!(matches!(err, NetConfigError::ExtraCaMalformed { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extra_ca_real_pem_joins_the_bundled_store() {
        let dir =
            std::env::temp_dir().join(format!("tebako-http-netcfg-pem-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ca.pem");
        std::fs::write(&path, TEST_CA_PEM).unwrap();
        let cfg = NetworkConfig {
            extra_ca: vec![path],
            ..Default::default()
        };
        match resolve_roots(&cfg).unwrap() {
            RootCerts::Specific(certs) => {
                // bundled Mozilla set + our one test root
                assert_eq!(
                    certs.len(),
                    webpki_root_certs::TLS_SERVER_ROOT_CERTS.len() + 1
                );
            }
            other => panic!("expected additive Specific roots, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_file_env_wins_per_key() {
        let env_layer = NetworkConfig {
            tls_roots: TlsRoots::Platform, // as if TEBAKO_TLS_PLATFORM_ROOTS were set
            ..NetworkConfig::from_env()
        };
        let merged = env_layer.merge_file(
            Some("http://user:secret@proxy.example.com:3128".to_string()),
            Some(TlsRoots::WebPki),
            vec![],
            Path::new("/home/u/.tebako/config.yaml"),
        );
        // env platform roots survive; the file's proxy lands — and the
        // audit record carries the REDACTED spelling, never the password.
        assert_eq!(merged.tls_roots, TlsRoots::Platform);
        assert_eq!(
            merged.proxy_url.as_deref(),
            Some("http://user:secret@proxy.example.com:3128")
        );
        let line = merged
            .audit
            .iter()
            .find(|l| l.starts_with("proxy="))
            .unwrap();
        assert!(line.contains("proxy=http://***@proxy.example.com:3128"));
        assert!(!line.contains("secret"));
    }

    // ---- NO_PROXY grammar (proptest against ureq's matcher, fed by
    // compose_proxy's comma split — spec 04 §4's de-facto grammar) -----

    fn no_proxy_matches(expr: &str, host: &str) -> bool {
        let proxy = compose_proxy("http://proxy.test:3128", Some(expr)).unwrap();
        let uri: ureq::http::Uri = format!("https://{host}/").parse().unwrap();
        proxy.is_no_proxy(&uri)
    }

    proptest::proptest! {
        /// `*` bypasses the proxy for every host.
        #[test]
        fn no_proxy_star_matches_everything(
            host in "[a-z][a-z0-9-]{0,12}(\\.[a-z][a-z0-9-]{0,12}){0,3}",
        ) {
            proptest::prop_assert!(no_proxy_matches("*", &host));
        }

        /// Exact entries match case-insensitively, both directions.
        #[test]
        fn no_proxy_exact_host_is_case_insensitive(
            lower in "[a-z][a-z0-9-]{0,12}(\\.[a-z][a-z0-9-]{0,12}){1,3}",
        ) {
            let upper = lower.to_ascii_uppercase();
            proptest::prop_assert!(no_proxy_matches(&upper, &lower));
            proptest::prop_assert!(no_proxy_matches(&lower, &upper));
        }

        /// `*.domain` matches any subdomain of domain but never the apex.
        #[test]
        fn no_proxy_wildcard_suffix_matches_subdomains_not_the_apex(
            sub in "[a-z][a-z0-9-]{0,8}",
            domain in "[a-z][a-z0-9-]{0,8}\\.[a-z]{2,6}",
        ) {
            let expr = format!("*.{domain}");
            let host = format!("{sub}.{domain}");
            proptest::prop_assert!(no_proxy_matches(&expr, &host));
            proptest::prop_assert!(!no_proxy_matches(&expr, &domain));
        }

        /// The suffix anchor is the DOT: neither a bare-string suffix
        /// (`not<domain>`) nor a longer name under the domain
        /// (`<domain>.<evil>`) may match `*.<domain>`.
        #[test]
        fn no_proxy_suffix_never_matches_a_lookalike(
            domain in "[a-z][a-z0-9-]{0,8}\\.[a-z]{2,6}",
            evil in "[a-z][a-z0-9-]{0,8}",
        ) {
            let expr = format!("*.{domain}");
            let squashed = format!("not{domain}");
            let longer = format!("{domain}.{evil}");
            proptest::prop_assert!(!no_proxy_matches(&expr, &squashed));
            proptest::prop_assert!(!no_proxy_matches(&expr, &longer));
        }

        /// The comma list matches any entry; entries trim whitespace.
        #[test]
        fn no_proxy_comma_list_matches_any_entry(
            a in "[a-z][a-z0-9-]{0,8}\\.[a-z]{2,6}",
            b in "[a-z][a-z0-9-]{0,8}\\.[a-z]{2,6}",
        ) {
            let expr = format!("{a}, {b}");
            proptest::prop_assert!(no_proxy_matches(&expr, &a));
            proptest::prop_assert!(no_proxy_matches(&expr, &b));
        }
    }
}
