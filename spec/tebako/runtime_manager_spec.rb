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
require "stringio"
require "tmpdir"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::RuntimeManager do
  let(:tebako_version) { "0.14.0" }
  let(:ruby_version) { "3.3.7" }
  let(:platform) { "macos-arm64" }
  let(:filename) { "tebako-runtime-#{tebako_version}-#{ruby_version}-#{platform}" }
  let(:package_body) { "fake tebako runtime binary\n" * 100 }
  let(:package_sha256) { Digest::SHA256.hexdigest(package_body) }

  let(:other_filename) { "tebako-runtime-#{tebako_version}-3.2.7-linux-gnu-x86_64" }
  let(:other_body) { "other fake tebako runtime binary\n" }
  let(:other_sha256) { Digest::SHA256.hexdigest(other_body) }

  let(:win_filename) { "tebako-runtime-#{tebako_version}-#{ruby_version}-windows-x86_64.exe" }
  let(:win_body) { "MZ fake windows tebako runtime\n" }
  let(:win_sha256) { Digest::SHA256.hexdigest(win_body) }

  let(:entry_name) { "ruby-#{ruby_version}-#{tebako_version}-#{platform}" }
  let(:entry_dir) { File.join(@cache_root, "runtimes", entry_name) }
  let(:executable) { File.join(entry_dir, filename) }

  around do |example|
    Dir.mktmpdir("tebako-runtime-manager-spec") do |dir|
      @mirror = File.join(dir, "mirror")
      @cache_root = File.join(dir, "tebako-home")
      FileUtils.mkdir_p(release_dir)
      example.run
    end
  end

  def release_dir
    File.join(@mirror, "v#{tebako_version}")
  end

  def manager(mirror_url: "file://#{@mirror}", **kwargs)
    options = { cache_root: @cache_root, mirror: mirror_url, lock_timeout: 5, retry_delay: 0 }.merge(kwargs)
    Tebako::RuntimeManager.new(**options)
  end

  def write_packages
    File.binwrite(File.join(release_dir, filename), package_body)
    File.binwrite(File.join(release_dir, other_filename), other_body)
    File.binwrite(File.join(release_dir, win_filename), win_body)
  end

  def manifest_entries(sha256: package_sha256)
    [
      { "tebako_version" => tebako_version, "ruby_version" => ruby_version, "platform" => platform,
        "filename" => filename, "sha256" => sha256, "size_bytes" => package_body.bytesize },
      { "tebako_version" => tebako_version, "ruby_version" => "3.2.7", "platform" => "linux-gnu-x86_64",
        "filename" => other_filename, "sha256" => other_sha256, "size_bytes" => other_body.bytesize },
      { "tebako_version" => tebako_version, "ruby_version" => ruby_version, "platform" => "windows-x86_64",
        "filename" => win_filename, "sha256" => win_sha256, "size_bytes" => win_body.bytesize }
    ]
  end

  def write_manifest(sha256: package_sha256)
    File.write(File.join(release_dir, "manifest.json"), JSON.pretty_generate(manifest_entries(sha256: sha256)))
  end

  def write_sha256sums(sha256: package_sha256)
    lines = ["#{sha256}  #{filename}", "#{other_sha256}  #{other_filename}", "#{win_sha256}  #{win_filename}"]
    File.write(File.join(release_dir, "SHA256SUMS.txt"), "#{lines.join("\n")}\n")
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

      it "downloads, verifies and installs the runtime into the cache" do
        path = manager.resolve(ruby_version, platform, tebako_version: tebako_version)

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
        first = manager.resolve(ruby_version, platform, tebako_version: tebako_version)
        mtime = File.mtime(first)
        FileUtils.rm_rf(@mirror)

        second = manager.resolve(ruby_version, platform, tebako_version: tebako_version)

        expect(second).to eq(first)
        expect(File.mtime(second)).to eq(mtime)
      end

      it "prefers manifest.json over SHA256SUMS.txt" do
        write_sha256sums(sha256: "0" * 64)
        expect(manager.resolve(ruby_version, platform, tebako_version: tebako_version)).to eq(executable)
      end
    end

    context "with only a SHA256SUMS.txt index (fallback)" do
      before do
        write_packages
        write_sha256sums
      end

      it "resolves and verifies the runtime via the checksums file" do
        path = manager.resolve(ruby_version, platform, tebako_version: tebako_version)

        expect(path).to eq(executable)
        expect(File.binread(path)).to eq(package_body)
      end

      it "resolves Windows .exe packages" do
        path = manager.resolve(ruby_version, "windows-x86_64", tebako_version: tebako_version)

        expect(path).to eq(File.join(@cache_root, "runtimes",
                                     "ruby-#{ruby_version}-#{tebako_version}-windows-x86_64", win_filename))
        expect(File.binread(path)).to eq(win_body)
      end
    end

    context "when the requested ruby/platform combination is not published" do
      before do
        write_packages
        write_manifest
      end

      it "fails listing available combinations and the --build-runtime hint" do
        expect { manager.resolve("9.9.9", platform, tebako_version: tebako_version) }
          .to raise_error(Tebako::Error) do |error|
            expect(error.error_code).to eq(120)
            expect(error.message).to include("3.3.7/macos-arm64", "3.2.7/linux-gnu-x86_64",
                                             "3.3.7/windows-x86_64", "--build-runtime")
          end
      end
    end

    context "when the release carries neither manifest.json nor SHA256SUMS.txt" do
      before { write_packages }

      it "fails naming the release and the tried URLs" do
        expect { manager.resolve(ruby_version, platform, tebako_version: tebako_version) }
          .to raise_error(Tebako::Error) do |error|
            expect(error.error_code).to eq(124)
            expect(error.message).to include("v#{tebako_version}", "manifest.json", "SHA256SUMS.txt")
          end
      end
    end

    context "when the download fails SHA256 verification" do
      before do
        write_packages
        write_manifest(sha256: "f" * 64)
      end

      it "deletes the download and never poisons the cache" do
        expect { manager.resolve(ruby_version, platform, tebako_version: tebako_version) }
          .to raise_error(Tebako::Error) { |error| expect(error.error_code).to eq(121) }

        expect(File.exist?(executable)).to be(false)
        tmp_glob = File.join(@cache_root, "tmp", "*")
        expect(Dir.glob(tmp_glob)).to be_empty
      end

      it "recovers on a later resolve once the index is fixed" do
        expect { manager.resolve(ruby_version, platform, tebako_version: tebako_version) }
          .to raise_error(Tebako::Error)

        write_manifest
        expect(manager.resolve(ruby_version, platform, tebako_version: tebako_version)).to eq(executable)
      end
    end

    context "when the network is unreachable" do
      it "retries and then fails with a clear download error" do
        unreachable = manager(mirror_url: "http://127.0.0.1:1")

        expect { unreachable.resolve(ruby_version, platform, tebako_version: tebako_version) }
          .to raise_error(Tebako::Error) do |error|
            expect(error.error_code).to eq(122)
            expect(error.message).to include("3 attempts")
          end
      end
    end

    context "when TEBAKO_OFFLINE=1" do
      it "fails with a clear error naming the runtime reference when not cached" do
        with_env("TEBAKO_OFFLINE" => "1") do
          expect { manager.resolve(ruby_version, platform, tebako_version: tebako_version) }
            .to raise_error(Tebako::Error) do |error|
              expect(error.error_code).to eq(123)
              expect(error.message).to include(ruby_version, platform, "TEBAKO_OFFLINE")
            end
        end
      end

      it "serves cache hits without any network access" do
        write_packages
        write_manifest
        path = manager.resolve(ruby_version, platform, tebako_version: tebako_version)
        FileUtils.rm_rf(@mirror)

        with_env("TEBAKO_OFFLINE" => "1") do
          expect(manager.resolve(ruby_version, platform, tebako_version: tebako_version)).to eq(path)
        end
      end
    end

    context "with concurrent installers" do
      before do
        write_packages
        write_manifest
      end

      it "serializes installs so every resolver gets the same intact runtime" do
        resolver = manager
        results = Array.new(4) do
          Thread.new { resolver.resolve(ruby_version, platform, tebako_version: tebako_version) }
        end.map(&:value)

        expect(results.uniq).to eq([executable])
        expect(File.binread(executable)).to eq(package_body)
      end

      it "times out with a clear error when the entry lock is held by another process" do
        FileUtils.mkdir_p(entry_dir)
        lock_path = File.join(entry_dir, ".install.lock")
        File.open(lock_path, File::RDWR | File::CREAT, 0o644) do |lock|
          lock.flock(File::LOCK_EX)
          expect { manager(lock_timeout: 0.3).resolve(ruby_version, platform, tebako_version: tebako_version) }
            .to raise_error(Tebako::Error) do |error|
              expect(error.error_code).to eq(125)
              expect(error.message).to match(/lock/i)
            end
        end
      end
    end
  end

  describe ".resolve" do
    it "resolves through TEBAKO_HOME and TEBAKO_RUNTIME_MIRROR" do
      write_packages
      write_manifest
      with_env("TEBAKO_HOME" => @cache_root, "TEBAKO_RUNTIME_MIRROR" => "file://#{@mirror}") do
        path = Tebako::RuntimeManager.resolve(ruby_version, platform, tebako_version: tebako_version)
        expect(path).to eq(executable)
        expect(File.binread(path)).to eq(package_body)
      end
    end
  end

  describe ".default_cache_root" do
    it "honors TEBAKO_HOME" do
      with_env("TEBAKO_HOME" => File.join(@cache_root, "custom")) do
        expect(Tebako::RuntimeManager.default_cache_root).to eq(File.join(@cache_root, "custom"))
      end
    end

    it "defaults to ~/.tebako" do
      with_env("TEBAKO_HOME" => nil) do
        expected = if Gem.win_platform?
                     File.join(ENV.fetch("LOCALAPPDATA", File.join(Dir.home, "AppData", "Local")), "tebako")
                   else
                     File.join(Dir.home, ".tebako")
                   end
        expect(Tebako::RuntimeManager.default_cache_root).to eq(expected)
      end
    end
  end

  describe ".layout" do
    it "extracts the runtime layout once, next to the cached package" do
      Dir.mktmpdir do |tmp|
        runtime_path = File.join(tmp, "tebako-runtime-x")
        layout_dir = File.join(tmp, "layout")

        expect(Tebako::BuildHelpers).to receive(:run_with_capture_v)
          .with([runtime_path, "--tebako-extract", layout_dir]) do
            FileUtils.mkdir_p(File.join(layout_dir, "lib"))
          end
        expect(Tebako::RuntimeManager.layout(runtime_path)).to eq(layout_dir)

        expect(Tebako::BuildHelpers).not_to receive(:run_with_capture_v)
        expect(Tebako::RuntimeManager.layout(runtime_path)).to eq(layout_dir)
      end
    end
  end

  describe "#entries and #prune" do
    def install_fake_entry(name, age_days)
      dir = File.join(@cache_root, "runtimes", name)
      FileUtils.mkdir_p(dir)
      File.binwrite(File.join(dir, "tebako-runtime-fake"), "x" * 1024)
      stamp = Time.now - (age_days * 86_400)
      File.utime(stamp, stamp, dir)
      dir
    end

    it "lists entries with sizes and ages" do
      install_fake_entry(entry_name, 2)

      entries = manager.entries

      expect(entries.size).to eq(1)
      expect(entries.first[:name]).to eq(entry_name)
      expect(entries.first[:size_bytes]).to eq(1024)
      expect(entries.first[:installed_at]).to be_within(5).of(Time.now - (2 * 86_400))
    end

    it "returns an empty list when the cache is empty" do
      expect(manager.entries).to eq([])
    end

    it "prunes only entries older than the requested age" do
      old_dir = install_fake_entry("ruby-3.1.6-0.13.4-linux-gnu-x86_64", 40)
      new_dir = install_fake_entry(entry_name, 2)

      removed = manager.prune(older_than_days: 30)

      expect(removed).to eq(["ruby-3.1.6-0.13.4-linux-gnu-x86_64"])
      expect(Dir.exist?(old_dir)).to be(false)
      expect(Dir.exist?(new_dir)).to be(true)
    end

    it "prunes everything with :all" do
      install_fake_entry("ruby-3.1.6-0.13.4-linux-gnu-x86_64", 40)
      install_fake_entry(entry_name, 2)

      removed = manager.prune(all: true)

      expect(removed.size).to eq(2)
      expect(Dir.children(File.join(@cache_root, "runtimes"))).to be_empty
    end

    it "refuses to prune without a selector" do
      expect { manager.prune }.to raise_error(ArgumentError)
    end
  end
