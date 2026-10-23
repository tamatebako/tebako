//! Registry transparency (spec 41): the RFC 6962-shaped tree over a
//! registry's canonical rows, and the signed head log that proves the
//! public fabric only ever grew.
//!
//! A signature over a registry proves who wrote *those bytes*; it says
//! nothing about what was there last month. Yanking a row and re-signing
//! is indistinguishable from the row never existing, and no mirror can
//! tell. The tree answers the missing question — COMPLETENESS — with the
//! construction third-party tooling already implements:
//!
//! - every row is a leaf (`SHA-256(0x00 ‖ line)`), interior nodes are
//!   `SHA-256(0x01 ‖ left ‖ right)`, and the fold is RFC 6962's (split at
//!   the largest power of two strictly smaller than the leaf count, an odd
//!   tail carried up, never padded);
//! - one row's presence is provable in `log2(n)` hashes without shipping
//!   the log;
//! - a chain of heads (`prev` = the previous root) proves the log only
//!   grew, and a fork is named at the head where it happens.
//!
//! This module owns the math and the head wire format. The registry
//! document → canonical lines mapping lives in the verb layer, so the tree
//! never depends on the registry model (and can be tested against
//! fixtures).

use sha2::{Digest, Sha256};

use crate::error::TebakoError;

const EX_USAGE: i32 = 64;

/// A transparency finding (spec 40 §6's ledger class, reused: both streams
/// are record streams whose integrity findings are one kind of event).
pub const EX_TEBAKO_LEDGER: i32 = tpkg::EX_TEBAKO_LEDGER;

/// The head payload's first line (spec 41 §4).
const HEAD_PAYLOAD_TAG: &str = "tebako-registry-log-1";

// ------------------------------------------------------------------- tree

/// `SHA-256(0x00 ‖ line)` — the domain separator makes a leaf hash
/// unforgeable as an interior node.
pub fn leaf_hash(line: &[u8]) -> [u8; 32] {
    let mut d = Sha256::new();
    d.update([0x00u8]);
    d.update(line);
    d.finalize().into()
}

/// `SHA-256(0x01 ‖ left ‖ right)`.
pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut d = Sha256::new();
    d.update([0x01u8]);
    d.update(left);
    d.update(right);
    d.finalize().into()
}

/// The Merkle Tree Hash of `lines` (RFC 6962 §2.1). An empty registry has
/// the defined root `SHA-256("")` — named, never a panic, because a
/// registry with no rows is a legitimate state and its root must still be
/// a value the log can chain.
pub fn tree_root<T: AsRef<[u8]>>(lines: &[T]) -> [u8; 32] {
    match lines.len() {
        0 => Sha256::digest([]).into(),
        1 => leaf_hash(lines[0].as_ref()),
        n => {
            let k = largest_pow2_lt(n);
            node_hash(&tree_root(&lines[..k]), &tree_root(&lines[k..]))
        }
    }
}

/// The largest power of two strictly smaller than `n` (`n > 1`).
fn largest_pow2_lt(n: usize) -> usize {
    let mut k = 1usize;
    while k << 1 < n {
        k <<= 1;
    }
    k
}

/// The inclusion proof for `index` (RFC 6962 §2.1.1), leaf-to-root: the
/// sibling hash of each level, innermost first.
pub fn inclusion_proof<T: AsRef<[u8]>>(
    lines: &[T],
    index: usize,
) -> Result<Vec<[u8; 32]>, TebakoError> {
    if lines.is_empty() {
        return Err(TebakoError::new(
            "an empty registry has no rows to prove — publish the snapshot first",
            EX_TEBAKO_LEDGER,
        ));
    }
    if index >= lines.len() {
        return Err(TebakoError::new(
            format!(
                "row {index} does not exist: the tree holds {} rows (0-based)",
                lines.len()
            ),
            EX_USAGE,
        ));
    }
    Ok(path(lines, index))
}

fn path<T: AsRef<[u8]>>(lines: &[T], m: usize) -> Vec<[u8; 32]> {
    let n = lines.len();
    if n == 1 {
        return Vec::new();
    }
    let k = largest_pow2_lt(n);
    let mut proof;
    if m < k {
        proof = path(&lines[..k], m);
        proof.push(tree_root(&lines[k..]));
    } else {
        proof = path(&lines[k..], m - k);
        proof.push(tree_root(&lines[..k]));
    }
    proof
}

