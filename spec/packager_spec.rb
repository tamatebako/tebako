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
require "tmpdir"

# rubocop:disable Metrics/BlockLength
RSpec.describe Tebako::Packager do
  describe "#deploy" do
    let(:target_dir) { "/path/to/target" }
    let(:pre_dir) { "/path/to/pre" }
    let(:ruby_ver) { "3.3.7" }
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

  describe "#check_prebuilt_env!" do
    it "passes when mkdwarfs is present in deps/bin" do
      Dir.mktmpdir do |deps_bin_dir|
        FileUtils.touch(File.join(deps_bin_dir, "mkdwarfs"))
        expect { described_class.check_prebuilt_env!(deps_bin_dir) }.not_to raise_error
      end
    end

    it "fails with error 128 when mkdwarfs is missing" do
      Dir.mktmpdir do |deps_bin_dir|
        expect { described_class.check_prebuilt_env!(deps_bin_dir) }
          .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(128) }
      end
    end
  end

  describe "#init" do
    let(:layout_dir) { "/path/to/layout" }
    let(:src_dir) { "/path/to/src" }
    let(:pre_dir) { "/path/to/pre" }
    let(:bin_dir) { "/path/to/bin" }

    before do
      allow(Tebako::BuildHelpers).to receive(:recreate)
      allow(FileUtils).to receive(:cp_r)
    end

    it "recreates the packaging directories" do
      expect(Tebako::BuildHelpers).to receive(:recreate).with([src_dir, pre_dir, bin_dir])
      described_class.init(layout_dir, src_dir, pre_dir, bin_dir)
    end

    it "seeds the packaging environment from the runtime layout" do
      expect(FileUtils).to receive(:cp_r).with("#{layout_dir}/.", src_dir)
      described_class.init(layout_dir, src_dir, pre_dir, bin_dir)
    end
  end

  describe "#align_layout_to_runtime!" do
    let(:ruby_ver) { Tebako::RubyVersion.new("3.3.7") }

    it "renames the image arch dirs to the runtime's and drops in its rbconfig" do
      Dir.mktmpdir do |tmp|
        image = File.join(tmp, "img")
        layout = File.join(tmp, "layout")
        FileUtils.mkdir_p(File.join(image, "lib/ruby/3.3.0/arm64-darwin23"))
        File.write(File.join(image, "lib/ruby/3.3.0/arm64-darwin23/rbconfig.rb"), "local")
        FileUtils.mkdir_p(File.join(image, "lib/ruby/gems/3.3.0/extensions/arm64-darwin-23"))
        FileUtils.mkdir_p(File.join(layout, "lib/ruby/3.3.0/arm64-darwin24"))
        File.write(File.join(layout, "lib/ruby/3.3.0/arm64-darwin24/rbconfig.rb"), "runtime")
        FileUtils.mkdir_p(File.join(layout, "lib/ruby/gems/3.3.0/extensions/arm64-darwin-24"))

        Tebako::Packager.align_layout_to_runtime!(image, layout, ruby_ver)

        expect(File.read(File.join(image, "lib/ruby/3.3.0/arm64-darwin24/rbconfig.rb"))).to eq("runtime")
        expect(Dir).not_to exist(File.join(image, "lib/ruby/3.3.0/arm64-darwin23"))
        expect(Dir).to exist(File.join(image, "lib/ruby/gems/3.3.0/extensions/arm64-darwin-24"))
        expect(Dir).not_to exist(File.join(image, "lib/ruby/gems/3.3.0/extensions/arm64-darwin-23"))
      end
    end

    it "is a no-op when the arch conventions already match" do
      Dir.mktmpdir do |tmp|
        image = File.join(tmp, "img")
        layout = File.join(tmp, "layout")
        FileUtils.mkdir_p(File.join(image, "lib/ruby/3.3.0/x86_64-linux"))
        File.write(File.join(image, "lib/ruby/3.3.0/x86_64-linux/rbconfig.rb"), "same")
        FileUtils.mkdir_p(File.join(layout, "lib/ruby/3.3.0/x86_64-linux"))
        File.write(File.join(layout, "lib/ruby/3.3.0/x86_64-linux/rbconfig.rb"), "same")

        Tebako::Packager.align_layout_to_runtime!(image, layout, ruby_ver)

        expect(File.read(File.join(image, "lib/ruby/3.3.0/x86_64-linux/rbconfig.rb"))).to eq("same")
      end
    end
  end

  describe "#mkdwarfs" do
    let(:deps_bin_dir) { "/path/to/deps/bin" }
    let(:data_bin_file) { "/path/to/output.dwarfs" }
    let(:data_src_dir) { "/path/to/src" }

    before do
      allow(Tebako::BuildHelpers).to receive(:run_with_capture_v)
    end

    it "runs mkdwarfs with the correct parameters" do
      params = [File.join(deps_bin_dir, "mkdwarfs"), "-o", data_bin_file, "-i", data_src_dir, "--no-progress"]
      expect(Tebako::BuildHelpers).to receive(:run_with_capture_v).with(params)
      described_class.mkdwarfs(deps_bin_dir, data_bin_file, data_src_dir)
    end
  end
end

# rubocop:enable Metrics/BlockLength
