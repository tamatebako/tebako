//! The press-local signing key: generated once, cached under
//! `$TEBAKO_HOME/keys`, and (by the tools) auto-registered into the local
//! trusted keyring — dev iteration never touches unsigned artifacts
//! (item 29 point 7).

use std::path::{Path, PathBuf};

use rnp::{Algorithm, Context, ExportFlags, Hash, KeyBuilder, KeyUsage};

use crate::error::{io_err, SignerError};

/// Directory name (under $TEBAKO_HOME) holding the cached key material.
pub const KEYS_DIR: &str = "keys";
/// Armored public export of the press-local key.
pub const PRESS_PUBLIC_FILE: &str = "press-local.pub";
/// Armored secret export of the press-local key (mode 0600 on unix).
pub const PRESS_SECRET_FILE: &str = "press-local.key";

/// The press-local key material plus its identifiers.
#[derive(Debug, Clone)]
pub struct PressKey {
    /// Armored public key export.
    pub public_key: Vec<u8>,
    /// Armored secret key export.
    pub secret_key: Vec<u8>,
    /// Signer keyid (low 64 bits of the fingerprint) as raw bytes — the
    /// value written into the tpkg v2 trailer.
    pub keyid: [u8; 8],
    /// Full OpenPGP fingerprint (hex).
    pub fingerprint: String,
}

impl PressKey {
    /// The signer keyid as the usual 16-character hex rendering.
    pub fn keyid_hex(&self) -> String {
        hex_lower(&self.keyid)
    }
}

/// The tebako home directory: $TEBAKO_HOME, else `~/.tebako`.
pub fn default_home() -> Result<PathBuf, SignerError> {
    if let Ok(home) = std::env::var("TEBAKO_HOME") {
        if !home.is_empty() {
            return Ok(PathBuf::from(home));
        }
    }
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => Ok(PathBuf::from(home).join(".tebako")),
        _ => Err(SignerError::KeyStore(
            "cannot determine tebako home (set TEBAKO_HOME)".into(),
        )),
    }
}

/// Load the press-local key from `home`, generating and caching it on
/// first use.
pub fn press_local_key(home: &Path) -> Result<PressKey, SignerError> {
    let keys_dir = home.join(KEYS_DIR);
    let pub_path = keys_dir.join(PRESS_PUBLIC_FILE);
    let sec_path = keys_dir.join(PRESS_SECRET_FILE);

    if pub_path.exists() && sec_path.exists() {
        let public_key = std::fs::read(&pub_path).map_err(|e| io_err(&pub_path, &e))?;
        let secret_key = std::fs::read(&sec_path).map_err(|e| io_err(&sec_path, &e))?;
        return identify(&public_key, &secret_key);
    }

    generate_and_cache(&keys_dir, &pub_path, &sec_path)
}

fn generate_and_cache(
    keys_dir: &Path,
    pub_path: &Path,
    sec_path: &Path,
) -> Result<PressKey, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Keygen(e.to_string()))?;
    let key = KeyBuilder::new(Algorithm::Eddsa)
        .hash(Hash::Sha256)
        .userid("tebako-press-local (per-machine package signing key)")
        .add_usage(KeyUsage::Sign)
        .build(&ctx)
        .map_err(|e| SignerError::Keygen(e.to_string()))?;

    let public_key = key
        .export(ExportFlags::ARMORED | ExportFlags::PUBLIC | ExportFlags::SUBKEYS)
        .map_err(|e| SignerError::Keygen(e.to_string()))?;
    let secret_key = key
        .export(ExportFlags::ARMORED | ExportFlags::SECRET | ExportFlags::SUBKEYS)
        .map_err(|e| SignerError::Keygen(e.to_string()))?;

    std::fs::create_dir_all(keys_dir).map_err(|e| io_err(keys_dir, &e))?;
    std::fs::write(pub_path, &public_key).map_err(|e| io_err(pub_path, &e))?;
    std::fs::write(sec_path, &secret_key).map_err(|e| io_err(sec_path, &e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(sec_path, std::fs::Permissions::from_mode(0o600));
    }

    identify(&public_key, &secret_key)
}

