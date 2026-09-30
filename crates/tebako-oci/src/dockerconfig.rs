//! The docker `config.json` credential fallback (spec 38 §6, tier 3 —
//! OCI-class URLs only): static `auths[<host>]` entries (the base64
//! `user:pass` pair) from `$DOCKER_CONFIG/config.json` (default
//! `~/.docker/config.json`). `credHelpers`/`credsStore` entries covering
//! a referenced host are REFUSED BY NAME — helpers are shell-outs,
//! forbidden by the no-shell-outs law, and the steer says so (move the
//! token into a `credentials:` entry's env var). An unparsable config
//! consulted by a fetch is a named error, never a silent skip.

use std::path::PathBuf;

use tebako_json::{parse as json_parse, Value as JsonValue};

/// The docker-config failure classes (the adapter maps them to the
/// spec 38 §9 named errors, exit class 65).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerConfigError {
    /// The config was consulted and did not parse (JSON, or an `auth`
    /// value that is not base64 `user:pass`).
    Malformed { path: PathBuf, reason: String },
    /// A `credHelpers`/`credsStore` entry would answer for the host —
    /// the no-shell-outs steer.
    HelperUnsupported { host: String, helper: String },
}

impl std::fmt::Display for DockerConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DockerConfigError::Malformed { path, reason } => write!(
                f,
                "DockerConfigMalformed: cannot parse {}: {reason}",
                path.display()
            ),
            DockerConfigError::HelperUnsupported { host, helper } => write!(
                f,
                "DockerCredentialHelperUnsupported: the docker config answers '{host}' through the '{helper}' credential helper — helpers are shell-outs, forbidden by the no-shell-outs law; move the token into a `credentials:` entry's env var in ~/.tebako/config.yaml"
            ),
        }
    }
}

impl std::error::Error for DockerConfigError {}

/// The parsed config (only the credential-relevant keys).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DockerConfig {
    /// The config's own path (error context).
    pub path: PathBuf,
    /// `(normalized host, user:pass)` from static `auths` entries.
    auths: Vec<(String, String)>,
    /// `(host, helper)` from `credHelpers`.
    cred_helpers: Vec<(String, String)>,
    /// The `credsStore` helper, covering every host without a static
    /// entry.
    creds_store: Option<String>,
}

/// The config's location: `$DOCKER_CONFIG/config.json`, else
/// `~/.docker/config.json`.
fn config_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("DOCKER_CONFIG") {
        return Some(PathBuf::from(dir).join("config.json"));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".docker").join("config.json"))
}

/// An auths key normalized to `host[:port]`: any `scheme://` prefix and
/// any path suffix (`/v1/`, …) dropped — docker's own spellings
/// (`https://index.docker.io/v1/`) reduce to the host.
fn normalize_auth_key(key: &str) -> &str {
    let bare = key.split_once("://").map(|(_, rest)| rest).unwrap_or(key);
    bare.split('/').next().unwrap_or(bare)
}

impl DockerConfig {
    /// Load the config, if one exists. Absent file → `Ok(None)`; a file
    /// that does not parse is the named Malformed, never a skip.
    pub fn load() -> Result<Option<DockerConfig>, DockerConfigError> {
        let Some(path) = config_path() else {
            return Ok(None);
        };
        Self::load_from(path)
    }

    /// The explicit-path half of [`DockerConfig::load`] (tests).
    pub fn load_from(path: PathBuf) -> Result<Option<DockerConfig>, DockerConfigError> {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(DockerConfigError::Malformed {
                    path,
                    reason: format!("{e} reading the file"),
                })
            }
        };
        let doc = json_parse(&text).map_err(|e| DockerConfigError::Malformed {
            path: path.clone(),
            reason: e,
        })?;
        let mut config = DockerConfig {
            path: path.clone(),
            ..DockerConfig::default()
        };
        if let Some(JsonValue::Object(entries)) = doc.find("auths") {
            for (key, entry) in entries {
                let Some(auth) = entry.find("auth").and_then(|a| a.as_string()) else {
                    // An entry without `auth` (a `{}` helper placeholder)
                    // carries no static credential — the helper rules
                    // below still apply.
                    continue;
                };
                let decoded =
                    base64_decode(auth.as_bytes()).ok_or_else(|| DockerConfigError::Malformed {
                        path: path.clone(),
                        reason: format!("auths[{key}].auth is not valid base64"),
                    })?;
                let pair = String::from_utf8(decoded).ok().filter(|p| p.contains(':'));
                let Some(pair) = pair else {
                    return Err(DockerConfigError::Malformed {
                        path: path.clone(),
                        reason: format!("auths[{key}].auth is not a base64 'user:password' pair"),
                    });
                };
                config
                    .auths
                    .push((normalize_auth_key(key).to_string(), pair));
            }
        }
        if let Some(JsonValue::Object(entries)) = doc.find("credHelpers") {
            for (key, helper) in entries {
                if let Some(helper) = helper.as_string() {
                    config
                        .cred_helpers
                        .push((normalize_auth_key(key).to_string(), helper));
                }
            }
        }
        config.creds_store = doc.find("credsStore").and_then(|s| s.as_string());
        Ok(Some(config))
    }

    /// The static `user:password` pair for `host` (spec 38 §6 tier 3):
    /// a static `auths` hit wins; a helper covering the host is the
    /// named refusal; no entry at all is anonymous (`Ok(None)`).
    pub fn credential_for(&self, host: &str) -> Result<Option<String>, DockerConfigError> {
        if let Some((_, pair)) = self.auths.iter().find(|(h, _)| h == host) {
            return Ok(Some(pair.clone()));
        }
        if let Some((_, helper)) = self.cred_helpers.iter().find(|(h, _)| h == host) {
            return Err(DockerConfigError::HelperUnsupported {
                host: host.to_string(),
                helper: helper.clone(),
            });
        }
        if let Some(helper) = &self.creds_store {
            if !helper.is_empty() {
                return Err(DockerConfigError::HelperUnsupported {
                    host: host.to_string(),
                    helper: helper.clone(),
                });
            }
        }
        Ok(None)
    }
}

