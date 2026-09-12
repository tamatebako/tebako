//! Golden-fixture tests for the payload manifest (spec 03): every fixture
//! parses, validates, round-trips, AND validates against the versioned
//! JSON Schema `schema/tpkg-manifest-v1.schema.json` (the schema and the
//! serde model are kept MECE with each other by this cross-check).

use tpkg::*;

const FIXTURES: [&str; 4] = ["runtime", "app-suite", "data", "executable-edge"];

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
        e1.runtime_requirement.as_ref().unwrap().entries()[0]
            .constraint
            .as_str(),
        ">= 3.3, < 5.0"
    );
    assert_eq!(e2.name, "metanorma-nokogiri");
    assert_eq!(
        e2.runtime_requirement.as_ref().unwrap().entries()[0]
            .constraint
            .as_str(),
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
            triplets: None,
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

/// An app manifest carrying `req_yaml` as the entrypoint's
/// runtime_requirement lines (indented for the provides.entrypoints
/// item — trailing newline included).
fn app_with_requirement(req_yaml: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: app\n  version: \"1.0\"\n  producer: {{tool: test, tool_version: \"1\"}}\n  created: \"2026-09-07T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: app\n      path: /bin/app\n{req_yaml}  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        "a".repeat(64),
        "b".repeat(64)
    )
}

#[test]
fn any_of_requirement_list_parses_and_round_trips() {
    // spec 28 §8 (schema_minor 7): the list form is `any_of` — OR in
    // declaration order — for pure-language payloads whose admissible set
    // differs per implementation.
    let text = app_with_requirement(
        "      runtime_requirement:\n        - {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n        - {engine: ruby, implementation: jruby, constraint: \"~> 9.5\"}\n",
    );
    let m = PayloadManifest::from_yaml(&text).unwrap();
    let Provides::App(app) = &m.provides else {
        panic!("app provides, got {:?}", m.provides)
    };
    let reqs = app.entrypoints[0].runtime_requirement.as_ref().unwrap();
    assert_eq!(reqs.entries().len(), 2);
    assert_eq!(reqs.engine(), "ruby");
    assert_eq!(reqs.entries()[0].implementation, None);
    assert_eq!(reqs.entries()[0].constraint.as_str(), ">= 3.3, < 5.0");
    assert_eq!(reqs.entries()[1].implementation.as_deref(), Some("jruby"));
    assert_eq!(reqs.entries()[1].constraint.as_str(), "~> 9.5");
    assert_eq!(reqs.to_string(), ">= 3.3, < 5.0 | ~> 9.5");
    // The list serializes as a list and re-parses to the same model…
    let yaml = m.to_yaml().unwrap();
    let reparsed = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(reparsed, m);
    // …and the JSON schema admits the list (MECE cross-check).
    assert!(schema_validator().is_valid(&yaml_text_to_json(&text)));
}

#[test]
fn the_single_requirement_map_never_sprouts_a_list() {
    // The long-standing single-map spelling: one entry serializes as the
    // bare map — existing manifests round-trip byte-shape-identically and
    // pre-schema_minor-7 readers keep parsing them.
    let text = app_with_requirement(
        "      runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n",
    );
    let m = PayloadManifest::from_yaml(&text).unwrap();
    let yaml = m.to_yaml().unwrap();
    assert!(
        !yaml.contains("- engine:"),
        "a bare map, never a one-element list: {yaml}"
    );
    let reparsed = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(reparsed, m);
    assert!(schema_validator().is_valid(&yaml_text_to_json(&text)));
    assert!(schema_validator().is_valid(&yaml_text_to_json(&yaml)));
}

#[test]
fn any_of_empty_list_is_a_named_manifest_error() {
    let text = app_with_requirement("      runtime_requirement: []\n");
    let err = PayloadManifest::from_yaml(&text).unwrap_err();
    assert!(
        err.to_string().contains("must not be empty"),
        "the empty list is named: {err}"
    );
}

#[test]
fn any_of_mixed_engines_is_a_named_manifest_error() {
    // spec 28 §8: the implementation is a sub-axis of the requirement —
    // every entry names the SAME engine; a mixed-engine list is never a
    // guessed union.
    let text = app_with_requirement(
        "      runtime_requirement:\n        - {engine: ruby, constraint: \">= 3.3\"}\n        - {engine: python, constraint: \">= 3.11\"}\n",
    );
    let err = PayloadManifest::from_yaml(&text).unwrap_err();
    assert!(
        err.to_string().contains("SAME engine"),
        "the mixed-engine list is named: {err}"
    );
}

#[test]
fn abi_without_implementation_is_a_named_authoring_error() {
    // spec 28 §8: an ABI is per-implementation by construction — a native
    // requirement must name its implementation. The rule disciplines NEW
    // authoring only: `tebako publish` (the registry-emission gate) is the
    // enforcement point (tebako#556).
    let text = app_with_requirement(
        "      runtime_requirement: {engine: ruby, constraint: \"~> 3.3.0\", abi: arm64-darwin-23}\n",
    );
    let err = PayloadManifest::from_yaml_authoring(&text).unwrap_err();
    assert!(
        err.to_string().contains("requires implementation")
            && err.to_string().contains("authoring rule"),
        "abi without implementation is named at publish: {err}"
    );
}

#[test]
fn pre_axis_abi_without_implementation_stays_dispatchable() {
    // tebako#556 (the schema evolution law, spec 18 §3): pre-axis published
    // manifests (metanorma 1.16.9-*, xml2rfc 3.34.0) declare `abi` without
    // `implementation` — a MINOR-era reader tightening that invalidates
    // them is forbidden. The consumer arm accepts the same document the
    // authoring gate refuses above.
    let text = app_with_requirement(
        "      runtime_requirement: {engine: ruby, constraint: \"~> 3.3.0\", abi: arm64-darwin-23}\n",
    );
    let m = PayloadManifest::from_yaml(&text).unwrap();
    let yaml = m.to_yaml().unwrap();
    let reparsed = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(reparsed, m);
    assert!(schema_validator().is_valid(&yaml_text_to_json(&text)));
}

