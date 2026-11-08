//! Build script for tebako-driver.
//!
//! macOS only: compiles `loader_interpose.c` — the spec 22 class-L micro
//! interpose-dylib — into `libtebako_loader_interpose.dylib` under
//! OUT_DIR, where `src/ffi/interpose.rs` embeds its bytes for the
//! boot-head self-insertion. Every other target builds nothing (the
//! module compiles out there), so the `cc` build-dependency is
//! macOS-gated in Cargo.toml.
//!
//! The link is `-dynamiclib -undefined dynamic_lookup`: the dylib binds
//! the exe's `tebako_fs_*` exports at run time (one VFS context, no
//! third artifact — spec 22 §2 "Phase 1 delivery"), so the tebako_fs_*
//! references stay undefined at link time by design.
//!
//! macOS only, second product: the `tebako-dlprobe` bin (tebako#989's
//! class-L dlopen probe) is sealed with the tfs crate's canonical
//! exports.txt via `-exported_symbols_list`, so the inserted interpose
//! dylib's dynamic_lookup references resolve against the probe exe
//! exactly the way they resolve against a signed runtime exe.

use std::env;
use std::path::Path;

const SOURCE: &str = "loader_interpose.c";
const DYLIB: &str = "libtebako_loader_interpose.dylib";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={SOURCE}");
    for var in ["CC", "CFLAGS", "MACOSX_DEPLOYMENT_TARGET"] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    let target = env::var("TARGET").unwrap();
    if !target.ends_with("-apple-darwin") {
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir).join(DYLIB);
    let source = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join(SOURCE);

    // The cc crate's compiler discovery (CC/CFLAGS, -arch for the cargo
    // TARGET), driven by hand because cc's own compile() archives — the
    // product here is a dylib, not a static library.
    let tool = cc::Build::new()
        .try_get_compiler()
        .unwrap_or_else(|e| panic!("no C compiler for the macOS interpose dylib: {e}"));
    let mut cmd = tool.to_command();
    cmd.arg("-dynamiclib")
        .arg("-undefined")
        .arg("dynamic_lookup")
        .arg("-std=c99")
        .arg("-o")
        .arg(&out)
        .arg(&source);
    let display = format!("{cmd:?}");
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn the C compiler: {e}\n  {display}"));
    if !output.status.success() {
        panic!(
            "the macOS interpose dylib failed to build: {}\n  {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            display,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    seal_dlprobe_exports(&out_dir);
}

/// The tebako#989 probe's link seal: `tebako-dlprobe` exports the
/// `tebako_fs_*` surface (the interpose dylib binds the exe's own exports
/// via `-undefined dynamic_lookup` — they must be dynamic globals in the
/// Mach-O, and the list doubles as the force that pulls the defining
/// objects out of the tfs rlib). The single source is the tfs crate's
/// checked-in `exports.txt`; the linker wants Mach-O spellings
/// (`_`-prefixed — ld as of Xcode 15 answers an unprefixed entry with an
/// unsatisfiable "initial-undefines" error), so an underscored copy is
/// generated per build under OUT_DIR and handed to exactly this bin.
fn seal_dlprobe_exports(out_dir: &str) {
    const BIN: &str = "tebako-dlprobe";
    let source = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../tfs/exports.txt");
    println!("cargo:rerun-if-changed={}", source.display());
    let text = std::fs::read_to_string(&source).unwrap_or_else(|e| {
        panic!(
            "the {BIN} export seal cannot read the tfs crate's canonical export \
             list at '{}' ({e}) — the probe's tebako_fs_* surface comes from \
             crates/tfs/exports.txt or nowhere",
            source.display()
        )
    });
    let mut sealed = String::new();
    for line in text.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        sealed.push('_');
        sealed.push_str(name);
        sealed.push('\n');
    }
    let out = Path::new(out_dir).join("tebako-dlprobe-exports.txt");
    std::fs::write(&out, sealed).unwrap_or_else(|e| {
        panic!(
            "the {BIN} export seal cannot write '{}': {e}",
            out.display()
        )
    });
    println!(
        "cargo::rustc-link-arg-bin={BIN}=-Wl,-exported_symbols_list,{}",
        out.display()
    );
}
