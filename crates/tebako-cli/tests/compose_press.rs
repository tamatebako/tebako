//! Compose press tests (spec 23 §3/§13 — the composition spectrum): the
//! document load + D5 overrides + runtime/entrypoint gates as pure units,
//! and the full closure resolution over `file://` registries (temp
//! TEBAKO_HOMEs, no network, no env mutation).

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use tebako_cli::compose;
use tebako_resolve::{sha256_hex, Fetcher};
use tpkg::{ComposePreset, Platform};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-compose-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A temp TEBAKO_HOME plus a mirror dir holding payload/registry files.
struct Fixture {
    dir: PathBuf,
    home: PathBuf,
    mirror: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let dir = scratch(tag);
        let home = dir.join("home");
        let mirror = dir.join("mirror");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&mirror).unwrap();
        Fixture { dir, home, mirror }
    }

    fn payload(&self, file: &str, bytes: &[u8]) -> String {
        fs::write(self.mirror.join(file), bytes).unwrap();
        tebako_http::file_url(&self.mirror.join(file))
    }

    fn registry(&self, file: &str, yaml: &str) -> String {
        fs::write(self.mirror.join(file), yaml).unwrap();
        tebako_http::file_url(&self.mirror.join(file))
    }

    /// Register a registry file in the home's config (the add-registry
    /// effect, without the verb's summary obligations).
    fn register(&self, reg_ref: &str) {
        tebako_cli::install::add_registry(&self.home, reg_ref).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn sha(byte: u8) -> String {
    String::from(char::from(byte)).repeat(64)
}

fn zip_image_with_manifest(manifest_yaml: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("__tpkg__/manifest.yaml", options)
        .unwrap();
    writer.write_all(manifest_yaml.as_bytes()).unwrap();
    writer.start_file("app/bin/app", options).unwrap();
    writer.write_all(b"#!/bin/sh\n").unwrap();
    writer.finish().unwrap().into_inner()
}

/// An app-kind embedded manifest; `requires` is the pre-indented edge
/// block ("" for none).
fn manifest_yaml(kind: &str, name: &str, version: &str, requires: &str) -> String {
    let provides = match kind {
        "app" => format!(
            "provides:\n  entrypoints:\n    - name: {name}\n      path: /app/bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n"
        ),
        "toolkit" => "provides:\n  platforms: universal\n  capabilities: {exec: false, read: true}\n"
            .to_string(),
        _ => "provides:\n  mount_semantics: {suggested: \"/\"}\n  capabilities: {exec: false, read: true}\n"
            .to_string(),
    };
    let requires = if requires.is_empty() {
        String::new()
    } else {
        format!("requires:\n{requires}")
    };
    format!(
        "identity:\n  schema_version: 1\n  kind: {kind}\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\n{provides}{requires}",
        sha(b'a'),
        sha(b'b')
    )
}

fn image(kind: &str, name: &str, version: &str, requires: &str) -> Vec<u8> {
    zip_image_with_manifest(&manifest_yaml(kind, name, version, requires))
}

/// A registry carrying one payload at the given versions; every ref must
/// already be written to the mirror.
fn registry_yaml(
    name: &str,
    kind: &str,
    versions: &[(&str, &str)],
    default: Option<&str>,
) -> String {
    let mut yaml = format!(
        "schema_version: 1\npayloads:\n  - name: {name}\n    kind: {kind}\n    versions:\n"
    );
    for (version, payload_ref) in versions {
        yaml.push_str(&format!(
            "      - version: {version}\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n"
        ));
        // The registry model refuses an app that declares no entrypoints
        // (the install.rs helper's shape).
        if kind == "app" {
            yaml.push_str(&format!(
                "        runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n        entrypoints: [{name}]\n"
            ));
        }
    }
    if let Some(d) = default {
        yaml.push_str(&format!("    default: {d}\n"));
    }
    yaml
}

fn doc(yaml_tail: &str) -> String {
    format!("version: 1\nruntime: {{ref: \"ruby@~> 3.3\"}}\n{yaml_tail}")
}

fn parse(yaml: &str) -> tpkg::ComposeDoc {
    let (doc, warnings) = tpkg::parse_compose(yaml).expect("the document parses");
    assert!(
        warnings.is_empty(),
        "no aliases in these fixtures: {warnings:?}"
    );
    doc
}

fn non_host_asset_name() -> String {
    Platform::ALL
        .iter()
        .find(|p| **p != Platform::host())
        .expect("the axis has more than one triplet")
        .release_asset_name()
        .to_string()
}

// ---------------------------------------------------------------------
// load / the pure gates
// ---------------------------------------------------------------------

#[test]
fn load_parses_the_document_and_surfaces_alias_warnings() {
    let fx = Fixture::new("load");
    let path = fx.dir.join("tebako.yaml");
    fs::write(&path, doc("preset: fat\n")).unwrap();
    let (parsed, warnings) = compose::load(&path).unwrap();
    assert_eq!(parsed.preset, ComposePreset::SelfContained);
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].contains("'fat' preset is deprecated"),
        "{warnings:?}"
    );

    let err = compose::load(&fx.dir.join("missing.yaml")).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message.contains("cannot read the compose document"),
        "{err}"
    );

    fs::write(&path, doc("policy: deny\n")).unwrap();
    let err = compose::load(&path).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("Phase-R"), "{err}");
}

