//! The spec 25 interception trace bus (phase T1).
//!
//! One structured event per interception decision, appended as one JSONL
//! line per event to the channel named by `TEBAKO_TRACE` (or the driver's
//! `--tebako-trace` argument, which wins when both are set — spec 25 §2).
//! The envelope grammar is owned by `docs/spec/schemas/trace-event.yaml`;
//! this module mirrors it field-for-field (`v:1`).
//!
//! The bus's laws (spec 25 §1/§2, restated where they bind the code):
//!
//! - **Observability never gates.** [`arm`] failure leaves the bus
//!   disarmed with one loud stderr note; the run proceeds. A poisoned
//!   channel lock or a failed write drops the event silently — the trace
//!   channel never fails, blocks, or alters the run it observes.
//! - **Disarmed cost is one branch.** [`Start::now`] is the single gate:
//!   one relaxed atomic load returning `None` when disarmed. Call sites
//!   build events only behind `if let Some(start) = Start::now()`.
//! - **Journal fd discipline.** The channel file is opened once at arm
//!   time (driver boot, before any mount — a path operation, which is
//!   exactly when it is safe: [`crate::journal`]'s rule is that path ops
//!   under the preload's context lock self-deadlock); [`emit`] only ever
//!   issues a bare `write(2)` on the open fd. Append-only, never
//!   policy-gated.
//! - **No unsafe, no shell-outs.** Pure safe Rust; serialization via the
//!   workspace's `tebako-json` (spec 25 §7's blessed dependency).
//!
//! ## pid / tid
//!
//! Every event carries `pid` (the OS process id, re-read per event so
//! forked children report their own) and `tid`. **`tid` is the bus's
//! per-process thread ordinal** — the first thread to emit is 1, then 2,
//! 3, … — not an OS thread id: stable Rust exposes no OS tid and the bus
//! forbids `unsafe` (which rules out the libc call). Grouping by
//! (pid, tid) regroups events by thread identically, which is all the
//! schema contract requires of the pair.
//!
//! ## Children
//!
//! A spawned/exec'd child re-derives the channel at its own driver boot:
//! `TEBAKO_TRACE` is in the inherited environment, the child's driver
//! calls [`arm`], and its events append to the same file. This is the
//! §2 "children re-derive" clause. The POSIX fd-inheritance spelling it
//! also mentions is not the mechanism (dup/CLOEXEC games would need
//! `unsafe`); env re-derivation covers both platforms uniformly. On
//! fork, the already-open fd IS inherited until the child's own [`arm`],
//! so pre-arm child events still land in the same channel — same
//! observable effect, zero unsafe.
//!
//! ## Op vocabulary
//!
//! [`Op`] is the full §2 table. T1 emits mount/open/stat/dlopen/exec/
//! materialize/jail from [`crate::context`]; T2 adds `Spawn` (the same
//! context routing, selected by the preload's posix_spawn surface) and
//! `Resolve` (emitted by the tebako-driver's image-triple resolution —
//! the bus itself stays emitter-agnostic: anything linked against tfs
//! appends through [`emit`]).

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tebako_json::Value;

/// The environment variable naming the trace channel (spec 25 §2). The
/// driver's `--tebako-trace` argument overrides it when both are set.
pub const TRACE_ENV: &str = "TEBAKO_TRACE";

/// The wire envelope version — the schema's `v` field. Additive-only,
/// never bumped (spec 25 §3's evolution rule).
pub const ENVELOPE_VERSION: u32 = 1;

static CHANNEL: Mutex<Option<File>> = Mutex::new(None);
static ARMED: AtomicBool = AtomicBool::new(false);
static TID_NEXT: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// The thread's cached bus ordinal (0 = not yet assigned).
    static TID: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The §2 op table. Wire tokens are the snake_case names; the vocabulary
/// is the schema's, additive-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Mount,
    Open,
    Stat,
    Dlopen,
    Exec,
    /// The posix_spawn surface's routing decision (T2: the preload's
    /// posix_spawn/posix_spawnp path — same grammar as `Exec`, the op
    /// token marking that a child process is created).
    Spawn,
    Materialize,
    Jail,
    /// The driver's `--tebako-image` triple resolution (T2:
    /// tebako-driver's resolve_image — whole / slot:<n> / error:<errno>).
    /// The PLANNED L3 cache resolution (cache/fetched) rides the same op
    /// when its loader-side channel story lands.
    Resolve,
}

