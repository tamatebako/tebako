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
RSpec.describe Tebako::Packager do
  describe ".crt_pass2_patch" do
    let(:ostype) { "linux-gnu" }
    let(:deps_lib_dir) { "/usr/lib" }
    let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }

    context "when on msys platform" do
      before do
        allow_any_instance_of(Tebako::ScenarioManagerBase).to receive(:msys?).and_return(true)
      end

      it "returns Pass2MSysPatch instance" do
        patch = described_class.crt_pass2_patch(ostype, deps_lib_dir, ruby_ver)
        expect(patch).to be_a(Tebako::Packager::Pass2MSysPatch)
      end
    end

    context "when not on msys platform" do
      before do
        allow_any_instance_of(Tebako::ScenarioManagerBase).to receive(:msys?).and_return(false)
      end

      it "returns Pass2NonMSysPatch instance" do
        patch = described_class.crt_pass2_patch(ostype, deps_lib_dir, ruby_ver)
        expect(patch).to be_a(Tebako::Packager::Pass2NonMSysPatch)
      end
    end
  end
end

RSpec.describe Tebako::Packager::Pass2Patch do
  let(:ostype) { "linux-gnu" }
  let(:deps_lib_dir) { "/usr/lib" }
  let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }
  let(:patch) { described_class.new(ostype, deps_lib_dir, ruby_ver) }
  let(:scmb) { Tebako::ScenarioManagerBase.new("linux-gnu") }

  before do
    allow(Tebako::ScenarioManagerBase).to receive(:new).and_return(scmb)
  end

  describe "#initialize" do
    it "sets instance variables correctly" do
      expect(patch.instance_variable_get(:@ostype)).to eq(ostype)
      expect(patch.instance_variable_get(:@deps_lib_dir)).to eq(deps_lib_dir)
      expect(patch.instance_variable_get(:@ruby_ver)).to eq(ruby_ver)
      expect(patch.instance_variable_get(:@scmb)).to be_instance_of(Tebako::ScenarioManagerBase)
    end
  end

  describe "#patch_map" do
    context "when on musl platform" do
      before { allow(scmb).to receive(:musl?).and_return(true) }

      it "includes thread_pthread.c patch" do
        expect(patch.patch_map).to include("thread_pthread.c" => described_class::LINUX_MUSL_THREAD_PTHREAD_PATCH)
      end
    end

    context "when ruby version is 3.4" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.4.1") }

      it "includes prism_compile.c patch" do
        expect(patch.patch_map).to include("prism_compile.c" => described_class::PRISM_PATCHES)
      end
    end
  end

  describe "#dir_c_patch" do
    context "when on msys platform" do
      before { allow(scmb).to receive(:msys?).and_return(true) }

      it "uses correct pattern and merges with base patch" do
        expect(Tebako::Packager::PatchHelpers)
          .to receive(:patch_c_file_pre)
          .with("/* define system APIs */")
          .and_return({})
        expect(patch.send(:dir_c_patch)).to eq(described_class::DIR_C_BASE_PATCH)
      end
    end

    context "when not on msys platform" do
      before { allow(scmb).to receive(:msys?).and_return(false) }

      it "uses correct pattern and merges with base patch" do
        expect(Tebako::Packager::PatchHelpers)
          .to receive(:patch_c_file_pre)
          .with("#ifdef HAVE_GETATTRLIST")
          .and_return({})
        expect(patch.send(:dir_c_patch)).to eq(described_class::DIR_C_BASE_PATCH)
      end
    end
  end

  describe "#dln_c_patch" do
    context "when on msys platform" do
      before { allow(scmb).to receive(:msys?).and_return(true) }

      context "when ruby version is 3.2" do
        let(:ruby_ver) { Tebako::RubyVersion.new("3.2.5") }

        it "includes msys patch for ruby 3.2" do
          patch_result = patch.send(:dln_c_patch)
          expect(patch_result).to include(described_class::DLN_C_MSYS_PATCH)
        end
      end

      context "when ruby version is pre-3.2" do
        let(:ruby_ver) { Tebako::RubyVersion.new("3.1.6") }

        it "includes msys patch for pre-3.2" do
          patch_result = patch.send(:dln_c_patch)
          expect(patch_result).to include(described_class::DLN_C_MSYS_PATCH_PRE32)
        end
      end
    end
  end

  describe "#io_c_msys_patch" do
    context "when ruby version is 3.2" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.2.5") }

      it "returns correct patch for ruby 3.2" do
        expect(patch.send(:io_c_msys_patch)).to include(described_class::IO_C_MSYS_PATCH)
      end
    end

    context "when ruby version is pre-3.2" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.1.6") }

      it "returns correct patch for pre-3.2" do
        expect(patch.send(:io_c_msys_patch)).to include(described_class::IO_C_MSYS_PATCH_PRE_32)
      end
    end
  end

  describe "#io_c_patch" do
    before do
      allow(Tebako::Packager::PatchHelpers).to receive(:patch_c_file_pre).and_return({})
    end

    it "calls patch_c_file_pre with correct pattern" do
      expect(Tebako::Packager::PatchHelpers).to receive(:patch_c_file_pre).with("/* define system APIs */")
      patch.send(:io_c_patch)
    end

    context "when on msys platform" do
      before { allow(scmb).to receive(:msys?).and_return(true) }

      it "merges with msys patch" do
        expect(patch).to receive(:io_c_msys_patch).and_return({})
        patch.send(:io_c_patch)
      end
    end
  end

  describe "#util_c_patch" do
    context "when ruby version is 3.1" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.1.6") }

      it "uses post-pattern for ruby 3.1" do
        expect(Tebako::Packager::PatchHelpers).to receive(:patch_c_file_post)
          .with("#endif /* !HAVE_GNU_QSORT_R */")
        patch.send(:util_c_patch)
      end
    end

    context "when ruby version is not 3.1" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.0.7") }

      it "uses pre-pattern for non-ruby 3.1" do
        expect(Tebako::Packager::PatchHelpers).to receive(:patch_c_file_pre)
          .with("#ifndef S_ISDIR")
        patch.send(:util_c_patch)
      end
    end
  end
