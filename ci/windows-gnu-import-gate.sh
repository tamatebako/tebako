#!/usr/bin/env bash
# ci/windows-gnu-import-gate.sh — audit PE import tables of the shipped
# windows-gnu binaries and FAIL on any non-inbox DLL.
#
#   windows-gnu-import-gate.sh <exe> [<exe> ...]
#
# Why: a windows-gnu exe that imports a MinGW runtime DLL
# (libstdc++-6.dll, libwinpthread-1.dll, libgcc_s_seh-1.dll, …) dies
# before main() on stock Windows — STATUS_DLL_NOT_FOUND, surfaced as
# exit 127 — because nothing outside a toolchain installs those DLLs.
# Audience law: a tebako user downloads and runs, full stop. The 0.1.1
# windows-ucrt64 assets shipped exactly this break; the informational
# "DLL-import forensics" step saw it and did not fail. This gate is the
# enforcement: the allowlist is the Windows-inbox surface (the UCRT
# api-ms-win-crt-*/api-ms-win-core-* set — inbox since Windows 10 — plus
# the classic system DLLs); anything else needs a conscious edit of this
# file with the reason recorded.
#
# The mechanism that makes the link clean lives in
# ci/windows-gnu-link-wrap.c; this gate is what PROVES it on the real
# binaries, in the same leg, so a wrapper miss can never ship silently.
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: $0 <exe> [<exe> ...]" >&2
  exit 64
fi

# Inbox DLL allowlist (lowercase, basename). api-ms-win-* is a family, not
# a fixed list — matched by prefix below.
ALLOWED_EXACT="
kernel32.dll
advapi32.dll
user32.dll
ws2_32.dll
wsock32.dll
mswsock.dll
ntdll.dll
msvcrt.dll
bcrypt.dll
bcryptprimitives.dll
crypt32.dll
shell32.dll
shlwapi.dll
psapi.dll
userenv.dll
ole32.dll
oleaut32.dll
uuid.dll
secur32.dll
sspicli.dll
iphlpapi.dll
rpcrt4.dll
dbghelp.dll
version.dll
gdi32.dll
comctl32.dll
setupapi.dll
netapi32.dll
pdh.dll
powrprof.dll
propsys.dll
shcore.dll
winmm.dll
wldap32.dll
imm32.dll
dwmapi.dll
uxtheme.dll
winspool.drv
cfgmgr32.dll
kernel.appcore.dll
authz.dll
credui.dll
cryptnet.dll
normaliz.dll
timezoneapi.dll
winhttp.dll
wininet.dll
comdlg32.dll
msimg32.dll
"

offenders=0
for exe in "$@"; do
  [ -f "$exe" ] || { echo "import-gate: no such file: $exe" >&2; exit 66; }
  echo "=== imports: $exe ==="
  dlls=$(objdump -p "$exe" | grep "DLL Name:" | sed 's/.*DLL Name: //' | sort -u)
  echo "$dlls"
  while IFS= read -r dll; do
    [ -n "$dll" ] || continue
    low=$(printf '%s' "$dll" | tr 'A-Z' 'a-z')
    case "$low" in
      api-ms-win-*) continue ;;
    esac
    if ! printf '%s\n' "$ALLOWED_EXACT" | grep -qxF "$low"; then
      echo "import-gate: OFF-LIST DLL import in $exe: $dll" >&2
      offenders=$((offenders + 1))
    fi
  done <<< "$dlls"
done

if [ "$offenders" -gt 0 ]; then
  echo "import-gate: FAIL — $offenders off-list DLL import(s). A shipped exe must run on stock Windows (ucrt64 bin is NOT on a user's PATH). See ci/windows-gnu-link-wrap.c for the link policy." >&2
  exit 1
fi
echo "import-gate: PASS ($# exe(s), inbox-only imports)"
