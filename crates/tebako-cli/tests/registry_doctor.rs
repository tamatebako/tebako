//! `tebako registry doctor` (arc 03's conformance kit): the library
//! surface against a `file://` mirror — the offline harness the fabric
//! never had. The INTACT fixture must be conformant; a drifted pin, a
//! declared-but-missing `.asc`, and a malformed document must each produce
//! their own named problem; a row the registry does not publish for this
//! host is a note, never a problem.

use std::fs;
use std::path::{Path, PathBuf};

use tebako_cli::registry::doctor;

fn scratch(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tebako-cli-regdoctor-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// sha256 of the fixture bytes, computed through the same helper the
/// doctor pins against.
fn digest(bytes: &[u8]) -> String {
    tebako_resolve::sha256_hex(bytes)
}

/// A universal-row registry over `file://`: the ref carries the pin, so
/// `--full` exercises the bytes-vs-pin leg with no network.
fn universal_registry(dir: &Path, pin: &str, signature: Option<&str>) -> String {
    let image = dir.join("demo.tfs");
    fs::write(&image, b"demo payload bytes").unwrap();
    let mut row = format!(
        "payloads:\n  - name: demo\n    kind: toolkit\n    versions:\n      - version: 1.0.0\n        platforms: universal\n        release: {{ref: \"file://{}?sha256={pin}\"}}\n",
        image.display()
    );
    if let Some(asc) = signature {
        row.push_str(&format!(
            "        signature: {{keyid: \"efc3c250f7862a48\", asc: \"{asc}\"}}\n"
        ));
    }
    format!("schema_version: 1\n{row}")
}

#[test]
fn the_intact_mirror_is_conformant() {
    let dir = scratch("intact");
    let bytes = b"demo payload bytes";
    let registry = universal_registry(&dir, &digest(bytes), None);
    let path = dir.join("tpkg-registry.yaml");
    fs::write(&path, &registry).unwrap();

    let (report, code) = doctor(path.to_str().unwrap(), true, None, false).unwrap();
    assert_eq!(code, 0, "{report}");
    assert!(
        report.contains("document: ok (1 payloads, 1 version rows)"),
        "{report}"
    );
    assert!(report.contains("bytes verify the pin"), "{report}");
    assert!(
        report.contains("tebako registry doctor: conformant"),
        "{report}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_drifted_pin_is_a_problem() {
    let dir = scratch("drift");
    let registry = universal_registry(&dir, &"0".repeat(64), None);
    let path = dir.join("tpkg-registry.yaml");
    fs::write(&path, &registry).unwrap();

    let (report, code) = doctor(path.to_str().unwrap(), true, None, false).unwrap();
    assert_eq!(code, 1, "{report}");
    assert!(report.contains("but the row pins"), "{report}");
    assert!(report.contains("1 problem(s)"), "{report}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_declared_signature_with_no_asc_is_a_problem() {
    let dir = scratch("noasc");
    let bytes = b"demo payload bytes";
    let missing = dir.join("missing.asc");
    let registry = universal_registry(
        &dir,
        &digest(bytes),
        Some(&format!("file://{}", missing.display())),
    );
    let path = dir.join("tpkg-registry.yaml");
    fs::write(&path, &registry).unwrap();

    let (report, code) = doctor(path.to_str().unwrap(), true, None, false).unwrap();
    assert_eq!(code, 1, "{report}");
    assert!(report.contains("does not serve"), "{report}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_row_the_host_is_not_published_for_is_a_note_not_a_problem() {
    let dir = scratch("hostnote");
    // The artifact-name grammar only exists on service releases, so the
    // per-triplet row rides a tfs:github: ref — the resolver refuses it
    // for THIS host before any fetch (the default pass downloads nothing,
    // so the fixture stays offline).
    let registry = format!(
        "schema_version: 1\npayloads:\n  - name: demo\n    kind: toolkit\n    versions:\n      - version: 1.0.0\n        platforms:\n          x86_64-linux-gnu:\n            artifact: demo-linux.tfs\n            sha256: \"{}\"\n        release: {{ref: \"tfs:github:o/demo:1.0.0\"}}\n",
        "a".repeat(64)
    );
    let path = dir.join("tpkg-registry.yaml");
    fs::write(&path, &registry).unwrap();

    let (report, code) = doctor(path.to_str().unwrap(), false, None, false).unwrap();
    let host_is_linux_x86 = cfg!(all(target_os = "linux", target_arch = "x86_64"));
    assert_eq!(code, 0, "{report}");
    if host_is_linux_x86 {
        assert!(report.contains("resolves to"), "{report}");
    } else {
        assert!(report.contains("note"), "{report}");
        assert!(
            report.contains("not published for the host triplet"),
            "{report}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_malformed_document_is_refused_with_the_gates_own_message() {
    let dir = scratch("bad");
    let path = dir.join("tpkg-registry.yaml");
    fs::write(&path, "schema_version: 1\npayloads:\n  - name: demo\n").unwrap();

    let (report, code) = doctor(path.to_str().unwrap(), false, None, false).unwrap();
    assert_eq!(code, 1, "{report}");
    assert!(report.contains("document: refused"), "{report}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_json_report_is_a_machine_document() {
    let dir = scratch("json");
    let bytes = b"demo payload bytes";
    let registry = universal_registry(&dir, &digest(bytes), None);
    let path = dir.join("tpkg-registry.yaml");
    fs::write(&path, &registry).unwrap();

    let (out, code) = doctor(path.to_str().unwrap(), true, None, true).unwrap();
    assert_eq!(code, 0, "{out}");
    let doc = tebako_pkg::json_parse(&out).expect("the report is JSON");
    assert_eq!(
        doc.find("registry_doctor_schema").and_then(|v| v.as_u64()),
        Some(1),
        "{out}"
    );
    assert_eq!(
        doc.find("problems").and_then(|v| v.as_u64()),
        Some(0),
        "{out}"
    );
    assert!(
        matches!(
            doc.find("document").and_then(|d| d.find("ok")),
            Some(tebako_pkg::JsonValue::Bool(true))
        ),
        "{out}"
    );
    assert_eq!(
        doc.find("transport").and_then(|v| v.as_string()).as_deref(),
        Some("file"),
        "{out}"
    );
    let Some(tebako_pkg::JsonValue::Array(findings)) = doc.find("findings") else {
        panic!("findings array missing: {out}");
    };
    assert_eq!(findings.len(), 1, "{out}");
    assert_eq!(
        findings[0]
            .find("severity")
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("ok"),
        "{out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_report_keeps_the_pass_beside_the_failure() {
    // One registry, two rows: one conformant, one drifting. The verdict is
    // a problem, and BOTH findings appear — a report that hides the pass
    // behind the failure is not evidence.
    let dir = scratch("mixed");
    let good = dir.join("good.tfs");
    let bad = dir.join("bad.tfs");
    fs::write(&good, b"good").unwrap();
    fs::write(&bad, b"bad").unwrap();
    let registry = format!(
        "schema_version: 1\npayloads:\n  - name: demo\n    kind: toolkit\n    versions:\n      - version: 1.0.0\n        platforms: universal\n        release: {{ref: \"file://{}?sha256={}\"}}\n      - version: 2.0.0\n        platforms: universal\n        release: {{ref: \"file://{}?sha256={}\"}}\n",
        good.display(),
        digest(b"good"),
        bad.display(),
        digest(b"not bad"),
    );
    let path = dir.join("tpkg-registry.yaml");
    fs::write(&path, &registry).unwrap();
    // --full samples ONE version per payload unless --sample says more,
    // so checking both rows of this fixture is an explicit ask.
    let (out, code) = doctor(path.to_str().unwrap(), true, Some(2), true).unwrap();
    assert_eq!(code, 1, "{out}");
    let doc = tebako_pkg::json_parse(&out).unwrap();
    let Some(tebako_pkg::JsonValue::Array(findings)) = doc.find("findings") else {
        panic!("findings array missing: {out}");
    };
    let severities: Vec<String> = findings
        .iter()
        .filter_map(|f| f.find("severity").and_then(|v| v.as_string()))
        .collect();
    assert_eq!(severities, vec!["ok", "problem"], "{out}");
    assert_eq!(
        doc.find("problems").and_then(|v| v.as_u64()),
        Some(1),
        "{out}"
    );
    let _ = fs::remove_dir_all(&dir);
}
