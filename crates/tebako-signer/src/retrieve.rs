//! Key retrieval from the trust-anchor channel (spec 09 §10).
//!
//! A signed artifact whose pinned signer keyid is absent from the local
//! keyring is no longer a dead end for keys ON THE TAMATEBAKO ROOT
//! CHAIN: the public key is fetched from the trust-anchor publication
//! and admitted ONLY through the chain — trust never extends silently,
//! and never extends by TOFU.
//!
//! The well-known grammar (tebako.org owns the spellings; consumers flow
//! [`anchor_key_url`] / [`anchor_successor_url`], never re-derive):
//!
//! - `…/.well-known/tebako-keys/<keyid>.asc` — armored public key by
//!   PRIMARY keyid (16 lowercase hex).
//! - `…/.well-known/tebako-successors/<predecessor-fingerprint>.asc` —
//!   the `TEBAKO-ROOT-SUCCESSOR-V1` statement rotating FROM that
//!   fingerprint (40 uppercase hex).
//!
//! Admission (any one): the key's primary fingerprint IS a trusted root
//! (the embedded root or the `TEBAKO_TRUSTED_ROOT` override); it chains
//! to a trusted root through verified successor statements (bounded
//! walk); or it is already in the trusted keyring (no fetch at all).
//! Anything else is the named [`SignerError::KeyRetrieval`] failure.

use std::path::Path;

use crate::error::SignerError;
use crate::keyring::{fingerprint_of, primary_keyid_of, register_trusted, RegisterOutcome};
use crate::keys::{hex_lower, keyid_bytes_from_fingerprint};
use crate::root::ROOT_FINGERPRINT;
use crate::sign::{verify_detached_full, VerifyOutcome};

/// The trust-anchor publication base (tebako.org's `.well-known`).
pub const ANCHOR_BASE: &str = "https://www.tebako.org/.well-known";

/// The `TEBAKO_ANCHOR_BASE` override (air-gapped mirrors, tests — e.g. a
/// `file://` mirror of the directory). This is an AVAILABILITY knob,
/// never a trust knob: a retrieved key is admitted only by chaining to
/// the embedded root, so a malicious mirror cannot inject trust.
pub const ANCHOR_BASE_ENV: &str = "TEBAKO_ANCHOR_BASE";

/// The effective anchor base ([`ANCHOR_BASE_ENV`] over [`ANCHOR_BASE`]).
pub fn anchor_base() -> String {
    std::env::var(ANCHOR_BASE_ENV)
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| ANCHOR_BASE.to_string())
}

/// The successor-chain walk's hop bound — a cycle is a dead end, not a
/// hang.
pub const MAX_CHAIN_HOPS: usize = 8;

/// The URL serving the armored public key whose PRIMARY keyid is
/// `keyid` (16 hex, case-insensitive — the grammar spells lowercase).
pub fn anchor_key_url(keyid: &str) -> String {
    key_url(&anchor_base(), keyid)
}

/// The URL serving the successor statement rotating FROM
/// `predecessor_fingerprint` (40 hex — the grammar spells uppercase).
pub fn anchor_successor_url(predecessor_fingerprint: &str) -> String {
    successor_url(&anchor_base(), predecessor_fingerprint)
}

fn key_url(base: &str, keyid: &str) -> String {
    format!("{}/tebako-keys/{}.asc", base, keyid.to_lowercase())
}

fn successor_url(base: &str, predecessor_fingerprint: &str) -> String {
    format!(
        "{}/tebako-successors/{}.asc",
        base,
        predecessor_fingerprint.to_uppercase()
    )
}

/// How a retrieved key earned admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetrievalBasis {
    /// The key's primary fingerprint IS a trusted root.
    EmbeddedRoot,
    /// The key chains to a trusted root; payload is the verified
    /// fingerprint path from the root to the key.
    SuccessorChain(Vec<String>),
    /// The key was already in the trusted keyring (no fetch happened).
    AlreadyTrusted,
}

impl RetrievalBasis {
    /// The audit-journal rendering (`embedded-root` /
    /// `successor:<root>›…›<key>` / `already-trusted`).
    pub fn journal_label(&self) -> String {
        match self {
            RetrievalBasis::EmbeddedRoot => "embedded-root".to_string(),
            RetrievalBasis::SuccessorChain(path) => format!("successor:{}", path.join(">")),
            RetrievalBasis::AlreadyTrusted => "already-trusted".to_string(),
        }
    }
}

