//! tebako-signer — the OpenPGP half of the tpkg chain of trust (item 29).
//!
//! One signature mechanism for the whole project (OpenPGP via
//! [rnp-rs](https://github.com/rnpgp/rnp-rs) over librnp):
//!
//! - **press-local key** ([`press_local_key`]): an Ed25519 key generated
//!   once per machine and cached under `$TEBAKO_HOME/keys`; every package
//!   is signed with it, and the tools auto-register it locally so dev
//!   iteration never produces or accepts unsigned artifacts.
//! - **trusted keyring** ([`keyring` module]): `$TEBAKO_HOME/keyring/
//!   trusted.pgp`, a binary GPG keyring; additional signer keys are
//!   TOFU-registered with named outcomes ([`keyring::RegisterOutcome`]).
//! - **sign/verify** ([`sign_detached`], [`verify_detached`]): detached
//!   OpenPGP signatures over byte strings; verification classifies into
//!   [`VerifyOutcome::Trusted`] / [`VerifyOutcome::Untrusted`] /
//!   [`VerifyOutcome::Invalid`] — the named trust errors of item 29 map
//!   onto these.

#![forbid(unsafe_code)]

mod error;
pub mod keyring;
mod keys;
mod sign;

pub use error::SignerError;
pub use keyring::{register_trusted, trusted_keyring_bytes, trusted_keyring_path, RegisterOutcome};
pub use keys::{default_home, hex_lower, keyid_bytes_from_fingerprint, press_local_key, PressKey};
pub use sign::{sign_detached, verify_detached, VerifyOutcome};
