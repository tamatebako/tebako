//! The runtime release card (spec 18 C2, scenarios S11/S12): the loader
//! reads the contract set of a runtime release manifest **before any
//! download** and refuses by name — never assumes, never silently serves.
//!
//! The contract set lives in the release's `manifest.json`, per package
//! entry (the runtime factory's release index; spec 18's
//! `runtime-manifest.yaml` carries the same fields when the factory
//! ships it — the reader is field-driven, not file-driven):
//!
//! - `contract_era` — the contract-graph generation. Anything
//!   undeclared is **pre-era (era 1)** and is refused by name.
//! - `contract_version` — the bootstrap↔runtime handoff semantics
//!   (spec 17). Newer than spoken → exit 75, both numbers named.
//! - `mount_root` — the env image's mount root the exe was built with.
//!   Declared, never guessed: a missing key is the pre-era signal, not
//!   a reason to invent a value.
//!
//! The fail-closed gate ([`gate`]) returns the declared set on success;
//! the refusal classes are [`ContractError]'s. tebako-bootstrap links this
//! crate with default-features = false (the gix/reqwest git stack stays
//! outside its 3 MiB gate) for the payload-cache layer, but keeps reading
//! the release card with its own string-scan reader — [`SPOKEN_ERA`]/
//! [`SPOKEN_CONTRACT`] are the canonical values, and both sides pin the
//! mirror with refusal-message tests.

use std::fmt;

/// The contract era this tebako speaks (spec 18: era 1 is the undeclared
/// pre-era). Mirrored by tebako-bootstrap's own release-card reader (see
/// the module docs); keep the values identical.
pub const SPOKEN_ERA: u32 = 2;

/// The bootstrap↔runtime handoff contract this loader speaks (spec 17's
/// argv/env grammar): **2** in the schema vocabulary
/// (docs/spec/schemas/runtime-manifest.yaml — 1 = spec 06, 2 = spec 17;
/// the roadmap-45 interim numbering is superseded and the factory
/// declares 2 from v0.16.0). A different declared contract is another
/// generation — refused either direction, both numbers named. Canonical
/// value — tebako-bootstrap's `SUPPORTED_CONTRACT` mirrors it (pinned
/// identical by both sides' refusal-message tests).
pub const SPOKEN_CONTRACT: u32 = 2;

/// The contract fields of one release-manifest entry, in refusal-message
/// order (the missing list of a pre-era entry names them all).
pub const CONTRACT_FIELDS: [&str; 3] = ["contract_era", "contract_version", "mount_root"];

/// A release entry's declared contract set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractSet {
    pub era: u32,
    pub contract_version: u32,
    pub mount_root: String,
}

/// The pre-download refusal classes (spec 18 C2; exit 75 at the loader
/// surfaces). Every message names both sides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    /// S11: the entry declares no contract set (any of
    /// [`CONTRACT_FIELDS`] missing, unparseable, or zero) — pre-era.
    PreEra {
        asset: String,
        missing: Vec<&'static str>,
    },
    /// The entry's era is newer than this loader speaks.
    EraTooNew { declared: u32, spoken: u32 },
    /// S12: the entry's contract_version is newer than spoken.
    ContractTooNew { declared: u32, spoken: u32 },
    /// The entry's contract_version is older than spoken — a different
    /// (earlier) handoff generation, just as undrivable (fail-closed
    /// either direction).
    ContractTooOld { declared: u32, spoken: u32 },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::PreEra { asset, missing } => write!(
                f,
                "runtime release is pre-era — its manifest entry for {asset} declares no contract set (missing: {}) — rebuild with the current factory (spec 18 C2)",
                missing.join(", ")
            ),
            ContractError::EraTooNew { declared, spoken } => write!(
                f,
                "runtime release speaks contract era {declared}, this tebako speaks era {spoken} — upgrade tebako"
            ),
            ContractError::ContractTooNew { declared, spoken } => write!(
                f,
                "runtime release declares contract_version {declared}, this tebako speaks contract {spoken} — upgrade tebako (or pin an older runtime)"
            ),
            ContractError::ContractTooOld { declared, spoken } => write!(
                f,
                "runtime release declares contract_version {declared}, this tebako speaks contract {spoken} — the runtime is from an older contract generation; re-resolve a current runtime"
            ),
        }
    }
}

impl std::error::Error for ContractError {}

