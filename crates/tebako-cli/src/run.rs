//! `tebako run <pkg>` — the dispatch surface for a pressed package
//! (spec 08 §2): the user's tightening (`--jail`, `--mount`, `--no-host`)
//! composed with the package's own `jail:` request — manifest request ∩
//! user policy = effective jail, the user TIGHTENS, never loosens. The
//! composed policy rides TEBAKO_JAIL to the package's bootstrap (which
//! composes once more against its own manifest read — the algebra is
//! idempotent, so old and new bootstraps behave identically) and the
//! runtime driver enforces it (spec 17 §2).
//!
//! Flags before `--` are this surface's; everything after `--` (or after
//! the first non-flag token) is the payload's, verbatim.

use std::path::{Path, PathBuf};

use crate::error::{packaging_error, plain_error, TebakoError};

/// The parsed `tebako run` argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunArgs {
    /// The package file to execute.
    pub package: String,
    /// Raw `--jail` spec (`open` | `deny` | `deny:arg` | a YAML file | the
    /// TEBAKO_JAIL env grammar).
    pub jail: Option<String>,
    /// Raw `--mount host:mount:ro|rw` grants (repeatable).
    pub mounts: Vec<String>,
    /// `--no-host`: tighten to a deny default.
    pub no_host: bool,
    /// Payload args, verbatim.
    pub args: Vec<String>,
}

/// The composed hand-off: the package, its args, and the jail env to
/// export (empty when no policy applies — byte-identical legacy runs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// (key, value) pairs exported on top of the inherited environment.
    pub env: Vec<(String, String)>,
}

/// Parse the `run` argv. Usage errors name the offending token (spec 14
/// §3's named errors on malformed input).
pub fn parse_run_args(args: &[String]) -> Result<RunArgs, String> {
    let Some(package) = args.first() else {
        return Err(
            "usage: tebako run <pkg> [--jail <spec>] [--mount <host:mount:ro|rw>]... [--no-host] [--] [<args>...]"
                .to_string(),
        );
    };
    let mut jail = None;
    let mut mounts = Vec::new();
    let mut no_host = false;
    let mut payload: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        match flag {
            "--" => {
                payload.extend_from_slice(&args[i + 1..]);
                break;
            }
            "--jail" | "--mount" => {
                let value = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        args.get(i)
                            .cloned()
                            .ok_or_else(|| format!("option '{flag}' requires a value"))?
                    }
                };
                if flag == "--jail" {
                    jail = Some(value);
                } else {
                    mounts.push(value);
                }
            }
            "--no-host" => {
                if inline.is_some() {
                    return Err("option '--no-host' takes no value".to_string());
                }
                no_host = true;
            }
            _ if flag.starts_with("--") => {
                return Err(format!(
                    "unknown run option '{flag}' (payload options ride after `--`)"
                ));
            }
            _ => {
                // The first non-flag token starts the payload's argv.
                payload.extend_from_slice(&args[i..]);
                break;
            }
        }
        i += 1;
    }
    Ok(RunArgs {
        package: package.clone(),
        jail,
        mounts,
        no_host,
        args: payload,
    })
}

/// The user's tightening from the parsed flags (the shared
/// dispatch-surface composer, `tpkg::jail::HostJail::from_dispatch_flags`).
/// `None` when no flag was given at all.
fn user_jail(parsed: &RunArgs) -> Result<Option<tpkg::HostJail>, TebakoError> {
    tpkg::HostJail::from_dispatch_flags(parsed.jail.as_deref(), &parsed.mounts, parsed.no_host)
        .map_err(|e| packaging_error(130, Some(&e.to_string())))
}

