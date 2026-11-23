//! The per-mount materialization ceiling (tebako#1102; spec 05 §4's
//! store discipline extended to the exec cache — the resource-bounding
//! arc's mount arm, TODO.sota2/03).
//!
//! The content pool has bounds; mounts materialize without one without
//! this ceiling: the exec-cache surfaces a mount writes (the class-R
//! `resources/<image-key>/` namespace, the windows tier's
//! `trees/<tree-key>/`, the persistent `tebako-dlcache/<image-key>/`)
//! accumulate silently across boots. The ceiling is config-driven —
//! `mounts.quota` in `~/.tebako/config.yaml` (store-config schema minor
//! 6) — and enforced at the materialize seam (the runtime driver's
//! class-R and tree tiers, the tfs dlcache tier) with a named error on
//! exhaustion, never a silent partial write.
//!
//! This module is the grammar's single owner (spec 00 §10): the shim's
//! config model and the runtime driver's boot-time resolve both parse
//! through here. It is deliberately YAML-free — the two consumers carry
//! different YAML crates; each adapts its document value to
//! [`parse`]/[`from_count`].
//!
//! The grammar (one key, three spellings):
//!
//! ```yaml
//! mounts:
//!   quota: 512M        # a string: decimal bytes, 1024-based K|M|G suffix (trailing B allowed)
//!   quota: 536870912   # an integer: bytes
//!   quota: off         # the explicit unbounded spelling
//! ```
//!
//! Absent section or key binds the documented default
//! ([`DEFAULT_BYTES`]). The value mirrors the jail grammar's `memory`
//! byte count (spec 08 §2.2) so the operator learns one byte grammar.

/// The default ceiling: 1 GiB per mount. Sized against the first-class
/// artifacts' honest footprints (a runtime env tree extracted whole by
/// the windows tier is O(100–300 MB); the macOS dlcache closure of a
/// runtime's native extensions is O(10–100 MB)) with headroom — a mount
/// that needs more is either mispacked or runaway, and the operator
/// raises the key or spells `off` explicitly.
pub const DEFAULT_BYTES: u64 = 1 << 30;

/// The config key this model parses (`mounts.quota`).
pub const CONFIG_KEY: &str = "mounts.quota";

/// The parsed ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountQuota {
    /// At most this many materialized bytes per mount.
    Limited(u64),
    /// `off` — enforcement disengages (the pre-quota behavior).
    Unlimited,
}

impl Default for MountQuota {
    fn default() -> Self {
        MountQuota::Limited(DEFAULT_BYTES)
    }
}

impl MountQuota {
    /// The ceiling in bytes, `None` when unlimited.
    pub fn ceiling(&self) -> Option<u64> {
        match self {
            MountQuota::Limited(bytes) => Some(*bytes),
            MountQuota::Unlimited => None,
        }
    }
}

/// Parse the string spelling: decimal bytes with an optional 1024-based
/// suffix — `K`/`M`/`G`, case-insensitive, an optional trailing `B` (so
/// `256M` and `256MB` are the same ceiling); a bare integer is bytes —
/// or `off` (case-insensitive) for the unbounded spelling. The byte
/// grammar mirrors the jail `memory` value's (spec 08 §2.2); a malformed
/// value is a named grammar error, never a guessed default.
pub fn parse(text: &str) -> Result<MountQuota, String> {
    if text.eq_ignore_ascii_case("off") {
        return Ok(MountQuota::Unlimited);
    }
    if text.is_empty() {
        return Err(format!("empty value for {CONFIG_KEY}"));
    }
    let digits_end = text.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits_end == 0 {
        return Err(format!(
            "{CONFIG_KEY} value {text:?} is not a byte count (want bytes with an optional K|M|G suffix, or `off`)"
        ));
    }
    let (mantissa, suffix) = text.split_at(digits_end);
    let shift = match suffix.to_ascii_uppercase().as_str() {
        "" => 0u32,
        "K" | "KB" => 10,
        "M" | "MB" => 20,
        "G" | "GB" => 30,
        _ => {
            return Err(format!(
                "{CONFIG_KEY} value {text:?} carries an unknown unit (want K|M|G, 1024-based, or `off`)"
            ))
        }
    };
    let m: u64 = mantissa
        .parse()
        .map_err(|_| format!("{CONFIG_KEY} value {text:?} is out of range"))?;
    let v = m
        .checked_mul(1u64 << shift)
        .ok_or_else(|| format!("{CONFIG_KEY} value {text:?} is out of range"))?;
    from_count(v)
}

/// The integer spelling (a YAML number): bytes. Zero is out of range —
/// `off` is the unbounded spelling, never a zero ceiling that refuses
/// every write by surprise.
pub fn from_count(v: u64) -> Result<MountQuota, String> {
    if v == 0 {
        return Err(format!(
            "{CONFIG_KEY} must be positive (got 0) — `off` is the unbounded spelling"
        ));
    }
    Ok(MountQuota::Limited(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_documented_gibibyte() {
        assert_eq!(MountQuota::default(), MountQuota::Limited(1 << 30));
        assert_eq!(MountQuota::default().ceiling(), Some(1 << 30));
    }

    #[test]
    fn bare_integers_are_bytes() {
        assert_eq!(parse("1048576"), Ok(MountQuota::Limited(1 << 20)));
        assert_eq!(from_count(42), Ok(MountQuota::Limited(42)));
    }

    #[test]
    fn the_suffix_grammar_is_1024_based_and_lenient() {
        assert_eq!(parse("256M"), Ok(MountQuota::Limited(256 << 20)));
        assert_eq!(parse("256MB"), Ok(MountQuota::Limited(256 << 20)));
        assert_eq!(parse("256mb"), Ok(MountQuota::Limited(256 << 20)));
        assert_eq!(parse("1G"), Ok(MountQuota::Limited(1 << 30)));
        assert_eq!(parse("2gB"), Ok(MountQuota::Limited(2 << 30)));
        assert_eq!(parse("64K"), Ok(MountQuota::Limited(64 << 10)));
        assert_eq!(parse("64kb"), Ok(MountQuota::Limited(64 << 10)));
    }

    #[test]
    fn off_is_the_unbounded_spelling() {
        assert_eq!(parse("off"), Ok(MountQuota::Unlimited));
        assert_eq!(parse("OFF"), Ok(MountQuota::Unlimited));
        assert_eq!(parse("Off"), Ok(MountQuota::Unlimited));
        assert_eq!(MountQuota::Unlimited.ceiling(), None);
    }

    #[test]
    fn malformed_values_are_named_errors() {
        for bad in ["", "lots", "M", "1T", "12.5M", "-5", "0x100", "1 MB"] {
            assert!(parse(bad).is_err(), "{bad:?} must be refused");
        }
        let err = parse("1T").unwrap_err();
        assert!(err.contains("unknown unit"), "{err}");
        assert!(err.contains(CONFIG_KEY), "{err}");
        let err = parse("lots").unwrap_err();
        assert!(err.contains("not a byte count"), "{err}");
        // Zero is out of range on both spellings — `off` is unbounded.
        let err = parse("0").unwrap_err();
        assert!(err.contains("must be positive"), "{err}");
        assert!(from_count(0).is_err());
    }

    #[test]
    fn out_of_range_is_a_named_error() {
        assert!(parse("99999999999999999999999999").is_err());
        // 2^44 G wraps u64 — refused, never wrapped small.
        assert!(parse("17592186044416G").is_err());
    }
}
