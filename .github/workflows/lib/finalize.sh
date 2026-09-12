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
# Windows platform ids carry the .exe suffix on the uploaded asset name
# (the frag files themselves are suffix-free).
# tebako#493: each line also lands as the per-asset sidecar
# out/sidecars/<asset>.sha256 — the sidecar-era authority the resolver
# reads FIRST; the monoliths (SHA256SUMS, manifest.json) become
# derived-only conveniences for old resolvers.
mkdir -p out/sidecars
: > out/SHA256SUMS
for platform_dir in "$FRAG"/frag-*; do
  case "$platform_dir" in */frag-installers-*) continue ;; esac   # installer frags: their own section below
  platform="${platform_dir##*/frag-}"
  exe=""; case "$platform" in windows-*) exe=".exe" ;; esac
  for tool in tebako-bootstrap tfs tebako-pkg tebako tebako-shim tebako-runtime-launcher; do
    sha=$(cat "$platform_dir/${tool}-${platform}.sha256")
    name="${tool}-${VERSION}-${platform}${exe}"
    echo "$sha  $name" >> out/SHA256SUMS
    echo "$sha  $name" > "out/sidecars/${name}.sha256"
  done
done

# ---- installer fragments (roadmap 83, spec 16 §7) -------------------------
# The installer legs upload frag-installers-<platform>/ dirs holding one
# <asset>.sha256 + <asset>.size per SHIPPED installer container (asset
# names are complete: tebako-setup-<VERSION>-<platform>.{msi,pkg}). A leg
# whose signing gate was disarmed ships nothing and uploads a .disabled
# marker instead — the glob simply finds no hashes. Installers get the
# same SHA256SUMS line + sidecar treatment as every released part.
for installer_dir in "$FRAG"/frag-installers-*/; do
  [ -d "$installer_dir" ] || continue
  for shafile in "$installer_dir"*.sha256; do
    [ -e "$shafile" ] || continue
    asset=$(basename "$shafile" .sha256)
    sha=$(cat "$shafile")
    echo "$sha  $asset" >> out/SHA256SUMS
    echo "$sha  $asset" > "out/sidecars/${asset}.sha256"
  done
done

sort -k2 -o out/SHA256SUMS out/SHA256SUMS

# ---- manifest.json ------------------------------------------------------
# {
#   "name": "tebako-rs",
#   "version": "<VERSION>",
#   "assets": [ {platform, file, sha256, size_bytes}, ... ],   # tebako-bootstrap
#   "tools": { "tfs": [...], "tebako-pkg": [...], "tebako": [...],
#              "tebako-shim": [...], "tebako-runtime-launcher": [...] }
# }
# The `assets` array is exactly the BootstrapManager's consumed shape, so
# the gem/tebako-cli resolve the Rust bootstrap unchanged. Windows asset
# names carry .exe (both here and in SHA256SUMS).
jq_entries() {
  local tool="$1"
  for platform_dir in "$FRAG"/frag-*; do
    case "$platform_dir" in */frag-installers-*) continue ;; esac
    platform="${platform_dir##*/frag-}"
    [ -f "$platform_dir/${tool}-${platform}.sha256" ] || continue
    exe=""; case "$platform" in windows-*) exe=".exe" ;; esac
    sha=$(cat "$platform_dir/${tool}-${platform}.sha256")
    size=$(cat "$platform_dir/${tool}-${platform}.size")
    jq -cn \
      --arg platform "$platform" \
      --arg file "${tool}-${VERSION}-${platform}${exe}" \
      --arg sha256 "$sha" \
      --argjson size_bytes "$size" \
      '{platform: $platform, file: $file, sha256: $sha256, size_bytes: $size_bytes}'
  done
}

bootstrap_json=$(jq_entries tebako-bootstrap | jq -s 'sort_by(.platform)')
tfs_json=$(jq_entries tfs | jq -s 'sort_by(.platform)')
pkg_json=$(jq_entries tebako-pkg | jq -s 'sort_by(.platform)')
cli_json=$(jq_entries tebako | jq -s 'sort_by(.platform)')
shim_json=$(jq_entries tebako-shim | jq -s 'sort_by(.platform)')
launcher_json=$(jq_entries tebako-runtime-launcher | jq -s 'sort_by(.platform)')

# Installers (roadmap 83): platform parsed out of the asset name
# (tebako-setup-<VERSION>-<platform>.<ext>); empty array when no leg
# shipped (gates disarmed — the key is always present, the array may be
# empty: additive schema, old readers ignore it).
installers_json=$(
  for installer_dir in "$FRAG"/frag-installers-*/; do
    [ -d "$installer_dir" ] || continue
    for shafile in "$installer_dir"*.sha256; do
      [ -e "$shafile" ] || continue
      asset=$(basename "$shafile" .sha256)
      sha=$(cat "$shafile")
      size=$(cat "$installer_dir/$asset.size")
      stem="${asset%.*}"                      # strip .msi / .pkg
      platform="${stem#*-setup-"$VERSION"-}"  # strip <name>-setup-<VERSION>-
      jq -cn \
        --arg platform "$platform" \
        --arg file "$asset" \
        --arg sha256 "$sha" \
        --argjson size_bytes "$size" \
        '{platform: $platform, file: $file, sha256: $sha256, size_bytes: $size_bytes}'
    done
  done | jq -s 'sort_by(.platform)'
)

jq -n \
  --arg version "$VERSION" \
  --argjson assets "$bootstrap_json" \
  --argjson tfs "$tfs_json" \
  --argjson pkg "$pkg_json" \
  --argjson cli "$cli_json" \
  --argjson shim "$shim_json" \
  --argjson launcher "$launcher_json" \
  --argjson installers "$installers_json" \
  '{
    name: "tebako-rs",
    version: $version,
    assets: $assets,
    installers: $installers,
    tools: {
      "tfs": $tfs,
      "tebako-pkg": $pkg,
      "tebako": $cli,
      "tebako-shim": $shim,
      "tebako-runtime-launcher": $launcher
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
  for platform_dir in $(ls -d "$FRAG"/frag-* | grep -v '/frag-installers-' | sort); do
    platform="${platform_dir##*/frag-}"
    for tool in tebako-bootstrap tfs tebako-pkg tebako tebako-shim tebako-runtime-launcher; do
      size=$(cat "$platform_dir/${tool}-${platform}.size")
      echo "| $platform | $tool | $size |"
    done
  done
  for installer_dir in "$FRAG"/frag-installers-*/; do
    [ -d "$installer_dir" ] || continue
    for sizefile in "$installer_dir"*.size; do
      [ -e "$sizefile" ] || continue
      asset=$(basename "$sizefile" .size)
      echo "| installer | $asset | $(cat "$sizefile") |"
    done
  done
  echo ""
  echo "tebako-bootstrap is size-gated at < 3 MB (rnp/botan feature-gated out — unverified-first)."
} > out/release-notes.md

if [ -n "${UPLOAD:-}" ]; then
  gh release edit "$TAG" --notes-file out/release-notes.md
fi
