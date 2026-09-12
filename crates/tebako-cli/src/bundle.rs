//! `tebako bundle` (spec 16 §6, roadmap 82/83): compose an OFFLINE
//! application bundle from the canonical published artifacts — a
//! pre-seeded store tree (payloads + the whole runtime closure +
//! registry caches + a rendered config.yaml) plus the platform's CLI
//! tool set, laid out for an installer (MSI/pkg/SCCM/zip) to drop onto a
//! machine that may never see the network.
//!
//! The hard constraints (owner 2026-09-10/12):
//! - the bundle is composed of the CANONICAL published artifacts —
//!   payloads and runtimes byte-identical with what the registries
//!   published (the store's own sha256/signature verification at stage
//!   time is the proof), merely PRE-POSITIONED. Nothing is rebuilt.
//! - configurability lives in the bundle's rendered `config.yaml`
//!   (the `--config` org overlay: the spec 04 `network:` block, runtime
//!   preferences, defaults), NEVER in rebuilt bytes.
//!
//! Layout:
//!
//! ```text
//! <out>/bin/          the platform's CLI tools (tebako, tebako-shim,
//!                     tfs, tebako-pkg — the dispatch/run machinery)
//! <out>/home/         the staged TEBAKO_HOME, verbatim store grammar
//!                     (payloads/, runtimes/, registries/, shims/,
//!                     config.yaml, journal.log, …)
//! <out>/BUNDLE.yaml   the descriptor: what was bundled, pinned, and the
//!                     commands the installer materializes onto PATH
//! ```
//!
//! The store tree is self-consistent: the rendered config.yaml pins
//! every staged runtime by its exact (engine, version, tebako-line)
//! identity, the registry caches let a cold machine resolve with zero
//! fetches (spec 05 §4's stale-serve covers day 2 with the cable out),
//! and the shim symlinks (unix) are RELATIVE so the tree relocates as
//! one piece. v1 builds for the HOST platform only — a cross-target
//! bundle rides the per-platform CI leg.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tebako_resolve::{Fetcher, Transport};
use tebako_shim::config::{self, UserConfig};
use tebako_shim::runtime::{self, RuntimeResolution};
use tebako_shim::Ctx;

use crate::error::TebakoError;
use crate::install;

/// Usage/authoring failures (malformed overlay, occupied output, …).
const EX_TEBAKO_USAGE: i32 = 65;

/// The four CLI tools a bundle carries (the platform's set, exe suffix
/// per the host).
const TOOLS: [&str; 4] = ["tebako", "tebako-shim", "tfs", "tebako-pkg"];

/// The optional packed form of the bundle directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

