//! `tebako audit` — the audit ledger's reader and its anchor (spec 40).
//!
//! The ledger itself is written by everyone else: the bootstrap records the
//! loader's fetch/verify/mount/exec decisions, `tfs::journal` records the
//! driver's and preload's policy/VFS/overlay/decrypt decisions, the shim
//! records dispatch. This module reads that one file, answers questions
//! about it, and anchors it:
//!
//! - **queries** (`--since`, `--until`, `--event`, `--artifact`, `--tail`)
//!   are read-only end to end — no store stamp, no network-config install,
//!   no journal write (spec 40 §1's read-only law: an auditor runs this on
//!   a host whose store may predate the binary in their hand);
//! - **`--seal`** appends one chained-digest record to `<journal>.seal`,
//!   signed with the press-local key unless the run opted out;
//! - **`--verify`** recomputes the chain and compares it to the last seal,
//!   naming the record interval that no longer matches.
//!
//! Damage is surfaced, never swallowed (spec 40 §2.2): an unparseable line
//! is reported with its line number and raw bytes, and `--strict` turns it
//! into exit 80.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::TebakoError;

const EX_USAGE: i32 = 64;
const EX_TEBAKO_IO: i32 = 74;
/// A ledger finding (spec 40 §6). The constant's owner is `tpkg`.
pub const EX_TEBAKO_LEDGER: i32 = tpkg::EX_TEBAKO_LEDGER;

/// The chain's domain separator (spec 40 §4.1).
const CHAIN_SEED: &str = "tebako-ledger-v1\n";

/// The seal payload's first line (spec 40 §4.2).
const SEAL_PAYLOAD_TAG: &str = "tebako-ledger-seal-v1";

/// The JSON document's version key and value (spec 40 §5).
const SCHEMA_KEY: &str = "audit_schema";
const SCHEMA_VERSION: u64 = 1;

const USAGE: &str = "usage: tebako audit [--since <t>] [--until <t>] [--event <e,…>] [--artifact <needle>] [--tail <n>] [--count] [--json] [--strict] [--seal] [--verify]";

// ---------------------------------------------------------------- records

/// One parsed ledger record. `raw` keeps the exact bytes (newline
/// excluded) so line-mode output is the ledger's own text, not a
/// re-rendering of it — the property that lets a reader pipe an excerpt
/// to a digest and compare it with a seal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub line: usize,
    pub ts: u64,
    pub event: String,
    pub fields: BTreeMap<String, String>,
    pub raw: String,
}

/// A line the grammar cannot read, kept with its position so the report
/// can point at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    pub line: usize,
    pub raw: String,
}

/// The whole ledger: the raw lines (the chain's input), the records they
/// parse into, and the damage among them.
///
/// `lines` holds BYTES, not strings: the chain must hash exactly what the
/// file holds (spec 40 §4.1's "exact bytes" is the whole anchor — hashing
/// a lossy UTF-8 rendering would let two different non-UTF-8 paths collide
/// to the same `U+FFFD` and change without detection). The parsed records
/// carry a lossy string for display and filtering, which is all a reader
/// needs.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    pub lines: Vec<Vec<u8>>,
    pub records: Vec<Record>,
    pub malformed: Vec<Malformed>,
}

impl Ledger {
    /// Parse a file's bytes. A final line without a terminating newline is
    /// still a record: the bytes are the record.
    pub fn parse(text: &[u8]) -> Ledger {
        let mut lines: Vec<Vec<u8>> = text.split(|b| *b == b'\n').map(<[u8]>::to_vec).collect();
        // A file ending with its newline yields one trailing empty piece:
        // that is the terminator, not a record. Interior blank lines stay
        // — they are damage the auditor must see.
        if lines.last().map(Vec::is_empty).unwrap_or(false) {
            lines.pop();
        }
        let mut records = Vec::new();
        let mut malformed = Vec::new();
        for (idx, raw) in lines.iter().enumerate() {
            match parse_record(idx + 1, &String::from_utf8_lossy(raw)) {
                Ok(r) => records.push(r),
                Err(bad) => malformed.push(bad),
            }
        }
        Ledger {
            lines,
            records,
            malformed,
        }
    }

    /// Read and parse a journal file. A missing file is a named error
    /// carrying the path and the remedy — never an empty ledger, which
    /// would make "no records" and "no file" indistinguishable.
    pub fn read(path: &Path) -> Result<Ledger, TebakoError> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(Ledger::parse(&bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(TebakoError::new(
                format!(
                    "no audit journal at {} — this host has run no tebako loads yet (set TEBAKO_HOME if the store lives elsewhere)",
                    path.display()
                ),
                EX_TEBAKO_IO,
            )),
            Err(e) => Err(TebakoError::new(
                format!("cannot read the audit journal {}: {e}", path.display()),
                EX_TEBAKO_IO,
            )),
        }
    }
}

