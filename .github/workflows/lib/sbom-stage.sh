#!/bin/bash
# sbom-stage.sh — spec 13 §5a (tebako#1120): one CycloneDX SBOM per
# staged release part, generated at stage time from the leg's REAL Cargo
# resolution. Runs after stage.sh (and link-unit-stage.sh when the leg
# carries the link unit): the staged bytes are final by then
# (sign-then-hash — the digest the SBOM claims is the shipped digest).
#
# The crate set is the target-filtered resolve graph
# (cargo metadata --filter-platform) walked from each tool's own
# workspace member, joined with the build's Cargo.lock checksums — the
# per-target feature shapes ride the manifests' cfg-gated declarations
# (windows' dwarfs-only / arm64's limnifs-only arms), so the default
# metadata resolution is honest with no CLI overrides. The document
# emits through the sbom-cargo crate (the tebako-sbom model's release
# half) — one CycloneDX owner, deterministic: SOURCE_DATE_EPOCH pins
# the tag commit's epoch (prepare's sbom_epoch output).
#
# Required env: VERSION, PLATFORM (the tebako platform id), TOOLS
# (stage.sh's grammar — space-separated tool ids). Optional: TARGET
# (rust triple; unset = the musl leg's default-host layout), EXE_SUFFIX
# (.exe on windows legs), LINK_UNIT=1 (also SBOM the link-unit
# tarball), LINK_UNIT_PLATFORM (the link-unit tarball's name token when
# it is NOT the leg's platform id — the windows-arm64 leg ships
# link-unit-<VERSION>-aarch64-windows-gnu.tar.gz; default $PLATFORM),
# SBOM_EPOCH (prepare's output — pins SOURCE_DATE_EPOCH),
# RELEASE_URL (the tag's download base — attestation/distribution
# external references; empty on rehearsals).
#
# Fragments written: fragments/frag-$PLATFORM/<stem>.cdx.sha256 (the
# finalize merge's SHA256SUMS rows — no sidecars, spec 13 §5a). The
# frag dir is always the leg's own; the <stem> carries each part's name
# token so finalize reads the arm64 link unit's divergent spelling back
# off the file name.
set -euo pipefail

VERSION="${VERSION:?VERSION is required}"
PLATFORM="${PLATFORM:?PLATFORM is required}"
TOOLS="${TOOLS:-}"
LINK_UNIT="${LINK_UNIT:-0}"
LINK_UNIT_PLATFORM="${LINK_UNIT_PLATFORM:-$PLATFORM}"
EXE="${EXE_SUFFIX:-}"

# The cargo target the leg built for (the musl leg builds native with no
# --target — its host triple IS the target).
TARGET="${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
TARGET_DIR="target/$TARGET/release"
if [ ! -d "$TARGET_DIR" ]; then
  TARGET_DIR="target/release"
fi

if [ "${SBOM_EPOCH:-}" != "" ]; then
  export SOURCE_DATE_EPOCH="$SBOM_EPOCH"
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    openssl dgst -sha256 -r "$1" | cut -d' ' -f1
  fi
}

# tool id (the asset stem) → the cargo package whose resolve closure
# describes the asset. libtfs_preload is the one underscore-shaped id.
cargo_package_of() {
  case "$1" in
    tfs) echo tfs-cli ;;
    tebako) echo tebako-cli ;;
    libtfs_preload) echo libtfs-preload ;;
    link-unit) echo tebako-driver ;;
    *) echo "$1" ;;
  esac
}

# The staged asset's file name for a tool (asset_name's sibling, the
# finalize.sh grammar): windows names carry .exe, the preload shim its
# library extension, the link unit .tar.gz. $2 is the name token —
# the leg's platform id for every tool; the link unit's divergent
# tarball spelling on the windows-arm64 leg (aarch64-windows-gnu).
staged_name_of() {
  local tool="$1" token="$2" ext="$EXE"
  if [ "$tool" = libtfs_preload ]; then
    case "$token" in
      macos-*) ext=".dylib" ;;
      linux-*) ext=".so" ;;
    esac
  fi
  if [ "$tool" = link-unit ]; then
    ext=".tar.gz"
  fi
  echo "${tool}-${VERSION}-${token}${ext}"
}

# The SBOM asset's file name: the staged name minus its
# .exe/.dylib/.so/.tar.gz suffix, plus .cdx.json (spec 13 §5a's
# spelling — the metanorma template's shape).
sbom_name_of() {
  local staged
  staged=$(staged_name_of "$1" "$2")
  staged="${staged%.exe}"
  staged="${staged%.dylib}"
  staged="${staged%.so}"
  staged="${staged%.tar.gz}"
  echo "${staged}.cdx.json"
}

# The generator, built once for the leg. A separate invocation from the
# product builds by design (the bootstrap's feature-isolation law is
# about invocations — this leaf crate changes nothing for the shipped
# exes, and building it beside them would only cost them its deps).
SBOM_CARGO="$TARGET_DIR/sbom-cargo${EXE}"
if [ ! -x "$SBOM_CARGO" ]; then
  if [ "$TARGET_DIR" = "target/$TARGET/release" ]; then
    cargo build --release --target "$TARGET" -p sbom-cargo
  else
    cargo build --release -p sbom-cargo
  fi
fi

# One metadata document for the whole leg (the target-filtered resolve
# is identical for every tool — only the walk root differs).
WORK="sbom-work"
mkdir -p "$WORK" "fragments/frag-$PLATFORM"
cargo metadata --format-version 1 --filter-platform "$TARGET" > "$WORK/metadata.json"

stage_sbom_one() { # $1 = tool id (or link-unit), $2 = the name token
  local tool="$1" token="$2" package staged_name staged sbom_name
  package=$(cargo_package_of "$tool")
  staged_name=$(staged_name_of "$tool" "$token")
  staged="out/$staged_name"
  sbom_name=$(sbom_name_of "$tool" "$token")
  if [ ! -f "$staged" ]; then
    echo "::error::sbom-stage: expected staged asset $staged for $tool — the leg's stage step did not produce it" >&2
    exit 1
  fi
  "$SBOM_CARGO" \
    --package "$package" \
    --tool "$tool" \
    --asset "$staged" \
    --name "$staged_name" \
    --version "$VERSION" \
    --platform "$token" \
    --target "$TARGET" \
    --metadata "$WORK/metadata.json" \
    --lock Cargo.lock \
    --out "out/$sbom_name" \
    ${RELEASE_URL:+--release-url "$RELEASE_URL"}
  # The finalize merge's SHA256SUMS row fragment (no sidecar — spec 13
  # §5a: the SBOM's own anchor is the signed SHA256SUMS row + its .asc).
  # The stem carries the name token so the merge reads the spelling back
  # off the file name (the arm64 link unit's is not the platform id).
  sha256_of "out/$sbom_name" > "fragments/frag-$PLATFORM/${tool}-${token}.cdx.sha256"
}

for tool in $TOOLS; do
  stage_sbom_one "$tool" "$PLATFORM"
done
if [ "$LINK_UNIT" = "1" ]; then
  stage_sbom_one link-unit "$LINK_UNIT_PLATFORM"
fi

echo "sbom-stage: staged SBOMs for $PLATFORM:"
ls -la out/*.cdx.json
