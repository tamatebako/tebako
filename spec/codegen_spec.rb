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

require "pathname"
require "fileutils"
require_relative "../lib/tebako/codegen"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::Codegen do
  let(:options_manager) do
    double(
      "OptionsManager",
      cwd: nil,
      ruby_ver: "3.2.5",
      ruby_src_dir: "/path/to/ruby",
      data_src_dir: "/path/to/data",
      deps_bin_dir: "/path/to/deps/bin",
      data_app_file: "/path/to/data_app",
      data_pre_dir: "/path/to/data_pre",
      data_bin_dir: "/path/to/data_bin",
      stash_dir: "/path/to/stash",
      package: "example_package",
      mode: "application",
      deps: "/path/to/deps"
    )
  end

  let(:scenario_manager) do
    double(
      "ScenarioManager",
      fs_mount_point: "/mnt",
      fs_entry_point: "/entry_point",
      msys?: false
    )
  end

  let(:scm) { double("ScenarioManager", msys?: true) }

  describe "Constants" do
    it "COMMON_C_HEADER contains the correct C header comment" do
      expect(Tebako::Codegen::COMMON_C_HEADER).to include("THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO")
    end

    it "COMMON_RUBY_HEADER contains the correct Ruby header comment" do
      expect(Tebako::Codegen::COMMON_RUBY_HEADER).to include("THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO")
    end
  end

  describe "#deploy_crt_implib" do
    it "returns correct string when msys? is true" do
      result = described_class.deploy_crt_implib(options_manager, scm)
      expect(result).to include("Tebako::Packager.create_implib")
    end

    it "returns an empty string when msys? is false" do
      allow(scm).to receive(:msys?).and_return(false)
      result = described_class.deploy_crt_implib(options_manager, scm)
      expect(result).to eq("")
    end
  end

  describe "#generate_stub_rb" do
    it "creates a stub.rb file with correct content" do
      allow(FileUtils).to receive(:mkdir_p)
      mock_file = instance_double("File")
      allow(File).to receive(:open).and_yield(mock_file)
      allow(mock_file).to receive(:write)

      described_class.generate_stub_rb(options_manager)

      expected_path = File.join(options_manager.deps, "src", "tebako", "local", "stub.rb")
      expect(FileUtils).to have_received(:mkdir_p).with(File.dirname(expected_path))
      expect(File).to have_received(:open).with(expected_path, "w")
      expect(mock_file).to have_received(:write).with(Tebako::Codegen::COMMON_RUBY_HEADER)
    end
  end

  describe "#generate_deploy_rb" do
    it "creates a deploy.rb file with correct content" do
      allow(FileUtils).to receive(:mkdir_p)
      mock_file = instance_double("File")
      allow(File).to receive(:open).and_yield(mock_file)
      allow(mock_file).to receive(:write)
      allow(File).to receive(:binwrite)

      described_class.generate_deploy_rb(options_manager, scenario_manager)

      expected_path = File.join(options_manager.deps, "bin", "deploy.rb")
      expect(FileUtils).to have_received(:mkdir_p).with(File.dirname(expected_path))
      expect(File).to have_received(:open).with(expected_path, "w")
      expect(mock_file).to have_received(:write).with(Tebako::Codegen::COMMON_RUBY_HEADER)
    end
  end

  describe "#deploy_mk" do
    it 'calls deploy_mk_stub when mode is "both"' do
      allow(options_manager).to receive(:mode).and_return("both")
      allow(described_class).to receive(:deploy_mk_stub).and_return("mk_both_result")
      result = described_class.deploy_mk(options_manager, scm)
      expect(result).to eq("mk_both_result")
    end

    it 'calls deploy_mk_stub when mode is "runtime"' do
      allow(options_manager).to receive(:mode).and_return("runtime")
      allow(described_class).to receive(:deploy_mk_stub).and_return("mk_stub_result")
      result = described_class.deploy_mk(options_manager, scm)
      expect(result).to eq("mk_stub_result")
    end

    it "calls deploy_mk_bundle when mode by default" do
      allow(options_manager).to receive(:mode).and_return("bundle")
      allow(described_class).to receive(:deploy_mk_bundle).and_return("mk_bundle_result")
      result = described_class.deploy_mk(options_manager, scm)
      expect(result).to eq("mk_bundle_result")
    end
  end

  describe "deploy_mk_* methods" do
    let(:options_manager) { double("OptionsManager") }
    let(:scenario_manager) { double("ScenarioManager") }

    let(:deps_bin_dir) { "bin/dir" }
    let(:data_app_file) { "path/to/app/file" }
    let(:data_src_dir) { "path/to/src" }
    let(:data_pre_dir) { "path/to/pre" }
    let(:data_bundle_file) { "path/to/bundle/file" }
    let(:data_stub_file) { "path/to/stub/file" }
    let(:root) { "root/path" }
    let(:fs_entrance) { "/entry" }
    let(:fs_mount_point) { "/mnt" }
    let(:fs_entry_point) { "/entry_point" }
    let(:cwd) { nil }
    let(:deps) { "path/to/deps" }
    let(:root) { "path/to/root" }
    let(:ruby_ver) { "3.2.5" }

    before do
      allow(options_manager).to receive(:deps).and_return(deps)
      allow(options_manager).to receive(:root).and_return(root)
      allow(options_manager).to receive(:deps_bin_dir).and_return(deps_bin_dir)
      allow(options_manager).to receive(:data_app_file).and_return(data_app_file)
      allow(options_manager).to receive(:data_src_dir).and_return(data_src_dir)
      allow(options_manager).to receive(:data_pre_dir).and_return(data_pre_dir)
      allow(options_manager).to receive(:data_bundle_file).and_return(data_bundle_file)
      allow(options_manager).to receive(:data_stub_file).and_return(data_stub_file)
      allow(options_manager).to receive(:cwd).and_return(cwd)
      allow(options_manager).to receive(:ruby_ver).and_return(ruby_ver)

      allow(scenario_manager).to receive(:fs_entrance).and_return(fs_entrance)
      allow(scenario_manager).to receive(:root).and_return(root)
      allow(scenario_manager).to receive(:fs_entry_point).and_return(fs_entry_point)
      allow(scenario_manager).to receive(:fs_mount_point).and_return(fs_mount_point)
    end

    describe "#deploy_mk_bundle" do
      it "generates the correct script for bundle deployment" do
        result = described_class.deploy_mk_bundle(options_manager, scenario_manager)
        expected = <<~SUBST
          Tebako::Packager.deploy("#{data_src_dir}", "#{data_pre_dir}",
                                  rv , "#{root}", "#{fs_entrance}", "#{options_manager.cwd}")
          Tebako::Packager.mkdwarfs("#{deps_bin_dir}", "#{data_bundle_file}",
                                    "#{data_src_dir}")
        SUBST

        expect(result).to eq(expected)
      end
    end

    describe "#deploy_mk_stub" do
      it "generates the correct script for stub deployment" do
        result = described_class.deploy_mk_stub(options_manager)
        expected = <<~SUBST
          Tebako::Packager.deploy("#{data_src_dir}", "#{data_pre_dir}",
                                  rv, "#{deps}/src/tebako/local", "stub.rb", nil)
          Tebako::Packager.mkdwarfs("#{deps_bin_dir}", "#{data_stub_file}", "#{data_src_dir}")
        SUBST

        expect(result).to eq(expected)
      end
    end
  end

  describe "#package_cwd" do
    let(:fs_mount_point) { "/mnt" }

    before do
      allow(scenario_manager).to receive(:fs_mount_point).and_return(fs_mount_point)
    end

    context "when cwd is nil" do
      it 'returns "nullptr"' do
        allow(options_manager).to receive(:cwd).and_return(nil)
        result = described_class.package_cwd(options_manager, scenario_manager)
        expect(result).to eq("nullptr")
      end
    end

    context "when cwd is not nil" do
      it "returns the correctly formatted path" do
        cwd = "custom/path"
        allow(options_manager).to receive(:cwd).and_return(cwd)
        result = described_class.package_cwd(options_manager, scenario_manager)
        expect(result).to eq("\"#{fs_mount_point}/#{cwd}\"")
      end
    end
  end

  describe "#tebako_version_h" do
    it "generates the correct version header content" do
      v_parts = [1, 0, 0]
      result = described_class.tebako_version_h(v_parts)
      expect(result).to include("const unsigned int tebako_version_major = 1;")
      expect(result).to include("const unsigned int tebako_version_minor = 0;")
      expect(result).to include("const unsigned int tebako_version_teeny = 0;")
    end
  end

  describe "#tebako_fs_cpp" do
    it 'calls tebako_fs_cpp_stub when mode is "runtime"' do
      allow(options_manager).to receive(:mode).and_return("runtime")
      allow(described_class).to receive(:tebako_fs_cpp_stub).and_return("stub_result")
      result = described_class.tebako_fs_cpp(options_manager, scenario_manager)
      expect(result).to eq("stub_result")
    end
  end

  describe "#tebako_fs_cpp_bundle" do
    let(:options_manager) { double("OptionsManager") }
    let(:scenario_manager) { double("ScenarioManager") }

    let(:log_level) { "debug" }
    let(:fs_mount_point) { "/mnt/bundle" }
    let(:fs_entry_point) { "/app/start.rb" }
    let(:cwd) { "working_directory" }
    let(:data_bundle_file) { "path/to/data.bundle" }

    before do
      allow(options_manager).to receive(:l_level).and_return(log_level)
      allow(options_manager).to receive(:data_bundle_file).and_return(data_bundle_file)
      allow(options_manager).to receive(:cwd).and_return(cwd)

      allow(scenario_manager).to receive(:fs_mount_point).and_return(fs_mount_point)
      allow(scenario_manager).to receive(:fs_entry_point).and_return(fs_entry_point)
    end

    context "when cwd is specified" do
      it "returns the correct C++ code for bundle mode with cwd" do
        result = described_class.tebako_fs_cpp_bundle(options_manager, scenario_manager)

        expected_result = <<~SUBST
          #include <limits.h>
          #include <incbin/incbin.h>

          namespace tebako {
            const  char * fs_log_level   = "#{log_level}";
            const  char * fs_mount_point = "#{fs_mount_point}";
            const  char * fs_entry_point = "#{fs_entry_point}";
            const  char * package_cwd 	 = "#{fs_mount_point}/#{cwd}";
            char   original_cwd[PATH_MAX];

            INCBIN(fs, "#{data_bundle_file}");
          }
        SUBST

        expect(result).to eq(expected_result)
      end
    end

    context "when cwd is nil" do
      before do
        allow(options_manager).to receive(:cwd).and_return(nil)
      end

      it "returns the correct C++ code for bundle mode without cwd" do
        result = described_class.tebako_fs_cpp_bundle(options_manager, scenario_manager)

        expected_result = <<~SUBST
          #include <limits.h>
          #include <incbin/incbin.h>

          namespace tebako {
            const  char * fs_log_level   = "#{log_level}";
            const  char * fs_mount_point = "#{fs_mount_point}";
            const  char * fs_entry_point = "#{fs_entry_point}";
            const  char * package_cwd 	 = nullptr;
            char   original_cwd[PATH_MAX];

            INCBIN(fs, "#{data_bundle_file}");
          }
        SUBST

        expect(result).to eq(expected_result)
      end
    end
  end

  describe "#tebako_fs_cpp_stub" do
    let(:options_manager) { double("OptionsManager") }
    let(:scenario_manager) { double("ScenarioManager") }
    let(:log_level) { "warn" }
    let(:fs_mount_point) { "/mnt/stub" }
    let(:data_stub_file) { "path/to/data.stub" }

    before do
      allow(options_manager).to receive(:l_level).and_return(log_level)
      allow(options_manager).to receive(:data_stub_file).and_return(data_stub_file)
      allow(scenario_manager).to receive(:fs_mount_point).and_return(fs_mount_point)
    end

    it "returns the correct C++ code for stub mode" do
      result = described_class.tebako_fs_cpp_stub(options_manager, scenario_manager)

      expected_result = <<~SUBST
        #include <limits.h>
        #include <incbin/incbin.h>

        namespace tebako {
          const  char * fs_log_level   = "#{log_level}";
          const  char * fs_mount_point = "#{fs_mount_point}";
          const  char * fs_entry_point = "/local/stub.rb";
          const  char * package_cwd 	 = nullptr;
          char   original_cwd[PATH_MAX];

          INCBIN(fs, "#{data_stub_file}");
        }
      SUBST

      expect(result).to eq(expected_result)
    end
  end

  describe "#generate_package_descriptor" do
    it "creates a package descriptor file with correct content" do
      allow(FileUtils).to receive(:mkdir_p)
      allow(File).to receive(:binwrite)

      result = described_class.generate_package_descriptor(options_manager, scenario_manager)

      expected_path = File.join(options_manager.deps, "src", "tebako", "package_descriptor")
      expect(FileUtils).to have_received(:mkdir_p).with(File.dirname(expected_path))
      expect(File).to have_received(:binwrite)
      expect(result).to eq(expected_path)
    end
  end
end

# rubocop:enable Metrics/BlockLength
