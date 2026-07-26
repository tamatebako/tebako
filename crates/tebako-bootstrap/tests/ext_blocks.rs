//! Package-manifest resolution tests (spec 02 §5b / spec 03 §6): when the
//! package carries the type-2 extension block, its entries[0].runtime_ref
//! drives runtime resolution; block-less packages fall back to the v1
//! trailer field byte-identically. Handoff argv is not involved here
//! (resolution behavior only — ABI stays 1).

use tebako_bootstrap::{resolution_runtime_ref, EX_TEBAKO_MANIFEST};

fn manifest_with_ref(trailer_ref: &str) -> tpkg::Manifest {
    let mut m = tpkg::Manifest::default();
    m.set_runtime_ref(trailer_ref.as_bytes());
    m.slots
        .push(tpkg::Slot::new(0, 100, tpkg::TPKG_FORMAT_ZIP, "/app"));
    m
}

fn package_manifest(entries: &[(&str, u32, &str, &str)]) -> tpkg::PackageManifest {
    tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: "metanorma".to_string(),
            version: "1.2.3".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.16.0".to_string(),
            },
            created: "2026-07-26T00:00:00Z".to_string(),
        },
        entries: entries
            .iter()
            .map(
                |&(name, slot, entrypoint, runtime_ref)| tpkg::PackageEntry {
                    name: name.to_string(),
                    slot,
                    entrypoint: entrypoint.to_string(),
                    runtime_ref: runtime_ref.to_string(),
                },
            )
            .collect(),
        jail: None,
        env: Default::default(),
    }
}

#[test]
fn block_less_package_reads_the_v1_trailer_field() {
    let m = manifest_with_ref("ruby@3.4.2;tebako=0.15.9");
    assert_eq!(
        resolution_runtime_ref(&m).unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
    // …and an empty trailer field stays empty (the classic-bundle error
    // path in run() is unchanged).
    let m = manifest_with_ref("");
    assert_eq!(resolution_runtime_ref(&m).unwrap(), "");
}

#[test]
fn type_2_block_wins_over_the_trailer_field() {
    let mut m = manifest_with_ref("ruby@3.3.7;tebako=0.15.9");
    m.set_package_manifest(&package_manifest(&[(
        "metanorma",
        0,
        "metanorma",
        "ruby@3.4.2;tebako=0.15.9",
    )]))
    .unwrap();
    assert_eq!(
        resolution_runtime_ref(&m).unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
}

#[test]
fn type_2_block_supplies_the_ref_when_the_trailer_field_is_empty() {
    let mut m = manifest_with_ref("");
    m.set_package_manifest(&package_manifest(&[(
        "metanorma",
        0,
        "metanorma",
        "ruby@3.4.2;tebako=0.15.9",
    )]))
    .unwrap();
    assert_eq!(
        resolution_runtime_ref(&m).unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
}

#[test]
fn suites_resolve_against_entries_0() {
    let mut m = manifest_with_ref("ruby@3.3.7;tebako=0.15.9");
    m.set_package_manifest(&package_manifest(&[
        ("metanorma", 0, "metanorma", "ruby@3.4.2;tebako=0.15.9"),
        ("mn2pdf", 1, "mn2pdf", "ruby@3.3.7;tebako=0.15.9"),
    ]))
    .unwrap();
    // entries[0] drives resolution; per-entry refs beyond it are the
    // dispatcher's business (spec 07), not the v1 bootstrap's.
    assert_eq!(
        resolution_runtime_ref(&m).unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
}

#[test]
fn per_entry_refs_may_exceed_the_128_byte_trailer_limit() {
    let long_ref = format!("ruby@3.4.2;tebako=0.15.9;sha256={}", "ab".repeat(96));
    assert!(long_ref.len() > tpkg::TPKG_RUNTIME_REF_LEN);
    let mut m = manifest_with_ref("ruby@3.3.7;tebako=0.15.9");
    m.set_package_manifest(&package_manifest(&[(
        "metanorma",
        0,
        "metanorma",
        &long_ref,
    )]))
    .unwrap();
    assert_eq!(resolution_runtime_ref(&m).unwrap(), long_ref);
}

#[test]
fn corrupt_block_is_a_named_error() {
    let mut m = manifest_with_ref("ruby@3.4.2;tebako=0.15.9");
    m.ext_blocks
        .push(tpkg::ExtBlock::new(2, b"schema_version: 99\n".to_vec()).unwrap());
    let err = resolution_runtime_ref(&m).unwrap_err();
    assert_eq!(err.code, EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("extension block type 2"), "{err:?}");
}
