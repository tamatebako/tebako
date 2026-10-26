//! CA distrust policy — the eMudhra exclusion (2026-10 incident).
//!
//! eMudhra's intermediate CA private keys were generated with
//! recoverable entropy (one key is literally the integer 4). Root
//! programs blocked the eight compromised intermediates, and
//! webpki-roots 1.0.10 dropped `emSign ECC Root CA - G3` — but the
//! Mozilla set still carries other eMudhra roots (`emSign Root CA -
//! C1`, `emSign Root CA - G1`, `emSign ECC Root CA - C3`), and the
//! operator/platform/extra_ca arms can reintroduce any of them.
//!
//! tebako excludes the CA outright: every certificate whose subject or
//! issuer carries the eMudhra/emSign identity is refused a place in any
//! trust set this stack assembles. The check is intentionally lexical
//! (subject/issuer DN markers in the DER) rather than a root-store
//! snapshot — it survives re-issues, cross-signs, and the Microsoft-only
//! `emSign Root TLS/CS CA - G3` pair that never entered the Mozilla set.
//!
//! Scope: trust-store assembly ONLY (bundled roots, `extra_ca`, the
//! operator CA store, the driver's platform enumeration when it merges a
//! runtime cert bundle). A server-sent leaf is never filtered here —
//! that path is rustls-webpki's job (floored at 0.103.16, which carries
//! the compromised-intermediate SPKI blocklist).

/// Identity markers present in every eMudhra/emSign certificate's
/// subject (and, for the self-signed roots, issuer). Case-insensitive
/// ASCII over the DER: the names are IA5/PrintableString/UTF8String
/// encoded as plain ASCII inside the structure.
const IDENTITY_MARKERS: [&[u8]; 2] = [b"emudhra", b"emsign"];

/// True when `der` (a single X.509 certificate) belongs to the eMudhra
/// / emSign CA hierarchy and must never enter a tebako trust set.
pub fn is_distrusted(der: &[u8]) -> bool {
    IDENTITY_MARKERS
        .iter()
        .any(|marker| contains_ascii_ci(der, marker))
}

/// True when any certificate block in `pem` is distrusted. Used on the
/// whole-file inputs (`extra_ca`, operator-store PEMs) where a corporate
/// export may mix several blocks. The subject markers live inside the
/// DER — invisible in the PEM's base64 body — so every block must be
/// parsed and checked; there is no safe file-level pre-filter.
pub fn any_distrusted_pem(pem: &[u8]) -> bool {
    let mut cursor = std::io::Cursor::new(pem);
    for item in rustls_pemfile::certs(&mut cursor) {
        match item {
            Ok(der) if is_distrusted(&der) => return true,
            Ok(_) => {}
            // Unparseable blocks are the caller's named error (load
            // already fail-closes on them); a distrust check is not the
            // place to invent a second verdict.
            Err(_) => {}
        }
    }
    false
}

