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

# Tebako - an executable packager
module Tebako
  # Resolution, download, verification and machine-wide caching of the
  # tebako-bootstrap launcher binaries published by tamatebako/tebako-bootstrap.
  # The launcher is the base binary of lean/fat three-part packages.
  #
  # Cache layout (rooted at $TEBAKO_HOME or ~/.tebako, shared with the
  # runtime package cache):
  #   bootstraps/tebako-bootstrap-<version>-<platform>/
  #     tebako-bootstrap-<version>-<platform>[.exe]
  #     sha256    -- digest the installed file was verified against
  #     origin    -- URL the binary was downloaded from
  #
  # All download/verify/lock machinery is inherited from RuntimeManager; only
  # the release-shape specifics (asset naming, manifest.json object format,
  # SHA256SUMS index name, mirror env var) differ.
  class BootstrapManager < RuntimeManager
    DEFAULT_MIRROR = "https://github.com/tamatebako/tebako-bootstrap/releases/download"
    BOOTSTRAPS_DIR = "bootstraps"
    INDEX_FILES = ["manifest.json", "SHA256SUMS"].freeze

    # tebako-bootstrap release consumed by default (TEBAKO_BOOTSTRAP_VERSION
    # overrides). 0.2.0 is the first release that installs a fat package's
    # runtime payload slot into the cache at first run.
    BOOTSTRAP_VERSION = "0.2.0"
    PAYLOAD_MIN_VERSION = "0.2.0"

    class << self
      def default_version
        version = ENV.fetch("TEBAKO_BOOTSTRAP_VERSION", nil)
        version.nil? || version.empty? ? BOOTSTRAP_VERSION : version.sub(/\Av/, "")
      end

      def resolve(platform, bootstrap_version: default_version)
        new.resolve(platform, bootstrap_version: bootstrap_version)
      end
    end

    def resolve(platform, bootstrap_version: self.class.default_version)
      super(bootstrap_version, platform, tebako_version: bootstrap_version)
    end

    private

    def default_mirror
      DEFAULT_MIRROR
    end

    def mirror_env_var
      "TEBAKO_BOOTSTRAP_MIRROR"
    end

    def index_files
      INDEX_FILES
    end

    def cache_subdir
      BOOTSTRAPS_DIR
    end

    def release_name
      "tebako-bootstrap"
    end

    def entry_dir(_version, platform, bootstrap_version)
      File.join(@cache_root, BOOTSTRAPS_DIR, "tebako-bootstrap-#{bootstrap_version}-#{platform}")
    end

    def runtime_filename(_version, platform, bootstrap_version)
      suffix = platform.start_with?("windows") ? ".exe" : ""
      "tebako-bootstrap-#{bootstrap_version}-#{platform}#{suffix}"
    end

    def runtime_ref(_version, platform, bootstrap_version)
      "tebako-bootstrap@#{bootstrap_version} (#{platform})"
    end

    def find_entry(index, _version, platform, bootstrap_version)
      entry = index.find { |candidate| candidate["platform"] == platform }
      return entry if entry

      platforms = index.map { |candidate| candidate["platform"] }
      Tebako.packaging_error(131, "no tebako-bootstrap package for #{platform} (bootstrap #{bootstrap_version}). " \
                                  "Available: #{platforms.join(", ")}.")
    end

    def offline_check(ref, bootstrap_version)
      return unless offline?

      Tebako.packaging_error(132, "#{ref} is not cached and downloads are disabled " \
                                  "(release index: #{index_urls(bootstrap_version).join(", ")}; " \
                                  "#{mirror_env_var}=#{@mirror})")
    end

    # tebako-bootstrap's manifest.json is an object
    # ({name:, version:, assets: [{platform:, file:, sha256:}]}), not the
    # entry array tebako-runtime-ruby publishes
    def parse_manifest(body, _bootstrap_version)
      manifest_assets(body)
        .select { |asset| asset["platform"] && asset["file"] && asset["sha256"] }
        .map { |asset| manifest_entry(asset) }
    end

    def manifest_assets(body)
      data = JSON.parse(body)
      assets = data.is_a?(Hash) ? data["assets"] : nil
      raise IndexUnavailable, "manifest.json has no assets array" unless assets.is_a?(Array)

      assets
    rescue JSON::ParserError
      raise IndexUnavailable, "manifest.json is not valid JSON"
    end

    def manifest_entry(asset)
      { "platform" => asset["platform"], "filename" => asset["file"], "sha256" => asset["sha256"].downcase }
    end

    def parse_sha256sums(body, bootstrap_version)
      pattern = /\Atebako-bootstrap-#{Regexp.escape(bootstrap_version)}-(.+?)(?:\.exe)?\z/
      body.each_line.filter_map do |line|
        sha256, file = line.strip.split(/\s+/, 2)
        match = file && pattern.match(file.sub(/\A\*/, ""))
        next unless match

        { "platform" => match[1], "filename" => file.sub(/\A\*/, ""), "sha256" => sha256.downcase }
      end
    end
  end
end
