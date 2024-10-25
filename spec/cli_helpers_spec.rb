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
  let(:ruby_ver) { "3.2.5" }
  let(:ruby_hash) { Tebako::RubyVersion::RUBY_VERSIONS["3.2.5"] }

  before do
    allow_any_instance_of(Pathname).to receive(:realpath) { |instance| instance }
    allow(Dir).to receive(:exist?).and_call_original
    allow(Dir).to receive(:exist?).with(options["root"]).and_return(true)
    allow(File).to receive(:file?).and_call_original
    allow(File).to receive(:file?).with(/entrypoint/).and_return(true)
  end

  describe "#do_press" do
    let(:options_manager) { Tebako::OptionsManager.new(options) }

    before do
      stub_const("RUBY_PLATFORM", "x86_64-linux")
    end

    it "executes the press command successfully" do
      allow(FileUtils).to receive(:rm_rf)
      allow(self).to receive(:system).and_return(true)
      allow(Tebako::Codegen).to receive(:generate_tebako_version_h).and_return(true)
      allow(Tebako::Codegen).to receive(:generate_tebako_fs_cpp).and_return(true)

      expect(do_press(options_manager)).to be_truthy
    end

    it "raises an error if the press command fails" do
      allow(FileUtils).to receive(:rm_rf)
      allow(self).to receive(:system).and_return(false)
      expect { do_press(options_manager) }.to raise_error(Tebako::Error)
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
        expect(do_setup(options_manager)).to be_truthy
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
