//! The python press backend's staging half: copy the project, vendor the
//! closure with the RESOLVED python pair, flatten the console script to
//! the payload root, and hand the staged tree to the engine-neutral
//! imaging path.
//!
//! The handoff this module encodes was proven against the shipped pair
//! before it was written (the arc-02 record, 2026-11-11): the runtime
//! mounts its own env image through `TEBAKO_RUNTIME_IMAGE`,
//! `--tebako-entry` names `pip3` INSIDE that mount (mount-relative), the
//! vendor step is two spawns with the staged tree as its cwd — the pinned
//! `requirements.txt` closure first, then the project itself `--no-deps`
//! (its PEP 621 ranges are trusted for the project's own code only) — and
//! the console script pip generates lands at `<stage>/bin/<name>` and
//! moves to the payload ROOT: python puts the script's directory on
//! `sys.path[0]`, so a script beside the packages imports them at any
//! mount point with no PYTHONPATH and no dispatcher script (the MOUNT
//! RULE makes a payload-authored PYTHONPATH wrong by construction — the
//! consumer owns the mount).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{plain_error, TebakoError};

/// The pip entry inside the python runtime's env image (the mount-relative
/// spelling `--tebako-entry` takes). The factory stages pip's console
/// scripts at the runtime root's `bin/`.
pub const PIP_CLI: &str = "/bin/pip3";

/// Names never copied into the staged tree: the closure is installed by
/// the vendor step, the VCS metadata and virtualenvs are not payload
/// material, and `.env` is a secrets file that must never bake into an
/// image (node's `.npmrc` precedent).
const SKIP: &[&str] = &[
    "__pycache__",
    ".git",
    ".gitignore",
    ".venv",
    "venv",
    ".env",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
];

/// Names skipped by suffix: build metadata directories.
const SKIP_SUFFIX: &[&str] = &[".egg-info"];

/// The press's FIRST python gate: the closure is vendored lockfile-true,
/// so a project without `requirements.txt` is refused by name — before
/// any runtime is resolved or downloaded. (The node's
/// `package-lock.json` precedent; a pinned requirements file is pip's
/// lockfile-true spelling.)
pub fn require_lockfile(project: &Path) -> Result<(), TebakoError> {
    if project.join("requirements.txt").is_file() {
        return Ok(());
    }
    Err(plain_error(format!(
        "{} holds no requirements.txt — the press vendors the closure lockfile-true; commit the pinned requirements (pip-compile / uv pip compile) and press again",
        project.display()
    )))
}

/// Copy `project` into `stage`: `pyproject.toml` and the requirements
/// gate first (a press with no lockfile is a NAMED error naming the
/// missing file — never a floating resolve). The console script the plan
/// derived does not exist before the vendor step (pip generates it), so
/// the entry check is the post-vendor assertion in [`pip_install`],
/// not a file probe here.
pub fn stage_project(project: &Path, stage: &Path) -> Result<(), TebakoError> {
    if !project.join("pyproject.toml").is_file() {
        return Err(plain_error(format!(
            "{} holds no pyproject.toml — a python press reads the manifest, the requirements, and the entry from it",
            project.display()
        )));
    }
    require_lockfile(project)?;
    // The staging tree must live OUTSIDE the project (the node arm's
    // ENAMETOOLONG lesson): inside it, the copy would walk the stage it
    // is writing.
    if let (Ok(project_abs), Some(stage_abs)) =
        (std::fs::canonicalize(project), canonical_prefix(stage))
    {
        if stage_abs.starts_with(&project_abs) {
            return Err(plain_error(format!(
                "the staging tree {} sits inside the project {} — stage outside the project so the payload cannot copy itself",
                stage.display(),
                project.display()
            )));
        }
    }
    if stage.exists() {
        std::fs::remove_dir_all(stage).map_err(|e| {
            plain_error(format!("{e} clearing the staging tree {}", stage.display()))
        })?;
    }
    std::fs::create_dir_all(stage)
        .map_err(|e| plain_error(format!("{e} creating {}", stage.display())))?;
    copy_tree(project, stage)
}

/// The canonical spelling of `path` even when it does not exist yet (the
/// node arm's helper, duplicated so the two backends stay independent
/// reads): canonicalize the longest existing ancestor and append the
/// rest.
fn canonical_prefix(path: &Path) -> Option<PathBuf> {
    let mut existing = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(&existing) {
            let mut out = canonical;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return Some(out);
        }
        tail.push(existing.file_name()?.to_os_string());
        existing = existing.parent()?.to_path_buf();
    }
}

