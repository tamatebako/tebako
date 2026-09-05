//! The composition spectrum e2e (spec 23 §13, spec 19 §6.1, tebako#458):
//! the bootstrap runs the L2 lock — a self-contained package stages its
//! carried runtime exe from its trailer slot, hands the env image to the
//! driver as `<package>:<slot>`, resolves shared slices by their locked
//! digest, and lazily seeds the machine cache (spec 05 §4). The fake
//! runtime echoes TEBAKO_RUNTIME_IMAGE and its argv, so the wire forms
//! are asserted byte-exact.

// The harness is shared with selftest/progress/chain/jail; this suite
// uses a subset of its API — silence per-binary unused-function warnings.
#[allow(dead_code)]
mod harness;

use std::path::{Path, PathBuf};

use harness::{rust_bootstrap, sha256_of, Harness};

const APP_VER: &str = "1.2.3";
const SHARED_VER: &str = "2.0.0";

fn pin(path: &Path) -> tpkg::DigestPin {
    tpkg::DigestPin::One(sha256_of(path))
}

/// The package manifest of a composed package: one `mnconvert` entry on
/// slot 0 plus the lock under test.
fn composed_pm(runtime_ref: &str, lock: tpkg::PackageLock) -> tpkg::PackageManifest {
    tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: "mnconvert".to_string(),
            version: APP_VER.to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.16.0".to_string(),
            },
            created: "2026-08-25T00:00:00Z".to_string(),
        },
        entries: vec![tpkg::PackageEntry {
            name: "mnconvert".to_string(),
            slot: Some(0),
            entrypoint: "mnconvert".to_string(),
            runtime_ref: runtime_ref.to_string(),
        }],
        jail: None,
        env: Default::default(),
        lock: Some(lock),
        mounts: Vec::new(),
    }
}

