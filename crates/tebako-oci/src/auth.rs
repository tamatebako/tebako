//! The distribution-spec Bearer challenge (spec 38 §6): the
//! `WWW-Authenticate` parse and the per-process token cache keyed by
//! (realm, service, scope, credential class). A 401 against a cached
//! token re-challenges ONCE (the cache entry is evicted and the dance
//! re-runs), then fails named — the retry discipline lives in the
//! client.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// A parsed `Bearer` challenge (`realm` required, `service`/`scope`
/// as served).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearerChallenge {
    pub realm: String,
    pub service: Option<String>,
    pub scope: Option<String>,
}

/// Parse a `WWW-Authenticate` value: the `Bearer` challenge's
/// comma-separated quoted `key="value"` pairs (case-insensitive scheme
/// and keys). `Ok(None)` when the header carries no Bearer challenge at
/// all (the caller maps a Basic-only or absent challenge to the named
/// `OciTokenChallengeInvalid`); `Err(reason)` when a Bearer challenge
/// is present but malformed (an unquoted value, a missing realm).
pub fn parse_challenge(header: &str) -> Result<Option<BearerChallenge>, String> {
    // Challenges are comma-separated schemes; the Bearer one is ours.
    // Scheme names carry no quotes, so scanning for the scheme token at
    // a word boundary is safe.
    let mut rest = header;
    loop {
        let Some((scheme, tail)) = split_scheme(rest) else {
            return Ok(None);
        };
        if scheme.eq_ignore_ascii_case("bearer") {
            return parse_bearer_params(tail).map(Some);
        }
        // Skip to the next scheme: a new scheme starts after a comma
        // followed by a token and a space (quoted strings can hold
        // commas — walk them).
        rest = match next_scheme(tail) {
            Some(r) => r,
            None => return Ok(None),
        };
    }
}

/// The leading `Scheme` token and the remainder after it.
fn split_scheme(header: &str) -> Option<(&str, &str)> {
    let header = header.trim_start();
    let end = header
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(header.len());
    let (scheme, tail) = header.split_at(end);
    if scheme.is_empty()
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return None;
    }
    Some((scheme, tail))
}

