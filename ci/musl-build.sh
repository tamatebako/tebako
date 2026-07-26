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
[ -f "$DWARFS_TRIPLETS/$DYN_TRIPLET.cmake" ] || {
  case "$DYN_TRIPLET" in
    x64-linux-musl-dynamic)   mk_triplet "$DYN_TRIPLET" x64 dynamic ;;
    arm64-linux-musl-dynamic) mk_triplet "$DYN_TRIPLET" arm64 dynamic ;;
  esac
}
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

echo "== pre-install Botan-3 + json-c ($DYN_TRIPLET, for vendored librnp) =="
crypto_rc=0
"$WS/.vcpkg-musl/vcpkg" install botan json-c \
  --vcpkg-root "$WS/.vcpkg-musl" \
  --x-wait-for-lock \
  --x-install-root "$WS/.crypto-musl" \
  --triplet "$DYN_TRIPLET" \
  --overlay-triplets "$DWARFS_TRIPLETS" || crypto_rc=$?
if [ "$crypto_rc" -ne 0 ]; then
  # The workflow log shows only the failing make command; the real
  # compiler/linker error lives in the container's buildtrees logs, which
  # are lost when the container exits. Dump them before dying (plus the
  # resource state — both musl legs of run 30221016817 died in botan's
  # debug build with the real error never captured).
  for log in "$WS"/.vcpkg-musl/buildtrees/*/build-*"$DYN_TRIPLET"*-err.log; do
    [ -f "$log" ] || continue
    echo "== $log (tail -n 120) =="
    tail -n 120 "$log"
  done
  df -h "$WS" || true
  free -m || true
  exit "$crypto_rc"
fi

echo "== cargo build (release, $TARGET) =="
cd "$WS/tebako-rs"
export DWARFS_RS_VCPKG_ROOT="$WS/.vcpkg-musl"
export DWARFS_RS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_INSTALLED_DIR="$WS/.sqfs-musl/$TRIPLET"
export RNP_VENDOR_CMAKE_ARGS="CMAKE_PREFIX_PATH=$WS/.crypto-musl/$DYN_TRIPLET"
export RUSTFLAGS="-L native=$WS/.crypto-musl/$DYN_TRIPLET/lib"
cargo build --release --target "$TARGET" \
  -p tebako-bootstrap -p tfs-cli -p tebako-pkg -p tebako-cli

echo "== stage / strip / size-gate =="
export TARGET
bash .github/workflows/lib/stage.sh
