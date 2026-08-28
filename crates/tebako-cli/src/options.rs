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
        Self::parse_named(s).map(|(mode, _)| mode)
    }

    /// The spec 23 §13.2 vocabulary: `self-contained`/`shared-runtime`
    /// are the locked preset names; `lean`/`fat` stay accepted as
    /// deprecated aliases and return a named warning for the caller to
    /// print (never silent). `classic`/`runtime` are unchanged.
    pub fn parse_named(s: &str) -> Result<(PressMode, Option<String>), String> {
        match s {
            "shared-runtime" => Ok((PressMode::Lean, None)),
            "self-contained" => Ok((PressMode::Fat, None)),
            "lean" => Ok((
                PressMode::Lean,
                Some(
                    "`--mode lean` is deprecated: the preset is now named `shared-runtime` (spec 23 §13.2)"
                        .to_string(),
                ),
            )),
            "fat" => Ok((
                PressMode::Fat,
                Some(
                    "`--mode fat` is deprecated: the preset is now named `self-contained` — the runtime travels as two carried slots (spec 23 §13.2)"
                        .to_string(),
                ),
            )),
            "classic" => Ok((PressMode::Classic, None)),
            "runtime" => Ok((PressMode::Runtime, None)),
            _ => Err(format!(
                "invalid mode '{s}' (self-contained/shared-runtime/classic/runtime expected; lean/fat accepted as deprecated aliases)"
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

/// The application image format (spec 20 §6): which writer the
/// packager's image build runs and which `format_id` hint the app-image
/// slots are stamped with. `limnifs` is the default (spec 20 §6); the
/// flag never changes anything else about press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressImageFormat {
    Dwarfs,
    Limnifs,
}

impl PressImageFormat {
    pub fn parse(s: &str) -> Result<PressImageFormat, String> {
        match s {
            "dwarfs" => Ok(PressImageFormat::Dwarfs),
            "limnifs" => Ok(PressImageFormat::Limnifs),
            _ => Err(format!(
                "unsupported image format '{s}' (supported: dwarfs, limnifs)"
            )),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PressImageFormat::Dwarfs => "dwarfs",
            PressImageFormat::Limnifs => "limnifs",
        }
    }

    /// The slot-record `format_id` hint (spec 02 §6; detection stays
    /// authoritative — the hint never overrides magic).
    pub fn tpkg_format_id(&self) -> u32 {
        match self {
            PressImageFormat::Dwarfs => tpkg::TPKG_FORMAT_DWARFS,
            PressImageFormat::Limnifs => tpkg::TPKG_FORMAT_LIMNIFS,
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
    /// --jail <spec> (spec 08): `open` | `deny` | `deny:arg` | a YAML file
    /// | the TEBAKO_JAIL env grammar. Written into the type-2 package
    /// manifest's `jail:` block — the package's host-access REQUEST.
    pub jail: Option<String>,
    /// --no-install (TODO.v2-1/12): bake TPKG_FLAG_NO_INSTALL — the package
    /// RUNS standalone but every install attempt (`--tebako-install`,
    /// `tebako install <path>`) is refused with a named error. Off by
    /// default (installable on explicit request).
    pub no_install: bool,
    /// --quiet-notices / --no-quiet-notices (spec 23 §14): the CLI channel
    /// of the `quiet_notices` registry setting — bake
    /// TPKG_FLAG_QUIET_NOTICES, suppressing the unsigned-legacy-trailer
    /// warning and the progress lines on every run. Tri-state: `None`
    /// leaves the env (`TEBAKO_QUIET_NOTICES`) and compose-document
    /// (`quiet_notices:`) channels to decide.
    pub quiet_notices: Option<bool>,
    /// --sign[=<keyid>] / --no-sign (spec 09 §9, spec 23 §14): the CLI
    /// channel of the `sign` registry setting — sign the package trailer
    /// at press (TPKG_FLAG_SIGNED_V2 + the v2 chain-of-trust extension).
    /// Bare `--sign` takes the press-local key; `--sign=<keyid>` names a
    /// secret key from $TEBAKO_HOME/keys. Tri-state: `None` leaves the
    /// env (`TEBAKO_SIGN`) and compose-document (`sign:`) channels to
    /// decide.
    pub sign: Option<tpkg::settings::SignCli>,
    /// --format <dwarfs|limnifs> (spec 20 §6): the application image
    /// format. Limnifs by default; `dwarfs` stays an explicit opt-in.
    pub format: PressImageFormat,
    /// --compose <tebako.yaml> (spec 23 §3 D2): the composition document
    /// naming the payload slices pressed around the local app.
    pub compose: Option<PathBuf>,
    /// --carry <all|none|name,…> (spec 23 §13.2): per-slice carry
    /// override; requires --compose.
    pub carry: Option<String>,
    /// --share <name,…> (spec 23 §13.2): per-slice share override;
    /// requires --compose.
    pub share: Option<String>,
    /// True when -m/--mode was passed explicitly (spec 23 §13.2: an
    /// explicit --mode OVERRIDES the compose document's preset — the
    /// invocation beats authored defaults; a defaulted mode never does).
    pub mode_explicit: bool,
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
/// (OptionsManager#host_platform). `tpkg::Platform` owns the vocabulary
/// and host detection (spec 03 §3); unsupported targets fail to compile
/// there, so this cannot produce an off-axis id.
pub fn host_platform() -> Result<String, TebakoError> {
    Ok(tpkg::Platform::host().release_asset_name().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_speaks_the_locked_vocabulary() {
        assert_eq!(
            PressMode::parse_named("self-contained").unwrap(),
            (PressMode::Fat, None)
        );
        assert_eq!(
            PressMode::parse_named("shared-runtime").unwrap(),
            (PressMode::Lean, None)
        );
        assert_eq!(
            PressMode::parse_named("classic").unwrap(),
            (PressMode::Classic, None)
        );
        assert_eq!(
            PressMode::parse_named("runtime").unwrap(),
            (PressMode::Runtime, None)
        );
    }

    #[test]
    fn parse_named_aliases_warn_never_silent() {
        let (mode, warning) = PressMode::parse_named("lean").unwrap();
        assert_eq!(mode, PressMode::Lean);
        let warning = warning.expect("the alias warns");
        assert!(warning.contains("deprecated"), "{warning}");
        assert!(warning.contains("shared-runtime"), "{warning}");

        let (mode, warning) = PressMode::parse_named("fat").unwrap();
        assert_eq!(mode, PressMode::Fat);
        let warning = warning.expect("the alias warns");
        assert!(warning.contains("deprecated"), "{warning}");
        assert!(warning.contains("self-contained"), "{warning}");

        // the quiet parse (non-CLI callers) keeps the alias mapping
        assert_eq!(PressMode::parse("fat").unwrap(), PressMode::Fat);
        assert_eq!(PressMode::parse("shared-runtime").unwrap(), PressMode::Lean);

        let err = PressMode::parse_named("chunky").unwrap_err();
        assert!(err.contains("self-contained/shared-runtime"), "{err}");
        assert!(err.contains("lean/fat"), "{err}");
    }
}
