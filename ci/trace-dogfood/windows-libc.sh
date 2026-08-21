#!/usr/bin/env bash
# ci/trace-dogfood/windows-libc.sh — the spec 25 §8-T3 libc-layer dogfood
# on windows. There is no LD_PRELOAD on windows: retrace's inline-hook
# backend (ADR-0009) injects retrace.dll at process creation
# (retrace-win-run: CreateProcess suspended → CreateRemoteThread(
# LoadLibrary) → hooks install in DLL_PROCESS_ATTACH → resume), hooking
# the subject's ucrt file traffic (fopen/_open/... — the preload-mingw
# backend for a ucrt64-built subject).
#
# HONEST SCOPE (read before extending): no tebako interposition ships on
# windows today (libtfs-preload is POSIX; the spec 25 bus rides the
# runtime driver, and windows runtimes are the factory's story). The
# inside stream is therefore the EMPTY capture by construction, and every
# under-prefix touch is a reported escape. When the chain WORKS this leg
# proves it — retrace's windows capture → the tebako correlator → a
# well-formed escapes report that names its producing layer — and gates
# on exactly that honesty (§6.1): exit 1, the subject's file named as an
# escape, the libc boundary named with its UNCERTIFIABLE sub-libc note.
# It never asserts coverage.
#
# THE PROBE LADDER (round 5 — the upstream prerequisite is PINNED, never
# assumed): upstream never smoke-tests a live windows capture at v2.14.0
# (its msys.yml "only exercises the library scaffold + public headers"),
# and round 4 (run 32453723170, job 96686997021) showed the injection
# failing on the windows-latest runner: retrace.dll's hook installation
# COMPLETES in the child (the "hooked 'fopen' …" diagnostics print),
# then LoadLibraryA fails inside retrace_core_boot() — the loader
# converts a crash in DllMain into ERROR_DLL_INIT_FAILED, the remote
# thread's exit code reads 0, and win-run reports "failed to launch"
# (src/backends/win_common/inject.c:73). The leg therefore probes before
# it believes, fail-closed in BOTH directions:
#
#   A  control: the subject runs STANDALONE and prints the file —
#      isolates our half (subject/file/runner) from upstream's
#      injection. A failure here is OURS and fails the leg.
#   B  candidate pin: win-run with RETRACE_LOGGER_RING=0 — the MinGW
#      build compiles the lock-free log ring + lazily-spawned flusher
#      thread (the MSVC build swaps in synchronous stubs; logger.c names
#      the knob for "platforms where the background thread is
#      unstable"). If B captures, the FULL chain gates run on B's
#      capture and the sync-logger pin IS the fix.
#   C  evidence (only when B fails): the round-4 shape (ring default) —
#      must reproduce the pinned signature, else the observed state
#      drifted and the leg fails loudly.
#   D  evidence (only when B fails): ring=0 WITHOUT RETRACE_JSON_CONFIG
#      — discriminates a conf_init crash from a logger/ring crash.
#
# When B fails with the pinned signature the leg reports PREREQUISITE
# ABSENT (spec 25 §6.4's documented-prerequisite shape — the procmon
# converter half carries the windows chain certification meanwhile) and
# passes LOUDLY: the banner, the artifacts, and the §6.4 table row carry
# the evidence. A drift in EITHER direction (different failure, or a
# silent fix) fails the leg for review.
#
# Env (required): RETRACE_BUILD — the retrace cmake build dir (holding
#                                 retrace.dll + tools/retrace-win-run.exe)
#                 TEBAKO        — the tebako binary (tebako.exe)
# Env (optional): WORK, ARTIFACT_DIR
set -euo pipefail

: "${RETRACE_BUILD:?set to the retrace build dir}"
: "${TEBAKO:?set to the tebako binary}"
# ucrt64's gcc is not on the default Git-bash PATH (the setup-msys2
# install root is /d/a/_temp/msys64); take the closed PATH's ucrt64 part
# (ci/windows-gnu-cli.sh) — Git's usr/bin stays for cygpath.
export PATH="/d/a/_temp/msys64/ucrt64/bin:/c/Program Files/Git/usr/bin:/c/Windows/System32:$PATH"
HERE=$(cd "$(dirname "$0")" && pwd)
# The scratch dir anchors at the cwd (the workflow's workspace): paths
# handed to NATIVE binaries (tebako.exe, retrace-win-run.exe) stay
# relative/plain; mktemp's /tmp/... form does not resolve for them.
WORK="${WORK:-$PWD/.dogfood-work-libc}"
mkdir -p "$WORK" "${ARTIFACT_DIR:-$WORK/artifacts}"
ART="${ARTIFACT_DIR:-$WORK/artifacts}"

