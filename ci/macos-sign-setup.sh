#!/bin/bash
# macos-sign-setup.sh — the spec 31 §5 enablement gate for the macOS
# release legs. The repo variable APPLE_SIGNING_ENABLED=true arms
# signing; anything else ships unsigned BY DESIGN (spec 00 invariant 7:
# unsigned stays first-class) and this step is a loud notice. Armed but
# a secret unresolved is a FAST named failure — never a partial release
# (a leg that hashes unsigned bytes while its siblings sign is the
# partial-release class; sign-then-hash is mandatory).
#
# Armed, this creates the throwaway signing keychain, imports the
# Developer ID Application identity, and exports for the leg's later
# steps (via GITHUB_ENV):
#   CODESIGN_HASH     — the cert SHA-1. Sign by hash, NEVER by name: a
#                       duplicate identity in any search-listed keychain
#                       makes name resolution ambiguous (proven locally
#                       and in the spec-31 sign-probe).
#   CODESIGN_KEYCHAIN — the keychain path codesign searches.
#
# Required env when armed: APPLE_DEVELOPER_ID_P12 (base64),
# APPLE_DEVELOPER_ID_P12_PASSWORD, APPLE_TEAM_ID. Runner env:
# APPLE_SIGNING_ENABLED (from vars.), RUNNER_TEMP, GITHUB_ENV.
set -euo pipefail

if [ "${APPLE_SIGNING_ENABLED:-false}" != "true" ]; then
  echo "::notice::spec 31: APPLE_SIGNING_ENABLED != true — this leg ships UNSIGNED (spec 00 invariant 7)"
  exit 0
fi

missing=""
for v in APPLE_DEVELOPER_ID_P12 APPLE_DEVELOPER_ID_P12_PASSWORD APPLE_TEAM_ID; do
  [ -n "${!v:-}" ] || missing="$missing $v"
done
if [ -n "$missing" ]; then
  echo "::error::APPLE_SIGNING_ENABLED=true but unset:$missing — spec 31 §5: fast failure, never a partial release"
  exit 1
fi

work="$RUNNER_TEMP/tebako-sign"
mkdir -p "$work"
printf '%s' "$APPLE_DEVELOPER_ID_P12" | base64 -d > "$work/devid.p12"
KC="$work/sign.keychain"
security create-keychain -p sign-kc-pass "$KC"
security unlock-keychain -p sign-kc-pass "$KC"
# codesign resolves identities through the user search list — a freshly
# created keychain is not on it (the sign-probe's attempt-2 "item could
# not be found" lesson).
security list-keychains -d user -s "$KC" $(security list-keychains -d user | tr -d '"')
security import "$work/devid.p12" -k "$KC" -P "$APPLE_DEVELOPER_ID_P12_PASSWORD" \
  -T /usr/bin/codesign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple: -k sign-kc-pass "$KC" >/dev/null

HASH=$(security find-identity -v -p codesigning "$KC" | awk '/Developer ID Application/ {print $2; exit}')
[ -n "$HASH" ] || { echo "::error::no Developer ID Application identity in the p12"; exit 1; }
echo "CODESIGN_HASH=$HASH" >> "$GITHUB_ENV"
echo "CODESIGN_KEYCHAIN=$KC" >> "$GITHUB_ENV"
echo "spec 31: signing armed — Developer ID Application cert ${HASH:0:10}… (keychain $KC)"