/// The package's type-2 manifest block; `None` for a block-less or
/// trailer-less package (classic bundles dispatch with the user's
/// tightening alone).
fn package_manifest(package: &Path) -> Result<Option<tpkg::PackageManifest>, TebakoError> {
    let mut f = std::fs::File::open(package).map_err(|e| {
        plain_error(format!(
            "cannot open the package {}: {e}",
            package.display()
        ))
    })?;
    match tpkg::read_from(&mut f) {
        Ok(m) => match m.package_manifest() {
            Ok(pm) => Ok(pm),
            Err(e) => Err(plain_error(format!(
                "invalid package manifest (extension block type 2) in {}: {e}",
                package.display()
            ))),
        },
        Err(tpkg::TpkgError::NoTrailer) => Ok(None),
        Err(e) => Err(plain_error(format!(
            "corrupt tebako manifest trailer in {} ({})",
            package.display(),
            tpkg::strerror(e.code())
        ))),
    }
}

/// Compose the run plan: package jail request ∩ user tightening, rendered
/// to the TEBAKO_JAIL env pair; the raw user tightening additionally rides
/// TEBAKO_JAIL_TIGHTENING so spawned children inherit it as their ceiling
/// (spec 32 §4). With `argument_files: auto-allowed` the
/// payload args naming existing files become read-only grants (resolved
/// against this process's cwd, which the child inherits). The composed
/// policy is bind-validated NOW — a grant naming a missing host path is a
/// named error here, not a surprise inside the package.
pub fn plan_run(parsed: &RunArgs) -> Result<RunPlan, TebakoError> {
    let java_opts = std::env::var(tebako_shim::trust::JAVA_TOOL_OPTIONS).ok();
    let home = tfs::journal::tebako_home_dir();
    plan_run_with(
        parsed,
        &tebako_http::netconfig::global(),
        java_opts.as_deref(),
        home.as_deref(),
    )
}

/// [`plan_run`] on an explicit netconfig / `JAVA_TOOL_OPTIONS` / home
/// (the hermetic form — the process-global reads live in [`plan_run`]).
///
/// The trust bridge (spec 17 §2.3): the resolved netconfig verdict rides
/// the handoff env so the package's bootstrap → driver path materializes
/// the merged cert bundle, and a spawned java row gets the dispatcher-side
/// `JAVA_TOOL_OPTIONS` append (windows + platform mode) — the JVM is a
/// spawned child, never a driver boot. The package's own runtime engine is
/// the bootstrap's resolution (the lock's `LockedRuntime` carries no
/// engine), so the java question is over the lock's `spawned[]` rows.
pub(crate) fn plan_run_with(
    parsed: &RunArgs,
    netcfg: &tebako_http::netconfig::NetworkConfig,
    java_opts: Option<&str>,
    home: Option<&Path>,
) -> Result<RunPlan, TebakoError> {
    let program = PathBuf::from(&parsed.package);
    if !program.is_file() {
        return Err(packaging_error(
            127,
            Some(&format!("package not found: {}", parsed.package)),
        ));
    }
    let user = user_jail(parsed)?;
    let manifest = package_manifest(&program)?;
    let mut env = Vec::new();
    if let Some(user) = &user {
        // spec 32 §4 (locked): operator tightening is HEREDITARY — the
        // parent's user directives ride every spawned child as the
        // ceiling over the child's own recomputed union. The driver's
        // spawn plan intersects it in; it inherits onward to deeper
        // spawns (the plan's env-op block never strips it).
        env.push((
            tpkg::runtime_store::JAIL_TIGHTENING_VAR.to_string(),
            user.to_env_spec(&[]),
        ));
    }
    if let Some((jail, source)) = tpkg::jail::effective(
        manifest.as_ref().and_then(|pm| pm.jail.as_ref()),
        user.as_ref(),
    ) {
        if !jail.is_trivially_open() {
            let arg_files = if jail.argument_files.auto {
                tpkg::jail::resolve_argument_files(&parsed.args)
            } else {
                Vec::new()
            };
            let spec = jail.to_env_spec(&arg_files);
            validate_binds(&spec)?;
            env.push(("TEBAKO_JAIL".to_string(), spec));
            env.push(("TEBAKO_JAIL_SOURCE".to_string(), source.to_string()));
        }
    }
    // The conveyance (every plane): a malformed-for-the-wire extra_ca is
    // the named 65-class config error here, not a surprise at boot.
    env.extend(
        tebako_http::netconfig::trust_bridge_env_from(netcfg)
            .map_err(|e| TebakoError::new(e.to_string(), 65))?,
    );
    let spawned_java = manifest
        .as_ref()
        .and_then(|pm| pm.lock.as_ref())
        .is_some_and(|lock| tebako_shim::trust::spawned_rows_have_java(&lock.spawned));
    if spawned_java {
        if cfg!(windows) && netcfg.tls_roots == tebako_http::netconfig::TlsRoots::Platform {
            if let Some(merged) = tebako_shim::trust::java_tool_options_merge(java_opts) {
                env.push((tebako_shim::trust::JAVA_TOOL_OPTIONS.to_string(), merged));
            }
        }
        if !netcfg.extra_ca.is_empty() {
            tebako_shim::trust::journal_java_gap(home, "tebako");
        }
    }
    Ok(RunPlan {
        program,
        args: parsed.args.clone(),
        env,
    })
}

