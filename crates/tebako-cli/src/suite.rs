//! Suite press (spec 03 §6, spec 07 §2.0 multi-command suites — roadmap
//! 34): `tebako press --suite <suite.yaml>` presses ONE package with N
//! invocable commands — per-entry imaging, one slot per entry, and the
//! type-2 package manifest carrying per-entry `runtime_ref`s (the
//! trailer's 128-byte field keeps entries[0]'s for v1-era loaders).
//!
//! The suite file:
//!
//! ```yaml
//! name: metanorma          # optional; default entries[0].name
//! version: 1.2.3           # optional; default "0.0.0"
//! entries:
//!   - name: metanorma      # the command name (a shim/binary link name)
//!     root: ./metanorma-app
//!     entry: metanorma     # the entry point inside that root (like -e)
//!     runtime_ref: ruby@3.4.2;tebako=0.15.9   # optional; falls back to
//!                              # the press-level -R (+ ;image when the
//!                              # resolved runtime is image-era)
//!   - name: mn2pdf
//!     root: ./mn2pdf-app
//!     entry: mn2pdf
//! ```
//!
//! Relative `root`s resolve against the suite file's directory. At run
//! time the bootstrap's argv0 selection (tebako-bootstrap's
//! [`select_entry`]) mounts only the selected entry's slot — every entry
//! image shares the scenario mount point by construction. Fat suites are
//! not a v1 form (one fat payload slot cannot serve per-entry runtimes).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{packaging_error, plain_error, TebakoError};
use crate::options::{host_platform, PressMode, PressOptions};
use crate::packager;
use crate::resolve::{Resolved, Resolver};
use crate::scenario::{self, ruby_version_with_gemfile, ScenarioManager};

// ---------------------------------------------------------------------
// the suite file
// ---------------------------------------------------------------------

/// One invocable command of the suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteEntry {
    /// The command name (shim/binary link name).
    pub name: String,
    /// The application root (as written; resolved against the suite
    /// file's directory at press time).
    pub root: String,
    /// The entry point inside the root (like press's -e).
    pub entry: String,
    /// Explicit per-entry runtime reference
    /// (`ruby@<version>;tebako=<abi>[;image]`); `None` falls back to the
    /// press-level -R.
    pub runtime_ref: Option<String>,
}

/// A parsed + validated suite file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteSpec {
    pub name: String,
    pub version: String,
    pub entries: Vec<SuiteEntry>,
}

#[derive(Deserialize)]
struct RawSuite {
    name: Option<String>,
    version: Option<serde_yml::Value>,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    name: String,
    root: String,
    entry: String,
    runtime_ref: Option<String>,
}

fn invalid(reason: impl Into<String>) -> TebakoError {
    packaging_error(65, Some(&reason.into()))
}

/// Parse and validate a suite file. Named errors everywhere: unknown
/// shape, duplicate/unsafe names, empty fields, slot overflow.
pub fn parse_suite(yaml: &str, source: &Path) -> Result<SuiteSpec, TebakoError> {
    let raw: RawSuite = serde_yml::from_str(yaml).map_err(|e| {
        invalid(format!(
            "cannot parse the suite file {} ({e})",
            source.display()
        ))
    })?;
    if raw.entries.is_empty() {
        return Err(invalid(format!(
            "suite file {} lists no entries (N >= 1 required)",
            source.display()
        )));
    }
    if raw.entries.len() > tpkg::TPKG_MAX_SLOTS as usize {
        return Err(invalid(format!(
            "suite file {} lists {} entries but a package carries at most {} slots",
            source.display(),
            raw.entries.len(),
            tpkg::TPKG_MAX_SLOTS
        )));
    }
    let mut entries = Vec::new();
    for e in &raw.entries {
        check_component("entry name", &e.name, source)?;
        if e.root.trim().is_empty() {
            return Err(invalid(format!(
                "suite entry \"{}\" has an empty root ({})",
                e.name,
                source.display()
            )));
        }
        if e.entry.trim().is_empty() {
            return Err(invalid(format!(
                "suite entry \"{}\" has an empty entry ({})",
                e.name,
                source.display()
            )));
        }
        if let Some(r) = &e.runtime_ref {
            check_runtime_ref(&e.name, r, source)?;
        }
        entries.push(SuiteEntry {
            name: e.name.clone(),
            root: e.root.replace('\\', "/"),
            entry: e.entry.replace('\\', "/"),
            runtime_ref: e.runtime_ref.clone(),
        });
    }
    let mut names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    names.sort();
    if names.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid(format!(
            "suite file {} lists a duplicate entry name",
            source.display()
        )));
    }
    let name = raw
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| entries[0].name.clone());
    check_component("suite name", &name, source)?;
    let version = match &raw.version {
        Some(v) => match v {
            serde_yml::Value::String(s) => s.clone(),
            serde_yml::Value::Number(n) => n.to_string(),
            other => {
                return Err(invalid(format!(
                    "suite file {} has a non-string version ({other:?})",
                    source.display()
                )))
            }
        },
        None => "0.0.0".to_string(),
    };
    check_component("suite version", &version, source)?;
    Ok(SuiteSpec {
        name,
        version,
        entries,
    })
}

