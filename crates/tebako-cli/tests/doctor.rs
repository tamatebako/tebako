//! `tebako doctor` surface tests (spec 35): synthetic TEBAKO_HOMEs, the
//! environment passed explicitly (PATH decides the shim's on-PATH check),
//! and the network section always offline — no probes in tests.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-doctor-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Fixture {
    dir: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let dir = scratch(tag);
        let home = dir.join("home");
        fs::create_dir_all(home.join("shims")).unwrap();
        fs::create_dir_all(home.join("payloads")).unwrap();
        fs::create_dir_all(home.join("runtimes")).unwrap();
        Fixture { dir, home }
    }

    /// The controlled environment: the fixture's shims dir IS on PATH
    /// (the dispatch section's PATH check passes), nothing else set.
    fn env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        env.insert(
            "PATH".to_string(),
            std::env::join_paths([self.home.join("shims").as_path(), Path::new("/usr/bin")])
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        );
        env
    }

    /// A cached payload record: image bytes + the correct sha256 trust
    /// anchor + the manifest mirror. Returns the image path.
    fn payload(&self, name: &str, version: &str) -> PathBuf {
        self.payload_with_signing(name, version, "{state: unsigned}")
    }

    /// The same record with an explicit `identity.signing` block.
    fn payload_with_signing(&self, name: &str, version: &str, signing: &str) -> PathBuf {
        let dir = self.home.join(format!("payloads/{name}"));
        fs::create_dir_all(&dir).unwrap();
        let image = dir.join(format!("{version}.tfs"));
        fs::write(&image, format!("the {name} {version} image bytes\n")).unwrap();
        fs::write(
            dir.join(format!("{version}.tfs.sha256")),
            format!(
                "{}  {version}.tfs\n",
                tebako_resolve::sha256_hex(&fs::read(&image).unwrap())
            ),
        )
        .unwrap();
        fs::write(
            dir.join(format!("{version}.manifest.yaml")),
            format!(
                "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {signing}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \"~> 3.3.0\", implementation: mri, abi: \"arm64-darwin-23\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
                "0".repeat(64),
                "0".repeat(64)
            ),
        )
        .unwrap();
        image
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn doctor_healthy_store_exits_zero() {
    let fx = Fixture::new("healthy");
    fx.payload("demo", "1.0.0");
    let (out, code) = tebako_cli::doctor::run(&fx.home, false, true, &fx.env()).unwrap();
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("tebako doctor: no problems found"), "{out}");
    // the five sections, in spec order
    for section in ["store:", "dispatch:", "network:", "trust:", "registries:"] {
        assert!(out.contains(section), "{out}");
    }
    assert!(out.contains("store root:"), "{out}");
    assert!(out.contains("store writable"), "{out}");
    assert!(out.contains("1 artifact(s) verified"), "{out}");
    // offline: the probes are skipped with a note, never run
    assert!(out.contains("offline — probes skipped"), "{out}");
}

#[test]
fn doctor_tampered_payload_is_a_named_problem() {
    let fx = Fixture::new("tampered");
    let image = fx.payload("demo", "1.0.0");
    fs::write(&image, b"tampered image bytes\n").unwrap();
    let (out, code) = tebako_cli::doctor::run(&fx.home, false, true, &fx.env()).unwrap();
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("tebako doctor: "), "{out}");
    assert!(out.contains("problem(s)"), "{out}");
    assert!(out.contains("sha256 MISMATCH"), "{out}");
    assert!(out.contains(&image.display().to_string()), "{out}");
}

#[test]
fn doctor_offline_skips_the_tls_probes() {
    let fx = Fixture::new("offline");
    let (out, code) = tebako_cli::doctor::run(&fx.home, false, true, &fx.env()).unwrap();
    assert_eq!(code, 0, "{out}");
    let network = out.split("network:").nth(1).unwrap();
    let network = network.split("trust:").next().unwrap();
    assert!(network.contains("offline — probes skipped"), "{out}");
    assert!(!network.contains("tls api.github.com"), "{out}");
}

#[test]
fn doctor_json_is_the_doctor_schema_document() {
    let fx = Fixture::new("json");
    fx.payload("demo", "1.0.0");
    let (out, code) = tebako_cli::doctor::run(&fx.home, true, true, &fx.env()).unwrap();
    assert_eq!(code, 0, "{out}");
    let doc = tebako_json::parse(out.trim()).unwrap();
    assert_eq!(doc.find("doctor_schema").unwrap().as_u64(), Some(1));
    assert_eq!(doc.find("problems").unwrap().as_u64(), Some(0));
    let sections = match doc.find("sections").unwrap() {
        tebako_json::Value::Array(items) => items,
        other => panic!("sections is not an array: {other:?}"),
    };
    let names: Vec<String> = sections
        .iter()
        .filter_map(|s| s.find("name").and_then(|n| n.as_string()))
        .collect();
    assert_eq!(
        names,
        ["store", "dispatch", "network", "trust", "registries"]
    );
    // every finding carries severity + text
    for section in sections {
        let findings = match section.find("findings").unwrap() {
            tebako_json::Value::Array(items) => items,
            other => panic!("findings is not an array: {other:?}"),
        };
        for finding in findings {
            let severity = finding.find("severity").unwrap().as_string().unwrap();
            assert!(
                ["ok", "note", "problem"].contains(&severity.as_str()),
                "{severity}"
            );
            assert!(finding.find("text").unwrap().as_string().is_some());
        }
    }
}

#[test]
fn doctor_json_reports_the_problem_count() {
    let fx = Fixture::new("json-problems");
    let image = fx.payload("demo", "1.0.0");
    fs::write(&image, b"tampered\n").unwrap();
    let (out, code) = tebako_cli::doctor::run(&fx.home, true, true, &fx.env()).unwrap();
    assert_eq!(code, 1, "{out}");
    let doc = tebako_json::parse(out.trim()).unwrap();
    let problems = doc.find("problems").unwrap().as_u64().unwrap();
    assert!(problems >= 1, "{out}");
}

#[test]
fn doctor_trust_mirrors_require_signed() {
    let fx = Fixture::new("trust");
    let env = fx.env();
    let (out, _) = tebako_cli::doctor::run(&fx.home, false, true, &env).unwrap();
    assert!(out.contains("TEBAKO_REQUIRE_SIGNED unset"), "{out}");
    let mut env = env;
    env.insert("TEBAKO_REQUIRE_SIGNED".to_string(), "1".to_string());
    let (out, _) = tebako_cli::doctor::run(&fx.home, false, true, &env).unwrap();
    assert!(out.contains("TEBAKO_REQUIRE_SIGNED=1"), "{out}");
}

#[test]
fn doctor_trust_lists_unsigned_and_signed_payloads() {
    let fx = Fixture::new("trust-listing");
    fx.payload("demo", "1.0.0");
    fx.payload_with_signing(
        "signed-app",
        "2.0.0",
        "{state: signed, keyid: \"0123456789abcdef\", mechanism: openpgp}",
    );
    let (out, code) = tebako_cli::doctor::run(&fx.home, false, true, &fx.env()).unwrap();
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 signed payload(s) in the store"), "{out}");
    assert!(
        out.contains("unsigned payload(s) in the store: demo 1.0.0"),
        "{out}"
    );
}
