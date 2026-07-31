//! tebako-cli — the packager CLI (item 17's SELF-HOSTING design), a port of
//! the reference gem's lean press (tebako-chainwt lib/tebako):
//!
//!   tebako press -r <root> -e <entry> [-o <output>] [-p <prefix>]
//!                [--cwd <dir>] [-R <ruby>] [-m lean|fat]
//!                [--image <path>:<mount>]... [--bootstrap <path>]
//!                [--tebako-version <v>]
//!   tebako press --suite <suite.yaml>   (one package, N commands —
//!                per-entry imaging + slots + the type-2 package manifest,
//!                spec 03 §6)
//!   tebako cache list
//!   tebako cache prune [--all] [--older-than Nd]
//!   tebako add-registry <ref>
//!   tebako list-registries
//!   tebako update-registries
//!   tebako install <ref | name[@version]>
//!   tebako uninstall <name>
//!
//! Lean press flow (gem's do_press_three_part): resolve the runtime into
//! the shared cache (in-process HTTPS via crates/tebako-http) → seed the
//! packaging environment from its layout → deploy the application under
//! the runtime itself (stub driver) → build the application image
//! in-process (the dwarfs-t Writer, `.tfs`) → stitch onto the bootstrap
//! with a tpkg trailer (runtime_ref + launcher ABI v1).
//!
//! The registry/install surface (spec 04 §2, spec 16 §3.3) resolves and
//! fetches through tebako-resolve, verifies signatures through
//! tebako-signer, and registers shims through tebako-shim — all as
//! libraries, no process spawns (spec 14 §3).
//!
//! Documented deviations from the gem (README carries the full list):
//! - the bootstrap portion defaults to the in-workspace Rust
//!   tebako-bootstrap (sibling of the tebako binary); --bootstrap /
//!   TEBAKO_BOOTSTRAP override, otherwise the C++ release is resolved
//!   with the gem's BootstrapManager machinery;
//! - no mkdwarfs binary anywhere: images are built in-process via the
//!   dwarfs-t Writer and named `.tfs` (dwarfs-t-native FlatBuffers
//!   metadata; `.dwarfs` stays for upstream-compatible images);
//! - the gem/gemspec scenarios are not ported (the RuntimeSdk — native
//!   extension deploy, src/sdk.rs — is: the deploy driver builds gems'
//!   native extensions against the runtime's provisioned header tree);
//! - bundler deploy ops follow the modern rubygems behavior: the gem's
//!   unconditional `bundle install --prefer-local` degrades remote
//!   (re)resolution to dependency-free gems (fontist 3.0.10 → 0.1.0),
//!   so it is opt-in (--prefer-local; a no-op with a complete lockfile),
//!   and the gem's `force_ruby_platform=true` bundle config is not
//!   emitted — precompiled platform gems are the default, with
//!   bundler's own ruby-platform fallback for gems without one (the
//!   ported SDK's source-build path). The deploy's strip re-signs
//!   ad-hoc on macOS so precompiled .so/.bundle stay loadable;
//! - images are stitched densely (tpkg slots carry absolute offsets; the
//!   gem's 8-byte padding is cosmetic);
//! - .tebako.yml is not read.

pub mod deploy;
pub mod error;
pub mod fetch;
pub mod image;
pub mod image_manifest;
pub mod info;
pub mod inspect;
pub mod install;
pub mod options;
pub mod packager;
pub mod publish;
pub mod resolve;
pub mod run;
pub mod runner;
pub mod scenario;
pub mod sdk;
pub mod strip;
pub mod suite;

use std::fs;
use std::path::{Path, PathBuf};

use error::{packaging_error, plain_error, TebakoError};
use options::{host_platform, PressMode, PressOptions};
use resolve::{Flavor, Resolver};
use scenario::{check_ruby_version, ruby_version_with_gemfile, ScenarioManager};

/// Launcher ABI v1 — the bootstrap → runtime handoff contract.
pub const LAUNCHER_ABI: u32 = 1;

/// The tebako version this CLI presses with: the runtime release consumed
/// and the tebako=<...> component of the trailer's runtime_ref. Matches
/// the reference gem's Tebako::VERSION at port time.
pub const DEFAULT_TEBAKO_VERSION: &str = "0.15.9";

