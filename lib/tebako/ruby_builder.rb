# frozen_string_literal: true

# Copyright (c) 2024-2025 [Ribose Inc](https://www.ribose.com).
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

require "fileutils"
require "find"

require_relative "build_helpers"

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (ruby builder)
  class RubyBuilder
    def initialize(ruby_ver, src_dir)
      @ruby_ver = ruby_ver
      @src_dir = src_dir
      @ncores = ScenarioManagerBase.new.ncores
    end

    # Final build of tebako package
    def toolchain_build
      puts "   ... building toolchain Ruby"
      Dir.chdir(@src_dir) do
        BuildHelpers.with_env({ "TEBAKO_PASS_THROUGH" => "1" }) do
          BuildHelpers.run_with_capture(["make", "-j#{@ncores}"])
          BuildHelpers.run_with_capture(["make", "install", "-j#{@ncores}"])
        end
      end
    end

    # Final build of tebako package
    def target_build(output_type)
      puts "   ... building tebako #{output_type}"
      Dir.chdir(@src_dir) do
        BuildHelpers.with_env({ "TEBAKO_PASS_THROUGH" => "1" }) do
          BuildHelpers.run_with_capture(["make", "ruby", "-j#{@ncores}"]) if @ruby_ver.ruby3x?
          BuildHelpers.run_with_capture(["make", "-j#{@ncores}"])
        end
      end
    end
  end
end
