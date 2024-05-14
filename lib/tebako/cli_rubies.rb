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

require "fileutils"
require "pathname"
require "rbconfig"

require_relative "error"
require_relative "version"

# Tebako - an executable packager
# Command-line interface methods
module Tebako
  # Ruby version helpers
  module CliRubies
    RUBY_VERSIONS = {
      "2.7.8" => "c2dab63cbc8f2a05526108ad419efa63a67ed4074dbbcf9fc2b1ca664cb45ba0",
      "3.0.7" => "2a3411977f2850431136b0fab8ad53af09fb74df2ee2f4fb7f11b378fe034388",
      "3.1.4" => "a3d55879a0dfab1d7141fdf10d22a07dbf8e5cdc4415da1bde06127d5cc3c7b6",
      "3.1.5" => "3685c51eeee1352c31ea039706d71976f53d00ab6d77312de6aa1abaf5cda2c5",
      "3.2.3" => "af7f1757d9ddb630345988139211f1fd570ff5ba830def1cc7c468ae9b65c9ba",
      "3.2.4" => "c72b3c5c30482dca18b0f868c9075f3f47d8168eaf626d4e682ce5b59c858692"
    }.freeze

    DEFAULT_RUBY_VERSION = "3.1.5"

    def version_check(version)
      return if RUBY_VERSIONS.key?(version)

      raise Tebako::Error.new(
        "Ruby version #{version} is not supported yet, exiting",
        253
      )
    end

    def version_check_msys(version)
      if Gem::Version.new(version) < Gem::Version.new(DEFAULT_RUBY_VERSION) && RUBY_PLATFORM =~ /msys|mingw|cygwin/
        raise Tebako::Error.new(
          "Windows packaging works for Ruby #{DEFAULT_RUBY_VERSION} or above, version #{version} is not supported",
          252
        )
      end
    end

    def extend_ruby_version
      version = options["Ruby"].nil? ? DEFAULT_RUBY_VERSION : options["Ruby"]
      version_check(version)
      version_check_msys(version)
      @extend_ruby_version ||= [version, RUBY_VERSIONS[version]]
    end
  end
end
