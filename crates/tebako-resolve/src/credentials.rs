//! The credential model (spec 37 §5 — locked confinement): the config's
//! `credentials:` section parsed into a [`CredentialBook`], the two-tier
//! most-specific-wins lookup, and the service-class header shaping — one
//! place, so no fetch path re-implements a piece of it.
//!
//! The locked rules:
//!
//! - **Config holds env var NAMES, never secrets.** The token value is
//!   read from the environment at decide time; an unset (or empty) var
//!   behaves as absent — the fetch goes anonymous, the service's
//!   ordinary 401/403 becomes the named `CredentialRequired` naming the
//!   registry and the env var it looked for.
//! - **Two tiers, most-specific-wins, exactly one match.** Tier 1 keys
//!   on the alias of the registry whose row directed the fetch
//!   (registry-file fetches match their own registry's alias, via
//!   [`CredentialBook::alias_of`]); tier 2 keys on the URL's exact
//!   host; then the ambient `TEBAKO_GITHUB_TOKEN` / `GITHUB_TOKEN`
//!   (unchanged — tebako-http's one host rule, ranked as the github.com
//!   tier); else anonymous. No third tier, no chain: a matched entry
//!   whose env var is unset does NOT fall through.
//! - **Confinement (locked).** A credential is presented ONLY to the
//!   host it was minted for: tier-1 to the registry ref's own host plus
//!   that service's API host, tier-2 to the named host exactly — never
//!   cross-host, never to a redirect target (ureq already never
//!   forwards auth headers on redirect — `RedirectAuthHeaders::Never`
//!   is the default). A registry row whose `release.ref` points at a
//!   DIFFERENT host falls to that host's own tier-2 entry or fails
//!   named; the directing registry's credential does not follow.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::reference::Service;

/// One `credentials:` entry (spec 37 §5): exactly one selector —
/// `registry: <alias>` (tier 1) or `host: <host>` (tier 2) — plus the
/// env var NAME holding the token. Both selectors or neither is a named
/// config error at validation (tebako-shim's config load), as is a
/// duplicate tier-1 alias or tier-2 host (`DuplicateCredentialSelector`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialEntry {
    /// Tier 1: the exact registry-alias match.
    pub registry: Option<String>,
    /// Tier 2: the host fallback.
    pub host: Option<String>,
    /// The env var NAME the token is read from at decide time.
    pub token_env: String,
}

impl<'de> Deserialize<'de> for CredentialEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct EntryVisitor;
        impl<'de> serde::de::Visitor<'de> for EntryVisitor {
            type Value = CredentialEntry;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a {registry | host, token_env} map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<CredentialEntry, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                use serde::de::Error as _;
                let mut registry = None;
                let mut host = None;
                let mut token_env = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "registry" => registry = Some(map.next_value::<String>()?),
                        "host" => host = Some(map.next_value::<String>()?),
                        "token_env" => token_env = Some(map.next_value::<String>()?),
                        // Unknown keys are NOT ignored here (unlike the
                        // registry book's forward-compat leniency): a
                        // misspelled selector would silently degrade a
                        // credential to an anonymous fetch.
                        other => {
                            return Err(A::Error::custom(format!(
                                "a `credentials` entry's keys are registry/host/token_env, not '{other}'"
                            )))
                        }
                    }
                }
                let Some(token_env) = token_env else {
                    return Err(A::Error::custom(
                        "a `credentials` entry needs a `token_env:` key",
                    ));
                };
                Ok(CredentialEntry {
                    registry,
                    host,
                    token_env,
                })
            }
        }
        deserializer.deserialize_map(EntryVisitor)
    }
}

/// One tier-1 row: the registry alias, the env var NAME, and the hosts
/// the token may be presented to — the registry ref's own host plus the
/// service's API host, derived from the adapters' base-URL construction
/// by the config layer (tebako-shim), never re-mapped here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tier1Entry {
    pub alias: String,
    pub token_env: String,
    pub allowed_hosts: BTreeSet<String>,
}

/// The validated credential book (spec 37 §5).
#[derive(Debug, Clone, Default)]
pub struct CredentialBook {
    /// Tier 1, keyed by registry alias.
    pub tier1: Vec<Tier1Entry>,
    /// Tier 2: `(host, token_env)` — the exact-host fallback.
    pub tier2: Vec<(String, String)>,
    /// `(canonical registry ref, alias)` — the fetch-time reverse
    /// lookup: a registry-file fetch matches its own registry's alias.
    pub ref_index: Vec<(String, String)>,
}

