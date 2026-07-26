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
    (
        out.status.code().unwrap_or(-1),
        strip_progress(&strip_legacy_warning(&text)),
    )
}

/// Spec 06 §5: fetches print progress on stderr BY DESIGN (phase lines +
/// one quiet cache-hit line even non-TTY). The C++ oracle and the gem's
/// golden outputs predate that contract, so comparisons strip these
/// additive lines (error bodies are untouched — golden parity holds).
/// Same shape as tebako-bootstrap's harness strip; the exact transcript
/// is asserted in tebako-bootstrap's tests/progress.rs.
fn strip_progress(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let is_progress = line.starts_with("resolving ")
            || line.starts_with("downloading ")
            || line.starts_with("installed ")
            || line == "verifying sha256"
            || line == "installing (locked)"
            || (line.starts_with("runtime ") && line.ends_with(" (cached)"));
        if !is_progress {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
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

// ---------------------------------------------------------------------
// item 30b e2e (gated on TEBAKO_IMAGE_FIXTURES = the tebako-runtime-ruby
// runtime-packages dir holding the real interpreter + .tfs/.dwarfs
// images): press against a file:// mirror whose index carries the image
// entry, then a cold first run — the bootstrap resolves interpreter +
// image, the app runs, and the cache holds the image + markers only.
// ---------------------------------------------------------------------

fn image_era_fixture(tag: &str) -> Option<(PathBuf, String, String, String)> {
    let dir = std::env::var("TEBAKO_IMAGE_FIXTURES")
        .ok()
        .map(PathBuf::from)?;
    let plat = tebako_cli::options::host_platform().ok()?;
    let ver = tebako_cli::DEFAULT_TEBAKO_VERSION;
    let ruby = "3.3.7";
    let asset = format!("tebako-runtime-{ver}-{ruby}-{plat}");
    let exe_src = dir.join(&asset);
    let image_name = format!("{asset}.tfs");
    let img_src = [dir.join(&image_name), dir.join(format!("{asset}.dwarfs"))]
        .into_iter()
        .find(|p| p.is_file());
    let Some(img_src) = img_src else {
        eprintln!(
            "skipping {tag}: no runtime image fixture in {}",
            dir.display()
        );
        return None;
    };
    if !exe_src.is_file() {
        eprintln!(
            "skipping {tag}: no runtime executable fixture in {}",
            dir.display()
        );
        return None;
    }

    let work = workdir(tag);
    let mirror_root = work.join("mirror");
    let mirror = mirror_root.join(format!("v{ver}"));
    fs::create_dir_all(&mirror).unwrap();
    fs::copy(&exe_src, mirror.join(&asset)).unwrap();
    fs::copy(&img_src, mirror.join(&image_name)).unwrap();
    let sha_exe = sha256_hex(&mirror.join(&asset));
    let sha_img = sha256_hex(&mirror.join(&image_name));
    fs::write(
        mirror.join("manifest.json"),
        format!(
            "[\n  {{\n    \"tebako_version\": \"{ver}\",\n    \"ruby_version\": \"{ruby}\",\n    \"platform\": \"{plat}\",\n    \"filename\": \"{asset}\",\n    \"sha256\": \"{sha_exe}\",\n    \"size_bytes\": 1,\n    \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{sha_img}\", \"size_bytes\": 1}}\n  }}\n]\n"
        ),
    )
    .unwrap();
    fs::write(
        mirror.join("SHA256SUMS.txt"),
        format!("{sha_exe}  {asset}\n{sha_img}  {image_name}\n"),
    )
    .unwrap();
    Some((
        work,
        mirror_root.to_string_lossy().into_owned(),
        asset,
        image_name,
    ))
}

fn sha256_hex(path: &Path) -> String {
    use sha2::Digest;
    let bytes = fs::read(path).unwrap();
    let digest = sha2::Sha256::digest(&bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn press_against_mirror(
    work: &Path,
    fixture: &str,
    entry: &str,
    package: &Path,
    mirror_root: &str,
) -> (i32, String) {
    let root = work.join("root");
    copy_dir(&fixtures().join(fixture), &root);
    let prefix = work.join("prefix");
    fs::create_dir_all(prefix.join("deps")).unwrap();
    seed_rs_version_file(&prefix);
    let mut cmd = Command::new(tebako_bin());
    cmd.arg("press")
        .arg("-r")
        .arg(&root)
        .arg("-e")
        .arg(entry)
        .arg("-o")
        .arg(package)
        .arg("-p")
        .arg(&prefix)
        .arg("-R")
        .arg("3.3.7")
        .env("TEBAKO_RUNTIME_MIRROR", format!("file://{mirror_root}"))
        .env("TEBAKO_HOME", work.join("home"));
    let sibling = tebako_bin().parent().unwrap().join(if cfg!(windows) {
        "tebako-bootstrap.exe"
    } else {
        "tebako-bootstrap"
    });
    if sibling.is_file() {
        cmd.env("TEBAKO_BOOTSTRAP", sibling);
    }
    run(&mut cmd)
}

#[test]
fn image_era_press_and_cold_run() {
    let _guard = press_lock().lock().unwrap();
    if !e2e_allowed() {
        return;
    }
    let Some((work, mirror_root, asset, image_name)) = image_era_fixture("image-era") else {
        return;
    };
    let home = work.join("home");
    let package = work.join("pkg");

    // Press (simple script): the runtime image is extracted in-process.
    let (code, log) = press_against_mirror(&work, "test-00", "test.rb", &package, &mirror_root);
    assert!(code == 0, "press failed:\n{log}");
    assert!(
        log.contains("extracting the runtime image"),
        "press must seed from the image:\n{log}"
    );

    // The trailer carries the `;image` flag.
    let mut f = fs::File::open(&package).unwrap();
    let manifest = tpkg::read_from(&mut f).unwrap();
    let runtime_ref = manifest.runtime_ref_str().unwrap().to_string();
    assert!(
        runtime_ref.contains(";image"),
        "runtime_ref must carry the ;image flag: {runtime_ref}"
    );

    // The press installed the image into the cache (bootstrap interop).
    let entry_dir = home.join("runtimes").join(format!(
        "ruby-3.3.7-{}-{}",
        tebako_cli::DEFAULT_TEBAKO_VERSION,
        tebako_cli::options::host_platform().unwrap()
    ));
    assert!(entry_dir.join(&asset).is_file(), "interpreter missing");
    assert!(entry_dir.join(&image_name).is_file(), "image missing");
    assert!(
        entry_dir.join(format!("{image_name}.sha256")).is_file(),
        "trusted marker missing"
    );
    assert!(
        !entry_dir.join("layout").exists(),
        "no extracted tree in the cache"
    );

    // Cold run: wipe the cache — the bootstrap downloads interpreter +
    // image from the mirror and the app runs.
    fs::remove_dir_all(&home).unwrap();
    let mut cold = Command::new(&package);
    cold.env("TEBAKO_RUNTIME_MIRROR", format!("file://{mirror_root}"))
        .env("TEBAKO_HOME", &home);
    let (code, out) = run(&mut cold);
    assert_eq!(code, 0, "cold run failed:\n{out}");
    assert!(
        out.contains("Hello!  This is test-00 talking from inside DwarFS"),
        "{out}"
    );

    // The cache holds interpreter + immutable image + markers — nothing else.
    assert!(entry_dir.join(&asset).is_file());
    let image = entry_dir.join(&image_name);
    assert!(image.is_file());
    assert!(entry_dir.join(format!("{image_name}.sha256")).is_file());
    assert!(entry_dir.join(format!("{image_name}.origin")).is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = fs::metadata(&image).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o444, "image must be read-only: {mode:o}");
    }
    assert!(
        !entry_dir.join("layout").exists(),
        "no extracted tree in the cache"
    );

    // The runtime executes a stub from the standalone image (the driver
    // mounts it, zero driver change): wrap the image as a tpkg package
    // and hand it over via the v1 --tebako-image mechanism.
    let base = work.join("empty.base");
    fs::write(&base, b"").unwrap();
    let wrapped = work.join("wrapped-runtime.pkg");
    tebako_pkg::bundle_exact(
        &base,
        &[tebako_pkg::PackageImage {
            path: entry_dir.join(&image_name),
            mount_point: "/__tebako_memfs__".to_string(),
            format_id: tpkg::TPKG_FORMAT_DWARFS,
        }],
        &wrapped,
        &tebako_pkg::PackageOptions {
            runtime_ref: runtime_ref.clone(),
            package_flags: tpkg::TPKG_FLAG_LEAN,
            launcher_abi: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut wrapped_run = Command::new(entry_dir.join(&asset));
    wrapped_run
        .arg("--tebako-image")
        .arg(format!("{}:0:/__tebako_memfs__", wrapped.display()));
    let (code, out) = run(&mut wrapped_run);
    assert_eq!(code, 0, "standalone image mount failed:\n{out}");
    assert!(
        out.contains("Tebako runtime stub"),
        "the runtime image's own stub must execute from the mount:\n{out}"
    );
}

/// Build an image-era mirror from the OFFICIAL released runtime: extract
/// its own layout (v1 mechanism, used only to manufacture the fixture)
/// and image it in-process (the same writer 30a's pipeline uses) — a
/// build-matched executable+image pair.
fn official_pair_fixture(tag: &str) -> Option<(PathBuf, String, String, String)> {
    let plat = tebako_cli::options::host_platform().ok()?;
    let ver = tebako_cli::DEFAULT_TEBAKO_VERSION;
    let ruby = "3.3.7";
    let asset = format!("tebako-runtime-{ver}-{ruby}-{plat}");
    let image_name = format!("{asset}.tfs");

    let work = workdir(tag);
    let mirror_root = work.join("mirror");
    let mirror = mirror_root.join(format!("v{ver}"));
    fs::create_dir_all(&mirror).unwrap();

    // Resolve the official runtime through the CLI itself (live download
    // or the shared cache), then extract its layout for imaging.
    let home = work.join("resolve-home");
    let resolver = tebako_cli::resolve::Resolver::new(tebako_cli::resolve::Flavor::Runtime);
    let runtime = resolver.resolve(ruby, &plat, ver).ok()?;
    fs::copy(&runtime, mirror.join(&asset)).unwrap();
    let layout = work.join("layout-src");
    let mut extract = Command::new(&runtime);
    extract
        .arg("--tebako-extract")
        .arg(&layout)
        .env("TEBAKO_HOME", &home);
    let (code, out) = run(&mut extract);
    assert_eq!(code, 0, "layout extraction failed:\n{out}");

    // Image the layout in-process (dwarfs-t native, the 30a format).
    let image_out = mirror.join(&image_name);
    let mut writer = dwarfs_t::Writer::new(dwarfs_t::WriterOptions::default()).unwrap();
    writer.add_tree(&layout, "/").unwrap();
    writer.write(&image_out).unwrap();

    let sha_exe = sha256_hex(&mirror.join(&asset));
    let sha_img = sha256_hex(&image_out);
    fs::write(
        mirror.join("manifest.json"),
        format!(
            "[\n  {{\n    \"tebako_version\": \"{ver}\",\n    \"ruby_version\": \"{ruby}\",\n    \"platform\": \"{plat}\",\n    \"filename\": \"{asset}\",\n    \"sha256\": \"{sha_exe}\",\n    \"size_bytes\": 1,\n    \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{sha_img}\", \"size_bytes\": 1}}\n  }}\n]\n"
        ),
    )
    .unwrap();
    fs::write(
        mirror.join("SHA256SUMS.txt"),
        format!("{sha_exe}  {asset}\n{sha_img}  {image_name}\n"),
    )
    .unwrap();
    Some((
        work,
        mirror_root.to_string_lossy().into_owned(),
        asset,
        image_name,
    ))
}

#[test]
fn image_era_full_flow_official_pair() {
    let _guard = press_lock().lock().unwrap();
    if !e2e_allowed() {
        return;
    }
    let Some((work, mirror_root, _asset, image_name)) = official_pair_fixture("image-era-official")
    else {
        eprintln!("skipping official-pair image-era e2e: runtime resolution failed");
        return;
    };
    let home = work.join("home");
    let entry_dir = home.join("runtimes").join(format!(
        "ruby-3.3.7-{}-{}",
        tebako_cli::DEFAULT_TEBAKO_VERSION,
        tebako_cli::options::host_platform().unwrap()
    ));

    for (fixture, entry, expect) in [
        (
            "test-00",
            "test.rb",
            "Hello!  This is test-00 talking from inside DwarFS",
        ),
        (
            "gemfile-app",
            "main.rb",
            "Hello from gemfile app with rake ",
        ),
    ] {
        let package = work.join(format!("pkg-{fixture}"));
        let (code, log) = press_against_mirror(&work, fixture, entry, &package, &mirror_root);
        assert!(code == 0, "{fixture} press failed:\n{log}");

        // Cold run: wipe the cache; the bootstrap resolves interpreter +
        // image from the mirror and the app runs.
        fs::remove_dir_all(&home).unwrap();
        let mut cold = Command::new(&package);
        cold.env("TEBAKO_RUNTIME_MIRROR", format!("file://{mirror_root}"))
            .env("TEBAKO_HOME", &home);
        let (code, out) = run(&mut cold);
        assert_eq!(code, 0, "{fixture} cold run failed:\n{out}");
        assert!(out.contains(expect), "{fixture}: {out}");
        assert!(
            entry_dir.join(&image_name).is_file(),
            "{fixture}: image missing"
        );
        assert!(
            !entry_dir.join("layout").exists(),
            "{fixture}: no extracted tree"
        );
    }
}
