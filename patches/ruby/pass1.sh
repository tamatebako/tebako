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
  test -e $1.old && cp -f $1.old $1
  cp -f $1 $1.old
}

# ....................................................
# Pin tebako static build libraries
# Ruby 2.7.4:  template is in 'ruby/template/Makefile.in'
# Ruby 2.6.3:  template is in 'ruby/Makefile.in'
restore_and_save $1/template/Makefile.in

re="MAINLIBS = @MAINLIBS@"
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
MAINLIBS = -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \\\\
-l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \\\\
-l:libzstd.a -l:libgdbm.a -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:librt.a -lpthread -ldl -lc -lm \\\\
-lgcc_eh -l:libunwind.a -l:libcrypt.a -l:libanl.a -l:libstdc++.a -l:liblzma.a 
# -- End of tebako patch --
EOM

sed -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" $1/template/Makefile.in

# ....................................................
# Disable dynamic extensions
restore_and_save $1/ext/Setup
sed -i "s/\#option nodynamic/option nodynamic/g" $1/ext/Setup

# ....................................................
# ruby/lib/rubygems/path_support.rb
# WE DO NOT ACCEPT OUTSIDE GEM PATHS
restore_and_save $1/lib/rubygems/path_support.rb

re="  @home = env\[\"GEM_HOME\"\] || Gem.default_dir"
! IFS= read -r -d '' sbst << EOM
    @home = env\["GEM_HOME"\] || Gem.default_dir
# -- Start of tebako patch --
    unless env\['TEBAKO_PASS_THROUGH'\]
      @home = Gem.default_dir unless @home.index('\/__tebako_memfs__') == 0
    end
# -- End of tebako patch --
EOM

sed -i "s/$re/${sbst//$'\n'/"\\n"}/g" $1/lib/rubygems/path_support.rb

re="@path = split_gem_path env\[\"GEM_PATH\"\], @home"
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

sed -i "s/$re/${sbst//$'\n'/"\\n"}/g" $1/lib/rubygems/path_support.rb


# ....................................................
# This is something that I cannnot explain
# (this patch does not seem related to static compilation)
# ruby/ext/bigdecimal/bigdecimal.h
restore_and_save $1/ext/bigdecimal/bigdecimal.h
re="#include <float.h>"

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

sed -i "s/$re/${sbst//$'\n'/"\\n"}/g" $1/ext/bigdecimal/bigdecimal.h

# ....................................................
# Roll-back pass2 patches from the previous run
restore_and_save $1/main.c
restore_and_save $1/dir.c
restore_and_save $1/dln.c
restore_and_save $1/file.c
restore_and_save $1/io.c
restore_and_save $1/util.c
restore_and_save $1/tool/mkconfig.rb

# restore_and_save $1/process.c
# restore_and_save $1/prelude.c
# $1/ext/openssl/ossl_x509store.c

# [TODO Windows]
# $1/win32/file.c
# $1/win32/win32.c


