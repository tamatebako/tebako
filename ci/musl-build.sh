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
  curl zip unzip tar ca-certificates linux-headers \
  clang19-libclang ruby

# The workspace is bind-mounted from the runner (files owned by uid 1001)
# but the build runs as root — alpine's git (>= 2.35.2) refuses the
# mounted repos as "dubious ownership", so dwarfs-t's cmake/version.cmake
# sees NO git metadata and dies ("missing version files - git metadata
# unavailable and no pre-generated version files found"; release run
# 30742821370). This is a single-use --rm container: trust every repo
# under the mount.
git config --global --add safe.directory '*'

# clang19-libclang: rnp-rs 0.1.10's build.rs runs bindgen (the rnp-src
# source-built model) — bindgen dlopens libclang.so at runtime, which
# only the versioned libclang package ships on alpine.

# vcpkg's bootstrap downloads glibc-linked cmake/ninja by default — they
# cannot run on musl (exit 127). Use the apk-provided tools everywhere.
export VCPKG_FORCE_SYSTEM_BINARIES=1

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
# for arm64 + sqfs + the crypto-dynamic flavor (upstreaming is a
# follow-up; see the release notes).
SQFS_TRIPLETS="$WS/tebako-rs/crates/sqfs-sys/vcpkg_triplets"
DWARFS_TRIPLETS="$WS/dwarfs-rs/dwarfs-t/vcpkg_triplets"
mk_triplet() {  # $1 = new name, $2 = arch (x64|arm64), $3 = linkage (static|dynamic)
  # CRT linkage must follow library linkage: with VCPKG_CRT_LINKAGE static,
  # vcpkg's Linux toolchain appends -static to CMAKE_*_LINKER_FLAGS, which
  # vcpkg_configure_make exports as LDFLAGS and breaks shared-library links
  # (botan:x64-linux-musl-dynamic failed with crtbeginT.o/__TMC_END__
  # relocation errors; release run 30217620407).
  crt=static; [ "$3" = dynamic ] && crt=dynamic
  sed -e "s/VCPKG_TARGET_ARCHITECTURE x64/VCPKG_TARGET_ARCHITECTURE $2/" \
      -e "s/VCPKG_LIBRARY_LINKAGE static/VCPKG_LIBRARY_LINKAGE $3/" \
      -e "s/VCPKG_CRT_LINKAGE static/VCPKG_CRT_LINKAGE $crt/" \
      "$DWARFS_TRIPLETS/x64-linux-musl.cmake" > "$DWARFS_TRIPLETS/$1.cmake"
}
[ -f "$DWARFS_TRIPLETS/$TRIPLET.cmake" ] || {
  case "$TRIPLET" in
    arm64-linux-musl) mk_triplet "$TRIPLET" arm64 static ;;
  esac
}
# The musl-dynamic triplets are checked in under dwarfs-t/vcpkg_triplets
# (with the CRT fix above baked in); only generate on the fly as a fallback.
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

# Crypto comes from crates: rnp-rs 0.1.10's vendored mode builds librnp +
# Botan + json-c + zlib + bzip2 from source via rnp-src (crates.io 0.1.2,
# the rnpgp/rnp-rs#63 fix).
#
# musl targets default to +crt-static, and a statically linked build
# script cannot dlopen — but rnp-rs's build.rs runs bindgen, which
# dlopens libclang. Build the musl legs with -crt-static OFF: the
# artifacts are dynamic-musl, same shape as the runtime factory's musl
# runtimes (musl libc is present on every musl system by definition;
# the symbol floor is documented alongside them).
#
# The flag must reach HOST units (build scripts) too — and with an
# explicit `--target` cargo applies RUSTFLAGS to the target units ONLY
# (proven against cargo 1.94: build scripts get neither RUSTFLAGS nor
# CARGO_TARGET_<host>_RUSTFLAGS once --target is passed), so rnp-rs's
# build script linked static and bindgen died on "Dynamic loading not
# supported" (run 30745532174). Every musl leg is native (the alpine
# container's host triple IS $TARGET), so build WITHOUT --target:
# RUSTFLAGS then covers host units, and the artifacts land in the
# default-target layout target/release (stage.sh and link-unit-stage.sh
# resolve both layouts).
# ...but the toolchain runtimes are NOT part of that contract: a NEEDED
# libstdc++.so.6/libgcc_s.so.1 is exit 127 on a vanilla alpine (proven by
# the 0.1.1 musl assets). Absorb them STATICALLY: the trailing-flags
# trick cannot (rustc emits a build script's -lstdc++/-lgcc_s before the
# -C link-arg tail — see ci/linux-link-wrap.sh), so the driver-boundary
# wrapper rewrites those tokens. Engaged via -C linker INSIDE RUSTFLAGS:
# this leg builds without --target, and cargo honors neither
# CARGO_TARGET_<host-triple>_LINKER nor target-unit-only flags for the
# shipped exes in that mode (proven on alpine:3.21 cargo) — RUSTFLAGS is
# the one channel that covers every unit here. The wrapper's rewrites
# are harmless to build-script links. The release leg's ship gate (the
# musl ldd whitelist in .github/workflows/lib/ship-gate.sh) is the
# enforcement: libc.musl ONLY.
export RUSTFLAGS="-C target-feature=-crt-static -C linker=$WS/tebako-rs/ci/linux-link-wrap.sh"
echo "== cargo build (release, $TARGET — native, no --target) =="
cd "$WS/tebako-rs"
export DWARFS_RS_VCPKG_ROOT="$WS/.vcpkg-musl"
export DWARFS_RS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_INSTALLED_DIR="$WS/.sqfs-musl/$TRIPLET"
cargo build --release \
  -p tebako-bootstrap -p tfs-cli -p tebako-pkg -p tebako-cli -p tebako-shim

echo "== stage / strip / size-gate =="
export TARGET
bash .github/workflows/lib/stage.sh

echo "== link unit (tebako-driver + tfs, scoped, + closure) =="
cargo build --release -p tfs -p tebako-driver -p libtfs-preload
TARGET= bash .github/workflows/lib/link-unit-stage.sh
