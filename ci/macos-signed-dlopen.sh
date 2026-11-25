#!/bin/bash
# macos-signed-dlopen.sh — tebako#989's signed class-L dlopen leg AND
# tebako#1125's signed in-process file-IO leg (the AMFI gate's regression
# nets in the product repo; spec 22 §2's insertion mechanism and §3.6's
# boot-tail shim re-exec, both proven under a REAL Developer ID signature
# on macOS >= 15).
#
# Phases:
#   build   — build the probes (tebako-driver's tebako-dlprobe and
#             tebako-ioprobe bins, sealed at link with the tfs crate's
#             exports.txt), the tfs CLI, and the preload shim cdylib;
#             compile the fixture dylib from its checked-in one-file
#             source; stage both fixture env images with `tfs mkimage`.
#             Runs on every event the job runs on — the disarmed form
#             still proves the build.
#   verify  — sign BOTH probes with the spec 31 §3 entitlement pair
#             (ci/macos-dlprobe.entitlements via macos-sign-one.sh's
#             CODESIGN_ENTITLEMENTS hook), run them against their fixture
#             images, and require the `DLPROBE-OK` + `IOPROBE-OK`
#             verdicts: the insertion fired (the driver's CMS probe read
#             the signature's superblob — tebako#988's regression class),
#             the interposed dlopen routed the in-VFS path, the tail's
#             re-exec armed the shim's file-IO family in the interpreter
#             process itself (tebako#1125 — a raw open(2) of an in-VFS
#             marker answers), and the materialized fixture answered. An
#             armed-but-unsigned probe (a broken signing chain) exits 3
#             here — the leg is red with the verdict in the log, never a
#             silent skip.
#
# The fixture layouts are the contracts shared with
# crates/tebako-driver/tests/interpose_dlopen.rs and
# crates/tebako-driver/tests/ioprobe.rs (which write the same trees
# in-process with limnifs-write — the local, unsigned truth):
#
#   lib/tebako/layout.yaml                  era-2 layout declaration
#   lib/dlprobe/libdlprobe_fixture.dylib    dlprobe_answer() -> 0xD10CCAFE0989
#
#   lib/tebako/layout.yaml                  era-2 declaration + the
#                                           preload_shim key
#   lib/tebako/libtfs_preload.dylib         the real preload shim cdylib
#   lib/shimprobe/marker.txt                ioprobe-marker-0x1125
#
# Runner env: the job's cargo/vcpkg env for build; CODESIGN_HASH +
# CODESIGN_KEYCHAIN + APPLE_TEAM_ID (macos-sign-setup.sh's exports) for
# verify. Usage: macos-signed-dlopen.sh build|verify
set -euo pipefail

phase="${1:-}"
work="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/tebako-dlprobe"
stage="$work/image-root"
iostage="$work/image-root-io"
image="$work/env.tfs"
ioimage="$work/env-io.tfs"
# CI builds into the workspace's target/; a local rehearsal may point
# CARGO_TARGET_DIR elsewhere — honor it.
target="${CARGO_TARGET_DIR:-target}"
probe="$target/debug/tebako-dlprobe"
ioprobe="$target/debug/tebako-ioprobe"
shim="$target/debug/libtfs_preload.dylib"
want="DLPROBE-OK 0xd10ccafe0989"
iowant="IOPROBE-OK ioprobe-marker-0x1125"

case "$phase" in
  build)
    cargo build -p tebako-driver -p tfs-cli -p libtfs-preload

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

    # The tebako#1125 fixture: the era-2 declaration WITH the
    # schema_minor-2 preload_shim key (byte-identical with
    # tests/ioprobe.rs's LAYOUT), the real shim cdylib at the declared
    # path, and the marker file the probe's raw open answers.
    mkdir -p "$iostage/lib/tebako" "$iostage/lib/shimprobe"
    cat > "$iostage/lib/tebako/layout.yaml" <<'EOF'
schema_version: 1
era: 2
image_layout: 1
mount_root: /__tfs__
interpreter_api_version: "3.4"
preload_shim: lib/tebako/libtfs_preload.dylib
EOF
    [ -f "$shim" ] || { echo "::error::tebako#1125: no preload shim at $shim — the -p libtfs-preload build above must produce it"; exit 1; }
    cp "$shim" "$iostage/lib/tebako/libtfs_preload.dylib"
    printf 'ioprobe-marker-0x1125' > "$iostage/lib/shimprobe/marker.txt"
    "$target/debug/tfs" mkimage "$iostage" --output "$ioimage" -v
    echo "dlprobe leg: probes + fixture images built ($image, $ioimage)"
    ;;

  verify)
    CODESIGN_ENTITLEMENTS=ci/macos-dlprobe.entitlements \
      bash ci/macos-sign-one.sh "$probe"
    CODESIGN_ENTITLEMENTS=ci/macos-dlprobe.entitlements \
      bash ci/macos-sign-one.sh "$ioprobe"
    # The profile is the leg's premise: a hardened-runtime exe without
    # the pair never sees the insertion (dyld strips the variable) or
    # never loads the materialized code (library validation) — assert it
    # HERE, not in the verdict's ambiguity.
    for exe in "$probe" "$ioprobe"; do
      codesign -d --entitlements - "$exe" > "$work/entitlements.xml" 2>&1
      grep -q "allow-dyld-environment-variables" "$work/entitlements.xml" \
        || { echo "::error::tebako#989: the signed $exe lacks allow-dyld-environment-variables"; exit 1; }
      grep -q "disable-library-validation" "$work/entitlements.xml" \
        || { echo "::error::tebako#989: the signed $exe lacks disable-library-validation"; exit 1; }
    done

    rc=0
    out=$("$probe" "$image" 2>"$work/probe.stderr") || rc=$?
    cat "$work/probe.stderr"
    echo "$out"
    if [ "$rc" -ne 0 ] || ! printf '%s\n' "$out" | grep -q "^$want$"; then
      echo "::error::tebako#989: the signed probe answered rc=$rc, want exit 0 with '$want' — the AMFI gate's regression net is red"
      exit 1
    fi
    echo "tebako#989: signed class-L dlopen leg green — $want"

    rc=0
    out=$("$ioprobe" "$ioimage" 2>"$work/ioprobe.stderr") || rc=$?
    cat "$work/ioprobe.stderr"
    echo "$out"
    if [ "$rc" -ne 0 ] || ! printf '%s\n' "$out" | grep -q "^$iowant$"; then
      echo "::error::tebako#1125: the signed io probe answered rc=$rc, want exit 0 with '$iowant' — the in-process file-IO regression net is red"
      exit 1
    fi
    echo "tebako#1125: signed in-process file-IO leg green — $iowant"
    ;;

  *)
    echo "usage: macos-signed-dlopen.sh build|verify" >&2
    exit 64
    ;;
esac
