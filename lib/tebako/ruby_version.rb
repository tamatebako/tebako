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

require "bundler"

# Tebako - an executable packager
module Tebako
  # Ruby version
  class RubyVersion
    # Ruby versions the gem can press packages for; the prebuilt runtime
    # packages published by tebako-runtime-ruby are the operative constraint
    # (resolution fails for versions without a published package)
    RUBY_VERSIONS = %w[
      2.7.8
      3.0.7
      3.1.6
      3.2.4
      3.2.5
      3.2.6
      3.2.7
      3.3.3
      3.3.4
      3.3.5
      3.3.6
      3.3.7
      3.4.1
      3.4.2
      4.0.0
      4.0.1
      4.0.2
      4.0.3
      4.0.4
      4.0.5
      4.0.6
    ].freeze

    MIN_RUBY_VERSION_WINDOWS = "3.1.6"
    DEFAULT_RUBY_VERSION = "3.3.7"

    def initialize(ruby_version)
      @ruby_version = ruby_version.nil? ? DEFAULT_RUBY_VERSION : ruby_version

      run_checks
    end

    attr_reader :ruby_version

    def api_version
      @api_version ||= "#{@ruby_version.split(".")[0..1].join(".")}.0"
    end

    def run_checks
      version_check_format
      version_check
      version_check_msys
    end

    def version_check
      return if RUBY_VERSIONS.include?(@ruby_version)

      raise Tebako::Error.new(
        "Ruby version #{@ruby_version} is not supported",
        110
      )
    end

    def version_check_format
      return if @ruby_version =~ /^\d+\.\d+\.\d+$/

      raise Tebako::Error.new("Invalid Ruby version format '#{@ruby_version}'. Expected format: x.y.z", 109)
    end

    def version_check_msys
      if Gem::Version.new(@ruby_version) < Gem::Version.new(MIN_RUBY_VERSION_WINDOWS) && ScenarioManagerBase.new.msys?
        raise Tebako::Error.new("Ruby version #{@ruby_version} is not supported on Windows", 111)
      end
    end
  end

  # Ruby version with Gemfile definition
  class RubyVersionWithGemfile < RubyVersion
    def initialize(ruby_version, gemfile_path)
      # Assuming that it does not attempt to load any gems or resolve dependencies
      # this can be done with any bundler version
      ruby_v = Bundler::Definition.build(gemfile_path, nil, nil).ruby_version&.versions
      if ruby_v.nil?
        super(ruby_version)
      else
        process_gemfile_ruby_version(ruby_version, ruby_v)
      end
    rescue Tebako::Error
      raise
    rescue StandardError => e
      Tebako.packaging_error(115, e.message)
    end

    def process_gemfile_ruby_version(ruby_version, ruby_v)
      puts "-- Found Gemfile with Ruby requirements #{ruby_v}"
      requirement = Gem::Requirement.new(ruby_v)

      if ruby_version.nil?
        process_gemfile_ruby_version_ud(requirement)
      else
        process_gemfile_ruby_version_d(ruby_version, requirement)
      end
      run_checks
    end

    def process_gemfile_ruby_version_d(ruby_version, requirement)
      current_version = Gem::Version.new(ruby_version)
      unless requirement.satisfied_by?(current_version)
        raise Tebako::Error.new("Ruby version #{ruby_version} does not satisfy requirement '#{requirement}'", 116)
      end

      @ruby_version = ruby_version
    end

    def process_gemfile_ruby_version_ud(requirement)
      available_versions = RUBY_VERSIONS.map { |v| Gem::Version.new(v) }
      matching_version = available_versions.find { |v| requirement.satisfied_by?(v) }
      puts "-- Found matching Ruby version #{matching_version}" if matching_version

      unless matching_version
        raise Tebako::Error.new("No available Ruby version satisfies requirement #{requirement}",
                                116)
      end

      @ruby_version = matching_version.to_s
    end
  end
end
