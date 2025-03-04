# frozen_string_literal: true

# Copyright (c) 2025 [Ribose Inc](https://www.ribose.com).
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

require "thor"
require_relative "packager"
require_relative "ruby_version"
require_relative "stripper"
require_relative "build_helpers"

module Tebako
  # Thor-based command set for Ruby patching operations
  class PatchCommands < Thor
    desc "pass1", "Run pass1 Ruby patching operations"
    method_option :ostype, type: :string, required: true, desc: "Operating system type"
    method_option :ruby_source_dir, type: :string, required: true, desc: "Ruby source directory"
    method_option :fs_mount_point, type: :string, required: true, desc: "Filesystem mount point"
    method_option :data_src_dir, type: :string, required: true, desc: "Data source directory"
    method_option :ruby_ver, type: :string, required: true, desc: "Ruby version string"

    # Method to run pass1 operations
    def pass1
      ostype = options[:ostype]
      ruby_source_dir = options[:ruby_source_dir]
      mount_point = options[:fs_mount_point]
      data_src_dir = options[:data_src_dir]
      ruby_ver_str = options[:ruby_ver]

      ruby_ver = Tebako::RubyVersion.new(ruby_ver_str)
      Tebako::Packager.pass1(ostype, ruby_source_dir, mount_point, data_src_dir, ruby_ver)
    end

    desc "pass2", "Run pass2 Ruby patching operations"
    method_option :ostype, type: :string, required: true, desc: "Operating system type"
    method_option :ruby_source_dir, type: :string, required: true, desc: "Ruby source directory"
    method_option :deps_lib_dir, type: :string, required: true, desc: "Dependencies library directory"
    method_option :data_src_dir, type: :string, required: true, desc: "Data source directory"
    method_option :ruby_stash_dir, type: :string, required: true, desc: "Ruby stash directory"
    method_option :ruby_ver, type: :string, required: true, desc: "Ruby version string"

    # Method to run pass2 operations
    def pass2 # rubocop:disable Metrics/AbcSize
      ostype = options[:ostype]
      ruby_source_dir = options[:ruby_source_dir]
      deps_lib_dir = options[:deps_lib_dir]
      data_src_dir = options[:data_src_dir]
      ruby_stash_dir = options[:ruby_stash_dir]
      ruby_ver_str = options[:ruby_ver]

      ruby_ver = Tebako::RubyVersion.new(ruby_ver_str)
      Tebako::Packager.pass1a(ruby_source_dir)
      Tebako::Packager.stash(data_src_dir, ruby_stash_dir, ruby_source_dir, ruby_ver)
      Tebako::Packager.pass2(ostype, ruby_source_dir, deps_lib_dir, ruby_ver)
    end
  end
end
