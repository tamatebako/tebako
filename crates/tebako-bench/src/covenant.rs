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
}
