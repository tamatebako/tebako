//! Suite dispatch end-to-end (spec 07 §2.0, spec 03 §6): one package with
//! N entries, each entry dispatched by argv0 to ITS slot and ITS runtime
//! — two commands of one package run DIFFERENT cached runtime versions.

#[allow(dead_code)]
mod harness;

use harness::{platform, rust_bootstrap, Harness, TEBAKO_VER};
use std::path::{Path, PathBuf};

const RUBY_A: &str = "3.4.2";
const RUBY_B: &str = "3.3.7";

/// Seed a cached runtime entry whose "runtime" is a shell script printing
/// a per-version marker plus its argv (the handoff triple/entry evidence).
fn seed_cached_runtime(home: &Path, ruby_ver: &str) -> PathBuf {
    let plat = platform();
    let exe = tebako_bootstrap::platform::exe_suffix();
    let dir = home
        .join("runtimes")
        .join(format!("ruby-{ruby_ver}-{TEBAKO_VER}-{plat}"));
    std::fs::create_dir_all(&dir).unwrap();
    let exe_path = dir.join(format!(
        "tebako-runtime-{TEBAKO_VER}-{ruby_ver}-{plat}{exe}"
    ));
    std::fs::write(
        &exe_path,
        format!(
            "#!/bin/sh\necho FAKE-RUNTIME-{ruby_ver}\ni=0\nfor a in \"$@\"; do\n  echo \"argv[$i]=$a\"\n  i=$((i+1))\ndone\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&exe_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    exe_path
}

/// A two-entry suite package: bootstrap + two fake app images in slots
/// 0/1, the type-2 package manifest pinning each entry to its own ruby.
fn suite_pkg(h: &Harness, name: &str) -> PathBuf {
    let out = h.tmp.0.join(name);
    let img_a = h.tmp.0.join("app-a.tfs");
    let img_b = h.tmp.0.join("app-b.tfs");
    std::fs::write(&img_a, b"FAKE TFS IMAGE A").unwrap();
    std::fs::write(&img_b, b"FAKE TFS IMAGE B").unwrap();

    let mut m = tpkg::Manifest {
        package_flags: 0,
        launcher_abi: 0,
        ..Default::default()
    };
    // The trailer field stays for v1-era loaders: entries[0]'s ref.
    m.set_runtime_ref(format!("ruby@{RUBY_A};tebako={TEBAKO_VER}").as_bytes());
    let mut pos = std::fs::metadata(&h.bootstrap).unwrap().len();
    for img in [&img_a, &img_b] {
        let size = std::fs::metadata(img).unwrap().len();
        m.slots.push(tpkg::Slot::new(
            pos,
            size,
            tpkg::TPKG_FORMAT_DWARFS,
            "/__tebako_memfs__",
        ));
        pos += size;
    }
    m.set_package_manifest(&tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: "hellosuite".to_string(),
            version: "1.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: TEBAKO_VER.to_string(),
            },
            created: "2026-07-27T00:00:00Z".to_string(),
        },
        entries: vec![
            tpkg::PackageEntry {
                name: "hello34".to_string(),
                slot: 0,
                entrypoint: "hello34".to_string(),
                runtime_ref: format!("ruby@{RUBY_A};tebako={TEBAKO_VER}"),
            },
            tpkg::PackageEntry {
                name: "hello33".to_string(),
                slot: 1,
                entrypoint: "hello33".to_string(),
                runtime_ref: format!("ruby@{RUBY_B};tebako={TEBAKO_VER}"),
            },
        ],
        jail: None,
        env: Default::default(),
    })
    .unwrap();

    let mut outf = std::fs::File::create(&out).unwrap();
    let mut copy = |p: &Path| {
        let data = std::fs::read(p).unwrap();
        use std::io::Write as _;
        outf.write_all(&data).unwrap();
    };
    copy(&h.bootstrap);
    copy(&img_a);
    copy(&img_b);
    tpkg::write_to(&mut outf, &m).unwrap();
    drop(outf);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    out
}

/// Copy the package under an entry's name: the invocation name (argv0) is
/// the selector — installed suites ship one copy/hardlink per command.
fn as_command(h: &Harness, pkg: &Path, name: &str) -> PathBuf {
    let copy = h.tmp.0.join(name);
    std::fs::copy(pkg, &copy).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    copy
}

