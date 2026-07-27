//! Port of the gem's OptionsManager (lib/tebako/options_manager.rb):
//! the press option set and every derived path of the packaging
//! environment (<prefix>/o/{s,r,p}, deps/bin, the package file name).

use std::path::{Path, PathBuf};

use crate::error::{packaging_error, TebakoError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressMode {
    Lean,
    Fat,
    Classic,
    Runtime,
}

impl PressMode {
    pub fn parse(s: &str) -> Result<PressMode, String> {
        match s {
            "lean" => Ok(PressMode::Lean),
            "fat" => Ok(PressMode::Fat),
            "classic" => Ok(PressMode::Classic),
            "runtime" => Ok(PressMode::Runtime),
            _ => Err(format!(
                "invalid mode '{s}' (lean/fat/classic/runtime expected)"
            )),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PressMode::Lean => "lean",
            PressMode::Fat => "fat",
            PressMode::Classic => "classic",
            PressMode::Runtime => "runtime",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PressOptions {
    /// Root folder as given on the command line, made absolute
    /// (gem keeps the trailing slash for absolute roots).
    pub root_arg: String,
    /// Entry point as given (-e), backslashes normalized.
    pub entrance: String,
    pub output: Option<String>,
    pub prefix: PathBuf,
    pub cwd: Option<String>,
    /// --Ruby value (validated/resolved against the Gemfile later).
    pub ruby_requested: Option<String>,
    pub mode: PressMode,
    pub log_level: String,
    /// Repeatable --image '<path>:<mount-point>'.
    pub image_specs: Vec<String>,
    /// --bootstrap override.
    pub bootstrap: Option<PathBuf>,
    /// tebako version (runtime release + runtime_ref), default crate const.
    pub tebako_version: String,
    /// --prefer-local: restore the gem-era `bundle install --prefer-local`
    /// (resolution prefers the runtime's own gems — bundled/default gems
    /// are used in place). Off by default: a remote (re)resolution under
    /// --prefer-local degrades to dependency-free gems (fontist 3.0.10
    /// came out as 0.1.0). A no-op with a complete Gemfile.lock.
    pub prefer_local: bool,
    pub verbose: bool,
    /// -D/--devmode: skip the cache version guard.
    pub devmode: bool,
    /// Press-time current directory.
    pub fs_current: String,
    /// --suite <suite.yaml>: one package, N entries (spec 03 §6) —
    /// per-entry imaging + slots + the type-2 package manifest. Roots and
    /// entry points come from the suite file; -r/-e are not accepted with
    /// it.
    pub suite: Option<PathBuf>,
}

impl PressOptions {
    /// OptionsManager#root: relative roots are joined onto the press-time
    /// current directory; absolute roots keep the gem's trailing slash.
    pub fn root(&self) -> String {
        if Path::new(&self.root_arg).is_absolute() {
            format!("{}/", self.root_arg.trim_end_matches('/'))
        } else {
            join_path(&self.fs_current, &self.root_arg)
        }
    }

    pub fn fs_entrance(&self) -> String {
        self.entrance.clone()
    }

    pub fn output_folder(&self) -> PathBuf {
        self.prefix.join("o")
    }

    pub fn data_src_dir(&self) -> PathBuf {
        self.output_folder().join("s")
    }

    pub fn data_pre_dir(&self) -> PathBuf {
        self.output_folder().join("r")
    }

    pub fn data_bin_dir(&self) -> PathBuf {
        self.output_folder().join("p")
    }

    pub fn data_bundle_file(&self) -> PathBuf {
        self.data_bin_dir().join("fs.tfs")
    }

    pub fn deps(&self) -> PathBuf {
        self.prefix.join("deps")
    }

    pub fn deps_bin_dir(&self) -> PathBuf {
        self.deps().join("bin")
    }

    /// OptionsManager#package: --output, or the entry point's base name in
    /// the current folder; relative outputs join onto fs_current.
    pub fn package(&self) -> String {
        let package = match &self.output {
            Some(o) => o.replace('\\', "/"),
            None => {
                let base = Path::new(&self.entrance)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| self.entrance.clone());
                let stem = base.split('.').next().unwrap_or(&base).to_string();
                join_path(&self.fs_current, &stem)
            }
        };
        if Path::new(&package).is_absolute() {
            package
        } else {
            join_path(&self.fs_current, &package)
        }
    }

    /// True when `folder` lies inside `root` (Pathname#ascend comparison).
    pub fn folder_within_root(&self, folder: &str) -> bool {
        let root_string = self.root().trim_end_matches('/').to_string();
        let root = Path::new(&root_string);
        let mut folder = Path::new(folder.trim_end_matches('/'));
        loop {
            if folder == root {
                return true;
            }
            match folder.parent() {
                Some(p) => folder = p,
                None => return false,
            }
        }
    }

    pub fn package_within_root(&self) -> bool {
        self.folder_within_root(&self.package())
    }

    pub fn prefix_within_root(&self) -> bool {
        self.folder_within_root(&self.prefix.to_string_lossy())
    }

    pub fn cwd_announce(&self) -> String {
        self.cwd
            .clone()
            .unwrap_or_else(|| "<Host current directory>".to_string())
    }

    pub fn press_announce(&self, ruby_ver: &str) -> String {
        format!(
            "Running tebako press at {}\n   Mode:                      '{}'\n   Ruby version:              '{}'\n   Project root:              '{}'\n   Application entry point:   '{}'\n   Package file name:         '{}'\n   Loging level:              '{}'\n   Package working directory: '{}'",
            self.prefix.display(),
            self.mode.name(),
            ruby_ver,
            self.root(),
            self.entrance,
            self.package(),
            self.log_level,
            self.cwd_announce()
        )
    }

    /// OptionsManager#images: '<path>:<mount-point>' split on the last
    /// colon; error 130 when either side is empty.
    pub fn images(&self) -> Result<Vec<(String, String)>, TebakoError> {
        let mut out = Vec::new();
        for spec in &self.image_specs {
            match spec.rfind(':') {
                Some(colon) if colon > 0 && colon + 1 < spec.len() => {
                    out.push((spec[..colon].to_string(), spec[colon + 1..].to_string()));
                }
                _ => {
                    let msg = format!(
                        "invalid --image specification '{spec}' ('<path>:<mount-point>' expected)"
                    );
                    return Err(packaging_error(130, Some(&msg)));
                }
            }
        }
        Ok(out)
    }
}

/// OptionsManager#prefix: -p value ('PWD' means the current directory),
/// else $TEBAKO_PREFIX, else ~/.tebako — with the gem's announcements.
pub fn resolve_prefix(prefix_arg: Option<&str>) -> PathBuf {
    match prefix_arg {
        Some("PWD") => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        Some(p) => {
            let p = p.replace('\\', "/");
            absolutize(&p)
        }
        None => match std::env::var("TEBAKO_PREFIX") {
            Ok(env_prefix) if !env_prefix.is_empty() => {
                println!("Using TEBAKO_PREFIX environment variable as prefix");
                absolutize(&env_prefix.replace('\\', "/"))
            }
            _ => {
                println!("No prefix specified, using ~/.tebako");
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".tebako")
            }
        },
    }
}

fn absolutize(p: &str) -> PathBuf {
    let path = PathBuf::from(p);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn join_path(base: &str, rel: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        rel.trim_start_matches('/')
    )
}

/// Platform id of the host, as used by tebako-runtime-ruby package names
/// (OptionsManager#host_platform).
pub fn host_platform() -> Result<String, TebakoError> {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(all(target_os = "linux", target_env = "musl")) {
        "linux-musl"
    } else if cfg!(target_os = "linux") {
        "linux-gnu"
    } else {
        return Err(packaging_error(112, Some(std::env::consts::OS)));
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Err(packaging_error(112, Some(std::env::consts::ARCH)));
    };
    Ok(format!("{os}-{arch}"))
}
