//! `tebako trace explain` — diagnosis (spec 25 §5, phase T4).
//!
//! Replays a capture (the bus's JSONL stream — a finished file or a
//! live run's growing one; the tolerant per-line scan drops a crashed
//! tail, the same leniency as `trace run`'s synthesis) into the hop
//! chain — mount → manifest read → resolve → materialize → OS bind —
//! and prints the FIRST hop whose verdict is red: the earliest event
//! matching a signature, or the end-of-stream absence signature when no
//! event matched.
//!
//! # The signature table is data
//!
//! §5's placement law: the table lives in `explain-signatures.yaml` in
//! this crate (embedded with `include_str!` — the shipped binary reads
//! no runtime data file), extended as incidents teach new signatures,
//! never hard-coded into the bus. The engine below interprets the
//! `when.kind` discriminators; an unknown kind is a named load error
//! (spec 00 law 9 — the table is compiled-in data, so a load failure is
//! a build-time bug the unit suite pins first).
//!
//! # Surface
//!
//! ```text
//! tebako trace explain <capture.jsonl>
//! ```
//!
//! - **stdout** is the diagnosis report and nothing else (the version
//!   banner rides stderr for this subcommand — main.rs's
//!   machine_stdout rule): the replay line (event counts, the hop
//!   chain), then the RED hop with its evidence event, or the GREEN
//!   verdict.
//! - **stderr** carries the stream-tolerance notes (a skipped partial
//!   line) — outside the report contract.
//! - **Exit codes** (the trace-verbs convention): 0 = no red hop, 1 =
//!   a red hop named, 2 = usage or I/O error.

use std::path::PathBuf;

use serde::Deserialize;
use tebako_json::Value;

/// The embedded signature table (the §5 data placement).
const SIGNATURES_YAML: &str = include_str!("explain-signatures.yaml");

// ---------------------------------------------------------------------
// The signature table (the data model)
// ---------------------------------------------------------------------

/// The parsed `explain-signatures.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureTable {
    #[allow(dead_code)]
    schema: String,
    #[allow(dead_code)]
    schema_version: u32,
    /// The hop chain, printed as the report's axis (§5's order).
    pub hop_chain: Vec<String>,
    /// The signature corpus, in table order.
    pub signatures: Vec<Signature>,
}

/// One signature: the stream shape → the named hop.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    /// The table key (reported as `[signature: <name>]`).
    pub name: String,
    /// The red hop this signature names (a hop-chain member or the
    /// cross-cutting `policy`).
    pub hop: String,
    /// §5's named-hop sentence.
    pub diagnosis: String,
    /// The stream predicate.
    pub when: When,
    /// The incident provenance (carried into the report).
    pub note: String,
}

/// The stream predicates (the `when.kind` discriminators).
/// (serde's deny_unknown_fields cannot ride an internally tagged enum;
/// the load test pins the table's spellings instead.)
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum When {
    /// End-of-stream absence: the stream carried at least `min_events`
    /// events but no `op` event with exactly this verdict. Evaluated
    /// only when no per-event signature matched (the absent hop is
    /// chronologically first anyway — the env image mounts first).
    Absent {
        op: String,
        verdict: String,
        #[serde(default = "default_min_events")]
        min_events: usize,
    },
    /// One event: op + verdict prefix, optionally the errno set and a
    /// closure-walk requirement.
    Event {
        op: String,
        verdict_prefix: String,
        #[serde(default)]
        errno: Vec<i64>,
        #[serde(default)]
        closure: Option<ClosureRule>,
    },
    /// The jail correlation: a deny immediately preceding a dependent
    /// open-class error (the error's path is the denied path or beneath
    /// it; a non-deny jail event for the denied path clears the pending
    /// deny).
    DenyThenError { deny: DenyRule, error: ErrorRule },
}

fn default_min_events() -> usize {
    1
}