#[test]
fn two_commands_run_against_their_own_cached_runtimes() {
    let h = Harness::new(rust_bootstrap());
    let pkg = suite_pkg(&h, "hellosuite");
    let home = h.home("home");
    seed_cached_runtime(&home, RUBY_A);
    seed_cached_runtime(&home, RUBY_B);

    // argv0 = hello34 → entries[0]: slot 0, ruby 3.4.2.
    let hello34 = as_command(&h, &pkg, "hello34");
    let (rc, out, err) = h.run(&hello34, &home, &[], &["world"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains(&format!("FAKE-RUNTIME-{RUBY_A}")), "{out}");
    assert!(!out.contains(&format!("FAKE-RUNTIME-{RUBY_B}")), "{out}");
    // ONLY the entry's slot mounts, and the entrypoint is the entry's.
    assert!(
        out.contains(&format!("{}:0:/__tebako_memfs__", hello34.display())),
        "{out}"
    );
    assert!(!out.contains(&format!("{}:1:", hello34.display())), "{out}");
    let argv: Vec<&str> = out.lines().collect();
    let entry_pos = argv
        .iter()
        .position(|l| l.ends_with("=--tebako-entry"))
        .unwrap_or_else(|| panic!("no --tebako-entry in {out}"));
    assert!(argv[entry_pos + 1].ends_with("=hello34"), "{out}");

    // argv0 = hello33 → entries[1]: slot 1, ruby 3.3.7 — simultaneously
    // resolvable: the two commands of one package use different runtime
    // versions side by side (spec 07 §2.0's case).
    let hello33 = as_command(&h, &pkg, "hello33");
    let (rc, out, err) = h.run(&hello33, &home, &[], &["--pdf", "in.xml"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains(&format!("FAKE-RUNTIME-{RUBY_B}")), "{out}");
    assert!(
        out.contains(&format!("{}:1:/__tebako_memfs__", hello33.display())),
        "{out}"
    );
    assert!(!out.contains(&format!("{}:0:", hello33.display())), "{out}");
    assert!(out.contains("--pdf"), "{out}");

    // argv0 matches NO entry name → entries[0] fallback.
    let bare = as_command(&h, &pkg, "hellosuite-run");
    let (rc, out, err) = h.run(&bare, &home, &[], &[]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains(&format!("FAKE-RUNTIME-{RUBY_A}")), "{out}");
    assert!(
        out.contains(&format!("{}:0:/__tebako_memfs__", bare.display())),
        "{out}"
    );
}

#[test]
fn suite_entry_slot_out_of_range_is_a_named_error() {
    let h = Harness::new(rust_bootstrap());
    let pkg = {
        // a manifest naming a slot the container does not carry
        let out = h.tmp.0.join("badsuite");
        let img = h.fake_image();
        let mut m = tpkg::Manifest {
            package_flags: 0,
            launcher_abi: 0,
            ..Default::default()
        };
        m.set_runtime_ref(format!("ruby@{RUBY_A};tebako={TEBAKO_VER}").as_bytes());
        let size = std::fs::metadata(&img).unwrap().len();
        let pos = std::fs::metadata(&h.bootstrap).unwrap().len();
        m.slots.push(tpkg::Slot::new(
            pos,
            size,
            tpkg::TPKG_FORMAT_DWARFS,
            "/__tebako_memfs__",
        ));
        m.set_package_manifest(&tpkg::PackageManifest {
            schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
            package: tpkg::PackageIdentity {
                name: "badsuite".to_string(),
                version: "1.0.0".to_string(),
                producer: tpkg::Producer {
                    tool: "tebako-cli".to_string(),
                    tool_version: TEBAKO_VER.to_string(),
                },
                created: "2026-07-27T00:00:00Z".to_string(),
            },
            entries: vec![
                tpkg::PackageEntry {
                    name: "a".to_string(),
                    slot: 0,
                    entrypoint: "a".to_string(),
                    runtime_ref: format!("ruby@{RUBY_A};tebako={TEBAKO_VER}"),
                },
                tpkg::PackageEntry {
                    name: "ghost".to_string(),
                    slot: 7,
                    entrypoint: "ghost".to_string(),
                    runtime_ref: format!("ruby@{RUBY_A};tebako={TEBAKO_VER}"),
                },
            ],
            jail: None,
            env: Default::default(),
        })
        .unwrap();
        let mut outf = std::fs::File::create(&out).unwrap();
        use std::io::Write as _;
        outf.write_all(&std::fs::read(&h.bootstrap).unwrap())
            .unwrap();
        outf.write_all(&std::fs::read(&img).unwrap()).unwrap();
        tpkg::write_to(&mut outf, &m).unwrap();
        drop(outf);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        out
    };
    let home = h.home("home2");
    seed_cached_runtime(&home, RUBY_A);
    let ghost = as_command(&h, &pkg, "ghost");
    let (rc, _, err) = h.run(&ghost, &home, &[], &[]);
    assert_eq!(rc, 65, "{err}");
    assert!(err.contains("references slot 7"), "{err}");
    // the fallback entry (slot 0, in range) still runs the same package.
    let a = as_command(&h, &pkg, "a");
    let (rc, out, err) = h.run(&a, &home, &[], &[]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains(&format!("FAKE-RUNTIME-{RUBY_A}")), "{out}");
}

/// The package bytes a reader sees: N slots + the type-2 block, trailer
/// runtime_ref intact for v1-era loaders.
#[test]
fn suite_package_wire_shape() {
    let h = Harness::new(rust_bootstrap());
    let pkg = suite_pkg(&h, "hellosuite-wire");
    let mut f = std::fs::File::open(&pkg).unwrap();
    let m = tpkg::read_from(&mut f).unwrap();
    assert_eq!(m.slots.len(), 2);
    assert_eq!(
        m.runtime_ref_str().unwrap(),
        format!("ruby@{RUBY_A};tebako={TEBAKO_VER}")
    );
    let pm = m.package_manifest().unwrap().unwrap();
    assert_eq!(pm.package.name, "hellosuite");
    assert_eq!(pm.entries.len(), 2);
    assert_eq!(pm.entries[0].name, "hello34");
    assert_eq!(pm.entries[1].name, "hello33");
    assert_eq!(pm.entries[1].slot, 1);
    assert_eq!(
        pm.entries[1].runtime_ref,
        format!("ruby@{RUBY_B};tebako={TEBAKO_VER}")
    );
    // the block is parseable and validates strict.
    m.validate_strict().unwrap();
}
