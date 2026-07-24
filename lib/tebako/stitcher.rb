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

require "fileutils"
require "open3"
require "rbconfig"
require "zlib"

# Tebako - an executable packager
module Tebako
  # Stitches tebako images onto a prebuilt runtime binary, producing a
  # single-file package with a tpkg manifest trailer (spec §4.3).
  #
  # Wire layout (all integers little-endian), appended after the runtime:
  #   [runtime bytes][pad to 8][image 0][pad to 8]...[image n-1]
  #   [slot table: slot_count x 280 bytes][trailer header: 166 bytes at EOF]
  #
  # This is a pure-Ruby reimplementation of the tpkg.h writer (libtfs
  # include/tebako/tpkg.h); the byte stream is identical to tpkg_write_fd()
  # for the same manifest. CRC32 is the zlib polynomial (0xEDB88320,
  # init/xorout 0xFFFFFFFF) — exactly what Zlib.crc32 computes.
  #
  # Codesigning: appending bytes invalidates any embedded code signature.
  # On macOS a signed runtime (ad-hoc included, detected via `codesign -dv`)
  # is re-signed ad-hoc after stitching (codesign --remove-signature, then
  # codesign --sign - --force). Note that codesign(1) refuses to re-sign thin
  # Mach-O binaries carrying trailing payload ("main executable failed strict
  # validation"), so the re-sign is best-effort: on failure a warning is
  # printed and the package is kept — the ad-hoc linker signature is
  # invalidated by construction, but the binary still executes on macOS
  # (verified on macOS 14/arm64). Re-signing with a real identity remains a
  # user post-press step. An unsigned runtime is left alone.
  # On Windows signing is a no-op — re-applying an Authenticode signature
  # (signtool) is left to the user as a post-press step.
  class Stitcher # rubocop:disable Metrics/ClassLength
    MAGIC = "TEBAKOTFS\0".b # 10 bytes, NUL-terminated
    VERSION = 1
    MAX_SLOTS = 8
    HEADER_SIZE = 166
    SLOT_SIZE = 280
    MOUNT_POINT_LEN = 256
    RUNTIME_REF_LEN = 128
    ALIGNMENT = 8

    FLAG_LEAN = 0x1

    FORMAT_AUTO = 0
    FORMAT_DWARFS = 1
    FORMAT_SQUASHFS = 2
    FORMAT_ZIP = 3
    # Runtime payload slot of a fat package: installed into the shared cache
    # by the bootstrap at first run, never mounted (tpkg.h TPKG_FORMAT_RUNTIME)
    FORMAT_RUNTIME = 4
    FORMAT_IDS = (FORMAT_AUTO..FORMAT_RUNTIME).freeze

    # Header field offsets
    OFF_PACKAGE_FLAGS = 14
    OFF_SLOT_COUNT = 18
    OFF_TABLE = 22
    OFF_RUNTIME_REF = 30
    OFF_CRC32 = 162

    class << self
      # Stitch +images+ (Array of {path:, mount_point:, format_id:}) onto a
      # copy of the runtime at +runtime_path+, writing +output+.
      # lean: true marks the package LEAN and records runtime_ref
      # ("ruby@<ruby_version>;tebako=<tebako_version>"); classic packages
      # carry an empty runtime_ref. runtime_sha256 appends ";sha256=<hex>" to
      # the runtime_ref — the checksum the bootstrap verifies a fat package's
      # runtime payload slot against before installing it into the cache.
      def stitch(runtime_path, images:, output:, lean: false, ruby_version: nil, # rubocop:disable Metrics/ParameterLists
                 tebako_version: Tebako::VERSION, launcher_abi: 0, runtime_sha256: nil)
        images = normalize_images(images)
        validate_inputs!(runtime_path, images, lean, ruby_version)
        validate_runtime_sha256!(runtime_sha256)

        FileUtils.mkdir_p(File.dirname(output))
        FileUtils.cp(runtime_path, output)
        FileUtils.chmod(0o755, output)

        append(images, output, package_flags(lean), runtime_ref(lean, ruby_version, tebako_version, runtime_sha256),
               launcher_abi)
        resign_if_needed(output)
        output
      end

      def macos?
        RbConfig::CONFIG["host_os"] =~ /darwin/i ? true : false
      end

      # True when +path+ carries any code signature (ad-hoc included)
      def signed?(path)
        _out, _err, status = Open3.capture3("codesign", "-dv", path)
        status.success?
      rescue Errno::ENOENT
        false
      end

      # Re-apply an ad-hoc signature after the binary was mutated; true on
      # success. codesign(1) refuses to re-sign thin Mach-O binaries with
      # trailing payload (strict validation) — callers treat failure as
      # non-fatal (see the class comment).
      def adhoc_resign(path)
        system("codesign", "--remove-signature", path, out: File::NULL, err: File::NULL) &&
          system("codesign", "--sign", "-", "--force", path, out: File::NULL, err: File::NULL)
      end

      private

      def normalize_images(images)
        images.map do |image|
          { path: image[:path], mount_point: image[:mount_point].to_s,
            format_id: image.fetch(:format_id, FORMAT_DWARFS) }
        end
      end

      def validate_inputs!(runtime_path, images, lean, ruby_version)
        Tebako.packaging_error(126, "at least one image is required") if images.empty?
        if images.size > MAX_SLOTS
          Tebako.packaging_error(126, "#{images.size} images given, at most #{MAX_SLOTS} are supported")
        end
        if lean && ruby_version.nil?
          Tebako.packaging_error(126, "lean packages require ruby_version for the runtime_ref")
        end
        validate_files!(runtime_path, images)
        validate_images!(images)
      end

      def validate_files!(runtime_path, images)
        Tebako.packaging_error(127, "runtime not found: #{runtime_path}") unless File.file?(runtime_path)
        images.each do |image|
          Tebako.packaging_error(127, "image not found: #{image[:path]}") unless File.file?(image[:path])
        end
      end

      def validate_images!(images)
        seen = {}
        images.each do |image|
          validate_image!(image)
          next if image[:format_id] == FORMAT_RUNTIME # payload slots are never mounted

          mount = image[:mount_point]
          Tebako.packaging_error(126, "duplicate mount point '#{mount}'") if seen[mount]

          seen[mount] = true
        end
      end

      def validate_image!(image)
        unless FORMAT_IDS.cover?(image[:format_id])
          Tebako.packaging_error(126, "invalid format_id #{image[:format_id]} (0..4 expected)")
        end
        return unless image[:mount_point].bytesize >= MOUNT_POINT_LEN

        Tebako.packaging_error(126,
                               "mount point '#{image[:mount_point][0, 32]}...' exceeds #{MOUNT_POINT_LEN - 1} bytes")
      end

      def validate_runtime_sha256!(runtime_sha256)
        return if runtime_sha256.nil? || runtime_sha256.match?(/\A[0-9a-f]{64}\z/)

        Tebako.packaging_error(126, "runtime_sha256 must be 64 lowercase hex characters")
      end

      def package_flags(lean)
        lean ? FLAG_LEAN : 0
      end

      def runtime_ref(lean, ruby_version, tebako_version, runtime_sha256)
        return "" unless lean

        ref = "ruby@#{ruby_version};tebako=#{tebako_version}"
        ref += ";sha256=#{runtime_sha256}" if runtime_sha256
        if ref.bytesize >= RUNTIME_REF_LEN
          Tebako.packaging_error(126, "runtime_ref '#{ref}' exceeds #{RUNTIME_REF_LEN - 1} bytes")
        end
        ref
      end

      # Appends images at 8-byte-aligned offsets; returns slot descriptors
      def append(images, output, package_flags, runtime_ref, launcher_abi)
        File.open(output, "ab") do |io|
          io.seek(0, IO::SEEK_END) # append mode starts at pos 0 until the first write
          slots = append_images(io, images)
          write_trailer(io, slots, package_flags, runtime_ref, launcher_abi)
        end
      end

      def append_images(io, images)
        images.map do |image|
          pad(io)
          offset = io.pos
          File.open(image[:path], "rb") { |src| IO.copy_stream(src, io) }
          { offset: offset, size: io.pos - offset, format_id: image[:format_id], flags: 0,
            mount_point: image[:mount_point] }
        end
      end

      def pad(io)
        remainder = io.pos % ALIGNMENT
        io.write("\0" * (ALIGNMENT - remainder)) unless remainder.zero?
      end

      def write_trailer(io, slots, package_flags, runtime_ref, launcher_abi)
        slot_table_offset = io.pos
        slots.each { |slot| io.write(pack_slot(slot)) }
        io.write(pack_header(slots.size, slot_table_offset, package_flags, runtime_ref, launcher_abi))
      end

      def pack_slot(slot)
        [slot[:offset], slot[:size]].pack("Q2") +
          [slot[:format_id], slot[:flags]].pack("V2") +
          fixed_string(slot[:mount_point], MOUNT_POINT_LEN)
      end

      def pack_header(slot_count, slot_table_offset, package_flags, runtime_ref, launcher_abi)
        header = MAGIC.dup
        header << [VERSION, package_flags, slot_count].pack("V3")
        header << [slot_table_offset].pack("Q")
        header << fixed_string(runtime_ref, RUNTIME_REF_LEN)
        header << [launcher_abi].pack("V")
        header << [Zlib.crc32(header)].pack("V") # header bytes [0, 162)
        header
      end

      # NUL-padded fixed-width field; callers validate the length beforehand
      def fixed_string(string, width)
        string.b.ljust(width, "\0")
      end

      def resign_if_needed(output)
        # Windows: no-op. Re-signing an Authenticode-signed runtime is a
        # documented user post-press step (signtool sign ...).
        return unless macos?
        return unless signed?(output)
        return if adhoc_resign(output)

        warn "Warning: ad-hoc re-sign failed for #{output}; the package still executes on macOS, " \
             "but its code signature is invalidated by the appended images. " \
             "Re-sign it with your own identity if you need a valid signature."
      end
    end
  end
end