impl ArchiveFormat {
    pub fn parse(s: &str) -> Result<Self, TebakoError> {
        match s {
            "tar.gz" | "tgz" => Ok(Self::TarGz),
            "zip" => Ok(Self::Zip),
            other => Err(TebakoError::new(
                format!("unknown --archive format \"{other}\" — expected `tar.gz` or `zip`"),
                EX_TEBAKO_USAGE,
            )),
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

/// What a bundle run produced (the installer-facing report).
#[derive(Debug)]
pub struct BundleOutcome {
    /// The bundle directory.
    pub dir: PathBuf,
    /// The packed archive, when `--archive` was given.
    pub archive: Option<PathBuf>,
    /// The primary payload (name, version).
    pub payload: (String, String),
    /// Every staged runtime as (engine, language version, tebako line).
    pub runtimes: Vec<(String, String, String)>,
    /// The commands the bundle's shims materialize onto PATH.
    pub commands: Vec<String>,
}

/// The inputs of one bundle run (grouped — the parameter list outgrew
/// the clippy budget at the env injection).
#[derive(Debug)]
pub struct BundleRequest<'a> {
    /// The operator's own TEBAKO_HOME (its registry registrations and
    /// runtime preferences seed the staging home).
    pub builder_home: &'a Path,
    /// The directory holding the platform's CLI binaries (the current
    /// exe's directory in production).
    pub tools_dir: &'a Path,
    /// The bundle target: `<ref | name[@ver]>`.
    pub target: &'a str,
    /// The output directory (created; must not exist).
    pub output: &'a Path,
    /// The `--config` org overlay (a config.yaml fragment).
    pub overlay: Option<&'a Path>,
    /// The optional packed form.
    pub archive: Option<ArchiveFormat>,
    /// The effective environment for the staging resolution
    /// (TEBAKO_RUNTIME_MIRROR and friends) — injected so tests never
    /// touch the process environment.
    pub env: &'a BTreeMap<String, String>,
}

/// `tebako bundle <ref | name[@ver]> --output <dir> [--config org.yaml]
/// [--archive tar.gz|zip]`.
pub fn bundle_with<T: Transport>(
    req: &BundleRequest,
    fetcher: &Fetcher<T>,
) -> Result<BundleOutcome, TebakoError> {
    let output = req.output;
    if output.exists() {
        return Err(TebakoError::new(
            format!(
                "bundle output {} already exists — choose an empty path (a bundle never overwrites)",
                output.display()
            ),
            EX_TEBAKO_USAGE,
        ));
    }
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| {
        TebakoError::new(
            format!("cannot create {}: {e}", parent.display()),
            EX_TEBAKO_USAGE,
        )
    })?;
    let tmp = parent.join(format!(".bundle-staging-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| {
        TebakoError::new(
            format!("cannot create the staging dir {}: {e}", tmp.display()),
            EX_TEBAKO_USAGE,
        )
    })?;
    let result = stage(req, &tmp, fetcher);
    match result {
        Ok(mut outcome) => {
            if let Err(e) = std::fs::rename(&tmp, output) {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(TebakoError::new(
                    format!(
                        "cannot move the staged bundle into {}: {e}",
                        output.display()
                    ),
                    EX_TEBAKO_USAGE,
                ));
            }
            outcome.dir = output.to_path_buf();
            outcome.archive = match req.archive {
                Some(fmt) => Some(pack(output, fmt)?),
                None => None,
            };
            Ok(outcome)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err(e)
        }
    }
}

/// The production entry (real transport, real network).
pub fn bundle(req: &BundleRequest) -> Result<BundleOutcome, TebakoError> {
    bundle_with(req, &Fetcher::new())
}

/// The staging pipeline: tools → config → registries → install (the
/// payload closure + the spawned runtimes, spec 30/32) → the primary
/// runtime warm → pin-from-reality → relocatable shims → the descriptor.
/// Everything happens inside `tmp` (renamed to the output by the caller
/// on success — a failed bundle never leaves a half-tree behind).
fn stage<T: Transport>(
    req: &BundleRequest,
    tmp: &Path,
    fetcher: &Fetcher<T>,
) -> Result<BundleOutcome, TebakoError> {
    let bin = tmp.join("bin");
    let home = tmp.join("home");
    std::fs::create_dir_all(&bin)
        .and_then(|_| std::fs::create_dir_all(&home))
        .map_err(|e| {
            TebakoError::new(
                format!("cannot lay out the bundle tree: {e}"),
                EX_TEBAKO_USAGE,
            )
        })?;

    stage_tools(req.tools_dir, &bin)?;
    stage_config(req.builder_home, &home, req.overlay)?;

    // The builder's registries register INTO the staging home: nickname
    // resolution needs them now, and the cached registry files are the
    // bundle's offline day-1 resolution surface.
    let registries = config::load_config(&home).map_err(map_shim)?.registries;
    for reg in &registries {
        install::add_registry_with(&home, reg, fetcher)?;
    }

    let shim_binary = bin.join(shim_tool_name());
    let outcome = install::install_with(&home, req.target, None, Some(&shim_binary), fetcher)?;

    warm_primary_runtime(&home, &outcome.name, &outcome.version, req.env)?;

    let runtimes = pin_runtimes_from_reality(&home)?;
    relativize_shims(&home, tmp)?;
    let descriptor = Descriptor {
        name: outcome.name.clone(),
        version: outcome.version.clone(),
        platform: tpkg::Platform::host().to_string(),
        tebako_version: env!("CARGO_PKG_VERSION").to_string(),
        commands: outcome.commands.clone(),
        runtimes: runtimes.clone(),
    };
    write_descriptor(tmp, &descriptor)?;

    journal(
        &home,
        &format!(
            "event=bundle-staged name={} version={} runtimes={} commands={}",
            outcome.name,
            outcome.version,
            runtimes.len(),
            outcome.commands.len()
        ),
    );

    Ok(BundleOutcome {
        dir: tmp.to_path_buf(),
        archive: None,
        payload: (outcome.name, outcome.version),
        runtimes,
        commands: outcome.commands,
    })
}

/// Copy the platform's CLI tool set into `bin/` — every tool of the
/// dispatch/run surface, a named error when one is missing (a partial
/// tool set makes a bundle that cannot maintain itself).
fn stage_tools(tools_dir: &Path, bin: &Path) -> Result<(), TebakoError> {
    let suffix = exe_suffix();
    for tool in TOOLS {
        let src = tools_dir.join(format!("{tool}{suffix}"));
        if !src.is_file() {
            return Err(TebakoError::new(
                format!(
                    "the tool set beside the tebako exe is incomplete: {} not found — run the bundle verb from a full CLI installation (all four tools ship together)",
                    src.display()
                ),
                EX_TEBAKO_USAGE,
            ));
        }
        std::fs::copy(&src, bin.join(format!("{tool}{suffix}"))).map_err(|e| {
            TebakoError::new(
                format!("cannot stage {}: {e}", src.display()),
                EX_TEBAKO_USAGE,
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dst = bin.join(format!("{tool}{suffix}"));
            let mut perms = std::fs::metadata(&dst)
                .map_err(|e| {
                    TebakoError::new(format!("stat {}: {e}", dst.display()), EX_TEBAKO_USAGE)
                })?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dst, perms).map_err(|e| {
                TebakoError::new(format!("chmod {}: {e}", dst.display()), EX_TEBAKO_USAGE)
            })?;
        }
    }
    Ok(())
}

/// Render the staging home's config.yaml: the builder's registry
/// registrations and runtime preferences seed it, the org overlay
/// (`--config`) overrides per section — the `network:` block (spec 04,
/// the 81 trust bridge), per-engine runtime prefs, per-tool defaults.
/// The bundle re-pins `runtimes:` from the staged reality after the
/// warm, so the shipped config can never name a runtime the bundle
/// does not carry.
fn stage_config(
    builder_home: &Path,
    home: &Path,
    overlay: Option<&Path>,
) -> Result<(), TebakoError> {
    let builder = config::load_config(builder_home).map_err(map_shim)?;
    let overlay: Option<UserConfig> = match overlay {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| {
                TebakoError::new(
                    format!("cannot read the org config overlay {}: {e}", path.display()),
                    EX_TEBAKO_USAGE,
                )
            })?;
            let parsed: UserConfig = serde_yml::from_str(&text).map_err(|e| {
                TebakoError::new(
                    format!(
                        "cannot parse the org config overlay {} ({e}) — the overlay is a config.yaml fragment (network:, runtimes:, defaults:, registries:)",
                        path.display()
                    ),
                    EX_TEBAKO_USAGE,
                )
            })?;
            Some(parsed)
        }
        None => None,
    };
    let mut registries = builder.registries.clone();
    if let Some(ov) = &overlay {
        for r in &ov.registries {
            if !registries.contains(r) {
                registries.push(r.clone());
            }
        }
    }
    let mut runtimes = builder.runtimes.clone();
    let mut defaults = builder.defaults.clone();
    let mut network = builder.network;
    if let Some(ov) = overlay {
        for (engine, pref) in ov.runtimes {
            runtimes.insert(engine, pref);
        }
        for (tool, pin) in ov.defaults {
            defaults.insert(tool, pin);
        }
        if ov.network.proxy.is_some()
            || ov.network.tls_roots.is_some()
            || !ov.network.extra_ca.is_empty()
        {
            network = ov.network;
        }
    }
    write_config(home, &registries, &runtimes, &defaults, &network)
}

