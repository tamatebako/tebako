//! Golden-fixture tests for the D2 composition document (spec 23 §3/§13):
//! the spec's example parses through the crate AND validates against the
//! versioned JSON Schema `schema/tebako-compose-v1.schema.json` (the schema
//! and the serde model are kept MECE with each other by this cross-check —
//! same discipline as the payload and package manifests).

use tpkg::*;

fn schema_path() -> String {
    format!(
        "{}/../../schema/tebako-compose-v1.schema.json",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// The spec 23 §3 example (the shared-runtime preset with one carried and
/// one shared slice, a universal platforms assertion, and the pointer
/// form's entry selector).
const SPEC_EXAMPLE: &str = "\
version: 1
preset: shared-runtime
runtime:
  name: ruby
  requirement: \"~> 3.3\"
  carry: false
  platforms: [macos-arm64, linux-gnu-x86_64]
slices:
  - name: metanorma
    requirement: \">= 2.1\"
    carry: true
  - ref: \"ourorg-templates@3\"
    carry: false
    platforms: universal
entrypoint: mnconvert
";

/// YAML value -> JSON value (for the jsonschema cross-check).
fn yaml_to_json(v: &serde_yml::Value) -> serde_json::Value {
    match v {
        serde_yml::Value::Null => serde_json::Value::Null,
        serde_yml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::from(i)
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::from(u)
            } else {
                serde_json::Value::from(n.as_f64().unwrap())
            }
        }
        serde_yml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json).collect())
        }
        serde_yml::Value::Mapping(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                let serde_yml::Value::String(key) = k else {
                    panic!("non-string mapping key {k:?}");
                };
                out.insert(key.clone(), yaml_to_json(v));
            }
            serde_json::Value::Object(out)
        }
        serde_yml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

#[test]
fn spec_example_validates_against_the_json_schema() {
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(schema_path()).expect("read schema"))
            .expect("schema json");
    let validator = jsonschema::validator_for(&schema).expect("the schema itself compiles");
    let value: serde_yml::Value = serde_yml::from_str(SPEC_EXAMPLE).expect("yaml parses");
    validator
        .validate(&yaml_to_json(&value))
        .unwrap_or_else(|e| panic!("the spec 23 example against the JSON schema: {e}"));
}

#[test]
fn the_schema_refuses_the_phase_r_keys_too() {
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(schema_path()).expect("read schema"))
            .expect("schema json");
    let validator = jsonschema::validator_for(&schema).expect("the schema itself compiles");
    for key in ["policy", "mounts", "needs"] {
        let doc = format!("version: 1\nruntime: {{ref: \"ruby@~> 3.3\"}}\n{key}: null\n");
        let value: serde_yml::Value = serde_yml::from_str(&doc).expect("yaml parses");
        assert!(
            validator.validate(&yaml_to_json(&value)).is_err(),
            "the schema must refuse the Phase-R key {key:?}"
        );
    }
}

#[test]
fn schema_and_crate_agree_on_the_example() {
    let (doc, warnings) = parse_compose(SPEC_EXAMPLE).expect("the crate parses the example");
    assert!(warnings.is_empty());
    assert_eq!(doc.preset, ComposePreset::SharedRuntime);
    assert_eq!(
        doc.runtime.platforms,
        Some(Platforms::Triplets(vec![
            Platform::Aarch64Macos,
            Platform::X86_64LinuxGnu
        ]))
    );
    assert_eq!(doc.slices[1].platforms, Some(Platforms::Universal));
}