/// Advance past the current scheme's parameters to the next scheme's
/// start (quoted strings may contain commas; a new scheme is a comma,
/// optional whitespace, a token, and whitespace-or-end — never `=`,
/// which would be another parameter of the current scheme).
fn next_scheme(params: &str) -> Option<&str> {
    let bytes = params.as_bytes();
    let mut i = 0;
    let mut quoted = false;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => quoted = !quoted,
            b',' if !quoted => {
                let candidate = params[i + 1..].trim_start();
                if let Some((scheme, tail)) = split_scheme(candidate) {
                    let tail = tail.trim_start();
                    if !tail.starts_with('=') {
                        let _ = scheme;
                        return Some(candidate);
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// `realm="…",service="…",scope="…"` — quoted pairs, any order, realm
/// required.
fn parse_bearer_params(params: &str) -> Result<BearerChallenge, String> {
    let mut realm = None;
    let mut service = None;
    let mut scope = None;
    let mut rest = params.trim();
    while !rest.is_empty() {
        let Some((key, tail)) = rest.split_once('=') else {
            return Err(format!("malformed Bearer parameter '{rest}'"));
        };
        let key = key.trim();
        let tail = tail.trim_start();
        let Some(tail) = tail.strip_prefix('"') else {
            return Err(format!("the Bearer {key} value is not quoted"));
        };
        let Some(end) = tail.find('"') else {
            return Err(format!("unterminated Bearer {key} value"));
        };
        let value = &tail[..end];
        if key.eq_ignore_ascii_case("realm") {
            realm = Some(value.to_string());
        } else if key.eq_ignore_ascii_case("service") {
            service = Some(value.to_string());
        } else if key.eq_ignore_ascii_case("scope") {
            scope = Some(value.to_string());
        }
        rest = tail[end + 1..]
            .trim_start()
            .trim_start_matches(',')
            .trim_start();
    }
    let realm = realm.ok_or_else(|| "the Bearer challenge carries no realm".to_string())?;
    Ok(BearerChallenge {
        realm,
        service,
        scope,
    })
}

// ---------------------------------------------------------------------
// The per-process token cache (spec 38 §6: keyed by (realm, service,
// scope, credential class))
// ---------------------------------------------------------------------

/// (realm, service, scope, credential class).
type CacheKey = (String, Option<String>, Option<String>, String);

static TOKENS: OnceLock<Mutex<HashMap<CacheKey, String>>> = OnceLock::new();

fn tokens() -> &'static Mutex<HashMap<CacheKey, String>> {
    TOKENS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The registry host's last-seen (realm, service), so a blob stream can
/// attach a cached token WITHOUT paying a probe 401 for the cache key.
type RealmInfo = (String, Option<String>);

static REALMS: OnceLock<Mutex<HashMap<String, RealmInfo>>> = OnceLock::new();

fn realms() -> &'static Mutex<HashMap<String, RealmInfo>> {
    REALMS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A cached token for exactly this (realm, service, scope, class).
pub fn cached_token(
    challenge: &BearerChallenge,
    scope: Option<&str>,
    class: &str,
) -> Option<String> {
    let key = (
        challenge.realm.clone(),
        challenge.service.clone(),
        scope.map(str::to_string),
        class.to_string(),
    );
    tokens()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .cloned()
}

/// A cached token addressed by registry host (the blob stream's
/// preemptive attach): the host's remembered (realm, service) plus the
/// call's scope and credential class.
pub fn cached_token_for_host(host: &str, scope: &str, class: &str) -> Option<String> {
    let (realm, service) = realms()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(host)
        .cloned()?;
    let challenge = BearerChallenge {
        realm,
        service,
        scope: None,
    };
    cached_token(&challenge, Some(scope), class)
}

/// Store a minted token and remember the host's (realm, service).
pub fn store_token(
    host: &str,
    challenge: &BearerChallenge,
    scope: Option<&str>,
    class: &str,
    token: &str,
) {
    let key = (
        challenge.realm.clone(),
        challenge.service.clone(),
        scope.map(str::to_string),
        class.to_string(),
    );
    tokens()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, token.to_string());
    realms().lock().unwrap_or_else(|e| e.into_inner()).insert(
        host.to_string(),
        (challenge.realm.clone(), challenge.service.clone()),
    );
}

/// Drop a token (a 401 against it — the re-challenge path).
pub fn evict_token(challenge: &BearerChallenge, scope: Option<&str>, class: &str) {
    let key = (
        challenge.realm.clone(),
        challenge.service.clone(),
        scope.map(str::to_string),
        class.to_string(),
    );
    tokens()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key);
}

/// The host-addressed half of [`evict_token`] (the preemptive-token
/// paths, which never saw the challenge itself).
pub fn evict_token_for_host(host: &str, scope: &str, class: &str) {
    let Some((realm, service)) = realms()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(host)
        .cloned()
    else {
        return;
    };
    let challenge = BearerChallenge {
        realm,
        service,
        scope: None,
    };
    evict_token(&challenge, Some(scope), class);
}

/// Test hook: the cache is process-global; suites that assert the dance
/// start cold.
#[doc(hidden)]
pub fn clear_tokens() {
    tokens().lock().unwrap_or_else(|e| e.into_inner()).clear();
    realms().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Test serialization: the cache is process-global and the test binary
/// is multi-threaded — every test that asserts the dance (or a cached
/// token's presence) holds this lock so a parallel `clear_tokens`
/// cannot interleave.
#[doc(hidden)]
pub static TEST_TOKEN_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ghcr_shape_parses() {
        let ch = parse_challenge(
            r#"Bearer realm="https://ghcr.io/token",service="ghcr.io",scope="repository:tebako-packages/metanorma:pull""#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(ch.realm, "https://ghcr.io/token");
        assert_eq!(ch.service.as_deref(), Some("ghcr.io"));
        assert_eq!(
            ch.scope.as_deref(),
            Some("repository:tebako-packages/metanorma:pull")
        );
    }

    #[test]
    fn scheme_and_keys_are_case_insensitive_and_order_free() {
        let ch = parse_challenge(r#"bearer scope="repository:r:pull", realm="https://h/t""#)
            .unwrap()
            .unwrap();
        assert_eq!(ch.realm, "https://h/t");
        assert_eq!(ch.scope.as_deref(), Some("repository:r:pull"));
        assert_eq!(ch.service, None);
    }

    #[test]
    fn a_basic_only_challenge_is_no_bearer_challenge() {
        assert_eq!(parse_challenge("Basic realm=\"registry\"").unwrap(), None);
        assert_eq!(parse_challenge("").unwrap(), None);
    }

    #[test]
    fn malformed_bearer_challenges_are_named() {
        // missing realm
        let err = parse_challenge(r#"Bearer service="s""#).unwrap_err();
        assert!(err.contains("realm"), "{err}");
        // unquoted value
        let err = parse_challenge(r#"Bearer realm=https://h/t"#).unwrap_err();
        assert!(err.contains("quoted"), "{err}");
        // unterminated
        let err = parse_challenge(r#"Bearer realm="https://h/t"#).unwrap_err();
        assert!(err.contains("unterminated"), "{err}");
        // garbage after the scheme
        let err = parse_challenge("Bearer !!!").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn a_multi_scheme_header_finds_the_bearer_one() {
        let ch = parse_challenge(
            r#"Basic realm="h", Bearer realm="https://h/token,with,commas",service="s""#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(ch.realm, "https://h/token,with,commas");
        assert_eq!(ch.service.as_deref(), Some("s"));
    }

    #[test]
    fn the_token_cache_keys_on_realm_service_scope_and_class() {
        let _guard = TEST_TOKEN_LOCK.lock().unwrap();
        clear_tokens();
        let ch = BearerChallenge {
            realm: "https://h/token".to_string(),
            service: Some("h".to_string()),
            scope: None,
        };
        store_token("h", &ch, Some("repository:r:pull"), "anonymous", "tok1");
        assert_eq!(
            cached_token(&ch, Some("repository:r:pull"), "anonymous").as_deref(),
            Some("tok1")
        );
        // a different class or scope is a different entry
        assert_eq!(
            cached_token(&ch, Some("repository:r:pull"), "token:env:X"),
            None
        );
        assert_eq!(
            cached_token(&ch, Some("repository:other:pull"), "anonymous"),
            None
        );
        // the host-addressed form sees it too (blob streams)
        assert_eq!(
            cached_token_for_host("h", "repository:r:pull", "anonymous").as_deref(),
            Some("tok1")
        );
        assert_eq!(
            cached_token_for_host("other-host", "repository:r:pull", "anonymous"),
            None
        );
        // eviction drops exactly the one entry
        evict_token(&ch, Some("repository:r:pull"), "anonymous");
        assert_eq!(
            cached_token(&ch, Some("repository:r:pull"), "anonymous"),
            None
        );
        clear_tokens();
    }
}
