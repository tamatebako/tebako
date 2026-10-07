//! The trusted keyring (`$TEBAKO_HOME/keyring/trusted.pgp`): a
//! concatenation of binary transferable public keys. Trust-on-first-use
//! registration with named outcomes (item 29 point 4: additional keys are
//! TOFU-registered; trust is established by registration, never by a skip
//! flag).

use std::path::{Path, PathBuf};

use rnp::{Context, KeyringFormat, LoadSaveFlags};

use crate::error::{io_err, SignerError};

/// Directory name (under $TEBAKO_HOME) holding the trusted keyring.
pub const KEYRING_DIR: &str = "keyring";
/// The trusted keyring file (binary GPG keyring: concatenated public keys).
pub const TRUSTED_FILE: &str = "trusted.pgp";

/// Path of the trusted keyring file.
pub fn trusted_keyring_path(home: &Path) -> PathBuf {
    home.join(KEYRING_DIR).join(TRUSTED_FILE)
}

/// Outcome of a TOFU registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterOutcome {
    /// The key was not in the keyring and has been added.
    Added(String),
    /// The key was already trusted; nothing changed.
    AlreadyTrusted(String),
}

/// Read the trusted keyring's raw bytes (empty when absent — an empty
/// keyring is valid input for verification and simply trusts nobody).
pub fn trusted_keyring_bytes(home: &Path) -> Result<Vec<u8>, SignerError> {
    let path = trusted_keyring_path(home);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(io_err(&path, &e)),
    }
}

/// Register a public key (binary or armored export) into the trusted
/// keyring, deduplicated by fingerprint.
pub fn register_trusted(home: &Path, public_key: &[u8]) -> Result<RegisterOutcome, SignerError> {
    let fingerprint = fingerprint_of(public_key)?;

    let existing = trusted_keyring_bytes(home)?;
    if !existing.is_empty() && contains_fingerprint(&existing, &fingerprint)? {
        return Ok(RegisterOutcome::AlreadyTrusted(fingerprint));
    }

    // Binary, non-armored export for the concatenated keyring file.
    let binary = export_binary_public(public_key)?;

    let path = trusted_keyring_path(home);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| io_err(dir, &e))?;
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| io_err(&path, &e))?;
    f.write_all(&binary).map_err(|e| io_err(&path, &e))?;

    Ok(RegisterOutcome::Added(fingerprint))
}

/// The fingerprint of the first key in a public key export.
pub(crate) fn fingerprint_of(public_key: &[u8]) -> Result<String, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Trust(e.to_string()))?;
    ctx.load_keys(KeyringFormat::Gpg, public_key, LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::Trust(format!("not a usable public key: {e}")))?;
    let mut fps = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .map_err(|e| SignerError::Trust(e.to_string()))?;
    fps.next()
        .ok_or_else(|| SignerError::Trust("no key in the public key material".into()))
}

/// Whether a keyring blob already contains a fingerprint.
pub(crate) fn contains_fingerprint(keyring: &[u8], fingerprint: &str) -> Result<bool, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Trust(e.to_string()))?;
    ctx.load_keys(KeyringFormat::Gpg, keyring, LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::Trust(format!("trusted keyring is unreadable: {e}")))?;
    let mut fps = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .map_err(|e| SignerError::Trust(e.to_string()))?;
    let want = fingerprint.to_uppercase();
    Ok(fps.any(|fp| fp.to_uppercase() == want))
}

/// Re-export a (possibly armored) public key as binary transferable public
/// key bytes for concatenation into the keyring file.
fn export_binary_public(public_key: &[u8]) -> Result<Vec<u8>, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Trust(e.to_string()))?;
    ctx.load_keys(KeyringFormat::Gpg, public_key, LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::Trust(format!("not a usable public key: {e}")))?;
    let fp = {
        let mut fps = ctx
            .identifiers(rnp::IdentifierKind::Fingerprint)
            .map_err(|e| SignerError::Trust(e.to_string()))?;
        fps.next()
            .ok_or_else(|| SignerError::Trust("no key in the public key material".into()))?
    };
    let key = ctx
        .find_key(rnp::KeyIdentifier::Fingerprint(&fp))
        .map_err(|e| SignerError::Trust(e.to_string()))?
        .ok_or_else(|| SignerError::Trust("cannot re-read the key".into()))?;
    key.export(rnp::ExportFlags::PUBLIC | rnp::ExportFlags::SUBKEYS)
        .map_err(|e| SignerError::Trust(e.to_string()))
}

/// The PRIMARY fingerprint of every key in a keyring blob (uppercase,
/// load order) — subkeys resolve to their primary, so each transferable
/// key lists once.
pub(crate) fn primary_fingerprints(keyring: &[u8]) -> Result<Vec<String>, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Trust(e.to_string()))?;
    ctx.load_keys(KeyringFormat::Gpg, keyring, LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::Trust(format!("trusted keyring is unreadable: {e}")))?;
    let fps: Vec<String> = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .map_err(|e| SignerError::Trust(e.to_string()))?
        .collect();
    let mut primaries: Vec<String> = Vec::new();
    for fp in fps {
        let key = ctx
            .find_key(rnp::KeyIdentifier::Fingerprint(&fp))
            .map_err(|e| SignerError::Trust(e.to_string()))?
            .ok_or_else(|| SignerError::Trust("cannot re-read a keyring key".into()))?;
        // The primary_keyid_of pattern: on a PRIMARY key librnp's
        // primary_fprint answers BadParameters / None — the key is its
        // own primary.
        let primary = match key.primary_fprint() {
            Ok(Some(pfp)) if !pfp.is_empty() => pfp,
            Ok(_) => fp.clone(),
            Err(e) if e.kind() == rnp::ErrorKind::BadParameters => fp.clone(),
            Err(e) => return Err(SignerError::Trust(e.to_string())),
        };
        let primary = primary.to_uppercase();
        if !primaries.contains(&primary) {
            primaries.push(primary);
        }
    }
    Ok(primaries)
}

