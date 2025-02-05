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
require_relative "../lib/tebako/packager"

# rubocop:disable Metrics/BlockLength
RSpec.describe Tebako::Packager do
  describe "#create_implib" do
    let(:src_dir) { "/path/to/src" }
    let(:package_src_dir) { "/path/to/package/src" }
    let(:app_name) { "my_app" }
    let(:ruby_ver) { Tebako::RubyVersion.new("3.2.5") }

    before do
      allow(Tebako::Packager).to receive(:create_def)
      allow(Tebako::BuildHelpers).to receive(:run_with_capture)
    end

    it "calls create_def with the correct parameters" do
      expect(Tebako::Packager).to receive(:create_def).with(src_dir, app_name)
      Tebako::Packager.create_implib(src_dir, package_src_dir, app_name, ruby_ver)
    end

    it "runs dlltool with the correct parameters" do
      params = ["dlltool", "-d",
                Tebako::Packager.send(:def_fname, src_dir, app_name), "-D",
                Tebako::Packager.send(:out_fname, app_name), "--output-lib",
                Tebako::Packager.send(:lib_fname, package_src_dir, ruby_ver)]
      expect(Tebako::BuildHelpers).to receive(:run_with_capture).with(params)
      Tebako::Packager.create_implib(src_dir, package_src_dir, app_name, ruby_ver)
    end
  end

  describe "#deploy" do
    let(:target_dir) { "/path/to/target" }
    let(:pre_dir) { "/path/to/pre" }
    let(:ruby_ver) { "2.7.2" }
    let(:fs_root) { "/path/to/fs_root" }
    let(:fs_entrance) { "/path/to/fs_entrance" }
    let(:cwd) { "/path/to/cwd" }
    let(:deploy_helper) { instance_double(Tebako::DeployHelper) }

    before do
      allow(Tebako::DeployHelper).to receive(:new).and_return(deploy_helper)
      allow(deploy_helper).to receive(:configure)
      allow(deploy_helper).to receive(:deploy)
      allow(Tebako::Stripper).to receive(:strip)
    end

    it "creates a new DeployHelper with the correct parameters" do
      expect(Tebako::DeployHelper).to receive(:new)
        .with(fs_root, fs_entrance, target_dir, pre_dir)
        .and_return(deploy_helper)
      Tebako::Packager.deploy(target_dir, pre_dir, ruby_ver, fs_root, fs_entrance, cwd)
    end

    it "configures the DeployHelper with the correct parameters" do
      expect(deploy_helper).to receive(:configure).with(ruby_ver, cwd)
      Tebako::Packager.deploy(target_dir, pre_dir, ruby_ver, fs_root, fs_entrance, cwd)
    end

    it "calls deploy on the DeployHelper" do
      expect(deploy_helper).to receive(:deploy)
      Tebako::Packager.deploy(target_dir, pre_dir, ruby_ver, fs_root, fs_entrance, cwd)
    end

    it "calls strip on the Tebako::Stripper" do
      expect(Tebako::Stripper).to receive(:strip).with(deploy_helper, target_dir)
      Tebako::Packager.deploy(target_dir, pre_dir, ruby_ver, fs_root, fs_entrance, cwd)
    end
  end

  describe "#finalize" do
    let(:os_type) { "linux" }
    let(:src_dir) { "/path/to/src" }
    let(:app_name) { "my_app" }
    let(:ruby_ver) { "2.7.2" }
    let(:patchelf) { "/usr/bin/patchelf" }
    let(:ruby_builder) { instance_double(Tebako::RubyBuilder) }

    before do
      allow(Tebako::RubyBuilder).to receive(:new).and_return(ruby_builder)
      allow(ruby_builder).to receive(:target_build)
      allow_any_instance_of(Tebako::ScenarioManagerBase).to receive(:exe_suffix).and_return("")
      allow(Tebako::Packager).to receive(:patchelf)
      allow(Tebako::Packager).to receive(:strip_or_copy)
    end

    it "creates a new RubyBuilder with the correct parameters" do
      expect(Tebako::RubyBuilder).to receive(:new).with(ruby_ver, src_dir).and_return(ruby_builder)
      Tebako::Packager.finalize(os_type, src_dir, app_name, ruby_ver, patchelf)
    end

    it "calls target_build on the RubyBuilder" do
      expect(ruby_builder).to receive(:target_build)
      Tebako::Packager.finalize(os_type, src_dir, app_name, ruby_ver, patchelf)
    end

    it "calls patchelf with the correct parameters" do
      src_name = File.join(src_dir, "ruby")
      expect(Tebako::Packager).to receive(:patchelf).with(src_name, patchelf)
      Tebako::Packager.finalize(os_type, src_dir, app_name, ruby_ver, patchelf)
    end

    it "calls strip_or_copy with the correct parameters" do
      src_name = File.join(src_dir, "ruby")
      package_name = app_name.to_s
      expect(Tebako::Packager).to receive(:strip_or_copy).with(os_type, src_name, package_name)
      Tebako::Packager.finalize(os_type, src_dir, app_name, ruby_ver, patchelf)
    end
  end

  describe "#init" do
    let(:stash_dir) { "/path/to/stash" }
    let(:src_dir) { "/path/to/src" }
    let(:pre_dir) { "/path/to/pre" }
    let(:bin_dir) { "/path/to/bin" }

    before do
      allow(Tebako::Packager::PatchHelpers).to receive(:recreate)
      allow(FileUtils).to receive(:cp_r)
    end

    it "recreates the stash directory" do
      expect(Tebako::Packager::PatchHelpers).to receive(:recreate).with([src_dir, pre_dir, bin_dir])
      described_class.init(stash_dir, src_dir, pre_dir, bin_dir)
    end

    it "copies the source directory to the stash directory" do
      expect(FileUtils).to receive(:cp_r).with("#{stash_dir}/.", src_dir)
      described_class.init(stash_dir, src_dir, pre_dir, bin_dir)
    end
  end

  describe "#mkdwarfs" do
    let(:deps_bin_dir) { "/path/to/deps/bin" }
    let(:data_bin_file) { "/path/to/output.dwarfs" }
    let(:data_src_dir) { "/path/to/src" }
    let(:descriptor) { "/path/to/descriptor" }

    before do
      allow(Tebako::BuildHelpers).to receive(:run_with_capture_v)
    end

    context "when descriptor is not provided" do
      it "runs mkdwarfs with the correct parameters" do
        params = [File.join(deps_bin_dir, "mkdwarfs"), "-o", data_bin_file, "-i", data_src_dir, "--no-progress"]
        expect(Tebako::BuildHelpers).to receive(:run_with_capture_v).with(params)
        described_class.mkdwarfs(deps_bin_dir, data_bin_file, data_src_dir)
      end
    end

    context "when descriptor is provided" do
      it "runs mkdwarfs with the correct parameters including the descriptor" do
        params = [File.join(deps_bin_dir, "mkdwarfs"), "-o", data_bin_file, "-i", data_src_dir, "--no-progress",
                  "--header", descriptor]
        expect(Tebako::BuildHelpers).to receive(:run_with_capture_v).with(params)
        described_class.mkdwarfs(deps_bin_dir, data_bin_file, data_src_dir, descriptor)
      end
    end
  end

  describe "#pass1" do
    let(:ostype) { "linux-gnu" }
    let(:ruby_source_dir) { "/path/to/ruby_source" }
    let(:mount_point) { "/__tebako_memfs__" }
    let(:src_dir) { "/path/to/src" }
    let(:ruby_ver) { Tebako::RubyVersion.new("3.2.6") }
    let(:patch_map) { { "file1" => "patch1", "file2" => "patch2" } }

    before do
      allow(Tebako::Packager::PatchHelpers).to receive(:recreate)
      allow_any_instance_of(Tebako::Packager::Pass1Patch)
        .to receive(:patch_map)
        .and_return(patch_map)
      allow(Tebako::Packager).to receive(:do_patch)
      allow(Tebako::Packager::PatchHelpers).to receive(:restore_and_save_files)
    end

    it "recreates the src directory" do
      expect(Tebako::Packager::PatchHelpers).to receive(:recreate).with(src_dir)
      described_class.pass1(ostype, ruby_source_dir, mount_point, src_dir, ruby_ver)
    end

    it "calls do_patch with the correct parameters" do
      expect(Tebako::Packager).to receive(:do_patch).with(patch_map, ruby_source_dir)
      described_class.pass1(ostype, ruby_source_dir, mount_point, src_dir, ruby_ver)
    end

    it "restores and saves files for FILES_TO_RESTORE" do
      expect(Tebako::Packager::PatchHelpers).to receive(:restore_and_save_files)
        .with(Tebako::Packager::FILES_TO_RESTORE, ruby_source_dir, strict: false)
      described_class.pass1(ostype, ruby_source_dir, mount_point, src_dir, ruby_ver)
    end
  end

  describe "#pass1a" do
    let(:ruby_source_dir) { "/path/to/ruby_source" }
    let(:patch_map) { { "file1" => "patch1", "file2" => "patch2" } }

    before do
      allow_any_instance_of(Tebako::Packager::Pass1APatch).to receive(:patch_map).and_return(patch_map)
      allow(Tebako::Packager).to receive(:do_patch)
    end

    it "calls do_patch with the correct parameters" do
      expect(Tebako::Packager)
        .to receive(:do_patch)
        .with(patch_map, ruby_source_dir)
      described_class.pass1a(ruby_source_dir)
    end
  end

  describe "#pass2" do
    let(:ostype) { "linux-gnu" }
    let(:ruby_source_dir) { "/path/to/ruby_source" }
    let(:deps_lib_dir) { "/path/to/deps/lib" }
    let(:ruby_ver) { "2.7.2" }
    let(:patch_map) { { "file1" => "patch1", "file2" => "patch2" } }

    before do
      allow(Tebako::Packager::Pass2).to receive(:get_patch_map).and_return(patch_map)
      allow(Tebako::Packager).to receive(:do_patch)
    end

    it "calls do_patch with the correct parameters" do
      expect(Tebako::Packager).to receive(:do_patch).with(patch_map, ruby_source_dir)
      described_class.pass2(ostype, ruby_source_dir, deps_lib_dir, ruby_ver)
    end
  end

  describe "#stash" do
    let(:stash_dir) { "/path/to/stash" }
    let(:src_dir) { "/path/to/src" }
    let(:ruby_source_dir) { "/path/to/ruby_source" }
    let(:ruby_ver) { Tebako::RubyVersion.new("3.2.6") }

    before do
      allow(Tebako::Packager::PatchHelpers).to receive(:recreate)
      allow(FileUtils).to receive(:cp_r)
    end

    it "recreates the source directory" do
      expect(Tebako::Packager::PatchHelpers).to receive(:recreate).with(src_dir)
      expect_any_instance_of(Tebako::RubyBuilder).to receive(:toolchain_build)
      described_class.stash(stash_dir, src_dir, ruby_source_dir, ruby_ver)
    end

    it "copies the stash directory to the source directory" do
      expect(FileUtils).to receive(:cp_r).with("#{stash_dir}/.", src_dir)
      expect_any_instance_of(Tebako::RubyBuilder).to receive(:toolchain_build)
      described_class.stash(stash_dir, src_dir, ruby_source_dir, ruby_ver)
    end
  end

  describe "#do_patch" do
    let(:patch_map) { { "file1.txt" => "mapping1", "file2.txt" => "mapping2" } }
    let(:root) { "/path/to/root" }

    it "calls PatchHelpers.patch_file for each file in the patch_map" do
      patch_map.each do |fname, mapping|
        expect(Tebako::Packager::PatchHelpers).to receive(:patch_file).with("#{root}/#{fname}", mapping)
      end

      Tebako::Packager.send(:do_patch, patch_map, root)
    end
  end

  describe "#patchelf" do
    let(:src_name) { "binary" }
    let(:patchelf) { "/usr/bin/patchelf" }

    context "when patchelf is nil" do
      it "does not run patchelf" do
        expect(Tebako::BuildHelpers).not_to receive(:run_with_capture)
        Tebako::Packager.send(:patchelf, src_name, nil)
      end
    end

    context "when patchelf is provided" do
      it "runs patchelf with the correct parameters" do
        params = [patchelf, "--remove-needed-version", "libpthread.so.0", "GLIBC_PRIVATE", src_name]
        expect(Tebako::BuildHelpers).to receive(:run_with_capture).with(params)
        Tebako::Packager.send(:patchelf, src_name, patchelf)
      end
    end
  end

  describe "#strip_or_copy" do
    let(:os_type) { "linux" }
    let(:src_name) { "binary" }
    let(:package_name) { "package" }

    context "when running on MSys" do
      before do
        allow(Tebako::Packager::PatchHelpers).to receive(:msys?).with(os_type).and_return(true)
      end

      it "copies the file" do
        expect(FileUtils).to receive(:cp).with(src_name, package_name)
        Tebako::Packager.send(:strip_or_copy, os_type, src_name, package_name)
      end
    end

    context "when not running on MSys" do
      before do
        allow(Tebako::Packager::PatchHelpers).to receive(:msys?).with(os_type).and_return(false)
      end

      it "strips the file" do
        expect(Tebako::Stripper).to receive(:strip_file).with(src_name, package_name)
        Tebako::Packager.send(:strip_or_copy, os_type, src_name, package_name)
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
