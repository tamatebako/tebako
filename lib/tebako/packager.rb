# frozen_string_literal: true

# Copyright (c) 2021-2025 [Ribose Inc](https://www.ribose.com).
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
require "find"
require "pathname"

require_relative "error"
require_relative "deploy_helper"
require_relative "ruby_builder"
require_relative "stripper"
require_relative "packager/pass1_patch"
require_relative "packager/pass1a_patch"
require_relative "packager/pass2_patch_crt"
require_relative "packager/patch_helpers"

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (internal)
  module Packager
    FILES_TO_RESTORE = %w[
      common.mk
      configure
      config.status
      dir.c
      dln.c
      file.c
      gem_prelude.rb
      io.c
      main.c
      Makefile
      ruby.c
      thread_pthread.c
      util.c
      ext/bigdecimal/bigdecimal.h
      ext/Setup
      cygwin/GNUmakefile.in
      include/ruby/onigmo.h
      lib/rubygems/openssl.rb
      lib/rubygems/path_support.rb
      template/Makefile.in
      tool/mkconfig.rb
      win32/winmain.c
      win32/file.c
    ].freeze

    class << self # rubocop:disable Metrics/ClassLength
      # Create implib
      def create_implib(src_dir, package_src_dir, app_name, ruby_ver)
        a_name = File.basename(app_name, ".*")
        create_def(src_dir, a_name)
        puts "   ... creating Windows import library"
        params = ["dlltool", "-d", def_fname(src_dir, a_name), "-D", out_fname(a_name), "--output-lib",
                  lib_fname(package_src_dir, ruby_ver)]
        BuildHelpers.run_with_capture(params)
      end

      # Deploy
      def deploy(target_dir, pre_dir, ruby_ver, fs_root, fs_entrance, cwd) # rubocop:disable Metrics/ParameterLists
        puts "-- Running deploy script"

        deploy_helper = Tebako::DeployHelper.new(fs_root, fs_entrance, target_dir, pre_dir)
        deploy_helper.configure(ruby_ver, cwd)
        deploy_helper.deploy
        Tebako::Stripper.strip(deploy_helper, target_dir)
      end

      def do_patch(patch_map, root)
        patch_map.each { |fname, mapping| PatchHelpers.patch_file("#{root}/#{fname}", mapping) }
      end

      def finalize(src_dir, app_name, ruby_ver, patchelf, output_type)
        puts "-- Running finalize script"

        RubyBuilder.new(ruby_ver, src_dir).target_build(output_type)
        exe_suffix = ScenarioManagerBase.new.exe_suffix
        src_name = File.join(src_dir, "ruby#{exe_suffix}")
        patchelf(src_name, patchelf)
        package_name = "#{app_name}#{exe_suffix}"
        # strip_or_copy(os_type, src_name, package_name)
        Tebako::Stripper.strip_file(src_name, package_name)
        puts "Created tebako #{output_type} at \"#{package_name}\""
      end

      # Init
      def init(stash_dir, src_dir, pre_dir, bin_dir)
        puts "-- Running init script"

        puts "   ... creating packaging environment at #{src_dir}"
        PatchHelpers.recreate([src_dir, pre_dir, bin_dir])
        FileUtils.cp_r "#{stash_dir}/.", src_dir
      end

      # Check that the packaging environment the prebuilt-runtime press
      # needs (pristine Ruby stash + mkdwarfs) is in place
      def check_prebuilt_env!(stash_dir, deps_bin_dir)
        missing = []
        missing << "Ruby environment stash (#{stash_dir})" unless Dir.exist?(stash_dir)
        missing << "mkdwarfs (#{deps_bin_dir})" if Dir.glob(File.join(deps_bin_dir, "mkdwarfs*")).empty?
        return if missing.empty?

        Tebako.packaging_error(128, missing.join(", "))
      end

      # Deploy the application and build its DwarFS image for stitching onto
      # a prebuilt runtime. Same layout as the bundle-mode image, plus an
      # entry dispatcher at /local/stub.rb (the runtime's compiled-in entry).
      # When layout_dir is given (the resolved runtime's extracted layout),
      # the image's arch conventions are aligned to the runtime's: ruby's
      # compiled-in search paths come from the runtime build, while the
      # local packaging environment names arch directories after the press
      # machine -- on macOS the arch string embeds the kernel version
      # (arm64-darwin23 vs arm64-darwin24), so they differ whenever press
      # and runtime were built on different macOS releases.
      def build_app_image(options_manager, scenario_manager, layout_dir = nil) # rubocop:disable Metrics/AbcSize
        init(options_manager.stash_dir, options_manager.data_src_dir, options_manager.data_pre_dir,
             options_manager.data_bin_dir)
        deploy(options_manager.data_src_dir, options_manager.data_pre_dir, options_manager.rv,
               options_manager.root, scenario_manager.fs_entrance, options_manager.cwd)
        align_layout_to_runtime!(options_manager.data_src_dir, layout_dir, options_manager.rv) if layout_dir
        write_entry_dispatcher(options_manager.data_src_dir, scenario_manager, options_manager.cwd)
        mkdwarfs(options_manager.deps_bin_dir, options_manager.data_bundle_file, options_manager.data_src_dir)
        options_manager.data_bundle_file
      end

      # Rename the image's arch directories to the runtime's names and drop
      # in the runtime's own rbconfig.rb. No-op when the conventions already
      # match (always the case off macOS, where the arch string carries no
      # OS version).
      def align_layout_to_runtime!(data_src_dir, layout_dir, ruby_ver)
        align_stdlib_arch!(data_src_dir, layout_dir, ruby_ver.api_version)
        align_gem_ext_arch!(data_src_dir, layout_dir, ruby_ver.api_version)
      end

      def align_stdlib_arch!(data_src_dir, layout_dir, api_ver)
        runtime_arch = arch_dir_of(File.join(layout_dir, "lib", "ruby", api_ver), "rbconfig.rb")
        image_arch = arch_dir_of(File.join(data_src_dir, "lib", "ruby", api_ver), "rbconfig.rb")
        return if runtime_arch.nil? || image_arch.nil? || runtime_arch == image_arch

        puts "   ... aligning app image layout to the runtime (#{image_arch} -> #{runtime_arch})"
        FileUtils.mv(File.join(data_src_dir, "lib", "ruby", api_ver, image_arch),
                     File.join(data_src_dir, "lib", "ruby", api_ver, runtime_arch))
        FileUtils.cp(File.join(layout_dir, "lib", "ruby", api_ver, runtime_arch, "rbconfig.rb"),
                     File.join(data_src_dir, "lib", "ruby", api_ver, runtime_arch, "rbconfig.rb"))
      end

      # Native-gem extensions dir uses the dashed flavor of the arch string
      # (arm64-darwin-23); align it to the runtime's as well
      def align_gem_ext_arch!(data_src_dir, layout_dir, api_ver)
        img_ext = File.join(data_src_dir, "lib", "ruby", "gems", api_ver, "extensions")
        rt_ext = File.join(layout_dir, "lib", "ruby", "gems", api_ver, "extensions")
        runtime_ext = first_dir(rt_ext)
        return if runtime_ext.nil? || !Dir.exist?(img_ext)

        Dir.children(img_ext).each do |d|
          next if d == runtime_ext || !File.directory?(File.join(img_ext, d))

          FileUtils.mv(File.join(img_ext, d), File.join(img_ext, runtime_ext))
        end
      end

      def arch_dir_of(dir, marker)
        return nil unless Dir.exist?(dir)

        Dir.children(dir).find { |d| File.exist?(File.join(dir, d, marker)) }
      end

      def first_dir(dir)
        return nil unless Dir.exist?(dir)

        Dir.children(dir).find { |d| File.directory?(File.join(dir, d)) }
      end

      # The prebuilt runtime packages are pressed in 'runtime' mode, so their
      # compiled-in entry point is /local/stub.rb. A stitched package mounts
      # the application image as the root filesystem; the dispatcher written
      # here receives control from the runtime and loads the real entry point
      # (replicating the bundle-mode working directory when --cwd was given).
      def write_entry_dispatcher(data_src_dir, scenario_manager, cwd)
        dispatcher = +""
        dispatcher << "Dir.chdir(\"#{scenario_manager.fs_mount_point}/#{cwd}\")\n" unless cwd.nil?
        dispatcher << "load \"#{scenario_manager.fs_mount_point}#{scenario_manager.fs_entry_point}\"\n"

        local_dir = File.join(data_src_dir, "local")
        FileUtils.mkdir_p(local_dir)
        File.write(File.join(local_dir, "stub.rb"), dispatcher)
      end

      def mkdwarfs(deps_bin_dir, data_bin_file, data_src_dir, descriptor = nil)
        puts "-- Running mkdwarfs script"
        FileUtils.chmod("a+x", Dir.glob(File.join(deps_bin_dir, "mkdwarfs*")))
        params = [File.join(deps_bin_dir, "mkdwarfs"), "-o", data_bin_file, "-i", data_src_dir, "--no-progress"]
        params << "--header" << descriptor if descriptor
        BuildHelpers.run_with_capture_v(params)
      end

      # Pass1
      # Executed before Ruby build, patching ensures that Ruby itself is linked statically
      def pass1(ostype, ruby_source_dir, mount_point, src_dir, ruby_ver)
        puts "-- Running pass1 script"
        PatchHelpers.recreate(src_dir)

        # Roll all known patches
        # Just in case we are recovering after some error
        PatchHelpers.restore_and_save_files(FILES_TO_RESTORE, ruby_source_dir, strict: false)

        patch = crt_pass1_patch(ostype, mount_point, ruby_ver)
        do_patch(patch.patch_map, ruby_source_dir)
      end

      # Pass1A
      # Patch gem_prelude.rb
      def pass1a(ruby_source_dir)
        puts "-- Running pass1a script"
        patch = Pass1APatch.new
        do_patch(patch.patch_map, ruby_source_dir)
      end

      # Pass2
      # Creates packaging environment, patching ensures that tebako package is linked statically
      def pass2(ostype, ruby_source_dir, deps_lib_dir, ruby_ver)
        puts "-- Running pass2 script"

        patch = crt_pass2_patch(ostype, deps_lib_dir, ruby_ver)
        do_patch(patch.patch_map, ruby_source_dir)
      end

      # Stash
      # Created and saves pristine Ruby environment that is used to deploy applications for packaging
      def stash(src_dir, stash_dir, ruby_source_dir, ruby_ver)
        puts "-- Running stash script"
        RubyBuilder.new(ruby_ver, ruby_source_dir).toolchain_build

        puts "   ... saving pristine Ruby environment to #{stash_dir}"
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
        File.join(src_dir, "lib", "libx64-ucrt-ruby#{ruby_ver.lib_version}.a")
      end

      def patchelf(src_name, patchelf)
        return if patchelf.nil?

        params = [patchelf, "--remove-needed-version", "libpthread.so.0", "GLIBC_PRIVATE", src_name]
        BuildHelpers.run_with_capture(params)
      end

      # def strip_or_copy(_os_type, src_name, package_name)
      # [TODO] On MSys strip sometimes creates a broken executable
      # https://github.com/tamatebako/tebako/issues/172
      # if Packager::PatchHelpers.msys?(os_type)
      # FileUtils.cp(src_name, package_name)
      # else
      # Tebako::Stripper.strip_file(src_name, package_name)
      # end
      # end
    end
  end
end
