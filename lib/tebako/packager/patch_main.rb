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
    # Ruby patching literals (main.c)
    module PatchMain
      MAIN_C_SUBST_RB_MAIN = <<~SUBST
        /* -- Start of tebako patch -- */
        #include <tebako/tebako-main.h>
        /* -- End of tebako patch -- */

        static int
        rb_main(int argc, char **argv)
      SUBST

      MAIN_C_SUBST_MAIN = <<~SUBST
        /* -- Start of tebako patch -- */
        #include <tebako/tebako-main.h>
        /* -- End of tebako patch -- */

        int
        main(int argc, char **argv)
      SUBST

      MAIN_C_SUBST_RB_MAIN_INNER = <<~SUBST
        ruby_init();
        /* -- Start of tebako patch -- */
            if (!tebako_is_running_miniruby())
            {
                VALUE cwd = rb_str_new_cstr(tebako_original_pwd());
                rb_gv_set("$tebako_original_pwd", cwd);
            }
        /* -- End of tebako patch -- */
      SUBST

      MAIN_C_SUBST_MAIN_INNER = <<~SUBST
            ruby_sysinit(&argc, &argv);
        /* -- Start of tebako patch -- */
            if (tebako_main(&argc, &argv) != 0) {
                printf("Tebako intialization failed.");
                return -1;
            }
        /* -- End of tebako patch -- */
      SUBST

      class << self
        def get_main_c_patch(ruby_ver)
          puts get_main_c_main_pattern(ruby_ver)
          {
            get_main_c_main_pattern(ruby_ver) => get_main_c_main_subst(ruby_ver),
            "ruby_init();" => MAIN_C_SUBST_RB_MAIN_INNER,
            "    ruby_sysinit(&argc, &argv);" => MAIN_C_SUBST_MAIN_INNER
          }
        end

        private

        def get_main_c_main_pattern(ruby_ver)
          ruby_ver.ruby32? ? "static int\nrb_main(int argc, char **argv)" : "int\nmain(int argc, char **argv)"
        end

        def get_main_c_main_subst(ruby_ver)
          ruby_ver.ruby32? ? MAIN_C_SUBST_RB_MAIN : MAIN_C_SUBST_MAIN
        end
      end
    end
  end
end
