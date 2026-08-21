#!/usr/bin/env bash
# ci/trace-dogfood/kernel-linux.sh — the spec 25 §8-T3 kernel-layer
# dogfood (linux): the SAME subject as the libc leg, but the outside
# capture is retrace's ptrace backend (`retrace attach` — native and
# shipped since retrace v2.4.0; the eBPF bridge stays future work per
# §6.4). ptrace sees real syscalls only: the shim-served VFS reads never
# reach the kernel (no event), while the raw-syscall probe DOES — and the
# inside stream has no event for it (the shim was bypassed).
#
# OBSERVED UPSTREAM SHAPE (pinned, v2.14.0 — revisited when the pin
# bumps): ptrace entries carry the syscall NAME but nil params (the
# ptrace backend ships no arch_spec; the frame reaches the engine through
# the preload backend's — byte-verified on docker aarch64: openat logged
# with "path":"(nil)"). §7's "a raw-syscall fixture under a KERNEL-layer
# capture must be caught at 100%" therefore CANNOT be discharged by cover
# on this channel — path attribution is impossible and asserting it would
# be coverage theater. The leg pins the honest contract instead:
#
# Gates:
#   * the subject printed the VFS secret + the probe's report under
#     ptrace observation (the compose works);
#   * the capture contains `"func":"open"` — the raw probe's unique
#     marker (SYS_open; glibc lowers every libc open to openat, so only
#     the probe can emit `open`) — the kernel layer SAW the sub-libc
#     touch the libc layer cannot certify;
#   * the shim-served under-prefix reads left NO kernel-layer trace
#     (escape-tolerant absence grep);
#   * cover --layer kernel is honestly VACUOUS over the nil-param
#     capture: rc 0, escapes=0, NO coverage block (a surface-coverage
#     claim here would be theater), the kernel layer named (§6.1). An
#     upstream deref fix flips this gate red on purpose.
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
# OBSERVED UPSTREAM SHAPE (pinned, v2.14.0): ptrace entries carry the
# right syscall NAME but nil params — the ptrace backend ships no
# arch_spec of its own, so the frame reaches the engine through the
# preload backend's (a WrapperSystemVFrame cast of retrace_ptrace_frame;
# verified byte-level on aarch64 docker: openat logged with
# "dirfd":"0","path":"(nil)"). Path attribution is therefore IMPOSSIBLE
# on this channel at v2.14.0, and the gates below key on the func NAME:
# the raw probe rides SYS_open — a name no glibc process emits at the
# syscall layer (libc open always lowers to openat) — so a `"func":"open"`
# entry IS the probe. If upstream ever fixes the deref, gate (c) flips
# red loudly and this pin gets revisited.
#
# (a) non-vacuity: the kernel layer saw the raw probe.
grep -q '"func":"open"' "$OUTSIDE" \
  || fail "the KERNEL capture missed the raw-syscall probe (func=open) — ptrace leg is vacuous"
# (b) absence: a shim-served memfs read must leave NO kernel-layer trace.
#     Escape-tolerant on purpose ('\/' and '/') — a false PASS here would
#     be silent coverage theater.
if grep -qE 'data(/|\\/)secret\.txt' "$OUTSIDE"; then
  fail "the shim-served VFS read reached the kernel — memfs reads must be syscall-free"
fi

# --- 5. cover --layer kernel: assert the capture's HONEST expressiveness --
# Path-less entries are un-attributable by cover's design (paths are
# path-like strings at any depth), so over a v2.14.0 ptrace capture cover
# must report ZERO under-prefix touches — no coverage block at all — and
# exit 0. Asserting a caught escape here would be theater; asserting the
# vacuity keeps the leg fail-closed against silent regressions AND
# against a future upstream fix (which would flip this red for review).
rc=0
"$TEBAKO" trace cover --inside "$INSIDE" --outside "$OUTSIDE" --prefix /tfs \
  --layer kernel > "$ART/cover.stdout" 2> "$ART/cover.stderr" || rc=$?
[ "$rc" = 0 ] || { cat "$ART/cover.stdout" "$ART/cover.stderr" >&2; fail "cover rc=$rc — the v2.14.0 pin says NO escape is attributable from a nil-param ptrace capture; investigate what changed"; }
grep -q 'escapes=0' "$ART/cover.stderr" \
  || fail "cover reported escapes over a nil-param capture: $(cat "$ART/cover.stderr")"
if grep -q 'coverage by surface' "$ART/cover.stderr"; then
  fail "cover claimed surface coverage over a path-less capture — that is coverage theater"
fi
grep -q 'outside capture layer:.*kernel syscall layer' "$ART/cover.stderr" \
  || fail "the report does not name the kernel layer (§6.1)"

cp "$INSIDE" "$OUTSIDE" "$WORK/subject.stdout" "$WORK/attach.stderr" "$ART/" 2>/dev/null || :
echo "kernel-linux: the raw probe IS visible at the kernel layer (func=open marker);"
echo "kernel-linux: path attribution UNCERTIFIABLE at retrace v2.14.0 (ptrace params nil) — cover's vacuity asserted, the func-level pin is the compensating control"
echo "kernel-linux: escapes report + captures in $ART"
