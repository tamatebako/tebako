//! Management commands: enable/disable round-trips, which, doctor, list,
//! and the argv0 two-face split (spec 07 §2.0, §3).

mod common;

use common::*;
use tebako_shim::{Action, Ctx};

fn run_ok(argv: &[String], ctx: &Ctx) -> Action {
    tebako_shim::run(argv, ctx).expect("run")
}

fn printed(action: Action) -> (String, u8) {
    match action {
        Action::Print { text, code } => (text, code),
        Action::Exec(_) => panic!("expected Print, got Exec"),
    }
}

fn seed(home: &std::path::Path) {
    write_payload(
        home,
        "metanorma",
        "1.2.3",
        &app_manifest(
            "metanorma",
            "1.2.3",
            &entrypoint_yaml(RUBY_ENTRY, "metanorma"),
        ),
    );
    write_runtime(home, "4.0.6", "0.16.0", false);
}

#[test]
fn disabled_selectors_gain_the_payload_dimension() {
    use tebako_shim::config::{claim_disabled, is_disabled, Disabled};
    let mut disabled = Disabled::default();
    // a bare version gates that version of ANY claim
    disabled.insert("pandoc".to_string(), vec!["1.0".to_string()]);
    assert!(is_disabled(&disabled, "pandoc", "anypayload", "1.0"));
    assert!(!is_disabled(&disabled, "pandoc", "anypayload", "1.1"));
    assert!(!claim_disabled(&disabled, "pandoc", "anypayload"));
    // `all` gates every claim
    disabled.insert("pandoc".to_string(), vec!["all".to_string()]);
    assert!(is_disabled(&disabled, "pandoc", "p", "9.9"));
    assert!(claim_disabled(&disabled, "pandoc", "p"));
    // p@all gates only payload p's claim
    disabled.insert("pandoc".to_string(), vec!["pandorc@all".to_string()]);
    assert!(is_disabled(&disabled, "pandoc", "pandorc", "1.2.0"));
    assert!(!is_disabled(&disabled, "pandoc", "pandora", "1.2.0"));
    assert!(claim_disabled(&disabled, "pandoc", "pandorc"));
    assert!(!claim_disabled(&disabled, "pandoc", "pandora"));
    // p@1.0 gates exactly that pair (and is not a full-claim gate)
    disabled.insert("pandoc".to_string(), vec!["pandorc@1.0".to_string()]);
    assert!(is_disabled(&disabled, "pandoc", "pandorc", "1.0"));
    assert!(!is_disabled(&disabled, "pandoc", "pandorc", "1.1"));
    assert!(!claim_disabled(&disabled, "pandoc", "pandorc"));
}

#[test]
fn an_unknown_selector_string_is_a_named_load_error() {
    let tmp = TempDir::new("bad-selector");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(home.join("shims")).unwrap();
    std::fs::write(
        home.join("shims").join(".disabled.yaml"),
        "pandoc:\n  - \"bogus@\"\n",
    )
    .unwrap();
    let err = tebako_shim::config::load_disabled(&home).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("bogus@"), "{}", err.message);
}

fn pinned_ctx(home: &std::path::Path, cwd: &std::path::Path) -> Ctx {
    let mut ctx = ctx(home, cwd);
    ctx.env
        .insert("TEBAKO_METANORMA_VERSION".into(), "1.2.3".into());
    ctx
}

#[test]
fn argv0_selects_dispatch_vs_management() {
    let tmp = TempDir::new("argv0");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    // invoked as the tool → an exec plan
    let action = run_ok(
        &[
            home.join("shims")
                .join("metanorma")
                .to_string_lossy()
                .into_owned(),
            "--version".into(),
        ],
        &ctx,
    );
    match action {
        Action::Exec(plan) => {
            assert_eq!(plan.argv.last().unwrap(), "--version");
        }
        Action::Print { .. } => panic!("expected Exec"),
    }

    // invoked as tebako-shim → management
    let action = run_ok(&["tebako-shim".into(), "help".into()], &ctx);
    let (text, code) = printed(action);
    assert_eq!(code, 0);
    assert!(text.contains("install-shell"));

    let err = tebako_shim::run(&["tebako-shim".into(), "frobnicate".into()], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_USAGE);
}

