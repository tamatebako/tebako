//! Suite press model tests (spec 03 §6, roadmap 34): the suite file's
//! parse + validation, the type-2 package manifest with per-entry
//! runtime_refs (press-level -R fallback), and the abi guard. The heavy
//! per-entry imaging legs ride the gated e2e presses (a runtime is
//! required); the composition logic is fully covered here.

use std::path::{Path, PathBuf};

use tebako_cli::options::{PressMode, PressOptions};
use tebako_cli::resolve::Resolved;
use tebako_cli::suite::{
    check_entry_abi, entry_runtime_ref, parse_suite, suite_package_manifest, SuiteEntry, SuiteSpec,
};

fn opts() -> PressOptions {
    PressOptions {
        root_arg: String::new(),
        entrance: String::new(),
        output: None,
        prefix: PathBuf::from("/tmp/prefix"),
        cwd: None,
        ruby_requested: Some("3.3.7".to_string()),
        mode: PressMode::Lean,
        log_level: "error".to_string(),
        image_specs: Vec::new(),
        bootstrap: None,
        tebako_version: tebako_cli::DEFAULT_TEBAKO_VERSION.to_string(),
        prefer_local: false,
        verbose: false,
        devmode: true,
        fs_current: "/tmp".to_string(),
        suite: None,
        jail: None,
        no_install: false,
        format: tebako_cli::options::PressImageFormat::Dwarfs,
    }
}

fn parse(yaml: &str) -> SuiteSpec {
    parse_suite(yaml, Path::new("suite.yaml")).expect("suite parses")
}

fn err(yaml: &str) -> String {
    parse_suite(yaml, Path::new("suite.yaml"))
        .expect_err("must fail")
        .message
}

const GOOD: &str = "name: metanorma\nversion: 1.2.3\nentries:\n  - {name: metanorma, root: ./app, entry: metanorma, runtime_ref: \"ruby@3.4.2;tebako=0.15.9\"}\n  - {name: mn2pdf, root: ./mn2pdf, entry: mn2pdf}\n";

#[test]
fn parse_accepts_the_spec_shape_and_defaults() {
    let spec = parse(GOOD);
    assert_eq!(spec.name, "metanorma");
    assert_eq!(spec.version, "1.2.3");
    assert_eq!(spec.entries.len(), 2);
    assert_eq!(
        spec.entries[0].runtime_ref.as_deref(),
        Some("ruby@3.4.2;tebako=0.15.9")
    );
    assert_eq!(spec.entries[1].runtime_ref, None);

    // defaults: name from entries[0], version "0.0.0"
    let spec = parse("entries:\n  - {name: tool, root: ./app, entry: tool}\n");
    assert_eq!(spec.name, "tool");
    assert_eq!(spec.version, "0.0.0");
}

#[test]
fn parse_rejects_bad_suites_with_named_errors() {
    for (yaml, needle) in [
        ("entries: []\n", "no entries"),
        ("entries:\n  - {name: '', root: r, entry: e}\n", "path component"),
        (
            "entries:\n  - {name: 'a/b', root: r, entry: e}\n",
            "path component",
        ),
        ("entries:\n  - {name: a, root: '', entry: e}\n", "empty root"),
        ("entries:\n  - {name: a, root: r, entry: ''}\n", "empty entry"),
        (
            "entries:\n  - {name: a, root: r, entry: e}\n  - {name: a, root: r2, entry: e}\n",
            "duplicate entry name",
        ),
        (
            "entries:\n  - {name: a, root: r, entry: e, runtime_ref: \"python@3.12;tebako=0.15.9\"}\n",
            "only language in v1 is ruby",
        ),
        (
            "entries:\n  - {name: a, root: r, entry: e, runtime_ref: \"ruby@3.4.2\"}\n",
            "missing ';tebako='",
        ),
        (
            "entries:\n  - {name: a, root: r, entry: e, runtime_ref: \"ruby@3.4.2;tebako=0.15.9;sha256=abc\"}\n",
            "fat-payload checksum",
        ),
        (
            "entries:\n  - {name: a, root: r, entry: e, runtime_ref: \"ruby@3.4.2;tebako=0.15.9;bogus\"}\n",
            "unknown parameter",
        ),
    ] {
        let msg = err(yaml);
        assert!(msg.contains(needle), "expected '{needle}' in: {msg}");
    }
}

