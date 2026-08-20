//! `tebako trace cover` — the escapes report (spec 25 §6, phase T3).
//!
//! Correlates an INSIDE stream (the tfs bus's JSONL capture, or any
//! retrace-shaped JSON) against an OUTSIDE capture (retrace-shaped JSON —
//! spec 25 §6.2: the format is retrace's single source) and reports the
//! outside events that touched paths under `--prefix` which the inside
//! stream never saw: the ESCAPES — operations that bypassed the VFS and
//! hit the host filesystem.
//!
//! # Parity contract (spec 25 §6.3, invariant 8/10)
//!
//! The algorithm is a safe-Rust port of retrace's `retrace-correlate`
//! (tools/correlate/match.c + correlate.c, tools/common/stream.c —
//! BSD-2-Clause, Ribose Inc). The shared golden fixtures (a byte-verbatim
//! copy of retrace's `tools/correlate/golden/` tree lives at
//! `crates/tebako-cli/tests/fixtures/correlate/`, SSOT upstream) pin the
//! stdout bytes and the exit code; `tests/trace_cover.rs` asserts them
//! against the real binary. The pinned semantics:
//!
//! - **Tolerant scan** (stream.c): top-level objects are sliced by brace
//!   depth (strings/escapes tracked), so the one-array document, JSONL,
//!   and a truncated tail (a crashed capture) all parse; complete but
//!   corrupt objects count as `skipped`. Everything between objects
//!   (brackets, commas, BOM, CRLF) is ignored.
//! - **Path extraction**: every path-like STRING at ANY depth of an entry
//!   is a path (`is_path_like`: starts with `/` or `\`, or a drive letter
//!   + `:`, or contains a slash).
//! - **Normalization** (`corr_normalize`): the NT prefixes `\??\`,
//!   `//?/` (libsass's forward-slash spelling, only when not followed by
//!   `/`), and `\\?\` strip; `\Device\HarddiskVolumeN\` rewrites to the
//!   DOS drive guess `N -> 'A'+N-1` (volume 3 -> `C:`); backslashes
//!   become slashes; one trailing slash drops (unless it is the whole
//!   path); the 1024-byte cap and the not-path-like rejection drop the
//!   path from correlation.
//! - **Comparison** (`corr_pathcmp`): case-sensitive everywhere EXCEPT a
//!   leading drive-letter pair (`c:/x` == `C:/x`). The PREFIX test is a
//!   plain case-sensitive byte prefix plus a component boundary
//!   (`/mnt/tfs2/...` is NOT under `/mnt/tfs`) — retrace's strncmp,
//!   deliberately not pathcmp.
//! - **Coverage**: an inside record covers an outside touch when the
//!   normalized paths match, the pid covers (equal, or either side
//!   pid-less), and `--window` seconds contain both timestamps (0 = pure
//!   set-difference — the lazy-materialize case). The inside record's
//!   time is the entry's numeric `time` field; the tebako bus's string
//!   `ts` (trace-event.yaml) does not feed `--window` (a retrace-shaped
//!   inside capture is the window mode's input — same as upstream).
//! - **Classification**: `message.func` (+ `message.detail`) drive
//!   probe/read/write: the name-keyed probe set (QueryOpen, stat, access,
//!   … — existence is information: a NAME-NOT-FOUND probe on an
//!   under-prefix path the VFS never served IS an escape), the
//!   name-keyed write set, CreateFile/NtCreateFile with "write" in the
//!   detail, fopen with a w/a mode char; everything else with a func is
//!   read; no func is none. `--exclude-probes` drops probe-class hits.
//! - One escape per outside entry at most: the FIRST under-prefix
//!   uncovered path (entry field order) is the reported one.
//!
//! # The report surfaces (spec 25 §6.3)
//!
//! - **stdout** is the golden contract: one
//!   `escape <path> func=<f> tid=<t> pid=<p> class=<probe|read|write|none>`
//!   line per escape (or the `--json` array), byte-compared against
//!   retrace. Nothing else ever prints to stdout (the version banner
//!   rides stderr for this subcommand — the `cache list --json`
//!   precedent in main.rs).
//! - **stderr** carries the retrace-shaped summary
//!   (`inside=N entries, M paths; prefix=P; escapes=E`) plus the spec's
//!   coverage block: escapes grouped by surface class (fs / exec /
//!   dlopen / spawn), the per-class coverage percentage over all
//!   under-prefix outside touches, and the producing layer named by
//!   `--layer libc|kernel` (default libc — a libc-boundary capture
//!   certifies libc-routed escapes only; sub-libc escapes stay
//!   UNCERTIFIABLE at that layer, §6.1's honesty rule). stderr is
//!   outside the golden contract by design (retrace golden README).
//! - **Exit codes** (retrace parity): 0 = no escapes, 1 = escapes found,
//!   2 = usage or I/O error.
//!
//! Deliberate deviations from retrace-correlate, all on error paths the
//! golden fixtures never exercise (tebako spec 00 law 9 — named errors,
//! never silent fallbacks): a non-numeric `--pid`/`--window` is a usage
//! error (retrace's atol/atof silently read 0); an unknown `--layer` is a
//! usage error. The `--flag=value` spelling rides alongside
//! `--flag value` (the repo's CLI convention; retrace takes only the
//! latter).

use std::cmp::Ordering;
use std::path::PathBuf;

use tebako_json::Value;

/// retrace's CORR_PATH_MAX: normalized paths longer than 1023 bytes drop
/// out of correlation (the C fixed buffer — kept for parity).
const CORR_PATH_MAX: usize = 1024;

/// The parsed `tebako trace cover` argv.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceCoverArgs {
    /// `--inside <path>`: the inside stream (the bus's JSONL capture or a
    /// retrace-shaped JSON document).
    pub inside: PathBuf,
    /// `--outside <path>`: the outside capture (retrace-shaped JSON).
    pub outside: PathBuf,
    /// `--prefix <path>`: the virtualized prefix (normalized like any path).
    pub prefix: String,
    /// `--pid N`: only outside entries with this pid are considered
    /// (0 = every pid).
    pub pid: i64,
    /// `--window SECS`: coverage time window in seconds (0 = pure
    /// set-difference).
    pub window: f64,
    /// `--exclude-probes`: drop probe-class (existence-leak) escapes.
    pub exclude_probes: bool,
    /// `--json`: the JSON escape array instead of the text lines.
    pub json: bool,
    /// `--layer libc|kernel`: the outside capture's producing layer
    /// (spec 25 §6.1 — named on stderr; stdout is unaffected).
    pub layer: Layer,
}

