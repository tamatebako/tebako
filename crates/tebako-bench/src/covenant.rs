//! The benchmark covenant (arc 07): per-axis budgets, evaluated against a
//! run's metrics map.
//!
//! The covenant is the difference between a number that is published and a
//! number that is *owed*: a cell names a statistic, the comparison it must
//! satisfy, and the bound, and a release fails when the run it produced
//! breaches one. The copy on the website is the same data, so the page and
//! the gate cannot drift.
//!
//! Three verdicts, and only one of them fails:
//!
//! * `pass` — the measured value satisfies the comparison;
//! * `over` — it does not, and the verdict carries both numbers;
//! * `absent` — the run produced no sample for that cell. That is a
//!   coverage gap, not a regression: it is reported loudly and does not
//!   fail the release, because a platform without a number must show the
//!   gap rather than be silently satisfied or falsely red.
//!
//! A gate that cannot be tripped on purpose cannot be trusted, so the
//! evaluator honours `TEBAKO_BENCH_INJECT=<cell-or-axis>:<factor>`: the
//! measured value of every matching cell is multiplied before comparison.
//! The dry run that proves the gate works is one cell at `1.10`.
//!
//! The fleet witness (tebako#1071, spec 27 §11.1): the producer attaches
//! each sourced cell's witness to the metrics it emits — the leg the
//! worst-of aggregate kept, the reference (native) arm's drift against
//! its own rolling published median, the pair delta (the current paired
//! ratio against ITS rolling median), and the leg's runner image
//! identity. When a cell trips, the verdict names the class the witness
//! implies: `fleet-move` (native drift large, pair delta small),
//! `regression` (the tebako arm moved against a flat native arm),
//! `mechanical` (the bound, not the pair, moved — neither arm left its
//! healthy band, or the leg carries zero published entries and the trip
//! is the bound's first fleet calibration), or `ambiguous` (both arms
//! moved divergently) — adjudication is a read of the artifact, not an
//! archaeology dig (the #1067 trip took three runs and a manual
//! image-history dig; v2.8.85's three trips all landed in the
//! then-unnamed mechanical bucket).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::BenchError;

/// The authored covenant document (`benchmarks/covenant.yaml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovenantFile {
    pub schema_version: u32,
    pub cells: Vec<Cell>,
}

/// One budgeted cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// The metric id the run must produce (dotted: `axis.statistic`).
    pub id: String,
    /// The bound, in the cell's unit.
    pub bound: f64,
    /// The comparison the measured value must satisfy.
    pub comparison: Comparison,
    /// What the number is. Documentation for the reader of a breach.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Why this bound is the bound. Required reading for a challenge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// How the run produces this cell's metric (spec 27 §11's covenant
    /// arm). Every cell the suites produce carries one; a cell whose
    /// source selects nothing answers `absent` from a bench run, the
    /// honest coverage gap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CellSource>,
}

/// How one covenant cell's metric is produced from a run's dashboard —
/// the producer's selection + derivation. The dashboard is the published
/// unit, so the covenant judges the published statistic, never a private
/// recomputation (spec 27 §7's stats are the produced set: median is the
/// headline, min the cross-noise-comparable figure on shared runners).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellSource {
    /// The workload-id suffix selectors: a dashboard cell matches when
    /// its workload id ENDS WITH any of these (`-boot` selects
    /// `ruby-boot`, `python-boot`, `java-boot`). Never empty.
    pub workloads: Vec<String>,
    /// The arm's target prefix (`tebako-` selects `tebako-ruby`,
    /// `tebako-python`, …). The baseline arm is read off the dashboard's
    /// declared baseline, never re-declared here — unless `baseline`
    /// overrides it.
    pub target: String,
    /// The baseline arm's target prefix OVERRIDE (spec 27 §11.1, amended
    /// 2026-11-21): ratio/speedup divide by the cell matching this
    /// prefix on the same triplet × workload × mode instead of the
    /// dashboard's declared baseline — the materialize cell divides the
    /// tebako arm by the harness's `extract-` arm, which is no suite's
    /// declared baseline. Rides ratio|speedup only (a check error
    /// otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
    /// The mode the cell measures.
    pub mode: crate::result::RunMode,
    /// The measure the cell rides.
    pub measure: Measure,
    /// The shape the measure is reported in.
    pub shape: Shape,
    /// The aggregation across the selected dashboard cells.
    pub aggregate: Aggregate,
}

/// The measure a sourced cell rides (the produced per-cell fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Measure {
    /// The median wall time (the headline stat).
    Wall,
    /// The median peak RSS, in bytes (file-backed-inclusive per spec 27
    /// §7's footer — the field is already the normalized byte count).
    Rss,
}

/// The shape the measure is reported in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// The overhead ratio: the cell's value ÷ the baseline's on the same
    /// triplet × workload × mode (the trend feed's `ratio_vs_baseline` —
    /// 1.00 is parity, above it is overhead).
    Ratio,
    /// The speedup shape: the baseline's value ÷ the cell's (the
    /// dashboard's published `speedup_vs_v1`).
    Speedup,
    /// The absolute measure, as produced.
    Absolute,
    /// The teardown delta (spec 27 §11.1, amended 2026-11-21): the cell's
    /// median wall minus its median first-output time — the exit tail a
    /// finished dispatch shows, on the same dashboard cell. Rides
    /// `measure: wall` only (a check error otherwise); divides by
    /// nothing, so it takes no `baseline` override. A cell whose runs
    /// produced no output carries no first-output median and contributes
    /// nothing.
    #[serde(rename = "teardown-delta")]
    TeardownDelta,
}

/// The aggregation across the dashboard cells one source selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aggregate {
    /// The worst cell — the gate is as honest as its worst leg. "Worst"
    /// is comparison-direction-aware at evaluation: the max for a `le`
    /// bound, the min for a `ge` bound.
    Worst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Comparison {
    /// measured ≤ bound (a latency, a cost, a size).
    Le,
    /// measured ≥ bound (a throughput, a speedup).
    Ge,
}

impl Comparison {
    fn holds(self, measured: f64, bound: f64) -> bool {
        match self {
            Comparison::Le => measured <= bound,
            Comparison::Ge => measured >= bound,
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Comparison::Le => "<=",
            Comparison::Ge => ">=",
        }
    }
}

/// The metrics a run produced: a flat `id -> value` map plus the release it
/// belongs to. Deliberately flat — the covenant owns the taxonomy, the run
/// owns the numbers, and neither reaches into the other's structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsFile {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
    pub metrics: BTreeMap<String, f64>,
    /// The fleet witness per sourced cell (additive, tebako#1071): the
    /// producer derives it from the same dashboard selection the value
    /// came from, plus the rolling bench-history medians when the
    /// workflow hands them over. Keyed by the cell id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub witness: BTreeMap<String, CellWitness>,
}

/// One sourced cell's fleet witness (tebako#1071): the leg the aggregate
/// kept, the reference arm's drift and the pair delta against the
/// rolling published medians, and the leg's runner image identity. Both
/// drift numbers are ratios against the cell's own rolling history:
/// 1.00 = flat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellWitness {
    /// The leg (triplet × workload × target × mode) whose derived value
    /// the aggregate kept — the leg that trips the gate on a breach.
    pub triplet: String,
    pub workload: String,
    pub target: String,
    pub mode: crate::result::RunMode,
    /// The reference arm the witness reads: the cell's baseline (the
    /// `source.baseline` override when declared, else the dashboard's
    /// declared baseline) — the native on-system arm for every
    /// comparison cell of the runtime suite. Absent when the cell's
    /// workload ran no reference arm (the author loop's v2-tools cells).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// The reference arm's current median ÷ its rolling median over the
    /// last ≤ `WITNESS_WINDOW` published runs — 1.00 = the native arm is
    /// flat against its own history. Null with `note` when unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_drift: Option<f64>,
    /// The current paired ratio (the dashboard's per-index
    /// `paired_ratio_vs_baseline`, falling back to the ratio of medians)
    /// ÷ its rolling median over the same window — 1.00 = the two arms'
    /// relationship is unchanged whatever the fleet did. Null with
    /// `note` when unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pair_delta: Option<f64>,
    /// The leg's runner image identity (captured per leg in the result
    /// document's runner block); absent off CI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_version: Option<String>,
    /// The leg's image identity differs from the newest history entry
    /// carrying one for the same suite × triplet; absent when either
    /// side lacks the identity (off CI, or history predating the field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_changed: Option<bool>,
    /// How many entries the published trend feed carries for the kept
    /// leg itself (suite × triplet × workload × target × mode), counted
    /// before any rolling median is attempted — 0 means the run that
    /// produced this witness is the cell's FIRST fleet measurement and
    /// any trip on it is the bound's first calibration (the
    /// `mechanical` class's structured input; additive, v2.8.85's
    /// adjudication). Absent when no feed was handed over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_entries: Option<u32>,
    /// Why a drift is null — never a silent null (invariant 9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The classification a tripped cell earns from its witness
