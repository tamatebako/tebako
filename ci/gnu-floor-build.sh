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
  libbz2-dev ruby

# The workspace is bind-mounted from the runner (files owned by uid
# 1001) but the build runs as root — focal's git (2.25 with the
# CVE-2022-24765 backport) refuses the mounted repos as "dubious
# ownership", so dwarfs-t's cmake/version.cmake sees NO git metadata and
# dies ("missing version files - git metadata unavailable and no
# pre-generated version files found"; run 30750321798 — it only surfaces
# now that cmake 3.31 lets the configure reach version.cmake). Same fix
# as the musl leg: single-use --rm container, trust every repo under the
# mount.
git config --global --add safe.directory '*'

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
# add-apt-repository's launchpad API lookup hangs ~9 min then dies on
# the arm64 runners ("'~ubuntu-toolchain-r' user or team does not
# exist"; run 30745532174) — the llvm.org pattern (key + deb line, no
# API call) works on both arches. Key 1E9377A2BA9EF27F ("Launchpad
# Toolchain builds") is the ppa's published signer; the ppa ships
# gcc-11/g++-11 for focal amd64 AND arm64.
curl -fsSL "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x1E9377A2BA9EF27F" \
  | gpg --dearmor -o /usr/share/keyrings/toolchainr.gpg
echo "deb [signed-by=/usr/share/keyrings/toolchainr.gpg] http://ppa.launchpad.net/ubuntu-toolchain-r/test/ubuntu focal main" \
  > /etc/apt/sources.list.d/toolchainr.list
apt-get update -qq
apt-get install -y -qq --no-install-recommends gcc-11 g++-11
update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-11 110 \
  --slave /usr/bin/g++ g++ /usr/bin/g++-11
update-alternatives --install /usr/bin/cc cc /usr/bin/gcc-11 110
update-alternatives --install /usr/bin/c++ c++ /usr/bin/g++-11 110
g++ --version | head -1

# glibc 2.31 keeps pthreads in a separate libpthread (2.34 folds it into
# libc), and librnp 0.18.1 builds its examples unconditionally
# (add_subdirectory — no switch) with no find_package(Threads) anywhere:
# their link of libbotan-3.a dies on undefined pthread_create /
# pthread_setname_np (run 30748661257). -pthread as a DRIVER flag is
# order-immune — gcc expands it in LIB_SPEC after the objects, so the
# front-of-line position cmake gives env flags cannot strand it ahead of
# the archives (a plain -lpthread there is dropped by --as-needed).
# cmake reads CFLAGS/CXXFLAGS into CMAKE_C(XX)_FLAGS at first configure,
# so this covers dwarfs-t-sys and every rnp-src dep; vcpkg's own
# toolchain overrides these for its ports, so the port builds are
# untouched.
export CFLAGS=-pthread
export CXXFLAGS=-pthread

# dwarfs-t's cmake_minimum_required is 3.28 (its non-TEBAKO_BUILD
# branch) and librnp 0.18.1's is 3.18 — focal's stock cmake is 3.16
# (run 30745532174). Kitware's release tarball is arch-exact and runs on
# focal's glibc; it shadows the stock one via /usr/local/bin.
echo "== cmake 3.31.8 (Kitware tarball — dwarfs-t needs >= 3.28) =="
cmake_arch=x86_64; [ "$(uname -m)" = "aarch64" ] && cmake_arch=aarch64
curl -fsSL "https://github.com/Kitware/CMake/releases/download/v3.31.8/cmake-3.31.8-linux-${cmake_arch}.tar.gz" \
  | tar -xz -C /opt
ln -sfn "/opt/cmake-3.31.8-linux-${cmake_arch}/bin/cmake" /usr/local/bin/cmake
cmake --version | head -1

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
# Compare the NUMERIC tail: with the GLIBC_ prefix in the sort input,
# strverscmp orders letters after digits, so every value — even a
# compliant GLIBC_2.30 — comes out ">" 2.31 (run 30756664324: the gate
# failed its own contract on a clean binary).
floor_num="${floor#GLIBC_}"
if [ -n "$floor_num" ] && [ "$(printf '%s\n2.31\n' "$floor_num" | sort -V | tail -1)" != "2.31" ]; then
  echo "::error::floor violation: the bootstrap requires $floor (> 2.31)" >&2
  exit 65
fi

echo "== link unit (tebako-driver + tfs, scoped, + closure) =="
cargo build --release --target "$TARGET" -p tfs -p tebako-driver -p libtfs-preload
bash .github/workflows/lib/link-unit-stage.sh
