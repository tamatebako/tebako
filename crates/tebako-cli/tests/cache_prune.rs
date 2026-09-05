//! `tebako cache prune --payloads` + the human `cache list` payload
//! section (spec 15 §4, the 2026-09-05 routing amendment's prune
//! protection): pins from config defaults, `.disabled.yaml` pairs and the
//! per-name newest floor are NEVER pruned — not even by `--all`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-cache-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(tebako_bin())
        .args(args)
        .env("TEBAKO_HOME", home)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("spawn failed: {e}"));
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// An installed payload record (image + trust anchor + origin; the
/// mirror only when the provider scan must read entrypoints).
fn write_payload(home: &Path, name: &str, version: &str, manifest_yaml: Option<&str>) {
    let dir = home.join("payloads").join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("{version}.tfs")), b"image").unwrap();
    fs::write(
        dir.join(format!("{version}.tfs.sha256")),
        format!("{}  {version}.tfs\n", "a".repeat(64)),
    )
    .unwrap();
    fs::write(dir.join(format!("{version}.tfs.origin")), "file:///x\n").unwrap();
    if let Some(m) = manifest_yaml {
        fs::write(dir.join(format!("{version}.manifest.yaml")), m).unwrap();
    }
}

/// The manifest-mirror shape the provider scan parses (the tebako-shim
/// fixture's, minimal): kind app with one entrypoint.
fn app_manifest(name: &str, version: &str, tool: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako-cli-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{tree}\"\n    blob_sha256: {blob}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {tool}\n      path: /app/bin/{tool}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        tree = "a".repeat(64),
        blob = "b".repeat(64),
    )
}

#[test]
fn prune_payloads_all_keeps_pins_and_the_newest_floor() {
    let dir = scratch("protect");
    let home = dir.join("home");
    for v in ["1.0", "2.0", "3.0"] {
        write_payload(&home, "tool", v, None);
    }
    fs::write(home.join("config.yaml"), "defaults: {tool: \"tool@1.0\"}\n").unwrap();

    let (code, stdout, _stderr) = run(&home, &dir, &["cache", "prune", "--payloads", "--all"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("Removed tool@2.0"), "{stdout}");
    assert!(!stdout.contains("Removed tool@1.0"), "{stdout}");
    assert!(!stdout.contains("Removed tool@3.0"), "{stdout}");
    assert!(stdout.contains("1 cached payload(s) removed"), "{stdout}");
    assert!(!home.join("payloads/tool/2.0.tfs").exists());
    assert!(!home.join("payloads/tool/2.0.tfs.sha256").exists());
    assert!(home.join("payloads/tool/1.0.tfs").is_file());
    assert!(home.join("payloads/tool/3.0.tfs").is_file());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bare_prune_all_touches_runtimes_only() {
    let dir = scratch("bare");
    let home = dir.join("home");
    // 1.0 would go under `--payloads --all` (2.0 is the newest floor).
    write_payload(&home, "tool", "1.0", None);
    write_payload(&home, "tool", "2.0", None);

    let (code, stdout, _stderr) = run(&home, &dir, &["cache", "prune", "--all"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("0 cached runtime package(s) removed"),
        "{stdout}"
    );
    assert!(!stdout.contains("payload"), "{stdout}");
    assert!(home.join("payloads/tool/1.0.tfs").is_file());
    assert!(home.join("payloads/tool/2.0.tfs").is_file());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn prune_payloads_older_than_prunes_old_unprotected() {
    let dir = scratch("older-than");
    let home = dir.join("home");
    write_payload(&home, "tool", "1.0", None);
    write_payload(&home, "tool", "2.0", None);

    // 0d: everything is old — the floor still keeps 2.0.
    let (code, stdout, _stderr) = run(
        &home,
        &dir,
        &["cache", "prune", "--payloads", "--older-than", "0d"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("Removed tool@1.0"), "{stdout}");
    assert!(!home.join("payloads/tool/1.0.tfs").exists());
    assert!(home.join("payloads/tool/2.0.tfs").is_file());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cache_list_human_shows_the_payload_section() {
    let dir = scratch("list");
    let home = dir.join("home");
    write_payload(&home, "tool", "1.0", None);

    let (code, stdout, _stderr) = run(&home, &dir, &["cache", "list"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("Payload cache"), "{stdout}");
    assert!(stdout.contains("tool@1.0"), "{stdout}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn prune_prints_the_project_pin_caveat_when_pins_exist() {
    let dir = scratch("caveat");
    let home = dir.join("home");
    write_payload(&home, "tool", "1.0", None);
    let proj = dir.join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join(".tebako-tools.yaml"), "tool: 1.0\n").unwrap();

    let (code, stdout, _stderr) = run(&home, &proj, &["cache", "prune", "--payloads", "--all"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains(
            "project pins (.tebako-tools.yaml) are per-directory and not visible to prune"
        ),
        "{stdout}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ambiguous_bare_default_prints_the_note_and_protects_nothing_extra() {
    let dir = scratch("ambiguous");
    let home = dir.join("home");
    write_payload(
        &home,
        "suite-a",
        "1.0",
        Some(&app_manifest("suite-a", "1.0", "mn")),
    );
    write_payload(
        &home,
        "suite-b",
        "1.0",
        Some(&app_manifest("suite-b", "1.0", "mn")),
    );
    fs::write(home.join("config.yaml"), "defaults: {mn: \"1.0\"}\n").unwrap();

    let (code, stdout, _stderr) = run(&home, &dir, &["cache", "prune", "--payloads", "--all"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("protecting nothing extra"), "{stdout}");
    assert!(stdout.contains("suite-a"), "{stdout}");
    assert!(stdout.contains("suite-b"), "{stdout}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn disabled_selector_pairs_are_protected() {
    let dir = scratch("disabled");
    let home = dir.join("home");
    write_payload(&home, "tool", "1.0", None);
    write_payload(&home, "tool", "2.0", None);
    write_payload(&home, "tool", "3.0", None);
    let shims = home.join("shims");
    fs::create_dir_all(&shims).unwrap();
    fs::write(shims.join(".disabled.yaml"), "tool:\n  - tool@1.0\n").unwrap();

    let (code, stdout, _stderr) = run(&home, &dir, &["cache", "prune", "--payloads", "--all"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("Removed tool@2.0"), "{stdout}");
    assert!(home.join("payloads/tool/1.0.tfs").is_file());
    assert!(!home.join("payloads/tool/2.0.tfs").exists());
    assert!(home.join("payloads/tool/3.0.tfs").is_file());
    let _ = fs::remove_dir_all(&dir);
}