pub const VERSION_BANNER: &str = "Tebako executable packager version 0.15.9";

pub(crate) const WARN: &str = "
******************************************************************************************************************
*                                                                                                                *
*  WARNING: You are packaging in-place, i.e.: tebako package will be placed inside application root.             *
*  It is not an error but we do not recommend it because it is a way to keep packaging old versions recrsively.  *
*                                                                                                                *
*  For example, ensure that `--root=` differs from `--output=` as described in README.adoc:                      *
*  tebako press --root='~/projects/myproject' --entry=start.rb --output=/temp/myproject.tebako                   *
*                                                                                                                *
******************************************************************************************************************
";

const WARN2: &str = "
******************************************************************************************************************
*                                                                                                                *
*  WARNING: You are creating packaging environment inside application root.                                      *
*  It is not an error but it means that all build-time artifacts will ne included in tebako package.             *
*  You do not need it unless under very special circumstances like tebako packaging tebako itself.               *
*                                                                                                                *
*  Please consider removing your exisitng `--prefix` folder abd use another one that points outside of `--root`  *
*  like tebako press --r ~/projects/myproject -e start.rb -o /temp/myproject.tebako -p ~/.tebako                 *
*                                                                                                                *
******************************************************************************************************************
";

// ---------------------------------------------------------------------
// press
// ---------------------------------------------------------------------

/// Where the package's bootstrap portion comes from.
pub(crate) enum BootstrapSource {
    /// An explicit local binary (--bootstrap, $TEBAKO_BOOTSTRAP, or the
    /// Rust tebako-bootstrap sitting next to the tebako binary).
    Path(PathBuf),
    /// Resolve the C++ release with the gem's BootstrapManager machinery.
    Download,
}

