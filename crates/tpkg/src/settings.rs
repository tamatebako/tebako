//! The declarative settings registry (spec 23 §14) — the single source
//! of truth for every user-facing setting, regardless of channel.
//!
//! A tebako setting is declarable through up to THREE channels: the CLI
//! flag, the environment variable, and the compose-document key. Each
//! channel used to be wired ad-hoc, which is how semantics drift (one
//! channel learns a behavior the others never hear about). Here every
//! setting is declared ONCE — all three spellings, the default, and the
//! one doc line — and every consumer resolves through `resolve_bool`
//! (boolean settings) or `resolve_sign` (the `sign` setting, whose CLI
//! channel carries an optional `=<keyid>`), so the channels share one
//! semantics by construction (invariant 10: a second hand-written copy
//! of a contract value is a bug on arrival).
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
//! `resolve_bool`/`resolve_sign` call at the consumer; nothing else can
//! drift.

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

/// `sign` — the package's signing declaration (spec 09 §9, tebako#400
/// S1). Resolved at press; a true resolution signs the package trailer
/// (`TPKG_FLAG_SIGNED_V2`, bit 1, + the v2 chain-of-trust extension),
/// unsigned v1 stays the default — byte-identical to pre-signing output,
/// no key material touched. The CLI channel is the only one that may
/// name a key (`--sign=<keyid>`): a keyid is per-machine material and
/// never rides a git-shared document. An explicit opt-out
/// (`--no-sign` / `TEBAKO_SIGN=0`) that drops a lower channel's
/// declaration is LOUD — a stderr warning plus the audit journal's
/// `event=press-sign-opt-out` (a quiet trust downgrade is invariant 9's
/// named failure).
pub const SIGN: Setting = Setting {
    config: Some("sign"),
    env: Some("TEBAKO_SIGN"),
    cli: Some("--sign"),
    doc: "sign the package trailer at press (TPKG_FLAG_SIGNED_V2 + the v2 \
          chain-of-trust extension); unsigned v1 stays the default, \
          an opt-out overriding a lower channel is loud",
};

/// Every registered setting (help/schema/docs render from this table).
pub const SETTINGS: &[Setting] = &[QUIET_NOTICES, SIGN];

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

/// The `sign` setting's CLI channel contribution (spec 09 §9): bare
/// `--sign` (the press-local key), `--sign=<keyid>` (a named secret key
/// from `$TEBAKO_HOME/keys`), `--no-sign` (the explicit opt-out). Only
/// this channel may carry a keyid — a keyid is per-machine material and
/// never rides a git-shared document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignCli {
    /// Bare `--sign`: sign with the press-local key.
    PressLocal,
    /// `--sign=<keyid>`: sign with the named key (16 hex chars).
    Keyid(String),
    /// `--no-sign`: the explicit opt-out.
    NoSign,
}

/// The signing decision one resolution arrives at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignDecision {
    /// No channel declared signing (or an opt-out won): the v1-unsigned
    /// trailer — byte-identical to pre-signing output, no key material
    /// touched.
    Unsigned,
    /// Sign with the press-local key (generated and cached under
    /// `$TEBAKO_HOME/keys` on first explicit use, auto-registered into
    /// the local trusted keyring).
    PressLocal,
    /// Sign with the secret key from `$TEBAKO_HOME/keys` whose keyid
    /// matches. A keyid naming no key is a NAMED error at the caller,
    /// raised before any heavy work — never a fallback to the
    /// press-local key.
    Keyid(String),
}

/// An explicit opt-out that dropped a lower channel's `sign`
/// declaration — the LOUD case (spec 09 §9: stderr warning + the audit
/// journal's `event=press-sign-opt-out`), because silently dropping an
/// authored signing declaration would be a quiet trust downgrade. An
/// opt-out over silence stays quiet (nothing was overridden).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignOptOut {
    /// The channel carrying the winning opt-out: `"cli"` (`--no-sign`)
    /// or `"env"` (`TEBAKO_SIGN=0`).
    pub by: &'static str,
    /// The highest lower channel whose sign declaration was dropped:
    /// `"env"` (`TEBAKO_SIGN=1`) or `"compose"` (`sign: true`).
    pub overridden: &'static str,
}

/// The `sign` resolution: the decision plus the loud-opt-out signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignResolution {
    pub decision: SignDecision,
    /// `Some` exactly when an explicit opt-out overrode a lower channel's
    /// sign declaration (see [`SignOptOut`]).
    pub opt_out: Option<SignOptOut>,
}