/// Parse one line into a record (spec 40 §2.1). A token without `=` is a
/// continuation of the previous value, so a path containing spaces stays
/// one value instead of turning a legitimate record into damage.
pub fn parse_record(line: usize, raw: &str) -> Result<Record, Malformed> {
    let bad = || Malformed {
        line,
        raw: raw.to_string(),
    };
    if raw.trim().is_empty() {
        return Err(bad());
    }
    let mut tokens = raw.split(' ');
    let ts: u64 = tokens.next().ok_or_else(bad)?.parse().map_err(|_| bad())?;
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut last: Option<String> = None;
    for token in tokens {
        if token.is_empty() {
            continue;
        }
        match token.split_once('=') {
            Some((k, v)) => {
                // First wins (spec 40 §5's fields_map_rule): the writer's
                // first statement stands.
                fields.entry(k.to_string()).or_insert_with(|| v.to_string());
                last = Some(k.to_string());
            }
            None => match last.clone() {
                Some(k) => {
                    let joined = format!("{} {token}", fields.get(&k).cloned().unwrap_or_default());
                    fields.insert(k, joined);
                }
                None => return Err(bad()),
            },
        }
    }
    let event = fields.get("event").cloned().ok_or_else(bad)?;
    Ok(Record {
        line,
        ts,
        event,
        fields,
        raw: raw.to_string(),
    })
}

// ------------------------------------------------------------------ chain

/// The chain's seed digest (spec 40 §4.1).
pub fn chain_seed() -> [u8; 32] {
    Sha256::digest(CHAIN_SEED.as_bytes()).into()
}

/// The chain over `lines`, as hex. Each link hashes the previous RAW
/// 32-byte digest with the line's EXACT bytes plus its terminator, so the
/// terminator is normalized (a file whose final newline an editor added or
/// stripped is the same ledger) and nothing else is: a byte changed
/// anywhere shifts every following digest.
pub fn chain_head<'a, I>(lines: I) -> String
where
    I: IntoIterator<Item = &'a Vec<u8>>,
{
    let mut head = chain_seed();
    for line in lines {
        let mut d = Sha256::new();
        d.update(head);
        d.update(line);
        d.update(b"\n");
        head = d.finalize().into();
    }
    hex(&head)
}

/// Lowercase hex — the one rendering every digest in this document uses.
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

/// The chain over a prefix of the ledger.
fn chain_prefix(ledger: &Ledger, count: usize) -> String {
    chain_head(ledger.lines.iter().take(count))
}

// ------------------------------------------------------------------ seals

/// One seal record (spec 40 §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seal {
    pub ts: u64,
    pub head: String,
    pub lines: usize,
    /// The signer's keyid in hex, or `-` for an unsigned seal.
    pub key: String,
    /// The detached armored signature, hex-encoded, or `-`.
    pub sig: String,
    /// The seal file's 1-based line this record came from.
    pub line: usize,
}

impl Seal {
    pub fn signed(&self) -> bool {
        self.key != "-" && self.sig != "-"
    }

    /// The canonical signed payload (spec 40 §4.2) — byte-exact, so a
    /// verifier reconstructs it rather than parsing the seal's own line.
    pub fn payload(&self) -> String {
        format!(
            "{SEAL_PAYLOAD_TAG}\n{}\n{}\n{}\n",
            self.head, self.lines, self.ts
        )
    }

    /// Render the line this seal is stored as.
    pub fn render(&self) -> String {
        format!(
            "{} head={} lines={} key={} sig={}",
            self.ts, self.head, self.lines, self.key, self.sig
        )
    }
}

/// Parse the seal file. Every line must parse: a seal file with damage is
/// a ledger finding, not something to skip past (corrupting the anchor is
/// an attacker's first move, so damage there is never noise).
pub fn parse_seals(text: &str) -> Result<Vec<Seal>, (usize, String)> {
    let mut seals = Vec::new();
    for (idx, raw) in text.split('\n').enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let line_no = idx + 1;
        let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
        let mut tokens = raw.split(' ');
        let ts: u64 = tokens.next().and_then(|t| t.parse().ok()).ok_or_else(|| {
            (
                line_no,
                format!("seal line {line_no} has no readable timestamp: {raw}"),
            )
        })?;
        for token in tokens {
            if let Some((k, v)) = token.split_once('=') {
                fields.entry(k).or_insert(v);
            }
        }
        let need = |k: &str| {
            fields.get(k).copied().ok_or_else(|| {
                (
                    line_no,
                    format!("seal line {line_no} is missing '{k}': {raw}"),
                )
            })
        };
        let head = need("head")?;
        if head.len() != 64 || !head.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err((
                line_no,
                format!("seal line {line_no} carries a malformed head '{head}'"),
            ));
        }
        let lines: usize = need("lines")?.parse().map_err(|_| {
            (
                line_no,
                format!("seal line {line_no} has an unreadable 'lines' count: {raw}"),
            )
        })?;
        seals.push(Seal {
            ts,
            head: head.to_ascii_lowercase(),
            lines,
            key: need("key")?.to_string(),
            sig: need("sig")?.to_string(),
            line: line_no,
        });
    }
    Ok(seals)
}

// ----------------------------------------------------------- verification

/// The three outcomes of spec 40 §4.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Sealed,
    Tampered,
    Unsealed,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Sealed => "sealed",
            Outcome::Tampered => "tampered",
            Outcome::Unsealed => "unsealed",
        }
    }
}

/// What a verification found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub outcome: Outcome,
    pub seal_ts: u64,
    pub sealed_lines: usize,
    pub head: String,
    /// The unsealed tail's record count.
    pub tail: usize,
    /// The record interval no longer covered by its seal, 1-based and
    /// inclusive. `None` when the outcome is `Sealed`.
    pub divergence: Option<(usize, usize)>,
    /// The seal's signer keyid (hex) when signed.
    pub signer: Option<String>,
    /// `valid` | `unknown-key` | `invalid` | `unsigned`.
    pub signature: &'static str,
    /// One sentence a human can act on.
    pub detail: String,
}

