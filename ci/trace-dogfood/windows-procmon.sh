#!/usr/bin/env bash
# ci/trace-dogfood/windows-procmon.sh — the spec 25 §6.2 windows
# kernel-layer path, end to end: a procmon CSV export (the canonical
# input: golden case 06-libsass-importer's outside.csv from the PINNED
# retrace clone — SSOT, never the vendored copy) converts through
# `tebako trace import procmon` and drives `tebako trace cover` to the
# case's golden verdict. Unit-level byte parity shipped with #429; this
# leg proves the chain in CI against upstream's own files and uploads the
# escapes report as the artifact.
#
# Gates: the conversion is byte-identical to upstream's outside.json (the
# document-level parity pin); cover over the converted stream reproduces
# expected.txt byte-for-byte and the exit.txt code.
#
# Env (required): RETRACE_SRC — the pinned retrace clone
#                 TEBAKO      — the tebako binary (tebako.exe)
# Env (optional): WORK, ARTIFACT_DIR
set -euo pipefail

: "${RETRACE_SRC:?set to the pinned retrace clone}"
: "${TEBAKO:?set to the tebako binary}"
# The scratch dir anchors at the cwd (the workflow's workspace): every
# path handed to a NATIVE binary (tebako.exe) stays relative/plain —
# mktemp's /tmp/... form does not resolve for non-msys processes.
WORK="${WORK:-$PWD/.dogfood-work-procmon}"
mkdir -p "$WORK" "${ARTIFACT_DIR:-$WORK/artifacts}"
ART="${ARTIFACT_DIR:-$WORK/artifacts}"

fail() { echo "windows-procmon: FAIL: $*" >&2; exit 1; }

CASE="$RETRACE_SRC/tools/correlate/golden/06-libsass-importer"
[ -f "$CASE/outside.csv" ] || fail "no outside.csv in $CASE (upstream moved the procmon fixture)"

# 1. The document-level parity pin: the conversion IS upstream's
#    outside.json, byte for byte.
rc=0
"$TEBAKO" trace import procmon "$CASE/outside.csv" > "$ART/converted.json" 2> "$ART/import.stderr" || rc=$?
[ "$rc" = 0 ] || fail "import rc=$rc: $(cat "$ART/import.stderr")"
cmp -s "$ART/converted.json" "$CASE/outside.json" \
  || fail "the conversion drifted from upstream's outside.json"

# 2. The converted stream drives cover to the golden verdict.
prefix=$(head -1 "$CASE/prefix.txt" | tr -d '\r\n')
want_rc=$(head -1 "$CASE/exit.txt" | tr -d '\r\n')
opts=()
if [ -f "$CASE/options.txt" ]; then
  # shellcheck disable=SC2207
  opts=($(head -1 "$CASE/options.txt" | tr -d '\r\n'))
fi
rc=0
"$TEBAKO" trace cover --inside "$CASE/inside.json" --outside "$ART/converted.json" \
  --prefix "$prefix" --layer kernel ${opts[@]+"${opts[@]}"} \
  > "$ART/cover.stdout" 2> "$ART/cover.stderr" || rc=$?
[ "$rc" = "$want_rc" ] || fail "cover rc=$rc != exit.txt $want_rc"
cmp -s "$ART/cover.stdout" "$CASE/expected.txt" \
  || { diff "$CASE/expected.txt" "$ART/cover.stdout" >&2 || :; fail "cover stdout != expected.txt"; }
grep -q 'outside capture layer:' "$ART/cover.stderr" \
  || fail "the report does not name the producing layer (§6.1)"

echo "windows-procmon: the procmon chain reproduced the golden verdict (exit $want_rc)"
echo "windows-procmon: conversion + escapes report in $ART"