/// Serialize a config.yaml — YAML by hand (the store grammar's rule:
/// YAML for all authored config, and the renderer escapes nothing that
/// the grammar admits — every value here is a plain scalar or path
/// string; a value that would break the plain-scalar shape is a named
/// error, never a silently quoted guess).
fn write_config(
    home: &Path,
    registries: &[String],
    runtimes: &BTreeMap<String, config::RuntimePref>,
    defaults: &BTreeMap<String, String>,
    network: &config::NetworkSection,
) -> Result<(), TebakoError> {
    let mut out = String::new();
    if !defaults.is_empty() {
        out.push_str("defaults:\n");
        for (tool, ver) in defaults {
            out.push_str(&format!("  {}: {}\n", scalar(tool)?, scalar(ver)?));
        }
    }
    if !registries.is_empty() {
        out.push_str("registries:\n");
        for r in registries {
            out.push_str(&format!("  - {}\n", scalar(r)?));
        }
    }
    if !runtimes.is_empty() {
        out.push_str("runtimes:\n");
        for (engine, pref) in runtimes {
            out.push_str(&format!("  {}:\n", scalar(engine)?));
            out.push_str(&format!("    version: {}\n", scalar(&pref.version)?));
            if !pref.tebako.is_empty() {
                out.push_str(&format!("    tebako: {}\n", scalar(&pref.tebako)?));
            }
            if let Some(source) = &pref.source {
                out.push_str(&format!("    source: {}\n", scalar(source)?));
            }
        }
    }
    if network.proxy.is_some() || network.tls_roots.is_some() || !network.extra_ca.is_empty() {
        out.push_str("network:\n");
        if let Some(proxy) = &network.proxy {
            out.push_str(&format!("  proxy: {}\n", scalar(proxy)?));
        }
        if let Some(roots) = &network.tls_roots {
            out.push_str(&format!("  tls_roots: {}\n", scalar(roots)?));
        }
        if !network.extra_ca.is_empty() {
            out.push_str("  extra_ca:\n");
            for ca in &network.extra_ca {
                out.push_str(&format!("    - {}\n", scalar(&ca.display().to_string())?));
            }
        }
    }
    std::fs::write(config::config_path(home), out).map_err(|e| {
        TebakoError::new(
            format!("cannot write the bundle config.yaml: {e}"),
            EX_TEBAKO_USAGE,
        )
    })
}

