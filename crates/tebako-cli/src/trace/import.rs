//! `tebako trace import procmon` — the spec 25 §6.2 offline converter
//! (the rest of phase T3): a Windows procmon CSV export becomes the
//! retrace-shaped JSON document `tebako trace cover` consumes as its
//! outside stream (the kernel-layer producer of §6.1's three-layer
//! model, normalized into retrace's format — producers normalize in,
//! never the reverse; pure Rust, in-process, law 4).
//!
//! # Parity contract
//!
//! A safe-Rust port of retrace's `procmon2retrace`
//! (tools/procmon2retrace/{procmon2retrace,convert,csv}.c — BSD-2-Clause,
//! Ribose Inc; reference: the v2.8.0 tool, verified against the local
//! clone @ v2.13.0). The golden fixture `tests/fixtures/correlate/
//! 06-libsass-importer/` pins both sides: `outside.csv` is the procmon
//! export (byte-verbatim upstream, CRLF + BOM via the .gitattributes
//! `-text` exception) and `outside.json` is upstream's own conversion of
//! it. This converter's stdout on `outside.csv` IS `outside.json`, byte
//! for byte (unit-pinned below); `tests/trace_import.rs` then feeds the
//! conversion to `tebako trace cover` and asserts the case's
//! `expected.txt`/`exit.txt` — the §6.3 golden verdict reproduced from
//! the CSV end.
//!
//! The pinned upstream semantics:
//!
//! - **The CSV scanner** (csv.c): byte-oriented state machine — quoted
//!   fields (embedded commas/CR/LF, `""` escapes), CRLF/LF/CR record
//!   ends, a UTF-8 BOM at buffer start, blank-line records dropped, a
//!   trailing record without a final newline delivered, EOF inside an
//!   open quote dropped and counted (the "truncated final record"
//!   note). The fixed C buffers are parity, not safety: fields
//!   truncate at 4095 bytes, a record keeps at most 16 fields, and a
//!   record's total field storage is 64 KB (a field starting past the
//!   cap converts as empty).
//! - **The header row** (convert.c `pmconv_map_header`): column names
//!   match case-insensitively (`Time of Day`, `Process Name`, `PID`,
//!   `Operation`, `Path`, `Result`, `Detail`); the first record is the
//!   header when `Operation` maps and at least one other column does.
//!   An unrecognized first record is an EVENT under procmon's canonical
//!   column order (the identity map).
//! - **The entry** (convert.c `pmconv_entry`): one JSON object per
//!   record — `time: 0`, `pid` (strtod of the PID column, 0 when
//!   absent), `tid: 0`, `module: "ETW"`, `severity` (`INFO` when the
//!   Result column is `SUCCESS`, else `WARN`), and the `message`
//!   carrying `func` (Operation), `process`, `time_of_day`, `path`,
//!   `result`, `detail` in that order, each omitted when its column is
//!   unmapped or the row is short. A record with an empty/missing
//!   Operation is a bad row: counted, skipped.
//! - **The document** (parson's `json_serialize_to_string_pretty` +
//!   procmon2retrace.c's emission): one JSON array, `[\n`, entries at
//!   brace level 0 with 4-space indents and `,\n` leading separators,
//!   closed `\n]\n` (or `]\n` when empty); strings escape `"` `\` `/`
//!   and the C0 controls exactly as parson; numbers are C's `%1.17g`.
//!
//! # Surface and exit codes (spec 25 §6.2; upstream's verbs)
//!
//! ```text
//! tebako trace import procmon <capture.csv>
//! ```
//!
//! - **stdout** is the JSON document and nothing else (a machine
//!   contract: the version banner rides stderr for this subcommand —
//!   main.rs's machine_stdout rule, the `trace cover` precedent).
//! - **stderr** carries the upstream summary
//!   (`entries=N bad-rows=M`, plus `(truncated final record)` when the
//!   tail dropped) under the tebako prefix.
//! - **Exit codes** are upstream's: 0 = entries emitted, 1 = the
//!   conversion ran but produced zero entries, 2 = usage or I/O error.
//!
//! Documented surface deviations from upstream (flag shape only; the
//! in-format behavior above is byte-parity): upstream's optional
//! `[out.json]` positional is the shell's redirection here (the spec's
//! verb takes exactly one positional), and upstream's conversion-time
//! `--pid N` scope filter rides `tebako trace cover --pid N` downstream
//! (the correlator's pid coverage is the same join, §6.3) — the
//! converted document keeps every pid.

use std::io::Write;
use std::path::PathBuf;

