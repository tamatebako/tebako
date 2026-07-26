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

/// Parse + verify a successor statement against the trusted keyring.
/// Returns the statement and the signature's trust classification
/// ([`VerifyOutcome::Trusted`] when the predecessor's key is trusted and
/// the signature is valid).
pub fn verify_successor_statement(
    trusted_keyring: &[u8],
    statement: &[u8],
) -> Result<(SuccessorStatement, VerifyOutcome), SignerError> {
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

    let keyid = keyid_bytes_from_fingerprint(predecessor)?;
    let signature = rnp::dearmor_bytes(signature)
        .map_err(|e| SignerError::Verify(format!("signature block does not dearmor: {e}")))?;
    let outcome = verify_detached(trusted_keyring, body.as_bytes(), &signature, &keyid)?;

    Ok((
        SuccessorStatement {
            predecessor_fingerprint: predecessor.to_uppercase(),
            successor_fingerprint: successor.to_uppercase(),
            created_unix: created,
        },
        outcome,
    ))
}

/// Evaluate a rotation chain: starting at `root_fingerprint`, apply the
/// statements in order. Each statement must (a) name the current
/// fingerprint as predecessor and (b) verify as [`VerifyOutcome::Trusted`]
/// against the trusted keyring. Returns the final root fingerprint after
/// the last verified rotation.
pub fn apply_successor_chain(
    root_fingerprint: &str,
    trusted_keyring: &[u8],
    statements: &[Vec<u8>],
) -> Result<String, SignerError> {
    let mut current = root_fingerprint.to_uppercase();
    check_fingerprint(&current, "root")?;
    for (i, statement) in statements.iter().enumerate() {
        let (stmt, outcome) = verify_successor_statement(trusted_keyring, statement)?;
        if stmt.predecessor_fingerprint != current {
            return Err(SignerError::Verify(format!(
                "successor chain broken at statement {i}: predecessor {} does not match the current root {current}",
                stmt.predecessor_fingerprint
            )));
        }
        match outcome {
            VerifyOutcome::Trusted(_) => {
                current = stmt.successor_fingerprint;
            }
            VerifyOutcome::Untrusted(keyid) => {
                return Err(SignerError::Trust(format!(
                    "successor statement {i} signed by an untrusted key {keyid}"
                )));
            }
            VerifyOutcome::Invalid(_) => {
                return Err(SignerError::Verify(format!(
                    "successor statement {i} has an invalid signature"
                )));
            }
        }
    }
    Ok(current)
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
