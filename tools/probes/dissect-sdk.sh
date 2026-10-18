#!/usr/bin/env bash
# dissect-sdk.sh — the runtime SDK's link surface, laid out for
# reading. After a press (any press that provisioned the SDK), this
# probe prints the SDK tree, the symbol stub, the dlltool import
# library and its .def, and the answers to the three questions that
# the rails-on-windows chain learned to ask (tebako#753):
#   1. does the def carry the DLL's TRUE export table (objdump -p),
#      not nm globals?
#   2. does the import library answer both the plain and __imp_
#      spellings?
#   3. do the extensions' import tables bind CRT names to the system
#      CRT and only genuine exports to the ruby DLL?
set -euo pipefail
. "$(dirname "$0")/lib/common.sh"

with_devkit
fetch_tools "${PROBE_REL:-v2.8.44}"

mkdir -p app && cd app
printf 'source "https://rubygems.org"\ngem "date"\n' > Gemfile
printf 'puts "hello"\n' > main.rb
press_app "$(pwd)"

home="$(cygpath "$USERPROFILE")/.tebako"
sdk_lib="$(ls -d "$home"/deps/sdk/*/lib 2>/dev/null | head -1)"
echo "==== the SDK lib dir ===="
ls -la "$sdk_lib" 2>/dev/null || echo "(no SDK lib dir — the press did not provision)"

echo "==== the def answers the chain's symbols? ===="
def="$(ls "$sdk_lib"/libruby.dll.def 2>/dev/null | head -1)"
if [ -n "$def" ]; then
  for sym in rb_intern ruby_snprintf localtime_r gmtime_r strlcpy; do
    grep -qx "$sym" "$def" && echo "  $sym: present" || echo "  $sym: MISSING"
  done
  echo "  CRT names absent (must be): $(grep -cxE 'memcpy|Sleep|DllMain|_initterm' "$def" || true)"
else
  echo "  (no def — the import library was not generated)"
fi

echo "==== the built .so's import tables (CRT must bind the system CRT) ===="
so="$(find "$home/o/s/lib/ruby/gems" -name "*.so" 2>/dev/null | head -1)"
if [ -n "$so" ]; then
  objdump -p "$so" | grep -E "DLL Name" | sort -u
  echo "  -- from the ruby DLL:"
  objdump -p "$so" | sed -n '/DLL Name: x64-ucrt-ruby/,/^$/p' | head -12
else
  echo "  (no .so built — the press compiled nothing)"
fi
