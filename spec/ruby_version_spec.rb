# frozen_string_literal: true

# Copyright (c) 2023-2025 [Ribose Inc](https://www.ribose.com).
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

RSpec.describe Tebako::RubyVersion do
  describe "#initialize" do
    it "initializes with a valid version string" do
      version = Tebako::RubyVersion.new("3.1.6")
      expect(version.ruby_version).to eq("3.1.6")
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

    it "uses default version a nil version string" do
      version = Tebako::RubyVersion.new(nil)
      expect(version.ruby_version).to eq(Tebako::RubyVersion::DEFAULT_RUBY_VERSION)
    end
  end

  describe "#api_version" do
    {
      "2.7.8" => "2.7.0",
      "3.1.6" => "3.1.0",
      "3.2.5" => "3.2.0",
      "3.3.7" => "3.3.0",
      "3.4.2" => "3.4.0"
    }.each do |ruby_version, api_version|
      it "returns '#{api_version}' for Ruby #{ruby_version}" do
        expect(Tebako::RubyVersion.new(ruby_version).api_version).to eq(api_version)
      end
    end
  end

  describe "#version_check" do
    context "when the Ruby version is supported" do
      let(:version) { Tebako::RubyVersion.new("3.2.5") }
      it "does not raise an error" do
        expect { version.version_check }.not_to raise_error
      end
    end

    context "when the Ruby version is not supported" do
      let(:version) { Tebako::RubyVersion.new("2.6.0") }
      it "raises a Tebako::Error" do
        expect do
          version.version_check("2.6.0")
        end.to raise_error(Tebako::Error, "Ruby version 2.6.0 is not supported")
      end
    end
  end

  describe "DEFAULT_RUBY_VERSION" do
    it "is set to 3.3.7" do
      expect(Tebako::RubyVersion::DEFAULT_RUBY_VERSION).to eq("3.3.7")
    end
  end

  describe "#version_check_msys" do
    let(:min_ruby_version_windows) { Gem::Version.new(Tebako::RubyVersion::MIN_RUBY_VERSION_WINDOWS) }

    context "when version is below minimum on Windows" do
      let(:version) { Tebako::RubyVersion.new("3.0.7") }
      it "raises a Tebako::Error" do
        stub_const("RUBY_PLATFORM", "msys")
        expect do
          version.version_check_msys
        end.to raise_error(Tebako::Error, "Ruby version 3.0.7 is not supported on Windows")
      end
    end

    context "when version is minimum on Windows" do
      let(:version) { Tebako::RubyVersion.new("3.1.6") }
      it "does not raise an error" do
        stub_const("RUBY_PLATFORM", "msys")
        expect { version.version_check_msys }.not_to raise_error
      end
    end

    context "when version is above minimum on Windows" do
      let(:version) { Tebako::RubyVersion.new("3.2.5") }
      it "does not raise an error" do
        stub_const("RUBY_PLATFORM", "msys")
        expect { version.version_check_msys }.not_to raise_error
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
