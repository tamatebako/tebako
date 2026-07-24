# frozen_string_literal: true

# Copyright (c)  2024-2025 [Ribose Inc](https://www.ribose.com).
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

# require "bundler"
require "fileutils"
require "openssl"

# Tebako - an executable packager
module Tebako
  # Tebako packaging support (deployer)
  #
  # Stages the application into the packaging environment (host file system)
  # and collects the gem/bundler operations the scenario needs. The
  # operations run inside the resolved prebuilt runtime (Tebako::RuntimeDeployer)
  # because the runtime image ships no bin/ tooling to shell out to; the
  # tebako-runtime gem itself ships pre-installed in the runtime layout the
  # packaging environment is seeded from.
  class DeployHelper < ScenarioManagerWithBundler # rubocop:disable Metrics/ClassLength
    def initialize(fs_root, fs_entrance, target_dir, pre_dir)
      super(fs_root, fs_entrance)
      @fs_root = fs_root
      @fs_entrance = fs_entrance
      @target_dir = target_dir
      @pre_dir = pre_dir
      @verbose = %w[yes true].include?(ENV.fetch("VERBOSE", nil))
    end

    attr_reader :gem_home

    def configure(ruby_ver, cwd)
      @ruby_ver = ruby_ver
      @cwd = cwd

      @tbd = File.join(@target_dir, "bin")
      @tgd = @gem_home = File.join(@target_dir, "lib", "ruby", "gems", @ruby_ver.api_version)
      @tld = File.join(@target_dir, "local")

      configure_scenario
    end

    def deploy(deployer)
      verify_runtime_gem!

      ops = []
      ops << install_gem_op("bundler", @bundler_version) if @needs_bundler
      ops << ["gem", ["env"]] if @verbose
      deploy_solution(ops)
      deployer.execute(ops, deploy_env, @target_dir, verbose: @verbose) unless ops.empty?

      check_solution
      check_cwd
    end

    def deploy_env
      {
        "GEM_HOME" => gem_home,
        "GEM_PATH" => gem_home,
        "GEM_SPEC_CACHE" => File.join(@target_dir, "spec_cache"),
        # The runtime's OpenSSL carries the build machine's certificate
        # paths; the deploy driver fetches gems through the press host's
        # certificate store instead
        "SSL_CERT_FILE" => OpenSSL::X509::DEFAULT_CERT_FILE,
        "SSL_CERT_DIR" => OpenSSL::X509::DEFAULT_CERT_DIR
      }
    end

    private

    # The runtime layout the packaging environment is seeded from carries the
    # tebako-runtime gem pre-installed (the runtime contract); the legacy
    # flow installed it from rubygems.org at deploy time
    def verify_runtime_gem!
      return unless Dir.glob(File.join(@tgd, "specifications", "tebako-runtime-*.gemspec")).empty?

      Tebako.packaging_error(129, File.join(@tgd, "specifications"))
    end

    def install_gem_op(name, ver = nil)
      puts "   ... installing #{name} gem#{" version #{ver}" if ver}"

      argv = ["install", name.to_s]
      argv += ["-v", ver.to_s] if ver
      ["gem", argv + install_argv_tail]
    end

    def install_argv_tail
      tail = ["--no-document", "--install-dir", @tgd, "--bindir", @tbd]
      tail << "--verbose" if @verbose
      tail += ["--platform", "ruby"] if msys?
      tail
    end

    # The version the bundle ops activate, mirroring the legacy bundle
    # binstub reference ('_x.y.z_' when the scenario pinned a bundler
    # version, the runtime's default otherwise)
    def bundler_activation
      @needs_bundler ? @bundler_version : nil
    end

    def bundle_op(argv)
      ["bundle", bundler_activation, argv]
    end

    def bundle_config_ops
      bundle_config_option_ops(["build.ffi", "--disable-system-libffi"]) +
        bundle_config_option_ops(["build.nokogiri", @nokogiri_option]) +
        bundle_config_option_ops(["force_ruby_platform", @force_ruby_platform])
    end

    def bundle_config_option_ops(opt)
      [bundle_op(["config", "set", "--local"] + opt)]
    end

    def bundle_install_op
      puts "   *** It may take a long time for a big project. It takes REALLY long time on Windows ***"
      bundle_op(["install", "--jobs=#{ncores}"])
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

    def check_solution
      case @scenario
      when :simple_script, :gemfile
        check_entry_point("local")
      when :gem, :gemspec, :gemspec_and_gemfile
        check_entry_point("bin")
      end
    end

    def collect_and_deploy_gem(gemspec, ops)
      puts "   ... collecting gem from gemspec #{gemspec}"

      stage_pre_dir(ops)
      ops << ["gem", ["build", gemspec]]
      ops << ["install_all", @pre_dir, install_argv_tail]
    end

    def collect_and_deploy_gem_and_gemfile(gemspec, ops)
      puts "   ... collecting gem from gemspec #{gemspec} and Gemfile"

      stage_pre_dir(ops)
      ops.concat(bundle_config_ops)
      ops << bundle_install_op
      ops << bundle_op(["exec", "gem", "build", gemspec])
      ops << ["install_all", @pre_dir, install_argv_tail]
    end

    def configure_scenario
      super
      configure_commands
    end

    def configure_commands
      if msys?
        configure_commands_msys
      else
        configure_commands_not_msys
      end
    end

    def configure_commands_msys
      @force_ruby_platform = "true"
      @nokogiri_option = "--use-system-libraries"
    end

    def configure_commands_not_msys
      # Force the ruby (source) platform for gems: precompiled variants link
      # against shared system libraries (libffi & co.) that do not exist
      # inside a tebako package -- e.g. ffi-x86_64-linux-gnu fails to load in
      # the memfs (tebako#343). msys has always forced this.
      @force_ruby_platform = "true"
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

    def deploy_gem(gem, ops)
      puts "   ... installing Ruby gem from #{gem}"

      stage_pre_dir(ops)
      ops << ["gem", ["install", gem] + install_argv_tail]
    end

    def deploy_gemfile(ops)
      puts "   ... deploying Gemfile"

      copy_files(@tld)
      ops << ["chdir", @tld]
      ops.concat(bundle_config_ops)
      ops << bundle_install_op
    end

    def deploy_simple_script
      puts "   ... collecting simple Ruby script from #{@fs_root}"

      copy_files(@tld)
    end

    def deploy_solution(ops) # rubocop:disable Metrics/MethodLength
      case @scenario
      when :simple_script
        deploy_simple_script
      when :gem
        deploy_gem(Dir.glob(File.join(@fs_root, "*.gem")).first, ops)
      when :gemfile
        deploy_gemfile(ops)
      when :gemspec
        collect_and_deploy_gem(Dir.glob(File.join(@fs_root, "*.gemspec")).first, ops)
      when :gemspec_and_gemfile
        collect_and_deploy_gem_and_gemfile(Dir.glob(File.join(@fs_root, "*.gemspec")).first, ops)
      end
    end

    def stage_pre_dir(ops)
      copy_files(@pre_dir)
      ops << ["chdir", @pre_dir]
    end
  end
end
