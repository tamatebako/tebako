#!/bin/bash
# sign-release.sh — spec 09 §2: every released part ships a detached
# .asc, and the release index (SHA256SUMS, manifest.json) ships signed.
# Runs in the finalize job AFTER the per-leg uploads, the SHA256SUMS /
# manifest.json / sidecar uploads — so the release it signs is already
# byte-complete (the completeness gate then counts the .asc files too).
#
# Required env: TAG (release tag), GH_TOKEN (gh).
# Gate env: TEBAKO_RELEASE_SIGNING_ENABLED ("true" arms; anything else =
# unsigned-first, exit 0 — unsigned stays first-class, spec 09 §3).
# Armed + an empty TEBAKO_RELEASE_SIGNING_KEY is a fast NAMED failure
# (the spec 31 §5 house style: a gate that cannot run must say so).
#
# TEBAKO_RELEASE_SIGNING_KEY: base64 of the unprotected armored export
# of the release signing SUBKEY (not the root primary — a CI compromise
# costs a subkey revocation, not the root).
#
# The signer binary is the PREVIOUS release's tebako-pkg, pinned by name
# and sha256-verified against that release's own SHA256SUMS — a release
# is never the first execution of its own bits, and the signing tool's
# provenance is exactly as pinned as the artifacts it signs.
#
# The 42 per-asset .sha256 sidecars are NOT separately signed: each is a
# derived line of the signed SHA256SUMS (tebako#493); signing the parts
# and the sums covers them.
set -euo pipefail
: "${TAG:?TAG is required}"

if [ "${TEBAKO_RELEASE_SIGNING_ENABLED:-}" != "true" ]; then
  echo "release signing disarmed (TEBAKO_RELEASE_SIGNING_ENABLED != 'true') — unsigned-first (spec 09 §3)"
  exit 0
fi
if [ -z "${TEBAKO_RELEASE_SIGNING_KEY:-}" ]; then
  echo "NAMED FAILURE: TEBAKO_RELEASE_SIGNING_ENABLED=true but the TEBAKO_RELEASE_SIGNING_KEY secret is not set" >&2
  exit 1
fi

WORK=sign-work
rm -rf "$WORK"
mkdir -p "$WORK/tool" "$WORK/assets" out/signatures

# ---- the signing key -----------------------------------------------------
printf '%s' "$TEBAKO_RELEASE_SIGNING_KEY" | base64 -d > "$WORK/release-key.asc"
chmod 600 "$WORK/release-key.asc"

# ---- the signer: the previous release's tebako-pkg, pinned + verified ----
PREV_TAG=$(gh release list --json tagName --limit 50 --jq '.[].tagName' | grep -vx "$TAG" | head -n 1)
if [ -z "$PREV_TAG" ]; then
  echo "NAMED FAILURE: no release before $TAG to source the signing tool from" >&2
  exit 1
fi
echo "signing tool: tebako-pkg from $PREV_TAG"
gh release download "$PREV_TAG" --dir "$WORK/tool" \
  --pattern 'tebako-pkg-*-linux-gnu-x86_64' --pattern 'SHA256SUMS' --clobber
PKG=$(ls "$WORK"/tool/tebako-pkg-*-linux-gnu-x86_64)
WANT=$(grep "  $(basename "$PKG")\$" "$WORK/tool/SHA256SUMS" | cut -d' ' -f1)
if [ -z "$WANT" ]; then
  echo "NAMED FAILURE: $(basename "$PKG") is not in $PREV_TAG's SHA256SUMS" >&2
  exit 1
fi
echo "$WANT  $PKG" | sha256sum -c -
chmod +x "$PKG"

# ---- the parts to sign ---------------------------------------------------
# Everything on the release except the indexes (SHA256SUMS, manifest.json)
# and the derived .sha256 sidecars. 49 base parts (6 tools x 7 platforms +
# 7 link-unit tarballs) + the installer containers (roadmap 83) EXACTLY
# when their OS planes armed them: the MSI ships signed-only
# (WINDOWS_SIGNING_ENABLED), the pkgs signed+notarized+stapled or not at
# all (APPLE_INSTALLER_SIGNING_ENABLED + APPLE_SIGNING_ENABLED) — the same
# gate vars the installer legs ship on, so the count never drifts from
# the release's real content.
INSTALLER_PARTS=0
if [ "${WINDOWS_SIGNING_ENABLED:-}" = "true" ]; then
  INSTALLER_PARTS=$(( INSTALLER_PARTS + 1 ))
fi
if [ "${APPLE_INSTALLER_SIGNING_ENABLED:-}" = "true" ] && [ "${APPLE_SIGNING_ENABLED:-}" = "true" ]; then
  INSTALLER_PARTS=$(( INSTALLER_PARTS + 2 ))
fi
EXPECTED_PARTS=$(( 49 + INSTALLER_PARTS ))
EXPECTED_SIGS=$(( 51 + INSTALLER_PARTS ))
gh release download "$TAG" --dir "$WORK/assets" --clobber
mapfile -t PARTS < <(find "$WORK/assets" -maxdepth 1 -type f \
  ! -name 'SHA256SUMS' ! -name 'manifest.json' ! -name '*.sha256' | sort)
if [ "${#PARTS[@]}" -ne "$EXPECTED_PARTS" ]; then
  echo "NAMED FAILURE: expected $EXPECTED_PARTS released parts to sign (49 + $INSTALLER_PARTS installers), found ${#PARTS[@]}" >&2
  printf '  %s\n' "${PARTS[@]}" >&2
  exit 1
fi

# ---- sign: one detached .asc per part + the two index files --------------
# --no-sums everywhere: the canonical SHA256SUMS is finalize.sh's, already
# uploaded; tebako-pkg sign must not regenerate it. out/SHA256SUMS and
# out/manifest.json are byte-identical to the uploaded ones (built here in
# the same job), so their .asc covers the release copies.
( cd "$WORK/assets" && "$OLDPWD/$PKG" sign --key-file "$OLDPWD/$WORK/release-key.asc" --no-sums $(for p in "${PARTS[@]}"; do basename "$p"; done) )
( cd out && "$OLDPWD/$PKG" sign --key-file "$OLDPWD/$WORK/release-key.asc" --no-sums SHA256SUMS manifest.json )

# ---- self-verify: every .asc must verify TRUSTED before any upload -------
# The sign flow registers the signing key in this job's TEBAKO_HOME, so
# verify proves the subkey->root chain resolves, not just the math.
( cd "$WORK/assets" && "$OLDPWD/$PKG" verify $(for p in "${PARTS[@]}"; do basename "$p"; done) )
( cd out && "$OLDPWD/$PKG" verify SHA256SUMS manifest.json )

mv "$WORK"/assets/*.asc out/signatures/
mv out/SHA256SUMS.asc out/manifest.json.asc out/signatures/
COUNT=$(ls out/signatures/*.asc | wc -l)
if [ "$COUNT" -ne "$EXPECTED_SIGS" ]; then
  echo "NAMED FAILURE: expected $EXPECTED_SIGS signatures ($EXPECTED_PARTS parts + SHA256SUMS.asc + manifest.json.asc), have $COUNT" >&2
  exit 1
fi
echo "signed: $COUNT signatures in out/signatures/"