/// (tebako#1071): the gate names the class so adjudication is a read of
/// the artifact, not an archaeology dig.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    /// Native-arm drift large + pair delta small: both arms rode the same
    /// shift — a runner image / infra move, not a regression.
    FleetMove,
    /// The tebako arm moved against a flat native arm — a real regression.
    Regression,
    /// The bound, not the pair, is what moved: neither arm left its
    /// healthy band (a tight or miscalibrated bound), or the cell has no
    /// published history at all (the trip is the bound's first fleet
    /// calibration — the run that trips IS the calibration the bound
    /// awaited). Adjudicate the bound, not the product.
    Mechanical,
    /// Both arms moved divergently — a human adjudicates (the witness
    /// names the leg and its image).
    Ambiguous,
}

impl Class {
    pub fn token(self) -> &'static str {
        match self {
            Class::FleetMove => "fleet-move",
            Class::Regression => "regression",
            Class::Mechanical => "mechanical",
            Class::Ambiguous => "ambiguous",
        }
    }
}

/// The witness window: the rolling medians read the last ≤8 published
/// entries for the cell's leg (the trend feed keeps 20 releases).
pub const WITNESS_WINDOW: usize = 8;
/// Below this many history entries the rolling median is noise — the
/// drift fields degrade to null with the note naming the count.
pub const WITNESS_MIN_HISTORY: usize = 3;
/// The moved/flat band: a drift or delta outside [1/1.30, 1.30] counts
/// as MOVED. Set from the healthy fleet's observed spread — the
/// on-system statloop legs drift up to ×1.27 week over week and the
/// published pair ratios up to ×1.19 (bench-history through
/// 2026-11-15), while the tebako#1067 fleet move read ×0.59 on the
/// native arm and ×2.24 on the pair.
pub const WITNESS_TOLERANCE: f64 = 1.30;

/// The classification rule (spec 27 §11.1). The first-calibration case
/// reads the witness's structured history count before any band math: a
/// leg with ZERO published entries trips on a bound that was authored
/// before fleet numbers — the run that trips is the calibration the
/// bound awaited (`mechanical`). Otherwise both witness numbers read
/// against the tolerance band. None when the trip is genuinely
/// unreadable — a degraded witness with history on record (1–2 entries,
/// no feed, no reference arm) — and the note says why.
pub fn classify(
    native_drift: Option<f64>,
    pair_delta: Option<f64>,
    history_entries: Option<u32>,
) -> Option<(Class, String)> {
    if history_entries == Some(0) {
        return Some((
            Class::Mechanical,
            "no published history for this cell's leg — the bound was authored before fleet numbers and this run is its first calibration; adjudicate the bound, not the product".to_string(),
        ));
    }
    let (drift, delta) = (native_drift?, pair_delta?);
    let moved = |x: f64| !(1.0 / WITNESS_TOLERANCE..=WITNESS_TOLERANCE).contains(&x);
    Some(match (moved(drift), moved(delta)) {
        (true, false) => (
            Class::FleetMove,
            format!(
                "native-arm drift large (×{drift:.2}) with the pair delta small (×{delta:.2}) — fleet move (runner image/infra), not a regression"
            ),
        ),
        (false, true) => (
            Class::Regression,
            format!(
                "the tebako arm moved (pair delta ×{delta:.2}) against a flat native arm (×{drift:.2}) — regression"
            ),
        ),
        (true, true) => (
            Class::Ambiguous,
            format!(
                "both arms moved divergently (native ×{drift:.2}, pair ×{delta:.2}) — ambiguous; adjudicate"
            ),
        ),
        (false, false) => (
            Class::Mechanical,
            format!(
                "neither witness moved (native ×{drift:.2}, pair ×{delta:.2}) — the bound sits inside the pair's healthy band: a tight or miscalibrated bound, not a fleet move and not a regression"
            ),
        ),
    })
}

/// One cell's verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellVerdict {
    pub id: String,
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured: Option<f64>,
    pub bound: f64,
    pub comparison: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injected: Option<f64>,
    /// The fleet witness (additive, tebako#1071): the native arm's drift
    /// and the pair delta against the cell's rolling history (1.00 =
    /// flat), flattened here for the gate's read; the full record (the
    /// leg, the reference arm, the image identity) rides `witness`.
    /// Null when the producer could not derive them — `witness.note`
    /// says why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_drift: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pair_delta: Option<f64>,
    /// The trip's class (fleet-move | regression | mechanical |
    /// ambiguous), set on `over` cells whose witness supports a reading
    /// (both drift numbers, or the zero-history first calibration).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<Class>,
    /// The classification sentence, as the report text prints it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_note: Option<String>,
    /// The cell's full witness record (the leg the aggregate kept, the
    /// reference arm, the runner image identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness: Option<CellWitness>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Pass,
    Over,
    Absent,
}

/// The whole run's verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
    pub verdict: &'static str,
    pub cells: Vec<CellVerdict>,
}

impl Report {
    /// True when nothing breached — `absent` cells do not fail.
    pub fn passes(&self) -> bool {
        !self.cells.iter().any(|c| c.state == State::Over)
    }
}

impl CovenantFile {
    pub fn from_yaml(text: &str) -> Result<Self, BenchError> {
        let doc: Self = serde_yml::from_str(text)
            .map_err(|e| BenchError::operational(format!("covenant is not valid YAML: {e}")))?;
        doc.check()?;
        Ok(doc)
    }

