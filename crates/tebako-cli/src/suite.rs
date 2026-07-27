//! Suite press (spec 03 §6, spec 07 §2.0): `tebako press --suite
//! <suite.yaml>` packages MULTIPLE applications (or one application with
//! multiple commands) into ONE tpkg — N payload slots plus the type-2
//! package manifest (extension block, spec 02 §5b) carrying per-entry
//! runtime_refs.
//!
//! The suite file is authored YAML (the tpkg manifest convention):
//!
//! ```yaml
//! package: hellosuite            # optional; default: the -o stem, else "suite"
//! version: 1.0.0                 # optional; default "0.0.0"
//! entries:
//!   - name: hello34              # the command name (argv0 selection, shim name)
//!     root: ./app34              # this entry's press root (-r)
//!     entry: hello.rb            # this entry's entry point (-e)
//!     runtime_ref: ruby@3.4.2;tebako=0.15.9   # optional; default below
//!   - name: hello33
//!     root: ./app33
//!     entry: hello.rb
//! ```
//!
//! Each entry is imaged with the single-press pipeline (scenario →
//! runtime resolution → deploy → in-process image) into its own packaging
//! environment (<prefix>/suite/<name>) and slotted in order. An entry
//! without `runtime_ref` falls back to the press-level `-R` (and the
//! scenario's Gemfile detection), exactly like a plain press. Per-entry
//! refs kill the trailer's 128-byte limit (spec 03 §6); the trailer's v1
//! field carries entries[0]'s ref for v1-era loaders.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{packaging_error, plain_error, TebakoError};
use crate::options::{host_platform, PressMode, PressOptions};
use crate::resolve::{Flavor, Resolver};
use crate::scenario::{self, check_ruby_version, ruby_version_with_gemfile, ScenarioManager};
use crate::{ensure_version_file, packager, stitch, version_cache_check, BootstrapSource};

/// The authored suite file (unknown keys tolerated, the tpkg manifest
/// convention).
#[derive(Debug, Deserialize)]
pub struct SuiteFile {
    /// The package name (type-2 manifest identity; the output default).
    pub package: Option<String>,
    /// The package version (type-2 manifest identity).
    pub version: Option<String>,
    /// One entry per invocable command (1..=TPKG_MAX_SLOTS).
    pub entries: Vec<SuiteEntry>,
}

/// One suite entry: an application to image plus its dispatch identity.
#[derive(Debug, Deserialize)]
pub struct SuiteEntry {
    /// The command name (argv0 selection at run time, the shim name at
    /// install time).
    pub name: String,
    /// This entry's press root (the -r of a plain press).
    pub root: String,
    /// This entry's entry point (the -e of a plain press).
    pub entry: String,
    /// This entry's runtime pin (`ruby@<version>;tebako=<abi>`); absent →
    /// the press-level -R (or the scenario default) applies.
    pub runtime_ref: Option<String>,
}

impl SuiteFile {
    /// Read and validate a suite file.
    pub fn load(path: &Path) -> Result<SuiteFile, TebakoError> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            plain_error(format!(
                "cannot read the suite file {}: {e}",
                path.display()
            ))
        })?;
        let suite: SuiteFile = serde_yml::from_str(&text).map_err(|e| {
            plain_error(format!(
                "cannot parse the suite file {}: {e}",
                path.display()
            ))
        })?;
        suite.validate(path)?;
        Ok(suite)
    }

    fn validate(&self, path: &Path) -> Result<(), TebakoError> {
        let what = |m: &str| format!("{}: {m}", path.display());
        if self.entries.is_empty() {
            return Err(packaging_error(
                126,
                Some(&what(
                    "the suite names no entries (one per invocable command expected)",
                )),
            ));
        }
        if self.entries.len() > tpkg::TPKG_MAX_SLOTS as usize {
            return Err(packaging_error(
                126,
                Some(&what(&format!(
                    "the suite names {} entries but a package carries at most {} slots",
                    self.entries.len(),
                    tpkg::TPKG_MAX_SLOTS
                ))),
            ));
        }
        let mut names: Vec<&str> = self.entries.iter().map(|e| e.name.as_str()).collect();
        names.sort_unstable();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(packaging_error(
                126,
                Some(&what(
                    "duplicate entries[].name (one entry per invocable command)",
                )),
            ));
        }
        for entry in &self.entries {
            tebako_shim::manifest::check_path_component("suite entry name", &entry.name)
                .map_err(|e| packaging_error(126, Some(&what(&e.message))))?;
            if entry.root.is_empty() {
                return Err(packaging_error(
                    126,
                    Some(&what(&format!("entry \"{}\" names no root", entry.name))),
                ));
            }
            if entry.entry.is_empty() {
                return Err(packaging_error(
                    126,
                    Some(&what(&format!(
                        "entry \"{}\" names no entry point",
                        entry.name
                    ))),
                ));
            }
        }
        Ok(())
    }
}

