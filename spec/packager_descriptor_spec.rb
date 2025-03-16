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

require "stringio"
require "tebako/package_descriptor"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::PackageDescriptor do
  let(:uby_version) { "3.1.2" }
  let(:tebako_version) { "1.0.0" }
  let(:mount_point) { "/app" }
  let(:entry_point) { "/app/start.rb" }
  let(:cwd) { "/app" }
  let(:serialized_signature) { Tebako::PackageDescriptor::SIGNATURE.bytes }

  describe "#initialize" do
    context "when initialized with valid parameters" do
      subject do
        described_class.new(uby_version, tebako_version, mount_point, entry_point, cwd)
      end

      it "parses the ruby version correctly" do
        expect(subject.ruby_version_major).to eq(3)
        expect(subject.ruby_version_minor).to eq(1)
        expect(subject.ruby_version_patch).to eq(2)
      end

      it "parses the tebako version correctly" do
        expect(subject.tebako_version_major).to eq(1)
        expect(subject.tebako_version_minor).to eq(0)
        expect(subject.tebako_version_patch).to eq(0)
      end

      it "assigns the mount point and entry point" do
        expect(subject.mount_point).to eq(mount_point)
        expect(subject.entry_point).to eq(entry_point)
      end

      it "assigns the cwd" do
        expect(subject.cwd).to eq(cwd)
      end
    end

    context "when initialized with an invalid number of arguments" do
      it "raises an ArgumentError" do
        expect { described_class.new }.to raise_error(ArgumentError, "Invalid arguments")
        expect { described_class.new("extra", "args") }.to raise_error(ArgumentError, "Invalid arguments")
      end
    end

    context "when initialized with a serialized buffer" do
      let(:buffer) do
        StringIO.new.tap do |io|
          io.write("TAMATEBAKO") # Signature
          io.write([3, 1, 2].pack("S*")) # Ruby version
          io.write([1, 0, 0].pack("S*")) # Tebako version
          io.write([mount_point.size].pack("S"))
          io.write(mount_point)
          io.write([entry_point.size].pack("S"))
          io.write(entry_point)
          io.write([1].pack("C")) # cwd present
          io.write([cwd.size].pack("S"))
          io.write(cwd)
        end.string
      end

      subject { described_class.new(buffer.bytes) }

      it "deserializes the ruby version correctly" do
        expect(subject.ruby_version_major).to eq(3)
        expect(subject.ruby_version_minor).to eq(1)
        expect(subject.ruby_version_patch).to eq(2)
      end

      it "deserializes the tebako version correctly" do
        expect(subject.tebako_version_major).to eq(1)
        expect(subject.tebako_version_minor).to eq(0)
        expect(subject.tebako_version_patch).to eq(0)
      end

      it "deserializes the mount point and entry point" do
        expect(subject.mount_point).to eq(mount_point)
        expect(subject.entry_point).to eq(entry_point)
      end

      it "deserializes the cwd correctly" do
        expect(subject.cwd).to eq(cwd)
      end

      context "when the signature is invalid" do
        let(:invalid_buffer) { ["INVALID".bytes, buffer.bytes.drop(Tebako::PackageDescriptor::SIGNATURE.size)].flatten }

        it "raises an ArgumentError" do
          expect { described_class.new(invalid_buffer) }.to raise_error(ArgumentError, "Invalid or missing signature")
        end
      end

      context "when cwd is absent" do
        let(:buffer_without_cwd) do
          StringIO.new.tap do |io|
            io.write("TAMATEBAKO") # Signature
            io.write([3, 1, 2].pack("S*")) # Ruby version
            io.write([1, 0, 0].pack("S*")) # Tebako version
            io.write([mount_point.size].pack("S"))
            io.write(mount_point)
            io.write([entry_point.size].pack("S"))
            io.write(entry_point)
            io.write([0].pack("C")) # cwd not present
          end.string
        end

        subject { described_class.new(buffer_without_cwd.bytes) }

        it "sets cwd to nil" do
          expect(subject.cwd).to be_nil
        end
      end
    end
  end

  describe "#serialize" do
    subject do
      described_class.new(uby_version, tebako_version, mount_point, entry_point, cwd)
    end

    context "when cwd is not nil" do
      it "serializes to a valid byte array" do
        serialized = subject.serialize
        expected = "TAMATEBAKO\u0003\u0000\u0001\u0000\u0002\u0000\u0001\u0000\u0000\u0000" \
                   "\u0000\u0000\u0004\u0000/app\r\u0000/app/start.rb\u0001\u0004\u0000/app"
        expect(serialized.bytes).to eq(expected.bytes)
      end
    end

    context "when cwd is nil" do
      subject do
        described_class.new(uby_version, tebako_version, mount_point, entry_point, nil)
      end

      it "serializes cwd absence as 0" do
        serialized = subject.serialize
        expected = "TAMATEBAKO\u0003\u0000\u0001\u0000\u0002\u0000\u0001\u0000\u0000\u0000" \
                   "\u0000\u0000\u0004\u0000/app\r\u0000/app/start.rb\u0000"
        expect(serialized.bytes).to eq(expected.bytes)
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
