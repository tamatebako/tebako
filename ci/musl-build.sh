#!/bin/sh
# musl-build.sh — the Linux-musl release build, run INSIDE an Alpine
# container (docker run from the glibc runner; node actions cannot run
# on musl). Native-musl path: apk toolchain, vcpkg bootstrapped from
# source with the musl overlay triplets (flatc and every other build
# tool is built for musl and runs natively — the reason this is not a
# cargo-zigbuild leg; see the item-31 notes in the release workflow).
#
# Required env: VERSION (tag minus v), PLATFORM (tebako platform id),
# TARGET (rust target triple), TRIPLET (vcpkg overlay triplet),
# VCPKG_COMMIT, RUST_VERSION. Optional: BOOTSTRAP_SIZE_BUDGET.
set -eu

WS="${GITHUB_WORKSPACE:-/ws}"
cd "$WS"

echo "== apk toolchain =="
apk --no-cache add \
  build-base cmake ninja git bash sudo \
  autoconf automake libtool make pkgconfig perl python3 \
  curl zip unzip tar ca-certificates linux-headers

echo "== rustup ($RUST_VERSION, $TARGET) =="
curl -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain "$RUST_VERSION" --target "$TARGET"
. "$HOME/.cargo/env"
rustc --version

echo "== vcpkg bootstrap ($VCPKG_COMMIT) =="
git clone --quiet https://github.com/microsoft/vcpkg "$WS/.vcpkg-musl"
git -C "$WS/.vcpkg-musl" checkout --quiet "$VCPKG_COMMIT"
"$WS/.vcpkg-musl/bootstrap-vcpkg.sh" -disableMetrics

# The musl overlay triplets live in dwarfs-t (x64) and are injected here
# for arm64 + sqfs (upstreaming is a follow-up; see the release notes).
SQFS_TRIPLETS="$WS/tebako-rs/crates/sqfs-sys/vcpkg_triplets"
DWARFS_TRIPLETS="$WS/dwarfs-rs/dwarfs-t/vcpkg_triplets"
if [ "$TRIPLET" = "arm64-linux-musl" ]; then
  sed -e 's/VCPKG_TARGET_ARCHITECTURE x64/VCPKG_TARGET_ARCHITECTURE arm64/' \
      "$DWARFS_TRIPLETS/x64-linux-musl.cmake" > "$DWARFS_TRIPLETS/arm64-linux-musl.cmake"
fi
if [ ! -f "$SQFS_TRIPLETS/$TRIPLET.cmake" ]; then
  sed -e "s/VCPKG_TARGET_ARCHITECTURE x64/VCPKG_TARGET_ARCHITECTURE $( [ "$TRIPLET" = "arm64-linux-musl" ] && echo arm64 || echo x64 )/" \
      "$DWARFS_TRIPLETS/x64-linux-musl.cmake" > "$SQFS_TRIPLETS/$TRIPLET.cmake"
fi

echo "== pre-install squashfs-tools-ng ($TRIPLET) =="
"$WS/.vcpkg-musl/vcpkg" install \
  --vcpkg-root "$WS/.vcpkg-musl" \
  --x-wait-for-lock \
  --x-manifest-root "$WS/tebako-rs/crates/sqfs-sys" \
  --x-install-root "$WS/.sqfs-musl" \
  --triplet "$TRIPLET" \
  --overlay-triplets "$SQFS_TRIPLETS" \
  --overlay-ports "$WS/tebako-rs/crates/sqfs-sys/vcpkg_ports"

echo "== cargo build (release, $TARGET) =="
cd "$WS/tebako-rs"
export DWARFS_RS_VCPKG_ROOT="$WS/.vcpkg-musl"
export DWARFS_RS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_INSTALLED_DIR="$WS/.sqfs-musl/$TRIPLET"
cargo build --release --target "$TARGET" \
  -p tebako-bootstrap -p tfs-cli -p tebako-pkg -p tebako-cli

echo "== stage / strip / size-gate =="
export TARGET
bash .github/workflows/lib/stage.sh
