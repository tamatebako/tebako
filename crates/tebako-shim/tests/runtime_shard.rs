//! The shard-first release-index reads (roadmap 85, spec 05 §2):
//! post-85 release lines ship per-package shards and no monoliths; the
//! shim's download path consumes the shard (signed or unsigned), and a
//! missing / triple-mismatched shard falls through to the immutable
//! monolith forms (invariant 7). Everything runs against file:// mirrors
//! and temp TEBAKO_HOMEs — no network.

mod common;

use common::*;
use tebako_shim::runtime::{self, RuntimeResolution};
use tpkg::{Constraint, RuntimeRequirement, RuntimeRequirements};

fn req_engine(engine: &str, constraint: &str) -> RuntimeRequirements {
    RuntimeRequirements::one(RuntimeRequirement {
        engine: engine.to_string(),
        constraint: Constraint::new(constraint).expect("test constraint parses"),
        implementation: None,
        abi: None,
    })
}

fn ready(res: RuntimeResolution) -> runtime::CachedRuntime {
    match res {
        RuntimeResolution::Ready(rt) => *rt,
        RuntimeResolution::Zero => panic!("expected a resolved runtime"),
    }
}

fn journal_text(home: &std::path::Path) -> String {
    std::fs::read_to_string(home.join("journal.log")).unwrap_or_default()
}

#[test]
fn a_signed_shard_only_release_resolves_and_caches_the_normalized_card() {
    // Acceptance 1 (shim): an exact-pin resolve against a release that
    // carries ONLY the per-package shard — no manifest.json, no
    // SHA256SUMS.txt.
    let tmp = TempDir::new("shard-signed");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    tebako_signer::register_trusted(&home, &key.public_key).expect("trust");
    let mirror = tmp.path().join("mirror");
    write_shard_release(
        &mirror,
        "ruby",
        "4.0.6",
        "0.16.0",
        "v0.16.0",
        Some(&key),
        None,
    );
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    ctx.env.insert("TEBAKO_REQUIRE_SIGNED".into(), "1".into());

    let rt = ready(
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), None, true, &ctx).unwrap(),
    );
    assert_eq!(rt.lang_version, "4.0.6");
    assert!(rt.exe.is_file());
    assert!(rt.image.is_some());

    // the consumed card is the shard NORMALIZED to the array shape — the
    // store's card readers (entry_meta / entry_asset_names) parse arrays
    let card = std::fs::read_to_string(rt.dir.join("manifest.json")).expect("the cached card");
    let card_trim = card.trim_start();
    assert!(
        card_trim.starts_with('['),
        "the cached manifest.json is the normalized array: {card}"
    );
    assert!(card.contains("\"ruby_version\": \"4.0.6\""), "{card}");

    let journal = journal_text(&home);
    let line = journal
        .lines()
        .find(|l| l.contains("event=runtime-fetch-verified"))
        .unwrap_or_else(|| panic!("no verified-fetch journal line in {journal}"));
    assert!(
        line.contains(&format!("signer={}", tebako_signer::hex_lower(&key.keyid))),
        "{line} names the shard's verified signer"
    );
    assert!(
        !journal.contains("event=unsigned-runtime-fetch"),
        "{journal}"
    );
}

#[test]
fn an_unsigned_shard_only_release_warns_journals_and_installs() {
    let tmp = TempDir::new("shard-unsigned");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_shard_release(&mirror, "ruby", "4.0.6", "0.16.0", "v0.16.0", None, None);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let rt = ready(
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), None, true, &ctx).unwrap(),
    );
    assert_eq!(rt.lang_version, "4.0.6");
    let journal = journal_text(&home);
    assert!(
        journal.contains("event=unsigned-runtime-fetch"),
        "{journal}"
    );
}

#[test]
fn an_unsigned_shard_only_release_under_require_signed_is_exit_71() {
    let tmp = TempDir::new("shard-unsigned-strict");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_shard_release(&mirror, "ruby", "4.0.6", "0.16.0", "v0.16.0", None, None);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    ctx.env.insert("TEBAKO_REQUIRE_SIGNED".into(), "1".into());
    let err = runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), None, true, &ctx)
        .unwrap_err();
    assert_eq!(
        err.code,
        tebako_shim::EX_TEBAKO_SIGNATURE,
        "{}",
        err.message
    );
    assert!(
        !home
            .join("runtimes")
            .join(format!("ruby-4.0.6-0.16.0-{}", platform()))
            .exists(),
        "a refused runtime entered the cache"
    );
}

#[test]
fn a_triple_mismatched_shard_falls_through_to_the_monolith() {
    // spec 05 §2: a shard naming another identity triple is not this
    // package's card — the read falls through to the monolith (which here
    // carries the requested triple) with the URL recorded.
    let tmp = TempDir::new("shard-mismatch");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    // The monolith release (the requested triple, unsigned).
    write_release_index(&mirror, "0.16.0", &["4.0.6"]);
    // …plus a shard naming a DIFFERENT ruby_version at the same stem.
    let dir = mirror.join("v0.16.0");
    let platform = platform();
    let stem = format!("tebako-runtime-0.16.0-4.0.6-{platform}");
    std::fs::write(
        dir.join(format!("{stem}.manifest.json")),
        format!(
            "{{\"tebako_version\": \"0.16.0\", \"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"ruby_version\": \"9.9.9\", \"platform\": \"{platform}\", \"filename\": \"{stem}\", \"sha256\": \"{}\"}}\n",
            "0".repeat(64)
        ),
    )
    .expect("mismatched shard");
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let rt = ready(
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), None, true, &ctx).unwrap(),
    );
    assert_eq!(
        rt.lang_version, "4.0.6",
        "the monolith served the download, not the mismatched shard"
    );
    assert!(rt.exe.is_file());
}
