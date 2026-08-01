#!/usr/bin/env bash
# ci/windows-gnu-cli.sh — the full-stack product crates on windows-gnu
# (ucrt64), end to end: tebako-cli, tfs-cli and tebako-pkg — the full
# dependency-stack build (tfs vendored-dwarfs via dwarfs-t-sys's
# CMake/vcpkg build against the x64-mingw-static baseline, tebako-signer's
# vendored rnp/Botan), DLL-import forensics, then the serialized test run.
#
# Everything the leg needs is HERE, not inline in the workflow YAML —
# run-blocks get string-edited and break silently; a script is reviewed
# and shellcheck-able. The workflow only exports the environment
# (DWARFS_RS_VCPKG_ROOT / VCPKG_ROOT for the vendored dwarfs build) and
# calls this file.
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

# tebako-signer's vendored rnp builds Botan via botan-src: configure.py
# defaults to MSVC on os=windows and there is no cl on the closed PATH —
# steer it to the ucrt64 gcc (probe-proven: with the default the
# configure dies on "Default compiler is msvc but could not find 'cl'").
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

# E2E presses stay off on this leg: they download a runtime and press
# side-by-side with the reference gem (a host ruby + thor) — the windows
# dogfood e2e is TODO.prepublish/06's story. Every other test runs.
export TEBAKO_CLI_SKIP_E2E=1

TARGET=x86_64-pc-windows-gnu

# The serialize note: a parallel harness hides which test crashes
# (windows-gnu legs died twice to a hidden segfault before this rule).
SERIAL="--test-threads=1"

# --- 1. build + test-compile ------------------------------------------------
cargo build -p tebako-cli -p tfs-cli -p tebako-pkg --target "$TARGET"
cargo test -p tebako-cli -p tfs-cli -p tebako-pkg --target "$TARGET" --no-run

# --- 2. DLL-import forensics ------------------------------------------------
# STATUS_ENTRYPOINT_NOT_FOUND fails the process BEFORE main, so the
# failure's own stderr never names the missing entry — enumerate every
# import up front instead. Unprefixed binutils: MSYS2's ucrt64 package
# ships objdump.exe/strip.exe WITHOUT the x86_64-w64-mingw32- alias
# (run 30697405256 proved the prefixed names do not resolve); the closed
# PATH makes the one toolchain's tools unambiguous.
for exe in target/"$TARGET"/debug/deps/*.exe; do
  echo "=== imports: $exe ==="
  objdump -p "$exe" \
    | grep -E "DLL Name|^\s+[0-9a-f]+\s+\S+\s*$" | head -60 || true
done

# --- 3. test (serialized) ---------------------------------------------------
cargo test -p tebako-cli -p tfs-cli -p tebako-pkg --target "$TARGET" -- "$SERIAL" --nocapture
