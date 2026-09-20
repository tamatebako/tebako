#!/usr/bin/env bash
# ci/windows-arm64-gnullvm-ship.sh — the windows-arm64 release leg
# (roadmap 02 Gap 1 phase 2), end to end: the release-profile build of
# tfs.exe + tebako-pkg.exe + the tebako-driver/tfs staticlibs for
# aarch64-pc-windows-gnullvm on the native windows-11-arm runner under
# msys2 clangarm64, manual staging with the sha/size fragments finalize
# merges into SHA256SUMS + manifest.json, then the ship gate on the
# STAGED bytes.
#
# Scope, by decision (phase 2):
#   * tfs + tebako-pkg of the six tools — the runtime factories consume
#     tfs + the link unit, and tebako-pkg makes windows-arm64 a
#     first-class AUTHORING platform (trailer surgery, sign/verify,
#     release-index). The other four tools are phase 3.
#   * limnifs-only — the dwarfs arm64 closure is dwarfs-t #100's
#     milestone; the link unit carries NO mingw closure at all (the
#     stage_link_unit --limnifs-only branch). A dwarfs read/write on
#     arm64 answers the compiled-out backend's named error.
#   * unsigned Authenticode — unsigned stays first-class (spec 00
#     invariant 7); Azure-signing the arm64 PEs lands with phase 3.
#
# Everything the leg needs is HERE, not inline in the workflow YAML (the
# ci/windows-gnu-release.sh argument): run-blocks get string-edited and
# break silently; a script is reviewed and shellcheck-able. The workflow
# only exports VERSION and calls this file.
set -euo pipefail

# The windows-11-arm image is single-disk: the workspace is C:\a, there
# is no D: drive (the x64 leg's /d/a/... idiom died at this script's
# first mkdir — run 35500262780). Anchor on RUNNER_TEMP, unix-form for
# the bash-native uses below (cygpath is Git-bash's own /usr/bin,
# present before the closed-PATH export).
RT_UNIX=$(cygpath -u "$RUNNER_TEMP")
CLANGARM64="$RT_UNIX/msys64/clangarm64"
TARGET=aarch64-pc-windows-gnullvm
PLATFORM=windows-ucrt-arm64
[ -n "${VERSION:-}" ] || { echo "VERSION is required (the release tag minus v)"; exit 64; }

# HOST build-deps compile AND LINK for aarch64-pc-windows-msvc (the
# rustup host triple): rustc drives link.exe for them, and Git-bash's
# coreutils `link` must never win (run 35501759953: getrandom's build
# script died "linking with link.exe" — /usr/bin/link took it). The
# job's msvc-dev-cmd step (arch: arm64) armed the HostARM64/ARM64
# tools; pull that bin dir into the closed PATH AHEAD of Git's
# /usr/bin. INCLUDE/LIB ride the environment, untouched by the PATH
# replacement. (The phase-1 idiom, ci/windows-arm64-msvc-build.sh.)
[ -n "${VCToolsInstallDir:-}" ] || { echo "::error::windows-arm64-ship: VCToolsInstallDir unset — the msvc-dev-cmd step (arch: arm64) must run before this script"; exit 1; }
VSTOOLS="${VCToolsInstallDir%/}"
VSTOOLS="${VSTOOLS%\\}"
VSBIN="$(cygpath -u "$VSTOOLS")/bin/HostARM64/ARM64"
[ -d "$VSBIN" ] || { echo "::error::windows-arm64-ship: VS ARM64 native tools not found at $VSBIN (VCToolsInstallDir=$VCToolsInstallDir) — toolchain layout changed"; exit 1; }

# The proven closed PATH (ci/windows-gnu-release.sh): the VS tools
# first (link.exe), then clangarm64's clang/lld; Git's /usr/bin for
# coreutils (safe — the ABI clash is specifically setup-msys2's
# /usr/bin, which stays OFF); git.exe from Git's /cmd; cargo; System32.
# No choco mingw, no stray runner toolchains (first-DLL-wins resolution
# is STATUS_ENTRYPOINT_NOT_FOUND at process start).
export PATH="$VSBIN:$CLANGARM64/bin:/c/Program Files/Git/usr/bin:/c/Program Files/Git/cmd:/c/Users/runneradmin/.cargo/bin:/c/Windows/System32"

