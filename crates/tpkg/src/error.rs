//! Error codes, 1:1 with the C implementation's `TPKG_ERR_*` values.

use std::fmt;

/// Error returned by all fallible tpkg operations.
///
/// The `code()` values are identical to the C implementation's `TPKG_ERR_*`
/// constants, and `Display` produces the same strings as `tpkg_strerror()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpkgError {
    /// No manifest trailer present (absent — not an error per se).
    NoTrailer,
    /// Magic prefix present but full magic mismatch (corrupt).
    Magic,
    /// `header_crc32` mismatch (corrupt).
    Crc,
    /// Underlying i/o failure.
    Io,
    /// Slot table outside file bounds.
    Bounds,
    /// `slot_count` == 0 or > `TPKG_MAX_SLOTS`.
    Slots,
    /// Structural validation failure.
    Invalid,
    /// Invalid argument.
    Arg,
    /// Unsupported manifest version.
    Version,
}

impl TpkgError {
    /// The numeric `TPKG_ERR_*` code (identical to the C implementation).
    pub fn code(self) -> i32 {
        match self {
            TpkgError::NoTrailer => 1,
            TpkgError::Magic => 2,
            TpkgError::Crc => 3,
            TpkgError::Io => 4,
            TpkgError::Bounds => 5,
            TpkgError::Slots => 6,
            TpkgError::Invalid => 7,
            TpkgError::Arg => 8,
            TpkgError::Version => 9,
        }
    }
}

/// Static string for a `TPKG_ERR_*` code — byte-identical to the C
/// implementation's `tpkg_strerror()` (including the `0` and unknown cases).
pub fn strerror(err: i32) -> &'static str {
    match err {
        0 => "success",
        1 => "no tpkg manifest trailer present",
        2 => "corrupt tpkg trailer magic",
        3 => "tpkg trailer header crc32 mismatch",
        4 => "tpkg i/o error",
        5 => "tpkg slot table out of file bounds",
        6 => "tpkg slot count out of range (1..TPKG_MAX_SLOTS)",
        7 => "invalid tpkg manifest structure",
        8 => "invalid tpkg argument",
        9 => "unsupported tpkg manifest version",
        _ => "unknown tpkg error",
    }
}

impl fmt::Display for TpkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(strerror(self.code()))
    }
}

impl std::error::Error for TpkgError {}
