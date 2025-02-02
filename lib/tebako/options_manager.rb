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

require_relative "codegen"
require_relative "error"
require_relative "ruby_version"
require_relative "version"

# Tebako - an executable packager
# Command-line interface methods
module Tebako
  # Cli helpers
  class OptionsManager # rubocop:disable Metrics/ClassLength
    def initialize(options)
      @options = options
      @rv = Tebako::RubyVersion.new(@options["Ruby"])
      @ruby_ver, @ruby_hash = @rv.extend_ruby_version
    end

    attr_reader :ruby_ver, :rv

    def b_env
      u_flags = if RbConfig::CONFIG["host_os"] =~ /darwin/
                  "-DTARGET_OS_SIMULATOR=0 -DTARGET_OS_IPHONE=0  #{ENV.fetch("CXXFLAGS", nil)}"
                else
                  ENV.fetch("CXXFLAGS", nil)
                end
      @b_env ||= { "CXXFLAGS" => u_flags }
    end

    def cfg_options
      ## {v_parts[3]} may be something like rc1 that won't work with CMake
      v_parts = Tebako::VERSION.split(".")
      # Cannot use 'xxx' as parameters because it does not work in Windows shells
      # So we have to use \"xxx\"
      @cfg_options ||=
        "-DCMAKE_BUILD_TYPE=Release -DRUBY_VER:STRING=\"#{@ruby_ver}\" -DRUBY_HASH:STRING=\"#{@ruby_hash}\" " \
        "-DDEPS:STRING=\"#{deps}\" -G \"#{m_files}\" -B \"#{output_folder}\" -S \"#{source}\" " \
        "#{remove_glibc_private} -DTEBAKO_VERSION:STRING=\"#{v_parts[0]}.#{v_parts[1]}.#{v_parts[2]}\""
    end

    def cwd
      f_cwd = @options["cwd"]&.gsub("\\", "/")
      @cwd ||= f_cwd
    end

    def cwd_announce
      @cwd_announce ||= cwd.nil? ? "<Host current directory>" : cwd
    end

    # DATA_BIN_DIR folder is used to create packaged filesystem
    # set(DATA_BIN_DIR  ${CMAKE_CURRENT_BINARY_DIR}/p)
    def data_bin_dir
      @data_bin_dir ||= File.join(output_folder, "p")
    end

    #  Mode       File(s)                       Content
    #  bundle     fs.bin                      Application
    #  both       fs.bin, fs2.bin     Stub, application respectively
    #  runtime    fs.bin                         Stub
    #  app        fs2.bin                     Application

    def data_bundle_file
      @data_bundle_file ||= File.join(data_bin_dir, "fs.bin")
    end

    def data_stub_file
      @data_stub_file ||= File.join(data_bin_dir, "fs.bin")
    end

    def data_app_file
      @data_app_file ||= File.join(data_bin_dir, "fs2.bin")
    end

    # DATA_PRE_DIR folder is used to build gems  that need to be packaged
    # set(DATA_PRE_DIR  ${CMAKE_CURRENT_BINARY_DIR}/r)
    def data_pre_dir
      @data_pre_dir ||= File.join(output_folder, "r")
    end

    # DATA_SRC_DIR folder is used to collect all files that need to be packaged
    # set(DATA_SRC_DIR  ${CMAKE_CURRENT_BINARY_DIR}/s)
    def data_src_dir
      @data_src_dir ||= File.join(output_folder, "s")
    end

    def deps
      @deps ||= File.join(prefix, "deps")
    end

    def deps_bin_dir
      @deps_bin_dir ||= File.join(deps, "bin")
    end

    def deps_lib_dir
      @deps_lib_dir ||= File.join(deps, "lib")
    end

    def fs_current
      fs_current = Dir.pwd
      if RUBY_PLATFORM =~ /msys|mingw|cygwin/
        fs_current, cygpath_res = Open3.capture2e("cygpath", "-w", fs_current)
        Tebako.packaging_error(101) unless cygpath_res.success?
        fs_current.strip!
      end
      @fs_current ||= fs_current&.gsub("\\", "/")
    end

    def fs_entrance
      @fs_entrance ||= @options["entry-point"]&.gsub("\\", "/")
    end

    def handle_nil_prefix
      env_prefix = ENV.fetch("TEBAKO_PREFIX", nil)
      if env_prefix.nil?
        puts "No prefix specified, using ~/.tebako"
        File.expand_path("~/.tebako")
      else
        puts "Using TEBAKO_PREFIX environment variable as prefix"
        File.expand_path(env_prefix.gsub("\\", "/"))
      end
    end

    def l_level
      @l_level ||= @options["log-level"].nil? ? "error" : @options["log-level"]
    end

    def mode
      @mode ||= @options["mode"].nil? ? "bundle" : @options["mode"]
    end

    def m_files
      # [TODO]
      # Ninja generates incorrect script for tebako press target -- gets lost in a chain custom targets
      # Using makefiles has negative performance impact so it needs to be fixed
      @m_files ||= case RUBY_PLATFORM
                   when /linux/, /darwin/
                     "Unix Makefiles"
                   when /msys|mingw|cygwin/
                     "MinGW Makefiles"
                   else
                     raise Tebako::Error.new("#{RUBY_PLATFORM} is not supported.", 112)
                   end
    end

    def output_folder
      @output_folder ||= File.join(prefix, "o")
    end

    def package
      package = if @options["output"].nil?
                  File.join(Dir.pwd, File.basename(fs_entrance, ".*"))
                else
                  @options["output"]&.gsub("\\", "/")
                end
      @package ||= if relative?(package)
                     File.join(fs_current, package)
                   else
                     package
                   end
    end

    def package_within_root?
      package_path = Pathname.new(package.chomp("/"))
      root_path = Pathname.new(root.chomp("/"))
      package_path.ascend do |path|
        return true if path == root_path
      end
      false
    end

    def prefix
      @prefix ||= if @options["prefix"].nil?
                    handle_nil_prefix
                  elsif @options["prefix"] == "PWD"
                    Dir.pwd
                  else
                    File.expand_path(@options["prefix"]&.gsub("\\", "/"))
                  end
    end

    def press_announce(is_msys)
      case mode
      when "application"
        press_announce_application(is_msys)
      when "both"
        press_announce_both
      when "bundle"
        press_announce_bundle
      when "runtime"
        press_announce_runtime
      end
    end

    def press_announce_ref(is_msys)
      if is_msys
        " referencing runtime at '#{ref}'"
      else
        ""
      end
    end

    def press_announce_application(is_msys)
      <<~ANN
        Running tebako press at #{prefix}
           Mode:                      'application'
           Ruby version:              '#{@ruby_ver}'
           Project root:              '#{root}'
           Application entry point:   '#{fs_entrance}'
           Package file name:         '#{package}.tebako'#{press_announce_ref(is_msys)}
           Package working directory: '#{cwd_announce}'
      ANN
    end

    def press_announce_both
      <<~ANN
        Running tebako press at #{prefix}
           Mode:                      'both'
           Ruby version:              '#{@ruby_ver}'
           Project root:              '#{root}'
           Application entry point:   '#{fs_entrance}'
           Runtime file name:         '#{package}'
           Package file name:         '#{package}.tebako'
           Loging level:              '#{l_level}'
           Package working directory: '#{cwd_announce}'
      ANN
    end

    def press_announce_bundle
      <<~ANN
        Running tebako press at #{prefix}
           Mode:                      'bundle'
           Ruby version:              '#{@ruby_ver}'
           Project root:              '#{root}'
           Application entry point:   '#{fs_entrance}'
           Package file name:         '#{package}'
           Loging level:              '#{l_level}'
           Package working directory: '#{cwd_announce}'
      ANN
    end

    def press_announce_runtime
      <<~ANN
        Running tebako press at #{prefix}
           Mode:                      'runtime'
           Ruby version:              '#{@ruby_ver}'
           Runtime file name:         '#{package}'
           Loging level:              '#{l_level}'
      ANN
    end

    def press_options
      @press_options ||= "-DPCKG:STRING='#{package}' -DLOG_LEVEL:STRING='#{l_level}' " \
    end

    def process_gemfile(gemfile_path)
      folder = File.dirname(gemfile_path)
      filename = File.basename(gemfile_path)
      # Change directory to the folder containing the Gemfile
      # Because Bundler::Definition.build *sometimes* requires to be in
      # the Gemfile directory
      Dir.chdir(folder) do
        @rv = Tebako::RubyVersionWithGemfile.new(@options["Ruby"], filename)
      end
      @ruby_ver, @ruby_hash = @rv.extend_ruby_version
      @ruby_src_dir = nil
    end

    def relative?(path)
      Pathname.new(path).relative?
    end

    def ref
      @ref ||= @options["ref"].nil? ? "tebako-runtime" : @options["ref"].gsub("\\", "/")
    end

    def remove_glibc_private
      @remove_glibc_private ||= if RUBY_PLATFORM.end_with?("linux") || RUBY_PLATFORM.end_with?("linux-gnu")
                                  "-DREMOVE_GLIBC_PRIVATE=#{@options["patchelf"] ? "ON" : "OFF"}"
                                else
                                  ""
                                end
    end

    def root
      f_root = @options["root"].nil? ? "" : @options["root"].gsub("\\", "/")
      @root ||= if relative?(f_root)
                  File.join(fs_current, f_root)
                else
                  File.join(f_root, "")
                end
    end

    def ruby_src_dir
      @ruby_src_dir ||= File.join(deps, "src", "_ruby_#{@ruby_ver}")
    end

    def source
      c_path = Pathname.new(__FILE__).realpath
      @source ||= File.expand_path("../../..", c_path)
    end

    def stash_dir(rver = nil)
      @stash_dir ||= "#{stash_dir_all}_#{rver || @ruby_ver}"
    end

    def stash_dir_all
      @stash_dir_all ||= File.join(deps, "stash")
    end
  end
end
