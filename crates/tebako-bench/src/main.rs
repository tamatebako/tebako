//! tebako-bench — the spec 27 benchmark harness. CI TOOLING, NEVER SHIPPED
//! (the boundary comment lives at the crate root; see lib.rs).
//!
//! ```text
//! tebako-bench run --suite <suite.yaml> --platforms <platforms.yaml>
//!                  --triplet <t> --out <dir> [--opt-in <workload-id>]...
//! tebako-bench report <results.json>... --md <report.md> --json <dashboard.json>
//!                     [--metrics <m.json>] [--history <trend.json>]
//! tebako-bench trend --release <name> --date <YYYY-MM-DD> --dashboard <d>...
//!                    [--previous <trend.json>] --out <trend.json>
//! tebako-bench validate --kind suite|result <file>
//! ```
//!
//! Exit codes (spec 27 §8): 0 success/valid · 1 invalid · 2 operational.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tebako_bench::error::BenchError;
use tebako_bench::exit;
use tebako_bench::validate::{self, DocKind};

#[derive(Parser)]
#[command(
    name = "tebako-bench",
    version,
    about = "tebako benchmark harness — CI tooling, never shipped",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Execute the benchmark matrix for one triplet (spec 27 §5–§7:
    /// acquire, warmup → warm interleaved → cold-with-wipe, results.json).
    Run {
        /// The suite document (benchmarks/suite.yaml).
        #[arg(long)]
        suite: PathBuf,
        /// The platforms document (benchmarks/platforms.yaml).
        #[arg(long)]
        platforms: PathBuf,
        /// The triplet this leg runs (the release vocabulary).
        #[arg(long)]
        triplet: String,
        /// Output directory for results.json + logs/.
        #[arg(long)]
        out: PathBuf,
        /// Opt an opt-in workload in (repeatable; e.g. --opt-in compile-medium-oiml-r060).
        #[arg(long = "opt-in")]
        opt_in: Vec<String>,
        /// Pin the tebako tools release (default: the latest release; the
        /// resolved version is learned from the release's SHA256SUMS).
        #[arg(long = "tebako-release")]
        tebako_release: Option<String>,
        /// Vendored source paths resolve against this root (the repo root).
        #[arg(long = "repo-root", default_value = ".")]
        repo_root: PathBuf,
    },
    /// Merge N triplet result files into a markdown report + dashboard JSON
    /// (spec 27 §7: stats re-derived from the merged run records).
    Report {
        /// The per-triplet results.json files to merge.
        #[arg(required = true)]
        results: Vec<PathBuf>,
        /// The markdown report destination.
        #[arg(long)]
        md: PathBuf,
        /// The site-ingestible dashboard JSON destination.
        #[arg(long)]
        json: PathBuf,
        /// The covenant metrics destination (spec 27 §11's covenant arm):
        /// the dashboard drives the covenant's sourced cells into the
        /// MetricsFile the `covenant` gate evaluates.
        #[arg(long)]
        metrics: Option<PathBuf>,
        /// The authored covenant (read only when --metrics is set).
        #[arg(long, default_value = "benchmarks/covenant.yaml")]
        covenant: PathBuf,
        /// The release the metrics belong to.
        #[arg(long)]
        release: Option<String>,
        /// The bench-history trend feed (the fleet witness's rolling
        /// baseline, tebako#1071): the workflow fetches
        /// latest/trend.json and names it here; absent, the witness
        /// drifts degrade to explicit nulls with the reason noted.
        #[arg(long)]
        history: Option<PathBuf>,
    },
    /// Append a run's dashboards to the release-over-release trend feed
    /// (spec 27 §11: dedupe per full key, capped at the last 20 releases).
    Trend {
        /// The release name this run publishes under (v2.8.8 or run-<id>).
        #[arg(long)]
        release: String,
        /// The run date, YYYY-MM-DD.
        #[arg(long)]
        date: String,
        /// The run's dashboard JSON files (both suites).
        #[arg(long, required = true)]
        dashboard: Vec<PathBuf>,
        /// The previous latest/trend.json, when it exists.
        #[arg(long)]
        previous: Option<PathBuf>,
        /// The trend.json destination.
        #[arg(long)]
        out: PathBuf,
    },
    /// Evaluate the benchmark covenant against a run's metrics: exit 0 when
    /// every cell passes (an `absent` cell is a coverage gap, reported and
    /// not failing), 1 on any breach. `TEBAKO_BENCH_INJECT=<cell-or-axis>:
    /// <factor>` multiplies a matching cell — the trip proof's mechanism.
    Covenant {
        /// The metrics document(s) the run produced; several are merged.
        #[arg(long, required = true)]
        metrics: Vec<PathBuf>,
        /// The authored covenant.
        #[arg(long, default_value = "benchmarks/covenant.yaml")]
        covenant: PathBuf,
        /// Write the verdict document here for the release gate to copy.
        #[arg(long)]
        json: Option<PathBuf>,
    },
    /// Schema-check a suite or result document (both gates: the versioned
    /// JSON Schema + the serde model, then the semantic rules).
    Validate {
        /// The document kind — explicit, never guessed (invariant 9).
        #[arg(long)]
        kind: DocKind,
        /// The document to validate.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("tebako-bench: {} [{}]", e.message, e.code);
            ExitCode::from(e.code.clamp(1, 255) as u8)
        }
    }
}

