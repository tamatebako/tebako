//! The spec-18 contract layer of the L2 package manifest (C6): the era
//! declaration every package pressed from now on carries in its type-2
//! extension block, and the fail-closed verification a reader performs on
//! open (exit 77).
//!
//! The type-2 block is ONE YAML document — its grammar owned by
//! `docs/spec/schemas/package-manifest.yaml` — with two typed views:
//! [`PackageManifest`] (composition — the era-1 model, unchanged) and
//! [`PackageContract`] (the era declaration this module owns). Packages
//! pressed by tebako < 0.16.1 carry no contract fields: era 1, refused by
//! name — never assumed, never silently served (spec 18, the law).
//!
//! The writer side rides [`Manifest::set_package_manifest`]
//! (crate::ext): the ONLY write path for the type-2 block, so every press
//! emits the declaration from one point.

use std::fmt;

use serde::Deserialize;

use crate::model::Manifest;
use crate::package::{PackageManifest, PackageManifestError};
use crate::{EX_TEBAKO_CONTRACT_ERA, TPKG_CONTRACT_ERA, TPKG_EXT_TYPE_PACKAGE_MANIFEST};

/// The era declaration of a package (the contract fields of the type-2
/// package-manifest block — `docs/spec/schemas/package-manifest.yaml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageContract {
    /// The contract era the package was pressed under (>= 2 once
    /// declared; era 1 is the undeclared pre-era).
    pub contract_era: u32,
    /// The tebako product version that pressed the package.
    pub pressed_by: String,
    /// The minimum reader era the package demands.
    pub reader_era: u32,
}

/// The tolerant serde view over the type-2 document: every contract field
/// optional, every other key ignored (the composition view owns them).
#[derive(Deserialize)]
struct ContractView {
    contract_era: Option<u32>,
    pressed_by: Option<String>,
    reader_era: Option<u32>,
}

impl PackageContract {
    /// The declaration this build presses: the current era, this crate's
    /// version (the workspace-unified tebako product version) as
    /// `pressed_by`, and the current era as the minimum reader era.
    pub fn current() -> PackageContract {
        PackageContract {
            contract_era: TPKG_CONTRACT_ERA,
            pressed_by: env!("CARGO_PKG_VERSION").to_string(),
            reader_era: TPKG_CONTRACT_ERA,
        }
    }

    /// Parse the contract fields of a type-2 block's YAML document;
    /// `Ok(None)` when the document declares no `contract_era` (a pre-era
    /// block). A document declaring `contract_era` but omitting
    /// `pressed_by` or `reader_era` is a partial declaration — malformed,
    /// never a silent default (fail-closed).
    pub fn from_yaml(text: &str) -> Result<Option<PackageContract>, ContractError> {
        let view: ContractView = serde_yml::from_str(text)
            .map_err(|e| ContractError::Malformed(format!("package manifest yaml error: {e}")))?;
        let Some(era) = view.contract_era else {
            return Ok(None);
        };
        let (Some(pressed_by), Some(reader_era)) = (view.pressed_by, view.reader_era) else {
            return Err(ContractError::Malformed(
                "partial contract block: contract_era is present but pressed_by/reader_era are missing"
                    .to_string(),
            ));
        };
        if pressed_by.is_empty() {
            return Err(ContractError::Malformed(
                "contract block pressed_by must not be empty".to_string(),
            ));
        }
        Ok(Some(PackageContract {
            contract_era: era,
            pressed_by,
            reader_era,
        }))
    }
}

/// Serialize a package manifest as the type-2 block payload WITH the
/// contract declaration merged in (the writer's form): the contract keys
/// ride directly after `schema_version`. The composition model never
/// carries them — the block's YAML document is the grammar, with the
/// composition and contract views over it.
pub(crate) fn block_payload_with_contract(
    manifest: &PackageManifest,
    contract: &PackageContract,
) -> Result<String, PackageManifestError> {
    manifest.validate()?;
    let value = serde_yml::to_value(manifest)?;
    let mut out = serde_yml::Mapping::new();
    if let serde_yml::Value::Mapping(map) = value {
        for (k, v) in map {
            let after_version = k.as_str() == Some("schema_version");
            out.insert(k, v);
            if after_version {
                out.insert(
                    serde_yml::Value::from("contract_era"),
                    serde_yml::Value::from(contract.contract_era),
                );
                out.insert(
                    serde_yml::Value::from("pressed_by"),
                    serde_yml::Value::from(contract.pressed_by.as_str()),
                );
                out.insert(
                    serde_yml::Value::from("reader_era"),
                    serde_yml::Value::from(contract.reader_era),
                );
            }
        }
    }
    Ok(serde_yml::to_string(&serde_yml::Value::Mapping(out))?)
}

