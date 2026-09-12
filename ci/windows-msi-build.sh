#!/bin/bash
# windows-msi-build.sh — build (or re-hash) the system MSI installer from
# the release's own staged windows exes (roadmap 83, spec 16 §7).
#
# Subcommands (the sign-then-hash law, spec 34 §1.3, splits the two —
# signing is a workflow ACTION between them):
#   build   verify the staged exes against the windows leg's OWN
#           fragments (frag-<platform>/<tool>-<platform>.sha256 — the
#           sign-then-hash anchors, spec 34 §1.3; THIS release's
#           SHA256SUMS does not exist yet — finalize makes it AFTER us),
#           then wix build the template into out/<asset>.msi (unsigned —
#           the Azure Artifact Signing step signs it in place when armed).
#   frag    hash the FINAL bytes (signed when the gate armed) into
#           fragments/frag-installers-<platform>/<asset>.{sha256,size}
#           for finalize.
#
# Required env: VERSION (tag minus v), SRC_DIR (the staged windows exes),
# FRAG_DIR (the downloaded frag-<platform> artifact), RUNNER_TEMP.
# Optional: PLATFORM (default windows-ucrt64), PRODUCT_NAME (default
# tebako), MANUFACTURER (default tamatebako), MSI_UPGRADE_CODE (REQUIRED
# for build — the pipeline passes the locked per-product GUID; clients
# pass their own).
#
# The staged tool set is CURATED (the 4 PATH tools: tebako, tebako-shim,
# tfs, tebako-pkg) — the bootstrap and runtime-launcher ship on the
# release for press/embedding, not for PATH. A missing staged exe fails
# HERE, named, never in a user's Add/Remove Programs.
set -euo pipefail
[ "$#" -eq 1 ] || { echo "usage: $0 build|frag" >&2; exit 64; }
MODE="$1"

: "${VERSION:?VERSION is required (the release version, tag minus v)}"
: "${SRC_DIR:?SRC_DIR is required (staged exes + SHA256SUMS)}"
PLATFORM="${PLATFORM:-windows-ucrt64}"
PRODUCT_NAME="${PRODUCT_NAME:-tebako}"
MANUFACTURER="${MANUFACTURER:-tamatebako}"
TOOLS="tebako tebako-shim tfs tebako-pkg"
ASSET="${PRODUCT_NAME}-setup-${VERSION}-${PLATFORM}.msi"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

case "$MODE" in
  build)
    : "${MSI_UPGRADE_CODE:?MSI_UPGRADE_CODE is required (the locked per-product UpgradeCode GUID)}"
    : "${FRAG_DIR:?FRAG_DIR is required (the downloaded frag-<platform> artifact)}"
    # Verify the staged exes against the windows leg's OWN fragments —
    # the MSI wraps EXACTLY the published signed bytes (the leg's frags
    # were re-anchored post-signing, spec 34 §1.3; a corrupted download
    # inside a signed installer is the failure this precludes).
    mkdir -p "msi-input"
    for tool in $TOOLS; do
      exe="${tool}-${VERSION}-${PLATFORM}.exe"
      [ -f "$SRC_DIR/$exe" ] || { echo "::error::$exe not staged in $SRC_DIR"; exit 1; }
      [ -f "$FRAG_DIR/${tool}-${PLATFORM}.sha256" ] || { echo "::error::no fragment ${tool}-${PLATFORM}.sha256 in $FRAG_DIR — the windows leg did not complete"; exit 1; }
      want=$(cat "$FRAG_DIR/${tool}-${PLATFORM}.sha256")
      got=$(sha256_of "$SRC_DIR/$exe")
      [ "$got" = "$want" ] || { echo "::error::sha256 mismatch on $exe (download corruption) — got $got want $want"; exit 1; }
      cp "$SRC_DIR/$exe" "msi-input/$tool.exe"
    done
    echo "staged + verified against the leg's signed-byte fragments: $TOOLS"
    mkdir -p out
    command -v wix >/dev/null 2>&1 || { echo "::error::wix (WiX v5 dotnet tool) not on PATH — the workflow installs it"; exit 1; }
    wix build -arch x64 \
      templates/installers/windows/tebako.wxs \
      -d "ProductName=$PRODUCT_NAME" \
      -d "ProductVersion=$VERSION" \
      -d "Manufacturer=$MANUFACTURER" \
      -d "UpgradeCode=$MSI_UPGRADE_CODE" \
      -d "BinDir=msi-input" \
      -o "out/$ASSET"
    echo "built: out/$ASSET (unsigned — the Azure step signs in place when armed)"
    ;;
  frag)
    [ -f "out/$ASSET" ] || { echo "::error::out/$ASSET missing — build step did not run?"; exit 1; }
    mkdir -p "fragments/frag-installers-$PLATFORM"
    sha256_of "out/$ASSET" > "fragments/frag-installers-$PLATFORM/$ASSET.sha256"
    stat -c %s "out/$ASSET" 2>/dev/null > "fragments/frag-installers-$PLATFORM/$ASSET.size" \
      || stat -f %z "out/$ASSET" > "fragments/frag-installers-$PLATFORM/$ASSET.size"
    echo "frag: $(cat fragments/frag-installers-$PLATFORM/$ASSET.sha256)  $ASSET (over the FINAL bytes — spec 34 §1.3)"
    ;;
  *) echo "usage: $0 build|frag" >&2; exit 64 ;;
esac
