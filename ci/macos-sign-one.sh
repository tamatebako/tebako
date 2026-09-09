#!/bin/bash
# macos-sign-one.sh — sign ONE staged binary with the Developer ID
# identity the setup step armed (spec 31 §5). Hardened runtime, no
# entitlements: the product tools JIT nothing and dlopen no materialized
# code (the runtime exe's entitlement pair is the factory's profile,
# spec 31 §3/§5). Gate disarmed → loud unsigned no-op (spec 00
# invariant 7). The TeamIdentifier is asserted against APPLE_TEAM_ID
# when present — a wrong-cert signature fails HERE, not on a user box.
#
# Usage: macos-sign-one.sh <binary>
set -euo pipefail
bin="$1"
if [ -z "${CODESIGN_HASH:-}" ]; then
  echo "unsigned: $(basename "$bin") (spec 31 gate disarmed — invariant 7)"
  exit 0
fi
codesign --force --options runtime --timestamp \
  --keychain "${CODESIGN_KEYCHAIN:?setup step did not export CODESIGN_KEYCHAIN}" \
  --sign "$CODESIGN_HASH" "$bin"
codesign --verify --strict --verbose=1 "$bin"
if [ -n "${APPLE_TEAM_ID:-}" ]; then
  codesign -dvv "$bin" 2>&1 | grep -q "TeamIdentifier=$APPLE_TEAM_ID" \
    || { echo "::error::$(basename "$bin"): TeamIdentifier is not $APPLE_TEAM_ID — signed by an unexpected identity"; exit 1; }
fi
echo "signed: $(basename "$bin") (hardened runtime)"
