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
  class RuntimeDeployer # rubocop:disable Metrics/ClassLength
    DRIVER_IMAGE = "deploy-driver.dwarfs"
    DRIVER_PACKAGE = "deploy-driver.pkg"
    EMPTY_BASE = "deploy-driver.base"
    BUNDLE_EXEC_SCRIPT_NAME = "bundle_exec.rb"

    # Toolchain candidates for mkmf/cmake builds inside the deploy driver:
    # the runtime's recorded tool first (a no-op when it exists), then an
    # available equivalent. CC/CXX prefer clang (the recorded flags are
    # clang-flavored); the llvm tools fall back to binutils.
    CLANG_VERSIONS = %w[20 19 18 17 16 15 14 13 12 11].freeze
    TOOLCHAIN_FALLBACKS = {
      "CC" => %w[clang cc gcc],
      "CXX" => ["clang++", "c++", "g++"],
      "AR" => %w[ar],
      "RANLIB" => %w[ranlib],
      "NM" => %w[nm],
      "OBJDUMP" => %w[objdump],
      "OBJCOPY" => %w[objcopy]
    }.freeze
    TOOLCHAIN_ENV_KEYS = TOOLCHAIN_FALLBACKS.keys.freeze

    class << self
      # [recorded tool, fallbacks...] for +key+ (one of TOOLCHAIN_ENV_KEYS)
      def tool_candidates(key, recorded)
        first, *rest = TOOLCHAIN_FALLBACKS[key]
        llvm_name = if key == "CC"
                      "clang"
                    else
                      (key == "CXX" ? "clang++" : "llvm-#{key.downcase}")
                    end
        [recorded, first, *CLANG_VERSIONS.map { |version| "#{llvm_name}-#{version}" }, *rest]
      end
    end

    def initialize(runtime_path, deps_bin_dir, staging_bin_dir, fs_mount_point, ruby_ver)
      @runtime_path = runtime_path
      @deps_bin_dir = deps_bin_dir
      @staging_bin_dir = staging_bin_dir
      @fs_mount_point = fs_mount_point
      @ruby_ver = ruby_ver
      # The deploy ruby shim re-enters the driver image through a POSIX
      # shell exec; msys has no such exec path (native extension builds on
      # Windows are out of the shim's reach)
      @shim_supported = !Gem.win_platform?
    end

    # ops: array of deploy directives, executed in order
    #   ["chdir", dir]                 -- Dir.chdir(dir)
    #   ["gem", argv]                  -- Gem::GemRunner.run(argv)
    #   ["bundle", version|nil, argv]  -- activate bundler (pinned when
    #                                     version given) and run its CLI
    #   ["bundle_exec", version|nil, argv]
    #                                  -- Bundler.setup + the gem command in
    #                                     a fresh re-exec (the 'bundle exec'
    #                                     environment; a default gem already
    #                                     activated in the driver process
    #                                     cannot be re-activated at the
    #                                     bundle's version)
    #   ["install_all", dir, argv]     -- gem install every *.gem in dir
    # env: GEM_HOME/GEM_PATH/GEM_SPEC_CACHE/SSL_CERT_* for the deploy; it
    # travels in the process environment because Gem::PathSupport snapshots
    # it at interpreter boot (setting it in the driver script would be too
    # late). TEBAKO_PASS_THROUGH joins it: the tebako-patched rubygems
    # filters gem paths to the memfs mount point unless it is set, and the
    # driver installs into the packaging environment on the host. The
    # resolved toolchain is exported for subprocess builds that never read
    # rbconfig (mini_portile/cmake: "CMAKE_C_COMPILER not set"), without
    # overriding CC/CXX & co the user set in their own environment
    def execute(ops, env, seed_dir, verbose: false)
      write_driver(seed_dir, ops)
      Tebako::Packager.mkdwarfs(@deps_bin_dir, driver_image, seed_dir)
      stitch_driver_package
      write_bundle_exec_script if @shim_supported
      write_ruby_shim if @shim_supported
      out = BuildHelpers.with_env(toolchain_env.merge(env).merge("TEBAKO_PASS_THROUGH" => "1")) do
        BuildHelpers.run_with_capture([@runtime_path, "--tebako-image", driver_image_ref])
      end
      puts out if verbose
    end

    private

    # Toolchain for the driver process environment: the resolved tool per
    # key, but only for keys the user has not already set in their own
    # environment (explicit user CC/CXX/... wins). POSIX-only, like the
    # in-driver cc_override: on Windows the deploy driver relies on the
    # runtime's own recorded toolchain (and the command -v probe has no
    # shell to run in anyway)
    def toolchain_env
      return {} unless @shim_supported

      TOOLCHAIN_ENV_KEYS.filter_map do |key|
        next unless ENV[key].nil? || ENV[key].empty?

        tool = first_tool(self.class.tool_candidates(key, RbConfig::CONFIG[key]))
        [key, tool] unless tool.nil?
      end.to_h
    end

    def first_tool(candidates)
      candidates.find { |candidate| !candidate.to_s.empty? && system("command -v #{candidate} >/dev/null 2>&1") }
    end

    def bundle_exec_script
      File.join(@staging_bin_dir, BUNDLE_EXEC_SCRIPT_NAME)
    end

    # mkmf-driven native extension builds spawn RbConfig.ruby / Gem.ruby as
    # a subprocess (the extconf.rb run) and compile against rubyhdrdir, both
    # stripped from the runtime image. The shim re-enters the driver image
    # for the spawn; the runtime SDK provides the headers.
    def sdk_root
      return nil unless @shim_supported

      @sdk_root ||= Tebako::RuntimeSdk.resolve(@runtime_path, File.dirname(@deps_bin_dir), @ruby_ver)
    end

    def driver_image
      File.join(@staging_bin_dir, DRIVER_IMAGE)
    end

    def driver_package
      File.join(@staging_bin_dir, DRIVER_PACKAGE)
    end

    def driver_image_ref
      "#{driver_package}:0:#{@fs_mount_point}"
    end

    # mkmf-driven native extension builds spawn RbConfig.ruby / Gem.ruby as
    # a subprocess (the extconf.rb run). The runtime image ships no bin/ruby,
    # so the driver's bindir points at this host shim: it re-enters the
    # driver image with the script as argument -- the same launcher-ABI
    # handoff the driver itself was started with (the stub runs it in
    # script mode)
    def write_ruby_shim
      File.write(ruby_shim_path, <<~SHIM)
        #!/bin/sh
        TEBAKO_DEPLOY_BINDIR="$(dirname "$0")"; export TEBAKO_DEPLOY_BINDIR
        exec "#{@runtime_path}" --tebako-image "#{driver_image_ref}" --tebako-entry ruby "$@"
      SHIM
      FileUtils.chmod(0o755, ruby_shim_path)
    end

    def ruby_shim_path
      File.join(@staging_bin_dir, "ruby")
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
        require "tmpdir"

        BUNDLE_EXEC_SCRIPT = #{bundle_exec_script.inspect}

        #{build_overrides}if ARGV.any?
          # Script mode: mkmf-driven extension builds spawn the ruby at
          # RbConfig's bindir (the host shim); the shim re-enters this image
          # with the script as argument. mkmf derives srcdir from $0, so the
          # script takes over the program name before it is loaded.
          $0 = ARGV.first
          load ARGV.shift
        else
        #{driver_body(ops)}
        end
      RUBY
    end

    # Companion to the driver's 'bundle_exec' directive: runs in the fresh
    # interpreter the shim re-enters (a default gem already activated in the
    # driver process cannot be re-activated at the bundle's version)
    def write_bundle_exec_script # rubocop:disable Metrics/MethodLength
      File.write(bundle_exec_script, <<~RUBY)
        # THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO. DO NOT CHANGE IT, PLEASE
        version = ARGV.shift
        gem "bundler", version unless version.nil? || version.empty?
        require "bundler"
        Bundler.setup
        require "rubygems"
        require "rubygems/gem_runner"
        begin
          Gem::GemRunner.new.run(ARGV)
        rescue SystemExit => e
          exit(e.status)
        end
      RUBY
    end

    # extconf/make recipes spawn RbConfig.ruby / Gem.ruby and compile
    # against rubyhdrdir; point bindir at the host shim and the header dirs
    # at the runtime SDK. mkmf's link probes expand $(LIBRUBYARG) only for
    # throwaway executables, so it receives the SDK's symbol stub archive --
    # true yes/no resolution; the shipped extension .so never links it and
    # resolves against the runtime executable (which exports the symbols) at
    # load time. mkmf reads MAKEFILE_CONFIG, rubygems reads CONFIG -- both
    # take the overrides.
    def build_overrides # rubocop:disable Metrics/AbcSize
      return "" unless @shim_supported

      lines = ["[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|"]
      lines << "  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", #{@staging_bin_dir.inspect})"
      return "#{lines.join("\n")}\nend\n" if sdk_root.nil?

      lines << "  tg_config[\"rubyhdrdir\"] = #{File.join(sdk_root, "include").inspect}"
      lines << "  tg_config[\"rubyarchhdrdir\"] = #{File.join(sdk_root, "archhdr").inspect}"
      lines << "  tg_config[\"LIBRUBYARG\"] = #{File.join(sdk_root, "lib", "libruby-stub.a").inspect}"
      lines << "  tg_config[\"EXTDLDFLAGS\"] = \"\""
      "#{lines.join("\n")}\n#{cc_override}end\n"
    end

    # The recorded toolchain comes from the runtime's build machine (an
    # LLVM release); when it is not installed on the press host, mkmf probes
    # and bundled-library links die at shell level ("The compiler failed to
    # generate an executable file", "command not found"). Fall back to the
    # first available equivalent: newer/older clang for the compilers
    # (recorded flags are clang-flavored), binutils for the llvm tools
    def cc_override # rubocop:disable Metrics/MethodLength
      lines = ["def tg_first_tool(*candidates)",
               "  candidates.find { |tg_c| !tg_c.to_s.empty? && system(\"command -v \#{tg_c} >/dev/null 2>&1\") }",
               "end",
               ""]
      lines << "{"
      TOOLCHAIN_ENV_KEYS.each do |key|
        recorded = key == "NM" ? "RbConfig::CONFIG[\"NM\"].to_s.split.first" : "RbConfig::CONFIG[#{key.inspect}]"
        lines << "  #{key.inspect} => [#{override_candidates(key, recorded)}],"
      end
      lines << "}.each do |tg_key, tg_candidates|"
      lines << "  tg_tool = tg_first_tool(*tg_candidates)"
      lines << "  next if tg_tool.nil?"
      lines << "  [RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each { |tg_config| tg_config[tg_key] = tg_tool }"
      lines << "end"
      "#{lines.join("\n")}\n"
    end

    # The emitted candidate list for +key+: the recorded tool as a code
    # reference (it reads the runtime's rbconfig inside the driver), then the
    # literal fallbacks
    def override_candidates(key, recorded)
      self.class.tool_candidates(key, recorded).map { |c| c.start_with?("RbConfig::") ? c : c.inspect }.join(", ")
    end

    def driver_body(ops)
      <<~RUBY
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

        # rubygems commands end with terminate_interaction(0) on success,
        # which raises Gem::SystemExitException (< SystemExit) and would end
        # this process before the remaining operations; bundler exits the
        # same way on failure. Guard both and re-raise on non-zero status.
        def tg_run_gem(args)
          puts "   ... @ gem \#{args.join(" ")}"
          begin
            Gem::GemRunner.new.run(args)
          rescue SystemExit => e
            raise "gem \#{args.first} failed (exit \#{e.status})" unless e.status.zero?
          end
          # Gems this operation installed must be visible to the following
          # ones (rubygems caches the spec index at interpreter boot)
          Gem::Specification.reset
        end

        def tg_run_bundle(version, args)
          puts "   ... @ bundle \#{args.join(" ")}"
          gem "bundler", version unless version.nil?
          ARGV.replace(args)
          begin
            load Gem.bin_path("bundler", "bundle")
          rescue SystemExit => e
            raise "bundle \#{args.first} failed (exit \#{e.status})" unless e.status.zero?
          end
        end

        # 'bundle exec' needs a fresh process: the driver itself may already
        # have activated a default gem at another version (openssl for the
        # fetch above), and a gem cannot be re-activated at the bundle's
        # version. The shim re-enters this image with the companion script
        # in a clean interpreter.
        def tg_bundle_exec(version, argv)
          puts "   ... @ bundle exec \#{argv.join(" ")}"
          raise "bundle exec \#{argv.first} failed" unless system(RbConfig.ruby, BUNDLE_EXEC_SCRIPT, version.to_s, *argv)
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

    def op_line(step) # rubocop:disable Metrics/AbcSize
      case step[0]
      when "chdir" then "Dir.chdir(#{step[1].inspect})"
      when "gem" then "tg_run_gem(#{step[1].inspect})"
      when "bundle" then "tg_run_bundle(#{step[1].inspect}, #{step[2].inspect})"
      when "bundle_exec" then "tg_bundle_exec(#{step[1].inspect}, #{step[2].inspect})"
      when "install_all" then "tg_install_all(#{step[1].inspect}, #{step[2].inspect})"
      else
        raise Tebako::Error, "Internal error: unknown deploy directive '#{step[0]}'"
      end
    end
  end
end
