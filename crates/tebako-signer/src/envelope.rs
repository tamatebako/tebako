//! DEK grant envelopes (spec 10 §2): a data-encryption key IS a persisted
//! OpenPGP session key, and rnp's native wrap/unwrap — PKESK packets to N
//! recipients over an AES-256 symmetrically-encrypted data packet — is the
//! envelope. No custom crypto anywhere: the PKESK layer is librnp's
//! ephemeral-ECDH (or RSA) wrap, the payload layer AES-256/SHA-2 per the
//! SUITE-1 registry entry (spec 10 §5).
//!
//! A wrapped DEK plus the manifest digest IS a capability: possessing it
//! grants exactly the subtree the DEK opens, nothing else. Envelopes are
//! produced ASCII-armored so they embed directly in the authored YAML
//! envelope manifest (`/__tpkg__/envelopes.yaml`) without a codec.

use rnp::{Cipher, Context, ExportFlags, Hash, KeyringFormat, LoadSaveFlags};

use crate::error::SignerError;

/// Every fingerprint currently in the context's keyrings.
fn fingerprints(ctx: &Context) -> Result<Vec<String>, SignerError> {
    ctx.identifiers(rnp::IdentifierKind::Fingerprint)
        .map(|it| it.collect())
        .map_err(|e| SignerError::Envelope(e.to_string()))
}

/// Load a public key export into `ctx` and return the freshly added
/// PRIMARY key (an export may carry encryption subkeys; recipients are
/// named by their primary).
fn load_recipient<'ctx>(
    ctx: &'ctx Context,
    public_key: &[u8],
    index: usize,
) -> Result<rnp::Key<'ctx>, SignerError> {
    let before = fingerprints(ctx)?;
    ctx.load_keys(KeyringFormat::Gpg, public_key, LoadSaveFlags::PUBLIC)
        .map_err(|e| {
            SignerError::Envelope(format!(
                "recipient {index}: cannot load the public key: {e}"
            ))
        })?;
    for fingerprint in fingerprints(ctx)? {
        if before.contains(&fingerprint) {
            continue;
        }
        let key = ctx
            .find_key(rnp::KeyIdentifier::Fingerprint(&fingerprint))
            .map_err(|e| SignerError::Envelope(e.to_string()))?
            .ok_or_else(|| {
                SignerError::Envelope(format!("recipient {index}: cannot re-read the public key"))
            })?;
        if key.is_primary().unwrap_or(false) {
            return Ok(key);
        }
    }
    Err(SignerError::Envelope(format!(
        "recipient {index}: the public key export contains no primary key"
    )))
}

/// Wrap `dek` to every recipient public key (armored or binary exports;
/// ≥ 1 required). The result is an ASCII-armored OpenPGP message:
/// one PKESK packet per recipient over the DEK as the session payload.
pub fn wrap_dek(dek: &[u8], recipient_public_keys: &[&[u8]]) -> Result<Vec<u8>, SignerError> {
    if dek.is_empty() {
        return Err(SignerError::Envelope("cannot wrap an empty DEK".into()));
    }
    if recipient_public_keys.is_empty() {
        return Err(SignerError::Envelope(
            "cannot wrap a DEK to zero recipients".into(),
        ));
    }
    let ctx = Context::new().map_err(|e| SignerError::Envelope(e.to_string()))?;
    // Load every recipient key first (the encryptor borrows them).
    let mut keys = Vec::with_capacity(recipient_public_keys.len());
    for (i, public_key) in recipient_public_keys.iter().enumerate() {
        keys.push(load_recipient(&ctx, public_key, i)?);
    }
    let mut encryptor = rnp::Encryptor::new(&ctx, dek)
        .map_err(|e| SignerError::Envelope(e.to_string()))?
        .cipher(Cipher::Aes256)
        .hash(Hash::Sha256)
        .armor(true);
    for key in &keys {
        encryptor = encryptor.add_recipient(key);
    }
    let mut output = rnp::Output::to_memory().map_err(|e| SignerError::Envelope(e.to_string()))?;
    encryptor
        .build(&mut output)
        .map_err(|e| SignerError::Envelope(format!("wrapping failed: {e}")))?;
    output
        .into_bytes()
        .map_err(|e| SignerError::Envelope(e.to_string()))
}