/// Recompute the chain and compare it to the seals. The LAST seal speaks
/// for the file; earlier seals locate the damage: the newest seal that
/// still matches marks the end of the known-good prefix, and the first
/// seal after it names the interval that changed.
pub fn verify(ledger: &Ledger, seals: &[Seal], keyring: &[u8]) -> Verification {
    let total = ledger.lines.len();
    let Some(last) = seals.last() else {
        return Verification {
            outcome: Outcome::Unsealed,
            seal_ts: 0,
            sealed_lines: 0,
            head: String::new(),
            tail: total,
            divergence: None,
            signer: None,
            signature: "unsigned",
            detail: format!(
                "no seal covers this journal yet — {total} record(s), none anchored; run 'tebako audit --seal'"
            ),
        };
    };

    let mut known_good = 0usize;
    let mut first_failure: Option<&Seal> = None;
    for seal in seals {
        let matches = seal.lines <= total && chain_prefix(ledger, seal.lines) == seal.head;
        if matches {
            known_good = known_good.max(seal.lines);
        } else if first_failure.is_none() {
            first_failure = Some(seal);
        }
    }

    let signature = check_signature(last, keyring);
    let signer = last.signed().then(|| last.key.clone());

    match first_failure {
        None => Verification {
            outcome: Outcome::Sealed,
            seal_ts: last.ts,
            sealed_lines: last.lines,
            head: last.head.clone(),
            tail: total - last.lines,
            divergence: None,
            signer,
            signature,
            detail: if signature == "invalid" {
                format!(
                    "the chain matches the seal of {} over {} record(s), but the seal's SIGNATURE DOES NOT VALIDATE — the seal was re-written",
                    last.ts, last.lines
                )
            } else if signature == "unknown-key" {
                format!(
                    "the chain matches the seal of {} over {} record(s); the signer key {} is not in the trusted keyring — authenticity unproven",
                    last.ts, last.lines, last.key
                )
            } else if last.signed() {
                format!(
                    "the chain matches the signed seal of {} over {} record(s) (signer {})",
                    last.ts, last.lines, last.key
                )
            } else {
                format!(
                    "the chain matches the UNSIGNED seal of {} over {} record(s) — edits are detected, forgery is not",
                    last.ts, last.lines
                )
            },
        },
        Some(failing) => {
            let removed = failing.lines > total;
            let from = if removed { total + 1 } else { known_good + 1 };
            let to = failing.lines;
            let detail = if removed {
                format!(
                    "records {from}–{to} are GONE: the seal of {} covers {} record(s) but the journal holds only {total}",
                    failing.ts, failing.lines
                )
            } else if known_good > 0 {
                format!(
                    "records {from}–{to} were altered: the seal of {} covers {} record(s) and no longer matches; the newest earlier seal still matching covers through record {known_good}, so the change is inside this interval",
                    failing.ts, failing.lines
                )
            } else {
                format!(
                    "records {from}–{to} were altered: the seal of {} covers {} record(s) and no longer matches (no earlier seal narrows it further)",
                    failing.ts, failing.lines
                )
            };
            Verification {
                outcome: Outcome::Tampered,
                seal_ts: failing.ts,
                sealed_lines: failing.lines,
                head: failing.head.clone(),
                tail: total.saturating_sub(failing.lines),
                divergence: Some((from, to)),
                signer,
                signature,
                detail,
            }
        }
    }
}

/// Classify the last seal's signature. A signed seal whose signature does
/// not validate is a tamper finding, not a warning (spec 40 §4.3).
fn check_signature(seal: &Seal, keyring: &[u8]) -> &'static str {
    if !seal.signed() {
        return "unsigned";
    }
    let Ok(sig) = from_hex(&seal.sig) else {
        return "invalid";
    };
    match tebako_signer::verify_detached_full(keyring, seal.payload().as_bytes(), &sig) {
        Ok(tebako_signer::VerifyOutcome::Trusted(_)) => "valid",
        Ok(tebako_signer::VerifyOutcome::Untrusted(_)) => "unknown-key",
        Ok(tebako_signer::VerifyOutcome::Invalid(_)) => "invalid",
        Err(_) => "invalid",
    }
}

// --------------------------------------------------------------- filters

/// The query filters (spec 40 §3.1).
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub since: Option<u64>,
    pub until: Option<u64>,
    pub events: BTreeSet<String>,
    pub artifact: Option<String>,
    pub tail: Option<usize>,
}

impl Query {
    /// Any filter set? `--seal` refuses them: an anchor covers the whole
    /// file or it is not an anchor.
    pub fn any(&self) -> bool {
        self.since.is_some()
            || self.until.is_some()
            || !self.events.is_empty()
            || self.artifact.is_some()
            || self.tail.is_some()
    }

    pub fn matches(&self, r: &Record) -> bool {
        if let Some(since) = self.since {
            if r.ts < since {
                return false;
            }
        }
        if let Some(until) = self.until {
            if r.ts > until {
                return false;
            }
        }
        if !self.events.is_empty() && !self.events.contains(&r.event) {
            return false;
        }
        if let Some(needle) = &self.artifact {
            // The body is everything after the timestamp: the artifact
            // reference as the writer recorded it (a name, a version, a
            // sha256, a URL fragment).
            let body = r.raw.split_once(' ').map_or("", |(_, b)| b);
            if !body.contains(needle.as_str()) {
                return false;
            }
        }
        true
    }