/// The closure-walk requirement on an `event` match (the dlopen
/// detail): the deps list must exist, be non-empty when `non_empty`,
/// and every dep's verdict must be in `dep_verdicts`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureRule {
    #[serde(default)]
    pub non_empty: bool,
    pub dep_verdicts: Vec<String>,
}

/// The deny arm of the jail correlation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DenyRule {
    pub op: String,
    pub verdict_prefix: String,
}

/// The error arm of the jail correlation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorRule {
    pub ops: Vec<String>,
    pub verdict_prefixes: Vec<String>,
}

/// Load and validate the embedded table. Fails loudly on a schema
/// drift (an unknown field/kind, an empty chain, a duplicate name) —
/// compiled-in data that does not load is a build-time bug.
pub fn load_signatures() -> Result<SignatureTable, String> {
    let table: SignatureTable = serde_yml::from_str(SIGNATURES_YAML)
        .map_err(|e| format!("explain-signatures.yaml: {e}"))?;
    if table.hop_chain.is_empty() {
        return Err("explain-signatures.yaml: the hop chain is empty".to_string());
    }
    if table.signatures.is_empty() {
        return Err("explain-signatures.yaml: no signatures".to_string());
    }
    let mut names = std::collections::HashSet::new();
    for sig in &table.signatures {
        if !names.insert(sig.name.as_str()) {
            return Err(format!(
                "explain-signatures.yaml: duplicate signature '{}'",
                sig.name
            ));
        }
    }
    Ok(table)
}

// ---------------------------------------------------------------------
// The replay
// ---------------------------------------------------------------------

/// One event, flattened to what the signatures read.
struct FlatEvent {
    /// The 1-based capture line number (the evidence reference).
    line: usize,
    ts: String,
    pid: i64,
    tid: i64,
    op: String,
    path: String,
    verdict: String,
    errno: Option<i64>,
    detail: Option<Value>,
}

impl FlatEvent {
    fn from_value(line: usize, doc: &Value) -> FlatEvent {
        let get_str = |key: &str| doc.find(key).and_then(Value::as_string).unwrap_or_default();
        let errno = match doc.find("errno") {
            Some(Value::Number(s)) => s.parse::<i64>().ok(),
            _ => None,
        };
        FlatEvent {
            line,
            ts: get_str("ts"),
            pid: match doc.find("pid") {
                Some(Value::Number(s)) => s.parse().unwrap_or(0),
                _ => 0,
            },
            tid: match doc.find("tid") {
                Some(Value::Number(s)) => s.parse().unwrap_or(0),
                _ => 0,
            },
            op: get_str("op"),
            path: get_str("path"),
            verdict: get_str("verdict"),
            errno,
            detail: doc.find("detail").cloned(),
        }
    }

    /// The event's errno: the typed field, else the verdict suffix
    /// (`error:<n>` / `denied:<rule>` carry it — trace-event.yaml).
    fn errno(&self) -> Option<i64> {
        self.errno.or_else(|| {
            let (_, suffix) = self.verdict.split_once(':')?;
            suffix.parse().ok()
        })
    }

    /// The evidence line (the report's pointer into the capture).
    fn evidence(&self) -> String {
        let errno = self
            .errno
            .map(|e| format!(" errno={e}"))
            .unwrap_or_default();
        format!(
            "event #{} ({} pid={} tid={}): {} {} verdict={}{}",
            self.line, self.ts, self.pid, self.tid, self.op, self.path, self.verdict, errno
        )
    }
}

/// A red hop: the signature that fired plus its evidence.
#[derive(Debug)]
pub struct RedHop {
    /// The signature's table key.
    pub signature: String,
    /// The named red hop.
    pub hop: String,
    /// §5's named-hop sentence.
    pub diagnosis: String,
    /// The evidence event's line (0 for the absence signature).
    pub evidence: Option<String>,
    /// A related event line (the policy-denial signature's deny).
    pub related: Option<String>,
    /// The bisect candidates (the os-bind signature's closure deps).
    pub bisect: Vec<String>,
    /// The signature's provenance note.
    pub note: String,
}

