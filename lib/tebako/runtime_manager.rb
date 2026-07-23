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
require "net/http"
require "uri"

require_relative "build_helpers"
require_relative "error"
require_relative "version"

# Tebako - an executable packager
module Tebako
  # Resolution, download, verification and machine-wide caching of prebuilt
  # tebako runtime packages published by tamatebako/tebako-runtime-ruby.
  #
  # Cache layout (rooted at $TEBAKO_HOME or ~/.tebako):
  #   runtimes/ruby-<ruby-version>-<tebakoabi>-<platform>/
  #     tebako-runtime-<tebakoabi>-<ruby-version>-<platform>[.exe]
  #     sha256    -- digest the installed file was verified against
  #     origin    -- URL the package was downloaded from
  #
  # Installs are serialized per entry with a flock'd lockfile; packages are
  # downloaded to tmp/, SHA256-verified against the release index and moved
  # into place with an atomic rename, so partial downloads never poison the
  # cache.
  class RuntimeManager # rubocop:disable Metrics/ClassLength
    DEFAULT_MIRROR = "https://github.com/tamatebako/tebako-runtime-ruby/releases/download"
    RUNTIMES_DIR = "runtimes"
    TMP_DIR = "tmp"
    LOCK_FILE = ".install.lock"
    SHA256_FILE = "sha256"
    ORIGIN_FILE = "origin"
    RUNTIME_TYPE = "ruby"
    LOCK_TIMEOUT = 300
    DOWNLOAD_ATTEMPTS = 3
    RETRY_DELAY = 1.0
    REDIRECT_LIMIT = 5
    INDEX_FILES = ["manifest.json", "SHA256SUMS.txt"].freeze

    # The requested index asset is missing or unusable; try the next one
    class IndexUnavailable < StandardError; end
    # A download failed (after redirects/retries, at the HTTP layer)
    class DownloadFailed < StandardError; end

    class << self
      def default_cache_root
        tebako_home = ENV.fetch("TEBAKO_HOME", nil)
        return tebako_home unless tebako_home.nil? || tebako_home.empty?

        if Gem.win_platform?
          File.join(ENV.fetch("LOCALAPPDATA", File.join(Dir.home, "AppData", "Local")), "tebako")
        else
          File.join(Dir.home, ".tebako")
        end
      end

      def resolve(ruby_version, platform, tebako_version: Tebako::VERSION)
        new.resolve(ruby_version, platform, tebako_version: tebako_version)
      end

      def layout(runtime_path)
        new.layout(runtime_path)
      end
    end

    def initialize(cache_root: nil, mirror: nil, lock_timeout: LOCK_TIMEOUT, retry_delay: RETRY_DELAY)
      @cache_root = cache_root || self.class.default_cache_root
      @mirror = (mirror || ENV.fetch(mirror_env_var, nil) || default_mirror).sub(%r{/+\z}, "")
      @lock_timeout = lock_timeout
      @retry_delay = retry_delay
    end

    attr_reader :cache_root

    def resolve(ruby_version, platform, tebako_version: Tebako::VERSION)
      dir = entry_dir(ruby_version, platform, tebako_version)
      executable = File.join(dir, runtime_filename(ruby_version, platform, tebako_version))
      return executable if File.file?(executable)

      with_entry_lock(dir, runtime_ref(ruby_version, platform, tebako_version)) do
        install(executable, ruby_version, platform, tebako_version) unless File.file?(executable)
      end
      executable
    end

    # Extract the runtime package's filesystem layout next to the cached
    # package (idempotent) and return the layout root. The layout is the
    # authoritative source of the runtime's arch conventions (rbconfig
    # location, gem extension dir naming), which the application image must
    # match: ruby's compiled-in search paths come from the runtime build,
    # while a local packaging environment names them after the press machine
    # (macOS embeds the kernel version, e.g. arm64-darwin23 vs -darwin24).
    def layout(runtime_path)
      layout_dir = File.join(File.dirname(runtime_path), "layout")
      return layout_dir if File.directory?(File.join(layout_dir, "lib"))

      FileUtils.mkdir_p(layout_dir)
      Tebako::BuildHelpers.run_with_capture_v([runtime_path, "--tebako-extract", layout_dir])
      layout_dir
    end

    def entries
      base = File.join(@cache_root, cache_subdir)
      return [] unless Dir.exist?(base)

      Dir.children(base).sort.filter_map do |name|
        path = File.join(base, name)
        next unless File.directory?(path)

        { name: name, path: path, size_bytes: dir_size(path), installed_at: File.mtime(path) }
      end
    end

    def prune(all: false, older_than_days: nil)
      raise ArgumentError, "prune requires :all or :older_than_days" unless all || older_than_days

      cutoff = older_than_days && (Time.now - (older_than_days * 86_400))
      entries.each_with_object([]) do |entry, removed|
        next unless all || entry[:installed_at] < cutoff

        FileUtils.rm_rf(entry[:path], secure: true)
        removed << entry[:name]
      end
    end

    private

    # ---- extension points for BootstrapManager (tebako-bootstrap release
    # resolution); overridden there, everything else is shared machinery ----

    def default_mirror
      DEFAULT_MIRROR
    end

    def mirror_env_var
      "TEBAKO_RUNTIME_MIRROR"
    end

    def index_files
      INDEX_FILES
    end

    def cache_subdir
      RUNTIMES_DIR
    end

    def release_name
      "tebako-runtime-ruby"
    end

    def install(executable, ruby_version, platform, tebako_version)
      ref = runtime_ref(ruby_version, platform, tebako_version)
      offline_check(ref, tebako_version)
      entry = find_entry(fetch_index(tebako_version), ruby_version, platform, tebako_version)
      url = package_url(entry["filename"], tebako_version)
      tmp = download(url, entry["filename"])
      verify!(tmp, entry)
      place(tmp, executable, entry, url)
    end

    def offline_check(ref, tebako_version)
      return unless offline?

      Tebako.packaging_error(123, "#{ref} is not cached and downloads are disabled " \
                                  "(release index: #{index_urls(tebako_version).join(", ")}; " \
                                  "#{mirror_env_var}=#{@mirror})")
    end

    def find_entry(index, ruby_version, platform, tebako_version)
      entry = index.find { |candidate| candidate["ruby_version"] == ruby_version && candidate["platform"] == platform }
      return entry if entry

      combos = index.map { |candidate| "#{candidate["ruby_version"]}/#{candidate["platform"]}" }
      Tebako.packaging_error(120, "no package for ruby #{ruby_version} on #{platform} " \
                                  "(tebako #{tebako_version}). Available: #{combos.join(", ")}. " \
                                  "Use --build-runtime to build the runtime from source instead.")
    end

    def verify!(tmp, entry)
      actual = Digest::SHA256.file(tmp).hexdigest
      expected = entry["sha256"].to_s.downcase
      return if actual == expected

      FileUtils.rm_f(tmp)
      Tebako.packaging_error(121, "#{entry["filename"]}: expected #{expected}, got #{actual}; download deleted")
    end

    def place(tmp, executable, entry, url)
      FileUtils.chmod(0o755, tmp)
      File.rename(tmp, executable)
      dir = File.dirname(executable)
      File.write(File.join(dir, SHA256_FILE), "#{entry["sha256"]}\n")
      File.write(File.join(dir, ORIGIN_FILE), "#{url}\n")
    end

    def fetch_index(tebako_version)
      tried = []
      index_files.each do |name|
        return parse_index(name, fetch_text(index_url(name, tebako_version)), tebako_version)
      rescue IndexUnavailable
        tried << index_url(name, tebako_version)
      rescue DownloadFailed => e
        Tebako.packaging_error(122, e.message)
      end
      Tebako.packaging_error(124, "#{release_name} release v#{tebako_version} provides no usable package " \
                                  "index (tried: #{tried.join(", ")})")
    end

    def parse_index(name, body, tebako_version)
      name == "manifest.json" ? parse_manifest(body, tebako_version) : parse_sha256sums(body, tebako_version)
    end

    def parse_manifest(body, tebako_version)
      data = JSON.parse(body)
      raise IndexUnavailable, "manifest.json is not an array" unless data.is_a?(Array)

      data.select { |entry| entry["tebako_version"] == tebako_version && entry["sha256"] && entry["filename"] }
    rescue JSON::ParserError
      raise IndexUnavailable, "manifest.json is not valid JSON"
    end

    def parse_sha256sums(body, tebako_version)
      pattern = /\Atebako-runtime-#{Regexp.escape(tebako_version)}-(\d+\.\d+\.\d+)-(.+?)(?:\.exe)?\z/
      body.each_line.filter_map do |line|
        sha256, file = line.strip.split(/\s+/, 2)
        match = file && pattern.match(file.sub(/\A\*/, ""))
        next unless match

        { "tebako_version" => tebako_version, "ruby_version" => match[1], "platform" => match[2],
          "filename" => file.sub(/\A\*/, ""), "sha256" => sha256.downcase, "size_bytes" => nil }
      end
    end

    def download(url, filename)
      FileUtils.mkdir_p(File.join(@cache_root, TMP_DIR))
      tmp = File.join(@cache_root, TMP_DIR, "#{filename}.#{Process.pid}.part")
      File.binwrite(tmp, with_retries(url) { read_url(url) })
      tmp
    rescue IndexUnavailable
      FileUtils.rm_f(tmp)
      Tebako.packaging_error(122, "#{url}: not found")
    rescue DownloadFailed => e
      FileUtils.rm_f(tmp)
      Tebako.packaging_error(122, e.message)
    end

    def fetch_text(url)
      with_retries(url) { read_url(url) }
    end

    def with_retries(url) # rubocop:disable Metrics/MethodLength
      attempts = 0
      begin
        attempts += 1
        yield
      rescue IndexUnavailable
        raise
      rescue StandardError => e
        raise DownloadFailed, retry_message(url, e) if attempts >= DOWNLOAD_ATTEMPTS

        sleep @retry_delay
        retry
      end
    end

    def retry_message(url, error)
      "failed to download #{url} after #{DOWNLOAD_ATTEMPTS} attempts: #{error.message}"
    end

    def read_url(url, redirects_left = REDIRECT_LIMIT)
      uri = URI.parse(url)
      return read_file_url(uri) if uri.scheme == "file"
      raise DownloadFailed, "too many redirects fetching #{url}" if redirects_left.zero?

      read_http_url(url, uri, redirects_left)
    end

    def read_file_url(uri)
      File.binread(uri.path)
    rescue Errno::ENOENT
      raise IndexUnavailable, uri.path
    end

    def read_http_url(url, uri, redirects_left)
      response = http_get(uri)
      case response
      when Net::HTTPSuccess then response.body
      when Net::HTTPRedirection
        read_url(URI.join(url, response["location"]).to_s, redirects_left - 1)
      when Net::HTTPNotFound then raise IndexUnavailable, url
      else raise DownloadFailed, "#{response.code} #{response.message} fetching #{url}"
      end
    end

    def http_get(uri)
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = uri.scheme == "https"
      http.open_timeout = 15
      http.read_timeout = 300
      http.start { |session| session.get(uri.request_uri.empty? ? "/" : uri.request_uri) }
    end

    def with_entry_lock(dir, ref)
      FileUtils.mkdir_p(dir)
      File.open(File.join(dir, LOCK_FILE), File::RDWR | File::CREAT, 0o644) do |lock|
        acquire_lock(lock, ref)
        begin
          yield
        ensure
          lock.flock(File::LOCK_UN)
        end
      end
    end

    def acquire_lock(lock, ref)
      deadline = Time.now + @lock_timeout
      until lock.flock(File::LOCK_EX | File::LOCK_NB)
        if Time.now >= deadline
          Tebako.packaging_error(125, "#{ref}: another process is installing this runtime " \
                                      "(no lock after #{@lock_timeout}s; lockfile: #{lock.path})")
        end
        sleep 0.1
      end
    end

    def offline?
      %w[1 true yes].include?(ENV.fetch("TEBAKO_OFFLINE", "").downcase)
    end

    def entry_dir(ruby_version, platform, tebako_version)
      File.join(@cache_root, RUNTIMES_DIR,
                "#{RUNTIME_TYPE}-#{ruby_version}-#{tebako_version}-#{platform}")
    end

    def runtime_filename(ruby_version, platform, tebako_version)
      suffix = platform.start_with?("windows") ? ".exe" : ""
      "tebako-runtime-#{tebako_version}-#{ruby_version}-#{platform}#{suffix}"
    end

    def runtime_ref(ruby_version, platform, tebako_version)
      "ruby@#{ruby_version} (tebako #{tebako_version}, #{platform})"
    end

    def release_url(tebako_version)
      "#{@mirror}/v#{tebako_version.to_s.sub(/\Av/, "")}"
    end

    def index_url(name, tebako_version)
      "#{release_url(tebako_version)}/#{name}"
    end

    def index_urls(tebako_version)
      index_files.map { |name| index_url(name, tebako_version) }
    end

    def package_url(filename, tebako_version)
      "#{release_url(tebako_version)}/#{filename}"
    end

    def dir_size(path)
      Dir.glob(File.join(path, "**", "*")).select { |file| File.file?(file) }.sum { |file| File.size(file) }
    end
  end
end