    /// Apply the filters in file order, then `--tail`. The order is never
    /// re-sorted: a ledger's order IS evidence.
    pub fn select<'a>(&self, ledger: &'a Ledger) -> Vec<&'a Record> {
        let mut out: Vec<&Record> = ledger.records.iter().filter(|r| self.matches(r)).collect();
        if let Some(n) = self.tail {
            if out.len() > n {
                out = out.split_off(out.len() - n);
            }
        }
        out
    }
}

/// Parse a time filter: unix seconds, or RFC 3339 UTC
/// (`YYYY-MM-DDTHH:MM:SSZ`). Nothing else — a local-time guess would
/// silently shift a window.
pub fn parse_time(raw: &str) -> Result<u64, TebakoError> {
    if let Ok(n) = raw.parse::<u64>() {
        return Ok(n);
    }
    let bad = || {
        TebakoError::new(
            format!(
                "unreadable time '{raw}' — use unix seconds or RFC 3339 UTC (YYYY-MM-DDTHH:MM:SSZ)"
            ),
            EX_USAGE,
        )
    };
    let b = raw.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[19] != b'Z' {
        return Err(bad());
    }
    let num = |from: usize, to: usize| -> Result<i64, TebakoError> {
        raw.get(from..to)
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(bad)
    };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, s) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return Err(bad());
    }
    Ok((days_from_civil(y, mo, d) * 86_400 + h * 3_600 + mi * 60 + s) as u64)
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm —
/// the standard closed form, valid for every proleptic Gregorian date).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

// ------------------------------------------------------------- argv → run

/// Does this invocation ask for the machine document? (main.rs moves the
/// banner to stderr so stdout stays the document alone.)
pub fn wants_json(args: &[String]) -> bool {
    args.iter().any(|a| a == "--json")
}

/// Does this invocation WRITE? Only `--seal` does (spec 40 §1's
/// read-only law); every other form must leave the store untouched, so
/// main.rs skips the store-stamp and network-config preflight.
pub fn writes(args: &[String]) -> bool {
    args.iter().any(|a| a == "--seal")
}

/// The value of a flag, or the named usage error.
fn value_of(args: &[String], i: &mut usize, name: &str) -> Result<String, TebakoError> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| TebakoError::new(format!("{name} needs a value ({USAGE})"), EX_USAGE))
}

/// `tebako audit [filters] [--count] [--json] [--strict] [--seal] [--verify]`
pub fn run(args: &[String]) -> Result<(), TebakoError> {
    let mut query = Query::default();
    let mut count = false;
    let mut json = false;
    let mut strict = false;
    let mut seal_request = false;
    let mut verify_request = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--since" => query.since = Some(parse_time(&value_of(args, &mut i, "--since")?)?),
            "--until" => query.until = Some(parse_time(&value_of(args, &mut i, "--until")?)?),
            "--event" => {
                for name in value_of(args, &mut i, "--event")?.split(',') {
                    let name = name.trim();
                    if name.is_empty() {
                        return Err(TebakoError::new(
                            "--event got an empty event name",
                            EX_USAGE,
                        ));
                    }
                    query.events.insert(name.to_string());
                }
            }
            "--artifact" => query.artifact = Some(value_of(args, &mut i, "--artifact")?),
            "--tail" => {
                let raw = value_of(args, &mut i, "--tail")?;
                query.tail = Some(raw.parse().map_err(|_| {
                    TebakoError::new(format!("--tail needs a number, got '{raw}'"), EX_USAGE)
                })?);
            }
            "--count" => count = true,
            "--json" => json = true,
            "--strict" => strict = true,
            "--seal" => seal_request = true,
            "--verify" => verify_request = true,
            other => {
                return Err(TebakoError::new(
                    format!("unknown audit option '{other}' ({USAGE})"),
                    EX_USAGE,
                ))
            }
        }
        i += 1;
    }

    if seal_request && verify_request {
        return Err(TebakoError::new(
            "--seal writes an anchor and --verify checks one — ask for one at a time",
            EX_USAGE,
        ));
    }

    let journal = tfs::journal::journal_file().ok_or_else(|| {
        TebakoError::new(
            "no tebako home resolves, so there is no audit journal to read (set TEBAKO_HOME or TEBAKO_JAIL_JOURNAL)",
            EX_TEBAKO_IO,
        )
    })?;

    if seal_request {
        return seal(&journal, &query, json);
    }
    if verify_request {
        return verify_report(&journal, json, strict);
    }

    let ledger = Ledger::read(&journal)?;
    if strict && !ledger.malformed.is_empty() {
        return Err(malformed_error(&ledger));
    }
    let selected = query.select(&ledger);

    if json {
        let doc = tebako_json::Value::Object(vec![
            (
                SCHEMA_KEY.to_string(),
                tebako_json::Value::Number(SCHEMA_VERSION.to_string()),
            ),
            (
                "journal".to_string(),
                tebako_json::Value::String(journal.display().to_string()),
            ),
            (
                "matched".to_string(),
                tebako_json::Value::Number(selected.len().to_string()),
            ),
            (
                "total".to_string(),
                tebako_json::Value::Number(ledger.lines.len().to_string()),
            ),
            ("records".to_string(), records_json(&selected)),
            ("malformed".to_string(), malformed_json(&ledger.malformed)),
            ("verify".to_string(), tebako_json::Value::Null),
        ]);
        println!("{}", tebako_json::to_string(&doc));
        return Ok(());
    }

    if count {
        println!("{}", selected.len());
        return Ok(());
    }

    let mut out = String::new();
    for r in &selected {
        out.push_str(&r.raw);
        out.push('\n');
    }
    print!("{out}");
    eprintln!(
        "{} of {} record(s) in {}",
        selected.len(),
        ledger.lines.len(),
        journal.display()
    );
    Ok(())
}

