#!/bin/sh
# ship-gate.sh — the release ship gate. A binary must FAIL ITS OWN
# RELEASE PIPELINE, never ship broken (the 0.1.1 lesson: the windows
# exes imported libstdc++-6.dll/libwinpthread-1.dll, the musl exes
# NEEDED libstdc++.so.6 — both exit 127 before main on a vanilla box,
# and build-success was the only gate).
#
#   ship-gate.sh <platform> <binary> [<binary> ...]
#
# Runs on the STAGED out/ artifacts (the exact shipped bytes), before
# any upload step of the leg. Two phases per binary:
#
#   SMOKE — a bare no-op launch (--version / --help; tebako-bootstrap
#   has no flags and must answer with its named bare-trailer error,
#   exit 65) in the CLEANEST environment the leg allows:
#     windows : PATH scrubbed to System32+Windows — no msys2/ucrt64/Git
#               bin dirs, because no user machine has them
#     linux   : the workflow runs this script inside a PRISTINE
#               ubuntu:20.04 (gnu) / alpine:3.21 (musl) container — the
#               exact floor the leg claims to target, with none of the
#               build container's apt/apk additions
#     macos   : plain launch on the runner
#   A process that dies before main (missing DLL/dylib: exit 127,
#   STATUS_DLL_NOT_FOUND, ld.so "not found") never produces the expected
#   no-op result, so the expected exit code is the whole assertion.
#
#   AUDIT — the dynamic-dependency whitelist, pinned from the actual
#   dumps of the shipped artifacts (0.1.0/0.1.1) and the audience law
#   (a user box has the OS base, never a toolchain):
#     windows-*    PE imports must be Windows-inbox — delegated to
#                  ci/windows-gnu-import-gate.sh (the same gate the
#                  windows-gnu build legs run; libstdc++-6.dll,
#                  libwinpthread-1.dll, libgcc_s* = FAIL)
#     linux-gnu-*  ldd may list ONLY the glibc surface:
#                  linux-vdso.so.1, ld-linux-{x86-64,aarch64}.so.2,
#                  libc.so.6, libm.so.6, libpthread.so.0, libdl.so.2,
#                  librt.so.1 — anything else (libstdc++.so.6,
#                  libgcc_s.so.1 included) = FAIL. The C++ runtime chain
#                  is absorbed statically by ci/linux-link-wrap.sh.
#     linux-musl-* ldd may list ONLY the musl libc:
#                  ld-musl-{x86_64,aarch64}.so.1,
#                  libc.musl-{x86_64,aarch64}.so.1 — anything else =
#                  FAIL. Dynamic-musl is the deliberate shape (see
#                  ci/musl-build.sh: musl libc is present on every musl
#                  system by definition); the toolchain runtimes are NOT
#                  part of a vanilla alpine and must be absorbed.
#     macos-*      otool -L entries must live under /usr/lib/ or
#                  /System/ (libSystem, libc++.1, libiconv, the
#                  Security/CoreFoundation frameworks — the actual
#                  0.1.1 set); @rpath/@executable_path/absolute
#                  toolchain paths = FAIL.
#
# POSIX sh: runs under bash (ubuntu/macos/windows legs) and busybox sh
# (the pristine alpine gate container).
set -eu
set -f

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <platform> <binary> [<binary> ...]" >&2
  exit 64
fi
PLATFORM=$1
shift

case $PLATFORM in
  windows-*|linux-gnu-*|linux-musl-*|macos-*) ;;
  *)
    echo "ship-gate: unknown platform family: $PLATFORM" >&2
    exit 64 ;;
esac

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/../../.." && pwd)
failures=0

note() { echo "ship-gate: $*"; }
bad() {
  echo "ship-gate: FAIL — $*" >&2
  failures=$((failures + 1))
}

# ---------------------------------------------------------------------
# smoke: one bare launch, expected exit code per tool
# ---------------------------------------------------------------------
# Returns the expected exit code for the tool named by the binary
# basename; the actual launch happens in smoke_one.
expected_rc() {
  case $1 in
    tebako-bootstrap*) echo 65 ;;  # named bare-trailer error (EX_TEBAKO_MANIFEST)
    # The spec-29 wrapper exe with no handoff answers its named
    # no-env-image error (spec 29 §2) — pinned: bare AND --version both
    # exit 65 (the interpreter lives in the env image; there is no
    # --version without one). Must precede the tebako-* catch-all.
    tebako-runtime-launcher*) echo 65 ;;
    tebako-shim*)      echo 0 ;;   # --help
    tebako-pkg*)       echo 0 ;;   # --help
    tebako-*)          echo 0 ;;   # --version
    tfs*)              echo 0 ;;   # --help
    *)
      echo "ship-gate: unknown tool name: $1" >&2
      return 64 ;;
  esac
}

smoke_args() {
  case $1 in
    tebako-bootstrap*) echo "" ;;
    tebako-runtime-launcher*) echo "" ;;
    tebako-shim*)      echo "--help" ;;
    tebako-pkg*)       echo "--help" ;;
    tebako-*)          echo "--version" ;;
    tfs*)              echo "--help" ;;
    *)                 echo "--help" ;;
  esac
}

