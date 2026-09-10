#!/bin/bash
# windows-sign-refrag.sh — spec 34 §1.3 (sign-then-hash): recompute the
# sha256/size fragments for the SIGNED staged exes. stage.sh hashed the
# unsigned bytes (Windows signing is a workflow action — it cannot run
# inside stage.sh the way macos-sign-one.sh does on the macOS legs), so
# after the sign + verify steps this script re-anchors every
# frag-windows-ucrt64 fragment to the signed bytes the finalize job
# merges into SHA256SUMS + manifest.json + the .sha256 sidecars.
#
# Enumerates by EXTENSION (out/*.exe — spec 34 §7.3, never a name list)
# and FAILS LOUDLY when a staged exe has no pre-existing fragment: that
# means stage.sh drifted from this contract.
#
# Required env: VERSION (release version, tag minus v). Optional:
# PLATFORM (defaults to windows-ucrt64).
set -euo pipefail

PLATFORM="${PLATFORM:-windows-ucrt64}"
: "${VERSION:?VERSION is required (the release version, tag minus v)}"

if ! compgen -G "out/*.exe" > /dev/null; then
  echo "::error::spec 34 §7.3: no staged .exe artifacts under out/ — stage.sh drifted?"
  exit 1
fi

echo "| binary | sha256 before (unsigned) | sha256 after (signed) |"
echo "|---|---|---|"
for exe in out/*.exe; do
  base=$(basename "$exe")
  tool="${base%-"$VERSION"-"$PLATFORM".exe}"
  if [ "$tool" = "$base" ]; then
    echo "::error::$base does not match the staged naming <tool>-$VERSION-$PLATFORM.exe — stage.sh drifted?"
    exit 1
  fi
  frag="fragments/frag-$PLATFORM/${tool}-${PLATFORM}"
  if [ ! -f "$frag.sha256" ] || [ ! -f "$frag.size" ]; then
    echo "::error::no fragment for staged $base (expected $frag.sha256 / $frag.size) — stage.sh drifted from the spec 34 §1.3 contract"
    exit 1
  fi
  before=$(cat "$frag.sha256")
  # The same shasum/sha256sum fallback idiom as stage.sh.
  if command -v shasum >/dev/null 2>&1; then
    after=$(shasum -a 256 "$exe" | cut -d' ' -f1)
  else
    after=$(sha256sum "$exe" | cut -d' ' -f1)
  fi
  size=$(stat -c %s "$exe" 2>/dev/null || stat -f %z "$exe")
  echo "$after" > "$frag.sha256"
  echo "$size" > "$frag.size"
  echo "| $base | $before | $after |"
done
echo "spec 34 §1.3: fragments re-anchored to the SIGNED bytes for $PLATFORM"
