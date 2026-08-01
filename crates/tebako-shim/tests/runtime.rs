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
    let err =
        runtime::resolve_runtime(Some(&req(">= 3.3")), true, &ctx(&home, tmp.path())).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE);
    assert!(
        err.message.contains("runtime preference"),
        "{}",
        err.message
    );
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
        format!("file://{}", mirror.display()),
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
        format!("file://{}", mirror.display()),
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
        format!("file://{}", mirror.display()),
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
        format!("file://{}", mirror.display()),
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
        format!("file://{}", mirror.display()),
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
