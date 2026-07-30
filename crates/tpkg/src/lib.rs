//! The tebako package manifest (tpkg) trailer format.
//!
//! This crate is the **single source of truth** for the tpkg wire format
//! (parse, serialize, validate). It is a byte-exact re-implementation of the
//! reference C99 mini-lib `include/tebako/tpkg.h` in
//! [libtfs](https://github.com/tamatebako/libtfs); the golden-vector tests
//! lock the layout against trailers produced by that C implementation.
//!
//! # Wire layout (v1, all integers little-endian)
//!
//! ```text
//! [payload][slot 0 .. slot n-1 records][trailer header — fixed size, at EOF]
//!
//! trailer header (TPKG_HEADER_SIZE = 166 bytes):
//!   offset  size  field
//!      0    10    magic "TEBAKOTFS\0" (10 bytes, NUL-terminated)
//!     10     4    u32 version (TPKG_VERSION = 1)
//!     14     4    u32 package_flags (bit 0: TPKG_FLAG_LEAN, bit 1: TPKG_FLAG_SIGNED_V2, bit 2: TPKG_FLAG_NO_INSTALL)
//!     18     4    u32 slot_count (1..TPKG_MAX_SLOTS)
//!     22     8    u64 slot_table_offset (absolute file offset of slot 0 record)
//!     30   128    char runtime_ref[128] (UTF-8, NUL-padded; empty = classic bundle)
//!    158     4    u32 launcher_abi
//!    162     4    u32 header_crc32 — tpkg_crc32() over header bytes [0, 162)
//!
//! slot record (TPKG_SLOT_SIZE = 280 bytes):
//!   offset  size  field
//!      0     8    u64 offset (image start, absolute file offset)
//!      8     8    u64 size   (image length in bytes)
//!     16     4    u32 format_id (0=auto, 1=dwarfs, 2=squashfs, 3=zip, 4=runtime)
//!     20     4    u32 flags
//!     24   256    char mount_point[256] (UTF-8, NUL-padded)
//! ```
//!
//! # Wire layout (v2 — the chain-of-trust extension)
//!
//! v2 carries the integrity/authenticity material BETWEEN the slot table
//! and the trailer header, keeping the header exactly where v1 readers
//! expect it (at EOF, version field still 1). v2-ness is marked by
//! `TPKG_FLAG_SIGNED_V2` (bit 1) in package_flags — a flag old readers
//! pass through untouched (the C `tpkg_validate` does not inspect flags,
//! and its bounds check never requires the table to abut the header), so
//! v1-era runtimes keep reading v2 packages exactly as before (they just
//! don't verify), while v2-aware readers enforce the chain. The header
//! keeps the v1 codec exactly (little-endian numerics, crc32 over header
//! bytes [0, 162) — the crc is an accident-integrity check, NOT
//! authenticity); slot records are unchanged as well. All NEW v2
//! extension numerics are big-endian:
//!
//! ```text
//! [payload][slot records][v2 extension][trailer header (166, at EOF)]
//!
//! v2 extension (TPKG_V2_EXT_FIXED = 268 bytes + variable signature):
//!   offset  size  field
//!      0   256    slot sha256 digests: 8 * 32 bytes, digest of slot i's
//!                  bytes at i*32; entries beyond slot_count are zeroed
//!    256     8    signer keyid (low 64 bits of the OpenPGP fingerprint,
//!                  big-endian)
//!    264 sig_len  OpenPGP detached signature (binary packets) over the
//!                  canonical trailer bytes
//! 264+sig_len 4  u32be signature length sig_len (1..TPKG_SIG_MAX)
//!
//! canonical (signed) bytes: slot table || digest array || keyid ||
//! trailer header — the two contiguous spans concatenated (everything
//! except the signature and its length field).
//! ```
//!
//! Detection: parse the v1 header at EOF as always; if
//! `TPKG_FLAG_SIGNED_V2` is set, the extension of exactly
//! `256 + 8 + sig_len + 4` bytes must fill the gap between the slot table
//! and the header (with the digest tail zeroed). A v1 trailer (flag
//! clear) has no extension and still parses exactly as before — v1 =
//! legacy unsigned (see item 29's v1-legacy rule).
//!
//! # Wire layout (typed extension blocks — spec 02 §5b, the L2 home)
//!
//! ```text
//! [bootstrap][payload slots][slot table][ext blocks…][v2 signing ext?][header @EOF]
//!
//! ext block: [u32be type][u32be length][payload bytes]
//!   type 1 = RESERVED for the v2 signing extension — NOT a block: that
//!            layout predates the block mechanism, it is delimited from
//!            the TAIL via its sig_len field (a forward block walker
//!            cannot parse it), and keeping its historical tail position
//!            keeps v2-signed files byte-identical and the canonical
//!            signed region stable. Reserving the type guarantees no
//!            future block collides with a signature in the tail slot.
//!   type 2 = package manifest (YAML — spec 03 §6, see the `package`
//!            module): composition identity, entrypoint/suite entries,
//!            package-level jail + env, per-entry runtime refs
//! ```
//!
//! Blocks walk forward from the end of the slot table; the v2 signing
//! extension, when present, is LAST before the header; extension blocks
//! sit INSIDE the canonical signed region (the v2 signature covers them).
//! Readers skip unknown block types (forward-compat) and carry them
//! verbatim — rewrites preserve blocks they do not understand — while
//! [`Manifest::validate_strict`] rejects unknown types with a named error.
//! A v1/v2 file without blocks parses byte-identically to before (the
//! golden vectors pin this).
//!
//! # Absent vs. corrupt
//!
//! A file whose last-166-byte window does not start with the 4-byte prefix
//! `"TEBA"` is reported as [`TpkgError::NoTrailer`] (a classic bundle without
//! a manifest — not an error condition per se; callers fall back to offset
//! auto-detection). A matching prefix with a mismatching full magic is
//! [`TpkgError::Magic`]; a magic-valid header with a bad crc is
//! [`TpkgError::Crc`].
//!
//! # cbindgen / C header
//!
//! The locked repo strategy (TODO.restructure/21) wants a C `tpkg.h`
//! generated from this crate via cbindgen for the C++ bootstrap/driver.
//! v1 deliberately does **not** wire cbindgen yet: the C++ side currently
//! vendors the self-contained C99 `tpkg.h` (declarations *and*
//! implementation), so nothing C++-side consumes a Rust-generated header
//! today; adding the pipeline now would produce an unchecked artifact.
//! The generation target lands together with `crates/tebako-bootstrap`
//! (item 22), the first real consumer. The wire layout is meanwhile pinned
//! by the golden vectors in `crates/tpkg/tests/golden.rs`, which are
//! byte-exact outputs of the C implementation.
//!
//! # The payload manifest (spec 03)
//!
//! Besides the trailer, this crate owns the second tpkg surface: the
//! in-image **payload manifest** at [`PAYLOAD_MANIFEST_PATH`] — authored
//! YAML (never JSON, owner rule), IDENTITY + PROVIDES + DEPENDS on a
//! common provenance/trust layer. See [`manifest`]'s module docs and the
//! versioned JSON Schema `schema/tpkg-manifest-v1.schema.json`. The two
//! surfaces have separate error types: [`TpkgError`] is 1:1 with the C
//! `TPKG_ERR_*` codes while the payload manifest has no C counterpart
//! ([`ManifestError`]).

