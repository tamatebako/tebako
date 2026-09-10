//! Root-of-trust rotation: the successor-key statement.
//!
//! A successor statement binds the CURRENT root fingerprint to its
//! SUCCESSOR, signed by the current root key. Machines that trust the
//! current root can forward their trust through a chain of such
//! statements without any out-of-band step (item 29's rotation story):
//! starting from the embedded root fingerprint, each verified statement
//! moves the trusted root to its successor.
//!
//! Wire format (one file, canonical):
//!
//! ```text
//! -----BEGIN TEBAKO SUCCESSOR STATEMENT-----
//! format: TEBAKO-ROOT-SUCCESSOR-V1
//! predecessor: <40-hex fingerprint>
//! successor: <40-hex fingerprint>
//! created: <unix seconds>
//! -----BEGIN PGP SIGNATURE-----
//! <armored detached OpenPGP signature over the four body lines>
//! ```
//!
//! The successor's PUBLIC key is distributed through the normal public
//! channel (tebako.org, keyring updates); the statement only proves the
//! rotation is authorized, so the channel it travels by does not matter.

use std::fmt::Write as _;

use crate::error::SignerError;
use crate::keys::keyid_bytes_from_fingerprint;
use crate::sign::{sign_detached, verify_detached, VerifyOutcome};

/// Statement format identifier.
pub const STATEMENT_FORMAT: &str = "TEBAKO-ROOT-SUCCESSOR-V1";
const BEGIN_STATEMENT: &str = "-----BEGIN TEBAKO SUCCESSOR STATEMENT-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN PGP SIGNATURE-----";

/// A parsed successor statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessorStatement {
    /// The current (signing) root fingerprint this statement rotates FROM.
    pub predecessor_fingerprint: String,
    /// The successor root fingerprint this statement rotates TO.
    pub successor_fingerprint: String,
    /// Statement creation time (unix seconds, informational).
    pub created_unix: u64,
}

fn canonical_body(
    predecessor_fingerprint: &str,
    successor_fingerprint: &str,
    created_unix: u64,
) -> Vec<u8> {
    format!(
        "format: {STATEMENT_FORMAT}\npredecessor: {predecessor_fingerprint}\nsuccessor: {successor_fingerprint}\ncreated: {created_unix}\n"
    )
    .into_bytes()
}

fn check_fingerprint(fp: &str, what: &str) -> Result<(), SignerError> {
    let ok = fp.len() == 40 && fp.bytes().all(|b| b.is_ascii_hexdigit());
    if ok {
        Ok(())
    } else {
        Err(SignerError::Sign(format!(
            "invalid {what} fingerprint (want 40 hex chars): {fp}"
        )))
    }
}

/// Produce a successor statement: the current root (identified by
/// `predecessor_fingerprint`, whose secret key signs) certifies
/// `successor_fingerprint` as the next root.
pub fn sign_successor_statement(
    secret_key: &[u8],
    predecessor_fingerprint: &str,
    successor_fingerprint: &str,
) -> Result<Vec<u8>, SignerError> {
    check_fingerprint(predecessor_fingerprint, "predecessor")?;
    check_fingerprint(successor_fingerprint, "successor")?;

    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body = canonical_body(predecessor_fingerprint, successor_fingerprint, created);
    let signature = sign_detached(&body, secret_key, predecessor_fingerprint)?;
    let signature = rnp::armor_bytes(&signature, rnp::ops::ArmorType::Signature)
        .map_err(|e| SignerError::Sign(e.to_string()))?;

    let mut out = Vec::new();
    out.extend_from_slice(BEGIN_STATEMENT.as_bytes());
    out.push(b'\n');
    out.extend_from_slice(&body);
    out.extend_from_slice(BEGIN_SIGNATURE.as_bytes());
    out.push(b'\n');
    out.extend_from_slice(&signature);
    Ok(out)
}

