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

require_relative "patch_literals"
require_relative "patch_main"
require_relative "patch_libraries"
require_relative "patch_helpers"
require_relative "patch_buildsystem"

# Tebako - an executable packager
module Tebako
  # Packager module
  module Packager
    # Ruby patching definitions (pass2)
    class Pass2Patch < Patch
      def initialize(ostype, deps_lib_dir, ruby_ver)
        super()
        @ostype = ostype
        @scmb = ScenarioManagerBase.new(@ostype)
        @deps_lib_dir = deps_lib_dir
        @ruby_ver = ruby_ver
      end

      def patch_map
        pm = patch_map_base
        pm.store("thread_pthread.c", LINUX_MUSL_THREAD_PTHREAD_PATCH) if @scmb.musl?
        pm.store("prism_compile.c", PRISM_PATCHES) if @ruby_ver.ruby34?
        pm
      end

      protected

      def io_c_patch
        PatchHelpers.patch_c_file_pre("/* define system APIs */")
      end

      private

      include Tebako::Packager::PatchBuildsystem
      include Tebako::Packager::PatchLiterals

      def dir_c_patch
        pattern = @scmb.msys? ? "/* define system APIs */" : "#ifdef HAVE_GETATTRLIST"
        patch = PatchHelpers.patch_c_file_pre(pattern)
        patch.merge!(DIR_C_BASE_PATCH)
        patch
      end

      def dln_c_patch
        pattern = "#ifndef dln_loaderror"
        # Not using substitutions of dlxxx functions on Windows
        patch = {
          pattern => "#{@scmb.msys? ? C_FILE_SUBST_LESS : C_FILE_SUBST}\n#{pattern}\n"
        }

        if @scmb.msys?
          patch.merge!(@ruby_ver.ruby32? ? DLN_C_MSYS_PATCH : DLN_C_MSYS_PATCH_PRE32)
        end

        patch
      end

      def util_c_patch
        if @ruby_ver.ruby31?
          PatchHelpers.patch_c_file_post("#endif /* !HAVE_GNU_QSORT_R */")
        else
          PatchHelpers.patch_c_file_pre("#ifndef S_ISDIR")
        end
      end

      def tool_mkconfig_rb_patch
        subst = @scmb.msys? ? TOOL_MKCONFIG_RB_SUBST_MSYS : TOOL_MKCONFIG_RB_SUBST
        {
          "    if fast[name]" => subst
        }
      end

      def patch_map_base
        {
          "template/Makefile.in" => template_makefile_in_patch,
          "tool/mkconfig.rb" => tool_mkconfig_rb_patch,
          "dir.c" => dir_c_patch, "dln.c" => dln_c_patch,
          "io.c" => io_c_patch, "main.c" => PatchMain.get_main_c_patch(@ruby_ver),
          "file.c" => PatchHelpers.patch_c_file_pre("/* define system APIs */"),
          "util.c" => util_c_patch
        }
      end

      def mlibs_subst
        yjit_libs = @ruby_ver.ruby32only? ? "$(YJIT_LIBS) " : ""
        {
          "MAINLIBS = #{yjit_libs}@MAINLIBS@" =>
            "# -- Start of tebako patch -- \n" \
            "MAINLIBS = #{yjit_libs}#{PatchLibraries.mlibs(@ostype, @deps_lib_dir, @ruby_ver, true)}\n" \
            "# -- End of tebako patch -- \n"
        }
      end

      def template_makefile_in_patch
        template_makefile_in_patch_two(@ruby_ver).merge(mlibs_subst)
      end
    end

    # Non-msys Pass2 patches
    class Pass2NonMSysPatch < Pass2Patch
      def patch_map
        pm = super
        pm.store("common.mk", COMMON_MK_PATCH) if @ruby_ver.ruby3x?
        pm.store("config.status", get_config_status_patch(@ostype, @deps_lib_dir, @ruby_ver)) if @ruby_ver.ruby33?
        pm
      end

      include Tebako::Packager::PatchLiterals
    end
  end
end
