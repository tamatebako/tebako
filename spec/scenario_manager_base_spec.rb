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

    context "on msys platform reported as cygwin" do
      before do
        stub_const("RUBY_PLATFORM", "cygwin")
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

  describe "#b_env" do
    before do
      @original_cxxflags = ENV.fetch("CXXFLAGS", nil)
    end

    after do
      ENV["CXXFLAGS"] = @original_cxxflags
    end

    context "when host OS is Darwin" do
      it "sets CXXFLAGS with TARGET_OS_SIMULATOR and TARGET_OS_IPHONE" do
        stub_const("RUBY_PLATFORM", "darwin")
        ENV["CXXFLAGS"] = "-O2"

        expected_flags = "-DTARGET_OS_SIMULATOR=0 -DTARGET_OS_IPHONE=0  -O2"
        expect(described_class.new.b_env["CXXFLAGS"]).to eq(expected_flags)
      end
    end

    context "when host OS is not Darwin" do
      it "sets CXXFLAGS with the value from ENV" do
        stub_const("RUBY_PLATFORM", "linux")
        ENV["CXXFLAGS"] = "-O2"

        expected_flags = "-O2"
        expect(described_class.new.b_env["CXXFLAGS"]).to eq(expected_flags)
      end
    end

    context "when CXXFLAGS is not set in ENV" do
      it "sets CXXFLAGS to nil" do
        stub_const("RUBY_PLATFORM", "linux")
        ENV.delete("CXXFLAGS")

        expect(described_class.new.b_env["CXXFLAGS"]).to be_nil
      end
    end
  end

  describe "#linux?" do
    context "on linux platform" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it "returns true" do
        expect(described_class.new.linux?).to be true
      end
    end

    context "on non-linux platform" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it "returns false" do
        expect(described_class.new.linux?).to be false
      end
    end
  end

  describe "#musl?" do
    context "on linux-musl platform" do
      before do
        stub_const("RUBY_PLATFORM", "linux-musl")
      end

      it "returns true" do
        expect(described_class.new.musl?).to be true
      end
    end

    context "on non-linux platform" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it "returns false" do
        expect(described_class.new.musl?).to be false
      end
    end
  end

  describe "#m_files" do
    context "when on a Linux platform" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
      end

      it 'returns "Unix Makefiles"' do
        expect(described_class.new.m_files).to eq("Unix Makefiles")
      end
    end

    context "when on a macOS platform" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
      end

      it 'returns "Unix Makefiles"' do
        expect(described_class.new.m_files).to eq("Unix Makefiles")
      end
    end

    context "when on a Windows platform" do
      before do
        stub_const("RUBY_PLATFORM", "msys")
      end

      it 'returns "MinGW Makefiles"' do
        expect(described_class.new.m_files).to eq("MinGW Makefiles")
      end
    end

    context "when on an unsupported platform" do
      before do
        stub_const("RUBY_PLATFORM", "unsupported")
      end

      it "raises a Tebako::Error" do
        expect { described_class.new.m_files }.to raise_error(Tebako::Error, "unsupported is not supported.")
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

    context "on msys platform reported as cygwin" do
      before do
        stub_const("RUBY_PLATFORM", "cygwin")
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

  describe "#ncores" do
    context "when on macOS" do
      before do
        stub_const("RUBY_PLATFORM", "darwin")
        status_double = double(exitstatus: 0, signaled?: false)
        allow(Open3).to receive(:capture2e).with("sysctl", "-n", "hw.ncpu").and_return(["4", status_double])
      end

      it "returns the number of cores" do
        expect(described_class.new.ncores).to eq(4)
      end
    end

    context "when on Linux" do
      before do
        stub_const("RUBY_PLATFORM", "linux")
        status_double = double(exitstatus: 0, signaled?: false)
        allow(Open3).to receive(:capture2e).with("nproc", "--all").and_return(["8", status_double])
      end

      it "returns the number of cores" do
        expect(described_class.new.ncores).to eq(8)
      end
    end

    context "when the command fails" do
      before do
        status_double = double(exitstatus: 1, signaled?: false)
        allow(Open3).to receive(:capture2e).and_return(["", status_double])
      end

      it "returns 4 as a default value" do
        expect(described_class.new.ncores).to eq(4)
      end
    end

    context "when the command is terminated by a signal" do
      before do
        status_double = double(exitstatus: nil, signaled?: true, termsig: 9)
        allow(Open3).to receive(:capture2e).and_return(["", status_double])
      end

      it "returns 4 as a default value" do
        expect(described_class.new.ncores).to eq(4)
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