/// A config/descriptor value renders as a YAML plain scalar only when
/// the plain style ROUND-TRIPS IT AS A STRING — a value the core schema
/// would re-type (int/float/bool/null: `1.0`, `3.13`, `true`) renders
/// double-quoted instead (the admitted charset never needs an escape
/// inside quotes). A value outside the charset is a named error, never
/// a guessed encoding.
fn scalar(v: &str) -> Result<std::borrow::Cow<'_, str>, TebakoError> {
    let safe = !v.is_empty()
        && v.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '.' | '-' | '_' | '/' | ':' | '+' | '@' | '~' | '=' | '?')
        })
        && !v.starts_with('-');
    if !safe {
        return Err(TebakoError::new(
            format!("config value {v:?} is not a YAML plain scalar — simplify it"),
            EX_TEBAKO_USAGE,
        ));
    }
    let mistyped = v.parse::<i64>().is_ok()
        || v.parse::<f64>().is_ok()
        || matches!(
            v,
            "true" | "false" | "True" | "False" | "TRUE" | "FALSE" | "null" | "Null" | "NULL" | "~"
        );
    Ok(if mistyped {
        format!("\"{v}\"").into()
    } else {
        v.into()
    })
}

/// Pre-stage the PRIMARY payload's runtime (the `kind: language` axis):
/// install stages the spawned edges (spec 30/32) but the payload's own
/// runtime resolves at dispatch — a bundle that ships without it is not
/// offline-ready. Resolve it exactly as dispatch would (the shim's own
/// resolver against the staging home), downloading on a cache miss.
fn warm_primary_runtime(
    home: &Path,
    name: &str,
    version: &str,
    env: &BTreeMap<String, String>,
) -> Result<(), TebakoError> {
    let record = tebako_shim::manifest::payload_record(home, name, version);
    let mirror =
        tebako_shim::manifest::Manifest::load(&record.manifest_mirror).map_err(map_shim)?;
    let ctx = Ctx {
        home: home.to_path_buf(),
        cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        env: env.clone(),
    };
    let mut warmed: Vec<String> = Vec::new();
    for ep in mirror.entrypoints() {
        let Some(reqs) = &ep.runtime_requirement else {
            continue;
        };
        let key = format!("{reqs:?}");
        if warmed.contains(&key) {
            continue;
        }
        match runtime::resolve_runtime(Some(reqs), true, &ctx).map_err(map_shim)? {
            RuntimeResolution::Ready(rt) => {
                journal(
                    home,
                    &format!(
                        "event=bundle-runtime-warm engine={} version={} tebako={}",
                        rt.engine, rt.lang_version, rt.tebako_version
                    ),
                );
                warmed.push(key);
            }
            RuntimeResolution::Zero => {}
        }
    }
    Ok(())
}