/// Unwrap a grant envelope with a secret key (armored or binary export).
/// Any failure to open a recipient slot is the named EKEY-class error
/// [`SignerError::Envelope`] — never garbage, never a partial key.
pub fn unwrap_dek(envelope: &[u8], secret_key: &[u8]) -> Result<Vec<u8>, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Envelope(e.to_string()))?;
    ctx.load_keys(
        KeyringFormat::Gpg,
        secret_key,
        LoadSaveFlags::PUBLIC | LoadSaveFlags::SECRET,
    )
    .map_err(|e| SignerError::Envelope(format!("cannot load the recipient secret key: {e}")))?;
    rnp::decrypt(&ctx, envelope).map_err(|e| {
        SignerError::Envelope(format!(
            "no envelope recipient slot opens with the given key: {e}"
        ))
    })
}

/// The recipient keyids (16 lowercase hex each) an envelope is wrapped
/// to, read off the PKESK packets — no keyring needed. Identification
/// only; unwrap is the authority on what a key actually opens.
pub fn envelope_recipients(envelope: &[u8]) -> Result<Vec<String>, SignerError> {
    let json = rnp::dump_packets_bytes_to_json(envelope, Default::default())
        .map_err(|e| SignerError::Envelope(format!("cannot parse the envelope: {e}")))?;
    let mut out = Vec::new();
    // The dump names the PKESK packet's recipient field plainly "keyid"
    // (an envelope carries no other keyid-bearing packets).
    let needle = "\"keyid\":\"";
    let mut rest = json.as_str();
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        let Some(end) = after.find('"') else {
            return Err(SignerError::Envelope(
                "malformed recipient keyid in the packet dump".into(),
            ));
        };
        out.push(after[..end].to_lowercase());
        rest = &after[end..];
    }
    if out.is_empty() {
        return Err(SignerError::Envelope(
            "the envelope carries no PKESK recipient".into(),
        ));
    }
    Ok(out)
}

/// The keyid (16 lowercase hex) of the primary key in a public-key
/// export — the human-meaningful recipient id recorded in the envelope
/// manifest.
pub fn public_key_keyid(public_key: &[u8]) -> Result<String, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::KeyStore(e.to_string()))?;
    ctx.load_keys(KeyringFormat::Gpg, public_key, LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::KeyStore(format!("cannot load the public key: {e}")))?;
    let key = primary_key(&ctx)?;
    let fingerprint = key
        .fingerprint()
        .map_err(|e| SignerError::KeyStore(e.to_string()))?;
    let keyid = crate::keys::keyid_bytes_from_fingerprint(&fingerprint)?;
    Ok(crate::keys::hex_lower(&keyid))
}

/// The primary key in a freshly loaded context (exports may carry
/// subkeys; the primary names the recipient).
fn primary_key<'ctx>(ctx: &'ctx Context) -> Result<rnp::Key<'ctx>, SignerError> {
    let fingerprints = fingerprints(ctx)?;
    for fingerprint in fingerprints {
        let key = ctx
            .find_key(rnp::KeyIdentifier::Fingerprint(&fingerprint))
            .map_err(|e| SignerError::KeyStore(e.to_string()))?
            .ok_or_else(|| SignerError::KeyStore("cannot re-read the key".into()))?;
        if key.is_primary().unwrap_or(false) {
            return Ok(key);
        }
    }
    Err(SignerError::KeyStore(
        "the key export contains no primary key".into(),
    ))
}

