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

require_relative "patch_helpers"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching definitions (pass2)
    module PatchLibraries
      class << self
        # rubocop:disable Style/WordArray
        DARWIN_BREW_LIBS = [
          ["zlib", "z"],              ["gdbm", "gdbm"],           ["readline", "readline"], ["libffi", "ffi"],
          ["ncurses", "ncurses"],     ["lz4", "lz4"],             ["xz", "lzma"],           ["libyaml", "yaml"]
        ].freeze

        DARWIN_BREW_LIBS_PRE_31 = [["openssl@1.1", "ssl"], ["openssl@1.1", "crypto"]].freeze

        DARWIN_BREW_LIBS_31 = [["openssl@3", "ssl"], ["openssl@3", "crypto"]].freeze

        DARWIN_DEP_LIBS_1 = ["tfs", "tebako_dirent_helper_c"].freeze
        # Referenced by full path from the vcpkg triplet lib dir (see
        # darwin_libraries): Apple ld does not implement the GNU-style
        # -l:<filename> library search, so -l:libX.a refs do not resolve.
        DARWIN_DEP_LIBS_2 = ["dwarfs_reader", "dwarfs_common", "dwarfs_metadata_legacy",
                             "dwarfs_decompressor", "flatbuffers", "zip",
                             "fmt", "xxhash", "zstd",
                             "brotlidec", "brotlienc", "brotlicommon",
                             "bz2", "boost_filesystem", "boost_chrono"].freeze
        # rubocop:enable Style/WordArray

        LIBTEBAKOFS = "-Wl,--push-state,--whole-archive -l:libtebako-fs.a -Wl,--pop-state"

        # --start-group/--end-group around the libtfs + transitive static
        # archives: the dwarfs reader set has circular member-level references
        # (compression registrar/registry) whose resolution order differs
        # between producer toolchains (clang-18-built packages trip GNU ld's
        # single-pass archive scanning where gcc-built ones did not).
        GROUP_BEGIN = "-Wl,--start-group"
        GROUP_END = "-Wl,--end-group"

        # libtfs (libtfs.a + its pure-C dirent helper) and the transitive static
        # set resolved by vcpkg into deps/vcpkg_installed/<triplet>/lib: the
        # dwarfs reader side, flatbuffers, zip and the C++ support libs.
        # Compression codecs register explicitly (compression_registry ctor),
        # so no --whole-archive compression lib is needed anymore.
        COMMON_LINUX_LIBRARIES = [
          LIBTEBAKOFS, "-l:libtfs.a", "-l:libtebako_dirent_helper_c.a",
          "-l:libdwarfs_reader.a", "-l:libdwarfs_common.a", "-l:libdwarfs_metadata_legacy.a",
          "-l:libdwarfs_decompressor.a", "-l:libflatbuffers.a", "-l:libzip.a",
          "-l:libfmt.a", "-l:libxxhash.a", "-l:libboost_filesystem.a",
          "-l:libboost_chrono.a"
        ].freeze

        COMMON_ARCHIEVE_LIBRARIES = [
          "-l:liblz4.a",           "-l:libz.a",         "-l:libzstd.a",
          "-l:libbrotlienc.a",     "-l:libbrotlidec.a", "-l:libbrotlicommon.a",
          "-l:liblzma.a",          "-l:libbz2.a"
        ].freeze

        LINUX_GNU_LIBRARIES = [
          "-l:libiberty.a",      "-l:libacl.a",          "-l:libssl.a",           "-l:libcrypto.a",
          "-l:libgdbm.a",        "-l:libreadline.a",     "-l:libtinfo.a",         "-l:libffi.a",
          "-l:libncurses.a",     "-l:libjemalloc.a",     "-l:libcrypt.a",         "-l:libanl.a",
          "LIBYAML",             "-l:libutil.a",
          "-l:libstdc++.a",      "-lgcc_eh",             "-l:libunwind.a",        "-l:liblzma.a",
          "-l:librt.a",          "-ldl",                 "-lpthread",             "-lm"
        ].freeze

        LINUX_MUSL_LIBRARIES = [
          "-l:libiberty.a",       "-l:libacl.a",          "-l:libssl.a",          "-l:libcrypto.a",
          "-l:libreadline.a",     "-l:libgdbm.a",         "-l:libffi.a",          "-l:libncurses.a",
          "-l:libjemalloc.a",     "-l:libcrypt.a",        "LIBYAML",
          "-l:librt.a",           "-l:libstdc++.a",       "-lgcc_eh",
          " -l:libunwind.a",      "-l:liblzma.a",         "-ldl", "-lpthread"
        ].freeze

        MSYS_LIBRARIES = [
          "-l:liblz4.a",             "-l:libz.a",               "-l:libzstd.a",            "-l:liblzma.a",
          "-l:libncurses.a",         "-l:liblzma.a",            "-l:libiberty.a",          "LIBYAML",
          "-l:libffi.a",             "-l:libstdc++.a",          "-l:libdl.a",
          "-static-libgcc",          "-static-libstdc++",       "-l:libssl.a",             "-l:libcrypto.a",
          "-l:libz.a",               "-l:libwinpthread.a",      "-lcrypt32",               "-lshlwapi",
          "-lwsock32",               "-liphlpapi",              "-limagehlp",              "-lbcrypt",
          "-lole32",                 "-loleaut32",              "-luuid",                  "-lws2_32"
        ].freeze

        def linux_gnu_libraries(ruby_ver, with_compression)
          libraries = [GROUP_BEGIN] + COMMON_LINUX_LIBRARIES + COMMON_ARCHIEVE_LIBRARIES +
                      [GROUP_END] + LINUX_GNU_LIBRARIES
          linux_libraries(libraries, ruby_ver, with_compression)
        end

        def linux_musl_libraries(ruby_ver, with_compression)
          libraries = [GROUP_BEGIN] + COMMON_LINUX_LIBRARIES + COMMON_ARCHIEVE_LIBRARIES +
                      [GROUP_END] + LINUX_MUSL_LIBRARIES
          linux_libraries(libraries, ruby_ver, with_compression)
        end

        def linux_libraries(libraries, ruby_ver, _with_compression)
          libraries.map! do |lib|
            if lib == "LIBYAML"
              PatchHelpers.yaml_reference(ruby_ver)
            else
              lib
            end
          end
          libraries.join(" ")
        end

        def msys_libraries(ruby_ver, with_compression)
          libraries = with_compression ? ["-Wl,-Bstatic"] : []
          libraries = libraries + COMMON_LINUX_LIBRARIES + MSYS_LIBRARIES
          linux_libraries(libraries, ruby_ver, with_compression)
        end

        def process_brew_libs!(libs, brew_libs)
          brew_libs.each { |lib| libs << "#{PatchHelpers.get_prefix_macos(lib[0]).chop}/lib/lib#{lib[1]}.a " }
        end

        def darwin_libraries(deps_lib_dir, ruby_ver, _with_compression)
          libs = String.new

          DARWIN_DEP_LIBS_1.each { |lib| libs << "#{deps_lib_dir}/lib#{lib}.a " }
          process_brew_libs!(libs, ruby_ver.ruby31? ? DARWIN_BREW_LIBS_31 : DARWIN_BREW_LIBS_PRE_31)
          process_brew_libs!(libs, DARWIN_BREW_LIBS)

          # The vcpkg set by full path: Apple ld does not implement -l:<filename>
          vcpkg_lib_dir = Dir.glob(File.join(deps_lib_dir, "..", "vcpkg_installed", "*", "lib")).min
          DARWIN_DEP_LIBS_2.each { |lib| libs << "#{vcpkg_lib_dir}/lib#{lib}.a " }

          "-ltebako-fs #{libs}-ljemalloc -lc++ -lc++abi"
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

        def mlibs(ostype, deps_lib_dir, ruby_ver, with_compression)
          scmb = ScenarioManagerBase.new(ostype)
          if scmb.msys?
            msys_libraries(ruby_ver, with_compression)
          elsif scmb.macos?
            darwin_libraries(deps_lib_dir, ruby_ver, with_compression)
          elsif scmb.musl?
            linux_musl_libraries(ruby_ver, with_compression)
          else
            linux_gnu_libraries(ruby_ver, with_compression)
          end
        end
      end
    end
  end
end