/// A suite entry's explicit runtime_ref, checked: the form is
/// `ruby@<version>;tebako=<abi>` (the CLI resolves ruby runtimes only),
/// the abi component must be the press's own tebako version (the package
/// resolves at run time exactly what it was pressed against). Returns the
/// pinned ruby version.
fn entry_ruby_version(
    entry: &SuiteEntry,
    opts: &PressOptions,
) -> Result<Option<String>, TebakoError> {
    let Some(runtime_ref) = &entry.runtime_ref else {
        return Ok(opts.ruby_requested.clone());
    };
    let invalid = || {
        packaging_error(
            126,
            Some(&format!(
                "invalid runtime_ref \"{runtime_ref}\" for suite entry \"{}\" — expected \"ruby@<version>;tebako=<abi>\"",
                entry.name
            )),
        )
    };
    let at = runtime_ref
        .find('@')
        .filter(|&at| at > 0)
        .ok_or_else(invalid)?;
    let rest = &runtime_ref[at + 1..];
    let semi = rest
        .find(";tebako=")
        .filter(|&s| s > 0)
        .ok_or_else(invalid)?;
    let engine = &runtime_ref[..at];
    let version = &rest[..semi];
    let abi = rest[semi + 8..].split(';').next().unwrap_or("");
    if engine != "ruby" || version.is_empty() || abi.is_empty() {
        return Err(invalid());
    }
    if abi != opts.tebako_version {
        return Err(packaging_error(
            126,
            Some(&format!(
                "runtime_ref \"{runtime_ref}\" for suite entry \"{}\" pins tebako={abi} but this press builds with tebako={} (--tebako-version)",
                entry.name, opts.tebako_version
            )),
        ));
    }
    Ok(Some(version.to_string()))
}

