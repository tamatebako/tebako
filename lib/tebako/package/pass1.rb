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

# Tebako - an executable packager
module Tebako
  # Ruby patching definitions (pass1)
  module Pass1
    TOOL_RBINSTALL_RB_PATCH = {
      "    next if files.empty?" => "# tebako patched    next if files.empty?"
    }.freeze

    RUBYGEM_OPENSSL_RB_PATCH = {
      'autoload :OpenSSL, "openssl"' => "require \"openssl\"\nautoload :OpenSSL, \"openssl\""
    }.freeze

    EXT_SETUP_PATCH = {
      "#option nodynamic" => "option nodynamic"
    }.freeze

    EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH = {
      "#include <float.h>" => <<~SUBST
        #include <float.h>

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

    CONFIGURE_PATCH = {
      "EXTDLDFLAGS=\"-bundle_loader '\\$(BUILTRUBY)'\"" => ""
    }.freeze

    class << self
      def get_patch_map(ostype, mount_point)
        patch_map = {
          # ....................................................
          # It won't install gems with no files defined in spec
          # However if
          #   -- we are installing a default gem from extension
          #   -- extension is build statically
          #  there may be no files install in addition to spec
          # Example: io/wait extension (and others)
          "tool/rbinstall.rb" => TOOL_RBINSTALL_RB_PATCH,

          # ....................................................
          # autoload :OpenSSL, "openssl"
          # fails to deal with a default gem from statically linked extension
          "lib/rubygems/openssl.rb" => RUBYGEM_OPENSSL_RB_PATCH,

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
        # ....................................................
        # Fixing (bypassing) configure script bug where a variable is used before initialization
        patch_map.store("configure", CONFIGURE_PATCH) if ostype =~ /darwin/

        patch_map
      end

      private

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
