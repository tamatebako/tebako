#!/bin/bash
# stage.sh — stage the four release binaries for one platform, strip
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

# Tool list: binary names equal the artifact tool names (no mapping
# table — this script must also run under bash 3.2 / POSIX sh, which
# have no `declare -A`).
TOOLS="tebako-bootstrap tfs tebako-pkg tebako"

mkdir -p out "fragments/frag-$PLATFORM"

echo "| platform | binary | size (bytes) |" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
echo "|---|---|---|" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"

for tool in $TOOLS; do
  src="target/${TARGET}/release/${tool}${EXE}"
  test -x "$src" || { echo "missing build output: $src"; exit 1; }
  dest="out/${tool}-${VERSION}-${PLATFORM}${EXE}"
  cp "$src" "$dest"
  if [ "$(uname -s)" = "Darwin" ]; then
    strip -S "$dest" 2>/dev/null || true
  else
    strip "$dest" 2>/dev/null || true
  fi
  size=$(stat -c %s "$dest" 2>/dev/null || stat -f %z "$dest")
  echo "| $PLATFORM | $tool | $size |" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
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
