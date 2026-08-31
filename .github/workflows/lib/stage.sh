#!/bin/bash
# stage.sh — stage the six release binaries for one platform, strip
# them, publish the size table, gate the bootstrap size, and write the
# sha/size fragments the finalize job merges into SHA256SUMS +
# manifest.json.
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
# have no `declare -A`). tebako-shim is the dispatcher (TODO.testing/07).
# tebako-runtime-launcher is the spec-29 wrapper exe (the repacked-
# runtime ship form — tebako-packages/openjdk#23's download).
TOOLS="tebako-bootstrap tfs tebako-pkg tebako tebako-shim tebako-runtime-launcher"

mkdir -p out "fragments/frag-$PLATFORM"

echo "| platform | binary | size (bytes) |" >> "$SUMMARY"
echo "|---|---|---|" >> "$SUMMARY"

for tool in $TOOLS; do
  src="target/${TARGET}/release/${tool}${EXE}"
  # A leg that builds native WITHOUT --target (the musl container — its
  # host triple IS the target, and RUSTFLAGS must reach host units there)
  # lands in the default-target layout.
  if [ ! -x "$src" ] && [ -x "target/release/${tool}${EXE}" ]; then
    src="target/release/${tool}${EXE}"
  fi
  test -x "$src" || { echo "missing build output: target/${TARGET}/release/${tool}${EXE} (or the default-target layout)"; exit 1; }
  dest="out/${tool}-${VERSION}-${PLATFORM}${EXE}"
  cp "$src" "$dest"
  if [ "$(uname -s)" = "Darwin" ]; then
    strip -S "$dest" 2>/dev/null || true
  else
    strip "$dest" 2>/dev/null || true
  fi
  size=$(stat -c %s "$dest" 2>/dev/null || stat -f %z "$dest")
  echo "| $PLATFORM | $tool | $size |" >> "$SUMMARY"
  if command -v shasum >/dev/null 2>&1; then
    sha256=$(shasum -a 256 "$dest" | cut -d' ' -f1)
  else
    sha256=$(sha256sum "$dest" | cut -d' ' -f1)
  fi
  echo "$sha256" > "fragments/frag-$PLATFORM/${tool}-${PLATFORM}.sha256"
  echo "$size" > "fragments/frag-$PLATFORM/${tool}-${PLATFORM}.size"

  if [ "$tool" = "tebako-bootstrap" ] && [ "$size" -ge "$BUDGET" ]; then
    echo "::error::tebako-bootstrap ($PLATFORM) is $size bytes — over the $BUDGET budget (item 22's extended gate)"
    exit 1
  fi
done
echo "staged for $PLATFORM:"
ls -la out/
