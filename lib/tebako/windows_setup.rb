# frozen_string_literal: true

# Copyright (c) 2023 [Ribose Inc](https://www.ribose.com).
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

require "digest"
require "net/http"
require "rubygems/package"
require "zlib"

require_relative "packager"

# Tebako - an executable packager
# Command-line interface methods
module Tebako
  # Cli methods
  module WindowsSetup
    XPATH = "c:/vcpkg/installed/x64-windows-static-release"
    LIBRUBYARG_SUBST = <<~SUBST
        # Start of tebako patch
        XEXTLIBS = ffi.lib libcrypto.lib libssl.lib ole32.lib oleaut32.lib crypt32.lib readline.lib yaml.lib zlib.lib
        LIBRUBYARG    = $(LIBRUBY_A)
        # End of tebako patch
      SUBST

    WIN32_PATCH = {
      "win32/Makefile.sub" => {
        "LIBRUBYARG    = $(LIBRUBY)" => LIBRUBYARG_SUBST,
        "$(OUTFLAG)$@ $(LIBRUBYARG) -link $(LDFLAGS) $(XLDFLAGS)" =>
                      "$(OUTFLAG)$@ $(DLDOBJS) $(LIBRUBYARG) $(XEXTLIBS) $(LIBS) -link $(LDFLAGS) $(XLDFLAGS) -libpath:#{XPATH}/lib",
        "$(RUBYW_INSTALL_NAME).res $(OUTFLAG)$@ $(LIBRUBYARG) \\" =>
                      "$(RUBYW_INSTALL_NAME).res $(OUTFLAG)$@ $(DLDOBJS) $(LIBRUBYARG) $(XEXTLIBS) $(LIBS) \\",
        "-link $(LDFLAGS) $(XLDFLAGS) -subsystem:Windows" => "-link $(LDFLAGS) $(XLDFLAGS) -libpath:#{XPATH}/lib -subsystem:Windows"
      },
      # because it is not possible to pass --enable-bundled-libyaml option ...
      "ext/psych/extconf.rb" => {
        "if enable_config(\"bundled-libyaml\", false) || !(find_header('yaml.h') && find_library('yaml', 'yaml_get_version'))" => "if true"
        }
        }.freeze

    ENV_CMD = "\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat\""
    CONFIGURE_CMD = "win32\\configure --prefix=D:\\Projects\\8.Projects\\ruby --with-static-linked-ext "\
                    "--disable-install-doc --with-opt-dir=#{XPATH}"
    class << self
      def setup(ruby_ver, ruby_hash, deps)
        begin
          # vcpkg
          @ruby_src_dir = ruby_source_dir(deps, ruby_ver)
          download_ruby(ruby_ver, ruby_hash, deps)
          Tebako::Packager.do_patch(WIN32_PATCH, @ruby_src_dir)
          system("call #{ENV_CMD} && cd #{@ruby_src_dir} && #{CONFIGURE_CMD} && nmake && nmake install")
#          system("call #{ENV_CMD} &&  cd #{@ruby_src_dir} && nmake install")
          # build
          # install
          # build extension
        rescue StandardError => e
          puts "Could not download Ruby sources: #{e.message}"
          return false
        end
        true
      end

      private

      def download_ruby(ruby_ver, ruby_hash, deps)
        puts "Downloading and extracting Ruby #{ruby_ver} sources"
        url = "https://cache.ruby-lang.org/pub/ruby/#{ruby_ver[0, 3]}/ruby-#{ruby_ver}.tar.gz"

        target_file = File.join(deps, "src", "ruby-#{ruby_ver}.tar.gz")
        FileUtils.mkdir_p(@ruby_src_dir)

        download_and_decompress(url, target_file, ruby_hash)
      end

      def check_existing(target_file, expected_checksum)
        if File.exist?(target_file)
          existing_file_contents = File.binread(target_file)
          existing_checksum = Digest::SHA256.hexdigest(existing_file_contents)

          if existing_checksum == expected_checksum
            puts "Checksum match, reusing file"
            return true
          end

          File.delete(target_file)
        end
        false
      end

      def decompress(source_file)
        FileUtils.rm_rf(@ruby_src_dir)
        FileUtils.mkdir_p(@ruby_src_dir)

        success = system("tar -xzf #{source_file} -C #{@ruby_src_dir} --strip-components=1")

        success || false

        #        Zlib::GzipReader.open(source_file) do |tar_gz|
        #          Gem::Package::TarReader.new(tar_gz) do |tar|
        #            decompress_tar(tar, target_folder)
        #          end
        #        end
        #        true
      end

      def decompress_tar(tar, target_folder)
        entries = {}

        tar.each do |entry|
          parts = entry.full_name.split("/").drop(1)
          new_name = parts.join("/")
          target_path = File.join(target_folder, new_name)
          puts "Extracting #{target_path}"
          if entry.directory?
            FileUtils.mkdir_p(target_path)
            entries[target_path] = entry.header.mtime
          elsif entry.file?
            File.binwrite(target_path, entry.read)
            FileUtils.touch(target_path, mtime: entry.header.mtime)
          end

          entries.each { |path, mtime| FileUtils.touch(path, mtime: mtime) }
        end
      end

      def download(url, target_file, expected_checksum)
        uri = URI(url)

        file_contents = Net::HTTP.get(uri)
        actual_checksum = Digest::SHA256.hexdigest(file_contents)

        if actual_checksum != expected_checksum
          puts "Checksum mismatch: expected #{expected_checksum}, got #{actual_checksum}"
          return false
        end

        File.binwrite(target_file, file_contents)
        true
      end

      def download_and_decompress(url, target_file, expected_checksum)
        if !check_existing(target_file, expected_checksum) && !download(url, target_file, expected_checksum)
          return false
        end

        decompress(target_file)
      end

      def fix_path(path, target_folder)
        parts = path.full_name.split("/").drop(1)
        new_name = parts.join("/")
        File.join(target_folder, new_name)
      end

      def ruby_source_dir(deps, ver)
        File.join(deps, "src", "_ruby_#{ver}")
      end
    end
  end
end

# c:\vcpkg\vcpkg install --triplet x64-windows-static-release libffi openssl readline libyaml zlib
