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

require "tebako/build_helpers"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::RubyVersion do
  describe "#initialize" do
    it "initializes with a valid version string" do
      version = Tebako::RubyVersion.new("3.1.0")
      expect(version.instance_variable_get(:@ruby_ver)).to eq("3.1.0")
    end

    it "raises an error with an invalid version string" do
      expect do
        Tebako::RubyVersion.new("3.1")
      end.to raise_error(Tebako::Error, "Invalid Ruby version format '3.1'. Expected format: x.y.z")
    end

    it "raises an error with an empty version string" do
      expect do
        Tebako::RubyVersion.new("")
      end.to raise_error(Tebako::Error, "Invalid Ruby version format ''. Expected format: x.y.z")
    end

    it "raises an error with a nil version string" do
      expect do
        Tebako::RubyVersion.new(nil)
      end.to raise_error(Tebako::Error, "Invalid Ruby version format ''. Expected format: x.y.z")
    end
  end

  describe "version checks" do
    context "with version 3.1.0" do
      let(:version) { Tebako::RubyVersion.new("3.1.6") }

      it "returns true for ruby3x?" do
        expect(version.ruby3x?).to be true
      end

      it "returns true for ruby31?" do
        expect(version.ruby31?).to be true
      end

      it "returns false for ruby32?" do
        expect(version.ruby32?).to be false
      end

      it "returns false for ruby32only?" do
        expect(version.ruby32only?).to be false
      end

      it "returns false for ruby33?" do
        expect(version.ruby33?).to be false
      end

      it "returns '3.1.0' for api_version" do
        expect(version.api_version).to eq("3.1.0")
      end

      it "returns '310' for lib_version" do
        expect(version.lib_version).to eq("310")
      end
    end

    context "with version 3.2.0" do
      let(:version) { Tebako::RubyVersion.new("3.2.0") }

      it "returns true for ruby3x?" do
        expect(version.ruby3x?).to be true
      end

      it "returns true for ruby31?" do
        expect(version.ruby31?).to be true
      end

      it "returns true for ruby32?" do
        expect(version.ruby32?).to be true
      end

      it "returns true for ruby32only?" do
        expect(version.ruby32only?).to be true
      end

      it "returns false for ruby33?" do
        expect(version.ruby33?).to be false
      end

      it "returns '3.2.0' for api_version" do
        expect(version.api_version).to eq("3.2.0")
      end

      it "returns '320' for lib_version" do
        expect(version.lib_version).to eq("320")
      end
    end

    context "with version 3.3.0" do
      let(:version) { Tebako::RubyVersion.new("3.3.0") }

      it "returns true for ruby3x?" do
        expect(version.ruby3x?).to be true
      end

      it "returns true for ruby31?" do
        expect(version.ruby31?).to be true
      end

      it "returns true for ruby32?" do
        expect(version.ruby32?).to be true
      end

      it "returns false for ruby32only?" do
        expect(version.ruby32only?).to be false
      end

      it "returns true for ruby33?" do
        expect(version.ruby33?).to be true
      end

      it "returns '3.3.0' for api_version" do
        expect(version.api_version).to eq("3.3.0")
      end

      it "returns '330' for lib_version" do
        expect(version.lib_version).to eq("330")
      end
    end

    context "with version 2.7.0" do
      let(:version) { Tebako::RubyVersion.new("2.7.8") }

      it "returns false for ruby3x?" do
        expect(version.ruby3x?).to be false
      end

      it "returns false for ruby31?" do
        expect(version.ruby31?).to be false
      end

      it "returns false for ruby32?" do
        expect(version.ruby32?).to be false
      end

      it "returns false for ruby32only?" do
        expect(version.ruby32only?).to be false
      end

      it "returns false for ruby33?" do
        expect(version.ruby33?).to be false
      end

      it "returns '2.7.0' for api_version" do
        expect(version.api_version).to eq("2.7.0")
      end

      it "returns '270' for lib_version" do
        expect(version.lib_version).to eq("270")
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