impl Op {
    /// The wire token (schema's `op` field).
    pub const fn as_str(self) -> &'static str {
        match self {
            Op::Mount => "mount",
            Op::Open => "open",
            Op::Stat => "stat",
            Op::Dlopen => "dlopen",
            Op::Exec => "exec",
            Op::Spawn => "spawn",
            Op::Materialize => "materialize",
            Op::Jail => "jail",
            Op::Resolve => "resolve",
        }
    }
}

/// One interception decision — one JSONL line. Rendered in schema field
/// order: v, ts, pid, tid, op, path, verdict, detail, dur_us, errno.
pub struct Event {
    op: Op,
    path: String,
    verdict: String,
    detail: Vec<(String, Value)>,
    errno: Option<i32>,
    dur_us: u64,
}

impl Event {
    /// A fresh event with `dur_us: 0` — set the measured duration with
    /// [`Event::dur`] from the [`Start`] token the call site gated on.
    pub fn new(op: Op, path: impl Into<String>, verdict: impl Into<String>) -> Self {
        Event {
            op,
            path: path.into(),
            verdict: verdict.into(),
            detail: Vec::new(),
            errno: None,
            dur_us: 0,
        }
    }

    /// Append one detail key (schema: per-op keys, insertion-ordered).
    pub fn detail(mut self, key: impl Into<String>, value: Value) -> Self {
        self.detail.push((key.into(), value));
        self
    }

    /// Attach the errno of a failed decision (schema: optional int).
    pub fn with_errno(mut self, errno: i32) -> Self {
        self.errno = Some(errno);
        self
    }

    /// Record the decision's wall time from the gating token.
    pub fn dur(mut self, start: Start) -> Self {
        self.dur_us = start.elapsed_us();
        self
    }

    /// The JSONL line (no trailing newline) in schema field order.
    fn render(&self) -> String {
        let mut fields = Vec::with_capacity(10);
        fields.push(("v".to_string(), num(ENVELOPE_VERSION)));
        fields.push(("ts".to_string(), Value::String(rfc3339_now())));
        fields.push(("pid".to_string(), num(std::process::id())));
        fields.push(("tid".to_string(), num(tid())));
        fields.push((
            "op".to_string(),
            Value::String(self.op.as_str().to_string()),
        ));
        fields.push(("path".to_string(), Value::String(self.path.clone())));
        fields.push(("verdict".to_string(), Value::String(self.verdict.clone())));
        fields.push(("detail".to_string(), Value::Object(self.detail.clone())));
        fields.push(("dur_us".to_string(), num(self.dur_us)));
        if let Some(errno) = self.errno {
            fields.push(("errno".to_string(), num(errno)));
        }
        tebako_json::to_line(&Value::Object(fields))
    }
}

/// The gating token and the disarmed-cost law in one: `Some` only when
/// the bus is armed, so call sites write
/// `if let Some(start) = Start::now() { …build + emit… }` and the
/// disarmed run costs one relaxed load + branch.
#[derive(Clone, Copy)]
pub struct Start(Instant);

impl Start {
    /// `Some` (and a timestamp) only while the bus is armed.
    pub fn now() -> Option<Start> {
        if ARMED.load(Ordering::Relaxed) {
            Some(Start(Instant::now()))
        } else {
            None
        }
    }