/// Parse a successor statement (format, fields, dearmored signature) —
/// no trust evaluation. Returns the statement, the canonical body bytes,
/// and the dearmored detached signature.
pub fn parse_successor_statement(
    statement: &[u8],
) -> Result<(SuccessorStatement, Vec<u8>, Vec<u8>), SignerError> {
    let text = std::str::from_utf8(statement)
        .map_err(|_| SignerError::Verify("successor statement is not UTF-8".into()))?;
    let Some(begin) = text.find(BEGIN_STATEMENT) else {
        return Err(SignerError::Verify(
            "no TEBAKO SUCCESSOR STATEMENT marker".into(),
        ));
    };
    let body_start = begin + BEGIN_STATEMENT.len();
    let Some(sig_start) = text.find(BEGIN_SIGNATURE) else {
        return Err(SignerError::Verify("no PGP SIGNATURE marker".into()));
    };
    if sig_start <= body_start {
        return Err(SignerError::Verify("malformed successor statement".into()));
    }
    let body = &text[body_start..sig_start];
    let body = body.strip_prefix('\n').unwrap_or(body);
    let signature = &text[sig_start + BEGIN_SIGNATURE.len()..];
    let signature = signature.as_bytes();

    let mut format = "";
    let mut predecessor = "";
    let mut successor = "";
    let mut created = 0u64;
    for line in body.lines() {
        if let Some(v) = line.strip_prefix("format: ") {
            format = v.trim();
        } else if let Some(v) = line.strip_prefix("predecessor: ") {
            predecessor = v.trim();
        } else if let Some(v) = line.strip_prefix("successor: ") {
            successor = v.trim();
        } else if let Some(v) = line.strip_prefix("created: ") {
            created = v.trim().parse().unwrap_or(0);
        }
    }
    if format != STATEMENT_FORMAT {
        return Err(SignerError::Verify(format!(
            "unsupported successor statement format: {format:?}"
        )));
    }
    check_fingerprint(predecessor, "predecessor")
        .map_err(|e| SignerError::Verify(e.to_string()))?;
    check_fingerprint(successor, "successor").map_err(|e| SignerError::Verify(e.to_string()))?;

    let signature = rnp::dearmor_bytes(signature)
        .map_err(|e| SignerError::Verify(format!("signature block does not dearmor: {e}")))?;

    Ok((
        SuccessorStatement {
            predecessor_fingerprint: predecessor.to_uppercase(),
            successor_fingerprint: successor.to_uppercase(),
            created_unix: created,
        },
        body.as_bytes().to_vec(),
        signature,
    ))
}

/// Parse + verify a successor statement against the trusted keyring.
/// Returns the statement and the signature's trust classification
/// ([`VerifyOutcome::Trusted`] when the predecessor's key is trusted and
/// the signature is valid).
pub fn verify_successor_statement(
    trusted_keyring: &[u8],
    statement: &[u8],
) -> Result<(SuccessorStatement, VerifyOutcome), SignerError> {
    let (stmt, body, signature) = parse_successor_statement(statement)?;
    let keyid = keyid_bytes_from_fingerprint(&stmt.predecessor_fingerprint)?;
    let outcome = verify_detached(trusted_keyring, &body, &signature, &keyid)?;
    Ok((stmt, outcome))
}

/// Evaluate a rotation chain: starting at `root_fingerprint`, walk the
/// statements by matching each hop's predecessor to the current root —
/// statements arrive in arbitrary order (directory scans, filename
/// sorting), so positional order is never assumed. The signature is
/// verified only for the statement taken at each hop; every statement is
/// consumed at most once, so gaps, misorderings, and cycles all
/// terminate in a named error. STRICT: any unclaimable or unverifiable
/// statement fails the whole chain. Returns the final root fingerprint
/// after the last verified rotation.
pub fn apply_successor_chain(
    root_fingerprint: &str,
    trusted_keyring: &[u8],
    statements: &[Vec<u8>],
) -> Result<String, SignerError> {
    let mut current = root_fingerprint.to_uppercase();
    check_fingerprint(&current, "root")?;
    let mut pending: Vec<(SuccessorStatement, Vec<u8>, Vec<u8>)> = statements
        .iter()
        .map(|s| parse_successor_statement(s))
        .collect::<Result<_, _>>()?;
    while !pending.is_empty() {
        let Some(idx) = pending
            .iter()
            .position(|(stmt, _, _)| stmt.predecessor_fingerprint == current)
        else {
            let available: Vec<&str> = pending
                .iter()
                .map(|(stmt, _, _)| stmt.predecessor_fingerprint.as_str())
                .collect();
            return Err(SignerError::Verify(format!(
                "successor chain broken: predecessors {available:?} does not match the current root {current}"
            )));
        };
        let (stmt, body, signature) = pending.swap_remove(idx);
        let keyid = keyid_bytes_from_fingerprint(&stmt.predecessor_fingerprint)?;
        match verify_detached(trusted_keyring, &body, &signature, &keyid)? {
            VerifyOutcome::Trusted(_) => {
                current = stmt.successor_fingerprint;
            }
            VerifyOutcome::Untrusted(keyid) => {
                return Err(SignerError::Trust(format!(
                    "successor statement signed by an untrusted key {keyid}"
                )));
            }
            VerifyOutcome::Invalid(_) => {
                return Err(SignerError::Verify(
                    "successor statement has an invalid signature".into(),
                ));
            }
        }
    }
    Ok(current)
}

