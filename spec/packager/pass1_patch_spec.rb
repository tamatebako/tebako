# frozen_string_literal: true

# Copyright (c) 2025 [Ribose Inc](https://www.ribose.com).
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

RSpec.describe Tebako::Packager do
  describe ".crt_pass1_patch" do
    let(:mount_point) { "/mnt" }
    let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }

    it "returns Pass1DarwinPatch for darwin os_type" do
      patch = described_class.crt_pass1_patch("darwin", mount_point, ruby_ver)
      expect(patch).to be_a(Tebako::Packager::Pass1DarwinPatch)
    end

    it "returns Pass1MSysPatch for msys os_type" do
      patch = described_class.crt_pass1_patch("msys", mount_point, ruby_ver)
      expect(patch).to be_a(Tebako::Packager::Pass1MSysPatch)
    end

    it "returns Pass1Patch for other os_types" do
      patch = described_class.crt_pass1_patch("linux-musl", mount_point, ruby_ver)
      expect(patch).to be_a(Tebako::Packager::Pass1Patch)
    end
  end
end

RSpec.describe Tebako::Packager::Pass1Patch do # rubocop:disable Metrics/BlockLength
  let(:mount_point) { "/mnt" }

  describe "#initialize" do
    let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }
    let(:patch) { described_class.new(mount_point, ruby_ver) }

    it "initializes with mount_point and ruby_ver" do
      expect(patch.instance_variable_get(:@mount_point)).to eq(mount_point)
      expect(patch.instance_variable_get(:@ruby_ver)).to eq(ruby_ver)
    end
  end

  describe "#patch_map" do # rubocop:disable Metrics/BlockLength
    let(:base_patch_map) do
      {
        "tool/rbinstall.rb" => "TOOL_RBINSTALL_RB_PATCH",
        "lib/rubygems/path_support.rb" => "rubygems_path_support_patch",
        "ext/Setup" => "EXT_SETUP_PATCH"
      }
    end

    before do
      allow_any_instance_of(described_class).to receive(:base_patch_map).and_return(base_patch_map)
    end

    context "when ruby_ver is not ruby34 and is ruby3x" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }
      let(:patch) { described_class.new(mount_point, ruby_ver) }

      it "includes additional patches for ruby3x" do
        expected_patch_map = base_patch_map.merge(
          "ext/bigdecimal/bigdecimal.h" => described_class::EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH,
          "lib/rubygems/openssl.rb" => described_class::RUBYGEMS_OPENSSL_RB_PATCH
        )
        expect(patch.patch_map).to eq(expected_patch_map)
      end
    end

    context "when ruby_ver is ruby34" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.4.1") }
      let(:patch) { described_class.new(mount_point, ruby_ver) }

      it "does not include additional patches for ruby3x" do
        expected_patch_map = base_patch_map
        expect(patch.patch_map).to eq(expected_patch_map)
      end
    end

    context "when ruby_ver is not  ruby3x" do
      let(:ruby_ver) { Tebako::RubyVersion.new("2.7.8") }
      let(:patch) { described_class.new(mount_point, ruby_ver) }

      before do
        stub_const("RUBY_PLATFORM", "x86_64-linux")
      end

      it "does not include additional patches for ruby3x" do
        expected_patch_map = base_patch_map.merge(
          "ext/bigdecimal/bigdecimal.h" => described_class::EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH
        )
        expect(patch.patch_map).to eq(expected_patch_map)
      end
    end

    it "returns a frozen hash" do
      ruby_ver = Tebako::RubyVersion.new("3.3.6")
      patch = described_class.new(mount_point, ruby_ver)
      expect(patch.patch_map).to be_frozen
    end
  end
end

RSpec.describe Tebako::Packager::Pass1DarwinPatch do # rubocop:disable Metrics/BlockLength
  let(:mount_point) { "/mnt" }
  let(:ruby_ver) { double("RubyVersion", ruby34?: false, ruby3x?: true) }
  let(:patch) { described_class.new(mount_point, ruby_ver) }

  describe "#initialize" do
    it "initializes with mount_point and ruby_ver" do
      expect(patch.instance_variable_get(:@mount_point)).to eq(mount_point)
      expect(patch.instance_variable_get(:@ruby_ver)).to eq(ruby_ver)
    end
  end

  describe "#patch_map" do
    let(:base_patch_map) do
      {
        "tool/rbinstall.rb" => "TOOL_RBINSTALL_RB_PATCH",
        "lib/rubygems/path_support.rb" => "rubygems_path_support_patch",
        "ext/Setup" => "EXT_SETUP_PATCH"
      }
    end

    before do
      allow_any_instance_of(described_class).to receive(:base_patch_map).and_return(base_patch_map)
    end

    context "when ruby_ver is not ruby34 and is ruby3x" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }
      let(:patch) { described_class.new(mount_point, ruby_ver) }

      it "includes additional patches for MacOs" do
        expected_patch_map = base_patch_map.merge(
          "configure" => described_class::DARWIN_CONFIGURE_PATCH,
          "ext/bigdecimal/bigdecimal.h" => described_class::EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH,
          "lib/rubygems/openssl.rb" => described_class::RUBYGEMS_OPENSSL_RB_PATCH
        )
        expect(patch.patch_map).to eq(expected_patch_map)
      end
    end
  end
