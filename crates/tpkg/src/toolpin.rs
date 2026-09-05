//! The `[payload@]version` grammar — the chain VALUE grammar of spec 07
//! §2.1's version chain and the selector grammar of the disabled-state
//! file (`~/.tebako/shims/.disabled.yaml`), ONE parser for both (spec 00
//! invariant 10 — the grammar SSOT; every consumer flows this module).
//!
//! The split rule is FIRST-`@`, at most one `@`: `@` never appears in
//! payload names or command names (the name grammar), so the split is
//! unambiguous. The spec-32 lock row's `name@version` (`crate::package`'s
//! digest-carrying record) is a DIFFERENT record that happens to share
//! the same split rule; it stays where it is.
//!
//! A payload-only pin (`name@`) does NOT exist: an incomplete link would
//! create cross-link partial application; route-out rides the
//! disable-selector side (`disable <tool> --of <payload>` →
//! `<payload>@all`).

use std::fmt;

/// A chain-value grammar error (the shim wraps it in
/// `EX_TEBAKO_MANIFEST`, naming the chain link and value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPinError(pub String);

impl fmt::Display for ToolPinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ToolPinError {}

/// A version-chain value: `version` or `payload@version` (spec 07 §2.1,
/// 2026-09-05 routing amendment). A payload-qualified value makes the
/// named payload THE provider — it must be installed and declare or
/// expose the command, else the named NotAProvider error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPin {
    pub payload: Option<String>,
    pub version: String,
}

impl ToolPin {
    pub fn parse(value: &str) -> Result<ToolPin, ToolPinError> {
        let err = |why: &str| {
            ToolPinError(format!(
                "invalid pin \"{value}\" ({why}) — the grammar is [payload@]version"
            ))
        };
        match value.split('@').collect::<Vec<_>>().as_slice() {
            [version] if !version.is_empty() => Ok(ToolPin {
                payload: None,
                version: (*version).to_string(),
            }),
            [payload, version] if !payload.is_empty() && !version.is_empty() => Ok(ToolPin {
                payload: Some((*payload).to_string()),
                version: (*version).to_string(),
            }),
            ["", _] => Err(err("empty payload")),
            [_, ""] => Err(err(
                "payload-only pins do not exist — route out with disable",
            )),
            [_, _, ..] => Err(err("at most one '@'")),
            [_] => Err(err("empty version")),
            _ => Err(err("empty value")),
        }
    }
}

impl fmt::Display for ToolPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.payload {
            Some(payload) => write!(f, "{payload}@{}", self.version),
            None => f.write_str(&self.version),
        }
    }
}

/// A `.disabled.yaml` selector: which claims of a command are gated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisableSelector {
    /// Every claim of the command.
    All,
    /// Every claim AT this version, whatever the payload.
    Version(String),
    /// Every claim BY this payload.
    PayloadAll(String),
    /// Exactly this (payload, version) claim.
    PayloadVersion(String, String),
}

impl DisableSelector {
    pub fn parse(value: &str) -> Result<DisableSelector, ToolPinError> {
        let err = |why: &str| {
            ToolPinError(format!(
                "invalid disable selector \"{value}\" ({why}) — the grammar is all | version | payload@all | payload@version"
            ))
        };
        if value == "all" {
            return Ok(DisableSelector::All);
        }
        match value.split('@').collect::<Vec<_>>().as_slice() {
            [version] if !version.is_empty() => {
                Ok(DisableSelector::Version((*version).to_string()))
            }
            [payload, tail] if !payload.is_empty() && !tail.is_empty() => Ok(if *tail == "all" {
                DisableSelector::PayloadAll((*payload).to_string())
            } else {
                DisableSelector::PayloadVersion((*payload).to_string(), (*tail).to_string())
            }),
            [_, _, ..] => Err(err("at most one '@'")),
            _ => Err(err(
                "a selector names a version, a payload, or both — never none",
            )),
        }
    }

    /// Does this selector gate the claim `payload` makes at `version`?
    pub fn matches(&self, payload: &str, version: &str) -> bool {
        match self {
            DisableSelector::All => true,
            DisableSelector::Version(v) => v == version,
            DisableSelector::PayloadAll(p) => p == payload,
            DisableSelector::PayloadVersion(p, v) => p == payload && v == version,
        }
    }
}

impl fmt::Display for DisableSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisableSelector::All => f.write_str("all"),
            DisableSelector::Version(v) => f.write_str(v),
            DisableSelector::PayloadAll(p) => write!(f, "{p}@all"),
            DisableSelector::PayloadVersion(p, v) => write!(f, "{p}@{v}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolpin_grammar() {
        let p = ToolPin::parse("3.34.0").unwrap();
        assert_eq!(p.payload, None);
        assert_eq!(p.version, "3.34.0");
        let q = ToolPin::parse("xml2rfc-b@3.34.0").unwrap();
        assert_eq!(q.payload.as_deref(), Some("xml2rfc-b"));
        assert_eq!(q.version, "3.34.0");
        assert!(ToolPin::parse("").is_err());
        assert!(ToolPin::parse("@3.34.0").is_err()); // empty payload
        assert!(ToolPin::parse("xml2rfc-b@").is_err()); // payload-only pins do not exist — route-out rides disable
        assert!(ToolPin::parse("a@b@c").is_err()); // one @ max
    }

    #[test]
    fn disable_selector_grammar_and_matching() {
        assert!(DisableSelector::parse("all").unwrap().matches("any", "1.0"));
        assert!(DisableSelector::parse("3.34.0")
            .unwrap()
            .matches("any", "3.34.0"));
        assert!(!DisableSelector::parse("3.34.0")
            .unwrap()
            .matches("any", "3.35.0"));
        assert!(DisableSelector::parse("xml2rfc-b@all")
            .unwrap()
            .matches("xml2rfc-b", "9.9"));
        assert!(!DisableSelector::parse("xml2rfc-b@all")
            .unwrap()
            .matches("xml2rfc", "9.9"));
        assert!(DisableSelector::parse("a@1.0").unwrap().matches("a", "1.0"));
        assert!(!DisableSelector::parse("a@1.0").unwrap().matches("b", "1.0"));
        assert!(DisableSelector::parse("a@").is_err());
    }
}
