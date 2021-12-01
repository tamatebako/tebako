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
cp -f $PATCH_DIR/mainlibs-pass1.mk $2/mainlibs-pass1.mk

# Pin tebako static build libraries
# Ruby 2.7.4:
restore_and_save $1/template/Makefile.in
sed -i "s/MAINLIBS = @MAINLIBS@/include  mainlibs-pass1.mk/g" $1/template/Makefile.in
# Ruby 2.6.3:
#restore_and_save $1/Makefile.in
#sed -i "s/MAINLIBS = @MAINLIBS@/include  mainlibs-pass1.mk/g" $1/Makefile.in

# Fix bigdecimal extension
# [I cannot explain why it is required. It does not seem to be related to any patching we do]
cp -f $PATCH_DIR/bigdecimal-patch.h $1/ext/bigdecimal/bigdecimal-patch.h
restore_and_save $1/ext/bigdecimal/bigdecimal.h
sed -i "s/#include <float.h>/#include <float.h>\n#include \"bigdecimal-patch.h\"\n/g" $1/ext/bigdecimal/bigdecimal.h

# Disable dynamic extensions
restore_and_save $1/ext/Setup
sed -i "s/\#option nodynamic/option nodynamic/g" $1/ext/Setup

restore_and_save $1/main.c
restore_and_save $1/dir.c
restore_and_save $1/dln.c
restore_and_save $1/file.c
restore_and_save $1/io.c

# WE DO NOT ACCEPT OUTSIDE GEM PATHS
# ruby/lib/rubygems/path_support.rb
restore_and_save $1/lib/rubygems/path_support.rb

re="  @home = env\[\"GEM_HOME\"\] || Gem.default_dir"
IFS= read -r -d '' sbst << EOM
    @home = env\["GEM_HOME"\] || Gem.default_dir
# -- Start of tebako patch --
    unless env\['TEBAKO_PASS_THROUGH'\]
      @home = Gem.default_dir unless @home.index('\/__tebako_memfs__') == 0
    end
# -- End of tebako patch --
EOM

sed -i "s/$re/${sbst//$'\n'/"\\n"}/g" $1/lib/rubygems/path_support.rb

re="@path = split_gem_path env\[\"GEM_PATH\"\], @home"
IFS= read -r -d '' sbst << EOM
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

restore_and_save $1/process.c
restore_and_save $1/tool/mkconfig.rb
restore_and_save $1/util.c
