//! SHA-256 file hashing (FIPS 180-4; the C++ bootstrap ships the same
//! algorithm inline — we use the small `sha2` crate instead of a hand-roll).

use std::io::{self, Read};
use std::path::Path;

use sha2::Digest;

/// SHA-256 of a file as lowercase hex (64 chars).
pub fn sha256_file_hex(path: &Path) -> io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = sha2::Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}
