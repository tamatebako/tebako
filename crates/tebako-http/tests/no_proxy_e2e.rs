//! NO_PROXY end-to-end (TODO.v2-1/33, spec 04 §4): the env-native path
//! (`ureq::Proxy::try_from_env` inside `resolve_proxy`) must carry
//! NO_PROXY's grammar to the wire — a covered target goes DIRECT (the
//! proxy's CONNECT log stays empty), an uncovered one rides the proxy.
//! One test in its own binary: env mutation can't race anything. This
//! also exercises the `TEBAKO_EXTRA_CA` env spelling end-to-end (trust
//! is held constant so routing is the only variable).

mod common;

use std::time::Duration;

use tebako_http::netconfig::NetworkConfig;

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn no_proxy_env_governs_direct_vs_proxied() {
    let server_port = common::spawn_tls_server();
    let proxy = common::spawn_connect_proxy();
    let ca = common::fixtures_dir().join("ca.pem");

    std::env::set_var("HTTPS_PROXY", format!("http://127.0.0.1:{}", proxy.port));
    std::env::set_var("TEBAKO_EXTRA_CA", &ca);

    // Covered by NO_PROXY → direct: the proxy sees nothing.
    std::env::set_var("NO_PROXY", "127.0.0.1");
    let cfg = NetworkConfig::from_env();
    let agent = tebako_http::agent_with_config(&cfg, TIMEOUT).unwrap();
    assert_eq!(common::fetch(&agent, server_port).unwrap(), common::BODY);
    assert!(
        proxy.connects.lock().unwrap().is_empty(),
        "a NO_PROXY-covered fetch must not touch the proxy"
    );

    // Not covered → the CONNECT rides the proxy.
    std::env::set_var("NO_PROXY", "example.com");
    let cfg = NetworkConfig::from_env();
    let agent = tebako_http::agent_with_config(&cfg, TIMEOUT).unwrap();
    assert_eq!(common::fetch(&agent, server_port).unwrap(), common::BODY);
    let seen = proxy.connects.lock().unwrap();
    assert!(
        seen.iter()
            .any(|t| t == &format!("127.0.0.1:{server_port}")),
        "an uncovered fetch must ride the proxy — seen: {seen:?}"
    );

    std::env::remove_var("HTTPS_PROXY");
    std::env::remove_var("NO_PROXY");
    std::env::remove_var("TEBAKO_EXTRA_CA");
}