#[test]
fn check_runtime_row_gates_the_engine_and_the_version() {
    let ok = parse(&doc(""));
    compose::check_runtime_row(&ok, "3.3.7").unwrap();

    let mismatch = parse("version: 1\nruntime: {name: ruby, requirement: \"~> 3.4\"}\n");
    let err = compose::check_runtime_row(&mismatch, "3.3.7").unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message
            .contains("does not cover the pressed ruby 3.3.7"),
        "{err}"
    );

    let python = parse("version: 1\nruntime: {ref: \"python@>= 3\"}\n");
    let err = compose::check_runtime_row(&python, "3.3.7").unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message
            .contains("ruby is the only runtime engine today"),
        "{err}"
    );
}

#[test]
fn check_entrypoint_gates_the_pointer_form() {
    let parsed = parse(&doc("entrypoint: mnconvert\n"));
    let err = compose::check_entrypoint(&parsed, "hello").unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("pointer-package"), "{err}");
    assert!(err.message.contains("N7"), "{err}");
    compose::check_entrypoint(&parsed, "mnconvert").unwrap();
    compose::check_entrypoint(&parse(&doc("")), "anything").unwrap();
}

#[test]
fn apply_overrides_rewrite_the_carry_verdicts() {
    let base = || {
        parse(&doc(
            "slices:\n  - {name: metanorma, requirement: \">= 2.0\"}\n  - {name: templates, requirement: \"3\", carry: true}\n",
        ))
    };

    // --carry=all: the runtime and every slice ride in the package
    let mut d = base();
    compose::apply_overrides(&mut d, Some("all"), None, "hello").unwrap();
    assert_eq!(d.runtime.carry, Some(true));
    assert!(d.slices.iter().all(|s| s.carry == Some(true)));

    // --carry=none: everything shares
    let mut d = base();
    compose::apply_overrides(&mut d, Some("none"), None, "hello").unwrap();
    assert_eq!(d.runtime.carry, Some(false));
    assert!(d.slices.iter().all(|s| s.carry == Some(false)));

    // named overrides, both directions
    let mut d = base();
    compose::apply_overrides(&mut d, Some("ruby"), Some("templates"), "hello").unwrap();
    assert_eq!(d.runtime.carry, Some(true));
    assert_eq!(d.slices[1].carry, Some(false));
    assert_eq!(d.slices[0].carry, None);

    // an unknown name is a named error listing the declared set
    let mut d = base();
    let err = compose::apply_overrides(&mut d, Some("wat"), None, "hello").unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("--carry names 'wat'"), "{err}");
    assert!(err.message.contains("ruby, metanorma, templates"), "{err}");

    // --share naming the local app is the pointer form — named, N7
    let mut d = base();
    let err = compose::apply_overrides(&mut d, None, Some("hello"), "hello").unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("publish it as a payload"), "{err}");

    // one slice in both flags is a conflict
    let mut d = base();
    let err = compose::apply_overrides(&mut d, Some("metanorma"), Some("metanorma"), "hello")
        .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("either carried or shared"), "{err}");
}

// ---------------------------------------------------------------------
// resolve_closure over file:// registries
// ---------------------------------------------------------------------

