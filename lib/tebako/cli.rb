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

require_relative "cli_helpers"
require_relative "cli_rubies"
require_relative "error"
require_relative "version"

# Tebako - an executable packager
# Implementation of tebako command-line interface
module Tebako
  OPTIONS_FILE = ".tebako.yml"
  # Tebako packager front-end
  class Cli < Thor
    package_name "Tebako"
    class_option :prefix, type: :string, aliases: "-p", required: false,
                          desc: "A path to tebako packaging environment, '~/.tebako' ('$HOME/.tebako') by default"
    class_option :devmode, type: :boolean, aliases: "-D", required: false,
                           desc: "Developer mode, please do not use if unsure"

    desc "clean", "Clean tebako packaging environment"
    def clean
      clean_cache
    end

    desc "clean_ruby", "Clean Ruby source from tebako packaging environment"
    method_option :Ruby, type: :string, aliases: "-R", required: false,
                         enum: Tebako::CliRubies::RUBY_VERSIONS.keys,
                         desc: "Ruby version to clean, all available versions by default"
    def clean_ruby
      puts "Cleaning Ruby sources from tebako packaging environment"
      suffix = options["Ruby"].nil? ? "" : "_#{options["Ruby"]}"
      nmr = "src/_ruby_#{suffix}*"
      nms = "stash_#{suffix}"
      FileUtils.rm_rf(Dir.glob(File.join(deps, nmr)), secure: true)
      FileUtils.rm_rf(Dir.glob(File.join(deps, nms)), secure: true)
    end

    desc "hash", "Print build script hash (ci cache key)"
    def hash
      print Digest::SHA256.hexdigest [File.read(File.join(source, "CMakeLists.txt")), Tebako::VERSION].join
    end

    desc "press", "Press tebako image"
    method_option :"entry-point", type: :string, aliases: ["-e", "--entry"], required: true,
                                  desc: "Ruby application entry point"
    method_option :"log-level", type: :string, aliases: "-l", required: false, enum: %w[error warn debug trace],
                                desc: "Tebako memfs logging level, 'error' by default"
    method_option :output, type: :string, aliases: "-o", required: false,
                           desc: "Tebako package file name, entry point base file name in the current folder by default"
    method_option :root, type: :string, aliases: "-r", required: true, desc: "Root folder of the Ruby application"
    method_option :Ruby, type: :string, aliases: "-R", required: false,
                         enum: Tebako::CliRubies::RUBY_VERSIONS.keys,
                         desc: "Tebako package Ruby version, #{Tebako::CliRubies::DEFAULT_RUBY_VERSION} by default"
    def press
      version_cache_check unless options[:devmode]

      puts press_announce
      do_press
      ensure_version_file
    rescue Tebako::Error => e
      puts "Tebako script failed: #{e.message} [#{e.error_code}]"
      exit e.error_code
    end

    desc "setup", "Set up tebako packaging environment"
    method_option :Ruby, type: :string, aliases: "-R", required: false,
                         enum: Tebako::CliRubies::RUBY_VERSIONS.keys,
                         desc: "Tebako package Ruby version, #{Tebako::CliRubies::DEFAULT_RUBY_VERSION} by default."
    def setup
      version_cache_check unless options[:devmode]

      puts "Setting up tebako packaging environment"
      do_setup
      ensure_version_file
    rescue Tebako::Error => e
      puts "Tebako script failed: #{e.message} [#{e.error_code}]"
      exit e.error_code
    end

    def self.exit_on_failure?
      true
    end

    no_commands do
      def initialize(*args)
        super
        return if args[2][:current_command].name.include?("hash")

        puts "Tebako executable packager version #{Tebako::VERSION}"
      end

      def options
        original_options = super

        return original_options unless File.exist?(OPTIONS_FILE)

        defaults = ::YAML.load_file(OPTIONS_FILE) || {}
        Thor::CoreExt::HashWithIndifferentAccess.new(defaults.merge(original_options))
      end
    end

    private

    no_commands do
      def do_press
        cfg_cmd = "cmake -DSETUP_MODE:BOOLEAN=OFF #{cfg_options} #{press_options}"
        build_cmd = "cmake --build #{output} --target tebako --parallel #{Etc.nprocessors}"
        merged_env = ENV.to_h.merge(b_env)
        Tebako.packaging_error(103) unless system(merged_env, cfg_cmd)
        Tebako.packaging_error(104) unless system(merged_env, build_cmd)
      end

      def do_setup
        cfg_cmd = "cmake -DSETUP_MODE:BOOLEAN=ON #{cfg_options}"
        build_cmd = "cmake --build \"#{output}\" --target setup --parallel #{Etc.nprocessors}"
        merged_env = ENV.to_h.merge(b_env)
        Tebako.packaging_error(101) unless system(merged_env, cfg_cmd)
        Tebako.packaging_error(102) unless system(merged_env, build_cmd)
      end
    end

    include Tebako::CliHelpers
    include Tebako::CliRubies
  end
end