/// The replay's outcome.
#[derive(Debug)]
pub struct Diagnosis {
    /// Well-formed events replayed.
    pub events: usize,
    /// Lines dropped as partial/corrupt (the crashed-tail leniency).
    pub skipped_lines: usize,
    /// The first red hop, when one fired.
    pub red: Option<RedHop>,
}

/// The pending deny of the deny-then-error correlation.
struct PendingDeny<'a> {
    sig: &'a Signature,
    path: String,
    evidence: String,
}

/// 1 if `path` is the denied path or beneath it (a component boundary
/// — `/secret/data/file` is dependent on `/secret/data`, `/secret/data2`
/// is not).
fn dependent(path: &str, denied: &str) -> bool {
    path == denied || path.starts_with(&format!("{denied}/"))
}

/// The closure-walk requirement check (the dlopen detail): deps exist,
/// non-empty when the rule asks, every dep's verdict in the rule's set.
fn closure_matches(detail: Option<&Value>, rule: &ClosureRule) -> bool {
    let Some(deps) = detail
        .and_then(|d| d.find("closure"))
        .and_then(|c| c.find("deps"))
    else {
        return false;
    };
    let Value::Array(deps) = deps else {
        return false;
    };
    if rule.non_empty && deps.is_empty() {
        return false;
    }
    deps.iter().all(|dep| {
        dep.find("verdict")
            .and_then(Value::as_string)
            .is_some_and(|v| rule.dep_verdicts.contains(&v))
    })
}

/// The bisect-candidate lines for the os-bind signature: each dep's
/// name → its resolution, with its verdict.
fn closure_bisect(detail: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(deps)) = detail
        .and_then(|d| d.find("closure"))
        .and_then(|c| c.find("deps"))
    else {
        return Vec::new();
    };
    deps.iter()
        .map(|dep| {
            let name = dep
                .find("name")
                .and_then(Value::as_string)
                .unwrap_or_default();
            let resolved = dep
                .find("resolved")
                .and_then(Value::as_string)
                .unwrap_or_else(|| "(host lookup)".to_string());
            let verdict = dep
                .find("verdict")
                .and_then(Value::as_string)
                .unwrap_or_default();
            format!("{name} → {resolved} ({verdict})")
        })
        .collect()
}

/// One signature's per-event match (the `event` kind).
fn event_matches(sig_when: &When, event: &FlatEvent) -> bool {
    let When::Event {
        op,
        verdict_prefix,
        errno,
        closure,
    } = sig_when
    else {
        return false;
    };
    if event.op != *op || !event.verdict.starts_with(verdict_prefix.as_str()) {
        return false;
    }
    if !errno.is_empty() && !event.errno().is_some_and(|e| errno.contains(&e)) {
        return false;
    }
    if let Some(rule) = closure {
        if !closure_matches(event.detail.as_ref(), rule) {
            return false;
        }
    }
    true
}

