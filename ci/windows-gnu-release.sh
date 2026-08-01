#!/usr/bin/env bash
# ci/windows-gnu-release.sh — the release build of the five product
# binaries on windows-gnu (ucrt64), end to end: the release-profile
# build (tfs vendored-dwarfs via dwarfs-t-sys's CMake/vcpkg build
# against the x64-mingw-static baseline, tebako-signer's vendored
# rnp/Botan), DLL-import forensics on the staged exes, then stage.sh
# (strip, the bootstrap size gate, the sha/size fragments).
#
# Everything the leg needs is HERE, not inline in the workflow YAML —
# run-blocks get string-edited and break silently; a script is reviewed
# and shellcheck-able. The workflow only exports the environment
# (DWARFS_RS_VCPKG_ROOT / VCPKG_ROOT for the vendored dwarfs build;
# VERSION / PLATFORM for the stage) and calls this file.
set -euo pipefail

# The proven closed PATH (ci/windows-gnu-bootstrap.sh): ucrt64 gcc +
# cmake + ninja first; Git's /usr/bin for coreutils (safe — the ABI clash
# is specifically setup-msys2's /usr/bin, which stays OFF); git.exe from
# Git's /cmd; cargo; System32. No choco mingw, no stray runner toolchains
# (first-DLL-wins resolution is STATUS_ENTRYPOINT_NOT_FOUND at process
# start).
export PATH="/d/a/_temp/msys64/ucrt64/bin:/c/Program Files/Git/usr/bin:/c/Program Files/Git/cmd:/c/Users/runneradmin/.cargo/bin:/c/Windows/System32"

# tebako-signer's vendored rnp builds Botan via botan-src, which spawns
# plain `make` (hardcoded upstream); MSYS2's ucrt64 ships only
# mingw32-make.exe. Give it a `make` on the closed PATH — a COPY (Git
# bash "symlinks" are text files to CreateProcess). Fails loudly here if
# the toolchain ever drops mingw32-make, instead of upstream's cryptic
# "program not found".
TOOLSHIM=/d/a/_temp/tebako-toolshim
mkdir -p "$TOOLSHIM"
cp "/d/a/_temp/msys64/ucrt64/bin/mingw32-make.exe" "$TOOLSHIM/make.exe"
export PATH="$TOOLSHIM:$PATH"

# One linker, resolved from the closed PATH above (ucrt64's gcc).
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=gcc.exe

# Static-link the mingw C/C++ runtimes; the trailing -lmsvcrt restores
# the msvcrt-after-mingwex order invariant (rustc's -nodefaultlibs tail
# violates it; the factory probe proved libmingwex's compat _assert.o
# then dies on __imp___msvcrt_assert).
export RUSTFLAGS="-C link-arg=-static-libgcc -C link-arg=-static-libstdc++ -C link-arg=-lmsvcrt"

# botan-src's configure.py defaults to MSVC on os=windows and there is no
# cl on the closed PATH — steer it to the ucrt64 gcc (probe-proven: with
# the default the configure dies on "Default compiler is msvc but could
# not find 'cl'").
export BOTAN_CONFIGURE_CC=gcc

# bindgen (rnp-rs's rnp bindings) drives the runner image's libclang in
# MSVC mode: with no mingw header dirs on its search path, rnp.h dies on
# <stdbool.h> (tebako-rs CI run 30714614829). Point clang at the ucrt64
# headers — the C library's and gcc's own (stdbool.h lives there) — and
# name the target explicitly. The paths must be WINDOWS-FORM (D:/...):
# libclang is a native Windows binary — the msys form (/d/a/...) does not
# resolve for it (openjdk feedstock run 30719756048 proved the msys form
# a no-op). Fail loudly if the toolchain layout moves.
UCRT64=/d/a/_temp/msys64/ucrt64
GCC_INCLUDE=$(echo "$UCRT64"/lib/gcc/x86_64-w64-mingw32/*/include)
if [ ! -d "$GCC_INCLUDE" ]; then
  echo "ucrt64 gcc include dir not found under $UCRT64/lib/gcc — toolchain layout changed"
  exit 1
fi
export BINDGEN_EXTRA_CLANG_ARGS="--target=x86_64-w64-mingw32 -isystem $(cygpath -m "$UCRT64/include") -isystem $(cygpath -m "$GCC_INCLUDE")"

TARGET=x86_64-pc-windows-gnu

# --- 1. release build -------------------------------------------------------
cargo build --release --target "$TARGET" \
  -p tebako-bootstrap -p tfs-cli -p tebako-pkg -p tebako-cli -p tebako-shim

# --- 2. stage (strip, size gate, fragments) ---------------------------------
# stage.sh strips, enforces the bootstrap size budget and writes the
# frag-<platform> sha/size fragments finalize merges into SHA256SUMS +
# manifest.json. The windows binaries carry .exe.
TARGET="$TARGET" EXE_SUFFIX=.exe bash .github/workflows/lib/stage.sh

# --- 3. DLL-import forensics ------------------------------------------------
# STATUS_ENTRYPOINT_NOT_FOUND fails the process BEFORE main, so a bad
# import never names itself — enumerate every import of the SHIPPED exes
# up front instead. Unprefixed binutils: MSYS2's ucrt64 ships objdump.exe
# without the x86_64-w64-mingw32- alias; the closed PATH makes the one
# toolchain's tools unambiguous. Informational (the static-libgcc /
# static-libstdc++ RUSTFLAGS above are the invariant this audits).
for exe in out/*.exe; do
  echo "=== imports: $exe ==="
  objdump -p "$exe" \
    | grep -E "DLL Name" | sort -u || true
done
