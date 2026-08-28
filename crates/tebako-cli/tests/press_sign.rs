//! Press signing at the CLI surface (spec 09 §9, spec 23 §14): the
//! `--sign[=<keyid>]` / `--no-sign` flags through the real binary, with
//! temp TEBAKO_HOMEs and no network —
//!
//! - a `--sign=<keyid>` naming no key in $TEBAKO_HOME/keys is the NAMED
//!   error 71 raised BEFORE any heavy work (the scenario, the bootstrap
//!   store, the runtime download all come later);
//! - `--no-sign` overriding `TEBAKO_SIGN=1` is LOUD (stderr warning +
//!   the audit journal's `event=press-sign-opt-out`) while the press
//!   itself proceeds unsigned — here it then stops at the offline
//!   bootstrap/runtime resolution, which changes nothing about the
//!   opt-out's loudness.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-cli-press-sign-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(cmd: &mut Command) -> (i32, String) {
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn failed: {e}"));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn sign_with_an_unknown_keyid_fails_before_any_heavy_work() {
    let dir = scratch("unknown-keyid");
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();

    // -r/-e deliberately name nothing real: the keyid check must fire
    // BEFORE the scenario checks, the bootstrap store flow and the
    // runtime download (spec 09 §9 — never a silent fallback).
    let (code, log) = run(Command::new(tebako_bin())
        .args([
            "press",
            "-r",
            "/nonexistent-root",
            "-e",
            "start.rb",
            "-o",
            &dir.join("pkg").display().to_string(),
            "-p",
            &dir.join("prefix").display().to_string(),
            "--sign=0000000000000000",
        ])
        .env("TEBAKO_HOME", &home)
        .env("TEBAKO_OFFLINE", "1")
        .env_remove("TEBAKO_BOOTSTRAP")
        .env_remove("TEBAKO_SIGN"));
    assert_eq!(code, 71, "{log}");
    assert!(
        log.contains("no secret key with keyid 0000000000000000"),
        "{log}"
    );
    // Nothing heavy ran: no key material, no store dirs.
    assert!(!home.join("keys").exists());
    assert!(!home.join("runtimes").exists());
    assert!(!home.join("bootstraps").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn no_sign_over_a_lower_channel_declaration_is_loud() {
    let dir = scratch("opt-out");
    let home = dir.join("home");
    let root = dir.join("root");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("start.rb"), "puts 'hi'\n").unwrap();

    // --no-sign over TEBAKO_SIGN=1: the press continues UNSIGNED (and
    // here stops at the offline bootstrap/runtime resolution), but the
    // dropped declaration is warned about and journaled.
    let (code, log) = run(Command::new(tebako_bin())
        .args([
            "press",
            "-r",
            &root.display().to_string(),
            "-e",
            "start.rb",
            "-o",
            &dir.join("pkg").display().to_string(),
            "-p",
            &dir.join("prefix").display().to_string(),
            "--no-sign",
        ])
        .env("TEBAKO_HOME", &home)
        .env("TEBAKO_SIGN", "1")
        .env("TEBAKO_OFFLINE", "1")
        .env_remove("TEBAKO_BOOTSTRAP"));
    assert_ne!(code, 0, "the offline press cannot complete:\n{log}");
    assert!(
        log.contains("sign opt-out") && log.contains("press-sign-opt-out"),
        "the opt-out warning must name what it dropped:\n{log}"
    );
    let journal =
        fs::read_to_string(home.join("journal.log")).expect("the opt-out must be journaled");
    assert!(
        journal.contains("event=press-sign-opt-out by=cli overridden=env"),
        "{journal}"
    );
    // The unsigned path touches no key material.
    assert!(!home.join("keys").exists());
    let _ = fs::remove_dir_all(&dir);
}
