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

require "tmpdir"
require_relative "../lib/tebako/scenario_manager"
# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::ScenarioManager do
  let(:fs_root) { "/fs/root" }
  let(:fs_entrance) { "entrance" }
  let(:scenario_manager) { Tebako::ScenarioManager.new(fs_root, fs_entrance) }

  before do
    allow_any_instance_of(Pathname).to receive(:realpath) { |instance| instance }
    allow(Dir).to receive(:exist?).and_call_original
    allow(Dir).to receive(:exist?).with(fs_root).and_return(true)
    allow(File).to receive(:file?).and_call_original
    allow(File).to receive(:file?).with(/entrance/).and_return(true)
  end

  describe "#initialize" do
    context "on non-Windows" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
        scenario_manager.configure_scenario
      end

      it "sets instance variables correctly" do
        expect(scenario_manager.instance_variable_get(:@fs_root)).to eq(fs_root)
        expect(scenario_manager.instance_variable_get(:@fs_entrance)).to eq(fs_entrance)
        expect(scenario_manager.instance_variable_get(:@fs_mount_point)).to eq("/__tebako_memfs__")
      end
    end

    context "on Windows" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
        scenario_manager.configure_scenario
      end

      it "sets instance variables correctly" do
        expect(scenario_manager.instance_variable_get(:@fs_root)).to eq(fs_root)
        expect(scenario_manager.instance_variable_get(:@fs_entrance)).to eq(fs_entrance)
        expect(scenario_manager.instance_variable_get(:@fs_mount_point)).to eq("A:/__tebako_memfs__")
      end
    end
  end
  describe "#configure_scenario_inner" do
    before do
      allow(scenario_manager).to receive(:lookup_files)
      allow(scenario_manager).to receive(:configure_scenario_inner)
      scenario_manager.configure_scenario
    end

    it "calls configure_scenario_inner" do
      expect(scenario_manager).to have_received(:configure_scenario_inner)
    end
  end

  describe "#configure_scenario" do
    context "when @gs_length is 0" do
      before do
        scenario_manager.instance_variable_set(:@gs_length, 0)
      end

      it "calls configure_scenario_no_gemspec" do
        expect(scenario_manager).to receive(:configure_scenario_no_gemspec)
        scenario_manager.send(:configure_scenario_inner)
      end
    end

    context "when @gs_length is 1" do
      before do
        scenario_manager.instance_variable_set(:@gs_length, 1)
      end

      context "and @gf_length is positive" do
        before do
          scenario_manager.instance_variable_set(:@gf_length, 1)
          scenario_manager.send(:configure_scenario_inner)
        end

        it "sets @scenario to :gemspec_and_gemfile" do
          expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gemspec_and_gemfile)
        end
      end

      context "and @gf_length is 0" do
        before do
          scenario_manager.instance_variable_set(:@gf_length, 0)
          scenario_manager.send(:configure_scenario_inner)
        end

        it "sets @scenario to :gemspec" do
          expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gemspec)
        end
      end
    end

    context "when @gs_length is greater than 1" do
      before do
        scenario_manager.instance_variable_set(:@gs_length, 2)
      end

      it "raises a Tebako::Error" do
        expect do
          scenario_manager.send(:configure_scenario_inner)
        end.to raise_error(Tebako::Error,
                           "Multiple Ruby gemspecs found in #{scenario_manager.instance_variable_get(:@fs_root)}")
      end
    end
  end

  describe "#configure_scenario_no_gemspec" do
    context "when @gf_length is positive" do
      before do
        scenario_manager.instance_variable_set(:@gf_length, 1)
        scenario_manager.instance_variable_set(:@g_length, 0)
        scenario_manager.send(:configure_scenario_no_gemspec)
      end

      it "sets @scenario to :gemfile" do
        expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gemfile)
      end
    end

    context "when @gf_length is 0 and @g_length is positive" do
      before do
        scenario_manager.instance_variable_set(:@gf_length, 0)
        scenario_manager.instance_variable_set(:@g_length, 1)
        scenario_manager.send(:configure_scenario_no_gemspec)
      end

      it "sets @scenario to :gem" do
        expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gem)
      end
    end

    context "when both @gf_length and @g_length are 0" do
      before do
        scenario_manager.instance_variable_set(:@gf_length, 0)
        scenario_manager.instance_variable_set(:@g_length, 0)
        scenario_manager.send(:configure_scenario_no_gemspec)
      end

      it "sets @scenario to :simple_script" do
        expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:simple_script)
      end
    end
  end

  describe "#exe_suffix" do
    context "when running on Windows" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
      end

      it "returns .exe" do
        expect(scenario_manager.exe_suffix).to eq(".exe")
      end
    end

    context "when not running on Windows" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it "returns an empty string" do
        expect(scenario_manager.exe_suffix).to eq("")
      end
    end
  end

  describe "#lookup_files" do
    it "sets @gs_length, @gf_length, and @g_length correctly for only gemspec files" do
      Dir.mktmpdir do |tmp_dir|
        FileUtils.touch(File.join(tmp_dir, "example1.gemspec"))
        FileUtils.touch(File.join(tmp_dir, "example2.gemspec"))
        scenario_manager = Tebako::ScenarioManager.new(tmp_dir, fs_entrance)

        scenario_manager.send(:lookup_files)

        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(2)
        expect(scenario_manager.instance_variable_get(:@gf_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(0)
      end
    end

    it "sets @gs_length, @gf_length, and @g_length correctly for only Gemfile" do
      Dir.mktmpdir do |tmp_dir|
        FileUtils.touch(File.join(tmp_dir, "Gemfile"))
        scenario_manager = Tebako::ScenarioManager.new(tmp_dir, fs_entrance)

        scenario_manager.send(:lookup_files)

        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@gf_length)).to eq(1)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(0)
      end
    end

    it "sets @gs_length, @gf_length, and @g_length correctly for only gem files" do
      Dir.mktmpdir do |tmp_dir|
        FileUtils.touch(File.join(tmp_dir, "example1.gem"))
        scenario_manager = Tebako::ScenarioManager.new(tmp_dir, fs_entrance)

        scenario_manager.send(:lookup_files)

        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@gf_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(1)
      end
    end

    it "sets @gs_length, @gf_length, and @g_length correctly for mixed files" do
      Dir.mktmpdir do |tmp_dir|
        FileUtils.touch(File.join(tmp_dir, "example1.gemspec"))
        FileUtils.touch(File.join(tmp_dir, "Gemfile"))
        FileUtils.touch(File.join(tmp_dir, "example2.gem"))
        scenario_manager = Tebako::ScenarioManager.new(tmp_dir, fs_entrance)

        scenario_manager.send(:lookup_files)

        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(1)
        expect(scenario_manager.instance_variable_get(:@gf_length)).to eq(1)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(1)
      end
    end

    it "sets @gs_length, @gf_length, and @g_length to zero when no relevant files are present" do
      Dir.mktmpdir do |tmp_dir|
        scenario_manager = Tebako::ScenarioManager.new(tmp_dir, fs_entrance)

        scenario_manager.send(:lookup_files)

        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@gf_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(0)
      end
    end
  end

  describe "#macos?" do
    context "when running on macOS" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it "returns true" do
        expect(scenario_manager.macos?).to be true
      end
    end

    context "when not running on macOS" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it "returns false" do
        expect(scenario_manager.macos?).to be false
      end
    end
  end
end
# rubocop:enable Metrics/BlockLength
