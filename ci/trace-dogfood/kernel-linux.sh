#!/usr/bin/env bash
# ci/trace-dogfood/kernel-linux.sh — the spec 25 §8-T3 kernel-layer
# dogfood (linux): the SAME subject as the libc leg, observed at the
# kernel syscall layer. Channel (pinned, round 4): strace capture →
# upstream's OWN retrace-strace2retrace converter (built from the pinned
# v2.14.0 clone — tools/strace2retrace, "the Linux kernel-truth
# converter") → retrace-shaped outside.json → tebako trace cover.
#
# Why not `retrace attach` (retrace's ptrace backend): at v2.14.0 that
# channel cannot attribute paths on either probed arch — the ptrace
# backend ships no arch_spec of its own, so the frame reaches the engine
# through the preload backend's param wrappers:
#   * aarch64 (docker probe): entries carry the syscall NAME but nil
#     params — a real openat(AT_FDCWD, "/tmp/raw-marker.txt", O_RDONLY)
#     arrived as "func":"openat","dirfd":"0","path":"(nil)";
#   * x86_64 (GH runner, run 32451535675 job 96680888031): the same wild
#     frame deref is FATAL — SIGSEGV right after the first logged event.
# §7's promise — "a raw-syscall fixture under a KERNEL-layer capture must
# be caught at 100%" — is undischargable on that channel at this pin.
# strace is the mature ptrace client and retrace itself ships the
# normalizer: the exact §6.2 producer-converter pattern the windows
# kernel layer already rides (procmon → converter). Same kernel truth,
# same pinned provenance, REAL paths — so this leg asserts the §7 STRONG
# contract the nil-params pin had to waive.
#
# Gates (all fail-closed):
#   * the subject printed the VFS secret + the probe's report under
#     strace (the compose works);
#   * the RAW strace log provably contains the probe's open-family line
#     (non-vacuity BEFORE any conversion);
#   * the shim-served under-prefix read left NO kernel-layer trace
#     (memfs reads are syscall-free — a hit here means the interposition
#     leaked to the kernel);
#   * the converter converted ≥1 line AND the probe survived conversion
#     with its path paired to an open-family func;
#   * cover --layer kernel: rc 1, EXACTLY ONE escape, the escape line
#     names the probe's path with class=read, and the report names the
#     kernel layer (§6.1).
# The tripwire survives the channel change: any upstream drift in the
# converter's output shape (on a pin bump) flips a gate red with the
# captures dumped.
#
# Env (required): RETRACE — the retrace CLI binary (built from the
#                           pinned tag; the converter path derives from
#                           its build layout)
#                 SHIM   — libtfs_preload.so
#                 TEBAKO — the tebako binary
# Env (optional): STRACE2RETRACE — the converter binary (derived from
#                                  RETRACE's build layout otherwise)
#                 WORK, ARTIFACT_DIR — as libc-linux.sh
set -euo pipefail

: "${RETRACE:?set to the retrace CLI binary}"
: "${SHIM:?set to libtfs_preload.so}"
: "${TEBAKO:?set to the tebako binary}"
# The converter rides the pinned source build (build/tools/) — derive it
# from the CLI's path (build/src/cli/retrace → build) unless given.
if [ -z "${STRACE2RETRACE:-}" ]; then
  STRACE2RETRACE="$(cd "$(dirname "$RETRACE")/../.." && pwd)/tools/retrace-strace2retrace"
fi
[ -x "$STRACE2RETRACE" ] \
  || { echo "kernel-linux: FAIL: no retrace-strace2retrace at $STRACE2RETRACE" >&2; exit 1; }
# strace is preinstalled on the ubuntu runners; install only when missing.
if ! command -v strace > /dev/null; then
  sudo apt-get update -qq
  sudo apt-get install -y --no-install-recommends strace
