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
fn fingerprint_of(public_key: &[u8]) -> Result<String, SignerError> {
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
fn contains_fingerprint(keyring: &[u8], fingerprint: &str) -> Result<bool, SignerError> {
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
