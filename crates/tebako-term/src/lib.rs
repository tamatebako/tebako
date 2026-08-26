//! tebako-term — terminal progress for the tebako stack (spec 06 §5, locked).
//!
//! The progress contract:
//! - full rendering iff stderr is a TTY and `TERM != dumb`; opt-outs
//!   `NO_COLOR` (present at all) and `TEBAKO_NO_PROGRESS` (set, non-empty,
//!   not "0") force the plain mode;
//! - plain mode (pipe, CI): exactly the start + done single
//!   lines, no terminal control sequences;
//! - `TEBAKO_NO_PROGRESS` additionally QUIETS every [`Progress::line`]
//!   output (the downloading/installed/cache-hit lines) — progress is
//!   informational, never results (tebako#400);
//! - TTY mode: phase lines plus a hand-rolled ANSI bar
//!   `[=====>    ] 62%  14.2/23.0 MB  3.1 MB/s` throttled to
//!   ≤ 10 redraws/s; unknown content-length → spinner frames + byte count;
//! - everything goes to the writer the caller hands in (the bootstrap
//!   uses stderr — stdout belongs to the payload).
//!
//! Zero dependencies, no async, no unsafe: TTY detection is std's
//! [`IsTerminal`] (isatty underneath). Tests inject any [`Write`] sink
//! plus an explicit mode — no global state, no pty required; the redraw
//! throttle takes an injectable clock ([`Progress::download_tick_at`]).

use std::ffi::OsString;
use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

/// The redraw budget: the bar repaints at most once per this interval
/// (spec 06 §5: ≤ 10 redraws/s).
const REDRAW_INTERVAL: Duration = Duration::from_millis(100);

/// Bar width in cells (`[=====>    ]` is 10).
const BAR_WIDTH: usize = 10;

/// Spinner frames for unknown-length downloads.
const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

/// Erase-from-cursor-to-end-of-line after a carriage return: the bar
/// redraws in place without leaking stale characters.
const REDRAW: &str = "\r\x1b[K";

/// The rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// stderr is a terminal and no opt-out is set: bar + phase lines.
    Tty,
    /// pipe/CI/opt-out: start + done lines only, no control sequences.
    Plain,
}

/// The mode for the real stderr (production entry point).
pub fn detect_mode() -> Mode {
    detect_mode_with(io::stderr().is_terminal(), |name| std::env::var_os(name))
}

/// The pure core of [`detect_mode`] — tests pass the tty flag and an env
/// lookup explicitly, so nothing touches process state.
pub fn detect_mode_with(tty: bool, env: impl Fn(&str) -> Option<OsString>) -> Mode {
    let term_ok = match env("TERM") {
        Some(v) => v != "dumb",
        None => true,
    };
    if tty && term_ok && env("NO_COLOR").is_none() && !no_progress_env(&env) {
        Mode::Tty
    } else {
        Mode::Plain
    }
}

/// The `TEBAKO_NO_PROGRESS` opt-out predicate: set, non-empty, not `"0"`.
/// Shared by mode detection and the quiet gate on [`Progress::line`].
fn no_progress_env(env: &impl Fn(&str) -> Option<OsString>) -> bool {
    match env("TEBAKO_NO_PROGRESS") {
        Some(v) => !v.is_empty() && v != "0",
        None => false,
    }
}

/// Human byte count: `181 B`, `3.1 MB`, `23.0 MB` — one decimal past 1 KB,
/// binary (1024) units.
pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// The `14.2/23.0 MB` pair: both sides in the total's unit.
fn human_pair(done: u64, total: u64) -> String {
    if total < 1024 {
        return format!("{done}/{total} B");
    }
    const DIVS: [f64; 4] = [1024.0, 1048576.0, 1073741824.0, 1099511627776.0];
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    let mut i = 0;
    while i + 1 < DIVS.len() && total as f64 >= DIVS[i + 1] {
        i += 1;
    }
    format!(
        "{:.1}/{:.1} {}",
        done as f64 / DIVS[i],
        total as f64 / DIVS[i],
        UNITS[i]
    )
}