/// The witness line under a tripped cell (tebako#1071): the leg the
/// aggregate kept, the two drift numbers, and the runner image identity
/// (changed-vs-history included). With no witness record at all (an
/// unsourced cell), says so — never a silent gap.
fn witness_line(cell: &tebako_bench::covenant::CellVerdict) -> String {
    use tebako_bench::covenant::CellWitness;
    let drift = |v: Option<f64>| {
        v.map(|x| format!("×{x:.2}"))
            .unwrap_or_else(|| "—".to_string())
    };
    match &cell.witness {
        None => "witness: none (the cell declares no source)".to_string(),
        Some(CellWitness {
            triplet,
            workload,
            target,
            mode,
            reference,
            native_drift,
            pair_delta,
            image_os,
            image_version,
            image_changed,
            history_entries,
            note,
        }) => {
            let mode = match mode {
                tebako_bench::result::RunMode::Warm => "warm",
                tebako_bench::result::RunMode::Cold => "cold",
            };
            let image = match (image_os, image_version) {
                (Some(os), Some(ver)) => format!(
                    " · image {os}/{ver}{}",
                    match image_changed {
                        Some(true) => " (CHANGED vs the previous published run)",
                        Some(false) => " (unchanged)",
                        None => "",
                    }
                ),
                _ => " · image unrecorded (off a GH-hosted runner)".to_string(),
            };
            let note = note
                .as_deref()
                .map(|n| format!(" · note: {n}"))
                .unwrap_or_default();
            let history = history_entries
                .map(|n| format!(" · history {n} published"))
                .unwrap_or_default();
            format!(
                "witness: leg {triplet} · {workload} · {mode} · {target} vs {} · native_drift {} · pair_delta {}{}{}{}",
                reference.as_deref().unwrap_or("(none)"),
                drift(*native_drift),
                drift(*pair_delta),
                image,
                history,
                note
            )
        }
    }
}

/// The class tally for the summary line: " [fleet-move 1, regression 1]"
/// when any tripped cell classified, empty otherwise.
fn class_tally(report: &tebako_bench::covenant::Report) -> String {
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for cell in &report.cells {
        if cell.state != tebako_bench::covenant::State::Over {
            continue;
        }
        let token = cell.class.map(|c| c.token()).unwrap_or("unclassified");
        match counts.iter_mut().find(|(t, _)| *t == token) {
            Some((_, n)) => *n += 1,
            None => counts.push((token, 1)),
        }
    }
    if counts.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = counts.iter().map(|(t, n)| format!("{t} {n}")).collect();
        format!(" [{}]", parts.join(", "))
    }
}

