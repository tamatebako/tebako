//! Package-manifest resolution tests (spec 02 §5b / spec 03 §6): when the
//! package carries the type-2 extension block, argv0 selects the entry
//! whose runtime_ref drives runtime resolution (exact name match,
//! entries[0] fallback); block-less packages fall back to the v1 trailer
//! field byte-identically. Handoff argv shape is not involved here
//! (resolution behavior only — ABI stays 1).

use tebako_bootstrap::{resolution_runtime_ref, select_entry, EX_TEBAKO_MANIFEST};

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
                    slot: Some(slot),
                    entrypoint: entrypoint.to_string(),
                    runtime_ref: runtime_ref.to_string(),
                },
            )
            .collect(),
        jail: None,
        env: Default::default(),
        lock: None,
        mounts: Vec::new(),
    }
}

#[test]
fn block_less_package_reads_the_v1_trailer_field() {
    let m = manifest_with_ref("ruby@3.4.2;tebako=0.15.9");
    assert_eq!(
        resolution_runtime_ref(&m, "metanorma").unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
    // …and an empty trailer field stays empty (the classic-bundle error
    // path in run() is unchanged).
    let m = manifest_with_ref("");
    assert_eq!(resolution_runtime_ref(&m, "metanorma").unwrap(), "");
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
        resolution_runtime_ref(&m, "metanorma").unwrap(),
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
        resolution_runtime_ref(&m, "metanorma").unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
}

#[test]
fn argv0_selects_the_suite_entry() {
    let mut m = manifest_with_ref("ruby@3.3.7;tebako=0.15.9");
    m.slots
        .push(tpkg::Slot::new(100, 100, tpkg::TPKG_FORMAT_ZIP, "/app"));
    m.set_package_manifest(&package_manifest(&[
        ("metanorma", 0, "metanorma", "ruby@3.4.2;tebako=0.15.9"),
        ("mn2pdf", 1, "mn2pdf", "ruby@3.3.7;tebako=0.15.9"),
    ]))
    .unwrap();

    // argv0 is the selector (spec 07 §2.0): each entry resolves ITS OWN
    // runtime_ref — two commands of one package, two runtimes.
    assert_eq!(
        resolution_runtime_ref(&m, "metanorma").unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
    assert_eq!(
        resolution_runtime_ref(&m, "mn2pdf").unwrap(),
        "ruby@3.3.7;tebako=0.15.9"
    );
    // full paths select by file name; a windows .exe suffix is stripped
    assert_eq!(
        resolution_runtime_ref(&m, "/usr/local/bin/mn2pdf").unwrap(),
        "ruby@3.3.7;tebako=0.15.9"
    );
    assert_eq!(
        resolution_runtime_ref(&m, "C:\\bin\\mn2pdf.exe").unwrap(),
        "ruby@3.3.7;tebako=0.15.9"
    );
}

#[test]
fn unknown_argv0_falls_back_to_entries_0() {
    let mut m = manifest_with_ref("ruby@3.3.7;tebako=0.15.9");
    m.slots
        .push(tpkg::Slot::new(100, 100, tpkg::TPKG_FORMAT_ZIP, "/app"));
    m.set_package_manifest(&package_manifest(&[
        ("metanorma", 0, "metanorma", "ruby@3.4.2;tebako=0.15.9"),
        ("mn2pdf", 1, "mn2pdf", "ruby@3.3.7;tebako=0.15.9"),
    ]))
    .unwrap();
    // the standalone-download case: the file name is arbitrary
    // (version/platform suffixes) — the primary command runs.
    assert_eq!(
        resolution_runtime_ref(&m, "metanorma-1.2.3-macos-arm64").unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );
    assert_eq!(
        resolution_runtime_ref(&m, "anything-at-all").unwrap(),
        "ruby@3.4.2;tebako=0.15.9"
    );

    // select_entry itself: exact match else entries[0]
    let pm = package_manifest(&[
        ("metanorma", 0, "metanorma", "ruby@3.4.2;tebako=0.15.9"),
        ("mn2pdf", 1, "mn2pdf", "ruby@3.3.7;tebako=0.15.9"),
    ]);
    assert_eq!(select_entry(&pm, "mn2pdf").name, "mn2pdf");
    assert_eq!(select_entry(&pm, "bogus").name, "metanorma");
}

#[test]
fn an_entry_naming_a_missing_slot_is_a_named_error() {
    let mut m = manifest_with_ref("ruby@3.4.2;tebako=0.15.9");
    m.set_package_manifest(&package_manifest(&[(
        "metanorma",
        3, // the container carries only slot 0
        "metanorma",
        "ruby@3.4.2;tebako=0.15.9",
    )]))
    .unwrap();
    let err = resolution_runtime_ref(&m, "metanorma").unwrap_err();
    assert_eq!(err.code, EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("slot 3"), "{err:?}");
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
    assert_eq!(resolution_runtime_ref(&m, "metanorma").unwrap(), long_ref);
}

#[test]
fn corrupt_block_is_a_named_error() {
    let mut m = manifest_with_ref("ruby@3.4.2;tebako=0.15.9");
    m.ext_blocks
        .push(tpkg::ExtBlock::new(2, b"schema_version: 99\n".to_vec()).unwrap());
    let err = resolution_runtime_ref(&m, "metanorma").unwrap_err();
    assert_eq!(err.code, EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("extension block type 2"), "{err:?}");
}

// ---------------------------------------------------------------------
// the entry-aware handoff (suite mount rule)
// ---------------------------------------------------------------------

fn suite_manifest() -> tpkg::Manifest {
    let mut m = manifest_with_ref("ruby@3.4.2;tebako=0.15.9");
    m.slots
        .push(tpkg::Slot::new(100, 100, tpkg::TPKG_FORMAT_ZIP, "/app"));
    m
}

#[test]
fn handoff_mounts_only_the_selected_entrys_slot() {
    let m = suite_manifest();
    let pm = package_manifest(&[
        ("metanorma", 0, "metanorma", "ruby@3.4.2;tebako=0.15.9"),
        ("mn2pdf", 1, "mn2pdf", "ruby@3.3.7;tebako=0.15.9"),
    ]);
    let self_path = std::path::Path::new("/pkg/suite");
    let runtime = std::path::Path::new("/rt/ruby");

    // selecting mn2pdf mounts slot 1 only, entry is mn2pdf's entrypoint
    let selection =
        tebako_bootstrap::package_selection(&m.with_package_manifest(&pm), "mn2pdf").unwrap();
    let argv = tebako_bootstrap::handoff_argv(
        runtime,
        self_path,
        &m.with_package_manifest(&pm),
        selection.as_ref(),
        &["mn2pdf".to_string(), "in.xml".to_string()],
        None,
        &[],
    );
    assert_eq!(
        argv,
        vec![
            runtime.to_string_lossy().into_owned(),
            "--tebako-image".to_string(),
            "/pkg/suite:1:/app".to_string(),
            "--tebako-entry".to_string(),
            "mn2pdf".to_string(),
            "in.xml".to_string(),
        ]
    );

    // selecting metanorma mounts slot 0 only
    let selection =
        tebako_bootstrap::package_selection(&m.with_package_manifest(&pm), "metanorma").unwrap();
    let argv = tebako_bootstrap::handoff_argv(
        runtime,
        self_path,
        &m.with_package_manifest(&pm),
        selection.as_ref(),
        &["metanorma".to_string()],
        None,
        &[],
    );
    assert_eq!(
        argv,
        vec![
            runtime.to_string_lossy().into_owned(),
            "--tebako-image".to_string(),
            "/pkg/suite:0:/app".to_string(),
            "--tebako-entry".to_string(),
            "metanorma".to_string(),
        ]
    );
}

#[test]
fn handoff_without_a_package_manifest_is_byte_identical_to_v1() {
    let m = suite_manifest(); // no block
    let argv = tebako_bootstrap::handoff_argv(
        std::path::Path::new("/rt/ruby"),
        std::path::Path::new("/pkg/app"),
        &m,
        None,
        &["./app".to_string(), "go".to_string()],
        None,
        &[],
    );
    assert_eq!(
        argv,
        vec![
            "/rt/ruby".to_string(),
            "--tebako-image".to_string(),
            "/pkg/app:0:/app".to_string(),
            "--tebako-image".to_string(),
            "/pkg/app:1:/app".to_string(),
            "--tebako-entry".to_string(),
            "./app".to_string(),
            "go".to_string(),
        ]
    );
}

/// Helper: a manifest carrying the package manifest block (clone-on-write
/// for the tests).
trait WithPackageManifest {
    fn with_package_manifest(&self, pm: &tpkg::PackageManifest) -> tpkg::Manifest;
}
impl WithPackageManifest for tpkg::Manifest {
    fn with_package_manifest(&self, pm: &tpkg::PackageManifest) -> tpkg::Manifest {
        let mut m = self.clone();
        m.set_package_manifest(pm).unwrap();
        m
    }
}