#[test]
fn resolve_closure_resolves_pins_and_caches_the_document_order() {
    let fx = Fixture::new("closure");
    let mn_image = image("app", "metanorma", "2.1.4", "");
    let mn_old = image("app", "metanorma", "2.0.0", "");
    let tpl_image = image("data", "templates", "3.2", "");
    let mn_ref = fx.payload("metanorma-2.1.4.tfs", &mn_image);
    let mn_old_ref = fx.payload("metanorma-2.0.0.tfs", &mn_old);
    let tpl_ref = fx.payload("templates-3.2.tfs", &tpl_image);
    fx.register(&fx.registry(
        "metanorma-registry.yaml",
        &registry_yaml(
            "metanorma",
            "app",
            &[("2.0.0", &mn_old_ref), ("2.1.4", &mn_ref)],
            None,
        ),
    ));
    fx.register(&fx.registry(
        "templates-registry.yaml",
        &registry_yaml("templates", "data", &[("3.2", &tpl_ref)], Some("3.2")),
    ));

    let doc = parse(&doc(
        "slices:\n  - {name: metanorma, requirement: \">= 2.0\"}\n  - {name: templates}\n",
    ));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();

    // document order; the newest satisfying version; shared-runtime
    // CARRIES the payload slices (only the runtime shares by default)
    assert_eq!(slices.len(), 2);
    assert_eq!(slices[0].name, "metanorma");
    assert_eq!(slices[0].version, "2.1.4");
    assert!(slices[0].carry);
    assert_eq!(slices[0].mount, None);
    assert_eq!(
        slices[0].pin,
        tpkg::DigestPin::One(sha256_hex(&mn_image)),
        "the pin is the verified universal digest"
    );
    assert_eq!(slices[1].name, "templates");
    assert_eq!(slices[1].version, "3.2");
    assert_eq!(slices[1].pin, tpkg::DigestPin::One(sha256_hex(&tpl_image)));
    for s in &slices {
        assert!(s.cache_path.is_file(), "{} cached", s.cache_path.display());
        assert!(s.source.starts_with("file://"), "{}", s.source);
        assert_eq!(
            fx.home
                .join("payloads")
                .join(&s.name)
                .join(format!("{}.tfs", s.version)),
            s.cache_path
        );
    }

    // a second resolution is a cache hit — same pins, no refetch
    let again = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(again[0].pin, slices[0].pin);
}

#[test]
fn resolve_closure_walks_the_requires_edges_and_the_doc_verdict_stands() {
    let fx = Fixture::new("deps");
    let a_image = image(
        "app",
        "appa",
        "1.0",
        "  - kind: toolkit\n    name: toolb\n    constraint: \">= 1.0\"\n    mount: /opt/toolb\n",
    );
    let b_image = image("toolkit", "toolb", "1.4", "");
    let a_ref = fx.payload("appa-1.0.tfs", &a_image);
    let b_ref = fx.payload("toolb-1.4.tfs", &b_image);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &a_ref)], Some("1.0")),
    ));
    fx.register(&fx.registry(
        "toolb-registry.yaml",
        &registry_yaml("toolb", "toolkit", &[("1.4", &b_ref)], Some("1.4")),
    ));

    // toolb NOT in the document: the discovered dep is carried, and the
    // requiring edge's mount flows into the lock row.
    let parsed = parse(&doc("slices:\n  - {name: appa, requirement: \"1.0\"}\n"));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(slices.len(), 2);
    assert_eq!(slices[1].name, "toolb");
    assert!(slices[1].carry, "a discovered dep rides in the package");
    assert_eq!(slices[1].mount.as_deref(), Some("/opt/toolb"));

    // toolb IN the document (shared): the doc verdict stands and the
    // dep edge upgrades the cache prime to a mounted share.
    let parsed = parse(&doc(
        "slices:\n  - {name: appa, requirement: \"1.0\"}\n  - {name: toolb, carry: false}\n",
    ));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(slices.len(), 2);
    assert_eq!(slices[1].name, "toolb");
    assert!(!slices[1].carry, "the document's verdict stands");
    assert_eq!(
        slices[1].mount.as_deref(),
        Some("/opt/toolb"),
        "the requiring edge's mount upgrades the cache prime"
    );
}

