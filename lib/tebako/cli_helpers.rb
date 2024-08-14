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

require "etc"
require "fileutils"
require "pathname"
require "rbconfig"

require_relative "cli_rubies"
require_relative "error"
require_relative "version"

# Tebako - an executable packager
# Command-line interface methods
module Tebako
  E_VERSION_FILE = ".environment.version"
  # Cli helpers
  module CliHelpers
    def b_env
      u_flags = if RbConfig::CONFIG["host_os"] =~ /darwin/
                  "-DTARGET_OS_SIMULATOR=0 -DTARGET_OS_IPHONE=0  #{ENV.fetch("CXXFLAGS", nil)}"
                else
                  ENV.fetch("CXXFLAGS", nil)
                end
      @b_env ||= { "CXXFLAGS" => u_flags }
    end

    def cfg_options
      ruby_ver, ruby_hash = extend_ruby_version
      # Cannot use 'xxx' as parameters because it does not work in Windows shells
      # So we have to use \"xxx\"
      @cfg_options ||=
        "-DCMAKE_BUILD_TYPE=Release -DRUBY_VER:STRING=\"#{ruby_ver}\" -DRUBY_HASH:STRING=\"#{ruby_hash}\" " \
        "-DDEPS:STRING=\"#{deps}\" -G \"#{m_files}\" -B \"#{output_folder}\" -S \"#{source}\" " \
        "-DTEBAKO_VERSION:STRING=\"#{Tebako::VERSION}\""
    end

    def clean_cache
      puts "Cleaning tebako packaging environment"
      # Using File.join(deps, "") to ensure that the slashes are appropriate
      FileUtils.rm_rf([File.join(deps, ""), File.join(output_folder, "")], secure: true)
    end

    def clean_output
      puts "Cleaning CMake cache and Ruby build files"

      nmr = "src/_ruby_*"
      nms = "stash_*"
      FileUtils.rm_rf(Dir.glob(File.join(deps, nmr)), secure: true)
      FileUtils.rm_rf(Dir.glob(File.join(deps, nms)), secure: true)

      # Using File.join(output_folder, "") to ensure that the slashes are appropriate
      FileUtils.rm_rf(File.join(output_folder, ""), secure: true)
    end

    def deps
      @deps ||= File.join(prefix, "deps")
    end

    def do_press
      cfg_cmd = "cmake -DSETUP_MODE:BOOLEAN=OFF #{cfg_options} #{press_options}"
      build_cmd = "cmake --build #{output_folder} --target tebako --parallel #{Etc.nprocessors}"
      merged_env = ENV.to_h.merge(b_env)
      Tebako.packaging_error(103) unless system(merged_env, cfg_cmd)
      Tebako.packaging_error(104) unless system(merged_env, build_cmd)
    end

    def do_setup
      cfg_cmd = "cmake -DSETUP_MODE:BOOLEAN=ON #{cfg_options}"
      build_cmd = "cmake --build \"#{output_folder}\" --target setup --parallel #{Etc.nprocessors}"
      merged_env = ENV.to_h.merge(b_env)
      Tebako.packaging_error(101) unless system(merged_env, cfg_cmd)
      Tebako.packaging_error(102) unless system(merged_env, build_cmd)
    end

    def ensure_version_file
      version_file_path = File.join(deps, E_VERSION_FILE)

      begin
        File.write(version_file_path, version_key)
        # puts "Set version information for tebako packaging environment to #{Tebako::VERSION}"
      rescue StandardError => e
        puts "An error occurred while creating or updating #{E_VERSION_FILE}: #{e.message}"
      end
    end

    def fs_current
      fs_current = Dir.pwd
      if RUBY_PLATFORM =~ /msys|mingw|cygwin/
        fs_current, cygpath_res = Open3.capture2e("cygpath", "-w", fs_current)
        Tebako.packaging_error(101) unless cygpath_res.success?
        fs_current.strip!
      end
      @fs_current ||= fs_current
    end

    def l_level
      @l_level ||= if options["log-level"].nil?
                     "error"
                   else
                     options["log-level"]
                   end
    end

    # rubocop:disable Metrics/MethodLength
    def m_files
      # [TODO]
      # Ninja generates incorrect script fot tebako press target -- gets lost in a chain custom targets
      # Using makefiles has negative performance impact so it needs to be fixed
      @m_files ||= case RUBY_PLATFORM
                   when /linux/, /darwin/
                     "Unix Makefiles"
                   when /msys|mingw|cygwin/
                     "MinGW Makefiles"
                   else
                     raise Tebako::Error.new(
                       "#{RUBY_PLATFORM} is not supported, exiting",
                       254
                     )
                   end
    end
    # rubocop:enable Metrics/MethodLength

    def options_from_tebafile(tebafile)
      ::YAML.load_file(tebafile)["options"] || {}
    rescue Psych::SyntaxError => e
      puts "Warning: The tebafile '#{tebafile}' contains invalid YAML syntax."
      puts e.message
      {}
    rescue StandardError => e
      puts "An unexpected error occurred while loading the tebafile '#{tebafile}'."
      puts e.message
      {}
    end

    def output_folder
      @output_folder ||= File.join(prefix, "o")
    end

    def package
      package = if options["output"].nil?
                  File.join(Dir.pwd, File.basename(options["entry-point"], ".*"))
                else
                  options["output"]
                end
      @package ||= if relative?(package)
                     File.join(fs_current, package)
                   else
                     package
                   end
    end

    def handle_nil_prefix
      env_prefix = ENV.fetch("TEBAKO_PREFIX", nil)
      if env_prefix.nil?
        puts "No prefix specified, using ~/.tebako"
        File.expand_path("~/.tebako")
      else
        puts "Using TEBAKO_PREFIX environment variable as prefix"
        File.expand_path(env_prefix)
      end
    end

    def prefix
      @prefix ||= if options["prefix"].nil?
                    handle_nil_prefix
                  elsif options["prefix"] == "PWD"
                    Dir.pwd
                  else
                    File.expand_path(options["prefix"])
                  end
    end

    def press_announce
      cwd_announce = options["cwd"].nil? ? "<Host current directory>" : options["cwd"]
      @press_announce ||= <<~ANN
        Running tebako press at #{prefix}
           Ruby version:              '#{extend_ruby_version[0]}'
           Project root:              '#{root}'
           Application entry point:   '#{options["entry-point"]}'
           Package file name:         '#{package}'
           Loging level:              '#{l_level}'
           Package working directory: '#{cwd_announce}'
      ANN
    end

    def press_options
      cwd_option = if options["cwd"].nil?
                     "-DPACKAGE_NEEDS_CWD:BOOL=OFF"
                   else
                     "-DPACKAGE_NEEDS_CWD:BOOL=ON -DPACKAGE_CWD:STRING='#{options["cwd"]}'"
                   end
      @press_options ||=
        "-DROOT:STRING='#{root}' -DENTRANCE:STRING='#{options["entry-point"]}' " \
        "-DPCKG:STRING='#{package}' -DLOG_LEVEL:STRING='#{options["log-level"]}' " \
        "#{cwd_option}"
    end

    def relative?(path)
      Pathname.new(path).relative?
    end

    def root
      @root ||= if relative?(options["root"])
                  File.join(fs_current, options["root"])
                else
                  File.join(options["root"], "")
                end
    end

    def source
      c_path = Pathname.new(__FILE__).realpath
      @source ||= File.expand_path("../../..", c_path)
    end

    def version_key
      @version_key ||= "#{Tebako::VERSION} at #{source}"
    end

    def version_cache
      version_file_path = File.join(deps, E_VERSION_FILE)
      file_version = File.open(version_file_path, &:readline).strip
      file_version.match(/(?<version>.+) at (?<source>.+)/)
    end

    def version_cache_check
      match_data = version_cache

      return version_unknown unless match_data

      if match_data[:version] != Tebako::VERSION
        version_mismatch(match_data[:version])
      elsif match_data[:source] != source
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
           "and cannot be used for '#{source}'"
      clean_output
    end

    def version_unknown
      puts "CMake cache version was not recognized, cleaning up"
      clean_cache
    end
  end
end
