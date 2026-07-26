//! End-to-end legs through the public API, no network: `file://` mirrors
//! (spec 04 §3) and a local git fixture cloned by the real gix adapter.

use std::fs;
use std::path::PathBuf;

use tebako_resolve::{
    fetch_and_cache, sha256_hex, Fetcher, InstallStatus, PayloadCache, Reference, ResolveError,
};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-resolve-it-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn file_mirror_fetch_and_cache() {
    let dir = scratch("file");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tool.tfs"), b"payload-bytes").unwrap();
    let cache = PayloadCache::with_root(dir.join("cache"));

    let reference = Reference::parse(&format!("file://{}/tool.tfs", mirror.display())).unwrap();
    let (entry, status) = fetch_and_cache(&cache, &reference, "tool", "1.2.3", None).unwrap();
    assert_eq!(status, InstallStatus::Installed);
    assert_eq!(fs::read(&entry.path).unwrap(), b"payload-bytes");
    assert_eq!(entry.sha256, sha256_hex(b"payload-bytes"));

    let (_entry, status) = fetch_and_cache(&cache, &reference, "tool", "1.2.3", None).unwrap();
    assert_eq!(status, InstallStatus::Hit);

    // A registry-supplied anchor matching the artifact also installs.
    let (entry, status) = fetch_and_cache(
        &cache,
        &reference,
        "tool",
        "2.0",
        Some(&sha256_hex(b"payload-bytes")),
    )
    .unwrap();
    assert_eq!(status, InstallStatus::Installed);
    assert!(entry.path.ends_with("payloads/tool/2.0.tfs"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn pinned_mirror_mismatch_caches_nothing() {
    let dir = scratch("mismatch");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tool.tfs"), b"payload-bytes").unwrap();
    let cache = PayloadCache::with_root(dir.join("cache"));

    let reference = Reference::parse(&format!(
        "file://{}/tool.tfs?sha256={}",
        mirror.display(),
        "0".repeat(64)
    ))
    .unwrap();
    let err = fetch_and_cache(&cache, &reference, "tool", "1.0", None).unwrap_err();
    assert!(matches!(err, ResolveError::Sha256Mismatch { .. }));
    assert!(!dir.join("cache/payloads/tool/1.0.tfs").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn git_reference_fetches_through_the_real_adapter() {
    let dir = scratch("git");
    let fixture = dir.join("fixture.git");
    // A bare repo holding two payload images on a versioned branch.
    let repo = gix::init_bare(&fixture).unwrap();
    drop(repo);
    // gix commit needs an identity; CI runners have no ambient git
    // config. Pin it in-repo (append — init already wrote [core]) and
    // re-open so the config is re-read.
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .append(true)
        .open(fixture.join("config"))
        .unwrap()
        .write_all(b"[user]\n\tname = tebako-test\n\temail = tebako-test@example.org\n")
        .unwrap();
    let repo = gix::open(&fixture).unwrap();
    let blob = repo.write_blob(b"git-payload").unwrap();
    let empty = gix::hash::ObjectId::empty_tree(repo.object_hash());
    let tree = {
        let mut ed = repo.edit_tree(empty).unwrap();
        ed.upsert("images/tool.tfs", gix::object::tree::EntryKind::Blob, blob)
            .unwrap();
        ed.write().unwrap()
    };
    repo.commit("refs/heads/v1", "v1", tree, gix::commit::NO_PARENT_IDS)
        .unwrap();

    let reference = Reference::Git {
        url: fixture.to_string_lossy().into_owned(),
        git_ref: Some("v1".to_string()),
        path: Some("images/tool.tfs".to_string()),
        sha256: Some(sha256_hex(b"git-payload")),
    };
    let fetcher = Fetcher::new();
    let got = fetcher.fetch(&reference).unwrap();
    assert_eq!(got.bytes, b"git-payload");

    let cache = PayloadCache::with_root(dir.join("cache"));
    let (entry, status) = fetch_and_cache(&cache, &reference, "tool", "1.0", None).unwrap();
    assert_eq!(status, InstallStatus::Installed);
    assert_eq!(fs::read(&entry.path).unwrap(), b"git-payload");
    // The origin marker records the full reference, pin included.
    let origin = fs::read_to_string(dir.join("cache/payloads/tool/1.0.tfs.origin")).unwrap();
    assert!(origin.contains("tfs+git://") && origin.contains("#images/tool.tfs"));
    let _ = fs::remove_dir_all(&dir);
}
