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

require "pathname"

require_relative "options_manager"
require_relative "scenario_manager"
require_relative "package_descriptor"
require_relative "packager"

module Tebako
  # Tebako application package descriptor
  class PackagerLite
    def initialize(options_manager, scenario_manager)
      @opts = options_manager
      @scm = scenario_manager
      @scm.configure_scenario
    end

    def codegen
      puts "-- Generating files"
      Tebako::Codegen.generate_package_descriptor(@opts, @scm)
    end

    def create_package
      Tebako::Packager.init(@opts.stash_dir, @opts.data_src_dir, @opts.data_pre_dir, @opts.data_bin_dir)
      Tebako::Packager.deploy(@opts.data_src_dir, @opts.data_pre_dir, @opts.rv,
                              @opts.root, @scm.fs_entrance, @opts.cwd)
      Tebako::Packager.mkdwarfs(@opts.deps_bin_dir, name, @opts.data_src_dir, codegen)
    end

    def name
      bname = Pathname.new(@opts.package).cleanpath.to_s
      "#{bname}.tebako"
    end
  end
end
