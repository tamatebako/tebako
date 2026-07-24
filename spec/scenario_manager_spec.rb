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

require "tmpdir"
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
      end

      it "sets the project root and entry point" do
        expect(scenario_manager.fs_root).to eq(fs_root)
        expect(scenario_manager.fs_entrance).to eq(fs_entrance)
      end
    end

    context "on Windows" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
      end

      it "sets the project root and entry point" do
        expect(scenario_manager.fs_root).to eq(fs_root)
        expect(scenario_manager.fs_entrance).to eq(fs_entrance)
      end
    end
  end

  describe "#configure_scenario" do
    let(:tmp_root) { Dir.mktmpdir }
    let(:scenario_manager) { described_class.new(tmp_root, "dummy_entry.rb") }

    after do
      FileUtils.remove_entry(tmp_root)
    end

    context "with an empty project" do
      it "selects the :simple_script scenario with a /local entry point" do
        scenario_manager.configure_scenario
        expect(scenario_manager.scenario).to eq(:simple_script)
        expect(scenario_manager.fs_entry_point).to eq("/local/dummy_entry.rb")
        expect(scenario_manager.with_gemfile).to be false
      end
    end

    context "with only a Gemfile" do
      before do
        File.write(File.join(tmp_root, "Gemfile"), "source 'https://rubygems.org'")
      end

      it "selects the :gemfile scenario with a /local entry point" do
        scenario_manager.configure_scenario
        expect(scenario_manager.scenario).to eq(:gemfile)
        expect(scenario_manager.with_gemfile).to be true
        expect(scenario_manager.gemfile_path).to eq(File.join(tmp_root, "Gemfile"))
        expect(scenario_manager.fs_entry_point).to eq("/local/dummy_entry.rb")
      end
    end

    context "with only a gem package" do
      before do
        FileUtils.touch(File.join(tmp_root, "test.gem"))
      end

      it "selects the :gem scenario with a /bin entry point" do
        scenario_manager.configure_scenario
        expect(scenario_manager.scenario).to eq(:gem)
        expect(scenario_manager.fs_entry_point).to eq("/bin/dummy_entry.rb")
      end
    end

    context "with one gemspec" do
      before do
        File.write(File.join(tmp_root, "test.gemspec"), "# gemspec content")
      end

      it "selects the :gemspec scenario" do
        scenario_manager.configure_scenario
        expect(scenario_manager.scenario).to eq(:gemspec)
        expect(scenario_manager.fs_entry_point).to eq("/bin/dummy_entry.rb")
      end

      context "and a Gemfile" do
        before do
          File.write(File.join(tmp_root, "Gemfile"), "source 'https://rubygems.org'")
        end

        it "selects the :gemspec_and_gemfile scenario" do
          scenario_manager.configure_scenario
          expect(scenario_manager.scenario).to eq(:gemspec_and_gemfile)
        end
      end
    end

    context "with a complete project structure" do
      before do
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
      end

      it "prefers the gemspec and exposes the Gemfile path" do
        scenario_manager.configure_scenario
        expect(scenario_manager.scenario).to eq(:gemspec_and_gemfile)
        expect(scenario_manager.gemfile_path).to eq(File.join(tmp_root, "Gemfile"))
        expect(scenario_manager.with_gemfile).to be true
      end
    end

    context "with multiple gemspecs" do
      before do
        File.write(File.join(tmp_root, "test1.gemspec"), "# gemspec 1")
        File.write(File.join(tmp_root, "test2.gemspec"), "# gemspec 2")
      end

      it "raises an error" do
        expect { scenario_manager.configure_scenario }.to raise_error(
          Tebako::Error,
          "Multiple Ruby gemspecs found in #{scenario_manager.fs_root}"
        )
      end
    end
  end

  describe "#linux_gnu?" do
    context "when running on GNU/Linux" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux-gnu")
      end

      it "returns true" do
        expect(scenario_manager.linux_gnu?).to be true
      end
    end

    context "when running on Linux musl" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux-musl")
      end

      it "returns false" do
        expect(scenario_manager.linux_gnu?).to be false
      end
    end

    context "when not running on Linux" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it "returns false" do
        expect(scenario_manager.linux_gnu?).to be false
      end
    end
  end

  describe "#linux_musl?" do
    context "when running on Linux musl" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux-musl")
      end

      it "returns true" do
        expect(scenario_manager.linux_musl?).to be true
      end
    end

    context "when running on GNU/Linux" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux-gnu")
      end

      it "returns false" do
        expect(scenario_manager.linux_musl?).to be false
      end
    end

    context "when not running on Linux" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it "returns false" do
        expect(scenario_manager.linux_musl?).to be false
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
