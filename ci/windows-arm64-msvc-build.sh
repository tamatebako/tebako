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
#   step is deliberately absent: no arm64-windows-static baseline is
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

# The bash shell wrapper prepends Git's /usr/bin to PATH, and coreutils'
# `link` shadows MSVC's link.exe (rustc dies on `/usr/bin/link: extra
# operand` — first run 35422912784). Re-prepend the VS tool bin dir —
# the x64-hosted ARM64-targeting tools (HostX64/arm64 under the
# x64_arm64 vcvars env) — ahead of it; every cargo/cmake/vcpkg child
# inherits the fixed order.
# one trailing separator — vcvars emits a trailing backslash; strip a
# forward slash first, then a backslash (both forms are tolerated).
VSTOOLS="${VCToolsInstallDir%/}"
VSTOOLS="${VSTOOLS%\\}"
VSBIN="$(cygpath -u "$VSTOOLS")/bin/HostX64/arm64"
if [ ! -d "$VSBIN" ]; then
  echo "::error::windows-arm64: VS ARM64 cross tools not found at $VSBIN (VCToolsInstallDir=$VCToolsInstallDir) — toolchain layout changed"
  exit 1
fi
export PATH="$VSBIN:$PATH"

# The environment must genuinely target ARM64: the msvc-dev-cmd arch
# input is easy to lose, and a plain x64 env compiles the whole leg into
# the wrong machine (named failure here, never a confusing link error).
CL_BANNER="$(cl 2>&1 | head -1 || true)"
echo "$CL_BANNER"
case "$CL_BANNER" in
  *"for ARM64"*) ;;
  *) echo "::error::windows-arm64: cl does not target ARM64 (\"$CL_BANNER\") — the msvc-dev-cmd arch must be x64_arm64"
     exit 1 ;;
esac

# --- 1. the signer-free build ----------------------------------------------
# The CRT story is FULLY STATIC (/MT): dwarfs-t's CMake pins
# CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded for MSVC unconditionally, and
# the org's arm64-windows-static overlay triplet (/MT — vcpkg_triplets/)
# matches it, so the Rust side opts into the static CRT too and every
# object in the final link agrees. (The -md triplet's /MD ports mismatch
# dwarfs_c's /MT at the tfs.dll link — LNK2038, run 35425206392.)
# dwarfs-t-sys's CRT probe demands the explicit triplet spelling for the
# /MT pairing — hence DWARFS_RS_VCPKG_TRIPLET here, not the default.
export RUSTFLAGS="-C target-feature=+crt-static"
export DWARFS_RS_VCPKG_TRIPLET=arm64-windows-static
# SCRUB FIRST, before any build: neither rnp-src (BOTAN_* env, no
# rerun-if-env-changed) nor dwarfs_t_sys (vendored content not in the
# fingerprint) keys its cargo unit on the inputs that changed across the
# fix iterations — a restored cache could otherwise leak pre-fix botan
# and boost objects into ANY link, including this script's first one.
rm -rf "target/$TARGET/release/build/rnp-src-"* \
       "target/$TARGET/release/build/dwarfs_t_sys-"* \
       "target/$TARGET/release/build/dwarfs-t-sys-"* || true
rm -f "target/$TARGET/release/deps/libdwarfs_t_sys-"*.rlib \
      "target/$TARGET/release/deps/libdwarfs_t_sys-"*.d \
      "target/$TARGET/release/deps/tfs.dll" \
      "target/$TARGET/release/deps/tfs.dll.lib" || true
# Botan's configure takes the toolchain from these (rnp-src's caller-
# respect guard — rnpgp/rnp-rs#103, pinned via [patch.crates-io] —
# leaves a caller-provided value in place; the x64_arm64 env's cl is
# what botan must use, and no gcc exists here).
export BOTAN_CONFIGURE_CC=cl
export BOTAN_CONFIGURE_CC_BIN=cl

# tebako-bootstrap keeps its OWN invocation: cargo feature unification
# with the other tools would re-enable tebako-resolve's `git` stack
# inside the size-gated loader (the v0.3.0 run 32975547796 lesson).
cargo build --release --target "$TARGET" -p tebako-bootstrap

# tfs on windows is dwarfs-only by target split (squashfs is POSIX-only;
# enc needs rnp — see the blocker below): the same feature set every
# windows consumer passes (crates/*/Cargo.toml, cfg(windows) tfs deps).
# Building tfs as a package also links the cdylib — the full libdwarfs_c
# + vcpkg arm64-windows-static closure compiles AND links under MSVC
# ARM64, which is the leg's core prove-out. tebako-driver (the runtime
# exe's mount engine) and the spec-29 wrapper exe ride along.
# Phase 1 (roadmap 02, rescoped): tfs builds LIMNIFS-ONLY here — the
# house format is pure Rust and is what every shipped artifact uses;
# the dwarfs backend is an optional compatibility surface and its
# arm64-static link closure (boost_program_options + crypt32 on the
# cdylib link line) is the dwarfs-t project's own milestone (dwarfs-t
# #100 / dwarfs-t-rs), tracked there. An arm64 dwarfs mount answers
# ENOTSUP exactly as any feature-disabled backend does.
cargo build --release --target "$TARGET" \
  -p tfs --no-default-features --features backend-limnifs \
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
                + glob.glob(rel + "/tfs.dll")
                + glob.glob(rel + "/tebako.exe")
                + glob.glob(rel + "/tebako-pkg.exe")
                + glob.glob(rel + "/tebako-shim.exe"))
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

# --- 4. the signer-linked tools (the rnp-src fix lands) ---------------------
# tebako (tebako-cli), tebako-pkg, tebako-shim link tebako-signer →
# rnp-rs → rnp-src, whose 0.3.0 release stomped a caller-provided
# BOTAN_CONFIGURE_CC on every windows host. The fork branch pinned via
# [patch.crates-io] (rnpgp/rnp-rs#103 — the caller-respect guard) makes
# the MSVC x64_arm64 build of Botan possible; these three complete the
# five-tool set. Phase 1 still ships nothing (roadmap 02 Gap 1).
cargo build --release --target "$TARGET" \
  -p tebako-cli -p tebako-pkg -p tebako-shim