/// The suite press: image every entry, then stitch one package — N slots,
/// the type-2 package manifest, entries[0]'s ref in the v1 trailer field.
pub fn press_suite(opts: &PressOptions, suite_path: &Path) -> Result<PathBuf, TebakoError> {
    if opts.mode != PressMode::Lean {
        return Err(packaging_error(
            130,
            Some(&format!(
                "--suite requires the lean press mode (--mode={} is not supported: a suite pins one lean runtime per entry)",
                opts.mode.name()
            )),
        ));
    }
    if !opts.image_specs.is_empty() {
        return Err(packaging_error(
            130,
            Some("--suite composes its own slots; --image extras are not supported"),
        ));
    }

    let suite = SuiteFile::load(suite_path)?;
    let platform = host_platform()?;
    let bootstrap_path = match crate::decide_bootstrap(opts) {
        BootstrapSource::Path(path) => {
            if !path.is_file() {
                return Err(packaging_error(
                    127,
                    Some(&format!("runtime not found: {}", path.display())),
                ));
            }
            path
        }
        BootstrapSource::Download => Resolver::new(crate::resolve::Flavor::Bootstrap).resolve(
            &crate::resolve::default_bootstrap_version(),
            &platform,
            &crate::resolve::default_bootstrap_version(),
        )?,
    };

    // ---- image every entry (the single-press pipeline, per entry) ----
    let mut images: Vec<(PathBuf, String, u32)> = Vec::with_capacity(suite.entries.len());
    let mut runtime_refs: Vec<String> = Vec::with_capacity(suite.entries.len());
    let mut exe_suffix = String::new();
    for entry in &suite.entries {
        let entry_opts = PressOptions {
            root_arg: entry.root.clone(),
            entrance: entry.entry.clone(),
            output: None,
            // one packaging environment per entry: roots and runtimes
            // differ, and o/{s,r,p} must never interleave.
            prefix: opts.prefix.join("suite").join(&entry.name),
            cwd: opts.cwd.clone(),
            ruby_requested: entry_ruby_version(entry, opts)?,
            mode: opts.mode,
            log_level: opts.log_level.clone(),
            image_specs: Vec::new(),
            bootstrap: opts.bootstrap.clone(),
            suite: None,
            tebako_version: opts.tebako_version.clone(),
            prefer_local: opts.prefer_local,
            verbose: opts.verbose,
            devmode: opts.devmode,
            fs_current: opts.fs_current.clone(),
        };
        if let Some(requested) = &entry_opts.ruby_requested {
            check_ruby_version(requested)?;
        }
        if !entry_opts.devmode {
            version_cache_check(&entry_opts);
        }
        let mut scenario = ScenarioManager::new(&entry_opts.root(), &entry_opts.fs_entrance())?;
        scenario.configure_scenario()?;
        let ruby_ver = if scenario.with_gemfile {
            ruby_version_with_gemfile(entry_opts.ruby_requested.as_deref(), &scenario.gemfile_path)?
        } else {
            entry_opts
                .ruby_requested
                .clone()
                .unwrap_or_else(|| scenario::DEFAULT_RUBY_VERSION.to_string())
        };
        println!("[suite entry \"{}\"]", entry.name);
        println!("{}", entry_opts.press_announce(&ruby_ver));

        let resolved = Resolver::new(Flavor::Runtime).resolve_runtime(
            &ruby_ver,
            &platform,
            &entry_opts.tebako_version,
        )?;
        let app_image =
            packager::build_app_image(&entry_opts, &mut scenario, &resolved, &ruby_ver)?;
        ensure_version_file(&entry_opts);
        exe_suffix = scenario.exe_suffix.clone();

        // The entry's runtime pin: explicit, or the press-level default
        // for this entry — with the `;image` flag when the resolved
        // runtime is image-era (item 30b), mirroring the single press.
        let mut runtime_ref = entry
            .runtime_ref
            .clone()
            .unwrap_or_else(|| format!("ruby@{ruby_ver};tebako={}", entry_opts.tebako_version));
        if resolved.image.is_some() && !runtime_ref.split(';').any(|s| s == "image") {
            runtime_ref.push_str(";image");
        }
        runtime_refs.push(runtime_ref);
        // Every entry's image carries its own full tree; the selected
        // entry's slot mounts at the canonical point per invocation
        // (spec 07 §2.0 — slots never collide: exactly one mounts).
        images.push((
            app_image,
            "/__tebako_memfs__".to_string(),
            tpkg::TPKG_FORMAT_DWARFS,
        ));
    }

    // ---- the type-2 package manifest (spec 03 §6) ----
    let package_name = suite.package.clone().unwrap_or_else(|| {
        opts.output
            .as_deref()
            .and_then(|o| {
                Path::new(o)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "suite".to_string())
    });
    let manifest = tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: package_name.clone(),
            version: suite.version.clone().unwrap_or_else(|| "0.0.0".to_string()),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: opts.tebako_version.clone(),
            },
            created: now_rfc3339(),
        },
        entries: suite
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| tpkg::PackageEntry {
                name: e.name.clone(),
                slot: i as u32,
                entrypoint: e.name.clone(),
                runtime_ref: runtime_refs[i].clone(),
            })
            .collect(),
        jail: None,
        env: Default::default(),
    };

    let package = format!(
        "{}{}",
        match &opts.output {
            Some(o) => {
                let o = o.replace('\\', "/");
                if Path::new(&o).is_absolute() {
                    o
                } else {
                    format!(
                        "{}/{}",
                        opts.fs_current.trim_end_matches('/'),
                        o.trim_start_matches('/')
                    )
                }
            }
            None => format!("{}/{package_name}", opts.fs_current.trim_end_matches('/')),
        },
        exe_suffix
    );
    // The v1 trailer field carries entries[0]'s ref for v1-era loaders;
    // per-entry refs live in the type-2 block (no 128-byte cap there).
    stitch(
        &bootstrap_path,
        &images,
        &package,
        &runtime_refs[0],
        Some(&manifest),
    )?;
    println!("Created tebako package at \"{package}\"");
    Ok(PathBuf::from(package))
}

