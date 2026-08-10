//! Cached-runtime compatibility selection (spec 05 §5: range vs
//! abi-line) and the download fallback (bootstrap discipline: flock,
//! tmp+rename, trust markers, read-only image).

mod common;

use common::*;
use tebako_shim::runtime::{self, RuntimeResolution};
use tpkg::{Constraint, RuntimeRequirement};

fn req(constraint: &str) -> RuntimeRequirement {
    RuntimeRequirement {
        engine: "ruby".to_string(),
        constraint: Constraint::new(constraint).expect("test constraint parses"),
        abi: None,
    }
}

fn ready(res: RuntimeResolution) -> runtime::CachedRuntime {
    match res {
        RuntimeResolution::Ready(rt) => rt,
        RuntimeResolution::Zero => panic!("expected a resolved runtime"),
    }
}

#[test]
fn range_constraint_picks_the_newest_cached() {
    let tmp = TempDir::new("range-newest");
    let home = tmp.path().join("home");
    write_runtime(&home, "3.3.5", "0.16.0", false);
    write_runtime(&home, "4.0.6", "0.16.0", false);
    write_runtime(&home, "3.4.2", "0.16.0", false);
    let rt = ready(
        runtime::resolve_runtime(Some(&req(">= 3.3, < 5.0")), true, &ctx(&home, tmp.path()))
            .unwrap(),
    );
    assert_eq!(rt.lang_version, "4.0.6");
}

#[test]
fn abi_line_constraint_locks_to_the_line() {
    let tmp = TempDir::new("abi-line");
    let home = tmp.path().join("home");
    write_runtime(&home, "3.3.5", "0.16.0", false);
    write_runtime(&home, "4.0.6", "0.16.0", false);
    write_runtime(&home, "3.4.2", "0.16.0", false);
    let rt = ready(
        runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx(&home, tmp.path())).unwrap(),
    );
    assert_eq!(rt.lang_version, "3.3.5");
}

#[test]
fn same_language_version_picks_the_newer_tebako_build() {
    let tmp = TempDir::new("tie-break");
    let home = tmp.path().join("home");
    write_runtime(&home, "3.3.7", "0.15.9", false);
    write_runtime(&home, "3.3.7", "0.16.0", false);
    let rt = ready(
        runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx(&home, tmp.path())).unwrap(),
    );
    assert_eq!(rt.lang_version, "3.3.7");
    assert_eq!(
        rt.tebako_version, "0.16.0",
        "a stale runtime must not shadow a fresh build of the same ruby"
    );
}

#[test]
fn no_compatible_cached_offline_is_the_named_compat_error() {
    let tmp = TempDir::new("offline-miss");
    let home = tmp.path().join("home");
    write_runtime(&home, "3.4.2", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert("TEBAKO_OFFLINE".into(), "1".into());
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.9\n    tebako: 0.16.0\n",
    );
    let err = runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("TEBAKO_OFFLINE"), "{}", err.message);
    // the message names the ABI-line semantics — never a segfault
    let err = runtime::resolve_runtime(Some(&req("~> 3.3.0")), false, &ctx).unwrap_err();
    assert!(
        err.message.contains("ABI line") || err.message.contains("~> 3.3.0"),
        "{}",
        err.message
    );
}

