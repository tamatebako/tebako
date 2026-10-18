#!/usr/bin/env bash
# common.sh — the shared harness of the probe scenarios
# (tools/probes). Each scenario script sources this file and receives:
#
#   fetch_tools <release-tag>   the pinned tebako tool + its release's
#                               SHA256SUMS cross-check, into $PWD/tools/
#   devkit_path                 the Ruby DevKit's ucrt64/bin (gcc, nm,
#                               ar, make, objdump, dlltool) and usr/bin
#                               (sh.exe, bash.exe) — prepended to PATH by
#                               with_devkit
#   press_app <root-dir>        press the app at <root-dir> with the
#                               fetched tool; prints the exit code and
#                               leaves press.out / press.err behind
#
# The probes exist to turn a release-cycle diagnosis (tag → release →
# leg, roughly two hours) into a five-minute runner experiment: the
# pattern located twelve defect layers in the rails-on-windows chain
# (tebako#753) that release cycles alone could not have separated.
set -euo pipefail

probe_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

fetch_tools() {
  local rel="$1"
  local ver="${rel#v}"
  mkdir -p tools && cd tools
  local base="https://github.com/tamatebako/tebako/releases/download/$rel"
  curl -fsSLO --retry 5 --retry-all-errors --retry-delay 3 "$base/tebako-$ver-windows-ucrt64.exe"
  curl -fsSLO --retry 5 --retry-all-errors --retry-delay 3 "$base/SHA256SUMS"
  grep "tebako-$ver-windows-ucrt64.exe\$" SHA256SUMS | sha256sum -c -
  mv "tebako-$ver-windows-ucrt64.exe" tebako.exe
  cd ..
}

devkit_path() {
  local root
  root="$(cygpath "$RI_DEVKIT")"
  echo "$root/ucrt64/bin"
}

with_devkit() {
  export PATH="$(devkit_path):$PATH"
}

press_app() {
  local root="$1"
  (
    cd "$root"
    set +e
    "$probe_root/tools/tebako.exe" press \
      --root . --entry-point main.rb --Ruby "${PROBE_RUBY:-3.4.10}" \
      --mode self-contained --output dist/app > press.out 2> press.err
    echo "press rc=$?"
    set -e
    grep -iE "error|failed|undefined" press.out | head -6 || true
  )
}
