#! /bin/bash
#
# Copyright (c) 2026, [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of the Tebako project.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
# 1. Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
# 2. Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
# TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR CONTRIBUTORS
# BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
# CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
# SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
# INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
# CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
# ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
# POSSIBILITY OF SUCH DAMAGE.

# ......................................................................
# Press modes end-to-end test (Stage 3B-3): lean (default) / fat / classic.
#
# Verifies the full three-part flow on the local machine:
#   1. lean press (the default mode) produces a bootstrap-based package whose
#      first run downloads the referenced runtime into a FRESH cache exactly
#      once and runs the app (memfs content + argv passthrough proven);
#   2. a second lean app (different image) with the same cache reuses the
#      cached runtime -- proven by removing the mirror AND going offline;
#   3. a lean package on a fresh cache with TEBAKO_OFFLINE=1 fails cleanly
#      (exit 69, names the knobs);
#   4. a fat package (runtime payload slot) runs on a fresh cache with
#      TEBAKO_OFFLINE=1 -- the payload installs itself, no network at all;
#   5. a classic package (prebuilt runtime + images, Stage-3A layout) runs
#      self-contained and never touches the cache.
#
# The runtime package is pressed from THIS repo's sources ('-m runtime'), so
# it carries this tree's tebako-main (launcher ABI v1 + TPKG_FORMAT_RUNTIME
# tolerance); it is served to the bootstrap through a local file mirror,
# because no tebako-runtime-ruby release with the launcher ABI exists yet
# (v0.15.0 packages are pending -- once they ship, TEBAKO_RUNTIME_MIRROR can
# simply be unset to exercise the real release). The tebako-bootstrap
# launcher (>= 0.2.0, payload install) is built from a sibling checkout and
# likewise served through a local mirror.
#
# Usage:
#   tests/scripts/press-modes-tests.sh
# Requires 'exe/tebako setup -R $RUBY_VER' to have completed in this repo
# (deps/ provisioned), cmake + a C compiler (for the bootstrap build), and a
# tebako-bootstrap checkout with payload support at ../tebako-bootstrap
# (override with TEBAKO_BOOTSTRAP_REPO).

# ......................................................................
# Helpers

size_of() {
   wc -c < "$1" | tr -d ' '
}

sha256_of() {
   if command -v sha256sum >/dev/null 2>&1; then
      sha256sum "$1" | cut -d' ' -f1 | sed 's/^\\//'
   else
      shasum -a 256 "$1" | cut -d' ' -f1 | sed 's/^\\//'
   fi
}

# Number of runtime entries in a cache root
runtime_entries() {
   if [ -d "$1/runtimes" ]; then
      ls "$1/runtimes" | wc -l | tr -d ' '
   else
      echo 0
   fi
}

# Run a package expecting success.
# $1 -- expected output substring; remaining args -- the invocation
package_runner_ok() {
   local expected="$1"; shift
   result="$( env "$@" 2>&1 )"
   assertEquals 0 $?
   assertContains "$result" "$expected"
}

# Press one fixture app; $1 -- mode ("" = default), $2 -- app dir, $3 -- output
press_app() {
   local mode="$1" app="$2" out="$3" mode_arg=""
   [ -n "$mode" ] && mode_arg="--mode=$mode"
   ( cd "${DIR_ROOT}" && env TEBAKO_HOME="${PRESS_HOME}" \
        TEBAKO_BOOTSTRAP_MIRROR="file://${BOOTSTRAP_MIRROR}" \
        TEBAKO_BOOTSTRAP_VERSION="${BOOTSTRAP_VER}" \
        TEBAKO_RUNTIME_MIRROR="file://${RUNTIME_MIRROR}" \
        "${DIR_BIN}/tebako" press -D -R "${RUBY_VER}" ${mode_arg} \
        --root="${WORK}/${app}" --entry-point=app.rb --output="$out" \
        > "${WORK}/press-$(basename "$out").log" 2>&1 ) || {
      cat "${WORK}/press-$(basename "$out").log"
      exit 1
   }
   [ -f "$out" ] || { echo "press produced no package: $out"; exit 1; }
}

# ......................................................................
# Tests

