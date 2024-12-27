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
require_relative "patch_main"
require_relative "patch_libraries"
require_relative "patch_helpers"
require_relative "patch_buildsystem"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching definitions (pass2)
    module Pass2
      class << self
        def get_patch_map(ostype, deps_lib_dir, ruby_ver)
          patch_map = get_patch_map_base(ostype, deps_lib_dir, ruby_ver)
          patch_map.store("thread_pthread.c", LINUX_MUSL_THREAD_PTHREAD_PATCH) if ostype =~ /linux-musl/
          if PatchHelpers.msys?(ostype)
            patch_map.merge!(get_msys_patches(ruby_ver))
          elsif ruby_ver.ruby3x?
            patch_map.store("common.mk", COMMON_MK_PATCH)
          end
          extend_patch_map_r33(patch_map, ostype, deps_lib_dir, ruby_ver)
          patch_map.store("prism_compile.c", PRISM_PATCHES) if ruby_ver.ruby34?
          patch_map
        end

        private

        include Tebako::Packager::PatchBuildsystem
        include Tebako::Packager::PatchLiterals
        def extend_patch_map_r33(patch_map, ostype, deps_lib_dir, ruby_ver)
          if ruby_ver.ruby33? || PatchHelpers.msys?(ostype)
            patch_map.store("config.status",
                            get_config_status_patch(ostype, deps_lib_dir, ruby_ver))
          end
          patch_map
        end

        def get_dir_c_patch(ostype)
          pattern = PatchHelpers.msys?(ostype) ? "/* define system APIs */" : "#ifdef HAVE_GETATTRLIST"
          dir_c_patch = PatchHelpers.patch_c_file_pre(pattern)
          dir_c_patch.merge!(DIR_C_BASE_PATCH)
          dir_c_patch
        end

        def get_dln_c_patch(ostype, ruby_ver)
          pattern = "#ifndef dln_loaderror"
          # Not using substitutions of dlxxx functions on Windows
          dln_c_patch = {
            pattern => "#{PatchHelpers.msys?(ostype) ? C_FILE_SUBST_LESS : C_FILE_SUBST}\n#{pattern}\n"
          }

          if PatchHelpers.msys?(ostype)
            patch = ruby_ver.ruby32? ? DLN_C_MSYS_PATCH : DLN_C_MSYS_PATCH_PRE32
            dln_c_patch.merge!(patch)
          end

          dln_c_patch
        end

        def get_io_c_msys_patch(ruby_ver)
          io_c_msys_patch = ruby_ver.ruby32? ? IO_C_MSYS_PATCH : IO_C_MSYS_PATCH_PRE_32
          io_c_msys_patch.merge(IO_C_MSYS_BASE_PATCH)
        end

        def get_io_c_patch(ostype, ruby_ver)
          io_c_patch = PatchHelpers.patch_c_file_pre("/* define system APIs */")
          io_c_patch.merge!(get_io_c_msys_patch(ruby_ver)) if PatchHelpers.msys?(ostype)
          io_c_patch
        end

        def get_util_c_patch(ruby_ver)
          if ruby_ver.ruby31?
            PatchHelpers.patch_c_file_post("#endif /* !HAVE_GNU_QSORT_R */")
          else
            PatchHelpers.patch_c_file_pre("#ifndef S_ISDIR")
          end
        end

        def get_tool_mkconfig_rb_patch(ostype)
          subst = PatchHelpers.msys?(ostype) ? TOOL_MKCONFIG_RB_SUBST_MSYS : TOOL_MKCONFIG_RB_SUBST
          {
            "    if fast[name]" => subst
          }
        end

        def get_msys_patches(ruby_ver)
          {
            "cygwin/GNUmakefile.in" => get_gnumakefile_in_patch_p2(ruby_ver),
            "ruby.c" => RUBY_C_MSYS_PATCHES,
            "win32/file.c" => WIN32_FILE_C_MSYS_PATCHES,
            "win32/win32.c" => WIN32_WIN32_C_MSYS_PATCHES
          }
        end

        def get_patch_map_base(ostype, deps_lib_dir, ruby_ver)
          {
            "template/Makefile.in" => template_makefile_in_patch(ostype, deps_lib_dir, ruby_ver),
            "tool/mkconfig.rb" => get_tool_mkconfig_rb_patch(ostype),
            "dir.c" => get_dir_c_patch(ostype),            "dln.c" => get_dln_c_patch(ostype, ruby_ver),
            "io.c" => get_io_c_patch(ostype, ruby_ver),    "main.c" => PatchMain.get_main_c_patch(ruby_ver),
            "file.c" => PatchHelpers.patch_c_file_pre("/* define system APIs */"),
            "util.c" => get_util_c_patch(ruby_ver)
          }
        end

        def mlibs_subst(ostype, deps_lib_dir, ruby_ver)
          yjit_libs = ruby_ver.ruby32only? ? "$(YJIT_LIBS) " : ""
          {
            "MAINLIBS = #{yjit_libs}@MAINLIBS@" =>
              "# -- Start of tebako patch -- \n" \
              "MAINLIBS = #{yjit_libs}#{PatchLibraries.mlibs(ostype, deps_lib_dir, ruby_ver, true)}\n" \
              "# -- End of tebako patch -- \n"
          }
        end

        def template_makefile_in_patch(ostype, deps_lib_dir, ruby_ver)
          template_makefile_in_patch_two(ostype, ruby_ver).merge(mlibs_subst(ostype, deps_lib_dir, ruby_ver))
        end
      end
    end
  end
end
