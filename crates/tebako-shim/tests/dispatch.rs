//! Hand-off composition (spec 07 §2.3 + spec 06 ABI v1 argv shape) and
//! zero-runtime dispatch (spec 03 §2.2).

mod common;

use common::*;
use tebako_shim::dispatch;
use tebako_shim::runtime::RuntimeResolution;

fn seed_tool(
    home: &std::path::Path,
    tool: &str,
    entry_yaml: &str,
    version: &str,
) -> std::path::PathBuf {
    write_payload(
        home,
        tool,
        version,
        &app_manifest(tool, version, entry_yaml),
    )
}

fn pin_env(ctx: &mut tebako_shim::Ctx, tool: &str, version: &str) {
    let var = tebako_shim::resolve::version_env_var(tool);
    ctx.env.insert(var, version.to_string());
}

#[test]
fn runtime_entrypoint_composes_the_abi_v1_handoff() {
    let tmp = TempDir::new("handoff");
    let home = tmp.path().join("home");
    let image = seed_tool(
        &home,
        "metanorma",
        "entrypoints:\n  - name: metanorma\n    path: /app/bin/metanorma\n    args_default: [\"--safe\"]\n    runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n",
        "1.2.3",
    );
    let exe = write_runtime(&home, "4.0.6", "0.16.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan =
        dispatch::dispatch("metanorma", &["compile".into(), "doc.xml".into()], &ctx).unwrap();

    assert_eq!(plan.program, exe);
    assert!(matches!(plan.runtime, RuntimeResolution::Ready(_)));
    // <runtime> --tebako-image <image>:0:/ --tebako-entry <path> <defaults> <user args>
    let expected: Vec<String> = vec![
        exe.to_string_lossy().into_owned(),
        "--tebako-image".into(),
        format!("{}:0:/", image.display()),
        "--tebako-entry".into(),
        "/app/bin/metanorma".into(),
        "--safe".into(),
        "compile".into(),
        "doc.xml".into(),
    ];
    assert_eq!(plan.argv, expected);
    // image-era runtime → TEBAKO_RUNTIME_IMAGE (spec 06 §2)
    let env_image = plan
        .env
        .iter()
        .find(|(k, _)| k == "TEBAKO_RUNTIME_IMAGE")
        .map(|(_, v)| v.clone());
    assert!(env_image.is_some(), "TEBAKO_RUNTIME_IMAGE must be exported");
    assert!(env_image.unwrap().ends_with(".tfs"));
    // mount set: payload only
    assert_eq!(plan.mounts.len(), 1);
    assert_eq!(plan.mounts[0].mount, "/");
}

#[test]
fn zero_runtime_entrypoint_skips_runtime_resolution() {
    let tmp = TempDir::new("zero-runtime");
    let home = tmp.path().join("home");
    // NO runtimes dir, NO mirror, NO config: a native entrypoint must not
    // touch runtime resolution at all.
    let image = seed_tool(
        &home,
        "inkview",
        "entrypoints:\n  - name: inkview\n    path: /app/bin/inkview\n",
        "8.1.0",
    );
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "inkview", "8.1.0");

    let plan = dispatch::dispatch("inkview", &["file.svg".into()], &ctx).unwrap();

    assert!(matches!(plan.runtime, RuntimeResolution::Zero));
    assert_eq!(plan.program, image);
    let expected: Vec<String> = vec![
        image.to_string_lossy().into_owned(),
        "--tebako-entry".into(),
        "/app/bin/inkview".into(),
        "file.svg".into(),
    ];
    assert_eq!(plan.argv, expected);
    assert!(plan.env.is_empty());
    assert_eq!(plan.mounts.len(), 1);
}

#[test]
fn declared_dependency_mounts_join_the_mount_set() {
    let tmp = TempDir::new("dep-mounts");
    let home = tmp.path().join("home");
    let image = seed_tool(
        &home,
        "metanorma",
        "entrypoints:\n  - name: metanorma\n    path: /app/bin/metanorma\n    runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\nrequires:\n  - kind: data\n    name: iso-codes\n    constraint: \">= 2024.1\"\n    mount: /__app__/share/iso-codes\n",
        "1.2.3",
    );
    let dep_old = write_payload(
        &home,
        "iso-codes",
        "2024.1",
        &app_manifest("iso-codes", "2024.1", ""),
    );
    let dep_new = write_payload(
        &home,
        "iso-codes",
        "2025.2",
        &app_manifest("iso-codes", "2025.2", ""),
    );
    let _ = dep_old;
    write_runtime(&home, "4.0.6", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan = dispatch::dispatch("metanorma", &[], &ctx).unwrap();
    assert_eq!(plan.mounts.len(), 2);
    assert_eq!(plan.mounts[0].image, image);
    assert_eq!(plan.mounts[1].image, dep_new, "newest satisfying dep");
    assert_eq!(plan.mounts[1].mount, "/__app__/share/iso-codes");
    // both mounts ride the argv shape
    let images: Vec<&String> = plan
        .argv
        .iter()
        .filter(|a| a.as_str() == "--tebako-image")
        .collect();
    assert_eq!(images.len(), 2);
}

#[test]
fn missing_dependency_is_a_named_error() {
    let tmp = TempDir::new("dep-missing");
    let home = tmp.path().join("home");
    seed_tool(
        &home,
        "metanorma",
        "entrypoints:\n  - name: metanorma\n    path: /app/bin/metanorma\nrequires:\n  - kind: toolkit\n    name: gtk-layer\n    constraint: \">= 3.24\"\n    mount: /__layers__/gtk\n",
        "1.2.3",
    );
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");
    let err = dispatch::dispatch("metanorma", &[], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("gtk-layer"), "{}", err.message);
}

#[test]
fn which_mode_never_downloads() {
    let tmp = TempDir::new("which-no-download");
    let home = tmp.path().join("home");
    seed_tool(
        &home,
        "metanorma",
        &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
        "1.2.3",
    );
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");
    let res = tebako_shim::resolve::resolve("metanorma", &ctx).unwrap();
    let err = dispatch::plan(&res, &[], &ctx, false).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("would download"), "{}", err.message);
}
