//! The spec 35 §3 TLS probe (tebako doctor's network section) against
//! the MITM fixture: a local rustls server presents a certificate signed
//! by a self-signed test CA. The verdict mapping, deterministically:
//!   - extra_ca carrying the CA PEM      → EffectiveOk;
//!   - default (bundled webpki) roots    → a cert rejection — the
//!     platform leg decides PlatformOnly vs NeitherTrusted (both are
//!     trust verdicts; which one depends on the host's OS store);
//!   - a closed port                     → Unreachable (a transport
//!     condition, never a trust finding).

mod common;

use std::path::Path;
use std::time::Duration;

use tebako_http::netconfig::NetworkConfig;
use tebako_http::TlsProbe;

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn probe_tls_accepts_the_chain_with_extra_ca() {
    let port = common::spawn_tls_server();
    let host = format!("127.0.0.1:{port}");
    let ca = common::fixtures_dir().join("ca.pem");
    let cfg = NetworkConfig::default().merge_file(
        None,
        None,
        vec![ca],
        Path::new("probe-fixture"),
    );
    assert_eq!(
        tebako_http::probe_tls(&host, &cfg, TIMEOUT),
        TlsProbe::EffectiveOk
    );
}

#[test]
fn probe_tls_names_an_untrusted_chain() {
    let port = common::spawn_tls_server();
    let host = format!("127.0.0.1:{port}");
    let cfg = NetworkConfig::default();
    let verdict = tebako_http::probe_tls(&host, &cfg, TIMEOUT);
    assert!(
        matches!(verdict, TlsProbe::NeitherTrusted | TlsProbe::PlatformOnly),
        "the fixture CA is in neither root set by default: {verdict:?}"
    );
}

#[test]
fn probe_tls_unreachable_is_not_a_trust_finding() {
    let cfg = NetworkConfig::default();
    let verdict = tebako_http::probe_tls("127.0.0.1:1", &cfg, Duration::from_secs(2));
    assert!(
        matches!(verdict, TlsProbe::Unreachable(_)),
        "a closed port is a transport condition: {verdict:?}"
    );
}