/// The producing layer of the outside capture (spec 25 §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The libc boundary (retrace preload / inline hooks) — the default.
    /// Certifies libc-routed escapes only.
    Libc,
    /// The kernel syscall layer (retrace ptrace, eBPF bridge, ETW/procmon
    /// via the §6.2 converter).
    Kernel,
}

const COVER_USAGE: &str = "usage: tebako trace cover --inside <tfs.json> --outside <retrace.json>\n\
     \x20                       --prefix <path> [--pid N] [--window SECS]\n\
     \x20                       [--exclude-probes] [--json] [--layer libc|kernel]";

/// Parse the `trace cover` argv (retrace-correlate's flag grammar, plus
/// the repo's `--flag=value` spelling). Errors are the usage text or a
/// named option error — the caller exits 2 (retrace parity).
pub fn parse_trace_cover_args(args: &[String]) -> Result<TraceCoverArgs, String> {
    let mut inside = None;
    let mut outside = None;
    let mut prefix = None;
    let mut pid = 0i64;
    let mut window = 0.0f64;
    let mut exclude_probes = false;
    let mut json = false;
    let mut layer = Layer::Libc;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let take_value = |flag: &str, i: &mut usize| -> Result<String, String> {
            match &inline {
                Some(v) => Ok(v.clone()),
                None => {
                    *i += 1;
                    args.get(*i)
                        .cloned()
                        .ok_or_else(|| format!("option '{flag}' requires a value"))
                }
            }
        };
        match flag {
            "--inside" => inside = Some(PathBuf::from(take_value(flag, &mut i)?)),
            "--outside" => outside = Some(PathBuf::from(take_value(flag, &mut i)?)),
            "--prefix" => prefix = Some(take_value(flag, &mut i)?),
            "--pid" => {
                let v = take_value(flag, &mut i)?;
                pid = v
                    .parse()
                    .map_err(|_| format!("option '--pid' needs an integer, got '{v}'"))?;
            }
            "--window" => {
                let v = take_value(flag, &mut i)?;
                window = v
                    .parse()
                    .map_err(|_| format!("option '--window' needs a number, got '{v}'"))?;
            }
            "--exclude-probes" => exclude_probes = true,
            "--json" => json = true,
            "--layer" => {
                let v = take_value(flag, &mut i)?;
                layer = match v.as_str() {
                    "libc" => Layer::Libc,
                    "kernel" => Layer::Kernel,
                    other => {
                        return Err(format!(
                            "unknown layer '{other}' (the producing layers: libc | kernel)"
                        ))
                    }
                };
            }
            _ => return Err(format!("unknown trace cover option '{arg}'\n{COVER_USAGE}")),
        }
        i += 1;
    }
    let (inside, outside, prefix) = match (inside, outside, prefix) {
        (Some(i), Some(o), Some(p)) => (i, o, p),
        _ => return Err(COVER_USAGE.to_string()),
    };
    Ok(TraceCoverArgs {
        inside,
        outside,
        prefix,
        pid,
        window,
        exclude_probes,
        json,
        layer,
    })
}

/// 1 if the string looks like a path (retrace `corr_is_path_like`):
/// starts with `/` or `\`, or a drive letter + `:`, or contains a slash.
pub fn is_path_like(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    if b[0] == b'/' || b[0] == b'\\' {
        return true;
    }
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return true;
    }
    s.contains('/') || s.contains('\\')
}

/// Normalize a path (retrace `corr_normalize`). Returns `None` when the
/// input normalizes to something not path-like or exceeds the 1024 cap
/// (both drop the path from correlation, as upstream).
pub fn corr_normalize(input: &str) -> Option<String> {
    let mut p = input;
    let mut drive = String::new();

    // NT prefix forms -> plain path. The '//?/' spelling is the
    // forward-slash variant libsass builds before flipping the
    // separators (src/file.cpp read_file/file_exists) — stripped only
    // when a non-'/' follows (upstream's guard).
    if let Some(rest) = p.strip_prefix("\\??\\") {
        p = rest;
    } else if p.starts_with("//?/") && matches!(p.as_bytes().get(4), Some(&c) if c != b'/') {
        p = &p[4..];
    } else if let Some(rest) = p.strip_prefix("\\\\?\\") {
        p = rest;
    } else if let Some(rest) = p.strip_prefix("\\Device\\HarddiskVolume") {
        // "\Device\HarddiskVolume3\rest" -> guess a DOS drive letter
        // (volume N -> 'A'+N-1, so volume 3 -> C:). Both sides of the
        // join normalize through this function, so consistency matters
        // more than accuracy (upstream's documented heuristic).
        let bytes = rest.as_bytes();
        let mut i = 0;
        let mut vol: i64 = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            vol = vol * 10 + i64::from(bytes[i] - b'0');
            i += 1;
        }
        if (1..=26).contains(&vol) {
            drive.push((b'A' + vol as u8 - 1) as char);
            drive.push(':');
        }
        // The separator after the volume digits slash-unifies into the
        // '/' of "C:/rest".
        p = &rest[i..];
    }

    // The fixed C buffer: prefix + n + NUL must fit CORR_PATH_MAX.
    if drive.len() + p.len() + 1 > CORR_PATH_MAX {
        return None;
    }
    let mut out = drive;
    out.extend(p.chars().map(|c| if c == '\\' { '/' } else { c }));

    // Drop a trailing slash unless it is the whole path.
    if out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    if !is_path_like(&out) {
        return None;
    }
    Some(out)
}

/// The normalized-path comparison (retrace `corr_pathcmp`):
/// case-sensitive everywhere except a leading drive-letter pair
/// (`c:/x` == `C:/x`). Byte-wise (C strcmp on UTF-8).
pub fn corr_pathcmp(a: &str, b: &str) -> Ordering {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    if ab.len() >= 2
        && bb.len() >= 2
        && ab[0].is_ascii_alphabetic()
        && ab[1] == b':'
        && bb[0].is_ascii_alphabetic()
        && bb[1] == b':'
    {
        let ca = ab[0].to_ascii_uppercase();
        let cb = bb[0].to_ascii_uppercase();
        if ca != cb {
            return ca.cmp(&cb);
        }
        return a[2..].cmp(&b[2..]);
    }
    a.cmp(b)
}

/// A sorted set of normalized paths (retrace `CorrSet`): dedup on insert
/// by corr_pathcmp, sorted at finish for the binary-search contains.
/// Unfinished, `items` keeps insertion order (the per-entry `seen` set
/// relies on that for the first-hit rule).
#[derive(Debug, Default)]
struct CorrSet {
    items: Vec<String>,
}

impl CorrSet {
    fn add(&mut self, path: &str) {
        if !self.items.iter().any(|i| corr_pathcmp(i, path) == Ordering::Equal) {
            self.items.push(path.to_string());
        }
    }