#[test]
fn disable_and_enable_roundtrip_through_run() {
    let tmp = TempDir::new("enable-disable");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    // disable the whole tool → dispatch refused
    let (text, _) = printed(run_ok(
        &["tebako-shim".into(), "disable".into(), "metanorma".into()],
        &ctx,
    ));
    assert!(text.contains("disabled metanorma"), "{text}");
    let err = tebako_shim::run(&["metanorma".into()], &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);

    // idempotent
    let (text, _) = printed(run_ok(
        &["tebako-shim".into(), "disable".into(), "metanorma".into()],
        &ctx,
    ));
    assert!(text.contains("already disabled"), "{text}");

    // enable restores
    run_ok(
        &["tebako-shim".into(), "enable".into(), "metanorma".into()],
        &ctx,
    );
    match run_ok(&["metanorma".into()], &ctx) {
        Action::Exec(_) => {}
        Action::Print { .. } => panic!("expected Exec after enable"),
    }
}

#[test]
fn enable_links_a_declared_but_unlinked_command() {
    // spec 03 §2.2's `active: false` shape: the command is declared in the
    // mirror, install linked nothing for it; `enable` materializes the
    // link (the dispatcher links itself) and clears the gate.
    let tmp = TempDir::new("enable-links");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    let link = home.join("shims").join(if cfg!(windows) {
        "metanorma.exe"
    } else {
        "metanorma"
    });
    assert!(!link.exists(), "seed links nothing");

    let (text, code) = printed(run_ok(
        &["tebako-shim".into(), "enable".into(), "metanorma".into()],
        &ctx,
    ));
    assert_eq!(code, 0);
    assert!(text.contains("linked"), "{text}");
    assert!(link.exists(), "enable materialized the link");

    // idempotent: a second enable finds the link present — no relink note
    let (text, _) = printed(run_ok(
        &["tebako-shim".into(), "enable".into(), "metanorma".into()],
        &ctx,
    ));
    assert!(!text.contains("linked"), "{text}");
}

#[test]
fn enable_an_undeclared_command_is_the_named_error_and_links_nothing() {
    let tmp = TempDir::new("enable-undeclared");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    let err = tebako_shim::run(
        &["tebako-shim".into(), "enable".into(), "phantom".into()],
        &ctx,
    )
    .unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("phantom"), "{}", err.message);
    let link = home.join("shims").join(if cfg!(windows) {
        "phantom.exe"
    } else {
        "phantom"
    });
    assert!(!link.exists(), "no dangling link for an undeclared command");
}

#[test]
fn which_reports_the_full_resolution() {
    let tmp = TempDir::new("which");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    let (text, code) = printed(run_ok(
        &["tebako-shim".into(), "which".into(), "metanorma".into()],
        &ctx,
    ));
    assert_eq!(code, 0);
    assert!(text.contains("tool: metanorma"), "{text}");
    assert!(text.contains("payload: metanorma 1.2.3"), "{text}");
    assert!(
        text.contains("version source: env TEBAKO_METANORMA_VERSION"),
        "{text}"
    );
    assert!(
        text.contains("runtime: ruby \">= 3.3, < 5.0\" → ruby 4.0.6 (cached)"),
        "{text}"
    );
    assert!(text.contains("--tebako-entry"), "{text}");
}

#[test]
fn list_shows_versions_commands_and_resolution() {
    let tmp = TempDir::new("list");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    let (text, code) = printed(run_ok(&["tebako-shim".into(), "list".into()], &ctx));
    assert_eq!(code, 0);
    assert!(text.contains("metanorma"), "{text}");
    assert!(text.contains("versions: 1.2.3"), "{text}");
    assert!(text.contains("command metanorma"), "{text}");
    assert!(
        text.contains("resolved: 1.2.3 (from env TEBAKO_METANORMA_VERSION)"),
        "{text}"
    );
}

#[test]
fn doctor_reports_a_clean_setup_and_corruption() {
    let tmp = TempDir::new("doctor");
    let home = tmp.path().join("home");
    seed(&home);
    let shims = home.join("shims");
    std::fs::create_dir_all(&shims).unwrap();
    std::fs::write(shims.join("metanorma"), b"shim link placeholder\n").unwrap();

    // clean: shim on PATH, all records consistent
    let mut ctx = pinned_ctx(&home, tmp.path());
    ctx.env.insert(
        "PATH".into(),
        std::env::join_paths([&shims, std::path::Path::new("/usr/bin")])
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    );
    let (text, code) = printed(run_ok(&["tebako-shim".into(), "doctor".into()], &ctx));
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("no problems found"), "{text}");

    // PATH without the shim dir → problem
    let (text, code) = printed(run_ok(
        &["tebako-shim".into(), "doctor".into()],
        &pinned_ctx(&home, tmp.path()),
    ));
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("not on PATH"), "{text}");

    // corrupt the payload image → the trust anchor catches it
    let ctx = {
        let mut c = pinned_ctx(&home, tmp.path());
        c.env.insert(
            "PATH".into(),
            std::env::join_paths([&shims, std::path::Path::new("/usr/bin")])
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        );
        c
    };
    std::fs::write(
        home.join("payloads").join("metanorma").join("1.2.3.tfs"),
        b"tampered image\n",
    )
    .unwrap();
    let (text, code) = printed(run_ok(&["tebako-shim".into(), "doctor".into()], &ctx));
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("sha256 mismatch"), "{text}");
}

