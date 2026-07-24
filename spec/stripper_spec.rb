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

require "fileutils"
require "open3"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::Stripper do
  describe ".strip" do
    let(:scm) { double("scm", exe_suffix: ".exe", msys?: false, macos?: false) }
    let(:src_dir) { "/path/to/src" }

    it "removes build artefact directories" do
      allow(Find).to receive(:find)
      expect(FileUtils).to receive(:rm_rf)
        .with([File.join(src_dir, "share"), File.join(src_dir, "include"), File.join(src_dir, "lib", "pkgconfig")])
      described_class.strip(scm, src_dir)
    end

    it "removes the ruby tooling binaries" do
      allow(Find).to receive(:find)
      files = Tebako::Stripper::BIN_FILES.flat_map do |f|
        ["#{src_dir}/bin/#{f}", "#{src_dir}/bin/#{f}.cmd", "#{src_dir}/bin/#{f}.bat"]
      end + ["#{src_dir}/bin/ruby.exe", "#{src_dir}/bin/rubyw.exe"]

      expect(FileUtils).to receive(:rm).with(files, force: true)
      described_class.strip(scm, src_dir)
    end

    context "when walking the output tree" do
      let(:object_file) { "/path/to/src/file.o" }
      let(:shared_file) { "/path/to/src/file.so" }

      before do
        allow(FileUtils).to receive(:rm_rf)
        allow(FileUtils).to receive(:rm)
        allow(File).to receive(:directory?).and_return(false)
      end

      it "removes files with build-artefact extensions" do
        allow(Find).to receive(:find).and_yield(object_file)
        expect(FileUtils).to receive(:rm).with(object_file)
        described_class.strip(scm, src_dir)
      end

      it "strips shared libraries" do
        allow(Find).to receive(:find).and_yield(shared_file)
        allow(File).to receive(:extname).and_return(".so")
        expect(Open3).to receive(:capture2e).with("strip", "-S", shared_file)
                                            .and_return(["", instance_double(Process::Status, exitstatus: 0)])
        described_class.strip(scm, src_dir)
      end

      it "prints a warning when strip fails" do
        allow(Find).to receive(:find).and_yield(shared_file)
        allow(File).to receive(:extname).and_return(".so")
        allow(Open3).to receive(:capture2e).with("strip", "-S", shared_file)
                                           .and_return(["error message",
                                                        instance_double(Process::Status, exitstatus: 1)])
        expect { described_class.strip(scm, src_dir) }.to output(/Warning: could not strip/).to_stdout
      end

      it "strips dylib and bundle files on macOS" do
        mac_scm = double("scm", exe_suffix: "", msys?: false, macos?: true)
        dylib_file = "/path/to/src/file.dylib"
        allow(Find).to receive(:find).and_yield(dylib_file)
        allow(File).to receive(:extname).and_return(".dylib")
        expect(Open3).to receive(:capture2e).with("strip", "-S", dylib_file)
                                            .and_return(["", instance_double(Process::Status, exitstatus: 0)])
        described_class.strip(mac_scm, src_dir)
      end

      it "strips dll files on msys" do
        msys_scm = double("scm", exe_suffix: ".exe", msys?: true, macos?: false)
        dll_file = "/path/to/src/file.dll"
        allow(Find).to receive(:find).and_yield(dll_file)
        allow(File).to receive(:extname).and_return(".dll")
        expect(Open3).to receive(:capture2e).with("strip", "-S", dll_file)
                                            .and_return(["", instance_double(Process::Status, exitstatus: 0)])
        described_class.strip(msys_scm, src_dir)
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