/// Recompute the root from a line, its index, the tree's leaf count and
/// the proof. Any mismatch is the proof's own failure, named.
///
/// The fold follows RFC 6962 §2.1.2's recursion exactly: the LAST proof
/// element belongs to the OUTERMOST split, so an unbalanced tree's
/// orientation comes from the level being combined, never from the leaf's
/// position at the root. The proof's LENGTH is per-leaf in such a tree
/// (`proof_len` is the expected count), so a short or long proof is caught
/// where it runs out or runs over, never by a global constant.
pub fn verify_inclusion(
    line: &[u8],
    index: usize,
    total: usize,
    proof: &[[u8; 32]],
) -> Result<[u8; 32], TebakoError> {
    if total == 0 || index >= total {
        return Err(TebakoError::new(
            format!("row {index} cannot exist in a tree of {total} rows"),
            EX_USAGE,
        ));
    }
    fold(line, index, total, proof)
}

fn fold(line: &[u8], index: usize, n: usize, proof: &[[u8; 32]]) -> Result<[u8; 32], TebakoError> {
    if n == 1 {
        if !proof.is_empty() {
            return Err(TebakoError::new(
                format!(
                    "the proof carries {} hash(es) more than this row's path — a long proof proves nothing",
                    proof.len()
                ),
                EX_TEBAKO_LEDGER,
            ));
        }
        return Ok(leaf_hash(line));
    }
    let (last, rest) = proof.split_last().ok_or_else(|| {
        TebakoError::new(
            "the proof is short: it ran out before the root (a short proof proves nothing)"
                .to_string(),
            EX_TEBAKO_LEDGER,
        )
    })?;
    let k = largest_pow2_lt(n);
    if index < k {
        Ok(node_hash(&fold(line, index, k, rest)?, last))
    } else {
        Ok(node_hash(last, &fold(line, index - k, n - k, rest)?))
    }
}

/// How many siblings row `index` has in a tree of `n` leaves. Unbalanced
/// trees give different lengths per leaf (n=3: two for leaves 0–1, one for
/// leaf 2), which is exactly why this is a function of the index.
pub fn proof_len(n: usize, index: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    let k = largest_pow2_lt(n);
    if index < k {
        proof_len(k, index) + 1
    } else {
        proof_len(n - k, index - k) + 1
    }
}

// ------------------------------------------------------------------ heads

/// One log head (spec 41 §4): the snapshot's root, its leaf count, and the
/// chain to the head before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub seq: u64,
    pub ts: u64,
    pub root: String,
    pub rows: usize,
    /// The previous head's root, or `-` for `seq 0`.
    pub prev: String,
    /// The registry's declared head keyid, or `-` for an unsigned head.
    pub key: String,
    /// The detached armored signature, hex-encoded, or `-`.
    pub sig: String,
    /// The log file's 1-based line this head came from.
    pub line: usize,
}

impl Head {
    pub fn signed(&self) -> bool {
        self.key != "-" && self.sig != "-"
    }

    /// The canonical signed payload (spec 41 §4) — byte-exact, so a
    /// verifier reconstructs it instead of parsing the head's own line.
    pub fn payload(&self) -> String {
        format!(
            "{HEAD_PAYLOAD_TAG}\n{}\n{}\n{}\n{}\n{}\n",
            self.seq, self.ts, self.root, self.rows, self.prev
        )
    }

    pub fn render(&self) -> String {
        format!(
            "{} ts={} root={} rows={} prev={} key={} sig={}",
            self.seq, self.ts, self.root, self.rows, self.prev, self.key, self.sig
        )
    }
}

pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn from_hex(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for pair in s.as_bytes().chunks(2) {
        let hi = (pair[0] as char).to_digit(16).ok_or(())?;
        let lo = (pair[1] as char).to_digit(16).ok_or(())?;
        out.push((hi * 16 + lo) as u8);
    }
    Ok(out)
}

