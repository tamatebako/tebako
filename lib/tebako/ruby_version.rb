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
      "3.2.7" => "8488fa620ff0333c16d437f2b890bba3b67f8745fdecb1472568a6114aad9741",
      "3.3.3" => "83c05b2177ee9c335b631b29b8c077b4770166d02fa527f3a9f6a40d13f3cce2",
      "3.3.4" => "fe6a30f97d54e029768f2ddf4923699c416cdbc3a6e96db3e2d5716c7db96a34",
      "3.3.5" => "3781a3504222c2f26cb4b9eb9c1a12dbf4944d366ce24a9ff8cf99ecbce75196",
      "3.3.6" => "8dc48fffaf270f86f1019053f28e51e4da4cce32a36760a0603a9aee67d7fd8d",
      "3.3.7" => "9c37c3b12288c7aec20ca121ce76845be5bb5d77662a24919651aaf1d12c8628",
      "3.4.1" => "3d385e5d22d368b064c817a13ed8e3cc3f71a7705d7ed1bae78013c33aa7c87f",
      "3.4.2" => "41328ac21f2bfdd7de6b3565ef4f0dd7543354d37e96f157a1552a6bd0eb364b",
      "4.0.0" => "2e8389c8c072cb658c93a1372732d9eac84082c88b065750db1e52a5ac630271",
      "4.0.1" => "3924be2d05db30f4e35f859bf028be85f4b7dd01714142fd823e4af5de2faf9d",
      "4.0.2" => "51502b26b50b68df4963336ca41e368cde92c928faf91654de4c4c1791f82aac",
      "4.0.3" => "77964acc370d5c8375b9502e5ba6c13c03ef91ab9eb9f521c84fb42b9c9a6b0f",
      "4.0.4" => "f35f6edfa3dabb3f723f9d0cf1906c6512ae77f4e412ab1e68cc6e91d230fa80",
      "4.0.5" => "7d6149079a63f8ae1d326c9fa65c6019ba2dc3155eae7b39159817911c88958e",
      "4.0.6" => "837d299e8f7ddf2be31a229a7a7e019d354979825117989acb3b32b1a9be262a"
    }.freeze

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

    def extend_ruby_version
      @extend_ruby_version ||= [@ruby_version, RUBY_VERSIONS[@ruby_version]]
    end

    def lib_version
      @lib_version ||= "#{@ruby_version.split(".")[0..1].join}0"
    end

    # Version gates compare numerically so 4.x lines fall out naturally
    # (string indexing broke the moment the major version hit 4)
    def ruby3x?
      @ruby3x ||= version_at_least?(3, 0)
    end

    def ruby31?
      @ruby31 ||= version_at_least?(3, 1)
    end

    def ruby32?
      @ruby32 ||= version_at_least?(3, 2)
    end

    def ruby32only?
      @ruby32only ||= major_minor == [3, 2]
    end

    def ruby33?
      @ruby33 ||= version_at_least?(3, 3)
    end

    def ruby33only?
      @ruby33only ||= major_minor == [3, 3]
    end

    def ruby3x7?
      @ruby3x7 ||= ruby34? ||
                   (ruby33only? && patch_version >= 7) ||
                   (ruby32only? && patch_version >= 7)
    end

    def ruby34?
      @ruby34 ||= version_at_least?(3, 4)
    end

    def run_checks
      version_check_format
      version_check
      version_check_msys
    end

    private

    def major_minor
      @major_minor ||= @ruby_version.split(".").first(2).map(&:to_i)
    end

    def patch_version
      @patch_version ||= @ruby_version.split(".")[2].to_i
    end

    def version_at_least?(major, minor)
      (major_minor <=> [major, minor]) >= 0
    end

    public

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