end

RSpec.describe "tebako cache subcommands" do
  around do |example|
    Dir.mktmpdir("tebako-cache-cli-spec") do |dir|
      @cache_root = File.join(dir, "tebako-home")
      saved = ENV.fetch("TEBAKO_HOME", nil)
      ENV["TEBAKO_HOME"] = @cache_root
      example.run
      saved.nil? ? ENV.delete("TEBAKO_HOME") : ENV["TEBAKO_HOME"] = saved
    end
  end

  def install_fake_entry(name, age_days)
    dir = File.join(@cache_root, "runtimes", name)
    FileUtils.mkdir_p(dir)
    File.binwrite(File.join(dir, "tebako-runtime-fake"), "x" * (2 * 1024 * 1024))
    stamp = Time.now - (age_days * 86_400)
    File.utime(stamp, stamp, dir)
    dir
  end

  def run_cli(*args)
    stdout = $stdout
    $stdout = StringIO.new
    Tebako::Cli.start(args)
    $stdout.string
  ensure
    $stdout = stdout
  end

  it "cache list shows entries with sizes and ages" do
    install_fake_entry("ruby-3.3.7-0.14.0-macos-arm64", 3)

    output = run_cli("cache", "list")

    expect(output).to include("ruby-3.3.7-0.14.0-macos-arm64", "2.0 MB", "3d ago")
  end

  it "cache list reports an empty cache" do
    expect(run_cli("cache", "list")).to match(/[Ee]mpty/)
  end

  it "cache prune --all removes every entry" do
    install_fake_entry("ruby-3.3.7-0.14.0-macos-arm64", 3)
    install_fake_entry("ruby-3.1.6-0.13.4-linux-gnu-x86_64", 40)

    output = run_cli("cache", "prune", "--all")

    expect(output).to include("Removed ruby-3.3.7-0.14.0-macos-arm64")
    expect(Dir.children(File.join(@cache_root, "runtimes"))).to be_empty
  end

  it "cache prune --older-than removes only old entries" do
    install_fake_entry("ruby-3.3.7-0.14.0-macos-arm64", 3)
    install_fake_entry("ruby-3.1.6-0.13.4-linux-gnu-x86_64", 40)

    run_cli("cache", "prune", "--older-than", "30d")

    expect(Dir.children(File.join(@cache_root, "runtimes"))).to eq(["ruby-3.3.7-0.14.0-macos-arm64"])
  end

  it "cache prune without a selector removes nothing" do
    install_fake_entry("ruby-3.3.7-0.14.0-macos-arm64", 3)

    output = run_cli("cache", "prune")

    expect(output).to match(/--all|--older-than/)
    expect(Dir.children(File.join(@cache_root, "runtimes"))).to eq(["ruby-3.3.7-0.14.0-macos-arm64"])
  end
end

# rubocop:enable Metrics/BlockLength
