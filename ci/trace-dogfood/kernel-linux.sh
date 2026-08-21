#!/usr/bin/env bash
# ci/trace-dogfood/kernel-linux.sh — the spec 25 §8-T3 kernel-layer
# dogfood (linux): the SAME subject as the libc leg, but the outside
# capture is retrace's ptrace backend (`retrace attach` — native and
# shipped since retrace v2.4.0; the eBPF bridge stays future work per
# §6.4). ptrace sees real syscalls only: the shim-served VFS reads never
# reach the kernel (no event), while the raw-syscall probe's openat of an
# under-prefix path DOES — and the inside stream has no event for it (the
# shim was bypassed). cover must therefore CATCH it: §7's "a raw-syscall
# fixture under a KERNEL-layer capture must be caught at 100%", with the
# report naming the kernel layer (§6.1).
#
# Gates:
#   * the attach produced a well-formed capture naming the raw path;
#   * cover --layer kernel exits 1 and the escape line names the raw
#     probe's path (the sub-libc escape, caught);
#   * the shim-served under-prefix reads produced NO kernel-layer touch
#     (memfs reads are invisible even to ptrace — the honest direction);
#   * stderr names the kernel syscall layer.
#
# Env (required): RETRACE — the retrace CLI binary (built from the
#                           pinned tag; attach needs no preload library)
#                 SHIM   — libtfs_preload.so
#                 TEBAKO — the tebako binary
# Env (optional): RETRACE_LIB — libretrace.so for attach's dlopen
#                               (derived from the build layout otherwise)
#                 WORK, ARTIFACT_DIR — as libc-linux.sh
set -euo pipefail

: "${RETRACE:?set to the retrace CLI binary}"
: "${SHIM:?set to libtfs_preload.so}"
: "${TEBAKO:?set to the tebako binary}"
# `retrace attach` dlopens libretrace.so (the ptrace backend lives in the
# library) and fails closed without RETRACE_LIB when the lib is not beside
# the CLI (retrace_cli.c: find_library). Derive it from the build layout
# (build/src/cli/retrace → build/src/v2/libretrace.so) unless given.
if [ -z "${RETRACE_LIB:-}" ]; then
  RETRACE_LIB=$(find "$(cd "$(dirname "$RETRACE")/.." && pwd)" -name 'libretrace.so*' 2>/dev/null | sort | head -1)
fi
[ -n "$RETRACE_LIB" ] && [ -f "$RETRACE_LIB" ] \
  || { echo "kernel-linux: FAIL: no libretrace.so near $RETRACE — set RETRACE_LIB" >&2; exit 1; }
HERE=$(cd "$(dirname "$0")" && pwd)
WORK="${WORK:-$(mktemp -d)}"
mkdir -p "$WORK"
echo "kernel-linux: work dir $WORK"
# The escapes report is the artifact — land SOMETHING on every failure.
mkdir -p "${ARTIFACT_DIR:-$WORK/artifacts}"
ART="${ARTIFACT_DIR:-$WORK/artifacts}"

fail() {
  echo "kernel-linux: FAIL: $*" >&2
  cp "$INSIDE" "$OUTSIDE" "$WORK/subject.stdout" "$WORK/attach.stderr" "$ART/" 2>/dev/null || :
  [ -f "$OUTSIDE" ] && { echo "kernel-linux: outside capture head:" >&2; head -c 2000 "$OUTSIDE" >&2 || :; }
  exit 1
}

# The capture config — the CLI's built-in default spelled out (see
# libc-linux.sh; without it the engine logs NOTHING).
CONF="$WORK/retrace-conf.json"
cat > "$CONF" <<'JSON'
{"intercept_scripts":[{"func_name":"*","actions":[{"action_name":"log_params"},{"action_name":"call_real"}]}]}
JSON

# --- 1. subject + image (identical shape to the libc leg) ----------------
cc -O2 -o "$WORK/trace-subject" "$HERE/trace-subject.c" \
  || fail "cc could not build trace-subject.c"
mkdir -p "$WORK/imgroot/data"
printf 'VFS-SECRET-E2E\n' > "$WORK/imgroot/data/secret.txt"
rm -f "$WORK/img.zip"
(cd "$WORK/imgroot" && zip -q ../img.zip data/ data/secret.txt)

# --- 2. ptrace permission: attach targets a process that is NOT retrace's
#        child, which yama's default ptrace_scope=1 denies (retrace's own
#        cli.md prescribes the relax); root would do as well. The file is
#        root-owned, so the relax must ride sudo unconditionally — a `-w`
#        guard silently skips it on CI runners.
if [ -e /proc/sys/kernel/yama/ptrace_scope ]; then
  echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope > /dev/null
  [ "$(cat /proc/sys/kernel/yama/ptrace_scope)" = "0" ] \
    || fail "could not relax yama ptrace_scope — attach will be denied"
