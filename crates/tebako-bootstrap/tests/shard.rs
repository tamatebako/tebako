//! Roadmap 85 (spec 05 §2): the shard-first release-card read path. A
//! release mirror carrying ONLY the per-identity `<stem>.manifest.json`
//! shard — no release-wide manifest.json, no SHA256SUMS.txt — resolves
//! exactly like the monolith era; a shard naming a different identity
//! triple falls through to the monolith; no card at all is the named
//! pre-era refusal naming BOTH tried URLs.

// The shared harness is compiled per test target; this target uses only
// a subset of its helpers (the convention progress.rs/compose.rs set).
#[allow(dead_code)]
mod harness;

use harness::{rust_bootstrap, Harness, TEBAKO_VER};

/// The shard stem (the exe asset's suffix-less name — the image asset
/// minus its `.tfs`).
fn stem(h: &Harness) -> String {
    h.image_asset
        .strip_suffix(".tfs")
        .expect("the harness image asset ends in .tfs")
        .to_string()
}

/// Rewrite the harness mirror into the post-85 shape: the per-identity
/// shard (a bare object, the factory's shard spelling) replaces the
/// release-wide manifest.json; SHA256SUMS.txt is gone too — every
/// checksum flows from the shard's own fields.
fn shard_only_mirror(h: &Harness) {
    let mirror = h.mirror_root.join(format!("v{TEBAKO_VER}"));
    let monolith = mirror.join("manifest.json");
    let text = std::fs::read_to_string(&monolith).unwrap();
    let body = text
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .expect("the harness monolith is a one-entry array")
        .trim();
    std::fs::write(
        mirror.join(format!("{}.manifest.json", stem(h))),
        format!("{body}\n"),
    )
    .unwrap();
    std::fs::remove_file(&monolith).unwrap();
    std::fs::remove_file(mirror.join("SHA256SUMS.txt")).unwrap();
}

#[test]
fn a_shard_only_mirror_resolves_the_exact_pin() {
    let h = Harness::new(rust_bootstrap());
    shard_only_mirror(&h);
    // the image-era package exercises BOTH shard readers (exe + image)
    let pkg = h.lean_pkg_image("myapp");
    let home = h.home("home");
    let (rc, out, err) = h.run(&pkg, &home, &[], &["hello", "arg two"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");

    // the cached card is the normalized array shape, never the raw shard
    let card = home.join("runtimes").join(&h.entry).join("manifest.json");
    let cached = std::fs::read_to_string(&card).unwrap();
    assert!(cached.starts_with('['), "{cached}");
    assert!(
        cached.contains(&format!("\"filename\": \"{}\"", h.asset)),
        "{cached}"
    );

    // the image landed from the same shard, with its trust markers
    assert!(h.cache_image(&home).is_file());
    assert!(home
        .join("runtimes")
        .join(&h.entry)
        .join(format!("{}.sha256", h.image_asset))
        .is_file());

    // a cached run needs no mirror at all (the normalized card serves)
    std::fs::rename(&h.mirror_root, h.tmp.0.join("mirror-gone")).unwrap();
    let (rc, out, err) = h.run(&pkg, &home, &[("TEBAKO_OFFLINE", "1")], &["hello"]);
    assert_eq!(rc, 0, "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
}

#[test]
fn a_shard_naming_another_identity_falls_through_to_the_monolith() {
    let h = Harness::new(rust_bootstrap());
    let mirror = h.mirror_root.join(format!("v{TEBAKO_VER}"));
    // a shard at OUR stem whose triple is another runtime's: present and
    // readable, never served (spec 05 §2's fall-through rule)
    std::fs::write(
        mirror.join(format!("{}.manifest.json", stem(&h))),
        "{\n  \"tebako_version\": \"9.9.9\",\n  \"ruby_version\": \"0.0.0\",\n  \"platform\": \"nowhere\",\n  \"filename\": \"tebako-runtime-9.9.9-0.0.0-nowhere\",\n  \"sha256\": \"0000000000000000000000000000000000000000000000000000000000000000\"\n}\n",
    )
    .unwrap();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home");
    let (rc, out, err) = h.run(&pkg, &home, &[], &["hello"]);
    assert_eq!((rc, err.as_str()), (0, ""), "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
}

#[test]
fn no_card_at_all_is_the_named_pre_era_refusal() {
    let h = Harness::new(rust_bootstrap());
    let mirror = h.mirror_root.join(format!("v{TEBAKO_VER}"));
    std::fs::remove_file(mirror.join("manifest.json")).unwrap();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home");
    let (rc, _, err) = h.run_raw(&pkg, &home, &[], &[]);
    assert_eq!(rc, 75, "{err}");
    assert!(err.contains("pre-era"), "{err}");
    assert!(err.contains("tried:"), "{err}");
    // both forms named: the per-identity shard first, the monolith second
    assert!(
        err.contains(&format!("{}.manifest.json", stem(&h))),
        "{err}"
    );
    assert!(
        err.contains(&format!("/v{TEBAKO_VER}/manifest.json")),
        "{err}"
    );
    assert!(
        !err.contains("downloading "),
        "the refusal must precede any download: {err}"
    );
    assert!(
        !home.join("runtimes").join(&h.entry).exists(),
        "a refused runtime entered the cache"
    );
}
