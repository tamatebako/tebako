#!/usr/bin/env bash
# press-gem.sh — press a one-gem application through the pinned tebako
# tool and report the verdict. The minimal reproduction vehicle: when a
# release leg fails inside a native-extension build, this probe answers
# whether the failure is gem-specific, tool-version-specific, or
# environmental, in one five-minute run instead of one release cycle.
#
# Inputs (env):
#   PROBE_GEM      the gem to press (default nio4r)
#   PROBE_GEM_VER  an optional version pin (default none)
#   PROBE_REL      the tebako release tag (default v2.8.44)
set -euo pipefail
. "$(dirname "$0")/lib/common.sh"

with_devkit
fetch_tools "${PROBE_REL:-v2.8.44}"

mkdir -p app && cd app
{
  echo 'source "https://rubygems.org"'
  if [ -n "${PROBE_GEM_VER:-}" ]; then
    echo "gem \"${PROBE_GEM:-nio4r}\", \"${PROBE_GEM_VER}\""
  else
    echo "gem \"${PROBE_GEM:-nio4r}\""
  fi
} > Gemfile
printf 'puts "hello"\n' > main.rb

press_app "$(pwd)"
