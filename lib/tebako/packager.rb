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
require "find"
require "pathname"

require_relative "error"
require_relative "deploy_helper"
require_relative "ruby_builder"
require_relative "stripper"
require_relative "packager/pass1"
require_relative "packager/pass1a"
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
      win32/file.c
    ].freeze
    # Do not need to restore cygwin/GNUmakefile.in
    # because it is patched (differently) both on pass 1 and pass2
    # cygwin/GNUmakefile.in

    FILES_TO_RESTORE_MUSL = %w[
      thread_pthread.c
    ].freeze

    class << self
      # Create implib
      def create_implib(src_dir, package_src_dir, app_name, ruby_ver)
        create_def(src_dir, app_name)
        puts "   ... creating Windows import library"
        params = ["dlltool", "-d", def_fname(src_dir, app_name), "-D", out_fname(app_name),
                  "--output-lib", lib_fname(package_src_dir, ruby_ver)]
        out, st = Open3.capture2e(*params)
        raise Tebako::Error, "Failed to create import library:\n #{out}" unless st.exitstatus.zero?
      end

      # Deploy
      def deploy(os_type, target_dir, pre_dir, ruby_ver, fs_root, fs_entrance, fs_mount_point, cwd) # rubocop:disable Metrics/ParameterLists
        puts "-- Running deploy script"

        deploy_helper = Tebako::DeployHelper.new(fs_root, fs_entrance, fs_mount_point, target_dir, pre_dir)
        deploy_helper.config(os_type, ruby_ver, cwd)
        deploy_helper.deploy
        Tebako::Stripper.strip(os_type, target_dir)
      end

      def finalize(os_type, src_dir, app_name, ruby_ver)
        RubyBuilder.final_build(ruby_ver, src_dir)
        exe_suffix = Packager::PatchHelpers.exe_suffix(os_type)
        src_name = File.join(src_dir, "ruby#{exe_suffix}")
        package_name = "#{app_name}#{exe_suffix}"
        # [TODO] On MSys strip sometimes creates a broken executable
        # https://github.com/tamatebako/tebako/issues/172
        if Packager::PatchHelpers.msys?(os_type)
          FileUtils.cp(src_name, package_name)
        else
          Tebako::Stripper.strip_file(src_name, package_name)
        end
        puts "Created tebako package at \"#{package_name}\""
      end

      # Init
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

        # Roll back pass1a, pass2 patches
        # Just in case we are recovering after some error
        PatchHelpers.restore_and_save_files(FILES_TO_RESTORE, ruby_source_dir)
        PatchHelpers.restore_and_save_files(FILES_TO_RESTORE_MUSL, ruby_source_dir) if ostype =~ /linux-musl/
        PatchHelpers.restore_and_save_files(FILES_TO_RESTORE_MSYS, ruby_source_dir) if ostype =~ /msys/
      end

      # Pass1A
      # Patch gem_prelude.rb
      def pass1a(ruby_source_dir)
        puts "-- Running pass1a script"

        do_patch(Pass1A.get_patch_map, ruby_source_dir)
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

      def create_def(src_dir, app_name)
        puts "   ... creating Windows def file"
        File.open(def_fname(src_dir, app_name), "w") do |file|
          file.puts "LIBRARY #{out_fname(app_name)}"
          File.readlines(File.join(src_dir, "tebako.def")).each do |line|
            file.puts line unless line.include?("DllMain")
          end
        end
      end

      def def_fname(src_dir, app_name)
        File.join(src_dir, "#{app_name}.def")
      end

      def out_fname(app_name)
        File.join("#{app_name}.exe")
      end

      def lib_fname(src_dir, ruby_ver)
        File.join(src_dir, "lib", "libx64-ucrt-ruby#{ruby_ver[0]}#{ruby_ver[2]}0.a")
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
    end
  end
end