pub fn press(opts: &PressOptions) -> Result<PathBuf, TebakoError> {
    // --suite: one package, N entries (spec 03 §6 — src/suite.rs).
    if let Some(suite_path) = &opts.suite {
        let yaml = fs::read_to_string(suite_path).map_err(|e| {
            plain_error(format!(
                "cannot read the suite file {}: {e}",
                suite_path.display()
            ))
        })?;
        let spec = suite::parse_suite(&yaml, suite_path)?;
        let dir = suite_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        return suite::press_suite(opts, &spec, &dir);
    }
    if opts.mode == PressMode::Runtime {
        return Err(packaging_error(133, None));
    }
    if opts.mode == PressMode::Classic {
        return Err(plain_error(
            "the 'classic' press mode is a later tebako-rs milestone (use --mode=lean or --mode=fat)",
        ));
    }

    // OptionsManager construction order: the --Ruby value is validated
    // before any scenario checks.
    if let Some(requested) = &opts.ruby_requested {
        check_ruby_version(requested)?;
    }

    // spec 08: --jail is validated before any heavy work (a bad spec must
    // not cost a runtime download). The parsed policy becomes the type-2
    // package manifest's `jail:` block at stitch time — the package's
    // host-access REQUEST (the user can tighten it at run time, never
    // loosen it).
    let jail = match &opts.jail {
        Some(spec) => Some(tpkg::HostJail::from_cli_spec(spec).map_err(|e| {
            packaging_error(
                130,
                Some(&format!("invalid --jail specification '{spec}': {e}")),
            )
        })?),
        None => None,
    };

    // Cli#bootstrap: the cache version guard runs before the press
    // (skipped in devmode, like the gem).
    if !opts.devmode {
        version_cache_check(opts);
    }

    let mut scenario = ScenarioManager::new(&opts.root(), &opts.fs_entrance())?;
    scenario.configure_scenario()?;

    let ruby_ver = if scenario.with_gemfile {
        ruby_version_with_gemfile(opts.ruby_requested.as_deref(), &scenario.gemfile_path)?
    } else {
        opts.ruby_requested
            .clone()
            .unwrap_or_else(|| scenario::DEFAULT_RUBY_VERSION.to_string())
    };

    check_warnings(opts);
    println!("{}", opts.press_announce(&ruby_ver));

    let platform = host_platform()?;
    let bootstrap_source = decide_bootstrap(opts);
    if opts.mode == PressMode::Fat {
        if let BootstrapSource::Download = bootstrap_source {
            check_bootstrap_version()?;
        }
    }

    let runtime_resolver = Resolver::new(Flavor::Runtime);
    let resolved = runtime_resolver.resolve_runtime(&ruby_ver, &platform, &opts.tebako_version)?;
    let runtime_path = resolved.executable.clone();

    let bootstrap_path = match &bootstrap_source {
        BootstrapSource::Path(path) => {
            if !path.is_file() {
                return Err(packaging_error(
                    127,
                    Some(&format!("runtime not found: {}", path.display())),
                ));
            }
            path.clone()
        }
        BootstrapSource::Download => Resolver::new(Flavor::Bootstrap).resolve(
            &resolve::default_bootstrap_version(),
            &platform,
            &resolve::default_bootstrap_version(),
        )?,
    };

    let app_image = packager::build_app_image(opts, &mut scenario, &resolved, &ruby_ver)?;

    let mut images: Vec<(PathBuf, String, u32)> = vec![(
        app_image,
        scenario.fs_mount_point.clone(),
        tpkg::TPKG_FORMAT_DWARFS,
    )];
    for (path, mount) in opts.images()? {
        images.push((PathBuf::from(path), mount, tpkg::TPKG_FORMAT_DWARFS));
    }
    let payload_sha256 = if opts.mode == PressMode::Fat {
        let sha = resolve::sha256_file_hex(&runtime_path)
            .ok_or_else(|| plain_error(format!("cannot hash {}", runtime_path.display())))?;
        images.push((
            runtime_path.clone(),
            String::new(),
            tpkg::TPKG_FORMAT_RUNTIME,
        ));
        Some(sha)
    } else {
        None
    };

    let mut runtime_ref = format!("ruby@{ruby_ver};tebako={}", opts.tebako_version);
    if resolved.image.is_some() {
        // item 30b: the runtime is image-era — the bootstrap resolves
        // the .tfs alongside the interpreter at first run.
        runtime_ref.push_str(";image");
    }
    if let Some(sha) = &payload_sha256 {
        runtime_ref.push_str(&format!(";sha256={sha}"));
    }

    let package = format!("{}{}", opts.package(), scenario.exe_suffix);
    // spec 08: a --jail press embeds the policy as the type-2 package
    // manifest's `jail:` block — the package's host-access REQUEST (the
    // user tightens it at run time, never loosens it; block-less packages
    // keep the exact v1 shape).
    let package_manifest =
        jail.map(|j| press_package_manifest(&package, &runtime_ref, &opts.tebako_version, j));
    stitch(
        &bootstrap_path,
        &images,
        &package,
        &runtime_ref,
        package_manifest.as_ref(),
        opts.no_install,
    )?;
    println!("Created tebako package at \"{package}\"");
    ensure_version_file(opts);
    Ok(PathBuf::from(package))
}

