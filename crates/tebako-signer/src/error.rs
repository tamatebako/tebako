//! Named errors for the tebako-signer crate.

use std::fmt;

/// Every fallible signer operation fails with one of these kinds; the
/// Display strings are meant to land in tool stderr output unwrapped.
#[derive(Debug)]
pub enum SignerError {
    /// Press-local key generation failed.
    Keygen(String),
    /// Reading/writing the cached key material failed.
    KeyStore(String),
    /// Producing a signature failed.
    Sign(String),
    /// Running a verification failed (a *bad* signature is a VerifyOutcome,
    /// not an error; this is for operational failures).
    Verify(String),
    /// DEK envelope wrap/unwrap failed (spec 10 §2): the EKEY-class named
    /// error — a recipient slot that does not open is this, never garbage.
    Envelope(String),
    /// Trusted-keyring loading/registration failed.
    Trust(String),
    /// Plain i/o failure with path context.
    Io(String),
}

impl fmt::Display for SignerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignerError::Keygen(m) => write!(f, "key generation failed: {m}"),
            SignerError::KeyStore(m) => write!(f, "key store error: {m}"),
            SignerError::Sign(m) => write!(f, "signing failed: {m}"),
            SignerError::Verify(m) => write!(f, "verification failed: {m}"),
            SignerError::Envelope(m) => write!(f, "key envelope error: {m}"),
            SignerError::Trust(m) => write!(f, "trusted keyring error: {m}"),
            SignerError::Io(m) => write!(f, "i/o error: {m}"),
        }
    }
}

impl std::error::Error for SignerError {}

pub(crate) fn io_err(path: &std::path::Path, e: &std::io::Error) -> SignerError {
    SignerError::Io(format!("{}: {e}", path.display()))
}
