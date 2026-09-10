#!/bin/bash
# windows-sign-gate.sh — the spec 34 §4 enablement gate for the Windows
# release legs. The repo variable WINDOWS_SIGNING_ENABLED=true arms
# Authenticode signing; anything else ships unsigned BY DESIGN (spec 00
# invariant 7: unsigned stays first-class) and this step is a loud
# notice. Armed but a secret/variable unresolved is a FAST named failure
# BEFORE any signing call — never a partial release (spec 34 §7.5; the
# same discipline as ci/macos-sign-setup.sh's spec 31 gate).
#
# Emits armed=<true|false> to GITHUB_OUTPUT; the leg's azure/login,
# artifact-signing, signtool-verify and re-hash steps all gate on it.
#
# Required env when armed (spec 34 §3): AZURE_CLIENT_ID,
# AZURE_TENANT_ID, AZURE_SUBSCRIPTION_ID (org secrets — the OIDC
# identity; no AZURE_CLIENT_SECRET exists anywhere), AZURE_ENDPOINT,
# AZURE_SIGNING_ACCOUNT_NAME, AZURE_SIGNING_CERT_PROFILE (org
# variables). Runner env: WINDOWS_SIGNING_ENABLED (from vars.),
# GITHUB_OUTPUT.
set -euo pipefail

if [ "${WINDOWS_SIGNING_ENABLED:-false}" != "true" ]; then
  echo "::notice::spec 34: WINDOWS_SIGNING_ENABLED != true — this leg ships UNSIGNED (spec 00 invariant 7)"
  echo "armed=false" >> "$GITHUB_OUTPUT"
  exit 0
fi

missing=""
for v in AZURE_CLIENT_ID AZURE_TENANT_ID AZURE_SUBSCRIPTION_ID \
         AZURE_ENDPOINT AZURE_SIGNING_ACCOUNT_NAME AZURE_SIGNING_CERT_PROFILE; do
  [ -n "${!v:-}" ] || missing="$missing $v"
done
if [ -n "$missing" ]; then
  echo "::error::WINDOWS_SIGNING_ENABLED=true but unset:$missing — spec 34 §7.5: fast failure before any signing call, never a partial release"
  exit 1
fi

echo "armed=true" >> "$GITHUB_OUTPUT"
echo "spec 34: signing armed — Artifact Signing account $AZURE_SIGNING_ACCOUNT_NAME / profile $AZURE_SIGNING_CERT_PROFILE ($AZURE_ENDPOINT)"
