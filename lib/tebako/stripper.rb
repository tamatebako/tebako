# frozen_string_literal: true

# Copyright (c) 2024-2025 [Ribose Inc](https://www.ribose.com).
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

require "fileutils"
require "find"

require_relative "packager/patch_helpers"

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (stripper)
  module Stripper
    DELETE_EXTENSIONS = %w[o lo obj a la lib].freeze
    BIN_FILES = %w[
      bundle bundler rbs erb gem irb racc racc2y
      rake rdoc ri y2racc rdbg syntax_suggest typeprof
    ].freeze
    CMD_SUFFIX = ".cmd"
    BAT_SUFFIX = ".bat"

    class << self
      # Strip
      # Removes build artefacts, strip shared objects
      def strip(scm, src_dir)
        puts "   ... stripping the output"
        strip_bs(src_dir)
        strip_fi(scm, src_dir)
        strip_li(scm, src_dir)
      end

      def strip_file(file_in, file_out = nil)
        params = ["strip", "-S", file_in]
        params << "-o" << file_out unless file_out.nil?
        out, st = Open3.capture2e(*params)

        # Some gems (well, libmspack) has bundled extensions for several architectures)
        # Getting something like:
        # strip: Unable to recognise the format of the input file
        # `/tmp/cirrus-ci-build/o/s/lib/ruby/gems/3.1.0/gems/libmspack-0.11.0/ext/x86_64-linux/libmspack.so'
        # on aarch64

        puts "Warning: could not strip #{file_in}:\n #{out}" unless st.exitstatus.zero?
      end

      private

      def get_files(scm)
        exe_suffix = scm.exe_suffix
        files = BIN_FILES.flat_map do |f|
          [f, "#{f}#{CMD_SUFFIX}", "#{f}#{BAT_SUFFIX}"]
        end

        files += ["ruby#{exe_suffix}", "rubyw#{exe_suffix}"]
        files
      end

      def strip_bs(src_dir)
        FileUtils.rm_rf([
                          File.join(src_dir, "share"),
                          File.join(src_dir, "include"),
                          File.join(src_dir, "lib", "pkgconfig")
                        ])
      end

      def strip_fi(scm, src_dir)
        files = get_files(scm).map { |f| "#{src_dir}/bin/#{f}" }
        FileUtils.rm(files, force: true)
      end

      def strip_li(scm, src_dir)
        sext = strip_extensions(scm)
        Find.find(src_dir) do |file|
          next if File.directory?(file)

          extension = File.extname(file).delete_prefix(".").downcase
          if DELETE_EXTENSIONS.include?(extension)
            FileUtils.rm(file)
          elsif sext.include?(extension)
            strip_file(file)
          end
        end
      end

      def strip_extensions(scm)
        sext = ["so"]
        sext << "dll" if scm.msys?
        sext << "dylib" << "bundle" if scm.macos?
        sext
      end
    end
  end
end