#[test]
fn no_runtime_preference_is_a_named_error() {
    let tmp = TempDir::new("no-pref");
    let home = tmp.path().join("home");
    // An empty mirror: the default-line index probe reads nothing (never
    // the real network), so the named refusal is what remains.
    let mirror = tmp.path().join("mirror");
    std::fs::create_dir_all(&mirror).expect("mirror dir");
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let err = runtime::resolve_runtime(Some(&req(">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(
        err.message.contains("runtime preference"),
        "{}",
        err.message
    );
}

#[test]
fn no_preference_downloads_the_index_pick_on_the_default_line() {
    let tmp = TempDir::new("prefless-index-pick");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    let line = tebako_resolve::DEFAULT_TEBAKO_VERSION;
    write_release_index(&mirror, line, &["3.3.7", "3.4.2"]);
    // NO runtime preference in config.yaml — the point of the test: the
    // default-line release index picks the newest satisfier (spec 13 §2a).
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let rt = ready(runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "3.3.7");
    assert_eq!(rt.tebako_version, line);
    assert!(rt.exe.is_file());
}

#[test]
fn no_preference_and_an_index_without_a_satisfier_is_the_platform_error() {
    let tmp = TempDir::new("prefless-unsatisfiable");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_release_index(
        &mirror,
        tebako_resolve::DEFAULT_TEBAKO_VERSION,
        WINDOWS_LINE,
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let err = runtime::resolve_runtime(Some(&req(">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("satisfies"), "{}", err.message);
}

#[test]
fn preference_outside_the_constraint_is_a_named_error() {
    let tmp = TempDir::new("pref-violates");
    let home = tmp.path().join("home");
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let err = runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx(&home, tmp.path()))
        .unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("does not satisfy"), "{}", err.message);
}

#[test]
fn download_installs_verified_readonly_image_and_markers() {
    let tmp = TempDir::new("download");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_mirror(&mirror, "4.0.6", "0.16.0", false);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let rt = ready(runtime::resolve_runtime(Some(&req(">= 3.3, < 5.0")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "4.0.6");
    assert!(rt.exe.is_file());
    let image = rt.image.expect("image-era runtime");
    assert!(image.is_file());
    // trust markers installed
    let marker = image.with_extension("tfs.sha256");
    assert!(marker.is_file(), "marker {}", marker.display());
    assert!(rt.dir.join("sha256").is_file());
    assert!(rt.dir.join("origin").is_file());
    // the image is read-only (0444)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            image.metadata().unwrap().permissions().mode() & 0o777,
            0o444
        );
    }

    // second resolution is a cache hit: the mirror is gone, still Ready
    std::fs::remove_dir_all(&mirror).unwrap();
    let rt2 = ready(runtime::resolve_runtime(Some(&req(">= 3.3, < 5.0")), true, &ctx).unwrap());
    assert_eq!(rt2.lang_version, "4.0.6");
}

#[test]
fn sha_mismatch_is_exit_70_and_nothing_enters_the_cache() {
    let tmp = TempDir::new("sha-mismatch");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_mirror(&mirror, "4.0.6", "0.16.0", true);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let err = runtime::resolve_runtime(Some(&req(">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_SHA);
    assert!(
        !home
            .join("runtimes")
            .join(format!("ruby-4.0.6-0.16.0-{}", platform()))
            .exists(),
        "a failed install must be invisible"
    );
}

#[test]
fn download_installs_the_dll_as_install_as_with_markers() {
    // tebako-runtime-ruby#40: a windows release declares the ruby DLL in
    // the additive `dll` key — it installs next to the exe under its PE
    // name (`install_as`), verified and marked like the image.
    let tmp = TempDir::new("download-dll");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    let (_, install_as) = write_mirror_dll(&mirror, "3.3.12", "0.16.3", false);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.12\n    tebako: 0.16.3\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let rt = ready(runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "3.3.12");
    assert!(rt.exe.is_file());
    // the dll materializes under its PE name — never the asset name
    let dll = rt.dir.join(&install_as);
    assert!(dll.is_file(), "{}", dll.display());
    let asset_name = rt
        .dir
        .join(format!("tebako-runtime-0.16.3-3.3.12-{}.dll", platform()));
    assert!(
        !asset_name.exists(),
        "the asset name is not the install name"
    );
    // trust markers installed
    let marker = rt.dir.join(format!("{install_as}.sha256"));
    assert!(marker.is_file(), "marker {}", marker.display());
    let text = std::fs::read_to_string(&marker).unwrap();
    assert_eq!(
        text,
        format!("{}  {install_as}\n", sha256_hex(b"mirrored ruby dll\n"))
    );
    let origin = std::fs::read_to_string(rt.dir.join(format!("{install_as}.origin"))).unwrap();
    assert!(
        origin.contains(&format!("tebako-runtime-0.16.3-3.3.12-{}.dll", platform())),
        "{origin}"
    );
    // the dll is read-only (0444)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(dll.metadata().unwrap().permissions().mode() & 0o777, 0o444);
    }

    // second resolution is a cache hit: the mirror is gone, still Ready
    std::fs::remove_dir_all(&mirror).unwrap();
    let rt2 = ready(runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx).unwrap());
    assert_eq!(rt2.lang_version, "3.3.12");
}

#[test]
fn dll_sha_mismatch_is_exit_70_and_nothing_enters_the_cache() {
    let tmp = TempDir::new("dll-sha-mismatch");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_mirror_dll(&mirror, "3.3.12", "0.16.3", true);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.12\n    tebako: 0.16.3\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let err = runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_SHA);
    assert!(
        !home
            .join("runtimes")
            .join(format!("ruby-3.3.12-0.16.3-{}", platform()))
            .exists(),
        "a failed install must be invisible"
    );
}

#[test]
fn a_release_without_the_dll_key_installs_the_exe_alone() {
    // the additive-key compat rule: a contract-complete release with no
    // `dll` key (every POSIX release) installs the exe alone.
    let tmp = TempDir::new("download-nodll");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_mirror(&mirror, "4.0.6", "0.16.0", false);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let rt = ready(runtime::resolve_runtime(Some(&req(">= 3.3, < 5.0")), true, &ctx).unwrap());
    assert!(rt.exe.is_file());
    assert!(
        !std::fs::read_dir(&rt.dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().ends_with(".dll")),
        "no dll facet declared, no dll installed"
    );
}

#[test]
fn a_dll_install_as_with_a_path_separator_is_a_named_error() {
    // the PE name installs a file into the cache entry — a name with a
    // separator would escape it; refuse by name, never install.
    let tmp = TempDir::new("dll-traversal");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_mirror_dll(&mirror, "3.3.12", "0.16.3", false);
    let manifest = mirror.join("v0.16.3").join("manifest.json");
    let text = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("x64-ucrt-ruby330.dll", "../evil.dll");
    std::fs::write(&manifest, text).unwrap();
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.12\n    tebako: 0.16.3\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let err = runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("bare file name"), "{}", err.message);
    assert!(!home.join("tmp").join("evil.dll").exists());
    assert!(
        !home
            .join("runtimes")
            .join(format!("ruby-3.3.12-0.16.3-{}", platform()))
            .exists(),
        "a refused install must be invisible"
    );
}

#[test]
fn pre_era_release_is_refused_before_download() {
    // spec 18 S11: a release whose manifest declares no contract set
    // (the pre-18 factory shape — contract fields absent) is refused by
    // name, exit 75, before any download. No old-path readers: the v1
    // graceful-degradation era is over.
    let tmp = TempDir::new("pre-era");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    let platform = platform();
    let dir = mirror.join("v9.9.9");
    std::fs::create_dir_all(&dir).expect("mirror dir");
    let exe_name = format!(
        "tebako-runtime-9.9.9-3.3.7-{platform}{}",
        tebako_shim::runtime::exe_suffix()
    );
    let exe_bytes = b"pre-era runtime exe\n";
    std::fs::write(dir.join(&exe_name), exe_bytes).expect("exe");
    std::fs::write(
        dir.join("manifest.json"),
        format!(
            "[{{\"filename\": \"{exe_name}\", \"sha256\": \"{}\"}}]\n",
            sha256_hex(exe_bytes)
        ),
    )
    .expect("manifest.json");
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.7\n    tebako: 9.9.9\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let err = runtime::resolve_runtime(Some(&req(">= 3.3, < 5.0")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_CONTRACT, "{}", err.message);
    assert!(err.message.contains("pre-era"), "{}", err.message);
    assert!(
        !home
            .join("runtimes")
            .join(format!("ruby-3.3.7-9.9.9-{platform}"))
            .exists(),
        "a pre-era runtime entered the cache"
    );
}

#[test]
fn a_newer_declared_contract_is_the_upgrade_refusal() {
    // spec 18 S12: contract_version newer than spoken → exit 75, both
    // numbers named; nothing enters the cache.
    let tmp = TempDir::new("contract-2");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_mirror(&mirror, "4.0.6", "0.16.0", false);
    let manifest = mirror.join("v0.16.0").join("manifest.json");
    let declared2 = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("\"contract_version\": 2", "\"contract_version\": 3");
    std::fs::write(&manifest, declared2).unwrap();
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let err = runtime::resolve_runtime(Some(&req(">= 3.3, < 5.0")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_CONTRACT, "{}", err.message);
    assert!(
        err.message.contains("contract_version 3"),
        "{}",
        err.message
    );
    assert!(err.message.contains("speaks contract 2"), "{}", err.message);
    assert!(
        !home
            .join("runtimes")
            .join(format!("ruby-4.0.6-0.16.0-{}", platform()))
            .exists(),
        "a refused-contract runtime entered the cache"
    );
}

#[test]
fn zero_requirement_skips_resolution_entirely() {
    let tmp = TempDir::new("zero-req");
    let home = tmp.path().join("home");
    // no runtimes dir, no mirror, no config — still Zero
    let res = runtime::resolve_runtime(None, true, &ctx(&home, tmp.path())).unwrap();
    assert!(matches!(res, RuntimeResolution::Zero));
}

#[test]
fn a_multi_package_manifest_verifies_against_the_right_entry() {
    let tmp = TempDir::new("multi-pkg");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    let platform = platform();
    let dir = mirror.join("v0.16.0");
    std::fs::create_dir_all(&dir).expect("mirror dir");
    // Two packages in one release index (the factory's real shape): the
    // 3.3.7 pair first, the 4.0.6 pair second. Resolving 3.3.7 must
    // verify against ITS OWN entry — the substring-scan era read the
    // NEXT entry's sha256 for the image asset.
    let mut manifest = String::from("[\n");
    let mut first = true;
    for lv in ["3.3.7", "4.0.6"] {
        let base = format!("tebako-runtime-0.16.0-{lv}-{platform}");
        let exe_name = format!("{base}{}", tebako_shim::runtime::exe_suffix());
        let image_name = format!("{base}.tfs");
        let exe_bytes = format!("{lv} exe\n");
        let image_bytes = format!("{lv} image\n");
        std::fs::write(dir.join(&exe_name), exe_bytes.as_bytes()).expect("exe");
        std::fs::write(dir.join(&image_name), image_bytes.as_bytes()).expect("image");
        if !first {
            manifest.push_str(",\n");
        }
        first = false;
        manifest.push_str(&format!(
            "  {{\"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"filename\": \"{exe_name}\", \"sha256\": \"{}\", \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{}\"}}}}",
            sha256_hex(exe_bytes.as_bytes()),
            sha256_hex(image_bytes.as_bytes())
        ));
    }
    manifest.push_str("\n]\n");
    std::fs::write(dir.join("manifest.json"), manifest).expect("manifest.json");
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.7\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let rt = ready(runtime::resolve_runtime(Some(&req("~> 3.3.0")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "3.3.7");
    assert!(rt.exe.is_file());
    assert!(rt.image.is_some());
}

// ---------------------------------------------------------------------
// the abi line (spec 05 §5: native-extension payloads match the
// runtime's own platform string, orthogonally to the version line)
// ---------------------------------------------------------------------

fn req_abi(constraint: &str, abi: &str) -> RuntimeRequirement {
    RuntimeRequirement {
        engine: "ruby".to_string(),
        constraint: Constraint::new(constraint).expect("test constraint parses"),
        abi: Some(abi.to_string()),
    }
}

#[test]
fn abi_line_filters_cached_runtimes_to_the_matching_platform_string() {
    let tmp = TempDir::new("abi-filter");
    let home = tmp.path().join("home");
    write_runtime_abi(&home, "3.3.7", "0.16.0", Some("arm64-darwin-24"));
    write_runtime_abi(&home, "3.3.7", "0.15.9", Some("arm64-darwin-23"));
    let rt = ready(
        runtime::resolve_runtime(
            Some(&req_abi("~> 3.3.0", "arm64-darwin-23")),
            false,
            &ctx(&home, tmp.path()),
        )
        .unwrap(),
    );
    assert_eq!(rt.tebako_version, "0.15.9");
}

#[test]
fn abi_mismatch_is_a_named_error_with_both_lines() {
    let tmp = TempDir::new("abi-mismatch");
    let home = tmp.path().join("home");
    write_runtime_abi(&home, "3.3.7", "0.16.0", Some("arm64-darwin-24"));
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.7\n    tebako: 0.16.0\n",
    );
    let err = runtime::resolve_runtime(
        Some(&req_abi("~> 3.3.0", "arm64-darwin-23")),
        false,
        &ctx(&home, tmp.path()),
    )
    .unwrap_err();
    assert!(err.message.contains("arm64-darwin-24"), "{}", err.message);
    assert!(err.message.contains("arm64-darwin-23"), "{}", err.message);
}

#[test]
fn a_runtime_without_an_abi_line_stays_eligible() {
    let tmp = TempDir::new("abi-compat");
    let home = tmp.path().join("home");
    // pre-abi release: no manifest.json — the compat window, never a
    // match failure of its own
    write_runtime(&home, "3.3.7", "0.16.0", false);
    let rt = ready(
        runtime::resolve_runtime(
            Some(&req_abi("~> 3.3.0", "arm64-darwin-23")),
            false,
            &ctx(&home, tmp.path()),
        )
        .unwrap(),
    );
    assert_eq!(rt.lang_version, "3.3.7");
}

// ---------------------------------------------------------------------
// the release-index download target (spec 05 §5's download half,
// completed; spec 13 §2a): on a cache miss the index — not only the
// config pin — names the newest RELEASED version satisfying the
// constraint on THIS platform, and a readable index with nothing
// satisfiable is the named platform-availability error, never a bare
// asset 404.
// ---------------------------------------------------------------------

/// The windows-ucrt64 release line as the factory publishes it today:
/// 3.3+ is deferred (the windows native-extension bug), so the platform
/// stops at 3.2.x. The fixture carries the CURRENT platform string —
/// the released-versions line is the point, not the triplet.
const WINDOWS_LINE: &[&str] = &["3.1.6", "3.2.4", "3.2.5", "3.2.6", "3.2.7", "3.2.11"];

#[test]
fn a_constraint_nothing_released_satisfies_is_the_platform_availability_error() {
    let tmp = TempDir::new("index-unsatisfiable");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_release_index(&mirror, "0.16.0", WINDOWS_LINE);
    // the pin satisfies the payload's constraint; the platform's
    // release line does not — availability, not the pin, is the cause.
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.3.7\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let err = runtime::resolve_runtime(Some(&req(">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    let platform = platform();
    assert!(
        err.message.contains(&format!(
            "no released ruby runtime for {platform} satisfies \">= 3.3\""
        )),
        "{}",
        err.message
    );
    assert!(
        err.message.contains(&format!(
            "released for {platform}: 3.1.6, 3.2.4, 3.2.5, 3.2.6, 3.2.7, 3.2.11"
        )),
        "{}",
        err.message
    );
    assert!(
        err.message
            .contains("this payload needs a newer ruby than this platform provides yet"),
        "{}",
        err.message
    );
    assert!(
        !home.join("runtimes").exists(),
        "a refused resolution installs nothing"
    );
}

#[test]
fn the_index_target_is_the_newest_released_version_satisfying_the_constraint() {
    let tmp = TempDir::new("index-target");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_release_index(&mirror, "0.16.0", WINDOWS_LINE);
    // the pin names the tebako line to consult (and satisfies the
    // constraint); the index, not the pin, picks the ruby version.
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.2.4\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );

    let rt = ready(runtime::resolve_runtime(Some(&req(">= 3.2")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "3.2.11");
    assert_eq!(rt.tebako_version, "0.16.0");
    assert!(rt.exe.is_file());
    assert!(rt.image.is_some());
    assert!(rt.dir.join("sha256").is_file());
    assert!(rt.dir.join("origin").is_file());
}

#[test]
fn a_cache_hit_never_consults_the_index() {
    // the cache wins outright: with a satisfying cached runtime the
    // mirror is never touched — here a mirror that does not even exist,
    // so any fetch would fall through to the no-preference error.
    let tmp = TempDir::new("index-cache-wins");
    let home = tmp.path().join("home");
    write_runtime(&home, "3.2.5", "0.16.0", false);
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&tmp.path().join("no-such-mirror")),
    );
    let rt = ready(runtime::resolve_runtime(Some(&req(">= 3.2")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "3.2.5");
}

#[test]
fn offline_never_consults_the_index() {
    // TEBAKO_OFFLINE is cache-or-named-error: the index consult never
    // fetches, so the download path's own offline error stands —
    // unchanged by the index-driven target selection.
    let tmp = TempDir::new("index-offline");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_release_index(&mirror, "0.16.0", &["3.2.11"]);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 3.2.11\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    ctx.env.insert("TEBAKO_OFFLINE".into(), "1".into());
    let err = runtime::resolve_runtime(Some(&req(">= 3.2")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(err.message.contains("TEBAKO_OFFLINE"), "{}", err.message);
}
