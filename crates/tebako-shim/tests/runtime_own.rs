//! The runtime-own command surface (spec 07 §10) — the argv0 dispatch of
//! a recorded claim: the spec-17 wire exactly (wrapper exe +
//! TEBAKO_RUNTIME_IMAGE + `--tebako-entry <token>` + verbatim user args),
//! the payload-wins collision rule, the disabled consultation, and the
//! per-engine version chain.

mod common;

use common::*;
use tebako_shim::dispatch::{self, ExecPlan};
use tebako_shim::runtime::RuntimeResolution;
use tebako_shim::{config, runtime_own};

fn env_get<'a>(plan: &'a ExecPlan, key: &str) -> Option<&'a str> {
    plan.env
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn pin_env(ctx: &mut tebako_shim::Ctx, tool: &str, version: &str) {
    ctx.env.insert(
        tebako_shim::resolve::version_env_var(tool),
        version.to_string(),
    );
}

#[test]
fn a_runtime_own_claim_dispatches_the_engine_boot() {
    // spec 07 §10: the bare form's wire token is `self` (the spec-17
    // reserved interpreter keyword); user args ride verbatim.
    let tmp = TempDir::new("ro-dispatch");
    let home = tmp.path().join("home");
    let java = write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let ctx = ctx(&home, tmp.path());

    let plan = dispatch::dispatch("java", &["-version".into(), "--verbose".into()], &ctx).unwrap();

    assert_eq!(plan.program, java);
    let expected: Vec<String> = vec![
        java.to_string_lossy().into_owned(),
        "--tebako-entry".into(),
        "self".into(),
        "-version".into(),
        "--verbose".into(),
    ];
    assert_eq!(plan.argv, expected);
    assert!(plan.mounts.is_empty(), "no payload ever mounts");
    let image = env_get(&plan, "TEBAKO_RUNTIME_IMAGE").expect("the env image rides");
    assert!(image.ends_with(".tfs"), "the env image: {image}");
    assert!(matches!(plan.runtime, RuntimeResolution::Ready(_)));
    assert!(
        env_get(&plan, "TEBAKO_SPAWN_LOCK").is_none(),
        "no spawn lock computation in this dispatch"
    );
}

#[test]
fn the_command_form_records_the_command_token() {
    let tmp = TempDir::new("ro-command");
    let home = tmp.path().join("home");
    let java = write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    runtime_own::record(&home, "keytool", "java", "keytool").unwrap();
    let ctx = ctx(&home, tmp.path());

    let plan = dispatch::dispatch("keytool", &["-list".into()], &ctx).unwrap();

    let expected: Vec<String> = vec![
        java.to_string_lossy().into_owned(),
        "--tebako-entry".into(),
        "keytool".into(),
        "-list".into(),
    ];
    assert_eq!(plan.argv, expected);
}

#[test]
fn a_payload_claim_wins_over_the_runtime_own_claim() {
    // The collision rule (MECE): the payload declared the tool; the
    // runtime surface is the fallback when NO payload provides it.
    let tmp = TempDir::new("ro-payload-wins");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "javatool",
        "1.0",
        &app_manifest("javatool", "1.0", &entrypoint_yaml(RUBY_ENTRY, "java")),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let mut ctx = ctx(&home, tmp.path());
    // The payload path's chain: the pin names the PAYLOAD's version (the
    // engine chain reads this var only when the runtime-own claim drives).
    pin_env(&mut ctx, "java", "1.0");

    let plan = dispatch::dispatch("java", &[], &ctx).unwrap();

    assert!(
        !plan.mounts.is_empty(),
        "the payload's dispatch mounts its image — the payload won"
    );
    let argv0 = plan.argv[0].clone();
    assert!(
        argv0.contains("ruby"),
        "the payload's runtime (ruby) boots, never the java engine: {argv0}"
    );
}

