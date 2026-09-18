//! tebako-term v2 — the fetch-plan surface (spec 06 §5a): one plan
//! header, one live line per concurrent worker, per-artifact phases
//! (download bar → verifying spinner → done), a plain-mode grammar of
//! start/done lines only. Thread-safe by construction: the fetch
//! pipeline's workers report through cloned handles over a shared
//! mutex; rendering stays on the caller's writer (stderr in products —
//! stdout belongs to the payload).
//!
//! The single-artifact [`crate::Progress`] renderer is unchanged; a
//! one-slot `ProgressSet` is its plan-shaped twin.

use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::{human_bytes, Mode};

/// The redraw budget for the WHOLE block (spec 06 §5a: ≤ 10 redraws/s,
/// per set, not per slot).
const REDRAW_INTERVAL: Duration = Duration::from_millis(100);

/// Bar width in cells.
const BAR_WIDTH: usize = 10;

/// Eighth-cell fractional blocks, low to high (`▏▎▍▌▋▊▉█`).
const BLOCKS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Braille spinner frames (unknown length, verifying phase).
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⦦', '⦧', '⦇', '⦏'];

/// EMA smoothing factor per redraw (spec 06 §5a: α = 0.3).
const EMA_ALPHA: f64 = 0.3;

/// ANSI colors — TTY mode only (plain mode is the NO_COLOR answer).
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

/// One slot's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Begun, downloading (bar or spinner).
    Downloading,
    /// Bytes staged; integrity/signature pass running (spinner).
    Verifying,
    /// Installed — the line stays until the plan finishes.
    Done,
    /// Failed — the plan's named error follows on a fresh line.
    Failed,
}

struct Slot {
    asset: String,
    phase: Phase,
    so_far: u64,
    total: Option<u64>,
    started: Instant,
    last_tick: Option<(Instant, u64)>,
    ema_rate: f64,
    ema_samples: u32,
    spinner: usize,
    /// Workers report out of plan order; a placeholder (an index a
    /// later slot's arrival created) never paints.
    begun: bool,
    /// The plain-mode start line prints once, at the first tick (the
    /// size is a transport fact — knowable only then) or at done.
    start_printed: bool,
    done_size: u64,
}

impl Slot {
    fn placeholder() -> Slot {
        Slot {
            asset: String::new(),
            phase: Phase::Downloading,
            so_far: 0,
            total: None,
            started: Instant::now(),
            last_tick: None,
            ema_rate: 0.0,
            ema_samples: 0,
            spinner: 0,
            begun: false,
            start_printed: false,
            done_size: 0,
        }
    }

    fn new(asset: &str, now: Instant) -> Slot {
        Slot {
            begun: true,
            ..Slot::placeholder()
        }
        .with(asset, now)
    }

    fn with(mut self, asset: &str, now: Instant) -> Slot {
        self.asset = asset.to_string();
        self.started = now;
        self
    }

    /// EMA-smoothed bytes/second (0 before the first sample).
    fn rate(&self) -> f64 {
        self.ema_rate
    }

    fn note_tick(&mut self, so_far: u64, total: Option<u64>, now: Instant) {
        if total.is_some() {
            self.total = total;
        }
        if let Some((t0, b0)) = self.last_tick {
            let dt = now.saturating_duration_since(t0).as_secs_f64();
            if dt > 0.0 && so_far >= b0 {
                let inst = (so_far - b0) as f64 / dt;
                self.ema_rate = if self.ema_samples == 0 {
                    inst
                } else {
                    EMA_ALPHA * inst + (1.0 - EMA_ALPHA) * self.ema_rate
                };
                self.ema_samples += 1;
            }
        }
        self.last_tick = Some((now, so_far));
        self.so_far = so_far;
        self.spinner += 1;
    }
}

