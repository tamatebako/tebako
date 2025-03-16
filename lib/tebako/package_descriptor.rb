# frozen_string_literal: true

# Copyright (c) 2023-2024 [Ribose Inc](https://www.ribose.com).
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

module Tebako
  # Tebako application package descriptor
  class PackageDescriptor
    SIGNATURE = "TAMATEBAKO"

    attr_reader :ruby_version_major, :ruby_version_minor, :ruby_version_patch, :tebako_version_major,
                :tebako_version_minor, :tebako_version_patch, :mount_point, :entry_point, :cwd

    def initialize(*args)
      if args.size == 1 && args[0].is_a?(Array)
        deserialize(args[0])
      elsif args.size == 5
        construct_from_params(*args)
      else
        raise ArgumentError, "Invalid arguments"
      end
    end

    def serialize
      buffer = StringIO.new

      buffer.write(SIGNATURE)
      serialize_versions(buffer)

      write_string(buffer, @mount_point)
      write_string(buffer, @entry_point)

      serialize_cwd(buffer)
      buffer.string
    end

    private

    def deserialize(buffer)
      stream = StringIO.new(buffer.pack("C*"))

      signature = stream.read(SIGNATURE.size)
      raise ArgumentError, "Invalid or missing signature" if signature != SIGNATURE

      deserialize_versions(stream)
      @mount_point = read_string(stream)
      @entry_point = read_string(stream)

      cwd_present = stream.read(1).unpack1("C")
      @cwd = cwd_present == 1 ? read_string(stream) : nil
    end

    def construct_from_params(ruby_version, tebako_version, mount_point, entry_point, cwd)
      parse_version(ruby_version, :@ruby_version_major, :@ruby_version_minor, :@ruby_version_patch)
      parse_version(tebako_version, :@tebako_version_major, :@tebako_version_minor, :@tebako_version_patch)

      @mount_point = mount_point
      @entry_point = entry_point
      @cwd = cwd
    end

    def deserialize_versions(stream)
      @ruby_version_major = read_uint16(stream)
      @ruby_version_minor = read_uint16(stream)
      @ruby_version_patch = read_uint16(stream)
      @tebako_version_major = read_uint16(stream)
      @tebako_version_minor = read_uint16(stream)
      @tebako_version_patch = read_uint16(stream)
    end

    def parse_version(version, major_sym, minor_sym, patch_sym)
      major, minor, patch = version.split(".").map(&:to_i)
      raise ArgumentError, "Invalid version format" unless major && minor && patch

      instance_variable_set(major_sym, major)
      instance_variable_set(minor_sym, minor)
      instance_variable_set(patch_sym, patch)
    end

    def write_uint16(buffer, value)
      buffer.write([value].pack("v"))
    end

    def read_uint16(stream)
      data = stream.read(2)
      raise ArgumentError, "Unexpected end of stream" if data.nil?

      data.unpack1("v")
    end

    def serialize_cwd(buffer)
      if @cwd
        buffer.write([1].pack("C"))
        write_string(buffer, @cwd)
      else
        buffer.write([0].pack("C"))
      end
    end

    def serialize_versions(buffer)
      write_uint16(buffer, @ruby_version_major)
      write_uint16(buffer, @ruby_version_minor)
      write_uint16(buffer, @ruby_version_patch)
      write_uint16(buffer, @tebako_version_major)
      write_uint16(buffer, @tebako_version_minor)
      write_uint16(buffer, @tebako_version_patch)
    end

    def write_string(buffer, str)
      write_uint16(buffer, str.size)
      buffer.write(str)
    end

    def read_string(stream)
      size = read_uint16(stream)
      stream.read(size)
    end
  end
end