#[cfg(not(windows))] // the unix rc-file flow; the Windows registry form is covered in shell_windows.rs
#[test]
fn install_shell_roundtrip_through_run() {
    let tmp = TempDir::new("install-shell");
    let home = tmp.path().join("home");
    let user_home = tmp.path().join("user");
    std::fs::create_dir_all(&user_home).unwrap();
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("HOME".into(), user_home.to_string_lossy().into_owned());

    let argv: Vec<String> = vec![
        "tebako-shim".into(),
        "install-shell".into(),
        "--shell".into(),
        "bash".into(),
    ];
    let (text, code) = printed(run_ok(&argv, &ctx));
    assert_eq!(code, 0);
    assert!(text.contains("installed"), "{text}");
    let rc = user_home.join(".bashrc");
    let first = std::fs::read_to_string(&rc).unwrap();
    assert!(first.contains(tebako_shim::shell::BEGIN_MARKER));

    // idempotent
    let (text, code) = printed(run_ok(&argv, &ctx));
    assert_eq!(code, 0);
    assert!(text.contains("already present"), "{text}");
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), first);

    let argv: Vec<String> = vec![
        "tebako-shim".into(),
        "uninstall-shell".into(),
        "--shell=bash".into(),
    ];
    let (text, code) = printed(run_ok(&argv, &ctx));
    assert_eq!(code, 0);
    assert!(text.contains("removed"), "{text}");
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), "");
}

#[test]
fn link_and_unlink_use_the_platform_shim_name() {
    // spec 07 §3 + TODO.v2-1/05: the shim file is `<command>` on unix
    // (permission-bit executability) and `<command>.exe` on Windows
    // (PATHEXT resolution); unlink removes exactly the same name.
    let tmp = TempDir::new("link-name");
    let home = tmp.path().join("home");
    let dispatcher = tmp.path().join("dispatcher-bin");
    std::fs::write(&dispatcher, b"dispatcher\n").unwrap();

    #[cfg(windows)]
    let want = "metanorma.exe";
    #[cfg(not(windows))]
    let want = "metanorma";

    let (linked, notes) =
        tebako_shim::manage::link_shims(&home, &dispatcher, &["metanorma".to_string()])
            .expect("link");
    assert_eq!(linked, vec![home.join("shims").join(want)]);
    assert!(home.join("shims").join(want).exists());
    // Same-volume temp dirs: the Windows shape is a same-content link
    // (hardlink or copy) — either way byte-identical to the dispatcher —
    // and the fallback note only fires when the hardlink is declined.
    #[cfg(unix)]
    {
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(
            std::fs::read_link(home.join("shims").join(want)).unwrap(),
            dispatcher
        );
    }
    #[cfg(windows)]
    {
        assert_eq!(
            std::fs::read(home.join("shims").join(want)).unwrap(),
            std::fs::read(&dispatcher).unwrap()
        );
        // A fallback (cross-volume) is the only note the link emits, and
        // it names itself.
        for note in &notes {
            assert!(note.contains("copied the dispatcher"), "{note}");
        }
    }

    let removed =
        tebako_shim::manage::unlink_shims(&home, &["metanorma".to_string()]).expect("unlink");
    assert_eq!(removed, vec![home.join("shims").join(want)]);
    assert!(!home.join("shims").join(want).exists());
}

// ---------------------------------------------------------------------
// The verb surface of the 2026-09-05 routing amendment (spec 07 §3):
// use / enable|disable --of / list --json / which provider / doctor.
// ---------------------------------------------------------------------

fn seed_two_providers(home: &std::path::Path) {
    write_payload(
        home,
        "pandora",
        "1.0.0",
        &app_manifest("pandora", "1.0.0", &entrypoint_yaml(RUBY_ENTRY, "pandoc")),
    );
    write_payload(
        home,
        "pandorc",
        "1.2.0",
        &app_manifest("pandorc", "1.2.0", &entrypoint_yaml(RUBY_ENTRY, "pandoc")),
    );
    write_runtime(home, "4.0.6", "0.16.0", false);
}