# lean, first run on a fresh cache: the runtime is downloaded once, the app
# runs from the mounted image and receives its argv
test_lean_first_run_downloads_runtime_once() {
   echo "==> lean first run downloads the runtime exactly once"
   package_runner_ok "press-modes: app1 ok" \
      TEBAKO_HOME="${HOME_RUN}" TEBAKO_RUNTIME_MIRROR="file://${RUNTIME_MIRROR}" \
      "${LEAN1}" one two
   assertContains "$result" "press-modes: memfs: APP1-DATA-CONTENT"
   assertContains "$result" 'press-modes: argv: ["one", "two"]'
   assertEquals "1" "$(runtime_entries "${HOME_RUN}")"
   assertTrue "runtime executable installed" \
      "[ -f \"${HOME_RUN}/runtimes/ruby-${RUBY_VER}-${TEBAKO_VER}-${PLAT}/tebako-runtime-${TEBAKO_VER}-${RUBY_VER}-${PLAT}\" ]"
}

# second lean app, same cache: no second download -- the mirror is removed
# and TEBAKO_OFFLINE=1 is set, so a download attempt would fail the run
test_second_lean_app_uses_cached_runtime() {
   echo "==> second lean app shares the cached runtime (mirror gone, offline)"
   mv "${RUNTIME_MIRROR}" "${WORK}/runtime-mirror-gone"
   package_runner_ok "press-modes: app2 ok" \
      TEBAKO_OFFLINE=1 TEBAKO_HOME="${HOME_RUN}" "${LEAN2}"
   assertContains "$result" "press-modes: memfs: APP2-DATA-CONTENT"
   assertEquals "1" "$(runtime_entries "${HOME_RUN}")"
}

# lean on a fresh cache with TEBAKO_OFFLINE=1: clean bootstrap error (69)
test_lean_offline_miss_fails_cleanly() {
   echo "==> lean offline miss exits 69 naming the knobs"
   result="$( env TEBAKO_OFFLINE=1 TEBAKO_HOME="${WORK}/home-miss" "${LEAN1}" 2>&1 )"
   assertEquals 69 $?
   assertContains "$result" "TEBAKO_OFFLINE"
   assertContains "$result" "TEBAKO_RUNTIME_MIRROR"
}

# fat on a fresh cache with TEBAKO_OFFLINE=1: the payload slot installs
test_fat_runs_offline_on_fresh_cache() {
   echo "==> fat runs offline on a fresh cache (payload install)"
   package_runner_ok "press-modes: app3 ok" \
      TEBAKO_OFFLINE=1 TEBAKO_HOME="${HOME_FAT}" "${FATPKG}"
   assertContains "$result" "press-modes: memfs: APP3-DATA-CONTENT"
   assertEquals "1" "$(runtime_entries "${HOME_FAT}")"
   ENTRY_DIR="${HOME_FAT}/runtimes/ruby-${RUBY_VER}-${TEBAKO_VER}-${PLAT}"
   assertTrue "payload provenance recorded" "[ -f \"${ENTRY_DIR}/origin\" ] && grep -q '^payload=' \"${ENTRY_DIR}/origin\""
}

# classic (Stage-3A stitched): self-contained, never touches the cache
test_classic_runs_self_contained() {
   echo "==> classic runs self-contained without cache access"
   package_runner_ok "press-modes: app4 ok" \
      TEBAKO_OFFLINE=1 TEBAKO_HOME="${WORK}/home-classic" "${CLASSICPKG}"
   assertContains "$result" "press-modes: memfs: APP4-DATA-CONTENT"
   assertEquals "0" "$(runtime_entries "${WORK}/home-classic")"
}