/// Export the public half of a secret key (armored) — for minting
/// recipient key pairs in tooling and tests.
pub fn public_key_from_secret(secret_key: &[u8]) -> Result<Vec<u8>, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::KeyStore(e.to_string()))?;
    ctx.load_keys(
        KeyringFormat::Gpg,
        secret_key,
        LoadSaveFlags::PUBLIC | LoadSaveFlags::SECRET,
    )
    .map_err(|e| SignerError::KeyStore(format!("cannot load the secret key: {e}")))?;
    let key = primary_key(&ctx)?;
    key.export(ExportFlags::ARMORED | ExportFlags::PUBLIC | ExportFlags::SUBKEYS)
        .map_err(|e| SignerError::KeyStore(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A SUITE-1-shaped recipient pair (spec 10 §5): Ed25519 primary
    /// (certify/sign) + X25519 encryption subkey, minted in-memory.
    fn recipient_key(userid: &str) -> (Vec<u8>, Vec<u8>) {
        let ctx = Context::new().unwrap();
        let primary = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
            .hash(rnp::Hash::Sha256)
            .userid(userid)
            .add_usage(rnp::KeyUsage::Sign)
            .build(&ctx)
            .unwrap();
        rnp::SubkeyBuilder::new(rnp::Algorithm::Ecdh)
            .curve(rnp::Curve::Curve25519)
            .hash(rnp::Hash::Sha256)
            .add_usage(rnp::KeyUsage::EncryptComms)
            .build(&ctx, &primary)
            .unwrap();
        let public_key = primary
            .export(ExportFlags::ARMORED | ExportFlags::PUBLIC | ExportFlags::SUBKEYS)
            .unwrap();
        let secret_key = primary
            .export(ExportFlags::ARMORED | ExportFlags::SECRET | ExportFlags::SUBKEYS)
            .unwrap();
        (public_key, secret_key)
    }

    #[test]
    fn wrap_unwrap_roundtrip_multi_recipient() {
        let (pub_a, sec_a) = recipient_key("alice <a@example.com>");
        let (pub_b, sec_b) = recipient_key("bob <b@example.com>");
        let dek = [0x42u8; 32];

        let envelope = wrap_dek(&dek, &[&pub_a, &pub_b]).unwrap();
        let text = String::from_utf8(envelope.clone()).unwrap();
        assert!(text.starts_with("-----BEGIN PGP MESSAGE-----"), "{text}");

        // Both recipients open the SAME DEK.
        assert_eq!(unwrap_dek(&envelope, &sec_a).unwrap(), dek);
        assert_eq!(unwrap_dek(&envelope, &sec_b).unwrap(), dek);

        // The recipients are listed (PKESK packets, no keyring).
        let recipients = envelope_recipients(&envelope).unwrap();
        assert_eq!(recipients.len(), 2);
        assert!(recipients.iter().all(|k| k.len() == 16));
    }

    #[test]
    fn unwrap_with_the_wrong_key_is_the_named_ekey_error() {
        let (pub_a, _sec_a) = recipient_key("alice <a@example.com>");
        let (_pub_b, sec_b) = recipient_key("bob <b@example.com>");
        let envelope = wrap_dek(&[0x42u8; 32], &[&pub_a]).unwrap();
        let err = unwrap_dek(&envelope, &sec_b).unwrap_err();
        assert!(
            matches!(err, SignerError::Envelope(_)),
            "wrong key must be the named envelope error, got {err:?}"
        );
        assert!(
            err.to_string().contains("no envelope recipient slot opens"),
            "{err}"
        );
    }

    #[test]
    fn keyid_of_a_public_export() {
        let (public_key, secret_key) = recipient_key("alice <a@example.com>");
        let keyid = public_key_keyid(&public_key).unwrap();
        assert_eq!(keyid.len(), 16);
        assert!(keyid
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        // The re-exported public half identifies identically.
        let reexported = public_key_from_secret(&secret_key).unwrap();
        assert_eq!(public_key_keyid(&reexported).unwrap(), keyid);
    }

    #[test]
    fn wrap_rejects_empty_inputs() {
        let (public_key, _) = recipient_key("alice <a@example.com>");
        assert!(wrap_dek(&[], &[&public_key]).is_err());
        assert!(wrap_dek(&[0x42; 32], &[]).is_err());
        assert!(envelope_recipients(b"not pgp").is_err());
    }
}