fn skipped(name: &str) -> bool {
    SKIP.contains(&name) || SKIP_SUFFIX.iter().any(|s| name.ends_with(s))
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), TebakoError> {
    let rd = std::fs::read_dir(from)
        .map_err(|e| plain_error(format!("{e} reading {}", from.display())))?;
    for item in rd {
        let item = item.map_err(|e| plain_error(format!("{e} reading {}", from.display())))?;
        let name = item.file_name();
        let name_lossy = name.to_string_lossy();
        if skipped(&name_lossy) {
            continue;
        }
        let src = item.path();
        let dst = to.join(&name);
        let file_type = item
            .file_type()
            .map_err(|e| plain_error(format!("{e} reading {}", src.display())))?;
        if file_type.is_dir() {
            std::fs::create_dir_all(&dst)
                .map_err(|e| plain_error(format!("{e} creating {}", dst.display())))?;
            copy_tree(&src, &dst)?;
        } else if file_type.is_symlink() {
            // A symlink resolves against the HOST at press time and
            // against the payload at run time — two different answers.
            // Materialize the target's bytes into the stage.
            let target = std::fs::read_link(&src)
                .map_err(|e| plain_error(format!("{e} reading {}", src.display())))?;
            let resolved = from.join(&target);
            if resolved.is_dir() {
                std::fs::create_dir_all(&dst)
                    .map_err(|e| plain_error(format!("{e} creating {}", dst.display())))?;
                copy_tree(&resolved, &dst)?;
            } else {
                std::fs::copy(&resolved, &dst).map_err(|e| {
                    plain_error(format!(
                        "{e} copying {} (symlink target) to {}",
                        resolved.display(),
                        dst.display()
                    ))
                })?;
            }
        } else {
            std::fs::copy(&src, &dst).map_err(|e| {
                plain_error(format!(
                    "{e} copying {} to {}",
                    src.display(),
                    dst.display()
                ))
            })?;
        }
    }
    Ok(())
}

/// One pip spawn through the resolved pair; the joined output on success.
fn pip(
    py_exe: &Path,
    runtime_image: &Path,
    stage: &Path,
    trailing: &[&str],
    verbose: bool,
) -> Result<String, TebakoError> {
    let mut args: Vec<String> = vec!["--tebako-entry".to_string(), PIP_CLI.to_string()];
    args.extend(trailing.iter().map(|s| s.to_string()));
    let output = Command::new(py_exe)
        .args(&args)
        .current_dir(stage)
        .env("TEBAKO_RUNTIME_IMAGE", runtime_image)
        .output()
        .map_err(|e| {
            plain_error(format!(
                "Failed to run {} (spawn failed: {e}):\n the python pair vendors its own closure",
                py_exe.display()
            ))
        })?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if verbose {
        print!("{text}");
    }
    if !output.status.success() {
        return Err(plain_error(format!(
            "Failed to run pip in {} ({}):\n {text}\n the closure is vendored by the resolved python pair; a failing install usually means requirements.txt disagrees with pyproject.toml, or a dependency ships no wheel for this platform (the press vendors wheels only — a source build is the author's own compilation)",
            stage.display(),
            output
                .status
                .code()
                .map(|c| format!("exit status {c}"))
                .unwrap_or_else(|| "killed by a signal".to_string())
        )));
    }
    Ok(text)
}