# ......................................................................
# Fixture build (once): runtime mirror, bootstrap mirror, four pressed apps
oneTimeSetUp() {
   WORK="$( mktemp -d "${TMPDIR:-/tmp}/tebako-press-modes.XXXXXX" )"
   echo "==> building press-modes fixtures in ${WORK}"
   (
      set -e
      cd "${WORK}"

      TEBAKO_VER=$( ruby -I "${DIR_ROOT}/lib" -rtebako/version -e 'print Tebako::VERSION' )
      echo "==> tebako ${TEBAKO_VER}, ruby ${RUBY_VER}, platform ${PLAT}"

      # -- runtime mirror: pressed from this repo's sources (launcher ABI v1)
      RT_ASSET="tebako-runtime-${TEBAKO_VER}-${RUBY_VER}-${PLAT}"
      RUNTIME_MIRROR="${WORK}/runtime-mirror"
      mkdir -p "${RUNTIME_MIRROR}/v${TEBAKO_VER}"
      echo "==> pressing the runtime package (this can take a few minutes)"
      ( cd "${DIR_ROOT}" && "${DIR_BIN}/tebako" press -D -R "${RUBY_VER}" -m runtime \
         --output "${RUNTIME_MIRROR}/v${TEBAKO_VER}/${RT_ASSET}" \
         > "${WORK}/press-runtime.log" 2>&1 ) || { cat "${WORK}/press-runtime.log"; exit 1; }
      [ -f "${RUNTIME_MIRROR}/v${TEBAKO_VER}/${RT_ASSET}" ] || { echo "no runtime package"; exit 1; }
      RT_SHA=$( sha256_of "${RUNTIME_MIRROR}/v${TEBAKO_VER}/${RT_ASSET}" )
      cat > "${RUNTIME_MIRROR}/v${TEBAKO_VER}/manifest.json" <<EOF
[
  {
    "tebako_version": "${TEBAKO_VER}",
    "ruby_version": "${RUBY_VER}",
    "platform": "${PLAT}",
    "filename": "${RT_ASSET}",
    "sha256": "${RT_SHA}",
    "size_bytes": $( size_of "${RUNTIME_MIRROR}/v${TEBAKO_VER}/${RT_ASSET}" )
  }
]
EOF
      echo "${RT_SHA}  ${RT_ASSET}" > "${RUNTIME_MIRROR}/v${TEBAKO_VER}/SHA256SUMS.txt"

      # -- bootstrap mirror: built from the sibling checkout (>= 0.2.0)
      BOOTSTRAP_VER="0.2.0"
      BS_ASSET="tebako-bootstrap-${BOOTSTRAP_VER}-${PLAT}"
      BOOTSTRAP_MIRROR="${WORK}/bootstrap-mirror"
      cmake -S "${TEBAKO_BOOTSTRAP_REPO}" -B "${WORK}/tbs-build" -DCMAKE_BUILD_TYPE=Release \
         > "${WORK}/tbs-configure.log" 2>&1 || { cat "${WORK}/tbs-configure.log"; exit 1; }
      cmake --build "${WORK}/tbs-build" --target tebako-bootstrap --parallel \
         > "${WORK}/tbs-build.log" 2>&1 || { cat "${WORK}/tbs-build.log"; exit 1; }
      mkdir -p "${BOOTSTRAP_MIRROR}/v${BOOTSTRAP_VER}"
      cp "${WORK}/tbs-build/tebako-bootstrap" "${BOOTSTRAP_MIRROR}/v${BOOTSTRAP_VER}/${BS_ASSET}"
      BS_SHA=$( sha256_of "${BOOTSTRAP_MIRROR}/v${BOOTSTRAP_VER}/${BS_ASSET}" )
      cat > "${BOOTSTRAP_MIRROR}/v${BOOTSTRAP_VER}/manifest.json" <<EOF
{
  "name": "tebako-bootstrap",
  "version": "${BOOTSTRAP_VER}",
  "assets": [
    { "platform": "${PLAT}", "file": "${BS_ASSET}", "sha256": "${BS_SHA}" }
  ]
}
EOF
      echo "${BS_SHA}  ${BS_ASSET}" > "${BOOTSTRAP_MIRROR}/v${BOOTSTRAP_VER}/SHA256SUMS"

      # -- fixture applications (distinct content + data files)
      for app in app1 app2 app3 app4; do
         mkdir "${app}"
         tag=$( echo "${app}" | tr 'a-z' 'A-Z' )
         cat > "${app}/app.rb" <<RUBY
puts "press-modes: ${app} ok"
puts "press-modes: memfs: #{File.read('/__tebako_memfs__/local/data.txt').strip}"
puts "press-modes: argv: #{ARGV.inspect}"
RUBY
         echo "${tag}-DATA-CONTENT" > "${app}/data.txt"
      done

      # -- press lean (default) x2, fat, classic
      PRESS_HOME="${WORK}/press-home"
      press_app ""       app1 "${WORK}/lean1"
      press_app ""       app2 "${WORK}/lean2"
      press_app "fat"    app3 "${WORK}/fat"
      press_app "classic" app4 "${WORK}/classic"
   ) || { echo "fixture build failed"; exit 1; }

   TEBAKO_VER=$( ruby -I "${DIR_ROOT}/lib" -rtebako/version -e 'print Tebako::VERSION' )
   BOOTSTRAP_VER="0.2.0"
   RUNTIME_MIRROR="${WORK}/runtime-mirror"
   LEAN1="${WORK}/lean1"
   LEAN2="${WORK}/lean2"
   FATPKG="${WORK}/fat"
   CLASSICPKG="${WORK}/classic"
   HOME_RUN="${WORK}/home-run"
   HOME_FAT="${WORK}/home-fat"
}

