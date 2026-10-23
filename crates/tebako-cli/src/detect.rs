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

/// What the project is — the engine axis the press targets. Ruby is the
/// engine the press backend implements today; Node's plan is DERIVED (so
/// the refusal can name the entry and the requirement it read) while its
/// vendoring backend is still to come; python and rust are recognized so
/// the refusal names them instead of calling the directory empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Ruby,
    Node,
}

impl ProjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectKind::Ruby => "ruby",
            ProjectKind::Node => "node",
        }
    }
}

/// A detected project: the press root (absolute, as given), the entry
/// point relative to it (the spelling `--entry-point` takes), what the
/// project is, and — where the engine's manifest declares one — the
/// engine requirement the runtime must satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPlan {
    pub root: PathBuf,
    pub entry: PathBuf,
    pub kind: ProjectKind,
    pub requirement: Option<String>,
}

/// The marker files probed for the RUBY layout, in order. A directory
/// carrying both a Gemfile and a package.json is a Ruby project with node
/// tooling, which is how the Ruby press reads it today.
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
            // A node project's plan IS derivable (package.json carries the
            // entry and the engine requirement), so it is derived here and
            // refused at the BACKEND, where the message can name what was
            // read. The kinds whose mainfest the press cannot yet read at
            // all keep the recognized-but-unsupported sentence: an author
            // whose python project is refused should not be told the
            // directory is empty.
            if dir.join("package.json").is_file() {
                let (entry, requirement) = node_plan(&dir)?;
                return Ok(ProjectPlan {
                    root: dir,
                    entry,
                    kind: ProjectKind::Node,
                    requirement,
                });
            }
            for (name, engine) in [("pyproject.toml", "python"), ("Cargo.toml", "rust")] {
                if dir.join(name).is_file() {
                    return Err(format!(
                        "this directory is a {engine} project ({name}): the {engine} press backend is not available in this release — the project-kind detection ships for ruby first; press a ruby project, or see the feedstock model in the guides for other engines"
                    ));
                }
            }
            // The usage line promises "the single root *.rb": a directory
            // with no marker but exactly one executable script (bin/, exe/,
            // or one root *.rb) IS a ruby project. Refusing it made the
            // shortest stranger path — write main.rb, press it — fail with a
            // message that contradicted our own help (arc 02's ten-minute
            // walkthrough found it on the first command).
            if let Ok(entry) = ruby_entry(&dir) {
                return Ok(ProjectPlan {
                    root: dir,
                    entry,
                    kind: ProjectKind::Ruby,
                    requirement: None,
                });
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
        requirement: None,
    })
}