fi
command -v strace > /dev/null \
  || { echo "kernel-linux: FAIL: no strace and apt could not provide it" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
WORK="${WORK:-$(mktemp -d)}"
mkdir -p "$WORK"
echo "kernel-linux: work dir $WORK"
# The escapes report is the artifact — land SOMETHING on every failure.
mkdir -p "${ARTIFACT_DIR:-$WORK/artifacts}"
ART="${ARTIFACT_DIR:-$WORK/artifacts}"
INSIDE="$WORK/inside.jsonl"
SLOG="$WORK/subject.strace"
OUTSIDE="$WORK/outside.json"

fail() {
  echo "kernel-linux: FAIL: $*" >&2
  cp "$INSIDE" "$OUTSIDE" "$SLOG" "$WORK/subject.stdout" "$WORK/subject.stderr" \
     "$WORK/convert.stderr" "$ART/" 2>/dev/null || :
  [ -f "$SLOG" ] && { echo "kernel-linux: strace log head:" >&2; head -c 2000 "$SLOG" >&2 || :; }
  [ -f "$OUTSIDE" ] && { echo "kernel-linux: outside capture head:" >&2; head -c 2000 "$OUTSIDE" >&2 || :; }
  exit 1
}

# --- 1. subject + image (identical shape to the libc leg) ----------------
cc -O2 -o "$WORK/trace-subject" "$HERE/trace-subject.c" \
  || fail "cc could not build trace-subject.c"
mkdir -p "$WORK/imgroot/data"
printf 'VFS-SECRET-E2E\n' > "$WORK/imgroot/data/secret.txt"
rm -f "$WORK/img.zip"
(cd "$WORK/imgroot" && zip -q ../img.zip data/ data/secret.txt)

# --- 2. the traced run: strace from launch (ptrace on strace's own child —
#        yama never enters: no attach of a foreign process), the shim
#        preloaded inside -------------------------------------------------
: > "$INSIDE"
strace -f -s 256 -e trace=%file -o "$SLOG" \
  env LD_PRELOAD="$SHIM" \
      TEBAKO_TRACE="$INSIDE" \
      TEBAKO_TFS_MOUNTS="$WORK/img.zip:/tfs" \
    "$WORK/trace-subject" --raw /tfs/data/raw-secret.txt \
      /tfs/data/secret.txt /etc/hosts \
  > "$WORK/subject.stdout" 2> "$WORK/subject.stderr" \
  || fail "the subject failed under strace: $(cat "$WORK/subject.stdout" "$WORK/subject.stderr")"

grep -qx 'VFS-SECRET-E2E' "$WORK/subject.stdout" \
  || fail "the VFS read was not served under strace observation"
grep -q '^raw:/tfs/data/raw-secret.txt:' "$WORK/subject.stdout" \
  || fail "the raw probe did not report"

# --- 3. the raw strace log: non-vacuity BEFORE any conversion ------------
# (a) the probe's open-family line is present (SYS_open where the arch has
#     it, SYS_openat otherwise — trace-subject.c pins the choice).
grep -E '\bopen(at|at2)?\(' "$SLOG" | grep -q 'raw-secret\.txt' \
  || fail "the KERNEL capture missed the raw-syscall probe — the strace log is vacuous"
# (b) absence: a shim-served memfs read must leave NO kernel-layer trace.
#     execve lines are excluded: strace echoes the subject's ARGV (which
#     names the under-prefix files as arguments) — an argv echo is not a
#     kernel touch of the path.
if grep -v 'execve(' "$SLOG" | grep -q 'data/secret\.txt'; then
  fail "the shim-served VFS read reached the kernel — memfs reads must be syscall-free"
fi

# --- 4. upstream's converter: strace log → retrace-shaped outside.json ---
"$STRACE2RETRACE" -o "$OUTSIDE" "$SLOG" 2> "$WORK/convert.stderr" \
  || fail "retrace-strace2retrace failed: $(cat "$WORK/convert.stderr")"
n=$(grep -oE '[0-9]+ syscall lines converted' "$WORK/convert.stderr" | grep -oE '^[0-9]+' || :)
[ "${n:-0}" -ge 1 ] \
  || fail "the converter produced an EMPTY stream from a non-vacuous log: $(cat "$WORK/convert.stderr")"
# The probe survived conversion, its path paired to an open-family func
# (parson preserves insertion order — func then path, the golden tree's
# shape — and escapes '/' as '\/', so the path patterns stay
# escape-tolerant).
grep -qE '"func":"open(at|at2)?","path":"[^"]*data(/|\\/)raw-secret\.txt"' "$OUTSIDE" \
  || fail "the converted stream lost the probe's path attribution: $(head -c 1000 "$OUTSIDE")"
if grep -qE 'data(/|\\/)secret\.txt' "$OUTSIDE"; then
  fail "the shim-served VFS read survived into the kernel stream — impossible unless the raw-log absence gate lied"
fi

# --- 5. cover --layer kernel: the escape MUST be caught (§7, 100%) -------
rc=0
"$TEBAKO" trace cover --inside "$INSIDE" --outside "$OUTSIDE" --prefix /tfs \
  --layer kernel > "$ART/cover.stdout" 2> "$ART/cover.stderr" || rc=$?
[ "$rc" = 1 ] || { cat "$ART/cover.stdout" "$ART/cover.stderr" >&2; fail "cover rc=$rc — the raw-syscall escape was NOT caught at the kernel layer"; }
grep -qE 'escapes=1([^0-9]|$)' "$ART/cover.stderr" \
  || fail "expected EXACTLY ONE escape (the raw probe); the report says: $(cat "$ART/cover.stderr")"
grep -q '^escape /tfs/data/raw-secret.txt .*class=read' "$ART/cover.stdout" \
  || fail "the escapes report does not name the probe's path as a read: $(cat "$ART/cover.stdout")"
grep -q 'outside capture layer:.*kernel syscall layer' "$ART/cover.stderr" \
  || fail "the report does not name the kernel layer (§6.1)"

cp "$INSIDE" "$OUTSIDE" "$SLOG" "$WORK/subject.stdout" "$WORK/subject.stderr" \
   "$WORK/convert.stderr" "$ART/" 2>/dev/null || :
echo "kernel-linux: the sub-libc escape was CAUGHT at the kernel layer — $(cat "$ART/cover.stdout")"
echo "kernel-linux: channel: strace -f -e trace=%file → retrace-strace2retrace (pinned v2.14.0) → cover"
echo "kernel-linux: escapes report + captures in $ART"
