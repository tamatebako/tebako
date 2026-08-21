#!/usr/bin/env bash
# ci/trace-dogfood/windows-libc.sh — the spec 25 §8-T3 libc-layer dogfood
# on windows. There is no LD_PRELOAD on windows: retrace's inline-hook
# backend (ADR-0009) injects retrace.dll at process creation
# (retrace-win-run: CreateProcess suspended → CreateRemoteThread(
# LoadLibrary) → hooks install in DLL_PROCESS_ATTACH → resume), hooking
# the subject's ucrt file traffic (fopen/_open/_stat/... — the
# preload-mingw backend for a ucrt64-built subject).
#
# HONEST SCOPE (read before extending): no tebako interposition ships on
# windows today (libtfs-preload is POSIX; the spec 25 bus rides the
# runtime driver, and windows runtimes are the factory's story). The
# inside stream is therefore the EMPTY capture by construction, and every
# under-prefix touch is a reported escape. This leg proves the CHAIN —
# retrace's windows capture → the tebako correlator → a well-formed
# escapes report that names its producing layer — and gates on exactly
# that honesty (§6.1): exit 1, the subject's file named as an escape,
# the libc boundary named with its UNCERTIFIABLE sub-libc note. It never
# asserts coverage.
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

fail() { echo "windows-libc: FAIL: $*" >&2; exit 1; }

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

# 2. The capture: inject + run. retrace.dll is a native binary — env
#    values ride in windows form; the empty inside stream is pre-created.
INSIDE="$WORK/inside.jsonl"
OUTSIDE="$WORK/outside.json"
touch "$INSIDE" "$OUTSIDE"
rc=0
RETRACE_V2_LIB="$(cygpath -m "$RT_DLL")" \
RETRACE_LOGGER_DEF_FN="$(cygpath -m "$OUTSIDE")" \
RETRACE_LOGGER_DEF_STDOUT_ENA=0 \
RETRACE_LOGGER_FMT=jsonl \
  "$WIN_RUN" "$(cygpath -m "$WORK/win-subject.exe")" "$SECRET_MIXED" \
  > "$WORK/subject.stdout" 2> "$WORK/subject.stderr" || rc=$?
[ "$rc" = 0 ] || { cat "$WORK/subject.stderr" >&2; fail "retrace-win-run rc=$rc"; }
grep -qx 'VFS-SECRET-E2E' "$WORK/subject.stdout" \
  || fail "the subject did not print the file: $(cat "$WORK/subject.stdout")"

# 3. Non-vacuity: the ucrt hooks really fired on the subject's traffic.
grep -q 'secret.txt' "$OUTSIDE" \
  || fail "the outside capture never saw the subject's file traffic (vacuous leg): $(cat "$WORK/subject.stderr")"

# 4. cover: every under-prefix touch escapes by construction (see the
#    HONEST SCOPE note); the gate is the report's shape and honesty.
rc=0
"$TEBAKO" trace cover --inside "$INSIDE" --outside "$OUTSIDE" --prefix "$VFS_MIXED" \
  --layer libc > "$ART/cover.stdout" 2> "$ART/cover.stderr" || rc=$?
[ "$rc" = 1 ] || { cat "$ART/cover.stdout" "$ART/cover.stderr" >&2; fail "cover rc=$rc — with an empty inside stream the touches MUST escape"; }
grep -q '^escape .*secret\.txt .*class=read' "$ART/cover.stdout" \
  || fail "the escapes report does not name the subject's file: $(cat "$ART/cover.stdout")"
grep -q 'outside capture layer:.*libc boundary' "$ART/cover.stderr" \
  || fail "the report does not name the libc boundary (§6.1)"
grep -q 'UNCERTIFIABLE' "$ART/cover.stderr" \
  || fail "the report does not carry the sub-libc UNCERTIFIABLE note (§7 honesty)"

cp "$INSIDE" "$OUTSIDE" "$WORK/subject.stdout" "$WORK/subject.stderr" "$ART/" 2>/dev/null || :
echo "windows-libc: the chain held — $(grep -c '^escape ' "$ART/cover.stdout") escape(s) reported, layer named"
echo "windows-libc: escapes report + captures in $ART"