fn check_warnings(opts: &PressOptions) {
    let mut warned = false;
    if opts.package_within_root() {
        println!("{WARN}");
        warned = true;
    }
    if opts.prefix_within_root() {
        println!("{WARN2}");
        warned = true;
    }
    if warned {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

/// Bootstrap lookup: --bootstrap > $TEBAKO_BOOTSTRAP > the Rust
/// tebako-bootstrap next to the tebako binary > the C++ release download.
pub(crate) fn decide_bootstrap(opts: &PressOptions) -> BootstrapSource {
    if let Some(path) = &opts.bootstrap {
        return BootstrapSource::Path(path.clone());
    }
    if let Ok(env_path) = std::env::var("TEBAKO_BOOTSTRAP") {
        if !env_path.is_empty() {
            return BootstrapSource::Path(PathBuf::from(env_path));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let name = if cfg!(windows) {
                "tebako-bootstrap.exe"
            } else {
                "tebako-bootstrap"
            };
            let sibling = dir.join(name);
            if sibling.is_file() {
                return BootstrapSource::Path(sibling);
            }
        }
    }
    BootstrapSource::Download
}

/// The fat payload slot is installed by the bootstrap at first run — a
/// capability added in tebako-bootstrap 0.2.0 (error 134). Applies to the
/// downloaded C++ bootstrap; the in-workspace Rust bootstrap is
/// payload-capable by construction.
fn check_bootstrap_version() -> Result<(), TebakoError> {
    let version = resolve::default_bootstrap_version();
    if scenario::version_cmp(&version, resolve::PAYLOAD_MIN_VERSION) != std::cmp::Ordering::Less {
        return Ok(());
    }
    Err(packaging_error(
        134,
        Some(&format!(
            "fat mode requires tebako-bootstrap >= {} (selected: {version}; set TEBAKO_BOOTSTRAP_VERSION to a payload-capable release)",
            resolve::PAYLOAD_MIN_VERSION
        )),
    ))
}

/// Stitcher.stitch (lean three-part): validate per the gem's error codes,
/// then assemble with tebako-pkg (dense image layout — tpkg slots carry
/// absolute offsets, so the gem's 8-byte padding is not required), chmod,
/// and re-sign ad-hoc on macOS when the binary was signed.
/// `runtime_ref` is the trailer's 128-byte field as built by the caller
/// (suites: entries[0]'s ref — the type-2 manifest carries the per-entry
/// refs, spec 02 §5b / spec 03 §6). `package_manifest`, when present, is
/// embedded as extension block type 2 (a `--jail` press embeds the policy
/// this way, spec 08 §4); `None` keeps the package block-less
/// (byte-identical pre-jails shape).
pub(crate) fn stitch(
    bootstrap_path: &Path,
    images: &[(PathBuf, String, u32)],
    package: &str,
    runtime_ref: &str,
    package_manifest: Option<&tpkg::PackageManifest>,
    no_install: bool,
) -> Result<(), TebakoError> {
    if images.is_empty() {
        return Err(packaging_error(126, Some("at least one image is required")));
    }
    if images.len() > tpkg::TPKG_MAX_SLOTS as usize {
        return Err(packaging_error(
            126,
            Some(&format!(
                "{} images given, at most {} are supported",
                images.len(),
                tpkg::TPKG_MAX_SLOTS
            )),
        ));
    }
    if !bootstrap_path.is_file() {
        return Err(packaging_error(
            127,
            Some(&format!("runtime not found: {}", bootstrap_path.display())),
        ));
    }
    for (path, mount, format_id) in images {
        if !path.is_file() {
            return Err(packaging_error(
                127,
                Some(&format!("image not found: {}", path.display())),
            ));
        }
        if *format_id > tpkg::TPKG_FORMAT_RUNTIME {
            return Err(packaging_error(
                126,
                Some(&format!("invalid format_id {format_id} (0..4 expected)")),
            ));
        }
        if mount.len() >= tpkg::TPKG_MOUNT_POINT_LEN {
            return Err(packaging_error(
                126,
                Some(&format!(
                    "mount point '{}'... exceeds {} bytes",
                    &mount[..32.min(mount.len())],
                    tpkg::TPKG_MOUNT_POINT_LEN - 1
                )),
            ));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for (_, mount, format_id) in images {
        if *format_id == tpkg::TPKG_FORMAT_RUNTIME {
            continue; // payload slots are never mounted
        }
        // Suite members share one mount point by construction (the
        // bootstrap's argv0 selection mounts only the selected entry's
        // slot — spec 03 §6); without a package manifest a duplicate
        // mount stays the error it always was.
        if package_manifest.is_some() {
            continue;
        }
        if !seen.insert(mount) {
            return Err(packaging_error(
                126,
                Some(&format!("duplicate mount point '{mount}'")),
            ));
        }
    }
    if runtime_ref.len() >= tpkg::TPKG_RUNTIME_REF_LEN {
        return Err(packaging_error(
            126,
            Some(&format!(
                "runtime_ref '{runtime_ref}' exceeds {} bytes",
                tpkg::TPKG_RUNTIME_REF_LEN - 1
            )),
        ));
    }

    let output = Path::new(package);
    if let Some(dir) = output.parent() {
        fs::create_dir_all(dir).map_err(|e| plain_error(format!("{e}")))?;
    }
    let pkg_images: Vec<tebako_pkg::PackageImage> = images
        .iter()
        .map(|(path, mount, format_id)| tebako_pkg::PackageImage {
            path: path.clone(),
            mount_point: mount.clone(),
            format_id: *format_id,
        })
        .collect();
    let pkg_options = tebako_pkg::PackageOptions {
        runtime_ref: runtime_ref.to_string(),
        // TPKG_FLAG_LEAN always; TPKG_FLAG_NO_INSTALL when the press
        // froze the package (--no-install, TODO.v2-1/12).
        package_flags: if no_install {
            tpkg::TPKG_FLAG_LEAN | tpkg::TPKG_FLAG_NO_INSTALL
        } else {
            tpkg::TPKG_FLAG_LEAN
        },
        launcher_abi: LAUNCHER_ABI,
        // The L2 package manifest rides along only when the press declares
        // one (today: a --jail policy); block-less packages keep the exact
        // v1 shape. The entry's runtime_ref mirrors the trailer's, so old
        // and new loaders resolve identically (spec 02 §5b).
        package_manifest: package_manifest.cloned(),
        ..Default::default()
    };
    tebako_pkg::bundle_exact(bootstrap_path, &pkg_images, output, &pkg_options)
        .map_err(plain_error)?;
    chmod_755(output);
    resign_if_needed(output);
    Ok(())
}

/// The L2 package manifest a `tebako press --jail` writes (spec 03 §6 /
/// spec 02 §5b): minimal identity (press has no package-version input —
/// a --package-version flag is a later milestone), one entry naming the
/// package, `runtime_ref` mirroring the trailer field, and the jail
/// request. The bootstrap reads this block and composes the jail with the
/// user's tightening at handoff (spec 08 §2).
fn press_package_manifest(
    package: &str,
    runtime_ref: &str,
    tebako_version: &str,
    jail: tpkg::HostJail,
) -> tpkg::PackageManifest {
    let stem = Path::new(package)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| package.to_string());
    tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: stem.clone(),
            version: "0.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: tebako_version.to_string(),
            },
            created: rfc3339_now(),
        },
        entries: vec![tpkg::PackageEntry {
            name: stem.clone(),
            slot: 0,
            entrypoint: stem,
            runtime_ref: runtime_ref.to_string(),
        }],
        jail: Some(jail),
        env: Default::default(),
    }
}

