//! Runtime fetch-time OpenPGP verification (spec 09 §4 — roadmap 80's G1)
//! and the per-engine download-source chain (spec 05 §2 — tebako#567).
//!
//! Fixtures are file:// release mirrors signed by a throwaway
//! `press_local_key`; trust is registered in the test home's keyring —
//! the exact shape the production path consumes.

mod common;

use std::path::Path;

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

/// A file:// release mirror in the factory's locked shape (spec 13 §2a),
/// signed per spec 09 §5's finalize pass when `key` is Some: the index
/// entry declares `signature` blocks for the exe and the image, and
/// `manifest.json.asc` + the per-asset ascs sit beside their bodies.
/// `tag` is the release directory — decoupled from the rows' tebako line
/// (the openjdk v2.5.1 shape). Returns the release directory.
#[allow(clippy::too_many_arguments)]
fn write_signed_release(
    root: &Path,
    engine: &str,
    lv: &str,
    tebako: &str,
    tag: &str,
    key: Option<&tebako_signer::PressKey>,
    declared_keyid: Option<&str>,
) -> std::path::PathBuf {
    let platform = platform();
    let dir = root.join(tag);
    std::fs::create_dir_all(&dir).expect("mirror dir");
    let asset_base = format!("tebako-runtime-{tebako}-{lv}-{platform}");
    let exe_name = format!("{asset_base}{}", tebako_shim::runtime::exe_suffix());
    let image_name = format!("{asset_base}.tfs");
    let exe_bytes = format!("signed runtime exe {lv}\n");
    let image_bytes = format!("signed runtime image {lv}\n");
    std::fs::write(dir.join(&exe_name), &exe_bytes).expect("exe");
    std::fs::write(dir.join(&image_name), &image_bytes).expect("image");
    let declared = key.map(|k| {
        declared_keyid
            .map(str::to_string)
            .unwrap_or_else(|| tebako_signer::hex_lower(&k.keyid))
    });
    let signature_block = |asc: &str| match &declared {
        Some(keyid) => format!(", \"signature\": {{\"keyid\": \"{keyid}\", \"asc\": \"{asc}\"}}"),
        None => String::new(),
    };
    let manifest = format!(
        "[{{\"tebako_version\": \"{tebako}\", \"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"{engine}_version\": \"{lv}\", \"platform\": \"{platform}\", \"filename\": \"{exe_name}\", \"sha256\": \"{}\"{}, \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{}\"{}}}}}]\n",
        sha256_hex(exe_bytes.as_bytes()),
        signature_block(&format!("{exe_name}.asc")),
        sha256_hex(image_bytes.as_bytes()),
        signature_block(&format!("{image_name}.asc")),
    );
    std::fs::write(dir.join("manifest.json"), &manifest).expect("manifest.json");
    if let Some(k) = key {
        let sign = |data: &[u8]| {
            tebako_signer::sign_detached(data, &k.secret_key, &k.fingerprint).expect("sign")
        };
        std::fs::write(dir.join("manifest.json.asc"), sign(manifest.as_bytes())).expect("asc");
        std::fs::write(
            dir.join(format!("{exe_name}.asc")),
            sign(exe_bytes.as_bytes()),
        )
        .expect("exe asc");
        std::fs::write(
            dir.join(format!("{image_name}.asc")),
            sign(image_bytes.as_bytes()),
        )
        .expect("image asc");
    }
    dir
}

fn journal_text(home: &Path) -> String {
    std::fs::read_to_string(home.join("journal.log")).unwrap_or_default()
}

