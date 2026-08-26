//! The declarative settings registry (spec 23 §14) — the single source
//! of truth for every user-facing setting, regardless of channel.
//!
//! A tebako setting is declarable through up to THREE channels: the CLI
//! flag, the environment variable, and the compose-document key. Each
//! channel used to be wired ad-hoc, which is how semantics drift (one
//! channel learns a behavior the others never hear about). Here every
//! setting is declared ONCE — all three spellings, the default, and the
//! one doc line — and every consumer resolves through `resolve_bool`,
//! so the channels share one semantics by construction (invariant 10:
//! a second hand-written copy of a contract value is a bug on arrival).
//!
//! Precedence is fixed, highest first: **CLI → environment → compose
//! document → default**. Every channel is tri-state (present-and-true,
//! present-and-false, absent): a compose-document `quiet_notices: true`
//! stays overridable per invocation (`--no-quiet-notices`,
//! `TEBAKO_QUIET_NOTICES=0`), and an environment that names a setting
//! with an unparseable value is a NAMED error, never a silent default
//! (invariant 9 — fail-closed).
//!
//! The registry also declares WHICH channels a setting supports: a
//! machine-level knob (a cache root) has no business in a git-shared
//! compose document, and a package-policy bit the press bakes must be
//! declarable in the document the repo carries. `None` on a channel
//! means the setting never rides that channel.
//!
//! `--help`, the compose JSON Schema, and the docs table all render
//! from this registry — adding a setting is one entry here plus one
//! `resolve_bool` call at the consumer; nothing else can drift.

use std::fmt;

/// One setting's channel spellings and documentation.
#[derive(Debug, Clone, Copy)]
pub struct Setting {
    /// The compose-document key (`None` = never settable from config).
    pub config: Option<&'static str>,
    /// The environment spelling (`None` = never settable from the env).
    pub env: Option<&'static str>,
    /// The CLI flag (`None` = never settable from the command line).
    pub cli: Option<&'static str>,
    /// The one doc line every surface renders (--help, schema, docs).
    pub doc: &'static str,
}

impl Setting {
    /// The environment channel's raw contribution (absent or unset →
    /// `None`; the value travels verbatim for strict parsing at
    /// resolution).
    pub fn env_value(&self) -> Option<String> {
        self.env.and_then(|name| std::env::var(name).ok())
    }
}

/// `quiet_notices` — the package's notice policy (tebako#400). Baked at
/// press as `TPKG_FLAG_QUIET_NOTICES` (bit 3): every run of the package
/// suppresses the unsigned-legacy-trailer warning and the progress
/// lines. The bit lives inside the signed region, so a signed
/// package's policy is unforgeable; `TEBAKO_REQUIRE_SIGNED=1` is
/// checked FIRST and always outranks it, and acceptance stays
/// journaled regardless.
pub const QUIET_NOTICES: Setting = Setting {
    config: Some("quiet_notices"),
    env: Some("TEBAKO_QUIET_NOTICES"),
    cli: Some("--quiet-notices"),
    doc: "suppress the unsigned-legacy-trailer warning and the progress \
          lines for every run of the package (baked as trailer flag bit 3)",
};

/// Every registered setting (help/schema/docs render from this table).
pub const SETTINGS: &[Setting] = &[QUIET_NOTICES];

/// A settings resolution failure.
#[derive(Debug)]
pub enum SettingsError {
    /// The environment named a setting with an unparseable value.
    InvalidEnvValue { env: &'static str, value: String },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::InvalidEnvValue { env, value } => write!(
                f,
                "{env}={value:?} is not a boolean (1/0, true/false, yes/no, on/off)"
            ),
        }
    }
}

impl std::error::Error for SettingsError {}

/// Parse one environment channel value, strictly.
fn parse_env_bool(setting: &Setting, raw: &str) -> Result<bool, SettingsError> {
    match raw {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(SettingsError::InvalidEnvValue {
            env: setting.env.expect("env-parsed settings carry an env name"),
            value: raw.to_string(),
        }),
    }
}

/// Resolve a boolean setting across the three channels. Precedence:
/// CLI → environment → compose document → default (`false`). Each
/// channel contributes `Some(v)` when present (explicit true OR false)
/// or `None` when absent; a malformed env value is a named error.
pub fn resolve_bool(
    setting: &Setting,
    cli: Option<bool>,
    env: Option<String>,
    config: Option<bool>,
) -> Result<bool, SettingsError> {
    if let Some(v) = cli {
        return Ok(v);
    }
    if let Some(raw) = env {
        return parse_env_bool(setting, &raw);
    }
    if let Some(v) = config {
        return Ok(v);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_beats_everything() {
        assert!(resolve_bool(&QUIET_NOTICES, Some(true), Some("0".into()), Some(false)).unwrap());
        assert!(
            !resolve_bool(&QUIET_NOTICES, Some(false), Some("1".into()), Some(true)).unwrap(),
            "an explicit --no-<flag> overrides both lower channels"
        );
    }

    #[test]
    fn env_beats_config_and_default() {
        assert!(resolve_bool(&QUIET_NOTICES, None, Some("yes".into()), Some(false)).unwrap());
        assert!(!resolve_bool(&QUIET_NOTICES, None, Some("off".into()), Some(true)).unwrap());
    }

    #[test]
    fn config_beats_default() {
        assert!(resolve_bool(&QUIET_NOTICES, None, None, Some(true)).unwrap());
        assert!(!resolve_bool(&QUIET_NOTICES, None, None, Some(false)).unwrap());
        assert!(!resolve_bool(&QUIET_NOTICES, None, None, None).unwrap());
    }

    #[test]
    fn every_env_spelling_parses() {
        for (raw, want) in [
            ("1", true),
            ("true", true),
            ("yes", true),
            ("on", true),
            ("0", false),
            ("false", false),
            ("no", false),
            ("off", false),
        ] {
            assert_eq!(
                resolve_bool(&QUIET_NOTICES, None, Some(raw.to_string()), None).unwrap(),
                want,
                "{raw:?}"
            );
        }
    }

    #[test]
    fn a_garbage_env_value_is_a_named_error_never_a_default() {
        let err = resolve_bool(&QUIET_NOTICES, None, Some("maybe".into()), Some(true))
            .expect_err("garbage env must not silently fall through to the config");
        let msg = err.to_string();
        assert!(msg.contains("TEBAKO_QUIET_NOTICES"), "{msg}");
        assert!(msg.contains("maybe"), "{msg}");
    }
}
