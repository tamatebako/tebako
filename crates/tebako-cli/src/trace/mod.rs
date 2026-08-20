//! `tebako trace` — spec 25's observability front-ends. Phase T1 ships
//! `tebako trace run` (§4, discovery): the package runs under
//! `TEBAKO_JAIL=record` with the interception bus armed
//! (`TEBAKO_TRACE=<capture>`), and the capture is synthesized into a
//! SUGGESTED manifest fragment — commented YAML on stdout (or `--out`),
//! never applied (spec 25 law 7).
//!
//! The bus events come from the runtime driver's `tfs::trace` (the child
//! inherits both env vars through the bootstrap — no bootstrap or driver
//! changes); the jail channel feeds spec 23 §8's `needs:` generator
//! unchanged (`tfs::needs::needs_from_journal`), re-rendered from bus
//! events into the journal line grammar. The bus extends the draft to
//! the axes §8 does not cover: `materialize:` candidates (an in-image
//! file served through a raw host fd — the dlmap redirect and the
//! preload's fopen routing), closure-covered dlopen NOTEs, and
//! host-executable entrypoint notes.
//!
//! `cover` (§6, phase T3) ships in the `cover` submodule: the escapes
//! correlator, golden-parity with retrace-correlate on the shared
//! fixtures. `explain` (§5, phase T4) and the procmon converter
//! (`import`, §6.2 — the rest of T3) are later milestones; phase T2 was
//! the spawn/resolve emission (the preload's posix_spawn surface and the
//! driver's image-triple resolution — pure bus-side work, no consumer
//! change needed here beyond the pinned spawn grammar below).

pub mod cover;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tebako_json::Value;

use crate::error::{packaging_error, plain_error, TebakoError};

/// The parsed `tebako trace run` argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceRunArgs {
    /// The package file to execute under the trace.
    pub package: String,
    /// `--capture <path>`: where the bus writes the JSONL capture.
    /// Default: a per-run file in the temp dir (kept, named on stderr).
    pub capture: Option<PathBuf>,
    /// `--out <path>`: the draft's destination (default: stdout).
    pub out: Option<PathBuf>,
    /// Payload args, verbatim.
    pub args: Vec<String>,
}

/// Parse the `trace run` argv (the `run.rs` conventions: flags before
/// `--`, the first non-flag token starts the payload's argv).
pub fn parse_trace_run_args(args: &[String]) -> Result<TraceRunArgs, String> {
    const USAGE: &str =
        "usage: tebako trace run <pkg> [--capture <path>] [--out <path>] [--] [<args>...]";
    let Some(package) = args.first() else {
        return Err(USAGE.to_string());
    };
    if package.starts_with('-') {
        return Err(USAGE.to_string());
    }
    let mut capture = None;
    let mut out = None;
    let mut payload: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        match flag {
            "--" => {
                payload.extend_from_slice(&args[i + 1..]);
                break;
            }
            "--capture" | "--out" => {
                let value = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        args.get(i)
                            .cloned()
                            .ok_or_else(|| format!("option '{flag}' requires a value"))?
                    }
                };
                if flag == "--capture" {
                    capture = Some(PathBuf::from(value));
                } else {
                    out = Some(PathBuf::from(value));
                }
            }
            _ if flag.starts_with("--") => {
                return Err(format!(
                    "unknown trace run option '{flag}' (payload options ride after `--`)"
                ));
            }
            _ => {
                payload.extend_from_slice(&args[i..]);
                break;
            }
        }
        i += 1;
    }
    Ok(TraceRunArgs {
        package: package.clone(),
        capture,
        out,
        args: payload,
    })
}

/// The default capture path: one JSONL file per run in the temp dir.
fn default_capture() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("tebako-trace-{}-{nanos}.jsonl", std::process::id()))
}

/// One observation's evidence (spec 25 §4: every draft entry carries its
/// `why:` — `observed: <n> events, first at <ts>`).
#[derive(Debug, Clone)]
pub struct Observation {
    pub count: u64,
    pub first_ts: String,
}

