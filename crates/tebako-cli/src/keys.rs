//! `tebako keys list | remove` — the trusted-signing-key surface
//! (spec 09 §9, tebako#617; `tebako keys import` shipped earlier and
//! stays in main).
//!
//! `list` renders every trust input in one report — the embedded
//! first-party root, the append-only trusted keyring
//! (`tebako keys import`), and the add-registry pin store — with the
//! registry bindings the book recorded at pin time, so an operator can
//! answer "which registry did I trust this key for" without reading
//! config.yaml. `remove` drops a fingerprint from every operator store
//! (pin file + keyring rebuild, both through the signer's fail-closed
//! discipline) and journals the removal; the embedded root is not
//! removable — a successor statement rotates it, never a local edit.

use std::path::Path;

use crate::error::TebakoError;
use crate::install::{journal, map_shim};

const EX_USAGE: i32 = 64;
const EX_TEBAKO_TRUST: i32 = 72;
const EX_TEBAKO_IO: i32 = 74;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, code)
}

/// One `keys list` row: the primary fingerprint, which stores trust it,
/// and the registries whose book entry recorded it at pin time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRow {
    pub fingerprint: String,
    /// Human spellings of the trust inputs: "embedded root", "keyring",
    /// "pin".
    pub sources: Vec<&'static str>,
    /// Canonical registry references whose book entry names this key.
    pub registries: Vec<String>,
}

/// Every trusted key: the embedded root first, then the operator stores
/// (sorted by fingerprint, the signer's order), with the registry
/// bindings folded in from the config book.
pub fn list_keys(home: &Path) -> Result<Vec<KeyRow>, TebakoError> {
    let mut rows = vec![KeyRow {
        fingerprint: tebako_signer::ROOT_FINGERPRINT.to_string(),
        sources: vec!["embedded root"],
        registries: Vec::new(),
    }];
    for key in tebako_signer::list_trusted(home).map_err(map_keys_signer)? {
        let mut sources = Vec::new();
        if key.in_keyring {
            sources.push("keyring");
        }
        if key.pinned {
            sources.push("pin");
        }
        // The root key ALSO pinned or imported folds into its own row —
        // one row per fingerprint.
        if key.fingerprint == tebako_signer::ROOT_FINGERPRINT {
            rows[0].sources.extend(sources);
            continue;
        }
        rows.push(KeyRow {
            fingerprint: key.fingerprint,
            sources,
            registries: Vec::new(),
        });
    }
    // The registry bindings: the book entry's `signing_fingerprint` is
    // the add-registry pin record (tebako-shim config, the book's owner).
    let config = tebako_shim::config::load_config(home).map_err(map_shim)?;
    for entry in &config.registries {
        let Some(fp) = &entry.signing_fingerprint else {
            continue;
        };
        let fp = fp.to_uppercase();
        if let Some(row) = rows.iter_mut().find(|r| r.fingerprint == fp) {
            row.registries.push(entry.reference.clone());
        }
    }
    Ok(rows)
}

/// Drop a fingerprint from every operator trust store. The embedded
/// root refuses (72 — rotating it is the successor-statement ceremony,
/// never a local edit); an unknown fingerprint is the named
/// nothing-to-remove (72) pointing at `keys list`. A removal journals
/// `event=trusted-key-removed`.
pub fn remove_key(home: &Path, fingerprint: &str) -> Result<String, TebakoError> {
    let fp = fingerprint.trim().to_uppercase();
    if fp.len() != 40 || !fp.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(err(
            EX_USAGE,
            format!(
                "'{fingerprint}' is not a key fingerprint — `tebako keys list` prints the 40-hex spellings"
            ),
        ));
    }
    if fp == tebako_signer::ROOT_FINGERPRINT {
        return Err(err(
            EX_TEBAKO_TRUST,
            "the embedded root key is not removable — it is the trust anchor every verification starts from"
                .to_string(),
        ));
    }
    match tebako_signer::remove_trusted(home, &fp).map_err(map_keys_signer)? {
        tebako_signer::RemovalOutcome::Removed(fp) => {
            journal(home, &format!("event=trusted-key-removed fingerprint={fp}"));
            Ok(fp)
        }
        tebako_signer::RemovalOutcome::NotTrusted(fp) => Err(err(
            EX_TEBAKO_TRUST,
            format!(
                "{fp} is not in the trusted keyring or the pin store — nothing to remove\n  \
                 `tebako keys list` prints the trusted fingerprints"
            ),
        )),
    }
}

fn map_keys_signer(e: tebako_signer::SignerError) -> TebakoError {
    match e {
        tebako_signer::SignerError::Trust(_) => err(EX_TEBAKO_TRUST, e.to_string()),
        other => err(
            EX_TEBAKO_IO,
            format!("could not maintain the trust store: {other}"),
        ),
    }
}
