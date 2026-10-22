//! The node press backend's staging half: copy the project, install the
//! closure with the RESOLVED node pair, and hand the staged tree to the
//! engine-neutral imaging path.
//!
//! The handoff this module encodes was proven against the shipped pair
//! before it was written: the runtime mounts its own env image through
//! `TEBAKO_RUNTIME_IMAGE`, `--tebako-entry` names `npm-cli.js` INSIDE that
//! mount (mount-relative — a `/__tfs__`-prefixed path is refused), and the
//! vendor step is one spawn with the staged tree as its cwd. Unlike the
//! ruby deploy there is no deploy image and no dispatcher script: the
//! payload's manifest carries the node requirement, and the driver
//! composes `node <entry> <args>` at run time.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{plain_error, TebakoError};

/// The npm entry inside the node runtime's env image (the mount-relative
/// spelling `--tebako-entry` takes).
pub const NPM_CLI: &str = "/lib/node_modules/npm/bin/npm-cli.js";

/// Names never copied into the staged tree: the closure is installed by
/// the vendor step, and the VCS metadata is not payload material.
const SKIP: &[&str] = &["node_modules", ".git", ".gitignore", ".npmrc"];

/// Copy `project` into `stage`, refusing a project whose closure cannot be
/// reproduced: `package-lock.json` is required (a press with no lockfile is
/// a NAMED error naming the missing file — never a floating resolve), and
/// the entry the plan derived must exist in the copy.
pub fn stage_project(project: &Path, entry: &Path, stage: &Path) -> Result<PathBuf, TebakoError> {
    let manifest = project.join("package.json");
    if !manifest.is_file() {
        return Err(plain_error(format!(
            "{} holds no package.json — a node press reads the manifest, the lockfile, and the entry from it",
            project.display()
        )));
    }
    let lock = project.join("package-lock.json");
    if !lock.is_file() {
        return Err(plain_error(format!(
            "{} holds no package-lock.json — the press vendors the closure lockfile-true; commit the lockfile (npm install --package-lock-only) and press again",
            project.display()
        )));
    }
    // The staging tree must live OUTSIDE the project: inside it, the copy
    // would walk the stage it is writing (the real-pair test found this as
    // an ENAMETOOLONG on the first run). Refuse by name instead of
    // silently skipping — a press whose staging dir is inside the payload
    // would leak the stage into itself.
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
    copy_tree(project, stage)?;
    let staged_entry = stage.join(entry);
    if !staged_entry.is_file() {
        return Err(plain_error(format!(
            "the derived entry {} is not a file under {} — pass -e <entry> to name the entry point",
            entry.display(),
            project.display()
        )));
    }
    Ok(staged_entry)
}

/// The canonical spelling of `path` even when it does not exist yet:
/// canonicalize the longest existing ancestor and append the rest. Plain
/// `canonicalize` fails on a missing path, and macOS temp dirs are
/// symlinked (`/var` → `/private/var`), so a lexical join would compare
/// two different spellings of the same place.
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

