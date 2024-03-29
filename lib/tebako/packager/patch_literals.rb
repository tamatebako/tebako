# frozen_string_literal: true

# Copyright (c) 2023 [Ribose Inc](https://www.ribose.com).
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

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching literals (pass2)
    # rubocop:disable Metrics/ModuleLength
    module PatchLiterals
      TOOL_MKCONFIG_RB_PATCH = {
        "    if fast[name]" => <<~SUBST
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
        "#if MAINSTACKADDR_AVAILABLE && !defined(get_main_stack)" => <<~SUBST
          /* -- Start of tebako patch -- */
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
                      if (n == 3 && strcmp(s, "[stack]") == 0)
                          break;
                      prevend = end;
                  }
                  n = 0;
              }
              fclose(f);
              if (n == 0)
                  return -1;
              limit = 100 << 20; /* 100MB stack limit */
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
          /* -- End of tebako patch -- */
        SUBST
      }.freeze

      MAIN_C_PATCH = {
        "int\nmain(int argc, char **argv)" =>
        "#include <tebako/tebako-main.h>\n\nint\nmain(int argc, char **argv)",

        "    ruby_sysinit(&argc, &argv);" => <<~SUBST
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
              /* tebako patch */ if (!within_tebako_memfs(path)) plain = 1; else magical = 1;
      SUBST

      DIR_C_BASE_PATCH = {
        "#if defined HAVE_GETATTRLIST && defined ATTR_DIR_ENTRYCOUNT" => DIR_C_BASE_PATCH_ONE,
        "#if USE_NAME_ON_FS == USE_NAME_ON_FS_REAL_BASENAME\n	    plain = 1;" => DIR_C_BASE_PATCH_TWO,
        "if (is_case_sensitive(dirp, path) == 0)" =>
          "if (is_case_sensitive(dirp, path) == 0 /* tebako patch */ && !within_tebako_memfs(path))",
        "if ((*cur)->type == ALPHA) {" =>
          "if ((*cur)->type == ALPHA /* tebako patch */ && !within_tebako_memfs(buf)) {",
        "else if (e == EIO) {" =>
          "else if (e == EIO /* tebako patch */ && !within_tebako_memfs(path)) {"
      }.freeze

      COMMON_MK_PATCH = {
        "ext/extinit.c: $(srcdir)/template/extinit.c.tmpl $(PREP)" =>
          "ext/extinit.c: $(srcdir)/template/extinit.c.tmpl $(PREP) $(EXTS_MK)"
      }.freeze

      C_FILES_TO_PATCH = [
        ["file.c", "/* define system APIs */"],
        ["io.c", "/* define system APIs */"],
        ["util.c", "#ifndef S_ISDIR"],
        ["dln.c", "static const char funcname_prefix[sizeof(FUNCNAME_PREFIX) - 1] = FUNCNAME_PREFIX;"]
      ].freeze

      TEMPLATE_MAKEFILE_IN_BASE_PATTERN_PRE_3_1 =
        "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(XLDFLAGS) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) $(LIBS) $(EXTLIBS) $(OUTFLAG)$@"

      TEMPLATE_MAKEFILE_IN_BASE_PATCH_PRE_3_1 =
        "# -- Start of tebako patch --\n" \
        "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(XLDFLAGS) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG_STATIC) $(OUTFLAG)$@\n" \
        "# -- End of tebako patch --"

      TEMPLATE_MAKEFILE_IN_BASE_PATTERN =
        "\t\t$(Q) $(PURIFY) $(CC) $(EXE_LDFLAGS) $(XLDFLAGS) $(MAINOBJ) $(EXTOBJS) " \
        "$(LIBRUBYARG) $(MAINLIBS) $(LIBS) $(EXTLIBS) $(OUTFLAG)$@"

      TEMPLATE_MAKEFILE_IN_BASE_PATCH =
        "# -- Start of tebako patch --\n" \
        "\t\t$(Q) $(PURIFY) $(CC) $(EXE_LDFLAGS) $(XLDFLAGS) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG_STATIC) $(OUTFLAG)$@\n" \
        "# -- End of tebako patch --"

      C_FILE_SUBST = <<~SUBST
        /* -- Start of tebako patch -- */
        #ifndef NO_TEBAKO_INCLUDES
        #include <tebako/tebako-config.h>
        #include <tebako/tebako-defines.h>
        #include <tebako/tebako-io-rb-w32.h>
        #include <tebako/tebako-io.h>
        #endif
        /* -- End of tebako patch -- */
      SUBST

      GEM_PRELUDE_RB_PATCH = {
        "if defined?(DidYouMean)" => <<~SUBST
          if defined?(DidYouMean)

          # -- Start of tebako patch --
          begin
            require 'tebako-runtime'
          rescue LoadError
            warn "'tebako-runtime' was not loaded."
          end
          # -- End of tebako patch --
        SUBST
      }.freeze

      MSYS_PATCHES = {
        "ruby.c" => {
          "#define RUBY_RELATIVE(path, len) rb_str_buf_cat(BASEPATH(), (path), (len))" =>
          "#define RUBY_RELATIVE(path, len) rubylib_path_new((path), (len))  /* tebako patched */",
          "#define PREFIX_PATH() sopath" =>
            "#define PREFIX_PATH() rubylib_path_new(tebako_mount_point(), " \
            "strlen(tebako_mount_point())) /* tebako patched */",
          '#include "mjit.h"' =>
            "#include \"mjit.h\"\n" \
            "/* -- Start of tebako patch -- */\n" \
            "#include <tebako/tebako-main.h>\n" \
            "/* -- End of tebako patch -- */"
        },
        "/win32/win32.c" => {
          "#undef __STRICT_ANSI__" =>
          "#undef __STRICT_ANSI__\n" \
          "/* -- Start of tebako patch -- */\n" \
          "#define NO_TEBAKO_INCLUDES\n" \
          "/* -- End of tebako patch -- */"
        },
        "win32/dir.h" => {
          "#define opendir(s)   rb_w32_opendir((s))" => "#{C_FILE_SUBST}\n#define opendir(s)   rb_w32_opendir((s))"
        },
        "file.c" => {
          "    wpath = mbstr_to_wstr(CP_UTF8, path, -1, &len);" =>
          "/* -- Start of tebako patch -- */\n" \
          "if (tebako_file_load_ok(path)) return 1;\n" \
          "/* -- End of tebako patch -- */\n" \
          "wpath = mbstr_to_wstr(CP_UTF8, path, -1, &len);",
          '#include "win32/file.h' =>
          "#include \"win32/file.h\"\n" \
          "/* -- Start of tebako patch -- */\n" \
          "#include <tebako/tebako-main.h>\n" \
          "/* -- End of tebako patch -- */"
        }

      }.freeze

      LINUX_PATCHES = {
        "ext/extmk.rb" => {
          "mf.macro \"EXTLIBS\", $extlibs" => "#  mf.macro \"EXTLIBS\", $extlibs   tebako patched"
        }
      }.freeze

      # rubocop:disable Style/WordArray

      # NOTE: folly provides build-in implementation of jemalloc

      DARWIN_BREW_LIBS = [
        ["zlib", "z"],            ["gdbm", "gdbm"],           ["readline", "readline"], ["libffi", "ffi"],
        ["ncurses", "ncurses"],   ["fmt", "fmt"],             ["lz4", "lz4"],           ["xz", "lzma"],
        ["libyaml", "yaml"],      ["boost", "boost_chrono"],
        ["double-conversion", "double-conversion"]
      ].freeze

      DARWIN_BREW_LIBS_PRE_31 = [["openssl@1.1", "ssl"], ["openssl@1.1", "crypto"]].freeze

      DARWIN_BREW_LIBS_31 = [["libyaml", "yaml"], ["openssl@3", "ssl"], ["openssl@3", "crypto"]].freeze

      DARWIN_DEP_LIBS = ["glog", "gflags", "brotlienc", "brotlidec", "brotlicommon"].freeze
      # rubocop:enable Style/WordArray
    end
    # rubocop:enable Metrics/ModuleLength
  end
end
