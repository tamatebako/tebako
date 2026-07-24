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
require "tmpdir"
require "zlib"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::Stitcher do
  let(:header_size) { Tebako::Stitcher::HEADER_SIZE }
  let(:slot_size) { Tebako::Stitcher::SLOT_SIZE }
  let(:magic) { Tebako::Stitcher::MAGIC }

  let(:runtime_body) { "fake tebako runtime binary\n" * 37 } # 1023 bytes, off-alignment on purpose
  let(:image_one) { "DWARFS-IMAGE-ONE|" * 61 } # 854 bytes
  let(:image_two) { "dw2|" * 100 } # 400 bytes

  around do |example|
    Dir.mktmpdir("tebako-stitcher-spec") do |dir|
      @dir = dir
      @runtime = File.join(dir, "runtime")
      File.binwrite(@runtime, runtime_body)
      @img1 = File.join(dir, "app.tfs")
      File.binwrite(@img1, image_one)
      @img2 = File.join(dir, "extra.tfs")
      File.binwrite(@img2, image_two)
      @output = File.join(dir, "out", "package")
      example.run
    end
  end

  def stitch(**kwargs)
    Tebako::Stitcher.stitch(@runtime,
                            images: [{ path: @img1, mount_point: "/__tebako_memfs__", format_id: 1 }],
                            output: @output, **kwargs)
  end

  # Independent reader of the spec §4.3 wire format, used to verify the writer
  def read_trailer(path)
    data = File.binread(path)
    header = data[-Tebako::Stitcher::HEADER_SIZE..]
    slot_count = header[18, 4].unpack1("V")
    table_offset = header[22, 8].unpack1("Q")
    read_header_fields(header).merge(
      slot_count: slot_count, slot_table_offset: table_offset,
      slots: read_slots(data, table_offset, slot_count),
      crc32: header[162, 4].unpack1("V"), crc_region: header[0, 162], total_size: data.size
    )
  end

  def read_header_fields(header)
    { magic: header[0, 10],
      version: header[10, 4].unpack1("V"),
      package_flags: header[14, 4].unpack1("V"),
      runtime_ref: header[30, 128].split("\0", 2).first,
      launcher_abi: header[158, 4].unpack1("V") }
  end

  def read_slots(data, table_offset, slot_count)
    (0...slot_count).map do |i|
      rec = data[table_offset + (i * Tebako::Stitcher::SLOT_SIZE), Tebako::Stitcher::SLOT_SIZE]
      { offset: rec[0, 8].unpack1("Q"), size: rec[8, 8].unpack1("Q"),
        format_id: rec[16, 4].unpack1("V"), flags: rec[20, 4].unpack1("V"),
        mount_point: rec[24, 256].split("\0", 2).first }
    end
  end

  describe ".stitch (classic, single image)" do
    before { stitch }

    it "copies the runtime verbatim at offset 0" do
      expect(File.binread(@output, runtime_body.bytesize)).to eq(runtime_body)
    end

    it "creates the output directory and an executable output" do
      expect(File.file?(@output)).to be true
      # File.executable? is extension-driven on Windows (only .exe/.bat/.com
      # report executable regardless of the mode bits), so assert existence
      # there -- the mode bits are a POSIX concept the spec exercises elsewhere
      expect(File.executable?(@output)).to be true unless Gem.win_platform?
    end

    it "appends the image at the next 8-byte aligned offset" do
      trailer = read_trailer(@output)
      expected_offset = (runtime_body.bytesize + 7) & ~7
      expect(trailer[:slots].first[:offset]).to eq(expected_offset)
      expect(File.binread(@output, image_one.bytesize, expected_offset)).to eq(image_one)
    end

    it "zero-fills the alignment gap" do
      gap = ((runtime_body.bytesize + 7) & ~7) - runtime_body.bytesize
      expect(File.binread(@output, gap, runtime_body.bytesize)).to eq("\0" * gap)
    end

    it "writes the slot table immediately after the last image" do
      trailer = read_trailer(@output)
      slot = trailer[:slots].first
      expect(trailer[:slot_table_offset]).to eq(slot[:offset] + slot[:size])
    end

    it "writes a spec §4.3 trailer header at EOF" do
      trailer = read_trailer(@output)
      expect(trailer[:magic]).to eq(magic)
      expect(trailer[:version]).to eq(1)
      expect(trailer[:package_flags]).to eq(0)
      expect(trailer[:slot_count]).to eq(1)
      expect(trailer[:runtime_ref]).to eq("")
      expect(trailer[:launcher_abi]).to eq(0)
      expect(trailer[:total_size]).to eq(trailer[:slot_table_offset] + slot_size + header_size)
    end

    it "stores a zlib-polynomial CRC32 of header bytes [0, 162)" do
      trailer = read_trailer(@output)
      expect(trailer[:crc32]).to eq(Zlib.crc32(trailer[:crc_region]))
    end

    it "records slot fields (size, format_id, flags, mount_point)" do
      slot = read_trailer(@output)[:slots].first
      expect(slot[:size]).to eq(image_one.bytesize)
      expect(slot[:format_id]).to eq(1)
      expect(slot[:flags]).to eq(0)
      expect(slot[:mount_point]).to eq("/__tebako_memfs__")
    end
  end

  describe ".stitch with two images" do
    it "writes two slot records in order at aligned offsets" do
      stitch(images: [{ path: @img1, mount_point: "/__tebako_memfs__", format_id: 1 },
                      { path: @img2, mount_point: "extra", format_id: 0 }])
      trailer = read_trailer(@output)
      expect(trailer[:slot_count]).to eq(2)

      first, second = trailer[:slots]
      expect(first[:offset]).to eq((runtime_body.bytesize + 7) & ~7)
      expect(second[:offset]).to eq(((first[:offset] + first[:size]) + 7) & ~7)
      expect(second[:format_id]).to eq(0)
      expect(second[:mount_point]).to eq("extra")
      expect(File.binread(@output, image_two.bytesize, second[:offset])).to eq(image_two)
      expect(trailer[:slot_table_offset]).to eq(second[:offset] + second[:size])
    end
  end

  describe ".stitch with lean: true" do
    it "sets the LEAN package flag and the runtime_ref" do
      stitch(lean: true, ruby_version: "3.3.7")
      trailer = read_trailer(@output)
      expect(trailer[:package_flags] & 0x1).to eq(1)
      expect(trailer[:runtime_ref]).to eq("ruby@3.3.7;tebako=#{Tebako::VERSION}")
    end

    it "honours an explicit tebako_version" do
      stitch(lean: true, ruby_version: "3.2.7", tebako_version: "0.15.0")
      expect(read_trailer(@output)[:runtime_ref]).to eq("ruby@3.2.7;tebako=0.15.0")
    end

    it "appends ;sha256= to the runtime_ref for a fat package" do
      sha256 = "a" * 64
      stitch(lean: true, ruby_version: "3.3.7", runtime_sha256: sha256)
      expect(read_trailer(@output)[:runtime_ref]).to eq("ruby@3.3.7;tebako=#{Tebako::VERSION};sha256=#{sha256}")
    end

    it "rejects a malformed runtime_sha256" do
      expect { stitch(lean: true, ruby_version: "3.3.7", runtime_sha256: "XYZ") }
        .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(126) }
    end

    it "fails without a ruby_version" do
      expect { stitch(lean: true) }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(126) }
    end
  end

  describe "validation" do
    it "rejects an empty image list" do
      expect { stitch(images: []) }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(126) }
    end

    it "rejects more than 8 images" do
      images = Array.new(9) { |i| { path: @img1, mount_point: "m#{i}", format_id: 1 } }
      expect { stitch(images: images) }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(126) }
    end

    it "rejects duplicate mount points" do
      images = [{ path: @img1, mount_point: "dup", format_id: 1 }, { path: @img2, mount_point: "dup", format_id: 1 }]
      expect { stitch(images: images) }.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(126) }
    end

    it "rejects a mount point of 256 bytes (255 is accepted)" do
      expect { stitch(images: [{ path: @img1, mount_point: "m" * 255, format_id: 1 }]) }.not_to raise_error
      expect { stitch(images: [{ path: @img1, mount_point: "m" * 256, format_id: 1 }]) }
        .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(126) }
    end

    it "rejects an out-of-range format_id" do
      expect { stitch(images: [{ path: @img1, mount_point: "m", format_id: 5 }]) }
        .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(126) }
    end

    it "accepts a runtime payload slot (format_id 4) with an empty mount point" do
      stitch(lean: true, ruby_version: "3.3.7",
             images: [{ path: @img1, mount_point: "/__tebako_memfs__", format_id: 1 },
                      { path: @img2, mount_point: "", format_id: Tebako::Stitcher::FORMAT_RUNTIME }])
      trailer = read_trailer(@output)
      expect(trailer[:slot_count]).to eq(2)
      expect(trailer[:slots].last[:format_id]).to eq(4)
      expect(trailer[:slots].last[:mount_point]).to eq("")
      expect(File.binread(@output, image_two.bytesize, trailer[:slots].last[:offset])).to eq(image_two)
    end

    it "rejects a missing runtime" do
      expect do
        Tebako::Stitcher.stitch(File.join(@dir, "nope"),
                                images: [{ path: @img1, mount_point: "m" }], output: @output)
      end.to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(127) }
    end

    it "rejects a missing image and does not leave a partial output" do
      expect { stitch(images: [{ path: File.join(@dir, "nope"), mount_point: "m" }]) }
        .to raise_error(Tebako::Error) { |e| expect(e.error_code).to eq(127) }
      expect(File.exist?(@output)).to be false
    end
  end

  describe "format_id default" do
    it "defaults to 1 (dwarfs) when omitted" do
      stitch(images: [{ path: @img1, mount_point: "/__tebako_memfs__" }])
      expect(read_trailer(@output)[:slots].first[:format_id]).to eq(1)
    end
  end

  describe "macOS ad-hoc re-sign" do
    it "is a no-op off macOS" do
      allow(Tebako::Stitcher).to receive(:macos?).and_return(false)
      expect(Tebako::Stitcher).not_to receive(:adhoc_resign)
      stitch
    end

    it "re-signs when the runtime carries a signature" do
      allow(Tebako::Stitcher).to receive(:macos?).and_return(true)
      allow(Tebako::Stitcher).to receive(:signed?).and_return(true)
      expect(Tebako::Stitcher).to receive(:adhoc_resign).with(@output)
      stitch
    end

    it "does not touch an unsigned runtime" do
      allow(Tebako::Stitcher).to receive(:macos?).and_return(true)
      allow(Tebako::Stitcher).to receive(:signed?).and_return(false)
      expect(Tebako::Stitcher).not_to receive(:adhoc_resign)
      stitch
    end

    it "warns and keeps the package when the re-sign fails" do
      allow(Tebako::Stitcher).to receive(:macos?).and_return(true)
      allow(Tebako::Stitcher).to receive(:signed?).and_return(true)
      allow(Tebako::Stitcher).to receive(:adhoc_resign).and_return(false)
      expect { stitch }.to output(/ad-hoc re-sign failed/).to_stderr
      expect(File.file?(@output)).to be true
      expect(read_trailer(@output)[:magic]).to eq(magic)
    end

    if Tebako::Stitcher.macos? && system("which codesign > /dev/null 2>&1")
      it "keeps a stitched signed binary valid (live codesign check)" do
        signed_runtime = File.join(@dir, "signed-runtime")
        FileUtils.cp("/bin/echo", signed_runtime)
        FileUtils.chmod(0o755, signed_runtime)
        skip "fixture binary is not signed" unless Tebako::Stitcher.signed?(signed_runtime)

        Tebako::Stitcher.stitch(signed_runtime,
                                images: [{ path: @img1, mount_point: "/__tebako_memfs__", format_id: 1 }],
                                output: @output)
        expect(system("codesign", "-v", @output)).to be(true)
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
