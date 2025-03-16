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

RSpec.describe Tebako do # rubocop:disable Metrics/BlockLength
  describe "PACKAGING_ERRORS" do # rubocop:disable Metrics/BlockLength
    it "is defined" do
      expect(defined?(Tebako::PACKAGING_ERRORS)).to be_truthy
    end

    describe ".packaging_error" do
      it "raises the correct error for known error codes" do
        expect { Tebako.packaging_error(101) }.to raise_error(Tebako::Error, "'tebako setup' configure step failed")
        expect do
          Tebako.packaging_error(106)
        end.to raise_error(Tebako::Error, "Entry point does not exist or is not accessible")
      end

      it "raises a generic error message for unknown error codes" do
        expect { Tebako.packaging_error(999) }.to raise_error(Tebako::Error, "Unknown packaging error")
      end
    end

    describe "Error" do
      it "is defined" do
        expect(defined?(Tebako::Error)).to be_truthy
      end

      it "inherits from StandardError" do
        expect(Tebako::Error).to be < StandardError
      end

      it "can be instantiated" do
        expect { Tebako::Error.new }.not_to raise_error
      end

      it "initializes with the correct message and error code" do
        error = Tebako::Error.new("Custom error message", 123)
        expect(error.message).to eq("Custom error message")
        expect(error.error_code).to eq(123)
      end

      it "has an accessible and modifiable error_code attribute" do
        error = Tebako::Error.new
        error.error_code = 456
        expect(error.error_code).to eq(456)
      end
    end
  end
end