#[test]
fn use_writes_clears_and_preserves_the_authored_config() {
    let tmp = TempDir::new("use-roundtrip");
    let home = tmp.path().join("home");
    write_config(
        &home,
        "registries:\n  - file:///opt/lib/tpkg-registry.yaml\ndefaults: {other: 9.9}\n",
    );
    let ctx = ctx(&home, tmp.path());

    // use <tool> <pin> writes the qualified default
    let (text, code) = printed(run_ok(
        &[
            "tebako-shim".into(),
            "use".into(),
            "pandoc".into(),
            "pandorc@1.2.0".into(),
        ],
        &ctx,
    ));
    assert_eq!(code, 0, "{text}");
    let cfg = tebako_shim::config::load_config(&home).unwrap();
    assert_eq!(
        cfg.defaults.get("pandoc").map(String::as_str),
        Some("pandorc@1.2.0")
    );
    assert_eq!(cfg.defaults.get("other").map(String::as_str), Some("9.9"));
    assert_eq!(
        cfg.registries,
        vec!["file:///opt/lib/tpkg-registry.yaml".to_string()]
    );

    // a second use edits in place
    run_ok(
        &[
            "tebako-shim".into(),
            "use".into(),
            "pandoc".into(),
            "1.0.0".into(),
        ],
        &ctx,
    );
    let cfg = tebako_shim::config::load_config(&home).unwrap();
    assert_eq!(
        cfg.defaults.get("pandoc").map(String::as_str),
        Some("1.0.0")
    );

    // --clear removes exactly the key
    run_ok(
        &[
            "tebako-shim".into(),
            "use".into(),
            "--clear".into(),
            "pandoc".into(),
        ],
        &ctx,
    );
    let cfg = tebako_shim::config::load_config(&home).unwrap();
    assert!(!cfg.defaults.contains_key("pandoc"));
    assert_eq!(cfg.defaults.get("other").map(String::as_str), Some("9.9"));

    // an unparseable pin is the named grammar error, and writes nothing
    let err = tebako_shim::run(
        &[
            "tebako-shim".into(),
            "use".into(),
            "pandoc".into(),
            "pandorc@".into(),
        ],
        &ctx,
    )
    .unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("pandorc@"), "{}", err.message);
    let cfg = tebako_shim::config::load_config(&home).unwrap();
    assert!(!cfg.defaults.contains_key("pandoc"));
}

#[test]
fn use_runtime_writes_the_runtimes_preference() {
    let tmp = TempDir::new("use-runtime");
    let home = tmp.path().join("home");
    let ctx = ctx(&home, tmp.path());

    run_ok(
        &[
            "tebako-shim".into(),
            "use".into(),
            "--runtime".into(),
            "ruby@3.4.2".into(),
        ],
        &ctx,
    );
    let cfg = tebako_shim::config::load_config(&home).unwrap();
    assert_eq!(
        cfg.runtimes.get("ruby").map(|r| r.version.as_str()),
        Some("3.4.2")
    );

    // the full form pins the tebako (launcher abi) line too
    run_ok(
        &[
            "tebako-shim".into(),
            "use".into(),
            "--runtime".into(),
            "ruby@3.4.2:2.2.0".into(),
        ],
        &ctx,
    );
    let cfg = tebako_shim::config::load_config(&home).unwrap();
    let pref = cfg.runtimes.get("ruby").expect("ruby pref");
    assert_eq!(pref.version, "3.4.2");
    assert_eq!(pref.tebako, "2.2.0");

    // no engine is a usage error
    let err = tebako_shim::run(
        &[
            "tebako-shim".into(),
            "use".into(),
            "--runtime".into(),
            "3.4.2".into(),
        ],
        &ctx,
    )
    .unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_USAGE);
}

#[test]
fn disable_of_writes_the_qualified_selector() {
    let tmp = TempDir::new("disable-of");
    let home = tmp.path().join("home");
    seed_two_providers(&home);
    // the env pin lets enable's link-on-demand resolve (spec 07 §3)
    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_PANDOC_VERSION".into(), "1.0.0".into());

    // --of alone → payload@all
    let (text, _) = printed(run_ok(
        &[
            "tebako-shim".into(),
            "disable".into(),
            "pandoc".into(),
            "--of".into(),
            "pandorc".into(),
        ],
        &ctx,
    ));
    assert!(text.contains("pandorc@all"), "{text}");
    let disabled = tebako_shim::config::load_disabled(&home).unwrap();
    assert_eq!(
        disabled.get("pandoc"),
        Some(&vec!["pandorc@all".to_string()])
    );

    // <tool>@<version> --of → payload@version
    run_ok(
        &[
            "tebako-shim".into(),
            "disable".into(),
            "pandoc@1.2.0".into(),
            "--of".into(),
            "pandorc".into(),
        ],
        &ctx,
    );
    let disabled = tebako_shim::config::load_disabled(&home).unwrap();
    assert_eq!(
        disabled.get("pandoc"),
        Some(&vec![
            "pandorc@all".to_string(),
            "pandorc@1.2.0".to_string()
        ])
    );

    // enable drops exactly the computed selector
    run_ok(
        &[
            "tebako-shim".into(),
            "enable".into(),
            "pandoc@1.2.0".into(),
            "--of".into(),
            "pandorc".into(),
        ],
        &ctx,
    );
    let disabled = tebako_shim::config::load_disabled(&home).unwrap();
    assert_eq!(
        disabled.get("pandoc"),
        Some(&vec!["pandorc@all".to_string()])
    );
}

