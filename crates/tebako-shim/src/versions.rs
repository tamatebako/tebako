//! Dotted-version comparison and requirement-constraint matching
//! (spec 05 §5 compatibility model) — OWNED by `tpkg::versions` (the
//! single home the driver also consumes for spec 30's spawn-time edge
//! resolution); this module re-exports it so the shim's call sites and
//! test surface are unchanged.

pub use tpkg::versions::{compare, from_validated, newest, parse_constraint, Constraint};
