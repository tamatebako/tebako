//! `tebako cache seal` + the lazy listing surface (spec 39 §5/§7 — the
//! PR-4 e2e): a LAZY_SEEDING runtime entry in a scratch store seals
//! over a `file://` source — the synchronous verb fills the missing
//! groups, commits the SEALED flip, and the entry becomes an ordinary
//! cached runtime. The offline legs: a miss is the named 69; a
//! complete block map seals with the origin DELETED (no network, no
//! origin read — the block cache is the whole truth).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-seal-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(home: &Path, args: &[&str], extra: &[(&str, &str)]) -> (i32, String, String) {
    let mut cmd = Command::new(tebako_bin());
    cmd.args(args).env("TEBAKO_HOME", home);
    for (k, v) in extra {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn failed: {e}"));
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The image bytes: 6 MiB of incompressible data → two 4 MiB groups.
/// Not a real limnifs image — the seal pass assembles the verified
/// groups and checks the whole-image sha; no mount happens in the verb.
fn image_bytes() -> Vec<u8> {
    let mut big = vec![0u8; 6 * 1024 * 1024];
    let mut state = 0x9E3779B97F4A7C15u64;
    for chunk in big.chunks_mut(8) {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let bytes = state.wrapping_mul(0x2545F4914F6CDD1D).to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
    big
}

/// A LAZY_SEEDING runtime entry in `home` (exe + seed descriptor, the
/// image ABSENT) plus the origin image + sidecar on disk. Returns
/// (entry_name, image_base).
fn seed_entry(home: &Path, origin: &Path) -> (String, String) {
    let platform = tpkg::runtime_store::platform_string();
    let entry_name = format!("ruby-3.3.12-0.16.9-{platform}");
    let dir = home.join("runtimes").join(&entry_name);
    fs::create_dir_all(&dir).unwrap();
    let (exe_name, image_base) =
        tpkg::runtime_store::entry_asset_names(&dir, "ruby", "3.3.12", "0.16.9", platform);
    fs::write(dir.join(&exe_name), b"exe").unwrap();
    let bytes = fs::read(origin).unwrap();
    let blksum = tpkg::lazy::Blksum::from_image_bytes(&bytes);
    let sidecar = blksum.render();
    fs::write(format!("{}.blksum.json", origin.display()), &sidecar).unwrap();
    let seed = tpkg::lazy::LazySeed {
        source: format!("file://{}", origin.display()),
        sha256: blksum.sha256.clone(),
        blksum_sha256: tpkg::lazy::sha256_hex(sidecar.as_bytes()),
        size_bytes: blksum.size_bytes,
        group_count: blksum.group_count(),
    };
    tpkg::lazy::write_descriptor(&dir, &image_base, &seed).unwrap();
    (entry_name, image_base)
}

/// Pre-seed a block file with the group's raw bytes (the shape
/// seed_group leaves after a verified fetch).
fn seed_block(home: &Path, entry_name: &str, image_base: &str, origin: &Path, index: u64) {
    let bytes = fs::read(origin).unwrap();
    let (offset, len) = tpkg::lazy::group_span(bytes.len() as u64, index).unwrap();
    let blocks = tpkg::lazy::blocks_dir(
        &home.join("runtimes").join(entry_name),
        image_base,
    );
    fs::create_dir_all(&blocks).unwrap();
    fs::write(
        blocks.join(tpkg::lazy::block_name(index)),
        &bytes[offset as usize..(offset + len) as usize],
    )
    .unwrap();
}

#[test]
fn seal_all_fills_and_flips_the_entry() {
    let home = scratch("all");
    let origin = home.join("origin.tfs");
    fs::write(&origin, image_bytes()).unwrap();
    let (entry_name, image_base) = seed_entry(&home, &origin);
    let entry_dir = home.join("runtimes").join(&entry_name);

    // The listing surface marks the seeding entry.
    let (code, out, _) = run(&home, &["cache", "list"], &[]);
    assert_eq!(code, 0);
    assert!(out.contains("(seeding 0%, 0/2 groups)"), "{out}");
    // The machine form carries the additive lazy_seeding object.
    let (code, out, _) = run(&home, &["cache", "list", "--json"], &[]);
    assert_eq!(code, 0);
    assert!(out.contains("\"lazy_seeding\""), "{out}");
    assert!(out.contains("\"total_groups\":\"2\""), "{out}");
    // Doctor reports the seed state by name.
    let (_, out, err) = run(&home, &["doctor"], &[]);
    assert!(
        out.contains("LAZY_SEEDING") || err.contains("LAZY_SEEDING"),
        "stdout: {out}\nstderr: {err}"
    );

    // The verb: fill + commit → SEALED.
    let (code, _, err) = run(&home, &["cache", "seal", "--all"], &[]);
    assert_eq!(code, 0, "{err}");
    let sealed = entry_dir.join(&image_base);
    assert!(sealed.is_file(), "the image landed");
    assert_eq!(fs::read(&sealed).unwrap(), fs::read(&origin).unwrap());
    assert!(entry_dir.join(format!("{image_base}.sha256")).is_file());
    assert!(!tpkg::lazy::descriptor_path(&entry_dir, &image_base).exists());
    assert!(!tpkg::lazy::blocks_dir(&entry_dir, &image_base).exists());

    // The listing is ordinary again; a second seal is a no-op.
    let (code, out, _) = run(&home, &["cache", "list"], &[]);
    assert_eq!(code, 0);
    assert!(!out.contains("seeding"), "{out}");
    let (code, out, _) = run(&home, &["cache", "seal", "--all"], &[]);
    assert_eq!(code, 0);
    assert!(out.contains("nothing to seal"), "{out}");

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn seal_a_named_entry_and_the_named_miss() {
    let home = scratch("named");
    let origin = home.join("origin.tfs");
    fs::write(&origin, image_bytes()).unwrap();
    let (entry_name, image_base) = seed_entry(&home, &origin);
    let entry_dir = home.join("runtimes").join(&entry_name);

    // An unknown entry name is the named 65.
    let (code, _, err) = run(&home, &["cache", "seal", "ruby-9.9.9-0.0.0-nope"], &[]);
    assert_eq!(code, 65, "{err}");
    assert!(err.contains("no cached runtime entry named"), "{err}");

    // The named selector seals exactly that entry.
    let (code, _, err) = run(&home, &["cache", "seal", &entry_name], &[]);
    assert_eq!(code, 0, "{err}");
    assert!(entry_dir.join(&image_base).is_file());

    // Sealing a sealed entry is a quiet no-op.
    let (code, out, _) = run(&home, &["cache", "seal", &entry_name], &[]);
    assert_eq!(code, 0);
    assert!(out.contains("sealed already"), "{out}");

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn offline_seals_only_a_complete_block_map() {
    let home = scratch("offline");
    let origin = home.join("origin.tfs");
    fs::write(&origin, image_bytes()).unwrap();
    let (entry_name, image_base) = seed_entry(&home, &origin);
    let entry_dir = home.join("runtimes").join(&entry_name);

    // Seed group 0, then DELETE the origin: the offline verb has no
    // source to read — a miss is the named 69, the entry stays
    // LAZY_SEEDING.
    seed_block(&home, &entry_name, &image_base, &origin, 0);
    fs::remove_file(&origin).unwrap();
    let (code, _, err) = run(
        &home,
        &["cache", "seal", "--all"],
        &[("TEBAKO_OFFLINE", "1")],
    );
    assert_eq!(code, 69, "{err}");
    assert!(tpkg::lazy::descriptor_path(&entry_dir, &image_base).exists());
    assert!(!entry_dir.join(&image_base).exists());

    // Complete the block map by hand: the offline seal assembles from
    // the verified groups alone (the origin never comes back).
    let rebuilt = home.join("rebuilt.tfs");
    fs::write(&rebuilt, image_bytes()).unwrap();
    seed_block(&home, &entry_name, &image_base, &rebuilt, 1);
    fs::remove_file(&rebuilt).unwrap();
    let (code, _, err) = run(
        &home,
        &["cache", "seal", "--all"],
        &[("TEBAKO_OFFLINE", "1")],
    );
    assert_eq!(code, 0, "{err}");
    assert!(entry_dir.join(&image_base).is_file());
    assert!(!tpkg::lazy::descriptor_path(&entry_dir, &image_base).exists());

    let _ = fs::remove_dir_all(&home);
}