    /// Microseconds since the token was taken (saturating cast).
    pub fn elapsed_us(self) -> u64 {
        u64::try_from(self.0.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

/// Open the channel at `path` (creating parent directories) and arm the
/// bus. Called once per process at driver boot, BEFORE any mount — the
/// open is a path operation, which is exactly when it is safe
/// (journal.rs's discipline). Failure is one loud stderr note and a
/// disarmed bus: the run proceeds (observability never gates). Returns
/// whether the bus armed.
pub fn arm(path: &Path) -> bool {
    match open_channel(path) {
        Ok(file) => {
            // A poisoned lock means a previous emitter panicked
            // mid-write; recover the channel — law 1 says trace trouble
            // never fails the run, arming included.
            let mut guard = CHANNEL.lock().unwrap_or_else(|p| p.into_inner());
            *guard = Some(file);
            ARMED.store(true, Ordering::SeqCst);
            true
        }
        Err(note) => {
            eprintln!("tebako: trace: {note} — trace disabled, the run proceeds");
            false
        }
    }
}

/// Disarm and close the channel. The test/reset seam — production never
/// disarms; the channel lives for the process's life.
pub fn disarm() {
    ARMED.store(false, Ordering::SeqCst);
    let mut guard = CHANNEL.lock().unwrap_or_else(|p| p.into_inner());
    *guard = None;
}

/// Whether the bus is armed (one relaxed load).
pub fn armed() -> bool {
    ARMED.load(Ordering::Relaxed)
}

/// Append one event as one JSONL line. The disarmed fast path is one
/// relaxed load; when armed, one mutex hold around one bare `write(2)`
/// on the channel fd (never a path op — the journal discipline). A
/// poisoned lock or a failed write drops the event: the trace never
/// disturbs the run it observes.
pub fn emit(event: Event) {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    let mut line = event.render();
    line.push('\n');
    if let Ok(mut guard) = CHANNEL.lock() {
        if let Some(file) = guard.as_mut() {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

/// The bus's per-process thread ordinal (see the module docs): 1 for
/// the first thread to emit, then sequential. Cached thread-locally;
/// the counter costs one atomic fetch-add per NEW thread.
pub fn tid() -> u64 {
    TID.with(|cell| {
        let cached = cell.get();
        if cached != 0 {
            cached
        } else {
            let ordinal = TID_NEXT.fetch_add(1, Ordering::Relaxed);
            cell.set(ordinal);
            ordinal
        }
    })
}

/// Envelope/detail shorthand: a JSON number from anything displayable
/// (tebako-json's `Number` carries the literal string).
pub fn num(v: impl fmt::Display) -> Value {
    Value::Number(v.to_string())
}

/// The dlopen event's closure-walk record (spec 25 §2: the dlopen detail
/// carries the closure walk — image format, dep list, per-dep verdict).
/// The top `extract_for_exec` frame fills it; rendered as the detail's
/// `closure` object: `{"format": <token|null>, "deps": [...]}`.
#[derive(Default)]
pub struct ClosureTrace {
    /// The parsed image format token (`macho`/`elf`/`pe`); None when the
    /// header parse was unsupported (or the answer came off the dl
    /// cache) — no dep walk ran.
    pub format: Option<String>,
    /// One entry per declared dependency, in declaration order.
    pub deps: Vec<ClosureDep>,
}

/// One dependency's walk verdict.
pub struct ClosureDep {
    /// The dependency name as the image declares it.
    pub name: String,
    /// The in-image path it resolved to; None for a host/system name.
    pub resolved: Option<String>,
    /// `materialized` | `host-system` | `error:<errno>`.
    pub verdict: String,
}

impl ClosureTrace {
    /// The detail value: `{"format":…, "deps":[{name,resolved,verdict}]}`.
    pub fn into_value(self) -> Value {
        let format = match self.format {
            Some(f) => Value::String(f),
            None => Value::Null,
        };
        let deps = self
            .deps
            .into_iter()
            .map(|dep| {
                let resolved = match dep.resolved {
                    Some(r) => Value::String(r),
                    None => Value::Null,
                };
                Value::Object(vec![
                    ("name".to_string(), Value::String(dep.name)),
                    ("resolved".to_string(), resolved),
                    ("verdict".to_string(), Value::String(dep.verdict)),
                ])
            })
            .collect();
        Value::Object(vec![
            ("format".to_string(), format),
            ("deps".to_string(), Value::Array(deps)),
        ])
    }
}

fn open_channel(path: &Path) -> Result<File, String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))
}

/// Current UTC time as the schema's `ts`: RFC 3339 with microsecond
/// precision (`2026-08-19T13:49:20.344123Z`).
fn rfc3339_now() -> String {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    format_ts(since.as_secs() as i64, since.subsec_micros())
}

/// Pure formatting core (testable): epoch seconds + microseconds →
/// `YYYY-MM-DDTHH:MM:SS.ffffffZ`. Civil date via Hinnant's
/// civil_from_days — no chrono, no tz database (spec 25 §7: no new
/// dependencies outside the workspace).
fn format_ts(secs: i64, micros: u32) -> String {
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}.{micros:06}Z",
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60,
    )
}

/// Howard Hinnant's civil_from_days: days since 1970-01-01 → (year,
/// month, day), proleptic Gregorian, correct for negative days.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::lock_global_context;