/// One bar frame: `[=====>    ] 62%  14.2/23.0 MB  3.1 MB/s`.
fn bar_frame(so_far: u64, total: u64, elapsed: Duration) -> String {
    let pct = ((so_far.min(total) * 100) / total) as usize;
    let mut bar = String::with_capacity(BAR_WIDTH);
    if pct >= 100 {
        bar.push_str(&"=".repeat(BAR_WIDTH));
    } else {
        // The '>' head replaces the cell after the completed ones.
        let done = pct * (BAR_WIDTH - 1) / 100;
        bar.push_str(&"=".repeat(done));
        bar.push('>');
        bar.push_str(&" ".repeat(BAR_WIDTH - done - 1));
    }
    let secs = elapsed.as_secs_f64().max(0.001);
    let rate = human_bytes((so_far as f64 / secs) as u64);
    format!(
        "[{bar}] {pct}%  {}  {rate}/s",
        human_pair(so_far.min(total), total)
    )
}

/// A progress renderer over any writer. Construct with [`Progress::stderr`]
/// in production or [`Progress::new`] (sink + explicit tty flag) in tests.
pub struct Progress<W: Write> {
    out: W,
    mode: Mode,
    /// `TEBAKO_NO_PROGRESS` gate: when set, [`Progress::line`] prints
    /// nothing at all (tebako#400 — the cache-hit and installed lines
    /// were unconditional noise on every invocation).
    quiet: bool,
    asset: String,
    header_printed: bool,
    line_open: bool,
    started: Instant,
    last_draw: Option<Instant>,
    last_so_far: u64,
    last_total: Option<u64>,
    drawn_so_far: u64,
    spinner: usize,
}

impl Progress<io::Stderr> {
    /// A renderer over the real stderr; the mode is auto-detected
    /// ([`detect_mode`]) and the quiet gate rides `TEBAKO_NO_PROGRESS`.
    pub fn stderr() -> Progress<io::Stderr> {
        Progress::with_mode_and_quiet(
            io::stderr(),
            detect_mode(),
            no_progress_env(&|name| std::env::var_os(name)),
        )
    }
}

impl<W: Write> Progress<W> {
    /// The injection seam: any writer, explicit tty flag (tests assert
    /// exact frames against a `Vec<u8>` sink — no pty, no global state).
    pub fn new(out: W, tty: bool) -> Progress<W> {
        Progress::with_mode(out, if tty { Mode::Tty } else { Mode::Plain })
    }

    /// A renderer over `out` in the given mode.
    pub fn with_mode(out: W, mode: Mode) -> Progress<W> {
        Progress::with_mode_and_quiet(out, mode, false)
    }

    /// A renderer over `out` with the quiet gate explicit.
    pub fn with_mode_and_quiet(out: W, mode: Mode, quiet: bool) -> Progress<W> {
        Progress {
            out,
            mode,
            quiet,
            asset: String::new(),
            header_printed: false,
            line_open: false,
            started: Instant::now(),
            last_draw: None,
            last_so_far: 0,
            last_total: None,
            drawn_so_far: 0,
            spinner: 0,
        }
    }

    /// The mode in effect.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Unwrap the writer (tests extract the sink for assertions).
    pub fn into_inner(self) -> W {
        self.out
    }

    fn write_str(&mut self, s: &str) {
        // Progress is best-effort by contract: a broken stderr must never
        // fail the run it decorates.
        let _ = self.out.write_all(s.as_bytes());
        let _ = self.out.flush();
    }

    /// Close an in-place bar line (if any) so plain text starts clean.
    fn close_line(&mut self) {
        if self.line_open {
            self.write_str("\n");
            self.line_open = false;
        }
    }

    /// A transient phase line (`resolving <ref>`, `verifying sha256`,
    /// `installing (locked)`): TTY mode only — plain mode prints exactly
    /// the start + done lines.
    pub fn phase(&mut self, text: &str) {
        if self.mode != Mode::Tty {
            return;
        }
        self.close_line();
        self.write_str(text);
        self.write_str("\n");
    }

    /// A printed line (the `downloading` start line, the `installed …`
    /// benefit line, the quiet cache-hit line) — in both modes, but
    /// suppressed entirely when the quiet gate is set
    /// (`TEBAKO_NO_PROGRESS=1`, tebako#400).
    pub fn line(&mut self, text: &str) {
        if self.quiet {
            return;
        }
        self.close_line();
        self.write_str(text);
        self.write_str("\n");
    }