/// Walk the rotation chain from `root_fingerprint` as far as it VERIFIES,
/// returning the verified trust path `[root, …, furthest reachable]`.
/// TOLERANT: the walk stops at the first hop that is unclaimed,
/// untrusted, or invalid — anything beyond a broken link is unreachable
/// anyway, and every fingerprint in the returned path was reached through
/// verified links only. Statements arrive in arbitrary order; each is
/// consumed at most once. This is the membership oracle for consumers
/// whose signer may be ANY link of the chain, not only its tip.
pub fn successor_chain_path(
    root_fingerprint: &str,
    trusted_keyring: &[u8],
    statements: &[Vec<u8>],
) -> Vec<String> {
    let mut current = root_fingerprint.to_uppercase();
    let mut path = vec![current.clone()];
    let mut pending: Vec<(SuccessorStatement, Vec<u8>, Vec<u8>)> = statements
        .iter()
        .map(|s| parse_successor_statement(s))
        .collect::<Result<_, _>>()
        .unwrap_or_default();
    while let Some(idx) = pending
        .iter()
        .position(|(stmt, _, _)| stmt.predecessor_fingerprint == current)
    {
        let (stmt, body, signature) = pending.swap_remove(idx);
        let Ok(keyid) = keyid_bytes_from_fingerprint(&stmt.predecessor_fingerprint) else {
            break;
        };
        let Ok(outcome) = verify_detached(trusted_keyring, &body, &signature, &keyid) else {
            break;
        };
        match outcome {
            VerifyOutcome::Trusted(_) => {
                current = stmt.successor_fingerprint;
                path.push(current.clone());
            }
            VerifyOutcome::Untrusted(_) | VerifyOutcome::Invalid(_) => break,
        }
    }
    path
}

/// Format a fingerprint for display (first 8 + last 16 hex chars).
pub fn short_fingerprint(fp: &str) -> String {
    if fp.len() > 24 {
        let mut s = String::new();
        let _ = write!(s, "{}…{}", &fp[..8], &fp[fp.len() - 16..]);
        s
    } else {
        fp.to_string()
    }
}

// ---------------------------------------------------------------------
// The embedded first-party root (spec 09 §2 + §9)
// ---------------------------------------------------------------------

/// The tamatebako release root fingerprint (spec 09 §2) — the SSOT for
/// the value the bootstrap carries as `EMBEDDED_ROOT_FINGERPRINT`. The
/// bootstrap keeps its own literal (it builds WITHOUT this crate when
/// `openpgp-verify` is off); tebako-cli's tests assert the two agree.
pub const ROOT_FINGERPRINT: &str = "9E210CA8E9FDE9E6587740B2EFC3C250F7862A48";

