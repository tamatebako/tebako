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
    // <runtime> --tebako-image <image>:0:/ --tebako-entry <path> <user args>
    // tebako#503: args_default does NOT ride the handoff — the runtime
    // side composes it (spec 29 §1 / spec 17 §1); the shim carrying it
    // doubled the composition on the wrapper path.
    let expected: Vec<String> = vec![
        exe.to_string_lossy().into_owned(),
        "--tebako-image".into(),
        format!("{}:0:/", image.display()),
        "--tebako-entry".into(),
        "/app/bin/metanorma".into(),
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
    let _image = seed_tool(
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
    let expected: Vec<String> = vec![entry_host.to_string_lossy().into_owned(), "file.svg".into()];
    assert_eq!(plan.argv, expected);
    // Zero-runtime: the child runs from the store tree (host paths) — no
    // mounts, no preload shim (the openjdk JVM boot-classpath failure,
    // dogfood-found 2026-08-12).
    assert!(
        !plan.env.iter().any(|(k, _)| k == "TEBAKO_TFS_MOUNTS"),
        "TEBAKO_TFS_MOUNTS must NOT be exported for zero-runtime"
    );
    assert_eq!(plan.mounts.len(), 1);
}

#[test]
fn zero_runtime_entrypoint_keeps_args_default_in_the_argv() {
    // tebako#503: a zero-runtime entry HAS no runtime side to compose
    // args_default — the shim owns them (the program's leading args).
    // For a runtime entry the shim appends nothing (the driver
    // composes, spec 29 §1).
    let tmp = TempDir::new("zero-runtime-defaults");
    let home = tmp.path().join("home");
    let _image = seed_tool(
        &home,
        "inkview",
        "  entrypoints:\n    - name: inkview\n      path: /app/bin/inkview\n      args_default: [\"--batch\"]\n",
        "8.1.0",
    );
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
    let expected: Vec<String> = vec![
        entry_host.to_string_lossy().into_owned(),
        "--batch".into(),
        "file.svg".into(),
    ];
    assert_eq!(plan.argv, expected);
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

// ---------------------------------------------------------------------
// spec 30: spawned-runtime edges — the dispatch-time lock (§4) and the
// expose-name dispatch (§3)
// ---------------------------------------------------------------------

const SPAWN_EDGE: &str =
    "requires:\n  - {kind: runtime, engine: java, constraint: \">= 21\", expose: [java, keytool]}\n";

#[test]
fn runtime_edge_is_never_co_mounted_and_exports_the_spawn_lock() {
    // spec 30 §1/§4: the edge rides the manifest, resolves at dispatch,
    // and pins the driver's spawn-time pick via TEBAKO_SPAWN_LOCK — the
    // mount set stays the payload alone.
    let tmp = TempDir::new("spawn-lock");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            SPAWN_EDGE,
        ),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap();

    // The edge is never a mount; the payload alone rides.
    assert_eq!(plan.mounts.len(), 1);
    assert_eq!(plan.mounts[0].mount, "/");
    // The lock pins engine=lang_version:tebako_version.
    assert_eq!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK"),
        Some("java=21.0.8:0.3.0")
    );
}

#[test]
fn exposed_name_dispatches_the_runtime_boot() {
    // spec 30 §3: invoking an EXPOSED name boots the depended runtime
    // directly — no payload mounts, the bare entry name resolves through
    // the runtime's own manifest child-side (spec 17 §1's bare-name rule).
    let tmp = TempDir::new("expose-dispatch");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            SPAWN_EDGE,
        ),
    );
    let java = write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    let mut ctx = ctx(&home, tmp.path());
    // The exposed name keys the version chain on the CONSUMER payload
    // (spec 07's argv0 model — TEBAKO_JAVA_VERSION pins metanorma here).
    pin_env(&mut ctx, "java", "1.2.3");

    let plan = dispatch::dispatch("java", &["-version".into()], &ctx).unwrap();

    assert_eq!(plan.program, java);
    let expected: Vec<String> = vec![
        java.to_string_lossy().into_owned(),
        "--tebako-entry".into(),
        "java".into(),
        "-version".into(),
    ];
    assert_eq!(plan.argv, expected);
    assert!(plan.mounts.is_empty(), "the consumer payload never mounts");
    let image = env_get(&plan, "TEBAKO_RUNTIME_IMAGE").expect("the env image rides");
    assert!(image.ends_with(".tfs"), "the env image: {image}");
    assert!(matches!(plan.runtime, RuntimeResolution::Ready(_)));
    assert!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK").is_none(),
        "the expose boot carries no lock — the child IS the runtime"
    );
}

