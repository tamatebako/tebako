# shellcheck shell=bash
# Copyright (c) 2021-2023 [Ribose Inc](https://www.ribose.com).
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
  echo "Pass 2 processing $1"
  test -e "$1.old" && cp -f "$1.old" "$1"
  cp -f "$1" "$1.old"
}

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  mPoint="\/__tebako_memfs__"
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

elif [[ "$OSTYPE" == "linux-musl"* ]]; then
  mPoint="\/__tebako_memfs__"
  gSed="sed"

# Alpine-specific patches https://github.com/docker-library/ruby/blob/master/3.1/alpine3.15/Dockerfile
# -- Patch no. 1 --
# https://github.com/docker-library/ruby/issues/196
# https://bugs.ruby-lang.org/issues/14387#note-13 (patch source)
# https://bugs.ruby-lang.org/issues/14387#note-16 ("Therefore ncopa's patch looks good for me in general." -- only breaks glibc which doesn't matter here)
#	wget -O 'thread-stack-fix.patch' 'https://bugs.ruby-lang.org/attachments/download/7081/0001-thread_pthread.c-make-get_main_stack-portable-on-lin.patch'; \
#	echo '3ab628a51d92fdf0d2b5835e93564857aea73e0c1de00313864a94a6255cb645 *thread-stack-fix.patch' | sha256sum --check --strict; \
#	patch -p1 -i thread-stack-fix.patch; \
#	rm thread-stack-fix.patch;
  restore_and_save "$1/thread_pthread.c"
  re="#if MAINSTACKADDR_AVAILABLE && !defined(get_main_stack)"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
