# frozen_string_literal: true

# Copyright (c) 2021-2023 [Ribose Inc](https://www.ribose.com).
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

    class << self
      # Pass1
      # Executed before Ruby build, patching ensures that Ruby itself is linked statically
      def pass1(ostype, ruby_source_dir, mount_point, src_dir, ruby_ver)
        puts "-- Running pass1 script"

        recreate(src_dir)
        do_patch(Pass1.get_patch_map(ostype, mount_point, ruby_ver), ruby_source_dir)

        # Roll back pass2 patches
        # Just in case we are recovering after some error
        restore_and_save_files(FILES_TO_RESTORE, ruby_source_dir)
        restore_and_save_files(FILES_TO_RESTORE_MUSL, ruby_source_dir) if ostype =~ /linux-musl/
        restore_and_save_files(FILES_TO_RESTORE_MSYS, ruby_source_dir) if ostype =~ /msys/
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
        recreate(stash_dir)
        FileUtils.cp_r "#{src_dir}/.", stash_dir
      end

      # Deploy
      def deploy(stash_dir, src_dir, pre_dir, bin_dir, tbd)
        puts "-- Running deploy script"

        puts "   ... creating packaging environment at #{src_dir}"
        recreate([src_dir, pre_dir, bin_dir])
        FileUtils.cp_r "#{stash_dir}/.", src_dir

        install_gem tbd, "tebako-runtime"
      end

      def do_patch(patch_map, root)
        patch_map.each { |fname, mapping| patch_file("#{root}/#{fname}", mapping) }
      end

      private

      def install_gem(tbd, name)
        puts "   ... installing #{name} gem"
        with_env(DEPLOY_ENV) do
          out, st = Open3.capture2e("#{tbd}/gem", "install", name.to_s, "--no-doc")
          raise Tebako::Error, "Failed to install #{name} (#{st}):\n #{out}" unless st.exitstatus.zero?
        end
      end

      def patch_file(fname, mapping)
        raise Tebako::Error, "Could not patch #{fname} because it does not exist." unless File.exist?(fname)

        puts "   ... patching #{fname}"
        restore_and_save(fname)
        contents = File.read(fname)
        mapping.each { |pattern, subst| contents.sub!(pattern, subst) }
        File.open(fname, "w") { |file| file << contents }
      end

      def recreate(dirname)
        FileUtils.rm_rf(dirname, noop: nil, verbose: nil, secure: true)
        FileUtils.mkdir(dirname)
      end

      def restore_and_save(fname)
        raise Tebako::Error, "Could not save #{fname} because it does not exist." unless File.exist?(fname)

        old_fname = "#{fname}.old"
        if File.exist?(old_fname)
          FileUtils.rm_f(fname)
          File.rename(old_fname, fname)
        end
        FileUtils.cp(fname, old_fname)
      end

      def restore_and_save_files(files, ruby_source_dir)
        files.each do |fname|
          restore_and_save "#{ruby_source_dir}/#{fname}"
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
          hash.each { |k, _v| ENV[k] = old[k] }
        end
      end
    end
  end
end