// ---------------------------------------------------------------------
// The CSV scanner (csv.c port)
// ---------------------------------------------------------------------

/// csv.h's PMCSV_MAX_FIELDS: a record keeps at most 16 fields.
const PMCSV_MAX_FIELDS: usize = 16;
/// csv.h's PMCSV_FIELD_MAX: a field truncates at 4095 bytes.
const PMCSV_FIELD_MAX: usize = 4096;
/// csv.h's row storage: PMCSV_FIELD_MAX * PMCSV_MAX_FIELDS — a record's
/// fields (plus their NUL separators) share one 64 KB buffer.
const PMCSV_STORAGE: usize = PMCSV_FIELD_MAX * PMCSV_MAX_FIELDS;

/// One scanned CSV record (upstream's `PmCsvRow`, owned per record —
/// upstream reuses the buffer across the callback; the port hands the
/// callback a borrow of the scanner's row, same lifetime rule).
#[derive(Debug, Default)]
pub struct CsvRow {
    /// The record's fields, unescaped (doubled quotes folded), in
    /// column order. Byte strings: procmon exports are not guaranteed
    /// UTF-8 and the parson emission below is byte-faithful.
    pub fields: Vec<Vec<u8>>,
}

/// The scanner's outcome counters (csv.c's return + `skipped`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScanStats {
    /// Records delivered to the callback.
    pub records: usize,
    /// Records dropped for a missing closing quote at EOF.
    pub truncated: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Between fields (record start or after a comma).
    FieldStart,
    /// Inside an unquoted field.
    Unquoted,
    /// Inside a quoted field.
    Quoted,
    /// Saw `"` inside a quoted field: a doubled quote or the close.
    Quote,
}

/// The scanner's mutable cursor (csv.c's `scan_state`): the shared
/// per-record storage is emulated by `off` (the write cursor, NULs
/// included) plus `cur` (the current field's stored bytes — exactly the
/// bytes upstream's buffer would hold for it).
struct ScanState {
    row: CsvRow,
    state: State,
    off: usize,
    field_len: usize,
    cur: Vec<u8>,
}

impl ScanState {
    fn push(&mut self, c: u8) {
        // csv.c's push: the per-field cap first, then the buffer cap.
        if self.field_len >= PMCSV_FIELD_MAX - 1 {
            return;
        }
        if self.off < PMCSV_STORAGE - 1 {
            self.cur.push(c);
            self.field_len += 1;
            self.off += 1;
        }
    }

    fn end_field(&mut self) {
        if self.row.fields.len() < PMCSV_MAX_FIELDS {
            if self.off >= PMCSV_STORAGE {
                // The field started past the buffer cap: upstream's NUL
                // lands at the buffer end and the field pointer reads as
                // the empty string. (A field IN PROGRESS can never reach
                // this — push stops one byte earlier.)
                self.row.fields.push(Vec::new());
            } else {
                self.row.fields.push(std::mem::take(&mut self.cur));
            }
        }
        self.off += 1; // the NUL's byte, recorded even past the caps
    }

    /// csv.c's end_record: a blank line (one empty field and nothing
    /// else) is not a record.
    fn end_record(&mut self, cb: &mut dyn FnMut(&CsvRow), stats: &mut ScanStats) {
        if self.row.fields.len() > 1 || self.row.fields.first().is_some_and(|f| !f.is_empty()) {
            cb(&self.row);
            stats.records += 1;
        }
        self.row.fields.clear();
        self.off = 0;
    }
}

fn is_rec_end(c: u8) -> bool {
    c == b'\r' || c == b'\n'
}