# The MSVC env must genuinely target ARM64 (the host machine): a
# misconfigured arch compiles the build-deps into the wrong machine and
# dies cryptically when the first build script runs. Named failure
# here, never a wrong-arch object.
CL_BANNER="$(cl 2>&1 | head -1 || true)"
echo "$CL_BANNER"
case "$CL_BANNER" in
  *"for ARM64"*) ;;
  *) echo "::error::windows-arm64-ship: cl does not target ARM64 (\"$CL_BANNER\") — the msvc-dev-cmd arch must be arm64"; exit 1 ;;
esac

# Plain-name shims for the tools upstream hardcodes (botan-src spawns
# bare `make`) or the gates call (objdump, strip). clangarm64 is pure
# LLVM: no binutils, so objdump/strip come from llvm-objdump/llvm-strip
# when the unprefixed names are absent. Always resolved FROM the
# clangarm64 bin dir (never PATH roulette), always COPIES — Git-bash
# "symlinks" are text files to CreateProcess. Fails loudly here if the
# toolchain layout moves, instead of upstream's cryptic "program not
# found".
TOOLSHIM="$RT_UNIX/tebako-toolshim-arm64"
mkdir -p "$TOOLSHIM"
shim() {
  plain=$1; shift
  for cand in "$@"; do
    if [ -x "$CLANGARM64/bin/$cand" ]; then
      cp "$CLANGARM64/bin/$cand" "$TOOLSHIM/$plain.exe"
      return 0
    fi
  done
  echo "no $plain under $CLANGARM64/bin (tried: $*) — toolchain layout changed" >&2
  exit 1
}
shim make mingw32-make.exe
shim objdump objdump.exe aarch64-w64-mingw32-objdump.exe llvm-objdump.exe
shim strip strip.exe aarch64-w64-mingw32-strip.exe llvm-strip.exe
export PATH="$TOOLSHIM:$PATH"

# One linker, target-scoped — NOT a global RUSTFLAGS/LINKER: this leg's
# rust host triple is aarch64-pc-windows-msvc (the runner image's rust),
# and stage_link_unit's `cargo run -p tebako-arscope` is a HOST build; a
# global -static-libstdc++ would reach link.exe and choke. rustc
# auto-finds the MSVC link.exe for the host build.
#
# The linker is the release-link WRAPPER (ci/windows-gnu-link-wrap.c,
# compiled by clangarm64's clang): rnp-sys emits an explicit
# `dylib=stdc++` for every windows && !msvc target — rustc places that
# -lstdc++ BEFORE the trailing -C link-args, so no driver flag can
# govern it (the 0.1.1 lesson, the x64 leg's comment), and clangarm64
# carries no libstdc++ AT ALL (pure LLVM). The wrapper rewrites the
# reference to -l:libc++.a — clangarm64's one C++ runtime, statically;
# the import gate (ship-gate.sh below) proves no libc++.dll on the
# staged exe. TEBAKO_LINK_WRAP_EXEC names the prefixed clang (the triple
# prefix of argv[0] drives clang's --target).
WRAP="$RUNNER_TEMP/tebako-link-wrap-arm64.exe"
clang -O2 -o "$WRAP" ci/windows-gnu-link-wrap.c
WRAP_WIN=$(cygpath -w "$WRAP")
export CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER="$WRAP_WIN"
export TEBAKO_LINK_WRAP_EXEC=aarch64-w64-mingw32-clang
export TEBAKO_LINK_WRAP_STDCXX_A=libc++.a
# Driver-level belt for clang's OWN expansions (compiler-rt builtins,
# the -stdlib expansion on any future -lc++ path); the early-position
# build-script emissions are the wrapper's job above.
export CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_RUSTFLAGS="-C link-arg=-static-libstdc++ -C link-arg=-static-libgcc"

# botan-src's configure.py defaults to MSVC on os=windows and there is no
# cl on the closed PATH — steer it to clangarm64's clang (the ucrt64
# leg's gcc probe: "Default compiler is msvc but could not find 'cl'").
# BOTAN_CONFIGURE_CC is botan's compiler-FAMILY name; the actual binary is
# --cc-bin, which rnp-src defaults to g++ on windows (rnp-src
# src/config.rs:80) — clangarm64 has no g++, and make died
# CreateProcess(NULL, g++ ...) e=2 (run 35502239547). rnp-src honors the
# caller override (config.rs:87-89): use the prefixed clang++ driver.
export BOTAN_CONFIGURE_CC=clang
export BOTAN_CONFIGURE_CC_BIN=aarch64-w64-mingw32-clang++

