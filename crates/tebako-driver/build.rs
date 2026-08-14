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
        .unwrap_or_else(|e| panic!("no C compiler for the spec 22 macOS interpose dylib: {e}"));
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
            "the spec 22 macOS interpose dylib failed to build: {}\n  {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            display,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