/// The current UTC time, RFC 3339 (civil-from-days; no chrono in the
/// tree — the manifest's `created` is a free-form string, not a parsed
/// instant).
fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> PressOptions {
        PressOptions {
            root_arg: String::new(),
            entrance: String::new(),
            output: None,
            prefix: PathBuf::from("/tmp/prefix"),
            cwd: None,
            ruby_requested: Some("3.4.2".to_string()),
            mode: PressMode::Lean,
            log_level: "error".to_string(),
            image_specs: Vec::new(),
            bootstrap: None,
            suite: None,
            tebako_version: "0.15.9".to_string(),
            prefer_local: false,
            verbose: false,
            devmode: false,
            fs_current: "/tmp".to_string(),
        }
    }

    #[test]
    fn suite_file_parse_and_validate() {
        let dir = std::env::temp_dir().join(format!("tebako-suite-parse-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("suite.yaml");
        std::fs::write(
            &file,
            "package: hellosuite\nversion: 1.0.0\nentries:\n  - name: hello34\n    root: ./app34\n    entry: hello.rb\n    runtime_ref: ruby@3.4.2;tebako=0.15.9\n  - name: hello33\n    root: ./app33\n    entry: hello.rb\n",
        )
        .unwrap();
        let suite = SuiteFile::load(&file).unwrap();
        assert_eq!(suite.package.as_deref(), Some("hellosuite"));
        assert_eq!(suite.entries.len(), 2);
        assert_eq!(suite.entries[1].runtime_ref, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validation_rejections_are_named_errors() {
        let bad = |yaml: &str| {
            let dir = std::env::temp_dir().join(format!("tebako-suite-bad-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join(format!("suite-{}.yaml", yaml.len()));
            std::fs::write(&file, yaml).unwrap();
            let err = SuiteFile::load(&file).unwrap_err();
            let _ = std::fs::remove_dir_all(&dir);
            err
        };
        assert!(bad("entries: []\n").message.contains("no entries"));
        assert!(bad(
            "entries:\n  - {name: a, root: r, entry: e}\n  - {name: a, root: r, entry: e}\n"
        )
        .message
        .contains("duplicate entries[].name"));
        assert!(bad("entries:\n  - {name: 'a/b', root: r, entry: e}\n")
            .message
            .contains("suite entry name"));
        assert!(bad("entries:\n  - {name: a, root: '', entry: e}\n")
            .message
            .contains("no root"));
        let too_many = format!(
            "entries:\n{}",
            (0..9)
                .map(|i| format!("  - {{name: e{i}, root: r, entry: e}}\n"))
                .collect::<String>()
        );
        assert!(bad(&too_many).message.contains("at most 8 slots"));
    }

    #[test]
    fn explicit_runtime_ref_drives_the_ruby_version() {
        let entry = SuiteEntry {
            name: "a".to_string(),
            root: "r".to_string(),
            entry: "e".to_string(),
            runtime_ref: Some("ruby@3.3.7;tebako=0.15.9".to_string()),
        };
        assert_eq!(
            entry_ruby_version(&entry, &opts()).unwrap(),
            Some("3.3.7".to_string())
        );
        // params after the abi are tolerated (the `;image` flag)
        let entry = SuiteEntry {
            runtime_ref: Some("ruby@3.3.7;tebako=0.15.9;image".to_string()),
            ..entry
        };
        assert_eq!(
            entry_ruby_version(&entry, &opts()).unwrap(),
            Some("3.3.7".to_string())
        );
        // an abi line other than the press's own is a named error
        let entry = SuiteEntry {
            runtime_ref: Some("ruby@3.3.7;tebako=0.16.0".to_string()),
            ..entry
        };
        assert!(entry_ruby_version(&entry, &opts())
            .unwrap_err()
            .message
            .contains("--tebako-version"));
        // non-ruby engines are rejected (the CLI resolves ruby runtimes)
        let entry = SuiteEntry {
            runtime_ref: Some("python@3.13;tebako=0.15.9".to_string()),
            ..entry
        };
        assert!(entry_ruby_version(&entry, &opts()).is_err());
        // no explicit ref → the press-level -R
        let entry = SuiteEntry {
            runtime_ref: None,
            ..entry
        };
        assert_eq!(
            entry_ruby_version(&entry, &opts()).unwrap(),
            Some("3.4.2".to_string())
        );
    }

    #[test]
    fn rfc3339_is_well_formed() {
        let stamp = now_rfc3339();
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert!(stamp.ends_with('Z'), "{stamp}");
        assert_eq!(&stamp[4..5], "-");
        assert_eq!(&stamp[10..11], "T");
    }
}
