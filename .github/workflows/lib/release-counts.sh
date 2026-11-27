#!/bin/bash
# release-counts.sh — the single owner of the release's asset arithmetic.
# Sourced by lib/sign-release.sh, release.yml's completeness gate, and
# repair-sign.yml. Never copy these numbers into another file.
#
# Reads the gate env vars: WINDOWS_SIGNING_ENABLED, APPLE_SIGNING_ENABLED,
# APPLE_INSTALLER_SIGNING_ENABLED, TEBAKO_RELEASE_SIGNING_ENABLED.

# The installer containers join the release exactly when their OS planes
# armed them (the same gate vars the installer legs ship on): the MSI
# ships signed-only; the pkgs signed+notarized+stapled or not at all.
release_installer_count() {
  local n=0
  [ "${WINDOWS_SIGNING_ENABLED:-}" = "true" ] && n=$(( n + 1 ))
  if [ "${APPLE_INSTALLER_SIGNING_ENABLED:-}" = "true" ] && [ "${APPLE_SIGNING_ENABLED:-}" = "true" ]; then
    n=$(( n + 2 ))
  fi
  echo "$n"
}

# Files the signer signs: 6 tools x 7 platforms + 7 link-unit tarballs +
# the windows-arm64 tfs.exe + tebako-pkg.exe + tebako-runtime-launcher.exe
# + their link-unit tarball (roadmap 02 phase 2; the launcher joined with
# v2.8.46 — tebako#830, the repack factories' pair half) + the preload
# shim cdylib x 6 POSIX platforms (tebako#1099 — windows has no preload
# delivery) + one CycloneDX SBOM per non-installer part (59 — spec 13
# §5a, tebako#1120) + the armed installers. (SHA256SUMS + manifest.json
# sign separately.)
release_expected_parts() {
  echo $(( 59 + 59 + $(release_installer_count) ))
}

# One .asc per part + SHA256SUMS.asc + manifest.json.asc.
release_expected_sigs() {
  echo $(( $(release_expected_parts) + 2 ))
}

# Every asset on the release: the parts + the 51 per-tool .sha256
# sidecars (tebako#493 — the link-unit tarballs have none; the arm64
# launcher and the six preload shims carry one like every tool; the
# SBOMs get NO sidecars — spec 13 §5a: a SBOM's anchor is its signed
# SHA256SUMS row + its own .asc) + the two indexes + one .sha256 sidecar
# per installer + the .asc set when the signing plane is armed.
release_expected_assets() {
  local installers n
  installers=$(release_installer_count)
  n=$(( 59 + 59 + 51 + 2 + installers * 2 ))
  if [ "${TEBAKO_RELEASE_SIGNING_ENABLED:-}" = "true" ]; then
    n=$(( n + $(release_expected_sigs) ))
  fi
  echo "$n"
}
