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

require "bundler"
require_relative "error"

# Tebako - an executable packager
module Tebako
  # Ruby version
  class RubyVersion
    RUBY_VERSIONS = {
      "2.7.8" => "c2dab63cbc8f2a05526108ad419efa63a67ed4074dbbcf9fc2b1ca664cb45ba0",
      "3.0.7" => "2a3411977f2850431136b0fab8ad53af09fb74df2ee2f4fb7f11b378fe034388",
      "3.1.6" => "0d0dafb859e76763432571a3109d1537d976266be3083445651dc68deed25c22",
      "3.2.4" => "c72b3c5c30482dca18b0f868c9075f3f47d8168eaf626d4e682ce5b59c858692",
      "3.2.5" => "ef0610b498f60fb5cfd77b51adb3c10f4ca8ed9a17cb87c61e5bea314ac34a16",
      "3.2.6" => "d9cb65ecdf3f18669639f2638b63379ed6fbb17d93ae4e726d4eb2bf68a48370",
      "3.3.3" => "83c05b2177ee9c335b631b29b8c077b4770166d02fa527f3a9f6a40d13f3cce2",
      "3.3.4" => "fe6a30f97d54e029768f2ddf4923699c416cdbc3a6e96db3e2d5716c7db96a34",
      "3.3.5" => "3781a3504222c2f26cb4b9eb9c1a12dbf4944d366ce24a9ff8cf99ecbce75196",
      "3.3.6" => "8dc48fffaf270f86f1019053f28e51e4da4cce32a36760a0603a9aee67d7fd8d",
      "3.3.7" => "9c37c3b12288c7aec20ca121ce76845be5bb5d77662a24919651aaf1d12c8628",
      "3.4.1" => "3d385e5d22d368b064c817a13ed8e3cc3f71a7705d7ed1bae78013c33aa7c87f"
    }.freeze

    MIN_RUBY_VERSION_WINDOWS = "3.1.6"
    DEFAULT_RUBY_VERSION = "3.2.6"

    def initialize(ruby_version)
      @ruby_version = ruby_version.nil? ? DEFAULT_RUBY_VERSION : ruby_version

      run_checks
    end

    attr_reader :ruby_version

    def api_version
      @api_version ||= "#{@ruby_version.split(".")[0..1].join(".")}.0"
    end

    def extend_ruby_version
      @extend_ruby_version ||= [@ruby_version, RUBY_VERSIONS[@ruby_version]]
    end

    def lib_version
      @lib_version ||= "#{@ruby_version.split(".")[0..1].join}0"
    end

    def ruby3x?
      @ruby3x ||= @ruby_version[0] == "3"
    end

    def ruby31?
      @ruby31 ||= ruby3x? && @ruby_version[2].to_i >= 1
    end

    def ruby32?
      @ruby32 ||= ruby3x? && @ruby_version[2].to_i >= 2
    end

    def ruby32only?
      @ruby32only ||= ruby3x? && @ruby_version[2] == "2"
    end

    def ruby33?
      @ruby33 ||= ruby3x? && @ruby_version[2].to_i >= 3
    end

    def ruby337?
      @ruby337 ||= ruby34? || (ruby33? && @ruby_version[2] == "3" && @ruby_version[4].to_i >= 7)
    end

    def ruby34?
      @ruby34 ||= ruby3x? && @ruby_version[2].to_i >= 4
    end

    def run_checks
      version_check_format
      version_check
      version_check_msys
    end

    def version_check
      return if RUBY_VERSIONS.key?(@ruby_version)

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
      if Gem::Version.new(@ruby_version) < Gem::Version.new(MIN_RUBY_VERSION_WINDOWS) &&
         RUBY_PLATFORM =~ /msys|mingw|cygwin/
        raise Tebako::Error.new("Ruby version #{@ruby_version} is not supported on Windows", 111)
      end
    end
  end

  # Ruby version with Gemfile definition
  class RubyVersionWithGemfile < RubyVersion
    def initialize(ruby_version, gemfile_path)
      # Assuming that it does not attempt to load any gems or resolve dependencies
      # this can be done with any bundler version
      gemfile = Bundler::Definition.build(gemfile_path, nil, nil)
      ruby_v = gemfile.ruby_version&.versions
      if ruby_v.nil?
        super(ruby_version)
      else
        process_gemfile_ruby_version(ruby_version, ruby_v)
        run_checks
      end
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
    end

    def process_gemfile_ruby_version_d(ruby_version, requirement)
      current_version = Gem::Version.new(ruby_version)
      unless requirement.satisfied_by?(current_version)
        raise Tebako::Error.new("Ruby version #{ruby_version} does not satisfy requirement #{ruby_v}", 116)
      end

      @ruby_version = ruby_version
    end

    def process_gemfile_ruby_version_ud(requirement)
      available_versions = RUBY_VERSIONS.keys.map { |v| Gem::Version.new(v) }
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