fail() {
  echo "windows-libc: FAIL: $*" >&2
  cp "$WORK"/*.stdout "$WORK"/*.stderr "$WORK"/*.json "$WORK"/*.jsonl "$ART/" 2>/dev/null || :
  exit 1
}

# The capture config — retrace's engine logs NOTHING without an
# intercept_scripts array (engine.c); this is the CLI's built-in default
# spelled out (config_builder.c): log_params + call_real for everything.
# Same core on every platform, so the windows dll reads it too.
CONF="$WORK/retrace-conf.json"
cat > "$CONF" <<'JSON'
{"intercept_scripts":[{"func_name":"*","actions":[{"action_name":"log_params"},{"action_name":"call_real"}]}]}
JSON

RT_DLL=$(find "$RETRACE_BUILD" -name 'retrace.dll' | head -1)
[ -n "$RT_DLL" ] || fail "no retrace.dll under $RETRACE_BUILD"
WIN_RUN=$(find "$RETRACE_BUILD" -name 'retrace-win-run.exe' | head -1)
[ -n "$WIN_RUN" ] || fail "no retrace-win-run.exe under $RETRACE_BUILD"

# 1. The subject (ucrt64 gcc) + the stand-in "virtualized" tree. The
#    prefix and the argv spelling must match byte-for-byte: cover's
#    prefix test is a case-sensitive strncmp by design (retrace parity).
cc -O2 -o "$WORK/win-subject.exe" "$HERE/win-subject.c" \
  || fail "cc could not build win-subject.c"
VFS="$WORK/vfs"
mkdir -p "$VFS"
printf 'VFS-SECRET-E2E\n' > "$VFS/secret.txt"
VFS_MIXED=$(cygpath -m "$VFS")
SECRET_MIXED="$VFS_MIXED/secret.txt"

# 2. Probe A — the control: the subject standalone. OUR half must be
#    sound before upstream's injection is indicted.
"$WORK/win-subject.exe" "$SECRET_MIXED" \
  > "$WORK/standalone.stdout" 2> "$WORK/standalone.stderr" \
  || fail "the subject does not run STANDALONE — our half is broken, not upstream's: $(cat "$WORK/standalone.stderr")"
grep -qx 'VFS-SECRET-E2E' "$WORK/standalone.stdout" \
  || fail "the standalone subject did not print the file: $(cat "$WORK/standalone.stdout")"

# 3. Probe B — the candidate pin: inject with the lock-free ring
#    disabled (synchronous logger). One run, one capture.
INSIDE="$WORK/inside.jsonl"
OUTSIDE="$WORK/outside.json"
touch "$INSIDE" "$OUTSIDE"
rcB=0
RETRACE_V2_LIB="$(cygpath -m "$RT_DLL")" \
RETRACE_JSON_CONFIG="$(cygpath -m "$CONF")" \
RETRACE_LOGGER_DEF_FN="$(cygpath -m "$OUTSIDE")" \
RETRACE_LOGGER_DEF_STDOUT_ENA=0 \
RETRACE_LOGGER_FMT=jsonl \
RETRACE_LOGGER_RING=0 \
  "$WIN_RUN" "$(cygpath -m "$WORK/win-subject.exe")" "$SECRET_MIXED" \
  > "$WORK/probeB.stdout" 2> "$WORK/probeB.stderr" || rcB=$?

if [ "$rcB" = 0 ]; then
  # The chain WORKS with the sync-logger pin — run the FULL gates.
  grep -qx 'VFS-SECRET-E2E' "$WORK/probeB.stdout" \
    || fail "probe B: the subject did not print the file under trace: $(cat "$WORK/probeB.stdout")"
  grep -q 'secret.txt' "$OUTSIDE" \
    || fail "probe B: the outside capture never saw the subject's file traffic (vacuous leg): $(cat "$WORK/probeB.stderr")"

  rc=0
  "$TEBAKO" trace cover --inside "$INSIDE" --outside "$OUTSIDE" --prefix "$VFS_MIXED" \
    --layer libc > "$ART/cover.stdout" 2> "$ART/cover.stderr" || rc=$?
  [ "$rc" = 1 ] || { cat "$ART/cover.stdout" "$ART/cover.stderr" >&2; fail "probe B: cover rc=$rc — with an empty inside stream the touches MUST escape"; }
  grep -q '^escape .*secret\.txt .*class=read' "$ART/cover.stdout" \
    || fail "probe B: the escapes report does not name the subject's file: $(cat "$ART/cover.stdout")"
  grep -q 'outside capture layer:.*libc boundary' "$ART/cover.stderr" \
    || fail "probe B: the report does not name the libc boundary (§6.1)"
  grep -q 'UNCERTIFIABLE' "$ART/cover.stderr" \
    || fail "probe B: the report does not carry the sub-libc UNCERTIFIABLE note (§7 honesty)"

  cp "$INSIDE" "$OUTSIDE" "$WORK"/standalone.* "$WORK"/probeB.* "$ART/" 2>/dev/null || :
  echo "windows-libc: the chain held under RETRACE_LOGGER_RING=0 — $(grep -c '^escape ' "$ART/cover.stdout") escape(s) reported, layer named"
  echo "windows-libc: the MinGW lock-free ring is unstable in the injected DllMain context at v2.14.0; the sync-logger pin is the leg's documented config (spec 25 §6.4)"
  echo "windows-libc: escapes report + captures in $ART"
  exit 0
fi

# 4. Probe B failed — the prerequisite stays ABSENT. Evidence probes C
#    (the round-4 shape: ring default) and D (ring=0, no config — a
#    conf_init crash discriminant). Both ride their own capture files.
echo "windows-libc: probe B (RETRACE_LOGGER_RING=0) rc=$rcB — running evidence probes C/D" >&2
rcC=0
RETRACE_V2_LIB="$(cygpath -m "$RT_DLL")" \
RETRACE_JSON_CONFIG="$(cygpath -m "$CONF")" \
RETRACE_LOGGER_DEF_FN="$(cygpath -m "$WORK/outside-default.json")" \
RETRACE_LOGGER_DEF_STDOUT_ENA=0 \
RETRACE_LOGGER_FMT=jsonl \
  "$WIN_RUN" "$(cygpath -m "$WORK/win-subject.exe")" "$SECRET_MIXED" \
  > "$WORK/probeC.stdout" 2> "$WORK/probeC.stderr" || rcC=$?
rcD=0
RETRACE_V2_LIB="$(cygpath -m "$RT_DLL")" \
RETRACE_LOGGER_DEF_FN="$(cygpath -m "$WORK/outside-noconf.json")" \
RETRACE_LOGGER_DEF_STDOUT_ENA=0 \
RETRACE_LOGGER_FMT=jsonl \
RETRACE_LOGGER_RING=0 \
  "$WIN_RUN" "$(cygpath -m "$WORK/win-subject.exe")" "$SECRET_MIXED" \
  > "$WORK/probeD.stdout" 2> "$WORK/probeD.stderr" || rcD=$?
echo "windows-libc: probe ladder: A=standalone-ok B(ring0)=rc$rcB C(default)=rc$rcC D(ring0-noconf)=rc$rcD" >&2

# Drift detection — the pinned v2.14.0 signature is: injection fails,
# "failed to launch" with the hook-install diagnostics present, and the
# subject never printed. C must match it exactly; B and D may differ
# only by rc (their signature is still the failure kind). A surprise
# SUCCESS (rc 0 with the subject printing) means the chain works in a
# shape this script did not certify — drift, fail loudly for review.
sig() {  # sig <stderr-file> <rc>: 0 when the pinned broken signature holds
  [ "$2" != 0 ] && grep -q 'failed to launch' "$1" \
    && grep -q "hooked 'fopen'" "$1" && ! grep -qx 'VFS-SECRET-E2E' "${1%.stderr}.stdout"
}
sig "$WORK/probeC.stderr" "$rcC" \
  || fail "the round-4 failure signature DRIFTED (rcC=$rcC) — re-derive the pin from the artifacts: $(tail -3 "$WORK/probeC.stderr")"
if [ "$rcC" = 0 ] || [ "$rcD" = 0 ]; then
  fail "an uncertified probe shape SUCCEEDED (C=$rcC D=$rcD) — the prerequisite may have landed in a shape this script did not certify; re-derive the gates from the artifacts"
fi

cp "$INSIDE" "$OUTSIDE" "$WORK"/standalone.* "$WORK"/probe[BCD].* "$WORK"/*.json "$ART/" 2>/dev/null || :
echo "windows-libc: PREREQUISITE ABSENT at retrace v2.14.0 — live ucrt injection fails inside the child's engine boot after hook installation (run 32453723170 job 96686997021; probes B/C/D evidence in $ART)"
echo "windows-libc: the failure is UPSTREAM's (the standalone control passed; spec 25 §6.4 carries the prerequisite row); the procmon-converter half carries the windows chain certification"
echo "windows-libc: leg passes on the documented-prerequisite shape — drift in either direction fails it"