    pub fn load(path: &Path) -> Result<Self, BenchError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| BenchError::operational(format!("cannot read {}: {e}", path.display())))?;
        Self::from_yaml(&text)
    }

    /// The structural rules a hand-edited covenant can break.
    pub fn check(&self) -> Result<(), BenchError> {
        if self.schema_version != 1 {
            return Err(BenchError::operational(format!(
                "covenant schema_version {} is not 1",
                self.schema_version
            )));
        }
        if self.cells.is_empty() {
            return Err(BenchError::operational(
                "covenant declares no cells — a gate with no budgets gates nothing",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for c in &self.cells {
            if c.id.is_empty() || c.id.contains(' ') {
                return Err(BenchError::operational(format!(
                    "covenant cell id {:?} is empty or contains a space",
                    c.id
                )));
            }
            if !seen.insert(c.id.clone()) {
                return Err(BenchError::operational(format!(
                    "covenant cell id {:?} is declared twice",
                    c.id
                )));
            }
            if !c.bound.is_finite() {
                return Err(BenchError::operational(format!(
                    "covenant cell {:?} has a non-finite bound",
                    c.id
                )));
            }
            if let Some(source) = &c.source {
                if source.workloads.is_empty() || source.workloads.iter().any(|w| w.is_empty()) {
                    return Err(BenchError::operational(format!(
                        "covenant cell {:?} declares a source with no workload selector — a source that selects nothing produces nothing",
                        c.id
                    )));
                }
                if source.target.is_empty() {
                    return Err(BenchError::operational(format!(
                        "covenant cell {:?} declares a source with an empty target prefix",
                        c.id
                    )));
                }
                if source.shape == Shape::TeardownDelta && source.measure != Measure::Wall {
                    return Err(BenchError::operational(format!(
                        "covenant cell {:?} declares teardown-delta on a non-wall measure — the shape is wall minus first-output",
                        c.id
                    )));
                }
                if source.baseline.is_some()
                    && !matches!(source.shape, Shape::Ratio | Shape::Speedup)
                {
                    return Err(BenchError::operational(format!(
                        "covenant cell {:?} declares a baseline override on a shape that divides by nothing — it rides ratio|speedup only",
                        c.id
                    )));
                }
            }
        }
        Ok(())
    }
}

impl MetricsFile {
    pub fn from_json(text: &str) -> Result<Self, BenchError> {
        let doc: Self = serde_json::from_str(text)
            .map_err(|e| BenchError::operational(format!("metrics are not valid JSON: {e}")))?;
        Ok(doc)
    }

    /// Merge another run's metrics into this one (a release runs several
    /// suites; each contributes its cells). Later values win, and the merge
    /// is recorded so a duplicated id is visible rather than silent. The
    /// witness map follows the same law (tebako#1071).
    pub fn merge(&mut self, other: &MetricsFile) -> Vec<String> {
        let mut collisions = Vec::new();
        for (k, v) in &other.metrics {
            if self.metrics.insert(k.clone(), *v).is_some() {
                collisions.push(k.clone());
            }
        }
        for (k, v) in &other.witness {
            self.witness.insert(k.clone(), v.clone());
        }
        if self.release.is_none() {
            self.release = other.release.clone();
        }
        collisions
    }
}

/// `<cell-or-axis>:<factor>` — the trip proof's injection.
pub fn parse_inject(spec: &str) -> Result<(String, f64), BenchError> {
    let (key, factor) = spec.rsplit_once(':').ok_or_else(|| {
        BenchError::operational(format!(
            "TEBAKO_BENCH_INJECT={spec:?} is not <cell-or-axis>:<factor>"
        ))
    })?;
    let factor: f64 = factor.parse().map_err(|_| {
        BenchError::operational(format!(
            "TEBAKO_BENCH_INJECT={spec:?}: the factor is not a number"
        ))
    })?;
    if !(factor.is_finite() && factor > 0.0) {
        return Err(BenchError::operational(format!(
            "TEBAKO_BENCH_INJECT={spec:?}: the factor must be finite and positive"
        )));
    }
    Ok((key.to_string(), factor))
}

/// Evaluate every cell. `inject` multiplies the measured value of each cell
/// whose id equals the key or begins with `key + "."` (so `filesystem`
/// reaches `filesystem.stat.p95`). Each cell's verdict also carries the
/// witness the producer attached to the metrics (tebako#1071): the drift
/// numbers flat, the full record under `witness`, and on `over` cells the
/// trip's classification (spec 27 §11.1).
pub fn evaluate(
    covenant: &CovenantFile,
    metrics: &MetricsFile,
    inject: Option<(&str, f64)>,
) -> Report {
    let cells = covenant
        .cells
        .iter()
        .map(|cell| {
            let measured = metrics.metrics.get(&cell.id).copied();
            let (value, injected) = match (measured, inject) {
                (Some(v), Some((key, factor))) if matches_inject(&cell.id, key) => {
                    (Some(v * factor), Some(v))
                }
                _ => (measured, None),
            };
            let state = match value {
                None => State::Absent,
                Some(v) if cell.comparison.holds(v, cell.bound) => State::Pass,
                Some(_) => State::Over,
            };
            let witness = metrics.witness.get(&cell.id).cloned();
            let (native_drift, pair_delta) = witness
                .as_ref()
                .map(|w| (w.native_drift, w.pair_delta))
                .unwrap_or((None, None));
            let history_entries = witness.as_ref().and_then(|w| w.history_entries);
            let (class, class_note) = match state {
                State::Over => classify(native_drift, pair_delta, history_entries)
                    .map(|(c, note)| (Some(c), Some(note)))
                    .unwrap_or((None, None)),
                _ => (None, None),
            };
            CellVerdict {
                id: cell.id.clone(),
                state,
                measured: value,
                bound: cell.bound,
                comparison: cell.comparison.token().to_string(),
                injected,
                native_drift,
                pair_delta,
                class,
                class_note,
                witness,
            }
        })
        .collect::<Vec<_>>();
    let verdict = if cells.iter().any(|c| c.state == State::Over) {
        "fail"
    } else {
        "pass"
    };
    Report {
        schema_version: 1,
        release: metrics.release.clone(),
        verdict,
        cells,
    }
}

fn matches_inject(id: &str, key: &str) -> bool {
    id == key || (id.len() > key.len() && id.starts_with(key) && id.as_bytes()[key.len()] == b'.')
}

// ---------------------------------------------------------------------
// The producer (spec 27 §11's covenant arm): derive each sourced cell's
// metric from the run's dashboards.
// ---------------------------------------------------------------------

/// The measure a dashboard cell contributes.
fn measure_of(cell: &crate::report::DashboardCell, measure: Measure) -> f64 {
    match measure {
        Measure::Wall => cell.median_wall_s,
        Measure::Rss => cell.median_peak_rss_bytes as f64,
    }
}

/// One selected dashboard cell's contribution: the derived value plus
/// the provenance the fleet witness reads (tebako#1071) — which leg it
/// came from, the reference arm (the cell's baseline) it compares
/// against, and the paired inputs the witness's pair delta needs.
struct Candidate {
    value: f64,
    suite: String,
    triplet: String,
    workload: String,
    target: String,
    mode: crate::result::RunMode,
    /// The reference arm's concrete target id + its current median for
    /// the cell's measure; None when the workload ran no reference arm.
    reference: Option<(String, f64)>,
    /// The published paired per-iteration ratio (None → the witness
    /// falls back to the ratio of medians).
    paired_ratio: Option<f64>,
    /// The selected cell's own median for the measure.
    cell_median: f64,
}

/// The cell's reference arm for one selected dashboard cell (the
/// witness's "native arm"): the `source.baseline` override when
/// declared (exact then prefix, the §11.1 rule), else the dashboard's
/// declared baseline. None when no such arm produced a cell.
fn reference_cell<'d>(
    source: &CellSource,
    dash: &'d crate::report::Dashboard,
    t: &'d crate::report::DashboardTriplet,
    c: &crate::report::DashboardCell,
) -> Option<&'d crate::report::DashboardCell> {
    match &source.baseline {
        Some(prefix) => t
            .cells
            .iter()
            .find(|bc| bc.workload == c.workload && bc.mode == c.mode && bc.target == *prefix)
            .or_else(|| {
                t.cells.iter().find(|bc| {
                    bc.workload == c.workload
                        && bc.mode == c.mode
                        && bc.target.starts_with(prefix.as_str())
                })
            }),
        None => crate::report::baseline_cell(dash, &t.triplet, &c.workload, c.mode),
    }
}