/// Bind-check the composed env spec (grant paths must exist at dispatch
/// time — the same fail-early contract as `tfs exec --jail`).
#[cfg(unix)]
fn validate_binds(spec: &str) -> Result<(), TebakoError> {
    let parsed = tfs::policy::JailSpec::parse(spec)
        .map_err(|e| packaging_error(130, Some(&e.to_string())))?;
    tfs::policy::HostPolicy::bind(parsed.default, parsed.mounts, parsed.arg_files).map_err(
        |e| {
            let text = String::from_utf8_lossy(tfs::errno::strerror(e)).into_owned();
            packaging_error(
                130,
                Some(&format!("--jail/--mount: cannot bind policy: {text}")),
            )
        },
    )?;
    Ok(())
}

/// Non-unix platforms skip the eager bind (the driver validates at install;
/// the preload/exec path is unix-first, spec 07 §8).
#[cfg(not(unix))]
fn validate_binds(_spec: &str) -> Result<(), TebakoError> {
    Ok(())
}

/// Execute the plan, replacing the process (unix). Never returns on
/// success; the error return is the spawn failure.
#[cfg(unix)]
pub fn exec_plan(plan: &RunPlan) -> TebakoError {
    use std::os::unix::process::CommandExt;
    let mut cmd = std::process::Command::new(&plan.program);
    cmd.args(&plan.args);
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    let err = cmd.exec();
    plain_error(format!(
        "cannot execute the package {}: {err}",
        plan.program.display()
    ))
}

