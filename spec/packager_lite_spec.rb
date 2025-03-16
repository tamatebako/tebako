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

require "pathname"
require_relative "../lib/tebako/packager_lite"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::PackagerLite do
  let(:options_manager) do
    double("OptionsManager", stash_dir: "/tmp/stash", data_src_dir: "/tmp/src", data_pre_dir: "/tmp/pre",
                             data_bin_dir: "/tmp/bin", deps_bin_dir: "/tmp/deps_bin", mode: "both",
                             package: "test_package", rv: "3.2.5", ruby_ver: "3.2.5", root: "/", cwd: "/app",
                             ruby_src_dir: "/tmp/ruby_src", output_type_second: "application package")
  end
  let(:scenario_manager) { double("ScenarioManager", fs_entrance: "/entry") }

  before do
    allow(scenario_manager).to receive(:configure_scenario)
    allow(Tebako::Codegen).to receive(:generate_package_descriptor)
    allow(Tebako::Packager).to receive(:init)
    allow(Tebako::Packager).to receive(:deploy)
    allow(Tebako::Packager).to receive(:mkdwarfs)
    allow(FileUtils).to receive(:rm_f)
  end

  describe "#initialize" do
    it "initializes with options_manager and scenario_manager" do
      packager_lite = described_class.new(options_manager, scenario_manager)
      expect(packager_lite.instance_variable_get(:@opts)).to eq(options_manager)
      expect(packager_lite.instance_variable_get(:@scm)).to eq(scenario_manager)
      expect(scenario_manager).to have_received(:configure_scenario)
    end
  end

  describe "#codegen" do
    it "calls generate_package_descriptor" do
      packager_lite = described_class.new(options_manager, scenario_manager)
      packager_lite.codegen
      expect(Tebako::Codegen).to have_received(:generate_package_descriptor).with(options_manager, scenario_manager)
    end
  end

  describe "#create_package" do
    let(:packager_lite) { described_class.new(options_manager, scenario_manager) }

    before do
      allow(packager_lite).to receive(:codegen).and_return("codegen_result")
      allow(scenario_manager).to receive(:msys?).and_return(true)
      allow(Tebako::Packager).to receive(:create_def)
      allow(Tebako::Packager).to receive(:create_implib)
    end

    it "calls Packager methods to create the package" do
      packager_lite.create_package
      expect(FileUtils).to have_received(:rm_f).with("test_package.tebako")
      expect(Tebako::Packager).to have_received(:mkdwarfs).with("/tmp/deps_bin", "test_package.tebako", "/tmp/src",
                                                                "codegen_result")
    end

    it "prints the correct completion message" do
      expect { packager_lite.create_package }.to output(
        /Created tebako application package at "test_package\.tebako"/
      ).to_stdout
    end
  end

  describe "#deploy" do
    context "when msys? is true" do
      it "calls create_implib and Packager methods to deploy the package" do
        allow(scenario_manager).to receive(:msys?).and_return(true)
        packager_lite = described_class.new(options_manager, scenario_manager)
        allow(packager_lite).to receive(:create_implib)
        allow(scenario_manager).to receive(:msys?).and_return(true)
        packager_lite.deploy
        expect(packager_lite).to have_received(:create_implib)
        expect(Tebako::Packager).to have_received(:init).with("/tmp/stash", "/tmp/src", "/tmp/pre", "/tmp/bin")
        expect(Tebako::Packager).to have_received(:deploy).with("/tmp/src", "/tmp/pre", "3.2.5", "/", "/entry", "/app")
      end
    end

    context "when msys? is false" do
      it "does not call create_implib but calls Packager methods to deploy the package" do
        allow(scenario_manager).to receive(:msys?).and_return(false)
        packager_lite = described_class.new(options_manager, scenario_manager)
        allow(packager_lite).to receive(:create_implib)
        allow(scenario_manager).to receive(:msys?).and_return(false)
        packager_lite.deploy
        expect(packager_lite).not_to have_received(:create_implib)
        expect(Tebako::Packager).to have_received(:init).with("/tmp/stash", "/tmp/src", "/tmp/pre", "/tmp/bin")
        expect(Tebako::Packager).to have_received(:deploy).with("/tmp/src", "/tmp/pre", "3.2.5", "/", "/entry", "/app")
      end
    end
  end

  describe "#name" do
    it "returns the correct package name" do
      packager_lite = described_class.new(options_manager, scenario_manager)
      expect(packager_lite.name).to eq("test_package.tebako")
    end
  end
end

# rubocop:enable Metrics/BlockLength
