//! Version resolution chain precedence (spec 07 §2.1):
//! TEBAKO_<TOOL>_VERSION env > nearest .tebako-tools.yaml walking up from
//! cwd > user default (~/.tebako/config.yaml) > registry default.

mod common;

use common::*;
use tebako_shim::resolve::{self, VersionSource};

fn seed_metanorma(home: &std::path::Path) {
    for v in ["1.0.0", "1.2.2", "1.2.3"] {
        write_payload(
            home,
            "metanorma",
            v,
            &app_manifest("metanorma", v, &entrypoint_yaml(RUBY_ENTRY, "metanorma")),
        );
    }
}

fn seed_registry(root: &std::path::Path, default: &str) -> std::path::PathBuf {
    let reg = root.join("tpkg-registry.yaml");
    std::fs::write(
        &reg,
        format!(
            "schema_version: 1\npayloads:\n  - name: metanorma\n    kind: app\n    default: {default}\n    versions:\n      - version: {default}\n        platforms: universal\n        release: {{ref: file:///metanorma-{default}.tfs}}\n        entrypoints: [metanorma]\n"
        ),
    )
    .expect("registry");
    reg
}

#[test]
fn env_beats_project_and_config() {
    let tmp = TempDir::new("env-beats");
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    seed_metanorma(&home);
    std::fs::write(proj.join(".tebako-tools.yaml"), "metanorma: 1.2.2\n").unwrap();
    write_config(&home, "defaults: {metanorma: 1.0.0}\n");
    let mut ctx = ctx(&home, &proj);
    ctx.env
        .insert("TEBAKO_METANORMA_VERSION".into(), "1.2.3".into());

    let res = resolve::resolve("metanorma", &ctx).unwrap();
    assert_eq!(res.version, "1.2.3");
    assert!(matches!(res.source, VersionSource::Env(_)));
}

#[test]
fn project_beats_config_and_nearest_wins() {
    let tmp = TempDir::new("project-beats");
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    let nested = proj.join("a").join("b");
    std::fs::create_dir_all(&nested).unwrap();
    seed_metanorma(&home);
    write_config(&home, "defaults: {metanorma: 1.0.0}\n");
    std::fs::write(proj.join(".tebako-tools.yaml"), "metanorma: 1.2.2\n").unwrap();
    std::fs::write(nested.join(".tebako-tools.yaml"), "metanorma: 1.2.3\n").unwrap();

    // nearest project file wins (and beats the config default)
    let cwd = nested.join("deeper");
    std::fs::create_dir_all(&cwd).unwrap();
    let res = resolve::resolve("metanorma", &ctx(&home, &cwd)).unwrap();
    assert_eq!(res.version, "1.2.3");
    assert!(matches!(res.source, VersionSource::ProjectFile(_)));

    // remove the nearer pin → the farther one applies
    let res = resolve::resolve("metanorma", &ctx(&home, &proj)).unwrap();
    assert_eq!(res.version, "1.2.2");

    // a nearer file that does not pin the tool does not shadow
    std::fs::write(nested.join(".tebako-tools.yaml"), "other-tool: 9.9\n").unwrap();
    let res = resolve::resolve("metanorma", &ctx(&home, &cwd)).unwrap();
    assert_eq!(res.version, "1.2.2");
}

#[test]
fn config_beats_registry() {
    let tmp = TempDir::new("config-beats");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let reg = seed_registry(tmp.path(), "1.0.0");
    write_config(
        &home,
        &format!(
            "defaults: {{metanorma: 1.2.3}}\nregistries:\n  - {}\n",
            tebako_http::file_url(&reg)
        ),
    );
    let res = resolve::resolve("metanorma", &ctx(&home, tmp.path())).unwrap();
    assert_eq!(res.version, "1.2.3");
    assert!(matches!(res.source, VersionSource::UserDefault));
}

#[test]
fn registry_default_is_the_last_resort() {
    let tmp = TempDir::new("registry-last");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let reg = seed_registry(tmp.path(), "1.0.0");
    write_config(
        &home,
        &format!("registries:\n  - {}\n", tebako_http::file_url(&reg)),
    );
    let res = resolve::resolve("metanorma", &ctx(&home, tmp.path())).unwrap();
    assert_eq!(res.version, "1.0.0");
    assert!(matches!(res.source, VersionSource::RegistryDefault(_)));
}

#[test]
fn env_pin_of_an_uninstalled_version_is_a_named_error() {
    let tmp = TempDir::new("env-uninstalled");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_METANORMA_VERSION".into(), "9.9.9".into());
    let err = resolve::resolve("metanorma", &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("9.9.9"), "{}", err.message);
    assert!(err.message.contains("not installed"), "{}", err.message);
}