    /// Arm a download: the `downloading <asset> (<size>)` header prints at
    /// the first tick (the size is a transport fact — content-length — so
    /// it is only knowable then).
    pub fn download_begin(&mut self, asset: &str) {
        self.download_begin_at(asset, Instant::now());
    }

    /// [`Progress::download_begin`] with an injected clock (throttle tests).
    pub fn download_begin_at(&mut self, asset: &str, now: Instant) {
        self.close_line();
        self.asset.clear();
        self.asset.push_str(asset);
        self.header_printed = false;
        self.started = now;
        self.last_draw = None;
        self.last_so_far = 0;
        self.last_total = None;
        self.drawn_so_far = 0;
        self.spinner = 0;
    }

    /// One chunk arrived: `so_far` bytes of `total` (None = unknown
    /// content-length → spinner + byte count). Throttled to ≤ 10
    /// redraws/s; the first frame always draws.
    pub fn download_tick(&mut self, so_far: u64, total: Option<u64>) {
        self.download_tick_at(so_far, total, Instant::now());
    }

    /// [`Progress::download_tick`] with an injected clock (throttle tests).
    pub fn download_tick_at(&mut self, so_far: u64, total: Option<u64>, now: Instant) {
        if total.is_some() {
            self.last_total = total;
        }
        self.last_so_far = so_far;
        if !self.header_printed {
            self.print_header();
            self.header_printed = true;
        }
        if self.mode != Mode::Tty {
            return;
        }
        let due = match self.last_draw {
            None => true,
            Some(t) => now.saturating_duration_since(t) >= REDRAW_INTERVAL,
        };
        if due {
            self.draw(now);
        }
    }

    /// The download finished: commit a final (complete) frame if the last
    /// drawn one is stale, then close the bar line. Plain mode already
    /// printed its start line; there is nothing more to do.
    pub fn download_end(&mut self) {
        if !self.header_printed {
            self.print_header();
            self.header_printed = true;
        }
        if self.mode != Mode::Tty {
            return;
        }
        if self.line_open && self.drawn_so_far != self.last_so_far {
            self.draw(Instant::now());
        }
        self.close_line();
    }

    /// The download failed: close any open bar line so the error body
    /// starts on a fresh line (error bodies are byte-stable).
    pub fn download_abort(&mut self) {
        self.close_line();
    }

    fn print_header(&mut self) {
        let text = match self.last_total {
            Some(total) => format!("downloading {} ({})", self.asset, human_bytes(total)),
            None => format!("downloading {}", self.asset),
        };
        self.line(&text);
    }

