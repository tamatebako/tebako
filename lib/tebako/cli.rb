#!/usr/bin/env ruby
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

require "digest"
require "fileutils"
require "open3"
require "thor"
require "yaml"

require_relative "cache_manager"
require_relative "cli_helpers"
require_relative "error"
require_relative "ruby_version"
require_relative "version"

# Tebako - an executable packager
# Implementation of tebako command-line interface
module Tebako
  DEFAULT_TEBAFILE = ".tebako.yml"
  # Tebako packager front-end
  class Cli < Thor
    package_name "Tebako"
    class_option :prefix, type: :string, aliases: "-p", required: false,
                          desc: "A path to tebako packaging environment, '~/.tebako' ('$HOME/.tebako') by default"
    class_option :devmode, type: :boolean, aliases: "-D",
                           desc: "Developer mode, please do not use if unsure"
    class_option :tebafile, type: :string, aliases: "-t", required: false,
                            desc: "tebako configuration file 'tebafile', '$PWD/.tebako.yml' by default"
    desc "clean", "Clean tebako packaging environment"
    def clean
      (_, cm) = bootstrap
      cm.clean_cache
    end

    desc "clean_ruby", "Clean Ruby source from tebako packaging environment"
    method_option :Ruby, type: :string, aliases: "-R", required: false,
                         enum: Tebako::RubyVersion::RUBY_VERSIONS.keys,
                         desc: "Ruby version to clean, all available versions by default"
    def clean_ruby
      puts "Cleaning Ruby sources from tebako packaging environment"
      (om,) = bootstrap

      suffix = options["Ruby"].nil? ? "" : "_#{options["Ruby"]}"
      nmr = "src/_ruby#{suffix}*"
      nms = "stash#{suffix}*"
      FileUtils.rm_rf(Dir.glob(File.join(om.deps, nmr)), secure: true)
      FileUtils.rm_rf(Dir.glob(File.join(om.deps, nms)), secure: true)
    end

    desc "hash", "Print build script hash (ci cache key)"
    def hash
      print Digest::SHA256.hexdigest [File.read(File.join(source, "CMakeLists.txt")), Tebako::VERSION].join
    end

    CWD_DESCRIPTION = <<~DESC
      Current working directory for packaged application. This directory shall be specified relative to root.
      #{" " * 65}# If this parameter is not set, the application will start in the current directory of the host file system.
    DESC

    RGP_DESCRIPTION = <<~DESC
      Activates removal a reference to GLIBC_PRIVATE version of libpthread from tebako package. This allows Linux Gnu packages to run against versions of
      #{" " * 65}# libpthread that differ from the version used for packaging. For example, package created at Ubuntu 20 system can be used on Ubuntu 22. This option works on Gnu Linux with
      #{" " * 65}# Gnu toolchain only (not for LLVM/clang). The feature is exeprimental, we may consider other approach in the future.
    DESC

    desc "press", "Press tebako image"
    method_option :cwd, type: :string, aliases: "-c", required: false,
                        desc: CWD_DESCRIPTION
    method_option :"entry-point", type: :string, aliases: ["-e", "--entry"], required: true,
                                  desc: "Ruby application entry point"
    method_option :"log-level", type: :string, aliases: "-l", required: false, enum: %w[error warn debug trace],
                                desc: "Tebako memfs logging level, 'error' by default"
    method_option :output, type: :string, aliases: "-o", required: false,
                           desc: "Tebako package file name, entry point base file name in the current folder by default"
    method_option :root, type: :string, aliases: "-r", required: true, desc: "Root folder of the Ruby application"
    method_option :Ruby, type: :string, aliases: "-R", required: false,
                         enum: Tebako::RubyVersion::RUBY_VERSIONS.keys,
                         desc: "Tebako package Ruby version, #{Tebako::RubyVersion::DEFAULT_RUBY_VERSION} by default"
    method_option :patchelf, aliases: "-P", type: :boolean,
                             desc: RGP_DESCRIPTION
    def press
      (om, cm) = bootstrap

      do_press(om)
      cm.ensure_version_file
    rescue Tebako::Error => e
      puts "Tebako script failed: #{e.message} [#{e.error_code}]"
      exit e.error_code
    end

    desc "setup", "Set up tebako packaging environment"
    method_option :Ruby, type: :string, aliases: "-R", required: false,
                         enum: Tebako::RubyVersion::RUBY_VERSIONS.keys,
                         desc: "Tebako package Ruby version, #{Tebako::RubyVersion::DEFAULT_RUBY_VERSION} by default."
    def setup
      (om, cm) = bootstrap

      do_setup(om)
      cm.ensure_version_file
    rescue Tebako::Error => e
      puts "Tebako script failed: #{e.message} [#{e.error_code}]"
      exit e.error_code
    end

    def self.exit_on_failure?
      true
    end

    no_commands do
      def bootstrap
        options_manager = Tebako::OptionsManager.new(options)
        cache_manager = Tebako::CacheManager.new(options_manager.deps, options_manager.source,
                                                 options_manager.output_folder)
        cache_manager.version_cache_check unless options[:devmode]
        [options_manager, cache_manager]
      end

      def initialize(*args)
        super
        return if args[2][:current_command].name.include?("hash")

        puts "Tebako executable packager version #{Tebako::VERSION}"
      end

      def options
        original_options = super
        tebafile = original_options["tebafile"].nil? ? DEFAULT_TEBAFILE : original_options["tebafile"]
        if File.exist?(tebafile)
          Thor::CoreExt::HashWithIndifferentAccess.new(options_from_tebafile(tebafile).merge(original_options))
        else
          puts "Warning: Tebako configuration file '#{tebafile}' not found." unless original_options["tebafile"].nil?
          original_options
        end
      end
    end

    no_commands do
      include Tebako::CliHelpers
    end
  end
end
