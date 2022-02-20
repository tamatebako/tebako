# shellcheck shell=bash
# Copyright (c) 2021-2022 [Ribose Inc](https://www.ribose.com).
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

set -o errexit -o pipefail -o noclobber -o nounset

# ....................................................
restore_and_save() {
  echo "Pass 2 processing $1"
  test -e "$1.old" && cp -f "$1.old" "$1"
  cp -f "$1" "$1.old"
}

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  gSed="sed"

# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \\\\
-l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \\\\
-l:libzstd.a -l:libgdbm.a -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:libcrypt.a -l:libanl.a -l:liblzma.a \\\\
-l:libboost_system.a -l:libstdc++.a -l:librt.a -ldl
# -- End of tebako patch --
EOM

elif [[ "$OSTYPE" == "darwin"* ]]; then
  gSed="gsed"
  tBrew="${3:-"guess"}"
  if [[ "$tBrew" == "guess" ]]; then
    tBrew="brew"
  else
    tBrew="$tBrew/bin/brew"
  fi
  p_libssl="$($tBrew --prefix openssl@1.1)/lib/libssl.a"
  p_libcrypto="$($tBrew --prefix openssl@1.1)/lib/libcrypto.a"
  p_libz="$($tBrew --prefix zlib)/lib/libz.a"
  p_libgdbm="$($tBrew --prefix gdbm)/lib/libgdbm.a"
  p_libreadline="$($tBrew --prefix readline)/lib/libreadline.a"
  p_libffi="$($tBrew --prefix libffi)/lib/libffi.a"
  p_libncurses="$($tBrew --prefix ncurses)/lib/libncurses.a"
  p_libfmt="$($tBrew --prefix fmt)/lib/libfmt.a"
  p_liblz4="$($tBrew --prefix lz4)/lib/liblz4.a"
  p_liblzma="$($tBrew --prefix xz)/lib/liblzma.a"
  p_libdc="$($tBrew --prefix double-conversion)/lib/libdouble-conversion.a"
# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = -ltebako-fs -ldwarfs-wr -ldwarfs -lfolly -lfsst -lmetadata_thrift -lthrift_light -lxxhash \\\\
-lzstd -lglog -lgflags $p_libfmt $p_liblz4 $p_liblzma $p_libdc \\\\
$p_libssl $p_libcrypto $p_libz $p_libgdbm $p_libreadline \\\\
$p_libffi $p_libncurses -ljemalloc -lc++
# -- End of tebako patch --
EOM

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
# Pin tebako static build libraries
# Ruby 2.7.4:  template is in 'ruby/template/Makefile.in'
# Ruby 2.6.3:  template is in 'ruby/Makefile.in'
restore_and_save "$1/template/Makefile.in"

re="MAINLIBS = @MAINLIBS@"
# shellcheck disable=SC2251

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

re="		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(XLDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG) \$(MAINLIBS) \$(LIBS) \$(EXTLIBS) \$(OUTFLAG)\$@"
if [[ "$OSTYPE" == "darwin"* ]]; then
# Ruby 2.7.5
# If cross compile sets XLDFLAGS to '-framework CoreFoundation Security'
# it is wrong syntax
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG) \$(MAINLIBS) \$(LIBS) \$(OUTFLAG)\$@
# -- End of tebako patch --
EOM
else
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(XLDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG) \$(MAINLIBS) \$(LIBS) \$(OUTFLAG)\$@
# -- End of tebako patch --
EOM
fi
#
"$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/template/Makefile.in"

# ....................................................
# Disable dynamic extensions
# ruby/ext/Setup
# Uses pass1 patch

# ....................................................
# WE DO NOT ACCEPT OUTSIDE GEM PATHS
# ruby/lib/rubygems/path_support.rb
# Uses pass1 patch