fn malformed_error(ledger: &Ledger) -> TebakoError {
    let first = &ledger.malformed[0];
    TebakoError::new(
        format!(
            "the audit journal holds {} unparseable line(s); the first is line {}: {}",
            ledger.malformed.len(),
            first.line,
            first.raw
        ),
        EX_TEBAKO_LEDGER,
    )
}

fn records_json(records: &[&Record]) -> tebako_json::Value {
    tebako_json::Value::Array(
        records
            .iter()
            .map(|r| {
                let fields = r
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), tebako_json::Value::String(v.clone())))
                    .collect();
                tebako_json::Value::Object(vec![
                    (
                        "line".to_string(),
                        tebako_json::Value::Number(r.line.to_string()),
                    ),
                    (
                        "ts".to_string(),
                        tebako_json::Value::Number(r.ts.to_string()),
                    ),
                    (
                        "event".to_string(),
                        tebako_json::Value::String(r.event.clone()),
                    ),
                    ("fields".to_string(), tebako_json::Value::Object(fields)),
                    ("raw".to_string(), tebako_json::Value::String(r.raw.clone())),
                ])
            })
            .collect(),
    )
}

fn malformed_json(malformed: &[Malformed]) -> tebako_json::Value {
    tebako_json::Value::Array(
        malformed
            .iter()
            .map(|m| {
                tebako_json::Value::Object(vec![
                    (
                        "line".to_string(),
                        tebako_json::Value::Number(m.line.to_string()),
                    ),
                    ("raw".to_string(), tebako_json::Value::String(m.raw.clone())),
                ])
            })
            .collect(),
    )
}

/// `--seal`: anchor the current file state.
fn seal(journal: &Path, query: &Query, json: bool) -> Result<(), TebakoError> {
    if query.any() {
        return Err(TebakoError::new(
            "--seal anchors the WHOLE ledger, so it takes no filters",
            EX_USAGE,
        ));
    }
    let ledger = Ledger::read(journal)?;
    let mut seal = Seal {
        ts: unix_now(),
        head: chain_head(ledger.lines.iter()),
        lines: ledger.lines.len(),
        key: "-".to_string(),
        sig: "-".to_string(),
        line: 0,
    };
    let mut warning = None;
    match seal_signing_key()? {
        Some((home, key)) => {
            // The machine's own seal must VERIFY on the machine that wrote
            // it: register the press-local key into this store's trusted
            // keyring first (the same first-use pin press applies). A
            // keyring that refuses is a loud unsigned-quality warning, not
            // a failed seal — the signature is still produced and a
            // verifier reads exactly what happened.
            match tebako_signer::register_trusted(&home, &key.public_key) {
                Ok(_) => {}
                Err(e) => {
                    warning = Some(format!(
                        "tebako: WARNING: the seal was signed with key {} but that key could not be registered in this store's trusted keyring ({e}) — 'tebako audit --verify' will report the signer as unknown",
                        key.keyid_hex()
                    ));
                }
            }
            let sig = tebako_signer::sign_detached(
                seal.payload().as_bytes(),
                &key.secret_key,
                &key.fingerprint,
            )
            .map_err(|e| {
                TebakoError::new(
                    format!(
                        "cannot sign the ledger seal with key {}: {e} (press 'tebako keys' to inspect the key store)",
                        key.keyid_hex()
                    ),
                    EX_TEBAKO_IO,
                )
            })?;
            seal.key = key.keyid_hex();
            seal.sig = hex(&sig);
        }
        None => warning = Some(unsigned_warning()),
    }

    let seal_path = tfs::journal::seal_file_of(journal);
    if let Some(dir) = seal_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| {
            TebakoError::new(
                format!("cannot create {}: {e}", dir.display()),
                EX_TEBAKO_IO,
            )
        })?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&seal_path)
        .map_err(|e| {
            TebakoError::new(
                format!("cannot open the seal {}: {e}", seal_path.display()),
                EX_TEBAKO_IO,
            )
        })?;
    writeln!(file, "{}", seal.render()).map_err(|e| {
        TebakoError::new(
            format!("cannot write the seal {}: {e}", seal_path.display()),
            EX_TEBAKO_IO,
        )
    })?;
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }

    if json {
        let doc = tebako_json::Value::Object(vec![
            (
                SCHEMA_KEY.to_string(),
                tebako_json::Value::Number(SCHEMA_VERSION.to_string()),
            ),
            (
                "journal".to_string(),
                tebako_json::Value::String(journal.display().to_string()),
            ),
            (
                "seal".to_string(),
                tebako_json::Value::Object(vec![
                    (
                        "path".to_string(),
                        tebako_json::Value::String(seal_path.display().to_string()),
                    ),
                    (
                        "ts".to_string(),
                        tebako_json::Value::Number(seal.ts.to_string()),
                    ),
                    (
                        "head".to_string(),
                        tebako_json::Value::String(seal.head.clone()),
                    ),
                    (
                        "lines".to_string(),
                        tebako_json::Value::Number(seal.lines.to_string()),
                    ),
                    (
                        "signer".to_string(),
                        if seal.signed() {
                            tebako_json::Value::String(seal.key.clone())
                        } else {
                            tebako_json::Value::Null
                        },
                    ),
                ]),
            ),
        ]);
        println!("{}", tebako_json::to_string(&doc));
    } else {
        println!(
            "sealed {} record(s) of {} at {} — head {}{}",
            seal.lines,
            journal.display(),
            seal.ts,
            seal.head,
            if seal.signed() {
                format!(" (signer {})", seal.key)
            } else {
                String::new()
            }
        );
    }
    Ok(())
}