end

RSpec.describe Tebako::Packager::Pass2MSysPatch do
  let(:ostype) { "msys" }
  let(:deps_lib_dir) { "/usr/lib" }
  let(:ruby_ver) { Tebako::RubyVersion.new("3.3.5") }
  let(:patch) { described_class.new(ostype, deps_lib_dir, ruby_ver) }

  describe "#patch_map" do
    before do
      allow(patch).to receive(:get_config_status_patch).and_return({})
      allow(patch).to receive(:gnumakefile_in_patch_p2).and_return({})
    end

    it "includes msys specific patches" do
      expect(patch.patch_map).to include(
        "cygwin/GNUmakefile.in",
        "ruby.c",
        "win32/file.c",
        "win32/win32.c",
        "config.status"
      )
    end

    it "calls required patch generation methods" do
      expect(patch).to receive(:get_config_status_patch)
      expect(patch).to receive(:gnumakefile_in_patch_p2)
      patch.patch_map
    end
  end

  describe "#gnumakefile_in_patch_p2" do
    shared_examples "common patches" do
      it "includes common patch elements" do
        result = patch.send(:gnumakefile_in_patch_p2)

        expect(result).to include(
          "$(Q) $(DLLWRAP) \\" => Tebako::Packager::GNUMAKEFILE_IN_DLLTOOL_SUBST,
          "--output-exp=$(RUBY_EXP) \\" => "# tebako patched --output-exp=$(RUBY_EXP) \\",
          "--export-all $(LIBRUBY_A) $(LIBS) -o $(PROGRAM)" =>
            "# tebako patched --export-all $(LIBRUBY_A) $(LIBS) -o $(PROGRAM)",
          "@rm -f $(PROGRAM)" => "# tebako patched @rm -f $(PROGRAM)",
          "\t$(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)" =>
            "# tebako patched  $(Q) $(LDSHARED) $(DLDFLAGS) $(OBJS) dmyext.o $(SOLIBS) -o $(PROGRAM)",
          "RUBYDEF = $(DLL_BASE_NAME).def" => Tebako::Packager::GNUMAKEFILE_IN_WINMAIN_SUBST,
          "$(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@" =>
            "$(WINMAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) -o $@  # tebako patched",
          "$(RUBY_EXP): $(LIBRUBY_A)" => "dummy.exp: $(LIBRUBY_A) # tebako patched"
        )
      end
    end

    context "when ruby version is 3.2" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.2.6") }

      include_examples "common patches"

      it "uses $(OBJEXT) for object extension" do
        result = patch.send(:gnumakefile_in_patch_p2)

        expect(result).to include(
          "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.$(OBJEXT)" =>
            "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.$(OBJEXT) $(WINMAINOBJ)  # tebako patched",
          "$(PROGRAM): $(RUBY_INSTALL_NAME).res.$(OBJEXT)" =>
            "$(PROGRAM): $(RUBY_INSTALL_NAME).res.$(OBJEXT) $(LIBRUBY_A) # tebako patched\n" \
            "$(LIBRUBY_A): $(LIBRUBY_A_OBJS) $(INITOBJS) # tebako patched\n"
        )
      end
    end

    context "when ruby version is not 3.2" do
      let(:ruby_ver) { Tebako::RubyVersion.new("3.1.6") }

      include_examples "common patches"

      it "uses @OBJEXT@ for object extension" do
        result = patch.send(:gnumakefile_in_patch_p2)

        expect(result).to include(
          "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.@OBJEXT@" =>
            "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.@OBJEXT@ $(WINMAINOBJ)  # tebako patched",
          "$(PROGRAM): $(RUBY_INSTALL_NAME).res.@OBJEXT@" =>
            "$(PROGRAM): $(RUBY_INSTALL_NAME).res.@OBJEXT@ $(LIBRUBY_A) # tebako patched\n" \
            "$(LIBRUBY_A): $(LIBRUBY_A_OBJS) $(INITOBJS) # tebako patched\n"
        )
      end
    end
  end
end

RSpec.describe Tebako::Packager::Pass2NonMSysPatch do
  let(:ostype) { "linux-gnu" }
  let(:deps_lib_dir) { "/usr/lib" }
  let(:ruby_ver) { Tebako::RubyVersion.new("3.3.6") }
  let(:patch) { described_class.new(ostype, deps_lib_dir, ruby_ver) }

  describe "#patch_map" do
    before do
      stub_const("RUBY_PLATFORM", "linux")
      allow(patch).to receive(:get_config_status_patch).and_return({})
    end

    context "when ruby version is 3.x" do
      it "includes common.mk patch" do
        expect(patch.patch_map).to include("common.mk" => described_class::COMMON_MK_PATCH)
      end
    end

    context "when ruby version is 3.3" do
      it "includes config.status patch" do
        expect(patch.patch_map).to include("config.status")
      end
    end

    context "when ruby version is not 3.x" do
      let(:ruby_ver) { Tebako::RubyVersion.new("2.7.8") }

      it "does not include common.mk patch" do
        expect(patch.patch_map).not_to include("common.mk")
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
