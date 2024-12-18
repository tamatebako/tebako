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

require "etc"
require "fileutils"
require "pathname"
require "rbconfig"

require_relative "codegen"
require_relative "error"
require_relative "options_manager"
require_relative "scenario_manager"
require_relative "packager_lite"

# Tebako - an executable packager
# Command-line interface methods
module Tebako
  # Cli helpers
  module CliHelpers
    def do_press(options_manager)
      scenario_manager = Tebako::ScenarioManager.new(options_manager.root, options_manager.fs_entrance)
      puts options_manager.press_announce(scenario_manager.msys?)

      if options_manager.mode == "both" || options_manager.mode == "runtime" || options_manager.mode == "bundle"
        do_press_runtime(options_manager, scenario_manager)
      end

      if options_manager.mode == "both" || options_manager.mode == "application"
        do_press_application(options_manager, scenario_manager)
      end

      true
    end

    def do_press_application(options_manager, scenario_manager)
      packager = Tebako::PackagerLite.new(options_manager, scenario_manager)
      packager.create_package
    end

    def do_press_runtime(options_manager, scenario_manager)
      generate_files(options_manager, scenario_manager)
      cfg_cmd = "cmake -DSETUP_MODE:BOOLEAN=OFF #{options_manager.cfg_options} #{options_manager.press_options}"
      build_cmd = "cmake --build #{options_manager.output_folder} --target tebako --parallel #{Etc.nprocessors}"
      merged_env = ENV.to_h.merge(options_manager.b_env)
      Tebako.packaging_error(103) unless system(merged_env, cfg_cmd)
      Tebako.packaging_error(104) unless system(merged_env, build_cmd)
      true
    end

    def do_setup(options_manager)
      puts "Setting up tebako packaging environment"

      cfg_cmd = "cmake -DSETUP_MODE:BOOLEAN=ON #{options_manager.cfg_options}"
      build_cmd = "cmake --build \"#{options_manager.output_folder}\" --target setup --parallel #{Etc.nprocessors}"
      merged_env = ENV.to_h.merge(options_manager.b_env)
      Tebako.packaging_error(101) unless system(merged_env, cfg_cmd)
      Tebako.packaging_error(102) unless system(merged_env, build_cmd)
      true
    end

    def generate_files(options_manager, scenario_manager)
      puts "-- Generating files"
      scenario_manager.configure_scenario

      v_parts = Tebako::VERSION.split(".")
      Tebako::Codegen.generate_tebako_version_h(options_manager, v_parts)
      Tebako::Codegen.generate_tebako_fs_cpp(options_manager, scenario_manager)
      Tebako::Codegen.generate_deploy_rb(options_manager, scenario_manager)

      return unless options_manager.mode == "both" || options_manager.mode == "runtime"

      Tebako::Codegen.generate_stub_rb(options_manager)
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
  end
end
