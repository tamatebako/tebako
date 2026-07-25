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
//!     14     4    u32 package_flags (bit 0: TPKG_FLAG_LEAN)
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

#![forbid(unsafe_code)]

mod codec;
mod crc32;
mod error;
mod io;
mod model;

pub use codec::{encode_trailer, parse_trailer};
pub use crc32::crc32;
pub use error::{strerror, TpkgError};
pub use io::{read_from, write_to};
pub use model::{Manifest, Slot};

/// Manifest format version.
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