impl Observation {
    fn record(&mut self, ts: &str) {
        if self.count == 0 {
            self.first_ts = ts.to_string();
        }
        self.count += 1;
    }

    fn why(&self) -> String {
        format!(
            "observed: {} events, first at {}",
            self.count, self.first_ts
        )
    }
}

/// The synthesized discovery result.
#[derive(Debug)]
pub struct Synthesis {
    /// Total well-formed events consumed from the capture.
    pub events: usize,
    /// The re-rendered §8 journal text fed to the needs generator.
    pub journal_text: String,
    /// The §8 needs draft (needs_from_journal's output, unchanged).
    pub needs_yaml: String,
    /// `materialize:` candidates: the in-image path → its evidence (an
    /// open-class event served the file through a raw host fd).
    pub materialize: BTreeMap<String, Observation>,
    /// Closure-covered dlopens (every declared dep resolved in-image):
    /// path → (dep count, evidence).
    pub closure_covered: BTreeMap<String, (usize, Observation)>,
    /// Host executables the run routed to: path → evidence.
    pub host_execs: BTreeMap<String, Observation>,
}

/// A host path component the exec-cache materialization machinery owns
/// (spec 25 §4's exclusion law extends §8's: the per-process dl/exec
/// tmpdirs are tebako's own scratch, never a payload need).
fn is_exec_cache_noise(path: &str) -> bool {
    Path::new(path).components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s.starts_with("tebako-dl-") || s.starts_with("tebako-exec-")
    })
}

