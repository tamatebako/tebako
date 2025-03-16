# frozen_string_literal: true

# Copyright (c) 2024 [Ribose Inc](https://www.ribose.com).
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

require "yaml"
require "tebako/cache_manager"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::CacheManager do
  let(:deps) { "/path/to/deps" }
  let(:output_folder) { "/path/to/output" }
  let(:source) { "/path/to/source" }
  let(:cache_manager) { Tebako::CacheManager.new(deps, source, output_folder) }

  describe "#ensure_version_file" do
    let(:vf) { Tebako::CacheManager::E_VERSION_FILE }
    it "writes version file" do
      expect(File).to receive(:write).with(File.join(deps, vf), cache_manager.version_key)
      cache_manager.ensure_version_file
    end
    it "prints error message if fails to create a file " do
      err = "Warning. Could not create cache version file #{vf}: No such file or directory @ rb_sysopen"
      expect(cache_manager).to receive(:puts).with("#{err} - #{deps}/#{vf}")
      cache_manager.ensure_version_file
    end
  end

  describe "#clean_cache" do
    it "cleans the cache by removing the appropriate directories" do
      expect(FileUtils).to receive(:rm_rf).with([File.join(deps, ""), File.join(output_folder, "")], secure: true)
      cache_manager.clean_cache
    end
  end

  describe "#clean_output" do
    it "cleans the output by removing the appropriate files and directories" do
      nmr = "src/_ruby_*"
      nms = "stash_*"
      expect(FileUtils).to receive(:rm_rf).with(Dir.glob(File.join(deps, nmr)), secure: true)
      expect(FileUtils).to receive(:rm_rf).with(Dir.glob(File.join(deps, nms)), secure: true)
      expect(FileUtils).to receive(:rm_rf).with(File.join(output_folder, ""), secure: true)
      cache_manager.clean_output
    end
  end

  describe "#version_cache" do
    it "parses version and source correctly from file" do
      fname = File.join(deps, Tebako::CacheManager::E_VERSION_FILE)
      allow(File).to receive(:open).with(fname).and_yield(StringIO.new("1.0.0 at source_path"))
      result = cache_manager.version_cache
      expect(result[:version]).to eq("1.0.0")
      expect(result[:source]).to eq("source_path")
    end
  end

  describe "#version_cache_check" do
    context "when version cache is unknown" do
      let(:match_data) { nil }

      it "calls version_unknown" do
        allow(cache_manager).to receive(:version_cache).and_return(:match_data)
        expect(cache_manager).to receive(:version_unknown)
        cache_manager.version_cache_check
      end
    end

    context "when version does not match" do
      let(:match_data) { { version: "0.7.0", source: "/path/to/source" } }
      it "calls version_mismatch" do
        allow(cache_manager).to receive(:version_cache).and_return(match_data)
        expect(cache_manager).to receive(:version_mismatch).with("0.7.0")
        cache_manager.version_cache_check
      end
    end

    context "when source does not match" do
      let(:match_data) { { version: Tebako::VERSION, source: "/different/source" } }

      it "calls version_source_mismatch" do
        allow(cache_manager).to receive(:version_cache).and_return(match_data)
        expect(cache_manager).to receive(:version_source_mismatch).with("/different/source")
        cache_manager.version_cache_check
      end
    end

    context "when version and source match" do
      let(:match_data) { { version: Tebako::VERSION, source: "/path/to/source" } }

      it "does not call any mismatch methods" do
        allow(cache_manager).to receive(:version_cache).and_return(match_data)
        expect(self).not_to receive(:version_unknown)
        expect(self).not_to receive(:version_mismatch)
        expect(self).not_to receive(:version_source_mismatch)
        cache_manager.version_cache_check
      end
    end
  end

  describe "#version_key" do
    it "returns the correct version key" do
      expect(cache_manager.version_key).to eq("#{Tebako::VERSION} at /path/to/source")
    end
  end

  describe "#version_mismatch" do
    it "prints the correct message and cleans the cache" do
      expect(cache_manager).to receive(:puts)
        .with("Tebako cache was created by a gem version 0.9.0 and cannot be used for gem version #{Tebako::VERSION}")
      expect(cache_manager).to receive(:clean_cache)
      cache_manager.version_mismatch("0.9.0")
    end
  end

  describe "#version_source_mismatch" do
    it "prints the correct message and cleans the output" do
      expect(cache_manager).to receive(:puts)
        .with("CMake cache was created for a different source directory '/different/source' " \
              "and cannot be used for '/path/to/source'")
      expect(cache_manager).to receive(:clean_output)
      allow(cache_manager).to receive(:source).and_return("/path/to/source")
      cache_manager.version_source_mismatch("/different/source")
    end
  end
  describe "#version_unknown" do
    it "prints the correct message and cleans the cache" do
      expect(cache_manager).to receive(:puts).with("CMake cache version was not recognized, cleaning up")
      expect(cache_manager).to receive(:clean_cache)
      cache_manager.version_unknown
    end
  end
end

# rubocop:enable Metrics/BlockLength