#[test]
fn a_signed_release_verifies_and_journals_the_strength() {
    let tmp = TempDir::new("g1-signed");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    // The consumer trusts the factory key (TOFU registration).
    tebako_signer::register_trusted(&home, &key.public_key).expect("trust");
    let mirror = tmp.path().join("mirror");
    write_signed_release(
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
    // Fail-closed mode on: a signed release must sail through it.
    ctx.env.insert("TEBAKO_REQUIRE_SIGNED".into(), "1".into());

    let rt =
        ready(runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "4.0.6");
    assert!(rt.exe.is_file());
    assert!(rt.image.is_some());
    let journal = journal_text(&home);
    let line = journal
        .lines()
        .find(|l| l.contains("event=runtime-fetch-verified"))
        .unwrap_or_else(|| panic!("no verified-fetch journal line in {journal}"));
    assert!(line.contains("channel=mirror-env"), "{line}");
    assert!(
        line.contains(&format!("signer={}", tebako_signer::hex_lower(&key.keyid))),
        "{line} names the verified signer"
    );
    assert!(
        !journal.contains("event=unsigned-runtime-fetch"),
        "{journal}"
    );
}

#[test]
fn an_untrusted_signer_is_exit_72() {
    let tmp = TempDir::new("g1-untrusted");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    // Signed — but the consumer's keyring never learned the key.
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    let mirror = tmp.path().join("mirror");
    write_signed_release(
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
    let err =
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_TRUST, "{}", err.message);
    assert!(
        err.message.contains("not in the trusted keyring"),
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
fn a_tampered_artifact_is_exit_71_before_the_checksum() {
    let tmp = TempDir::new("g1-tampered");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    tebako_signer::register_trusted(&home, &key.public_key).expect("trust");
    let mirror = tmp.path().join("mirror");
    let dir = write_signed_release(
        &mirror,
        "ruby",
        "4.0.6",
        "0.16.0",
        "v0.16.0",
        Some(&key),
        None,
    );
    // Tamper the exe AFTER signing: the declared asc no longer matches.
    let exe = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.starts_with("tebako-runtime-") && !n.ends_with(".tfs") && !n.ends_with(".asc"))
        .expect("exe asset");
    std::fs::write(dir.join(&exe), b"evicted bytes\n").unwrap();
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let err =
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(
        err.code,
        tebako_shim::EX_TEBAKO_SIGNATURE,
        "{}",
        err.message
    );
    assert!(err.message.contains("invalid signature"), "{}", err.message);
}

#[test]
fn a_declared_signature_that_does_not_fetch_is_exit_71() {
    let tmp = TempDir::new("g1-missing-asc");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    tebako_signer::register_trusted(&home, &key.public_key).expect("trust");
    let mirror = tmp.path().join("mirror");
    let dir = write_signed_release(
        &mirror,
        "ruby",
        "4.0.6",
        "0.16.0",
        "v0.16.0",
        Some(&key),
        None,
    );
    // The entry declares the exe's asc; the release does not carry it.
    let asc = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.starts_with("tebako-runtime-") && n.ends_with(".asc") && !n.contains("tfs"))
        .expect("exe asc");
    std::fs::remove_file(dir.join(&asc)).unwrap();
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let err =
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(
        err.code,
        tebako_shim::EX_TEBAKO_SIGNATURE,
        "{}",
        err.message
    );
    assert!(err.message.contains("did not fetch"), "{}", err.message);
}

#[test]
fn a_signer_key_mismatch_against_the_declared_keyid_is_exit_72() {
    let tmp = TempDir::new("g1-keyid-mismatch");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    tebako_signer::register_trusted(&home, &key.public_key).expect("trust");
    let mirror = tmp.path().join("mirror");
    // The entry pins a DIFFERENT primary than the one that signed —
    // spec 09 §9's SignerKeyChanged.
    write_signed_release(
        &mirror,
        "ruby",
        "4.0.6",
        "0.16.0",
        "v0.16.0",
        Some(&key),
        Some("0000000000000000"),
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
    let err =
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_TRUST, "{}", err.message);
    assert!(
        err.message.contains("signer key changed"),
        "{}",
        err.message
    );
}

#[test]
fn an_unsigned_release_warns_journals_and_installs() {
    let tmp = TempDir::new("g1-unsigned");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_signed_release(&mirror, "ruby", "4.0.6", "0.16.0", "v0.16.0", None, None);
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let rt =
        ready(runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap());
    assert_eq!(rt.lang_version, "4.0.6");
    let journal = journal_text(&home);
    let line = journal
        .lines()
        .find(|l| l.contains("event=unsigned-runtime-fetch"))
        .unwrap_or_else(|| panic!("no unsigned-fetch journal line in {journal}"));
    assert!(line.contains("channel=mirror-env"), "{line}");
}

#[test]
fn an_unsigned_release_under_require_signed_is_exit_71() {
    let tmp = TempDir::new("g1-unsigned-strict");
    let home = tmp.path().join("home");
    let mirror = tmp.path().join("mirror");
    write_signed_release(&mirror, "ruby", "4.0.6", "0.16.0", "v0.16.0", None, None);
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
    let err =
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(
        err.code,
        tebako_shim::EX_TEBAKO_SIGNATURE,
        "{}",
        err.message
    );
    assert!(
        err.message.contains("TEBAKO_REQUIRE_SIGNED"),
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
fn a_sums_only_signed_release_has_no_trusted_contract_card() {
    // Only SHA256SUMS.txt verifies: its digests are trustworthy, but the
    // contract card's carrier (manifest.json) is unsigned — the spec 18
    // C2 refusal (75), never a card read from an unverified form.
    let tmp = TempDir::new("g1-sums-only");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    tebako_signer::register_trusted(&home, &key.public_key).expect("trust");
    let mirror = tmp.path().join("mirror");
    let dir = write_signed_release(
        &mirror,
        "ruby",
        "4.0.6",
        "0.16.0",
        "v0.16.0",
        Some(&key),
        None,
    );
    // Drop the manifest's asc; sign a SHA256SUMS.txt instead.
    std::fs::remove_file(dir.join("manifest.json.asc")).unwrap();
    let mut sums = String::new();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".asc") || name == "manifest.json" || name == "SHA256SUMS.txt" {
            continue;
        }
        let bytes = std::fs::read(entry.path()).unwrap();
        sums.push_str(&format!("{}  {name}\n", sha256_hex(&bytes)));
    }
    std::fs::write(dir.join("SHA256SUMS.txt"), &sums).unwrap();
    let asc = tebako_signer::sign_detached(sums.as_bytes(), &key.secret_key, &key.fingerprint)
        .expect("sign sums");
    std::fs::write(dir.join("SHA256SUMS.txt.asc"), asc).unwrap();
    write_config(
        &home,
        "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n",
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&mirror),
    );
    let err =
        runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_CONTRACT, "{}", err.message);
    assert!(
        err.message.contains("SHA256SUMS.txt verified"),
        "{}",
        err.message
    );
}

