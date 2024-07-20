# frozen_string_literal: true

# Copyright (c) 2023-2024 [Ribose Inc](https://www.ribose.com).
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

require_relative "patch_helpers"

# Tebako - an executable packager
module Tebako
  module Packager
    # Ruby buildsystem patches (pass2)
    module PatchBuildsystem
      # This patch forces rebuild of extinit.c when exts.mk changes
      # exts.mk changes when we rebuild extensions to make them statically linked
      # [TODO] it is possible that we can build extensions statically on pass1 and it will
      #        eliminate the need for this patch
      COMMON_MK_PATCH = {
        "ext/extinit.c: $(srcdir)/template/extinit.c.tmpl $(PREP)" =>
          "ext/extinit.c: $(srcdir)/template/extinit.c.tmpl $(PREP) $(EXTS_MK)"
      }.freeze

      # This patch changes libraries that are used for Ruby linking
      # MAINLIBS is patched elsewhere (not with literal but dynamically)
      #          to haold the list of sattic libraries and related options
      # Also we have to put LIBRUBYARG_STATIC instead of LIBRUBYARG to link with static libruby,
      # configure's --disable-shared option does not do it
      # Several variants depending on os and version
      TEMPLATE_MAKEFILE_IN_BASE_PATTERN_PRE_3_1 =
        "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(XLDFLAGS) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) $(LIBS) $(EXTLIBS) $(OUTFLAG)$@"

      TEMPLATE_MAKEFILE_IN_BASE_PATCH_PRE_3_1 =
        "# -- Start of tebako patch --\n" \
        "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG_STATIC) $(OUTFLAG)$@\n" \
        "# -- End of tebako patch --"

      TEMPLATE_MAKEFILE_IN_BASE_PATTERN_PRE_3_3 =
        "\t\t$(Q) $(PURIFY) $(CC) $(EXE_LDFLAGS) $(XLDFLAGS) $(MAINOBJ) $(EXTOBJS) " \
        "$(LIBRUBYARG) $(MAINLIBS) $(LIBS) $(EXTLIBS) $(OUTFLAG)$@"

      TEMPLATE_MAKEFILE_IN_BASE_PATTERN =
        "\t\t$(Q) $(PURIFY) $(CC) $(EXE_LDFLAGS) $(XLDFLAGS) $(MAINOBJ) $(EXTOBJS) " \
        "$(LIBRUBYARG) $(MAINLIBS) $(EXTLIBS) $(OUTFLAG)$@"

      TEMPLATE_MAKEFILE_IN_BASE_PATCH =
        "# -- Start of tebako patch --\n" \
        "\t\t$(Q) $(PURIFY) $(CC) $(EXE_LDFLAGS) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG_STATIC) $(OUTFLAG)$@\n" \
        "# -- End of tebako patch --"

      TEMPLATE_MAKEFILE_IN_BASE_PATCH_MSYS =
        "# -- Start of tebako patch --\n" \
        "\t\t$(Q) $(PURIFY) $(CC) $(EXE_LDFLAGS) $(RUBY_EXP) $(MAINOBJ) " \
        "$(EXTOBJS) $(LIBRUBYARG_STATIC) $(OUTFLAG)$@\n" \
        "# -- End of tebako patch --"

      def template_makefile_in_subst(ostype, ruby_ver)
        if PatchHelpers.msys?(ostype)
          TEMPLATE_MAKEFILE_IN_BASE_PATCH_MSYS
        elsif !PatchHelpers.ruby31?(ruby_ver)
          TEMPLATE_MAKEFILE_IN_BASE_PATCH_PRE_3_1
        else
          TEMPLATE_MAKEFILE_IN_BASE_PATCH
        end
      end

      def template_makefile_in_patch_two(ostype, ruby_ver)
        if !PatchHelpers.ruby31?(ruby_ver)
          { TEMPLATE_MAKEFILE_IN_BASE_PATTERN_PRE_3_1 => template_makefile_in_subst(ostype, ruby_ver) }
        elsif !PatchHelpers.ruby33?(ruby_ver)
          { TEMPLATE_MAKEFILE_IN_BASE_PATTERN_PRE_3_3 => template_makefile_in_subst(ostype, ruby_ver) }
        else
          { TEMPLATE_MAKEFILE_IN_BASE_PATTERN => template_makefile_in_subst(ostype, ruby_ver) }
        end
      end

      # This MSYS specific thing ensure compilation of winmain.c
      # Did try to understand why it did not work out of the box
      GNUMAKEFILE_IN_WINMAIN_SUBST = <<~SUBST
        RUBYDEF = $(DLL_BASE_NAME).def

        # Start of tebako patch
        WINMAINOBJ    = win32/winmain.$(OBJEXT)
        $(WINMAINOBJ): win32/winmain.c
        # End of tebako patch
      SUBST

      def get_config_status_pattern(ostype)
        case ostype
        when /linux-/
          "S[\"MAINLIBS\"]=\"-lz -lrt -lrt -ldl -lcrypt -lm -lpthread \""
        when /darwin/
          "S[\"MAINLIBS\"]=\"-ldl -lobjc -lpthread \""
        # when /msys/
        else
          raise Tebako::Error, "Unknown ostype #{ostype}"
        end
      end

      def get_config_status_patch(ostype, deps_lib_dir, ruby_ver)
        {
          get_config_status_pattern(ostype) =>
            "S[\"MAINLIBS\"]=\"#{PatchLibraries.mlibs(ostype, deps_lib_dir, ruby_ver, false)}\""
        }
      end

      # Other MSYS (GNUMakefile) specific patches
      #  - The same issue with libraries as for Makefile above
      #  - 'Kill' ruby.exp regeneration on pass2
      #     since we want to use output from pass1 for implib generation
      #     [VERY UGLY HACK]
      #  - Introduce LIBRUBY dependency on static extensions
      #    This is an addition to COMMON_MK_PATCH specified above
      def get_gnumakefile_in_patch_p2(ruby_ver) # rubocop:disable Metrics/MethodLength
        objext = PatchHelpers.ruby32?(ruby_ver) ? "$(OBJEXT)" : "@OBJEXT@"

        {
          "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext}" =>
            "$(WPROGRAM): $(RUBYW_INSTALL_NAME).res.#{objext} $(WINMAINOBJ)  # tebako patched",

          "RUBYDEF = $(DLL_BASE_NAME).def" => GNUMAKEFILE_IN_WINMAIN_SUBST,

          "$(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(LIBS) -o $@" =>
            "$(WINMAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) -o $@  # tebako patched",

          "$(RUBY_EXP): $(LIBRUBY_A)" => "dummy.exp: $(LIBRUBY_A) # tebako patched",

          "$(PROGRAM): $(RUBY_INSTALL_NAME).res.#{objext}" =>
            "$(PROGRAM): $(RUBY_INSTALL_NAME).res.#{objext} $(LIBRUBY_A) # tebako patched\n" \
            "$(LIBRUBY_A): $(LIBRUBY_A_OBJS) $(INITOBJS) # tebako patched\n"
        }
      end
    end
  end
end