/// Parse the capture and synthesize the discovery result. Pure: the
/// §8 substitution atoms, exclusions, and the exists probe are injected
/// so the unit suite never touches the environment or the filesystem.
/// Tolerant of the stream's realities (spec 25 §3): a trailing partial
/// line (a crashed tail) and mid-stream interleave damage are skipped,
/// never fatal.
pub fn synthesize(
    capture_text: &str,
    substitutions: &[(PathBuf, &str)],
    exclusions: &[PathBuf],
    exists: &dyn Fn(&str) -> bool,
) -> Synthesis {
    let mut events = 0usize;
    let mut journal_lines: Vec<String> = Vec::new();
    let mut materialize: BTreeMap<String, Observation> = BTreeMap::new();
    let mut closure_covered: BTreeMap<String, (usize, Observation)> = BTreeMap::new();
    let mut host_execs: BTreeMap<String, Observation> = BTreeMap::new();

    let lines: Vec<&str> = capture_text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(doc) = tebako_json::parse(line) else {
            // The trailing line may be a crashed tail's partial write;
            // anything else is interleave damage. Both are dropped.
            eprintln!(
                "tebako: trace: note: capture line {} is not a complete event — skipped",
                i + 1
            );
            continue;
        };
        events += 1;
        let op = doc
            .find("op")
            .and_then(Value::as_string)
            .unwrap_or_default();
        let path = doc
            .find("path")
            .and_then(Value::as_string)
            .unwrap_or_default();
        let verdict = doc
            .find("verdict")
            .and_then(Value::as_string)
            .unwrap_or_default();
        let ts = doc
            .find("ts")
            .and_then(Value::as_string)
            .unwrap_or_default();
        let detail = doc.find("detail");
        match op.as_str() {
            // The jail channel IS the §8 audit journal, formalized (law
            // 6): re-render the events into the journal line grammar and
            // feed the needs generator unchanged. A denial is an unmet
            // need — the generator's own rule.
            "jail" => {
                let event = match verdict.as_str() {
                    "record" => Some("jail-allow"),
                    v if v.starts_with("allow:") => Some("jail-allow"),
                    v if v.starts_with("deny:") => Some("jail-deny"),
                    _ => None,
                };
                let access = detail
                    .and_then(|d| d.find("access"))
                    .and_then(Value::as_string)
                    .unwrap_or_default();
                if let (Some(event), "read" | "write") = (event, access.as_str()) {
                    if !path.is_empty() && !is_exec_cache_noise(&path) {
                        journal_lines.push(format!(
                            "event={event} path={path} op={access} source=record"
                        ));
                    }
                }
            }
            // An open-class event that served the file through a raw host
            // fd (the dlmap redirect, or the preload's fopen routing) is
            // the §4 materialize-candidate signal: the consumer needed
            // real host bytes for a VFS-resident file.
            "open" => {
                let Some(materialized) = detail.and_then(|d| d.find("materialized")) else {
                    continue;
                };
                if materialized.as_string().is_none() {
                    continue;
                }
                // The in-image path the author declares: the redirect's
                // tail when the dlmap prefix was spelled, else the path
                // as dispatched (the fopen surface).
                let candidate = detail
                    .and_then(|d| d.find("dlmap_redirect"))
                    .and_then(Value::as_string)
                    .unwrap_or(path.clone());
                materialize
                    .entry(candidate)
                    .or_insert(Observation {
                        count: 0,
                        first_ts: String::new(),
                    })
                    .record(&ts);
            }
            // An in-image dlopen whose closure walk resolved every dep
            // in-image needs no declaration — a NOTE, not an entry.
            "dlopen" => {
                if !verdict.starts_with("materialized:") {
                    continue;
                }
                let deps = detail
                    .and_then(|d| d.find("closure"))
                    .and_then(|c| c.find("deps"));
                let Some(Value::Array(deps)) = deps else {
                    continue;
                };
                if deps.is_empty() {
                    continue;
                }
                let all_in_image = deps.iter().all(|dep| {
                    dep.find("verdict").and_then(Value::as_string).as_deref()
                        == Some("materialized")
                });
                if all_in_image {
                    let entry = closure_covered.entry(path.clone()).or_insert((
                        deps.len(),
                        Observation {
                            count: 0,
                            first_ts: String::new(),
                        },
                    ));
                    entry.1.record(&ts);
                }
            }
            // A host-absolute exec the runtime routed OUT of the image:
            // the entrypoint/runtime-dep note. The spawn surface (T2:
            // the preload's posix_spawn path) reports the same grammar
            // and joins exec here.
            "exec" | "spawn" => {
                if verdict == "host" && Path::new(&path).is_absolute() {
                    host_execs
                        .entry(path.clone())
                        .or_insert(Observation {
                            count: 0,
                            first_ts: String::new(),
                        })
                        .record(&ts);
                }
            }
            _ => {}
        }
    }

    let mut journal_text = journal_lines.join("\n");
    if !journal_text.is_empty() {
        journal_text.push('\n');
    }
    let needs_yaml =
        tfs::needs::needs_from_journal(&journal_text, substitutions, exclusions, exists);
    Synthesis {
        events,
        journal_text,
        needs_yaml,
        materialize,
        closure_covered,
        host_execs,
    }
}

/// Double-quoted YAML scalar escaping (needs.rs's rule).
fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Render the suggested manifest fragment (commented YAML — a draft for
/// review, never applied, spec 25 law 7).
pub fn render_draft(package: &str, capture: &Path, synthesis: &Synthesis) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Suggested manifest additions for {package} — `tebako trace run` (spec 25 §4, discovery).\n\
         # {} interception event(s) observed; the capture is {}\n\
         # REVIEW BEFORE MERGING: a generated suggestion never edits a manifest by itself\n\
         # (spec 25 law 7). Flip ro/rw, delete noise, fill every `why`.\n",
        synthesis.events,
        capture.display()
    ));
    if synthesis.events == 0 {
        out.push_str(
            "# No interception events were captured — the run touched nothing declarable\n\
             # (or the package is not a tebako v2 composition: nothing mounted the bus).\n",
        );
    }
    out.push_str(&synthesis.needs_yaml);
    if !synthesis.materialize.is_empty() {
        out.push_str(
            "# Files the run consumed through a RAW host fd (a native library's own\n\
             # stdio/loader read below the interposition — the exec-cache answer):\n\
             materialize:\n",
        );
        for (path, obs) in &synthesis.materialize {
            out.push_str(&format!(
                "  - path: \"{}\"\n    why: \"TODO — read through a materialized host copy ({})\"\n",
                yaml_escape(path),
                obs.why()
            ));
        }
    }
    for (path, (deps, obs)) in &synthesis.closure_covered {
        out.push_str(&format!(
            "# NOTE: closure-covered — nothing to declare: \"{}\" ({deps} deps, all in-image; {})\n",
            path,
            obs.why()
        ));
    }
    for (path, obs) in &synthesis.host_execs {
        out.push_str(&format!(
            "# NOTE: host executable observed: \"{}\" — an entrypoint/runtime-dep candidate ({})\n",
            path,
            obs.why()
        ));
    }
    out
}