#[test]
fn any_of_list_with_abi_is_a_named_manifest_error() {
    // spec 28 §8: a second implementation means a second build — the list
    // form is forbidden for a native-extension requirement.
    let text = app_with_requirement(
        "      runtime_requirement:\n        - {engine: ruby, implementation: mri, constraint: \"~> 3.3.0\", abi: arm64-darwin-23}\n        - {engine: ruby, constraint: \">= 3.3\"}\n",
    );
    let err = PayloadManifest::from_yaml(&text).unwrap_err();
    assert!(
        err.to_string().contains("list form is forbidden"),
        "the abi-carrying list is named: {err}"
    );
}

#[test]
fn native_requirement_with_implementation_round_trips() {
    // The native shape spec 28 §8 locks: implementation + abi on the one
    // entry, matched against the runtime's own version line.
    let text = app_with_requirement(
        "      runtime_requirement: {engine: ruby, implementation: mri, constraint: \"~> 3.3.0\", abi: arm64-darwin-23}\n",
    );
    let m = PayloadManifest::from_yaml(&text).unwrap();
    let yaml = m.to_yaml().unwrap();
    let reparsed = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(reparsed, m);
    // …and the authoring gate accepts the shape it demands
    // (implementation + abi on the one entry)
    PayloadManifest::from_yaml_authoring(&text).unwrap();
    assert!(schema_validator().is_valid(&yaml_text_to_json(&text)));
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
fn library_aliases_key_is_schema_legal() {
    // spec 03 §2.5 / spec 22 §2.1 (schema_minor 2): the additive
    // `library_aliases:` key — the versioned JSON Schema admits it and
    // the model round-trips it (the windows Class-L bare-name
    // declarations).
    let text = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: 4.0.6\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-15T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: x86_64-windows-ucrt}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n\
        library_aliases:\n  - {name: libfoo-3.dll, path: /lib/libfoo-3.dll}\n";
    let m = PayloadManifest::from_yaml(text).unwrap();
    assert_eq!(
        m.library_aliases,
        vec![LibraryAlias {
            name: "libfoo-3.dll".to_string(),
            path: "/lib/libfoo-3.dll".to_string(),
        }]
    );
    // …and the schema agrees (MECE cross-check).
    let validator = schema_validator();
    validator
        .validate(&yaml_text_to_json(text))
        .expect("library_aliases: is schema-legal");
    let back = PayloadManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
    assert_eq!(back, m);
}

