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

require_relative "../../lib/tebako/packager/pass2msys_patch"

# rubocop:disable Metrics/BlockLength
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

# rubocop:enable Metrics/BlockLength
