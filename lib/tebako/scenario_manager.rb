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

require "pathname"

require_relative "error"

# Tebako - an executable packager
module Tebako
  # Manages packaging scenario based on input files (gemfile, gemspec, etc)
  class ScenarioManager
    def initialize(fs_root, fs_entrance)
      initialize_root(fs_root)
      initialize_entry_point(fs_entrance || "stub.rb")
    end

    attr_reader :fs_entry_point, :fs_mount_point, :fs_entrance

    def configure_scenario
      @fs_mount_point = if msys?
                          "A:/__tebako_memfs__"
                        else
                          "/__tebako_memfs__"
                        end

      lookup_files
      configure_scenario_inner
    end

    def exe_suffix
      @exe_suffix ||= msys? ? ".exe" : ""
    end

    def macos?
      @macos ||= RUBY_PLATFORM =~ /darwin/ ? true : false
    end

    def msys?
      @msys ||= RUBY_PLATFORM =~ /msys|mingw|cygwin/ ? true : false
    end

    private

    def initialize_entry_point(fs_entrance)
      @fs_entrance = Pathname.new(fs_entrance).cleanpath.to_s

      if Pathname.new(@fs_entrance).absolute?
        Tebako.packaging_error 114 unless @fs_entrance.start_with?(@fs_root)

        fetmp = @fs_entrance
        @fs_entrance = Pathname.new(@fs_entrance).relative_path_from(Pathname.new(@fs_root)).to_s
        puts "-- Absolute path to entry point '#{fetmp}' will be reduced to '#{@fs_entrance}' relative to '#{@fs_root}'"
      end
      # Can check after deploy, because entry point can be generated during bundle install or gem install
      # Tebako.packaging_error 106 unless File.file?(File.join(@fs_root, @fs_entrance))
      @fs_entry_point = "/bin/#{@fs_entrance}"
    end

    def initialize_root(fs_root)
      Tebako.packaging_error 107 unless Dir.exist?(fs_root)
      p_root = Pathname.new(fs_root).cleanpath
      Tebako.packaging_error 113 unless p_root.absolute?
      @fs_root = p_root.realpath.to_s
    end

    def configure_scenario_inner
      case @gs_length
      when 0
        configure_scenario_no_gemspec
      when 1
        @scenario = @gf_length.positive? ? :gemspec_and_gemfile : :gemspec
      else
        raise Tebako::Error, "Multiple Ruby gemspecs found in #{@fs_root}"
      end
    end

    def configure_scenario_no_gemspec
      @fs_entry_point = "/local/#{@fs_entrance}" if @gf_length.positive? || @g_length.zero?

      @scenario = if @gf_length.positive?
                    :gemfile
                  elsif @g_length.positive?
                    :gem
                  else
                    :simple_script
                  end
    end

    def lookup_files
      @gs_length = Dir.glob(File.join(@fs_root, "*.gemspec")).length
      @gf_length = Dir.glob(File.join(@fs_root, "Gemfile")).length
      @g_length = Dir.glob(File.join(@fs_root, "*.gem")).length
    end
  end
end