/// A contract refusal of the spec-18 C6 verification (fail-closed, on
/// open — never after exec). Every variant maps to exit 77
/// ([`EX_TEBAKO_CONTRACT_ERA`]) — the loader's named code for
/// package/payload contract-era failure (spec 18 §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    /// Era 1: the package carries no type-2 block, or a block without
    /// `contract_era` (pressed by tebako < 0.16.1).
    PreEra,
    /// The type-2 block is present but its contract declaration is
    /// partial or corrupt (fail-closed — never a silent default).
    Malformed(String),
    /// The package's `contract_era` is newer than the era this reader
    /// speaks (S1): the package's era and the reader's era.
    EraTooNew { package_era: u32, reader_era: u32 },
    /// The package's minimum reader era (`reader_era`) exceeds the era
    /// this reader speaks: the demanded era and the reader's era.
    ReaderTooOld { demanded_era: u32, reader_era: u32 },
    /// An extension block of an unknown base type carries the critical
    /// flag (spec 18 §3.7 / S10): refused by name instead of skipped.
    CriticalBlock(u32),
}

impl ContractError {
    /// The loader exit code for every contract refusal (spec 18 §7).
    pub fn exit_code(&self) -> i32 {
        EX_TEBAKO_CONTRACT_ERA
    }
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::PreEra => write!(
                f,
                "pre-era package — re-press with tebako ≥ 0.16.1 or run it with tebako v1"
            ),
            ContractError::Malformed(m) => write!(f, "invalid package contract block: {m}"),
            ContractError::EraTooNew {
                package_era,
                reader_era,
            } => write!(
                f,
                "package from a newer tebako (era {package_era}) — upgrade your tebako (speaks era {reader_era})"
            ),
            ContractError::ReaderTooOld {
                demanded_era,
                reader_era,
            } => write!(
                f,
                "package requires a tebako reader of era ≥ {demanded_era} — upgrade your tebako (speaks era {reader_era})"
            ),
            ContractError::CriticalBlock(t) => write!(
                f,
                "unknown critical extension block type {t} — upgrade your tebako (a reader that does not understand a critical block refuses, never skips)"
            ),
        }
    }
}

impl std::error::Error for ContractError {}

impl Manifest {
    /// The package's era declaration (the contract view of the type-2
    /// block); `Ok(None)` when the package carries no type-2 block or a
    /// block without `contract_era` (era 1).
    pub fn package_contract(&self) -> Result<Option<PackageContract>, ContractError> {
        let Some(block) = self.ext_block(TPKG_EXT_TYPE_PACKAGE_MANIFEST) else {
            return Ok(None);
        };
        let text = std::str::from_utf8(&block.payload)
            .map_err(|_| ContractError::Malformed("extension block is not UTF-8".to_string()))?;
        PackageContract::from_yaml(text)
    }

