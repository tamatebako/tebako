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
        options["runtime"] = "source"
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
        options["runtime"] = "source"
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
        options["runtime"] = "source"
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

    context "when runtime is prebuilt (the bundle-mode default)" do
      before do
        options["mode"] = "bundle"
      end

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "resolves the runtime, builds the app image and stitches" do
        expect(Tebako::Packager).to receive(:check_prebuilt_env!)
        expect(Tebako::RuntimeManager).to receive(:resolve)
          .with(options_manager.ruby_ver, options_manager.host_platform)
          .and_return("/cached/runtime")
        expect(Tebako::Packager).to receive(:build_app_image).and_return("/o/p/fs.bin")
        expect(Tebako::Stitcher).to receive(:stitch) do |runtime, images:, output:|
          expect(runtime).to eq("/cached/runtime")
          expect(images.first[:path]).to eq("/o/p/fs.bin")
          expect(images.first[:mount_point]).to eq("/__tebako_memfs__")
          expect(output).to eq(options_manager.package)
        end

        do_press(options_manager)
      end

      it "appends --image entries after the application image" do
        options["image"] = ["/data/extra.tfs:extra"]
        allow(Tebako::Packager).to receive(:check_prebuilt_env!)
        allow(Tebako::RuntimeManager).to receive(:resolve).and_return("/cached/runtime")
        allow(Tebako::Packager).to receive(:build_app_image).and_return("/o/p/fs.bin")
        expect(Tebako::Stitcher).to receive(:stitch) do |_runtime, images:, **_|
          expect(images.size).to eq(2)
          expect(images.last).to eq({ path: "/data/extra.tfs", mount_point: "extra",
                                      format_id: Tebako::Stitcher::FORMAT_DWARFS })
        end

        do_press(options_manager)
      end

      it "fails with error 128 when the packaging environment is missing" do
        allow(Tebako::Packager).to receive(:check_prebuilt_env!)
          .and_raise(Tebako::Error.new("Prebuilt runtime press requires the packaging environment", 128))
        expect(Tebako::RuntimeManager).not_to receive(:resolve)
        expect { do_press(options_manager) }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(128) }
      end
    end

    context "when mode is 'lean' (the default)" do
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "resolves the bootstrap and stitches a lean three-part package" do
        expect(Tebako::Packager).to receive(:check_prebuilt_env!)
        expect(Tebako::BootstrapManager).to receive(:resolve)
          .with(options_manager.host_platform)
          .and_return("/cached/bootstrap")
        expect(Tebako::RuntimeManager).not_to receive(:resolve)
        expect(Tebako::Packager).to receive(:build_app_image).and_return("/o/p/fs.bin")
        expect(Tebako::Stitcher).to receive(:stitch) do |bootstrap, images:, output:, **kwargs|
          expect(bootstrap).to eq("/cached/bootstrap")
          expect(images.size).to eq(1)
          expect(images.first[:path]).to eq("/o/p/fs.bin")
          expect(images.first[:mount_point]).to eq("/__tebako_memfs__")
          expect(output).to eq(options_manager.package)
          expect(kwargs[:lean]).to be(true)
          expect(kwargs[:ruby_version]).to eq(options_manager.ruby_ver)
          expect(kwargs[:launcher_abi]).to eq(Tebako::LauncherAbi::VERSION)
          expect(kwargs[:runtime_sha256]).to be_nil
        end

        do_press(options_manager)
      end

      it "rejects --runtime source with error 130 before resolving anything" do
        options["runtime"] = "source"
        expect(Tebako::BootstrapManager).not_to receive(:resolve)
        expect(Tebako::Packager).not_to receive(:build_app_image)
        expect { do_press(options_manager) }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(130) }
      end
    end

    context "when mode is 'fat'" do
      before { options["mode"] = "fat" }

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "adds the runtime as a payload slot with its checksum in the runtime_ref" do
        expect(Tebako::Packager).to receive(:check_prebuilt_env!)
        expect(Tebako::BootstrapManager).to receive(:resolve).and_return("/cached/bootstrap")
        expect(Tebako::RuntimeManager).to receive(:resolve)
          .with(options_manager.ruby_ver, options_manager.host_platform)
          .and_return("/cached/runtime")
        expect(Tebako::Packager).to receive(:build_app_image).and_return("/o/p/fs.bin")
        expect(Digest::SHA256).to receive(:file).with("/cached/runtime")
                                                .and_return(instance_double(Digest::SHA256, hexdigest: "a" * 64))
        expect(Tebako::Stitcher).to receive(:stitch) do |_bootstrap, images:, **kwargs|
          expect(images.size).to eq(2)
          expect(images.last).to eq({ path: "/cached/runtime", mount_point: "",
                                      format_id: Tebako::Stitcher::FORMAT_RUNTIME })
          expect(kwargs[:lean]).to be(true)
          expect(kwargs[:runtime_sha256]).to eq("a" * 64)
        end

        do_press(options_manager)
      end

      it "fails with error 134 when the selected bootstrap predates payload support" do
        allow(Tebako::BootstrapManager).to receive(:default_version).and_return("0.1.0")
        expect(Tebako::BootstrapManager).not_to receive(:resolve)
        expect { do_press(options_manager) }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(134) }
      end
    end

    context "when mode is 'classic'" do
      before { options["mode"] = "classic" }

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "stitches onto a prebuilt runtime like bundle mode" do
        expect(Tebako::Packager).to receive(:check_prebuilt_env!)
        expect(Tebako::RuntimeManager).to receive(:resolve).and_return("/cached/runtime")
        expect(Tebako::BootstrapManager).not_to receive(:resolve)
        expect(Tebako::Packager).to receive(:build_app_image).and_return("/o/p/fs.bin")
        expect(Tebako::Stitcher).to receive(:stitch) do |runtime, images:, output:, **kwargs|
          expect(runtime).to eq("/cached/runtime")
          expect(images.first[:path]).to eq("/o/p/fs.bin")
          expect(output).to eq(options_manager.package)
          expect(kwargs[:lean]).to be_falsy
        end

        do_press(options_manager)
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
          patchelf_path,
          "package"
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
          nil,
          "package"
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
          nil,
          "package"
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

    context "when mode is 'classic'" do
      before { options["mode"] = "classic" }

      it "builds from source exactly like bundle mode" do
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
