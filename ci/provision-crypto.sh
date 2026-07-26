#!/bin/bash
# provision-crypto.sh — Botan-3 + json-c for rnp-rs's vendored librnp
# build (rnp 0.18 requires Botan 3; ubuntu 24.04 and Alpine ship only
# Botan 2, so vcpkg is the uniform source). rnp-rs emits `dylib=` links
# for the crypto deps, so the DYNAMIC triplet flavor is installed.
#
# Installs into $CRYPTO_ROOT/<triplet> and, when $GITHUB_ENV is set,
# exports for subsequent steps:
#   RNP_VENDOR_CMAKE_ARGS=CMAKE_PREFIX_PATH=<root>/<triplet>
#   RUSTFLAGS=-L native=<root>/<triplet>/lib
#   LD_LIBRARY_PATH / DYLD_FALLBACK_LIBRARY_PATH (test-time .so lookup)
#
# Required env: TRIPLET (e.g. x64-linux-dynamic), VCPKG_ROOT,
# OVERLAY_TRIPLETS_DIR. Optional: CRYPTO_ROOT (default
# $PWD/.crypto-installed).
set -euo pipefail

TRIPLET="${TRIPLET:?set TRIPLET}"
VCPKG_ROOT="${VCPKG_ROOT:?set VCPKG_ROOT}"
OVERLAY="${OVERLAY_TRIPLETS_DIR:?set OVERLAY_TRIPLETS_DIR}"
ROOT="${CRYPTO_ROOT:-$PWD/.crypto-installed}"

"$VCPKG_ROOT/vcpkg" install botan json-c \
  --vcpkg-root "$VCPKG_ROOT" \
  --x-wait-for-lock \
  --x-install-root "$ROOT" \
  --triplet "$TRIPLET" \
  --overlay-triplets "$OVERLAY"

LIB="$ROOT/$TRIPLET/lib"
if [ -n "${GITHUB_ENV:-}" ]; then
  {
    echo "RNP_VENDOR_CMAKE_ARGS=CMAKE_PREFIX_PATH=$ROOT/$TRIPLET"
    echo "RUSTFLAGS=-L native=$LIB"
    echo "LD_LIBRARY_PATH=$LIB"
    echo "DYLD_FALLBACK_LIBRARY_PATH=$LIB"
  } >> "$GITHUB_ENV"
fi
