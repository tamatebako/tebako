#!/bin/bash
# macos-floor-symbol-check.sh — the macOS floor's symbol gate.
#
# Every undefined dynamic symbol a shipped product references must exist
# in the floor's export set (ci/floor-symbols/macos-14.0.txt — the five
# libraries our binaries import: libSystem.B, libc++.1, libc++abi,
# CoreFoundation, Security, extracted from the 14.0 SDK's .tbd symbol
# lists; regenerate against the floor SDK when the floor moves).
#
# Why: tebako-runtime-ruby#323. A component compiled against the current
# SDK's headers can emit a call to a symbol the floor's system libraries
# do not export (v0.17.6's exe died at dyld on macOS 14.1.1:
# std::exception_ptr::__from_native_exception_pointer +
# __cxa_init_primary_exception, both introduced with macOS 15's libc++
# family). The LC_BUILD_VERSION minos records intent; this gate checks
# what the runtime can actually resolve. A hard reference fails the leg;
# a WEAK reference to an absent symbol fails too (the code paths that
# would call it do not probe first).
#
# Usage: macos-floor-symbol-check.sh <linked Mach-O product> [...]
# (exes and dylibs — NOT archives: an archive's undefined set includes
# what sibling archives define, which only a link resolves; link first,
# gate the product.)
# Exit 0 when every undefined symbol of every input is in the floor set;
# exit 1 naming every offender otherwise.
set -uo pipefail

FLOOR="${FLOOR_SYMBOL_SET:-$(cd "$(dirname "$0")" && pwd)/floor-symbols/macos-14.0.txt}"
[ -f "$FLOOR" ] || { echo "::error::floor gate: no floor symbol set at $FLOOR"; exit 64; }

status=0
for product in "$@"; do
  [ -f "$product" ] || { echo "::error::floor gate: no such product $product"; status=1; continue; }
  if head -c 8 "$product" | grep -q '!<arch>'; then
    echo "::error::floor gate: $product is an archive — link its members into a product and gate THAT"
    status=1
    continue
  fi
  undef=$(mktemp)
  miss=$(mktemp)
  # nm -m: undefined lines read "(undefined) external _SYM (from LIB)" or
  # "(undefined) weak external _SYM (from LIB)". BSD sed has no \? — two
  # patterns. Weak imports fail too: the code paths that would call the
  # absent symbol do not probe first.
  nm -m "$product" 2>/dev/null \
    | sed -n -e 's/^ *(undefined) external \([^ ]*\).*/\1/p' \
             -e 's/^ *(undefined) weak external \([^ ]*\).*/\1/p' \
    | LC_ALL=C sort -u > "$undef"
  # LC_ALL=C is load-bearing, on both sides of the comm: the set leads
  # with 26k `$ld$` linker-directive lines (`$` < `_` byte-wise), and a
  # locale-collated comm (Darwin 15's en_US.UTF-8 ignores punctuation on
  # the first pass — "ld" > "error") walks the two files in different
  # orders and reports EVERY symbol as absent (the v2.8.75-floor-gate
  # rehearsal's 155/356/443/437/464 false-miss legs).
  n_floor=$(wc -l < "$FLOOR" | tr -d ' ')
  LC_ALL=C comm -23 "$undef" "$FLOOR" > "$miss"
  n=$(wc -l < "$miss" | tr -d ' ')
  if [ "$n" != "0" ]; then
    echo "::error::floor gate: $product references $n symbol(s) absent from the macOS 14.0 floor ($n_floor exports):"
    sed 's/^/::error::  /' "$miss"
    status=1
  else
    echo "floor gate: $product clean ($(wc -l < "$undef" | tr -d ' ') undefined symbols, all in the floor set)"
  fi
  rm -f "$undef" "$miss"
done
exit "$status"
