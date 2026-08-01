//! Typed extension blocks (spec 02 §5b): the L2 home between the slot
//! table and the trailer header.
//!
//! Wire form: `[u32be type][u32be length][payload bytes]`, walked forward
//! from the end of the slot table (type+length self-delimit). The v2
//! signing extension — when present — stays LAST before the header,
//! self-delimiting from the tail via its `sig_len` field, with unchanged
//! bytes and position.
//!
//! Block type 1 is RESERVED for that signing extension and is never
//! reframed as a block: the signing layout predates the block mechanism,
//! it is delimited from the TAIL (a forward block walker cannot parse
//! it), and keeping its historical position keeps v2-signed files
//! byte-identical and the canonical signed region (spec 02 §4) stable.
//! Reserving the type also guarantees no future block collides with a
//! v2 signature sitting in its tail slot.
//!
//! Readers skip unknown block types (forward-compat) and carry them
//! verbatim, so rewrites preserve blocks they do not understand;
//! [`Manifest::validate_strict`] is the fail-closed gate that rejects
//! unknown types with a named error. Type 2 is the L2 package manifest
//! (spec 03 §6 — see [`crate::package`]; its spec-18 contract layer is
//! [`crate::contract`]).
//!
//! The type word's high bit ([`TPKG_EXT_FLAG_CRITICAL`]) marks a block
//! CRITICAL (spec 18 §3.7): a reader that does not know the base type
//! refuses instead of skipping (S10). The bit is wire-compatible — only
//! blocks that opt in change bytes.

use std::fmt;

use crate::error::TpkgError;
use crate::model::Manifest;
use crate::package::{PackageManifest, PackageManifestError};
use crate::{
    PackageContract, TPKG_EXT_FLAG_CRITICAL, TPKG_EXT_HEADER_SIZE, TPKG_EXT_TYPE_PACKAGE_MANIFEST,
    TPKG_EXT_TYPE_V2_SIGNING,
};

/// One typed extension block (spec 02 §5b). The payload is carried
/// verbatim; only type 2 has an interpreted form in this build.
/// `block_type` is the raw wire word (base type | critical flag) — use
/// [`ExtBlock::base_type`] for dispatch and [`ExtBlock::is_critical`] for
/// the spec 18 §3.7 rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtBlock {
    /// The raw block type word (`TPKG_EXT_TYPE_*` base type, possibly
    /// ORed with [`TPKG_EXT_FLAG_CRITICAL`]; base type 1 is reserved for
    /// the v2 signing extension and never appears in a validated
    /// manifest).
    pub block_type: u32,
    /// The payload bytes (`length` on the wire).
    pub payload: Vec<u8>,
}

impl ExtBlock {
    /// A block of a known-free type. Base type 1 is reserved for the v2
    /// signing extension and can never be constructed as a block (with or
    /// without the critical flag).
    pub fn new(block_type: u32, payload: Vec<u8>) -> Result<ExtBlock, ExtError> {
        if block_type & !TPKG_EXT_FLAG_CRITICAL == TPKG_EXT_TYPE_V2_SIGNING {
            return Err(ExtError::ReservedType);
        }
        Ok(ExtBlock {
            block_type,
            payload,
        })
    }

    /// A CRITICAL block of a known-free base type (spec 18 §3.7): a
    /// reader that does not understand it refuses instead of skipping.
    pub fn new_critical(base_type: u32, payload: Vec<u8>) -> Result<ExtBlock, ExtError> {
        ExtBlock::new(base_type | TPKG_EXT_FLAG_CRITICAL, payload)
    }

    /// The type word without the critical flag — the dispatch key.
    pub fn base_type(&self) -> u32 {
        self.block_type & !TPKG_EXT_FLAG_CRITICAL
    }

    /// True when the block carries the critical flag (spec 18 §3.7).
    pub fn is_critical(&self) -> bool {
        self.block_type & TPKG_EXT_FLAG_CRITICAL != 0
    }

    /// On-disk size: the 8-byte block header + the payload.
    pub fn encoded_len(&self) -> usize {
        TPKG_EXT_HEADER_SIZE + self.payload.len()
    }
}

/// Error of the extension-block surface.
///
/// Deliberately separate from [`TpkgError`]: `TpkgError`'s codes are 1:1
/// with the C implementation's `TPKG_ERR_*` values and the block
/// mechanism has no C counterpart (the same discipline as
/// [`crate::ManifestError`]).
#[derive(Debug)]
pub enum ExtError {
    /// A structural trailer problem surfaced by strict validation (it
    /// re-runs the [`Manifest::validate`] gate first).
    Trailer(TpkgError),
    /// Block type 1 is reserved for the v2 signing extension — it keeps
    /// its historical tail position and is never a block.
    ReservedType,
    /// Strict validation met a block type this build does not know:
    /// readers skip unknown types (forward-compat), validation refuses
    /// them (fail-closed).
    UnknownType(u32),
    /// Strict validation met a CRITICAL block type this build does not
    /// know (spec 18 §3.7): refused by name, never skipped.
    CriticalType(u32),
}

