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

# Tebako - an executable packager
module Tebako
  module Packager
    # Substitutions for cygwin/Gnumakefile.in shared across Pass1 and Pass2
    GNUMAKEFILE_IN_DLLTOOL_SUBST = <<~SUBST
      $(Q) dlltool --output-lib=$(LIBRUBY) --output-def=tebako.def --export-all $(LIBRUBY_A) --output-exp=$(RUBY_EXP)   # tebako patched
    SUBST

    # This MSYS specific thing ensure compilation of winmain.c
    # Did not try to understand why it did not work out of the box
    # It is shared across Pass1 and Pass2
    GNUMAKEFILE_IN_WINMAIN_SUBST = <<~SUBST
      RUBYDEF = $(DLL_BASE_NAME).def

      # Start of tebako patch
      WINMAINOBJ    = win32/winmain.$(OBJEXT)
      $(WINMAINOBJ): win32/winmain.c
      # End of tebako patch
    SUBST

    # Ruby patching definitions (common base)
    class Patch
      def patch_map
        {}.freeze
      end
    end
  end
end
