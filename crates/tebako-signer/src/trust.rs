//! The pinned-key trust store (`$TEBAKO_HOME/trust/<FINGERPRINT>.pub`):
//! one armored public key per file, named by the key's PRIMARY
//! fingerprint (40 uppercase hex). This is the add-registry TOFU pin
//! layout (spec 09 §9.1): a registry's head `signing:` block presents a
//! key, the operator confirms it against an out-of-band channel, and the
//! confirmed key lands here — a verification input on par with the
//! trusted keyring ([`crate::root::verification_keyring`] concatenates
//! both), but per-key addressable so a pin can be listed and removed
//! without rebuilding the keyring blob.
//!
//! Discipline (the store rules): tmp + rename installs (a partial pin is
//! invisible), read-only artifacts, fail closed on corruption — a `.pub`
//! file that does not parse, or parses to a key whose primary fingerprint
//! is not the filename, is a named [`SignerError::Trust`], never a skip.

use std::path::{Path, PathBuf};

use crate::error::{io_err, SignerError};

/// The pin file's extension under the trust dir.
pub const PIN_SUFFIX: &str = ".pub";

/// Outcome of a pin write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinOutcome {
    /// The key was not pinned and now is (the fingerprint).
    Pinned(String),
    /// The fingerprint was already pinned; nothing changed.
    AlreadyPinned(String),
}

/// Outcome of a key removal across both trust inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalOutcome {
    /// The key was trusted and no longer is (the fingerprint).
    Removed(String),
    /// The fingerprint named nothing in either store; nothing changed.
    NotTrusted(String),
}

/// One trusted key for the `keys list` rendering: its primary
/// fingerprint and which stores hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedKey {
    pub fingerprint: String,
    /// In the append-only trusted keyring (`tebako keys import`).
    pub in_keyring: bool,
    /// Pinned under `trust/<FINGERPRINT>.pub` (the add-registry TOFU).
    pub pinned: bool,
}

/// The pin file of one fingerprint (the filename carries the uppercase
/// spelling; lookups normalize).
pub fn pin_path(home: &Path, fingerprint: &str) -> PathBuf {
    tpkg::runtime_store::trust_dir(home).join(format!("{}{PIN_SUFFIX}", fingerprint.to_uppercase()))
}

/// Pin a public key (binary or armored export) into the trust store,
/// deduplicated by primary fingerprint. The pin file carries the key's
/// own armored spelling, read-only.
pub fn pin_trusted(home: &Path, public_key: &[u8]) -> Result<PinOutcome, SignerError> {
    let fingerprint = crate::keyring::fingerprint_of(public_key)?.to_uppercase();
    let path = pin_path(home, &fingerprint);
    if path.exists() {
        return Ok(PinOutcome::AlreadyPinned(fingerprint));
    }
    let dir = path.parent().ok_or_else(|| {
        SignerError::KeyStore(format!("the pin path '{}' has no parent", path.display()))
    })?;
    std::fs::create_dir_all(dir).map_err(|e| io_err(dir, &e))?;
    let tmp = dir.join(format!(".{fingerprint}.part-{}", std::process::id()));
    std::fs::write(&tmp, public_key).map_err(|e| io_err(&tmp, &e))?;
    std::fs::rename(&tmp, &path).map_err(|e| io_err(&path, &e))?;
    // Store artifacts are read-only; after the rename so the staging
    // write never races the attribute.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444));
    }
    Ok(PinOutcome::Pinned(fingerprint))
}

/// Whether a fingerprint is already a trust input — the embedded
/// first-party root, the trusted keyring, or a pin file (the add-registry
/// TOFU's short-circuit: an already-trusted key asks nothing).
pub fn is_trusted(home: &Path, fingerprint: &str) -> Result<bool, SignerError> {
    let want = fingerprint.to_uppercase();
    if want == crate::root::ROOT_FINGERPRINT {
        return Ok(true);
    }
    if pin_path(home, &want).exists() {
        return Ok(true);
    }
    let ring = crate::keyring::trusted_keyring_bytes(home)?;
    Ok(!ring.is_empty() && crate::keyring::contains_fingerprint(&ring, &want)?)
}