#[test]
fn a_sole_disabled_payload_claim_keeps_its_named_refusal() {
    // The collision rule holds for a DISABLED payload claim too — the
    // payload path's named refusal answers, never a silent runtime-own
    // shadow (dispatch refuses loudly; the user re-enables or disables
    // the payload claim's whole class explicitly).
    let tmp = TempDir::new("ro-disabled-payload");
    let home = tmp.path().join("home");
    write_payload(
        &home,
        "javatool",
        "1.0",
        &app_manifest("javatool", "1.0", &entrypoint_yaml(RUBY_ENTRY, "java")),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let mut disabled = config::Disabled::new();
    disabled.insert("java".to_string(), vec!["all".to_string()]);
    config::save_disabled(&home, &disabled).unwrap();
    let ctx = ctx(&home, tmp.path());

    let err = dispatch::dispatch("java", &[], &ctx).unwrap_err();
    assert!(
        err.message.contains("disabled"),
        "the payload path's disabled refusal answers: {err:?}"
    );
}

#[test]
fn a_disabled_runtime_own_claim_is_the_named_refusal() {
    // `all` and `<engine>@all` gate the claim (the engine rides the
    // selector's payload slot).
    let tmp = TempDir::new("ro-disabled");
    let home = tmp.path().join("home");
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let mut disabled = config::Disabled::new();
    disabled.insert("java".to_string(), vec!["java@all".to_string()]);
    config::save_disabled(&home, &disabled).unwrap();
    let ctx = ctx(&home, tmp.path());

    let err = dispatch::dispatch("java", &[], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE, "{err:?}");
    assert!(err.message.contains("runtime-own"), "{err:?}");
    assert!(err.message.contains("engine java"), "{err:?}");
    assert!(
        err.message.contains("tebako-shim enable java --of java"),
        "{err:?}"
    );

    // A version-only selector does NOT gate the class (the runtime-own
    // claim has no version axis — version gating rides the engine chain).
    let mut disabled = config::Disabled::new();
    disabled.insert("java".to_string(), vec!["21.0.8".to_string()]);
    config::save_disabled(&home, &disabled).unwrap();
    dispatch::dispatch("java", &[], &ctx).unwrap();
}

#[test]
fn the_version_chain_pins_the_engine() {
    // TEBAKO_<ENGINE>_VERSION is the chain's first link (spec 07 §10).
    let tmp = TempDir::new("ro-chain");
    let home = tmp.path().join("home");
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    write_runtime_engine(&home, "java", "21.0.12", "0.3.1", true);
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let mut ctx = ctx(&home, tmp.path());

    // Unpinned: the newest compatible cached runtime answers.
    let plan = dispatch::dispatch("java", &[], &ctx).unwrap();
    let RuntimeResolution::Ready(rt) = &plan.runtime else {
        panic!("a runtime resolves")
    };
    assert_eq!(rt.lang_version, "21.0.12");

    // The env pin wins.
    pin_env(&mut ctx, "java", "21.0.8");
    let plan = dispatch::dispatch("java", &[], &ctx).unwrap();
    let RuntimeResolution::Ready(rt) = &plan.runtime else {
        panic!("a runtime resolves")
    };
    assert_eq!(rt.lang_version, "21.0.8");

    // A payload-qualified pin is meaningless on the engine axis — the
    // named grammar error, naming the link and value.
    pin_env(&mut ctx, "java", "javatool@21.0.8");
    let err = dispatch::dispatch("java", &[], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST, "{err:?}");
    assert!(err.message.contains("payload@version"), "{err:?}");
}

#[test]
fn an_unclaimed_name_never_enters_the_runtime_own_path() {
    let tmp = TempDir::new("ro-unclaimed");
    let home = tmp.path().join("home");
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    let ctx = ctx(&home, tmp.path());
    let err = dispatch::dispatch("java", &[], &ctx).unwrap_err();
    assert!(
        err.message
            .contains("no installed payload provides or exposes"),
        "the payload path's no-provider error answers: {err:?}"
    );
}

#[test]
fn offline_with_nothing_cached_is_the_named_cache_or_error() {
    // spec 07 §10 × spec 05: TEBAKO_OFFLINE=1 + no compatible cached
    // runtime → the cache-or-error shape, naming the engine.
    let tmp = TempDir::new("ro-offline");
    let home = tmp.path().join("home");
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert("TEBAKO_OFFLINE".into(), "1".into());

    let err = dispatch::dispatch("java", &[], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE, "{err:?}");
    assert!(err.message.contains("java"), "{err:?}");
}

#[test]
fn an_unknown_engine_is_the_named_no_channel_error() {
    // spec 07 §10's errors: no registry carries the engine and the store
    // holds no runtime declaring it → EX_TEBAKO_UNAVAILABLE (69), naming
    // the engine and the channels consulted.
    let tmp = TempDir::new("ro-unknown");
    let home = tmp.path().join("home");
    write_config(&home, "registries: []\n");
    runtime_own::record(&home, "cobol", "cobol", "self").unwrap();
    let ctx = ctx(&home, tmp.path());

    let err = dispatch::dispatch("cobol", &[], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE, "{err:?}");
    assert!(err.message.contains("cobol"), "{err:?}");
    assert!(err.message.contains("kind: runtime"), "{err:?}");
}

#[test]
fn link_shims_displaces_a_runtime_own_claim_loudly() {
    // The registration chokepoint (spec 07 §10): a later payload claim on
    // the name displaces the runtime-own claim with a note — never a
    // silent shadow.
    let tmp = TempDir::new("ro-displace");
    let home = tmp.path().join("home");
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let binary = tmp.path().join("tebako-shim");
    std::fs::write(&binary, b"#!/bin/sh\n").unwrap();

    let (links, notes) =
        tebako_shim::manage::link_shims(&home, &binary, &["java".to_string()]).unwrap();

    assert_eq!(links.len(), 1);
    assert!(
        runtime_own::claim_for(&home, "java").unwrap().is_none(),
        "the claim is displaced"
    );
    assert!(
        notes
            .iter()
            .any(|n| n.contains("displaces the runtime-own claim") && n.contains("engine java")),
        "the displacement is loud: {notes:?}"
    );
}

#[test]
fn list_renders_the_runtime_own_rows() {
    let tmp = TempDir::new("ro-list");
    let home = tmp.path().join("home");
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    runtime_own::record(&home, "keytool", "java", "keytool").unwrap();
    let ctx = ctx(&home, tmp.path());

    let action = tebako_shim::run(&["tebako-shim".to_string(), "list".to_string()], &ctx).unwrap();
    let tebako_shim::Action::Print { text, code } = action else {
        panic!("list prints")
    };
    assert_eq!(code, 0);
    assert!(text.contains("runtime-own commands:"), "{text}");
    assert!(text.contains("command keytool (engine java)"), "{text}");
    assert!(text.contains("resolved: 21.0.8 (tebako 0.3.0)"), "{text}");

    // The JSON shape: provider_kind runtime-own + the engine class.
    let action = tebako_shim::run(
        &[
            "tebako-shim".to_string(),
            "list".to_string(),
            "--json".to_string(),
        ],
        &ctx,
    )
    .unwrap();
    let tebako_shim::Action::Print { text, .. } = action else {
        panic!("list prints")
    };
    let doc = tebako_json::parse(&text).unwrap();
    let commands = doc.find("commands").expect("commands key");
    let tebako_json::Value::Array(rows) = commands else {
        panic!("commands is an array")
    };
    let row = rows
        .iter()
        .find(|r| r.find("name").and_then(|v| v.as_string()) == Some("keytool".to_string()))
        .expect("the keytool row");
    assert_eq!(
        row.find("provider_kind")
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("runtime-own")
    );
    assert_eq!(
        row.find("engine").and_then(|v| v.as_string()).as_deref(),
        Some("java")
    );
    assert_eq!(
        row.find("version").and_then(|v| v.as_string()).as_deref(),
        Some("21.0.8")
    );
}

#[test]
fn doctor_reports_a_runtime_own_shim_by_its_engine() {
    let tmp = TempDir::new("ro-doctor");
    let home = tmp.path().join("home");
    write_runtime_engine(&home, "java", "21.0.8", "0.3.0", true);
    runtime_own::record(&home, "java", "java", "self").unwrap();
    // The link itself (doctor scans the shim dir).
    let binary = tmp.path().join("tebako-shim");
    std::fs::write(&binary, b"#!/bin/sh\n").unwrap();
    // NOTE: link_shims displaces the claim by design — re-record after.
    tebako_shim::manage::link_shims(&home, &binary, &["java".to_string()]).unwrap();
    runtime_own::record(&home, "java", "java", "self").unwrap();
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "PATH".into(),
        format!(
            "{}:{}",
            home.join("shims").display(),
            std::env::var("PATH").unwrap_or_default()
        ),
    );

    let action =
        tebako_shim::run(&["tebako-shim".to_string(), "doctor".to_string()], &ctx).unwrap();
    let tebako_shim::Action::Print { text, .. } = action else {
        panic!("doctor prints")
    };
    assert!(
        text.contains("shim java: ok (runtime-own, engine java"),
        "{text}"
    );
}
