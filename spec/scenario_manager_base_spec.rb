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

require "tmpdir"
require_relative "../lib/tebako/scenario_manager"

# rubocop:disable Metrics/BlockLength
RSpec.describe Tebako::ScenarioManagerBase do
  describe "#initialize" do
    context "on msys platform" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
      end

      it "sets correct fs_mount_point and exe_suffix" do
        manager = described_class.new
        expect(manager.fs_mount_point).to eq("A:/__tebako_memfs__")
        expect(manager.exe_suffix).to eq(".exe")
      end
    end

    context "on non-msys platform" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it "sets correct fs_mount_point and exe_suffix" do
        manager = described_class.new
        expect(manager.fs_mount_point).to eq("/__tebako_memfs__")
        expect(manager.exe_suffix).to eq("")
      end
    end
  end

  describe "#macos?" do
    context "on macos platform" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it "returns true" do
        expect(described_class.new.macos?).to be true
      end
    end

    context "on non-macos platform" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it "returns false" do
        expect(described_class.new.macos?).to be false
      end
    end
  end

  describe "#msys?" do
    context "on msys platform" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
      end

      it "returns true" do
        expect(described_class.new.msys?).to be true
      end
    end

    context "on non-msys platform" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it "returns false" do
        expect(described_class.new.msys?).to be false
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