# botan 3.13's os_utils.cpp:686 gates its windows thread-naming call on
#   defined(BOTAN_TARGET_OS_HAS_WIN32) && defined(_LIBCPP_HAS_THREAD_API_PTHREAD)
# — but libc++ >= 19 defines _LIBCPP_HAS_THREAD_API_PTHREAD as 0/1
# (#cmakedefine01, llvm libcxx __config_site.in:20), so defined() is true
# EITHER WAY. MSYS2's libc++ is built LIBCXX_HAS_WIN32_THREAD_API=ON
# (mingw-w64-libc++ PKGBUILD:97 — std::thread native_handle is void*),
# the pthread branch goes live, and the call dies: "cannot convert
# 'native_handle_type' (aka 'void *') to 'pthread_t'" (run 35503816333
# job 106060195357; ucrt64's libstdc++ never defines the macro, which is
# why only arm64 hits it). Upstream master still carries the bug.
# Patch the extracted source through botan-src's BOTAN_SRC_DIR escape
# hatch (botan-src src/lib.rs:186 — no sha256 gate on custom dirs): the
# value test kills the branch and set_thread_name falls through to the
# final no-op — exactly the ucrt64 leg's shipping behavior (thread naming
# is cosmetic). Extracts the crate's OWN vendored tarball, so the source
# stays version-locked to the cargo resolution; the grep gate drops the
# patch automatically the day a fixed botan-src ships.
cargo fetch --quiet
CARGO_HOME_MSYS=$(cygpath "${CARGO_HOME:-$USERPROFILE/.cargo}")
# registry/src/<index>/botan-src-<ver>/vendor/Botan-<ver>.tar.xz — the
# glob keeps the LAST (newest) match if a warm cache carries several.
BOTAN_TARBALL=
for f in "$CARGO_HOME_MSYS"/registry/src/*/botan-src-*/vendor/Botan-*.tar.xz; do BOTAN_TARBALL=$f; done
[ -n "$BOTAN_TARBALL" ] && [ -f "$BOTAN_TARBALL" ] || { echo "vendored botan tarball not found under $CARGO_HOME_MSYS"; exit 1; }
BOTAN_PATCHED="$RUNNER_TEMP/botan-src-patched"
rm -rf "$BOTAN_PATCHED" && mkdir -p "$BOTAN_PATCHED"
tar -xJf "$BOTAN_TARBALL" -C "$BOTAN_PATCHED"
BOTAN_ROOT=$(find "$BOTAN_PATCHED" -mindepth 1 -maxdepth 1 -type d | head -1)
OS_UTILS="$BOTAN_ROOT/src/lib/utils/os_utils/os_utils.cpp"
if grep -q 'defined(_LIBCPP_HAS_THREAD_API_PTHREAD)' "$OS_UTILS"; then
  sed -i 's/defined(_LIBCPP_HAS_THREAD_API_PTHREAD)/_LIBCPP_HAS_THREAD_API_PTHREAD/' "$OS_UTILS"
  ! grep -q 'defined(_LIBCPP_HAS_THREAD_API_PTHREAD)' "$OS_UTILS" || { echo "botan patch failed to apply"; exit 1; }
  BOTAN_SRC_DIR=$(cygpath -m "$BOTAN_ROOT")
  export BOTAN_SRC_DIR
  echo "patched botan (libc++ thread-api value test): $BOTAN_SRC_DIR"
else
  echo "NOTE: botan source lacks the defined() bug — building the vendored source unpatched"
fi

# bindgen (rnp-sys's rnp bindings) drives libclang: with no mingw header
# dirs on its search path, rnp.h dies on <stdbool.h> (tebako-rs CI run
# 30714614829). Point clang at the clangarm64 headers and name the
# target explicitly. The paths must be WINDOWS-FORM (C:/... on this
# runner): libclang is a native Windows binary — the msys form
# (/c/a/...) does not resolve
# for it (openjdk feedstock run 30719756048 proved the msys form a
# no-op). libclang.dll itself is found via the closed PATH
# (clangarm64/bin carries it).
CLANGARM64_INCLUDE=$(cygpath -m "$CLANGARM64/include")
export BINDGEN_EXTRA_CLANG_ARGS="--target=aarch64-w64-mingw32 -isystem $CLANGARM64_INCLUDE"

