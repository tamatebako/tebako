//! End-to-end tests for the tebako CLI: lean press of the simple-script
//! and Gemfile scenarios, cache management, option error codes, and the
//! side-by-side golden against the reference Ruby gem.
//!
//! Gates:
//! - the press tests need a mkdwarfs binary ($TEBAKO_MKDWARFS or PATH)
//!   and network access for the runtime download (skipped when
//!   TEBAKO_CLI_SKIP_E2E is set);
//! - the golden test additionally needs $TEBAKO_REFERENCE_GEM pointing at
//!   a checkout of the reference gem (tamatebako/tebako, the three-part
//!   model) and a host ruby with the thor gem.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// The heavy presses share ~/.tebako; serialize them so first-run layout
/// extraction cannot race (the reference gem accepts the same race).
fn press_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn e2e_allowed() -> bool {
    std::env::var_os("TEBAKO_CLI_SKIP_E2E").is_none()
}

fn workdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-e2e-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run a command; returns (exit code, combined stdout+stderr).
fn run(cmd: &mut Command) -> (i32, String) {
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn failed: {e}"));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), strip_legacy_warning(&text))
}

/// Item 29 v1-legacy rule: packages built unsigned (the default, per the
/// opt-in rule) make the bootstrap print a loud warning on stderr BY
/// DESIGN (two lines). Strip it for output comparison — the warning's
/// emission is covered by the bootstrap chain tests.
fn strip_legacy_warning(text: &str) -> String {
    let mut out = String::new();
    let mut skip_next = false;
    for line in text.lines() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if line.starts_with("tebako-bootstrap: WARNING: ")
            && line.contains("unsigned v1 (legacy) tpkg trailer")
        {
            skip_next = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for child in fs::read_dir(src).unwrap() {
        let child = child.unwrap();
        let target = dst.join(child.file_name());
        if child.path().is_dir() {
            copy_dir(&child.path(), &target);
        } else {
            fs::copy(child.path(), &target).unwrap();
        }
    }
}

/// A scratch press environment: the fixture root copied into a temp dir,
/// an empty packaging prefix (the CLI builds images in-process — no
/// mkdwarfs to provision; the golden's reference gem still provisions
/// its own, see golden_scenario).
struct PressEnv {
    work: PathBuf,
    root: PathBuf,
    prefix: PathBuf,
}

fn press_env(tag: &str, fixture: &str) -> Option<PressEnv> {
    if !e2e_allowed() {
        eprintln!("skipping {tag}: TEBAKO_CLI_SKIP_E2E is set");
        return None;
    }
    let work = workdir(tag);
    let root = work.join("root");
    copy_dir(&fixtures().join(fixture), &root);
    let prefix = work.join("prefix");
    fs::create_dir_all(prefix.join("deps")).unwrap();
    // Seed the CLI's cache version key so the cache guard stays silent
    // (and does not clean prefix/deps). The golden re-seeds the reference
    // gem's key before the gem press.
    seed_rs_version_file(&prefix);
    Some(PressEnv { work, root, prefix })
}

/// The CLI's own cache version key ("<version> at <crate manifest dir>").
fn seed_rs_version_file(prefix: &Path) {
    fs::write(
        prefix.join("deps").join(".environment.version"),
        format!(
            "{} at {}",
            tebako_cli::DEFAULT_TEBAKO_VERSION,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .unwrap();
}

fn press_command(env: &PressEnv, entry: &str, output: &Path) -> Command {
    let mut cmd = Command::new(tebako_bin());
    cmd.arg("press")
        .arg("-r")
        .arg(&env.root)
        .arg("-e")
        .arg(entry)
        .arg("-o")
        .arg(output)
        .arg("-p")
        .arg(&env.prefix);
    // Dogfood the in-workspace Rust bootstrap when it sits next to the
    // tebako binary (the decide_bootstrap default); otherwise the C++
    // release is downloaded — both are valid press paths.
    let sibling = tebako_bin().parent().unwrap().join(if cfg!(windows) {
        "tebako-bootstrap.exe"
    } else {
        "tebako-bootstrap"
    });
    if sibling.is_file() {
        cmd.env("TEBAKO_BOOTSTRAP", sibling);
    }
    cmd
}

#[test]
fn press_simple_script_runs() {
    let _guard = press_lock().lock().unwrap();
    let Some(env) = press_env("simple", "test-00") else {
        return;
    };
    let package = env.work.join("test-00");
    let (code, log) = run(&mut press_command(&env, "test.rb", &package));
    assert!(code == 0, "press failed:\n{log}");
    assert!(
        log.contains(&format!(
            "Created tebako package at \"{}\"",
            package.display()
        )),
        "unexpected press output:\n{log}"
    );
    let (code, out) = run(&mut Command::new(&package));
    assert_eq!(code, 0, "packaged binary failed:\n{out}");
    assert_eq!(out, "Hello!  This is test-00 talking from inside DwarFS\n");
}

#[test]
fn press_gemfile_runs() {
    let _guard = press_lock().lock().unwrap();
    let Some(env) = press_env("gemfile", "gemfile-app") else {
        return;
    };
    let package = env.work.join("gemfile-app");
    let (code, log) = run(&mut press_command(&env, "main.rb", &package));
    assert!(code == 0, "press failed:\n{log}");
    let (code, out) = run(&mut Command::new(&package));
    assert_eq!(code, 0, "packaged binary failed:\n{out}");
    assert!(
        out.starts_with("Hello from gemfile app with rake "),
        "unexpected output: {out}"
    );
}

#[test]
fn press_missing_entry_point_is_106() {
    let _guard = press_lock().lock().unwrap();
    let Some(env) = press_env("e106", "test-00") else {
        return;
    };
    let package = env.work.join("nope-pkg");
    let (code, log) = run(&mut press_command(&env, "nope.rb", &package));
    assert_eq!(code, 106, "expected exit 106:\n{log}");
    assert!(log.contains("[106]"), "expected the 106 tag:\n{log}");
}

#[test]
fn cache_list_and_prune() {
    let work = workdir("cache");
    let home = work.join("home");
    let entry = home.join("runtimes").join("ruby-9.9.9-0.0.1-testplatform");
    fs::create_dir_all(&entry).unwrap();
    fs::write(
        entry.join("tebako-runtime-0.0.1-9.9.9-testplatform"),
        b"fake",
    )
    .unwrap();

    let (code, out) = run(Command::new(tebako_bin())
        .args(["cache", "list"])
        .env("TEBAKO_HOME", &home));
    assert_eq!(code, 0);
    assert!(out.contains("ruby-9.9.9-0.0.1-testplatform"), "{out}");
    assert!(out.contains("Total (1 package(s))"), "{out}");

    let (code, out) = run(Command::new(tebako_bin())
        .args(["cache", "prune"])
        .env("TEBAKO_HOME", &home));
    assert_eq!(code, 0);
    assert!(
        out.contains("Nothing to do: pass --all or --older-than Nd"),
        "{out}"
    );

    let (code, out) = run(Command::new(tebako_bin())
        .args(["cache", "prune", "--all"])
        .env("TEBAKO_HOME", &home));
    assert_eq!(code, 0);
    assert!(
        out.contains("Removed ruby-9.9.9-0.0.1-testplatform"),
        "{out}"
    );
    assert!(out.contains("1 cached runtime package(s) removed"), "{out}");
    assert!(!entry.exists());
}

#[test]
fn press_option_errors() {
    let fixture = fixtures().join("test-00");
    let cases: Vec<(Vec<String>, i32, &str)> = vec![
        (
            vec!["press".into(), "-e".into(), "test.rb".into()],
            1,
            "No value provided for required options  '--root'",
        ),
        (
            vec![
                "press".into(),
                "-r".into(),
                "/nonexistent-tebako-root".into(),
                "-e".into(),
                "x.rb".into(),
            ],
            107,
            "[107]",
        ),
        (
            vec![
                "press".into(),
                "-r".into(),
                fixture.to_string_lossy().into_owned(),
                "-e".into(),
                "/etc/passwd".into(),
            ],
            114,
            "[114]",
        ),
        (
            vec![
                "press".into(),
                "-r".into(),
                fixture.to_string_lossy().into_owned(),
                "-e".into(),
                "test.rb".into(),
                "-R".into(),
                "9.9.9".into(),
            ],
            110,
            "[110]",
        ),
        (
            vec![
                "press".into(),
                "-r".into(),
                fixture.to_string_lossy().into_owned(),
                "-e".into(),
                "test.rb".into(),
                "-R".into(),
                "3.3".into(),
            ],
            109,
            "[109]",
        ),
        (
            vec![
                "press".into(),
                "-r".into(),
                fixture.to_string_lossy().into_owned(),
                "-e".into(),
                "test.rb".into(),
                "-m".into(),
                "runtime".into(),
            ],
            133,
            "[133]",
        ),
        (
            vec![
                "press".into(),
                "-r".into(),
                fixture.to_string_lossy().into_owned(),
                "-e".into(),
                "test.rb".into(),
                "-m".into(),
                "classic".into(),
            ],
            255,
            "later tebako-rs milestone",
        ),
    ];
    for (args, want_code, want_text) in cases {
        let (code, out) = run(Command::new(tebako_bin()).args(&args));
        assert_eq!(code, want_code, "args {args:?}:\n{out}");
        assert!(
            out.contains(want_text),
            "args {args:?} want '{want_text}':\n{out}"
        );
    }
}

// ---------------------------------------------------------------------
// Golden: side-by-side with the reference Ruby gem's lean press
// ---------------------------------------------------------------------

struct GoldenGem {
    root: PathBuf,
    version: String,
}

fn reference_gem() -> Option<GoldenGem> {
    let dir = std::env::var("TEBAKO_REFERENCE_GEM").ok()?;
    let root = fs::canonicalize(dir).ok()?;
    if !root.join("lib").join("tebako.rb").is_file() || !root.join("exe").join("tebako").is_file() {
        eprintln!("skipping golden: {root:?} is not a reference gem checkout");
        return None;
    }
    // thor is the gem's only non-stdlib dependency
    let (code, _) = run(Command::new("ruby").args(["-e", "require \"thor\""]));
    if code != 0 {
        eprintln!("skipping golden: host ruby lacks the thor gem");
        return None;
    }
    let version_rb = fs::read_to_string(root.join("lib").join("tebako").join("version.rb")).ok()?;
    let version = version_rb.split('"').nth(1).unwrap_or_default().to_string();
    if version != tebako_cli::DEFAULT_TEBAKO_VERSION {
        eprintln!(
            "skipping golden: reference gem is {version}, the CLI presses with {}",
            tebako_cli::DEFAULT_TEBAKO_VERSION
        );
        return None;
    }
    Some(GoldenGem { root, version })
}

/// Seed the gem's cache version file so its version_cache_check stays
/// silent ("<version> at <gem source>").
fn seed_gem_version_file(gem: &GoldenGem, prefix: &Path) {
    let deps = prefix.join("deps");
    fs::create_dir_all(&deps).unwrap();
    fs::write(
        deps.join(".environment.version"),
        format!("{} at {}", gem.version, gem.root.display()),
    )
    .unwrap();
}

fn gem_press_command(gem: &GoldenGem, env: &PressEnv, entry: &str, output: &Path) -> Command {
    let mut cmd = Command::new("ruby");
    cmd.arg("-I")
        .arg(gem.root.join("lib"))
        .arg(gem.root.join("exe").join("tebako"))
        .arg("press")
        .arg("-r")
        .arg(&env.root)
        .arg("-e")
        .arg(entry)
        .arg("-o")
        .arg(output)
        .arg("-p")
        .arg(&env.prefix);
    cmd
}

/// Lines that legitimately differ between the two presses:
/// - the RuntimeSdk provisioning (the documented deviation: the CLI does
///   not download the ruby src release — pure-ruby deploys never need
///   it);
/// - the `--tebako-extract` layout extraction, which only the FIRST
///   press on a cold runtime cache performs (whichever tool runs first —
///   an environment-state difference, not a behavioral one);
/// - the image build lines: the gem shells out to mkdwarfs ("-- Running
///   mkdwarfs script" + the "   ... @ <mkdwarfs> ..." echo) while the CLI
///   builds in-process ("-- Building DwarFS image ...") — the owner rule
///   is no mkdwarfs anywhere on the Rust side.
fn normalize_press_log(log: &str) -> String {
    log.lines()
        .filter(|line| {
            let gem_image_build = line.starts_with("-- Running mkdwarfs script")
                || (line.starts_with("   ... @ ") && line.contains("mkdwarfs"));
            !line.starts_with("-- Provisioning the runtime SDK")
                && !line.starts_with("   ... tfs-ruby-")
                && !line.contains(" --tebako-extract ")
                && !line.starts_with("-- Building DwarFS image")
                && !gem_image_build
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The reference gem still shells out to mkdwarfs; it is provisioned for
/// its press only (TEBAKO_MKDWARFS or PATH).
fn mkdwarfs_for_gem() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TEBAKO_MKDWARFS") {
        if Path::new(&p).is_file() {
            return Some(PathBuf::from(p));
        }
    }
    let path_var = std::env::var("PATH").ok()?;
    for dir in path_var.split(if cfg!(windows) { ';' } else { ':' }) {
        let candidate = Path::new(dir).join("mkdwarfs");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn golden_scenario(gem: &GoldenGem, tag: &str, fixture: &str, entry: &str, expect: &str) {
    let Some(env) = press_env(tag, fixture) else {
        return;
    };
    let package = env.work.join("pkg");

    // The gem errors 128 without <prefix>/deps/bin/mkdwarfs*; the CLI
    // needs nothing (in-process Writer).
    let mkdwarfs = mkdwarfs_for_gem().expect("golden requires mkdwarfs for the reference gem");
    let deps_bin = env.prefix.join("deps").join("bin");
    fs::create_dir_all(&deps_bin).unwrap();
    fs::copy(&mkdwarfs, deps_bin.join(mkdwarfs.file_name().unwrap())).unwrap();

    seed_gem_version_file(gem, &env.prefix);
    let (code, gem_log) = run(&mut gem_press_command(gem, &env, entry, &package));
    assert!(code == 0, "reference gem press failed:\n{gem_log}");
    let (code, gem_out) = run(&mut Command::new(&package));
    assert_eq!(code, 0, "gem-pressed binary failed:\n{gem_out}");

    // The gem's press left its own version key; re-seed the CLI's so its
    // cache guard stays silent as well.
    seed_rs_version_file(&env.prefix);
    let (code, rs_log) = run(&mut press_command(&env, entry, &package));
    assert!(code == 0, "tebako-rs press failed:\n{rs_log}");
    let (code, rs_out) = run(&mut Command::new(&package));
    assert_eq!(code, 0, "tebako-rs-pressed binary failed:\n{rs_out}");

    let gem_log = normalize_press_log(&gem_log);
    let rs_log = normalize_press_log(&rs_log);
    assert_eq!(
        gem_log.trim_end(),
        rs_log.trim_end(),
        "press outputs diverge (gem left, tebako-rs right)"
    );
    assert_eq!(gem_out, rs_out, "packaged binary outputs diverge");
    assert!(
        rs_out.contains(expect),
        "unexpected binary output: {rs_out}"
    );
}

#[test]
fn golden_side_by_side_with_the_gem() {
    let _guard = press_lock().lock().unwrap();
    if !e2e_allowed() || mkdwarfs_for_gem().is_none() {
        eprintln!("skipping golden: e2e disabled or no mkdwarfs for the reference gem");
        return;
    }
    let Some(gem) = reference_gem() else {
        return;
    };
    golden_scenario(
        &gem,
        "golden-simple",
        "test-00",
        "test.rb",
        "Hello!  This is test-00 talking from inside DwarFS",
    );
    golden_scenario(
        &gem,
        "golden-gemfile",
        "gemfile-app",
        "main.rb",
        "Hello from gemfile app with rake ",
    );
}

#[test]
fn cache_version_guard_matches_the_gem() {
    // A press that fails fast (missing root) AFTER the guard ran: the
    // guard's lines must precede the error, and the guard must act.
    let run_press = |prefix: &Path, extra: &[&str]| {
        let mut cmd = Command::new(tebako_bin());
        cmd.arg("press")
            .arg("-r")
            .arg("/nonexistent-tebako-root")
            .arg("-e")
            .arg("x.rb")
            .arg("-p")
            .arg(prefix)
            .args(extra);
        run(&mut cmd)
    };

    // Fresh prefix: "not recognized" + clean_cache (both lines), then 107.
    let work = workdir("vcache-fresh");
    let prefix = work.join("prefix");
    let (code, out) = run_press(&prefix, &[]);
    assert_eq!(code, 107, "{out}");
    assert!(
        out.contains(
            "CMake cache version was not recognized, cleaning up\nCleaning tebako packaging environment\n"
        ),
        "{out}"
    );

    // Stale version: "created by a gem version" + clean_cache.
    let work = workdir("vcache-stale");
    let prefix = work.join("prefix");
    let deps = prefix.join("deps");
    fs::create_dir_all(&deps).unwrap();
    fs::write(deps.join(".environment.version"), "0.0.0 at somewhere").unwrap();
    fs::create_dir_all(prefix.join("o")).unwrap();
    let (code, out) = run_press(&prefix, &[]);
    assert_eq!(code, 107, "{out}");
    assert!(
        out.contains("Tebako cache was created by a gem version 0.0.0 and cannot be used for gem version 0.15.9"),
        "{out}"
    );
    assert!(!deps.exists(), "stale cache must be cleaned");
    assert!(!prefix.join("o").exists(), "stale output must be cleaned");

    // Foreign source: "different source directory" + clean_output only.
    let work = workdir("vcache-foreign");
    let prefix = work.join("prefix");
    let deps = prefix.join("deps");
    fs::create_dir_all(&deps).unwrap();
    fs::write(
        deps.join(".environment.version"),
        "0.15.9 at /some/other/source",
    )
    .unwrap();
    fs::create_dir_all(prefix.join("o")).unwrap();
    let (code, out) = run_press(&prefix, &[]);
    assert_eq!(code, 107, "{out}");
    assert!(
        out.contains(
            "CMake cache was created for a different source directory '/some/other/source'"
        ),
        "{out}"
    );
    assert!(deps.exists(), "deps survive a source mismatch");
    assert!(!prefix.join("o").exists(), "output is cleaned");

    // Own key: silent. Devmode: silent even with a foreign key.
    let work = workdir("vcache-own");
    let prefix = work.join("prefix");
    let deps = prefix.join("deps");
    fs::create_dir_all(&deps).unwrap();
    fs::write(
        deps.join(".environment.version"),
        format!(
            "{} at {}",
            tebako_cli::DEFAULT_TEBAKO_VERSION,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .unwrap();
    let (code, out) = run_press(&prefix, &[]);
    assert_eq!(code, 107, "{out}");
    assert!(!out.contains("CMake cache"), "{out}");

    let (code, out) = run_press(&prefix, &["-D"]);
    assert_eq!(code, 107, "{out}");
    assert!(!out.contains("CMake cache"), "{out}");
}