/// RFC 3339 UTC rendering of now (the manifest `created` convention; no
/// chrono in the tree — the civil-from-days algorithm is Howard
/// Hinnant's).
fn rfc3339_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    rfc3339_utc(secs)
}

fn rfc3339_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn chmod_755(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = fs::metadata(path) {
            let mut perms = m.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }
}

/// Codesigning: appending bytes invalidates any embedded code signature.
/// On macOS a signed binary (ad-hoc included) is re-signed ad-hoc,
/// best-effort — codesign(1) refuses to re-sign thin Mach-O binaries
/// carrying trailing payload, so on failure a warning is printed and the
/// package is kept (it still executes on macOS).
fn resign_if_needed(output: &Path) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let signed = std::process::Command::new("codesign")
        .args(["-dv"])
        .arg(output)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !signed {
        return;
    }
    let ok = std::process::Command::new("codesign")
        .arg("--remove-signature")
        .arg(output)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        && std::process::Command::new("codesign")
            .args(["--sign", "-", "--force"])
            .arg(output)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if !ok {
        // the gem warns on stderr (Kernel#warn)
        eprintln!(
            "Warning: ad-hoc re-sign failed for {}; the package still executes on macOS, but its code signature is invalidated by the appended images. Re-sign it with your own identity if you need a valid signature.",
            output.display()
        );
    }
}

/// CacheManager#ensure_version_file (best effort).
pub(crate) fn ensure_version_file(opts: &PressOptions) {
    let deps = opts.deps();
    let _ = fs::create_dir_all(&deps);
    let version_file = deps.join(".environment.version");
    if let Err(e) = fs::write(&version_file, version_key(opts)) {
        println!(
            "{} .environment.version: {e}",
            error::packaging_message(201).unwrap_or("Warning. Could not create cache version file")
        );
    }
}

/// The version key: "<tebako version> at <CLI source dir>" (the gem uses
/// the gem's source checkout; the CLI uses its crate manifest dir).
fn version_key(opts: &PressOptions) -> String {
    format!("{} at {}", opts.tebako_version, env!("CARGO_MANIFEST_DIR"))
}

