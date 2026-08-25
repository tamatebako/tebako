//! The crate's named error (the workspace shape: message + exit code,
//! mirroring tebako-cli's TebakoError). Validation findings are NOT errors
//! in this sense — they are the `Ok(Vec<String>)` payload of
//! `validate_text`; a BenchError is always an operational failure
//! (spec 27 §8 exit 2).

use std::fmt;

#[derive(Debug)]
pub struct BenchError {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for BenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BenchError {}

impl BenchError {
    /// An operational failure (spec 27 §8 exit 2): unreadable input,
    /// schema/model disagreement, an unimplemented surface, I/O.
    pub fn operational(message: impl Into<String>) -> Self {
        BenchError {
            message: message.into(),
            code: i32::from(crate::exit::OPERATIONAL),
        }
    }

    /// The run/report stubs before their slices land: named, never a bare
    /// exit (invariant 9).
    pub fn not_implemented(surface: &str, slice: &str) -> Self {
        BenchError::operational(format!(
            "`tebako-bench {surface}` is not implemented yet (planned: {slice} of the spec 27 benchmark plan)"
        ))
    }
}