/// Replay a capture into the hop chain (spec 25 §5): the first
/// signature match in stream order is the red hop; the absence
/// signatures evaluate at end of stream only when nothing matched.
pub fn replay(capture_text: &str, table: &SignatureTable) -> Diagnosis {
    let mut events = 0usize;
    let mut skipped_lines = 0usize;
    let mut pending: Option<PendingDeny> = None;
    let mut red: Option<RedHop> = None;
    // The absence signatures' suppression: an (op, verdict) pair seen
    // kills the absence rule asking for it.
    let absent_sigs: Vec<&Signature> = table
        .signatures
        .iter()
        .filter(|s| matches!(s.when, When::Absent { .. }))
        .collect();
    let mut absent_alive: Vec<bool> = vec![true; absent_sigs.len()];

    for (i, line) in capture_text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(doc) = tebako_json::parse(line) else {
            // The trailing line may be a crashed tail's partial write;
            // anything else is interleave damage. Both are dropped
            // (the `trace run` synthesis's leniency, §3).
            skipped_lines += 1;
            eprintln!(
                "tebako: trace explain: note: capture line {} is not a complete event — skipped",
                i + 1
            );
            continue;
        };
        let event = FlatEvent::from_value(i + 1, &doc);
        events += 1;

        // The absence rules' liveness.
        for (j, sig) in absent_sigs.iter().enumerate() {
            if let When::Absent { op, verdict, .. } = &sig.when {
                if event.op == *op && event.verdict == *verdict {
                    absent_alive[j] = false;
                }
            }
        }

        // The deny-then-error correlations' state: a fresh deny for the
        // pending path re-anchors the evidence; a non-deny jail event
        // for it clears the correlation (the table's note).
        enum PendingShift {
            Reanchor(String),
            Clear,
        }
        let shift = match &pending {
            Some(p) => {
                let (deny_op, deny_prefix) = match &p.sig.when {
                    When::DenyThenError { deny, .. } => {
                        (deny.op.as_str(), deny.verdict_prefix.as_str())
                    }
                    _ => unreachable!("pending only holds deny-then-error signatures"),
                };
                if event.op == deny_op && event.path == p.path {
                    if event.verdict.starts_with(deny_prefix) {
                        Some(PendingShift::Reanchor(event.evidence()))
                    } else {
                        Some(PendingShift::Clear)
                    }
                } else {
                    None
                }
            }
            None => None,
        };
        match shift {
            Some(PendingShift::Reanchor(evidence)) => {
                if let Some(p) = pending.as_mut() {
                    p.evidence = evidence;
                }
            }
            Some(PendingShift::Clear) => pending = None,
            None => {}
        }

        // The per-event signatures, in table order; the first match in
        // stream order ends the replay (§5: the FIRST red hop).
        for sig in &table.signatures {
            let matched = match &sig.when {
                When::Absent { .. } => false,
                w @ When::Event { .. } => event_matches(w, &event),
                When::DenyThenError { error, .. } => {
                    error.ops.contains(&event.op)
                        && error
                            .verdict_prefixes
                            .iter()
                            .any(|p| event.verdict.starts_with(p.as_str()))
                        && pending.as_ref().is_some_and(|p| {
                            std::ptr::eq(p.sig, sig) && dependent(&event.path, &p.path)
                        })
                }
            };
            if matched {
                red = Some(RedHop {
                    signature: sig.name.clone(),
                    hop: sig.hop.clone(),
                    diagnosis: sig.diagnosis.clone(),
                    evidence: Some(event.evidence()),
                    related: pending.as_ref().map(|p| p.evidence.clone()),
                    bisect: closure_bisect(event.detail.as_ref()),
                    note: sig.note.clone(),
                });
                break;
            }
        }
        if red.is_some() {
            break;
        }

        // A deny event arms its correlation (after the match scan, so a
        // deny never matches its own event).
        for sig in &table.signatures {
            if let When::DenyThenError { deny, .. } = &sig.when {
                if event.op == deny.op && event.verdict.starts_with(deny.verdict_prefix.as_str()) {
                    pending = Some(PendingDeny {
                        sig,
                        path: event.path.clone(),
                        evidence: event.evidence(),
                    });
                }
            }
        }
    }

    // End of stream: the absence signatures (only when nothing matched).
    if red.is_none() {
        for (sig, alive) in absent_sigs.iter().zip(absent_alive) {
            let When::Absent { min_events, .. } = &sig.when else {
                continue;
            };
            if alive && events >= *min_events {
                red = Some(RedHop {
                    signature: sig.name.clone(),
                    hop: sig.hop.clone(),
                    diagnosis: sig.diagnosis.clone(),
                    evidence: None,
                    related: None,
                    bisect: Vec::new(),
                    note: sig.note.clone(),
                });
                break;
            }
        }
    }

    Diagnosis {
        events,
        skipped_lines,
        red,
    }
}

