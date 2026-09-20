#!/usr/bin/env bash
# ci/windows-arm64-gnullvm-ship.sh — the windows-arm64 release leg
# (roadmap 02 Gap 1 phase 2), end to end: the release-profile build of
# tfs.exe + the tebako-driver/tfs staticlibs for aarch64-pc-windows-
# gnullvm on the native windows-11-arm runner under msys2 clangarm64,
# manual staging with the sha/size fragments finalize merges into
# SHA256SUMS + manifest.json, then the ship gate on the STAGED bytes.
#
# Scope, by decision (phase 2):
#   * tfs.exe only of the six tools — the runtime factories consume
#     exactly tfs + the link unit (a grep of the factory workflows shows
#     zero tebako-pkg usage); the full six-tool arm64 set is phase 3.
#   * limnifs-only — the dwarfs arm64 closure is dwarfs-t #100's
#     milestone; the link unit carries NO mingw closure at all (the
#     stage_link_unit --limnifs-only branch).
#   * unsigned Authenticode — unsigned stays first-class (spec 00
#     invariant 7); Azure-signing the arm64 PEs lands with phase 3.
#
# Everything the leg needs is HERE, not inline in the workflow YAML (the
# ci/windows-gnu-release.sh argument): run-blocks get string-edited and
# break silently; a script is reviewed and shellcheck-able. The workflow
# only exports VERSION and calls this file.
set -euo pipefail

CLANGARM64=/d/a/_temp/msys64/clangarm64
TARGET=aarch64-pc-windows-gnullvm
PLATFORM=windows-ucrt-arm64
[ -n "${VERSION:-}" ] || { echo "VERSION is required (the release tag minus v)"; exit 64; }

# The proven closed PATH (ci/windows-gnu-release.sh): clangarm64's
# clang/lld first; Git's /usr/bin for coreutils (safe — the ABI clash is
# specifically setup-msys2's /usr/bin, which stays OFF); git.exe from
# Git's /cmd; cargo; System32. No choco mingw, no stray runner toolchains
# (first-DLL-wins resolution is STATUS_ENTRYPOINT_NOT_FOUND at process
# start).
export PATH="$CLANGARM64/bin:/c/Program Files/Git/usr/bin:/c/Program Files/Git/cmd:/c/Users/runneradmin/.cargo/bin:/c/Windows/System32"

# Plain-name shims for the tools upstream hardcodes (botan-src spawns
# bare `make`) or the gates call (objdump, strip). clangarm64 is pure
# LLVM: no binutils, so objdump/strip come from llvm-objdump/llvm-strip
# when the unprefixed names are absent. Always resolved FROM the
# clangarm64 bin dir (never PATH roulette), always COPIES — Git-bash
# "symlinks" are text files to CreateProcess. Fails loudly here if the
# toolchain layout moves, instead of upstream's cryptic "program not
# found".
TOOLSHIM=/d/a/_temp/tebako-toolshim-arm64
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
export BOTAN_CONFIGURE_CC=clang

# bindgen (rnp-sys's rnp bindings) drives libclang: with no mingw header
# dirs on its search path, rnp.h dies on <stdbool.h> (tebako-rs CI run
# 30714614829). Point clang at the clangarm64 headers and name the
# target explicitly. The paths must be WINDOWS-FORM (D:/...): libclang
# is a native Windows binary — the msys form (/d/a/...) does not resolve
# for it (openjdk feedstock run 30719756048 proved the msys form a
# no-op). libclang.dll itself is found via the closed PATH
# (clangarm64/bin carries it).
CLANGARM64_INCLUDE=$(cygpath -m "$CLANGARM64/include")
export BINDGEN_EXTRA_CLANG_ARGS="--target=aarch64-w64-mingw32 -isystem $CLANGARM64_INCLUDE"

# --- 1. release build -------------------------------------------------------
# The feature flags on `-p tfs` are MANDATORY: cargo unifies features
# within one invocation, and a bare `-p tfs` would pull tfs's default
# features (the dwarfs backend → dwarfs-t-sys's CMake/vcpkg build) into
# the graph — the arm64 closure that does not exist. The explicit
# selection matches the crates' windows-aarch64 target cfgs
# (backend-limnifs only), so the unified tfs is limnifs-only.
cargo build --release --target "$TARGET" \
  -p tfs --no-default-features --features backend-limnifs \
  -p tebako-driver -p tfs-cli

# --- 2. stage (strip, fragments) --------------------------------------------
# stage.sh demands all six tools — unusable for this leg. Same steps by
# hand: strip, the frag-<platform> sha/size fragments finalize merges
# into SHA256SUMS + manifest.json, the size-table row.
SUMMARY="${GITHUB_STEP_SUMMARY:-/dev/null}"
[ -d "$(dirname "$SUMMARY")" ] || SUMMARY=/dev/null

mkdir -p out "fragments/frag-$PLATFORM"
src="target/$TARGET/release/tfs.exe"
test -x "$src" || { echo "missing build output: $src"; exit 1; }
dest="out/tfs-${VERSION}-${PLATFORM}.exe"
cp "$src" "$dest"
strip "$dest" 2>/dev/null || true
size=$(stat -c %s "$dest")
sha256=$(sha256sum "$dest" | cut -d' ' -f1)
echo "$sha256" > "fragments/frag-$PLATFORM/tfs-${PLATFORM}.sha256"
echo "$size" > "fragments/frag-$PLATFORM/tfs-${PLATFORM}.size"
{
  echo "| platform | binary | size (bytes) |"
  echo "|---|---|---|"
  echo "| $PLATFORM | tfs | $size |"
} >> "$SUMMARY"

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
bash .github/workflows/lib/ship-gate.sh "$PLATFORM" "$dest"

echo "staged for $PLATFORM:"
ls -la out/