oneTimeTearDown() {
   [ -n "${WORK}" ] && [ "${KEEP_PRESS_MODES_WORK}" != "yes" ] && rm -rf "${WORK}"
   return 0
}

# ......................................................................
# main

DIR0=$( dirname "$0" )
DIR_ROOT=$( cd "$DIR0"/../.. && pwd )
DIR_BIN=$( cd "$DIR_ROOT"/exe && pwd )
DIR_TESTS=$( cd "$DIR_ROOT"/tests && pwd )
RUBY_VER=${RUBY_VER:-3.3.7}
TEBAKO_BOOTSTRAP_REPO=${TEBAKO_BOOTSTRAP_REPO:-$( cd "$DIR_ROOT/.." && pwd )/tebako-bootstrap}

# platform id, as used by tebako-runtime-ruby/tebako-bootstrap asset names
UNAME_S=$( uname -s )
UNAME_M=$( uname -m )
case "$UNAME_S" in
   Darwin)
      case "$UNAME_M" in
         arm64 | aarch64) PLAT=macos-arm64 ;;
         x86_64)          PLAT=macos-x86_64 ;;
         *) echo "ERROR: unsupported arch: $UNAME_M"; exit 1 ;;
      esac
      ;;
   Linux)
      case "$UNAME_M" in
         x86_64 | amd64) MACHINE=x86_64 ;;
         aarch64 | arm64) MACHINE=arm64 ;;
         *) echo "ERROR: unsupported arch: $UNAME_M"; exit 1 ;;
      esac
      if ldd --version 2>&1 | grep -qi musl; then PLAT="linux-musl-$MACHINE"; else PLAT="linux-gnu-$MACHINE"; fi
      ;;
   MINGW* | MSYS* | CYGWIN* | Windows_NT) PLAT=windows-x86_64 ;;
   *) echo "ERROR: unsupported OS: $UNAME_S"; exit 1 ;;
esac

if [ ! -x "${DIR_ROOT}/deps/bin/mkdwarfs" ]; then
   echo "ERROR: no provisioned mkdwarfs at ${DIR_ROOT}/deps/bin/mkdwarfs --"
   echo "       run 'exe/tebako setup -R ${RUBY_VER}' from the repo root first"
   exit 1
fi

if ! command -v cmake > /dev/null 2>&1 || ! command -v cc > /dev/null 2>&1; then
   echo "ERROR: cmake and a C compiler are required to build tebako-bootstrap"
   exit 1
fi

if [ ! -f "${TEBAKO_BOOTSTRAP_REPO}/src/tebako-bootstrap.c" ]; then
   echo "ERROR: no tebako-bootstrap checkout at ${TEBAKO_BOOTSTRAP_REPO} --"
   echo "       clone it there or set TEBAKO_BOOTSTRAP_REPO (payload support >= 0.2.0 required)"
   exit 1
fi

if ! grep -q "TPKG_FORMAT_RUNTIME" "${TEBAKO_BOOTSTRAP_REPO}/include/tebako/tpkg.h"; then
   echo "ERROR: ${TEBAKO_BOOTSTRAP_REPO} predates the runtime payload support (>= 0.2.0 required)"
   exit 1
fi

echo "Running press-modes tests for Ruby ${RUBY_VER} on ${PLAT}"

# shellcheck source=/dev/null
. "${DIR_TESTS}/shunit2/shunit2"
