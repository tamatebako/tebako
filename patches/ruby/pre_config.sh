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
cp -f $PATCH_DIR/mainlibs.mk $2/mainlibs.mk 

# Pin tebako static build libraries
restore_and_save $1/template/Makefile.in
sed -i "s/MAINLIBS = @MAINLIBS@/include  mainlibs.mk/g" $1/template/Makefile.in

# Fix bigdecimal extension
# [I cannot explain why it is required. It does not semm to be related to any patching we do]
cp -f $PATCH_DIR/bigdecimal-patch.h $1/ext/bigdecimal/bigdecimal-patch.h
restore_and_save $1/ext/bigdecimal/bigdecimal.h
sed -i "s/#include <float.h>/#include <float.h>\n#include \"bigdecimal-patch.h\"\n/g" $1/ext/bigdecimal/bigdecimal.h 

# Disable dynamic extensions
restore_and_save $1/ext/Setup
sed -i "s/\#option nodynamic/option nodynamic/g" $1/ext/Setup

# ....................................................
# Patch main in order to redefine command line
restore_and_save $1/main.c
# Replace only the first occurence
# https://www.linuxtopia.org/online_books/linux_tool_guides/the_sed_faq/sedfaq4_004.html
# [TODO this looks a kind of risky]
sed -i "0,/int$/s//#include <tebako-main.h>\n\nint/" $1/main.c

# ....................................................
# Put lidwarfs IO bindings to Ruby files

# ruby/dir.c
restore_and_save $1/dir.c
# Replace only the first occurence
sed -i "0,/#ifdef __APPLE__/s//#include <tebako\/tebako-defines.h>\n#include <tebako\/tebako-io.h>\n\n#ifdef __APPLE__/" $1/dir.c
#  [TODO MacOS]  libdwarfs issues 45,46

# ruby/dln.c
restore_and_save $1/dln.c
sed -i "s/static st_table \*sym_tbl;/#include <tebako\/tebako-defines.h>\n#include <tebako\/tebako-io.h>\n\nstatic st_table *sym_tbl;/g" $1/dln.c


# ruby/ext/openssl/ossl_x509store.c
#  [TODO ???]

# ruby/file.c
restore_and_save $1/file.c
sed -i "s/VALUE rb_cFile;/#include <tebako\/tebako-defines.h>\n#include <tebako\/tebako-io.h>\n\nVALUE rb_cFile;/g" $1/file.c

# ruby/io.c
restore_and_save $1/io.c
sed -i "s/VALUE rb_cIO;/#include <tebako\/tebako-defines.h>\n#include <tebako\/tebako-io.h>\n\nVALUE rb_cIO;/g" $1/io.c

# ruby/lib/rubygems/path_support.rb 
# restore_and_save $1/lib/rubygems/path_support.rb
# #  [TODO ???]


# ruby/prelude.c        
# restore_and_save $1/prelude.c            
# [TODO ???]


# ruby/process.c             
restore_and_save $1/process.c       
# [TODO ???]

# ruby/tool/mkconfig.rb             
restore_and_save $1/tool/mkconfig.rb       
# [TODO ???]

# ruby/util.c
restore_and_save $1/util.c
sed -i  "s/#ifndef S_ISDIR/#include <tebako\/tebako-defines.h>\n#include <tebako\/tebako-io.h>\n\n#ifndef S_ISDIR/g" $1/util.c


# [TODO Windows]
# ruby/win32/file.c
# ruby/win32/win32.c

