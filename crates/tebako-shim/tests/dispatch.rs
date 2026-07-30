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
        "  entrypoints:\n    - name: metanorma\n      path: /app/bin/metanorma\n      args_default: [\"--safe\"]\n      runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n",
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
        "  entrypoints:\n    - name: inkview\n      path: /app/bin/inkview\n",
        "8.1.0",
    );
    // The install-time materialization: <version>.tree/<in-image path>
    // (dispatch never materializes — install is the explicit verb).
    let entry_host = home
        .join("payloads")
        .join("inkview")
        .join("8.1.0.tree")
        .join("app/bin/inkview");
    std::fs::create_dir_all(entry_host.parent().unwrap()).unwrap();
    std::fs::write(&entry_host, b"#!/bin/sh\n").unwrap();
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "inkview", "8.1.0");

    let plan = dispatch::dispatch("inkview", &["file.svg".into()], &ctx).unwrap();

    assert!(matches!(plan.runtime, RuntimeResolution::Zero));
    assert_eq!(plan.program, entry_host);
    let expected: Vec<String> = vec![
        entry_host.to_string_lossy().into_owned(),
        "file.svg".into(),
    ];
    assert_eq!(plan.argv, expected);
    // the mounts grammar rides the child's env (the preload re-mounts)
    let mounts_env = plan
        .env
        .iter()
        .find(|(k, _)| k == "TEBAKO_TFS_MOUNTS")
        .map(|(_, v)| v.clone())
        .expect("TEBAKO_TFS_MOUNTS must be exported");
    assert_eq!(mounts_env, format!("{}:/", image.display()));
    assert_eq!(plan.mounts.len(), 1);
}

#[test]
fn zero_runtime_entrypoint_without_materialization_is_a_named_error() {
    let tmp = TempDir::new("zero-runtime-unmat");
    let home = tmp.path().join("home");
    seed_tool(
        &home,
        "inkview",
        "  entrypoints:\n    - name: inkview\n      path: /app/bin/inkview\n",
        "8.1.0",
    );
    // NO 8.1.0.tree/ — install's materialization never ran.
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "inkview", "8.1.0");

    let err = dispatch::dispatch("inkview", &["file.svg".into()], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE, "{err:?}");
    assert!(err.message.contains("not materialized"), "{}", err.message);
    assert!(err.message.contains("tebako install"), "{}", err.message);
}

#[test]
fn declared_dependency_mounts_join_the_mount_set() {
    let tmp = TempDir::new("dep-mounts");
    let home = tmp.path().join("home");
    let image = write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            "  entrypoints:\n    - name: metanorma\n      path: /app/bin/metanorma\n      runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n",
            "requires:\n  - kind: data\n    name: iso-codes\n    constraint: \">= 2024.1\"\n    mount: /__app__/share/iso-codes\n",
        ),
    );
    let dep_old = write_payload(
        &home,
        "iso-codes",
        "2024.1",
        &data_manifest("iso-codes", "2024.1"),
    );
    let dep_new = write_payload(
        &home,
        "iso-codes",
        "2025.2",
        &data_manifest("iso-codes", "2025.2"),
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
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            "  entrypoints:\n    - name: metanorma\n      path: /app/bin/metanorma\n",
            "requires:\n  - kind: toolkit\n    name: gtk-layer\n    constraint: \">= 3.24\"\n    mount: /__layers__/gtk\n",
        ),
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
    let err = dispatch::plan(&res, &[], &ctx, false, Vec::new()).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("would download"), "{}", err.message);
}

