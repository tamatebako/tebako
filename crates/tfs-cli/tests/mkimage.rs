//! mkimage tests: in-process writer round-trip (create → info/tree/cat/
//! stat/extract verify), overwrite semantics and the error surfaces.
//! No mkdwarfs binary anywhere (the dwarfs-t Writer is linked in).

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::TempDir;

fn rust_tfs() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tfs"))
}

fn run(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let mut cmd = Command::new(rust_tfs());
    cmd.args(args).current_dir(cwd);
    let out = cmd.output().expect("spawn tfs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn make_source(w: &TempDir) -> PathBuf {
    let src = w.0.join("app");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("one.txt"), "one").unwrap();
    std::fs::write(src.join("sub/two.txt"), "two").unwrap();
    std::fs::write(src.join("sub/three.txt"), "three").unwrap();
    src
}

#[test]
fn mkimage_roundtrip_ls_cat_stat_extract() {
    let w = TempDir::new("mkimg");
    let src = make_source(&w);
    // dwarfs-t-native (FlatBuffers metadata) images carry .tfs
    let img = w.0.join("app.tfs");

    let (rc, _, err) = run(
        &[
            "mkimage",
            "--format",
            "dwarfs",
            src.to_str().unwrap(),
            "-o",
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed");
    assert!(img.is_file());

    // The produced image is a real dwarfs image: mount and verify.
    let (rc, out, _) = run(&["info", img.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("Type: DwarFS"), "{out}");
    assert!(out.contains("Files: 3"), "{out}");
    assert!(out.contains("Directories: 1"), "{out}");

    let (rc, out, _) = run(&["tree", img.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("one.txt"), "{out}");
    assert!(out.contains("sub/"), "{out}");
    assert!(out.contains("two.txt"), "{out}");

    let (rc, out, _) = run(&["cat", img.to_str().unwrap(), "sub/three.txt"], &w.0);
    assert_eq!((rc, out.as_str()), (0, "three"));

    let (rc, out, _) = run(&["stat", img.to_str().unwrap(), "one.txt"], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("Type: file"), "{out}");
    assert!(out.contains("Size: 3.0 B (3 bytes)"), "{out}");

    let dest = w.0.join("extracted");
    std::fs::create_dir_all(&dest).unwrap();
    let (rc, _, _) = run(
        &[
            "extract",
            "-d",
            dest.to_str().unwrap(),
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(std::fs::read(dest.join("one.txt")).unwrap(), b"one");
    assert_eq!(std::fs::read(dest.join("sub/two.txt")).unwrap(), b"two");
}

/// The limnifs writer path (spec 20 §6): same tree in, same CLI
/// answers out — `Type: LimniFS` comes off the mounted backend, never
/// the extension. Runs on windows too (dwarfs+limnifs there).
#[test]
fn mkimage_limnifs_roundtrip_ls_cat_stat_extract() {
    let w = TempDir::new("mkimglim");
    let src = make_source(&w);
    let img = w.0.join("app.tfs");

    let (rc, _, err) = run(
        &[
            "mkimage",
            "--format",
            "limnifs",
            src.to_str().unwrap(),
            "-o",
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed");
    assert!(img.is_file());

    let (rc, out, _) = run(&["info", img.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("Type: LimniFS"), "{out}");
    assert!(out.contains("Files: 3"), "{out}");
    assert!(out.contains("Directories: 1"), "{out}");

    let (rc, out, _) = run(&["tree", img.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("one.txt"), "{out}");
    assert!(out.contains("two.txt"), "{out}");

    let (rc, out, _) = run(&["cat", img.to_str().unwrap(), "sub/three.txt"], &w.0);
    assert_eq!((rc, out.as_str()), (0, "three"));

    let dest = w.0.join("extracted");
    std::fs::create_dir_all(&dest).unwrap();
    let (rc, _, _) = run(
        &[
            "extract",
            "-d",
            dest.to_str().unwrap(),
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(std::fs::read(dest.join("one.txt")).unwrap(), b"one");
    assert_eq!(std::fs::read(dest.join("sub/two.txt")).unwrap(), b"two");
}

/// The default format is limnifs (spec 20 §6): a `--format`-less
/// mkimage writes LMFS-magic bytes that mount through the limnifs
/// backend. Dwarfs stays the explicit opt-in (`--format dwarfs`).
#[test]
fn mkimage_default_format_is_limnifs() {
    let w = TempDir::new("mkimgdefault");
    let src = make_source(&w);
    let img = w.0.join("app.tfs");

    let (rc, _, err) = run(
        &[
            "mkimage",
            src.to_str().unwrap(),
            "-o",
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed: {err}");

    let bytes = std::fs::read(&img).unwrap();
    assert_eq!(
        &bytes[..4],
        b"LMFS",
        "the default image opens with the limnifs magic"
    );

    let (rc, out, _) = run(&["info", img.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("Type: LimniFS"), "{out}");
}

/// The floor writer recipe (spec 20 §5) under the hazard shapes that
/// pinned it: duplicate small files (the shared-inline-table trigger —
/// the 0.2.50 reader mask rejects them, and a writer that skips the
/// table must still serve every byte), a large compressible text file
/// (the tournament's brotli pick that floor readers fail to decode),
/// and a large incompressible binary (the tournament-list coupling
/// that once mis-encoded binary drops to a zero-byte readback). Every
/// file must read back byte-exact through the limnifs backend.
#[test]
fn mkimage_limnifs_floor_recipe_readback() {
    let w = TempDir::new("mkimgfloor");
    let src = w.0.join("app");
    std::fs::create_dir_all(&src).unwrap();

    let dup = b"duplicate inline content: the same 200-ish bytes in three files, so the writer's inline dedup fires on every realistic tree. Padding padding padding padding padding!";
    assert!(dup.len() <= 4096);
    std::fs::write(src.join("dup-a.txt"), dup).unwrap();
    std::fs::write(src.join("dup-b.txt"), dup).unwrap();
    std::fs::write(src.join("dup-c.txt"), dup).unwrap();

    let text: Vec<u8> = (0..1400u32)
        .flat_map(|i| format!("line {i}: the quick brown fox jumps over the lazy dog, pack my box with five dozen liquor jugs\n").into_bytes())
        .collect();
    assert!(text.len() > 64 * 1024);
    std::fs::write(src.join("big-text.rb"), &text).unwrap();

    // Deterministic incompressible bytes (a splitmix64 stream) — the
    // store arm of the tournament.
    let mut state = 0x9E3779B97F4A7C15u64;
    let binary: Vec<u8> = (0..96 * 1024)
        .map(|_| {
            state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            (z ^ (z >> 31)) as u8
        })
        .collect();
    std::fs::write(src.join("big-binary.bundle"), &binary).unwrap();

    let img = w.0.join("app.tfs");
    let (rc, _, err) = run(
        &[
            "mkimage",
            src.to_str().unwrap(),
            "-o",
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed: {err}");

    let dest = w.0.join("extracted");
    std::fs::create_dir_all(&dest).unwrap();
    let (rc, _, err) = run(
        &[
            "extract",
            "-d",
            dest.to_str().unwrap(),
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "extract must succeed: {err}");

    for (name, want) in [
        ("dup-a.txt", dup.as_slice()),
        ("dup-b.txt", dup.as_slice()),
        ("dup-c.txt", dup.as_slice()),
        ("big-text.rb", text.as_slice()),
        ("big-binary.bundle", binary.as_slice()),
    ] {
        let got = std::fs::read(dest.join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"));
        assert_eq!(got.len(), want.len(), "{name}: byte count must round-trip");
        assert_eq!(got, want, "{name}: bytes must round-trip");
    }
}

/// Spec 20 §5 constraint 1, structurally: the floor recipe keeps the
/// shared-inline table OFF (`defaults.shared_inline = false`), so no
/// inode in an emitted image carries the 0x08 flag — the bit every
/// floor-era reader (limnifs-core < 0.2.53) rejects via its reserved
/// mask (limnifs#186). Mirrors upstream's #189 regression shape
/// (limnifs-write's shared_inline_round_trip.rs): three identical
/// inline-sized files are exactly the dedup trigger, so a knob-on
/// regression fails here loudly. The parse surface is the reader's:
/// the 0x08 flag decodes to `ContentHandle::SharedInline`, so "no
/// SharedInline handle" IS "no 0x08 on the wire".
#[test]
fn mkimage_limnifs_floor_recipe_emits_no_shared_inline() {
    let w = TempDir::new("mkimgnosil");
    let src = w.0.join("app");
    std::fs::create_dir_all(&src).unwrap();
    let dup =
        b"identical inline bytes in three small files - the shared-inline dedup trigger shape";
    assert!(dup.len() <= 4096);
    for name in ["dup-a.txt", "dup-b.txt", "dup-c.txt"] {
        std::fs::write(src.join(name), dup).unwrap();
    }

    let img = w.0.join("app.tfs");
    let (rc, _, err) = run(
        &[
            "mkimage",
            src.to_str().unwrap(),
            "-o",
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed: {err}");

    let bytes = std::fs::read(&img).unwrap();
    let mut cursor = limnifs_core::ManifestCursor::new(&bytes);
    limnifs_core::parse_manifest_header(&mut cursor).expect("header parses");
    limnifs_core::parse_feature_flags_section(&mut cursor).expect("feature flags parse");
    let reference =
        limnifs_core::parse_metadata_reference(&mut cursor).expect("metadata reference parses");
    let inline = reference
        .inline_metadata
        .as_deref()
        .expect("a self-contained image inlines the metadata");
    let blob = limnifs_core::parse_metadata_blob(&mut limnifs_core::ManifestCursor::new(inline))
        .expect("metadata blob parses");

    let mut dups = 0;
    for inode in &blob.inodes {
        match &inode.content_handle {
            limnifs_core::ContentHandle::InlineData(d) if d.as_slice() == dup => dups += 1,
            limnifs_core::ContentHandle::SharedInline(_) => {
                panic!("the floor recipe must not emit SharedInline inodes")
            }
            _ => {}
        }
    }
    assert_eq!(dups, 3, "all three duplicates ride plain inline data");
}

#[test]
fn mkimage_overwrites_existing_output() {
    let w = TempDir::new("mkimg3");
    let src = make_source(&w);
    let img = w.0.join("app.tfs");
    for round in 0..2 {
        let (rc, _, err) = run(
            &[
                "mkimage",
                "--format",
                "dwarfs",
                src.to_str().unwrap(),
                "-o",
                img.to_str().unwrap(),
            ],
            &w.0,
        );
        assert_eq!((rc, err.as_str()), (0, ""), "round {round}");
    }
}

#[test]
fn mkimage_error_surfaces() {
    let w = TempDir::new("mkimg2");
    let src = make_source(&w);

    for (args, expect) in [
        (
            vec!["mkimage", "--format", "zip", src.to_str().unwrap(), "-o", "x.zip"],
            "Error: mkimage failed: mkimage --format zip is not supported: the zip backend is read-only (only 'dwarfs' and 'limnifs' can be written)\n",
        ),
        (
            vec!["mkimage", "--format", "squashfs", src.to_str().unwrap(), "-o", "x.sqfs"],
            "Error: mkimage failed: mkimage --format squashfs is not supported (LGPL; opt-in source builds only)\n",
        ),
        (
            vec!["mkimage", "--format", "foo", src.to_str().unwrap(), "-o", "x"],
            "Error: mkimage failed: unsupported image format 'foo' (supported: dwarfs, limnifs)\n",
        ),
        (
            vec!["mkimage", "--format", "dwarfs", "nosuchdir", "-o", "x.tfs"],
            "Error: mkimage failed: source directory not found: nosuchdir\n",
        ),
    ] {
        let (rc, _, err) = run(&args, &w.0);
        assert_eq!((rc, err.as_str()), (1, expect), "{args:?}");
    }

    // The writer's own failure surface (output directory missing).
    let (rc, _, err) = run(
        &[
            "mkimage",
            "--format",
            "dwarfs",
            src.to_str().unwrap(),
            "-o",
            "no/such/dir/x.tfs",
        ],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert!(
        err.starts_with("Error: mkimage failed: dwarfs writer: "),
        "{err}"
    );
}