struct Inner<W: Write> {
    out: W,
    mode: Mode,
    quiet: bool,
    title: String,
    artifacts: usize,
    total_hint: Option<u64>,
    begun: bool,
    finished: bool,
    slots: Vec<Slot>,
    /// How many terminal lines the live block currently occupies.
    painted: usize,
    last_draw: Option<Instant>,
    plan_started: Instant,
}

impl<W: Write> Inner<W> {
    fn write_str(&mut self, s: &str) {
        // Progress is best-effort by contract: a broken stderr must
        // never fail the run it decorates.
        let _ = self.out.write_all(s.as_bytes());
        let _ = self.out.flush();
    }

    /// Repaint the slot block in place (TTY mode). The cursor sits
    /// below the block between paints; `\x1b[<n>A` returns to its top.
    fn paint(&mut self) {
        if self.painted > 0 {
            self.write_str(&format!("\x1b[{}A", self.painted));
        }
        self.painted = 0;
        let frames: Vec<String> = self
            .slots
            .iter()
            .filter(|s| s.begun)
            .map(slot_frame)
            .collect();
        for frame in frames {
            self.write_str("\r\x1b[K");
            self.write_str(&frame);
            self.write_str("\n");
            self.painted += 1;
        }
    }

    /// Erase the block and leave the cursor where it started (a plain
    /// line — a phase, a done line, an error — prints above the block,
    /// which repaints on the next draw).
    fn unpaint(&mut self) {
        if self.painted > 0 {
            self.write_str(&format!("\x1b[{}A", self.painted));
            self.write_str("\x1b[J");
            self.painted = 0;
        }
    }

    fn draw_due(&self, now: Instant) -> bool {
        match self.last_draw {
            None => true,
            Some(t) => now.saturating_duration_since(t) >= REDRAW_INTERVAL,
        }
    }

    /// A TTY redraw honoring the block-wide throttle.
    fn draw_throttled(&mut self, now: Instant) {
        if self.mode != Mode::Tty || !self.draw_due(now) {
            return;
        }
        self.paint();
        self.last_draw = Some(now);
    }

    /// A state change that must become visible NOW (phase transitions
    /// are rare; the throttle budgets ticks, not lifecycle).
    fn draw_forced(&mut self, now: Instant) {
        if self.mode != Mode::Tty {
            return;
        }
        self.paint();
        self.last_draw = Some(now);
    }

    /// The plain-mode start line of a slot, once.
    fn print_start_line(&mut self, idx: usize) {
        if self.quiet || self.slots[idx].start_printed {
            return;
        }
        self.slots[idx].start_printed = true;
        let text = match self.slots[idx].total {
            Some(total) => format!(
                "downloading {} ({})",
                self.slots[idx].asset,
                human_bytes(total)
            ),
            None => format!("downloading {}", self.slots[idx].asset),
        };
        if self.mode == Mode::Tty {
            self.unpaint();
        }
        self.write_str(&text);
        self.write_str("\n");
    }
}

/// One slot line: colored per phase, fractional bar + EMA rate + ETA.
fn slot_frame(slot: &Slot) -> String {
    let (color, body) = match slot.phase {
        Phase::Downloading => (CYAN, download_frame(slot)),
        Phase::Verifying => {
            let c = SPINNER[slot.spinner % SPINNER.len()];
            (CYAN, format!("{}  {c} verifying", slot.asset))
        }
        Phase::Done => (
            GREEN,
            format!("✓ {} ({})", slot.asset, human_bytes(slot.done_size)),
        ),
        Phase::Failed => (RED, format!("✗ {}", slot.asset)),
    };
    format!("{color}{body}{RESET}")
}