/// The entry point and engine requirement of a node project, read from
/// `package.json`: `bin` (the string form, or the map's entry — the one
/// named like the package wins, else a single key, else the entry is
/// ambiguous), else `main`, else `index.js` when it exists. `engines.node`
/// is the requirement the runtime must satisfy. Anything unreadable or
/// ambiguous is refused by name — a guessed entry is a broken package the
/// author only meets later.
fn node_plan(dir: &Path) -> Result<(PathBuf, Option<String>), String> {
    let path = dir.join("package.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("its package.json cannot be read: {e}"))?;
    let parsed =
        tebako_json::parse(&text).map_err(|e| format!("its package.json does not parse: {e}"))?;
    let requirement = parsed
        .find("engines")
        .and_then(|e| e.find("node"))
        .and_then(|v| v.as_string());
    let package_name = parsed.find("name").and_then(|v| v.as_string());

    if let Some(bin) = parsed.find("bin") {
        match bin {
            tebako_json::Value::String(s) => return Ok((PathBuf::from(s), requirement)),
            tebako_json::Value::Object(entries) => {
                if let Some((_, value)) = package_name
                    .as_ref()
                    .and_then(|name| entries.iter().find(|(k, _)| k == name))
                {
                    if let Some(spelled) = value.as_string() {
                        return Ok((PathBuf::from(spelled), requirement));
                    }
                }
                match entries.as_slice() {
                    [(_, value)] => {
                        if let Some(spelled) = value.as_string() {
                            return Ok((PathBuf::from(spelled), requirement));
                        }
                    }
                    [] => {}
                    _ => {
                        return Err(format!(
                            "its package.json declares several bin commands ({}) and none is named like the package — pass -e <entry> to name the entry point",
                            entries
                                .iter()
                                .map(|(k, _)| k.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(main) = parsed.find("main").and_then(|v| v.as_string()) {
        return Ok((PathBuf::from(main), requirement));
    }
    if dir.join("index.js").is_file() {
        return Ok((PathBuf::from("index.js"), requirement));
    }
    Err("its package.json declares no bin/main script and the root holds no index.js".to_string())
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
    fn a_bare_root_script_is_detected_without_a_marker() {
        // The usage line's own promise: "the single root *.rb".
        let dir = temp("bare");
        std::fs::write(dir.join("main.rb"), "puts :ok\n").unwrap();
        let plan = detect(&dir).unwrap();
        assert_eq!(plan.kind, ProjectKind::Ruby);
        assert_eq!(plan.entry, PathBuf::from("main.rb"));
        assert_eq!(plan.requirement, None);
    }

    #[test]
    fn a_bare_bin_script_is_detected_without_a_marker() {
        let dir = temp("bare-bin");
        script(&dir.join("bin").join("cli"));
        let plan = detect(&dir).unwrap();
        assert_eq!(plan.kind, ProjectKind::Ruby);
        assert_eq!(plan.entry, PathBuf::from("bin/cli"));
    }

    #[test]
    fn an_empty_directory_keeps_the_named_refusal() {
        let dir = temp("empty");
        let err = detect(&dir).unwrap_err();
        assert!(err.contains("no project detected"), "{err}");
        assert!(err.contains("-r <root> -e <entry>"), "{err}");
    }

    #[test]
    fn two_root_scripts_without_a_marker_are_not_guessed() {
        let dir = temp("ambiguous");
        std::fs::write(dir.join("one.rb"), "puts 1\n").unwrap();
        std::fs::write(dir.join("two.rb"), "puts 2\n").unwrap();
        let err = detect(&dir).unwrap_err();
        assert!(err.contains("no project detected"), "{err}");
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
    fn a_node_project_plan_is_derived_from_package_json() {
        let dir = temp("node-bin");
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"hello","bin":{"hello":"cli.js","other":"x.js"},"engines":{"node":">=24"}}"#,
        )
        .unwrap();
        let plan = detect(&dir).unwrap();
        assert_eq!(plan.kind, ProjectKind::Node);
        assert_eq!(plan.entry, PathBuf::from("cli.js"));
        assert_eq!(plan.requirement.as_deref(), Some(">=24"));

        // The bin string form, and the single-key map's fallback.
        let one = temp("node-bin-string");
        std::fs::write(one.join("package.json"), r#"{"bin":"run.js"}"#).unwrap();
        assert_eq!(detect(&one).unwrap().entry, PathBuf::from("run.js"));

        let single = temp("node-bin-single");
        std::fs::write(single.join("package.json"), r#"{"bin":{"tool":"t.js"}}"#).unwrap();
        assert_eq!(detect(&single).unwrap().entry, PathBuf::from("t.js"));
        assert_eq!(detect(&single).unwrap().requirement, None);

        // `main`, then the root index.js.
        let main = temp("node-main");
        std::fs::write(main.join("package.json"), r#"{"main":"lib/app.js"}"#).unwrap();
        assert_eq!(detect(&main).unwrap().entry, PathBuf::from("lib/app.js"));

        let index = temp("node-index");
        std::fs::write(index.join("package.json"), "{}").unwrap();
        std::fs::write(index.join("index.js"), "\n").unwrap();
        assert_eq!(detect(&index).unwrap().entry, PathBuf::from("index.js"));
    }

    #[test]
    fn a_node_project_with_no_derivable_entry_is_refused_by_name() {
        let dir = temp("node-empty");
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        let err = detect(&dir).unwrap_err();
        assert!(err.contains("no bin/main script"), "{err}");

        let ambiguous = temp("node-ambiguous");
        std::fs::write(
            ambiguous.join("package.json"),
            r#"{"name":"hello","bin":{"one":"a.js","two":"b.js"}}"#,
        )
        .unwrap();
        let err = detect(&ambiguous).unwrap_err();
        assert!(err.contains("several bin commands"), "{err}");
        assert!(err.contains("one, two"), "{err}");

        let broken = temp("node-broken");
        std::fs::write(broken.join("package.json"), "{not json").unwrap();
        assert!(detect(&broken).unwrap_err().contains("does not parse"));
    }

    #[test]
    fn the_kinds_with_no_manifest_reader_still_name_their_engine() {
        let py = temp("python");
        std::fs::write(py.join("pyproject.toml"), "").unwrap();
        assert!(detect(&py).unwrap_err().contains("python"));

        let rs = temp("rust");
        std::fs::write(rs.join("Cargo.toml"), "").unwrap();
        assert!(detect(&rs).unwrap_err().contains("rust"));
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