#![forbid(unsafe_code)]

mod codec;
mod crc32;
mod envelope;
mod error;
mod ext;
mod io;
pub mod jail;
mod manifest;
pub mod merkle;
pub mod merkle_host;
mod model;
mod package;

pub use codec::{
    encode_ext_blocks, encode_trailer, parse_ext_blocks, parse_trailer, trailer_len,
    v2_signed_region,
};
pub use crc32::{crc32, Crc32};
pub use envelope::{EnvelopeManifest, Grant, Suite, ENVELOPES_PATH, ENVELOPES_SCHEMA_VERSION};
pub use error::{strerror, TpkgError};
pub use ext::{ExtBlock, ExtError};
pub use io::{read_from, write_to};
pub use jail::{ArgumentFiles, HostJail, JailAccess, JailError, JailMount};
pub use manifest::{
    AppProvides, BuiltFrom, Capabilities, Constraint, DataProvides, Digest, Encryption,
    EncryptionPart, EncryptionState, EngineProvides, Entrypoint, Identity, ManifestError,
    MountSemantics, PayloadKind, PayloadManifest, Platform, Platforms, Producer, Provides,
    Requirement, RuntimeProvides, RuntimeRequirement, Sbom, Signing, SigningMechanism,
    SigningState, Source, ToolkitExecutable, ToolkitLibrary, ToolkitProvides,
    PAYLOAD_MANIFEST_PATH, PAYLOAD_SCHEMA_VERSION,
};
pub use merkle::{render_tree_hash, tree_digest, Child, MerkleDigest, NodeKind, TreeWalk};
pub use model::{Manifest, Slot, V2Extension};
pub use package::{
    PackageEntry, PackageIdentity, PackageManifest, PackageManifestError, PACKAGE_SCHEMA_VERSION,
};

/// Manifest format version (stays 1: the chain-of-trust extension is
/// flagged via `TPKG_FLAG_SIGNED_V2`, not a version bump, so v1-era
/// readers keep working).
pub const TPKG_VERSION: u32 = 1;
/// Maximum number of payload slots.
pub const TPKG_MAX_SLOTS: u32 = 8;
/// Trailer header size in bytes.
pub const TPKG_HEADER_SIZE: usize = 166;
/// Slot record size in bytes.
pub const TPKG_SLOT_SIZE: usize = 280;
/// Fixed width of the `mount_point` field.
pub const TPKG_MOUNT_POINT_LEN: usize = 256;
/// Fixed width of the `runtime_ref` field.
pub const TPKG_RUNTIME_REF_LEN: usize = 128;
/// Trailer magic: "TEBAKOTFS\0" (10 bytes, NUL-terminated).
pub const TPKG_MAGIC: &[u8; 10] = b"TEBAKOTFS\0";
/// Magic length including the terminating NUL.
pub const TPKG_MAGIC_LEN: usize = 10;
/// "TEBA" prefix length: the absent-vs-corrupt discriminator.
pub const TPKG_MAGIC_PREFIX_LEN: usize = 4;