    fn finish(&mut self) {
        self.items.sort_by(|a, b| corr_pathcmp(a, b));
    }

    fn contains(&self, path: &str) -> bool {
        self.items
            .binary_search_by(|i| corr_pathcmp(i, path))
            .is_ok()
    }
}

/// Event classification (retrace `CorrClass`, TODO.windows/03): PROBE =
/// existence leak (read-attributes semantics), WRITE = potential
/// mutation, READ = data access, NONE = no func on the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    None,
    Probe,
    Read,
    Write,
}

impl Class {
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Probe => "probe",
            Class::Read => "read",
            Class::Write => "write",
            Class::None => "none",
        }
    }
}

/// Probe = read-attributes semantics: existence information (upstream's
/// name-keyed set, pre-folded lowercase).
const PROBE_FUNCS: &[&str] = &[
    "queryopen",
    "getfileattributesw",
    "getfileattributesa",
    "ntqueryattributesfile",
    "stat",
    "lstat",
    "fstatat",
    "access",
    "faccessat",
    "stat64",
];

/// Write = potential mutation (upstream's name-keyed set).
const WRITE_FUNCS: &[&str] = &[
    "writefile",
    "ntwritefile",
    "deletefilew",
    "deletefilea",
    "movefilew",
    "movefileexw",
    "copyfilew",
    "rename",
    "unlink",
    "rmdir",
    "truncate",
    "ftruncate",
    "mkdir",
    "mkdirat",
    "creat",
];

/// ASCII-lowercase, truncated to `cap - 1` bytes — retrace's `fold`
/// scratch buffers (the truncation caps are parity, not safety).
fn fold(s: &str, cap: usize) -> String {
    let bytes = s.as_bytes();
    let n = bytes.len().min(cap - 1);
    String::from_utf8_lossy(&bytes[..n].to_ascii_lowercase()).into_owned()
}

/// Classify one event by its func name and detail/params text (retrace
/// `corr_classify`). Heuristics pinned by upstream's unit tests: the
/// probe and write sets are name-keyed; CreateFile/NtCreateFile with
/// "write" in the detail classifies WRITE; fopen with a w/a mode char
/// classifies WRITE; everything else with a func is READ.
pub fn corr_classify(func: Option<&str>, detail: Option<&str>) -> Class {
    let Some(func) = func else {
        return Class::None;
    };
    if func.is_empty() {
        return Class::None;
    }
    let f = fold(func, 128);
    if PROBE_FUNCS.contains(&f.as_str()) {
        return Class::Probe;
    }
    if WRITE_FUNCS.contains(&f.as_str()) {
        return Class::Write;
    }
    if let Some(detail) = detail {
        let d = fold(detail, 256);
        if (f == "createfile" || f == "ntcreatefile") && d.contains("write") {
            return Class::Write;
        }
        // fopen modes: w/a variants write, r variants read.
        if f == "fopen" && (d.contains('w') || d.contains('a')) {
            return Class::Write;
        }
    }
    Class::Read
}

/// One observed path record of the inside stream (retrace `CorrRec`).
#[derive(Debug)]
struct Rec {
    path: String,
    pid: i64,
    time: f64,
}

/// The inside index (retrace `CorrIndex`): records sorted by (path,
/// time) plus the sorted unique-path set for the fast "never seen"
/// reject.
#[derive(Debug, Default)]
struct CorrIndex {
    recs: Vec<Rec>,
    set: CorrSet,
}