fn run(cli: Cli) -> Result<u8, BenchError> {
    match cli.command {
        Command::Covenant {
            metrics,
            covenant,
            json,
        } => {
            let covenant = tebako_bench::covenant::CovenantFile::load(&covenant)?;
            let mut merged = tebako_bench::covenant::MetricsFile {
                schema_version: 1,
                release: None,
                metrics: Default::default(),
                witness: Default::default(),
            };
            for path in &metrics {
                let text = std::fs::read_to_string(path).map_err(|e| {
                    BenchError::operational(format!("cannot read {}: {e}", path.display()))
                })?;
                let doc = tebako_bench::covenant::MetricsFile::from_json(&text)?;
                for id in merged.merge(&doc) {
                    eprintln!(
                        "{}: metric {id} appears in more than one file; the last wins",
                        path.display()
                    );
                }
            }
            let inject = match std::env::var("TEBAKO_BENCH_INJECT") {
                Ok(spec) if !spec.is_empty() => {
                    let (key, factor) = tebako_bench::covenant::parse_inject(&spec)?;
                    eprintln!("covenant: INJECTING {key} x{factor} (the trip proof)");
                    Some((key, factor))
                }
                _ => None,
            };
            let report = tebako_bench::covenant::evaluate(
                &covenant,
                &merged,
                inject.as_ref().map(|(k, f)| (k.as_str(), *f)),
            );
            for cell in &report.cells {
                match cell.state {
                    tebako_bench::covenant::State::Pass => println!(
                        "PASS  {} {:.3} {} {:.3}",
                        cell.id,
                        cell.measured.unwrap_or_default(),
                        cell.comparison,
                        cell.bound
                    ),
                    tebako_bench::covenant::State::Over => {
                        println!(
                            "OVER  {} {:.3} {} {:.3}{}",
                            cell.id,
                            cell.measured.unwrap_or_default(),
                            cell.comparison,
                            cell.bound,
                            match cell.injected {
                                Some(v) => format!(" (injected: measured {v:.3})"),
                                None => String::new(),
                            }
                        );
                        // The fleet witness (tebako#1071): a trip names its
                        // class and the leg the witness read — adjudication
                        // is a read of this output, not an archaeology dig.
                        println!("        {}", witness_line(cell));
                        if let (Some(class), Some(note)) = (cell.class, &cell.class_note) {
                            println!("        class: {} — {note}", class.token());
                        }
                    }
                    tebako_bench::covenant::State::Absent => {
                        println!("ABSENT {} — the run carries no sample", cell.id)
                    }
                }
            }
            if let Some(path) = &json {
                let text = serde_json::to_string_pretty(&report).map_err(|e| {
                    BenchError::operational(format!("cannot render the verdict: {e}"))
                })?;
                std::fs::write(path, text).map_err(|e| {
                    BenchError::operational(format!("cannot write {}: {e}", path.display()))
                })?;
            }
            println!(
                "covenant: {} ({} over{}, {} absent)",
                report.verdict.to_uppercase(),
                report
                    .cells
                    .iter()
                    .filter(|c| c.state == tebako_bench::covenant::State::Over)
                    .count(),
                class_tally(&report),
                report
                    .cells
                    .iter()
                    .filter(|c| c.state == tebako_bench::covenant::State::Absent)
                    .count()
            );
            Ok(if report.passes() {
                exit::OK
            } else {
                exit::INVALID
            })
        }
        Command::Validate { kind, file } => {
            let violations = validate::validate_file(kind, &file)?;
            if violations.is_empty() {
                println!("{}: VALID", file.display());
                Ok(exit::OK)
            } else {
                for v in &violations {
                    eprintln!("{}: {v}", file.display());
                }
                eprintln!(
                    "{}: INVALID ({} violation(s))",
                    file.display(),
                    violations.len()
                );
                Ok(exit::INVALID)
            }
        }
        Command::Run {
            suite,
            platforms,
            triplet,
            out,
            opt_in,
            tebako_release,
            repo_root,
        } => {
            let suite_text = std::fs::read_to_string(&suite).map_err(|e| {
                BenchError::operational(format!("cannot read {}: {e}", suite.display()))
            })?;
            let suite = tebako_bench::SuiteFile::from_yaml(&suite_text).map_err(|e| {
                BenchError::operational(format!("cannot parse {}: {e}", suite.display()))
            })?;
            let platforms_text = std::fs::read_to_string(&platforms).map_err(|e| {
                BenchError::operational(format!("cannot read {}: {e}", platforms.display()))
            })?;
            let platforms =
                tebako_bench::PlatformFile::from_yaml(&platforms_text).map_err(|e| {
                    BenchError::operational(format!("cannot parse {}: {e}", platforms.display()))
                })?;
            tebako_bench::engine::run(&tebako_bench::engine::RunRequest {
                suite,
                platforms,
                triplet,
                out,
                opt_in,
                tebako_release,
                repo_root,
            })
        }
        Command::Report {
            results,
            md,
            json,
            metrics,
            covenant,
            release,
            history,
        } => {
            let has_metrics = metrics.is_some();
            tebako_bench::report::report(&tebako_bench::report::ReportRequest {
                results,
                md,
                json,
                metrics,
                covenant: if has_metrics { Some(covenant) } else { None },
                release,
                history,
            })
        }
        Command::Trend {
            release,
            date,
            dashboard,
            previous,
            out,
        } => tebako_bench::trend::trend(&tebako_bench::trend::TrendRequest {
            release,
            date,
            dashboards: dashboard,
            previous,
            out,
        }),
    }
}
