#!/bin/bash
# Copyright (c) 2021, [Ribose Inc](https://www.ribose.com).
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

# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = -l:libssl.a -l:libcrypto.a -l:libz.a -l:libgdbm.a -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a \\\\
-l:libjemalloc.a -l:libcrypt.a -l:libanl.a -l:librt.a -ldl 
# -- End of tebako patch --
EOM

elif [[ "$OSTYPE" == "darwin"* ]]; then
  gSed="gsed"
  p_libssl="$(brew --prefix openssl@1.1)/lib/libssl.a"
  p_libcrypto="$(brew --prefix openssl@1.1)/lib/libcrypto.a"
  p_libz="$(brew --prefix zlib)/lib/libz.a"
  p_libgdbm="$(brew --prefix gdbm)/lib/libgdbm.a"
  p_libreadline="$(brew --prefix readline)/lib/libreadline.a"
  p_libffi="$(brew --prefix libffi)/lib/libffi.a"
  p_libncurses="$(brew --prefix ncurses)/lib/libncurses.a"
#  p_libjemalloc="$(brew --prefix jemalloc)/lib/libjemalloc.a"
# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = $p_libssl $p_libcrypto $p_libz $p_libgdbm $p_libreadline \\\\
$p_libffi $p_libncurses -ljemalloc -ldl
# -- End of tebako patch --
EOM

else
  exit 1
fi  


# ....................................................
# Pin tebako static build libraries
# Ruby 2.7.4:  template is in 'ruby/template/Makefile.in'
# Ruby 2.6.3:  template is in 'ruby/Makefile.in'
restore_and_save "$1/template/Makefile.in"

re="MAINLIBS = @MAINLIBS@"
# shellcheck disable=SC2251
#! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
#MAINLIBS = -l:/opt/homebrew/opt/openssl@1.1/lib/libssl.a -l:libcrypto.a -l:libz.a -l:libgdbm.a -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a \\\\
#-l:libjemalloc.a -l:libcrypt.a -l:libanl.a -l:librt.a -ldl 
# -- End of tebako patch --
#EOM

#
"$gSed" -i "0,/$re/s||${mLibs//$'\n'/"\\n"}|g" "$1/template/Makefile.in"

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


