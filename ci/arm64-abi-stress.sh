#!/bin/sh
# arm64-abi-stress.sh — tebako#874's recurrence rig, in the repo.
#
# The signature it hunts: on the arm64-linux leg the tebako-info lib test
# binary aborted ONCE with `free(): double free detected in tcache 2`
# (SIGABRT) mid-suite; the next run of the same commit was green. A
# tcache double free is memory corruption, so the interesting question is
# which test dies and where — and the plain suite cannot answer it: cargo
# reports the binary's exit, and the test list is only printed after the
# binary finishes (which an abort prevents).
#
# This script answers it by running the suite ONE TEST PER PROCESS-CALL at
# a time (`--test-threads=1`), under glibc's malloc checker
# (`MALLOC_CHECK_=3`, abort on any inconsistency) and with freed memory
# poisoned (`MALLOC_PERTURB_=165`, so a use-after-free reads garbage
# instead of a plausible value). Each iteration names itself before it
# runs, so the last line of the log names the iteration that died; the
# suite's own output for that iteration follows the failure line.
#
# Usage: ci/arm64-abi-stress.sh [runs] [package] [target]
#   runs     iterations (default 40)
#   package  cargo package to stress (default tebako-info)
#   target   test target kind: `lib` (default) or `--test <name>`
#
# Exit: 0 when every iteration passed; 1 on the first failing iteration
# (the log tail is reproduced on stdout); 64 on a bad argument.
set -u

RUNS=${1:-40}
PACKAGE=${2:-tebako-info}
KIND=${3:-lib}

case "$RUNS" in
  '' | *[!0-9]*)
    echo "arm64-abi-stress: runs must be a positive integer (got '$RUNS')" >&2
    exit 64
    ;;
esac

if [ "$KIND" = "lib" ]; then
  TARGET_ARGS="--lib"
else
  TARGET_ARGS="--test $KIND"
fi

export MALLOC_CHECK_=3
export MALLOC_PERTURB_=165
export RUST_BACKTRACE=1

echo "arm64-abi-stress: package=$PACKAGE target=$KIND runs=$RUNS"
echo "arm64-abi-stress: uname=$(uname -srm), glibc=$(getconf GNU_LIBC_VERSION 2>/dev/null || echo unknown)"

i=1
while [ "$i" -le "$RUNS" ]; do
  echo "=== run $i/$RUNS ==="
  # shellcheck disable=SC2086  # TARGET_ARGS is a deliberate two-word split
  if ! cargo test -q -p "$PACKAGE" $TARGET_ARGS -- --test-threads=1 2>&1; then
    echo "=== run $i/$RUNS FAILED (rc=$?) — the log above names the test that died ==="
    exit 1
  fi
  i=$((i + 1))
done

echo "arm64-abi-stress: $RUNS iterations green (no allocator abort, no test failure)"
