//! Shared fixture machinery for the enterprise-networking acceptance
//! tests (TODO.v2-1/33, spec 04 §4): a local rustls HTTPS server
//! presenting a certificate signed by a self-signed test CA, and a local
//! CONNECT proxy recording every CONNECT it carries (the proof a fetch
//! really rode the proxy — never a silent direct fallback).
//!
//! Assumes no ambient `NO_PROXY`/`no_proxy` covers 127.0.0.1 (true on
//! CI); the grammar itself is property-tested in `netconfig::tests`.
//!
//! The PEM fixtures were generated 2026-09-06 with OpenSSL 3.6 (CA:
//! 100-year validity; the server leaf is capped at 825 days — Apple's
//! notStandardCompliant policy (-67901) refuses longer-lived TLS server
//! certs even under a user-trusted root; the leaf expires ~2028-12,
//! regenerate then — SANs DNS:localhost + IP:127.0.0.1):
//!
//! ```sh
//! # ca.cnf: [req] distinguished_name=dn x509_extensions=v3_ca / [dn] /
//! #         [v3_ca] basicConstraints=critical,CA:TRUE
//! #         keyUsage=critical,keyCertSign,cRLSign subjectKeyIdentifier=hash
//! openssl req -x509 -newkey rsa:2048 -nodes -days 36500 -config ca.cnf \
//!   -subj "/CN=Tebako MITM Test CA" -keyout ca-key.pem -out ca.pem
//! openssl req -new -newkey rsa:2048 -nodes -subj "/CN=localhost" \
//!   -keyout server-key.pem -out server.csr
//! # server.ext: basicConstraints=critical,CA:FALSE
//! #   keyUsage=critical,digitalSignature,keyEncipherment
//! #   extendedKeyUsage=serverAuth subjectAltName=DNS:localhost,IP:127.0.0.1
//! openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem \
//!   -CAcreateserial -days 825 -extfile server.ext -out server.pem
//! ```
//!
//! The CA key is NOT checked in — regenerate the whole set if the
//! fixtures ever need to change.

#![allow(dead_code)] // each test binary uses a subset

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

pub const BODY: &str = "tebako-mitm-ok";

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"))
}

/// The fixture server's provider — the same platform split as the
/// shipped client (ring everywhere except windows-gnu's aws-lc-rs),
/// explicit because the unified test build carries one rustls with the
/// target's provider and auto-detection must never guess.
#[cfg(all(windows, target_env = "gnu"))]
fn crypto_provider() -> rustls::crypto::CryptoProvider {
    rustls::crypto::aws_lc_rs::default_provider()
}

#[cfg(not(all(windows, target_env = "gnu")))]
fn crypto_provider() -> rustls::crypto::CryptoProvider {
    rustls::crypto::ring::default_provider()
}

/// The TLS endpoint: one accept loop, one thread per connection, a fixed
/// 200 answer to any request. Returns the bound port.
pub fn spawn_tls_server() -> u16 {
    use rustls::pki_types::pem::PemObject;
    let dir = fixtures_dir();
    let certs: Vec<_> = rustls::pki_types::CertificateDer::pem_file_iter(dir.join("server.pem"))
        .expect("server.pem readable")
        .collect::<Result<_, _>>()
        .expect("server.pem parses");
    let key = rustls::pki_types::PrivateKeyDer::from_pem_file(dir.join("server-key.pem"))
        .expect("server-key.pem parses");
    let config = Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(crypto_provider()))
            .with_safe_default_protocol_versions()
            .expect("protocol versions")
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .expect("server config"),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tls server");
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || loop {
        let (stream, _) = match listener.accept() {
            Ok(x) => x,
            Err(_) => break,
        };
        let config = Arc::clone(&config);
        thread::spawn(move || handle_tls_conn(stream, config));
    });
    port
}

fn handle_tls_conn(mut stream: TcpStream, config: Arc<rustls::ServerConfig>) {
    let mut conn = match rustls::ServerConnection::new(config) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut tls = rustls::Stream::new(&mut conn, &mut stream);
    // Read the request head (bounded); the TLS handshake rides the first
    // read — a client that refuses the cert breaks here, which is fine.
    let mut head = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 {
        match tls.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    let resp = format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        BODY.len(),
        BODY
    );
    let _ = tls.write_all(resp.as_bytes());
}

/// The CONNECT proxy: cleartext CONNECT head, a recorded target, a 200,
/// then a byte tunnel. `connects` is the proof the fetch rode the proxy.
pub struct ProxyHandle {
    pub port: u16,
    pub connects: Arc<Mutex<Vec<String>>>,
}

pub fn spawn_connect_proxy() -> ProxyHandle {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy");
    let port = listener.local_addr().unwrap().port();
    let connects = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&connects);
    thread::spawn(move || loop {
        let (client, _) = match listener.accept() {
            Ok(x) => x,
            Err(_) => break,
        };
        let seen = Arc::clone(&seen);
        thread::spawn(move || handle_connect_conn(client, seen));
    });
    ProxyHandle { port, connects }
}

fn handle_connect_conn(mut client: TcpStream, seen: Arc<Mutex<Vec<String>>>) {
    let mut head = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 {
        match client.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head);
    let Some(target) = head.split_whitespace().nth(1) else {
        return;
    };
    seen.lock().unwrap().push(target.to_string());
    let mut upstream = match TcpStream::connect(target) {
        Ok(s) => s,
        Err(_) => return,
    };
    if client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .is_err()
    {
        return;
    }
    let (Ok(mut c2), Ok(mut u2)) = (client.try_clone(), upstream.try_clone()) else {
        return;
    };
    thread::spawn(move || {
        let _ = std::io::copy(&mut c2, &mut u2);
    });
    let _ = std::io::copy(&mut upstream, &mut client);
}

/// One GET through the given agent; Ok(body) / Err(Display of the ureq
/// error).
pub fn fetch(agent: &ureq::Agent, port: u16) -> Result<String, String> {
    let url = format!("https://127.0.0.1:{port}/");
    let mut resp = agent.get(&url).call().map_err(|e| format!("{e}"))?;
    let bytes = resp
        .body_mut()
        .with_config()
        .limit(1 << 20)
        .read_to_vec()
        .map_err(|e| format!("{e}"))?;
    String::from_utf8(bytes).map_err(|e| format!("{e}"))
}
