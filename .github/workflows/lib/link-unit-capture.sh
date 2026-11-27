#!/bin/bash
# link-unit-capture.sh — spec 15 §8a (tebako#1121): emit the build-time
# crate-inventory capture INTO the staged link-unit tree, BEFORE the tar
# — so the shipped tarball carries crate-inventory.json beside the
# staticlibs whose vendored crate set it describes. One hash (the
# tarball's) then covers both: a runtime factory's exe SBOM
# (`tebako sbom --runtime-exe --crates …`) can never silently drift from
# the build's real resolution, on the pin-hit lane and the source-build
# lane alike (the factory runs sbom-cargo --inventory-only itself on the
# latter).
#
# Usage: link-unit-capture.sh <staged-link-unit-dir>
# Optional env: TARGET (rust triple; unset = the musl leg's
# default-host layout — its host triple IS the target), EXE_SUFFIX
# (.exe on the windows legs).
set -euo pipefail

dir="${1:?usage: link-unit-capture.sh <staged-link-unit-dir>}"
TARGET="${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
build_dir="target/$TARGET/release"
if [ ! -d "$build_dir" ]; then
  build_dir="target/release"
fi

# The generator, built once per leg (sbom-stage.sh's lazy build reuses
# this same path — a second build is a cargo cache hit).
SBOM_CARGO="$build_dir/sbom-cargo${EXE_SUFFIX:-}"
if [ ! -x "$SBOM_CARGO" ]; then
  if [ "$build_dir" = "target/$TARGET/release" ]; then
    cargo build --release --target "$TARGET" -p sbom-cargo
  else
    cargo build --release -p sbom-cargo
  fi
fi

# The leg's real Cargo resolution — the same inputs sbom-stage.sh's
# document emission consumes, so the capture and the tarball's own
# CycloneDX document are two serializations of ONE resolution.
mkdir -p sbom-work
cargo metadata --format-version 1 --filter-platform "$TARGET" > sbom-work/metadata.json
"$SBOM_CARGO" --inventory-only \
  --package tebako-driver \
  --target "$TARGET" \
  --metadata sbom-work/metadata.json \
  --lock Cargo.lock \
  --out "$dir/crate-inventory.json"