impl fmt::Display for ExtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtError::Trailer(e) => write!(f, "{e}"),
            ExtError::ReservedType => write!(
                f,
                "tpkg extension block type 1 is reserved for the v2 signing extension \
                 (it keeps its historical tail position and is never a block)"
            ),
            ExtError::UnknownType(t) => write!(
                f,
                "unknown tpkg extension block type {t} \
                 (readers skip unknown types; strict validation rejects them)"
            ),
            ExtError::CriticalType(t) => write!(
                f,
                "unknown CRITICAL tpkg extension block type {t} \
                 (a reader that does not understand a critical block refuses, never skips — spec 18 §3.7)"
            ),
        }
    }
}

impl std::error::Error for ExtError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExtError::Trailer(e) => Some(e),
            _ => None,
        }
    }
}

impl Manifest {
    /// The first extension block of `block_type` (a base type — the
    /// critical flag does not discriminate), in wire order.
    pub fn ext_block(&self, block_type: u32) -> Option<&ExtBlock> {
        self.ext_blocks.iter().find(|b| b.base_type() == block_type)
    }

    /// Insert a block, replacing any existing block of the same base type
    /// (block types are singletons in v1 — one package manifest at most),
    /// appending in wire order otherwise. Type 1 is reserved
    /// ([`ExtError::ReservedType`]).
    pub fn insert_ext_block(&mut self, block: ExtBlock) -> Result<(), ExtError> {
        if block.base_type() == TPKG_EXT_TYPE_V2_SIGNING {
            return Err(ExtError::ReservedType);
        }
        match self
            .ext_blocks
            .iter()
            .position(|b| b.base_type() == block.base_type())
        {
            Some(i) => self.ext_blocks[i] = block,
            None => self.ext_blocks.push(block),
        }
        Ok(())
    }

    /// Remove every block of `block_type` (a base type); true when any
    /// was removed.
    pub fn remove_ext_block(&mut self, block_type: u32) -> bool {
        let before = self.ext_blocks.len();
        self.ext_blocks.retain(|b| b.base_type() != block_type);
        self.ext_blocks.len() != before
    }

    /// Strict validation: the forward-compat gate (spec 02 §5b). Readers
    /// skip unknown block types; validation refuses them. Every block
    /// type must be known to this build (type 2 today) — the reserved
    /// type 1 and any unknown type are named errors, with unknown
    /// CRITICAL blocks named separately (spec 18 §3.7). The structural
    /// [`Manifest::validate`] gate runs as well.
    pub fn validate_strict(&self) -> Result<(), ExtError> {
        for b in &self.ext_blocks {
            match b.base_type() {
                TPKG_EXT_TYPE_V2_SIGNING => return Err(ExtError::ReservedType),
                TPKG_EXT_TYPE_PACKAGE_MANIFEST => {}
                other if b.is_critical() => return Err(ExtError::CriticalType(other)),
                other => return Err(ExtError::UnknownType(other)),
            }
        }
        self.validate().map_err(ExtError::Trailer)
    }

    /// The L2 package manifest (ext block type 2, spec 03 §6), parsed and
    /// validated; `None` when the package carries no such block.
    pub fn package_manifest(&self) -> Result<Option<PackageManifest>, PackageManifestError> {
        let Some(block) = self.ext_block(TPKG_EXT_TYPE_PACKAGE_MANIFEST) else {
            return Ok(None);
        };
        let text = std::str::from_utf8(&block.payload)
            .map_err(|_| PackageManifestError::Invalid("extension block is not UTF-8"))?;
        PackageManifest::from_yaml(text).map(Some)
    }

    /// Embed a package manifest as the type-2 extension block, replacing
    /// any existing one. The manifest is validated and serialized to YAML
    /// (the only authored-manifest format — owner rule) WITH the spec-18
    /// contract declaration merged in ([`crate::PackageContract::current`]
    /// — every press declares its era from this single write point; a
    /// freshly written block passes [`Manifest::verify_contract`]).
    pub fn set_package_manifest(
        &mut self,
        manifest: &PackageManifest,
    ) -> Result<(), PackageManifestError> {
        let yaml =
            crate::contract::block_payload_with_contract(manifest, &PackageContract::current())?;
        self.ext_blocks
            .retain(|b| b.base_type() != TPKG_EXT_TYPE_PACKAGE_MANIFEST);
        self.ext_blocks.push(ExtBlock {
            block_type: TPKG_EXT_TYPE_PACKAGE_MANIFEST,
            payload: yaml.into_bytes(),
        });
        Ok(())
    }
}
