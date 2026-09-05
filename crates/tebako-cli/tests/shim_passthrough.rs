//! `tebako shim <verb> …` ≡ `tebako-shim <verb> …` (spec 07 §3's dual
//! spelling): the CLI calls the shim crate as a library, never spawned.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-shim-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(home: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(tebako_bin())
        .args(args)
        .env("TEBAKO_HOME", home)
        .output()
        .unwrap_or_else(|e| panic!("spawn failed: {e}"));
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn shim_list_on_an_empty_store_prints_the_empty_line() {
    let dir = scratch("list");
    let home = dir.join("home");
    let (code, stdout, _stderr) = run(&home, &["shim", "list"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("no installed payloads"), "{stdout}");
}

#[test]
fn shim_unknown_verb_is_ex_usage() {
    let dir = scratch("usage");
    let home = dir.join("home");
    let (code, _stdout, _stderr) = run(&home, &["shim", "frobnicate"]);
    assert_eq!(code, i32::from(tebako_shim::EX_USAGE));
}

#[test]
fn shim_use_roundtrips_through_the_passthrough() {
    let dir = scratch("use");
    let home = dir.join("home");
    let (code, stdout, _stderr) = run(&home, &["shim", "use", "pandoc", "pandorc@1.2.0"]);
    assert_eq!(code, 0, "{stdout}");
    let cfg = fs::read_to_string(home.join("config.yaml")).unwrap();
    assert!(cfg.contains("pandoc"), "{cfg}");
    assert!(cfg.contains("pandorc@1.2.0"), "{cfg}");
    let (code, stdout, _stderr) = run(&home, &["shim", "use", "--clear", "pandoc"]);
    assert_eq!(code, 0, "{stdout}");
    let cfg = fs::read_to_string(home.join("config.yaml")).unwrap();
    assert!(!cfg.contains("pandoc"), "{cfg}");
}
