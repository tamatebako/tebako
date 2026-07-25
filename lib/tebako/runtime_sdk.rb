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
require "net/http"
require "open3"
require "rbconfig"
require "uri"

# Tebako - an executable packager
module Tebako
  # The native-build SDK of a prebuilt tebako runtime.
  #
  # Prebuilt runtime images are stripped for size: no bin/ruby and no ruby
  # headers, so mkmf-driven gem native extension builds cannot run against
  # them directly. The SDK closes the gap from the runtime's own provenance:
  # it fetches the pre-patched ruby source release the runtime was built
  # from (tamatebako/ruby, the same artifact tebako-runtime-ruby consumes)
  # and replays the configure arguments recorded in the runtime's rbconfig
  # (build-machine paths filtered out) to generate the matching header tree.
  # Provisioned once per (ruby version, src release, press platform) into
  # the packaging environment and reused afterwards.
  class RuntimeSdk # rubocop:disable Metrics/ClassLength
    DEFAULT_SRC_RELEASE = "v0.2.1"
    DEFAULT_MIRROR = "https://github.com/tamatebako/ruby/releases/download"
    SUMS_FILE = "SHA256SUMS"
    MARKER_FILE = ".sdk-complete"
    LOCK_FILE = ".sdk.lock"
    LOCK_TIMEOUT = 600

    class << self
      # SDK root for the runtime at +runtime_path+; provisions on first use
      def resolve(runtime_path, deps_dir, ruby_ver)
        new(runtime_path, deps_dir, ruby_ver).resolve
      end
    end

    def initialize(runtime_path, deps_dir, ruby_ver)
      @runtime_path = runtime_path
      @ruby_ver = ruby_ver
      @src_release = ENV.fetch("TEBAKO_SDK_SRC_RELEASE", nil) || DEFAULT_SRC_RELEASE
      @mirror = (ENV.fetch("TEBAKO_SDK_SRC_MIRROR", nil) || DEFAULT_MIRROR).sub(%r{/+\z}, "")
      @sdk_root = File.join(deps_dir, "sdk", "#{ruby_ver.ruby_version}-#{@src_release}-#{host_tag}")
    end

    attr_reader :sdk_root

    def resolve
      return sdk_root if complete?

      FileUtils.mkdir_p(sdk_root)
      File.open(File.join(sdk_root, LOCK_FILE), File::RDWR | File::CREAT, 0o644) do |lock|
        acquire_lock(lock)
        provision unless complete?
      end
      sdk_root
    end

    def include_dir
      File.join(sdk_root, "include")
    end

    def archhdr_dir
      File.join(sdk_root, "archhdr")
    end

    private

    def host_tag
      "#{RbConfig::CONFIG["host_os"]}-#{RbConfig::CONFIG["host_cpu"]}".gsub(/[^\w.-]/, "-")
    end

    def complete?
      File.file?(File.join(sdk_root, MARKER_FILE)) &&
        File.file?(File.join(include_dir, "ruby.h")) &&
        File.file?(File.join(archhdr_dir, "ruby", "config.h"))
    end

    def acquire_lock(lock)
      deadline = Time.now + LOCK_TIMEOUT
      until lock.flock(File::LOCK_EX | File::LOCK_NB)
        Tebako.packaging_error(125, "runtime SDK #{sdk_root}") if Time.now >= deadline

        sleep 0.1
      end
    end

    def provision # rubocop:disable Metrics/MethodLength
      puts "-- Provisioning the runtime SDK (ruby headers for native extension builds)"
      tmp = File.join(sdk_root, "tmp-#{Process.pid}")
      FileUtils.rm_rf(tmp)
      FileUtils.mkdir_p(tmp)
      tarball = download_source(tmp)
      configure(tmp, tarball)
      install_headers(tmp)
      generate_symbol_stub(tmp)
      File.write(File.join(sdk_root, MARKER_FILE),
                 "ruby #{@ruby_ver.ruby_version} #{@src_release} #{host_tag}\n")
      FileUtils.rm_rf(tmp)
    end

    def download_source(tmp)
      filename = "tfs-ruby-#{@ruby_ver.ruby_version}-src.tar.gz"
      sha256 = source_sha256(filename)
      Tebako.packaging_error(135, "#{filename} not found in #{@src_release} #{SUMS_FILE}") if sha256.nil?

      tarball = File.join(tmp, filename)
      body = fetch("#{release_url}/#{filename}")
      Tebako.packaging_error(135, "#{filename}: expected #{sha256}, got #{Digest::SHA256.hexdigest(body)}") unless
        Digest::SHA256.hexdigest(body) == sha256
      File.binwrite(tarball, body)
      puts "   ... #{filename} (SHA256 verified)"
      tarball
    end

    def source_sha256(filename)
      sums = fetch("#{release_url}/#{SUMS_FILE}")
      line = sums.each_line.find { |l| l.strip.end_with?(" #{filename}") || l.strip.end_with?(" *#{filename}") }
      sha = line&.split&.first
      sha&.downcase
    end

    def release_url
      "#{@mirror}/#{@src_release}"
    end

    def fetch(url, redirects_left = 5)
      Tebako.packaging_error(122, "too many redirects fetching #{url}") if redirects_left.zero?

      uri = URI.parse(url)
      return File.binread(uri.path) if uri.scheme == "file"

      response = http_get(uri)
      case response
      when Net::HTTPSuccess then response.body
      when Net::HTTPRedirection then fetch(URI.join(url, response["location"]).to_s, redirects_left - 1)
      else
        Tebako.packaging_error(122, "#{response.code} #{response.message} fetching #{url}")
      end
    end

    def http_get(uri)
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = uri.scheme == "https"
      http.open_timeout = 15
      http.read_timeout = 300
      http.start { |session| session.get(uri.request_uri.empty? ? "/" : uri.request_uri) }
    end

    def configure(tmp, tarball)
      out, st = Open3.capture2e("tar", "-xzf", tarball, "-C", tmp)
      Tebako.packaging_error(135, "failed to extract #{tarball}: #{out}") unless st.success?

      src_dir = Dir.glob(File.join(tmp, "tfs-ruby-*-src")).first
      args = filtered_configure_args + ["--prefix=#{File.join(tmp, "install")}"]
      out, st = Open3.capture2e("./configure", *args, chdir: src_dir)
      Tebako.packaging_error(135, "ruby configure failed:\n#{out.lines.last(10).join}") unless st.success?
    end

    # The runtime's own configure arguments, replayed from its rbconfig with
    # the build machine's paths and compiler assignments filtered out (the
    # press host supplies those); feature flags (--with/--without/--disable)
    # are kept verbatim so the generated config.h matches the runtime.
    # rbconfig normalizes '--with-out-ext' to '--without-ext', which only the
    # original spelling configures
    def filtered_configure_args
      configure_args.scan(/'([^']*)'/).flatten.filter_map do |arg|
        next if arg.start_with?("--prefix=") || arg.match?(/\A[A-Z_]+=/i)

        arg.sub(/\A--without-ext=/, "--with-out-ext=")
      end
    end

    def configure_args
      rbconfig = Dir.glob(File.join(layout_dir, "lib", "ruby", "*", "*", "rbconfig.rb")).first
      Tebako.packaging_error(135, "no rbconfig.rb found in #{layout_dir}") if rbconfig.nil?

      match = File.read(rbconfig).match(/CONFIG\["configure_args"\] = "(.*)"$/)
      Tebako.packaging_error(135, "no configure_args recorded in #{rbconfig}") if match.nil?

      match[1].gsub("\\\"", "\"").gsub("\\'", "'")
    end

    def layout_dir
      @layout_dir ||= Tebako::RuntimeManager.layout(@runtime_path)
    end

    def install_headers(tmp) # rubocop:disable Metrics/AbcSize
      src_dir = Dir.glob(File.join(tmp, "tfs-ruby-*-src")).first
      FileUtils.rm_rf(include_dir)
      FileUtils.rm_rf(archhdr_dir)
      FileUtils.cp_r(File.join(src_dir, "include"), include_dir)

      config_h = Dir.glob(File.join(src_dir, ".ext", "include", "*", "ruby", "config.h")).first
      Tebako.packaging_error(135, "configure produced no ruby/config.h") if config_h.nil?

      FileUtils.mkdir_p(File.join(archhdr_dir, "ruby"))
      FileUtils.cp(config_h, File.join(archhdr_dir, "ruby", "config.h"))
    end

    # The runtime ships no libruby archive, so mkmf's link probes have
    # nothing true to resolve against (linking with undefined-symbol lookup
    # makes every probe succeed, which mis-detects features). The stub is an
    # archive re-declaring every symbol the runtime executable exports --
    # the exact, provenance-true symbol table. Only throwaway probe binaries
    # link it; shipped extensions stay on dynamic lookup against the
    # executable (which exports these symbols).
    def generate_symbol_stub(tmp) # rubocop:disable Metrics/AbcSize, Metrics/MethodLength
      out, st = Open3.capture2e(*nm_command)
      Tebako.packaging_error(135, "nm failed on #{@runtime_path}: #{out}") unless st.success?

      symbols = out.scan(/^\h+ [A-Za-z] (\S+)$/).flatten.uniq.reject { |s| s.start_with?("__mh_") }
      Tebako.packaging_error(135, "no exported symbols found in #{@runtime_path}") if symbols.empty?

      asm = File.join(tmp, "symbols.s")
      File.write(asm, ".text\n#{symbols.map { |s| ".globl #{s}\n#{s}: ret\n" }.join}")
      stub_dir = File.join(sdk_root, "lib")
      FileUtils.mkdir_p(stub_dir)
      object = File.join(tmp, "symbols.o")
      out, st = Open3.capture2e("cc", "-c", asm, "-o", object)
      Tebako.packaging_error(135, "stub compile failed: #{out}") unless st.success?

      out, st = Open3.capture2e("ar", "rcs", File.join(stub_dir, "libruby-stub.a"), object)
      Tebako.packaging_error(135, "ar failed: #{out}") unless st.success?
    end

    def nm_command
      RUBY_PLATFORM.match?(/darwin/) ? ["nm", "-gU", @runtime_path] : ["nm", "-g", "--defined-only", @runtime_path]
    end
  end
end