/// Scan `text` as a procmon CSV export (csv.c's `pmcsv_scan`): `cb`
/// fires once per delivered record; the row is reused, so the callback
/// must copy anything it keeps. A UTF-8 BOM at buffer start is skipped.
pub fn scan_csv(text: &[u8], cb: &mut dyn FnMut(&CsvRow)) -> ScanStats {
    let mut st = ScanState {
        row: CsvRow::default(),
        state: State::FieldStart,
        off: 0,
        field_len: 0,
        cur: Vec::new(),
    };
    let mut stats = ScanStats::default();
    let mut i = 0usize;
    // UTF-8 BOM at buffer start only (Excel round-trips).
    if text.len() >= 3 && text[..3] == [0xEF, 0xBB, 0xBF] {
        i = 3;
    }
    while i < text.len() {
        let c = text[i];
        match st.state {
            State::FieldStart => {
                st.field_len = 0;
                if c == b'"' {
                    st.state = State::Quoted;
                } else if c == b',' {
                    st.end_field();
                } else if is_rec_end(c) {
                    st.end_field();
                    st.end_record(cb, &mut stats);
                    st.state = State::FieldStart;
                    if c == b'\r' && i + 1 < text.len() && text[i + 1] == b'\n' {
                        i += 1;
                    }
                } else {
                    st.push(c);
                    st.state = State::Unquoted;
                }
            }
            State::Unquoted => {
                if c == b',' {
                    st.end_field();
                    st.state = State::FieldStart;
                } else if is_rec_end(c) {
                    st.end_field();
                    st.end_record(cb, &mut stats);
                    st.state = State::FieldStart;
                    if c == b'\r' && i + 1 < text.len() && text[i + 1] == b'\n' {
                        i += 1;
                    }
                } else {
                    st.push(c);
                }
            }
            State::Quoted => {
                if c == b'"' {
                    st.state = State::Quote;
                } else {
                    st.push(c);
                }
            }
            State::Quote => {
                if c == b'"' {
                    st.push(b'"');
                    st.state = State::Quoted;
                } else if c == b',' {
                    st.end_field();
                    st.state = State::FieldStart;
                } else if is_rec_end(c) {
                    st.end_field();
                    st.end_record(cb, &mut stats);
                    st.state = State::FieldStart;
                    if c == b'\r' && i + 1 < text.len() && text[i + 1] == b'\n' {
                        i += 1;
                    }
                } else {
                    // Stray text after a closing quote: tolerated,
                    // appended (csv.c's documented leniency).
                    st.push(c);
                    st.state = State::Unquoted;
                }
            }
        }
        i += 1;
    }

    // A trailing record without a final newline.
    if st.state == State::Quoted {
        stats.truncated += 1; // EOF inside an open quote: dropped
    } else if st.state != State::FieldStart || st.off > 0 {
        // Both sub-branches end the pending field (the FieldStart one
        // is the file ending right after a comma — csv.c sets
        // start=off and ends an empty field).
        st.end_field();
        if st.row.fields.len() > 1 || st.row.fields.first().is_some_and(|f| !f.is_empty()) {
            cb(&st.row);
            stats.records += 1;
        }
    }
    stats
}

// ---------------------------------------------------------------------
// The header mapping (convert.c port)
// ---------------------------------------------------------------------

/// The procmon CSV columns, by role (convert.h's PMCOL_*).
const COL_TOD: usize = 0;
const COL_PROCESS: usize = 1;
const COL_PID: usize = 2;
const COL_OPERATION: usize = 3;
const COL_PATH: usize = 4;
const COL_RESULT: usize = 5;
const COL_DETAIL: usize = 6;
const COL_COUNT: usize = 7;

const COL_NAMES: [&[u8]; COL_COUNT] = [
    b"time of day",
    b"process name",
    b"pid",
    b"operation",
    b"path",
    b"result",
    b"detail",
];

/// convert.c's `ieq`: ASCII case-insensitive equality.
fn ieq(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// The column map: `colmap[role]` is the field index or `NONE`. The
/// identity map is procmon's canonical column order (the fallback when
/// the first record is not a recognizable header).
type ColMap = [usize; COL_COUNT];
const NONE: usize = usize::MAX;
const IDENTITY_COLMAP: ColMap = [0, 1, 2, 3, 4, 5, 6];

/// convert.c's `pmconv_map_header`: the row is a header when Operation
/// maps and at least one other column does (a lone Operation column is
/// not a procmon header).
fn map_header(row: &CsvRow) -> Option<ColMap> {
    let mut colmap = [NONE; COL_COUNT];
    let mut mapped = 0usize;
    for (col, field) in row.fields.iter().enumerate() {
        for role in 0..COL_COUNT {
            if colmap[role] == NONE && ieq(field, COL_NAMES[role]) {
                colmap[role] = col;
                mapped += 1;
                break;
            }
        }
    }
    if colmap[COL_OPERATION] == NONE || mapped <= 1 {
        return None;
    }
    Some(colmap)
}

/// convert.c's `field_or_null`: the mapped field, or None when the
/// column is unmapped or the row is short (a present-but-empty field is
/// Some — the message carries `""`).
fn field<'a>(row: &'a CsvRow, colmap: &ColMap, role: usize) -> Option<&'a [u8]> {
    let idx = colmap[role];
    if idx == NONE {
        return None;
    }
    row.fields.get(idx).map(Vec::as_slice)
}

