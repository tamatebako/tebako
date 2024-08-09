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

require_relative "../lib/tebako/deploy_helper"

RSpec.describe Tebako::DeployHelper do # rubocop:disable Metrics/BlockLength
  let(:fs_root) { "/fs/root" }
  let(:fs_entrance) { "/fs/entrance" }
  let(:fs_mount_point) { "/fs/mount_point" }
  let(:target_dir) { "/target/dir" }
  let(:pre_dir) { "/pre/dir" }
  let(:deploy_helper) { Tebako::DeployHelper.new(fs_root, fs_entrance, fs_mount_point, target_dir, pre_dir) }

  describe "#initialize" do
    it "sets instance variables correctly" do
      expect(deploy_helper.instance_variable_get(:@fs_root)).to eq(fs_root)
      expect(deploy_helper.instance_variable_get(:@fs_entrance)).to eq(fs_entrance)
      expect(deploy_helper.instance_variable_get(:@fs_mount_point)).to eq(fs_mount_point)
      expect(deploy_helper.instance_variable_get(:@target_dir)).to eq(target_dir)
      expect(deploy_helper.instance_variable_get(:@pre_dir)).to eq(pre_dir)
      expect(deploy_helper.instance_variable_get(:@verbose)).to eq(false)
      expect(deploy_helper.instance_variable_get(:@ncores)).to be_a(Integer)
    end
  end

  describe "#config" do
    let(:os_type) { "linux" }
    let(:r_v) { "3.2.4" }
    let(:ruby_ver) { Tebako::RubyVersion.new(r_v) }
    let(:cwd) { "/current/working/dir" }

    before do
      allow(deploy_helper).to receive(:lookup_files)
      allow(deploy_helper).to receive(:configure_scenario)
      allow(deploy_helper).to receive(:configure_commands)
      deploy_helper.config(os_type, ruby_ver, cwd)
    end

    it "sets configuration variables correctly" do
      r_vv = deploy_helper.instance_variable_get(:@ruby_ver)
      expect(r_vv.instance_variable_get(:@ruby_ver)).to eq(r_v)
      expect(deploy_helper.instance_variable_get(:@os_type)).to eq(os_type)
      expect(deploy_helper.instance_variable_get(:@cwd)).to eq(cwd)
      expect(deploy_helper.instance_variable_get(:@tbd)).to eq(File.join(target_dir, "bin"))
      expect(deploy_helper.instance_variable_get(:@tgd)).to eq(File.join(target_dir, "lib", "ruby", "gems",
                                                                         r_vv.api_version))
      expect(deploy_helper.instance_variable_get(:@tld)).to eq(File.join(target_dir, "local"))
    end

    it "calls lookup_files, configure_scenario, and configure_commands" do
      expect(deploy_helper).to have_received(:lookup_files)
      expect(deploy_helper).to have_received(:configure_scenario)
      expect(deploy_helper).to have_received(:configure_commands)
    end
  end
  describe "#check_cwd" do
    context "when tebako cwd setting is nil" do
      it "returns (check succeeds)" do
        deploy_helper.instance_variable_set(:@cwd, nil)
        expect { deploy_helper.send(:check_cwd) }.not_to raise_error
      end
    end

    context "when tebako cwd setting is not an existing folder" do
      it "raises TebakoException with code 108" do
        deploy_helper.instance_variable_set(:@cwd, "/non/existing/folder")
        expect { deploy_helper.send(:check_cwd) }.to raise_error(Tebako::Error) { |e|
          expect(e.error_code).to eq(108)
        }
      end
    end
    context "when tebako cwd setting is an existing folder" do
      it "returns without exception" do
        deploy_helper.instance_variable_set(:@cwd, "/existing/folder")
        allow(File).to receive(:directory?).with(a_string_ending_with("existing/folder")).and_return(true)
        expect { deploy_helper.send(:check_cwd) }.not_to raise_error
      end
    end
  end
end