/// The entry's numeric field as f64 (parson's json_object_get_number
/// semantics: absent or non-numeric reads 0).
fn entry_number(entry: &Value, key: &str) -> f64 {
    match entry.find(key) {
        Some(Value::Number(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// The entry's numeric field as i64 ((long) on the double — truncation,
/// as upstream).
fn entry_long(entry: &Value, key: &str) -> i64 {
    entry_number(entry, key) as i64
}

/// One depth-first walk over an entry's string values (retrace
/// `walk_paths`): every path-like string, normalized, handed to the sink
/// in document order.
fn walk_paths(v: &Value, sink: &mut dyn FnMut(&str)) {
    match v {
        Value::String(s) => {
            if is_path_like(s) {
                if let Some(norm) = corr_normalize(s) {
                    sink(&norm);
                }
            }
        }
        Value::Object(members) => {
            for (_, member) in members {
                walk_paths(member, sink);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_paths(item, sink);
            }
        }
        _ => {}
    }
}

impl CorrIndex {
    /// Index one inside entry (retrace `corr_index_add_entry`): one
    /// record per extracted path, carrying the entry's pid and time.
    fn add_entry(&mut self, entry: &Value) {
        let pid = entry_long(entry, "pid");
        let time = entry_number(entry, "time");
        walk_paths(entry, &mut |path| {
            self.recs.push(Rec {
                path: path.to_string(),
                pid,
                time,
            });
            self.set.add(path);
        });
    }

    /// Sort records by (path, time) and the set (retrace
    /// `corr_index_finish`) — required before escape queries.
    fn finish(&mut self) {
        self.recs.sort_by(|a, b| {
            corr_pathcmp(&a.path, &b.path).then_with(|| {
                if a.time < b.time {
                    Ordering::Less
                } else if a.time > b.time {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
        });
        self.set.finish();
    }

    /// covered: the path was seen by the inside stream, from a covering
    /// pid (pid-less entries are wildcards), within the window when one
    /// is set (retrace `covered`).
    fn covered(&self, path: &str, pid: i64, time: f64, window: f64) -> bool {
        if !self.set.contains(path) {
            return false;
        }
        // Lower bound of the equal-path record range.
        let lo = self
            .recs
            .partition_point(|rec| corr_pathcmp(&rec.path, path) == Ordering::Less);
        for rec in &self.recs[lo..] {
            if corr_pathcmp(&rec.path, path) != Ordering::Equal {
                break;
            }
            if rec.pid != 0 && pid != 0 && rec.pid != pid {
                continue;
            }
            if window > 0.0 {
                if (rec.time - time).abs() <= window {
                    return true;
                }
            } else {
                return true;
            }
        }
        false
    }
}

/// One escape hit, for reporting (retrace `CorrEscape`).
#[derive(Debug)]
pub struct Escape {
    pub path: String,
    /// The entry's message.func (None renders `-` in text, "" in JSON).
    pub func: Option<String>,
    pub tid: i64,
    pub pid: i64,
    pub time: f64,
    pub class: Class,
}

/// The decision criteria value object (retrace `CorrCriteria`): new
/// criteria extend this struct, never fork the code path.
#[derive(Debug)]
pub struct Criteria {
    /// The normalized virtualization root.
    pub prefix: String,
    /// 0 = every pid.
    pub pid: i64,
    /// Seconds; 0 = pure set-difference.
    pub window: f64,
    /// Drop probe-class hits from the report (jail-grant policy).
    pub exclude_probes: bool,
}

/// The message object's func/detail strings (the classification inputs —
/// retrace reads exactly these two keys of `message`).
fn message_func_detail(entry: &Value) -> (Option<String>, Option<String>) {
    match entry.find("message") {
        Some(msg @ Value::Object(_)) => (
            msg.find("func").and_then(Value::as_string),
            msg.find("detail").and_then(Value::as_string),
        ),
        _ => (None, None),
    }
}

/// 1 if the normalized path is under the normalized prefix: a
/// case-sensitive byte prefix (retrace's strncmp — deliberately not
/// corr_pathcmp) landing on a component boundary.
fn under_prefix(path: &str, prefix: &str) -> bool {
    if !path.starts_with(prefix) {
        return false;
    }
    path.len() == prefix.len() || path.as_bytes()[prefix.len()] == b'/'
}

/// Walk one outside entry (retrace `corr_entry_is_escape`): if it
/// carries a path under the prefix NOT covered by the inside index,
/// the first such hit (entry field order) is the escape. Also feeds the
/// stderr coverage accounting (every under-prefix touch, covered or
/// not — beyond the first-hit return upstream stops at).
fn entry_scan(entry: &Value, criteria: &Criteria, inside: &CorrIndex, scan: &mut EntryScan) {
    if criteria.pid != 0 && criteria.pid != entry_long(entry, "pid") {
        return;
    }
    let mut seen = CorrSet::default();
    walk_paths(entry, &mut |path| seen.add(path));
    if seen.items.is_empty() {
        return;
    }
    let pid = entry_long(entry, "pid");
    let time = entry_number(entry, "time");
    let (func, detail) = message_func_detail(entry);
    let class = corr_classify(func.as_deref(), detail.as_deref());
    for path in &seen.items {
        if !under_prefix(path, &criteria.prefix) {
            continue;
        }
        if inside.covered(path, pid, time, criteria.window) {
            scan.covered.push((path.clone(), class));
            continue;
        }
        scan.escaped.push((path.clone(), class));
        // Jail-grant policy: read-attributes leaks are droppable from
        // the report (but still counted as escapes in the coverage
        // accounting — they ARE escapes, policy-hidden).
        if criteria.exclude_probes && class == Class::Probe {
            continue;
        }
        if scan.escape.is_none() {
            scan.escape = Some(Escape {
                path: path.clone(),
                func: func.clone(),
                tid: entry_long(entry, "tid"),
                pid,
                time,
                class,
            });
        }
    }
}

/// One outside entry's scan outcome: the reportable escape (first hit)
/// plus the coverage accounting over every under-prefix touch.
#[derive(Debug, Default)]
struct EntryScan {
    escape: Option<Escape>,
    covered: Vec<(String, Class)>,
    escaped: Vec<(String, Class)>,
}

/// The tolerant retrace-log scanner (retrace `corr_stream_scan`):
/// top-level objects are sliced at brace depth 0 (quotes and escapes
/// tracked, so an entry carrying "func": "a{b\"c" yields exactly one
/// object) and parsed one by one. Returns (entries handed to the sink,
/// complete-but-corrupt objects skipped). A truncated trailing object is
/// neither counted nor an error; everything between objects (the array
/// brackets, commas, whitespace, a BOM, CRLF) is ignored.
fn scan_stream(text: &str, sink: &mut dyn FnMut(&Value)) -> (usize, usize) {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut esc = false;
    let mut start = 0usize;
    let mut entries = 0usize;
    let mut skipped = 0usize;
    for (i, &c) in bytes.iter().enumerate() {
        if depth == 0 {
            if c == b'{' {
                depth = 1;
                start = i;
                in_str = false;
                esc = false;
            }
            continue;
        }
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        if c == b'"' {
            in_str = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                if let Ok(v @ Value::Object(_)) = tebako_json::parse(&text[start..=i]) {
                    sink(&v);
                    entries += 1;
                } else {
                    skipped += 1;
                }
            }
        }
    }
    (entries, skipped)
}

/// The spec 25 §6.3 surface classes (the stderr coverage grouping).
/// Mapped from the entry's func; an entry with no func lands in Fs (the
/// vast majority of path events are filesystem touches).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Surface {
    Fs,
    Exec,
    Dlopen,
    Spawn,
}

impl Surface {
    fn of_func(func: Option<&str>) -> Surface {
        match func.map(|f| fold(f, 128)).as_deref() {
            Some(
                "execve" | "execv" | "execvp" | "execvpe" | "execl" | "execlp" | "execle"
                | "fexecve" | "createprocess" | "createprocessw" | "createprocessa"
                | "shellexecute" | "shellexecutew" | "shellexecutea",
            ) => Surface::Exec,
            Some(
                "dlopen" | "dlmopen" | "loadlibrary" | "loadlibraryw" | "loadlibrarya"
                | "loadlibraryexw" | "loadlibraryexa" | "ldrloaddll",
            ) => Surface::Dlopen,
            Some("posix_spawn" | "posix_spawnp") => Surface::Spawn,
            _ => Surface::Fs,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Surface::Fs => "fs",
            Surface::Exec => "exec",
            Surface::Dlopen => "dlopen",
            Surface::Spawn => "spawn",
        }
    }
}

/// Per-surface coverage accounting (spec 25 §6.3's per-class coverage
/// percentage over the under-prefix outside touches).
#[derive(Debug, Default)]
pub struct Coverage {
    /// (covered, escaped) per surface class.
    pub per_surface: std::collections::BTreeMap<&'static str, (u64, u64)>,
    /// Probe-class escapes dropped by --exclude-probes (policy-hidden).
    pub probes_excluded: u64,
}

/// The correlation result.
#[derive(Debug)]
pub struct Correlation {
    /// The escapes, in outside-stream order (one per entry at most).
    pub escapes: Vec<Escape>,
    /// Well-formed inside entries consumed.
    pub inside_entries: usize,
    /// Unique normalized inside paths.
    pub inside_paths: usize,
    /// Complete-but-corrupt objects skipped in the outside stream (a
    /// corrupt log, not a truncated one — the retrace summary's note).
    pub outside_skipped: usize,
    /// The coverage accounting (stderr).
    pub coverage: Coverage,
}

/// The correlation kernel (spec 25 §6.3): normalize both sides, subtract
/// every outside-observed op an inside event accounts for; the remainder
/// is the escapes list.
pub fn correlate(inside_text: &str, outside_text: &str, criteria: &Criteria) -> Correlation {
    let mut index = CorrIndex::default();
    let mut inside_entries = 0usize;
    scan_stream(inside_text, &mut |entry| {
        inside_entries += 1;
        index.add_entry(entry);
    });
    index.finish();

    let mut escapes = Vec::new();
    let mut coverage = Coverage::default();
    let (_, outside_skipped) = scan_stream(outside_text, &mut |entry| {
        let mut scan = EntryScan::default();
        entry_scan(entry, criteria, &index, &mut scan);
        // The coverage class is the ENTRY's surface (one func per entry;
        // the first-hit escape carries the same func).
        let (func, _) = message_func_detail(entry);
        let surface = Surface::of_func(func.as_deref());
        if scan.covered.is_empty() && scan.escaped.is_empty() {
            if let Some(escape) = scan.escape {
                escapes.push(escape);
            }
            return;
        }
        let slot = coverage.per_surface.entry(surface.name()).or_default();
        slot.0 += scan.covered.len() as u64;
        slot.1 += scan.escaped.len() as u64;
        for (_, class) in &scan.escaped {
            if criteria.exclude_probes && *class == Class::Probe {
                coverage.probes_excluded += 1;
            }
        }
        if let Some(escape) = scan.escape {
            escapes.push(escape);
        }
    });

    Correlation {
        escapes,
        inside_entries,
        inside_paths: index.set.items.len(),
        outside_skipped,
        coverage,
    }
}

/// The stdout report (the golden contract): one
/// `escape <path> func=<f> tid=<t> pid=<p> class=<c>` line per escape,
/// or retrace's `--json` array. Byte-identical with retrace-correlate.
pub fn render_report(escapes: &[Escape], json: bool) -> String {
    let mut out = String::new();
    if json {
        out.push_str("[\n");
        for (i, esc) in escapes.iter().enumerate() {
            if i > 0 {
                out.push_str(",\n");
            }
            out.push_str("{\n");
            out.push_str(&format!("  \"path\": \"{}\",\n", esc.path));
            out.push_str(&format!(
                "  \"func\": \"{}\",\n",
                esc.func.as_deref().unwrap_or("")
            ));
            out.push_str(&format!("  \"tid\": {},\n", esc.tid));
            out.push_str(&format!("  \"pid\": {},\n", esc.pid));
            out.push_str(&format!("  \"time\": {:.0},\n", esc.time));
            out.push_str(&format!("  \"class\": \"{}\"\n", esc.class.as_str()));
            out.push('}');
        }
        out.push_str(if escapes.is_empty() { "]\n" } else { "\n]\n" });
    } else {
        for esc in escapes {
            out.push_str(&format!(
                "escape {} func={} tid={} pid={} class={}\n",
                esc.path,
                esc.func.as_deref().unwrap_or("-"),
                esc.tid,
                esc.pid,
                esc.class.as_str()
            ));
        }
    }
    out
}

/// The stderr coverage block (spec 25 §6.3 — outside the golden
/// contract): the retrace-shaped summary, the per-surface-class grouping
/// with coverage percentages, and the producing layer named.
fn render_stderr(result: &Correlation, prefix: &str, layer: Layer) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "tebako: trace cover: inside={} entries, {} paths; prefix={}; escapes={}{}\n",
        result.inside_entries,
        result.inside_paths,
        prefix,
        result.escapes.len(),
        if result.outside_skipped > 0 {
            " (entries skipped: corrupt log?)"
        } else {
            ""
        }
    ));
    let layer_note = match layer {
        Layer::Libc => {
            "libc boundary (retrace preload / inline hooks) — certifies libc-routed \
             escapes only; sub-libc escapes (raw syscall, loader-internal probes) are \
             UNCERTIFIABLE at this layer"
        }
        Layer::Kernel => {
            "kernel syscall layer (retrace ptrace / eBPF bridge / ETW-procmon via the \
             converter) — certifies the libc-routed and sub-libc escapes visible to \
             that producer"
        }
    };
    out.push_str(&format!(
        "tebako: trace cover: outside capture layer: {layer_note} (spec 25 §6.1; named by \
         --layer, default libc)\n"
    ));
    if !result.coverage.per_surface.is_empty() {
        out.push_str(
            "tebako: trace cover: coverage by surface class (under-prefix outside touches):\n",
        );
        for (surface, &(covered, escaped)) in &result.coverage.per_surface {
            let total = covered + escaped;
            let pct = if total > 0 {
                100.0 * covered as f64 / total as f64
            } else {
                100.0
            };
            out.push_str(&format!(
                "tebako: trace cover:   {surface}: {covered}/{total} covered ({pct:.1}%), {escaped} escapes\n"
            ));
        }
    }
    if result.coverage.probes_excluded > 0 {
        out.push_str(&format!(
            "tebako: trace cover: {} probe-class escape(s) hidden by --exclude-probes\n",
            result.coverage.probes_excluded
        ));
    }
    out
}

/// `tebako trace cover` — never returns; the process exit code IS the
/// verdict (retrace-correlate parity: 0 no escapes, 1 escapes found,
/// 2 usage or I/O error).
pub fn trace_cover(args: &[String]) -> ! {
    let parsed = match parse_trace_cover_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("tebako: trace cover: {msg}");
            std::process::exit(2);
        }
    };
    let Some(prefix) = corr_normalize(&parsed.prefix) else {
        eprintln!(
            "tebako: trace cover: --prefix is not a path: {}",
            parsed.prefix
        );
        std::process::exit(2);
    };
    let read = |path: &std::path::Path| -> String {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("tebako: trace cover: cannot read {}: {e}", path.display());
                std::process::exit(2);
            }
        }
    };
    let inside_text = read(&parsed.inside);
    let outside_text = read(&parsed.outside);

    let criteria = Criteria {
        prefix: prefix.clone(),
        pid: parsed.pid,
        window: parsed.window,
        exclude_probes: parsed.exclude_probes,
    };
    let result = correlate(&inside_text, &outside_text, &criteria);
    print!("{}", render_report(&result.escapes, parsed.json));
    eprint!("{}", render_stderr(&result, &prefix, parsed.layer));
    std::process::exit(if result.escapes.is_empty() { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// retrace test_correlate_match.c's norm_case.
    fn norm_case(input: &str, want: &str) {
        assert_eq!(corr_normalize(input).as_deref(), Some(want), "normalize {input:?}");
    }

    #[test]
    fn is_path_like_table() {
        assert!(is_path_like("/mnt/tfs/a"));
        assert!(is_path_like("C:\\pkg\\a"));
        assert!(is_path_like("c:/pkg/a"));
        assert!(is_path_like("rel/path"));
        assert!(is_path_like("\\\\server\\share"));
        assert!(!is_path_like("hello"));
        assert!(!is_path_like("a"));
        assert!(!is_path_like(""));
    }

    #[test]
    fn normalize_posix() {
        norm_case("/mnt/tfs/pkg/file.so", "/mnt/tfs/pkg/file.so");
        norm_case("/mnt/tfs/", "/mnt/tfs");
        norm_case("/", "/");
    }

    #[test]
    fn normalize_nt_prefixes() {
        norm_case("\\??\\C:\\pkg\\file", "C:/pkg/file");
        norm_case("\\\\?\\C:\\pkg\\file", "C:/pkg/file");
        // Case is preserved past the drive letter.
        norm_case("\\??\\c:\\PKG\\File", "c:/PKG/File");
        // libsass's forward-slash spelling before it flips the
        // separators (src/file.cpp).
        norm_case("//?/C:\\pkg\\file", "C:/pkg/file");
        norm_case("//?/C:/pkg/file", "C:/pkg/file");
        // POSIX UNC keeps its leading slashes.
        norm_case("//server/share/x", "//server/share/x");
        // "//?/" followed by '/' is not the extended prefix.
        norm_case("//?//server/x", "//?//server/x");
    }

    #[test]
    fn normalize_harddisk_volume() {
        // Volume N -> 'A'+N-1; on a standard install volume 3 is C:.
        norm_case("\\Device\\HarddiskVolume3\\pkg\\file", "C:/pkg/file");
        norm_case("\\Device\\HarddiskVolume4\\pkg\\file", "D:/pkg/file");
        norm_case("\\Device\\HarddiskVolume3\\", "C:");
        // Out of letter range: no drive guess, but the path itself still
        // normalizes (the separator is kept).
        norm_case("\\Device\\HarddiskVolume27\\x", "/x");
    }

    #[test]
    fn normalize_rejects() {
        assert_eq!(corr_normalize("malloc"), None);
        // The 1024 cap: a 2047-byte path drops out of correlation.
        let big = format!("/{}", "a".repeat(2046));
        assert_eq!(corr_normalize(&big), None);
        // Exactly at the cap (1023 bytes out + NUL) still normalizes.
        let fits = format!("/{}", "a".repeat(1022));
        assert_eq!(corr_normalize(&fits).map(|s| s.len()), Some(1023));
    }

    #[test]
    fn pathcmp_table() {
        assert_eq!(corr_pathcmp("c:/pkg/a", "C:/pkg/a"), Ordering::Equal);
        assert_eq!(corr_pathcmp("C:/pkg/a", "C:/pkg/a"), Ordering::Equal);
        assert_eq!(corr_pathcmp("C:/pkg/a", "C:/pkg/b"), Ordering::Less);
        assert_eq!(corr_pathcmp("C:/pkg/b", "C:/pkg/a"), Ordering::Greater);
        // Past the drive letter everything is case-sensitive.
        assert_ne!(corr_pathcmp("C:/PKG/a", "C:/pkg/a"), Ordering::Equal);
        assert_eq!(corr_pathcmp("/a/b", "/a/c"), Ordering::Less);
    }

    #[test]
    fn set_dedupe_and_contains() {
        let mut s = CorrSet::default();
        s.add("/mnt/tfs/a");
        s.add("/mnt/tfs/b");
        // Dedupe is by corr_pathcmp: drive case-insensitive.
        s.add("C:/x");
        s.add("c:/x");
        assert_eq!(s.items.len(), 3);
        s.finish();
        assert!(s.contains("/mnt/tfs/a"));
        assert!(!s.contains("/mnt/tfs/zz"));
        assert!(s.contains("C:/x"));
        assert!(s.contains("c:/x"));
    }

    #[test]
    fn classify_table() {
        assert_eq!(corr_classify(None, None), Class::None);
        assert_eq!(corr_classify(Some("QueryOpen"), None), Class::Probe);
        assert_eq!(corr_classify(Some("GetFileAttributesW"), None), Class::Probe);
        assert_eq!(corr_classify(Some("stat"), None), Class::Probe);
        assert_eq!(corr_classify(Some("access"), None), Class::Probe);
        assert_eq!(
            corr_classify(Some("CreateFile"), Some("Desired Access: Generic Read")),
            Class::Read
        );
        assert_eq!(
            corr_classify(Some("CreateFile"), Some("Desired Access: Generic Write")),
            Class::Write
        );
        assert_eq!(
            corr_classify(Some("NtCreateFile"), Some("Desired Access: Write Data")),
            Class::Write
        );
        assert_eq!(corr_classify(Some("fopen"), Some("mode w")), Class::Write);
        assert_eq!(corr_classify(Some("fopen"), Some("mode r")), Class::Read);
        assert_eq!(corr_classify(Some("WriteFile"), None), Class::Write);
        assert_eq!(corr_classify(Some("unlink"), None), Class::Write);
        assert_eq!(corr_classify(Some("ReadFile"), None), Class::Read);
        assert_eq!(corr_classify(Some("open"), None), Class::Read);
        assert_eq!(Class::Probe.as_str(), "probe");
        assert_eq!(Class::Read.as_str(), "read");
        assert_eq!(Class::Write.as_str(), "write");
        assert_eq!(Class::None.as_str(), "none");
    }

    /// retrace test_correlate_match.c's entry_full, as parsed JSON.
    fn entry_full(func: Option<&str>, path: &str, pid: i64, tid: i64, t: f64) -> Value {
        let func_kv = match func {
            Some(f) => format!(r#""func": "{f}", "#),
            None => String::new(),
        };
        let text = format!(
            r#"{{ "pid": {pid}, "tid": {tid}, "time": {t}, "message": {{ {func_kv}"path": "{path}" }} }}"#
        );
        tebako_json::parse(&text).unwrap()
    }

    fn criteria(prefix: &str) -> Criteria {
        Criteria {
            prefix: prefix.to_string(),
            pid: 0,
            window: 0.0,
            exclude_probes: false,
        }
    }

    fn index_of(entries: &[Value]) -> CorrIndex {
        let mut idx = CorrIndex::default();
        for e in entries {
            idx.add_entry(e);
        }
        idx.finish();
        idx
    }

    /// The escape-only view of entry_scan (retrace corr_entry_is_escape).
    fn is_escape(entry: &Value, c: &Criteria, inside: &CorrIndex) -> Option<Escape> {
        let mut scan = EntryScan::default();
        entry_scan(entry, c, inside, &mut scan);
        scan.escape
    }

    #[test]
    fn escape_set_semantics() {
        let inside = index_of(&[entry_full(Some("open"), "/mnt/tfs/covered.so", 601, 3, 100.0)]);
        let c = criteria("/mnt/tfs");

        let e = entry_full(Some("open"), "/mnt/tfs/covered.so", 601, 3, 101.0);
        assert!(is_escape(&e, &c, &inside).is_none());

        let e = entry_full(Some("creat"), "/mnt/tfs/escape.bin", 601, 9, 102.0);
        let esc = is_escape(&e, &c, &inside).expect("the creat is an escape");
        assert_eq!(esc.path, "/mnt/tfs/escape.bin");
        assert_eq!(esc.func.as_deref(), Some("creat"));
        assert_eq!(esc.tid, 9);
        assert_eq!(esc.pid, 601);
        assert_eq!(esc.class, Class::Write);

        // Prefix matches on a path component, never a substring.
        let e = entry_full(Some("stat"), "/mnt/tfs2/cache.bin", 601, 9, 103.0);
        assert!(is_escape(&e, &c, &inside).is_none());

        let e = entry_full(Some("open"), "/var/log/x", 601, 9, 104.0);
        assert!(is_escape(&e, &c, &inside).is_none());
    }

    #[test]
    fn escape_pid_coverage() {
        // pid 601 saw the path; pid 777 touching the same path is NOT
        // covered (TODO.windows/01).
        let inside = index_of(&[entry_full(Some("open"), "/mnt/tfs/a", 601, 1, 100.0)]);
        let mut c = criteria("/mnt/tfs");

        let e = entry_full(Some("open"), "/mnt/tfs/a", 777, 1, 101.0);
        assert!(is_escape(&e, &c, &inside).is_some());

        // Same pid: covered.
        let e = entry_full(Some("open"), "/mnt/tfs/a", 601, 1, 101.0);
        assert!(is_escape(&e, &c, &inside).is_none());

        // Pid-less outside entry: wildcard, covered.
        let e = entry_full(Some("open"), "/mnt/tfs/a", 0, 1, 101.0);
        assert!(is_escape(&e, &c, &inside).is_none());

        // --pid filter drops other pids entirely.
        c.pid = 601;
        let e = entry_full(Some("open"), "/mnt/tfs/other", 777, 1, 102.0);
        assert!(is_escape(&e, &c, &inside).is_none());
        c.pid = 777;
        assert!(is_escape(&e, &c, &inside).is_some());
    }

    #[test]
    fn escape_time_window() {
        // Lazy materialization: the open at t=100 PRECEDES the
        // materialize record at t=101 (TODO.windows/02).
        let inside = index_of(&[entry_full(
            Some("materialize"),
            "/mnt/tfs/lazy.dat",
            42,
            1,
            101.0,
        )]);

        // Pure set semantics: covered (the path is eventually seen,
        // whenever the materialize landed).
        let mut c = criteria("/mnt/tfs");
        let e = entry_full(Some("open"), "/mnt/tfs/lazy.dat", 42, 7, 100.0);
        assert!(is_escape(&e, &c, &inside).is_none());

        // Window 2s covers the 1s lazy gap.
        c.window = 2.0;
        assert!(is_escape(&e, &c, &inside).is_none());

        // Open at t=98: the materialize 3s later is too late to be the
        // server of this open — an escape under the window.
        let e = entry_full(Some("open"), "/mnt/tfs/lazy.dat", 42, 7, 98.0);
        assert!(is_escape(&e, &c, &inside).is_some());
    }

    #[test]
    fn escape_exclude_probes() {
        let inside = index_of(&[]);
        let mut c = criteria("/mnt/tfs");
        c.exclude_probes = true;

        let e = entry_full(Some("QueryOpen"), "/mnt/tfs/probe.dat", 5, 1, 100.0);
        assert!(is_escape(&e, &c, &inside).is_none());

        let e = entry_full(Some("CreateFile"), "/mnt/tfs/data.dat", 5, 1, 100.0);
        let esc = is_escape(&e, &c, &inside).expect("the CreateFile is an escape");
        assert_eq!(esc.class, Class::Read);

        // Without the flag the probe is reported (and classified).
        c.exclude_probes = false;
        let e = entry_full(Some("QueryOpen"), "/mnt/tfs/probe.dat", 5, 1, 100.0);
        let esc = is_escape(&e, &c, &inside).expect("the probe is reported");
        assert_eq!(esc.class, Class::Probe);
    }

    #[test]
    fn exclude_probes_drops_every_hit_of_a_probe_entry() {
        // Classification is per-ENTRY (func/detail live on the message):
        // an entry with func=QueryOpen carrying two under-prefix escape
        // paths reports NOTHING under --exclude-probes (upstream's
        // `continue` never reaches a non-probe hit in the same entry).
        let inside = index_of(&[]);
        let mut c = criteria("/mnt/tfs");
        c.exclude_probes = true;
        let e = tebako_json::parse(
            r#"{ "pid": 5, "tid": 1, "time": 100,
                 "message": { "func": "QueryOpen", "path": "/mnt/tfs/probe.dat",
                              "other": "/mnt/tfs/also-probe.dat" } }"#,
        )
        .unwrap();
        assert!(is_escape(&e, &c, &inside).is_none());
        // ...but both paths still count as escapes in the coverage
        // accounting (policy-hidden, not covered).
        let mut scan = EntryScan::default();
        entry_scan(&e, &c, &inside, &mut scan);
        assert_eq!(scan.escaped.len(), 2);
    }

    #[test]
    fn one_escape_per_entry_the_first_hit_in_field_order() {
        // An entry carrying two under-prefix escape paths reports the
        // FIRST (document field order — upstream returns at the hit);
        // the coverage accounting still sees both.
        let inside = index_of(&[]);
        let c = criteria("/mnt/tfs");
        let e = tebako_json::parse(
            r#"{ "pid": 5, "tid": 1, "time": 100,
                 "message": { "func": "open", "path": "/mnt/tfs/first",
                              "other": "/mnt/tfs/second" } }"#,
        )
        .unwrap();
        let mut scan = EntryScan::default();
        entry_scan(&e, &c, &inside, &mut scan);
        let esc = scan.escape.expect("the entry escapes");
        assert_eq!(esc.path, "/mnt/tfs/first");
        assert_eq!(scan.escaped.len(), 2);
    }

    #[test]
    fn index_nested_extraction() {
        // Every path-like string at any depth indexes, carrying the
        // entry's pid and time.
        let e = tebako_json::parse(
            r#"{ "pid": 7, "tid": 8, "time": 42,
                 "message": { "func": "open", "path": "/mnt/tfs/a.so",
                              "extra": "/mnt/tfs/b.flag" } }"#,
        )
        .unwrap();
        let idx = index_of(&[e]);
        assert_eq!(idx.recs.len(), 2);
        assert!(idx.set.contains("/mnt/tfs/a.so"));
        assert!(idx.set.contains("/mnt/tfs/b.flag"));
        assert_eq!(idx.recs[0].pid, 7);
        assert_eq!(idx.recs[0].time, 42.0);
    }

    #[test]
    fn stream_scan_shapes() {
        // The one-array document.
        let (entries, skipped) = scan_stream(
            "[\n{ \"a\": 1 }\n,\n{ \"b\": \"/x\" }\n]\n",
            &mut |_| {},
        );
        assert_eq!((entries, skipped), (2, 0));

        // JSONL (one object per line), CRLF and a BOM between entries.
        let (entries, skipped) = scan_stream(
            "\u{feff}{ \"a\": 1 }\r\n{ \"b\": 2 }\r\n",
            &mut |_| {},
        );
        assert_eq!((entries, skipped), (2, 0));

        // A truncated tail drops silently — neither counted nor corrupt.
        let (entries, skipped) = scan_stream(
            "[{ \"a\": 1 }, { \"b\": \"/mnt/tfs/esc",
            &mut |_| {},
        );
        assert_eq!((entries, skipped), (1, 0));

        // A complete but corrupt object counts as skipped.
        let (entries, skipped) = scan_stream("[{ \"a\": 1 }, { nope }]", &mut |_| {});
        assert_eq!((entries, skipped), (1, 1));

        // Braces and quotes inside strings stay in the entry.
        let mut seen = Vec::new();
        let (entries, _) = scan_stream(
            r#"[{ "func": "a{b\"c", "path": "/x" }, { "path": "/y" }]"#,
            &mut |v| seen.push(v.clone()),
        );
        assert_eq!(entries, 2);
        assert_eq!(
            seen[0].find("func").and_then(Value::as_string).as_deref(),
            Some("a{b\"c")
        );
    }

    #[test]
    fn correlate_end_to_end() {
        let inside = "[\n{ \"time\": 100, \"pid\": 5, \"tid\": 1, \"module\": \"tfs\", \"message\": { \"op\": \"open\", \"path\": \"/mnt/tfs/served.dat\" } }\n]\n";
        let outside = concat!(
            "[\n",
            "{ \"time\": 101, \"pid\": 5, \"tid\": 2, \"message\": { \"func\": \"QueryOpen\", \"path\": \"/mnt/tfs/miss.scss\" } },\n",
            "{ \"time\": 102, \"pid\": 5, \"tid\": 3, \"message\": { \"func\": \"CreateFile\", \"path\": \"/mnt/tfs/data.dat\" } },\n",
            "{ \"time\": 103, \"pid\": 5, \"tid\": 4, \"message\": { \"func\": \"open\", \"path\": \"/mnt/tfs/served.dat\" } }\n",
            "]\n",
        );
        let result = correlate(inside, outside, &criteria("/mnt/tfs"));
        assert_eq!(result.inside_entries, 1);
        assert_eq!(result.inside_paths, 1);
        assert_eq!(result.escapes.len(), 2);
        assert_eq!(result.escapes[0].path, "/mnt/tfs/miss.scss");
        assert_eq!(result.escapes[0].class, Class::Probe);
        assert_eq!(result.escapes[1].path, "/mnt/tfs/data.dat");
        // The report is retrace's line format, byte-exact.
        assert_eq!(
            render_report(&result.escapes, false),
            "escape /mnt/tfs/miss.scss func=QueryOpen tid=2 pid=5 class=probe\n\
             escape /mnt/tfs/data.dat func=CreateFile tid=3 pid=5 class=read\n"
        );
        // The coverage accounting: fs surface, 1 covered + 2 escaped.
        let (covered, escaped) = result.coverage.per_surface["fs"];
        assert_eq!((covered, escaped), (1, 2));

        // A clean run renders nothing and exits through the count.
        let result = correlate(inside, inside, &criteria("/mnt/tfs"));
        assert!(result.escapes.is_empty());
        assert_eq!(render_report(&result.escapes, false), "");
        assert_eq!(render_report(&result.escapes, true), "[\n]\n");
    }

    #[test]
    fn json_report_shape() {
        let inside = index_of(&[]);
        let c = criteria("/mnt/tfs");
        let e = entry_full(Some("open"), "/mnt/tfs/x", 601, 9, 1755580003.0);
        let esc = is_escape(&e, &c, &inside).unwrap();
        assert_eq!(
            render_report(&[esc], true),
            "[\n{\n  \"path\": \"/mnt/tfs/x\",\n  \"func\": \"open\",\n  \"tid\": 9,\n  \"pid\": 601,\n  \"time\": 1755580003,\n  \"class\": \"read\"\n}\n]\n"
        );
        // No func: "" in JSON, "-" in text (retrace's emit).
        let e = entry_full(None, "/mnt/tfs/y", 1, 2, 3.0);
        let esc = is_escape(&e, &c, &inside).unwrap();
        assert!(render_report(std::slice::from_ref(&esc), true).contains("\"func\": \"\""));
        assert!(render_report(&[esc], false).contains("func=-"));
    }

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_cover_args_table() {
        let p = parse_trace_cover_args(&args(&[
            "--inside", "in.jsonl",
            "--outside=out.json",
            "--prefix", "C:/pkg",
            "--pid", "9012",
            "--window=2.5",
            "--exclude-probes",
            "--json",
            "--layer", "kernel",
        ]))
        .unwrap();
        assert_eq!(p.inside, PathBuf::from("in.jsonl"));
        assert_eq!(p.outside, PathBuf::from("out.json"));
        assert_eq!(p.prefix, "C:/pkg");
        assert_eq!(p.pid, 9012);
        assert_eq!(p.window, 2.5);
        assert!(p.exclude_probes && p.json);
        assert_eq!(p.layer, Layer::Kernel);

        // The required trio.
        assert!(parse_trace_cover_args(&args(&["--inside", "x"])).is_err());
        assert!(parse_trace_cover_args(&args(&["--inside", "x", "--outside"])).is_err());
        // Unknown flag / bad numbers / bad layer: named usage errors
        // (retrace's atol/atof silent zero is a documented deviation).
        assert!(parse_trace_cover_args(&args(&["--frobnicate"])).is_err());
        assert!(parse_trace_cover_args(&args(&[
            "--inside", "i", "--outside", "o", "--prefix", "/p", "--pid", "abc"
        ]))
        .is_err());
        assert!(parse_trace_cover_args(&args(&[
            "--inside", "i", "--outside", "o", "--prefix", "/p", "--layer", "loader"
        ]))
        .is_err());
    }
}
