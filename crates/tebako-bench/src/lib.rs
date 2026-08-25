//! tebako-bench — the spec 27 benchmark harness (v2 tebako vs v1 packed-mn).
//!
//! **CI TOOLING, NEVER SHIPPED.** This crate is NOT part of
//! `.github/workflows/release.yml`'s shipped binary set and must never be
//! added to it, never published to a release page, never installed by any
//! installer. It drives the product binaries; it is not one of them. The
//! audience law is unaffected: nothing a user or payload developer runs
//! involves this crate.
//!
//! Constraints that follow (spec 27 §0):
//!
//! - **Pure Rust, no vcpkg** — no tfs/dwarfs/sqfs/rnp dependency, so the
//!   crate builds in the pure-Rust CI legs (incl. `test-windows`).
//! - **No shell-outs, ever** (spec 00 invariant 1 applies to tooling by
//!   choice): downloads are in-process HTTP, archive handling in-process,
//!   measurement in-process FFI. That uniformity is why ONE implementation
//!   serves all seven triplets, musl and Windows included.
//! - **Named errors, named exit codes** (invariant 9): the exit surface is
//!   spec 27 §8 — 0 success/valid, 1 invalid/all-arms-failed, 2 operational
//!   (including the not-implemented stubs).

pub mod error;
pub mod platforms;
pub mod result;
pub mod sampler;
pub mod suite;
mod sys;
pub mod validate;

pub use error::BenchError;
pub use platforms::{PlatformFile, Triplet};
pub use result::{ResultFile, RunRecord, StatRecord};
pub use suite::{RunPolicy, SuiteFile, Target, TargetKind, Workload};
pub use validate::{validate_file, validate_text, DocKind};

/// Total physical RAM of the runner in bytes (`runner.ram_bytes`, spec 27
/// §6). The platform mechanics live in `crate::sys` (FFI-quarantined).
pub fn ram_total_bytes() -> Result<u64, BenchError> {
    sys::ram_total_bytes()
}

/// The process exit codes (spec 27 §8).
pub mod exit {
    /// Success; for `validate`: the document is VALID.
    pub const OK: u8 = 0;
    /// `validate`: the document is INVALID. `run`/`report`: completed with
    /// every arm failed/unavailable (artifacts still written).
    pub const INVALID: u8 = 1;
    /// Operational error (I/O, parse, unknown kind, not implemented).
    pub const OPERATIONAL: u8 = 2;
}
