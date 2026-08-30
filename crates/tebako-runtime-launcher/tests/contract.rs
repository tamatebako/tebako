//! The tebako-runtime-launcher contract suite: the REAL process layer
//! (spec 29 §1/§4) driven end-to-end — the binary boots the spec-17
//! wire, materializes the declared interpreter, and execs it, with the
//! exit code and argv observable from outside.
//!
//! The "interpreter" is a `#!/bin/sh` script: the tests carry no
//! compiler, and a script is the one executable form every unix host
//! already has. The fixtures declare `visibility: exec-cache` — never
//! preload: on macOS the preload tier would inject into /bin/sh (an
//! Apple platform binary, where DYLD_INSERT_LIBRARIES is fatal), and on
//! linux the interposition is the driver's own suite's business, not
//! this process-level one. The fixtures are TAR images written in-code
//! (the zip backend pins files to 0o644 — the stub must materialize
//! with its exec bit; tar honors entry modes).
//!
//! unix-only: the windows process layer (spawn+wait+propagate) is real
//! std-only code in main.rs but no windows leg builds or runs this
//! crate yet — said so in the PR.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

const LAUNCHER: &str = env!("CARGO_BIN_EXE_tebako-runtime-launcher");

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tebako-launcher-contract-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A tar image: explicit directory entries (0o755) plus files at their
/// declared modes — the exec bit is the point (see the header).
fn write_tar(path: &Path, dirs: &[&str], files: &[(&str, &[u8], u32)]) {
    let file = std::fs::File::create(path).expect("create tar");
    let mut b = tar::Builder::new(file);
    for d in dirs {
        let mut h = tar::Header::new_gnu();
        h.set_path(d).unwrap();
        h.set_size(0);
        h.set_mode(0o755);
        h.set_entry_type(tar::EntryType::Directory);
        h.set_cksum();
        b.append(&h, std::io::empty()).unwrap();
    }
    for (name, data, mode) in files {
        let mut h = tar::Header::new_gnu();
        h.set_path(name).unwrap();
        h.set_size(data.len() as u64);
        h.set_mode(*mode);
        h.set_cksum();
        b.append(&h, &data[..]).unwrap();
    }
    b.finish().unwrap();
}

/// The era-2 layout declaration (spec 18 C3) + the caller's spec-29
/// keys. The mount root pairs with the launcher's baked root.
fn layout(spec29_keys: &str) -> String {
    format!(
        "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"21\"\n{spec29_keys}"
    )
}

/// The stub interpreter: prints its argv and the two env markers, then
/// exits 7 — the verbatim-exit-code witness.
const STUB: &[u8] = b"#!/bin/sh\necho \"ARGV0=$0\"\ni=1\nfor a in \"$@\"; do echo \"ARG$i=$a\"; i=$((i+1)); done\necho \"EXEC_CACHE=${TEBAKO_EXEC_CACHE:-unset}\"\necho \"MARKER=${TEBAKO_STUB_MARKER:-unset}\"\nexit 7\n";

/// The env image: the layout declaration + the stub at bin/stub.
fn write_env_image(dir: &Path, spec29_keys: &str) -> PathBuf {
    let l = layout(spec29_keys);
    let p = dir.join("env.tfs");
    write_tar(
        &p,
        &["bin/", "lib/", "lib/tebako/"],
        &[
            ("lib/tebako/layout.yaml", l.as_bytes(), 0o644),
            ("bin/stub", STUB, 0o755),
        ],
    );
    p
}