#[test]
fn runtime_edge_is_schema_legal() {
    // spec 30 §1 (schema_minor 4): the additive `{kind: runtime}`
    // requires edge — the depended runtime resolves through the RUNTIME
    // index into the store's runtimes/ area and is NEVER co-mounted;
    // `expose` names the depended entries the payload surfaces (the §3
    // shim surface). The versioned JSON Schema admits it and the model
    // round-trips it.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-31T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires:\n  - {kind: language, engine: ruby, constraint: \"~> 3.3.0\"}\n\
        \x20 - {kind: runtime, engine: java, implementation: temurin, constraint: \">= 21, < 26\", expose: [java, keytool]}\n";
    let m = PayloadManifest::from_yaml(text).unwrap();
    let Some(Requirement::Runtime {
        engine,
        implementation,
        expose,
        ..
    }) = m.requires.get(1)
    else {
        panic!("runtime edge, got {:?}", m.requires);
    };
    assert_eq!(engine, "java");
    assert_eq!(implementation.as_deref(), Some("temurin"));
    assert_eq!(expose, &["java".to_string(), "keytool".to_string()]);
    // …and the schema agrees (MECE cross-check).
    let validator = schema_validator();
    validator
        .validate(&yaml_text_to_json(text))
        .expect("the kind-runtime requires edge is schema-legal");
    let back = PayloadManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
    assert_eq!(back, m);

    // The minimal form: implementation and expose omitted — and omitted
    // on the wire (never null / empty-list spellings).
    let minimal = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-31T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires: [{kind: runtime, engine: java, constraint: \">= 21\"}]\n";
    let m = PayloadManifest::from_yaml(minimal).unwrap();
    let Some(Requirement::Runtime {
        implementation,
        expose,
        ..
    }) = m.requires.first()
    else {
        panic!("runtime edge, got {:?}", m.requires);
    };
    assert!(implementation.is_none());
    assert!(expose.is_empty());
    let yaml = m.to_yaml().unwrap();
    assert!(!yaml.contains("expose"), "omitted on the wire: {yaml}");
    assert!(
        !yaml.contains("implementation"),
        "omitted on the wire: {yaml}"
    );
    validator
        .validate(&yaml_text_to_json(minimal))
        .expect("the minimal runtime edge is schema-legal");
    let back = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(back, m);
}

#[test]
fn runtime_edge_expose_never_collides_with_own_entrypoints() {
    // spec 30 §3: an exposed depended-entry name colliding with the
    // payload's OWN entrypoint is a named error at parse/validate.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-31T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: java, path: /bin/java}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires: [{kind: runtime, engine: java, constraint: \">= 21\", expose: [java]}]\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string().contains("collides"),
        "the expose x own-entrypoint collision is a named error: {err}"
    );
    // …but the schema stays silent — cross-field set intersection is
    // not JSON-Schema-expressible; the model owns this refusal.
    assert!(schema_validator().is_valid(&yaml_text_to_json(text)));
}

#[test]
fn executable_edge_fixture_shape() {
    // spec 32 §1 (schema_minor 5): the executable edge with BOTH
    // orthogonal axes — mount (the VFS surface) AND expose (the spawn
    // surface) — plus the provider pin and the critical flag.
    let text = read(&fixture_path("executable-edge"));
    let m = PayloadManifest::from_yaml(&text).unwrap();
    assert_eq!(m.identity.kind, PayloadKind::App);
    assert_eq!(m.requires.len(), 2);
    let Requirement::Executable {
        name,
        payload,
        constraint,
        mount,
        expose,
        critical,
        triplets,
    } = &m.requires[1]
    else {
        panic!("executable edge, got {:?}", m.requires[1]);
    };
    assert_eq!(name, "xml2rfc");
    assert_eq!(payload.as_deref(), Some("xml2rfc"));
    assert_eq!(constraint.as_str(), ">= 3.34");
    assert_eq!(mount.as_deref(), Some("/opt/xml2rfc"));
    assert_eq!(expose, &["xml2rfc".to_string()]);
    assert!(critical);
    assert!(triplets.is_none());
}

#[test]
fn executable_edge_is_schema_legal() {
    // The spawn-only form (expose without mount — the xml2rfc dispatch
    // without a VFS surface): the versioned JSON Schema admits it and
    // the model round-trips it.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 2.1.10}\n  created: \"2026-09-05T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires:\n  - {kind: language, engine: ruby, constraint: \"~> 3.3.0\"}\n\
        \x20 - {kind: executable, name: xml2rfc, constraint: \">= 3.34\", expose: [xml2rfc], critical: true}\n";
    let m = PayloadManifest::from_yaml(text).unwrap();
    let Some(Requirement::Executable {
        name,
        payload,
        mount,
        expose,
        critical,
        ..
    }) = m.requires.get(1)
    else {
        panic!("executable edge, got {:?}", m.requires);
    };
    assert_eq!(name, "xml2rfc");
    assert!(payload.is_none());
    assert!(mount.is_none());
    assert_eq!(expose, &["xml2rfc".to_string()]);
    assert!(critical);
    // …and the schema agrees (MECE cross-check).
    let validator = schema_validator();
    validator
        .validate(&yaml_text_to_json(text))
        .expect("the expose-only executable edge is schema-legal");
    let back = PayloadManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
    assert_eq!(back, m);

    // The mount-only form (the spec 03 §8 co-mount, unchanged): expose
    // and critical stay omitted on the wire (never null / empty-list /
    // false spellings).
    let co_mount = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 2.1.10}\n  created: \"2026-09-05T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires: [{kind: executable, name: dot, payload: graphviz, constraint: \">= 2.40\", mount: /opt/graphviz}]\n";
    let m = PayloadManifest::from_yaml(co_mount).unwrap();
    let Some(Requirement::Executable {
        expose, critical, ..
    }) = m.requires.first()
    else {
        panic!("executable edge, got {:?}", m.requires);
    };
    assert!(expose.is_empty());
    assert!(!critical);
    let yaml = m.to_yaml().unwrap();
    assert!(!yaml.contains("expose"), "omitted on the wire: {yaml}");
    assert!(!yaml.contains("critical"), "omitted on the wire: {yaml}");
    validator
        .validate(&yaml_text_to_json(co_mount))
        .expect("the mount-only executable edge is schema-legal");
    let back = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(back, m);
}

#[test]
fn executable_edge_requires_a_surface() {
    // spec 32 §1: an edge with NEITHER mount nor expose declares a
    // dependency that opens no surface — a named manifest error.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 2.1.10}\n  created: \"2026-09-05T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires: [{kind: executable, name: xml2rfc, constraint: \">= 3.34\"}]\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string().contains("neither mount nor expose"),
        "a contentless executable edge is a named error: {err}"
    );
    // …but the schema stays silent — the neither/nor shape is not
    // JSON-Schema-expressible without duplicating the variant; the model
    // owns this refusal.
    assert!(schema_validator().is_valid(&yaml_text_to_json(text)));
}

#[test]
fn executable_edge_expose_requires_the_capability_name() {
    // spec 32 §1: expose present requires name ∈ expose — the depended
    // capability must itself be surfaced.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 2.1.10}\n  created: \"2026-09-05T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires: [{kind: executable, name: xml2rfc, constraint: \">= 3.34\", expose: [other-tool]}]\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string().contains("name ∈ expose"),
        "an expose list without the capability name is a named error: {err}"
    );
    assert!(schema_validator().is_valid(&yaml_text_to_json(text)));
}

#[test]
fn executable_edge_expose_never_collides_with_own_entrypoints() {
    // spec 32 §1: the expose × own-entrypoint collision refusal is spec
    // 30 §3's, extended to the executable edge's expose list.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 2.1.10}\n  created: \"2026-09-05T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: xml2rfc, path: /bin/xml2rfc}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires: [{kind: executable, name: xml2rfc, constraint: \">= 3.34\", expose: [xml2rfc]}]\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string().contains("collides"),
        "the expose x own-entrypoint collision is a named error: {err}"
    );
    assert!(schema_validator().is_valid(&yaml_text_to_json(text)));
}

#[test]
fn edge_triplets_generalize_beyond_toolkit() {
    // spec 03 §2.3 (schema_minor 9, roadmap 86): the toolkit edge's
    // `triplets:` conditioning generalizes to the data, runtime, and
    // executable edges — one spelling per axis (payload-level coverage
    // stays `platforms:`).
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 2.1.10}\n  created: \"2026-09-12T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires:\n\
        \x20 - {kind: language, engine: ruby, constraint: \"~> 3.3.0\"}\n\
        \x20 - {kind: data, name: iso-codes, constraint: \">= 2024.1\", triplets: [x86_64-linux-gnu, aarch64-linux-gnu], mount: /__app__/share/iso-codes}\n\
        \x20 - {kind: runtime, engine: java, constraint: \">= 21\", expose: [java], triplets: [x86_64-windows-ucrt]}\n\
        \x20 - {kind: executable, name: xml2rfc, constraint: \">= 3.34\", expose: [xml2rfc], critical: true, triplets: [aarch64-macos]}\n";
    let m = PayloadManifest::from_yaml(text).unwrap();
    assert_eq!(m.requires.len(), 4);
    let Requirement::Data {
        triplets: data_ts, ..
    } = &m.requires[1]
    else {
        panic!("data edge, got {:?}", m.requires[1]);
    };
    assert_eq!(
        data_ts.as_deref(),
        Some(&[Platform::X86_64LinuxGnu, Platform::Aarch64LinuxGnu][..])
    );
    let Requirement::Runtime {
        triplets: rt_ts, ..
    } = &m.requires[2]
    else {
        panic!("runtime edge, got {:?}", m.requires[2]);
    };
    assert_eq!(rt_ts.as_deref(), Some(&[Platform::X86_64WindowsUcrt][..]));
    let Requirement::Executable {
        triplets: ex_ts, ..
    } = &m.requires[3]
    else {
        panic!("executable edge, got {:?}", m.requires[3]);
    };
    assert_eq!(ex_ts.as_deref(), Some(&[Platform::Aarch64Macos][..]));
    // …and the schema agrees (MECE cross-check), and the model
    // round-trips the fields.
    schema_validator()
        .validate(&yaml_text_to_json(text))
        .expect("per-edge triplets are schema-legal on data/runtime/executable");
    let yaml = m.to_yaml().unwrap();
    let back = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(back, m);
}

