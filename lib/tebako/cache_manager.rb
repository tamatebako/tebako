# frozen_string_literal: true

# Copyright (c)  2024-2025 [Ribose Inc](https://www.ribose.com).
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
require_relative "version"

# Tebako - an executable packager
module Tebako
  # Cache management
  class CacheManager
    E_VERSION_FILE = ".environment.version"

    def initialize(deps, src_dir, out_dir)
      @deps = deps
      @src_dir = src_dir
      @out_dir = out_dir
    end

    def clean_cache
      puts "Cleaning tebako packaging environment"
      # Using File.join(deps, "") to ensure that the slashes are appropriate
      FileUtils.rm_rf([File.join(@deps, ""), File.join(@out_dir, "")], secure: true)
    end

    def clean_output
      puts "Cleaning CMake cache and Ruby build files"

      nmr = "src/_ruby_*"
      nms = "stash_*"
      FileUtils.rm_rf(Dir.glob(File.join(@deps, nmr)), secure: true)
      FileUtils.rm_rf(Dir.glob(File.join(@deps, nms)), secure: true)

      # Using File.join(output_folder, "") to ensure that the slashes are appropriate
      FileUtils.rm_rf(File.join(@out_dir, ""), secure: true)
    end

    def ensure_version_file
      version_file_path = File.join(@deps, E_VERSION_FILE)

      begin
        File.write(version_file_path, version_key)
        # puts "Set version information for tebako packaging environment to #{Tebako::VERSION}"
      rescue StandardError => e
        puts "#{Tebako::PACKAGING_ERRORS[201]} #{E_VERSION_FILE}: #{e.message}"
      end
    end

    def version_key
      @version_key ||= "#{Tebako::VERSION} at #{@src_dir}"
    end

    def version_cache
      version_file_path = File.join(@deps, E_VERSION_FILE)
      file_version = File.open(version_file_path, &:readline).strip
      file_version.match(/(?<version>.+) at (?<source>.+)/)
    end

    def version_cache_check
      match_data = version_cache
      return version_unknown unless match_data

      if match_data[:version] != Tebako::VERSION
        version_mismatch(match_data[:version])
      elsif match_data[:source] != @src_dir
        version_source_mismatch(match_data[:source])
      end
    rescue StandardError
      version_unknown
    end

    def version_mismatch(cached_version)
      puts "Tebako cache was created by a gem version #{cached_version} " \
           "and cannot be used for gem version #{Tebako::VERSION}"
      clean_cache
    end

    def version_source_mismatch(cached_source)
      puts "CMake cache was created for a different source directory '#{cached_source}' " \
           "and cannot be used for '#{@src_dir}'"
      clean_output
    end

    def version_unknown
      puts "CMake cache version was not recognized, cleaning up"
      clean_cache
    end
  end
end
