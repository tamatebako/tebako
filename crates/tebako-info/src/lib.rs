//! tebako-info — the spec-15 info surface (payload and package
//! introspection) shared by the two MECE front-ends:
//!
//! - `tfs info` (tfs-cli) — standalone payload images;
//! - `tebako-pkg info` / `tebako-pkg validate` — packed binaries (the tpkg
//!   container AND its slot payloads).
//!
//! Every artifact is self-describing (spec 03); this crate exposes ALL of
//! it — container, manifest, declarations, trust state and derived facts —
//! in both human and machine form, without ever mutating the artifact or
//! the cache. Info is read-only; verification is a named, explicit mode
//! with strict exit codes ([`exit_code`]).
//!
//! The default outputs of both CLIs stay byte-parity with the C++ oracle
//! and do NOT pass through here; only the additive flags do.

#![forbid(unsafe_code)]

pub mod constraint;
pub mod derived;
pub mod format;
pub mod manifest_json;
pub mod package;
pub mod payload;
pub mod render;
pub mod verify;

pub use derived::{Derived, RuntimeCompat};
pub use format::FormatInfo;
pub use package::{PackageInspection, SlotInspection};
pub use payload::PayloadInspection;
pub use verify::{Check, CheckResult};

/// The strict verification exit codes (spec 15 §5).
pub mod exit_code {
    /// All checks passed.
    pub const OK: i32 = 0;
    /// Trailer/manifest missing, malformed, or schema-invalid.
    pub const MALFORMED: i32 = 65;
    /// sha256 mismatch (slot digest vs content, manifest digest vs image).
    pub const DIGEST: i32 = 70;
    /// Signature invalid (or unsigned under `--require-signed`).
    pub const SIGNATURE: i32 = 71;
    /// Signer key not in the trusted keyring.
    pub const TRUST: i32 = 72;
}

/// The JSON document schema version (spec 15 §6: consumers pin to
/// `info_schema`, not to field order).
pub const INFO_SCHEMA: u32 = 1;

/// A named error of the info surface (never a panic on malformed input).
#[derive(Debug)]
pub struct InfoError(pub String);

impl std::fmt::Display for InfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InfoError {}

pub(crate) fn err(msg: impl Into<String>) -> InfoError {
    InfoError(msg.into())
}

/// Human-readable size ("%.1f <unit>", units dividing by 1024) — the same
/// rendering the CLIs use for their legacy outputs.
pub fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut unit = 0;
    let mut size_d = size as f64;
    while size_d >= 1024.0 && unit < 4 {
        size_d /= 1024.0;
        unit += 1;
    }
    format!("{size_d:.1} {}", UNITS[unit])
}

/// Thousands-separated byte count (`3,842,112`) for the container report.
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
