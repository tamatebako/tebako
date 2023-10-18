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

require_relative "patch_literals"
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
          patch_map.store("common.mk", COMMON_MK_PATCH) if PatchHelpers.ruby3x?(ruby_ver)

          ostype =~ /msys/ ? patch_map.merge!(MSYS_PATCHES) : patch_map
          ostype =~ /linux/ ? patch_map.merge!(LINUX_PATCHES) : patch_map
        end

        private

        include Tebako::Packager::PatchLiterals

        def get_dir_c_patch(ostype)
          dir_c_patch = patch_c_file(ostype =~ /msys/ ? "/* define system APIs */" : "#ifdef HAVE_GETATTRLIST")
          dir_c_patch.merge!(DIR_C_BASE_PATCH)
        end

        def get_patch_map_base(ostype, deps_lib_dir, ruby_ver)
          {
            "template/Makefile.in" => template_makefile_in_patch(ostype, deps_lib_dir, ruby_ver),
            "main.c" => MAIN_C_PATCH,
            "tool/mkconfig.rb" => TOOL_MKCONFIG_RB_PATCH,
            "gem_prelude.rb" => GEM_PRELUDE_RB_PATCH,
            "dir.c" => get_dir_c_patch(ostype)
          }
        end

        def process_brew_libs!(libs, brew_libs)
          brew_libs.each { |lib| libs << "#{PatchHelpers.get_prefix_macos(lib[0]).chop}/lib/lib#{lib[1]}.a " }
        end

        def darwin_libs(deps_lib_dir, ruby_ver)
          libs = String.new
          process_brew_libs!(libs, DARWIN_BREW_LIBS)
          process_brew_libs!(libs, DARWIN_BREW_LIBS_32) if PatchHelpers.ruby32?(ruby_ver)
          DARWIN_DEP_LIBS.each { |lib| libs << "#{deps_lib_dir}/lib#{lib}.a " }
          <<~SUBST
            -ltebako-fs -ldwarfs-wr -ldwarfs -lfolly -lfsst -lmetadata_thrift -lthrift_light -lxxhash \
            -lzstd #{libs} -ljemalloc -lc++
          SUBST
        end

        # .....................................................
        #  Notes re linux libraries
        #   1) This order is important: -lgcc_eh -l:libunwind.a -l:liblzma.a  lzma is used to process debug sections.
        #      gcc_eh shall be linked before unwind to avoid duplicate symbols.
        #   2) -lgcc_eh assumes -static-libgcc (applied in CMakeLists.ext, RUBY_C_FLAGS)
        #   3) -static-libstdc++ did not work, not sure why  [TODO ?]
        #   4) When clang is used linker links libraries specified in exensions in such way that they are linked shared
        #      (libz, libffi, libreadline, libncurses, libtinfo, ... )
        #      Using stuff like -l:libz.a  does not help; there is a reference to libz.so anyway.
        #      This is fixed by ext/extmk.rb patch [TODO ?]
        # .....................................................

        def linux_gnu_libs(ruby_ver)
          <<~SUBST
            -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -Wl,--push-state,--whole-archive -l:libdwarfs_compression.a -Wl,--pop-state -l:libfolly.a -l:libfsst.a       \
            -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a -l:libarchive.a -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a     \
            -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a -l:libzstd.a -l:libbrotlienc.a -l:libbrotlidec.a -l:libbrotlicommon.a -l:libgdbm.a  \
            -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libcrypt.a -l:libanl.a #{PatchHelpers.yaml_reference(ruby_ver)}                  \
            -l:libboost_system.a -l:libboost_chrono.a  -l:libutil.a -l:libstdc++.a -lgcc_eh -l:libunwind.a -l:liblzma.a -l:librt.a -ldl -lpthread
          SUBST
        end

        def linux_musl_libs(ruby_ver)
          <<~SUBST
            -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -Wl,--push-state,--whole-archive -l:libdwarfs_compression.a -Wl,--pop-state -l:libfolly.a -l:libfsst.a         \
            -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a        \
            -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a -l:libzstd.a -l:libbrotlienc.a -l:libbrotlidec.a -l:libbrotlicommon.a -l:libgdbm.a -l:libreadline.a  \
            -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libcrypt.a  #{PatchHelpers.yaml_reference(ruby_ver)} -l:libboost_system.a -l:libboost_chrono.a -l:librt.a         \
            -l:libstdc++.a -lgcc_eh -l:libunwind.a -l:liblzma.a -ldl -lpthread
          SUBST
        end

        def msys_libs(ruby_ver)
          <<~SUBST
            -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \
            -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \
            -l:libzstd.a -l:libffi.a -l:libgdbm.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:liblzma.a -l:libiberty.a \
            #{PatchHelpers.yaml_reference(ruby_ver)} -l:libstdc++.a -l:libdl.a -lole32 -loleaut32 -luuid
          SUBST
        end

        # rubocop:disable Metrics/MethodLength
        def mlibs(ostype, deps_lib_dir, ruby_ver)
          case ostype
          when /linux-gnu/
            linux_gnu_libs(ruby_ver)
          when /linux-musl/
            linux_musl_libs(ruby_ver)
          when /darwin/
            darwin_libs(deps_lib_dir, ruby_ver)
          when /msys/
            msys_libs(ruby_ver)
          else
            raise Tebako::Error, "Unknown ostype #{ostype}"
          end
        end
        # rubocop:enable Metrics/MethodLength

        def mlibs_subst(ostype, deps_lib_dir, ruby_ver)
          yjit_libs = PatchHelpers.ruby32?(ruby_ver) ? "$(YJIT_LIBS) " : ""
          {
            "MAINLIBS = #{yjit_libs}@MAINLIBS@" =>
              "# -- Start of tebako patch -- \n" \
              "MAINLIBS = #{yjit_libs}#{mlibs(ostype, deps_lib_dir, ruby_ver)}" \
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
            { TEMPLATE_MAKEFILE_IN_BASE_PATTERN_TWO => TEMPLATE_MAKEFILE_IN_BASE_PATCH_TWO }
          else
            { TEMPLATE_MAKEFILE_IN_BASE_PATTERN_TWO_PRE_3_1 => TEMPLATE_MAKEFILE_IN_BASE_PATCH_TWO_PRE_3_1 }
          end
        end
      end
    end
  end
end