fi

# --- 3. the traced run: shimmed subject + ptrace attach ------------------
INSIDE="$WORK/inside.jsonl"
OUTSIDE="$WORK/outside.json"
touch "$INSIDE" "$OUTSIDE"
HS="$WORK/handshake"
mkdir -p "$HS"

LD_PRELOAD="$SHIM" \
TEBAKO_TRACE="$INSIDE" \
TEBAKO_TFS_MOUNTS="$WORK/img.zip:/tfs" \
  "$WORK/trace-subject" --wait "$HS" --raw /tfs/data/raw-secret.txt \
    /tfs/data/secret.txt /etc/hosts > "$WORK/subject.stdout" &
subject_pid=$!

# Wait for the subject's ready file (it writes its pid).
for _ in $(seq 1 100); do [ -f "$HS/ready" ] && break; sleep 0.1; done
[ -f "$HS/ready" ] || fail "the subject never signalled ready"

RETRACE_LIB="$RETRACE_LIB" RETRACE_LOGGER_FMT=jsonl \
  "$RETRACE" attach --config "$CONF" --log "$OUTSIDE" "$subject_pid" > "$WORK/attach.stdout" 2> "$WORK/attach.stderr" &
attach_pid=$!
# Let the attach land before releasing the subject (retrace prints its
# "tracing pid" line when attached; fall through on a generous timeout —
# the subject's own go-timeout fails the leg loudly if attach is stuck).
for _ in $(seq 1 100); do grep -q "tracing pid" "$WORK/attach.stderr" 2>/dev/null && break; sleep 0.1; done
touch "$HS/go"
rc=0
wait "$subject_pid" || rc=$?
[ "$rc" = 0 ] || fail "subject rc=$rc (stderr above; $WORK/subject.stdout: $(cat "$WORK/subject.stdout"))"
wait "$attach_pid" || fail "retrace attach failed: $(cat "$WORK/attach.stderr")"

grep -qx 'VFS-SECRET-E2E' "$WORK/subject.stdout" \
  || fail "the VFS read was not served under ptrace observation"
grep -q '^raw:/tfs/data/raw-secret.txt:' "$WORK/subject.stdout" \
  || fail "the raw probe did not report"

# --- 4. the layer model, verified against the kernel capture -------------
# (jsonl — one entry per line — so the grep names the ENTRY: the raw
# touch must ride an openat-family syscall entry, not e.g. a write
# payload echoing the probe's own stdout report. retrace escapes '/' as
# '\/' in JSON — the slash-free filename is the robust key. If the ptrace
# backend ever stops dereferencing target strings this gate fails LOUDLY
# with the capture dumped — the shape is asserted, never assumed.)
grep 'raw-secret' "$OUTSIDE" | grep -q 'openat' \
  || fail "the KERNEL capture missed the raw-syscall openat — ptrace leg is vacuous"
# Absence gate: a shim-served memfs read must leave NO kernel-layer trace.
# Escape-tolerant on purpose ('\/' and '/') — a false PASS here would be
# silent coverage theater.
if grep -qE 'data(/|\\/)secret\.txt' "$OUTSIDE"; then
  fail "the shim-served VFS read reached the kernel — memfs reads must be syscall-free"
fi

# --- 5. cover --layer kernel: the escape MUST be caught ------------------
rc=0
"$TEBAKO" trace cover --inside "$INSIDE" --outside "$OUTSIDE" --prefix /tfs \
  --layer kernel > "$ART/cover.stdout" 2> "$ART/cover.stderr" || rc=$?
[ "$rc" = 1 ] || { cat "$ART/cover.stdout" "$ART/cover.stderr" >&2; fail "cover rc=$rc — the raw-syscall escape was NOT caught at the kernel layer"; }
grep -q '^escape /tfs/data/raw-secret.txt ' "$ART/cover.stdout" \
  || fail "the escapes report does not name the raw-syscall path: $(cat "$ART/cover.stdout")"
grep -q 'outside capture layer:.*kernel syscall layer' "$ART/cover.stderr" \
  || fail "the report does not name the kernel layer (§6.1)"

cp "$INSIDE" "$OUTSIDE" "$WORK/subject.stdout" "$WORK/attach.stderr" "$ART/" 2>/dev/null || :
echo "kernel-linux: the sub-libc escape was caught — $(cat "$ART/cover.stdout")"
echo "kernel-linux: escapes report + captures in $ART"