/// Resolve the `sign` setting across the three channels (spec 09 §9).
/// Same fixed precedence as [`resolve_bool`] — CLI → environment →
/// compose document → default (unsigned) — with the CLI channel's
/// optional `=<keyid>` riding [`SignCli`]. The env and compose channels
/// are plain booleans and always mean the press-local key. As with
/// [`resolve_bool`], a malformed env value is a named error when the env
/// channel rules, and moot when a higher channel is present.
pub fn resolve_sign(
    setting: &Setting,
    cli: Option<SignCli>,
    env: Option<String>,
    config: Option<bool>,
) -> Result<SignResolution, SettingsError> {
    // The lower channels' declarations, for the loud-opt-out signal. The
    // env parses strictly only when it rules (below); for the override
    // check a malformed env simply does not declare (a higher channel
    // moots it — resolve_bool's precedence).
    let env_declares = || {
        env.as_deref()
            .and_then(|raw| parse_env_bool(setting, raw).ok())
            == Some(true)
    };
    let config_declares = || config == Some(true);
    let opt_out = |by: &'static str| {
        if env_declares() {
            Some(SignOptOut {
                by,
                overridden: "env",
            })
        } else if config_declares() {
            Some(SignOptOut {
                by,
                overridden: "compose",
            })
        } else {
            None
        }
    };
    let resolved = |decision, opt_out| SignResolution { decision, opt_out };
    match cli {
        Some(SignCli::PressLocal) => Ok(resolved(SignDecision::PressLocal, None)),
        Some(SignCli::Keyid(keyid)) => Ok(resolved(SignDecision::Keyid(keyid), None)),
        Some(SignCli::NoSign) => Ok(resolved(SignDecision::Unsigned, opt_out("cli"))),
        None => match &env {
            Some(raw) => match parse_env_bool(setting, raw)? {
                true => Ok(resolved(SignDecision::PressLocal, None)),
                false => Ok(resolved(SignDecision::Unsigned, opt_out("env"))),
            },
            None => Ok(resolved(
                if config == Some(true) {
                    SignDecision::PressLocal
                } else {
                    SignDecision::Unsigned
                },
                None,
            )),
        },
    }
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

    // ---- the `sign` setting (spec 09 §9) ----

    fn unsigned(resolved: SignResolution) -> SignResolution {
        assert_eq!(resolved.decision, SignDecision::Unsigned);
        resolved
    }

    #[test]
    fn sign_cli_beats_everything() {
        // Bare --sign signs with the press-local key, however the lower
        // channels declare.
        let r = resolve_sign(
            &SIGN,
            Some(SignCli::PressLocal),
            Some("0".into()),
            Some(false),
        )
        .unwrap();
        assert_eq!(r.decision, SignDecision::PressLocal);
        assert_eq!(r.opt_out, None);
        // --sign=<keyid> carries the keyid through.
        let r = resolve_sign(
            &SIGN,
            Some(SignCli::Keyid("0123456789abcdef".into())),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            r.decision,
            SignDecision::Keyid("0123456789abcdef".to_string())
        );
        // --no-sign wins over every lower channel.
        let r = resolve_sign(&SIGN, Some(SignCli::NoSign), Some("1".into()), Some(true)).unwrap();
        assert_eq!(
            unsigned(r).opt_out.map(|o| (o.by, o.overridden)),
            Some(("cli", "env"))
        );
    }

    #[test]
    fn sign_env_beats_config_and_default() {
        let r = resolve_sign(&SIGN, None, Some("1".into()), Some(false)).unwrap();
        assert_eq!(r.decision, SignDecision::PressLocal);
        // TEBAKO_SIGN=0 overriding the document's `sign: true` is the
        // loud opt-out.
        let r = resolve_sign(&SIGN, None, Some("0".into()), Some(true)).unwrap();
        assert_eq!(
            unsigned(r).opt_out.map(|o| (o.by, o.overridden)),
            Some(("env", "compose"))
        );
        // …over the default alone it stays quiet (nothing overridden).
        let r = resolve_sign(&SIGN, None, Some("0".into()), None).unwrap();
        assert_eq!(unsigned(r).opt_out, None);
    }

    #[test]
    fn sign_config_beats_default() {
        let r = resolve_sign(&SIGN, None, None, Some(true)).unwrap();
        assert_eq!(r.decision, SignDecision::PressLocal);
        // The document never names a key — boolean true is the
        // press-local key (spec 09 §9).
        let r = resolve_sign(&SIGN, None, None, Some(false)).unwrap();
        assert_eq!(unsigned(r.clone()).opt_out, None);
        let r = resolve_sign(&SIGN, None, None, None).unwrap();
        assert_eq!(unsigned(r).opt_out, None);
    }

    #[test]
    fn sign_opt_out_over_silence_is_quiet() {
        // --no-sign with no lower declaration is not an override.
        let r = resolve_sign(&SIGN, Some(SignCli::NoSign), None, None).unwrap();
        assert_eq!(unsigned(r).opt_out, None);
        let r = resolve_sign(&SIGN, Some(SignCli::NoSign), Some("0".into()), Some(false)).unwrap();
        assert_eq!(unsigned(r).opt_out, None);
        // --no-sign over the document alone names the compose channel.
        let r = resolve_sign(&SIGN, Some(SignCli::NoSign), None, Some(true)).unwrap();
        assert_eq!(
            unsigned(r).opt_out.map(|o| (o.by, o.overridden)),
            Some(("cli", "compose"))
        );
    }

    #[test]
    fn sign_env_garbage_is_named_only_when_the_env_rules() {
        // Same discipline as resolve_bool: strict when the env channel
        // rules, moot when the CLI is present.
        let err = resolve_sign(&SIGN, None, Some("maybe".into()), Some(true))
            .expect_err("garbage env must not silently fall through to the config");
        assert!(err.to_string().contains("TEBAKO_SIGN"), "{err}");
        let r = resolve_sign(
            &SIGN,
            Some(SignCli::NoSign),
            Some("maybe".into()),
            Some(true),
        )
        .unwrap();
        assert_eq!(
            unsigned(r).opt_out.map(|o| (o.by, o.overridden)),
            Some(("cli", "compose"))
        );
    }
}
