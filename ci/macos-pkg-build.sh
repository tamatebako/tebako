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
# INSTALL_ROOT (/opt/tebako), MIN_MACOS (12.0). Web-bootstrapper (spec 16
# §7): BOOTSTRAP_REGISTRY (the client's registry ref) + BOOTSTRAP_PAYLOADS
# (space-separated names) — stages <root>/bootstrap-seed.sh (self-locating,
# user-re-runnable) + the home/shims grammar marker (spec 05 §3.1) and the
# postinstall runs the seed best-effort. Optional third knob
# BOOTSTRAP_WARM (a subset of BOOTSTRAP_PAYLOADS): the seed also dispatches
# each named shim once, pulling its RUNTIME into the shared home at
# install time (as the install user). After a warm, every user's dispatch
# is read-only against the root-owned home — the runtime download is the
# only dispatch-time write (the registry refresh degrades to loud
# stale-serve, the journal is best-effort). Warm only bounded,
# print-and-exit entrypoints: the seed runs no timeout.
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
# pkgbuild/productbuild versions are numeric triplets — a roadmap-89
# rehearsal's VERSION is a PR ref ("590-merge"), not a product version.
PKG_VERSION="$VERSION"
[[ "$PKG_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || PKG_VERSION="0.0.0"
sed -e "s/@PRODUCT_NAME@/$PRODUCT_NAME/g" -e "s/@VERSION@/$PKG_VERSION/g" \
    -e "s/@ORG_ID@/$ORG_ID/g" -e "s/@MIN_MACOS@/$MIN_MACOS/g" \
    templates/installers/macos/distribution.xml > build/distribution.xml
sed -e "s/@PRODUCT_NAME@/$PRODUCT_NAME/g" -e "s|@INSTALL_ROOT@|$INSTALL_ROOT|g" \
    templates/installers/macos/scripts/postinstall > build/scripts/postinstall
# Optional web-bootstrapper seed (spec 16 §7; the MSI leg's contract,
# mirrored): a self-locating, user-re-runnable seed script at the install
# root + the home/shims store-grammar marker (spec 05 §3.1); the
# postinstall splices the best-effort hook at @BOOTSTRAP_HOOK@ (the
# placeholder is deleted outright when unbound).
if [ -n "${BOOTSTRAP_REGISTRY:-}" ]; then
  : "${BOOTSTRAP_PAYLOADS:?BOOTSTRAP_PAYLOADS is required when BOOTSTRAP_REGISTRY is set (space-separated payload names)}"
  {
    echo '#!/bin/bash'
    echo "# $PRODUCT_NAME bootstrap seed (spec 16 §7) — composed at build time; safe to re-run (tebako install is idempotent)."
    echo 'set -u'
    echo 'ROOT="$(cd "$(dirname "$0")" && pwd)"'
    echo 'export TEBAKO_HOME="$ROOT/home"'
    echo 'mkdir -p "$TEBAKO_HOME/shims"'
    echo "echo \"seeding the $PRODUCT_NAME payload set into \$TEBAKO_HOME (network required)…\""
    echo "\"\$ROOT/bin/tebako\" add-registry \"$BOOTSTRAP_REGISTRY\""
    for p in $BOOTSTRAP_PAYLOADS; do
      echo "\"\$ROOT/bin/tebako\" install \"$p\""
    done
    # Optional warm (spec 16 §7): dispatch each named shim once so its
    # RUNTIME lands in the shared home at install time — after that a
    # user dispatch is read-only against the root-owned home.
    for w in ${BOOTSTRAP_WARM:-}; do
      case " $BOOTSTRAP_PAYLOADS " in
        *" $w "*) ;;
        *) echo "::error::BOOTSTRAP_WARM entry $w is not in BOOTSTRAP_PAYLOADS — warm is a subset"; exit 1 ;;
      esac
      echo "\"\$ROOT/home/shims/$w\" || echo \"warning: warm dispatch of $w failed (offline?) — the first user run downloads its runtime\" >&2"
    done
  } > "pkg-root$INSTALL_ROOT/bootstrap-seed.sh"
  chmod 755 "pkg-root$INSTALL_ROOT/bootstrap-seed.sh"
  mkdir -p "pkg-root$INSTALL_ROOT/home/shims"
  cat > build/bootstrap-hook <<EOF