# cc-crate steering, both triples — cc otherwise grabs the bare clang
# off the closed PATH for every C compile: for the HOST triple
# (aarch64-pc-windows-msvc — ring & co. as build-deps) that emits
# --target=msvc with no MSVC headers on the search path (run
# 35501035272: ring's check.h found no assert.h); for the TARGET triple
# the bare driver spells it gnullvm, whose per-triple sysroot dirs
# clangarm64 does not carry. The job's msvc-dev-cmd step arms
# HostARM64/arm64's cl for host compiles; the prefixed clang driver
# pins the msys2 triple spelling for the target (the same spelling
# TEBAKO_LINK_WRAP_EXEC links with).
export CC_aarch64_pc_windows_msvc=cl
export CXX_aarch64_pc_windows_msvc=cl
export CC_aarch64_pc_windows_gnullvm=aarch64-w64-mingw32-clang
export CXX_aarch64_pc_windows_gnullvm=aarch64-w64-mingw32-clang++

# --- 1. release build -------------------------------------------------------
# The feature flags on `-p tfs` are MANDATORY: cargo unifies features
# within one invocation, and a bare `-p tfs` would pull tfs's default
# features (the dwarfs backend → dwarfs-t-sys's CMake/vcpkg build) into
# the graph — the arm64 closure that does not exist. The explicit
# selection matches the crates' windows-aarch64 target cfgs
# (backend-limnifs only), so the unified tfs is limnifs-only.
cargo build --release --target "$TARGET" \
  -p tfs --no-default-features --features backend-limnifs \
  -p tebako-driver -p tfs-cli -p tebako-pkg

# --- 2. stage (strip, fragments) --------------------------------------------
# stage.sh demands all six tools — unusable for this leg. Same steps by
# hand: strip, the frag-<platform> sha/size fragments finalize merges
# into SHA256SUMS + manifest.json, the size-table rows.
SUMMARY="${GITHUB_STEP_SUMMARY:-/dev/null}"
[ -d "$(dirname "$SUMMARY")" ] || SUMMARY=/dev/null

# tebako-pkg rides the same graph (tebako-signer/tebako-info/tebako-
# resolve are already arm64-clean for tfs-cli) — windows-arm64 is a
# first-class authoring platform, never x64-under-emulation.
TOOLS="tfs tebako-pkg"
mkdir -p out "fragments/frag-$PLATFORM"
{
  echo "| platform | binary | size (bytes) |"
  echo "|---|---|---|"
} >> "$SUMMARY"
for tool in $TOOLS; do
  src="target/$TARGET/release/${tool}.exe"
  test -x "$src" || { echo "missing build output: $src"; exit 1; }
  dest="out/${tool}-${VERSION}-${PLATFORM}.exe"
  cp "$src" "$dest"
  strip "$dest" 2>/dev/null || true
  size=$(stat -c %s "$dest")
  sha256=$(sha256sum "$dest" | cut -d' ' -f1)
  echo "$sha256" > "fragments/frag-$PLATFORM/${tool}-${PLATFORM}.sha256"
  echo "$size" > "fragments/frag-$PLATFORM/${tool}-${PLATFORM}.size"
  echo "| $PLATFORM | $tool | $size |" >> "$SUMMARY"
done

# --- 3. the link unit (the factories' staticlib input) ----------------------
# --limnifs-only: the arm64 graph carries no native closure by target
# cfg — closure/ stays empty BY DESIGN (the dwarfs/sqfs/rnp harvest and
# the boost required-set check skip).
mkdir -p out/link-unit-aarch64-windows-gnu
ruby tools/stage_link_unit out/link-unit-aarch64-windows-gnu \
  --target "$TARGET" --skip-build --limnifs-only
tar -czf "out/link-unit-${VERSION}-aarch64-windows-gnu.tar.gz" -C out link-unit-aarch64-windows-gnu

# --- 4. ship gate -----------------------------------------------------------
# The same gate every leg runs (ship-gate.sh): the bare-launch smoke with
# PATH scrubbed to System32+Windows — what a user machine actually looks
# like — then the PE import audit (inbox DLLs only; libc++.dll would be
# the arm64 failure class). This runner IS the target arch: the smoke is
# a real launch, no cross-compile proxy.
bash .github/workflows/lib/ship-gate.sh "$PLATFORM" out/*.exe

echo "staged for $PLATFORM:"
ls -la out/
