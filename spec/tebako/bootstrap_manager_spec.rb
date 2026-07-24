# frozen_string_literal: true

# Copyright (c) 2026 [Ribose Inc](https://www.ribose.com).
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

require "digest"
require "fileutils"
require "json"
require "tmpdir"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::BootstrapManager do
  let(:bootstrap_version) { "0.2.0" }
  let(:platform) { "macos-arm64" }
  let(:filename) { "tebako-bootstrap-#{bootstrap_version}-#{platform}" }
  let(:package_body) { "fake tebako-bootstrap launcher\n" * 100 }
  let(:package_sha256) { Digest::SHA256.hexdigest(package_body) }

  let(:other_filename) { "tebako-bootstrap-#{bootstrap_version}-linux-gnu-x86_64" }
  let(:other_body) { "other fake tebako-bootstrap launcher\n" }
  let(:other_sha256) { Digest::SHA256.hexdigest(other_body) }

  let(:win_filename) { "tebako-bootstrap-#{bootstrap_version}-windows-x86_64.exe" }
  let(:win_body) { "MZ fake windows tebako-bootstrap\n" }
  let(:win_sha256) { Digest::SHA256.hexdigest(win_body) }

  let(:entry_dir) { File.join(@cache_root, "bootstraps", "tebako-bootstrap-#{bootstrap_version}-#{platform}") }
  let(:executable) { File.join(entry_dir, filename) }

  around do |example|
    Dir.mktmpdir("tebako-bootstrap-manager-spec") do |dir|
      @mirror = File.join(dir, "mirror")
      @cache_root = File.join(dir, "tebako-home")
      FileUtils.mkdir_p(release_dir)
      example.run
    end
  end

  def release_dir
    File.join(@mirror, "v#{bootstrap_version}")
  end

  def manager(mirror_url: "file://#{@mirror}", **kwargs)
    options = { cache_root: @cache_root, mirror: mirror_url, lock_timeout: 5, retry_delay: 0 }.merge(kwargs)
    Tebako::BootstrapManager.new(**options)
  end

  def write_packages
    File.binwrite(File.join(release_dir, filename), package_body)
    File.binwrite(File.join(release_dir, other_filename), other_body)
    File.binwrite(File.join(release_dir, win_filename), win_body)
  end

  def write_manifest(sha256: package_sha256)
    manifest = { "name" => "tebako-bootstrap", "version" => bootstrap_version,
                 "assets" => [
                   { "platform" => platform, "file" => filename, "sha256" => sha256 },
                   { "platform" => "linux-gnu-x86_64", "file" => other_filename, "sha256" => other_sha256 },
                   { "platform" => "windows-x86_64", "file" => win_filename, "sha256" => win_sha256 }
                 ] }
    File.write(File.join(release_dir, "manifest.json"), JSON.pretty_generate(manifest))
  end

  def write_sha256sums(sha256: package_sha256)
    lines = ["#{sha256}  #{filename}", "#{other_sha256}  #{other_filename}", "#{win_sha256}  #{win_filename}"]
    File.write(File.join(release_dir, "SHA256SUMS"), "#{lines.join("\n")}\n")
  end

  def with_env(vars)
    saved = vars.keys.to_h { |key| [key, ENV.fetch(key, nil)] }
    vars.each { |key, value| value.nil? ? ENV.delete(key) : ENV[key] = value }
    yield
  ensure
    saved.each { |key, value| value.nil? ? ENV.delete(key) : ENV[key] = value }
  end

  describe "#resolve" do
    context "with a manifest.json index" do
      before do
        write_packages
        write_manifest
      end

      it "downloads, verifies and installs the bootstrap into the cache" do
        path = manager.resolve(platform, bootstrap_version: bootstrap_version)

        expect(path).to eq(executable)
        expect(File.binread(path)).to eq(package_body)
        # File.executable? is extension-driven on Windows (only .exe/.bat/.com
        # report executable regardless of the mode bits), so assert mode there
        if Gem.win_platform?
          expect(File.exist?(path)).to be(true)
        else
          expect(File.executable?(path)).to be(true)
        end
        expect(File.read(File.join(entry_dir, "sha256"))).to include(package_sha256)
        expect(File.read(File.join(entry_dir, "origin"))).to include(filename)
      end

      it "returns the cached path without any download on the second resolve" do
        first = manager.resolve(platform, bootstrap_version: bootstrap_version)
        mtime = File.mtime(first)
        FileUtils.rm_rf(@mirror)

        second = manager.resolve(platform, bootstrap_version: bootstrap_version)

        expect(second).to eq(first)
        expect(File.mtime(second)).to eq(mtime)
      end

      it "prefers manifest.json over SHA256SUMS" do
        write_sha256sums(sha256: "0" * 64)
        expect(manager.resolve(platform, bootstrap_version: bootstrap_version)).to eq(executable)
      end

      it "resolves Windows .exe packages" do
        path = manager.resolve("windows-x86_64", bootstrap_version: bootstrap_version)

        expect(path).to eq(File.join(@cache_root, "bootstraps",
                                     "tebako-bootstrap-#{bootstrap_version}-windows-x86_64", win_filename))
        expect(File.binread(path)).to eq(win_body)
      end
    end

    context "with only a SHA256SUMS index (fallback)" do
      before do
        write_packages
        write_sha256sums
      end

      it "resolves and verifies the bootstrap via the checksums file" do
        path = manager.resolve(platform, bootstrap_version: bootstrap_version)

        expect(path).to eq(executable)
        expect(File.binread(path)).to eq(package_body)
      end
    end

    context "when the requested platform is not published" do
      before do
        write_packages
        write_manifest
      end

      it "fails listing the available platforms" do
        expect { manager.resolve("linux-musl-arm64", bootstrap_version: bootstrap_version) }
          .to raise_error(Tebako::Error) do |error|
            expect(error.error_code).to eq(131)
            expect(error.message).to include("linux-musl-arm64", "macos-arm64", "linux-gnu-x86_64", "windows-x86_64")
          end
      end
    end

    context "when the release carries neither manifest.json nor SHA256SUMS" do
      before { write_packages }

      it "fails naming the release and the tried URLs" do
        expect { manager.resolve(platform, bootstrap_version: bootstrap_version) }
          .to raise_error(Tebako::Error) do |error|
            expect(error.error_code).to eq(124)
            expect(error.message).to include("tebako-bootstrap", "v#{bootstrap_version}", "manifest.json", "SHA256SUMS")
          end
      end
    end

    context "when the download fails SHA256 verification" do
      before do
        write_packages
        write_manifest(sha256: "f" * 64)
      end

      it "deletes the download and never poisons the cache" do
        expect { manager.resolve(platform, bootstrap_version: bootstrap_version) }
          .to raise_error(Tebako::Error) { |error| expect(error.error_code).to eq(121) }

        expect(File.exist?(executable)).to be(false)
        expect(Dir.glob(File.join(@cache_root, "tmp", "*"))).to be_empty
      end
    end

    context "when TEBAKO_OFFLINE=1" do
      it "fails with a clear error naming the bootstrap reference and mirror knob" do
        with_env("TEBAKO_OFFLINE" => "1") do
          expect { manager.resolve(platform, bootstrap_version: bootstrap_version) }
            .to raise_error(Tebako::Error) do |error|
              expect(error.error_code).to eq(132)
              expect(error.message).to include("tebako-bootstrap@#{bootstrap_version}", platform,
                                               "TEBAKO_BOOTSTRAP_MIRROR")
            end
        end
      end

      it "serves cache hits without any network access" do
        write_packages
        write_manifest
        path = manager.resolve(platform, bootstrap_version: bootstrap_version)
        FileUtils.rm_rf(@mirror)

        with_env("TEBAKO_OFFLINE" => "1") do
          expect(manager.resolve(platform, bootstrap_version: bootstrap_version)).to eq(path)
        end
      end
    end

    context "with concurrent installers" do
      before do
        write_packages
        write_manifest
      end

      it "serializes installs so every resolver gets the same intact bootstrap" do
        resolver = manager
        results = Array.new(4) do
          Thread.new { resolver.resolve(platform, bootstrap_version: bootstrap_version) }
        end.map(&:value)

        expect(results.uniq).to eq([executable])
        expect(File.binread(executable)).to eq(package_body)
      end
    end
  end

  describe ".resolve" do
    it "resolves through TEBAKO_HOME and TEBAKO_BOOTSTRAP_MIRROR" do
      write_packages
      write_manifest
      with_env("TEBAKO_HOME" => @cache_root, "TEBAKO_BOOTSTRAP_MIRROR" => "file://#{@mirror}",
               "TEBAKO_BOOTSTRAP_VERSION" => bootstrap_version) do
        path = Tebako::BootstrapManager.resolve(platform)
        expect(path).to eq(executable)
        expect(File.binread(path)).to eq(package_body)
      end
    end
  end

  describe ".default_version" do
    it "defaults to the pinned bootstrap release" do
      with_env("TEBAKO_BOOTSTRAP_VERSION" => nil) do
        expect(Tebako::BootstrapManager.default_version).to eq(Tebako::BootstrapManager::BOOTSTRAP_VERSION)
      end
    end

    it "honors TEBAKO_BOOTSTRAP_VERSION with or without a v prefix" do
      with_env("TEBAKO_BOOTSTRAP_VERSION" => "v0.3.0") do
        expect(Tebako::BootstrapManager.default_version).to eq("0.3.0")
      end
      with_env("TEBAKO_BOOTSTRAP_VERSION" => "0.3.1") do
        expect(Tebako::BootstrapManager.default_version).to eq("0.3.1")
      end
    end
  end

  describe "#entries and #prune" do
    def install_fake_entry(name, age_days)
      dir = File.join(@cache_root, "bootstraps", name)
      FileUtils.mkdir_p(dir)
      File.binwrite(File.join(dir, "tebako-bootstrap-fake"), "x" * 1024)
      stamp = Time.now - (age_days * 86_400)
      File.utime(stamp, stamp, dir)
      dir
    end

    it "lists bootstrap entries separately from the runtime cache" do
      install_fake_entry("tebako-bootstrap-#{bootstrap_version}-#{platform}", 2)

      entries = manager.entries

      expect(entries.size).to eq(1)
      expect(entries.first[:name]).to eq("tebako-bootstrap-#{bootstrap_version}-#{platform}")
      expect(entries.first[:size_bytes]).to eq(1024)
    end

    it "prunes bootstrap entries only" do
      old_dir = install_fake_entry("tebako-bootstrap-0.1.0-macos-arm64", 40)
      new_dir = install_fake_entry("tebako-bootstrap-#{bootstrap_version}-#{platform}", 2)

      removed = manager.prune(older_than_days: 30)

      expect(removed).to eq(["tebako-bootstrap-0.1.0-macos-arm64"])
      expect(Dir.exist?(old_dir)).to be(false)
      expect(Dir.exist?(new_dir)).to be(true)
    end
  end
end

# rubocop:enable Metrics/BlockLength
