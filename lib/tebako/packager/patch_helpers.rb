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

require "open3"
require_relative "patch_literals"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching helpers (pass2)
    module PatchHelpers
      class << self
        def patch_file(fname, mapping)
          raise Tebako::Error, "Could not patch #{fname} because it does not exist." unless File.exist?(fname)

          puts "   ... patching #{fname}"
          restore_and_save(fname)
          contents = File.read(fname)

          mapping.each { |pattern, subst| contents.sub!(pattern, subst) }
          File.open(fname, "w") { |file| file << contents }
        end

        def get_prefix_macos(package)
          out, st = Open3.capture2("brew --prefix #{package}")
          raise Tebako::Error, "brew --prefix #{package} failed with code #{st.exitstatus}" unless st.exitstatus.zero?

          out
        end

        def get_prefix_linux(package)
          out, st = Open3.capture2("pkg-config --variable=libdir #{package}")
          unless st.exitstatus.zero?
            raise Tebako::Error,
                  "pkg-config --variable=libdir #{package} failed with code #{st.exitstatus}"
          end

          out
        end

        def patch_c_file_pre(pattern)
          {
            pattern => "#{PatchLiterals::C_FILE_SUBST}\n#{pattern}"
          }
        end

        def patch_c_file_post(pattern)
          {
            pattern => "#{pattern}\n\n#{PatchLiterals::C_FILE_SUBST}"
          }
        end

        def recreate(dirname)
          FileUtils.rm_rf(dirname, noop: nil, verbose: nil, secure: true)
          FileUtils.mkdir(dirname)
        end

        def restore_and_save(fname, strict: true)
          unless File.exist?(fname)
            return unless strict

            raise Tebako::Error, "Could not save #{fname} because it does not exist."
          end

          old_fname = "#{fname}.old"
          if File.exist?(old_fname)
            FileUtils.rm_f(fname)
            File.rename(old_fname, fname)
          end
          FileUtils.cp(fname, old_fname)
        end

        def restore_and_save_files(files, ruby_source_dir, strict: true)
          files.each do |fname|
            restore_and_save("#{ruby_source_dir}/#{fname}", strict: strict)
          end
        end

        def yaml_reference(ruby_ver)
          ruby_ver.ruby32? ? "-l:libyaml.a" : ""
        end
      end
    end
  end
end
