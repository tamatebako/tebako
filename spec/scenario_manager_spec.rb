# frozen_string_literal: true

# Copyright (c) 2024-2025 [Ribose Inc](https://www.ribose.com).
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
      end
    end
  end

  describe "#configure_scenario" do
    before do
      allow(scenario_manager).to receive(:lookup_files)
    end

    context "when no gemspecs are present" do
      before do
        scenario_manager.instance_variable_set(:@gs_length, 0)
        scenario_manager.instance_variable_set(:@g_length, 0)
      end

      it "sets scenario to :simple_script" do
        scenario_manager.configure_scenario
        expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:simple_script)
      end

      context "with Gemfile" do
        before do
          scenario_manager.instance_variable_set(:@with_gemfile, true)
        end

        it "sets scenario to :gemfile" do
          scenario_manager.configure_scenario
          expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gemfile)
        end
      end
    end

    context "when one gemspec is present" do
      before do
        scenario_manager.instance_variable_set(:@gs_length, 1)
      end

      context "without Gemfile" do
        before do
          scenario_manager.instance_variable_set(:@with_gemfile, false)
        end

        it "sets scenario to :gemspec" do
          scenario_manager.configure_scenario
          expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gemspec)
        end
      end

      context "with Gemfile" do
        before do
          scenario_manager.instance_variable_set(:@with_gemfile, true)
        end

        it "sets scenario to :gemspec_and_gemfile" do
          scenario_manager.configure_scenario
          expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gemspec_and_gemfile)
        end
      end
    end

    context "when multiple gemspecs are present" do
      before do
        scenario_manager.instance_variable_set(:@gs_length, 2)
      end

      it "raises error" do
        expect { scenario_manager.configure_scenario }.to raise_error(
          Tebako::Error,
          "Multiple Ruby gemspecs found in #{fs_root}"
        )
      end
    end
  end

  describe "#configure_scenario_no_gemspec" do
    context "when @with_gemfile is true" do
      before do
        scenario_manager.instance_variable_set(:@with_gemfile, true)
        scenario_manager.instance_variable_set(:@g_length, 0)
        scenario_manager.send(:configure_scenario_no_gemspec)
      end

      it "sets @scenario to :gemfile" do
        expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gemfile)
      end
    end

    context "when @with_gemfile is false and @g_length is positive" do
      before do
        scenario_manager.instance_variable_set(:@with_gemfile, false)
        scenario_manager.instance_variable_set(:@g_length, 1)
        scenario_manager.send(:configure_scenario_no_gemspec)
      end

      it "sets @scenario to :gem" do
        expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:gem)
      end
    end

    context "when @with_gemfile is false and @g_length is 0" do
      before do
        scenario_manager.instance_variable_set(:@with_gemfile, false)
        scenario_manager.instance_variable_set(:@g_length, 0)
        scenario_manager.send(:configure_scenario_no_gemspec)
      end

      it "sets @scenario to :simple_script" do
        expect(scenario_manager.instance_variable_get(:@scenario)).to eq(:simple_script)
      end
    end
  end

  describe "#lookup_files" do
    let(:tmp_root) { Dir.mktmpdir }
    let(:scenario_manager) { described_class.new(tmp_root, "dummy_entry.rb") }

    after do
      FileUtils.remove_entry(tmp_root)
    end

    context "with complete project structure" do
      before do
        # Create test files
        File.write(File.join(tmp_root, "Gemfile"), "source 'https://rubygems.org'")
        File.write(File.join(tmp_root, "Gemfile.lock"), <<~LOCKFILE
          GEM
            remote: https://rubygems.org/
            specs:
              dummy (1.0.0)
          BUNDLED WITH
             2.5.23
        LOCKFILE
        )
        File.write(File.join(tmp_root, "test.gemspec"), "# gemspec content")
        FileUtils.touch(File.join(tmp_root, "test.gem"))

        scenario_manager.send(:lookup_files)
      end

      it "sets correct paths and counts" do
        expect(scenario_manager.instance_variable_get(:@gemfile_path)).to eq(File.join(tmp_root, "Gemfile"))
        expect(scenario_manager.instance_variable_get(:@lockfile_path)).to eq(File.join(tmp_root, "Gemfile.lock"))
        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(1)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(1)
        expect(scenario_manager.instance_variable_get(:@with_gemfile)).to be true
        expect(scenario_manager.instance_variable_get(:@with_lockfile)).to be true
      end
    end

    context "with multiple gemspecs" do
      before do
        File.write(File.join(tmp_root, "test1.gemspec"), "# gemspec 1")
        File.write(File.join(tmp_root, "test2.gemspec"), "# gemspec 2")
        scenario_manager.send(:lookup_files)
      end

      it "counts multiple gemspec files" do
        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(2)
      end
    end

    context "with only Gemfile" do
      before do
        File.write(File.join(tmp_root, "Gemfile"), "source 'https://rubygems.org'")
        scenario_manager.send(:lookup_files)
      end

      it "sets Gemfile-related variables correctly" do
        expect(scenario_manager.instance_variable_get(:@with_gemfile)).to be true
        expect(scenario_manager.instance_variable_get(:@with_lockfile)).to be false
        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(0)
      end
    end

    context "with empty directory" do
      before do
        scenario_manager.send(:lookup_files)
      end

      it "sets default/empty values" do
        expect(scenario_manager.instance_variable_get(:@with_gemfile)).to be false
        expect(scenario_manager.instance_variable_get(:@with_lockfile)).to be false
        expect(scenario_manager.instance_variable_get(:@gs_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@g_length)).to eq(0)
        expect(scenario_manager.instance_variable_get(:@bundler_version)).to eq(Tebako::BUNDLER_VERSION)
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
