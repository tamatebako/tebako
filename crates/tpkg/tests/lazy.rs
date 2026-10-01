//! The spec 39 lazy models cross-checked against their versioned JSON
//! Schemas (`schema/tpkg-blksum-v1.schema.json`,
//! `schema/tpkg-lazy-seed-v1.schema.json`): every rendered document
//! validates, and the schemas REJECT the malformed classes the parsers
//! name — the schema and the Rust model stay MECE with each other.

use tpkg::lazy::{Blksum, LazySeed, LAZY_GROUP_SIZE};

const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn schema(name: &str) -> jsonschema::Validator {
    let path = format!("{}/../../schema/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let json: serde_json::Value = serde_json::from_str(&text).expect("schema json");
    jsonschema::validator_for(&json).expect("the schema itself compiles")
}

fn instance(rendered: &str) -> serde_json::Value {
    serde_json::from_str(rendered).expect("the render is json")
}

#[test]
fn blksum_render_validates_against_the_schema() {
    let validator = schema("tpkg-blksum-v1.schema.json");
    let bytes: Vec<u8> = (0..(LAZY_GROUP_SIZE + 100) as usize)
        .map(|i| (i % 253) as u8)
        .collect();
    let sum = Blksum::from_image_bytes(&bytes);
    let doc = instance(&sum.render());
    assert!(
        validator.is_valid(&doc),
        "the rendered sidecar validates: {:?}",
        validator.iter_errors(&doc).collect::<Vec<_>>()
    );
    // The schema rejects what it can see (the digest/count-vs-size
    // cross-field math is the parser's check — JSON Schema cannot
    // spell it).
    let bad_digest: serde_json::Value = serde_json::from_str(&format!(
        "{{\"schema_version\":1,\"group_size\":4194304,\"size_bytes\":100,\"sha256\":\"{HEX}\",\"groups\":[\"not-hex\"]}}"
    ))
    .unwrap();
    assert!(!validator.is_valid(&bad_digest));
    let empty_groups: serde_json::Value = serde_json::from_str(&format!(
        "{{\"schema_version\":1,\"group_size\":4194304,\"size_bytes\":100,\"sha256\":\"{HEX}\",\"groups\":[]}}"
    ))
    .unwrap();
    assert!(!validator.is_valid(&empty_groups));
    let bad_size = sum.render().replace("4194304", "1048576");
    assert!(!validator.is_valid(&instance(&bad_size)));
}

#[test]
fn lazy_seed_render_validates_against_the_schema() {
    let validator = schema("tpkg-lazy-seed-v1.schema.json");
    let seed = LazySeed {
        source: "https://releases.example.test/tebako-runtime.tfs".to_string(),
        sha256: HEX.to_string(),
        blksum_sha256: HEX.to_string(),
        size_bytes: 2 * LAZY_GROUP_SIZE + 7,
        group_count: 3,
    };
    let doc = instance(&seed.render());
    assert!(
        validator.is_valid(&doc),
        "the rendered descriptor validates: {:?}",
        validator.iter_errors(&doc).collect::<Vec<_>>()
    );
    // The schema rejects what it can see (cross-field count math is
    // the parser's check — JSON Schema cannot spell it).
    let missing = seed.render().replace("\"blksum_sha256\":", "\"blksum\":");
    assert!(!validator.is_valid(&instance(&missing)));
    let bad_version = seed
        .render()
        .replace("\"schema_version\":1", "\"schema_version\":2");
    assert!(!validator.is_valid(&instance(&bad_version)));
}