/// The seal's signer, plus the store whose keyring its key belongs to: the
/// press-local key unless the run opted out with `TEBAKO_SIGN=0` (spec 40
/// §4.2). A key that cannot be produced is a loud unsigned seal, never a
/// failed seal: an unanchorable ledger is worse than an unsigned anchor.
fn seal_signing_key() -> Result<Option<(std::path::PathBuf, tebako_signer::PressKey)>, TebakoError>
{
    if let Some(raw) = tpkg::settings::SIGN.env_value() {
        match raw.as_str() {
            "0" | "false" | "no" | "off" => return Ok(None),
            "1" | "true" | "yes" | "on" => {}
            _ => {
                let msg = format!(
                    "TEBAKO_SIGN='{raw}' is not a boolean — set 1/0, true/false, yes/no or on/off"
                );
                return Err(TebakoError::new(msg, EX_USAGE));
            }
        }
    }
    let Some(home) = tfs::journal::tebako_home_dir() else {
        return Ok(None);
    };
    match tebako_signer::press_local_key(&home) {
        Ok(key) => Ok(Some((home, key))),
        Err(_) => Ok(None),
    }
}

fn unsigned_warning() -> String {
    "tebako: WARNING: ledger sealed WITHOUT a signature — this detects edits, not forgery (configure a key with 'tebako keys')".to_string()
}

