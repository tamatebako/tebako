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

require "fileutils"
require_relative "../lib/tebako/codegen"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::Codegen do
  let(:options_manager) do
    double("options_manager", cwd: "app", deps: "deps_path", l_level: "INFO", output_folder: "output")
  end
  let(:scenario_manager) { double("scenario_manager", fs_mount_point: "/mount", fs_entry_point: "/entry") }
  let(:v_parts) { [1, 0, 0] }

  describe ".package_cwd" do
    it 'returns "nullptr" when cwd is nil' do
      allow(options_manager).to receive(:cwd).and_return(nil)
      expect(described_class.package_cwd(options_manager, scenario_manager)).to eq("nullptr")
    end

    it "returns the mount point path when cwd is present" do
      expect(described_class.package_cwd(options_manager, scenario_manager)).to eq("\"/mount/app\"")
    end
  end

  describe ".generate_tebako_fs_cpp" do
    let(:file_mock) { instance_double("File") }
    let(:expected_path) { File.join(options_manager.deps, "src", "tebako", "tebako-fs.cpp") }

    before do
      allow(FileUtils).to receive(:mkdir_p)
      allow(File).to receive(:open).with(expected_path, "w").and_yield(file_mock)
      allow(file_mock).to receive(:write)
    end

    it "creates the necessary directories" do
      described_class.generate_tebako_fs_cpp(options_manager, scenario_manager)
      expect(FileUtils).to have_received(:mkdir_p).with(File.dirname(expected_path))
    end

    it "writes COMMON_C_HEADER and tebako_fs_cpp output to the file" do
      described_class.generate_tebako_fs_cpp(options_manager, scenario_manager)
      expect(file_mock).to have_received(:write).with(Tebako::Codegen::COMMON_C_HEADER)
      expect(file_mock).to have_received(:write).with(Tebako::Codegen.tebako_fs_cpp(options_manager, scenario_manager))
    end
  end

  describe ".generate_tebako_version_h" do
    let(:file_mock) { instance_double("File") }
    let(:expected_path) { File.join(options_manager.deps, "include", "tebako", "tebako-version.h") }

    before do
      allow(FileUtils).to receive(:mkdir_p)
      allow(File).to receive(:open).with(expected_path, "w").and_yield(file_mock)
      allow(file_mock).to receive(:write)
    end

    it "creates the necessary directories" do
      described_class.generate_tebako_version_h(options_manager, v_parts)
      expect(FileUtils).to have_received(:mkdir_p).with(File.dirname(expected_path))
    end

    it "writes COMMON_C_HEADER and tebako_version_h output to the file" do
      described_class.generate_tebako_version_h(options_manager, v_parts)
      expect(file_mock).to have_received(:write).with(Tebako::Codegen::COMMON_C_HEADER)
      expect(file_mock).to have_received(:write).with(Tebako::Codegen.tebako_version_h(v_parts))
    end
  end

  describe ".tebako_fs_cpp" do
    it "generates the correct cpp code snippet based on managers" do
      result = described_class.tebako_fs_cpp(options_manager, scenario_manager)
      expect(result).to include('const  char * fs_log_level   = "INFO";')
      expect(result).to include('const  char * fs_mount_point = "/mount";')
      expect(result).to include('INCBIN(fs, "output/p/fs.bin");')
    end
  end

  describe ".tebako_version_h" do
    it "generates the correct header file with version numbers" do
      result = described_class.tebako_version_h(v_parts)
      expect(result).to include("const unsigned int tebako_version_major = 1;")
      expect(result).to include("const unsigned int tebako_version_minor = 0;")
      expect(result).to include("const unsigned int tebako_version_teeny = 0;")
    end
  end
end
# rubocop:enable Metrics/BlockLength