/// Install the closure with the RESOLVED python pair — never the host's
/// python: the pair the package ships with is the one that vendors it.
/// Two spawns (the lockfile-true shape): the pinned `requirements.txt`
/// closure wheel-only, then the project itself `--no-deps`. Afterwards the
/// console script pip generated at `<stage>/bin/<entry>` moves to the
/// payload ROOT so the run resolves imports through `sys.path[0]` alone —
/// the flat layout the handoff proof verified mount-independent.
pub fn pip_install(
    py_exe: &Path,
    runtime_image: &Path,
    stage: &Path,
    entry: &str,
    verbose: bool,
) -> Result<String, TebakoError> {
    let mut text = pip(
        py_exe,
        runtime_image,
        stage,
        &[
            "install",
            "--target",
            ".",
            "-r",
            "requirements.txt",
            "--only-binary",
            ":all:",
            "--no-cache-dir",
            "--root-user-action=ignore",
        ],
        verbose,
    )?;
    text.push_str(&pip(
        py_exe,
        runtime_image,
        stage,
        &[
            "install",
            "--target",
            ".",
            "--no-deps",
            "--only-binary",
            ":all:",
            "--no-cache-dir",
            "--root-user-action=ignore",
            ".",
        ],
        verbose,
    )?);
    // The flatten: pip's --target scheme puts console scripts in `bin/`;
    // the payload carries them at its root instead (python adds the
    // script's directory to sys.path[0], and the packages sit beside it).
    let bin = stage.join("bin");
    let generated = bin.join(entry);
    if !generated.is_file() {
        return Err(plain_error(format!(
            "pip generated no bin/{entry} in {} — the [project.scripts] entry must name a console script the project build produces",
            stage.display()
        )));
    }
    let rd = std::fs::read_dir(&bin)
        .map_err(|e| plain_error(format!("{e} reading {}", bin.display())))?;
    for item in rd {
        let item = item.map_err(|e| plain_error(format!("{e} reading {}", bin.display())))?;
        let dst = stage.join(item.file_name());
        if dst.exists() {
            return Err(plain_error(format!(
                "the flatten cannot move {} to the payload root — {} already exists there (a package and a console script share a name)",
                item.path().display(),
                dst.display()
            )));
        }
        std::fs::rename(item.path(), &dst).map_err(|e| {
            plain_error(format!(
                "{e} moving {} to the payload root",
                item.path().display()
            ))
        })?;
    }
    std::fs::remove_dir(&bin)
        .map_err(|e| plain_error(format!("{e} removing {}", bin.display())))?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tebako-python-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fixture(dir: &Path) {
        std::fs::write(
            dir.join("pyproject.toml"),
            "[project]\nname = \"hello-py\"\nversion = \"0.1.0\"\n\n[project.scripts]\nhello-py = \"hello_py.cli:main\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("requirements.txt"), "isodate==0.7.2\n").unwrap();
        std::fs::create_dir_all(dir.join("src").join("hello_py")).unwrap();
        std::fs::write(
            dir.join("src").join("hello_py").join("cli.py"),
            "def main():\n    print('hi')\n",
        )
        .unwrap();
    }

    #[test]
    fn staging_copies_the_project_and_skips_caches_vcs_and_secrets() {
        let project = temp("stage-src");
        fixture(&project);
        std::fs::create_dir_all(project.join("__pycache__")).unwrap();
        std::fs::write(project.join("__pycache__").join("c.pyc"), "\n").unwrap();
        std::fs::create_dir_all(project.join(".git")).unwrap();
        std::fs::write(project.join(".env"), "SECRET=1\n").unwrap();
        std::fs::create_dir_all(project.join("hello_py.egg-info")).unwrap();
        std::fs::write(project.join("hello_py.egg-info").join("PKG-INFO"), "\n").unwrap();

        let stage = temp("stage-dst").join("tree");
        stage_project(&project, &stage).unwrap();
        assert!(stage.join("pyproject.toml").is_file());
        assert!(stage.join("requirements.txt").is_file());
        assert!(stage.join("src").join("hello_py").join("cli.py").is_file());
        assert!(!stage.join("__pycache__").exists());
        assert!(!stage.join(".git").exists());
        assert!(
            !stage.join(".env").exists(),
            "a secrets file never bakes into a payload"
        );
        assert!(!stage.join("hello_py.egg-info").exists());
    }

    #[test]
    fn a_missing_requirements_is_a_named_error_naming_the_file() {
        let project = temp("stage-nolock");
        fixture(&project);
        std::fs::remove_file(project.join("requirements.txt")).unwrap();
        let err = stage_project(&project, &temp("stage-nolock-dst")).unwrap_err();
        assert!(err.message.contains("requirements.txt"), "{}", err.message);
        assert!(err.message.contains("lockfile-true"), "{}", err.message);
    }

    #[test]
    fn a_stage_inside_the_project_is_refused_by_name() {
        let project = temp("stage-inside");
        fixture(&project);
        let stage = project.join("out").join("stage");
        let err = stage_project(&project, &stage).unwrap_err();
        assert!(
            err.message.contains("sits inside the project"),
            "{}",
            err.message
        );
        assert!(!stage.exists(), "nothing was created");
    }

    #[cfg(unix)]
    #[test]
    fn pip_install_runs_the_resolved_pair_with_the_proven_argv() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = temp("pip-argv");
        // A stand-in for the python pair: records each argv + the runtime
        // image it was handed, generates the console script on the second
        // (project) invocation, then exits 0.
        let fake = dir.join("fake-python");
        std::fs::write(
            &fake,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$PWD/argv.txt\"\nprintf '%s\\n' \"$TEBAKO_RUNTIME_IMAGE\" >> \"$PWD/image.txt\"\ncase \"$*\" in\n  *\" .\") mkdir -p bin && printf '#!python\\n' > bin/hello-py;;\nesac\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&fake).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).unwrap();

        let stage = dir.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        let image = dir.join("runtime.tfs");
        std::fs::write(&image, b"image").unwrap();
        pip_install(&fake, &image, &stage, "hello-py", false).unwrap();

        let argv = std::fs::read_to_string(stage.join("argv.txt")).unwrap();
        assert_eq!(
            argv,
            "--tebako-entry\n/bin/pip3\ninstall\n--target\n.\n-r\nrequirements.txt\n--only-binary\n:all:\n--no-cache-dir\n--root-user-action=ignore\n--tebako-entry\n/bin/pip3\ninstall\n--target\n.\n--no-deps\n--only-binary\n:all:\n--no-cache-dir\n--root-user-action=ignore\n.\n",
            "{argv}"
        );
        let handed = std::fs::read_to_string(stage.join("image.txt")).unwrap();
        assert_eq!(
            handed.trim(),
            format!("{0}\n{0}", image.to_string_lossy()),
            "both spawns mount the resolved pair's env image"
        );
        // The flatten moved the console script to the payload root.
        assert!(stage.join("hello-py").is_file());
        assert!(!stage.join("bin").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_missing_console_script_is_a_named_error() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = temp("pip-noentry");
        let fake = dir.join("fake-python");
        std::fs::write(&fake, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&fake).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).unwrap();
        let stage = dir.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        let image = dir.join("runtime.tfs");
        std::fs::write(&image, b"image").unwrap();
        let err = pip_install(&fake, &image, &stage, "hello-py", false).unwrap_err();
        assert!(
            err.message.contains("pip generated no bin/hello-py"),
            "{}",
            err.message
        );
        assert!(err.message.contains("[project.scripts]"), "{}", err.message);
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_install_names_the_wheel_only_contract() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = temp("pip-fail");
        let fake = dir.join("fake-python");
        std::fs::write(
            &fake,
            "#!/bin/sh\necho 'ERROR: No matching distribution' >&2\nexit 1\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&fake).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).unwrap();
        let stage = dir.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        let image = dir.join("runtime.tfs");
        std::fs::write(&image, b"image").unwrap();
        let err = pip_install(&fake, &image, &stage, "hello-py", false).unwrap_err();
        assert!(err.message.contains("wheels only"), "{}", err.message);
        assert!(
            err.message.contains("No matching distribution"),
            "{}",
            err.message
        );
    }

    /// The same contract against a REAL pair, opt-in: set
    /// `TEBAKO_PYTHON_PAIR` (the runtime exe) and `TEBAKO_PYTHON_IMAGE`
    /// (its env image) from an installed store entry, then
    /// `cargo test -p tebako-cli --lib python:: -- --ignored`.
    #[test]
    #[ignore = "needs TEBAKO_PYTHON_PAIR and TEBAKO_PYTHON_IMAGE (a real installed python pair)"]
    fn a_real_pair_vendors_a_one_dependency_closure() {
        let (Ok(pair), Ok(image)) = (
            std::env::var("TEBAKO_PYTHON_PAIR"),
            std::env::var("TEBAKO_PYTHON_IMAGE"),
        ) else {
            panic!(
                "set TEBAKO_PYTHON_PAIR and TEBAKO_PYTHON_IMAGE to a real installed python pair"
            );
        };
        let dir = temp("real-pair");
        fixture(&dir);
        let stage = temp("real-pair-stage");
        stage_project(&dir, &stage).unwrap();
        let out = pip_install(
            &PathBuf::from(pair),
            &PathBuf::from(image),
            &stage,
            "hello-py",
            false,
        )
        .unwrap();
        assert!(!out.is_empty(), "pip said nothing");
        assert!(stage.join("hello-py").is_file());
        assert!(
            stage.join("isodate").is_dir(),
            "the pinned dependency is vendored"
        );
    }
}
