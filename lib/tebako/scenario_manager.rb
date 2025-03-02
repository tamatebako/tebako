# frozen_string_literal: true

# Copyright (c) 2024-2025 [Ribose Inc](https://www.ribose.com).
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
require "bundler"

require_relative "error"

# Tebako - an executable packager
module Tebako
  # Magic version numbers used to ensure compatibility for Ruby 2.7.x, 3.0.x
  # These are the minimal versions required to provide linux-gnu / linux-musl differentiation by bundler
  # Ruby 3.1+ default rubygems versions work correctly out of the box
  BUNDLER_VERSION = "2.4.22"
  RUBYGEMS_VERSION = "3.4.22"

  # A couple of static Scenario definitions
  class ScenarioManagerBase
    def initialize(ostype = RUBY_PLATFORM)
      @ostype = ostype
      @linux = @ostype =~ /linux/ ? true : false
      @musl = @ostype =~ /linux-musl/ ? true : false
      @macos = @ostype =~ /darwin/ ? true : false
      @msys  = @ostype =~ /msys|mingw|cygwin/ ? true : false

      @fs_mount_point = @msys ? "A:/__tebako_memfs__" : "/__tebako_memfs__"
      @exe_suffix = @msys ? ".exe" : ""
    end

    attr_reader :fs_mount_point, :exe_suffix

    def b_env
      u_flags = if @macos
                  "-DTARGET_OS_SIMULATOR=0 -DTARGET_OS_IPHONE=0  #{ENV.fetch("CXXFLAGS", nil)}"
                else
                  ENV.fetch("CXXFLAGS", nil)
                end
      @b_env ||= { "CXXFLAGS" => u_flags }
    end

    def linux?
      @linux
    end

    def linux_gnu?
      @linux && !@musl
    end

    def linux_musl?
      @linux && @musl
    end

    def m_files
      # [TODO]
      # Ninja generates incorrect script for tebako press target -- gets lost in a chain custom targets
      # Using makefiles has negative performance impact so it needs to be fixed

      @m_files ||= if @linux || @macos
                     "Unix Makefiles"
                   elsif @msys
                     "MinGW Makefiles"
                   else
                     raise Tebako::Error.new("#{RUBY_PLATFORM} is not supported.", 112)
                   end
    end

    def macos?
      @macos
    end

    def msys?
      @msys
    end

    def musl?
      @musl
    end

    def ncores
      if @ncores.nil?
        if @macos
          out, st = Open3.capture2e("sysctl", "-n", "hw.ncpu")
        else
          out, st = Open3.capture2e("nproc", "--all")
        end

        @ncores = !st.signaled? && st.exitstatus.zero? ? out.strip.to_i : 4
      end
      @ncores
    end
  end

  # Manages packaging scenario based on input files (gemfile, gemspec, etc)
  class ScenarioManager < ScenarioManagerBase
    def initialize(fs_root, fs_entrance)
      super()
      @with_gemfile = @with_lockfile = @needs_bundler = false
      @bundler_version = BUNDLER_VERSION
      initialize_root(fs_root)
      initialize_entry_point(fs_entrance || "stub.rb")
    end

    attr_reader :fs_entry_point, :fs_entrance, :gemfile_path, :needs_bundler, :with_gemfile

    def bundler_reference
      @needs_bundler ? "_#{@bundler_version}_" : nil
    end

    def configure_scenario
      lookup_files

      case @gs_length
      when 0
        configure_scenario_no_gemspec
      when 1
        @scenario = @with_gemfile ? :gemspec_and_gemfile : :gemspec
      else
        raise Tebako::Error, "Multiple Ruby gemspecs found in #{@fs_root}"
      end
    end

    private

    def configure_scenario_no_gemspec
      @fs_entry_point = "/local/#{@fs_entrance}" if @with_gemfile || @g_length.zero?

      @scenario = if @with_gemfile
                    :gemfile
                  elsif @g_length.positive?
                    :gem
                  else
                    :simple_script
                  end
    end

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

    def lookup_files
      @gs_length = Dir.glob(File.join(@fs_root, "*.gemspec")).length
      @g_length = Dir.glob(File.join(@fs_root, "*.gem")).length
      @with_gemfile = File.exist?(@gemfile_path = File.join(@fs_root, "Gemfile"))
      @with_lockfile = File.exist?(@lockfile_path = File.join(@fs_root, "Gemfile.lock"))
    end
  end

  # Configure scenraio and do bundler resolution
  class ScenarioManagerWithBundler < ScenarioManager
    protected

    def lookup_files
      super
      if @with_lockfile
        update_bundler_version_from_lockfile(@lockfile_path)
      elsif @with_gemfile
        update_bundler_version_from_gemfile(@gemfile_path)
      end
    end

    private

    def store_compatible_bundler_version(requirement)
      fetcher = Gem::SpecFetcher.fetcher
      tuples = fetcher.detect(:released) do |name_tuple|
        name_tuple.name == "bundler" && requirement.satisfied_by?(name_tuple.version)
      end

      Tebako.packaging_error 119 if tuples.empty?

      # Get latest compatible version
      @bundler_version = tuples.map { |tuple, _| tuple.version }.max.to_s
    end

    def update_bundler_version_from_gemfile(gemfile_path)
      # Build definition without lockfile
      definition = Bundler::Definition.build(gemfile_path, nil, nil)

      # Get bundler dependency from Gemfile
      bundler_dep = definition.dependencies.find { |d| d.name == "bundler" }

      return unless bundler_dep

      @needs_bundler = true
      min_requirement = Gem::Requirement.create(">= #{Tebako::BUNDLER_VERSION}")
      requirement = Gem::Requirement.create(bundler_dep.requirement, min_requirement)

      store_compatible_bundler_version(requirement)
    end

    def update_bundler_version_from_lockfile(lockfile_path)
      puts "   ... using lockfile at #{@lockfile_path}"
      Tebako.packaging_error 117 unless File.exist?(lockfile_path)

      lockfile_content = File.read(lockfile_path)
      Tebako.packaging_error 117 unless lockfile_content =~ /BUNDLED WITH\n\s+(#{Gem::Version::VERSION_PATTERN})\n/

      @bundler_version = ::Regexp.last_match(1)
      @needs_bundler = true

      bundler_requirement = Gem::Requirement.new(">= #{BUNDLER_VERSION}")
      return if bundler_requirement.satisfied_by?(Gem::Version.new(@bundler_version))

      Tebako.packaging_error 118, " : #{@bundler_version} requested, #{BUNDLER_VERSION} minimum required"
    end
  end
end
