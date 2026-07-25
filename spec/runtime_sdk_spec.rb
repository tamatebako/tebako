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
require "tmpdir"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::RuntimeSdk do
  let(:ruby_ver) { Tebako::RubyVersion.new("3.3.7") }
  let(:runtime_path) { "/cached/runtime/tebako-runtime-0.15.9-3.3.7-macos-arm64" }
  let(:tarball_bytes) { "tarball bytes" }
  let(:tarball_sha256) { Digest::SHA256.hexdigest(tarball_bytes) }

  around do |example|
    Dir.mktmpdir do |tmp|
      @tmp = tmp
      example.run
    end
  end

  let(:deps_dir) { File.join(@tmp, "deps") }
  let(:layout_dir) { File.join(@tmp, "layout") }
  let(:sdk) { described_class.new(runtime_path, deps_dir, ruby_ver) }
  let(:sdk_root) { sdk.sdk_root }

  def write_layout_rbconfig(configure_args)
    arch_dir = File.join(layout_dir, "lib", "ruby", "3.3.0", "arm64-darwin24")
    FileUtils.mkdir_p(arch_dir)
    File.write(File.join(arch_dir, "rbconfig.rb"), %(  CONFIG["configure_args"] = " #{configure_args}"\n))
  end

  def ok_response(body)
    Net::HTTPOK.new("1.1", 200, "OK").tap do |response|
      allow(response).to receive(:body).and_return(body)
    end
  end

  def stub_http # rubocop:disable Metrics/AbcSize
    sums = ok_response("#{tarball_sha256}  tfs-ruby-3.3.7-src.tar.gz\n")
    tarball = ok_response(tarball_bytes)
    http = instance_double(Net::HTTP)
    allow(Net::HTTP).to receive(:new).and_return(http)
    allow(http).to receive(:use_ssl=)
    allow(http).to receive(:open_timeout=)
    allow(http).to receive(:read_timeout=)
    allow(http).to receive(:start).and_return(sums, tarball, tarball)
  end

  # Fake the external steps of provisioning: extraction plants the source
  # tree, configure plants the generated arch header, nm/cc/ar plant the
  # symbol stub archive
  def stub_build_steps(recorded, nm_output: nil, asm_contents: []) # rubocop:disable Metrics/AbcSize, Metrics/MethodLength
    src_name = "tfs-ruby-3.3.7-src"
    ok = instance_double(Process::Status, success?: true)
    allow(Open3).to receive(:capture2e) do |*args, **kwargs|
      case args.first
      when "tar"
        src_dir = File.join(args.last, src_name)
        FileUtils.mkdir_p(File.join(src_dir, "include"))
        File.write(File.join(src_dir, "include", "ruby.h"), "/* ruby.h */")
        ["", ok]
      when "./configure"
        recorded << args
        hdr = File.join(kwargs[:chdir], ".ext", "include", "arm64-darwin23", "ruby")
        FileUtils.mkdir_p(hdr)
        File.write(File.join(hdr, "config.h"), "/* config.h */")
        ["", ok]
      when "nm"
        [nm_output || "0000000100001234 T _rb_eval_string\n0000000100005678 T _rb_intern", ok]
      when "cc"
        asm_contents << File.read(args[2])
        FileUtils.touch(args.last)
        ["", ok]
      when "ar"
        File.write(args[2], "!<arch>\n")
        ["", ok]
      end
    end
  end

  describe "#resolve" do
    it "returns the sdk root without any provisioning when the marker is complete" do
      FileUtils.mkdir_p(File.join(sdk_root, "include"))
      File.write(File.join(sdk_root, "include", "ruby.h"), "x")
      FileUtils.mkdir_p(File.join(sdk_root, "archhdr", "ruby"))
      File.write(File.join(sdk_root, "archhdr", "ruby", "config.h"), "x")
      File.write(File.join(sdk_root, described_class::MARKER_FILE), "x")

      expect(Net::HTTP).not_to receive(:new)
      expect(Open3).not_to receive(:capture2e)
      expect(sdk.resolve).to eq(sdk_root)
    end

    it "fails with error 135 when the runtime layout carries no rbconfig" do
      allow(Tebako::RuntimeManager).to receive(:layout).and_return(layout_dir)
      expect { sdk.resolve }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(135) }
    end

    it "fails with error 135 when the source tarball fails verification" do
      write_layout_rbconfig("'--without-gmp'")
      allow(Tebako::RuntimeManager).to receive(:layout).and_return(layout_dir)
      sums = ok_response("#{"0" * 64}  tfs-ruby-3.3.7-src.tar.gz\n")
      http = instance_double(Net::HTTP)
      allow(Net::HTTP).to receive(:new).and_return(http)
      allow(http).to receive(:use_ssl=)
      allow(http).to receive(:open_timeout=)
      allow(http).to receive(:read_timeout=)
      allow(http).to receive(:start).and_return(sums)

      expect { sdk.resolve }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(135) }
    end

    it "replays the recorded configure flags filtered of build-machine assignments" do
      write_layout_rbconfig(
        "'--with-openssl-dir=/opt/homebrew/opt/openssl@3' '--without-gmp' '--disable-shared' " \
        "'--without-ext=dbm,win32,win32ole,-test-/*' '--prefix=/build/machine/prefix' " \
        "'cflags=-I/build/machine/include' 'LDFLAGS=-L/build/machine/lib' 'LIBS=' 'CC=clang'"
      )
      allow(Tebako::RuntimeManager).to receive(:layout).and_return(layout_dir)
      stub_http
      recorded = []
      stub_build_steps(recorded)

      expect(sdk.resolve).to eq(sdk_root)

      configure_call = recorded.find { |args| args.first == "./configure" }
      expect(configure_call).not_to be_nil
      args = configure_call[1..]
      expect(args).to include("--with-openssl-dir=/opt/homebrew/opt/openssl@3",
                              "--without-gmp", "--disable-shared")
      expect(args).to include("--with-out-ext=dbm,win32,win32ole,-test-/*")
      expect(args).not_to include(a_string_starting_with("cflags="),
                                  a_string_starting_with("LDFLAGS="),
                                  a_string_starting_with("LIBS="),
                                  a_string_starting_with("CC="),
                                  "--prefix=/build/machine/prefix")
      expect(args.grep(/--prefix=/).first).to start_with("--prefix=#{File.join(sdk_root)}/tmp-")
    end

    it "installs the header tree, the arch config header and the symbol stub archive" do
      write_layout_rbconfig("'--without-gmp'")
      allow(Tebako::RuntimeManager).to receive(:layout).and_return(layout_dir)
      stub_http
      stub_build_steps([])

      sdk.resolve

      expect(File.read(File.join(sdk.include_dir, "ruby.h"))).to eq("/* ruby.h */")
      expect(File.read(File.join(sdk.archhdr_dir, "ruby", "config.h"))).to eq("/* config.h */")
      expect(File.file?(File.join(sdk_root, "lib", "libruby-stub.a"))).to be(true)
      expect(File.file?(File.join(sdk_root, described_class::MARKER_FILE))).to be(true)
    end

    it "keeps CRT boundary symbols, Mach-O pseudo-symbols and third-party library symbols out of the stub" do
      write_layout_rbconfig("'--without-gmp'")
      allow(Tebako::RuntimeManager).to receive(:layout).and_return(layout_dir)
      stub_http
      asm_contents = []
      stub_build_steps([], asm_contents: asm_contents,
                           nm_output: "000000000029aa44 T _start\n" \
                                      "0000000001a16740 B __bss_start\n" \
                                      "0000000001a0ffa0 D __data_start\n" \
                                      "0000000001acd7b0 B _end\n" \
                                      "0000000100001234 T __mh_execute_header\n" \
                                      "00000001002cd2c8 T _rb_hash_bulk_insert\n" \
                                      "0000000100abcdef T _ruby_xmalloc\n" \
                                      "0000000100202068 T _EVP_PKEY_new_raw_private_key\n" \
                                      "0000000100ffffff T _SSL_CTX_set_ciphersuites\n")

      sdk.resolve

      asm = asm_contents.join
      expect(asm).to include(".globl _rb_hash_bulk_insert")
      expect(asm).to include(".globl _ruby_xmalloc")
      %w[_start __bss_start __data_start _end __mh_execute_header
         _EVP_PKEY_new_raw_private_key _SSL_CTX_set_ciphersuites].each do |symbol|
        expect(asm).not_to include(".globl #{symbol}\n")
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
