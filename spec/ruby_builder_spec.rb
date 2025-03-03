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

require "tebako/ruby_builder"
require "tebako/build_helpers"
require "tebako/packager/patch_helpers"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::RubyBuilder do
  describe "#target_build" do
    let(:ruby_ver) { "3.1.6" }
    let(:src_dir) { "/path/to/src" }
    let(:ncores) { 4 }
    let(:builder) { described_class.new(Tebako::RubyVersion.new(ruby_ver), src_dir) }
    let(:output_type) { "package" }

    before do
      allow_any_instance_of(Tebako::ScenarioManagerBase).to receive(:ncores).and_return(ncores)
      allow(Tebako::BuildHelpers).to receive(:run_with_capture)
      allow(Dir).to receive(:chdir).with(src_dir).and_yield
    end

    shared_examples "build behavior" do |type|
      it "prints the correct building message" do
        expect { builder.target_build(type) }.to output(/building tebako #{type}/).to_stdout
      end

      it "changes to the source directory" do
        expect(Dir).to receive(:chdir).with(src_dir).and_yield
        builder.target_build(type)
      end

      context "when ruby version is 3.x" do
        it "runs make ruby with the correct number of cores" do
          expect(Tebako::BuildHelpers).to receive(:run_with_capture).with(["make", "ruby", "-j#{ncores}"])
          builder.target_build(type)
        end
      end

      it "runs make with the correct number of cores" do
        expect(Tebako::BuildHelpers).to receive(:run_with_capture).with(["make", "-j#{ncores}"])
        builder.target_build(type)
      end
    end

    context "with 'package' output type" do
      include_examples "build behavior", "package"
    end

    context "with 'runtime package' output type" do
      include_examples "build behavior", "runtime package"
    end
  end

  describe "#toochain_build" do
    let(:ruby_ver) { "3.1.6" }
    let(:src_dir) { "/path/to/src" }
    let(:ncores) { 4 }
    let(:builder) { described_class.new(Tebako::RubyVersion.new(ruby_ver), src_dir) }

    before do
      allow_any_instance_of(Tebako::ScenarioManagerBase).to receive(:ncores).and_return(ncores)
      allow(Tebako::BuildHelpers).to receive(:run_with_capture)
      allow(Dir).to receive(:chdir).with(src_dir).and_yield
    end

    it "prints the building message" do
      expect { builder.toolchain_build }.to output(/building toolchain Ruby/).to_stdout
    end

    it "changes to the source directory" do
      expect(Dir).to receive(:chdir).with(src_dir).and_yield
      builder.toolchain_build
    end

    it "runs make with the correct number of cores" do
      expect(Tebako::BuildHelpers).to receive(:run_with_capture).with(["make", "-j#{ncores}"])
      expect(Tebako::BuildHelpers).to receive(:run_with_capture).with(["make", "install", "-j#{ncores}"])
      builder.toolchain_build
    end
  end
end
# rubocop:enable Metrics/BlockLength
