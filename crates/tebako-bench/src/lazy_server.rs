//! The lazy arm's loopback Range fixture (spec 27 §10.5): an in-process
//! `127.0.0.1` HTTP server serving the acquired env image (whole GET +
//! `Range:`) and its blksum sidecar — the spec 38 §8 carve-out spelling
//! of the distribution surface, keeping WAN variance out of the lazy
//! measurement. The server lives for the whole leg (the seal thread's
//! between-runs fetches ride it too) and shuts down on drop.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::BenchError;

/// A bound, serving loopback fixture. `port` is the OS-assigned listen
/// port the seed descriptor's `source` was written against.
pub struct LazyServer {
    pub port: u16,
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for LazyServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyServer")
            .field("port", &self.port)
            .finish()
    }
}

impl Drop for LazyServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        // Wake the accept loop out of its blocking accept so the join
        // below returns promptly.
        let _ = std::net::TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

impl LazyServer {
    /// Bind `127.0.0.1:0` and serve: `GET /<image_name>` (whole or
    /// `Range: bytes=<start>-[<end>]` → 206) from `image`, and
    /// `GET /<image_name>.blksum.json` → 200 with `sidecar`. Anything
    /// else is a 404 (the driver only ever asks for these two).
    pub fn start(image: PathBuf, sidecar: Vec<u8>) -> Result<LazyServer, BenchError> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| {
            BenchError::operational(format!("lazy-server: cannot bind 127.0.0.1:0: {e}"))
        })?;
        let port = listener
            .local_addr()
            .map_err(|e| BenchError::operational(format!("lazy-server: local_addr failed: {e}")))?
            .port();
        let image_name = image
            .file_name()
            .ok_or_else(|| {
                BenchError::operational(format!(
                    "lazy-server: image path {} has no file name",
                    image.display()
                ))
            })?
            .to_string_lossy()
            .into_owned();
        let shutdown = Arc::new(AtomicBool::new(false));
        let flag = shutdown.clone();
        let accept_thread = std::thread::spawn(move || {
            let state = Arc::new((image, image_name, sidecar));
            while !flag.load(Ordering::Relaxed) {
                let (stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(_) => break,
                };
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                let state = state.clone();
                std::thread::spawn(move || serve_connection(stream, state));
            }
        });
        Ok(LazyServer {
            port,
            shutdown,
            accept_thread: Some(accept_thread),
        })
    }
}

/// One connection: a loop of simple GET exchanges (tebako-http's agent
/// pools connections — `Connection: close` or EOF ends the loop).
fn serve_connection(
    mut stream: std::net::TcpStream,
    state: Arc<(PathBuf, String, Vec<u8>)>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut pending: Vec<u8> = Vec::new();
    loop {
        let Some(request) = read_request(&mut stream, &mut pending) else {
            return;
        };
        let close = request
            .lines()
            .any(|l| l.eq_ignore_ascii_case("connection: close"));
        let mut parts = request.split_whitespace();
        let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        let range = request
            .lines()
            .find_map(|l| l.strip_prefix("Range:").or_else(|| l.strip_prefix("range:")))
            .and_then(|v| parse_bytes_range(v.trim()));
        let response = if method != "GET" {
            (405, "Method Not Allowed".to_string(), Vec::new())
        } else if path == format!("/{}", state.1) {
            serve_image(&state.0, range)
        } else if path == format!("/{}.blksum.json", state.1) {
            (200, "OK".to_string(), state.2.clone())
        } else {
            (404, "Not Found".to_string(), Vec::new())
        };
        let keep = !close;
        if write_response(&mut stream, response, keep).is_err() || !keep {
            return;
        }
    }
}

/// Read one request head (through the blank line); `pending` carries
/// any bytes already read past the previous head. Bodies never ride
/// these GETs.
fn read_request(stream: &mut std::net::TcpStream, pending: &mut Vec<u8>) -> Option<String> {
    let mut buf = [0u8; 4096];
    loop {
        if let Some(at) = find_subsequence(pending, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&pending[..at]).into_owned();
            pending.drain(..at + 4);
            return Some(head);
        }
        match stream.read(&mut buf) {
            Ok(0) => return None,
            Ok(n) => pending.extend_from_slice(&buf[..n]),
            Err(_) => return None,
        }
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// `bytes=<start>-` / `bytes=<start>-<end>` → `(start, inclusive end)`.
/// Suffix ranges never ride this surface (tebako-http's range GET
/// always sends a bounded window).
fn parse_bytes_range(value: &str) -> Option<(u64, Option<u64>)> {
    let spec = value.strip_prefix("bytes=")?;
    let (start, end) = spec.split_once('-')?;
    let start = start.trim().parse().ok()?;
    let end = end.trim();
    let end = if end.is_empty() {
        None
    } else {
        Some(end.parse().ok()?)
    };
    Some((start, end))
}

/// The image exchange: a whole GET is the 200, a satisfiable range the
/// 206 with a well-formed `Content-Range` (the byte source validates
/// the window against its request — spec 39 §8's answer law).
fn serve_image(path: &std::path::Path, range: Option<(u64, Option<u64>)>) -> (u16, String, Vec<u8>) {
    let Ok(bytes) = std::fs::read(path) else {
        return (500, "Internal Server Error".to_string(), Vec::new());
    };
    match range {
        None => (200, "OK".to_string(), bytes),
        Some((start, end)) => {
            let len = bytes.len() as u64;
            if start >= len {
                return (
                    416,
                    format!("Range Not Satisfiable|Content-Range: bytes */{len}"),
                    Vec::new(),
                );
            }
            let end = end.unwrap_or(len - 1).min(len - 1);
            let window = bytes[start as usize..=(end as usize)].to_vec();
            (
                206,
                format!("Partial Content|Content-Range: bytes {start}-{end}/{len}"),
                window,
            )
        }
    }
}

/// Status line + headers + body. Extra headers smuggle inside `reason`
/// after a `|` (this fixture needs exactly one: Content-Range).
fn write_response(
    stream: &mut std::net::TcpStream,
    (status, reason, body): (u16, String, Vec<u8>),
    keep_alive: bool,
) -> std::io::Result<()> {
    let (reason, extra) = match reason.split_once('|') {
        Some((reason, extra)) => (reason, format!("{extra}\r\n")),
        None => (reason.as_str(), String::new()),
    };
    let connection = if keep_alive { "keep-alive" } else { "close" };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: {connection}\r\n{extra}\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_grammar_parses_the_bounded_and_open_forms() {
        assert_eq!(parse_bytes_range("bytes=0-4194303"), Some((0, Some(4194303))));
        assert_eq!(parse_bytes_range("bytes=4096-"), Some((4096, None)));
        assert_eq!(parse_bytes_range("items=0-10"), None);
        assert_eq!(parse_bytes_range("bytes=-500"), None);
    }
}