#[test]
fn resolve_closure_skips_an_edge_not_covering_the_target_host() {
    // spec 03 §2.3 (schema_minor 9): a requires edge whose triplets: list
    // does not cover the compose's target host contributes NO dependency
    // row — never resolved, never carried, never in the lock. toolb's
    // registry is deliberately NEVER registered: any resolution attempt
    // on the skipped edge would fail by name.
    let fx = Fixture::new("edge-skip");
    let skipped = Platform::ALL
        .iter()
        .find(|p| **p != Platform::host())
        .unwrap()
        .as_triplet();
    let host = Platform::host().as_triplet();
    let appa_image = image(
        "app",
        "appa",
        "1.0",
        &format!("  - kind: toolkit\n    name: toolb\n    constraint: \">= 1.0\"\n    triplets: [{skipped}]\n    mount: /opt/toolb\n"),
    );
    let appc_image = image(
        "app",
        "appc",
        "1.0",
        &format!("  - kind: toolkit\n    name: toolb\n    constraint: \">= 1.0\"\n    triplets: [{host}]\n    mount: /opt/toolb\n"),
    );
    let toolb_image = image("toolkit", "toolb", "1.4", "");
    let appa_ref = fx.payload("appa-1.0.tfs", &appa_image);
    let appc_ref = fx.payload("appc-1.0.tfs", &appc_image);
    let toolb_ref = fx.payload("toolb-1.4.tfs", &toolb_image);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &appa_ref)], Some("1.0")),
    ));
    fx.register(&fx.registry(
        "appc-registry.yaml",
        &registry_yaml("appc", "app", &[("1.0", &appc_ref)], Some("1.0")),
    ));
    fx.register(&fx.registry(
        "toolb-registry.yaml",
        &registry_yaml("toolb", "toolkit", &[("1.4", &toolb_ref)], Some("1.4")),
    ));

    // the skipped edge: the closure is the app alone
    let parsed = parse(&doc("slices:\n  - {name: appa, requirement: \"1.0\"}\n"));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(
        slices.len(),
        1,
        "the skipped edge contributes no dependency row"
    );
    assert_eq!(slices[0].name, "appa");

    // the covering edge: the closure resolves the dep exactly as an
    // unconditioned edge would
    let parsed = parse(&doc("slices:\n  - {name: appc, requirement: \"1.0\"}\n"));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(slices.len(), 2, "the covering edge resolves");
    assert_eq!(slices[1].name, "toolb");
    assert_eq!(slices[1].mount.as_deref(), Some("/opt/toolb"));
}

#[test]
fn resolve_closure_re_encounter_must_agree_on_the_version() {
    let fx = Fixture::new("reencounter");
    let a_image = image(
        "app",
        "appa",
        "1.0",
        "  - kind: toolkit\n    name: toolb\n    constraint: \">= 2.0\"\n",
    );
    let b_old = image("toolkit", "toolb", "1.4", "");
    let b_new = image("toolkit", "toolb", "2.1", "");
    let a_ref = fx.payload("appa-1.0.tfs", &a_image);
    let b_old_ref = fx.payload("toolb-1.4.tfs", &b_old);
    let b_new_ref = fx.payload("toolb-2.1.tfs", &b_new);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &a_ref)], Some("1.0")),
    ));
    fx.register(&fx.registry(
        "toolb-registry.yaml",
        &registry_yaml(
            "toolb",
            "toolkit",
            &[("1.4", &b_old_ref), ("2.1", &b_new_ref)],
            None,
        ),
    ));

    // The document locks toolb < 2; appa's edge wants >= 2.0 — named.
    let doc = parse(&doc(
        "slices:\n  - {name: appa, requirement: \"1.0\"}\n  - {name: toolb, requirement: \"< 2\"}\n",
    ));
    let err = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("one version per package"), "{err}");
    assert!(err.message.contains("'>= 2.0'"), "{err}");
    assert!(err.message.contains("locked 1.4"), "{err}");
}

#[test]
fn resolve_closure_mount_conflicts_are_named() {
    let fx = Fixture::new("mountconflict");
    let a_image = image(
        "app",
        "appa",
        "1.0",
        "  - kind: toolkit\n    name: toolb\n    constraint: \">= 1.0\"\n    mount: /x\n",
    );
    let c_image = image(
        "app",
        "appc",
        "1.0",
        "  - kind: toolkit\n    name: toolb\n    constraint: \">= 1.0\"\n    mount: /y\n",
    );
    let b_image = image("toolkit", "toolb", "1.4", "");
    let a_ref = fx.payload("appa-1.0.tfs", &a_image);
    let c_ref = fx.payload("appc-1.0.tfs", &c_image);
    let b_ref = fx.payload("toolb-1.4.tfs", &b_image);
    let both = format!(
        "schema_version: 1\npayloads:\n  - name: appa\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {a_ref}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n        entrypoints: [appa]\n  - name: appc\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {c_ref}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n        entrypoints: [appc]\n  - name: toolb\n    kind: toolkit\n    versions:\n      - version: 1.4\n        platforms: universal\n        release: {{ref: {b_ref}}}\n"
    );
    fx.register(&fx.registry("all-registry.yaml", &both));

    let doc = parse(&doc(
        "slices:\n  - {name: appa, requirement: \"1.0\"}\n  - {name: appc, requirement: \"1.0\"}\n",
    ));
    let err = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message.contains("mounted at both '/x' and '/y'"),
        "{err}"
    );
}