    /// Disarm on drop even when a test panics mid-assertion, so armed
    /// state never leaks into the next test on this process-wide bus.
    struct ArmedGuard;
    impl ArmedGuard {
        /// Arm at `path` after clearing any leftover state; the caller
        /// must already hold the crate-wide global-context lock so no
        /// concurrent context test can interleave its events into this
        /// test's capture file.
        fn arm(path: &Path) -> (ArmedGuard, bool) {
            disarm();
            let armed = arm(path);
            (ArmedGuard, armed)
        }
    }
    impl Drop for ArmedGuard {
        fn drop(&mut self) {
            disarm();
        }
    }

    #[test]
    fn format_ts_golden() {
        assert_eq!(format_ts(0, 0), "1970-01-01T00:00:00.000000Z");
        assert_eq!(format_ts(1_700_000_000, 0), "2023-11-14T22:13:20.000000Z");
        assert_eq!(format_ts(1_700_000_000, 42), "2023-11-14T22:13:20.000042Z");
        // Negative seconds: the civil math is proleptic, not clamped.
        assert_eq!(format_ts(-1, 0), "1969-12-31T23:59:59.000000Z");
    }

    #[test]
    fn op_wire_tokens_are_the_schema_vocabulary() {
        let tokens: Vec<&str> = [
            Op::Mount,
            Op::Open,
            Op::Stat,
            Op::Dlopen,
            Op::Exec,
            Op::Spawn,
            Op::Materialize,
            Op::Jail,
            Op::Resolve,
        ]
        .iter()
        .map(|op| op.as_str())
        .collect();
        assert_eq!(
            tokens,
            vec![
                "mount",
                "open",
                "stat",
                "dlopen",
                "exec",
                "spawn",
                "materialize",
                "jail",
                "resolve"
            ]
        );
    }

    #[test]
    fn tid_is_stable_per_thread_and_distinct_across_threads() {
        let mine = tid();
        assert_eq!(mine, tid());
        assert_ne!(mine, 0);
        let theirs = std::thread::spawn(tid).join().expect("spawn tid");
        assert_ne!(mine, theirs);
    }

    #[test]
    fn disarmed_start_is_none() {
        let _lock = lock_global_context();
        disarm();
        assert!(!armed());
        assert!(Start::now().is_none());
    }

