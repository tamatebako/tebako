#!/bin/bash
# macos-notarize.sh — notarize the leg's staged (already signed)
# binaries and leave each one quarantine-marked, so the leg's OWN ship
# gate then smokes every binary under the exact Gatekeeper path a user's
# download takes (spec 31 §6.3). Bare CLI Mach-O: stapling is
# unsupported and spctl rejects standalone tools BY DESIGN — Apple keeps
# an ONLINE ticket (Gatekeeper resolves it at first exec), so the
# documented verification is codesign's notarization check; the
# quarantine mark + the standard ship gate is the exec canary (a
# notarization that only partially landed fails THIS leg, never a user
# box). The workflow skips this step entirely when the spec 31 gate is
# disarmed (if: vars.APPLE_SIGNING_ENABLED == 'true').
#
# Required env: APPLE_ASC_KEY_P8, APPLE_ASC_KEY_ID, APPLE_ASC_ISSUER_ID,
# RUNNER_TEMP.
# Usage: macos-notarize.sh <binary> [<binary> ...]
set -euo pipefail
[ "$#" -ge 1 ] || { echo "usage: $0 <binary> [<binary> ...]" >&2; exit 64; }

work="$RUNNER_TEMP/tebako-notarize"
mkdir -p "$work"
printf '%s' "$APPLE_ASC_KEY_P8" > "$work/AuthKey.p8"
# -j: archive the bare file names, not the out/ path prefix.
zip -q -j "$work/binaries.zip" "$@"
xcrun notarytool submit "$work/binaries.zip" --key "$work/AuthKey.p8" \
  --key-id "$APPLE_ASC_KEY_ID" --issuer "$APPLE_ASC_ISSUER_ID" \
  --wait --timeout 20m | tee "$work/notary.txt"
grep -q 'status: Accepted' "$work/notary.txt" || {
  xcrun notarytool log "$(grep -m1 -oE '[0-9a-f-]{36}' "$work/notary.txt")" \
    --key "$work/AuthKey.p8" --key-id "$APPLE_ASC_KEY_ID" --issuer "$APPLE_ASC_ISSUER_ID" 2>&1 | tail -20
  echo "::error::notarytool did not Accept the submission"; exit 1; }

for bin in "$@"; do
  codesign --verify --strict --check-notarization -R=notarized "$bin"
  xattr -w com.apple.quarantine '0081;00000000;Safari;' "$bin"
done
echo "notarized + quarantine-marked: $# binaries (the ship gate is the exec canary)"
