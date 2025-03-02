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

require "yaml"
require "tebako/cli_helpers"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::CliHelpers do
  include Tebako::CliHelpers

  let(:options) do
    { "output" => "/path/to/output", "deps" => "/path/to/deps", "entry-point" => "entrypoint",
      "root" => "/tmp/path/to/root/" }
  end
  let(:ruby_ver) { "3.2.6" }
  let(:ruby_hash) { Tebako::RubyVersion::RUBY_VERSIONS["3.2.6"] }

  before do
    allow_any_instance_of(Pathname).to receive(:realpath) { |instance| instance }
    allow(Dir).to receive(:exist?).and_call_original
    allow(Dir).to receive(:exist?).with(options["root"]).and_return(true)
    allow(File).to receive(:file?).and_call_original
    allow(File).to receive(:file?).with(/entrypoint/).and_return(true)
  end

  describe "#do_press" do
    before do
      stub_const("RUBY_PLATFORM", "x86_64-linux")
    end

    context "when mode is set to 'application'" do
      before do
        options["mode"] = "application"
      end

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "executes the press command successfully" do
        allow_any_instance_of(Tebako::PackagerLite).to receive(:create_package).and_return(true)
        expect { do_press(options_manager) }.not_to raise_error
      end
    end

    context "when mode is set to 'bundle'" do
      before do
        options["mode"] = "bundle"
      end

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "executes the press command successfully" do
        allow(FileUtils).to receive(:rm_rf)
        allow(self).to receive(:system).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_tebako_version_h).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_tebako_fs_cpp).and_return(true)
        allow(Tebako::Packager).to receive(:finalize)

        expect { do_press(options_manager) }.not_to raise_error
      end

      it "raises an error if the press command fails" do
        allow(FileUtils).to receive(:rm_rf)
        allow(self).to receive(:system).and_return(false)
        expect { do_press(options_manager) }.to raise_error(Tebako::Error)
      end
    end

    context "when mode is set to 'runtime'" do
      before do
        options["mode"] = "bundle"
      end

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "executes the press command successfully" do
        allow(FileUtils).to receive(:rm_rf)
        allow(self).to receive(:system).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_tebako_version_h).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_tebako_fs_cpp).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_package_header).and_return(true)
        allow(Tebako::Packager).to receive(:finalize)

        expect { do_press(options_manager) }.not_to raise_error
      end

      it "raises an error if the press command fails" do
        allow(FileUtils).to receive(:rm_rf)
        allow(self).to receive(:system).and_return(false)
        expect { do_press(options_manager) }.to raise_error(Tebako::Error)
      end
    end

    context "when package_within_root? is true" do
      before do
        options["mode"] = "bundle"
        options["output"] = "/tmp/path/to/root/output"
      end

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "shows a warning and executes the press command successfully" do
        allow(FileUtils).to receive(:rm_rf)
        allow(self).to receive(:system).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_tebako_version_h).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_tebako_fs_cpp).and_return(true)
        allow(Tebako::Codegen).to receive(:generate_package_header).and_return(true)
        allow(Tebako::Packager).to receive(:finalize)

        allow(self).to receive(:sleep).with(any_args).and_return(nil)
        expect { do_press(options_manager) }.to output(/WARNING/).to_stdout
      end
    end
  end

  describe "#check_warnings" do
    let(:options_manager) { Tebako::OptionsManager.new(options) }

    context "when mode is runtime" do
      before do
        options["mode"] = "runtime"
      end

      it "does not display any warnings" do
        expect { check_warnings(options_manager) }.not_to output.to_stdout
      end
    end

    context "when package is within root" do
      before do
        options["mode"] = "both"
        allow(options_manager).to receive(:package_within_root?).and_return(true)
        allow(options_manager).to receive(:prefix_within_root?).and_return(false)
        allow(self).to receive(:sleep)
      end

      it "displays package warning" do
        expect { check_warnings(options_manager) }.to output(Tebako::CliHelpers::WARN).to_stdout
      end
    end

    context "when prefix is within root" do
      before do
        options["mode"] = "both"
        allow(options_manager).to receive(:package_within_root?).and_return(false)
        allow(options_manager).to receive(:prefix_within_root?).and_return(true)
        allow(self).to receive(:sleep)
      end

      it "displays prefix warning" do
        expect { check_warnings(options_manager) }.to output(Tebako::CliHelpers::WARN2).to_stdout
      end
    end

    context "when both package and prefix are within root" do
      before do
        options["mode"] = "both"
        allow(options_manager).to receive(:package_within_root?).and_return(true)
        allow(options_manager).to receive(:prefix_within_root?).and_return(true)
        allow(self).to receive(:sleep)
      end

      it "displays both warnings" do
        expect do
          check_warnings(options_manager)
        end.to output(Tebako::CliHelpers::WARN + Tebako::CliHelpers::WARN2).to_stdout
      end
    end
  end

  describe "#do_setup" do
    let(:options_manager) { Tebako::OptionsManager.new(options) }

    context "when running on Gnu Linux" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux")
      end

      it "executes the setup command successfully" do
        allow(FileUtils).to receive(:rm_rf)
        allow(self).to receive(:system).and_return(true)
        expect { do_setup(options_manager) }.not_to raise_error
      end

      it "raises an error if the setup command fails" do
        allow(FileUtils).to receive(:rm_rf)
        allow(self).to receive(:system).and_return(false)
        expect { do_setup(options_manager) }.to raise_error(Tebako::Error)
      end
    end
  end

  describe "#finalize" do
    let(:options_manager) { Tebako::OptionsManager.new(options) }
    let(:scenario_manager) { Tebako::ScenarioManager.new(options_manager.root, options_manager.fs_entrance) }
    let(:patchelf_path) { File.join(options_manager.deps_bin_dir, "patchelf") }

    before do
      allow(Tebako::Packager).to receive(:finalize)
    end

    context "when patchelf is enabled and running on GNU/Linux" do
      before do
        options["patchelf"] = true
        allow(scenario_manager).to receive(:linux_gnu?).and_return(true)
      end

      it "calls Packager.finalize with patchelf path" do
        expect(Tebako::Packager).to receive(:finalize).with(
          options_manager.ruby_src_dir,
          options_manager.package,
          options_manager.rv,
          patchelf_path
        )
        finalize(options_manager, scenario_manager)
      end
    end

    context "when patchelf is disabled" do
      before do
        options["patchelf"] = false
        allow(scenario_manager).to receive(:linux_gnu?).and_return(true)
      end

      it "calls Packager.finalize without patchelf path" do
        expect(Tebako::Packager).to receive(:finalize).with(
          options_manager.ruby_src_dir,
          options_manager.package,
          options_manager.rv,
          nil
        )
        finalize(options_manager, scenario_manager)
      end
    end

    context "when not running on GNU/Linux" do
      before do
        options["patchelf"] = true
        allow(scenario_manager).to receive(:linux_gnu?).and_return(false)
      end

      it "calls Packager.finalize without patchelf path" do
        expect(Tebako::Packager).to receive(:finalize).with(
          options_manager.ruby_src_dir,
          options_manager.package,
          options_manager.rv,
          nil
        )
        finalize(options_manager, scenario_manager)
      end
    end
  end

  describe "#do_press_runtime" do
    let(:options_manager) { Tebako::OptionsManager.new(options) }
    let(:scenario_manager) { Tebako::ScenarioManager.new(options_manager.root, options_manager.fs_entrance) }

    before do
      allow(Tebako::Codegen).to receive(:generate_tebako_version_h)
      allow(Tebako::Codegen).to receive(:generate_tebako_fs_cpp)
      allow(Tebako::Codegen).to receive(:generate_deploy_rb)
      allow(Tebako::Codegen).to receive(:generate_stub_rb)
      allow(self).to receive(:system).and_return(true)
      allow(self).to receive(:finalize)
    end

    context "when mode is 'both'" do
      before { options["mode"] = "both" }

      it "generates files and executes commands" do
        expect(Tebako::Codegen).to receive(:generate_tebako_version_h)
        expect(Tebako::Codegen).to receive(:generate_tebako_fs_cpp)
        expect(Tebako::Codegen).to receive(:generate_deploy_rb)
        expect(Tebako::Codegen).to receive(:generate_stub_rb)
        expect(self).to receive(:system).exactly(2).times.and_return(true)
        expect(self).to receive(:finalize)
        do_press_runtime(options_manager, scenario_manager)
      end
    end

    context "when mode is 'runtime'" do
      before { options["mode"] = "runtime" }

      it "generates files and executes commands" do
        expect(Tebako::Codegen).to receive(:generate_tebako_version_h)
        expect(Tebako::Codegen).to receive(:generate_tebako_fs_cpp)
        expect(Tebako::Codegen).to receive(:generate_deploy_rb)
        expect(Tebako::Codegen).to receive(:generate_stub_rb)
        expect(self).to receive(:system).exactly(2).times.and_return(true)
        expect(self).to receive(:finalize)
        do_press_runtime(options_manager, scenario_manager)
      end
    end

    context "when mode is 'bundle'" do
      before { options["mode"] = "bundle" }

      it "generates files and executes commands" do
        expect(Tebako::Codegen).to receive(:generate_tebako_version_h)
        expect(Tebako::Codegen).to receive(:generate_tebako_fs_cpp)
        expect(Tebako::Codegen).to receive(:generate_deploy_rb)
        expect(Tebako::Codegen).not_to receive(:generate_stub_rb)
        expect(self).to receive(:system).exactly(2).times.and_return(true)
        expect(self).to receive(:finalize)
        do_press_runtime(options_manager, scenario_manager)
      end
    end

    context "when mode is 'application'" do
      before { options["mode"] = "application" }

      it "returns early without doing anything" do
        expect(Tebako::Codegen).not_to receive(:generate_tebako_version_h)
        expect(self).not_to receive(:system)
        expect(self).not_to receive(:finalize)
        do_press_runtime(options_manager, scenario_manager)
      end
    end

    context "when system commands fail" do
      before { options["mode"] = "bundle" }

      it "raises error when press_cfg_cmd fails" do
        allow(self).to receive(:system).and_return(false)
        expect { do_press_runtime(options_manager, scenario_manager) }.to raise_error(Tebako::Error)
      end

      it "raises error when press_build_cmd fails" do
        allow(self).to receive(:system).with(anything, press_cfg_cmd(options_manager)).and_return(true)
        allow(self).to receive(:system).with(anything, press_build_cmd(options_manager)).and_return(false)
        expect { do_press_runtime(options_manager, scenario_manager) }.to raise_error(Tebako::Error)
      end
    end
  end

  describe "#options_from_tebafile" do
    let(:tebafile) { "spec/fixtures/tebafile.yml" }

    context "when the tebafile contains valid YAML" do
      it "loads options from the tebafile" do
        allow(YAML).to receive(:load_file).and_return({ "options" => { "key" => "value" } })
        expect(options_from_tebafile(tebafile)).to eq({ "key" => "value" })
      end
    end

    context "when the tebafile contains invalid YAML" do
      it "returns an empty hash and prints a warning" do
        allow(YAML).to receive(:load_file).and_raise(Psych::SyntaxError.new("file", 1, 1, 1, "message", "context"))
        expect { options_from_tebafile(tebafile) }.to output(/Warning: The tebafile/).to_stdout
        expect(options_from_tebafile(tebafile)).to eq({})
      end
    end

    context "when an unexpected error occurs" do
      it "returns an empty hash and prints a warning" do
        allow(YAML).to receive(:load_file).and_raise(StandardError.new("Unexpected error"))
        expect { options_from_tebafile(tebafile) }.to output(/An unexpected error occurred/).to_stdout
        expect(options_from_tebafile(tebafile)).to eq({})
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
