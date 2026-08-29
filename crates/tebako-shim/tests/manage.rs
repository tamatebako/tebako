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

    let link = home
        .join("shims")
        .join(if cfg!(windows) { "metanorma.exe" } else { "metanorma" });
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

    let err = tebako_shim::run(&["tebako-shim".into(), "enable".into(), "phantom".into()], &ctx)
        .unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("phantom"), "{}", err.message);
    let link = home
        .join("shims")
        .join(if cfg!(windows) { "phantom.exe" } else { "phantom" });
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
