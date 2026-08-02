#!/bin/bash
# link-unit-stage.sh — stage the v2 link unit for one POSIX release leg:
# bridge the pre-installed squashfs tree into the shape the closure
# harvest expects, run tools/stage_link_unit with --skip-build (the libs
# were built by the leg's own build step), and tar the result.
#
# Required env: VERSION (tag minus v), PLATFORM (tebako platform id).
# Optional: TARGET (rust triple — when set, the leg built with --target
# and the tool must be told; unset = the cargo default-target layout
# target/release, e.g. the musl leg's no---target build),
# SQFS_SYS_VCPKG_INSTALLED_DIR.
set -euo pipefail

TARGET="${TARGET:-}"
build_dir="target/release"
target_args=()
if [ -n "$TARGET" ]; then
  build_dir="target/$TARGET/release"
  target_args=(--target "$TARGET")
fi

# sqfs-sys short-circuits its own vcpkg install when
# SQFS_SYS_VCPKG_INSTALLED_DIR points at a serialized pre-install
# (parallel build scripts would otherwise race dwarfs-t-sys on the
# vcpkg-root lock) — its <out>/vcpkg_installed then stays empty, and the
# harvest's <sqfs-out>/vcpkg_installed/{*-static,*-linux-musl}/lib glob
# finds no libsquashfs.a (run 30745532174, both macOS legs). Bridge the
# pre-installed tree into the newest sqfs-sys out dir (same newest-mtime
# pick the tool makes).
if [ -n "${SQFS_SYS_VCPKG_INSTALLED_DIR:-}" ]; then
  sqfs_out=$(ls -dt "$build_dir"/build/sqfs-sys-*/out 2>/dev/null | head -1 || true)
  if [ -z "$sqfs_out" ]; then
    echo "link-unit-stage: no sqfs-sys build under $build_dir/build (build the libs first)" >&2
    exit 64
  fi
  trip=$(basename "$SQFS_SYS_VCPKG_INSTALLED_DIR")
  if [ ! -e "$sqfs_out/vcpkg_installed/$trip/lib" ]; then
    mkdir -p "$sqfs_out/vcpkg_installed"
    ln -sfn "$SQFS_SYS_VCPKG_INSTALLED_DIR" "$sqfs_out/vcpkg_installed/$trip"
    echo "link-unit-stage: bridged $SQFS_SYS_VCPKG_INSTALLED_DIR -> $sqfs_out/vcpkg_installed/$trip"
  fi
fi

mkdir -p "out/link-unit-$PLATFORM"
# bash 3.2 (the macOS runners' /bin/bash) calls an empty "${array[@]}"
# unbound under set -u — the [@]+ guard expands to nothing instead.
ruby tools/stage_link_unit "out/link-unit-$PLATFORM" ${target_args[@]+"${target_args[@]}"} --skip-build
tar -czf "out/link-unit-${VERSION}-${PLATFORM}.tar.gz" -C out "link-unit-$PLATFORM"
