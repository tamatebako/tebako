#!/usr/bin/env bash
# ci/trace-dogfood/windows-gnu-build.sh — build tebako-cli for the
# windows trace-dogfood legs on the SHIPPED target (x86_64-pc-windows-
# gnu, ucrt64). The env discipline is ci/windows-gnu-cli.sh's, verbatim:
# the closed PATH (ucrt64 first, setup-msys2's /usr/bin stays OFF), the
# `make` toolshim for botan-src, the static-runtime link wrapper, the
# Botan/bindgen steering. Build-only — the import gate and the test run
# stay with the test-windows-gnu-cli leg that owns them.
set -euo pipefail

export PATH="/d/a/_temp/msys64/ucrt64/bin:/c/Program Files/Git/usr/bin:/c/Program Files/Git/cmd:/c/Users/runneradmin/.cargo/bin:/c/Windows/System32"

TOOLSHIM=/d/a/_temp/tebako-toolshim
mkdir -p "$TOOLSHIM"
cp "/d/a/_temp/msys64/ucrt64/bin/mingw32-make.exe" "$TOOLSHIM/make.exe"
export PATH="$TOOLSHIM:$PATH"

WRAP="$RUNNER_TEMP/tebako-link-wrap.exe"
gcc -O2 -o "$WRAP" ci/windows-gnu-link-wrap.c
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$(cygpath -w "$WRAP")"

export RUSTFLAGS="-C link-arg=-static-libgcc -C link-arg=-static-libstdc++ -C link-arg=-lmsvcrt"
export BOTAN_CONFIGURE_CC=gcc

UCRT64=/d/a/_temp/msys64/ucrt64
GCC_INCLUDE=$(echo "$UCRT64"/lib/gcc/x86_64-w64-mingw32/*/include)
if [ ! -d "$GCC_INCLUDE" ]; then
  echo "ucrt64 gcc include dir not found under $UCRT64/lib/gcc — toolchain layout changed"
  exit 1
fi
export BINDGEN_EXTRA_CLANG_ARGS="--target=x86_64-w64-mingw32 -isystem $(cygpath -m "$UCRT64/include") -isystem $(cygpath -m "$GCC_INCLUDE")"

cargo build -p tebako-cli --target x86_64-pc-windows-gnu
