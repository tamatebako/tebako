#!/usr/bin/env bash
# ci/trace-dogfood/libc-linux.sh — the spec 25 §8-T3 libc-layer dogfood
# (linux): a real payload runs under BOTH interposition layers — retrace
# first (the outside libc-boundary observer) and libtfs-preload second
# (tebako's own interposition, spec 07 §8 tier 1) — one LD_PRELOAD chain:
# the app's libc call hits retrace's wrapper (logged), whose real-impl
# resolution (RTLD_NEXT) lands in the shim, which serves the VFS path or
# passes through. The inside stream is the shim's spec 25 §2 bus (armed
# from TEBAKO_TRACE at the shim constructor); the outside stream is
# retrace's JSON capture. `tebako trace cover` subtracts inside from
# outside; the escapes report is the CI artifact.
#
# Gates (the spec-honest conditions, §6.1/§7):
#   * the outside capture DID see the under-prefix calls (non-vacuity);
#   * cover exits 0 — every under-prefix libc-routed touch is covered;
#   * the raw-syscall probe's touch is INVISIBLE in the libc capture and
#     the report names sub-libc escapes UNCERTIFIABLE at this layer —
#     never "covered" (§7's honesty rule);
#   * the stderr coverage block names the producing layer.
#
# Env (required): RETRACE_LIB  — libretrace.so (the pinned release asset
#                                or a built one)
#                 SHIM         — libtfs_preload.so (cargo build
#                                -p libtfs-preload)
#                 TEBAKO       — the tebako binary (cargo build
#                                -p tebako-cli)
# Env (optional): WORK         — scratch dir (default: mktemp -d)
#                 ARTIFACT_DIR — report + captures land there when set
set -euo pipefail

: "${RETRACE_LIB:?set to libretrace.so}"
: "${SHIM:?set to libtfs_preload.so}"
: "${TEBAKO:?set to the tebako binary}"
HERE=$(cd "$(dirname "$0")" && pwd)
WORK="${WORK:-$(mktemp -d)}"
mkdir -p "$WORK"
echo "libc-linux: work dir $WORK"

fail() { echo "libc-linux: FAIL: $*" >&2; exit 1; }

# --- 1. the subject + the payload image (the libtfs-preload e2e shape) --
cc -O2 -o "$WORK/trace-subject" "$HERE/trace-subject.c" \
  || fail "cc could not build trace-subject.c"
mkdir -p "$WORK/imgroot/data"
printf 'VFS-SECRET-E2E\n' > "$WORK/imgroot/data/secret.txt"
rm -f "$WORK/img.zip"
(cd "$WORK/imgroot" && zip -q ../img.zip data/ data/secret.txt)

# --- 2. the traced runs --------------------------------------------------
INSIDE="$WORK/inside.jsonl"
OUTSIDE="$WORK/outside.json"
# Pre-create the capture files: through the preload chain an open(O_CREAT)
# rides retrace's varargs wrapper, which does not forward the mode (the
# preload-macho/mode-000 quirk) — a pre-existing file sidesteps it.
touch "$INSIDE" "$OUTSIDE"

run_subject() {
  LD_PRELOAD="$RETRACE_LIB $SHIM" \
  RETRACE_LOGGER_DEF_FN="$OUTSIDE" \
  RETRACE_LOGGER_DEF_STDOUT_ENA=0 \
  RETRACE_LOGGER_FMT=jsonl \
  TEBAKO_TRACE="$INSIDE" \
  TEBAKO_TFS_MOUNTS="$WORK/img.zip:/tfs" \
    "$WORK/trace-subject" "$@"
}

# Run 1: the raw-syscall probe (invisible at this layer) + the VFS read.
rc=0
run_subject --raw /tfs/data/raw-secret.txt /tfs/data/secret.txt > "$WORK/run1.stdout" || rc=$?
[ "$rc" = 0 ] || fail "run 1 rc=$rc"
grep -qx 'VFS-SECRET-E2E' "$WORK/run1.stdout" \
  || fail "run 1 did not print the VFS secret through the shim: $(cat "$WORK/run1.stdout")"
grep -q '^raw:/tfs/data/raw-secret.txt:' "$WORK/run1.stdout" \
  || fail "run 1: the raw probe did not report"

# Run 2 (a second process appending to the same channels — §2's
# re-derivation clause): a host passthrough read.
rc=0
run_subject /etc/hosts > "$WORK/run2.stdout" || rc=$?
[ "$rc" = 0 ] || fail "run 2 rc=$rc"
[ -s "$WORK/run2.stdout" ] || fail "run 2: /etc/hosts read came back empty"

# --- 3. the captures say what the layer model predicts --------------------
# jsonl: one compact entry per line — the greps name the ENTRY, not the doc.
grep '/tfs/data/secret.txt' "$OUTSIDE" | grep -qE 'open|stat|fopen' \
  || fail "the outside capture never saw the under-prefix calls (vacuous leg)"
if grep -q 'raw-secret' "$OUTSIDE"; then
  fail "the raw-syscall touch leaked into the LIBC capture — the layer model is broken"
fi
grep -q '"op":"open".*secret.txt\|secret.txt.*"op":"open"' "$INSIDE" \
  || fail "the inside stream has no open event for the memfs file"

# --- 4. cover: the escapes report (the CI artifact) -----------------------
mkdir -p "${ARTIFACT_DIR:-$WORK/artifacts}"
ART="${ARTIFACT_DIR:-$WORK/artifacts}"
rc=0
"$TEBAKO" trace cover --inside "$INSIDE" --outside "$OUTSIDE" --prefix /tfs \
  > "$ART/cover.stdout" 2> "$ART/cover.stderr" || rc=$?
[ "$rc" = 0 ] || { cat "$ART/cover.stdout" "$ART/cover.stderr" >&2; fail "cover rc=$rc — libc-routed escapes found"; }
grep -q 'outside capture layer:.*libc boundary' "$ART/cover.stderr" \
  || fail "the report does not name the producing layer (§6.1)"
grep -q 'UNCERTIFIABLE' "$ART/cover.stderr" \
  || fail "the report does not name sub-libc escapes UNCERTIFIABLE (§7 honesty)"
fs_line=$(grep 'coverage by surface' -A5 "$ART/cover.stderr" | grep 'fs:' || true)
[ -n "$fs_line" ] || fail "no fs coverage line in the report"
echo "$fs_line" | grep -q '100.0%' || fail "fs coverage is not 100%: $fs_line"
touches=$(echo "$fs_line" | sed -E 's/.*fs: ([0-9]+)\/.*/\1/')
[ "${touches:-0}" -ge 1 ] || fail "fs coverage is vacuous: $fs_line"

cp "$INSIDE" "$OUTSIDE" "$WORK/run1.stdout" "$WORK/run2.stdout" "$ART/" 2>/dev/null || :
echo "libc-linux: covered — $fs_line"
echo "libc-linux: escapes report + captures in $ART"
