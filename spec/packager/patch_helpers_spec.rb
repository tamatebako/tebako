# frozen_string_literal: true

# Copyright (c) 2025 [Ribose Inc](https://www.ribose.com).
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
require_relative "../../lib/tebako/packager/patch_helpers"

RSpec.describe Tebako::Packager::PatchHelpers do # rubocop:disable Metrics/BlockLength
  let(:temp_dir) { File.join(Dir.tmpdir, "tebako_patch_helpers_test") }

  before do
    FileUtils.mkdir_p(temp_dir)
  end

  after do
    FileUtils.rm_rf(temp_dir)
  end

  describe "#patch_file" do
    let(:temp_dir) { Dir.mktmpdir }
    let(:temp_file) { File.join(temp_dir, "sample.txt") }

    after do
      FileUtils.remove_entry_secure(temp_dir)
    end

    context "when the file does not exist" do
      it "raises an error" do
        expect do
          Tebako::Packager::PatchHelpers.patch_file(File.join(temp_dir, "missing.txt"), { /old/ => "new" })
        end.to raise_error(Tebako::Error, /Could not patch/)
      end
    end

    context "when the file exists" do
      before do
        File.write(temp_file, "old content")
      end

      it "patches the file content" do
        expect do
          Tebako::Packager::PatchHelpers.patch_file(temp_file, { /old/ => "new" })
        end.not_to raise_error
        expect(File.read(temp_file)).to eq("new content")
      end
    end
  end

  describe "#get_prefix_macos" do
    it "returns brew prefix when brew command succeeds" do
      allow(Open3).to receive(:capture2).with("brew --prefix mypackage")
                                        .and_return(["/usr/local/Cellar/mypackage", double(exitstatus: 0)])

      expect(Tebako::Packager::PatchHelpers.get_prefix_macos("mypackage")).to eq("/usr/local/Cellar/mypackage")
    end

    it "raises an error if brew command fails" do
      allow(Open3).to receive(:capture2).with("brew --prefix mypackage")
                                        .and_return(["", double(exitstatus: 1)])

      expect do
        Tebako::Packager::PatchHelpers.get_prefix_macos("mypackage")
      end.to raise_error(Tebako::Error, /brew --prefix mypackage failed/)
    end
  end

  describe "#get_prefix_linux" do
    it "returns pkg-config libdir when pkg-config command succeeds" do
      allow(Open3).to receive(:capture2).with("pkg-config --variable=libdir mypackage")
                                        .and_return(["/usr/lib/mypackage", double(exitstatus: 0)])

      expect(Tebako::Packager::PatchHelpers.get_prefix_linux("mypackage")).to eq("/usr/lib/mypackage")
    end

    it "raises an error if pkg-config command fails" do
      allow(Open3).to receive(:capture2).with("pkg-config --variable=libdir mypackage")
                                        .and_return(["", double(exitstatus: 1)])

      expect do
        Tebako::Packager::PatchHelpers.get_prefix_linux("mypackage")
      end.to raise_error(Tebako::Error, /pkg-config --variable=libdir mypackage failed/)
    end
  end

  describe "#restore_and_save_files" do # rubocop:disable Metrics/BlockLength
    let(:ruby_source_dir) { File.join(temp_dir, "ruby_source") }
    let(:test_files) { ["test1.rb", "test2.rb"] }

    before do
      FileUtils.mkdir_p(ruby_source_dir)
      test_files.each do |f|
        File.write(File.join(ruby_source_dir, f), "original content")
        File.write(File.join(ruby_source_dir, "#{f}.old"), "backup content")
      end
    end

    it "restores files from backups and saves existing ones" do
      expect do
        Tebako::Packager::PatchHelpers.restore_and_save_files(test_files, ruby_source_dir)
      end.not_to raise_error

      test_files.each do |f|
        target = File.join(ruby_source_dir, f)
        old_file = File.join(ruby_source_dir, "#{f}.old")
        expect(File.read(target)).to eq("backup content")
        expect(File.read(old_file)).to eq("backup content")
      end
    end

    it "raises an error in strict mode if afile is missing" do
      FileUtils.rm(File.join(ruby_source_dir, "test1.rb"))
      expect do
        Tebako::Packager::PatchHelpers.restore_and_save_files(test_files, ruby_source_dir, strict: true)
      end.to raise_error(Tebako::Error)
    end

    it "does not raise an error in non-strict mode if a file is missing" do
      FileUtils.rm(File.join(ruby_source_dir, "test1.rb"))
      expect do
        Tebako::Packager::PatchHelpers.restore_and_save_files(test_files, ruby_source_dir, strict: false)
      end.not_to raise_error
    end
  end

  describe "#yaml_reference" do
    let(:ruby_version_double) { double("RubyVersion") }

    context "when Ruby 3.2" do
      it "returns '-l:libyaml.a'" do
        allow(ruby_version_double).to receive(:ruby32?).and_return(true)
        expect(Tebako::Packager::PatchHelpers.yaml_reference(ruby_version_double)).to eq("-l:libyaml.a")
      end
    end

    context "otherwise" do
      it "returns an empty string" do
        allow(ruby_version_double).to receive(:ruby32?).and_return(false)
        expect(Tebako::Packager::PatchHelpers.yaml_reference(ruby_version_double)).to eq("")
      end
    end
  end
end
