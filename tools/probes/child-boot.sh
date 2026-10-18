#!/usr/bin/env bash
# child-boot.sh — the packaged child's boot, isolated layer by layer.
# The ruby.exe re-entry vehicle boots through: the bootstrap (trailer
# → runtime resolution) → the driver (mounts + aliases) → the stub.
# This probe runs each layer alone with the same bytes, so a boot
# failure names its layer in one run:
#   arm 1  ruby.exe -e          the full child, exactly as a spawn sees it
#   arm 2  the runtime exe + the deploy pkg, no bootstrap
#   arm 3  the runtime exe with the env image only (no payload)
set -euo pipefail
. "$(dirname "$0")/lib/common.sh"

with_devkit
fetch_tools "${PROBE_REL:-v2.8.44}"

mkdir -p app && cd app
printf 'source "https://rubygems.org"\ngem "nio4r"\n' > Gemfile
printf 'puts "hello"\n' > main.rb
press_app "$(pwd)"

home="$(cygpath "$USERPROFILE")/.tebako"
P="$home/o/p"
rt_dir="$(ls -d "$LOCALAPPDATA"/tebako/runtimes/ruby-* 2>/dev/null | head -1)"
RT="$(find "$rt_dir" -maxdepth 1 -type f ! -name "*.tfs" ! -name "*.sha256" ! -name "*.origin" ! -name "*.json" ! -name ".*" 2>/dev/null | head -1)"
export TEBAKO_RUNTIME_IMAGE="$rt_dir/$(basename "$rt_dir").tfs"
export MSYS2_ARG_CONV_EXCL="*"

run_arm() {
  echo "==== $1 ===="
  shift
  set +e
  "$@" 2>&1 | head -6
  set -e
  echo
}

run_arm "arm 1: the child replica (ruby.exe)" \
  "$P/ruby.exe" -e 'puts "CHILD OK"'
run_arm "arm 2: the runtime + deploy pkg (no bootstrap)" \
  "$RT" --tebako-image "$P/deploy-driver.pkg:0:/t" --tebako-entry /local/stub.rb -e 'puts "DIRECT OK"'
run_arm "arm 3: the runtime with the env image only" \
  "$RT" --tebako-entry /local/stub.rb -e 'puts "ENVONLY OK"'
