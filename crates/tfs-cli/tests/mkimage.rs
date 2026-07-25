//! mkimage tests: mkdwarfs round-trip (create → ls/cat/stat/extract
//! verify) and the error surfaces. mkdwarfs is located via
//! TEBAKO_MKDWARFS, the dwarfs-t build tree, or PATH; tests skip without it.

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::TempDir;

fn find_mkdwarfs() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TEBAKO_MKDWARFS") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let well_known = PathBuf::from("/Users/mulgogi/src/tamatebako/dwarfs-t/build/mkdwarfs");
    if well_known.is_file() {
        return Some(well_known);
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let cand = PathBuf::from(dir).join("mkdwarfs");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn rust_tfs() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tfs");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tfs binary not built")
}

fn run(args: &[&str], cwd: &Path, env: Option<(&str, &str)>) -> (i32, String, String) {
    let mut cmd = Command::new(rust_tfs());
    cmd.args(args).current_dir(cwd);
    if let Some((k, v)) = env {
        cmd.env(k, v);
    }
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
    let Some(mkd) = find_mkdwarfs() else {
        eprintln!("skipping mkimage test: no mkdwarfs (set TEBAKO_MKDWARFS)");
        return;
    };
    let w = TempDir::new("mkimg");
    let src = make_source(&w);
    let img = w.0.join("app.dwarfs");
    let env = Some(("TEBAKO_MKDWARFS", mkd.to_str().unwrap()));

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
        env,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed");
    assert!(img.is_file());

    // The produced image is a real dwarfs image: mount and verify.
    let (rc, out, _) = run(&["info", img.to_str().unwrap()], &w.0, None);
    assert_eq!(rc, 0);
    assert!(out.contains("Type: DwarFS"), "{out}");
    assert!(out.contains("Files: 3"), "{out}");
    assert!(out.contains("Directories: 1"), "{out}");

    let (rc, out, _) = run(&["tree", img.to_str().unwrap()], &w.0, None);
    assert_eq!(rc, 0);
    assert!(out.contains("one.txt"), "{out}");
    assert!(out.contains("sub/"), "{out}");
    assert!(out.contains("two.txt"), "{out}");

    let (rc, out, _) = run(&["cat", img.to_str().unwrap(), "sub/three.txt"], &w.0, None);
    assert_eq!((rc, out.as_str()), (0, "three"));

    let (rc, out, _) = run(&["stat", img.to_str().unwrap(), "one.txt"], &w.0, None);
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
        None,
    );
    assert_eq!(rc, 0);
    assert_eq!(std::fs::read(dest.join("one.txt")).unwrap(), b"one");
    assert_eq!(std::fs::read(dest.join("sub/two.txt")).unwrap(), b"two");
}

#[test]
fn mkimage_error_surfaces() {
    let w = TempDir::new("mkimg2");
    let src = make_source(&w);
    let fake_tool = w.0.join("true");
    std::fs::write(&fake_tool, b"").unwrap();

    for (args, expect) in [
        (
            vec!["mkimage", "--format", "zip", src.to_str().unwrap(), "-o", "x.zip"],
            "Error: mkimage failed: mkimage --format zip is not supported: the zip backend is read-only (only 'dwarfs' can be written)\n",
        ),
        (
            vec!["mkimage", "--format", "squashfs", src.to_str().unwrap(), "-o", "x.sqfs"],
            "Error: mkimage failed: mkimage --format squashfs is not supported (LGPL; opt-in source builds only)\n",
        ),
        (
            vec!["mkimage", "--format", "foo", src.to_str().unwrap(), "-o", "x"],
            "Error: mkimage failed: unsupported image format 'foo' (supported: dwarfs)\n",
        ),
        (
            vec!["mkimage", "--format", "dwarfs", "nosuchdir", "-o", "x.dwarfs"],
            "Error: mkimage failed: source directory not found: nosuchdir\n",
        ),
    ] {
        let (rc, _, err) = run(&args, &w.0, None);
        assert_eq!((rc, err.as_str()), (1, expect), "{args:?}");
    }

    // mkdwarfs not found: point the env override at a nonexistent path.
    let (rc, _, err) = run(
        &[
            "mkimage",
            "--format",
            "dwarfs",
            src.to_str().unwrap(),
            "-o",
            "x.dwarfs",
        ],
        &w.0,
        Some(("TEBAKO_MKDWARFS", "/nonexistent/mkdwarfs")),
    );
    assert_eq!(rc, 1);
    assert_eq!(
        err,
        "Error: mkimage failed: mkdwarfs not found: /nonexistent/mkdwarfs\n"
    );

    // A tool that exits non-zero surfaces its exit code.
    let fail_tool = w.0.join("fail.sh");
    std::fs::write(&fail_tool, "#!/bin/sh\nexit 3\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&fail_tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let (rc, _, err) = run(
        &[
            "mkimage",
            "--format",
            "dwarfs",
            src.to_str().unwrap(),
            "-o",
            "x.dwarfs",
        ],
        &w.0,
        Some(("TEBAKO_MKDWARFS", fail_tool.to_str().unwrap())),
    );
    assert_eq!(rc, 1);
    assert!(err.contains("mkdwarfs failed (exit code"), "{err}");
}