    fn draw(&mut self, now: Instant) {
        let frame = match self.last_total {
            Some(total) if total > 0 => bar_frame(
                self.last_so_far,
                total,
                now.saturating_duration_since(self.started),
            ),
            _ => {
                let c = SPINNER[self.spinner % SPINNER.len()];
                format!("{c} {}", human_bytes(self.last_so_far))
            }
        };
        self.spinner += 1;
        self.write_str(REDRAW);
        self.write_str(&frame);
        self.line_open = true;
        self.last_draw = Some(now);
        self.drawn_so_far = self.last_so_far;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), OsString::from(v)))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn sink_text(p: Progress<Vec<u8>>) -> String {
        String::from_utf8(p.into_inner()).unwrap()
    }

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(181), "181 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(1048576), "1.0 MB");
        assert_eq!(human_bytes(14889779), "14.2 MB");
        assert_eq!(human_bytes(24117248), "23.0 MB");
        assert_eq!(human_bytes(24191976), "23.1 MB");
        assert_eq!(human_bytes(1073741824), "1.0 GB");
        assert_eq!(human_bytes(26214400), "25.0 MB");
    }

    #[test]
    fn detect_mode_matrix() {
        let plain = env_of(&[]);
        // not a tty → plain regardless of env
        assert_eq!(detect_mode_with(false, &plain), Mode::Plain);
        // a capable terminal → tty
        assert_eq!(
            detect_mode_with(true, env_of(&[("TERM", "xterm-256color")])),
            Mode::Tty
        );
        // TERM unset still qualifies (only TERM=dumb is excluded)
        assert_eq!(detect_mode_with(true, &plain), Mode::Tty);
        // TERM=dumb → plain
        assert_eq!(
            detect_mode_with(true, env_of(&[("TERM", "dumb")])),
            Mode::Plain
        );
        // NO_COLOR present (any value, even empty) → plain
        assert_eq!(
            detect_mode_with(true, env_of(&[("NO_COLOR", "1")])),
            Mode::Plain
        );
        assert_eq!(
            detect_mode_with(true, env_of(&[("NO_COLOR", "")])),
            Mode::Plain
        );
        // TEBAKO_NO_PROGRESS=1 → plain; "0"/empty do not opt out
        assert_eq!(
            detect_mode_with(true, env_of(&[("TEBAKO_NO_PROGRESS", "1")])),
            Mode::Plain
        );
        assert_eq!(
            detect_mode_with(true, env_of(&[("TEBAKO_NO_PROGRESS", "0")])),
            Mode::Tty
        );
        assert_eq!(
            detect_mode_with(true, env_of(&[("TEBAKO_NO_PROGRESS", "")])),
            Mode::Tty
        );
        // opt-outs cannot rescue a non-tty
        assert_eq!(
            detect_mode_with(false, env_of(&[("TEBAKO_NO_PROGRESS", "1")])),
            Mode::Plain
        );
    }

    #[test]
    fn bar_first_frame_exact() {
        // 15 MiB of 24_119_976 B (23.0 MiB) in 5.0 s → 65%, 3.0 MB/s.
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.download_begin_at("asset.bin", t0);
        p.download_tick_at(15_728_640, Some(24_119_976), t0 + Duration::from_secs(5));
        assert_eq!(
            sink_text(p),
            "downloading asset.bin (23.0 MB)\n\r\x1b[K[=====>    ] 65%  15.0/23.0 MB  3.0 MB/s"
        );
    }

    #[test]
    fn bar_hundred_percent_is_all_equals() {
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.download_begin_at("a", t0);
        p.download_tick_at(1000, Some(1000), t0 + Duration::from_secs(2));
        let text = sink_text(p);
        assert!(text.contains("[==========] 100%"), "{text}");
    }

    #[test]
    fn throttle_caps_redraws_at_ten_per_second() {
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.download_begin_at("a", t0);
        // 20 ticks inside one interval: only the first draws.
        for i in 1..=20 {
            p.download_tick_at(i * 100, Some(100_000), t0 + Duration::from_millis(i * 4));
        }
        // one more tick a full interval after the first draw: a second frame.
        p.download_tick_at(99_999, Some(100_000), t0 + Duration::from_millis(200));
        let text = sink_text(p);
        assert_eq!(text.matches(REDRAW).count(), 2, "{text:?}");
        assert!(text.contains("99%"), "{text}");
    }

    #[test]
    fn end_commits_a_final_complete_frame_once() {
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.download_begin_at("a", t0);
        p.download_tick_at(100, Some(100), t0 + Duration::from_secs(1));
        p.download_end();
        let text = sink_text(p);
        // first frame already showed 100% → no redraw, just the newline.
        assert_eq!(text.matches(REDRAW).count(), 1, "{text:?}");
        assert!(text.ends_with("100%  100/100 B  100 B/s\n"), "{text:?}");
    }

    #[test]
    fn end_repaints_when_the_last_frame_is_stale() {
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.download_begin_at("a", t0);
        p.download_tick_at(10, Some(100), t0);
        // a second tick inside the throttle window is skipped …
        p.download_tick_at(100, Some(100), t0 + Duration::from_millis(50));
        // … but end() must commit the completion frame.
        p.download_end();
        let text = sink_text(p);
        assert_eq!(text.matches(REDRAW).count(), 2, "{text:?}");
        assert!(text.contains("[==========] 100%"), "{text}");
        assert!(text.ends_with('\n'), "{text:?}");
    }

    #[test]
    fn unknown_length_spins_and_counts_bytes() {
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.download_begin_at("big.bin", t0);
        p.download_tick_at(1536, None, t0);
        p.download_tick_at(26214400, None, t0 + Duration::from_millis(100));
        let text = sink_text(p);
        assert_eq!(
            text,
            "downloading big.bin\n\r\x1b[K| 1.5 KB\r\x1b[K/ 25.0 MB"
        );
        // no percent/bar without a length
        assert!(!text.contains('%'), "{text}");
    }

    #[test]
    fn plain_mode_prints_exactly_start_and_done() {
        let mut p = Progress::new(Vec::new(), false);
        p.phase("resolving ruby@3.3.7;tebako=9.9.9");
        p.download_begin("tebako-runtime-9.9.9-3.3.7-macos-arm64");
        for i in 1..=10 {
            p.download_tick(i * 100, Some(1000));
        }
        p.download_end();
        p.phase("verifying sha256");
        p.phase("installing (locked)");
        p.line("installed ruby-3.3.7-9.9.9-macos-arm64 (1000 B) — cached at /x and shared by every tebako app on this machine");
        assert_eq!(
            sink_text(p),
            "downloading tebako-runtime-9.9.9-3.3.7-macos-arm64 (1000 B)\n\
             installed ruby-3.3.7-9.9.9-macos-arm64 (1000 B) — cached at /x and shared by every tebako app on this machine\n"
        );
    }

    #[test]
    fn empty_download_still_prints_the_start_line() {
        let mut p = Progress::new(Vec::new(), false);
        p.download_begin("empty");
        p.download_end(); // no tick ever arrived (0-byte body)
        assert_eq!(sink_text(p), "downloading empty\n");
    }

    #[test]
    fn tty_flow_is_the_spec_phase_sequence() {
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.phase("resolving ruby@3.3.7;tebako=9.9.9");
        p.download_begin_at("asset", t0);
        p.download_tick_at(500, Some(1000), t0 + Duration::from_secs(1));
        p.download_tick_at(1000, Some(1000), t0 + Duration::from_secs(2));
        p.download_end();
        p.phase("verifying sha256");
        p.phase("installing (locked)");
        p.line("installed entry (1.0 KB) — cached at /home/u/.tebako/runtimes/entry and shared by every tebako app on this machine");
        assert_eq!(
            sink_text(p),
            "resolving ruby@3.3.7;tebako=9.9.9\n\
             downloading asset (1000 B)\n\
             \r\x1b[K[====>     ] 50%  500/1000 B  500 B/s\
             \r\x1b[K[==========] 100%  1000/1000 B  500 B/s\n\
             verifying sha256\n\
             installing (locked)\n\
             installed entry (1.0 KB) — cached at /home/u/.tebako/runtimes/entry and shared by every tebako app on this machine\n"
        );
    }

    #[test]
    fn abort_closes_the_bar_line() {
        let mut p = Progress::new(Vec::new(), true);
        let t0 = Instant::now();
        p.download_begin_at("a", t0);
        p.download_tick_at(10, Some(100), t0 + Duration::from_secs(1));
        p.download_abort();
        p.line("tebako-bootstrap: boom");
        let text = sink_text(p);
        assert!(
            text.ends_with("10 B/s\ntebako-bootstrap: boom\n"),
            "{text:?}"
        );
    }

    #[test]
    fn quiet_lines_work_in_both_modes() {
        for tty in [true, false] {
            let mut p = Progress::new(Vec::new(), tty);
            p.line("runtime ruby-3.3.7 (cached)");
            assert_eq!(sink_text(p), "runtime ruby-3.3.7 (cached)\n");
        }
    }

    #[test]
    fn quiet_gate_suppresses_lines_and_the_download_header() {
        // tebako#400: the cache-hit / installed / downloading header lines
        // are progress, not results — the quiet gate silences them all.
        for mode in [Mode::Tty, Mode::Plain] {
            let mut p = Progress::with_mode_and_quiet(Vec::new(), mode, true);
            p.line("runtime ruby-3.3.7 (cached)");
            p.download_begin("asset");
            p.download_tick_at(500, Some(1000), Instant::now());
            p.download_end();
            p.line("installed entry (1.0 KB)");
            let text = sink_text(p);
            assert!(!text.contains("downloading asset"), "{text:?}");
            assert!(!text.contains("(cached)"), "{text:?}");
            assert!(!text.contains("installed entry"), "{text:?}");
            if mode == Mode::Plain {
                assert_eq!(text, "");
            }
        }
    }
}