#[test]
fn disabled_version_is_refused_and_enable_restores() {
    let tmp = TempDir::new("disabled");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_METANORMA_VERSION".into(), "1.2.3".into());

    tebako_shim::run(
        &[
            "tebako-shim".into(),
            "disable".into(),
            "metanorma@1.2.3".into(),
        ],
        &ctx,
    )
    .unwrap();
    let err = resolve::resolve("metanorma", &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("disabled"), "{}", err.message);

    // another version of the same tool still resolves
    ctx.env
        .insert("TEBAKO_METANORMA_VERSION".into(), "1.2.2".into());
    assert_eq!(
        resolve::resolve("metanorma", &ctx).unwrap().version,
        "1.2.2"
    );

    tebako_shim::run(
        &[
            "tebako-shim".into(),
            "enable".into(),
            "metanorma@1.2.3".into(),
        ],
        &ctx,
    )
    .unwrap();
    ctx.env
        .insert("TEBAKO_METANORMA_VERSION".into(), "1.2.3".into());
    assert_eq!(
        resolve::resolve("metanorma", &ctx).unwrap().version,
        "1.2.3"
    );
}

#[test]
fn unknown_tool_points_at_doctor() {
    let tmp = TempDir::new("unknown-tool");
    let home = tmp.path().join("home");
    let err = resolve::resolve("nosuchtool", &ctx(&home, tmp.path())).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(
        err.message.contains("tebako-shim doctor"),
        "{}",
        err.message
    );
}

#[test]
fn nothing_in_the_chain_is_a_named_error() {
    let tmp = TempDir::new("chain-empty");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let err = resolve::resolve("metanorma", &ctx(&home, tmp.path())).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(
        err.message.contains("no version resolved"),
        "{}",
        err.message
    );
}

#[test]
fn suite_commands_map_to_their_own_payload() {
    let tmp = TempDir::new("suite");
    let home = tmp.path().join("home");
    // one package, N entrypoints → N commands (spec 07 §2.0)
    let manifest = app_manifest(
        "metasuite",
        "2.0.0",
        "  entrypoints:\n    - name: alpha\n      path: /app/bin/alpha\n    - name: beta\n      path: /app/bin/beta\n",
    );
    write_payload(&home, "metasuite", "2.0.0", &manifest);
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert("TEBAKO_BETA_VERSION".into(), "2.0.0".into());

    let res = resolve::resolve("beta", &ctx).unwrap();
    assert_eq!(res.payload_name, "metasuite");
    assert_eq!(res.version, "2.0.0");
    assert_eq!(
        res.manifest.entrypoint("beta").unwrap().path,
        "/app/bin/beta"
    );
}

// ---------------------------------------------------------------------
// The 2026-09-05 routing amendment (spec 07 §2 step 0.5): qualified
// `[payload@]version` pins + per-payload disable route the PROVIDER.
// ---------------------------------------------------------------------

fn seed_two_providers(home: &std::path::Path) {
    write_payload(
        home,
        "pandora",
        "1.0.0",
        &app_manifest("pandora", "1.0.0", &entrypoint_yaml(NATIVE_ENTRY, "pandoc")),
    );
    write_payload(
        home,
        "pandorc",
        "1.2.0",
        &app_manifest("pandorc", "1.2.0", &entrypoint_yaml(NATIVE_ENTRY, "pandoc")),
    );
}

fn write_disabled(home: &std::path::Path, yaml: &str) {
    let shims = home.join("shims");
    std::fs::create_dir_all(&shims).unwrap();
    std::fs::write(shims.join(".disabled.yaml"), yaml).unwrap();
}

#[test]
fn two_providers_unqualified_is_the_collision_error_with_the_routing_hint() {
    let tmp = TempDir::new("collision");
    let home = tmp.path().join("home");
    seed_two_providers(&home);
    let err = resolve::resolve("pandoc", &ctx(&home, tmp.path())).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("more than one"), "{}", err.message);
    assert!(
        err.message.contains("pandoc: <payload>@<version>"),
        "{}",
        err.message
    );
    assert!(
        err.message
            .contains("tebako-shim disable pandoc --of <payload>"),
        "{}",
        err.message
    );
}

#[test]
fn a_qualified_project_pin_routes_the_named_provider() {
    let tmp = TempDir::new("pin-project");
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    seed_two_providers(&home);
    std::fs::write(proj.join(".tebako-tools.yaml"), "pandoc: pandorc@1.2.0\n").unwrap();

    let res = resolve::resolve("pandoc", &ctx(&home, &proj)).unwrap();
    assert_eq!(res.payload_name, "pandorc");
    assert_eq!(res.version, "1.2.0");
    assert!(matches!(res.source, VersionSource::ProjectFile(_)));
    assert!(matches!(res.provider, resolve::ProviderKind::Pinned));
}

#[test]
fn an_qualified_env_pin_overrides_the_project_pin() {
    let tmp = TempDir::new("pin-env");
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    seed_two_providers(&home);
    std::fs::write(proj.join(".tebako-tools.yaml"), "pandoc: pandorc@1.2.0\n").unwrap();
    let mut ctx = ctx(&home, &proj);
    ctx.env
        .insert("TEBAKO_PANDOC_VERSION".into(), "pandora@1.0.0".into());

    let res = resolve::resolve("pandoc", &ctx).unwrap();
    assert_eq!(res.payload_name, "pandora");
    assert_eq!(res.version, "1.0.0");
    assert!(matches!(res.source, VersionSource::Env(_)));
}

