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

require_relative "../lib/tebako/deploy_helper"
require "open3"
# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::DeployHelper do
  let(:fs_root) { "/fs/root" }
  let(:fs_entrance) { "/fs/root/entrance" }
  let(:fs_mount_point) { "/fs/mount_point" }
  let(:target_dir) { "/target/dir" }
  let(:pre_dir) { "/pre/dir" }
  let(:deploy_helper) { Tebako::DeployHelper.new(fs_root, fs_entrance, target_dir, pre_dir) }

  before do
    allow_any_instance_of(Pathname).to receive(:realpath) { |instance| instance }
    allow(Dir).to receive(:exist?).and_call_original
    allow(Dir).to receive(:exist?).with(fs_root).and_return(true)
    allow(File).to receive(:file?).and_call_original
    allow(File).to receive(:file?).with(fs_entrance).and_return(true)
  end

  describe "#initialize" do
    it "sets instance variables correctly" do
      expect(deploy_helper.instance_variable_get(:@fs_root)).to eq(fs_root)
      expect(deploy_helper.instance_variable_get(:@fs_entrance)).to eq(fs_entrance)
      expect(deploy_helper.instance_variable_get(:@target_dir)).to eq(target_dir)
      expect(deploy_helper.instance_variable_get(:@pre_dir)).to eq(pre_dir)
      expect(deploy_helper.instance_variable_get(:@verbose)).to eq(false)
      expect(deploy_helper.instance_variable_get(:@ncores)).to be_a(Integer)
    end
  end

  describe "#bundle_config" do
    let(:r_v) { "3.2.4" }
    let(:ruby_ver) { Tebako::RubyVersion.new(r_v) }
    let(:cwd) { "/current/working/dir" }

    context "on linux" do
      before do
        allow(Tebako::BuildHelpers).to receive(:ncores).and_return(1) if RUBY_PLATFORM =~ /darwin/
        stub_const("RUBY_PLATFORM", "linux")
        allow(deploy_helper).to receive(:lookup_files)
        allow(deploy_helper).to receive(:configure_scenario)
        deploy_helper.configure(ruby_ver, cwd)
      end

      it "calls BuildHelpers.run_with_capture_v with the correct commands" do
        bundle = "/target/dir/bin/bundle"
        expect(Tebako::BuildHelpers)
          .to receive(:run_with_capture_v)
          .with([bundle, "config", "set", "--local", "build.ffi", "--disable-system-libffi"])
          .once
        expect(Tebako::BuildHelpers)
          .to receive(:run_with_capture_v)
          .with([bundle, "config", "set", "--local", "build.nokogiri", "--no-use-system-libraries"])
          .once
        expect(Tebako::BuildHelpers)
          .to receive(:run_with_capture_v)
          .with([bundle, "config", "set", "--local", "force_ruby_platform", "false"])
          .once

        deploy_helper.send(:bundle_config)
      end
    end

    context "on msys" do
      before do
        allow(Tebako::BuildHelpers).to receive(:ncores).and_return(1) if RUBY_PLATFORM =~ /darwin/
        stub_const("RUBY_PLATFORM", "msys")
        allow(deploy_helper).to receive(:lookup_files)
        allow(deploy_helper).to receive(:configure_scenario)
        deploy_helper.configure(ruby_ver, cwd)
      end

      it "calls BuildHelpers.run_with_capture_v with the correct commands" do
        bundle = "/target/dir/bin/bundle.bat"
        expect(Tebako::BuildHelpers)
          .to receive(:run_with_capture_v)
          .with([bundle, "config", "set", "--local", "build.ffi", "--disable-system-libffi"])
          .once
        expect(Tebako::BuildHelpers)
          .to receive(:run_with_capture_v)
          .with([bundle, "config", "set", "--local", "build.nokogiri", "--use-system-libraries"])
          .once
        expect(Tebako::BuildHelpers)
          .to receive(:run_with_capture_v)
          .with([bundle, "config", "set", "--local", "force_ruby_platform", "true"])
          .once

        deploy_helper.send(:bundle_config)
      end
    end
  end

  describe "#configure" do
    let(:r_v) { "3.2.4" }
    let(:ruby_ver) { Tebako::RubyVersion.new(r_v) }
    let(:cwd) { "/current/working/dir" }

    before do
      allow(Tebako::BuildHelpers).to receive(:ncores).and_return(1) if RUBY_PLATFORM =~ /darwin/
      stub_const("RUBY_PLATFORM", "linux")
      allow(deploy_helper).to receive(:lookup_files)
      allow(deploy_helper).to receive(:configure_scenario)
      allow(deploy_helper).to receive(:configure_commands)
      deploy_helper.configure(ruby_ver, cwd)
    end

    it "sets configuration variables correctly" do
      r_vv = deploy_helper.instance_variable_get(:@ruby_ver)
      expect(r_vv.ruby_version).to eq(r_v)
      expect(deploy_helper.instance_variable_get(:@cwd)).to eq(cwd)
      expect(deploy_helper.instance_variable_get(:@tbd)).to eq(File.join(target_dir, "bin"))
      expect(deploy_helper.instance_variable_get(:@tgd)).to eq(File.join(target_dir, "lib", "ruby", "gems",
                                                                         r_vv.api_version))
      expect(deploy_helper.instance_variable_get(:@tld)).to eq(File.join(target_dir, "local"))
    end

    it "calls lookup_files, configure_scenario, and configure_commands" do
      expect(deploy_helper).to have_received(:configure_scenario)
    end
  end

  describe "#check_cwd" do
    context "when tebako cwd setting is nil" do
      it "returns (check succeeds)" do
        deploy_helper.instance_variable_set(:@cwd, nil)
        expect { deploy_helper.send(:check_cwd) }.not_to raise_error
      end
    end

    context "when tebako cwd setting is not an existing folder" do
      it "raises TebakoException with code 108" do
        deploy_helper.instance_variable_set(:@cwd, "/non/existing/folder")
        expect { deploy_helper.send(:check_cwd) }.to raise_error(Tebako::Error) { |e|
          expect(e.error_code).to eq(108)
        }
      end
    end
    context "when tebako cwd setting is an existing folder" do
      it "returns without exception" do
        deploy_helper.instance_variable_set(:@cwd, "/existing/folder")
        allow(File).to receive(:directory?).with(a_string_ending_with("existing/folder")).and_return(true)
        expect { deploy_helper.send(:check_cwd) }.not_to raise_error
      end
    end
  end

  describe "#check_entry_point" do
    let(:entry_point_root) { "/project/entry_points" }
    let(:r_v) { "3.2.4" }
    let(:ruby_ver) { Tebako::RubyVersion.new(r_v) }
    let(:cwd) { "/current/working/dir" }

    before do
      allow(Tebako::BuildHelpers).to receive(:ncores).and_return(1) if RUBY_PLATFORM =~ /darwin/
      stub_const("RUBY_PLATFORM", "linux")
      allow(deploy_helper).to receive(:lookup_files)
      allow(deploy_helper).to receive(:configure_scenario_inner)
      deploy_helper.configure(ruby_ver, cwd)
    end

    context "when the entry point file exists" do
      before do
        allow(File).to receive(:exist?).and_return(true)
      end

      it "does not raise an error" do
        expect do
          deploy_helper.send(:check_entry_point, entry_point_root)
        end.not_to raise_error
      end
    end

    context "when the entry point file does not exist" do
      before do
        allow(File).to receive(:exist?).and_return(false)
      end

      it "raises a Tebako::Error" do
        expect do
          deploy_helper.send(:check_entry_point, entry_point_root)
        end.to raise_error(Tebako::Error, /Entry point/)
      end
    end
  end

  describe "#configure_commands" do
    context "when os_type is msys" do
      before do
        allow(Tebako::BuildHelpers).to receive(:ncores).and_return(1) if RUBY_PLATFORM =~ /darwin/
        stub_const("RUBY_PLATFORM", "msys")
        deploy_helper.instance_variable_set(:@tbd, "/path/to/tbd")
        deploy_helper.send(:configure_commands)
      end

      it "sets the correct command suffixes" do
        expect(deploy_helper.instance_variable_get(:@cmd_suffix)).to eq(".cmd")
        expect(deploy_helper.instance_variable_get(:@bat_suffix)).to eq(".bat")
      end

      it "sets the correct gem and bundler commands" do
        expect(deploy_helper.instance_variable_get(:@gem_command)).to eq("/path/to/tbd/gem.cmd")
        expect(deploy_helper.instance_variable_get(:@bundler_command)).to eq("/path/to/tbd/bundle.bat")
      end

      it "sets the correct force ruby platform" do
        expect(deploy_helper.instance_variable_get(:@force_ruby_platform)).to eq("true")
      end

      it "sets the correct nokogiri option" do
        expect(deploy_helper.instance_variable_get(:@nokogiri_option)).to eq("--use-system-libraries")
      end
    end

    context "when os_type is not msys" do
      before do
        allow(Tebako::BuildHelpers).to receive(:ncores).and_return(1) if RUBY_PLATFORM =~ /darwin/
        stub_const("RUBY_PLATFORM", "linux")
        deploy_helper.instance_variable_set(:@tbd, "/path/to/tbd")
        deploy_helper.send(:configure_commands)
      end

      it "sets the correct command suffixes" do
        expect(deploy_helper.instance_variable_get(:@cmd_suffix)).to eq("")
        expect(deploy_helper.instance_variable_get(:@bat_suffix)).to eq("")
      end

      it "sets the correct gem and bundler commands" do
        expect(deploy_helper.instance_variable_get(:@gem_command)).to eq("/path/to/tbd/gem")
        expect(deploy_helper.instance_variable_get(:@bundler_command)).to eq("/path/to/tbd/bundle")
      end

      it "sets the correct force ruby platform" do
        expect(deploy_helper.instance_variable_get(:@force_ruby_platform)).to eq("false")
      end

      it "sets the correct nokogiri option" do
        expect(deploy_helper.instance_variable_get(:@nokogiri_option)).to eq("--no-use-system-libraries")
      end
    end
  end

  describe "#copy_files" do
    let(:destination) { "/fake/dest" }
    let(:entry_point_root) { "/project/entry_points" }
    let(:r_v) { "3.2.6" }
    let(:ruby_ver) { Tebako::RubyVersion.new(r_v) }
    let(:cwd) { "/current/working/dir" }

    before do
      allow(Tebako::BuildHelpers).to receive(:ncores).and_return(1) if RUBY_PLATFORM =~ /darwin/
      stub_const("RUBY_PLATFORM", "linux")
      allow(deploy_helper).to receive(:lookup_files)
      allow(deploy_helper).to receive(:configure_scenario_inner)
      deploy_helper.configure(ruby_ver, cwd)
    end

    context "when @fs_root exists and is readable" do
      before do
        allow(Dir).to receive(:exist?).with(deploy_helper.instance_variable_get(:@fs_root)).and_return(true)
        allow(File).to receive(:readable?).with(deploy_helper.instance_variable_get(:@fs_root)).and_return(true)
        allow(FileUtils).to receive(:mkdir_p).with(destination)
      end

      it "copies files without raising an error" do
        expect(FileUtils)
          .to receive(:cp_r)
          .with(File.join(deploy_helper.instance_variable_get(:@fs_root), "."), destination)
        expect do
          deploy_helper.send(:copy_files, destination)
        end.not_to raise_error
      end

      it "raises a Tebako::Error if FileUtils.cp_r fails" do
        allow(FileUtils).to receive(:cp_r).and_raise(StandardError)
        expect do
          deploy_helper.send(:copy_files, destination)
        end.to raise_error(Tebako::Error, /does not exist or is not accessible/)
      end
    end

    context "when @fs_root does not exist or is unreadable" do
      before do
        allow(FileUtils).to receive(:mkdir_p).with(destination)
      end

      it "raises a Tebako::Error if the directory does not exist" do
        allow(Dir).to receive(:exist?).with(deploy_helper.instance_variable_get(:@fs_root)).and_return(false)
        expect do
          deploy_helper.send(:copy_files, destination)
        end.to raise_error(Tebako::Error, /not accessible or is not a directory/)
      end

      it "raises a Tebako::Error if the directory is not readable" do
        allow(Dir).to receive(:exist?).with(deploy_helper.instance_variable_get(:@fs_root)).and_return(true)
        allow(File).to receive(:readable?).with(deploy_helper.instance_variable_get(:@fs_root)).and_return(false)
        expect do
          deploy_helper.send(:copy_files, destination)
        end.to raise_error(Tebako::Error, /not accessible or is not a directory/)
      end
    end
  end

  describe "#deploy_env" do
    let(:gem_home) { File.join(target_dir, "gems") }

    before do
      deploy_helper.instance_variable_set(:@gem_home, gem_home)
    end

    it "returns the correct environment variables" do
      expected_env = {
        "GEM_HOME" => gem_home,
        "GEM_PATH" => gem_home,
        "GEM_SPEC_CACHE" => File.join(target_dir, "spec_cache"),
        "TEBAKO_PASS_THROUGH" => "1"
      }
      expect(deploy_helper.deploy_env).to eq(expected_env)
    end
  end

  describe "#install_gem" do
    let(:gem_name) { "some_gem" }
    let(:gem_version) { "1.0.0" }
    let(:gem_command) { "/path/to/gem" }
    let(:bundler_command) { "/path/to/bundle" }

    before do
      deploy_helper.instance_variable_set(:@tgd, "/path/to/tgd")
      deploy_helper.instance_variable_set(:@gem_command, gem_command)
      deploy_helper.instance_variable_set(:@bundler_command, bundler_command)
      allow(Open3).to receive(:capture2e).and_return(["", double("status", signaled?: false, exitstatus: 0)])
    end

    context "when gem version is provided" do
      it "installs the gem with the specified version" do
        expect(Open3).to receive(:capture2e)
          .with(gem_command, "install", gem_name, "-v", gem_version, "--no-document",
                "--install-dir", "/path/to/tgd")
        deploy_helper.install_gem(gem_name, gem_version)
      end
    end

    context "when gem version is not provided" do
      it "installs the gem without specifying the version" do
        expect(Open3).to receive(:capture2e)
          .with(gem_command, "install", gem_name, "--no-document", "--install-dir", "/path/to/tgd")
        deploy_helper.install_gem(gem_name)
      end
    end
  end

  describe "#needs_bundler?" do
    context "when @gf_length is greater than 0" do
      before do
        deploy_helper.instance_variable_set(:@gf_length, 1)
      end

      context "and @ruby_ver is less than 3.1" do
        unless RUBY_PLATFORM =~ /msys|mingw|cygwin/
          before do
            deploy_helper.instance_variable_set(:@ruby_ver, Tebako::RubyVersion.new("3.0.7"))
          end

          it "returns true" do
            if RUBY_PLATFORM =~ /msys|mingw|cygwin/
              expect { deploy_helper.needs_bundler? }.to raise_error(Tebako::Error) { |e|
                expect(e.message).to eq("Ruby version 3.0.7 is not supported on Windows")
                expect(e.error_code).to eq(111)
              }
            else
              expect(deploy_helper.needs_bundler?).to be true
            end
          end
        end
      end

      context "and @ruby_ver is 3.1 or greater" do
        before do
          deploy_helper.instance_variable_set(:@ruby_ver, Tebako::RubyVersion.new("3.1.6"))
        end

        it "returns false" do
          expect(deploy_helper.needs_bundler?).to be false
        end
      end
    end

    context "when @gf_length is 0" do
      before do
        deploy_helper.instance_variable_set(:@gf_length, 0)
      end

      context "and @ruby_ver is less than 3.1" do
        unless RUBY_PLATFORM =~ /msys|mingw|cygwin/
          before do
            deploy_helper.instance_variable_set(:@ruby_ver, Tebako::RubyVersion.new("3.0.7"))
          end

          it "returns false" do
            expect(deploy_helper.needs_bundler?).to be false
          end
        end
      end

      context "and @ruby_ver is 3.1 or greater" do
        before do
          deploy_helper.instance_variable_set(:@ruby_ver, Tebako::RubyVersion.new("3.1.6"))
        end

        it "returns false" do
          expect(deploy_helper.needs_bundler?).to be false
        end
      end
    end
  end

  describe "#update_rubygems" do
    let(:ruby_ver) { instance_double("RubyVersion", ruby31?: false, api_version: "2.7.0") }
    let(:gem_command) { "/path/to/gem" }

    before do
      deploy_helper.instance_variable_set(:@ruby_ver, ruby_ver)
      deploy_helper.instance_variable_set(:@gem_command, gem_command)
      allow(Open3).to receive(:capture2e).and_return(["", double("status", signaled?: false, exitstatus: 0)])
      allow(Tebako::Packager::PatchHelpers).to receive(:patch_file).and_return(true)
    end

    context "when ruby version is 3.1 or higher" do
      before do
        allow(ruby_ver).to receive(:ruby31?).and_return(true)
      end

      it "does not update rubygems" do
        expect(Tebako::BuildHelpers).not_to receive(:run_with_capture_v)
        deploy_helper.update_rubygems
      end
    end

    context "when ruby version is lower than 3.1" do
      before do
        allow(ruby_ver).to receive(:ruby31?).and_return(false)
      end

      it "updates rubygems to the specified version" do
        expect(Open3).to receive(:capture2e)
          .with("/path/to/gem", "update", "--no-doc", "--system", Tebako::RUBYGEMS_VERSION)
          .and_return(["", double("status", signaled?: false, exitstatus: 0)])
        deploy_helper.update_rubygems
      end

      it "calls patch_after_rubygems_update with the correct parameters" do
        allow(File).to receive(:exist?).and_return(true)
        expect(deploy_helper).to receive(:patch_after_rubygems_update).with(target_dir, "2.7.0")
        deploy_helper.update_rubygems
      end
    end
  end
end
# rubocop:enable Metrics/BlockLength
