//! Project detection for the zero-ceremony press (the author-tier
//! friction floor): with no `-r`/`-e`, `tebako press` reads the project
//! in the current directory and derives the root, the entry point and the
//! engine requirement from what the project IS, rather than making the
//! author restate it as flags.
//!
//! The detector is deliberately strict and total: every outcome is either
//! a complete plan or a named error that lists what was looked for and how
//! to say it explicitly. A guessed entry point would be worse than no
//! detection at all — the author would learn about the guess as a broken
//! package.

use std::path::{Path, PathBuf};

/// What the project is — the engine axis the press will target. Today the
/// press engine is the Ruby scenario machinery; the other project kinds
/// are recognized so the refusal can name them instead of calling an
/// unknown directory empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Ruby,
}

impl ProjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectKind::Ruby => "ruby",
        }
    }
}

/// A detected project: the press root (absolute, as given), the entry
/// point relative to it (the spelling `--entry-point` takes), and what the
/// project is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPlan {
    pub root: PathBuf,
    pub entry: PathBuf,
    pub kind: ProjectKind,
}

/// The marker files probed, in order. The order IS the priority: a
/// directory carrying both a Gemfile and a package.json is a Ruby project
/// with node tooling, which is how the Ruby press reads it today.
const MARKERS: &[(&str, ProjectKind)] = &[("Gemfile", ProjectKind::Ruby)];

/// Detect the project rooted at `dir`. `Err` carries the complete named
/// message (the caller renders it as the usage refusal).
pub fn detect(dir: &Path) -> Result<ProjectPlan, String> {
    let dir = if dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        dir.to_path_buf()
    };
    let marker = MARKERS.iter().find(|(name, _)| dir.join(name).is_file());
    let (marker_name, kind) = match marker {
        Some((name, kind)) => (*name, *kind),
        None => {
            // Recognized-but-unsupported projects get their own sentence:
            // an author whose python project is refused should not be told
            // the directory is empty.
            for (name, engine) in [
                ("package.json", "node"),
                ("pyproject.toml", "python"),
                ("Cargo.toml", "rust"),
            ] {
                if dir.join(name).is_file() {
                    return Err(format!(
                        "this directory is a {engine} project ({name}): the {engine} press backend is not available in this release — the project-kind detection ships for ruby first; press a ruby project, or see the feedstock model in the guides for other engines"
                    ));
                }
            }
            return Err(format!(
                "no project detected in {} — looked for {}; press with -r <root> -e <entry> to state the root and entry point explicitly",
                dir.display(),
                MARKERS
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    };
    let entry = ruby_entry(&dir).map_err(|why| {
        format!(
            "{} is a {} project (marker: {marker_name}) but {why} — pass -e <entry> to name the entry point (relative to {}), or keep the executable under bin/ or exe/",
            dir.display(),
            kind.as_str(),
            dir.display()
        )
    })?;
    Ok(ProjectPlan {
        root: dir,
        entry,
        kind,
    })
}

/// The entry point of a Ruby project: the single executable script under
/// `bin/` then `exe/` (the gem layout), else the single `*.rb` file at the
/// root (the script layout). Anything ambiguous is refused by name.
fn ruby_entry(dir: &Path) -> Result<PathBuf, String> {
    for layout in ["bin", "exe"] {
        let candidates = executables(&dir.join(layout));
        match candidates.len() {
            1 => return Ok(PathBuf::from(layout).join(&candidates[0])),
            0 => {}
            _ => {
                return Err(format!(
                    "{layout}/ holds {} executable scripts ({}); the entry point is ambiguous",
                    candidates.len(),
                    candidates.join(", ")
                ))
            }
        }
    }
    let root_scripts = root_ruby_scripts(dir);
    match root_scripts.len() {
        1 => Ok(root_scripts[0].clone()),
        0 => {
            Err("no executable script under bin/ or exe/ and no *.rb file at its root".to_string())
        }
        _ => Err(format!(
            "its root holds {} Ruby scripts ({}); the entry point is ambiguous",
            root_scripts.len(),
            root_scripts
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Executable, regular files under `dir` whose first line is a Ruby
/// shebang (`#!/usr/bin/env ruby`, `#!/usr/bin/ruby`, `#!/.../ruby3.3`).
fn executables(dir: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .flatten()
        .filter(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && is_executable(&e.path())
                && ruby_shebang(&e.path())
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// `*.rb` files at the project root (not recursive), sorted.
fn root_ruby_scripts(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .flatten()
        .filter(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && e.file_name().to_string_lossy().ends_with(".rb")
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names.into_iter().map(PathBuf::from).collect()
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// On windows the executable bit does not exist; the shebang is the
/// signal (a Ruby script's first line is its contract).
#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

/// Does the file's first line name `ruby` as the interpreter?
fn ruby_shebang(path: &Path) -> bool {
    use std::io::Read as _;
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 128];
    let Ok(n) = f.read(&mut head) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(&head[..n]) else {
        return false;
    };
    let Some(first) = text.lines().next() else {
        return false;
    };
    first.starts_with("#!") && first.contains("ruby")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tebako-detect-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn script(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, "#!/usr/bin/env ruby\nputs :ok\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    fn a_gem_layout_entry_is_detected() {
        let dir = temp("gem");
        std::fs::write(dir.join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();
        script(&dir.join("exe").join("tool"));
        let plan = detect(&dir).unwrap();
        assert_eq!(plan.kind, ProjectKind::Ruby);
        assert_eq!(plan.entry, PathBuf::from("exe/tool"));
        assert_eq!(plan.root, dir);
    }

    #[test]
    fn bin_wins_over_exe_and_a_single_root_script_is_the_fallback() {
        let dir = temp("bin");
        std::fs::write(dir.join("Gemfile"), "").unwrap();
        script(&dir.join("bin").join("cli"));
        assert_eq!(detect(&dir).unwrap().entry, PathBuf::from("bin/cli"));

        let script_dir = temp("script");
        std::fs::write(script_dir.join("Gemfile"), "").unwrap();
        script(&script_dir.join("app.rb"));
        assert_eq!(detect(&script_dir).unwrap().entry, PathBuf::from("app.rb"));
    }

    #[test]
    fn ambiguous_entries_are_refused_by_name() {
        let dir = temp("ambiguous");
        std::fs::write(dir.join("Gemfile"), "").unwrap();
        script(&dir.join("bin").join("one"));
        script(&dir.join("bin").join("two"));
        let err = detect(&dir).unwrap_err();
        assert!(err.contains("one") && err.contains("two"), "{err}");
        assert!(err.contains("-e <entry>"), "{err}");

        let root = temp("ambiguous-root");
        std::fs::write(root.join("Gemfile"), "").unwrap();
        script(&root.join("a.rb"));
        script(&root.join("b.rb"));
        assert!(detect(&root).unwrap_err().contains("a.rb"));
    }

    #[test]
    fn a_non_ruby_project_names_its_engine_and_the_current_limit() {
        let dir = temp("node");
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        let err = detect(&dir).unwrap_err();
        assert!(
            err.contains("node") && err.contains("package.json"),
            "{err}"
        );
        assert!(!err.contains("no project detected"), "{err}");

        let py = temp("python");
        std::fs::write(py.join("pyproject.toml"), "").unwrap();
        assert!(detect(&py).unwrap_err().contains("python"));
    }

    #[test]
    fn an_empty_directory_names_what_was_looked_for() {
        let dir = temp("empty");
        let err = detect(&dir).unwrap_err();
        assert!(
            err.contains("Gemfile") && err.contains("-r <root> -e <entry>"),
            "{err}"
        );
    }
}
