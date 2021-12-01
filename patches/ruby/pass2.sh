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

restore_and_save() {
  test -e $1.old && cp -f $1.old $1
  cp -f $1 $1.old
}

# Copy make script include file that list all libraries required for tebako static build
PATCH_DIR="$( cd "$( dirname "$0" )" && pwd )"
cp -f $PATCH_DIR/mainlibs-pass2.mk $2/mainlibs-pass2.mk

# Pin tebako static build libraries
# Ruby 2.7.4:
restore_and_save $1/template/Makefile.in
sed -i "s/MAINLIBS = @MAINLIBS@/include  mainlibs-pass2.mk/g" $1/template/Makefile.in

# Fix bigdecimal extension
# [I cannot explain why it is required. It does not seem to be related to any patching we do]
cp -f $PATCH_DIR/bigdecimal-patch.h $1/ext/bigdecimal/bigdecimal-patch.h
restore_and_save $1/ext/bigdecimal/bigdecimal.h
sed -i "s/#include <float.h>/#include <float.h>\n#include \"bigdecimal-patch.h\"\n/g" $1/ext/bigdecimal/bigdecimal.h

# Disable dynamic extensions
restore_and_save $1/ext/Setup
sed -i "s/\#option nodynamic/option nodynamic/g" $1/ext/Setup

# Patch main in order to redefine command line
restore_and_save $1/main.c
# Replace only the first occurence
# https://www.linuxtopia.org/online_books/linux_tool_guides/the_sed_faq/sedfaq4_004.html
# [TODO this looks a kind of risky]
sed -i "0,/int$/s//#include <tebako-main.h>\n\nint/" $1/main.c
sed -i "0,/{$/s//{\n    if (tebako_main(\&argc, \&argv) != 0) { return -1; }\n/" $1/main.c

# ....................................................
# Put lidwarfs IO bindings to Ruby files

# ruby/dir.c
restore_and_save $1/dir.c
# Replace only the first occurence
sed -i "0,/#ifdef __APPLE__/s//#include <tebako\/tebako-defines.h>\n#include <tebako\/tebako-io.h>\n\n#ifdef __APPLE__/" $1/dir.c
#  [TODO MacOS]  libdwarfs issues 45,46

# Addition to C files
IFS= read -r -d '' c_sbst << EOM

\/* -- Start of tebako patch -- *\/
#include <tebako\/tebako-defines.h>
#include <tebako\/tebako-io.h>
\/* -- End of tebako patch -- *\/

EOM

patch_c_file() {
  restore_and_save $1
  sbst="${c_sbst}$2"
  sed -i "0,/$2/s//${sbst//$'\n'/"\\n"}/g" $1
}

# ruby/dln.c
patch_c_file "$1/dln.c"  "static const char funcname_prefix\[sizeof(FUNCNAME_PREFIX) - 1\] = FUNCNAME_PREFIX;"

# ruby/file.c
patch_c_file "$1/file.c"  "VALUE rb_cFile;"

# ruby/io.c
patch_c_file "$1/io.c"  "VALUE rb_cIO;"

# ruby/util.c
patch_c_file "$1/util.c"  "#ifndef S_ISDIR"

# ruby/tool/mkconfig.rb
restore_and_save $1/tool/mkconfig.rb
re="if fast\[name\]"
IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
     v_head_comp = \"  CONFIG\[\\\\\"prefix\\\\\"\] #{eq} \"
     if v_head_comp == v\[0...(v_head_comp.length)\]
       if win32
         v = \"#{v\[0...(v_head_comp.length)\]}CONFIG\[\\\\\"RUBY_EXEC_PREFIX\\\\\"\] = '\/__tebako_memfs__'\\n\"
       else
         v = "#{v\[0...(v_head_comp.length)\]}'\/__tebako_memfs__'\\n\"
       end
     end
     v_head_comp = \"  CONFIG\[\\\\\"RUBY_EXEC_PREFIX\\\\\"\] #{eq} \"
     if v_head_comp == v\[0...(v_head_comp.length)\]
       v = \"#{v\[0...(v_head_comp.length)\]}'\/__tebako_memfs__'\\n\"
     end
# -- End of tebako patch --
     if fast\[name\]
EOM

sed -i "s/$re/${sbst//$'\n'/"\\n"}/g" $1/tool/mkconfig.rb

# ruby/ext/openssl/ossl_x509store.c
#  [TODO ???]

# ruby/prelude.c
# restore_and_save $1/prelude.c
# [TODO ???]

# ruby/process.c
#restore_and_save $1/process.c
# [TODO ???]

# [TODO Windows]
# ruby/win32/file.c
# ruby/win32/win32.c
