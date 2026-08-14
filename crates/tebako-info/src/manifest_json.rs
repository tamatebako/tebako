//! The payload manifest mapped 1:1 onto a JSON value (spec 15 §6:
//! `manifest` is "spec 03 mapped 1:1" — same field names as the authored
//! YAML, values carried across). Built from the PARSED model, so a
//! leniently-read invalid manifest still maps (absent optional members
//! stay absent).

use tebako_json::Value as Json;
use tpkg::{
    Capabilities, Identity, PayloadManifest, Platforms, Provides, Requirement, RuntimeProvides,
};

fn s(v: &str) -> Json {
    Json::String(v.to_string())
}

fn n(v: u64) -> Json {
    Json::Number(v.to_string())
}

/// Convert a free-form YAML value (annotations, env, toolkit/language
/// provides) preserving the YAML shape.
fn yaml_value(v: &serde_yml::Value) -> Json {
    match v {
        serde_yml::Value::Null => Json::Null,
        serde_yml::Value::Bool(b) => Json::Bool(*b),
        serde_yml::Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                Json::Number(i.to_string())
            } else if let Some(u) = num.as_u64() {
                Json::Number(u.to_string())
            } else if let Some(f) = num.as_f64() {
                Json::Number(f.to_string())
            } else {
                Json::Null
            }
        }
        serde_yml::Value::String(text) => s(text),
        serde_yml::Value::Sequence(items) => Json::Array(items.iter().map(yaml_value).collect()),
        serde_yml::Value::Mapping(map) => Json::Object(
            map.iter()
                .map(|(k, v)| (yaml_key(k), yaml_value(v)))
                .collect(),
        ),
        serde_yml::Value::Tagged(tagged) => yaml_value(&tagged.value),
    }
}

fn yaml_key(k: &serde_yml::Value) -> String {
    match k {
        serde_yml::Value::String(text) => text.clone(),
        other => match yaml_value(other) {
            Json::String(text) => text,
            Json::Number(text) => text,
            Json::Bool(b) => b.to_string(),
            _ => "(non-scalar key)".to_string(),
        },
    }
}

fn platforms_json(p: &Platforms) -> Json {
    match p {
        Platforms::Universal => s("universal"),
        Platforms::Triplets(ts) => Json::Array(ts.iter().map(|t| s(t.as_triplet())).collect()),
    }
}

fn capabilities_json(c: &Capabilities) -> Json {
    let mut out = vec![
        ("exec".to_string(), Json::Bool(c.exec)),
        ("read".to_string(), Json::Bool(c.read)),
    ];
    if let Some(runtime) = c.runtime {
        out.push(("runtime".to_string(), Json::Bool(runtime)));
    }
    Json::Object(out)
}