/// Every pinned key as (primary fingerprint, binary public-key bytes) —
/// the verification keyring's pin leg. Sorted by fingerprint for a
/// deterministic concatenation. An absent trust dir pins nothing; a
/// pin file that does not parse, or whose key's primary fingerprint is
/// not the filename, is a named trust error (fail closed — the store is
/// the operator's declaration, never something to guess around).
pub fn pinned_public_keys(home: &Path) -> Result<Vec<(String, Vec<u8>)>, SignerError> {
    let dir = tpkg::runtime_store::trust_dir(home);
    let read = match std::fs::read_dir(&dir) {
        Ok(read) => read,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err(&dir, &e)),
    };
    let mut pins: Vec<(String, Vec<u8>)> = Vec::new();
    for entry in read {
        let entry = entry.map_err(|e| io_err(&dir, &e))?;
        let path = entry.path();
        if !path.is_file() || !path.to_string_lossy().ends_with(PIN_SUFFIX) {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| io_err(&path, &e))?;
        let fingerprint = crate::keyring::fingerprint_of(&bytes)
            .map_err(|e| {
                SignerError::Trust(format!(
                    "the pinned key '{}' does not parse: {e} — remove it (`tebako keys remove <fingerprint>`) or replace it with the publisher's exported public key",
                    path.display()
                ))
            })?
            .to_uppercase();
        let named = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        if named != fingerprint {
            return Err(SignerError::Trust(format!(
                "the pinned key '{}' holds {fingerprint} but is named for {named} — the trust store is inconsistent; remove the file and re-pin through `tebako add-registry`",
                path.display()
            )));
        }
        let binary = rnp::dearmor_bytes(&bytes).unwrap_or(bytes);
        pins.push((fingerprint, binary));
    }
    pins.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(pins)
}

/// Every trusted key across both stores, for `tebako keys list`.
/// Sorted by fingerprint; a key in both stores lists once with both
/// flags set.
pub fn list_trusted(home: &Path) -> Result<Vec<TrustedKey>, SignerError> {
    let mut rows: Vec<TrustedKey> = Vec::new();
    let ring = crate::keyring::trusted_keyring_bytes(home)?;
    if !ring.is_empty() {
        for fingerprint in crate::keyring::primary_fingerprints(&ring)? {
            rows.push(TrustedKey {
                fingerprint,
                in_keyring: true,
                pinned: false,
            });
        }
    }
    for (fingerprint, _) in pinned_public_keys(home)? {
        match rows.iter_mut().find(|r| r.fingerprint == fingerprint) {
            Some(row) => row.pinned = true,
            None => rows.push(TrustedKey {
                fingerprint,
                in_keyring: false,
                pinned: true,
            }),
        }
    }
    rows.sort_by(|a, b| a.fingerprint.cmp(&b.fingerprint));
    Ok(rows)
}

/// Remove a key from every trust input (the pin file and the trusted
/// keyring), by primary OR subkey fingerprint, case-insensitively. The
/// keyring half rebuilds the blob without the key (tmp + rename — the
/// keyring is never edited in place).
pub fn remove_trusted(home: &Path, fingerprint: &str) -> Result<RemovalOutcome, SignerError> {
    let want = fingerprint.to_uppercase();
    let mut removed = false;
    let pin = pin_path(home, &want);
    if pin.exists() {
        // A read-only artifact needs the write bit back before removal
        // on platforms whose unlink honors it (windows).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(&pin, std::fs::Permissions::from_mode(0o644));
        }
        std::fs::remove_file(&pin).map_err(|e| io_err(&pin, &e))?;
        removed = true;
    }
    let ring = crate::keyring::trusted_keyring_bytes(home)?;
    if !ring.is_empty() && crate::keyring::contains_fingerprint(&ring, &want)? {
        let kept = crate::keyring::rebuild_without(&ring, &want)?;
        let path = crate::keyring::trusted_keyring_path(home);
        let dir = path.parent().ok_or_else(|| {
            SignerError::KeyStore(format!(
                "the keyring path '{}' has no parent",
                path.display()
            ))
        })?;
        std::fs::create_dir_all(dir).map_err(|e| io_err(dir, &e))?;
        let tmp = dir.join(format!(".trusted.part-{}", std::process::id()));
        std::fs::write(&tmp, &kept).map_err(|e| io_err(&tmp, &e))?;
        std::fs::rename(&tmp, &path).map_err(|e| io_err(&path, &e))?;
        removed = true;
    }
    Ok(if removed {
        RemovalOutcome::Removed(want)
    } else {
        RemovalOutcome::NotTrusted(want)
    })
}

/// The fingerprint of the first key in a public key export (binary or
/// armored) — the registry `signing:` block's consistency check (the
/// declared fingerprint must BE the key's).
pub fn public_key_fingerprint(public_key: &[u8]) -> Result<String, SignerError> {
    crate::keyring::fingerprint_of(public_key)
}

