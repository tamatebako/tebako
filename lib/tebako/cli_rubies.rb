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
      "3.1.6" => "0d0dafb859e76763432571a3109d1537d976266be3083445651dc68deed25c22",
      "3.2.4" => "c72b3c5c30482dca18b0f868c9075f3f47d8168eaf626d4e682ce5b59c858692",
      "3.2.5" => "ef0610b498f60fb5cfd77b51adb3c10f4ca8ed9a17cb87c61e5bea314ac34a16",
      "3.3.3" => "83c05b2177ee9c335b631b29b8c077b4770166d02fa527f3a9f6a40d13f3cce2",
      "3.3.4" => "fe6a30f97d54e029768f2ddf4923699c416cdbc3a6e96db3e2d5716c7db96a34",
      "3.3.5" => "3781a3504222c2f26cb4b9eb9c1a12dbf4944d366ce24a9ff8cf99ecbce75196"
    }.freeze

    MIN_RUBY_VERSION_WINDOWS = "3.1.6"
    DEFAULT_RUBY_VERSION = "3.2.5"

    def version_check(version)
      return if RUBY_VERSIONS.key?(version)

      raise Tebako::Error.new(
        "Ruby version #{version} is not supported, exiting",
        253
      )
    end

    def version_check_msys(version)
      if Gem::Version.new(version) < Gem::Version.new(MIN_RUBY_VERSION_WINDOWS) && RUBY_PLATFORM =~ /msys|mingw|cygwin/
        raise Tebako::Error.new(
          "Windows packaging works for Ruby #{MIN_RUBY_VERSION_WINDOWS} or above, version #{version} is not supported",
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
