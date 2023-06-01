# frozen_string_literal: true

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

LINUX_GNU_LIBS = <<~SUBST
  # -- Start of tebako patch --
  MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \
  -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \
  -l:libzstd.a -l:libgdbm.a -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:libcrypt.a -l:libanl.a -l:liblzma.a \
  -l:libboost_system.a -l:libstdc++.a -l:librt.a -ldl
  # -- End of tebako patch --
SUBST

LINUX_MUSL_LIBS = <<~SUBST
  # -- Start of tebako patch --
  MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \
  -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \
  -l:libzstd.a -l:libgdbm.a -l:libreadline.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:libcrypt.a -l:liblzma.a \
  -l:libboost_system.a -l:libstdc++.a -l:librt.a -ldl
  # -- End of tebako patch --
SUBST

MSYS_LIBS = <<~SUBST
  # -- Start of tebako patch --
  MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \
  -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \
  -l:libzstd.a -l:libffi.a -l:libgdbm.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:liblzma.a -l:libiberty.a \
  -l:libstdc++.a -l:libdl.a -lole32 -loleaut32 -luuid
  # -- End of tebako patch --
SUBST

TOOL_MKCONFIG_RB_PATCH = {
  '    if fast[name]' => <<~SUBST
    # -- Start of tebako patch --
        v_head_comp = "  CONFIG[\\"prefix\\"] \#{eq} "
        if v_head_comp == v[0...(v_head_comp.length)]
          if win32
           v = "\#{v[0...(v_head_comp.length)]}CONFIG[\\"RUBY_EXEC_PREFIX\\"] = '/__tebako_memfs__'
    "
          else
            v = "\#{v[0...(v_head_comp.length)]}'/__tebako_memfs__'
    "
          end
        end
        v_head_comp = "  CONFIG[\\"RUBY_EXEC_PREFIX\\"] \#{eq} "
        if v_head_comp == v[0...(v_head_comp.length)]
          v = "\#{v[0...(v_head_comp.length)]}'/__tebako_memfs__'
    "
        end
    # -- End of tebako patch --
        if fast[name]
  SUBST
}.freeze

# Alpine-specific patches https://github.com/docker-library/ruby/blob/master/3.1/alpine3.15/Dockerfile
# -- Patch no. 1 --
# https://github.com/docker-library/ruby/issues/196
# https://bugs.ruby-lang.org/issues/14387#note-13 (patch source)
# https://bugs.ruby-lang.org/issues/14387#note-16
# ("Therefore ncopa's patch looks good for me in general." -- only breaks glibc which doesn't matter here)

# -- Patch no. 2 -- not needed: isnan, isinf are detected correctly by configure script
# the configure script does not detect isnan/isinf as macros
#	export ac_cv_func_isnan=yes ac_cv_func_isinf=yes;

#	-- Patch no. 3 --
# hack in "ENABLE_PATH_CHECK" disabling to suppress:  "warning: Insecure world writable dir"
# Applied in CMakeLists.txt

LINUX_MUSL_THREAD_PTHREAD_PATCH = {
  '#if MAINSTACKADDR_AVAILABLE && !defined(get_main_stack)' => <<~SUBST
    \/* -- Start of tebako patch -- *\/
    #if defined(__linux__) && !defined(__GLIBC__) && defined(HAVE_GETRLIMIT)
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
        f = fopen("/proc/self/maps", "re");
        if (!f)
            return -1;
        n = 0;
        while (fgets(buf, sizeof buf, f)) {
            n = sscanf(buf, "%zx-%zx %*s %*s %*s %*s %7s", &start, &end, s);
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
        if (getrlimit(RLIMIT_STACK, &r)==0 && r.rlim_cur < limit)
            limit = r.rlim_cur & -PAGE_SIZE;
        if (limit > end) limit = end;
        if (prevend < end - limit) prevend = end - limit;
        if (start > prevend) start = prevend;
        *addr = IS_STACK_DIR_UPPER() ? (void *)start : (void *)end;
        *size = end - start;
        return 0;
    }
    #else
    \/* -- End of tebako patch -- *\/
  SUBST
}.freeze

MAIN_C_PATCH = {
  "int\nmain(int argc, char **argv)" =>
  "#include <tebako-main.h>\n\nint\nmain(int argc, char **argv)",

  '    ruby_sysinit(&argc, &argv);' => <<~SUBST
        ruby_sysinit(&argc, &argv);
    /* -- Start of tebako patch -- */
        if (tebako_main(&argc, &argv) != 0) {
          return -1;
        }
    /* -- End of tebako patch -- */
  SUBST
}.freeze