/// Windows has no execve(2): spawn, wait, and propagate the child's exit
/// code (the bootstrap's own handoff rule).
#[cfg(not(unix))]
pub fn exec_plan(plan: &RunPlan) -> TebakoError {
    let mut cmd = std::process::Command::new(&plan.program);
    cmd.args(&plan.args);
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    match cmd.spawn().and_then(|mut child| child.wait()) {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(e) => plain_error(format!(
            "cannot execute the package {}: {e}",
            plan.program.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_flags_and_payload_split() {
        let p = parse_run_args(&args(&[
            "pkg",
            "--jail",
            "deny",
            "--mount",
            "/a:/a:ro",
            "--mount=/b:/b:rw",
            "--no-host",
            "--",
            "-x",
            "in.csv",
        ]))
        .unwrap();
        assert_eq!(p.package, "pkg");
        assert_eq!(p.jail, Some("deny".to_string()));
        assert_eq!(
            p.mounts,
            vec!["/a:/a:ro".to_string(), "/b:/b:rw".to_string()]
        );
        assert!(p.no_host);
        assert_eq!(p.args, vec!["-x".to_string(), "in.csv".to_string()]);

        // No `--`: the first non-flag token starts the payload argv.
        let p = parse_run_args(&args(&["pkg", "--no-host", "input.csv", "--verbose"])).unwrap();
        assert!(p.no_host);
        assert_eq!(
            p.args,
            vec!["input.csv".to_string(), "--verbose".to_string()]
        );

        // No flags at all.
        let p = parse_run_args(&args(&["pkg"])).unwrap();
        assert_eq!(p.jail, None);
        assert!(p.args.is_empty());

        // Errors.
        assert!(parse_run_args(&args(&[])).is_err());
        assert!(parse_run_args(&args(&["pkg", "--jail"])).is_err());
        assert!(parse_run_args(&args(&["pkg", "--frobnicate"])).is_err());
        assert!(parse_run_args(&args(&["pkg", "--no-host=x"])).is_err());
    }

    #[test]
    fn user_jail_composition() {
        // --no-host alone: deny.
        let p = parse_run_args(&args(&["pkg", "--no-host"])).unwrap();
        assert_eq!(user_jail(&p).unwrap(), Some(tpkg::HostJail::deny()));
        // --no-host tightens even an explicit --jail open (never loosens).
        let p = parse_run_args(&args(&["pkg", "--jail", "open", "--no-host"])).unwrap();
        assert_eq!(user_jail(&p).unwrap(), Some(tpkg::HostJail::deny()));
        // --mount under an open default keeps the access bit (docker-style).
        let p = parse_run_args(&args(&["pkg", "--mount", "/a:/work:ro"])).unwrap();
        let user = user_jail(&p).unwrap().unwrap();
        assert!(user.default_open);
        assert_eq!(user.mounts.len(), 1);
        assert_eq!(user.mounts[0].access, tpkg::jail::JailAccess::Ro);
        // --jail deny:arg + --mount.
        let p =
            parse_run_args(&args(&["pkg", "--jail", "deny:arg", "--mount", "/a:/w:rw"])).unwrap();
        let user = user_jail(&p).unwrap().unwrap();
        assert!(!user.default_open);
        assert!(user.argument_files.auto);
        assert_eq!(user.mounts.len(), 1);
        // No flags: no tightening.
        let p = parse_run_args(&args(&["pkg"])).unwrap();
        assert_eq!(user_jail(&p).unwrap(), None);
        // A malformed grant is a named (130) error.
        let p = parse_run_args(&args(&["pkg", "--mount", "frob"])).unwrap();
        assert_eq!(user_jail(&p).unwrap_err().code, 130);
    }

    #[test]
    fn plan_run_exports_the_hereditary_tightening() {
        // spec 32 §4: the raw user tightening rides
        // TEBAKO_JAIL_TIGHTENING so every spawned child inherits the
        // ceiling over its own recomputed union.
        let dir = std::env::temp_dir().join(format!("tebako-cli-run-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = dir.join("pkg");
        std::fs::write(&pkg, b"not-a-package\n").unwrap();
        let pkg = pkg.to_string_lossy().into_owned();

        let p = parse_run_args(&args(&[&pkg, "--jail", "deny"])).unwrap();
        let plan = plan_run(&p).unwrap();
        let tightening = plan
            .env
            .iter()
            .find(|(k, _)| k == tpkg::runtime_store::JAIL_TIGHTENING_VAR)
            .map(|(_, v)| v.clone());
        assert_eq!(tightening, Some(tpkg::HostJail::deny().to_env_spec(&[])));
        assert!(plan.env.iter().any(|(k, _)| k == "TEBAKO_JAIL"));

        // No flags: no tightening key at all (byte-identical legacy run).
        let p = parse_run_args(&args(&[&pkg])).unwrap();
        let plan = plan_run(&p).unwrap();
        assert!(!plan
            .env
            .iter()
            .any(|(k, _)| k == tpkg::runtime_store::JAIL_TIGHTENING_VAR));

        let _ = std::fs::remove_dir_all(&dir);
    }

    use tebako_http::netconfig::{NetworkConfig, TlsRoots};

    /// A package carrying a `lock.spawned[]` java row (spec 23 §13.6's
    /// shared-row shape: no slots, the press-resolved coordinates + pins).
    const JAVA_SPAWNED_PM: &str = "schema_version: 1\n\
         package: {name: mn, version: 1.0.0, producer: {tool: tebako-cli, tool_version: 2.7.0}, created: 2026-09-12T00:00:00Z}\n\
         entries:\n  - {name: probe, slot: 0, entrypoint: probe, runtime_ref: ruby@3.4.2;tebako=0.15.9}\n\
         lock:\n  spawned:\n   - engine: java\n     constraint: \">= 21, < 26\"\n     expose: [java]\n     version: \"21.0.12\"\n     tebako: \"2.1.5\"\n     carry: false\n     exe: {sha256: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n     image: {sha256: \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}\n     source: \"https://github.com/tamatebako/tebako-runtime-openjdk/releases/download\"\n";

    fn package_with_manifest(dir: &std::path::Path, name: &str, pm_yaml: &str) -> PathBuf {
        let pkg = dir.join(name);
        std::fs::write(&pkg, vec![0u8; 100]).unwrap();
        let mut m = tpkg::Manifest::default();
        m.slots
            .push(tpkg::Slot::new(0, 100, tpkg::TPKG_FORMAT_ZIP, "/m"));
        let pm = tpkg::PackageManifest::from_yaml(pm_yaml).unwrap();
        m.set_package_manifest(&pm).unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&pkg).unwrap();
        tpkg::write_to(&mut f, &m).unwrap();
        pkg
    }

    fn plain_package(dir: &std::path::Path, name: &str) -> PathBuf {
        let pkg = dir.join(name);
        std::fs::write(&pkg, b"not-a-package\n").unwrap();
        pkg
    }

    #[test]
    fn plan_run_conveys_the_resolved_netconfig_verdict() {
        let dir = std::env::temp_dir().join(format!("tebako-cli-run-trust-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = plain_package(&dir, "pkg");
        let pkg = pkg.to_string_lossy().into_owned();

        // The default verdict exports nothing (byte-identical legacy run).
        let p = parse_run_args(&args(&[&pkg])).unwrap();
        let plan = plan_run_with(&p, &NetworkConfig::default(), None, None).unwrap();
        assert!(!plan
            .env
            .iter()
            .any(|(k, _)| k == tebako_http::PLATFORM_ROOTS_ENV
                || k == tebako_http::netconfig::EXTRA_CA_ENV));

        // Platform mode conveys the marker.
        let cfg = NetworkConfig {
            tls_roots: TlsRoots::Platform,
            ..Default::default()
        };
        let plan = plan_run_with(&p, &cfg, None, None).unwrap();
        assert_eq!(
            plan.env
                .iter()
                .find(|(k, _)| k == tebako_http::PLATFORM_ROOTS_ENV)
                .map(|(_, v)| v.as_str()),
            Some("1")
        );

        // extra_ca conveys the joined paths.
        let cfg = NetworkConfig {
            extra_ca: vec![PathBuf::from("/etc/pki/corp.pem")],
            ..Default::default()
        };
        let plan = plan_run_with(&p, &cfg, None, None).unwrap();
        assert_eq!(
            plan.env
                .iter()
                .find(|(k, _)| k == tebako_http::netconfig::EXTRA_CA_ENV)
                .map(|(_, v)| v.as_str()),
            Some("/etc/pki/corp.pem")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_run_refuses_an_unexpressible_extra_ca() {
        let dir = std::env::temp_dir().join(format!("tebako-cli-run-badca-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = plain_package(&dir, "pkg");
        let pkg = pkg.to_string_lossy().into_owned();
        let sep = if cfg!(windows) { ';' } else { ':' };
        let cfg = NetworkConfig {
            extra_ca: vec![PathBuf::from(format!("/etc/pki/corp{sep}odd.pem"))],
            ..Default::default()
        };
        let p = parse_run_args(&args(&[&pkg])).unwrap();
        let err = plan_run_with(&p, &cfg, None, None).unwrap_err();
        assert_eq!(err.code, 65, "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn plan_run_bridges_the_spawned_java_row_on_windows_platform_mode() {
        let dir = std::env::temp_dir().join(format!("tebako-cli-run-java-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = package_with_manifest(&dir, "pkg-java", JAVA_SPAWNED_PM);
        let pkg = pkg.to_string_lossy().into_owned();
        let cfg = NetworkConfig {
            tls_roots: TlsRoots::Platform,
            ..Default::default()
        };
        let p = parse_run_args(&args(&[&pkg])).unwrap();
        let plan = plan_run_with(&p, &cfg, Some("-Xmx1g"), None).unwrap();
        assert_eq!(
            plan.env
                .iter()
                .find(|(k, _)| k == tebako_shim::trust::JAVA_TOOL_OPTIONS)
                .map(|(_, v)| v.as_str()),
            Some("-Xmx1g -Djavax.net.ssl.trustStoreType=Windows-ROOT")
        );
        // The user's own truststore config is never stomped.
        let plan = plan_run_with(
            &p,
            &cfg,
            Some("-Djavax.net.ssl.trustStore=C:/corp.p12"),
            None,
        )
        .unwrap();
        assert!(!plan
            .env
            .iter()
            .any(|(k, _)| k == tebako_shim::trust::JAVA_TOOL_OPTIONS));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn plan_run_appends_nothing_for_the_java_plane_on_posix() {
        // The windows-only append is exactly that (the spec names no
        // POSIX java bridge) — the conveyance still rides.
        let dir = std::env::temp_dir().join(format!("tebako-cli-run-java-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = package_with_manifest(&dir, "pkg-java", JAVA_SPAWNED_PM);
        let pkg = pkg.to_string_lossy().into_owned();
        let cfg = NetworkConfig {
            tls_roots: TlsRoots::Platform,
            ..Default::default()
        };
        let p = parse_run_args(&args(&[&pkg])).unwrap();
        let plan = plan_run_with(&p, &cfg, Some("-Xmx1g"), None).unwrap();
        assert!(!plan
            .env
            .iter()
            .any(|(k, _)| k == tebako_shim::trust::JAVA_TOOL_OPTIONS));
        assert!(plan
            .env
            .iter()
            .any(|(k, _)| k == tebako_http::PLATFORM_ROOTS_ENV));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_run_journals_the_extra_ca_java_gap() {
        let dir = std::env::temp_dir().join(format!("tebako-cli-run-gap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let pkg = package_with_manifest(&dir, "pkg-java", JAVA_SPAWNED_PM);
        let pkg = pkg.to_string_lossy().into_owned();
        let cfg = NetworkConfig {
            extra_ca: vec![PathBuf::from("/etc/pki/corp.pem")],
            ..Default::default()
        };
        let p = parse_run_args(&args(&[&pkg])).unwrap();
        let plan = plan_run_with(&p, &cfg, None, Some(&home)).unwrap();
        // The additive verdict still conveys (the driver merges it for
        // the openssl-fashioned planes)…
        assert!(plan
            .env
            .iter()
            .any(|(k, _)| k == tebako_http::netconfig::EXTRA_CA_ENV));
        // …and the gap is journaled by name.
        let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
        assert!(
            journal.contains("event=trust-bridge-java-gap mode=extra-ca"),
            "{journal}"
        );

        // No java row in force → no gap line.
        let home2 = dir.join("home2");
        std::fs::create_dir_all(&home2).unwrap();
        let pkg2 = plain_package(&dir, "pkg-plain");
        let pkg2 = pkg2.to_string_lossy().into_owned();
        let p2 = parse_run_args(&args(&[&pkg2])).unwrap();
        plan_run_with(&p2, &cfg, None, Some(&home2)).unwrap();
        assert!(!home2.join("journal.log").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