#[test]
fn expose_ambiguity_is_a_named_error() {
    // spec 30 §3: two installed payloads exposing the same name is the
    // suite-ambiguity class — named, never a coin flip.
    let tmp = TempDir::new("expose-ambig");
    let home = tmp.path().join("home");
    for (name, version) in [("metanorma", "1.2.3"), ("mn2pdf", "2.0")] {
        write_payload(
            &home,
            name,
            version,
            &app_manifest_requires(
                name,
                version,
                &entrypoint_yaml(RUBY_ENTRY, name),
                SPAWN_EDGE,
            ),
        );
    }
    let ctx = ctx(&home, tmp.path());
    let err = dispatch::dispatch("java", &[], &ctx).unwrap_err();
    assert!(
        err.message.contains("exposed by more than one"),
        "{}",
        err.message
    );
}

#[test]
fn spawn_edge_implementation_mismatch_is_a_named_error() {
    // spec 28 §8 × spec 30 §1: the edge's implementation axis filters the
    // cache; a version-matching runtime of the WRONG implementation is
    // ineligible — and the error says so (never "version not satisfied").
    let tmp = TempDir::new("spawn-impl");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            "requires:\n  - {kind: runtime, engine: java, implementation: corretto, constraint: \">= 21\", expose: [java]}\n",
        ),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine_meta(&home, "java", "21.0.8", "0.3.0", None, Some("temurin"));
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let res = tebako_shim::resolve::resolve("metanorma", &ctx).unwrap();
    // allow_download=false (the `which` surface): no network in tests.
    let err = dispatch::plan(&res, &[], &ctx, false, Vec::new()).unwrap_err();
    assert!(
        err.message.contains("implementation mismatch"),
        "{}",
        err.message
    );
}

#[test]
fn spawn_edge_requires_the_env_image_pair() {
    // spec 30 §4: a cached runtime WITHOUT the verified image pair can
    // never serve a spawn — named error, never a silent skip.
    let tmp = TempDir::new("spawn-noimg");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            SPAWN_EDGE,
        ),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", false);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let res = tebako_shim::resolve::resolve("metanorma", &ctx).unwrap();
    let err = dispatch::plan(&res, &[], &ctx, false, Vec::new()).unwrap_err();
    assert!(
        err.message.contains("no verified env image"),
        "{}",
        err.message
    );
}

// ---------------------------------------------------------------------
// spec 32: spawned-payload edges — the co-mount axis, the dispatch-time
// payload lock rows (§5), and the expose-name provider dispatch (§2/§3)
// ---------------------------------------------------------------------

/// A python-needing app entrypoint block (the xml2rfc provider's shape).
const PYTHON_ENTRY: &str = "  entrypoints:\n    - name: TOOL\n      path: /app/bin/TOOL\n      runtime_requirement: {engine: python, constraint: \">= 3.10\"}\n";

/// The consumer's executable edge, provider-pinned (metanorma's xml2rfc).
const EXEC_EDGE: &str =
    "requires:\n  - {kind: executable, name: xml2rfc, payload: xml2rfc, constraint: \">= 3.34\", expose: [xml2rfc], critical: true}\n";

/// The unpinned form (capability resolution answers the provider).
const EXEC_EDGE_UNPINNED: &str =
    "requires:\n  - {kind: executable, name: xml2rfc, constraint: \">= 3.34\", expose: [xml2rfc]}\n";

fn seed_xml2rfc_provider(home: &std::path::Path, name: &str, requires: &str) -> std::path::PathBuf {
    write_payload(
        home,
        name,
        "3.34.0",
        &app_manifest_requires(
            name,
            "3.34.0",
            &entrypoint_yaml(PYTHON_ENTRY, "xml2rfc"),
            requires,
        ),
    )
}

#[test]
fn executable_edge_co_mounts_on_the_mount_axis() {
    // spec 32 §1: mount and expose are ORTHOGONAL — a mount-only
    // executable edge co-mounts the provider image at the
    // consumer-declared path and exports NO spawn-lock row.
    let tmp = TempDir::new("exec-mount");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            "requires:\n  - {kind: executable, name: dot, payload: graphviz, constraint: \">= 2.40\", mount: /opt/graphviz}\n",
        ),
    );
    let provider = write_payload(
        &home,
        "graphviz",
        "2.40.1",
        &app_manifest("graphviz", "2.40.1", &entrypoint_yaml(NATIVE_ENTRY, "dot")),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap();

    assert_eq!(plan.mounts.len(), 2);
    assert_eq!(plan.mounts[1].image, provider);
    assert_eq!(plan.mounts[1].mount, "/opt/graphviz");
    assert!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK").is_none(),
        "a mount-only edge opens no spawn surface"
    );
}