smoke_one() {
  bin=$1
  base=$(basename "$bin")
  base=${base%.exe}
  want=$(expected_rc "$base") || exit 64
  args=$(smoke_args "$base")

  # tebako-shim dispatches on argv[0]'s basename: invoked under its
  # staged name (tebako-shim-<ver>-<platform>) it would try to dispatch
  # that "command" instead of entering management mode. Run the smoke
  # through a scratch copy carrying the real dispatcher name. (A copy,
  # not a symlink: Git-bash "symlinks" are text files to CreateProcess.)
  case $base in
    tebako-shim*)
      shimdir=$(mktemp -d "${TMPDIR:-/tmp}/tebako-ship-gate.XXXXXX")
      case $PLATFORM in
        windows-*) shimbin="$shimdir/tebako-shim.exe" ;;
        *)         shimbin="$shimdir/tebako-shim" ;;
      esac
      cp "$bin" "$shimbin"
      bin=$shimbin ;;
  esac

  note "smoke: $base ${args:-<no args>} (expect exit $want)"
  set +e
  if [ "${PLATFORM#windows-}" != "$PLATFORM" ]; then
    # A user machine has System32 and Windows on PATH — and nothing
    # else of ours: no msys2, no ucrt64, no Git bin dir. The msys2
    # runtime re-encodes PATH to windows form for the native child.
    # $args is a single flag word (or empty): deliberate word splitting.
    # shellcheck disable=SC2086
    out=$( PATH="/c/Windows/System32:/c/Windows"
           export PATH
           "$bin" $args </dev/null 2>&1 )
    rc=$?
  else
    # shellcheck disable=SC2086
    out=$("$bin" $args </dev/null 2>&1)
    rc=$?
  fi
  set -e
  printf '%s\n' "$out" | sed 's/^/    /' | head -5
  if [ "$rc" -ne "$want" ]; then
    bad "$base launched with exit $rc, expected $want — the process did not reach its own no-op path (missing runtime library?)"
  fi
}

# ---------------------------------------------------------------------
# audit: the per-family dependency whitelist
# ---------------------------------------------------------------------
gnu_whitelist() {
  case $1 in
    linux-vdso.so.1|ld-linux-x86-64.so.2|ld-linux-aarch64.so.1|\
    libc.so.6|libm.so.6|libpthread.so.0|libdl.so.2|librt.so.1) return 0 ;;
    *) return 1 ;;
  esac
}

musl_whitelist() {
  case $1 in
    ld-musl-x86_64.so.1|ld-musl-aarch64.so.1|\
    libc.musl-x86_64.so.1|libc.musl-aarch64.so.1) return 0 ;;
    *) return 1 ;;
  esac
}

audit_ldd() {
  bin=$1
  family=$2
  deps=$(ldd "$bin" 2>&1) || true
  if printf '%s\n' "$deps" | grep -qi 'not a dynamic executable\|not a valid dynamic program'; then
    note "audit: $(basename "$bin") is fully static — no dynamic deps to audit"
    return 0
  fi
  printf '%s\n' "$deps" | sed 's/^/    /'
  if printf '%s\n' "$deps" | grep -qi 'not found\|error loading\|error relocating'; then
    bad "$(basename "$bin"): ldd reports an unresolvable dependency"
    return 0
  fi
  # First field of each line is the soname (or the interpreter path).
  for dep in $(printf '%s\n' "$deps" | awk '{print $1}'); do
    soname=$(basename "$dep")
    if ! "${family}"_whitelist "$soname"; then
      bad "$(basename "$bin"): off-whitelist dependency: $soname"
    fi
  done
}

audit_macos() {
  bin=$1
  deps=$(otool -L "$bin" | tail -n +2 | awk '{print $1}')
  printf '%s\n' "$deps" | sed 's/^/    /'
  for dep in $deps; do
    case $dep in
      /usr/lib/*|/System/*) ;;
      *) bad "$(basename "$bin"): off-whitelist dylib: $dep" ;;
    esac
  done
}

note "platform $PLATFORM — gating $# binar$( [ "$#" -eq 1 ] && echo y || echo ies)"

for bin in "$@"; do
  [ -f "$bin" ] || { echo "ship-gate: no such file: $bin" >&2; exit 66; }
  note "=== $bin ==="
  smoke_one "$bin"
  case $PLATFORM in
    windows-*)
      # objdump-based PE import audit (the windows-gnu legs' own gate);
      # needs the leg's toolchain PATH, so it runs here, not in the
      # scrubbed smoke environment.
      if ! bash "$REPO_ROOT/ci/windows-gnu-import-gate.sh" "$bin"; then
        failures=$((failures + 1))
      fi ;;
    linux-gnu-*)  audit_ldd "$bin" gnu ;;
    linux-musl-*) audit_ldd "$bin" musl ;;
    macos-*)      audit_macos "$bin" ;;
  esac
done

if [ "$failures" -gt 0 ]; then
  echo "ship-gate: FAIL — $failures violation(s); a broken binary fails its own release pipeline, it never ships" >&2
  exit 1
fi
note "PASS ($# gated: clean-env smoke + dependency whitelist)"
