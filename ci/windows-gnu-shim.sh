#!/usr/bin/env bash
# ci/windows-gnu-shim.sh — tebako-shim and tebako-resolve on windows-gnu
# (ucrt64), end to end: build, DLL-import forensics, then the serialized
# test run. tebako-resolve rides along: its gix/reqwest TLS stack is
# provider-split on this target (rustls/aws-lc-rs — ring 0.17 does not
# compile under mingw; the tebako-http split of commit 8de4335), and this
# leg is where that stack is proven on the real toolchain.
#
# Everything the leg needs is HERE, not inline in the workflow YAML —
# run-blocks get string-edited and break silently; a script is reviewed
# and shellcheck-able. The workflow only exports the environment and
# calls this file.
set -euo pipefail

# The proven closed PATH (dwarfs-rs's ci/windows-gnu.sh): ucrt64 gcc
# first; Git's /usr/bin for coreutils (safe — the ABI clash is
# specifically setup-msys2's /usr/bin, which stays OFF); git.exe from
# Git's /cmd; cargo; System32. No choco mingw, no stray runner toolchains
# (the runner carries several mingw installs and first-DLL-wins
# resolution is STATUS_ENTRYPOINT_NOT_FOUND at process start).
export PATH="/d/a/_temp/msys64/ucrt64/bin:/c/Program Files/Git/usr/bin:/c/Program Files/Git/cmd:/c/Users/runneradmin/.cargo/bin:/c/Windows/System32"

# One linker, resolved from the closed PATH above (ucrt64's gcc).
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=gcc.exe

# Static-link the mingw C/C++ runtimes; the trailing -lmsvcrt restores
# the msvcrt-after-mingwex order invariant (rustc's -nodefaultlibs tail
# violates it; the factory probe proved libmingwex's compat _assert.o
# then dies on __imp___msvcrt_assert).
export RUSTFLAGS="-C link-arg=-static-libgcc -C link-arg=-static-libstdc++ -C link-arg=-lmsvcrt"

TARGET=x86_64-pc-windows-gnu

# The serialize note: a parallel harness hides which test crashes
# (windows-gnu legs died twice to a hidden segfault before this rule).
SERIAL="--test-threads=1"

# --- 1. build + test-compile ------------------------------------------------
cargo build -p tebako-shim -p tebako-resolve --target "$TARGET"
cargo test -p tebako-shim -p tebako-resolve --target "$TARGET" --no-run

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
cargo test -p tebako-shim -p tebako-resolve --target "$TARGET" -- "$SERIAL" --nocapture
