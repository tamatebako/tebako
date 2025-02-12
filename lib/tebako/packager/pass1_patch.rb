# frozen_string_literal: true

# Copyright (c) 2021-2025 [Ribose Inc](https://www.ribose.com).
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

require_relative "patch_helpers"
require_relative "patch_buildsystem"

require_relative "patch"
require_relative "rubygems_patch"
# Tebako - an executable packager
module Tebako
  # Packager module
  module Packager
    class << self
      def crt_pass1_patch(ostype, mount_point, ruby_ver)
        scmb = ScenarioManagerBase.new(ostype)
        if scmb.macos?
          Pass1DarwinPatch.new(mount_point, ruby_ver)
        elsif scmb.msys?
          Pass1MSysPatch.new(mount_point, ruby_ver)
        else
          Pass1Patch.new(mount_point, ruby_ver)
        end
      end
    end
    # Ruby patching definitions (pass1 - common)
    class Pass1Patch < RubygemsPatch
      # [TODO] looks like it does not exist in 3.1.4
      # May be obsolete
      TOOL_RBINSTALL_RB_PATCH = {
        "    next if files.empty?" => "# tebako patched    next if files.empty?"
      }.freeze

      EXT_SETUP_PATCH = {
        "#option nodynamic" => "option nodynamic"
      }.freeze

      # ....................................................
      # This is something that I cannnot explain
      # (this patch does not seem related to static compilation)

      EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH = {
        "#include \"ruby/ruby.h\"" => <<~SUBST
          #include "ruby/ruby.h"

          /* -- Start of tebako patch -- */
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

          #ifndef HAVE_RB_COMPLEX_REAL
          #define HAVE_RB_COMPLEX_REAL
          #endif

          #ifndef HAVE_RB_COMPLEX_IMAG
          #define HAVE_RB_COMPLEX_IMAG
          #endif
          /* -- End of tebako patch -- */

        SUBST
      }.freeze

      # The logic for statically linked extensions is broken entirely in the latest Ruby versions.
      # The code below looks reasonble - we do not set -bundle_loader when building with -with-static-ext option
      # However Gems bundled with Ruby just ignore with-static-ext option and build extensions as shared libraries
      # So  the -bundler_loader option is required for them to link.
      # It is strange that it is disabled in any case because this option does not create any issues for static
      # libraries.
      # ---------------------------------------------------
      # elif test "x$EXTSTATIC" = x
      # then :
      #
      #                            # When building exts as bundles, a mach-o bundle needs to know its loader
      #                            # program to bind symbols from the ruby executable
      #                            EXTDLDFLAGS="-bundle_loader '\$(BUILTRUBY)'"

      OPENSSL_EXTCONF_RB_SUBST = <<~SUBST
        # Start of tebako patch
        $defs.push("-DRUBY_EXPORT=1")
        # End of tebako patch

        Logging::message "=== Checking done. ===\\n"
      SUBST

      OPENSSL_EXTCONF_RB_PATCH = {
        "Logging::message \"=== Checking done. ===\\n\"" => OPENSSL_EXTCONF_RB_SUBST
      }.freeze

      def initialize(mount_point, ruby_ver)
        super(mount_point)
        @ruby_ver = ruby_ver
      end

      def base_patch_map
        {
          # ....................................................
          # It won't install gems with no files defined in spec
          # However if
          #   -- we are installing a default gem from extension
          #   -- extension is build statically
          #  there may be no files install in addition to spec
          # Example: io/wait extension (and others)
          # [TODO]  Check if it is still required
          # No match and patching on Ruby 3.1.4 but works wo issues
          "tool/rbinstall.rb" => TOOL_RBINSTALL_RB_PATCH,

          # ....................................................
          # Allow only packaged gems (from within memfs)
          "lib/rubygems/path_support.rb" => rubygems_path_support_patch(@mount_point),

          # ....................................................
          # Disable dynamic extensions
          "ext/Setup" => EXT_SETUP_PATCH
        }
      end

      def patch_map
        pm = base_patch_map
        pm.merge!(super)

        # ....................................................
        pm.store("ext/bigdecimal/bigdecimal.h", EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH) unless @ruby_ver.ruby34?

        # ....................................................
        # autoload :OpenSSL, "openssl"
        # fails to deal with a default gem from statically linked extension
        pm.store("lib/rubygems/openssl.rb", RUBYGEMS_OPENSSL_RB_PATCH) if @ruby_ver.ruby3x?

        pm.freeze
      end
    end

    # Ruby patching definitions (pass1 - darwin)
    class Pass1DarwinPatch < Pass1Patch
      DARWIN_CONFIGURE_PATCH = {
        "elif test \"x$EXTSTATIC\" = x" => "elif true"
      }.freeze

      def patch_map
        pm = { "configure" => DARWIN_CONFIGURE_PATCH }
        pm.merge!(super)
        pm.freeze
      end
    end

    # Ruby patching definitions (pass1 - Windows)
    class Pass1MSysPatch < Pass1Patch
      INCLUDE_RUBY_ONIGMO_H_PATCH = {
        "#  define ONIG_EXTERN   RUBY_EXTERN" => "#  define ONIG_EXTERN  extern"
      }.freeze

      WIN32_WINMAIN_C_PATCH = {
        "WinMain(HINSTANCE current, HINSTANCE prev, LPSTR cmdline, int showcmd)" =>
          "wWinMain(HINSTANCE current, HINSTANCE prev, LPWSTR cmdline, int showcmd) /* tebako patched */"
      }.freeze

      def patch_map
        pm = msys_patches
        pm.merge!(super)
        pm.freeze
      end

      private

      def gnumakefile_in_patch_p1 # rubocop:disable Metrics/MethodLength
        objext = @ruby_ver.ruby32? ? "$(OBJEXT)" : "@OBJEXT@"
        {
          "$(Q) $(DLLWRAP) \\" => GNUMAKEFILE_IN_DLLTOOL_SUBST,

          "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext}" =>
            "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext} $(WINMAINOBJ)  # tebako patched",

          "$(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@" =>
            "$(WINMAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@  # tebako patched",

          "--output-exp=$(RUBY_EXP) \\" => "# tebako patched --output-exp=$(RUBY_EXP) \\",

          "--export-all $(LIBRUBY_A) $(LIBS) -o $(PROGRAM)" =>
            "# tebako patched --export-all $(LIBRUBY_A) $(LIBS) -o $(PROGRAM)",

          "@rm -f $(PROGRAM)" => "# tebako patched @rm -f $(PROGRAM)",

          "	$(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)" =>
           "# tebako patched  $(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)",

          "RUBYDEF = $(DLL_BASE_NAME).def" => GNUMAKEFILE_IN_WINMAIN_SUBST
        }
      end

      def msys_base_patches
        {
          # ....................................................
          # Generate export definitions; use WinMain to build rubyw.exe
          "cygwin/GNUmakefile.in" => gnumakefile_in_patch_p1,
          # ....................................................
          # RUBY_EXPORT=1 (shall be set for static builds but is missing in openssl extension)
          "ext/openssl/extconf.rb" => OPENSSL_EXTCONF_RB_PATCH
        }
      end

      def msys_patches
        pm = msys_base_patches

        if @ruby_ver.ruby3x7?
          # ....................................................
          # RUBY_EXTERN shall be extern for static build but is set to __declspec(dllimport) for encodin libarary
          pm.store("include/ruby/onigmo.h", INCLUDE_RUBY_ONIGMO_H_PATCH)
          pm.store("win32/winmain.c", WIN32_WINMAIN_C_PATCH)
        end
        pm
      end
    end
  end
end
