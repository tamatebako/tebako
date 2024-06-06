# frozen_string_literal: true

# Copyright (c) 2024 [Ribose Inc](https://www.ribose.com).
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

require "fileutils"
require "find"

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (stripper)
  module Stripper
    DELETE_EXTENSIONS = %w[o lo obj a lib].freeze
    STRIP_EXTENSIONS = %w[dll so].freeze
    BIN_FILES = %w[
      bundle bundler rbs erb gem irb racc racc2y rake rdoc ri y2racc rdbg typeprof
    ].freeze
    CMD_SUFFIX = ".cmd"
    BAT_SUFFIX = ".bat"

    class << self
      # Strip
      # Removes build artefacts, strip shared objects
      # [TODO] We probably need debug option/flag to say whether we shall delete ruby binaries
      #        from memfs or not. For debugging purposes it is very handy to have it here
      def strip(ostype, src_dir)
        puts "   ... stripping the output"
        strip_bs(ostype, src_dir)
        strip_fi(ostype, src_dir)
        strip_li(ostype, src_dir)
      end

      private

      def get_files(ostype)
        exe_suffix = ostype =~ /msys/ ? ".exe" : ""
        files = BIN_FILES.flat_map do |f|
          [f, "#{f}#{CMD_SUFFIX}", "#{f}#{BAT_SUFFIX}"]
        end

        files += ["ruby#{exe_suffix}", "rubyw#{exe_suffix}"]
        files
      end

      def strip_bs(_ostype, src_dir)
        FileUtils.rm_rf([
                          File.join(src_dir, "share"),
                          File.join(src_dir, "include"),
                          File.join(src_dir, "lib", "pkgconfig")
                        ])
      end

      def strip_fi(ostype, src_dir)
        files = get_files(ostype).map { |f| "#{src_dir}/bin/#{f}" }
        FileUtils.rm(files, force: true)
      end

      def strip_li(ostype, src_dir)
        Find.find(src_dir) do |file|
          next if File.directory?(file)

          extension = File.extname(file).delete_prefix(".").downcase
          if DELETE_EXTENSIONS.include?(extension)
            FileUtils.rm(file)
          elsif STRIP_EXTENSIONS.include?(extension)
            system("strip \"#{file}\"") unless ostype =~ /darwin/
          end
        end
      end
    end
  end
end
