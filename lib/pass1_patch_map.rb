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

def get_pass1_patch_map(mount_point)
  {
    # ....................................................
    # It won't install gems with no files defined in spec
    # However if
    #   -- we are installing a default gem from extension
    #   -- extension is build statically
    #  there may be no files install in addition to spec
    # Example: io/wait extension (and others)
    'tool/rbinstall.rb' => {
      '    next if files.empty?' => '# tebako patched    next if files.empty?'
    },

    # ....................................................
    # autoload :OpenSSL, "openssl"
    # fails to deal with a default gem from statically linked extension
    'lib/rubygems/openssl.rb' => {
      'autoload :OpenSSL, "openssl"' => "require \"openssl\"\nautoload :OpenSSL, \"openssl\""
    },

    # ....................................................
    # This is something that I cannnot explain
    # (this patch does not seem related to static compilation)
    'ext/bigdecimal/bigdecimal.h' => {
      '#include <float.h>' =>
        <<~SUBST
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
    },

    'lib/rubygems/path_support.rb' => {
      '  @home = env["GEM_HOME"] || Gem.default_dir' =>

        "  @home = env[\"GEM_HOME\"] || Gem.default_dir\n" \
        "# -- Start of tebako patch --\n" \
        "    unless env[\"TEBAKO_PASS_THROUGH\"]\n" \
        "      @home = Gem.default_dir unless @home.index(\"#{mount_point}\") == 0\n" \
        "    end\n" \
        "# -- End of tebako patch --\n\n",

      '  @path = split_gem_path env["GEM_PATH"], @home' =>

          "  @path = split_gem_path env[\"GEM_PATH\"], @home\n" \
          "# -- Start of tebako patch --\n" \
          "    unless env[\"TEBAKO_PASS_THROUGH\"]\n" \
          "      @path.keep_if do |xpath|\n" \
          "        xpath.index(\"#{mount_point}\") == 0\n" \
          "      end\n" \
          "    end\n" \
          "# -- End of tebako patch --\n\n"
    },

    # ....................................................
    # Disable dynamic extensions
    'ext/Setup' => {
      '#option nodynamic' => 'option nodynamic'
    }
  }
end
