#!/bin/bash
# macos-pkg-build.sh — build the system pkg installer from the release's
# own staged macOS binaries (roadmap 83, spec 16 §7 / spec 31 §5a).
# pkgbuild (component) → productbuild (distribution) → [armed] productsign
# (Developer ID **Installer** — NOT the Application cert) → notarize →
# STAPLE (a pkg carries the offline Gatekeeper ticket bare exes cannot).
# Sign-then-hash is honored by construction: signing is IN-SCRIPT before
# the fragment is written (the macOS pattern — spec 34 §1.3's two-phase
# dance is the Windows exception, needed only because its signing is a
# workflow action).
#
# Required env: VERSION (tag minus v), PLATFORM (macos-arm64|macos-x86_64),
# SRC_DIR (the 4 staged binaries), FRAG_DIR (the downloaded
# frag-<platform> artifact — the sign-then-hash anchors; THIS release's
# SHA256SUMS does not exist yet, finalize makes it AFTER us), RUNNER_TEMP.
# Optional: PRODUCT_NAME (tebako), ORG_ID (org.tamatebako),
# INSTALL_ROOT (/opt/tebako), MIN_MACOS (12.0).
# Gate env (the workflow's setup step): APPLE_INSTALLER_SIGNING_ENABLED,
# INSTALLER_SIGN_HASH, INSTALLER_KEYCHAIN; notary: APPLE_ASC_KEY_P8,
# APPLE_ASC_KEY_ID, APPLE_ASC_ISSUER_ID.
#
# Ship gates (all run in-script, before any upload): pkgutil
# --check-signature (armed), stapler validate + spctl --assess -t install
# (armed), and ALWAYS a real install rehearsal (sudo installer -target /)
# proving the payload lands and postinstall writes the paths.d entry.
set -euo pipefail

: "${VERSION:?VERSION is required (the release version, tag minus v)}"
: "${PLATFORM:?PLATFORM is required (macos-arm64|macos-x86_64)}"
: "${SRC_DIR:?SRC_DIR is required (staged binaries + SHA256SUMS)}"
PRODUCT_NAME="${PRODUCT_NAME:-tebako}"
ORG_ID="${ORG_ID:-org.tamatebako}"
INSTALL_ROOT="${INSTALL_ROOT:-/opt/tebako}"
MIN_MACOS="${MIN_MACOS:-12.0}"
TOOLS="tebako tebako-shim tfs tebako-pkg"
ASSET="${PRODUCT_NAME}-setup-${VERSION}-${PLATFORM}.pkg"

# ---- 1. stage + verify the binaries against the leg's fragments ---------
# The macOS leg's frags were computed over the SIGNED+notarized bytes
# (sign-then-hash, spec 31 §5) — the pkg wraps EXACTLY the published
# bytes; a corrupted download inside a signed installer is the failure
# this precludes.
: "${FRAG_DIR:?FRAG_DIR is required (the downloaded frag-$PLATFORM artifact)}"
rm -rf pkg-root build out
mkdir -p "pkg-root$INSTALL_ROOT/bin" build/scripts out
for tool in $TOOLS; do
  bin="${tool}-${VERSION}-${PLATFORM}"
  [ -f "$SRC_DIR/$bin" ] || { echo "::error::$bin not staged in $SRC_DIR"; exit 1; }
  [ -f "$FRAG_DIR/${tool}-${PLATFORM}.sha256" ] || { echo "::error::no fragment ${tool}-${PLATFORM}.sha256 in $FRAG_DIR — the macos leg did not complete"; exit 1; }
  want=$(cat "$FRAG_DIR/${tool}-${PLATFORM}.sha256")
  got=$(shasum -a 256 "$SRC_DIR/$bin" | cut -d' ' -f1)
  [ "$got" = "$want" ] || { echo "::error::sha256 mismatch on $bin — got $got want $want"; exit 1; }
  cp "$SRC_DIR/$bin" "pkg-root$INSTALL_ROOT/bin/$tool"
  chmod 755 "pkg-root$INSTALL_ROOT/bin/$tool"
done
echo "staged + verified against the leg's signed-byte fragments: $TOOLS"

# ---- 2. render the template tokens (productbuild has no binds) ----------
sed -e "s/@PRODUCT_NAME@/$PRODUCT_NAME/g" -e "s/@VERSION@/$VERSION/g" \
    -e "s/@ORG_ID@/$ORG_ID/g" -e "s/@MIN_MACOS@/$MIN_MACOS/g" \
    templates/installers/macos/distribution.xml > build/distribution.xml
sed -e "s/@PRODUCT_NAME@/$PRODUCT_NAME/g" -e "s|@INSTALL_ROOT@|$INSTALL_ROOT|g" \
    templates/installers/macos/scripts/postinstall > build/scripts/postinstall
chmod +x build/scripts/postinstall
if grep -qE '@(PRODUCT_NAME|VERSION|ORG_ID|MIN_MACOS|INSTALL_ROOT)@' build/distribution.xml build/scripts/postinstall; then
  echo "::error::unrendered template token left in the pkg inputs — the template drifted from this script"; exit 1
fi

