# frozen_string_literal: true

# Copyright (c) 2023-2025 [Ribose Inc](https://www.ribose.com).
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

require "etc"
require "fileutils"
require "open3"
require "pathname"
require "rbconfig"

# Tebako - an executable packager
# Command-line interface methods
module Tebako
  # Cli helpers
  class OptionsManager # rubocop:disable Metrics/ClassLength
    # Press modes producing three-part packages (tebako-bootstrap + image
    # slots + tpkg trailer): 'lean' resolves the runtime at first run, 'fat'
    # additionally embeds it as a payload slot (self-installing, offline)
    THREE_PART_MODES = %w[lean fat].freeze

    def initialize(options)
      @options = options
      @rv = Tebako::RubyVersion.new(@options["Ruby"])
      @ruby_ver = @rv.ruby_version
      @scmb = ScenarioManagerBase.new
    end

    attr_reader :ruby_ver, :rv

    def cfg_options
      ## {v_parts[3]} may be something like rc1 that won't work with CMake
      v_parts = Tebako::VERSION.split(".")
      # Cannot use 'xxx' as parameters because it does not work in Windows shells
      # So we have to use \"xxx\"
      @cfg_options ||=
        "-DCMAKE_BUILD_TYPE=Release -DDEPS:STRING=\"#{deps}\" -G \"#{@scmb.m_files}\" -B \"#{output_folder}\" " \
        "-S \"#{source}\" -DTEBAKO_VERSION:STRING=\"#{v_parts[0]}.#{v_parts[1]}.#{v_parts[2]}\""
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

    # DATA_BIN_FILE is the packaged filesystem itself (fs.bin)
    def data_bundle_file
      @data_bundle_file ||= File.join(data_bin_dir, "fs.bin")
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

    def folder_within_root?(folder)
      folder_path = Pathname.new(folder.chomp("/"))
      root_path = Pathname.new(root.chomp("/"))
      folder_path.ascend do |path|
        return true if path == root_path
      end
      false
    end

    def fs_current
      fs_current = Dir.pwd
      if @scmb.msys?
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
      @mode ||= @options["mode"].nil? ? "lean" : @options["mode"]
    end

    def three_part?
      THREE_PART_MODES.include?(mode)
    end

    def fat?
      mode == "fat"
    end

    # Additional images for the stitched package, from repeatable
    # '--image <path>:<mount-point>' (split on the last colon so Windows
    # drive-letter paths survive)
    def images
      @images ||= Array(@options["image"]).map do |spec|
        path, _sep, mount = spec.to_s.rpartition(":")
        if path.empty? || mount.empty?
          Tebako.packaging_error(130, "invalid --image specification '#{spec}' ('<path>:<mount-point>' expected)")
        end

        { path: path, mount_point: mount, format_id: Tebako::Stitcher::FORMAT_DWARFS }
      end
    end

    # Platform id of the host, as used by tebako-runtime-ruby package names
    # (e.g. "macos-arm64", "linux-gnu-x86_64")
    def host_platform(ostype = RUBY_PLATFORM, arch = RbConfig::CONFIG["host_cpu"])
      @host_platform ||= "#{host_os_id(ostype)}-#{host_arch_id(arch)}"
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
      folder_within_root?(package)
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

    def prefix_within_root?
      folder_within_root?(prefix)
    end

    def press_announce
      <<~ANN
        Running tebako press at #{prefix}
           Mode:                      '#{mode}'
           Ruby version:              '#{@ruby_ver}'
           Project root:              '#{root}'
           Application entry point:   '#{fs_entrance}'
           Package file name:         '#{package}'
           Loging level:              '#{l_level}'
           Package working directory: '#{cwd_announce}'
      ANN
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
      @ruby_ver = @rv.ruby_version
    end

    def relative?(path)
      Pathname.new(path).relative?
    end

    def root
      f_root = @options["root"].nil? ? "" : @options["root"].gsub("\\", "/")
      @root ||= if relative?(f_root)
                  File.join(fs_current, f_root)
                else
                  File.join(f_root, "")
                end
    end

    def source
      c_path = Pathname.new(__FILE__).realpath
      @source ||= File.expand_path("../../..", c_path)
    end

    private

    def host_os_id(ostype)
      case ostype
      when /msys|mingw|cygwin/ then "windows"
      when /darwin/ then "macos"
      when /linux-musl/ then "linux-musl"
      when /linux/ then "linux-gnu"
      else
        Tebako.packaging_error(112, ostype)
      end
    end

    def host_arch_id(arch)
      case arch
      when /^(x86_64|amd64|x64)$/ then "x86_64"
      when /^(aarch64|arm64)$/ then "arm64"
      else
        Tebako.packaging_error(112, arch)
      end
    end
  end
end
