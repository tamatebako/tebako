# frozen_string_literal: true

# Copyright (c) 2026 [Ribose Inc](https://www.ribose.com).
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
  # Executes deploy operations (gem/bundler installs and builds) inside the
  # resolved prebuilt runtime.
  #
  # The prebuilt runtime packages are pressed in 'runtime' mode: their
  # compiled-in entry point is /local/stub.rb and their image carries a full
  # Ruby environment (stdlib, rubygems, bundler) but no bin/ tooling (it is
  # stripped at runtime press time). Deploy therefore cannot shell out to
  # bin/gem the way the legacy stash flow did; instead the operations the
  # DeployHelper collects are serialized into a driver script placed at
  # /local/stub.rb of a throwaway image, and the runtime itself is exec'd
  # with that image (--tebako-image, the launcher ABI handoff). The driver
  # runs with the runtime's own Ruby -- exactly the version/ABI the package
  # will run with -- and installs into the packaging environment through
  # absolute host paths (paths outside the memfs mount point reach the host
  # filesystem directly).
  class RuntimeDeployer
    DRIVER_IMAGE = "deploy-driver.dwarfs"
    DRIVER_PACKAGE = "deploy-driver.pkg"
    EMPTY_BASE = "deploy-driver.base"

    def initialize(runtime_path, deps_bin_dir, staging_bin_dir, fs_mount_point, ruby_ver)
      @runtime_path = runtime_path
      @deps_bin_dir = deps_bin_dir
      @staging_bin_dir = staging_bin_dir
      @fs_mount_point = fs_mount_point
      @ruby_ver = ruby_ver
    end

    # ops: array of deploy directives, executed in order
    #   ["chdir", dir]                 -- Dir.chdir(dir)
    #   ["gem", argv]                  -- Gem::GemRunner.run(argv)
    #   ["bundle", version|nil, argv]  -- activate bundler (pinned when
    #                                     version given) and run its CLI
    #   ["install_all", dir, argv]     -- gem install every *.gem in dir
    # env: GEM_HOME/GEM_PATH/GEM_SPEC_CACHE/SSL_CERT_* for the deploy; it
    # travels in the process environment because Gem::PathSupport snapshots
    # it at interpreter boot (setting it in the driver script would be too
    # late). TEBAKO_PASS_THROUGH joins it: the tebako-patched rubygems
    # filters gem paths to the memfs mount point unless it is set, and the
    # driver installs into the packaging environment on the host.
    def execute(ops, env, seed_dir, verbose: false)
      write_driver(seed_dir, ops)
      Tebako::Packager.mkdwarfs(@deps_bin_dir, driver_image, seed_dir)
      stitch_driver_package
      out = BuildHelpers.with_env(env.merge("TEBAKO_PASS_THROUGH" => "1")) do
        BuildHelpers.run_with_capture([@runtime_path, "--tebako-image", driver_image_ref])
      end
      puts out if verbose
    end

    private

    def driver_image
      File.join(@staging_bin_dir, DRIVER_IMAGE)
    end

    def driver_package
      File.join(@staging_bin_dir, DRIVER_PACKAGE)
    end

    def driver_image_ref
      "#{driver_package}:0:#{@fs_mount_point}"
    end

    def write_driver(seed_dir, ops)
      File.write(File.join(seed_dir, "local", "stub.rb"), driver_source(ops))
    end

    # The runtime reads the slot region referenced by the file's tpkg
    # trailer; the base bytes are irrelevant to the mount, so the package is
    # stitched onto an empty base
    def stitch_driver_package
      empty_base = File.join(@staging_bin_dir, EMPTY_BASE)
      File.write(empty_base, "")
      Tebako::Stitcher.stitch(empty_base,
                              images: [{ path: driver_image, mount_point: @fs_mount_point,
                                         format_id: Tebako::Stitcher::FORMAT_DWARFS }],
                              output: driver_package, lean: true,
                              ruby_version: @ruby_ver.ruby_version,
                              launcher_abi: Tebako::LauncherAbi::VERSION)
    end

    def driver_source(ops)
      <<~RUBY
        # THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO. DO NOT CHANGE IT, PLEASE
        require "rubygems"
        require "rubygems/gem_runner"
        require "rubygems/request"
        require "fileutils"

        # OpenSSL reads certificate files at the C level, where the memfs is
        # invisible; give rubygems and bundler host-side copies of the CA
        # certs vendored in the image
        TG_DEPLOY_CERT_DIR = File.join(ENV.fetch("GEM_SPEC_CACHE", Dir.mktmpdir), "ssl_certs")
        FileUtils.mkdir_p(TG_DEPLOY_CERT_DIR)

        module TebakoDeployCerts
          def get_cert_files
            super.map do |src|
              dst = File.join(TG_DEPLOY_CERT_DIR, File.basename(src))
              FileUtils.cp(src, dst) unless File.exist?(dst)
              dst
            end
          end
        end
        Gem::Request.singleton_class.prepend(TebakoDeployCerts)

        def tg_run_gem(args)
          puts "   ... @ gem \#{args.join(" ")}"
          Gem::GemRunner.new.run(args)
        end

        def tg_run_bundle(version, args)
          puts "   ... @ bundle \#{args.join(" ")}"
          gem "bundler", version unless version.nil?
          ARGV.replace(args)
          load Gem.bin_path("bundler", "bundle")
        end

        def tg_install_all(dir, args)
          gems = Dir.glob(File.join(dir, "*.gem"))
          raise "No gem files found after build" if gems.empty?

          gems.each { |gem_file| tg_run_gem(["install", gem_file] + args) }
        end

        #{op_lines(ops)}
      RUBY
    end

    def op_lines(ops)
      ops.map { |step| op_line(step) }.join("\n")
    end

    def op_line(step)
      case step[0]
      when "chdir" then "Dir.chdir(#{step[1].inspect})"
      when "gem" then "tg_run_gem(#{step[1].inspect})"
      when "bundle" then "tg_run_bundle(#{step[1].inspect}, #{step[2].inspect})"
      when "install_all" then "tg_install_all(#{step[1].inspect}, #{step[2].inspect})"
      else
        raise Tebako::Error, "Internal error: unknown deploy directive '#{step[0]}'"
      end
    end
  end
end