# --- web-bootstrapper seed (spec 16 §7): best-effort — a failed seed (an
# offline machine) never fails the install; the user re-runs it later.
printf '%s\n' "$INSTALL_ROOT/home/shims" >> "/etc/paths.d/$PRODUCT_NAME"
"$INSTALL_ROOT/bootstrap-seed.sh" || echo "warning: the $PRODUCT_NAME payload seed failed (offline?) — re-run later: $INSTALL_ROOT/bootstrap-seed.sh" >&2
EOF
  echo "bootstrap seed staged: registry $BOOTSTRAP_REGISTRY — payloads: $BOOTSTRAP_PAYLOADS"
else
  : > build/bootstrap-hook
fi
sed -e "/@BOOTSTRAP_HOOK@/r build/bootstrap-hook" -e "/@BOOTSTRAP_HOOK@/d" \
    build/scripts/postinstall > build/scripts/postinstall.rendered
mv build/scripts/postinstall.rendered build/scripts/postinstall
# The splice recreates the file (umask 644) — the exec bit goes LAST, on
# the file that actually ships (an un-executable postinstall fails the
# install: "an error occurred while running scripts from the package").
chmod +x build/scripts/postinstall
if grep -qE '@(PRODUCT_NAME|VERSION|ORG_ID|MIN_MACOS|INSTALL_ROOT|BOOTSTRAP_HOOK)@' build/distribution.xml build/scripts/postinstall; then
  echo "::error::unrendered template token left in the pkg inputs — the template drifted from this script"; exit 1
fi

# ---- 3. pkgbuild → productbuild ------------------------------------------
# --root is pkg-root, not pkg-root$INSTALL_ROOT: the payload must KEEP its
# opt/<name>/bin prefix under --install-location / (landing at
# /opt/<name>/bin on the data volume). Rooting at the INSTALL_ROOT subdir
# drops that prefix and the payload lands at /bin — the sealed system
# volume, which installer(8) refuses outright (the v2.8.0 legs).
pkgbuild --root pkg-root --install-location / \
  --identifier "$ORG_ID.$PRODUCT_NAME.pkg" --version "$PKG_VERSION" \
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
  # staple fetches the online ticket — the same Accepted≠visible race as
  # the binaries' notarization check: bounded poll, then a named failure.
  staple_deadline=$(( $(date +%s) + 300 ))
  until xcrun stapler staple "out/$ASSET"; do
    [ "$(date +%s)" -lt "$staple_deadline" ] || { echo "::error::stapler could not fetch the ticket for $ASSET within 300s"; exit 1; }
    echo "ticket not yet fetchable for $ASSET — polling again in 10 s"
    sleep 10
  done
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
if [ -n "${BOOTSTRAP_REGISTRY:-}" ]; then
  # The seed ran during the install (the runner is networked): its effects
  # are the rehearsal — registry registered, every payload cached into the
  # machine home, the shims dir on PATH.
  [ -x "$INSTALL_ROOT/bootstrap-seed.sh" ] || { echo "::error::bootstrap-seed.sh missing after install"; exit 1; }
  grep -q "$INSTALL_ROOT/home/shims" "/etc/paths.d/$PRODUCT_NAME" || { echo "::error::/etc/paths.d/$PRODUCT_NAME lacks the shims line"; exit 1; }
  [ -f "$INSTALL_ROOT/home/config.yaml" ] || { echo "::error::the seed did not register the registry ($INSTALL_ROOT/home/config.yaml missing)"; exit 1; }
  for p in $BOOTSTRAP_PAYLOADS; do
    [ -d "$INSTALL_ROOT/home/payloads/$p" ] || { echo "::error::seed payload $p missing under $INSTALL_ROOT/home/payloads"; exit 1; }
  done
  if [ -n "${BOOTSTRAP_WARM:-}" ]; then
    # The warm dispatch cached each warmed payload's RUNTIME at install
    # time: the shared home is complete and a user's dispatch is read-only.
    [ -d "$INSTALL_ROOT/home/runtimes" ] && [ -n "$(ls -A "$INSTALL_ROOT/home/runtimes")" ] \
      || { echo "::error::BOOTSTRAP_WARM bound but $INSTALL_ROOT/home/runtimes is empty — the warm dispatch did not land"; exit 1; }
    echo "warm rehearsal OK (runtimes cached: $(ls "$INSTALL_ROOT/home/runtimes" | tr '\n' ' '))"
  fi
  echo "bootstrap seed rehearsal OK (registry registered, payloads cached, shims on PATH)"
fi
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