/// CacheManager#version_cache_check: a stale or foreign packaging
/// environment is cleaned up before the press. Missing/unparseable file
/// → "not recognized" + clean_cache; version mismatch → "created by a
/// gem version" + clean_cache; source mismatch → "created for a
/// different source directory" + clean_output.
pub(crate) fn version_cache_check(opts: &PressOptions) {
    let version_file = opts.deps().join(".environment.version");
    let parsed = fs::read_to_string(&version_file).ok().and_then(|content| {
        let line = content.lines().next()?.to_string();
        let (version, source) = line.split_once(" at ")?;
        Some((version.to_string(), source.to_string()))
    });
    let Some((version, source)) = parsed else {
        println!("CMake cache version was not recognized, cleaning up");
        clean_cache(opts);
        return;
    };
    if version != opts.tebako_version {
        println!(
            "Tebako cache was created by a gem version {version} and cannot be used for gem version {}",
            opts.tebako_version
        );
        clean_cache(opts);
    } else if source != env!("CARGO_MANIFEST_DIR") {
        println!(
            "CMake cache was created for a different source directory '{source}' and cannot be used for '{}'",
            env!("CARGO_MANIFEST_DIR")
        );
        clean_output(opts);
    }
}

/// clean_cache: rm -rf <prefix>/deps and <prefix>/o.
fn clean_cache(opts: &PressOptions) {
    println!("Cleaning tebako packaging environment");
    let _ = fs::remove_dir_all(opts.deps());
    let _ = fs::remove_dir_all(opts.output_folder());
}

/// clean_output: rm -rf <prefix>/o.
fn clean_output(opts: &PressOptions) {
    println!("Cleaning CMake cache");
    let _ = fs::remove_dir_all(opts.output_folder());
}

// ---------------------------------------------------------------------
// cache subcommands (CacheCli port)
// ---------------------------------------------------------------------

pub fn cache_list() {
    let manager = Resolver::new(Flavor::Runtime);
    let entries = manager.entries();
    if entries.is_empty() {
        println!(
            "Runtime package cache is empty ({})",
            manager.cache_root.join("runtimes").display()
        );
        return;
    }
    let mut total = 0u64;
    for entry in &entries {
        total += entry.size_bytes;
        println!(
            "{:<44} {:>9}  {}",
            entry.name,
            human_size(entry.size_bytes),
            human_age(entry.installed_at)
        );
    }
    println!(
        "{:<44} {:>9}",
        format!("Total ({} package(s))", entries.len()),
        human_size(total)
    );
}

/// `tebako cache list --json` (spec 15 §4, additive): the cached runtimes
/// AND payloads with their trust anchors, origins and sizes, as one
/// `"info_schema": 1` document. Read-only; the human form is unchanged.
pub fn cache_list_json() {
    use tebako_pkg::{json_to_string, JsonValue as J};

    let s = |v: &str| J::String(v.to_string());
    let n = |v: u64| J::Number(v.to_string());

    let manager = Resolver::new(Flavor::Runtime);
    let mut total = 0u64;

    let mut runtimes = Vec::new();
    for entry in manager.entries() {
        total += entry.size_bytes;
        let marker = |name: &str| {
            fs::read_to_string(entry.path.join(name))
                .ok()
                .map(|t| t.trim().to_string())
        };
        // The trust anchor is the digest; tolerate the sha256sum form
        // ("<digest>  <file>") by taking the first token.
        let anchor = marker("sha256").and_then(|t| t.split_whitespace().next().map(str::to_string));
        let mut obj = vec![
            ("name".to_string(), s(&entry.name)),
            ("path".to_string(), s(&entry.path.display().to_string())),
            ("size_bytes".to_string(), n(entry.size_bytes)),
            (
                "installed_at_unix".to_string(),
                n(entry
                    .installed_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()),
            ),
        ];
        if let Some(sha) = anchor {
            obj.push(("trust_anchor_sha256".to_string(), s(&sha)));
        }
        if let Some(origin) = marker("origin") {
            obj.push(("origin".to_string(), s(&origin)));
        }
        runtimes.push(J::Object(obj));
    }

    let mut payloads = Vec::new();
    let cache = tebako_resolve::PayloadCache::with_root(&manager.cache_root);
    for entry in cache.list() {
        let size = fs::metadata(&entry.path).map(|m| m.len()).unwrap_or(0);
        total += size;
        let mut obj = vec![
            ("name".to_string(), s(&entry.name)),
            ("version".to_string(), s(&entry.version)),
            ("path".to_string(), s(&entry.path.display().to_string())),
            ("size_bytes".to_string(), n(size)),
            ("trust_anchor_sha256".to_string(), s(&entry.sha256)),
        ];
        if let Some(origin) = &entry.origin {
            obj.push(("origin".to_string(), s(origin)));
        }
        payloads.push(J::Object(obj));
    }

    let doc = J::Object(vec![
        ("info_schema".to_string(), n(1)),
        (
            "cache_root".to_string(),
            s(&manager.cache_root.display().to_string()),
        ),
        ("runtimes".to_string(), J::Array(runtimes)),
        ("payloads".to_string(), J::Array(payloads)),
        ("total_bytes".to_string(), n(total)),
    ]);
    println!("{}", json_to_string(&doc));
}

