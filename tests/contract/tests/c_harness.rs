//! The C harness leg: compile `c_harness/mount_read.c` against the Rust
//! libtfs (cdylib) with the system C compiler and run it end-to-end —
//! proving a plain C consumer can mount a zip and read a file through the
//! `tebako_fs_*` ABI.
//!
//! Skips (with a message) when no C compiler is available or when the
//! cdylib has not been built yet (`cargo build -p tfs`).

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::{build_fixture_zip, TempDir};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn libtfs_cdylib() -> Option<PathBuf> {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root().join("target"));
    for profile in ["debug", "release"] {
        for name in ["libtfs.dylib", "libtfs.so", "tfs.dll"] {
            let candidate = target_dir.join(profile).join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn have_cc() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn c_harness_mounts_and_reads_zip() {
    if !have_cc() {
        eprintln!("skipping C harness test: no `cc` available");
        return;
    }
    let dylib = match libtfs_cdylib() {
        Some(d) => d,
        None => {
            // `cargo test` alone does not build the cdylib (dependencies are
            // built as rlibs only); build it ourselves so the harness always
            // runs, never silently skips.
            eprintln!("libtfs cdylib not found; building it (`cargo build -p tfs`)");
            let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
                .args(["build", "-p", "tfs"])
                .current_dir(workspace_root())
                .status()
                .expect("spawn cargo build -p tfs");
            assert!(status.success(), "cargo build -p tfs failed");
            libtfs_cdylib().expect("cdylib must exist after building it")
        }
    };

    let tmp = TempDir::new("c-harness");
    let zip_path = tmp.0.join("test.zip");
    build_fixture_zip(&zip_path);
    let harness_bin = tmp.0.join("mount_read");
    let harness_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("c_harness/mount_read.c");

    let compile = Command::new("cc")
        .arg("-O1")
        .arg(&harness_src)
        .arg(&dylib)
        .arg(format!("-Wl,-rpath,{}", dylib.parent().unwrap().display()))
        .arg("-o")
        .arg(&harness_bin)
        .output()
        .expect("spawn cc");
    assert!(
        compile.status.success(),
        "cc failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&harness_bin)
        .arg(&zip_path)
        .arg("/__tebako_test__/content/hello.txt")
        .output()
        .expect("run harness");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        run.status.success(),
        "harness failed: stdout={stdout} stderr={stderr}"
    );

    assert!(
        stdout.contains("backend ZIP"),
        "expected backend name: {stdout}"
    );
    assert!(stdout.contains("size=13"), "expected file size: {stdout}");
    assert!(
        stdout.contains("content=Hello, World!"),
        "expected file content: {stdout}"
    );
}
