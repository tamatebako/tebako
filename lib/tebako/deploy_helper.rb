# frozen_string_literal: true

# Copyright (c) 2024 [Ribose Inc](https://www.ribose.com).
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

# require "bundler"
require "fileutils"
require "find"

require_relative "error"
require_relative "build_helpers"
require_relative "packager/patch_helpers"
require_relative "scenario_manager"

require_relative "packager/patch"
require_relative "packager/rubygems_patch"

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (deployer)
  class DeployHelper < ScenarioManager # rubocop:disable Metrics/ClassLength
    def initialize(fs_root, fs_entrance, target_dir, pre_dir)
      super(fs_root, fs_entrance)
      @fs_root = fs_root
      @fs_entrance = fs_entrance
      @target_dir = target_dir
      @pre_dir = pre_dir
      @verbose = %w[yes true].include?(ENV.fetch("VERBOSE", nil))
      @ncores = BuildHelpers.ncores
    end

    attr_reader :bundler_command, :gem_command, :gem_home

    def configure(ruby_ver, cwd)
      @ruby_ver = ruby_ver
      @cwd = cwd

      @tbd = File.join(@target_dir, "bin")
      @tgd = @gem_home = File.join(@target_dir, "lib", "ruby", "gems", @ruby_ver.api_version)
      @tld = File.join(@target_dir, "local")

      configure_scenario
      configure_commands
    end

    def deploy
      BuildHelpers.with_env(deploy_env) do
        update_rubygems
        system("#{gem_command} env") if @verbose
        install_gem("tebako-runtime")
        install_gem("bundler", @bundler_version) if needs_bundler?
        deploy_solution
        check_cwd
      end
    end

    def deploy_env
      {
        "GEM_HOME" => gem_home,
        "GEM_PATH" => gem_home,
        "GEM_SPEC_CACHE" => File.join(@target_dir, "spec_cache"),
        "TEBAKO_PASS_THROUGH" => "1"
      }
    end

    def install_gem(name, ver = nil)
      puts "   ... installing #{name} gem#{" version #{ver}" if ver}"

      params = [@gem_command, "install", name.to_s]
      params.push("-v", ver.to_s) if ver
      ["--no-document", "--install-dir", @tgd, "--bindir", @tbd].each do |param|
        params.push(param)
      end
      BuildHelpers.run_with_capture_v(params)
    end

    def needs_bundler?
      puts !@ruby_ver.ruby31?
      puts @with_gemfile_lock
      @with_gemfile && (!@ruby_ver.ruby31? || @with_gemfile_lock)
    end

    def update_rubygems
      return if @ruby_ver.ruby31?

      puts "   ... updating rubygems to #{Tebako::RUBYGEMS_VERSION}"
      BuildHelpers.run_with_capture_v([@gem_command, "update", "--no-doc", "--system",
                                       Tebako::RUBYGEMS_VERSION])

      patch = Packager::RubygemsUpdatePatch.new(@fs_mount_point).patch_map
      Packager.do_patch(patch, "#{@target_dir}/lib/ruby/site_ruby/#{@ruby_ver.api_version}")
    end

    private

    def bundle_config
      BuildHelpers.run_with_capture_v([@bundler_command, "config", "set", "--local", "build.ffi",
                                       "--disable-system-libffi"])
      BuildHelpers.run_with_capture_v([@bundler_command, "config", "set", "--local", "build.nokogiri",
                                       @nokogiri_option])
      BuildHelpers.run_with_capture_v([@bundler_command, "config", "set", "--local", "force_ruby_platform",
                                       @force_ruby_platform])
    end

    def check_entry_point(entry_point_root)
      fs_entry_point = File.join(entry_point_root, @fs_entrance)
      puts "   ... target entry point will be at #{File.join(@fs_mount_point, fs_entry_point)}"

      return if File.exist?(File.join(@target_dir, fs_entry_point))

      raise Tebako::Error.new("Entry point #{fs_entry_point} does not exist or is not accessible", 106)
    end

    def check_cwd
      return if @cwd.nil?

      cwd_full = File.join(@target_dir, @cwd)
      return if File.directory?(cwd_full)

      raise Tebako::Error.new("Package working directory #{@cwd} does not exist", 108)
    end

    def collect_and_deploy_gem(gemspec)
      puts "   ... collecting gem from gemspec #{gemspec}"

      copy_files(@pre_dir)

      Dir.chdir(@pre_dir) do
        # spec = Bundler.load_gemspec(gemspec)
        # puts spec.executables.first unless spec.executables.empty?
        # puts spec.bindir

        BuildHelpers.run_with_capture_v([@gem_command, "build", gemspec])
        install_all_gems_or_fail
      end

      check_entry_point("bin")
    end

    def collect_and_deploy_gem_and_gemfile(gemspec)
      puts "   ... collecting gem from gemspec #{gemspec} and Gemfile"

      copy_files(@pre_dir)

      Dir.chdir(@pre_dir) do
        bundle_config
        puts "   *** It may take a long time for a big project. It takes REALLY long time on Windows ***"
        BuildHelpers.run_with_capture_v([@bundler_command, "install", "--jobs=#{@ncores}"])
        BuildHelpers.run_with_capture_v([@bundler_command, "exec", @gem_command, "build", gemspec])
        install_all_gems_or_fail
      end

      check_entry_point("bin")
    end

    def configure_commands
      if msys?
        configure_commands_msys
      else
        configure_commands_not_msys
      end

      @gem_command = File.join(@tbd, "gem#{@cmd_suffix}")
      @bundler_command = File.join(@tbd, "bundle#{@bat_suffix}")
    end

    def configure_commands_msys
      @cmd_suffix = ".cmd"
      @bat_suffix = ".bat"
      @force_ruby_platform = "true"
      @nokogiri_option = "--use-system-libraries"
    end

    def configure_commands_not_msys
      @cmd_suffix = ""
      @bat_suffix = ""
      @force_ruby_platform = "false"
      @nokogiri_option = "--no-use-system-libraries"
    end

    def copy_files(dest)
      FileUtils.mkdir_p(dest)
      if Dir.exist?(@fs_root) && File.readable?(@fs_root)
        begin
          FileUtils.cp_r(File.join(@fs_root, "."), dest)
        rescue StandardError
          raise Tebako::Error.new("#{@fs_root} does not exist or is not accessible.", 107)
        end
        return
      end
      raise Tebako::Error.new("#{@fs_root} is not accessible or is not a directory.", 107)
    end

    def deploy_gem(gem)
      puts "   ... installing Ruby gem from #{gem}"
      copy_files(@pre_dir)
      Dir.chdir(@pre_dir) { install_gem(gem) }
      check_entry_point("bin")
    end

    def deploy_gemfile
      puts "   ... deploying Gemfile"
      copy_files(@tld)

      Dir.chdir(@tld) do
        bundle_config
        puts "   *** It may take a long time for a big project. It takes REALLY long time on Windows ***"
        BuildHelpers.run_with_capture_v([@bundler_command, "install", "--jobs=#{@ncores}"])
      end

      check_entry_point("local")
    end

    def deploy_simple_script
      puts "   ... collecting simple Ruby script from #{@fs_root}"
      copy_files(@tld)
      check_entry_point("local")
    end

    def deploy_solution # rubocop:disable Metrics/MethodLength
      case @scenario
      when :simple_script
        deploy_simple_script
      when :gem
        deploy_gem(Dir.glob(File.join(@fs_root, "*.gem")).first)
      when :gemfile
        deploy_gemfile
      when :gemspec
        collect_and_deploy_gem(Dir.glob(File.join(@fs_root, "*.gemspec")).first)
      when :gemspec_and_gemfile
        collect_and_deploy_gem_and_gemfile(Dir.glob(File.join(@fs_root, "*.gemspec")).first)
      end
    end

    def install_all_gems_or_fail
      gem_files = Dir.glob("*.gem").map { |file| File.expand_path(file) }
      raise Tebako::Error, "No gem files found after build" if gem_files.empty?

      gem_files.each { |gem_file| install_gem(gem_file) }
    end
  end
end
