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
    /// arm). Absent on a cell no bench suite produces (the author-loop
    /// cells read the walkthrough's recorded timings) — such a cell
    /// answers `absent` from a bench run, the honest coverage gap.
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
    /// declared baseline, never re-declared here.
    pub target: String,
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
    /// is recorded so a duplicated id is visible rather than silent.
    pub fn merge(&mut self, other: &MetricsFile) -> Vec<String> {
        let mut collisions = Vec::new();
        for (k, v) in &other.metrics {
            if self.metrics.insert(k.clone(), *v).is_some() {
                collisions.push(k.clone());
            }
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
/// reaches `filesystem.stat.p95`).
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
            CellVerdict {
                id: cell.id.clone(),
                state,
                measured: value,
                bound: cell.bound,
                comparison: cell.comparison.token().to_string(),
                injected,
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

/// One sourced cell's value across the dashboards: every dashboard cell
/// the source selects (workload suffix + target prefix + mode) derives a
/// value per the shape, and the aggregate keeps the WORST of them — the
/// max for a `le` bound, the min for a `ge` bound, so the gate is as
/// honest as its worst leg. A ratio/speedup cell whose baseline arm is
/// absent contributes nothing (the named-gap rule — never an invented
/// ratio), and a cell selecting nothing answers None (the covenant
/// reports it absent).
fn derive_cell(
    source: &CellSource,
    comparison: Comparison,
    dashboards: &[crate::report::Dashboard],
) -> Option<f64> {
    let mut values: Vec<f64> = Vec::new();
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
                let base = measure_of(c, source.measure);
                let value = match source.shape {
                    Shape::Absolute => Some(base),
                    Shape::Ratio | Shape::Speedup => {
                        // No baseline arm for this cell → it contributes
                        // nothing (the named-gap rule, never an invented
                        // ratio) — skip it; other cells may still answer.
                        crate::report::baseline_cell(dash, &t.triplet, &c.workload, c.mode)
                            .map(|bc| measure_of(bc, source.measure))
                            .map(|b| {
                                if source.shape == Shape::Ratio {
                                    base / b
                                } else {
                                    b / base
                                }
                            })
                    }
                };
                if let Some(v) = value {
                    values.push(v);
                }
            }
        }
    }
    if values.is_empty() {
        return None;
    }
    // The worst-of aggregation (the only shape today).
    match comparison {
        Comparison::Le => values.into_iter().reduce(f64::max),
        Comparison::Ge => values.into_iter().reduce(f64::min),
    }
}

/// The producer: the MetricsFile a run's dashboards imply for the
/// covenant's sourced cells. Cells without a source (the author-loop
/// cells — the walkthrough's recorded timings) and cells whose source
/// selects nothing stay absent — the covenant reports the gap, never a
/// fabricated number.
pub fn produce(
    covenant: &CovenantFile,
    dashboards: &[crate::report::Dashboard],
    release: Option<&str>,
) -> MetricsFile {
    let mut metrics = BTreeMap::new();
    for cell in &covenant.cells {
        let Some(source) = &cell.source else {
            continue;
        };
        if let Some(value) = derive_cell(source, cell.comparison, dashboards) {
            metrics.insert(cell.id.clone(), value);
        }
    }
    MetricsFile {
        schema_version: 1,
        release: release.map(str::to_string),
        metrics,
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
            median_cpu_s: median_wall_s,
            median_peak_rss_bytes,
            speedup_vs_v1: None,
        }
    }

    /// A one-triplet dashboard over the given cells, the runtime suite's
    /// declared baseline (`on-system`) in place.
    fn dashboard(cells: Vec<crate::report::DashboardCell>) -> crate::report::Dashboard {
        crate::report::Dashboard {
            suite: "runtime-on-system-vs-tebako".to_string(),
            generated_by: "test".to_string(),
            baseline: Some(crate::result::Baseline {
                target: "on-system".to_string(),
                label: "on-system".to_string(),
            }),
            notes: Vec::new(),
            triplets: vec![crate::report::DashboardTriplet {
                triplet: "macos-arm64".to_string(),
                runner: crate::result::RunnerMeta {
                    runs_on: "macos-15".to_string(),
                    arch: "arm64".to_string(),
                    cpus: 10,
                    ram_bytes: 32 * 1024 * 1024 * 1024,
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
        let m = produce(&cov, &[dash], Some("v9.9.9"));
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
        let m = produce(&cov, &[dash], None);
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
        let m = produce(&cov, &[dash], None);
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
        let m = produce(&cov, &[dash], None);
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
        let m = produce(&cov, &[dash], None);
        assert!(!m.metrics.contains_key("first-response.warm.median"));
    }

    #[test]
    fn the_covenant_check_refuses_a_source_with_no_selector() {
        let bad = "schema_version: 1\ncells:\n  - id: x\n    bound: 1.0\n    comparison: le\n    source: {workloads: [], target: \"tebako-\", mode: warm, measure: wall, shape: ratio, aggregate: worst}\n";
        let err = CovenantFile::from_yaml(bad).unwrap_err();
        assert!(err.to_string().contains("no workload selector"), "{err}");
    }
}
