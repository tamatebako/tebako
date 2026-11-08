#!/bin/bash
# macos-signed-dlopen.sh — tebako#989's signed class-L dlopen leg (the
# AMFI gate's regression net in the product repo; spec 22 §2's insertion
# mechanism proven under a REAL Developer ID signature on macOS >= 15).
#
# Phases:
#   build   — build the probe (tebako-driver's tebako-dlprobe bin, sealed
#             at link with the tfs crate's exports.txt) and the tfs CLI,
#             compile the fixture dylib from its checked-in one-file
#             source, and stage the fixture env image with `tfs mkimage`.
#             Runs on every event the job runs on — the disarmed form
#             still proves the build.
#   verify  — sign the probe with the spec 31 §3 entitlement pair
#             (ci/macos-dlprobe.entitlements via macos-sign-one.sh's
#             CODESIGN_ENTITLEMENTS hook), run it against the fixture
#             image, and require the `DLPROBE-OK` verdict: the insertion
#             fired (the driver's CMS probe read the signature's
#             superblob — tebako#988's regression class), the interposed
#             dlopen routed the in-VFS path, and the materialized fixture
#             answered. An armed-but-unsigned probe (a broken signing
#             chain) exits 3 (DLPROBE-UNINSERTED) here — the leg is red
#             with the verdict in the log, never a silent skip.
#
# The fixture layout is the contract shared with
# crates/tebako-driver/tests/interpose_dlopen.rs (which writes the same
# tree in-process with limnifs-write — the local, unsigned truth):
#
#   lib/tebako/layout.yaml                  era-2 layout declaration
#   lib/dlprobe/libdlprobe_fixture.dylib    dlprobe_answer() -> 0xD10CCAFE0989
#
# Runner env: the job's cargo/vcpkg env for build; CODESIGN_HASH +
# CODESIGN_KEYCHAIN + APPLE_TEAM_ID (macos-sign-setup.sh's exports) for
# verify. Usage: macos-signed-dlopen.sh build|verify
set -euo pipefail

phase="${1:-}"
work="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/tebako-dlprobe"
stage="$work/image-root"
image="$work/env.tfs"
# CI builds into the workspace's target/; a local rehearsal may point
# CARGO_TARGET_DIR elsewhere — honor it.
target="${CARGO_TARGET_DIR:-target}"
probe="$target/debug/tebako-dlprobe"
want="DLPROBE-OK 0xd10ccafe0989"

case "$phase" in
  build)
    cargo build -p tebako-driver -p tfs-cli

    rm -rf "$work"
    mkdir -p "$stage/lib/tebako" "$stage/lib/dlprobe"
    # The era-2 layout declaration (spec 18 C3) — byte-identical with the
    # integration test's GOOD_LAYOUT constant.
    cat > "$stage/lib/tebako/layout.yaml" <<'EOF'
schema_version: 1
era: 2
image_layout: 1
mount_root: /__tfs__
interpreter_api_version: "3.4"
EOF
    rustc --edition 2021 --crate-type cdylib \
      crates/tebako-driver/tests/fixtures/dlprobe_fixture.rs \
      -o "$stage/lib/dlprobe/libdlprobe_fixture.dylib"
    # limnifs is mkimage's default format (spec 20 §6) — no --format pin.
    "$target/debug/tfs" mkimage "$stage" --output "$image" -v
    echo "dlprobe leg: probe + fixture image built ($image)"
    ;;

  verify)
    CODESIGN_ENTITLEMENTS=ci/macos-dlprobe.entitlements \
      bash ci/macos-sign-one.sh "$probe"
    # The profile is the leg's premise: a hardened-runtime exe without
    # the pair never sees the insertion (dyld strips the variable) or
    # never loads the materialized code (library validation) — assert it
    # HERE, not in the verdict's ambiguity.
    codesign -d --entitlements - "$probe" > "$work/entitlements.xml" 2>&1
    grep -q "allow-dyld-environment-variables" "$work/entitlements.xml" \
      || { echo "::error::tebako#989: the signed probe lacks allow-dyld-environment-variables"; exit 1; }
    grep -q "disable-library-validation" "$work/entitlements.xml" \
      || { echo "::error::tebako#989: the signed probe lacks disable-library-validation"; exit 1; }

    rc=0
    out=$("$probe" "$image" 2>"$work/probe.stderr") || rc=$?
    cat "$work/probe.stderr"
    echo "$out"
    if [ "$rc" -ne 0 ] || ! printf '%s\n' "$out" | grep -q "^$want$"; then
      echo "::error::tebako#989: the signed probe answered rc=$rc, want exit 0 with '$want' — the AMFI gate's regression net is red"
      exit 1
    fi
    echo "tebako#989: signed class-L dlopen leg green — $want"
    ;;

  *)
    echo "usage: macos-signed-dlopen.sh build|verify" >&2
    exit 64
    ;;
esac
