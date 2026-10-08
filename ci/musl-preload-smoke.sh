#!/bin/sh
# musl-preload-smoke.sh — tebako#518: the PR-time musl leg for the
# preload shim, run INSIDE an alpine:3.21 container (docker run from the
# glibc runner; node actions cannot run on musl — the release leg's
# proven native-musl path, ci/musl-build.sh).
#
# Before this leg the first musl COMPILE OR RUN of any preload change was
# the runtime factory's boot smoke — the last possible moment. Two
# regressions escaped through that window on 2026-09-02 (tebako#516's
# realpath NULL-arm reroute named canonicalize_file_name, a symbol musl
# has never shipped: tebako CI's 8 gnu/macos/windows legs were green and
# all 44 factory musl jobs then aborted every jailed child with
# `cannot resolve libc symbol "canonicalize_file_name" via RTLD_NEXT`).
# This leg closes the window at PR time:
#
#   1. COMPILE: the shim (its production feature set — vendored-dwarfs +
#      limnifs) and its unit tests build for musl libc;
#   2. UNIT: the route matrix runs on musl (the policy bind's
#      canonicalize path is exactly where the 0.16.19 escape fired);
#   3. RUN: the smoke matrix under LD_PRELOAD on real musl libc —
#      a. memfs read through the shim (print-data),
#      b. the constructor + jail bind inside a jailed SHELL child — the
#         factory boot smoke's exact host_shell_string shape
#         (/bin/sh -c; alpine's busybox is dynamically linked),
#      c. the tebako#534 dup reproducer (dup-probe — libxml2's
#         xmlInputFromFd shape: open a VFS file, dup, read through the
#         dup),
#      d. the tebako#444 fopen write gate (fopen-probe: deny → EPERM,
#         ro grant → EROFS, rw grant passes, memfs-held → EROFS) and the
#         audit's creat leg.
#
# All fixtures are plain POSIX — they link and run identically on glibc,
# musl, and libSystem (the e2e suite rides the same sources).
#
# Required env: RUST_VERSION, VCPKG_COMMIT. Optional: TRIPLET (the vcpkg
# overlay triplet, default x64-linux-musl — set arm64-linux-musl for a
# native run on an arm64 host; the release matrix proves both). Runs as
# root in the single-use container with the workspace bind-mounted at
# $GITHUB_WORKSPACE (tebako-rs/ + dwarfs-rs/ siblings below it).
set -eu

WS="${GITHUB_WORKSPACE:-/ws}"
cd "$WS"

: "${RUST_VERSION:?RUST_VERSION is required}"
: "${VCPKG_COMMIT:?VCPKG_COMMIT is required}"
TRIPLET="${TRIPLET:-x64-linux-musl}"

fail() {
    echo "SMOKE FAIL: $*" >&2
    exit 1
}

echo "== apk toolchain =="
apk --no-cache add \
  build-base cmake ninja git bash sudo \
  autoconf automake libtool make pkgconfig perl python3 \
  curl zip unzip tar ca-certificates linux-headers \
  clang19-libclang ruby

# The workspace is bind-mounted from the runner (files owned by uid
# 1001) but the build runs as root — alpine's git refuses the mounted
# repos as "dubious ownership" (ci/musl-build.sh's note; release run
# 30742821370). Single-use --rm container: trust every repo under it.
git config --global --add safe.directory '*'

# vcpkg's bootstrap downloads glibc-linked cmake/ninja by default — they
# cannot run on musl (exit 127). Use the apk-provided tools everywhere.
export VCPKG_FORCE_SYSTEM_BINARIES=1

echo "== rustup ($RUST_VERSION) =="
curl -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain "$RUST_VERSION"
. "$HOME/.cargo/env"
rustc --version

echo "== vcpkg bootstrap ($VCPKG_COMMIT) =="
git clone --quiet https://github.com/microsoft/vcpkg "$WS/.vcpkg-musl-ci"
git -C "$WS/.vcpkg-musl-ci" checkout --quiet "$VCPKG_COMMIT"
"$WS/.vcpkg-musl-ci/bootstrap-vcpkg.sh" -disableMetrics