/// Entry names become shim/binary file names and the type-2 manifest's
/// selectors; the rule is the payload cache's key rule.
fn check_component(what: &str, value: &str, source: &Path) -> Result<(), TebakoError> {
    let bad = value.is_empty()
        || value == "."
        || value == ".."
        || value
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control() || c.is_whitespace());
    if bad {
        return Err(invalid(format!(
            "{what} '{value}' must be a single non-empty path component ({})",
            source.display()
        )));
    }
    Ok(())
}

/// The per-entry runtime_ref grammar (v1): `ruby@<version>;tebako=<abi>`
/// with optional trailing `;image`. The `;sha256=` parameter is the FAT
/// payload's checksum slot — meaningless for lean suite entries and
/// rejected, never ignored.
fn check_runtime_ref(
    entry_name: &str,
    runtime_ref: &str,
    source: &Path,
) -> Result<(), TebakoError> {
    let err = |reason: &str| {
        invalid(format!(
            "suite entry \"{entry_name}\" has an invalid runtime_ref \"{runtime_ref}\" ({reason}; expected \"ruby@<version>;tebako=<abi>[;image]\", {})",
            source.display()
        ))
    };
    let Some(at) = runtime_ref.find('@') else {
        return Err(err("missing '@'"));
    };
    let (ty, rest) = (&runtime_ref[..at], &runtime_ref[at + 1..]);
    if ty != "ruby" {
        return Err(err("the only language in v1 is ruby"));
    }
    let Some(semi) = rest.find(";tebako=") else {
        return Err(err("missing ';tebako='"));
    };
    let version = &rest[..semi];
    let tail = &rest[semi + 8..];
    let mut parts = tail.split(';');
    let abi = parts.next().unwrap_or("");
    if version.is_empty() || abi.is_empty() {
        return Err(err("empty version or abi"));
    }
    for part in [version, abi] {
        if part
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ' ' | '\t' | '\r' | '\n'))
        {
            return Err(err("no whitespace or separators in components"));
        }
    }
    for param in parts {
        match param {
            "image" => {}
            p if p.starts_with("sha256=") => {
                return Err(err(
                    ";sha256= is the fat-payload checksum — suite presses are lean",
                ))
            }
            _ => return Err(err("unknown parameter")),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// the type-2 package manifest
// ---------------------------------------------------------------------

/// The package manifest (ext block type 2) for a suite: one entry per
/// command, slot i = entries[i]'s image, per-entry runtime refs (already
/// fallback-resolved).
pub fn suite_package_manifest(
    spec: &SuiteSpec,
    runtime_refs: &[String],
    created: &str,
) -> Result<tpkg::PackageManifest, TebakoError> {
    if runtime_refs.len() != spec.entries.len() {
        return Err(plain_error(format!(
            "internal: {} runtime refs for {} suite entries",
            runtime_refs.len(),
            spec.entries.len()
        )));
    }
    let manifest = tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: spec.name.clone(),
            version: spec.version.clone(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            created: created.to_string(),
        },
        entries: spec
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| tpkg::PackageEntry {
                name: e.name.clone(),
                slot: Some(i as u32),
                entrypoint: e.name.clone(),
                runtime_ref: runtime_refs[i].clone(),
            })
            .collect(),
        jail: None,
        env: BTreeMap::new(),
        lock: None,
        // Suite members keep exclusive mounts for now: their shared
        // point collides with the env image's runtime root exactly like
        // the plain press's app slot did — the union rows for suites
        // land with the suite mount-model follow-up (TODO.prepublish/12).
        mounts: Vec::new(),
    };
    manifest
        .validate()
        .map_err(|e| plain_error(format!("invalid suite package manifest: {e}")))?;
    Ok(manifest)
}

