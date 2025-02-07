# frozen_string_literal: true

# Copyright (c) 2025 [Ribose Inc](https://www.ribose.com).
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

# rubocop:disable Metrics/BlockLength
RSpec.describe Tebako::Packager::Pass2Patch do
  let(:ostype) { "linux-gnu" }
  let(:deps_lib_dir) { "deps/lib" }
  let(:ruby_ver) do
    double("RubyVersion", ruby34?: false, ruby3x?: true, ruby32only?: true, ruby33?: false, ruby32?: false,
                          ruby31?: false)
  end
  let(:patch) { described_class.new(ostype, deps_lib_dir, ruby_ver) }
  let(:scmb) { instance_double(Tebako::ScenarioManagerBase, msys?: false, musl?: false) }

  before do
    allow(Tebako::ScenarioManagerBase).to receive(:new).and_return(scmb)
  end

  describe "#initialize" do
    it "sets instance variables correctly" do
      expect(patch.instance_variable_get(:@ostype)).to eq(ostype)
      expect(patch.instance_variable_get(:@deps_lib_dir)).to eq(deps_lib_dir)
      expect(patch.instance_variable_get(:@ruby_ver)).to eq(ruby_ver)
    end
  end

  describe "#patch_map" do
    context "when on musl platform" do
      before { allow(scmb).to receive(:musl?).and_return(true) }

      it "includes LINUX_MUSL_THREAD_PTHREAD_PATCH" do
        expect(patch.patch_map).to include("thread_pthread.c" => described_class::LINUX_MUSL_THREAD_PTHREAD_PATCH)
      end
    end

    context "when on msys platform" do
      before do
        allow(scmb).to receive(:msys?).and_return(true)
        allow(patch).to receive(:msys_patches).and_return({ "msys_file" => "msys_patch" })
      end

      it "includes msys patches" do
        expect(patch.patch_map).to include("msys_file" => "msys_patch")
      end
    end

    context "when ruby version is 3.x" do
      before { allow(ruby_ver).to receive(:ruby3x?).and_return(true) }

      it "includes COMMON_MK_PATCH when not on msys" do
        expect(patch.patch_map).to include("common.mk" => described_class::COMMON_MK_PATCH)
      end
    end

    context "when ruby version is 3.3" do
      before do
        allow(ruby_ver).to receive(:ruby33?).and_return(true)
        allow(patch).to receive(:get_config_status_patch).and_return("config_status_patch")
      end

      it "includes config.status patch" do
        expect(patch.patch_map).to include("config.status" => "config_status_patch")
      end
    end

    context "when ruby version is 3.4" do
      before { allow(ruby_ver).to receive(:ruby34?).and_return(true) }

      it "includes PRISM_PATCHES" do
        expect(patch.patch_map).to include("prism_compile.c" => described_class::PRISM_PATCHES)
      end
    end
  end

  describe "#patch_map_base" do
    it "includes all base patches" do
      base_patches = patch.send(:patch_map_base)
      expect(base_patches.keys).to include(
        "template/Makefile.in",
        "tool/mkconfig.rb",
        "dir.c",
        "dln.c",
        "io.c",
        "main.c",
        "file.c",
        "util.c"
      )
    end
  end

  describe "#dir_c_patch" do
    context "when on msys" do
      let(:ostype) { "msys" }
      before { allow(scmb).to receive(:msys?).and_return(true) }

      it "uses correct pattern for msys" do
        patch = described_class.new(ostype, deps_lib_dir, ruby_ver)
        expect(patch.send(:dir_c_patch)).to include("/* define system APIs */" => anything)
      end
    end

    context "when not on msys" do
      before { allow(scmb).to receive(:msys?).and_return(false) }

      it "uses correct pattern for non-msys" do
        patch = described_class.new(ostype, deps_lib_dir, ruby_ver)
        expect(patch.send(:dir_c_patch)).to include("#ifdef HAVE_GETATTRLIST" => anything)
      end
    end
  end

  describe "#msys_patches" do
    let(:expected_gnumakefile_patch) { "gnumakefile_patch_content" }

    before do
      allow(patch).to receive(:get_gnumakefile_in_patch_p2)
        .with(ruby_ver)
        .and_return(expected_gnumakefile_patch)
    end

    it "returns correct patches for MSys platform" do
      expected_patches = {
        "cygwin/GNUmakefile.in" => expected_gnumakefile_patch,
        "ruby.c" => described_class::RUBY_C_MSYS_PATCHES,
        "win32/file.c" => described_class::WIN32_FILE_C_MSYS_PATCHES,
        "win32/win32.c" => described_class::WIN32_WIN32_C_MSYS_PATCHES
      }

      expect(patch.send(:msys_patches)).to eq(expected_patches)
    end

    it "calls get_gnumakefile_in_patch_p2 with correct ruby_ver" do
      expect(patch).to receive(:get_gnumakefile_in_patch_p2).with(ruby_ver)
      patch.send(:msys_patches)
    end
  end

  describe "#util_c_patch" do
    context "when ruby version is 3.1" do
      before do
        allow(ruby_ver).to receive(:ruby31?).and_return(true)
      end

      it "uses post-pattern for ruby 3.1" do
        patch = described_class.new(ostype, deps_lib_dir, ruby_ver)
        expect(Tebako::Packager::PatchHelpers).to receive(:patch_c_file_post)
          .with("#endif /* !HAVE_GNU_QSORT_R */")
        patch.send(:util_c_patch)
      end
    end

    context "when ruby version is not 3.1" do
      before do
        allow(ruby_ver).to receive(:ruby31?).and_return(false)
      end

      it "uses pre-pattern for non-ruby 3.1" do
        patch = described_class.new(ostype, deps_lib_dir, ruby_ver)
        expect(Tebako::Packager::PatchHelpers).to receive(:patch_c_file_pre)
          .with("#ifndef S_ISDIR")
        patch.send(:util_c_patch)
      end
    end
  end
end
# rubocop:enable Metrics/BlockLength
