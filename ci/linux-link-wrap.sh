#!/bin/sh
# linux-link-wrap.sh — the release-link policy for the shipped LINUX
# binaries (the gnu floor legs and the musl legs): the C/C++ runtime
# chain is ALWAYS absorbed statically, no matter which crate in the
# dependency tree emitted the link directive.
#
# Why (the exit-127 class, linux edition): a shipped exe that NEEDs
# libstdc++.so.6 / libgcc_s.so.1 dies before main on a vanilla target
# box — alpine:3.21 has neither (the 0.1.1 musl assets exit 127 there),
# and a gnu build against the ppa gcc's libstdc++ imports GLIBCXX symbol
# versions above the claimed glibc-2.31 floor. RUSTFLAGS tail flags
# (-C link-arg=-static-libstdc++ -C link-arg=-static-libgcc) look like
# the fix but are not: rustc passes a build script's
# cargo:rustc-link-lib=dylib=stdc++ (rnp-rs emits exactly that) as an
# explicit -lstdc++ wrapped in -Wl,-Bdynamic at its OWN position, BEFORE
# the trailing -C link-args — and gcc's -static-libstdc++ only rewrites
# the driver's own implicit -lstdc++, never an explicit one (proven on
# ubuntu:20.04 gcc-9: `gcc t.o -Wl,-Bdynamic -lstdc++ -static-libstdc++`
# keeps the NEEDED libstdc++.so.6). Same conclusion the windows-gnu legs
# reached (ci/windows-gnu-link-wrap.c).
#
# What it does: every reference is rewritten to a form that is immune to
# position and to the surrounding -Bstatic/-Bdynamic mode:
#
#   -lstdc++   ->  -l:libstdc++.a            (exact-archive form)
#   -lgcc_s    ->  -l:libgcc.a -l:libgcc_eh.a
#
# in all three rustc emission shapes: bare argv entries, members of
# -Wl,<a>,<b>,... comma lists, and tokens inside @response files (rustc
# moves long link lines into one). Everything else passes through
# untouched: the glibc surface (-lc -lm -lpthread -ldl -lrt -lutil)
# stays dynamic by design, and the musl legs keep libc.musl dynamic —
# the same shape as the runtime factory's musl runtimes (musl libc is
# present on every musl system by definition).
#
# Engaged from ci/gnu-floor-build.sh / ci/musl-build.sh via
#   CARGO_TARGET_<TRIPLE>_LINKER=<this script>
# (cargo spawns the linker directly — the shebang makes a script a valid
# linker; the windows-gnu wrapper must be a compiled exe only because
# CreateProcess has no shebang). The real driver is $TEBAKO_LINK_WRAP_CC,
# default cc — resolved from the leg's PATH at exec time.
#
# The ship gate (.github/workflows/lib/ship-gate.sh) audits the result
# on the staged bytes; this wrapper is the mechanism it verifies.
set -eu
# No globbing anywhere below (the IFS=, member split must not expand).
set -f

REAL="${TEBAKO_LINK_WRAP_CC:-cc}"

TMPD=$(mktemp -d "${TMPDIR:-/tmp}/tebako-link-wrap.XXXXXX")
trap 'rm -rf "$TMPD"' EXIT HUP INT TERM

# rewrite_respfile <path> <scratch-name> -> prints the rewritten copy's
# path. The rewrite targets are exact flag spellings that can never
# appear inside a quoted path member, so a byte-level global rewrite of
# the response file is token-safe (sed BRE: '+' is literal).
rewrite_respfile() {
  nf="$TMPD/$2"
  sed -e 's/-lstdc++/-l:libstdc++.a/g' \
      -e 's/-lgcc_s/-l:libgcc.a -l:libgcc_eh.a/g' "$1" > "$nf"
  printf '%s' "$nf"
}

# rewrite_wl <-Wl,a,b,...> -> prints the rewritten list, newline-ended
# (members never contain spaces; a -lgcc_s member becomes two
# comma-joined members).
rewrite_wl() {
  rest=${1#-Wl,}
  out="-Wl,"
  first=1
  oldifs=$IFS
  IFS=,
  for m in $rest; do
    IFS=$oldifs
    case $m in
      -lstdc++) r='-l:libstdc++.a' ;;
      -lgcc_s)  r='-l:libgcc.a,-l:libgcc_eh.a' ;;
      *)        r=$m ;;
    esac
    if [ "$first" -eq 1 ]; then
      out="$out$r"
      first=0
    else
      out="$out,$r"
    fi
    IFS=,
  done
  IFS=$oldifs
  printf '%s\n' "$out"
}

# Rebuild the positional parameters with every rewrite applied. The
# rewritten argv is collected one token per line in a scratch file (a
# passthrough token may legitimately contain spaces; newline framing
# keeps it intact).
LIST="$TMPD/argv"
: > "$LIST"
i=0
for a in "$@"; do
  case $a in
    -lstdc++)
      printf '%s\n' '-l:libstdc++.a' >> "$LIST" ;;
    -lgcc_s)
      printf '%s\n%s\n' '-l:libgcc.a' '-l:libgcc_eh.a' >> "$LIST" ;;
    -Wl,*)
      rewrite_wl "$a" >> "$LIST" ;;
    @*)
      f=${a#@}
      if [ -f "$f" ]; then
        i=$((i + 1))
        nf=$(rewrite_respfile "$f" "resp.$i")
        printf '@%s\n' "$nf" >> "$LIST"
      else
        printf '%s\n' "$a" >> "$LIST"
      fi ;;
    *)
      printf '%s\n' "$a" >> "$LIST" ;;
  esac
done

set --
while IFS= read -r line; do
  set -- "$@" "$line"
done < "$LIST"

exec "$REAL" "$@"