# The musl overlay triplets live in dwarfs-t (x64 checked in) and are
# injected here where missing (arm64 static + the sqfs copies) — the
# release leg's exact block (ci/musl-build.sh).
SQFS_TRIPLETS="$WS/tebako-rs/crates/sqfs-sys/vcpkg_triplets"
DWARFS_TRIPLETS="$WS/dwarfs-rs/dwarfs-t/vcpkg_triplets"
mk_triplet() {  # $1 = new name, $2 = arch (x64|arm64), $3 = linkage (static|dynamic)
  # CRT linkage must follow library linkage (the musl-build.sh note:
  # a static CRT under dynamic linkage breaks shared-library links).
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
if [ ! -f "$SQFS_TRIPLETS/$TRIPLET.cmake" ]; then
  sed -e "s/VCPKG_TARGET_ARCHITECTURE x64/VCPKG_TARGET_ARCHITECTURE $( [ "$TRIPLET" = "arm64-linux-musl" ] && echo arm64 || echo x64 )/" \
      "$DWARFS_TRIPLETS/x64-linux-musl.cmake" > "$SQFS_TRIPLETS/$TRIPLET.cmake"
fi

# Pre-install squashfs-tools-ng in a serialized step — a parallel build
# script would otherwise race dwarfs-t-sys's CMake-driven vcpkg run on
# the vcpkg-root filesystem lock (the sqfs-sys build.rs note).
echo "== pre-install squashfs-tools-ng ($TRIPLET) =="
"$WS/.vcpkg-musl-ci/vcpkg" install \
  --vcpkg-root "$WS/.vcpkg-musl-ci" \
  --x-wait-for-lock \
  --x-manifest-root "$WS/tebako-rs/crates/sqfs-sys" \
  --x-install-root "$WS/.sqfs-musl-ci" \
  --triplet "$TRIPLET" \
  --overlay-triplets "$SQFS_TRIPLETS" \
  --overlay-ports "$WS/tebako-rs/crates/sqfs-sys/vcpkg_ports"

# musl targets default to +crt-static, and a statically linked build
# script cannot dlopen — the musl-build.sh contract: -crt-static OFF
# (the artifacts are dynamic-musl, the factory's musl runtime shape),
# build NATIVE (no --target, so RUSTFLAGS covers host units too —
# proven against cargo 1.94: with --target the build scripts get
# neither), and libstdc++/libgcc_s absorbed through the driver-boundary
# wrapper (a NEEDED libstdc++.so.6 is exit 127 on a vanilla alpine).
export RUSTFLAGS="-C target-feature=-crt-static -C linker=$WS/tebako-rs/ci/linux-link-wrap.sh"
export CARGO_NET_GIT_FETCH_WITH_CLI=true
cd "$WS/tebako-rs"
export DWARFS_RS_VCPKG_ROOT="$WS/.vcpkg-musl-ci"
export DWARFS_RS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_TRIPLET="$TRIPLET"
export SQFS_SYS_VCPKG_INSTALLED_DIR="$WS/.sqfs-musl-ci/$TRIPLET"

echo "== cargo build (libtfs-preload, native musl) =="
cargo build -p libtfs-preload

echo "== cargo test (libtfs-preload unit, native musl) =="
cargo test -p libtfs-preload --lib

echo "== smoke fixtures =="
SMOKE="$(mktemp -d)"
mkdir -p "$SMOKE/img/data" "$SMOKE/img/dir" "$SMOKE/bin" "$SMOKE/work" "$SMOKE/rw"
printf 'VFS-SECRET-E2E\n' > "$SMOKE/img/data/secret.txt"
printf 'a\n' > "$SMOKE/img/dir/a.txt"
printf 'b\n' > "$SMOKE/img/dir/b.txt"
printf 'HOST-FILE\n' > "$SMOKE/work/hostfile.txt"
(cd "$SMOKE/img" && zip -q -r "$SMOKE/img.zip" .)
FIX="$WS/tebako-rs/crates/libtfs-preload/tests/fixtures"
for t in print-data dup-probe fopen-probe mk-dir; do
    cc -O2 -o "$SMOKE/bin/$t" "$FIX/$t.c"
done

SHIM="$WS/tebako-rs/target/debug/libtfs_preload.so"
[ -f "$SHIM" ] || fail "the built shim is missing: $SHIM"

cd "$SMOKE"
export LD_PRELOAD="$SHIM"
export TEBAKO_TFS_MOUNTS="$SMOKE/img.zip:/tfs"

# 3a. memfs read through musl libc — /tfs does not exist on the host, so
# the bytes can only have come through the VFS.
out="$(./bin/print-data /tfs/data/secret.txt)" || fail "memfs read rc=$?"
[ "$out" = "VFS-SECRET-E2E" ] || fail "memfs read got: $out"
echo "smoke: memfs read ok"

# 3b. The constructor + jail bind inside a jailed SHELL child — the
# shape that aborted the factory's 44 musl jobs (the jail bind's
# canonicalize rides the realpath NULL arm, musl's no-canonicalize_file_name
# path). busybox sh on alpine is dynamically linked, so the preload
# applies.
out="$(TEBAKO_JAIL=deny /bin/sh -c 'echo shell-boot-ok')" || fail "the jailed shell child did not boot rc=$?"
[ "$out" = "shell-boot-ok" ] || fail "the jailed shell got: $out"
if TEBAKO_JAIL=deny /bin/sh -c 'cat /etc/hostname' 2>/dev/null; then
    fail "a jailed host read through /bin/sh succeeded"
fi
echo "smoke: jailed shell child ok (boot + EPERM read)"

# 3c. tebako#534: dup of a VFS fd (libxml2's xmlInputFromFd shape).
out="$(./bin/dup-probe /tfs/data/secret.txt)" || fail "dup-probe rc=$?"
[ "$out" = "dup-probe:ok" ] || fail "dup-probe got: $out"
echo "smoke: dup family ok"

# 3d. tebako#444: the fopen write-mode gate + the creat audit leg.
# EPERM=1, EROFS=30 (the fixtures exit with the errno).
for mode in w a r+ w+ a+ wx; do
    rc=0
    TEBAKO_JAIL=deny ./bin/fopen-probe fopen-write "$mode" /etc/hostname 2>/dev/null || rc=$?
    [ "$rc" -eq 1 ] || fail "fopen($mode) under deny rc=$rc (want 1/EPERM)"
done
rc=0
TEBAKO_JAIL=deny ./bin/fopen-probe creat-write /etc/hostname 2>/dev/null || rc=$?
[ "$rc" -eq 1 ] || fail "creat under deny rc=$rc (want 1/EPERM)"
rc=0
TEBAKO_JAIL="deny;$SMOKE/work:/work:ro" ./bin/fopen-probe fopen-write w "$SMOKE/work/hostfile.txt" 2>/dev/null || rc=$?
[ "$rc" -eq 30 ] || fail "fopen(w) against the ro grant rc=$rc (want 30/EROFS)"
out="$(TEBAKO_JAIL="deny;$SMOKE/rw:/rw:rw" ./bin/fopen-probe fopen-write w "$SMOKE/rw/out.txt")" \
    || fail "rw-granted fopen write rc=$?"
[ "$out" = "OK:w" ] || fail "rw-granted fopen write got: $out"
[ "$(cat "$SMOKE/rw/out.txt")" = "Z" ] || fail "the rw-granted write did not land"
rc=0
./bin/fopen-probe fopen-write w /tfs/data/secret.txt 2>/dev/null || rc=$?
[ "$rc" -eq 30 ] || fail "memfs-held fopen(w) rc=$rc (want 30/EROFS)"
echo "smoke: fopen write gate ok"

echo "== musl preload smoke: ALL GREEN =="