#[test]
fn a_pin_naming_a_non_provider_is_the_notaprovider_error() {
    let tmp = TempDir::new("pin-notaprovider");
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    seed_two_providers(&home);
    write_payload(
        &home,
        "othertools",
        "9.9",
        &app_manifest("othertools", "9.9", &entrypoint_yaml(NATIVE_ENTRY, "other")),
    );
    std::fs::write(proj.join(".tebako-tools.yaml"), "pandoc: othertools@9.9\n").unwrap();

    let err = resolve::resolve("pandoc", &ctx(&home, &proj)).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("othertools@9.9"), "{}", err.message);
    assert!(err.message.contains("NotAProvider"), "{}", err.message);
    assert!(err.message.contains("pandoc"), "{}", err.message);
}

#[test]
fn a_pin_naming_an_uninstalled_version_is_the_not_installed_error() {
    let tmp = TempDir::new("pin-uninstalled");
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    seed_two_providers(&home);
    std::fs::write(proj.join(".tebako-tools.yaml"), "pandoc: pandorc@9.9.9\n").unwrap();

    let err = resolve::resolve("pandoc", &ctx(&home, &proj)).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("9.9.9"), "{}", err.message);
    assert!(err.message.contains("not installed"), "{}", err.message);
}

#[test]
fn an_unparseable_chain_value_is_the_named_grammar_error() {
    let tmp = TempDir::new("pin-bad");
    let home = tmp.path().join("home");
    seed_metanorma(&home);
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_METANORMA_VERSION".into(), "metanorma@".into());
    let err = resolve::resolve("metanorma", &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(
        err.message.contains("TEBAKO_METANORMA_VERSION"),
        "{}",
        err.message
    );
    assert!(err.message.contains("metanorma@"), "{}", err.message);
}

#[test]
fn disabling_all_but_one_claim_routes_without_a_pin() {
    let tmp = TempDir::new("route-by-disable");
    let home = tmp.path().join("home");
    seed_two_providers(&home);
    // `tebako-shim disable pandoc --of pandorc` writes exactly this.
    write_disabled(&home, "pandoc:\n  - pandorc@all\n");
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_PANDOC_VERSION".into(), "1.0.0".into());

    let res = resolve::resolve("pandoc", &ctx).unwrap();
    assert_eq!(res.payload_name, "pandora");
    assert_eq!(res.version, "1.0.0");
    assert!(matches!(res.provider, resolve::ProviderKind::Own));
}

#[test]
fn both_claims_disabled_is_the_no_provider_error() {
    let tmp = TempDir::new("route-all-disabled");
    let home = tmp.path().join("home");
    seed_two_providers(&home);
    write_disabled(&home, "pandoc:\n  - pandorc@all\n  - pandora@all\n");
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_PANDOC_VERSION".into(), "1.0.0".into());

    let err = resolve::resolve("pandoc", &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(
        err.message
            .contains("no installed payload provides or exposes"),
        "{}",
        err.message
    );
}

// spec 30 §3 exposers: the SAME routing surface (S9).

const JAVA_EDGE: &str =
    "requires:\n  - {kind: runtime, engine: java, constraint: \">= 21\", expose: [java]}\n";

fn seed_two_exposers(home: &std::path::Path) {
    for (name, version) in [("expa", "1.0.0"), ("expb", "2.0.0")] {
        write_payload(
            home,
            name,
            version,
            &app_manifest_requires(
                name,
                version,
                &entrypoint_yaml(NATIVE_ENTRY, name),
                JAVA_EDGE,
            ),
        );
    }
}

#[test]
fn two_exposers_route_through_the_same_pin_and_disable_surface() {
    let tmp = TempDir::new("route-exposed");
    let home = tmp.path().join("home");
    seed_two_exposers(&home);

    // unqualified → the collision error with the routing hint
    let err = resolve::resolve("java", &ctx(&home, tmp.path())).unwrap_err();
    assert!(
        err.message.contains("exposed by more than one"),
        "{}",
        err.message
    );
    assert!(
        err.message
            .contains("tebako-shim disable java --of <payload>"),
        "{}",
        err.message
    );

    // a qualified env pin routes the named exposer
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_JAVA_VERSION".into(), "expb@2.0.0".into());
    let res = resolve::resolve("java", &ctx).unwrap();
    assert_eq!(res.payload_name, "expb");
    assert_eq!(res.version, "2.0.0");
    assert!(matches!(res.provider, resolve::ProviderKind::Pinned));
    assert!(res.exposed.is_some(), "the expose edge rides");

    // disabling expb's claim routes to expa with a bare-version pin
    write_disabled(&home, "java:\n  - expb@all\n");
    let mut ctx2 = common::ctx(&home, tmp.path());
    ctx2.env
        .insert("TEBAKO_JAVA_VERSION".into(), "1.0.0".into());
    let res = resolve::resolve("java", &ctx2).unwrap();
    assert_eq!(res.payload_name, "expa");
    assert!(matches!(res.provider, resolve::ProviderKind::Exposed));
}
