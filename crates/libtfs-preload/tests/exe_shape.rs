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

    // The raw archive bundles the whole dependency graph, and cargo packs
    // the same vendored codec object once per crate path — the enc
    // feature's botan and the dwarfs backend each carry zlib, so a
    // --whole-archive link of the RAW archive dies on multiple
    // definitions (tebako#953's first CI run). The production consumption
    // is the staged archive, where arscope folds the byte-identical
    // members; mirror it with a fold-only pass (--keep-prefix "" keeps
    // every name — the probe links against the libc spellings).
    let folded = dir.path().join("libtfs_preload.folded.a");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf();
    let fold = Command::new("cargo")
        .args(["run", "-q", "-p", "tebako-arscope", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .arg("--")
        .arg(&archive)
        .arg(&folded)
        .args(["--keep-prefix", "", "--fold-identical"])
        .output()
        .expect("arscope spawn");
    assert!(
        fold.status.success(),
        "folding the archive failed:\n{}",
        String::from_utf8_lossy(&fold.stderr)
    );

    // --whole-archive: the archive's interpose objects are referenced by
    // nobody in the program's own code, so without it the linker drops them
    // and `fopen` binds to libc.
    let link = Command::new("cc")
        .args(["-O1", "-o"])
        .arg(&exe)
        .arg(&src)
        .arg("-Wl,--whole-archive")
        .arg(&folded)
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

/// Run a probe binary with a deadline: the engine-IO re-entry class
/// (tebako#964) WEDGES the process instead of failing it, and an
/// unbounded `Command::output` would hang the CI leg to its job
/// timeout. A probe that outlives the deadline fails the test with
/// whatever output it produced.
fn run_with_timeout(
    exe: &Path,
    arg: &str,
    mounts: &str,
    exec_cache: Option<&Path>,
    timeout: std::time::Duration,
) -> std::process::Output {
    let mut cmd = Command::new(exe);
    cmd.arg(arg)
        .env("TEBAKO_TFS_MOUNTS", mounts)
        .env_remove("LD_PRELOAD")
        .env_remove("TEBAKO_JAIL")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if let Some(cache) = exec_cache {
        cmd.env("TEBAKO_EXEC_CACHE", cache);
    }
    let mut child = cmd.spawn().expect("probe spawn");
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if child.try_wait().expect("probe wait").is_some() {
            return child.wait_with_output().expect("probe output");
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let out = child.wait_with_output().expect("probe output");
            panic!(
                "the probe did not exit within {timeout:?} — the engine-IO re-entry class (tebako#964) wedges here\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// The materialization probe (the runtime's fiddle path, spec 22 §3.6's
/// class-L seam): with the family armed, a VFS-resident library
/// materializes to the exec cache through the engine's OWN C ABI and
/// the real dlopen loads that copy. The pin-roll's first built runtime
/// died here (tebako-runtime-ruby#306's smoke): the extract's host
/// writes re-entered the armed family — a lock recursion in this
/// single-copy link (tebako#964), a phantom host path in the runtime's
/// two-copy exe — and the dlopen hit an absent file. The probe also
/// opens the materialized copy's parent directory: a dlmap spelling of
/// the mount ROOT is a directory tail and must answer the real host
/// dir, never the materialize answer's EISDIR (the exit-cleanup tree
/// walk's class).
fn dlprobe_source() -> &'static str {
    r#"
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

extern int tebako_preload_init(void);
extern char *tebako_fs_dlmap2file(const char *path);

int main(int argc, char **argv) {
    if (argc < 2) return 64;
    int rc = tebako_preload_init();
    printf("init=%d\n", rc);
    fflush(stdout);
    if (rc != 0) return 65;

    char *host = tebako_fs_dlmap2file(argv[1]);
    if (!host) { printf("dlmap2file=NULL\n"); return 66; }
    printf("host=%s\n", host);
    fflush(stdout);

    void *h = dlopen(host, RTLD_NOW);
    if (!h) { printf("dlopen=FAILED %s\n", dlerror()); return 67; }
    int (*answer)(void) = (int(*)(void))dlsym(h, "probe_answer");
    printf("answer=%d\n", answer ? answer() : -1);
    if (!answer || answer() != 42) return 68;

    char dirbuf[4096];
    strncpy(dirbuf, host, sizeof(dirbuf) - 1);
    dirbuf[sizeof(dirbuf) - 1] = 0;
    char *slash = strrchr(dirbuf, '/');
    if (!slash) return 69;
    *slash = 0;
    int dfd = open(dirbuf, O_RDONLY | O_DIRECTORY);
    printf("diropen=%s\n", dfd >= 0 ? "ok" : "FAILED");
    if (dfd < 0) return 70;
    close(dfd);
    return 0;
}
"#
}

/// A tiny image carrying a shared library at a known path: the library
/// is compiled with cc (the probe's fixture needs a real ELF), then
/// packed with the crate's own zip backend like the other fixtures.
fn lib_image(dir: &Path) -> Option<PathBuf> {
    let src = dir.join("probe_lib.c");
    std::fs::write(&src, "int probe_answer(void) { return 42; }\n").unwrap();
    let lib = dir.join("libprobe.so");
    let cc = Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&lib)
        .arg(&src)
        .output()
        .ok()?;
    if !cc.status.success() {
        return None;
    }
    let path = dir.join("exe-shape-lib.zip");
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("libfake.so", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut zip, &std::fs::read(&lib).unwrap()).unwrap();
    zip.finish().unwrap();
    Some(path)
}

#[test]
fn a_linked_binary_materializes_and_dlopens_a_vfs_library() {
    if !cfg!(target_os = "linux") {
        assert!(skip("the exe shape's static preemption is ELF-only"));
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
    let Some(image) = lib_image(dir.path()) else {
        assert!(skip("cc could not build the fixture library"));
        return;
    };
    let src = dir.path().join("dlprobe.c");
    std::fs::write(&src, dlprobe_source()).unwrap();
    let exe = dir.path().join("dlprobe");

    // The same fold-then-link the first proof uses.
    let folded = dir.path().join("libtfs_preload.folded.a");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf();
    let fold = Command::new("cargo")
        .args(["run", "-q", "-p", "tebako-arscope", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .arg("--")
        .arg(&archive)
        .arg(&folded)
        .args(["--keep-prefix", "", "--fold-identical"])
        .output()
        .expect("arscope spawn");
    assert!(
        fold.status.success(),
        "folding the archive failed:\n{}",
        String::from_utf8_lossy(&fold.stderr)
    );

    let link = Command::new("cc")
        .args(["-O1", "-o"])
        .arg(&exe)
        .arg(&src)
        .arg("-Wl,--whole-archive")
        .arg(&folded)
        .arg("-Wl,--no-whole-archive")
        // --export-dynamic: the engine-IO guard crosses the C ABI /
        // family boundary by dlsym(RTLD_DEFAULT) — the production link
        // line's own flag (spec 22 §3.6), and without it the probe's
        // guard resolves nothing and the #964 wedge stands.
        .arg("-Wl,--export-dynamic")
        .args(["-lstdc++", "-ldl", "-lpthread", "-lm"])
        .output()
        .expect("cc spawn");
    assert!(
        link.status.success(),
        "linking the dlprobe failed:\n{}",
        String::from_utf8_lossy(&link.stderr)
    );

    let mounts = format!("{}:/img", image.display());
    let exec_cache = dir.path().join("exec-cache");
    let out = run_with_timeout(
        &exe,
        "/img/libfake.so",
        &mounts,
        Some(&exec_cache),
        std::time::Duration::from_secs(60),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "dlprobe failed (status {:?})\nstdout:\n{stdout}\nstderr:\n{stderr}",
        out.status
    );
    assert!(
        stdout.contains("init=0"),
        "the explicit init must report success:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("answer=42"),
        "the materialized library must answer 42 — a NULL dlmap2file or an absent host copy is the tebako-runtime-ruby#306 failure:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("diropen=ok"),
        "the materialized copy's parent dir (a mount-root dlmap spelling) must answer the real host dir:\n{stdout}\n{stderr}"
    );
}
