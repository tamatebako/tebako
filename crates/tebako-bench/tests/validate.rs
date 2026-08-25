//! Golden-fixture + cross-check tests for `tebako-bench validate`
//! (spec 27 §8): every valid fixture passes BOTH gates (the embedded
//! versioned JSON Schema and the serde model), every invalid fixture fails
//! with named violations, the models round-trip, and the repo's authored
//! documents (benchmarks/suite.yaml, benchmarks/platforms.yaml) validate —
//! the schema and the model stay MECE by construction here (the tpkg
//! pattern).

use tebako_bench::platforms::PlatformFile;
use tebako_bench::result::ResultFile;
use tebako_bench::suite::SuiteFile;
use tebako_bench::validate::{self, DocKind};

fn fixture_path(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn repo_path(name: &str) -> String {
    format!("{}/../../{name}", env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn embedded_schemas_compile() {
    for (name, text) in [
        ("suite", validate::SUITE_SCHEMA),
        ("result", validate::RESULT_SCHEMA),
    ] {
        let schema: serde_json::Value =
            serde_json::from_str(text).unwrap_or_else(|e| panic!("{name} schema json: {e}"));
        jsonschema::validator_for(&schema)
            .unwrap_or_else(|e| panic!("{name} schema compiles: {e}"));
    }
}

#[test]
fn valid_fixtures_pass_both_gates() {
    for (kind, name) in [
        (DocKind::Suite, "suite-valid.yaml"),
        (DocKind::Result, "result-valid.json"),
    ] {
        let violations = validate::validate_text(kind, &read(&fixture_path(name)))
            .unwrap_or_else(|e| panic!("{name}: operational error: {e}"));
        assert!(
            violations.is_empty(),
            "{name} must be VALID, violations: {violations:?}"
        );
    }
}

#[test]
fn invalid_fixtures_are_named() {
    for (kind, name, needle) in [
        (DocKind::Suite, "suite-invalid-floating-ref.yaml", "ref"),
        (
            DocKind::Suite,
            "suite-invalid-v2-missing-payload.yaml",
            "payload",
        ),
        (
            DocKind::Result,
            "result-invalid-gap-no-reason.json",
            "reason",
        ),
        (
            DocKind::Result,
            "result-invalid-ok-missing-metrics.json",
            "wall_s",
        ),
    ] {
        let violations = validate::validate_text(kind, &read(&fixture_path(name)))
            .unwrap_or_else(|e| panic!("{name}: operational error: {e}"));
        assert!(
            !violations.is_empty(),
            "{name} must be INVALID but validated clean"
        );
        let joined = violations.join("\n");
        assert!(
            joined.contains(needle),
            "{name}: a violation must name '{needle}', got:\n{joined}"
        );
    }
}

#[test]
fn models_round_trip() {
    let suite_text = read(&fixture_path("suite-valid.yaml"));
    let suite = SuiteFile::from_yaml(&suite_text).expect("suite parses");
    let rendered = serde_yml::to_string(&suite).expect("suite renders");
    let reparsed = SuiteFile::from_yaml(&rendered).expect("suite re-parses");
    assert_eq!(suite, reparsed, "suite YAML round-trip");

    let result_text = read(&fixture_path("result-valid.json"));
    let result = ResultFile::from_json(&result_text).expect("result parses");
    let rendered = result.to_json().expect("result renders");
    let reparsed = ResultFile::from_json(&rendered).expect("result re-parses");
    assert_eq!(result, reparsed, "result JSON round-trip");
}

/// The authored SSOT documents must validate against their own contracts —
/// a suite that fails its schema is a bug on arrival.
#[test]
fn repo_authored_documents_validate() {
    let violations =
        validate::validate_text(DocKind::Suite, &read(&repo_path("benchmarks/suite.yaml")))
            .expect("suite.yaml: operational");
    assert!(
        violations.is_empty(),
        "benchmarks/suite.yaml must be VALID, violations: {violations:?}"
    );

    // platforms.yaml has no JSON Schema artifact in this revision (spec 27
    // §3): the serde model + its semantic rules are the gate.
    let platforms_text = read(&repo_path("benchmarks/platforms.yaml"));
    let platforms =
        PlatformFile::from_yaml(&platforms_text).expect("platforms.yaml parses the model");
    assert!(
        platforms.semantic_violations().is_empty(),
        "platforms.yaml semantic violations"
    );
    assert_eq!(
        platforms.triplets.len(),
        7,
        "the release vocabulary's seven triplets"
    );
}

/// MECE: the schema gate and the model gate must agree on validity for
/// every fixture — a disagreement is a harness bug, not a document verdict.
#[test]
fn schema_and_model_gates_agree() {
    for (kind, name) in [
        (DocKind::Suite, "suite-valid.yaml"),
        (DocKind::Suite, "suite-invalid-floating-ref.yaml"),
        (DocKind::Suite, "suite-invalid-v2-missing-payload.yaml"),
        (DocKind::Result, "result-valid.json"),
        (DocKind::Result, "result-invalid-gap-no-reason.json"),
        (DocKind::Result, "result-invalid-ok-missing-metrics.json"),
    ] {
        let text = read(&fixture_path(name));
        let violations = validate::validate_text(kind, &text).expect("operational");
        let schema_failed = violations.iter().any(|v| v.starts_with("schema:"));
        // The model gate is the serde parse AND the semantic rules — both
        // are the crate's side of the contract.
        let model_failed = violations
            .iter()
            .any(|v| v.starts_with("model:") || v.starts_with("semantic:"));
        assert_eq!(
            schema_failed, model_failed,
            "{name}: gates disagree — violations: {violations:?}"
        );
    }
}