    #[test]
    fn arm_failure_stays_disarmed_and_returns_false() {
        let _lock = lock_global_context();
        let tmp = tempfile::tempdir().expect("tempdir");
        // A path whose parent component is a regular FILE: neither
        // create_dir_all nor open can succeed.
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"x").expect("write blocker");
        let (_guard, armed) = ArmedGuard::arm(&blocker.join("child.jsonl"));
        assert!(!armed);
        assert!(!super::armed());
        assert!(Start::now().is_none());
    }

    #[test]
    fn armed_emit_writes_schema_ordered_jsonl() {
        let _lock = lock_global_context();
        let tmp = tempfile::tempdir().expect("tempdir");
        let capture = tmp.path().join("nested").join("trace.jsonl");
        let (_guard, armed) = ArmedGuard::arm(&capture);
        assert!(armed, "arm creates parent directories");

        let start = Start::now().expect("armed start");
        emit(
            Event::new(Op::Open, "/__tfs__/app.rb", "image:/app")
                .detail("need", Value::String("read".to_string()))
                .dur(start),
        );
        emit(Event::new(Op::Open, "/etc/passwd", "denied:user").with_errno(libc::EPERM));
        drop(_guard);

        let text = std::fs::read_to_string(&capture).expect("read capture");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "one JSONL line per event");

        let first = tebako_json::parse(lines[0]).expect("first line parses");
        let get = |key: &str| first.find(key).unwrap_or_else(|| panic!("{key} present"));
        assert_eq!(get("v").as_u64(), Some(1));
        assert_eq!(
            get("pid").as_u64(),
            Some(u64::from(std::process::id())),
            "pid is the emitting process"
        );
        assert!(get("tid").as_u64().expect("tid int") >= 1);
        assert_eq!(get("op").as_string().as_deref(), Some("open"));
        assert_eq!(get("path").as_string().as_deref(), Some("/__tfs__/app.rb"));
        assert_eq!(get("verdict").as_string().as_deref(), Some("image:/app"));
        let ts = get("ts").as_string().expect("ts string");
        assert!(
            ts.ends_with('Z') && ts.contains('T') && ts.contains('.'),
            "rfc3339µs: {ts}"
        );
        assert!(get("dur_us").as_u64().is_some(), "dur_us int");
        assert!(first.find("errno").is_none(), "errno omitted when unset");
        let detail = get("detail");
        assert_eq!(
            detail.find("need").and_then(Value::as_string).as_deref(),
            Some("read")
        );

        // Schema field order on the wire: v, ts, pid, tid, op, path,
        // verdict, detail, dur_us, (errno).
        let order = [
            "\"v\":",
            "\"ts\":",
            "\"pid\":",
            "\"tid\":",
            "\"op\":",
            "\"path\":",
            "\"verdict\":",
            "\"detail\":",
            "\"dur_us\":",
        ];
        let mut at = 0;
        for key in order {
            let found = lines[0][at..]
                .find(key)
                .unwrap_or_else(|| panic!("{key} after offset {at} in {}", lines[0]));
            at += found + key.len();
        }

        let second = tebako_json::parse(lines[1]).expect("second line parses");
        assert_eq!(
            second.find("errno").and_then(Value::as_u64),
            Some(libc::EPERM as u64),
            "errno present when set"
        );
        assert_eq!(
            second.find("verdict").and_then(Value::as_string).as_deref(),
            Some("denied:user")
        );

        // Compact JSONL: no pretty whitespace, no embedded newlines.
        assert!(!lines[0].contains(": "), "compact emission");
    }

    #[test]
    fn closure_trace_renders_the_schema_shape() {
        let ct = ClosureTrace {
            format: Some("macho".to_string()),
            deps: vec![
                ClosureDep {
                    name: "@rpath/libx.dylib".to_string(),
                    resolved: Some("/__tfs__/lib/libx.dylib".to_string()),
                    verdict: "materialized".to_string(),
                },
                ClosureDep {
                    name: "/usr/lib/libSystem.B.dylib".to_string(),
                    resolved: None,
                    verdict: "host-system".to_string(),
                },
            ],
        };
        let line = tebako_json::to_line(&ct.into_value());
        assert_eq!(
            line,
            "{\"format\":\"macho\",\"deps\":[\
{\"name\":\"@rpath/libx.dylib\",\"resolved\":\"/__tfs__/lib/libx.dylib\",\"verdict\":\"materialized\"},\
{\"name\":\"/usr/lib/libSystem.B.dylib\",\"resolved\":null,\"verdict\":\"host-system\"}]}"
        );
        // The empty walk (unsupported header / cache hit): null + [].
        let empty = tebako_json::to_line(&ClosureTrace::default().into_value());
        assert_eq!(empty, "{\"format\":null,\"deps\":[]}");
    }

    #[test]
    fn emit_after_disarm_is_a_no_op() {
        let _lock = lock_global_context();
        let tmp = tempfile::tempdir().expect("tempdir");
        let capture = tmp.path().join("trace.jsonl");
        {
            let (_guard, armed) = ArmedGuard::arm(&capture);
            assert!(armed);
            emit(Event::new(Op::Stat, "/x", "host"));
        } // guard drops → disarmed
        emit(Event::new(Op::Stat, "/y", "host"));
        let text = std::fs::read_to_string(&capture).expect("read capture");
        assert_eq!(text.lines().count(), 1, "the disarmed emit never lands");
    }

    /// The spec 25 §3/§7 property: driving the public tfs op matrix
    /// through the process-global context emits exactly one schema-valid
    /// event per decision. Every line of the capture is validated against
    /// the trace-event.yaml contract (required keys and types, the op
    /// vocabulary, the per-op verdict grammar, errno present exactly when
    /// the verdict names one); the expected (op, verdict) decisions are
    /// then asserted present. The grammar owner is
    /// `docs/spec/schemas/trace-event.yaml`; this test is the CI gate
    /// that document names.
    #[test]
    fn the_op_matrix_emits_one_schema_valid_event_per_decision() {
        use std::io::Write as _;

        let _lock = lock_global_context();
        let tmp = tempfile::tempdir().expect("tempdir");
        let capture = tmp.path().join("nested").join("capture.jsonl");
        let (_guard, armed) = ArmedGuard::arm(&capture);
        assert!(armed);

        // The one-file zip fixture (context.rs's shape).
        let image = tmp.path().join("img.zip");
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("data/secret.txt", options).unwrap();
            writer.write_all(b"hush").unwrap();
            let bytes = writer.finish().unwrap().into_inner();
            std::fs::write(&image, bytes).unwrap();
        }
        let journal_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(tmp.path().join("journal.log"))
            .unwrap();
        // A host file outside every mount, for the passthrough decisions.
        let host_file = tmp.path().join("host.txt");
        std::fs::write(&host_file, b"host").unwrap();
        let host_path = host_file.to_string_lossy().into_owned();

        // Hoisted out of the context block: the redirect assertion below
        // compares against the materialized host path (deferred init —
        // assigned once inside, read after).
        let materialized: std::ffi::CString;
        {
            let mut ctx = crate::context::context().write().unwrap();
            ctx.set_host_policy(
                crate::policy::HostPolicy::bind(
                    crate::policy::PolicyDefault::Record,
                    vec![],
                    vec![],
                )
                .unwrap(),
                Some(journal_file),
            );
            let mount = crate::mount::build_from_file(image.to_str().unwrap(), "/tfs").unwrap();
            ctx.mount_checked(mount).unwrap(); // mount ok (insert)

            let fd = ctx.open("/tfs/data/secret.txt", libc::O_RDONLY).unwrap();
            let _ = ctx.close(fd);
            ctx.stat("/tfs/data/secret.txt").unwrap(); // stat image:/tfs

            // Host passthroughs under the record policy: the op event
            // (`host`) AND the jail event (`record`) — two layers, spec
            // 25 §2's stacked-decision rule.
            assert_eq!(ctx.open(&host_path, libc::O_RDONLY), Err(libc::ENOENT));
            assert_eq!(ctx.stat(&host_path).map(|_| ()), Err(libc::ENOENT));

            // The spec 24 §5 write gate.
            assert_eq!(
                ctx.open("/tfs/data/secret.txt", libc::O_WRONLY),
                Err(libc::EROFS)
            );

            // The exec surface (closure route) extracts first…
            let routed = ctx.exec_materialize("/tfs/data/secret.txt").unwrap();
            assert!(std::path::Path::new(&routed.to_string_lossy().into_owned()).is_file());
            // …then the dlopen surface answers from the dl cache.
            materialized = ctx.dlmap2file("/tfs/data/secret.txt").unwrap();
            assert_eq!(routed, materialized);

            // The spawn surface (spec 25 §2's shared exec/spawn row, phase
            // T2): the same routing decision, emitted as `spawn` — the
            // preload's posix_spawn path's op.
            let spawned = ctx
                .exec_materialize_for_spawn("/tfs/data/secret.txt")
                .unwrap();
            assert_eq!(materialized, spawned);

            // The dlmap-prefix redirect (the §4 class-R materialize
            // signal): opening the materialized copy's path serves a raw
            // host fd under the `image:<mount>` verdict. The fd is left
            // to process exit — closing it would need unsafe, which the
            // bus forbids, tests included.
            let raw = ctx
                .open(materialized.to_str().unwrap(), libc::O_RDONLY)
                .unwrap();
            assert_eq!(raw & crate::context::TEBAKO_FD_FLAG, 0, "a raw host fd");

            // The deny half of the jail channel.
            ctx.set_host_policy(
                crate::policy::HostPolicy::bind(crate::policy::PolicyDefault::Deny, vec![], vec![])
                    .unwrap(),
                None,
            );
            assert_eq!(ctx.open(&host_path, libc::O_RDONLY), Err(libc::EPERM));

            // Reset the shared context for the next test on this process,
            // then the clear-all unmount event.
            ctx.set_host_policy(crate::policy::HostPolicy::open(), None);
            ctx.unmount();
        }
        drop(_guard);

        let text = std::fs::read_to_string(&capture).expect("read capture");
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines.len() >= 15, "the matrix's decisions: {text}");

        // -----------------------------------------------------------
        // Every line validates against the schema contract.
        // -----------------------------------------------------------
        const OPS: [&str; 9] = [
            "mount",
            "open",
            "stat",
            "dlopen",
            "exec",
            "spawn",
            "materialize",
            "jail",
            "resolve",
        ];
        let verdict_ok = |op: &str, verdict: &str| -> bool {
            match op {
                "mount" => verdict == "ok" || verdict.starts_with("error:"),
                "open" | "stat" => {
                    verdict == "host"
                        || verdict.starts_with("image:")
                        || verdict.starts_with("denied:")
                        || verdict.starts_with("error:")
                }
                "dlopen" => {
                    verdict == "host"
                        || verdict.starts_with("materialized:")
                        || verdict.starts_with("error:")
                }
                "exec" => {
                    verdict == "host"
                        || verdict.starts_with("routed:")
                        || verdict.starts_with("error:")
                }
                "materialize" => {
                    verdict == "cache-hit"
                        || verdict.starts_with("ok:")
                        || verdict.starts_with("error:")
                }
                "jail" => {
                    verdict == "record"
                        || verdict.starts_with("allow:")
                        || verdict.starts_with("deny:")
                }
                // T2: the spawn surface shares the exec row's grammar
                // (the preload's posix_spawn path routes here).
                "spawn" => {
                    verdict == "host"
                        || verdict.starts_with("routed:")
                        || verdict.starts_with("error:")
                }
                // T2: the driver's image-triple resolution (whole /
                // slot:<n> / error:<errno>); the L3 cache/fetched half is
                // PLANNED — listed so a stream carrying it validates.
                "resolve" => {
                    verdict == "whole"
                        || verdict.starts_with("slot:")
                        || verdict == "cache"
                        || verdict == "fetched"
                        || verdict.starts_with("error:")
                }
                _ => false,
            }
        };
        let mut seen: Vec<(String, String)> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let doc = tebako_json::parse(line)
                .unwrap_or_else(|e| panic!("line {} parses as JSON: {e}: {line}", i + 1));
            let get = |key: &str| {
                doc.find(key)
                    .unwrap_or_else(|| panic!("line {}: `{key}` present: {line}", i + 1))
            };
            assert_eq!(get("v").as_u64(), Some(1), "line {}", i + 1);
            let ts = get("ts").as_string().expect("ts string");
            assert!(
                ts.ends_with('Z') && ts.contains('T') && ts.contains('.'),
                "line {}: rfc3339µs ts: {ts}",
                i + 1
            );
            assert_eq!(get("pid").as_u64(), Some(u64::from(std::process::id())));
            assert!(
                get("tid").as_u64().is_some_and(|t| t >= 1),
                "line {}",
                i + 1
            );
            let op = get("op").as_string().expect("op string");
            assert!(
                OPS.contains(&op.as_str()),
                "line {}: op in the vocabulary",
                i + 1
            );
            assert!(get("path").as_string().is_some(), "line {}", i + 1);
            let verdict = get("verdict").as_string().expect("verdict string");
            assert!(
                verdict_ok(&op, &verdict),
                "line {}: `{verdict}` is a {op} verdict: {line}",
                i + 1
            );
            assert!(
                matches!(get("detail"), Value::Object(_)),
                "line {}: detail is an object",
                i + 1
            );
            assert!(get("dur_us").as_u64().is_some(), "line {}", i + 1);
            match doc.find("errno") {
                Some(errno) => {
                    let n = errno.as_u64().expect("errno int");
                    assert!(
                        verdict.starts_with("error:")
                            || verdict.starts_with("denied:")
                            || verdict.starts_with("deny:"),
                        "line {}: errno rides a failing verdict: {line}",
                        i + 1
                    );
                    if let Some(suffix) = verdict.strip_prefix("error:") {
                        assert_eq!(
                            suffix.parse::<u64>().ok(),
                            Some(n),
                            "line {}: the verdict's errno IS the field: {line}",
                            i + 1
                        );
                    }
                }
                None => assert!(
                    !verdict.starts_with("error:")
                        && !verdict.starts_with("denied:")
                        && !verdict.starts_with("deny:"),
                    "line {}: a failing verdict carries errno: {line}",
                    i + 1
                ),
            }
            seen.push((op, verdict));
        }

        // -----------------------------------------------------------
        // The expected decisions are all present.
        // -----------------------------------------------------------
        let has = |op: &str, prefix: &str| {
            seen.iter()
                .any(|(o, v)| o == op && (prefix.is_empty() || v.starts_with(prefix)))
        };
        assert!(has("mount", "ok"), "mount ok: {seen:?}");
        assert!(has("open", "image:/tfs"), "open image: {seen:?}");
        assert!(has("stat", "image:/tfs"), "stat image: {seen:?}");
        assert!(has("open", "host"), "open host: {seen:?}");
        assert!(has("stat", "host"), "stat host: {seen:?}");
        assert!(has("jail", "record"), "jail record: {seen:?}");
        assert!(has("open", "denied:write-gate"), "write gate: {seen:?}");
        assert!(has("open", "denied:"), "open denied: {seen:?}");
        assert!(has("jail", "deny:"), "jail deny: {seen:?}");
        assert!(has("dlopen", "materialized:"), "dlopen: {seen:?}");
        assert!(has("exec", "routed:"), "exec routed: {seen:?}");
        assert!(has("spawn", "routed:"), "spawn routed: {seen:?}");
        assert!(has("materialize", "ok:"), "materialize ok: {seen:?}");
        assert!(has("materialize", "cache-hit"), "materialize hit: {seen:?}");

        // The decision-level details (schema's per-op detail grammar).
        let detail_of = |op: &str, prefix: &str| {
            let line = lines[seen
                .iter()
                .position(|(o, v)| o == op && v.starts_with(prefix))
                .unwrap_or_else(|| panic!("a {op} {prefix} event in {seen:?}"))];
            let doc = tebako_json::parse(line).unwrap();
            doc.find("detail").unwrap().clone()
        };
        let insert = detail_of("mount", "ok");
        assert_eq!(
            insert.find("action").and_then(Value::as_string).as_deref(),
            Some("insert")
        );
        assert_eq!(
            insert.find("image").and_then(Value::as_string).as_deref(),
            Some(image.to_str().unwrap())
        );
        let clear = lines
            .iter()
            .map(|l| tebako_json::parse(l).unwrap())
            .find(|d| {
                d.find("detail")
                    .and_then(|dt| dt.find("action"))
                    .and_then(Value::as_string)
                    .as_deref()
                    == Some("clear")
            })
            .expect("the clear-all unmount event");
        assert_eq!(
            clear.find("path").and_then(Value::as_string).as_deref(),
            Some("")
        );
        assert!(
            clear
                .find("detail")
                .and_then(|d| d.find("count"))
                .and_then(Value::as_u64)
                == Some(1),
            "the clear reports its mount count: {clear:?}"
        );
        let passthrough = detail_of("open", "host");
        assert_eq!(
            passthrough
                .find("need")
                .and_then(Value::as_string)
                .as_deref(),
            Some("read")
        );
        let jail = detail_of("jail", "record");
        assert_eq!(
            jail.find("access").and_then(Value::as_string).as_deref(),
            Some("read")
        );
        let exec = detail_of("exec", "routed:");
        assert_eq!(
            exec.find("route").and_then(Value::as_string).as_deref(),
            Some("dlmap-closure")
        );
        // The spawn event is the exec row's grammar on the child-creating
        // surface — the op token is the only difference.
        let spawn = detail_of("spawn", "routed:");
        assert_eq!(
            spawn.find("route").and_then(Value::as_string).as_deref(),
            Some("dlmap-closure")
        );
        // The dlopen event carries the closure walk (schema §2): a
        // non-binary fixture parses no header — format null, deps [].
        let dl = detail_of("dlopen", "materialized:");
        let closure = dl.find("closure").expect("closure detail");
        assert!(matches!(closure.find("format"), Some(Value::Null)));
        assert!(matches!(closure.find("deps"), Some(Value::Array(d)) if d.is_empty()));
        // The dlmap redirect: the open verdict names the mount and the
        // details carry the tail + the materialized host path.
        let redirect = lines
            .iter()
            .map(|l| tebako_json::parse(l).unwrap())
            .find(|d| {
                d.find("op").and_then(Value::as_string).as_deref() == Some("open")
                    && d.find("detail")
                        .and_then(|dt| dt.find("dlmap_redirect"))
                        .and_then(Value::as_string)
                        .as_deref()
                        == Some("/tfs/data/secret.txt")
            })
            .expect("the dlmap-redirect open event");
        assert_eq!(
            redirect
                .find("verdict")
                .and_then(Value::as_string)
                .as_deref(),
            Some("image:/tfs")
        );
        assert_eq!(
            redirect
                .find("detail")
                .and_then(|d| d.find("materialized"))
                .and_then(Value::as_string)
                .as_deref(),
            Some(materialized.to_str().unwrap())
        );
    }
}
