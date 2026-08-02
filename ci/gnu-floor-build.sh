#!/bin/bash
# gnu-floor-build.sh — the linux-gnu release build, run INSIDE an
# ubuntu:20.04 container (docker run from the 24.04 runner). The floor
# rule (spec 19 §3): glibc symbol versions are one-directional, so a
# binary built on 24.04 (glibc 2.39) refuses to start anywhere older
# than 2.39. Building on the 20.04 floor (glibc 2.31) makes the
# artifact run on 20.04 and everything newer.
#
# Required env: VERSION (tag minus v), PLATFORM (tebako platform id),
# TARGET (rust target triple), TRIPLET (vcpkg overlay triplet),
# VCPKG_COMMIT, RUST_VERSION. Optional: BOOTSTRAP_SIZE_BUDGET.
set -euo pipefail

WS="${GITHUB_WORKSPACE:-/ws}"
cd "$WS"

echo "== apt toolchain (focal) =="
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends \
  build-essential cmake ninja-build pkg-config \
  autoconf automake autoconf-archive libtool \
  curl zip unzip tar ca-certificates git gnupg lsb-release wget \
  libbz2-dev ruby software-properties-common

# rnp-rs's build.rs runs bindgen, which dlopens libclang. Focal's stock
# libclang (v10) is too old for the current bindgen; llvm.org publishes
# clang-19 for focal (the same version the musl leg uses).
echo "== clang-19 (llvm.org apt) =="
curl -fsSL https://apt.llvm.org/llvm-snapshot.gpg.key | gpg --dearmor -o /usr/share/keyrings/llvm.gpg
echo "deb [signed-by=/usr/share/keyrings/llvm.gpg] http://apt.llvm.org/focal/ llvm-toolchain-focal-19 main" \
  > /etc/apt/sources.list.d/llvm19.list
apt-get update -qq
apt-get install -y -qq --no-install-recommends clang-19 libclang-19-dev
echo "/usr/lib/llvm-19/lib" > /etc/ld.so.conf.d/llvm19.conf
ldconfig
export LIBCLANG_PATH=/usr/lib/llvm-19/lib

# rnp-src 0.2.0 builds Botan 3.12 (botan-src 0.31200.0): its configure
# hard-gates on gcc >= 11, and Botan 3's public headers need a C++20
# stdlib (std::span/concepts) that focal's stock libstdc++-9 lacks —
# focal main tops out at gcc 10 (run 30742821370 died on "This version
# of Botan requires at least gcc 11.0"). rnp-src pins the librnp
# compiler by NAME (gcc/g++), so the lever is to make those names
# resolve to the toolchain-r ppa's gcc-11: botan's configure
# auto-detect then passes and librnp/dwarfs-t/vcpkg ports all build with
# the same compiler. libstdc++ is absorbed STATICALLY into the shipped
# binaries (the v0.1.0 artifacts have no libstdc++.so NEEDED entry), so
# the ppa's newer libstdc++ adds no runtime floor — the GLIBC gate below
# stays the arbiter.
echo "== gcc-11 (ubuntu-toolchain-r ppa) =="
add-apt-repository -y -n ppa:ubuntu-toolchain-r/test
apt-get update -qq
apt-get install -y -qq --no-install-recommends gcc-11 g++-11
update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-11 110 \
  --slave /usr/bin/g++ g++ /usr/bin/g++-11
update-alternatives --install /usr/bin/cc cc /usr/bin/gcc-11 110
update-alternatives --install /usr/bin/c++ c++ /usr/bin/g++-11 110
g++ --version | head -1

echo "== rustup ($RUST_VERSION, $TARGET) =="
curl -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain "$RUST_VERSION" --target "$TARGET"
. "$HOME/.cargo/env"
rustc --version

echo "== vcpkg bootstrap ($VCPKG_COMMIT) =="
git clone --quiet https://github.com/microsoft/vcpkg "$WS/.vcpkg-floor"
git -C "$WS/.vcpkg-floor" checkout --quiet "$VCPKG_COMMIT"
"$WS/.vcpkg-floor/bootstrap-vcpkg.sh" -disableMetrics

# NOTE — no vcpkg-baseline restore here, deliberately: the published
# baseline trees are built on ubuntu-24.04, and their port objects would
# re-import glibc 2.39 symbols through the back door (the floor rule
# applies to every object in the link, not just ours). A floor-built
# baseline variant is a follow-up; the ports build in-leg meanwhile.
SQFS_TRIPLETS="$WS/tebako-rs/crates/sqfs-sys/vcpkg_triplets"
DWARFS_TRIPLETS="$WS/dwarfs-rs/dwarfs-t/vcpkg_triplets"
if [ ! -f "$SQFS_TRIPLETS/$TRIPLET.cmake" ]; then
  arch=x64; [ "$TARGET" = "aarch64-unknown-linux-gnu" ] && arch=arm64
  sed -e "s/VCPKG_TARGET_ARCHITECTURE x64/VCPKG_TARGET_ARCHITECTURE $arch/" \
      "$DWARFS_TRIPLETS/x64-linux-static.cmake" > "$SQFS_TRIPLETS/$TRIPLET.cmake"
fi

echo "== pre-install squashfs-tools-ng ($TRIPLET) =="
"$WS/.vcpkg-floor/vcpkg" install \
  --vcpkg-root "$WS/.vcpkg-floor" \
  --x-wait-for-lock \
  --x-manifest-root "$WS/tebako-rs/crates/sqfs-sys" \
  --x-install-root "$WS/.sqfs-floor" \
  --triplet "$TRIPLET" \
  --overlay-triplets "$SQFS_TRIPLETS" \
  --overlay-ports "$WS/tebako-rs/crates/sqfs-sys/vcpkg_ports"

echo "== cargo build (release, $TARGET) =="
cd "$WS/tebako-rs"
export DWARFS_RS_VCPKG_ROOT="$WS/.vcpkg-floor"
export DWARFS_RS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_INSTALLED_DIR="$WS/.sqfs-floor/$TRIPLET"
export CARGO_NET_GIT_FETCH_WITH_CLI=true
cargo build --release --target "$TARGET" \
  -p tebako-bootstrap -p tfs-cli -p tebako-pkg -p tebako-cli -p tebako-shim

echo "== stage / strip / size-gate =="
export TARGET
bash .github/workflows/lib/stage.sh

echo "== glibc floor gate (the artifact must need ≤ 2.31) =="
floor="$(objdump -T "out/tebako-bootstrap-${VERSION}-${PLATFORM}" \
  | grep -oE 'GLIBC_[0-9]+\.[0-9]+' | sort -Vu | tail -1)"
echo "max glibc symbol version required: ${floor:-none}"
if [ -n "$floor" ] && [ "$(printf '%s\n2.31\n' "$floor" | sort -V | tail -1)" != "2.31" ]; then
  echo "::error::floor violation: the bootstrap requires $floor (> 2.31)" >&2
  exit 65
fi

echo "== link unit (tebako-driver + tfs, scoped, + closure) =="
cargo build --release --target "$TARGET" -p tfs -p tebako-driver -p libtfs-preload
bash .github/workflows/lib/link-unit-stage.sh