/// The stdout report (the diagnosis contract).
pub fn render_report(
    capture: &std::path::Path,
    diagnosis: &Diagnosis,
    table: &SignatureTable,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "tebako trace explain: {} — {} event(s) replayed (hop chain: {})\n",
        capture.display(),
        diagnosis.events,
        table.hop_chain.join(" → ")
    ));
    match &diagnosis.red {
        Some(red) => {
            out.push_str(&format!(
                "RED hop: {} — {} [signature: {}]\n",
                red.hop, red.diagnosis, red.signature
            ));
            if let Some(evidence) = &red.evidence {
                out.push_str(&format!("evidence: {evidence}\n"));
            } else {
                out.push_str(&format!(
                    "evidence: no `{}` verdict reached the stream in {} event(s) — corroborate with the child's prelude-class stderr (spec 25 §5)\n",
                    // The absence rule's (op, verdict), re-read from the
                    // table for the message.
                    table
                        .signatures
                        .iter()
                        .find(|s| s.name == red.signature)
                        .and_then(|s| match &s.when {
                            When::Absent { op, verdict, .. } => {
                                Some(format!("{op}/{verdict}"))
                            }
                            _ => None,
                        })
                        .unwrap_or_default(),
                    diagnosis.events
                ));
            }
            if let Some(related) = &red.related {
                out.push_str(&format!("related:  {related}\n"));
            }
            if !red.bisect.is_empty() {
                out.push_str(
                    "bisect candidates (the closure walk resolved every dep — the OS loader refused the bind):\n",
                );
                for dep in &red.bisect {
                    out.push_str(&format!("  - {dep}\n"));
                }
            }
            out.push_str(&format!("note: {}\n", red.note));
        }
        None => {
            out.push_str(&format!(
                "GREEN: no red hop — every hop's verdict is clean in {} event(s)\n",
                diagnosis.events
            ));
        }
    }
    out
}