/// The untrusted-signer refusal's shared wording (exit 72 at every
/// boundary) — ONE owner for the payload-install, slice-fetch, and
/// runtime-fetch sites, which used to drift. User-facing: the failure,
/// then the remedy — never a path to hand-edit.
pub fn untrusted_signer_message(origin: &str, keyid: &str) -> String {
    format!(
        "{origin} is signed by {keyid}, which is not in the trusted keyring — refusing to install or execute; nothing was cached\n  if you trust this signer, register its public key with `tebako keys import <file>` (the publisher's exported public key, confirmed out of band)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-signer-trust-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A freshly generated Ed25519 key: (armored public, fingerprint).
    fn make_key(userid: &str) -> (Vec<u8>, String) {
        let ctx = rnp::Context::new().unwrap();
        let key = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
            .hash(rnp::Hash::Sha256)
            .userid(userid)
            .add_usage(rnp::KeyUsage::Sign)
            .build(&ctx)
            .unwrap();
        let public = key
            .export(
                rnp::ExportFlags::ARMORED | rnp::ExportFlags::PUBLIC | rnp::ExportFlags::SUBKEYS,
            )
            .unwrap();
        let fp = key.fingerprint().unwrap().to_uppercase();
        (public, fp)
    }

    #[test]
    fn a_pin_lands_read_only_and_dedupes() {
        let home = temp_home("pin");
        let (public, fp) = make_key("pin@example");
        assert_eq!(
            pin_trusted(&home, &public).unwrap(),
            PinOutcome::Pinned(fp.clone())
        );
        let path = pin_path(&home, &fp);
        assert_eq!(std::fs::read(&path).unwrap(), public);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o444
            );
        }
        assert_eq!(
            pin_trusted(&home, &public).unwrap(),
            PinOutcome::AlreadyPinned(fp.clone())
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn pins_join_the_verification_keyring_and_lists() {
        let home = temp_home("verify");
        let (public, fp) = make_key("pin2@example");
        // No trust dir at all: the verification keyring is keyring+root only.
        assert!(pinned_public_keys(&home).unwrap().is_empty());
        assert!(!is_trusted(&home, &fp).unwrap());
        pin_trusted(&home, &public).unwrap();
        assert!(is_trusted(&home, &fp).unwrap());
        // Case-insensitive lookups.
        assert!(is_trusted(&home, &fp.to_lowercase()).unwrap());
        let ring = crate::root::verification_keyring(&home).unwrap();
        assert!(crate::keyring::contains_fingerprint(&ring, &fp).unwrap());
        // The embedded root is trusted with nothing on disk.
        assert!(is_trusted(&home, crate::root::ROOT_FINGERPRINT).unwrap());
        let rows = list_trusted(&home).unwrap();
        assert_eq!(
            rows,
            vec![TrustedKey {
                fingerprint: fp.clone(),
                in_keyring: false,
                pinned: true,
            }]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_corrupt_or_misnamed_pin_fails_closed() {
        let home = temp_home("corrupt");
        let dir = tpkg::runtime_store::trust_dir(&home);
        std::fs::create_dir_all(&dir).unwrap();
        // Garbage under a .pub name.
        std::fs::write(dir.join("DEADBEEF.pub"), b"not a key\n").unwrap();
        let err = pinned_public_keys(&home).unwrap_err();
        assert!(err.to_string().contains("does not parse"), "{err}");
        // A well-formed key under another fingerprint's name.
        let (public, fp) = make_key("misnamed@example");
        let other = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert_ne!(fp, other);
        std::fs::write(dir.join(format!("{other}.pub")), &public).unwrap();
        let err = pinned_public_keys(&home).unwrap_err();
        assert!(err.to_string().contains("but is named for"), "{err}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn removal_covers_the_pin_and_the_keyring() {
        let home = temp_home("remove");
        let (pub_a, fp_a) = make_key("a@example");
        let (pub_b, fp_b) = make_key("b@example");
        pin_trusted(&home, &pub_a).unwrap();
        crate::keyring::register_trusted(&home, &pub_b).unwrap();
        // Both list; removing the pin leaves the keyring entry.
        assert_eq!(list_trusted(&home).unwrap().len(), 2);
        assert_eq!(
            remove_trusted(&home, &fp_a.to_lowercase()).unwrap(),
            RemovalOutcome::Removed(fp_a.clone())
        );
        assert!(!pin_path(&home, &fp_a).exists());
        assert!(!is_trusted(&home, &fp_a).unwrap());
        assert!(is_trusted(&home, &fp_b).unwrap());
        // The keyring leg: rebuilt without the key, the survivor intact.
        assert_eq!(
            remove_trusted(&home, &fp_b).unwrap(),
            RemovalOutcome::Removed(fp_b.clone())
        );
        assert!(!is_trusted(&home, &fp_b).unwrap());
        assert_eq!(
            remove_trusted(&home, &fp_b).unwrap(),
            RemovalOutcome::NotTrusted(fp_b.clone())
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_untrusted_signer_wording_names_the_verb() {
        let msg = untrusted_signer_message("app@1.0", "efc3c250f7862a48");
        assert!(
            msg.contains("app@1.0 is signed by efc3c250f7862a48"),
            "{msg}"
        );
        assert!(msg.contains("not in the trusted keyring"), "{msg}");
        assert!(msg.contains("tebako keys import <file>"), "{msg}");
        assert!(msg.contains("nothing was cached"), "{msg}");
    }
}
