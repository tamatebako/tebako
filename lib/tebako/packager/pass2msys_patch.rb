# frozen_string_literal: true

# Copyright (c) 2021-2025 [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of the Tebako project.
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

require_relative "pass2_patch"
require_relative "patch_helpers"
require_relative "patch_literals"

# Tebako - an executable packager
module Tebako
  # Packager module
  module Packager
    IO_C_MSYS_BASE_PATCH = {
      "#define open	rb_w32_uopen" => "#define open(p, f, m) tebako_open(3, (p), (f), (m))"
    }.freeze

    IO_C_MSYS_PATCH_PRE_32 = {
      "(rb_w32_io_cancelable_p((fptr)->fd) ? Qnil : rb_io_wait(fptr->self, RB_INT2NUM(RUBY_IO_READABLE), Qnil))" =>
          "((is_tebako_file_descriptor((fptr)->fd) || rb_w32_io_cancelable_p((fptr)->fd)) ? \\\n" \
          "Qnil : rb_io_wait(fptr->self, RB_INT2NUM(RUBY_IO_READABLE), Qnil))"
    }.freeze

    IO_C_MSYS_PATCH = {
      "(rb_w32_io_cancelable_p((fptr)->fd) ? Qnil : rb_io_wait(fptr->self, " \
      "RB_INT2NUM(RUBY_IO_READABLE), RUBY_IO_TIMEOUT_DEFAULT))" =>
          "((is_tebako_file_descriptor((fptr)->fd) || rb_w32_io_cancelable_p((fptr)->fd)) ? \\\n" \
          "Qnil : rb_io_wait(fptr->self, RB_INT2NUM(RUBY_IO_READABLE), RUBY_IO_TIMEOUT_DEFAULT))"
    }.freeze

    RUBY_C_MSYS_PATH_SUBST = <<~SUBST
      /* -- Start of tebako patch -- */
              VALUE path = within_tebako_memfs(paths) ?
                              rb_str_new_cstr(paths) :
                              RUBY_RELATIVE(paths, len);
              /* -- End of tebako patch -- */
    SUBST

    RUBY_C_MSYS_PATCHES = {
      "#ifndef MAXPATHLEN" => "#{PatchLiterals::C_FILE_SUBST_LESS}\n#ifndef MAXPATHLEN",
      "VALUE path = RUBY_RELATIVE(paths, len);" => RUBY_C_MSYS_PATH_SUBST
    }.freeze

    WIN32_FILE_C_MSYS_SUBST = <<~SUBST
      /* -- Start of tebako patch -- */
        if (tebako_file_load_ok(path)) return ret;
          /* -- End of tebako patch -- */
      wpath = mbstr_to_wstr(CP_UTF8, path, -1, &len);
    SUBST

    WIN32_FILE_C_MSYS_PATCHES = {
      "#ifndef INVALID_FILE_ATTRIBUTES" => "#{PatchLiterals::C_FILE_SUBST_LESS}\n#ifndef INVALID_FILE_ATTRIBUTES",
      "wpath = mbstr_to_wstr(CP_UTF8, path, -1, &len);" => WIN32_FILE_C_MSYS_SUBST
    }.freeze

    WIN32_WIN32_C_MSYS_SUBST = <<~SUBST
      /* -- Start of tebako patch -- */
        if (is_tebako_cwd()) {
          char* tebako_cwd = tebako_getcwd(NULL,0);
          if (tebako_cwd == NULL) {
            errno = ENOMEM;
            return NULL;
          }
          len = strlen(tebako_cwd) + 1;

          if (buffer) {
            if (size < len) {
              free(tebako_cwd);
              errno = ERANGE;
              return NULL;
            }
          }
          else {
            buffer = (*alloc)(len, arg);
            if (!buffer) {
              free(tebako_cwd);
              errno = ENOMEM;
              return NULL;
            }
          }
          translate_char(tebako_cwd, (char)0x5c, '/', CP_UTF8);
          strcpy(buffer, tebako_cwd);
          free(tebako_cwd);
          return buffer;
        }
        /* -- End of tebako patch -- */

        len = GetCurrentDirectoryW(0, NULL);
    SUBST

    WIN32_WIN32_C_MSYS_PATCHES = {
      "#if defined _MSC_VER && _MSC_VER <= 1200" =>
        "#{PatchLiterals::C_FILE_SUBST_LESS}\n#if defined _MSC_VER && _MSC_VER <= 1200",
      "len = GetCurrentDirectoryW(0, NULL);" => WIN32_WIN32_C_MSYS_SUBST
    }.freeze

    # Msys Pass2 patches
    class Pass2MSysPatch < Pass2Patch
      def patch_map
        pm = super
        pm.merge!(msys_patches)
        pm.store("config.status", get_config_status_patch(@ostype, @deps_lib_dir, @ruby_ver))
        pm
      end

      private

      # Other MSYS (GNUMakefile) specific patches
      #  - The same issue with libraries as for Makefile above
      #  - 'Kill' ruby.exp regeneration on pass2
      #     since we want to use output from pass1 for implib generation
      #     [VERY UGLY HACK]
      #  - Introduce LIBRUBY dependency on static extensions
      #    This is an addition to COMMON_MK_PATCH specified above
      def gnumakefile_in_patch_p2 # rubocop:disable Metrics/MethodLength
        objext = @ruby_ver.ruby32? ? "$(OBJEXT)" : "@OBJEXT@"

        {
          "$(Q) $(DLLWRAP) \\" => GNUMAKEFILE_IN_DLLTOOL_SUBST,

          "--output-exp=$(RUBY_EXP) \\" => "# tebako patched --output-exp=$(RUBY_EXP) \\",

          "--export-all $(LIBRUBY_A) $(LIBS) -o $(PROGRAM)" =>
            "# tebako patched --export-all $(LIBRUBY_A) $(LIBS) -o $(PROGRAM)",

          "@rm -f $(PROGRAM)" => "# tebako patched @rm -f $(PROGRAM)",

          "	$(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)" =>
           "# tebako patched  $(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)",

          "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext}" =>
            "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext} $(WINMAINOBJ)  # tebako patched",

          "RUBYDEF = $(DLL_BASE_NAME).def" => GNUMAKEFILE_IN_WINMAIN_SUBST,

          "$(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@" =>
            "$(WINMAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) -o $@  # tebako patched",

          "$(RUBY_EXP): $(LIBRUBY_A)" => "dummy.exp: $(LIBRUBY_A) # tebako patched",

          "$(PROGRAM): $(RUBY_INSTALL_NAME).res.#{objext}" =>
            "$(PROGRAM): $(RUBY_INSTALL_NAME).res.#{objext} $(LIBRUBY_A) # tebako patched\n" \
            "$(LIBRUBY_A): $(LIBRUBY_A_OBJS) $(INITOBJS) # tebako patched\n"
        }
      end

      def io_c_patch
        patch = super
        patch.merge!(@ruby_ver.ruby32? ? IO_C_MSYS_PATCH : IO_C_MSYS_PATCH_PRE_32)
        patch.merge!(IO_C_MSYS_BASE_PATCH)
        patch
      end

      def msys_patches
        {
          "cygwin/GNUmakefile.in" => gnumakefile_in_patch_p2,
          "ruby.c" => RUBY_C_MSYS_PATCHES,
          "win32/file.c" => WIN32_FILE_C_MSYS_PATCHES,
          "win32/win32.c" => WIN32_WIN32_C_MSYS_PATCHES
        }
      end
    end
  end
end
