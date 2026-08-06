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
/// the extension.
#[test]
#[cfg(not(windows))] // windows ships a dwarfs-only tfs (TODO.v2-1/02)
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
