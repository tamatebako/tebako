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
elif [[ "$OSTYPE" == "linux-musl"* ]]; then
  gSed="sed"
  restore_and_save "$1/thread_pthread.c"
elif [[ "$OSTYPE" == "darwin"* ]]; then
  gSed="gsed"
else
  exit 1
fi

# ....................................................
# Disable dynamic extensions
# ruby/ext/Setup
restore_and_save "$1/ext/Setup"
"$gSed" -i "s/\#option nodynamic/option nodynamic/g" "$1/ext/Setup"

# ....................................................
# WE DO NOT ACCEPT OUTSIDE GEM PATHS
# ruby/lib/rubygems/path_support.rb
restore_and_save "$1/lib/rubygems/path_support.rb"

re="  @home = env\[\"GEM_HOME\"\] || Gem.default_dir"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
    @home = env\["GEM_HOME"\] || Gem.default_dir
# -- Start of tebako patch --
    unless env\['TEBAKO_PASS_THROUGH'\]
      @home = Gem.default_dir unless @home.index('\/__tebako_memfs__') == 0
    end
# -- End of tebako patch --
EOM

"$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/lib/rubygems/path_support.rb"

re="@path = split_gem_path env\[\"GEM_PATH\"\], @home"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
    @path = split_gem_path env\["GEM_PATH"\], @home
# -- Start of tebako patch --
    unless env\['TEBAKO_PASS_THROUGH'\]
      @path.keep_if do |xpath|
        xpath.index('\/__tebako_memfs__') == 0
      end
    end
# -- End of tebako patch --
EOM

"$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/lib/rubygems/path_support.rb"

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
# Roll-back pass2 patches from the previous run
restore_and_save "$1/main.c"
restore_and_save "$1/dir.c"
restore_and_save "$1/dln.c"
restore_and_save "$1/file.c"
restore_and_save "$1/io.c"
restore_and_save "$1/util.c"
restore_and_save "$1/tool/mkconfig.rb"

# restore_and_save $1/process.c
# restore_and_save $1/prelude.c
# $1/ext/openssl/ossl_x509store.c

# [TODO Windows]
# $1/win32/file.c
# $1/win32/win32.c


