//! The zero-ceremony press's refusals, at the CLI boundary: a project the
//! press cannot build must say WHICH project it found and what it read —
//! the author-tier failure contract. These cases never reach the heavy
//! work (no runtime download, no bootstrap), so the suite needs nothing
//! beyond the binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tebako-press-detect-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run `tebako press` in `dir` (no flags) and return (exit code, combined
/// output): usage refusals go to stderr, named packaging errors to stdout
/// (the CLI's own split), and the contract is the TEXT either way.
fn press(dir: &Path) -> (i32, String) {
    let out = Command::new(tebako_bin())
        .arg("press")
        .current_dir(dir)
        .env("TEBAKO_HOME", dir.join("home"))
        .output()
        .expect("the CLI binary must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn a_node_project_without_a_lockfile_is_refused_by_name() {
    // The node backend presses (no longer a refusal), and its FIRST gate is
    // the lockfile: the press vendors lockfile-true, so a project without
    // one is a named error naming the file and the remedy — before any
    // runtime is resolved.
    let dir = scratch("node-nolock");
    fs::write(
        dir.join("package.json"),
        r#"{"name":"hello","bin":{"hello":"cli.js"},"engines":{"node":">=24"}}"#,
    )
    .unwrap();
    fs::write(dir.join("cli.js"), "console.log('hi')\n").unwrap();

    let (code, err) = press(&dir);
    assert_eq!(code, 255, "{err}");
    assert!(err.contains("package-lock.json"), "{err}");
    assert!(err.contains("lockfile-true"), "{err}");
    assert!(err.contains("npm install --package-lock-only"), "{err}");
}

#[test]
fn a_node_project_with_an_ambiguous_bin_is_refused_by_name() {
    let dir = scratch("node-ambiguous");
    fs::write(
        dir.join("package.json"),
        r#"{"name":"hello","bin":{"one":"a.js","two":"b.js"}}"#,
    )
    .unwrap();

    let (code, err) = press(&dir);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("several bin commands"), "{err}");
    assert!(err.contains("one, two"), "{err}");
    assert!(err.contains("-e <entry>"), "{err}");
}

#[test]
fn a_python_project_names_its_engine_and_the_current_limit() {
    let dir = scratch("python");
    fs::write(dir.join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();

    let (code, err) = press(&dir);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("python project"), "{err}");
    // No console command declared: the entry point is exactly what is
    // missing, so the refusal names that rather than the whole backend.
    assert!(err.contains("[project.scripts]"), "{err}");
}

/// PEP 621's console command and `requires-python` are derived, so the
/// refusal names what was read instead of calling the directory unknown
/// (tebako#862's first increment).
#[test]
fn a_python_project_names_the_console_command_and_requirement_it_read() {
    let dir = scratch("python-scripts");
    fs::write(
        dir.join("pyproject.toml"),
        "[project]\nname = \"x\"\nrequires-python = \">=3.11,<4\"\n\n[project.scripts]\nmycli = \"x.cli:main\"\n",
    )
    .unwrap();

    let (code, err) = press(&dir);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("console command is 'mycli'"), "{err}");
    assert!(err.contains("requires python >=3.11,<4"), "{err}");
    assert!(err.contains("derived, not guessed"), "{err}");
}

#[test]
fn a_python_project_with_several_scripts_is_refused_with_their_names() {
    let dir = scratch("python-many");
    fs::write(
        dir.join("pyproject.toml"),
        "[project]\nname = \"x\"\n\n[project.scripts]\na-one = \"x.a:main\"\nb-two = \"x.b:main\"\n",
    )
    .unwrap();

    let (code, err) = press(&dir);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("2 console commands"), "{err}");
    assert!(err.contains("a-one, b-two"), "{err}");
    assert!(err.contains("-e <entry>"), "{err}");
}

#[test]
fn an_unparseable_pyproject_is_named_as_such() {
    let dir = scratch("python-broken");
    fs::write(dir.join("pyproject.toml"), "[project\nname = ").unwrap();

    let (code, err) = press(&dir);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("pyproject.toml"), "{err}");
    assert!(err.contains("does not parse"), "{err}");
}

#[test]
fn an_empty_directory_names_what_was_looked_for() {
    let dir = scratch("empty");
    let (code, err) = press(&dir);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("no project detected"), "{err}");
    assert!(err.contains("Gemfile"), "{err}");
    assert!(err.contains("-r <root> -e <entry>"), "{err}");
}