/// The contract fields an entry actually declares: the missing list of
/// [`CONTRACT_FIELDS`] (absent, unparseable, zero, or an empty
/// mount_root all count as undeclared — never coerced, never defaulted).
fn missing_fields(entry: &tebako_json::Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    let era_ok = entry
        .find("contract_era")
        .and_then(|v| v.as_u64())
        .is_some_and(|v| v > 0);
    if !era_ok {
        missing.push("contract_era");
    }
    let contract_ok = entry
        .find("contract_version")
        .and_then(|v| v.as_u64())
        .is_some_and(|v| v > 0);
    if !contract_ok {
        missing.push("contract_version");
    }
    let mount_ok = entry
        .find("mount_root")
        .and_then(|v| v.as_string())
        .is_some_and(|v| !v.is_empty());
    if !mount_ok {
        missing.push("mount_root");
    }
    missing
}

/// The contract verdict for the manifest entry whose `filename` is
/// `asset`. `Ok(None)` when the manifest carries no entry for the asset
/// at all — not a contract question (the checksum path names that
/// failure). `Err` is the named pre-download refusal; `Ok(Some)` the
/// declared, acceptable set.
pub fn gate(manifest_text: &str, asset: &str) -> Result<Option<ContractSet>, ContractError> {
    let Ok(parsed) = tebako_json::parse(manifest_text) else {
        // An unparseable index declares nothing — the pre-era signal
        // (a corrupt index is indistinguishable from a pre-era one at
        // this layer; the checksum path would fail on it anyway).
        return Err(ContractError::PreEra {
            asset: asset.to_string(),
            missing: CONTRACT_FIELDS.to_vec(),
        });
    };
    let tebako_json::Value::Array(entries) = &parsed else {
        return Err(ContractError::PreEra {
            asset: asset.to_string(),
            missing: CONTRACT_FIELDS.to_vec(),
        });
    };
    let Some(entry) = entries
        .iter()
        .find(|e| e.find("filename").and_then(|f| f.as_string()).as_deref() == Some(asset))
    else {
        return Ok(None);
    };
    let missing = missing_fields(entry);
    if !missing.is_empty() {
        return Err(ContractError::PreEra {
            asset: asset.to_string(),
            missing,
        });
    }
    // Every field validated above; the unwraps cannot fire.
    let era = entry
        .find("contract_era")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let contract_version = entry
        .find("contract_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let mount_root = entry
        .find("mount_root")
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    if era > SPOKEN_ERA {
        return Err(ContractError::EraTooNew {
            declared: era,
            spoken: SPOKEN_ERA,
        });
    }
    if contract_version > SPOKEN_CONTRACT {
        return Err(ContractError::ContractTooNew {
            declared: contract_version,
            spoken: SPOKEN_CONTRACT,
        });
    }
    if contract_version < SPOKEN_CONTRACT {
        return Err(ContractError::ContractTooOld {
            declared: contract_version,
            spoken: SPOKEN_CONTRACT,
        });
    }
    Ok(Some(ContractSet {
        era,
        contract_version,
        mount_root,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASSET: &str = "tebako-runtime-0.16.1-3.4.2-macos-arm64";

    fn manifest(entry_body: &str) -> String {
        format!("[{{\n{entry_body}\n}}]\n")
    }

    fn full_entry() -> String {
        format!(
            "    \"tebako_version\": \"0.16.1\",\n    \"contract_era\": 2,\n    \"contract_version\": 2,\n    \"mount_root\": \"/__tfs__\",\n    \"ruby_version\": \"3.4.2\",\n    \"platform\": \"macos-arm64\",\n    \"filename\": \"{ASSET}\",\n    \"sha256\": \"604e87a1b1d74a6868b35ecdbb11c4e3db01b23286cea9f078636fdf246172b8\""
        )
    }

    #[test]
    fn a_fully_declared_entry_is_accepted() {
        let set = gate(&manifest(&full_entry()), ASSET).unwrap().unwrap();
        assert_eq!(
            set,
            ContractSet {
                era: 2,
                contract_version: 2,
                mount_root: "/__tfs__".to_string(),
            }
        );
    }

    #[test]
    fn an_entry_without_the_contract_fields_is_pre_era() {
        // The pre-18 factory shape: every contract field absent.
        let entry = full_entry()
            .replace("    \"contract_era\": 2,\n", "")
            .replace("    \"contract_version\": 2,\n", "")
            .replace("    \"mount_root\": \"/__tfs__\",\n", "");
        let err = gate(&manifest(&entry), ASSET).unwrap_err();
        assert_eq!(
            err,
            ContractError::PreEra {
                asset: ASSET.to_string(),
                missing: CONTRACT_FIELDS.to_vec(),
            }
        );
        let msg = err.to_string();
        assert!(msg.contains("pre-era"), "{msg}");
        assert!(msg.contains("rebuild with the current factory"), "{msg}");
        assert!(msg.contains(ASSET), "{msg}");
    }

    #[test]
    fn a_missing_contract_version_alone_is_pre_era() {
        // spec 18: treat missing contract_version as era 1 — even when
        // the other two fields ARE declared.
        let entry = full_entry().replace("    \"contract_version\": 2,\n", "");
        let err = gate(&manifest(&entry), ASSET).unwrap_err();
        assert_eq!(
            err,
            ContractError::PreEra {
                asset: ASSET.to_string(),
                missing: vec!["contract_version"],
            }
        );
    }

    #[test]
    fn a_missing_mount_root_is_pre_era_never_a_fallback() {
        // The mount root is known to the ecosystem ("/__tfs__") — the
        // reader must still refuse to invent it (spec 18 C2: declared,
        // never guessed).
        let entry = full_entry().replace("    \"mount_root\": \"/__tfs__\",\n", "");
        let err = gate(&manifest(&entry), ASSET).unwrap_err();
        assert_eq!(
            err,
            ContractError::PreEra {
                asset: ASSET.to_string(),
                missing: vec!["mount_root"],
            }
        );
    }

    #[test]
    fn zero_and_wrong_typed_fields_are_undeclared() {
        let zeroed = full_entry().replace("\"contract_version\": 2", "\"contract_version\": 0");
        assert!(matches!(
            gate(&manifest(&zeroed), ASSET),
            Err(ContractError::PreEra { .. })
        ));
        let stringy = full_entry().replace("\"contract_era\": 2", "\"contract_era\": \"2\"");
        assert!(matches!(
            gate(&manifest(&stringy), ASSET),
            Err(ContractError::PreEra { .. })
        ));
        let empty_root = full_entry().replace("\"/__tfs__\"", "\"\"");
        assert!(matches!(
            gate(&manifest(&empty_root), ASSET),
            Err(ContractError::PreEra { .. })
        ));
    }

    #[test]
    fn a_newer_era_is_an_upgrade_refusal() {
        let entry = full_entry().replace("\"contract_era\": 2", "\"contract_era\": 3");
        let err = gate(&manifest(&entry), ASSET).unwrap_err();
        assert_eq!(
            err,
            ContractError::EraTooNew {
                declared: 3,
                spoken: SPOKEN_ERA
            }
        );
        let msg = err.to_string();
        assert!(msg.contains("era 3"), "{msg}");
        assert!(msg.contains("upgrade tebako"), "{msg}");
    }

    #[test]
    fn a_newer_contract_version_names_both_numbers() {
        let entry = full_entry().replace("\"contract_version\": 2", "\"contract_version\": 3");
        let err = gate(&manifest(&entry), ASSET).unwrap_err();
        assert_eq!(
            err,
            ContractError::ContractTooNew {
                declared: 3,
                spoken: SPOKEN_CONTRACT
            }
        );
        let msg = err.to_string();
        assert!(msg.contains("contract_version 3"), "{msg}");
        assert!(msg.contains("speaks contract 2"), "{msg}");
        // The two refusal classes read differently (spec 18: the message
        // distinguishes "pre-era manifest" from "N > spoken M").
        assert!(!msg.contains("pre-era"), "{msg}");
    }

    #[test]
    fn an_older_contract_generation_is_also_refused() {
        // contract 1 (spec 06 semantics) with a contract-2 loader: a
        // different generation, fail-closed either direction.
        let entry = full_entry().replace("\"contract_version\": 2", "\"contract_version\": 1");
        let err = gate(&manifest(&entry), ASSET).unwrap_err();
        assert_eq!(
            err,
            ContractError::ContractTooOld {
                declared: 1,
                spoken: SPOKEN_CONTRACT
            }
        );
        let msg = err.to_string();
        assert!(msg.contains("contract_version 1"), "{msg}");
        assert!(msg.contains("speaks contract 2"), "{msg}");
        assert!(msg.contains("older contract generation"), "{msg}");
    }

    #[test]
    fn no_entry_for_the_asset_is_not_a_contract_question() {
        assert_eq!(gate(&manifest(&full_entry()), "nope").unwrap(), None);
    }

    #[test]
    fn an_unparseable_manifest_is_the_pre_era_signal() {
        assert!(matches!(
            gate("not json", ASSET),
            Err(ContractError::PreEra { .. })
        ));
        assert!(matches!(
            gate("{\"object\": true}", ASSET),
            Err(ContractError::PreEra { .. })
        ));
    }
}
