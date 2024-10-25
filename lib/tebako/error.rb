# frozen_string_literal: true

# Copyright (c) 2023-204 [Ribose Inc](https://www.ribose.com).
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

# Tebako - an executable packager
module Tebako
  PACKAGING_ERRORS = {
    101 => "'tebako setup' configure step failed",
    102 => "'tebako setup' build step failed",
    103 => "'tebako press' configure step failed",
    104 => "'tebako press' build step failed",
    105 => "Failed to map MSys path to Windows",
    106 => "Entry point does not exist or is not accessible",
    107 => "Project root does not exist or is not accessible",
    108 => "Package working directory does not exist",
    109 => "Invalid Ruby version format",
    110 => "Ruby version is not supported",
    111 => "Ruby version is not supported on Windows",
    112 => "OS is not supported",
    113 => "Path to root shall be absolute. Relative path is not allowed",
    114 => "Entry point is not within the project root",
    201 => "Warning. Could not create cache version file"
  }.freeze

  class << self
    def packaging_error(code)
      msg = PACKAGING_ERRORS[code]
      msg = "Unknown packaging error" if msg.nil?
      raise Tebako::Error.new msg, code
    end
  end

  # Tebako error class
  class Error < StandardError
    def initialize(msg = "Unspecified error", code = 255)
      @error_code = code
      super(msg)
    end
    attr_accessor :error_code
  end
end
