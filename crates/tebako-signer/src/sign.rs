//! Detached OpenPGP sign/verify over byte strings (the tpkg v2 trailer's
//! signature block is produced and checked here).

use rnp::{Context, KeyringFormat, LoadSaveFlags};

use crate::error::SignerError;

/// Produce a detached OpenPGP signature over `data` with the given
/// (armored or binary) secret key. `fingerprint` selects the signing key.
pub fn sign_detached(
    data: &[u8],
    secret_key: &[u8],
    fingerprint: &str,
) -> Result<Vec<u8>, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Sign(e.to_string()))?;
    ctx.load_keys(KeyringFormat::Gpg, secret_key, LoadSaveFlags::SECRET)
        .map_err(|e| SignerError::Sign(format!("cannot load the signing key: {e}")))?;
    let key = ctx
        .find_key(rnp::KeyIdentifier::Fingerprint(fingerprint))
        .map_err(|e| SignerError::Sign(e.to_string()))?
        .ok_or_else(|| SignerError::Sign("signing key not found after load".into()))?;
    rnp::sign_detached(&ctx, data, &key).map_err(|e| SignerError::Sign(e.to_string()))
}

/// The outcome of a detached-signature verification against the trusted
/// keyring. A verification that ran is never an error — the outcome
/// carries the classification (item 29's named trust errors map onto
/// these).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// The signature is valid and the signer's key is in the trusted
    /// keyring. Payload is the signer keyid (hex).
    Trusted(String),
    /// The signature is well-formed but the signer's key is NOT in the
    /// trusted keyring (TOFU candidates: register the key, then re-verify).
    Untrusted(String),
    /// The signature does not validate (tampered data or signature).
    /// Payload is the signer keyid when the signature carries one.
    Invalid(Option<String>),
}

/// Extract the issuer fingerprint (40-hex, uppercase) of a detached
/// signature, via the OpenPGP packet dump — no keyring needed.
pub fn signature_issuer_fingerprint(signature: &[u8]) -> Result<String, SignerError> {
    let json = rnp::dump_packets_bytes_to_json(signature, Default::default())
        .map_err(|e| SignerError::Verify(format!("cannot parse the signature: {e}")))?;
    let needle = "\"issuer fingerprint\"";
    let pos = json
        .find(needle)
        .ok_or_else(|| SignerError::Verify("no issuer fingerprint subpacket in the signature".into()))?;
    let rest = &json[pos..];
    let tag = "\"fingerprint\":\"";
    let fp_pos = rest
        .find(tag)
        .ok_or_else(|| SignerError::Verify("no fingerprint value in the signature".into()))?;
    let fp = &rest[fp_pos + tag.len()..];
    let end = fp
        .find('"')
        .ok_or_else(|| SignerError::Verify("malformed fingerprint value in the signature".into()))?;
    Ok(fp[..end].to_uppercase())
}

/// Verify a detached signature with the full Trusted/Untrusted/Invalid
/// classification, self-hinting from the signature's issuer fingerprint:
/// a signature from a key that IS in the keyring but does not validate is
/// `Invalid`; a signature from a key NOT in the keyring is `Untrusted`.
pub fn verify_detached_full(
    trusted_keyring: &[u8],
    data: &[u8],
    signature: &[u8],
) -> Result<VerifyOutcome, SignerError> {
    let hint = signature_issuer_fingerprint(signature)
        .ok()
        .and_then(|fp| crate::keys::keyid_bytes_from_fingerprint(&fp).ok())
        .unwrap_or([0; 8]);
    verify_detached(trusted_keyring, data, signature, &hint)
}
/// bytes (empty = trust nobody). `signer_keyid_hint` is the signer keyid
/// recorded alongside the signature (the tpkg v2 trailer carries it);
/// pass all-zero when unknown — it is used to classify an unknown signer
/// (`Untrusted`) apart from a bad signature (`Invalid`) when librnp only
/// reports a bare "invalid signature".
pub fn verify_detached(
    trusted_keyring: &[u8],
    data: &[u8],
    signature: &[u8],
    signer_keyid_hint: &[u8; 8],
) -> Result<VerifyOutcome, SignerError> {
    let ctx = Context::new().map_err(|e| SignerError::Verify(e.to_string()))?;
    if !trusted_keyring.is_empty() {
        ctx.load_keys(KeyringFormat::Gpg, trusted_keyring, LoadSaveFlags::PUBLIC)
            .map_err(|e| SignerError::Verify(format!("cannot load the trusted keyring: {e}")))?;
    }

    let hint_hex = if *signer_keyid_hint == [0; 8] {
        String::new()
    } else {
        crate::keys::hex_lower(signer_keyid_hint)
    };

    let verified = rnp::verify_detached(&ctx, data, signature);
    match verified {
        Ok(result) => {
            let signatures = result
                .signatures()
                .map_err(|e| SignerError::Verify(e.to_string()))?;
            let Some(sig) = signatures.first() else {
                return Ok(VerifyOutcome::Invalid(None));
            };
            let keyid = sig.keyid().unwrap_or_default();
            match sig.status() {
                rnp::SignatureStatus::Valid => Ok(VerifyOutcome::Trusted(keyid)),
                rnp::SignatureStatus::Unknown => {
                    Ok(VerifyOutcome::Untrusted(if keyid.is_empty() {
                        hint_hex
                    } else {
                        keyid
                    }))
                }
                _ => Ok(VerifyOutcome::Invalid(if keyid.is_empty() {
                    None
                } else {
                    Some(keyid)
                })),
            }
        }
        Err(e) => {
            use rnp::ErrorKind;
            match e.kind() {
                // librnp knows the signer is missing from the keyring
                ErrorKind::SigNoSignerKey
                | ErrorKind::SigNoSignerId
                | ErrorKind::SignatureUnknown => Ok(VerifyOutcome::Untrusted(hint_hex)),
                // bare "invalid signature": an unknown signer and a tampered
                // message look alike to librnp — the keyring membership of
                // the hinted keyid tells them apart
                ErrorKind::SignatureInvalid => {
                    if !hint_hex.is_empty() && keyring_has_keyid(&ctx, &hint_hex)? {
                        Ok(VerifyOutcome::Invalid(Some(hint_hex)))
                    } else {
                        Ok(VerifyOutcome::Untrusted(hint_hex))
                    }
                }
                ErrorKind::NoSignaturesFound => Ok(VerifyOutcome::Invalid(None)),
                _ => Err(SignerError::Verify(e.to_string())),
            }
        }
    }
}

/// Whether the keyring attached to `ctx` holds a key with this keyid.
fn keyring_has_keyid(ctx: &Context, keyid_hex: &str) -> Result<bool, SignerError> {
    let mut ids = ctx
        .identifiers(rnp::IdentifierKind::Keyid)
        .map_err(|e| SignerError::Verify(e.to_string()))?;
    let want = keyid_hex.to_uppercase();
    Ok(ids.any(|id| id.to_uppercase() == want))
}
