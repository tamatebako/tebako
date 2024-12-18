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

require "spec_helper"
require "tebako/stripper"
require "fileutils"
require "open3"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::Stripper do
  describe ".strip" do
    let(:scm) { double("scm") }
    let(:src_dir) { "/path/to/src" }

    before do
      allow(described_class).to receive(:strip_bs)
      allow(described_class).to receive(:strip_fi)
      allow(described_class).to receive(:strip_li)
    end

    it "calls strip_bs with the correct parameters" do
      expect(described_class).to receive(:strip_bs).with(src_dir)
      described_class.strip(scm, src_dir)
    end

    it "calls strip_fi with the correct parameters" do
      expect(described_class).to receive(:strip_fi).with(scm, src_dir)
      described_class.strip(scm, src_dir)
    end

    it "calls strip_li with the correct parameters" do
      expect(described_class).to receive(:strip_li).with(scm, src_dir)
      described_class.strip(scm, src_dir)
    end
  end

  describe ".strip_file" do
    let(:file_in) { "/path/to/file_in" }
    let(:file_out) { "/path/to/file_out" }

    context "when file_out is not provided" do
      it "runs strip with the correct parameters" do
        params = ["strip", "-S", file_in]
        expect(Open3).to receive(:capture2e).with(*params).and_return(["",
                                                                       instance_double(Process::Status, exitstatus: 0)])
        described_class.strip_file(file_in)
      end
    end

    context "when file_out is provided" do
      it "runs strip with the correct parameters" do
        params = ["strip", "-S", file_in, "-o", file_out]
        expect(Open3).to receive(:capture2e).with(*params).and_return(["",
                                                                       instance_double(Process::Status, exitstatus: 0)])
        described_class.strip_file(file_in, file_out)
      end
    end

    context "when strip command fails" do
      it "prints a warning message" do
        params = ["strip", "-S", file_in]
        expect(Open3).to receive(:capture2e).with(*params).and_return(["error message",
                                                                       instance_double(Process::Status, exitstatus: 1)])
        expect { described_class.strip_file(file_in) }.to output(/Warning: could not strip/).to_stdout
      end
    end
  end

  describe ".get_files" do
    let(:scm) { double("scm", exe_suffix: ".exe") }

    it "returns the correct list of files" do
      expected_files = Tebako::Stripper::BIN_FILES.flat_map do |f|
        [f, "#{f}#{Tebako::Stripper::CMD_SUFFIX}", "#{f}#{Tebako::Stripper::BAT_SUFFIX}"]
      end
      expected_files += ["ruby.exe", "rubyw.exe"]

      expect(described_class.send(:get_files, scm)).to eq(expected_files)
    end
  end

  describe ".strip_bs" do
    let(:src_dir) { "/path/to/src" }

    it "removes the share directory" do
      expect(FileUtils).to receive(:rm_rf)
        .with([File.join(src_dir, "share"), File.join(src_dir, "include"), File.join(src_dir, "lib", "pkgconfig")])
      described_class.send(:strip_bs, src_dir)
    end
  end

  describe ".strip_fi" do
    let(:scm) { double("scm", exe_suffix: ".exe") }
    let(:src_dir) { "/path/to/src" }
    let(:files) do
      Tebako::Stripper::BIN_FILES.flat_map { |f|
        ["#{src_dir}/bin/#{f}", "#{src_dir}/bin/#{f}.cmd",
         "#{src_dir}/bin/#{f}.bat"]
      } + ["#{src_dir}/bin/ruby.exe", "#{src_dir}/bin/rubyw.exe"]
    end

    it "removes the correct files" do
      expect(FileUtils).to receive(:rm).with(files, force: true)
      described_class.send(:strip_fi, scm, src_dir)
    end
  end

  describe ".strip_li" do
    let(:scm) { double("scm", msys?: false, macos?: false) }
    let(:src_dir) { "/path/to/src" }
    let(:file) { "/path/to/src/file.so" }

    before do
      allow(Find).to receive(:find).and_yield(file)
      allow(File).to receive(:directory?).and_return(false)
      allow(File).to receive(:extname).and_return(".so")
    end

    it "strips the correct files" do
      expect(described_class).to receive(:strip_file).with(file)
      described_class.send(:strip_li, scm, src_dir)
    end

    it "removes files with DELETE_EXTENSIONS" do
      allow(File).to receive(:extname).and_return(".o")
      expect(FileUtils).to receive(:rm).with(file)
      described_class.send(:strip_li, scm, src_dir)
    end
  end

  describe ".strip_extensions" do
    let(:scm) { double("scm", msys?: false, macos?: false) }

    it "returns the correct extensions for non-msys and non-macos" do
      expect(described_class.send(:strip_extensions, scm)).to eq(["so"])
    end

    it "returns the correct extensions for msys" do
      allow(scm).to receive(:msys?).and_return(true)
      expect(described_class.send(:strip_extensions, scm)).to eq(%w[so dll])
    end

    it "returns the correct extensions for macos" do
      allow(scm).to receive(:macos?).and_return(true)
      expect(described_class.send(:strip_extensions, scm)).to eq(%w[so dylib bundle])
    end
  end
end

# rubocop:enable Metrics/BlockLength
