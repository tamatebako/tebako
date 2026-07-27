//! Property tests for the payload manifest: any valid manifest survives
//! `to_yaml` → `from_yaml` unchanged, and the parser never panics on
//! garbage.

use proptest::prelude::*;
use tpkg::*;

fn arb_platform() -> impl Strategy<Value = Platform> {
    // the reserved triplet is excluded: validate() rejects it
    prop::sample::select(vec![
        Platform::Aarch64Macos,
        Platform::X86_64Macos,
        Platform::X86_64LinuxGnu,
        Platform::Aarch64LinuxGnu,
        Platform::X86_64LinuxMusl,
        Platform::Aarch64LinuxMusl,
        Platform::X86_64WindowsUcrt,
    ])
}

fn arb_triplets() -> impl Strategy<Value = Vec<Platform>> {
    prop::collection::hash_set(arb_platform(), 1..=3).prop_map(|s| s.into_iter().collect())
}

fn arb_platforms() -> impl Strategy<Value = Platforms> {
    prop_oneof![
        Just(Platforms::Universal),
        arb_triplets().prop_map(Platforms::Triplets),
    ]
}

fn arb_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9-]{0,11}").unwrap()
}

fn arb_hex(len: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(0u8..16, len..=len).prop_map(|nibbles| {
        nibbles
            .into_iter()
            .map(|n| b"0123456789abcdef"[n as usize] as char)
            .collect()
    })
}

fn arb_constraint() -> impl Strategy<Value = Constraint> {
    let op = prop::sample::select(vec![">=", "<=", "~>", ">", "<", "!=", "=", ""]);
    let clause = (op, prop::collection::vec(0u32..100, 1..=3)).prop_map(|(op, nums)| {
        let version = nums
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".");
        if op.is_empty() {
            version
        } else {
            format!("{op} {version}")
        }
    });
    prop::collection::vec(clause, 1..=2)
        .prop_map(|clauses| Constraint::new(&clauses.join(", ")).unwrap())
}

fn arb_path() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_name(), 1..=3).prop_map(|segs| format!("/{}", segs.join("/")))
}

fn arb_signing() -> impl Strategy<Value = Signing> {
    prop_oneof![
        Just(Signing {
            state: SigningState::Unsigned,
            keyid: None,
            mechanism: None,
        }),
        arb_hex(16).prop_map(|keyid| Signing {
            state: SigningState::Signed,
            keyid: Some(keyid),
            mechanism: Some(SigningMechanism::Openpgp),
        }),
    ]
}

fn arb_encryption() -> impl Strategy<Value = Encryption> {
    let part = (
        prop::collection::vec(arb_path(), 1..=2),
        prop::sample::select(vec![
            "age-x25519".to_string(),
            "age-x25519-kyber".to_string(),
        ]),
        prop::collection::vec(arb_name(), 1..=2),
    )
        .prop_map(|(paths, algorithm, envelope_refs)| EncryptionPart {
            paths,
            algorithm,
            envelope_refs,
        });
    prop_oneof![
        Just(Encryption {
            state: EncryptionState::None,
            parts: Vec::new(),
        }),
        prop::collection::vec(part, 1..=2).prop_map(|parts| Encryption {
            state: EncryptionState::Encrypted,
            parts,
        }),
    ]
}

fn arb_annotation_value() -> impl Strategy<Value = serde_yml::Value> {
    prop_oneof![
        any::<bool>().prop_map(serde_yml::Value::from),
        any::<i64>().prop_map(serde_yml::Value::from),
        arb_name().prop_map(serde_yml::Value::from),
    ]
}

fn arb_identity(kind: PayloadKind) -> impl Strategy<Value = Identity> {
    (
        arb_name(),
        arb_name(),
        arb_signing(),
        arb_encryption(),
        arb_hex(64),
        arb_hex(64),
        prop::collection::btree_map(arb_name(), arb_annotation_value(), 0..3),
    )
        .prop_map(
            move |(name, tool, signing, encryption, tree, blob, annotations)| Identity {
                schema_version: PAYLOAD_SCHEMA_VERSION,
                kind,
                name,
                version: "1.0.0".to_string(),
                producer: Producer {
                    tool,
                    tool_version: "0.16.0".to_string(),
                },
                created: "2026-07-26T00:00:00Z".to_string(),
                source: None,
                sbom: None,
                digest: Digest {
                    tree_hash: format!("sha256:{tree}"),
                    blob_sha256: blob,
                },
                signing,
                encryption,
                annotations,
            },
        )
}