pub fn cache_prune(all: bool, older_than: Option<&str>) -> Result<(), TebakoError> {
    let manager = Resolver::new(Flavor::Runtime);
    let removed = if all {
        manager.prune(true, None)?
    } else if let Some(days) = older_than.and_then(parse_days) {
        manager.prune(false, Some(days))?
    } else {
        println!("Nothing to do: pass --all or --older-than Nd");
        return Ok(());
    };
    for name in &removed {
        println!("Removed {name}");
    }
    println!("{} cached runtime package(s) removed", removed.len());
    Ok(())
}

fn parse_days(spec: &str) -> Option<u64> {
    let digits = spec.strip_suffix('d').unwrap_or(spec);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn human_size(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn human_age(installed_at: std::time::SystemTime) -> String {
    let age = std::time::SystemTime::now()
        .duration_since(installed_at)
        .unwrap_or_default()
        .as_secs();
    if age < 3600 {
        format!("{}m ago", age / 60)
    } else if age < 86_400 {
        format!("{}h ago", age / 3600)
    } else {
        format!("{}d ago", age / 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_utc_known_dates() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z"); // leap day
        assert_eq!(rfc3339_utc(1_704_067_200), "2024-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(1_767_225_600), "2026-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(86_399), "1970-01-01T23:59:59Z");
    }

    #[test]
    fn press_package_manifest_carries_the_jail_and_mirrors_the_trailer() {
        let jail = tpkg::HostJail::from_cli_spec("deny:arg").unwrap();
        let m = press_package_manifest(
            "/tmp/out/hello",
            "ruby@3.4.2;tebako=0.15.9;image",
            "0.15.9",
            jail,
        );
        // Valid per the tpkg discipline (schema version, N>=1 entries, the
        // jail block's own validation).
        m.validate().unwrap();
        assert_eq!(m.package.name, "hello");
        assert_eq!(m.package.producer.tool, "tebako-cli");
        assert_eq!(m.entries.len(), 1);
        assert_eq!(m.entries[0].slot, 0);
        assert_eq!(m.entries[0].runtime_ref, "ruby@3.4.2;tebako=0.15.9;image");
        let jail = m.jail.as_ref().unwrap();
        assert!(!jail.default_open);
        assert!(jail.argument_files.auto);
        // The YAML form survives a round trip (the block embeds as YAML).
        let back = tpkg::PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn stitch_bakes_the_no_install_flag_only_when_asked() {
        // TODO.v2-1/12: --no-install freezes the package in the trailer
        // (TPKG_FLAG_NO_INSTALL); the default press leaves the bit clear
        // (installable-on-request, pre-era shape).
        let dir = std::env::temp_dir().join(format!("tebako-cli-stitch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("bootstrap");
        std::fs::write(&base, b"BASE").unwrap();
        let img = dir.join("img.tfs");
        std::fs::write(&img, b"IMG").unwrap();

        let frozen = dir.join("frozen");
        stitch(
            &base,
            &[(img.clone(), "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            frozen.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            true,
        )
        .unwrap();
        let mut f = std::fs::File::open(&frozen).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert!(m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL != 0);
        assert!(m.package_flags & tpkg::TPKG_FLAG_LEAN != 0);

        let plain = dir.join("plain");
        stitch(
            &base,
            &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            plain.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            false,
        )
        .unwrap();
        let mut f = std::fs::File::open(&plain).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL, 0);
    }
}
