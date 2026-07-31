//! Binary-level smoke of the registry/install surface: main.rs argv
//! handling end to end (add-registry → list-registries → install →
//! uninstall) against a file:// mirror and a temp TEBAKO_HOME.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-reg-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(home: &PathBuf, shim: &PathBuf, args: &[&str]) -> (i32, String) {
    let out = Command::new(tebako_bin())
        .args(args)
        .env("TEBAKO_HOME", home)
        .env("TEBAKO_SHIM_BINARY", shim)
        .output()
        .unwrap_or_else(|e| panic!("spawn failed: {e}"));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn registry_install_uninstall_smoke() {
    let dir = scratch("smoke");
    let home = dir.join("home");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    let shim = dir.join("tebako-shim");
    fs::write(&shim, b"#!/bin/sh\n").unwrap();

    fs::write(mirror.join("app-1.0.tfs"), b"app-bytes").unwrap();
    fs::write(
        mirror.join("tpkg-registry.yaml"),
        format!(
            "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: file://{}/app-1.0.tfs}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [app]\n    default: 1.0\n",
            mirror.display()
        ),
    )
    .unwrap();
    let reg_ref = format!("file://{}/tpkg-registry.yaml", mirror.display());

    let (code, text) = run(&home, &shim, &["add-registry", &reg_ref]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("registered registry"), "{text}");
    assert!(text.contains("1 payload(s): app"), "{text}");

    let (code, text) = run(&home, &shim, &["list-registries"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains(&reg_ref), "{text}");

    // update-registries: a file:// registry is reported as local (nothing
    // to cache), exit 0
    let (code, text) = run(&home, &shim, &["update-registries"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("nothing to cache"), "{text}");

    let (code, text) = run(&home, &shim, &["install", "app"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("installed app 1.0"), "{text}");
    assert!(home.join("payloads/app/1.0.tfs").is_file(), "{text}");
    assert!(home.join("shims/app").exists(), "{text}");

    // the legacy unsigned warning fires on stderr but does not fail
    assert!(text.contains("WARNING"), "{text}");

    // nickname resolution error surfaces with the registry list + hint
    let (code, text) = run(&home, &shim, &["install", "bogus"]);
    assert_ne!(code, 0);
    assert!(
        text.contains("no registered registry carries a payload named 'bogus'"),
        "{text}"
    );
    assert!(text.contains("tebako add-registry <ref>"), "{text}");

    let (code, text) = run(&home, &shim, &["uninstall", "app"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("removed app (1.0)"), "{text}");
    assert!(!home.join("payloads/app").exists());
    assert!(!home.join("shims/app").exists());

    let _ = fs::remove_dir_all(&dir);
}
