#!/usr/bin/env bash
# ci/trace-dogfood/retrace-build.sh — build the pinned retrace from
# source. One script, two platform shapes:
#
#   linux:  the full build (the `retrace` CLI for the ptrace attach leg
#           and `retrace-correlate` for the parity leg); plain cmake +
#           ninja + cc, no third-party deps (OpenSSL has been optional
#           since v1's ssl.c was removed).
#   windows (ucrt64): the ADR-0009 shape — retrace.dll (the inline-hook
#           backend) + tools/retrace-win-run.exe + the offline tools.
#           The closed PATH is ci/windows-gnu-cli.sh's (ucrt64 first,
#           setup-msys2's /usr/bin stays OFF).
#
# Usage: retrace-build.sh <retrace-clone> <build-dir>
set -euo pipefail

SRC="${1:?usage: retrace-build.sh <retrace-clone> <build-dir>}"
BUILD="${2:?missing build dir}"

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    # The closed PATH (ci/windows-gnu-cli.sh): ucrt64 first,
    # setup-msys2's /usr/bin stays OFF.
    export PATH="/d/a/_temp/msys64/ucrt64/bin:/c/Program Files/Git/usr/bin:/c/Windows/System32"
    ;;
esac
cmake -S "$SRC" -B "$BUILD" -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build "$BUILD" --parallel

# Name what was produced — a silent layout drift fails HERE, not in a leg.
if [ "$(uname -s)" = Linux ]; then
  [ -x "$BUILD/src/cli/retrace" ] || { echo "retrace-build: no CLI at $BUILD/src/cli/retrace" >&2; exit 1; }
  [ -x "$BUILD/tools/retrace-correlate" ] || { echo "retrace-build: no retrace-correlate under $BUILD/tools" >&2; exit 1; }
  echo "retrace-build: CLI + tools built ($BUILD)"
else
  find "$BUILD" -name 'retrace.dll' | grep -q . || { echo "retrace-build: no retrace.dll under $BUILD" >&2; exit 1; }
  find "$BUILD" -name 'retrace-win-run.exe' | grep -q . || { echo "retrace-build: no retrace-win-run.exe under $BUILD" >&2; exit 1; }
  echo "retrace-build: retrace.dll + retrace-win-run.exe built ($BUILD)"
fi