#[test]
fn the_config_source_pin_shadows_the_mirror_loudly() {
    // spec 05 §2 channel 1: `source:` wins; a differing
    // TEBAKO_RUNTIME_MIRROR is shadowed — loud + journaled.
    let tmp = TempDir::new("567-source-pin");
    let home = tmp.path().join("home");
    let factory = tmp.path().join("factory");
    let key = tebako_signer::press_local_key(&factory).expect("factory key");
    tebako_signer::register_trusted(&home, &key.public_key).expect("trust");
    let pinned = tmp.path().join("pinned");
    write_signed_release(
        &pinned,
        "ruby",
        "4.0.6",
        "0.16.0",
        "v0.16.0",
        Some(&key),
        None,
    );
    let shadowed = tmp.path().join("shadowed");
    std::fs::create_dir_all(&shadowed).unwrap(); // empty — consulting it fails
    let pinned_url = tebako_http::file_url(&pinned);
    write_config(
        &home,
        &format!(
            "runtimes:\n  ruby:\n    version: 4.0.6\n    tebako: 0.16.0\n    source: \"{pinned_url}\"\n"
        ),
    );
    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        "TEBAKO_RUNTIME_MIRROR".into(),
        tebako_http::file_url(&shadowed),
    );
    let rt =
        ready(runtime::resolve_runtime(Some(&req_engine("ruby", ">= 3.3")), true, &ctx).unwrap());
    assert_eq!(
        rt.lang_version, "4.0.6",
        "the config source served, not the (empty) mirror"
    );
    let journal = journal_text(&home);
    let line = journal
        .lines()
        .find(|l| l.contains("event=runtime-source-shadow"))
        .unwrap_or_else(|| panic!("no shadowing journal line in {journal}"));
    assert!(line.contains("engine=ruby"), "{line}");
    let verified = journal
        .lines()
        .find(|l| l.contains("event=runtime-fetch-verified"))
        .unwrap_or_else(|| panic!("no verified-fetch journal line in {journal}"));
    assert!(verified.contains("channel=config-source"), "{verified}");
}

#[test]
fn a_non_ruby_engine_no_channel_answers_is_the_named_error() {
    // tebako#567's closeout: no config source, no mirror, no registry,
    // and the default hosts ruby only — the error enumerates the chain.
    let tmp = TempDir::new("567-no-channel");
    let home = tmp.path().join("home");
    let err = runtime::resolve_runtime(
        Some(&req_engine("java", ">= 21")),
        true,
        &ctx(&home, tmp.path()),
    )
    .unwrap_err();
    assert_eq!(
        err.code,
        tebako_shim::EX_TEBAKO_UNAVAILABLE,
        "{}",
        err.message
    );
    for channel in [
        "runtimes: {java: {source:",
        "TEBAKO_RUNTIME_MIRROR",
        "kind: runtime",
        "hosts ruby runtimes only",
    ] {
        assert!(err.message.contains(channel), "{channel} — {}", err.message);
    }
}
