# Copyright (c) 2021, [Ribose Inc](https://www.ribose.com).
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


# ...................................................................
# ENSURE_BUNDLER
# This function ensures that the bundler gem of given version has been downloaded 
# VER                -- gem version
# HASH               -- gem file hash
# DIR                -- gem file folder

function(ENSURE_BUNDLER VER HASH DIR)
  set(BUNDLER_LOAD false)
  if (NOT EXISTS ${DIR}/bundler-${VER}.gem)
    set(BUNDLER_LOAD true)
  else()
    file(SHA256 ${DIR}/bundler-${VER}.gem BUNDLER_FILE_HASH)
    if (NOT ${HASH} STREQUAL ${BUNDLER_FILE_HASH})
      set(BUNDLER_LOAD true)
    endif()
  endif()
     
  if (${BUNDLER_LOAD})
    message("Loading bundler from https://rubygems.org/downloads/bundler-${VER}.gem ")
    file(DOWNLOAD https://rubygems.org/downloads/bundler-${VER}.gem 
                  ${DIR}/bundler-${VER}.gem
                  EXPECTED_HASH SHA256=${HASH}
    )
   else()
     message("Reusing bundler at ${DIR}/bundler-${VER}.gem")
   endif()
endfunction()