\/* -- Start of tebako patch -- *\/
#if defined(__linux__) \&\& !defined(__GLIBC__) \&\& defined(HAVE_GETRLIMIT)
#ifndef PAGE_SIZE
#include <unistd.h>
#define PAGE_SIZE sysconf(_SC_PAGE_SIZE)
#endif
static int
get_main_stack(void **addr, size_t *size)
{
    size_t start, end, limit, prevend = 0;
    struct rlimit r;
    FILE *f;
    char buf[PATH_MAX+80], s[8];
    int n;
    STACK_GROW_DIR_DETECTION;
    f = fopen(\"\/proc\/self\/maps\", \"re\");
    if (!f)
        return -1;
    n = 0;
    while (fgets(buf, sizeof buf, f)) {
        n = sscanf(buf, \"%zx-%zx %*s %*s %*s %*s %7s\", \&start, \&end, s);
        if (n >= 2) {
            if (n == 3 \&\& strcmp(s, \"[stack]\") == 0)
                break;
            prevend = end;
        }
        n = 0;
    }
    fclose(f);
    if (n == 0)
        return -1;
    limit = 100 << 20; \/* 100MB stack limit *\/
    if (getrlimit(RLIMIT_STACK, \&r)==0 \&\& r.rlim_cur < limit)
        limit = r.rlim_cur \& -PAGE_SIZE;
    if (limit > end) limit = end;
    if (prevend < end - limit) prevend = end - limit;
    if (start > prevend) start = prevend;
    *addr = IS_STACK_DIR_UPPER() ? (void *)start : (void *)end;
    *size = end - start;
    return 0;
}
#else
\/* -- End of tebako patch -- *\/
EOM

  "$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/thread_pthread.c"

# -- Patch no. 2 -- not needed: isnan, isinf are detected correctly by configure script
# the configure script does not detect isnan/isinf as macros
#	export ac_cv_func_isnan=yes ac_cv_func_isinf=yes;

#	-- Patch no. 3 --
# hack in "ENABLE_PATH_CHECK" disabling to suppress:  "warning: Insecure world writable dir"
# Applied in CMakeLists.txt

# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \\\\
-l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \\\\
-l:libzstd.a -l:libgdbm.a -l:libreadline.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:libcrypt.a -l:liblzma.a \\\\
-l:libboost_system.a -l:libstdc++.a -l:librt.a -ldl
# -- End of tebako patch --
EOM

elif [[ "$OSTYPE" == "darwin"* ]]; then
  mPoint="\/__tebako_memfs__"
  gSed="gsed"
  p_libssl="$(brew --prefix openssl@1.1)/lib/libssl.a"
  p_libcrypto="$(brew --prefix openssl@1.1)/lib/libcrypto.a"
  p_libz="$(brew --prefix zlib)/lib/libz.a"
  p_libgdbm="$(brew --prefix gdbm)/lib/libgdbm.a"
  p_libreadline="$(brew --prefix readline)/lib/libreadline.a"
  p_libffi="$(brew --prefix libffi)/lib/libffi.a"
  p_libncurses="$(brew --prefix ncurses)/lib/libncurses.a"
  p_libfmt="$(brew --prefix fmt)/lib/libfmt.a"
  p_liblz4="$(brew --prefix lz4)/lib/liblz4.a"
  p_liblzma="$(brew --prefix xz)/lib/liblzma.a"
  p_libdc="$(brew --prefix double-conversion)/lib/libdouble-conversion.a"
  p_glog="$2/lib/libglog.a"
  p_gflags="$2/lib/libgflags.a"
# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = -ltebako-fs -ldwarfs-wr -ldwarfs -lfolly -lfsst -lmetadata_thrift -lthrift_light -lxxhash \\\\
-lzstd $p_glog $p_gflags $p_libfmt $p_liblz4 $p_liblzma $p_libdc \\\\
$p_libssl $p_libcrypto $p_libz $p_libgdbm $p_libreadline \\\\
$p_libffi $p_libncurses -ljemalloc -lc++
# -- End of tebako patch --
EOM

elif [[ "$OSTYPE" == "msys"* ]]; then
  mPoint="A:\/__tebako_memfs__"
  gSed="sed"

# shellcheck disable=SC2251
! IFS= read -r -d '' mLibs << EOM
# -- Start of tebako patch --
MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \\\\
-l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \\\\
-l:libzstd.a -l:libffi.a -l:libgdbm.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:liblzma.a -l:libiberty.a \\\\
-l:libstdc++.a -l:libdl.a -lole32 -loleaut32 -luuid
# -- End of tebako patch --
EOM
# win32ole extension requirements: -lole32 -loleaut32 -luuid
# Disabled extensions (gdbm, readline): -l:libgdbm.a -l:libreadline.a
# Looks like not required:  -l:libwinpthread.a
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

"$gSed" -i "0,/$re/s||${mLibs//$'\n'/"\\n"}|g" "$1/template/Makefile.in"

re="LIBS = @LIBS@ \$(EXTLIBS)"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
LIBS = \$(MAINLIBS) @LIBS@
# -- End of tebako patch --
EOM
#
"$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/template/Makefile.in"

re="		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(XLDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG) \$(MAINLIBS) \$(LIBS) \$(EXTLIBS) \$(OUTFLAG)\$@"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
# -- Start of tebako patch --
#		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(XLDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG_STATIC) \$(MAINLIBS) \$(LIBS) \$(OUTFLAG)\$@
		\$(Q) \$(PURIFY) \$(CC) \$(LDFLAGS) \$(XLDFLAGS) \$(MAINOBJ) \$(EXTOBJS) \$(LIBRUBYARG_STATIC) \$(LIBS) \$(OUTFLAG)\$@
# -- End of tebako patch --
EOM
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

re="    ruby_sysinit(&argc, &argv);"
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
    ruby_sysinit(\&argc, \&argv);
\/* -- Start of tebako patch -- *\/
    if (tebako_main(\&argc, \&argv) != 0) {
      return -1;
    }
\/* -- End of tebako patch -- *\/

EOM
  "$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/main.c"

# ....................................................
# Put lidwarfs IO bindings to other c files

patch_c_file() {
  restore_and_save "$1"

# shellcheck disable=SC2251
! IFS= read -r -d '' c_sbst << EOM

\/* -- Start of tebako patch -- *\/
#ifndef NO_TEBAKO_INCLUDES
#include <tebako\/tebako-config.h>
#include <tebako\/tebako-defines.h>
#include <tebako\/tebako-io-rb-w32.h>
#include <tebako\/tebako-io.h>
#endif
\/* -- End of tebako patch -- *\/

EOM

  sbst="${c_sbst}$2"
  "$gSed" -i "0,/$2/s//${sbst//$'\n'/"\\n"}/g" "$1"
}

# ruby/dln.c
patch_c_file "$1/dln.c"  "static const char funcname_prefix\[sizeof(FUNCNAME_PREFIX) - 1\] = FUNCNAME_PREFIX;"

# ruby/file.c
patch_c_file "$1/file.c"  "\/\* define system APIs \*\/"

# ruby/io.c
patch_c_file "$1/io.c"  "\/\* define system APIs \*\/"

# ruby/util.c
patch_c_file "$1/util.c"  "#ifndef S_ISDIR"

# ....................................................
# ruby/dir.c
if [[ "$OSTYPE" == "msys"* ]]; then
  patch_c_file "$1/dir.c"  "\/\* define system APIs \*\/"
else
  patch_c_file "$1/dir.c"  "#ifdef HAVE_GETATTRLIST"
fi

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
         v = \"#{v\[0...(v_head_comp.length)\]}CONFIG\[\\\\\"RUBY_EXEC_PREFIX\\\\\"\] = '$mPoint'\\n\"
       else
         v = "#{v\[0...(v_head_comp.length)\]}'$mPoint'\\n\"
       end
     end
     v_head_comp = \"  CONFIG\[\\\\\"RUBY_EXEC_PREFIX\\\\\"\] #{eq} \"
     if v_head_comp == v\[0...(v_head_comp.length)\]
       v = \"#{v\[0...(v_head_comp.length)\]}'$mPoint'\\n\"
     end
# -- End of tebako patch --
     if fast\[name\]
EOM

"$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/tool/mkconfig.rb"

if [[ "$OSTYPE" == "msys"* ]]; then
# ....................................................
# ruby/ruby.c
  restore_and_save "$1/ruby.c"
  re="#define RUBY_RELATIVE(path, len) rb_str_buf_cat(BASEPATH(), (path), (len))"
  sbst="#define RUBY_RELATIVE(path, len) rubylib_path_new((path), (len))  \/* tebako patched *\/"
  "$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/ruby.c"

  re="#define PREFIX_PATH() sopath"
  sbst="#define PREFIX_PATH() rubylib_path_new((tebako_mount_point()), (strlen(tebako_mount_point())))  \/* tebako patched *\/"
  "$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/ruby.c"

    re="#include \"mjit.h\""
# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
#include \"mjit.h\"
\/* -- Start of tebako patch -- *\/
#include <tebako-main.h>
\/* -- End of tebako patch -- *\/
EOM
  "$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/ruby.c"


# ....................................................
# ruby/win32/win32.c
  restore_and_save "$1/win32/win32.c"
  re="#undef __STRICT_ANSI__"

# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
#undef __STRICT_ANSI__
\/* -- Start of tebako patch -- *\/
#define NO_TEBAKO_INCLUDES
\/* -- End of tebako patch -- *\/
EOM

  "$gSed" -i "0,/$re/s//${sbst//$'\n'/"\\n"}/g" "$1/win32/win32.c"

# ....................................................
# ruby/win32/dir.h
  patch_c_file "$1/win32/dir.h"  "#define opendir(s)   rb_w32_opendir((s))"

# ....................................................
# ruby/win32/file.c
  restore_and_save "$1/win32/file.c"

  re="    wpath = mbstr_to_wstr(CP_UTF8, path, -1, &len);"

# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
    \/* -- Start of tebako patch -- *\/
    if (tebako_file_load_ok(path)) return 1;
    \/* -- End of tebako patch -- *\/
    wpath = mbstr_to_wstr(CP_UTF8, path, -1, \&len);
EOM
  "$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/win32/file.c"

  re="#include \"win32\/file.h\""

# shellcheck disable=SC2251
! IFS= read -r -d '' sbst << EOM
#include \"win32\/file.h\"
\/* -- Start of tebako patch -- *\/
#include <tebako-main.h>
\/* -- End of tebako patch -- *\/
EOM
  "$gSed" -i "s/$re/${sbst//$'\n'/"\\n"}/g" "$1/win32/file.c"

fi

# ....................................................
# ruby/ext/bigdecimal/bigdecimal.h
# Uses pass1 patch

# ....................................................
# ruby/configure
# Uses pass1 patch

# ....................................................
# ruby/cygwin/GNUmakefile.in
# Uses pass1 patch

# ruby/prelude.c
# restore_and_save $1/prelude.c
# [TODO ???]

# ruby/process.c
#restore_and_save $1/process.c
# [TODO ???]
