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
  end

  describe "#execute" do
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

    it "prints the driver output in verbose mode" do
      allow(Tebako::BuildHelpers).to receive(:run_with_capture).and_return("driver says hi")
      expect { deployer.execute([], env, seed_dir, verbose: true) }.to output(/driver says hi/).to_stdout
    end

    it "passes the deploy environment and pass-through flag to the runtime process" do
      expect(Tebako::BuildHelpers).to receive(:with_env)
        .with(env.merge("TEBAKO_PASS_THROUGH" => "1")).and_call_original
      deployer.execute([], env, seed_dir)
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
             ["bundle", nil, ["exec", "gem", "build", "app.gemspec"]],
             ["install_all", "/pre/dir", ["--no-document"]]]
      driver = generated_driver(ops)

      expect(driver).to include('Dir.chdir("/target/local")')
      expect(driver).to include('tg_run_gem(["install", "bundler", "-v", "2.4.22"])')
      expect(driver).to include('tg_run_bundle("2.4.22", ["install", "--jobs=8"])')
      expect(driver).to include('tg_run_bundle(nil, ["exec", "gem", "build", "app.gemspec"])')
      expect(driver).to include('tg_install_all("/pre/dir", ["--no-document"])')
      expect(driver.index('Dir.chdir("/target/local")')).to be < driver.index('tg_run_gem(["install", "bundler"')
      expect(driver.index('tg_run_gem(["install", "bundler"')).to be < driver.index('tg_install_all("/pre/dir"')
    end

    it "raises Tebako::Error for an unknown directive" do
      expect { deployer.execute([%w[frobnicate x]], env, seed_dir) }.to raise_error(Tebako::Error)
    end
  end
end

# rubocop:enable Metrics/BlockLength