#[test]
fn language_edge_forbids_triplets() {
    // spec 03 §2.3: a skipped language edge would boot a payload with no
    // runtime — platform reach is the payload-level `platforms:` axis's
    // statement; the key on a language edge is a NAMED parse error
    // steering the author there.
    let text = "identity:\n  schema_version: 1\n  kind: app\n  name: metanorma\n  version: \"1\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 2.1.10}\n  created: \"2026-09-12T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  entrypoints:\n    - {name: metanorma, path: /bin/metanorma}\n\
        \x20 platforms: universal\n  capabilities: {exec: true, read: true}\n\
        requires: [{kind: language, engine: ruby, constraint: \"~> 3.3.0\", triplets: [aarch64-macos]}]\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string()
            .contains("not allowed on a kind: language edge"),
        "triplets on a language edge is a named error: {err}"
    );
    assert!(
        err.to_string().contains("platforms:"),
        "the refusal steers to the payload-level axis: {err}"
    );
}

#[test]
fn edge_triplets_list_grammar_holds_on_the_new_arms() {
    // The toolkit edge's list grammar (non-empty, no duplicates, no
    // reserved triplet) applies verbatim to the generalized arms.
    for edge in [
        "{kind: data, name: d, constraint: \">= 1\", mount: /d, triplets: []}",
        "{kind: data, name: d, constraint: \">= 1\", mount: /d, triplets: [aarch64-macos, aarch64-macos]}",
        "{kind: data, name: d, constraint: \">= 1\", mount: /d, triplets: [aarch64-windows-ucrt]}",
        "{kind: runtime, engine: java, constraint: \">= 21\", triplets: []}",
        "{kind: executable, name: x, constraint: \">= 1\", mount: /x, triplets: [x86_64-linux-gnu, x86_64-linux-gnu]}",
        "{kind: toolkit, name: t, constraint: \">= 1\", triplets: [aarch64-windows-ucrt]}",
    ] {
        let text = format!(
            "identity:\n  schema_version: 1\n  kind: app\n  name: m\n  version: \"1\"\n\
            \x20 producer: {{tool: tebako-cli, tool_version: 2.1.10}}\n  created: \"2026-09-12T00:00:00Z\"\n\
            \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
            \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
            \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
            provides:\n  entrypoints:\n    - {{name: m, path: /bin/m}}\n\
            \x20 platforms: universal\n  capabilities: {{exec: true, read: true}}\n\
            requires: [{edge}]\n",
        );
        let err = PayloadManifest::from_yaml(&text).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("triplets") || msg.contains("platform triplet"),
            "{edge}: a named list-grammar error, got {msg}"
        );
    }
}

#[test]
fn covers_host_semantics() {
    // spec 03 §2.3: absent = universal (byte-identical with pre-minor-9
    // behavior); a listed edge covers the listed hosts only.
    let universal = Requirement::Data {
        name: "d".into(),
        constraint: Constraint::new(">= 1").unwrap(),
        triplets: None,
        mount: None,
    };
    assert!(universal.triplets().is_none());
    for p in Platform::ALL {
        assert!(universal.covers_host(p), "universal covers {p}");
    }
    let listed = Requirement::Data {
        name: "d".into(),
        constraint: Constraint::new(">= 1").unwrap(),
        triplets: Some(vec![Platform::Aarch64Macos]),
        mount: None,
    };
    assert!(listed.covers_host(Platform::Aarch64Macos));
    assert!(!listed.covers_host(Platform::X86_64LinuxGnu));
    // The language edge never conditions (validate refuses the key).
    let language = Requirement::Language {
        engine: "ruby".into(),
        constraint: Constraint::new("~> 3.3.0").unwrap(),
        triplets: None,
    };
    assert!(language.triplets().is_none());
    assert!(language.covers_host(Platform::X86_64WindowsUcrt));
    // The skip note and journal line name the edge and the host through
    // ONE spelling (spec 00 invariant 10).
    assert_eq!(listed.edge_label(), "data:d");
    let note = listed.platform_skip_note(Platform::X86_64LinuxGnu);
    assert!(note.contains("data:d"), "{note}");
    assert!(note.contains("x86_64-linux-gnu"), "{note}");
    assert!(note.contains("aarch64-macos"), "{note}");
    let journal = listed.platform_skip_journal(Platform::X86_64LinuxGnu);
    assert_eq!(
        journal,
        "event=edge-platform-skip edge=data:d host=x86_64-linux-gnu"
    );
}

#[test]
fn absent_triplets_stay_off_the_wire() {
    // The additive-field law: edges without triplets serialize exactly
    // as before — no `triplets: null`, no empty list.
    let m = PayloadManifest::from_yaml(&read(&fixture_path("executable-edge"))).unwrap();
    let yaml = m.to_yaml().unwrap();
    assert!(
        !yaml.contains("triplets"),
        "an unconditioned edge emits no triplets key: {yaml}"
    );
}

