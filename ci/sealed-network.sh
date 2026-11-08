#!/usr/bin/env bash
# ci/sealed-network.sh — the sealed-network proof (TODO.sota/05 item 3):
# a machine with NO network builds, verifies, and dispatches a metanorma
# compile from a tebako bundle only.
#
# Two docker phases make the seal airtight by construction — the proof
# never asserts "we did not touch the network", the second phase's
# namespace HAS no network (`--network none`), so a smuggled fetch is not
# a dishonesty, it is impossible:
#
#   phase 1 (networked, linux/amd64): fetch the tebako CLI release asset,
#     `tebako install metanorma` + `tebako bundle metanorma --archive
#     tar.gz` — the bundle is a pre-seeded store, byte-identical with the
#     published artifacts (spec 16 §6's own machinery: registry
#     resolution, sha256 + signature verification at stage time).
#   phase 2 (`--network none`): unpack the bundle in a fresh container,
#     PATH the bundle shims, `metanorma compile` a fixture document —
#     the payload runtime closure resolves from the bundle store,
#     the compile produces XML.
#
# Requires: docker (linux/amd64 emulation on arm64 hosts is fine — the
# seal is the namespace, not the speed). Honest failure, never a silent
# skip: docker absent or the daemon down exits 2 with the reason named.
#
#   ci/sealed-network.sh [metanorma-version]   (default: the registry's
#   default; set TEBAKO_CLI_VERSION to pin the CLI, default v2.8.73)

set -euo pipefail

MN_VERSION="${1:-}"
TEBAKO_CLI_VERSION="${TEBAKO_CLI_VERSION:-v2.8.73}"
export MN_VERSION TEBAKO_CLI_VERSION
WORK="$(cd "$(dirname "$0")/.." && pwd)/out/sealed-network"
IMAGE=debian:12-slim

if ! docker info > /dev/null 2>&1; then
  echo "sealed-network: the docker daemon does not answer — start it and re-run" >&2
  exit 2
fi

rm -rf "$WORK"
mkdir -p "$WORK"

echo "== phase 1: build the bundle (networked, linux/amd64)"
docker run --rm --platform linux/amd64 \
  -v "$WORK:/work" -w /work \
  -e TEBAKO_CLI_VERSION -e MN_VERSION \
  "$IMAGE" bash -euxo pipefail -c '
    apt-get update -qq && apt-get install -y -qq curl ca-certificates > /dev/null
    # The bundle verb copies the CLI tool set from beside the running exe
    # (spec 16 §6: a partial set is a named error) — fetch all four, each
    # sha256-verified against the release sidecar.
    for tool in tebako tebako-shim tfs tebako-pkg; do
      curl -fsSL -o "$tool" "https://github.com/tamatebako/tebako/releases/download/${TEBAKO_CLI_VERSION}/${tool}-${TEBAKO_CLI_VERSION#v}-linux-gnu-x86_64"
      curl -fsSL -o "$tool.sha256" "https://github.com/tamatebako/tebako/releases/download/${TEBAKO_CLI_VERSION}/${tool}-${TEBAKO_CLI_VERSION#v}-linux-gnu-x86_64.sha256"
      echo "$(cut -d" " -f1 "$tool.sha256")  $tool" | sha256sum -c -
      chmod +x "$tool"
    done
    export TEBAKO_HOME=/work/home
    ./tebako add-registry tfs:github:tebako-packages/metanorma
    target="metanorma"
    [ -n "$MN_VERSION" ] && target="metanorma@$MN_VERSION"
    ./tebako install "$target"
    ./tebako bundle "$target" --output /work/bundle --archive tar.gz
    ls -la /work/bundle
  '

echo "== phase 2: the sealed run (--network none — no route exists)"
docker run --rm --platform linux/amd64 --network none \
  -v "$WORK:/work" -w /work \
  "$IMAGE" bash -euxo pipefail -c '
    # The tree root is wherever BUNDLE.yaml sits in the archive (the
    # packers own descriptor is the anchor, never a guessed prefix).
    mkdir -p /work/tree
    tar -xzf bundle.tar.gz -C /work/tree
    bundle_yaml="$(find /work/tree -name BUNDLE.yaml -maxdepth 3 | head -1)"
    [ -n "$bundle_yaml" ] || { echo "no BUNDLE.yaml in the archive — the bundle is malformed"; exit 1; }
    tree="$(dirname "$bundle_yaml")"
    export TEBAKO_HOME="$tree/home"
    export PATH="$tree/home/shims:$tree/bin:$PATH"
    # The fixture: a minimal ISO-flavoured document (the feedstock
    # smoke shape — nodoc/novalid/no-isobib keep the compile offline and
    # fast, which the seal demands anyway).
    # NOTE: the fixture text rides DOUBLE quotes — the inner script is
    # single-quoted in this file; an apostrophe anywhere inside would
    # truncate the script at that byte (the attempt-4 false green).
    printf "= Sealed\nAuthor\n:nodoc:\n:novalid:\n:no-isobib:\n\n== Scope\n\nA sealed network compiles.\n" > sealed.adoc
    metanorma compile -t iso sealed.adoc
    ls -la sealed.xml
    grep -q "<iso-standard" sealed.xml
    echo "SEALED-COMPILE-OK"
  ' | tee "$WORK/phase2.log"
# The verdict is the marker, never the exit code alone: an inner script
# truncated by a quoting slip exits 0 without compiling (attempt 4).
# pipefail keeps a docker failure fatal; the grep names a false green.
grep -q "SEALED-COMPILE-OK" "$WORK/phase2.log" || {
  echo "sealed-network: the sealed run never printed SEALED-COMPILE-OK — the compile did not prove itself (the transcript: $WORK/phase2.log)" >&2
  exit 1
}

echo "sealed-network: PROOF GREEN — a networkless namespace compiled from the bundle"