/// Parse a log file. Every line must parse: a damaged log is a finding,
/// never a line to skip past.
pub fn parse_heads(text: &str) -> Result<Vec<Head>, (usize, String)> {
    let mut heads = Vec::new();
    for (idx, raw) in text.split('\n').enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let line_no = idx + 1;
        let mut tokens = raw.split(' ');
        let seq: u64 = tokens.next().and_then(|t| t.parse().ok()).ok_or_else(|| {
            (
                line_no,
                format!("head {line_no} has no readable seq: {raw}"),
            )
        })?;
        let mut fields: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
        for token in tokens {
            if let Some((k, v)) = token.split_once('=') {
                fields.entry(k).or_insert(v);
            }
        }
        let need = |k: &str| {
            fields
                .get(k)
                .copied()
                .ok_or_else(|| (line_no, format!("head {line_no} is missing '{k}': {raw}")))
        };
        let root = need("root")?;
        if root.len() != 64 || !root.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err((
                line_no,
                format!("head {line_no} carries a malformed root '{root}'"),
            ));
        }
        let prev = need("prev")?;
        if prev.len() != 64 && prev != "-" {
            return Err((
                line_no,
                format!("head {line_no} carries a malformed prev '{prev}'"),
            ));
        }
        heads.push(Head {
            seq,
            ts: need("ts")?.parse().map_err(|_| {
                (
                    line_no,
                    format!("head {line_no} has an unreadable ts: {raw}"),
                )
            })?,
            root: root.to_ascii_lowercase(),
            rows: need("rows")?.parse().map_err(|_| {
                (
                    line_no,
                    format!("head {line_no} has an unreadable rows count: {raw}"),
                )
            })?,
            prev: prev.to_ascii_lowercase(),
            key: need("key")?.to_string(),
            sig: need("sig")?.to_string(),
            line: line_no,
        });
    }
    Ok(heads)
}

/// What a chain verification found.
#[derive(Debug, Clone, Default)]
pub struct ChainReport {
    /// The head count the log holds.
    pub heads: usize,
    /// Every problem found, in log order. Empty = the chain is sound.
    pub problems: Vec<String>,
    /// A row-count decrease that no tombstone explains (the caller with
    /// the registry in hand decides; the chain alone can only report it).
    pub row_drops: Vec<String>,
    /// The signature verdict of the newest head.
    pub signature: &'static str,
}