#[test]
fn runtime_spawn_surface_is_schema_legal() {
    // spec 30 §2 (schema_minor 4): the additive spawn surface on the
    // runtime's own PROVIDES — `provides.entrypoints` (the app-entrypoint
    // grammar minus runtime_requirement: the commands this runtime boots
    // for a consumer's expose list) and `provides.provides[].implementation`
    // (spec 28 §8 — the engine implementation the edge's filter matches).
    // The versioned JSON Schema admits both and the model round-trips.
    let text = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-java\n  version: \"21.0.8\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-31T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: java, version: \"21.0.8\", abi_line: \"21\", platform: aarch64-macos, implementation: temurin}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 entrypoints:\n    - {name: java, path: /bin/java}\n    - {name: keytool, path: /bin/keytool, args_default: [--help]}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n";
    let m = PayloadManifest::from_yaml(text).unwrap();
    let Provides::Runtime(rt) = &m.provides else {
        panic!("runtime provides, got {:?}", m.provides);
    };
    assert_eq!(rt.provides[0].implementation.as_deref(), Some("temurin"));
    assert_eq!(rt.entrypoints.len(), 2);
    assert_eq!(rt.entrypoints[0].name, "java");
    assert_eq!(rt.entrypoints[1].args_default, vec!["--help".to_string()]);
    assert!(rt
        .entrypoints
        .iter()
        .all(|ep| ep.runtime_requirement.is_none()));
    // …and the schema agrees (MECE cross-check).
    let validator = schema_validator();
    validator
        .validate(&yaml_text_to_json(text))
        .expect("the runtime spawn surface is schema-legal");
    let back = PayloadManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
    assert_eq!(back, m);

    // The pre-surface shape stays legal — no entrypoints key, no
    // implementation key — and both stay omitted on the wire (never
    // null / empty-list spellings).
    let bare = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: 4.0.6\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-31T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: aarch64-macos}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n";
    let m = PayloadManifest::from_yaml(bare).unwrap();
    let Provides::Runtime(rt) = &m.provides else {
        panic!("runtime provides, got {:?}", m.provides);
    };
    assert!(rt.entrypoints.is_empty());
    assert!(rt.provides[0].implementation.is_none());
    let yaml = m.to_yaml().unwrap();
    assert!(!yaml.contains("entrypoints"), "omitted on the wire: {yaml}");
    assert!(
        !yaml.contains("implementation"),
        "omitted on the wire: {yaml}"
    );
    validator
        .validate(&yaml_text_to_json(bare))
        .expect("the pre-surface runtime provides stays schema-legal");
    let back = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(back, m);
}

#[test]
fn runtime_spawn_entrypoint_rejects_runtime_requirement() {
    // spec 30 §2: a runtime runs on ITSELF — a spawn-surface entrypoint
    // carrying runtime_requirement is a named manifest error (the model
    // owns this refusal; the item grammar is not JSON-Schema-expressible
    // per-kind without duplicating the entrypoint def).
    let text = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-java\n  version: \"21.0.8\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-31T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: java, version: \"21.0.8\", abi_line: \"21\", platform: aarch64-macos}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 entrypoints:\n    - {name: java, path: /bin/java, runtime_requirement: {engine: ruby, constraint: \">= 3.3\"}}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string()
            .contains("runtime_requirement is meaningless on a runtime"),
        "a spawn-surface runtime_requirement is a named error: {err}"
    );
}

#[test]
fn runtime_provides_rejects_empty_implementation() {
    // spec 28 §8 / spec 30 §2: `implementation` is optional but never
    // empty — both layers refuse (the schema's minLength and the model's
    // check agree here).
    let text = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-java\n  version: \"21.0.8\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-31T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: java, version: \"21.0.8\", abi_line: \"21\", platform: aarch64-macos, implementation: \"\"}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string()
            .contains("provides.provides[].implementation must not be empty"),
        "an empty implementation is a named error: {err}"
    );
    assert!(!schema_validator().is_valid(&yaml_text_to_json(text)));
}

#[test]
fn engine_provides_language_version_parses_and_round_trips() {
    // spec 28 §8 (schema_minor 7): `language_version` is the language
    // level the build implements (jruby 9.4 → "3.1"; for mri it equals
    // version) — what a language-level requirement entry matches.
    // Additive: absent on pre-field manifests, omitted on the wire.
    let with = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-jruby\n  version: \"9.4.8\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-09-07T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: ruby, implementation: jruby, version: \"9.4.8\", language_version: \"3.1\", abi_line: \"9.4\", platform: aarch64-macos}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n";
    let m = PayloadManifest::from_yaml(with).unwrap();
    let Provides::Runtime(rt) = &m.provides else {
        panic!("runtime provides, got {:?}", m.provides)
    };
    assert_eq!(rt.provides[0].language_version.as_deref(), Some("3.1"));
    let yaml = m.to_yaml().unwrap();
    assert!(yaml.contains("language_version"), "emitted: {yaml}");
    let reparsed = PayloadManifest::from_yaml(&yaml).unwrap();
    assert_eq!(reparsed, m);
    assert!(schema_validator().is_valid(&yaml_text_to_json(with)));

    // The compat window: a manifest predating the field parses, and the
    // key stays omitted on the wire.
    let without = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: \"4.0.6\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-09-07T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: ruby, version: \"4.0.6\", abi_line: \"4.0\", platform: aarch64-macos}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n";
    let m2 = PayloadManifest::from_yaml(without).unwrap();
    let Provides::Runtime(rt2) = &m2.provides else {
        panic!("runtime provides, got {:?}", m2.provides)
    };
    assert_eq!(rt2.provides[0].language_version, None);
    assert!(
        !m2.to_yaml().unwrap().contains("language_version"),
        "absent stays omitted"
    );
    assert!(schema_validator().is_valid(&yaml_text_to_json(without)));
}

#[test]
fn runtime_provides_rejects_empty_language_version() {
    let text = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-jruby\n  version: \"9.4.8\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-09-07T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: ruby, implementation: jruby, version: \"9.4.8\", language_version: \"\", abi_line: \"9.4\", platform: aarch64-macos}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n";
    let err = PayloadManifest::from_yaml(text).unwrap_err();
    assert!(
        err.to_string()
            .contains("provides.provides[].language_version must not be empty"),
        "an empty language_version is a named error: {err}"
    );
    assert!(!schema_validator().is_valid(&yaml_text_to_json(text)));
}