# Compensate ruby incorrect processing of (f)getattrlist returning ENOTSUP
# Note. We are not patching need_normalization function
# In this function (f)getattrlist failure with ENOTSUP is processed correctly

DIR_C_BASE_PATCH_ONE = <<~SUBST
  #if defined HAVE_GETATTRLIST && defined ATTR_DIR_ENTRYCOUNT
      /* tebako patch */ if (!within_tebako_memfs(path))
SUBST

DIR_C_BASE_PATCH_TWO = <<~SUBST
  #if USE_NAME_ON_FS == USE_NAME_ON_FS_REAL_BASENAME
        /* tebako patch */ if (!within_tebako_memfs(path)) plain = 1; else magical = 1;",
SUBST

DIR_C_BASE_PATCH = {
  '#if defined HAVE_GETATTRLIST && defined ATTR_DIR_ENTRYCOUNT' => DIR_C_BASE_PATCH_ONE,
  "#if USE_NAME_ON_FS == USE_NAME_ON_FS_REAL_BASENAME\n	    plain = 1;" => DIR_C_BASE_PATCH_TWO,
  'if (is_case_sensitive(dirp, path) == 0)' =>
    "if (is_case_sensitive(dirp, path) == 0 \/* tebako patch *\/ && !within_tebako_memfs(path))",
  'if ((*cur)->type == ALPHA) {' =>
    "if ((*cur)->type == ALPHA \/* tebako patch *\/ && !within_tebako_memfs(buf)) {",
  'else if (e == EIO) {' =>
    "else if (e == EIO \/* tebako patch *\/ && !within_tebako_memfs(path)) {"
}.freeze

def get_prefix(package)
  out, st = capture2("brew --prefix #{package}")
  raise TebakoError, "brew --prefix #{package} failed with code #{st.exitstatus}" unless st.exitstatus.zero?

  out
end

DARWIN_BREW_LIBS = [
  %w[openssl@1.1 ssl],   %w[openssl@1.1 crypto],
  %w[zlib z],            %w[gdbm gdbm],
  %w[readline readline], %w[ffi ffi],
  %w[ncurses ncurses],   %w[fmt fmt],
  %w[lz4 lz4],           %w[xz lzma],
  %w[double-conversion double-conversion]
].freeze

DARWIN_DEP_LIBS = %w[glog gflags].freeze

def darwin_libs(deps_lib_dir)
  libs = ''
  DARWIN_BREW_LIBS.each do |lib|
    libs << get_prefix(lib[0]) << "/lib/lib#{lib[1]}.a "
  end
  DARWIN_DEP_LIBS.each do |lib|
    libs << deps_lib_dir << "/lib#{lib}.a "
  end
  <<~SUBST
    # -- Start of tebako patch --
    MAINLIBS = -ltebako-fs -ldwarfs-wr -ldwarfs -lfolly -lfsst -lmetadata_thrift -lthrift_light -lxxhash \
    -lzstd #{libs} -ljemalloc -lc++
    # -- End of tebako patch --
  SUBST
end

def mlibs(ostype, deps_lib_dir)
  case ostype
  when /linux-gnu/
    LINUX_GNU_LIBS
  when /linux-musl/
    LINUX_MUSL_LIBS
  when /darwin/
    darwin_libs(deps_lib_dir)
  when /msys/
    MSYS_LIBS
  else
    raise TebakoError, "Unknown ostype #{ostype}"
  end
end

TEMPLATE_MAKEFILE_IN_BASE_PATCH_ONE = <<~SUBST
  # -- Start of tebako patch --
  LIBS = $(MAINLIBS) @LIBS@
  # -- End of tebako patch --
SUBST

TEMPLATE_MAKEFILE_IN_BASE_PATTERN_TWO =
  "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(XLDFLAGS) $(MAINOBJ) " \
  '$(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) $(LIBS) $(EXTLIBS) $(OUTFLAG)$@'

TEMPLATE_MAKEFILE_IN_BASE_PATCH_TWO =
  "# -- Start of tebako patch --\n" \
  "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(XLDFLAGS) $(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG_STATIC) $(LIBS) $(OUTFLAG)$@\n" \
  '# -- End of tebako patch --'
