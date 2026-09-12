#!/bin/bash
# macos-sign-setup-installer.sh — the Developer ID **Installer** half of
# the Apple signing setup (roadmap 83, spec 31 §5a): a DIFFERENT
# certificate from the Developer ID Application one macos-sign-setup.sh
# imports. productsign authenticates the pkg CONTAINER; codesign
# authenticates the Mach-Os inside — two certs, two keychains, two gates.
#
# The repo variable APPLE_INSTALLER_SIGNING_ENABLED=true arms this gate;
# anything else is a loud unsigned no-op (spec 00 invariant 7). Armed but
# a secret unresolved is a FAST named failure (spec 31 §5's discipline).
#
# Armed, exports (via GITHUB_ENV):
#   INSTALLER_SIGN_HASH  — the cert SHA-1 (sign by hash, NEVER by name —
#                          the spec-31 duplicate-identity lesson)
#   INSTALLER_KEYCHAIN   — the keychain productsign searches
# and emits armed=<true|false> to GITHUB_OUTPUT (the leg's ship/upload
# steps gate on it — the same contract as ci/windows-sign-gate.sh).
#
# Required env when armed: APPLE_DEVELOPER_ID_INSTALLER_P12 (base64),
# APPLE_DEVELOPER_ID_INSTALLER_P12_PASSWORD. Runner env: RUNNER_TEMP,
# GITHUB_ENV.
set -euo pipefail

if [ "${APPLE_INSTALLER_SIGNING_ENABLED:-false}" != "true" ]; then
  echo "::notice::spec 31 §5a: APPLE_INSTALLER_SIGNING_ENABLED != true — the pkg ships UNSIGNED (spec 00 invariant 7)"
  echo "armed=false" >> "$GITHUB_OUTPUT"
  exit 0
fi

missing=""
for v in APPLE_DEVELOPER_ID_INSTALLER_P12 APPLE_DEVELOPER_ID_INSTALLER_P12_PASSWORD; do
  [ -n "${!v:-}" ] || missing="$missing $v"
done
if [ -n "$missing" ]; then
  echo "::error::APPLE_INSTALLER_SIGNING_ENABLED=true but unset:$missing — fast failure, never a partial release (spec 31 §5a)"
  exit 1
fi
# A signed-but-unnotarized pkg never ships: the productsign plane requires
# the notary plane (spec 31 §5's APPLE_SIGNING_ENABLED) armed beside it.
if [ "${APPLE_SIGNING_ENABLED:-false}" != "true" ]; then
  echo "::error::APPLE_INSTALLER_SIGNING_ENABLED=true but APPLE_SIGNING_ENABLED != true — a signed pkg without notarization+stapling is a named misconfiguration (spec 31 §5a)"
  exit 1
fi

work="$RUNNER_TEMP/tebako-sign-installer"
mkdir -p "$work"
printf '%s' "$APPLE_DEVELOPER_ID_INSTALLER_P12" | base64 -d > "$work/devid-installer.p12"
KC="$work/sign-installer.keychain"
security create-keychain -p sign-inst-kc-pass "$KC"
security unlock-keychain -p sign-inst-kc-pass "$KC"
security list-keychains -d user -s "$KC" $(security list-keychains -d user | tr -d '"')
security import "$work/devid-installer.p12" -k "$KC" -P "$APPLE_DEVELOPER_ID_INSTALLER_P12_PASSWORD" \
  -T /usr/bin/productsign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple: -k sign-inst-kc-pass "$KC" >/dev/null

HASH=$(security find-identity -v -p basic "$KC" | awk '/Developer ID Installer/ {print $2; exit}')
[ -n "$HASH" ] || { echo "::error::no Developer ID Installer identity in the p12 (the Application cert is a DIFFERENT type and cannot productsign)"; exit 1; }
echo "INSTALLER_SIGN_HASH=$HASH" >> "$GITHUB_ENV"
echo "INSTALLER_KEYCHAIN=$KC" >> "$GITHUB_ENV"
echo "armed=true" >> "$GITHUB_OUTPUT"
echo "spec 31 §5a: installer signing armed — Developer ID Installer cert ${HASH:0:10}… (keychain $KC)"