/// `tebako trace run <pkg> [--capture <path>] [--out <path>] [--] [args...]`.
///
/// Runs the package under `TEBAKO_JAIL=record` with the bus armed, then
/// synthesizes the capture into the suggested-manifest draft. The process
/// exits with the PAYLOAD's exit code (the run's own verdict is the
/// payload's — the draft lands either way; spec 25 law 1: observability
/// never gates).
pub fn trace_run(parsed: &TraceRunArgs) -> Result<(), TebakoError> {
    let program = PathBuf::from(&parsed.package);
    if !program.is_file() {
        return Err(packaging_error(
            127,
            Some(&format!("package not found: {}", parsed.package)),
        ));
    }
    let capture = parsed.capture.clone().unwrap_or_else(default_capture);

    // Spawn + wait on BOTH platforms (the synthesis runs after the
    // payload, so the unix execve replacement of `tebako run` is not
    // available here). stdio is inherited: the payload runs as if direct.
    let status = std::process::Command::new(&program)
        .args(&parsed.args)
        .env(tfs::trace::TRACE_ENV, &capture)
        .env("TEBAKO_JAIL", "record")
        .status()
        .map_err(|e| {
            plain_error(format!(
                "cannot execute the package {}: {e}",
                program.display()
            ))
        })?;
    let code = status.code().unwrap_or(1);

    // The capture read is tolerant (law 1): a missing/unreadable channel
    // degrades to a note and a no-events draft.
    let capture_text = match std::fs::read_to_string(&capture) {
        Ok(text) => text,
        Err(e) => {
            eprintln!(
                "tebako: trace: note: no capture at {} ({e}) — drafting with zero events",
                capture.display()
            );
            String::new()
        }
    };

    // The §8 generator's inputs (the tfs-cli needs-from-journal set):
    // $HOME/$TMPDIR substitution atoms, the platform floor + tebako home
    // + the exec cache as exclusions, canonicalize as the exists probe.
    let mut substitutions: Vec<(PathBuf, &str)> = Vec::new();
    for (var, atom) in [("HOME", "$HOME"), ("TMPDIR", "$TMPDIR")] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                substitutions.push((PathBuf::from(v), atom));
            }
        }
    }
    let mut exclusions = tfs::policy::platform_floor();
    if let Some(home) = tfs::journal::tebako_home_dir() {
        exclusions.push(home);
    }
    if let Ok(cache) = std::env::var("TEBAKO_EXEC_CACHE") {
        if !cache.is_empty() {
            exclusions.push(PathBuf::from(cache));
        }
    }

    let synthesis = synthesize(&capture_text, &substitutions, &exclusions, &|p| {
        std::fs::canonicalize(p).is_ok()
    });
    let draft = render_draft(&parsed.package, &capture, &synthesis);
    match &parsed.out {
        Some(out) => {
            std::fs::write(out, &draft)
                .map_err(|e| plain_error(format!("cannot write {}: {e}", out.display())))?;
            eprintln!("tebako: trace: draft written to {}", out.display());
        }
        None => print!("{draft}"),
    }
    eprintln!("tebako: trace: the capture is {}", capture.display());
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_flags_and_payload_split() {
        let p = parse_trace_run_args(&args(&[
            "pkg",
            "--capture",
            "/tmp/c.jsonl",
            "--out=/tmp/draft.yaml",
            "--",
            "-x",
            "in.csv",
        ]))
        .unwrap();
        assert_eq!(p.package, "pkg");
        assert_eq!(p.capture, Some(PathBuf::from("/tmp/c.jsonl")));
        assert_eq!(p.out, Some(PathBuf::from("/tmp/draft.yaml")));
        assert_eq!(p.args, vec!["-x".to_string(), "in.csv".to_string()]);

        // No `--`: the first non-flag token starts the payload argv.
        let p = parse_trace_run_args(&args(&["pkg", "input.csv", "--verbose"])).unwrap();
        assert!(p.capture.is_none() && p.out.is_none());
        assert_eq!(
            p.args,
            vec!["input.csv".to_string(), "--verbose".to_string()]
        );

        // Errors.
        assert!(parse_trace_run_args(&args(&[])).is_err());
        assert!(parse_trace_run_args(&args(&["--capture", "x"])).is_err());
        assert!(parse_trace_run_args(&args(&["pkg", "--capture"])).is_err());
        assert!(parse_trace_run_args(&args(&["pkg", "--frobnicate"])).is_err());
    }

    /// A bus event line, the tfs::trace render shape.
    fn event(op: &str, path: &str, verdict: &str, ts: &str, detail: &str) -> String {
        format!(
            "{{\"v\":1,\"ts\":\"{ts}\",\"pid\":1,\"tid\":1,\"op\":\"{op}\",\"path\":\"{path}\",\"verdict\":\"{verdict}\",\"detail\":{detail},\"dur_us\":3}}"
        )
    }

    /// A host-absolute executable path for the platform (the host_execs
    /// gate is `Path::is_absolute`; forward slashes keep the JSON simple).
    const HOST_EXE: &str = if cfg!(windows) {
        "C:/Windows/system32/findstr.exe"
    } else {
        "/usr/bin/git"
    };

    /// A second host executable, reached via the spawn surface (T2).
    const HOST_SPAWN: &str = if cfg!(windows) {
        "C:/Windows/system32/where.exe"
    } else {
        "/usr/bin/curl"
    };

    #[test]
    fn synthesize_feeds_the_needs_generator_and_names_candidates() {
        let capture = [
            // The jail channel: two reads and a write of the same host
            // path, one read of another, and exec-cache noise to drop.
            event("jail", "/work/data", "record", "2026-08-19T01:00:01.000000Z", "{\"access\":\"read\"}"),
            event("jail", "/work/data", "record", "2026-08-19T01:00:02.000000Z", "{\"access\":\"read\"}"),
            event("jail", "/work/data", "record", "2026-08-19T01:00:03.000000Z", "{\"access\":\"write\"}"),
            event("jail", "/etc/custom.conf", "record", "2026-08-19T01:00:04.000000Z", "{\"access\":\"read\"}"),
            event("jail", "/tmp/tebako-dl-abc/tfs/lib/x.so", "record", "2026-08-19T01:00:05.000000Z", "{\"access\":\"read\"}"),
            // A jail deny is an unmet need (the generator's own rule).
            event("jail", "/secret", "deny:user", "2026-08-19T01:00:06.000000Z", "{\"access\":\"read\"}"),
            // The dlmap redirect: an in-image file served via a raw fd.
            event("open", "/tmp/tebako-dl-abc/tfs/data/font.png", "image:/tfs", "2026-08-19T01:00:07.000000Z",
                  "{\"dlmap_redirect\":\"/tfs/data/font.png\",\"materialized\":\"/tmp/tebako-dl-abc/tfs/data/font.png\"}"),
            // The fopen surface: path as dispatched + the host copy.
            event("open", "/tfs/data/font.png", "image:/tfs", "2026-08-19T01:00:08.000000Z",
                  "{\"materialized\":\"/tmp/tebako-dl-abc/tfs/data/font.png\"}"),
            // The closure-covered dlopen (two deps, both in-image).
            event("dlopen", "/tfs/lib/libsass.so", "materialized:/tmp/tebako-dl-abc/tfs/lib/libsass.so", "2026-08-19T01:00:09.000000Z",
                  "{\"closure\":{\"format\":\"elf\",\"deps\":[{\"name\":\"libc.so\",\"resolved\":\"/tfs/lib/libc.so\",\"verdict\":\"materialized\"},{\"name\":\"libm.so\",\"resolved\":\"/tfs/lib/libm.so\",\"verdict\":\"materialized\"}]}}"),
            // A dlopen with a host-system dep: no closure NOTE.
            event("dlopen", "/tfs/lib/libmixed.so", "materialized:/tmp/tebako-dl-abc/tfs/lib/libmixed.so", "2026-08-19T01:00:10.000000Z",
                  "{\"closure\":{\"format\":\"elf\",\"deps\":[{\"name\":\"libSystem.dylib\",\"resolved\":null,\"verdict\":\"host-system\"}]}}"),
            // A host-absolute exec (the entrypoint note) and an in-image
            // one (no note). The T2 spawn surface joins exec here: same
            // grammar, same note.
            event("exec", HOST_EXE, "host", "2026-08-19T01:00:11.000000Z", "{}"),
            event("exec", "/tfs/bin/tool", "routed:/tmp/tebako-exec-home-1/bin/tool", "2026-08-19T01:00:12.000000Z", "{\"route\":\"home-tree\"}"),
            event("spawn", HOST_SPAWN, "host", "2026-08-19T01:00:13.000000Z", "{}"),
            event("spawn", "/tfs/bin/tool", "routed:/tmp/tebako-exec-home-1/bin/tool", "2026-08-19T01:00:14.000000Z", "{\"route\":\"home-tree\"}"),
            // A crashed tail: the partial last line is dropped, not fatal.
            "{\"v\":1,\"ts\":\"2026-08-19T01:00:15".to_string(),
        ]
        .join("\n");
        let s = synthesize(&capture, &[], &[], &|_| true);
        assert_eq!(s.events, 14, "14 well-formed events of 15 lines");
        // The needs feed: /work/data collapses to one rw entry
        // (strongest-observed-op wins), the deny rides along as ro, the
        // exec-cache noise never reaches it.
        assert!(
            s.journal_text
                .contains("event=jail-allow path=/work/data op=write"),
            "{}",
            s.journal_text
        );
        assert!(
            s.journal_text
                .contains("event=jail-deny path=/secret op=read"),
            "{}",
            s.journal_text
        );
        assert!(!s.journal_text.contains("tebako-dl-"), "{}", s.journal_text);
        assert!(
            s.needs_yaml.contains("path: \"/work/data\""),
            "{}",
            s.needs_yaml
        );
        assert!(s.needs_yaml.contains("access: rw"), "{}", s.needs_yaml);
        assert!(
            s.needs_yaml.contains("observed: 2 read, 1 write"),
            "{}",
            s.needs_yaml
        );
        assert!(
            s.needs_yaml.contains("path: \"/etc/custom.conf\""),
            "{}",
            s.needs_yaml
        );
        assert!(
            s.needs_yaml.contains("path: \"/secret\""),
            "{}",
            s.needs_yaml
        );
        // The materialize candidate is the IN-IMAGE path (the redirect's
        // tail; the fopen surface's dispatched path names the same file),
        // deduped with both events as evidence.
        assert_eq!(s.materialize.len(), 1);
        let obs = &s.materialize["/tfs/data/font.png"];
        assert_eq!(obs.count, 2);
        assert_eq!(obs.first_ts, "2026-08-19T01:00:07.000000Z");
        // The closure NOTE: only the all-in-image dlopen.
        assert_eq!(s.closure_covered.len(), 1);
        let (deps, obs) = &s.closure_covered["/tfs/lib/libsass.so"];
        assert_eq!(*deps, 2);
        assert_eq!(obs.count, 1);
        // The host-exec notes: the exec surface and the spawn surface
        // each contributed one.
        assert_eq!(s.host_execs.len(), 2);
        assert!(s.host_execs.contains_key(HOST_EXE));
        assert!(s.host_execs.contains_key(HOST_SPAWN));
    }

    #[test]
    fn synthesize_tolerates_empty_and_substitutes_atoms() {
        // Empty capture: the needs block keeps the §8 empty spelling.
        let s = synthesize("", &[], &[], &|_| true);
        assert_eq!(s.events, 0);
        assert!(s.needs_yaml.contains("host: []"), "{}", s.needs_yaml);

        // Substitutions re-spell the needs paths; the exclusion drops.
        let home = PathBuf::from("/home/u");
        let capture = event(
            "jail",
            "/home/u/.config/app",
            "record",
            "2026-08-19T01:00:01.000000Z",
            "{\"access\":\"read\"}",
        ) + "\n"
            + &event(
                "jail",
                "/sys/kernel",
                "record",
                "2026-08-19T01:00:02.000000Z",
                "{\"access\":\"read\"}",
            );
        let s = synthesize(
            &capture,
            &[(home, "$HOME")],
            &[PathBuf::from("/sys")],
            &|_| true,
        );
        assert!(
            s.needs_yaml.contains("path: \"$HOME/.config/app\""),
            "{}",
            s.needs_yaml
        );
        assert!(!s.needs_yaml.contains("/sys/kernel"), "{}", s.needs_yaml);
    }

    #[test]
    fn render_draft_carries_the_review_contract() {
        let capture = [
            event("jail", "/work/data", "record", "2026-08-19T01:00:01.000000Z", "{\"access\":\"read\"}"),
            event("open", "/tfs/data/font.png", "image:/tfs", "2026-08-19T01:00:02.000000Z",
                  "{\"materialized\":\"/tmp/tebako-dl-abc/tfs/data/font.png\"}"),
            event("dlopen", "/tfs/lib/libx.so", "materialized:/tmp/tebako-dl-abc/tfs/lib/libx.so", "2026-08-19T01:00:03.000000Z",
                  "{\"closure\":{\"format\":\"macho\",\"deps\":[{\"name\":\"liby.dylib\",\"resolved\":\"/tfs/lib/liby.dylib\",\"verdict\":\"materialized\"}]}}"),
            event("exec", HOST_EXE, "host", "2026-08-19T01:00:04.000000Z", "{}"),
        ]
        .join("\n");
        let s = synthesize(&capture, &[], &[], &|_| true);
        let draft = render_draft("pkg", Path::new("/tmp/capture.jsonl"), &s);
        assert!(draft.contains("spec 25 §4"), "{draft}");
        assert!(
            draft.contains("never edits a manifest by itself"),
            "{draft}"
        );
        assert!(draft.contains("4 interception event(s)"), "{draft}");
        assert!(draft.contains("needs:"), "{draft}");
        assert!(draft.contains("path: \"/work/data\""), "{draft}");
        assert!(draft.contains("materialize:"), "{draft}");
        assert!(draft.contains("path: \"/tfs/data/font.png\""), "{draft}");
        assert!(
            draft.contains("observed: 1 events, first at 2026-08-19T01:00:02.000000Z"),
            "{draft}"
        );
        assert!(
            draft.contains("# NOTE: closure-covered — nothing to declare: \"/tfs/lib/libx.so\" (1 deps, all in-image"),
            "{draft}"
        );
        assert!(
            draft.contains(&format!("# NOTE: host executable observed: \"{HOST_EXE}\"")),
            "{draft}"
        );

        // The zero-event capture: the no-events comment, no sections.
        let s = synthesize("", &[], &[], &|_| true);
        let draft = render_draft("pkg", Path::new("/tmp/capture.jsonl"), &s);
        assert!(
            draft.contains("No interception events were captured"),
            "{draft}"
        );
        assert!(!draft.contains("materialize:"), "{draft}");
        assert!(!draft.contains("# NOTE:"), "{draft}");
    }
}
