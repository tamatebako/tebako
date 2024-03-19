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
require_relative "patch_libraries"
require_relative "patch_helpers"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching definitions (pass2)
    module Pass2
      class << self
        def get_patch_map(ostype, deps_lib_dir, ruby_ver)
          patch_map = get_patch_map_base(ostype, deps_lib_dir, ruby_ver)

          C_FILES_TO_PATCH.each { |patch| patch_map.store(patch[0], patch_c_file(patch[1])) }

          patch_map.store("thread_pthread.c", LINUX_MUSL_THREAD_PTHREAD_PATCH) if ostype =~ /linux-musl/

          if ostype =~ /msys/
            patch_map.merge!(get_msys_patches)
          elsif PatchHelpers.ruby3x?(ruby_ver)
            # [TODO] Do we really need it for platforms other then Windows ??
            patch_map.store("common.mk", COMMON_MK_PATCH)
          end

          patch_map
          # ostype =~ /msys/ ? patch_map.merge!(MSYS_PATCHES) : patch_map
          #          patch_map.merge!(LINUX_PATCHES)
        end

        private

        include Tebako::Packager::PatchLiterals

        def get_dir_c_patch(ostype)
          dir_c_patch = patch_c_file(ostype =~ /msys/ ? "/* define system APIs */" : "#ifdef HAVE_GETATTRLIST")
          dir_c_patch.merge!(DIR_C_BASE_PATCH)
        end

        def get_io_c_patch(ostype)
          io_c_patch = patch_c_file("/* define system APIs */")
          io_c_patch.merge!(IO_C_MSYS_PATCH) if ostype =~ /msys/
          io_c_patch
        end

        def get_msys_patches # rubocop:disable Naming/AccessorMethodName
          {
            "cygwin/GNUmakefile.in" => GNUMAKEFILE_IN_PATCH_P2,
            "ruby.c" => RUBY_C_MSYS_PATCHES,
            "win32/file.c" => WIN32_FILE_C_MSYS_PATCHES
          }
        end

        def get_patch_map_base(ostype, deps_lib_dir, ruby_ver)
          {
            "template/Makefile.in" => template_makefile_in_patch(ostype, deps_lib_dir, ruby_ver),
            "main.c" => MAIN_C_PATCH,
            "tool/mkconfig.rb" => ostype =~ /msys/ ? TOOL_MKCONFIG_RB_PATCH_MSYS : TOOL_MKCONFIG_RB_PATCH,
            "gem_prelude.rb" => GEM_PRELUDE_RB_PATCH,
            "dir.c" => get_dir_c_patch(ostype),
            "io.c" => get_io_c_patch(ostype)
          }
        end

        def mlibs_subst(ostype, deps_lib_dir, ruby_ver)
          yjit_libs = PatchHelpers.ruby32only?(ruby_ver) ? "$(YJIT_LIBS) " : ""
          {
            "MAINLIBS = #{yjit_libs}@MAINLIBS@" =>
              "# -- Start of tebako patch -- \n" \
              "MAINLIBS = #{yjit_libs}#{PatchLibraries.mlibs(ostype, deps_lib_dir, ruby_ver)}" \
              "# -- End of tebako patch -- \n"
          }
        end

        def patch_c_file(pattern)
          {
            pattern => "#{C_FILE_SUBST}\n#{pattern}"
          }
        end

        def template_makefile_in_patch(ostype, deps_lib_dir, ruby_ver)
          template_makefile_in_patch_two(ruby_ver).merge(mlibs_subst(ostype, deps_lib_dir, ruby_ver))
        end

        def template_makefile_in_patch_two(ruby_ver)
          if PatchHelpers.ruby31?(ruby_ver)
            { TEMPLATE_MAKEFILE_IN_BASE_PATTERN => TEMPLATE_MAKEFILE_IN_BASE_PATCH }
          else
            { TEMPLATE_MAKEFILE_IN_BASE_PATTERN_PRE_3_1 => TEMPLATE_MAKEFILE_IN_BASE_PATCH_PRE_3_1 }
          end
        end
      end
    end
  end
end
