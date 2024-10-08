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

require "tebako/cli_rubies"

RSpec.describe Tebako::CliRubies do # rubocop:disable Metrics/BlockLength
  include Tebako::CliRubies

  describe "#version_check" do
    context "when the Ruby version is supported" do
      it "does not raise an error" do
        expect { version_check("3.2.5") }.not_to raise_error
      end
    end

    context "when the Ruby version is not supported" do
      it "raises a Tebako::Error" do
        expect do
          version_check("2.6.0")
        end.to raise_error(Tebako::Error, "Ruby version 2.6.0 is not supported, exiting")
      end
    end
  end

  describe "DEFAULT_RUBY_VERSION" do
    it "is set to 3.2.5" do
      expect(Tebako::CliRubies::DEFAULT_RUBY_VERSION).to eq("3.2.5")
    end
  end

  describe "#version_check_msys" do
    let(:min_ruby_version_windows) { Gem::Version.new(Tebako::CliRubies::MIN_RUBY_VERSION_WINDOWS) }

    context "when version is below minimum on Windows" do
      it "raises a Tebako::Error" do
        version = "3.0.7"
        stub_const("RUBY_PLATFORM", "msys")
        expect { version_check_msys(version) }.to raise_error(Tebako::Error, /Windows packaging works for Ruby/)
      end
    end

    context "when version is minimum on Windows" do
      it "does not raise an error" do
        version = "3.1.6"
        stub_const("RUBY_PLATFORM", "msys")
        expect { version_check_msys(version) }.not_to raise_error
      end
    end

    context "when version is above minimum on Windows" do
      it "does not raise an error" do
        version = "3.2.5"
        stub_const("RUBY_PLATFORM", "msys")
        expect { version_check_msys(version) }.not_to raise_error
      end
    end
  end
end