/// Re-pin the staging config's `runtimes:` from the staged reality
/// (scan the staged store, one pin per engine — the newest staged when
/// several lines coexist; the dispatch-time resolver still serves every
/// staged line cache-first, the pin is the bundle's own consistency
/// record and the day-2 download fallback). A pref for an engine the
/// bundle did not stage survives verbatim (the operator pinned it for
/// runtimes the payload resolves later).
fn pin_runtimes_from_reality(home: &Path) -> Result<Vec<(String, String, String)>, TebakoError> {
    let staged = tpkg::runtime_store::scan_all_cached(home);
    let mut cfg = config::load_config(home).map_err(map_shim)?;
    let mut pins: Vec<(String, String, String)> = Vec::new();
    let mut engines: Vec<&str> = staged.iter().map(|r| r.engine.as_str()).collect();
    engines.sort();
    engines.dedup();
    for engine in engines {
        let entries: Vec<_> = staged.iter().filter(|r| r.engine == engine).collect();
        let pick = entries
            .iter()
            .max_by(|a, b| {
                tpkg::versions::compare(&a.lang_version, &b.lang_version)
                    .then_with(|| tpkg::versions::compare(&a.tebako_version, &b.tebako_version))
            })
            .expect("engines derives from staged");
        cfg.runtimes.insert(
            engine.to_string(),
            config::RuntimePref {
                version: pick.lang_version.clone(),
                tebako: pick.tebako_version.clone(),
                source: cfg.runtimes.get(engine).and_then(|p| p.source.clone()),
            },
        );
        pins.push((
            engine.to_string(),
            pick.lang_version.clone(),
            pick.tebako_version.clone(),
        ));
    }
    let path = config::config_path(home);
    // Re-render with the SAME shape stage_config produced (pins merged
    // over the overlay), preserving registries/defaults/network.
    write_config(
        home,
        &cfg.registries,
        &cfg.runtimes,
        &cfg.defaults,
        &cfg.network,
    )
    .map_err(|e| {
        TebakoError::new(
            format!("cannot re-pin {}: {e}", path.display()),
            EX_TEBAKO_USAGE,
        )
    })?;
    Ok(pins)
}

/// Make the shims relocatable: install links them against the bundle's
/// staged dispatcher (an ABSOLUTE path into the build location) — unix
/// links are rewritten to the bundle-relative `../../bin/tebako-shim`;
/// windows shims are byte copies/hardlinks and self-contained already.
fn relativize_shims(home: &Path, bundle_root: &Path) -> Result<(), TebakoError> {
    let dir = home.join("shims");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Ok(());
    };
    for entry in rd.flatten() {
        let link = entry.path();
        #[cfg(unix)]
        {
            let Ok(target) = std::fs::read_link(&link) else {
                continue;
            };
            if !target.is_absolute() {
                continue;
            }
            if !target.starts_with(bundle_root) {
                continue;
            }
            let file = link
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default()
                .to_string();
            let tool = if file.ends_with(".exe") {
                "tebako-shim.exe"
            } else {
                "tebako-shim"
            };
            let rel = Path::new("..").join("..").join("bin").join(tool);
            std::fs::remove_file(&link)
                .and_then(|_| std::os::unix::fs::symlink(&rel, &link))
                .map_err(|e| {
                    TebakoError::new(
                        format!("cannot relativize the shim {}: {e}", link.display()),
                        EX_TEBAKO_USAGE,
                    )
                })?;
        }
        #[cfg(not(unix))]
        {
            let _ = (link, bundle_root);
        }
    }
    Ok(())
}

