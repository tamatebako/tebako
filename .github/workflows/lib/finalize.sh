#!/bin/bash
# finalize.sh — merge the per-platform fragments into SHA256SUMS and
# manifest.json (the shape tebako-bootstrap releases use today, extended
# with a per-tool `tools` map), and append the size table to the release
# notes. Runs in the finalize job after actions/download-artifact has
# laid every frag-<platform> directory flat into ./fragments.
# spec 13 §5a (tebako#1120): the legs' `.cdx.sha256` fragments add one
# SHA256SUMS row per CycloneDX SBOM asset (no sidecar — a SBOM's anchor
# is its signed SHA256SUMS row + its own .asc); the SBOMs join no other
# index (manifest.json carries the resolve-consumed parts only).
#
# Required env: VERSION (tag minus v), TAG (release tag).
set -euo pipefail

mkdir -p out
FRAG=fragments

# ---- SHA256SUMS (sorted, "<sha>  <file>") ------------------------------
# Windows platform ids carry the .exe suffix on the uploaded asset name
# (the frag files themselves are suffix-free); the preload shim
# (tebako#1099, POSIX-only) carries its library extension — .dylib on
# macOS, .so on linux.
# tebako#493: each line also lands as the per-asset sidecar
# out/sidecars/<asset>.sha256 — the sidecar-era authority the resolver
# reads FIRST; the monoliths (SHA256SUMS, manifest.json) become
# derived-only conveniences for old resolvers.
asset_name() {  # <tool> <platform> → the released asset's file name
  local tool="$1" platform="$2" ext=""
  case "$platform" in windows-*) ext=".exe" ;; esac
  if [ "$tool" = libtfs_preload ]; then
    case "$platform" in
      macos-*) ext=".dylib" ;;
      linux-*) ext=".so" ;;
    esac
  fi
  echo "${tool}-${VERSION}-${platform}${ext}"
}

mkdir -p out/sidecars
: > out/SHA256SUMS
for platform_dir in "$FRAG"/frag-*; do
  case "$platform_dir" in */frag-installers-*) continue ;; esac   # installer frags: their own section below
  platform="${platform_dir##*/frag-}"
  for tool in tebako-bootstrap tfs tebako-pkg tebako tebako-shim tebako-runtime-launcher libtfs_preload; do
    # A partial platform leg (windows-ucrt-arm64 ships tfs + tebako-pkg
    # + tebako-runtime-launcher, roadmap 02 phase 2; windows legs never
    # carry the preload shim) carries only its own fragments.
    [ -f "$platform_dir/${tool}-${platform}.sha256" ] || continue
    sha=$(cat "$platform_dir/${tool}-${platform}.sha256")
    name=$(asset_name "$tool" "$platform")
    echo "$sha  $name" >> out/SHA256SUMS
    echo "$sha  $name" > "out/sidecars/${name}.sha256"
  done
done

# The SBOM asset's file name for a (tool, platform-token) pair — the
# staged asset's name minus its .exe/.dylib/.so/.tar.gz suffix, plus
# .cdx.json (spec 13 §5a; sbom-stage.sh's sbom_name_of is the generating
# sibling — the two stay in lockstep). The link unit's name token is NOT
# always the leg's platform id (the windows-arm64 leg ships its tarball
# as aarch64-windows-gnu), which is why the merge below reads the token
# back off the fragment's file name instead of deriving it.
sbom_asset_name() {  # <tool> <platform-token> → the SBOM asset's file name
  local name
  if [ "$1" = link-unit ]; then
    name="link-unit-${VERSION}-$2.tar.gz"
  else
    name=$(asset_name "$1" "$2")
  fi
  name="${name%.exe}"
  name="${name%.dylib}"
  name="${name%.so}"
  name="${name%.tar.gz}"
  echo "${name}.cdx.json"
}

# ---- CycloneDX SBOM fragments (spec 13 §5a, tebako#1120) ------------------
# One .cdx.json per non-installer part, generated in-leg from the real
# Cargo resolution. A SBOM gets a SHA256SUMS row like every released
# part but NO per-asset .sha256 sidecar: its anchor is the signed
# SHA256SUMS row plus its own .asc — a sidecar would duplicate that
# anchor without independent trust. The fragment stem is
# <tool>-<platform-token>; the token rides the fragment's file name
# because the link unit's spelling is not always the leg's platform id
# (windows-arm64 ships link-unit-<VERSION>-aarch64-windows-gnu.tar.gz
# from the windows-ucrt-arm64 leg). The classification tries the longest
# tool ids first ("tebako" is a proper prefix of four sibling ids — a
# prefix glob would misfile their rows), and an unclassifiable fragment
# is a named failure: a silently-dropped row would pass every count
# gate with the SBOM's anchor missing.
for platform_dir in "$FRAG"/frag-*; do
  case "$platform_dir" in */frag-installers-*) continue ;; esac
  for cdxfrag in "$platform_dir"/*.cdx.sha256; do
    [ -e "$cdxfrag" ] || continue
    stem=$(basename "$cdxfrag" .cdx.sha256)
    tool="" token=""
    for candidate in tebako-runtime-launcher tebako-bootstrap tebako-pkg tebako-shim libtfs_preload link-unit tfs tebako; do
      case "$stem" in
        "$candidate"-*)
          tool="$candidate"
          token="${stem#"$candidate"-}"
          break
          ;;
      esac
    done
    if [ -z "$tool" ]; then
      echo "finalize: unclassifiable SBOM fragment $cdxfrag — the stem matches no known tool id" >&2
      exit 1
    fi
    sha=$(cat "$cdxfrag")
    echo "$sha  $(sbom_asset_name "$tool" "$token")" >> out/SHA256SUMS
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
#              "tebako-shim": [...], "tebako-runtime-launcher": [...],
#              "libtfs_preload": [...] }   # the POSIX preload shim (tebako#1099)
# }
# The `assets` array is exactly the BootstrapManager's consumed shape, so
# the gem/tebako-cli resolve the Rust bootstrap unchanged. Windows asset
# names carry .exe, the preload shim its library extension (both here
# and in SHA256SUMS — asset_name is the single resolver).
jq_entries() {
  local tool="$1"
  for platform_dir in "$FRAG"/frag-*; do
    case "$platform_dir" in */frag-installers-*) continue ;; esac
    platform="${platform_dir##*/frag-}"
    [ -f "$platform_dir/${tool}-${platform}.sha256" ] || continue
    sha=$(cat "$platform_dir/${tool}-${platform}.sha256")
    size=$(cat "$platform_dir/${tool}-${platform}.size")
    jq -cn \
      --arg platform "$platform" \
      --arg file "$(asset_name "$tool" "$platform")" \
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
preload_json=$(jq_entries libtfs_preload | jq -s 'sort_by(.platform)')

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
  --argjson preload "$preload_json" \
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
      "tebako-runtime-launcher": $launcher,
      "libtfs_preload": $preload
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
    for tool in tebako-bootstrap tfs tebako-pkg tebako tebako-shim tebako-runtime-launcher libtfs_preload; do
      [ -f "$platform_dir/${tool}-${platform}.size" ] || continue
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