# ---- 3. pkgbuild → productbuild ------------------------------------------
pkgbuild --root "pkg-root$INSTALL_ROOT" --install-location / \
  --identifier "$ORG_ID.$PRODUCT_NAME.pkg" --version "$VERSION" \
  --scripts build/scripts \
  "build/$PRODUCT_NAME-component.pkg"
productbuild --distribution build/distribution.xml \
  --package-path build "out/$ASSET.unsigned"
mv "out/$ASSET.unsigned" "out/$ASSET"

# ---- 4. sign + notarize + staple (the armed path) -------------------------
if [ "${APPLE_INSTALLER_SIGNING_ENABLED:-false}" = "true" ]; then
  : "${INSTALLER_SIGN_HASH:?the setup step did not export INSTALLER_SIGN_HASH}"
  : "${INSTALLER_KEYCHAIN:?the setup step did not export INSTALLER_KEYCHAIN}"
  mv "out/$ASSET" "out/$ASSET.unsigned"
  productsign --keychain "$INSTALLER_KEYCHAIN" --sign "$INSTALLER_SIGN_HASH" \
    "out/$ASSET.unsigned" "out/$ASSET"
  pkgutil --check-signature "out/$ASSET" | tee build/pkg-sig.txt
  grep -q "Developer ID Installer" build/pkg-sig.txt \
    || { echo "::error::pkg not signed by a Developer ID Installer identity"; exit 1; }
  grep -q 'Status: signed' build/pkg-sig.txt || { echo "::error::pkgutil reports the pkg unsigned"; exit 1; }

  work="$RUNNER_TEMP/tebako-pkg-notarize"
  mkdir -p "$work"
  printf '%s' "$APPLE_ASC_KEY_P8" > "$work/AuthKey.p8"
  xcrun notarytool submit "out/$ASSET" --key "$work/AuthKey.p8" \
    --key-id "$APPLE_ASC_KEY_ID" --issuer "$APPLE_ASC_ISSUER_ID" \
    --wait --timeout 20m | tee "$work/notary.txt"
  grep -q 'status: Accepted' "$work/notary.txt" || {
    xcrun notarytool log "$(grep -m1 -oE '[0-9a-f-]{36}' "$work/notary.txt")" \
      --key "$work/AuthKey.p8" --key-id "$APPLE_ASC_KEY_ID" --issuer "$APPLE_ASC_ISSUER_ID" 2>&1 | tail -20
    echo "::error::notarytool did not Accept the pkg"; exit 1; }
  xcrun stapler staple "out/$ASSET"
  xcrun stapler validate "out/$ASSET"
  # §6.3a: the pkg's assessment IS the right spctl gate (unlike bare
  # exes): notarized + stapled must Assess accepted, source=Notarized
  # Developer ID. (spctl prints its verdict on STDERR — 2>&1 or the tee
  # captures nothing.)
  spctl --assess -t install -v "out/$ASSET" 2>&1 | tee build/spctl.txt
  grep -q "accepted" build/spctl.txt || { echo "::error::spctl rejected the notarized pkg"; exit 1; }
  echo "signed + notarized + STAPLED: $ASSET"
else
  echo "::notice::spec 31 §5a gate disarmed — the pkg is UNSIGNED (spec 00 invariant 7); the install rehearsal still gates it"
fi

# ---- 5. the install rehearsal (always — signed or not) --------------------
sudo installer -pkg "out/$ASSET" -target /
[ -x "$INSTALL_ROOT/bin/$PRODUCT_NAME" ] || { echo "::error::$INSTALL_ROOT/bin/$PRODUCT_NAME missing after install"; exit 1; }
"$INSTALL_ROOT/bin/$PRODUCT_NAME" --version
[ -f "/etc/paths.d/$PRODUCT_NAME" ] || { echo "::error::/etc/paths.d/$PRODUCT_NAME missing — postinstall did not run"; exit 1; }
grep -q "$INSTALL_ROOT/bin" "/etc/paths.d/$PRODUCT_NAME" || { echo "::error::/etc/paths.d/$PRODUCT_NAME content wrong"; exit 1; }
sudo rm -rf "$INSTALL_ROOT" "/etc/paths.d/$PRODUCT_NAME"
echo "install rehearsal OK (payload lands, postinstall writes the paths.d entry, the installed tool runs)"

# ---- 6. the fragment over the FINAL bytes (ship-armed runs only) --------
# A disarmed run still builds + rehearses the pkg (the template's CI
# signal) but ships NOTHING — an unsigned installer container never lands
# on the release (the container's audience is exactly the one SmartScreen
# / Gatekeeper warnings burn; the loose binaries remain the
# unsigned-first-class path, spec 00 invariant 7). No ship → no frag:
# finalize folds whatever frags exist into SHA256SUMS, and a frag without
# its asset is broken data.
if [ "${APPLE_INSTALLER_SIGNING_ENABLED:-false}" = "true" ]; then
  mkdir -p "fragments/frag-installers-$PLATFORM"
  shasum -a 256 "out/$ASSET" | cut -d' ' -f1 > "fragments/frag-installers-$PLATFORM/$ASSET.sha256"
  stat -f %z "out/$ASSET" > "fragments/frag-installers-$PLATFORM/$ASSET.size"
  echo "frag: $(cat fragments/frag-installers-$PLATFORM/$ASSET.sha256)  $ASSET"
fi
