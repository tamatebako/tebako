//! Golden-fixture tests for the package manifest (spec 03 §6): the
//! fixture parses, validates, round-trips, AND validates against the
//! versioned JSON Schema `schema/tpkg-package-manifest-v1.schema.json`
//! (the schema and the serde model are kept MECE with each other by this
//! cross-check — same discipline as the payload manifest).

use tpkg::*;

fn fixture_path(name: &str) -> String {
    format!(
        "{}/tests/fixtures/manifests/{name}.yaml",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn schema_path() -> String {
    format!(
        "{}/../../schema/tpkg-package-manifest-v1.schema.json",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

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
fn fixture_parses_and_validates() {
    let text = read(&fixture_path("package-suite"));
    PackageManifest::from_yaml(&text).unwrap_or_else(|e| panic!("fixture: {e}"));
}

#[test]
fn fixture_validates_against_the_json_schema() {
    let schema: serde_json::Value =
        serde_json::from_str(&read(&schema_path())).expect("schema json");
    let validator = jsonschema::validator_for(&schema).expect("the schema itself compiles");
    let value: serde_yml::Value =
        serde_yml::from_str(&read(&fixture_path("package-suite"))).expect("yaml parses");
    validator
        .validate(&yaml_to_json(&value))
        .unwrap_or_else(|e| panic!("fixture against the JSON schema: {e}"));
}

#[test]
fn fixture_round_trips_through_yaml() {
    let text = read(&fixture_path("package-suite"));
    let manifest = PackageManifest::from_yaml(&text).unwrap();
    let rendered = manifest.to_yaml().unwrap();
    let reparsed = PackageManifest::from_yaml(&rendered).unwrap();
    assert_eq!(reparsed, manifest);
}

#[test]
fn fixture_shape_is_the_spec_example() {
    let text = read(&fixture_path("package-suite"));
    let m = PackageManifest::from_yaml(&text).unwrap();
    assert_eq!(m.schema_version, PACKAGE_SCHEMA_VERSION);
    assert_eq!(m.package.name, "metanorma");
    assert_eq!(m.package.version, "1.2.3");
    assert_eq!(m.package.producer.tool, "tebako-cli");
    assert_eq!(m.entries.len(), 2);
    assert_eq!(m.entries[0].name, "metanorma");
    assert_eq!(m.entries[0].slot, 0);
    assert_eq!(m.entries[0].entrypoint, "metanorma");
    assert_eq!(m.entries[0].runtime_ref, "ruby@3.4.2;tebako=0.15.9");
    assert_eq!(m.entries[1].name, "mn2pdf");
    assert_eq!(m.entries[1].slot, 1);
    assert_eq!(m.entries[1].runtime_ref, "ruby@3.3.7;tebako=0.15.9");
    assert!(m.jail.is_some());
    assert_eq!(
        m.env.get("GEM_HOME").map(String::as_str),
        Some("/__tebako__/gems")
    );
}

#[test]
fn per_entry_runtime_refs_exceed_the_trailer_field_limit() {
    // The point of the block: per-entry refs with no 128-byte cap.
    let text = read(&fixture_path("package-suite"));
    let mut m = PackageManifest::from_yaml(&text).unwrap();
    let long_ref = format!("ruby@3.4.2;tebako=0.15.9;sha256={}", "ab".repeat(96));
    assert!(long_ref.len() > TPKG_RUNTIME_REF_LEN);
    m.entries[0].runtime_ref = long_ref.clone();
    let back = PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
    assert_eq!(back.entries[0].runtime_ref, long_ref);
}