/// `package_flags` bit 0: lean package (bootstrap+images only, runtime
/// resolved at run time).
pub const TPKG_FLAG_LEAN: u32 = 0x1;
/// `package_flags` bit 1: the package carries the v2 chain-of-trust
/// extension between the slot table and the trailer header (item 29).
/// Old readers pass the bit through untouched — that is what keeps v2
/// packages readable by v1-era runtimes.
pub const TPKG_FLAG_SIGNED_V2: u32 = 0x2;
/// `package_flags` bit 2: the publisher froze this package — it RUNS
/// standalone but every install attempt (`--tebako-install`,
/// `tebako install <path>`) is refused with a named error (TODO.v2-1/12).
/// Absence means installable-on-request, including packages written
/// before the install verb existed — pass-through like every flag.
pub const TPKG_FLAG_NO_INSTALL: u32 = 0x4;

/// `format_id`: auto-detect from image magic.
pub const TPKG_FORMAT_AUTO: u32 = 0;
/// `format_id`: DwarFS image.
pub const TPKG_FORMAT_DWARFS: u32 = 1;
/// `format_id`: SquashFS image.
pub const TPKG_FORMAT_SQUASHFS: u32 = 2;
/// `format_id`: ZIP archive.
pub const TPKG_FORMAT_ZIP: u32 = 3;
/// `format_id`: runtime payload slot (fat packages).
pub const TPKG_FORMAT_RUNTIME: u32 = 4;

// ---------------------------------------------------------------------
// Typed extension blocks (spec 02 §5b; all block numerics BIG-ENDIAN,
// like the v2 extension's). Blocks sit between the slot table and the
// v2 extension / trailer header; readers skip unknown types, rewrites
// preserve them, `Manifest::validate_strict` rejects them.
// ---------------------------------------------------------------------

/// Extension block header size: u32be type + u32be length.
pub const TPKG_EXT_HEADER_SIZE: usize = 8;
/// Extension block type 1: RESERVED for the v2 chain-of-trust extension.
/// Never a block — the signing extension predates the block mechanism, is
/// self-delimiting from the tail via its sig_len field, and keeps its
/// historical position immediately before the trailer header.
pub const TPKG_EXT_TYPE_V2_SIGNING: u32 = 1;
/// Extension block type 2: the L2 package manifest (YAML, spec 03 §6).
pub const TPKG_EXT_TYPE_PACKAGE_MANIFEST: u32 = 2;

// ---------------------------------------------------------------------
// v2 chain-of-trust extension (all v2-extension numerics BIG-ENDIAN;
// the 166-byte trailer header keeps the v1 codec — little-endian —
// unchanged, as do the slot records; see the module docs)
// ---------------------------------------------------------------------

/// Length of one slot's SHA-256 digest in bytes.
pub const TPKG_SHA256_LEN: usize = 32;
/// Size of the per-slot digest array (one digest per possible slot).
pub const TPKG_DIGESTS_SIZE: usize = TPKG_MAX_SLOTS as usize * TPKG_SHA256_LEN;
/// Length of the signer key id (low 64 bits of the OpenPGP fingerprint).
pub const TPKG_KEYID_LEN: usize = 8;
/// Size of the big-endian signature-length field.
pub const TPKG_SIGLEN_SIZE: usize = 4;
/// Fixed part of the v2 extension (digests + keyid + siglen, signature
/// bytes excluded).
pub const TPKG_V2_EXT_FIXED: usize = TPKG_DIGESTS_SIZE + TPKG_KEYID_LEN + TPKG_SIGLEN_SIZE;
/// Maximum accepted signature size (sanity bound, not a format limit).
pub const TPKG_SIG_MAX: u32 = 65536;

/// Header field offsets (see the module docs).
pub(crate) mod off {
    pub const MAGIC: usize = 0;
    pub const VERSION: usize = 10;
    pub const PACKAGE_FLAGS: usize = 14;
    pub const SLOT_COUNT: usize = 18;
    pub const TABLE: usize = 22;
    pub const RUNTIME_REF: usize = 30;
    pub const LAUNCHER_ABI: usize = 158;
    pub const CRC32: usize = 162;
}

/// Slot record field offsets (see the module docs).
pub(crate) mod rec {
    pub const OFFSET: usize = 0;
    pub const SIZE: usize = 8;
    pub const FORMAT: usize = 16;
    pub const FLAGS: usize = 20;
    pub const MOUNT: usize = 24;
}
