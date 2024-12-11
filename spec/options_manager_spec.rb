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

require "open3"

require "tebako/options_manager"
require "tebako/ruby_version"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::OptionsManager do
  let(:options) { {} }
  let(:ruby_ver) { "3.2.5" }
  let(:ruby_hash) { Tebako::RubyVersion::RUBY_VERSIONS["3.2.5"] }

  describe "#b_env" do
    let(:options_manager) { Tebako::OptionsManager.new({}) }
    before do
      @original_host_os = RbConfig::CONFIG["host_os"]
      @original_cxxflags = ENV.fetch("CXXFLAGS", nil)
    end

    after do
      RbConfig::CONFIG["host_os"] = @original_host_os
      ENV["CXXFLAGS"] = @original_cxxflags
    end

    context "when host OS is Darwin" do
      it "sets CXXFLAGS with TARGET_OS_SIMULATOR and TARGET_OS_IPHONE" do
        RbConfig::CONFIG["host_os"] = "darwin"
        ENV["CXXFLAGS"] = "-O2"

        expected_flags = "-DTARGET_OS_SIMULATOR=0 -DTARGET_OS_IPHONE=0  -O2"
        expect(options_manager.b_env["CXXFLAGS"]).to eq(expected_flags)
      end
    end

    context "when host OS is not Darwin" do
      it "sets CXXFLAGS with the value from ENV" do
        RbConfig::CONFIG["host_os"] = "linux"
        ENV["CXXFLAGS"] = "-O2"

        expected_flags = "-O2"
        expect(options_manager.b_env["CXXFLAGS"]).to eq(expected_flags)
      end
    end

    context "when CXXFLAGS is not set in ENV" do
      it "sets CXXFLAGS to nil" do
        RbConfig::CONFIG["host_os"] = "linux"
        ENV.delete("CXXFLAGS")

        expect(options_manager.b_env["CXXFLAGS"]).to be_nil
      end
    end
  end

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
        exp_opt = "-DCMAKE_BUILD_TYPE=Release -DRUBY_VER:STRING=\"#{ruby_ver}\" -DRUBY_HASH:STRING=\"#{ruby_hash}\" " \
                  "-DDEPS:STRING=\"#{deps}\" -G \"#{m_files}\" -B \"#{output_folder}\" -S \"#{Dir.pwd}\" " \
                  "-DREMOVE_GLIBC_PRIVATE=OFF -DTEBAKO_VERSION:STRING=\"#{v_parts[0]}.#{v_parts[1]}.#{v_parts[2]}\""
        expect(options_manager.cfg_options).to eq(exp_opt)
      end
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

    context "when @fs_current is already set" do
      before do
        options_manager.instance_variable_set(:@fs_current, "/cached/path")
      end

      it "returns the cached value" do
        expect(options_manager.fs_current).to eq("/cached/path")
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
          options_manager.send(:handle_nil_prefix)
        end.to output("No prefix specified, using ~/.tebako\n").to_stdout
        expect(options_manager.send(:handle_nil_prefix)).to eq(File.expand_path("~/.tebako"))
      end
    end

    context "when TEBAKO_PREFIX environment variable is set" do
      let(:env_prefix) { "/custom/prefix" }

      before do
        allow(ENV).to receive(:fetch).with("TEBAKO_PREFIX", nil).and_return(env_prefix)
      end

      it "prints a message and returns the expanded path to the environment variable" do
        expect do
          options_manager.send(:handle_nil_prefix)
        end.to output("Using TEBAKO_PREFIX environment variable as prefix\n").to_stdout
        expect(options_manager.send(:handle_nil_prefix)).to eq(File.expand_path(env_prefix))
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

  describe "#m_files" do
    let(:options) { {} }
    let(:options_manager) { Tebako::OptionsManager.new(options) }
    context "when on a Linux platform" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it 'returns "Unix Makefiles"' do
        expect(options_manager.m_files).to eq("Unix Makefiles")
      end
    end

    context "when on a macOS platform" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it 'returns "Unix Makefiles"' do
        expect(options_manager.m_files).to eq("Unix Makefiles")
      end
    end

    context "when on a Windows platform" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
      end

      it 'returns "MinGW Makefiles"' do
        expect(options_manager.m_files).to eq("MinGW Makefiles")
      end
    end

    context "when on an unsupported platform" do
      before do
        stub_const("RUBY_PLATFORM", "unsupported")
      end

      it "raises a Tebako::Error" do
        expect { options_manager.m_files }.to raise_error(Tebako::Error, "unsupported is not supported.")
      end
    end
  end

  let(:options_manager) { OptionsManager.new(options) }

  describe "#package" do
    context 'when @options["output"] is set' do
      let(:options) { { "output" => "custom_package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the package option" do
        expect(options_manager.package).to eq(File.expand_path("custom_package"))
      end
    end

    context 'when @options["output"] is not set' do
      let(:options) { { "entry-point" => "app.rb" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the default package name based on entry-point" do
        expected_package = File.join(Dir.pwd, "app")
        expect(options_manager.package).to eq(expected_package)
      end
    end

    context "when the package path is relative" do
      let(:options) { { "output" => "relative/path/to/package" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      before do
        allow(options_manager).to receive(:fs_current).and_return("/current/fs/path")
        allow(options_manager).to receive(:relative?).and_return(true)
      end

      it "returns the absolute package path" do
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

      it "returns the absolute package path" do
        expect(options_manager.package).to eq("/absolute/path/to/package")
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

    context 'when @options["prefix"] is already set' do
      let(:options) { { "prefix" => "some/path" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns the cached value" do
        options_manager.instance_variable_set(:@prefix, "cached_value")
        expect(options_manager.prefix).to eq("cached_value")
      end
    end
  end

  describe "#press_announce" do
    context 'when options["cwd"] is set' do
      let(:options) do
        { "cwd" => "/some/path", "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" }
      end
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      let(:root) { File.join(Dir.pwd, options["root"]) }
      let(:pckg) { File.join(Dir.pwd, "main") }
      let(:prefix) { File.expand_path("~/.tebako") }

      it "returns the correct announcement" do
        options_manager.cfg_options
        expected_announce = <<~ANN
          Running tebako press at #{prefix}
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

    context 'when options["cwd"] is not set' do
      let(:options) { { "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      let(:root) { File.join(Dir.pwd, options["root"]) }
      let(:pckg) { File.join(Dir.pwd, "main") }
      let(:prefix) { File.expand_path("~/.tebako") }

      it "returns the correct announcement with default cwd" do
        options_manager.cfg_options
        expected_announce = <<~ANN
          Running tebako press at #{prefix}
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

  describe "#press_options" do
    context 'when options["cwd"] is set' do
      let(:options) do
        { "cwd" => "/some/path", "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" }
      end
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      let(:root) { File.join(Dir.pwd, options["root"]) }
      let(:pckg) { File.join(Dir.pwd, "main") }

      it "returns the correct options string" do
        expected_options = "-DPCKG:STRING='#{pckg}' -DLOG_LEVEL:STRING='#{options["log-level"]}' "
        expect(options_manager.press_options).to eq(expected_options)
      end
    end

    context 'when options["cwd"] is not set' do
      let(:options) { { "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }
      let(:root) { File.join(Dir.pwd, options["root"]) }
      let(:pckg) { File.join(Dir.pwd, "main") }

      it "returns the correct options string with default cwd option" do
        expected_options = "-DPCKG:STRING='#{pckg}' -DLOG_LEVEL:STRING='#{options["log-level"]}' "
        expect(options_manager.press_options).to eq(expected_options)
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

  describe "#remove_glibc_private" do
    context "when running on Linux" do
      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux-gnu")
      end

      context "when patchelf option is set" do
        let(:options) { { "patchelf" => true } }
        let(:options_manager) { Tebako::OptionsManager.new(options) }

        it "returns -DREMOVE_GLIBC_PRIVATE=ON" do
          expect(options_manager.remove_glibc_private).to eq("-DREMOVE_GLIBC_PRIVATE=ON")
        end
      end

      context "when patchelf option is not set" do
        let(:options) { { "patchelf" => false } }
        let(:options_manager) { Tebako::OptionsManager.new(options) }

        it "returns -DREMOVE_GLIBC_PRIVATE=OFF" do
          expect(options_manager.remove_glibc_private).to eq("-DREMOVE_GLIBC_PRIVATE=OFF")
        end
      end
    end

    context "when not running on Gnu Linux" do
      let(:options) { { "patchelf" => true } }
      let(:options_manager) { Tebako::OptionsManager.new(options) }

      it "returns an empty string for MacOS" do
        stub_const("RUBY_PLATFORM", "darwin")
        expect(options_manager.remove_glibc_private).to eq("")
      end

      it "returns an empty string for Musl Linux" do
        stub_const("RUBY_PLATFORM", "linux musl")
        expect(options_manager.remove_glibc_private).to eq("")
      end
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

  describe "#ruby_src_dir" do
    context "when Ruby version is set" do
      let(:options_manager) { Tebako::OptionsManager.new({ "Ruby" => "3.1.6" }) }

      it "returns Ruby source folder name" do
        expected = "#{options_manager.deps}/src/_ruby_3.1.6"
        expect(options_manager.ruby_src_dir).to eq(expected)
      end
    end

    context "when Ruby version is not set" do
      let(:options_manager) { Tebako::OptionsManager.new({}) }

      it "returns Ruby source folder name" do
        expected = "#{options_manager.deps}/src/_ruby_#{Tebako::RubyVersion::DEFAULT_RUBY_VERSION}"
        expect(options_manager.ruby_src_dir).to eq(expected)
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