/// The outcome of one fetch's credential lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Attach this header (the service-class shape, value included —
    /// redacted everywhere else). `class` is the journal's credential
    /// class: `token:env:<NAME>`.
    Attach {
        header_name: &'static str,
        header_value: String,
        class: String,
    },
    /// Ride anonymous. `looked_for` carries the env var NAME a MATCHED
    /// entry wanted (the match happened; the token was absent — no
    /// fall-through), None when nothing matched at all.
    Anonymous { looked_for: Option<String> },
}

impl CredentialBook {
    /// True when the book holds no entries — decisions then
    /// short-circuit to the pre-spec-37 behavior (the ambient github
    /// token, else anonymous) and the fetch journal stays silent (the
    /// audit trail's subject is the mixed federation; book-empty
    /// machines journal nothing).
    pub fn is_empty(&self) -> bool {
        self.tier1.is_empty() && self.tier2.is_empty()
    }

    /// The alias a canonical registry ref maps to (the ref_index
    /// reverse lookup) — `resolve_registry`'s tier-1 key.
    pub fn alias_of(&self, canonical_ref: &str) -> Option<String> {
        self.ref_index
            .iter()
            .find(|(r, _)| r == canonical_ref)
            .map(|(_, a)| a.clone())
    }

    /// The two-tier most-specific-wins lookup (spec 37 §5): tier 1 by
    /// the directing registry's alias (attached ONLY when the URL's
    /// host is in the entry's allowed set — a confined-out row falls
    /// through to the host chain, the credential does not follow the
    /// ref), tier 2 by the URL's exact host, the ambient github token,
    /// else anonymous.
    pub fn decide(&self, alias: Option<&str>, url: &str, service: Option<Service>) -> Decision {
        let host = url_host(url);
        if let Some(alias) = alias {
            if let Some(entry) = self.tier1.iter().find(|e| e.alias == alias) {
                if host.is_some_and(|h| entry.allowed_hosts.contains(h)) {
                    return attach_or_absent(&entry.token_env, service);
                }
            }
        }
        if let Some(host) = host {
            if let Some((_, token_env)) = self.tier2.iter().find(|(h, _)| h == host) {
                return attach_or_absent(token_env, service);
            }
        }
        // The ambient tier: TEBAKO_GITHUB_TOKEN / GITHUB_TOKEN rides
        // tebako-http's ONE host rule (the github.com tier of spec 37
        // §5, unchanged by the book).
        if tebako_http::carries_ambient_github_token(url) {
            if let Some(token) = tebako_http::github_token_from_env() {
                let (header_name, header_value) = header_for(service, &token);
                return Decision::Attach {
                    header_name,
                    header_value,
                    class: format!("token:env:{}", ambient_env_name()),
                };
            }
        }
        Decision::Anonymous { looked_for: None }
    }

    /// Could this book authenticate a fetch to any of `hosts`? (The
    /// transport wrapper's `authenticated()` — adapters consult it to
    /// pick API asset URLs over browser URLs for private repos.) A
    /// credential COUNTS only when its env var is set: an
    /// entry-without-token must not steer the fetch onto the
    /// authenticated URL shape it cannot satisfy.
    pub fn can_authenticate(&self, hosts: &BTreeSet<String>) -> bool {
        self.tier1
            .iter()
            .any(|e| !e.allowed_hosts.is_disjoint(hosts) && token_set(&e.token_env))
            || self
                .tier2
                .iter()
                .any(|(h, env)| hosts.contains(h) && token_set(env))
    }
}

/// The env var read at decide time: empty counts as unset (spec 37 §5 —
/// a credential whose env var is unset behaves as absent).
fn token_set(env_name: &str) -> bool {
    std::env::var(env_name).is_ok_and(|v| !v.is_empty())
}

/// A matched entry: attach its token, or — the env var unset — the
/// absent-token anonymous naming what it looked for (NO fall-through:
/// the match happened; the token was absent).
fn attach_or_absent(token_env: &str, service: Option<Service>) -> Decision {
    match std::env::var(token_env).ok().filter(|v| !v.is_empty()) {
        Some(token) => {
            let (header_name, header_value) = header_for(service, &token);
            Decision::Attach {
                header_name,
                header_value,
                class: format!("token:env:{token_env}"),
            }
        }
        None => Decision::Anonymous {
            looked_for: Some(token_env.to_string()),
        },
    }
}