/// THE suite proof (spec 07 §2.0 multi-command suites): ONE installed
/// suite payload, TWO shim commands, EACH resolving its OWN runtime
/// requirement — two commands of one package run DIFFERENT runtime
/// versions simultaneously (both dispatch plans are composed and usable
/// at once; the runtime swap never touches the payload).
#[test]
fn suite_commands_run_different_runtime_versions_simultaneously() {
    let tmp = TempDir::new("suite-two-runtimes");
    let home = tmp.path().join("home");
    // one suite payload, two entrypoints with DIFFERENT runtime
    // requirements (the metanorma/mn2pdf shape, spec 03 §6)
    write_payload(
        &home,
        "metasuite",
        "2.0.0",
        &app_manifest(
            "metasuite",
            "2.0.0",
            "  entrypoints:\n    - name: metanorma\n      path: /app/bin/metanorma\n      runtime_requirement: {engine: ruby, constraint: \"~> 3.3.0\"}\n    - name: mn2pdf\n      path: /app/bin/mn2pdf\n      runtime_requirement: {engine: ruby, constraint: \"~> 3.4.0\"}\n",
        ),
    );
    // two cached runtimes, one per abi line
    let exe_33 = write_runtime(&home, "3.3.9", "0.16.0", false);
    let exe_34 = write_runtime(&home, "3.4.2", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "2.0.0");
    pin_env(&mut ctx, "mn2pdf", "2.0.0");

    // both dispatch plans coexist — the simultaneous case
    let plan_a = dispatch::dispatch("metanorma", &[], &ctx).unwrap();
    let plan_b = dispatch::dispatch("mn2pdf", &[], &ctx).unwrap();

    // each resolved its OWN abi line's newest cached runtime
    assert_eq!(plan_a.program, exe_33);
    assert_eq!(plan_b.program, exe_34);
    assert_ne!(plan_a.program, plan_b.program);
    // …and each hands off its own entrypoint of the SAME payload image
    let image = home.join("payloads/metasuite/2.0.0.tfs");
    assert_eq!(plan_a.mounts[0].image, image);
    assert_eq!(plan_b.mounts[0].image, image);
    assert!(plan_a.argv.iter().any(|a| a == "/app/bin/metanorma"));
    assert!(plan_b.argv.iter().any(|a| a == "/app/bin/mn2pdf"));
}

// ---------------------------------------------------------------------
// Jail flags (spec 08 §2/§4 — the dispatcher's tightening surface)
// ---------------------------------------------------------------------

fn env_get<'a>(plan: &'a dispatch::ExecPlan, key: &str) -> Option<&'a str> {
    plan.env
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// An app manifest mirror whose `capabilities.host` declares the jail
/// request (spec 08 §4) in the unified payload-manifest shape.
fn jailed_manifest(tool: &str, version: &str, host_yaml: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {tool}\n  version: \"{version}\"\n  producer: {{tool: tebako-shim-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: {}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {tool}\n      path: /app/bin/{tool}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true, host: {host_yaml}}}\n",
        "a".repeat(64),
        "b".repeat(64),
    )
}

fn seed_jailed_tool(
    home: &std::path::Path,
    tool: &str,
    host_yaml: &str,
    version: &str,
) -> std::path::PathBuf {
    write_payload(
        home,
        tool,
        version,
        &jailed_manifest(tool, version, host_yaml),
    )
}