/// convert.c's pid read: strtod on the PID column (0 when absent or
/// unparsable — the prefix parse below is strtod's decimal grammar:
/// leading whitespace skipped, the longest valid prefix taken).
fn row_pid(row: &CsvRow, colmap: &ColMap) -> f64 {
    field(row, colmap, COL_PID)
        .map(strtod_prefix)
        .unwrap_or(0.0)
}

/// strtod's decimal form (the C-locale `isspace` skip; an exponent is
/// consumed only with at least one digit). Hex floats and the inf/nan
/// words are NOT parsed (a documented garbage-field deviation — procmon
/// PID columns are decimal integers; upstream's strtod would take them,
/// this port reads 0).
fn strtod_prefix(s: &[u8]) -> f64 {
    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }
    let start = i;
    if i < s.len() && matches!(s[i], b'+' | b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - digits_start;
    let mut frac_digits = 0usize;
    if i < s.len() && s[i] == b'.' {
        let frac_start = i + 1;
        let mut j = frac_start;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        frac_digits = j - frac_start;
        if int_digits > 0 || frac_digits > 0 {
            i = j;
        }
    }
    if int_digits == 0 && frac_digits == 0 {
        return 0.0;
    }
    let mut end = i;
    if i < s.len() && matches!(s[i], b'e' | b'E') {
        let mut j = i + 1;
        if j < s.len() && matches!(s[j], b'+' | b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            end = j;
        }
    }
    std::str::from_utf8(&s[start..end])
        .ok()
        .and_then(|t| t.parse::<f64>().ok())
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------
// The JSON emission (parson's pretty printer, byte-exact)
// ---------------------------------------------------------------------

/// parson's `json_serialize_string` over bytes: `"` `\` `/` (the
/// XML-embeddable escape) and the C0 controls; every other byte is
/// copied verbatim (parson does not validate UTF-8 on output).
fn parson_escape_into(out: &mut Vec<u8>, s: &[u8]) {
    out.push(b'"');
    for &c in s {
        match c {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'/' => out.extend_from_slice(b"\\/"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x0c => out.extend_from_slice(b"\\f"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            c if c < 0x20 => out.extend_from_slice(format!("\\u{c:04x}").as_bytes()),
            c => out.push(c),
        }
    }
    out.push(b'"');
}

/// C's `%1.17g` (parson's FLOAT_FORMAT): 17 significant digits, the
/// fixed style for exponents in [-4, 17), the scientific style
/// otherwise, trailing zeros stripped.
fn format_g17(v: f64) -> String {
    const P: i32 = 17;
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    // The %e form at P-1 decimals: the printed exponent is the
    // post-rounding one (Rust normalizes the mantissa to [1, 10)).
    let sci = format!("{:.*e}", (P - 1) as usize, v.abs());
    let epos = sci.rfind('e').expect("Rust's {:e} always carries e");
    let x: i32 = sci[epos + 1..].parse().expect("Rust's {:e} exponent");
    let strip = |s: String| -> String {
        if s.contains('.') {
            let s = s.trim_end_matches('0');
            s.strip_suffix('.').unwrap_or(s).to_string()
        } else {
            s
        }
    };
    if (-4..P).contains(&x) {
        strip(format!("{:.*}", (P - 1 - x) as usize, v))
    } else {
        let mantissa = strip(sci[..epos].to_string());
        let sign = if v < 0.0 { "-" } else { "" };
        format!(
            "{sign}{mantissa}e{}{:02}",
            if x < 0 { "-" } else { "+" },
            x.abs()
        )
    }
}

/// One CSV record → one retrace-shaped entry, parson-pretty at brace
/// level 0 (convert.c's `pmconv_entry` + parson's serializer), as the
/// document bytes. `None` is the bad row: an empty/missing Operation
/// (upstream counts and skips).
fn entry_json(row: &CsvRow, colmap: &ColMap) -> Option<Vec<u8>> {
    let op = field(row, colmap, COL_OPERATION)?;
    if op.is_empty() {
        return None;
    }
    let result = field(row, colmap, COL_RESULT);
    let severity = match result {
        Some(r) if ieq(r, b"success") => "INFO",
        _ => "WARN",
    };
    let mut out = Vec::new();
    out.extend_from_slice(b"{\n");
    out.extend_from_slice(format!("    \"time\": {},\n", format_g17(0.0)).as_bytes());
    out.extend_from_slice(
        format!("    \"pid\": {},\n", format_g17(row_pid(row, colmap))).as_bytes(),
    );
    out.extend_from_slice(b"    \"tid\": 0,\n");
    out.extend_from_slice(b"    \"module\": \"ETW\",\n");
    out.extend_from_slice(format!("    \"severity\": \"{severity}\",\n").as_bytes());
    out.extend_from_slice(b"    \"message\": {\n");
    // convert.c's set_if order: func, process, time_of_day, path,
    // result, detail — each present only when its column mapped and the
    // row carried it.
    let members: [(usize, &[u8]); 6] = [
        (COL_OPERATION, b"func"),
        (COL_PROCESS, b"process"),
        (COL_TOD, b"time_of_day"),
        (COL_PATH, b"path"),
        (COL_RESULT, b"result"),
        (COL_DETAIL, b"detail"),
    ];
    let set: Vec<(&[u8], &[u8])> = members
        .iter()
        .filter_map(|(role, key)| field(row, colmap, *role).map(|v| (*key, v)))
        .collect();
    for (i, (key, value)) in set.iter().enumerate() {
        out.extend_from_slice(b"        ");
        parson_escape_into(&mut out, key);
        out.extend_from_slice(b": ");
        parson_escape_into(&mut out, value);
        out.extend_from_slice(if i + 1 < set.len() { b",\n" } else { b"\n" });
    }
    out.extend_from_slice(b"    }\n}");
    // The emitted keys are ASCII constants and the escape table covers
    // every byte that could break the string, so the document is
    // well-formed for arbitrary field bytes — a non-UTF-8 field is
    // emitted byte-verbatim, exactly as upstream's parson emits it.
    Some(out)
}

/// The conversion result counters (the stderr summary).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImportStats {
    /// Entries emitted into the document.
    pub entries: usize,
    /// Records skipped for an empty/missing Operation.
    pub bad_rows: usize,
    /// Records dropped for a missing closing quote at EOF.
    pub truncated: usize,
}

/// Convert a procmon CSV export into the retrace-shaped JSON array
/// document (procmon2retrace.c's main flow): the header row maps the
/// columns or the first record converts under the canonical order.
pub fn convert_procmon(text: &[u8]) -> (Vec<u8>, ImportStats) {
    struct Conv {
        colmap: ColMap,
        header_done: bool,
        first: bool,
        out: Vec<u8>,
        stats: ImportStats,
    }
    let mut conv = Conv {
        colmap: IDENTITY_COLMAP,
        header_done: false,
        first: true,
        out: b"[\n".to_vec(),
        stats: ImportStats::default(),
    };
    let scan = scan_csv(text, &mut |row| {
        if !conv.header_done {
            conv.header_done = true;
            if let Some(colmap) = map_header(row) {
                conv.colmap = colmap;
                return; // the header is consumed, not an event
            }
            conv.colmap = IDENTITY_COLMAP;
        }
        match entry_json(row, &conv.colmap) {
            Some(entry) => {
                if !conv.first {
                    conv.out.extend_from_slice(b",\n");
                }
                conv.first = false;
                conv.out.extend_from_slice(&entry);
                conv.out.push(b'\n');
                conv.stats.entries += 1;
            }
            None => conv.stats.bad_rows += 1,
        }
    });
    conv.stats.truncated = scan.truncated;
    conv.out.extend_from_slice(if conv.stats.entries > 0 {
        b"\n]\n"
    } else {
        b"]\n"
    });
    (conv.out, conv.stats)
}

// ---------------------------------------------------------------------
// The CLI verb
// ---------------------------------------------------------------------

/// The parsed `tebako trace import` argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceImportArgs {
    /// The procmon CSV export to convert.
    pub csv: PathBuf,
}

const IMPORT_USAGE: &str = "usage: tebako trace import procmon <capture.csv>";

/// Parse the `trace import` argv (spec 25 §6.2's exact surface: one
/// format token, one positional). Errors are the usage text or a named
/// error — the caller exits 2 (the trace-verbs convention).
pub fn parse_trace_import_args(args: &[String]) -> Result<TraceImportArgs, String> {
    match args {
        [format, csv] if format == "procmon" => Ok(TraceImportArgs {
            csv: PathBuf::from(csv),
        }),
        [format, ..] if format != "procmon" && !format.starts_with('-') => Err(format!(
            "unknown import format '{format}' (the outside producers: procmon)\n{IMPORT_USAGE}"
        )),
        _ => Err(IMPORT_USAGE.to_string()),
    }
}

/// `tebako trace import procmon <csv>` — never returns; the process
/// exit code is upstream procmon2retrace's: 0 entries emitted, 1 zero
/// entries, 2 usage or I/O error.
pub fn trace_import(args: &[String]) -> ! {
    let parsed = match parse_trace_import_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("tebako: trace import: {msg}");
            std::process::exit(2);
        }
    };
    let text = match std::fs::read(&parsed.csv) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "tebako: trace import: cannot read {}: {e}",
                parsed.csv.display()
            );
            std::process::exit(2);
        }
    };
    let (doc, stats) = convert_procmon(&text);
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if let Err(e) = lock.write_all(&doc).and_then(|()| lock.flush()) {
        eprintln!("tebako: trace import: cannot write stdout: {e}");
        std::process::exit(2);
    }
    eprintln!(
        "tebako: trace import: entries={} bad-rows={}{}",
        stats.entries,
        stats.bad_rows,
        if stats.truncated > 0 {
            " (truncated final record)"
        } else {
            ""
        }
    );
    std::process::exit(if stats.entries > 0 { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(text: &[u8]) -> (Vec<Vec<Vec<u8>>>, ScanStats) {
        let mut rows = Vec::new();
        let stats = scan_csv(text, &mut |row| rows.push(row.fields.clone()));
        (rows, stats)
    }

    fn row(fields: &[&[u8]]) -> CsvRow {
        CsvRow {
            fields: fields.iter().map(|f| f.to_vec()).collect(),
        }
    }

    #[test]
    fn csv_scan_basic_shapes() {
        // CRLF, LF and CR record ends; quoted fields with an embedded
        // comma, CRLF and a doubled quote; the BOM at buffer start.
        let (rows, stats) =
            scan(b"\xEF\xBB\xBF\"a\",\"b,b\"\r\n\"c\r\nd\",\"e\"\"f\"\nlast,one\rfinal,line");
        assert_eq!(
            stats,
            ScanStats {
                records: 4,
                truncated: 0
            }
        );
        assert_eq!(rows[0], vec![b"a".to_vec(), b"b,b".to_vec()]);
        assert_eq!(rows[1], vec![b"c\r\nd".to_vec(), b"e\"f".to_vec()]);
        assert_eq!(rows[2], vec![b"last".to_vec(), b"one".to_vec()]);
        assert_eq!(rows[3], vec![b"final".to_vec(), b"line".to_vec()]);

        // Blank lines are not records; unquoted fields pass through.
        let (rows, stats) = scan(b"\n\na,b\n\n\nc\n");
        assert_eq!(stats.records, 2);
        assert_eq!(rows[0], vec![b"a".to_vec(), b"b".to_vec()]);
        assert_eq!(rows[1], vec![b"c".to_vec()]);

        // EOF inside an open quote drops the record and counts it.
        let (rows, stats) = scan(b"a,b\n\"truncated,never-closed");
        assert_eq!(
            stats,
            ScanStats {
                records: 1,
                truncated: 1
            }
        );
        assert_eq!(rows.len(), 1);

        // Stray text after a closing quote is tolerated and appended.
        let (rows, _) = scan(b"\"quoted\"tail,next\n");
        assert_eq!(rows[0], vec![b"quotedtail".to_vec(), b"next".to_vec()]);

        // A trailing comma at EOF completes an empty final field.
        let (rows, _) = scan(b"a,b,");
        assert_eq!(rows[0], vec![b"a".to_vec(), b"b".to_vec(), Vec::new()]);
    }

    #[test]
    fn csv_scan_caps() {
        // The 4095-byte field cap truncates.
        let big = vec![b'x'; 5000];
        let mut text = b"hdr,".to_vec();
        text.extend_from_slice(&big);
        text.push(b'\n');
        let (rows, _) = scan(&text);
        assert_eq!(rows[0][1].len(), PMCSV_FIELD_MAX - 1);

        // Fields past the 16th are ignored.
        let text = (0..20)
            .map(|i| format!("f{i}"))
            .collect::<Vec<_>>()
            .join(",")
            + "\n";
        let (rows, _) = scan(text.as_bytes());
        assert_eq!(rows[0].len(), PMCSV_MAX_FIELDS);
        assert_eq!(rows[0][15], b"f15".to_vec());

        // The 64 KB record storage is sized to exactly fit the other
        // caps (16 fields × (4095 + the NUL) = 65536), so the
        // past-the-cap empty-field branch is unreachable in upstream
        // and in this port alike: 16 full fields survive verbatim.
        let field = vec![b'x'; PMCSV_FIELD_MAX - 1];
        let mut text = Vec::new();
        for _ in 0..PMCSV_MAX_FIELDS {
            text.extend_from_slice(&field);
            text.push(b',');
        }
        text.push(b'\n');
        let (rows, _) = scan(&text);
        // (17th field past the comma at EOF: ignored, the trailing
        // comma completes an unrecorded field.)
        assert_eq!(rows[0].len(), PMCSV_MAX_FIELDS);
        assert!(rows[0].iter().all(|f| f.len() == PMCSV_FIELD_MAX - 1));
    }

    #[test]
    fn header_mapping_table() {
        // The canonical header maps, case-insensitively.
        let h = row(&[
            b"Time of Day",
            b"Process Name",
            b"PID",
            b"Operation",
            b"Path",
            b"Result",
            b"Detail",
        ]);
        assert_eq!(map_header(&h), Some(IDENTITY_COLMAP));
        let h = row(&[
            b"time of day",
            b"process name",
            b"pid",
            b"operation",
            b"path",
            b"result",
            b"detail",
        ]);
        assert_eq!(map_header(&h), Some(IDENTITY_COLMAP));

        // A reordered/subset header maps by name.
        let h = row(&[b"Operation", b"Path", b"Result"]);
        let mut want = [NONE; COL_COUNT];
        want[COL_OPERATION] = 0;
        want[COL_PATH] = 1;
        want[COL_RESULT] = 2;
        assert_eq!(map_header(&h), Some(want));

        // No Operation column, or ONLY the Operation column: not a
        // header (the row converts as an event under the identity map).
        assert_eq!(map_header(&row(&[b"Time of Day", b"Path"])), None);
        assert_eq!(map_header(&row(&[b"Operation"])), None);
    }

    #[test]
    fn entry_shaping_table() {
        // The full row, canonical order (the fixture's first record).
        let r = row(&[
            b"11:02:01.1001220 AM",
            b"sassc.exe",
            b"9012",
            b"QueryOpen",
            b"C:\\pkg\\scss\\_a.scss",
            b"NAME NOT FOUND",
            b"SyncType: Sync+Create",
        ]);
        let entry = entry_json(&r, &IDENTITY_COLMAP).unwrap();
        let entry = String::from_utf8(entry).unwrap();
        assert_eq!(
            entry,
            "{\n    \"time\": 0,\n    \"pid\": 9012,\n    \"tid\": 0,\n    \"module\": \"ETW\",\n    \"severity\": \"WARN\",\n    \"message\": {\n        \"func\": \"QueryOpen\",\n        \"process\": \"sassc.exe\",\n        \"time_of_day\": \"11:02:01.1001220 AM\",\n        \"path\": \"C:\\\\pkg\\\\scss\\\\_a.scss\",\n        \"result\": \"NAME NOT FOUND\",\n        \"detail\": \"SyncType: Sync+Create\"\n    }\n}"
        );

        // SUCCESS (any case) is INFO; a missing Result column is WARN.
        let mut ok = r.fields.clone();
        ok[COL_RESULT] = b"Success".to_vec();
        let ok_row = CsvRow { fields: ok };
        let entry = String::from_utf8(entry_json(&ok_row, &IDENTITY_COLMAP).unwrap()).unwrap();
        assert!(entry.contains("\"severity\": \"INFO\""), "{entry}");
        let mut noresult = [NONE; COL_COUNT];
        noresult[COL_OPERATION] = 1;
        noresult[COL_PATH] = 0;
        let entry =
            String::from_utf8(entry_json(&row(&[b"/x", b"open"]), &noresult).unwrap()).unwrap();
        assert!(entry.contains("\"severity\": \"WARN\""), "{entry}");
        assert!(!entry.contains("\"result\""), "{entry}");
        assert!(entry.contains("\"pid\": 0,"), "{entry}");

        // A short row omits the trailing columns (an empty PRESENT
        // field is kept).
        let short = row(&[b"tod", b"proc", b"5", b"open"]);
        let entry = String::from_utf8(entry_json(&short, &IDENTITY_COLMAP).unwrap()).unwrap();
        assert!(entry.contains("\"func\": \"open\""), "{entry}");
        assert!(!entry.contains("\"path\""), "{entry}");
        let empty_detail = row(&[b"t", b"p", b"5", b"open", b"/x", b"SUCCESS", b""]);
        let entry =
            String::from_utf8(entry_json(&empty_detail, &IDENTITY_COLMAP).unwrap()).unwrap();
        assert!(entry.contains("\"detail\": \"\""), "{entry}");

        // The bad row: an empty or missing Operation.
        assert!(entry_json(&row(&[b"t", b"p", b"5", b""]), &IDENTITY_COLMAP).is_none());
        assert!(entry_json(&row(&[b"t"]), &IDENTITY_COLMAP).is_none());
    }

    #[test]
    fn g17_table() {
        // C's %1.17g, the parson number spelling.
        assert_eq!(format_g17(0.0), "0");
        assert_eq!(format_g17(9012.0), "9012");
        assert_eq!(format_g17(0.5), "0.5");
        assert_eq!(format_g17(-3.0), "-3");
        assert_eq!(format_g17(1234.5678), "1234.5678");
        assert_eq!(format_g17(1e17), "1e+17");
        // 2.5e-5 is not exactly representable; at 17 significant digits
        // C's %1.17g prints the neighbor tail too (printf-verified).
        assert_eq!(format_g17(2.5e-5), "2.5000000000000001e-05");
        // An exact binary value stays short.
        assert_eq!(format_g17(7.62939453125e-06), "7.62939453125e-06");
        assert_eq!(format_g17(1e-4), "0.0001");
        assert_eq!(format_g17(-1.25e100), "-1.25e+100");
    }

    #[test]
    fn strtod_prefix_table() {
        assert_eq!(strtod_prefix(b"9012"), 9012.0);
        assert_eq!(strtod_prefix(b"  42"), 42.0);
        assert_eq!(strtod_prefix(b"-7"), -7.0);
        assert_eq!(strtod_prefix(b"3.5e2"), 350.0);
        assert_eq!(strtod_prefix(b"1e"), 1.0); // the exponent needs digits
        assert_eq!(strtod_prefix(b"12junk"), 12.0);
        assert_eq!(strtod_prefix(b""), 0.0);
        assert_eq!(strtod_prefix(b"abc"), 0.0);
        assert_eq!(strtod_prefix(b"."), 0.0);
        assert_eq!(strtod_prefix(b".5"), 0.5);
    }

    #[test]
    fn document_shape() {
        // Zero entries: "[\n]\n" (and the empty input converts, it does
        // not error — the exit code carries the zero).
        let (doc, stats) = convert_procmon(b"");
        assert_eq!(doc, b"[\n]\n");
        assert_eq!(stats.entries, 0);

        // The header alone consumes the first record.
        let (doc, stats) = convert_procmon(
            b"\"Time of Day\",\"Process Name\",\"PID\",\"Operation\",\"Path\",\"Result\",\"Detail\"\r\n",
        );
        assert_eq!(doc, b"[\n]\n");
        assert_eq!(stats.entries, 0);

        // An unrecognized first record is an EVENT under the canonical
        // column order (procmon2retrace.c's fallthrough).
        let (doc, stats) = convert_procmon(b"t,p,7,open,/x,SUCCESS,d\n");
        assert_eq!(stats.entries, 1);
        assert!(String::from_utf8_lossy(&doc).contains("\"pid\": 7,"));

        // The separator: ",\n" before every entry after the first; the
        // close is "\n]\n".
        let (doc, _) = convert_procmon(b"t,p,1,open,/a,SUCCESS,d\nt,p,2,open,/b,SUCCESS,d\n");
        let text = String::from_utf8_lossy(&doc);
        assert!(text.starts_with("[\n{\n"), "{text}");
        assert!(text.contains("}\n,\n{\n"), "{text}");
        assert!(text.ends_with("}\n\n]\n"), "{text}");
    }

    #[test]
    fn golden_csv_converts_byte_for_byte() {
        // The parity contract: tests/fixtures/correlate/06-libsass-
        // importer/outside.json is upstream procmon2retrace's own
        // conversion of outside.csv — reproduce it exactly.
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/correlate/06-libsass-importer"
        );
        let csv = std::fs::read(format!("{dir}/outside.csv")).unwrap();
        let want = std::fs::read(format!("{dir}/outside.json")).unwrap();
        let (doc, stats) = convert_procmon(&csv);
        assert_eq!(stats.entries, 6);
        assert_eq!(stats.bad_rows, 0);
        assert_eq!(stats.truncated, 0);
        assert_eq!(
            doc,
            want,
            "the conversion drifted from upstream's outside.json:\n--- want ---\n{}\n--- got ---\n{}",
            String::from_utf8_lossy(&want),
            String::from_utf8_lossy(&doc)
        );
    }

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_import_args_table() {
        let p = parse_trace_import_args(&args(&["procmon", "capture.csv"])).unwrap();
        assert_eq!(p.csv, PathBuf::from("capture.csv"));

        assert!(parse_trace_import_args(&args(&[])).is_err());
        assert!(parse_trace_import_args(&args(&["procmon"])).is_err());
        let err = parse_trace_import_args(&args(&["strace", "x.log"])).unwrap_err();
        assert!(err.contains("unknown import format 'strace'"), "{err}");
        assert!(parse_trace_import_args(&args(&["procmon", "a.csv", "extra"])).is_err());
    }
}
