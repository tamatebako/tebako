//! Build script for sqfs-sys.
//!
//! `vendored` mode (the only mode in v1): installs `squashfs-tools-ng`
//! (libsquashfs) and its compression dependencies via vcpkg using this
//! crate's own manifest, then links them statically. The vcpkg baseline is
//! pinned in `vcpkg.json` (same baseline libtfs uses, keeping the
//! libsquashfs version identical on both sides of the parity gate).
//!
//! Environment knobs:
//!
//! - `SQFS_SYS_VCPKG_ROOT` (or `DWARFS_RS_VCPKG_ROOT`, or `VCPKG_ROOT`) —
//!   vcpkg installation root. REQUIRED.
//! - `SQFS_SYS_VCPKG_TRIPLET` — vcpkg triplet; default is derived from the
//!   Rust target (e.g. `arm64-osx-static`, `x64-linux-static`).
//! - `SQFS_SYS_VERBOSE=1` — stream vcpkg output instead of swallowing it.
//!
//! Notes:
//! - squashfs-tools-ng is a POSIX/autotools-only library (no Windows port,
//!   same restriction as the C++ side).
//! - The install is cached in the cargo OUT_DIR; the vcpkg binary archive
//!   cache (shared with the dwarfs chain) keeps repeat builds fast.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Compression libraries libsquashfs links against (vcpkg names).
const DEPS: &[&str] = &["z", "lz4", "lzma", "zstd"];

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=abi_check.c");
    println!("cargo:rerun-if-changed=shim.c");
    println!("cargo:rerun-if-changed=vcpkg.json");
    for var in [
        "SQFS_SYS_VCPKG_ROOT",
        "DWARFS_RS_VCPKG_ROOT",
        "VCPKG_ROOT",
        "SQFS_SYS_VCPKG_TRIPLET",
        "SQFS_SYS_VCPKG_INSTALLED_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    // ---------------------------------------------------------------
    // vcpkg root + triplet
    // ---------------------------------------------------------------
    let vcpkg_root = env::var("SQFS_SYS_VCPKG_ROOT")
        .or_else(|_| env::var("DWARFS_RS_VCPKG_ROOT"))
        .or_else(|_| env::var("VCPKG_ROOT"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            panic!(
                "vcpkg root not set.\n\
                 Set SQFS_SYS_VCPKG_ROOT (or DWARFS_RS_VCPKG_ROOT, or VCPKG_ROOT)\n\
                 to a vcpkg installation."
            )
        });
    let vcpkg_exe = vcpkg_root.join(if target.contains("windows") {
        "vcpkg.exe"
    } else {
        "vcpkg"
    });
    if !vcpkg_exe.exists() {
        panic!(
            "vcpkg executable not found at {} (bootstrap vcpkg first)",
            vcpkg_exe.display()
        );
    }

    if target.contains("windows") && env::var("SQFS_SYS_VCPKG_TRIPLET").is_err() {
        panic!(
            "squashfs-tools-ng is a POSIX/autotools-only library and cannot be \
             built for Windows (same restriction as the C++ libtfs backend). \
             The tfs consumers gate this feature off per-target on Windows \
             (TODO.v2-1/02) — reaching this panic means it was enabled by hand."
        );
    }

    let triplet = env::var("SQFS_SYS_VCPKG_TRIPLET").unwrap_or_else(|_| default_triplet(&target));
    let verbose = env::var("SQFS_SYS_VERBOSE").is_ok();

    // ---------------------------------------------------------------
    // vcpkg install (manifest mode, into OUT_DIR)
    //
    // SQFS_SYS_VCPKG_INSTALLED_DIR short-circuits the install: CI
    // pre-installs the packages in a serialized step (parallel build
    // scripts would otherwise race dwarfs-t-sys's CMake-driven vcpkg run
    // on the vcpkg-root filesystem lock — the dwarfs chain holds it for
    // ~45 minutes on a cold archive cache). Local builds self-install,
    // retrying on that lock for up to ~60 minutes.
    // ---------------------------------------------------------------
    let preinstalled = env::var("SQFS_SYS_VCPKG_INSTALLED_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|d| d.join("include/sqfs").exists());
    let install_root = out_dir.join("vcpkg_installed");
    let prefix = if let Some(dir) = preinstalled {
        dir
    } else {
        if !install_root.join(&triplet).join("include").exists() {
            let mut attempt = 0u32;
            loop {
                attempt += 1;
                let mut cmd = Command::new(&vcpkg_exe);
                cmd.arg("install")
                    .arg("--vcpkg-root")
                    .arg(&vcpkg_root)
                    .arg("--x-wait-for-lock")
                    .arg("--x-manifest-root")
                    .arg(&manifest_dir)
                    .arg("--x-install-root")
                    .arg(&install_root)
                    .arg("--triplet")
                    .arg(&triplet)
                    .arg("--overlay-triplets")
                    .arg(manifest_dir.join("vcpkg_triplets"))
                    .arg("--overlay-ports")
                    .arg(manifest_dir.join("vcpkg_ports"))
                    .env("VCPKG_ROOT", &vcpkg_root)
                    .env_remove("VCPKG_MANIFEST_FEATURES");
                match run(cmd, verbose) {
                    Ok(()) => break,
                    Err(e) => {
                        if attempt >= 120 {
                            panic!(
                                "vcpkg install squashfs-tools-ng failed after {attempt} attempts: {e}\n\
                                 (hint: pre-install and set SQFS_SYS_VCPKG_INSTALLED_DIR to avoid \
                                 the vcpkg-root lock race with dwarfs-t-sys)"
                            );
                        }
                        println!(
                            "cargo:warning=vcpkg install failed (attempt {attempt}, retrying in 30s): {e}"
                        );
                        std::thread::sleep(std::time::Duration::from_secs(30));
                    }
                }
            }
        }
        install_root.join(&triplet)
    };
    let include = prefix.join("include");
    let lib = prefix.join("lib");

    // ---------------------------------------------------------------
    // ABI cross-check + the small C shim
    // ---------------------------------------------------------------
    cc::Build::new()
        .file("abi_check.c")
        .file("shim.c")
        .include(&include)
        .compile("sqfs_abi_check");

    // ---------------------------------------------------------------
    // Link
    // ---------------------------------------------------------------
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=squashfs");
    for dep in DEPS {
        if lib.join(format!("lib{dep}.a")).exists() || lib.join(format!("{dep}.lib")).exists() {
            println!("cargo:rustc-link-lib=static={dep}");
        }
    }
}

fn default_triplet(target: &str) -> String {
    match target {
        "aarch64-apple-darwin" => "arm64-osx-static",
        "x86_64-apple-darwin" => "x64-osx-static",
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => "x64-linux-static",
        "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => "arm64-linux-static",
        other => panic!(
            "no default vcpkg triplet for target {other}; set SQFS_SYS_VCPKG_TRIPLET explicitly"
        ),
    }
    .to_string()
}

fn run(mut cmd: Command, verbose: bool) -> Result<(), String> {
    let display = format!("{cmd:?}");
    let output = if verbose {
        let status = cmd
            .status()
            .map_err(|e| format!("failed to spawn: {e}\n  {display}"))?;
        return if status.success() {
            Ok(())
        } else {
            Err(format!("exit status: {status}\n  {display}"))
        };
    } else {
        cmd.output()
            .map_err(|e| format!("failed to spawn: {e}\n  {display}"))?
    };
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "exit status: {}\n  {display}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
