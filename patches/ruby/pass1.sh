# shellcheck shell=bash
# Copyright (c) 2021-2022, [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of tebako
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
# 1. Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
# 2. Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
# TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR CONTRIBUTORS
# BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
# CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
# SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
# INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
# CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
# ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
# POSSIBILITY OF SUCH DAMAGE.

# More safety, by turning some bugs into errors.
# Without `errexit` you don’t need ! and can replace
# PIPESTATUS with a simple $?
set -o errexit -o pipefail -o noclobber -o nounset

# ....................................................
restore_and_save() {
  echo "Pass 1 processing $1"
  test -e "$1.old" && cp -f "$1.old" "$1"
  cp -f "$1" "$1.old"
}

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  gSed="sed"
elif [[ "$OSTYPE" == "darwin"* ]]; then
  gSed="gsed"
  tBrew="${3:-"guess"}"
  if [[ "$tBrew" == "guess" ]]; then
    tBrew="brew"
  else
    tBrew="$tBrew/bin/brew"
  fi  
else
  exit 1
fi

# ....................................................
# Disable dynamic extensions
# ruby/ext/Setup
restore_and_save "$1/ext/Setup"
"$gSed" -i "s/\#option nodynamic/option nodynamic/g" "$1/ext/Setup"

# ....................................................
# This is something that I cannnot explain
# (this patch does not seem related to static compilation)
# ruby/ext/bigdecimal/bigdecimal.h
restore_and_save "$1/ext/bigdecimal/bigdecimal.h"
re="#include <float.h>"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
#include <float.h>

\/* -- Start of tebako patch -- *\/
#ifndef HAVE_RB_SYM2STR
#define HAVE_RB_SYM2STR  1
#endif

#ifndef HAVE_RB_ARRAY_CONST_PTR
#define HAVE_RB_ARRAY_CONST_PTR 1
#endif

#ifndef HAVE_RB_RATIONAL_NUM
#define HAVE_RB_RATIONAL_NUM 1
#endif

#ifndef HAVE_RB_RATIONAL_DEN
#define HAVE_RB_RATIONAL_DEN 1
#endif
\/* -- End of tebako patch -- *\/

EOM

"$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/ext/bigdecimal/bigdecimal.h"

# ....................................................
# Ruby 2.7.4:  template is in 'ruby/template/Makefile.in'
# Ruby 2.6.3:  template is in 'ruby/Makefile.in'

if [[ "$OSTYPE" == "darwin"* ]]; then

  restore_and_save "$1/template/Makefile.in"

  re="		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(XLDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG) \$(MAINLIBS) \$(LIBS) \$(EXTLIBS) \$(OUTFLAG)\$@"
# Ruby 2.7.5
# If cross compile sets XLDFLAGS to '-framework CoreFoundation Security'
# it is wrong syntax
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG) \$(MAINLIBS) \$(LIBS) \$(EXTLIBS) \$(OUTFLAG)\$@
# -- End of tebako patch --
EOM

  "$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/template/Makefile.in"

# Pin tebako static build libraries
  re="MAINLIBS = @MAINLIBS@"

# ....................................................
  p_libssl="$($tBrew --prefix openssl@1.1)/lib/libssl.a"
  p_libcrypto="$($tBrew --prefix openssl@1.1)/lib/libcrypto.a"
  p_libz="$($tBrew --prefix zlib)/lib/libz.a"
  p_libgdbm="$($tBrew --prefix gdbm)/lib/libgdbm.a"
  p_libreadline="$($tBrew --prefix readline)/lib/libreadline.a"
  p_libffi="$($tBrew --prefix libffi)/lib/libffi.a"
  p_libncurses="$($tBrew --prefix ncurses)/lib/libncurses.a"
# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = $p_libssl $p_libcrypto $p_libz $p_libgdbm $p_libreadline \\\\
$p_libffi $p_libncurses -ljemalloc -lc++
# -- End of tebako patch --
EOM

  "$gSed" -i "0,/$re/s||${mLibs//$'\n'/"\\n"}|g" "$1/template/Makefile.in"

  re="LIBS = @LIBS@ \$(EXTLIBS)"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
LIBS = @LIBS@
# -- End of tebako patch --
EOM
#

  "$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/template/Makefile.in" 

# ....................................................
  restore_and_save "$1/configure"

  re="   LDFLAGS=\"\$LDFLAGS \$opt\""
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
		LDFLAGS=\"\$LDFLAGS -Wl,\$opt\"
# -- End of tebako patch --
EOM

  "$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/configure"


fi
