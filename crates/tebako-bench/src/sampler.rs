//! The benchmark sampler (spec 27 §4): spawn one child, measure wall /
//! CPU / peak RSS, enforce the timeout. Platform mechanics live in
//! `crate::sys` (FFI-quarantined); this module is the neutral surface.
//!
//! **The one-child-at-a-time discipline.** A `Sampler` measures exactly
//! one child at a time, and a benchmark process holds exactly one
//! `Sampler` (the run engine's). The rusage numbers are per-child
//! (POSIX `wait4` reaps the measured child itself — see
//! `sys::posix` for why not `RUSAGE_CHILDREN`), but the discipline still
//! stands: it keeps wall-time/timeout semantics clean, makes cross-run
//! attribution impossible by construction, and matches the Windows
//! handle model. Concurrent measured runs are forbidden even when
//! workloads are independent — cross-leg parallelism is the workflow
//! matrix's job, never the sampler's.

use std::path::PathBuf;
use std::time::Duration;

use crate::error::BenchError;

/// What to run and how to bound it.
#[derive(Debug, Clone)]
pub struct ChildSpec {
    /// argv[0] is the program; the rest are passed verbatim.
    pub argv: Vec<String>,
    /// The child's working directory (the per-run scratch dir).
    pub cwd: PathBuf,
    /// Overrides on the inherited environment (HOME/TMPDIR/… — the bench
    /// hermetic home, spec 27 §5).
    pub env: Vec<(String, String)>,
    /// stdout+stderr are appended to this file (one log per run).
    pub log_path: PathBuf,
    /// The workload's timeout. Expiry kills the child's tree
    /// (POSIX: the child's process group; Windows: the job object,
    /// falling back to the direct child) and records `timed_out`.
    pub timeout: Duration,
}

/// One measured run.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    /// Instant-elapsed seconds around spawn→reap.
    pub wall_s: f64,
    /// User CPU seconds attributed to the child.
    pub cpu_user_s: f64,
    /// System CPU seconds attributed to the child.
    pub cpu_sys_s: f64,
    /// Peak RSS in BYTES, always (unit-normalized at record time —
    /// `ru_maxrss` is KiB on Linux/musl, bytes on macOS; Windows's
    /// `PeakWorkingSetSize` is bytes).
    pub peak_rss_bytes: u64,
    /// The process exit status; 128+signal when signal-killed (the shell
    /// convention — a timeout kill records 137).
    pub exit: i32,
    /// True when the run was killed at the spec's timeout.
    pub timed_out: bool,
}

/// The one-per-process measured-run surface. Not `Clone`/`Sync`: the
/// discipline above is a type-level fact, not a comment.
pub struct Sampler {
    _seal: std::marker::PhantomData<*const ()>,
}

impl Sampler {
    pub fn new() -> Self {
        Sampler {
            _seal: std::marker::PhantomData,
        }
    }

    /// Run the child to completion (or timeout) and return its sample.
    pub fn run(&mut self, spec: &ChildSpec) -> Result<Sample, BenchError> {
        if spec.argv.is_empty() {
            return Err(BenchError::operational(
                "sampler: empty argv (argv[0] is the program)",
            ));
        }
        crate::sys::run_child(spec)
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}
