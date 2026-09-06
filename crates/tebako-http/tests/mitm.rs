//! The MITM acceptance fixture (TODO.v2-1/33, spec 04 §4): a local
//! rustls server presents a certificate signed by a self-signed test CA,
//! reached THROUGH a local CONNECT proxy. The loader's client must:
//!   - fail closed on the default (bundled webpki) roots;
//!   - accept with `extra_ca` pointing at the CA PEM (additive roots —
//!     the bundled store PLUS the corporate root, never instead of it);
//!   - accept with `tls_roots: platform` when the CA is in the OS store
//!     (best-effort install; a named SKIP where the store isn't
//!     writable — the acceptance's own escape hatch);
//!   - record every decision in the config's audit lines, and ride the
//!     proxy observably (the proxy's CONNECT log asserts it).

mod common;

use std::path::Path;
use std::time::Duration;

use tebako_http::netconfig::{NetworkConfig, TlsRoots};

const TIMEOUT: Duration = Duration::from_secs(10);

fn proxy_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

fn assert_rode_the_proxy(proxy: &common::ProxyHandle, server_port: u16) {
    let seen = proxy.connects.lock().unwrap();
    assert!(
        seen.iter()
            .any(|t| t == &format!("127.0.0.1:{server_port}")),
        "the CONNECT must ride the proxy — seen: {seen:?}"
    );
}

#[test]
fn default_roots_refuse_the_mitm_cert() {
    let server_port = common::spawn_tls_server();
    let proxy = common::spawn_connect_proxy();
    let cfg = NetworkConfig::default().merge_file(
        Some(proxy_url(proxy.port)),
        None,
        vec![],
        Path::new("mitm-fixture"),
    );
    assert!(
        cfg.audit
            .iter()
            .any(|l| l.starts_with("proxy=http://127.0.0.1:")),
        "audit: {:?}",
        cfg.audit
    );
    let agent = tebako_http::agent_with_config(&cfg, TIMEOUT).unwrap();
    let err = common::fetch(&agent, server_port).unwrap_err();
    assert!(
        err.contains("certificate"),
        "the failure must be the certificate, not the transport: {err}"
    );
    assert_rode_the_proxy(&proxy, server_port);
}

#[test]
fn extra_ca_roots_accept_the_mitm_cert() {
    let server_port = common::spawn_tls_server();
    let proxy = common::spawn_connect_proxy();
    let ca = common::fixtures_dir().join("ca.pem");
    let cfg = NetworkConfig::default().merge_file(
        Some(proxy_url(proxy.port)),
        None,
        vec![ca],
        Path::new("mitm-fixture"),
    );
    assert!(
        cfg.audit
            .iter()
            .any(|l| l.starts_with("extra_ca=1 file(s)")),
        "audit: {:?}",
        cfg.audit
    );
    let agent = tebako_http::agent_with_config(&cfg, TIMEOUT).unwrap();
    assert_eq!(common::fetch(&agent, server_port).unwrap(), common::BODY);
    assert_rode_the_proxy(&proxy, server_port);
}

#[test]
fn platform_roots_accept_the_mitm_cert_when_the_ca_is_installed() {
    let server_port = common::spawn_tls_server();
    let proxy = common::spawn_connect_proxy();
    let ca = common::fixtures_dir().join("ca.pem");
    let _install = match OsStoreInstall::install(&ca) {
        Ok(g) => g,
        Err(why) => {
            eprintln!("SKIP platform-roots leg: {why}");
            return;
        }
    };
    let cfg = NetworkConfig::default().merge_file(
        Some(proxy_url(proxy.port)),
        Some(TlsRoots::Platform),
        vec![],
        Path::new("mitm-fixture"),
    );
    let agent = tebako_http::agent_with_config(&cfg, TIMEOUT).unwrap();
    assert_eq!(common::fetch(&agent, server_port).unwrap(), common::BODY);
    assert_rode_the_proxy(&proxy, server_port);
}

/// Best-effort install of the test CA into the OS trust store, removed
/// on drop. Test-only shell-outs (the no-shell-out law binds shipped
/// artifacts, not fixtures); any failure is a named skip, never a false
/// green.
struct OsStoreInstall {
    undo: Vec<Vec<String>>,
}

impl OsStoreInstall {
    fn install(ca: &Path) -> Result<Self, String> {
        let ca = ca.canonicalize().map_err(|e| e.to_string())?;
        let (install, undo): (Vec<Vec<String>>, Vec<Vec<String>>) = if cfg!(target_os = "macos") {
            let keychain = format!(
                "{}/Library/Keychains/login.keychain-db",
                std::env::var("HOME").map_err(|e| e.to_string())?
            );
            (
                vec![vec![
                    "security".into(),
                    "add-trusted-cert".into(),
                    "-r".into(),
                    "trustRoot".into(),
                    "-k".into(),
                    keychain,
                    ca.to_string_lossy().into_owned(),
                ]],
                vec![vec![
                    "security".into(),
                    "delete-certificate".into(),
                    "-c".into(),
                    "Tebako MITM Test CA".into(),
                ]],
            )
        } else if cfg!(target_os = "linux") {
            let dest = "/usr/local/share/ca-certificates/tebako-mitm-test.crt";
            (
                vec![
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "cp".into(),
                        ca.to_string_lossy().into_owned(),
                        dest.into(),
                    ],
                    vec!["sudo".into(), "-n".into(), "update-ca-certificates".into()],
                ],
                vec![
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "rm".into(),
                        "-f".into(),
                        dest.into(),
                    ],
                    vec!["sudo".into(), "-n".into(), "update-ca-certificates".into()],
                ],
            )
        } else if cfg!(target_os = "windows") {
            (
                vec![vec![
                    "certutil".into(),
                    "-user".into(),
                    "-addstore".into(),
                    "Root".into(),
                    ca.to_string_lossy().into_owned(),
                ]],
                vec![vec![
                    "certutil".into(),
                    "-user".into(),
                    "-delstore".into(),
                    "Root".into(),
                    "Tebako MITM Test CA".into(),
                ]],
            )
        } else {
            return Err("no OS-store installer for this platform".to_string());
        };
        for cmd in &install {
            Self::run(cmd)?;
        }
        Ok(Self { undo })
    }

    /// Bounded, never-interactive: a trust-store tool that wants a GUI
    /// authorization (a locked keychain on a headless runner) must become
    /// a named skip, not a hung CI leg (the v1 of this fixture hung
    /// macos-14 + windows-latest for 4 h on exactly that).
    fn run(argv: &[String]) -> Result<(), String> {
        let mut child = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("{}: {e}", argv[0]))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stderr = String::new();
                    if let Some(mut pipe) = child.stderr.take() {
                        use std::io::Read as _;
                        let _ = pipe.read_to_string(&mut stderr);
                    }
                    return if status.success() {
                        Ok(())
                    } else {
                        Err(format!("{}: status {status} — {stderr}", argv[0]))
                    };
                }
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "{}: timed out after 20 s (a locked keychain / interactive \
                         authorization prompt reads as a skip here)",
                        argv[0]
                    ));
                }
                Err(e) => return Err(format!("{}: {e}", argv[0])),
            }
        }
    }
}

impl Drop for OsStoreInstall {
    fn drop(&mut self) {
        for cmd in &self.undo {
            let _ = Self::run(cmd);
        }
    }
}