#[test]
fn executable_edge_exports_the_payload_lock_row() {
    // spec 32 §5: the expose-carrying executable edge pins the provider
    // payload AND the provider's resolved runtime pair as the
    // `<payload>@<version>=<engine>=<lv>:<tebako>` row.
    let tmp = TempDir::new("exec-lock");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE,
        ),
    );
    seed_xml2rfc_provider(&home, "xml2rfc", "");
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine(&home, "python", "3.13.15", "2.1.10", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap();

    // The edge is never a mount on its expose axis; the payload alone rides.
    assert_eq!(plan.mounts.len(), 1);
    assert_eq!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK"),
        Some("xml2rfc@3.34.0=python=3.13.15:2.1.10")
    );
}

#[test]
fn executable_edge_lock_composes_transitively() {
    // spec 32 §2: the provider's OWN spawn edges join the parent's lock
    // (the spawned child has no loader — the transitive pins compose at
    // dispatch).
    let tmp = TempDir::new("exec-lock-deep");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE,
        ),
    );
    // The provider itself spawns a java runtime.
    seed_xml2rfc_provider(&home, "xml2rfc", SPAWN_EDGE);
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine(&home, "python", "3.13.15", "2.1.10", true);
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let plan = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap();

    assert_eq!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK"),
        Some("xml2rfc@3.34.0=python=3.13.15:2.1.10;java=21.0.8:0.3.0")
    );
}

#[test]
fn exposed_executable_dispatches_the_provider_boot() {
    // spec 32 §2/§3: invoking an EXPOSED name composes the provider
    // payload's own managed dispatch as the child — the provider image
    // co-mounts at /, the entry resolves against the provider's
    // entrypoints, and the version chain keys on the CONSUMER (spec 07's
    // argv0 model). The consumer payload is never mounted in the child.
    //
    // White-box note: once the provider is INSTALLED, spec 07 §2.0's
    // own-claim precedence (a payload declaring the entrypoint beats an
    // exposer) routes the shim to the provider's own record — the
    // composition below is identical in shape either way (spec 32 §2).
    // The Exposed arm answers when no installed payload declares the
    // entrypoint; plan() is exercised directly with the edge resolve()
    // would have captured.
    let tmp = TempDir::new("exec-expose");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE_UNPINNED,
        ),
    );
    let provider_image = seed_xml2rfc_provider(&home, "xml2rfc-py", "");
    let python = write_runtime_engine(&home, "python", "3.13.15", "2.1.10", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let mut res = tebako_shim::resolve::resolve("metanorma", &ctx).unwrap();
    res.exposed = res
        .manifest
        .requires()
        .iter()
        .find(|r| matches!(r, tpkg::Requirement::Executable { .. }))
        .cloned();
    res.tool = "xml2rfc".to_string();

    let plan = dispatch::plan(&res, &["--help".into()], &ctx, false, Vec::new()).unwrap();

    assert_eq!(plan.program, python);
    let expected: Vec<String> = vec![
        python.to_string_lossy().into_owned(),
        "--tebako-image".into(),
        format!("{}:0:/", provider_image.display()),
        "--tebako-entry".into(),
        "xml2rfc".into(),
        "--help".into(),
    ];
    assert_eq!(plan.argv, expected);
    assert_eq!(plan.mounts.len(), 1);
    assert_eq!(plan.mounts[0].image, provider_image);
    let image = env_get(&plan, "TEBAKO_RUNTIME_IMAGE").expect("the env image rides");
    assert!(image.ends_with(".tfs"), "the env image: {image}");
    assert!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK").is_none(),
        "a provider without spawn edges exports no child lock"
    );
}

