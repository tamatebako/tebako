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

require_relative "patch"
# Tebako - an executable packager
module Tebako
  module Packager
    # Shared accross Pass1Patch and RybugemsUpdatePatch
    class RubygemsPatch < Patch
      RUBYGEMS_OPENSSL_RB_SUBST = <<~SUBST
        # Start of tebako patch
        require "openssl"
        # End of tebako patch
        autoload :OpenSSL, "openssl"
      SUBST

      RUBYGEMS_OPENSSL_RB_PATCH = {
        'autoload :OpenSSL, "openssl"' => RUBYGEMS_OPENSSL_RB_SUBST
      }.freeze

      def initialize(mount_point)
        super()
        @mount_point = mount_point
      end

      protected

      def rubygems_path_support_patch(mount_point)
        patch = <<~SUBST
          # -- Start of tebako patch --
              @home = Gem.default_dir unless @home.index("#{mount_point}") == 0 unless env["TEBAKO_PASS_THROUGH"]
          # -- End of tebako patch --
              @path = split_gem_path env["GEM_PATH"], @home
          # -- Start of tebako patch --
              @path.keep_if do |xpath|
                xpath.index("#{mount_point}") == 0
              end unless env["TEBAKO_PASS_THROUGH"]
          # -- End of tebako patch --
        SUBST

        {
          '  @path = split_gem_path env["GEM_PATH"], @home' => patch
        }
      end
    end

    # Rubygems patch after update
    class RubygemsUpdatePatch < RubygemsPatch
      def patch_map
        pm = {
          "rubygems/openssl.rb" => RUBYGEMS_OPENSSL_RB_PATCH,
          "rubygems/path_support.rb" => rubygems_path_support_patch(@mount_point)
        }
        pm.merge!(super)
        pm.freeze
      end
    end
  end
end