#[test]
fn resolve_closure_the_platforms_assertion_is_fail_closed() {
    let fx = Fixture::new("platforms");
    let tpl_image = image("data", "templates", "3.2", "");
    let tpl_ref = fx.payload("templates-3.2.tfs", &tpl_image);
    fx.register(&fx.registry(
        "templates-registry.yaml",
        &registry_yaml("templates", "data", &[("3.2", &tpl_ref)], Some("3.2")),
    ));

    // The assertion names a triplet that is not the host: even over a
    // universal payload the host check is fail-closed (spec 23 §13.3).
    let doc = parse(&doc(&format!(
        "slices:\n  - {{name: templates, platforms: [{}]}}\n",
        non_host_asset_name()
    )));
    let err = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message.contains("does not cover the host triplet"),
        "{err}"
    );
    assert!(err.message.contains("templates"), "{err}");
}

#[test]
fn resolve_closure_a_pin_mismatch_fails_closed_70() {
    let fx = Fixture::new("pinmismatch");
    let tpl_image = image("data", "templates", "3.2", "");
    let tpl_ref = fx.payload("templates-3.2.tfs", &tpl_image);
    fx.register(&fx.registry(
        "templates-registry.yaml",
        &registry_yaml(
            "templates",
            "data",
            &[("3.2", &format!("\"{tpl_ref}?sha256={}\"", sha(b'0')))],
            Some("3.2"),
        ),
    ));
    let doc = parse(&doc("slices:\n  - {name: templates}\n"));
    let err = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap_err();
    assert_eq!(err.code, 70, "{err:?}");
    // nothing entered the cache
    assert!(!fx.home.join("payloads").join("templates").exists());
}

#[test]
fn resolve_closure_refuses_a_runtime_kind_slice() {
    let fx = Fixture::new("runtimekind");
    let rt_ref = fx.payload("ruby-ish.tfs", b"not-an-image");
    fx.register(&fx.registry(
        "runtime-registry.yaml",
        &registry_yaml("ruby-ish", "runtime", &[("1.0", &rt_ref)], Some("1.0")),
    ));
    let doc = parse(&doc("slices:\n  - {name: ruby-ish}\n"));
    let err = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message.contains("runtime: row owns the engine"),
        "{err}"
    );
}

#[test]
fn resolve_closure_requires_a_registered_registry() {
    let fx = Fixture::new("noregistry");
    let doc = parse(&doc("slices:\n  - {name: ghost}\n"));
    let err = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message
            .contains("not carried by any registered registry"),
        "{err}"
    );
    assert!(err.message.contains("tebako add-registry"), "{err}");
}

#[test]
fn resolve_closure_self_contained_carries_everything() {
    let fx = Fixture::new("selfcontained");
    let mn_image = image("app", "metanorma", "2.1.4", "");
    let mn_ref = fx.payload("metanorma-2.1.4.tfs", &mn_image);
    fx.register(&fx.registry(
        "metanorma-registry.yaml",
        &registry_yaml("metanorma", "app", &[("2.1.4", &mn_ref)], Some("2.1.4")),
    ));
    // the preset carries every slice; the authored `carry: false`
    // overrides it (spec 23 §13.2)
    let doc = parse(
        "version: 1\npreset: self-contained\nruntime: {ref: \"ruby@~> 3.3\"}\nslices:\n  - {name: metanorma}\n",
    );
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SelfContained,
        Platform::host(),
    )
    .unwrap();
    assert!(slices[0].carry);

    let doc = parse(
        "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nslices:\n  - {name: metanorma, carry: false}\n",
    );
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &doc,
        ComposePreset::SelfContained,
        Platform::host(),
    )
    .unwrap();
    assert!(!slices[0].carry, "the authored verdict beats the preset");
}

// ---------------------------------------------------------------------
// the executable edge (spec 32 §1) — the mount axis at press
// ---------------------------------------------------------------------

#[test]
fn resolve_closure_executable_mount_axis_co_mounts_the_provider() {
    // spec 32 §1: the mount axis co-mounts the provider like a
    // toolkit/data edge; the pinned provider resolves by name and the
    // consumer-declared mount flows into the lock row.
    let fx = Fixture::new("execmount");
    let a_image = image(
        "app",
        "appa",
        "1.0",
        "  - kind: executable\n    name: xml2rfc\n    payload: xml2rfc\n    constraint: \">= 3.0\"\n    mount: /opt/xml2rfc\n",
    );
    let b_image = image("app", "xml2rfc", "3.2.1", "");
    let a_ref = fx.payload("appa-1.0.tfs", &a_image);
    let b_ref = fx.payload("xml2rfc-3.2.1.tfs", &b_image);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &a_ref)], Some("1.0")),
    ));
    fx.register(&fx.registry(
        "xml2rfc-registry.yaml",
        &registry_yaml("xml2rfc", "app", &[("3.2.1", &b_ref)], Some("3.2.1")),
    ));

    let parsed = parse(&doc("slices:\n  - {name: appa, requirement: \"1.0\"}\n"));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(slices.len(), 2);
    assert_eq!(slices[1].name, "xml2rfc");
    assert_eq!(slices[1].version, "3.2.1");
    assert!(slices[1].carry, "a discovered dep rides in the package");
    assert_eq!(slices[1].mount.as_deref(), Some("/opt/xml2rfc"));
}

