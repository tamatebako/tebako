#!/usr/bin/env bash
# ci/trace-dogfood/parity.sh — the spec 25 §6.3 parity leg: the FORMAT is
# retrace's single source; the ALGORITHM has two implementations that must
# agree. Runs BOTH correlators (upstream retrace-correlate and `tebako
# trace cover`) over the shared golden fixtures from the PINNED retrace
# clone (SSOT — never the vendored copy) and asserts byte-identical stdout
# and identical exit codes per case, each checked against the fixture's
# own expected.txt / exit.txt (so upstream drift fails loudly and names
# the side that moved).
#
# Usage: parity.sh <retrace-clone> <retrace-correlate-bin> <tebako-bin>
# Env:   ARTIFACT_DIR — when set, each case's outputs land there.
set -euo pipefail

SRC="${1:?usage: parity.sh <retrace-clone> <retrace-correlate-bin> <tebako-bin>}"
CORRELATE="${2:?missing retrace-correlate binary}"
TEBAKO="${3:?missing tebako binary}"
GOLDEN="$SRC/tools/correlate/golden"
[ -d "$GOLDEN" ] || { echo "parity: no golden tree at $GOLDEN" >&2; exit 2; }

# The golden runner's chomp: first line, trailing CR/LF stripped.
line_of() { head -1 "$1" | tr -d '\r\n'; }

cases=0
failures=0
for case_dir in "$GOLDEN"/*/; do
  [ -f "$case_dir/inside.json" ] || continue
  name=$(basename "$case_dir")
  cases=$((cases + 1))
  prefix=$(line_of "$case_dir/prefix.txt")
  want_rc=$(line_of "$case_dir/exit.txt")
  # options.txt is optional: verbatim extra flags, word-split like the
  # shell splice in retrace's golden_runner (golden README).
  opts=()
  if [ -f "$case_dir/options.txt" ]; then
    # shellcheck disable=SC2207
    opts=($(line_of "$case_dir/options.txt"))
  fi

  out_re="${TMPDIR:-/tmp}/parity-$name.retrace.out"
  out_te="${TMPDIR:-/tmp}/parity-$name.tebako.out"
  rc_re=0; rc_te=0
  "$CORRELATE" --inside "$case_dir/inside.json" --outside "$case_dir/outside.json" \
    --prefix "$prefix" ${opts[@]+"${opts[@]}"} >"$out_re" 2>/dev/null || rc_re=$?
  "$TEBAKO" trace cover --inside "$case_dir/inside.json" --outside "$case_dir/outside.json" \
    --prefix "$prefix" ${opts[@]+"${opts[@]}"} >"$out_te" 2>"$out_te.err" || rc_te=$?

  ok=yes
  [ "$rc_re" = "$want_rc" ] || { ok=no; echo "parity: $name: retrace-correlate rc $rc_re != exit.txt $want_rc (upstream drift?)"; }
  cmp -s "$out_re" "$case_dir/expected.txt" || { ok=no; echo "parity: $name: retrace-correlate stdout != expected.txt (upstream drift?)"; }
  [ "$rc_te" = "$rc_re" ] || { ok=no; echo "parity: $name: tebako rc $rc_te != retrace rc $rc_re"; }
  cmp -s "$out_te" "$out_re" || { ok=no; echo "parity: $name: tebako stdout != retrace stdout"; diff "$out_re" "$out_te" || :; }
  # §6.1's honesty: tebako's stderr names the producing layer (outside
  # the golden contract, asserted here — the report is the artifact).
  grep -q "outside capture layer:" "$out_te.err" || { ok=no; echo "parity: $name: tebako stderr does not name the producing layer"; }
  [ "$ok" = yes ] || failures=$((failures + 1))

  if [ "${ARTIFACT_DIR:-}" ]; then
    mkdir -p "$ARTIFACT_DIR/$name"
    cp "$out_re" "$ARTIFACT_DIR/$name/retrace.out"
    cp "$out_te" "$ARTIFACT_DIR/$name/tebako.out"
    cp "$out_te.err" "$ARTIFACT_DIR/$name/tebako.stderr"
  fi
done

[ "$cases" -ge 10 ] || { echo "parity: expected >= 10 golden cases, found $cases" >&2; exit 2; }
if [ "$failures" -gt 0 ]; then
  echo "parity: $failures of $cases cases FAILED" >&2
  exit 1
fi
echo "parity: $cases golden cases — retrace-correlate and tebako trace cover agree byte-for-byte"