fn copy_tree(from: &Path, to: &Path) -> Result<(), TebakoError> {
    let rd = std::fs::read_dir(from)
        .map_err(|e| plain_error(format!("{e} reading {}", from.display())))?;
    for item in rd {
        let item = item.map_err(|e| plain_error(format!("{e} reading {}", from.display())))?;
        let name = item.file_name();
        let name_lossy = name.to_string_lossy();
        if SKIP.iter().any(|skip| *skip == name_lossy) {
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
            // A symlink inside the project would resolve against the HOST at
            // press time and against the payload at run time — two different
            // answers. Materialize the target's bytes into the stage.
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

/// Install the closure with the RESOLVED node pair: `npm ci --omit=dev` in
/// `stage`, with the runtime's env image mounted. Never the host's node —
/// the pair the package will ship with is the one that vendors it.
pub fn npm_ci(
    node_exe: &Path,
    runtime_image: &Path,
    stage: &Path,
    verbose: bool,
) -> Result<String, TebakoError> {
    let args: Vec<String> = vec![
        "--tebako-entry".to_string(),
        NPM_CLI.to_string(),
        "ci".to_string(),
        "--omit=dev".to_string(),
        "--no-audit".to_string(),
        "--no-fund".to_string(),
    ];
    let output = Command::new(node_exe)
        .args(&args)
        .current_dir(stage)
        .env("TEBAKO_RUNTIME_IMAGE", runtime_image)
        .output()
        .map_err(|e| {
            plain_error(format!(
                "Failed to run {} (spawn failed: {e}):\n the node pair vendors its own closure",
                node_exe.display()
            ))
        })?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if verbose {
        print!("{text}");
    }
    if !output.status.success() {
        return Err(plain_error(format!(
            "Failed to run npm ci in {} ({}):\n {text}\n the closure is vendored by the resolved node pair; a failing install usually means the lockfile disagrees with package.json",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tebako-node-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fixture(dir: &Path) {
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"hello","version":"1.0.0","bin":{"hello":"cli.js"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("package-lock.json"),
            r#"{"name":"hello","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"hello","version":"1.0.0"}}}"#,
        )
        .unwrap();
        std::fs::write(dir.join("cli.js"), "console.log('hi')\n").unwrap();
    }

    #[test]
    fn staging_copies_the_project_and_skips_the_closure_and_vcs() {
        let project = temp("stage-src");
        fixture(&project);
        std::fs::create_dir_all(project.join("node_modules").join("dep")).unwrap();
        std::fs::write(project.join("node_modules").join("dep").join("i.js"), "\n").unwrap();
        std::fs::create_dir_all(project.join(".git")).unwrap();
        std::fs::write(project.join(".git").join("HEAD"), "\n").unwrap();
        std::fs::create_dir_all(project.join("lib")).unwrap();
        std::fs::write(project.join("lib").join("util.js"), "module.exports = 1;\n").unwrap();

        let stage = temp("stage-dst").join("tree");
        let entry = stage_project(&project, Path::new("cli.js"), &stage).unwrap();
        assert!(entry.ends_with("cli.js") && entry.is_file());
        assert!(stage.join("package-lock.json").is_file());
        assert!(stage.join("lib").join("util.js").is_file());
        assert!(
            !stage.join("node_modules").exists(),
            "the closure is installed, never copied"
        );
        assert!(!stage.join(".git").exists());
    }

    #[test]
    fn a_missing_lockfile_is_a_named_error_naming_the_file() {
        let project = temp("stage-nolock");
        fixture(&project);
        std::fs::remove_file(project.join("package-lock.json")).unwrap();
        let err =
            stage_project(&project, Path::new("cli.js"), &temp("stage-nolock-dst")).unwrap_err();
        assert!(err.message.contains("package-lock.json"), "{}", err.message);
        assert!(err.message.contains("lockfile-true"), "{}", err.message);
    }

    #[test]
    fn a_missing_entry_is_a_named_error() {
        let project = temp("stage-noentry");
        fixture(&project);
        let err =
            stage_project(&project, Path::new("nope.js"), &temp("stage-noentry-dst")).unwrap_err();
        assert!(err.message.contains("-e <entry>"), "{}", err.message);
    }

    #[cfg(unix)]
    #[test]
    fn npm_ci_runs_the_resolved_pair_with_the_proven_argv() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = temp("npm-argv");
        // A stand-in for the node pair: records argv + the runtime image it
        // was handed, then exits 0.
        let fake = dir.join("fake-node");
        std::fs::write(
            &fake,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$PWD/argv.txt\"\nprintf '%s\\n' \"$TEBAKO_RUNTIME_IMAGE\" > \"$PWD/image.txt\"\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&fake).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).unwrap();

        let stage = dir.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        let image = dir.join("runtime.tfs");
        std::fs::write(&image, b"image").unwrap();
        npm_ci(&fake, &image, &stage, false).unwrap();

        let argv = std::fs::read_to_string(stage.join("argv.txt")).unwrap();
        assert_eq!(
            argv,
            "--tebako-entry\n/lib/node_modules/npm/bin/npm-cli.js\nci\n--omit=dev\n--no-audit\n--no-fund\n",
            "{argv}"
        );
        let handed = std::fs::read_to_string(stage.join("image.txt")).unwrap();
        assert_eq!(handed.trim(), image.to_string_lossy());
    }

    /// The same contract against a REAL pair, opt-in: set
    /// `TEBAKO_NODE_PAIR` (the runtime exe) and `TEBAKO_NODE_IMAGE` (its env
    /// image) from an installed store entry, then
    /// `cargo test -p tebako-cli --lib node:: -- --ignored`.
    ///
    /// This is the test the stand-in cannot be: it proves the argv spelling
    /// against the driver that will actually run it (the mount-relative
    /// entry was discovered exactly this way).
    #[test]
    #[ignore = "needs TEBAKO_NODE_PAIR and TEBAKO_NODE_IMAGE (a real installed node pair)"]
    fn a_real_pair_installs_a_zero_dependency_closure() {
        let (Ok(pair), Ok(image)) = (
            std::env::var("TEBAKO_NODE_PAIR"),
            std::env::var("TEBAKO_NODE_IMAGE"),
        ) else {
            panic!("set TEBAKO_NODE_PAIR and TEBAKO_NODE_IMAGE to a real installed node pair");
        };
        let dir = temp("real-pair");
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"hello","version":"1.0.0","bin":{"hello":"cli.js"},"engines":{"node":">=24"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("package-lock.json"),
            r#"{"name":"hello","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"hello","version":"1.0.0"}}}"#,
        )
        .unwrap();
        std::fs::write(dir.join("cli.js"), "console.log('hi')\n").unwrap();
        let stage = temp("real-pair-stage");
        stage_project(&dir, Path::new("cli.js"), &stage).unwrap();
        let out = npm_ci(&PathBuf::from(pair), &PathBuf::from(image), &stage, false).unwrap();
        // An empty closure: npm reports it and creates no node_modules.
        assert!(!out.is_empty(), "npm said nothing");
    }

    #[test]
    fn a_stage_inside_the_project_is_refused_by_name() {
        // The real-pair run found this as an ENAMETOOLONG: a stage under the
        // project makes the copy walk the tree it is writing.
        let project = temp("stage-inside");
        fixture(&project);
        let stage = project.join("out").join("stage");
        let err = stage_project(&project, Path::new("cli.js"), &stage).unwrap_err();
        assert!(
            err.message.contains("sits inside the project"),
            "{}",
            err.message
        );
        assert!(!stage.exists(), "nothing was created");
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_install_names_the_lockfile_disagreement() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = temp("npm-fail");
        let fake = dir.join("fake-node");
        std::fs::write(
            &fake,
            "#!/bin/sh\necho 'npm ERR! lockfile mismatch' >&2\nexit 1\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&fake).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).unwrap();
        let stage = dir.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        let image = dir.join("runtime.tfs");
        std::fs::write(&image, b"image").unwrap();
        let err = npm_ci(&fake, &image, &stage, false).unwrap_err();
        assert!(
            err.message.contains("lockfile disagrees"),
            "{}",
            err.message
        );
        assert!(
            err.message.contains("npm ERR! lockfile mismatch"),
            "{}",
            err.message
        );
    }
}