// ---------------------------------------------------------------------
// the press
// ---------------------------------------------------------------------

/// `tebako press --suite`: per-entry imaging against each entry's own
/// runtime, then ONE stitch: N slots (all at the scenario mount point —
/// the bootstrap's argv0 selection mounts only the selected entry's
/// slot) plus the type-2 package manifest. Lean only (a fat payload
/// slot cannot serve per-entry runtimes).
pub fn press_suite(
    opts: &PressOptions,
    spec: &SuiteSpec,
    suite_dir: &Path,
) -> Result<PathBuf, TebakoError> {
    if opts.mode == PressMode::Fat {
        return Err(packaging_error(
            126,
            Some("fat suites are not a v1 form (one payload slot cannot serve per-entry runtimes) — press lean"),
        ));
    }
    if opts.mode == PressMode::Runtime || opts.mode == PressMode::Classic {
        return Err(packaging_error(133, None));
    }
    if let Some(requested) = &opts.ruby_requested {
        scenario::check_ruby_version(requested)?;
    }
    for entry in &spec.entries {
        check_entry_abi(entry, opts)?;
    }
    // spec 09 §9: the `sign` registry setting resolves BEFORE any heavy
    // work — a keyid naming no key in $TEBAKO_HOME/keys is the named
    // error here, not after the per-entry imaging. A suite rides no
    // compose document: the CLI and env channels decide (the registry's
    // precedence holds).
    let sign = crate::effective_sign(opts, None)?;
    if !opts.devmode {
        crate::version_cache_check(opts);
    }
    suite_warnings(opts, spec, suite_dir);
    let platform = host_platform()?;
    // Local sources first, else the spec 19 §4 store flow — the retired
    // v1 C++ bootstrap download never fires.
    let bootstrap_path = crate::press_bootstrap(opts, &platform)?;

    // Per-entry imaging; runtimes resolve once per distinct ruby version.
    let mut runtimes: BTreeMap<String, Resolved> = BTreeMap::new();
    let mut images: Vec<(PathBuf, String, u32)> = Vec::new();
    let mut runtime_refs: Vec<String> = Vec::new();
    for (i, entry) in spec.entries.iter().enumerate() {
        let root = entry_root(suite_dir, &entry.root);
        let mut entry_opts = opts.clone();
        entry_opts.root_arg = root.to_string_lossy().into_owned();
        entry_opts.entrance = entry.entry.clone();
        entry_opts.output = None;
        entry_opts.suite = None;
        let mut scenario_mgr = ScenarioManager::new(&entry_opts.root(), &entry_opts.fs_entrance())?;
        scenario_mgr.configure_scenario()?;
        let ruby_ver = entry_ruby_version(opts, &scenario_mgr, entry)?;
        if !runtimes.contains_key(&ruby_ver) {
            let r = Resolver::new().resolve_runtime(&ruby_ver, &platform, &opts.tebako_version)?;
            runtimes.insert(ruby_ver.clone(), r);
        }
        let resolved = &runtimes[&ruby_ver];
        println!(
            "-- Suite entry {}/{}: {} (ruby {})",
            i + 1,
            spec.entries.len(),
            entry.name,
            ruby_ver
        );
        let app_image =
            packager::build_app_image(&entry_opts, &mut scenario_mgr, resolved, &ruby_ver)?;
        // the next entry's build recreates the packaging environment —
        // move this entry's image aside first
        let staged = opts
            .output_folder()
            .join(format!("suite-entry-{i}-{}.tfs", entry.name));
        std::fs::rename(&app_image, &staged).map_err(|e| {
            plain_error(format!(
                "cannot stage {} -> {}: {e}",
                app_image.display(),
                staged.display()
            ))
        })?;
        images.push((
            staged,
            crate::declared_mount(&scenario_mgr.fs_mount_point).to_string(),
            entry_opts.format.tpkg_format_id(),
        ));
        runtime_refs.push(entry_runtime_ref(entry, &ruby_ver, opts, resolved));
    }

    let created = crate::install::rfc3339_utc(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    );
    let package_manifest = suite_package_manifest(spec, &runtime_refs, &created)?;
    let package = suite_package_path(opts, spec);
    // spec 23 §14: a suite rides no compose document — the CLI and env
    // channels decide `quiet_notices` (the registry's precedence holds).
    let quiet_notices = crate::effective_quiet_notices(opts, None)?;
    crate::stitch(
        &bootstrap_path,
        &images,
        &package,
        &runtime_refs[0],
        Some(&package_manifest),
        // Suites never carry the runtime (press_suite refuses the
        // self-contained mode) — the LEAN flag stands.
        crate::StitchFlags {
            lean: true,
            no_install: opts.no_install,
            quiet_notices,
            sign,
        },
    )?;
    println!("Created tebako suite package at \"{package}\"");
    crate::ensure_version_file(opts);
    Ok(PathBuf::from(package))
}

