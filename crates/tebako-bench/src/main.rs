//! tebako-bench — the spec 27 benchmark harness. CI TOOLING, NEVER SHIPPED
//! (the boundary comment lives at the crate root; see lib.rs).
//!
//! ```text
//! tebako-bench run --suite <suite.yaml> --platforms <platforms.yaml>
//!                  --triplet <t> --out <dir> [--opt-in <workload-id>]...
//! tebako-bench report <results.json>... --md <report.md> --json <dashboard.json>
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
    about = "tebako benchmark harness (spec 27) — CI tooling, never shipped",
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
    /// (planned: the report-renderer slice).
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

fn run(cli: Cli) -> Result<u8, BenchError> {
    match cli.command {
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
        Command::Report { .. } => Err(BenchError::not_implemented(
            "report",
            "slice 6 (report renderer)",
        )),
    }
}
