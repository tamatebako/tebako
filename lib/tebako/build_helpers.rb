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

require "open3"

# Tebako - an executable packager
module Tebako
  # Ruby build helpers
  module BuildHelpers
    class << self
      def ncores
        if RUBY_PLATFORM.include?("darwin")
          out, st = Open3.capture2e("sysctl", "-n", "hw.ncpu")
        else
          out, st = Open3.capture2e("nproc", "--all")
        end

        if !st.signaled? && st.exitstatus.zero?
          out.strip.to_i
        else
          4
        end
      end

      def run_with_capture(args)
        puts "   ... @ #{args.join(" ")}"
        out, st = Open3.capture2e(*args)
        raise Tebako::Error, "Failed to run #{args.join(" ")} (#{st}):\n #{out}" if st.signaled? || !st.exitstatus.zero?

        out
      end

      def run_with_capture_v(args)
        if @verbose
          args_v = args.dup
          args_v.push("--verbose")
          puts run_with_capture(args_v)
        else
          run_with_capture(args)
        end
      end

      # Sets up temporary environment variables and yields to the
      # block. When the block exits, the environment variables are set
      # back to their original values.
      def with_env(hash)
        old = {}
        hash.each do |k, v|
          old[k] = ENV.fetch(k, nil)
          ENV[k] = v
        end
        begin
          yield
        ensure
          hash.each_key { |k| ENV[k] = old[k] }
        end
      end
    end
  end
end
