# frozen_string_literal: true

# Copyright (c) 2021-2024 [Ribose Inc](https://www.ribose.com).
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

require_relative "error"
require_relative "packager/pass1"
require_relative "packager/pass2"
require_relative "packager/patch_helpers"

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (internal)
  module Packager
    FILES_TO_RESTORE = %w[
      main.c
      dir.c
      dln.c
      file.c
      io.c
      tool/mkconfig.rb
      gem_prelude.rb
    ].freeze

    FILES_TO_RESTORE_MSYS = %w[
      ruby.c
      win32/win32.c
      win32/file.c
      win32/dir.h
    ].freeze

    FILES_TO_RESTORE_MUSL = %w[
      thread_pthread.c
    ].freeze

    DEPLOY_ENV = {
      "GEM_HOME" => nil,
      "GEM_PATH" => nil,
      "TEBAKO_PASS_THROUGH" => "1"
    }.freeze

    # Magic version numbers used to ensure compatibility for Ruby 2.7.x, 3.0.x
    # These are the minimal versions required to provide linux-gnu / linux-musl differentiantion by bundler
    # Ruby 3.1+ default bubdler/rubygems versions work correctly out of the box
    BUNDLER_VERSION = "2.4.22"
    RUBYGEMS_VERSION = "3.4.22"

    class << self
      # Deploy
      def deploy(src_dir, tbd, gflength)
        puts "-- Running deploy script"

        ruby_ver = ruby_version(tbd)
        update_rubygems(tbd, "#{src_dir}/lib", ruby_ver, RUBYGEMS_VERSION)
        install_gem tbd, "tebako-runtime"
        install_gem tbd, "bundler", (PatchHelpers.ruby31?(ruby_ver) ? nil : BUNDLER_VERSION) if gflength.to_i != 0
      end

      # Deploy
      def init(stash_dir, src_dir, pre_dir, bin_dir)
        puts "-- Running init script"

        puts "   ... creating packaging environment at #{src_dir}"
        PatchHelpers.recreate([src_dir, pre_dir, bin_dir])
        FileUtils.cp_r "#{stash_dir}/.", src_dir
      end

      # Pass1
      # Executed before Ruby build, patching ensures that Ruby itself is linked statically
      def pass1(ostype, ruby_source_dir, mount_point, src_dir, ruby_ver)
        puts "-- Running pass1 script"

        PatchHelpers.recreate(src_dir)
        do_patch(Pass1.get_patch_map(ostype, mount_point, ruby_ver), ruby_source_dir)

        # Roll back pass2 patches
        # Just in case we are recovering after some error
        PatchHelpers.restore_and_save_files(FILES_TO_RESTORE, ruby_source_dir)
        PatchHelpers.restore_and_save_files(FILES_TO_RESTORE_MUSL, ruby_source_dir) if ostype =~ /linux-musl/
        PatchHelpers.restore_and_save_files(FILES_TO_RESTORE_MSYS, ruby_source_dir) if ostype =~ /msys/
      end

      # Pass2
      # Creates packaging environment, patching ensures that tebako package is linked statically
      def pass2(ostype, ruby_source_dir, deps_lib_dir, ruby_ver)
        puts "-- Running pass2 script"

        do_patch(Pass2.get_patch_map(ostype, deps_lib_dir, ruby_ver), ruby_source_dir)
      end

      # Stash
      # Saves pristine Ruby environment that is used to deploy applications for packaging
      def stash(src_dir, stash_dir)
        puts "-- Running stash script"
        #  .... this code snippet is executed 'outside' of Ruby scripts
        # shall be reconsidered
        #    FileUtils.cd ruby_source_dir do
        #        puts "   ... creating pristine ruby environment at #{src_dir} [patience, it will take some time]"
        #        out, st = Open3.capture2e("cmake", "-E", "chdir", ruby_source_dir, "make", "install")
        #        print out if st.exitstatus != 0 || verbose
        #        raise Tebako::Error.new("stash [make install] failed with code #{st.exitstatus}") if st.exitstatus != 0
        #    end

        puts "   ... saving pristine ruby environment to #{stash_dir}"
        PatchHelpers.recreate(stash_dir)
        FileUtils.cp_r "#{src_dir}/.", stash_dir
      end

      private

      def install_gem(tbd, name, ver = nil)
        puts "   ... installing #{name} gem#{" version #{ver}" if ver}"
        PatchHelpers.with_env(DEPLOY_ENV) do
          params = ["#{tbd}/gem", "install", name.to_s]
          params.push("-v", ver.to_s) if ver

          out, st = Open3.capture2e(*params)
          raise Tebako::Error, "Failed to install #{name} (#{st}):\n #{out}" unless st.exitstatus.zero?
        end
      end

      def do_patch(patch_map, root)
        patch_map.each { |fname, mapping| PatchHelpers.patch_file("#{root}/#{fname}", mapping) }
      end

      def ruby_version(tbd)
        ruby_version = nil
        PatchHelpers.with_env(DEPLOY_ENV) do
          out, st = Open3.capture2e("#{tbd}/ruby", "--version")
          raise Tebako::Error, "Failed to run ruby --version" unless st.exitstatus.zero?

          match = out.match(/ruby (\d+\.\d+\.\d+)/)
          raise Tebako::Error, "Failed to parse Ruby version from #{out}" unless match

          ruby_version = match[1]
        end
        ruby_version
      end

      def update_rubygems(tbd, tld, ruby_ver, gem_ver)
        return if PatchHelpers.ruby31?(ruby_ver)

        puts "   ... updating rubygems to #{gem_ver}"
        PatchHelpers.with_env(DEPLOY_ENV) do
          out, st = Open3.capture2e("#{tbd}/gem", "update", "--no-doc", "--system", gem_ver.to_s)
          raise Tebako::Error, "Failed to update rubugems to #{gem_ver} (#{st}):\n #{out}" unless st.exitstatus.zero?
        end
        ruby_api_ver = ruby_ver.split(".")[0..1].join(".")
        # Autoload cannot handle statically linked openssl extension
        # Changing it to require seems to be the simplest solution
        PatchHelpers.patch_file("#{tld}/ruby/site_ruby/#{ruby_api_ver}.0/rubygems/openssl.rb",
                                { "autoload :OpenSSL, \"openssl\"" => "require \"openssl\"" })
      end
    end
  end
end