/// `tebako trace explain <capture>` — never returns; the exit code is
/// the diagnosis: 0 no red hop, 1 a red hop named, 2 usage or I/O
/// error (the trace-verbs convention).
pub fn trace_explain(args: &[String]) -> ! {
    const USAGE: &str = "usage: tebako trace explain <capture.jsonl>";
    let parsed: PathBuf = match args {
        [capture] if !capture.starts_with('-') => PathBuf::from(capture),
        [flag, ..] if flag.starts_with('-') => {
            eprintln!("tebako: trace explain: unknown option '{flag}'\n{USAGE}");
            std::process::exit(2);
        }
        _ => {
            eprintln!("tebako: trace explain: {USAGE}");
            std::process::exit(2);
        }
    };
    let text = match std::fs::read_to_string(&parsed) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "tebako: trace explain: cannot read {}: {e}",
                parsed.display()
            );
            std::process::exit(2);
        }
    };
    let table = match load_signatures() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("tebako: trace explain: the signature table does not load: {e}");
            std::process::exit(2);
        }
    };
    let diagnosis = replay(&text, &table);
    print!("{}", render_report(&parsed, &diagnosis, &table));
    std::process::exit(if diagnosis.red.is_some() { 1 } else { 0 });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bus event line, the tfs::trace render shape (trace/mod.rs's
    /// test helper grammar).
    fn event(line_op: &str, path: &str, verdict: &str, ts: &str, detail: &str) -> String {
        format!(
            "{{\"v\":1,\"ts\":\"{ts}\",\"pid\":1,\"tid\":1,\"op\":\"{line_op}\",\"path\":\"{path}\",\"verdict\":\"{verdict}\",\"detail\":{detail},\"dur_us\":3}}"
        )
    }

    fn event_errno(op: &str, path: &str, verdict: &str, errno: i64, detail: &str) -> String {
        format!(
            "{{\"v\":1,\"ts\":\"2026-08-20T01:00:00.000000Z\",\"pid\":1,\"tid\":1,\"op\":\"{op}\",\"path\":\"{path}\",\"verdict\":\"{verdict}\",\"detail\":{detail},\"dur_us\":3,\"errno\":{errno}}}"
        )
    }

    #[test]
    fn the_embedded_table_loads_and_pins_the_seed_rows() {
        // The §5 seed: four signatures, the hop chain in §5's order.
        let table = load_signatures().unwrap();
        assert_eq!(
            table.hop_chain,
            vec![
                "mount",
                "manifest read",
                "resolve",
                "materialize",
                "OS bind"
            ]
        );
        let names: Vec<&str> = table.signatures.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "env-image-never-mounted",
                "os-bind-module-not-found",
                "policy-denial",
                "materialize-error"
            ]
        );
    }

    #[test]
    fn absence_signature_fires_only_on_a_live_stream_without_the_verdict() {
        let table = load_signatures().unwrap();
        // A live stream with no mount/ok: the env image never mounted
        // (a mount ERROR does not suppress — the note on the row).
        let capture = [
            event(
                "mount",
                "/__tfs__",
                "error:2",
                "2026-08-20T01:00:00.000000Z",
                "{}",
            ),
            event(
                "open",
                "/__tfs__/lib/x.rb",
                "error:2",
                "2026-08-20T01:00:01.000000Z",
                "{}",
            ),
        ]
        .join("\n");
        let d = replay(&capture, &table);
        let red = d.red.expect("the absence signature fires");
        assert_eq!(red.signature, "env-image-never-mounted");
        assert_eq!(red.hop, "mount");
        assert!(red.evidence.is_none());

        // A mount/ok suppresses it.
        let capture = [
            event(
                "mount",
                "/__tfs__",
                "ok",
                "2026-08-20T01:00:00.000000Z",
                "{\"action\":\"insert\",\"handle\":1}",
            ),
            event(
                "open",
                "/__tfs__/lib/x.rb",
                "image:/__tfs__",
                "2026-08-20T01:00:01.000000Z",
                "{}",
            ),
        ]
        .join("\n");
        let d = replay(&capture, &table);
        assert!(d.red.is_none());

        // An EMPTY stream is not a diagnosis (min_events).
        let d = replay("", &table);
        assert!(d.red.is_none());
        assert_eq!(d.events, 0);
    }

    #[test]
    fn os_bind_signature_needs_the_closure_walked_and_resolved() {
        let table = load_signatures().unwrap();
        let closure = "{\"closure\":{\"format\":\"elf\",\"deps\":[{\"name\":\"libc.so\",\"resolved\":\"/tfs/lib/libc.so\",\"verdict\":\"materialized\"},{\"name\":\"libSystem.dylib\",\"resolved\":null,\"verdict\":\"host-system\"}]}}";
        // error 126 + every dep materialized|host-system: the OS bind.
        let capture = event_errno("dlopen", "/tfs/lib/app.so", "error:126", 126, closure);
        let d = replay(&capture, &table);
        let red = d.red.expect("the os-bind signature fires");
        assert_eq!(red.signature, "os-bind-module-not-found");
        assert_eq!(red.hop, "OS bind");
        assert_eq!(red.bisect.len(), 2);
        assert!(red.bisect[0].contains("libc.so → /tfs/lib/libc.so (materialized)"));
        assert!(red.bisect[1].contains("libSystem.dylib"));

        // The POSIX analogue: ENOENT.
        let capture = event_errno("dlopen", "/tfs/lib/app.so", "error:2", 2, closure);
        let d = replay(&capture, &table);
        assert_eq!(
            d.red.map(|r| r.signature).as_deref(),
            Some("os-bind-module-not-found")
        );

        // A dep that errored: the closure did NOT resolve — no fire.
        // (The mount/ok keeps the absence signature out of these
        // negative cases: a mount-less fragment always reads as the
        // never-mounted shape, by the signature's design.)
        let mount = event(
            "mount",
            "/__tfs__",
            "ok",
            "2026-08-20T00:59:59.000000Z",
            "{\"action\":\"insert\",\"handle\":1}",
        );
        let bad = "{\"closure\":{\"format\":\"elf\",\"deps\":[{\"name\":\"libx.so\",\"resolved\":null,\"verdict\":\"error:2\"}]}}";
        let capture = format!(
            "{mount}\n{}",
            event_errno("dlopen", "/tfs/lib/app.so", "error:126", 126, bad)
        );
        let d = replay(&capture, &table);
        assert!(d.red.is_none());

        // The cache-hit empty walk (format null, deps []) never fires.
        let empty = "{\"closure\":{\"format\":null,\"deps\":[]}}";
        let capture = format!(
            "{mount}\n{}",
            event_errno("dlopen", "/tfs/lib/app.so", "error:126", 126, empty)
        );
        let d = replay(&capture, &table);
        assert!(d.red.is_none());

        // A different errno (EACCES) is not this signature.
        let capture = format!(
            "{mount}\n{}",
            event_errno("dlopen", "/tfs/lib/app.so", "error:13", 13, closure)
        );
        let d = replay(&capture, &table);
        assert!(d.red.is_none());
    }

    #[test]
    fn policy_denial_joins_the_deny_with_the_dependent_error() {
        let table = load_signatures().unwrap();
        // deny → dependent open error: fires, quoting the deny.
        let capture = [
            event(
                "jail",
                "/secret/data",
                "deny:user",
                "2026-08-20T01:00:00.000000Z",
                "{\"access\":\"read\"}",
            ),
            event(
                "open",
                "/secret/data/file",
                "denied:user",
                "2026-08-20T01:00:01.000000Z",
                "{\"need\":\"read\"}",
            ),
        ]
        .join("\n");
        let d = replay(&capture, &table);
        let red = d.red.expect("the policy-denial signature fires");
        assert_eq!(red.signature, "policy-denial");
        assert_eq!(red.hop, "policy");
        let evidence = red.evidence.unwrap();
        assert!(evidence.contains("open /secret/data/file"), "{evidence}");
        let related = red.related.unwrap();
        assert!(
            related.contains("jail /secret/data verdict=deny:user"),
            "{related}"
        );

        // The same path exactly also dependents; an error: verdict too.
        let capture = [
            event(
                "jail",
                "/secret",
                "deny:manifest",
                "2026-08-20T01:00:00.000000Z",
                "{\"access\":\"write\"}",
            ),
            event(
                "stat",
                "/secret",
                "error:13",
                "2026-08-20T01:00:01.000000Z",
                "{}",
            ),
        ]
        .join("\n");
        let d = replay(&capture, &table);
        assert_eq!(d.red.map(|r| r.signature).as_deref(), Some("policy-denial"));

        // An unrelated path is not dependent (a component boundary).
        let mount = event(
            "mount",
            "/__tfs__",
            "ok",
            "2026-08-20T00:59:59.000000Z",
            "{\"action\":\"insert\",\"handle\":1}",
        );
        let capture = [
            mount.clone(),
            event(
                "jail",
                "/secret/data",
                "deny:user",
                "2026-08-20T01:00:00.000000Z",
                "{\"access\":\"read\"}",
            ),
            event(
                "open",
                "/secret/data2/file",
                "error:2",
                "2026-08-20T01:00:01.000000Z",
                "{}",
            ),
        ]
        .join("\n");
        let d = replay(&capture, &table);
        assert!(d.red.is_none());

        // An intervening non-deny jail event for the path clears it.
        let capture = [
            mount,
            event(
                "jail",
                "/secret",
                "deny:user",
                "2026-08-20T01:00:00.000000Z",
                "{\"access\":\"read\"}",
            ),
            event(
                "jail",
                "/secret",
                "allow:user",
                "2026-08-20T01:00:01.000000Z",
                "{\"access\":\"read\"}",
            ),
            event(
                "open",
                "/secret",
                "error:13",
                "2026-08-20T01:00:02.000000Z",
                "{}",
            ),
        ]
        .join("\n");
        let d = replay(&capture, &table);
        assert!(d.red.is_none());
    }

    #[test]
    fn materialize_error_names_the_exec_cache() {
        let table = load_signatures().unwrap();
        let capture = event_errno(
            "materialize",
            "/tfs/bin/tool",
            "error:28",
            28,
            "{\"dest\":\"dlcache\"}",
        );
        let d = replay(&capture, &table);
        let red = d.red.expect("the materialize signature fires");
        assert_eq!(red.signature, "materialize-error");
        assert_eq!(red.hop, "materialize");
        assert_eq!(red.diagnosis, "exec-cache write failure");
    }

    #[test]
    fn the_first_red_hop_in_stream_order_wins() {
        let table = load_signatures().unwrap();
        // The materialize error precedes the dlopen bind failure: the
        // materialize hop is the report.
        let closure = "{\"closure\":{\"format\":\"elf\",\"deps\":[{\"name\":\"libc.so\",\"resolved\":\"/tfs/lib/libc.so\",\"verdict\":\"materialized\"}]}}";
        let capture = [
            event(
                "mount",
                "/__tfs__",
                "ok",
                "2026-08-20T01:00:00.000000Z",
                "{\"action\":\"insert\",\"handle\":1}",
            ),
            event_errno("materialize", "/tfs/bin/tool", "error:28", 28, "{}"),
            event_errno("dlopen", "/tfs/lib/app.so", "error:126", 126, closure),
        ]
        .join("\n");
        let d = replay(&capture, &table);
        let red = d.red.unwrap();
        assert_eq!(red.signature, "materialize-error");
        assert!(red.evidence.unwrap().contains("event #2"));
    }

    #[test]
    fn the_tolerant_scan_drops_a_crashed_tail() {
        let table = load_signatures().unwrap();
        let capture = [
            event(
                "mount",
                "/__tfs__",
                "ok",
                "2026-08-20T01:00:00.000000Z",
                "{\"action\":\"insert\",\"handle\":1}",
            ),
            event(
                "open",
                "/__tfs__/lib/x.rb",
                "image:/__tfs__",
                "2026-08-20T01:00:01.000000Z",
                "{}",
            ),
            "{\"v\":1,\"ts\":\"2026-08-20T01:00:02".to_string(), // the crashed tail
        ]
        .join("\n");
        let d = replay(&capture, &table);
        assert_eq!(d.events, 2);
        assert_eq!(d.skipped_lines, 1);
        assert!(d.red.is_none());
    }

    #[test]
    fn render_report_shapes() {
        let table = load_signatures().unwrap();
        // RED with evidence.
        let capture = event_errno("materialize", "/tfs/bin/tool", "error:28", 28, "{}");
        let d = replay(&capture, &table);
        let report = render_report(std::path::Path::new("/tmp/c.jsonl"), &d, &table);
        assert!(
            report.contains("hop chain: mount → manifest read → resolve → materialize → OS bind"),
            "{report}"
        );
        assert!(
            report.contains(
                "RED hop: materialize — exec-cache write failure [signature: materialize-error]"
            ),
            "{report}"
        );
        assert!(report.contains("evidence: event #1"), "{report}");

        // The absence signature's evidence names the missing verdict.
        let capture = event("open", "/x", "host", "2026-08-20T01:00:00.000000Z", "{}");
        let d = replay(&capture, &table);
        let report = render_report(std::path::Path::new("/tmp/c.jsonl"), &d, &table);
        assert!(
            report.contains("RED hop: mount — env image never mounted (handoff env lost)"),
            "{report}"
        );
        assert!(report.contains("no `mount/ok` verdict"), "{report}");

        // GREEN.
        let capture = event(
            "mount",
            "/__tfs__",
            "ok",
            "2026-08-20T01:00:00.000000Z",
            "{\"action\":\"insert\",\"handle\":1}",
        );
        let d = replay(&capture, &table);
        let report = render_report(std::path::Path::new("/tmp/c.jsonl"), &d, &table);
        assert!(report.contains("GREEN: no red hop"), "{report}");
    }
}
