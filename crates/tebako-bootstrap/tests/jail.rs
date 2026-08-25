//! Jail application e2e (spec 08 §2/§4): the bootstrap composes the
//! package's `jail:` request (type-2 block) with the user's TEBAKO_JAIL
//! tightening — manifest request ∩ user policy, the user TIGHTENS, never
//! loosens — and hands the effective policy to the runtime driver through
//! the policy env (TEBAKO_JAIL + TEBAKO_JAIL_SOURCE + TEBAKO_JAIL_JOURNAL,
//! spec 17 §2). A fake runtime echoes the env it receives; enforcement of
//! the env form itself is the tfs/preload suites' proof.

// The harness is shared with selftest/progress/chain; this suite uses a
// subset of its API — silence per-binary unused-function warnings.
#[allow(dead_code)]
mod harness;

use std::path::{Path, PathBuf};

use harness::{rust_bootstrap, Harness};

/// A fake runtime that reports the jail env the bootstrap exported:
/// the package's own stub binary (it prints JAIL= JAIL-SOURCE=
/// JAIL-JOURNAL= among its probe lines). A compiled stub is what
/// CreateProcess can run on Windows — a /bin/sh script is not.
fn write_probe_runtime(h: &Harness, home: &Path) {
    let exe = h.cache_exe(home);
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::copy(harness::fake_runtime_path(), &exe).unwrap();
}

fn package_manifest(runtime_ref: &str, jail: tpkg::HostJail) -> tpkg::PackageManifest {
    tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: "jailtest".to_string(),
            version: "1.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.16.0".to_string(),
            },
            created: "2026-07-27T00:00:00Z".to_string(),
        },
        entries: vec![tpkg::PackageEntry {
            name: "jailtest".to_string(),
            slot: Some(0),
            entrypoint: "jailtest".to_string(),
            runtime_ref: runtime_ref.to_string(),
        }],
        jail: Some(jail),
        env: Default::default(),
        lock: None,
        mounts: Vec::new(),
    }
}

/// The harness stitch, plus an optional type-2 package manifest block.
fn stitch_pkg(h: &Harness, name: &str, jail: Option<tpkg::HostJail>) -> PathBuf {
    let out = h.tmp.0.join(name);
    let mut m = tpkg::Manifest {
        package_flags: 0,
        launcher_abi: 0,
        ..Default::default()
    };
    m.set_runtime_ref(h.runtime_ref.as_bytes());
    let img = h.fake_image();
    let mut pos = std::fs::metadata(&h.bootstrap).unwrap().len();
    let size = std::fs::metadata(&img).unwrap().len();
    let slot = tpkg::Slot::new(pos, size, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__");
    pos += size;
    let _ = pos;
    m.slots.push(slot);
    if let Some(jail) = jail {
        m.set_package_manifest(&package_manifest(&h.runtime_ref, jail))
            .unwrap();
    }
    let mut outf = std::fs::File::create(&out).unwrap();
    {
        use std::io::Write as _;
        outf.write_all(&std::fs::read(&h.bootstrap).unwrap())
            .unwrap();
        outf.write_all(&std::fs::read(&img).unwrap()).unwrap();
    }
    tpkg::write_to(&mut outf, &m).unwrap();
    drop(outf);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    out
}

fn line<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .unwrap_or_else(|| panic!("{key} missing from stdout:\n{stdout}"))
}

#[test]
fn package_jail_is_applied_via_the_policy_env() {
    let h = Harness::new(rust_bootstrap());
    let home = h.home("home1");
    write_probe_runtime(&h, &home);
    let input = h.tmp.0.join("input.csv");
    std::fs::write(&input, b"a,b\n").unwrap();
    let pkg = stitch_pkg(
        &h,
        "pkg-deny-arg",
        Some(tpkg::HostJail::deny_with_arg_files()),
    );

    let (rc, stdout, _stderr) = h.run(&pkg, &home, &[], &[input.to_str().unwrap()]);
    assert_eq!(rc, 0);
    assert_eq!(line(&stdout, "JAIL="), format!("deny;@{}", input.display()));
    assert_eq!(line(&stdout, "JAIL-SOURCE="), "manifest");
    assert_eq!(
        line(&stdout, "JAIL-JOURNAL="),
        home.join("journal.log").to_string_lossy()
    );
}

