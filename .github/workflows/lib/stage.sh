#!/bin/bash
# stage.sh — stage the six release binaries for one platform, strip
# them, sign them (macOS only, spec 31 §5: BEFORE the digests below —
# sign-then-hash is mandatory), publish the size table, gate the
# bootstrap size, and write the sha/size fragments the finalize job
# merges into SHA256SUMS + manifest.json. POSIX legs also stage the
# preload shim cdylib (tebako#1099 — the install lanes place it beside
# the CLI, where `tfs exec` looks).
#
# Required env: VERSION (release version, tag minus v), PLATFORM (the
# tebako platform id, e.g. macos-arm64). Optional: TARGET (defaults to
# host), BOOTSTRAP_SIZE_BUDGET (default 3145728), EXE_SUFFIX.
set -euo pipefail

BUDGET="${BOOTSTRAP_SIZE_BUDGET:-3145728}"
TARGET="${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
EXE="${EXE_SUFFIX:-}"

# The docker legs (floor/musl) inherit GITHUB_STEP_SUMMARY by name, but
# the runner's file-command directory is outside the mounted workspace —
# the append would die under set -e (run 30748661257, both musl legs
# after full green builds). Fall back to /dev/null when it is not there.
SUMMARY="${GITHUB_STEP_SUMMARY:-/dev/null}"
[ -d "$(dirname "$SUMMARY")" ] || SUMMARY=/dev/null

# Tool list: binary names equal the artifact tool names (no mapping
# table — this script must also run under bash 3.2 / POSIX sh, which
# have no `declare -A`). tebako-shim is the dispatcher.
# tebako-runtime-launcher is the spec-29 wrapper exe (the repacked-
# runtime ship form — tamatebako/tebako-runtime-openjdk#23's download).
TOOLS="tebako-bootstrap tfs tebako-pkg tebako tebako-shim tebako-runtime-launcher"

mkdir -p out "fragments/frag-$PLATFORM"

echo "| platform | binary | size (bytes) |" >> "$SUMMARY"
echo "|---|---|---|" >> "$SUMMARY"

# stage_one <build-output file> <staged name> <fragment stem>
# Copies from the target dir (either layout), strips, signs (macOS,
# armed legs only — sign-then-hash: the fragment anchors the SIGNED
# bytes), and writes the .sha256/.size fragments.
stage_one() {
  src="target/${TARGET}/release/$1"
  # A leg that builds native WITHOUT --target (the musl container — its
  # host triple IS the target, and RUSTFLAGS must reach host units there)
  # lands in the default-target layout.
  if [ ! -x "$src" ] && [ -x "target/release/$1" ]; then
    src="target/release/$1"
  fi
  test -x "$src" || { echo "missing build output: target/${TARGET}/release/$1 (or the default-target layout)"; exit 1; }
  dest="out/$2"
  cp "$src" "$dest"
  if [ "$(uname -s)" = "Darwin" ]; then
    strip -S "$dest" 2>/dev/null || true
    # Spec 31 §5: sign the final bytes BEFORE the sha256 fragment is
    # written below — SHA256SUMS/sidecars must anchor the SIGNED bytes.
    # Loud no-op when the leg's enablement gate is disarmed.
    bash ci/macos-sign-one.sh "$dest"
  else
    strip "$dest" 2>/dev/null || true
  fi
  size=$(stat -c %s "$dest" 2>/dev/null || stat -f %z "$dest")
  echo "| $PLATFORM | $3 | $size |" >> "$SUMMARY"
  if command -v shasum >/dev/null 2>&1; then
    sha256=$(shasum -a 256 "$dest" | cut -d' ' -f1)
  else
    sha256=$(sha256sum "$dest" | cut -d' ' -f1)
  fi
  echo "$sha256" > "fragments/frag-$PLATFORM/$3.sha256"
  echo "$size" > "fragments/frag-$PLATFORM/$3.size"
}

for tool in $TOOLS; do
  stage_one "${tool}${EXE}" "${tool}-${VERSION}-${PLATFORM}${EXE}" "${tool}-${PLATFORM}"
  size=$(cat "fragments/frag-$PLATFORM/${tool}-${PLATFORM}.size")
  if [ "$tool" = "tebako-bootstrap" ] && [ "$size" -ge "$BUDGET" ]; then
    echo "::error::tebako-bootstrap ($PLATFORM) is $size bytes — over the $BUDGET budget (item 22's extended gate)"
    exit 1
  fi
done

# The preload shim cdylib (tebako#1099): POSIX legs only — windows has
# no LD_PRELOAD/DYLD_INSERT_LIBRARIES delivery (`tfs exec` is unix-only;
# the windows CLI says so with its own named error). The staged name
# carries the library extension by design: the ship gate's
# `out/*-$VERSION-$PLATFORM` binary glob then never tries to EXECUTE a
# library (the leg hands it to the gate explicitly, for the dependency
# audit — a shim injected into arbitrary user processes is held to the
# same closure whitelist as the binaries).
PRELOAD_LIB=""
case "$PLATFORM" in
  macos-*) PRELOAD_LIB="libtfs_preload.dylib" ;;
  linux-*) PRELOAD_LIB="libtfs_preload.so" ;;
esac
if [ -n "$PRELOAD_LIB" ]; then
  stage_one "$PRELOAD_LIB" "libtfs_preload-${VERSION}-${PLATFORM}${PRELOAD_LIB#libtfs_preload}" "libtfs_preload-${PLATFORM}"
fi
echo "staged for $PLATFORM:"
ls -la out/