#[test]
fn checks_key_is_schema_legal() {
    // spec 26 §1 (schema_minor 3): the additive `checks:` key — the
    // versioned JSON Schema admits it and the model round-trips it.
    // EXEC form (the runtime slice's boot-and-stdlib smoke, §1.1):
    let exec = "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: 4.0.6\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-19T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  provides: {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: aarch64-macos}\n\
        \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
        \x20 capabilities: {exec: true, read: true, runtime: true}\n\
        checks:\n  boot-and-stdlib:\n    entry: self\n\
        \x20   argv: [\"-e\", 'require \"json\"; puts JSON.generate({ok: 1})']\n\
        \x20   expect: {exit: 0, stdout: '\"ok\":1'}\n    timeout: 60\n";
    // STRUCTURAL form (the data slice's layout assertions, §1.1):
    let structural = "identity:\n  schema_version: 1\n  kind: data\n  name: acme-templates\n  version: \"3\"\n\
        \x20 producer: {tool: tebako-cli, tool_version: 0.16.0}\n  created: \"2026-08-19T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
        provides:\n  mount_semantics: {suggested: /templates/acme}\n  capabilities: {exec: false, read: true}\n\
        checks:\n  layout:\n    expect:\n      image_files: [/templates/acme/cover.adoc, /templates/acme/header.html]\n";
    let validator = schema_validator();
    for (text, what) in [(exec, "exec"), (structural, "structural")] {
        let m = PayloadManifest::from_yaml(text).unwrap_or_else(|e| panic!("{what} checks: {e}"));
        assert_eq!(m.checks.len(), 1);
        // …and the schema agrees (MECE cross-check).
        validator
            .validate(&yaml_text_to_json(text))
            .unwrap_or_else(|e| panic!("{what} checks against the JSON schema: {e}"));
        let back = PayloadManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m, "{what} checks round-trip");
    }
}

/// A minimal kind:app manifest base for checks-validation tests (an exec
/// check needs an exec-capable kind).
fn app_manifest_with_checks(checks: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: acme-app\n  version: \"1\"\n\
        \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-19T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
        provides:\n  entrypoints: [{{name: acme, path: /bin/acme}}]\n  platforms: [aarch64-macos]\n  capabilities: {{exec: true, read: true}}\n\
        checks:\n{checks}"
    )
}

#[test]
fn checks_fixture_families_are_mece() {
    // spec 26 §2.1: `fixtures` is the slice family's (in-image) source;
    // `fixtures_inline`/`fixtures_host` are the composition family's.
    // Each is a named error in the other's context.
    let err = PayloadManifest::from_yaml(&app_manifest_with_checks(
        "  c1:\n    entry: /bin/acme\n    fixtures_inline: {a.txt: hi}\n",
    ))
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("fixtures_inline/fixtures_host belong to composition checks"),
        "{err}"
    );
    let err = PayloadManifest::from_yaml(&app_manifest_with_checks(
        "  c1:\n    entry: /bin/acme\n    fixtures_host: fixtures\n",
    ))
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("fixtures_inline/fixtures_host belong to composition checks"),
        "{err}"
    );
    // …and the composition context refuses the slice family.
    let check: Check = serde_yml::from_str("entry: /bin/acme\nfixtures: /fixtures\n").unwrap();
    let err = check.validate_composition().unwrap_err();
    assert!(
        err.to_string()
            .contains("a composition check declares fixtures_inline or fixtures_host"),
        "{err}"
    );
    // Both composition families validate in their own context.
    let check: Check =
        serde_yml::from_str("entry: /bin/acme\nfixtures_inline: {a.txt: hi, sub/b.txt: bye}\n")
            .unwrap();
    check.validate_composition().unwrap();
    let check: Check =
        serde_yml::from_str("entry: /bin/acme\nfixtures_host: fixtures/iso\n").unwrap();
    check.validate_composition().unwrap();
}

#[test]
fn checks_fixtures_host_path_rules() {
    // fixtures_host is relative to the composition FILE — the absolute
    // spellings of EITHER platform family are refused everywhere (the
    // validator's answer never depends on the host OS).
    for bad in [
        "/abs/fixtures",
        "\\\\share\\\\fixtures",
        "C:/fixtures",
        "../up",
        "a/../b",
    ] {
        let check: Check =
            serde_yml::from_str(&format!("entry: /bin/acme\nfixtures_host: \"{bad}\"\n")).unwrap();
        let err = check.validate_composition().unwrap_err();
        assert!(
            err.to_string()
                .contains("fixtures_host must be relative to the composition file"),
            "{bad}: {err}"
        );
    }
    let check: Check = serde_yml::from_str("entry: /bin/acme\nfixtures_host: \"\"\n").unwrap();
    let err = check.validate_composition().unwrap_err();
    assert!(
        err.to_string().contains("fixtures_host must not be empty"),
        "{err}"
    );
}

#[test]
fn checks_fixtures_inline_name_rules() {
    for bad in ["/abs.txt", "../up.txt", "a//b.txt"] {
        let check: Check = serde_yml::from_str(&format!(
            "entry: /bin/acme\nfixtures_inline: {{\"{bad}\": x}}\n"
        ))
        .unwrap();
        let err = check.validate_composition().unwrap_err();
        assert!(
            err.to_string()
                .contains("fixtures_inline names must be non-empty scratch-relative file paths"),
            "{bad}: {err}"
        );
    }
}

#[test]
fn structural_checks_refuse_every_fixture_key() {
    // A structural check (no entry) has no exec surface at all: every
    // fixture key is a named error, in either context.
    let err = PayloadManifest::from_yaml(&app_manifest_with_checks(
        "  c1:\n    fixtures: /fixtures\n    expect: {image_files: [/a]}\n",
    ))
    .unwrap_err();
    assert!(err.to_string().contains("are exec-only"), "{err}");
    let check: Check =
        serde_yml::from_str("fixtures_inline: {a.txt: hi}\nexpect: {image_files: [/a]}\n").unwrap();
    let err = check.validate_composition().unwrap_err();
    assert!(err.to_string().contains("are exec-only"), "{err}");
}