#[test]
fn parse_rejects_slot_overflow() {
    let entries = (0..9)
        .map(|i| format!("  - {{name: e{i}, root: r, entry: e}}"))
        .collect::<Vec<_>>()
        .join("\n");
    let msg = err(&format!("entries:\n{entries}\n"));
    assert!(msg.contains("at most 8 slots"), "{msg}");
}

#[test]
fn package_manifest_carries_per_entry_refs_and_slots() {
    let spec = parse(GOOD);
    let refs = vec![
        "ruby@3.4.2;tebako=0.15.9".to_string(),
        "ruby@3.3.7;tebako=0.15.9;image".to_string(),
    ];
    let pm = suite_package_manifest(&spec, &refs, "2026-07-27T00:00:00Z").unwrap();
    assert_eq!(pm.package.name, "metanorma");
    assert_eq!(pm.package.version, "1.2.3");
    assert_eq!(pm.entries.len(), 2);
    assert_eq!(pm.entries[0].name, "metanorma");
    assert_eq!(pm.entries[0].slot, Some(0));
    assert_eq!(pm.entries[0].entrypoint, "metanorma");
    assert_eq!(pm.entries[0].runtime_ref, refs[0]);
    assert_eq!(pm.entries[1].name, "mn2pdf");
    assert_eq!(pm.entries[1].slot, Some(1));
    assert_eq!(pm.entries[1].runtime_ref, refs[1]);
    // the block validates + round-trips (set_package_manifest re-validates)
    let yaml = pm.to_yaml().unwrap();
    let back = tpkg::PackageManifest::from_yaml(&yaml).unwrap();
    assert_eq!(back, pm);
}

#[test]
fn runtime_ref_falls_back_to_the_press_level_ruby() {
    let o = opts();
    let resolved = Resolved {
        executable: PathBuf::from("/tmp/ruby"),
        image: None,
    };
    let explicit = SuiteEntry {
        name: "a".to_string(),
        root: "r".to_string(),
        entry: "e".to_string(),
        runtime_ref: Some("ruby@3.4.2;tebako=0.15.9".to_string()),
    };
    assert_eq!(
        entry_runtime_ref(&explicit, "3.3.7", &o, &resolved),
        "ruby@3.4.2;tebako=0.15.9"
    );
    let fallback = SuiteEntry {
        runtime_ref: None,
        ..explicit
    };
    // the fallback rides the press's own tebako version (the default at
    // test time — version-agnostic on purpose)
    assert_eq!(
        entry_runtime_ref(&fallback, "3.3.7", &o, &resolved),
        format!("ruby@3.3.7;tebako={}", tebako_cli::DEFAULT_TEBAKO_VERSION)
    );
}

#[test]
fn explicit_refs_must_match_the_press_abi() {
    let o = opts();
    let good = SuiteEntry {
        name: "a".to_string(),
        root: "r".to_string(),
        entry: "e".to_string(),
        runtime_ref: Some(format!(
            "ruby@3.4.2;tebako={}",
            tebako_cli::DEFAULT_TEBAKO_VERSION
        )),
    };
    check_entry_abi(&good, &o).unwrap();
    let bad = SuiteEntry {
        runtime_ref: Some("ruby@3.4.2;tebako=9.9.9".to_string()),
        ..good
    };
    let e = check_entry_abi(&bad, &o).unwrap_err();
    assert!(e.message.contains("9.9.9"), "{e:?}");
    assert!(e.message.contains("abi"), "{e:?}");
}