/// A completed key retrieval: everything the audit journal's row needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRetrieval {
    /// The pinned keyid that triggered the retrieval (16 lowercase hex).
    pub keyid: String,
    /// The admitted key's primary fingerprint (40 uppercase hex).
    pub fingerprint: String,
    /// The URL the key bytes came from (empty for `AlreadyTrusted`).
    pub source_url: String,
    /// The admission basis.
    pub basis: RetrievalBasis,
    /// Whether this call added the key to the trusted keyring.
    pub registered: bool,
}

/// A fetch of one URL — the production wrapper binds
/// `tebako_http::get`; tests bind fixtures. The error is display text
/// (status, URL context) for the named failure's message.
pub type Fetch<'a> = dyn Fn(&str) -> Result<Vec<u8>, String> + 'a;

/// The outcome of a verification that may have retrieved the signer's
/// key: the (possibly re-run) classification plus the retrieval record
/// when one happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    /// The final verification outcome.
    pub outcome: VerifyOutcome,
    /// The key retrieval, when the initial outcome was `Untrusted` and
    /// the ceremony ran.
    pub retrieval: Option<KeyRetrieval>,
}

/// The trusted root fingerprints the chain may start from: the embedded
/// root plus the `TEBAKO_TRUSTED_ROOT` dev override (a bare fingerprint,
/// or the fingerprint of the key a named file carries).
fn root_fingerprints() -> Vec<String> {
    let mut roots = vec![ROOT_FINGERPRINT.to_string()];
    let override_value = std::env::var("TEBAKO_TRUSTED_ROOT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(v) = override_value {
        if v.len() == 40 && v.bytes().all(|b| b.is_ascii_hexdigit()) {
            roots.push(v.to_uppercase());
        } else if let Some(key) = crate::root::trusted_root_override_key(Some(v)) {
            if let Ok(fp) = fingerprint_of(&key) {
                roots.push(fp.to_uppercase());
            }
        }
    }
    roots
}