/// `<asset>  [███▉      ] 42%  14.2/23.0 MB  3.1 MB/s  eta 12s` —
/// or the braille spinner + byte count when the length is unknown.
fn download_frame(slot: &Slot) -> String {
    match slot.total {
        Some(total) if total > 0 => {
            let done = slot.so_far.min(total);
            let pct = (done as f64 * 100.0 / total as f64).min(100.0);
            let bar = fractional_bar(done, total);
            let rate = slot.rate();
            let mut tail = format!(
                "{}  {}/s",
                crate::human_pair(done, total),
                human_bytes(rate as u64)
            );
            // The ETA shows once the EMA has converged past its warmup.
            if slot.ema_samples >= 2 && rate > 0.0 && done < total {
                let eta = (total - done) as f64 / rate;
                tail.push_str(&format!("  eta {}", human_duration(eta)));
            }
            format!("{}  {} {:>3.0}%  {}", slot.asset, bar, pct, tail)
        }
        _ => {
            let c = SPINNER[slot.spinner % SPINNER.len()];
            format!("{}  {c} {}", slot.asset, human_bytes(slot.so_far))
        }
    }
}

/// The fractional-block bar: `[███▉      ]` — eighth-cell resolution.
fn fractional_bar(done: u64, total: u64) -> String {
    let eighths = if total == 0 {
        0
    } else {
        (done.min(total) as u128 * (BAR_WIDTH as u128 * 8) / total as u128) as usize
    };
    let full = (eighths / 8).min(BAR_WIDTH);
    let part = eighths % 8;
    let mut bar = String::with_capacity(BAR_WIDTH);
    bar.push_str(&"█".repeat(full));
    if full < BAR_WIDTH {
        if part > 0 {
            bar.push(BLOCKS[part - 1]);
        }
        let used = full + usize::from(part > 0);
        bar.push_str(&" ".repeat(BAR_WIDTH - used));
    }
    format!("[{bar}]")
}

/// `12s`, `1m05s` — the ETA's whole-second rendering.
fn human_duration(secs: f64) -> String {
    let s = secs.round() as u64;
    if s < 60 {
        format!("{s}s")
    } else {
        format!("{}m{:02}s", s / 60, s % 60)
    }
}

/// The fetch-plan progress renderer (spec 06 §5a). Clone the handle for
/// each worker thread; every method takes `&self`.
pub struct ProgressSet<W: Write> {
    inner: Arc<Mutex<Inner<W>>>,
}