#[test]
fn user_tebako_jail_intersects_never_loosens() {
    let h = Harness::new(rust_bootstrap());
    let home = h.home("home2");
    write_probe_runtime(&h, &home);
    let mut package = tpkg::HostJail::deny();
    package.mounts.push(tpkg::JailMount {
        host: "/data".to_string(),
        mount: "/data".to_string(),
        access: tpkg::JailAccess::Ro,
    });
    let pkg = stitch_pkg(&h, "pkg-deny-data", Some(package));

    // The user tightens to --no-host: the request's grant is dropped.
    let (rc, stdout, _stderr) = h.run(&pkg, &home, &[("TEBAKO_JAIL", "deny")], &[]);
    assert_eq!(rc, 0);
    assert_eq!(line(&stdout, "JAIL="), "deny");
    assert_eq!(line(&stdout, "JAIL-SOURCE="), "manifest+user");

    // The user asking for open does NOT loosen the manifest's deny
    // (tighter wins; wider latitude comes by trust policy, never a flag).
    let (rc, stdout, _stderr) = h.run(&pkg, &home, &[("TEBAKO_JAIL", "open")], &[]);
    assert_eq!(rc, 0);
    assert_eq!(line(&stdout, "JAIL="), "deny;/data:/data:ro");
    assert_eq!(line(&stdout, "JAIL-SOURCE="), "manifest+user");
}

#[test]
fn user_tebako_jail_alone_when_the_package_is_silent() {
    let h = Harness::new(rust_bootstrap());
    let home = h.home("home3");
    write_probe_runtime(&h, &home);
    let pkg = stitch_pkg(&h, "pkg-silent", None);

    let (rc, stdout, _stderr) = h.run(&pkg, &home, &[("TEBAKO_JAIL", "deny;/etc:/etc:ro")], &[]);
    assert_eq!(rc, 0);
    assert_eq!(line(&stdout, "JAIL="), "deny;/etc:/etc:ro");
    assert_eq!(line(&stdout, "JAIL-SOURCE="), "user");
}

#[test]
fn no_policy_anywhere_exports_nothing() {
    let h = Harness::new(rust_bootstrap());
    let home = h.home("home4");
    write_probe_runtime(&h, &home);
    let pkg = stitch_pkg(&h, "pkg-legacy", None);

    let (rc, stdout, _stderr) = h.run(&pkg, &home, &[], &[]);
    assert_eq!(rc, 0);
    // Byte-identical legacy behavior: no jail env reaches the driver.
    assert_eq!(line(&stdout, "JAIL="), "UNSET");
    assert_eq!(line(&stdout, "JAIL-SOURCE="), "UNSET");
    assert_eq!(line(&stdout, "JAIL-JOURNAL="), "UNSET");
}

#[test]
fn malformed_tebako_jail_fails_closed() {
    let h = Harness::new(rust_bootstrap());
    let home = h.home("home5");
    write_probe_runtime(&h, &home);
    let pkg = stitch_pkg(&h, "pkg-badenv", None);

    let (rc, _stdout, stderr) = h.run(&pkg, &home, &[("TEBAKO_JAIL", "frob")], &[]);
    assert_eq!(rc, tebako_bootstrap::EX_TEBAKO_JAIL as i32);
    assert!(stderr.contains("invalid jail spec"), "stderr: {stderr}");
}

#[test]
fn auto_allowed_resolves_only_existing_argv_files() {
    let h = Harness::new(rust_bootstrap());
    let home = h.home("home6");
    write_probe_runtime(&h, &home);
    let input = h.tmp.0.join("real.csv");
    std::fs::write(&input, b"x\n").unwrap();
    let pkg = stitch_pkg(&h, "pkg-auto", Some(tpkg::HostJail::deny_with_arg_files()));

    let (rc, stdout, _stderr) = h.run(
        &pkg,
        &home,
        &[],
        &[input.to_str().unwrap(), "missing-file-xyz", "--verbose"],
    );
    assert_eq!(rc, 0);
    // Only the existing file is granted (flags and missing names skipped).
    assert_eq!(line(&stdout, "JAIL="), format!("deny;@{}", input.display()));
}

// ---------------------------------------------------------------------
// The whole-chain confinement proof: pressed jail → bootstrap compose →
// driver policy env → the preload shim ENFORCES + JOURNALS (spec 08 §2)
// ---------------------------------------------------------------------