# ....................................................
# Patch main in order to redefine command line
restore_and_save "$1/main.c"
# Replace only the first occurence
# https://www.linuxtopia.org/online_books/linux_tool_guides/the_sed_faq/sedfaq4_004.html
# [TODO this looks a kind of risky]
"$gSed" -i "0,/int$/s//#include <tebako-main.h>\n\nint/" "$1/main.c"
"$gSed" -i "0,/{$/s//{\n    if (tebako_main(\&argc, \&argv) != 0) { return -1; }\n/" "$1/main.c"

# ....................................................
# Put lidwarfs IO bindings to other c files

patch_c_file() {
  restore_and_save "$1"

# shellcheck disable=SC2251
! IFS= read -r -d '' c_sbst << EOM

\/* -- Start of tebako patch -- *\/
#include <tebako\/tebako-defines.h>
#include <tebako\/tebako-io.h>
\/* -- End of tebako patch -- *\/

EOM

  sbst="${c_sbst}$2"
  "$gSed" -i "0,/$2/s//${sbst//$'\n'/"\\n"}/g" "$1"
}

# ruby/dln.c
patch_c_file "$1/dln.c"  "static const char funcname_prefix\[sizeof(FUNCNAME_PREFIX) - 1\] = FUNCNAME_PREFIX;"

# ruby/file.c
patch_c_file "$1/file.c"  "VALUE rb_cFile;"

# ruby/io.c
patch_c_file "$1/io.c"  "VALUE rb_cIO;"

# ruby/util.c
patch_c_file "$1/util.c"  "#ifndef S_ISDIR"

# ....................................................
# ruby/dir.c
restore_and_save "$1/dir.c"
# Replace only the first occurence
# As opposed to other c files subsitution inserts includes before the pattern, not after

re="#ifdef __APPLE__"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM

\/* -- Start of tebako patch -- *\/
#include <tebako\/tebako-defines.h>
#include <tebako\/tebako-io.h>
\/* -- End of tebako patch -- *\/

#ifdef __APPLE__
EOM

"$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/dir.c"

# Compensate ruby incorrect processing of (f)getattrlist returning ENOTSUP
"$gSed" -i "s/if ((\*cur)->type == ALPHA) {/if ((*cur)->type == ALPHA \/* tebako patch *\/ \&\& !within_tebako_memfs(buf)) {/g" "$1/dir.c"
"$gSed" -i "s/else if (e == EIO) {/else if (e == EIO \/* tebako patch *\/ \&\& !within_tebako_memfs(path)) {/g" "$1/dir.c"
"$gSed" -i "s/if (is_case_sensitive(dirp, path) == 0)/if (is_case_sensitive(dirp, path) == 0 \/* tebako patch *\/ \&\& !within_tebako_memfs(path))/g" "$1/dir.c"
"$gSed" -i "0,/plain = 1;/! s/plain = 1;/\/* tebako patch *\/ if (!within_tebako_memfs(path)) plain = 1; else magical = 1;/g" "$1/dir.c"

re="#if defined HAVE_GETATTRLIST && defined ATTR_DIR_ENTRYCOUNT"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
#if defined HAVE_GETATTRLIST \&\& defined ATTR_DIR_ENTRYCOUNT
\/* tebako patch *\/ if (!within_tebako_memfs(path))
EOM

"$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/dir.c"

# Note. We are not patching need_normalization function
# In this function (f)getattrlist failure with ENOTSUP is processed correctly

# ....................................................
# ruby/tool/mkconfig.rb
restore_and_save "$1/tool/mkconfig.rb"
re="if fast\[name\]"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
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

"$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/tool/mkconfig.rb"

# ruby/ext/openssl/ossl_x509store.c
#  [TODO ???]

# ruby/prelude.c
# restore_and_save $1/prelude.c
# [TODO ???]

# ruby/process.c
#restore_and_save $1/process.c
# [TODO ???]

# [TODO Windows]
# $1/win32/file.c
# $1/win32/win32.c