#[test]
fn resolve_closure_executable_capability_scan_matches_the_entrypoint_mirror() {
    // spec 32 §1 + spec 03 §8: unpinned — the registry capability scan
    // matches the ENTRYPOINT mirror, never the payload name; exactly one
    // provider resolves.
    let fx = Fixture::new("execcap");
    let a_image = image(
        "app",
        "appa",
        "1.0",
        "  - kind: executable\n    name: xml2rfc\n    constraint: \">= 3.0\"\n    mount: /opt/xml2rfc\n",
    );
    let b_image = image("app", "xml2rfc-pkg", "3.2.1", "");
    let a_ref = fx.payload("appa-1.0.tfs", &a_image);
    let b_ref = fx.payload("xml2rfc-pkg-3.2.1.tfs", &b_image);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &a_ref)], Some("1.0")),
    ));
    let b_reg = format!(
        "schema_version: 1\npayloads:\n  - name: xml2rfc-pkg\n    kind: app\n    versions:\n      - version: 3.2.1\n        platforms: universal\n        release: {{ref: {b_ref}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n        entrypoints: [xml2rfc]\n    default: 3.2.1\n"
    );
    fx.register(&fx.registry("xml2rfc-registry.yaml", &b_reg));

    let parsed = parse(&doc("slices:\n  - {name: appa, requirement: \"1.0\"}\n"));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(slices.len(), 2);
    assert_eq!(slices[1].name, "xml2rfc-pkg");
    assert_eq!(slices[1].mount.as_deref(), Some("/opt/xml2rfc"));
}

#[test]
fn resolve_closure_executable_expose_only_is_never_co_mounted() {
    // spec 32 §1: the expose axis is a SPAWN surface — no mount, no
    // closure slice; the edge rides the embedded manifest into the lock's
    // spawned[] rows (the press's spawn walk composes them, spec 23
    // §13.6).
    let fx = Fixture::new("execexpose");
    let a_image = image(
        "app",
        "appa",
        "1.0",
        "  - kind: executable\n    name: xml2rfc\n    constraint: \">= 3.0\"\n    expose: [xml2rfc]\n",
    );
    let a_ref = fx.payload("appa-1.0.tfs", &a_image);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &a_ref)], Some("1.0")),
    ));

    let parsed = parse(&doc("slices:\n  - {name: appa, requirement: \"1.0\"}\n"));
    let slices = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap();
    assert_eq!(slices.len(), 1, "the expose axis adds no slice");
}

#[test]
fn resolve_closure_executable_capability_without_a_provider_is_a_named_error() {
    // spec 32 §1: zero registry providers is the named not-found — the
    // `payload:` pin hint rides the message.
    let fx = Fixture::new("execcapnone");
    let a_image = image(
        "app",
        "appa",
        "1.0",
        "  - kind: executable\n    name: xml2rfc\n    constraint: \">= 3.0\"\n    mount: /opt/xml2rfc\n",
    );
    let a_ref = fx.payload("appa-1.0.tfs", &a_image);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &a_ref)], Some("1.0")),
    ));

    let parsed = parse(&doc("slices:\n  - {name: appa, requirement: \"1.0\"}\n"));
    let err = compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
    .unwrap_err();
    assert!(err.message.contains("executable xml2rfc"), "{err:?}");
    assert!(err.message.contains("payload:"), "{err:?}");
}

// ---------------------------------------------------------------------
// signatures at press time (spec 09 §4's press-time point): a declared
// `signature` verifies BEFORE the bytes enter the cache and before the
// lock pins their digest; unsigned rides the loud + journaled rule.
// ---------------------------------------------------------------------

/// A registry document whose selected version carries a signature pin
/// (the full-reference asc form).
fn signed_registry_yaml(
    name: &str,
    version: &str,
    payload_ref: &str,
    keyid: &str,
    asc_ref: &str,
) -> String {
    format!(
        "schema_version: 1\npayloads:\n  - name: {name}\n    kind: app\n    versions:\n      - version: {version}\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n        signature: {{keyid: \"{keyid}\", asc: \"{asc_ref}\"}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n        entrypoints: [{name}]\n    default: {version}\n"
    )
}

