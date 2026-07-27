//! Cached-runtime compatibility selection (spec 05 §5: range vs
//! abi-line) and the download fallback (bootstrap discipline: flock,
//! tmp+rename, trust markers, read-only image).

mod common;

use common::*;
use tebako_shim::manifest::{Constraint, RuntimeRequirement};
use tebako_shim::runtime::{self, RuntimeResolution};

fn req(constraint: &str) -> RuntimeRequirement {
    RuntimeRequirement {
        engine: "ruby".to_string(),
        constraint: Constraint::new(constraint).expect("fixture constraint parses"),
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
fn zero_requirement_skips_resolution_entirely() {
    let tmp = TempDir::new("zero-req");
    let home = tmp.path().join("home");
    // no runtimes dir, no mirror, no config — still Zero
    let res = runtime::resolve_runtime(None, true, &ctx(&home, tmp.path())).unwrap();
    assert!(matches!(res, RuntimeResolution::Zero));
}