/// The bundle descriptor (BUNDLE.yaml): the installer's manifest of
/// what the tree carries — the payload identity, the staged runtimes,
/// and the command list whose shims land on PATH.
struct Descriptor {
    name: String,
    version: String,
    platform: String,
    tebako_version: String,
    commands: Vec<String>,
    runtimes: Vec<(String, String, String)>,
}

fn write_descriptor(root: &Path, d: &Descriptor) -> Result<(), TebakoError> {
    let mut out = String::from("schema_version: 1\n");
    out.push_str(&format!("payload: {}\n", scalar(&d.name)?));
    out.push_str(&format!("version: {}\n", scalar(&d.version)?));
    out.push_str(&format!("platform: {}\n", scalar(&d.platform)?));
    out.push_str(&format!("tebako: {}\n", scalar(&d.tebako_version)?));
    if !d.runtimes.is_empty() {
        out.push_str("runtimes:\n");
        for (engine, lv, tv) in &d.runtimes {
            out.push_str(&format!(
                "  - {{engine: {}, version: {}, tebako: {}}}\n",
                scalar(engine)?,
                scalar(lv)?,
                scalar(tv)?
            ));
        }
    }
    if !d.commands.is_empty() {
        out.push_str("commands:\n");
        for c in &d.commands {
            out.push_str(&format!("  - {}\n", scalar(c)?));
        }
    }
    std::fs::write(root.join("BUNDLE.yaml"), out)
        .map_err(|e| TebakoError::new(format!("cannot write BUNDLE.yaml: {e}"), EX_TEBAKO_USAGE))
}

/// Pack the bundle directory next to itself (`<out>.tar.gz` / `<out>.zip`).
fn pack(dir: &Path, fmt: ArchiveFormat) -> Result<PathBuf, TebakoError> {
    let archive = dir.with_extension(fmt.extension());
    match fmt {
        ArchiveFormat::TarGz => {
            let file = std::fs::File::create(&archive).map_err(|e| {
                TebakoError::new(
                    format!("cannot create {}: {e}", archive.display()),
                    EX_TEBAKO_USAGE,
                )
            })?;
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut builder = tar::Builder::new(enc);
            let base = dir
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("bundle")
                .to_string();
            // Symlinks must survive the pack (the relocatable shim links
            // ARE the bundle's dispatch surface) — follow_links off.
            builder.follow_symlinks(false);
            builder.append_dir_all(&base, dir).map_err(|e| {
                TebakoError::new(
                    format!("cannot pack {}: {e}", dir.display()),
                    EX_TEBAKO_USAGE,
                )
            })?;
            builder
                .into_inner()
                .and_then(|enc| enc.finish())
                .map_err(|e| {
                    TebakoError::new(
                        format!("cannot finish {}: {e}", archive.display()),
                        EX_TEBAKO_USAGE,
                    )
                })?;
        }
        ArchiveFormat::Zip => {
            return Err(TebakoError::new(
                "the zip bundle leg is not written yet — tar.gz covers the POSIX legs today; zip lands with the windows installer templates (roadmap 83's second half)",
                EX_TEBAKO_USAGE,
            ));
        }
    }
    Ok(archive)
}

/// The dispatcher tool's file name on this platform.
fn shim_tool_name() -> String {
    format!("tebako-shim{}", exe_suffix())
}

fn exe_suffix() -> &'static str {
    if cfg!(windows) {
        ".exe"
    } else {
        ""
    }
}

fn map_shim(e: tebako_shim::ShimError) -> TebakoError {
    TebakoError::new(e.message, i32::from(e.code))
}

/// Append one line to the staging home's audit journal — the shim's
/// journal line grammar (unix-secs + line), mirrored here because the
/// shim's own `journal` is crate-private. Best-effort, like every
/// journal write: the bundle never fails on the record.
fn journal(home: &Path, line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("journal.log"))
    {
        use std::io::Write as _;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = f.write_all(format!("{now} {line}\n").as_bytes());
    }
}

/// `tebako bundle`'s exit-code surface: install failures keep the
/// install code; bundle authoring failures are usage-class.
pub fn exit_code(err: &TebakoError) -> i32 {
    err.code
}
