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

require "open3"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::OptionsManager do
  let(:options) { {} }
  let(:ruby_ver) { Tebako::RubyVersion::DEFAULT_RUBY_VERSION }

  describe "#cfg_options" do
    let(:deps) { File.join(Dir.pwd, "deps") }
    let(:output_folder) { File.join(Dir.pwd, "o") }
    let(:source) { "/path/to/source" }
    let(:m_files) { "Unix Makefiles" }

    let(:options) { { "output" => "/path/to/output", "prefix" => "PWD" } }
    let(:options_manager) { Tebako::OptionsManager.new(options) }

    before do
      stub_const("RUBY_PLATFORM", "linux")
    end

    context "when on a Gnu Linux platform" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux")
      end

      it "returns the correct configuration options string" do
        v_parts = Tebako::VERSION.split(".")
        exp_opt = "-DCMAKE_BUILD_TYPE=Release -DDEPS:STRING=\"#{deps}\" -G \"#{m_files}\" " \
                  "-B \"#{output_folder}\" -S \"#{Dir.pwd}\" " \
                  "-DTEBAKO_VERSION:STRING=\"#{v_parts[0]}.#{v_parts[1]}.#{v_parts[2]}\""
        expect(options_manager.cfg_options).to eq(exp_opt)
      end
    end
  end

  describe "#data_bundle_file" do
    let(:options_manager) { Tebako::OptionsManager.new({}) }
    it "returns the correct data bundle file path" do
      allow(options_manager).to receive(:data_bin_dir).and_return("/path/to/data_bin")
      expect(options_manager.data_bundle_file).to eq("/path/to/data_bin/fs.bin")
    end
  end

  describe "#fs_current" do
    let(:options_manager) { Tebako::OptionsManager.new({}) }
    context "when the platform is Windows (msys, mingw, cygwin)" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
        s = String.new("C:\\path\\to\\current")
        allow(Open3).to receive(:capture2e)
          .with("cygpath", "-w", Dir.pwd).and_return([s, double(success?: true)])
      end

      it "returns the Windows path" do
        expect(options_manager.fs_current).to eq("C:/path/to/current")
      end

      it "raises an error if cygpath command fails" do
        allow(Open3).to receive(:capture2e).with("cygpath", "-w", Dir.pwd).and_return(["", double(success?: false)])
        expect { options_manager.fs_current }.to raise_error(Tebako::Error)
      end
    end

    context "when the platform is not Windows" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux")
      end

      it "returns the current working directory" do
        expect(options_manager.fs_current).to eq(Dir.pwd)
      end
    end
  end

  describe "#handle_nil_prefix" do
    let(:options_manager) { Tebako::OptionsManager.new({}) }

    context "when TEBAKO_PREFIX environment variable is not set" do
      before do
        allow(ENV).to receive(:fetch).with("TEBAKO_PREFIX", nil).and_return(nil)
      end

      it "prints a message and returns the expanded path to ~/.tebako" do
        expect do
          options_manager.handle_nil_prefix
        end.to output("No prefix specified, using ~/.tebako\n").to_stdout
        expect(options_manager.handle_nil_prefix).to eq(File.expand_path("~/.tebako"))
      end
    end

    context "when TEBAKO_PREFIX environment variable is set" do
      let(:env_prefix) { "/custom/prefix" }

      before do
        allow(ENV).to receive(:fetch).with("TEBAKO_PREFIX", nil).and_return(env_prefix)
      end

      it "prints a message and returns the expanded path to the environment variable" do
        expect do
          options_manager.handle_nil_prefix
        end.to output("Using TEBAKO_PREFIX environment variable as prefix\n").to_stdout
        expect(options_manager.handle_nil_prefix).to eq(File.expand_path(env_prefix))
      end
    end
  end

  describe "#l_level" do
    context "when log-level option is not set" do
      let(:options) { {} }
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      it 'returns "error"' do
        expect(options_manager.l_level).to eq("error")
      end
    end

    context "when log-level option is set" do
      let(:options) { { "log-level" => "info" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the value of the log-level option" do
        expect(options_manager.l_level).to eq("info")
      end
    end
  end

  describe "#mode" do
    context "when mode option is not set" do
      let(:options) { {} }
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      it 'returns "lean" (the default press mode)' do
        expect(options_manager.mode).to eq("lean")
      end
    end

    context "when mode option is set" do
      let(:options) { { "mode" => "both" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the value of the mode option" do
        expect(options_manager.mode).to eq("both")
      end
    end
  end

  describe "#package" do
    context 'when @options["output"] is set' do
      let(:options) { { "output" => "custom_package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the package option" do
        expect(options_manager.package).to eq(File.expand_path("custom_package"))
      end

      # requires platform to have cygpath
      context "with Windows-style paths", if: Gem.win_platform? do
        let(:options) { { "output" => "C:\\path\\to\\package" } }
        it "converts backslashes to forward slashes" do
          expect(options_manager.package).to eq("C:/path/to/package")
        end
      end
    end

    context 'when @options["output"] is not set' do
      context 'when mode is "lean" or not set' do
        let(:options) { { "entry-point" => "app.rb" } }
        let(:options_manager) { Tebako::OptionsManager.new(options) }

        it "returns package name based on entry-point without extension" do
          expected_package = File.join(Dir.pwd, "app")
          expect(options_manager.package).to eq(expected_package)
        end
      end

      context 'when mode is "classic"' do
        let(:options) { { "mode" => "classic", "entry-point" => "app.rb" } }
        let(:options_manager) { Tebako::OptionsManager.new(options) }

        it "returns package name based on entry-point without extension" do
          expected_package = File.join(Dir.pwd, "app")
          expect(options_manager.package).to eq(expected_package)
        end
      end
    end

    context "when the package path is relative" do
      let(:options) { { "output" => "relative/path/to/package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      before do
        allow(options_manager).to receive(:fs_current).and_return("/current/fs/path")
        allow(options_manager).to receive(:relative?).and_return(true)
      end

      it "joins with fs_current to create absolute path" do
        expected_package = File.join("/current/fs/path", "relative/path/to/package")
        expect(options_manager.package).to eq(expected_package)
      end
    end

    context "when the package path is absolute" do
      let(:options) { { "output" => "/absolute/path/to/package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      before do
        allow(options_manager).to receive(:relative?).and_return(false)
      end

      it "uses the path as is" do
        expect(options_manager.package).to eq("/absolute/path/to/package")
      end
    end
  end

  describe "#package_within_root?" do
    context "when the package path is within the root directory" do
      let(:options) { { "root" => "/absolute/path", "output" => "/absolute/path/package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      it "returns true" do
        result = options_manager.package_within_root?
        expect(result).to be(true)
      end
    end

    context "when the package path is outside the root directory" do
      let(:options) { { "root" => "/absolute/path", "output" => "/absolute/otherpath/package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      it "returns false" do
        result = options_manager.package_within_root?
        expect(result).to be(false)
      end
    end

    context "when the package path is outside the root directory (funcky)" do
      let(:options) { { "root" => "/absolute/path/package-dir", "output" => "/absolute/path/package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      it "returns false" do
        result = options_manager.package_within_root?
        expect(result).to be(false)
      end
    end
  end

  describe "#prefix" do
    context 'when @options["prefix"] is nil' do
      let(:options) { { "prefix" => nil } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "calls handle_nil_prefix" do
        expect(options_manager).to receive(:handle_nil_prefix)
        options_manager.prefix
      end
    end

    context 'when @options["prefix"] is "PWD"' do
      let(:options) { { "prefix" => "PWD" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the current working directory" do
        expect(options_manager.prefix).to eq(Dir.pwd)
      end
    end

    context 'when @options["prefix"] is a specific path' do
      let(:specific_path) { "/path/to/somewhere" }
      let(:options) { { "prefix" => specific_path } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the expanded path" do
        expect(options_manager.prefix).to eq(File.expand_path(specific_path))
      end
    end

    context "when the prefix is queried repeatedly" do
      let(:options) { { "prefix" => "some/path" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "resolves the path only once" do
        expect(File).to receive(:expand_path).with("some/path").once.and_return("/cached_value")
        expect(options_manager.prefix).to eq("/cached_value")
        expect(options_manager.prefix).to eq("/cached_value")
      end
    end
  end

  describe "#prefix_within_root?" do
    context "when the prefix path is within the root directory" do
      let(:options) { { "root" => "relative/path", "prefix" => "relative/path/prefix" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns true" do
        expect(options_manager.prefix_within_root?).to be(true)
      end
    end

    context "when the prefix path is outside the root directory" do
      let(:options) { { "root" => "relative/path", "prefix" => "relative/otherpath/prefix" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns false" do
        expect(options_manager.prefix_within_root?).to be(false)
      end
    end

    context "when the prefix path is outside the root directory (funcky case)" do
      let(:options) { { "root" => "relative/path/prefix-dir", "prefix" => "relative/path/prefix" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns false" do
        expect(options_manager.prefix_within_root?).to be(false)
      end
    end
  end

  describe "#press_announce" do
    context 'when mode is "classic" and options["cwd"] is set' do
      let(:options) do
        { "cwd" => "/some/path", "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root",
          "mode" => "classic" }
      end
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      let(:root) { File.join(Dir.pwd, options["root"]) }
      let(:pckg) { File.join(Dir.pwd, "main") }
      let(:prefix) { File.expand_path("~/.tebako") }

      it "returns the correct announcement" do
        options_manager.cfg_options
        expected_announce = <<~ANN
          Running tebako press at #{prefix}
             Mode:                      'classic'
             Ruby version:              '#{Tebako::RubyVersion::DEFAULT_RUBY_VERSION}'
             Project root:              '#{root}'
             Application entry point:   '#{options["entry-point"]}'
             Package file name:         '#{pckg}'
             Loging level:              '#{options["log-level"]}'
             Package working directory: '#{options["cwd"]}'
        ANN
        expect(options_manager.press_announce).to eq(expected_announce)
      end
    end

    context 'when mode is "lean" and options["cwd"] is not set' do
      let(:options) do
        { "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root", "mode" => "lean" }
      end
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      let(:root) { File.join(Dir.pwd, options["root"]) }
      let(:pckg) { File.join(Dir.pwd, "main") }
      let(:prefix) { File.expand_path("~/.tebako") }

      it "returns the correct announcement with default cwd" do
        options_manager.cfg_options
        expected_announce = <<~ANN
          Running tebako press at #{prefix}
             Mode:                      'lean'
             Ruby version:              '#{Tebako::RubyVersion::DEFAULT_RUBY_VERSION}'
             Project root:              '#{root}'
             Application entry point:   '#{options["entry-point"]}'
             Package file name:         '#{pckg}'
             Loging level:              '#{options["log-level"]}'
             Package working directory: '<Host current directory>'
        ANN
        expect(options_manager.press_announce).to eq(expected_announce)
      end
    end
  end

  describe "#process_gemfile" do
    let(:options) { { "Ruby" => "3.2.6" } }
    let(:options_manager) { described_class.new(options) }
    let(:gemfile_path) { "/path/to/Gemfile" }
    let(:mock_ruby_ver) { "3.2.6" }
    let(:mock_rv) { instance_double(Tebako::RubyVersionWithGemfile) }

    before do
      allow(File).to receive(:dirname).with(gemfile_path).and_return("/path/to")
      allow(File).to receive(:basename).with(gemfile_path).and_return("Gemfile")
      allow(Dir).to receive(:chdir).and_yield
      allow(Tebako::RubyVersionWithGemfile).to receive(:new).and_return(mock_rv)
      allow(mock_rv).to receive(:ruby_version).and_return(mock_ruby_ver)
    end

    it "processes gemfile and updates ruby version info" do
      expect(Dir).to receive(:chdir).with("/path/to")
      expect(Tebako::RubyVersionWithGemfile).to receive(:new).with(options["Ruby"], "Gemfile")

      options_manager.process_gemfile(gemfile_path)

      expect(options_manager.rv).to eq(mock_rv)
      expect(options_manager.ruby_ver).to eq(mock_ruby_ver)
    end

    context "when RubyVersionWithGemfile raises error" do
      before do
        allow(Tebako::RubyVersionWithGemfile).to receive(:new)
          .and_raise(Tebako::Error.new("Gemfile error", 1))
      end

      it "propagates the error" do
        expect { options_manager.process_gemfile(gemfile_path) }
          .to raise_error(Tebako::Error)
      end
    end
  end

  describe "#relative?" do
    let(:options_manager) { Tebako::OptionsManager.new({}) }
    it "returns true for a relative path" do
      expect(options_manager.relative?("relative/path")).to be true
    end

    it "returns false for an absolute path" do
      expect(options_manager.relative?("/absolute/path")).to be false
    end
  end

  describe "#root" do
    context 'when options["root"] is a relative path' do
      let(:options) { { "root" => "relative/path" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the correct root path" do
        expected_root = File.join(Dir.pwd, options["root"])
        expect(options_manager.root).to eq(expected_root)
      end
    end

    context 'when options["root"] is an absolute path' do
      let(:options) { { "root" => "/absolute/path" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the correct root path" do
        expected_root = File.join(options["root"], "")
        expect(options_manager.root).to eq(expected_root)
      end
    end
  end

  describe "#images" do
    context "when --image is not given" do
      let(:options_manager) { Tebako::OptionsManager.new({}) }

      it "returns an empty list" do
        expect(options_manager.images).to eq([])
      end
    end

    context "when --image is given once" do
      let(:options_manager) { Tebako::OptionsManager.new({ "image" => ["/data/extra.tfs:extra"] }) }

      it "parses path and mount point with the dwarfs format" do
        expect(options_manager.images).to eq([{ path: "/data/extra.tfs", mount_point: "extra",
                                                format_id: Tebako::Stitcher::FORMAT_DWARFS }])
      end
    end

    context "when --image is repeated" do
      let(:options_manager) { Tebako::OptionsManager.new({ "image" => ["a.tfs:/mnt/a", "b.tfs:/mnt/b"] }) }

      it "parses every entry" do
        expect(options_manager.images.map { |image| image[:mount_point] }).to eq(["/mnt/a", "/mnt/b"])
      end
    end

    context "when the path contains a colon (Windows drive letter)" do
      let(:options_manager) { Tebako::OptionsManager.new({ "image" => ["C:/data/extra.tfs:extra"] }) }

      it "splits on the last colon" do
        expect(options_manager.images.first[:path]).to eq("C:/data/extra.tfs")
        expect(options_manager.images.first[:mount_point]).to eq("extra")
      end
    end

    context "when the mount point is missing" do
      let(:options_manager) { Tebako::OptionsManager.new({ "image" => ["a.tfs"] }) }

      it "fails with error 130" do
        expect { options_manager.images }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(130) }
      end
    end
  end

  describe "#host_platform" do
    let(:options_manager) { Tebako::OptionsManager.new({}) }

    it "maps darwin/arm64 to macos-arm64" do
      expect(options_manager.host_platform("arm64-darwin23", "arm64")).to eq("macos-arm64")
    end

    it "maps linux/x86_64 to linux-gnu-x86_64" do
      expect(options_manager.host_platform("x86_64-linux", "x86_64")).to eq("linux-gnu-x86_64")
    end

    it "maps linux-musl/aarch64 to linux-musl-arm64" do
      expect(options_manager.host_platform("aarch64-linux-musl", "aarch64")).to eq("linux-musl-arm64")
    end

    it "maps msys to windows" do
      expect(options_manager.host_platform("x64-mingw-ucrt", "x86_64")).to eq("windows-x86_64")
    end

    it "rejects an unsupported os with error 112" do
      expect { options_manager.host_platform("sparc-solaris", "sparc") }
        .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(112) }
    end
  end
end

# rubocop:enable Metrics/BlockLength
