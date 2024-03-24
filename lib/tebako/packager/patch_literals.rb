# frozen_string_literal: true

# Copyright (c) 2023-2024 [Ribose Inc](https://www.ribose.com).
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
    module PatchLiterals
      TOOL_MKCONFIG_RB_PATCH = {
        "    if fast[name]" => <<~SUBST
          # -- Start of tebako patch --
              v_head_comp = "  CONFIG[\\"prefix\\"] \#{eq} "
              if v_head_comp == v[0...(v_head_comp.length)]
                  v = "\#{v[0...(v_head_comp.length)]}'/__tebako_memfs__'
          "
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

      TOOL_MKCONFIG_RB_PATCH_MSYS = {
        "    if fast[name]" => <<~SUBST
          # -- Start of tebako patch --
              v_head_comp = "  CONFIG[\\"prefix\\"] \#{eq} "
              if v_head_comp == v[0...(v_head_comp.length)]
                v = "\#{v[0...(v_head_comp.length)]}CONFIG[\\"RUBY_EXEC_PREFIX\\"] = 'A:/__tebako_memfs__'
          "
              end
              v_head_comp = "  CONFIG[\\"RUBY_EXEC_PREFIX\\"] \#{eq} "
              if v_head_comp == v[0...(v_head_comp.length)]
                v = "\#{v[0...(v_head_comp.length)]}'A:/__tebako_memfs__'
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
                printf("Tebako intialization failed.");
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

      DLN_C_MSYS_PATCH = {
        "    winfile = rb_w32_mbstr_to_wstr(CP_UTF8, file, -1, NULL);" => <<~SUBST
          /* -- Start of tebako patch -- */
            char *f = NULL;
            winfile = NULL;
            if (file && within_tebako_memfs(file)) {
              f = tebako_dlmap2file(file);
              if (f) {
                winfile = rb_w32_mbstr_to_wstr(CP_UTF8, f, -1, NULL);
                free(f);
              }
              else {
                goto failed;
              }
            }
            else {
              winfile = rb_w32_mbstr_to_wstr(CP_UTF8, file, -1, NULL);
            }
          /* -- End of tebako patch -- */
        SUBST
      }.freeze

      COMMON_MK_PATCH = {
        "ext/extinit.c: $(srcdir)/template/extinit.c.tmpl $(PREP)" =>
          "ext/extinit.c: $(srcdir)/template/extinit.c.tmpl $(PREP) $(EXTS_MK)"
      }.freeze

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

      TEMPLATE_MAKEFILE_IN_BASE_PATCH_MSYS =
        "# -- Start of tebako patch --\n" \
        "\t\t$(Q) $(PURIFY) $(CC) $(EXE_LDFLAGS) $(XLDFLAGS) $(RUBY_EXP) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG_STATIC) $(OUTFLAG)$@\n" \
        "# -- End of tebako patch --"

      C_FILE_SUBST = <<~SUBST
        /* -- Start of tebako patch -- */
        #include <tebako/tebako-config.h>
        #include <tebako/tebako-defines.h>
        #include <tebako/tebako-io.h>
        /* -- End of tebako patch -- */
      SUBST

      C_FILE_SUBST_LESS = <<~SUBST
        /* -- Start of tebako patch -- */
        #include <tebako/tebako-config.h>
        #include <tebako/tebako-io.h>
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

      GNUMAKEFILE_IN_WINMAIN_SUBST = <<~SUBST
        RUBYDEF = $(DLL_BASE_NAME).def

        # Start of tebako patch
        WINMAINOBJ    = win32/winmain.$(OBJEXT)
        $(WINMAINOBJ): win32/winmain.c
        # End of tebako patch
      SUBST

      GNUMAKEFILE_IN_PATCH_P1 = {
        "  DLLWRAP += -mno-cygwin" =>
          "# tebako patched  DLLWRAP += -mno-cygwin",

        "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.@OBJEXT@" =>
          "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.@OBJEXT@ $(WINMAINOBJ)  # tebako patched",

        "$(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@" =>
          "$(WINMAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@  # tebako patched",

        "--output-exp=$(RUBY_EXP) \\" =>
         "--output-exp=$(RUBY_EXP) --output-lib=$(LIBRUBY) \\",

        "	@rm -f $(PROGRAM)" =>
          "# tebako patched  @rm -f $(PROGRAM)",

        "	$(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)" =>
          "# tebako patched  $(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)",

        "RUBYDEF = $(DLL_BASE_NAME).def" => GNUMAKEFILE_IN_WINMAIN_SUBST
      }.freeze

      # For pass 2 we 'kill ruby.exp' regenaration
      GNUMAKEFILE_IN_PATCH_P2 = {
        "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.@OBJEXT@" =>
          "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.@OBJEXT@ $(WINMAINOBJ)  # tebako patched",

        "$(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@" =>
          "$(WINMAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) -o $@  # tebako patched",

        "RUBYDEF = $(DLL_BASE_NAME).def" => GNUMAKEFILE_IN_WINMAIN_SUBST,

        "$(RUBY_EXP): $(LIBRUBY_A)" => "dummy.exp: $(LIBRUBY_A) # tebako patched",

        "$(PROGRAM): $(RUBY_INSTALL_NAME).res.@OBJEXT@" =>
          "$(PROGRAM): $(RUBY_INSTALL_NAME).res.@OBJEXT@ $(LIBRUBY_A) # tebako patched\n" \
          "$(LIBRUBY_A): $(LIBRUBY_A_OBJS) $(INITOBJS) # tebako patched\n"
      }.freeze

      IO_C_SUBST = <<~SUBST
        /* -- Start of tebako patch -- */
            if (is_tebako_file_descriptor(fd)) return;
            /* -- End of tebako patch -- */
            flags = fcntl(fd, F_GETFD); /* should not fail except EBADF. */
      SUBST

      IO_C_MSYS_PATCH = {
        "#define open	rb_w32_uopen" => "#define open(p, f, m) tebako_open(3, (p), (f), (m))",
        "(rb_w32_io_cancelable_p((fptr)->fd) ? Qnil : rb_io_wait(fptr->self, RB_INT2NUM(RUBY_IO_READABLE), Qnil))" =>
            "((is_tebako_file_descriptor((fptr)->fd) || rb_w32_io_cancelable_p((fptr)->fd)) ? \\\n" \
            "Qnil : rb_io_wait(fptr->self, RB_INT2NUM(RUBY_IO_READABLE), Qnil))"
      }.freeze

      FILE_C_MSYS_SUBST = <<~SUBST
        /* -- Start of tebako patch -- */
               if (is_tebako_file_descriptor((fptr)->fd)) return ENOTSUP;
               /* -- End of tebako patch -- */
               while ((int)rb_thread_io_blocking_region(rb_thread_flock, op, fptr->fd) < 0) {
      SUBST

      FILE_C_MSYS_PATCH = {
        "while ((int)rb_thread_io_blocking_region(rb_thread_flock, op, fptr->fd) < 0) {" => FILE_C_MSYS_SUBST
      }.freeze

      RUBY_C_MSYS_PATH_SUBST = <<~SUBST
        /* -- Start of tebako patch -- */
                VALUE path = within_tebako_memfs(paths) ?
                                rb_str_new_cstr(paths) :
                                RUBY_RELATIVE(paths, len);
               /* -- End of tebako patch -- */
      SUBST

      RUBY_C_MSYS_PATCHES = {
        "#ifndef MAXPATHLEN" => "#{C_FILE_SUBST_LESS}\n#ifndef MAXPATHLEN",
        "VALUE path = RUBY_RELATIVE(paths, len);" => RUBY_C_MSYS_PATH_SUBST
      }.freeze

      WIN32_FILE_C_MSYS_SUBST = <<~SUBST
        /* -- Start of tebako patch -- */
          if (tebako_file_load_ok(path)) return ret;
            /* -- End of tebako patch -- */
        wpath = mbstr_to_wstr(CP_UTF8, path, -1, &len);
      SUBST

      WIN32_FILE_C_MSYS_PATCHES = {
        "#ifndef INVALID_FILE_ATTRIBUTES" => "#{C_FILE_SUBST_LESS}\n#ifndef INVALID_FILE_ATTRIBUTES",
        "wpath = mbstr_to_wstr(CP_UTF8, path, -1, &len);" => WIN32_FILE_C_MSYS_SUBST
      }.freeze

      LINUX_PATCHES = {
        "ext/extmk.rb" => {
          "mf.macro \"EXTLIBS\", $extlibs" => "#  mf.macro \"EXTLIBS\", $extlibs   tebako patched"
        }
      }.freeze
    end
  end
end