fn arb_provides(kind: PayloadKind) -> impl Strategy<Value = Provides> {
    let entrypoint =
        (arb_name(), arb_path(), arb_constraint()).prop_map(|(name, path, c)| Entrypoint {
            name,
            path,
            args_default: Vec::new(),
            runtime_requirement: Some(RuntimeRequirement {
                engine: "ruby".to_string(),
                constraint: c,
            }),
        });
    let app = (prop::collection::vec(entrypoint, 1..=3), arb_platforms()).prop_map(
        |(entrypoints, platforms)| {
            Provides::App(AppProvides {
                entrypoints,
                platforms,
                capabilities: Capabilities {
                    exec: true,
                    read: true,
                    runtime: None,
                },
            })
        },
    );
    let engine = arb_platform().prop_map(|platform| EngineProvides {
        engine: "ruby".to_string(),
        version: "4.0.6".to_string(),
        abi_line: "4.0".to_string(),
        platform,
    });
    let runtime = (
        prop::collection::vec(engine, 1..=3),
        arb_hex(64),
        prop::collection::btree_map(arb_name(), arb_name(), 0..2),
    )
        .prop_map(|(provides, src, env)| {
            Provides::Runtime(RuntimeProvides {
                provides,
                built_from: BuiltFrom {
                    src_sha256: src,
                    patch_set: "v0.2.8".to_string(),
                },
                env,
                capabilities: Capabilities {
                    exec: true,
                    read: true,
                    runtime: Some(true),
                },
            })
        });
    let data = (arb_path(), prop::collection::vec(arb_name(), 0..=2)).prop_map(|(s, consumers)| {
        Provides::Data(DataProvides {
            mount_semantics: MountSemantics { suggested: s },
            consumers,
            capabilities: Capabilities {
                exec: false,
                read: true,
                runtime: None,
            },
        })
    });
    let other = prop::collection::btree_map(arb_name(), arb_annotation_value(), 0..2)
        .prop_map(Provides::Other);
    match kind {
        PayloadKind::App => app.boxed(),
        PayloadKind::Runtime => runtime.boxed(),
        PayloadKind::Data => data.boxed(),
        PayloadKind::Toolkit | PayloadKind::Language => other.boxed(),
    }
}

fn arb_requirement() -> impl Strategy<Value = Requirement> {
    prop_oneof![
        (arb_name(), arb_constraint())
            .prop_map(|(engine, constraint)| { Requirement::Language { engine, constraint } }),
        (arb_name(), arb_constraint(), arb_triplets(), arb_path()).prop_map(
            |(name, constraint, triplets, mount)| Requirement::Toolkit {
                name,
                constraint,
                triplets: Some(triplets),
                mount: Some(mount),
            }
        ),
        (arb_name(), arb_constraint(), arb_path()).prop_map(|(name, constraint, mount)| {
            Requirement::Data {
                name,
                constraint,
                mount: Some(mount),
            }
        }),
    ]
}

fn arb_manifest() -> impl Strategy<Value = PayloadManifest> {
    prop::sample::select(vec![
        PayloadKind::App,
        PayloadKind::Runtime,
        PayloadKind::Data,
        PayloadKind::Toolkit,
        PayloadKind::Language,
    ])
    .prop_flat_map(|kind| {
        (
            arb_identity(kind),
            arb_provides(kind),
            prop::collection::vec(arb_requirement(), 0..=3),
        )
    })
    .prop_map(|(identity, provides, requires)| PayloadManifest {
        identity,
        provides,
        requires,
    })
}

proptest! {
    #[test]
    fn yaml_roundtrip(m in arb_manifest()) {
        let rendered = m.to_yaml().expect("serialize own model");
        let back = PayloadManifest::from_yaml(&rendered).expect("parse own output");
        prop_assert_eq!(&back, &m);
    }

    #[test]
    fn parser_never_panics_on_garbage(data in prop::collection::vec(any::<u8>(), 0..4096)) {
        // Must return an error or a manifest, but never panic/overflow.
        let text = String::from_utf8_lossy(&data);
        let _ = PayloadManifest::from_yaml(&text);
    }

    #[test]
    fn parser_never_panics_on_structured_garbage(
        // YAML-ish garbage: random tokens over the manifest's own alphabet.
        parts in prop::collection::vec(
            prop::sample::select(vec![
                "identity:", "provides:", "requires:", "kind:", "app", "data",
                "schema_version: 1", "capabilities:", "{exec: true}", "- ", "[", "]",
                "{", "}", "platforms:", "universal", "~> 3.3.0", "null", "~",
            ]),
            0..24,
        ),
    ) {
        let text = parts.join(" ");
        let _ = PayloadManifest::from_yaml(&text);
    }
}