/// One sourced cell's value across the dashboards: every dashboard cell
/// the source selects (workload suffix + target prefix + mode) derives a
/// value per the shape, and the aggregate keeps the WORST of them — the
/// max for a `le` bound, the min for a `ge` bound, so the gate is as
/// honest as its worst leg. A ratio/speedup cell whose baseline arm is
/// absent contributes nothing (the named-gap rule — never an invented
/// ratio), a teardown-delta cell whose runs produced no output
/// contributes nothing (no first-output median), and a cell selecting
/// nothing answers None (the covenant reports it absent). The returned
/// candidate is the one the aggregate KEPT — the witness reads its leg.
fn derive_cell(
    source: &CellSource,
    comparison: Comparison,
    dashboards: &[crate::report::Dashboard],
) -> Option<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for dash in dashboards {
        for t in &dash.triplets {
            for c in &t.cells {
                let selected = source
                    .workloads
                    .iter()
                    .any(|w| c.workload.ends_with(w.as_str()))
                    && c.target.starts_with(source.target.as_str())
                    && c.mode == source.mode;
                if !selected {
                    continue;
                }
                let reference = reference_cell(source, dash, t, c);
                let value = match source.shape {
                    Shape::Absolute => Some(measure_of(c, source.measure)),
                    // wall − first-output on the SAME cell; absent when
                    // the cell's runs never produced output.
                    Shape::TeardownDelta => {
                        c.median_first_output_s.map(|first| c.median_wall_s - first)
                    }
                    Shape::Ratio | Shape::Speedup => {
                        let base = measure_of(c, source.measure);
                        // No baseline arm for this cell → it contributes
                        // nothing (the named-gap rule, never an invented
                        // ratio) — skip it; other cells may still answer.
                        reference.map(|bc| measure_of(bc, source.measure)).map(|b| {
                            if source.shape == Shape::Ratio {
                                base / b
                            } else {
                                b / base
                            }
                        })
                    }
                };
                if let Some(v) = value {
                    candidates.push(Candidate {
                        value: v,
                        suite: dash.suite.clone(),
                        triplet: t.triplet.clone(),
                        workload: c.workload.clone(),
                        target: c.target.clone(),
                        mode: c.mode,
                        reference: reference
                            .map(|bc| (bc.target.clone(), measure_of(bc, source.measure))),
                        paired_ratio: c.paired_ratio_vs_baseline,
                        cell_median: measure_of(c, source.measure),
                    });
                }
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    // The worst-of aggregation (the only shape today), keeping the
    // candidate the aggregate picked so the witness reads the leg that
    // trips the gate.
    let worst = candidates.into_iter().reduce(|a, b| match comparison {
        Comparison::Le => {
            if b.value > a.value {
                b
            } else {
                a
            }
        }
        Comparison::Ge => {
            if b.value < a.value {
                b
            } else {
                a
            }
        }
    })?;
    Some(worst)
}

/// The rolling median of the last ≤ `WITNESS_WINDOW` history entries for
/// one leg of one cell, newest last in the feed's order. None (with the
/// reason) when the feed names fewer than `WITNESS_MIN_HISTORY`
/// qualifying entries — a thinner history is noise, not a baseline.
fn rolling_median(
    history: &crate::trend::TrendFile,
    suite: &str,
    triplet: &str,
    workload: &str,
    target: &str,
    mode: crate::result::RunMode,
    pick: impl Fn(&crate::trend::TrendCell) -> Option<f64>,
) -> Result<f64, String> {
    let mut values: Vec<f64> = history
        .cells
        .iter()
        .filter(|c| {
            c.suite == suite
                && c.triplet == triplet
                && c.workload == workload
                && c.target == target
                && c.mode == mode
        })
        .filter_map(&pick)
        .collect();
    let n = values.len();
    if n < WITNESS_MIN_HISTORY {
        return Err(format!(
            "history too thin for {target} on {triplet}: {n} published entr{} (need {WITNESS_MIN_HISTORY})",
            if n == 1 { "y" } else { "ies" }
        ));
    }
    let values = &mut values[n.saturating_sub(WITNESS_WINDOW)..];
    values.sort_by(f64::total_cmp);
    Ok(crate::engine::median_f64(values))
}

/// Derive one sourced cell's fleet witness (tebako#1071) from the
/// candidate the aggregate kept: the leg identity + image off the
/// current dashboard, the two drift numbers against the rolling history
/// when the feed carried enough of it. Every degradation is an explicit
/// null with the reason in `note` — never a silent hole.
fn derive_witness(
    source: &CellSource,
    kept: &Candidate,
    dashboards: &[crate::report::Dashboard],
    history: Option<&crate::trend::TrendFile>,
) -> CellWitness {
    let dash = dashboards
        .iter()
        .find(|d| d.suite == kept.suite && d.triplets.iter().any(|t| t.triplet == kept.triplet));
    let runner = dash
        .and_then(|d| d.triplets.iter().find(|t| t.triplet == kept.triplet))
        .map(|t| &t.runner);
    let image_os = runner.and_then(|r| r.image_os.clone());
    let image_version = runner.and_then(|r| r.image_version.clone());

    // The kept leg's own published-entry count — the `mechanical`
    // class's first-calibration input. Computed before the reference
    // early-return so a no-reference cell (the author loop) carries it
    // too; 0 = this run is the cell's first fleet measurement.
    let history_entries = history.map(|h| {
        h.cells
            .iter()
            .filter(|c| {
                c.suite == kept.suite
                    && c.triplet == kept.triplet
                    && c.workload == kept.workload
                    && c.target == kept.target
                    && c.mode == kept.mode
            })
            .count() as u32
    });

    let mut notes: Vec<String> = Vec::new();
    let mut native_drift = None;
    let mut pair_delta = None;

    let Some((ref_target, ref_median)) = &kept.reference else {
        notes.push(format!(
            "no reference arm for {}/{} — the workload ran no baseline cell, the witness has nothing to compare against",
            kept.workload, kept.target
        ));
        return CellWitness {
            triplet: kept.triplet.clone(),
            workload: kept.workload.clone(),
            target: kept.target.clone(),
            mode: kept.mode,
            reference: None,
            native_drift,
            pair_delta,
            image_os,
            image_version,
            image_changed: None,
            history_entries,
            note: Some(notes.join("; ")),
        };
    };

    if let Some(h) = history {
        let measure_pick = |c: &crate::trend::TrendCell| -> Option<f64> {
            match source.measure {
                Measure::Wall => Some(c.median_wall_s),
                Measure::Rss => c.median_peak_rss_bytes.map(|b| b as f64),
            }
        };
        match rolling_median(
            h,
            &kept.suite,
            &kept.triplet,
            &kept.workload,
            ref_target,
            kept.mode,
            measure_pick,
        ) {
            Ok(rolling) if rolling > 0.0 => native_drift = Some(ref_median / rolling),
            Ok(_) => notes.push("the rolling reference median is zero".to_string()),
            Err(why) => notes.push(why),
        }
        // The current pair ratio: the published paired per-iteration
        // figure when the cell carries one, else the ratio of medians.
        let current_pair = kept
            .paired_ratio
            .or_else(|| (*ref_median > 0.0).then(|| kept.cell_median / ref_median));
        match current_pair {
            None => notes.push(format!(
                "no pair ratio for {}/{} — the reference median is zero",
                kept.workload, kept.target
            )),
            Some(current) => match rolling_median(
                h,
                &kept.suite,
                &kept.triplet,
                &kept.workload,
                &kept.target,
                kept.mode,
                |c| c.ratio_vs_baseline,
            ) {
                Ok(rolling) if rolling > 0.0 => pair_delta = Some(current / rolling),
                Ok(_) => notes.push("the rolling pair ratio is zero".to_string()),
                Err(why) => notes.push(why),
            },
        }
    } else {
        notes.push(
            "no history feed (the gate ran without the bench-history trend) — the drifts are null"
                .to_string(),
        );
    }

    // The image-changed observation (the #1067 smoking gun): the leg's
    // image identity vs the newest history entry carrying one for the
    // same suite × triplet.
    let image_changed = match (history, &image_os, &image_version) {
        (Some(h), Some(os), Some(ver)) => h
            .cells
            .iter()
            .rev()
            .find(|c| {
                c.suite == kept.suite && c.triplet == kept.triplet && c.image_version.is_some()
            })
            .map(|prev| {
                prev.image_os.as_deref() != Some(os.as_str())
                    || prev.image_version.as_deref() != Some(ver.as_str())
            }),
        _ => None,
    };

    CellWitness {
        triplet: kept.triplet.clone(),
        workload: kept.workload.clone(),
        target: kept.target.clone(),
        mode: kept.mode,
        reference: Some(ref_target.clone()),
        native_drift,
        pair_delta,
        image_os,
        image_version,
        image_changed,
        history_entries,
        note: if notes.is_empty() {
            None
        } else {
            Some(notes.join("; "))
        },
    }
}

/// The producer: the MetricsFile a run's dashboards imply for the
/// covenant's sourced cells. Cells whose source selects nothing (a suite
/// the run did not execute, an arm that produced no numbers) stay absent
/// — the covenant reports the gap, never a fabricated number. Each
/// derived cell also carries its fleet witness (tebako#1071); `history`
/// is the bench-history trend feed when the workflow fetched it.
pub fn produce(
    covenant: &CovenantFile,
    dashboards: &[crate::report::Dashboard],
    release: Option<&str>,
    history: Option<&crate::trend::TrendFile>,
) -> MetricsFile {
    let mut metrics = BTreeMap::new();
    let mut witness = BTreeMap::new();
    for cell in &covenant.cells {
        let Some(source) = &cell.source else {
            continue;
        };
        if let Some(kept) = derive_cell(source, cell.comparison, dashboards) {
            witness.insert(
                cell.id.clone(),
                derive_witness(source, &kept, dashboards, history),
            );
            metrics.insert(cell.id.clone(), kept.value);
        }
    }
    MetricsFile {
        schema_version: 1,
        release: release.map(str::to_string),
        metrics,
        witness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covenant() -> CovenantFile {
        CovenantFile::from_yaml(
            "schema_version: 1\ncells:\n  \
             - {id: author-loop.repress, bound: 6.0, comparison: le, unit: s}\n  \
             - {id: filesystem.stat.p95, bound: 2.0, comparison: le, unit: ratio}\n  \
             - {id: steady.throughput, bound: 0.85, comparison: ge, unit: ratio}\n",
        )
        .expect("fixture covenant")
    }

    fn metrics(pairs: &[(&str, f64)]) -> MetricsFile {
        MetricsFile {
            schema_version: 1,
            release: Some("v2.8.61".to_string()),
            metrics: pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            witness: Default::default(),
        }
    }

    #[test]
    fn every_cell_inside_its_bound_passes() {
        let r = evaluate(
            &covenant(),
            &metrics(&[
                ("author-loop.repress", 4.2),
                ("filesystem.stat.p95", 1.4),
                ("steady.throughput", 0.97),
            ]),
            None,
        );
        assert_eq!(r.verdict, "pass");
        assert!(r.passes());
        assert!(r.cells.iter().all(|c| c.state == State::Pass));
    }

    #[test]
    fn a_breach_names_the_cell_and_both_numbers() {
        let r = evaluate(
            &covenant(),
            &metrics(&[
                ("author-loop.repress", 7.5),
                ("filesystem.stat.p95", 1.4),
                ("steady.throughput", 0.97),
            ]),
            None,
        );
        assert_eq!(r.verdict, "fail");
        let over: Vec<_> = r.cells.iter().filter(|c| c.state == State::Over).collect();
        assert_eq!(over.len(), 1);
        assert_eq!(over[0].id, "author-loop.repress");
        assert_eq!(over[0].measured, Some(7.5));
        assert_eq!(over[0].bound, 6.0);
        assert_eq!(over[0].comparison, "<=");
    }

    #[test]
    fn a_ge_cell_fails_when_the_ratio_drops() {
        let r = evaluate(
            &covenant(),
            &metrics(&[
                ("author-loop.repress", 4.0),
                ("filesystem.stat.p95", 1.2),
                ("steady.throughput", 0.60),
            ]),
            None,
        );
        assert_eq!(r.verdict, "fail");
        assert_eq!(
            r.cells
                .iter()
                .find(|c| c.id == "steady.throughput")
                .unwrap()
                .state,
            State::Over
        );
    }

    #[test]
    fn a_missing_metric_is_absent_and_does_not_fail() {
        let r = evaluate(&covenant(), &metrics(&[("author-loop.repress", 4.0)]), None);
        assert_eq!(r.verdict, "pass");
        assert_eq!(
            r.cells.iter().filter(|c| c.state == State::Absent).count(),
            2
        );
    }

    #[test]
    fn injection_multiplies_one_axis_and_trips_it() {
        // The trip proof: a 10% injection on the author loop turns a 5.5 s
        // median into 6.05 s and breaches the 6 s bound, while the other
        // cells keep their measured values.
        let r = evaluate(
            &covenant(),
            &metrics(&[
                ("author-loop.repress", 5.5),
                ("filesystem.stat.p95", 1.4),
                ("steady.throughput", 0.97),
            ]),
            Some(("author-loop", 1.10)),
        );
        assert_eq!(r.verdict, "fail");
        let cell = r
            .cells
            .iter()
            .find(|c| c.id == "author-loop.repress")
            .unwrap();
        assert_eq!(cell.state, State::Over);
        let measured = cell.measured.expect("the injected value");
        assert!(
            (measured - 6.05).abs() < 1e-9,
            "5.5 x 1.10 must be 6.05 (got {measured})"
        );
        assert_eq!(cell.injected, Some(5.5));
        let others: Vec<_> = r
            .cells
            .iter()
            .filter(|c| c.id != "author-loop.repress")
            .collect();
        assert!(others.iter().all(|c| c.injected.is_none()));
    }

    #[test]
    fn injection_matches_an_exact_id_too() {
        let r = evaluate(
            &covenant(),
            &metrics(&[("filesystem.stat.p95", 1.9)]),
            Some(("filesystem.stat.p95", 1.10)),
        );
        let cell = r
            .cells
            .iter()
            .find(|c| c.id == "filesystem.stat.p95")
            .unwrap();
        assert_eq!(cell.state, State::Over);
        assert_eq!(cell.injected, Some(1.9));
    }

    #[test]
    fn duplicate_and_empty_cell_ids_are_named_errors() {
        let err = CovenantFile::from_yaml(
            "schema_version: 1\ncells:\n  - {id: a.b, bound: 1.0, comparison: le}\n  \
             - {id: a.b, bound: 2.0, comparison: le}\n",
        )
        .unwrap_err();
        assert!(err.message.contains("declared twice"), "{}", err.message);
        let err = CovenantFile::from_yaml("schema_version: 1\ncells: []\n").unwrap_err();
        assert!(err.message.contains("no cells"), "{}", err.message);
    }

    #[test]
    fn inject_spec_parses_and_refuses_nonsense() {
        assert_eq!(parse_inject("a.b:1.10").unwrap(), ("a.b".to_string(), 1.10));
        assert!(parse_inject("a.b").is_err());
        assert!(parse_inject("a.b:xyz").is_err());
        assert!(parse_inject("a.b:-1").is_err());
    }

    // ---- the producer (spec 27 §11's covenant arm) ----

    fn dash_cell(
        workload: &str,
        target: &str,
        mode: crate::result::RunMode,
        median_wall_s: f64,
        median_peak_rss_bytes: u64,
    ) -> crate::report::DashboardCell {
        crate::report::DashboardCell {
            workload: workload.to_string(),
            target: target.to_string(),
            mode,
            n: 5,
            median_wall_s,
            min_wall_s: median_wall_s,
            max_wall_s: median_wall_s,
            stdev_wall_s: None,
            mean_wall_s: median_wall_s,
            median_first_output_s: None,
            median_cpu_s: median_wall_s,
            median_peak_rss_bytes,
            speedup_vs_v1: None,
            paired_ratio_vs_baseline: None,
        }
    }

    /// A one-triplet dashboard over the given cells, the runtime suite's
    /// declared baseline (`on-system`) in place.
    fn dashboard(cells: Vec<crate::report::DashboardCell>) -> crate::report::Dashboard {
        dashboard_on("macos-arm64", cells)
    }

    /// The dashboard helper with the triplet chosen (the witness tests
    /// key history per triplet).
    fn dashboard_on(
        triplet: &str,
        cells: Vec<crate::report::DashboardCell>,
    ) -> crate::report::Dashboard {
        crate::report::Dashboard {
            suite: "runtime-on-system-vs-tebako".to_string(),
            generated_by: "test".to_string(),
            baseline: Some(crate::result::Baseline {
                target: "on-system".to_string(),
                label: "on-system".to_string(),
            }),
            notes: Vec::new(),
            triplets: vec![crate::report::DashboardTriplet {
                triplet: triplet.to_string(),
                runner: crate::result::RunnerMeta {
                    runs_on: "macos-15".to_string(),
                    arch: "arm64".to_string(),
                    cpus: 10,
                    ram_bytes: 32 * 1024 * 1024 * 1024,
                    image_os: Some("macos15".to_string()),
                    image_version: Some("20261111.127".to_string()),
                },
                versions: Default::default(),
                cells,
                unavailable: Vec::new(),
                failed: Vec::new(),
            }],
        }
    }

    fn sourced(
        id: &str,
        bound: f64,
        comparison: Comparison,
        workloads: &[&str],
        measure: Measure,
        shape: Shape,
    ) -> Cell {
        Cell {
            id: id.to_string(),
            bound,
            comparison,
            unit: None,
            note: None,
            source: Some(CellSource {
                workloads: workloads.iter().map(|s| s.to_string()).collect(),
                target: "tebako-".to_string(),
                baseline: None,
                mode: crate::result::RunMode::Warm,
                measure,
                shape,
                aggregate: Aggregate::Worst,
            }),
        }
    }

    fn covenant_with(cells: Vec<Cell>) -> CovenantFile {
        CovenantFile {
            schema_version: 1,
            cells,
        }
    }

    #[test]
    fn the_producer_derives_a_ratio_cell_worst_of_across_engines() {
        use crate::result::RunMode::Warm;
        // ruby at parity (1.00), python at 1.30, java at 1.60 — the
        // le-bound cell reads the WORST (the max), never the mean.
        let dash = dashboard(vec![
            dash_cell("ruby-boot", "on-system-ruby", Warm, 0.40, 10),
            dash_cell("ruby-boot", "tebako-ruby", Warm, 0.40, 12),
            dash_cell("python-boot", "on-system-python", Warm, 0.50, 10),
            dash_cell("python-boot", "tebako-python", Warm, 0.65, 12),
            dash_cell("java-boot", "on-system-java", Warm, 1.00, 10),
            dash_cell("java-boot", "tebako-java", Warm, 1.60, 12),
        ]);
        let cov = covenant_with(vec![sourced(
            "first-response.warm.median",
            1.5,
            Comparison::Le,
            &["-boot"],
            Measure::Wall,
            Shape::Ratio,
        )]);
        let m = produce(&cov, &[dash], Some("v9.9.9"), None);
        assert_eq!(m.release.as_deref(), Some("v9.9.9"));
        assert_eq!(m.metrics.get("first-response.warm.median"), Some(&1.60));
    }

    #[test]
    fn the_producer_derives_a_speedup_cell_worst_of_as_the_min() {
        use crate::result::RunMode::Warm;
        // speedup = baseline/tebako: ruby 1.25, java 0.90 — the ge-bound
        // cell reads the WORST (the min).
        let dash = dashboard(vec![
            dash_cell("ruby-fib", "on-system-ruby", Warm, 1.00, 10),
            dash_cell("ruby-fib", "tebako-ruby", Warm, 0.80, 12),
            dash_cell("java-fib", "on-system-java", Warm, 1.00, 10),
            dash_cell("java-fib", "tebako-java", Warm, 1.111, 12),
        ]);
        let cov = covenant_with(vec![sourced(
            "steady.cpu.throughput",
            0.85,
            Comparison::Ge,
            &["-fib"],
            Measure::Wall,
            Shape::Speedup,
        )]);
        let m = produce(&cov, &[dash], None, None);
        let got = m.metrics["steady.cpu.throughput"];
        // the min of 1.25 and 1.0/1.111
        let expect = 1.0f64 / 1.111f64;
        assert!((got - expect).abs() < 1e-9, "{got} != {expect}");
    }

    #[test]
    fn the_producer_rides_the_rss_measure() {
        use crate::result::RunMode::Warm;
        let dash = dashboard(vec![
            dash_cell("ruby-boot", "on-system-ruby", Warm, 0.40, 100_000),
            dash_cell("ruby-boot", "tebako-ruby", Warm, 0.40, 120_000),
        ]);
        let cov = covenant_with(vec![sourced(
            "memory.rss.ratio",
            1.25,
            Comparison::Le,
            &["-boot"],
            Measure::Rss,
            Shape::Ratio,
        )]);
        let m = produce(&cov, &[dash], None, None);
        assert_eq!(m.metrics.get("memory.rss.ratio"), Some(&1.2));
    }

    #[test]
    fn a_cell_with_no_baseline_arm_contributes_nothing() {
        use crate::result::RunMode::Warm;
        // Only the tebako arm — the baseline arm is absent, so the ratio
        // cell derives nothing and the metric stays absent.
        let dash = dashboard(vec![dash_cell("ruby-boot", "tebako-ruby", Warm, 0.40, 12)]);
        let cov = covenant_with(vec![sourced(
            "first-response.warm.median",
            1.5,
            Comparison::Le,
            &["-boot"],
            Measure::Wall,
            Shape::Ratio,
        )]);
        let m = produce(&cov, &[dash], None, None);
        assert!(!m.metrics.contains_key("first-response.warm.median"));
    }

    #[test]
    fn a_source_selecting_nothing_leaves_the_cell_absent() {
        use crate::result::RunMode::Warm;
        let dash = dashboard(vec![
            dash_cell("ruby-fib", "on-system-ruby", Warm, 1.0, 10),
            dash_cell("ruby-fib", "tebako-ruby", Warm, 1.0, 10),
        ]);
        let cov = covenant_with(vec![sourced(
            "first-response.warm.median",
            1.5,
            Comparison::Le,
            &["-boot"],
            Measure::Wall,
            Shape::Ratio,
        )]);
        let m = produce(&cov, &[dash], None, None);
        assert!(!m.metrics.contains_key("first-response.warm.median"));
    }

    #[test]
    fn the_covenant_check_refuses_a_source_with_no_selector() {
        let bad = "schema_version: 1\ncells:\n  - id: x\n    bound: 1.0\n    comparison: le\n    source: {workloads: [], target: \"tebako-\", mode: warm, measure: wall, shape: ratio, aggregate: worst}\n";
        let err = CovenantFile::from_yaml(bad).unwrap_err();
        assert!(err.to_string().contains("no workload selector"), "{err}");
    }

    #[test]
    fn the_producer_derives_a_teardown_delta_worst_of() {
        use crate::result::RunMode::Warm;
        // wall − first-output per cell: ruby 0.40 − 0.39 = 0.01, java
        // 2.10 − 1.90 = 0.20 — the le-bound cell reads the WORST (max).
        let mut ruby = dash_cell("ruby-boot", "tebako-ruby", Warm, 0.40, 12);
        ruby.median_first_output_s = Some(0.39);
        let mut java = dash_cell("java-boot", "tebako-java", Warm, 2.10, 12);
        java.median_first_output_s = Some(1.90);
        let dash = dashboard(vec![
            dash_cell("ruby-boot", "on-system-ruby", Warm, 0.05, 10),
            ruby,
            dash_cell("java-boot", "on-system-java", Warm, 0.60, 10),
            java,
        ]);
        let cov = covenant_with(vec![sourced(
            "teardown.exit_seconds",
            0.5,
            Comparison::Le,
            &["-boot"],
            Measure::Wall,
            Shape::TeardownDelta,
        )]);
        let m = produce(&cov, &[dash], None, None);
        let got = m.metrics["teardown.exit_seconds"];
        assert!((got - 0.20).abs() < 1e-9, "{got} != 0.20");
    }

    #[test]
    fn a_teardown_delta_with_no_first_output_contributes_nothing() {
        use crate::result::RunMode::Warm;
        // The selected cells carry no first-output median (silent runs)
        // — the delta derives nothing and the metric stays absent.
        let dash = dashboard(vec![dash_cell("ruby-boot", "tebako-ruby", Warm, 0.40, 12)]);
        let cov = covenant_with(vec![sourced(
            "teardown.exit_seconds",
            0.5,
            Comparison::Le,
            &["-boot"],
            Measure::Wall,
            Shape::TeardownDelta,
        )]);
        let m = produce(&cov, &[dash], None, None);
        assert!(!m.metrics.contains_key("teardown.exit_seconds"));
    }

    #[test]
    fn the_producer_honours_a_baseline_override() {
        use crate::result::RunMode::Warm;
        // The override divides the tebako arm by the harness's extract
        // arm — not the dashboard's declared on-system baseline.
        let dash = dashboard(vec![
            dash_cell("ruby-spawn", "on-system-ruby", Warm, 0.02, 10),
            dash_cell("ruby-spawn", "tebako-ruby", Warm, 0.29, 12),
            dash_cell("ruby-spawn", "extract-ruby", Warm, 0.40, 12),
        ]);
        let mut cell = sourced(
            "teardown.materialize_vs_extract",
            1.0,
            Comparison::Le,
            &["-spawn"],
            Measure::Wall,
            Shape::Ratio,
        );
        cell.source.as_mut().unwrap().baseline = Some("extract-".to_string());
        let cov = covenant_with(vec![cell]);
        let m = produce(&cov, &[dash], None, None);
        let got = m.metrics["teardown.materialize_vs_extract"];
        assert!((got - 0.29 / 0.40).abs() < 1e-9, "{got} != 0.725");
    }

    #[test]
    fn the_check_pins_the_shape_rules() {
        // teardown-delta rides the wall measure only.
        let bad = "schema_version: 1\ncells:\n  - id: x\n    bound: 1.0\n    comparison: le\n    source: {workloads: [\"-boot\"], target: \"tebako-\", mode: warm, measure: rss, shape: teardown-delta, aggregate: worst}\n";
        let err = CovenantFile::from_yaml(bad).unwrap_err();
        assert!(err.to_string().contains("wall"), "{err}");
        // A baseline override rides ratio|speedup only.
        let bad = "schema_version: 1\ncells:\n  - id: x\n    bound: 1.0\n    comparison: le\n    source: {workloads: [\"-boot\"], target: \"tebako-\", baseline: \"extract-\", mode: warm, measure: wall, shape: absolute, aggregate: worst}\n";
        let err = CovenantFile::from_yaml(bad).unwrap_err();
        assert!(err.to_string().contains("ratio|speedup"), "{err}");
    }

    // ---- the fleet witness (tebako#1071) ----

    fn trend_cell(
        release: &str,
        triplet: &str,
        workload: &str,
        target: &str,
        median: f64,
        ratio: Option<f64>,
        image_version: &str,
    ) -> crate::trend::TrendCell {
        crate::trend::TrendCell {
            release: release.to_string(),
            date: "2026-11-10".to_string(),
            suite: "runtime-on-system-vs-tebako".to_string(),
            triplet: triplet.to_string(),
            workload: workload.to_string(),
            target: target.to_string(),
            mode: crate::result::RunMode::Warm,
            median_wall_s: median,
            ratio_vs_baseline: ratio,
            median_peak_rss_bytes: Some(100),
            image_os: Some("ubuntu24".to_string()),
            image_version: Some(image_version.to_string()),
        }
    }

    /// Eight flat published runs of the statloop leg: native median
    /// 0.10, tebako median 1.00 (pair ratio 10.0), image .370 — the
    /// pre-move fleet (the #1067 shape).
    fn healthy_history() -> crate::trend::TrendFile {
        let mut cells = Vec::new();
        for i in 0..8 {
            let rel = format!("run-4{i}");
            cells.push(trend_cell(
                &rel,
                "linux-musl-x86_64",
                "ruby-statloop",
                "on-system-ruby",
                0.10,
                Some(1.0),
                "20261111.370",
            ));
            cells.push(trend_cell(
                &rel,
                "linux-musl-x86_64",
                "ruby-statloop",
                "tebako-ruby",
                1.00,
                Some(10.0),
                "20261111.370",
            ));
        }
        crate::trend::TrendFile {
            generated_by: "test".to_string(),
            cells,
        }
    }

    /// The current run's statloop dashboard on the musl leg with the
    /// given arm medians; the tebako cell carries the published paired
    /// per-iteration ratio (the interleave's figure).
    fn statloop_dashboard(native: f64, tebako: f64, paired: f64) -> crate::report::Dashboard {
        let mut dash = dashboard_on(
            "linux-musl-x86_64",
            vec![
                dash_cell(
                    "ruby-statloop",
                    "on-system-ruby",
                    crate::result::RunMode::Warm,
                    native,
                    10,
                ),
                {
                    let mut c = dash_cell(
                        "ruby-statloop",
                        "tebako-ruby",
                        crate::result::RunMode::Warm,
                        tebako,
                        12,
                    );
                    c.paired_ratio_vs_baseline = Some(paired);
                    c
                },
            ],
        );
        dash.triplets[0].runner.image_os = Some("ubuntu24".to_string());
        dash.triplets[0].runner.image_version = Some("20261111.127".to_string());
        dash
    }

    fn statloop_covenant(bound: f64) -> CovenantFile {
        covenant_with(vec![sourced(
            "filesystem.stat.median",
            bound,
            Comparison::Le,
            &["-statloop"],
            Measure::Wall,
            Shape::Ratio,
        )])
    }

    #[test]
    fn a_flat_fleet_reads_both_witnesses_flat() {
        let m = produce(
            &statloop_covenant(26.0),
            &[statloop_dashboard(0.10, 1.00, 10.0)],
            None,
            Some(&healthy_history()),
        );
        let w = &m.witness["filesystem.stat.median"];
        assert_eq!(w.triplet, "linux-musl-x86_64");
        assert_eq!(w.workload, "ruby-statloop");
        assert_eq!(w.target, "tebako-ruby");
        assert_eq!(w.reference.as_deref(), Some("on-system-ruby"));
        assert_eq!(w.native_drift, Some(1.0), "native 0.10 vs rolling 0.10");
        assert_eq!(w.pair_delta, Some(1.0), "paired 10.0 vs rolling 10.0");
        assert_eq!(w.note, None);
        assert_eq!(w.image_version.as_deref(), Some("20261111.127"));
        assert_eq!(
            w.image_changed,
            Some(true),
            "the leg's .127 vs the history's .370 — the #1067 smoking gun"
        );
    }

    #[test]
    fn a_fleet_move_reads_native_drift_large_pair_delta_small() {
        // Both arms doubled together (the runner got slower): the native
        // drift is large, the paired relationship is unmoved.
        let m = produce(
            &statloop_covenant(26.0),
            &[statloop_dashboard(0.20, 2.00, 10.0)],
            None,
            Some(&healthy_history()),
        );
        let w = &m.witness["filesystem.stat.median"];
        assert_eq!(w.native_drift, Some(2.0));
        assert_eq!(w.pair_delta, Some(1.0));
        // The trip (20.0 > 9.0) classifies as a fleet move.
        let report = evaluate(&statloop_covenant(9.0), &m, None);
        let cell = &report.cells[0];
        assert_eq!(cell.state, State::Over);
        assert_eq!(cell.class, Some(Class::FleetMove));
        assert!(
            cell.class_note.as_deref().unwrap().contains("fleet move"),
            "{:?}",
            cell.class_note
        );
        assert_eq!(cell.native_drift, Some(2.0));
        assert_eq!(cell.pair_delta, Some(1.0));
    }

    #[test]
    fn a_regression_reads_native_flat_pair_delta_moved() {
        // The tebako arm alone worsened: native flat, pair moved.
        let m = produce(
            &statloop_covenant(26.0),
            &[statloop_dashboard(0.10, 1.80, 18.0)],
            None,
            Some(&healthy_history()),
        );
        let report = evaluate(&statloop_covenant(9.0), &m, None);
        let cell = &report.cells[0];
        assert_eq!(cell.state, State::Over);
        assert_eq!(cell.class, Some(Class::Regression));
        assert!(
            cell.class_note.as_deref().unwrap().contains("regression"),
            "{:?}",
            cell.class_note
        );
    }

    #[test]
    fn divergent_arms_read_ambiguous_the_1067_shape() {
        // #1067 itself: the new image made the native stat FASTER (×0.6)
        // while the mount-routed path slowed — both witnesses moved.
        let m = produce(
            &statloop_covenant(26.0),
            &[statloop_dashboard(0.06, 1.60, 26.67)],
            None,
            Some(&healthy_history()),
        );
        let w = &m.witness["filesystem.stat.median"];
        assert!(
            (w.native_drift.unwrap() - 0.6).abs() < 1e-9,
            "{:?}",
            w.native_drift
        );
        assert!(
            (w.pair_delta.unwrap() - 2.667).abs() < 1e-3,
            "{:?}",
            w.pair_delta
        );
        let report = evaluate(&statloop_covenant(9.0), &m, None);
        let cell = &report.cells[0];
        assert_eq!(cell.class, Some(Class::Ambiguous));
        assert!(
            cell.class_note.as_deref().unwrap().contains("divergently"),
            "{:?}",
            cell.class_note
        );
        // The adjudication input rides the artifact: the leg and its
        // image identity.
        let w = cell.witness.as_ref().unwrap();
        assert_eq!(w.triplet, "linux-musl-x86_64");
        assert_eq!(w.image_changed, Some(true));
    }

    #[test]
    fn a_passing_cell_carries_the_witness_but_no_class() {
        let m = produce(
            &statloop_covenant(26.0),
            &[statloop_dashboard(0.10, 1.00, 10.0)],
            None,
            Some(&healthy_history()),
        );
        let report = evaluate(&statloop_covenant(26.0), &m, None);
        let cell = &report.cells[0];
        assert_eq!(cell.state, State::Pass);
        assert_eq!(cell.native_drift, Some(1.0), "the witness is the record");
        assert_eq!(cell.class, None, "classes are for trips");
    }

    #[test]
    fn thin_history_degrades_to_null_with_the_note() {
        let mut h = healthy_history();
        h.cells.truncate(4); // two releases, under the 3-entry floor
        let m = produce(
            &statloop_covenant(26.0),
            &[statloop_dashboard(0.10, 1.00, 10.0)],
            None,
            Some(&h),
        );
        let w = &m.witness["filesystem.stat.median"];
        assert_eq!(w.native_drift, None);
        assert_eq!(w.pair_delta, None);
        let note = w.note.as_deref().unwrap();
        assert!(note.contains("history too thin"), "{note}");
        assert!(note.contains("2"), "{note}");
    }

    #[test]
    fn no_history_feed_degrades_to_null_with_the_note() {
        let m = produce(
            &statloop_covenant(26.0),
            &[statloop_dashboard(0.10, 1.00, 10.0)],
            None,
            None,
        );
        let w = &m.witness["filesystem.stat.median"];
        assert_eq!(w.native_drift, None);
        assert_eq!(w.pair_delta, None);
        assert!(
            w.note.as_deref().unwrap().contains("no history feed"),
            "{:?}",
            w.note
        );
        // The leg + image identity are still recorded (the current
        // dashboard carries them) — only the drifts degrade.
        assert_eq!(w.image_version.as_deref(), Some("20261111.127"));
        assert_eq!(w.image_changed, None);
    }

    #[test]
    fn no_reference_arm_degrades_with_the_note() {
        use crate::result::RunMode::Warm;
        // An absolute cell whose workload ran no baseline arm (the
        // author loop's shape): the witness has nothing to compare
        // against and says so.
        let dash = dashboard_on(
            "macos-arm64",
            vec![dash_cell("author-repress", "v2-tools", Warm, 7.9, 100)],
        );
        let mut cell = sourced(
            "author-loop.repress.median",
            6.0,
            Comparison::Le,
            &["-repress"],
            Measure::Wall,
            Shape::Absolute,
        );
        cell.source.as_mut().unwrap().target = "v2-tools".to_string();
        let cov = covenant_with(vec![cell]);
        let m = produce(&cov, &[dash], None, Some(&healthy_history()));
        let w = &m.witness["author-loop.repress.median"];
        assert_eq!(w.reference, None);
        assert_eq!(w.native_drift, None);
        assert_eq!(w.pair_delta, None);
        assert!(
            w.note.as_deref().unwrap().contains("no reference arm"),
            "{:?}",
            w.note
        );
    }

    #[test]
    fn the_pair_delta_falls_back_to_the_ratio_of_medians() {
        // A dashboard cell without the published paired ratio (an old
        // dashboard): the witness divides the medians instead.
        let mut dash = statloop_dashboard(0.10, 1.00, 10.0);
        dash.triplets[0].cells[1].paired_ratio_vs_baseline = None;
        let m = produce(
            &statloop_covenant(26.0),
            &[dash],
            None,
            Some(&healthy_history()),
        );
        let w = &m.witness["filesystem.stat.median"];
        assert_eq!(w.pair_delta, Some(1.0), "1.00/0.10 = 10.0 vs rolling 10.0");
    }

    #[test]
    fn classify_is_the_truth_table() {
        let class_of = |d: f64, p: f64| classify(Some(d), Some(p), Some(8)).map(|(c, _)| c);
        // fleet move: native drift large, pair delta small
        assert_eq!(class_of(2.0, 1.0), Some(Class::FleetMove));
        assert_eq!(class_of(0.5, 1.29), Some(Class::FleetMove));
        // regression: tebako arm moved, native flat
        assert_eq!(class_of(1.0, 2.0), Some(Class::Regression));
        assert_eq!(class_of(1.29, 1.31), Some(Class::Regression));
        // ambiguous: both moved divergently
        assert_eq!(class_of(0.6, 2.24), Some(Class::Ambiguous));
        // mechanical: neither arm left its healthy band — the bound, not
        // the pair, is what moved (a tight or miscalibrated bound)
        assert_eq!(class_of(1.0, 1.0), Some(Class::Mechanical));
        // the tolerance band's edges are flat (the trip then reads
        // mechanical); just outside them is a move
        assert_eq!(class_of(WITNESS_TOLERANCE, 1.0), Some(Class::Mechanical));
        assert_eq!(
            class_of(1.0 / WITNESS_TOLERANCE, 1.0),
            Some(Class::Mechanical)
        );
        assert_eq!(
            class_of(WITNESS_TOLERANCE + 0.01, 1.0),
            Some(Class::FleetMove)
        );
        assert_eq!(
            class_of(1.0 / (WITNESS_TOLERANCE + 0.01), 1.0),
            Some(Class::FleetMove)
        );
        // a degraded witness with history on record stays unclassified…
        assert_eq!(classify(None, Some(2.0), Some(8)), None);
        assert_eq!(classify(Some(1.0), None, Some(2)), None);
        // …but zero published entries is the first-calibration case, and
        // it wins over the band table (the band has no numbers yet) —
        // v2.8.85's teardown.materialize_vs_extract and
        // author-loop.repress trips land here
        let first = classify(None, None, Some(0)).map(|(c, _)| c);
        assert_eq!(first, Some(Class::Mechanical));
        let first = classify(Some(55.51), Some(0.72), Some(0)).map(|(c, _)| c);
        assert_eq!(first, Some(Class::Mechanical));
        // no feed at all (None) is not the first-calibration case
        assert_eq!(classify(None, None, None), None);
    }

    #[test]
    fn the_witness_map_merges_last_wins() {
        let mut a = metrics(&[("x.y", 1.0)]);
        let mut b = metrics(&[("z.w", 2.0)]);
        let w = |t: &str| CellWitness {
            triplet: t.to_string(),
            workload: "ruby-statloop".to_string(),
            target: "tebako-ruby".to_string(),
            mode: crate::result::RunMode::Warm,
            reference: Some("on-system-ruby".to_string()),
            native_drift: Some(1.0),
            pair_delta: Some(1.0),
            image_os: None,
            image_version: None,
            image_changed: None,
            history_entries: Some(8),
            note: None,
        };
        a.witness.insert("x.y".to_string(), w("macos-arm64"));
        b.witness.insert("z.w".to_string(), w("linux-musl-x86_64"));
        let collisions = a.merge(&b);
        assert!(collisions.is_empty());
        assert_eq!(a.witness.len(), 2);
        assert_eq!(a.witness["z.w"].triplet, "linux-musl-x86_64");
    }
}