impl ChainReport {
    pub fn sound(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Walk the chain: `seq` continuity, the `prev` link, and the newest
/// head's signature. Row monotonicity is REPORTED separately
/// (`row_drops`) because only the registry can say whether a tombstone
/// accounts for it — a chain that silently accepted a drop would be the
/// mass-yank alarm switched off.
pub fn verify_chain(heads: &[Head], keyring: &[u8]) -> ChainReport {
    let mut report = ChainReport {
        heads: heads.len(),
        // A head is signed only when both fields are present; the
        // verdict is upgraded below when a signature verifies.
        signature: if heads.last().map(Head::signed).unwrap_or(false) {
            "unknown-key"
        } else {
            "unsigned"
        },
        ..ChainReport::default()
    };
    if heads.is_empty() {
        report
            .problems
            .push("the log holds no heads — nothing to verify".to_string());
        return report;
    }

    for (i, head) in heads.iter().enumerate() {
        if i == 0 {
            if head.prev != "-" {
                report.problems.push(format!(
                    "head {} starts the log but names prev={} — the log was truncated",
                    head.seq, head.prev
                ));
            }
        } else {
            let before = &heads[i - 1];
            if head.seq != before.seq + 1 {
                report.problems.push(format!(
                    "head {} follows head {} — the seq must be contiguous (a gap is an edited log)",
                    head.seq, before.seq
                ));
            }
            if head.prev != before.root {
                report.problems.push(format!(
                    "FORK: head {} names prev={} but head {}'s root is {} — the log diverged",
                    head.seq, head.prev, before.seq, before.root
                ));
            }
        }
        if let Some(before) = i.checked_sub(1).map(|j| &heads[j]) {
            if head.rows < before.rows {
                report.row_drops.push(format!(
                    "head {} holds {} rows, down from {} at head {} — legal only with a tombstone (status: withdrawn)",
                    head.seq, head.rows, before.rows, before.seq
                ));
            }
        }
        if let Ok(sig) = from_hex(&head.sig) {
            match tebako_signer::verify_detached_full(keyring, head.payload().as_bytes(), &sig) {
                Ok(tebako_signer::VerifyOutcome::Invalid(_)) => report.problems.push(format!(
                    "head {}: the SIGNATURE DOES NOT VALIDATE — the head was re-written",
                    head.seq
                )),
                Ok(tebako_signer::VerifyOutcome::Trusted(_)) if i + 1 == heads.len() => {
                    report.signature = "valid"
                }
                Ok(tebako_signer::VerifyOutcome::Untrusted(_)) if i + 1 == heads.len() => {
                    report.signature = "unknown-key"
                }
                _ => {}
            }
        } else if head.signed() {
            report
                .problems
                .push(format!("head {}: the signature is not hex", head.seq));
        }
    }
    report
}

// ------------------------------------------------------- canonical rows

/// One registry row, rendered canonically (spec 41 §2): the sort key and
/// the line are the same string, so the order is a pure function of the
/// row set — an author may reindent, reorder or re-comment the YAML and
/// the snapshot does not move.
///
/// ```text
/// <kind> <name>@<version> <platform> <sha256|-> <state> <keyid|->
/// ```
pub fn canonical_lines(
    registry: &tebako_resolve::registry::Registry,
) -> Result<Vec<String>, TebakoError> {
    let mut rows: Vec<String> = Vec::new();
    for payload in &registry.payloads {
        let kind = kind_str(&payload.kind);
        for version in &payload.versions {
            let state = version
                .status
                .clone()
                .unwrap_or_else(|| "active".to_string());
            let keyid = version
                .signature
                .as_ref()
                .map(|s| s.keyid.clone())
                .unwrap_or_else(|| "-".to_string());
            let head = format!("{kind} {}@{} ", payload.name, version.version);
            match (&version.platforms, &version.variants) {
                (Some(platforms), None) => match platforms {
                    tebako_resolve::registry::RegistryPlatforms::Universal => {
                        rows.push(format!("{head}universal {} {state} {keyid}", "-"));
                    }
                    tebako_resolve::registry::RegistryPlatforms::PerTriplet(map) => {
                        for (triplet, artifact) in map {
                            rows.push(format!(
                                "{head}{triplet} {} {state} {keyid}",
                                artifact.sha256
                            ));
                        }
                    }
                },
                (None, Some(variants)) => {
                    for arm in variants {
                        let id = tebako_resolve::registry::variant_id(
                            arm.runtime_requirement.as_ref(),
                        )
                        .map_err(|e| {
                            TebakoError::new(
                                format!(
                                    "the variant arm of {} {} has no derivable id: {e}",
                                    payload.name, version.version
                                ),
                                EX_TEBAKO_LEDGER,
                            )
                        })?;
                        match &arm.platforms {
                            // The arm id never stands alone: `<id>:<universal
                            // |triplet>` keeps one reading of the platform
                            // column across both forms, so a universal arm is
                            // never mistaken for a triplet.
                            tebako_resolve::registry::RegistryPlatforms::Universal => {
                                rows.push(format!("{head}{id}:universal {} {state} {keyid}", "-"));
                            }
                            tebako_resolve::registry::RegistryPlatforms::PerTriplet(map) => {
                                for (triplet, artifact) in map {
                                    rows.push(format!(
                                        "{head}{id}:{triplet} {} {state} {keyid}",
                                        artifact.sha256
                                    ));
                                }
                            }
                        }
                    }
                }
                // The registry validator refuses both/neither (spec 28 §10);
                // reaching here means a document bypassed validation.
                _ => {
                    return Err(TebakoError::new(
                        format!(
                            "{} {} carries neither exactly one of platforms:/variants: — the registry is not validated",
                            payload.name, version.version
                        ),
                        EX_TEBAKO_LEDGER,
                    ))
                }
            }
        }
    }
    rows.sort();
    Ok(rows)
}

fn kind_str(kind: &tpkg::PayloadKind) -> &'static str {
    match kind {
        tpkg::PayloadKind::App => "app",
        tpkg::PayloadKind::Data => "data",
        tpkg::PayloadKind::Toolkit => "toolkit",
        tpkg::PayloadKind::Runtime => "runtime",
        tpkg::PayloadKind::Language => "language",
    }
}

// ------------------------------------------------- registry vs. the log

/// What `registry log verify --registry <path>` reports (spec 41 §5): the
/// registry in hand either IS the row set the newest head sealed, or the
/// difference is named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryCheck {
    /// Rows the registry document holds.
    pub registry_rows: usize,
    /// Rows the newest head was sealed over.
    pub sealed_rows: usize,
    /// The recomputed root equals the newest head's root.
    pub root_match: bool,
    /// One sentence a human can act on.
    pub detail: String,
}

