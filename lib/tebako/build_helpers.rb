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

require "fileutils"
require "open3"

# Tebako - an executable packager
module Tebako
  # Ruby build helpers
  module BuildHelpers
    # Environment bindings that couple a spawned process to the press host's
    # own Ruby/bundler setup. Runtime spawns (layout extraction, the deploy
    # driver) boot the prebuilt runtime's embedded Ruby, which must resolve
    # gems against the memfs and the deploy GEM_HOME alone: an inherited
    # bundler context (a 'bundle exec'-driven press) makes that Ruby boot
    # into the press's own bundle -- crashing with Bundler::GemNotFound at
    # best, silently deploying the wrong gem set at worst. Scrubbed at this
    # single spawn choke point (nil values unset the variable in the child).
    RUBY_ENV_SCRUB = %w[RUBYOPT RUBYLIB].freeze
    RUBY_ENV_SCRUB_PREFIXES = %w[BUNDLE_ BUNDLER_].freeze

    class << self
      # rm_rf + mkdir_p: in the prebuilt press flows no cmake configure runs,
      # so the parent output folder may not exist yet
      def recreate(dirname)
        FileUtils.rm_rf(dirname, noop: nil, verbose: nil, secure: true)
        FileUtils.mkdir_p(dirname)
      end

      def run_with_capture(args)
        args = args.compact
        puts "   ... @ #{args.join(" ")}"
        out, st = Open3.capture2e(ruby_env_scrub, *args)
        raise Tebako::Error, "Failed to run #{args.join(" ")} (#{st}):\n #{out}" if st.signaled? || !st.exitstatus.zero?

        out
      end

      # The scrub environment for spawned processes: RUBYOPT/RUBYLIB plus
      # every BUNDLE_*/BUNDLER_* variable currently set, all mapped to nil
      def ruby_env_scrub
        ENV.each_key.with_object(RUBY_ENV_SCRUB.to_h { |key| [key, nil] }) do |key, scrub|
          scrub[key] = nil if RUBY_ENV_SCRUB_PREFIXES.any? { |prefix| key.start_with?(prefix) }
        end
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