#[test]
fn jail_flags_split_from_payload_args() {
    let args: Vec<String> = [
        "--no-host",
        "--jail=deny",
        "--mount",
        "/a:/a:ro",
        "in.csv",
        "--verbose",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let (flags, rest) = dispatch::parse_jail_flags(&args).unwrap();
    assert!(flags.no_host);
    assert_eq!(flags.jail.as_deref(), Some("deny"));
    assert_eq!(flags.mounts, vec!["/a:/a:ro".to_string()]);
    assert_eq!(rest, vec!["in.csv".to_string(), "--verbose".to_string()]);

    // `--` ends the scan; a payload arg literally named --jail survives.
    let args: Vec<String> = ["--no-host", "--", "--jail", "x"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (flags, rest) = dispatch::parse_jail_flags(&args).unwrap();
    assert!(flags.no_host);
    assert_eq!(rest, vec!["--jail".to_string(), "x".to_string()]);

    // Unknown flags are the payload's (a shim forwards argv).
    let args: Vec<String> = ["--verbose", "--no-host"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (flags, rest) = dispatch::parse_jail_flags(&args).unwrap();
    assert!(!flags.no_host);
    assert_eq!(rest.len(), 2);

    // A missing value is a named usage error.
    let args: Vec<String> = vec!["--jail".to_string()];
    let err = dispatch::parse_jail_flags(&args).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_USAGE);
}

#[test]
fn dispatch_no_jail_anywhere_exports_no_jail_env() {
    let tmp = TempDir::new("jail-none");
    let home = tmp.path().join("home");
    seed_tool(
        &home,
        "metanorma",
        &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
        "1.2.3",
    );
    write_runtime(&home, "4.0.6", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan = dispatch::dispatch("metanorma", &["doc.xml".into()], &ctx).unwrap();
    assert_eq!(env_get(&plan, "TEBAKO_JAIL"), None);
    assert_eq!(env_get(&plan, "TEBAKO_JAIL_SOURCE"), None);
    assert_eq!(env_get(&plan, "TEBAKO_JAIL_JOURNAL"), None);
}

#[test]
fn dispatch_manifest_request_alone_maps_to_tebako_jail() {
    let tmp = TempDir::new("jail-manifest");
    let home = tmp.path().join("home");
    seed_jailed_tool(
        &home,
        "metanorma",
        "{default: deny, argument_files: auto-allowed}",
        "1.2.3",
    );
    write_runtime(&home, "4.0.6", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    // An existing payload arg becomes the argument-file grant.
    let input = tmp.path().join("doc.xml");
    std::fs::write(&input, b"<doc/>").unwrap();
    let plan = dispatch::dispatch(
        "metanorma",
        &["compile".into(), input.to_string_lossy().into_owned()],
        &ctx,
    )
    .unwrap();
    assert_eq!(
        env_get(&plan, "TEBAKO_JAIL"),
        Some(format!("deny;@{}", input.display()).as_str())
    );
    assert_eq!(env_get(&plan, "TEBAKO_JAIL_SOURCE"), Some("manifest"));
    assert_eq!(
        env_get(&plan, "TEBAKO_JAIL_JOURNAL"),
        Some(home.join("journal.log").to_string_lossy().as_ref())
    );
    // The payload args pass through untouched.
    assert_eq!(
        plan.argv.last().unwrap(),
        &input.to_string_lossy().into_owned()
    );
}

#[test]
fn dispatch_user_tightening_intersects_never_loosens() {
    let tmp = TempDir::new("jail-precedence");
    let home = tmp.path().join("home");
    // The manifest requests /data ro under deny; --no-host caps it.
    seed_jailed_tool(
        &home,
        "metanorma",
        "{default: deny, mounts: [{host: /data, mount: /data, access: ro}]}",
        "1.2.3",
    );
    write_runtime(&home, "4.0.6", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan = dispatch::dispatch("metanorma", &["--no-host".into()], &ctx).unwrap();
    assert_eq!(env_get(&plan, "TEBAKO_JAIL"), Some("deny"));
    assert_eq!(env_get(&plan, "TEBAKO_JAIL_SOURCE"), Some("manifest+user"));
    // And --no-host survives as a dispatcher flag, never reaching argv.
    assert!(!plan.argv.iter().any(|a| a == "--no-host"));
}

#[test]
fn dispatch_user_flags_alone_apply_when_the_manifest_is_silent() {
    let tmp = TempDir::new("jail-user");
    let home = tmp.path().join("home");
    seed_tool(
        &home,
        "metanorma",
        &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
        "1.2.3",
    );
    write_runtime(&home, "4.0.6", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let work = tmp.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let spec = format!("--mount={}:/work:rw", work.display());
    let plan = dispatch::dispatch(
        "metanorma",
        &["--jail".into(), "deny".into(), spec, "compile".into()],
        &ctx,
    )
    .unwrap();
    assert_eq!(
        env_get(&plan, "TEBAKO_JAIL"),
        Some(format!("deny;{}:/work:rw", work.display()).as_str())
    );
    assert_eq!(env_get(&plan, "TEBAKO_JAIL_SOURCE"), Some("user"));
    assert_eq!(plan.argv.last().unwrap(), "compile");
}

#[test]
fn dispatch_malformed_jail_flag_is_a_usage_error() {
    let tmp = TempDir::new("jail-bad");
    let home = tmp.path().join("home");
    seed_tool(
        &home,
        "metanorma",
        &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
        "1.2.3",
    );
    write_runtime(&home, "4.0.6", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let err = dispatch::dispatch("metanorma", &["--jail".into(), "frob".into()], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_USAGE);
    assert!(err.message.contains("invalid jail spec"), "{}", err.message);
}