/// Mint the fixture home's press-local key, sign `sign_over`, and mirror
/// the payload bytes + the detached signature. Returns the payload ref,
/// the asc ref, the signer keyid, and the public key (for registration).
fn signed_payload(
    fx: &Fixture,
    file: &str,
    bytes: &[u8],
    sign_over: &[u8],
) -> (String, String, String, Vec<u8>) {
    let key = tebako_signer::press_local_key(&fx.home).unwrap();
    let asc = tebako_signer::sign_detached(sign_over, &key.secret_key, &key.fingerprint).unwrap();
    let payload_ref = fx.payload(file, bytes);
    let asc_ref = fx.payload(&format!("{file}.asc"), &asc);
    (
        payload_ref,
        asc_ref,
        tebako_signer::hex_lower(&key.keyid),
        key.public_key.clone(),
    )
}

fn journal(fx: &Fixture) -> String {
    fs::read_to_string(fx.home.join("journal.log")).unwrap_or_default()
}

fn closure_of(
    fx: &Fixture,
    doc_yaml: &str,
) -> Result<Vec<compose::ComposeSlice>, tebako_cli::error::TebakoError> {
    let parsed = parse(&doc(doc_yaml));
    compose::resolve_closure(
        &fx.home,
        &Fetcher::new(),
        &parsed,
        ComposePreset::SharedRuntime,
        Platform::host(),
    )
}

#[test]
fn signed_slice_verifies_before_caching_and_journals_the_signer() {
    let fx = Fixture::new("signok");
    let img = image("app", "appa", "1.0", "");
    let (payload_ref, asc_ref, keyid, public) = signed_payload(&fx, "appa-1.0.tfs", &img, &img);
    tebako_signer::register_trusted(&fx.home, &public).unwrap();
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &signed_registry_yaml("appa", "1.0", &payload_ref, &keyid, &asc_ref),
    ));

    let slices = closure_of(&fx, "slices:\n  - {name: appa, requirement: \"1.0\"}\n").unwrap();
    assert_eq!(slices.len(), 1);
    assert_eq!(slices[0].pin, tpkg::DigestPin::One(sha256_hex(&img)));
    assert!(slices[0].cache_path.is_file());
    let journal = journal(&fx);
    assert!(
        journal.contains("event=payload-signature-trusted"),
        "{journal}"
    );
    assert!(journal.contains(&format!("signer={keyid}")), "{journal}");
}

#[test]
fn signed_slice_with_tampered_bytes_fails_closed_before_caching() {
    // the registry line, the bytes, AND the digest pin all swapped
    // together (the attacker holds the channel) — the signature over
    // DIFFERENT content is what fails closed, by name.
    let fx = Fixture::new("sigbad");
    let img = image("app", "appa", "1.0", "");
    let (payload_ref, asc_ref, keyid, public) =
        signed_payload(&fx, "appa-1.0.tfs", &img, b"attacker-controlled-bytes");
    tebako_signer::register_trusted(&fx.home, &public).unwrap();
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &signed_registry_yaml("appa", "1.0", &payload_ref, &keyid, &asc_ref),
    ));

    let err = closure_of(&fx, "slices:\n  - {name: appa, requirement: \"1.0\"}\n").unwrap_err();
    assert_eq!(err.code, 71, "{err:?}");
    assert!(
        err.message.contains("signature verification failed"),
        "{err}"
    );
    assert!(
        !fx.home.join("payloads/appa/1.0.tfs").exists(),
        "nothing was cached"
    );
}

#[test]
fn true_signature_over_swapped_bytes_fails_closed() {
    // the vice-versa tamper: the asc is VALID — over the release's
    // original bytes — but the served payload was swapped afterwards.
    let fx = Fixture::new("sigswap");
    let img = image("app", "appa", "1.0", "");
    let (payload_ref, asc_ref, keyid, public) = signed_payload(&fx, "appa-1.0.tfs", &img, &img);
    tebako_signer::register_trusted(&fx.home, &public).unwrap();
    fs::write(fx.mirror.join("appa-1.0.tfs"), b"swapped-after-signing").unwrap();
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &signed_registry_yaml("appa", "1.0", &payload_ref, &keyid, &asc_ref),
    ));

    let err = closure_of(&fx, "slices:\n  - {name: appa, requirement: \"1.0\"}\n").unwrap_err();
    assert_eq!(err.code, 71, "{err:?}");
    assert!(
        err.message.contains("signature verification failed"),
        "{err}"
    );
    assert!(
        !fx.home.join("payloads/appa/1.0.tfs").exists(),
        "nothing was cached"
    );
}