fn keyid_grammar(keyid: &str) -> bool {
    keyid.len() == 16 && keyid.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Whether the keyring holds a key whose primary fingerprint is `fp`.
fn keyring_has_fingerprint(keyring: &[u8], fp: &str) -> Result<bool, SignerError> {
    if keyring.is_empty() {
        return Ok(false);
    }
    let ctx = rnp::Context::new().map_err(|e| SignerError::Trust(e.to_string()))?;
    ctx.load_keys(rnp::KeyringFormat::Gpg, keyring, rnp::LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::Trust(format!("trusted keyring is unreadable: {e}")))?;
    let mut fps = ctx
        .identifiers(rnp::IdentifierKind::Fingerprint)
        .map_err(|e| SignerError::Trust(e.to_string()))?;
    let want = fp.to_uppercase();
    Ok(fps.any(|f| f.to_uppercase() == want))
}

/// Walk the successor chain from `root` towards `target_fp`, fetching
/// statements (and intermediate keys, pinned by the verified statement
/// that named them) as it goes. Returns the verified fingerprint path
/// when the walk reaches `target_fp` within [`MAX_CHAIN_HOPS`]; `None`
/// on any dead end (no statement, bad signature, misbehaving server).
fn walk_chain(
    base: &str,
    root: &str,
    target_fp: &str,
    pool: &[u8],
    fetch: &Fetch<'_>,
) -> Option<Vec<String>> {
    let mut pool = pool.to_vec();
    let mut current = root.to_uppercase();
    let mut path = vec![current.clone()];
    for _ in 0..MAX_CHAIN_HOPS {
        if current == target_fp {
            return Some(path);
        }
        let statement = fetch(&successor_url(base, &current)).ok()?;
        let (stmt, outcome) = crate::root::verify_successor_statement(&pool, &statement).ok()?;
        if stmt.predecessor_fingerprint != current || !matches!(outcome, VerifyOutcome::Trusted(_))
        {
            return None;
        }
        let next = stmt.successor_fingerprint.clone();
        path.push(next.clone());
        if next == target_fp {
            return Some(path);
        }
        // The next hop verifies against `next`'s key: the verified
        // statement pins its fingerprint, so a served key whose
        // fingerprint matches is admitted to the pool for this walk.
        if !keyring_has_fingerprint(&pool, &next).ok()? {
            let keyid = hex_lower(&keyid_bytes_from_fingerprint(&next).ok()?);
            let key_bytes = fetch(&key_url(base, &keyid)).ok()?;
            if fingerprint_of(&key_bytes).ok()?.to_uppercase() != next {
                return None;
            }
            let binary = crate::sign::dearmor_bytes(&key_bytes).unwrap_or(key_bytes);
            pool.extend_from_slice(&binary);
        }
        current = next;
    }
    None
}

/// Retrieve the public key for `pinned_keyid` from the trust-anchor
/// channel and admit it per spec 09 §10. `Ok(None)` means the ceremony
/// did not run (`offline`) and the caller keeps the pre-ceremony
/// outcome; `Err` is the named `KeyRetrieval` failure carrying the
/// retrieved fingerprint for out-of-band comparison.
pub fn retrieve_signer_key(
    home: &Path,
    pinned_keyid: &str,
    offline: bool,
    fetch: &Fetch<'_>,
) -> Result<Option<KeyRetrieval>, SignerError> {
    retrieve_signer_key_with_base(home, &anchor_base(), pinned_keyid, offline, fetch)
}

/// [`retrieve_signer_key`] against an explicit publication base — the
/// caller resolved the base itself (a Ctx-snapshotted env, a test
/// fixture) instead of flowing [`anchor_base`]'s process env.
pub fn retrieve_signer_key_with_base(
    home: &Path,
    base: &str,
    pinned_keyid: &str,
    offline: bool,
    fetch: &Fetch<'_>,
) -> Result<Option<KeyRetrieval>, SignerError> {
    retrieve_with_roots(
        home,
        base,
        pinned_keyid,
        offline,
        fetch,
        &root_fingerprints(),
    )
}

/// The walk with an explicit root set — the seam tests drive (the dev
/// override env is process-global; parallel tests never race it).
fn retrieve_with_roots(
    home: &Path,
    base: &str,
    pinned_keyid: &str,
    offline: bool,
    fetch: &Fetch<'_>,
    roots: &[String],
) -> Result<Option<KeyRetrieval>, SignerError> {
    let keyid = pinned_keyid.to_lowercase();
    if !keyid_grammar(&keyid) {
        return Err(SignerError::KeyRetrieval(format!(
            "invalid pinned keyid (want 16 hex chars): {pinned_keyid}"
        )));
    }

    let pool = crate::root::verification_keyring(home)?;

    // Admission 3: already trusted — no fetch at all.
    if primary_keyid_of(&pool, &keyid)?.is_some() {
        let fingerprint = fingerprint_for_keyid(&pool, &keyid)?;
        return Ok(Some(KeyRetrieval {
            keyid,
            fingerprint,
            source_url: String::new(),
            basis: RetrievalBasis::AlreadyTrusted,
            registered: false,
        }));
    }

    if offline {
        return Ok(None);
    }

    let url = key_url(base, &keyid);
    let key_bytes = fetch(&url).map_err(|e| {
        SignerError::KeyRetrieval(format!(
            "the signer's key is not published on the trust-anchor channel ({url} does not resolve: {e})\n\
             if you trust this signer, import its public key manually:\n\
             \n    tebako keys import <file>"
        ))
    })?;
    let fingerprint = fingerprint_of(&key_bytes)
        .map_err(|e| SignerError::KeyRetrieval(format!("{url}: {e}")))?
        .to_uppercase();
    let served_keyid = hex_lower(&keyid_bytes_from_fingerprint(&fingerprint)?);
    if served_keyid != keyid {
        return Err(SignerError::KeyRetrieval(format!(
            "{url} served a key whose primary keyid is {served_keyid}, not the pinned {keyid}"
        )));
    }

    let roots: Vec<String> = roots.iter().map(|r| r.to_uppercase()).collect();
    let basis = if roots.contains(&fingerprint) {
        RetrievalBasis::EmbeddedRoot
    } else {
        let mut found = None;
        for root in &roots {
            if let Some(path) = walk_chain(base, root, &fingerprint, &pool, fetch) {
                found = Some(RetrievalBasis::SuccessorChain(path));
                break;
            }
        }
        match found {
            Some(basis) => basis,
            None => {
                return Err(SignerError::KeyRetrieval(format!(
                    "the key served for {keyid} (fingerprint {fingerprint}) is not on the \
                     tamatebako root chain — it is NOT trusted.\n\
                     Confirm the fingerprint out of band, then import the key manually:\n\
                     \n    tebako keys import <file>\n\
                     \n\
                     fingerprint: {fingerprint}"
                )))
            }
        }
    };

    let registered = matches!(
        register_trusted(home, &key_bytes)?,
        RegisterOutcome::Added(_)
    );
    Ok(Some(KeyRetrieval {
        keyid,
        fingerprint,
        source_url: url,
        basis,
        registered,
    }))
}

/// The primary fingerprint of the key holding `keyid` in the keyring.
fn fingerprint_for_keyid(keyring: &[u8], keyid: &str) -> Result<String, SignerError> {
    let ctx = rnp::Context::new().map_err(|e| SignerError::Trust(e.to_string()))?;
    ctx.load_keys(rnp::KeyringFormat::Gpg, keyring, rnp::LoadSaveFlags::PUBLIC)
        .map_err(|e| SignerError::Trust(format!("trusted keyring is unreadable: {e}")))?;
    let key = ctx
        .find_key(rnp::KeyIdentifier::Keyid(&keyid.to_lowercase()))
        .map_err(|e| SignerError::Trust(e.to_string()))?
        .ok_or_else(|| SignerError::Trust(format!("keyid {keyid} vanished from the keyring")))?;
    match key.primary_fprint() {
        Ok(Some(fp)) => Ok(fp.to_uppercase()),
        Ok(None) | Err(_) => Ok(key
            .fingerprint()
            .map_err(|e| SignerError::Trust(e.to_string()))?
            .to_uppercase()),
    }
}

/// Verify a detached signature, running the key-retrieval ceremony when
/// the signer's key is unknown: an `Untrusted` outcome with a usable
/// keyid (the registry's pin when given, else the signature's issuer)
/// fetches and admits the key per spec 09 §10, then re-verifies against
/// the widened keyring. `TEBAKO_OFFLINE` keeps the pre-ceremony outcome.
pub fn verify_with_retrieval(
    home: &Path,
    data: &[u8],
    signature: &[u8],
    pinned_keyid: Option<&str>,
    offline: bool,
    fetch: &Fetch<'_>,
) -> Result<VerifyReport, SignerError> {
    let ring = crate::root::verification_keyring(home)?;
    let outcome = verify_detached_full(&ring, data, signature)?;
    let VerifyOutcome::Untrusted(issuer_keyid) = &outcome else {
        return Ok(VerifyReport {
            outcome,
            retrieval: None,
        });
    };

    let keyid = pinned_keyid
        .filter(|k| keyid_grammar(&k.to_lowercase()))
        .map(|k| k.to_lowercase())
        .unwrap_or_else(|| issuer_keyid.clone());
    if !keyid_grammar(&keyid) {
        return Ok(VerifyReport {
            outcome,
            retrieval: None,
        });
    }

    let Some(retrieval) = retrieve_signer_key(home, &keyid, offline, fetch)? else {
        return Ok(VerifyReport {
            outcome,
            retrieval: None,
        });
    };
    let ring = crate::root::verification_keyring(home)?;
    let outcome = verify_detached_full(&ring, data, signature)?;
    Ok(VerifyReport {
        outcome,
        retrieval: Some(retrieval),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-retrieve-test-{}-{name}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_key(userid: &str) -> (Vec<u8>, Vec<u8>, String) {
        let ctx = rnp::Context::new().unwrap();
        let key = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
            .hash(rnp::Hash::Sha256)
            .userid(userid)
            .add_usage(rnp::KeyUsage::Sign)
            .build(&ctx)
            .unwrap();
        let secret = key
            .export(
                rnp::ExportFlags::ARMORED | rnp::ExportFlags::SECRET | rnp::ExportFlags::SUBKEYS,
            )
            .unwrap();
        let public = key
            .export(
                rnp::ExportFlags::ARMORED | rnp::ExportFlags::PUBLIC | rnp::ExportFlags::SUBKEYS,
            )
            .unwrap();
        let fp = key.fingerprint().unwrap().to_uppercase();
        (secret, public, fp)
    }

    fn keyid_of(fp: &str) -> String {
        hex_lower(&keyid_bytes_from_fingerprint(fp).unwrap())
    }

    /// A fixture "trust-anchor server": URL → bytes, 404 for anything
    /// else.
    struct Fixture {
        pages: HashMap<String, Vec<u8>>,
        misses: std::sync::Mutex<Vec<String>>,
    }

    impl Fixture {
        fn new() -> Self {
            Fixture {
                pages: HashMap::new(),
                misses: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn serve_key(&mut self, public: &[u8], fp: &str) {
            self.pages
                .insert(anchor_key_url(&keyid_of(fp)), public.to_vec());
        }

        fn serve_statement(&mut self, predecessor_fp: &str, statement: &[u8]) {
            self.pages
                .insert(anchor_successor_url(predecessor_fp), statement.to_vec());
        }

        fn fetcher(&self) -> impl Fn(&str) -> Result<Vec<u8>, String> + '_ {
            move |url| match self.pages.get(url) {
                Some(bytes) => Ok(bytes.clone()),
                None => {
                    self.misses.lock().unwrap().push(url.to_string());
                    Err(format!("404 Not Found: {url}"))
                }
            }
        }
    }

    #[test]
    fn url_grammar() {
        assert_eq!(
            anchor_key_url("ABCDEF0123456789"),
            "https://www.tebako.org/.well-known/tebako-keys/abcdef0123456789.asc"
        );
        assert_eq!(
            anchor_successor_url("9e210ca8e9fde9e6587740b2efc3c250f7862a48"),
            "https://www.tebako.org/.well-known/tebako-successors/9E210CA8E9FDE9E6587740B2EFC3C250F7862A48.asc"
        );
    }

    #[test]
    fn embedded_root_admits_directly() {
        let home = home("root");
        let (_secret, public, fp) = make_key("dev-root");
        let mut fixture = Fixture::new();
        fixture.serve_key(&public, &fp);
        let roots = vec![fp.clone()];

        let got = retrieve_with_roots(
            &home,
            ANCHOR_BASE,
            &keyid_of(&fp),
            false,
            &fixture.fetcher(),
            &roots,
        )
        .unwrap()
        .expect("a retrieval happened");
        assert_eq!(got.fingerprint, fp);
        assert_eq!(got.basis, RetrievalBasis::EmbeddedRoot);
        assert!(got.registered);
        // … and the keyring now holds it.
        assert!(primary_keyid_of(
            &crate::keyring::trusted_keyring_bytes(&home).unwrap(),
            &keyid_of(&fp)
        )
        .unwrap()
        .is_some());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn successor_chain_admits_the_tip() {
        let home = home("chain");
        let (root_secret, root_public, root_fp) = make_key("chain-root");
        let (s1_secret, s1_public, s1_fp) = make_key("chain-s1");
        let (_s2_secret, s2_public, s2_fp) = make_key("chain-s2");

        // The root key is locally trusted (the dev-override shape);
        // s1 and s2 arrive through the anchor channel.
        register_trusted(&home, &root_public).unwrap();
        let mut fixture = Fixture::new();
        fixture.serve_key(&s2_public, &s2_fp);
        fixture.serve_key(&s1_public, &s1_fp);
        let hop1 = crate::root::sign_successor_statement(&root_secret, &root_fp, &s1_fp).unwrap();
        let hop2 = crate::root::sign_successor_statement(&s1_secret, &s1_fp, &s2_fp).unwrap();
        fixture.serve_statement(&root_fp, &hop1);
        fixture.serve_statement(&s1_fp, &hop2);

        let roots = vec![root_fp.clone()];
        let got = retrieve_with_roots(
            &home,
            ANCHOR_BASE,
            &keyid_of(&s2_fp),
            false,
            &fixture.fetcher(),
            &roots,
        )
        .unwrap()
        .expect("a retrieval happened");
        assert_eq!(got.fingerprint, s2_fp);
        assert_eq!(
            got.basis,
            RetrievalBasis::SuccessorChain(vec![root_fp, s1_fp, s2_fp])
        );
        assert!(got.registered);
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn unknown_key_is_the_named_loud_error() {
        let home = home("unknown");
        let (_root_secret, root_public, root_fp) = make_key("u-root");
        let (_rogue_secret, rogue_public, rogue_fp) = make_key("rogue");
        register_trusted(&home, &root_public).unwrap();
        let mut fixture = Fixture::new();
        fixture.serve_key(&rogue_public, &rogue_fp);
        // No successor statements at all: the walk dies at the root.

        let roots = vec![root_fp];
        let err = retrieve_with_roots(
            &home,
            ANCHOR_BASE,
            &keyid_of(&rogue_fp),
            false,
            &fixture.fetcher(),
            &roots,
        )
        .unwrap_err();
        let text = err.to_string();
        assert!(
            matches!(err, SignerError::KeyRetrieval(_)),
            "named KeyRetrieval: {text}"
        );
        assert!(text.contains(&rogue_fp), "the fingerprint is loud: {text}");
        assert!(
            text.contains("tebako keys import"),
            "the manual path: {text}"
        );
        // The key was NOT registered.
        assert!(
            crate::keyring::trusted_keyring_bytes(&home)
                .unwrap()
                .is_empty()
                || primary_keyid_of(
                    &crate::keyring::trusted_keyring_bytes(&home).unwrap(),
                    &keyid_of(&rogue_fp)
                )
                .unwrap()
                .is_none()
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn offline_skips_the_ceremony() {
        let home = home("offline");
        let (_secret, public, fp) = make_key("off-root");
        let fixture = Fixture::new(); // serves nothing; must not be asked
        let roots = vec![fp.clone()];
        let got = retrieve_with_roots(
            &home,
            ANCHOR_BASE,
            &keyid_of(&fp),
            true,
            &fixture.fetcher(),
            &roots,
        )
        .unwrap();
        assert!(got.is_none());
        assert!(fixture.misses.lock().unwrap().is_empty());
        let _ = public;
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn already_trusted_never_fetches() {
        let home = home("trusted");
        let (_secret, public, fp) = make_key("t-root");
        register_trusted(&home, &public).unwrap();
        let fixture = Fixture::new();
        let roots = vec![fp.clone()];
        let got = retrieve_with_roots(
            &home,
            ANCHOR_BASE,
            &keyid_of(&fp),
            false,
            &fixture.fetcher(),
            &roots,
        )
        .unwrap()
        .expect("a retrieval record");
        assert_eq!(got.basis, RetrievalBasis::AlreadyTrusted);
        assert!(!got.registered);
        assert!(fixture.misses.lock().unwrap().is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn keyid_mismatch_is_a_named_error() {
        let home = home("mismatch");
        let (_secret, public, fp) = make_key("m-root");
        let mut fixture = Fixture::new();
        fixture.serve_key(&public, &fp);
        let roots = vec![fp];
        let err = retrieve_with_roots(
            &home,
            ANCHOR_BASE,
            "0000000000000000",
            false,
            &fixture.fetcher(),
            &roots,
        )
        .unwrap_err();
        assert!(matches!(err, SignerError::KeyRetrieval(_)));
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn verify_with_retrieval_closes_the_gap() {
        let home = home("vwr");
        let (root_secret, root_public, root_fp) = make_key("v-root");
        let (s1_secret, s1_public, s1_fp) = make_key("v-s1");
        register_trusted(&home, &root_public).unwrap();

        let mut fixture = Fixture::new();
        fixture.serve_key(&s1_public, &s1_fp);
        let hop = crate::root::sign_successor_statement(&root_secret, &root_fp, &s1_fp).unwrap();
        fixture.serve_statement(&root_fp, &hop);

        let data = b"a signed payload";
        let sig = crate::sign::sign_detached(data, &s1_secret, &s1_fp).unwrap();

        // The ceremony: Untrusted → retrieve → Trusted.
        // TEBAKO_TRUSTED_ROOT would be process-global; drive the roots
        // through the keyring instead — the embedded root const is not
        // this test's root, so verify against retrieve_with_roots's
        // logic via verify_with_retrieval is not reachable here. Use the
        // public flow with the root registered and the env unset: the
        // walk starts from the embedded root, which this chain is not
        // under — so this test drives retrieve + re-verify directly.
        let roots = vec![root_fp];
        let retrieval = retrieve_with_roots(
            &home,
            ANCHOR_BASE,
            &keyid_of(&s1_fp),
            false,
            &fixture.fetcher(),
            &roots,
        )
        .unwrap()
        .expect("retrieved");
        assert_eq!(retrieval.fingerprint, s1_fp);
        let ring = crate::root::verification_keyring(&home).unwrap();
        let outcome = verify_detached_full(&ring, data, &sig).unwrap();
        assert!(matches!(outcome, VerifyOutcome::Trusted(_)));
        std::fs::remove_dir_all(&home).ok();
    }
}