#[test]
fn exposed_executable_child_lock_carries_the_providers_own_edges() {
    // spec 32 §2 (locked): the child env SETS a fresh TEBAKO_SPAWN_LOCK
    // carrying the provider's own resolved pins — the parent's lock is
    // never inherited. (White-box on the Exposed arm — see the provider
    // boot test's note.)
    let tmp = TempDir::new("exec-expose-lock");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE_UNPINNED,
        ),
    );
    seed_xml2rfc_provider(&home, "xml2rfc-py", SPAWN_EDGE);
    write_runtime_engine(&home, "python", "3.13.15", "2.1.10", true);
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let mut res = tebako_shim::resolve::resolve("metanorma", &ctx).unwrap();
    res.exposed = res
        .manifest
        .requires()
        .iter()
        .find(|r| matches!(r, tpkg::Requirement::Executable { .. }))
        .cloned();
    res.tool = "xml2rfc".to_string();

    let plan = dispatch::plan(&res, &["--help".into()], &ctx, false, Vec::new()).unwrap();

    assert_eq!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK"),
        Some("java=21.0.8:0.3.0"),
        "the child's fresh lock carries the provider's own pins"
    );
}

#[test]
fn executable_edge_missing_provider_is_dependency_not_found() {
    // spec 32 §5/§7: dispatch is cache-only for provider payloads — a
    // provider nobody installed is the named DependencyNotFound pointing
    // at the install verb, never a download and never a host fallback.
    let tmp = TempDir::new("exec-missing");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE_UNPINNED,
        ),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let err = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap_err();
    assert!(err.message.contains("DependencyNotFound"), "{}", err.message);
    assert!(err.message.contains("tebako install"), "{}", err.message);
}

#[test]
fn executable_edge_ambiguous_provider_is_a_named_error() {
    // spec 03 §8 × spec 32 §1: two installed payloads providing the
    // capability is AmbiguousProvider — pin with `payload:`.
    let tmp = TempDir::new("exec-ambig");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE_UNPINNED,
        ),
    );
    seed_xml2rfc_provider(&home, "xml2rfc-py", "");
    seed_xml2rfc_provider(&home, "xml2rfc-alt", "");
    write_runtime(&home, "4.0.6", "0.16.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let err = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap_err();
    assert!(err.message.contains("AmbiguousProvider"), "{}", err.message);
    assert!(err.message.contains("payload:"), "{}", err.message);
}

#[test]
fn executable_edge_runtime_less_match_is_a_named_error() {
    // spec 32 §0/§1: an exposed name matching a runtime-less entry (a
    // native entrypoint) is a named resolution error — never an
    // exec-tier fallback.
    let tmp = TempDir::new("exec-native");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE,
        ),
    );
    // The provider's entrypoint carries NO runtime_requirement.
    write_payload(
        &home,
        "xml2rfc",
        "3.34.0",
        &app_manifest("xml2rfc", "3.34.0", &entrypoint_yaml(NATIVE_ENTRY, "xml2rfc")),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let err = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap_err();
    assert!(
        err.message.contains("no runtime_requirement"),
        "{}",
        err.message
    );
}

#[test]
fn executable_edge_cycle_is_a_named_error() {
    // spec 32 §2: a cycle through spawn edges is the resolver's named
    // cycle error, never a recursion trap.
    let tmp = TempDir::new("exec-cycle");
    let home = tmp.path().join("home");
    // metanorma exposes xml2rfc (unpinned) …
    write_payload(
        &home,
        "metanorma",
        "1.2.3",
        &app_manifest_requires(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
            EXEC_EDGE_UNPINNED,
        ),
    );
    // … and the provider exposes metanorma back.
    write_payload(
        &home,
        "xml2rfc-py",
        "3.34.0",
        &app_manifest_requires(
            "xml2rfc-py",
            "3.34.0",
            &entrypoint_yaml(PYTHON_ENTRY, "xml2rfc"),
            "requires:\n  - {kind: executable, name: metanorma, constraint: \">= 1\", expose: [metanorma]}\n",
        ),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine(&home, "python", "3.13.15", "2.1.10", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.2.3");

    let err = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap_err();
    assert!(err.message.contains("cycle"), "{}", err.message);
}

#[test]
fn user_tightening_exports_the_hereditary_ceiling() {
    // spec 32 §4 (locked): operator tightening is HEREDITARY — the
    // parent's user directives ride TEBAKO_JAIL_TIGHTENING so every
    // spawned child re-applies them as the ceiling.
    let tmp = TempDir::new("jail-ceiling");
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

    let plan = dispatch::dispatch("metanorma", &["--no-host".into()], &ctx).unwrap();
    assert_eq!(
        env_get(&plan, "TEBAKO_JAIL_TIGHTENING"),
        Some("deny"),
        "the tightening rides for spawned children"
    );
    // No flags: no ceiling channel at all.
    let plan = dispatch::dispatch("metanorma", &["compile".into()], &ctx).unwrap();
    assert_eq!(env_get(&plan, "TEBAKO_JAIL_TIGHTENING"), None);
}
