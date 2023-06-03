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

require_relative "tebako/version"

# Tebako - an executable packager
# Implementation of tebako error class and some support methods
module Tebako
  # Tebako error class
  class TebakoError < StandardError
    def initialize(msg = "Unspecified error", code = 255)
      @error_code = code
      super(msg)
    end
    attr_accessor :error_code
  end

  class << self
    def m_files
      @m_files ||= case RbConfig::CONFIG["host_os"]
                   when /linux/, /darwin/
                     "Unix Makefiles"
                   when /msys/
                     "Ninja"
                   else
                     raise TebakoError.new "#{RbConfig::CONFIG["host_os"]} is not supported yet, exiting", 254
                   end
    end

    def packaging_error(code)
      msg = PACKAGING_ERRORS[code]
      msg = "Unknown packaging error" if msg.nil?
      raise TebakoError.new msg, code
    end

    PACKAGING_ERRORS = {
      101 => "'tebako setup' configure step failed",
      102 => "'tebako setup' build step failed",
      103 => "'tebako press' configure step failed",
      104 => "'tebako press' build step failed"
    }.freeze
  end
end
