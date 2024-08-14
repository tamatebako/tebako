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
require "tebako/cli_rubies"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::CliHelpers do
  include Tebako::CliHelpers
  include Tebako::CliRubies

  let(:options) { {} }

  before do
    allow(self).to receive(:options).and_return(options)
  end

  describe "#b_env" do
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
        expect(b_env["CXXFLAGS"]).to eq(expected_flags)
      end
    end

    context "when host OS is not Darwin" do
      it "sets CXXFLAGS with the value from ENV" do
        RbConfig::CONFIG["host_os"] = "linux"
        ENV["CXXFLAGS"] = "-O2"

        expected_flags = "-O2"
        expect(b_env["CXXFLAGS"]).to eq(expected_flags)
      end
    end

    context "when CXXFLAGS is not set in ENV" do
      it "sets CXXFLAGS to nil" do
        RbConfig::CONFIG["host_os"] = "linux"
        ENV.delete("CXXFLAGS")

        expect(b_env["CXXFLAGS"]).to be_nil
      end
    end
  end

  describe "#l_level" do
    context "when log-level option is not set" do
      it 'returns "error"' do
        expect(l_level).to eq("error")
      end
    end

    context "when log-level option is set" do
      let(:options) { { "log-level" => "info" } }

      it "returns the value of the log-level option" do
        expect(l_level).to eq("info")
      end
    end
  end

  describe "#m_files" do
    context "when on a Linux platform" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it 'returns "Unix Makefiles"' do
        expect(m_files).to eq("Unix Makefiles")
      end
    end

    context "when on a macOS platform" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it 'returns "Unix Makefiles"' do
        expect(m_files).to eq("Unix Makefiles")
      end
    end

    context "when on a Windows platform" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
      end

      it 'returns "MinGW Makefiles"' do
        expect(m_files).to eq("MinGW Makefiles")
      end
    end

    context "when on an unsupported platform" do
      before do
        stub_const("RUBY_PLATFORM", "unsupported")
      end

      it "raises a Tebako::Error" do
        expect { m_files }.to raise_error(Tebako::Error, "unsupported is not supported, exiting")
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

  describe "#version_key" do
    it "returns the correct version key" do
      allow(self).to receive(:source).and_return("/path/to/source")
      expect(version_key).to eq("#{Tebako::VERSION} at /path/to/source")
    end
  end

  describe "#version_cache" do
    it "returns the correct version and source from the cache file" do
      version_file_content = "#{Tebako::VERSION} at /path/to/source\n"
      allow(self).to receive(:deps).and_return("/path/to/deps")
      allow(File).to receive(:join).with("/path/to/deps", Tebako::E_VERSION_FILE).and_return("/path/to/version_file")
      allow(File).to receive(:open).with("/path/to/version_file").and_yield(StringIO.new(version_file_content))
    end
  end

  describe "#version_cache_check" do
    before do
      allow(self).to receive(:version_cache).and_return(match_data)
      allow(self).to receive(:version_unknown)
      allow(self).to receive(:version_mismatch)
      allow(self).to receive(:version_source_mismatch)
    end

    context "when version cache is unknown" do
      let(:match_data) { nil }

      it "calls version_unknown" do
        expect(self).to receive(:version_unknown)
        version_cache_check
      end
    end

    context "when version does not match" do
      let(:match_data) { { version: "0.7.0", source: "/path/to/source" } }
      it "calls version_mismatch" do
        expect(self).to receive(:version_mismatch).with("0.7.0")
        version_cache_check
      end
    end

    context "when source does not match" do
      let(:match_data) { { version: Tebako::VERSION, source: "/different/source" } }

      it "calls version_source_mismatch" do
        allow(self).to receive(:source).and_return("/path/to/source")
        expect(self).to receive(:version_source_mismatch).with("/different/source")
        version_cache_check
      end
    end

    context "when version and source match" do
      let(:match_data) { { version: Tebako::VERSION, source: "/path/to/source" } }

      it "does not call any mismatch methods" do
        allow(self).to receive(:source).and_return("/path/to/source")
        expect(self).not_to receive(:version_unknown)
        expect(self).not_to receive(:version_mismatch)
        expect(self).not_to receive(:version_source_mismatch)
        version_cache_check
      end
    end
  end
  describe "#version_mismatch" do
    it "prints the correct message and cleans the cache" do
      expect(self).to receive(:puts)
        .with("Tebako cache was created by a gem version 0.9.0 and cannot be used for gem version #{Tebako::VERSION}")
      expect(self).to receive(:clean_cache)
      allow(Tebako).to receive(:VERSION).and_return("1.0.0")
      version_mismatch("0.9.0")
    end
  end
  describe "#version_source_mismatch" do
    it "prints the correct message and cleans the output" do
      expect(self).to receive(:puts)
        .with("CMake cache was created for a different source directory '/different/source' " \
              "and cannot be used for '/path/to/source'")
      expect(self).to receive(:clean_output)
      allow(self).to receive(:source).and_return("/path/to/source")
      version_source_mismatch("/different/source")
    end
  end
  describe "#version_unknown" do
    it "prints the correct message and cleans the cache" do
      expect(self).to receive(:puts).with("CMake cache version was not recognized, cleaning up")
      expect(self).to receive(:clean_cache)
      version_unknown
    end
  end
  describe "#press_announce" do
    context 'when options["cwd"] is set' do
      let(:options) do
        { "cwd" => "/some/path", "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" }
      end

      it "returns the correct announcement" do
        expected_announce = <<~ANN
          Running tebako press at #{prefix}
             Ruby version:              '#{extend_ruby_version[0]}'
             Project root:              '#{root}'
             Application entry point:   '#{options["entry-point"]}'
             Package file name:         '#{package}'
             Loging level:              '#{l_level}'
             Package working directory: '#{options["cwd"]}'
        ANN
        expect(press_announce).to eq(expected_announce)
      end
    end
    context 'when options["cwd"] is not set' do
      let(:options) { { "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" } }

      it "returns the correct announcement with default cwd" do
        expected_announce = <<~ANN
          Running tebako press at #{prefix}
             Ruby version:              '#{extend_ruby_version[0]}'
             Project root:              '#{root}'
             Application entry point:   '#{options["entry-point"]}'
             Package file name:         '#{package}'
             Loging level:              '#{l_level}'
             Package working directory: '<Host current directory>'
        ANN
        expect(press_announce).to eq(expected_announce)
      end
    end
    describe "#press_options" do
      context 'when options["cwd"] is set' do
        let(:options) do
          { "cwd" => "/some/path", "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" }
        end

        it "returns the correct options string" do
          expected_options = "-DROOT:STRING='#{root}' -DENTRANCE:STRING='#{options["entry-point"]}' " \
                             "-DPCKG:STRING='#{package}' -DLOG_LEVEL:STRING='#{options["log-level"]}' " \
                             "-DPACKAGE_NEEDS_CWD:BOOL=ON -DPACKAGE_CWD:STRING='#{options["cwd"]}'"
          expect(press_options).to eq(expected_options)
        end
      end

      context 'when options["cwd"] is not set' do
        let(:options) { { "entry-point" => "main.rb", "log-level" => "info", "root" => "test_root" } }

        it "returns the correct options string with default cwd option" do
          expected_options = "-DROOT:STRING='#{root}' -DENTRANCE:STRING='#{options["entry-point"]}' " \
                             "-DPCKG:STRING='#{package}' -DLOG_LEVEL:STRING='#{options["log-level"]}' " \
                             "-DPACKAGE_NEEDS_CWD:BOOL=OFF"
          expect(press_options).to eq(expected_options)
        end
      end
    end
    describe "#relative?" do
      it "returns true for a relative path" do
        expect(relative?("relative/path")).to be true
      end

      it "returns false for an absolute path" do
        expect(relative?("/absolute/path")).to be false
      end
    end

    describe "#root" do
      context 'when options["root"] is a relative path' do
        let(:options) { { "root" => "relative/path" } }

        it "returns the correct root path" do
          expected_root = File.join(fs_current, options["root"])
          expect(root).to eq(expected_root)
        end
      end

      context 'when options["root"] is an absolute path' do
        let(:options) { { "root" => "/absolute/path" } }

        it "returns the correct root path" do
          expected_root = File.join(options["root"], "")
          expect(root).to eq(expected_root)
        end
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
