# frozen_string_literal: true

# Copyright (c) 2021-2024 [Ribose Inc](https://www.ribose.com).
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

require_relative "patch_literals"
require_relative "patch_helpers"
require_relative "patch_buildsystem"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching definitions (pass1)
    module Pass1
      # [TODO] looks like it does not exist in 3.1.4
      # May be obsolete
      TOOL_RBINSTALL_RB_PATCH = {
        "    next if files.empty?" => "# tebako patched    next if files.empty?"
      }.freeze

      RUBYGEM_OPENSSL_RB_SUBST = <<~SUBST
        # Start of tebako patch
        require "openssl"
        # End of tebako patch
        autoload :OpenSSL, "openssl"
      SUBST

      RUBYGEM_OPENSSL_RB_PATCH = {
        'autoload :OpenSSL, "openssl"' => RUBYGEM_OPENSSL_RB_SUBST
      }.freeze

      EXT_SETUP_PATCH = {
        "#option nodynamic" => "option nodynamic"
      }.freeze

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

      DARWIN_CONFIGURE_PATCH = {
        "elif test \"x$EXTSTATIC\" = x" => "elif true"
      }.freeze

      OPENSSL_EXTCONF_RB_SUBST = <<~SUBST
        # Start of tebako patch
        $defs.push("-DRUBY_EXPORT=1")
        # End of tebako patch

        Logging::message "=== Checking done. ===\\n"
      SUBST

      OPENSSL_EXTCONF_RB_PATCH = {
        "Logging::message \"=== Checking done. ===\\n\"" => OPENSSL_EXTCONF_RB_SUBST
      }.freeze

      class << self
        def get_base_patch_map(mount_point)
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
            # This is something that I cannnot explain
            # (this patch does not seem related to static compilation)
            "ext/bigdecimal/bigdecimal.h" => EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH,

            # ....................................................
            # Allow only packaged gems (from within memfs)
            "lib/rubygems/path_support.rb" => rubygems_path_support_patch(mount_point),

            # ....................................................
            # Disable dynamic extensions
            "ext/Setup" => EXT_SETUP_PATCH
          }
        end

        def get_patch_map(ostype, mount_point, ruby_ver)
          patch_map = get_base_patch_map(mount_point)

          # ....................................................
          patch_map.store("configure", DARWIN_CONFIGURE_PATCH) if ostype =~ /darwin/

          # ....................................................
          # autoload :OpenSSL, "openssl"
          # fails to deal with a default gem from statically linked extension
          patch_map.store("lib/rubygems/openssl.rb", RUBYGEM_OPENSSL_RB_PATCH) if ruby_ver.ruby3x?

          if ostype =~ /msys/
            # ....................................................
            # Generate export definitions; use WinMain to build rubyw.exe
            patch_map.store("cygwin/GNUmakefile.in", get_gnumakefile_in_patch_p1(ruby_ver))
            # ....................................................
            # RUBY_EXPORT=1 (shall be set for static builds but is missing in openssl extension)
            patch_map.store("ext/openssl/extconf.rb", OPENSSL_EXTCONF_RB_PATCH)
          end

          patch_map
        end

        private

        include Tebako::Packager::PatchLiterals
        include Tebako::Packager::PatchBuildsystem

        def get_gnumakefile_in_patch_p1(ruby_ver) # rubocop:disable Metrics/MethodLength
          objext = ruby_ver.ruby32? ? "$(OBJEXT)" : "@OBJEXT@"
          {
            "  DLLWRAP += -mno-cygwin" =>
              "# tebako patched  DLLWRAP += -mno-cygwin",

            "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext}" =>
              "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext} $(WINMAINOBJ)  # tebako patched",

            "$(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@" =>
              "$(WINMAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@  # tebako patched",

            "--output-exp=$(RUBY_EXP) \\" =>
             "--output-exp=$(RUBY_EXP) --output-lib=$(LIBRUBY) --output-def=tebako.def \\",

            "--export-all $(LIBRUBY_A) $(LIBS) -o $(PROGRAM)" =>
              "--export-all $(LIBRUBY_A) $(LIBS) -o program-stub.exe   # tebako patched",

            "@rm -f $(PROGRAM)" =>
              "@rm -f program-stub.exe   # tebako patched",

            "	$(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)" =>
             "# tebako patched  $(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)",

            "RUBYDEF = $(DLL_BASE_NAME).def" => GNUMAKEFILE_IN_WINMAIN_SUBST
          }
        end

        def rubygems_path_support_patch_one(mount_point)
          <<~SUBST
              @home = env["GEM_HOME"] || Gem.default_dir
            # -- Start of tebako patch --
                unless env["TEBAKO_PASS_THROUGH"]
                  @home = Gem.default_dir unless @home.index("#{mount_point}") == 0
                end
            # -- End of tebako patch --

          SUBST
        end

        def rubygems_path_support_patch_two(mount_point)
          <<~SUBST

              @path = split_gem_path env["GEM_PATH"], @home
            # -- Start of tebako patch --
                unless env["TEBAKO_PASS_THROUGH"]
                  @path.keep_if do |xpath|
                    xpath.index("#{mount_point}") == 0
                  end
                end
            # -- End of tebako patch --

          SUBST
        end

        def rubygems_path_support_patch(mount_point)
          {
            '  @home = env["GEM_HOME"] || Gem.default_dir' => rubygems_path_support_patch_one(mount_point),
            '  @path = split_gem_path env["GEM_PATH"], @home' => rubygems_path_support_patch_two(mount_point)
          }
        end
      end
    end
  end
end
