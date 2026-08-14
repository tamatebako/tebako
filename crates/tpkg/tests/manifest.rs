//! Golden-fixture tests for the payload manifest (spec 03): every fixture
//! parses, validates, round-trips, AND validates against the versioned
//! JSON Schema `schema/tpkg-manifest-v1.schema.json` (the schema and the
//! serde model are kept MECE with each other by this cross-check).

use tpkg::*;

const FIXTURES: [&str; 3] = ["runtime", "app-suite", "data"];

fn fixture_path(name: &str) -> String {
    format!(
        "{}/tests/fixtures/manifests/{name}.yaml",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn schema_path() -> String {
    format!(
        "{}/../../schema/tpkg-manifest-v1.schema.json",
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

fn yaml_text_to_json(text: &str) -> serde_json::Value {
    let value: serde_yml::Value = serde_yml::from_str(text).expect("yaml parses");
    yaml_to_json(&value)
}

fn schema_validator() -> jsonschema::Validator {
    let schema: serde_json::Value =
        serde_json::from_str(&read(&schema_path())).expect("schema json");
    jsonschema::validator_for(&schema).expect("the schema itself compiles")
}

#[test]
fn fixtures_parse_and_validate() {
    for name in FIXTURES {
        let text = read(&fixture_path(name));
        PayloadManifest::from_yaml(&text).unwrap_or_else(|e| panic!("fixture {name}: {e}"));
    }
}

#[test]
fn fixtures_validate_against_the_json_schema() {
    let validator = schema_validator();
    for name in FIXTURES {
        let instance = yaml_text_to_json(&read(&fixture_path(name)));
        validator
            .validate(&instance)
            .unwrap_or_else(|e| panic!("fixture {name} against the JSON schema: {e}"));
    }
}

#[test]
fn fixtures_round_trip_through_yaml() {
    for name in FIXTURES {
        let text = read(&fixture_path(name));
        let manifest = PayloadManifest::from_yaml(&text).unwrap();
        let rendered = manifest.to_yaml().unwrap();
        let reparsed = PayloadManifest::from_yaml(&rendered).unwrap();
        assert_eq!(&reparsed, &manifest, "fixture {name} round-trip");
    }
}

#[test]
fn runtime_fixture_shape() {
    let text = read(&fixture_path("runtime"));
    let m = PayloadManifest::from_yaml(&text).unwrap();
    assert_eq!(m.identity.kind, PayloadKind::Runtime);
    assert_eq!(m.identity.name, "tebako-runtime-ruby");
    assert_eq!(m.identity.version, "4.0.6");
    let Provides::Runtime(rt) = &m.provides else {
        panic!("runtime provides, got {:?}", m.provides);
    };
    // two triplets in one image (universal2 macOS)
    assert_eq!(rt.provides.len(), 2);
    assert_eq!(rt.provides[0].engine, "ruby");
    assert_eq!(rt.provides[0].version, "4.0.6");
    assert_eq!(rt.provides[0].abi_line, "4.0");
    assert_eq!(rt.provides[0].platform, Platform::Aarch64Macos);
    assert_eq!(rt.provides[1].platform, Platform::X86_64Macos);
    assert_eq!(rt.built_from.patch_set, "v0.2.8");
    assert_eq!(rt.env.get("GEM_HOME").unwrap(), "/__tebako__/gems");
    assert_eq!(rt.capabilities.runtime, Some(true));
    // signed identity
    assert_eq!(m.identity.signing.state, SigningState::Signed);
    assert_eq!(
        m.identity.signing.keyid.as_deref(),
        Some("0123456789abcdef")
    );
    assert_eq!(
        m.identity.signing.mechanism,
        Some(SigningMechanism::Openpgp)
    );
    // no DEPENDS block
    assert!(m.requires.is_empty());
    // annotations with a NON-string value survive
    assert_eq!(
        m.identity.annotations.get("ci/build-number"),
        Some(&serde_yml::Value::from(4711))
    );
}

#[test]
fn app_suite_fixture_shape() {
    let text = read(&fixture_path("app-suite"));
    let m = PayloadManifest::from_yaml(&text).unwrap();
    assert_eq!(m.identity.kind, PayloadKind::App);
    let Provides::App(app) = &m.provides else {
        panic!("app provides, got {:?}", m.provides);
    };
    // TWO entrypoints with DIFFERENT runtime requirements
    assert_eq!(app.entrypoints.len(), 2);
    let [e1, e2] = &app.entrypoints[..] else {
        panic!()
    };
    assert_eq!(e1.name, "metanorma");
    assert_eq!(
        e1.runtime_requirement.as_ref().unwrap().constraint.as_str(),
        ">= 3.3, < 5.0"
    );
    assert_eq!(e2.name, "metanorma-nokogiri");
    assert_eq!(
        e2.runtime_requirement.as_ref().unwrap().constraint.as_str(),
        "~> 3.3.0"
    );
    assert_eq!(e1.args_default, vec!["--format", "pretty"]);
    assert_eq!(
        app.platforms,
        Platforms::Triplets(vec![Platform::Aarch64Macos, Platform::X86_64LinuxGnu])
    );
    // DEPENDS: a language edge and ONE toolkit dep with consumer-declared mount
    assert_eq!(m.requires.len(), 2);
    assert_eq!(
        m.requires[0],
        Requirement::Language {
            engine: "ruby".into(),
            constraint: Constraint::new("~> 3.3.0").unwrap(),
        }
    );
    let Requirement::Toolkit {
        name,
        constraint,
        triplets,
        mount,
    } = &m.requires[1]
    else {
        panic!("toolkit requirement, got {:?}", m.requires[1]);
    };
    assert_eq!(name, "gtk-layer");
    assert_eq!(constraint.as_str(), ">= 3.24, < 3.25");
    assert_eq!(
        triplets.as_deref(),
        Some(&[Platform::Aarch64Macos, Platform::X86_64LinuxGnu][..])
    );
    // the MOUNT RULE: the consumer declares the mount point
    assert_eq!(mount.as_deref(), Some("/__layers__/gtk"));
}

#[test]
fn native_entrypoints_omit_runtime_requirement() {
    // spec 03 §2.2 (locked): an entrypoint whose executable is native (or
    // self-contained) declares NO runtime_requirement — the dispatcher
    // mounts zero runtime payloads. The key is omitted on the wire.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: inkview\n  version: \"1.3\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-07-27T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - name: inkview\n      path: /bin/inkview\n\
        \x20 platforms: [x86_64-linux-gnu]\n  capabilities: {exec: true, read: true}\n";
    let m = PayloadManifest::from_yaml(text).unwrap();
    let Provides::App(app) = &m.provides else {
        panic!("app provides, got {:?}", m.provides);
    };
    assert_eq!(app.entrypoints.len(), 1);
    assert!(app.entrypoints[0].runtime_requirement.is_none());
    // serialization omits the key (never writes a null)
    let yaml = m.to_yaml().unwrap();
    assert!(
        !yaml.contains("runtime_requirement"),
        "omitted on the wire: {yaml}"
    );
    // …and the schema agrees (MECE cross-check)
    assert!(schema_validator().is_valid(&yaml_text_to_json(text)));
}

#[test]
fn data_fixture_shape() {
    let text = read(&fixture_path("data"));
    let m = PayloadManifest::from_yaml(&text).unwrap();
    assert_eq!(m.identity.kind, PayloadKind::Data);
    // datever stays a free-form string
    assert_eq!(m.identity.version, "2024.11");
    let Provides::Data(data) = &m.provides else {
        panic!("data provides, got {:?}", m.provides);
    };
    assert_eq!(data.mount_semantics.suggested, "/usr/share/fonts");
    assert_eq!(data.consumers, vec!["metanorma"]);
    assert!(!data.capabilities.exec);
    // per-part encryption: references, NEVER keys
    assert_eq!(m.identity.encryption.state, EncryptionState::Encrypted);
    assert_eq!(m.identity.encryption.parts.len(), 1);
    let part = &m.identity.encryption.parts[0];
    assert_eq!(part.paths, vec!["/fonts/licensed/"]);
    assert_eq!(part.algorithm, "age-x25519");
    assert_eq!(part.envelope_refs, vec!["vault:tebako/fonts-dek#3"]);
}

#[test]
fn materialize_key_is_schema_legal() {
    // spec 22 §4 class R (schema_minor 1): the additive `materialize:`
    // key — the versioned JSON Schema admits it and the model
    // round-trips it (the runtime env image's cert declaration is the
    // canonical shape).
    let text = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: 4.0.6\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-14T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: aarch64-macos}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n\
        materialize: [/lib/tebako/cacert.pem]\n";
    let m = PayloadManifest::from_yaml(text).unwrap();
    assert_eq!(m.materialize, vec!["/lib/tebako/cacert.pem".to_string()]);
    // …and the schema agrees (MECE cross-check).
    let validator = schema_validator();
    validator
        .validate(&yaml_text_to_json(text))
        .expect("materialize: is schema-legal");
    let back = PayloadManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
    assert_eq!(back, m);
}

#[test]
fn unknown_keys_are_tolerated_annotations_preserved() {
    let text = read(&fixture_path("data"));
    let with_extras = format!(
        "{text}\
         future_top_level: {{x: 1}}\n"
    );
    let mut m = PayloadManifest::from_yaml(&with_extras).expect("unknown top-level key tolerated");
    m.identity.annotations.insert(
        "custom/nested".into(),
        serde_yml::from_str("{list: [1, two, true], map: {k: v}}").unwrap(),
    );
    let rendered = m.to_yaml().unwrap();
    let reparsed = PayloadManifest::from_yaml(&rendered).unwrap();
    assert_eq!(
        reparsed, m,
        "nested annotation values survive the round-trip"
    );
}

#[test]
fn model_and_schema_agree_on_rejections() {
    let validator = schema_validator();
    let cases: [(&str, &str); 4] = [
        // kind app with a data-shaped provides
        (
            "identity: {schema_version: 1, kind: app, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: false, read: true}}\n",
            "kind/provides mismatch",
        ),
        // unsigned carrying a keyid
        (
            "identity: {schema_version: 1, kind: data, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned, keyid: 0123456789abcdef}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: false, read: true}}\n",
            "unsigned with keyid",
        ),
        // data capabilities violating the truth table
        (
            "identity: {schema_version: 1, kind: data, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: true, read: true}}\n",
            "data with exec: true",
        ),
        // the reserved triplet
        (
            "identity: {schema_version: 1, kind: data, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: false, read: true}}\n\
             requires: [{kind: toolkit, name: gtk, constraint: \">= 3\", \
             triplets: [aarch64-windows-ucrt]}]\n",
            "reserved triplet",
        ),
    ];
    for (text, what) in cases {
        assert!(
            PayloadManifest::from_yaml(text).is_err(),
            "model rejects {what}"
        );
        let instance = yaml_text_to_json(text);
        assert!(!validator.is_valid(&instance), "schema rejects {what}");
    }
}

#[test]
fn schema_is_coarse_where_the_model_is_strict() {
    // The constraint grammar is only presence-checked by the schema;
    // PayloadManifest::validate() (via Constraint's construction) is the gate.
    let text = "identity: {schema_version: 1, kind: app, name: x, version: \"1\", \
         producer: {tool: t, tool_version: \"1\"}, created: now, \
         digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
         blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
         signing: {state: unsigned}, encryption: {state: none}}\n\
         provides: {entrypoints: [{name: x, path: /x, \
         runtime_requirement: {engine: ruby, constraint: \"=> 3.3\"}}], \
         platforms: universal, capabilities: {exec: true, read: true}}\n";
    assert!(PayloadManifest::from_yaml(text).is_err());
    let validator = schema_validator();
    assert!(validator.is_valid(&yaml_text_to_json(text)));
}

#[test]
fn spec_03_smoke_examples_parse() {
    // The spec's inline shapes (§2.1's signing/encryption/sbom forms,
    // §2.3's data-dep edge) must parse through the same model.
    let text = "identity:\n  schema_version: 1\n  kind: language\n  name: ruby\n  version: 4.0.6\n\
         \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: 2026-07-26T00:00:00Z\n\
         \x20 sbom: {ref: \"sbom/x.json\"}\n\
         \x20 digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
         blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}\n\
         \x20 signing: {state: signed, keyid: 0123456789abcdef, mechanism: openpgp}\n\
         \x20 encryption: {state: none}\nprovides: {}\n\
         requires:\n  - kind: data\n    name: iso-codes\n    constraint: \">= 2024.1\"\n\
         \x20   mount: /__app__/share/iso-codes\n";
    let m = PayloadManifest::from_yaml(text).expect("spec smoke example");
    assert_eq!(m.identity.kind, PayloadKind::Language);
    assert!(matches!(m.provides, Provides::Other(_)));
    let Requirement::Data { name, mount, .. } = &m.requires[0] else {
        panic!()
    };
    assert_eq!(name, "iso-codes");
    assert_eq!(mount.as_deref(), Some("/__app__/share/iso-codes"));
}
