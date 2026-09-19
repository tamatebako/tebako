#!/usr/bin/env bash
# ci/windows-arm64-msvc-build.sh — the windows-arm64 tools leg (roadmap 02
# Gap 1, phase 1): cross-build the product tools for
# aarch64-pc-windows-msvc on a windows-x64 runner under the VS x64_arm64
# cross environment (GitHub has no arm64 windows hosted runners), prove
# the produced PE images are ARM64, gate the bootstrap size, and name the
# tools the vendored-rnp chain still blocks.
#
# RELEASE-GATED OFF by construction: the calling job carries no upload
# steps and is not in finalize's needs — a green phase 1 proves the leg,
# it ships nothing (no half-served platform; roadmap 02's own rule).
#
# Environment this script expects (the calling job exports all of it):
# - the msvc-dev-cmd (x64_arm64) env: cl targets ARM64, nmake is on PATH
#   (botan-src's windows branch builds with nmake), LIB points at the
#   arm64 Windows SDK / VC libs;
# - DWARFS_RS_VCPKG_ROOT / VCPKG_ROOT — the run-vcpkg root (dwarfs-t-sys's
#   CMake/vcpkg build);
# - DWARFS_RS_VCPKG_INSTALLED_DIR=C:/vcpkg-inst — the short-root MAX_PATH
#   dodge (same root the x64-mingw-static leg pins). The baseline-restore
#   step is deliberately absent: no arm64-windows-static-md baseline is
#   published, so the ports build in-leg — that build IS the phase-1
#   prove-out (roadmap 02: "dwarfs-t needs a leg + CI proof");
# - BOOTSTRAP_SIZE_BUDGET — the workflow env (the gate's single owner);
#   this script flows it, stage.sh enforces the same value on the
#   shipping legs.
set -euo pipefail

TARGET=aarch64-pc-windows-msvc
REL="target/${TARGET}/release"

# The VS cross environment must be active before anything else.
if [ -z "${VCToolsInstallDir:-}" ]; then
  echo "::error::windows-arm64: the VS x64_arm64 cross environment is not active (VCToolsInstallDir unset) — the msvc-dev-cmd step must run before this script"
  exit 1
fi
cl 2>&1 | head -2 || true

# --- 1. the signer-free build ----------------------------------------------
# tebako-bootstrap keeps its OWN invocation: cargo feature unification
# with the other tools would re-enable tebako-resolve's `git` stack
# inside the size-gated loader (the v0.3.0 run 32975547796 lesson).
cargo build --release --target "$TARGET" -p tebako-bootstrap

# tfs on windows is dwarfs-only by target split (squashfs is POSIX-only;
# enc needs rnp — see the blocker below): the same feature set every
# windows consumer passes (crates/*/Cargo.toml, cfg(windows) tfs deps).
# Building tfs as a package also links the cdylib — the full libdwarfs_c
# + vcpkg arm64-windows-static-md closure compiles AND links under MSVC
# ARM64, which is the leg's core prove-out. tebako-driver (the runtime
# exe's mount engine) and the spec-29 wrapper exe ride along.
cargo build --release --target "$TARGET" \
  -p tfs --no-default-features --features vendored-dwarfs,backend-limnifs \
  -p tebako-driver -p tebako-runtime-launcher

# --- 2. size gates ----------------------------------------------------------
# The bootstrap's < 3 MiB loader gate holds on every platform (item 22);
# the value flows from the workflow env, never re-authored here.
BOOT="${BOOTSTRAP_SIZE_BUDGET:-3145728}"
BSIZE=$(stat -c %s "${REL}/tebako-bootstrap.exe")
echo "tebako-bootstrap (windows-arm64): ${BSIZE} bytes (budget ${BOOT})"
if [ "$BSIZE" -ge "$BOOT" ]; then
  echo "::error::tebako-bootstrap (windows-arm64) is ${BSIZE} bytes — over the ${BOOT} budget"
  exit 1
fi
# The wrapper's 10 MiB budget gates SHIP forms only (spec 29 §5);
# windows-arm64 is not one yet — measured and reported.
LSIZE=$(stat -c %s "${REL}/tebako-runtime-launcher.exe")
echo "tebako-runtime-launcher (windows-arm64): ${LSIZE} bytes (ship-form budget 10485760, not gated on this leg)"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "| windows-arm64 | binary | size (bytes) |"
    echo "|---|---|---|"
    echo "| aarch64-pc-windows-msvc | tebako-bootstrap | ${BSIZE} |"
    echo "| aarch64-pc-windows-msvc | tebako-runtime-launcher | ${LSIZE} |"
  } >> "$GITHUB_STEP_SUMMARY"
fi

# --- 3. PE machine gate ------------------------------------------------------
# No arm64 windows emulation exists on the x64 runner, so the smoke is a
# header assert instead of a --version run: every produced image must be
# a PE with Machine == 0xAA64 (IMAGE_FILE_MACHINE_ARM64). The cdylib
# rides along — it is the same libdwarfs_c closure the driver loads.
python3 - "$REL" <<'EOF'
import glob, struct, sys

rel = sys.argv[1]
images = sorted(glob.glob(rel + "/tebako-bootstrap.exe")
                + glob.glob(rel + "/tebako-runtime-launcher.exe")
                + glob.glob(rel + "/tfs.dll"))
if not images:
    print("::error::windows-arm64: no PE images found under " + rel)
    sys.exit(1)

failed = False
for path in images:
    with open(path, "rb") as f:
        data = f.read()
    machine = None
    if data[:2] == b"MZ" and len(data) >= 0x40:
        e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
        if data[e_lfanew:e_lfanew + 4] == b"PE\x00\x00":
            machine = struct.unpack_from("<H", data, e_lfanew + 4)[0]
    ok = machine == 0xAA64
    if machine is None:
        detail = "not a PE image"
    else:
        detail = f"machine=0x{machine:04X}" + (" ARM64" if ok else " (expected 0xAA64 ARM64)")
    print(f"{path}: {detail}" if ok else f"{path}: FAIL — {detail}")
    failed = failed or not ok
sys.exit(1 if failed else 0)
EOF

# --- 4. the named blocker (this leg's expected red) -------------------------
# The remaining tools — tebako (tebako-cli), tfs (tfs-cli), tebako-pkg,
# tebako-shim — all link tebako-signer → rnp-rs 0.1.15 (vendored) →
# rnp-sys 0.1.2 → rnp-src, and rnp-src 0.3.0 hardcodes Botan's toolchain
# to gcc on EVERY windows host (src/lib.rs, the
# `if cfg!(target_os = "windows")` block: BOTAN_CONFIGURE_CC=gcc,
# BOTAN_CONFIGURE_CC_BIN=g++) — set before botan-src reads the env, so no
# override can win. A VS x64_arm64 environment has no gcc, and no
# mingw-w64 gcc emits ARM64 COFF an MSVC link accepts, so botan's
# configure dies before compiling a single object. Until rnp-src grows an
# MSVC path (rnpgp upstream), phase 1 proves the signer-free set and
# these four tools stay a NAMED red — an honest red beats a fake green.
echo "::error::windows-arm64: tebako / tfs (tfs-cli) / tebako-pkg / tebako-shim cannot build on aarch64-pc-windows-msvc — blocker: rnp-src 0.3.0 hardcodes BOTAN_CONFIGURE_CC=gcc on every windows host (no MSVC path; a VS x64_arm64 env has no gcc, and mingw gcc cannot emit ARM64 COFF for an MSVC link). Findings recorded in roadmap 02 Gap 1; phase 1 ships nothing."
exit 1
