//! Multi-command suite dispatch (spec 07 §2.0): one installed suite → N
//! shims, each dispatching to its own image slot AND its own runtime
//! requirement — two commands of one package run different runtime
//! versions simultaneously.

mod common;

use common::*;
use tebako_shim::dispatch;
use tebako_shim::runtime::RuntimeResolution;

const SUITE_MANIFEST: &str = "schema_version: 1\nkind: app\nname: hellosuite\nversion: 1.0.0\nentrypoints:\n  - name: hello34\n    path: hello34\n    slot: 0\n    runtime_requirement: {engine: ruby, constraint: \"3.4.2\"}\n  - name: hello33\n    path: hello33\n    slot: 1\n    runtime_requirement: {engine: ruby, constraint: \"3.3.7\"}\n";

fn seed_suite(home: &std::path::Path) -> std::path::PathBuf {
    write_payload(home, "hellosuite", "1.0.0", SUITE_MANIFEST)
}

fn pin_both(ctx: &mut tebako_shim::Ctx) {
    ctx.env
        .insert("TEBAKO_HELLO34_VERSION".into(), "1.0.0".into());
    ctx.env
        .insert("TEBAKO_HELLO33_VERSION".into(), "1.0.0".into());
}

#[test]
fn suite_entries_dispatch_to_their_own_slot_and_runtime() {
    let tmp = TempDir::new("suite-dispatch");
    let home = tmp.path().join("home");
    let image = seed_suite(&home);
    let exe34 = write_runtime(&home, "3.4.2", "0.15.9", true);
    let exe33 = write_runtime(&home, "3.3.7", "0.15.9", true);
    let mut ctx = ctx(&home, tmp.path());
    pin_both(&mut ctx);

    // Both commands plan against THEIR OWN cached runtimes — differing
    // versions, simultaneously usable (spec 07 §2.0's case).
    let plan34 = dispatch::dispatch("hello34", &["world".into()], &ctx).unwrap();
    let plan33 = dispatch::dispatch("hello33", &["--pdf".into()], &ctx).unwrap();

    let RuntimeResolution::Ready(rt34) = &plan34.runtime else {
        panic!("hello34 must resolve a runtime");
    };
    let RuntimeResolution::Ready(rt33) = &plan33.runtime else {
        panic!("hello33 must resolve a runtime");
    };
    assert_eq!(rt34.lang_version, "3.4.2");
    assert_eq!(rt33.lang_version, "3.3.7");
    assert_ne!(rt34.exe, rt33.exe, "each command runs its own runtime");
    assert_eq!(plan34.program, exe34);
    assert_eq!(plan33.program, exe33);

    // Each entry mounts ITS OWN slot of the one installed package.
    assert_eq!(
        plan34.mounts[0].triple(),
        format!("{}:0:/", image.display())
    );
    assert_eq!(
        plan33.mounts[0].triple(),
        format!("{}:1:/", image.display())
    );

    // And the ABI argv carries the entry's own --tebako-entry.
    let expected34: Vec<String> = vec![
        exe34.to_string_lossy().into_owned(),
        "--tebako-image".into(),
        format!("{}:0:/", image.display()),
        "--tebako-entry".into(),
        "hello34".into(),
        "world".into(),
    ];
    assert_eq!(plan34.argv, expected34);
    let expected33: Vec<String> = vec![
        exe33.to_string_lossy().into_owned(),
        "--tebako-image".into(),
        format!("{}:1:/", image.display()),
        "--tebako-entry".into(),
        "hello33".into(),
        "--pdf".into(),
    ];
    assert_eq!(plan33.argv, expected33);
}

#[test]
fn unknown_command_against_an_installed_suite_is_a_named_error() {
    let tmp = TempDir::new("suite-unknown");
    let home = tmp.path().join("home");
    seed_suite(&home);
    let ctx = ctx(&home, tmp.path());

    let err = dispatch::dispatch("nosuchtool", &[], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(
        err.message
            .contains("no installed payload provides the command \"nosuchtool\""),
        "{}",
        err.message
    );
}

#[test]
fn pre_suite_mirrors_default_to_slot_zero() {
    // Mirrors written before the slot field existed carry no `slot:` —
    // they parse with slot 0 (whole image), dispatch unchanged.
    let m: tebako_shim::manifest::Manifest = tebako_shim::manifest::Manifest::parse(
        "schema_version: 1\nkind: app\nname: app\nversion: 1.0\nentrypoints:\n  - name: app\n    path: /app/bin/app\n",
        std::path::Path::new("test"),
    )
    .unwrap();
    assert_eq!(m.entrypoint("app").unwrap().slot, 0);
    // …and slot 0 serializes back out (the mirror stays minimal).
    let yaml = serde_yaml::to_string(&m).unwrap();
    assert!(!yaml.contains("slot:"), "{yaml}");
}
