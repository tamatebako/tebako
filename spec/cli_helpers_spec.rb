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

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::CliHelpers do
  include Tebako::CliHelpers

  let(:options) do
    { "output" => "/path/to/output", "deps" => "/path/to/deps", "entry-point" => "entrypoint",
      "root" => "/tmp/path/to/root/" }
  end
  let(:ruby_ver) { "3.2.6" }

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

    context "when mode is set to 'classic'" do
      before do
        options["mode"] = "classic"
      end

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "resolves the runtime, builds the app image and stitches" do
        expect(Tebako::Packager).to receive(:check_prebuilt_env!)
        expect(Tebako::RuntimeManager).to receive(:resolve)
          .with(options_manager.ruby_ver, options_manager.host_platform)
          .and_return("/cached/runtime")
        expect(Tebako::RuntimeManager).to receive(:layout)
          .with("/cached/runtime").and_return("/cached/layout")
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
        allow(Tebako::RuntimeManager).to receive(:layout).and_return("/cached/layout")
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

      it "does not resolve the bootstrap" do
        allow(Tebako::Packager).to receive(:check_prebuilt_env!)
        allow(Tebako::RuntimeManager).to receive(:resolve).and_return("/cached/runtime")
        allow(Tebako::RuntimeManager).to receive(:layout).and_return("/cached/layout")
        allow(Tebako::Packager).to receive(:build_app_image).and_return("/o/p/fs.bin")
        expect(Tebako::BootstrapManager).not_to receive(:resolve)
        expect(Tebako::Stitcher).to receive(:stitch) do |_runtime, output:, **kwargs|
          expect(output).to eq(options_manager.package)
          expect(kwargs[:lean]).to be_falsy
        end

        do_press(options_manager)
      end
    end

    context "when package_within_root? is true" do
      before do
        options["mode"] = "classic"
        options["output"] = "/tmp/path/to/root/output"
      end

      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "shows a warning and executes the press command successfully" do
        allow(Tebako::Packager).to receive(:check_prebuilt_env!)
        allow(Tebako::RuntimeManager).to receive(:resolve).and_return("/cached/runtime")
        allow(Tebako::RuntimeManager).to receive(:layout).and_return("/cached/layout")
        allow(Tebako::Packager).to receive(:build_app_image).and_return("/o/p/fs.bin")
        allow(Tebako::Stitcher).to receive(:stitch)

        allow(self).to receive(:sleep).with(any_args).and_return(nil)
        expect { do_press(options_manager) }.to output(/WARNING/).to_stdout
      end
    end

    context "when mode is 'lean' (the default)" do
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "resolves the bootstrap and stitches a lean three-part package" do
        expect(Tebako::Packager).to receive(:check_prebuilt_env!)
        expect(Tebako::BootstrapManager).to receive(:resolve)
          .with(options_manager.host_platform)
          .and_return("/cached/bootstrap")
        # lean resolves the runtime too: its extracted layout aligns the app
        # image's arch conventions (the payload slot and sha stay fat-only)
        expect(Tebako::RuntimeManager).to receive(:resolve)
          .with(options_manager.ruby_ver, options_manager.host_platform)
          .and_return("/cached/runtime")
        expect(Tebako::RuntimeManager).to receive(:layout)
          .with("/cached/runtime").and_return("/cached/layout")
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
        expect(Tebako::RuntimeManager).to receive(:layout)
          .with("/cached/runtime").and_return("/cached/layout")
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
  end

  describe "#check_warnings" do
    let(:options_manager) { Tebako::OptionsManager.new(options) }

    context "when neither package nor prefix is within root" do
      before do
        allow(options_manager).to receive(:package_within_root?).and_return(false)
        allow(options_manager).to receive(:prefix_within_root?).and_return(false)
      end

      it "does not display any warnings" do
        expect { check_warnings(options_manager) }.not_to output.to_stdout
      end
    end

    context "when package is within root" do
      before do
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