/// The armored root public key (the classical Ed25519 primary plus its
/// signing and encryption subkeys) — byte-identical with
/// `https://www.tebako.org/.well-known/tebako-key.asc`. The CLI folds
/// this into every payload-signature verification (spec 09 §9:
/// first-party verification is zero-interaction — the key is embedded,
/// never fetched). The size-gated bootstrap stays fingerprint-only.
pub const ROOT_PUBLIC_KEY: &str = r#"-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEaqEttxYJKwYBBAHaRw8BAQdAQ9aJGss4ei8aArTRLHeUtOgAz1zlnw+JRu8Y
6HuGRcO0LXRhbWF0ZWJha28gcm9vdCAoY2xhc3NpY2FsKSA8cm9vdEB0ZWJha28u
b3JnPoivBBMWCgBXFiEEniEMqOn96eZYd0Cy78PCUPeGKkgFAmqhLbcbFIAAAAAA
BAAObWFudTIsMi41KzEuMTIsMCwzAhsBBQsJCAcCAiICBhUKCQgLAgQWAgMBAh4H
AheAAAoJEO/DwlD3hipIH1IA/j/VFOmgaiPQMVtioApA2MHywksYwEm+PTwlNGwB
QbGiAP41hDmHUQXhOKUFshzxjsA03R3eDwu8QoB6qtc71tY1DrgzBGqhLfsWCSsG
AQQB2kcPAQEHQJnJ/w917M45IsuTicHF8AUuoRlOS1lOG+NoLRZfpVn2iQELBBgW
CgA8FiEEniEMqOn96eZYd0Cy78PCUPeGKkgFAmqhLfsbFIAAAAAABAAObWFudTIs
Mi41KzEuMTIsMCwzAhsCAIEJEO/DwlD3hipIdiAEGRYKAB0WIQTkVZIau8Pn2blz
K/56Q31zgqPPWgUCaqEt+wAKCRB6Q31zgqPPWnXiAQC22wIPOaGgGuBvAytHmEwO
Phvhel6PktQUND+DYDcapAEA3GqCPkmH2xa9pFiuP9nWb15cmsQh2jFuuWB6OB6G
hQStsAD/SCuSbO/S9u0J4ubcBBAPZ0irYNHUWYMvBi0I47W0SbwA/RAcmoAyfDzr
wvm1aEaAkxZRXK4PsQw/u/RLwDPMpLwJuDgEaqEt/RIKKwYBBAGXVQEFAQEHQL+M
G3+rmdDRuAejNTXPYcHdg+Gk/Gvy0anBmhrjMzo2AwEIB4iUBBgWCgA8FiEEniEM
qOn96eZYd0Cy78PCUPeGKkgFAmqhLf0bFIAAAAAABAAObWFudTIsMi41KzEuMTIs
MCwzAhsMAAoJEO/DwlD3hipIW04A/303fBIF0M+48T9tzzTz/hXyqpzPgNxZV1JJ
6Yv2Bpg5AQC29iNf9hj7wp93ZMzIDOVyUFhtk5NGTwT5G66NxP/UCw==
=jH00
-----END PGP PUBLIC KEY BLOCK-----
"#;

/// The keyring a payload-signature verification runs against (spec 09
/// §9's zero-interaction rule for first-party artifacts): the user's
/// trusted keyring, the embedded root public key, and the
/// `TEBAKO_TRUSTED_ROOT` dev override's bundled key when it names a file.
pub fn verification_keyring(home: &std::path::Path) -> Result<Vec<u8>, SignerError> {
    let mut ring = crate::keyring::trusted_keyring_bytes(home)?;
    // The embedded const is ours and the tests prove it dearmors; a
    // failure here can only be memory corruption — skip defensively,
    // never fail an install over it.
    if let Ok(root) = rnp::dearmor_bytes(ROOT_PUBLIC_KEY.as_bytes()) {
        ring.extend_from_slice(&root);
    }
    if let Some(extra) = trusted_root_override_key(std::env::var("TEBAKO_TRUSTED_ROOT").ok()) {
        ring.extend_from_slice(&extra);
    }
    Ok(ring)
}

/// The `TEBAKO_TRUSTED_ROOT` dev override's bundled public key: when the
/// value names an existing file, its (possibly armored) bytes join the
/// verification keyring; a bare fingerprint names a key the trusted
/// keyring must already hold (the bootstrap's grammar, mirrored).
pub fn trusted_root_override_key(value: Option<String>) -> Option<Vec<u8>> {
    let v = value?;
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    let path = std::path::Path::new(v);
    if !path.is_file() {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(rnp::dearmor_bytes(&bytes).unwrap_or(bytes))
}
