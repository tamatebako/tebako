#!/bin/bash
# finalize.sh — merge the per-platform fragments into SHA256SUMS and
# manifest.json (the shape tebako-bootstrap releases use today, extended
# with a per-tool `tools` map), and append the size table to the release
# notes. Runs in the finalize job after actions/download-artifact has
# laid every frag-<platform> directory flat into ./fragments.
#
# Required env: VERSION (tag minus v), TAG (release tag).
set -euo pipefail

mkdir -p out
FRAG=fragments

# ---- SHA256SUMS (sorted, "<sha>  <file>") ------------------------------
: > out/SHA256SUMS
for platform_dir in "$FRAG"/frag-*; do
  platform="${platform_dir##*/frag-}"
  for tool in tebako-bootstrap tfs tebako-pkg tebako; do
    sha=$(cat "$platform_dir/${tool}-${platform}.sha256")
    echo "$sha  ${tool}-${VERSION}-${platform}" >> out/SHA256SUMS
  done
done
sort -k2 -o out/SHA256SUMS out/SHA256SUMS

# ---- manifest.json ------------------------------------------------------
# {
#   "name": "tebako-rs",
#   "version": "<VERSION>",
#   "assets": [ {platform, file, sha256, size_bytes}, ... ],   # tebako-bootstrap
#   "tools": { "tfs": [...], "tebako-pkg": [...], "tebako": [...] }
# }
# The `assets` array is exactly the BootstrapManager's consumed shape, so
# the gem/tebako-cli resolve the Rust bootstrap unchanged.
jq_entries() {
  local tool="$1"
  for platform_dir in "$FRAG"/frag-*; do
    platform="${platform_dir##*/frag-}"
    [ -f "$platform_dir/${tool}-${platform}.sha256" ] || continue
    sha=$(cat "$platform_dir/${tool}-${platform}.sha256")
    size=$(cat "$platform_dir/${tool}-${platform}.size")
    jq -cn \
      --arg platform "$platform" \
      --arg file "${tool}-${VERSION}-${platform}" \
      --arg sha256 "$sha" \
      --argjson size_bytes "$size" \
      '{platform: $platform, file: $file, sha256: $sha256, size_bytes: $size_bytes}'
  done
}

bootstrap_json=$(jq_entries tebako-bootstrap | jq -s 'sort_by(.platform)')
tfs_json=$(jq_entries tfs | jq -s 'sort_by(.platform)')
pkg_json=$(jq_entries tebako-pkg | jq -s 'sort_by(.platform)')
cli_json=$(jq_entries tebako | jq -s 'sort_by(.platform)')

jq -n \
  --arg version "$VERSION" \
  --argjson assets "$bootstrap_json" \
  --argjson tfs "$tfs_json" \
  --argjson pkg "$pkg_json" \
  --argjson cli "$cli_json" \
  '{
    name: "tebako-rs",
    version: $version,
    assets: $assets,
    tools: {
      "tfs": $tfs,
      "tebako-pkg": $pkg,
      "tebako": $cli
    }
  }' > out/manifest.json

echo "== SHA256SUMS =="
cat out/SHA256SUMS
echo "== manifest.json =="
cat out/manifest.json

# ---- size table into the release notes ----------------------------------
{
  echo ""
  echo "## Binary sizes"
  echo ""
  echo "| platform | binary | size (bytes) |"
  echo "|---|---|---|"
  for platform_dir in $(ls -d "$FRAG"/frag-* | sort); do
    platform="${platform_dir##*/frag-}"
    for tool in tebako-bootstrap tfs tebako-pkg tebako; do
      size=$(cat "$platform_dir/${tool}-${platform}.size")
      echo "| $platform | $tool | $size |"
    done
  done
  echo ""
  echo "tebako-bootstrap is size-gated at < 6 MB (raised 2026-07-27: full static botan via crate deps)."
} > out/release-notes.md

if [ -n "${UPLOAD:-}" ]; then
  gh release edit "$TAG" --notes-file out/release-notes.md
fi
