# frozen_string_literal: true

# Copyright (c) 2023 [Ribose Inc](https://www.ribose.com).
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
require_relative "patch_literals"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby patching helpers (pass2)
    module PatchHelpers
      class << self
        def get_prefix(package)
          out, st = Open3.capture2("brew --prefix #{package}")
          raise Tebako::Error, "brew --prefix #{package} failed with code #{st.exitstatus}" unless st.exitstatus.zero?

          out
        end

        def ruby3x?(ruby_ver)
          ruby_ver[0] == "3"
        end

        def ruby31?(ruby_ver)
          ruby3x?(ruby_ver) && ruby_ver[2].to_i >= 1
        end

        def ruby32?(ruby_ver)
          ruby3x?(ruby_ver) && ruby_ver[2].to_i >= 2
        end

        def yaml_reference(ruby_ver)
          ruby32?(ruby_ver) ? "-l:libyaml.a" : ""
        end
      end
    end
  end
end
