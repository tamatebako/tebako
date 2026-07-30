//! The spec-17 runtime driver (language-agnostic).
//!
//! Linked into every tebako runtime executable, the driver performs the
//! loader-side half of the launcher handoff BEFORE the interpreter starts:
//!
//! ```text
//! <runtime> --tebako-image <self|image-path>:<slot|->:<mount> ...
//!           --tebako-entry <argv0> <user args...>
//! ```
//!
//! - the **env image** (`TEBAKO_RUNTIME_IMAGE`, a bare `.tfs`) mounts whole
//!   at the runtime root the interpreter was compiled against;
//! - each **payload triple** mounts its image: a bare file whole (slot `0`
//!   ≡ `-`), or a package file's trailer-described slot region;
//! - `TEBAKO_JAIL` is parsed and installed (after the mounts — the mount
//!   family's image read is itself policy-gated once a policy is active);
//! - argv is rewritten to `[<original argv0>, <entry resolved in the VFS>,
//!   <user args…>]` — the program name stays at index 0 so the
//!   interpreter takes the entry as its script.
//!
//! Any failure unmounts everything and returns a named error carrying the
//! loader's exit codes (spec 06 §4): never a partial mount, never a
//! crash. The interpreter's `main` calls [`ffi::tebako_driver_boot`]
//! first and continues with the rewritten argv on success.
//!
//! This crate is the Rust successor of the v1 C++ `tebako-main` driver:
//! no embedded-image knowledge (the image era), no `/local/stub.rb`
//! convention, multi-mount by construction.

#![deny(unsafe_code)]

pub mod driver;
pub mod ffi;
pub mod handoff;

pub use driver::{boot, BootOutcome, DriverError, Env, ProcessEnv};
pub use handoff::{Handoff, ImageSource, ImageSpec, SlotRef};

/// The bootstrap↔runtime contract semantics this driver implements
/// (spec 06 §6): spec 17's widened grammar — image-path triples,
/// bare-file slot tokens (`0` ≡ `-`), env-image-first multi-mount, and
/// direct `--tebako-entry` execution. Compiled into the runtime and
/// declared in its release manifest; the loader refuses anything newer.
pub const TEBAKO_CONTRACT_VERSION: u32 = 2;

// The loader's named exit codes (spec 06 §4) the driver reports with.
pub(crate) const EX_TEBAKO_MANIFEST: i32 = 65;
pub(crate) const EX_TEBAKO_UNAVAILABLE: i32 = 69;
pub(crate) const EX_TEBAKO_JAIL: i32 = 73;
pub(crate) const EX_TEBAKO_IO: i32 = 74;
