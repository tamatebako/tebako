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

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (internal)
  module Packager
    class << self
      # Deploy
      def deploy(target_dir, pre_dir, ruby_ver, fs_root, fs_entrance, cwd) # rubocop:disable Metrics/ParameterLists
        puts "-- Running deploy script"

        deploy_helper = Tebako::DeployHelper.new(fs_root, fs_entrance, target_dir, pre_dir)
        deploy_helper.configure(ruby_ver, cwd)
        deploy_helper.deploy
        Tebako::Stripper.strip(deploy_helper, target_dir)
      end

      # Check that the packaging environment the press needs (the prebuilt
      # mkdwarfs provisioned by 'tebako setup') is in place
      def check_prebuilt_env!(deps_bin_dir)
        return unless Dir.glob(File.join(deps_bin_dir, "mkdwarfs*")).empty?

        Tebako.packaging_error(128, "mkdwarfs (#{deps_bin_dir})")
      end

      # Deploy the application and build its DwarFS image for stitching onto
      # a prebuilt runtime. The image is seeded from the resolved runtime's
      # extracted filesystem layout (layout_dir): the pristine Ruby
      # environment the deploy step runs against. The layout also carries an
      # entry dispatcher placeholder at /local/stub.rb (the runtime's
      # compiled-in entry), replaced below with the application's dispatcher.
      def build_app_image(options_manager, scenario_manager, layout_dir) # rubocop:disable Metrics/AbcSize
        init(layout_dir, options_manager.data_src_dir, options_manager.data_pre_dir,
             options_manager.data_bin_dir)
        deploy(options_manager.data_src_dir, options_manager.data_pre_dir, options_manager.rv,
               options_manager.root, scenario_manager.fs_entrance, options_manager.cwd)
        align_layout_to_runtime!(options_manager.data_src_dir, layout_dir, options_manager.rv)
        write_entry_dispatcher(options_manager.data_src_dir, scenario_manager, options_manager.cwd)
        mkdwarfs(options_manager.deps_bin_dir, options_manager.data_bundle_file, options_manager.data_src_dir)
        options_manager.data_bundle_file
      end

      # Rename the image's arch directories to the runtime's names and drop
      # in the runtime's own rbconfig.rb. No-op when the conventions already
      # match (always the case when the image was seeded from the runtime's
      # own layout, and off macOS in general, where the arch string carries
      # no OS version).
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

      def mkdwarfs(deps_bin_dir, data_bin_file, data_src_dir)
        puts "-- Running mkdwarfs script"
        FileUtils.chmod("a+x", Dir.glob(File.join(deps_bin_dir, "mkdwarfs*")))
        params = [File.join(deps_bin_dir, "mkdwarfs"), "-o", data_bin_file, "-i", data_src_dir, "--no-progress"]
        BuildHelpers.run_with_capture_v(params)
      end

      # Init
      # Seeds the packaging environment from the resolved runtime's extracted
      # filesystem layout (the pristine Ruby environment applications are
      # deployed against)
      def init(layout_dir, src_dir, pre_dir, bin_dir)
        puts "-- Running init script"

        puts "   ... creating packaging environment at #{src_dir}"
        BuildHelpers.recreate([src_dir, pre_dir, bin_dir])
        FileUtils.cp_r "#{layout_dir}/.", src_dir
      end
    end
  end
end