impl<W: Write> Clone for ProgressSet<W> {
    fn clone(&self) -> Self {
        ProgressSet {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl ProgressSet<io::Stderr> {
    /// A renderer over the real stderr; the mode is auto-detected and
    /// the quiet gate rides `TEBAKO_NO_PROGRESS` (spec 06 §5).
    pub fn stderr() -> ProgressSet<io::Stderr> {
        ProgressSet::with_mode_and_quiet(
            io::stderr(),
            crate::detect_mode(),
            match std::env::var_os("TEBAKO_NO_PROGRESS") {
                Some(v) => !v.is_empty() && v != "0",
                None => false,
            },
        )
    }
}

impl<W: Write> ProgressSet<W> {
    /// The injection seam: any writer, explicit tty flag (tests assert
    /// exact frames against a `Vec<u8>` sink — no pty, no global state).
    pub fn new(out: W, tty: bool) -> ProgressSet<W> {
        ProgressSet::with_mode(out, if tty { Mode::Tty } else { Mode::Plain })
    }

    /// A renderer over `out` in the given mode.
    pub fn with_mode(out: W, mode: Mode) -> ProgressSet<W> {
        ProgressSet::with_mode_and_quiet(out, mode, false)
    }

    /// A renderer over `out` with the quiet gate explicit.
    pub fn with_mode_and_quiet(out: W, mode: Mode, quiet: bool) -> ProgressSet<W> {
        ProgressSet {
            inner: Arc::new(Mutex::new(Inner {
                out,
                mode,
                quiet,
                title: String::new(),
                artifacts: 0,
                total_hint: None,
                begun: false,
                finished: false,
                slots: Vec::new(),
                painted: 0,
                last_draw: None,
                plan_started: Instant::now(),
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner<W>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The mode in effect.
    pub fn mode(&self) -> Mode {
        self.lock().mode
    }

    /// A snapshot of the sink (tests assert frames off it).
    pub fn snapshot(&self) -> W
    where
        W: Clone,
    {
        self.lock().out.clone()
    }

    /// Unwrap the writer when this handle is the last one (`Err(())`
    /// when other handles still share the sink).
    pub fn into_inner(self) -> Result<W, ()> {
        match Arc::try_unwrap(self.inner) {
            Ok(m) => Ok(m.into_inner().unwrap_or_else(|e| e.into_inner()).out),
            Err(_) => Err(()),
        }
    }

    /// The plan header — `fetching <title>: <N> artifacts, <total>
    /// total` — printed once, in both modes (quiet-gated).
    pub fn begin(&self, title: &str, artifacts: usize, total_hint: Option<u64>) {
        let mut inner = self.lock();
        inner.title = title.to_string();
        inner.artifacts = artifacts;
        inner.total_hint = total_hint;
        inner.begun = true;
        inner.plan_started = Instant::now();
        if inner.quiet {
            return;
        }
        let total = total_hint
            .map(human_bytes)
            .unwrap_or_else(|| "unknown".to_string());
        inner.write_str(&format!(
            "fetching {title}: {artifacts} artifacts, {total} total\n"
        ));
    }

    /// A transient phase line (`resolving <ref>`): TTY mode only —
    /// plain mode prints exactly the header + start/done lines.
    pub fn phase(&self, text: &str) {
        let mut inner = self.lock();
        if inner.mode != Mode::Tty {
            return;
        }
        inner.unpaint();
        inner.write_str(text);
        inner.write_str("\n");
    }

    /// A printed line in both modes, suppressed entirely by the quiet
    /// gate (tebako#400 — progress is informational, never results).
    pub fn line(&self, text: &str) {
        let mut inner = self.lock();
        if inner.quiet {
            return;
        }
        if inner.mode == Mode::Tty {
            inner.unpaint();
        }
        inner.write_str(text);
        inner.write_str("\n");
    }

    /// Arm a slot's download. Slot indexes are the caller's plan order;
    /// a re-begin (a retry) resets the slot to downloading.
    pub fn download_begin(&self, slot: usize, asset: &str) {
        self.download_begin_at(slot, asset, Instant::now());
    }

    /// [`ProgressSet::download_begin`] with an injected clock.
    pub fn download_begin_at(&self, slot: usize, asset: &str, now: Instant) {
        let mut inner = self.lock();
        while inner.slots.len() <= slot {
            inner.slots.push(Slot::placeholder());
        }
        inner.slots[slot] = Slot::new(asset, now);
        inner.draw_throttled(now);
    }

    /// One chunk arrived for `slot`. Throttled to the block budget.
    pub fn download_tick(&self, slot: usize, so_far: u64, total: Option<u64>) {
        self.download_tick_at(slot, so_far, total, Instant::now());
    }

    /// [`ProgressSet::download_tick`] with an injected clock.
    pub fn download_tick_at(&self, slot: usize, so_far: u64, total: Option<u64>, now: Instant) {
        let mut inner = self.lock();
        if slot >= inner.slots.len() || !inner.slots[slot].begun {
            return;
        }
        if inner.mode == Mode::Plain && !inner.slots[slot].start_printed {
            // The start line carries the size — a transport fact known
            // only at the first tick.
            if total.is_some() {
                inner.slots[slot].total = total;
            }
            inner.print_start_line(slot);
        }
        inner.slots[slot].note_tick(so_far, total, now);
        inner.draw_throttled(now);
    }

    /// The slot's bytes are staged; the integrity/signature pass runs
    /// (the verifying spinner).
    pub fn verifying(&self, slot: usize) {
        let mut inner = self.lock();
        if slot >= inner.slots.len() || !inner.slots[slot].begun {
            return;
        }
        if inner.mode == Mode::Plain {
            inner.print_start_line(slot);
        }
        inner.slots[slot].phase = Phase::Verifying;
        let now = Instant::now();
        inner.draw_forced(now);
    }

    /// The slot installed: `✓ <asset> (<size>)` in the block; in plain
    /// mode the done line `installed <asset> (<size>)` (or `text` when
    /// the caller carries a benefit line — spec 06 §5).
    pub fn installed(&self, slot: usize, size: u64, text: Option<&str>) {
        let mut inner = self.lock();
        if slot >= inner.slots.len() || !inner.slots[slot].begun {
            return;
        }
        inner.slots[slot].phase = Phase::Done;
        inner.slots[slot].done_size = size;
        match inner.mode {
            Mode::Plain => {
                inner.print_start_line(slot);
                if !inner.quiet {
                    let done = match text {
                        Some(t) => t.to_string(),
                        None => format!(
                            "installed {} ({})",
                            inner.slots[slot].asset,
                            human_bytes(size)
                        ),
                    };
                    inner.write_str(&done);
                    inner.write_str("\n");
                }
            }
            Mode::Tty => {
                let now = Instant::now();
                inner.draw_forced(now);
            }
        }
    }

    /// The slot failed: `✗ <asset>` in the block (TTY); plain mode
    /// prints nothing — the plan's named error is the report.
    pub fn failed(&self, slot: usize) {
        let mut inner = self.lock();
        if slot >= inner.slots.len() || !inner.slots[slot].begun {
            return;
        }
        if inner.mode == Mode::Plain {
            inner.print_start_line(slot);
        }
        inner.slots[slot].phase = Phase::Failed;
        let now = Instant::now();
        inner.draw_forced(now);
    }

    /// The plan completed: the block's final repaint (TTY), then the
    /// summary line `fetched <title>: <N> artifacts, <total> in <secs>s`
    /// (both modes, quiet-gated).
    pub fn finish(&self) {
        let mut inner = self.lock();
        if inner.finished {
            return;
        }
        inner.finished = true;
        if inner.mode == Mode::Tty {
            inner.paint();
        }
        inner.painted = 0;
        if !inner.quiet && inner.begun {
            let elapsed = inner.plan_started.elapsed().as_secs_f64();
            let total = inner
                .total_hint
                .map(human_bytes)
                .unwrap_or_else(|| "unknown".to_string());
            let summary = format!(
                "fetched {}: {} artifacts, {total} in {elapsed:.1}s\n",
                inner.title, inner.artifacts
            );
            inner.write_str(&summary);
        }
    }

    /// The plan aborted: close the block so the error body starts on a
    /// fresh line (error bodies are byte-stable). No summary line.
    pub fn abort(&self) {
        let mut inner = self.lock();
        if inner.finished {
            return;
        }
        inner.finished = true;
        if inner.mode == Mode::Tty {
            inner.paint();
        }
        inner.painted = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sink_text(set: ProgressSet<Vec<u8>>) -> String {
        String::from_utf8(set.snapshot()).unwrap()
    }

    #[test]
    fn fractional_bar_math() {
        assert_eq!(fractional_bar(0, 100), "[          ]");
        assert_eq!(fractional_bar(100, 100), "[██████████]");
        // 50% of 10 cells = 5 full cells
        assert_eq!(fractional_bar(50, 100), "[█████     ]");
        // 12.5% = 1 full + 0.25 cell (▎ = 2/8)
        assert_eq!(fractional_bar(125, 1000), "[█▎        ]");
        // 1/8 of a cell at 1.25%: part 1 → ▏
        assert_eq!(fractional_bar(125, 10000), "[▏         ]");
        // never overflows on done > total
        assert_eq!(fractional_bar(200, 100), "[██████████]");
    }

    #[test]
    fn human_duration_renders() {
        assert_eq!(human_duration(12.4), "12s");
        assert_eq!(human_duration(59.6), "1m00s");
        assert_eq!(human_duration(65.0), "1m05s");
    }

    #[test]
    fn plain_mode_grammar_is_header_start_done_summary() {
        let set = ProgressSet::new(Vec::new(), false);
        set.begin("runtime ruby 3.3.12", 2, Some(3000));
        set.download_begin(0, "exe");
        set.download_tick(0, 1000, Some(1000));
        set.verifying(0);
        set.installed(0, 1000, None);
        set.download_begin(1, "image.tfs");
        set.download_tick(1, 2000, Some(2000));
        set.installed(1, 2000, None);
        set.finish();
        let text = sink_text(set);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[0],
            "fetching runtime ruby 3.3.12: 2 artifacts, 2.9 KB total"
        );
        assert_eq!(lines[1], "downloading exe (1000 B)");
        assert_eq!(lines[2], "installed exe (1000 B)");
        assert_eq!(lines[3], "downloading image.tfs (2.0 KB)");
        assert_eq!(lines[4], "installed image.tfs (2.0 KB)");
        assert!(
            lines[5].starts_with("fetched runtime ruby 3.3.12: 2 artifacts, 2.9 KB in "),
            "{lines:?}"
        );
        assert!(lines[5].ends_with('s'), "{lines:?}");
        assert_eq!(lines.len(), 6, "{lines:?}");
        assert!(!text.contains("\x1b"), "{text:?}");
    }

    #[test]
    fn plain_mode_unknown_length_start_line_has_no_size() {
        let set = ProgressSet::new(Vec::new(), false);
        set.begin("payload tool", 1, None);
        set.download_begin(0, "tool.tfs");
        set.download_tick(0, 512, None);
        set.installed(0, 512, None);
        set.finish();
        let text = sink_text(set);
        assert!(text.contains("fetching payload tool: 1 artifacts, unknown total\n"));
        assert!(text.contains("downloading tool.tfs\n"), "{text}");
        assert!(text.contains("installed tool.tfs (512 B)\n"), "{text}");
    }

    #[test]
    fn quiet_gate_silences_the_whole_grammar() {
        let set = ProgressSet::with_mode_and_quiet(Vec::new(), Mode::Plain, true);
        set.begin("runtime ruby 3.3.12", 1, Some(1000));
        set.download_begin(0, "exe");
        set.download_tick(0, 1000, Some(1000));
        set.installed(0, 1000, None);
        set.finish();
        assert_eq!(sink_text(set), "");
    }

    #[test]
    fn tty_block_paints_and_repaints_in_place() {
        let set = ProgressSet::new(Vec::new(), true);
        let t0 = Instant::now();
        set.begin("runtime ruby 3.3.12", 2, Some(2000));
        set.download_begin_at(0, "exe", t0);
        set.download_begin_at(1, "image.tfs", t0);
        set.download_tick_at(0, 500, Some(1000), t0 + Duration::from_millis(150));
        set.download_tick_at(1, 1000, Some(1000), t0 + Duration::from_millis(300));
        let text = sink_text(set.clone());
        assert!(text.contains("fetching runtime ruby 3.3.12: 2 artifacts, 2.0 KB total\n"));
        // two slot lines painted per redraw, block rewinds with cursor-up
        assert!(text.contains("\x1b[2A"), "{text:?}");
        assert!(text.contains("[█████     ]  50%"), "{text:?}");
        assert!(text.contains("[██████████] 100%"), "{text:?}");
        set.installed(0, 1000, None);
        set.installed(1, 1000, None);
        set.finish();
        let text = sink_text(set);
        assert!(text.contains("\x1b[32m✓ exe (1000 B)\x1b[0m"), "{text:?}");
        assert!(text.contains("fetched runtime ruby 3.3.12: 2 artifacts, 2.0 KB in "));
    }

    #[test]
    fn tty_throttle_is_block_wide() {
        let set = ProgressSet::new(Vec::new(), true);
        let t0 = Instant::now();
        set.begin("p", 1, Some(100_000));
        set.download_begin_at(0, "a", t0);
        // 20 ticks inside one interval across slots: only the first draws.
        for i in 1..=20 {
            set.download_tick_at(0, i * 100, Some(100_000), t0 + Duration::from_millis(i * 4));
        }
        set.download_tick_at(0, 99_999, Some(100_000), t0 + Duration::from_millis(200));
        let text = sink_text(set);
        // two paints: the first tick's and the post-interval one
        assert_eq!(text.matches('\u{2588}').count() > 0, true);
        assert_eq!(text.matches("\r\x1b[K").count(), 2, "{text:?}");
    }

    #[test]
    fn phase_transitions_force_a_draw_inside_the_throttle_window() {
        let set = ProgressSet::new(Vec::new(), true);
        let t0 = Instant::now();
        set.begin("p", 1, Some(100));
        set.download_begin_at(0, "a", t0);
        set.download_tick_at(0, 100, Some(100), t0 + Duration::from_millis(10));
        set.verifying(0);
        set.installed(0, 100, None);
        let text = sink_text(set);
        assert!(text.contains("verifying"), "{text:?}");
        assert!(text.contains("✓ a (100 B)"), "{text:?}");
    }

    #[test]
    fn failed_slot_marks_red_and_abort_closes_the_block() {
        let set = ProgressSet::new(Vec::new(), true);
        let t0 = Instant::now();
        set.begin("p", 1, Some(100));
        set.download_begin_at(0, "a", t0);
        set.download_tick_at(0, 10, Some(100), t0 + Duration::from_millis(150));
        set.failed(0);
        set.abort();
        let text = sink_text(set);
        assert!(text.contains("\x1b[31m✗ a\x1b[0m"), "{text:?}");
        assert!(!text.contains("fetched p"), "{text:?}");
        assert!(text.ends_with('\n'), "{text:?}");
    }

    #[test]
    fn unknown_length_slot_spins_with_byte_count() {
        let set = ProgressSet::new(Vec::new(), true);
        let t0 = Instant::now();
        set.begin("p", 1, None);
        set.download_begin_at(0, "big.bin", t0);
        set.download_tick_at(0, 1536, None, t0 + Duration::from_millis(150));
        let text = sink_text(set);
        assert!(text.contains("big.bin  "), "{text:?}");
        assert!(text.contains("1.5 KB"), "{text:?}");
        assert!(!text.contains('%'), "{text}");
    }

    #[test]
    fn ema_rate_warms_up_and_eta_appears() {
        let set = ProgressSet::new(Vec::new(), true);
        let t0 = Instant::now();
        set.begin("p", 1, Some(10_000));
        set.download_begin_at(0, "a", t0);
        set.download_tick_at(0, 1000, Some(10_000), t0 + Duration::from_secs(1));
        // one sample → no ETA yet
        let text = sink_text(set.clone());
        assert!(!text.contains("eta"), "{text:?}");
        set.download_tick_at(0, 2000, Some(10_000), t0 + Duration::from_secs(2));
        set.download_tick_at(0, 3000, Some(10_000), t0 + Duration::from_secs(3));
        let text = sink_text(set);
        assert!(text.contains("1000 B/s"), "{text:?}");
        assert!(text.contains("eta 7s"), "{text:?}");
    }

    #[test]
    fn handles_are_thread_safe() {
        let set = ProgressSet::new(Vec::new(), false);
        set.begin("p", 4, None);
        let mut joins = Vec::new();
        for i in 0..4 {
            let h = set.clone();
            joins.push(std::thread::spawn(move || {
                h.download_begin(i, &format!("a{i}"));
                h.download_tick(i, 100, Some(100));
                h.installed(i, 100, None);
            }));
        }
        for j in joins {
            j.join().unwrap();
        }
        set.finish();
        let text = sink_text(set);
        for i in 0..4 {
            assert!(text.contains(&format!("installed a{i} (100 B)")), "{text}");
        }
    }
}