/// Derive the identifiers of a public+secret key pair by loading them.
fn identify(public_key: &[u8], secret_key: &[u8]) -> Result<PressKey, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::KeyStore(e.to_string()))?;
    ctx.load_keys(
        rnp::KeyringFormat::Gpg,
        secret_key,
        rnp::LoadSaveFlags::SECRET,
    )
    .map_err(|e| SignerError::KeyStore(format!("cached press-local key is unreadable: {e}")))?;

    let mut fingerprints = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .map_err(|e| SignerError::KeyStore(e.to_string()))?;
    let Some(fingerprint) = fingerprints.next() else {
        return Err(SignerError::KeyStore(
            "cached press-local key contains no key".into(),
        ));
    };
    let keyid = keyid_bytes_from_fingerprint(&fingerprint)?;
    Ok(PressKey {
        public_key: public_key.to_vec(),
        secret_key: secret_key.to_vec(),
        keyid,
        fingerprint,
    })
}

/// Load a secret key from `path` and derive its full identity (the public
/// half is re-exported from the loaded secret key, so a lone `.key` file
/// is sufficient).
fn identify_secret(secret_key: &[u8]) -> Result<PressKey, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::KeyStore(e.to_string()))?;
    // PGP private key blocks carry the public key material as well; load
    // both halves so the public key can be re-exported from the secret
    // file alone.
    ctx.load_keys(
        rnp::KeyringFormat::Gpg,
        secret_key,
        rnp::LoadSaveFlags::PUBLIC | rnp::LoadSaveFlags::SECRET,
    )
    .map_err(|e| SignerError::KeyStore(format!("secret key is unreadable: {e}")))?;
    let mut fingerprints = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .map_err(|e| SignerError::KeyStore(e.to_string()))?;
    let Some(fingerprint) = fingerprints.next() else {
        return Err(SignerError::KeyStore(
            "secret key file contains no key".into(),
        ));
    };
    let key = ctx
        .find_key(rnp::KeyIdentifier::Fingerprint(&fingerprint))
        .map_err(|e| SignerError::KeyStore(e.to_string()))?
        .ok_or_else(|| SignerError::KeyStore("cannot re-read the secret key".into()))?;
    let public_key = key
        .export(ExportFlags::ARMORED | ExportFlags::PUBLIC | ExportFlags::SUBKEYS)
        .map_err(|e| SignerError::KeyStore(e.to_string()))?;
    let keyid = keyid_bytes_from_fingerprint(&fingerprint)?;
    Ok(PressKey {
        public_key,
        secret_key: secret_key.to_vec(),
        keyid,
        fingerprint,
    })
}

/// Find a secret key in `$TEBAKO_HOME/keys` whose keyid (16-hex,
/// case-insensitive) matches. Returns `Ok(None)` when no key matches —
/// callers decide whether that is a named error.
pub fn secret_key_by_keyid(home: &Path, keyid_hex: &str) -> Result<Option<PressKey>, SignerError> {
    let want = keyid_hex.to_lowercase();
    if want.len() != 16 || !want.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(SignerError::KeyStore(format!(
            "invalid keyid (want 16 hex chars): {keyid_hex}"
        )));
    }
    let dir = home.join(KEYS_DIR);
    let entries = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err(&dir, &e)),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("key") {
            continue;
        }
        let secret = std::fs::read(&path).map_err(|e| io_err(&path, &e))?;
        let Ok(key) = identify_secret(&secret) else {
            continue; // not a usable secret key file
        };
        if key.keyid_hex() == want {
            return Ok(Some(key));
        }
    }
    Ok(None)
}

/// The low 64 bits of a hex OpenPGP fingerprint as raw bytes.
pub fn keyid_bytes_from_fingerprint(fingerprint: &str) -> Result<[u8; 8], SignerError> {
    let hex: String = fingerprint.chars().filter(|c| !c.is_whitespace()).collect();
    if hex.len() < 16 {
        return Err(SignerError::KeyStore(format!(
            "fingerprint too short: {fingerprint}"
        )));
    }
    let keyid_hex = &hex[hex.len() - 16..];
    let mut keyid = [0u8; 8];
    for (i, b) in keyid.iter_mut().enumerate() {
        *b = u8::from_str_radix(&keyid_hex[2 * i..2 * i + 2], 16)
            .map_err(|_| SignerError::KeyStore(format!("bad fingerprint: {fingerprint}")))?;
    }
    Ok(keyid)
}

/// Lowercase hex rendering.
pub fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}
