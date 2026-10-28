//! The EXE shape's proof (spec 22 §3.6): the family linked INTO a binary
//! instead of injected into it.
//!
//! The interposed-dylib tests in `e2e.rs` prove the injection delivery. This
//! file proves the other one: a program that links `libtfs_preload.a`
//! (`--features exe-shape`, which compiles the library constructor out) and
//! calls `tebako_preload_init()` at its own boot head gets the same routing
//! for its own C-library calls — no `LD_PRELOAD`, no environment injection,
//! no dynamic lookup.
//!
//! The mechanism is ELF symbol preemption: the family's definitions live in
//! the linked archive and every reference in the final binary, including the
//! statically linked C library a runtime carries, resolves to them. That is
//! exactly the property the runtime exe needs, since the interpreter's
//! in-process OpenSSL reads `fopen64`/`openat64` inside the process and no
//! injected image exists to catch them.
//!
//! Not applicable on macOS: dyld honours `__interpose` tuples only from a
//! dylib image, so the exe shape there is the driver's micro dylib and this
//! test skips with a note.
//!
//! Skip policy matches `e2e.rs`: skip (pass trivially, note on stderr) when
//! the platform is not linux, when `cc` is absent, or when the archive has
//! not been built; a `cc` that runs but fails is a hard failure.

#![cfg(unix)]
#![allow(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The archive cargo builds for `crate-type = ["staticlib", ...]`.
///
/// `TEBAKO_EXE_SHAPE_ARCHIVE` wins when set: CI builds the exe-shape archive
/// into its OWN target directory, because the same crate's cdylib is what the
/// injected-shim suites load, and a `--features exe-shape` build in the shared
/// directory leaves a constructor-less copy beside the real one — the loader
/// then picks one of the hashed `.so` files, and every mount silently
/// disappears (the `tebako-bootstrap` jail suite catches it: no denial is
/// journaled). Isolating the archive is the fix; the size of the build is the
/// price.
fn staticlib() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("TEBAKO_EXE_SHAPE_ARCHIVE") {
        let p = PathBuf::from(explicit);
        return p.exists().then_some(p);
    }
    let dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target")));
    let candidate = dir.join("debug").join("libtfs_preload.a");
    candidate.exists().then_some(candidate)
}

fn skip(why: &str) -> bool {
    eprintln!("exe-shape proof skipped: {why}");
    true
}

/// A tiny image with one file at a known path, written with the crate's own
/// zip backend (no external tool, same as the e2e fixtures).
fn image(dir: &Path) -> PathBuf {
    let path = dir.join("exe-shape.zip");
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("hello.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut zip, b"exe-shape payload\n").unwrap();
    zip.finish().unwrap();
    path
}

/// The probe: init, then read the in-image path through plain `fopen`.
fn probe_source() -> &'static str {
    r#"
#include <stdio.h>
#include <string.h>

extern int tebako_preload_init(void);

int main(int argc, char **argv) {
    if (argc < 2) return 64;
    int rc = tebako_preload_init();
    printf("init=%d\n", rc);
    fflush(stdout);
    if (rc != 0) return 65;

    FILE *f = fopen(argv[1], "r");
    if (!f) {
        printf("fopen=FAILED\n");
        return 66;
    }
    char buf[64];
    memset(buf, 0, sizeof(buf));
    size_t n = fread(buf, 1, sizeof(buf) - 1, f);
    fclose(f);
    printf("read=%zu:%s", n, buf);
    return 0;
}
"#
}

#[test]
fn a_binary_that_links_the_family_reads_an_image_without_injection() {
    if !cfg!(target_os = "linux") {
        assert!(skip(
            "the exe shape's static preemption is ELF-only; macOS uses the driver dylib"
        ));
        return;
    }
    if Command::new("cc").arg("--version").output().is_err() {
        assert!(skip("no C compiler on PATH"));
        return;
    }
    let Some(archive) = staticlib() else {
        assert!(skip(
            "libtfs_preload.a not built (run `cargo build -p libtfs-preload --features exe-shape`)"
        ));
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let image = image(dir.path());
    let src = dir.path().join("probe.c");
    std::fs::write(&src, probe_source()).unwrap();
    let exe = dir.path().join("probe");

    // --whole-archive: the archive's interpose objects are referenced by
    // nobody in the program's own code, so without it the linker drops them
    // and `fopen` binds to libc.
    let link = Command::new("cc")
        .args(["-O1", "-o"])
        .arg(&exe)
        .arg(&src)
        .arg("-Wl,--whole-archive")
        .arg(&archive)
        .arg("-Wl,--no-whole-archive")
        // The archive carries the C++ dwarfs backend (the only C++ in the
        // tree), so the link needs the C++ runtime too — the same flag the
        // factory's link line will need.
        .args(["-lstdc++", "-ldl", "-lpthread", "-lm"])
        .output()
        .expect("cc spawn");
    assert!(
        link.status.success(),
        "linking the probe failed:\n{}",
        String::from_utf8_lossy(&link.stderr)
    );

    let mounts = format!("{}:/img", image.display());
    let out = Command::new(&exe)
        .arg("/img/hello.txt")
        .env("TEBAKO_TFS_MOUNTS", &mounts)
        .env_remove("LD_PRELOAD")
        .env_remove("TEBAKO_JAIL")
        .output()
        .expect("probe spawn");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "probe failed (status {:?})\nstdout:\n{stdout}\nstderr:\n{stderr}",
        out.status
    );
    assert!(
        stdout.contains("init=0"),
        "the explicit init must report success:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("read=18:exe-shape payload"),
        "the in-image read must come from the image, not the host:\n{stdout}\n{stderr}"
    );
}
