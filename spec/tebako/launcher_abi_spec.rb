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

RSpec.describe Tebako::LauncherAbi do
  # These values are the wire contract with tebako-bootstrap and with
  # src/tebako-main.cpp — changing any of them is an ABI break.
  it "pins launcher ABI version 1" do
    expect(Tebako::LauncherAbi::VERSION).to eq(1)
  end

  it "pins the option spellings the bootstrap emits" do
    expect(Tebako::LauncherAbi::IMAGE_ARG).to eq("--tebako-image")
    expect(Tebako::LauncherAbi::ENTRY_ARG).to eq("--tebako-entry")
    expect(Tebako::LauncherAbi::VERSION_ARG).to eq("--tebako-launcher-abi")
  end

  describe ".image_spec" do
    it "formats <file>:<slot>:<mount-point>" do
      expect(Tebako::LauncherAbi.image_spec("/pkg/app", 0, "/__tebako_memfs__"))
        .to eq("/pkg/app:0:/__tebako_memfs__")
    end

    it "leaves colons inside the file component alone (split is on the last two)" do
      expect(Tebako::LauncherAbi.image_spec("C:/pkg/app", 3, "/data"))
        .to eq("C:/pkg/app:3:/data")
    end
  end
end