/// The keyring blob without the transferable key whose primary OR any
/// of whose subkeys' fingerprint is `fingerprint` (case-insensitive) —
/// the removal half of the trust store's keyring leg. The remaining
/// primaries re-export (binary, subkeys included) in load order.
pub(crate) fn rebuild_without(keyring: &[u8], fingerprint: &str) -> Result<Vec<u8>, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Trust(e.to_string()))?;
    ctx.load_keys(KeyringFormat::Gpg, keyring, LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::Trust(format!("trusted keyring is unreadable: {e}")))?;
    let want = fingerprint.to_uppercase();
    let fps: Vec<String> = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .map_err(|e| SignerError::Trust(e.to_string()))?
        .collect();
    // Resolve every enumerated fingerprint to its primary; a target hit
    // at either level drops the whole transferable key.
    let mut dropped: Vec<String> = Vec::new();
    let mut primaries: Vec<String> = Vec::new();
    for fp in &fps {
        let key = ctx
            .find_key(rnp::KeyIdentifier::Fingerprint(fp))
            .map_err(|e| SignerError::Trust(e.to_string()))?
            .ok_or_else(|| SignerError::Trust("cannot re-read a keyring key".into()))?;
        let primary = match key.primary_fprint() {
            Ok(Some(pfp)) if !pfp.is_empty() => pfp,
            Ok(_) => fp.clone(),
            Err(e) if e.kind() == rnp::ErrorKind::BadParameters => fp.clone(),
            Err(e) => return Err(SignerError::Trust(e.to_string())),
        }
        .to_uppercase();
        if fp.to_uppercase() == want {
            dropped.push(primary.clone());
        }
        if !primaries.contains(&primary) {
            primaries.push(primary);
        }
    }
    let mut out: Vec<u8> = Vec::new();
    for primary in primaries {
        if dropped.contains(&primary) || primary == want {
            continue;
        }
        let key = ctx
            .find_key(rnp::KeyIdentifier::Fingerprint(&primary))
            .map_err(|e| SignerError::Trust(e.to_string()))?
            .ok_or_else(|| SignerError::Trust("cannot re-read a keyring key".into()))?;
        let bytes = key
            .export(rnp::ExportFlags::PUBLIC | rnp::ExportFlags::SUBKEYS)
            .map_err(|e| SignerError::Trust(e.to_string()))?;
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// The keyid (16 lowercase hex) of the PRIMARY key that `issuer_keyid`
/// belongs to in this keyring: the issuer itself when it IS a primary,
/// `Ok(None)` when the keyring holds no such key. Registry signature pins
/// name the signing key's PRIMARY keyid (spec 09 §9 — the identity, not
/// the rotating subkey instrument); verification resolves a signature's
/// issuer through this before comparing.
pub fn primary_keyid_of(keyring: &[u8], issuer_keyid: &str) -> Result<Option<String>, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Verify(e.to_string()))?;
    if !keyring.is_empty() {
        ctx.load_keys(KeyringFormat::Gpg, keyring, LoadSaveFlags::PUBLIC)
            .map_err(|e| SignerError::Verify(format!("cannot load the keyring: {e}")))?;
    }
    let want = issuer_keyid.to_lowercase();
    // Membership first: librnp's locate-by-keyid does not reliably answer
    // "absent" (an all-zero keyid comes back as SOME key handle) — the
    // crate's established pattern (sign.rs's keyring_has_keyid) iterates
    // the keyring's keyids instead.
    let mut ids = ctx
        .identifiers(rnp::IdentifierKind::Keyid)
        .map_err(|e| SignerError::Verify(e.to_string()))?;
    if !ids.any(|id| id.to_lowercase() == want) {
        return Ok(None);
    }
    let Some(key) = ctx
        .find_key(rnp::KeyIdentifier::Keyid(&want))
        .map_err(|e| SignerError::Verify(format!("cannot look up keyid {want}: {e}")))?
    else {
        return Ok(None);
    };
    // On a PRIMARY key librnp's rnp_key_get_primary_fprint answers
    // BadParameters (there is no primary above it): like an empty value,
    // that means the issuer is its own primary.
    let primary_fp = match key.primary_fprint() {
        Ok(Some(fp)) => fp,
        Ok(None) => key
            .fingerprint()
            .map_err(|e| SignerError::Verify(e.to_string()))?,
        Err(e) if e.kind() == rnp::ErrorKind::BadParameters => key
            .fingerprint()
            .map_err(|e| SignerError::Verify(e.to_string()))?,
        Err(e) => return Err(SignerError::Verify(e.to_string())),
    };
    let keyid = crate::keys::keyid_bytes_from_fingerprint(&primary_fp)?;
    Ok(Some(crate::keys::hex_lower(&keyid)))
}
