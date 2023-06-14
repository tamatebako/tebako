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

require "fileutils"
require "thor"
require "yaml"

require_relative "cli_helpers"
require_relative "error"
require_relative "version"

# Tebako - an executable packager
# Implementation of tebako command-line interface
module Tebako
  OPTIONS_FILE = ".tebako.yml"
  # Tebako packager front-end
  class TebakoCli < Thor
    package_name "Tebako"
    class_option :prefix, type: :string, aliases: "-p", required: false,
                          desc: "A path to tebako packaging environment, '~/.tebako' ('$HOME/.tebako') by default"

    desc "clean", "Clean tebako packaging environment"
    def clean
      puts "Cleaning tebako packaging environment"
      FileUtils.rm_rf([deps, output], secure: true)
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
                         enum: Tebako::CliHelpers::RUBY_VERSIONS.keys,
                         desc: "Tebako package Ruby version, #{Tebako::CliHelpers::DEFAULT_RUBY_VERSION} by default"
    def press
      puts press_announce
      do_press
    rescue TebakoError => e
      puts "Tebako script failed: #{e.message} [#{e.error_code}]"
      exit e.error_code
    end

    desc "setup", "Set up tebako packaging environment"
    method_option :Ruby, type: :string, aliases: "-R", required: false,
                         enum: Tebako::CliHelpers::RUBY_VERSIONS.keys,
                         desc: "Tebako package Ruby version, #{Tebako::CliHelpers::DEFAULT_RUBY_VERSION} by default"
    def setup
      puts "Setting up tebako packaging environment"
      do_setup
    rescue TebakoError => e
      puts "Tebako script failed: #{e.message} [#{e.error_code}]"
      exit e.error_code
    end

    desc "nosetup", "Skip setup, may be used as gem install subcommand like gem install tebako -- nosetup"
    def nosetup
      puts "Skipping set up of tebako packaging environment"
    end

    def self.exit_on_failure?
      true
    end

    no_commands do
      def options
        original_options = super

        return original_options unless File.exist?(OPTIONS_FILE)

        defaults = ::YAML.load_file(OPTIONS_FILE) || {}
        puts defaults.merge(original_options)
        Thor::CoreExt::HashWithIndifferentAccess.new(defaults.merge(original_options))
      end

      def generate
        FileUtils.mkdir_p(prefix)
        File.write(File.join(prefix, "version.txt"), "#{Tebako::VERSION}\n")
        puts("all: ")
      end
    end

    private

    no_commands do
      def do_press
        packaging_error(103) unless system(b_env, "cmake -DSETUP_MODE:BOOLEAN=OFF #{cfg_options} #{press_options}")
        packaging_error(104) unless system(b_env,
                                           "cmake --build #{output} --target tebako --parallel #{Etc.nprocessors}")
      end

      def do_setup
        FileUtils.mkdir_p(prefix)
        File.write(File.join(prefix, "version.txt"), "#{Tebako::VERSION}\n")
        packaging_error(101) unless system(b_env, "cmake -DSETUP_MODE:BOOLEAN=ON #{cfg_options}")
        packaging_error(102) unless system(b_env,
                                           "cmake --build #{output} --target setup --parallel #{Etc.nprocessors}")
      end

      def press_announce
        @press_announce ||= <<~ANN
          Running tebako press at #{prefix}
             Ruby version:            '#{extend_ruby_version[0]}'
             Project root:            '#{options["root"]}'
             Application entry point: '#{options["entry-point"]}'
             Package file name:       '#{package}'
             Loging level:            '#{l_level}'
        ANN
      end
    end

    include Tebako::CliHelpers
  end
end