/// Standard base64 decode (the `auth` values). The encoder lives in the
/// crate root ([`crate::base64_encode`]); this is the docker-config
/// reader's own half — one alphabet, strict (no whitespace, padding
/// required).
pub(crate) fn base64_decode(text: &[u8]) -> Option<Vec<u8>> {
    fn val(b: u8) -> Option<u32> {
        match b {
            b'A'..=b'Z' => Some((b - b'A') as u32),
            b'a'..=b'z' => Some((b - b'a' + 26) as u32),
            b'0'..=b'9' => Some((b - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    if text.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    for (i, chunk) in text.chunks(4).enumerate() {
        let last = i == text.len() / 4 - 1;
        let pad = if last {
            chunk.iter().filter(|&&b| b == b'=').count()
        } else {
            0
        };
        if pad > 2 || (!last && chunk.contains(&b'=')) {
            return None;
        }
        let mut acc = 0u32;
        for (j, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                if j < 4 - pad {
                    return None;
                }
                continue;
            }
            acc = (acc << 6) | val(b)?;
        }
        // `acc` carries exactly the data chars' 6*(4-pad) bits; the
        // decoded bytes are its top 8-bit groups (3-pad of them).
        let data_bits = 6 * (4 - pad);
        for k in 0..(3 - pad) {
            out.push((acc >> (data_bits - 8 * (k + 1))) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tebako-oci-docker-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_static_auths_entry_decodes_the_pair() {
        let dir = scratch("auths");
        let path = dir.join("config.json");
        // user:pass → dXNlcjpwYXNz
        std::fs::write(
            &path,
            r#"{"auths":{"ghcr.io":{"auth":"dXNlcjpwYXNz"},"https://harbor.corp.internal/v1/":{"auth":"aG9sYTptaS1wYXNz"}}}"#,
        )
        .unwrap();
        let config = DockerConfig::load_from(path).unwrap().unwrap();
        assert_eq!(
            config.credential_for("ghcr.io").unwrap().as_deref(),
            Some("user:pass")
        );
        // docker's scheme+path spellings normalize to the host
        assert_eq!(
            config
                .credential_for("harbor.corp.internal")
                .unwrap()
                .as_deref(),
            Some("hola:mi-pass")
        );
        assert_eq!(config.credential_for("other.example").unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_covering_helper_is_the_named_refusal() {
        let dir = scratch("helper");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"credHelpers":{"ghcr.io":"ghcr-login"},"credsStore":"desktop"}"#,
        )
        .unwrap();
        let config = DockerConfig::load_from(path).unwrap().unwrap();
        let err = config.credential_for("ghcr.io").unwrap_err();
        assert!(
            matches!(err, DockerConfigError::HelperUnsupported { ref host, ref helper } if host == "ghcr.io" && helper == "ghcr-login"),
            "{err:?}"
        );
        assert!(err.to_string().contains("no-shell-outs law"), "{err}");
        // credsStore covers every host without a static entry
        let err = config.credential_for("harbor.corp.internal").unwrap_err();
        assert!(
            matches!(err, DockerConfigError::HelperUnsupported { ref helper, .. } if helper == "desktop"),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_static_entry_wins_over_the_store() {
        let dir = scratch("wins");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"auths":{"ghcr.io":{"auth":"dXNlcjpwYXNz"}},"credsStore":"desktop"}"#,
        )
        .unwrap();
        let config = DockerConfig::load_from(path).unwrap().unwrap();
        assert_eq!(
            config.credential_for("ghcr.io").unwrap().as_deref(),
            Some("user:pass")
        );
        assert!(matches!(
            config.credential_for("other.example"),
            Err(DockerConfigError::HelperUnsupported { .. })
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_configs_are_named_never_skipped() {
        let dir = scratch("malformed");
        let path = dir.join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        let err = DockerConfig::load_from(path.clone()).unwrap_err();
        assert!(
            matches!(err, DockerConfigError::Malformed { .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains("DockerConfigMalformed"), "{err}");
        // a non-base64 auth value
        std::fs::write(&path, r#"{"auths":{"h":{"auth":"!!"}}}"#).unwrap();
        assert!(matches!(
            DockerConfig::load_from(path.clone()),
            Err(DockerConfigError::Malformed { .. })
        ));
        // base64 of a non-pair
        std::fs::write(&path, r#"{"auths":{"h":{"auth":"bm9jb2xvbg=="}}}"#).unwrap();
        assert!(matches!(
            DockerConfig::load_from(path.clone()),
            Err(DockerConfigError::Malformed { .. })
        ));
        // an absent file is None, not an error
        assert_eq!(
            DockerConfig::load_from(dir.join("missing.json")).unwrap(),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn base64_decode_round_trips_the_alphabet() {
        // vectors shared with tebako-resolve's encoder tests
        for (raw, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
            ("user:app-password", "dXNlcjphcHAtcGFzc3dvcmQ="),
        ] {
            assert_eq!(
                base64_decode(encoded.as_bytes()),
                Some(raw.as_bytes().to_vec()),
                "{encoded}"
            );
        }
        assert_eq!(base64_decode(b"Zg="), None); // bad length
        assert_eq!(base64_decode(b"Zg== "), None); // whitespace/length
        assert_eq!(base64_decode(b"===="), None); // no data
    }
}