/// The harness stitch, generalized: explicit parts + a type-2 package
/// manifest block + explicit package flags.
fn stitch_composed(
    h: &Harness,
    name: &str,
    parts: &[(PathBuf, u32, &str)],
    runtime_ref: &str,
    pm: &tpkg::PackageManifest,
    package_flags: u32,
) -> PathBuf {
    let out = h.tmp.0.join(name);
    let mut m = tpkg::Manifest {
        package_flags,
        launcher_abi: 0,
        ..Default::default()
    };
    m.set_runtime_ref(runtime_ref.as_bytes());
    let mut pos = std::fs::metadata(&h.bootstrap).unwrap().len();
    for (path, format_id, mount) in parts {
        let size = std::fs::metadata(path).unwrap().len();
        let slot = tpkg::Slot::new(pos, size, *format_id, mount);
        m.slots.push(slot);
        pos += size;
    }
    m.set_package_manifest(pm).unwrap();
    let mut outf = std::fs::File::create(&out).unwrap();
    {
        use std::io::Write as _;
        outf.write_all(&std::fs::read(&h.bootstrap).unwrap())
            .unwrap();
        for (p, _, _) in parts {
            outf.write_all(&std::fs::read(p).unwrap()).unwrap();
        }
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

/// A self-contained package (spec 23 §13.2): app slice on slot 0, the
/// carried runtime pair on slots 1 (exe) and 2 (env image) — exe bytes
/// are the fake-runtime stub, so a successful boot prints its markers.
/// Returns (package path, env-image bytes path, app image path).
fn self_contained_pkg(h: &Harness, name: &str, package_flags: u32) -> (PathBuf, PathBuf, PathBuf) {
    let app = h.fake_image();
    let env_image = h.tmp.0.join(format!("{name}-env.tfs"));
    std::fs::write(&env_image, b"FAKE RUNTIME ENV IMAGE").unwrap();
    let runtime_ref = format!("{};image", h.runtime_ref);
    let lock = tpkg::PackageLock {
        runtime: Some(tpkg::LockedRuntime {
            version: harness::RUBY_VER.to_string(),
            carry: true,
            exe: Some(tpkg::LockedArtifact {
                slot: 1,
                sha256: pin(&h.fake_runtime),
                install_as: None,
            }),
            image: Some(tpkg::LockedArtifact {
                slot: 2,
                sha256: pin(&env_image),
                install_as: None,
            }),
            dll: None,
        }),
        slices: vec![tpkg::LockedSlice {
            name: "mnconvert".to_string(),
            version: APP_VER.to_string(),
            carry: true,
            slot: Some(0),
            mount: Some("/__tfs__".to_string()),
            sha256: pin(&app),
            source: None,
        }],
        spawned: vec![],
    };
    let pm = composed_pm(&runtime_ref, lock);
    let pkg = stitch_composed(
        h,
        name,
        &[
            (app.clone(), tpkg::TPKG_FORMAT_DWARFS, "/__tfs__"),
            (h.fake_runtime.clone(), tpkg::TPKG_FORMAT_AUTO, ""),
            (env_image.clone(), tpkg::TPKG_FORMAT_DWARFS, ""),
        ],
        &runtime_ref,
        &pm,
        package_flags,
    );
    (pkg, env_image, app)
}

fn journal_of(home: &Path) -> String {
    std::fs::read_to_string(home.join("journal.log")).unwrap_or_default()
}

/// The payload cache entry a seed would have written.
fn cached_slice(home: &Path, name: &str, version: &str) -> PathBuf {
    home.join("payloads")
        .join(name)
        .join(format!("{version}.tfs"))
}

/// spec 19 §6.1 / spec 23 §13.2: a self-contained package boots with an
/// EMPTY cache and NO network — the carried exe stages into the runtime
/// cache, the env image rides to the driver as `<package>:2`, the app
/// slice mounts from slot 0, and both carried artifacts seed the cache
/// (spec 05 §4), journaled.
#[test]
fn the_carried_package_boots_offline_and_seeds_the_cache() {
    let h = Harness::new(rust_bootstrap());
    let (pkg, env_image, app) = self_contained_pkg(&h, "mnconvert-sc", 0);
    let home = h.home("home");

    let (rc, out, _err) = h.run(
        &pkg,
        &home,
        &[("TEBAKO_OFFLINE", "1")],
        &["mnconvert", "doc.xml"],
    );
    assert_eq!(rc, 0, "stdout:\n{out}");
    assert!(out.contains("FAKE-RUNTIME"), "stdout:\n{out}");
    // The bootstrap canonicalizes its own path (run()'s current_exe).
    let canon = pkg.canonicalize().unwrap();
    // The env image slot form (spec 17 §2.1) — the driver is handed the
    // package's own path plus the slot number, never an extracted copy.
    assert!(
        out.contains(&format!("TEBAKO_RUNTIME_IMAGE={}:2", canon.display())),
        "stdout:\n{out}"
    );
    assert!(
        out.contains(&format!("argv[1]={}:0:/__tfs__", canon.display())),
        "stdout:\n{out}"
    );
    // argv[3] is the entrypoint; the user's argv (mnconvert doc.xml) follows.
    assert!(out.contains("argv[4]=mnconvert"), "stdout:\n{out}");
    assert!(out.contains("argv[5]=doc.xml"), "stdout:\n{out}");

    // The staged exe IS the cache entry (spec 19 §6.1).
    assert!(h.cache_exe(&home).is_file());
    // The lazy seed: the env image in the runtime cache entry, the
    // carried app slice in the payload cache — both journaled.
    assert!(h.cache_image(&home).is_file());
    let marker = std::fs::read_to_string(format!("{}.sha256", h.cache_image(&home).display()))
        .expect("the seeded image carries its trust anchor");
    assert!(
        marker.starts_with(&sha256_of(&env_image)),
        "marker: {marker}"
    );
    let seeded = cached_slice(&home, "mnconvert", APP_VER);
    assert!(seeded.is_file());
    let anchor =
        std::fs::read_to_string(format!("{}.sha256", seeded.display())).expect("slice anchor");
    assert!(anchor.starts_with(&sha256_of(&app)), "anchor: {anchor}");
    let journal = journal_of(&home);
    assert!(
        journal.contains("event=lazy-seed artifact=mnconvert@1.2.3"),
        "journal:\n{journal}"
    );
    assert!(
        journal.contains("event=lazy-seed artifact="),
        "journal:\n{journal}"
    );
}

/// spec 23 §13.1: a shared slice resolves at first run BY THE LOCKED
/// DIGEST into the payload cache and mounts from the cache file as
/// `<path>:-:<mount>`; a warm cache then serves offline.
#[test]
fn the_shared_slice_resolves_mid_run_by_its_locked_digest() {
    let h = Harness::new(rust_bootstrap());
    let app = h.fake_image();
    let shared_file = h.tmp.0.join("mn2pdf-2.0.0.tfs");
    std::fs::write(&shared_file, b"FAKE MN2PDF SLIDE IMAGE").unwrap();
    let runtime_ref = format!("{};image", h.runtime_ref);
    let lock = tpkg::PackageLock {
        runtime: Some(tpkg::LockedRuntime {
            version: harness::RUBY_VER.to_string(),
            carry: false,
            exe: None,
            image: None,
            dll: None,
        }),
        slices: vec![
            tpkg::LockedSlice {
                name: "mnconvert".to_string(),
                version: APP_VER.to_string(),
                carry: true,
                slot: Some(0),
                mount: Some("/__tfs__".to_string()),
                sha256: pin(&app),
                source: None,
            },
            tpkg::LockedSlice {
                name: "mn2pdf".to_string(),
                version: SHARED_VER.to_string(),
                carry: false,
                slot: None,
                mount: Some("/__tfs__/mnt/mn2pdf".to_string()),
                sha256: pin(&shared_file),
                source: Some(tebako_http::file_url(&shared_file)),
            },
        ],
        spawned: vec![],
    };
    let pm = composed_pm(&runtime_ref, lock);
    let pkg = stitch_composed(
        &h,
        "mnconvert-lean",
        &[(app, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__")],
        &runtime_ref,
        &pm,
        0,
    );
    let home = h.home("home");

    let (rc, out, err) = h.run(&pkg, &home, &[], &["mnconvert", "in.xml"]);
    assert_eq!(rc, 0, "stdout:\n{out}\nstderr:\n{err}");
    // The runtime pair lean-resolved from the mirror into the cache.
    assert!(
        out.contains(&format!(
            "TEBAKO_RUNTIME_IMAGE={}",
            h.cache_image(&home).display()
        )),
        "stdout:\n{out}"
    );
    // Mount order is the lock's slice order: the carried app slice, then
    // the shared slice's cache file as a bare image (slot `-`). The
    // package path rides canonicalized (run()'s current_exe).
    let canon = pkg.canonicalize().unwrap();
    let cached = cached_slice(&home, "mn2pdf", SHARED_VER);
    assert!(
        out.contains(&format!("argv[1]={}:0:/__tfs__", canon.display())),
        "stdout:\n{out}"
    );
    assert!(
        out.contains(&format!(
            "argv[3]={}:-:/__tfs__/mnt/mn2pdf",
            cached.display()
        )),
        "stdout:\n{out}"
    );
    assert!(cached.is_file());

    // Second run, cold network: the cache serves both the runtime pair
    // and the shared slice.
    let (rc, out, err) = h.run(&pkg, &home, &[("TEBAKO_OFFLINE", "1")], &["mnconvert"]);
    assert_eq!(rc, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("FAKE-RUNTIME"), "stdout:\n{out}");
}

/// spec 19 §6.1's convergence: the staged exe + the seeded env image ARE
/// the cache entry — a later shared-runtime package on the same machine
/// boots offline from what the self-contained run left behind.
#[test]
fn the_seeded_runtime_pair_serves_a_shared_runtime_package_offline() {
    let h = Harness::new(rust_bootstrap());
    let (pkg, _, _) = self_contained_pkg(&h, "mnconvert-sc", 0);
    let home = h.home("home");
    let (rc, out, _err) = h.run(&pkg, &home, &[], &[]);
    assert_eq!(rc, 0, "stdout:\n{out}");

    // A plain lean package (no lock, no type-2 block) on the same home,
    // fully offline: the seeded cache must serve it without the mirror.
    let lean = h.lean_pkg_image("mnconvert-lean");
    let (rc, out, err) = h.run(&lean, &home, &[("TEBAKO_OFFLINE", "1")], &["mnconvert"]);
    assert_eq!(rc, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(
        out.contains(&format!(
            "TEBAKO_RUNTIME_IMAGE={}",
            h.cache_image(&home).display()
        )),
        "stdout:\n{out}"
    );
}

/// spec 23 §13.4: run-time resolution verifies against the lock,
/// fail-closed — carried bytes that no longer match the press-time pin
/// are a named exit 70, and the cache is not touched.
#[test]
fn a_tampered_carried_exe_fails_closed_with_70() {
    let h = Harness::new(rust_bootstrap());
    // A package pressed with a pin that matches OTHER bytes (the app
    // image): the exe slot's content no longer matches the lock.
    let app = h.fake_image();
    let env_image = h.tmp.0.join("tampered-env.tfs");
    std::fs::write(&env_image, b"FAKE RUNTIME ENV IMAGE").unwrap();
    let runtime_ref = format!("{};image", h.runtime_ref);
    let lock = tpkg::PackageLock {
        runtime: Some(tpkg::LockedRuntime {
            version: harness::RUBY_VER.to_string(),
            carry: true,
            exe: Some(tpkg::LockedArtifact {
                slot: 1,
                sha256: pin(&app), // wrong on purpose: the exe slot's bytes differ
                install_as: None,
            }),
            image: Some(tpkg::LockedArtifact {
                slot: 2,
                sha256: pin(&env_image),
                install_as: None,
            }),
            dll: None,
        }),
        slices: vec![],
        spawned: vec![],
    };
    let pm = composed_pm(&runtime_ref, lock);
    let pkg = stitch_composed(
        &h,
        "mnconvert-tampered",
        &[
            (app, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__"),
            (h.fake_runtime.clone(), tpkg::TPKG_FORMAT_AUTO, ""),
            (env_image, tpkg::TPKG_FORMAT_DWARFS, ""),
        ],
        &runtime_ref,
        &pm,
        0,
    );
    let home = h.home("home");

    let (rc, out, err) = h.run(&pkg, &home, &[], &["mnconvert"]);
    assert_eq!(rc, 70, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("SHA256 mismatch"), "stderr:\n{err}");
    assert!(err.contains("lock pin"), "stderr:\n{err}");
    assert!(!h.cache_exe(&home).exists(), "the cache was not touched");
}

/// The same fail-closed rule on the shared path: registry bytes that do
/// not match the lock pin are a named exit 70 (never a silent re-fetch).
#[test]
fn a_shared_slice_mismatching_its_lock_pin_fails_closed_with_70() {
    let h = Harness::new(rust_bootstrap());
    let app = h.fake_image();
    let shared_file = h.tmp.0.join("mn2pdf-evil.tfs");
    std::fs::write(&shared_file, b"NOT THE PRESSED BYTES").unwrap();
    let runtime_ref = format!("{};image", h.runtime_ref);
    let lock = tpkg::PackageLock {
        runtime: Some(tpkg::LockedRuntime {
            version: harness::RUBY_VER.to_string(),
            carry: false,
            exe: None,
            image: None,
            dll: None,
        }),
        slices: vec![
            tpkg::LockedSlice {
                name: "mnconvert".to_string(),
                version: APP_VER.to_string(),
                carry: true,
                slot: Some(0),
                mount: Some("/__tfs__".to_string()),
                sha256: pin(&app),
                source: None,
            },
            tpkg::LockedSlice {
                name: "mn2pdf".to_string(),
                version: SHARED_VER.to_string(),
                carry: false,
                slot: None,
                mount: Some("/__tfs__/mnt/mn2pdf".to_string()),
                sha256: pin(&app), // wrong on purpose
                source: Some(tebako_http::file_url(&shared_file)),
            },
        ],
        spawned: vec![],
    };
    let pm = composed_pm(&runtime_ref, lock);
    let pkg = stitch_composed(
        &h,
        "mnconvert-evil",
        &[(app, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__")],
        &runtime_ref,
        &pm,
        0,
    );
    let home = h.home("home");

    let (rc, out, err) = h.run(&pkg, &home, &[], &["mnconvert"]);
    assert_eq!(rc, 70, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("SHA256 mismatch"), "stderr:\n{err}");
    assert!(!cached_slice(&home, "mn2pdf", SHARED_VER).exists());
}

/// spec 23 §13.3: a per-triplet digest map that does not cover the host
/// is a named coverage error (exit 65), never a nearest-platform guess.
#[test]
fn a_lock_without_host_coverage_is_a_named_65() {
    let h = Harness::new(rust_bootstrap());
    let app = h.fake_image();
    let other = tpkg::Platform::ALL
        .iter()
        .copied()
        .find(|p| *p != tpkg::Platform::host())
        .unwrap();
    let runtime_ref = format!("{};image", h.runtime_ref);
    let lock = tpkg::PackageLock {
        runtime: Some(tpkg::LockedRuntime {
            version: harness::RUBY_VER.to_string(),
            carry: false,
            exe: None,
            image: None,
            dll: None,
        }),
        slices: vec![
            tpkg::LockedSlice {
                name: "mnconvert".to_string(),
                version: APP_VER.to_string(),
                carry: true,
                slot: Some(0),
                mount: Some("/__tfs__".to_string()),
                sha256: pin(&app),
                source: None,
            },
            tpkg::LockedSlice {
                name: "mn2pdf".to_string(),
                version: SHARED_VER.to_string(),
                carry: false,
                slot: None,
                mount: Some("/__tfs__/mnt/mn2pdf".to_string()),
                sha256: tpkg::DigestPin::PerTriplet(std::collections::BTreeMap::from([(
                    other.release_asset_name().to_string(),
                    sha256_of(&app),
                )])),
                source: Some("file:///nonexistent.tfs".to_string()),
            },
        ],
        spawned: vec![],
    };
    let pm = composed_pm(&runtime_ref, lock);
    let pkg = stitch_composed(
        &h,
        "mnconvert-uncovered",
        &[(app, tpkg::TPKG_FORMAT_DWARFS, "/__tfs__")],
        &runtime_ref,
        &pm,
        0,
    );
    let home = h.home("home");

    let (rc, out, err) = h.run(&pkg, &home, &[], &["mnconvert"]);
    assert_eq!(rc, 65, "stdout:\n{out}\nstderr:\n{err}");
    assert!(
        err.contains("does not cover this platform"),
        "stderr:\n{err}"
    );
    assert!(
        err.contains(tpkg::Platform::host().release_asset_name()),
        "stderr:\n{err}"
    );
}

/// spec 05 §4: TPKG_FLAG_NO_INSTALL packages never seed — the run works
/// standalone and leaves the payload store untouched.
#[test]
fn a_no_install_package_runs_but_never_seeds() {
    let h = Harness::new(rust_bootstrap());
    let (pkg, _, _) = self_contained_pkg(&h, "mnconvert-frozen", tpkg::TPKG_FLAG_NO_INSTALL);
    let home = h.home("home");

    let (rc, out, err) = h.run(&pkg, &home, &[], &["mnconvert"]);
    assert_eq!(rc, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("FAKE-RUNTIME"), "stdout:\n{out}");
    // The run's own exe staging is not a seed; the payload store and the
    // env-image cache copy are.
    assert!(!cached_slice(&home, "mnconvert", APP_VER).exists());
    assert!(!h.cache_image(&home).exists());
    assert!(!journal_of(&home).contains("event=lazy-seed"));
}

/// The pointer-entry mount rule (spec 23 §4): the shared app slice's
/// triple leads the image list — the entrypoint resolves against the
/// FIRST `--tebako-image` mount (spec 17 §1). Asserted at the argv level
/// (the driver never runs here).
#[test]
fn the_pointer_entrys_shared_slice_leads_the_image_list() {
    let mut m = tpkg::Manifest::default();
    m.slots.push(tpkg::Slot::new(
        0,
        100,
        tpkg::TPKG_FORMAT_DWARFS,
        "/__tfs__",
    ));
    m.slots
        .push(tpkg::Slot::new(100, 50, tpkg::TPKG_FORMAT_AUTO, ""));
    m.slots
        .push(tpkg::Slot::new(150, 60, tpkg::TPKG_FORMAT_DWARFS, ""));
    let runtime_ref = "ruby@3.3.7;tebako=9.9.9;image";
    let sha = |c: char| c.to_string().repeat(64);
    let lock = tpkg::PackageLock {
        runtime: Some(tpkg::LockedRuntime {
            version: "3.3.7".to_string(),
            carry: true,
            exe: Some(tpkg::LockedArtifact {
                slot: 1,
                sha256: tpkg::DigestPin::One(sha('c')),
                install_as: None,
            }),
            image: Some(tpkg::LockedArtifact {
                slot: 2,
                sha256: tpkg::DigestPin::One(sha('d')),
                install_as: None,
            }),
            dll: None,
        }),
        slices: vec![
            tpkg::LockedSlice {
                name: "mnconvert".to_string(),
                version: APP_VER.to_string(),
                carry: true,
                slot: Some(0),
                mount: Some("/__tfs__".to_string()),
                sha256: tpkg::DigestPin::One(sha('a')),
                source: None,
            },
            tpkg::LockedSlice {
                name: "mn2pdf".to_string(),
                version: SHARED_VER.to_string(),
                carry: false,
                slot: None,
                mount: Some("/__tfs__/mnt/mn2pdf".to_string()),
                sha256: tpkg::DigestPin::One(sha('b')),
                source: Some("tfs:github:tebako-packages/mn2pdf-feedstock:2.0.0".to_string()),
            },
        ],
        spawned: vec![],
    };
    let pm = tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: "suite".to_string(),
            version: "1.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.16.0".to_string(),
            },
            created: "2026-08-25T00:00:00Z".to_string(),
        },
        entries: vec![
            tpkg::PackageEntry {
                name: "mnconvert".to_string(),
                slot: Some(0),
                entrypoint: "mnconvert".to_string(),
                runtime_ref: runtime_ref.to_string(),
            },
            tpkg::PackageEntry {
                name: "mn2pdf".to_string(),
                slot: None, // the pointer form — backed by the shared slice
                entrypoint: "bin/mn2pdf".to_string(),
                runtime_ref: runtime_ref.to_string(),
            },
        ],
        jail: None,
        env: Default::default(),
        lock: Some(lock),
        mounts: Vec::new(),
    };
    m.set_package_manifest(&pm).unwrap();

    let runtime = Path::new("/rt/ruby");
    let self_path = Path::new("/pkg/suite");
    let cache_path = PathBuf::from("/home/.tebako/payloads/mn2pdf/2.0.0.tfs");
    let shared = vec![("mn2pdf".to_string(), cache_path.clone())];

    // The pointer entry: the shared slice's triple leads; the carried
    // slot-0 image (another entry's) does not mount.
    let selection = tebako_bootstrap::package_selection(&m, "mn2pdf").unwrap();
    let argv = tebako_bootstrap::handoff_argv(
        runtime,
        self_path,
        &m,
        selection.as_ref(),
        &["mn2pdf".to_string()],
        pm.lock.as_ref(),
        &shared,
    );
    assert_eq!(
        argv,
        vec![
            "/rt/ruby".to_string(),
            "--tebako-image".to_string(),
            format!("{}:-:/__tfs__/mnt/mn2pdf", cache_path.display()),
            "--tebako-entry".to_string(),
            "bin/mn2pdf".to_string(),
        ]
    );

    // The primary entry: carried slot 0 first, the shared slice second;
    // the claimed runtime slots (1, 2) never mount.
    let selection = tebako_bootstrap::package_selection(&m, "mnconvert").unwrap();
    let argv = tebako_bootstrap::handoff_argv(
        runtime,
        self_path,
        &m,
        selection.as_ref(),
        &["mnconvert".to_string()],
        pm.lock.as_ref(),
        &shared,
    );
    assert_eq!(
        argv,
        vec![
            "/rt/ruby".to_string(),
            "--tebako-image".to_string(),
            "/pkg/suite:0:/__tfs__".to_string(),
            "--tebako-image".to_string(),
            format!("{}:-:/__tfs__/mnt/mn2pdf", cache_path.display()),
            "--tebako-entry".to_string(),
            "mnconvert".to_string(),
        ]
    );
}

/// spec 30 §2's dispatch half (spec 23 §13.6): a self-contained package
/// whose lock carries a spawned-runtime edge stages the carried pair
/// into the machine runtime cache and hands the driver the spawn lock —
/// `TEBAKO_SPAWN_LOCK=java=21.0.12:2.1.5` — echoed here by the fake
/// runtime. The second run is the cache hit; both run under
/// TEBAKO_OFFLINE (carried staging moves no network bytes). The staged
/// entry is EXACTLY what the driver's spawn planner picks
/// (tpkg::runtime_store::resolve_locked, the spec 30 §3 consumer).
#[test]
fn a_carried_spawned_runtime_dispatches_and_exports_the_lock() {
    let h = Harness::new(rust_bootstrap());
    let app = h.fake_image();
    let env_image = h.tmp.0.join("java-edge-env.tfs");
    std::fs::write(&env_image, b"FAKE RUNTIME ENV IMAGE").unwrap();
    let java_exe = h.tmp.0.join("java-exe");
    std::fs::write(&java_exe, b"FAKE SPAWNED JAVA EXE").unwrap();
    let java_image = h.tmp.0.join("java-image.tfs");
    std::fs::write(&java_image, b"FAKE SPAWNED JAVA IMAGE").unwrap();
    let runtime_ref = format!("{};image", h.runtime_ref);
    let lock = tpkg::PackageLock {
        runtime: Some(tpkg::LockedRuntime {
            version: harness::RUBY_VER.to_string(),
            carry: true,
            exe: Some(tpkg::LockedArtifact {
                slot: 1,
                sha256: pin(&h.fake_runtime),
                install_as: None,
            }),
            image: Some(tpkg::LockedArtifact {
                slot: 2,
                sha256: pin(&env_image),
                install_as: None,
            }),
            dll: None,
        }),
        slices: vec![tpkg::LockedSlice {
            name: "mnconvert".to_string(),
            version: APP_VER.to_string(),
            carry: true,
            slot: Some(0),
            mount: Some("/__tfs__".to_string()),
            sha256: pin(&app),
            source: None,
        }],
        spawned: vec![tpkg::LockedSpawned::Runtime(tpkg::LockedSpawnedRuntime {
            engine: "java".to_string(),
            implementation: None,
            constraint: tpkg::Constraint::new(">= 21, < 26").unwrap(),
            expose: vec!["java".to_string()],
            version: "21.0.12".to_string(),
            tebako: "2.1.5".to_string(),
            carry: true,
            exe: tpkg::LockedSpawnedArtifact {
                slot: Some(3),
                sha256: pin(&java_exe),
                install_as: None,
            },
            image: tpkg::LockedSpawnedArtifact {
                slot: Some(4),
                sha256: pin(&java_image),
                install_as: None,
            },
            dll: None,
            source: None,
        })],
    };
    let pm = composed_pm(&runtime_ref, lock);
    let pkg = stitch_composed(
        &h,
        "mnconvert-java",
        &[
            (app.clone(), tpkg::TPKG_FORMAT_DWARFS, "/__tfs__"),
            (h.fake_runtime.clone(), tpkg::TPKG_FORMAT_AUTO, ""),
            (env_image.clone(), tpkg::TPKG_FORMAT_DWARFS, ""),
            (java_exe.clone(), tpkg::TPKG_FORMAT_AUTO, ""),
            (java_image.clone(), tpkg::TPKG_FORMAT_DWARFS, ""),
        ],
        &runtime_ref,
        &pm,
        0,
    );
    let home = h.home("home");

    for attempt in 0..2 {
        let (rc, out, err) = h.run(
            &pkg,
            &home,
            &[("TEBAKO_OFFLINE", "1")],
            &["mnconvert", "doc.xml"],
        );
        assert_eq!(rc, 0, "run {attempt} stdout:\n{out}\nstderr:\n{err}");
        assert!(
            out.contains("SPAWN-LOCK=java=21.0.12:2.1.5\n"),
            "run {attempt} stdout:\n{out}"
        );
    }

    // The carried pair landed in the runtime cache, in the store
    // grammar the driver's resolve_locked scans — and the driver's
    // locked pick finds exe + verified image.
    let plat = harness::platform();
    let exe = tebako_bootstrap::platform::exe_suffix();
    let entry = home
        .join("runtimes")
        .join(format!("java-21.0.12-2.1.5-{plat}"));
    assert!(
        entry
            .join(format!("tebako-runtime-2.1.5-21.0.12-{plat}{exe}"))
            .is_file(),
        "the staged exe: {}",
        entry.display()
    );
    let image = entry.join(format!("tebako-runtime-2.1.5-21.0.12-{plat}.tfs"));
    assert!(image.is_file(), "the staged image: {}", entry.display());
    assert!(
        entry
            .join(format!("tebako-runtime-2.1.5-21.0.12-{plat}.tfs.sha256"))
            .is_file(),
        "the image trust marker: {}",
        entry.display()
    );
    let picked = tpkg::runtime_store::resolve_locked(&home, "java", None, "21.0.12", "2.1.5")
        .expect("the staged entry is the driver's spawn pick");
    assert_eq!(
        picked.image.as_deref(),
        Some(image.as_path()),
        "the driver's pick carries the verified image"
    );
}
