# frozen_string_literal: true

# Copyright (c) 2024-2025 [Ribose Inc](https://www.ribose.com).
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

RSpec.describe Tebako::DeployHelper do
  let(:ruby_ver) { Tebako::RubyVersion.new("3.3.7") }
  let(:api_ver) { ruby_ver.api_version }
  let(:deployer) { double("deployer") }

  around do |example|
    Dir.mktmpdir do |tmp|
      @tmp = tmp
      example.run
    end
  end

  let(:fs_root) { File.join(@tmp, "root") }
  let(:target_dir) { File.join(@tmp, "target") }
  let(:pre_dir) { File.join(@tmp, "pre") }
  let(:deploy_helper) { Tebako::DeployHelper.new(fs_root, "hello.rb", target_dir, pre_dir) }

  def seed_runtime_gem
    spec_dir = File.join(target_dir, "lib", "ruby", "gems", api_ver, "specifications")
    FileUtils.mkdir_p(spec_dir)
    File.write(File.join(spec_dir, "tebako-runtime-0.7.0.gemspec"), "# stub")
  end

  def configure_deploy_helper(cwd = nil)
    seed_runtime_gem
    deploy_helper.configure(ruby_ver, cwd)
  end

  def captured_ops
    ops = nil
    allow(deployer).to receive(:execute) { |*args| ops = args[0] }
    yield if block_given?
    ops
  end

  describe "#configure" do
    before { FileUtils.mkdir_p(fs_root) }

    it "exposes the gem home for the runtime's api version" do
      seed_runtime_gem
      deploy_helper.configure(ruby_ver, nil)
      expect(deploy_helper.gem_home).to eq(File.join(target_dir, "lib", "ruby", "gems", api_ver))
    end
  end

  describe "#deploy" do
    context "when the seeded environment does not carry the tebako-runtime gem" do
      before { FileUtils.mkdir_p(fs_root) }

      it "fails with error 129" do
        deploy_helper.configure(ruby_ver, nil)
        expect { deploy_helper.deploy(deployer) }
          .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(129) }
      end
    end

    context "with a simple script scenario" do
      before do
        FileUtils.mkdir_p(fs_root)
        File.write(File.join(fs_root, "hello.rb"), "puts 'hello'\n")
      end

      it "stages the script and runs no deploy operations" do
        configure_deploy_helper
        expect(deployer).not_to receive(:execute)
        deploy_helper.deploy(deployer)
        expect(File.read(File.join(target_dir, "local", "hello.rb"))).to eq("puts 'hello'\n")
      end

      it "fails with error 106 when the entry point is missing" do
        FileUtils.rm(File.join(fs_root, "hello.rb"))
        File.write(File.join(fs_root, "other.rb"), "# other\n")
        configure_deploy_helper
        expect { deploy_helper.deploy(deployer) }
          .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(106) }
      end

      it "fails with error 108 when the package working directory is missing" do
        configure_deploy_helper("no/such/dir")
        expect { deploy_helper.deploy(deployer) }
          .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(108) }
      end

      it "accepts an existing package working directory" do
        configure_deploy_helper("work")
        FileUtils.mkdir_p(File.join(target_dir, "work"))
        allow(deployer).to receive(:execute)
        expect { deploy_helper.deploy(deployer) }.not_to raise_error
      end
    end

    context "with a gemfile scenario" do
      before do
        FileUtils.mkdir_p(fs_root)
        File.write(File.join(fs_root, "hello.rb"), "puts 'hello'\n")
        File.write(File.join(fs_root, "Gemfile"), "source 'https://rubygems.org'\n")
      end

      it "collects bundle config and install operations with the default bundler" do
        configure_deploy_helper
        tld = File.join(target_dir, "local")
        ops = captured_ops { deploy_helper.deploy(deployer) }

        expect(ops).to eq(
          [["chdir", tld],
           ["bundle", nil, ["config", "set", "--local", "build.ffi", "--disable-system-libffi"]],
           ["bundle", nil, ["config", "set", "--local", "build.nokogiri", "--no-use-system-libraries"]],
           ["bundle", nil, ["config", "set", "--local", "force_ruby_platform", "true"]],
           ["bundle", nil, ["install", "--jobs=#{Tebako::ScenarioManagerBase.new.ncores}"]]]
        )
      end

      it "passes the gem environment and the target dir to the deployer" do
        configure_deploy_helper
        expect(deployer).to receive(:execute) do |ops, env, seed_dir, verbose:|
          expect(seed_dir).to eq(target_dir)
          expect(verbose).to be(false)
          expect(env).to eq(deploy_helper.deploy_env)
          expect(ops).not_to be_empty
        end
        deploy_helper.deploy(deployer)
      end

      context "when the lockfile pins a bundler version" do
        before do
          File.write(File.join(fs_root, "Gemfile.lock"), "BUNDLED WITH\n   2.4.22\n")
        end

        it "installs and activates the pinned bundler" do
          configure_deploy_helper
          tgd = File.join(target_dir, "lib", "ruby", "gems", api_ver)
          tbd = File.join(target_dir, "bin")
          ops = captured_ops { deploy_helper.deploy(deployer) }

          expect(ops.first).to eq(
            ["gem", ["install", "bundler", "-v", "2.4.22", "--no-document", "--install-dir", tgd, "--bindir", tbd]]
          )
          expect(ops).to include(["bundle", "2.4.22", ["install", "--jobs=#{Tebako::ScenarioManagerBase.new.ncores}"]])
        end
      end
    end

    context "with a gem scenario" do
      before do
        FileUtils.mkdir_p(fs_root)
        File.write(File.join(fs_root, "app.gem"), "gem bytes")
      end

      it "collects the gem install operation" do
        configure_deploy_helper
        tgd = File.join(target_dir, "lib", "ruby", "gems", api_ver)
        tbd = File.join(target_dir, "bin")
        gem_file = File.join(fs_root, "app.gem")
        allow(deployer).to receive(:execute) do
          FileUtils.mkdir_p(tbd)
          File.write(File.join(tbd, "hello.rb"), "# binstub\n")
        end

        expect { deploy_helper.deploy(deployer) }.not_to raise_error

        expect(deployer).to have_received(:execute) do |ops, _env, _seed, verbose:|
          expect(ops).to eq(
            [["chdir", pre_dir],
             ["gem", ["install", gem_file, "--no-document", "--install-dir", tgd, "--bindir", tbd]]]
          )
          expect(verbose).to be(false)
        end
      end

      it "fails with error 106 when the installed gem provides no entry point" do
        configure_deploy_helper
        allow(deployer).to receive(:execute)
        expect { deploy_helper.deploy(deployer) }
          .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(106) }
      end
    end

    context "with a gemspec scenario" do
      before do
        FileUtils.mkdir_p(fs_root)
        File.write(File.join(fs_root, "app.gemspec"), "# gemspec")
      end

      it "collects the gem build and install-all operations" do
        configure_deploy_helper
        tgd = File.join(target_dir, "lib", "ruby", "gems", api_ver)
        tbd = File.join(target_dir, "bin")
        gemspec = File.join(fs_root, "app.gemspec")
        allow(deployer).to receive(:execute) do
          FileUtils.mkdir_p(tbd)
          File.write(File.join(tbd, "hello.rb"), "# binstub\n")
        end

        expect { deploy_helper.deploy(deployer) }.not_to raise_error

        expect(deployer).to have_received(:execute) do |ops, _env, _seed, verbose:|
          expect(ops).to eq(
            [["chdir", pre_dir],
             ["gem", ["build", gemspec]],
             ["install_all", pre_dir, ["--no-document", "--install-dir", tgd, "--bindir", tbd]]]
          )
          expect(verbose).to be(false)
        end
      end
    end

    context "with a gemspec and gemfile scenario" do
      before do
        FileUtils.mkdir_p(fs_root)
        File.write(File.join(fs_root, "app.gemspec"), "# gemspec")
        File.write(File.join(fs_root, "Gemfile"), "source 'https://rubygems.org'\n")
      end

      it "collects bundler, bundle-exec build and install-all operations" do
        configure_deploy_helper
        tgd = File.join(target_dir, "lib", "ruby", "gems", api_ver)
        tbd = File.join(target_dir, "bin")
        gemspec = File.join(fs_root, "app.gemspec")
        allow(deployer).to receive(:execute) do
          FileUtils.mkdir_p(tbd)
          File.write(File.join(tbd, "hello.rb"), "# binstub\n")
        end

        expect { deploy_helper.deploy(deployer) }.not_to raise_error

        expect(deployer).to have_received(:execute) do |ops, _env, _seed, verbose:|
          expect(ops).to eq(
            [["chdir", pre_dir],
             ["bundle", nil, ["config", "set", "--local", "build.ffi", "--disable-system-libffi"]],
             ["bundle", nil, ["config", "set", "--local", "build.nokogiri", "--no-use-system-libraries"]],
             ["bundle", nil, ["config", "set", "--local", "force_ruby_platform", "true"]],
             ["bundle", nil, ["install", "--jobs=#{Tebako::ScenarioManagerBase.new.ncores}"]],
             ["bundle", nil, ["exec", "gem", "build", gemspec]],
             ["install_all", pre_dir, ["--no-document", "--install-dir", tgd, "--bindir", tbd]]]
          )
          expect(verbose).to be(false)
        end
      end
    end
  end

  describe "#deploy_env" do
    before { FileUtils.mkdir_p(fs_root) }

    it "returns the gem environment for the deploy driver" do
      configure_deploy_helper
      expect(deploy_helper.deploy_env).to eq(
        "GEM_HOME" => deploy_helper.gem_home,
        "GEM_PATH" => deploy_helper.gem_home,
        "GEM_SPEC_CACHE" => File.join(target_dir, "spec_cache"),
        "SSL_CERT_FILE" => OpenSSL::X509::DEFAULT_CERT_FILE,
        "SSL_CERT_DIR" => OpenSSL::X509::DEFAULT_CERT_DIR
      )
    end
  end
end

# rubocop:enable Metrics/BlockLength