/// Which ambient env var supplied the token (the journal class).
fn ambient_env_name() -> &'static str {
    if token_set("TEBAKO_GITHUB_TOKEN") {
        "TEBAKO_GITHUB_TOKEN"
    } else {
        "GITHUB_TOKEN"
    }
}

/// The header shape by service class (spec 37 §5 — the adapter owns the
/// shape; authored config never spells header mechanics): GitHub/GHE
/// `Authorization: Bearer`, GitLab `PRIVATE-TOKEN`, Bitbucket Cloud
/// `Authorization: Basic` (the env value is the `user:app-password`
/// pair, base64'd here), generic HTTPS (no service) `Bearer`.
pub fn header_for(service: Option<Service>, token: &str) -> (&'static str, String) {
    match service {
        Some(Service::Gitlab) => ("PRIVATE-TOKEN", token.to_string()),
        Some(Service::Bitbucket) => (
            "Authorization",
            format!("Basic {}", base64_encode(token.as_bytes())),
        ),
        Some(Service::Github) | None => ("Authorization", format!("Bearer {token}")),
    }
}

/// Standard base64 with padding (the Bitbucket Basic credential). The
/// decode half lives in adapters.rs (the contents API's `content`) —
/// one alphabet, both directions, this the only encoder.
pub fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let acc = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(acc >> 18) as usize & 63] as char);
        out.push(ALPHABET[(acc >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(acc >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[acc as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// The `host[:port]` an http(s) URL names (None for `file://` and every
/// non-URL string — those fetches never carry a credential).
pub fn url_host(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    rest.split('/').next().filter(|h| !h.is_empty())
}

// ---------------------------------------------------------------------
// the process-global book + journal home (tebako-http's netconfig
// pattern: OnceLock<RwLock>, re-installable — tebako-shim's config load
// is the ONE install point, so the cli, the shim dispatch, and pkg
// decide with no call-site churn)
// ---------------------------------------------------------------------

static BOOK: OnceLock<RwLock<CredentialBook>> = OnceLock::new();

fn book_slot() -> &'static RwLock<CredentialBook> {
    BOOK.get_or_init(|| RwLock::new(CredentialBook::default()))
}

/// Install the process-wide credential book. Book-empty configs install
/// the empty book — decisions short-circuit to the ambient/anonymous
/// behavior and the journal stays silent.
pub fn install_book(book: CredentialBook) {
    *book_slot().write().unwrap_or_else(|e| e.into_inner()) = book;
}

/// The installed book (an EMPTY book when none was installed).
pub fn book() -> CredentialBook {
    book_slot()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

static JOURNAL_HOME: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();

fn journal_slot() -> &'static RwLock<Option<PathBuf>> {
    JOURNAL_HOME.get_or_init(|| RwLock::new(None))
}

/// The tebako home the fetch journal appends to (set beside
/// [`install_book`] at config load). None = no fetch journal
/// (tebako-resolve used standalone, or before any config load).
pub fn set_journal_home(home: Option<PathBuf>) {
    *journal_slot().write().unwrap_or_else(|e| e.into_inner()) = home;
}

/// The installed journal home, if any.
pub fn journal_home() -> Option<PathBuf> {
    journal_slot()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Append one fetch-decision line (`event=fetch host=<h>
/// credential=<class>` — the credential CLASS, the value always
/// redacted) to `<home>/journal.log`. Best-effort, like every journal
/// write: a journal error never fails the fetch.
pub(crate) fn journal_fetch(home: &Path, host: &str, class: &str) {
    use std::io::Write as _;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("journal.log"))
    {
        let _ = writeln!(f, "{now} event=fetch host={host} credential={class}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    const VAR: &str = "TEBAKO_TEST_CREDENTIAL_TOKEN";
    const VAR2: &str = "TEBAKO_TEST_CREDENTIAL_TOKEN_TWO";

    fn tier1_book() -> CredentialBook {
        CredentialBook {
            tier1: vec![Tier1Entry {
                alias: "nist".to_string(),
                token_env: VAR.to_string(),
                allowed_hosts: BTreeSet::from([
                    "api.github.com".to_string(),
                    "github.com".to_string(),
                ]),
            }],
            tier2: vec![("ghe.corp.internal".to_string(), VAR2.to_string())],
            ref_index: vec![(
                "tfs:github:metanorma/metanorma-flavor-nist".to_string(),
                "nist".to_string(),
            )],
        }
    }

    #[test]
    fn tier1_hit_attaches_the_bearer_and_the_class_names_the_env() {
        let _guard = env_guard();
        std::env::set_var(VAR, "sekrit");
        let d = tier1_book().decide(
            Some("nist"),
            "https://api.github.com/repos/o/r/contents/tpkg-registry.yaml",
            Some(Service::Github),
        );
        std::env::remove_var(VAR);
        assert_eq!(
            d,
            Decision::Attach {
                header_name: "Authorization",
                header_value: "Bearer sekrit".to_string(),
                class: format!("token:env:{VAR}"),
            }
        );
    }

    #[test]
    fn tier1_confined_out_falls_to_the_host_chain_not_the_credential() {
        let _guard = env_guard();
        std::env::set_var(VAR, "sekrit");
        std::env::set_var(VAR2, "ghe-token");
        let book = tier1_book();
        // the release.ref points at the GHE host: the nist credential
        // does NOT follow — the host's own tier-2 entry answers.
        let d = book.decide(
            Some("nist"),
            "https://ghe.corp.internal/api/v3/repos/o/r/releases/tags/v1",
            Some(Service::Github),
        );
        assert_eq!(
            d,
            Decision::Attach {
                header_name: "Authorization",
                header_value: "Bearer ghe-token".to_string(),
                class: format!("token:env:{VAR2}"),
            }
        );
        // …and with no tier-2 entry either, the answer is anonymous —
        // never the confined-out tier-1 token.
        let d = book.decide(
            Some("nist"),
            "https://other.example.com/x.tfs",
            Some(Service::Github),
        );
        assert_eq!(d, Decision::Anonymous { looked_for: None });
        std::env::remove_var(VAR);
        std::env::remove_var(VAR2);
    }

    #[test]
    fn tier1_matched_but_env_unset_is_absent_with_no_fall_through() {
        let _guard = env_guard();
        std::env::remove_var(VAR);
        let d = tier1_book().decide(
            Some("nist"),
            "https://api.github.com/repos/o/r/releases/tags/v1",
            Some(Service::Github),
        );
        assert_eq!(
            d,
            Decision::Anonymous {
                looked_for: Some(VAR.to_string())
            }
        );
        // empty counts as unset
        std::env::set_var(VAR, "");
        let d = tier1_book().decide(
            Some("nist"),
            "https://api.github.com/repos/o/r/releases/tags/v1",
            Some(Service::Github),
        );
        std::env::remove_var(VAR);
        assert_eq!(
            d,
            Decision::Anonymous {
                looked_for: Some(VAR.to_string())
            }
        );
    }

    #[test]
    fn tier2_matches_the_exact_host_only() {
        let _guard = env_guard();
        std::env::set_var(VAR2, "ghe-token");
        let book = tier1_book();
        let d = book.decide(
            None,
            "https://ghe.corp.internal/api/v3/repos/o/r/releases/tags/v1",
            Some(Service::Github),
        );
        assert!(matches!(
            d,
            Decision::Attach { ref header_value, .. } if header_value == "Bearer ghe-token"
        ));
        // no suffix games: a subdomain of the named host is a different host
        let d = book.decide(
            None,
            "https://evil.ghe.corp.internal/x",
            Some(Service::Github),
        );
        assert_eq!(d, Decision::Anonymous { looked_for: None });
        std::env::remove_var(VAR2);
    }

    #[test]
    fn the_ambient_token_rides_the_github_api_host_only() {
        let _guard = env_guard();
        let saved: Vec<(&str, Option<String>)> = ["TEBAKO_GITHUB_TOKEN", "GITHUB_TOKEN"]
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();
        std::env::remove_var("TEBAKO_GITHUB_TOKEN");
        std::env::set_var("GITHUB_TOKEN", "ci-token");
        let book = CredentialBook::default();
        let d = book.decide(
            None,
            "https://api.github.com/repos/o/r/releases/tags/v1",
            Some(Service::Github),
        );
        assert_eq!(
            d,
            Decision::Attach {
                header_name: "Authorization",
                header_value: "Bearer ci-token".to_string(),
                class: "token:env:GITHUB_TOKEN".to_string(),
            }
        );
        // …and nowhere else
        let d = book.decide(
            None,
            "https://github.com/o/r/releases/download/v1/a.tfs",
            None,
        );
        assert_eq!(d, Decision::Anonymous { looked_for: None });
        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }

    #[test]
    fn an_empty_book_and_no_alias_answer_anonymous() {
        let _guard = env_guard();
        let saved_te = std::env::var("TEBAKO_GITHUB_TOKEN").ok();
        let saved_gh = std::env::var("GITHUB_TOKEN").ok();
        std::env::remove_var("TEBAKO_GITHUB_TOKEN");
        std::env::remove_var("GITHUB_TOKEN");
        let book = CredentialBook::default();
        assert!(book.is_empty());
        assert_eq!(
            book.decide(None, "https://example.com/x", None),
            Decision::Anonymous { looked_for: None }
        );
        // a tier-1 alias the book does not carry is simply no match
        let book = tier1_book();
        assert!(!book.is_empty());
        assert_eq!(
            book.decide(Some("nosuch"), "https://example.com/x", None),
            Decision::Anonymous { looked_for: None }
        );
        // file:// never carries a credential
        assert_eq!(
            book.decide(Some("nist"), "file:///tmp/x.tfs", None),
            Decision::Anonymous { looked_for: None }
        );
        match saved_te {
            Some(v) => std::env::set_var("TEBAKO_GITHUB_TOKEN", v),
            None => std::env::remove_var("TEBAKO_GITHUB_TOKEN"),
        }
        match saved_gh {
            Some(v) => std::env::set_var("GITHUB_TOKEN", v),
            None => std::env::remove_var("GITHUB_TOKEN"),
        }
    }

    #[test]
    fn alias_of_reverse_looks_up_the_canonical_ref() {
        let book = tier1_book();
        assert_eq!(
            book.alias_of("tfs:github:metanorma/metanorma-flavor-nist"),
            Some("nist".to_string())
        );
        assert_eq!(book.alias_of("tfs:github:acme/other"), None);
    }

    #[test]
    fn header_for_shapes_by_service_class() {
        assert_eq!(
            header_for(Some(Service::Github), "tok"),
            ("Authorization", "Bearer tok".to_string())
        );
        assert_eq!(
            header_for(Some(Service::Gitlab), "tok"),
            ("PRIVATE-TOKEN", "tok".to_string())
        );
        assert_eq!(
            header_for(None, "tok"),
            ("Authorization", "Bearer tok".to_string())
        );
        // Bitbucket Cloud: Basic of the raw `user:app-password` value
        let (name, value) = header_for(Some(Service::Bitbucket), "user:app-password");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Basic dXNlcjphcHAtcGFzc3dvcmQ=");
        assert_eq!(
            base64_encode(b"user:app-password"),
            "dXNlcjphcHAtcGFzc3dvcmQ="
        );
    }

    #[test]
    fn base64_encode_pads_and_round_trips() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn url_host_extracts_host_and_optional_port() {
        assert_eq!(
            url_host("https://ghe.corp.internal:8443/api/v3/x"),
            Some("ghe.corp.internal:8443")
        );
        assert_eq!(url_host("https://api.github.com/x"), Some("api.github.com"));
        assert_eq!(url_host("file:///tmp/x"), None);
        assert_eq!(url_host("not a url"), None);
    }

    #[test]
    fn can_authenticate_requires_the_env_var_set() {
        let _guard = env_guard();
        let book = tier1_book();
        let gh = BTreeSet::from(["api.github.com".to_string(), "github.com".to_string()]);
        std::env::remove_var(VAR);
        assert!(!book.can_authenticate(&gh));
        std::env::set_var(VAR, "sekrit");
        assert!(book.can_authenticate(&gh));
        std::env::remove_var(VAR);
        // tier-2 by host
        let ghe = BTreeSet::from(["ghe.corp.internal".to_string()]);
        assert!(!book.can_authenticate(&ghe));
        std::env::set_var(VAR2, "ghe-token");
        assert!(book.can_authenticate(&ghe));
        std::env::remove_var(VAR2);
        // disjoint hosts: no
        let other = BTreeSet::from(["other.example.com".to_string()]);
        assert!(!book.can_authenticate(&other));
    }
}
