# frozen_string_literal: true

# Copyright (c) 2023-2025 [Ribose Inc](https://www.ribose.com).
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

require "digest"
require "etc"
require "yaml"

# Tebako - an executable packager
# Command-line interface methods
module Tebako
  # Cli helpers
  module CliHelpers
    WARN = <<~WARN

      ******************************************************************************************************************
      *                                                                                                                *
      *  WARNING: You are packaging in-place, i.e.: tebako package will be placed inside application root.             *
      *  It is not an error but we do not recommend it because it is a way to keep packaging old versions recrsively.  *
      *                                                                                                                *
      *  For example, ensure that `--root=` differs from `--output=` as described in README.adoc:                      *
      *  tebako press --root='~/projects/myproject' --entry=start.rb --output=/temp/myproject.tebako                   *
      *                                                                                                                *
      ******************************************************************************************************************

    WARN

    WARN2 = <<~WARN

      ******************************************************************************************************************
      *                                                                                                                *
      *  WARNING: You are creating packaging environment inside application root.                                      *
      *  It is not an error but it means that all build-time artifacts will ne included in tebako package.             *
      *  You do not need it unless under very special circumstances like tebako packaging tebako itself.               *
      *                                                                                                                *
      *  Please consider removing your exisitng `--prefix` folder abd use another one that points outside of `--root`  *
      *  like tebako press --r ~/projects/myproject -e start.rb -o /temp/myproject.tebako -p ~/.tebako                 *
      *                                                                                                                *
      ******************************************************************************************************************

    WARN

    def check_warnings(options_manager)
      puts WARN if options_manager.package_within_root?
      puts WARN2 if options_manager.prefix_within_root?
      sleep 5
    end

    def do_press(options_manager)
      scenario_manager = Tebako::ScenarioManager.new(options_manager.root, options_manager.fs_entrance)
      scenario_manager.configure_scenario
      options_manager.process_gemfile(scenario_manager.gemfile_path) if scenario_manager.with_gemfile
      check_warnings(options_manager)
      puts options_manager.press_announce
      dispatch_press(options_manager, scenario_manager)
    end

    def dispatch_press(options_manager, scenario_manager)
      if options_manager.three_part?
        do_press_three_part(options_manager, scenario_manager)
      else
        do_press_prebuilt(options_manager, scenario_manager)
      end
    end

    # Press onto a prebuilt runtime package ('classic'): resolve (download/
    # verify/cache) the runtime, deploy the application image, stitch,
    # re-sign on macOS.
    def do_press_prebuilt(options_manager, scenario_manager)
      Tebako::Packager.check_prebuilt_env!(options_manager.deps_bin_dir)
      runtime_path = Tebako::RuntimeManager.resolve(options_manager.ruby_ver, options_manager.host_platform)
      app_image = Tebako::Packager.build_app_image(options_manager, scenario_manager, runtime_path)

      images = [{ path: app_image, mount_point: scenario_manager.fs_mount_point,
                  format_id: Tebako::Stitcher::FORMAT_DWARFS }] + options_manager.images
      package = "#{options_manager.package}#{scenario_manager.exe_suffix}"
      Tebako::Stitcher.stitch(runtime_path, images: images, output: package)
      puts "Created tebako package at \"#{package}\""
    end

    # Press a three-part package (Stage 3B): tebako-bootstrap + application
    # image slot(s) + tpkg trailer with the runtime reference (launcher ABI
    # v1). 'lean' (the default) resolves the runtime into the shared cache at
    # first run; 'fat' additionally embeds the runtime package as a payload
    # slot, so the first run installs it without any network access.
    def do_press_three_part(options_manager, scenario_manager)
      bootstrap_path, runtime_path = resolve_three_part_parts(options_manager)
      app_image = Tebako::Packager.build_app_image(options_manager, scenario_manager, runtime_path)
      payload_path = options_manager.fat? ? runtime_path : nil
      images = three_part_images(options_manager, scenario_manager, app_image, payload_path)
      package = "#{options_manager.package}#{scenario_manager.exe_suffix}"
      stitch_three_part(options_manager, bootstrap_path, images, package, payload_path)
      puts "Created tebako package at \"#{package}\""
    end

    def stitch_three_part(options_manager, bootstrap_path, images, package, payload_path)
      Tebako::Stitcher.stitch(bootstrap_path, images: images, output: package, lean: true,
                                              ruby_version: options_manager.ruby_ver,
                                              launcher_abi: Tebako::LauncherAbi::VERSION,
                                              runtime_sha256: payload_sha256(payload_path))
    end

    def payload_sha256(payload_path)
      payload_path && Digest::SHA256.file(payload_path).hexdigest
    end

    # Validate the packaging environment, then resolve the bootstrap and the
    # runtime into the shared cache. The runtime is needed in both three-part
    # modes: 'fat' embeds it as a payload slot, 'lean' uses its extracted
    # layout to seed the application image (and references it in the trailer
    # for first-run resolution)
    def resolve_three_part_parts(options_manager)
      check_bootstrap_version!(options_manager)
      Tebako::Packager.check_prebuilt_env!(options_manager.deps_bin_dir)
      runtime_path = Tebako::RuntimeManager.resolve(options_manager.ruby_ver, options_manager.host_platform)
      [Tebako::BootstrapManager.resolve(options_manager.host_platform), runtime_path]
    end

    def three_part_images(options_manager, scenario_manager, app_image, runtime_path)
      images = [{ path: app_image, mount_point: scenario_manager.fs_mount_point,
                  format_id: Tebako::Stitcher::FORMAT_DWARFS }] + options_manager.images
      images << { path: runtime_path, mount_point: "", format_id: Tebako::Stitcher::FORMAT_RUNTIME } if runtime_path
      images
    end

    # The fat payload slot is installed by the bootstrap at first run — a
    # capability added in tebako-bootstrap 0.2.0
    def check_bootstrap_version!(options_manager)
      return unless options_manager.fat?

      version = Tebako::BootstrapManager.default_version
      minimum = Tebako::BootstrapManager::PAYLOAD_MIN_VERSION
      return if payload_capable?(version, minimum)

      Tebako.packaging_error(134, "fat mode requires tebako-bootstrap >= #{minimum} (selected: #{version}; " \
                                  "set TEBAKO_BOOTSTRAP_VERSION to a payload-capable release)")
    end

    def payload_capable?(version, minimum)
      Gem::Version.new(version) >= Gem::Version.new(minimum)
    rescue ArgumentError
      false
    end

    def do_setup(options_manager)
      puts "Setting up tebako packaging environment"

      merged_env = ENV.to_h.merge(Tebako::ScenarioManagerBase.new.b_env)
      Tebako.packaging_error(101) unless system(merged_env, setup_cfg_cmd(options_manager))
      Tebako.packaging_error(102) unless system(merged_env, setup_build_cmd(options_manager))
    end

    def options_from_tebafile(tebafile)
      ::YAML.load_file(tebafile)["options"] || {}
    rescue Psych::SyntaxError => e
      puts "Warning: The tebafile '#{tebafile}' contains invalid YAML syntax."
      puts e.message
      {}
    rescue StandardError => e
      puts "An unexpected error occurred while loading the tebafile '#{tebafile}'."
      puts e.message
      {}
    end

    def setup_build_cmd(options_manager)
      "cmake --build \"#{options_manager.output_folder}\" --target setup --parallel #{Etc.nprocessors}"
    end

    def setup_cfg_cmd(options_manager)
      "cmake -DSETUP_MODE:BOOLEAN=ON #{options_manager.cfg_options}"
    end
  end
end
