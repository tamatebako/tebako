//! `tebako-bench validate` — the two-gate document check (spec 27 §8):
//! the versioned JSON Schema (structure) AND the serde model (the shape the
//! run engine consumes), then the model's semantic rules. Every violation
//! is reported, one per line — never a bare exit (invariant 9).
//!
//! The schemas ride INSIDE the binary (include_str!) so an installed
//! copy never depends on a repo-relative path; the unit-test cross-check
//! exercises these same embedded bytes against the repo files.

use crate::error::BenchError;
use crate::result::ResultFile;
use crate::suite::SuiteFile;

pub const SUITE_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../schema/tebako-bench-suite-v1.schema.json"
));
pub const RESULT_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../schema/tebako-bench-result-v1.schema.json"
));

/// The document kinds validate understands (explicit `--kind`, never a
/// guess — invariant 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DocKind {
    Suite,
    Result,
}

impl DocKind {
    fn schema_text(&self) -> &'static str {
        match self {
            DocKind::Suite => SUITE_SCHEMA,
            DocKind::Result => RESULT_SCHEMA,
        }
    }
}

/// Validate a file on disk. I/O failure is operational (exit 2).
pub fn validate_file(kind: DocKind, path: &std::path::Path) -> Result<Vec<String>, BenchError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| BenchError::operational(format!("cannot read {}: {e}", path.display())))?;
    validate_text(kind, &text)
}

/// Validate a document, returning every violation found (empty = VALID).
///
/// Operational errors (a schema that fails to compile — a harness bug, not
/// the document's) come back as `Err`. Parse failures and gate
/// disagreements are violations (exit 1), prefixed with their gate.
pub fn validate_text(kind: DocKind, text: &str) -> Result<Vec<String>, BenchError> {
    let mut violations = Vec::new();

    // Gate 0: the document must parse. Suites are authored YAML; results
    // are machine-written JSON.
    let instance: serde_json::Value = match kind {
        DocKind::Suite => match serde_yml::from_str::<serde_yml::Value>(text) {
            Ok(v) => yaml_to_json(&v),
            Err(e) => {
                violations.push(format!("yaml: {e}"));
                return Ok(violations);
            }
        },
        DocKind::Result => match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                violations.push(format!("json: {e}"));
                return Ok(violations);
            }
        },
    };

    // Gate 1: the versioned JSON Schema (the structural contract).
    let schema: serde_json::Value = serde_json::from_str(kind.schema_text()).map_err(|e| {
        BenchError::operational(format!(
            "the embedded {} schema does not parse (harness bug): {e}",
            kind_name(kind)
        ))
    })?;
    let validator = jsonschema::validator_for(&schema).map_err(|e| {
        BenchError::operational(format!(
            "the embedded {} schema does not compile (harness bug): {e}",
            kind_name(kind)
        ))
    })?;
    for e in validator.iter_errors(&instance) {
        violations.push(format!("schema: {}: {e}", e.instance_path));
    }

    // Gate 2: the serde model (the run engine's shape). A schema-passing
    // document the model rejects is reported with its gate named — the
    // unit-test cross-check keeps the two MECE on the shipped shapes.
    match kind {
        DocKind::Suite => match SuiteFile::from_yaml(text) {
            Ok(suite) => violations.extend(
                suite
                    .semantic_violations()
                    .into_iter()
                    .map(|v| format!("semantic: {v}")),
            ),
            Err(e) => violations.push(format!("model: {e}")),
        },
        DocKind::Result => match ResultFile::from_json(text) {
            Ok(result) => violations.extend(
                result
                    .semantic_violations()
                    .into_iter()
                    .map(|v| format!("semantic: {v}")),
            ),
            Err(e) => violations.push(format!("model: {e}")),
        },
    }

    Ok(violations)
}

fn kind_name(kind: DocKind) -> &'static str {
    match kind {
        DocKind::Suite => "tebako-bench-suite-v1",
        DocKind::Result => "tebako-bench-result-v1",
    }
}

/// YAML value → JSON value (the schema gate speaks JSON).
fn yaml_to_json(v: &serde_yml::Value) -> serde_json::Value {
    match v {
        serde_yml::Value::Null => serde_json::Value::Null,
        serde_yml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::from(i)
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::from(u)
            } else if let Some(f) = n.as_f64() {
                serde_json::Value::from(f)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json).collect())
        }
        serde_yml::Value::Mapping(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                match k {
                    serde_yml::Value::String(key) => {
                        out.insert(key.clone(), yaml_to_json(v));
                    }
                    other => {
                        out.insert(format!("{other:?}"), yaml_to_json(v));
                    }
                }
            }
            serde_json::Value::Object(out)
        }
        serde_yml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}