/// `--verify`: recompute the chain and report against the last seal.
fn verify_report(journal: &Path, json: bool, strict: bool) -> Result<(), TebakoError> {
    let ledger = Ledger::read(journal)?;
    let seal_path = tfs::journal::seal_file_of(journal);
    let seals = match std::fs::read_to_string(&seal_path) {
        Ok(text) => {
            parse_seals(&text).map_err(|(_, msg)| TebakoError::new(msg, EX_TEBAKO_LEDGER))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            return Err(TebakoError::new(
                format!("cannot read the seal {}: {e}", seal_path.display()),
                EX_TEBAKO_IO,
            ))
        }
    };
    let keyring = match tfs::journal::tebako_home_dir() {
        Some(home) => tebako_signer::verification_keyring(&home).unwrap_or_default(),
        None => Vec::new(),
    };
    let v = verify(&ledger, &seals, &keyring);

    if v.outcome == Outcome::Unsealed {
        return Err(TebakoError::new(
            format!("{}: {}", journal.display(), v.detail),
            EX_TEBAKO_IO,
        ));
    }
    if strict && !ledger.malformed.is_empty() {
        return Err(malformed_error(&ledger));
    }

    if json {
        let divergence = match v.divergence {
            Some((from, to)) => tebako_json::Value::Object(vec![
                (
                    "from_line".to_string(),
                    tebako_json::Value::Number(from.to_string()),
                ),
                (
                    "to_line".to_string(),
                    tebako_json::Value::Number(to.to_string()),
                ),
            ]),
            None => tebako_json::Value::Null,
        };
        let doc = tebako_json::Value::Object(vec![
            (
                SCHEMA_KEY.to_string(),
                tebako_json::Value::Number(SCHEMA_VERSION.to_string()),
            ),
            (
                "journal".to_string(),
                tebako_json::Value::String(journal.display().to_string()),
            ),
            (
                "total".to_string(),
                tebako_json::Value::Number(ledger.lines.len().to_string()),
            ),
            (
                "verify".to_string(),
                tebako_json::Value::Object(vec![
                    (
                        "outcome".to_string(),
                        tebako_json::Value::String(v.outcome.as_str().to_string()),
                    ),
                    (
                        "seal_ts".to_string(),
                        tebako_json::Value::Number(v.seal_ts.to_string()),
                    ),
                    (
                        "sealed_lines".to_string(),
                        tebako_json::Value::Number(v.sealed_lines.to_string()),
                    ),
                    (
                        "head".to_string(),
                        tebako_json::Value::String(v.head.clone()),
                    ),
                    (
                        "tail".to_string(),
                        tebako_json::Value::Number(v.tail.to_string()),
                    ),
                    ("divergence".to_string(), divergence),
                    (
                        "signer".to_string(),
                        match &v.signer {
                            Some(k) => tebako_json::Value::String(k.clone()),
                            None => tebako_json::Value::Null,
                        },
                    ),
                    (
                        "signature".to_string(),
                        tebako_json::Value::String(v.signature.to_string()),
                    ),
                    (
                        "detail".to_string(),
                        tebako_json::Value::String(v.detail.clone()),
                    ),
                ]),
            ),
        ]);
        println!("{}", tebako_json::to_string(&doc));
    } else {
        println!("{}: {}", journal.display(), v.outcome.as_str());
        println!("  {}", v.detail);
        println!(
            "  seal {} covers {} record(s); the journal holds {} now; {} unsealed",
            v.seal_ts,
            v.sealed_lines,
            ledger.lines.len(),
            v.tail
        );
        println!("  signature: {}", v.signature);
    }

    if v.outcome == Outcome::Tampered || v.signature == "invalid" {
        return Err(TebakoError::new(
            format!("{}: {}", journal.display(), v.detail),
            EX_TEBAKO_LEDGER,
        ));
    }
    Ok(())
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINES: [&str; 4] = [
        "1761000000 event=fetch origin=https://example.test/a.tfs sha256=abc",
        "1761000001 event=runtime-fetch-verified sha256=abc verdict=ok",
        "1761000002 event=jail-deny path=/etc/shadow op=read source=policy.yaml",
        "1761000003 event=payload-installed name=metanorma version=1.17.0",
    ];

    fn ledger_of(lines: &[&str]) -> Ledger {
        Ledger::parse(format!("{}\n", lines.join("\n")).as_bytes())
    }

    fn seal_over(ledger: &Ledger, ts: u64) -> Seal {
        Seal {
            ts,
            head: chain_head(ledger.lines.iter()),
            lines: ledger.lines.len(),
            key: "-".to_string(),
            sig: "-".to_string(),
            line: 1,
        }
    }

    #[test]
    fn a_record_splits_at_the_first_equals_and_keeps_values_intact() {
        let r = parse_record(1, "1761000000 event=fetch origin=https://x.test/a?b=c&d=e")
            .expect("parses");
        assert_eq!(r.ts, 1_761_000_000);
        assert_eq!(r.event, "fetch");
        assert_eq!(
            r.fields.get("origin").map(String::as_str),
            Some("https://x.test/a?b=c&d=e")
        );
    }

    #[test]
    fn a_token_without_equals_continues_the_previous_value() {
        // A path with a space must not turn a legitimate record into damage.
        let r = parse_record(1, "1761000000 event=jail-deny path=/a b/c op=read").expect("parses");
        assert_eq!(r.fields.get("path").map(String::as_str), Some("/a b/c"));
        assert_eq!(r.fields.get("op").map(String::as_str), Some("read"));
    }

    #[test]
    fn damage_is_reported_with_its_line_and_never_dropped() {
        let ledger =
            Ledger::parse(b"1761000000 event=fetch\ngarbage here\n\n1761000001 event=exec\n");
        assert_eq!(ledger.records.len(), 2);
        assert_eq!(ledger.malformed.len(), 2);
        assert_eq!(ledger.malformed[0].line, 2);
        assert_eq!(ledger.malformed[1].line, 3);
        assert_eq!(ledger.lines.len(), 4);
    }

    #[test]
    fn a_final_record_without_its_newline_is_still_a_record() {
        let ledger = Ledger::parse(b"1761000000 event=fetch");
        assert_eq!(ledger.records.len(), 1);
        assert_eq!(ledger.malformed.len(), 0);
    }

    #[test]
    fn the_chain_changes_when_any_byte_changes_and_preserves_order() {
        let a = ledger_of(&LINES);
        let base = chain_head(a.lines.iter());

        let mut edited = a.lines.clone();
        edited[1] = String::from_utf8(edited[1].clone())
            .unwrap()
            .replace("verdict=ok", "verdict=no")
            .into_bytes();
        assert_ne!(chain_head(edited.iter()), base);

        let mut reordered = a.lines.clone();
        reordered.swap(0, 1);
        assert_ne!(chain_head(reordered.iter()), base);

        let mut removed = a.lines.clone();
        removed.remove(2);
        assert_ne!(chain_head(removed.iter()), base);

        let appended =
            Ledger::parse(format!("{}\n1761000004 event=exec\n", LINES.join("\n")).as_bytes());
        assert_ne!(chain_head(appended.lines.iter()), base);
    }

    #[test]
    fn non_utf8_bytes_chain_distinctly_so_a_byte_swap_cannot_hide() {
        // Both lines render to the SAME replacement character; only the
        // bytes differ. Hashing a lossy rendering would make them equal —
        // and let an edit inside a non-UTF-8 path go unreported.
        let a = Ledger::parse(b"1761000000 event=jail-deny path=/\xff\n");
        let b = Ledger::parse(b"1761000000 event=jail-deny path=/\xfe\n");
        assert_eq!(
            String::from_utf8_lossy(&a.lines[0]),
            String::from_utf8_lossy(&b.lines[0]),
            "the two lines must be indistinguishable as TEXT"
        );
        assert_ne!(
            chain_head(a.lines.iter()),
            chain_head(b.lines.iter()),
            "the chain must still tell the BYTES apart"
        );
    }

    #[test]
    fn a_terminator_only_change_is_not_a_tamper_finding() {
        // The chain normalizes the record terminator: an editor that adds
        // a final newline did not alter the ledger.
        let with = Ledger::parse(format!("{}\n", LINES.join("\n")).as_bytes());
        let without = Ledger::parse(LINES.join("\n").as_bytes());
        assert_eq!(
            chain_head(with.lines.iter()),
            chain_head(without.lines.iter())
        );
    }

    #[test]
    fn a_sealed_ledger_verifies_and_reports_its_unsealed_tail() {
        let mut ledger = ledger_of(&LINES);
        let seal = seal_over(&ledger, 1_761_000_100);
        ledger
            .lines
            .push(b"1761000004 event=exec argv=metanorma".to_vec());
        let v = verify(&ledger, &[seal], b"");
        assert_eq!(v.outcome, Outcome::Sealed);
        assert_eq!(v.sealed_lines, 4);
        assert_eq!(v.tail, 1);
        assert!(v.divergence.is_none());
        assert_eq!(v.signature, "unsigned");
    }

    #[test]
    fn an_altered_record_names_the_interval_that_changed() {
        let ledger = ledger_of(&LINES);
        let seal = seal_over(&ledger, 1_761_000_100);
        let mut tampered = ledger_of(&LINES);
        tampered.lines[2] =
            b"1761000002 event=jail-allow path=/etc/shadow op=read source=policy.yaml".to_vec();
        tampered.records = Ledger::parse(&tampered.lines.concat()).records;
        let v = verify(&tampered, &[seal], b"");
        assert_eq!(v.outcome, Outcome::Tampered);
        // No earlier seal survives, so the whole covered range is named.
        assert_eq!(v.divergence, Some((1, 4)));
        assert!(v.detail.contains("altered"), "{}", v.detail);
    }

    #[test]
    fn an_earlier_seal_still_matching_narrows_the_interval() {
        let ledger = ledger_of(&LINES);
        let early = Seal {
            ts: 1_761_000_050,
            head: chain_head(ledger.lines.iter().take(2)),
            lines: 2,
            key: "-".to_string(),
            sig: "-".to_string(),
            line: 1,
        };
        let late = seal_over(&ledger, 1_761_000_100);
        let mut tampered = ledger_of(&LINES);
        tampered.lines[3] =
            b"1761000003 event=payload-installed name=metanorma version=9.9.9".to_vec();
        let v = verify(&tampered, &[early, late], b"");
        assert_eq!(v.outcome, Outcome::Tampered);
        // Records 1–2 still match the early seal, so the damage starts at 3.
        assert_eq!(v.divergence, Some((3, 4)));
    }

    #[test]
    fn removed_records_are_gone_not_merely_altered() {
        let ledger = ledger_of(&LINES);
        let seal = seal_over(&ledger, 1_761_000_100);
        let shrunk = Ledger::parse(format!("{}\n", LINES[..2].join("\n")).as_bytes());
        let v = verify(&shrunk, &[seal], b"");
        assert_eq!(v.outcome, Outcome::Tampered);
        assert_eq!(v.divergence, Some((3, 4)));
        assert!(v.detail.contains("GONE"), "{}", v.detail);
    }

    #[test]
    fn no_seal_is_unsealed_and_names_the_remedy() {
        let ledger = ledger_of(&LINES);
        let v = verify(&ledger, &[], b"");
        assert_eq!(v.outcome, Outcome::Unsealed);
        assert!(v.detail.contains("--seal"), "{}", v.detail);
    }

    #[test]
    fn a_seal_round_trips_through_its_rendered_line() {
        let seal = Seal {
            ts: 1_761_000_100,
            head: "a".repeat(64),
            lines: 7,
            key: "0123456789abcdef".to_string(),
            sig: "deadbeef".to_string(),
            line: 1,
        };
        let text = format!("{}\n", seal.render());
        let parsed = parse_seals(&text).expect("parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].head, seal.head);
        assert_eq!(parsed[0].lines, 7);
        assert_eq!(parsed[0].key, "0123456789abcdef");
        assert!(parsed[0].signed());
        assert_eq!(parsed[0].payload(), seal.payload());
    }

    #[test]
    fn a_damaged_seal_file_is_a_finding_not_a_skip() {
        let err = parse_seals("1761000000 head=nothex lines=1 key=- sig=-\n").unwrap_err();
        assert!(err.1.contains("malformed head"), "{}", err.1);
        let err = parse_seals("nonsense\n").unwrap_err();
        assert!(err.1.contains("timestamp"), "{}", err.1);
    }

    #[test]
    fn filters_compose_with_and_and_events_with_or() {
        let ledger = ledger_of(&LINES);
        let mut q = Query {
            since: Some(1_761_000_001),
            until: Some(1_761_000_003),
            ..Query::default()
        };
        q.events.insert("jail-deny".to_string());
        q.events.insert("payload-installed".to_string());
        let picked = q.select(&ledger);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].event, "jail-deny");
        assert_eq!(picked[1].event, "payload-installed");
    }

    #[test]
    fn the_artifact_filter_reads_the_body_not_the_timestamp() {
        let ledger = ledger_of(&LINES);
        let q = Query {
            artifact: Some("metanorma".to_string()),
            ..Query::default()
        };
        let picked = q.select(&ledger);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].line, 4);
        // The timestamp is not part of the searched body.
        let q = Query {
            artifact: Some("761000002".to_string()),
            ..Query::default()
        };
        assert!(q.select(&ledger).is_empty());
    }

    #[test]
    fn tail_keeps_the_last_matches_in_file_order() {
        let ledger = ledger_of(&LINES);
        let q = Query {
            tail: Some(2),
            ..Query::default()
        };
        let picked = q.select(&ledger);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].line, 3);
        assert_eq!(picked[1].line, 4);
    }

    #[test]
    fn time_filters_read_unix_seconds_and_rfc3339_utc() {
        assert_eq!(parse_time("1761000000").unwrap(), 1_761_000_000);
        assert_eq!(parse_time("2025-10-20T22:40:00Z").unwrap(), 1_761_000_000);
        assert_eq!(parse_time("1970-01-01T00:00:00Z").unwrap(), 0);
        assert!(parse_time("2025-10-21 00:00").is_err());
        assert!(parse_time("2025-13-01T00:00:00Z").is_err());
        assert!(parse_time("yesterday").is_err());
    }

    #[test]
    fn writes_is_true_only_for_the_seal() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(!writes(&args(&["--since", "0"])));
        assert!(!writes(&args(&["--verify"])));
        assert!(writes(&args(&["--verify", "--seal"])));
        assert!(wants_json(&args(&["--json"])));
        assert!(!wants_json(&args(&["--count"])));
    }
}
