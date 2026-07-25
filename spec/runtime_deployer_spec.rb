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
require "open3"
require "tmpdir"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::RuntimeDeployer do
  let(:ruby_ver) { Tebako::RubyVersion.new("3.3.7") }
  let(:runtime_path) { "/cached/runtime/tebako-runtime-0.15.9-3.3.7-macos-arm64" }
  let(:deployer) { described_class.new(runtime_path, "/deps/bin", staging_dir, "/__tebako_memfs__", ruby_ver) }
  let(:env) { { "GEM_HOME" => "/target/lib/ruby/gems/3.3.0", "GEM_PATH" => "/target/lib/ruby/gems/3.3.0" } }

  around do |example|
    Dir.mktmpdir do |tmp|
      @tmp = tmp
      example.run
    end
  end

  let(:staging_dir) { File.join(@tmp, "p") }
  let(:seed_dir) { File.join(@tmp, "s") }

  before do
    FileUtils.mkdir_p(staging_dir)
    FileUtils.mkdir_p(File.join(seed_dir, "local"))
    allow(Tebako::Packager).to receive(:mkdwarfs)
    allow(Tebako::Stitcher).to receive(:stitch)
    allow(Tebako::BuildHelpers).to receive(:run_with_capture).and_return("")
    allow(Tebako::RuntimeSdk).to receive(:resolve).and_return("/sdk/ruby-3.3.7")
  end

  describe "#execute" do
    # Run the example with AR unset and CC/CXX user-set, restoring the real
    # environment afterwards
    def with_toolchain_env
      user_cc = ENV.fetch("CC", nil)
      user_cxx = ENV.fetch("CXX", nil)
      ENV.delete("AR")
      ENV["CC"] = "user-cc"
      ENV["CXX"] = "user-c++"
      yield
    ensure
      ENV["CC"] = user_cc
      ENV["CXX"] = user_cxx
    end

    it "builds the driver image from the seeded environment" do
      deployer.execute([], env, seed_dir)
      expect(Tebako::Packager).to have_received(:mkdwarfs)
        .with("/deps/bin", File.join(staging_dir, "deploy-driver.dwarfs"), seed_dir)
    end

    it "stitches the driver image onto an empty base with a lean trailer" do
      deployer.execute([], env, seed_dir)
      expect(Tebako::Stitcher).to have_received(:stitch) do |base, images:, output:, **kwargs|
        expect(File.read(base)).to eq("")
        expect(images).to eq([{ path: File.join(staging_dir, "deploy-driver.dwarfs"),
                                mount_point: "/__tebako_memfs__",
                                format_id: Tebako::Stitcher::FORMAT_DWARFS }])
        expect(output).to eq(File.join(staging_dir, "deploy-driver.pkg"))
        expect(kwargs[:lean]).to be(true)
        expect(kwargs[:ruby_version]).to eq("3.3.7")
        expect(kwargs[:launcher_abi]).to eq(Tebako::LauncherAbi::VERSION)
      end
    end

    it "execs the runtime with the driver image handoff" do
      deployer.execute([], env, seed_dir)
      expect(Tebako::BuildHelpers).to have_received(:run_with_capture)
        .with([runtime_path, "--tebako-image",
               "#{File.join(staging_dir, "deploy-driver.pkg")}:0:/__tebako_memfs__"])
    end

    it "passes the deploy environment and pass-through flag to the runtime process" do
      expect(Tebako::BuildHelpers).to receive(:with_env) do |env_hash, &block|
        expect(env_hash).to include(env.merge("TEBAKO_PASS_THROUGH" => "1"))
        block.call
      end
      deployer.execute([], env, seed_dir)
    end

    # Toolchain resolution is POSIX-only (the command -v probe has no shell
    # on Windows, matching the POSIX-only in-driver overrides)
    it "exports a resolved toolchain for subprocess builds, without overriding user-set variables",
       unless: Gem.win_platform? do
      with_toolchain_env do
        expect(Tebako::BuildHelpers).to receive(:with_env) do |env_hash, &block|
          # user-set CC/CXX are not clobbered: they reach the driver by
          # environment inheritance, not through the override hash
          expect(env_hash).not_to have_key("CC")
          expect(env_hash).not_to have_key("CXX")
          expect(env_hash["AR"]).not_to be_nil
          expect(env_hash["RANLIB"]).not_to be_nil
          block.call
        end
        deployer.execute([], env, seed_dir)
      end
    end

    it "exports no toolchain variables on Windows", if: Gem.win_platform? do
      with_toolchain_env do
        expect(Tebako::BuildHelpers).to receive(:with_env) do |env_hash, &block|
          expect(env_hash).not_to have_key("CC")
          expect(env_hash).not_to have_key("CXX")
          expect(env_hash).not_to have_key("AR")
          expect(env_hash).not_to have_key("RANLIB")
          block.call
        end
        deployer.execute([], env, seed_dir)
      end
    end
  end

  describe "driver script generation" do
    def generated_driver(ops)
      deployer.execute(ops, env, seed_dir)
      File.read(File.join(seed_dir, "local", "stub.rb"))
    end

    it "serializes every deploy directive in order" do
      ops = [["chdir", "/target/local"],
             ["gem", ["install", "bundler", "-v", "2.4.22"]],
             ["bundle", "2.4.22", ["install", "--jobs=8"]],
             ["bundle_exec", "2.4.22", ["build", "app.gemspec"]],
             ["install_all", "/pre/dir", ["--no-document"]]]
      driver = generated_driver(ops)

      expect(driver).to include('Dir.chdir("/target/local")')
      expect(driver).to include('tg_run_gem(["install", "bundler", "-v", "2.4.22"])')
      expect(driver).to include('tg_run_bundle("2.4.22", ["install", "--jobs=8"])')
      expect(driver).to include('tg_bundle_exec("2.4.22", ["build", "app.gemspec"])')
      expect(driver).to include('tg_install_all("/pre/dir", ["--no-document"])')
      expect(driver.index('Dir.chdir("/target/local")')).to be < driver.index('tg_run_gem(["install", "bundler"')
      expect(driver.index('tg_run_gem(["install", "bundler"')).to be < driver.index('tg_install_all("/pre/dir"')
    end

    # The deploy ruby shim (and everything depending on it: bindir/SDK
    # overrides, the toolchain fallback, the companion script) exists only on
    # POSIX platforms; on Windows none of it is emitted
    it "writes the bundle_exec companion script", unless: Gem.win_platform? do
      deployer.execute([["bundle_exec", nil, ["build", "app.gemspec"]]], env, seed_dir)
      script = File.join(staging_dir, "bundle_exec.rb")

      expect(File.file?(script)).to be(true)
      expect(File.read(script)).to include("Bundler.setup")
    end

    it "does not write the bundle_exec companion script on Windows", if: Gem.win_platform? do
      deployer.execute([["bundle_exec", nil, ["build", "app.gemspec"]]], env, seed_dir)

      expect(File.exist?(File.join(staging_dir, "bundle_exec.rb"))).to be(false)
    end

    it "guards gem and bundle operations against the status-0 exit rubygems ends with" do
      driver = generated_driver([["gem", ["--version"]]])

      expect(driver).to include("rescue SystemExit => e")
      expect(driver).to include("unless e.status.zero?")
    end

    it "points the driver's bindir at the host shim and dispatches script mode", unless: Gem.win_platform? do
      driver = generated_driver([])

      expect(driver).to include("[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|")
      expect(driver).to include(%(tg_config["bindir"] = ENV.fetch("TEBAKO_DEPLOY_BINDIR", "#{staging_dir}")))
      expect(driver).to include("$0 = ARGV.first")
      expect(driver).to include("load ARGV.shift")
    end

    it "points the driver's header dirs at the runtime SDK and links probes against the symbol stub",
       unless: Gem.win_platform? do
      driver = generated_driver([])

      expect(driver).to include('tg_config["rubyhdrdir"] = "/sdk/ruby-3.3.7/include"')
      expect(driver).to include('tg_config["rubyarchhdrdir"] = "/sdk/ruby-3.3.7/archhdr"')
      expect(driver).to include('tg_config["LIBRUBYARG"] = "/sdk/ruby-3.3.7/lib/libruby-stub.a"')
      expect(driver).to include('tg_config["EXTDLDFLAGS"] = ""')
    end

    it "falls back to an available host toolchain when the recorded one is missing", unless: Gem.win_platform? do
      driver = generated_driver([])

      expect(driver).to include("def tg_first_tool(*candidates)")
      expect(driver).to include('"CC" => [RbConfig::CONFIG["CC"], "clang"')
      expect(driver).to include('"AR" => [RbConfig::CONFIG["AR"]')
      expect(driver).to include('"NM" => [RbConfig::CONFIG["NM"].to_s.split.first')
      expect(driver).to include("tg_config[tg_key] = tg_tool")
    end

    it "emits no shim-dependent overrides on Windows", if: Gem.win_platform? do
      driver = generated_driver([])

      expect(driver).not_to include("[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|")
      expect(driver).not_to include("def tg_first_tool")
      expect(driver).not_to include("tg_config[\"bindir\"]")
      expect(driver).not_to include("tg_config[\"rubyhdrdir\"]")
      expect(driver).to include("$0 = ARGV.first")
      expect(driver).to include("load ARGV.shift")
    end

    # Windows has no exec-bit concept the POSIX shim relies on (and the shim
    # itself is not written there); the exec check is POSIX-only
    it "writes an executable ruby shim that re-enters the driver image", unless: Gem.win_platform? do
      deployer.execute([], env, seed_dir)
      shim = File.join(staging_dir, "ruby")

      expect(File.executable?(shim)).to be(true)
      content = File.read(shim)
      expect(content).to include(%(exec "#{runtime_path}"))
      expect(content).to include(%(--tebako-image "#{File.join(staging_dir, "deploy-driver.pkg")}:0:/__tebako_memfs__"))
      expect(content).to include("--tebako-entry ruby")
    end

    it "does not write a ruby shim on Windows", if: Gem.win_platform? do
      deployer.execute([], env, seed_dir)

      expect(File.exist?(File.join(staging_dir, "ruby"))).to be(false)
    end

    it "continues after a successful gem command (the fontist deploy regression)" do
      ops = [["gem", ["--version"]], ["gem", ["--version"]]]
      generated_driver(ops)

      out = Open3.capture2e(Gem.ruby, File.join(seed_dir, "local", "stub.rb")).first
      expect(out.scan("... @ gem --version").size).to eq(2)
    end

    it "raises Tebako::Error for an unknown directive" do
      expect { deployer.execute([%w[frobnicate x]], env, seed_dir) }.to raise_error(Tebako::Error)
    end
  end
end

# rubocop:enable Metrics/BlockLength
