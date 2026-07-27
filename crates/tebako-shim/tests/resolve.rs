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
            "defaults: {{metanorma: 1.2.3}}\nregistries:\n  - file://{}\n",
            reg.display()
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
        &format!("registries:\n  - file://{}\n", reg.display()),
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
