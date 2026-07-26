//! The env-mutating legs of the install surface (TEBAKO_OFFLINE,
//! TEBAKO_REQUIRE_SIGNED) — a separate test binary because those
//! variables are process-global and would race the parallel suite in
//! install.rs. Within this file every test takes the one lock.

use std::fs;
use std::path::PathBuf;

use tebako_cli::install;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-cli-installenv-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Env {
    dir: PathBuf,
    home: PathBuf,
    shim_binary: PathBuf,
}

impl Env {
    fn new(tag: &str) -> Env {
        let dir = scratch(tag);
        let home = dir.join("home");
        let mirror = dir.join("mirror");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&mirror).unwrap();
        fs::write(mirror.join("app-1.0.tfs"), b"app-bytes").unwrap();
        fs::write(
            mirror.join("tpkg-registry.yaml"),
            format!(
                "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: file://{}/app-1.0.tfs}}\n        entrypoints: [app]\n    default: 1.0\n",
                mirror.display()
            ),
        )
        .unwrap();
        let shim_binary = dir.join("tebako-shim");
        fs::write(&shim_binary, b"#!/bin/sh\n").unwrap();
        Env {
            dir,
            home,
            shim_binary,
        }
    }

    fn register(&self) {
        let reg_ref = format!("file://{}/mirror/tpkg-registry.yaml", self.dir.display());
        install::add_registry(&self.home, &reg_ref).unwrap();
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn offline_is_cache_hit_or_hard_error() {
    let _guard = ENV_LOCK.lock().unwrap();
    let env = Env::new("offline");
    env.register();

    std::env::set_var("TEBAKO_OFFLINE", "1");
    // a miss is the named hard error — no download attempted
    let err = install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap_err();
    assert_eq!(err.code, 69, "{err:?}");
    assert!(err.message.contains("TEBAKO_OFFLINE"), "{err:?}");

    // install online, then a reinstall offline is a clean cache hit
    std::env::remove_var("TEBAKO_OFFLINE");
    install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap();
    std::env::set_var("TEBAKO_OFFLINE", "yes");
    let out = install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap();
    assert_eq!(out.status, tebako_resolve::InstallStatus::Hit);
    std::env::remove_var("TEBAKO_OFFLINE");
}

#[test]
fn require_signed_hard_fails_unsigned_entries() {
    let _guard = ENV_LOCK.lock().unwrap();
    let env = Env::new("reqsigned");
    env.register();

    std::env::set_var("TEBAKO_REQUIRE_SIGNED", "1");
    let err = install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap_err();
    assert_eq!(err.code, 71, "{err:?}");
    assert!(err.message.contains("TEBAKO_REQUIRE_SIGNED"), "{err:?}");
    assert!(!env.home.join("payloads/app/1.0.tfs").exists());
    std::env::remove_var("TEBAKO_REQUIRE_SIGNED");

    // without it the same entry installs (legacy warn path)
    install::install(&env.home, "app", None, Some(&env.shim_binary)).unwrap();
    assert!(env.home.join("payloads/app/1.0.tfs").exists());
}