fn identity_json(id: &Identity) -> Json {
    let mut out: Vec<(String, Json)> = vec![
        (
            "schema_version".to_string(),
            n(u64::from(id.schema_version)),
        ),
        (
            "kind".to_string(),
            s(match id.kind {
                tpkg::PayloadKind::App => "app",
                tpkg::PayloadKind::Data => "data",
                tpkg::PayloadKind::Toolkit => "toolkit",
                tpkg::PayloadKind::Runtime => "runtime",
                tpkg::PayloadKind::Language => "language",
            }),
        ),
        ("name".to_string(), s(&id.name)),
        ("version".to_string(), s(&id.version)),
        (
            "producer".to_string(),
            Json::Object(vec![
                ("tool".to_string(), s(&id.producer.tool)),
                ("tool_version".to_string(), s(&id.producer.tool_version)),
            ]),
        ),
        ("created".to_string(), s(&id.created)),
    ];
    if let Some(source) = &id.source {
        let mut src = Vec::new();
        if let Some(sha) = &source.src_sha256 {
            src.push(("src_sha256".to_string(), s(sha)));
        }
        if let Some(commit) = &source.commit {
            src.push(("commit".to_string(), s(commit)));
        }
        if let Some(builder) = &source.builder {
            src.push(("builder".to_string(), s(builder)));
        }
        out.push(("source".to_string(), Json::Object(src)));
    }
    if let Some(sbom) = &id.sbom {
        out.push((
            "sbom".to_string(),
            Json::Object(vec![("ref".to_string(), s(&sbom.r#ref))]),
        ));
    }
    out.push((
        "digest".to_string(),
        Json::Object(vec![
            ("tree_hash".to_string(), s(&id.digest.tree_hash)),
            ("blob_sha256".to_string(), s(&id.digest.blob_sha256)),
        ]),
    ));
    let mut signing = vec![(
        "state".to_string(),
        s(match id.signing.state {
            tpkg::SigningState::Unsigned => "unsigned",
            tpkg::SigningState::Signed => "signed",
        }),
    )];
    if let Some(keyid) = &id.signing.keyid {
        signing.push(("keyid".to_string(), s(keyid)));
    }
    if id.signing.mechanism.is_some() {
        signing.push(("mechanism".to_string(), s("openpgp")));
    }
    out.push(("signing".to_string(), Json::Object(signing)));
    let mut encryption = vec![(
        "state".to_string(),
        s(match id.encryption.state {
            tpkg::EncryptionState::None => "none",
            tpkg::EncryptionState::Encrypted => "encrypted",
        }),
    )];
    if !id.encryption.parts.is_empty() {
        encryption.push((
            "parts".to_string(),
            Json::Array(
                id.encryption
                    .parts
                    .iter()
                    .map(|p| {
                        Json::Object(vec![
                            (
                                "paths".to_string(),
                                Json::Array(p.paths.iter().map(|x| s(x)).collect()),
                            ),
                            ("algorithm".to_string(), s(&p.algorithm)),
                            (
                                "envelope_refs".to_string(),
                                Json::Array(p.envelope_refs.iter().map(|x| s(x)).collect()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    out.push(("encryption".to_string(), Json::Object(encryption)));
    if !id.annotations.is_empty() {
        out.push((
            "annotations".to_string(),
            Json::Object(
                id.annotations
                    .iter()
                    .map(|(k, v)| (k.clone(), yaml_value(v)))
                    .collect(),
            ),
        ));
    }
    Json::Object(out)
}

fn runtime_provides_json(p: &RuntimeProvides) -> Json {
    let engines: Vec<Json> = p
        .provides
        .iter()
        .map(|e| {
            Json::Object(vec![
                ("engine".to_string(), s(&e.engine)),
                ("version".to_string(), s(&e.version)),
                ("abi_line".to_string(), s(&e.abi_line)),
                ("platform".to_string(), s(e.platform.as_triplet())),
            ])
        })
        .collect();
    // The one-or-many wire shape: a single engine maps back to the
    // single-mapping form.
    let provides = if let [one] = engines.as_slice() {
        one.clone()
    } else {
        Json::Array(engines)
    };
    let mut out = vec![
        ("provides".to_string(), provides),
        (
            "built_from".to_string(),
            Json::Object(vec![
                ("src_sha256".to_string(), s(&p.built_from.src_sha256)),
                ("patch_set".to_string(), s(&p.built_from.patch_set)),
            ]),
        ),
    ];
    if !p.env.is_empty() {
        out.push((
            "env".to_string(),
            Json::Object(p.env.iter().map(|(k, v)| (k.clone(), s(v))).collect()),
        ));
    }
    out.push((
        "capabilities".to_string(),
        capabilities_json(&p.capabilities),
    ));
    Json::Object(out)
}

fn provides_json(p: &Provides) -> Json {
    match p {
        Provides::App(app) => Json::Object(vec![
            (
                "entrypoints".to_string(),
                Json::Array(
                    app.entrypoints
                        .iter()
                        .map(|e| {
                            let mut obj = vec![
                                ("name".to_string(), s(&e.name)),
                                ("path".to_string(), s(&e.path)),
                                (
                                    "args_default".to_string(),
                                    Json::Array(e.args_default.iter().map(|a| s(a)).collect()),
                                ),
                            ];
                            // omitted for native entrypoints (the YAML key
                            // is omitted too — spec 03 §2.2)
                            if let Some(req) = &e.runtime_requirement {
                                obj.push((
                                    "runtime_requirement".to_string(),
                                    Json::Object(vec![
                                        ("engine".to_string(), s(&req.engine)),
                                        ("constraint".to_string(), s(req.constraint.as_str())),
                                    ]),
                                ));
                            }
                            Json::Object(obj)
                        })
                        .collect(),
                ),
            ),
            ("platforms".to_string(), platforms_json(&app.platforms)),
            (
                "capabilities".to_string(),
                capabilities_json(&app.capabilities),
            ),
        ]),
        Provides::Runtime(rt) => runtime_provides_json(rt),
        Provides::Toolkit(tk) => Json::Object(vec![
            (
                "executables".to_string(),
                Json::Array(
                    tk.executables
                        .iter()
                        .map(|e| {
                            let mut obj = vec![
                                ("name".to_string(), s(&e.name)),
                                ("path".to_string(), s(&e.path)),
                            ];
                            if let Some(v) = &e.version {
                                obj.push(("version".to_string(), s(v)));
                            }
                            Json::Object(obj)
                        })
                        .collect(),
                ),
            ),
            (
                "libraries".to_string(),
                Json::Array(
                    tk.libraries
                        .iter()
                        .map(|l| {
                            Json::Object(vec![
                                ("name".to_string(), s(&l.name)),
                                ("path".to_string(), s(&l.path)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("platforms".to_string(), platforms_json(&tk.platforms)),
            (
                "capabilities".to_string(),
                capabilities_json(&tk.capabilities),
            ),
        ]),
        Provides::Data(data) => {
            let mut out = vec![
                (
                    "mount_semantics".to_string(),
                    Json::Object(vec![(
                        "suggested".to_string(),
                        s(&data.mount_semantics.suggested),
                    )]),
                ),
                (
                    "consumers".to_string(),
                    Json::Array(data.consumers.iter().map(|c| s(c)).collect()),
                ),
            ];
            out.push((
                "capabilities".to_string(),
                capabilities_json(&data.capabilities),
            ));
            Json::Object(out)
        }
        Provides::Other(map) => Json::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), yaml_value(v)))
                .collect(),
        ),
    }
}

fn requirement_json(r: &Requirement) -> Json {
    match r {
        Requirement::Language { engine, constraint } => Json::Object(vec![
            ("kind".to_string(), s("language")),
            ("engine".to_string(), s(engine)),
            ("constraint".to_string(), s(constraint.as_str())),
        ]),
        Requirement::Toolkit {
            name,
            constraint,
            triplets,
            mount,
        } => {
            let mut out = vec![
                ("kind".to_string(), s("toolkit")),
                ("name".to_string(), s(name)),
                ("constraint".to_string(), s(constraint.as_str())),
            ];
            if let Some(ts) = triplets {
                out.push((
                    "triplets".to_string(),
                    Json::Array(ts.iter().map(|t| s(t.as_triplet())).collect()),
                ));
            }
            if let Some(m) = mount {
                out.push(("mount".to_string(), s(m)));
            }
            Json::Object(out)
        }
        Requirement::Data {
            name,
            constraint,
            mount,
        } => {
            let mut out = vec![
                ("kind".to_string(), s("data")),
                ("name".to_string(), s(name)),
                ("constraint".to_string(), s(constraint.as_str())),
            ];
            if let Some(m) = mount {
                out.push(("mount".to_string(), s(m)));
            }
            Json::Object(out)
        }
    }
}

/// The parsed manifest as one JSON object (spec 03 field names 1:1).
pub fn manifest_to_json(m: &PayloadManifest) -> Json {
    let mut out = vec![
        ("identity".to_string(), identity_json(&m.identity)),
        ("provides".to_string(), provides_json(&m.provides)),
    ];
    if !m.requires.is_empty() {
        out.push((
            "requires".to_string(),
            Json::Array(m.requires.iter().map(requirement_json).collect()),
        ));
    }
    if !m.materialize.is_empty() {
        out.push((
            "materialize".to_string(),
            Json::Array(m.materialize.iter().map(|p| s(p)).collect()),
        ));
    }
    Json::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PayloadManifest {
        PayloadManifest::from_yaml(include_str!(
            "../../tpkg/tests/fixtures/manifests/app-suite.yaml"
        ))
        .unwrap()
    }

    #[test]
    fn app_manifest_maps_one_to_one() {
        let m = fixture();
        let j = manifest_to_json(&m);
        let id = j.find("identity").unwrap();
        assert_eq!(id.find("kind").unwrap().as_string().as_deref(), Some("app"));
        assert_eq!(
            id.find("name").unwrap().as_string().as_deref(),
            Some("metanorma")
        );
        assert_eq!(
            id.find("producer")
                .unwrap()
                .find("tool_version")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("0.16.0")
        );
        assert_eq!(
            id.find("digest")
                .unwrap()
                .find("blob_sha256")
                .unwrap()
                .as_string()
                .as_deref()
                .map(str::len),
            Some(64)
        );
        let annotations = id.find("annotations").unwrap();
        assert_eq!(
            annotations
                .find("metanorma.org/flavor")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("full")
        );

        let provides = j.find("provides").unwrap();
        let Json::Array(eps) = provides.find("entrypoints").unwrap() else {
            panic!("entrypoints must be an array");
        };
        assert_eq!(eps.len(), 2);
        assert_eq!(
            eps[1]
                .find("runtime_requirement")
                .unwrap()
                .find("constraint")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("~> 3.3.0")
        );
        assert_eq!(
            provides.find("platforms").unwrap().as_string().as_deref(),
            None // a triplet list maps to an array, not a string
        );

        let Json::Array(reqs) = j.find("requires").unwrap() else {
            panic!("requires must be an array");
        };
        assert_eq!(reqs.len(), 2);
        assert_eq!(
            reqs[0].find("engine").unwrap().as_string().as_deref(),
            Some("ruby")
        );
        assert_eq!(
            reqs[1].find("mount").unwrap().as_string().as_deref(),
            Some("/__layers__/gtk")
        );
    }

    #[test]
    fn runtime_manifest_keeps_the_one_or_many_shape() {
        let m = PayloadManifest::from_yaml(include_str!(
            "../../tpkg/tests/fixtures/manifests/runtime.yaml"
        ))
        .unwrap();
        let j = manifest_to_json(&m);
        let provides = j.find("provides").unwrap();
        let Json::Array(engines) = provides.find("provides").unwrap() else {
            panic!("two engines map to an array");
        };
        assert_eq!(engines.len(), 2);
        assert_eq!(
            provides
                .find("built_from")
                .unwrap()
                .find("patch_set")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("v0.2.8")
        );
        assert_eq!(
            provides
                .find("env")
                .unwrap()
                .find("GEM_HOME")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("/__tebako__/gems")
        );
        assert_eq!(
            provides
                .find("capabilities")
                .unwrap()
                .find("runtime")
                .unwrap(),
            &Json::Bool(true)
        );
    }

    #[test]
    fn materialize_maps_to_json_when_present() {
        // spec 22 §4 class R (schema_minor 1): the additive key renders
        // 1:1 when declared and stays absent otherwise.
        let mut m = fixture();
        m.materialize = vec!["/lib/tebako/cacert.pem".to_string()];
        let j = manifest_to_json(&m);
        let Json::Array(paths) = j.find("materialize").unwrap() else {
            panic!("materialize must be an array");
        };
        assert_eq!(paths.len(), 1);
        assert_eq!(
            paths[0].as_string().as_deref(),
            Some("/lib/tebako/cacert.pem")
        );
        assert!(manifest_to_json(&fixture()).find("materialize").is_none());
    }

    #[test]
    fn data_manifest_maps_encryption_parts() {
        let m = PayloadManifest::from_yaml(include_str!(
            "../../tpkg/tests/fixtures/manifests/data.yaml"
        ))
        .unwrap();
        let j = manifest_to_json(&m);
        let enc = j.find("identity").unwrap().find("encryption").unwrap();
        assert_eq!(
            enc.find("state").unwrap().as_string().as_deref(),
            Some("encrypted")
        );
        let Json::Array(parts) = enc.find("parts").unwrap() else {
            panic!("parts must be an array");
        };
        assert_eq!(
            parts[0].find("algorithm").unwrap().as_string().as_deref(),
            Some("age-x25519")
        );
        assert_eq!(
            j.find("provides")
                .unwrap()
                .find("mount_semantics")
                .unwrap()
                .find("suggested")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("/usr/share/fonts")
        );
    }
}