/// An explicit entry runtime_ref must target the tebako abi this press
/// builds against (one release per press — per-entry abi differences are
/// a later milestone).
pub fn check_entry_abi(entry: &SuiteEntry, opts: &PressOptions) -> Result<(), TebakoError> {
    let Some(r) = &entry.runtime_ref else {
        return Ok(());
    };
    // validated at parse: ruby@<version>;tebako=<abi>[;image]
    let semi = r.find(";tebako=").expect("runtime_ref validated");
    let abi = r[semi + 8..].split(';').next().unwrap_or("");
    if abi != opts.tebako_version {
        return Err(invalid(format!(
            "suite entry \"{}\" pins tebako abi {abi} but this press builds against {} — align them (--tebako-version, or drop the abi from the entry's runtime_ref)",
            entry.name, opts.tebako_version
        )));
    }
    Ok(())
}

/// The entry's runtime_ref: explicit, else the press-level fallback
/// (its resolved ruby version + the tebako abi + `;image` when the
/// resolved runtime is image-era).
pub fn entry_runtime_ref(
    entry: &SuiteEntry,
    ruby_ver: &str,
    opts: &PressOptions,
    resolved: &Resolved,
) -> String {
    if let Some(r) = &entry.runtime_ref {
        return r.clone();
    }
    let mut r = format!("ruby@{ruby_ver};tebako={}", opts.tebako_version);
    if resolved.image.is_some() {
        r.push_str(";image");
    }
    r
}

/// The ruby version an entry presses against: its explicit runtime_ref's
/// version, else press's exact rule (the entry's Gemfile when present,
/// else the press-level -R, else the default).
fn entry_ruby_version(
    opts: &PressOptions,
    scenario_mgr: &ScenarioManager,
    entry: &SuiteEntry,
) -> Result<String, TebakoError> {
    if let Some(r) = &entry.runtime_ref {
        // validated at parse: ruby@<version>;tebako=<abi>
        let at = r.find('@').expect("runtime_ref validated");
        let semi = r[at + 1..].find(";tebako=").expect("runtime_ref validated");
        return Ok(r[at + 1..at + 1 + semi].to_string());
    }
    if scenario_mgr.with_gemfile {
        ruby_version_with_gemfile(opts.ruby_requested.as_deref(), &scenario_mgr.gemfile_path)
    } else {
        Ok(opts
            .ruby_requested
            .clone()
            .unwrap_or_else(|| scenario::DEFAULT_RUBY_VERSION.to_string()))
    }
}

/// A suite entry's root, resolved against the suite file's directory.
fn entry_root(suite_dir: &Path, root: &str) -> PathBuf {
    let p = PathBuf::from(root);
    if p.is_absolute() {
        p
    } else {
        suite_dir.join(p)
    }
}

/// The suite package path: --output, or <cwd>/<suite name> (+ exe suffix).
fn suite_package_path(opts: &PressOptions, spec: &SuiteSpec) -> String {
    let package = match &opts.output {
        Some(o) => o.replace('\\', "/"),
        None => format!("{}/{}", opts.fs_current.trim_end_matches('/'), spec.name),
    };
    if Path::new(&package).is_absolute() {
        package
    } else {
        format!("{}/{}", opts.fs_current.trim_end_matches('/'), package)
    }
}

/// The recursive-packaging warnings, per entry root (press's
/// check_warnings covers one root; a suite has N).
fn suite_warnings(opts: &PressOptions, spec: &SuiteSpec, suite_dir: &Path) {
    let package = suite_package_path(opts, spec);
    for entry in &spec.entries {
        let root = entry_root(suite_dir, &entry.root);
        let root_s = root.to_string_lossy().trim_end_matches('/').to_string();
        let mut probe = PathBuf::from(package.trim_end_matches('/'));
        loop {
            if probe == Path::new(&root_s) {
                println!("{}", crate::WARN);
                std::thread::sleep(std::time::Duration::from_secs(5));
                return;
            }
            if !probe.pop() {
                break;
            }
        }
    }
}