#[test]
fn duplicate_check_names_are_refused() {
    // The checks map's own deserializer refuses a re-declared name — an
    // authoring ambiguity is a named structural error, never last-wins.
    let err = PayloadManifest::from_yaml(&app_manifest_with_checks(
        "  c1:\n    entry: /bin/acme\n  c1:\n    entry: /bin/acme\n",
    ))
    .unwrap_err();
    assert!(
        err.to_string().contains("duplicate check name \"c1\""),
        "{err}"
    );
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
    let cases: [(&str, &str); 13] = [
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
        // a check name outside the [A-Za-z0-9][A-Za-z0-9._-]* grammar (spec 26 §1)
        (
            "identity: {schema_version: 1, kind: app, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {entrypoints: [{name: x, path: /x}], platforms: universal, \
             capabilities: {exec: true, read: true}}\n\
             checks: {\"-lead\": {entry: /bin/x}}\n",
            "bad check name",
        ),
        // a when value outside the windows/macos/linux family set
        (
            "identity: {schema_version: 1, kind: app, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {entrypoints: [{name: x, path: /x}], platforms: universal, \
             capabilities: {exec: true, read: true}}\n\
             checks: {c: {entry: /bin/x, when: [solaris]}}\n",
            "bad when value",
        ),
        // spec 30 §1: expose entries are bare command names — no path
        // separator (schema pattern + model, MECE)
        (
            "identity: {schema_version: 1, kind: data, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: false, read: true}}\n\
             requires: [{kind: runtime, engine: java, constraint: \">= 21\", \
             expose: [/usr/bin/java]}]\n",
            "expose entry with a path separator",
        ),
        // an empty expose entry
        (
            "identity: {schema_version: 1, kind: data, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: false, read: true}}\n\
             requires: [{kind: runtime, engine: java, constraint: \">= 21\", \
             expose: [\"\"]}]\n",
            "empty expose entry",
        ),
        // a repeated expose entry
        (
            "identity: {schema_version: 1, kind: data, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: false, read: true}}\n\
             requires: [{kind: runtime, engine: java, constraint: \">= 21\", \
             expose: [java, java]}]\n",
            "duplicate expose entry",
        ),
        // implementation present but empty
        (
            "identity: {schema_version: 1, kind: data, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {mount_semantics: {suggested: /x}, capabilities: {exec: false, read: true}}\n\
             requires: [{kind: runtime, engine: java, implementation: \"\", \
             constraint: \">= 21\"}]\n",
            "empty implementation",
        ),
        // spec 32 §1: the executable edge's expose entries ride the same
        // bare-name grammar (schema pattern + model, MECE)
        (
            "identity: {schema_version: 1, kind: app, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {entrypoints: [{name: x, path: /x}], platforms: universal, \
             capabilities: {exec: true, read: true}}\n\
             requires: [{kind: executable, name: xml2rfc, constraint: \">= 3.34\", \
             expose: [/usr/bin/xml2rfc]}]\n",
            "executable-edge expose entry with a path separator",
        ),
        // an empty expose entry on the executable edge
        (
            "identity: {schema_version: 1, kind: app, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {entrypoints: [{name: x, path: /x}], platforms: universal, \
             capabilities: {exec: true, read: true}}\n\
             requires: [{kind: executable, name: xml2rfc, constraint: \">= 3.34\", \
             expose: [\"\"]}]\n",
            "executable-edge empty expose entry",
        ),
        // a repeated expose entry on the executable edge
        (
            "identity: {schema_version: 1, kind: app, name: x, version: \"1\", \
             producer: {tool: t, tool_version: \"1\"}, created: now, \
             digest: {tree_hash: \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \
             blob_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}, \
             signing: {state: unsigned}, encryption: {state: none}}\n\
             provides: {entrypoints: [{name: x, path: /x}], platforms: universal, \
             capabilities: {exec: true, read: true}}\n\
             requires: [{kind: executable, name: xml2rfc, constraint: \">= 3.34\", \
             expose: [xml2rfc, xml2rfc]}]\n",
            "executable-edge duplicate expose entry",
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

// ---------------------------------------------------------------------
// spec 33 (schema_minor 6): the on_runtime block — runtime-on-runtime
// composition (the owner edge is an expose-less kind:runtime requires
// on a kind:runtime payload).
// ---------------------------------------------------------------------

/// A jruby-style depending runtime manifest. `provides_extra` rides at
/// the provides-level indent (the on_runtime block or nothing);
/// `requires_block` at the top level ("" or "requires:\n  - …\n").
fn on_runtime_manifest(provides_extra: &str, requires_block: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-jruby\n  version: 9.4.8.0\n\
        \x20 producer: {{tool: tebako-cli, tool_version: 2.5.0}}\n  created: \"2026-09-07T00:00:00Z\"\n\
        \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
        \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
        \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
        provides:\n  provides: {{engine: ruby, implementation: jruby, version: 9.4.8.0, abi_line: \"9.4\", platform: aarch64-macos}}\n\
        \x20 built_from: {{src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v2.4.0}}\n\
        {provides_extra}\
        \x20 capabilities: {{exec: true, read: true, runtime: true}}\n\
        {requires_block}"
    )
}

const ON_RUNTIME_BLOCK: &str = "  on_runtime:\n    mount: /__runners__/jruby\n    argv_template: [\"-classpath\", \"{mount}/lib/jruby.jar\", \"org.jruby.Main\"]\n    owner_contract: \">= 2\"\n";
const OWNER_EDGE: &str = "requires:\n  - {kind: runtime, engine: java, constraint: \">= 21\"}\n";

#[test]
fn on_runtime_block_is_schema_legal() {
    // spec 33 §2 (schema_minor 6): the additive on_runtime block under
    // RuntimeProvides — the owner edge present, the block declaring the
    // composition. The versioned JSON Schema admits it and the model
    // round-trips it.
    let text = on_runtime_manifest(ON_RUNTIME_BLOCK, OWNER_EDGE);
    let m = PayloadManifest::from_yaml(&text).unwrap();
    let Provides::Runtime(rt) = &m.provides else {
        panic!("runtime provides, got {:?}", m.provides)
    };
    let on = rt.on_runtime.as_ref().expect("the on_runtime block parses");
    assert_eq!(on.mount, "/__runners__/jruby");
    assert_eq!(
        on.argv_template,
        vec![
            "-classpath".to_string(),
            "{mount}/lib/jruby.jar".to_string(),
            "org.jruby.Main".to_string()
        ]
    );
    assert_eq!(on.owner_contract.as_ref().map(|c| c.as_str()), Some(">= 2"));
    // …and the schema agrees (MECE cross-check).
    let validator = schema_validator();
    validator
        .validate(&yaml_text_to_json(&text))
        .expect("on_runtime: is schema-legal");
    let back = PayloadManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
    assert_eq!(back, m);

    // owner_contract omitted: the model keeps it absent (the ">= 2"
    // default applies at negotiation, not at parse) and off the wire.
    let block =
        "  on_runtime:\n    mount: /__runners__/jruby\n    argv_template: [\"org.jruby.Main\"]\n";
    let text = on_runtime_manifest(block, OWNER_EDGE);
    let m = PayloadManifest::from_yaml(&text).unwrap();
    let Provides::Runtime(rt) = &m.provides else {
        panic!("runtime provides")
    };
    let on = rt.on_runtime.as_ref().expect("the on_runtime block parses");
    assert!(on.owner_contract.is_none());
    let wire = m.to_yaml().unwrap();
    assert!(!wire.contains("owner_contract"), "{wire}");
    assert!(wire.contains("on_runtime:"), "{wire}");
    let back = PayloadManifest::from_yaml(&wire).unwrap();
    assert_eq!(back, m);

    // The block absent (an ordinary runtime) stays absent on the wire —
    // a reader predating schema_minor 6 never sees the key.
    let plain = on_runtime_manifest("", "");
    let m = PayloadManifest::from_yaml(&plain).unwrap();
    let Provides::Runtime(rt) = &m.provides else {
        panic!("runtime provides")
    };
    assert!(rt.on_runtime.is_none());
    let wire = m.to_yaml().unwrap();
    assert!(!wire.contains("on_runtime"), "{wire}");
}

#[test]
fn on_runtime_without_owner_edge_is_a_named_error() {
    // spec 33 §2: the block is FORBIDDEN without the owner edge — the
    // composition declares its rewrite or it is not a composition.
    let text = on_runtime_manifest(ON_RUNTIME_BLOCK, "");
    let err = PayloadManifest::from_yaml(&text).unwrap_err();
    assert!(
        err.to_string().contains("on_runtime"),
        "the error names the key: {err}"
    );
}

#[test]
fn owner_edge_without_on_runtime_is_a_named_error() {
    // spec 33 §2: an expose-less kind:runtime edge on a kind:runtime
    // payload is an owner edge, and the owner edge REQUIRES the block.
    let text = on_runtime_manifest("", OWNER_EDGE);
    let err = PayloadManifest::from_yaml(&text).unwrap_err();
    assert!(
        err.to_string().contains("on_runtime"),
        "the error names the missing block: {err}"
    );
}

#[test]
fn two_owner_edges_are_a_named_error() {
    // spec 33 §1: at most ONE owner edge per payload — a runtime needing
    // two process owners is two payloads.
    let text = on_runtime_manifest(
        ON_RUNTIME_BLOCK,
        "requires:\n  - {kind: runtime, engine: java, constraint: \">= 21\"}\n  - {kind: runtime, engine: python, constraint: \">= 3.11\"}\n",
    );
    let err = PayloadManifest::from_yaml(&text).unwrap_err();
    assert!(
        err.to_string().contains("one owner edge") || err.to_string().contains("at most one"),
        "the error names the cardinality: {err}"
    );
}

#[test]
fn spawned_edges_are_not_owner_edges() {
    // spec 33 §1: a runtime MAY additionally carry spec-30 spawned edges
    // (expose: present) — they never count toward the owner cardinality,
    // and a runtime with ONLY a spawned edge declares no on_runtime.
    let spawned_only = on_runtime_manifest(
        "",
        "requires:\n  - {kind: runtime, engine: java, constraint: \">= 21\", expose: [java]}\n",
    );
    PayloadManifest::from_yaml(&spawned_only).expect("a spawned edge alone needs no block");
    let both = on_runtime_manifest(
        ON_RUNTIME_BLOCK,
        "requires:\n  - {kind: runtime, engine: java, constraint: \">= 21\"}\n  - {kind: runtime, engine: python, constraint: \">= 3.11\", expose: [python]}\n",
    );
    PayloadManifest::from_yaml(&both).expect("owner edge + spawned edge + block is legal");
}

#[test]
fn on_runtime_field_validation() {
    let cases = [
        (
            // mount must be POSIX-absolute
            "  on_runtime:\n    mount: runners/jruby\n    argv_template: [\"org.jruby.Main\"]\n",
            "absolute",
        ),
        (
            // mount: / is invalid at validation (spec 33 §2)
            "  on_runtime:\n    mount: /\n    argv_template: [\"org.jruby.Main\"]\n",
            "must not be \"/\"",
        ),
        (
            // the template must not be empty
            "  on_runtime:\n    mount: /__runners__/jruby\n    argv_template: []\n",
            "argv_template",
        ),
        (
            // template elements must not be empty strings
            "  on_runtime:\n    mount: /__runners__/jruby\n    argv_template: [\"-classpath\", \"\"]\n",
            "argv_template",
        ),
    ];
    for (block, want) in cases {
        let text = on_runtime_manifest(block, OWNER_EDGE);
        let err = PayloadManifest::from_yaml(&text).unwrap_err();
        assert!(err.to_string().contains(want), "case wants '{want}': {err}");
    }
    // owner_contract must parse as a constraint (the model's type
    // refuses garbage at deserialize).
    let text = on_runtime_manifest(
        "  on_runtime:\n    mount: /__runners__/jruby\n    argv_template: [\"org.jruby.Main\"]\n    owner_contract: \"bogus\"\n",
        OWNER_EDGE,
    );
    assert!(PayloadManifest::from_yaml(&text).is_err());
}
