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


# Copy make script include file that list all libraries required for tebako static build
PATCH_DIR="$( cd "$( dirname "$0" )" && pwd )"
cp $PATCH_DIR/mainlibs.mk $2/mainlibs.mk 

# Pin tebako static build libraries
cp $1/template/Makefile.in $1/template/Makefile.in.old
sed -i "s/MAINLIBS = @MAINLIBS@/include  mainlibs.mk/g" $1/template/Makefile.in


cp $PATCH_DIR/bd-patch.h $1/ext/bigdecimal/bd-patch.h
cp $1/ext/bigdecimal/bigdecimal.h $1/ext/bigdecimal/bigdecimal.h.old
sed -i "s/#include <float.h>/#include <float.h>\n#include \"bd-patch.h\"\n/g" $1/ext/bigdecimal/bigdecimal.h 


# [????] not sure if it is required
cp $1/ext/Setup $1/ext/Setup.old
sed -i "s/\#option nodynamic/option nodynamic/g" $1/ext/Setup

# Put lidwarfs IO bindings to Ruby files
cp $1/io.c $1/io.c.old
sed -i "s/VALUE rb_cIO;/#include <tebako\/tebako-defines.h>\n#include <tebako\/tebako-io.h>\n\nVALUE rb_cIO;/g" $1/io.c