#[test]
fn list_json_is_a_machine_document_with_provider_fields() {
    let tmp = TempDir::new("list-json");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    let (text, code) = printed(run_ok(
        &["tebako-shim".into(), "list".into(), "--json".into()],
        &ctx,
    ));
    assert_eq!(code, 0, "{text}");
    let doc = tebako_json::parse(&text).expect("list --json parses as JSON");
    assert_eq!(doc.find("info_schema").and_then(|v| v.as_u64()), Some(1));
    let commands = doc.find("commands").expect("commands");
    let tebako_json::Value::Array(commands) = commands else {
        panic!("commands is an array");
    };
    let cmd = commands
        .iter()
        .find(|c| c.find("name").and_then(|v| v.as_string()).as_deref() == Some("metanorma"))
        .expect("the metanorma command entry");
    assert_eq!(
        cmd.find("provider").and_then(|v| v.as_string()).as_deref(),
        Some("metanorma")
    );
    assert_eq!(
        cmd.find("provider_kind")
            .and_then(|v| v.as_string())
            .as_deref(),
        Some("own")
    );
    assert_eq!(
        cmd.find("version").and_then(|v| v.as_string()).as_deref(),
        Some("1.2.3")
    );
    assert!(
        cmd.find("source")
            .and_then(|v| v.as_string())
            .is_some_and(|s| s.contains("TEBAKO_METANORMA_VERSION")),
        "the source string"
    );
}

#[test]
fn which_names_the_provider_and_its_kind() {
    let tmp = TempDir::new("which-provider");
    let home = tmp.path().join("home");
    seed(&home);
    let ctx = pinned_ctx(&home, tmp.path());

    let (text, code) = printed(run_ok(
        &["tebako-shim".into(), "which".into(), "metanorma".into()],
        &ctx,
    ));
    assert_eq!(code, 0);
    assert!(text.contains("provider: metanorma (own)"), "{text}");
}

#[test]
fn which_shows_the_qualified_source_when_pinned() {
    let tmp = TempDir::new("which-pinned");
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    seed_two_providers(&home);
    std::fs::write(proj.join(".tebako-tools.yaml"), "pandoc: pandorc@1.2.0\n").unwrap();
    let ctx = ctx(&home, &proj);

    let (text, code) = printed(run_ok(
        &["tebako-shim".into(), "which".into(), "pandoc".into()],
        &ctx,
    ));
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("provider: pandorc (pinned)"), "{text}");
    assert!(text.contains("pandorc@1.2.0"), "{text}");
}

#[test]
fn doctor_reports_collisions_dangling_pins_and_disabled_but_pinned() {
    let tmp = TempDir::new("doctor-routing");
    let home = tmp.path().join("home");
    seed_two_providers(&home);
    let shims = home.join("shims");
    std::fs::create_dir_all(&shims).unwrap();
    std::fs::write(shims.join("pandoc"), b"shim link placeholder\n").unwrap();
    std::fs::write(shims.join(".disabled.yaml"), "pandoc:\n  - pandora@1.0.0\n").unwrap();
    write_config(
        &home,
        "defaults: {pandoc: \"pandora@1.0.0\", ghost: \"ghostcmd@9.9\"}\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "PATH".into(),
        std::env::join_paths([&shims, std::path::Path::new("/usr/bin")])
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    );

    let (text, _code) = printed(run_ok(&["tebako-shim".into(), "doctor".into()], &ctx));
    assert!(text.contains("more than one enabled"), "{text}");
    assert!(text.contains("ghostcmd@9.9"), "{text}");
    assert!(
        text.contains("dangling") || text.contains("not installed"),
        "{text}"
    );
    assert!(
        text.contains("disabled") && text.contains("pandora@1.0.0"),
        "{text}"
    );
}