end

RSpec.describe Tebako::Packager::Pass1MSysPatch do # rubocop:disable Metrics/BlockLength
  let(:mount_point) { "/mnt" }
  let(:base_patch_map) do
    {
      "tool/rbinstall.rb" => "TOOL_RBINSTALL_RB_PATCH",
      "lib/rubygems/path_support.rb" => "rubygems_path_support_patch",
      "ext/Setup" => "EXT_SETUP_PATCH"
    }
  end

  before do
    allow_any_instance_of(described_class).to receive(:base_patch_map).and_return(base_patch_map)
  end

  describe "#initialize" do
    let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }
    let(:patch) { described_class.new(mount_point, ruby_ver) }

    it "initializes with mount_point and ruby_ver" do
      expect(patch.instance_variable_get(:@mount_point)).to eq(mount_point)
      expect(patch.instance_variable_get(:@ruby_ver)).to eq(ruby_ver)
    end
  end

  describe "#patch_map" do # rubocop:disable Metrics/BlockLength
    context "when ruby_ver is not ruby34 and is ruby3x" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }
      let(:patch) { described_class.new(mount_point, ruby_ver) }

      it "includes additional patches for MSys and ruby3x" do
        expected_patch_map = base_patch_map.merge(
          "ext/bigdecimal/bigdecimal.h" => described_class::EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH,
          "lib/rubygems/openssl.rb" => described_class::RUBYGEMS_OPENSSL_RB_PATCH,
          "cygwin/GNUmakefile.in" => patch.send(:gnumakefile_in_patch_p1),
          "ext/io/console/win32_vk.inc" => described_class::EXT_IO_CONSOLE_WIN32_VK_INC_PATCH,
          "ext/openssl/extconf.rb" => described_class::OPENSSL_EXTCONF_RB_PATCH
        )
        expect(patch.patch_map).to eq(expected_patch_map)
      end
    end

    context "when ruby version is 3.3.7" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.3.7") }
      let(:patch) { described_class.new(mount_point, ruby_ver) }

      it "includes additional patches for MSys and ruby 3.3.7" do
        expected_patch_map = base_patch_map.merge(
          "ext/bigdecimal/bigdecimal.h" => described_class::EXT_BIGDECIMAL_BIGDECIMAL_H_PATCH,
          "cygwin/GNUmakefile.in" => patch.send(:gnumakefile_in_patch_p1),
          "ext/io/console/win32_vk.inc" => described_class::EXT_IO_CONSOLE_WIN32_VK_INC_PATCH,
          "ext/openssl/extconf.rb" => described_class::OPENSSL_EXTCONF_RB_PATCH,
          "lib/rubygems/openssl.rb" => described_class::RUBYGEMS_OPENSSL_RB_PATCH,
          "include/ruby/onigmo.h" => described_class::INCLUDE_RUBY_ONIGMO_H_PATCH,
          "win32/winmain.c" => described_class::WIN32_WINMAIN_C_PATCH
        )
        expect(patch.patch_map).to eq(expected_patch_map)
      end
    end

    context "when ruby version is 3.4.1" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.4.1") }
      let(:patch) { described_class.new(mount_point, ruby_ver) }

      it "includes additional patches for MSys and ruby 3.4.1" do
        expected_patch_map = base_patch_map.merge(
          "cygwin/GNUmakefile.in" => patch.send(:gnumakefile_in_patch_p1),
          "ext/io/console/win32_vk.inc" => described_class::EXT_IO_CONSOLE_WIN32_VK_INC_PATCH,
          "ext/openssl/extconf.rb" => described_class::OPENSSL_EXTCONF_RB_PATCH,
          "lib/rubygems/openssl.rb" => described_class::RUBYGEMS_OPENSSL_RB_PATCH,
          "include/ruby/onigmo.h" => described_class::INCLUDE_RUBY_ONIGMO_H_PATCH,
          "win32/winmain.c" => described_class::WIN32_WINMAIN_C_PATCH
        )
        expect(patch.patch_map).to eq(expected_patch_map)
      end
    end
  end
end