/// The app payload: bin/app plus the manifest declaring the
/// entrypoint's `args_default: ["-jar"]` (spec 03 §2.2).
fn write_app_image(dir: &Path) -> PathBuf {
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: app\n  version: \"1\"\n  \
         producer: {{tool: t, tool_version: \"1\"}}\n  \
         created: \"2026-08-13T00:00:00Z\"\n  \
         digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
         signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
         provides:\n  \
         entrypoints: [{{name: app, path: /bin/app, args_default: [\"-jar\"]}}]\n  \
         platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        z = "0".repeat(64)
    );
    let p = dir.join("app.tfs");
    write_tar(
        &p,
        &["bin/", "__tpkg__/"],
        &[
            (
                "bin/app",
                b"#!/usr/bin/env java\necho app\n".as_slice(),
                0o755,
            ),
            ("__tpkg__/manifest.yaml", manifest.as_bytes(), 0o644),
        ],
    );
    p
}

/// The launcher invocation, hermetic: a clean environment carrying only
/// the temp redirect (the materialization cache roots under it), the
/// env-image handoff, and the test's marker.
fn launcher(tmp: &TempDir, env_image: &Path) -> Command {
    let mut cmd = Command::new(LAUNCHER);
    cmd.env_clear()
        .env("TMPDIR", tmp.path().join("tmp"))
        .env("TEBAKO_RUNTIME_IMAGE", env_image)
        .env("TEBAKO_STUB_MARKER", "contract-marker");
    std::fs::create_dir_all(tmp.path().join("tmp")).expect("create tmp redirect");
    cmd
}

fn stdout_lines(out: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(out)
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn exec_composes_argv_and_passes_the_exit_code_verbatim() {
    let tmp = TempDir::new("exec");
    let env_image = write_env_image(
        tmp.path(),
        "interpreter: /bin/stub\nvisibility: exec-cache\n",
    );
    let app = write_app_image(tmp.path());

    let out = launcher(&tmp, &env_image)
        .args([
            "--tebako-image".into(),
            format!("{}:-:/", app.display()),
            "--tebako-entry".into(),
            "/bin/app".into(),
            "user1".into(),
            "user 2".into(),
        ])
        .output()
        .expect("spawn the launcher");

    assert_eq!(
        out.status.code(),
        Some(7),
        "the stub's exit code passes through verbatim; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines = stdout_lines(&out.stdout);
    // [interpreter, args_default…, entry, user args…] (spec 29 §1).
    assert_eq!(lines.len(), 7, "{lines:?}");
    // ARGV0 is the MATERIALIZED host copy of the in-image stub.
    let argv0 = lines[0].strip_prefix("ARGV0=").expect("ARGV0 line");
    assert_ne!(argv0, "/__tfs__/bin/stub");
    assert_eq!(std::fs::read(argv0).unwrap(), STUB);
    assert_eq!(lines[1], "ARG1=-jar", "the entrypoint's args_default");
    // The entry is bridged to its host twin — the host-plain stub cannot
    // read the VFS spelling (spec 29 §3's exec-cache rule).
    let entry = lines[2].strip_prefix("ARG2=").expect("ARG2 line");
    assert_ne!(entry, "/bin/app");
    assert_eq!(
        std::fs::read(entry).unwrap(),
        b"#!/usr/bin/env java\necho app\n"
    );
    assert_eq!(lines[3], "ARG3=user1");
    assert_eq!(lines[4], "ARG4=user 2");
    // The boot's env arms flow to the exec'd child; the caller's own
    // env passes through untouched.
    assert!(lines[5].starts_with("EXEC_CACHE="), "{lines:?}");
    assert_ne!(lines[5], "EXEC_CACHE=unset", "the exec-cache export arms");
    assert_eq!(lines[6], "MARKER=contract-marker");
}

#[test]
fn the_bare_smoke_form_composes_the_interpreters_own_args() {
    let tmp = TempDir::new("bare");
    let env_image = write_env_image(
        tmp.path(),
        "interpreter: /bin/stub\nvisibility: exec-cache\n",
    );

    let out = launcher(&tmp, &env_image)
        .arg("--version")
        .output()
        .expect("spawn the launcher");

    assert_eq!(out.status.code(), Some(7));
    let lines = stdout_lines(&out.stdout);
    assert_eq!(lines[1], "ARG1=--version", "{lines:?}");
    // No payload, no entry: marker + exec-cache lines follow.
    assert_eq!(lines.len(), 4, "{lines:?}");
}

#[test]
fn the_absent_interpreter_key_is_a_named_65() {
    let tmp = TempDir::new("no-interp");
    let env_image = write_env_image(tmp.path(), "");

    let out = launcher(&tmp, &env_image)
        .arg("--version")
        .output()
        .expect("spawn the launcher");

    assert_eq!(out.status.code(), Some(65));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("layout.interpreter"), "{stderr}");
    assert!(stderr.contains("LINKED"), "{stderr}");
}

#[test]
fn an_unresolvable_interpreter_is_a_named_65() {
    let tmp = TempDir::new("interp-miss");
    let env_image = write_env_image(
        tmp.path(),
        "interpreter: /bin/java\nvisibility: exec-cache\n",
    );

    let out = launcher(&tmp, &env_image)
        .arg("--version")
        .output()
        .expect("spawn the launcher");

    assert_eq!(out.status.code(), Some(65));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("/bin/java"), "{stderr}");
    assert!(stderr.contains("/__tfs__"), "{stderr}");
}

#[test]
fn extract_dumps_and_exits_zero_without_exec() {
    let tmp = TempDir::new("extract");
    let env_image = write_env_image(
        tmp.path(),
        "interpreter: /bin/stub\nvisibility: exec-cache\n",
    );
    let app = write_app_image(tmp.path());
    let dest = tmp.path().join("dump");

    let out = launcher(&tmp, &env_image)
        .args([
            "--tebako-extract".into(),
            dest.display().to_string(),
            "--tebako-image".into(),
            format!("{}:-:/", app.display()),
        ])
        .output()
        .expect("spawn the launcher");

    assert_eq!(
        out.status.code(),
        Some(0),
        "extract exits 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The interpreter never ran — no ARGV lines, the note is on stderr.
    assert!(
        out.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("extracted"));
    assert_eq!(std::fs::read(dest.join("__tfs__/bin/stub")).unwrap(), STUB);
    assert!(dest.join("root/bin/app").is_file());
}
