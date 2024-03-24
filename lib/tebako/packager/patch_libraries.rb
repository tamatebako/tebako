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

require_relative "patch_helpers"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching definitions (pass2)
    module PatchLibraries
      class << self
        # rubocop:disable Style/WordArray

        # NOTE: folly provides build-in implementation of jemalloc

        DARWIN_BREW_LIBS = [
          ["zlib", "z"],            ["gdbm", "gdbm"],           ["readline", "readline"], ["libffi", "ffi"],
          ["ncurses", "ncurses"],   ["fmt", "fmt"],             ["lz4", "lz4"],           ["xz", "lzma"],
          ["libyaml", "yaml"],      ["boost", "boost_chrono"],
          ["double-conversion", "double-conversion"]
        ].freeze

        DARWIN_BREW_LIBS_PRE_31 = [["openssl@1.1", "ssl"], ["openssl@1.1", "crypto"]].freeze

        DARWIN_BREW_LIBS_31 = [["libyaml", "yaml"], ["openssl@3", "ssl"], ["openssl@3", "crypto"]].freeze

        DARWIN_DEP_LIBS = ["glog", "gflags", "brotlienc", "brotlidec", "brotlicommon"].freeze
        # rubocop:enable Style/WordArray

        def process_brew_libs!(libs, brew_libs)
          brew_libs.each { |lib| libs << "#{PatchHelpers.get_prefix_macos(lib[0]).chop}/lib/lib#{lib[1]}.a " }
        end

        def darwin_libs(deps_lib_dir, ruby_ver)
          libs = String.new

          process_brew_libs!(libs, PatchHelpers.ruby31?(ruby_ver) ? DARWIN_BREW_LIBS_31 : DARWIN_BREW_LIBS_PRE_31)
          process_brew_libs!(libs, DARWIN_BREW_LIBS)

          DARWIN_DEP_LIBS.each { |lib| libs << "#{deps_lib_dir}/lib#{lib}.a " }
          <<~SUBST
            -ltebako-fs -ldwarfs-wr -ldwarfs -force_load #{deps_lib_dir}/libdwarfs_compression.a -lfolly -lfsst -lmetadata_thrift -lthrift_light -lxxhash \
            -lzstd #{libs} -ljemalloc -lc++ -lc++abi
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

        def linux_common_libs
          <<~SUBST
            -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -Wl,--push-state,--whole-archive -l:libdwarfs_compression.a -Wl,--pop-state -l:libfolly.a -l:libfsst.a           \\
            -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a                         \\
          SUBST
        end

        def common_enc_libs
          "-l:liblz4.a -l:libz.a -l:libzstd.a -l:libbrotlienc.a -l:libbrotlidec.a -l:libbrotlicommon.a -l:liblzma.a"
        end

        def linux_gnu_libs(ruby_ver)
          <<~SUBST
            #{linux_common_libs} \
            -l:libarchive.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a #{common_enc_libs} \\
            -l:libgdbm.a -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libcrypt.a -l:libanl.a #{PatchHelpers.yaml_reference(ruby_ver)} \\
            -l:libboost_system.a -l:libboost_chrono.a -l:libutil.a -l:libstdc++.a -lgcc_eh -l:libunwind.a -l:liblzma.a -l:librt.a -ldl -lpthread -lm
          SUBST
        end

        def linux_musl_libs(ruby_ver)
          <<~SUBST
            #{linux_common_libs} \
            -l:libiberty.a  -l:libacl.a -l:libssl.a -l:libcrypto.a #{common_enc_libs} -l:libreadline.a  \\
            -l:libgdbm.a  -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libcrypt.a  #{PatchHelpers.yaml_reference(ruby_ver)} -l:libboost_system.a -l:libboost_chrono.a \\
            -l:librt.a -l:libstdc++.a -lgcc_eh -l:libunwind.a -l:liblzma.a -ldl -lpthread
          SUBST
        end

        def msys_libs(ruby_ver)
          <<~SUBST
            -Wl,-Bstatic #{linux_common_libs} \
            -l:liblz4.a -l:libz.a -l:libzstd.a -l:liblzma.a -l:libncurses.a -l:libunwind.a -l:liblzma.a -l:libiberty.a #{PatchHelpers.yaml_reference(ruby_ver)}            \\
            -l:libffi.a -l:libboost_system-mt.a -l:libboost_chrono-mt.a -l:libstdc++.a -l:libdl.a -static-libgcc -static-libstdc++ -l:libssl.a -l:libcrypto.a -l:libz.a    \\
            -l:libwinpthread.a -lcrypt32 -lshlwapi -lwsock32 -liphlpapi -limagehlp -lshlwapi -lbcrypt -lws2_32 -lole32 -loleaut32 -luuid
          SUBST
        end

        def mlibs(ostype, deps_lib_dir, ruby_ver) # rubocop:disable Metrics/MethodLength
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
      end
    end
  end
end