#[test]
fn sha_pinned_slice_with_swapped_bytes_fails_at_the_fetch_boundary() {
    // the integrity direction: the reference pins the original digest and
    // the served bytes were swapped — the sha256 anchor fails closed
    // (exit 70) before the signature step is even reached.
    let fx = Fixture::new("shaswap");
    let img = image("app", "appa", "1.0", "");
    let payload_ref = format!(
        "{}?sha256={}",
        fx.payload("appa-1.0.tfs", &img),
        sha256_hex(&img)
    );
    fs::write(fx.mirror.join("appa-1.0.tfs"), b"swapped-after-pinning").unwrap();
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &format!(
            "schema_version: 1\npayloads:\n  - name: appa\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: \"{payload_ref}\"}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n        entrypoints: [appa]\n    default: 1.0\n"
        ),
    ));

    let err = closure_of(&fx, "slices:\n  - {name: appa, requirement: \"1.0\"}\n").unwrap_err();
    assert_eq!(err.code, 70, "{err:?}");
    assert!(
        !fx.home.join("payloads/appa/1.0.tfs").exists(),
        "nothing was cached"
    );
}

#[test]
fn signed_slice_from_an_untrusted_key_fails_closed() {
    let fx = Fixture::new("signuntrusted");
    let img = image("app", "appa", "1.0", "");
    // the key is never registered — not in the trusted keyring. The
    // spec 09 §10 ceremony looks it up on the trust-anchor channel
    // first — here an empty LOCAL anchor (never the network), so the
    // refusal is the named key-retrieval failure (72).
    let (payload_ref, asc_ref, keyid, _public) = signed_payload(&fx, "appa-1.0.tfs", &img, &img);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &signed_registry_yaml("appa", "1.0", &payload_ref, &keyid, &asc_ref),
    ));
    let anchor = fx.home.join("anchor-empty");
    std::fs::create_dir_all(&anchor).unwrap();
    let old_anchor = std::env::var("TEBAKO_ANCHOR_BASE").ok();
    std::env::set_var("TEBAKO_ANCHOR_BASE", tebako_http::file_url(&anchor));
    let result = closure_of(&fx, "slices:\n  - {name: appa, requirement: \"1.0\"}\n");
    match &old_anchor {
        Some(v) => std::env::set_var("TEBAKO_ANCHOR_BASE", v),
        None => std::env::remove_var("TEBAKO_ANCHOR_BASE"),
    }
    let err = result.unwrap_err();
    assert_eq!(err.code, 72, "{err:?}");
    assert!(
        err.message.contains("trust-anchor channel"),
        "the retrieval failure names the channel: {err}"
    );
    assert!(
        err.message.contains("tebako keys import"),
        "the manual path is named: {err}"
    );
    assert!(
        !fx.home.join("payloads/appa/1.0.tfs").exists(),
        "nothing was cached"
    );
}

#[test]
fn signed_slice_with_a_changed_signer_pin_fails_closed() {
    // spec 09 §9's continuity rule: the entry pins a DIFFERENT primary
    // keyid than the verified signer's — SignerKeyChanged, exit 72.
    let fx = Fixture::new("sigchanged");
    let img = image("app", "appa", "1.0", "");
    let (payload_ref, asc_ref, _keyid, public) = signed_payload(&fx, "appa-1.0.tfs", &img, &img);
    tebako_signer::register_trusted(&fx.home, &public).unwrap();
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &signed_registry_yaml("appa", "1.0", &payload_ref, "ffffffffffffffff", &asc_ref),
    ));

    let err = closure_of(&fx, "slices:\n  - {name: appa, requirement: \"1.0\"}\n").unwrap_err();
    assert_eq!(err.code, 72, "{err:?}");
    assert!(err.message.contains("signer key changed"), "{err}");
    assert!(
        !fx.home.join("payloads/appa/1.0.tfs").exists(),
        "nothing was cached"
    );
}

#[test]
fn unsigned_slice_presses_with_the_legacy_warn_and_journal_line() {
    let fx = Fixture::new("signnone");
    let img = image("app", "appa", "1.0", "");
    let payload_ref = fx.payload("appa-1.0.tfs", &img);
    fx.register(&fx.registry(
        "appa-registry.yaml",
        &registry_yaml("appa", "app", &[("1.0", &payload_ref)], Some("1.0")),
    ));

    let slices = closure_of(&fx, "slices:\n  - {name: appa, requirement: \"1.0\"}\n").unwrap();
    assert_eq!(slices.len(), 1);
    let journal = journal(&fx);
    assert!(
        journal.contains("event=legacy-unsigned-accepted"),
        "{journal}"
    );
}
