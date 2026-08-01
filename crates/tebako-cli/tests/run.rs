//! `tebako run` e2e (spec 08 §2 — the dispatch surface): the user's
//! tightening flags composed with the package's own `jail:` request reach
//! the package as TEBAKO_JAIL (+ the audit source label). The fixture
//! "package" is a shell script with a tpkg trailer stitched on — the
//! kernel execs the shebang and the script reports the env; the trailer
//! bytes after `exit 0` are never read. Unix only (shebang exec).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn workdir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tebako-run-e2e-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const PROBE: &str = "#!/bin/sh\n\
echo \"JAIL=${TEBAKO_JAIL-UNSET}\"\n\
echo \"JAIL-SOURCE=${TEBAKO_JAIL_SOURCE-UNSET}\"\n\
exit 0\n";

fn package_manifest(jail: tpkg::HostJail) -> tpkg::PackageManifest {
    tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: "probe".to_string(),
            version: "1.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.15.9".to_string(),
            },
            created: "2026-07-27T00:00:00Z".to_string(),
        },
        entries: vec![tpkg::PackageEntry {
            name: "probe".to_string(),
            slot: 0,
            entrypoint: "probe".to_string(),
            runtime_ref: "ruby@9.9.9;tebako=9.9.9".to_string(),
        }],
        jail: Some(jail),
        env: Default::default(),
    }
}

/// The probe script + one dummy slot + optional type-2 jail block.
fn script_pkg(dir: &Path, name: &str, jail: Option<tpkg::HostJail>) -> PathBuf {
    let pkg = dir.join(name);
    std::fs::write(&pkg, PROBE).unwrap();
    let payload = dir.join(format!("{name}.payload"));
    std::fs::write(&payload, b"fake image payload").unwrap();

    let mut m = tpkg::Manifest {
        package_flags: 0,
        launcher_abi: 0,
        ..Default::default()
    };
    m.set_runtime_ref(b"ruby@9.9.9;tebako=9.9.9");
    let base = std::fs::metadata(&pkg).unwrap().len();
    let size = std::fs::metadata(&payload).unwrap().len();
    m.slots.push(tpkg::Slot::new(
        base,
        size,
        tpkg::TPKG_FORMAT_DWARFS,
        "/__tfs__",
    ));
    if let Some(jail) = jail {
        m.set_package_manifest(&package_manifest(jail)).unwrap();
    }
    {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new().append(true).open(&pkg).unwrap();
        f.write_all(&std::fs::read(&payload).unwrap()).unwrap();
        tpkg::write_to(&mut f, &m).unwrap();
    }
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&pkg, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    pkg
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

fn tebako_run(args: &[&str]) -> Run {
    let out = Command::new(tebako_bin())
        .arg("run")
        .args(args)
        .output()
        .unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn line<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .unwrap_or_else(|| panic!("{key} missing from stdout:\n{stdout}"))
}

#[test]
fn run_package_jail_alone_maps_to_tebako_jail() {
    let dir = workdir("manifest");
    let input = dir.join("in.csv");
    std::fs::write(&input, b"x").unwrap();
    let pkg = script_pkg(&dir, "pkg1", Some(tpkg::HostJail::deny_with_arg_files()));

    let r = tebako_run(&[pkg.to_str().unwrap(), "--", input.to_str().unwrap()]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(
        line(&r.stdout, "JAIL="),
        format!("deny;@{}", input.display())
    );
    assert_eq!(line(&r.stdout, "JAIL-SOURCE="), "manifest");
}

#[test]
fn run_user_tightening_intersects_never_loosens() {
    let dir = workdir("precedence");
    let mut package = tpkg::HostJail::deny();
    package.mounts.push(tpkg::JailMount {
        host: "/data".to_string(),
        mount: "/data".to_string(),
        access: tpkg::JailAccess::Ro,
    });
    let pkg = script_pkg(&dir, "pkg2", Some(package));

    // --no-host caps the manifest's grant (the user TIGHTENS, wins).
    let r = tebako_run(&[pkg.to_str().unwrap(), "--no-host"]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(line(&r.stdout, "JAIL="), "deny");
    assert_eq!(line(&r.stdout, "JAIL-SOURCE="), "manifest+user");
}

#[test]
fn run_user_flags_alone_when_the_package_is_silent() {
    let dir = workdir("user");
    let pkg = script_pkg(&dir, "pkg3", None);
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();

    let spec = format!("{}:/work:rw", work.display());
    let r = tebako_run(&[pkg.to_str().unwrap(), "--jail", "deny", "--mount", &spec]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(
        line(&r.stdout, "JAIL="),
        format!("deny;{}:/work:rw", work.display())
    );
    assert_eq!(line(&r.stdout, "JAIL-SOURCE="), "user");
}

#[test]
fn run_no_policy_anywhere_exports_nothing() {
    let dir = workdir("legacy");
    let pkg = script_pkg(&dir, "pkg4", None);

    let r = tebako_run(&[pkg.to_str().unwrap()]);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(line(&r.stdout, "JAIL="), "UNSET");
    assert_eq!(line(&r.stdout, "JAIL-SOURCE="), "UNSET");
}

#[test]
fn run_mount_with_a_missing_host_fails_at_dispatch() {
    let dir = workdir("bindfail");
    let pkg = script_pkg(&dir, "pkg5", None);

    let r = tebako_run(&[
        pkg.to_str().unwrap(),
        "--jail",
        "deny",
        "--mount",
        "/no/such/dir-xyz:/w:ro",
    ]);
    assert_eq!(r.rc, 130, "stdout: {} stderr: {}", r.stdout, r.stderr);
    assert!(
        r.stdout.contains("cannot bind policy"),
        "stdout: {}",
        r.stdout
    );
}

#[test]
fn run_rejects_unknown_options_and_missing_values() {
    let dir = workdir("usage");
    let pkg = script_pkg(&dir, "pkg6", None);

    let r = tebako_run(&[pkg.to_str().unwrap(), "--frobnicate"]);
    assert_eq!(r.rc, 1);
    assert!(
        r.stderr.contains("unknown run option"),
        "stderr: {}",
        r.stderr
    );

    let r = tebako_run(&[pkg.to_str().unwrap(), "--jail"]);
    assert_eq!(r.rc, 1);
    assert!(
        r.stderr.contains("requires a value"),
        "stderr: {}",
        r.stderr
    );
}