fn preload_dylib() -> Option<PathBuf> {
    let name = if cfg!(target_os = "macos") {
        "libtfs_preload.dylib"
    } else {
        "libtfs_preload.so"
    };
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        for cand in [
            target.join(profile).join(name),
            target.join(profile).join("deps").join(name),
        ] {
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn cc_available() -> bool {
    std::process::Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A driver that honors the policy env through the preload shim (the spec
/// 17 §2 contract): everything before `--tebako-entry` is the loader's;
/// the user args ride to a preload-injected native tool.
fn write_enforcing_runtime(h: &Harness, home: &Path, shim: &Path, tool: &Path) {
    let exe = h.cache_exe(home);
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    let preload_var = if cfg!(target_os = "macos") {
        "DYLD_INSERT_LIBRARIES"
    } else {
        "LD_PRELOAD"
    };
    std::fs::write(
        &exe,
        format!(
            "#!/bin/sh\n\
             while [ \"$1\" != \"--tebako-entry\" ]; do shift; done\n\
             shift\n\
             shift\n\
             exec env {preload_var}={} {} \"$@\"\n",
            shim.display(),
            tool.display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Compile the shared print-data proof tool (exit code = errno).
fn compile_print_data(dir: &Path) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../libtfs-preload/tests/fixtures/print-data.c")
        .canonicalize()
        .unwrap();
    let out = dir.join("print-data");
    let o = std::process::Command::new("cc")
        .arg("-O2")
        .arg("-o")
        .arg(&out)
        .arg(&src)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "cc failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    out
}

/// Pressed with `--jail deny`, the package RUNS CONFINED: a host read
/// fails EPERM end-to-end (bootstrap compose → driver env → preload
/// enforcement), and the violation lands in the audit journal with the
/// manifest source. Pressed with `deny:arg`, the argument file is the one
/// allowed read (the auto-allowed grant enforced through the shim).
/// Skip policy (documented): no `cc` or no built preload shim.
#[test]
fn pressed_jail_package_runs_confined_and_journals() {
    let Some(shim) = preload_dylib() else {
        eprintln!("skip: libtfs_preload cdylib not built");
        return;
    };
    if !cc_available() {
        eprintln!("skip: no C compiler on PATH");
        return;
    }
    let h = Harness::new(rust_bootstrap());
    let home = h.home("home7");
    let tool = compile_print_data(&h.tmp.0);
    write_enforcing_runtime(&h, &home, &shim, &tool);
    let pkg = stitch_pkg(&h, "pkg-confined", Some(tpkg::HostJail::deny()));

    // Outside every grant: EPERM, end to end.
    let (rc, _stdout, _stderr) = h.run(&pkg, &home, &[], &["/etc/hosts"]);
    assert_eq!(rc, libc::EPERM);

    // …and the violation is in the tebako audit journal of the run's home
    // (the bootstrap exported TEBAKO_JAIL_JOURNAL; the shim's tfs layer
    // journaled the denial with the manifest source). Assert the SPECIFIC
    // probe, not the global count: under a deny-all jail the runtime may
    // legitimately trip extra denials from environment lookups (e.g.
    // /etc/localtime on hosts without TZ set — seen on CI runners).
    let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
    let deny_hosts: Vec<&str> = journal
        .lines()
        .filter(|l| {
            l.contains("event=jail-deny") && l.contains("path=/etc/hosts op=read source=manifest")
        })
        .collect();
    assert_eq!(deny_hosts.len(), 1, "journal:\n{journal}");
    let (ts, rest) = deny_hosts[0].split_once(' ').unwrap();
    assert!(!ts.is_empty() && ts.bytes().all(|b| b.is_ascii_digit()));
    assert_eq!(
        rest,
        "event=jail-deny path=/etc/hosts op=read source=manifest"
    );

    // The argument file is the one allowed read (deny:arg auto-allowed,
    // enforced — not just composed).
    let pkg_arg = stitch_pkg(
        &h,
        "pkg-confined-arg",
        Some(tpkg::HostJail::deny_with_arg_files()),
    );
    let input = h.tmp.0.join("input.csv");
    std::fs::write(&input, b"CONFINED-OK\n").unwrap();
    let (rc, stdout, _stderr) = h.run(&pkg_arg, &home, &[], &[input.to_str().unwrap()]);
    assert_eq!(rc, 0);
    assert_eq!(stdout, "CONFINED-OK\n");
}