    /// The spec-18 C6 verification (fail-closed, on open): unknown
    /// CRITICAL extension blocks refuse (§3.7 — non-critical unknowns
    /// still skip, invariant 7), then the era negotiation runs both
    /// directions against this reader's era ([`TPKG_CONTRACT_ERA`]).
    pub fn verify_contract(&self) -> Result<(), ContractError> {
        for b in &self.ext_blocks {
            if b.is_critical() && b.base_type() != TPKG_EXT_TYPE_PACKAGE_MANIFEST {
                return Err(ContractError::CriticalBlock(b.base_type()));
            }
        }
        let Some(contract) = self.package_contract()? else {
            return Err(ContractError::PreEra);
        };
        // A declared era 1 is the pre-era by another spelling.
        if contract.contract_era < 2 {
            return Err(ContractError::PreEra);
        }
        if contract.contract_era > TPKG_CONTRACT_ERA {
            return Err(ContractError::EraTooNew {
                package_era: contract.contract_era,
                reader_era: TPKG_CONTRACT_ERA,
            });
        }
        if contract.reader_era > TPKG_CONTRACT_ERA {
            return Err(ContractError::ReaderTooOld {
                demanded_era: contract.reader_era,
                reader_era: TPKG_CONTRACT_ERA,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Slot, TPKG_FORMAT_ZIP};

    fn one_slot_manifest() -> Manifest {
        let mut m = Manifest::default();
        m.slots.push(Slot::new(0, 100, TPKG_FORMAT_ZIP, "/m"));
        m
    }

    fn block_with(yaml: &str) -> Manifest {
        let mut m = one_slot_manifest();
        m.ext_blocks.push(
            crate::ExtBlock::new(TPKG_EXT_TYPE_PACKAGE_MANIFEST, yaml.as_bytes().to_vec()).unwrap(),
        );
        m
    }

    const PRE_ERA_YAML: &str = "schema_version: 1\n\
         package: {name: metanorma, version: 1.2.3, producer: {tool: tebako-cli, tool_version: 0.16.0}, created: 2026-07-26T00:00:00Z}\n\
         entries:\n  - {name: metanorma, slot: 0, entrypoint: metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n";

    const ERA2_YAML: &str = "schema_version: 1\n\
         contract_era: 2\npressed_by: 0.16.1\nreader_era: 2\n\
         package: {name: metanorma, version: 1.2.3, producer: {tool: tebako-cli, tool_version: 0.16.1}, created: 2026-08-01T00:00:00Z}\n\
         entries:\n  - {name: metanorma, slot: 0, entrypoint: metanorma, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n";

    #[test]
    fn current_declares_the_spoken_era() {
        let c = PackageContract::current();
        assert_eq!(c.contract_era, TPKG_CONTRACT_ERA);
        assert_eq!(c.reader_era, TPKG_CONTRACT_ERA);
        assert!(!c.pressed_by.is_empty());
    }

    #[test]
    fn from_yaml_distinguishes_pre_era_current_and_partial() {
        assert_eq!(PackageContract::from_yaml(PRE_ERA_YAML).unwrap(), None);
        let c = PackageContract::from_yaml(ERA2_YAML).unwrap().unwrap();
        assert_eq!(c.contract_era, 2);
        assert_eq!(c.pressed_by, "0.16.1");
        assert_eq!(c.reader_era, 2);
        // partial declaration: malformed, never a silent default
        assert!(matches!(
            PackageContract::from_yaml("schema_version: 1\ncontract_era: 2\n"),
            Err(ContractError::Malformed(_))
        ));
        // garbage: malformed
        assert!(matches!(
            PackageContract::from_yaml("schema_version: [1\n"),
            Err(ContractError::Malformed(_))
        ));
    }

    #[test]
    fn verify_contract_refuses_pre_era_both_spellings() {
        // no block at all
        let m = one_slot_manifest();
        assert_eq!(m.verify_contract(), Err(ContractError::PreEra));
        // a block without contract_era
        let m = block_with(PRE_ERA_YAML);
        assert_eq!(m.verify_contract(), Err(ContractError::PreEra));
        // a declared era 1 is the pre-era by another spelling
        let m = block_with("schema_version: 1\ncontract_era: 1\npressed_by: 0.9\nreader_era: 1\n");
        assert_eq!(m.verify_contract(), Err(ContractError::PreEra));
        // the refusal carries the S2 message and the 77 code
        let e = ContractError::PreEra;
        assert_eq!(e.exit_code(), 77);
        assert!(e.to_string().contains("re-press with tebako ≥ 0.16.1"));
    }

    #[test]
    fn verify_contract_negotiates_both_directions() {
        // current era: accepted
        let m = block_with(ERA2_YAML);
        m.verify_contract().unwrap();

        // package era newer than the reader speaks (S1): both printed
        let m =
            block_with("schema_version: 1\ncontract_era: 3\npressed_by: 0.20.0\nreader_era: 2\n");
        let err = m.verify_contract().unwrap_err();
        assert_eq!(
            err,
            ContractError::EraTooNew {
                package_era: 3,
                reader_era: TPKG_CONTRACT_ERA
            }
        );
        assert_eq!(
            err.to_string(),
            "package from a newer tebako (era 3) — upgrade your tebako (speaks era 2)"
        );

        // the package demands a newer reader: both printed
        let m =
            block_with("schema_version: 1\ncontract_era: 2\npressed_by: 0.16.1\nreader_era: 3\n");
        let err = m.verify_contract().unwrap_err();
        assert_eq!(
            err,
            ContractError::ReaderTooOld {
                demanded_era: 3,
                reader_era: TPKG_CONTRACT_ERA
            }
        );
        assert!(err.to_string().contains("era ≥ 3"), "{err}");
        assert!(err.to_string().contains("speaks era 2"), "{err}");
    }

    #[test]
    fn verify_contract_refuses_unknown_critical_blocks_but_skips_the_rest() {
        // unknown non-critical blocks skip (invariant 7)
        let mut m = block_with(ERA2_YAML);
        m.ext_blocks
            .push(crate::ExtBlock::new(7, b"future".to_vec()).unwrap());
        m.verify_contract().unwrap();

        // the same block marked critical refuses (spec 18 §3.7 / S10)
        let mut m = block_with(ERA2_YAML);
        m.ext_blocks
            .push(crate::ExtBlock::new_critical(7, b"future".to_vec()).unwrap());
        assert_eq!(m.verify_contract(), Err(ContractError::CriticalBlock(7)));
        let e = ContractError::CriticalBlock(7);
        assert_eq!(e.exit_code(), 77);
        assert!(e.to_string().contains("critical extension block type 7"));
    }

    #[test]
    fn malformed_block_is_a_named_refusal_not_a_parse_crash() {
        // unparseable YAML: a malformed declaration (fail-closed)
        let m = block_with("schema_version: [1\n");
        let err = m.verify_contract().unwrap_err();
        assert!(matches!(err, ContractError::Malformed(_)), "{err}");
        // a partial contract declaration is malformed too
        let m = block_with("schema_version: 1\ncontract_era: 2\n");
        assert!(matches!(
            m.verify_contract(),
            Err(ContractError::Malformed(_))
        ));
        // a block the COMPOSITION model would reject (schema_version 99)
        // but which declares no contract is a pre-era block to this layer
        // (composition validation is PackageManifestError's domain)
        let m = block_with("schema_version: 99\n");
        assert_eq!(m.verify_contract(), Err(ContractError::PreEra));
    }

    #[test]
    fn the_writer_merges_the_contract_keys_after_schema_version() {
        let pm = PackageManifest::from_yaml(PRE_ERA_YAML).unwrap();
        let yaml = block_payload_with_contract(&pm, &PackageContract::current()).unwrap();
        // the emitted shape: schema_version, then the contract triple
        let head = yaml.lines().take(4).collect::<Vec<_>>();
        assert_eq!(head[0], "schema_version: 1");
        assert_eq!(head[1], format!("contract_era: {TPKG_CONTRACT_ERA}"));
        assert!(head[2].starts_with("pressed_by: "), "{yaml}");
        assert_eq!(head[3], format!("reader_era: {TPKG_CONTRACT_ERA}"));
        // …and the composition rides unchanged after them
        assert!(yaml.contains("package:"), "{yaml}");
        assert!(yaml.contains("entries:"), "{yaml}");
        // the composition view round-trips (it tolerates the contract keys)
        let back = PackageManifest::from_yaml(&yaml).unwrap();
        assert_eq!(back, pm);
        // the contract view reads the same document
        let c = PackageContract::from_yaml(&yaml).unwrap().unwrap();
        assert_eq!(c, PackageContract::current());
    }

    #[test]
    fn set_package_manifest_emits_the_contract_and_verifies() {
        let pm = PackageManifest::from_yaml(PRE_ERA_YAML).unwrap();
        let mut m = one_slot_manifest();
        m.set_package_manifest(&pm).unwrap();
        // a freshly pressed package passes its own contract gate
        m.verify_contract().unwrap();
        let c = m.package_contract().unwrap().unwrap();
        assert_eq!(c.contract_era, TPKG_CONTRACT_ERA);
        // …and the composition view is unchanged by the contract keys
        assert_eq!(m.package_manifest().unwrap(), Some(pm));
    }
}