/// Case-insensitive ASCII substring search.
fn contains_ascii_ci(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

/// Drop every distrusted DER from an assembled root list (the bundled
/// Mozilla set, additive PEMs, platform enumeration). Returns the kept
/// certs and how many were dropped — callers journal the count; the
/// drop itself is never silent at the audit layer.
pub fn retain_trusted(der_certs: Vec<Vec<u8>>) -> (Vec<Vec<u8>>, usize) {
    let total = der_certs.len();
    let kept: Vec<Vec<u8>> = der_certs
        .into_iter()
        .filter(|c| !is_distrusted(c))
        .collect();
    let dropped = total - kept.len();
    (kept, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real self-signed test root (same fixture as netconfig's).
    const TRUSTED_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
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

    /// Real emSign root subject (emSign Root CA - C1 identity) rendered
    /// as a DER fragment the lexical check must catch. Not a valid
    /// certificate — `is_distrusted` is a lexical identity check over
    /// the DER, so a subject-bearing blob is exactly the unit under test.
    const EMUDHRA_SUBJECT_BLOB: &[u8] = b"0\x131\x110\x0f\x06\x03U\x04\x0a\x13\x08eMudhra Inc";

    const EMSIGN_SUBJECT_BLOB: &[u8] =
        b"0\x1c1\x1a0\x18\x06\x03U\x04\x03\x13\x11emSign Root CA - C1";

    #[test]
    fn a_trusted_certificate_is_not_distrusted() {
        let der = extract_first_pem(TRUSTED_PEM);
        assert!(!is_distrusted(&der));
        assert!(!any_distrusted_pem(TRUSTED_PEM.as_bytes()));
    }

    #[test]
    fn the_emudhra_organization_marker_distrusts() {
        assert!(is_distrusted(EMUDHRA_SUBJECT_BLOB));
    }

    #[test]
    fn the_emsign_common_name_marker_distrusts() {
        assert!(is_distrusted(EMSIGN_SUBJECT_BLOB));
    }

    #[test]
    fn the_markers_match_case_insensitively() {
        assert!(is_distrusted(b"....EMUDHRA...."));
        assert!(is_distrusted(b"....EMSIGN...."));
        assert!(is_distrusted(b"....emsign...."));
    }

    #[test]
    fn an_unrelated_certificate_is_not_distrusted() {
        assert!(!is_distrusted(b"CN=DigiCert, O=DigiCert Inc"));
    }

    #[test]
    fn retain_trusted_drops_the_distrusted_blocks() {
        let trusted = extract_first_pem(TRUSTED_PEM);
        let (kept, dropped) = retain_trusted(vec![trusted, EMUDHRA_SUBJECT_BLOB.to_vec()]);
        assert_eq!(dropped, 1);
        assert_eq!(kept.len(), 1);
    }

    /// Real compromised eMudhra intermediate (EM DV TLS CA - G3B,
    /// 2026-10 incident — the private key was recoverable). Subject
    /// `O=eMudhra BV`; used as a live PEM fixture for the file-level
    /// check.
    const EMUDHRA_INTERMEDIATE_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIDaDCCAu6gAwIBAgIQAKSu5njyM2K0wORgt3LyFDAKBggqhkjOPQQDAzBWMQsw\n\
CQYDVQQGEwJJTjElMCMGA1UECgwcZU11ZGhyYSBUZWNobm9sb2dpZXMgTGltaXRl\n\
ZDEgMB4GA1UEAwwXZW1TaWduIFJvb3QgVExTIENBIC0gRzMwHhcNMjUwNDMwMTU1\n\
NDA4WhcNNDAwNDI4MTgzMDAwWjA/MQswCQYDVQQGEwJOTDETMBEGA1UECgwKZU11\n\
ZGhyYSBCVjEbMBkGA1UEAwwSRU0gRFYgVExTIENBIC0gRzNCMHYwEAYHKoZIzj0C\n\
AQYFK4EEACIDYgAE557ZwEqXc8RzXlERHahawr1eZKs3S4DhLLwgOOweTw47gS/J\n\
hZ4eZ6+wTi+LyX90hbplqpEe6KTkW2LVzAPVa+Z4s8OMtNyTfKJGDiOEU8iSk10p\n\
1E+R4VYGtjzO2rRoo4IBljCCAZIwHwYDVR0jBBgwFoAU4TqrIXzx401UrtRyt/2L\n\
WUIgkFQwHQYDVR0OBBYEFOIeL+ccGTkOVUtiwSAkVzUkiiUUMB0GA1UdJQQWMBQG\n\
CCsGAQUFBwMBBggrBgEFBQcDAjBdBgNVHSAEVjBUMDkGCysGAQQBg44hAQABMCow\n\
KAYIKwYBBQUHAgEWHGh0dHA6Ly9yZXBvc2l0b3J5LmVtc2lnbi5jb20wDQYLKwYB\n\
BAGDjiEBAmQwCAYGZ4EMAQIBMHUGCCsGAQUFBwEBBGkwZzAkBggrBgEFBQcwAYYY\n\
aHR0cDovL29jc3AtYi5lbXNpZ24uY29tMD8GCCsGAQUFBzAChjNodHRwOi8vcmVw\n\
b3NpdG9yeS5lbXNpZ24uY29tL2NlcnRzL0VNRFZUTFNDQUczQi5wN2MwNwYDVR0f\n\
BDAwLjAsoCqgKIYmaHR0cDovL2NybC5lbXNpZ24uY29tLz9yb290dGxzY2FnMy5j\n\
cmwwDgYDVR0PAQH/BAQDAgEGMBIGA1UdEwEB/wQIMAYBAf8CAQAwCgYIKoZIzj0E\n\
AwMDaAAwZQIwLmsGXi6oFMJ2klkcEMXMug6gvxQWZWkewiDWVzQyl2L6z6DuQfPO\n\
Ibh1grvdULDCAjEAxZvLlxUavG69J6wnSjC8XBbqNIQsJ6CE2QdNOe84Ptxb6VG8\n\
gt2TlfJbHii+KNvK\n\
-----END CERTIFICATE-----\n";

    #[test]
    fn a_pem_file_carrying_an_emudhra_block_is_flagged() {
        assert!(!any_distrusted_pem(TRUSTED_PEM.as_bytes()));
        assert!(any_distrusted_pem(EMUDHRA_INTERMEDIATE_PEM.as_bytes()));
        // Mixed export: trusted root + the compromised intermediate.
        let mut mixed = TRUSTED_PEM.as_bytes().to_vec();
        mixed.extend_from_slice(EMUDHRA_INTERMEDIATE_PEM.as_bytes());
        assert!(any_distrusted_pem(&mixed));
    }

    #[test]
    fn the_live_emudhra_intermediate_is_distrusted() {
        let der = extract_first_pem(EMUDHRA_INTERMEDIATE_PEM);
        assert!(is_distrusted(&der));
    }

    /// First `-----BEGIN CERTIFICATE-----` block, DER bytes.
    fn extract_first_pem(pem: &str) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(pem.as_bytes());
        let mut certs = rustls_pemfile::certs(&mut cursor);
        let der = certs
            .next()
            .expect("fixture has one cert")
            .expect("fixture parses")
            .to_vec();
        der
    }
}
