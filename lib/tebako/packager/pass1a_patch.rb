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

require_relative "patch"
# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching definitions (pass1a)
    class Pass1APatch < Patch
      GEM_PRELUDE_RB_PATCH = {
        "if defined?(DidYouMean)" => <<~SUBST
          if defined?(DidYouMean)

          # -- Start of tebako patch --
          if ENV.fetch("TEBAKO_PASS_THROUGH", nil).nil?
            begin
              puts "ENV[TEBAKO_PASS_THROUGH] = " + ENV.fetch("TEBAKO_PASS_THROUGH", nil).to_s
              require 'tebako-runtime'
            rescue LoadError
              warn "'tebako-runtime' was not loaded."
              gets
            end
          end
          # -- End of tebako patch --
        SUBST
      }.freeze

      def patch_map
        {
          "gem_prelude.rb" => GEM_PRELUDE_RB_PATCH
        }.freeze
      end
    end
  end
end