/// Compare a registry document against a log. Recomputing the tree from
/// the document's canonical rows and comparing the ROOT is both the
/// membership check and the completeness check in one number: if the roots
/// agree, the row set is exactly what was sealed, and no row could have
/// been added or removed without moving it.
pub fn verify_registry_against_log(
    registry: &tebako_resolve::registry::Registry,
    heads: &[Head],
) -> Result<RegistryCheck, TebakoError> {
    let lines = canonical_lines(registry)?;
    let registry_rows = lines.len();
    let Some(newest) = heads.last() else {
        return Err(TebakoError::new(
            "the log holds no heads — publish a snapshot before verifying a registry against it"
                .to_string(),
            EX_TEBAKO_LEDGER,
        ));
    };
    let root = hex(&tree_root(&lines));
    let root_match = root == newest.root;
    let detail = if root_match && registry_rows == newest.rows {
        format!(
            "the registry IS head {}: {registry_rows} rows, root {root}",
            newest.seq
        )
    } else if root_match {
        // Same tree, different declared count: the head's own number is
        // wrong, which is a producer bug rather than a yank — say which.
        format!(
            "the roots agree but head {} declares {} rows while the registry holds {registry_rows} — the head's count is wrong",
            newest.seq, newest.rows
        )
    } else {
        format!(
            "the registry is NOT what head {} sealed: it holds {registry_rows} rows (root {root}) but the head was sealed over {} rows (root {}) — a row was added, removed or edited after the seal",
            newest.seq, newest.rows, newest.root
        )
    };
    Ok(RegistryCheck {
        registry_rows,
        sealed_rows: newest.rows,
        root_match,
        detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(n: usize) -> Vec<Vec<u8>> {
        (0..n)
            .map(|i| format!("app app@{i}.0 universal {i:064x} active -").into_bytes())
            .collect()
    }

    #[test]
    fn the_empty_tree_has_its_defined_root() {
        let none: [Vec<u8>; 0] = [];
        assert_eq!(tree_root(&none), {
            let d: [u8; 32] = Sha256::digest([]).into();
            d
        });
    }

    #[test]
    fn a_single_leaf_is_its_own_root_and_needs_no_proof() {
        let l = lines(1);
        assert_eq!(tree_root(&l), leaf_hash(&l[0]));
        let proof = inclusion_proof(&l, 0).unwrap();
        assert!(proof.is_empty());
        assert_eq!(
            verify_inclusion(&l[0], 0, 1, &proof).unwrap(),
            tree_root(&l)
        );
    }

    #[test]
    fn every_row_of_every_tree_to_sixteen_leaves_proves_against_its_root() {
        for n in 1..=16usize {
            let l = lines(n);
            let root = tree_root(&l);
            for i in 0..n {
                let proof = inclusion_proof(&l, i).unwrap();
                let got = verify_inclusion(&l[i], i, n, &proof).unwrap();
                assert_eq!(got, root, "n={n} i={i}: proof does not rebuild the root");
            }
        }
    }

    #[test]
    fn a_wrong_line_index_or_total_fails() {
        let l = lines(5);
        let proof = inclusion_proof(&l, 2).unwrap();
        let root = tree_root(&l);
        // Another row's bytes do not verify at this index.
        assert_ne!(
            verify_inclusion(&l[3], 2, 5, &proof).unwrap_or([0; 32]),
            root
        );
        // The same proof at another index does not rebuild the root.
        assert_ne!(
            verify_inclusion(&l[2], 3, 5, &proof).unwrap_or([0; 32]),
            root
        );
        // A short proof is named, not silently accepted.
        let short = &proof[..proof.len() - 1];
        let err = verify_inclusion(&l[2], 2, 5, short).unwrap_err();
        assert!(err.message.contains("short"), "{}", err.message);
    }

    #[test]
    fn proofs_are_log_depth_and_per_leaf_in_unbalanced_trees() {
        for n in [1usize, 2, 3, 5, 8, 9, 16, 17] {
            let l = lines(n);
            for i in 0..n {
                let proof = inclusion_proof(&l, i).unwrap();
                assert_eq!(proof.len(), proof_len(n, i), "n={n} i={i}");
                assert!(proof.len() <= 5, "n={n} i={i}: {} levels", proof.len());
            }
        }
        // The unbalanced case that broke a global-depth assumption.
        assert_eq!(proof_len(3, 0), 2);
        assert_eq!(proof_len(3, 2), 1);
    }

    fn head(seq: u64, root: &str, rows: usize, prev: &str) -> Head {
        Head {
            seq,
            ts: 1_761_000_000 + seq,
            root: root.to_string(),
            rows,
            prev: prev.to_string(),
            key: "-".to_string(),
            sig: "-".to_string(),
            line: (seq + 1) as usize,
        }
    }

    #[test]
    fn a_sound_chain_verifies_and_a_fork_is_named() {
        let r0 = hex(&tree_root(&lines(1)));
        let r1 = hex(&tree_root(&lines(2)));
        let chain = vec![head(0, &r0, 1, "-"), head(1, &r1, 2, &r0)];
        let report = verify_chain(&chain, b"");
        assert!(report.sound(), "{:?}", report.problems);
        assert!(report.row_drops.is_empty());

        // A fork: the second head names a prev that is not the first root.
        let forked = vec![head(0, &r0, 1, "-"), head(1, &r1, 2, &hex(&[9u8; 32]))];
        let report = verify_chain(&forked, b"");
        assert!(!report.sound());
        assert!(report.problems[0].contains("FORK"), "{:?}", report.problems);
    }

    #[test]
    fn a_seq_gap_is_named() {
        let r0 = hex(&tree_root(&lines(1)));
        let chain = vec![head(0, &r0, 1, "-"), head(2, &r0, 1, &r0)];
        let report = verify_chain(&chain, b"");
        assert!(report.problems.iter().any(|p| p.contains("contiguous")));
    }

    #[test]
    fn a_row_drop_is_reported_not_accepted() {
        let r = hex(&tree_root(&lines(2)));
        let chain = vec![head(0, &r, 2, "-"), head(1, &r, 1, &r)];
        let report = verify_chain(&chain, b"");
        assert!(report.sound(), "the chain itself is sound");
        assert_eq!(report.row_drops.len(), 1);
        assert!(report.row_drops[0].contains("tombstone"));
    }

    fn registry(yaml: &str) -> tebako_resolve::registry::Registry {
        tebako_resolve::registry::Registry::from_yaml(yaml)
            .unwrap_or_else(|e| panic!("fixture must validate: {e}"))
    }

    #[test]
    fn canonical_rows_are_a_pure_function_of_the_row_set() {
        let yaml = "\
schema_version: 1
payloads:
  - name: hello
    kind: app
    versions:
      - version: 1.0.0
        platforms: universal
        release: {ref: \"tfs:github:acme/hello:1.0.0\"}
        entrypoints: [hello]
    default: 1.0.0
";
        let rows = canonical_lines(&registry(yaml)).unwrap();
        // A universal row carries no digest of its own: the release's
        // SHA256SUMS anchors the bytes, this log anchors the row set.
        assert_eq!(rows, vec!["app hello@1.0.0 universal - active -"]);
    }

    #[test]
    fn per_triplet_rows_render_their_triplet_and_digest_and_sort() {
        let digest = "a".repeat(64);
        let yaml = format!(
            "\
schema_version: 1
payloads:
  - name: hello
    kind: app
    versions:
      - version: 1.0.0
        platforms:
          x86_64-linux-gnu: {{artifact: hello.tfs, sha256: \"{digest}\"}}
          aarch64-macos: {{artifact: hello.tfs, sha256: \"{digest}\"}}
        release: {{ref: \"tfs:github:acme/hello:1.0.0\"}}
        entrypoints: [hello]
    default: 1.0.0
"
        );
        let rows = canonical_lines(&registry(&yaml)).unwrap();
        assert_eq!(
            rows,
            vec![
                format!("app hello@1.0.0 aarch64-macos {digest} active -"),
                format!("app hello@1.0.0 x86_64-linux-gnu {digest} active -"),
            ]
        );
    }

    #[test]
    fn a_withdrawn_row_keeps_its_place_with_the_status_verbatim() {
        let yaml = "\
schema_version: 1
payloads:
  - name: hello
    kind: app
    versions:
      - version: 1.0.0
        platforms: universal
        release: {ref: \"tfs:github:acme/hello:1.0.0\"}
        entrypoints: [hello]
        status: withdrawn
    default: 1.0.0
";
        let rows = canonical_lines(&registry(yaml)).unwrap();
        assert_eq!(rows, vec!["app hello@1.0.0 universal - withdrawn -"]);
    }

    #[test]
    fn a_variant_arm_renders_its_derived_id_and_triplet() {
        let digest = "b".repeat(64);
        // The id derives from implementation + ABI line (spec 28 §2): a
        // pure-language requirement does not fork a variant at all, which
        // is why the fixture carries two implementations.
        let yaml = format!(
            "\
schema_version: 1
payloads:
  - name: hello
    kind: app
    default_variant: java-temurin-21.0
    versions:
      - version: 1.0.0
        variants:
          - runtime_requirement: {{engine: java, implementation: temurin, constraint: \"~> 21.0\"}}
            platforms: universal
          - runtime_requirement: {{engine: java, implementation: graalvm, constraint: \"~> 24.0\"}}
            platforms:
              x86_64-linux-gnu: {{artifact: hello.tfs, sha256: \"{digest}\"}}
        release: {{ref: \"tfs:github:acme/hello:1.0.0\"}}
        entrypoints: [hello]
    default: 1.0.0
"
        );
        let rows = canonical_lines(&registry(&yaml)).unwrap();
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert!(
            rows.iter()
                .any(|r| r.contains("java-graalvm-24.0:x86_64-linux-gnu ")),
            "{rows:?}"
        );
        assert!(
            rows.iter()
                .any(|r| r.contains("java-temurin-21.0:universal - active -")),
            "{rows:?}"
        );
        // A snapshot that collapsed the two arms would hide one build.
        assert_ne!(rows[0].split(' ').nth(2), rows[1].split(' ').nth(2));
    }

    fn head_over(registry: &tebako_resolve::registry::Registry, seq: u64) -> Head {
        let lines = canonical_lines(registry).unwrap();
        Head {
            seq,
            ts: 1_761_000_000,
            root: hex(&tree_root(&lines)),
            rows: lines.len(),
            prev: if seq == 0 {
                "-".to_string()
            } else {
                "0".repeat(64)
            },
            key: "-".to_string(),
            sig: "-".to_string(),
            line: 1,
        }
    }

    #[test]
    fn a_registry_verifies_against_the_head_that_sealed_it() {
        let yaml = "\
schema_version: 1
payloads:
  - name: hello
    kind: app
    versions:
      - version: 1.0.0
        platforms: universal
        release: {ref: \"tfs:github:acme/hello:1.0.0\"}
        entrypoints: [hello]
    default: 1.0.0
";
        let reg = registry(yaml);
        let head = head_over(&reg, 0);
        let check = verify_registry_against_log(&reg, std::slice::from_ref(&head)).unwrap();
        assert!(check.root_match);
        assert_eq!(check.registry_rows, 1);
        assert!(check.detail.contains("IS head 0"), "{}", check.detail);
    }

    #[test]
    fn a_yank_after_the_seal_is_named_not_shrugged_off() {
        let one = "\
schema_version: 1
payloads:
  - name: hello
    kind: app
    versions:
      - version: 1.0.0
        platforms: universal
        release: {ref: \"tfs:github:acme/hello:1.0.0\"}
        entrypoints: [hello]
    default: 1.0.0
";
        let two = "\
schema_version: 1
payloads:
  - name: hello
    kind: app
    versions:
      - version: 1.0.0
        platforms: universal
        release: {ref: \"tfs:github:acme/hello:1.0.0\"}
        entrypoints: [hello]
      - version: 2.0.0
        platforms: universal
        release: {ref: \"tfs:github:acme/hello:2.0.0\"}
        entrypoints: [hello]
    default: 2.0.0
";
        // The head sealed the TWO-row registry; the file in hand has one.
        let head = head_over(&registry(two), 4);
        let check =
            verify_registry_against_log(&registry(one), std::slice::from_ref(&head)).unwrap();
        assert!(!check.root_match);
        assert_eq!(check.registry_rows, 1);
        assert_eq!(check.sealed_rows, 2);
        assert!(check.detail.contains("NOT what head 4 sealed"));
    }

    #[test]
    fn heads_round_trip_through_their_rendered_line() {
        let r = "a".repeat(64);
        let h = head(3, &r, 9, &"b".repeat(64));
        let text = format!("{}\n", h.render());
        let parsed = parse_heads(&text).expect("parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].seq, 3);
        assert_eq!(parsed[0].rows, 9);
        assert_eq!(parsed[0].payload(), h.payload());
        assert!(!parsed[0].signed());
    }

    #[test]
    fn a_damaged_head_line_is_a_finding() {
        assert!(parse_heads("0 ts=1 root=zz rows=1 prev=- key=- sig=-\n")
            .unwrap_err()
            .1
            .contains("malformed root"));
        assert!(parse_heads("nonsense\n").unwrap_err().1.contains("seq"));
    }
}
