//! The platform mechanics of the sampler — the crate's FFI boundary
//! (workspace rule: `unsafe` only inside boundary modules). `posix.rs`
//! and `windows.rs` export the same two functions; the sampler calls the
//! alias, never a platform module directly.

#[cfg(unix)]
pub mod posix;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub(crate) use posix::{ram_total_bytes, run_child};
#[cfg(windows)]
pub(crate) use windows::{ram_total_bytes, run_child};
